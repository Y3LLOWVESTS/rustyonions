//! RO:WHAT — Native Passport PIN-derived and platform-factor-bound VMK wrapping plus a bounded deterministic vault-key codec.
//! RO:WHY — Phase 15Q adds the first real local vault cryptography while preserving the established Passport vault protocol and ron-kms authority boundary.
//! RO:INTERACTS — Phase 0I authenticated-header fixture, Phase 5 compartment labels, Phase 6 vault/PIN contracts, Phase 15I secret/encrypted-vault wrappers, and future Tauri PlatformSealer/VaultStore orchestration.
//! RO:INVARIANTS — one local PIN may protect both independently wrapped compartments; each compartment uses a distinct salt, nonce, platform factor, Argon2id profile, HKDF domain, and random VMK; ordinary unlock exposes only the device/operational VMK.
//! RO:SECURITY — no PIN persistence, root unlock API, OS adapter call, filesystem I/O, Tauri command, frontend secret DTO, Ed25519/KMS key lifecycle, capability issuance, wallet mutation, or ledger mutation.
//! RO:TEST — module fixture-compatibility tests and tests/native_passport_phase15q_native_vault_crypto_and_operational_unlock.rs.

use std::fmt;

use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::{
    aead::{Aead, KeyInit, Payload},
    Key, XChaCha20Poly1305, XNonce,
};
use hkdf::Hkdf;
use sha2::Sha256;
use zeroize::Zeroizing;

use super::{
    NativeEncryptedVaultV1, NativePlatformStorageError, NativeSecretBytes, NativeSecureCompartment,
    PHASE5B_DEVICE_KEY_COMPARTMENT_LABEL, PHASE5B_RECOVERY_ROOT_COMPARTMENT_LABEL,
    PHASE6A_AEAD_NONCE_LEN, PHASE6A_AEAD_TAG_LEN, PHASE6A_AUTHENTICATED_HEADER_LABEL,
    PHASE6A_KDF_SALT_LEN, PHASE6A_TWO_COMPARTMENT_VAULT_HEADER_DOMAIN,
    PHASE6A_VAULT_HEADER_VERSION, PHASE6B_MAX_PIN_LENGTH, PHASE6B_MIN_PIN_LENGTH,
};

pub const NATIVE_PASSPORT_PHASE15Q_LABEL: &str =
    "NATIVE_PASSPORT_PHASE15Q_NATIVE_VAULT_CRYPTO_AND_OPERATIONAL_UNLOCK";

pub const PHASE15Q_AUTHENTICATED_HEADER_DOMAIN: &str =
    "rustyonions.native-passport.vault-header.v1";

pub const PHASE15Q_AUTHENTICATED_HEADER_ENCODING: &str = "pipe-delimited-canonical-v1";

pub const PHASE15Q_AUTHENTICATED_HEADER_INPUT_FORMAT: &str =
    "utf8:domain|version|compartment_kind|kdf|kdf_version|kdf_params|salt_hex|aead|nonce_hex|compartment_purpose|domain_tag";

pub const PHASE15Q_KDF_DISPLAY_LABEL: &str = "Argon2id";

pub const PHASE15Q_KDF_VERSION_LABEL: &str = "v1";

pub const PHASE15Q_AEAD_DISPLAY_LABEL: &str = "XChaCha20Poly1305";

pub const PHASE15Q_KEK_DOMAIN: &str = "rustyonions.native-passport.vault-kek.v1";

pub const PHASE15Q_ROOT_DOMAIN_TAG: &str = "rustyonions.native-passport.vault-root-compartment.v1";

pub const PHASE15Q_OPERATIONAL_DOMAIN_TAG: &str =
    "rustyonions.native-passport.vault-device-compartment.v1";

pub const PHASE15Q_ROOT_PURPOSE: &str =
    "future encrypted custody for recovery phrase and Passport root key material";

pub const PHASE15Q_OPERATIONAL_PURPOSE: &str =
    "future encrypted custody for local device key material";

pub const PHASE15Q_ROOT_MEMORY_MIB: u32 = 64;

pub const PHASE15Q_OPERATIONAL_MEMORY_MIB: u32 = 32;

pub const PHASE15Q_ARGON2_TIME_COST: u32 = 3;

pub const PHASE15Q_ARGON2_PARALLELISM: u32 = 1;

pub const PHASE15Q_DERIVED_KEY_BYTES: usize = 32;

pub const PHASE15Q_PLATFORM_FACTOR_BYTES: usize = 32;

pub const PHASE15Q_VAULT_MASTER_KEY_BYTES: usize = 32;

pub const PHASE15Q_WRAPPED_VMK_BYTES: usize =
    PHASE15Q_VAULT_MASTER_KEY_BYTES + PHASE6A_AEAD_TAG_LEN;

pub const PHASE15Q_VAULT_KEY_CODEC_MAGIC: &[u8; 8] = b"RONPVK01";

pub const PHASE15Q_VAULT_KEY_CODEC_VERSION: u16 = 1;

pub const PHASE15Q_REQUIRED_COMPARTMENT_COUNT: u8 = 2;

pub const PHASE15Q_MAX_ENCODED_VAULT_KEY_BYTES: usize = 4_096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeVaultKdfProfileV1 {
    pub memory_mib: u32,
    pub time_cost: u32,
    pub parallelism: u32,
    pub output_len_bytes: usize,
}

pub const PHASE15Q_ROOT_KDF_PROFILE: NativeVaultKdfProfileV1 = NativeVaultKdfProfileV1 {
    memory_mib: PHASE15Q_ROOT_MEMORY_MIB,
    time_cost: PHASE15Q_ARGON2_TIME_COST,
    parallelism: PHASE15Q_ARGON2_PARALLELISM,
    output_len_bytes: PHASE15Q_DERIVED_KEY_BYTES,
};

pub const PHASE15Q_OPERATIONAL_KDF_PROFILE: NativeVaultKdfProfileV1 = NativeVaultKdfProfileV1 {
    memory_mib: PHASE15Q_OPERATIONAL_MEMORY_MIB,
    time_cost: PHASE15Q_ARGON2_TIME_COST,
    parallelism: PHASE15Q_ARGON2_PARALLELISM,
    output_len_bytes: PHASE15Q_DERIVED_KEY_BYTES,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeVaultCryptoError {
    InvalidPinLength {
        actual: usize,
        minimum: usize,
        maximum: usize,
    },
    InvalidPlatformFactorLength {
        actual: usize,
        expected: usize,
    },
    InvalidVaultMasterKeyLength {
        actual: usize,
        expected: usize,
    },
    InvalidWrappedVmkLength {
        actual: usize,
        expected: usize,
    },
    InvalidAuthenticatedHeader,
    InvalidEncodedVault,
    EncodedVaultTooLarge {
        actual: usize,
        maximum: usize,
    },
    UnsupportedCodecVersion {
        actual: u16,
    },
    InvalidCompartmentCount {
        actual: u8,
    },
    UnsupportedCompartmentCode {
        actual: u8,
    },
    UnexpectedCompartment {
        expected: NativeSecureCompartment,
        actual: NativeSecureCompartment,
    },
    OperationalCompartmentRequired,
    CompartmentMaterialReuse,
    KdfFailure,
    KekDerivationFailure,
    EncryptionFailure,
    AuthenticationFailed,
    StorageContractViolation,
}

impl From<NativePlatformStorageError> for NativeVaultCryptoError {
    fn from(_: NativePlatformStorageError) -> Self {
        Self::StorageContractViolation
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct NativePinWrappedCompartmentVmkV1 {
    compartment: NativeSecureCompartment,
    salt: [u8; PHASE6A_KDF_SALT_LEN],
    nonce: [u8; PHASE6A_AEAD_NONCE_LEN],
    authenticated_header: Vec<u8>,
    wrapped_vmk: Vec<u8>,
}

impl NativePinWrappedCompartmentVmkV1 {
    pub fn new(
        compartment: NativeSecureCompartment,
        salt: [u8; PHASE6A_KDF_SALT_LEN],
        nonce: [u8; PHASE6A_AEAD_NONCE_LEN],
        authenticated_header: Vec<u8>,
        wrapped_vmk: Vec<u8>,
    ) -> Result<Self, NativeVaultCryptoError> {
        if wrapped_vmk.len() != PHASE15Q_WRAPPED_VMK_BYTES {
            return Err(NativeVaultCryptoError::InvalidWrappedVmkLength {
                actual: wrapped_vmk.len(),
                expected: PHASE15Q_WRAPPED_VMK_BYTES,
            });
        }

        let expected_header = encode_authenticated_header_bytes(compartment, &salt, &nonce);

        if authenticated_header != expected_header {
            return Err(NativeVaultCryptoError::InvalidAuthenticatedHeader);
        }

        Ok(Self {
            compartment,
            salt,
            nonce,
            authenticated_header,
            wrapped_vmk,
        })
    }

    pub fn compartment(&self) -> NativeSecureCompartment {
        self.compartment
    }

    pub fn salt(&self) -> &[u8; PHASE6A_KDF_SALT_LEN] {
        &self.salt
    }

    pub fn nonce(&self) -> &[u8; PHASE6A_AEAD_NONCE_LEN] {
        &self.nonce
    }

    pub fn authenticated_header(&self) -> &[u8] {
        &self.authenticated_header
    }

    pub fn wrapped_vmk_len(&self) -> usize {
        self.wrapped_vmk.len()
    }

    fn wrapped_vmk(&self) -> &[u8] {
        &self.wrapped_vmk
    }
}

impl fmt::Debug for NativePinWrappedCompartmentVmkV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NativePinWrappedCompartmentVmkV1")
            .field("compartment", &self.compartment)
            .field("salt", &"PUBLIC_REDACTED_LENGTH_16")
            .field("nonce", &"PUBLIC_REDACTED_LENGTH_24")
            .field(
                "authenticated_header_length",
                &self.authenticated_header.len(),
            )
            .field("wrapped_vmk", &"REDACTED_AUTHENTICATED_CIPHERTEXT")
            .field("wrapped_vmk_length", &self.wrapped_vmk.len())
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct NativePinWrappedVaultKeysV1 {
    recovery_root: NativePinWrappedCompartmentVmkV1,
    operational: NativePinWrappedCompartmentVmkV1,
}

impl NativePinWrappedVaultKeysV1 {
    pub fn new(
        recovery_root: NativePinWrappedCompartmentVmkV1,
        operational: NativePinWrappedCompartmentVmkV1,
    ) -> Result<Self, NativeVaultCryptoError> {
        if recovery_root.compartment() != NativeSecureCompartment::RecoveryRoot {
            return Err(NativeVaultCryptoError::UnexpectedCompartment {
                expected: NativeSecureCompartment::RecoveryRoot,
                actual: recovery_root.compartment(),
            });
        }

        if operational.compartment() != NativeSecureCompartment::DeviceKey {
            return Err(NativeVaultCryptoError::UnexpectedCompartment {
                expected: NativeSecureCompartment::DeviceKey,
                actual: operational.compartment(),
            });
        }

        if recovery_root.salt() == operational.salt()
            || recovery_root.nonce() == operational.nonce()
        {
            return Err(NativeVaultCryptoError::CompartmentMaterialReuse);
        }

        Ok(Self {
            recovery_root,
            operational,
        })
    }

    pub fn recovery_root(&self) -> &NativePinWrappedCompartmentVmkV1 {
        &self.recovery_root
    }

    pub fn operational(&self) -> &NativePinWrappedCompartmentVmkV1 {
        &self.operational
    }
}

impl fmt::Debug for NativePinWrappedVaultKeysV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NativePinWrappedVaultKeysV1")
            .field("codec_version", &PHASE15Q_VAULT_KEY_CODEC_VERSION)
            .field("recovery_root", &self.recovery_root)
            .field("operational", &self.operational)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeVaultCryptoPosture {
    pub phase_label: &'static str,
    pub canonical_owner: &'static str,
    pub phase0i_header_encoding_preserved: bool,
    pub phase0i_fixture_salt_is_contract_only: bool,
    pub phase6_runtime_salt_length_preserved: bool,
    pub argon2id_pin_key_added: bool,
    pub hkdf_sha256_kek_added: bool,
    pub xchacha20poly1305_vmk_wrap_added: bool,
    pub same_pin_may_protect_both_compartments: bool,
    pub platform_factor_required_for_each_wrap: bool,
    pub ordinary_unlock_is_operational_only: bool,
    pub public_root_unlock_api_added: bool,
    pub ron_kms_key_lifecycle_touched: bool,
    pub ed25519_signing_or_verification_added: bool,
    pub pin_persisted: bool,
    pub platform_sealer_called: bool,
    pub filesystem_io_added: bool,
    pub tauri_command_added: bool,
    pub frontend_secret_dto_added: bool,
    pub wallet_or_ledger_mutation_added: bool,
}

pub fn native_vault_crypto_posture() -> NativeVaultCryptoPosture {
    NativeVaultCryptoPosture {
        phase_label: NATIVE_PASSPORT_PHASE15Q_LABEL,
        canonical_owner: "svc-passport",
        phase0i_header_encoding_preserved: true,
        phase0i_fixture_salt_is_contract_only: true,
        phase6_runtime_salt_length_preserved: true,
        argon2id_pin_key_added: true,
        hkdf_sha256_kek_added: true,
        xchacha20poly1305_vmk_wrap_added: true,
        same_pin_may_protect_both_compartments: true,
        platform_factor_required_for_each_wrap: true,
        ordinary_unlock_is_operational_only: true,
        public_root_unlock_api_added: false,
        ron_kms_key_lifecycle_touched: false,
        ed25519_signing_or_verification_added: false,
        pin_persisted: false,
        platform_sealer_called: false,
        filesystem_io_added: false,
        tauri_command_added: false,
        frontend_secret_dto_added: false,
        wallet_or_ledger_mutation_added: false,
    }
}

pub fn encode_native_vault_authenticated_header(
    compartment: NativeSecureCompartment,
    salt: &[u8; PHASE6A_KDF_SALT_LEN],
    nonce: &[u8; PHASE6A_AEAD_NONCE_LEN],
) -> Vec<u8> {
    encode_authenticated_header_bytes(compartment, salt, nonce)
}

pub fn native_vault_kek_info(compartment: NativeSecureCompartment) -> Vec<u8> {
    format!(
        "{}|{}|{}|{}|{}",
        PHASE15Q_KEK_DOMAIN,
        PHASE6A_TWO_COMPARTMENT_VAULT_HEADER_DOMAIN,
        PHASE6A_VAULT_HEADER_VERSION,
        compartment_label(compartment),
        PHASE6A_AUTHENTICATED_HEADER_LABEL,
    )
    .into_bytes()
}

pub fn wrap_native_compartment_vmk(
    compartment: NativeSecureCompartment,
    pin: &[u8],
    platform_factor: &NativeSecretBytes,
    salt: &[u8; PHASE6A_KDF_SALT_LEN],
    nonce: &[u8; PHASE6A_AEAD_NONCE_LEN],
    vault_master_key: &NativeSecretBytes,
) -> Result<NativePinWrappedCompartmentVmkV1, NativeVaultCryptoError> {
    validate_pin(pin)?;
    validate_platform_factor(platform_factor)?;
    validate_vault_master_key(vault_master_key)?;

    let pin_key = derive_pin_key(compartment, pin, salt)?;

    let kek = derive_compartment_kek(compartment, &pin_key, platform_factor)?;

    let authenticated_header = encode_authenticated_header_bytes(compartment, salt, nonce);

    let key = Key::from(*kek);
    let cipher = XChaCha20Poly1305::new(&key);
    let xnonce = XNonce::from(*nonce);

    let wrapped_vmk = cipher
        .encrypt(
            &xnonce,
            Payload {
                msg: vault_master_key.as_slice(),
                aad: &authenticated_header,
            },
        )
        .map_err(|_| NativeVaultCryptoError::EncryptionFailure)?;

    NativePinWrappedCompartmentVmkV1::new(
        compartment,
        *salt,
        *nonce,
        authenticated_header,
        wrapped_vmk,
    )
}

pub fn unlock_native_operational_vmk(
    envelope: &NativePinWrappedCompartmentVmkV1,
    pin: &[u8],
    platform_factor: &NativeSecretBytes,
) -> Result<NativeSecretBytes, NativeVaultCryptoError> {
    if envelope.compartment() != NativeSecureCompartment::DeviceKey {
        return Err(NativeVaultCryptoError::OperationalCompartmentRequired);
    }

    validate_pin(pin)?;
    validate_platform_factor(platform_factor)?;

    decrypt_compartment_vmk(envelope, pin, platform_factor)
}

/// Verify the native recovery-root PIN without returning or retaining the
/// recovery-root VMK.
///
/// Successful authenticated decryption proves that the supplied PIN and
/// platform-sealed recovery factor match the RecoveryRoot compartment. The
/// decrypted VMK remains in `NativeSecretBytes` only long enough to validate
/// authentication and is dropped before this function returns.
pub fn verify_native_recovery_root_pin(
    envelope: &NativePinWrappedCompartmentVmkV1,
    pin: &[u8],
    platform_factor: &NativeSecretBytes,
) -> Result<(), NativeVaultCryptoError> {
    if envelope.compartment() != NativeSecureCompartment::RecoveryRoot {
        return Err(NativeVaultCryptoError::UnexpectedCompartment {
            expected: NativeSecureCompartment::RecoveryRoot,
            actual: envelope.compartment(),
        });
    }

    validate_pin(pin)?;
    validate_platform_factor(platform_factor)?;

    let verified_root_vmk = decrypt_compartment_vmk(envelope, pin, platform_factor)?;

    drop(verified_root_vmk);

    Ok(())
}

pub fn encode_native_pin_wrapped_vault_keys(
    vault_keys: &NativePinWrappedVaultKeysV1,
) -> Result<NativeEncryptedVaultV1, NativeVaultCryptoError> {
    let mut encoded = Vec::with_capacity(1_024);

    encoded.extend_from_slice(PHASE15Q_VAULT_KEY_CODEC_MAGIC);

    encoded.extend_from_slice(&PHASE15Q_VAULT_KEY_CODEC_VERSION.to_be_bytes());

    encoded.push(PHASE15Q_REQUIRED_COMPARTMENT_COUNT);

    encode_compartment(&mut encoded, vault_keys.recovery_root())?;

    encode_compartment(&mut encoded, vault_keys.operational())?;

    if encoded.len() > PHASE15Q_MAX_ENCODED_VAULT_KEY_BYTES {
        return Err(NativeVaultCryptoError::EncodedVaultTooLarge {
            actual: encoded.len(),
            maximum: PHASE15Q_MAX_ENCODED_VAULT_KEY_BYTES,
        });
    }

    NativeEncryptedVaultV1::new(encoded).map_err(Into::into)
}

pub fn decode_native_pin_wrapped_vault_keys(
    encoded: &NativeEncryptedVaultV1,
) -> Result<NativePinWrappedVaultKeysV1, NativeVaultCryptoError> {
    let bytes = encoded.as_slice();

    if bytes.len() > PHASE15Q_MAX_ENCODED_VAULT_KEY_BYTES {
        return Err(NativeVaultCryptoError::EncodedVaultTooLarge {
            actual: bytes.len(),
            maximum: PHASE15Q_MAX_ENCODED_VAULT_KEY_BYTES,
        });
    }

    let mut cursor = 0usize;

    let magic = read_array::<8>(bytes, &mut cursor)?;

    if &magic != PHASE15Q_VAULT_KEY_CODEC_MAGIC {
        return Err(NativeVaultCryptoError::InvalidEncodedVault);
    }

    let version = u16::from_be_bytes(read_array::<2>(bytes, &mut cursor)?);

    if version != PHASE15Q_VAULT_KEY_CODEC_VERSION {
        return Err(NativeVaultCryptoError::UnsupportedCodecVersion { actual: version });
    }

    let compartment_count = read_u8(bytes, &mut cursor)?;

    if compartment_count != PHASE15Q_REQUIRED_COMPARTMENT_COUNT {
        return Err(NativeVaultCryptoError::InvalidCompartmentCount {
            actual: compartment_count,
        });
    }

    let recovery_root = decode_compartment(bytes, &mut cursor)?;

    let operational = decode_compartment(bytes, &mut cursor)?;

    if cursor != bytes.len() {
        return Err(NativeVaultCryptoError::InvalidEncodedVault);
    }

    NativePinWrappedVaultKeysV1::new(recovery_root, operational)
}

fn decrypt_compartment_vmk(
    envelope: &NativePinWrappedCompartmentVmkV1,
    pin: &[u8],
    platform_factor: &NativeSecretBytes,
) -> Result<NativeSecretBytes, NativeVaultCryptoError> {
    let expected_header = encode_authenticated_header_bytes(
        envelope.compartment(),
        envelope.salt(),
        envelope.nonce(),
    );

    if envelope.authenticated_header() != expected_header {
        return Err(NativeVaultCryptoError::InvalidAuthenticatedHeader);
    }

    let pin_key = derive_pin_key(envelope.compartment(), pin, envelope.salt())?;

    let kek = derive_compartment_kek(envelope.compartment(), &pin_key, platform_factor)?;

    let key = Key::from(*kek);
    let cipher = XChaCha20Poly1305::new(&key);
    let xnonce = XNonce::from(*envelope.nonce());

    let plaintext = cipher
        .decrypt(
            &xnonce,
            Payload {
                msg: envelope.wrapped_vmk(),
                aad: envelope.authenticated_header(),
            },
        )
        .map_err(|_| NativeVaultCryptoError::AuthenticationFailed)?;

    if plaintext.len() != PHASE15Q_VAULT_MASTER_KEY_BYTES {
        return Err(NativeVaultCryptoError::AuthenticationFailed);
    }

    NativeSecretBytes::new(plaintext).map_err(Into::into)
}

fn derive_pin_key(
    compartment: NativeSecureCompartment,
    pin: &[u8],
    salt: &[u8; PHASE6A_KDF_SALT_LEN],
) -> Result<Zeroizing<[u8; PHASE15Q_DERIVED_KEY_BYTES]>, NativeVaultCryptoError> {
    let profile = kdf_profile(compartment);

    let memory_kib = profile
        .memory_mib
        .checked_mul(1_024)
        .ok_or(NativeVaultCryptoError::KdfFailure)?;

    let params = Params::new(
        memory_kib,
        profile.time_cost,
        profile.parallelism,
        Some(profile.output_len_bytes),
    )
    .map_err(|_| NativeVaultCryptoError::KdfFailure)?;

    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

    let mut pin_key = Zeroizing::new([0u8; PHASE15Q_DERIVED_KEY_BYTES]);

    argon2
        .hash_password_into(pin, salt, &mut pin_key[..])
        .map_err(|_| NativeVaultCryptoError::KdfFailure)?;

    Ok(pin_key)
}

fn derive_compartment_kek(
    compartment: NativeSecureCompartment,
    pin_key: &[u8; PHASE15Q_DERIVED_KEY_BYTES],
    platform_factor: &NativeSecretBytes,
) -> Result<Zeroizing<[u8; PHASE15Q_DERIVED_KEY_BYTES]>, NativeVaultCryptoError> {
    let hkdf = Hkdf::<Sha256>::new(Some(platform_factor.as_slice()), pin_key);

    let info = native_vault_kek_info(compartment);

    let mut kek = Zeroizing::new([0u8; PHASE15Q_DERIVED_KEY_BYTES]);

    hkdf.expand(&info, &mut kek[..])
        .map_err(|_| NativeVaultCryptoError::KekDerivationFailure)?;

    Ok(kek)
}

fn validate_pin(pin: &[u8]) -> Result<(), NativeVaultCryptoError> {
    let minimum = usize::from(PHASE6B_MIN_PIN_LENGTH);

    let maximum = usize::from(PHASE6B_MAX_PIN_LENGTH);

    if pin.len() < minimum || pin.len() > maximum {
        return Err(NativeVaultCryptoError::InvalidPinLength {
            actual: pin.len(),
            minimum,
            maximum,
        });
    }

    Ok(())
}

fn validate_platform_factor(
    platform_factor: &NativeSecretBytes,
) -> Result<(), NativeVaultCryptoError> {
    if platform_factor.len() != PHASE15Q_PLATFORM_FACTOR_BYTES {
        return Err(NativeVaultCryptoError::InvalidPlatformFactorLength {
            actual: platform_factor.len(),
            expected: PHASE15Q_PLATFORM_FACTOR_BYTES,
        });
    }

    Ok(())
}

fn validate_vault_master_key(
    vault_master_key: &NativeSecretBytes,
) -> Result<(), NativeVaultCryptoError> {
    if vault_master_key.len() != PHASE15Q_VAULT_MASTER_KEY_BYTES {
        return Err(NativeVaultCryptoError::InvalidVaultMasterKeyLength {
            actual: vault_master_key.len(),
            expected: PHASE15Q_VAULT_MASTER_KEY_BYTES,
        });
    }

    Ok(())
}

fn kdf_profile(compartment: NativeSecureCompartment) -> NativeVaultKdfProfileV1 {
    match compartment {
        NativeSecureCompartment::RecoveryRoot => PHASE15Q_ROOT_KDF_PROFILE,
        NativeSecureCompartment::DeviceKey => PHASE15Q_OPERATIONAL_KDF_PROFILE,
    }
}

fn encode_authenticated_header_bytes(
    compartment: NativeSecureCompartment,
    salt: &[u8],
    nonce: &[u8],
) -> Vec<u8> {
    let profile = kdf_profile(compartment);

    format!(
        "{}|{}|{}|{}|{}|m={},t={},p={},dk={}|{}|{}|{}|{}|{}",
        PHASE15Q_AUTHENTICATED_HEADER_DOMAIN,
        PHASE6A_VAULT_HEADER_VERSION,
        compartment_label(compartment),
        PHASE15Q_KDF_DISPLAY_LABEL,
        PHASE15Q_KDF_VERSION_LABEL,
        profile.memory_mib,
        profile.time_cost,
        profile.parallelism,
        profile.output_len_bytes,
        lower_hex(salt),
        PHASE15Q_AEAD_DISPLAY_LABEL,
        lower_hex(nonce),
        compartment_purpose(compartment),
        compartment_domain_tag(compartment),
    )
    .into_bytes()
}

fn lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let mut output = String::with_capacity(bytes.len() * 2);

    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));

        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }

    output
}

fn compartment_label(compartment: NativeSecureCompartment) -> &'static str {
    match compartment {
        NativeSecureCompartment::RecoveryRoot => PHASE5B_RECOVERY_ROOT_COMPARTMENT_LABEL,
        NativeSecureCompartment::DeviceKey => PHASE5B_DEVICE_KEY_COMPARTMENT_LABEL,
    }
}

fn compartment_purpose(compartment: NativeSecureCompartment) -> &'static str {
    match compartment {
        NativeSecureCompartment::RecoveryRoot => PHASE15Q_ROOT_PURPOSE,
        NativeSecureCompartment::DeviceKey => PHASE15Q_OPERATIONAL_PURPOSE,
    }
}

fn compartment_domain_tag(compartment: NativeSecureCompartment) -> &'static str {
    match compartment {
        NativeSecureCompartment::RecoveryRoot => PHASE15Q_ROOT_DOMAIN_TAG,
        NativeSecureCompartment::DeviceKey => PHASE15Q_OPERATIONAL_DOMAIN_TAG,
    }
}

fn compartment_code(compartment: NativeSecureCompartment) -> u8 {
    match compartment {
        NativeSecureCompartment::RecoveryRoot => 1,
        NativeSecureCompartment::DeviceKey => 2,
    }
}

fn compartment_from_code(code: u8) -> Result<NativeSecureCompartment, NativeVaultCryptoError> {
    match code {
        1 => Ok(NativeSecureCompartment::RecoveryRoot),
        2 => Ok(NativeSecureCompartment::DeviceKey),
        actual => Err(NativeVaultCryptoError::UnsupportedCompartmentCode { actual }),
    }
}

fn encode_compartment(
    output: &mut Vec<u8>,
    envelope: &NativePinWrappedCompartmentVmkV1,
) -> Result<(), NativeVaultCryptoError> {
    output.push(compartment_code(envelope.compartment()));

    output.extend_from_slice(envelope.salt());

    output.extend_from_slice(envelope.nonce());

    append_len_prefixed(output, envelope.authenticated_header())?;

    append_len_prefixed(output, envelope.wrapped_vmk())?;

    Ok(())
}

fn decode_compartment(
    bytes: &[u8],
    cursor: &mut usize,
) -> Result<NativePinWrappedCompartmentVmkV1, NativeVaultCryptoError> {
    let compartment = compartment_from_code(read_u8(bytes, cursor)?)?;

    let salt = read_array::<PHASE6A_KDF_SALT_LEN>(bytes, cursor)?;

    let nonce = read_array::<PHASE6A_AEAD_NONCE_LEN>(bytes, cursor)?;

    let authenticated_header = read_len_prefixed(bytes, cursor)?;

    let wrapped_vmk = read_len_prefixed(bytes, cursor)?;

    NativePinWrappedCompartmentVmkV1::new(
        compartment,
        salt,
        nonce,
        authenticated_header,
        wrapped_vmk,
    )
}

fn append_len_prefixed(output: &mut Vec<u8>, value: &[u8]) -> Result<(), NativeVaultCryptoError> {
    let length =
        u16::try_from(value.len()).map_err(|_| NativeVaultCryptoError::InvalidEncodedVault)?;

    output.extend_from_slice(&length.to_be_bytes());

    output.extend_from_slice(value);

    Ok(())
}

fn read_u8(bytes: &[u8], cursor: &mut usize) -> Result<u8, NativeVaultCryptoError> {
    let value = *bytes
        .get(*cursor)
        .ok_or(NativeVaultCryptoError::InvalidEncodedVault)?;

    *cursor += 1;

    Ok(value)
}

fn read_array<const N: usize>(
    bytes: &[u8],
    cursor: &mut usize,
) -> Result<[u8; N], NativeVaultCryptoError> {
    let end = cursor
        .checked_add(N)
        .ok_or(NativeVaultCryptoError::InvalidEncodedVault)?;

    let source = bytes
        .get(*cursor..end)
        .ok_or(NativeVaultCryptoError::InvalidEncodedVault)?;

    let mut output = [0u8; N];
    output.copy_from_slice(source);

    *cursor = end;

    Ok(output)
}

fn read_len_prefixed(bytes: &[u8], cursor: &mut usize) -> Result<Vec<u8>, NativeVaultCryptoError> {
    let length = usize::from(u16::from_be_bytes(read_array::<2>(bytes, cursor)?));

    let end = cursor
        .checked_add(length)
        .ok_or(NativeVaultCryptoError::InvalidEncodedVault)?;

    let source = bytes
        .get(*cursor..end)
        .ok_or(NativeVaultCryptoError::InvalidEncodedVault)?;

    *cursor = end;

    Ok(source.to_vec())
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;

    const PHASE0I_FIXTURE: &str =
        include_str!("../../tests/vectors/native_passport_vault_header_v1.json");

    #[test]
    fn phase15q_authenticated_header_builder_reproduces_phase0i_fixture_encoding() {
        let value: Value =
            serde_json::from_str(PHASE0I_FIXTURE).expect("Phase 0I vault fixture JSON");

        let compartments = value["compartments"]
            .as_array()
            .expect("Phase 0I compartments");

        for fixture in compartments {
            let compartment = match fixture["compartment_kind"]
                .as_str()
                .expect("compartment kind")
            {
                "passport_root_compartment" => NativeSecureCompartment::RecoveryRoot,
                "device_compartment" => NativeSecureCompartment::DeviceKey,
                other => {
                    panic!("unexpected Phase 0I compartment: {other}")
                }
            };

            let salt = decode_hex(fixture["salt_hex"].as_str().expect("salt hex"));

            let nonce = decode_hex(fixture["nonce_hex"].as_str().expect("nonce hex"));

            let encoded = encode_authenticated_header_bytes(compartment, &salt, &nonce);

            assert_eq!(
                encoded,
                fixture["authenticated_header_input_utf8"]
                    .as_str()
                    .expect("authenticated header input",)
                    .as_bytes(),
            );
        }
    }

    fn decode_hex(input: &str) -> Vec<u8> {
        assert_eq!(input.len() % 2, 0,);

        input
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| {
                let high = hex_nibble(pair[0]);

                let low = hex_nibble(pair[1]);

                (high << 4) | low
            })
            .collect()
    }

    fn hex_nibble(value: u8) -> u8 {
        match value {
            b'0'..=b'9' => value - b'0',
            b'a'..=b'f' => value - b'a' + 10,
            b'A'..=b'F' => value - b'A' + 10,
            _ => {
                panic!("invalid hex nibble")
            }
        }
    }
}
