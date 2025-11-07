//! RO:WHAT — Library root for svc-passport: issue/verify Ed25519 "passports" with versioned KID.
//! RO:WHY  — P3 Identity & Keys; Concerns: SEC/RES/PERF. Short-TTL capability tokens with batch verify.
//! RO:INTERACTS — http::handlers, token::{encode,macaroon}, kms::client, state::issuer, policy::eval
//! RO:INVARIANTS — strict Ed25519 only; deterministic envelopes; no locks across .await
//! RO:METRICS — passport_ops_total, passport_failures_total, passport_op_latency_seconds, passport_batch_len
//! RO:CONFIG — see config.rs (ttl, batch, caps); /metrics + /healthz + /readyz via ron-kernel surfaces
//! RO:SECURITY — no ambient authority; admin routes gated; zeroize ephemeral secrets
//! RO:TEST — tests/* (API smoke), fuzz/* (envelope/caveat), loom/* (rotation under races)
#![forbid(unsafe_code)]

pub mod bootstrap;
pub mod config;
pub mod dto;
pub mod error;
pub mod health;
pub mod http;
pub mod kms;
pub mod metrics;
pub mod policy;
pub mod state;
pub mod telemetry;
pub mod token;
pub mod util;
pub mod verify;

pub use crate::config::Config;
