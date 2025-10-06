/*!
Minimal façade for ron-metrics2.
This crate exposes a tiny HTTP exposer for GET /metrics, /healthz, /readyz.
No implementation code in scaffold — only module layout and docs.
*/
pub mod config;
pub mod metrics;
pub mod registry;
pub mod labels;
pub mod readiness;
pub mod health;
pub mod errors;
pub mod build_info;
pub mod pq;
pub mod zk;
pub mod exposer;
pub mod exporters;

/* Public re-exports go here when implemented:
   pub use metrics::Metrics;
   pub use config::Config;
*/
