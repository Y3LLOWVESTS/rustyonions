//! RO:WHAT — Route module fanout for svc-storage.
//! RO:WHY — Keeps object, paid-object, observability, and version handlers discoverable.
//! RO:INTERACTS — http::server, route handlers under src/http/routes.
//! RO:INVARIANTS — free CAS path remains available; paid path must require proof headers.
//! RO:METRICS — individual handlers own metrics when wired.
//! RO:CONFIG — no direct config reads.
//! RO:SECURITY — route modules define write admission behavior.
//! RO:TEST — tests/http_blackbox.rs and tests/web3_paid_storage_loop.rs.

pub mod get_object;
pub mod head_object;
pub mod health;
pub mod metrics;
pub mod paid_object;
pub mod post_object;
pub mod put_object;
pub mod ready;
pub mod version;