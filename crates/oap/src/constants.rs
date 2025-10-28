//! RO:WHAT — Canonical OAP/1 constants and limits.
//! RO:WHY — Keep protocol bounds single-sourced and drift-proof (Hardening & Interop blueprints).
//! RO:INTERACTS — Used by header/frame/codec modules and by callers for guardrails.
//! RO:INVARIANTS — max_frame=1MiB; bounded decompress guard ≤ 8× frame cap; chunk hint=64KiB.

/// Maximum allowed OAP frame size in bytes (protocol invariant).
pub const MAX_FRAME_BYTES: u32 = 1024 * 1024; // 1 MiB

/// Recommended streaming chunk size used by storage paths (not a protocol limit).
pub const STREAM_CHUNK_SIZE: usize = 64 * 1024; // 64 KiB

/// Upper bound on decompressed size relative to `MAX_FRAME_BYTES` when COMP flag is set.
/// Per Interop blueprint: bounded inflate (≤ 8× frame cap) → reject with 413 if exceeded.
pub const MAX_DECOMPRESS_EXPANSION: u32 = 8;

/// OAP protocol version.
pub const OAP_VERSION: u16 = 1;
