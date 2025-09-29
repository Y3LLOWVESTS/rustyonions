#![forbid(unsafe_code)]

//! ron-proto: wire-level types, signatures, and envelopes shared across services.
//!
//! - Serde DTOs for stable inter-crate/API boundaries.
//! - Optional rmp-serde for compact wire format (`feature = "rmp"`).
//! - Simple, explicit error type.
//!
//! NOTE: This is crypto-agnostic. Signing is done by ron-kms; we only define
//! algorithms and envelope shapes here.

use base64::prelude::*;
use hex::ToHex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

#[derive(Debug, thiserror::Error)]
pub enum ProtoError {
    #[error("serialization error: {0}")]
    Serde(String),
    #[error("deserialization error: {0}")]
    DeSerde(String),
    #[error("unsupported operation: {0}")]
    Unsupported(String),
}

pub type Result<T> = std::result::Result<T, ProtoError>;

/// Supported signing algorithms. Extend as needed.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Algo {
    /// HMAC-SHA256 (symmetric). Great for bootstrap and testing.
    HmacSha256,
    /// Ed25519 (asymmetric). Enable in KMS via the "ed25519" feature.
    Ed25519,
}

/// Stable identifier for a key within the KMS.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct KeyId(pub String);

impl fmt::Display for KeyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Opaque signature bytes (binary, transported as bytes in MessagePack or base64 in JSON).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Signature(#[serde(with = "serde_bytes")] pub Vec<u8>);

impl Signature {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
    pub fn to_base64(&self) -> String {
        BASE64_STANDARD.encode(&self.0)
    }
    pub fn from_bytes(b: Vec<u8>) -> Self {
        Signature(b)
    }
}

/// Compute a hex-encoded SHA256 of arbitrary bytes (used for payload_hash).
pub fn sha256_hex(data: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(data);
    let out = h.finalize();
    out.encode_hex::<String>()
}

/// Generic signed envelope for any payload `T`.
///
/// `payload_hash` is SHA256(payload_bytes) in hex to make signature coverage explicit and
/// enable out-of-band integrity checks.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignedEnvelope<T: Serialize + for<'de> Deserialize<'de>> {
    /// Version for forward-compatibility (bump on breaking changes).
    pub v: u8,
    /// Logical type name (e.g., "Put", "GetReq", "Chunk", "Event").
    pub typ: String,
    /// Algorithm used to produce `sig`.
    pub algo: Algo,
    /// Key identifier used to create `sig`.
    pub kid: KeyId,
    /// Hash of the serialized payload (hex-encoded SHA256 of `payload_bytes`).
    pub payload_hash: String,
    /// Raw signature bytes (algorithm-dependent).
    pub sig: Signature,
    /// The actual payload.
    pub payload: T,
}

impl<T> SignedEnvelope<T>
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    /// Create an unsigned envelope with computed payload_hash; caller fills kid/sig later.
    pub fn unsigned(v: u8, typ: impl Into<String>, algo: Algo, payload: T, payload_bytes: &[u8]) -> Self {
        let payload_hash = sha256_hex(payload_bytes);
        Self {
            v,
            typ: typ.into(),
            algo,
            kid: KeyId(String::new()),
            payload_hash,
            sig: Signature(Vec::new()),
            payload,
        }
    }

    /// Attach signature and key id.
    pub fn with_signature(mut self, kid: KeyId, sig: Signature) -> Self {
        self.kid = kid;
        self.sig = sig;
        self
    }

    /// Verify that the supplied `payload_bytes` matches `payload_hash`.
    pub fn payload_hash_matches(&self, payload_bytes: &[u8]) -> bool {
        self.payload_hash == sha256_hex(payload_bytes)
    }
}

/// Wire helpers — encode/decode via MessagePack (default feature) or JSON.
pub mod wire {
    use super::{ProtoError, Result};
    use serde::{de::DeserializeOwned, Serialize};

    #[cfg(feature = "rmp")]
    pub fn to_msgpack<T: Serialize>(value: &T) -> Result<Vec<u8>> {
        rmp_serde::to_vec_named(value).map_err(|e| ProtoError::Serde(e.to_string()))
    }

    #[cfg(feature = "rmp")]
    pub fn from_msgpack<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
        rmp_serde::from_slice(bytes).map_err(|e| ProtoError::DeSerde(e.to_string()))
    }

    pub fn to_json<T: Serialize>(value: &T) -> Result<String> {
        serde_json::to_string(value).map_err(|e| ProtoError::Serde(e.to_string()))
    }

    pub fn from_json<T: DeserializeOwned>(s: &str) -> Result<T> {
        serde_json::from_str(s).map_err(|e| ProtoError::DeSerde(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
    struct Demo {
        id: u32,
        name: String,
    }

    #[test]
    fn payload_hash_roundtrip_json() {
        let p = Demo { id: 7, name: "ron".into() };
        let bytes = serde_json::to_vec(&p).unwrap();
        let env = SignedEnvelope::unsigned(1, "Demo", Algo::HmacSha256, p.clone(), &bytes);
        assert!(env.payload_hash_matches(&bytes));
        let s = wire::to_json(&env).unwrap();
        let back: SignedEnvelope<Demo> = wire::from_json(&s).unwrap();
        assert_eq!(back.payload, p);
    }

    #[cfg(feature = "rmp")]
    #[test]
    fn payload_hash_roundtrip_msgpack() {
        let p = Demo { id: 42, name: "proto".into() };
        let bytes = wire::to_msgpack(&p).unwrap();
        let env = SignedEnvelope::unsigned(1, "Demo", Algo::HmacSha256, p.clone(), &bytes);
        assert!(env.payload_hash_matches(&bytes));
        let b = wire::to_msgpack(&env).unwrap();
        let back: SignedEnvelope<Demo> = wire::from_msgpack(&b).unwrap();
        assert_eq!(back.payload, p);
    }

    #[test]
    fn signature_helpers_base64() {
        let sig = Signature(vec![1, 2, 3, 4, 5]);
        let b64 = sig.to_base64();
        assert!(!b64.is_empty());
    }
}
