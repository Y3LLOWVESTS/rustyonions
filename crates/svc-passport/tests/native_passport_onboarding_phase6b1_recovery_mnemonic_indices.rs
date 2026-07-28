#[cfg(not(feature = "native-passport"))]
#[test]
fn phase6b1_recovery_indices_remain_feature_gated_by_default() {
    assert!(!cfg!(feature = "native-passport"));
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use svc_passport::native::{
        derive_native_recovery_mnemonic_indices, NativeRecoveryMnemonicIndicesError,
        NativeSecretBytes, ONBOARDING_PHASE6B1_NATIVE_LABEL, PHASE6B1_RECOVERY_CHECKSUM_BITS,
        PHASE6B1_RECOVERY_ENTROPY_BYTES, PHASE6B1_RECOVERY_FINGERPRINT_HEX_LENGTH,
        PHASE6B1_RECOVERY_WORDLIST_LENGTH, PHASE6B1_RECOVERY_WORD_COUNT,
    };

    fn factor(fill: u8, length: usize) -> NativeSecretBytes {
        NativeSecretBytes::new(vec![fill; length]).expect("bounded recovery-factor fixture")
    }

    #[test]
    fn phase6b1_public_api_encodes_the_existing_recovery_factor() {
        let recovery = derive_native_recovery_mnemonic_indices(&factor(0, 32))
            .expect("canonical factor should encode");

        let final_index = recovery.with_native_word_indices(|indices| indices[23]);

        assert_eq!(
            ONBOARDING_PHASE6B1_NATIVE_LABEL,
            "ONBOARDING_PHASE6B1_RECOVERY_MNEMONIC_INDICES",
        );

        assert_eq!(PHASE6B1_RECOVERY_ENTROPY_BYTES, 32,);

        assert_eq!(PHASE6B1_RECOVERY_CHECKSUM_BITS, 8,);

        assert_eq!(PHASE6B1_RECOVERY_WORD_COUNT, 24,);

        assert_eq!(PHASE6B1_RECOVERY_WORDLIST_LENGTH, 2048,);

        assert_eq!(recovery.word_count(), 24,);

        assert_eq!(final_index, 102,);

        assert_eq!(
            recovery.redacted_fingerprint_hex().len(),
            PHASE6B1_RECOVERY_FINGERPRINT_HEX_LENGTH,
        );
    }

    #[test]
    fn phase6b1_public_api_rejects_wrong_factor_length() {
        assert_eq!(
            derive_native_recovery_mnemonic_indices(&factor(9, 16),),
            Err(
                NativeRecoveryMnemonicIndicesError::RecoveryFactorLengthMismatch {
                    actual: 16,
                    expected: 32,
                },
            ),
        );
    }

    #[test]
    fn phase6b1_public_debug_never_discloses_indices() {
        let recovery = derive_native_recovery_mnemonic_indices(&factor(0, 32))
            .expect("canonical factor should encode");

        let debug = format!("{recovery:?}");

        assert!(debug.contains("REDACTED"));

        assert!(!debug.contains("102"));

        assert!(!debug.contains(&recovery.redacted_fingerprint_hex(),));
    }
}
