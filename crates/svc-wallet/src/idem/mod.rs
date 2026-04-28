//! RO:WHAT — Idempotency module tree.
//! RO:WHY  — Pillar 12; Concerns: ECON/RES/DX. Retries must not mutate value twice.
//! RO:INTERACTS — dto requests/responses, routes/v1, ledger client.
//! RO:INVARIANTS — same key+same fingerprint returns identical receipt; same key+different fingerprint conflicts.
//! RO:METRICS — wallet_idem_replays_total on replay.
//! RO:CONFIG — TTL comes from WalletConfig.
//! RO:SECURITY — stores request fingerprints and receipts, not bearer tokens.
//! RO:TEST — replay_same_request; conflict_different_request.

pub mod store;
