//! RO:WHAT — Documentation contract tests for svc-index QuickChain Phase-0 preflight.
//! RO:WHY — Keep lookup/pointer authority distinct from economic, paid-access, and QuickChain authority.
//! RO:INTERACTS — docs/quickchain-preflight.md.
//! RO:INVARIANTS — index truth is not economic truth; manifests/names/b3/cache do not unlock paid content.
//! RO:TEST — run with `cargo test -p svc-index --test quickchain_preflight_docs`.

use std::{fs, path::PathBuf};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_preflight_doc() -> String {
    let path = manifest_dir().join("docs/quickchain-preflight.md");
    fs::read_to_string(&path).unwrap_or_else(|err| {
        panic!(
            "failed to read QuickChain preflight docs at {}: {err}",
            path.display()
        )
    })
}

fn assert_contains_all(haystack: &str, needles: &[&str]) {
    for needle in needles {
        assert!(
            haystack.contains(needle),
            "QuickChain preflight docs must contain required phrase: {needle:?}"
        );
    }
}

#[test]
fn docs_define_svc_index_as_lookup_pointer_service_only() {
    let doc = read_preflight_doc().to_lowercase();

    assert_contains_all(
        &doc,
        &[
            "svc-index is a lookup and pointer service",
            "index truth is not economic truth",
            "pointer truth is not receipt truth",
            "name resolution is not ownership proof",
            "b3 byte identity is not payment proof",
            "manifest lookup is not paid unlock",
            "policy metadata is not wallet authority",
            "provider lookup is not settlement finality",
        ],
    );
}

#[test]
fn docs_preserve_backend_paid_access_truth_path() {
    let doc = read_preflight_doc().to_lowercase();

    assert_contains_all(
        &doc,
        &[
            "paid access must be proven through backend service paths",
            "svc-wallet mutation or lookup path",
            "ron-ledger durable receipt truth",
            "svc-storage / gateway paid enforcement",
            "index entry → unlock",
            "manifest lookup → unlock",
            "cache hit → unlock",
            "client says paid → unlock",
        ],
    );
}

#[test]
fn docs_forbid_quickchain_runtime_and_external_settlement_scope() {
    let doc = read_preflight_doc().to_lowercase();

    assert_contains_all(
        &doc,
        &[
            "roots",
            "state roots",
            "receipt roots",
            "checkpoints",
            "validators",
            "consensus",
            "settlement",
            "external anchors",
            "bridges",
            "rox",
            "solana",
            "staking",
            "liquidity",
            "root-producing index snapshots",
            "fake receipts",
            "fake balances",
            "fake finality",
            "fake unlocks",
        ],
    );
}

#[test]
fn docs_warn_not_to_ban_legitimate_metadata_blindly() {
    let doc = read_preflight_doc().to_lowercase();

    assert_contains_all(
        &doc,
        &[
            "do not ban legitimate metadata words blindly",
            "owner",
            "creator",
            "recipient",
            "policy",
            "manifest",
            "wallet as a reference string",
            "ban or constrain authority-shaped usage",
            "unlock_from_index",
            "receipt_from_index",
            "balance_from_index",
            "finality_from_index",
        ],
    );
}
