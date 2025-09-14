#![forbid(unsafe_code)]
// ron-auth: zero-trust envelopes for internal IPC & UDS boundaries.

use hmac::{Hmac, Mac};
use rand::RngCore;
use sha2::Sha256;
use smallvec::SmallVec;
use time::OffsetDateTime;
use uuid::Uuid;

use serde::{Deserialize, Serialize};

pub type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Plane { Node, App }

/// Generic, self-authenticating envelope around a protocol header+payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope<H, P> {
    pub plane: Plane,
    pub origin_svc: &'static str,
    pub origin_instance: Uuid,
    pub tenant_id: Option<String>,
    #[serde(with = "serde_scopes")]
    pub scopes: SmallVec<[String; 4]>,
    pub nonce: [u8; 16],
    pub iat: i64, // seconds since epoch
    pub exp: i64, // seconds since epoch
    pub header: H,
    pub payload: P,
    pub tag: [u8; 32], // HMAC-SHA256
}

/// A minimal trait for envelope key derivation.
/// Implement this in services, usually by delegating to ron-kms.
pub trait KeyDeriver: Send + Sync + 'static {
    /// 32-byte HMAC key for (svc, instance, epoch).
    fn derive_origin_key(&self, svc: &str, instance: &Uuid, epoch: u64) -> [u8; 32];
}

/// Construct tag input deterministically using rmp-serde to avoid JSON ambiguity.
fn encode_for_mac<H: Serialize, P: Serialize>(env: &Envelope<H, P>) -> Vec<u8> {
    #[derive(Serialize)]
    struct MacView<'a, H, P> {
        plane: &'a Plane,
        origin_svc: &'a str,
        origin_instance: &'a Uuid,
        tenant_id: &'a Option<String>,
        #[serde(with = "serde_scopes")]
        scopes: &'a SmallVec<[String; 4]>,
        nonce: &'a [u8; 16],
        iat: i64,
        exp: i64,
        header: &'a H,
        payload: &'a P,
    }
    let mv = MacView {
        plane: &env.plane,
        origin_svc: env.origin_svc,
        origin_instance: &env.origin_instance,
        tenant_id: &env.tenant_id,
        scopes: &env.scopes,
        nonce: &env.nonce,
        iat: env.iat,
        exp: env.exp,
        header: &env.header,
        payload: &env.payload,
    };
    rmp_serde::to_vec_named(&mv).expect("rmp encode")
}

/// Sign an envelope given a KeyDeriver and epoch (e.g., day number).
pub fn sign_envelope<H: Serialize, P: Serialize>(
    kd: &dyn KeyDeriver,
    svc: &str,
    instance: &Uuid,
    epoch: u64,
    mut env: Envelope<H, P>,
) -> Envelope<H, P> {
    let key = kd.derive_origin_key(svc, instance, epoch);
    let mut mac = HmacSha256::new_from_slice(&key).expect("HMAC key");
    let bytes = encode_for_mac(&env);
    mac.update(&bytes);
    let tag = mac.finalize().into_bytes();
    env.tag.copy_from_slice(&tag);
    env
}

/// Verify tag, time window, and required scopes. Sender must match `expected_svc`.
pub fn verify_envelope<H: Serialize, P: Serialize>(
    kd: &dyn KeyDeriver,
    expected_svc: &str,
    epoch: u64,
    required_scopes: &[&str],
    env: &Envelope<H, P>,
) -> Result<(), VerifyError> {
    verify_common(kd, epoch, required_scopes, env, &|svc| svc == expected_svc)
}

/// Like `verify_envelope` but allow any sender in `allowed_senders`.
pub fn verify_envelope_from_any<H: Serialize, P: Serialize>(
    kd: &dyn KeyDeriver,
    allowed_senders: &[&str],
    epoch: u64,
    required_scopes: &[&str],
    env: &Envelope<H, P>,
) -> Result<(), VerifyError> {
    verify_common(kd, epoch, required_scopes, env, &|svc| allowed_senders.iter().any(|s| s == &svc))
}

fn verify_common<H: Serialize, P: Serialize>(
    kd: &dyn KeyDeriver,
    epoch: u64,
    required_scopes: &[&str],
    env: &Envelope<H, P>,
    sender_ok: &dyn Fn(&str) -> bool,
) -> Result<(), VerifyError> {
    // Time window check
    let now = OffsetDateTime::now_utc().unix_timestamp();
    if now < env.iat || now > env.exp {
        return Err(VerifyError::Expired);
    }
    // Scope check
    for need in required_scopes {
        if !env.scopes.iter().any(|s| s == need) {
            return Err(VerifyError::MissingScope((*need).to_string()));
        }
    }
    if !sender_ok(env.origin_svc) {
        return Err(VerifyError::WrongOrigin);
    }

    // HMAC verify
    let key = kd.derive_origin_key(env.origin_svc, &env.origin_instance, epoch);
    let mut mac = HmacSha256::new_from_slice(&key).map_err(|_| VerifyError::Crypto)?;
    let bytes = encode_for_mac(env);
    mac.update(&bytes);
    mac.verify_slice(&env.tag).map_err(|_| VerifyError::BadTag)
}

#[derive(Debug, thiserror::Error)]
pub enum VerifyError {
    #[error("expired or not yet valid")]
    Expired,
    #[error("missing scope {0}")]
    MissingScope(String),
    #[error("wrong origin service")]
    WrongOrigin,
    #[error("bad tag")]
    BadTag,
    #[error("crypto error")]
    Crypto,
}

/// Helper to create a fresh random nonce.
pub fn generate_nonce() -> [u8; 16] {
    let mut n = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut n);
    n
}

mod serde_scopes {
    use super::*;
    use serde::{Deserializer, Serializer};

    pub fn serialize<S: Serializer>(v: &SmallVec<[String; 4]>, s: S) -> Result<S::Ok, S::Error> {
        v.as_slice().serialize(s)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<SmallVec<[String; 4]>, D::Error> {
        let v = Vec::<String>::deserialize(d)?;
        Ok(SmallVec::from_vec(v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct TestKD([u8; 32]);
    impl KeyDeriver for TestKD {
        fn derive_origin_key(&self, _svc: &str, _i: &Uuid, _e: u64) -> [u8; 32] { self.0 }
    }

    #[test]
    fn sign_and_verify() {
        let kd = TestKD([7u8; 32]);
        let instance = Uuid::nil();
        let now = OffsetDateTime::now_utc().unix_timestamp();
        let env = Envelope {
            plane: Plane::Node,
            origin_svc: "svc-overlay",
            origin_instance: instance,
            tenant_id: None,
            scopes: SmallVec::from_vec(vec!["overlay:route".into()]),
            nonce: generate_nonce(),
            iat: now - 10,
            exp: now + 60,
            header: (),
            payload: ("ping", 1u8),
            tag: [0u8; 32],
        };
        let env = sign_envelope(&kd, "svc-overlay", &instance, 42, env);
        verify_envelope(&kd, "svc-overlay", 42, &["overlay:route"], &env).unwrap();
        verify_envelope_from_any(&kd, &["svc-overlay", "svc-index"], 42, &["overlay:route"], &env).unwrap();
    }
}
