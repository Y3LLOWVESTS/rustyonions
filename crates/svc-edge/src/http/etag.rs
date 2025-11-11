//! ETag helpers (opaque ETag strings) — stub.

/// Strong ETag wrapper (opaque).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ETag(pub String);

impl ETag {
    /// Format as a strong ETag: `"value"`.
    pub fn strong(value: impl AsRef<str>) -> Self {
        Self(format!("\"{}\"", value.as_ref()))
    }
}
