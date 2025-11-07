// RO:WHAT   IssuerState: thin service state around a KMS client + helpers.
// RO:WHY    Handlers expect helpers: build_envelope, jwks, rotate, attest, verify_envelope.

use crate::{config::Config, dto::verify::Envelope, error::Error, kms::client::KmsClient};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use parking_lot::RwLock;
use serde_json::{json, Value};
use std::{collections::HashMap, sync::Arc};

#[derive(Clone)]
pub struct IssuerState {
    pub cfg: Config,
    pub kms: Arc<dyn KmsClient>,
    pub cache: Arc<RwLock<HashMap<String, ()>>>,
}

impl IssuerState {
    pub fn new(cfg: Config, kms: Arc<dyn KmsClient>) -> Self {
        Self {
            cfg,
            kms,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Sign raw message bytes; returns (kid, signature).
    pub async fn sign(&self, msg: &[u8]) -> Result<(String, Vec<u8>), Error> {
        self.kms.sign(msg).await.map_err(Error::Internal)
    }

    /// Verify signature with kid; returns true/false.
    pub async fn verify(&self, kid: &str, msg: &[u8], sig: &[u8]) -> Result<bool, Error> {
        self.kms
            .verify(kid, msg, sig)
            .await
            .map_err(Error::Internal)
    }

    /// Build a transport envelope from parts.
    pub fn build_envelope(&self, kid: String, msg: Vec<u8>, sig: Vec<u8>) -> Envelope {
        Envelope {
            alg: "Ed25519".to_string(),
            kid,
            msg_b64: STANDARD.encode(msg),
            sig_b64: STANDARD.encode(sig),
        }
    }

    /// Public JWKS (OKP/Ed25519). Converts KMS `public_keys()` shape -> standard JWKS.
    ///
    /// DevKms `public_keys()` returns:
    /// { "alg":"Ed25519", "current":"ed25519/default/vN",
    ///   "keys":[{"kid": "...", "vk_b64": "<urlsafe-b64>", "alg":"Ed25519"}] }
    pub async fn jwks(&self) -> Result<Value, Error> {
        let kms_view = self.kms.public_keys().await.map_err(Error::Internal)?;
        let Some(keys) = kms_view.get("keys").and_then(|v| v.as_array()) else {
            return Ok(json!({ "keys": [] }));
        };

        // Map into OKP JWKS set
        let jwk_keys: Vec<Value> = keys
            .iter()
            .filter_map(|k| {
                let kid = k.get("kid")?.as_str()?;
                let x = k.get("vk_b64")?.as_str()?; // already URL_SAFE_NO_PAD encoded
                Some(json!({
                    "kty": "OKP",
                    "crv": "Ed25519",
                    "use": "sig",
                    "key_ops": ["verify"],
                    "alg": "EdDSA",
                    "kid": kid,
                    "x": x
                }))
            })
            .collect();

        Ok(json!({ "keys": jwk_keys }))
    }
}
