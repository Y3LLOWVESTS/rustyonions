//! Order (inner→outer): timeouts → `body_caps` → `decode_guard` → `rate_limit` → drr → tarpit → auth → corr

pub mod concurrency;
pub mod corr;
pub mod decode_guard;
pub mod timeouts;
pub mod body_caps;
pub mod rate_limit;