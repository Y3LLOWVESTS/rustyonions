//! RO:WHAT — Route tests for GET /paid/o/estimate.
//! RO:WHY — Pillar 12; Concerns: ECON/DX/GOV. Clients need preflight pricing before creating wallet holds.
//! RO:INTERACTS — svc_storage::http::server, policy::economics, configs/roc-economics.toml.
//! RO:INVARIANTS — estimate route is read-only; shares pricing with /paid/o; no wallet/ledger/accounting mutation.
//! RO:METRICS — none.
//! RO:CONFIG — RON_STORAGE_ROC_ECONOMICS_PATH, RON_STORAGE_ROC_ECONOMICS_ACTION.
//! RO:SECURITY — no bearer tokens, receipts, account IDs, object bytes, or external network.
//! RO:TEST — cargo test -p svc-storage --test paid_write_estimate.

use std::{
    env, fs,
    path::PathBuf,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    body::{to_bytes, Body},
    http::{Method, Request, StatusCode},
    Router,
};
use serde_json::Value;
use svc_storage::{
    http::{extractors::AppState, server::build_router},
    policy::economics::{ENV_ROC_ECONOMICS_ACTION, ENV_ROC_ECONOMICS_PATH},
    storage::{MemoryStorage, Storage},
};
use tokio::sync::Mutex;
use tower::ServiceExt;

const CHECKED_IN_ECONOMICS: &str = include_str!("../../../configs/roc-economics.toml");

static ENV_LOCK: Mutex<()> = Mutex::const_new(());

fn storage_app() -> Router {
    let store: Arc<dyn Storage> = Arc::new(MemoryStorage::default());
    let state = AppState { store };

    build_router().with_state(state)
}

#[tokio::test]
async fn legacy_estimate_preserves_beta_bytes_pricing_when_policy_unset() {
    let _guard = ENV_LOCK.lock().await;
    clear_economics_env();

    let (status, body) = send(storage_app(), estimate_request("48")).await;

    assert_eq!(status, StatusCode::OK);

    let json = json_body(&body);
    assert_eq!(json["schema"], "svc-storage.paid-storage-estimate.v1");
    assert_eq!(json["route"], "/paid/o");
    assert_eq!(json["action"], "paid_storage_put");
    assert_eq!(json["asset"], "roc");
    assert_eq!(json["bytes"], 48);
    assert_eq!(json["amount_minor"], "48");
    assert_eq!(json["minimum_hold_minor"], "48");
    assert_eq!(json["pricing_mode"], "legacy");
    assert!(json["economics_policy_path"].is_null());

    clear_economics_env();
}

#[tokio::test]
async fn economics_estimate_uses_checked_in_roc_economics_policy() {
    let _guard = ENV_LOCK.lock().await;
    clear_economics_env();

    let path = write_temp_policy(CHECKED_IN_ECONOMICS);
    env::set_var(ENV_ROC_ECONOMICS_PATH, &path);

    let (status, body) = send(storage_app(), estimate_request("48")).await;

    assert_eq!(status, StatusCode::OK);

    let json = json_body(&body);
    assert_eq!(json["schema"], "svc-storage.paid-storage-estimate.v1");
    assert_eq!(json["route"], "/paid/o");
    assert_eq!(json["action"], "paid_storage_put");
    assert_eq!(json["asset"], "roc");
    assert_eq!(json["bytes"], 48);
    assert_eq!(json["amount_minor"], "84");
    assert_eq!(json["minimum_hold_minor"], "84");
    assert_eq!(json["pricing_mode"], "roc-economics");
    assert_eq!(
        json["economics_policy_path"],
        path.to_string_lossy().to_string()
    );

    clear_economics_env();
    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn economics_estimate_honors_action_override() {
    let _guard = ENV_LOCK.lock().await;
    clear_economics_env();

    let path = write_temp_policy(CHECKED_IN_ECONOMICS);
    env::set_var(ENV_ROC_ECONOMICS_PATH, &path);
    env::set_var(ENV_ROC_ECONOMICS_ACTION, "paid_content_view");

    let (status, body) = send(storage_app(), estimate_request("48")).await;

    assert_eq!(status, StatusCode::OK);

    let json = json_body(&body);
    assert_eq!(json["action"], "paid_content_view");
    assert_eq!(json["asset"], "roc");
    assert_eq!(json["bytes"], 48);
    assert_eq!(json["amount_minor"], "5");
    assert_eq!(json["minimum_hold_minor"], "5");
    assert_eq!(json["pricing_mode"], "roc-economics");

    clear_economics_env();
    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn estimate_rejects_missing_bytes_query() {
    let _guard = ENV_LOCK.lock().await;
    clear_economics_env();

    let (status, body) = send(storage_app(), get("/paid/o/estimate")).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);

    let json = json_body(&body);
    assert_eq!(json["error"], "bad_request");
    assert_eq!(json["reason"], "missing required query parameter: bytes");

    clear_economics_env();
}

#[tokio::test]
async fn estimate_rejects_invalid_bytes_query() {
    let _guard = ENV_LOCK.lock().await;
    clear_economics_env();

    let (status, body) = send(storage_app(), estimate_request("not-a-number")).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);

    let json = json_body(&body);
    assert_eq!(json["error"], "bad_request");
    assert_eq!(json["reason"], "bytes must be an unsigned integer");

    clear_economics_env();
}

#[tokio::test]
async fn explicit_bad_policy_path_fails_closed() {
    let _guard = ENV_LOCK.lock().await;
    clear_economics_env();

    env::set_var(
        ENV_ROC_ECONOMICS_PATH,
        "/definitely/not/a/real/roc-economics.toml",
    );

    let (status, body) = send(storage_app(), estimate_request("48")).await;

    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);

    let json = json_body(&body);
    assert_eq!(json["error"], "config_error");
    assert!(json["reason"]
        .as_str()
        .expect("reason should be a string")
        .contains("failed to read"));

    clear_economics_env();
}

async fn send(router: Router, request: Request<Body>) -> (StatusCode, Vec<u8>) {
    let response = router
        .oneshot(request)
        .await
        .expect("router request should complete");

    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should read")
        .to_vec();

    (status, body)
}

fn estimate_request(bytes: &str) -> Request<Body> {
    get(&format!("/paid/o/estimate?bytes={bytes}"))
}

fn get(uri: &str) -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri(uri)
        .body(Body::empty())
        .expect("request should build")
}

fn json_body(bytes: &[u8]) -> Value {
    serde_json::from_slice::<Value>(bytes).expect("response body should be JSON")
}

fn clear_economics_env() {
    env::remove_var(ENV_ROC_ECONOMICS_PATH);
    env::remove_var(ENV_ROC_ECONOMICS_ACTION);
}

fn write_temp_policy(contents: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();

    let path = env::temp_dir().join(format!(
        "rustyonions-roc-economics-estimate-{}-{nanos}.toml",
        std::process::id()
    ));

    fs::write(&path, contents).expect("temp economics policy should write");
    path
}
