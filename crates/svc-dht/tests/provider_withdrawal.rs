//! RO:WHAT — Exact local provider-record withdrawal tests.
//!
//! RO:WHY — Physical object pruning must remove this node's provider record
//! without removing neighboring providers or claiming fake success.
//!
//! RO:INTERACTS — provider::Store and the typed CrabNodeId identity.
//!
//! RO:INVARIANTS — exact CID plus exact node; idempotent not-found outcome;
//! no raw IP, socket, transport route, wallet, ledger, or reward behavior.
//!
//! RO:TEST — cargo test -p svc-dht --test provider_withdrawal.

use std::time::Duration;

use svc_dht::{
    provider::{ProviderWithdrawalOutcome, Store},
    types::CrabNodeId,
};

const CID_A: &str = "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const CID_B: &str = "b3:abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";

const NODE_A_URI: &str =
    "crab://node/00000000000000000000000000000000000000000000000000000000000000a1";
const NODE_B_URI: &str =
    "crab://node/00000000000000000000000000000000000000000000000000000000000000b2";

fn node(uri: &str) -> CrabNodeId {
    CrabNodeId::from_uri(uri).expect("test node URI must be canonical")
}

#[test]
fn withdrawal_removes_only_the_exact_cid_and_node_record() {
    let store = Store::new(Duration::from_secs(60));

    store
        .add(CID_A.to_string(), NODE_A_URI.to_string(), Some(Duration::from_secs(60)))
        .expect("node A provider record should be accepted");
    store
        .add(CID_A.to_string(), NODE_B_URI.to_string(), Some(Duration::from_secs(60)))
        .expect("node B provider record should be accepted");
    store
        .add(CID_B.to_string(), NODE_A_URI.to_string(), Some(Duration::from_secs(60)))
        .expect("node A second-CID provider record should be accepted");

    assert_eq!(store.withdraw_id(CID_A, node(NODE_A_URI)), ProviderWithdrawalOutcome::Withdrawn);

    assert_eq!(store.get_live(CID_A), vec![NODE_B_URI.to_string()]);
    assert_eq!(store.get_live(CID_B), vec![NODE_A_URI.to_string()]);
}

#[test]
fn repeated_withdrawal_reports_not_found_instead_of_fake_success() {
    let store = Store::new(Duration::from_secs(60));

    store
        .add(CID_A.to_string(), NODE_A_URI.to_string(), Some(Duration::from_secs(60)))
        .expect("provider record should be accepted");

    assert_eq!(store.withdraw_id(CID_A, node(NODE_A_URI)), ProviderWithdrawalOutcome::Withdrawn);
    assert_eq!(store.withdraw_id(CID_A, node(NODE_A_URI)), ProviderWithdrawalOutcome::NotFound);
}

#[test]
fn withdrawal_of_absent_provider_does_not_mutate_other_records() {
    let store = Store::new(Duration::from_secs(60));

    store
        .add(CID_A.to_string(), NODE_B_URI.to_string(), Some(Duration::from_secs(60)))
        .expect("neighbor provider record should be accepted");

    assert_eq!(store.withdraw_id(CID_A, node(NODE_A_URI)), ProviderWithdrawalOutcome::NotFound);
    assert_eq!(store.get_live(CID_A), vec![NODE_B_URI.to_string()]);
}
