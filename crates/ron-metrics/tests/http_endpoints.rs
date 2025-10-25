use axum::body::Body; // request body type
use ron_metrics::build_info::build_version;
use ron_metrics::{BaseLabels, HealthState, Metrics};

use http_body_util::BodyExt; // for .collect().to_bytes()
use hyper::{Request, StatusCode};
use hyper_util::client::legacy::{connect::HttpConnector, Client};
use hyper_util::rt::TokioExecutor;
use std::net::SocketAddr;

#[tokio::test]
async fn http_endpoints_smoke() {
    let base = BaseLabels {
        service: "test-svc".into(),
        instance: "itest-1".into(),
        build_version: build_version(),
        amnesia: "off".into(),
    };
    let health = HealthState::new();
    health.set("config_loaded".into(), true);
    health.set("db".into(), false);

    let metrics = Metrics::new(base, health).expect("metrics new");

    let (_jh, addr) = metrics
        .clone()
        .serve("127.0.0.1:0".parse::<SocketAddr>().unwrap())
        .await
        .expect("serve");

    // Hyper 1.x client via hyper-util
    let connector = HttpConnector::new();
    let client: Client<_, Body> = Client::builder(TokioExecutor::new()).build(connector);

    // /healthz -> 200
    let resp = client
        .request(
            Request::builder()
                .uri(format!("http://{addr}/healthz"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // /readyz -> 503 (db=false) + Retry-After + JSON body
    let resp = client
        .request(
            Request::builder()
                .uri(format!("http://{addr}/readyz"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert!(resp.headers().get(hyper::header::RETRY_AFTER).is_some());
    let body_bytes = resp
        .into_body()
        .collect()
        .await
        .expect("collect body")
        .to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert!(v.get("degraded").and_then(|b| b.as_bool()).unwrap());

    // flip -> ready
    metrics.set_ready("db", true);
    let resp = client
        .request(
            Request::builder()
                .uri(format!("http://{addr}/readyz"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // /metrics -> 200 text/plain
    let resp = client
        .request(
            Request::builder()
                .uri(format!("http://{addr}/metrics"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let ctype = resp
        .headers()
        .get(hyper::header::CONTENT_TYPE)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(ctype.starts_with("text/plain"));
}
