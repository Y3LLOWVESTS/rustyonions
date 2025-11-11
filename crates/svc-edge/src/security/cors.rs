//! CORS policy placeholder (stub).

/// CORS posture (placeholder).
#[derive(Debug, Clone, Copy)]
pub enum Cors {
    /// Deny all cross-origin requests.
    Deny,
    /// Allow a restricted set (details TBD).
    Restricted,
}

impl Default for Cors {
    fn default() -> Self {
        Cors::Deny
    }
}
