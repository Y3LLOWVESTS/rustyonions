//! RO:WHAT — Tests signed global moderation-policy validation and rollback protection.
//! RO:WHY — BUILD_PLAN_Z Phase 10 requires authenticated, expiring global policy snapshots.
//! RO:INTERACTS — ron-policy signed_moderation, moderation policy, ron-kms Ed25519.
//! RO:INVARIANTS — tamper, untrusted signer, expiry, and rollback all fail closed.
//! RO:SECURITY — test keys only; no runtime key custody or economic authority.
//! RO:TEST — cargo test -p ron-policy --test signed_moderation_policy.

use ron_kms::backends::ed25519;
use ron_policy::{
    encode_ed25519_signature, verify_signed_moderation_policy, B3Id, ModerationReasonCode,
    SignedModerationPolicyError, SignedModerationPolicyV1, TrustedModerationSigner,
    SIGNED_MODERATION_POLICY_VERSION,
};

const OBJECT: &str = "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn signed_snapshot(
    epoch: u64,
    issued_at_unix_s: u64,
    expires_at_unix_s: u64,
) -> (SignedModerationPolicyV1, TrustedModerationSigner) {
    let (public_key, secret_seed) = ed25519::generate();
    let trusted = TrustedModerationSigner::new("global-policy-root-1", public_key)
        .expect("trusted signer should be canonical");

    let object: B3Id = OBJECT.parse().expect("OBJECT should be canonical");

    let mut policy = ron_policy::ModerationPolicy::default();
    assert!(policy.insert_global_deny(object));

    let mut snapshot = SignedModerationPolicyV1 {
        version: SIGNED_MODERATION_POLICY_VERSION,
        signer_id: trusted.signer_id().to_owned(),
        epoch,
        issued_at_unix_s,
        expires_at_unix_s,
        policy,
        signature_hex: String::new(),
    };

    let payload = snapshot
        .signing_payload()
        .expect("signing payload should encode");
    let signature = ed25519::sign(&secret_seed, &payload);
    snapshot.signature_hex = encode_ed25519_signature(&signature);

    (snapshot, trusted)
}

#[test]
fn valid_signed_snapshot_authenticates_exact_policy() {
    let (snapshot, trusted) = signed_snapshot(7, 1_000, 2_000);

    let verified = verify_signed_moderation_policy(&snapshot, &trusted, 1_500, Some(6))
        .expect("valid signed policy should verify");

    assert_eq!(verified.signer_id, "global-policy-root-1");
    assert_eq!(verified.epoch, 7);
    assert_eq!(verified.issued_at_unix_s, 1_000);
    assert_eq!(verified.expires_at_unix_s, 2_000);

    let object: B3Id = OBJECT.parse().expect("OBJECT should be canonical");

    assert_eq!(
        verified.policy.evaluate(&object).reason,
        ModerationReasonCode::GlobalDeny
    );
}

#[test]
fn policy_tampering_after_signing_is_rejected() {
    let (mut snapshot, trusted) = signed_snapshot(7, 1_000, 2_000);

    let second: B3Id = concat!(
        "b3:",
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
    )
    .parse()
    .expect("second ID should be canonical");

    assert!(snapshot.policy.insert_global_deny(second));

    let error = verify_signed_moderation_policy(&snapshot, &trusted, 1_500, None)
        .expect_err("tampered policy must fail");

    assert!(matches!(
        error,
        SignedModerationPolicyError::SignatureVerificationFailed
    ));
}

#[test]
fn signer_mismatch_is_rejected_before_signature_use() {
    let (snapshot, trusted) = signed_snapshot(7, 1_000, 2_000);

    let other = TrustedModerationSigner::new("different-policy-root", *trusted.public_key())
        .expect("alternate signer ID should be canonical");

    let error = verify_signed_moderation_policy(&snapshot, &other, 1_500, None)
        .expect_err("untrusted signer must fail");

    assert!(matches!(
        error,
        SignedModerationPolicyError::UntrustedSigner
    ));
}

#[test]
fn validity_window_is_enforced_at_both_boundaries() {
    let (snapshot, trusted) = signed_snapshot(7, 1_000, 2_000);

    let early = verify_signed_moderation_policy(&snapshot, &trusted, 999, None)
        .expect_err("future policy must fail");

    assert!(matches!(
        early,
        SignedModerationPolicyError::NotYetValid { .. }
    ));

    let expired = verify_signed_moderation_policy(&snapshot, &trusted, 2_000, None)
        .expect_err("policy expires at exact boundary");

    assert!(matches!(
        expired,
        SignedModerationPolicyError::Expired { .. }
    ));

    let (invalid_window, invalid_trusted) = signed_snapshot(8, 2_000, 2_000);

    let invalid = verify_signed_moderation_policy(&invalid_window, &invalid_trusted, 2_000, None)
        .expect_err("zero-length window must fail");

    assert!(matches!(
        invalid,
        SignedModerationPolicyError::InvalidValidityWindow
    ));
}

#[test]
fn equal_or_older_epoch_is_rejected_as_rollback() {
    let (snapshot, trusted) = signed_snapshot(7, 1_000, 2_000);

    for last_accepted_epoch in [7, 8] {
        let error =
            verify_signed_moderation_policy(&snapshot, &trusted, 1_500, Some(last_accepted_epoch))
                .expect_err("non-advancing epoch must fail");

        assert!(matches!(
            error,
            SignedModerationPolicyError::Rollback { .. }
        ));
    }
}

#[test]
fn signature_encoding_and_json_shape_are_strict() {
    let (mut snapshot, trusted) = signed_snapshot(7, 1_000, 2_000);

    snapshot.signature_hex.make_ascii_uppercase();

    let uppercase = verify_signed_moderation_policy(&snapshot, &trusted, 1_500, None)
        .expect_err("uppercase signature must fail");

    assert!(matches!(
        uppercase,
        SignedModerationPolicyError::InvalidSignatureEncoding
    ));

    let mut value = serde_json::to_value(&snapshot).expect("snapshot JSON");

    value
        .as_object_mut()
        .expect("snapshot should be an object")
        .insert("unknown".to_owned(), serde_json::json!(true));

    assert!(serde_json::from_value::<SignedModerationPolicyV1>(value).is_err());
}

#[test]
fn signing_payload_is_stable_across_json_round_trip() {
    let (snapshot, _) = signed_snapshot(7, 1_000, 2_000);

    let before = snapshot.signing_payload().expect("payload should encode");

    let encoded = serde_json::to_vec(&snapshot).expect("snapshot JSON");
    let decoded: SignedModerationPolicyV1 =
        serde_json::from_slice(&encoded).expect("snapshot should decode");

    let after = decoded
        .signing_payload()
        .expect("decoded payload should encode");

    assert_eq!(before, after);
}
