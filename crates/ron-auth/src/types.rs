//! RO:WHAT  Public types: Capability, Scope, Caveat, Decision, VerifierConfig, RequestCtx.
//! RO:WHY   Stable, boring DTOs; serde/CBOR friendly; no alloc surprises.
//! RO:INVARIANTS Deterministic encoding; strict bounds; no I/O.

use crate::prelude::*;
use std::net::IpAddr;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Scope {
    /// Optional resource prefix (e.g., "/index/").
    pub prefix: Option<String>,
    /// Allowed HTTP-style methods (e.g., "GET","PUT").
    pub methods: Vec<String>,
    /// Max payload bytes permitted by this capability.
    pub max_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "t", content = "v")]
pub enum Caveat {
    Exp(u64),
    Nbf(u64),
    Aud(String),
    Method(Vec<String>),
    PathPrefix(String),
    IpCidr(String),
    BytesLe(u64),
    Rate {
        per_s: u32,
        burst: u32,
    },
    Tenant(String),
    Amnesia(bool),
    GovPolicyDigest(String),
    Custom {
        ns: String,
        name: String,
        cbor: serde_cbor::Value,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Capability {
    /// Tenant/domain namespace for multi-tenant safety.
    pub tid: String,
    /// Key identifier for MAC lookup.
    pub kid: String,
    pub scope: Scope,
    pub caveats: Vec<Caveat>,
    /// Final MAC (BLAKE3 keyed), 32 bytes.
    #[serde(with = "serde_bytes")]
    pub mac: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerifierConfig {
    /// Upper bound after Base64URL decode.
    pub max_token_bytes: usize,
    /// Max allowed caveats.
    pub max_caveats: usize,
    /// Clock skew in seconds for exp/nbf.
    pub clock_skew_secs: i64,
}

impl VerifierConfig {
    pub fn with_defaults() -> Self {
        Self {
            max_token_bytes: 4096,
            max_caveats: 64,
            clock_skew_secs: 60,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RequestCtx {
    pub now_unix_s: u64,
    pub method: String,
    pub path: String,
    pub peer_ip: Option<IpAddr>,
    pub object_addr: Option<String>,
    pub tenant: String,
    pub amnesia: bool,
    pub policy_digest_hex: Option<String>,
    pub extras: serde_cbor::Value,
}

/// Final decision.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Decision {
    Allow {
        scope: Scope,
    },
    Deny {
        reasons: Vec<crate::errors::DenyReason>,
    },
}

/// Opaque MAC key (32 bytes for BLAKE3 keyed mode).
#[derive(Debug, Clone)]
pub struct MacKey(pub [u8; 32]);

/// Caller-provided keys (e.g., from ron-kms). No I/O here.
pub trait MacKeyProvider {
    fn key_for(&self, kid: &str, tid: &str) -> Option<MacKey>;
}
