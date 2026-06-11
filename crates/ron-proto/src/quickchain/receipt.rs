//! RO:WHAT — QuickChain receipt DTO shape for backend-derived ROC receipt references.
//! RO:WHY — ECON/RES: receipt status and operation shape must be explicit before future epoch vectors.
//! RO:INTERACTS — future svc-wallet receipts, ron-ledger truth, operation ids, hold ids, vector DTOs.
//! RO:INVARIANTS — DTO-only; exact operation-field matrix; no receipt issuance, roots, settlement, or mutation.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — a parsed receipt DTO is not authority; only svc-wallet/ron-ledger can create truth.
//! RO:TEST — tests/quickchain_receipt_dto.rs.

use serde::{Deserialize, Serialize};

use crate::id::ContentId;

use super::{
    ids::{validate_hold_id_v1, validate_idempotency_key_v1, validate_operation_id_v1},
    money::validate_quickchain_minor_units,
    operation::QuickChainOperationClassV1,
    validate_chain_id, validate_ref, validate_schema, validate_token, validate_version,
    QuickChainResult, QuickChainValidationError,
};

pub const QUICKCHAIN_RECEIPT_SCHEMA: &str = "quickchain.receipt.v1";
pub const QUICKCHAIN_RECEIPT_ASSET_ROC: &str = "roc";

pub const MAX_QUICKCHAIN_RECEIPT_OP_BYTES: usize = 96;
pub const MAX_QUICKCHAIN_RECEIPT_MEMO_BYTES: usize = 512;
pub const MAX_QUICKCHAIN_SESSION_BUDGET_ID_BYTES: usize = 128;

/// Honest settlement/finality status for a backend-derived receipt.
///
/// These labels do not create settlement. They let clients and services label
/// receipt status without pretending an accepted hot-path receipt is already
/// finalized or externally anchored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainReceiptStatusV1 {
    /// svc-wallet/ron-ledger accepted the mutation. This may be enough for local paid unlock.
    Accepted,

    /// Future receipt root includes this receipt in an epoch.
    EpochIncluded,

    /// Future challenge/finality threshold has been reached.
    Finalized,

    /// Future optional external anchor proves checkpoint commitment existed externally.
    Anchored,
}

impl QuickChainReceiptStatusV1 {
    #[must_use]
    pub fn is_backend_accepted(self) -> bool {
        matches!(
            self,
            Self::Accepted | Self::EpochIncluded | Self::Finalized | Self::Anchored
        )
    }

    #[must_use]
    pub fn is_epoch_included_or_stronger(self) -> bool {
        matches!(self, Self::EpochIncluded | Self::Finalized | Self::Anchored)
    }

    #[must_use]
    pub fn is_finalized_or_stronger(self) -> bool {
        matches!(self, Self::Finalized | Self::Anchored)
    }
}

/// Strict receipt DTO for future QuickChain receipt vectors.
///
/// This DTO is inert data. It does not issue receipts, verify signatures,
/// compute receipt hashes, compute roots, unlock paid content, or mutate ledger
/// truth. Receipt authority remains with svc-wallet/ron-ledger.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainReceiptV1 {
    pub schema: String,
    pub version: u16,
    pub chain_id: String,

    /// Backend wallet/ledger transaction or receipt id.
    pub txid: String,

    /// Durable backend-assigned operation identity.
    pub operation_id: String,

    /// Human/action token such as `paid_site_visit` or `hold_capture`.
    pub op: String,

    /// Coarse operation class used for validation and future replay planning.
    pub op_class: QuickChainOperationClassV1,

    pub status: QuickChainReceiptStatusV1,

    #[serde(default)]
    pub from_account_id: Option<String>,

    #[serde(default)]
    pub to_account_id: Option<String>,

    /// Must remain `"roc"` during the internal ROC phase.
    pub asset: String,

    /// Integer minor-unit string. No floats.
    pub amount_minor: String,

    #[serde(default)]
    pub account_sequence: Option<u64>,

    #[serde(default)]
    pub hold_id: Option<String>,

    #[serde(default)]
    pub session_budget_id: Option<String>,

    pub idempotency_key: String,

    /// Optional future operation hash reference. This module does not compute it.
    #[serde(default)]
    pub operation_hash: Option<ContentId>,

    /// Optional future receipt hash reference. This module does not compute it.
    #[serde(default)]
    pub receipt_hash: Option<ContentId>,

    #[serde(default)]
    pub receipt_root: Option<ContentId>,

    #[serde(default)]
    pub checkpoint_hash: Option<ContentId>,

    #[serde(default)]
    pub ledger_seq_start: Option<u64>,

    #[serde(default)]
    pub ledger_seq_end: Option<u64>,

    #[serde(default)]
    pub previous_ledger_root: Option<ContentId>,

    #[serde(default)]
    pub new_ledger_root: Option<ContentId>,

    #[serde(default)]
    pub memo: Option<String>,

    pub produced_at_ms: u64,
}

impl QuickChainReceiptV1 {
    /// Validate DTO shape only. This does not verify economic truth.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainReceiptV1.schema",
            &self.schema,
            QUICKCHAIN_RECEIPT_SCHEMA,
        )?;
        validate_version("QuickChainReceiptV1.version", self.version)?;
        validate_chain_id(&self.chain_id)?;
        validate_ref("txid", &self.txid)?;
        validate_operation_id_v1("operation_id", &self.operation_id)?;
        validate_token("op", &self.op, MAX_QUICKCHAIN_RECEIPT_OP_BYTES)?;
        validate_asset(&self.asset)?;
        validate_quickchain_minor_units("amount_minor", &self.amount_minor)?;
        validate_idempotency_key_v1("idempotency_key", &self.idempotency_key)?;

        if let Some(from_account_id) = &self.from_account_id {
            validate_ref("from_account_id", from_account_id)?;
        }

        if let Some(to_account_id) = &self.to_account_id {
            validate_ref("to_account_id", to_account_id)?;
        }

        if let Some(hold_id) = &self.hold_id {
            validate_hold_id_v1("hold_id", hold_id)?;
        }

        if let Some(session_budget_id) = &self.session_budget_id {
            validate_token(
                "session_budget_id",
                session_budget_id,
                MAX_QUICKCHAIN_SESSION_BUDGET_ID_BYTES,
            )?;
        }

        if let Some(memo) = &self.memo {
            validate_receipt_memo(memo)?;
        }

        if let Some(account_sequence) = self.account_sequence {
            if account_sequence == 0 {
                return Err(QuickChainValidationError::InvalidField {
                    field: "account_sequence",
                    reason: "must be greater than zero when present",
                });
            }
        }

        if self.produced_at_ms == 0 {
            return Err(QuickChainValidationError::InvalidField {
                field: "produced_at_ms",
                reason: "must be greater than zero",
            });
        }

        self.validate_operation_shape()?;
        self.validate_ledger_range()?;
        self.validate_status_evidence()
    }

    /// Validate the Phase 0 operation-field matrix.
    ///
    /// The matrix prevents optional receipt fields from producing contradictory
    /// interpretations before canonical receipt bytes and hashes are frozen.
    ///
    /// Current Phase 0 shapes:
    ///
    /// - issue: destination only; no source or hold
    /// - transfer: source and destination; no hold
    /// - burn: source only; no destination or hold
    /// - hold_open: source and hold; destination is optional
    /// - hold_capture: source, destination, and hold
    /// - hold_release: source and hold; no destination
    /// - hold_expire: source and hold; no destination
    fn validate_operation_shape(&self) -> QuickChainResult<()> {
        match self.op_class {
            QuickChainOperationClassV1::Issue => {
                forbid_optional_field("from_account_id", self.from_account_id.is_some())?;
                require_account("to_account_id", self.to_account_id.as_deref())?;
                forbid_optional_field("hold_id", self.hold_id.is_some())
            }

            QuickChainOperationClassV1::Transfer => {
                require_account("from_account_id", self.from_account_id.as_deref())?;
                require_account("to_account_id", self.to_account_id.as_deref())?;
                forbid_optional_field("hold_id", self.hold_id.is_some())
            }

            QuickChainOperationClassV1::Burn => {
                require_account("from_account_id", self.from_account_id.as_deref())?;
                forbid_optional_field("to_account_id", self.to_account_id.is_some())?;
                forbid_optional_field("hold_id", self.hold_id.is_some())
            }

            QuickChainOperationClassV1::HoldOpen => {
                require_account("from_account_id", self.from_account_id.as_deref())?;
                require_hold(self.hold_id.as_deref())
            }

            QuickChainOperationClassV1::HoldCapture => {
                require_account("from_account_id", self.from_account_id.as_deref())?;
                require_account("to_account_id", self.to_account_id.as_deref())?;
                require_hold(self.hold_id.as_deref())
            }

            QuickChainOperationClassV1::HoldRelease | QuickChainOperationClassV1::HoldExpire => {
                require_account("from_account_id", self.from_account_id.as_deref())?;
                forbid_optional_field("to_account_id", self.to_account_id.is_some())?;
                require_hold(self.hold_id.as_deref())
            }
        }
    }

    fn validate_ledger_range(&self) -> QuickChainResult<()> {
        match (self.ledger_seq_start, self.ledger_seq_end) {
            (Some(start), Some(end)) => {
                if start == 0 || end == 0 || end < start {
                    return Err(QuickChainValidationError::InvalidField {
                        field: "ledger_seq_range",
                        reason: "ledger sequence range must be non-zero and ordered",
                    });
                }
            }
            (None, None) => {}
            _ => {
                return Err(QuickChainValidationError::InvalidField {
                    field: "ledger_seq_range",
                    reason: "ledger_seq_start and ledger_seq_end must be present together",
                });
            }
        }

        if self.previous_ledger_root.is_some() != self.new_ledger_root.is_some() {
            return Err(QuickChainValidationError::InvalidField {
                field: "ledger_roots",
                reason: "previous_ledger_root and new_ledger_root must be present together",
            });
        }

        Ok(())
    }

    fn validate_status_evidence(&self) -> QuickChainResult<()> {
        if !self.status.is_epoch_included_or_stronger() {
            return Ok(());
        }

        if self.receipt_hash.is_none() {
            return Err(QuickChainValidationError::InvalidField {
                field: "receipt_hash",
                reason: "required for epoch_included/finalized/anchored receipt status",
            });
        }

        if self.receipt_root.is_none() {
            return Err(QuickChainValidationError::InvalidField {
                field: "receipt_root",
                reason: "required for epoch_included/finalized/anchored receipt status",
            });
        }

        if self.checkpoint_hash.is_none() {
            return Err(QuickChainValidationError::InvalidField {
                field: "checkpoint_hash",
                reason: "required for epoch_included/finalized/anchored receipt status",
            });
        }

        if self.ledger_seq_start.is_none() || self.ledger_seq_end.is_none() {
            return Err(QuickChainValidationError::InvalidField {
                field: "ledger_seq_range",
                reason: "required for epoch_included/finalized/anchored receipt status",
            });
        }

        Ok(())
    }
}

fn validate_asset(asset: &str) -> QuickChainResult<()> {
    if asset == QUICKCHAIN_RECEIPT_ASSET_ROC {
        return Ok(());
    }

    Err(QuickChainValidationError::InvalidField {
        field: "asset",
        reason: "must be roc during the internal ROC phase",
    })
}

fn validate_receipt_memo(memo: &str) -> QuickChainResult<()> {
    if memo.trim().is_empty() {
        return Err(QuickChainValidationError::EmptyField { field: "memo" });
    }

    if memo.len() > MAX_QUICKCHAIN_RECEIPT_MEMO_BYTES {
        return Err(QuickChainValidationError::FieldTooLong {
            field: "memo",
            max: MAX_QUICKCHAIN_RECEIPT_MEMO_BYTES,
            actual: memo.len(),
        });
    }

    Ok(())
}

fn require_account(field: &'static str, value: Option<&str>) -> QuickChainResult<()> {
    if value.is_some() {
        return Ok(());
    }

    Err(QuickChainValidationError::InvalidField {
        field,
        reason: "required for this receipt operation class",
    })
}

fn require_hold(value: Option<&str>) -> QuickChainResult<()> {
    if value.is_some() {
        return Ok(());
    }

    Err(QuickChainValidationError::InvalidField {
        field: "hold_id",
        reason: "required for this receipt operation class",
    })
}

fn forbid_optional_field(field: &'static str, present: bool) -> QuickChainResult<()> {
    if !present {
        return Ok(());
    }

    Err(QuickChainValidationError::InvalidField {
        field,
        reason: "must be absent for this receipt operation class",
    })
}
