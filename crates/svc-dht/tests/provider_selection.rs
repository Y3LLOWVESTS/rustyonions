//! RO:WHAT — Local provider-status and deterministic candidate-selection tests.
//! RO:WHY — Phase 12 fetching needs bounded alternatives without trusting self-declared health.
//! RO:INTERACTS — provider::{ProviderStatusHint, ProviderStatusUpdateOutcome, Store}.
//! RO:INVARIANTS — expired/unavailable excluded; ordering deterministic; no raw network address.
//! RO:TEST — cargo test -p svc-dht --test provider_selection.

use std::time::{Duration, Instant};

use svc_dht::{
    provider::{ProviderStatusHint, ProviderStatusUpdateOutcome, Store},
    types::CrabNodeId,
};

const CID: &str = "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

const NODE_A_URI: &str =
    "crab://node/00000000000000000000000000000000000000000000000000000000000000a1";
const NODE_B_URI: &str =
    "crab://node/00000000000000000000000000000000000000000000000000000000000000b2";
const NODE_C_URI: &str =
    "crab://node/00000000000000000000000000000000000000000000000000000000000000c3";
const NODE_D_URI: &str =
    "crab://node/00000000000000000000000000000000000000000000000000000000000000d4";
const NODE_E_URI: &str =
    "crab://node/00000000000000000000000000000000000000000000000000000000000000e5";

fn node(uri: &str) -> CrabNodeId {
    CrabNodeId::from_uri(uri).expect("test node URI must be canonical")
}

fn uris(nodes: Vec<CrabNodeId>) -> Vec<String> {
    nodes.into_iter().map(CrabNodeId::to_uri).collect()
}

#[test]
fn candidate_selection_is_status_aware_deterministic_and_bounded() {
    let store = Store::new(Duration::from_secs(60));

    // Insert deliberately out of candidate order.
    for uri in [NODE_E_URI, NODE_C_URI, NODE_A_URI, NODE_D_URI, NODE_B_URI] {
        store
            .add(CID.to_owned(), uri.to_owned(), Some(Duration::from_secs(60)))
            .expect("canonical provider record");
    }

    assert_eq!(
        store.record_local_status(CID, node(NODE_B_URI), ProviderStatusHint::Responsive,),
        ProviderStatusUpdateOutcome::Updated
    );

    assert_eq!(
        store.record_local_status(CID, node(NODE_D_URI), ProviderStatusHint::Responsive,),
        ProviderStatusUpdateOutcome::Updated
    );

    assert_eq!(
        store.record_local_status(CID, node(NODE_A_URI), ProviderStatusHint::Degraded,),
        ProviderStatusUpdateOutcome::Updated
    );

    assert_eq!(
        store.record_local_status(CID, node(NODE_E_URI), ProviderStatusHint::Unavailable,),
        ProviderStatusUpdateOutcome::Updated
    );

    // Responsive B/D first, then unknown C, then degraded A.
    // Unavailable E is not a candidate.
    assert_eq!(
        uris(store.select_candidates(CID, 4)),
        vec![
            NODE_B_URI.to_owned(),
            NODE_D_URI.to_owned(),
            NODE_C_URI.to_owned(),
            NODE_A_URI.to_owned(),
        ]
    );

    assert_eq!(
        uris(store.select_candidates(CID, 2)),
        vec![NODE_B_URI.to_owned(), NODE_D_URI.to_owned(),]
    );

    assert!(store.select_candidates(CID, 0).is_empty());
}

#[test]
fn provider_readvertisement_cannot_reset_local_unavailable_status() {
    let store = Store::new(Duration::from_secs(60));

    store
        .add(CID.to_owned(), NODE_A_URI.to_owned(), Some(Duration::from_secs(60)))
        .expect("canonical provider record");

    assert_eq!(
        store.record_local_status(CID, node(NODE_A_URI), ProviderStatusHint::Unavailable,),
        ProviderStatusUpdateOutcome::Updated
    );

    // Refresh the external advertisement. This may extend expiry, but it must
    // not erase the local unavailable observation.
    store
        .add(CID.to_owned(), NODE_A_URI.to_owned(), Some(Duration::from_secs(120)))
        .expect("canonical provider refresh");

    assert!(
        store.select_candidates(CID, 1).is_empty(),
        "provider advertisement must not self-reset local status"
    );

    // A later successful local observation may restore eligibility.
    assert_eq!(
        store.record_local_status(CID, node(NODE_A_URI), ProviderStatusHint::Responsive,),
        ProviderStatusUpdateOutcome::Updated
    );

    assert_eq!(uris(store.select_candidates(CID, 1)), vec![NODE_A_URI.to_owned()]);
}

#[test]
fn candidate_selection_excludes_expired_records_without_sleeping() {
    let store = Store::new(Duration::from_secs(60));

    store
        .add(CID.to_owned(), NODE_A_URI.to_owned(), Some(Duration::from_secs(60)))
        .expect("shorter provider record");

    store
        .add(CID.to_owned(), NODE_B_URI.to_owned(), Some(Duration::from_secs(120)))
        .expect("longer provider record");

    assert_eq!(
        store.record_local_status(CID, node(NODE_A_URI), ProviderStatusHint::Responsive,),
        ProviderStatusUpdateOutcome::Updated
    );

    assert_eq!(
        store.record_local_status(CID, node(NODE_B_URI), ProviderStatusHint::Responsive,),
        ProviderStatusUpdateOutcome::Updated
    );

    let simulated_future = Instant::now() + Duration::from_secs(61);

    assert_eq!(
        uris(store.select_candidates_at(CID, 4, simulated_future,)),
        vec![NODE_B_URI.to_owned()]
    );
}

#[test]
fn local_status_update_rejects_missing_or_invalid_records() {
    let store = Store::new(Duration::from_secs(60));

    assert_eq!(
        store.record_local_status(CID, node(NODE_A_URI), ProviderStatusHint::Responsive,),
        ProviderStatusUpdateOutcome::NotFound
    );

    assert_eq!(
        store.record_local_status(
            "b3:not-canonical",
            node(NODE_A_URI),
            ProviderStatusHint::Responsive,
        ),
        ProviderStatusUpdateOutcome::NotFound
    );
}
