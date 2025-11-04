//! admission/residency.rs — Thin adapter to `policy::residency` — placeholder.

#[must_use]
pub fn region_ok(_tenant: &str, _region: &str) -> bool {
    true
}
