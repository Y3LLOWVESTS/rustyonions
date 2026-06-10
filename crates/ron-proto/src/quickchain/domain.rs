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

/// Required version suffix for Phase 0 / v1 domain-separation strings.
pub const QUICKCHAIN_DOMAIN_VERSION_SUFFIX_V1: &str = ".v1";

/// Account-state Merkle leaf context.
pub const QUICKCHAIN_ACCOUNT_STATE_LEAF_DOMAIN_V1: &str = "quickchain.account_state.leaf.v1";

/// Account-state Merkle internal-node context.
pub const QUICKCHAIN_ACCOUNT_STATE_NODE_DOMAIN_V1: &str = "quickchain.account_state.node.v1";

/// Account-state empty-root context.
pub const QUICKCHAIN_ACCOUNT_STATE_EMPTY_DOMAIN_V1: &str = "quickchain.account_state.empty.v1";

/// Receipt Merkle leaf context.
pub const QUICKCHAIN_RECEIPT_LEAF_DOMAIN_V1: &str = "quickchain.receipt.leaf.v1";

/// Receipt Merkle internal-node context.
pub const QUICKCHAIN_RECEIPT_NODE_DOMAIN_V1: &str = "quickchain.receipt.node.v1";

/// Receipt empty-root context.
pub const QUICKCHAIN_RECEIPT_EMPTY_DOMAIN_V1: &str = "quickchain.receipt.empty.v1";

/// Receipt-batch metadata/header context.
pub const QUICKCHAIN_RECEIPT_BATCH_HEADER_DOMAIN_V1: &str = "quickchain.receipt_batch.header.v1";

/// Checkpoint header context.
pub const QUICKCHAIN_CHECKPOINT_HEADER_DOMAIN_V1: &str = "quickchain.checkpoint.header.v1";

/// Validator checkpoint signature context.
pub const QUICKCHAIN_CHECKPOINT_SIGNATURE_DOMAIN_V1: &str = "quickchain.checkpoint.signature.v1";

/// Validator-set descriptor context.
pub const QUICKCHAIN_VALIDATOR_SET_DOMAIN_V1: &str = "quickchain.validator_set.v1";

/// Chain-params descriptor context.
pub const QUICKCHAIN_CHAIN_PARAMS_DOMAIN_V1: &str = "quickchain.chain_params.v1";

/// Challenge evidence context.
pub const QUICKCHAIN_CHALLENGE_EVIDENCE_DOMAIN_V1: &str = "quickchain.challenge.evidence.v1";

/// Data-availability Merkle leaf context.
pub const QUICKCHAIN_DATA_AVAILABILITY_LEAF_DOMAIN_V1: &str =
    "quickchain.data_availability.leaf.v1";

/// Data-availability Merkle internal-node context.
pub const QUICKCHAIN_DATA_AVAILABILITY_NODE_DOMAIN_V1: &str =
    "quickchain.data_availability.node.v1";

/// Data-availability empty-root context.
pub const QUICKCHAIN_DATA_AVAILABILITY_EMPTY_DOMAIN_V1: &str =
    "quickchain.data_availability.empty.v1";

/// Accounting snapshot root context.
pub const QUICKCHAIN_ACCOUNTING_SNAPSHOT_DOMAIN_V1: &str = "quickchain.accounting_snapshot.v1";

/// Reward manifest root context.
pub const QUICKCHAIN_REWARD_MANIFEST_DOMAIN_V1: &str = "quickchain.reward_manifest.v1";

/// External anchor payload context.
///
/// This is a string constant only. It does not enable anchors.
pub const QUICKCHAIN_ANCHOR_PAYLOAD_DOMAIN_V1: &str = "quickchain.anchor_payload.v1";

/// All Phase 0 v1 QuickChain domain-separation strings.
///
/// Keep this list stable and test-gated. Adding a new item is allowed only when
/// the context is documented and has a matching test update.
pub const QUICKCHAIN_DOMAIN_SEPARATORS_V1: &[&str] = &[
    QUICKCHAIN_ACCOUNT_STATE_LEAF_DOMAIN_V1,
    QUICKCHAIN_ACCOUNT_STATE_NODE_DOMAIN_V1,
    QUICKCHAIN_ACCOUNT_STATE_EMPTY_DOMAIN_V1,
    QUICKCHAIN_RECEIPT_LEAF_DOMAIN_V1,
    QUICKCHAIN_RECEIPT_NODE_DOMAIN_V1,
    QUICKCHAIN_RECEIPT_EMPTY_DOMAIN_V1,
    QUICKCHAIN_RECEIPT_BATCH_HEADER_DOMAIN_V1,
    QUICKCHAIN_CHECKPOINT_HEADER_DOMAIN_V1,
    QUICKCHAIN_CHECKPOINT_SIGNATURE_DOMAIN_V1,
    QUICKCHAIN_VALIDATOR_SET_DOMAIN_V1,
    QUICKCHAIN_CHAIN_PARAMS_DOMAIN_V1,
    QUICKCHAIN_CHALLENGE_EVIDENCE_DOMAIN_V1,
    QUICKCHAIN_DATA_AVAILABILITY_LEAF_DOMAIN_V1,
    QUICKCHAIN_DATA_AVAILABILITY_NODE_DOMAIN_V1,
    QUICKCHAIN_DATA_AVAILABILITY_EMPTY_DOMAIN_V1,
    QUICKCHAIN_ACCOUNTING_SNAPSHOT_DOMAIN_V1,
    QUICKCHAIN_REWARD_MANIFEST_DOMAIN_V1,
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
            reason: "must be lowercase ASCII letters, digits, dots, or underscores",
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
    byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_')
}
