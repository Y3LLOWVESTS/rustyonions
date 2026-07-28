//! RO:WHAT — Platform-neutral Native Passport sealer and encrypted VaultStore trait foundation.
//! RO:WHY — Phase 15I provides the bounded interfaces that desktop OS adapters and atomic app-data persistence will implement.
//! RO:INTERACTS — Phase 5A PlatformSealer contracts, Phase 6A vault headers, Phase 15H ownership inspection, and future CrabLink Tauri platform adapters.
//! RO:INVARIANTS — secret buffers are bounded and zeroized on drop; vault stores accept encrypted vault envelopes only; atomic write and interrupted-write recovery are explicit trait obligations.
//! RO:SECURITY — no OS keychain, DPAPI, Secret Service, filesystem, encryption algorithm, PIN handling, vault unlock, capability issuance, wallet, or ledger implementation is added here.
//! RO:TEST — tests/native_passport_phase15i_platform_sealer_vault_store_trait_foundation.rs.

use std::fmt;

use super::{NativePlatformFamily, NativeSecureCompartment};

pub const NATIVE_PASSPORT_PHASE15I_LABEL: &str =
    "NATIVE_PASSPORT_PHASE15I_PLATFORM_SEALER_AND_VAULT_STORE_TRAIT_FOUNDATION";

pub const PHASE15I_PLATFORM_STORAGE_DOMAIN: &str = "native-passport/platform-storage/v1";

pub const PHASE15I_PLATFORM_STORAGE_VERSION: u16 = 1;

pub const PHASE15I_MAX_SECRET_MATERIAL_BYTES: usize = 65_536;

pub const PHASE15I_MAX_SEALED_MATERIAL_BYTES: usize = 131_072;

pub const PHASE15I_MAX_ENCRYPTED_VAULT_BYTES: usize = 4_194_304;

pub const PHASE15I_ATOMIC_VAULT_WRITE_STEPS: &[&str] = &[
    "write_encrypted_temporary_file",
    "sync_temporary_file",
    "sync_parent_directory_where_supported",
    "atomic_rename",
    "sync_parent_directory_after_rename_where_supported",
    "remove_stale_temporary_file",
];

pub const PHASE15I_FORBIDDEN_PLATFORM_STORAGE_AUTHORITY_FLAGS: &[&str] = &[
    "os_platform_adapter",
    "filesystem_adapter",
    "plaintext_temporary_file",
    "frontend_secret_custody",
    "raw_secret_export",
    "pin_unlock",
    "vault_decryption",
    "vault_encryption",
    "capability_issuance",
    "wallet_spend",
    "ledger_mutation",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativePlatformStorageOperation {
    Seal,
    Unseal,
    LoadEncryptedVault,
    WriteEncryptedVaultAtomic,
    RecoverInterruptedWrite,
    RemoveEncryptedVault,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativePlatformStorageError {
    EmptySecretMaterial,
    SecretMaterialTooLarge {
        actual: usize,
        maximum: usize,
    },
    EmptySealedMaterial,
    SealedMaterialTooLarge {
        actual: usize,
        maximum: usize,
    },
    EmptyEncryptedVault,
    EncryptedVaultTooLarge {
        actual: usize,
        maximum: usize,
    },
    PlatformFamilyMismatch {
        expected: NativePlatformFamily,
        actual: NativePlatformFamily,
    },
    CompartmentMismatch {
        expected: NativeSecureCompartment,
        actual: NativeSecureCompartment,
    },
    BackendUnavailable {
        operation: NativePlatformStorageOperation,
    },
    BackendFailure {
        operation: NativePlatformStorageOperation,
    },
}

#[derive(PartialEq, Eq)]
pub struct NativeSecretBytes {
    bytes: Vec<u8>,
}

impl NativeSecretBytes {
    pub fn new(mut bytes: Vec<u8>) -> Result<Self, NativePlatformStorageError> {
        if bytes.is_empty() {
            bytes.fill(0);
            return Err(NativePlatformStorageError::EmptySecretMaterial);
        }

        if bytes.len() > PHASE15I_MAX_SECRET_MATERIAL_BYTES {
            let actual = bytes.len();
            bytes.fill(0);

            return Err(NativePlatformStorageError::SecretMaterialTooLarge {
                actual,
                maximum: PHASE15I_MAX_SECRET_MATERIAL_BYTES,
            });
        }

        Ok(Self { bytes })
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.bytes
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
}

impl fmt::Debug for NativeSecretBytes {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NativeSecretBytes")
            .field("length", &self.bytes.len())
            .field("material", &"REDACTED")
            .finish()
    }
}

impl Drop for NativeSecretBytes {
    fn drop(&mut self) {
        self.bytes.fill(0);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeSealedMaterialV1 {
    pub contract_domain: &'static str,
    pub contract_version: u16,
    pub platform_family: NativePlatformFamily,
    pub compartment: NativeSecureCompartment,
    sealed_bytes: Vec<u8>,
}

impl NativeSealedMaterialV1 {
    pub fn new(
        platform_family: NativePlatformFamily,
        compartment: NativeSecureCompartment,
        sealed_bytes: Vec<u8>,
    ) -> Result<Self, NativePlatformStorageError> {
        validate_sealed_material(&sealed_bytes)?;

        Ok(Self {
            contract_domain: PHASE15I_PLATFORM_STORAGE_DOMAIN,
            contract_version: PHASE15I_PLATFORM_STORAGE_VERSION,
            platform_family,
            compartment,
            sealed_bytes,
        })
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.sealed_bytes
    }

    pub fn len(&self) -> usize {
        self.sealed_bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sealed_bytes.is_empty()
    }

    pub fn validate(&self) -> Result<(), NativePlatformStorageError> {
        validate_sealed_material(&self.sealed_bytes)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeEncryptedVaultV1 {
    pub contract_domain: &'static str,
    pub contract_version: u16,
    encrypted_bytes: Vec<u8>,
}

impl NativeEncryptedVaultV1 {
    pub fn new(encrypted_bytes: Vec<u8>) -> Result<Self, NativePlatformStorageError> {
        validate_encrypted_vault(&encrypted_bytes)?;

        Ok(Self {
            contract_domain: PHASE15I_PLATFORM_STORAGE_DOMAIN,
            contract_version: PHASE15I_PLATFORM_STORAGE_VERSION,
            encrypted_bytes,
        })
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.encrypted_bytes
    }

    pub fn len(&self) -> usize {
        self.encrypted_bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.encrypted_bytes.is_empty()
    }

    pub fn validate(&self) -> Result<(), NativePlatformStorageError> {
        validate_encrypted_vault(&self.encrypted_bytes)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeVaultRecoveryOutcome {
    NoRecoveryNeeded,
    StaleTemporaryFileRemoved,
    ValidTemporaryFilePromoted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeVaultRemovalOutcome {
    NotFound,
    Removed,
}

pub trait NativePlatformSealer: Send + Sync {
    fn platform_family(&self) -> NativePlatformFamily;

    fn seal(
        &self,
        compartment: NativeSecureCompartment,
        secret: &NativeSecretBytes,
    ) -> Result<NativeSealedMaterialV1, NativePlatformStorageError>;

    fn unseal(
        &self,
        sealed: &NativeSealedMaterialV1,
    ) -> Result<NativeSecretBytes, NativePlatformStorageError>;
}

pub trait NativeVaultStore: Send + Sync {
    fn load_encrypted_vault(
        &self,
    ) -> Result<Option<NativeEncryptedVaultV1>, NativePlatformStorageError>;

    fn write_encrypted_vault_atomic(
        &self,
        vault: &NativeEncryptedVaultV1,
    ) -> Result<(), NativePlatformStorageError>;

    fn recover_interrupted_write(
        &self,
    ) -> Result<NativeVaultRecoveryOutcome, NativePlatformStorageError>;

    fn remove_encrypted_vault(
        &self,
    ) -> Result<NativeVaultRemovalOutcome, NativePlatformStorageError>;
}

pub fn seal_native_secret<S>(
    sealer: &S,
    expected_platform: NativePlatformFamily,
    compartment: NativeSecureCompartment,
    secret: &NativeSecretBytes,
) -> Result<NativeSealedMaterialV1, NativePlatformStorageError>
where
    S: NativePlatformSealer + ?Sized,
{
    validate_platform_family(expected_platform, sealer.platform_family())?;

    let sealed = sealer.seal(compartment, secret)?;

    validate_platform_family(expected_platform, sealed.platform_family)?;

    if sealed.compartment != compartment {
        return Err(NativePlatformStorageError::CompartmentMismatch {
            expected: compartment,
            actual: sealed.compartment,
        });
    }

    sealed.validate()?;

    Ok(sealed)
}

pub fn unseal_native_secret<S>(
    sealer: &S,
    expected_platform: NativePlatformFamily,
    expected_compartment: NativeSecureCompartment,
    sealed: &NativeSealedMaterialV1,
) -> Result<NativeSecretBytes, NativePlatformStorageError>
where
    S: NativePlatformSealer + ?Sized,
{
    validate_platform_family(expected_platform, sealer.platform_family())?;

    validate_platform_family(expected_platform, sealed.platform_family)?;

    if sealed.compartment != expected_compartment {
        return Err(NativePlatformStorageError::CompartmentMismatch {
            expected: expected_compartment,
            actual: sealed.compartment,
        });
    }

    sealed.validate()?;

    let secret = sealer.unseal(sealed)?;

    if secret.is_empty() {
        return Err(NativePlatformStorageError::EmptySecretMaterial);
    }

    Ok(secret)
}

pub fn load_native_encrypted_vault<S>(
    store: &S,
) -> Result<Option<NativeEncryptedVaultV1>, NativePlatformStorageError>
where
    S: NativeVaultStore + ?Sized,
{
    let vault = store.load_encrypted_vault()?;

    if let Some(ref loaded) = vault {
        loaded.validate()?;
    }

    Ok(vault)
}

pub fn write_native_encrypted_vault_atomic<S>(
    store: &S,
    vault: &NativeEncryptedVaultV1,
) -> Result<(), NativePlatformStorageError>
where
    S: NativeVaultStore + ?Sized,
{
    vault.validate()?;
    store.write_encrypted_vault_atomic(vault)
}

pub fn recover_native_interrupted_vault_write<S>(
    store: &S,
) -> Result<NativeVaultRecoveryOutcome, NativePlatformStorageError>
where
    S: NativeVaultStore + ?Sized,
{
    store.recover_interrupted_write()
}

pub fn remove_native_encrypted_vault<S>(
    store: &S,
) -> Result<NativeVaultRemovalOutcome, NativePlatformStorageError>
where
    S: NativeVaultStore + ?Sized,
{
    store.remove_encrypted_vault()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativePlatformStorageTraitPosture {
    pub phase_label: &'static str,
    pub secret_buffer_added: bool,
    pub secret_buffer_zeroizes_on_drop: bool,
    pub sealed_material_envelope_added: bool,
    pub encrypted_vault_envelope_added: bool,
    pub platform_sealer_trait_added: bool,
    pub vault_store_trait_added: bool,
    pub atomic_write_contract_added: bool,
    pub interrupted_write_recovery_contract_added: bool,
    pub bounded_validation_helpers_added: bool,
    pub os_platform_adapter_added: bool,
    pub filesystem_adapter_added: bool,
    pub encryption_runtime_added: bool,
    pub decryption_runtime_added: bool,
    pub vault_unlock_added: bool,
    pub runtime_io_added: bool,
    pub frontend_secret_custody_added: bool,
    pub capability_issuance_added: bool,
    pub wallet_or_ledger_mutation_added: bool,
    pub forbidden_authority_flags: &'static [&'static str],
}

pub fn native_platform_storage_trait_posture() -> NativePlatformStorageTraitPosture {
    NativePlatformStorageTraitPosture {
        phase_label: NATIVE_PASSPORT_PHASE15I_LABEL,
        secret_buffer_added: true,
        secret_buffer_zeroizes_on_drop: true,
        sealed_material_envelope_added: true,
        encrypted_vault_envelope_added: true,
        platform_sealer_trait_added: true,
        vault_store_trait_added: true,
        atomic_write_contract_added: true,
        interrupted_write_recovery_contract_added: true,
        bounded_validation_helpers_added: true,
        os_platform_adapter_added: false,
        filesystem_adapter_added: false,
        encryption_runtime_added: false,
        decryption_runtime_added: false,
        vault_unlock_added: false,
        runtime_io_added: false,
        frontend_secret_custody_added: false,
        capability_issuance_added: false,
        wallet_or_ledger_mutation_added: false,
        forbidden_authority_flags: PHASE15I_FORBIDDEN_PLATFORM_STORAGE_AUTHORITY_FLAGS,
    }
}

fn validate_platform_family(
    expected: NativePlatformFamily,
    actual: NativePlatformFamily,
) -> Result<(), NativePlatformStorageError> {
    if expected != actual {
        return Err(NativePlatformStorageError::PlatformFamilyMismatch { expected, actual });
    }

    Ok(())
}

fn validate_sealed_material(bytes: &[u8]) -> Result<(), NativePlatformStorageError> {
    if bytes.is_empty() {
        return Err(NativePlatformStorageError::EmptySealedMaterial);
    }

    if bytes.len() > PHASE15I_MAX_SEALED_MATERIAL_BYTES {
        return Err(NativePlatformStorageError::SealedMaterialTooLarge {
            actual: bytes.len(),
            maximum: PHASE15I_MAX_SEALED_MATERIAL_BYTES,
        });
    }

    Ok(())
}

fn validate_encrypted_vault(bytes: &[u8]) -> Result<(), NativePlatformStorageError> {
    if bytes.is_empty() {
        return Err(NativePlatformStorageError::EmptyEncryptedVault);
    }

    if bytes.len() > PHASE15I_MAX_ENCRYPTED_VAULT_BYTES {
        return Err(NativePlatformStorageError::EncryptedVaultTooLarge {
            actual: bytes.len(),
            maximum: PHASE15I_MAX_ENCRYPTED_VAULT_BYTES,
        });
    }

    Ok(())
}
