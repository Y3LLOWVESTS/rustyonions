//! RO:WHAT — Pure projection of deterministic QuickChain snapshots into frozen ron-proto leaf payload DTOs.
//! RO:WHY — ECON/RES: ledger-derived state and externally supplied immutable projection context must agree before canonical bytes or roots exist.
//! RO:INTERACTS — state_snapshot.rs and ron_proto::quickchain active-hold leaf payload contracts.
//! RO:INVARIANTS — exact context set; explicit epoch binding; sorted output; no defaults, serialization, hashing, roots, clocks, IO, or mutation.
//! RO:METRICS — none.
//! RO:CONFIG — none; available only through the quickchain-preflight feature.
//! RO:SECURITY — purpose, policy identity, and epoch IDs are explicit inputs and grant no wallet, hold, or spend authority.
//! RO:TEST — tests/quickchain_active_hold_leaf_projection.rs.

use std::collections::{BTreeMap, BTreeSet};

use ron_proto::{
    quickchain::{
        QuickChainActiveHoldLeafPayloadV1, QuickChainActiveHoldStatusV1,
        QUICKCHAIN_ACTIVE_HOLD_LEAF_PAYLOAD_SCHEMA, QUICKCHAIN_DTO_VERSION,
    },
    ContentId,
};
use thiserror::Error;

use super::state_snapshot::QuickChainStateSnapshot;

/// Explicit binding between one ledger execution epoch number and one
/// canonical QuickChain epoch identifier.
///
/// The ledger currently executes against numeric epochs while the frozen
/// ron-proto leaf payload carries canonical string epoch IDs. This type keeps
/// that conversion explicit instead of silently calling `u64::to_string()`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickChainEpochBinding {
    epoch_number: u64,
    epoch_id: String,
}

impl QuickChainEpochBinding {
    /// Create an explicit numeric-to-canonical epoch binding.
    ///
    /// The canonical epoch identifier is validated by the final ron-proto
    /// payload contract during projection.
    #[must_use]
    pub fn new(epoch_number: u64, epoch_id: impl Into<String>) -> Self {
        Self {
            epoch_number,
            epoch_id: epoch_id.into(),
        }
    }

    /// Numeric epoch used by deterministic ledger execution.
    #[must_use]
    pub const fn epoch_number(&self) -> u64 {
        self.epoch_number
    }

    /// Canonical epoch identifier intended for the frozen leaf DTO.
    #[must_use]
    pub fn epoch_id(&self) -> &str {
        &self.epoch_id
    }
}

/// Immutable non-ledger context required to project one active hold leaf.
///
/// `ron-ledger` does not invent any value in this context. A future trusted
/// policy/operation boundary must supply the purpose, policy hash, and reviewed
/// epoch bindings that belong to the opening operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickChainActiveHoldLeafProjectionContext {
    hold_id: String,
    purpose: String,
    created_at_epoch: QuickChainEpochBinding,
    expires_at_epoch: QuickChainEpochBinding,
    policy_hash: ContentId,
}

impl QuickChainActiveHoldLeafProjectionContext {
    /// Create explicit immutable projection context for one active hold.
    #[must_use]
    pub fn new(
        hold_id: impl Into<String>,
        purpose: impl Into<String>,
        created_at_epoch: QuickChainEpochBinding,
        expires_at_epoch: QuickChainEpochBinding,
        policy_hash: ContentId,
    ) -> Self {
        Self {
            hold_id: hold_id.into(),
            purpose: purpose.into(),
            created_at_epoch,
            expires_at_epoch,
            policy_hash,
        }
    }

    /// Hold lifecycle identifier this context belongs to.
    #[must_use]
    pub fn hold_id(&self) -> &str {
        &self.hold_id
    }

    /// Immutable reviewed purpose committed by the future hold leaf.
    #[must_use]
    pub fn purpose(&self) -> &str {
        &self.purpose
    }

    /// Explicit binding for the hold creation epoch.
    #[must_use]
    pub const fn created_at_epoch(&self) -> &QuickChainEpochBinding {
        &self.created_at_epoch
    }

    /// Explicit binding for the first expiry-eligible epoch.
    #[must_use]
    pub const fn expires_at_epoch(&self) -> &QuickChainEpochBinding {
        &self.expires_at_epoch
    }

    /// Immutable reviewed policy content identifier.
    #[must_use]
    pub const fn policy_hash(&self) -> &ContentId {
        &self.policy_hash
    }
}

/// Deterministic failure while projecting snapshots into frozen leaf DTOs.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum QuickChainLeafProjectionError {
    /// More than one projection context was supplied for one hold lifecycle.
    #[error("duplicate active-hold leaf projection context for {hold_id}")]
    DuplicateActiveHoldContext {
        /// Duplicated hold lifecycle identifier.
        hold_id: String,
    },

    /// An active snapshot row had no corresponding explicit projection context.
    #[error("missing active-hold leaf projection context for {hold_id}")]
    MissingActiveHoldContext {
        /// Active hold lifecycle lacking projection context.
        hold_id: String,
    },

    /// Projection context was supplied for a hold absent from active state.
    #[error("projection context targets a non-active hold: {hold_id}")]
    UnknownActiveHoldContext {
        /// Hold lifecycle not present in the active snapshot.
        hold_id: String,
    },

    /// Active state existed without the chain binding required by the leaf DTO.
    #[error("active-hold leaf projection requires a bound chain_id")]
    MissingChainId,

    /// Creation-epoch context did not bind the number stored by the ledger.
    #[error(
        "created epoch number mismatch for {hold_id}: \
         expected={expected_epoch_number}, actual={actual_epoch_number}"
    )]
    CreatedEpochNumberMismatch {
        /// Hold lifecycle whose creation binding disagreed.
        hold_id: String,

        /// Numeric creation epoch stored by the ledger.
        expected_epoch_number: u64,

        /// Numeric creation epoch supplied by projection context.
        actual_epoch_number: u64,
    },

    /// Expiry-epoch context did not bind the number stored by the ledger.
    #[error(
        "expiry epoch number mismatch for {hold_id}: \
         expected={expected_epoch_number}, actual={actual_epoch_number}"
    )]
    ExpiresEpochNumberMismatch {
        /// Hold lifecycle whose expiry binding disagreed.
        hold_id: String,

        /// Numeric expiry epoch stored by the ledger.
        expected_epoch_number: u64,

        /// Numeric expiry epoch supplied by projection context.
        actual_epoch_number: u64,
    },

    /// One numeric epoch was assigned two different canonical identifiers.
    #[error(
        "conflicting canonical IDs for epoch number {epoch_number} while projecting {hold_id}: \
         expected={expected_epoch_id}, actual={actual_epoch_id}"
    )]
    EpochNumberBindingConflict {
        /// Hold being projected when the conflict was found.
        hold_id: String,

        /// Numeric epoch assigned conflicting identifiers.
        epoch_number: u64,

        /// Canonical identifier established earlier in this projection.
        expected_epoch_id: String,

        /// Conflicting canonical identifier supplied later.
        actual_epoch_id: String,
    },

    /// One canonical epoch identifier was assigned two different numbers.
    #[error(
        "canonical epoch ID {epoch_id} maps to conflicting numbers while projecting {hold_id}: \
         expected={expected_epoch_number}, actual={actual_epoch_number}"
    )]
    EpochIdBindingConflict {
        /// Hold being projected when the conflict was found.
        hold_id: String,

        /// Canonical identifier assigned conflicting numbers.
        epoch_id: String,

        /// Numeric epoch established earlier in this projection.
        expected_epoch_number: u64,

        /// Conflicting numeric epoch supplied later.
        actual_epoch_number: u64,
    },

    /// The assembled DTO failed the frozen ron-proto validation contract.
    #[error("invalid active-hold leaf payload for {hold_id}: {reason}")]
    InvalidActiveHoldPayload {
        /// Hold lifecycle whose assembled payload was invalid.
        hold_id: String,

        /// Bounded validation explanation from ron-proto.
        reason: String,
    },
}

impl QuickChainStateSnapshot {
    /// Project all active holds into frozen ron-proto leaf payloads.
    ///
    /// Output follows the snapshot's ascending bytewise `hold_id` order.
    /// Projection requires exactly one explicit context per active hold:
    ///
    /// - missing context rejects;
    /// - duplicate context rejects;
    /// - context for a terminal, unknown, or otherwise inactive hold rejects;
    /// - numeric epoch bindings must match ledger-owned values;
    /// - one epoch number cannot map to multiple canonical IDs;
    /// - one canonical epoch ID cannot map to multiple epoch numbers;
    /// - every assembled payload must pass ron-proto validation.
    ///
    /// An empty active-hold snapshot with no contexts returns an empty vector.
    /// This function performs no serialization, hashing, root production, IO,
    /// clock access, randomness, persistence, or state mutation.
    pub fn project_active_hold_leaf_payloads(
        &self,
        contexts: &[QuickChainActiveHoldLeafProjectionContext],
    ) -> Result<Vec<QuickChainActiveHoldLeafPayloadV1>, QuickChainLeafProjectionError> {
        let mut contexts_by_hold =
            BTreeMap::<String, &QuickChainActiveHoldLeafProjectionContext>::new();

        for context in contexts {
            let hold_id = context.hold_id().to_string();

            if contexts_by_hold.insert(hold_id.clone(), context).is_some() {
                return Err(QuickChainLeafProjectionError::DuplicateActiveHoldContext { hold_id });
            }
        }

        let active_hold_ids: BTreeSet<String> = self
            .active_holds()
            .iter()
            .map(|hold| hold.hold_id().to_string())
            .collect();

        for hold in self.active_holds() {
            if !contexts_by_hold.contains_key(hold.hold_id()) {
                return Err(QuickChainLeafProjectionError::MissingActiveHoldContext {
                    hold_id: hold.hold_id().to_string(),
                });
            }
        }

        for hold_id in contexts_by_hold.keys() {
            if !active_hold_ids.contains(hold_id) {
                return Err(QuickChainLeafProjectionError::UnknownActiveHoldContext {
                    hold_id: hold_id.clone(),
                });
            }
        }

        if self.active_holds().is_empty() {
            return Ok(Vec::new());
        }

        let chain_id = self
            .chain_id()
            .ok_or(QuickChainLeafProjectionError::MissingChainId)?;

        let mut epoch_id_by_number = BTreeMap::<u64, String>::new();
        let mut epoch_number_by_id = BTreeMap::<String, u64>::new();
        let mut payloads = Vec::with_capacity(self.active_holds().len());

        for hold in self.active_holds() {
            let context = contexts_by_hold
                .get(hold.hold_id())
                .copied()
                .ok_or_else(|| QuickChainLeafProjectionError::MissingActiveHoldContext {
                    hold_id: hold.hold_id().to_string(),
                })?;

            if context.created_at_epoch().epoch_number() != hold.created_at_epoch_number() {
                return Err(QuickChainLeafProjectionError::CreatedEpochNumberMismatch {
                    hold_id: hold.hold_id().to_string(),
                    expected_epoch_number: hold.created_at_epoch_number(),
                    actual_epoch_number: context.created_at_epoch().epoch_number(),
                });
            }

            if context.expires_at_epoch().epoch_number() != hold.expires_at_epoch_number() {
                return Err(QuickChainLeafProjectionError::ExpiresEpochNumberMismatch {
                    hold_id: hold.hold_id().to_string(),
                    expected_epoch_number: hold.expires_at_epoch_number(),
                    actual_epoch_number: context.expires_at_epoch().epoch_number(),
                });
            }

            register_epoch_binding(
                hold.hold_id(),
                context.created_at_epoch(),
                &mut epoch_id_by_number,
                &mut epoch_number_by_id,
            )?;

            register_epoch_binding(
                hold.hold_id(),
                context.expires_at_epoch(),
                &mut epoch_id_by_number,
                &mut epoch_number_by_id,
            )?;

            let payload = QuickChainActiveHoldLeafPayloadV1 {
                schema: QUICKCHAIN_ACTIVE_HOLD_LEAF_PAYLOAD_SCHEMA.to_string(),
                version: QUICKCHAIN_DTO_VERSION,
                chain_id: chain_id.to_string(),
                hold_id: hold.hold_id().to_string(),
                account_id: hold.account_id().to_string(),
                counterparty_account_id: hold.counterparty_account_id().map(str::to_owned),
                amount_minor: hold.amount_minor().to_string(),
                purpose: context.purpose().to_string(),
                created_at_epoch: context.created_at_epoch().epoch_id().to_string(),
                expires_at_epoch: context.expires_at_epoch().epoch_id().to_string(),
                status: QuickChainActiveHoldStatusV1::Open,
                operation_id: hold.opened_operation_id().to_string(),
                idempotency_key: hold.opened_idempotency_key().to_string(),
                policy_hash: context.policy_hash().clone(),
            };

            payload.validate().map_err(|error| {
                QuickChainLeafProjectionError::InvalidActiveHoldPayload {
                    hold_id: hold.hold_id().to_string(),
                    reason: error.to_string(),
                }
            })?;

            payloads.push(payload);
        }

        Ok(payloads)
    }
}

fn register_epoch_binding(
    hold_id: &str,
    binding: &QuickChainEpochBinding,
    epoch_id_by_number: &mut BTreeMap<u64, String>,
    epoch_number_by_id: &mut BTreeMap<String, u64>,
) -> Result<(), QuickChainLeafProjectionError> {
    if let Some(expected_epoch_id) = epoch_id_by_number.get(&binding.epoch_number()) {
        if expected_epoch_id != binding.epoch_id() {
            return Err(QuickChainLeafProjectionError::EpochNumberBindingConflict {
                hold_id: hold_id.to_string(),
                epoch_number: binding.epoch_number(),
                expected_epoch_id: expected_epoch_id.clone(),
                actual_epoch_id: binding.epoch_id().to_string(),
            });
        }
    } else {
        epoch_id_by_number.insert(binding.epoch_number(), binding.epoch_id().to_string());
    }

    if let Some(expected_epoch_number) = epoch_number_by_id.get(binding.epoch_id()) {
        if *expected_epoch_number != binding.epoch_number() {
            return Err(QuickChainLeafProjectionError::EpochIdBindingConflict {
                hold_id: hold_id.to_string(),
                epoch_id: binding.epoch_id().to_string(),
                expected_epoch_number: *expected_epoch_number,
                actual_epoch_number: binding.epoch_number(),
            });
        }
    } else {
        epoch_number_by_id.insert(binding.epoch_id().to_string(), binding.epoch_number());
    }

    Ok(())
}
