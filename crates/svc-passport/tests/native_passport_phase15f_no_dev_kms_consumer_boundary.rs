#![cfg(all(feature = "native-passport", not(feature = "dev-kms")))]

use std::{fs, path::PathBuf};

use svc_passport::kms::{service_kms_injection_posture, ServiceKmsConstructionMode};

fn repo_file(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

#[test]
fn native_passport_consumer_compiles_without_dev_kms_authority() {
    assert!(cfg!(feature = "native-passport"));
    assert!(!cfg!(feature = "dev-kms"));

    let posture = service_kms_injection_posture();

    assert_eq!(
        posture.default_mode,
        ServiceKmsConstructionMode::ExternalInjected
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
fn kms_trait_remains_available_without_compiling_dev_kms() {
    let source = fs::read_to_string(repo_file("src/kms/client.rs")).expect("KMS client source");

    assert!(source.contains("pub trait KmsClient"));
    assert!(source.contains("#[cfg(feature = \"dev-kms\")]"));
    assert!(source.contains("pub use dev::DevKms;"));

    assert!(
        !source.contains("compile_error!"),
        "native-only consumers must not be blocked by a server DevKms compile error"
    );
}
