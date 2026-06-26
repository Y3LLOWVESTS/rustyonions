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

/// Schema label for the inert svc-wallet bond review artifact.
///
/// This is a wallet-side review/confirmation shape only. It is not a live
/// staking route, not a wallet mutation, not penalty enforcement, not a receipt,
/// not settlement, and not public validator economy authority.
pub const SVC_WALLET_QUICKCHAIN_BOND_REVIEW_SCHEMA: &str = "svc-wallet.quickchain-bond-review.v1";

/// Maximum preflight bond-review reference bytes.
pub const MAX_PREFLIGHT_BOND_REF_BYTES: usize = 128;

/// Explicit bond-review action requested for future Phase 4 operator UX.
///
/// These actions are review-only in Phase 4 Round 1. The wallet does not expose
/// a live route for them here and does not mutate balances from this helper.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainWalletBondAction {
    /// Review opening a new internal bond account.
    OpenBond,
    /// Review increasing an existing internal bond account.
    IncreaseBond,
    /// Review requesting a future unlock window.
    RequestUnlock,
    /// Review canceling a pending unlock request.
    CancelUnlockRequest,
}

/// Wallet-side bond review status.
///
/// Only review-only is allowed in this Phase 4 Round 1 helper. Anything stronger
/// belongs to a later explicitly authorized wallet route.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainWalletBondReviewStatus {
    /// Review artifact only; no live wallet mutation.
    ReviewOnly,
}

/// Inert wallet-side bond review artifact for Phase 4 Round 1.
///
/// This shape is intentionally strict so future UI/operator flows must show an
/// explicit review step before any later live bond route can exist. It carries
/// enough context for display/review, but it cannot create a receipt, lock ROC,
/// mutate balances, or authorize public market behavior.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainWalletBondReview {
    /// Review schema.
    pub schema: String,
    /// Explicit chain id context.
    pub chain_id: String,
    /// Explicit backend-assigned bond intent identity.
    pub intent_id: String,
    /// Internal bond account identifier.
    pub bond_account_id: String,
    /// Actor wallet account that would have to confirm later.
    pub actor_account_id: String,
    /// Human/operator/reviewer subject label.
    pub reviewer_subject: String,
    /// Asset, currently `roc`.
    pub asset: String,
    /// Amount in integer minor units.
    pub amount_minor: AmountMinor,
    /// Wallet idempotency/retry key for review identity.
    pub idempotency_key: String,
    /// Requested review action.
    pub action: QuickChainWalletBondAction,
    /// Review status.
    pub status: QuickChainWalletBondReviewStatus,
    /// Must be true so no hidden lock/spend can be represented.
    pub requires_explicit_confirmation: bool,
    /// Must be false in Phase 4 Round 1.
    pub live_wallet_mutation: bool,
    /// Must be false in Phase 4 Round 1.
    pub auto_penalty_enabled: bool,
    /// Must be false in Phase 4 Round 1.
    pub public_market: bool,
    /// Must be false in Phase 4 Round 1.
    pub liquidity_enabled: bool,
}

impl QuickChainWalletBondReview {
    /// Build a review-only bond artifact.
    #[allow(clippy::too_many_arguments)]
    pub fn review_only(
        chain_id: impl Into<String>,
        intent_id: impl Into<String>,
        bond_account_id: impl Into<String>,
        actor_account_id: impl Into<String>,
        reviewer_subject: impl Into<String>,
        amount_minor: u128,
        idempotency_key: impl Into<String>,
        action: QuickChainWalletBondAction,
    ) -> WalletResult<Self> {
        let review = Self {
            schema: SVC_WALLET_QUICKCHAIN_BOND_REVIEW_SCHEMA.to_string(),
            chain_id: chain_id.into(),
            intent_id: intent_id.into(),
            bond_account_id: bond_account_id.into(),
            actor_account_id: actor_account_id.into(),
            reviewer_subject: reviewer_subject.into(),
            asset: DEFAULT_ASSET.to_string(),
            amount_minor: AmountMinor::new(amount_minor)?,
            idempotency_key: idempotency_key.into(),
            action,
            status: QuickChainWalletBondReviewStatus::ReviewOnly,
            requires_explicit_confirmation: true,
            live_wallet_mutation: false,
            auto_penalty_enabled: false,
            public_market: false,
            liquidity_enabled: false,
        };

        review.validate()?;
        Ok(review)
    }

    /// Validate review shape without granting spend or settlement authority.
    pub fn validate(&self) -> WalletResult<()> {
        if self.schema != SVC_WALLET_QUICKCHAIN_BOND_REVIEW_SCHEMA {
            return Err(WalletError::bad_request(
                "invalid svc-wallet QuickChain bond review schema",
            ));
        }

        validate_visible_token(
            "chain_id",
            &self.chain_id,
            MAX_PREFLIGHT_CHAIN_ID_BYTES,
            |ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.'),
        )?;
        validate_visible_token(
            "intent_id",
            &self.intent_id,
            MAX_PREFLIGHT_BOND_REF_BYTES,
            |ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.' | '/'),
        )?;
        validate_visible_token(
            "bond_account_id",
            &self.bond_account_id,
            MAX_PREFLIGHT_BOND_REF_BYTES,
            |ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.' | '/'),
        )?;
        validate_account_id(&self.actor_account_id)?;
        validate_visible_token(
            "reviewer_subject",
            &self.reviewer_subject,
            MAX_PREFLIGHT_BOND_REF_BYTES,
            |ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.' | '@' | '/'),
        )?;

        if self.asset != DEFAULT_ASSET {
            return Err(WalletError::bad_request(
                "svc-wallet QuickChain bond review currently supports only roc",
            ));
        }

        if self.amount_minor.get() == 0 {
            return Err(WalletError::bad_request(
                "amount_minor must be positive for bond review",
            ));
        }

        validate_idempotency_key(&self.idempotency_key)?;

        if !matches!(self.status, QuickChainWalletBondReviewStatus::ReviewOnly) {
            return Err(WalletError::bad_request(
                "svc-wallet Phase 4 Round 1 bond review may only be review_only",
            ));
        }

        if !self.requires_explicit_confirmation {
            return Err(WalletError::bad_request(
                "bond review must require explicit confirmation",
            ));
        }

        if self.live_wallet_mutation {
            return Err(WalletError::bad_request(
                "bond review must not represent a live wallet mutation",
            ));
        }

        if self.auto_penalty_enabled {
            return Err(WalletError::bad_request(
                "bond review must not enable automatic economic penalties",
            ));
        }

        if self.public_market {
            return Err(WalletError::bad_request(
                "bond review must not enable a public market",
            ));
        }

        if self.liquidity_enabled {
            return Err(WalletError::bad_request(
                "bond review must not enable liquidity behavior",
            ));
        }

        Ok(())
    }
}

/// Schema label for the inert svc-wallet bond dispute review artifact.
///
/// This is a wallet-side review/acknowledgement shape only. It is not a live
/// penalty route, not a wallet mutation, not a balance lock, not a receipt, not
/// finality, not settlement, and not public validator economy authority.
pub const SVC_WALLET_QUICKCHAIN_BOND_DISPUTE_REVIEW_SCHEMA: &str =
    "svc-wallet.quickchain-bond-dispute-review.v1";

/// Explicit disputed-bond review action for Phase 4 Round 2.
///
/// These actions mirror disputed-bond simulation states from lower layers, but
/// remain review-only here. svc-wallet does not execute them as economic
/// mutations in this round.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainWalletBondDisputeAction {
    /// Review a simulated freeze pending an appeal window.
    FreezePendingAppeal,
    /// Review a simulated appeal submission.
    SubmitAppeal,
    /// Review a simulated no-penalty resolution.
    ResolveNoPenalty,
    /// Review rejection of irreversible penalty execution.
    RejectIrreversiblePenalty,
}

/// Wallet-side disputed-bond review status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainWalletBondDisputeReviewStatus {
    /// Review artifact only; no live wallet mutation.
    ReviewOnly,
}

/// Inert wallet-side disputed-bond review artifact for Phase 4 Round 2.
///
/// This shape lets the wallet display/review disputed-bond simulation state
/// without creating spend authority, balance locks, finality claims, receipts,
/// public market behavior, or irreversible enforcement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainWalletBondDisputeReview {
    /// Review schema.
    pub schema: String,
    /// Explicit chain id context.
    pub chain_id: String,
    /// Explicit disputed-bond identity.
    pub dispute_id: String,
    /// Internal bond account identifier.
    pub bond_account_id: String,
    /// Actor wallet account that would need explicit confirmation in any later live flow.
    pub actor_account_id: String,
    /// Human/operator/reviewer subject label.
    pub reviewer_subject: String,
    /// Asset, currently `roc`.
    pub asset: String,
    /// Disputed amount in integer minor units.
    pub disputed_amount_minor: AmountMinor,
    /// Simulated frozen amount in canonical integer minor units; may be "0".
    pub frozen_minor: String,
    /// Wallet idempotency/retry key for review identity.
    pub idempotency_key: String,
    /// Requested review action.
    pub action: QuickChainWalletBondDisputeAction,
    /// Review status.
    pub status: QuickChainWalletBondDisputeReviewStatus,
    /// Must be true so no hidden lock/spend can be represented.
    pub requires_explicit_confirmation: bool,
    /// Must be false in Phase 4 Round 2.
    pub live_wallet_mutation: bool,
    /// Must be false in Phase 4 Round 2.
    pub balance_side_effect: bool,
    /// Must be false in Phase 4 Round 2.
    pub auto_penalty_enabled: bool,
    /// Must be false in Phase 4 Round 2.
    pub finality_claim: bool,
}

impl QuickChainWalletBondDisputeReview {
    /// Build a review-only disputed-bond artifact.
    #[allow(clippy::too_many_arguments)]
    pub fn review_only(
        chain_id: impl Into<String>,
        dispute_id: impl Into<String>,
        bond_account_id: impl Into<String>,
        actor_account_id: impl Into<String>,
        reviewer_subject: impl Into<String>,
        disputed_amount_minor: u128,
        frozen_minor: u128,
        idempotency_key: impl Into<String>,
        action: QuickChainWalletBondDisputeAction,
    ) -> WalletResult<Self> {
        let review = Self {
            schema: SVC_WALLET_QUICKCHAIN_BOND_DISPUTE_REVIEW_SCHEMA.to_string(),
            chain_id: chain_id.into(),
            dispute_id: dispute_id.into(),
            bond_account_id: bond_account_id.into(),
            actor_account_id: actor_account_id.into(),
            reviewer_subject: reviewer_subject.into(),
            asset: DEFAULT_ASSET.to_string(),
            disputed_amount_minor: AmountMinor::new(disputed_amount_minor)?,
            frozen_minor: frozen_minor.to_string(),
            idempotency_key: idempotency_key.into(),
            action,
            status: QuickChainWalletBondDisputeReviewStatus::ReviewOnly,
            requires_explicit_confirmation: true,
            live_wallet_mutation: false,
            balance_side_effect: false,
            auto_penalty_enabled: false,
            finality_claim: false,
        };

        review.validate()?;
        Ok(review)
    }

    /// Validate review shape without granting spend, lock, settlement, or finality authority.
    pub fn validate(&self) -> WalletResult<()> {
        if self.schema != SVC_WALLET_QUICKCHAIN_BOND_DISPUTE_REVIEW_SCHEMA {
            return Err(WalletError::bad_request(
                "invalid svc-wallet QuickChain bond dispute review schema",
            ));
        }

        validate_visible_token(
            "chain_id",
            &self.chain_id,
            MAX_PREFLIGHT_CHAIN_ID_BYTES,
            |ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.'),
        )?;
        validate_visible_token(
            "dispute_id",
            &self.dispute_id,
            MAX_PREFLIGHT_BOND_REF_BYTES,
            |ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.' | '/'),
        )?;
        validate_visible_token(
            "bond_account_id",
            &self.bond_account_id,
            MAX_PREFLIGHT_BOND_REF_BYTES,
            |ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.' | '/'),
        )?;
        validate_account_id(&self.actor_account_id)?;
        validate_visible_token(
            "reviewer_subject",
            &self.reviewer_subject,
            MAX_PREFLIGHT_BOND_REF_BYTES,
            |ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.' | '@' | '/'),
        )?;

        if self.asset != DEFAULT_ASSET {
            return Err(WalletError::bad_request(
                "svc-wallet QuickChain bond dispute review currently supports only roc",
            ));
        }

        if self.disputed_amount_minor.get() == 0 {
            return Err(WalletError::bad_request(
                "disputed_amount_minor must be positive for disputed-bond review",
            ));
        }

        let frozen_minor = parse_canonical_minor_units("frozen_minor", &self.frozen_minor)?;
        if frozen_minor > self.disputed_amount_minor.get() {
            return Err(WalletError::bad_request(
                "frozen_minor must not exceed disputed_amount_minor",
            ));
        }

        validate_idempotency_key(&self.idempotency_key)?;

        if !matches!(
            self.status,
            QuickChainWalletBondDisputeReviewStatus::ReviewOnly
        ) {
            return Err(WalletError::bad_request(
                "svc-wallet Phase 4 Round 2 bond dispute review may only be review_only",
            ));
        }

        if !self.requires_explicit_confirmation {
            return Err(WalletError::bad_request(
                "bond dispute review must require explicit confirmation",
            ));
        }

        if self.live_wallet_mutation {
            return Err(WalletError::bad_request(
                "bond dispute review must not represent a live wallet mutation",
            ));
        }

        if self.balance_side_effect {
            return Err(WalletError::bad_request(
                "bond dispute review must not represent a wallet balance side effect",
            ));
        }

        if self.auto_penalty_enabled {
            return Err(WalletError::bad_request(
                "bond dispute review must not enable automatic economic penalties",
            ));
        }

        if self.finality_claim {
            return Err(WalletError::bad_request(
                "bond dispute review must not claim settlement finality",
            ));
        }

        match self.action {
            QuickChainWalletBondDisputeAction::FreezePendingAppeal
            | QuickChainWalletBondDisputeAction::SubmitAppeal => {
                if frozen_minor == 0 {
                    return Err(WalletError::bad_request(
                        "freeze and appeal review actions require nonzero frozen_minor",
                    ));
                }
            }
            QuickChainWalletBondDisputeAction::ResolveNoPenalty
            | QuickChainWalletBondDisputeAction::RejectIrreversiblePenalty => {
                if frozen_minor != 0 {
                    return Err(WalletError::bad_request(
                        "terminal review actions must not carry frozen_minor",
                    ));
                }
            }
        }

        Ok(())
    }
}

fn parse_canonical_minor_units(field: &str, value: &str) -> WalletResult<u128> {
    if value.is_empty() {
        return Err(WalletError::bad_request(format!(
            "{field} must not be empty"
        )));
    }

    if value.len() > 1 && value.starts_with('0') {
        return Err(WalletError::bad_request(format!(
            "{field} must be canonical integer minor units"
        )));
    }

    if !value.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(WalletError::bad_request(format!(
            "{field} must be integer minor units"
        )));
    }

    value.parse::<u128>().map_err(|err| {
        WalletError::bad_request(format!("{field} is not a u128 minor-unit value: {err}"))
    })
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
