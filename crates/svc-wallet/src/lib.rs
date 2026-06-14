//! RO:WHAT — svc-wallet service internals for DTOs, nonce/idempotency gates, policy seams, and ledger commit adapters.
//! RO:WHY  — Pillar 12; Concerns: ECON/SEC/RES/DX. Wallet is the mutation front-door while ron-ledger remains truth.
//! RO:INTERACTS — dto, seq, idem, auth, policy, ledger, util; upstream ron-ledger.
//! RO:INVARIANTS — no floats; DTO hygiene; no double-spend; ledger primacy; no hidden persistence in amnesia mode.
//! RO:METRICS — metrics module owns future wallet_* counters and request_latency_seconds labels.
//! RO:CONFIG — WalletConfig carries body/decompress/timeout/inflight/idempotency/amount ceilings.
//! RO:SECURITY — capabilities are verified at the service boundary; no key custody in wallet.
//! RO:TEST — unit tests in each module; integration tests will hit HTTP once routes are wired.

#![forbid(unsafe_code)]

pub mod accounting;
pub mod auth;
pub mod cache;
pub mod config;
pub mod dto;
pub mod errors;
pub mod idem;
pub mod ledger;
pub mod metrics;
pub mod middleware;
pub mod policy;
#[cfg(feature = "quickchain-preflight")]
pub mod quickchain;
pub mod readiness;
pub mod routes;
pub mod seq;
pub mod supervisor;
pub mod util;
