//! Produce a quoted `ETag` value for a BLAKE3 address (e.g., `b3:abcd...`).

#[must_use]
pub fn etag_from_b3(addr: &str) -> String {
    // clippy(pedantic): prefer inline args over `"{}"`
    format!("\"{addr}\"")
}
