//! Metrics boot helpers.
//! RO:WHAT   (Placeholder) Hooks to pre-warm metric label sets at startup.
//! RO:WHY    Avoid first-hit allocations when a route is hit under burst.
//! RO:PLAN   Once `observability::http_metrics` exposes a `prewarm_labels(...)`
//!           we call it here for the known {route,method,status} tuples.
//! RO:SAFE   Currently a no-op to avoid any registration conflicts.

/// Pre-warm metric label sets (currently a no-op).
///
/// # Notes
/// - Intentionally empty until `http_metrics` exposes a safe prewarm function.
/// - Kept as a separate module so wiring it later is a one-line change.
pub fn prewarm() {
    // no-op (will call into http_metrics once the prewarm function is exposed)
}
