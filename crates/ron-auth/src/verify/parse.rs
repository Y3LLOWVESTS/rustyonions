//! RO:WHAT   Token parsing helpers (base64url, CBOR).
//! RO:WHY    Keep low-level parsing separate from pipeline orchestration.
//! RO:INVARIANTS No I/O; deterministic; URL-safe base64 without padding.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use serde::de::DeserializeOwned;

/// Decode URL-safe base64 (no padding) with a conservative early size cap.
/// `max_bytes` is the maximum allowed decoded length (post-base64).
#[allow(dead_code)]
#[inline]
pub fn b64url_decode(bytes_b64url: &str, max_bytes: usize) -> Result<Vec<u8>, &'static str> {
    // Early cap on input chars: ceil(max_bytes * 4 / 3)
    let max_in = max_bytes.saturating_mul(4).div_ceil(3);
    if bytes_b64url.len() > max_in {
        return Err("b64url: input too large");
    }
    URL_SAFE_NO_PAD
        .decode(bytes_b64url.as_bytes())
        .map_err(|_| "b64url: decode error")
}

/// Decode CBOR value from a slice using serde_cbor.
#[allow(dead_code)]
#[inline]
pub fn cbor_from_slice<T: DeserializeOwned>(buf: &[u8]) -> Result<T, &'static str> {
    serde_cbor::from_slice(buf).map_err(|_| "cbor: decode error")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_roundtrip() {
        let payload = b"hello";
        let enc = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload);
        let out = b64url_decode(&enc, 1024).unwrap();
        assert_eq!(out, payload);
    }
}
