//! RO:WHAT — Single-writer append-only ledger engine that validates batches, appends records, computes roots, and emits checkpoints.
//! RO:WHY  — Pillar 12; Concerns: ECON/RES/GOV. This is the deterministic truth path that wallet/service wrappers will build on.
//! RO:INTERACTS — crate::api, crate::config, crate::error, crate::types, crate::engine::{storage,replay,accumulator,checkpoint,observer}.
//! RO:INVARIANTS — append-only commits; deterministic validation order; single mutation lock; replay equals live root; no floats; non-negative balances.
//! RO:METRICS — none directly; observer hooks expose committed/rejected/replayed/checkpointed events.
//! RO:CONFIG — LedgerConfig controls batch cap, checkpoint cadence, and accumulator kind.
//! RO:SECURITY — capability/KID values are stored as IDs only; external verification belongs outside this crate.
//! RO:TEST — idempotency_prop.rs, replay_recovery.rs, reject_taxonomy.rs, interop_vectors.rs, benches/micro.rs.

use std::{collections::HashMap, sync::Arc};

use parking_lot::Mutex;

use crate::{
    api::{IngestRequest, IngestResponse, RejectItem, RootItem, RootsResponse},
    config::LedgerConfig,
    error::{LedgerError, RejectReason},
    types::{AccountId, Entry, EntryRecord, Root, Seq},
};

use super::{
    checkpoint::build_checkpoint,
    observer::{LedgerEvent, NoopObserver, Observer},
    replay::{apply_entry, replay_records},
    storage::Storage,
};

#[derive(Debug, Clone)]
struct StoredBatchResponse {
    seq_start: Seq,
    seq_end: Seq,
    new_root: Root,
}

#[derive(Debug)]
struct State {
    next_seq: u64,
    head_root: Root,
    balances: HashMap<AccountId, u128>,
    seen_entry_ids: HashMap<String, Seq>,
    batch_idem: HashMap<String, StoredBatchResponse>,
    roots: Vec<RootItem>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            next_seq: 1,
            head_root: Root::zero(),
            balances: HashMap::new(),
            seen_entry_ids: HashMap::new(),
            batch_idem: HashMap::new(),
            roots: Vec::new(),
        }
    }
}

/// Deterministic ledger engine.
pub struct Ledger<S: Storage> {
    config: LedgerConfig,
    storage: Arc<S>,
    observer: Arc<dyn Observer>,
    state: Mutex<State>,
}

impl<S: Storage> Ledger<S> {
    /// Create a new ledger with a default noop observer.
    pub fn new(storage: S, config: LedgerConfig) -> Result<Self, LedgerError> {
        Self::with_observer(storage, config, NoopObserver)
    }

    /// Create a new ledger with an explicit observer.
    pub fn with_observer<O: Observer>(
        storage: S,
        config: LedgerConfig,
        observer: O,
    ) -> Result<Self, LedgerError> {
        config.validate()?;
        let storage = Arc::new(storage);
        let observer: Arc<dyn Observer> = Arc::new(observer);
        let records = storage.load_records()?;
        let checkpoints = storage.load_checkpoints()?;
        let replayed = replay_records(config.accumulator_kind, &records)?;

        let mut roots: Vec<RootItem> = checkpoints
            .iter()
            .map(|cp| RootItem {
                seq: cp.seq,
                root: cp.root,
                ts: cp.ts,
            })
            .collect();
        if replayed.next_seq > 1 {
            let head_seq = Seq(replayed.next_seq - 1);
            if roots.last().map(|r| r.seq) != Some(head_seq) {
                roots.push(RootItem {
                    seq: head_seq,
                    root: replayed.head_root,
                    ts: 0,
                });
            }
        }

        observer.on_event(&LedgerEvent::Replayed {
            entries: records.len(),
            new_root: replayed.head_root,
        });

        Ok(Self {
            config,
            storage,
            observer,
            state: Mutex::new(State {
                next_seq: replayed.next_seq,
                head_root: replayed.head_root,
                balances: replayed.balances,
                seen_entry_ids: replayed.seen_entry_ids,
                batch_idem: HashMap::new(),
                roots,
            }),
        })
    }

    /// Ingest a batch of entries.
    pub fn ingest(&self, request: IngestRequest) -> Result<IngestResponse, LedgerError> {
        self.validate_request(&request)?;

        if let Some(idem_id) = request.idem_id.as_ref() {
            let state = self.state.lock();
            if let Some(stored) = state.batch_idem.get(idem_id) {
                return Ok(IngestResponse {
                    accepted: true,
                    seq_start: Some(stored.seq_start),
                    seq_end: Some(stored.seq_end),
                    new_root: stored.new_root,
                    reasons: Vec::new(),
                });
            }
        }

        let scratch_balances = {
            let state = self.state.lock();
            for (idx, entry) in request.batch.iter().enumerate() {
                if state.seen_entry_ids.contains_key(&entry.id) {
                    self.observer.on_event(&LedgerEvent::Rejected {
                        reason: RejectReason::Conflict,
                        idx: Some(idx),
                    });
                    return Ok(IngestResponse {
                        accepted: false,
                        seq_start: None,
                        seq_end: None,
                        new_root: state.head_root,
                        reasons: vec![RejectItem {
                            idx,
                            reason: RejectReason::Conflict,
                        }],
                    });
                }
            }
            state.balances.clone()
        };

        let mut simulated = scratch_balances;
        for entry in &request.batch {
            apply_entry(&mut simulated, &entry.account, entry.kind, entry.amount)?;
        }

        let mut state = self.state.lock();
        let seq_start = Seq(state.next_seq);
        let mut prev_root = state.head_root;
        let mut last_seq = seq_start;

        for entry in &request.batch {
            let seq = Seq(state.next_seq);
            let mut record = EntryRecord {
                seq,
                entry: entry.clone(),
                prev_root,
                new_root: Root::zero(),
            };
            let new_root = crate::engine::accumulator::next_root(
                self.config.accumulator_kind,
                prev_root,
                &record,
            )?;
            record.new_root = new_root;
            self.storage.append_record(&record)?;
            apply_entry(
                &mut state.balances,
                &entry.account,
                entry.kind,
                entry.amount,
            )?;
            state.seen_entry_ids.insert(entry.id.clone(), seq);
            state.next_seq += 1;
            prev_root = new_root;
            last_seq = seq;
        }

        state.head_root = prev_root;
        let ts = request.batch.last().map_or(0, |e| e.ts);
        state.roots.push(RootItem {
            seq: last_seq,
            root: prev_root,
            ts,
        });

        let committed_count = last_seq.get();
        if committed_count % self.config.checkpoint_interval == 0 {
            let checkpoint = build_checkpoint(last_seq, prev_root, ts);
            self.storage.append_checkpoint(&checkpoint)?;
            self.observer.on_event(&LedgerEvent::Checkpointed {
                seq: checkpoint.seq,
                root: checkpoint.root,
            });
        }

        if let Some(idem_id) = request.idem_id {
            state.batch_idem.insert(
                idem_id,
                StoredBatchResponse {
                    seq_start,
                    seq_end: last_seq,
                    new_root: prev_root,
                },
            );
        }

        self.observer.on_event(&LedgerEvent::BatchCommitted {
            seq_start,
            seq_end: last_seq,
            new_root: prev_root,
            entries: request.batch.len(),
        });

        Ok(IngestResponse {
            accepted: true,
            seq_start: Some(seq_start),
            seq_end: Some(last_seq),
            new_root: prev_root,
            reasons: Vec::new(),
        })
    }

    /// Return the current balance for an account.
    pub fn balance(&self, account: &AccountId) -> Result<u128, LedgerError> {
        let state = self.state.lock();
        Ok(*state.balances.get(account).unwrap_or(&0))
    }

    /// Fetch roots after a given sequence (exclusive).
    pub fn roots_since(&self, since: u64) -> Result<RootsResponse, LedgerError> {
        let state = self.state.lock();
        let roots: Vec<RootItem> = state
            .roots
            .iter()
            .filter(|item| item.seq.get() > since)
            .cloned()
            .collect();
        Ok(RootsResponse {
            roots,
            next: Seq(state.next_seq),
        })
    }

    fn validate_request(&self, request: &IngestRequest) -> Result<(), LedgerError> {
        if request.batch.is_empty() {
            return Err(LedgerError::reject(
                RejectReason::Invalid,
                "batch must not be empty",
            ));
        }
        if request.batch.len() > self.config.limits.batch_max_entries {
            return Err(LedgerError::reject(
                RejectReason::TooLarge,
                format!(
                    "batch size {} exceeds limit {}",
                    request.batch.len(),
                    self.config.limits.batch_max_entries
                ),
            ));
        }
        let encoded = serde_json::to_vec(request)?;
        if encoded.len() > self.config.limits.max_body_bytes {
            return Err(LedgerError::reject(
                RejectReason::TooLarge,
                format!(
                    "encoded request size {} exceeds max_body_bytes {}",
                    encoded.len(),
                    self.config.limits.max_body_bytes
                ),
            ));
        }
        if let Some(idem_id) = request.idem_id.as_ref() {
            if idem_id.is_empty() || idem_id.len() > 128 {
                return Err(LedgerError::reject(
                    RejectReason::Invalid,
                    "idem_id must be 1..=128 bytes when provided",
                ));
            }
        }
        self.validate_batch_shape(&request.batch)?;
        self.validate_conservation(&request.batch)?;
        Ok(())
    }

    fn validate_batch_shape(&self, batch: &[Entry]) -> Result<(), LedgerError> {
        let mut ids = std::collections::HashSet::new();
        for entry in batch {
            if !ids.insert(entry.id.clone()) {
                return Err(LedgerError::reject(
                    RejectReason::Conflict,
                    format!("duplicate entry id {} in same batch", entry.id),
                ));
            }
        }
        Ok(())
    }

    fn validate_conservation(&self, batch: &[Entry]) -> Result<(), LedgerError> {
        let mut credits = 0_u128;
        let mut debits = 0_u128;
        let mut tracked = false;
        for entry in batch {
            if entry.kind.is_conservation_tracked() {
                tracked = true;
                if entry.kind.is_credit_like() {
                    credits = credits.saturating_add(entry.amount as u128);
                } else {
                    debits = debits.saturating_add(entry.amount as u128);
                }
            }
        }
        if tracked && credits != debits {
            return Err(LedgerError::reject(
                RejectReason::Invalid,
                format!(
                    "conservation mismatch: credits {} != debits {}",
                    credits, debits
                ),
            ));
        }
        Ok(())
    }
}
