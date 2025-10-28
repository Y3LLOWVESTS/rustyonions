//! RO:WHAT — Public crate surface & re-exports for svc-dht (Kademlia service)
//! RO:WHY — P10 Overlay/Transport/Discovery; Concerns: SEC/RES/PERF/GOV
//! RO:INTERACTS — ron-kernel (Bus/Health), ron-transport (I/O), axum (admin), ron-proto (DTOs)
//! RO:INVARIANTS — no lock across .await; single-writer per k-bucket; OAP max_frame=1MiB; chunk≈64KiB
//! RO:METRICS — exposes dht_* histograms/counters; /metrics, /healthz, /readyz
//! RO:CONFIG — svc-dht Config; amnesia honored
//! RO:SECURITY — capability checks occur at ingress/gateway; DHT path rejects oversize/abuse
//! RO:TEST — tests/* integration; loom later for kbucket single-writer

pub mod config;
pub mod errors;
pub mod health;
pub mod metrics;
pub mod readiness;
pub mod tracing;
pub use tracing as ro_tracing;

pub mod bootstrap;
pub mod cache;
pub mod codec;
pub mod peer;
pub mod pipeline;
pub mod provider;
pub mod rpc;
pub mod supervision;
pub mod transport;
pub mod types;

pub use config::Config;
pub use health::HealthHandles;
pub use metrics::DhtMetrics;
pub use provider::Store as ProviderStore;
pub use readiness::ReadyGate;
