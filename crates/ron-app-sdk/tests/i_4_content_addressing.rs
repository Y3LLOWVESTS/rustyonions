//! I-4 — Content-addressing roundtrip against a mock gateway.
//!
//! RO:WHAT — Spins up a tiny Axum server that exposes just enough of the
//!           gateway surface for `storage_put` / `storage_get` to work.
//! RO:WHY  — Proves the SDK can:
//!             - speak OAP over HTTP to a gateway-like surface,
//!             - roundtrip blobs via content-addressed IDs (`b3:<hex>`),
//!             - honour the `SdkConfig` / `Transport` wiring.
//! RO:INVARIANTS —
//!   - Address format is `b3:<64 hex>`.
//!   - Returned digest matches `blake3(blob)`.
//!   - `storage_get` returns the exact blob we stored.

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

use blake3;
use hex;

use ron_app_sdk::{check_ready, Capability, RonAppSdk, SdkConfig, Timeouts, Transport};

/// In-memory CAS store keyed by `b3:<hex>` strings.
///
/// We only need this inside the test to emulate the gateway's storage plane.
type Store = Arc<Mutex<HashMap<String, Vec<u8>>>>;

/// Construct a very simple capability token for tests.
///
/// We don't care about real macaroon semantics here — just that the SDK can
/// serialize and send a `Capability` header.
fn mk_cap() -> Capability {
    // These fields mirror the simple header used elsewhere in tests.
    // No real validation happens in this mock; the transport currently
    // treats capabilities as opaque.
    let now: u64 = 1_700_000_000;
    Capability {
        subject: "itest-user".to_string(),
        scope: "storage:rw".to_string(),
        issued_at: now,
        expires_at: now + 3600,
        caveats: Vec::new(),
    }
}

/// Handler for `POST /put`.
///
/// - Body: raw blob bytes.
/// - Response: `b3:<hex>` as UTF-8 text.
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
/// - Body: ignored (SDK sends an empty body for `storage_get`).
/// - Response: raw bytes for the given content address, or 404 if missing.
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

/// Spin up a minimal Axum-based mock gateway.
///
/// Returns:
///   - `base_url` suitable for `SdkConfig.gateway_addr` (e.g. `http://127.0.0.1:12345`)
///   - shared `Store` so the test can peek into the server-side CAS
///   - join handle for the server task (we just let it run for the test lifetime)
async fn spawn_mock_gateway() -> (String, Store, tokio::task::JoinHandle<()>) {
    let store: Store = Arc::new(Mutex::new(HashMap::new()));

    // Endpoints chosen to match `planes::storage`:
    // - storage_put → POST /put
    // - storage_get → POST /o/{addr}
    let app = Router::new()
        .route("/put", post(handle_storage_put))
        .route("/o/:addr", post(handle_storage_get))
        .with_state(store.clone());

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock gateway");
    let addr: SocketAddr = listener.local_addr().expect("local_addr");
    let base_url = format!("http://{}", addr);

    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("mock gateway server failed");
    });

    (base_url, store, server)
}

#[tokio::test]
async fn oap_content_addressing_roundtrip() {
    // 1) Start the mock gateway.
    let (base_url, store, _server) = spawn_mock_gateway().await;

    // 2) Build an SDK config that points at the mock gateway.
    //
    // We keep it close to the default posture but override:
    //   - gateway_addr
    //   - timeouts
    let mut cfg = SdkConfig::default();
    cfg.gateway_addr = base_url;
    cfg.transport = Transport::Tls;
    cfg.timeouts = Timeouts {
        connect: Duration::from_millis(500),
        read: Duration::from_millis(1_000),
        write: Duration::from_millis(1_000),
    };
    cfg.overall_timeout = Duration::from_millis(5_000);

    // 3) Ready check: prove config + transport wiring are sane.
    let ready = check_ready(&cfg);
    assert!(ready.is_ready(), "SDK ready check failed: {:?}", ready);

    // 4) Instantiate the SDK.
    let sdk = RonAppSdk::new(cfg).await.expect("construct RonAppSdk");

    let cap = mk_cap();
    let deadline = Duration::from_millis(1_000);

    // 5) PUT a blob via the storage plane.
    let blob = Bytes::from_static(b"hello-oap-content-addressing");
    let addr = sdk
        .storage_put(cap.clone(), blob.clone(), deadline, None)
        .await
        .expect("storage_put should succeed");

    // Ensure address format is `b3:<64 hex>`.
    let addr_str = addr.as_str();
    assert!(
        addr_str.starts_with("b3:"),
        "content address should start with 'b3:', got {addr_str}",
    );
    let hex_part = &addr_str[3..];
    assert_eq!(
        hex_part.len(),
        64,
        "digest hex should be 64 chars, got {}",
        hex_part.len()
    );

    // Verify digest matches blake3(blob).
    let expected_hex = hex::encode(blake3::hash(&blob).as_bytes());
    assert_eq!(
        hex_part, expected_hex,
        "content ID digest did not match BLAKE3(blob)"
    );

    // 6) Peek into the mock gateway's store and ensure it has the blob.
    {
        let guard = store.lock().expect("store mutex poisoned");
        let stored = guard
            .get(addr_str)
            .expect("mock gateway did not store blob");
        assert_eq!(
            stored.as_slice(),
            blob.as_ref(),
            "server-side blob mismatch"
        );
    }

    // 7) GET the blob back via the storage plane.
    let roundtrip = sdk
        .storage_get(cap, addr_str, deadline)
        .await
        .expect("storage_get should succeed");

    assert_eq!(roundtrip.as_ref(), blob.as_ref(), "roundtrip blob mismatch");
}
