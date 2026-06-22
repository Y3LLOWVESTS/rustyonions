#![allow(clippy::missing_panics_doc, clippy::must_use_candidate)]

//! RO:WHAT — QC-1A pair-interlock tests for svc-index.
//! RO:WHY — Keeps lookup/pointer/index infrastructure from becoming policy, wallet, ledger, paid-unlock, root, finality, bridge, or settlement authority.
//! RO:INTERACTS — docs/quickchain-preflight.md, scripts/dev-quickchain-preflight.sh, svc-index pointer DTOs and route/source boundaries.
//! RO:INVARIANTS — index truth is pointer truth only; names/manifests/b3/cache/policy metadata do not prove payment, receipt truth, balance, roots, or finality.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — blocks authority creep through pointer DTOs, routes, source shortcuts, names, manifests, cache hits, or policy metadata.
//! RO:TEST — cargo test -p svc-index --test quickchain_preflight_phase1_pair_interlock.

use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use std::{
    fmt::Debug,
    fs,
    path::{Path, PathBuf},
};

use svc_index::types::{
    AssetManifestPointer, PutAssetManifestPointer, PutSiteManifestPointer, SiteManifestPointer,
};

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_rel(path: &str) -> String {
    let full = crate_root().join(path);
    fs::read_to_string(&full).unwrap_or_else(|err| panic!("read {}: {err}", full.display()))
}

fn assert_contains_all(haystack: &str, label: &str, phrases: &[&str]) {
    for phrase in phrases {
        assert!(
            haystack.contains(phrase),
            "{label} must contain QC-1A interlock phrase `{phrase}`"
        );
    }
}

fn assert_contains_none(haystack: &str, label: &str, snippets: &[&str]) {
    let normalized = haystack.to_ascii_lowercase();

    for snippet in snippets {
        let needle = snippet.to_ascii_lowercase();
        assert!(
            !normalized.contains(&needle),
            "{label} must not contain forbidden QC-1A authority snippet `{snippet}`"
        );
    }
}

fn collect_rust_files(root: &Path, out: &mut Vec<PathBuf>) {
    for entry in
        fs::read_dir(root).unwrap_or_else(|err| panic!("read dir {}: {err}", root.display()))
    {
        let path = entry
            .unwrap_or_else(|err| panic!("read dir entry in {}: {err}", root.display()))
            .path();

        if path.is_dir() {
            collect_rust_files(&path, out);
        } else if path.extension().and_then(|value| value.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

fn strip_comment_only_lines(input: &str) -> String {
    let mut out = String::new();

    for line in input.lines() {
        let trimmed = line.trim_start();

        if trimmed.starts_with("//")
            || trimmed.starts_with("//!")
            || trimmed.starts_with("///")
            || trimmed.starts_with("/*")
            || trimmed.starts_with('*')
        {
            continue;
        }

        out.push_str(line);
        out.push('\n');
    }

    out
}

fn source_text_under(relative_root: &str) -> String {
    let scan_root = crate_root().join(relative_root);
    let mut files = Vec::new();
    collect_rust_files(&scan_root, &mut files);
    files.sort();

    let mut combined = String::new();

    for path in files {
        let rel = path
            .strip_prefix(crate_root())
            .unwrap_or(path.as_path())
            .display()
            .to_string();
        let raw = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
        let stripped = strip_comment_only_lines(&raw);

        combined.push_str("\n// FILE: ");
        combined.push_str(&rel);
        combined.push('\n');
        combined.push_str(&stripped);
    }

    combined
}

#[test]
fn docs_lock_index_policy_pair_roles_without_authority_transfer() {
    let docs = read_rel("docs/quickchain-preflight.md").to_lowercase();

    assert_contains_all(
        &docs,
        "svc-index quickchain-preflight.md",
        &[
            "svc-index is a lookup and pointer service",
            "index truth is not economic truth",
            "pointer truth is not receipt truth",
            "name resolution is not ownership proof",
            "b3 byte identity is not payment proof",
            "manifest lookup is not paid unlock",
            "policy metadata is not wallet authority",
            "provider lookup is not settlement finality",
            "paid access must be proven through backend service paths",
            "svc-wallet mutation or lookup path",
            "ron-ledger durable receipt truth",
        ],
    );
}

#[test]
fn manifest_does_not_add_policy_wallet_ledger_or_chain_runtime_dependencies() {
    let cargo = read_rel("Cargo.toml");

    assert_contains_none(
        &cargo,
        "svc-index Cargo.toml",
        &[
            "ron-policy",
            "svc-wallet",
            "ron-ledger",
            "ron-accounting",
            "svc-rewarder",
            "ron_proto::quickchain",
            "quickchain-runtime",
            "quickchain-validator",
            "quickchain-consensus",
            "solana",
            "anchor-lang",
            "spl-token",
            "ethers",
            "web3",
        ],
    );
}

#[test]
fn pointer_dtos_reject_authority_shaped_fields() {
    let put_asset = json!({
        "asset_kind": "image",
        "manifest_cid": "b3:abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789",
        "owner_passport_subject": "passport:main:alice",
        "owner_wallet_account": "acct_creator_alice",
        "updated_at_ms": 1
    });

    let put_site = json!({
        "manifest_cid": "b3:abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789",
        "owner_passport_subject": "passport:main:alice",
        "owner_wallet_account": "acct_creator_alice",
        "updated_at_ms": 1
    });

    let asset_pointer = json!({
        "version": 1,
        "asset_cid": "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        "asset_kind": "image",
        "manifest_cid": "b3:abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789",
        "owner_passport_subject": "passport:main:alice",
        "owner_wallet_account": "acct_creator_alice",
        "updated_at_ms": 1
    });

    let site_pointer = json!({
        "version": 1,
        "name": "alice-site",
        "manifest_cid": "b3:abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789",
        "owner_passport_subject": "passport:main:alice",
        "owner_wallet_account": "acct_creator_alice",
        "updated_at_ms": 1
    });

    for authority_field in [
        "receipt_id",
        "receipt_hash",
        "receipt_root",
        "receipt_proof",
        "balance_minor",
        "wallet_balance",
        "ledger_balance",
        "unlock_granted",
        "paid_proof",
        "finality",
        "settlement_status",
        "state_root",
        "checkpoint_root",
        "checkpoint_hash",
        "validator_signature",
        "bridge_proof",
        "operation_id",
        "idempotency_key",
        "account_sequence",
        "hold_id",
    ] {
        assert_unknown_field_rejects::<PutAssetManifestPointer>(
            "PutAssetManifestPointer",
            put_asset.clone(),
            authority_field,
        );
        assert_unknown_field_rejects::<PutSiteManifestPointer>(
            "PutSiteManifestPointer",
            put_site.clone(),
            authority_field,
        );
        assert_unknown_field_rejects::<AssetManifestPointer>(
            "AssetManifestPointer",
            asset_pointer.clone(),
            authority_field,
        );
        assert_unknown_field_rejects::<SiteManifestPointer>(
            "SiteManifestPointer",
            site_pointer.clone(),
            authority_field,
        );
    }
}

fn assert_unknown_field_rejects<T>(label: &str, mut value: Value, authority_field: &str)
where
    T: DeserializeOwned + Debug,
{
    let Value::Object(ref mut object) = value else {
        panic!("{label} fixture must be a JSON object");
    };

    object.insert(authority_field.to_owned(), json!("forbidden-authority"));

    let err = serde_json::from_value::<T>(value)
        .expect_err("authority-shaped unknown pointer field must reject");

    assert!(
        err.to_string().contains("unknown field"),
        "{label} must reject authority-shaped field {authority_field:?}, got {err}"
    );
}

#[test]
fn route_source_does_not_expose_policy_or_quickchain_settlement_unlock_routes() {
    let routes = source_text_under("src/http/routes");

    assert_contains_none(
        &routes,
        "svc-index route source",
        &[
            "\"/quickchain",
            "\"/checkpoint",
            "\"/state-root",
            "\"/receipt-root",
            "\"/validator",
            "\"/validators",
            "\"/settlement",
            "\"/external-settlement",
            "\"/bridge",
            "\"/staking",
            "\"/liquidity",
            "\"/rox",
            "\"/solana",
            "\"/policy/allow",
            "\"/policy/unlock",
            "\"/paid/unlock",
            "\"/unlock",
            "\"/receipt",
            "\"/balance",
            "\"/finality",
        ],
    );
}

#[test]
fn runtime_source_does_not_define_index_or_policy_as_paid_authority() {
    let source = source_text_under("src");

    assert_contains_none(
        &source,
        "svc-index runtime source",
        &[
            "ron_policy::",
            "svc_wallet::",
            "ron_ledger::",
            "ron_proto::quickchain",
            "quickchain::",
            "unlock_from_index",
            "unlock_from_policy",
            "unlock_from_manifest",
            "unlock_from_name",
            "unlock_from_b3",
            "unlock_from_cache",
            "paid_from_index",
            "paid_from_policy",
            "receipt_from_index",
            "receipt_from_policy",
            "balance_from_index",
            "balance_from_policy",
            "finality_from_index",
            "finality_from_policy",
            "checkpoint_from_index",
            "checkpoint_from_policy",
            "root_from_index",
            "root_from_policy",
            "index_entry_proves_payment",
            "policy_decision_proves_payment",
            "manifest_proves_paid_access",
            "b3_proves_payment",
            "cache_hit_proves_entitlement",
            "create_receipt(",
            "put_receipt(",
            "insert_receipt(",
            "accept_receipt(",
            "commit_receipt(",
            "mutate_balance(",
            "set_balance(",
            "credit_account(",
            "debit_account(",
            "produce_root(",
            "produce_checkpoint(",
            "sign_checkpoint(",
            "anchor_checkpoint(",
            "bridge_settlement(",
        ],
    );
}

#[test]
fn dynamic_preflight_will_pick_up_the_phase1_pair_interlock_suite() {
    let script = read_rel("scripts/dev-quickchain-preflight.sh");

    assert_contains_all(
        &script,
        "svc-index dev-quickchain-preflight.sh",
        &[
            "find \"$TEST_DIR\"",
            "-name 'quickchain*.rs'",
            "basename \"$test_file\" .rs",
            "test -p \"$PKG\" --test \"$test_name\"",
        ],
    );
}
