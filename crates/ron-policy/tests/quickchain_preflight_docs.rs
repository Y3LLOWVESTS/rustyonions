//! RO:WHAT — Docs gate for ron-policy QuickChain preflight.
//! RO:WHY — Ensures policy remains documented as declarative, not economic authority.
//! RO:INTERACTS — `docs/quickchain-preflight.md`.
//! RO:INVARIANTS — docs must plainly separate policy decisions from wallet/ledger truth.

use std::path::Path;

#[test]
fn quickchain_preflight_doc_exists() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/quickchain-preflight.md");
    assert!(
        path.exists(),
        "missing ron-policy QuickChain preflight doc at {}",
        path.display()
    );
}

#[test]
fn quickchain_preflight_doc_contains_required_boundary_phrases() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/quickchain-preflight.md");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("read {}: {err}", path.display()));

    for phrase in [
        "ron-policy is declarative policy infrastructure",
        "policy decision is not economic truth",
        "policy allow is not paid proof",
        "policy obligation is not receipt proof",
        "policy explanation is not finality proof",
        "economics policy config is not ledger mutation",
        "feature flag is not settlement authority",
        "Policy must not manufacture paid proof",
        "Policy must not manufacture receipt proof",
        "Policy must not manufacture finality proof",
        "Policy must not manufacture balance proof",
    ] {
        assert!(
            text.contains(phrase),
            "quickchain preflight doc missing required phrase: {phrase}"
        );
    }
}

#[test]
fn quickchain_preflight_doc_keeps_forbidden_runtime_scope_explicit() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/quickchain-preflight.md");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("read {}: {err}", path.display()));

    for phrase in [
        "root-producing code",
        "checkpoint-producing code",
        "validator code",
        "settlement code",
        "wallet mutation",
        "ledger mutation",
        "paid unlock finality",
        "external anchors",
        "bridge logic",
        "staking",
        "liquidity",
        "ROX",
        "Solana",
    ] {
        assert!(
            text.contains(phrase),
            "quickchain preflight doc missing forbidden-scope phrase: {phrase}"
        );
    }
}
