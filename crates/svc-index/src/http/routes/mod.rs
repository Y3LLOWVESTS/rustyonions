//! RO:WHAT — HTTP route modules for svc-index.
//! RO:WHY — Keep route ownership explicit and small.
//! RO:INVARIANTS — routes must not store raw bytes or mutate wallet/ledger state.

pub mod admin;
pub mod health;
pub mod index_manifests;
pub mod metrics;
pub mod providers;
pub mod resolve;
pub mod version;
