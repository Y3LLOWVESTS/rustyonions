//! RO:WHAT — Utility module tree for receipt hashing, header parsing, and validation helpers.
//! RO:WHY  — Pillar 12; Concerns: DX/SEC/RES. Keeps route code small and deterministic.
//! RO:INTERACTS — dto, routes, idem, ledger.
//! RO:INVARIANTS — deterministic hashes; bounded header values; no secret logging.
//! RO:METRICS — none directly.
//! RO:CONFIG — parsing helpers receive WalletConfig when needed.
//! RO:SECURITY — Authorization is never parsed here for logging; caps module handles auth semantics.
//! RO:TEST — unit tests in child modules.

pub mod blake3_receipt;
pub mod headers;
pub mod parsing;
