//! RO:WHAT — QuickChain event-class DTOs for accounting/reward safety.
//! RO:WHY — ECON/GOV: fakeable engagement must not silently become ROC balance truth.
//! RO:INTERACTS — ron-accounting snapshots, svc-rewarder payout planning, svc-wallet receipts.
//! RO:INVARIANTS — strict enum; unknown classes reject; accounting events do not mutate balances.
//! RO:METRICS — none.
//! RO:CONFIG — none; downstream policy decides allowed use.
//! RO:SECURITY — raw engagement classes cannot masquerade as economic receipts.
//! RO:TEST — tests/quickchain_event_class.rs.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::id::ContentId;

use super::{
    ids::validate_idempotency_key_v1, money::validate_quickchain_minor_units,
    validate_bounded_nonempty, validate_ref, validate_schema, validate_token, validate_version,
    QuickChainResult, QuickChainValidationError,
};

pub const QUICKCHAIN_USAGE_EVENT_SCHEMA: &str = "quickchain.usage-event.v1";
pub const MAX_QUICKCHAIN_ACTION_BYTES: usize = 96;
pub const MAX_QUICKCHAIN_LABELS: usize = 32;
pub const MAX_QUICKCHAIN_LABEL_KEY_BYTES: usize = 64;
pub const MAX_QUICKCHAIN_LABEL_VALUE_BYTES: usize = 128;

/// Strict event classes from QC-0A event/reward safety rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainEventClassV1 {
    /// Payer-authorized wallet event or ledger mutation. Balance effects still
    /// happen only through svc-wallet/ron-ledger, never through this DTO.
    EconomicReceipt,

    /// Usage measurement. Cannot mutate balances directly.
    Metering,

    /// Candidate proof signal that requires verification/challenge before reward planning.
    ProofEligible,

    /// Explicit advertiser/sponsor budget event; not protocol minting.
    AdBudgeted,

    /// Display/reporting signal. Never enters protocol ROC reward allocation.
    AnalyticsOnly,
}

impl QuickChainEventClassV1 {
    pub fn may_represent_balance_truth(self) -> bool {
        matches!(self, Self::EconomicReceipt)
    }

    pub fn may_enter_reward_manifest_without_extra_proof(self) -> bool {
        matches!(self, Self::EconomicReceipt | Self::AdBudgeted)
    }
}

/// Usage/accounting event DTO for QC-0B preflight tests.
///
/// This is not balance truth. It is a strict wire shape that future accounting
/// windows may consume after policy and vector gates are defined.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainUsageEventV1 {
    pub schema: String,
    pub version: u16,
    pub event_id: String,
    pub action: String,
    pub event_class: QuickChainEventClassV1,
    #[serde(default)]
    pub account_id: Option<String>,
    #[serde(default)]
    pub counterparty_account_id: Option<String>,
    #[serde(default)]
    pub object_cid: Option<ContentId>,
    #[serde(default)]
    pub site_name: Option<String>,
    #[serde(default)]
    pub amount_minor: Option<String>,
    pub units: u64,
    #[serde(default)]
    pub labels: BTreeMap<String, String>,
    pub produced_at_ms: u64,
    pub idempotency_key: String,
}

impl QuickChainUsageEventV1 {
    /// Validate DTO shape only. This does not record usage or mutate balances.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainUsageEventV1.schema",
            &self.schema,
            QUICKCHAIN_USAGE_EVENT_SCHEMA,
        )?;
        validate_version("QuickChainUsageEventV1.version", self.version)?;
        validate_ref("event_id", &self.event_id)?;
        validate_token("action", &self.action, MAX_QUICKCHAIN_ACTION_BYTES)?;
        validate_idempotency_key_v1("idempotency_key", &self.idempotency_key)?;

        if let Some(account_id) = &self.account_id {
            validate_ref("account_id", account_id)?;
        }
        if let Some(counterparty_account_id) = &self.counterparty_account_id {
            validate_ref("counterparty_account_id", counterparty_account_id)?;
        }
        if let Some(site_name) = &self.site_name {
            validate_ref("site_name", site_name)?;
        }
        if let Some(amount_minor) = &self.amount_minor {
            validate_quickchain_minor_units("amount_minor", amount_minor)?;
        }
        if self.produced_at_ms == 0 {
            return Err(QuickChainValidationError::InvalidField {
                field: "produced_at_ms",
                reason: "must be greater than zero",
            });
        }

        validate_labels(&self.labels)
    }
}

fn validate_labels(labels: &BTreeMap<String, String>) -> QuickChainResult<()> {
    if labels.len() > MAX_QUICKCHAIN_LABELS {
        return Err(QuickChainValidationError::TooManyItems {
            field: "labels",
            max: MAX_QUICKCHAIN_LABELS,
            actual: labels.len(),
        });
    }

    for (key, value) in labels {
        validate_token("labels.key", key, MAX_QUICKCHAIN_LABEL_KEY_BYTES)?;
        validate_bounded_nonempty("labels.value", value, MAX_QUICKCHAIN_LABEL_VALUE_BYTES)?;
    }

    Ok(())
}
