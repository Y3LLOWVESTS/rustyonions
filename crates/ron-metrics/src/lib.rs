//! RO:WHAT — Public façade for ron-metrics: golden families, health/readiness, HTTP exposer.
//! RO:WHY  — Pillar 5 Observability; Concerns: PERF/RES/GOV.
//! RO:INTERACTS — crate::{metrics,registry,labels,health,readiness}, axum; prometheus Registry.
//! RO:INVARIANTS — single registration per process; GET-only; no lock across .await; TLS type=tokio_rustls::rustls::ServerConfig.
//! RO:METRICS — service_restarts_total, bus_lagged_total, request_latency_seconds, exposition_latency_seconds, health_ready{service}.
//! RO:CONFIG — base labels include {service,instance,build_version,amnesia}; amnesia truthful.

#![forbid(unsafe_code)]

pub mod build_info;
mod errors;
mod health;
mod labels;
mod metrics;
mod readiness;
mod registry;
pub mod exposer;
pub mod exporters;
pub mod bus_watcher;
pub mod axum_latency;
pub mod axum_status;




pub use crate::errors::MetricsError;
pub use crate::health::HealthState;
pub use crate::labels::BaseLabels;
pub use crate::metrics::Metrics;
pub use crate::readiness::{ReadyJson, ReadyPolicy};
pub use crate::registry::SafeRegistry;
