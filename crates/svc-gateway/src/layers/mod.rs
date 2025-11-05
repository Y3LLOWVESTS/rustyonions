//! Order (inner→outer): timeouts → `body_caps` → `decode_guard` → `rate_limit` → drr → tarpit → auth → corr

pub mod body_caps;
pub mod concurrency;
pub mod corr;
pub mod decode_guard;
pub mod rate_limit;
pub mod timeouts;
