//! RO:WHAT — Hot-reload scaffold (listen for kernel ConfigUpdated, apply).
//! RO:WHY  — RON pattern: runtime config changes without restart; Concerns: GOV/RES.
//! RO:INTERACTS — ron-kernel bus events; to be wired in Phase 2.

pub struct Reload; // placeholder for future reload worker
