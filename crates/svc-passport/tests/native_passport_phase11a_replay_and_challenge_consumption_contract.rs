#[cfg(not(feature = "native-passport"))]
#[test]
fn phase11a_replay_consumption_contract_is_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not compile native replay/challenge consumption surfaces"
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use std::{fs, path::PathBuf};

    use svc_passport::native::{
        native_replay_challenge_consumption_contract_posture,
        review_native_replay_challenge_consumption_contract, B3DigestHex, ChallengeIdV1,
        DeviceIdV1, NativePassportProofAuthority, NativePassportProofKind, NativePassportSurface,
        NativeProofSignedPayloadHex, NativeProofSigningOperationKind,
        NativeProofVerificationAdapterEnvelopeV1, NativeReplayChallengeConsumptionContractDraftV1,
        NativeReplayChallengeConsumptionContractReviewError, PassportIdV1,
        NATIVE_PASSPORT_PHASE11A_LABEL, PHASE10B_ENABLED_SURFACES,
        PHASE10B_PROOF_VERIFICATION_ADAPTER_DOMAIN, PHASE10B_PROOF_VERIFICATION_ADAPTER_VERSION,
        PHASE11A_ENABLED_SURFACES, PHASE11A_FORBIDDEN_CONSUMPTION_AUTHORITY_FLAGS,
        PHASE11A_REPLAY_CHALLENGE_CONSUMPTION_CONTRACT_DOMAIN,
        PHASE11A_REPLAY_CHALLENGE_CONSUMPTION_CONTRACT_VERSION,
        PHASE8B_DEVICE_PROOF_SIGNATURE_ALGORITHM, PHASE8B_PROOF_CONTRACT_DOMAIN,
        PHASE8B_PROOF_CONTRACT_VERSION, PHASE8B_ROOT_PROOF_SIGNATURE_ALGORITHM,
        PHASE8C_PROOF_REQUEST_CONTRACT_DOMAIN, PHASE8C_PROOF_REQUEST_CONTRACT_VERSION,
    };

    const HEX_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const HEX_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const HEX_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    const HEX_D: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
    const HEX_E: &str = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
    const HEX_F: &str = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";

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
        ChallengeIdV1::parse(format!("challenge:v1:b3:{HEX_F}")).expect("other challenge id")
    }

    fn passport_id() -> PassportIdV1 {
        PassportIdV1::parse(format!("passport:v1:main:ed25519:b3:{HEX_B}")).expect("passport id")
    }

    fn device_id() -> DeviceIdV1 {
        DeviceIdV1::parse(format!("device:v1:ed25519:b3:{HEX_C}")).expect("device id")
    }

    fn signed_payload(hex_pair: &'static str) -> NativeProofSignedPayloadHex {
        NativeProofSignedPayloadHex::parse("phase11a_signed_payload_hex", hex_pair.repeat(64))
            .expect("signed payload")
    }

    fn root_envelope() -> NativeProofVerificationAdapterEnvelopeV1 {
        NativeProofVerificationAdapterEnvelopeV1 {
            contract_domain: PHASE10B_PROOF_VERIFICATION_ADAPTER_DOMAIN,
            contract_version: PHASE10B_PROOF_VERIFICATION_ADAPTER_VERSION,
            operation_kind: NativeProofSigningOperationKind::RootProof,
            reviewed_authority: NativePassportProofAuthority::RootAuthority,
            challenge_id: challenge_id(),
            passport_id: Some(passport_id()),
            device_id: Some(device_id()),
            proof_contract_domain: PHASE8B_PROOF_CONTRACT_DOMAIN,
            proof_contract_version: PHASE8B_PROOF_CONTRACT_VERSION,
            request_contract_domain: None,
            request_contract_version: None,
            proof_kind: NativePassportProofKind::RootRegistration,
            transcript_hash: digest("proof_transcript_hash", HEX_D),
            signer_public_key_label: "public-key:v1:phase11a-root",
            expected_signature_algorithm: PHASE8B_ROOT_PROOF_SIGNATURE_ALGORITHM,
            signed_payload_hex: signed_payload("aa"),
            verification_requested_at_ms: 1_035_001,
            evidence_label: "phase11a-root-evidence",
            verifier_public_key_label: "public-key:v1:phase11a-verifier",
            signed_payload_accepted: true,
            replay_state_mutated: false,
            challenge_consumed: false,
            capability_state_changed: false,
            runtime_io_performed: false,
            wallet_or_ledger_mutated: false,
        }
    }

    fn request_envelope() -> NativeProofVerificationAdapterEnvelopeV1 {
        NativeProofVerificationAdapterEnvelopeV1 {
            contract_domain: PHASE10B_PROOF_VERIFICATION_ADAPTER_DOMAIN,
            contract_version: PHASE10B_PROOF_VERIFICATION_ADAPTER_VERSION,
            operation_kind: NativeProofSigningOperationKind::RequestProof,
            reviewed_authority: NativePassportProofAuthority::DeviceAuthority,
            challenge_id: challenge_id(),
            passport_id: Some(passport_id()),
            device_id: Some(device_id()),
            proof_contract_domain: PHASE8B_PROOF_CONTRACT_DOMAIN,
            proof_contract_version: PHASE8B_PROOF_CONTRACT_VERSION,
            request_contract_domain: Some(PHASE8C_PROOF_REQUEST_CONTRACT_DOMAIN),
            request_contract_version: Some(PHASE8C_PROOF_REQUEST_CONTRACT_VERSION),
            proof_kind: NativePassportProofKind::DeviceSession,
            transcript_hash: digest("request_transcript_hash", HEX_E),
            signer_public_key_label: "public-key:v1:phase11a-device",
            expected_signature_algorithm: PHASE8B_DEVICE_PROOF_SIGNATURE_ALGORITHM,
            signed_payload_hex: signed_payload("bb"),
            verification_requested_at_ms: 1_045_001,
            evidence_label: "phase11a-request-evidence",
            verifier_public_key_label: "public-key:v1:phase11a-verifier",
            signed_payload_accepted: true,
            replay_state_mutated: false,
            challenge_consumed: false,
            capability_state_changed: false,
            runtime_io_performed: false,
            wallet_or_ledger_mutated: false,
        }
    }

    fn consumption_draft(
        envelope: &NativeProofVerificationAdapterEnvelopeV1,
    ) -> NativeReplayChallengeConsumptionContractDraftV1 {
        NativeReplayChallengeConsumptionContractDraftV1 {
            contract_domain: PHASE11A_REPLAY_CHALLENGE_CONSUMPTION_CONTRACT_DOMAIN,
            contract_version: PHASE11A_REPLAY_CHALLENGE_CONSUMPTION_CONTRACT_VERSION,
            operation_kind: envelope.operation_kind,
            expected_authority: envelope.reviewed_authority,
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
            expected_signature_algorithm: envelope.expected_signature_algorithm,
            signed_payload_hex: envelope.signed_payload_hex.clone(),
            verification_evidence_label: envelope.evidence_label,
            verifier_public_key_label: envelope.verifier_public_key_label,
            replay_key_hash: digest("replay_key_hash", HEX_D),
            idempotency_key_hash: digest("idempotency_key_hash", HEX_E),
            consumption_requested_at_ms: envelope.verification_requested_at_ms + 1,
            expects_verified_payload_accepted: true,
            expects_replay_state_unseen: true,
            expects_challenge_unconsumed: true,
            contract_only: true,
            requests_replay_store_mutation: false,
            requests_challenge_consumption_execution: false,
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
    fn phase11a_label_and_posture_are_locked() {
        let posture = native_replay_challenge_consumption_contract_posture();

        assert_eq!(
            NATIVE_PASSPORT_PHASE11A_LABEL,
            "NATIVE_PASSPORT_PHASE11A_REPLAY_AND_CHALLENGE_CONSUMPTION_CONTRACT"
        );
        assert_eq!(posture.phase_label, NATIVE_PASSPORT_PHASE11A_LABEL);
        assert!(posture.replay_challenge_consumption_contract_added);
        assert!(posture.replay_key_binding_added);
        assert!(posture.challenge_consumption_review_added);
        assert!(!posture.replay_store_mutation_added);
        assert!(!posture.challenge_consumption_execution_added);
        assert!(!posture.capability_issuance_added);
        assert!(!posture.runtime_io_added);
        assert!(!posture.storage_mutation_added);
        assert!(!posture.wallet_or_ledger_mutation_added);
        assert!(!posture.native_secret_implementation_added);

        for forbidden in [
            "replay_store_mutation",
            "challenge_consumption_execution",
            "capability_consumption",
            "capability_issuance",
            "runtime_io",
            "storage_mutation",
            "wallet_spend",
            "ledger_mutation",
        ] {
            assert!(posture.forbidden_authority_flags.contains(&forbidden));
            assert!(PHASE11A_FORBIDDEN_CONSUMPTION_AUTHORITY_FLAGS.contains(&forbidden));
        }
    }

    #[test]
    fn phase11a_surfaces_extend_phase10b_without_back_mutating_it() {
        assert_eq!(
            PHASE11A_ENABLED_SURFACES.len(),
            PHASE10B_ENABLED_SURFACES.len() + 1
        );
        assert_eq!(
            &PHASE11A_ENABLED_SURFACES[..PHASE10B_ENABLED_SURFACES.len()],
            PHASE10B_ENABLED_SURFACES
        );
        assert_eq!(
            PHASE11A_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::ReplayChallengeConsumptionContractDto)
        );
        assert_eq!(
            PHASE10B_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::ProofVerificationAdapterDto)
        );
    }

    #[test]
    fn phase11a_reviews_root_and_request_consumption_contracts_without_mutation() {
        for envelope in [root_envelope(), request_envelope()] {
            let decision = review_native_replay_challenge_consumption_contract(
                &envelope,
                consumption_draft(&envelope),
            )
            .expect("accepted verification envelope should review for consumption contract");

            assert_eq!(
                decision.contract_domain,
                PHASE11A_REPLAY_CHALLENGE_CONSUMPTION_CONTRACT_DOMAIN
            );
            assert_eq!(decision.operation_kind, envelope.operation_kind);
            assert_eq!(decision.reviewed_authority, envelope.reviewed_authority);
            assert_eq!(decision.challenge_id, envelope.challenge_id);
            assert_eq!(decision.transcript_hash, envelope.transcript_hash);
            assert_eq!(decision.signed_payload_hex, envelope.signed_payload_hex);
            assert_eq!(
                decision.verification_evidence_label,
                envelope.evidence_label
            );
            assert_eq!(
                decision.verifier_public_key_label,
                envelope.verifier_public_key_label
            );
            assert!(decision.replay_state_reviewed_unseen);
            assert!(decision.challenge_reviewed_unconsumed);
            assert!(!decision.replay_store_mutated);
            assert!(!decision.challenge_consumed);
            assert!(!decision.capability_state_changed);
            assert!(!decision.runtime_io_performed);
            assert!(!decision.wallet_or_ledger_mutated);
            assert!(decision.contract_only);
        }
    }

    #[test]
    fn phase11a_rejects_replay_challenge_envelope_and_binding_drift() {
        let mut rejected_envelope = request_envelope();
        rejected_envelope.signed_payload_accepted = false;
        assert_eq!(
            review_native_replay_challenge_consumption_contract(
                &rejected_envelope,
                consumption_draft(&rejected_envelope),
            ),
            Err(NativeReplayChallengeConsumptionContractReviewError::VerificationEnvelopeNotAccepted)
        );

        let mut mutated_envelope = request_envelope();
        mutated_envelope.replay_state_mutated = true;
        assert_eq!(
            review_native_replay_challenge_consumption_contract(
                &mutated_envelope,
                consumption_draft(&mutated_envelope),
            ),
            Err(
                NativeReplayChallengeConsumptionContractReviewError::VerificationEnvelopeAlreadyMutatedState
            )
        );

        let envelope = request_envelope();

        let mut wrong_challenge = consumption_draft(&envelope);
        wrong_challenge.challenge_id = other_challenge_id();
        assert_eq!(
            review_native_replay_challenge_consumption_contract(&envelope, wrong_challenge),
            Err(NativeReplayChallengeConsumptionContractReviewError::ChallengeIdMismatch)
        );

        let mut wrong_payload = consumption_draft(&envelope);
        wrong_payload.signed_payload_hex = signed_payload("cc");
        assert_eq!(
            review_native_replay_challenge_consumption_contract(&envelope, wrong_payload),
            Err(NativeReplayChallengeConsumptionContractReviewError::SignedPayloadMismatch)
        );

        let mut replay_seen = consumption_draft(&envelope);
        replay_seen.expects_replay_state_unseen = false;
        assert_eq!(
            review_native_replay_challenge_consumption_contract(&envelope, replay_seen),
            Err(NativeReplayChallengeConsumptionContractReviewError::ReplayStateAlreadySeen)
        );

        let mut consumed = consumption_draft(&envelope);
        consumed.expects_challenge_unconsumed = false;
        assert_eq!(
            review_native_replay_challenge_consumption_contract(&envelope, consumed),
            Err(NativeReplayChallengeConsumptionContractReviewError::ChallengeAlreadyConsumed)
        );

        let mut early = consumption_draft(&envelope);
        early.consumption_requested_at_ms = envelope.verification_requested_at_ms - 1;
        assert_eq!(
            review_native_replay_challenge_consumption_contract(&envelope, early),
            Err(NativeReplayChallengeConsumptionContractReviewError::InvalidConsumptionTime)
        );
    }

    #[test]
    fn phase11a_rejects_all_unsafe_contract_authority_flags() {
        let envelope = request_envelope();

        let mutators: &[fn(&mut NativeReplayChallengeConsumptionContractDraftV1)] = &[
            |draft| draft.requests_replay_store_mutation = true,
            |draft| draft.requests_challenge_consumption_execution = true,
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
            let mut draft = consumption_draft(&envelope);
            mutate(&mut draft);
            assert_eq!(
                review_native_replay_challenge_consumption_contract(&envelope, draft),
                Err(NativeReplayChallengeConsumptionContractReviewError::UnsafeConsumptionAuthorityFlag)
            );
        }
    }

    #[test]
    fn phase11a_source_remains_contract_only_without_store_challenge_capability_io_or_mutation() {
        let source = fs::read_to_string(repo_file("src/native/replay_consumption.rs"))
            .expect("replay consumption source");
        let native_mod =
            fs::read_to_string(repo_file("src/native/mod.rs")).expect("native module source");

        assert!(source.contains("NativeReplayChallengeConsumptionContractDraftV1"));
        assert!(source.contains("NativeReplayChallengeConsumptionContractDecisionV1"));
        assert!(source.contains("review_native_replay_challenge_consumption_contract"));
        assert!(native_mod.contains("ReplayChallengeConsumptionContractDto"));
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
            "refresh_capability(",
            "revoke_capability(",
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
                "Phase 11A replay/challenge consumption source must not gain runtime pattern {forbidden_runtime_pattern}"
            );
        }
    }
}
