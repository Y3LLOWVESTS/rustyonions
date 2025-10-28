//! RO:WHAT — Central protocol size/time limits (OAP guidance mirrored)
//! RO:WHY — Hardening; Concerns: SEC
pub const MAX_FRAME_BYTES: usize = 1_048_576; // 1 MiB
pub const CHUNK_BYTES: usize = 64 * 1024; // 64 KiB (storage stream knob)
