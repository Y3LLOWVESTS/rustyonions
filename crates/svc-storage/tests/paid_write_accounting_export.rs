//! RO:WHAT — Route-level tests for paid-storage usage event HTTP export.
//! RO:WHY — Pillar 12; Concerns: ECON/RES/DX. Storage usage must reach accounting adapter shape.
//! RO:INTERACTS — svc_storage::http::server, accounting::exporter, mock accounting /v1/usage-events.
//! RO:INVARIANTS — export is opt-in; idempotent; export failure must not be ledger/wallet mutation.
//! RO:METRICS — exercises storage_accounting_export_total path indirectly.
//! RO:CONFIG — sets RON_STORAGE_ACCOUNTING_EXPORT_MODE/base URL/bearer/timeout.
//! RO:SECURITY — mock bearer only; no real wallet secret, body bytes, private content, or external chain.
//! RO:TEST — cargo test -p svc-storage --test paid_write_accounting_export.

use std::sync::{Arc, Mutex};

use axum::{
    body::{to_bytes, Body},
    extract::State,
    http::{header, HeaderMap, Method, Request, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use serde_json::{json, Value};
use svc_storage::{
    http::{extractors::AppState, server::build_router},
    storage::{MemoryStorage, Storage},
};
use tower::ServiceExt;

const OBJECT_BYTES: &[u8] = b"accounting export paid storage object";
const RECEIPT_HASH: &str = "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

#[derive(Clone, Default)]
struct AccountingMockState {
    requests: Arc<Mutex<Vec<Value>>>,
    idempotency_keys: Arc<Mutex<Vec<String>>>,
}

async fn accounting_ingest(
    State(state): State<AccountingMockState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    let bearer = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");

    if bearer != "Bearer dev" {
        return (StatusCode::UNAUTHORIZED, "missing bearer").into_response();
    }

    let idem = headers
        .get("idempotency-key")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_string();

    if !idem.starts_with("storage_acct:") {
        return (
            StatusCode::BAD_REQUEST,
            format!("bad idempotency key: {idem}"),
        )
            .into_response();
    }

    state
        .idempotency_keys
        .lock()
        .expect("idempotency key list lock should not be poisoned")
        .push(idem);

    state
        .requests
        .lock()
        .expect("request list lock should not be poisoned")
        .push(body);

    (StatusCode::ACCEPTED, Json(json!({"ok": true}))).into_response()
}

async fn spawn_accounting_mock() -> (String, AccountingMockState) {
    let state = AccountingMockState::default();
    let app = Router::new()
        .route("/v1/usage-events", post(accounting_ingest))
        .with_state(state.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("mock accounting listener should bind");

    let addr = listener
        .local_addr()
        .expect("mock accounting listener should expose local addr");

    tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("mock accounting server should run");
    });

    (format!("http://{addr}"), state)
}

fn storage_app() -> Router {
    let store: Arc<dyn Storage> = Arc::new(MemoryStorage::default());
    let state = AppState { store };

    build_router().with_state(state)
}

fn paid_headers() -> Vec<(String, String)> {
    vec![
        ("x-ron-paid-op".to_string(), "hold".to_string()),
        ("x-ron-paid-asset".to_string(), "roc".to_string()),
        ("x-ron-paid-estimate-minor".to_string(), "70".to_string()),
        (
            "x-ron-wallet-txid".to_string(),
            "tx_accounting_export".to_string(),
        ),
        (
            "x-ron-wallet-receipt-hash".to_string(),
            RECEIPT_HASH.to_string(),
        ),
        ("x-ron-wallet-from".to_string(), "acct_user".to_string()),
        (
            "x-ron-wallet-to".to_string(),
            "escrow_paid_write".to_string(),
        ),
        ("x-ron-tenant".to_string(), "7".to_string()),
        (
            "x-ron-accounting-subject".to_string(),
            "svc_storage_provider".to_string(),
        ),
        ("x-ron-region".to_string(), "us-central".to_string()),
        ("x-ron-pin-seconds".to_string(), "60".to_string()),
    ]
}

fn paid_post_with_headers(headers: &[(String, String)], body: &[u8]) -> Request<Body> {
    let mut builder = Request::builder()
        .method(Method::POST)
        .uri("/paid/o")
        .header(header::CONTENT_TYPE, "application/octet-stream");

    for (name, value) in headers {
        builder = builder.header(name.as_str(), value.as_str());
    }

    builder
        .body(Body::from(body.to_vec()))
        .expect("paid POST request should build")
}

async fn send(router: Router, request: Request<Body>) -> (StatusCode, HeaderMap, Vec<u8>) {
    let response = router
        .oneshot(request)
        .await
        .expect("router request should complete");

    let status = response.status();
    let headers = response.headers().clone();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should read")
        .to_vec();

    (status, headers, body)
}

fn json_body(bytes: &[u8]) -> Value {
    serde_json::from_slice::<Value>(bytes).expect("response body should be JSON")
}

fn configure_accounting_export(base_url: &str) {
    std::env::set_var("RON_STORAGE_PAID_WRITE_VERIFIER_MODE", "dev-header");
    std::env::set_var("RON_STORAGE_PAID_SETTLEMENT_MODE", "disabled");
    std::env::set_var("RON_STORAGE_ACCOUNTING_EXPORT_MODE", "http");
    std::env::set_var("RON_STORAGE_ACCOUNTING_BASE_URL", base_url);
    std::env::set_var("RON_STORAGE_ACCOUNTING_BEARER", "dev");
    std::env::set_var("RON_STORAGE_ACCOUNTING_TIMEOUT_MS", "2000");
}

fn clear_accounting_export() {
    std::env::remove_var("RON_STORAGE_PAID_WRITE_VERIFIER_MODE");
    std::env::remove_var("RON_STORAGE_PAID_SETTLEMENT_MODE");
    std::env::remove_var("RON_STORAGE_ACCOUNTING_EXPORT_MODE");
    std::env::remove_var("RON_STORAGE_ACCOUNTING_BASE_URL");
    std::env::remove_var("RON_STORAGE_ACCOUNTING_BEARER");
    std::env::remove_var("RON_STORAGE_ACCOUNTING_TIMEOUT_MS");
}

#[tokio::test]
async fn paid_route_exports_usage_events_to_accounting_http_adapter() {
    let (base_url, accounting_state) = spawn_accounting_mock().await;
    configure_accounting_export(&base_url);

    let app = storage_app();
    let (status, _headers, body) =
        send(app, paid_post_with_headers(&paid_headers(), OBJECT_BYTES)).await;

    assert_eq!(status, StatusCode::OK);

    let json = json_body(&body);
    assert_eq!(json["paid"], true);
    assert_eq!(json["accounting_export"]["mode"], "http");
    assert_eq!(json["accounting_export"]["status"], "exported");
    assert_eq!(json["accounting_export"]["event_count"], 3);
    assert_eq!(json["accounting_export"]["http_status"], 202);

    let export_key = json["accounting_export"]["idempotency_key"]
        .as_str()
        .expect("export idempotency key should be a string");
    assert!(export_key.starts_with("storage_acct:"));

    let requests = accounting_state
        .requests
        .lock()
        .expect("request list lock should not be poisoned")
        .clone();

    assert_eq!(requests.len(), 1);

    let request = &requests[0];
    assert_eq!(request["schema"], "svc-storage.usage-events.v1");
    assert_eq!(request["wallet_txid"], "tx_accounting_export");
    assert_eq!(request["source_service"], "svc-storage");

    let events = request["events"]
        .as_array()
        .expect("events should be an array");

    assert_eq!(events.len(), 3);

    assert!(events.iter().any(|event| {
        event["tenant"] == 7
            && event["subject"] == "svc_storage_provider"
            && event["metric_kind"] == "bytes_stored"
            && event["value"] == OBJECT_BYTES.len()
            && event["source_service"] == "svc-storage"
            && event["region"] == "us-central"
            && event["route"] == "/paid/o"
    }));

    assert!(events.iter().any(|event| {
        event["tenant"] == 7
            && event["subject"] == "svc_storage_provider"
            && event["metric_kind"] == "request_ok"
            && event["value"] == 1
    }));

    assert!(events.iter().any(|event| {
        event["tenant"] == 7
            && event["subject"] == "svc_storage_provider"
            && event["metric_kind"] == "pin_seconds"
            && event["value"] == 60
    }));

    let idempotency_keys = accounting_state
        .idempotency_keys
        .lock()
        .expect("idempotency key list lock should not be poisoned")
        .clone();

    assert_eq!(idempotency_keys, vec![export_key.to_string()]);

    clear_accounting_export();
}
