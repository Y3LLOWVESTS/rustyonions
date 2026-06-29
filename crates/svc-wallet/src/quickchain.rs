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

/// Schema label for the svc-wallet controlled internal bond enforcement confirmation artifact.
///
/// This is a wallet-side confirmation artifact for Phase 4 Round 3. It is not
/// a public staking route, not a wallet receipt, not a balance lock, not a
/// finality claim, not bridge behavior, and not external settlement. Any actual
/// economic state transition must go through the backend wallet/ledger path.
pub const SVC_WALLET_QUICKCHAIN_BOND_ENFORCEMENT_CONFIRMATION_SCHEMA: &str =
    "svc-wallet.quickchain-bond-enforcement-confirmation.v1";

/// Controlled internal-only enforcement action acknowledged at the wallet boundary.
///
/// These actions mirror the safe internal ledger enforcement vocabulary, but
/// the wallet artifact itself grants no direct state authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainWalletBondEnforcementAction {
    /// Confirm moving available bonded ROC into evidence-reserved review state.
    ReserveSlash,
    /// Confirm returning evidence-reserved ROC back to available bonded state.
    ReleaseSlashReserve,
    /// Confirm capturing only ROC that is already evidence-reserved.
    CaptureSlashReserve,
}

/// Wallet-side controlled enforcement confirmation status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainWalletBondEnforcementConfirmationStatus {
    /// Explicitly confirmed for the internal backend path only.
    ConfirmedInternalOnly,
}

/// Wallet-side explicit confirmation artifact for controlled internal bond enforcement.
///
/// This type is intentionally strict. It gives future operator/user UX a typed
/// confirmation object while preserving the boundary that svc-wallet does not
/// invent receipts, bypass ledger truth, run a public staking market, or expose
/// external settlement behavior from this helper.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainWalletBondEnforcementConfirmation {
    /// Confirmation schema.
    pub schema: String,
    /// Explicit chain id context.
    pub chain_id: String,
    /// Backend-assigned enforcement identity.
    pub enforcement_id: String,
    /// Internal bond account identifier.
    pub bond_account_id: String,
    /// Actor wallet account that explicitly confirmed the action.
    pub actor_account_id: String,
    /// Operator/reviewer subject that performed the confirmation.
    pub operator_subject: String,
    /// Asset, currently `roc`.
    pub asset: String,
    /// Amount in integer minor units.
    pub amount_minor: AmountMinor,
    /// Wallet idempotency/retry key for this confirmation identity.
    pub idempotency_key: String,
    /// Controlled internal enforcement action.
    pub action: QuickChainWalletBondEnforcementAction,
    /// Confirmation status.
    pub status: QuickChainWalletBondEnforcementConfirmationStatus,
    /// Must be true so hidden enforcement cannot be represented.
    pub explicit_confirmation: bool,
    /// Must be true: backend wallet/ledger path remains required.
    pub backend_ledger_path_required: bool,
    /// Must be false: this helper itself is not a live wallet mutation.
    pub live_wallet_mutation: bool,
    /// Must be false: this helper itself does not create a wallet receipt.
    pub wallet_receipt_created: bool,
    /// Must be false: this helper itself does not lock or unlock balances.
    pub balance_side_effect: bool,
    /// Must be false: no automatic penalty execution.
    pub auto_penalty_enabled: bool,
    /// Must be false: no public staking market.
    pub public_market: bool,
    /// Must be false: no liquidity behavior.
    pub liquidity_enabled: bool,
    /// Must be false: no bridge or external settlement behavior.
    pub outside_settlement: bool,
}

impl QuickChainWalletBondEnforcementConfirmation {
    /// Build an explicit internal-only confirmation artifact.
    #[allow(clippy::too_many_arguments)]
    pub fn confirmed_internal_only(
        chain_id: impl Into<String>,
        enforcement_id: impl Into<String>,
        bond_account_id: impl Into<String>,
        actor_account_id: impl Into<String>,
        operator_subject: impl Into<String>,
        amount_minor: u128,
        idempotency_key: impl Into<String>,
        action: QuickChainWalletBondEnforcementAction,
    ) -> WalletResult<Self> {
        let confirmation = Self {
            schema: SVC_WALLET_QUICKCHAIN_BOND_ENFORCEMENT_CONFIRMATION_SCHEMA.to_string(),
            chain_id: chain_id.into(),
            enforcement_id: enforcement_id.into(),
            bond_account_id: bond_account_id.into(),
            actor_account_id: actor_account_id.into(),
            operator_subject: operator_subject.into(),
            asset: DEFAULT_ASSET.to_string(),
            amount_minor: AmountMinor::new(amount_minor)?,
            idempotency_key: idempotency_key.into(),
            action,
            status: QuickChainWalletBondEnforcementConfirmationStatus::ConfirmedInternalOnly,
            explicit_confirmation: true,
            backend_ledger_path_required: true,
            live_wallet_mutation: false,
            wallet_receipt_created: false,
            balance_side_effect: false,
            auto_penalty_enabled: false,
            public_market: false,
            liquidity_enabled: false,
            outside_settlement: false,
        };

        confirmation.validate()?;
        Ok(confirmation)
    }

    /// Validate confirmation shape without granting spend, receipt, or settlement authority.
    pub fn validate(&self) -> WalletResult<()> {
        if self.schema != SVC_WALLET_QUICKCHAIN_BOND_ENFORCEMENT_CONFIRMATION_SCHEMA {
            return Err(WalletError::bad_request(
                "invalid svc-wallet QuickChain bond enforcement confirmation schema",
            ));
        }

        validate_visible_token(
            "chain_id",
            &self.chain_id,
            MAX_PREFLIGHT_CHAIN_ID_BYTES,
            |ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.'),
        )?;
        validate_visible_token(
            "enforcement_id",
            &self.enforcement_id,
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
            "operator_subject",
            &self.operator_subject,
            MAX_PREFLIGHT_BOND_REF_BYTES,
            |ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.' | '@' | '/'),
        )?;

        if self.asset != DEFAULT_ASSET {
            return Err(WalletError::bad_request(
                "svc-wallet QuickChain bond enforcement confirmation currently supports only roc",
            ));
        }

        if self.amount_minor.get() == 0 {
            return Err(WalletError::bad_request(
                "amount_minor must be positive for bond enforcement confirmation",
            ));
        }

        validate_idempotency_key(&self.idempotency_key)?;

        if !matches!(
            self.status,
            QuickChainWalletBondEnforcementConfirmationStatus::ConfirmedInternalOnly
        ) {
            return Err(WalletError::bad_request(
                "svc-wallet bond enforcement confirmation may only be confirmed_internal_only",
            ));
        }

        if !self.explicit_confirmation {
            return Err(WalletError::bad_request(
                "bond enforcement confirmation must be explicit",
            ));
        }

        if !self.backend_ledger_path_required {
            return Err(WalletError::bad_request(
                "bond enforcement confirmation must require backend wallet/ledger path",
            ));
        }

        if self.live_wallet_mutation {
            return Err(WalletError::bad_request(
                "bond enforcement confirmation artifact must not be a live wallet mutation",
            ));
        }

        if self.wallet_receipt_created {
            return Err(WalletError::bad_request(
                "bond enforcement confirmation artifact must not create a wallet receipt",
            ));
        }

        if self.balance_side_effect {
            return Err(WalletError::bad_request(
                "bond enforcement confirmation artifact must not carry balance side effects",
            ));
        }

        if self.auto_penalty_enabled {
            return Err(WalletError::bad_request(
                "bond enforcement confirmation must not enable automatic penalties",
            ));
        }

        if self.public_market {
            return Err(WalletError::bad_request(
                "bond enforcement confirmation must not enable a public market",
            ));
        }

        if self.liquidity_enabled {
            return Err(WalletError::bad_request(
                "bond enforcement confirmation must not enable liquidity behavior",
            ));
        }

        if self.outside_settlement {
            return Err(WalletError::bad_request(
                "bond enforcement confirmation must not enable external settlement",
            ));
        }

        Ok(())
    }
}

/// Schema label for the svc-wallet Phase 5 Round 1 anchor evidence report.
///
/// This report is read-only metadata that binds an already backend-derived
/// wallet receipt projection to an anchor dry-run checkpoint commitment. It is
/// not a wallet receipt, not balance truth, not paid unlock authority, not a
/// bridge, and not outside settlement.
pub const SVC_WALLET_QUICKCHAIN_ANCHOR_EVIDENCE_SCHEMA: &str =
    "svc-wallet.quickchain-anchor-evidence.v1";

/// Maximum anchor evidence identifier bytes accepted by the wallet adapter.
pub const MAX_PREFLIGHT_ANCHOR_ID_BYTES: usize = 128;

/// Honest Phase 5 Round 1 anchor evidence status for the wallet boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainWalletAnchorEvidenceStatus {
    /// Anchor dry-run evidence only; no wallet authority is granted.
    DryRunEvidenceOnly,
}

/// Read-only wallet-side report relating a backend wallet receipt projection to
/// an anchor dry-run commitment.
///
/// This is an evidence/report artifact only. It cannot issue, transfer, burn,
/// hold, capture, release, mutate receipts, mutate balances, unlock paid
/// content, or treat any outside chain as ROC truth.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainWalletAnchorEvidenceReport {
    /// Report schema.
    pub schema: String,
    /// Explicit chain id context.
    pub chain_id: String,
    /// Dry-run anchor identifier supplied by backend tooling.
    pub anchor_id: String,
    /// Reviewed checkpoint commitment hash.
    pub checkpoint_hash: String,
    /// Wallet transaction id copied from backend-derived wallet receipt evidence.
    pub wallet_txid: String,
    /// Wallet receipt hash copied from backend-derived wallet receipt evidence.
    pub wallet_receipt_hash: String,
    /// Backend-assigned durable operation identity copied from the projection.
    pub operation_id: String,
    /// Caller-supplied production timestamp for this report.
    pub produced_at_ms: u64,
    /// Honest status for this report.
    pub status: QuickChainWalletAnchorEvidenceStatus,
    /// Must remain true: this artifact is evidence only.
    pub evidence_only: bool,
    /// Must remain false: this artifact does not mutate wallet state.
    pub wallet_side_effect: bool,
    /// Must remain false: this artifact does not create or rewrite receipts.
    pub receipt_side_effect: bool,
    /// Must remain false: this artifact does not mutate balances.
    pub balance_side_effect: bool,
    /// Must remain false: this artifact does not mutate holds.
    pub hold_side_effect: bool,
    /// Must remain false: this artifact cannot unlock paid content.
    pub paid_unlock_authority: bool,
    /// Must remain false: this artifact does not enable outside settlement.
    pub outside_settlement: bool,
    /// Must remain false: an outside chain is not ROC truth.
    pub outside_chain_truth: bool,
}

impl QuickChainWalletAnchorEvidenceReport {
    /// Build a read-only anchor evidence report from a wallet receipt projection.
    pub fn new_from_projection(
        projection: &QuickChainWalletReceiptProjection,
        anchor_id: impl Into<String>,
        checkpoint_hash: impl Into<String>,
        produced_at_ms: u64,
    ) -> WalletResult<Self> {
        projection.validate()?;

        let report = Self {
            schema: SVC_WALLET_QUICKCHAIN_ANCHOR_EVIDENCE_SCHEMA.to_string(),
            chain_id: projection.chain_id.clone(),
            anchor_id: anchor_id.into(),
            checkpoint_hash: checkpoint_hash.into(),
            wallet_txid: projection.txid.clone(),
            wallet_receipt_hash: projection.receipt_hash.clone(),
            operation_id: projection.operation_id.clone(),
            produced_at_ms,
            status: QuickChainWalletAnchorEvidenceStatus::DryRunEvidenceOnly,
            evidence_only: true,
            wallet_side_effect: false,
            receipt_side_effect: false,
            balance_side_effect: false,
            hold_side_effect: false,
            paid_unlock_authority: false,
            outside_settlement: false,
            outside_chain_truth: false,
        };

        report.validate()?;
        Ok(report)
    }

    /// Validate the report shape without granting authority.
    pub fn validate(&self) -> WalletResult<()> {
        if self.schema != SVC_WALLET_QUICKCHAIN_ANCHOR_EVIDENCE_SCHEMA {
            return Err(WalletError::bad_request(
                "invalid svc-wallet QuickChain anchor evidence schema",
            ));
        }

        validate_visible_token(
            "chain_id",
            &self.chain_id,
            MAX_PREFLIGHT_CHAIN_ID_BYTES,
            |ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.'),
        )?;
        validate_visible_token(
            "anchor_id",
            &self.anchor_id,
            MAX_PREFLIGHT_ANCHOR_ID_BYTES,
            |ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.' | '/'),
        )?;
        validate_visible_token(
            "wallet_txid",
            &self.wallet_txid,
            MAX_PREFLIGHT_TXID_BYTES,
            |ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.'),
        )?;
        validate_visible_token(
            "operation_id",
            &self.operation_id,
            MAX_PREFLIGHT_OPERATION_ID_BYTES,
            |ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.' | '/'),
        )?;

        validate_b3_hash("checkpoint_hash", &self.checkpoint_hash)?;
        validate_b3_hash("wallet_receipt_hash", &self.wallet_receipt_hash)?;

        if self.produced_at_ms == 0 {
            return Err(WalletError::bad_request(
                "anchor evidence produced_at_ms must be nonzero",
            ));
        }

        if self.status != QuickChainWalletAnchorEvidenceStatus::DryRunEvidenceOnly {
            return Err(WalletError::bad_request(
                "wallet anchor evidence may only be dry_run_evidence_only",
            ));
        }

        if !self.evidence_only {
            return Err(WalletError::bad_request(
                "wallet anchor evidence must remain evidence-only",
            ));
        }

        if self.wallet_side_effect {
            return Err(WalletError::bad_request(
                "wallet anchor evidence must not mutate wallet state",
            ));
        }

        if self.receipt_side_effect {
            return Err(WalletError::bad_request(
                "wallet anchor evidence must not create or rewrite receipts",
            ));
        }

        if self.balance_side_effect {
            return Err(WalletError::bad_request(
                "wallet anchor evidence must not mutate balances",
            ));
        }

        if self.hold_side_effect {
            return Err(WalletError::bad_request(
                "wallet anchor evidence must not mutate holds",
            ));
        }

        if self.paid_unlock_authority {
            return Err(WalletError::bad_request(
                "wallet anchor evidence must not unlock paid content",
            ));
        }

        if self.outside_settlement {
            return Err(WalletError::bad_request(
                "wallet anchor evidence must not enable outside settlement",
            ));
        }

        if self.outside_chain_truth {
            return Err(WalletError::bad_request(
                "wallet anchor evidence must not make an outside chain ROC truth",
            ));
        }

        Ok(())
    }
}

/// Schema label for the svc-wallet Phase 5 Round 2 DA fallback evidence report.
///
/// This report is read-only metadata that binds an already backend-derived
/// wallet receipt projection to reviewed DA/archive/challenge fallback context.
/// It is not a wallet receipt, not balance truth, not paid unlock authority, and
/// not outside-chain truth.
pub const SVC_WALLET_QUICKCHAIN_DA_FALLBACK_EVIDENCE_SCHEMA: &str =
    "svc-wallet.quickchain-da-fallback-evidence.v1";

/// Maximum DA fallback plan identifier bytes accepted by the wallet adapter.
pub const MAX_PREFLIGHT_FALLBACK_PLAN_ID_BYTES: usize = 128;
/// Maximum DA chunk identifier bytes accepted by the wallet adapter.
pub const MAX_PREFLIGHT_DA_CHUNK_ID_BYTES: usize = 128;

/// Honest Phase 5 Round 2 DA fallback evidence status for the wallet boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainWalletDaFallbackEvidenceStatus {
    /// Archive/challenge evidence only; no wallet authority is granted.
    ArchiveChallengeEvidenceOnly,
}

/// Read-only wallet-side report relating a backend wallet receipt projection to
/// DA/archive/challenge fallback evidence.
///
/// This is an evidence/report artifact only. It cannot issue, transfer, burn,
/// hold, capture, release, mutate receipts, mutate balances, unlock paid
/// content, unblock history deletion, or treat outside DA/carrier material as
/// ROC truth.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainWalletDaFallbackEvidenceReport {
    /// Report schema.
    pub schema: String,
    /// Explicit chain id context.
    pub chain_id: String,
    /// Reviewed DA fallback plan identifier.
    pub fallback_plan_id: String,
    /// Reviewed checkpoint commitment hash.
    pub checkpoint_hash: String,
    /// Reviewed data availability root.
    pub data_availability_root: String,
    /// Optional challenged chunk identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub challenged_chunk_id: Option<String>,
    /// Wallet transaction id copied from backend-derived wallet receipt evidence.
    pub wallet_txid: String,
    /// Wallet receipt hash copied from backend-derived wallet receipt evidence.
    pub wallet_receipt_hash: String,
    /// Backend-assigned durable operation identity copied from the projection.
    pub operation_id: String,
    /// Caller-supplied production timestamp for this report.
    pub produced_at_ms: u64,
    /// Honest status for this report.
    pub status: QuickChainWalletDaFallbackEvidenceStatus,
    /// Must remain true: this artifact is evidence only.
    pub evidence_only: bool,
    /// Must remain true: archive fallback was checked by caller-supplied evidence.
    pub archive_fallback_checked: bool,
    /// Must remain true: missing-data challenge handling was checked.
    pub missing_data_challenge_checked: bool,
    /// Must remain true: restore path was checked.
    pub restore_path_checked: bool,
    /// Must remain true: this report blocks deletion/unavailability shortcuts.
    pub pruning_blocked: bool,
    /// Must remain false: this artifact does not mutate wallet state.
    pub wallet_side_effect: bool,
    /// Must remain false: this artifact does not create or rewrite receipts.
    pub receipt_side_effect: bool,
    /// Must remain false: this artifact does not mutate balances.
    pub balance_side_effect: bool,
    /// Must remain false: this artifact does not mutate holds.
    pub hold_side_effect: bool,
    /// Must remain false: this artifact cannot unlock paid content.
    pub paid_unlock_authority: bool,
    /// Must remain false: this artifact cannot authorize deletion/unavailability shortcuts.
    pub pruning_authority: bool,
    /// Must remain false: outside DA material is not ROC wallet truth.
    pub outside_data_availability_truth: bool,
    /// Must remain false: this artifact does not enable outside settlement.
    pub outside_settlement: bool,
    /// Must remain false: an outside chain is not ROC truth.
    pub outside_chain_truth: bool,
}

impl QuickChainWalletDaFallbackEvidenceReport {
    /// Build a read-only DA fallback evidence report from a wallet receipt projection.
    pub fn new_from_projection(
        projection: &QuickChainWalletReceiptProjection,
        fallback_plan_id: impl Into<String>,
        checkpoint_hash: impl Into<String>,
        data_availability_root: impl Into<String>,
        challenged_chunk_id: Option<String>,
        produced_at_ms: u64,
    ) -> WalletResult<Self> {
        projection.validate()?;

        let report = Self {
            schema: SVC_WALLET_QUICKCHAIN_DA_FALLBACK_EVIDENCE_SCHEMA.to_string(),
            chain_id: projection.chain_id.clone(),
            fallback_plan_id: fallback_plan_id.into(),
            checkpoint_hash: checkpoint_hash.into(),
            data_availability_root: data_availability_root.into(),
            challenged_chunk_id,
            wallet_txid: projection.txid.clone(),
            wallet_receipt_hash: projection.receipt_hash.clone(),
            operation_id: projection.operation_id.clone(),
            produced_at_ms,
            status: QuickChainWalletDaFallbackEvidenceStatus::ArchiveChallengeEvidenceOnly,
            evidence_only: true,
            archive_fallback_checked: true,
            missing_data_challenge_checked: true,
            restore_path_checked: true,
            pruning_blocked: true,
            wallet_side_effect: false,
            receipt_side_effect: false,
            balance_side_effect: false,
            hold_side_effect: false,
            paid_unlock_authority: false,
            pruning_authority: false,
            outside_data_availability_truth: false,
            outside_settlement: false,
            outside_chain_truth: false,
        };

        report.validate()?;
        Ok(report)
    }

    /// Validate the report shape without granting authority.
    pub fn validate(&self) -> WalletResult<()> {
        if self.schema != SVC_WALLET_QUICKCHAIN_DA_FALLBACK_EVIDENCE_SCHEMA {
            return Err(WalletError::bad_request(
                "invalid svc-wallet QuickChain DA fallback evidence schema",
            ));
        }

        validate_visible_token(
            "chain_id",
            &self.chain_id,
            MAX_PREFLIGHT_CHAIN_ID_BYTES,
            |ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.'),
        )?;
        validate_visible_token(
            "fallback_plan_id",
            &self.fallback_plan_id,
            MAX_PREFLIGHT_FALLBACK_PLAN_ID_BYTES,
            |ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.' | '/'),
        )?;
        validate_visible_token(
            "wallet_txid",
            &self.wallet_txid,
            MAX_PREFLIGHT_TXID_BYTES,
            |ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.'),
        )?;
        validate_visible_token(
            "operation_id",
            &self.operation_id,
            MAX_PREFLIGHT_OPERATION_ID_BYTES,
            |ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.' | '/'),
        )?;

        if let Some(challenged_chunk_id) = self.challenged_chunk_id.as_deref() {
            validate_visible_token(
                "challenged_chunk_id",
                challenged_chunk_id,
                MAX_PREFLIGHT_DA_CHUNK_ID_BYTES,
                |ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.' | '/'),
            )?;
        }

        validate_b3_hash("checkpoint_hash", &self.checkpoint_hash)?;
        validate_b3_hash("data_availability_root", &self.data_availability_root)?;
        validate_b3_hash("wallet_receipt_hash", &self.wallet_receipt_hash)?;

        if self.produced_at_ms == 0 {
            return Err(WalletError::bad_request(
                "DA fallback evidence produced_at_ms must be nonzero",
            ));
        }

        if self.status != QuickChainWalletDaFallbackEvidenceStatus::ArchiveChallengeEvidenceOnly {
            return Err(WalletError::bad_request(
                "wallet DA fallback evidence may only be archive_challenge_evidence_only",
            ));
        }

        if !self.evidence_only {
            return Err(WalletError::bad_request(
                "wallet DA fallback report must remain evidence-only",
            ));
        }

        if !self.archive_fallback_checked {
            return Err(WalletError::bad_request(
                "wallet DA fallback report must check archive fallback",
            ));
        }

        if !self.missing_data_challenge_checked {
            return Err(WalletError::bad_request(
                "wallet DA fallback report must check missing-data challenge handling",
            ));
        }

        if !self.restore_path_checked {
            return Err(WalletError::bad_request(
                "wallet DA fallback report must check restore path",
            ));
        }

        if !self.pruning_blocked {
            return Err(WalletError::bad_request(
                "wallet DA fallback report must keep pruning blocked",
            ));
        }

        if self.wallet_side_effect {
            return Err(WalletError::bad_request(
                "wallet DA fallback report must not carry wallet side effects",
            ));
        }

        if self.receipt_side_effect {
            return Err(WalletError::bad_request(
                "wallet DA fallback report must not carry receipt side effects",
            ));
        }

        if self.balance_side_effect {
            return Err(WalletError::bad_request(
                "wallet DA fallback report must not carry balance side effects",
            ));
        }

        if self.hold_side_effect {
            return Err(WalletError::bad_request(
                "wallet DA fallback report must not carry hold side effects",
            ));
        }

        if self.paid_unlock_authority {
            return Err(WalletError::bad_request(
                "wallet DA fallback report must not carry paid unlock authority",
            ));
        }

        if self.pruning_authority {
            return Err(WalletError::bad_request(
                "wallet DA fallback report must not carry pruning authority",
            ));
        }

        if self.outside_data_availability_truth {
            return Err(WalletError::bad_request(
                "wallet DA fallback report must not treat outside DA as wallet truth",
            ));
        }

        if self.outside_settlement {
            return Err(WalletError::bad_request(
                "wallet DA fallback report must not enable outside settlement",
            ));
        }

        if self.outside_chain_truth {
            return Err(WalletError::bad_request(
                "wallet DA fallback report must not treat outside chain as ROC truth",
            ));
        }

        Ok(())
    }
}

/// Schema label for the svc-wallet Phase 5 Round 3 external posture evidence report.
///
/// This report is read-only metadata that binds an already backend-derived
/// wallet receipt projection to the selected anchor-only external posture. It is
/// not a wallet receipt, not balance truth, not paid unlock authority, not a
/// bridge, not outside settlement, and not a public market.
pub const SVC_WALLET_QUICKCHAIN_EXTERNAL_POSTURE_EVIDENCE_SCHEMA: &str =
    "svc-wallet.quickchain-external-posture-evidence.v1";

/// Maximum external posture identifier bytes accepted by the wallet adapter.
pub const MAX_PREFLIGHT_EXTERNAL_POSTURE_ID_BYTES: usize = 128;

/// Honest Phase 5 Round 3 external posture status for the wallet boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainWalletExternalPostureEvidenceStatus {
    /// Anchor-only evidence; no wallet authority is granted.
    AnchorOnlyEvidenceOnly,
}

/// Read-only wallet-side report relating a backend wallet receipt projection to
/// the selected external posture.
///
/// This is an evidence/report artifact only. It cannot issue, transfer, burn,
/// hold, capture, release, mutate receipts, mutate balances, unlock paid
/// content, enable public market behavior, or treat any outside system as ROC truth.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainWalletExternalPostureEvidenceReport {
    /// Report schema.
    pub schema: String,
    /// Explicit chain id context.
    pub chain_id: String,
    /// Reviewed external posture identifier.
    pub posture_id: String,
    /// Reviewed checkpoint commitment hash.
    pub checkpoint_hash: String,
    /// Wallet transaction id copied from backend-derived wallet receipt evidence.
    pub wallet_txid: String,
    /// Wallet receipt hash copied from backend-derived wallet receipt evidence.
    pub wallet_receipt_hash: String,
    /// Backend-assigned durable operation identity copied from the projection.
    pub operation_id: String,
    /// Caller-supplied production timestamp for this report.
    pub produced_at_ms: u64,
    /// Honest status for this report.
    pub status: QuickChainWalletExternalPostureEvidenceStatus,
    /// Must remain true: anchor-only was selected.
    pub anchor_only_selected: bool,
    /// Must remain true: this artifact is evidence only.
    pub evidence_only: bool,
    /// Must remain true: wallet/ledger truth remains canonical.
    pub wallet_ledger_truth_canonical: bool,
    /// Must remain false: this artifact does not mutate wallet state.
    pub wallet_side_effect: bool,
    /// Must remain false: this artifact does not create or rewrite receipts.
    pub receipt_side_effect: bool,
    /// Must remain false: this artifact does not mutate balances.
    pub balance_side_effect: bool,
    /// Must remain false: this artifact does not mutate holds.
    pub hold_side_effect: bool,
    /// Must remain false: this artifact cannot unlock paid content.
    pub paid_unlock_authority: bool,
    /// Must remain false: this artifact does not enable outside settlement.
    pub outside_settlement: bool,
    /// Must remain false: this artifact does not create bridge authority.
    pub bridge_authority: bool,
    /// Must remain false: this artifact does not enable outside program authority.
    pub outside_program_authority: bool,
    /// Must remain false: this artifact does not enable listing authority.
    pub listing_authority: bool,
    /// Must remain false: this artifact does not create a public market.
    pub public_market: bool,
    /// Must remain false: this artifact does not create liquidity behavior.
    pub liquidity_enabled: bool,
    /// Must remain false: this artifact does not create bonded-economy authority.
    pub bonded_economy_authority: bool,
}

impl QuickChainWalletExternalPostureEvidenceReport {
    /// Build a read-only external posture evidence report from a wallet receipt projection.
    pub fn new_from_projection(
        projection: &QuickChainWalletReceiptProjection,
        posture_id: impl Into<String>,
        checkpoint_hash: impl Into<String>,
        produced_at_ms: u64,
    ) -> WalletResult<Self> {
        projection.validate()?;

        let report = Self {
            schema: SVC_WALLET_QUICKCHAIN_EXTERNAL_POSTURE_EVIDENCE_SCHEMA.to_string(),
            chain_id: projection.chain_id.clone(),
            posture_id: posture_id.into(),
            checkpoint_hash: checkpoint_hash.into(),
            wallet_txid: projection.txid.clone(),
            wallet_receipt_hash: projection.receipt_hash.clone(),
            operation_id: projection.operation_id.clone(),
            produced_at_ms,
            status: QuickChainWalletExternalPostureEvidenceStatus::AnchorOnlyEvidenceOnly,
            anchor_only_selected: true,
            evidence_only: true,
            wallet_ledger_truth_canonical: true,
            wallet_side_effect: false,
            receipt_side_effect: false,
            balance_side_effect: false,
            hold_side_effect: false,
            paid_unlock_authority: false,
            outside_settlement: false,
            bridge_authority: false,
            outside_program_authority: false,
            listing_authority: false,
            public_market: false,
            liquidity_enabled: false,
            bonded_economy_authority: false,
        };

        report.validate()?;
        Ok(report)
    }

    /// Validate the report shape without granting wallet authority.
    pub fn validate(&self) -> WalletResult<()> {
        if self.schema != SVC_WALLET_QUICKCHAIN_EXTERNAL_POSTURE_EVIDENCE_SCHEMA {
            return Err(WalletError::bad_request(
                "invalid svc-wallet QuickChain external posture evidence schema",
            ));
        }

        validate_visible_token(
            "chain_id",
            &self.chain_id,
            MAX_PREFLIGHT_CHAIN_ID_BYTES,
            |ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.'),
        )?;
        validate_visible_token(
            "posture_id",
            &self.posture_id,
            MAX_PREFLIGHT_EXTERNAL_POSTURE_ID_BYTES,
            |ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.' | '/'),
        )?;
        validate_visible_token(
            "wallet_txid",
            &self.wallet_txid,
            MAX_PREFLIGHT_TXID_BYTES,
            |ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.'),
        )?;
        validate_visible_token(
            "operation_id",
            &self.operation_id,
            MAX_PREFLIGHT_OPERATION_ID_BYTES,
            |ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.' | '/'),
        )?;

        validate_b3_hash("checkpoint_hash", &self.checkpoint_hash)?;
        validate_b3_hash("wallet_receipt_hash", &self.wallet_receipt_hash)?;

        if self.produced_at_ms == 0 {
            return Err(WalletError::bad_request(
                "external posture report produced_at_ms must be nonzero",
            ));
        }

        if self.status != QuickChainWalletExternalPostureEvidenceStatus::AnchorOnlyEvidenceOnly {
            return Err(WalletError::bad_request(
                "external posture report must remain anchor-only evidence",
            ));
        }

        if !self.anchor_only_selected {
            return Err(WalletError::bad_request(
                "external posture report must select anchor-only posture",
            ));
        }

        if !self.evidence_only {
            return Err(WalletError::bad_request(
                "external posture report must remain evidence-only",
            ));
        }

        if !self.wallet_ledger_truth_canonical {
            return Err(WalletError::bad_request(
                "external posture report must keep wallet/ledger truth canonical",
            ));
        }

        if self.wallet_side_effect {
            return Err(WalletError::bad_request(
                "external posture report must not mutate wallet state",
            ));
        }

        if self.receipt_side_effect {
            return Err(WalletError::bad_request(
                "external posture report must not create or rewrite receipts",
            ));
        }

        if self.balance_side_effect {
            return Err(WalletError::bad_request(
                "external posture report must not mutate balances",
            ));
        }

        if self.hold_side_effect {
            return Err(WalletError::bad_request(
                "external posture report must not mutate holds",
            ));
        }

        if self.paid_unlock_authority {
            return Err(WalletError::bad_request(
                "external posture report must not unlock paid content",
            ));
        }

        if self.outside_settlement {
            return Err(WalletError::bad_request(
                "external posture report must not enable outside settlement",
            ));
        }

        if self.bridge_authority {
            return Err(WalletError::bad_request(
                "external posture report must not create bridge authority",
            ));
        }

        if self.outside_program_authority {
            return Err(WalletError::bad_request(
                "external posture report must not create outside program authority",
            ));
        }

        if self.listing_authority {
            return Err(WalletError::bad_request(
                "external posture report must not create listing authority",
            ));
        }

        if self.public_market {
            return Err(WalletError::bad_request(
                "external posture report must not enable a public market",
            ));
        }

        if self.liquidity_enabled {
            return Err(WalletError::bad_request(
                "external posture report must not enable liquidity behavior",
            ));
        }

        if self.bonded_economy_authority {
            return Err(WalletError::bad_request(
                "external posture report must not create bonded-economy authority",
            ));
        }

        Ok(())
    }
}
