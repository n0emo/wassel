use std::{convert::Infallible, sync::Arc};

use bytes::Bytes;
use http_body_util::Full;
use hyper::{server::conn::http1, service::service_fn, Response};
use hyper_util::rt::{TokioIo, TokioTimer};
use plugins::{PluginPool, PoolConfig};
use tokio::net::TcpListener;

mod plugins;

struct MyService {
    pool: PluginPool,
}
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Loading plugins");

    let pool = PluginPool::new(&PoolConfig::default()).await?;
    let service = Arc::new(MyService { pool });

    println!("Starting server");
    let listener = TcpListener::bind("127.0.0.1:9150").await?;

    loop {
        let (tcp, _) = listener.accept().await?;
        let io = TokioIo::new(tcp);

        let s = Arc::clone(&service);
        tokio::task::spawn(async move {
            if let Err(e) = http1::Builder::new()
                .timer(TokioTimer::new())
                .serve_connection(io, service_fn(move |req| {
                    let s = s.clone();

                    async move {
                        let Some(plugin) = s.pool.plugin_at(req.uri().path()) else {
                            return Ok::<_, Infallible>(Response::new(Full::new(Bytes::from("Not found"))));
                        };

                        let res = plugin.handle(req.uri().path()).await;

                        Ok::<_, Infallible>(Response::new(Full::new(Bytes::from(res))))
                    }
                }))
                .await {
                println!("Error serving: {e}");
            }
        });
    }
}
