//! RO:WHAT — High-level Name/Content addressing grammar.
//! RO:WHY  — Single, portable enum to represent user-facing addresses.
//! RO:INTERACTS — types::{Fqdn, ContentId}, version::NameVersion, normalize.
//! RO:INVARIANTS — Content ids are "b3:<hex>"; names are normalized ASCII; optional "@<semver>" suffix for versions.
//! RO:TEST — tests/address_hygiene.rs

use crate::normalize::{normalize_fqdn_ascii, NormalizedFqdn};
use crate::types::{ContentId, Fqdn};
use crate::version::{parse_version, NameVersion};
use serde::{Deserialize, Serialize};

/// A user-facing address: either content-id (b3:...) or a (name[@version]) tuple.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Address {
    /// Canonical content-address (BLAKE3-256).
    ///
    /// The `id` must be of the form `"b3:<64 hex>"`, all lowercase.
    Content {
        /// Canonical content identifier (`"b3:<hex>"`).
        id: ContentId,
    },
    /// Named address with optional semantic version.
    Name {
        /// Normalized ASCII FQDN (no trailing dot).
        fqdn: Fqdn,
        /// Optional semantic version tagged to the name (e.g., `1.2.3`).
        version: Option<NameVersion>,
    },
}

impl Address {
    /// Parse from a user string: either `b3:<hex>` or `name[@semver]`.
    pub fn parse(s: &str) -> Result<Self, ParseAddressError> {
        let s = s.trim();
        if s.starts_with("b3:") {
            let id = ContentId(s.to_owned());
            if !id.validate() {
                return Err(ParseAddressError::InvalidContentId);
            }
            return Ok(Address::Content { id });
        }
        // version suffix: name@1.2.3 (optional)
        let (name_part, ver_opt) = match s.rsplit_once('@') {
            Some((left, right)) if !right.is_empty() && left.contains('.') => (left, Some(right)),
            _ => (s, None),
        };
        let NormalizedFqdn(Fqdn(name)) =
            normalize_fqdn_ascii(name_part).map_err(|_| ParseAddressError::InvalidName)?;
        let fqdn = Fqdn(name);
        let version = match ver_opt {
            Some(vs) => Some(parse_version(vs).map_err(|_| ParseAddressError::InvalidVersion)?),
            None => None,
        };
        Ok(Address::Name { fqdn, version })
    }

    /// Render compact string form: `b3:<hex>` or `name[@ver]`.
    pub fn to_compact(&self) -> String {
        match self {
            Address::Content { id } => id.0.clone(),
            Address::Name { fqdn, version } => match version {
                Some(v) => format!("{}@{}", fqdn.0, v),
                None => fqdn.0.clone(),
            },
        }
    }
}

/// Parse errors for [`Address::parse`].
#[derive(thiserror::Error, Debug)]
pub enum ParseAddressError {
    /// The content id is not a valid `"b3:<64 hex>"` string.
    #[error("invalid content id")]
    InvalidContentId,
    /// The provided name failed IDNA/ASCII hygiene.
    #[error("invalid name")]
    InvalidName,
    /// The version part is not valid semantic versioning.
    #[error("invalid version")]
    InvalidVersion,
}
