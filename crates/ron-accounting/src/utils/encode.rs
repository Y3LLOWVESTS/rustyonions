//! RO:WHAT — Canonical JSON encoding wrappers for sealed accounting preimages.
//! RO:WHY — Pillar 12; Concerns: ECON/DX. Provides deterministic digest input until DAG-CBOR lands.
//! RO:INTERACTS — accounting::slice and interop vector generation.
//! RO:INVARIANTS — encoded bytes must be deterministic and ≤ OAP/1 max_frame=1MiB.
//! RO:METRICS — callers increment schema rejection counters.
//! RO:CONFIG — MAX_CANONICAL_BYTES=1MiB.
//! RO:SECURITY — serde structs must deny/avoid unknown field drift at adapter boundaries.
//! RO:TEST — prop: encoding_prop; unit: recording_tests digest checks.

use serde::Serialize;

use crate::errors::{Error, Result};

/// Maximum canonical payload bytes accepted by the core encoder.
pub const MAX_CANONICAL_BYTES: usize = 1_048_576;

/// Serialize a value into deterministic compact JSON bytes and enforce the 1MiB cap.
///
/// Batch 1 uses sorted structs/vectors with JSON as a stable local representation. The
/// interop layer can swap this implementation for DAG-CBOR/MsgPack without changing
/// the `SealedSlice` contract because digests are always over this helper's output.
pub fn to_canonical_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    let bytes = serde_json::to_vec(value).map_err(|err| Error::schema(err.to_string()))?;
    enforce_max_bytes(&bytes)?;
    Ok(bytes)
}

/// Reject encoded payloads above the OAP/1 max-frame budget.
pub fn enforce_max_bytes(bytes: &[u8]) -> Result<()> {
    if bytes.len() > MAX_CANONICAL_BYTES {
        return Err(Error::schema(format!(
            "canonical payload exceeds {} bytes",
            MAX_CANONICAL_BYTES
        )));
    }
    Ok(())
}
