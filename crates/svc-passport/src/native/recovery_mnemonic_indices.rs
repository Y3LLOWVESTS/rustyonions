//! RO:WHAT — Converts the existing 256-bit Native Passport recovery factor into canonical BIP-39 word indices and checksum.
//! RO:WHY — Onboarding Phase 6 needs deterministic recovery material before the platform-native word display is implemented.
//! RO:INTERACTS — Phase 15Q recovery factor, NativeSecretBytes, and the later native wordlist/display surface.
//! RO:INVARIANTS — exactly 32 bytes of entropy; 8 checksum bits; 24 11-bit indices; redacted Debug; secret indices zeroized on drop.
//! RO:SECURITY — no wordlist, serialization, filesystem, logging, clipboard, WebView return, root export, wallet mutation, or ledger mutation.
//! RO:TEST — module tests and native_passport_onboarding_phase6b1_recovery_mnemonic_indices.rs.

use std::fmt;

use sha2::{Digest, Sha256};
use zeroize::Zeroize;

use super::{NativeSecretBytes, PHASE15Q_PLATFORM_FACTOR_BYTES};

pub const ONBOARDING_PHASE6B1_NATIVE_LABEL: &str = "ONBOARDING_PHASE6B1_RECOVERY_MNEMONIC_INDICES";

pub const PHASE6B1_RECOVERY_ENTROPY_BYTES: usize = 32;

pub const PHASE6B1_RECOVERY_CHECKSUM_BITS: usize = 8;

pub const PHASE6B1_RECOVERY_WORD_COUNT: usize = 24;

pub const PHASE6B1_RECOVERY_WORD_INDEX_BITS: usize = 11;

pub const PHASE6B1_RECOVERY_WORDLIST_LENGTH: usize = 2048;

pub const PHASE6B1_RECOVERY_FINGERPRINT_HEX_LENGTH: usize = 16;

pub const PHASE6B1_RECOVERY_FINGERPRINT_DOMAIN: &str =
    "rustyonions.native-passport.recovery-mnemonic-fingerprint.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeRecoveryMnemonicIndicesError {
    RecoveryFactorLengthMismatch { actual: usize, expected: usize },
}

#[derive(PartialEq, Eq)]
pub struct NativeRecoveryMnemonicIndicesV1 {
    word_indices: [u16; PHASE6B1_RECOVERY_WORD_COUNT],
    fingerprint: [u8; 8],
}

impl NativeRecoveryMnemonicIndicesV1 {
    pub fn word_count(&self) -> usize {
        PHASE6B1_RECOVERY_WORD_COUNT
    }

    pub fn redacted_fingerprint_hex(&self) -> String {
        hex_lower(&self.fingerprint)
    }

    /// Temporarily lends the secret indices to a native-only caller without
    /// serializing or allocating another copy.
    pub fn with_native_word_indices<T>(
        &self,
        visitor: impl FnOnce(&[u16; PHASE6B1_RECOVERY_WORD_COUNT]) -> T,
    ) -> T {
        visitor(&self.word_indices)
    }
}

impl fmt::Debug for NativeRecoveryMnemonicIndicesV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NativeRecoveryMnemonicIndicesV1")
            .field("word_count", &PHASE6B1_RECOVERY_WORD_COUNT)
            .field("word_indices", &"REDACTED")
            .field("fingerprint", &"REDACTED")
            .finish()
    }
}

impl Drop for NativeRecoveryMnemonicIndicesV1 {
    fn drop(&mut self) {
        self.word_indices.zeroize();
        self.fingerprint.zeroize();
    }
}

pub fn derive_native_recovery_mnemonic_indices(
    recovery_factor: &NativeSecretBytes,
) -> Result<NativeRecoveryMnemonicIndicesV1, NativeRecoveryMnemonicIndicesError> {
    if recovery_factor.len() != PHASE6B1_RECOVERY_ENTROPY_BYTES {
        return Err(
            NativeRecoveryMnemonicIndicesError::RecoveryFactorLengthMismatch {
                actual: recovery_factor.len(),
                expected: PHASE6B1_RECOVERY_ENTROPY_BYTES,
            },
        );
    }

    debug_assert_eq!(
        PHASE15Q_PLATFORM_FACTOR_BYTES,
        PHASE6B1_RECOVERY_ENTROPY_BYTES,
    );

    let entropy = recovery_factor.as_slice();

    let checksum = Sha256::digest(entropy)[0];

    let mut word_indices = [0u16; PHASE6B1_RECOVERY_WORD_COUNT];

    for (word_position, word_index) in word_indices.iter_mut().enumerate() {
        let mut value = 0u16;

        for bit_offset in 0..PHASE6B1_RECOVERY_WORD_INDEX_BITS {
            let absolute_bit = word_position * PHASE6B1_RECOVERY_WORD_INDEX_BITS + bit_offset;

            let bit = if absolute_bit < PHASE6B1_RECOVERY_ENTROPY_BYTES * 8 {
                bit_from_slice(entropy, absolute_bit)
            } else {
                bit_from_byte(checksum, absolute_bit - PHASE6B1_RECOVERY_ENTROPY_BYTES * 8)
            };

            value = (value << 1) | u16::from(bit);
        }

        debug_assert!(usize::from(value) < PHASE6B1_RECOVERY_WORDLIST_LENGTH);

        *word_index = value;
    }

    Ok(NativeRecoveryMnemonicIndicesV1 {
        word_indices,
        fingerprint: recovery_fingerprint(entropy),
    })
}

fn bit_from_slice(bytes: &[u8], bit_index: usize) -> u8 {
    let byte = bytes[bit_index / 8];

    let shift = 7 - (bit_index % 8);

    (byte >> shift) & 1
}

fn bit_from_byte(byte: u8, bit_index: usize) -> u8 {
    let shift = 7 - bit_index;

    (byte >> shift) & 1
}

fn recovery_fingerprint(entropy: &[u8]) -> [u8; 8] {
    let mut hasher = blake3::Hasher::new();

    hasher.update(PHASE6B1_RECOVERY_FINGERPRINT_DOMAIN.as_bytes());

    hasher.update(&[0]);
    hasher.update(entropy);

    let digest = hasher.finalize();

    let mut fingerprint = [0u8; 8];

    fingerprint.copy_from_slice(&digest.as_bytes()[..8]);

    fingerprint
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let mut output = String::with_capacity(bytes.len() * 2);

    for byte in bytes.iter().copied() {
        output.push(char::from(HEX[usize::from(byte >> 4)]));

        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn factor(bytes: Vec<u8>) -> NativeSecretBytes {
        NativeSecretBytes::new(bytes).expect("bounded recovery-factor fixture")
    }

    fn indices(recovery: &NativeRecoveryMnemonicIndicesV1) -> [u16; PHASE6B1_RECOVERY_WORD_COUNT] {
        recovery.with_native_word_indices(|value| *value)
    }

    #[test]
    fn phase6b1_zero_entropy_matches_the_locked_bip39_index_vector() {
        let recovery = derive_native_recovery_mnemonic_indices(&factor(vec![0u8; 32]))
            .expect("zero entropy should encode");

        let mut expected = [0u16; PHASE6B1_RECOVERY_WORD_COUNT];

        expected[23] = 102;

        assert_eq!(indices(&recovery), expected,);

        assert_eq!(recovery.word_count(), 24,);

        assert_eq!(
            recovery.redacted_fingerprint_hex().len(),
            PHASE6B1_RECOVERY_FINGERPRINT_HEX_LENGTH,
        );
    }

    #[test]
    fn phase6b1_max_entropy_matches_the_locked_bip39_index_vector() {
        let recovery = derive_native_recovery_mnemonic_indices(&factor(vec![0xffu8; 32]))
            .expect("maximum entropy should encode");

        let mut expected = [2047u16; PHASE6B1_RECOVERY_WORD_COUNT];

        expected[23] = 1967;

        assert_eq!(indices(&recovery), expected,);
    }

    #[test]
    fn phase6b1_rejects_noncanonical_recovery_factor_lengths() {
        let error = derive_native_recovery_mnemonic_indices(&factor(vec![7u8; 31]))
            .expect_err("31-byte factor must fail closed");

        assert_eq!(
            error,
            NativeRecoveryMnemonicIndicesError::RecoveryFactorLengthMismatch {
                actual: 31,
                expected: 32,
            },
        );
    }

    #[test]
    fn phase6b1_debug_output_is_redacted() {
        let recovery = derive_native_recovery_mnemonic_indices(&factor(vec![0u8; 32]))
            .expect("fixture should encode");

        let debug = format!("{recovery:?}");

        assert!(debug.contains("REDACTED"));

        assert!(!debug.contains("102"));

        assert!(!debug.contains(&recovery.redacted_fingerprint_hex(),));
    }

    #[test]
    fn phase6b1_indices_round_trip_entropy_and_checksum() {
        let entropy: Vec<u8> = (0u8..32u8).collect();

        let recovery = derive_native_recovery_mnemonic_indices(&factor(entropy.clone()))
            .expect("fixture should encode");

        let (decoded_entropy, decoded_checksum) = decode_indices_for_test(&indices(&recovery));

        assert_eq!(decoded_entropy.as_slice(), entropy.as_slice(),);

        assert_eq!(decoded_checksum, Sha256::digest(&entropy)[0],);
    }

    fn decode_indices_for_test(indices: &[u16; PHASE6B1_RECOVERY_WORD_COUNT]) -> ([u8; 32], u8) {
        let mut entropy = [0u8; 32];

        let mut checksum = 0u8;

        for absolute_bit in 0..264usize {
            let word_position = absolute_bit / PHASE6B1_RECOVERY_WORD_INDEX_BITS;

            let bit_within_word = absolute_bit % PHASE6B1_RECOVERY_WORD_INDEX_BITS;

            let shift = 10 - bit_within_word;

            let bit = ((indices[word_position] >> shift) & 1) as u8;

            if absolute_bit < 256 {
                let byte_index = absolute_bit / 8;

                let entropy_shift = 7 - (absolute_bit % 8);

                entropy[byte_index] |= bit << entropy_shift;
            } else {
                let checksum_shift = 7 - (absolute_bit - 256);

                checksum |= bit << checksum_shift;
            }
        }

        (entropy, checksum)
    }
}
