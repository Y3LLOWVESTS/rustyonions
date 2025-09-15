#![forbid(unsafe_code)]

//! ron-kms: minimal pluggable Key Management for RustyOnions.
//!
//! - Default: HMAC-SHA256 KMS (symmetric) with in-memory keystore.
//! - Optional: Ed25519 (feature = "ed25519") for asymmetric signing.
//! - Integrates with `ron-proto::SignedEnvelope<T>` helpers.
//!
//! This crate focuses on a clean trait boundary. Storage/HSM backends can be
//! swapped later (e.g., file, Sled, cloud KMS) behind the same interface.

use hex::ToHex;
use parking_lot::RwLock;
use rand::{rng, RngCore};
use ron_proto::{Algo, KeyId, Signature};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, sync::Arc};

#[derive(Debug, thiserror::Error)]
pub enum KmsError {
    #[error("unknown key id: {0}")]
    UnknownKey(String),
    #[error("algorithm mismatch: expected {expected:?}, got {got:?}")]
    AlgoMismatch { expected: Algo, got: Algo },
    #[error("unsupported operation: {0}")]
    Unsupported(String),
    #[error("crypto error: {0}")]
    Crypto(String),
}

pub type Result<T> = std::result::Result<T, KmsError>;

/// Opaque private key material for different algos.
#[derive(Clone, Serialize, Deserialize)]
enum KeyMaterial {
    /// Symmetric secret for HMAC-SHA256.
    Hmac(Vec<u8>),
    /// Ed25519 secret key bytes (if feature enabled).
    #[cfg(feature = "ed25519")]
    Ed25519(ed25519_dalek::SecretKey),
}

#[derive(Clone, Serialize, Deserialize)]
struct KeyEntry {
    algo: Algo,
    mat: KeyMaterial,
}

#[derive(Default)]
pub struct InMemoryKms {
    inner: Arc<RwLock<HashMap<String, KeyEntry>>>,
}

impl InMemoryKms {
    pub fn new() -> Self {
        Self { inner: Arc::new(RwLock::new(HashMap::new())) }
    }

    /// Generate a new key for the given algo; returns KeyId.
    pub fn generate_key(&self, algo: Algo) -> Result<KeyId> {
        match algo {
            Algo::HmacSha256 => {
                let mut buf = vec![0u8; 32];
                rng().fill_bytes(&mut buf);
                let kid = self.derive_kid(algo.clone(), &buf);
                let entry = KeyEntry { algo, mat: KeyMaterial::Hmac(buf) };
                self.inner.write().insert(kid.0.clone(), entry);
                Ok(kid)
            }
            #[cfg(feature = "ed25519")]
            Algo::Ed25519 => {
                use ed25519_dalek::{SecretKey, SigningKey};
                // Generate securely from OS RNG via dalek
                let signing = SigningKey::generate(&mut rand::rng());
                let secret = signing.to_secret_key();
                let kid = self.derive_kid(Algo::Ed25519, secret.as_bytes());
                let entry = KeyEntry { algo: Algo::Ed25519, mat: KeyMaterial::Ed25519(secret) };
                self.inner.write().insert(kid.0.clone(), entry);
                Ok(kid)
            }
            #[cfg(not(feature = "ed25519"))]
            Algo::Ed25519 => Err(KmsError::Unsupported(
                "Ed25519 requested but crate built without feature \"ed25519\"".into(),
            )),
        }
    }

    /// Import a pre-existing secret (bytes meaning depends on algo).
    pub fn import_key(&self, algo: Algo, key_bytes: &[u8]) -> Result<KeyId> {
        match algo {
            Algo::HmacSha256 => {
                if key_bytes.is_empty() {
                    return Err(KmsError::Crypto("empty key".into()));
                }
                let kid = self.derive_kid(algo.clone(), key_bytes);
                let entry = KeyEntry { algo, mat: KeyMaterial::Hmac(key_bytes.to_vec()) };
                self.inner.write().insert(kid.0.clone(), entry);
                Ok(kid)
            }
            #[cfg(feature = "ed25519")]
            Algo::Ed25519 => {
                use ed25519_dalek::SecretKey;
                let sk = SecretKey::from_bytes(key_bytes)
                    .map_err(|e| KmsError::Crypto(format!("bad ed25519 secret: {e}")))?;
                let kid = self.derive_kid(Algo::Ed25519, sk.as_bytes());
                let entry = KeyEntry { algo: Algo::Ed25519, mat: KeyMaterial::Ed25519(sk) };
                self.inner.write().insert(kid.0.clone(), entry);
                Ok(kid)
            }
            #[cfg(not(feature = "ed25519"))]
            Algo::Ed25519 => Err(KmsError::Unsupported(
                "Ed25519 requested but crate built without feature \"ed25519\"".into(),
            )),
        }
    }

    /// Delete a key from the in-memory store.
    pub fn delete_key(&self, kid: &KeyId) -> Result<()> {
        let removed = self.inner.write().remove(&kid.0);
        if removed.is_some() {
            Ok(())
        } else {
            Err(KmsError::UnknownKey(kid.0.clone()))
        }
    }

    /// Produce a signature over `msg` using the key `kid`.
    pub fn sign(&self, kid: &KeyId, algo: Algo, msg: &[u8]) -> Result<Signature> {
        let entry = self.inner.read().get(&kid.0).cloned().ok_or_else(|| KmsError::UnknownKey(kid.0.clone()))?;
        if entry.algo != algo {
            return Err(KmsError::AlgoMismatch { expected: entry.algo, got: algo });
        }
        match entry.mat {
            KeyMaterial::Hmac(ref k) => {
                use hmac::{Hmac, Mac};
                type HmacSha256 = Hmac<sha2::Sha256>;
                let mut mac = HmacSha256::new_from_slice(k).map_err(|e| KmsError::Crypto(e.to_string()))?;
                mac.update(msg);
                let res = mac.finalize().into_bytes().to_vec();
                Ok(Signature::from_bytes(res))
            }
            #[cfg(feature = "ed25519")]
            KeyMaterial::Ed25519(ref sk) => {
                use ed25519_dalek::{SecretKey, SigningKey, Signer};
                let signing = SigningKey::from(sk.clone());
                let sig = signing.sign(msg);
                Ok(Signature::from_bytes(sig.to_bytes().to_vec()))
            }
        }
    }

    /// Verify a signature over `msg` using the key `kid`.
    pub fn verify(&self, kid: &KeyId, algo: Algo, msg: &[u8], sig: &Signature) -> Result<bool> {
        let entry = self.inner.read().get(&kid.0).cloned().ok_or_else(|| KmsError::UnknownKey(kid.0.clone()))?;
        if entry.algo != algo {
            return Err(KmsError::AlgoMismatch { expected: entry.algo, got: algo });
        }
        match entry.mat {
            KeyMaterial::Hmac(ref k) => {
                use hmac::{Hmac, Mac};
                type HmacSha256 = Hmac<sha2::Sha256>;
                let mut mac = HmacSha256::new_from_slice(k).map_err(|e| KmsError::Crypto(e.to_string()))?;
                mac.update(msg);
                let expected = mac.finalize().into_bytes().to_vec();
                Ok(constant_time_eq::constant_time_eq(sig.as_bytes(), &expected))
            }
            #[cfg(feature = "ed25519")]
            KeyMaterial::Ed25519(ref sk) => {
                use ed25519_dalek::{SecretKey, SigningKey, VerifyingKey, Signature as DalekSig, Verifier};
                let signing = SigningKey::from(sk.clone());
                let vk: VerifyingKey = signing.verifying_key();
                let dsig = DalekSig::from_bytes(sig.as_bytes()).map_err(|e| KmsError::Crypto(e.to_string()))?;
                Ok(vk.verify(msg, &dsig).is_ok())
            }
        }
    }

    /// Sign a `ron_proto::SignedEnvelope<T>` payload bytes and attach KID+signature.
    ///
    /// `payload_bytes` must be exactly the bytes used to compute `payload_hash`.
    pub fn sign_envelope<T: serde::Serialize + for<'de> serde::Deserialize<'de>>(
        &self,
        mut env: ron_proto::SignedEnvelope<T>,
        kid: &KeyId,
        algo: Algo,
        payload_bytes: &[u8],
    ) -> Result<ron_proto::SignedEnvelope<T>> {
        if !env.payload_hash_matches(payload_bytes) {
            return Err(KmsError::Crypto("payload bytes do not match envelope.payload_hash".into()));
        }
        let sig = self.sign(kid, algo.clone(), payload_bytes)?;
        Ok(env.with_signature(kid.clone(), sig))
    }

    /// Minimal stable KID derivation: `kid = algo || ":" || hex(sha256(secret_bytes))`
    fn derive_kid(&self, algo: Algo, secret: &[u8]) -> KeyId {
        let mut hasher = Sha256::new();
        hasher.update(secret);
        let digest = hasher.finalize().encode_hex::<String>();
        let prefix = match algo {
            Algo::HmacSha256 => "hmac",
            Algo::Ed25519 => "ed25519",
        };
        KeyId(format!("{prefix}:{digest}"))
    }
}

// Constant-time equality (tiny helper) without adding a new crate for one fn.
mod constant_time_eq {
    pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
        if a.len() != b.len() {
            return false;
        }
        let mut r = 0u8;
        for (x, y) in a.iter().zip(b.iter()) {
            r |= x ^ y;
        }
        r == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ron_proto::{wire, SignedEnvelope};

    #[test]
    fn hmac_sign_verify() {
        let kms = InMemoryKms::new();
        let kid = kms.generate_key(Algo::HmacSha256).unwrap();

        let msg = b"hello world";
        let sig = kms.sign(&kid, Algo::HmacSha256, msg).unwrap();
        assert!(kms.verify(&kid, Algo::HmacSha256, msg, &sig).unwrap());
        assert!(!kms.verify(&kid, Algo::HmacSha256, b"tampered", &sig).unwrap());
    }

    #[test]
    fn sign_envelope_msgpack_roundtrip() {
        #[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
        struct Demo { id: u64, body: String }

        let kms = InMemoryKms::new();
        let kid = kms.generate_key(Algo::HmacSha256).unwrap();

        let payload = Demo { id: 1, body: "ron".into() };
        let bytes = wire::to_json(&payload).unwrap().into_bytes(); // using JSON for the demo

        let env = SignedEnvelope::unsigned(1, "Demo", Algo::HmacSha256, payload.clone(), &bytes);
        let signed = kms.sign_envelope(env, &kid, Algo::HmacSha256, &bytes).unwrap();

        // payload hash should match and signature should verify
        assert!(signed.payload_hash_matches(&bytes));
        assert!(kms.verify(&kid, Algo::HmacSha256, &bytes, &signed.sig).unwrap());
    }

    #[cfg(feature = "ed25519")]
    #[test]
    fn ed25519_sign_verify() {
        let kms = InMemoryKms::new();
        let kid = kms.generate_key(Algo::Ed25519).unwrap();
        let msg = b"hello ed25519";
        let sig = kms.sign(&kid, Algo::Ed25519, msg).unwrap();
        assert!(kms.verify(&kid, Algo::Ed25519, msg, &sig).unwrap());
    }
}
