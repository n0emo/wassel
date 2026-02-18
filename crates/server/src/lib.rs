use anyhow::Context;
use config::Config;
use server::Server;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

mod config;
mod server;

pub async fn run_server() -> anyhow::Result<()> {
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy()
        .add_directive("wasmtime=info".parse()?)
        .add_directive("cranelift_codegen=info".parse()?)
        .add_directive("cranelift_frontend=info".parse()?);

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();

    let config = Config::load().context("Loading config")?;

    let server = Server::new(config);
    server.serve().await.context("Serving")?;

    Ok(())
}
