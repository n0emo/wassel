use std::{pin, sync::Arc};

use bytes::Buf;
use http_body_util::combinators::BoxBody;
use hyper::{body::{Body, Frame}, server::conn::http1, service::service_fn, Response};
use hyper_util::rt::{TokioIo, TokioTimer};
use plugins::{PluginPool, PoolConfig};
use tokio::net::TcpListener;
use wasmtime_wasi_http::bindings::http::types::ErrorCode;

mod plugins;

struct FullBody<D: Buf> {
    data: Option<D>
}

impl<D> Body for FullBody<D>
where D: Buf + Unpin {
    type Data = D;
    type Error = ErrorCode;

    fn poll_frame(
        self: pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        std::task::Poll::Ready(self.get_mut().data.take().map(|d| Ok(Frame::data(d))))
    }
}

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
                .serve_connection(
                    io,
                    service_fn(move |req| {
                        let s = s.clone();


                        async move {
                            let Some(plugin) = s.pool.plugin_at(req.uri().path()) else {
                                let body = FullBody { data: Some(bytes::Bytes::from_static(b"Not found")) };
                                let response = Response::new(BoxBody::new(body));
                                return Ok::<_, ErrorCode>(response);
                            };

                            Ok(plugin.handle(req).await)
                        }
                    }),
                )
                .await
            {
                println!("Error serving: {e}");
            }
        });
    }
}
