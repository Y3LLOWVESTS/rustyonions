//! RO:WHAT — Deterministic Physical M1 tests for recovery-factor-to-DeviceAuthorizationV1 root signing.
//! RO:WHY — Prove CrabLink can later request a root-confirmed device authorization without receiving the BIP-39 seed or duplicating root derivation outside svc-passport.
//! RO:INTERACTS — recovery identity derivation, purpose-specific recovery-factor signer, device identity derivation, ron-proto authorization DTOs, and ron-auth strict verifier.
//! RO:INVARIANTS — deterministic nonphysical recovery/device material only; Passport and Device-ID binding failures reject; the returned authorization verifies against the recovery-derived trusted root.
//! RO:METRICS — prints redacted green markers only.
//! RO:CONFIG — deterministic nonphysical fixtures only.
//! RO:SECURITY — no physical Passport, PIN, Keychain, vault, Tauri, network, capability, username, wallet, or ledger mutation; no secret values are printed.
//! RO:TEST — cargo test -p svc-passport --no-default-features --features native-passport --test physical_m1_native_recovery_device_authorization_signing.

#[cfg(not(feature = "native-passport"))]
#[test]
fn physical_m1_recovery_device_authorization_signer_is_feature_gated() {}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use ron_auth::native_passport::{
        verify_device_authorization_v1_strict, DeviceAuthorizationVerificationContextV1,
    };
    use ron_proto::{
        DeviceAuthorizationNonceV1, DeviceAuthorizationScopeCeilingV1,
        DeviceAuthorizationSigningPayloadV1, DeviceClassV1, DeviceIdV1 as ProtoDeviceIdV1,
        Ed25519PublicKeyHex as ProtoEd25519PublicKeyHex, NativePassportContextLabelV1,
        NativePassportScopeV1, PassportIdV1 as ProtoPassportIdV1, DEVICE_AUTHORIZATION_V1_VERSION,
    };
    use svc_passport::native::{
        derive_native_device_public_identity_v1, derive_native_recovery_public_identity_v1,
        sign_native_recovery_device_authorization_v1, NativeDeviceAuthorizationSigningError,
        NativeRecoveryDeviceAuthorizationSigningError, NativeSecretBytes,
        PHYSICAL_M1_RECOVERY_DEVICE_AUTHORIZATION_SIGNING_LABEL,
    };

    const ISSUED_AT_MS: u64 = 1_720_000_000_000;
    const EXPIRES_AT_MS: u64 = 1_720_000_060_000;
    const VERIFY_AT_MS: u64 = 1_720_000_010_000;
    const ROOT_KEY_EPOCH: u64 = 7;

    fn recovery_factor() -> NativeSecretBytes {
        NativeSecretBytes::new(vec![0x22; 32]).expect("nonphysical recovery factor")
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

    fn signing_payload(recovery_factor: &NativeSecretBytes) -> DeviceAuthorizationSigningPayloadV1 {
        let root_identity = derive_native_recovery_public_identity_v1(recovery_factor)
            .expect("recovery-derived public root identity");

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
    fn physical_m1_recovery_factor_signature_cross_verifies_through_ron_auth() {
        assert_eq!(
            PHYSICAL_M1_RECOVERY_DEVICE_AUTHORIZATION_SIGNING_LABEL,
            "PHYSICAL_M1_NATIVE_RECOVERY_DEVICE_AUTHORIZATION_SIGNING_V1",
        );

        let recovery_factor = recovery_factor();
        let payload = signing_payload(&recovery_factor);

        let authorization = sign_native_recovery_device_authorization_v1(&recovery_factor, payload)
            .expect("recovery factor signs canonical DeviceAuthorizationV1");

        let trusted_root = derive_native_recovery_public_identity_v1(&recovery_factor)
            .expect("trusted recovery-derived root");

        let trusted_passport_id =
            ProtoPassportIdV1::parse(trusted_root.passport_id.as_str()).unwrap();

        let trusted_root_public_key =
            ProtoEd25519PublicKeyHex::parse(trusted_root.root_public_key.as_str()).unwrap();

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
        .expect("recovery-factor signature must verify through ron-auth");

        println!("RECOVERY_FACTOR_DEVICE_AUTHORIZATION_CROSS_VERIFY=GREEN");
    }

    #[test]
    fn physical_m1_recovery_factor_signing_is_deterministic_for_identical_inputs() {
        let first_factor = recovery_factor();
        let first_payload = signing_payload(&first_factor);

        let first = sign_native_recovery_device_authorization_v1(&first_factor, first_payload)
            .expect("first deterministic authorization");

        let second_factor = recovery_factor();
        let second_payload = signing_payload(&second_factor);

        let second = sign_native_recovery_device_authorization_v1(&second_factor, second_payload)
            .expect("second deterministic authorization");

        assert_eq!(
            serde_json::to_value(first).unwrap(),
            serde_json::to_value(second).unwrap(),
        );
    }

    #[test]
    fn physical_m1_recovery_factor_signer_rejects_wrong_passport_binding() {
        let recovery_factor = recovery_factor();
        let mut payload = signing_payload(&recovery_factor);

        payload.passport_id = ProtoPassportIdV1::parse(
            "passport:v1:main:ed25519:b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        )
        .unwrap();

        let error = sign_native_recovery_device_authorization_v1(&recovery_factor, payload)
            .expect_err("wrong Passport binding must reject");

        assert!(matches!(
            error,
            NativeRecoveryDeviceAuthorizationSigningError::DeviceAuthorizationSigning(
                NativeDeviceAuthorizationSigningError::PassportBindingMismatch
            )
        ));
    }

    #[test]
    fn physical_m1_recovery_factor_signer_rejects_wrong_device_id_binding() {
        let recovery_factor = recovery_factor();
        let mut payload = signing_payload(&recovery_factor);

        payload.device_id = ProtoDeviceIdV1::parse(
            "device:v1:ed25519:b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        )
        .unwrap();

        let error = sign_native_recovery_device_authorization_v1(&recovery_factor, payload)
            .expect_err("wrong Device ID binding must reject");

        assert!(matches!(
            error,
            NativeRecoveryDeviceAuthorizationSigningError::DeviceAuthorizationSigning(
                NativeDeviceAuthorizationSigningError::DeviceIdBindingMismatch
            )
        ));
    }

    #[test]
    fn physical_m1_recovery_factor_signer_rejects_malformed_recovery_factor() {
        let valid_factor = recovery_factor();
        let payload = signing_payload(&valid_factor);

        let malformed = NativeSecretBytes::new(vec![0x22; 31]).expect("bounded malformed fixture");

        let error = sign_native_recovery_device_authorization_v1(&malformed, payload)
            .expect_err("malformed recovery factor must reject");

        assert!(matches!(
            error,
            NativeRecoveryDeviceAuthorizationSigningError::RecoveryFactorInvalid
        ));
    }
}
