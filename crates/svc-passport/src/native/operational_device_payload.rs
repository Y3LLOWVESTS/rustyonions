//! RO:WHAT — Defines and cryptographically protects the versioned Native Passport operational device payload containing the per-device Ed25519 signing seed plus its canonical public-key binding.
//! RO:WHY — Physical M1 needs durable device identity without placing the signing seed in the platform-factor slot; persisting the canonical public binding lets decode reject seed/identity mismatch before the first physical V2 migration.
//! RO:INTERACTS — NativeSecretBytes, canonical device identity derivation, Ed25519PublicKeyHex, the 32-byte operational VMK, HKDF-SHA256 key separation, and XChaCha20-Poly1305.
//! RO:INVARIANTS — device seed is exactly 32 bytes; persisted public binding must equal the canonical public key rederived from that seed; operational VMK is exactly 32 bytes; payload key is domain-separated from the VMK; nonce is 24 bytes; authenticated envelope is versioned and bounded.
//! RO:METRICS — none; secret or ciphertext material is never logged.
//! RO:CONFIG — native-passport feature only; randomness for the nonce remains the platform runtime's responsibility.
//! RO:SECURITY — plaintext and derived keys are zeroized; Debug is redacted; no filesystem, PlatformSealer, Tauri, WebView, root signing, capability issuance, username, wallet, or ledger authority.
//! RO:TEST — tests/physical_m1_native_operational_device_payload.rs.

use std::fmt;

use chacha20poly1305::{
    aead::{Aead, KeyInit, Payload},
    Key, XChaCha20Poly1305, XNonce,
};
use hkdf::Hkdf;
use sha2::Sha256;
use zeroize::Zeroizing;

use super::{
    derive_native_device_public_identity_v1, Ed25519PublicKeyHex, NativeSecretBytes,
    DEVICE_ID_V1_SIGNING_SEED_BYTES, PHASE15Q_VAULT_MASTER_KEY_BYTES, PHASE6A_AEAD_NONCE_LEN,
};

pub const PHYSICAL_M1_OPERATIONAL_DEVICE_PAYLOAD_LABEL: &str =
    "PHYSICAL_M1_NATIVE_OPERATIONAL_DEVICE_PAYLOAD_V1";

pub const PHYSICAL_M1_DEVICE_PAYLOAD_VERSION: u16 = 1;

pub const PHYSICAL_M1_DEVICE_PAYLOAD_PLAINTEXT_MAGIC: &[u8; 8] = b"RONODS01";

pub const PHYSICAL_M1_DEVICE_PAYLOAD_ENVELOPE_MAGIC: &[u8; 8] = b"RONODE01";

pub const PHYSICAL_M1_OPERATIONAL_VAULT_DOMAIN: &str =
    "rustyonions.native-passport.operational-vault.v1";

pub const PHYSICAL_M1_DEVICE_PAYLOAD_KEY_DOMAIN: &str = PHYSICAL_M1_OPERATIONAL_VAULT_DOMAIN;

pub const PHYSICAL_M1_DEVICE_PAYLOAD_AAD_DOMAIN: &str = PHYSICAL_M1_OPERATIONAL_VAULT_DOMAIN;

pub const PHYSICAL_M1_DEVICE_PAYLOAD_MAX_CIPHERTEXT_BYTES: usize = 256;

pub const PHYSICAL_M1_DEVICE_PAYLOAD_MAX_ENCODED_BYTES: usize = 512;

/// `Ed25519PublicKeyHex` is the canonical 32-byte public key rendered as
/// exactly 64 lowercase hexadecimal ASCII bytes.
const DEVICE_PUBLIC_KEY_HEX_BYTES: usize = 64;

const DEVICE_PAYLOAD_PLAINTEXT_BYTES: usize = PHYSICAL_M1_DEVICE_PAYLOAD_PLAINTEXT_MAGIC.len()
    + std::mem::size_of::<u16>()
    + DEVICE_ID_V1_SIGNING_SEED_BYTES
    + DEVICE_PUBLIC_KEY_HEX_BYTES;

const DEVICE_PAYLOAD_ENVELOPE_FIXED_BYTES: usize = PHYSICAL_M1_DEVICE_PAYLOAD_ENVELOPE_MAGIC.len()
    + std::mem::size_of::<u16>()
    + PHASE6A_AEAD_NONCE_LEN
    + std::mem::size_of::<u16>();

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeOperationalDevicePayloadError {
    InvalidOperationalVmkLength { actual: usize, expected: usize },
    InvalidDeviceSigningSeedLength { actual: usize, expected: usize },
    DeviceIdentityDerivationFailed,
    DevicePublicBindingMismatch,
    InvalidNonceLength { actual: usize, expected: usize },
    KeyDerivationFailed,
    EncryptionFailed,
    AuthenticationFailed,
    InvalidEncodedPayload,
    UnsupportedVersion { actual: u16 },
    CiphertextTooLarge { actual: usize, maximum: usize },
    EncodedPayloadTooLarge { actual: usize, maximum: usize },
    SecretConstructionFailed,
}

/// Plaintext operational device material.
///
/// This object is native-only and deliberately does not implement Clone.
#[derive(PartialEq, Eq)]
pub struct NativeOperationalDevicePayloadV1 {
    device_signing_seed: NativeSecretBytes,
    device_public_key: Ed25519PublicKeyHex,
}

impl NativeOperationalDevicePayloadV1 {
    pub fn new(
        device_signing_seed: NativeSecretBytes,
    ) -> Result<Self, NativeOperationalDevicePayloadError> {
        validate_device_signing_seed(&device_signing_seed)?;

        let identity = derive_native_device_public_identity_v1(&device_signing_seed)
            .map_err(|_| NativeOperationalDevicePayloadError::DeviceIdentityDerivationFailed)?;

        Ok(Self {
            device_signing_seed,
            device_public_key: identity.device_public_key,
        })
    }

    pub fn device_signing_seed(&self) -> &NativeSecretBytes {
        &self.device_signing_seed
    }

    pub fn device_public_key(&self) -> &Ed25519PublicKeyHex {
        &self.device_public_key
    }
}

impl fmt::Debug for NativeOperationalDevicePayloadV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NativeOperationalDevicePayloadV1")
            .field("version", &PHYSICAL_M1_DEVICE_PAYLOAD_VERSION)
            .field(
                "device_signing_seed",
                &"REDACTED_NATIVE_DEVICE_SIGNING_SEED",
            )
            .field(
                "device_signing_seed_length",
                &self.device_signing_seed.len(),
            )
            .field("device_public_key", &self.device_public_key.as_str())
            .finish()
    }
}

/// Persistable authenticated ciphertext for the operational device payload.
#[derive(Clone, PartialEq, Eq)]
pub struct NativeEncryptedOperationalDevicePayloadV1 {
    nonce: [u8; PHASE6A_AEAD_NONCE_LEN],
    ciphertext: Vec<u8>,
}

impl NativeEncryptedOperationalDevicePayloadV1 {
    pub fn new(
        nonce: [u8; PHASE6A_AEAD_NONCE_LEN],
        ciphertext: Vec<u8>,
    ) -> Result<Self, NativeOperationalDevicePayloadError> {
        validate_ciphertext(&ciphertext)?;

        Ok(Self { nonce, ciphertext })
    }

    pub fn nonce(&self) -> &[u8; PHASE6A_AEAD_NONCE_LEN] {
        &self.nonce
    }

    pub fn ciphertext(&self) -> &[u8] {
        &self.ciphertext
    }

    pub fn ciphertext_len(&self) -> usize {
        self.ciphertext.len()
    }
}

impl fmt::Debug for NativeEncryptedOperationalDevicePayloadV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NativeEncryptedOperationalDevicePayloadV1")
            .field("version", &PHYSICAL_M1_DEVICE_PAYLOAD_VERSION)
            .field("nonce", &"REDACTED_NONCE")
            .field("ciphertext", &"REDACTED_AUTHENTICATED_CIPHERTEXT")
            .field("ciphertext_length", &self.ciphertext.len())
            .finish()
    }
}

/// Encrypt one Native Passport device signing seed under a key derived from
/// the already-unlocked operational VMK.
///
/// The caller owns secure random generation of the 24-byte XChaCha nonce.
pub fn encrypt_native_operational_device_payload_v1(
    operational_vmk: &NativeSecretBytes,
    nonce: &[u8; PHASE6A_AEAD_NONCE_LEN],
    payload: &NativeOperationalDevicePayloadV1,
) -> Result<NativeEncryptedOperationalDevicePayloadV1, NativeOperationalDevicePayloadError> {
    validate_operational_vmk(operational_vmk)?;
    validate_device_signing_seed(payload.device_signing_seed())?;
    validate_device_public_binding(payload)?;

    let derived_key = derive_payload_key(operational_vmk)?;

    let plaintext = encode_plaintext(payload)?;

    let key = Key::from(*derived_key);
    let cipher = XChaCha20Poly1305::new(&key);
    let xnonce = XNonce::from(*nonce);

    let ciphertext = cipher
        .encrypt(
            &xnonce,
            Payload {
                msg: plaintext.as_slice(),
                aad: PHYSICAL_M1_DEVICE_PAYLOAD_AAD_DOMAIN.as_bytes(),
            },
        )
        .map_err(|_| NativeOperationalDevicePayloadError::EncryptionFailed)?;

    NativeEncryptedOperationalDevicePayloadV1::new(*nonce, ciphertext)
}

/// Authenticate and decrypt one Native Passport operational device payload.
pub fn decrypt_native_operational_device_payload_v1(
    operational_vmk: &NativeSecretBytes,
    encrypted: &NativeEncryptedOperationalDevicePayloadV1,
) -> Result<NativeOperationalDevicePayloadV1, NativeOperationalDevicePayloadError> {
    validate_operational_vmk(operational_vmk)?;
    validate_ciphertext(encrypted.ciphertext())?;

    let derived_key = derive_payload_key(operational_vmk)?;

    let key = Key::from(*derived_key);
    let cipher = XChaCha20Poly1305::new(&key);
    let xnonce = XNonce::from(*encrypted.nonce());

    let plaintext = cipher
        .decrypt(
            &xnonce,
            Payload {
                msg: encrypted.ciphertext(),
                aad: PHYSICAL_M1_DEVICE_PAYLOAD_AAD_DOMAIN.as_bytes(),
            },
        )
        .map_err(|_| NativeOperationalDevicePayloadError::AuthenticationFailed)?;

    let plaintext = Zeroizing::new(plaintext);

    decode_plaintext(plaintext.as_slice())
}

/// Encode the authenticated ciphertext into a strict bounded persistence blob.
///
/// Wire shape:
/// magic[8] | version:u16 | nonce[24] | ciphertext_len:u16 | ciphertext
pub fn encode_native_encrypted_operational_device_payload_v1(
    encrypted: &NativeEncryptedOperationalDevicePayloadV1,
) -> Result<Vec<u8>, NativeOperationalDevicePayloadError> {
    validate_ciphertext(encrypted.ciphertext())?;

    let ciphertext_length = u16::try_from(encrypted.ciphertext_len()).map_err(|_| {
        NativeOperationalDevicePayloadError::CiphertextTooLarge {
            actual: encrypted.ciphertext_len(),
            maximum: PHYSICAL_M1_DEVICE_PAYLOAD_MAX_CIPHERTEXT_BYTES,
        }
    })?;

    let encoded_length = DEVICE_PAYLOAD_ENVELOPE_FIXED_BYTES
        .checked_add(encrypted.ciphertext_len())
        .ok_or(NativeOperationalDevicePayloadError::InvalidEncodedPayload)?;

    if encoded_length > PHYSICAL_M1_DEVICE_PAYLOAD_MAX_ENCODED_BYTES {
        return Err(
            NativeOperationalDevicePayloadError::EncodedPayloadTooLarge {
                actual: encoded_length,
                maximum: PHYSICAL_M1_DEVICE_PAYLOAD_MAX_ENCODED_BYTES,
            },
        );
    }

    let mut encoded = Vec::with_capacity(encoded_length);

    encoded.extend_from_slice(PHYSICAL_M1_DEVICE_PAYLOAD_ENVELOPE_MAGIC);

    encoded.extend_from_slice(&PHYSICAL_M1_DEVICE_PAYLOAD_VERSION.to_be_bytes());

    encoded.extend_from_slice(encrypted.nonce());

    encoded.extend_from_slice(&ciphertext_length.to_be_bytes());

    encoded.extend_from_slice(encrypted.ciphertext());

    Ok(encoded)
}

/// Decode a strict bounded operational-device ciphertext envelope.
pub fn decode_native_encrypted_operational_device_payload_v1(
    encoded: &[u8],
) -> Result<NativeEncryptedOperationalDevicePayloadV1, NativeOperationalDevicePayloadError> {
    if encoded.len() > PHYSICAL_M1_DEVICE_PAYLOAD_MAX_ENCODED_BYTES {
        return Err(
            NativeOperationalDevicePayloadError::EncodedPayloadTooLarge {
                actual: encoded.len(),
                maximum: PHYSICAL_M1_DEVICE_PAYLOAD_MAX_ENCODED_BYTES,
            },
        );
    }

    if encoded.len() < DEVICE_PAYLOAD_ENVELOPE_FIXED_BYTES {
        return Err(NativeOperationalDevicePayloadError::InvalidEncodedPayload);
    }

    let mut cursor = 0usize;

    let magic_end = cursor
        .checked_add(PHYSICAL_M1_DEVICE_PAYLOAD_ENVELOPE_MAGIC.len())
        .ok_or(NativeOperationalDevicePayloadError::InvalidEncodedPayload)?;

    if encoded.get(cursor..magic_end) != Some(PHYSICAL_M1_DEVICE_PAYLOAD_ENVELOPE_MAGIC.as_slice())
    {
        return Err(NativeOperationalDevicePayloadError::InvalidEncodedPayload);
    }

    cursor = magic_end;

    let version_end = cursor
        .checked_add(std::mem::size_of::<u16>())
        .ok_or(NativeOperationalDevicePayloadError::InvalidEncodedPayload)?;

    let version_bytes: [u8; 2] = encoded
        .get(cursor..version_end)
        .ok_or(NativeOperationalDevicePayloadError::InvalidEncodedPayload)?
        .try_into()
        .map_err(|_| NativeOperationalDevicePayloadError::InvalidEncodedPayload)?;

    let version = u16::from_be_bytes(version_bytes);

    if version != PHYSICAL_M1_DEVICE_PAYLOAD_VERSION {
        return Err(NativeOperationalDevicePayloadError::UnsupportedVersion { actual: version });
    }

    cursor = version_end;

    let nonce_end = cursor
        .checked_add(PHASE6A_AEAD_NONCE_LEN)
        .ok_or(NativeOperationalDevicePayloadError::InvalidEncodedPayload)?;

    let nonce: [u8; PHASE6A_AEAD_NONCE_LEN] = encoded
        .get(cursor..nonce_end)
        .ok_or(NativeOperationalDevicePayloadError::InvalidEncodedPayload)?
        .try_into()
        .map_err(|_| NativeOperationalDevicePayloadError::InvalidEncodedPayload)?;

    cursor = nonce_end;

    let length_end = cursor
        .checked_add(std::mem::size_of::<u16>())
        .ok_or(NativeOperationalDevicePayloadError::InvalidEncodedPayload)?;

    let ciphertext_length_bytes: [u8; 2] = encoded
        .get(cursor..length_end)
        .ok_or(NativeOperationalDevicePayloadError::InvalidEncodedPayload)?
        .try_into()
        .map_err(|_| NativeOperationalDevicePayloadError::InvalidEncodedPayload)?;

    let ciphertext_length = usize::from(u16::from_be_bytes(ciphertext_length_bytes));

    cursor = length_end;

    if ciphertext_length == 0 || ciphertext_length > PHYSICAL_M1_DEVICE_PAYLOAD_MAX_CIPHERTEXT_BYTES
    {
        return Err(NativeOperationalDevicePayloadError::InvalidEncodedPayload);
    }

    let ciphertext_end = cursor
        .checked_add(ciphertext_length)
        .ok_or(NativeOperationalDevicePayloadError::InvalidEncodedPayload)?;

    if ciphertext_end != encoded.len() {
        return Err(NativeOperationalDevicePayloadError::InvalidEncodedPayload);
    }

    let ciphertext = encoded
        .get(cursor..ciphertext_end)
        .ok_or(NativeOperationalDevicePayloadError::InvalidEncodedPayload)?
        .to_vec();

    NativeEncryptedOperationalDevicePayloadV1::new(nonce, ciphertext)
}

fn derive_payload_key(
    operational_vmk: &NativeSecretBytes,
) -> Result<Zeroizing<[u8; PHASE15Q_VAULT_MASTER_KEY_BYTES]>, NativeOperationalDevicePayloadError> {
    validate_operational_vmk(operational_vmk)?;

    let hkdf = Hkdf::<Sha256>::new(None, operational_vmk.as_slice());

    let mut derived_key = Zeroizing::new([0u8; PHASE15Q_VAULT_MASTER_KEY_BYTES]);

    hkdf.expand(
        PHYSICAL_M1_DEVICE_PAYLOAD_KEY_DOMAIN.as_bytes(),
        &mut derived_key[..],
    )
    .map_err(|_| NativeOperationalDevicePayloadError::KeyDerivationFailed)?;

    Ok(derived_key)
}

fn encode_plaintext(
    payload: &NativeOperationalDevicePayloadV1,
) -> Result<Zeroizing<Vec<u8>>, NativeOperationalDevicePayloadError> {
    validate_device_signing_seed(payload.device_signing_seed())?;
    validate_device_public_binding(payload)?;

    let mut plaintext = Zeroizing::new(Vec::with_capacity(DEVICE_PAYLOAD_PLAINTEXT_BYTES));

    plaintext.extend_from_slice(PHYSICAL_M1_DEVICE_PAYLOAD_PLAINTEXT_MAGIC);

    plaintext.extend_from_slice(&PHYSICAL_M1_DEVICE_PAYLOAD_VERSION.to_be_bytes());

    plaintext.extend_from_slice(payload.device_signing_seed().as_slice());

    plaintext.extend_from_slice(payload.device_public_key().as_str().as_bytes());

    if plaintext.len() != DEVICE_PAYLOAD_PLAINTEXT_BYTES {
        return Err(NativeOperationalDevicePayloadError::InvalidEncodedPayload);
    }

    Ok(plaintext)
}

fn decode_plaintext(
    plaintext: &[u8],
) -> Result<NativeOperationalDevicePayloadV1, NativeOperationalDevicePayloadError> {
    if plaintext.len() != DEVICE_PAYLOAD_PLAINTEXT_BYTES {
        return Err(NativeOperationalDevicePayloadError::InvalidEncodedPayload);
    }

    let magic_end = PHYSICAL_M1_DEVICE_PAYLOAD_PLAINTEXT_MAGIC.len();

    if plaintext.get(..magic_end) != Some(PHYSICAL_M1_DEVICE_PAYLOAD_PLAINTEXT_MAGIC.as_slice()) {
        return Err(NativeOperationalDevicePayloadError::InvalidEncodedPayload);
    }

    let version_end = magic_end
        .checked_add(std::mem::size_of::<u16>())
        .ok_or(NativeOperationalDevicePayloadError::InvalidEncodedPayload)?;

    let version_bytes: [u8; 2] = plaintext
        .get(magic_end..version_end)
        .ok_or(NativeOperationalDevicePayloadError::InvalidEncodedPayload)?
        .try_into()
        .map_err(|_| NativeOperationalDevicePayloadError::InvalidEncodedPayload)?;

    let version = u16::from_be_bytes(version_bytes);

    if version != PHYSICAL_M1_DEVICE_PAYLOAD_VERSION {
        return Err(NativeOperationalDevicePayloadError::UnsupportedVersion { actual: version });
    }

    let seed_end = version_end
        .checked_add(DEVICE_ID_V1_SIGNING_SEED_BYTES)
        .ok_or(NativeOperationalDevicePayloadError::InvalidEncodedPayload)?;

    let public_key_end = seed_end
        .checked_add(DEVICE_PUBLIC_KEY_HEX_BYTES)
        .ok_or(NativeOperationalDevicePayloadError::InvalidEncodedPayload)?;

    if public_key_end != plaintext.len() {
        return Err(NativeOperationalDevicePayloadError::InvalidEncodedPayload);
    }

    let seed_bytes = plaintext
        .get(version_end..seed_end)
        .ok_or(NativeOperationalDevicePayloadError::InvalidEncodedPayload)?
        .to_vec();

    let persisted_public_key = plaintext
        .get(seed_end..public_key_end)
        .ok_or(NativeOperationalDevicePayloadError::InvalidEncodedPayload)?;

    let device_signing_seed = NativeSecretBytes::new(seed_bytes)
        .map_err(|_| NativeOperationalDevicePayloadError::SecretConstructionFailed)?;

    let payload = NativeOperationalDevicePayloadV1::new(device_signing_seed)?;

    if payload.device_public_key().as_str().as_bytes() != persisted_public_key {
        return Err(NativeOperationalDevicePayloadError::DevicePublicBindingMismatch);
    }

    Ok(payload)
}

fn validate_device_public_binding(
    payload: &NativeOperationalDevicePayloadV1,
) -> Result<(), NativeOperationalDevicePayloadError> {
    let identity = derive_native_device_public_identity_v1(payload.device_signing_seed())
        .map_err(|_| NativeOperationalDevicePayloadError::DeviceIdentityDerivationFailed)?;

    if identity.device_public_key != *payload.device_public_key() {
        return Err(NativeOperationalDevicePayloadError::DevicePublicBindingMismatch);
    }

    Ok(())
}

fn validate_operational_vmk(
    operational_vmk: &NativeSecretBytes,
) -> Result<(), NativeOperationalDevicePayloadError> {
    if operational_vmk.len() != PHASE15Q_VAULT_MASTER_KEY_BYTES {
        return Err(
            NativeOperationalDevicePayloadError::InvalidOperationalVmkLength {
                actual: operational_vmk.len(),
                expected: PHASE15Q_VAULT_MASTER_KEY_BYTES,
            },
        );
    }

    Ok(())
}

fn validate_device_signing_seed(
    device_signing_seed: &NativeSecretBytes,
) -> Result<(), NativeOperationalDevicePayloadError> {
    if device_signing_seed.len() != DEVICE_ID_V1_SIGNING_SEED_BYTES {
        return Err(
            NativeOperationalDevicePayloadError::InvalidDeviceSigningSeedLength {
                actual: device_signing_seed.len(),
                expected: DEVICE_ID_V1_SIGNING_SEED_BYTES,
            },
        );
    }

    Ok(())
}

fn validate_ciphertext(ciphertext: &[u8]) -> Result<(), NativeOperationalDevicePayloadError> {
    if ciphertext.is_empty() {
        return Err(NativeOperationalDevicePayloadError::InvalidEncodedPayload);
    }

    if ciphertext.len() > PHYSICAL_M1_DEVICE_PAYLOAD_MAX_CIPHERTEXT_BYTES {
        return Err(NativeOperationalDevicePayloadError::CiphertextTooLarge {
            actual: ciphertext.len(),
            maximum: PHYSICAL_M1_DEVICE_PAYLOAD_MAX_CIPHERTEXT_BYTES,
        });
    }

    Ok(())
}

#[allow(dead_code)]
fn validate_nonce_length(nonce: &[u8]) -> Result<(), NativeOperationalDevicePayloadError> {
    if nonce.len() != PHASE6A_AEAD_NONCE_LEN {
        return Err(NativeOperationalDevicePayloadError::InvalidNonceLength {
            actual: nonce.len(),
            expected: PHASE6A_AEAD_NONCE_LEN,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physical_m1_plaintext_rejects_seed_public_binding_mismatch() {
        let seed =
            NativeSecretBytes::new((0u8..32u8).collect()).expect("deterministic device seed");

        let payload = NativeOperationalDevicePayloadV1::new(seed).expect("canonical payload");

        let mut plaintext = encode_plaintext(&payload).expect("encode canonical plaintext");

        let binding_offset = PHYSICAL_M1_DEVICE_PAYLOAD_PLAINTEXT_MAGIC.len()
            + std::mem::size_of::<u16>()
            + DEVICE_ID_V1_SIGNING_SEED_BYTES;

        let first = plaintext
            .get_mut(binding_offset)
            .expect("public binding byte");

        *first = if *first == b'0' { b'1' } else { b'0' };

        assert_eq!(
            decode_plaintext(plaintext.as_slice()),
            Err(NativeOperationalDevicePayloadError::DevicePublicBindingMismatch,),
        );
    }
}
