//! RO:WHAT — Exact local object-byte prune tests.
//!
//! RO:WHY — `BUILD_PLAN_Z` Phase 10 requires physical removal before the CLI may claim pruning.
//!
//! RO:INTERACTS — The live object-safe `Storage` trait and `MemoryStorage` backend.
//!
//! RO:INVARIANTS — Exact canonical B3 identity; truthful removed/not-found result; unrelated bytes survive.
//!
//! RO:SECURITY — Local bytes only; no provider, index, reward, wallet, ledger, or policy mutation.
//!
//! RO:TEST — `cargo test -p svc-storage --test storage_prune`.

use bytes::Bytes;
use ron_policy::B3Id;
use svc_storage::{
    errors::StorageError,
    storage::{MemoryStorage, PruneOutcome, Storage},
};

fn object_for(bytes: &[u8]) -> B3Id {
    format!("b3:{}", blake3::hash(bytes).to_hex())
        .parse()
        .expect("BLAKE3 output must form a canonical B3 object identifier")
}

#[tokio::test]
async fn prune_removes_exact_bytes_and_all_read_surfaces() {
    let store = MemoryStorage::default();
    let body = Bytes::from_static(b"physical prune target");
    let object = object_for(&body);

    store
        .put(object.as_str(), body.clone())
        .await
        .expect("object write should succeed");

    assert!(store
        .exists(object.as_str())
        .await
        .expect("existence lookup should succeed"));
    assert_eq!(
        store
            .head(object.as_str())
            .await
            .expect("metadata should exist before prune")
            .len,
        u64::try_from(body.len()).expect("test body length must fit u64")
    );
    assert_eq!(
        store
            .get_full(object.as_str())
            .await
            .expect("bytes should exist before prune"),
        body
    );

    let outcome = store
        .prune(&object)
        .await
        .expect("prune operation should succeed");

    assert_eq!(
        outcome,
        PruneOutcome::Removed {
            bytes: u64::try_from(body.len()).expect("test body length must fit u64"),
        }
    );

    assert!(!store
        .exists(object.as_str())
        .await
        .expect("post-prune existence lookup should succeed"));
    assert!(matches!(
        store.head(object.as_str()).await,
        Err(StorageError::NotFound)
    ));
    assert!(matches!(
        store.get_full(object.as_str()).await,
        Err(StorageError::NotFound)
    ));
    assert!(matches!(
        store.get_range(object.as_str(), 0, 0).await,
        Err(StorageError::NotFound)
    ));
}

#[tokio::test]
async fn repeated_prune_reports_not_found_instead_of_fake_success() {
    let store = MemoryStorage::default();
    let body = Bytes::from_static(b"one-time prune target");
    let object = object_for(&body);

    store
        .put(object.as_str(), body)
        .await
        .expect("object write should succeed");

    assert!(matches!(
        store.prune(&object).await,
        Ok(PruneOutcome::Removed { .. })
    ));

    assert_eq!(
        store
            .prune(&object)
            .await
            .expect("repeated prune lookup should succeed"),
        PruneOutcome::NotFound
    );
}

#[tokio::test]
async fn prune_uses_exact_full_b3_identity() {
    let store = MemoryStorage::default();

    let removed_body = Bytes::from_static(b"remove only this object");
    let retained_body = Bytes::from_static(b"retain this neighboring object");

    let removed_object = object_for(&removed_body);
    let retained_object = object_for(&retained_body);

    store
        .put(removed_object.as_str(), removed_body)
        .await
        .expect("removed-object write should succeed");
    store
        .put(retained_object.as_str(), retained_body.clone())
        .await
        .expect("retained-object write should succeed");

    assert!(matches!(
        store.prune(&removed_object).await,
        Ok(PruneOutcome::Removed { .. })
    ));

    assert!(!store
        .exists(removed_object.as_str())
        .await
        .expect("removed-object existence lookup should succeed"));
    assert!(store
        .exists(retained_object.as_str())
        .await
        .expect("retained-object existence lookup should succeed"));
    assert_eq!(
        store
            .get_full(retained_object.as_str())
            .await
            .expect("unrelated object must remain readable"),
        retained_body
    );
}
