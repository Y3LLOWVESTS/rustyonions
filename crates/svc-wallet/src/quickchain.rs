//! RO:WHAT — Inert QuickChain preflight projection helpers for svc-wallet receipts.
//! RO:WHY — svc-wallet is the ROC mutation front-door; future QuickChain receipt
//! vectors need wallet receipts to expose explicit operation/idempotency/receipt
//! fields without turning wallet into chain authority.
//! RO:INTERACTS — dto::responses::Receipt, WalletOp, AmountMinor, and future
//! ron-proto QuickChain receipt DTO/vector work.
//! RO:INVARIANTS — projection only; no roots, no checkpoint production, no
//! validators, no settlement, no anchors, no bridge logic, no pruning, no live
//! ledger mutation, no fake finality.
//! RO:METRICS — none.
//! RO:CONFIG — available only with the quickchain-preflight feature.
//! RO:SECURITY — caller must provide backend-assigned operation_id and chain_id;
//! this module never derives economic authority from client idempotency keys.
//! RO:TEST — tests/quickchain_preflight_boundary.rs.

use serde::{Deserialize, Serialize};

use crate::{
    config::DEFAULT_ASSET,
    dto::{
        requests::AmountMinor,
        responses::{Receipt, WalletOp},
    },
    errors::{WalletError, WalletResult},
    util::parsing::{validate_account_id, validate_idempotency_key},
};

/// Schema label for the inert svc-wallet receipt projection.
///
/// This is intentionally not `quickchain.receipt.v1`; the canonical chain DTO
/// remains owned by ron-proto. This projection is a wallet-side compatibility
/// seam for tests, review, and future vector planning.
pub const SVC_WALLET_QUICKCHAIN_RECEIPT_PROJECTION_SCHEMA: &str =
    "svc-wallet.quickchain-receipt-projection.v1";

/// Maximum preflight chain id bytes accepted by this wallet adapter.
pub const MAX_PREFLIGHT_CHAIN_ID_BYTES: usize = 96;
/// Maximum backend operation id bytes accepted by this wallet adapter.
pub const MAX_PREFLIGHT_OPERATION_ID_BYTES: usize = 128;
/// Maximum wallet txid bytes accepted by this wallet adapter.
pub const MAX_PREFLIGHT_TXID_BYTES: usize = 96;

/// Honest receipt settlement label for the wallet hot path.
///
/// Only `Accepted` is produced by this wallet preflight module. Stronger states
/// such as epoch-included, finalized, or anchored are future QuickChain states
/// and must not be invented here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainWalletReceiptStatus {
    /// svc-wallet/ron-ledger accepted the mutation.
    Accepted,
}

/// Explicit context required to project a wallet receipt toward future
/// QuickChain receipt-vector work.
///
/// The wallet receipt already has `txid`, `idem`, ledger sequence hints, legacy
/// ledger root, and receipt hash. The caller must still supply `operation_id`
/// and `chain_id` explicitly because those values are authority-sensitive and
/// must not be silently derived from txid, idempotency key, route labels, or UI
/// state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainWalletReceiptProjectionContext {
    /// QuickChain chain id context, for example `roc-dev`.
    pub chain_id: String,
    /// Backend-assigned durable operation identity.
    pub operation_id: String,
    /// Honest settlement status for this wallet-side projection.
    pub settlement_status: QuickChainWalletReceiptStatus,
}

impl QuickChainWalletReceiptProjectionContext {
    /// Build accepted hot-path projection context.
    pub fn accepted(
        chain_id: impl Into<String>,
        operation_id: impl Into<String>,
    ) -> WalletResult<Self> {
        let context = Self {
            chain_id: chain_id.into(),
            operation_id: operation_id.into(),
            settlement_status: QuickChainWalletReceiptStatus::Accepted,
        };
        context.validate()?;
        Ok(context)
    }

    /// Validate context shape without granting authority.
    pub fn validate(&self) -> WalletResult<()> {
        validate_visible_token(
            "chain_id",
            &self.chain_id,
            MAX_PREFLIGHT_CHAIN_ID_BYTES,
            |ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.'),
        )?;
        validate_visible_token(
            "operation_id",
            &self.operation_id,
            MAX_PREFLIGHT_OPERATION_ID_BYTES,
            |ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.' | '/'),
        )?;

        if !matches!(
            self.settlement_status,
            QuickChainWalletReceiptStatus::Accepted
        ) {
            return Err(WalletError::bad_request(
                "svc-wallet preflight projection may only label receipts as accepted",
            ));
        }

        Ok(())
    }
}

/// Inert wallet receipt projection for QuickChain Phase-0 review and tests.
///
/// This is not a chain receipt, not a consensus DTO, not a proof, and not a
/// root input commitment. It is a strict, typed bridge showing how the wallet
/// receipt surface maps to the future QuickChain receipt vocabulary while
/// preserving the rule that wallet/ledger remain the only economic authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainWalletReceiptProjection {
    /// Projection schema.
    pub schema: String,
    /// Explicit chain id context.
    pub chain_id: String,
    /// Explicit backend-assigned operation identity.
    pub operation_id: String,
    /// Wallet txid.
    pub txid: String,
    /// Wallet operation.
    pub op: WalletOp,
    /// Debit-side account where applicable.
    pub from: Option<String>,
    /// Credit-side account where applicable.
    pub to: Option<String>,
    /// Asset, currently `roc`.
    pub asset: String,
    /// Amount in integer minor units.
    pub amount_minor: AmountMinor,
    /// Debit-side nonce where applicable.
    pub nonce: Option<u64>,
    /// Wallet idempotency key echoed as retry identity.
    pub idempotency_key: String,
    /// Wallet/ledger produced timestamp in milliseconds.
    pub produced_at_ms: u64,
    /// First primitive ledger sequence assigned by the current ledger adapter.
    pub ledger_seq_start: u64,
    /// Last primitive ledger sequence assigned by the current ledger adapter.
    pub ledger_seq_end: u64,
    /// Legacy ron-ledger accumulator root copied as opaque legacy continuity.
    ///
    /// This is deliberately named `legacy_ledger_root` so it cannot be confused
    /// with a future QuickChain state root, receipt root, or checkpoint root.
    pub legacy_ledger_root: String,
    /// Backend-derived wallet receipt hash.
    pub receipt_hash: String,
    /// Honest wallet-side status.
    pub settlement_status: QuickChainWalletReceiptStatus,
}

impl QuickChainWalletReceiptProjection {
    /// Validate the projected DTO shape.
    pub fn validate(&self) -> WalletResult<()> {
        if self.schema != SVC_WALLET_QUICKCHAIN_RECEIPT_PROJECTION_SCHEMA {
            return Err(WalletError::bad_request(
                "invalid svc-wallet QuickChain receipt projection schema",
            ));
        }

        validate_visible_token(
            "chain_id",
            &self.chain_id,
            MAX_PREFLIGHT_CHAIN_ID_BYTES,
            |ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.'),
        )?;
        validate_visible_token(
            "operation_id",
            &self.operation_id,
            MAX_PREFLIGHT_OPERATION_ID_BYTES,
            |ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.' | '/'),
        )?;
        validate_visible_token("txid", &self.txid, MAX_PREFLIGHT_TXID_BYTES, |ch| {
            ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.')
        })?;

        if self.asset != DEFAULT_ASSET {
            return Err(WalletError::bad_request(
                "svc-wallet QuickChain projection currently supports only roc",
            ));
        }

        if let Some(account) = self.from.as_deref() {
            validate_account_id(account)?;
        }
        if let Some(account) = self.to.as_deref() {
            validate_account_id(account)?;
        }

        validate_idempotency_key(&self.idempotency_key)?;

        if self.produced_at_ms == 0 {
            return Err(WalletError::bad_request(
                "produced_at_ms must be nonzero in receipt projection",
            ));
        }

        if self.ledger_seq_start == 0 || self.ledger_seq_end == 0 {
            return Err(WalletError::bad_request(
                "ledger sequence range must be present and nonzero",
            ));
        }

        if self.ledger_seq_end < self.ledger_seq_start {
            return Err(WalletError::bad_request(
                "ledger sequence end precedes start",
            ));
        }

        validate_lower_hex_64("legacy_ledger_root", &self.legacy_ledger_root)?;
        validate_b3_hash("receipt_hash", &self.receipt_hash)?;

        if !matches!(
            self.settlement_status,
            QuickChainWalletReceiptStatus::Accepted
        ) {
            return Err(WalletError::bad_request(
                "svc-wallet preflight projection may only label receipts as accepted",
            ));
        }

        Ok(())
    }
}

/// Project a backend-derived wallet receipt into the inert QuickChain preflight
/// inspection shape.
pub fn project_wallet_receipt_for_quickchain_preflight(
    receipt: &Receipt,
    context: &QuickChainWalletReceiptProjectionContext,
) -> WalletResult<QuickChainWalletReceiptProjection> {
    context.validate()?;

    let ledger_seq_start = receipt.ledger_seq_start.ok_or_else(|| {
        WalletError::bad_request("receipt is missing ledger_seq_start for preflight projection")
    })?;
    let ledger_seq_end = receipt.ledger_seq_end.ok_or_else(|| {
        WalletError::bad_request("receipt is missing ledger_seq_end for preflight projection")
    })?;

    let projection = QuickChainWalletReceiptProjection {
        schema: SVC_WALLET_QUICKCHAIN_RECEIPT_PROJECTION_SCHEMA.to_string(),
        chain_id: context.chain_id.clone(),
        operation_id: context.operation_id.clone(),
        txid: receipt.txid.clone(),
        op: receipt.op,
        from: receipt.from.clone(),
        to: receipt.to.clone(),
        asset: receipt.asset.clone(),
        amount_minor: receipt.amount_minor,
        nonce: receipt.nonce,
        idempotency_key: receipt.idem.clone(),
        produced_at_ms: receipt.ts,
        ledger_seq_start,
        ledger_seq_end,
        legacy_ledger_root: receipt.ledger_root.clone(),
        receipt_hash: receipt.receipt_hash.clone(),
        settlement_status: context.settlement_status,
    };

    projection.validate()?;
    Ok(projection)
}

fn validate_visible_token(
    field: &str,
    value: &str,
    max_len: usize,
    allowed: impl Fn(char) -> bool,
) -> WalletResult<()> {
    if value.is_empty() || value.len() > max_len {
        return Err(WalletError::bad_request(format!(
            "{field} must be 1..={max_len} bytes"
        )));
    }

    if !value.chars().all(allowed) {
        return Err(WalletError::bad_request(format!(
            "{field} contains unsupported characters"
        )));
    }

    Ok(())
}

fn validate_b3_hash(field: &str, value: &str) -> WalletResult<()> {
    let Some(hex) = value.strip_prefix("b3:") else {
        return Err(WalletError::bad_request(format!(
            "{field} must be b3:<64 lowercase hex>"
        )));
    };

    validate_lower_hex_64(field, hex)
}

fn validate_lower_hex_64(field: &str, value: &str) -> WalletResult<()> {
    if value.len() != 64 || !value.chars().all(|ch| matches!(ch, '0'..='9' | 'a'..='f')) {
        return Err(WalletError::bad_request(format!(
            "{field} must be 64 lowercase hex characters"
        )));
    }

    Ok(())
}
