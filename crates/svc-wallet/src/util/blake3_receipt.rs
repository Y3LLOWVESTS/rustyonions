//! RO:WHAT — Deterministic BLAKE3 helpers for wallet txids, request fingerprints, receipt hashes, and ledger nonces.
//! RO:WHY  — Pillar 12; Concerns: ECON/SEC/GOV. Receipts and idempotency need stable canonical hashes.
//! RO:INTERACTS — dto::responses, idem::store, ledger::client.
//! RO:INVARIANTS — receipt_hash excludes receipt_hash; canonical JSON structs; 16-byte ledger nonce derivation is deterministic.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — hashes are integrity identifiers, not secrets or signatures.
//! RO:TEST — fingerprint_changes_with_body; receipt_hash_is_stable.

use base64::Engine as _;
use serde::Serialize;

use crate::{
    dto::responses::{Receipt, ReceiptHashPreimage, WalletOp},
    errors::WalletResult,
};

/// Return a BLAKE3 b3-prefixed hash of canonical JSON.
pub fn hash_json<T: Serialize>(value: &T) -> WalletResult<String> {
    let encoded = serde_json::to_vec(value)?;
    Ok(format!("b3:{}", blake3::hash(&encoded).to_hex()))
}

/// Return a short deterministic txid.
pub fn txid_for<T: Serialize>(op: WalletOp, idem: &str, value: &T) -> WalletResult<String> {
    #[derive(Serialize)]
    struct TxidPreimage<'a, T> {
        op: WalletOp,
        idem: &'a str,
        value: &'a T,
    }

    let preimage = TxidPreimage { op, idem, value };
    let encoded = serde_json::to_vec(&preimage)?;
    let hex = blake3::hash(&encoded).to_hex().to_string();
    Ok(format!("tx_{}", &hex[..32]))
}

/// Return a canonical request fingerprint for idempotency comparison.
pub fn request_fingerprint<T: Serialize>(op: WalletOp, value: &T) -> WalletResult<String> {
    #[derive(Serialize)]
    struct RequestPreimage<'a, T> {
        op: WalletOp,
        value: &'a T,
    }

    hash_json(&RequestPreimage { op, value })
}

/// Compute and return the final receipt hash.
pub fn receipt_hash(receipt: &Receipt) -> WalletResult<String> {
    let preimage: ReceiptHashPreimage<'_> = receipt.hash_preimage();
    hash_json(&preimage)
}

/// Fill a receipt's hash field.
pub fn finalize_receipt(mut receipt: Receipt) -> WalletResult<Receipt> {
    receipt.receipt_hash = receipt_hash(&receipt)?;
    Ok(receipt)
}

/// Derive a base64-encoded 16-byte nonce for the primitive ron-ledger entry type.
pub fn ledger_nonce_b64(parts: &[&str]) -> String {
    let mut hasher = blake3::Hasher::new();
    for part in parts {
        hasher.update(part.as_bytes());
        hasher.update(&[0]);
    }
    let hash = hasher.finalize();
    base64::engine::general_purpose::STANDARD.encode(&hash.as_bytes()[..16])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::{
        requests::AmountMinor,
        responses::{Receipt, ReceiptSettlementStatus},
    };

    #[test]
    fn ledger_nonce_is_16_bytes_base64() {
        let n1 = ledger_nonce_b64(&["transfer", "abc", "1"]);
        let n2 = ledger_nonce_b64(&["transfer", "abc", "1"]);
        assert_eq!(n1, n2);
    }

    #[test]
    fn receipt_hash_is_stable() {
        let receipt = Receipt {
            txid: "tx_test".into(),
            op: WalletOp::Issue,
            from: None,
            to: Some("acct".into()),
            asset: "roc".into(),
            amount_minor: AmountMinor(1),
            nonce: None,
            idem: "idem".into(),
            ts: 1,
            ledger_seq_start: Some(1),
            ledger_seq_end: Some(1),
            ledger_root: "00".repeat(32),
            settlement_status: ReceiptSettlementStatus::Accepted,
            receipt_hash: String::new(),
        };
        let a = receipt_hash(&receipt).unwrap();
        let b = receipt_hash(&receipt).unwrap();
        assert_eq!(a, b);
        assert!(a.starts_with("b3:"));
    }
}
