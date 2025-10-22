//! RO:WHAT — Internal glue (non-public) for kernel constants and helpers.
//! RO:WHY  — Keep public surface frozen; avoid leaking new types.
//! RO:INTERACTS — constants used by bus/supervisor; not re-exported.
pub mod constants;
