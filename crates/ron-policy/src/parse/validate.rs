//! RO:WHAT — Structural validation for `PolicyBundle`.
//!
//! RO:WHY — Keep policy declarative: deny ambiguous bundles, oversized caps, and
//! authority-shaped conditions/obligations that could be mistaken for wallet/ledger truth.
//!
//! RO:INTERACTS — `model::PolicyBundle`, `model::Obligation`, parse loaders.
//!
//! RO:INVARIANTS — deny-by-default; no paid-unlock authority; no receipt/balance/finality authority.

use crate::{
    errors::Error,
    model::{Obligation, PolicyBundle},
};
use std::collections::BTreeSet;

/// Validate a `PolicyBundle` for basic invariants.
///
/// # Errors
///
/// Returns `Error::Validation` if the bundle violates invariants such as duplicate rule IDs,
/// empty IDs, body caps over 1 MiB, authority-shaped condition tags, or authority-shaped obligations.
pub fn validate(b: &PolicyBundle) -> Result<(), Error> {
    if b.version == 0 {
        return Err(Error::Validation("version must be ≥ 1".into()));
    }

    let mut ids = BTreeSet::<&str>::new();
    for r in &b.rules {
        if r.id.trim().is_empty() {
            return Err(Error::Validation("rule.id must be non-empty".into()));
        }
        if !ids.insert(&r.id) {
            return Err(Error::Validation(format!("duplicate rule id: {}", r.id)));
        }
        if let Some(n) = r.when.max_body_bytes {
            if n > 1_048_576 {
                return Err(Error::Validation(format!(
                    "rule {} max_body_bytes > 1MiB",
                    r.id
                )));
            }
        }

        validate_condition_tags(&r.id, &r.when.require_tags_all)?;
        validate_obligations(&r.id, &r.obligations)?;
    }

    if let Some(n) = b.defaults.max_body_bytes {
        if n > 1_048_576 {
            return Err(Error::Validation("defaults.max_body_bytes > 1MiB".into()));
        }
    }

    Ok(())
}

fn validate_condition_tags(rule_id: &str, tags: &[String]) -> Result<(), Error> {
    for (index, tag) in tags.iter().enumerate() {
        if tag.trim().is_empty() {
            return Err(Error::Validation(format!(
                "rule {rule_id} require_tags_all {index} must be non-empty"
            )));
        }

        if is_forbidden_authority_condition_tag(tag) {
            return Err(Error::Validation(format!(
                "rule {rule_id} require_tags_all {index} looks like economic authority: {tag}"
            )));
        }
    }

    Ok(())
}

fn validate_obligations(rule_id: &str, obligations: &[Obligation]) -> Result<(), Error> {
    for (index, obligation) in obligations.iter().enumerate() {
        if obligation.kind.trim().is_empty() {
            return Err(Error::Validation(format!(
                "rule {rule_id} obligation {index} kind must be non-empty"
            )));
        }

        if is_forbidden_authority_kind(&obligation.kind) {
            return Err(Error::Validation(format!(
                "rule {rule_id} obligation {index} kind looks like economic authority: {}",
                obligation.kind
            )));
        }

        for key in obligation.params.keys() {
            if is_forbidden_authority_param_key(key) {
                return Err(Error::Validation(format!(
                    "rule {rule_id} obligation {index} param key looks like economic authority: {key}"
                )));
            }
        }
    }

    Ok(())
}

fn normalize_authority_token(input: &str) -> String {
    input
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .flat_map(char::to_lowercase)
        .collect()
}

fn is_forbidden_authority_condition_tag(tag: &str) -> bool {
    matches!(
        normalize_authority_token(tag).as_str(),
        "receiptid"
            | "receipthash"
            | "receiptroot"
            | "receiptproof"
            | "accountproof"
            | "inclusionproof"
            | "balance"
            | "balanceminor"
            | "walletbalance"
            | "ledgerbalance"
            | "finality"
            | "finalized"
            | "unlockgranted"
            | "paidproof"
            | "settlementstatus"
            | "spendauthority"
            | "captureauthority"
            | "stateroot"
            | "checkpointroot"
            | "checkpointhash"
            | "validatorsignature"
            | "bridgeproof"
            | "mintauthority"
            | "operationid"
            | "idempotencykey"
            | "accountsequence"
            | "holdid"
    )
}

fn is_forbidden_authority_param_key(key: &str) -> bool {
    matches!(
        normalize_authority_token(key).as_str(),
        "receiptid"
            | "receipthash"
            | "receiptroot"
            | "receiptproof"
            | "accountproof"
            | "inclusionproof"
            | "balance"
            | "balanceminor"
            | "walletbalance"
            | "ledgerbalance"
            | "finality"
            | "finalized"
            | "unlockgranted"
            | "paidproof"
            | "settlementstatus"
            | "spendauthority"
            | "captureauthority"
            | "stateroot"
            | "checkpointroot"
            | "checkpointhash"
            | "validatorsignature"
            | "bridgeproof"
            | "mintauthority"
            | "operationid"
            | "idempotencykey"
            | "accountsequence"
            | "holdid"
    )
}

fn is_forbidden_authority_kind(kind: &str) -> bool {
    const FORBIDDEN_KIND_SHAPES: &[&str] = &[
        "issue",
        "transfer",
        "burn",
        "mintroc",
        "allocateroc",
        "createreceipt",
        "putreceipt",
        "insertreceipt",
        "acceptreceipt",
        "commitreceipt",
        "finalizereceipt",
        "verifyreceipt",
        "verifypayment",
        "mutatebalance",
        "setbalance",
        "creditaccount",
        "debitaccount",
        "openhold",
        "capturehold",
        "releasehold",
        "expirehold",
        "commithold",
        "unlockpaidcontent",
        "provepaymentfinality",
        "validateproof",
        "verifyproof",
        "produceaccountproof",
        "producereceiptproof",
        "producestateproof",
        "producecheckpoint",
        "produceroot",
        "signcheckpoint",
        "anchorcheckpoint",
        "settlementcomplete",
        "settlementfinalized",
        "bridgesettlement",
    ];

    let normalized = normalize_authority_token(kind);
    FORBIDDEN_KIND_SHAPES
        .iter()
        .any(|shape| normalized.contains(shape))
}
