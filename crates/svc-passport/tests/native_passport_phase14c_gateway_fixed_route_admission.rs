#[cfg(not(feature = "native-passport"))]
#[test]
fn phase14c_gateway_fixed_route_admission_is_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not compile native gateway fixed-route admission surfaces"
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use std::{fs, path::PathBuf};

    use svc_passport::native::{
        native_gateway_fixed_route_admission_posture, review_native_gateway_fixed_route_admission,
        NativeGatewayFixedRouteAdmissionDraftV1, NativeGatewayFixedRouteAdmissionReviewError,
        NativeGatewayOmnigateRouteKind, NativeGatewayOmnigateRouteMethod,
        NativeGatewayOmnigateRouteSpecV1, NativePassportSurface, NATIVE_PASSPORT_PHASE14C_LABEL,
        PHASE14A_GATEWAY_OMNIGATE_ROUTE_CATALOG, PHASE14A_GATEWAY_OMNIGATE_ROUTE_CONTRACT_DOMAIN,
        PHASE14A_GATEWAY_OMNIGATE_ROUTE_CONTRACT_VERSION, PHASE14B_ENABLED_SURFACES,
        PHASE14B_OMNIGATE_FIXED_ROUTE_ADMISSION_DOMAIN,
        PHASE14B_OMNIGATE_FIXED_ROUTE_ADMISSION_VERSION, PHASE14C_ENABLED_SURFACES,
        PHASE14C_FORBIDDEN_GATEWAY_ADMISSION_AUTHORITY_FLAGS,
        PHASE14C_GATEWAY_FIXED_ROUTE_ADMISSION_DOMAIN,
        PHASE14C_GATEWAY_FIXED_ROUTE_ADMISSION_VERSION,
    };

    fn repo_file(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
    }

    fn spec_for(
        route_kind: NativeGatewayOmnigateRouteKind,
    ) -> &'static NativeGatewayOmnigateRouteSpecV1 {
        PHASE14A_GATEWAY_OMNIGATE_ROUTE_CATALOG
            .iter()
            .find(|spec| spec.kind == route_kind)
            .expect("fixed route kind")
    }

    fn admission_draft(
        route_kind: NativeGatewayOmnigateRouteKind,
    ) -> NativeGatewayFixedRouteAdmissionDraftV1 {
        let spec = spec_for(route_kind);
        NativeGatewayFixedRouteAdmissionDraftV1 {
            contract_domain: PHASE14C_GATEWAY_FIXED_ROUTE_ADMISSION_DOMAIN,
            contract_version: PHASE14C_GATEWAY_FIXED_ROUTE_ADMISSION_VERSION,
            route_kind,
            method: spec.method,
            public_gateway_path: spec.public_gateway_path,
            omnigate_path: spec.omnigate_path,
            downstream_passport_path: spec.downstream_passport_path,
            request_body_len_bytes: if spec.body_cap_bytes == 0 { 0 } else { 512 },
            deadline_ms: spec.deadline_ms,
            correlation_id: "corr-phase14c",
            device_bound_capability_present: spec.requires_device_bound_capability,
            fresh_proof_present: spec.requires_fresh_proof,
            mutating_scope_requested: false,
            gateway_proxy_only: true,
            omnigate_orchestration_only: true,
            svc_passport_authority: true,
            fixed_gateway_route_catalog_reviewed: true,
            omnigate_fixed_route_admission_available: true,
            typed_redacted_problem_available: true,
            request_body_logging_disabled: true,
            response_body_logging_disabled: true,
            caller_controlled_downstream_url: false,
            requests_live_gateway_mount: false,
            requests_live_omnigate_mount: false,
            requests_storage_mutation_inside_gateway: false,
            requests_storage_mutation_inside_omnigate: false,
            requests_secret_key_access: false,
            requests_key_loading: false,
            requests_key_derivation_runtime: false,
            requests_vault_unlock: false,
            includes_vault_runtime: false,
            requests_platform_sealer_unseal: false,
            requests_wallet_or_ledger_mutation: false,
        }
    }

    #[test]
    fn phase14c_label_and_posture_are_locked() {
        let posture = native_gateway_fixed_route_admission_posture();

        assert_eq!(
            NATIVE_PASSPORT_PHASE14C_LABEL,
            "NATIVE_PASSPORT_PHASE14C_GATEWAY_FIXED_ROUTE_ADMISSION"
        );
        assert_eq!(posture.phase_label, NATIVE_PASSPORT_PHASE14C_LABEL);
        assert!(posture.gateway_fixed_route_admission_added);
        assert!(posture.fixed_gateway_route_catalog_reused);
        assert!(posture.omnigate_fixed_route_admission_reused);
        assert!(posture.body_cap_review_added);
        assert!(posture.deadline_review_added);
        assert!(posture.correlation_id_review_added);
        assert!(posture.capability_requirement_review_added);
        assert!(posture.fresh_proof_requirement_review_added);
        assert!(posture.typed_redacted_problem_review_added);
        assert!(!posture.live_gateway_route_mount_added);
        assert!(!posture.live_omnigate_route_mount_added);
        assert!(!posture.gateway_identity_authority_added);
        assert!(!posture.omnigate_identity_authority_added);
        assert!(!posture.storage_mutation_inside_gateway_or_omnigate_added);
        assert!(!posture.wallet_or_ledger_mutation_added);
        assert!(!posture.secret_key_access_added);
        assert!(!posture.native_secret_implementation_added);

        for forbidden in [
            "dynamic_public_route_admission",
            "caller_controlled_downstream_url",
            "gateway_identity_authority",
            "omnigate_identity_authority",
            "request_body_logging",
            "response_body_logging",
            "secret_echo",
            "wallet_spend",
            "ledger_mutation",
        ] {
            assert!(posture.forbidden_authority_flags.contains(&forbidden));
            assert!(PHASE14C_FORBIDDEN_GATEWAY_ADMISSION_AUTHORITY_FLAGS.contains(&forbidden));
        }
    }

    #[test]
    fn phase14c_surfaces_extend_phase14b_without_back_mutating_it() {
        assert_eq!(
            PHASE14C_ENABLED_SURFACES.len(),
            PHASE14B_ENABLED_SURFACES.len() + 1
        );
        assert_eq!(
            &PHASE14C_ENABLED_SURFACES[..PHASE14B_ENABLED_SURFACES.len()],
            PHASE14B_ENABLED_SURFACES
        );
        assert_eq!(
            PHASE14C_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::GatewayFixedRouteAdmissionDto)
        );
        assert_eq!(
            PHASE14B_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::OmnigateFixedRouteAdmissionDto)
        );
    }

    #[test]
    fn phase14c_accepts_every_fixed_gateway_route_and_delegates_to_omnigate_admission() {
        for spec in PHASE14A_GATEWAY_OMNIGATE_ROUTE_CATALOG {
            let decision = review_native_gateway_fixed_route_admission(admission_draft(spec.kind))
                .expect("fixed gateway route admission should review");

            assert_eq!(
                decision.contract_domain,
                PHASE14C_GATEWAY_FIXED_ROUTE_ADMISSION_DOMAIN
            );
            assert_eq!(
                decision.route_contract_domain,
                PHASE14A_GATEWAY_OMNIGATE_ROUTE_CONTRACT_DOMAIN
            );
            assert_eq!(
                decision.route_contract_version,
                PHASE14A_GATEWAY_OMNIGATE_ROUTE_CONTRACT_VERSION
            );
            assert_eq!(
                decision.omnigate_admission_contract_domain,
                PHASE14B_OMNIGATE_FIXED_ROUTE_ADMISSION_DOMAIN
            );
            assert_eq!(
                decision.omnigate_admission_contract_version,
                PHASE14B_OMNIGATE_FIXED_ROUTE_ADMISSION_VERSION
            );
            assert_eq!(decision.route_kind, spec.kind);
            assert_eq!(decision.method, spec.method);
            assert_eq!(decision.public_gateway_path, spec.public_gateway_path);
            assert_eq!(decision.omnigate_path, spec.omnigate_path);
            assert_eq!(
                decision.downstream_passport_path,
                spec.downstream_passport_path
            );
            assert!(decision.fixed_gateway_route_admitted);
            assert!(decision.omnigate_fixed_route_admission_reviewed);
            assert_eq!(
                decision.device_bound_capability_required,
                spec.requires_device_bound_capability
            );
            assert_eq!(decision.fresh_proof_required, spec.requires_fresh_proof);
            assert!(decision.gateway_proxy_only);
            assert!(decision.omnigate_orchestration_only);
            assert!(decision.svc_passport_authority);
            assert!(decision.typed_redacted_problem_available);
            assert!(decision.request_body_logging_disabled);
            assert!(decision.response_body_logging_disabled);
            assert!(!decision.mutating_scope_allowed);
            assert!(!decision.live_gateway_mount_added);
            assert!(!decision.live_omnigate_mount_added);
            assert!(!decision.storage_mutated_inside_gateway_or_omnigate);
            assert!(!decision.wallet_or_ledger_mutated);
            assert!(!decision.secret_material_exposed);
            assert!(decision.contract_only);
        }
    }

    #[test]
    fn phase14c_rejects_dynamic_public_paths_caps_deadlines_and_missing_guards() {
        let mut wrong_path = admission_draft(NativeGatewayOmnigateRouteKind::Register);
        wrong_path.public_gateway_path = "/identity/passport/not-fixed";
        assert_eq!(
            review_native_gateway_fixed_route_admission(wrong_path),
            Err(NativeGatewayFixedRouteAdmissionReviewError::GatewayPathMismatch)
        );

        let mut wrong_method = admission_draft(NativeGatewayOmnigateRouteKind::ProtectedRead);
        wrong_method.method = NativeGatewayOmnigateRouteMethod::Post;
        assert_eq!(
            review_native_gateway_fixed_route_admission(wrong_method),
            Err(NativeGatewayFixedRouteAdmissionReviewError::MethodMismatch)
        );

        let mut too_large = admission_draft(NativeGatewayOmnigateRouteKind::Register);
        too_large.request_body_len_bytes = 16_385;
        assert_eq!(
            review_native_gateway_fixed_route_admission(too_large),
            Err(NativeGatewayFixedRouteAdmissionReviewError::BodyCapExceeded)
        );

        let mut too_slow = admission_draft(NativeGatewayOmnigateRouteKind::Register);
        too_slow.deadline_ms = 5_001;
        assert_eq!(
            review_native_gateway_fixed_route_admission(too_slow),
            Err(NativeGatewayFixedRouteAdmissionReviewError::DeadlineExceeded)
        );

        let mut no_correlation = admission_draft(NativeGatewayOmnigateRouteKind::Register);
        no_correlation.correlation_id = "";
        assert_eq!(
            review_native_gateway_fixed_route_admission(no_correlation),
            Err(NativeGatewayFixedRouteAdmissionReviewError::MissingCorrelationId)
        );

        let mut no_capability = admission_draft(NativeGatewayOmnigateRouteKind::ProtectedRead);
        no_capability.device_bound_capability_present = false;
        assert_eq!(
            review_native_gateway_fixed_route_admission(no_capability),
            Err(NativeGatewayFixedRouteAdmissionReviewError::MissingRequiredDeviceBoundCapability)
        );

        let mut no_proof = admission_draft(NativeGatewayOmnigateRouteKind::Register);
        no_proof.fresh_proof_present = false;
        assert_eq!(
            review_native_gateway_fixed_route_admission(no_proof),
            Err(NativeGatewayFixedRouteAdmissionReviewError::MissingRequiredFreshProof)
        );
    }

    #[test]
    fn phase14c_rejects_authority_shift_body_logging_live_mounts_and_unsafe_flags() {
        let mut mutating_scope = admission_draft(NativeGatewayOmnigateRouteKind::ProtectedRead);
        mutating_scope.mutating_scope_requested = true;
        assert_eq!(
            review_native_gateway_fixed_route_admission(mutating_scope),
            Err(NativeGatewayFixedRouteAdmissionReviewError::MutatingScopeEscalationRequested)
        );

        let mut gateway_authority = admission_draft(NativeGatewayOmnigateRouteKind::Register);
        gateway_authority.gateway_proxy_only = false;
        assert_eq!(
            review_native_gateway_fixed_route_admission(gateway_authority),
            Err(NativeGatewayFixedRouteAdmissionReviewError::GatewayClaimsIdentityAuthority)
        );

        let mut omnigate_authority = admission_draft(NativeGatewayOmnigateRouteKind::Register);
        omnigate_authority.omnigate_orchestration_only = false;
        assert_eq!(
            review_native_gateway_fixed_route_admission(omnigate_authority),
            Err(NativeGatewayFixedRouteAdmissionReviewError::OmnigateClaimsIdentityAuthority)
        );

        let mut body_logging = admission_draft(NativeGatewayOmnigateRouteKind::Register);
        body_logging.response_body_logging_disabled = false;
        assert_eq!(
            review_native_gateway_fixed_route_admission(body_logging),
            Err(NativeGatewayFixedRouteAdmissionReviewError::RequestOrResponseBodyLoggingEnabled)
        );

        let mut caller_url = admission_draft(NativeGatewayOmnigateRouteKind::Register);
        caller_url.caller_controlled_downstream_url = true;
        assert_eq!(
            review_native_gateway_fixed_route_admission(caller_url),
            Err(NativeGatewayFixedRouteAdmissionReviewError::CallerControlledDownstreamUrl)
        );

        let mut live_mount = admission_draft(NativeGatewayOmnigateRouteKind::Register);
        live_mount.requests_live_gateway_mount = true;
        assert_eq!(
            review_native_gateway_fixed_route_admission(live_mount),
            Err(NativeGatewayFixedRouteAdmissionReviewError::LiveRouteMountRequested)
        );

        let mut no_omnigate_admission = admission_draft(NativeGatewayOmnigateRouteKind::Register);
        no_omnigate_admission.omnigate_fixed_route_admission_available = false;
        assert_eq!(
            review_native_gateway_fixed_route_admission(no_omnigate_admission),
            Err(
                NativeGatewayFixedRouteAdmissionReviewError::OmnigateFixedRouteAdmissionUnavailable
            )
        );

        let mutators: &[fn(&mut NativeGatewayFixedRouteAdmissionDraftV1)] = &[
            |draft| draft.requests_storage_mutation_inside_gateway = true,
            |draft| draft.requests_storage_mutation_inside_omnigate = true,
            |draft| draft.requests_secret_key_access = true,
            |draft| draft.requests_key_loading = true,
            |draft| draft.requests_key_derivation_runtime = true,
            |draft| draft.requests_vault_unlock = true,
            |draft| draft.includes_vault_runtime = true,
            |draft| draft.requests_platform_sealer_unseal = true,
            |draft| draft.requests_wallet_or_ledger_mutation = true,
        ];

        for mutate in mutators {
            let mut draft = admission_draft(NativeGatewayOmnigateRouteKind::Register);
            mutate(&mut draft);
            assert_eq!(
                review_native_gateway_fixed_route_admission(draft),
                Err(NativeGatewayFixedRouteAdmissionReviewError::UnsafeGatewayAdmissionAuthorityFlag)
            );
        }
    }

    #[test]
    fn phase14c_source_remains_gateway_admission_contract_only_without_live_mounts_storage_wallet_ledger_or_secrets(
    ) {
        let source = fs::read_to_string(repo_file("src/native/gateway_fixed_route_admission.rs"))
            .expect("gateway fixed route admission source");
        let omnigate_source =
            fs::read_to_string(repo_file("src/native/omnigate_fixed_route_admission.rs"))
                .expect("omnigate fixed route admission source");
        let route_source = fs::read_to_string(repo_file("src/native/gateway_omnigate_routes.rs"))
            .expect("gateway omnigate route source");
        let native_mod =
            fs::read_to_string(repo_file("src/native/mod.rs")).expect("native module source");

        assert!(source.contains("review_native_gateway_fixed_route_admission"));
        assert!(source.contains("review_native_omnigate_fixed_route_admission"));
        assert!(source.contains("PHASE14A_GATEWAY_OMNIGATE_ROUTE_CATALOG"));
        assert!(omnigate_source.contains("review_native_omnigate_fixed_route_admission"));
        assert!(route_source.contains("review_native_gateway_omnigate_route_catalog"));
        assert!(native_mod.contains("GatewayFixedRouteAdmissionDto"));
        assert!(!source.contains("serde_json"));

        for forbidden_runtime_pattern in [
            "Router::",
            ".route(",
            "axum::routing",
            "lookup_projection.write(",
            "index.write(",
            "username_store.write(",
            "storage.write(",
            "persist_username(",
            "finalize_username(",
            "mutate_index_projection(",
            "issue_capability(",
            "refresh_capability(",
            "revoke_capability(",
            "load_secret",
            "load_key",
            "secret_key_bytes",
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
                "Phase 14C gateway fixed route admission source must not gain runtime pattern {forbidden_runtime_pattern}"
            );
        }
    }
}
