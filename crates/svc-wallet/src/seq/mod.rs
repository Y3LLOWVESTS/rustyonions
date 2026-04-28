//! RO:WHAT — Per-account sequence/nonce module tree.
//! RO:WHY  — Pillar 12; Concerns: ECON/RES. Nonce reservation is the first no-doublespend gate.
//! RO:INTERACTS — dto requests, ledger client, idem store.
//! RO:INVARIANTS — strict next nonce; atomic reserve/rollback; no locks across await because methods are sync.
//! RO:METRICS — future wallet_conflicts_total on nonce failures.
//! RO:CONFIG — NONCE_START from config.
//! RO:SECURITY — account ids only; no tokens.
//! RO:TEST — nonce_table_enforces_strict_next.

pub mod nonce;
