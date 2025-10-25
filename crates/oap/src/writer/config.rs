//! RO:WHAT — Writer configuration.
//! RO:WHY  — Centralize tunables for buffered writes.
//! RO:INTERACTS — Used by `OapWriter`.
//! RO:INVARIANTS — Defaults tuned to typical MTU/page sizes.

use crate::constants::STREAM_CHUNK_SIZE;

/// Writer configuration.
#[derive(Clone, Copy, Debug)]
pub struct WriterConfig {
    /// When internal buffer reaches this size, `write_frame` will flush it automatically.
    pub flush_hint_bytes: usize,
}

impl Default for WriterConfig {
    fn default() -> Self {
        // Use the 64 KiB stream chunk as a reasonable flush hint.
        Self { flush_hint_bytes: STREAM_CHUNK_SIZE }
    }
}
