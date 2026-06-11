//! RO:WHAT — Deterministic Phase 0 receipt-order key bytes for future receipt-root inputs.
//! RO:WHY — ECON/RES: receipt ordering must not inherit database, arrival, map, or scheduler order.
//! RO:INTERACTS — receipt DTO planning, ledger sequence ranges, Phase 0 locked-byte vectors.
//! RO:INVARIANTS — fixed-width big-endian sequence prefix; bytewise txid tie-break; no hashing or roots.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — ordering bytes create no receipt truth, proof, settlement, or ledger authority.
//! RO:TEST — tests/quickchain_receipt_order.rs and receipt_sort_keys_locked_bytes_v1.json.

use super::{validate_ref, QuickChainResult, QuickChainValidationError};

/// Exact number of bytes used for the unsigned ledger-sequence prefix.
pub const QUICKCHAIN_RECEIPT_SORT_KEY_LEDGER_SEQ_BYTES_V1: usize = core::mem::size_of::<u64>();

/// Human-readable audited description of the Phase 0 receipt ordering rule.
pub const QUICKCHAIN_RECEIPT_SORT_KEY_RULE_V1: &str = "u64_be(ledger_seq_start) || utf8(txid)";

/// Derive one deterministic receipt-order key.
///
/// Exact framing:
///
/// `u64_be(ledger_seq_start) || utf8(txid)`
///
/// The fixed-width unsigned big-endian prefix ensures lexicographic byte order
/// matches numeric ledger-sequence order. The canonical txid bytes break ties.
///
/// This helper does not hash the receipt, build a Merkle tree, produce a root,
/// or validate ledger-range overlap.
pub fn quickchain_receipt_sort_key_v1(
    ledger_seq_start: u64,
    txid: &str,
) -> QuickChainResult<Vec<u8>> {
    if ledger_seq_start == 0 {
        return Err(QuickChainValidationError::InvalidField {
            field: "ledger_seq_start",
            reason: "must be greater than zero for receipt ordering",
        });
    }

    validate_ref("txid", txid)?;

    let mut key = Vec::with_capacity(QUICKCHAIN_RECEIPT_SORT_KEY_LEDGER_SEQ_BYTES_V1 + txid.len());

    key.extend_from_slice(&ledger_seq_start.to_be_bytes());
    key.extend_from_slice(txid.as_bytes());

    Ok(key)
}
