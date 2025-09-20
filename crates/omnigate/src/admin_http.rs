// crates/svc-omnigate/src/admin_http.rs
#![forbid(unsafe_code)]

use anyhow::Result;
use bytes::Bytes;
use http_body_util::Full;
use hyper::server::conn::http1;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::error;

use crate::metrics::Metrics;

fn resp_ok(body: &'static [u8]) -> Response<Full<Bytes>> {
    Response::new(Full::new(Bytes::from_static(body)))
}

fn resp_with_status(body: &'static [u8], code: StatusCode) -> Response<Full<Bytes>> {
    let mut resp = Response::new(Full::new(Bytes::from_static(body)));
    *resp.status_mut() = code;
    resp
}

fn resp_metrics(text: String) -> Response<Full<Bytes>> {
    let mut resp = Response::new(Full::new(Bytes::from(text)));
    resp.headers_mut().insert(
        hyper::header::CONTENT_TYPE,
        hyper::header::HeaderValue::from_static("text/plain; version=0.0.4"),
    );
    resp
}

pub async fn run(addr: SocketAddr, max_inflight: u64, metrics: Arc<Metrics>) -> Result<()> {
    let listener = TcpListener::bind(addr).await?;
    // If inflight utilization >= READY_OVERLOAD_PCT, report 503 on /readyz
    // Default 90 (%). Override with env READY_OVERLOAD_PCT=NN.
    let ready_threshold = std::env::var("READY_OVERLOAD_PCT")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(90);

    loop {
        let (stream, _) = listener.accept().await?;
        let metrics = metrics.clone();
        let svc = hyper::service::service_fn(move |req: Request<hyper::body::Incoming>| {
            let metrics = metrics.clone();
            async move {
                match (req.method(), req.uri().path()) {
                    (&hyper::Method::GET, "/healthz") => {
                        Ok::<_, std::convert::Infallible>(resp_ok(b"ok"))
                    }
                    (&hyper::Method::GET, "/readyz") => {
                        let inflight = metrics
                            .inflight_current
                            .load(std::sync::atomic::Ordering::Relaxed);
                        let limit = max_inflight.max(1);
                        let pct = inflight.saturating_mul(100) / limit;
                        let ready = pct < ready_threshold;
                        let resp = if ready {
                            resp_with_status(b"ready", StatusCode::OK)
                        } else {
                            resp_with_status(b"overloaded", StatusCode::SERVICE_UNAVAILABLE)
                        };
                        Ok::<_, std::convert::Infallible>(resp)
                    }
                    (&hyper::Method::GET, "/metrics") => {
                        let text = metrics.to_prom();
                        Ok::<_, std::convert::Infallible>(resp_metrics(text))
                    }
                    _ => Ok::<_, std::convert::Infallible>(resp_with_status(
                        b"nf",
                        StatusCode::NOT_FOUND,
                    )),
                }
            }
        });

        tokio::spawn(async move {
            let io = TokioIo::new(stream);
            if let Err(e) = http1::Builder::new().serve_connection(io, svc).await {
                error!("admin http connection error: {e}");
            }
        });
    }
}
