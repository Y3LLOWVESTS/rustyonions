use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Alg {
    Ed25519,
    // Future: MlDsa, SlhDsa, X25519, MlKem...
}

impl Alg {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Alg::Ed25519 => "ed25519",
        }
    }
}

impl fmt::Display for Alg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Alg::Ed25519 => write!(f, "Ed25519"),
        }
    }
}

impl FromStr for Alg {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Ed25519" | "ed25519" => Ok(Self::Ed25519),
            _ => Err("unknown alg"),
        }
    }
}

/// Versioned key identifier: `<tenant>/<purpose>/<alg>/<uuid>#vN`
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyId {
    pub tenant: String,
    pub purpose: String,
    pub alg: Alg,
    pub uuid: Uuid,
    pub version: u32,
}

impl KeyId {
    #[must_use]
    pub fn new(tenant: impl Into<String>, purpose: impl Into<String>, alg: Alg) -> Self {
        Self {
            tenant: tenant.into(),
            purpose: purpose.into(),
            alg,
            uuid: Uuid::new_v4(),
            version: 1,
        }
    }

    #[must_use]
    pub fn bump(&self) -> Self {
        let mut k = self.clone();
        k.version += 1;
        k
    }

    #[must_use]
    pub fn now_utc_ms() -> i128 {
        OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000
    }
}

impl fmt::Display for KeyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}/{}/{}/{}#v{}",
            self.tenant, self.purpose, self.alg, self.uuid, self.version
        )
    }
}

impl FromStr for KeyId {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (left, vpart) = s.rsplit_once("#v").ok_or("missing version")?;
        let version: u32 = vpart.parse().map_err(|_| "bad version")?;
        let mut it = left.split('/');
        let tenant = it.next().ok_or("missing tenant")?;
        let purpose = it.next().ok_or("missing purpose")?;
        let alg = it.next().ok_or("missing alg")?.parse().map_err(|_| "alg")?;
        let uuid_s = it.next().ok_or("missing uuid")?;
        if it.next().is_some() {
            return Err("too many parts");
        }
        let uuid = uuid::Uuid::parse_str(uuid_s).map_err(|_| "uuid")?;
        Ok(Self {
            tenant: tenant.to_string(),
            purpose: purpose.to_string(),
            alg,
            uuid,
            version,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyMeta {
    pub alg: Alg,
    pub current_version: u32,
    pub versions: Vec<u32>,
    pub created_ms: i128,
}
