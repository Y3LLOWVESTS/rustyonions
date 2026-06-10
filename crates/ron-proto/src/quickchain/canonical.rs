//! RO:WHAT — Phase 0 canonical JSON helpers for QuickChain DTO byte experiments.
//! RO:WHY — ECON/GOV: exact bytes must be proven before roots, signatures, pruning, or anchors exist.
//! RO:INTERACTS — super QuickChain DTOs, serde_json, future root/hash modules outside ron-proto.
//! RO:INVARIANTS — DTO-only; no hashing; no IO; no consensus; no service calls; no wallet/ledger mutation.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — canonical bytes are not proof of settlement; unknown fields must reject before canonicalization.
//! RO:TEST — tests/quickchain_canonical_json.rs.

use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;
use thiserror::Error;

/// Result type for QuickChain canonical JSON helpers.
pub type QuickChainCanonicalResult<T> = Result<T, QuickChainCanonicalError>;

/// Errors from Phase 0 canonical JSON helpers.
///
/// These helpers intentionally do not hash bytes. Hashing/root construction
/// belongs in a later root module once domain separation is frozen.
#[derive(Debug, Error)]
pub enum QuickChainCanonicalError {
    #[error("canonical json serialization failed: {0}")]
    Serialize(serde_json::Error),

    #[error("canonical json deserialization failed: {0}")]
    Deserialize(serde_json::Error),

    #[error("canonical json utf8 conversion failed: {0}")]
    Utf8(std::string::FromUtf8Error),

    #[error("canonical json roundtrip mismatch")]
    RoundtripMismatch,
}

/// Serialize a typed DTO into the Phase 0 canonical JSON byte experiment.
///
/// Phase 0 canonical JSON is intentionally narrow:
///
/// - typed structs only
/// - serde struct field order
/// - minified UTF-8 JSON
/// - no arbitrary JSON maps in hashed DTOs
/// - no hash/root/signature claim
pub fn to_canonical_json_vec<T>(value: &T) -> QuickChainCanonicalResult<Vec<u8>>
where
    T: Serialize,
{
    serde_json::to_vec(value).map_err(QuickChainCanonicalError::Serialize)
}

/// Serialize a typed DTO into the Phase 0 canonical JSON string experiment.
pub fn to_canonical_json_string<T>(value: &T) -> QuickChainCanonicalResult<String>
where
    T: Serialize,
{
    let bytes = to_canonical_json_vec(value)?;
    String::from_utf8(bytes).map_err(QuickChainCanonicalError::Utf8)
}

/// Deserialize a typed DTO from canonical JSON bytes.
///
/// Unknown-field rejection is provided by each DTO's serde attributes.
pub fn from_canonical_json_slice<T>(bytes: &[u8]) -> QuickChainCanonicalResult<T>
where
    T: DeserializeOwned,
{
    serde_json::from_slice(bytes).map_err(QuickChainCanonicalError::Deserialize)
}

/// Validate that a DTO round-trips through the Phase 0 canonical JSON surface.
pub fn validate_canonical_json_roundtrip<T>(value: &T) -> QuickChainCanonicalResult<()>
where
    T: Serialize + DeserializeOwned + PartialEq + Debug,
{
    let bytes = to_canonical_json_vec(value)?;
    let decoded: T = from_canonical_json_slice(&bytes)?;

    if decoded == *value {
        return Ok(());
    }

    Err(QuickChainCanonicalError::RoundtripMismatch)
}
