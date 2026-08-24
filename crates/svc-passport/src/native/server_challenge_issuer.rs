//! RO:WHAT — Private svc-passport runtime that constructs, signs, and immediately strict-verifies one Native Passport V1 service challenge.
//! RO:WHY — Physical M1 requires a real server-authenticated challenge before root/device proofs can become durable authority; contract-only placeholders are insufficient.
//! RO:INTERACTS — KmsClient exact-KID signing, ron-proto PassportChallengeV1 types, ron-auth canonical challenge transcript/strict verifier, UUID-v4 OS randomness, and the future durable challenge/replay runtime.
//! RO:INVARIANTS — trusted network/environment/audience/service identity and TTL are constructor-owned; challenge ID and nonce use fresh native randomness; the KMS key identity is selected before transcript construction; exactly that KID signs exactly once; the completed challenge must strict-cross-verify before return; requested scopes/bindings remain protocol validated and are never silently repaired.
//! RO:METRICS — none yet; the future mounted challenge runtime owns operational counters.
//! RO:CONFIG — caller supplies trusted Native Passport context and a TTL bounded by PASSPORT_CHALLENGE_V1_MAX_TTL_MS.
//! RO:SECURITY — private/unmounted until durable one-time issuance/consumption wraps it; no service private key leaves KMS, no root/device secret enters this layer, and no replay/capability/username/value mutation occurs here.
//! RO:TEST — module tests prove valid signing, fresh challenge material, exact single signing, fail-closed malformed requests, KMS/public-key mismatch rejection, and absence of unrelated authority surfaces.

#![forbid(unsafe_code)]

use crate::kms::client::KmsClient;

use ron_auth::native_passport::{
    canonical_passport_challenge_v1_transcript, verify_passport_challenge_v1_strict,
    PassportChallengeVerificationContextV1,
};
use ron_proto::{
    B3DigestHex, ChallengeIdV1, DeviceIdV1, Ed25519PublicKeyHex, Ed25519SignatureV1,
    NativePassportContextLabelV1, NativePassportScopeV1, PassportChallengePurposeV1,
    PassportChallengeSigningPayloadV1, PassportChallengeV1, PassportIdV1, ServiceKeyIdV1,
    PASSPORT_CHALLENGE_V1_MAX_TTL_MS, PASSPORT_CHALLENGE_V1_VERSION,
};
use thiserror::Error;
use uuid::Uuid;

/// Public operation inputs accepted by the private challenge issuer.
///
/// Trusted service context, service key identity, time, challenge ID, nonce,
/// and expiry are deliberately not caller-controlled fields here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NativePassportServerChallengeIssueRequestV1 {
    pub(super) purpose: PassportChallengePurposeV1,
    pub(super) requested_scopes: Vec<NativePassportScopeV1>,
    pub(super) passport_id: Option<PassportIdV1>,
    pub(super) device_id: Option<DeviceIdV1>,
    pub(super) operation_body_hash: Option<B3DigestHex>,
}

/// Failure returned before a signed challenge may escape this private runtime.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(super) enum NativePassportServerChallengeIssuerError {
    #[error("Native Passport challenge TTL is invalid")]
    InvalidChallengeTtl,

    #[error("Native Passport trusted challenge time is invalid")]
    InvalidTrustedTime,

    #[error("Native Passport challenge expiry overflowed")]
    ChallengeExpiryOverflow,

    #[error("Native Passport KMS signing identity is unavailable")]
    KmsIdentityUnavailable,

    #[error("Native Passport KMS service key ID is invalid")]
    InvalidServiceKeyId,

    #[error("Native Passport KMS service public key is invalid")]
    InvalidServicePublicKey,

    #[error("Native Passport challenge ID generation failed")]
    ChallengeIdGenerationFailed,

    #[error("Native Passport challenge nonce generation failed")]
    ChallengeNonceGenerationFailed,

    #[error("Native Passport challenge payload is invalid")]
    InvalidChallengePayload,

    #[error("Native Passport exact-KID challenge signing failed")]
    KmsSigningFailed,

    #[error("Native Passport challenge signature length is invalid")]
    InvalidSignatureLength,

    #[error("Native Passport signed challenge failed strict cross-verification")]
    StrictCrossVerificationFailed,
}

/// Private server-owned challenge signer.
///
/// This runtime intentionally has no route and no persistence. The next
/// durable challenge/replay layer must become its only production caller.
pub(super) struct NativePassportServerChallengeIssuer<'a> {
    kms: &'a dyn KmsClient,
    network_id: NativePassportContextLabelV1,
    environment: NativePassportContextLabelV1,
    audience: NativePassportContextLabelV1,
    issuing_service_id: NativePassportContextLabelV1,
    challenge_ttl_ms: u64,
}

impl<'a> NativePassportServerChallengeIssuer<'a> {
    pub(super) fn new(
        kms: &'a dyn KmsClient,
        network_id: NativePassportContextLabelV1,
        environment: NativePassportContextLabelV1,
        audience: NativePassportContextLabelV1,
        issuing_service_id: NativePassportContextLabelV1,
        challenge_ttl_ms: u64,
    ) -> Result<Self, NativePassportServerChallengeIssuerError> {
        if challenge_ttl_ms == 0 || challenge_ttl_ms > PASSPORT_CHALLENGE_V1_MAX_TTL_MS {
            return Err(NativePassportServerChallengeIssuerError::InvalidChallengeTtl);
        }

        Ok(Self {
            kms,
            network_id,
            environment,
            audience,
            issuing_service_id,
            challenge_ttl_ms,
        })
    }

    /// Construct and sign exactly one server challenge.
    ///
    /// `now_ms` is trusted server/process time supplied by composition. It is
    /// not accepted from the eventual HTTP request body.
    pub(super) async fn issue(
        &self,
        request: NativePassportServerChallengeIssueRequestV1,
        now_ms: u64,
    ) -> Result<PassportChallengeV1, NativePassportServerChallengeIssuerError> {
        if now_ms == 0 {
            return Err(NativePassportServerChallengeIssuerError::InvalidTrustedTime);
        }

        let expires_at_ms = now_ms
            .checked_add(self.challenge_ttl_ms)
            .ok_or(NativePassportServerChallengeIssuerError::ChallengeExpiryOverflow)?;

        /*
         * Select KID and public key before constructing the signed transcript.
         * sign_with_kid later rejects if rotation makes this selection stale.
         */
        let signing_identity = self
            .kms
            .active_signing_identity()
            .await
            .map_err(|_| NativePassportServerChallengeIssuerError::KmsIdentityUnavailable)?;

        let service_key_id = ServiceKeyIdV1::parse(signing_identity.kid.clone())
            .map_err(|_| NativePassportServerChallengeIssuerError::InvalidServiceKeyId)?;

        let service_public_key =
            Ed25519PublicKeyHex::parse(lower_hex(&signing_identity.public_key))
                .map_err(|_| NativePassportServerChallengeIssuerError::InvalidServicePublicKey)?;

        let payload = PassportChallengeSigningPayloadV1 {
            version: PASSPORT_CHALLENGE_V1_VERSION,
            challenge_id: fresh_challenge_id()?,
            network_id: self.network_id.clone(),
            environment: self.environment.clone(),
            audience: self.audience.clone(),
            issuing_service_id: self.issuing_service_id.clone(),
            service_key_id: service_key_id.clone(),
            purpose: request.purpose,
            requested_scopes: request.requested_scopes,
            passport_id: request.passport_id,
            device_id: request.device_id,
            operation_body_hash: request.operation_body_hash,
            nonce: fresh_challenge_nonce()?,
            issued_at_ms: now_ms,
            expires_at_ms,
        };

        /*
         * ron-auth owns both structural validation and exact canonical bytes.
         * We never sign JSON or a svc-passport-local serialization.
         */
        let transcript = canonical_passport_challenge_v1_transcript(&payload)
            .map_err(|_| NativePassportServerChallengeIssuerError::InvalidChallengePayload)?;

        /*
         * Exactly one KMS signing operation occurs. A concurrent rotation
         * causes sign_with_kid to fail rather than substituting another key.
         */
        let signature = self
            .kms
            .sign_with_kid(service_key_id.as_str(), &transcript)
            .await
            .map_err(|_| NativePassportServerChallengeIssuerError::KmsSigningFailed)?;

        let signature_bytes: [u8; 64] = signature
            .try_into()
            .map_err(|_| NativePassportServerChallengeIssuerError::InvalidSignatureLength)?;

        let challenge = PassportChallengeV1 {
            version: payload.version,
            challenge_id: payload.challenge_id,
            network_id: payload.network_id,
            environment: payload.environment,
            audience: payload.audience,
            issuing_service_id: payload.issuing_service_id,
            service_key_id: payload.service_key_id,
            purpose: payload.purpose,
            requested_scopes: payload.requested_scopes,
            passport_id: payload.passport_id,
            device_id: payload.device_id,
            operation_body_hash: payload.operation_body_hash,
            nonce: payload.nonce,
            issued_at_ms: payload.issued_at_ms,
            expires_at_ms: payload.expires_at_ms,
            service_signature: Ed25519SignatureV1::from_bytes(signature_bytes),
        };

        /*
         * Do not trust success from the KMS alone. Verify the exact challenge
         * we are about to return against the preselected public identity.
         */
        verify_passport_challenge_v1_strict(
            &challenge,
            PassportChallengeVerificationContextV1 {
                trusted_service_public_key: &service_public_key,
                expected_network_id: &self.network_id,
                expected_environment: &self.environment,
                expected_audience: &self.audience,
                expected_issuing_service_id: &self.issuing_service_id,
                expected_service_key_id: &service_key_id,
                now_ms,
                max_clock_skew_ms: 0,
            },
        )
        .map_err(|_| NativePassportServerChallengeIssuerError::StrictCrossVerificationFailed)?;

        Ok(challenge)
    }
}

/// Generate a ChallengeIdV1 from fresh OS-backed UUID-v4 entropy.
///
/// Two independently generated UUID-v4 values are hashed so the wire ID is a
/// real `challenge:v1:b3:<digest>` identifier rather than raw UUID text.
fn fresh_challenge_id() -> Result<ChallengeIdV1, NativePassportServerChallengeIssuerError> {
    let entropy = fresh_native_entropy_32();
    let digest = blake3::hash(&entropy).to_hex().to_string();

    ChallengeIdV1::parse(format!("challenge:v1:b3:{digest}"))
        .map_err(|_| NativePassportServerChallengeIssuerError::ChallengeIdGenerationFailed)
}

/// Generate a fresh independent 32-byte challenge nonce representation.
fn fresh_challenge_nonce() -> Result<B3DigestHex, NativePassportServerChallengeIssuerError> {
    let entropy = fresh_native_entropy_32();
    let digest = blake3::hash(&entropy).to_hex().to_string();

    B3DigestHex::parse("challenge_nonce", digest)
        .map_err(|_| NativePassportServerChallengeIssuerError::ChallengeNonceGenerationFailed)
}

/// Produce native entropy without introducing a JS/client randomness input.
///
/// `uuid` V4 generation uses the crate's OS-backed randomness path. Two
/// independent UUID values provide the entropy material for each BLAKE3
/// challenge digest.
fn fresh_native_entropy_32() -> [u8; 32] {
    let first = Uuid::new_v4();
    let second = Uuid::new_v4();

    let mut output = [0_u8; 32];
    output[..16].copy_from_slice(first.as_bytes());
    output[16..].copy_from_slice(second.as_bytes());
    output
}

fn lower_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    let mut output = String::with_capacity(bytes.len() * 2);

    for byte in bytes {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }

    output
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use async_trait::async_trait;
    use ed25519_dalek::{Signer as _, SigningKey};
    use serde_json::json;

    use crate::kms::client::{KmsClient, KmsSigningIdentity};

    use super::*;

    const HEX_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const HEX_D: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";

    struct TestKms {
        signing_key: SigningKey,
        advertised_public_key: [u8; 32],
        kid: String,
        exact_sign_calls: AtomicUsize,
        legacy_sign_calls: AtomicUsize,
    }

    impl TestKms {
        fn valid() -> Self {
            let signing_key = SigningKey::from_bytes(&[0x44; 32]);
            let advertised_public_key = signing_key.verifying_key().to_bytes();

            Self {
                signing_key,
                advertised_public_key,
                kid: "ed25519/default/v1".to_string(),
                exact_sign_calls: AtomicUsize::new(0),
                legacy_sign_calls: AtomicUsize::new(0),
            }
        }

        fn mismatched_public_identity() -> Self {
            let signing_key = SigningKey::from_bytes(&[0x44; 32]);
            let other_key = SigningKey::from_bytes(&[0x55; 32]);

            Self {
                signing_key,
                advertised_public_key: other_key.verifying_key().to_bytes(),
                kid: "ed25519/default/v1".to_string(),
                exact_sign_calls: AtomicUsize::new(0),
                legacy_sign_calls: AtomicUsize::new(0),
            }
        }

        fn exact_sign_calls(&self) -> usize {
            self.exact_sign_calls.load(Ordering::SeqCst)
        }

        fn legacy_sign_calls(&self) -> usize {
            self.legacy_sign_calls.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl KmsClient for TestKms {
        async fn active_signing_identity(&self) -> anyhow::Result<KmsSigningIdentity> {
            Ok(KmsSigningIdentity {
                kid: self.kid.clone(),
                public_key: self.advertised_public_key,
            })
        }

        async fn sign_with_kid(&self, kid: &str, msg: &[u8]) -> anyhow::Result<Vec<u8>> {
            self.exact_sign_calls.fetch_add(1, Ordering::SeqCst);

            if kid != self.kid {
                return Err(anyhow::anyhow!("unexpected KID"));
            }

            Ok(self.signing_key.sign(msg).to_bytes().to_vec())
        }

        async fn sign(&self, _msg: &[u8]) -> anyhow::Result<(String, Vec<u8>)> {
            self.legacy_sign_calls.fetch_add(1, Ordering::SeqCst);
            Err(anyhow::anyhow!(
                "legacy signing must not be used by challenge issuer"
            ))
        }

        async fn verify(&self, kid: &str, msg: &[u8], sig: &[u8]) -> anyhow::Result<bool> {
            if kid != self.kid {
                return Ok(false);
            }

            let Ok(signature) = ed25519_dalek::Signature::from_slice(sig) else {
                return Ok(false);
            };

            Ok(self
                .signing_key
                .verifying_key()
                .verify_strict(msg, &signature)
                .is_ok())
        }

        async fn public_keys(&self) -> anyhow::Result<serde_json::Value> {
            Ok(json!({
                "alg": "Ed25519",
                "current": self.kid,
                "keys": []
            }))
        }

        async fn rotate(&self) -> anyhow::Result<String> {
            Err(anyhow::anyhow!("rotation is not needed by this fixture"))
        }

        async fn attest(&self) -> anyhow::Result<serde_json::Value> {
            self.public_keys().await
        }
    }

    fn context(value: &str) -> NativePassportContextLabelV1 {
        NativePassportContextLabelV1::parse(value).expect("context")
    }

    fn scope(value: &str) -> NativePassportScopeV1 {
        NativePassportScopeV1::parse(value).expect("scope")
    }

    fn passport_id() -> PassportIdV1 {
        PassportIdV1::parse(format!("passport:v1:main:ed25519:b3:{HEX_A}")).expect("passport id")
    }

    fn operation_body_hash() -> B3DigestHex {
        B3DigestHex::parse("operation_body_hash", HEX_D).expect("operation body hash")
    }

    fn request() -> NativePassportServerChallengeIssueRequestV1 {
        NativePassportServerChallengeIssueRequestV1 {
            purpose: PassportChallengePurposeV1::RegisterRoot,
            requested_scopes: vec![scope("identity.read"), scope("profile.read")],
            passport_id: Some(passport_id()),
            device_id: None,
            operation_body_hash: Some(operation_body_hash()),
        }
    }

    fn issuer<'a>(kms: &'a dyn KmsClient) -> NativePassportServerChallengeIssuer<'a> {
        NativePassportServerChallengeIssuer::new(
            kms,
            context("rustyonions-devnet"),
            context("private-beta"),
            context("svc-passport"),
            context("svc-passport"),
            60_000,
        )
        .expect("issuer")
    }

    #[tokio::test]
    async fn real_private_issuer_signs_once_and_strict_cross_verifies() {
        let kms = TestKms::valid();
        let issuer = issuer(&kms);

        let challenge = issuer
            .issue(request(), 1_000_000)
            .await
            .expect("signed challenge");

        assert_eq!(kms.exact_sign_calls(), 1);
        assert_eq!(kms.legacy_sign_calls(), 0);

        assert_eq!(challenge.version, PASSPORT_CHALLENGE_V1_VERSION);
        assert_eq!(challenge.purpose, PassportChallengePurposeV1::RegisterRoot);
        assert_eq!(challenge.network_id.as_str(), "rustyonions-devnet");
        assert_eq!(challenge.environment.as_str(), "private-beta");
        assert_eq!(challenge.audience.as_str(), "svc-passport");
        assert_eq!(challenge.issuing_service_id.as_str(), "svc-passport");
        assert_eq!(challenge.service_key_id.as_str(), "ed25519/default/v1");
        assert_eq!(challenge.issued_at_ms, 1_000_000);
        assert_eq!(challenge.expires_at_ms, 1_060_000);
    }

    #[tokio::test]
    async fn independent_issuance_uses_fresh_challenge_id_nonce_and_signature() {
        let kms = TestKms::valid();
        let issuer = issuer(&kms);

        let first = issuer
            .issue(request(), 1_000_000)
            .await
            .expect("first challenge");

        let second = issuer
            .issue(request(), 1_000_000)
            .await
            .expect("second challenge");

        assert_ne!(first.challenge_id, second.challenge_id);
        assert_ne!(first.nonce, second.nonce);
        assert_ne!(first.service_signature, second.service_signature);

        assert_eq!(kms.exact_sign_calls(), 2);
        assert_eq!(kms.legacy_sign_calls(), 0);
    }

    #[tokio::test]
    async fn protocol_invalid_request_is_rejected_without_signature_output() {
        let kms = TestKms::valid();
        let issuer = issuer(&kms);

        let mut invalid = request();
        invalid.passport_id = None;

        let result = issuer.issue(invalid, 1_000_000).await;

        assert_eq!(
            result,
            Err(NativePassportServerChallengeIssuerError::InvalidChallengePayload)
        );

        assert_eq!(kms.exact_sign_calls(), 0);
        assert_eq!(kms.legacy_sign_calls(), 0);
    }

    #[tokio::test]
    async fn mismatched_kms_public_identity_fails_after_single_exact_signature() {
        let kms = TestKms::mismatched_public_identity();
        let issuer = issuer(&kms);

        let result = issuer.issue(request(), 1_000_000).await;

        assert_eq!(
            result,
            Err(NativePassportServerChallengeIssuerError::StrictCrossVerificationFailed)
        );

        assert_eq!(kms.exact_sign_calls(), 1);
        assert_eq!(kms.legacy_sign_calls(), 0);
    }

    #[test]
    fn issuer_configuration_rejects_zero_and_excessive_ttl() {
        let kms = TestKms::valid();

        let zero = NativePassportServerChallengeIssuer::new(
            &kms,
            context("rustyonions-devnet"),
            context("private-beta"),
            context("svc-passport"),
            context("svc-passport"),
            0,
        );

        assert!(matches!(
            zero,
            Err(NativePassportServerChallengeIssuerError::InvalidChallengeTtl)
        ));

        let excessive = NativePassportServerChallengeIssuer::new(
            &kms,
            context("rustyonions-devnet"),
            context("private-beta"),
            context("svc-passport"),
            context("svc-passport"),
            PASSPORT_CHALLENGE_V1_MAX_TTL_MS + 1,
        );

        assert!(matches!(
            excessive,
            Err(NativePassportServerChallengeIssuerError::InvalidChallengeTtl)
        ));
    }

    #[test]
    fn private_issuer_source_has_no_route_replay_capability_username_or_value_authority() {
        let source = include_str!("server_challenge_issuer.rs");
        let implementation = source
            .split("\n#[cfg(test)]")
            .next()
            .expect("implementation");

        for forbidden in [
            "Router::",
            ".route(",
            "consume_challenge(",
            "replay_store.write(",
            "issue_capability(",
            "claim_username(",
            "wallet.spend(",
            "ledger.write(",
            "root_private_key:",
            "device_private_key:",
            "raw_pin:",
            "tauri::",
        ] {
            assert!(
                !implementation.contains(forbidden),
                "private challenge issuer gained forbidden authority pattern {forbidden}"
            );
        }
    }
}
