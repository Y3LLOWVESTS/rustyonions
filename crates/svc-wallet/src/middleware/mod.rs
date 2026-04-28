//! RO:WHAT — HTTP middleware module tree for svc-wallet.
//! RO:WHY  — Pillar 12; Concerns: SEC/RES/PERF. Keeps admission-control helpers separate from handlers.
//! RO:INTERACTS — routes, config, readiness, future tower layers.
//! RO:INVARIANTS — body≤1MiB; decompression≤10x; bounded inflight; no secret logging.
//! RO:METRICS — callers map decisions to wallet_rejects_total.
//! RO:CONFIG — WalletConfig limit helpers.
//! RO:SECURITY — Authorization is never logged.
//! RO:TEST — unit tests in child modules as they become active.

pub mod decompress_cap;
pub mod limits;
pub mod rate_limit;
pub mod request_id;
pub mod shedder;
pub mod timeouts;
pub mod tracing_log;
