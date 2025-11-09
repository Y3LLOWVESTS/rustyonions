//! RO:WHAT  CBOR (de)serialization + Base64URL helpers (no padding).
//! RO:WHY   Keep encoding deterministic; centralized parsing; low alloc.
//! RO:INVARIANTS Deterministic CBOR; URL_SAFE_NO_PAD; strict size checks; buffer reuse.

#![allow(clippy::needless_return)]

use crate::{errors::AuthError, types::Capability};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;

#[cfg(feature = "fast-cbor")]
use ciborium::de::from_reader as cbor_from_reader;
#[cfg(not(feature = "fast-cbor"))]
use serde_cbor::from_slice as cbor_from_slice;

#[cfg(feature = "simd-b64")]
use base64_simd::URL_SAFE_NO_PAD as SIMD_URL_SAFE_NO_PAD;

/// Decode a Base64URL (no padding) CBOR-encoded Capability into `scratch` to minimize allocs.
pub fn decode_b64url_cbor_capability_with_buf(
    b64: &str,
    max_bytes: usize,
    scratch: &mut Vec<u8>,
) -> Result<Capability, AuthError> {
    // Decode Base64URL → bytes into `scratch`.
    #[cfg(feature = "simd-b64")]
    {
        // SIMD decode returns a fresh Vec; move it into `scratch` with no extra copy.
        let decoded = SIMD_URL_SAFE_NO_PAD
            .decode_to_vec(b64.as_bytes())
            .map_err(|_| AuthError::Malformed("base64url"))?;
        if decoded.len() > max_bytes {
            return Err(AuthError::Bounds);
        }
        *scratch = decoded;
    }

    #[cfg(not(feature = "simd-b64"))]
    {
        scratch.clear();
        URL_SAFE_NO_PAD
            .decode_vec(b64.as_bytes(), scratch)
            .map_err(|_| AuthError::Malformed("base64url"))?;
        if scratch.len() > max_bytes {
            return Err(AuthError::Bounds);
        }
    }

    // Decode CBOR → Capability (serde-compatible). `fast-cbor` uses a reader form.
    #[cfg(feature = "fast-cbor")]
    let cap: Capability =
        cbor_from_reader(scratch.as_slice()).map_err(|_| AuthError::Malformed("cbor"))?;

    #[cfg(not(feature = "fast-cbor"))]
    let cap: Capability = cbor_from_slice(scratch).map_err(|_| AuthError::Malformed("cbor"))?;

    // Basic MAC sanity: 32 bytes (BLAKE3 keyed).
    if cap.mac.len() != 32 {
        return Err(AuthError::Malformed("mac_len"));
    }
    Ok(cap)
}

/// Backwards-compatible wrapper (allocates) — prefer the `_with_buf` variant on hot paths.
#[allow(dead_code)]
pub fn decode_b64url_cbor_capability(b64: &str, max_bytes: usize) -> Result<Capability, AuthError> {
    let mut tmp = Vec::new();
    decode_b64url_cbor_capability_with_buf(b64, max_bytes, &mut tmp)
}

#[allow(dead_code)]
pub fn encode_b64url_cbor_capability(cap: &Capability) -> String {
    let bytes = serde_cbor::to_vec(cap).expect("capability to cbor");
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Deterministic CBOR fragment encoder for MAC chaining (allocating version).
#[allow(dead_code)]
pub fn cbor_fragment<T: serde::Serialize>(t: &T) -> Vec<u8> {
    serde_cbor::to_vec(t).expect("cbor fragment")
}

/// Deterministic CBOR fragment encoder that writes into `buf` (reused across calls).
pub fn cbor_fragment_into<T: serde::Serialize>(t: &T, buf: &mut Vec<u8>) {
    buf.clear();
    // serde_cbor::to_writer keeps canonical ordering for our simple maps/seq usage.
    serde_cbor::to_writer(buf, t).expect("cbor fragment into buf");
}
