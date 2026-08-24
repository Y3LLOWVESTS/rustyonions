//! RO:WHAT — Defines the V2 platform-bound Native Passport vault envelope and pure V1-to-V2 migration preparation.
//! RO:WHY — Physical M1 needs to persist the authenticated operational device payload without mutating or ambiguously extending the released V1 envelope.
//! RO:INTERACTS — Phase 15R NativePlatformBoundVaultV1 codec, encrypted operational-device payload codec, NativeEncryptedVaultV1 bounded storage envelope, and future CrabLink atomic migration orchestration.
//! RO:INVARIANTS — V1 remains independently decodable; V2 embeds one canonical encoded V1 base plus one bounded encrypted operational payload; version dispatch is explicit; decode rejects truncation, trailing bytes, malformed lengths, and unsupported versions.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — sealed factors and authenticated ciphertext only; no plaintext device key, PIN, VMK, root material, PlatformSealer call, filesystem I/O, Tauri command, capability issuance, wallet mutation, or ledger mutation.
//! RO:TEST — tests/physical_m1_platform_bound_vault_v2_migration.rs plus existing Phase 15R and operational-payload regressions.

use std::fmt;

use super::{
    decode_native_encrypted_operational_device_payload_v1, decode_native_platform_bound_vault,
    encode_native_encrypted_operational_device_payload_v1, encode_native_platform_bound_vault,
    NativeEncryptedOperationalDevicePayloadV1, NativeEncryptedVaultV1,
    NativeOperationalDevicePayloadError, NativePlatformBoundVaultError, NativePlatformBoundVaultV1,
    NativePlatformStorageError, PHASE15I_MAX_ENCRYPTED_VAULT_BYTES,
    PHASE15R_PLATFORM_BOUND_VAULT_MAGIC,
};

pub const PHYSICAL_M1_PLATFORM_BOUND_VAULT_V2_LABEL: &str = "PHYSICAL_M1_PLATFORM_BOUND_VAULT_V2";

pub const PHYSICAL_M1_PLATFORM_BOUND_VAULT_V2_MAGIC: &[u8; 8] = PHASE15R_PLATFORM_BOUND_VAULT_MAGIC;

pub const PHYSICAL_M1_PLATFORM_BOUND_VAULT_V2_VERSION: u16 = 2;

const V2_HEADER_BYTES: usize =
    PHYSICAL_M1_PLATFORM_BOUND_VAULT_V2_MAGIC.len() + std::mem::size_of::<u16>();

const V2_LENGTH_FIELDS_BYTES: usize = std::mem::size_of::<u32>() + std::mem::size_of::<u16>();

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativePlatformBoundVaultV2Error {
    InvalidEncodedVault,
    EncodedVaultTooLarge { actual: usize, maximum: usize },
    UnsupportedVersion { actual: u16 },
    BaseVaultTooLarge { actual: usize, maximum: usize },
    OperationalPayloadTooLarge { actual: usize, maximum: usize },
    BaseVaultContractViolation(NativePlatformBoundVaultError),
    OperationalPayloadContractViolation(NativeOperationalDevicePayloadError),
    PlatformStorageContractViolation,
    MigrationVerificationFailed,
}

impl From<NativePlatformBoundVaultError> for NativePlatformBoundVaultV2Error {
    fn from(error: NativePlatformBoundVaultError) -> Self {
        Self::BaseVaultContractViolation(error)
    }
}

impl From<NativeOperationalDevicePayloadError> for NativePlatformBoundVaultV2Error {
    fn from(error: NativeOperationalDevicePayloadError) -> Self {
        Self::OperationalPayloadContractViolation(error)
    }
}

impl From<NativePlatformStorageError> for NativePlatformBoundVaultV2Error {
    fn from(_: NativePlatformStorageError) -> Self {
        Self::PlatformStorageContractViolation
    }
}

/// V2 keeps the complete validated V1 platform-bound vault as its base and
/// adds exactly one authenticated encrypted operational-device payload.
#[derive(Clone, PartialEq, Eq)]
pub struct NativePlatformBoundVaultV2 {
    base_v1: NativePlatformBoundVaultV1,
    operational_device_payload: NativeEncryptedOperationalDevicePayloadV1,
}

impl NativePlatformBoundVaultV2 {
    pub fn new(
        base_v1: NativePlatformBoundVaultV1,
        operational_device_payload: NativeEncryptedOperationalDevicePayloadV1,
    ) -> Result<Self, NativePlatformBoundVaultV2Error> {
        let base_encoded = encode_native_platform_bound_vault(&base_v1)?;

        let payload_encoded =
            encode_native_encrypted_operational_device_payload_v1(&operational_device_payload)?;

        validate_component_lengths(base_encoded.as_slice().len(), payload_encoded.len())?;

        Ok(Self {
            base_v1,
            operational_device_payload,
        })
    }

    pub fn base_v1(&self) -> &NativePlatformBoundVaultV1 {
        &self.base_v1
    }

    pub fn operational_device_payload(&self) -> &NativeEncryptedOperationalDevicePayloadV1 {
        &self.operational_device_payload
    }
}

impl fmt::Debug for NativePlatformBoundVaultV2 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NativePlatformBoundVaultV2")
            .field("version", &PHYSICAL_M1_PLATFORM_BOUND_VAULT_V2_VERSION)
            .field("base_v1", &self.base_v1)
            .field(
                "operational_device_payload",
                &self.operational_device_payload,
            )
            .finish()
    }
}

/// Explicit versioned decode result used by future migration/runtime code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativePlatformBoundVaultVersioned {
    V1(NativePlatformBoundVaultV1),
    V2(NativePlatformBoundVaultV2),
}

impl NativePlatformBoundVaultVersioned {
    /// Return the canonical V1 platform/factor/wrapped-key base for either
    /// persisted envelope version.
    ///
    /// V2 deliberately embeds V1 rather than redefining platform custody.
    /// Callers that need recovery/operational factor semantics should use
    /// this projection instead of duplicating version matching.
    pub fn base_v1(&self) -> &NativePlatformBoundVaultV1 {
        match self {
            Self::V1(v1) => v1,
            Self::V2(v2) => v2.base_v1(),
        }
    }
}

/// Encode V2 as:
///
/// magic[8] | version:u16 | base_v1_len:u32 | base_v1 |
/// operational_payload_len:u16 | operational_payload
///
/// The embedded V1 bytes are produced by the canonical Phase 15R encoder.
/// V2 does not duplicate platform-factor or wrapped-VMK serialization.
pub fn encode_native_platform_bound_vault_v2(
    vault: &NativePlatformBoundVaultV2,
) -> Result<NativeEncryptedVaultV1, NativePlatformBoundVaultV2Error> {
    let base_encoded = encode_native_platform_bound_vault(vault.base_v1())?;

    let payload_encoded =
        encode_native_encrypted_operational_device_payload_v1(vault.operational_device_payload())?;

    validate_component_lengths(base_encoded.as_slice().len(), payload_encoded.len())?;

    let base_len = u32::try_from(base_encoded.as_slice().len()).map_err(|_| {
        NativePlatformBoundVaultV2Error::BaseVaultTooLarge {
            actual: base_encoded.as_slice().len(),
            maximum: u32::MAX as usize,
        }
    })?;

    let payload_len = u16::try_from(payload_encoded.len()).map_err(|_| {
        NativePlatformBoundVaultV2Error::OperationalPayloadTooLarge {
            actual: payload_encoded.len(),
            maximum: u16::MAX as usize,
        }
    })?;

    let total_len = V2_HEADER_BYTES
        .checked_add(V2_LENGTH_FIELDS_BYTES)
        .and_then(|value| value.checked_add(base_encoded.as_slice().len()))
        .and_then(|value| value.checked_add(payload_encoded.len()))
        .ok_or(NativePlatformBoundVaultV2Error::InvalidEncodedVault)?;

    if total_len > PHASE15I_MAX_ENCRYPTED_VAULT_BYTES {
        return Err(NativePlatformBoundVaultV2Error::EncodedVaultTooLarge {
            actual: total_len,
            maximum: PHASE15I_MAX_ENCRYPTED_VAULT_BYTES,
        });
    }

    let mut encoded = Vec::with_capacity(total_len);

    encoded.extend_from_slice(PHYSICAL_M1_PLATFORM_BOUND_VAULT_V2_MAGIC);

    encoded.extend_from_slice(&PHYSICAL_M1_PLATFORM_BOUND_VAULT_V2_VERSION.to_be_bytes());

    encoded.extend_from_slice(&base_len.to_be_bytes());

    encoded.extend_from_slice(base_encoded.as_slice());

    encoded.extend_from_slice(&payload_len.to_be_bytes());

    encoded.extend_from_slice(&payload_encoded);

    NativeEncryptedVaultV1::new(encoded).map_err(Into::into)
}

/// Decode V2 while delegating all embedded V1 validation to the canonical
/// Phase 15R decoder and all operational payload validation to its codec.
pub fn decode_native_platform_bound_vault_v2(
    encoded: &NativeEncryptedVaultV1,
) -> Result<NativePlatformBoundVaultV2, NativePlatformBoundVaultV2Error> {
    let bytes = encoded.as_slice();

    if bytes.len() > PHASE15I_MAX_ENCRYPTED_VAULT_BYTES {
        return Err(NativePlatformBoundVaultV2Error::EncodedVaultTooLarge {
            actual: bytes.len(),
            maximum: PHASE15I_MAX_ENCRYPTED_VAULT_BYTES,
        });
    }

    let mut cursor = 0usize;

    let magic = read_array::<8>(bytes, &mut cursor)?;

    if &magic != PHYSICAL_M1_PLATFORM_BOUND_VAULT_V2_MAGIC {
        return Err(NativePlatformBoundVaultV2Error::InvalidEncodedVault);
    }

    let version = u16::from_be_bytes(read_array::<2>(bytes, &mut cursor)?);

    if version != PHYSICAL_M1_PLATFORM_BOUND_VAULT_V2_VERSION {
        return Err(NativePlatformBoundVaultV2Error::UnsupportedVersion { actual: version });
    }

    let base_len_u32 = u32::from_be_bytes(read_array::<4>(bytes, &mut cursor)?);

    let base_len = usize::try_from(base_len_u32)
        .map_err(|_| NativePlatformBoundVaultV2Error::InvalidEncodedVault)?;

    if base_len == 0 || base_len > PHASE15I_MAX_ENCRYPTED_VAULT_BYTES {
        return Err(NativePlatformBoundVaultV2Error::InvalidEncodedVault);
    }

    let base_end = cursor
        .checked_add(base_len)
        .ok_or(NativePlatformBoundVaultV2Error::InvalidEncodedVault)?;

    let base_bytes = bytes
        .get(cursor..base_end)
        .ok_or(NativePlatformBoundVaultV2Error::InvalidEncodedVault)?;

    cursor = base_end;

    let payload_len = usize::from(u16::from_be_bytes(read_array::<2>(bytes, &mut cursor)?));

    if payload_len == 0 {
        return Err(NativePlatformBoundVaultV2Error::InvalidEncodedVault);
    }

    let payload_end = cursor
        .checked_add(payload_len)
        .ok_or(NativePlatformBoundVaultV2Error::InvalidEncodedVault)?;

    if payload_end != bytes.len() {
        return Err(NativePlatformBoundVaultV2Error::InvalidEncodedVault);
    }

    let payload_bytes = bytes
        .get(cursor..payload_end)
        .ok_or(NativePlatformBoundVaultV2Error::InvalidEncodedVault)?;

    validate_component_lengths(base_bytes.len(), payload_bytes.len())?;

    let base_envelope = NativeEncryptedVaultV1::new(base_bytes.to_vec())?;

    let base_v1 = decode_native_platform_bound_vault(&base_envelope)?;

    let operational_device_payload =
        decode_native_encrypted_operational_device_payload_v1(payload_bytes)?;

    NativePlatformBoundVaultV2::new(base_v1, operational_device_payload)
}

/// Decode either the released V1 envelope or V2 without weakening either
/// format's own strict decoder.
pub fn decode_native_platform_bound_vault_versioned(
    encoded: &NativeEncryptedVaultV1,
) -> Result<NativePlatformBoundVaultVersioned, NativePlatformBoundVaultV2Error> {
    let bytes = encoded.as_slice();

    if bytes.len() < V2_HEADER_BYTES {
        return Err(NativePlatformBoundVaultV2Error::InvalidEncodedVault);
    }

    if bytes.get(..PHYSICAL_M1_PLATFORM_BOUND_VAULT_V2_MAGIC.len())
        != Some(PHYSICAL_M1_PLATFORM_BOUND_VAULT_V2_MAGIC.as_slice())
    {
        return Err(NativePlatformBoundVaultV2Error::InvalidEncodedVault);
    }

    let version_start = PHYSICAL_M1_PLATFORM_BOUND_VAULT_V2_MAGIC.len();

    let version_end = version_start + std::mem::size_of::<u16>();

    let version_bytes: [u8; 2] = bytes
        .get(version_start..version_end)
        .ok_or(NativePlatformBoundVaultV2Error::InvalidEncodedVault)?
        .try_into()
        .map_err(|_| NativePlatformBoundVaultV2Error::InvalidEncodedVault)?;

    let version = u16::from_be_bytes(version_bytes);

    match version {
        1 => Ok(NativePlatformBoundVaultVersioned::V1(
            decode_native_platform_bound_vault(encoded)?,
        )),
        PHYSICAL_M1_PLATFORM_BOUND_VAULT_V2_VERSION => Ok(NativePlatformBoundVaultVersioned::V2(
            decode_native_platform_bound_vault_v2(encoded)?,
        )),
        actual => Err(NativePlatformBoundVaultV2Error::UnsupportedVersion { actual }),
    }
}

/// Pure migration preparation.
///
/// This performs no storage mutation. It creates V2 from the existing
/// validated V1 object and already-encrypted operational payload, then
/// encodes and decodes the candidate before returning it. Future desktop
/// orchestration may atomically replace the persisted V1 only after this
/// verification succeeds.
pub fn prepare_native_platform_bound_vault_v1_to_v2_migration(
    v1: &NativePlatformBoundVaultV1,
    operational_device_payload: &NativeEncryptedOperationalDevicePayloadV1,
) -> Result<NativePlatformBoundVaultV2, NativePlatformBoundVaultV2Error> {
    let candidate =
        NativePlatformBoundVaultV2::new(v1.clone(), operational_device_payload.clone())?;

    let encoded = encode_native_platform_bound_vault_v2(&candidate)?;

    let verified = decode_native_platform_bound_vault_v2(&encoded)?;

    if verified != candidate {
        return Err(NativePlatformBoundVaultV2Error::MigrationVerificationFailed);
    }

    Ok(verified)
}

fn validate_component_lengths(
    base_len: usize,
    payload_len: usize,
) -> Result<(), NativePlatformBoundVaultV2Error> {
    if base_len == 0 || base_len > PHASE15I_MAX_ENCRYPTED_VAULT_BYTES {
        return Err(NativePlatformBoundVaultV2Error::BaseVaultTooLarge {
            actual: base_len,
            maximum: PHASE15I_MAX_ENCRYPTED_VAULT_BYTES,
        });
    }

    if payload_len == 0 || payload_len > u16::MAX as usize {
        return Err(
            NativePlatformBoundVaultV2Error::OperationalPayloadTooLarge {
                actual: payload_len,
                maximum: u16::MAX as usize,
            },
        );
    }

    Ok(())
}

fn read_array<const N: usize>(
    bytes: &[u8],
    cursor: &mut usize,
) -> Result<[u8; N], NativePlatformBoundVaultV2Error> {
    let end = cursor
        .checked_add(N)
        .ok_or(NativePlatformBoundVaultV2Error::InvalidEncodedVault)?;

    let value: [u8; N] = bytes
        .get(*cursor..end)
        .ok_or(NativePlatformBoundVaultV2Error::InvalidEncodedVault)?
        .try_into()
        .map_err(|_| NativePlatformBoundVaultV2Error::InvalidEncodedVault)?;

    *cursor = end;

    Ok(value)
}
