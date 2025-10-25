//! RO:WHAT — Status helpers and canonical outcome→label mapping for metrics.
//! RO:WHY  — Keep vocabulary consistent across services; no direct prometheus deps here.
//! RO:INTERACTS — Callers map these to ron-metrics counters/histograms.
//! RO:INVARIANTS — Stable label triplets; conservative defaults.

use crate::error::{OapDecodeError, StatusCode};

/// Human-friendly reason text for a status code (stable subset).
pub fn reason(code: StatusCode) -> &'static str {
    match code {
        StatusCode::Ok => "OK",
        StatusCode::Partial => "Partial Content",
        StatusCode::BadRequest => "Bad Request",
        StatusCode::Unauthorized => "Unauthorized",
        StatusCode::Forbidden => "Forbidden",
        StatusCode::NotFound => "Not Found",
        StatusCode::PayloadTooLarge => "Payload Too Large",
        StatusCode::TooManyRequests => "Too Many Requests",
        StatusCode::Internal => "Internal Server Error",
        StatusCode::Unavailable => "Service Unavailable",
    }
}

/// Quick predicates.
pub fn is_success(code: StatusCode) -> bool { (code as u16) / 100 == 2 }
pub fn is_client_err(code: StatusCode) -> bool { (code as u16) / 100 == 4 }
pub fn is_server_err(code: StatusCode) -> bool { (code as u16) / 100 == 5 }

/// Outcome class for accounting.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum OutcomeClass {
    Success,
    ClientError,
    ServerError,
    DecodeError,
    Oversize,
}

/// Convert a status code into a high-level outcome class.
pub fn outcome_from_status(code: StatusCode) -> OutcomeClass {
    match (code as u16) / 100 {
        2 => OutcomeClass::Success,
        4 => OutcomeClass::ClientError,
        5 => OutcomeClass::ServerError,
        _ => OutcomeClass::ClientError, // default conservative
    }
}

/// Map a decode error to an outcome class (for ingress parse failures).
pub fn outcome_from_decode(err: &OapDecodeError) -> OutcomeClass {
    use OapDecodeError::*;
    match err {
        FrameTooLarge { .. } => OutcomeClass::Oversize,
        BadVersion(_) | BadFlags(_) | CapOnNonStart | CapOutOfBounds | PayloadOutOfBounds => {
            OutcomeClass::DecodeError
        }
        TruncatedHeader | DecompressBoundExceeded | ZstdFeatureNotEnabled | Zstd(_) | Io(_) => {
            OutcomeClass::DecodeError
        }
    }
}

/// Convert outcome into stable label triplet (kind, cause, detail).
/// Keep these short and low-cardinality.
pub fn labels_for_outcome(outcome: OutcomeClass) -> (&'static str, &'static str, &'static str) {
    match outcome {
        OutcomeClass::Success => ("oap", "ok", "2xx"),
        OutcomeClass::ClientError => ("oap", "client", "4xx"),
        OutcomeClass::ServerError => ("oap", "server", "5xx"),
        OutcomeClass::DecodeError => ("oap", "decode", "error"),
        OutcomeClass::Oversize => ("oap", "oversize", "413"),
    }
}
