//! RO:WHAT — Private-beta Native Passport device-class scope-ceiling policy.
//!
//! RO:WHY — Physical M1 needs policy-owned ceilings before a real root signs a device.
//!
//! RO:INTERACTS — `ron_proto::{DeviceClassV1, DeviceAuthorizationScopeCeilingV1, NativePassportScopeV1}` and `svc-passport`.
//!
//! RO:INVARIANTS — policy cannot widen from client input; recovery has no ordinary scopes; test harness is non-production; no economic/node authority.
//!
//! RO:CONFIG — V1 private-beta scope vocabulary only; network, environment, root epoch, time, nonce, and authorization expiry are external trusted context.
//!
//! RO:SECURITY — declarative public policy only; no key custody, signing, capability issuance, registry mutation, wallet, or ledger authority.
//!
//! RO:TEST — `tests/physical_m1_native_passport_device_policy.rs`.

#![forbid(unsafe_code)]
#![allow(clippy::module_name_repetitions)]

use ron_proto::{DeviceAuthorizationScopeCeilingV1, DeviceClassV1, NativePassportScopeV1};
use thiserror::Error;

/// Current private-beta Native Passport device scope-policy version.
pub const NATIVE_PASSPORT_PRIVATE_BETA_DEVICE_POLICY_VERSION: u16 = 1;

const TV_READ_ONLY_SCOPES: &[&str] = &[
    "capability.revoke_self",
    "catalog.read",
    "confirmed_roc.read",
    "content.read",
    "entitlement.read",
    "identity.read",
    "receipts.read",
];

const PERSONAL_IDENTITY_SCOPES: &[&str] = &[
    "capability.revoke_self",
    "catalog.read",
    "confirmed_roc.read",
    "content.read",
    "entitlement.read",
    "identity.device.list",
    "identity.profile.read",
    "identity.profile.update",
    "identity.read",
    "receipts.read",
];

const ROOT_ADMIN_IDENTITY_SCOPES: &[&str] = &[
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
];

/// Errors returned by private-beta Native Passport device policy.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum NativePassportDevicePolicyError {
    /// Test-harness devices are not admitted by production private-beta policy.
    #[error("device class is restricted to non-production test policy: {device_class}")]
    NonProductionDeviceClass {
        /// Canonical rejected device-class spelling.
        device_class: &'static str,
    },

    /// A future protocol device class is not understood by this policy version.
    #[error("unsupported Native Passport device class for private-beta policy")]
    UnsupportedDeviceClass,

    /// A policy-owned scope constant failed canonical protocol parsing.
    #[error("invalid Native Passport policy scope constant: {scope}")]
    InvalidScopeConstant {
        /// Invalid policy-owned scope.
        scope: &'static str,
    },

    /// The policy-owned scope collection violated canonical ceiling invariants.
    #[error("invalid canonical Native Passport device scope ceiling")]
    InvalidScopeCeiling,
}

/// Return the maximum ordinary capability scope ceiling permitted for a V1
/// private-beta Native Passport device class.
///
/// This is a ceiling only. Actual capability scopes must still be intersected
/// with requested scopes, Passport/device state, route policy, and server
/// configuration.
///
/// # Errors
///
/// Returns [`NativePassportDevicePolicyError`] for non-production test-harness
/// devices, unknown future protocol classes, or invalid policy constants.
pub fn private_beta_device_authorization_scope_ceiling_v1(
    device_class: DeviceClassV1,
) -> Result<DeviceAuthorizationScopeCeilingV1, NativePassportDevicePolicyError> {
    let scope_names = match device_class {
        DeviceClassV1::RootAdminDesktop | DeviceClassV1::RootAdminMobile => {
            ROOT_ADMIN_IDENTITY_SCOPES
        }
        DeviceClassV1::PersonalDesktop | DeviceClassV1::PersonalMobile => PERSONAL_IDENTITY_SCOPES,
        DeviceClassV1::TvReadOnly => TV_READ_ONLY_SCOPES,
        DeviceClassV1::RecoveryOnly => &[],
        DeviceClassV1::TestHarness => {
            return Err(NativePassportDevicePolicyError::NonProductionDeviceClass {
                device_class: device_class.as_str(),
            });
        }
        _ => return Err(NativePassportDevicePolicyError::UnsupportedDeviceClass),
    };

    canonical_scope_ceiling(scope_names)
}

fn canonical_scope_ceiling(
    scope_names: &'static [&'static str],
) -> Result<DeviceAuthorizationScopeCeilingV1, NativePassportDevicePolicyError> {
    let scopes = scope_names
        .iter()
        .map(|scope| {
            NativePassportScopeV1::parse(*scope)
                .map_err(|_| NativePassportDevicePolicyError::InvalidScopeConstant { scope })
        })
        .collect::<Result<Vec<_>, _>>()?;

    DeviceAuthorizationScopeCeilingV1::new(scopes)
        .map_err(|_| NativePassportDevicePolicyError::InvalidScopeCeiling)
}
