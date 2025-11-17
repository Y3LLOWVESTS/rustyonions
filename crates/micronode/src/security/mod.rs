// crates/micronode/src/security/mod.rs
//! RO:WHAT — Security helpers for Micronode (amnesia posture, capability extraction).
//! RO:WHY  — Keep core security decisions such as amnesia-first posture and macaroon
//!           headers in one place.
//! RO:INTERACTS — `amnesia` reads env toggles and `auth_macaroon` extracts raw
//!                macaroons from HTTP requests.
//! RO:INVARIANTS — Defaults are amnesia-first and macaroons are treated as opaque blobs.
//! RO:CONFIG — Env keys for amnesia include `MICRO_AMNESIA` and a legacy `MICRO_PERSIST`.
//! RO:SECURITY — This module does not verify capabilities; that work lives in
//!                `ron-auth` and `ron-policy`.
//! RO:TEST — To be covered by `tests/amnesia_proof.rs` and future security tests.

pub mod amnesia;
pub mod auth_macaroon;

// PQ / TLS modules are kept as stubs for now; they will be fleshed out later.
pub mod pq_config;
pub mod pq_observe;
pub mod pq_toggle;
pub mod tls_rustls;
