#![forbid(unsafe_code)]

use std::fmt;

/// Canonical BLAKE3-256 digest size in bytes and hex length.
pub const B3_LEN: usize = 32;
pub const B3_HEX_LEN: usize = 64;

/// Compute BLAKE3-256 over `bytes`, returning the raw 32-byte array.
#[inline]
pub fn b3(bytes: &[u8]) -> [u8; B3_LEN] {
    blake3::hash(bytes).into()
}

/// Compute the canonical lowercase hex string (64 chars) for BLAKE3-256.
#[inline]
pub fn b3_hex(bytes: &[u8]) -> String {
    let h = blake3::hash(bytes);
    format!("{:x}", h)
}

/// Parse a 64-hex lowercase (or uppercase) BLAKE3 digest into 32 raw bytes.
/// Returns `None` if the string is not exactly 64 hex chars.
pub fn parse_b3_hex<S: AsRef<str>>(s: S) -> Option<[u8; B3_LEN]> {
    let s = s.as_ref();
    if s.len() != B3_HEX_LEN || !s.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let mut out = [0u8; B3_LEN];
    for i in 0..B3_LEN {
        let byte = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok()?;
        out[i] = byte;
    }
    Some(out)
}

/// Render helper for debug prints.
pub struct B3Hex<'a>(pub &'a [u8; B3_LEN]);

impl<'a> fmt::Display for B3Hex<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for b in self.0 {
            write!(f, "{:02x}", b)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_roundtrip() {
        let h = b3(b"hello world");
        let s = B3Hex(&h).to_string();
        assert_eq!(s.len(), B3_HEX_LEN);
        let parsed = parse_b3_hex(&s).unwrap();
        assert_eq!(h, parsed);
    }

    #[test]
    fn rejects_bad_len() {
        assert!(parse_b3_hex("abcd").is_none());
    }

    #[test]
    fn b3_hex_is_lowercase() {
        let s = b3_hex(b"xyz");
        assert!(s.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
        assert_eq!(s.len(), B3_HEX_LEN);
    }
}
