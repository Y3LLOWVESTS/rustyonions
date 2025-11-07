//! RO:WHAT — KmsClient trait; DevKms (ed25519-dalek) for dev/tests.
//! RO:INVARIANTS — versioned KID "ed25519/default/v{n}"
//! RO:NOTES — DevKms is feature-gated behind `dev-kms` and uses rand_core::OsRng.

use async_trait::async_trait;

#[async_trait]
pub trait KmsClient: Send + Sync {
    async fn sign(&self, msg: &[u8]) -> anyhow::Result<(String, Vec<u8>)>;
    async fn verify(&self, kid: &str, msg: &[u8], sig: &[u8]) -> anyhow::Result<bool>;
    async fn public_keys(&self) -> anyhow::Result<serde_json::Value>;
    async fn rotate(&self) -> anyhow::Result<String>;
    async fn attest(&self) -> anyhow::Result<serde_json::Value>;
}

#[cfg(feature = "dev-kms")]
mod dev {
    use super::*;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;
    use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
    use parking_lot::RwLock;
    use rand_core::OsRng; // rand_core 0.6 — CSPRNG compatible with dalek v2
    use serde_json::json;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// Simple in-proc KMS for dev/tests.
    pub struct DevKms {
        version: AtomicU64,
        head: RwLock<(SigningKey, VerifyingKey)>,
        history: RwLock<Vec<(String, VerifyingKey)>>, // (kid, vk)
    }

    impl Default for DevKms {
        fn default() -> Self {
            Self::new()
        }
    }

    impl DevKms {
        pub fn new() -> Self {
            // Use OsRng (rand_core 0.6) to satisfy dalek's CryptoRngCore bound.
            let mut csprng = OsRng;
            let sk = SigningKey::generate(&mut csprng);
            let vk = sk.verifying_key();
            let kid = "ed25519/default/v1".to_string();
            Self {
                version: AtomicU64::new(1),
                head: RwLock::new((sk, vk)),
                history: RwLock::new(vec![(kid, vk)]),
            }
        }

        fn current_kid(&self) -> String {
            format!("ed25519/default/v{}", self.version.load(Ordering::SeqCst))
        }
    }

    #[async_trait]
    impl KmsClient for DevKms {
        async fn sign(&self, msg: &[u8]) -> anyhow::Result<(String, Vec<u8>)> {
            let (sk, _vk) = &*self.head.read();
            let sig = sk.sign(msg).to_bytes().to_vec();
            Ok((self.current_kid(), sig))
        }

        async fn verify(&self, kid: &str, msg: &[u8], sig: &[u8]) -> anyhow::Result<bool> {
            // find vk by kid
            let vk = {
                let hist = self.history.read();
                hist.iter().find(|(k, _)| k == kid).map(|(_, v)| *v)
            }
            .ok_or_else(|| anyhow::anyhow!("unknown kid"))?;

            let Ok(sig) = ed25519_dalek::Signature::from_slice(sig) else {
                return Ok(false);
            };
            Ok(vk.verify_strict(msg, &sig).is_ok())
        }

        async fn public_keys(&self) -> anyhow::Result<serde_json::Value> {
            let keys: Vec<_> = {
                let hist = self.history.read();
                hist.iter()
                    .map(|(kid, vk)| {
                        json!({"kid": kid, "vk_b64": URL_SAFE_NO_PAD.encode(vk.to_bytes()), "alg":"Ed25519"})
                    })
                    .collect()
            };
            Ok(json!({"alg":"Ed25519","current": self.current_kid(), "keys": keys }))
        }

        async fn rotate(&self) -> anyhow::Result<String> {
            // Bump to a new key/version using OsRng again.
            let mut csprng = OsRng;
            let sk = SigningKey::generate(&mut csprng);
            let vk = sk.verifying_key();

            {
                let mut head = self.head.write();
                *head = (sk, vk);
            }
            let next = self.version.load(Ordering::SeqCst) + 1;
            self.version.store(next, Ordering::SeqCst);
            let kid = self.current_kid();
            self.history.write().push((kid.clone(), vk));
            Ok(kid)
        }

        async fn attest(&self) -> anyhow::Result<serde_json::Value> {
            self.public_keys().await
        }
    }
}

#[cfg(feature = "dev-kms")]
pub use dev::DevKms;

#[cfg(not(feature = "dev-kms"))]
compile_error!("Enable feature `dev-kms` or wire a real ron-kms client here.");
