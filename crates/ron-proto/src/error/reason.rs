//! RO:WHAT — Stable mapping from `Kind` to metric reason strings.
//! RO:WHY  — Metric labels must be immutable across versions.

use super::Kind;

pub fn stable_reason(k: Kind) -> &'static str {
    match k {
        Kind::BadRequest => "bad_request",
        Kind::Unauthorized => "unauthorized",
        Kind::Forbidden => "forbidden",
        Kind::NotFound => "not_found",
        Kind::Conflict => "conflict",
        Kind::TooLarge => "too_large",
        Kind::RateLimited => "rate_limited",
        Kind::Internal => "internal",
        Kind::Unavailable => "unavailable",
        Kind::ProtoMismatch => "proto_mismatch",
    }
}
