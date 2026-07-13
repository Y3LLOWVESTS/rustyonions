//! Legacy HTTP moderation enforcement tests.
//!
//! GET and HEAD refusal must happen before any storage lookup. The policy
//! decision comes from `ron-policy`; this test adds no parallel precedence
//! or acceptance rules.

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use axum::{
    body::{Body, Bytes},
    http::{Method, Request, StatusCode},
    Router,
};
use ron_policy::{B3Id, ModerationPolicy};
use svc_storage::{
    errors::StorageError,
    http::{extractors::AppState, server::build_router_with_moderation},
    storage::{DynStorage, HeadMeta, PruneOutcome, Storage},
};
use tower::ServiceExt;

const BLOCKED_CID: &str = "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

#[derive(Default)]
struct TrackingStorage {
    calls: AtomicUsize,
}

impl TrackingStorage {
    fn call_count(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }

    fn touched<T>(&self) -> Result<T, StorageError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Err(StorageError::NotFound)
    }
}

#[async_trait::async_trait]
impl Storage for TrackingStorage {
    async fn put(&self, _cid: &str, _data: Bytes) -> Result<(), StorageError> {
        self.touched()
    }

    async fn exists(&self, _cid: &str) -> Result<bool, StorageError> {
        self.touched()
    }

    async fn head(&self, _cid: &str) -> Result<HeadMeta, StorageError> {
        self.touched()
    }

    async fn prune(&self, _object: &B3Id) -> Result<PruneOutcome, StorageError> {
        self.touched()
    }

    async fn get_full(&self, _cid: &str) -> Result<Bytes, StorageError> {
        self.touched()
    }

    async fn get_range(
        &self,
        _cid: &str,
        _start: u64,
        _end_inclusive: u64,
    ) -> Result<(Bytes, u64), StorageError> {
        self.touched()
    }
}

fn blocked_app(tracking: Arc<TrackingStorage>) -> Router {
    let object: B3Id = BLOCKED_CID.parse().expect("blocked test CID must parse");

    let mut moderation = ModerationPolicy::default();

    assert!(moderation.insert_local_block(object));

    let store: DynStorage = tracking;

    build_router_with_moderation(Arc::new(moderation)).with_state(AppState { store })
}

fn request(method: Method) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(format!("/o/{BLOCKED_CID}"))
        .body(Body::empty())
        .expect("legacy request should build")
}

#[tokio::test]
async fn blocked_get_and_head_refuse_before_storage() {
    let tracking = Arc::new(TrackingStorage::default());

    let app = blocked_app(tracking.clone());

    let get_response = app
        .clone()
        .oneshot(request(Method::GET))
        .await
        .expect("GET request should complete");

    assert_eq!(get_response.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        tracking.call_count(),
        0,
        "blocked GET must not reach storage"
    );

    let head_response = app
        .oneshot(request(Method::HEAD))
        .await
        .expect("HEAD request should complete");

    assert_eq!(head_response.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        tracking.call_count(),
        0,
        "blocked HEAD must not reach storage"
    );
}
