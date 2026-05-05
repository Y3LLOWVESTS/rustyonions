//! I-4 — Content-addressing roundtrip against a mock gateway.
//!
//! RO:WHAT — Spins up a tiny Axum server that exposes just enough of the
//!           gateway surface for `storage_put` / `storage_get` to work.
//! RO:WHY  — Proves the SDK can speak to a gateway-like surface, roundtrip
//!           blobs by `b3:<hex>`, and honor `SdkConfig` / `Transport` wiring.
//! RO:INVARIANTS — Address format is `b3:<64 hex>`; digest matches BLAKE3;
//!                 `storage_get` returns the exact blob we stored.
//! RO:SECURITY — Capability is test-only and opaque; no secrets are logged.
//! RO:TEST — cargo test -p ron-app-sdk --test i_4_content_addressing.

use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::{
    body::Bytes as BodyBytes,
    extract::{Path, State},
    http::StatusCode,
    routing::post,
    Router,
};
use bytes::Bytes;
use tokio::net::TcpListener;

use ron_app_sdk::{check_ready, Capability, RonAppSdk, SdkConfig, Timeouts, Transport};

/// In-memory CAS store keyed by `b3:<hex>` strings.
///
/// We only need this inside the test to emulate the gateway's storage plane.
type Store = Arc<Mutex<HashMap<String, Vec<u8>>>>;

/// Construct a simple capability token for tests.
///
/// Real macaroon verification is not part of this mock. The SDK treats
/// capabilities as opaque at this layer.
fn mk_cap() -> Capability {
    let now: u64 = 1_700_000_000;
    Capability {
        subject: "itest-user".to_owned(),
        scope: "storage:rw".to_owned(),
        issued_at: now,
        expires_at: now + 3_600,
        caveats: Vec::new(),
    }
}

/// Handler for `POST /put`.
///
/// Body: raw blob bytes.
/// Response: `b3:<hex>` as UTF-8 text.
async fn handle_storage_put(State(store): State<Store>, body: BodyBytes) -> (StatusCode, String) {
    let blob = body.to_vec();
    let digest = blake3::hash(&blob);
    let addr = format!("b3:{}", hex::encode(digest.as_bytes()));

    {
        let mut guard = store.lock().expect("store mutex poisoned");
        guard.insert(addr.clone(), blob);
    }

    (StatusCode::OK, addr)
}

/// Handler for `POST /o/:addr`.
///
/// Body is ignored because the SDK currently sends an empty payload for
/// storage reads. Response is raw bytes for the given content address.
async fn handle_storage_get(
    State(store): State<Store>,
    Path(addr): Path<String>,
) -> (StatusCode, Bytes) {
    let maybe = {
        let guard = store.lock().expect("store mutex poisoned");
        guard.get(&addr).cloned()
    };

    match maybe {
        Some(buf) => (StatusCode::OK, Bytes::from(buf)),
        None => (StatusCode::NOT_FOUND, Bytes::new()),
    }
}

/// Start the mock gateway on a random local port.
async fn spawn_mock_gateway(store: Store) -> SocketAddr {
    let router = Router::new()
        .route("/put", post(handle_storage_put))
        .route("/o/:addr", post(handle_storage_get))
        .with_state(store);

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock gateway");

    let addr = listener.local_addr().expect("mock gateway local addr");

    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("mock gateway serve");
    });

    addr
}

#[tokio::test]
async fn oap_content_addressing_roundtrip() {
    let store: Store = Arc::new(Mutex::new(HashMap::new()));
    let addr = spawn_mock_gateway(store.clone()).await;
    let base_url = format!("http://{addr}");

    let cfg = SdkConfig {
        gateway_addr: base_url,
        transport: Transport::Tls,
        timeouts: Timeouts {
            connect: Duration::from_millis(500),
            read: Duration::from_millis(1_000),
            write: Duration::from_millis(1_000),
        },
        overall_timeout: Duration::from_millis(5_000),
        ..Default::default()
    };

    let ready = check_ready(&cfg);
    assert!(ready.is_ready(), "SDK ready check failed: {ready:?}");

    let sdk = RonAppSdk::new(cfg).await.expect("construct RonAppSdk");

    let cap = mk_cap();
    let deadline = Duration::from_millis(1_000);

    let blob = Bytes::from_static(b"hello-oap-content-addressing");
    let content_id = sdk
        .storage_put(cap.clone(), blob.clone(), deadline, None)
        .await
        .expect("storage_put should succeed");

    let content_id_str = content_id.as_str();
    assert!(
        content_id_str.starts_with("b3:"),
        "content address should start with 'b3:', got {content_id_str}",
    );

    let hex_part = &content_id_str[3..];
    assert_eq!(
        hex_part.len(),
        64,
        "digest hex should be 64 chars, got {}",
        hex_part.len()
    );

    let expected_hex = hex::encode(blake3::hash(&blob).as_bytes());
    assert_eq!(
        hex_part, expected_hex,
        "content ID digest did not match BLAKE3(blob)"
    );

    {
        let guard = store.lock().expect("store mutex poisoned");
        let stored = guard
            .get(content_id_str)
            .expect("mock gateway did not store blob");

        assert_eq!(
            stored.as_slice(),
            blob.as_ref(),
            "server-side blob mismatch"
        );
    }

    let roundtrip = sdk
        .storage_get(cap, content_id_str, deadline)
        .await
        .expect("storage_get should succeed");

    assert_eq!(roundtrip.as_ref(), blob.as_ref(), "roundtrip blob mismatch");
}
