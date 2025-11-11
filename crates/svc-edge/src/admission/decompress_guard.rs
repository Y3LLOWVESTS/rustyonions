//! Admission guard: optional transparent decompression (stub).

/// Decompression posture (placeholder).
#[derive(Debug, Clone, Copy)]
pub enum Decompress {
    /// Allow a safe set (e.g., gzip) — details TBD.
    Safe,
    /// Disable all decompression.
    Off,
}

impl Default for Decompress {
    fn default() -> Self {
        Decompress::Off
    }
}
