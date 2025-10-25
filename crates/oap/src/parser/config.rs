//! RO:WHAT — Parser configuration (buffer policy hooks).
//! RO:WHY  — Centralize tunables; allow callers to cap parser memory if desired.
//! RO:INTERACTS — Used by `ParserState`.
//! RO:INVARIANTS — Defaults are conservative and safe.

/// Parser configuration.
/// Currently only exposes a soft maximum buffer size; the OAP decoder still
/// enforces per-frame bounds independently.
#[derive(Clone, Copy, Debug)]
pub struct ParserConfig {
    /// Soft cap for buffered bytes. `None` disables the soft check.
    pub max_buffer_bytes: Option<usize>,
}

impl Default for ParserConfig {
    fn default() -> Self {
        // 4 MiB soft buffer cap is usually sufficient for a few frames in flight.
        Self {
            max_buffer_bytes: Some(4 * 1024 * 1024),
        }
    }
}
