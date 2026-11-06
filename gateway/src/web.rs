use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    extract::{Form, Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Router,
};
use chrono::NaiveDate;
use minijinja::{context, Environment};
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::Mutex;

use crate::{command, config::DeviceConfig, store::Store};

/// View model for a single device row.
#[derive(serde::Serialize)]
pub struct DeviceRow {
    pub local_id: String,
    pub tb_name: String,
    pub values: Option<serde_json::Map<String, Value>>,
    pub last_seen_ms: Option<i64>,
    pub last_seen_ago_s: i64,
    pub stale: bool,
}

pub struct AppState {
    pub store: Store,
    pub devices: Vec<DeviceConfig>,
    pub env: Environment<'static>,
    /// Shared counter for correlation IDs (simple incrementing)
    pub id_counter: Arc<Mutex<u64>>,
    /// Optional local MQTT client to publish device control commands
    pub local_client: Option<rumqttc::AsyncClient>,
    pub tb_host: String,
    pub tb_port: u16,
    pub tb_device: String,
}

fn is_authenticated(headers: &HeaderMap) -> bool {
    headers
        .get(header::COOKIE)
        .and_then(|val| val.to_str().ok())
        .map(|cookie| cookie.contains("auth_session=admin_logged_in"))
        .unwrap_or(false)
}

fn redirect_to_login() -> Response {
    (
        StatusCode::SEE_OTHER,
        [(header::LOCATION, "/login")],
        Html(""),
    )
        .into_response()
}

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(dashboard))
        .route("/login", get(login_page).post(login_submit))
        .route("/logout", get(logout))
        .route("/devices", get(devices_fragment))
        .route("/devices/esp2/light", post(set_light))
        .route("/devices/esp2/ota", post(trigger_ota_post))
        .route("/firmware/esp2.bin", get(serve_firmware))
        .route("/firmware/version.json", get(serve_firmware_version))
        .route("/devices/:device_id/chart", get(device_chart))
        .route("/queue/clean", post(clean_queue))
        .route("/static/style.css", get(stylesheet))
        .with_state(state)
}

#[derive(Deserialize)]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}

async fn login_page(State(s): State<Arc<AppState>>) -> impl IntoResponse {
    let tmpl = s.env.get_template("login.html").unwrap();
    Html(tmpl.render(context!(error => None::<String>)).unwrap())
}

async fn login_submit(
    State(s): State<Arc<AppState>>,
    Form(form): Form<LoginForm>,
) -> Response {
    if form.username == "admin" && form.password == "admin" {
        (
            StatusCode::SEE_OTHER,
            [
                (header::LOCATION, "/"),
                (header::SET_COOKIE, "auth_session=admin_logged_in; Path=/; HttpOnly; SameSite=Lax"),
            ],
            Html(""),
        )
            .into_response()
    } else {
        let tmpl = s.env.get_template("login.html").unwrap();
        Html(tmpl.render(context!(error => Some("Invalid username or password"))).unwrap()).into_response()
    }
}

async fn logout() -> Response {
    (
        StatusCode::SEE_OTHER,
        [
            (header::LOCATION, "/login"),
            (header::SET_COOKIE, "auth_session=; Path=/; Max-Age=0"),
        ],
        Html(""),
    )
        .into_response()
}

async fn device_chart(
    State(s): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(device_id): Path<String>,
) -> Response {
    if !is_authenticated(&headers) {
        return redirect_to_login();
    }
    let metrics = s.store.history_metrics(&device_id).await.unwrap_or_default();
    let mut series_map = serde_json::Map::new();
    for m in &metrics {
        let pts = s.store.history_points(&device_id, m, 200).await.unwrap_or_default();
        let pts_val: Vec<Value> = pts.into_iter().map(|(ts, val)| serde_json::json!([ts, val])).collect();
        series_map.insert(m.clone(), Value::Array(pts_val));
    }
    let series_json = serde_json::to_string(&series_map).unwrap_or_else(|_| "{}".into());
    match s.env.get_template("chart.html") {
        Ok(tmpl) => match tmpl.render(context!(device_id => device_id, metrics => metrics, series_json => series_json)) {
            Ok(rendered) => Html(rendered).into_response(),
            Err(e) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        },
        Err(e) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn dashboard(State(s): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    if !is_authenticated(&headers) {
        return redirect_to_login();
    }
    let tmpl = s.env.get_template("index.html").unwrap();
    Html(
        tmpl.render(context!(
            tb_host => s.tb_host,
            tb_port => s.tb_port,
            tb_device => s.tb_device,
        ))
        .unwrap(),
    )
    .into_response()
}

async fn devices_fragment(State(s): State<Arc<AppState>>) -> impl IntoResponse {
    let rows = build_rows(&s).await;
    let queue_count = s.store.outbox_count().await.unwrap_or(0);
    let tmpl = s.env.get_template("_devices.html").unwrap();
    Html(tmpl.render(context!(devices => rows, queue_count => queue_count)).unwrap())
}

#[derive(Deserialize)]
pub struct LightForm {
    pub value: String,
}

#[derive(Deserialize)]
pub struct OtaForm {
    pub force: Option<String>,
}

async fn trigger_ota_post(
    State(s): State<Arc<AppState>>,
    Form(form): Form<OtaForm>,
) -> Response {
    let mut counter = s.id_counter.lock().await;
    *counter += 1;
    let cid = format!("dash-ota-{}", *counter);
    drop(counter);

    let force = form.force.as_deref() == Some("true");
    let payload = serde_json::json!({
        "id": cid,
        "method": "ota_update",
        "params": {
            "url": "http://192.168.1.19:8080/firmware/esp2.bin",
            "version": "v1.0.1",
            "force": force
        }
    })
    .to_string();

    if let Some(client) = &s.local_client {
        let _ = client
            .publish(
                "iot/esp2/command",
                rumqttc::QoS::AtLeastOnce,
                false,
                payload.as_bytes(),
            )
            .await;
    }

    let rows = build_rows(&s).await;
    let queue_count = s.store.outbox_count().await.unwrap_or(0);
    let tmpl = s.env.get_template("_devices.html").unwrap();
    Html(tmpl.render(context!(devices => rows, queue_count => queue_count)).unwrap()).into_response()
}

async fn serve_firmware() -> Response {
    let paths = [
        "firmware/esp2.bin",
        "gateway/firmware/esp2.bin",
        "../gateway/firmware/esp2.bin",
    ];
    for p in &paths {
        if let Ok(bytes) = tokio::fs::read(p).await {
            return (
                StatusCode::OK,
                [
                    (header::CONTENT_TYPE, "application/octet-stream"),
                    (
                        header::CONTENT_DISPOSITION,
                        "attachment; filename=\"esp2.bin\"",
                    ),
                ],
                bytes,
            )
                .into_response();
        }
    }
    (StatusCode::NOT_FOUND, "Firmware binary not found").into_response()
}

async fn serve_firmware_version() -> Response {
    let paths = [
        "gateway/firmware/version.json",
        "firmware/version.json",
        "../gateway/firmware/version.json",
    ];
    for p in &paths {
        if let Ok(content) = tokio::fs::read_to_string(p).await {
            return (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "application/json")],
                content,
            )
                .into_response();
        }
    }
    let payload = serde_json::json!({
        "version": "1.0.4",
        "url": "http://192.168.1.19:8080/firmware/esp2.bin"
    });
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        payload.to_string(),
    )
        .into_response()
}

#[derive(Deserialize)]
pub struct CleanQueueForm {
    pub from_date: String,
    pub to_date: String,
}

async fn clean_queue(
    State(s): State<Arc<AppState>>,
    Form(form): Form<CleanQueueForm>,
) -> Response {
    let parse_from = NaiveDate::parse_from_str(&form.from_date, "%Y-%m-%d")
        .map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp_millis());
    let parse_to = NaiveDate::parse_from_str(&form.to_date, "%Y-%m-%d")
        .map(|d| d.and_hms_opt(23, 59, 59).unwrap().and_utc().timestamp_millis());

    match (parse_from, parse_to) {
        (Ok(from_ms), Ok(to_ms)) => {
            let _ = s.store.clean_outbox_range(from_ms, to_ms).await;
            let rows = build_rows(&s).await;
            let queue_count = s.store.outbox_count().await.unwrap_or(0);
            let tmpl = s.env.get_template("_devices.html").unwrap();
            Html(tmpl.render(context!(devices => rows, queue_count => queue_count)).unwrap()).into_response()
        }
        _ => (axum::http::StatusCode::BAD_REQUEST, "invalid date format, use YYYY-MM-DD").into_response(),
    }
}

async fn set_light(
    State(s): State<Arc<AppState>>,
    Form(form): Form<LightForm>,
) -> Response {
    let mut counter = s.id_counter.lock().await;
    *counter += 1;
    let cid = format!("dash-{}", *counter);
    drop(counter);

    match command::from_dashboard("esp2", "set_light", &form.value, &mut || cid.clone()) {
        Ok(cmd) => {
            if let Some(client) = &s.local_client {
                let _ = client.publish(&cmd.topic, rumqttc::QoS::AtLeastOnce, false, cmd.payload.as_bytes()).await;
            }
            let rows = build_rows(&s).await;
            let queue_count = s.store.outbox_count().await.unwrap_or(0);
            let tmpl = s.env.get_template("_devices.html").unwrap();
            Html(tmpl.render(context!(devices => rows, queue_count => queue_count)).unwrap()).into_response()
        }
        Err(e) => {
            (axum::http::StatusCode::BAD_REQUEST, e.to_string()).into_response()
        }
    }
}

async fn stylesheet() -> impl IntoResponse {
    (
        [("content-type", "text/css")],
        include_str!("../static/style.css"),
    )
}

const STALE_THRESHOLD_MS: i64 = 60_000;

async fn build_rows(s: &Arc<AppState>) -> Vec<DeviceRow> {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let mut rows = Vec::new();
    for dev in &s.devices {
        let state = s.store.latest_state(&dev.local_id).await.ok().flatten();
        let (values, last_seen_ms) = match &state {
            Some(st) => {
                let parsed: Option<serde_json::Map<String, Value>> =
                    serde_json::from_str::<Value>(&st.payload_json)
                        .ok()
                        .and_then(|v| {
                            v.get("values")
                                .and_then(|v| v.as_object())
                                .cloned()
                        });
                (parsed, Some(st.last_seen_ms))
            }
            None => (None, None),
        };
        let last_seen_ago_s = last_seen_ms.map(|ms| (now_ms - ms) / 1000).unwrap_or(0);
        let stale = last_seen_ms
            .map(|ms| now_ms - ms > STALE_THRESHOLD_MS)
            .unwrap_or(true);
        rows.push(DeviceRow {
            local_id: dev.local_id.clone(),
            tb_name: dev.name.clone(),
            values,
            last_seen_ms,
            last_seen_ago_s,
            stale,
        });
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::to_bytes, http::Request};
    use minijinja::Environment;
    use tower::ServiceExt;

    fn test_env() -> Environment<'static> {
        let mut env = Environment::new();
        env.add_template_owned(
            "login.html",
            include_str!("../templates/login.html").to_owned(),
        )
        .unwrap();
        env.add_template_owned(
            "index.html",
            include_str!("../templates/index.html").to_owned(),
        )
        .unwrap();
        env.add_template_owned(
            "_devices.html",
            include_str!("../templates/_devices.html").to_owned(),
        )
        .unwrap();
        env.add_template_owned(
            "chart.html",
            include_str!("../templates/chart.html").to_owned(),
        )
        .unwrap();
        env
    }

    async fn test_state() -> Arc<AppState> {
        let store = Store::connect("sqlite::memory:").await.unwrap();
        Arc::new(AppState {
            store,
            devices: vec![
                DeviceConfig { local_id: "esp1".into(), name: "ESP1 CO2".into(), profile: "CO2".into() },
                DeviceConfig { local_id: "esp2".into(), name: "ESP2 TDS".into(), profile: "TDS".into() },
            ],
            env: test_env(),
            id_counter: Arc::new(Mutex::new(0)),
            local_client: None,
            tb_host: "127.0.0.1".into(),
            tb_port: 1883,
            tb_device: "TestGateway".into(),
        })
    }

    #[tokio::test]
    async fn dashboard_has_htmx_polling() {
        let state = test_state().await;
        let app = router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header("cookie", "auth_session=admin_logged_in")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let html = std::str::from_utf8(&body).unwrap();
        assert!(html.contains("hx-get=\"/devices\""), "fragment polling missing");
    }

    #[tokio::test]
    async fn devices_fragment_has_device_rows() {
        let state = test_state().await;
        let app = router(state);
        let resp = app
            .oneshot(Request::builder().uri("/devices").body(axum::body::Body::empty()).unwrap())
            .await
            .unwrap();
        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let html = std::str::from_utf8(&body).unwrap();
        assert!(html.contains("esp1"), "ESP1 row missing");
        assert!(html.contains("esp2"), "ESP2 row missing");
        assert!(html.contains("ESP1 CO2"), "ESP1 tb_name missing");
        assert!(html.contains("Outbox Queue"), "Queue panel missing");
    }

    #[tokio::test]
    async fn clean_queue_endpoint_works() {
        let state = test_state().await;
        let app = router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/queue/clean")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(axum::body::Body::from("from_date=2026-01-01&to_date=2026-12-31"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
    }

    #[tokio::test]
    async fn light_on_accepted() {
        let state = test_state().await;
        let app = router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/devices/esp2/light")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(axum::body::Body::from("value=on"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
    }

    #[tokio::test]
    async fn ota_notify_and_force_accepted() {
        let state = test_state().await;
        let app = router(state.clone());
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/devices/esp2/ota")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(axum::body::Body::from("force=false"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);

        let app2 = router(state);
        let resp2 = app2
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/devices/esp2/ota")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(axum::body::Body::from("force=true"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp2.status(), 200);
    }

    #[tokio::test]
    async fn firmware_endpoint_works() {
        let state = test_state().await;
        let app = router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/firmware/esp2.bin")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        // Since firmware/esp2.bin or gateway/firmware/esp2.bin exists, it returns 200
        assert_eq!(resp.status(), 200);
        assert_eq!(
            resp.headers().get("content-type").unwrap(),
            "application/octet-stream"
        );
    }

    #[tokio::test]
    async fn light_invalid_value_returns_400() {
        let state = test_state().await;
        let app = router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/devices/esp2/light")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(axum::body::Body::from("value=maybe"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), 400);
    }

    #[tokio::test]
    async fn device_chart_endpoint_works() {
        let state = test_state().await;
        let app = router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/devices/esp1/chart")
                    .header("cookie", "auth_session=admin_logged_in")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let html = std::str::from_utf8(&body).unwrap();
        assert!(html.contains("Device History: esp1"));
    }

    #[tokio::test]
    async fn unauthenticated_redirects_to_login() {
        let state = test_state().await;
        let app = router(state);
        let resp = app
            .oneshot(Request::builder().uri("/").body(axum::body::Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), 303);
        assert_eq!(resp.headers().get("location").unwrap(), "/login");
    }

    #[tokio::test]
    async fn login_with_admin_sets_session_cookie() {
        let state = test_state().await;
        let app = router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/login")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(axum::body::Body::from("username=admin&password=admin"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), 303);
        assert!(resp.headers().get("set-cookie").unwrap().to_str().unwrap().contains("auth_session=admin_logged_in"));
    }
}

