//! RO:WHAT — Generates one bounded Native Passport Ed25519 device signing seed through an injected native randomness port.
//! RO:WHY — Physical M1 needs a real random per-device key before root authorization, proof signing, and authenticated CrabNode requests can become live.
//! RO:INTERACTS — NativeSecretBytes, the canonical 32-byte device signing-seed size, and a platform-supplied cryptographic randomness source.
//! RO:INVARIANTS — exactly 32 random bytes; no deterministic production RNG; no key derivation from Passport root, recovery phrase, PIN, username, or wallet state.
//! RO:METRICS — none; secret generation is intentionally not labeled with identity material.
//! RO:CONFIG — randomness is supplied through NativeDeviceKeyRandomSource so portable svc-passport does not own an OS framework.
//! RO:SECURITY — temporary seed storage is zeroized; output remains NativeSecretBytes; no serialization, logging, persistence, platform-sealer mutation, vault mutation, signing, route, capability, wallet, or ledger authority.
//! RO:TEST — tests/physical_m1_native_device_key_generation.rs.

use zeroize::Zeroizing;

use super::{NativeSecretBytes, DEVICE_ID_V1_SIGNING_SEED_BYTES};

pub const PHYSICAL_M1_DEVICE_KEY_GENERATION_LABEL: &str =
    "PHYSICAL_M1_NATIVE_DEVICE_KEY_GENERATION_V1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeDeviceKeyGenerationError {
    RandomnessUnavailable,
    SecretConstructionFailed,
}

/// Supplies cryptographically secure bytes to portable Native Passport logic.
///
/// Production desktop implementations must bind this port to the operating
/// system CSPRNG. Tests may inject deterministic sources.
pub trait NativeDeviceKeyRandomSource: Send + Sync {
    fn fill(&self, output: &mut [u8]) -> Result<(), NativeDeviceKeyGenerationError>;
}

/// Generate one random Ed25519 device signing seed.
///
/// This function deliberately performs no persistence. The resulting secret
/// must eventually be stored inside the encrypted operational Passport
/// compartment rather than replacing the platform-bound operational factor.
pub fn generate_native_device_signing_seed_v1_with_random<R>(
    random: &R,
) -> Result<NativeSecretBytes, NativeDeviceKeyGenerationError>
where
    R: NativeDeviceKeyRandomSource + ?Sized,
{
    let mut seed = Zeroizing::new([0u8; DEVICE_ID_V1_SIGNING_SEED_BYTES]);

    random.fill(&mut seed[..])?;

    NativeSecretBytes::new(seed[..].to_vec())
        .map_err(|_| NativeDeviceKeyGenerationError::SecretConstructionFailed)
}
