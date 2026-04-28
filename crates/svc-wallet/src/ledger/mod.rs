//! RO:WHAT — Ledger adapter module tree for svc-wallet.
//! RO:WHY  — Pillar 12; Concerns: ECON/RES. Wallet constructs validated transactions and commits only through ron-ledger.
//! RO:INTERACTS — ron-ledger, dto requests/responses, util receipt hashing.
//! RO:INVARIANTS — ledger primacy; balanced transfer batches; deterministic ids/nonces; no direct balance DB.
//! RO:METRICS — caller maps adapter errors/successes to wallet metrics.
//! RO:CONFIG — WalletConfig amount ceilings and asset.
//! RO:SECURITY — KID/cap refs are identifiers; external cap verification is auth module’s job.
//! RO:TEST — transfer_builds_balanced_batch; issue_updates_balance.

pub mod client;
pub mod types;
