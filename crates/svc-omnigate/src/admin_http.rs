#![forbid(unsafe_code)]

use anyhow::Result;
use bytes::Bytes;
use http_body_util::Full;
use hyper::server::conn::http1;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use rand::Rng;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::error;

use crate::metrics::Metrics;

pub async fn run(addr: SocketAddr, max_inflight: u64, metrics: Arc<Metrics>) -> Result<()> {
    use hyper::service::service_fn;

    let listener = TcpListener::bind(addr).await?;
    tracing::info!("admin http listening on {}", addr);

    loop {
        let (stream, _peer) = listener.accept().await?;
        let metrics = metrics.clone();

        // Closure returns a Future; we let the async block infer, and
        // pin the concrete Response/Body type inside each branch.
        let svc = service_fn(move |req: Request<hyper::body::Incoming>| {
            let metrics = metrics.clone();
            async move {
                match (req.method().as_str(), req.uri().path()) {
                    ("GET", "/healthz") => {
                        let resp: Response<Full<Bytes>> =
                            Response::new(Full::new(Bytes::from_static(b"ok")));
                        Ok::<Response<Full<Bytes>>, hyper::Error>(resp)
                    }

                    ("GET", "/readyz") => {
                        let inflight =
                            metrics.inflight_current.load(std::sync::atomic::Ordering::Relaxed);
                        if inflight >= max_inflight {
                            let retry_after = rand::thread_rng().gen_range(1..=5);
                            let mut resp: Response<Full<Bytes>> =
                                Response::new(Full::new(Bytes::from_static(b"overloaded")));
                            *resp.status_mut() = StatusCode::SERVICE_UNAVAILABLE;
                            resp.headers_mut().insert(
                                hyper::header::RETRY_AFTER,
                                hyper::header::HeaderValue::from_str(&retry_after.to_string())
                                    .unwrap(),
                            );
                            Ok::<Response<Full<Bytes>>, hyper::Error>(resp)
                        } else {
                            Ok::<Response<Full<Bytes>>, hyper::Error>(Response::new(Full::new(
                                Bytes::from_static(b"ready"),
                            )))
                        }
                    }

                    ("GET", "/metrics") => {
                        let body = metrics.to_prom();
                        Ok::<Response<Full<Bytes>>, hyper::Error>(Response::new(Full::new(
                            Bytes::from(body),
                        )))
                    }

                    _ => {
                        let mut resp: Response<Full<Bytes>> =
                            Response::new(Full::new(Bytes::from_static(b"not found")));
                        *resp.status_mut() = StatusCode::NOT_FOUND;
                        Ok::<Response<Full<Bytes>>, hyper::Error>(resp)
                    }
                }
            }
        });

        tokio::spawn(async move {
            // Hyper 1 expects IO that implements hyper::rt::{Read, Write, Unpin}
            let io = TokioIo::new(stream);
            if let Err(e) = http1::Builder::new().serve_connection(io, svc).await {
                error!("admin http connection error: {e}");
            }
        });
    }
}
