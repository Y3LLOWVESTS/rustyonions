//! RO:WHAT — Output module facade for manifests, intents, artifacts, attestations, and wallet egress seams.
//! RO:WHY — Pillar 12; Concerns: ECON/GOV/SEC. Keeps audit output and settlement intent seams explicit.
//! RO:INTERACTS — core compute and HTTP handlers.
//! RO:INVARIANTS — manifest commitment is deterministic; intent emission is idempotent; amnesia skips disk.
//! RO:METRICS — callers update reward and ledger-intent metrics.
//! RO:CONFIG — artifact writer honors amnesia and artifact_dir; wallet path/base URL are config-driven.
//! RO:SECURITY — no private key material in public DTOs.
//! RO:TEST — unit/integration tests.

pub mod artifacts;
pub mod attestation;
pub mod intents;
pub mod manifest;
pub mod wallet;

pub use attestation::Attestation;
pub use intents::{
    plan_settlement_intents, IntentResult, IntentStore, SettlementBatch, SettlementIntent,
    WalletIssueBatch, WalletIssueRequest, ROC_ASSET, WALLET_ISSUE_PATH,
};
pub use manifest::{
    commitment_for_manifest, LedgerSummary, ManifestStatus, PolicySummary, RewardManifest,
    RewardPayout, RewardTotals,
};
pub use wallet::{
    DevWalletIssueClient, HttpWalletIssueClient, WalletHttpIssueOutcome, WalletIssueClient,
    WalletIssueOutcome,
};
