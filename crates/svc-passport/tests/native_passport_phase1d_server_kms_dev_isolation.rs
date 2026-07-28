use std::{fs, path::PathBuf, sync::Arc};

use svc_passport::{
    config::{Cache, Config, Limits, Passport, Security, Server, VerifyPolicy},
    health::Health,
    http::router::build_router_with_kms,
    kms::{
        client::{DevKms, KmsClient},
        service_kms_injection_posture, ServiceKmsConstructionMode, NATIVE_PASSPORT_PHASE1D_LABEL,
    },
};

fn repo_file(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn test_config() -> Config {
    Config {
        server: Server {
            bind: "127.0.0.1:0".to_owned(),
            admin_bind: "127.0.0.1:0".to_owned(),
        },
        passport: Passport {
            issuer: "svc-passport-test".to_owned(),
            default_ttl_s: 60,
            max_ttl_s: 3600,
            clock_skew_s: 5,
        },
        verify: VerifyPolicy {
            target_batch: 16,
            max_batch: 64,
            max_wait_us: 1000,
        },
        cache: Cache {
            vk_ttl_s: 60,
            jwks_ttl_s: 60,
        },
        limits: Limits {
            max_msg_bytes: 1024 * 1024,
            max_batch: 64,
        },
        security: Security { require_aud: false },
    }
}

#[test]
fn phase1d_label_and_kms_posture_are_locked() {
    let posture = service_kms_injection_posture();

    assert_eq!(
        NATIVE_PASSPORT_PHASE1D_LABEL,
        "NATIVE_PASSPORT_PHASE1D_SERVER_KMS_DEV_ISOLATION"
    );
    assert_eq!(posture.phase_label, NATIVE_PASSPORT_PHASE1D_LABEL);
    assert_eq!(
        posture.default_mode,
        ServiceKmsConstructionMode::DevelopmentInProcess
    );
    assert!(posture.explicit_injection_supported);
    assert!(posture.dev_default_constructor_isolated);
    assert!(!posture.runtime_authority_changed);
    assert!(!posture.native_secret_implementation_added);
    assert!(!posture.native_passport_signing_runtime_added);
    assert!(!posture.vault_runtime_added);
    assert!(!posture.capability_issuance_added);
}

#[test]
fn router_builds_with_explicitly_injected_kms_client() {
    let kms: Arc<dyn KmsClient> = Arc::new(DevKms::new());
    let _router = build_router_with_kms(test_config(), Health::default(), kms);
}

#[test]
fn router_default_dev_kms_constructor_is_isolated_from_injected_builder() {
    let router_source = fs::read_to_string(repo_file("src/http/router.rs"))
        .expect("router source should be readable");

    assert!(
        router_source.contains("pub fn build_router_with_kms"),
        "router must expose explicit KMS injection builder"
    );
    assert!(
        router_source.contains("fn default_dev_kms()"),
        "router must keep default dev KMS construction in a named helper"
    );
    assert!(
        router_source.contains("build_router_with_kms(cfg, health, kms)"),
        "default router constructor must delegate to injected builder"
    );

    let injected_start = router_source
        .find("pub fn build_router_with_kms")
        .expect("injected builder must exist");
    let injected_body = &router_source[injected_start..];

    assert!(
        !injected_body.contains("DevKms::new()"),
        "injected router builder must not construct DevKms internally"
    );
}

#[test]
fn phase1d_does_not_add_native_passport_secret_or_runtime_authority_terms() {
    let posture = service_kms_injection_posture();

    assert!(!posture.runtime_authority_changed);
    assert!(!posture.native_secret_implementation_added);
    assert!(!posture.native_passport_signing_runtime_added);
    assert!(!posture.vault_runtime_added);
    assert!(!posture.capability_issuance_added);

    let router_source = fs::read_to_string(repo_file("src/http/router.rs"))
        .expect("router source should be readable");

    let injected_start = router_source
        .find("pub fn build_router_with_kms")
        .expect("injected builder must exist");
    let injected_body = &router_source[injected_start..];

    for forbidden_runtime_token in [
        "DevKms::new()",
        "generate_mnemonic",
        "derive_root_key",
        "generate_device_key",
        "root_sign",
        "device_sign",
        "verify_challenge_proof",
        "verify_request_proof",
        "issue_capability",
        "encrypt_vault",
        "decrypt_vault",
        "wallet.spend",
        "ledger.write",
    ] {
        assert!(
            !injected_body.contains(forbidden_runtime_token),
            "injected router builder must not contain runtime token {forbidden_runtime_token}"
        );
    }
}
