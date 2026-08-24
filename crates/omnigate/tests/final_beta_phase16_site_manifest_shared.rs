//! RO:WHAT — FINAL_BETA Phase 16 source regression proving one shared Omnigate Site-manifest parser.
//! RO:WHY — Prevents sites.rs and site_visit.rs from drifting into incompatible manifest contracts again.
//! RO:INTERACTS — routes/v1/site_manifest.rs, sites.rs, site_visit.rs.
//! RO:INVARIANTS — one SiteManifestDocument definition; both route modules import it; strict unknown-field rejection remains.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — shared parsing introduces no ownership, wallet, ledger, receipt, or paid-unlock authority.
//! RO:TEST — cargo test -p omnigate --test final_beta_phase16_site_manifest_shared.

use std::{fs, path::PathBuf};

fn crate_file(relative: &str) -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    fs::read_to_string(root.join(relative))
        .unwrap_or_else(|error| panic!("read {relative}: {error}"))
}

#[test]
fn phase16_site_and_paid_visit_use_one_shared_manifest_document() {
    let shared = crate_file("src/routes/v1/site_manifest.rs");

    let sites = crate_file("src/routes/v1/sites.rs");

    let visit = crate_file("src/routes/v1/site_visit.rs");

    assert_eq!(shared.matches("struct SiteManifestDocument").count(), 1,);

    assert_eq!(sites.matches("struct SiteManifestDocument").count(), 0,);

    assert_eq!(visit.matches("struct SiteManifestDocument").count(), 0,);

    assert!(sites.contains("super::site_manifest"),);

    assert!(visit.contains("super::site_manifest"),);

    assert!(shared.contains("#[serde(deny_unknown_fields)]"),);

    for required in [
        "asset_map",
        "route_map",
        "owner",
        "payout",
        "metadata",
        "rendering",
        "provenance",
        "storage",
        "receipts",
    ] {
        assert!(
            shared.contains(required),
            "shared Site manifest missing {required}",
        );
    }
}

#[test]
fn phase16_shared_manifest_parser_has_no_economic_mutation_authority() {
    let shared = crate_file("src/routes/v1/site_manifest.rs");

    for forbidden in [
        "wallet_transfer",
        "ledger_mutation",
        "capture_hold",
        "create_receipt",
        "mark_paid",
        "unlock_content",
    ] {
        assert!(
            !shared.contains(forbidden),
            "shared manifest parser gained forbidden authority: {forbidden}",
        );
    }
}
