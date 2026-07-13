//! RO:WHAT — Focused tests for bounded amnesia-first object storage.
//! RO:WHY — Phase 11 requires unknown bytes to remain RAM-only, bounded,
//! evictable, and non-durable by default.
//! RO:INTERACTS — `MemoryStorage`, `Storage`, exact B3 pruning, and capacity
//! enforcement.
//! RO:INVARIANTS — finite limits; FIFO eviction; no fake restart durability;
//! exact byte accounting after replacement and pruning.
//! RO:SECURITY — no filesystem, provider, moderation, reward, wallet, or ledger
//! authority.
//! RO:TEST — cargo test -p svc-storage --test amnesia_storage.

use axum::body::Bytes;
use ron_policy::B3Id;
use svc_storage::{
    errors::StorageError,
    storage::{MemoryStorage, MemoryStorageLimits, PruneOutcome, Storage, StorageResidency},
};

fn cid_for(bytes: &[u8]) -> String {
    format!("b3:{}", blake3::hash(bytes).to_hex())
}

fn exact_object(cid: &str) -> B3Id {
    cid.parse()
        .expect("test CID must be an exact canonical B3 identifier")
}

fn limits(max_objects: usize, max_bytes: u64) -> MemoryStorageLimits {
    MemoryStorageLimits::try_new(max_objects, max_bytes)
        .expect("test limits must be finite and non-zero")
}

#[test]
fn default_storage_is_finite_ephemeral_and_evictable() {
    let store = MemoryStorage::default();
    let configured = store.limits();

    assert_eq!(store.residency(), StorageResidency::EphemeralMemory);
    assert_eq!(store.residency().as_str(), "ephemeral_memory");
    assert!(store.is_evictable());

    assert!(configured.max_objects() > 0);
    assert!(configured.max_bytes() > 0);
    assert!(configured.max_objects() < usize::MAX);
    assert!(configured.max_bytes() < u64::MAX);

    assert_eq!(store.object_count(), 0);
    assert_eq!(store.total_bytes(), 0);
}

#[tokio::test]
async fn object_limit_evicts_the_oldest_entry() {
    let store = MemoryStorage::with_limits(limits(2, 1_024));

    let first = Bytes::from_static(b"first");
    let second = Bytes::from_static(b"second");
    let third = Bytes::from_static(b"third");

    let first_cid = cid_for(&first);
    let second_cid = cid_for(&second);
    let third_cid = cid_for(&third);

    store.put(&first_cid, first).await.unwrap();
    store.put(&second_cid, second).await.unwrap();
    store.put(&third_cid, third).await.unwrap();

    assert!(!store.exists(&first_cid).await.unwrap());
    assert!(store.exists(&second_cid).await.unwrap());
    assert!(store.exists(&third_cid).await.unwrap());
    assert_eq!(store.object_count(), 2);
}

#[tokio::test]
async fn byte_limit_evicts_until_the_new_object_fits() {
    let store = MemoryStorage::with_limits(limits(8, 6));

    let first = Bytes::from_static(b"1234");
    let second = Bytes::from_static(b"5678");
    let third = Bytes::from_static(b"90");

    let first_cid = cid_for(&first);
    let second_cid = cid_for(&second);
    let third_cid = cid_for(&third);

    store.put(&first_cid, first).await.unwrap();
    store.put(&second_cid, second).await.unwrap();

    assert!(!store.exists(&first_cid).await.unwrap());
    assert!(store.exists(&second_cid).await.unwrap());
    assert_eq!(store.total_bytes(), 4);

    store.put(&third_cid, third).await.unwrap();

    assert!(store.exists(&second_cid).await.unwrap());
    assert!(store.exists(&third_cid).await.unwrap());
    assert_eq!(store.total_bytes(), 6);
}

#[tokio::test]
async fn one_object_larger_than_the_cache_is_rejected_without_mutation() {
    let store = MemoryStorage::with_limits(limits(4, 3));
    let body = Bytes::from_static(b"four");
    let cid = cid_for(&body);

    let error = store
        .put(&cid, body)
        .await
        .expect_err("an object larger than the full cache must reject");

    assert!(matches!(error, StorageError::CapacityExceeded));
    assert_eq!(store.object_count(), 0);
    assert_eq!(store.total_bytes(), 0);
    assert!(!store.exists(&cid).await.unwrap());
}

#[tokio::test]
async fn replacing_an_exact_key_preserves_truthful_accounting_and_order() {
    let store = MemoryStorage::with_limits(limits(2, 6));

    let original_a = Bytes::from_static(b"aaa");
    let body_b = Bytes::from_static(b"bbb");
    let replacement_a = Bytes::from_static(b"aa");
    let body_c = Bytes::from_static(b"ccc");

    let cid_a = cid_for(&original_a);
    let cid_b = cid_for(&body_b);
    let cid_c = cid_for(&body_c);

    store.put(&cid_a, original_a).await.unwrap();
    store.put(&cid_b, body_b).await.unwrap();

    store.put(&cid_a, replacement_a.clone()).await.unwrap();

    assert_eq!(store.object_count(), 2);
    assert_eq!(store.total_bytes(), 5);
    assert_eq!(store.get_full(&cid_a).await.unwrap(), replacement_a);

    // Replacing A made it newest, so inserting C evicts B.
    store.put(&cid_c, body_c).await.unwrap();

    assert!(store.exists(&cid_a).await.unwrap());
    assert!(!store.exists(&cid_b).await.unwrap());
    assert!(store.exists(&cid_c).await.unwrap());
    assert_eq!(store.total_bytes(), 5);
}

#[tokio::test]
async fn pruning_updates_cache_accounting_and_remains_exact() {
    let store = MemoryStorage::with_limits(limits(4, 64));
    let body = Bytes::from_static(b"prune-me");
    let cid = cid_for(&body);
    let object = exact_object(&cid);

    store.put(&cid, body.clone()).await.unwrap();

    assert_eq!(store.object_count(), 1);
    assert_eq!(store.total_bytes(), body.len() as u64);

    assert_eq!(
        store.prune(&object).await.unwrap(),
        PruneOutcome::Removed {
            bytes: body.len() as u64,
        }
    );

    assert_eq!(store.object_count(), 0);
    assert_eq!(store.total_bytes(), 0);
    assert!(!store.exists(&cid).await.unwrap());

    assert_eq!(store.prune(&object).await.unwrap(), PruneOutcome::NotFound);
}

#[tokio::test]
async fn a_new_backend_instance_contains_no_previous_bytes() {
    let body = Bytes::from_static(b"process-local-only");
    let cid = cid_for(&body);

    {
        let store = MemoryStorage::with_limits(limits(4, 64));
        store.put(&cid, body).await.unwrap();
        assert!(store.exists(&cid).await.unwrap());
    }

    let replacement_instance = MemoryStorage::with_limits(limits(4, 64));

    assert!(
        !replacement_instance.exists(&cid).await.unwrap(),
        "a new memory backend must not claim restart durability"
    );
}
