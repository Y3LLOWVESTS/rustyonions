#![forbid(unsafe_code)]

use blake3::Hasher;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

/// Compute BLAKE3 (256-bit) hex digest of a byte slice (64 hex chars).
pub fn b3_hex(bytes: &[u8]) -> String {
    let mut h = Hasher::new();
    h.update(bytes);
    h.finalize().to_hex().to_string()
}

/// Stream a file through BLAKE3 and return 64-hex. Uses a 1MiB buffer.
pub fn b3_hex_file(path: &Path) -> io::Result<String> {
    const BUF: usize = 1 << 20;
    let mut f = File::open(path)?;
    let mut h = Hasher::new();
    let mut buf = vec![0u8; BUF];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(h.finalize().to_hex().to_string())
}

/// Format "b3:<hex>.{tld}" (explicit) or "<hex>.{tld}" (bare).
pub fn format_addr(hex64: &str, tld: &str, explicit_algo_prefix: bool) -> String {
    if explicit_algo_prefix {
        format!("b3:{}.{}", hex64, tld)
    } else {
        format!("{}.{}", hex64, tld)
    }
}

/// Parse "b3:<hex>.tld" or "<hex>.tld" (treated as BLAKE3). Returns (hex, tld).
pub fn parse_addr(addr: &str) -> Option<(String, String)> {
    let (left, tld) = addr.rsplit_once('.')?;
    let hex = if let Some((algo, hex)) = left.split_once(':') {
        let algo = algo.to_ascii_lowercase();
        if algo != "b3" && algo != "blake3" {
            return None;
        }
        hex
    } else {
        left
    };
    if hex.len() != 64 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    Some((hex.to_ascii_lowercase(), tld.to_string()))
}

/// Two-hex shard folder, e.g. "ad" for "ad…64".
pub fn shard2(hex64: &str) -> &str {
    &hex64[..2]
}
