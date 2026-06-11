//! RO:WHAT — Versioned domain-separation strings for future QuickChain hash/signature preimages.
//! RO:WHY — ECON/GOV: prevent cross-context hash/signature reuse before roots and validators exist.
//! RO:INTERACTS — future root/hash/signature modules; current quickchain DTO/canonical modules.
//! RO:INVARIANTS — constants only; no hashing; no signatures; no IO; no wallet/ledger mutation.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — every future preimage context must be unique, lowercase, versioned, and test-gated.
//! RO:TEST — tests/quickchain_domain_separation.rs.

use super::{QuickChainResult, QuickChainValidationError};

/// Maximum length for an audited QuickChain domain-separation string.
pub const MAX_QUICKCHAIN_DOMAIN_SEPARATOR_BYTES: usize = 128;

/// Required prefix for all QuickChain domain-separation strings.
pub const QUICKCHAIN_DOMAIN_PREFIX: &str = "quickchain.";

/// Required version suffix for Phase 0A / v1 domain-separation strings.
pub const QUICKCHAIN_DOMAIN_VERSION_SUFFIX_V1: &str = ".v1";

/// Receipt hash context.
///
/// Blueprint name:
/// `receipt_hash_domain = "quickchain.receipt.v1"`
pub const QUICKCHAIN_RECEIPT_HASH_DOMAIN_V1: &str = "quickchain.receipt.v1";

/// Operation-intent hash context.
///
/// This context is reserved for a future `operation_hash` over canonical
/// `QuickChainOperationIntentV1` bytes. This module does not calculate it.
pub const QUICKCHAIN_OPERATION_INTENT_HASH_DOMAIN_V1: &str = "quickchain.operation-intent.v1";

/// Account-state leaf hash context.
///
/// Blueprint name:
/// `account_leaf_hash_domain = "quickchain.account-state.v1"`
pub const QUICKCHAIN_ACCOUNT_LEAF_HASH_DOMAIN_V1: &str = "quickchain.account-state.v1";

/// Hold-state leaf hash context.
///
/// Blueprint name:
/// `hold_leaf_hash_domain = "quickchain.hold-state.v1"`
pub const QUICKCHAIN_HOLD_LEAF_HASH_DOMAIN_V1: &str = "quickchain.hold-state.v1";

/// Active hold-tree root hash context.
///
/// This is distinct from the hold-state leaf domain so an empty or non-empty
/// hold-tree commitment cannot be confused with one hold leaf commitment.
pub const QUICKCHAIN_HOLD_ROOT_HASH_DOMAIN_V1: &str = "quickchain.hold-root.v1";

/// Receipt root hash context.
///
/// Blueprint name:
/// `receipt_root_domain = "quickchain.receipt-root.v1"`
pub const QUICKCHAIN_RECEIPT_ROOT_HASH_DOMAIN_V1: &str = "quickchain.receipt-root.v1";

/// Global state root hash context.
///
/// Blueprint name:
/// `state_root_domain = "quickchain.state-root.v1"`
pub const QUICKCHAIN_STATE_ROOT_HASH_DOMAIN_V1: &str = "quickchain.state-root.v1";

/// Accounting root hash context.
///
/// Blueprint name:
/// `accounting_root_domain = "quickchain.accounting-root.v1"`
pub const QUICKCHAIN_ACCOUNTING_ROOT_HASH_DOMAIN_V1: &str = "quickchain.accounting-root.v1";

/// Reward root hash context.
///
/// Blueprint name:
/// `reward_root_domain = "quickchain.reward-root.v1"`
pub const QUICKCHAIN_REWARD_ROOT_HASH_DOMAIN_V1: &str = "quickchain.reward-root.v1";

/// Checkpoint hash context.
///
/// Blueprint name:
/// `checkpoint_hash_domain = "quickchain.checkpoint.v1"`
pub const QUICKCHAIN_CHECKPOINT_HASH_DOMAIN_V1: &str = "quickchain.checkpoint.v1";

/// Chain params hash context.
pub const QUICKCHAIN_CHAIN_PARAMS_HASH_DOMAIN_V1: &str = "quickchain.chain-params.v1";

/// Validator-set hash context.
pub const QUICKCHAIN_VALIDATOR_SET_HASH_DOMAIN_V1: &str = "quickchain.validator-set.v1";

/// Policy hash context.
pub const QUICKCHAIN_POLICY_HASH_DOMAIN_V1: &str = "quickchain.policy.v1";

/// Data-availability root context.
pub const QUICKCHAIN_DATA_AVAILABILITY_ROOT_HASH_DOMAIN_V1: &str =
    "quickchain.data-availability-root.v1";

/// Receipt-batch artifact context.
pub const QUICKCHAIN_RECEIPT_BATCH_DOMAIN_V1: &str = "quickchain.receipt-batch.v1";

/// Accounting-window artifact context.
pub const QUICKCHAIN_ACCOUNTING_WINDOW_DOMAIN_V1: &str = "quickchain.accounting-window.v1";

/// Reward-manifest artifact context.
pub const QUICKCHAIN_REWARD_MANIFEST_DOMAIN_V1: &str = "quickchain.reward-manifest.v1";

/// Challenge evidence context.
pub const QUICKCHAIN_CHALLENGE_EVIDENCE_DOMAIN_V1: &str = "quickchain.challenge-evidence.v1";

/// External anchor payload context.
///
/// This is a string constant only. It does not enable anchors.
pub const QUICKCHAIN_ANCHOR_PAYLOAD_DOMAIN_V1: &str = "quickchain.anchor-payload.v1";

/// All Phase 0A v1 QuickChain domain-separation strings.
///
/// Keep this list stable and test-gated. Adding a new item is allowed only when
/// the context is documented and has a matching test update.
pub const QUICKCHAIN_DOMAIN_SEPARATORS_V1: &[&str] = &[
    QUICKCHAIN_RECEIPT_HASH_DOMAIN_V1,
    QUICKCHAIN_OPERATION_INTENT_HASH_DOMAIN_V1,
    QUICKCHAIN_ACCOUNT_LEAF_HASH_DOMAIN_V1,
    QUICKCHAIN_HOLD_LEAF_HASH_DOMAIN_V1,
    QUICKCHAIN_HOLD_ROOT_HASH_DOMAIN_V1,
    QUICKCHAIN_RECEIPT_ROOT_HASH_DOMAIN_V1,
    QUICKCHAIN_STATE_ROOT_HASH_DOMAIN_V1,
    QUICKCHAIN_ACCOUNTING_ROOT_HASH_DOMAIN_V1,
    QUICKCHAIN_REWARD_ROOT_HASH_DOMAIN_V1,
    QUICKCHAIN_CHECKPOINT_HASH_DOMAIN_V1,
    QUICKCHAIN_CHAIN_PARAMS_HASH_DOMAIN_V1,
    QUICKCHAIN_VALIDATOR_SET_HASH_DOMAIN_V1,
    QUICKCHAIN_POLICY_HASH_DOMAIN_V1,
    QUICKCHAIN_DATA_AVAILABILITY_ROOT_HASH_DOMAIN_V1,
    QUICKCHAIN_RECEIPT_BATCH_DOMAIN_V1,
    QUICKCHAIN_ACCOUNTING_WINDOW_DOMAIN_V1,
    QUICKCHAIN_REWARD_MANIFEST_DOMAIN_V1,
    QUICKCHAIN_CHALLENGE_EVIDENCE_DOMAIN_V1,
    QUICKCHAIN_ANCHOR_PAYLOAD_DOMAIN_V1,
];

/// Validate one v1 QuickChain domain-separation string.
pub fn validate_domain_separator_v1(separator: &str) -> QuickChainResult<()> {
    if separator.is_empty() {
        return Err(QuickChainValidationError::InvalidField {
            field: "domain_separator",
            reason: "must not be empty",
        });
    }

    if separator.len() > MAX_QUICKCHAIN_DOMAIN_SEPARATOR_BYTES {
        return Err(QuickChainValidationError::InvalidField {
            field: "domain_separator",
            reason: "must not exceed maximum length",
        });
    }

    if !separator.starts_with(QUICKCHAIN_DOMAIN_PREFIX) {
        return Err(QuickChainValidationError::InvalidField {
            field: "domain_separator",
            reason: "must start with quickchain.",
        });
    }

    if !separator.ends_with(QUICKCHAIN_DOMAIN_VERSION_SUFFIX_V1) {
        return Err(QuickChainValidationError::InvalidField {
            field: "domain_separator",
            reason: "must end with .v1",
        });
    }

    if separator.contains("..") {
        return Err(QuickChainValidationError::InvalidField {
            field: "domain_separator",
            reason: "must not contain empty segments",
        });
    }

    if !separator.bytes().all(is_allowed_domain_separator_byte) {
        return Err(QuickChainValidationError::InvalidField {
            field: "domain_separator",
            reason: "must be lowercase ASCII letters, digits, dots, underscores, or hyphens",
        });
    }

    Ok(())
}

/// Validate every built-in v1 QuickChain domain-separation string.
pub fn validate_all_domain_separators_v1() -> QuickChainResult<()> {
    for separator in QUICKCHAIN_DOMAIN_SEPARATORS_V1 {
        validate_domain_separator_v1(separator)?;
    }

    for (left_index, left) in QUICKCHAIN_DOMAIN_SEPARATORS_V1.iter().enumerate() {
        for right in QUICKCHAIN_DOMAIN_SEPARATORS_V1.iter().skip(left_index + 1) {
            if left == right {
                return Err(QuickChainValidationError::InvalidField {
                    field: "domain_separators",
                    reason: "must be unique",
                });
            }
        }
    }

    Ok(())
}

fn is_allowed_domain_separator_byte(byte: u8) -> bool {
    byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
}
