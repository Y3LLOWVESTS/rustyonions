//! RO:WHAT — Deterministic Phase 0 sort-key helpers for future QuickChain account and hold leaves.
//! RO:WHY — ECON/RES: roots must never depend on database, map, locale, or arrival ordering.
//! RO:INTERACTS — account/hold DTO planning, QC-0A state-tree vectors, future root code outside ron-proto.
//! RO:INVARIANTS — bytewise ascending only; duplicate keys reject; ROC asset is exact; no hashing or roots.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — helpers validate key inputs but create no proof, balance truth, receipt, or economic authority.
//! RO:TEST — tests/quickchain_sort_keys.rs and sort_keys_locked_bytes_v1.json.

use super::{ids::validate_hold_id_v1, validate_ref, QuickChainResult, QuickChainValidationError};

/// Internal ROC asset token used by the Phase 0 account-leaf sort rule.
pub const QUICKCHAIN_ACCOUNT_LEAF_ASSET_ROC_V1: &str = "roc";

/// Exact separator between account ID and asset bytes.
pub const QUICKCHAIN_ACCOUNT_LEAF_SORT_KEY_DELIMITER_V1: u8 = 0x00;

/// Derive the Phase 0 account-leaf sort key.
///
/// Exact framing:
///
/// `utf8(account_id) || 0x00 || utf8(asset)`
///
/// During the internal ROC phase, `asset` must be exactly `"roc"`.
pub fn quickchain_account_leaf_sort_key_v1(
    account_id: &str,
    asset: &str,
) -> QuickChainResult<Vec<u8>> {
    validate_ref("account_id", account_id)?;

    if asset != QUICKCHAIN_ACCOUNT_LEAF_ASSET_ROC_V1 {
        return Err(QuickChainValidationError::InvalidField {
            field: "asset",
            reason: "must be roc for the Phase 0 account-leaf sort key",
        });
    }

    let mut key = Vec::with_capacity(account_id.len() + 1 + asset.len());
    key.extend_from_slice(account_id.as_bytes());
    key.push(QUICKCHAIN_ACCOUNT_LEAF_SORT_KEY_DELIMITER_V1);
    key.extend_from_slice(asset.as_bytes());

    Ok(key)
}

/// Derive the Phase 0 active-hold leaf sort key.
///
/// Exact framing:
///
/// `utf8(hold_id)`
pub fn quickchain_hold_leaf_sort_key_v1(hold_id: &str) -> QuickChainResult<Vec<u8>> {
    validate_hold_id_v1("hold_id", hold_id)?;
    Ok(hold_id.as_bytes().to_vec())
}

/// Sort keys using Rust byte-vector ordering and reject duplicates.
///
/// `Vec<u8>` ordering is lexicographic bytewise ordering. This helper does not
/// hash keys, build a tree, or claim that a root has been produced.
pub fn sort_quickchain_keys_v1(keys: &mut [Vec<u8>]) -> QuickChainResult<()> {
    keys.sort_unstable();
    validate_quickchain_sorted_unique_keys_v1(keys)
}

/// Validate that keys are nonempty and strictly bytewise ascending.
///
/// Empty collections are valid because an empty tree has a separate canonical
/// payload. Individual empty keys are invalid.
pub fn validate_quickchain_sorted_unique_keys_v1(keys: &[Vec<u8>]) -> QuickChainResult<()> {
    for key in keys {
        if key.is_empty() {
            return Err(QuickChainValidationError::InvalidField {
                field: "sort_keys",
                reason: "sort keys must not be empty",
            });
        }
    }

    for pair in keys.windows(2) {
        let left = pair[0].as_slice();
        let right = pair[1].as_slice();

        if left == right {
            return Err(QuickChainValidationError::InvalidField {
                field: "sort_keys",
                reason: "duplicate sort keys are forbidden",
            });
        }

        if left > right {
            return Err(QuickChainValidationError::InvalidField {
                field: "sort_keys",
                reason: "sort keys must be strictly bytewise ascending",
            });
        }
    }

    Ok(())
}
