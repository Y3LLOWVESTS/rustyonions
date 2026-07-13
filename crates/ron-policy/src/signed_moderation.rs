//! RO:WHAT — Verifies signed, epoch-bound global moderation-policy snapshots.
//!
//! RO:WHY — `BUILD_PLAN_Z` Phase 10 requires authenticated global deny/tombstone policy lists.
//!
//! RO:INTERACTS — `moderation::Policy`, `ron-kms` Ed25519 verification, and future `macronode` activation.
//!
//! RO:INVARIANTS — Domain-separated bytes; trusted signer only; expiry and rollback rejected.
//!
//! RO:SECURITY — Declarative verification only; no storage, provider, reward, wallet, or ledger mutation.
//!
//! RO:TEST — `tests/signed_moderation_policy.rs`.

#![forbid(unsafe_code)]

use ron_kms::backends::ed25519;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ModerationPolicy;

/// Current signed moderation-policy wire version.
pub const SIGNED_MODERATION_POLICY_VERSION: u16 = 1;

/// Domain separator included in every signed payload.
pub const SIGNED_MODERATION_POLICY_DOMAIN: &str = "rustyonions.moderation-policy.v1";

const ED25519_SIGNATURE_HEX_LEN: usize = 128;
const MAX_SIGNER_ID_BYTES: usize = 128;

/// Strict signed global moderation-policy snapshot.
///
/// `signature_hex` is an Ed25519 signature over [`Self::signing_payload`].
/// The public key is deliberately not embedded as trust material. Callers must
/// provide a separately configured [`TrustedModerationSigner`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedModerationPolicyV1 {
    /// Must equal [`SIGNED_MODERATION_POLICY_VERSION`].
    pub version: u16,
    /// Stable identifier for the configured trusted signer.
    pub signer_id: String,
    /// Strictly increasing global policy epoch.
    pub epoch: u64,
    /// Earliest Unix second at which this snapshot is valid.
    pub issued_at_unix_s: u64,
    /// First Unix second at which this snapshot is expired.
    pub expires_at_unix_s: u64,
    /// Exact-B3 moderation policy covered by the signature.
    pub policy: ModerationPolicy,
    /// Canonical lowercase hexadecimal Ed25519 signature.
    pub signature_hex: String,
}

impl SignedModerationPolicyV1 {
    /// Produce deterministic, domain-separated bytes covered by the signature.
    ///
    /// The signature itself is excluded. Struct field order and the
    /// moderation policy's ordered sets make the JSON byte representation
    /// deterministic for this version.
    ///
    /// # Errors
    ///
    /// Returns a JSON serialization error if the signable payload cannot be
    /// encoded.
    pub fn signing_payload(&self) -> Result<Vec<u8>, serde_json::Error> {
        let payload = SignableModerationPolicyV1 {
            domain: SIGNED_MODERATION_POLICY_DOMAIN,
            version: self.version,
            signer_id: &self.signer_id,
            epoch: self.epoch,
            issued_at_unix_s: self.issued_at_unix_s,
            expires_at_unix_s: self.expires_at_unix_s,
            policy: &self.policy,
        };

        serde_json::to_vec(&payload)
    }
}

#[derive(Serialize)]
struct SignableModerationPolicyV1<'a> {
    domain: &'static str,
    version: u16,
    signer_id: &'a str,
    epoch: u64,
    issued_at_unix_s: u64,
    expires_at_unix_s: u64,
    policy: &'a ModerationPolicy,
}

/// Separately configured trust anchor for one moderation-policy signer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustedModerationSigner {
    signer_id: String,
    public_key: [u8; 32],
}

impl TrustedModerationSigner {
    /// Construct a trusted signer configuration.
    ///
    /// # Errors
    ///
    /// Returns [`SignedModerationPolicyError::InvalidSignerId`] when the ID is
    /// empty, oversized, or contains characters outside the stable ASCII
    /// signer-ID alphabet.
    pub fn new(
        signer_id: impl Into<String>,
        public_key: [u8; 32],
    ) -> Result<Self, SignedModerationPolicyError> {
        let signer_id = signer_id.into();
        validate_signer_id(&signer_id)?;

        Ok(Self {
            signer_id,
            public_key,
        })
    }

    /// Return the stable trusted signer identifier.
    #[must_use]
    pub fn signer_id(&self) -> &str {
        &self.signer_id
    }

    /// Return the trusted Ed25519 public key.
    #[must_use]
    pub const fn public_key(&self) -> &[u8; 32] {
        &self.public_key
    }
}

/// Successfully verified signed moderation snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedModerationPolicy {
    /// Authenticated moderation policy.
    pub policy: ModerationPolicy,
    /// Trusted signer that authenticated the snapshot.
    pub signer_id: String,
    /// Accepted strictly increasing epoch.
    pub epoch: u64,
    /// Snapshot validity start.
    pub issued_at_unix_s: u64,
    /// Snapshot expiry.
    pub expires_at_unix_s: u64,
}

/// Signed moderation-policy rejection.
#[derive(Debug, Error)]
pub enum SignedModerationPolicyError {
    /// Unsupported signed snapshot version.
    #[error("unsupported signed moderation policy version: expected {expected}, found {found}")]
    UnsupportedVersion {
        /// Supported version.
        expected: u16,
        /// Supplied version.
        found: u16,
    },

    /// Signer identifier is not canonical.
    #[error("invalid moderation policy signer identifier")]
    InvalidSignerId,

    /// Snapshot signer does not match configured trust.
    #[error("moderation policy signer is not trusted")]
    UntrustedSigner,

    /// Epoch zero is not accepted.
    #[error("moderation policy epoch must be greater than zero")]
    InvalidEpoch,

    /// Expiration must be strictly after issue time.
    #[error("moderation policy validity window is invalid")]
    InvalidValidityWindow,

    /// Snapshot has not reached its issue time.
    #[error("moderation policy is not yet valid: now={now_unix_s}, issued_at={issued_at_unix_s}")]
    NotYetValid {
        /// Current Unix time.
        now_unix_s: u64,
        /// Snapshot issue time.
        issued_at_unix_s: u64,
    },

    /// Snapshot has expired.
    #[error("moderation policy is expired: now={now_unix_s}, expires_at={expires_at_unix_s}")]
    Expired {
        /// Current Unix time.
        now_unix_s: u64,
        /// Snapshot expiration time.
        expires_at_unix_s: u64,
    },

    /// Epoch does not advance beyond the previously accepted snapshot.
    #[error(
        "moderation policy rollback rejected: epoch={epoch}, last_accepted_epoch={last_accepted_epoch}"
    )]
    Rollback {
        /// Supplied epoch.
        epoch: u64,
        /// Most recently accepted epoch.
        last_accepted_epoch: u64,
    },

    /// Signature is not canonical lowercase Ed25519 hex.
    #[error("moderation policy signature must be 128 lowercase hexadecimal characters")]
    InvalidSignatureEncoding,

    /// Signature did not authenticate the deterministic payload.
    #[error("moderation policy signature verification failed")]
    SignatureVerificationFailed,

    /// Deterministic payload serialization failed.
    #[error("failed to encode moderation policy signing payload: {0}")]
    PayloadEncoding(#[source] serde_json::Error),
}

/// Verify one signed moderation-policy snapshot.
///
/// `last_accepted_epoch` provides rollback protection. Passing `Some(epoch)`
/// requires the incoming snapshot to have a strictly larger epoch.
///
/// # Errors
///
/// Returns [`SignedModerationPolicyError`] for malformed identity, unsupported
/// version, invalid validity time, rollback, malformed signature, or failed
/// cryptographic verification.
pub fn verify_signed_moderation_policy(
    snapshot: &SignedModerationPolicyV1,
    trusted_signer: &TrustedModerationSigner,
    now_unix_s: u64,
    last_accepted_epoch: Option<u64>,
) -> Result<VerifiedModerationPolicy, SignedModerationPolicyError> {
    if snapshot.version != SIGNED_MODERATION_POLICY_VERSION {
        return Err(SignedModerationPolicyError::UnsupportedVersion {
            expected: SIGNED_MODERATION_POLICY_VERSION,
            found: snapshot.version,
        });
    }

    validate_signer_id(&snapshot.signer_id)?;

    if snapshot.signer_id != trusted_signer.signer_id {
        return Err(SignedModerationPolicyError::UntrustedSigner);
    }

    if snapshot.epoch == 0 {
        return Err(SignedModerationPolicyError::InvalidEpoch);
    }

    if snapshot.expires_at_unix_s <= snapshot.issued_at_unix_s {
        return Err(SignedModerationPolicyError::InvalidValidityWindow);
    }

    if now_unix_s < snapshot.issued_at_unix_s {
        return Err(SignedModerationPolicyError::NotYetValid {
            now_unix_s,
            issued_at_unix_s: snapshot.issued_at_unix_s,
        });
    }

    if now_unix_s >= snapshot.expires_at_unix_s {
        return Err(SignedModerationPolicyError::Expired {
            now_unix_s,
            expires_at_unix_s: snapshot.expires_at_unix_s,
        });
    }

    if let Some(last_accepted_epoch) = last_accepted_epoch {
        if snapshot.epoch <= last_accepted_epoch {
            return Err(SignedModerationPolicyError::Rollback {
                epoch: snapshot.epoch,
                last_accepted_epoch,
            });
        }
    }

    let signature = decode_signature_hex(&snapshot.signature_hex)?;
    let payload = snapshot
        .signing_payload()
        .map_err(SignedModerationPolicyError::PayloadEncoding)?;

    if !ed25519::verify(trusted_signer.public_key(), &payload, &signature) {
        return Err(SignedModerationPolicyError::SignatureVerificationFailed);
    }

    Ok(VerifiedModerationPolicy {
        policy: snapshot.policy.clone(),
        signer_id: snapshot.signer_id.clone(),
        epoch: snapshot.epoch,
        issued_at_unix_s: snapshot.issued_at_unix_s,
        expires_at_unix_s: snapshot.expires_at_unix_s,
    })
}

/// Encode an Ed25519 signature as canonical lowercase hexadecimal.
#[must_use]
pub fn encode_ed25519_signature(signature: &[u8; 64]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let mut encoded = String::with_capacity(ED25519_SIGNATURE_HEX_LEN);

    for &byte in signature {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }

    encoded
}

fn validate_signer_id(signer_id: &str) -> Result<(), SignedModerationPolicyError> {
    let valid = !signer_id.is_empty()
        && signer_id.len() <= MAX_SIGNER_ID_BYTES
        && signer_id.bytes().all(|byte| {
            matches!(
                byte,
                b'a'..=b'z'
                    | b'A'..=b'Z'
                    | b'0'..=b'9'
                    | b'-'
                    | b'_'
                    | b'.'
                    | b':'
            )
        });

    if valid {
        Ok(())
    } else {
        Err(SignedModerationPolicyError::InvalidSignerId)
    }
}

fn decode_signature_hex(raw: &str) -> Result<[u8; 64], SignedModerationPolicyError> {
    if raw.len() != ED25519_SIGNATURE_HEX_LEN {
        return Err(SignedModerationPolicyError::InvalidSignatureEncoding);
    }

    let mut decoded = [0_u8; 64];

    for (index, pair) in raw.as_bytes().chunks_exact(2).enumerate() {
        let high = decode_lower_hex(pair[0])
            .ok_or(SignedModerationPolicyError::InvalidSignatureEncoding)?;
        let low = decode_lower_hex(pair[1])
            .ok_or(SignedModerationPolicyError::InvalidSignatureEncoding)?;

        decoded[index] = (high << 4) | low;
    }

    Ok(decoded)
}

const fn decode_lower_hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}
