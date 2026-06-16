//! RO:WHAT — Source guards against fake receipt/balance/finality authority in omnigate.
//! RO:WHY — Paid access must depend on backend wallet/ledger truth, not local fabrication.
//! RO:INTERACTS — omnigate route sources, wallet/content/site/chat/stream paid paths.
//! RO:INVARIANTS — no fake receipts, no fake balances, no fake finality, no root/checkpoint authority.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — prevents display DTOs from becoming economic authority.
//! RO:TEST — cargo test -p omnigate --test quickchain_preflight_no_fake_receipts.

use std::{
    fs,
    path::{Path, PathBuf},
};

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_rel(path: &str) -> String {
    let full = crate_root().join(path);
    fs::read_to_string(&full).unwrap_or_else(|err| {
        panic!("failed to read {}: {err}", full.display());
    })
}

fn collect_rust_files(root: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(root).unwrap_or_else(|err| {
        panic!("failed to read directory {}: {err}", root.display());
    });

    for entry in entries {
        let path = entry
            .unwrap_or_else(|err| panic!("failed to read directory entry: {err}"))
            .path();

        if path.is_dir() {
            collect_rust_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

fn production_sources() -> Vec<(PathBuf, String)> {
    let mut files = Vec::new();
    collect_rust_files(&crate_root().join("src"), &mut files);
    files
        .into_iter()
        .map(|path| {
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
            (path, text)
        })
        .collect()
}

fn compact_lower(source: &str) -> String {
    source
        .to_ascii_lowercase()
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect()
}

#[test]
fn production_source_does_not_construct_obvious_fake_receipts_or_finality() {
    let forbidden_compact_markers = [
        "fake_receipt",
        "synthetic_receipt",
        "dummy_receipt",
        "receipt_id:\"dev",
        "receipt_id:\"fake",
        "receipt_id:\"dummy",
        "receipt_hash:\"dev",
        "receipt_hash:\"fake",
        "receipt_hash:\"dummy",
        "\"receipt_id\":\"dev",
        "\"receipt_id\":\"fake",
        "\"receipt_id\":\"dummy",
        "\"receipt_hash\":\"dev",
        "\"receipt_hash\":\"fake",
        "\"receipt_hash\":\"dummy",
        "finalized:true",
        "\"finalized\":true",
        "finality:true",
        "\"finality\":true",
        "bridge_settled:true",
        "\"bridge_settled\":true",
        "external_settlement:true",
        "\"external_settlement\":true",
    ];

    for (path, source) in production_sources() {
        let compact = compact_lower(&source);
        for marker in forbidden_compact_markers {
            assert!(
                !compact.contains(marker),
                "{} must not construct fake receipt/finality marker `{marker}`",
                path.display()
            );
        }
    }
}

#[test]
fn production_source_does_not_construct_root_or_validator_authority_fields() {
    let forbidden_compact_markers = [
        "state_root:",
        "\"state_root\":",
        "receipt_root:",
        "\"receipt_root\":",
        "checkpoint_hash:",
        "\"checkpoint_hash\":",
        "checkpoint_root:",
        "\"checkpoint_root\":",
        "validator_signature:",
        "\"validator_signature\":",
        "validator_set:",
        "\"validator_set\":",
        "anchor_commitment:",
        "\"anchor_commitment\":",
    ];

    for (path, source) in production_sources() {
        let compact = compact_lower(&source);
        for marker in forbidden_compact_markers {
            assert!(
                !compact.contains(marker),
                "{} must not construct root/validator authority marker `{marker}`",
                path.display()
            );
        }
    }
}

#[test]
fn wallet_balance_fallback_is_explicitly_non_authoritative() {
    let wallet = read_rel("src/routes/v1/wallet.rs");

    for required in [
        "ledger_backed: false",
        "display-only dev fallback",
        "real balance must come from svc-wallet and ron-ledger",
        "do not treat this route as spend authority",
    ] {
        assert!(
            wallet.contains(required),
            "wallet display fallback must include non-authoritative marker `{required}`"
        );
    }

    assert!(
        !wallet.contains("ledger_backed: true,\n            source: \"omnigate"),
        "omnigate must not mark its own fallback as ledger-backed truth"
    );
}

#[test]
fn paid_routes_use_wallet_receipt_values_without_dev_receipt_fabrication() {
    for rel in [
        "src/routes/v1/content_view.rs",
        "src/routes/v1/site_visit.rs",
        "src/routes/v1/chat.rs",
        "src/routes/v1/streams.rs",
    ] {
        let source = read_rel(rel);
        assert!(
            source.contains("wallet_receipt")
                || source.contains("svc-wallet")
                || source.contains("backend wallet"),
            "{rel} must visibly depend on wallet/backend receipt truth"
        );

        let compact = compact_lower(&source);
        for marker in [
            "receipt_id:\"dev",
            "receipt_hash:\"dev",
            "\"receipt_id\":\"dev",
            "\"receipt_hash\":\"dev",
            "fake_receipt",
            "synthetic_receipt",
            "dummy_receipt",
        ] {
            assert!(
                !compact.contains(marker),
                "{rel} must not fabricate dev/fake receipt marker `{marker}`"
            );
        }
    }
}
