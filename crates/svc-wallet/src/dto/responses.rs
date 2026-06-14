//! RO:WHAT — Strict response DTOs for wallet v1 balances, receipts, and commit status.
//! RO:WHY  — Pillar 12; Concerns: ECON/DX/GOV. Receipts are the client-visible proof surface.
//! RO:INTERACTS — util::blake3_receipt, ledger::client, routes/v1.
//! RO:INVARIANTS — amount strings; receipt_hash is computed over canonical fields excluding receipt_hash itself.
//! RO:METRICS — route layer increments success counters by WalletOp.
//! RO:CONFIG — no direct config reads.
//! RO:SECURITY — identifiers only; no bearer tokens or secrets are serialized here.
//! RO:TEST — receipt_hash_is_deterministic.

use serde::{Deserialize, Serialize};

use crate::dto::requests::AmountMinor;

/// Wallet operation reflected in receipts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WalletOp {
    /// Supply-creating issue/mint.
    Issue,
    /// Account-to-account movement.
    Transfer,
    /// Supply-destroying burn.
    Burn,
    /// Escrow hold reserved for paid storage/pinning slice.
    Hold,
    /// Escrow capture reserved for paid storage/pinning slice.
    Capture,
    /// Escrow release reserved for paid storage/pinning slice.
    Release,
}

impl WalletOp {
    /// Stable lower-case label.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Issue => "issue",
            Self::Transfer => "transfer",
            Self::Burn => "burn",
            Self::Hold => "hold",
            Self::Capture => "capture",
            Self::Release => "release",
        }
    }
}

/// Honest settlement status exposed by svc-wallet receipts.
///
/// svc-wallet may honestly report `accepted` after the wallet/ledger hot path
/// commits. Stronger labels such as epoch-included, finalized, or anchored are
/// future QuickChain settlement states and must not be invented by svc-wallet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ReceiptSettlementStatus {
    /// svc-wallet/ron-ledger accepted the mutation.
    Accepted,
}

impl ReceiptSettlementStatus {
    /// Stable lower-case label.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
        }
    }
}

/// Balance response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BalanceResponse {
    /// Account identifier.
    pub account: String,
    /// Asset identifier.
    pub asset: String,
    /// Balance in minor units as a string.
    pub amount_minor: AmountMinor,
    /// Ledger height/sequence observed for this read when known.
    pub as_of_height: Option<u64>,
    /// Cache staleness in milliseconds.
    pub stale_ms: u64,
}

/// Common short receipt for issue/transfer/burn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    /// Deterministic wallet transaction id.
    pub txid: String,
    /// Operation kind.
    pub op: WalletOp,
    /// Debit-side account where applicable.
    pub from: Option<String>,
    /// Credit-side account where applicable.
    pub to: Option<String>,
    /// Asset identifier.
    pub asset: String,
    /// Amount in minor units.
    pub amount_minor: AmountMinor,
    /// Debit-side nonce where applicable.
    pub nonce: Option<u64>,
    /// Idempotency key echoed to the caller.
    pub idem: String,
    /// Server/ledger timestamp in unix milliseconds.
    pub ts: u64,
    /// First ledger sequence assigned when known.
    pub ledger_seq_start: Option<u64>,
    /// Last ledger sequence assigned when known.
    pub ledger_seq_end: Option<u64>,
    /// Ledger accumulator root after commit.
    pub ledger_root: String,
    /// Honest wallet-side settlement status.
    pub settlement_status: ReceiptSettlementStatus,
    /// BLAKE3 hash over the canonical receipt fields excluding this field.
    pub receipt_hash: String,
}

/// Canonical receipt preimage used for hashing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReceiptHashPreimage<'a> {
    /// Deterministic wallet transaction id.
    pub txid: &'a str,
    /// Operation kind.
    pub op: WalletOp,
    /// Debit-side account where applicable.
    pub from: &'a Option<String>,
    /// Credit-side account where applicable.
    pub to: &'a Option<String>,
    /// Asset identifier.
    pub asset: &'a str,
    /// Amount in minor units.
    pub amount_minor: AmountMinor,
    /// Debit-side nonce where applicable.
    pub nonce: Option<u64>,
    /// Idempotency key.
    pub idem: &'a str,
    /// Server timestamp.
    pub ts: u64,
    /// First ledger sequence assigned when known.
    pub ledger_seq_start: Option<u64>,
    /// Last ledger sequence assigned when known.
    pub ledger_seq_end: Option<u64>,
    /// Ledger root.
    pub ledger_root: &'a str,
    /// Honest wallet-side settlement status.
    pub settlement_status: ReceiptSettlementStatus,
}

impl Receipt {
    /// Return the canonical hash preimage.
    pub fn hash_preimage(&self) -> ReceiptHashPreimage<'_> {
        ReceiptHashPreimage {
            txid: &self.txid,
            op: self.op,
            from: &self.from,
            to: &self.to,
            asset: &self.asset,
            amount_minor: self.amount_minor,
            nonce: self.nonce,
            idem: &self.idem,
            ts: self.ts,
            ledger_seq_start: self.ledger_seq_start,
            ledger_seq_end: self.ledger_seq_end,
            ledger_root: &self.ledger_root,
            settlement_status: self.settlement_status,
        }
    }
}
