//! RO:WHAT — Physical M1 nonphysical cross-verification tests for svc-passport root-signed DeviceAuthorizationV1.
//! RO:WHY — Prove the real svc-passport root derivation/signing path produces exactly what ron-auth accepts before any physical Passport root is allowed to sign.
//! RO:INTERACTS — svc-passport NativeSecretBytes/root/device identity derivation and signer, ron-proto authorization DTOs, and ron-auth strict verification.
//! RO:INVARIANTS — deterministic fake root only; wrong Passport binding, wrong Device binding, and malformed root seed fail before producing an accepted authorization; identical inputs yield identical Ed25519 signatures.
//! RO:METRICS — prints only public authorization transcript/signature evidence labels; no secret material.
//! RO:CONFIG — deterministic nonphysical test material only.
//! RO:SECURITY — no physical recovery factor, recovery phrase, PIN, vault, Keychain, Tauri, HTTP, capability, username, wallet, or ledger mutation.
//! RO:TEST — cargo test -p svc-passport --no-default-features --features native-passport --test physical_m1_native_device_authorization_signing.

#[cfg(not(feature = "native-passport"))]
#[test]
fn physical_m1_root_device_authorization_signer_is_feature_gated() {}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use ron_auth::native_passport::{
        device_authorization_v1_transcript_b3_hex, verify_device_authorization_v1_strict,
        DeviceAuthorizationVerificationContextV1,
    };
    use ron_proto::{
        DeviceAuthorizationNonceV1, DeviceAuthorizationScopeCeilingV1,
        DeviceAuthorizationSigningPayloadV1, DeviceClassV1, DeviceIdV1 as ProtoDeviceIdV1,
        Ed25519PublicKeyHex as ProtoEd25519PublicKeyHex, NativePassportContextLabelV1,
        NativePassportScopeV1, PassportIdV1 as ProtoPassportIdV1, DEVICE_AUTHORIZATION_V1_VERSION,
    };
    use svc_passport::native::{
        derive_native_device_public_identity_v1, derive_native_root_public_identity_v1,
        sign_native_device_authorization_v1, NativeDeviceAuthorizationSigningError,
        NativeSecretBytes, PHYSICAL_M1_DEVICE_AUTHORIZATION_SIGNING_LABEL,
    };

    const ISSUED_AT_MS: u64 = 1_720_000_000_000;
    const EXPIRES_AT_MS: u64 = 1_720_000_060_000;
    const VERIFY_AT_MS: u64 = 1_720_000_010_000;
    const ROOT_KEY_EPOCH: u64 = 7;

    fn root_seed() -> NativeSecretBytes {
        NativeSecretBytes::new(vec![0x31; 64]).expect("nonphysical root seed")
    }

    fn device_seed() -> NativeSecretBytes {
        NativeSecretBytes::new(vec![0x42; 32]).expect("nonphysical device seed")
    }

    fn network() -> NativePassportContextLabelV1 {
        NativePassportContextLabelV1::parse("rustyonions-devnet").unwrap()
    }

    fn environment() -> NativePassportContextLabelV1 {
        NativePassportContextLabelV1::parse("local-dev").unwrap()
    }

    fn scope(value: &str) -> NativePassportScopeV1 {
        NativePassportScopeV1::parse(value).unwrap()
    }

    fn scope_ceiling() -> DeviceAuthorizationScopeCeilingV1 {
        DeviceAuthorizationScopeCeilingV1::new(vec![
            scope("capability.revoke_self"),
            scope("catalog.read"),
            scope("content.read"),
            scope("identity.read"),
        ])
        .unwrap()
    }

    fn signing_payload(root_seed: &NativeSecretBytes) -> DeviceAuthorizationSigningPayloadV1 {
        let root_identity =
            derive_native_root_public_identity_v1(root_seed).expect("root public identity");

        let device_identity = derive_native_device_public_identity_v1(&device_seed())
            .expect("device public identity");

        DeviceAuthorizationSigningPayloadV1 {
            version: DEVICE_AUTHORIZATION_V1_VERSION,
            network_id: network(),
            environment: environment(),
            passport_id: ProtoPassportIdV1::parse(root_identity.passport_id.as_str()).unwrap(),
            root_key_epoch: ROOT_KEY_EPOCH,
            device_id: ProtoDeviceIdV1::parse(device_identity.device_id.as_str()).unwrap(),
            device_public_key: ProtoEd25519PublicKeyHex::parse(
                device_identity.device_public_key.as_str(),
            )
            .unwrap(),
            device_class: DeviceClassV1::RootAdminDesktop,
            authorized_scope_ceiling: scope_ceiling(),
            authorization_nonce: DeviceAuthorizationNonceV1::from_bytes([
                0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
                0x0e, 0x0f,
            ]),
            issued_at_ms: ISSUED_AT_MS,
            expires_at_ms: Some(EXPIRES_AT_MS),
        }
    }

    #[test]
    fn physical_m1_nonphysical_root_signature_cross_verifies_through_ron_auth() {
        assert_eq!(
            PHYSICAL_M1_DEVICE_AUTHORIZATION_SIGNING_LABEL,
            "PHYSICAL_M1_NATIVE_DEVICE_AUTHORIZATION_SIGNING_V1",
        );

        let root_seed = root_seed();
        let payload = signing_payload(&root_seed);

        let expected_transcript_b3 = device_authorization_v1_transcript_b3_hex(&payload).unwrap();

        let authorization =
            sign_native_device_authorization_v1(&root_seed, payload).expect("root signs");

        let root_identity =
            derive_native_root_public_identity_v1(&root_seed).expect("trusted root descriptor");

        let trusted_passport_id =
            ProtoPassportIdV1::parse(root_identity.passport_id.as_str()).unwrap();

        let trusted_root_public_key =
            ProtoEd25519PublicKeyHex::parse(root_identity.root_public_key.as_str()).unwrap();

        let network = network();
        let environment = environment();

        verify_device_authorization_v1_strict(
            &authorization,
            DeviceAuthorizationVerificationContextV1 {
                trusted_passport_id: &trusted_passport_id,
                trusted_root_public_key: &trusted_root_public_key,
                trusted_root_key_epoch: ROOT_KEY_EPOCH,
                expected_network_id: &network,
                expected_environment: &environment,
                now_ms: VERIFY_AT_MS,
                max_clock_skew_ms: 0,
            },
        )
        .expect("svc-passport signature must verify through ron-auth");

        let actual_transcript_b3 =
            device_authorization_v1_transcript_b3_hex(&authorization.signing_payload()).unwrap();

        assert_eq!(actual_transcript_b3, expected_transcript_b3);

        println!("PHYSICAL_M1_NONPHYSICAL_ROOT_SIGNER_CROSS_VERIFY=GREEN");
        println!("PHYSICAL_M1_NONPHYSICAL_TRANSCRIPT_B3={actual_transcript_b3}");
    }

    #[test]
    fn physical_m1_root_signing_is_deterministic_for_identical_inputs() {
        let root_seed = root_seed();
        let payload = signing_payload(&root_seed);

        let first = sign_native_device_authorization_v1(&root_seed, payload.clone()).unwrap();

        let second = sign_native_device_authorization_v1(&root_seed, payload).unwrap();

        assert_eq!(
            first.root_signature.as_bytes(),
            second.root_signature.as_bytes(),
        );
    }

    #[test]
    fn physical_m1_wrong_passport_binding_is_rejected_before_accepted_signature() {
        let root_seed = root_seed();
        let mut payload = signing_payload(&root_seed);

        payload.passport_id = ProtoPassportIdV1::parse(
            "passport:v1:main:ed25519:b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        )
        .unwrap();

        assert_eq!(
            sign_native_device_authorization_v1(&root_seed, payload),
            Err(NativeDeviceAuthorizationSigningError::PassportBindingMismatch),
        );
    }

    #[test]
    fn physical_m1_wrong_device_binding_is_rejected_before_accepted_signature() {
        let root_seed = root_seed();
        let mut payload = signing_payload(&root_seed);

        payload.device_id = ProtoDeviceIdV1::parse(
            "device:v1:ed25519:b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        )
        .unwrap();

        assert_eq!(
            sign_native_device_authorization_v1(&root_seed, payload),
            Err(NativeDeviceAuthorizationSigningError::DeviceIdBindingMismatch),
        );
    }

    #[test]
    fn physical_m1_invalid_root_seed_length_fails_closed() {
        let valid_root = root_seed();
        let payload = signing_payload(&valid_root);

        let short_root =
            NativeSecretBytes::new(vec![0x31; 63]).expect("bounded but invalid root seed length");

        assert_eq!(
            sign_native_device_authorization_v1(&short_root, payload),
            Err(NativeDeviceAuthorizationSigningError::RootIdentityDerivationFailed),
        );
    }
}
