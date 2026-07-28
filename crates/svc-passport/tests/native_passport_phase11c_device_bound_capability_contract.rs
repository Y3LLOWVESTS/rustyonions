#[cfg(not(feature = "native-passport"))]
#[test]
fn phase11c_device_bound_capability_contract_is_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not compile native device-bound capability contract surfaces"
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use std::{fs, path::PathBuf};

    use svc_passport::native::{
        native_device_bound_capability_contract_posture,
        review_native_device_bound_capability_contract, B3DigestHex, ChallengeIdV1, DeviceIdV1,
        NativeDeviceBoundCapabilityContractDraftV1, NativeDeviceBoundCapabilityContractReviewError,
        NativePassportProofAuthority, NativePassportProofKind, NativePassportScope,
        NativePassportSurface, NativeProofSignedPayloadHex, NativeProofSigningOperationKind,
        NativeReplayChallengeConsumptionAdapterEnvelopeV1, PassportIdV1,
        NATIVE_PASSPORT_PHASE11C_LABEL, PHASE11B_ENABLED_SURFACES,
        PHASE11B_REPLAY_CHALLENGE_CONSUMPTION_ADAPTER_DOMAIN,
        PHASE11B_REPLAY_CHALLENGE_CONSUMPTION_ADAPTER_VERSION,
        PHASE11C_DEVICE_BOUND_CAPABILITY_CONTRACT_DOMAIN,
        PHASE11C_DEVICE_BOUND_CAPABILITY_CONTRACT_VERSION, PHASE11C_ENABLED_SURFACES,
        PHASE11C_FORBIDDEN_CAPABILITY_CONTRACT_AUTHORITY_FLAGS, PHASE11C_MAX_CAPABILITY_TTL_MS,
        PHASE11C_MAX_CLOCK_SKEW_MS, PHASE8B_DEVICE_PROOF_SIGNATURE_ALGORITHM,
        PHASE8B_PROOF_CONTRACT_DOMAIN, PHASE8B_PROOF_CONTRACT_VERSION,
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
        NativeProofSignedPayloadHex::parse("phase11c_signed_payload_hex", hex_pair.repeat(64))
            .expect("signed payload")
    }

    fn request_envelope() -> NativeReplayChallengeConsumptionAdapterEnvelopeV1 {
        NativeReplayChallengeConsumptionAdapterEnvelopeV1 {
            contract_domain: PHASE11B_REPLAY_CHALLENGE_CONSUMPTION_ADAPTER_DOMAIN,
            contract_version: PHASE11B_REPLAY_CHALLENGE_CONSUMPTION_ADAPTER_VERSION,
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
            transcript_hash: digest("request_transcript_hash", HEX_D),
            signer_public_key_label: "public-key:v1:phase11c-device",
            expected_signature_algorithm: PHASE8B_DEVICE_PROOF_SIGNATURE_ALGORITHM,
            signed_payload_hex: signed_payload("ab"),
            verification_evidence_label: "phase11c-verification-evidence",
            verifier_public_key_label: "public-key:v1:phase11c-verifier",
            replay_key_hash: digest("replay_key_hash", HEX_E),
            idempotency_key_hash: digest("idempotency_key_hash", HEX_D),
            consumption_requested_at_ms: 1_045_002,
            state_adapter_label: "phase11c-state-adapter",
            replay_recorded_by_adapter: true,
            challenge_consumed_by_adapter: true,
            capability_state_changed: false,
            runtime_io_performed: false,
            wallet_or_ledger_mutated: false,
        }
    }

    fn root_envelope() -> NativeReplayChallengeConsumptionAdapterEnvelopeV1 {
        NativeReplayChallengeConsumptionAdapterEnvelopeV1 {
            operation_kind: NativeProofSigningOperationKind::RootProof,
            reviewed_authority: NativePassportProofAuthority::RootAuthority,
            request_contract_domain: None,
            request_contract_version: None,
            proof_kind: NativePassportProofKind::RootRegistration,
            expected_signature_algorithm: PHASE8B_DEVICE_PROOF_SIGNATURE_ALGORITHM,
            ..request_envelope()
        }
    }

    fn capability_draft(
        envelope: &NativeReplayChallengeConsumptionAdapterEnvelopeV1,
    ) -> NativeDeviceBoundCapabilityContractDraftV1 {
        NativeDeviceBoundCapabilityContractDraftV1 {
            contract_domain: PHASE11C_DEVICE_BOUND_CAPABILITY_CONTRACT_DOMAIN,
            contract_version: PHASE11C_DEVICE_BOUND_CAPABILITY_CONTRACT_VERSION,
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
            verification_evidence_label: envelope.verification_evidence_label,
            verifier_public_key_label: envelope.verifier_public_key_label,
            state_adapter_label: envelope.state_adapter_label,
            replay_key_hash: envelope.replay_key_hash.clone(),
            idempotency_key_hash: envelope.idempotency_key_hash.clone(),
            capability_id_hash: digest("capability_id_hash", HEX_A),
            capability_binding_hash: digest("capability_binding_hash", HEX_B),
            capability_scope_hash: digest("capability_scope_hash", HEX_C),
            capability_scopes: vec![
                NativePassportScope::IdentityRead,
                NativePassportScope::CatalogRead,
            ],
            capability_issued_at_ms: envelope.consumption_requested_at_ms + 1,
            capability_expires_at_ms: envelope.consumption_requested_at_ms + 60_000,
            max_clock_skew_ms: 10_000,
            device_bound: true,
            expects_replay_recorded_by_adapter: true,
            expects_challenge_consumed_by_adapter: true,
            contract_only: true,
            requests_capability_issuance_execution: false,
            requests_capability_consumption_execution: false,
            requests_capability_lifecycle_runtime: false,
            requests_capability_storage_mutation: false,
            requests_secret_key_access: false,
            requests_key_loading: false,
            requests_key_derivation_runtime: false,
            requests_vault_unlock: false,
            includes_vault_runtime: false,
            requests_platform_sealer_unseal: false,
            requests_runtime_io: false,
            adds_routes: false,
            requests_storage_mutation_inside_native: false,
            requests_wallet_or_ledger_mutation: false,
        }
    }

    #[test]
    fn phase11c_label_and_posture_are_locked() {
        let posture = native_device_bound_capability_contract_posture();

        assert_eq!(
            NATIVE_PASSPORT_PHASE11C_LABEL,
            "NATIVE_PASSPORT_PHASE11C_DEVICE_BOUND_CAPABILITY_CONTRACT"
        );
        assert_eq!(posture.phase_label, NATIVE_PASSPORT_PHASE11C_LABEL);
        assert!(posture.device_bound_capability_contract_added);
        assert!(posture.capability_scope_binding_added);
        assert!(posture.capability_id_binding_added);
        assert!(!posture.capability_issuance_execution_added);
        assert!(!posture.capability_consumption_execution_added);
        assert!(!posture.capability_storage_mutation_added);
        assert!(!posture.runtime_io_added);
        assert!(!posture.storage_mutation_inside_native_added);
        assert!(!posture.wallet_or_ledger_mutation_added);
        assert!(!posture.native_secret_implementation_added);
        assert_eq!(PHASE11C_MAX_CAPABILITY_TTL_MS, 3_600_000);
        assert_eq!(PHASE11C_MAX_CLOCK_SKEW_MS, 30_000);

        for forbidden in [
            "capability_issuance_execution",
            "capability_consumption_execution",
            "capability_storage_mutation",
            "runtime_io",
            "storage_mutation_inside_svc_passport_native",
            "wallet_spend",
            "ledger_mutation",
        ] {
            assert!(posture.forbidden_authority_flags.contains(&forbidden));
            assert!(PHASE11C_FORBIDDEN_CAPABILITY_CONTRACT_AUTHORITY_FLAGS.contains(&forbidden));
        }
    }

    #[test]
    fn phase11c_surfaces_extend_phase11b_without_back_mutating_it() {
        assert_eq!(
            PHASE11C_ENABLED_SURFACES.len(),
            PHASE11B_ENABLED_SURFACES.len() + 1
        );
        assert_eq!(
            &PHASE11C_ENABLED_SURFACES[..PHASE11B_ENABLED_SURFACES.len()],
            PHASE11B_ENABLED_SURFACES
        );
        assert_eq!(
            PHASE11C_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::DeviceBoundCapabilityContractDto)
        );
        assert_eq!(
            PHASE11B_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::ReplayChallengeConsumptionAdapterDto)
        );
    }

    #[test]
    fn phase11c_reviews_device_bound_capability_contract_without_issuance_execution() {
        let envelope = request_envelope();
        let decision =
            review_native_device_bound_capability_contract(&envelope, capability_draft(&envelope))
                .expect("request envelope should review for device-bound capability contract");

        assert_eq!(
            decision.contract_domain,
            PHASE11C_DEVICE_BOUND_CAPABILITY_CONTRACT_DOMAIN
        );
        assert_eq!(
            decision.operation_kind,
            NativeProofSigningOperationKind::RequestProof
        );
        assert_eq!(
            decision.reviewed_authority,
            NativePassportProofAuthority::DeviceAuthority
        );
        assert_eq!(decision.challenge_id, envelope.challenge_id);
        assert_eq!(decision.device_id, envelope.device_id);
        assert_eq!(decision.transcript_hash, envelope.transcript_hash);
        assert_eq!(decision.replay_key_hash, envelope.replay_key_hash);
        assert_eq!(decision.idempotency_key_hash, envelope.idempotency_key_hash);
        assert_eq!(decision.capability_scopes.len(), 2);
        assert!(decision.device_bound);
        assert!(decision.replay_recorded_reviewed);
        assert!(decision.challenge_consumed_reviewed);
        assert!(!decision.capability_issued);
        assert!(!decision.capability_state_changed);
        assert!(!decision.runtime_io_performed);
        assert!(!decision.wallet_or_ledger_mutated);
        assert!(decision.contract_only);
    }

    #[test]
    fn phase11c_rejects_root_missing_request_wrong_challenge_and_unapplied_state() {
        let root = root_envelope();
        assert_eq!(
            review_native_device_bound_capability_contract(&root, capability_draft(&root)),
            Err(NativeDeviceBoundCapabilityContractReviewError::UnsupportedCapabilityOperation)
        );

        let mut unapplied = request_envelope();
        unapplied.challenge_consumed_by_adapter = false;
        assert_eq!(
            review_native_device_bound_capability_contract(
                &unapplied,
                capability_draft(&unapplied),
            ),
            Err(NativeDeviceBoundCapabilityContractReviewError::ConsumptionEnvelopeStateNotApplied)
        );

        let envelope = request_envelope();

        let mut wrong_challenge = capability_draft(&envelope);
        wrong_challenge.challenge_id = other_challenge_id();
        assert_eq!(
            review_native_device_bound_capability_contract(&envelope, wrong_challenge),
            Err(NativeDeviceBoundCapabilityContractReviewError::ChallengeIdMismatch)
        );

        let mut missing_device = capability_draft(&envelope);
        missing_device.device_id = None;
        assert_eq!(
            review_native_device_bound_capability_contract(&envelope, missing_device),
            Err(NativeDeviceBoundCapabilityContractReviewError::DeviceBindingMissing)
        );

        let mut missing_request = capability_draft(&envelope);
        missing_request.request_contract_domain = None;
        assert_eq!(
            review_native_device_bound_capability_contract(&envelope, missing_request),
            Err(NativeDeviceBoundCapabilityContractReviewError::RequestProofContractDomainMissing)
        );
    }

    #[test]
    fn phase11c_rejects_scope_time_device_and_unsafe_authority_drift() {
        let envelope = request_envelope();

        let mut duplicate_scope = capability_draft(&envelope);
        duplicate_scope.capability_scopes = vec![
            NativePassportScope::IdentityRead,
            NativePassportScope::IdentityRead,
        ];
        assert_eq!(
            review_native_device_bound_capability_contract(&envelope, duplicate_scope),
            Err(NativeDeviceBoundCapabilityContractReviewError::DuplicateCapabilityScopes)
        );

        let mut not_device_bound = capability_draft(&envelope);
        not_device_bound.device_bound = false;
        assert_eq!(
            review_native_device_bound_capability_contract(&envelope, not_device_bound),
            Err(NativeDeviceBoundCapabilityContractReviewError::CapabilityNotDeviceBound)
        );

        let mut too_long = capability_draft(&envelope);
        too_long.capability_expires_at_ms =
            too_long.capability_issued_at_ms + PHASE11C_MAX_CAPABILITY_TTL_MS + 1;
        assert_eq!(
            review_native_device_bound_capability_contract(&envelope, too_long),
            Err(NativeDeviceBoundCapabilityContractReviewError::CapabilityTtlTooLong)
        );

        let mut clock_skew = capability_draft(&envelope);
        clock_skew.max_clock_skew_ms = PHASE11C_MAX_CLOCK_SKEW_MS + 1;
        assert_eq!(
            review_native_device_bound_capability_contract(&envelope, clock_skew),
            Err(NativeDeviceBoundCapabilityContractReviewError::ClockSkewTooLarge)
        );

        let mut unsafe_draft = capability_draft(&envelope);
        unsafe_draft.requests_capability_issuance_execution = true;
        assert_eq!(
            review_native_device_bound_capability_contract(&envelope, unsafe_draft),
            Err(NativeDeviceBoundCapabilityContractReviewError::UnsafeCapabilityAuthorityFlag)
        );
    }

    #[test]
    fn phase11c_source_remains_contract_only_without_capability_storage_routes_io_wallet_ledger_or_secrets(
    ) {
        let source = fs::read_to_string(repo_file("src/native/capability_contract.rs"))
            .expect("capability contract source");
        let native_mod =
            fs::read_to_string(repo_file("src/native/mod.rs")).expect("native module source");

        assert!(source.contains("NativeDeviceBoundCapabilityContractDraftV1"));
        assert!(source.contains("NativeDeviceBoundCapabilityContractDecisionV1"));
        assert!(source.contains("review_native_device_bound_capability_contract"));
        assert!(native_mod.contains("DeviceBoundCapabilityContractDto"));
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
                "Phase 11C capability contract source must not gain runtime pattern {forbidden_runtime_pattern}"
            );
        }
    }
}
