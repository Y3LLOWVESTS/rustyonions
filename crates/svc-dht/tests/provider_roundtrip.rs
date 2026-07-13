//! RO:WHAT — Provider-store validation, refresh, lookup, and TTL-expiry tests.
//! RO:WHY — Phase 12 provider records must use canonical B3 IDs before discovery state exists.
//! RO:INTERACTS — provider::{ProviderStoreError, Store}.
//! RO:INVARIANTS — exact B3 syntax; exact Crab node syntax; refresh de-duplicates; TTL expires.
//! RO:TEST — cargo test -p svc-dht --test provider_roundtrip.

use std::time::Duration;

use svc_dht::provider::{ProviderStoreError, Store};

const CID: &str = "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

const NODE_A_URI: &str =
    "crab://node/00000000000000000000000000000000000000000000000000000000000000a1";

const NODE_B_URI: &str =
    "crab://node/00000000000000000000000000000000000000000000000000000000000000b2";

#[test]
fn provider_add_get_prune_roundtrip() {
    let store = Store::new(Duration::from_secs(2));

    store
        .add(CID.to_owned(), NODE_A_URI.to_owned(), Some(Duration::from_millis(250)))
        .expect("valid crab node A");

    store
        .add(CID.to_owned(), NODE_B_URI.to_owned(), Some(Duration::from_millis(250)))
        .expect("valid crab node B");

    store
        .add(CID.to_owned(), NODE_A_URI.to_owned(), Some(Duration::from_millis(250)))
        .expect("valid refresh for crab node A");

    let mut live = store.get_live(CID);
    live.sort();

    assert_eq!(live, vec![NODE_A_URI.to_owned(), NODE_B_URI.to_owned()]);

    std::thread::sleep(Duration::from_millis(300));

    let purged = store.purge_expired();

    assert!(purged >= 1, "expected at least one purged; got {purged}");

    let live_after_expiry = store.get_live(CID);

    assert!(
        live_after_expiry.is_empty(),
        "expected no live providers after purge, got {live_after_expiry:?}"
    );
}

#[test]
fn provider_store_rejects_noncanonical_cids_before_insert() {
    let store = Store::new(Duration::from_secs(60));

    for bad_cid in [
        "b3:deadbeef",
        " b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        "B3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        "b3:0123456789ABCDEF0123456789abcdef0123456789abcdef0123456789abcdef",
        "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef ",
    ] {
        let error = store
            .add(bad_cid.to_owned(), NODE_A_URI.to_owned(), Some(Duration::from_secs(60)))
            .expect_err("noncanonical CID must be rejected");

        assert!(
            matches!(error, ProviderStoreError::InvalidCid(_)),
            "unexpected error for {bad_cid}: {error}"
        );

        assert!(store.get_live(bad_cid).is_empty());
    }

    assert!(store.debug_snapshot().is_empty(), "rejected CIDs must not create provider state");
}
