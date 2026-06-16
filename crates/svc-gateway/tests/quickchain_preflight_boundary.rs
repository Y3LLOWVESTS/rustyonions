#![allow(clippy::missing_panics_doc, clippy::must_use_candidate)]

//! RO:WHAT — `QuickChain` Phase-0 boundary tests for svc-gateway.
//! RO:WHY — P6/P12; concerns: SEC/ECON/GOV. Gateway must remain a proxy/admission boundary, not economic or chain authority.
//! RO:INTERACTS — `routes::build_router`, product/paid/app/object proxy header filters, mock omnigate.
//! RO:INVARIANTS — no root/finality/validator/bridge headers; no direct ledger/QuickChain routes; wallet paths are proxy-only.
//! RO:METRICS — exercises existing gateway HTTP/correlation layers.
//! RO:CONFIG — `SVC_GATEWAY_OMNIGATE_BASE_URL`, `SVC_GATEWAY_BIND_ADDR`.
//! RO:SECURITY — caller-supplied authority-looking claims are stripped before upstream.
//! RO:TEST — `cargo test -p svc-gateway --test quickchain_preflight_boundary`.

use std::{collections::HashMap, net::SocketAddr, time::Duration};

use axum::{
    body::{Body, Bytes},
    http::{HeaderMap, Method, Request, StatusCode, Uri},
    routing::get,
    Json, Router,
};
use once_cell::sync::OnceCell;
use serde::Serialize;
use serde_json::Value;
use svc_gateway::{config::Config, observability::metrics, routes, state::AppState};
use tokio::{net::TcpListener, sync::Mutex};
use tower::ServiceExt;

static ENV_LOCK: Mutex<()> = Mutex::const_new(());

#[derive(Debug, Serialize)]
struct HeaderBoundaryEcho {
    method: String,
    path: String,
    query: HashMap<String, String>,
    body_len: usize,
    authorization: Option<String>,
    passport: Option<String>,
    wallet_account: Option<String>,
    idempotency_key: Option<String>,
    wallet_receipt_hash: Option<String>,
    state_root: Option<String>,
    receipt_root: Option<String>,
    checkpoint_hash: Option<String>,
    validator_signature: Option<String>,
    bridge_authorized: Option<String>,
    finality: Option<String>,
    operation_id: Option<String>,
    account_sequence: Option<String>,
    quickchain_claim: Option<String>,
    ledger_mutation: Option<String>,
}

fn test_metrics_handles() -> metrics::MetricsHandles {
    static CELL: OnceCell<metrics::MetricsHandles> = OnceCell::new();
    CELL.get_or_init(|| metrics::register().expect("register metrics once for test process"))
        .clone()
}

fn test_state_with_omnigate_base(base: impl Into<String>) -> AppState {
    let mut cfg = Config::default();
    cfg.upstreams.omnigate_base_url = base.into();
    cfg.server.bind_addr = "127.0.0.1:0".to_string();

    AppState::new(cfg, test_metrics_handles())
}

async fn start_dummy_omnigate() -> SocketAddr {
    async fn healthz() -> &'static str {
        "ok"
    }

    async fn echo_handler(
        method: Method,
        uri: Uri,
        headers: HeaderMap,
        body: Bytes,
    ) -> (StatusCode, Json<Value>) {
        (
            StatusCode::OK,
            Json(serde_json::json!(HeaderBoundaryEcho {
                method: method.as_str().to_owned(),
                path: uri.path().to_owned(),
                query: parse_query(&uri),
                body_len: body.len(),
                authorization: grab(&headers, "authorization"),
                passport: grab(&headers, "x-ron-passport"),
                wallet_account: grab(&headers, "x-ron-wallet-account"),
                idempotency_key: grab(&headers, "idempotency-key"),
                wallet_receipt_hash: grab(&headers, "x-ron-wallet-receipt-hash"),
                state_root: grab(&headers, "x-ron-state-root"),
                receipt_root: grab(&headers, "x-ron-receipt-root"),
                checkpoint_hash: grab(&headers, "x-ron-checkpoint-hash"),
                validator_signature: grab(&headers, "x-ron-validator-signature"),
                bridge_authorized: grab(&headers, "x-ron-bridge-authorized"),
                finality: grab(&headers, "x-ron-finality"),
                operation_id: grab(&headers, "x-ron-operation-id"),
                account_sequence: grab(&headers, "x-ron-account-sequence"),
                quickchain_claim: grab(&headers, "x-ron-quickchain-claim"),
                ledger_mutation: grab(&headers, "x-ron-ledger-mutation"),
            })),
        )
    }

    let router = Router::new()
        .route("/healthz", get(healthz))
        .fallback(echo_handler);

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind dummy omnigate");
    let addr = listener.local_addr().expect("dummy omnigate local_addr");

    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("dummy omnigate serve");
    });

    wait_for_health(format!("http://{addr}/healthz")).await;
    addr
}

async fn start_gateway(omnigate_addr: SocketAddr) -> SocketAddr {
    std::env::set_var(
        "SVC_GATEWAY_OMNIGATE_BASE_URL",
        format!("http://{omnigate_addr}"),
    );
    std::env::set_var("SVC_GATEWAY_BIND_ADDR", "127.0.0.1:0");

    let cfg = Config::load().expect("load gateway config");
    let state = AppState::new(cfg.clone(), test_metrics_handles());
    let router = routes::build_router(&state);

    let listener = TcpListener::bind(&cfg.server.bind_addr)
        .await
        .expect("bind gateway");
    let gateway_addr = listener.local_addr().expect("gateway local_addr");

    tokio::spawn(async move {
        axum::serve(listener, router).await.expect("gateway serve");
    });

    wait_for_health(format!("http://{gateway_addr}/healthz")).await;
    gateway_addr
}

#[tokio::test]
async fn product_proxy_strips_caller_supplied_quickchain_authority_headers() {
    let _guard = ENV_LOCK.lock().await;
    clear_gateway_env();

    let omnigate_addr = start_dummy_omnigate().await;
    let gateway_addr = start_gateway(omnigate_addr).await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{gateway_addr}/wallet/hold"))
        .header("authorization", "Bearer dev")
        .header("x-ron-passport", "passport:main:visitor-b")
        .header("x-ron-wallet-account", "acct_visitor_b")
        .header("idempotency-key", "idem-wallet-hold-qc-boundary")
        .header(
            "x-ron-wallet-receipt-hash",
            "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        )
        .header(
            "x-ron-state-root",
            "b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        )
        .header(
            "x-ron-receipt-root",
            "b3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
        )
        .header("x-ron-checkpoint-hash", "fake-checkpoint")
        .header("x-ron-validator-signature", "fake-validator-signature")
        .header("x-ron-bridge-authorized", "true")
        .header("x-ron-finality", "finalized")
        .header("x-ron-operation-id", "client-supplied-op")
        .header("x-ron-account-sequence", "42")
        .header("x-ron-quickchain-claim", "client-supplied-root-truth")
        .header("x-ron-ledger-mutation", "transfer")
        .json(&serde_json::json!({
            "account": "acct_visitor_b",
            "amount_minor": "10",
            "asset": "roc"
        }))
        .send()
        .await
        .expect("gateway wallet hold proxy response");

    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = resp.json().await.expect("parse header boundary echo");
    assert_eq!(body["method"], "POST");
    assert_eq!(body["path"], "/v1/wallet/hold");
    assert_eq!(body["authorization"], "Bearer dev");
    assert_eq!(body["passport"], "passport:main:visitor-b");
    assert_eq!(body["wallet_account"], "acct_visitor_b");
    assert_eq!(body["idempotency_key"], "idem-wallet-hold-qc-boundary");
    assert_eq!(
        body["wallet_receipt_hash"],
        "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    );

    for denied in [
        "state_root",
        "receipt_root",
        "checkpoint_hash",
        "validator_signature",
        "bridge_authorized",
        "finality",
        "operation_id",
        "account_sequence",
        "quickchain_claim",
        "ledger_mutation",
    ] {
        assert!(
            body[denied].is_null(),
            "gateway must strip caller-supplied authority header echoed as {denied}: {body}"
        );
    }

    clear_gateway_env();
}

#[tokio::test]
async fn unsupported_quickchain_ledger_bridge_root_routes_fail_closed_at_gateway() {
    let state = test_state_with_omnigate_base("http://127.0.0.1:1");
    let app = routes::build_router(&state);

    for (method, uri) in [
        (Method::POST, "/quickchain/root"),
        (Method::POST, "/quickchain/checkpoint"),
        (Method::POST, "/quickchain/validator"),
        (Method::POST, "/quickchain/settle"),
        (Method::GET, "/quickchain/state-root"),
        (Method::GET, "/quickchain/receipt-root"),
        (Method::POST, "/ledger/issue"),
        (Method::POST, "/ledger/transfer"),
        (Method::POST, "/ledger/burn"),
        (Method::POST, "/ledger/hold"),
        (Method::POST, "/ledger/capture"),
        (Method::POST, "/ledger/release"),
        (Method::POST, "/wallet/issue"),
        (Method::POST, "/wallet/transfer"),
        (Method::POST, "/wallet/burn"),
        (Method::POST, "/wallet/capture"),
        (Method::POST, "/wallet/release"),
        (Method::POST, "/bridge/anchor"),
        (Method::POST, "/bridge/settle"),
        (Method::POST, "/anchors"),
        (Method::POST, "/validators"),
        (Method::POST, "/staking/delegate"),
        (Method::POST, "/liquidity/add"),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(method.clone())
                    .uri(uri)
                    .body(Body::empty())
                    .expect("request builds"),
            )
            .await
            .expect("router responds");

        assert!(
            !response.status().is_success(),
            "svc-gateway must not expose QuickChain/ledger/bridge authority route {method} {uri}; got {}",
            response.status()
        );
        assert_ne!(
            response.status(),
            StatusCode::BAD_GATEWAY,
            "route {method} {uri} matched and attempted upstream proxying; it must fail closed locally"
        );
    }
}

#[test]
fn gateway_source_does_not_import_wallet_ledger_or_quickchain_runtime_authority() {
    let route_sources = [
        ("routes/app.rs", include_str!("../src/routes/app.rs")),
        (
            "routes/objects.rs",
            include_str!("../src/routes/objects.rs"),
        ),
        (
            "routes/paid_storage.rs",
            include_str!("../src/routes/paid_storage.rs"),
        ),
        (
            "routes/product.rs",
            include_str!("../src/routes/product.rs"),
        ),
        ("state.rs", include_str!("../src/state.rs")),
    ];

    for (name, source) in route_sources {
        let code = strip_rust_comments(source);

        for forbidden in [
            "ron_ledger::",
            "svc_wallet::",
            "ron_accounting::",
            "svc_rewarder::",
            "use ron_ledger",
            "use svc_wallet",
            "quickchain::",
            "ValidatorSet",
            "RootProducer",
            "CheckpointWriter",
            "ExternalSettlement",
        ] {
            assert!(
                !code.contains(forbidden),
                "{name} must not import or implement runtime authority term {forbidden}"
            );
        }
    }
}

#[test]
fn route_source_keeps_wallet_surface_limited_to_proxy_hold_and_display_balance() {
    let product_source = include_str!("../src/routes/product.rs");

    assert!(
        product_source.contains(r#"/wallet/:account/balance"#),
        "gateway should expose wallet balance display proxy"
    );
    assert!(
        product_source.contains(r#"/wallet/hold"#),
        "gateway should expose wallet hold proxy"
    );

    for forbidden_route in [
        r#""/wallet/issue""#,
        r#""/wallet/transfer""#,
        r#""/wallet/burn""#,
        r#""/wallet/capture""#,
        r#""/wallet/release""#,
        r#""/ledger/""#,
        r#""/quickchain/""#,
        r#""/bridge/""#,
        r#""/validators""#,
        r#""/anchors""#,
    ] {
        assert!(
            !product_source.contains(forbidden_route),
            "gateway product router must not expose forbidden route literal {forbidden_route}"
        );
    }
}

async fn wait_for_health(url: String) {
    let client = reqwest::Client::new();

    for _ in 0..50 {
        if let Ok(resp) = client.get(&url).send().await {
            if resp.status().is_success() {
                return;
            }
        }

        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    panic!("service did not become healthy at {url}");
}

fn parse_query(uri: &Uri) -> HashMap<String, String> {
    uri.query()
        .unwrap_or_default()
        .split('&')
        .filter(|pair| !pair.is_empty())
        .filter_map(|pair| {
            let (key, value) = pair.split_once('=')?;
            Some((key.to_owned(), value.to_owned()))
        })
        .collect()
}

fn grab(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

fn strip_rust_comments(source: &str) -> String {
    source
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !(trimmed.starts_with("//") || trimmed.starts_with("///") || trimmed.starts_with("//!"))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn clear_gateway_env() {
    std::env::remove_var("SVC_GATEWAY_OMNIGATE_BASE_URL");
    std::env::remove_var("SVC_GATEWAY_STORAGE_BASE_URL");
    std::env::remove_var("SVC_GATEWAY_BIND_ADDR");
}
