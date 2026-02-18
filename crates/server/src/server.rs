use anyhow::Context as _;
use hyper::server::conn::http1;
use hyper_util::rt::{TokioIo, TokioTimer};
use tokio::net::TcpListener;
use tracing::{error, info};

use wassel_plugin_stack::Stack;

use crate::config::Config;

pub struct Server {
    config: Config,
}

impl Server {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub async fn serve(&self) -> anyhow::Result<()> {
        let stack = Stack::load(".").await.context("Loading stack")?;

        let addr = format!(
            "{host}:{port}",
            host = &self.config.host,
            port = &self.config.port
        );
        info!("Starting server at {addr}");
        let listener = TcpListener::bind(&addr)
            .await
            .context("Binding to {addr}")?;

        loop {
            let (tcp, _) = listener.accept().await.context("Accepting connection")?;
            let io = TokioIo::new(tcp);

            let service = stack.clone();

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
