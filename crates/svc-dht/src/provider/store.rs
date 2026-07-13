//! RO:WHAT — Privacy-safe provider records, local status observations, and candidate selection.
//! RO:WHY — Phase 12 discovery must reject malformed IDs and avoid unavailable providers.
//! RO:INTERACTS — provider::record, types::{B3Cid, CrabNodeId}, rpc::http, pipeline::lookup.
//! RO:INVARIANTS — canonical B3 keys; TTL respected; local status cannot be self-reset.
//! RO:SECURITY — records contain only typed content and crab://node identities.
//! RO:TEST — tests/provider_selection.rs, tests/provider_roundtrip.rs,
//!   tests/crab_node_identity.rs, tests/provider_withdrawal.rs.

use super::record::{ProviderRecord, ProviderStatusHint};
use crate::types::{B3Cid, CrabNodeId, CrabNodeIdError};
use parking_lot::RwLock;
use std::{
    collections::HashMap,
    fmt,
    time::{Duration, Instant},
};

/// Provider-record validation failure at the store boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderStoreError {
    /// The object identifier was not canonical BLAKE3-256 form.
    InvalidCid(&'static str),

    /// The provider identity was not canonical crab://node form.
    InvalidNode(CrabNodeIdError),
}

impl ProviderStoreError {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidCid(reason) => reason,
            Self::InvalidNode(reason) => reason.as_str(),
        }
    }
}

impl fmt::Display for ProviderStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCid(reason) => write!(f, "invalid b3 cid: {reason}"),
            Self::InvalidNode(reason) => {
                write!(f, "invalid crab node URI: {reason}")
            }
        }
    }
}

impl std::error::Error for ProviderStoreError {}

/// Truthful result of withdrawing one exact provider record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderWithdrawalOutcome {
    /// The exact CID and node record existed and was removed.
    Withdrawn,

    /// No record existed for that exact CID and node identity.
    NotFound,
}

/// Truthful result of recording a local provider-status observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderStatusUpdateOutcome {
    /// A live record existed and received the local observation.
    Updated,

    /// No matching live provider record existed.
    NotFound,
}

#[derive(Default)]
pub struct Store {
    inner: RwLock<HashMap<B3Cid, Vec<ProviderRecord>>>,
    default_ttl: Duration,
}

impl Store {
    pub fn new(default_ttl: Duration) -> Self {
        Self { inner: RwLock::new(HashMap::new()), default_ttl }
    }

    pub fn default_ttl(&self) -> Duration {
        self.default_ttl
    }

    /// Validate and add or refresh an externally advertised provider record.
    ///
    /// The advertisement cannot supply or reset the locally observed status.
    pub fn add(
        &self,
        cid: String,
        node_uri: String,
        ttl: Option<Duration>,
    ) -> Result<(), ProviderStoreError> {
        let cid = cid.parse::<B3Cid>().map_err(ProviderStoreError::InvalidCid)?;

        let node = CrabNodeId::from_uri(&node_uri).map_err(ProviderStoreError::InvalidNode)?;

        self.add_id(cid, node, ttl);
        Ok(())
    }

    /// Add or refresh a provider using already-validated typed IDs.
    ///
    /// Refreshing an existing advertisement extends its expiry but deliberately
    /// preserves the local status observation. A provider therefore cannot
    /// erase a degraded or unavailable observation by advertising again.
    pub fn add_id(&self, cid: B3Cid, node: CrabNodeId, ttl: Option<Duration>) {
        let ttl = ttl.unwrap_or(self.default_ttl);

        let mut records = self.inner.write();
        let providers = records.entry(cid.clone()).or_default();

        if let Some(existing) = providers.iter_mut().find(|record| record.node == node) {
            existing.refresh_expiry(ttl);
        } else {
            providers.push(ProviderRecord::new(cid, node, ttl));
        }
    }

    /// Record a local, non-authoritative provider-status observation.
    ///
    /// Callers should use this only after a local probe, fetch attempt, timeout,
    /// integrity review, or equivalent observed behavior. Expired and missing
    /// records are not updated.
    pub fn record_local_status(
        &self,
        cid: &str,
        node: CrabNodeId,
        status_hint: ProviderStatusHint,
    ) -> ProviderStatusUpdateOutcome {
        let Ok(cid) = cid.parse::<B3Cid>() else {
            return ProviderStatusUpdateOutcome::NotFound;
        };

        let now = Instant::now();
        let mut records = self.inner.write();

        let Some(record) = records.get_mut(&cid).and_then(|providers| {
            providers.iter_mut().find(|record| record.node == node && !record.expired(now))
        }) else {
            return ProviderStatusUpdateOutcome::NotFound;
        };

        record.set_status_hint(status_hint);
        ProviderStatusUpdateOutcome::Updated
    }

    /// Select bounded, privacy-safe provider candidates.
    ///
    /// Selection order is deterministic:
    ///
    /// 1. responsive
    /// 2. unknown
    /// 3. degraded
    /// 4. unavailable records are excluded
    ///
    /// Equal-status candidates are ordered by canonical Crab node ID bytes.
    pub fn select_candidates(&self, cid: &str, limit: usize) -> Vec<CrabNodeId> {
        self.select_candidates_at(cid, limit, Instant::now())
    }

    /// Deterministic-time variant used by tests and controlled local review.
    pub fn select_candidates_at(&self, cid: &str, limit: usize, now: Instant) -> Vec<CrabNodeId> {
        if limit == 0 {
            return Vec::new();
        }

        let Ok(cid) = cid.parse::<B3Cid>() else {
            return Vec::new();
        };

        let records = self.inner.read();

        let mut candidates: Vec<(u8, CrabNodeId)> = records
            .get(&cid)
            .into_iter()
            .flatten()
            .filter(|record| !record.expired(now))
            .filter_map(|record| {
                record.status_hint().selection_rank().map(|rank| (rank, record.node))
            })
            .collect();

        candidates.sort_unstable_by(|(left_rank, left_node), (right_rank, right_node)| {
            left_rank.cmp(right_rank).then_with(|| left_node.as_bytes().cmp(right_node.as_bytes()))
        });

        candidates.truncate(limit);

        candidates.into_iter().map(|(_, node)| node).collect()
    }

    /// Withdraw one exact provider identity for one exact CID.
    ///
    /// Invalid CIDs are treated as absent rather than normalized into another
    /// key. Both live and expired matching records are removed so an explicit
    /// prune cannot leave a stale local advertisement.
    pub fn withdraw_id(&self, cid: &str, node: CrabNodeId) -> ProviderWithdrawalOutcome {
        let Ok(cid) = cid.parse::<B3Cid>() else {
            return ProviderWithdrawalOutcome::NotFound;
        };

        let mut records = self.inner.write();

        let Some(providers) = records.get_mut(&cid) else {
            return ProviderWithdrawalOutcome::NotFound;
        };

        let before = providers.len();
        providers.retain(|record| record.node != node);

        let removed = providers.len() != before;
        let remove_cid_entry = providers.is_empty();

        if remove_cid_entry {
            records.remove(&cid);
        }

        if removed {
            ProviderWithdrawalOutcome::Withdrawn
        } else {
            ProviderWithdrawalOutcome::NotFound
        }
    }

    /// Read-only view of all non-expired provider identities.
    ///
    /// This is an inventory view, not candidate selection. Use
    /// `select_candidates` for availability-aware fetch ordering.
    pub fn get_live_ids(&self, cid: &str) -> Vec<CrabNodeId> {
        let Ok(cid) = cid.parse::<B3Cid>() else {
            return Vec::new();
        };

        let now = Instant::now();
        let records = self.inner.read();

        records
            .get(&cid)
            .map(|providers| {
                providers
                    .iter()
                    .filter(|record| !record.expired(now))
                    .map(|record| record.node)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Read-only API and CLI representation as crab://node URIs.
    pub fn get_live(&self, cid: &str) -> Vec<String> {
        self.get_live_ids(cid).into_iter().map(CrabNodeId::to_uri).collect()
    }

    /// Prune expired records; called by the background pruner.
    pub fn purge_expired(&self) -> usize {
        let now = Instant::now();
        let mut records = self.inner.write();
        let mut purged = 0usize;

        for providers in records.values_mut() {
            let before = providers.len();
            providers.retain(|record| !record.expired(now));
            purged += before.saturating_sub(providers.len());
        }

        records.retain(|_, providers| !providers.is_empty());
        purged
    }

    /// Debug snapshot with canonical IDs, status hints, and TTL remaining.
    pub fn debug_snapshot(&self) -> Vec<DebugCid> {
        let now = Instant::now();
        let records = self.inner.read();
        let mut out = Vec::new();

        for (cid, providers) in records.iter() {
            let entries = providers
                .iter()
                .map(|record| DebugEntry {
                    node: record.node.to_uri(),
                    status_hint: record.status_hint().as_str(),
                    secs_left: record.expires_at.saturating_duration_since(now).as_secs_f64(),
                })
                .collect();

            out.push(DebugCid { cid: cid.to_string(), entries });
        }

        out
    }
}

#[derive(serde::Serialize)]
pub struct DebugCid {
    pub cid: String,
    pub entries: Vec<DebugEntry>,
}

#[derive(serde::Serialize)]
pub struct DebugEntry {
    pub node: String,
    pub status_hint: &'static str,
    pub secs_left: f64,
}
