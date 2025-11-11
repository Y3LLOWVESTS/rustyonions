//! HSTS policy placeholder (stub).

/// HSTS posture (placeholder).
#[derive(Debug, Clone, Copy)]
pub enum Hsts {
    /// Disabled.
    Off,
    /// Enabled with defaults (details TBD).
    On,
}

impl Default for Hsts {
    fn default() -> Self {
        Hsts::Off
    }
}
