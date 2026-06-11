//! RO:WHAT — QuickChain Phase 0 concurrent-hold replay and closed-hold compaction scenario DTOs.
//! RO:WHY — ECON/RES: retries and terminal hold cleanup must be unambiguous before state transition or root code exists.
//! RO:INTERACTS — operation intents, hold states, replay semantics, future ron-ledger replay, QC-0A vectors.
//! RO:INVARIANTS — DTO/validation only; retries do not add commits; only open holds remain active; terminal receipt evidence remains.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — no balance authority, mutation, persistence, hashing, roots, expiry worker, or settlement.
//! RO:TEST — tests/quickchain_hold_scenarios.rs and hold_scenarios vector files.

use std::collections::{btree_map::Entry, BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::{
    ids::validate_hold_id_v1,
    money::validate_quickchain_minor_units,
    operation::{QuickChainOperationClassV1, QuickChainOperationIntentV1},
    validate_chain_id, validate_ref, validate_schema, validate_version, QuickChainHoldStateV1,
    QuickChainHoldStatusV1, QuickChainResult, QuickChainValidationError,
};

/// Schema tag for concurrent-hold replay scenario DTOs.
pub const QUICKCHAIN_CONCURRENT_HOLD_REPLAY_SCHEMA: &str = "quickchain.concurrent-hold-replay.v1";

/// Schema tag for closed-hold compaction scenario DTOs.
pub const QUICKCHAIN_HOLD_COMPACTION_SCHEMA: &str = "quickchain.hold-compaction.v1";

/// Maximum submitted operations in one Phase 0 hold replay scenario.
pub const MAX_QUICKCHAIN_HOLD_REPLAY_STEPS: usize = 64;

/// Maximum hold states in one Phase 0 compaction scenario.
pub const MAX_QUICKCHAIN_COMPACTION_HOLDS: usize = 128;

/// Outcome of one submitted operation in a concurrent-hold replay scenario.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainHoldReplayStepOutcomeV1 {
    /// The backend accepted one new durable operation and produced one receipt.
    Committed,

    /// An identical retry returns the original receipt without another commit.
    ReturnOriginalReceipt,
}

/// Safety properties recorded by the concurrent-hold replay scenario.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainHoldReplayInvariantV1 {
    /// Retrying capture cannot debit or capture the held value twice.
    RetryCaptureNoDoubleSpend,

    /// Retrying release cannot change state or recreate the hold.
    RetryReleaseNoStateChange,

    /// Account sequence advances only for newly committed operations.
    AccountSequenceAdvancesOnCommitOnly,

    /// Captured, released, and expired holds are absent from active state.
    ClosedHoldsAbsentFromActiveSet,

    /// Terminal hold history remains available through receipt evidence.
    TerminalLifecycleRetainedByReceipt,
}

/// Safety properties recorded by a closed-hold compaction scenario.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainHoldCompactionInvariantV1 {
    /// The future active-hold collection contains open holds only.
    OnlyOpenHoldsRemainActive,

    /// Terminal holds retain one matching terminal receipt reference.
    TerminalHoldsRetainReceiptEvidence,

    /// A terminal hold ID cannot become active again.
    TerminalHoldsCannotResurrect,

    /// Input or database iteration order does not determine active output order.
    InputOrderDoesNotAffectActiveSet,

    /// Duplicate hold IDs are rejected rather than silently overwritten.
    DuplicateHoldIdsReject,
}

/// Scenario-only account snapshot.
///
/// This is not the future canonical account-state leaf and must not be hashed as
/// one. It records human-reviewable replay expectations only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainHoldReplayAccountSnapshotV1 {
    pub account_id: String,
    pub total_minor: String,
    pub available_minor: String,
    pub held_minor: String,
    pub account_sequence: u64,
}

impl QuickChainHoldReplayAccountSnapshotV1 {
    /// Validate snapshot shape and basic accounting consistency.
    ///
    /// This performs no transition execution. It only rejects a contradictory
    /// snapshot where total does not equal available plus held.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_ref("account_snapshot.account_id", &self.account_id)?;

        let total = parse_minor("account_snapshot.total_minor", &self.total_minor)?;
        let available = parse_minor("account_snapshot.available_minor", &self.available_minor)?;
        let held = parse_minor("account_snapshot.held_minor", &self.held_minor)?;

        let combined =
            available
                .checked_add(held)
                .ok_or(QuickChainValidationError::InvalidField {
                    field: "account_snapshot",
                    reason: "available_minor plus held_minor must fit in u128",
                })?;

        if total != combined {
            return Err(QuickChainValidationError::InvalidField {
                field: "account_snapshot",
                reason: "total_minor must equal available_minor plus held_minor",
            });
        }

        Ok(())
    }
}

/// One submitted operation and its expected backend replay disposition.
///
/// `receipt_account_sequence` is the sequence recorded by the returned receipt.
/// For a retry, it remains the sequence assigned to the original operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainHoldReplayStepV1 {
    pub ordinal: u16,
    pub intent: QuickChainOperationIntentV1,
    pub expected_outcome: QuickChainHoldReplayStepOutcomeV1,
    pub receipt_txid: String,
    pub receipt_account_sequence: u64,
}

impl QuickChainHoldReplayStepV1 {
    /// Validate one replay step without executing it.
    pub fn validate(&self) -> QuickChainResult<()> {
        if self.ordinal == 0 {
            return Err(QuickChainValidationError::InvalidField {
                field: "steps[].ordinal",
                reason: "must be greater than zero",
            });
        }

        self.intent.validate()?;
        validate_hold_operation_class(self.intent.op_class)?;
        validate_ref("steps[].receipt_txid", &self.receipt_txid)?;

        if self.receipt_account_sequence == 0 {
            return Err(QuickChainValidationError::InvalidField {
                field: "steps[].receipt_account_sequence",
                reason: "must be greater than zero",
            });
        }

        Ok(())
    }
}

/// Phase 0 concurrent-hold replay scenario.
///
/// This records expected deterministic results only. It does not execute holds,
/// calculate balances from operations, store idempotency state, or mint receipts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainConcurrentHoldReplayV1 {
    pub schema: String,
    pub version: u16,
    pub chain_id: String,
    pub scenario_id: String,
    pub initial_account: QuickChainHoldReplayAccountSnapshotV1,
    pub steps: Vec<QuickChainHoldReplayStepV1>,
    pub expected_final_account: QuickChainHoldReplayAccountSnapshotV1,
    pub expected_active_hold_ids: Vec<String>,
    pub expected_terminal_receipt_txids: Vec<String>,
    pub expected_economic_commit_count: u16,
    pub expected_state_transition_count: u16,
    pub invariants: Vec<QuickChainHoldReplayInvariantV1>,
}

impl QuickChainConcurrentHoldReplayV1 {
    /// Validate replay identity, receipt reuse, and ledger sequence expectations.
    ///
    /// This deliberately does not calculate balance transitions or mutate an
    /// active-hold collection.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainConcurrentHoldReplayV1.schema",
            &self.schema,
            QUICKCHAIN_CONCURRENT_HOLD_REPLAY_SCHEMA,
        )?;
        validate_version("QuickChainConcurrentHoldReplayV1.version", self.version)?;
        validate_chain_id(&self.chain_id)?;
        validate_ref("scenario_id", &self.scenario_id)?;

        self.initial_account.validate()?;
        self.expected_final_account.validate()?;

        if self.initial_account.account_id != self.expected_final_account.account_id {
            return Err(QuickChainValidationError::InvalidField {
                field: "expected_final_account.account_id",
                reason: "must match initial_account.account_id",
            });
        }

        if self.steps.is_empty() {
            return Err(QuickChainValidationError::InvalidField {
                field: "steps",
                reason: "must contain at least one submitted operation",
            });
        }

        if self.steps.len() > MAX_QUICKCHAIN_HOLD_REPLAY_STEPS {
            return Err(QuickChainValidationError::TooManyItems {
                field: "steps",
                max: MAX_QUICKCHAIN_HOLD_REPLAY_STEPS,
                actual: self.steps.len(),
            });
        }

        validate_required_replay_invariants(&self.invariants)?;
        validate_sorted_unique_hold_ids(
            "expected_active_hold_ids",
            &self.expected_active_hold_ids,
        )?;
        validate_unique_refs(
            "expected_terminal_receipt_txids",
            &self.expected_terminal_receipt_txids,
        )?;

        let mut committed_operations: BTreeMap<String, (QuickChainOperationIntentV1, String, u64)> =
            BTreeMap::new();
        let mut committed_receipt_txids = BTreeSet::new();
        let mut committed_terminal_receipt_txids = Vec::new();
        let mut last_committed_sequence = self.initial_account.account_sequence;
        let mut committed_count = 0_usize;

        for (index, step) in self.steps.iter().enumerate() {
            step.validate()?;

            let expected_ordinal =
                u16::try_from(index + 1).map_err(|_| QuickChainValidationError::InvalidField {
                    field: "steps[].ordinal",
                    reason: "scenario contains too many steps for u16 ordinal",
                })?;

            if step.ordinal != expected_ordinal {
                return Err(QuickChainValidationError::InvalidField {
                    field: "steps[].ordinal",
                    reason: "must be contiguous and begin at one",
                });
            }

            if step.intent.chain_id != self.chain_id {
                return Err(QuickChainValidationError::InvalidField {
                    field: "steps[].intent.chain_id",
                    reason: "must match scenario chain_id",
                });
            }

            if step.intent.actor_account_id != self.initial_account.account_id {
                return Err(QuickChainValidationError::InvalidField {
                    field: "steps[].intent.actor_account_id",
                    reason: "must match the scenario account",
                });
            }

            match step.expected_outcome {
                QuickChainHoldReplayStepOutcomeV1::Committed => {
                    let expected_sequence = last_committed_sequence.checked_add(1).ok_or(
                        QuickChainValidationError::InvalidField {
                            field: "steps[].receipt_account_sequence",
                            reason: "account sequence overflow",
                        },
                    )?;

                    if step.receipt_account_sequence != expected_sequence {
                        return Err(QuickChainValidationError::InvalidField {
                            field: "steps[].receipt_account_sequence",
                            reason: "new commits must advance account sequence by exactly one",
                        });
                    }

                    if !committed_receipt_txids.insert(step.receipt_txid.clone()) {
                        return Err(QuickChainValidationError::InvalidField {
                            field: "steps[].receipt_txid",
                            reason: "new commits must have unique receipt txids",
                        });
                    }

                    match committed_operations.entry(step.intent.operation_id.clone()) {
                        Entry::Vacant(entry) => {
                            entry.insert((
                                step.intent.clone(),
                                step.receipt_txid.clone(),
                                step.receipt_account_sequence,
                            ));
                        }
                        Entry::Occupied(_) => {
                            return Err(QuickChainValidationError::InvalidField {
                                field: "steps[].intent.operation_id",
                                reason: "one durable operation_id may commit only once",
                            });
                        }
                    }

                    if matches!(
                        step.intent.op_class,
                        QuickChainOperationClassV1::HoldCapture
                            | QuickChainOperationClassV1::HoldRelease
                            | QuickChainOperationClassV1::HoldExpire
                    ) {
                        committed_terminal_receipt_txids.push(step.receipt_txid.clone());
                    }

                    last_committed_sequence = step.receipt_account_sequence;
                    committed_count += 1;
                }

                QuickChainHoldReplayStepOutcomeV1::ReturnOriginalReceipt => {
                    let Some((original_intent, original_txid, original_sequence)) =
                        committed_operations.get(&step.intent.operation_id)
                    else {
                        return Err(QuickChainValidationError::InvalidField {
                            field: "steps[].intent.operation_id",
                            reason: "retry must reference an earlier committed operation",
                        });
                    };

                    if &step.intent != original_intent {
                        return Err(QuickChainValidationError::InvalidField {
                            field: "steps[].intent",
                            reason: "retry intent must exactly match the original committed intent",
                        });
                    }

                    if &step.receipt_txid != original_txid {
                        return Err(QuickChainValidationError::InvalidField {
                            field: "steps[].receipt_txid",
                            reason: "retry must return the original receipt txid",
                        });
                    }

                    if step.receipt_account_sequence != *original_sequence {
                        return Err(QuickChainValidationError::InvalidField {
                            field: "steps[].receipt_account_sequence",
                            reason: "retry must retain the original receipt account sequence",
                        });
                    }
                }
            }
        }

        if self.expected_final_account.account_sequence != last_committed_sequence {
            return Err(QuickChainValidationError::InvalidField {
                field: "expected_final_account.account_sequence",
                reason: "must equal the last newly committed account sequence",
            });
        }

        if usize::from(self.expected_economic_commit_count) != committed_count {
            return Err(QuickChainValidationError::InvalidField {
                field: "expected_economic_commit_count",
                reason: "must equal the number of newly committed operations",
            });
        }

        if usize::from(self.expected_state_transition_count) != committed_count {
            return Err(QuickChainValidationError::InvalidField {
                field: "expected_state_transition_count",
                reason: "retries must not add state transitions",
            });
        }

        if self.expected_terminal_receipt_txids != committed_terminal_receipt_txids {
            return Err(QuickChainValidationError::InvalidField {
                field: "expected_terminal_receipt_txids",
                reason: "must match committed terminal hold operations in scenario order",
            });
        }

        Ok(())
    }
}

/// Receipt evidence retained for one terminal hold lifecycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainTerminalHoldReceiptRefV1 {
    pub hold_id: String,
    pub terminal_status: QuickChainHoldStatusV1,
    pub receipt_txid: String,
}

impl QuickChainTerminalHoldReceiptRefV1 {
    /// Validate one terminal hold receipt reference.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_hold_id_v1("terminal_receipts[].hold_id", &self.hold_id)?;
        validate_ref("terminal_receipts[].receipt_txid", &self.receipt_txid)?;

        if matches!(self.terminal_status, QuickChainHoldStatusV1::Open) {
            return Err(QuickChainValidationError::InvalidField {
                field: "terminal_receipts[].terminal_status",
                reason: "must be captured, released, or expired",
            });
        }

        Ok(())
    }
}

/// Phase 0 closed-hold compaction scenario.
///
/// Input hold order is intentionally not authoritative. Validation derives the
/// sorted active set from statuses and requires complete terminal receipt
/// evidence without building a tree or producing a root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainHoldCompactionV1 {
    pub schema: String,
    pub version: u16,
    pub chain_id: String,
    pub scenario_id: String,
    pub unordered_holds: Vec<QuickChainHoldStateV1>,
    pub terminal_receipts: Vec<QuickChainTerminalHoldReceiptRefV1>,
    pub expected_active_hold_ids: Vec<String>,
    pub expected_rejected_resurrection_hold_ids: Vec<String>,
    pub expected_compacted_terminal_count: u16,
    pub invariants: Vec<QuickChainHoldCompactionInvariantV1>,
}

impl QuickChainHoldCompactionV1 {
    /// Validate deterministic open-only compaction expectations.
    ///
    /// This does not delete data, mutate a hold table, build an active-hold
    /// tree, verify receipts, or produce a root.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainHoldCompactionV1.schema",
            &self.schema,
            QUICKCHAIN_HOLD_COMPACTION_SCHEMA,
        )?;
        validate_version("QuickChainHoldCompactionV1.version", self.version)?;
        validate_chain_id(&self.chain_id)?;
        validate_ref("scenario_id", &self.scenario_id)?;

        if self.unordered_holds.is_empty() {
            return Err(QuickChainValidationError::InvalidField {
                field: "unordered_holds",
                reason: "must contain at least one hold",
            });
        }

        if self.unordered_holds.len() > MAX_QUICKCHAIN_COMPACTION_HOLDS {
            return Err(QuickChainValidationError::TooManyItems {
                field: "unordered_holds",
                max: MAX_QUICKCHAIN_COMPACTION_HOLDS,
                actual: self.unordered_holds.len(),
            });
        }

        validate_required_compaction_invariants(&self.invariants)?;
        validate_sorted_unique_hold_ids(
            "expected_active_hold_ids",
            &self.expected_active_hold_ids,
        )?;
        validate_sorted_unique_hold_ids(
            "expected_rejected_resurrection_hold_ids",
            &self.expected_rejected_resurrection_hold_ids,
        )?;

        let mut hold_statuses = BTreeMap::new();
        let mut active_hold_ids = BTreeSet::new();
        let mut terminal_hold_ids = BTreeSet::new();

        for hold in &self.unordered_holds {
            hold.validate()?;

            if hold.chain_id != self.chain_id {
                return Err(QuickChainValidationError::InvalidField {
                    field: "unordered_holds[].chain_id",
                    reason: "must match compaction scenario chain_id",
                });
            }

            match hold_statuses.entry(hold.hold_id.clone()) {
                Entry::Vacant(entry) => {
                    entry.insert(hold.status);
                }
                Entry::Occupied(_) => {
                    return Err(QuickChainValidationError::InvalidField {
                        field: "unordered_holds",
                        reason: "duplicate hold_id is forbidden",
                    });
                }
            }

            match hold.status {
                QuickChainHoldStatusV1::Open => {
                    active_hold_ids.insert(hold.hold_id.clone());
                }
                QuickChainHoldStatusV1::Captured
                | QuickChainHoldStatusV1::Released
                | QuickChainHoldStatusV1::Expired => {
                    terminal_hold_ids.insert(hold.hold_id.clone());
                }
            }
        }

        let derived_active_hold_ids: Vec<String> = active_hold_ids.into_iter().collect();

        if self.expected_active_hold_ids != derived_active_hold_ids {
            return Err(QuickChainValidationError::InvalidField {
                field: "expected_active_hold_ids",
                reason: "must contain exactly the sorted open hold IDs",
            });
        }

        let mut receipt_hold_ids = BTreeSet::new();
        let mut receipt_txids = BTreeSet::new();

        for receipt in &self.terminal_receipts {
            receipt.validate()?;

            let Some(actual_status) = hold_statuses.get(&receipt.hold_id) else {
                return Err(QuickChainValidationError::InvalidField {
                    field: "terminal_receipts[].hold_id",
                    reason: "must reference a hold in unordered_holds",
                });
            };

            if *actual_status != receipt.terminal_status {
                return Err(QuickChainValidationError::InvalidField {
                    field: "terminal_receipts[].terminal_status",
                    reason: "must match the referenced terminal hold status",
                });
            }

            if !receipt_hold_ids.insert(receipt.hold_id.clone()) {
                return Err(QuickChainValidationError::InvalidField {
                    field: "terminal_receipts",
                    reason: "each terminal hold must have exactly one receipt reference",
                });
            }

            if !receipt_txids.insert(receipt.receipt_txid.clone()) {
                return Err(QuickChainValidationError::InvalidField {
                    field: "terminal_receipts[].receipt_txid",
                    reason: "terminal receipt txids must be unique",
                });
            }
        }

        if receipt_hold_ids != terminal_hold_ids {
            return Err(QuickChainValidationError::InvalidField {
                field: "terminal_receipts",
                reason: "must contain exactly one receipt reference for every terminal hold",
            });
        }

        let rejected_resurrection_ids: BTreeSet<String> = self
            .expected_rejected_resurrection_hold_ids
            .iter()
            .cloned()
            .collect();

        if rejected_resurrection_ids != terminal_hold_ids {
            return Err(QuickChainValidationError::InvalidField {
                field: "expected_rejected_resurrection_hold_ids",
                reason: "must contain exactly all terminal hold IDs",
            });
        }

        if usize::from(self.expected_compacted_terminal_count) != terminal_hold_ids.len() {
            return Err(QuickChainValidationError::InvalidField {
                field: "expected_compacted_terminal_count",
                reason: "must equal the number of terminal holds removed from active state",
            });
        }

        Ok(())
    }
}

fn validate_hold_operation_class(op_class: QuickChainOperationClassV1) -> QuickChainResult<()> {
    match op_class {
        QuickChainOperationClassV1::HoldOpen
        | QuickChainOperationClassV1::HoldCapture
        | QuickChainOperationClassV1::HoldRelease
        | QuickChainOperationClassV1::HoldExpire => Ok(()),

        QuickChainOperationClassV1::Issue
        | QuickChainOperationClassV1::Transfer
        | QuickChainOperationClassV1::Burn => Err(QuickChainValidationError::InvalidField {
            field: "steps[].intent.op_class",
            reason: "concurrent-hold scenarios accept hold lifecycle operations only",
        }),
    }
}

fn parse_minor(field: &'static str, value: &str) -> QuickChainResult<u128> {
    validate_quickchain_minor_units(field, value)?;

    value
        .parse::<u128>()
        .map_err(|_| QuickChainValidationError::InvalidMoney {
            field,
            reason: "must fit in u128 minor units",
        })
}

fn validate_sorted_unique_hold_ids(
    field: &'static str,
    hold_ids: &[String],
) -> QuickChainResult<()> {
    for hold_id in hold_ids {
        validate_hold_id_v1(field, hold_id)?;
    }

    for pair in hold_ids.windows(2) {
        if pair[0] == pair[1] {
            return Err(QuickChainValidationError::InvalidField {
                field,
                reason: "duplicate hold IDs are forbidden",
            });
        }

        if pair[0] > pair[1] {
            return Err(QuickChainValidationError::InvalidField {
                field,
                reason: "hold IDs must be in ascending bytewise order",
            });
        }
    }

    Ok(())
}

fn validate_unique_refs(field: &'static str, values: &[String]) -> QuickChainResult<()> {
    let mut seen = BTreeSet::new();

    for value in values {
        validate_ref(field, value)?;

        if !seen.insert(value.clone()) {
            return Err(QuickChainValidationError::InvalidField {
                field,
                reason: "duplicate references are forbidden",
            });
        }
    }

    Ok(())
}

fn validate_required_replay_invariants(
    invariants: &[QuickChainHoldReplayInvariantV1],
) -> QuickChainResult<()> {
    let actual: BTreeSet<_> = invariants.iter().copied().collect();

    if actual.len() != invariants.len() {
        return Err(QuickChainValidationError::InvalidField {
            field: "invariants",
            reason: "duplicate replay invariants are forbidden",
        });
    }

    let required: BTreeSet<_> = [
        QuickChainHoldReplayInvariantV1::RetryCaptureNoDoubleSpend,
        QuickChainHoldReplayInvariantV1::RetryReleaseNoStateChange,
        QuickChainHoldReplayInvariantV1::AccountSequenceAdvancesOnCommitOnly,
        QuickChainHoldReplayInvariantV1::ClosedHoldsAbsentFromActiveSet,
        QuickChainHoldReplayInvariantV1::TerminalLifecycleRetainedByReceipt,
    ]
    .into_iter()
    .collect();

    if actual != required {
        return Err(QuickChainValidationError::InvalidField {
            field: "invariants",
            reason: "must contain the complete concurrent-hold safety invariant set",
        });
    }

    Ok(())
}

fn validate_required_compaction_invariants(
    invariants: &[QuickChainHoldCompactionInvariantV1],
) -> QuickChainResult<()> {
    let actual: BTreeSet<_> = invariants.iter().copied().collect();

    if actual.len() != invariants.len() {
        return Err(QuickChainValidationError::InvalidField {
            field: "invariants",
            reason: "duplicate compaction invariants are forbidden",
        });
    }

    let required: BTreeSet<_> = [
        QuickChainHoldCompactionInvariantV1::OnlyOpenHoldsRemainActive,
        QuickChainHoldCompactionInvariantV1::TerminalHoldsRetainReceiptEvidence,
        QuickChainHoldCompactionInvariantV1::TerminalHoldsCannotResurrect,
        QuickChainHoldCompactionInvariantV1::InputOrderDoesNotAffectActiveSet,
        QuickChainHoldCompactionInvariantV1::DuplicateHoldIdsReject,
    ]
    .into_iter()
    .collect();

    if actual != required {
        return Err(QuickChainValidationError::InvalidField {
            field: "invariants",
            reason: "must contain the complete closed-hold compaction invariant set",
        });
    }

    Ok(())
}
