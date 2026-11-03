mod config;
mod store;
mod telemetry;

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
    let config = config::Config::from_env()?;
    let _store = store::Store::connect(&config.database_url).await?;
    tokio::signal::ctrl_c().await?;
    Ok(())
}
