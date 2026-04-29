//! RO:WHAT — Chaos-adjacent validation smoke tests for malformed pointer inputs.
//! RO:WHY — Replaces placeholder with deterministic rejection checks.
//! RO:INVARIANTS — malformed input fails closed before storage.

use svc_index::types::{normalize_b3_cid, normalize_optional_ref, normalize_site_name};

#[test]
fn malformed_inputs_fail_closed() {
    let bad_cids = ["", "b3:", "b3:abc", "b3:../asset", "b3:0123"];
    for cid in bad_cids {
        assert!(normalize_b3_cid(cid).is_err(), "bad CID accepted: {cid:?}");
    }

    let bad_names = ["", ".", "..", "a/b", "a\\b", "owner@example"];
    for name in bad_names {
        assert!(
            normalize_site_name(name).is_err(),
            "bad name accepted: {name:?}"
        );
    }
}

#[test]
fn owner_reference_normalization_drops_empty_values() {
    assert_eq!(
        normalize_optional_ref("owner_wallet_account", Some("   ".to_owned())).unwrap(),
        None
    );

    assert_eq!(
        normalize_optional_ref("owner_wallet_account", Some(" acct_creator ".to_owned())).unwrap(),
        Some("acct_creator".to_owned())
    );
}
