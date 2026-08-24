//! RO:WHAT — Admits one root-signed Native Passport DeviceAuthorizationV1 into the canonical durable server registry.
//! RO:WHY — CN-4 must turn the already-root-authorized physical device into durable svc-passport truth before device possession or capability issuance may occur.
//! RO:INTERACTS — ron-auth strict DeviceAuthorizationV1 verification, ron-proto canonical authorization DTOs, server_registry_store, and NativePassportServerRuntimeMountConfigV1.
//! RO:INVARIANTS — registered Passport root must already exist; network/environment/root epoch/signature/current time must verify; exact authorization is idempotent; conflicts and revoked records never silently reactivate; publication uses the existing immutable-generation CAS store.
//! RO:METRICS — none yet; mounted HTTP/service orchestration owns future bounded admission metrics.
//! RO:CONFIG — trusted network/environment and registry path come only from service-owned Native Passport runtime configuration.
//! RO:SECURITY — accepts public root-signed authorization evidence only; no root/device private key, PIN, RecoveryRoot, KMS signing, device-possession proof, capability, username, wallet, or ledger authority.
//! RO:TEST — focused unit tests below plus crabnode_cn4_device_authorize_route.rs.

#![forbid(unsafe_code)]

use std::path::Path;

use ron_auth::native_passport::{
    verify_device_authorization_v1_strict, DeviceAuthorizationVerificationContextV1,
};
use ron_proto::{DeviceAuthorizationV1, NativePassportContextLabelV1};

use super::{
    server_registry_store::{
        LoadedNativePassportServerRegistryV1, NativePassportServerDeviceRecordV1,
        NativePassportServerDeviceStatusV1, NativePassportServerRegistrySnapshotStore,
        NativePassportServerRegistryStoreError,
    },
    server_runtime_mount::NativePassportServerRuntimeMountConfigV1,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativePassportServerDeviceRegistrationDispositionV1 {
    Registered,
    AlreadyRegistered,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativePassportServerDeviceRegistrationOutcomeV1 {
    pub(crate) disposition: NativePassportServerDeviceRegistrationDispositionV1,

    pub(crate) durable_generation: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativePassportServerDeviceRegistrationError {
    TrustedContextInvalid,
    UnknownPassport,
    InvalidRegistrationTime,
    AuthorizationRejected,
    DeviceRegistrationConflict,
    GenerationOverflow,
    StorageUnavailable(&'static str),
    StorageCorrupt(&'static str),
}

pub(crate) struct NativePassportServerDeviceRegistrationRuntime {
    store: NativePassportServerRegistrySnapshotStore,
    loaded: LoadedNativePassportServerRegistryV1,
    expected_network_id: NativePassportContextLabelV1,
    expected_environment: NativePassportContextLabelV1,
}

impl NativePassportServerDeviceRegistrationRuntime {
    pub(crate) fn open(
        root: &Path,
        expected_network_id: NativePassportContextLabelV1,
        expected_environment: NativePassportContextLabelV1,
    ) -> Result<Self, NativePassportServerDeviceRegistrationError> {
        let (store, loaded) = NativePassportServerRegistrySnapshotStore::open(
            root,
            expected_network_id.clone(),
            expected_environment.clone(),
        )
        .map_err(map_store_error)?;

        Ok(Self {
            store,
            loaded,
            expected_network_id,
            expected_environment,
        })
    }

    pub(crate) fn register_authorization(
        &mut self,
        authorization: DeviceAuthorizationV1,
        registered_at_ms: u64,
    ) -> Result<
        NativePassportServerDeviceRegistrationOutcomeV1,
        NativePassportServerDeviceRegistrationError,
    > {
        if registered_at_ms == 0 {
            return Err(NativePassportServerDeviceRegistrationError::InvalidRegistrationTime);
        }

        let root = self
            .loaded
            .snapshot
            .passports
            .iter()
            .find(|record| record.passport_id == authorization.passport_id)
            .cloned()
            .ok_or(NativePassportServerDeviceRegistrationError::UnknownPassport)?;

        if registered_at_ms < root.registered_at_ms || registered_at_ms < authorization.issued_at_ms
        {
            return Err(NativePassportServerDeviceRegistrationError::InvalidRegistrationTime);
        }

        verify_device_authorization_v1_strict(
            &authorization,
            DeviceAuthorizationVerificationContextV1 {
                trusted_passport_id: &root.passport_id,
                trusted_root_public_key: &root.root_public_key,
                trusted_root_key_epoch: root.root_key_epoch,
                expected_network_id: &self.expected_network_id,
                expected_environment: &self.expected_environment,
                now_ms: registered_at_ms,
                max_clock_skew_ms: 0,
            },
        )
        .map_err(|_| NativePassportServerDeviceRegistrationError::AuthorizationRejected)?;

        if let Some(existing) = self
            .loaded
            .snapshot
            .devices
            .iter()
            .find(|record| record.device_id == authorization.device_id)
        {
            if existing.passport_id == authorization.passport_id
                && existing.authorization == authorization
                && existing.status == NativePassportServerDeviceStatusV1::Authorized
                && existing.revoked_at_ms.is_none()
            {
                return Ok(NativePassportServerDeviceRegistrationOutcomeV1 {
                    disposition:
                        NativePassportServerDeviceRegistrationDispositionV1::AlreadyRegistered,

                    durable_generation: self.loaded.generation,
                });
            }

            return Err(NativePassportServerDeviceRegistrationError::DeviceRegistrationConflict);
        }

        let next_generation = self
            .loaded
            .generation
            .checked_add(1)
            .ok_or(NativePassportServerDeviceRegistrationError::GenerationOverflow)?;

        let mut next_snapshot = self.loaded.snapshot.clone();

        next_snapshot
            .devices
            .push(NativePassportServerDeviceRecordV1 {
                passport_id: authorization.passport_id.clone(),

                device_id: authorization.device_id.clone(),

                authorization,

                status: NativePassportServerDeviceStatusV1::Authorized,

                registered_at_ms,

                revoked_at_ms: None,
            });

        next_snapshot.devices.sort_by(|left, right| {
            left.passport_id
                .as_str()
                .cmp(right.passport_id.as_str())
                .then_with(|| left.device_id.as_str().cmp(right.device_id.as_str()))
        });

        self.store
            .persist(
                self.loaded.generation,
                &self.loaded.snapshot,
                next_generation,
                &next_snapshot,
            )
            .map_err(map_store_error)?;

        self.loaded = LoadedNativePassportServerRegistryV1 {
            generation: next_generation,
            snapshot: next_snapshot,
        };

        Ok(NativePassportServerDeviceRegistrationOutcomeV1 {
            disposition: NativePassportServerDeviceRegistrationDispositionV1::Registered,

            durable_generation: next_generation,
        })
    }
}

pub(crate) fn register_device_authorization_durable(
    config: &NativePassportServerRuntimeMountConfigV1,
    authorization: DeviceAuthorizationV1,
    registered_at_ms: u64,
) -> Result<
    NativePassportServerDeviceRegistrationOutcomeV1,
    NativePassportServerDeviceRegistrationError,
> {
    let network_id = NativePassportContextLabelV1::parse(config.network_id.clone())
        .map_err(|_| NativePassportServerDeviceRegistrationError::TrustedContextInvalid)?;

    let environment = NativePassportContextLabelV1::parse(config.environment.clone())
        .map_err(|_| NativePassportServerDeviceRegistrationError::TrustedContextInvalid)?;

    let mut runtime = NativePassportServerDeviceRegistrationRuntime::open(
        &config.registry_root,
        network_id,
        environment,
    )?;

    runtime.register_authorization(authorization, registered_at_ms)
}

fn map_store_error(
    error: NativePassportServerRegistryStoreError,
) -> NativePassportServerDeviceRegistrationError {
    match error {
        NativePassportServerRegistryStoreError::Unavailable(reason) => {
            NativePassportServerDeviceRegistrationError::StorageUnavailable(reason)
        }

        NativePassportServerRegistryStoreError::Corrupt(reason) => {
            NativePassportServerDeviceRegistrationError::StorageCorrupt(reason)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use ron_proto::{
        DeviceAuthorizationNonceV1, DeviceAuthorizationScopeCeilingV1,
        DeviceAuthorizationSigningPayloadV1, DeviceAuthorizationV1, DeviceClassV1,
        DeviceIdV1 as ProtoDeviceIdV1, Ed25519PublicKeyHex as ProtoEd25519PublicKeyHex,
        NativePassportContextLabelV1, NativePassportScopeV1, PassportIdV1 as ProtoPassportIdV1,
        DEVICE_AUTHORIZATION_V1_VERSION,
    };

    use crate::native::{
        derive_native_device_public_identity_v1, derive_native_recovery_public_identity_v1,
        sign_native_recovery_device_authorization_v1, NativeSecretBytes,
    };

    use super::{
        NativePassportServerDeviceRegistrationDispositionV1,
        NativePassportServerDeviceRegistrationError, NativePassportServerDeviceRegistrationRuntime,
    };

    use crate::native::server_registry_store::{
        NativePassportServerRegistrySnapshotStore, NativePassportServerRegistrySnapshotV1,
        NativePassportServerRootRecordV1,
    };

    const ROOT_KEY_EPOCH: u64 = 0;
    const ROOT_REGISTERED_AT_MS: u64 = 1_000_000;
    const AUTH_ISSUED_AT_MS: u64 = 1_100_000;
    const DEVICE_REGISTERED_AT_MS: u64 = 1_200_000;

    struct TestDirectory {
        path: PathBuf,
    }

    impl TestDirectory {
        fn new(label: &str) -> Self {
            let stamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos();

            Self {
                path: std::env::temp_dir().join(format!(
                    "svc-passport-cn4-device-registration-{label}-{}-{stamp}",
                    std::process::id(),
                )),
            }
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn network() -> NativePassportContextLabelV1 {
        NativePassportContextLabelV1::parse("rustyonions-devnet").expect("network")
    }

    fn environment() -> NativePassportContextLabelV1 {
        NativePassportContextLabelV1::parse("private-beta").expect("environment")
    }

    fn recovery_factor(byte: u8) -> NativeSecretBytes {
        NativeSecretBytes::new(vec![byte; 32]).expect("nonphysical recovery factor")
    }

    fn device_seed(byte: u8) -> NativeSecretBytes {
        NativeSecretBytes::new(vec![byte; 32]).expect("nonphysical device seed")
    }

    fn scope(value: &str) -> NativePassportScopeV1 {
        NativePassportScopeV1::parse(value).expect("scope")
    }

    fn scope_ceiling() -> DeviceAuthorizationScopeCeilingV1 {
        DeviceAuthorizationScopeCeilingV1::new(vec![
            scope("capability.revoke_self"),
            scope("catalog.read"),
            scope("content.read"),
            scope("identity.read"),
        ])
        .expect("canonical scope ceiling")
    }

    fn seed_root(root: &Path, recovery_factor: &NativeSecretBytes) {
        let identity = derive_native_recovery_public_identity_v1(recovery_factor)
            .expect("root public identity");

        let passport_id =
            ProtoPassportIdV1::parse(identity.passport_id.as_str()).expect("Passport ID");

        let root_public_key = ProtoEd25519PublicKeyHex::parse(identity.root_public_key.as_str())
            .expect("root public key");

        let (store, loaded) =
            NativePassportServerRegistrySnapshotStore::open(root, network(), environment())
                .expect("open empty registry");

        let snapshot = NativePassportServerRegistrySnapshotV1 {
            passports: vec![NativePassportServerRootRecordV1 {
                passport_id,
                root_public_key,
                root_key_epoch: ROOT_KEY_EPOCH,
                registered_at_ms: ROOT_REGISTERED_AT_MS,
            }],

            devices: Vec::new(),
        };

        store
            .persist(loaded.generation, &loaded.snapshot, 1, &snapshot)
            .expect("seed generation-one root");
    }

    fn signed_authorization(
        recovery_factor: &NativeSecretBytes,
        device_seed_byte: u8,
    ) -> DeviceAuthorizationV1 {
        let root =
            derive_native_recovery_public_identity_v1(recovery_factor).expect("root identity");

        let device = derive_native_device_public_identity_v1(&device_seed(device_seed_byte))
            .expect("device identity");

        let payload = DeviceAuthorizationSigningPayloadV1 {
            version: DEVICE_AUTHORIZATION_V1_VERSION,

            network_id: network(),

            environment: environment(),

            passport_id: ProtoPassportIdV1::parse(root.passport_id.as_str()).expect("Passport ID"),

            root_key_epoch: ROOT_KEY_EPOCH,

            device_id: ProtoDeviceIdV1::parse(device.device_id.as_str()).expect("device ID"),

            device_public_key: ProtoEd25519PublicKeyHex::parse(device.device_public_key.as_str())
                .expect("device public key"),

            device_class: DeviceClassV1::RootAdminDesktop,

            authorized_scope_ceiling: scope_ceiling(),

            authorization_nonce: DeviceAuthorizationNonceV1::from_bytes([
                0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
                0x0e, 0x0f,
            ]),

            issued_at_ms: AUTH_ISSUED_AT_MS,

            expires_at_ms: None,
        };

        sign_native_recovery_device_authorization_v1(recovery_factor, payload)
            .expect("root-signed device authorization")
    }

    fn open_runtime(root: &Path) -> NativePassportServerDeviceRegistrationRuntime {
        NativePassportServerDeviceRegistrationRuntime::open(root, network(), environment())
            .expect("open device registration runtime")
    }

    #[test]
    fn signed_device_registers_once_and_survives_restart() {
        let directory = TestDirectory::new("register");

        let factor = recovery_factor(0x22);

        seed_root(&directory.path, &factor);

        let authorization = signed_authorization(&factor, 0x42);

        let mut runtime = open_runtime(&directory.path);

        let first = runtime
            .register_authorization(authorization.clone(), DEVICE_REGISTERED_AT_MS)
            .expect("first registration");

        assert_eq!(
            first.disposition,
            NativePassportServerDeviceRegistrationDispositionV1::Registered,
        );

        assert_eq!(first.durable_generation, 2,);

        let repeated = runtime
            .register_authorization(authorization.clone(), DEVICE_REGISTERED_AT_MS + 1)
            .expect("idempotent repeat");

        assert_eq!(
            repeated.disposition,
            NativePassportServerDeviceRegistrationDispositionV1::AlreadyRegistered,
        );

        assert_eq!(repeated.durable_generation, 2,);

        let reopened = open_runtime(&directory.path);

        assert_eq!(reopened.loaded.generation, 2,);

        assert_eq!(reopened.loaded.snapshot.passports.len(), 1,);

        assert_eq!(reopened.loaded.snapshot.devices.len(), 1,);

        assert_eq!(
            reopened.loaded.snapshot.devices[0].authorization,
            authorization,
        );
    }

    #[test]
    fn unknown_root_and_tampered_authorization_fail_without_generation_advance() {
        let directory = TestDirectory::new("reject");

        let trusted_factor = recovery_factor(0x31);

        seed_root(&directory.path, &trusted_factor);

        let unregistered_factor = recovery_factor(0x32);

        let unknown = signed_authorization(&unregistered_factor, 0x51);

        let mut runtime = open_runtime(&directory.path);

        assert_eq!(
            runtime.register_authorization(unknown, DEVICE_REGISTERED_AT_MS,),
            Err(NativePassportServerDeviceRegistrationError::UnknownPassport,),
        );

        let mut tampered = signed_authorization(&trusted_factor, 0x52);

        tampered.authorization_nonce = DeviceAuthorizationNonceV1::from_bytes([
            0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d,
            0x1e, 0x1f,
        ]);

        assert_eq!(
            runtime.register_authorization(tampered, DEVICE_REGISTERED_AT_MS,),
            Err(NativePassportServerDeviceRegistrationError::AuthorizationRejected,),
        );

        let reopened = open_runtime(&directory.path);

        assert_eq!(reopened.loaded.generation, 1,);

        assert!(reopened.loaded.snapshot.devices.is_empty());
    }

    #[test]
    fn device_registration_runtime_has_no_possession_capability_profile_or_value_authority() {
        let implementation = include_str!("server_device_registration_runtime.rs")
            .split_once("\n#[cfg(test)]")
            .expect("implementation precedes tests")
            .0;

        for forbidden in [
            "Router::new",
            ".route(",
            "device_private_key",
            "NativeSecureCompartment::RecoveryRoot",
            "unseal_native_secret(",
            "verify_native_recovery_root_pin(",
            "sign_native_recovery_device_authorization_v1(",
            "raw_pin",
            "issue_capability(",
            "claim_username(",
            "wallet.spend(",
            "ledger.write(",
            "tauri::",
        ] {
            assert!(
                !implementation.contains(forbidden),
                "device registration runtime gained forbidden authority pattern {forbidden}",
            );
        }
    }
}
