use server::Server;

mod plugin;
mod server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = Server::new();
    server.serve().await?;

    Ok(())
}
