#[cfg(feature = "native-passport")]
use svc_passport::native::{
    native_passport_feature_posture, NativePassportSurface, NATIVE_PASSPORT_FEATURE_NAME,
    NATIVE_PASSPORT_PHASE1A_LABEL, PHASE1A_ENABLED_SURFACES, PHASE1A_FORBIDDEN_BEHAVIORS,
};

#[cfg(not(feature = "native-passport"))]
#[test]
fn native_passport_module_is_not_exercised_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not enable native-passport feature implicitly"
    );
}

#[cfg(feature = "native-passport")]
#[test]
fn native_passport_feature_gate_exposes_phase1a_posture_only() {
    let posture = native_passport_feature_posture();

    assert_eq!(posture.owner, "svc-passport");
    assert_eq!(posture.feature_name, NATIVE_PASSPORT_FEATURE_NAME);
    assert_eq!(
        posture.phase_label,
        "NATIVE_PASSPORT_PHASE1A_FEATURE_ISOLATION_NATIVE_MODULE_GATES"
    );
    assert_eq!(
        NATIVE_PASSPORT_PHASE1A_LABEL,
        "NATIVE_PASSPORT_PHASE1A_FEATURE_ISOLATION_NATIVE_MODULE_GATES"
    );

    assert_eq!(
        posture.enabled_surfaces,
        &[
            NativePassportSurface::FeaturePosture,
            NativePassportSurface::Phase0Contracts
        ]
    );
    assert_eq!(posture.enabled_surfaces, PHASE1A_ENABLED_SURFACES);
}

#[cfg(feature = "native-passport")]
#[test]
fn native_passport_feature_gate_does_not_add_runtime_authority() {
    let posture = native_passport_feature_posture();

    assert!(!posture.runtime_authority_changed);
    assert!(!posture.native_secret_implementation_added);
    assert!(!posture.routes_added);
    assert!(!posture.signing_or_verification_runtime_added);
    assert!(!posture.vault_runtime_added);
    assert!(!posture.capability_issuance_added);
}

#[cfg(feature = "native-passport")]
#[test]
fn native_passport_feature_gate_keeps_dangerous_behaviors_forbidden() {
    let posture = native_passport_feature_posture();

    for required in [
        "mnemonic generation",
        "root key derivation runtime",
        "device key generation runtime",
        "root signature generation",
        "device signature generation",
        "challenge route implementation",
        "request verification route",
        "capability issuance",
        "vault encryption",
        "vault decryption",
        "PIN capture",
        "platform keystore access",
        "wallet mutation",
        "ledger mutation",
        "new Passport crate",
    ] {
        assert!(
            posture.forbidden_behaviors.contains(&required),
            "Phase 1A must keep {required} forbidden"
        );
        assert!(
            PHASE1A_FORBIDDEN_BEHAVIORS.contains(&required),
            "forbidden behavior constant must include {required}"
        );
    }
}
