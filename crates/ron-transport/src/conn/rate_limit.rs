//! RO:WHAT — Placeholder for per-conn rate limiting (tokens).
#[derive(Clone, Default)]
pub struct RateLimit;
impl RateLimit {
    pub fn allow(&self, _bytes: usize) -> bool {
        true
    }
}
