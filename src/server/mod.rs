use hyper::server::conn::http1;
use hyper_util::rt::{TokioIo, TokioTimer};
use service::WasselService;
use tokio::net::TcpListener;

use crate::plugin::{PluginPool, PoolConfig};

pub mod body;
pub mod errors;
pub mod response;
pub mod service;

pub struct Server {}

impl Server {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn serve(&self) -> anyhow::Result<()> {
        println!("Loading plugins");
        let pool = PluginPool::new(&PoolConfig::default()).await?;
        let service = WasselService::new(pool);

        println!("Starting server");
        let listener = TcpListener::bind("127.0.0.1:9150").await?;

        loop {
            let (tcp, _) = listener.accept().await?;
            let io = TokioIo::new(tcp);

            let service = service.clone();

            tokio::task::spawn(async move {
                if let Err(e) = http1::Builder::new()
                    .timer(TokioTimer::new())
                    .serve_connection(io, service)
                    .await
                {
                    println!("Error serving: {e}");
                }
            });
        }
    }
}
