//! RO:WHAT — Issuer state: binds Config + KMS; signs, verifies, batches by KID.

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

use crate::{
    dto::verify::Envelope, error::Error, kms::client::KmsClient, token::encode::decode_envelope,
    Config,
};

#[derive(Clone)]
pub struct IssuerState {
    cfg: Config,
    kms: Arc<dyn KmsClient>,
    // cache kid -> (vk present?) for quick "unknown_kid" fast-path; Bronze keeps empty (KMS holds truth)
    cache: Arc<RwLock<HashMap<String, ()>>>,
}

impl IssuerState {
    pub fn new(cfg: Config, kms: Arc<dyn KmsClient>) -> Self {
        Self {
            cfg,
            kms,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn sign(&self, payload: &serde_json::Value) -> Result<(String, Vec<u8>), Error> {
        let bytes = serde_json::to_vec(payload).map_err(|_| Error::Malformed)?;
        let (kid, sig) = self
            .kms
            .sign(&bytes)
            .await
            .map_err(|e| Error::Internal(e.into()))?;
        Ok((kid, sig))
    }

    pub async fn jwks(&self) -> Result<serde_json::Value, Error> {
        self.kms
            .public_keys()
            .await
            .map_err(|e| Error::Internal(e.into()))
    }

    pub async fn rotate(&self) -> Result<String, Error> {
        self.kms
            .rotate()
            .await
            .map_err(|e| Error::Internal(e.into()))
    }

    pub async fn attest(&self) -> Result<serde_json::Value, Error> {
        self.kms
            .attest()
            .await
            .map_err(|e| Error::Internal(e.into()))
    }

    pub async fn verify(&self, env: &Envelope) -> Result<bool, Error> {
        let (msg, sig) = decode_envelope(env).map_err(|_| Error::Malformed)?;
        self.kms.verify(&env.kid, &msg, &sig).await.map_err(|e| {
            match e.to_string().contains("unknown") {
                true => Error::UnknownKid,
                false => Error::Internal(e.into()),
            }
        })
    }

    pub async fn group_by_kid(
        &self,
        envs: &[Envelope],
        idxs: &[usize],
    ) -> Result<HashMap<String, Vec<usize>>, Error> {
        let mut m: HashMap<String, Vec<usize>> = HashMap::new();
        for &i in idxs {
            m.entry(envs[i].kid.clone()).or_default().push(i);
        }
        Ok(m)
    }

    pub async fn verify_many(&self, envs: &[Envelope], idxs: &[usize]) -> Result<Vec<bool>, Error> {
        // Bronze path: loop; Silver+ will call ron-kms batch verify.
        let mut out = Vec::with_capacity(idxs.len());
        for &i in idxs {
            let (msg, sig) = decode_envelope(&envs[i]).map_err(|_| Error::Malformed)?;
            let ok = self
                .kms
                .verify(&envs[i].kid, &msg, &sig)
                .await
                .map_err(|e| {
                    if e.to_string().contains("unknown") {
                        Error::UnknownKid
                    } else {
                        Error::Internal(e.into())
                    }
                })?;
            out.push(ok);
        }
        Ok(out)
    }
}
