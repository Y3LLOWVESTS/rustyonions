//! RO:WHAT — Locks the canonical Native Passport V1 device-class wire vocabulary.
//! RO:WHY — DeviceAuthorizationV1 must use the active seven-class policy model rather than older contract-only desktop/mobile read-only placeholders.
//! RO:INTERACTS — ron-proto DeviceClassV1, future ron-auth DeviceAuthorizationV1 transcript verification, and ron-policy class-to-scope policy.
//! RO:INVARIANTS — exactly seven canonical V1 classes; snake_case wire names; retired desktop_read_only/mobile_read_only names do not deserialize.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — class is a policy input only; this DTO test grants no capability or signing authority.
//! RO:TEST — cargo test -p ron-proto --test physical_m1_device_class_v1_canonical.

use ron_proto::DeviceClassV1;

#[test]
fn physical_m1_device_class_v1_matches_active_closed_vocabulary() {
    let expected = [
        (DeviceClassV1::RootAdminDesktop, "root_admin_desktop"),
        (DeviceClassV1::RootAdminMobile, "root_admin_mobile"),
        (DeviceClassV1::PersonalDesktop, "personal_desktop"),
        (DeviceClassV1::PersonalMobile, "personal_mobile"),
        (DeviceClassV1::TvReadOnly, "tv_read_only"),
        (DeviceClassV1::RecoveryOnly, "recovery_only"),
        (DeviceClassV1::TestHarness, "test_harness"),
    ];

    assert_eq!(DeviceClassV1::ALL.len(), expected.len(),);

    for (actual_class, (expected_class, expected_wire)) in
        DeviceClassV1::ALL.into_iter().zip(expected)
    {
        assert_eq!(actual_class, expected_class);
        assert_eq!(actual_class.as_str(), expected_wire);

        let encoded =
            serde_json::to_string(&actual_class).expect("serialize canonical DeviceClassV1");

        assert_eq!(encoded, format!("\"{expected_wire}\""),);

        let decoded: DeviceClassV1 =
            serde_json::from_str(&encoded).expect("deserialize canonical DeviceClassV1");

        assert_eq!(decoded, actual_class);
    }
}

#[test]
fn physical_m1_retired_generic_read_only_class_names_fail_closed() {
    for retired in ["\"desktop_read_only\"", "\"mobile_read_only\""] {
        assert!(
            serde_json::from_str::<DeviceClassV1>(retired,).is_err(),
            "retired generic class must not become canonical network authority: {retired}",
        );
    }
}
