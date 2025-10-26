//! RO:WHAT — Hard transport limits aligned to OAP/1 & Hardening v2.
//! RO:WHY  — Prevent DoS/compression bombs; deterministic errors.
//! RO:INTERACTS — conn::{reader,writer}, reason::RejectReason.

/// OAP/1 protocol frame max (bytes).
pub const MAX_FRAME_BYTES: usize = 1 * 1024 * 1024; // 1 MiB

/// Typical streaming chunk size (~storage path guidance).
pub const STREAM_CHUNK_BYTES: usize = 64 * 1024; // 64 KiB

/// Maximum decompressed size multiplier (defense-in-depth).
pub const MAX_DECOMP_RATIO: u32 = 10;

/// Inflight per-connection frame bound (defensive default).
pub const MAX_INFLIGHT_FRAMES: usize = 64;
