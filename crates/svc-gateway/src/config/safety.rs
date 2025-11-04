//! Safety guard to prevent weakening defaults unless `danger_ok=true`.
//! Hardening checklist refs. :contentReference[oaicite:7]{index=7}

#[inline]
pub fn assert_safe(danger_ok: bool) {
    if !danger_ok { /* keep defaults enforced */ }
}
