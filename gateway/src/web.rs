use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    extract::{Form, State},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Router,
};
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
}

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(dashboard))
        .route("/devices", get(devices_fragment))
        .route("/devices/esp2/light", post(set_light))
        .route("/static/style.css", get(stylesheet))
        .with_state(state)
}

async fn dashboard(State(s): State<Arc<AppState>>) -> impl IntoResponse {
    let tmpl = s.env.get_template("index.html").unwrap();
    Html(tmpl.render(context!()).unwrap())
}

async fn devices_fragment(State(s): State<Arc<AppState>>) -> impl IntoResponse {
    let rows = build_rows(&s).await;
    let tmpl = s.env.get_template("_devices.html").unwrap();
    Html(tmpl.render(context!(devices => rows)).unwrap())
}

#[derive(Deserialize)]
pub struct LightForm {
    pub value: String,
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
        Ok(_cmd) => {
            // In full wiring _cmd would be published; for now just re-render the row
            let rows = build_rows(&s).await;
            let tmpl = s.env.get_template("_devices.html").unwrap();
            Html(tmpl.render(context!(devices => rows)).unwrap()).into_response()
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
            "index.html",
            include_str!("../templates/index.html").to_owned(),
        )
        .unwrap();
        env.add_template_owned(
            "_devices.html",
            include_str!("../templates/_devices.html").to_owned(),
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
        })
    }

    #[tokio::test]
    async fn dashboard_has_htmx_polling() {
        let state = test_state().await;
        let app = router(state);
        let resp = app
            .oneshot(Request::builder().uri("/").body(axum::body::Body::empty()).unwrap())
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
}

