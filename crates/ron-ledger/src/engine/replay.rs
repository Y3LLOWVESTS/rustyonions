//! RO:WHAT — Replay helpers that rebuild balances, roots, and indexes from append-only records and checkpoints.
//! RO:WHY  — Pillar 12; Concerns: ECON/RES. Recovery must produce the same state every time from the same history.
//! RO:INTERACTS — crate::engine::storage, crate::engine::accumulator, crate::engine::ledger, crate::types.
//! RO:INVARIANTS — records replay in sequence order; prev_root/new_root continuity is checked; balance rules are identical to live commit.
//! RO:METRICS — observer emits a replay-complete event; no metrics coupling here.
//! RO:CONFIG — accumulator kind influences root verification.
//! RO:SECURITY — replay validates identifiers and roots but never re-verifies external capability secrets.
//! RO:TEST — replay_recovery.rs and interop_vectors.rs.

use std::collections::HashMap;

use crate::{
    config::AccumulatorKind,
    error::{LedgerError, RejectReason},
    types::{AccountId, EntryKind, EntryRecord, Root, Seq},
};

/// Rebuilt state after replay.
#[derive(Debug, Clone)]
pub struct ReplayedState {
    /// Head root after all records.
    pub head_root: Root,
    /// Next sequence to assign.
    pub next_seq: u64,
    /// Balances reconstructed from history.
    pub balances: HashMap<AccountId, u128>,
    /// Record ids already present.
    pub seen_entry_ids: HashMap<String, Seq>,
}

/// Rebuild state from ordered records.
pub fn replay_records(
    kind: AccumulatorKind,
    records: &[EntryRecord],
) -> Result<ReplayedState, LedgerError> {
    let mut head_root = Root::zero();
    let mut next_seq = 1_u64;
    let mut balances: HashMap<AccountId, u128> = HashMap::new();
    let mut seen_entry_ids: HashMap<String, Seq> = HashMap::new();

    for record in records {
        if record.seq.get() != next_seq {
            return Err(LedgerError::reject(
                RejectReason::Conflict,
                format!(
                    "sequence gap: expected {}, got {}",
                    next_seq,
                    record.seq.get()
                ),
            ));
        }
        if record.prev_root != head_root {
            return Err(LedgerError::reject(
                RejectReason::Conflict,
                "prev_root mismatch during replay",
            ));
        }
        let expected = crate::engine::accumulator::next_root(kind, record.prev_root, record)?;
        if expected != record.new_root {
            return Err(LedgerError::reject(
                RejectReason::Conflict,
                "new_root mismatch during replay",
            ));
        }

        apply_entry(
            &mut balances,
            &record.entry.account,
            record.entry.kind,
            record.entry.amount,
        )?;
        seen_entry_ids.insert(record.entry.id.clone(), record.seq);
        head_root = record.new_root;
        next_seq += 1;
    }

    Ok(ReplayedState {
        head_root,
        next_seq,
        balances,
        seen_entry_ids,
    })
}

/// Apply a primitive entry effect to balances.
pub fn apply_entry(
    balances: &mut HashMap<AccountId, u128>,
    account: &AccountId,
    kind: EntryKind,
    amount: u64,
) -> Result<(), LedgerError> {
    let entry = balances.entry(account.clone()).or_insert(0);
    if kind.is_credit_like() {
        *entry = entry.saturating_add(amount as u128);
        return Ok(());
    }
    let current = *entry;
    let needed = amount as u128;
    if current < needed {
        return Err(LedgerError::reject(
            RejectReason::Conflict,
            format!("insufficient balance for account {}", account.as_str()),
        ));
    }
    *entry = current - needed;
    Ok(())
}
