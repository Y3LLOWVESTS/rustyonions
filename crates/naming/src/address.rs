use crate::tld::TldType;
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Address {
    /// Lower-case hex hash string (e.g., sha256).
    pub hex: String,
    pub tld: TldType,
}

impl Address {
    pub fn new(hex: impl Into<String>, tld: TldType) -> Self {
        Self { hex: hex.into().to_lowercase(), tld }
    }
    pub fn to_string_addr(&self) -> String {
        format!("{}.{}", self.hex, self.tld)
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_string_addr())
    }
}

#[derive(Debug, Error)]
pub enum AddressParseError {
    #[error("invalid address")]
    Invalid,
    #[error("bad tld: {0}")]
    Tld(#[from] crate::tld::TldParseError),
}

impl FromStr for Address {
    type Err = AddressParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Expect "<hex>.<tld>"
        let (hex, tld_str) = s.rsplit_once('.').ok_or(AddressParseError::Invalid)?;
        if hex.is_empty() { return Err(AddressParseError::Invalid); }
        let tld = tld_str.parse()?;
        Ok(Self::new(hex, tld))
    }
}
