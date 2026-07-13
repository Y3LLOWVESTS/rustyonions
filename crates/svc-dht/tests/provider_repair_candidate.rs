//! RO:WHAT — Policy-gated provider-count repair-candidate tests.
//! RO:WHY — Phase 12 requires under-replication review without repairing refused content.
//! RO:INTERACTS — pipeline::repair, provider::Store, ron-policy moderation.
//! RO:INVARIANTS — tombstones/denies/blocks/quarantine suppress; no fake completion.
//! RO:SECURITY — candidate sources remain canonical crab://node identities.
//! RO:TEST — cargo test -p svc-dht --test provider_repair_candidate.

use std::time::Duration;

use ron_policy::{B3Id, ModerationPolicy, ModerationReasonCode};
use svc_dht::{
    pipeline::repair::{
        review_repair_need, RepairReview, RepairReviewError, MAX_REPLICATION_TARGET,
    },
    provider::{ProviderStatusHint, ProviderStatusUpdateOutcome, Store},
    types::{B3Cid, CrabNodeId},
};

const CID: &str = "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

const NODE_A_URI: &str =
    "crab://node/00000000000000000000000000000000000000000000000000000000000000a1";
const NODE_B_URI: &str =
    "crab://node/00000000000000000000000000000000000000000000000000000000000000b2";
const NODE_C_URI: &str =
    "crab://node/00000000000000000000000000000000000000000000000000000000000000c3";

fn cid() -> B3Cid {
    CID.parse().expect("test CID must be canonical")
}

fn moderation_id() -> B3Id {
    CID.parse().expect("test moderation ID must be canonical")
}

fn node(uri: &str) -> CrabNodeId {
    CrabNodeId::from_uri(uri).expect("test node URI must be canonical")
}

fn add_provider(store: &Store, uri: &str) {
    store
        .add(CID.to_owned(), uri.to_owned(), Some(Duration::from_secs(60)))
        .expect("canonical provider record");
}

fn default_policy() -> ModerationPolicy {
    ModerationPolicy::default()
}

#[test]
fn repair_candidate_is_created_below_target() {
    let store = Store::new(Duration::from_secs(60));
    let policy = default_policy();

    add_provider(&store, NODE_A_URI);

    let review = review_repair_need(&store, &cid(), 3, &policy).expect("valid repair review");

    let RepairReview::Candidate(candidate) = review else {
        panic!("one provider must be below target three");
    };

    assert_eq!(candidate.cid, cid());
    assert_eq!(candidate.target_provider_count, 3);
    assert_eq!(candidate.current_provider_count, 1);
    assert_eq!(candidate.missing_provider_count, 2);
    assert_eq!(candidate.source_providers, vec![node(NODE_A_URI)]);
}

#[test]
fn no_provider_still_creates_a_truthful_repair_candidate() {
    let store = Store::new(Duration::from_secs(60));
    let policy = default_policy();

    let review = review_repair_need(&store, &cid(), 3, &policy).expect("valid repair review");

    let RepairReview::Candidate(candidate) = review else {
        panic!("zero providers must create a candidate");
    };

    assert_eq!(candidate.current_provider_count, 0);
    assert_eq!(candidate.missing_provider_count, 3);
    assert!(candidate.source_providers.is_empty());
}

#[test]
fn unavailable_provider_does_not_satisfy_replication_target() {
    let store = Store::new(Duration::from_secs(60));
    let policy = default_policy();

    add_provider(&store, NODE_A_URI);
    add_provider(&store, NODE_B_URI);
    add_provider(&store, NODE_C_URI);

    assert_eq!(
        store.record_local_status(CID, node(NODE_B_URI), ProviderStatusHint::Unavailable,),
        ProviderStatusUpdateOutcome::Updated
    );

    let review = review_repair_need(&store, &cid(), 3, &policy).expect("valid repair review");

    let RepairReview::Candidate(candidate) = review else {
        panic!("unavailable provider must not satisfy target");
    };

    assert_eq!(candidate.current_provider_count, 2);
    assert_eq!(candidate.missing_provider_count, 1);
    assert_eq!(candidate.source_providers, vec![node(NODE_A_URI), node(NODE_C_URI)]);
}

#[test]
fn target_satisfied_does_not_create_a_repair_candidate() {
    let store = Store::new(Duration::from_secs(60));
    let policy = default_policy();

    add_provider(&store, NODE_A_URI);
    add_provider(&store, NODE_B_URI);
    add_provider(&store, NODE_C_URI);

    assert_eq!(
        review_repair_need(&store, &cid(), 3, &policy),
        Ok(RepairReview::TargetSatisfied { target_provider_count: 3 })
    );
}

#[test]
fn owner_tombstoned_content_does_not_repair() {
    let store = Store::new(Duration::from_secs(60));
    let object = moderation_id();
    let mut policy = ModerationPolicy::default();

    // A local allow must not override the stronger owner tombstone.
    assert!(policy.insert_local_allow(object.clone()));
    assert!(policy.insert_owner_tombstone(object));

    add_provider(&store, NODE_A_URI);

    assert_eq!(
        review_repair_need(&store, &cid(), 3, &policy),
        Ok(RepairReview::Suppressed { reason: ModerationReasonCode::OwnerTombstone })
    );

    // Review is declarative and must not mutate provider inventory.
    assert_eq!(store.get_live(CID), vec![NODE_A_URI.to_owned()]);
}

#[test]
fn every_canonical_refusal_state_suppresses_repair() {
    let store = Store::new(Duration::from_secs(60));

    let mut global_deny = ModerationPolicy::default();
    assert!(global_deny.insert_global_deny(moderation_id()));

    let mut local_block = ModerationPolicy::default();
    assert!(local_block.insert_local_block(moderation_id()));

    let mut quarantine = ModerationPolicy::default();
    assert!(quarantine.insert_quarantine(moderation_id()));

    for (policy, reason) in [
        (global_deny, ModerationReasonCode::GlobalDeny),
        (local_block, ModerationReasonCode::LocalBlock),
        (quarantine, ModerationReasonCode::Quarantined),
    ] {
        assert_eq!(
            review_repair_need(&store, &cid(), 3, &policy),
            Ok(RepairReview::Suppressed { reason })
        );
    }
}

#[test]
fn repair_target_is_strictly_bounded() {
    let store = Store::new(Duration::from_secs(60));
    let policy = default_policy();

    assert_eq!(review_repair_need(&store, &cid(), 0, &policy), Err(RepairReviewError::ZeroTarget));

    assert_eq!(
        review_repair_need(&store, &cid(), MAX_REPLICATION_TARGET + 1, &policy,),
        Err(RepairReviewError::TargetTooLarge {
            requested: MAX_REPLICATION_TARGET + 1,
            maximum: MAX_REPLICATION_TARGET,
        })
    );
}
