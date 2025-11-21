//! RO:WHAT — Post-quantum (PQ) posture helpers for Macronode.
//! RO:WHY  — Interpret env/config flags into a simple runtime enum that
//!           higher layers (TLS, KMS, gateway/overlay) can inspect once
//!           they grow PQ support.
//!
//! RO:INVARIANTS —
//!   - This module performs *no* cryptographic operations.
//!   - Default posture is `PqPosture::Off` to preserve interop until
//!     operators explicitly opt in.
//!   - Unknown/invalid mode strings never panic; they map to `Off` with
//!     a best-effort warning to stderr.
//!
//! RO:CONFIG —
//!   - Env: `RON_PQ_MODE` (foundation cut)
//!       * "off" (default)  → PqPosture::Off
//!       * "hybrid"         → PqPosture::Hybrid
//!     Additional aliases: "0"/"false"/"" → Off, "1"/"true"/"on" → Hybrid.
//!
//! Future slices can add integration with structured config
//! (`config::schema` and overlays) once PQ is wired downstream.

use std::env;

/// Runtime PQ posture for this macronode process.
///
/// Foundation slice keeps this intentionally small: either PQ is off, or we
/// allow a **hybrid** posture where classical + PQ can both be used by
/// lower layers once they are ready.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PqPosture {
    /// PQ disabled; classical-only handshakes/keys.
    Off,
    /// Hybrid posture (classical + PQ), where downstream planes can
    /// negotiate PQ-enabled edges when peers support it.
    Hybrid,
}

impl PqPosture {
    /// Derive posture from the `RON_PQ_MODE` environment variable.
    ///
    /// This is intentionally forgiving and never panics: unrecognised
    /// values simply fall back to `Off` and emit a best-effort warning.
    #[must_use]
    pub fn from_env() -> Self {
        match env::var("RON_PQ_MODE") {
            Ok(raw) => Self::from_str(raw.trim()),
            Err(_) => PqPosture::Off,
        }
    }

    /// Parse posture from a string, accepting a few convenient aliases.
    ///
    /// Known values (case-insensitive):
    ///   - "off", "0", "false", ""       → Off
    ///   - "hybrid", "on", "1", "true"   → Hybrid
    #[must_use]
    pub fn from_str(raw: &str) => Self {
        let lowered = raw.to_ascii_lowercase();
        match lowered.as_str() {
            "" | "off" | "0" | "false" | "disabled" => PqPosture::Off,
            "hybrid" | "on" | "1" | "true" | "enabled" => PqPosture::Hybrid,
            other => {
                // Foundation cut: we don't have tracing plumbed into this
                // module yet, so log to stderr. Later we can route this
                // through `tracing::warn!` once call-sites exist.
                eprintln!(
                    "[macronode-pq] RON_PQ_MODE={other:?} not recognised; defaulting to Off"
                );
                PqPosture::Off
            }
        }
    }

    /// Convenience boolean for feature gating.
    #[must_use]
    pub const fn is_enabled(self) -> bool {
        !matches!(self, PqPosture::Off)
    }
}

#[cfg(test)]
mod tests {
    use super::PqPosture;

    #[test]
    fn from_str_off_aliases() {
        for v in ["", "off", "OFF", "0", "false", "FALSE", "disabled"] {
            assert_eq!(PqPosture::from_str(v), PqPosture::Off, "value={v:?}");
        }
    }

    #[test]
    fn from_str_hybrid_aliases() {
        for v in ["hybrid", "HYBRID", "1", "on", "ON", "true", "TRUE", "enabled"] {
            assert_eq!(PqPosture::from_str(v), PqPosture::Hybrid, "value={v:?}");
        }
    }

    #[test]
    fn unknown_values_fall_back_to_off() {
        assert_eq!(PqPosture::from_str("weird-mode"), PqPosture::Off);
    }
}
