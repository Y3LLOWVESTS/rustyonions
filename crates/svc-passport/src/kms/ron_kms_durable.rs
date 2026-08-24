//! RO:WHAT — `svc-passport::KmsClient` adapter over one durable `ron-kms` Ed25519 service key.
//! RO:WHY — Native Passport challenges require a restart-stable service signing identity without constructing `DevKms` or duplicating secret custody.
//! RO:INTERACTS — `KmsClient`, `ron-kms::DurableEd25519ServiceKey`, `ron-kms` `Signer`/`Verifier`, and `ron-proto::ServiceKeyIdV1`.
//! RO:INVARIANTS — one persisted key maps deterministically to one canonical Native Passport wire KID; exact-KID signing rejects all other KIDs; unsupported rotation fails closed; no lock crosses `.await`.
//! RO:METRICS — none.
//! RO:CONFIG — durable key ownership/path remains caller-owned; this adapter receives an already-open `ron-kms` service key.
//! RO:SECURITY — no secret getter, key generation, key export, KMS administration, capability issuance, username mutation, wallet mutation, or ledger mutation.
//! RO:TEST — CN-4 adapter tests prove stable KID/public identity, exact-KID signing, verification across reopen, and fail-closed rotation.

#![forbid(unsafe_code)]

use std::sync::Arc;

use async_trait::async_trait;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use ron_kms::{
    DurableEd25519ServiceKey, KeyId as RonKeyId, Signer as RonSigner, Verifier as RonVerifier,
};
use ron_proto::ServiceKeyIdV1;
use serde_json::json;

use super::client::{KmsClient, KmsSigningIdentity};

/// Adapter from canonical durable `ron-kms` custody into the service-local
/// asynchronous `KmsClient` boundary.
pub struct DurableRonKmsClient {
    key: Arc<DurableEd25519ServiceKey>,
    concrete_key_id: RonKeyId,
    wire_kid: ServiceKeyIdV1,
}

impl DurableRonKmsClient {
    /// Bind one already-open durable service key to its deterministic Native
    /// Passport wire KID.
    pub fn new(key: Arc<DurableEd25519ServiceKey>) -> anyhow::Result<Self> {
        let concrete_key_id = key.key_id().clone();

        /*
         * ron-kms KeyId Display is intentionally richer than the Native
         * Passport wire identifier and contains characters the protocol does
         * not admit. Map the same persisted identity deterministically instead
         * of weakening ServiceKeyIdV1 validation.
         */
        let wire_kid_text = format!(
            "{}/{}/{}/{}/v{}",
            concrete_key_id.alg.as_str(),
            concrete_key_id.tenant,
            concrete_key_id.purpose,
            concrete_key_id.uuid,
            concrete_key_id.version,
        );

        let wire_kid = ServiceKeyIdV1::parse(wire_kid_text).map_err(|error| {
            anyhow::anyhow!(
                "durable ron-kms key cannot map to canonical Native Passport service KID: {error}"
            )
        })?;

        Ok(Self {
            key,
            concrete_key_id,
            wire_kid,
        })
    }

    /// Canonical Native Passport wire KID for this persisted key.
    #[must_use]
    pub fn wire_kid(&self) -> &str {
        self.wire_kid.as_str()
    }

    /// Public verification key safe for challenge verification.
    #[must_use]
    pub fn public_key(&self) -> [u8; 32] {
        self.key.public_key()
    }

    fn require_exact_wire_kid(&self, kid: &str) -> anyhow::Result<()> {
        if kid != self.wire_kid.as_str() {
            return Err(anyhow::anyhow!(
                "requested service KID does not match active durable key"
            ));
        }

        Ok(())
    }
}

#[async_trait]
impl KmsClient for DurableRonKmsClient {
    async fn active_signing_identity(&self) -> anyhow::Result<KmsSigningIdentity> {
        Ok(KmsSigningIdentity {
            kid: self.wire_kid.to_string(),
            public_key: self.key.public_key(),
        })
    }

    async fn sign_with_kid(&self, kid: &str, msg: &[u8]) -> anyhow::Result<Vec<u8>> {
        self.require_exact_wire_kid(kid)?;

        RonSigner::sign(self.key.as_ref(), &self.concrete_key_id, msg)
            .map_err(|error| anyhow::anyhow!("durable ron-kms exact-KID signing failed: {error}"))
    }

    async fn sign(&self, msg: &[u8]) -> anyhow::Result<(String, Vec<u8>)> {
        let signature = RonSigner::sign(self.key.as_ref(), &self.concrete_key_id, msg)
            .map_err(|error| anyhow::anyhow!("durable ron-kms signing failed: {error}"))?;

        Ok((self.wire_kid.to_string(), signature))
    }

    async fn verify(&self, kid: &str, msg: &[u8], sig: &[u8]) -> anyhow::Result<bool> {
        self.require_exact_wire_kid(kid)?;

        RonVerifier::verify(self.key.as_ref(), &self.concrete_key_id, msg, sig)
            .map_err(|error| anyhow::anyhow!("durable ron-kms verification failed: {error}"))
    }

    async fn public_keys(&self) -> anyhow::Result<serde_json::Value> {
        let public_key = self.key.public_key();

        Ok(json!({
            "alg": "Ed25519",
            "current": self.wire_kid.as_str(),
            "keys": [
                {
                    "kid": self.wire_kid.as_str(),
                    "vk_b64": URL_SAFE_NO_PAD.encode(public_key),
                    "alg": "Ed25519"
                }
            ]
        }))
    }

    async fn rotate(&self) -> anyhow::Result<String> {
        Err(anyhow::anyhow!(
            "durable CN-4 service-key rotation is not implemented"
        ))
    }

    async fn attest(&self) -> anyhow::Result<serde_json::Value> {
        self.public_keys().await
    }
}
