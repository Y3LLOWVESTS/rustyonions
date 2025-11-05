//! RO:WHAT — Observability surfaces: logging and version/health/ready adapters.
//! RO:WHY  — Keep app.rs lean; centralize obs stack.

pub mod health;
pub mod logging;
pub mod ready;
pub mod version;
