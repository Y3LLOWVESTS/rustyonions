//! RO:WHAT — Central HTTP/OAP limit constants (foundation only).
//! RO:WHY  — Hardening blueprint: size/time/concurrency must be explicit.
//! RO:INVARIANTS — OAP max_frame=1MiB; decoded body cap defaults to 1MiB.

pub const OAP_MAX_FRAME_BYTES: usize = 1_048_576; // 1 MiB
pub const HTTP_BODY_CAP_BYTES: usize = 1_048_576; // 1 MiB
