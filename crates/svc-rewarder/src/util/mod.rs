//! RO:WHAT — Utility module tree for svc-rewarder.
//! RO:WHY — Pillar 12; Concerns: RES/PERF. Shared helpers stay small and testable.
//! RO:INTERACTS — config, http, inputs, outputs.
//! RO:INVARIANTS — helpers are pure unless explicitly documented; checked arithmetic.
//! RO:METRICS — none directly.
//! RO:CONFIG — bytes and timeout parsers support config validation.
//! RO:SECURITY — malformed inputs fail closed.
//! RO:TEST — config and unit tests exercise parsers.

pub mod bytes;
pub mod timeouts;
