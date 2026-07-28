#[cfg(not(feature = "native-passport"))]
#[test]
fn phase13_local_status_and_inspection_is_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not compile native local status inspection surfaces"
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use std::{fs, path::PathBuf};

    use svc_passport::native::{
        native_local_status_inspection_posture, review_native_local_status_inspection,
        NativeLocalStatusInspectionDraftV1, NativeLocalStatusInspectionReviewError,
        NativePassportSurface, NATIVE_PASSPORT_PHASE13_LABEL, PHASE12C_ENABLED_SURFACES,
        PHASE13_ENABLED_SURFACES, PHASE13_FORBIDDEN_LOCAL_STATUS_AUTHORITY_FLAGS,
        PHASE13_LOCAL_STATUS_INSPECTION_DOMAIN, PHASE13_LOCAL_STATUS_INSPECTION_VERSION,
        PHASE13_REDACTED_VALUE,
    };

    fn repo_file(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
    }

    fn safe_draft() -> NativeLocalStatusInspectionDraftV1 {
        NativeLocalStatusInspectionDraftV1 {
            contract_domain: PHASE13_LOCAL_STATUS_INSPECTION_DOMAIN,
            contract_version: PHASE13_LOCAL_STATUS_INSPECTION_VERSION,
            requester_label: "local-cli:passport-status",
            local_node_label: "local-node:dev",
            status_requested_at_ms: 1_300_000,
            enabled_surface_count: PHASE12C_ENABLED_SURFACES.len(),
            expected_enabled_surface_count: PHASE12C_ENABLED_SURFACES.len(),
            latest_completed_phase_label:
                "NATIVE_PASSPORT_PHASE12D_USERNAME_PRIVATE_BETA_ACCEPTANCE",
            native_feature_enabled: true,
            local_only: true,
            inspection_only: true,
            redaction_required: true,
            exposes_passport_identifier: false,
            exposes_device_identifier: false,
            exposes_username_handle: false,
            exposes_proof_material: false,
            exposes_capability_material: false,
            exposes_vault_material: false,
            requests_secret_key_access: false,
            requests_key_loading: false,
            requests_key_derivation_runtime: false,
            requests_vault_unlock: false,
            includes_vault_runtime: false,
            requests_platform_sealer_unseal: false,
            requests_runtime_io: false,
            adds_routes: false,
            requests_storage_mutation_inside_native: false,
            treats_index_projection_as_authority: false,
            requests_wallet_or_ledger_mutation: false,
        }
    }

    #[test]
    fn phase13_label_and_posture_are_locked() {
        let posture = native_local_status_inspection_posture();

        assert_eq!(
            NATIVE_PASSPORT_PHASE13_LABEL,
            "NATIVE_PASSPORT_PHASE13_LOCAL_STATUS_AND_INSPECTION"
        );
        assert_eq!(posture.phase_label, NATIVE_PASSPORT_PHASE13_LABEL);
        assert!(posture.local_status_inspection_added);
        assert!(posture.redacted_snapshot_added);
        assert!(posture.local_only_review_added);
        assert!(!posture.route_added);
        assert!(!posture.runtime_io_added);
        assert!(!posture.storage_mutation_inside_native_added);
        assert!(!posture.index_projection_authority_added);
        assert!(!posture.wallet_or_ledger_mutation_added);
        assert!(!posture.secret_key_access_added);
        assert!(!posture.key_loading_added);
        assert!(!posture.vault_unlock_added);
        assert!(!posture.vault_runtime_added);
        assert!(!posture.native_secret_implementation_added);

        for forbidden in [
            "passport_identifier_exposure",
            "device_identifier_exposure",
            "username_handle_exposure",
            "proof_material_exposure",
            "capability_material_exposure",
            "vault_material_exposure",
            "route_added",
            "storage_mutation_inside_svc_passport_native",
            "index_projection_authority",
            "wallet_spend",
            "ledger_mutation",
        ] {
            assert!(posture.forbidden_authority_flags.contains(&forbidden));
            assert!(PHASE13_FORBIDDEN_LOCAL_STATUS_AUTHORITY_FLAGS.contains(&forbidden));
        }
    }

    #[test]
    fn phase13_surfaces_extend_phase12c_without_back_mutating_it() {
        assert_eq!(
            PHASE13_ENABLED_SURFACES.len(),
            PHASE12C_ENABLED_SURFACES.len() + 1
        );
        assert_eq!(
            &PHASE13_ENABLED_SURFACES[..PHASE12C_ENABLED_SURFACES.len()],
            PHASE12C_ENABLED_SURFACES
        );
        assert_eq!(
            PHASE13_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::LocalStatusInspectionDto)
        );
        assert_eq!(
            PHASE12C_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::UsernamePrivateBetaIndexProjectionDto)
        );
    }

    #[test]
    fn phase13_reviews_redacted_local_status_snapshot_without_runtime_authority() {
        let snapshot =
            review_native_local_status_inspection(safe_draft()).expect("safe local status review");

        assert_eq!(
            snapshot.contract_domain,
            PHASE13_LOCAL_STATUS_INSPECTION_DOMAIN
        );
        assert_eq!(snapshot.phase_label, NATIVE_PASSPORT_PHASE13_LABEL);
        assert_eq!(snapshot.requester_label, "local-cli:passport-status");
        assert_eq!(
            snapshot.enabled_surface_count,
            PHASE12C_ENABLED_SURFACES.len()
        );
        assert_eq!(
            snapshot.redacted_passport_identifier,
            PHASE13_REDACTED_VALUE
        );
        assert_eq!(snapshot.redacted_device_identifier, PHASE13_REDACTED_VALUE);
        assert_eq!(snapshot.redacted_username_handle, PHASE13_REDACTED_VALUE);
        assert_eq!(snapshot.redacted_proof_material, PHASE13_REDACTED_VALUE);
        assert_eq!(
            snapshot.redacted_capability_material,
            PHASE13_REDACTED_VALUE
        );
        assert_eq!(snapshot.redacted_vault_material, PHASE13_REDACTED_VALUE);
        assert!(snapshot.native_feature_enabled);
        assert!(snapshot.local_only);
        assert!(snapshot.inspection_only);
        assert!(snapshot.contract_only);
        assert!(!snapshot.routes_added);
        assert!(!snapshot.runtime_io_performed);
        assert!(!snapshot.storage_mutated_inside_native);
        assert!(!snapshot.index_projection_authoritative);
        assert!(!snapshot.wallet_or_ledger_mutated);
        assert!(!snapshot.secret_material_exposed);
    }

    #[test]
    fn phase13_rejects_identifier_material_and_nonlocal_inspection_drift() {
        let mut passport_identifier = safe_draft();
        passport_identifier.exposes_passport_identifier = true;
        assert_eq!(
            review_native_local_status_inspection(passport_identifier),
            Err(NativeLocalStatusInspectionReviewError::IdentifierExposure)
        );

        let mut device_identifier = safe_draft();
        device_identifier.exposes_device_identifier = true;
        assert_eq!(
            review_native_local_status_inspection(device_identifier),
            Err(NativeLocalStatusInspectionReviewError::IdentifierExposure)
        );

        let mut username_handle = safe_draft();
        username_handle.exposes_username_handle = true;
        assert_eq!(
            review_native_local_status_inspection(username_handle),
            Err(NativeLocalStatusInspectionReviewError::UsernameHandleExposure)
        );

        let mut proof_material = safe_draft();
        proof_material.exposes_proof_material = true;
        assert_eq!(
            review_native_local_status_inspection(proof_material),
            Err(NativeLocalStatusInspectionReviewError::ProofMaterialExposure)
        );

        let mut capability_material = safe_draft();
        capability_material.exposes_capability_material = true;
        assert_eq!(
            review_native_local_status_inspection(capability_material),
            Err(NativeLocalStatusInspectionReviewError::CapabilityMaterialExposure)
        );

        let mut vault_material = safe_draft();
        vault_material.exposes_vault_material = true;
        assert_eq!(
            review_native_local_status_inspection(vault_material),
            Err(NativeLocalStatusInspectionReviewError::VaultMaterialExposure)
        );

        let mut remote = safe_draft();
        remote.local_only = false;
        assert_eq!(
            review_native_local_status_inspection(remote),
            Err(NativeLocalStatusInspectionReviewError::NotLocalOnly)
        );

        let mut surface_mismatch = safe_draft();
        surface_mismatch.enabled_surface_count += 1;
        assert_eq!(
            review_native_local_status_inspection(surface_mismatch),
            Err(NativeLocalStatusInspectionReviewError::SurfaceCountMismatch)
        );
    }

    #[test]
    fn phase13_rejects_all_unsafe_local_status_authority_flags() {
        let mutators: &[fn(&mut NativeLocalStatusInspectionDraftV1)] = &[
            |draft| draft.requests_secret_key_access = true,
            |draft| draft.requests_key_loading = true,
            |draft| draft.requests_key_derivation_runtime = true,
            |draft| draft.requests_vault_unlock = true,
            |draft| draft.includes_vault_runtime = true,
            |draft| draft.requests_platform_sealer_unseal = true,
            |draft| draft.requests_runtime_io = true,
            |draft| draft.adds_routes = true,
            |draft| draft.requests_storage_mutation_inside_native = true,
            |draft| draft.treats_index_projection_as_authority = true,
            |draft| draft.requests_wallet_or_ledger_mutation = true,
        ];

        for mutate in mutators {
            let mut draft = safe_draft();
            mutate(&mut draft);
            assert_eq!(
                review_native_local_status_inspection(draft),
                Err(NativeLocalStatusInspectionReviewError::UnsafeLocalStatusAuthorityFlag)
            );
        }
    }

    #[test]
    fn phase13_sources_preserve_local_redacted_inspection_without_routes_storage_wallet_ledger_or_secrets(
    ) {
        let source = fs::read_to_string(repo_file("src/native/local_status_inspection.rs"))
            .expect("local status inspection source");
        let redacted_status_source =
            fs::read_to_string(repo_file("src/native/status.rs")).expect("status source");
        let native_mod =
            fs::read_to_string(repo_file("src/native/mod.rs")).expect("native module source");

        assert!(source.contains("NativeLocalStatusInspectionDraftV1"));
        assert!(source.contains("review_native_local_status_inspection"));
        assert!(source.contains("PHASE13_REDACTED_VALUE"));
        assert!(redacted_status_source.contains("NATIVE_PASSPORT_REDACTED_VALUE"));
        assert!(native_mod.contains("LocalStatusInspectionDto"));
        assert!(!source.contains("serde_json"));

        for forbidden_runtime_pattern in [
            "Router::",
            ".route(",
            "lookup_projection.write(",
            "index.write(",
            "username_store.write(",
            "storage.write(",
            "persist_username(",
            "finalize_username(",
            "mutate_index_projection(",
            "load_secret",
            "load_key",
            "secret_key_bytes",
            "raw_passport_id:",
            "raw_device_id:",
            "raw_username:",
            "proof_payload:",
            "capability_token:",
            "raw_capability:",
            "vault_secret:",
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
        ] {
            assert!(
                !source.contains(forbidden_runtime_pattern),
                "Phase 13 local status inspection source must not gain runtime pattern {forbidden_runtime_pattern}"
            );
        }
    }
}
