//! RO:WHAT   Fast structural/limit checks executed before MAC work.
//! RO:WHY    Shed bad requests early; keep hot path predictable.
//! RO:INVARIANTS Pure; constant-time unrelated to secrets.

use crate::types::VerifierConfig;

#[allow(dead_code)]
#[inline]
pub fn check_size_cap(cfg: &VerifierConfig, token_len: usize) -> Result<(), &'static str> {
    if token_len > cfg.max_token_bytes {
        return Err("cap: token too large");
    }
    Ok(())
}

#[allow(dead_code)]
#[inline]
pub fn check_caveat_count(cfg: &VerifierConfig, count: usize) -> Result<(), &'static str> {
    if count > cfg.max_caveats {
        return Err("cap: too many caveats");
    }
    Ok(())
}
