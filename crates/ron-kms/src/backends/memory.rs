// RO:WHAT  Dev in-memory KMS with version-retaining verification (contiguous VKs).
// RO:INV   Non-exportability; only the latest version can sign; any version can verify.

use crate::{
    error::KmsError,
    traits::pubkey::PubkeyProvider,
    traits::{Keystore, Signer, Verifier},
    types::{Alg, KeyId, KeyMeta},
    util::time::now_utc_ms,
};
use ahash::AHashMap as HashMap;
use ed25519_dalek::Signer as _; // bring .sign() into scope
use ed25519_dalek::{Signature, SigningKey, VerifyingKey};
use parking_lot::RwLock;
use rand::rngs::OsRng;
use std::sync::Arc;

#[derive(Clone)]
pub struct MemoryKeystore(Arc<RwLock<State>>);

#[derive(Default)]
struct State {
    // Map by "stable root" (tenant/purpose/alg/uuid → per-root record)
    roots: HashMap<String, Root>,
}

struct Root {
    alg: Alg,
    current_version: u32,
    created_ms: i128,
    // Only the latest private key is retained.
    sk: SigningKey,
    // Convenience copy of the current verifying key.
    vk: VerifyingKey,
    // Verification keys for all versions we've ever issued (including current).
    // Index = version - 1.
    vks: Vec<VerifyingKey>,
}

impl Default for MemoryKeystore {
    fn default() -> Self {
        Self(Arc::new(RwLock::new(State {
            roots: HashMap::new(),
        })))
    }
}

impl MemoryKeystore {
    fn root_id(tenant: &str, purpose: &str, alg: Alg, uuid: uuid::Uuid) -> String {
        format!("{tenant}/{purpose}/{alg}/{uuid}")
    }

    fn gen_signing_key() -> SigningKey {
        // Requires ed25519-dalek feature "rand_core".
        SigningKey::generate(&mut OsRng)
    }
}

impl Keystore for MemoryKeystore {
    fn create_ed25519(&self, tenant: &str, purpose: &str) -> Result<KeyId, KmsError> {
        let alg = Alg::Ed25519;
        let kid = KeyId::new(tenant, purpose, alg);
        let root_id = Self::root_id(&kid.tenant, &kid.purpose, kid.alg, kid.uuid);

        let sk = Self::gen_signing_key();
        let vk = VerifyingKey::from(&sk);

        let mut vks = Vec::with_capacity(4);
        vks.push(vk);

        let root = Root {
            alg,
            current_version: kid.version,
            created_ms: now_utc_ms(),
            sk,
            vk,
            vks,
        };

        let mut st = self.0.write();
        st.roots.insert(root_id, root);
        Ok(kid)
    }

    fn rotate(&self, kid: &KeyId) -> Result<KeyId, KmsError> {
        let root_id = Self::root_id(&kid.tenant, &kid.purpose, kid.alg, kid.uuid);
        let mut st = self.0.write();
        let root = st.roots.get_mut(&root_id).ok_or(KmsError::NoSuchKey)?;
        if root.alg != Alg::Ed25519 {
            return Err(KmsError::AlgUnavailable);
        }
        // New keypair → bump version → retain vk for verify
        let sk = Self::gen_signing_key();
        let vk = VerifyingKey::from(&sk);

        root.current_version = root.current_version.saturating_add(1);
        // Compare without truncation: convert len → u32 (Option) and compare to checked_sub(1).
        debug_assert_eq!(
            u32::try_from(root.vks.len()).ok(),
            root.current_version.checked_sub(1)
        );
        root.vks.push(vk);
        root.sk = sk;
        root.vk = vk;

        let mut new = kid.clone();
        new.version = root.current_version;
        Ok(new)
    }

    fn alg(&self, kid: &KeyId) -> Result<Alg, KmsError> {
        let root_id = Self::root_id(&kid.tenant, &kid.purpose, kid.alg, kid.uuid);
        let st = self.0.read();
        let root = st.roots.get(&root_id).ok_or(KmsError::NoSuchKey)?;
        Ok(root.alg)
    }

    fn meta(&self, kid: &KeyId) -> Result<KeyMeta, KmsError> {
        let root_id = Self::root_id(&kid.tenant, &kid.purpose, kid.alg, kid.uuid);
        let st = self.0.read();
        let root = st.roots.get(&root_id).ok_or(KmsError::NoSuchKey)?;
        // Versions are 1..=current_version by invariant.
        let versions: Vec<u32> = (1..=root.current_version).collect();
        Ok(KeyMeta {
            alg: root.alg,
            current_version: root.current_version,
            versions,
            created_ms: root.created_ms,
        })
    }
}

impl Signer for MemoryKeystore {
    fn sign(&self, kid: &KeyId, msg: &[u8]) -> Result<Vec<u8>, KmsError> {
        if kid.alg != Alg::Ed25519 {
            return Err(KmsError::AlgUnavailable);
        }
        let root_id = Self::root_id(&kid.tenant, &kid.purpose, kid.alg, kid.uuid);
        let st = self.0.read();
        let root = st.roots.get(&root_id).ok_or(KmsError::NoSuchKey)?;

        // Only the latest version is allowed to sign.
        if kid.version != root.current_version {
            return Err(KmsError::Busy);
        }

        let sig: Signature = root.sk.sign(msg);
        Ok(sig.to_bytes().to_vec())
    }
}

impl Verifier for MemoryKeystore {
    fn verify(&self, kid: &KeyId, msg: &[u8], sig: &[u8]) -> Result<bool, KmsError> {
        if kid.alg != Alg::Ed25519 {
            return Err(KmsError::AlgUnavailable);
        }
        let root_id = Self::root_id(&kid.tenant, &kid.purpose, kid.alg, kid.uuid);
        let st = self.0.read();
        let root = st.roots.get(&root_id).ok_or(KmsError::NoSuchKey)?;

        let idx = kid.version.checked_sub(1).ok_or(KmsError::NoSuchKey)? as usize;
        let vk = root.vks.get(idx).ok_or(KmsError::NoSuchKey)?;
        let sig = ed25519_dalek::Signature::from_slice(sig).map_err(|_| KmsError::VerifyFailed)?;
        Ok(vk.verify_strict(msg, &sig).is_ok())
    }
}

impl PubkeyProvider for MemoryKeystore {
    fn verifying_key_bytes(&self, kid: &KeyId) -> Result<[u8; 32], KmsError> {
        if kid.alg != Alg::Ed25519 {
            return Err(KmsError::AlgUnavailable);
        }
        let root_id = Self::root_id(&kid.tenant, &kid.purpose, kid.alg, kid.uuid);
        let st = self.0.read();
        let root = st.roots.get(&root_id).ok_or(KmsError::NoSuchKey)?;
        let idx = kid.version.checked_sub(1).ok_or(KmsError::NoSuchKey)? as usize;
        let vk = root.vks.get(idx).ok_or(KmsError::NoSuchKey)?;
        Ok(vk.to_bytes())
    }
}
