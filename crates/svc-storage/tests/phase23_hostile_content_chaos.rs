//! RO:WHAT — Phase 23 hostile-content and cache-state chaos drills.
//! RO:WHY — Proves corrupt, evicted, globally denied, and owner-tombstoned
//! objects cannot produce successful object delivery.
//! RO:INTERACTS — bounded MemoryStorage, canonical moderation policy,
//! LocalOapObjectService, OAP stream verification, and BLAKE3 content IDs.
//! RO:INVARIANTS — complete digest verification; evicted objects are absent;
//! stronger moderation defeats local allow; resident refused bytes are not
//! confused with successfully served or physically deleted content.
//! RO:SECURITY — no accounting acceptance, reward truth, wallet/ledger
//! mutation, receipt creation, payout authority, or finality.
//! RO:TEST — cargo test -p svc-storage --test
//! phase23_hostile_content_chaos.

use std::sync::Arc;

use bytes::Bytes;
use ron_policy::{B3Id, ModerationPolicy, ModerationReasonCode};
use ron_proto::ContentId;
use svc_storage::{
    errors::StorageError,
    oap_object::{build_obj_get_request, verify_obj_stream, LocalOapObjectService, OapObjectError},
    storage::{DynStorage, MemoryStorage, MemoryStorageLimits, Storage},
};

fn cid_for(bytes: &[u8]) -> ContentId {
    format!("b3:{}", blake3::hash(bytes).to_hex())
        .parse()
        .expect("calculated BLAKE3 must be a canonical ContentId")
}

fn moderation_id(cid: &ContentId) -> B3Id {
    cid.as_str()
        .parse()
        .expect("canonical ContentId must parse as moderation B3Id")
}

fn one_object_limits() -> MemoryStorageLimits {
    MemoryStorageLimits::try_new(1, 1_024)
        .expect("Phase 23 cache limits must be finite and nonzero")
}

#[tokio::test]
async fn corrupt_provider_bytes_fail_complete_digest_validation() {
    let claimed = cid_for(b"expected provider object");
    let corrupt = Bytes::from_static(b"hostile provider bytes");

    let store: DynStorage = Arc::new(MemoryStorage::default());

    // MemoryStorage deliberately accepts a caller-supplied key. This models a
    // hostile or corrupted provider backend that returns bytes inconsistent
    // with the requested content identity.
    store
        .put(claimed.as_str(), corrupt.clone())
        .await
        .expect("corrupt provider fixture must enter the local backend");

    let service = LocalOapObjectService::privacy_aware_local(store.clone())
        .expect("canonical local OAP policy must build");

    let request = build_obj_get_request(claimed.clone(), 23, 1)
        .expect("Phase 23 corrupt-byte request must build");

    let error = service
        .serve_obj_get(request)
        .await
        .expect_err("wrong bytes must never become a successful OAP stream");

    let expected_actual = format!("b3:{}", blake3::hash(&corrupt).to_hex());

    assert_eq!(
        error,
        OapObjectError::DigestMismatch {
            expected: claimed.to_string(),
            actual: expected_actual,
        }
    );

    assert!(
        store
            .exists(claimed.as_str())
            .await
            .expect("corrupt fixture existence check"),
        "digest rejection must not be misreported as physical byte deletion",
    );
}

#[tokio::test]
async fn cache_eviction_returns_not_found_without_false_serve_success() {
    let cache = Arc::new(MemoryStorage::with_limits(one_object_limits()));
    let store: DynStorage = cache.clone();

    let first_bytes = Bytes::from_static(b"first cached object");
    let second_bytes = Bytes::from_static(b"replacement cached object");

    let first = cid_for(&first_bytes);
    let second = cid_for(&second_bytes);

    store
        .put(first.as_str(), first_bytes)
        .await
        .expect("first cache object must store");

    store
        .put(second.as_str(), second_bytes.clone())
        .await
        .expect("second cache object must store and evict the first");

    assert_eq!(cache.object_count(), 1);
    assert_eq!(
        cache.total_bytes(),
        u64::try_from(second_bytes.len()).expect("small fixture length must fit u64"),
    );

    assert!(
        !store
            .exists(first.as_str())
            .await
            .expect("evicted object existence check"),
        "oldest object must be absent after bounded FIFO eviction",
    );

    let service = LocalOapObjectService::privacy_aware_local(store)
        .expect("canonical local OAP policy must build");

    let evicted_request =
        build_obj_get_request(first, 23, 2).expect("evicted-object request must build");

    assert_eq!(
        service
            .serve_obj_get(evicted_request)
            .await
            .expect_err("evicted object must not produce a response stream"),
        OapObjectError::NotFound,
    );

    let retained_request =
        build_obj_get_request(second.clone(), 23, 3).expect("retained-object request must build");

    let retained_frames = service
        .serve_obj_get(retained_request)
        .await
        .expect("retained object must remain readable");

    let verified = verify_obj_stream(&second, &retained_frames)
        .expect("retained response must pass full stream verification");

    assert_eq!(verified, second_bytes);
}

#[tokio::test]
async fn deny_and_tombstone_override_local_allow_while_bytes_remain_resident() {
    let bytes = Bytes::from_static(b"resident moderated object");
    let cid = cid_for(&bytes);
    let store: DynStorage = Arc::new(MemoryStorage::default());

    store
        .put(cid.as_str(), bytes)
        .await
        .expect("resident moderation fixture must store");

    let refusal_cases = [
        (ModerationReasonCode::GlobalDeny, 4_u64),
        (ModerationReasonCode::OwnerTombstone, 5_u64),
    ];

    for (reason, correlation_id) in refusal_cases {
        let object = moderation_id(&cid);
        let mut moderation = ModerationPolicy::default();

        assert!(
            moderation.insert_local_allow(object.clone()),
            "local allow fixture must be newly inserted",
        );

        let inserted = match reason {
            ModerationReasonCode::GlobalDeny => moderation.insert_global_deny(object),
            ModerationReasonCode::OwnerTombstone => moderation.insert_owner_tombstone(object),
            ModerationReasonCode::NoRule
            | ModerationReasonCode::LocalAllow
            | ModerationReasonCode::LocalBlock
            | ModerationReasonCode::Quarantined => {
                unreachable!("Phase 23D only tests stronger global refusal")
            }
        };

        assert!(inserted);

        let service = LocalOapObjectService::privacy_aware_local(store.clone())
            .expect("canonical local OAP policy must build")
            .with_moderation_policy(moderation);

        let request = build_obj_get_request(cid.clone(), 23, correlation_id)
            .expect("moderated request must build");

        assert_eq!(
            service
                .serve_obj_get(request)
                .await
                .expect_err("resident denied content must not produce a response",),
            OapObjectError::ModerationDenied { reason },
        );

        assert!(
            store
                .exists(cid.as_str())
                .await
                .expect("resident refused object existence check"),
            "moderation refusal must not fabricate physical deletion",
        );
    }

    println!(
        "Phase 23D passed: corrupt provider bytes failed complete BLAKE3 \
         verification, cache eviction produced truthful not-found behavior, \
         global deny and owner tombstone defeated local allow while bytes \
         remained resident, and no hostile state produced successful \
         delivery or economic/finality authority."
    );
}

#[tokio::test]
async fn oversized_cache_object_rejects_without_evicting_valid_content() {
    let cache = Arc::new(MemoryStorage::with_limits(
        MemoryStorageLimits::try_new(2, 16).expect("small Phase 23 capacity must validate"),
    ));

    let valid = Bytes::from_static(b"valid");
    let valid_cid = cid_for(&valid);

    cache
        .put(valid_cid.as_str(), valid.clone())
        .await
        .expect("valid object must store");

    let oversized = Bytes::from(vec![0_u8; 17]);
    let oversized_cid = cid_for(&oversized);

    let error = cache
        .put(oversized_cid.as_str(), oversized)
        .await
        .expect_err("oversized object must reject");

    assert!(
        matches!(error, StorageError::CapacityExceeded),
        "unexpected oversized-object rejection: {error:?}",
    );

    assert!(
        cache
            .exists(valid_cid.as_str())
            .await
            .expect("valid object existence check"),
        "rejected oversized input must not evict valid resident content",
    );

    assert_eq!(cache.object_count(), 1);
    assert_eq!(
        cache.total_bytes(),
        u64::try_from(valid.len()).expect("small fixture length must fit u64"),
    );
}
