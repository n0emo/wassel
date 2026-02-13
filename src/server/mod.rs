use hyper::server::conn::http1;
use hyper_util::rt::{TokioIo, TokioTimer};
use service::WasselService;
use tokio::net::TcpListener;
use tracing::{error, info};

use crate::{config::Config, plugin::PluginPool};

pub mod body;
pub mod errors;
pub mod response;
pub mod service;

pub struct Server {
    config: Config,
}

impl Server {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub async fn serve(&self) -> anyhow::Result<()> {
        let pool = PluginPool::new(&self.config).await?;
        let service = WasselService::new(pool);

        let addr = format!(
            "{host}:{port}",
            host = &self.config.host,
            port = &self.config.port
        );
        info!("Starting server at {addr}");
        let listener = TcpListener::bind(addr).await?;

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
                    error!("Error serving: {e:?}");
                }
            });
        }
    }
}
