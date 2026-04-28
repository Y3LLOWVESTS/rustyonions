//! RO:WHAT — Configuration module facade for svc-rewarder.
//! RO:WHY — Pillar 12; Concerns: RES/DX/GOV. Provides one stable config entry point.
//! RO:INTERACTS — main, tests, readiness, telemetry.
//! RO:INVARIANTS — effective config is validated before use; amnesia explicit.
//! RO:METRICS — none directly.
//! RO:CONFIG — owns types, load, validate.
//! RO:SECURITY — TLS and macaroon paths are config only; no secret logging.
//! RO:TEST — config unit tests.

pub mod load;
pub mod types;
pub mod validate;

pub use load::{load_config_file, load_config_from_env};
pub use types::*;
pub use validate::validate_config;
