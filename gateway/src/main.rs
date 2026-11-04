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
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let config = Config::from_env()?;
    let store = Store::connect(&config.database_url).await?;

    info!("starting local NanoMQ consumer");
    let _local_mqtt = LocalMqtt::start(
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

    // Log incoming RPC (full routing handled via command module)
    tokio::spawn(async move {
        while let Some(rpc) = tb.rpc_rx.recv().await {
            info!(rpc = %rpc, "received ThingsBoard RPC");
        }
    });

    let mut jinja_env = Environment::new();
    jinja_env
        .add_template_owned("index.html", include_str!("../templates/index.html").to_owned())?;
    jinja_env
        .add_template_owned("_devices.html", include_str!("../templates/_devices.html").to_owned())?;

    let state = Arc::new(AppState {
        store,
        devices: config.devices.to_vec(),
        env: jinja_env,
        id_counter: Arc::new(Mutex::new(0)),
    });

    let router = web::router(state);
    let listener = tokio::net::TcpListener::bind(config.http_bind).await?;
    info!(addr = %config.http_bind, "dashboard listening");

    axum::serve(listener, router)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
            info!("shutdown signal received");
        })
        .await?;

    Ok(())
}

