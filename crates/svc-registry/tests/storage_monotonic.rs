// crates/svc-registry/tests/storage_monotonic.rs
use std::sync::Arc;
use std::time::Duration;

use svc_registry::storage::{inmem::InMemoryStore, RegistryStore};
use tokio::time::timeout;
use tokio_stream::StreamExt; // for .next()

#[tokio::test]
async fn head_is_monotonic_and_events_fire() {
    // Fresh in-memory store
    let store = Arc::new(InMemoryStore::new());

    // Head starts at version 0
    let h0 = store.head().await;
    assert_eq!(h0.version, 0);

    // Subscribe to events as a BroadcastStream
    let mut rx = store.subscribe();

    // Commit #1
    let h1 = store
        .commit("b3:abc".to_string())
        .await
        .expect("commit #1 ok");
    assert_eq!(h1.version, 1);

    // Expect an event within 500 ms
    let evt1 = timeout(Duration::from_millis(500), rx.next())
        .await
        .expect("event delivery timed out");
    assert!(evt1.is_some(), "stream ended unexpectedly");
    // If you want to assert it wasn't an error from channel:
    assert!(evt1.unwrap().is_ok(), "recv error on broadcast stream");

    // Head should now be 1
    let head_now = store.head().await;
    assert_eq!(head_now.version, 1);

    // Commit #2
    let h2 = store
        .commit("b3:def".to_string())
        .await
        .expect("commit #2 ok");
    assert_eq!(h2.version, 2);

    // Another event should arrive
    let evt2 = timeout(Duration::from_millis(500), rx.next())
        .await
        .expect("event delivery timed out (second)");
    assert!(evt2.is_some(), "stream ended unexpectedly (second)");
    assert!(evt2.unwrap().is_ok(), "recv error on broadcast stream (second)");
}
