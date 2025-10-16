/*!
ron-accounting2 — library scaffold only (no implementation).

Modules:
- errors.rs — error taxonomy
- metrics.rs — metric names/registration
- readiness.rs — readiness keys
- normalize.rs — label normalization contract
- utils/ — small helpers
- accounting/ — labels, recorder, window, slice, rollover
- exporter/ — trait, router, lane, worker, ack_lru
- wal/ — feature-gated persistence (auto-off under Amnesia)
- config/ — schema + validate + load

Replace these placeholders with real code when ready.
*/

pub mod errors;
pub mod metrics;
pub mod readiness;
pub mod normalize;
pub mod utils;
pub mod accounting;
pub mod exporter;
#[cfg(feature = "wal")]
pub mod wal;
pub mod config;
