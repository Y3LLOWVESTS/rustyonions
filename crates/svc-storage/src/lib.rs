//! svc-storage library entry — exposes modules to the bin target.
//! RO:WHAT  — Crate root and module exposes.
//! RO:WHY   — Keep bin thin; organize HTTP, policy, metrics, config, and storage layers cleanly.
//! RO:INTERACTS — http routes, policy verifier seams, storage trait, metrics module, config module.
//! RO:INVARIANTS — forbid unsafe; CAS remains BLAKE3 b3:<hex>; paid writes fail closed.
//! RO:METRICS — exposes local metrics module when the metrics feature is enabled.
//! RO:CONFIG — exposes paid-write verifier mode and storage runtime config.
//! RO:SECURITY — policy module owns paid-write verifier seams; no ambient write authority.
//! RO:TEST — cargo clippy -p svc-storage --all-targets; cargo test -p svc-storage --all-targets.

#![forbid(unsafe_code)]

pub mod config;
pub mod errors;
#[cfg(feature = "metrics")]
pub mod metrics;
pub mod policy;
pub mod readiness;
pub mod storage;
pub mod types;
pub mod version;

pub mod http {
    pub mod error;
    pub mod extractors;
    pub mod middleware;
    pub mod routes {
        pub mod get_object;
        pub mod head_object;
        pub mod health;
        pub mod metrics;
        pub mod paid_object;
        pub mod post_object;
        pub mod put_object;
        pub mod ready;
        pub mod version;
    }
    pub mod server;
}
