#[cfg(not(feature = "native-passport"))]
#[test]
fn phase6b2a_recovery_words_remain_feature_gated_by_default() {
    assert!(!cfg!(feature = "native-passport"));
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use std::{fs, path::PathBuf};

    use svc_passport::native::{
        bip39_english_wordlist_count, derive_native_recovery_mnemonic_indices,
        with_native_recovery_mnemonic_phrase, NativeSecretBytes, ONBOARDING_PHASE6B2A_NATIVE_LABEL,
        PHASE6B2A_BIP39_ENGLISH_WORDLIST_SHA256,
    };

    fn factor(bytes: Vec<u8>) -> NativeSecretBytes {
        NativeSecretBytes::new(bytes).expect("bounded recovery-factor fixture")
    }

    #[test]
    fn phase6b2a_public_api_maps_zero_entropy_to_24_words() {
        let indices = derive_native_recovery_mnemonic_indices(&factor(vec![0u8; 32]))
            .expect("canonical factor should encode");

        let (phrase, fingerprint_length) =
            with_native_recovery_mnemonic_phrase(&indices, |phrase, fingerprint| {
                (phrase.to_owned(), fingerprint.len())
            })
            .expect("canonical indices should map");

        let mut expected = vec!["abandon"; 23];

        expected.push("art");

        assert_eq!(phrase, expected.join(" "),);

        assert_eq!(phrase.split_ascii_whitespace().count(), 24,);

        assert_eq!(fingerprint_length, 16,);

        assert_eq!(bip39_english_wordlist_count(), 2048,);

        assert_eq!(
            ONBOARDING_PHASE6B2A_NATIVE_LABEL,
            "ONBOARDING_PHASE6B2A_CANONICAL_BIP39_ENGLISH_WORD_MAPPING",
        );

        assert_eq!(
            PHASE6B2A_BIP39_ENGLISH_WORDLIST_SHA256,
            "2f5eed53a4727b4bf8880d8f3f199efc90e58503646d9ff8eff3a2ed3b24dbda",
        );
    }

    #[test]
    fn phase6b2a_public_api_maps_max_entropy_to_vote() {
        let indices = derive_native_recovery_mnemonic_indices(&factor(vec![0xffu8; 32]))
            .expect("canonical factor should encode");

        let phrase = with_native_recovery_mnemonic_phrase(&indices, |phrase, _fingerprint| {
            phrase.to_owned()
        })
        .expect("canonical indices should map");

        let mut expected = vec!["zoo"; 23];

        expected.push("vote");

        assert_eq!(phrase, expected.join(" "),);
    }

    #[test]
    fn phase6b2a_source_keeps_phrase_inside_native_callback() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        let source = fs::read_to_string(root.join("src/native/recovery_mnemonic_words.rs"))
            .expect("Phase 6B2A source");

        let production_source = source
            .split_once("#[cfg(test)]")
            .map(|(production, _tests)| production)
            .expect("test-only boundary must be present");

        let production_code = production_source
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");

        let wordlist = fs::read_to_string(root.join("src/native/bip39_english.txt"))
            .expect("canonical English wordlist");

        for required in [
            "Zeroizing<String>",
            "FnOnce(&str, &str)",
            "include_str!(\"bip39_english.txt\")",
            "with_native_word_indices",
        ] {
            assert!(production_code.contains(required), "missing {required}",);
        }

        for forbidden in [
            "serde::",
            "Serialize",
            "println!",
            "eprintln!",
            "tracing::",
            "clipboard",
            "localStorage",
            "sessionStorage",
            "wallet_or_ledger_mutated: true",
            "recovery_root_exported: true",
        ] {
            assert!(
                !production_code.contains(forbidden),
                "production code contains forbidden {forbidden}",
            );
        }

        assert_eq!(wordlist.lines().count(), 2048,);

        assert_eq!(wordlist.lines().next(), Some("abandon"),);

        assert_eq!(wordlist.lines().last(), Some("zoo"),);
    }
}
