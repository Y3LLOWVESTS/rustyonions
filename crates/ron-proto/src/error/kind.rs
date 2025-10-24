//! RO:WHAT — Enumerated error kinds used across DTOs.
//! RO:WHY  — Stable, additive error space for interop & metrics.

use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Kind {
    BadRequest,
    Unauthorized,
    Forbidden,
    NotFound,
    Conflict,
    TooLarge,
    RateLimited,
    Internal,
    Unavailable,
    ProtoMismatch,
}
