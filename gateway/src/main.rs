use std::sync::Arc;

use anyhow::Result;
use minijinja::Environment;
use tokio::sync::Mutex;
use tracing::info;

use iot_gateway::{
    config::Config,
    local_mqtt::LocalMqtt,
    store::Store,
    thingsboard::ThingsBoard,
    web::{self, AppState},
};

#[tokio::main]
async fn main() -> Result<()> {
    if let Err(e) = real_run().await {
        tracing::error!(reason = %e, "FATAL_MAIN_ERROR");
        eprintln!("FATAL_MAIN_ERROR: {:?}", e);
    }
    Ok(())
}

async fn real_run() -> Result<()> {
    tokio::spawn(async {
        use tokio::signal::unix::{signal, SignalKind};
        if let Ok(mut hup) = signal(SignalKind::hangup()) {
            while hup.recv().await.is_some() {}
        }
    });
    // Load gateway/.env or .env if present
    for env_path in &[".env", "gateway/.env"] {
        if let Ok(content) = std::fs::read_to_string(env_path) {
            for line in content.lines() {
                if let Some((k, v)) = line.split_once('=') {
                    let k = k.trim();
                    let v = v.trim();
                    if !k.is_empty() && !k.starts_with('#') && std::env::var(k).is_err() {
                        std::env::set_var(k, v);
                    }
                }
            }
        }
    }

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let config = Config::from_env()?;
    let store = Store::connect(&config.database_url).await?;

    info!("starting local NanoMQ consumer");
    let (local_client, _local_mqtt) = LocalMqtt::start(
        &config.nanomq_host,
        config.nanomq_port,
        &config.nanomq_username,
        &config.nanomq_password,
        config.devices.to_vec(),
        store.clone(),
    )
    .await?;

    info!("starting ThingsBoard gateway client");
    let mut tb = ThingsBoard::start(
        &config.tb_mqtt_host,
        config.tb_mqtt_port,
        &config.tb_gateway_token,
        &config.devices,
        store.clone(),
    )
    .await?;

    // Route incoming RPC from ThingsBoard to local device via local_client
    let rpc_local_client = local_client.clone();
    let rpc_tb_client = tb.client.clone();
    let dev_lookup = config.devices.clone();
    let id_gen = Arc::new(Mutex::new(0u64));
    let store_rpc = store.clone();
    tokio::spawn(async move {
        while let Some(rpc) = tb.rpc_rx.recv().await {
            info!(rpc = %rpc, "received ThingsBoard RPC");
            let mut counter = id_gen.lock().await;
            *counter += 1;
            let cid = format!("rpc-{}", *counter);
            drop(counter);

            let lookup_fn = |tb_name: &str| -> Option<String> {
                dev_lookup.iter().find(|d| d.name == tb_name).map(|d| d.local_id.clone())
            };

            match iot_gateway::command::from_rpc(&rpc, &lookup_fn, &mut || cid.clone()) {
                Ok(cmd) => {
                    if cmd.is_get {
                        if let (Some(tb_name), Some(rpc_id)) = (cmd.tb_device_name, cmd.rpc_id) {
                            let is_on = match store_rpc.latest_state(&cmd.device_id).await {
                                Ok(Some(st)) => {
                                    serde_json::from_str::<serde_json::Value>(&st.payload_json)
                                        .ok()
                                        .and_then(|v| v.get("light_on").and_then(|l| l.as_bool()))
                                        .unwrap_or(false)
                                }
                                _ => false,
                            };
                            let reply = serde_json::json!({
                                "device": tb_name,
                                "id": rpc_id,
                                "data": { "state": is_on }
                            }).to_string();
                            let _ = rpc_tb_client.publish("v1/gateway/rpc", rumqttc::QoS::AtLeastOnce, false, reply.as_bytes()).await;
                            info!(device = %tb_name, id = rpc_id, is_on, "replied to ThingsBoard get_light");
                        }
                    } else {
                        info!(topic = %cmd.topic, payload = %cmd.payload, "routing RPC to local device");
                        let _ = rpc_local_client.publish(&cmd.topic, rumqttc::QoS::AtLeastOnce, false, cmd.payload.as_bytes()).await;

                        if let (Some(tb_name), Some(rpc_id)) = (cmd.tb_device_name, cmd.rpc_id) {
                            let st = cmd.state.unwrap_or(true);
                            let reply = serde_json::json!({
                                "device": tb_name,
                                "id": rpc_id,
                                "data": { "state": st }
                            }).to_string();
                            let _ = rpc_tb_client.publish("v1/gateway/rpc", rumqttc::QoS::AtLeastOnce, false, reply.as_bytes()).await;
                            info!(device = %tb_name, id = rpc_id, "replied to ThingsBoard set_light");
                        }
                    }
                }
                Err(e) => tracing::warn!(reason = %e, "rejected ThingsBoard RPC"),
            }
        }
    });

    let mut jinja_env = Environment::new();
    jinja_env
        .add_template_owned("login.html", include_str!("../templates/login.html").to_owned())?;
    jinja_env
        .add_template_owned("index.html", include_str!("../templates/index.html").to_owned())?;
    jinja_env
        .add_template_owned("_devices.html", include_str!("../templates/_devices.html").to_owned())?;
    jinja_env
        .add_template_owned("chart.html", include_str!("../templates/chart.html").to_owned())?;

    let state = Arc::new(AppState {
        store,
        devices: config.devices.to_vec(),
        env: jinja_env,
        id_counter: Arc::new(Mutex::new(0)),
        local_client: Some(local_client),
        tb_host: config.tb_mqtt_host.clone(),
        tb_port: config.tb_mqtt_port,
        tb_device: config.tb_gateway_device_name.clone(),
    });

    let router = web::router(state);
    let listener = tokio::net::TcpListener::bind(config.http_bind).await?;
    info!(addr = %config.http_bind, "dashboard listening");

    match axum::serve(listener, router).await {
        Ok(_) => info!("axum exited Ok"),
        Err(e) => tracing::error!(reason = %e, "axum error"),
    }

    Ok(())
}

