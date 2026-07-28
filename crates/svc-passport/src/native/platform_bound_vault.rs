//! RO:WHAT — Encodes the two platform-sealed compartment factors beside the PIN-wrapped Native Passport VMKs.
//! RO:WHY — Windows DPAPI returns sealed bytes that must survive restart, while Keychain and Secret Service return bounded references; both require one canonical persisted envelope.
//! RO:INTERACTS — Phase 15I sealed/encrypted material contracts, Phase 15Q wrapped VMKs, desktop PlatformSealer adapters, and the atomic VaultStore.
//! RO:INVARIANTS — exactly one recovery-root factor, one operational factor, one platform family, and one validated two-compartment wrapped-key payload.
//! RO:SECURITY — sealed material and authenticated ciphertext only; no PIN, VMK, platform-factor plaintext, signing key, capability, wallet, ledger, filesystem, or Tauri command.
//! RO:TEST — tests/native_passport_phase15r_platform_bound_vault_envelope.rs.

use std::fmt;

use super::{
    decode_native_pin_wrapped_vault_keys, encode_native_pin_wrapped_vault_keys,
    NativeEncryptedVaultV1, NativePinWrappedVaultKeysV1, NativePlatformFamily,
    NativePlatformStorageError, NativeSealedMaterialV1, NativeSecureCompartment,
    NativeVaultCryptoError, PHASE15I_MAX_ENCRYPTED_VAULT_BYTES,
};

pub const NATIVE_PASSPORT_PHASE15R_CORE_LABEL: &str =
    "NATIVE_PASSPORT_PHASE15R_PLATFORM_BOUND_VAULT_ENVELOPE";

pub const PHASE15R_PLATFORM_BOUND_VAULT_MAGIC: &[u8; 8] = b"RONPBF01";

pub const PHASE15R_PLATFORM_BOUND_VAULT_VERSION: u16 = 1;

pub const PHASE15R_REQUIRED_SEALED_FACTOR_COUNT: u8 = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativePlatformBoundVaultError {
    PlatformFamilyMismatch {
        expected: NativePlatformFamily,
        actual: NativePlatformFamily,
    },
    CompartmentMismatch {
        expected: NativeSecureCompartment,
        actual: NativeSecureCompartment,
    },
    InvalidEncodedVault,
    EncodedVaultTooLarge {
        actual: usize,
        maximum: usize,
    },
    UnsupportedVersion {
        actual: u16,
    },
    InvalidSealedFactorCount {
        actual: u8,
    },
    UnsupportedPlatformFamily {
        actual: NativePlatformFamily,
    },
    UnsupportedPlatformCode {
        actual: u8,
    },
    UnsupportedCompartmentCode {
        actual: u8,
    },
    PlatformStorageContractViolation,
    VaultCryptoContractViolation,
}

impl From<NativePlatformStorageError> for NativePlatformBoundVaultError {
    fn from(_: NativePlatformStorageError) -> Self {
        Self::PlatformStorageContractViolation
    }
}

impl From<NativeVaultCryptoError> for NativePlatformBoundVaultError {
    fn from(_: NativeVaultCryptoError) -> Self {
        Self::VaultCryptoContractViolation
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct NativePlatformBoundVaultV1 {
    platform_family: NativePlatformFamily,
    recovery_root_factor: NativeSealedMaterialV1,
    operational_factor: NativeSealedMaterialV1,
    wrapped_keys: NativePinWrappedVaultKeysV1,
}

impl NativePlatformBoundVaultV1 {
    pub fn new(
        platform_family: NativePlatformFamily,
        recovery_root_factor: NativeSealedMaterialV1,
        operational_factor: NativeSealedMaterialV1,
        wrapped_keys: NativePinWrappedVaultKeysV1,
    ) -> Result<Self, NativePlatformBoundVaultError> {
        validate_sealed_factor(
            platform_family,
            NativeSecureCompartment::RecoveryRoot,
            &recovery_root_factor,
        )?;

        validate_sealed_factor(
            platform_family,
            NativeSecureCompartment::DeviceKey,
            &operational_factor,
        )?;

        Ok(Self {
            platform_family,
            recovery_root_factor,
            operational_factor,
            wrapped_keys,
        })
    }

    pub fn platform_family(&self) -> NativePlatformFamily {
        self.platform_family
    }

    pub fn recovery_root_factor(&self) -> &NativeSealedMaterialV1 {
        &self.recovery_root_factor
    }

    pub fn operational_factor(&self) -> &NativeSealedMaterialV1 {
        &self.operational_factor
    }

    pub fn wrapped_keys(&self) -> &NativePinWrappedVaultKeysV1 {
        &self.wrapped_keys
    }
}

impl fmt::Debug for NativePlatformBoundVaultV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NativePlatformBoundVaultV1")
            .field("platform_family", &self.platform_family)
            .field("recovery_root_factor", &"REDACTED_PLATFORM_SEALED_MATERIAL")
            .field("operational_factor", &"REDACTED_PLATFORM_SEALED_MATERIAL")
            .field("wrapped_keys", &self.wrapped_keys)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativePlatformBoundVaultPosture {
    pub phase_label: &'static str,
    pub canonical_owner: &'static str,
    pub sealed_root_factor_persisted: bool,
    pub sealed_operational_factor_persisted: bool,
    pub phase15q_wrapped_keys_reused: bool,
    pub one_platform_family_bound: bool,
    pub bounded_binary_codec_added: bool,
    pub plaintext_platform_factor_persisted: bool,
    pub pin_persisted: bool,
    pub vmk_plaintext_persisted: bool,
    pub platform_backend_called: bool,
    pub filesystem_io_added: bool,
    pub tauri_command_added: bool,
    pub ron_kms_key_lifecycle_touched: bool,
    pub frontend_secret_custody_added: bool,
    pub wallet_or_ledger_mutation_added: bool,
}

pub fn native_platform_bound_vault_posture() -> NativePlatformBoundVaultPosture {
    NativePlatformBoundVaultPosture {
        phase_label: NATIVE_PASSPORT_PHASE15R_CORE_LABEL,
        canonical_owner: "svc-passport",
        sealed_root_factor_persisted: true,
        sealed_operational_factor_persisted: true,
        phase15q_wrapped_keys_reused: true,
        one_platform_family_bound: true,
        bounded_binary_codec_added: true,
        plaintext_platform_factor_persisted: false,
        pin_persisted: false,
        vmk_plaintext_persisted: false,
        platform_backend_called: false,
        filesystem_io_added: false,
        tauri_command_added: false,
        ron_kms_key_lifecycle_touched: false,
        frontend_secret_custody_added: false,
        wallet_or_ledger_mutation_added: false,
    }
}

pub fn encode_native_platform_bound_vault(
    vault: &NativePlatformBoundVaultV1,
) -> Result<NativeEncryptedVaultV1, NativePlatformBoundVaultError> {
    let wrapped_keys = encode_native_pin_wrapped_vault_keys(vault.wrapped_keys())?;

    let mut encoded = Vec::with_capacity(
        wrapped_keys.len()
            + vault.recovery_root_factor().len()
            + vault.operational_factor().len()
            + 64,
    );

    encoded.extend_from_slice(PHASE15R_PLATFORM_BOUND_VAULT_MAGIC);

    encoded.extend_from_slice(&PHASE15R_PLATFORM_BOUND_VAULT_VERSION.to_be_bytes());

    encoded.push(platform_code(vault.platform_family())?);

    encoded.push(PHASE15R_REQUIRED_SEALED_FACTOR_COUNT);

    encode_sealed_factor(&mut encoded, vault.recovery_root_factor())?;

    encode_sealed_factor(&mut encoded, vault.operational_factor())?;

    append_len_prefixed(&mut encoded, wrapped_keys.as_slice())?;

    if encoded.len() > PHASE15I_MAX_ENCRYPTED_VAULT_BYTES {
        return Err(NativePlatformBoundVaultError::EncodedVaultTooLarge {
            actual: encoded.len(),
            maximum: PHASE15I_MAX_ENCRYPTED_VAULT_BYTES,
        });
    }

    NativeEncryptedVaultV1::new(encoded).map_err(Into::into)
}

pub fn decode_native_platform_bound_vault(
    encoded: &NativeEncryptedVaultV1,
) -> Result<NativePlatformBoundVaultV1, NativePlatformBoundVaultError> {
    let bytes = encoded.as_slice();

    if bytes.len() > PHASE15I_MAX_ENCRYPTED_VAULT_BYTES {
        return Err(NativePlatformBoundVaultError::EncodedVaultTooLarge {
            actual: bytes.len(),
            maximum: PHASE15I_MAX_ENCRYPTED_VAULT_BYTES,
        });
    }

    let mut cursor = 0usize;

    let magic = read_array::<8>(bytes, &mut cursor)?;

    if &magic != PHASE15R_PLATFORM_BOUND_VAULT_MAGIC {
        return Err(NativePlatformBoundVaultError::InvalidEncodedVault);
    }

    let version = u16::from_be_bytes(read_array::<2>(bytes, &mut cursor)?);

    if version != PHASE15R_PLATFORM_BOUND_VAULT_VERSION {
        return Err(NativePlatformBoundVaultError::UnsupportedVersion { actual: version });
    }

    let platform_family = platform_from_code(read_u8(bytes, &mut cursor)?)?;

    let factor_count = read_u8(bytes, &mut cursor)?;

    if factor_count != PHASE15R_REQUIRED_SEALED_FACTOR_COUNT {
        return Err(NativePlatformBoundVaultError::InvalidSealedFactorCount {
            actual: factor_count,
        });
    }

    let recovery_root_factor = decode_sealed_factor(bytes, &mut cursor, platform_family)?;

    let operational_factor = decode_sealed_factor(bytes, &mut cursor, platform_family)?;

    let wrapped_key_bytes = read_len_prefixed(bytes, &mut cursor)?;

    if cursor != bytes.len() {
        return Err(NativePlatformBoundVaultError::InvalidEncodedVault);
    }

    let wrapped_key_envelope = NativeEncryptedVaultV1::new(wrapped_key_bytes)?;

    let wrapped_keys = decode_native_pin_wrapped_vault_keys(&wrapped_key_envelope)?;

    NativePlatformBoundVaultV1::new(
        platform_family,
        recovery_root_factor,
        operational_factor,
        wrapped_keys,
    )
}

fn validate_sealed_factor(
    expected_platform: NativePlatformFamily,
    expected_compartment: NativeSecureCompartment,
    factor: &NativeSealedMaterialV1,
) -> Result<(), NativePlatformBoundVaultError> {
    factor.validate()?;

    if factor.platform_family != expected_platform {
        return Err(NativePlatformBoundVaultError::PlatformFamilyMismatch {
            expected: expected_platform,
            actual: factor.platform_family,
        });
    }

    if factor.compartment != expected_compartment {
        return Err(NativePlatformBoundVaultError::CompartmentMismatch {
            expected: expected_compartment,
            actual: factor.compartment,
        });
    }

    Ok(())
}

fn platform_code(platform: NativePlatformFamily) -> Result<u8, NativePlatformBoundVaultError> {
    match platform {
        NativePlatformFamily::MacosKeychain => Ok(1),
        NativePlatformFamily::WindowsDpapi => Ok(2),
        NativePlatformFamily::LinuxSecretService => Ok(3),
        NativePlatformFamily::IosKeychain => Ok(4),
        NativePlatformFamily::AndroidKeystore => Ok(5),
        NativePlatformFamily::UnknownLocal => {
            Err(NativePlatformBoundVaultError::UnsupportedPlatformFamily {
                actual: NativePlatformFamily::UnknownLocal,
            })
        }
    }
}

fn platform_from_code(code: u8) -> Result<NativePlatformFamily, NativePlatformBoundVaultError> {
    match code {
        1 => Ok(NativePlatformFamily::MacosKeychain),
        2 => Ok(NativePlatformFamily::WindowsDpapi),
        3 => Ok(NativePlatformFamily::LinuxSecretService),
        4 => Ok(NativePlatformFamily::IosKeychain),
        5 => Ok(NativePlatformFamily::AndroidKeystore),
        actual => Err(NativePlatformBoundVaultError::UnsupportedPlatformCode { actual }),
    }
}

fn compartment_code(compartment: NativeSecureCompartment) -> u8 {
    match compartment {
        NativeSecureCompartment::RecoveryRoot => 1,
        NativeSecureCompartment::DeviceKey => 2,
    }
}

fn compartment_from_code(
    code: u8,
) -> Result<NativeSecureCompartment, NativePlatformBoundVaultError> {
    match code {
        1 => Ok(NativeSecureCompartment::RecoveryRoot),
        2 => Ok(NativeSecureCompartment::DeviceKey),
        actual => Err(NativePlatformBoundVaultError::UnsupportedCompartmentCode { actual }),
    }
}

fn encode_sealed_factor(
    output: &mut Vec<u8>,
    factor: &NativeSealedMaterialV1,
) -> Result<(), NativePlatformBoundVaultError> {
    factor.validate()?;

    output.push(compartment_code(factor.compartment));

    append_len_prefixed(output, factor.as_slice())
}

fn decode_sealed_factor(
    bytes: &[u8],
    cursor: &mut usize,
    platform_family: NativePlatformFamily,
) -> Result<NativeSealedMaterialV1, NativePlatformBoundVaultError> {
    let compartment = compartment_from_code(read_u8(bytes, cursor)?)?;

    let sealed_bytes = read_len_prefixed(bytes, cursor)?;

    NativeSealedMaterialV1::new(platform_family, compartment, sealed_bytes).map_err(Into::into)
}

fn append_len_prefixed(
    output: &mut Vec<u8>,
    value: &[u8],
) -> Result<(), NativePlatformBoundVaultError> {
    let length = u32::try_from(value.len())
        .map_err(|_| NativePlatformBoundVaultError::InvalidEncodedVault)?;

    output.extend_from_slice(&length.to_be_bytes());

    output.extend_from_slice(value);

    Ok(())
}

fn read_u8(bytes: &[u8], cursor: &mut usize) -> Result<u8, NativePlatformBoundVaultError> {
    let value = *bytes
        .get(*cursor)
        .ok_or(NativePlatformBoundVaultError::InvalidEncodedVault)?;

    *cursor += 1;

    Ok(value)
}

fn read_array<const N: usize>(
    bytes: &[u8],
    cursor: &mut usize,
) -> Result<[u8; N], NativePlatformBoundVaultError> {
    let end = cursor
        .checked_add(N)
        .ok_or(NativePlatformBoundVaultError::InvalidEncodedVault)?;

    let source = bytes
        .get(*cursor..end)
        .ok_or(NativePlatformBoundVaultError::InvalidEncodedVault)?;

    let mut output = [0u8; N];
    output.copy_from_slice(source);

    *cursor = end;

    Ok(output)
}

fn read_len_prefixed(
    bytes: &[u8],
    cursor: &mut usize,
) -> Result<Vec<u8>, NativePlatformBoundVaultError> {
    let length = usize::try_from(u32::from_be_bytes(read_array::<4>(bytes, cursor)?))
        .map_err(|_| NativePlatformBoundVaultError::InvalidEncodedVault)?;

    let end = cursor
        .checked_add(length)
        .ok_or(NativePlatformBoundVaultError::InvalidEncodedVault)?;

    let source = bytes
        .get(*cursor..end)
        .ok_or(NativePlatformBoundVaultError::InvalidEncodedVault)?;

    *cursor = end;

    Ok(source.to_vec())
}
