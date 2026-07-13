//! RO:WHAT — Bounded repair queue, policy revalidation, and churn recovery tests.
//! RO:WHY — Phase 12 exit work requires backpressure and recovery after provider loss.
//! RO:INTERACTS — repair_queue, repair review, alternate fetch, provider store.
//! RO:INVARIANTS — bounded; deduplicated; policy rechecked; verified source bytes only.
//! RO:SECURITY — tombstoned queued work never executes; identities remain crab://node.
//! RO:TEST — cargo test -p svc-dht --test provider_repair_queue.

use bytes::Bytes;
use ron_policy::{B3Id, ModerationPolicy, ModerationReasonCode};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use svc_dht::{
    pipeline::{
        fetch::fetch_from_candidates,
        repair::{review_repair_need, RepairReview},
        repair_queue::{
            bounded_repair_queue, run_repair_worker_once, RepairEnqueueError, RepairEnqueueOutcome,
            RepairQueueConfigError, RepairWorkerOutcome,
        },
    },
    provider::{ProviderStatusHint, ProviderStatusUpdateOutcome, Store},
    types::{B3Cid, CrabNodeId},
};

const CID_ONE: &str = "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

const CID_TWO: &str = "b3:1111111111111111111111111111111111111111111111111111111111111111";

const NODE_A_URI: &str =
    "crab://node/00000000000000000000000000000000000000000000000000000000000000a1";
const NODE_B_URI: &str =
    "crab://node/00000000000000000000000000000000000000000000000000000000000000b2";
const NODE_C_URI: &str =
    "crab://node/00000000000000000000000000000000000000000000000000000000000000c3";
const NODE_D_URI: &str =
    "crab://node/00000000000000000000000000000000000000000000000000000000000000d4";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MockRepairError {
    FetchFailed,
    AdvertisementFailed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MockFetchError {
    Unreachable,
}

fn cid(value: &str) -> B3Cid {
    value.parse().expect("test CID must be canonical")
}

fn cid_for(bytes: &[u8]) -> B3Cid {
    format!("b3:{}", blake3::hash(bytes).to_hex()).parse().expect("computed CID must be canonical")
}

fn moderation_id(value: &str) -> B3Id {
    value.parse().expect("test moderation ID must be canonical")
}

fn node(uri: &str) -> CrabNodeId {
    CrabNodeId::from_uri(uri).expect("test node URI must be canonical")
}

fn add_provider(store: &Store, object: &B3Cid, uri: &str) {
    store
        .add(object.to_string(), uri.to_owned(), Some(Duration::from_secs(60)))
        .expect("canonical provider advertisement");
}

#[test]
fn repair_queue_is_bounded_and_rejects_duplicate_pending_work() {
    let store = Store::new(Duration::from_secs(60));
    let policy = ModerationPolicy::default();

    let (queue, _receiver) = bounded_repair_queue(1).expect("bounded queue");

    assert_eq!(
        queue.review_and_enqueue(&store, &cid(CID_ONE), 3, &policy,),
        Ok(RepairEnqueueOutcome::Enqueued)
    );

    assert_eq!(
        queue.review_and_enqueue(&store, &cid(CID_ONE), 3, &policy,),
        Ok(RepairEnqueueOutcome::DuplicatePending)
    );

    assert_eq!(
        queue.review_and_enqueue(&store, &cid(CID_TWO), 3, &policy,),
        Err(RepairEnqueueError::QueueFull)
    );

    assert!(matches!(bounded_repair_queue(0), Err(RepairQueueConfigError::ZeroCapacity)));
}

#[tokio::test]
async fn worker_revalidates_policy_before_executing_queued_work() {
    let store = Store::new(Duration::from_secs(60));
    let mut policy = ModerationPolicy::default();
    let object = cid(CID_ONE);

    let (queue, mut receiver) = bounded_repair_queue(2).expect("bounded queue");

    assert_eq!(
        queue.review_and_enqueue(&store, &object, 3, &policy,),
        Ok(RepairEnqueueOutcome::Enqueued)
    );

    assert!(policy.insert_owner_tombstone(moderation_id(CID_ONE),));

    let called = Arc::new(AtomicBool::new(false));
    let called_by_worker = called.clone();

    let outcome =
        run_repair_worker_once(&mut receiver, &queue, &store, &policy, move |_candidate| {
            called_by_worker.store(true, Ordering::SeqCst);

            async { Ok::<(), MockRepairError>(()) }
        })
        .await;

    assert_eq!(
        outcome,
        RepairWorkerOutcome::Suppressed {
            cid: object,
            reason: ModerationReasonCode::OwnerTombstone,
        }
    );

    assert!(!called.load(Ordering::SeqCst), "tombstoned queued work must never reach the executor");
}

#[tokio::test]
async fn failed_worker_attempt_can_be_enqueued_again() {
    let store = Store::new(Duration::from_secs(60));
    let policy = ModerationPolicy::default();
    let object = cid(CID_ONE);

    let (queue, mut receiver) = bounded_repair_queue(1).expect("bounded queue");

    assert_eq!(
        queue.review_and_enqueue(&store, &object, 3, &policy,),
        Ok(RepairEnqueueOutcome::Enqueued)
    );

    let outcome =
        run_repair_worker_once(&mut receiver, &queue, &store, &policy, |_candidate| async {
            Err::<(), MockRepairError>(MockRepairError::FetchFailed)
        })
        .await;

    assert_eq!(
        outcome,
        RepairWorkerOutcome::Attempted {
            cid: object.clone(),
            result: Err(MockRepairError::FetchFailed),
        }
    );

    assert_eq!(
        queue.review_and_enqueue(&store, &object, 3, &policy,),
        Ok(RepairEnqueueOutcome::Enqueued),
        "terminal worker failure must release the pending key"
    );
}

#[tokio::test]
async fn verified_repair_worker_recovers_from_provider_churn() {
    let expected_bytes = Bytes::from_static(b"phase-12 churn recovery object");
    let object = cid_for(expected_bytes.as_ref());
    let store = Arc::new(Store::new(Duration::from_secs(60)));
    let policy = ModerationPolicy::default();

    for uri in [NODE_A_URI, NODE_B_URI, NODE_C_URI] {
        add_provider(&store, &object, uri);
    }

    // Simulate one service-node loss from an original target of three.
    assert_eq!(
        store.record_local_status(
            object.as_str(),
            node(NODE_C_URI),
            ProviderStatusHint::Unavailable,
        ),
        ProviderStatusUpdateOutcome::Updated
    );

    let before = review_repair_need(&store, &object, 3, &policy).expect("valid repair review");

    let RepairReview::Candidate(candidate) = before else {
        panic!("provider loss must create a repair candidate");
    };

    assert_eq!(candidate.current_provider_count, 2);
    assert_eq!(candidate.missing_provider_count, 1);

    let (queue, mut receiver) = bounded_repair_queue(2).expect("bounded queue");

    assert_eq!(
        queue.review_and_enqueue(&store, &object, 3, &policy,),
        Ok(RepairEnqueueOutcome::Enqueued)
    );

    let worker_store = store.clone();
    let worker_bytes = expected_bytes.clone();

    let outcome =
        run_repair_worker_once(&mut receiver, &queue, &store, &policy, move |candidate| {
            let worker_store = worker_store.clone();
            let worker_bytes = worker_bytes.clone();

            async move {
                let fetched = fetch_from_candidates(
                    &worker_store,
                    &candidate.cid,
                    candidate.source_providers.len(),
                    move |provider, _requested_cid| {
                        let worker_bytes = worker_bytes.clone();

                        async move {
                            if provider == node(NODE_A_URI) {
                                Err(MockFetchError::Unreachable)
                            } else {
                                Ok(worker_bytes)
                            }
                        }
                    },
                )
                .await
                .map_err(|_| MockRepairError::FetchFailed)?;

                worker_store
                    .add(
                        candidate.cid.to_string(),
                        NODE_D_URI.to_owned(),
                        Some(Duration::from_secs(60)),
                    )
                    .map_err(|_| MockRepairError::AdvertisementFailed)?;

                assert_eq!(fetched.bytes, expected_bytes);
                Ok::<(), MockRepairError>(())
            }
        })
        .await;

    assert_eq!(outcome, RepairWorkerOutcome::Attempted { cid: object.clone(), result: Ok(()) });

    assert_eq!(
        review_repair_need(&store, &object, 3, &policy,),
        Ok(RepairReview::TargetSatisfied { target_provider_count: 3 })
    );

    let selected = store.select_candidates(object.as_str(), 3);

    assert_eq!(selected.len(), 3);
    assert!(selected.contains(&node(NODE_D_URI)));
    assert!(!selected.contains(&node(NODE_C_URI)), "lost provider must remain excluded");
}
