//! RO:WHAT — Edge plane helpers.
//! RO:WHY  — Give applications a small, ergonomic facade over the edge
//!           HTTP/OAP routes exposed by svc-edge (I-2).
//! RO:INTERACTS — Uses the shared transport handle and metrics facade.
//! RO:INVARIANTS —
//!   - Never panic; surface failures as `SdkError`.
//!   - Respect ByteRange semantics (inclusive bounds).
//!   - Keep this layer "dumb": no policy, just DTO/wire mapping.

use std::time::Duration;

use bytes::Bytes;

use crate::{
    errors::SdkError,
    metrics::SdkMetrics,
    transport::TransportHandle,
    types::{ByteRange, Capability},
};

/// Logical facade for `GET /edge/{path}` with optional byte-range.
///
/// This is intentionally a *thin* wrapper; most of the interesting work
/// (capability verification, OAP envelope building, retries, etc.) happens
/// in the transport layer and gateway services.
///
/// For now, this helper focuses on:
///   - enforcing local invariants we can check without I/O (byte-range),
///   - shaping the call surface so applications have a stable API.
///
/// The actual transport wiring will come in a later step.
///
/// # Errors
///
/// * `SdkError::OapViolation{ reason: "range" }` when the supplied
///   `ByteRange` is malformed (end < start).
/// * `SdkError::Unknown(..)` as a placeholder until the transport
///   integration is wired. This avoids `todo!()` panics while still
///   making it obvious that the edge plane is not live yet.
pub async fn edge_get(
    transport: &TransportHandle,
    metrics: &dyn SdkMetrics,
    cap: Capability,
    path: &str,
    range: Option<ByteRange>,
    deadline: Duration,
) -> Result<Bytes, SdkError> {
    // --- Local invariants we *can* enforce here ---

    if let Some(ref r) = range {
        validate_range(r)?;
        // Precompute the Range header value we will eventually attach
        // once the HTTP/OAP transport is wired. For now this is kept
        // only to exercise the mapping in tests and to avoid unused
        // argument warnings.
        let _range_header = format_range_header(r);
        let _len = r.len(); // keeps edge logic mindful of inclusive semantics
    }

    // For now we only enforce shape/validation and return a typed
    // "not wired" error instead of panicking. This keeps the SDK
    // safe to link even before the edge plane is fully implemented.
    let _ = (transport, metrics, cap, path, deadline);

    Err(SdkError::Unknown(
        "edge_get not wired to transport yet".to_string(),
    ))
}

/// Validate a `ByteRange` for use as an HTTP `Range: bytes=…` header.
///
/// We currently enforce only a minimal invariant:
///   * `end >= start`
///
/// Length and upper bounds (e.g., 1 MiB) are enforced at the OAP/transport
/// level where we know the concrete frame limits.
///
/// # Errors
///
/// Returns `SdkError::OapViolation { reason: "range" }` if the range
/// is malformed.
fn validate_range(range: &ByteRange) -> Result<(), SdkError> {
    if range.end < range.start {
        return Err(SdkError::OapViolation { reason: "range" });
    }
    Ok(())
}

/// Format a `ByteRange` as an HTTP `Range` header value.
///
/// Example:
///
/// ```text
/// bytes=0-65535
/// ```
fn format_range_header(range: &ByteRange) -> String {
    format!("bytes={}-{}", range.start, range.end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_range_validation_accepts_well_formed_range() {
        let r = ByteRange { start: 0, end: 9 };
        assert!(validate_range(&r).is_ok());
    }

    #[test]
    fn byte_range_validation_rejects_inverted_range() {
        let r = ByteRange { start: 10, end: 5 };
        let err = validate_range(&r).unwrap_err();

        match err {
            SdkError::OapViolation { reason } => {
                assert_eq!(reason, "range");
            }
            other => panic!("unexpected error variant: {other:?}"),
        }
    }

    #[test]
    fn byte_range_header_format_is_inclusive() {
        let r = ByteRange {
            start: 0,
            end: 65_535,
        };
        let header = format_range_header(&r);
        assert_eq!(header, "bytes=0-65535");
    }
}
