//! RO:WHAT — Library root for svc-passport: issue/verify Ed25519 passports plus public profile claim helpers.
//! RO:WHY  — P3 Identity & Keys; Concerns: SEC/RES/PERF/GOV. Short-TTL capability tokens and username/profile contracts.
//! RO:INTERACTS — http::handlers, token::{encode,macaroon}, kms::client, state::issuer, policy::eval, profile
//! RO:INVARIANTS — strict Ed25519 capability envelope path; deterministic profile claims; no locks across .await
//! RO:METRICS — passport_ops_total, passport_failures_total, passport_op_latency_seconds, passport_batch_len
//! RO:CONFIG — see config.rs (ttl, batch, caps); /metrics + /healthz + /readyz via service HTTP surfaces
//! RO:SECURITY — no ambient authority; no wallet spend authority in profile DTOs; no private alt linkage
//! RO:TEST — tests/* (API/profile smoke), fuzz/* (envelope/caveat), loom/* (rotation under races)
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
pub mod profile;
pub mod state;
pub mod telemetry;
pub mod token;
pub mod util;
pub mod verify;

pub use crate::config::Config;
