//! RO:WHAT — Physical-M1 service KMS tests for exact challenge signing-key preselection.
//! RO:WHY — A signed Passport challenge must bind the same KID/public key that performs its Ed25519 signature even if rotation occurs between operations.
//! RO:INTERACTS — svc-passport KmsClient and development DevKms.
//! RO:INVARIANTS — selected KID/public key agree; exact-KID signing verifies; rotation changes identity atomically; stale KID signing rejects; historical verification remains valid.
//! RO:METRICS — none.
//! RO:CONFIG — test uses the existing `dev-kms` feature.
//! RO:SECURITY — only public KID/key/signature evidence leaves KMS; no challenge issuance, replay mutation, route, capability, username, wallet, or ledger authority.
//! RO:TEST — this file.

#[cfg(not(feature = "dev-kms"))]
#[test]
fn service_challenge_kms_identity_is_feature_gated() {
    const {
        assert!(!cfg!(feature = "dev-kms"));
    }
}

#[cfg(feature = "dev-kms")]
mod feature_tests {
    use svc_passport::kms::client::{DevKms, KmsClient};

    #[tokio::test]
    async fn selected_identity_and_exact_kid_signature_are_consistent() {
        let kms = DevKms::new();

        let identity = kms
            .active_signing_identity()
            .await
            .expect("active signing identity");

        assert_eq!(identity.kid, "ed25519/default/v1");

        assert_ne!(identity.public_key, [0_u8; 32]);

        let message = b"native-passport-service-challenge";

        let signature = kms
            .sign_with_kid(&identity.kid, message)
            .await
            .expect("exact KID signature");

        assert!(kms
            .verify(&identity.kid, message, &signature,)
            .await
            .expect("signature verification"));

        let (legacy_kid, legacy_signature) = kms.sign(message).await.expect("legacy sign");

        assert_eq!(legacy_kid, identity.kid);

        assert!(kms
            .verify(&legacy_kid, message, &legacy_signature,)
            .await
            .expect("legacy signature verification"));
    }

    #[tokio::test]
    async fn rotation_rejects_stale_preselected_kid_and_preserves_history() {
        let kms = DevKms::new();

        let before = kms
            .active_signing_identity()
            .await
            .expect("identity before rotation");

        let message = b"rotation-boundary";

        let before_signature = kms
            .sign_with_kid(&before.kid, message)
            .await
            .expect("signature before rotation");

        let rotated = kms.rotate().await.expect("KMS rotation");

        assert_eq!(rotated, "ed25519/default/v2");

        let after = kms
            .active_signing_identity()
            .await
            .expect("identity after rotation");

        assert_eq!(after.kid, rotated);

        assert_ne!(after.public_key, before.public_key);

        assert!(kms.sign_with_kid(&before.kid, message,).await.is_err());

        let after_signature = kms
            .sign_with_kid(&after.kid, message)
            .await
            .expect("signature after rotation");

        assert!(kms
            .verify(&after.kid, message, &after_signature,)
            .await
            .expect("current signature verification"));

        assert!(kms
            .verify(&before.kid, message, &before_signature,)
            .await
            .expect("historical signature verification"));
    }
}
