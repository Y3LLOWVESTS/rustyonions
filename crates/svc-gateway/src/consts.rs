// crates/svc-gateway/src/consts.rs
#![allow(clippy::module_name_repetitions)]

pub const DEFAULT_MAX_CONNS: usize = 2_048;

// 1 MiB (avoid `1 * 1024 * 1024` which trips clippy::identity_op)
pub const DEFAULT_BODY_CAP_BYTES: usize = 1_048_576;

// Decompression guard
pub const DEFAULT_DECODE_RATIO_MAX: usize = 10;
pub const DEFAULT_DECODE_ABS_CAP_BYTES: usize = 16 * 1_048_576; // 16 MiB

// Read timeouts
pub const DEFAULT_READ_TIMEOUT_SECS: u64 = 10;
pub const DEFAULT_WRITE_TIMEOUT_SECS: u64 = 10;
pub const DEFAULT_IDLE_TIMEOUT_SECS: u64 = 30;

// Rate limit (as u64 to match config field)
pub const DEFAULT_RPS: u64 = 500;
