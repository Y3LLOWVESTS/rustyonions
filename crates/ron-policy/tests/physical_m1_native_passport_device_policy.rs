//! RO:WHAT — Locks private-beta Native Passport device-class scope policy.
//! RO:WHY — Physical M1 must obtain its root-admin ceiling from policy instead of client/test fixtures.
//! RO:INTERACTS — `ron_policy::native_passport` and canonical `ron_proto::DeviceClassV1`.
//! RO:INVARIANTS — root admin gets identity administration; personal devices cannot administer; TV is read-only; recovery is empty; no economic/node authority.
//! RO:SECURITY — policy tests create no keys, signatures, capabilities, registry entries, username mutations, wallet mutations, or ledger mutations.
//! RO:TEST — focused test target of this file.

use ron_policy::{
    private_beta_device_authorization_scope_ceiling_v1, NativePassportDevicePolicyError,
    NATIVE_PASSPORT_PRIVATE_BETA_DEVICE_POLICY_VERSION,
};
use ron_proto::{DeviceAuthorizationScopeCeilingV1, DeviceClassV1};

fn scope_names(ceiling: &DeviceAuthorizationScopeCeilingV1) -> Vec<&str> {
    ceiling
        .as_slice()
        .iter()
        .map(ron_proto::NativePassportScopeV1::as_str)
        .collect()
}

#[test]
fn physical_m1_root_admin_desktop_receives_exact_identity_admin_ceiling() {
    assert_eq!(NATIVE_PASSPORT_PRIVATE_BETA_DEVICE_POLICY_VERSION, 1);

    let ceiling =
        private_beta_device_authorization_scope_ceiling_v1(DeviceClassV1::RootAdminDesktop)
            .unwrap();

    assert_eq!(
        scope_names(&ceiling),
        vec![
            "capability.revoke_self",
            "catalog.read",
            "confirmed_roc.read",
            "content.read",
            "entitlement.read",
            "identity.device.authorize",
            "identity.device.list",
            "identity.device.revoke",
            "identity.profile.read",
            "identity.profile.update",
            "identity.read",
            "identity.username.claim",
            "identity.username.release",
            "identity.username.transfer",
            "receipts.read",
        ],
    );
}

#[test]
fn physical_m1_personal_devices_cannot_receive_root_admin_identity_scopes() {
    for device_class in [
        DeviceClassV1::PersonalDesktop,
        DeviceClassV1::PersonalMobile,
    ] {
        let ceiling = private_beta_device_authorization_scope_ceiling_v1(device_class).unwrap();

        let names = scope_names(&ceiling);

        for forbidden in [
            "identity.device.authorize",
            "identity.device.revoke",
            "identity.username.claim",
            "identity.username.release",
            "identity.username.transfer",
        ] {
            assert_eq!(names.contains(&forbidden), false);
        }

        assert_eq!(names.contains(&"identity.profile.update"), true);
        assert_eq!(names.contains(&"identity.device.list"), true);
    }
}

#[test]
fn physical_m1_tv_policy_is_exact_read_only_scope_set() {
    let ceiling =
        private_beta_device_authorization_scope_ceiling_v1(DeviceClassV1::TvReadOnly).unwrap();

    assert_eq!(
        scope_names(&ceiling),
        vec![
            "capability.revoke_self",
            "catalog.read",
            "confirmed_roc.read",
            "content.read",
            "entitlement.read",
            "identity.read",
            "receipts.read",
        ],
    );
}

#[test]
fn physical_m1_recovery_only_has_no_ordinary_network_scope() {
    let ceiling =
        private_beta_device_authorization_scope_ceiling_v1(DeviceClassV1::RecoveryOnly).unwrap();

    assert_eq!(ceiling.as_slice().is_empty(), true);
}

#[test]
fn physical_m1_test_harness_is_rejected_by_private_beta_production_policy() {
    let error =
        private_beta_device_authorization_scope_ceiling_v1(DeviceClassV1::TestHarness).unwrap_err();

    assert!(matches!(
        error,
        NativePassportDevicePolicyError::NonProductionDeviceClass {
            device_class: "test_harness",
        }
    ));
}

#[test]
fn physical_m1_private_beta_device_policy_never_grants_forbidden_authority() {
    for device_class in [
        DeviceClassV1::RootAdminDesktop,
        DeviceClassV1::RootAdminMobile,
        DeviceClassV1::PersonalDesktop,
        DeviceClassV1::PersonalMobile,
        DeviceClassV1::TvReadOnly,
        DeviceClassV1::RecoveryOnly,
    ] {
        let ceiling = private_beta_device_authorization_scope_ceiling_v1(device_class).unwrap();

        let names = scope_names(&ceiling);

        for forbidden in [
            "content.publish",
            "wallet.spend",
            "wallet.transfer",
            "ledger.write",
            "reward.issue",
            "node.control",
            "operator.admin",
            "capability.delegate_unbounded",
            "bridge.settle",
            "staking.open",
        ] {
            assert_eq!(
                names.contains(&forbidden),
                false,
                "{device_class:?} unexpectedly received {forbidden}",
            );
        }
    }
}
