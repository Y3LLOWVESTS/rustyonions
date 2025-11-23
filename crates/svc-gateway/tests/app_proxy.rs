//! app_proxy.rs — integration tests for `/app/*` → omnigate app plane.
//!
//! RO:WHAT  Spin up a dummy omnigate, then a real svc-gateway, and assert that
//!          `/app/*` forwards correctly: happy path, header propagation,
//!          error envelope pass-through, method/body/query semantics, and 502 Problems.
//! RO:WHY   Proves env wiring (`SVC_GATEWAY_OMNIGATE_BASE_URL`), proxy plumbing, and
//!          gives the SDKs a stable, tested contract surface.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Duration;

use axum::{
    http::{HeaderMap, Method, StatusCode, Uri},
    routing::{get, post},
    Json, Router,
};
use once_cell::sync::OnceCell;
use serde::Serialize;
use serde_json::Value;
use svc_gateway::{config::Config, observability::metrics, routes, state::AppState};
use tokio::net::TcpListener;

/// DTO for header echo responses.
#[derive(Serialize)]
struct HeaderEcho {
    authorization: Option<String>,
    x_ron_token: Option<String>,
    x_ron_passport: Option<String>,
    x_correlation_id: Option<String>,
    x_request_id: Option<String>,
}

/// DTO for method echo responses.
#[derive(Serialize)]
struct MethodEcho {
    method: String,
}

/// DTO for body + query echo responses.
#[derive(Serialize)]
struct BodyEcho {
    method: String,
    body: String,
    query: HashMap<String, String>,
}

/// Minimal Problem body used by dummy omnigate for 4xx/5xx responses.
#[derive(Serialize)]
struct ProblemBody {
    code: &'static str,
}

/// Test-only helper: register gateway metrics once per test binary and return
/// cloned handles for each caller.
///
/// RO:WHY
///   Prometheus uses a *global* registry. Calling `metrics::register()` more
///   than once in the same process tries to re-register the same metric names
///   and will error. In tests we often spin multiple gateways, so we guard the
///   registration behind a `OnceCell` here.
fn test_metrics_handles() -> metrics::MetricsHandles {
    static CELL: OnceCell<metrics::MetricsHandles> = OnceCell::new();
    CELL.get_or_init(|| metrics::register().expect("register metrics (test once)"))
        .clone()
}

/// Start a dummy omnigate app-plane server that exposes multiple app-plane routes:
/// - `/v1/app/ping`       → `{ "ok": true }`
/// - `/v1/app/headers`    → echoes selected headers
/// - `/v1/app/method`     → echoes HTTP method for all verbs
/// - `/v1/app/body`       → echoes method, body, and query
/// - `/v1/app/problem400` → 400 JSON Problem
/// - `/v1/app/problem403` → 403 JSON Problem
/// - `/v1/app/problem500` → 500 JSON Problem
async fn start_dummy_omnigate() -> SocketAddr {
    async fn ping_handler() -> Json<Value> {
        Json(serde_json::json!({ "ok": true }))
    }

    async fn headers_handler(headers: HeaderMap) -> Json<HeaderEcho> {
        fn grab(headers: &HeaderMap, name: &str) -> Option<String> {
            headers
                .get(name)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_owned())
        }

        Json(HeaderEcho {
            authorization: grab(&headers, "authorization"),
            x_ron_token: grab(&headers, "x-ron-token"),
            x_ron_passport: grab(&headers, "x-ron-passport"),
            x_correlation_id: grab(&headers, "x-correlation-id"),
            x_request_id: grab(&headers, "x-request-id"),
        })
    }

    async fn method_handler(method: Method) -> Json<MethodEcho> {
        Json(MethodEcho {
            method: method.as_str().to_owned(),
        })
    }

    // Manual query parser to avoid the `Query` extractor feature.
    fn parse_query(uri: &Uri) -> HashMap<String, String> {
        let mut map = HashMap::new();
        if let Some(qs) = uri.query() {
            for pair in qs.split('&') {
                if pair.is_empty() {
                    continue;
                }
                let mut parts = pair.splitn(2, '=');
                let key = parts.next().unwrap_or("").trim();
                if key.is_empty() {
                    continue;
                }
                let value = parts.next().unwrap_or("").trim();
                map.insert(key.to_owned(), value.to_owned());
            }
        }
        map
    }

    async fn body_handler(method: Method, uri: Uri, body: String) -> Json<BodyEcho> {
        let query = parse_query(&uri);
        Json(BodyEcho {
            method: method.as_str().to_owned(),
            body,
            query,
        })
    }

    async fn problem_400() -> (StatusCode, Json<ProblemBody>) {
        (
            StatusCode::BAD_REQUEST,
            Json(ProblemBody {
                code: "problem_400",
            }),
        )
    }

    async fn problem_403() -> (StatusCode, Json<ProblemBody>) {
        (
            StatusCode::FORBIDDEN,
            Json(ProblemBody {
                code: "problem_403",
            }),
        )
    }

    async fn problem_500() -> (StatusCode, Json<ProblemBody>) {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ProblemBody {
                code: "problem_500",
            }),
        )
    }

    let router = Router::new()
        .route("/v1/app/ping", get(ping_handler))
        .route("/v1/app/headers", get(headers_handler))
        .route(
            "/v1/app/method",
            get(method_handler)
                .post(method_handler)
                .put(method_handler)
                .delete(method_handler)
                .patch(method_handler),
        )
        .route("/v1/app/body", post(body_handler))
        .route("/v1/app/problem400", get(problem_400))
        .route("/v1/app/problem403", get(problem_403))
        .route("/v1/app/problem500", get(problem_500));

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind dummy omnigate");
    let addr = listener.local_addr().expect("omnigate local_addr");

    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("dummy omnigate serve");
    });

    // Wait until the dummy omnigate is actually accepting connections.
    let base = format!("http://{}", addr);
    let client = reqwest::Client::new();
    for _ in 0..50 {
        if let Ok(resp) = client.get(format!("{base}/v1/app/ping")).send().await {
            if resp.status().is_success() {
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    addr
}

/// Start a real svc-gateway instance wired to the given omnigate base address.
/// Returns the bound gateway socket address.
async fn start_gateway(omnigate_addr: SocketAddr) -> SocketAddr {
    let omnigate_base = format!("http://{}", omnigate_addr);

    // Configure gateway via env.
    std::env::set_var("SVC_GATEWAY_OMNIGATE_BASE_URL", &omnigate_base);
    std::env::set_var("SVC_GATEWAY_BIND_ADDR", "127.0.0.1:0");

    let cfg = Config::load().expect("load config with env overrides");
    let metrics_handles = test_metrics_handles();
    let state = AppState::new(cfg.clone(), metrics_handles);

    let router = routes::build_router(&state);

    let listener = TcpListener::bind(&cfg.server.bind_addr)
        .await
        .expect("bind gateway");
    let gateway_addr = listener.local_addr().expect("gateway local_addr");

    tokio::spawn(async move {
        axum::serve(listener, router).await.expect("gateway serve");
    });

    // Give the gateway a brief moment to boot.
    tokio::time::sleep(Duration::from_millis(100)).await;

    gateway_addr
}

/// Happy-path roundtrip: `/app/ping` should be forwarded to omnigate and
/// return whatever omnigate replies.
///
/// This uses env vars to wire the base URL:
/// - `SVC_GATEWAY_OMNIGATE_BASE_URL`
/// - `SVC_GATEWAY_BIND_ADDR`
#[tokio::test]
async fn app_proxy_happy_path() {
    // 1) Start dummy omnigate.
    let omnigate_addr = start_dummy_omnigate().await;

    // 2) Start gateway wired to omnigate.
    let gateway_addr = start_gateway(omnigate_addr).await;

    // 3) Call /app/ping on the gateway and assert we get omnigate's response.
    let url = format!("http://{}/app/ping", gateway_addr);
    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .send()
        .await
        .expect("gateway /app/ping response");

    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "expected 200 from gateway, got {}",
        resp.status()
    );

    let body: Value = resp.json().await.expect("parse JSON body");
    assert_eq!(body, serde_json::json!({ "ok": true }));
}

/// Header propagation: auth + app headers + correlation/request IDs should flow
/// through unchanged, and x-request-id must be present even when the client does
/// not send one (corr layer must add one).
#[tokio::test]
async fn app_proxy_preserves_auth_and_corr_headers_and_generates_request_id() {
    let omnigate_addr = start_dummy_omnigate().await;
    let gateway_addr = start_gateway(omnigate_addr).await;

    let url = format!("http://{}/app/headers", gateway_addr);
    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header("authorization", "Bearer test-token")
        .header("x-ron-token", "ron-token-123")
        .header("x-ron-passport", "passport-abc")
        .header("x-correlation-id", "corr-xyz")
        // NOTE: deliberately *not* sending x-request-id; corr layer must add one.
        .send()
        .await
        .expect("gateway /app/headers response");

    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = resp.json().await.expect("parse JSON body");

    assert_eq!(body["authorization"], "Bearer test-token");
    assert_eq!(body["x_ron_token"], "ron-token-123");
    assert_eq!(body["x_ron_passport"], "passport-abc");
    assert_eq!(body["x_correlation_id"], "corr-xyz");

    let req_id = body["x_request_id"].as_str().unwrap_or_default();
    assert!(
        !req_id.is_empty(),
        "expected corr layer to generate x-request-id"
    );
}

/// Upstream 4xx/5xx Problem bodies should pass through the gateway unchanged
/// (status + JSON body).
#[tokio::test]
async fn app_proxy_passes_problem_bodies_unchanged() {
    let omnigate_addr = start_dummy_omnigate().await;
    let gateway_addr = start_gateway(omnigate_addr).await;

    let client = reqwest::Client::new();

    let cases = [
        ("problem400", StatusCode::BAD_REQUEST, "problem_400"),
        ("problem403", StatusCode::FORBIDDEN, "problem_403"),
        (
            "problem500",
            StatusCode::INTERNAL_SERVER_ERROR,
            "problem_500",
        ),
    ];

    for (tail, expected_status, expected_code) in cases {
        let url = format!("http://{}/app/{tail}", gateway_addr);
        let resp = client
            .get(&url)
            .send()
            .await
            .unwrap_or_else(|e| panic!("gateway /app/{tail} response: {e}"));

        assert_eq!(
            resp.status(),
            expected_status,
            "status mismatch for /app/{tail}"
        );

        let body: Value = resp.json().await.expect("parse JSON body");
        assert_eq!(
            body["code"], expected_code,
            "problem code mismatch for /app/{tail}"
        );
    }
}

/// Method + body + query semantics:
/// - All HTTP verbs should round-trip.
/// - Body bytes and query parameters must arrive intact at omnigate.
#[tokio::test]
async fn app_proxy_preserves_methods_body_and_query() {
    let omnigate_addr = start_dummy_omnigate().await;
    let gateway_addr = start_gateway(omnigate_addr).await;

    let client = reqwest::Client::new();
    let method_url = format!("http://{}/app/method", gateway_addr);

    // Verify method round-trip for the common verbs.
    let methods = [
        reqwest::Method::GET,
        reqwest::Method::POST,
        reqwest::Method::PUT,
        reqwest::Method::DELETE,
        reqwest::Method::PATCH,
    ];

    for method in methods.iter() {
        let resp = client
            .request(method.clone(), &method_url)
            .send()
            .await
            .unwrap_or_else(|e| panic!("gateway /app/method {method:?} response: {e}"));

        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "status mismatch for method {}",
            method.as_str()
        );

        let body: Value = resp.json().await.expect("parse JSON body");
        assert_eq!(
            body["method"],
            method.as_str(),
            "method round-trip mismatch for {}",
            method.as_str()
        );
    }

    // Verify body + query propagation on POST.
    let body_url = format!("http://{}/app/body?foo=1&bar=2", gateway_addr);
    let payload = "hello-world";

    let resp = client
        .post(&body_url)
        .body(payload.to_string())
        .send()
        .await
        .expect("gateway /app/body response");

    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = resp.json().await.expect("parse JSON body");

    assert_eq!(body["method"], "POST");
    assert_eq!(body["body"], payload);
    assert_eq!(body["query"]["foo"], "1");
    assert_eq!(body["query"]["bar"], "2");
}

/// Transport failure (upstream unreachable) must return 502 with a plain Problem
/// JSON envelope `{ code: "upstream_unavailable", ... }`.
#[tokio::test]
async fn app_proxy_upstream_connect_failure_yields_problem_502() {
    // No omnigate started on purpose; point to an unbound port.
    std::env::set_var("SVC_GATEWAY_OMNIGATE_BASE_URL", "http://127.0.0.1:1");
    std::env::set_var("SVC_GATEWAY_BIND_ADDR", "127.0.0.1:0");

    let cfg = Config::load().expect("load config with env overrides");
    let metrics_handles = test_metrics_handles();
    let state = AppState::new(cfg.clone(), metrics_handles);

    let router = routes::build_router(&state);

    let listener = TcpListener::bind(&cfg.server.bind_addr)
        .await
        .expect("bind gateway");
    let gateway_addr = listener.local_addr().expect("gateway local_addr");

    tokio::spawn(async move {
        axum::serve(listener, router).await.expect("gateway serve");
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    let url = format!("http://{}/app/ping", gateway_addr);
    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .send()
        .await
        .expect("gateway /app/ping response");

    assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);

    let body: Value = resp.json().await.expect("parse JSON Problem body");
    assert_eq!(body["code"], "upstream_unavailable");
    assert_eq!(body["retryable"], true);
}
