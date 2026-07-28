//! RO:WHAT — Maps canonical Phase 6B1 recovery indices to BIP-0039 English words inside a zeroizing native-only phrase buffer.
//! RO:WHY — Onboarding Phase 6 needs exact recovery words before CrabLink adds the one-time platform-native display and acknowledgement surface.
//! RO:INTERACTS — recovery_mnemonic_indices.rs, bip39_english.txt, and the later CrabLink desktop recovery surface.
//! RO:INVARIANTS — exactly 2,048 canonical words; exactly 24 mapped words; the phrase exists only during a bounded callback; phrase-return DTOs are absent.
//! RO:SECURITY — phrase bytes are held in Zeroizing<String>; no filesystem write, logging, clipboard, WebView return, root export, capability issuance, wallet mutation, or ledger mutation.
//! RO:TEST — module tests and native_passport_onboarding_phase6b2a_recovery_mnemonic_words.rs.

use zeroize::Zeroizing;

use super::{
    NativeRecoveryMnemonicIndicesV1, PHASE6B1_RECOVERY_WORDLIST_LENGTH,
    PHASE6B1_RECOVERY_WORD_COUNT,
};

pub const ONBOARDING_PHASE6B2A_NATIVE_LABEL: &str =
    "ONBOARDING_PHASE6B2A_CANONICAL_BIP39_ENGLISH_WORD_MAPPING";

pub const PHASE6B2A_BIP39_ENGLISH_WORDLIST_SHA256: &str =
    "2f5eed53a4727b4bf8880d8f3f199efc90e58503646d9ff8eff3a2ed3b24dbda";

const BIP39_ENGLISH_WORDLIST_TEXT: &str = include_str!("bip39_english.txt");

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeRecoveryMnemonicWordsError {
    WordlistLengthMismatch {
        actual: usize,
        expected: usize,
    },
    WordIndexOutOfRange {
        position: usize,
        maximum_exclusive: usize,
    },
    EmptyWord {
        position: usize,
    },
}

pub fn bip39_english_wordlist_count() -> usize {
    BIP39_ENGLISH_WORDLIST_TEXT.lines().count()
}

/// Builds the secret phrase only for the duration of `visitor`.
///
/// The visitor must remain inside the native/platform boundary. This function
/// deliberately does not return the phrase, expose a phrase getter, or
/// implement serialization for phrase material.
pub fn with_native_recovery_mnemonic_phrase<T>(
    recovery: &NativeRecoveryMnemonicIndicesV1,
    visitor: impl FnOnce(&str, &str) -> T,
) -> Result<T, NativeRecoveryMnemonicWordsError> {
    let words: Vec<&str> = BIP39_ENGLISH_WORDLIST_TEXT.lines().collect();

    if words.len() != PHASE6B1_RECOVERY_WORDLIST_LENGTH {
        return Err(NativeRecoveryMnemonicWordsError::WordlistLengthMismatch {
            actual: words.len(),
            expected: PHASE6B1_RECOVERY_WORDLIST_LENGTH,
        });
    }

    let mut phrase: Zeroizing<String> = Zeroizing::new(String::with_capacity(256));

    recovery.with_native_word_indices(|indices| {
        for (position, index) in indices.iter().copied().enumerate() {
            let index = usize::from(index);

            let word = words.get(index).copied().ok_or(
                NativeRecoveryMnemonicWordsError::WordIndexOutOfRange {
                    position,
                    maximum_exclusive: PHASE6B1_RECOVERY_WORDLIST_LENGTH,
                },
            )?;

            if word.is_empty() {
                return Err(NativeRecoveryMnemonicWordsError::EmptyWord { position });
            }

            if position > 0 {
                phrase.push(' ');
            }

            phrase.push_str(word);
        }

        Ok::<(), NativeRecoveryMnemonicWordsError>(())
    })?;

    debug_assert_eq!(
        phrase.split_ascii_whitespace().count(),
        PHASE6B1_RECOVERY_WORD_COUNT,
    );

    let fingerprint = recovery.redacted_fingerprint_hex();

    Ok(visitor(phrase.as_str(), fingerprint.as_str()))
}

#[cfg(test)]
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

    use sha2::{Digest, Sha256};

    use crate::native::{derive_native_recovery_mnemonic_indices, NativeSecretBytes};

    fn factor(bytes: Vec<u8>) -> NativeSecretBytes {
        NativeSecretBytes::new(bytes).expect("bounded recovery-factor fixture")
    }

    fn phrase_for(bytes: Vec<u8>) -> String {
        let indices = derive_native_recovery_mnemonic_indices(&factor(bytes))
            .expect("canonical factor should encode");

        with_native_recovery_mnemonic_phrase(&indices, |phrase, _fingerprint| phrase.to_owned())
            .expect("canonical indices should map")
    }

    #[test]
    fn phase6b2a_wordlist_is_the_locked_canonical_english_file() {
        let words: Vec<&str> = BIP39_ENGLISH_WORDLIST_TEXT.lines().collect();

        assert_eq!(words.len(), 2048);
        assert_eq!(words.first(), Some(&"abandon"),);
        assert_eq!(words.last(), Some(&"zoo"),);

        assert!(words.windows(2).all(|pair| pair[0] < pair[1]));

        assert!(words.iter().all(|word| {
            !word.is_empty() && word.bytes().all(|byte| byte.is_ascii_lowercase())
        }));

        let digest = Sha256::digest(BIP39_ENGLISH_WORDLIST_TEXT.as_bytes());

        assert_eq!(
            hex_lower(digest.as_slice(),),
            PHASE6B2A_BIP39_ENGLISH_WORDLIST_SHA256,
        );
    }

    #[test]
    fn phase6b2a_zero_entropy_maps_to_the_locked_24_word_vector() {
        let mut expected = vec!["abandon"; 23];

        expected.push("art");

        assert_eq!(phrase_for(vec![0u8; 32]), expected.join(" "),);
    }

    #[test]
    fn phase6b2a_max_entropy_maps_to_the_locked_24_word_vector() {
        let mut expected = vec!["zoo"; 23];

        expected.push("vote");

        assert_eq!(phrase_for(vec![0xffu8; 32]), expected.join(" "),);
    }

    #[test]
    fn phase6b2a_callback_receives_24_words_and_fingerprint() {
        let indices = derive_native_recovery_mnemonic_indices(&factor((0u8..32u8).collect()))
            .expect("canonical factor should encode");

        let (word_count, fingerprint_length) =
            with_native_recovery_mnemonic_phrase(&indices, |phrase, fingerprint| {
                (phrase.split_ascii_whitespace().count(), fingerprint.len())
            })
            .expect("canonical indices should map");

        assert_eq!(word_count, 24);
        assert_eq!(fingerprint_length, 16,);
    }

    #[test]
    fn phase6b2a_phrase_owner_is_zeroizing_and_not_serializable() {
        let source = include_str!("recovery_mnemonic_words.rs");

        let production_source = source
            .split_once("#[cfg(test)]")
            .map(|(production, _tests)| production)
            .expect("test-only boundary must be present");

        assert!(production_source.contains("Zeroizing<String>"));

        assert!(production_source.contains("FnOnce(&str, &str)"));

        for forbidden in ["serde::", "Serialize", "println!", "eprintln!", "tracing::"] {
            assert!(
                !production_source.contains(forbidden),
                "production source contains {forbidden}",
            );
        }
    }
}
