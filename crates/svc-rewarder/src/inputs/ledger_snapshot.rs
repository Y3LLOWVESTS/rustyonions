//! RO:WHAT — Optional sealed ledger snapshot seam for future reward policies.
//! RO:WHY — Pillar 12; Concerns: ECON/GOV. Keeps ledger reads explicit and separate from transient accounting.
//! RO:INTERACTS — future policy algorithms, outputs::intents.
//! RO:INVARIANTS — rewarder does not mutate ledger directly; this type is read-only metadata.
//! RO:METRICS — dependency errors counted by callers.
//! RO:CONFIG — future adapter uses ingress.ledger_base_url or wallet endpoint.
//! RO:SECURITY — snapshot roots only; no private keys or token material.
//! RO:TEST — compile coverage in module tree.

use serde::{Deserialize, Serialize};

/// Read-only ledger head metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LedgerSnapshot {
    /// Ledger root hash at snapshot time.
    pub root: String,
    /// Monotonic ledger sequence.
    pub seq: u64,
}
