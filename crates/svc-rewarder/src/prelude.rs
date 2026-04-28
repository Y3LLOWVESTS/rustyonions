//! RO:WHAT — Small import prelude for svc-rewarder modules.
//! RO:WHY — Pillar 12; Concerns: DX. Reduces repetitive imports without expanding public semantics.
//! RO:INTERACTS — crate errors, tracing, serde.
//! RO:INVARIANTS — no hidden IO; no side effects; no policy decisions.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — none.
//! RO:TEST — compile coverage through all modules.

pub use crate::errors::{Result, RewarderError};
pub use serde::{Deserialize, Serialize};
pub use tracing::{debug, error, info, warn};
