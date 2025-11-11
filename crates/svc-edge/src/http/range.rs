//! HTTP range parsing primitives (single-range only) — stub.

/// Single byte range (inclusive start, inclusive end).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByteRange {
    /// Start offset.
    pub start: u64,
    /// End offset (inclusive).
    pub end: u64,
}

impl ByteRange {
    /// Length of the range in bytes, saturating.
    pub fn len(&self) -> u64 {
        self.end.saturating_sub(self.start) + 1
    }
}
