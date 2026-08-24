//! RO:WHAT — svc-passport service KMS trait plus development Ed25519 implementation.
//! RO:WHY — Native Passport challenge signing must bind one preselected KID/public key to the exact signing key even when rotation occurs concurrently.
//! RO:INTERACTS — IssuerState, full svc-passport signing/JWKS/admin surfaces, Native Passport signed-challenge issuer, and future injected production KMS implementations.
//! RO:INVARIANTS — legacy sign/verify/public_keys/rotate/attest behavior remains; new preselection methods fail closed by default; DevKms KID/version/current key move atomically under one lock; stale preselected KIDs never sign; no lock crosses `.await`.
//! RO:METRICS — caller-owned existing service metrics remain unchanged.
//! RO:CONFIG — DevKms remains feature-gated by `dev-kms`; canonical KIDs remain `ed25519/default/v{n}`.
//! RO:SECURITY — private keys remain inside the KMS; active identity exposes only KID/public key; this layer does not issue challenges/capabilities, mutate replay state, or touch wallet/ledger authority.
//! RO:TEST — tests/physical_m1_service_challenge_kms_identity.rs plus existing issuer/JWKS tests.

use async_trait::async_trait;

/// Public identity of one exact service signing key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KmsSigningIdentity {
    /// Stable KMS key identifier.
    pub kid: String,

    /// Exact Ed25519 public key bytes for the selected KID.
    pub public_key: [u8; 32],
}

#[async_trait]
pub trait KmsClient: Send + Sync {
    /// Return the exact currently active KID/public-key pair.
    ///
    /// The default is deliberately fail-closed so existing injected KMS
    /// implementations do not silently acquire Native Passport challenge
    /// signing authority.
    async fn active_signing_identity(&self) -> anyhow::Result<KmsSigningIdentity> {
        Err(anyhow::anyhow!(
            "KMS does not expose an active Native Passport signing identity"
        ))
    }

    /// Sign only when `kid` is still the current signing key.
    ///
    /// Implementations used for Native Passport challenge issuance must reject
    /// a KID that became stale after rotation.
    async fn sign_with_kid(&self, _kid: &str, _msg: &[u8]) -> anyhow::Result<Vec<u8>> {
        Err(anyhow::anyhow!(
            "KMS does not support exact-KID Native Passport signing"
        ))
    }

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
    use rand_core::OsRng;
    use serde_json::json;

    struct DevKmsState {
        version: u64,
        kid: String,
        signing_key: SigningKey,
        verifying_key: VerifyingKey,
        history: Vec<(String, VerifyingKey)>,
    }

    /// Simple in-process development KMS.
    ///
    /// Current version/KID/signing key/verifying key share one lock so readers
    /// never observe a mixed signing identity during rotation.
    pub struct DevKms {
        state: RwLock<DevKmsState>,
    }

    impl Default for DevKms {
        fn default() -> Self {
            Self::new()
        }
    }

    impl DevKms {
        pub fn new() -> Self {
            let mut csprng = OsRng;

            let signing_key = SigningKey::generate(&mut csprng);

            let verifying_key = signing_key.verifying_key();

            let version = 1;
            let kid = kid_for_version(version);

            Self {
                state: RwLock::new(DevKmsState {
                    version,
                    kid: kid.clone(),
                    signing_key,
                    verifying_key,
                    history: vec![(kid, verifying_key)],
                }),
            }
        }
    }

    #[async_trait]
    impl KmsClient for DevKms {
        async fn active_signing_identity(&self) -> anyhow::Result<KmsSigningIdentity> {
            let state = self.state.read();

            Ok(KmsSigningIdentity {
                kid: state.kid.clone(),
                public_key: state.verifying_key.to_bytes(),
            })
        }

        async fn sign_with_kid(&self, kid: &str, msg: &[u8]) -> anyhow::Result<Vec<u8>> {
            let state = self.state.read();

            if state.kid != kid {
                return Err(anyhow::anyhow!("requested signing KID is no longer active"));
            }

            Ok(state.signing_key.sign(msg).to_bytes().to_vec())
        }

        async fn sign(&self, msg: &[u8]) -> anyhow::Result<(String, Vec<u8>)> {
            let state = self.state.read();

            let signature = state.signing_key.sign(msg).to_bytes().to_vec();

            Ok((state.kid.clone(), signature))
        }

        async fn verify(&self, kid: &str, msg: &[u8], sig: &[u8]) -> anyhow::Result<bool> {
            let verifying_key = {
                let state = self.state.read();

                state
                    .history
                    .iter()
                    .find(|(candidate, _)| candidate == kid)
                    .map(|(_, key)| *key)
            }
            .ok_or_else(|| anyhow::anyhow!("unknown kid"))?;

            let Ok(signature) = ed25519_dalek::Signature::from_slice(sig) else {
                return Ok(false);
            };

            Ok(verifying_key.verify_strict(msg, &signature).is_ok())
        }

        async fn public_keys(&self) -> anyhow::Result<serde_json::Value> {
            let state = self.state.read();

            let keys: Vec<_> = state
                .history
                .iter()
                .map(|(kid, verifying_key)| {
                    json!({
                        "kid": kid,
                        "vk_b64":
                            URL_SAFE_NO_PAD.encode(
                                verifying_key.to_bytes()
                            ),
                        "alg": "Ed25519"
                    })
                })
                .collect();

            Ok(json!({
                "alg": "Ed25519",
                "current": state.kid.clone(),
                "keys": keys
            }))
        }

        async fn rotate(&self) -> anyhow::Result<String> {
            /*
             * Generate random key material before taking the write lock.
             * There is no `.await` while the lock is held.
             */
            let mut csprng = OsRng;

            let signing_key = SigningKey::generate(&mut csprng);

            let verifying_key = signing_key.verifying_key();

            let mut state = self.state.write();

            let next_version = state
                .version
                .checked_add(1)
                .ok_or_else(|| anyhow::anyhow!("KMS version overflow"))?;

            let kid = kid_for_version(next_version);

            state.version = next_version;
            state.kid = kid.clone();
            state.signing_key = signing_key;
            state.verifying_key = verifying_key;

            state.history.push((kid.clone(), verifying_key));

            Ok(kid)
        }

        async fn attest(&self) -> anyhow::Result<serde_json::Value> {
            self.public_keys().await
        }
    }

    fn kid_for_version(version: u64) -> String {
        format!("ed25519/default/v{version}")
    }
}

#[cfg(feature = "dev-kms")]
pub use dev::DevKms;

// When `dev-kms` is disabled this module intentionally exposes only the KMS
// contract and public signing-identity DTO. Server callers must inject a real
// implementation; native clients receive no in-process service signing key.
