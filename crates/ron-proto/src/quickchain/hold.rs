//! RO:WHAT — QuickChain hold-state DTO shape for concurrent-hold preflight.
//! RO:WHY — ECON/RES: holds need explicit lifecycle identity before future replay/root work.
//! RO:INTERACTS — operation-intent DTOs, future wallet hold/capture/release receipts.
//! RO:INVARIANTS — DTO-only; no hold execution; no expiry worker; no state root; no ledger mutation.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — closed holds remain provable by receipt references; this module creates no proof.
//! RO:TEST — tests/quickchain_operation_hold.rs.

use serde::{Deserialize, Serialize};

use super::{
    ids::{validate_hold_id_v1, validate_operation_id_v1},
    money::validate_quickchain_minor_units,
    validate_chain_id, validate_ref, validate_schema, validate_version, QuickChainResult,
    QuickChainValidationError,
};

pub const QUICKCHAIN_HOLD_STATE_SCHEMA: &str = "quickchain.hold-state.v1";

/// Future hold lifecycle status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainHoldStatusV1 {
    Open,
    Captured,
    Released,
    Expired,
}

/// Hold-state DTO for QC-0B shape tests.
///
/// This is not the active hold table. It is strict data for future canonical
/// byte vectors after the hold lifecycle is fully locked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainHoldStateV1 {
    pub schema: String,
    pub version: u16,
    pub chain_id: String,
    pub hold_id: String,
    pub account_id: String,
    #[serde(default)]
    pub counterparty_account_id: Option<String>,
    pub amount_minor: String,
    pub status: QuickChainHoldStatusV1,
    pub opened_operation_id: String,
    #[serde(default)]
    pub terminal_operation_id: Option<String>,
    pub opened_at_ms: u64,
    pub expires_at_ms: u64,
    #[serde(default)]
    pub terminal_at_ms: Option<u64>,
    pub account_sequence_opened: u64,
    #[serde(default)]
    pub account_sequence_terminal: Option<u64>,
}

impl QuickChainHoldStateV1 {
    /// Validate DTO shape only. This does not open/capture/release a hold.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainHoldStateV1.schema",
            &self.schema,
            QUICKCHAIN_HOLD_STATE_SCHEMA,
        )?;
        validate_version("QuickChainHoldStateV1.version", self.version)?;
        validate_chain_id(&self.chain_id)?;
        validate_hold_id_v1("hold_id", &self.hold_id)?;
        validate_ref("account_id", &self.account_id)?;
        if let Some(counterparty_account_id) = &self.counterparty_account_id {
            validate_ref("counterparty_account_id", counterparty_account_id)?;
        }
        validate_quickchain_minor_units("amount_minor", &self.amount_minor)?;
        validate_operation_id_v1("opened_operation_id", &self.opened_operation_id)?;
        if let Some(terminal_operation_id) = &self.terminal_operation_id {
            validate_operation_id_v1("terminal_operation_id", terminal_operation_id)?;
        }
        validate_hold_timestamps(self.opened_at_ms, self.expires_at_ms, self.terminal_at_ms)?;
        self.validate_lifecycle_fields()
    }

    fn validate_lifecycle_fields(&self) -> QuickChainResult<()> {
        match self.status {
            QuickChainHoldStatusV1::Open => {
                if self.terminal_operation_id.is_some()
                    || self.terminal_at_ms.is_some()
                    || self.account_sequence_terminal.is_some()
                {
                    return Err(QuickChainValidationError::InvalidField {
                        field: "status",
                        reason: "open holds must not include terminal lifecycle fields",
                    });
                }
                Ok(())
            }
            QuickChainHoldStatusV1::Captured
            | QuickChainHoldStatusV1::Released
            | QuickChainHoldStatusV1::Expired => {
                if self.terminal_operation_id.is_none()
                    || self.terminal_at_ms.is_none()
                    || self.account_sequence_terminal.is_none()
                {
                    return Err(QuickChainValidationError::InvalidField {
                        field: "status",
                        reason: "terminal holds require terminal lifecycle fields",
                    });
                }
                Ok(())
            }
        }
    }
}

fn validate_hold_timestamps(
    opened_at_ms: u64,
    expires_at_ms: u64,
    terminal_at_ms: Option<u64>,
) -> QuickChainResult<()> {
    if opened_at_ms == 0 || expires_at_ms == 0 || expires_at_ms < opened_at_ms {
        return Err(QuickChainValidationError::InvalidTimestampOrder {
            field: "hold_time_range",
        });
    }

    if let Some(terminal_at_ms) = terminal_at_ms {
        if terminal_at_ms < opened_at_ms {
            return Err(QuickChainValidationError::InvalidTimestampOrder {
                field: "hold_time_range",
            });
        }
    }

    Ok(())
}
