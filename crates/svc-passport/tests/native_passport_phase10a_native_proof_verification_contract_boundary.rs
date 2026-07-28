#[cfg(not(feature = "native-passport"))]
#[test]
fn phase10a_proof_verification_boundary_is_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not compile native proof-verification boundary surfaces"
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use std::{fs, path::PathBuf};

    use svc_passport::native::{
        native_proof_verification_contract_boundary_posture,
        review_native_proof_verification_contract_boundary, B3DigestHex, ChallengeIdV1, DeviceIdV1,
        NativePassportProofAuthority, NativePassportProofKind, NativePassportSurface,
        NativeProofSignatureAlgorithm, NativeProofSignedPayloadHex,
        NativeProofSigningAdapterEnvelopeV1, NativeProofSigningOperationKind,
        NativeProofVerificationContractBoundaryDraftV1,
        NativeProofVerificationContractBoundaryReviewError, PassportIdV1,
        NATIVE_PASSPORT_PHASE10A_LABEL, PHASE10A_ENABLED_SURFACES,
        PHASE10A_FORBIDDEN_PROOF_VERIFICATION_AUTHORITY_FLAGS,
        PHASE10A_PROOF_VERIFICATION_BOUNDARY_DOMAIN, PHASE10A_PROOF_VERIFICATION_BOUNDARY_VERSION,
        PHASE8B_DEVICE_PROOF_SIGNATURE_ALGORITHM, PHASE8B_PROOF_CONTRACT_DOMAIN,
        PHASE8B_PROOF_CONTRACT_VERSION, PHASE8B_ROOT_PROOF_SIGNATURE_ALGORITHM,
        PHASE8C_PROOF_REQUEST_CONTRACT_DOMAIN, PHASE8C_PROOF_REQUEST_CONTRACT_VERSION,
        PHASE9B_ENABLED_SURFACES, PHASE9B_PROOF_SIGNING_ADAPTER_DOMAIN,
        PHASE9B_PROOF_SIGNING_ADAPTER_VERSION,
    };

    const HEX_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const HEX_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const HEX_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    const HEX_D: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
    const HEX_E: &str = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";

    fn repo_file(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
    }

    fn digest(label: &'static str, hex: &'static str) -> B3DigestHex {
        B3DigestHex::parse(label, hex).expect(label)
    }

    fn challenge_id() -> ChallengeIdV1 {
        ChallengeIdV1::parse(format!("challenge:v1:b3:{HEX_A}")).expect("challenge id")
    }

    fn other_challenge_id() -> ChallengeIdV1 {
        ChallengeIdV1::parse(format!("challenge:v1:b3:{HEX_E}")).expect("other challenge id")
    }

    fn passport_id() -> PassportIdV1 {
        PassportIdV1::parse(format!("passport:v1:main:ed25519:b3:{HEX_B}")).expect("passport id")
    }

    fn device_id() -> DeviceIdV1 {
        DeviceIdV1::parse(format!("device:v1:ed25519:b3:{HEX_C}")).expect("device id")
    }

    fn signed_payload() -> NativeProofSignedPayloadHex {
        NativeProofSignedPayloadHex::parse("signed_payload_hex", "44".repeat(64))
            .expect("signed payload")
    }

    fn root_envelope() -> NativeProofSigningAdapterEnvelopeV1 {
        NativeProofSigningAdapterEnvelopeV1 {
            contract_domain: PHASE9B_PROOF_SIGNING_ADAPTER_DOMAIN,
            contract_version: PHASE9B_PROOF_SIGNING_ADAPTER_VERSION,
            operation_kind: NativeProofSigningOperationKind::RootProof,
            signer_authority: NativePassportProofAuthority::RootAuthority,
            challenge_id: challenge_id(),
            passport_id: Some(passport_id()),
            device_id: Some(device_id()),
            proof_contract_domain: PHASE8B_PROOF_CONTRACT_DOMAIN,
            proof_contract_version: PHASE8B_PROOF_CONTRACT_VERSION,
            request_contract_domain: None,
            request_contract_version: None,
            proof_kind: NativePassportProofKind::RootRegistration,
            transcript_hash: digest("proof_transcript_hash", HEX_D),
            signer_public_key_label: "public-key:v1:phase10a-root",
            requested_signature_algorithm: PHASE8B_ROOT_PROOF_SIGNATURE_ALGORITHM,
            signing_requested_at_ms: 1_035_000,
            signed_payload_hex: signed_payload(),
            signed_payload_present: true,
            secret_material_exposed: false,
            runtime_io_performed: false,
            wallet_or_ledger_mutated: false,
        }
    }

    fn request_envelope() -> NativeProofSigningAdapterEnvelopeV1 {
        NativeProofSigningAdapterEnvelopeV1 {
            contract_domain: PHASE9B_PROOF_SIGNING_ADAPTER_DOMAIN,
            contract_version: PHASE9B_PROOF_SIGNING_ADAPTER_VERSION,
            operation_kind: NativeProofSigningOperationKind::RequestProof,
            signer_authority: NativePassportProofAuthority::DeviceAuthority,
            challenge_id: challenge_id(),
            passport_id: Some(passport_id()),
            device_id: Some(device_id()),
            proof_contract_domain: PHASE8B_PROOF_CONTRACT_DOMAIN,
            proof_contract_version: PHASE8B_PROOF_CONTRACT_VERSION,
            request_contract_domain: Some(PHASE8C_PROOF_REQUEST_CONTRACT_DOMAIN),
            request_contract_version: Some(PHASE8C_PROOF_REQUEST_CONTRACT_VERSION),
            proof_kind: NativePassportProofKind::DeviceSession,
            transcript_hash: digest("request_transcript_hash", HEX_E),
            signer_public_key_label: "public-key:v1:phase10a-device",
            requested_signature_algorithm: PHASE8B_DEVICE_PROOF_SIGNATURE_ALGORITHM,
            signing_requested_at_ms: 1_045_000,
            signed_payload_hex: signed_payload(),
            signed_payload_present: true,
            secret_material_exposed: false,
            runtime_io_performed: false,
            wallet_or_ledger_mutated: false,
        }
    }

    fn verification_draft(
        envelope: &NativeProofSigningAdapterEnvelopeV1,
    ) -> NativeProofVerificationContractBoundaryDraftV1 {
        NativeProofVerificationContractBoundaryDraftV1 {
            contract_domain: PHASE10A_PROOF_VERIFICATION_BOUNDARY_DOMAIN,
            contract_version: PHASE10A_PROOF_VERIFICATION_BOUNDARY_VERSION,
            operation_kind: envelope.operation_kind,
            expected_authority: envelope.signer_authority,
            challenge_id: envelope.challenge_id.clone(),
            passport_id: envelope.passport_id.clone(),
            device_id: envelope.device_id.clone(),
            proof_contract_domain: envelope.proof_contract_domain,
            proof_contract_version: envelope.proof_contract_version,
            request_contract_domain: envelope.request_contract_domain,
            request_contract_version: envelope.request_contract_version,
            proof_kind: envelope.proof_kind,
            transcript_hash: envelope.transcript_hash.clone(),
            signer_public_key_label: envelope.signer_public_key_label,
            expected_signature_algorithm: envelope.requested_signature_algorithm,
            signed_payload_hex: envelope.signed_payload_hex.clone(),
            verification_requested_at_ms: envelope.signing_requested_at_ms + 1,
            expects_signed_payload_present: true,
            requests_cryptographic_signature_verification_runtime: false,
            requests_request_proof_verification_runtime: false,
            requests_service_challenge_verification: false,
            requests_replay_store_mutation: false,
            requests_challenge_consumption: false,
            requests_capability_consumption: false,
            requests_capability_issuance: false,
            requests_capability_lifecycle_runtime: false,
            requests_secret_key_access: false,
            requests_key_loading: false,
            requests_key_derivation_runtime: false,
            requests_vault_unlock: false,
            includes_vault_runtime: false,
            requests_platform_sealer_unseal: false,
            requests_runtime_io: false,
            adds_routes: false,
            requests_storage_mutation: false,
            requests_wallet_or_ledger_mutation: false,
        }
    }

    #[test]
    fn phase10a_label_and_posture_are_locked() {
        let posture = native_proof_verification_contract_boundary_posture();

        assert_eq!(
            NATIVE_PASSPORT_PHASE10A_LABEL,
            "NATIVE_PASSPORT_PHASE10A_NATIVE_PROOF_VERIFICATION_CONTRACT_BOUNDARY"
        );
        assert_eq!(posture.phase_label, NATIVE_PASSPORT_PHASE10A_LABEL);
        assert!(posture.proof_verification_contract_boundary_added);
        assert!(!posture.cryptographic_signature_verification_runtime_added);
        assert!(!posture.request_proof_verification_runtime_added);
        assert!(!posture.replay_store_mutation_added);
        assert!(!posture.challenge_consumption_added);
        assert!(!posture.capability_issuance_added);
        assert!(!posture.runtime_io_added);
        assert!(!posture.wallet_or_ledger_mutation_added);
        assert!(!posture.native_secret_implementation_added);

        for forbidden in [
            "cryptographic_signature_verification_runtime",
            "request_proof_verification_runtime",
            "replay_store_mutation",
            "challenge_consumption",
            "secret_key_access",
            "runtime_io",
            "wallet_spend",
            "ledger_mutation",
        ] {
            assert!(posture.forbidden_authority_flags.contains(&forbidden));
            assert!(PHASE10A_FORBIDDEN_PROOF_VERIFICATION_AUTHORITY_FLAGS.contains(&forbidden));
        }
    }

    #[test]
    fn phase10a_surfaces_extend_phase9b_without_back_mutating_it() {
        assert_eq!(
            PHASE10A_ENABLED_SURFACES.len(),
            PHASE9B_ENABLED_SURFACES.len() + 1
        );
        assert_eq!(
            &PHASE10A_ENABLED_SURFACES[..PHASE9B_ENABLED_SURFACES.len()],
            PHASE9B_ENABLED_SURFACES
        );
        assert_eq!(
            PHASE10A_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::ProofVerificationContractBoundaryDto)
        );
        assert_eq!(
            PHASE9B_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::ProofSigningAdapterDto)
        );
    }

    #[test]
    fn phase10a_reviews_root_signed_envelope_without_cryptographic_runtime() {
        let envelope = root_envelope();
        let decision = review_native_proof_verification_contract_boundary(
            &envelope,
            verification_draft(&envelope),
        )
        .expect("root signed envelope boundary should review");

        assert_eq!(
            decision.contract_domain,
            PHASE10A_PROOF_VERIFICATION_BOUNDARY_DOMAIN
        );
        assert_eq!(
            decision.operation_kind,
            NativeProofSigningOperationKind::RootProof
        );
        assert_eq!(
            decision.reviewed_authority,
            NativePassportProofAuthority::RootAuthority
        );
        assert_eq!(decision.transcript_hash, envelope.transcript_hash);
        assert_eq!(decision.signed_payload_hex, envelope.signed_payload_hex);
        assert!(!decision.cryptographic_verification_performed);
        assert!(!decision.replay_state_mutated);
        assert!(!decision.challenge_consumed);
        assert!(!decision.capability_state_changed);
        assert!(!decision.runtime_io_performed);
        assert!(!decision.wallet_or_ledger_mutated);
        assert!(decision.contract_only);
    }

    #[test]
    fn phase10a_reviews_request_signed_envelope_without_consumption_or_mutation() {
        let envelope = request_envelope();
        let decision = review_native_proof_verification_contract_boundary(
            &envelope,
            verification_draft(&envelope),
        )
        .expect("request signed envelope boundary should review");

        assert_eq!(
            decision.operation_kind,
            NativeProofSigningOperationKind::RequestProof
        );
        assert_eq!(
            decision.reviewed_authority,
            NativePassportProofAuthority::DeviceAuthority
        );
        assert_eq!(
            decision.request_contract_domain,
            Some(PHASE8C_PROOF_REQUEST_CONTRACT_DOMAIN)
        );
        assert_eq!(
            decision.expected_signature_algorithm,
            NativeProofSignatureAlgorithm::DeviceEd25519V1
        );
        assert_eq!(decision.signed_payload_hex.as_str().len(), 128);
        assert!(!decision.cryptographic_verification_performed);
        assert!(!decision.replay_state_mutated);
        assert!(!decision.challenge_consumed);
        assert!(!decision.capability_state_changed);
    }

    #[test]
    fn phase10a_rejects_unsigned_unsafe_mismatched_and_early_verification_envelopes() {
        let mut unsigned = root_envelope();
        unsigned.signed_payload_present = false;
        assert_eq!(
            review_native_proof_verification_contract_boundary(
                &unsigned,
                verification_draft(&unsigned),
            ),
            Err(NativeProofVerificationContractBoundaryReviewError::SigningEnvelopeUnsigned)
        );

        let mut unsafe_envelope = root_envelope();
        unsafe_envelope.runtime_io_performed = true;
        assert_eq!(
            review_native_proof_verification_contract_boundary(
                &unsafe_envelope,
                verification_draft(&unsafe_envelope),
            ),
            Err(NativeProofVerificationContractBoundaryReviewError::SigningEnvelopeUnsafe)
        );

        let envelope = root_envelope();

        let mut wrong_challenge = verification_draft(&envelope);
        wrong_challenge.challenge_id = other_challenge_id();
        assert_eq!(
            review_native_proof_verification_contract_boundary(&envelope, wrong_challenge),
            Err(NativeProofVerificationContractBoundaryReviewError::ChallengeIdMismatch)
        );

        let mut wrong_payload = verification_draft(&envelope);
        wrong_payload.signed_payload_hex =
            NativeProofSignedPayloadHex::parse("other_payload", "55".repeat(64))
                .expect("other payload");
        assert_eq!(
            review_native_proof_verification_contract_boundary(&envelope, wrong_payload),
            Err(NativeProofVerificationContractBoundaryReviewError::SignedPayloadMismatch)
        );

        let mut wrong_algorithm = verification_draft(&envelope);
        wrong_algorithm.expected_signature_algorithm =
            NativeProofSignatureAlgorithm::DeviceEd25519V1;
        assert_eq!(
            review_native_proof_verification_contract_boundary(&envelope, wrong_algorithm),
            Err(NativeProofVerificationContractBoundaryReviewError::SignatureAlgorithmMismatch)
        );

        let mut early = verification_draft(&envelope);
        early.verification_requested_at_ms = envelope.signing_requested_at_ms - 1;
        assert_eq!(
            review_native_proof_verification_contract_boundary(&envelope, early),
            Err(NativeProofVerificationContractBoundaryReviewError::VerificationTimeBeforeSigning)
        );
    }

    #[test]
    fn phase10a_rejects_all_unsafe_verification_authority_flags() {
        let envelope = request_envelope();

        let mutators: &[fn(&mut NativeProofVerificationContractBoundaryDraftV1)] = &[
            |draft| draft.requests_cryptographic_signature_verification_runtime = true,
            |draft| draft.requests_request_proof_verification_runtime = true,
            |draft| draft.requests_service_challenge_verification = true,
            |draft| draft.requests_replay_store_mutation = true,
            |draft| draft.requests_challenge_consumption = true,
            |draft| draft.requests_capability_consumption = true,
            |draft| draft.requests_capability_issuance = true,
            |draft| draft.requests_capability_lifecycle_runtime = true,
            |draft| draft.requests_secret_key_access = true,
            |draft| draft.requests_key_loading = true,
            |draft| draft.requests_key_derivation_runtime = true,
            |draft| draft.requests_vault_unlock = true,
            |draft| draft.includes_vault_runtime = true,
            |draft| draft.requests_platform_sealer_unseal = true,
            |draft| draft.requests_runtime_io = true,
            |draft| draft.adds_routes = true,
            |draft| draft.requests_storage_mutation = true,
            |draft| draft.requests_wallet_or_ledger_mutation = true,
        ];

        for mutate in mutators {
            let mut draft = verification_draft(&envelope);
            mutate(&mut draft);
            assert_eq!(
                review_native_proof_verification_contract_boundary(&envelope, draft),
                Err(NativeProofVerificationContractBoundaryReviewError::UnsafeVerificationAuthorityFlag)
            );
        }
    }

    #[test]
    fn phase10a_source_remains_contract_boundary_without_crypto_replay_io_or_mutation() {
        let source = fs::read_to_string(repo_file("src/native/proof_verification.rs"))
            .expect("proof verification boundary source");
        let native_mod =
            fs::read_to_string(repo_file("src/native/mod.rs")).expect("native module source");

        assert!(source.contains("NativeProofVerificationContractBoundaryDraftV1"));
        assert!(source.contains("NativeProofVerificationContractBoundaryDecisionV1"));
        assert!(source.contains("review_native_proof_verification_contract_boundary"));
        assert!(native_mod.contains("ProofVerificationContractBoundaryDto"));
        assert!(!source.contains("serde_json"));

        for forbidden_runtime_pattern in [
            "Router::",
            ".route(",
            "verify_signature(",
            "verify_proof(",
            "verify_request_proof(",
            "load_secret",
            "load_key",
            "secret_key_bytes",
            "consume_challenge(",
            "consume_capability(",
            "replay_store.write(",
            "issue_capability(",
            "generate_device_key(",
            "derive_root_key(",
            "derive_device_key(",
            "validate_pin(",
            "derive_pin(",
            "unlock_vault(",
            "platform_seal(",
            "platform_unseal(",
            "encrypt(",
            "decrypt(",
            "rpc.submit(",
            "rpc.send(",
            "runtime_read(",
            "runtime_write(",
            "storage.write(",
            "wallet.spend(",
            "ledger.write(",
            "mint_roc(",
            "burn_roc(",
            "mnemonic:",
            "seed_phrase:",
            "raw_pin:",
            "pin_hash:",
            "root_private_key:",
            "device_private_key:",
            "vault_master_key:",
            "ciphertext:",
            "plaintext:",
            "sealed_bytes:",
            "request_signature_bytes:",
            "proof_signature_bytes:",
            "capability_token:",
        ] {
            assert!(
                !source.contains(forbidden_runtime_pattern),
                "Phase 10A proof-verification boundary source must not gain runtime pattern {forbidden_runtime_pattern}"
            );
        }
    }
}
