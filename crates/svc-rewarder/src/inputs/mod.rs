//! RO:WHAT — Input module facade for sealed reward computation sources.
//! RO:WHY — Pillar 12; Concerns: ECON/RES. Keeps accounting, policy, ledger snapshots, and CIDs separate.
//! RO:INTERACTS — core compute and HTTP DTOs.
//! RO:INVARIANTS — accounting is transient; ledger snapshot is read-only; CIDs are canonical.
//! RO:METRICS — dependency errors are counted by callers.
//! RO:CONFIG — future adapters use ingress endpoints and cache TTL.
//! RO:SECURITY — signed policy verification seam remains explicit.
//! RO:TEST — unit/integration reward compute tests.

pub mod accounting;
pub mod cache;
pub mod cid;
pub mod ledger_snapshot;
pub mod policy;

pub use accounting::{
    canonical_snapshot_cid, resolve_accounting_snapshot, AccountContribution, AccountingSnapshot,
};
pub use cid::ContentCid;
pub use ledger_snapshot::LedgerSnapshot;
pub use policy::{
    policy_hash_is_canonical, resolve_reward_policy, validate_reward_policy, RewardPolicy,
};
