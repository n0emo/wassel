#[tokio::main]
async fn main() -> anyhow::Result<()> {
    wassel_server::run_server().await
}
