//! RO:WHAT — Lightweight, DTO-level validation helpers (no I/O).
//! RO:WHY  — Central place to enforce protocol invariants shared by hosts.
//! RO:INTERACTS — oap::{hello,start,data,end}, version::PROTO_VERSION, error::{ProtoError,Kind}.
//! RO:INVARIANTS — Pure functions; return `ProtoError` with stable reason strings.

use crate::error::{Kind, ProtoError};

/// Validate that the peer speaks our protocol/version.
pub fn validate_hello(h: &crate::oap::hello::Hello) -> Result<(), ProtoError> {
    if h.protocol != "OAP/1" {
        return Err(ProtoError {
            kind: Kind::ProtoMismatch,
            message: format!("unsupported protocol '{}'", h.protocol),
        });
    }
    if h.version != crate::version::PROTO_VERSION {
        return Err(ProtoError {
            kind: Kind::ProtoMismatch,
            message: format!("version {} != {}", h.version, crate::version::PROTO_VERSION),
        });
    }
    Ok(())
}

/// Validate START frame limits against the OAP cap.
pub fn validate_start(s: &crate::oap::start::Start) -> Result<(), ProtoError> {
    if (s.max_frame_bytes as usize) > crate::oap::MAX_FRAME_BYTES {
        return Err(ProtoError {
            kind: Kind::TooLarge,
            message: format!(
                "max_frame_bytes={} exceeds cap {}",
                s.max_frame_bytes,
                crate::oap::MAX_FRAME_BYTES
            ),
        });
    }
    Ok(())
}

/// Validate a DATA frame's payload size against a negotiated bound.
///
/// `negotiated_max` should come from `Start.max_frame_bytes` (after `validate_start`).
pub fn validate_data(
    d: &crate::oap::data::Data,
    negotiated_max: u32,
) -> Result<(), ProtoError> {
    let len = d.bytes.len() as u32;
    if len > negotiated_max {
        return Err(ProtoError {
            kind: Kind::TooLarge,
            message: format!("data bytes={} > negotiated_max={}", len, negotiated_max),
        });
    }
    Ok(())
}

/// Validate monotonic sequence progression (host streams can opt-in).
pub fn validate_seq_progress(prev: u64, next: u64) -> Result<(), ProtoError> {
    if next <= prev {
        return Err(ProtoError {
            kind: Kind::BadRequest,
            message: format!("non-monotonic seq: next={} <= prev={}", next, prev),
        });
    }
    Ok(())
}

// Re-export trait sugar for callers who prefer impl-based validation.
pub mod validate;
pub use validate::{Validate, Limits}; // <— re-export both so users can `use ron_proto::{Validate, Limits};`
