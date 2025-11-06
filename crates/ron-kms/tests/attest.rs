use ron_kms::ops::attest::attest;
use ron_kms::{memory_keystore, Keystore}; // <-- import the function, not the module

#[test]
fn attest_reports_versions_and_current() {
    let kms = memory_keystore();
    let kid_v1 = kms.create_ed25519("auth", "signing").expect("create");

    let meta1 = attest(&kms, &kid_v1).expect("attest v1");
    assert_eq!(meta1.current_version, 1);
    assert_eq!(meta1.versions, vec![1]);
    assert!(meta1.created_ms > 0);

    let kid_v2 = kms.rotate(&kid_v1).expect("rotate");
    let meta2 = attest(&kms, &kid_v2).expect("attest v2");
    assert_eq!(meta2.current_version, 2);
    assert_eq!(meta2.versions, vec![1, 2]);
    assert_eq!(meta2.created_ms, meta1.created_ms);
}
