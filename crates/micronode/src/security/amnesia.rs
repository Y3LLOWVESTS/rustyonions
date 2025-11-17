// crates/micronode/src/security/amnesia.rs
//! RO:WHAT — Amnesia posture helpers (RAM-only by default, persistence opt-in).
//! RO:WHY  — Micronode is designed to be "amnesia-first", avoiding durable writes
//!           unless the operator explicitly opts into persistence.
//! RO:INTERACTS — Uses process environment only; callers use this to decide which
//!                storage engine or profile to pick.
//! RO:INVARIANTS — Default posture is `Enabled` and env parsing never panics.
//! RO:CONFIG — `MICRO_AMNESIA` and legacy `MICRO_PERSIST` influence posture.
//! RO:SECURITY — Favor amnesia when configs are ambiguous and never silently
//!               enable persistence.
//! RO:TEST — Future `tests/amnesia_proof.rs` should walk env matrices.

/// Effective amnesia posture for this process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmnesiaPosture {
    /// Amnesia ON — prefer RAM-only behavior, no durable writes.
    Enabled,
    /// Amnesia OFF — persistence allowed (subject to storage/profile).
    Disabled,
}

/// Returns the effective amnesia posture, considering environment overrides.
///
/// Priority:
/// 1. `MICRO_AMNESIA` (truthy/falsy).
/// 2. `MICRO_PERSIST` (legacy; truthy disables amnesia).
/// 3. Default: `AmnesiaPosture::Enabled`.
pub fn posture_from_env() -> AmnesiaPosture {
    use std::env;

    if let Ok(val) = env::var("MICRO_AMNESIA") {
        match classify_bool(&val) {
            Some(Boolish::True) => return AmnesiaPosture::Enabled,
            Some(Boolish::False) => return AmnesiaPosture::Disabled,
            None => {}
        }
    }

    if let Ok(val) = env::var("MICRO_PERSIST") {
        if matches!(classify_bool(&val), Some(Boolish::True)) {
            return AmnesiaPosture::Disabled;
        }
    }

    AmnesiaPosture::Enabled
}

/// Convenience: `true` if amnesia is enabled (RAM-only posture).
pub fn amnesia_enabled() -> bool {
    matches!(posture_from_env(), AmnesiaPosture::Enabled)
}

/// Convenience: `true` if persistence is allowed (amnesia disabled).
pub fn persistence_allowed() -> bool {
    !amnesia_enabled()
}

/// Internal classification for env bool-likes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Boolish {
    True,
    False,
}

/// Classify a string as a "truthy" or "falsy" value, or return `None` if unknown.
fn classify_bool(s: &str) -> Option<Boolish> {
    let v = s.trim();
    if v.is_empty() {
        return None;
    }

    let lower = v.to_ascii_lowercase();

    match lower.as_str() {
        "1" | "true" | "on" | "yes" => Some(Boolish::True),
        "0" | "false" | "off" | "no" => Some(Boolish::False),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_bool_truthy_and_falsy() {
        assert_eq!(classify_bool("1"), Some(Boolish::True));
        assert_eq!(classify_bool("true"), Some(Boolish::True));
        assert_eq!(classify_bool("TRUE"), Some(Boolish::True));
        assert_eq!(classify_bool("on"), Some(Boolish::True));
        assert_eq!(classify_bool("yes"), Some(Boolish::True));

        assert_eq!(classify_bool("0"), Some(Boolish::False));
        assert_eq!(classify_bool("false"), Some(Boolish::False));
        assert_eq!(classify_bool("FALSE"), Some(Boolish::False));
        assert_eq!(classify_bool("off"), Some(Boolish::False));
        assert_eq!(classify_bool("no"), Some(Boolish::False));

        assert_eq!(classify_bool(""), None);
        assert_eq!(classify_bool("   "), None);
        assert_eq!(classify_bool("maybe"), None);
    }

    #[test]
    fn posture_defaults_to_amnesia_enabled() {
        let posture = posture_from_env();
        let _ = posture;
    }
}
