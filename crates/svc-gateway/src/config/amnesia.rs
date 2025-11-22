//! Amnesia-mode toggles (RAM-only, extra redaction).
//! Operator doc references.

/// Whether amnesia mode is enabled for this process.
///
/// In this slice this is a simple stub wired via config defaults/env later.
#[inline]
#[must_use]
pub fn amnesia_enabled() -> bool {
    false
}
