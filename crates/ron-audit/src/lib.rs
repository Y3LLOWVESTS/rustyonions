#![forbid(unsafe_code)]
// ron-audit: signed, chained audit log (append-only).

use anyhow::Result;
use blake3::Hasher;
use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug)]
pub struct Auditor {
    sk: SigningKey,
    vk: VerifyingKey,
    prev_hash: [u8; 32],
    #[cfg(feature = "fs")]
    dir: std::path::PathBuf,
}

impl Auditor {
    pub fn new() -> Self {
        let mut rng = OsRng;
        let sk = SigningKey::generate(&mut rng);
        let vk = sk.verifying_key();
        Self { sk, vk, prev_hash: [0u8; 32], #[cfg(feature="fs")] dir: std::path::PathBuf::new() }
    }
    #[cfg(feature = "fs")]
    pub fn with_dir(mut self, dir: impl Into<std::path::PathBuf>) -> Self {
        self.dir = dir.into();
        std::fs::create_dir_all(&self.dir).ok();
        self
    }
    pub fn verifying_key(&self) -> &VerifyingKey { &self.vk }

    pub fn append(&mut self, kind: &'static str, data: serde_json::Value) -> Result<AuditRecord> {
        let ts = OffsetDateTime::now_tc().unix_timestamp();
        let mut hasher = Hasher::new();
        hasher.update(&self.prev_hash);
        let body = AuditBody { ts, kind, data };
        let ser = rmp_serde::to_vec_named(&body)?;
        hasher.update(&ser);
        let hash = *hasher.finalize().as_bytes();
        let sig = self.sk.sign(&hash);
        let rec = AuditRecord { prev_hash: self.prev_hash, hash, sig: sig.to_bytes().to_vec(), body };
        self.prev_hash = hash;

        #[cfg(feature = "fs")]
        {
            let p = self.dir.join(format!("{ts}-{kind}.bin"));
            std::fs::write(p, rmp_serde::to_vec_named(&rec)?)?;
        }
        Ok(rec)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditBody {
    pub ts: i64,
    pub kind: &'static str,
    /// Arbitrary JSON (small)
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    pub prev_hash: [u8; 32],
    pub hash: [u8; 32],
    pub sig: Vec<u8>,
    pub body: AuditBody,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chain_two() {
        let mut a = Auditor::new();
        let r1 = a.append("auth-fail", serde_json::json!({"who":"svc-gateway"})).unwrap();
        let r2 = a.append("key-rotated", serde_json::json!({"id":"epoch-42"})).unwrap();
        assert_eq!(r1.hash, r2.prev_hash);
        assert_ne!(r1.hash, r2.hash);
    }
}
