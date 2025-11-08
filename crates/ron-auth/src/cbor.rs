//! RO:WHAT  CBOR (de)serialization + Base64URL helpers (no padding).
//! RO:WHY   Keep encoding deterministic; centralized parsing; low alloc.
//! RO:INVARIANTS Deterministic CBOR; URL_SAFE_NO_PAD; strict size checks; buffer reuse.

use crate::{bounds, errors::AuthError, types::Capability};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;

/// Decode a Base64URL (no padding) CBOR-encoded Capability into `scratch` to minimize allocs.
pub fn decode_b64url_cbor_capability_with_buf(
    b64: &str,
    max_bytes: usize,
    scratch: &mut Vec<u8>,
) -> Result<Capability, AuthError> {
    if b64.len() > bounds::max_b64url_chars_for(max_bytes) {
        return Err(AuthError::Bounds);
    }
    scratch.clear();
    URL_SAFE_NO_PAD
        .decode_vec(b64.as_bytes(), scratch)
        .map_err(|_| AuthError::Malformed("base64url"))?;
    if scratch.len() > max_bytes {
        return Err(AuthError::Bounds);
    }
    let cap: Capability =
        serde_cbor::from_slice(scratch).map_err(|_| AuthError::Malformed("cbor"))?;
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
    // serde_cbor::to_writer guarantees canonical ordering for our simple maps/seq usage.
    serde_cbor::to_writer(buf, t).expect("cbor fragment into buf");
}
