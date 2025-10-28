//! RO:WHAT — Health/liveness helpers
//! RO:WHY — Truthful health; Concerns: RES/GOV
//! RO:INTERACTS — /healthz
//! RO:INVARIANTS — cheap checks; truth over green
//! RO:TEST — healthz returns 200

use ron_kernel::HealthState;
use std::sync::Arc;

pub type HealthHandles = Arc<HealthState>;
