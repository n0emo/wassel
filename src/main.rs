use server::Server;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

mod plugin;
mod server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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

    let server = Server::new();
    server.serve().await?;

    Ok(())
}
