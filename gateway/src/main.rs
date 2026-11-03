use iot_gateway::{config::Config, local_mqtt::LocalMqtt, store::Store};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    if let Err(error) = start().await {
        tracing::error!(%error, "gateway startup failed");
        std::process::exit(1);
    }
}

async fn start() -> anyhow::Result<()> {
    let config = Config::from_env()?;
    let store = Store::connect(&config.database_url).await?;
    let mqtt = LocalMqtt::start(
        &config.nanomq_host,
        config.nanomq_port,
        &config.nanomq_username,
        &config.nanomq_password,
        config.devices.to_vec(),
        store,
    )
    .await?;
    tokio::signal::ctrl_c().await?;
    mqtt.stop();
    Ok(())
}
