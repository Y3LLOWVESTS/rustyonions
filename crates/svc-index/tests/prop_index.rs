//! RO:WHAT — Property-style validation tests for svc-index manifest pointer input normalization.
//! RO:WHY — Bad b3/name forms must reject deterministically before storage.
//! RO:INVARIANTS — CIDs are lowercase b3; site names reject path traversal and credentials.

use svc_index::types::{normalize_asset_kind, normalize_b3_cid, normalize_site_name};

#[test]
fn b3_validation_rejects_bad_lengths_and_characters() {
    assert!(normalize_b3_cid("b3:abc").is_err());

    let uppercase = "b3:0123456789ABCDEF0123456789abcdef0123456789abcdef0123456789abcdef";
    assert!(normalize_b3_cid(uppercase).is_err());

    let bad_char = "b3:g123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    assert!(normalize_b3_cid(bad_char).is_err());
}

#[test]
fn site_name_validation_rejects_unsafe_forms() {
    for bad in [
        "",
        ".",
        "..",
        "../sealobsta",
        "sealobsta/manifest",
        "user@sealobsta",
        "sealobsta..com",
        "sea lobsta",
        "sealobsta\n",
    ] {
        assert!(
            normalize_site_name(bad).is_err(),
            "expected reject: {bad:?}"
        );
    }
}

#[test]
fn asset_kind_validation_rejects_unknown_forms() {
    assert!(normalize_asset_kind("").is_err());
    assert!(normalize_asset_kind("binary").is_err());
    assert!(normalize_asset_kind("../image").is_err());
}
