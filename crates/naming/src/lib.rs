#![forbid(unsafe_code)]

use anyhow::{anyhow, Result};
use std::fmt;
use std::str::FromStr;
pub mod manifest;

/// Canonical RustyOnions address (BLAKE3-only):
/// - Accepts "b3:<hex>.tld" **or** "<hex>.tld" on input
/// - `Display` renders canonical form **with** "b3:" prefix
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Address {
    /// 64-hex BLAKE3 digest (lowercase)
    pub hex: String,
    /// TLD like "image", "video", "post"
    pub tld: String,
    /// Whether the original string included an explicit "b3:" prefix
    pub explicit_b3: bool,
}

impl Address {
    /// Parse "b3:<hex>.tld" or "<hex>.tld" (treated as BLAKE3).
    pub fn parse(s: &str) -> Result<Self> {
        let (left, tld) = s
            .rsplit_once('.')
            .ok_or_else(|| anyhow!("missing .tld in address"))?;

        let (explicit_b3, hex) = if let Some((algo, hex)) = left.split_once(':') {
            let algo = algo.to_ascii_lowercase();
            if algo != "b3" && algo != "blake3" {
                return Err(anyhow!("unsupported algo: {algo} (only b3/blake3 allowed)"));
            }
            (true, hex)
        } else {
            (false, left)
        };

        if hex.len() != 64 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(anyhow!("invalid digest: expected 64 hex chars"));
        }

        Ok(Address {
            hex: hex.to_ascii_lowercase(),
            tld: tld.to_string(),
            explicit_b3,
        })
    }

    /// Render with or without "b3:" prefix.
    pub fn to_string_with_prefix(&self, explicit: bool) -> String {
        if explicit {
            self.canonical_string()
        } else {
            self.to_bare_string()
        }
    }

    /// "<hex>.tld"
    pub fn to_bare_string(&self) -> String {
        format!("{}.{}", self.hex, self.tld)
    }

    /// "b3:<hex>.tld"
    pub fn canonical_string(&self) -> String {
        format!("b3:{}.{}", self.hex, self.tld)
    }
}

/// Make `addr.to_string()` work, returning the **canonical** "b3:<hex>.tld".
impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("b3:")?;
        f.write_str(&self.hex)?;
        f.write_str(".")?;
        f.write_str(&self.tld)
    }
}

/// Allow `"<addr>".parse::<Address>()`
impl FromStr for Address {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        Address::parse(s)
    }
}
