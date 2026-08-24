//! RO:WHAT — Derives canonical public Native Passport V1 device identity from a bounded Ed25519 signing seed or validated public key.
//! RO:WHY — Physical M1 needs a real per-device identity before root authorization and server challenge/proof can become live.
//! RO:INTERACTS — Phase 0F Device-ID constants, ed25519-dalek, NativeSecretBytes, DeviceIdV1, and Ed25519PublicKeyHex.
//! RO:INVARIANTS — signing seed is exactly 32 bytes; Device ID transcript is domain|ed25519|device_public_key_hex; root, username, PIN, wallet, and ledger material do not participate.
//! RO:METRICS — none; pure local derivation.
//! RO:CONFIG — compiled only through the native-passport feature.
//! RO:SECURITY — returns public identity only; no RNG, signing, serialization, logging, persistence, vault I/O, routes, capabilities, username mutation, wallet mutation, or ledger mutation.
//! RO:TEST — tests/physical_m1_native_device_identity_derivation.rs.

use ed25519_dalek::SigningKey;
use zeroize::Zeroizing;

use crate::native_plan::{DEVICE_ID_V1_ED25519_B3_PREFIX, DEVICE_ID_V1_HASH_DOMAIN};

use super::{DeviceIdV1, Ed25519PublicKeyHex, NativePassportDtoError, NativeSecretBytes};

pub const PHYSICAL_M1_DEVICE_IDENTITY_DERIVATION_LABEL: &str =
    "PHYSICAL_M1_NATIVE_DEVICE_IDENTITY_DERIVATION_V1";

/// Ed25519 device signing seeds are 256 bits.
pub const DEVICE_ID_V1_SIGNING_SEED_BYTES: usize = 32;

/// Public-only identity derived from one Native Passport device key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeDevicePublicIdentityV1 {
    pub device_id: DeviceIdV1,
    pub device_public_key: Ed25519PublicKeyHex,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeDeviceIdentityDerivationError {
    InvalidDeviceSigningSeedLength { actual: usize, expected: usize },
    InvalidDevicePublicKey,
    InvalidDeviceId,
}

/// Derive the canonical DeviceIdV1 from a validated Ed25519 public key.
///
/// Frozen Phase 0F hash input:
/// `domain|ed25519|device_public_key_hex`.
pub fn derive_native_device_id_v1(
    device_public_key: &Ed25519PublicKeyHex,
) -> Result<DeviceIdV1, NativePassportDtoError> {
    let hash_input = format!(
        "{}|ed25519|{}",
        DEVICE_ID_V1_HASH_DOMAIN,
        device_public_key.as_str(),
    );

    let digest = blake3::hash(hash_input.as_bytes()).to_hex().to_string();

    DeviceIdV1::parse(format!("{}{}", DEVICE_ID_V1_ED25519_B3_PREFIX, digest,))
}

/// Derive public device identity from an already-created native Ed25519 seed.
///
/// This function deliberately does not generate or persist the seed. The
/// device-key creation/runtime layer remains responsible for OS-CSPRNG
/// generation and durable native custody.
pub fn derive_native_device_public_identity_v1(
    device_signing_seed: &NativeSecretBytes,
) -> Result<NativeDevicePublicIdentityV1, NativeDeviceIdentityDerivationError> {
    if device_signing_seed.len() != DEVICE_ID_V1_SIGNING_SEED_BYTES {
        return Err(
            NativeDeviceIdentityDerivationError::InvalidDeviceSigningSeedLength {
                actual: device_signing_seed.len(),
                expected: DEVICE_ID_V1_SIGNING_SEED_BYTES,
            },
        );
    }

    let mut seed = Zeroizing::new([0u8; DEVICE_ID_V1_SIGNING_SEED_BYTES]);
    seed.copy_from_slice(device_signing_seed.as_slice());

    let signing_key = SigningKey::from_bytes(&seed);
    let verifying_key = signing_key.verifying_key();

    let device_public_key = Ed25519PublicKeyHex::parse(lower_hex(&verifying_key.to_bytes()))
        .map_err(|_| NativeDeviceIdentityDerivationError::InvalidDevicePublicKey)?;

    let device_id = derive_native_device_id_v1(&device_public_key)
        .map_err(|_| NativeDeviceIdentityDerivationError::InvalidDeviceId)?;

    Ok(NativeDevicePublicIdentityV1 {
        device_id,
        device_public_key,
    })
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
