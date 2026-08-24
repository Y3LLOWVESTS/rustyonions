#[cfg(not(feature = "native-passport"))]
#[test]
fn phase14d_gateway_omnigate_route_mount_acceptance_is_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not compile native gateway/Omnigate route mount acceptance surfaces"
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use std::{fs, path::PathBuf};

    use svc_passport::native::{
        native_gateway_omnigate_route_mount_acceptance_fixed_route_count,
        native_gateway_omnigate_route_mount_acceptance_fixed_route_kinds,
        native_gateway_omnigate_route_mount_acceptance_fixed_route_methods,
        native_gateway_omnigate_route_mount_acceptance_posture,
        review_native_gateway_fixed_route_admission,
        review_native_gateway_omnigate_route_mount_acceptance,
        NativeGatewayFixedRouteAdmissionDecisionV1, NativeGatewayFixedRouteAdmissionDraftV1,
        NativeGatewayOmnigateRouteKind, NativeGatewayOmnigateRouteMethod,
        NativeGatewayOmnigateRouteMountAcceptanceDraftV1,
        NativeGatewayOmnigateRouteMountAcceptanceReviewError, NativeGatewayOmnigateRouteSpecV1,
        NativePassportSurface, NATIVE_PASSPORT_PHASE14D_LABEL,
        PHASE14A_GATEWAY_OMNIGATE_ROUTE_CATALOG, PHASE14A_GATEWAY_OMNIGATE_ROUTE_CONTRACT_DOMAIN,
        PHASE14A_GATEWAY_OMNIGATE_ROUTE_CONTRACT_VERSION, PHASE14C_ENABLED_SURFACES,
        PHASE14C_GATEWAY_FIXED_ROUTE_ADMISSION_DOMAIN,
        PHASE14C_GATEWAY_FIXED_ROUTE_ADMISSION_VERSION, PHASE14D_ENABLED_SURFACES,
        PHASE14D_FORBIDDEN_ROUTE_MOUNT_AUTHORITY_FLAGS,
        PHASE14D_GATEWAY_OMNIGATE_ROUTE_MOUNT_ACCEPTANCE_DOMAIN,
        PHASE14D_GATEWAY_OMNIGATE_ROUTE_MOUNT_ACCEPTANCE_VERSION,
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

    fn gateway_admission_draft(
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
            correlation_id: "corr-phase14d",
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

    fn accepted_gateway_decisions() -> Vec<NativeGatewayFixedRouteAdmissionDecisionV1> {
        PHASE14A_GATEWAY_OMNIGATE_ROUTE_CATALOG
            .iter()
            .map(|spec| {
                review_native_gateway_fixed_route_admission(gateway_admission_draft(spec.kind))
                    .expect("gateway admission decision")
            })
            .collect()
    }

    fn acceptance_draft() -> NativeGatewayOmnigateRouteMountAcceptanceDraftV1 {
        NativeGatewayOmnigateRouteMountAcceptanceDraftV1 {
            contract_domain: PHASE14D_GATEWAY_OMNIGATE_ROUTE_MOUNT_ACCEPTANCE_DOMAIN,
            contract_version: PHASE14D_GATEWAY_OMNIGATE_ROUTE_MOUNT_ACCEPTANCE_VERSION,
            expected_route_count: PHASE14A_GATEWAY_OMNIGATE_ROUTE_CATALOG.len(),
            fixed_route_catalog_reviewed: true,
            gateway_admission_reviewed: true,
            omnigate_admission_reviewed: true,
            all_routes_have_correlation_id: true,
            all_routes_have_body_caps: true,
            all_routes_have_deadlines: true,
            typed_redacted_problem_available: true,
            gateway_proxy_only: true,
            omnigate_orchestration_only: true,
            svc_passport_authority: true,
            mount_plan_only: true,
            requests_live_gateway_route_mount: false,
            requests_live_omnigate_route_mount: false,
            requests_dynamic_route_mount: false,
            caller_controlled_downstream_url: false,
            requests_request_body_logging: false,
            requests_response_body_logging: false,
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
    fn phase14d_label_and_posture_are_locked() {
        let posture = native_gateway_omnigate_route_mount_acceptance_posture();

        assert_eq!(
            NATIVE_PASSPORT_PHASE14D_LABEL,
            "NATIVE_PASSPORT_PHASE14D_GATEWAY_OMNIGATE_ROUTE_MOUNT_ACCEPTANCE"
        );
        assert_eq!(posture.phase_label, NATIVE_PASSPORT_PHASE14D_LABEL);
        assert!(posture.route_mount_acceptance_added);
        assert!(posture.fixed_route_catalog_reused);
        assert!(posture.gateway_admission_chain_reused);
        assert!(posture.omnigate_admission_chain_reused);
        assert!(posture.body_cap_acceptance_added);
        assert!(posture.deadline_acceptance_added);
        assert!(posture.correlation_id_acceptance_added);
        assert!(posture.typed_redacted_problem_acceptance_added);
        assert!(!posture.live_gateway_route_mount_added);
        assert!(!posture.live_omnigate_route_mount_added);
        assert!(!posture.dynamic_route_mount_added);
        assert!(!posture.gateway_identity_authority_added);
        assert!(!posture.omnigate_identity_authority_added);
        assert!(!posture.storage_mutation_inside_gateway_or_omnigate_added);
        assert!(!posture.wallet_or_ledger_mutation_added);
        assert!(!posture.secret_key_access_added);
        assert!(!posture.native_secret_implementation_added);

        assert_eq!(
            native_gateway_omnigate_route_mount_acceptance_fixed_route_count(),
            15
        );
        assert!(
            native_gateway_omnigate_route_mount_acceptance_fixed_route_methods()
                .contains(&NativeGatewayOmnigateRouteMethod::Post)
        );
        assert!(
            native_gateway_omnigate_route_mount_acceptance_fixed_route_methods()
                .contains(&NativeGatewayOmnigateRouteMethod::Get)
        );
        assert!(
            native_gateway_omnigate_route_mount_acceptance_fixed_route_kinds()
                .contains(&NativeGatewayOmnigateRouteKind::ProtectedRead)
        );

        for forbidden in [
            "dynamic_route_mount",
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
            assert!(PHASE14D_FORBIDDEN_ROUTE_MOUNT_AUTHORITY_FLAGS.contains(&forbidden));
        }
    }

    #[test]
    fn phase14d_surfaces_extend_phase14c_without_back_mutating_it() {
        assert_eq!(
            PHASE14D_ENABLED_SURFACES.len(),
            PHASE14C_ENABLED_SURFACES.len() + 1
        );
        assert_eq!(
            &PHASE14D_ENABLED_SURFACES[..PHASE14C_ENABLED_SURFACES.len()],
            PHASE14C_ENABLED_SURFACES
        );
        assert_eq!(
            PHASE14D_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::GatewayOmnigateRouteMountAcceptanceDto)
        );
        assert_eq!(
            PHASE14C_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::GatewayFixedRouteAdmissionDto)
        );
    }

    #[test]
    fn phase14d_accepts_full_fixed_route_mount_plan_from_gateway_to_omnigate_to_svc_passport() {
        let gateway_decisions = accepted_gateway_decisions();
        let decision = review_native_gateway_omnigate_route_mount_acceptance(
            acceptance_draft(),
            &gateway_decisions,
        )
        .expect("route mount acceptance should review");

        assert_eq!(
            decision.contract_domain,
            PHASE14D_GATEWAY_OMNIGATE_ROUTE_MOUNT_ACCEPTANCE_DOMAIN
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
            decision.gateway_admission_contract_domain,
            PHASE14C_GATEWAY_FIXED_ROUTE_ADMISSION_DOMAIN
        );
        assert_eq!(
            decision.gateway_admission_contract_version,
            PHASE14C_GATEWAY_FIXED_ROUTE_ADMISSION_VERSION
        );
        assert_eq!(decision.accepted_route_count, 15);
        assert!(decision.all_fixed_routes_covered);
        assert!(decision.fixed_route_catalog_reviewed);
        assert!(decision.gateway_admission_reviewed);
        assert!(decision.omnigate_admission_reviewed);
        assert!(decision.all_routes_have_correlation_id);
        assert!(decision.all_routes_have_body_caps);
        assert!(decision.all_routes_have_deadlines);
        assert!(decision.typed_redacted_problem_available);
        assert!(decision.gateway_proxy_only);
        assert!(decision.omnigate_orchestration_only);
        assert!(decision.svc_passport_authority);
        assert!(decision.mount_plan_only);
        assert!(!decision.live_gateway_route_mount_added);
        assert!(!decision.live_omnigate_route_mount_added);
        assert!(!decision.dynamic_route_mount_added);
        assert!(!decision.storage_mutated_inside_gateway_or_omnigate);
        assert!(!decision.wallet_or_ledger_mutated);
        assert!(!decision.secret_material_exposed);
        assert!(decision.contract_only);
    }

    #[test]
    fn phase14d_rejects_missing_duplicate_drifted_or_unsafe_gateway_admission_decisions() {
        let mut missing = accepted_gateway_decisions();
        missing.pop();
        assert_eq!(
            review_native_gateway_omnigate_route_mount_acceptance(acceptance_draft(), &missing),
            Err(NativeGatewayOmnigateRouteMountAcceptanceReviewError::RouteCountMismatch)
        );

        let mut duplicate = accepted_gateway_decisions();
        duplicate[1] = duplicate[0].clone();
        assert_eq!(
            review_native_gateway_omnigate_route_mount_acceptance(acceptance_draft(), &duplicate),
            Err(
                NativeGatewayOmnigateRouteMountAcceptanceReviewError::DuplicateGatewayRouteDecision
            )
        );

        let mut drifted = accepted_gateway_decisions();
        drifted[0].public_gateway_path = "/identity/passport/not-fixed";
        assert_eq!(
            review_native_gateway_omnigate_route_mount_acceptance(acceptance_draft(), &drifted),
            Err(NativeGatewayOmnigateRouteMountAcceptanceReviewError::RouteDecisionShapeMismatch)
        );

        let mut unsafe_decision = accepted_gateway_decisions();
        unsafe_decision[0].gateway_proxy_only = false;
        assert_eq!(
            review_native_gateway_omnigate_route_mount_acceptance(
                acceptance_draft(),
                &unsafe_decision
            ),
            Err(NativeGatewayOmnigateRouteMountAcceptanceReviewError::RouteDecisionUnsafe)
        );

        let mut wrong_contract = accepted_gateway_decisions();
        wrong_contract[0].contract_domain = "wrong";
        assert_eq!(
            review_native_gateway_omnigate_route_mount_acceptance(
                acceptance_draft(),
                &wrong_contract
            ),
            Err(
                NativeGatewayOmnigateRouteMountAcceptanceReviewError::GatewayAdmissionDecisionContractMismatch
            )
        );
    }

    #[test]
    fn phase14d_rejects_mount_plan_drift_live_mounts_logging_urls_and_unsafe_flags() {
        let gateway_decisions = accepted_gateway_decisions();

        let mut no_gateway = acceptance_draft();
        no_gateway.gateway_admission_reviewed = false;
        assert_eq!(
            review_native_gateway_omnigate_route_mount_acceptance(no_gateway, &gateway_decisions),
            Err(NativeGatewayOmnigateRouteMountAcceptanceReviewError::GatewayAdmissionNotReviewed)
        );

        let mut no_omnigate = acceptance_draft();
        no_omnigate.omnigate_admission_reviewed = false;
        assert_eq!(
            review_native_gateway_omnigate_route_mount_acceptance(no_omnigate, &gateway_decisions),
            Err(NativeGatewayOmnigateRouteMountAcceptanceReviewError::OmnigateAdmissionNotReviewed)
        );

        let mut live_mount = acceptance_draft();
        live_mount.requests_live_gateway_route_mount = true;
        assert_eq!(
            review_native_gateway_omnigate_route_mount_acceptance(live_mount, &gateway_decisions),
            Err(NativeGatewayOmnigateRouteMountAcceptanceReviewError::LiveRouteMountRequested)
        );

        let mut dynamic_mount = acceptance_draft();
        dynamic_mount.requests_dynamic_route_mount = true;
        assert_eq!(
            review_native_gateway_omnigate_route_mount_acceptance(
                dynamic_mount,
                &gateway_decisions
            ),
            Err(NativeGatewayOmnigateRouteMountAcceptanceReviewError::DynamicRouteMountRequested)
        );

        let mut caller_url = acceptance_draft();
        caller_url.caller_controlled_downstream_url = true;
        assert_eq!(
            review_native_gateway_omnigate_route_mount_acceptance(caller_url, &gateway_decisions),
            Err(
                NativeGatewayOmnigateRouteMountAcceptanceReviewError::CallerControlledDownstreamUrl
            )
        );

        let mut body_logging = acceptance_draft();
        body_logging.requests_request_body_logging = true;
        assert_eq!(
            review_native_gateway_omnigate_route_mount_acceptance(body_logging, &gateway_decisions),
            Err(
                NativeGatewayOmnigateRouteMountAcceptanceReviewError::RequestOrResponseBodyLoggingRequested
            )
        );

        let mutators: &[fn(&mut NativeGatewayOmnigateRouteMountAcceptanceDraftV1)] = &[
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
            let mut draft = acceptance_draft();
            mutate(&mut draft);
            assert_eq!(
                review_native_gateway_omnigate_route_mount_acceptance(draft, &gateway_decisions),
                Err(
                    NativeGatewayOmnigateRouteMountAcceptanceReviewError::UnsafeRouteMountAuthorityFlag
                )
            );
        }
    }

    #[test]
    fn phase14d_source_remains_mount_acceptance_contract_only_without_live_mounts_storage_wallet_ledger_or_secrets(
    ) {
        let source = fs::read_to_string(repo_file(
            "src/native/gateway_omnigate_route_mount_acceptance.rs",
        ))
        .expect("gateway omnigate route mount acceptance source");
        let gateway_source =
            fs::read_to_string(repo_file("src/native/gateway_fixed_route_admission.rs"))
                .expect("gateway fixed route admission source");
        let omnigate_source =
            fs::read_to_string(repo_file("src/native/omnigate_fixed_route_admission.rs"))
                .expect("omnigate fixed route admission source");
        let route_source = fs::read_to_string(repo_file("src/native/gateway_omnigate_routes.rs"))
            .expect("gateway omnigate route source");
        let native_mod =
            fs::read_to_string(repo_file("src/native/mod.rs")).expect("native module source");

        assert!(source.contains("review_native_gateway_omnigate_route_mount_acceptance"));
        assert!(gateway_source.contains("review_native_gateway_fixed_route_admission"));
        assert!(omnigate_source.contains("review_native_omnigate_fixed_route_admission"));
        assert!(route_source.contains("review_native_gateway_omnigate_route_catalog"));
        assert!(native_mod.contains("GatewayOmnigateRouteMountAcceptanceDto"));
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
                "Phase 14D route mount acceptance source must not gain runtime pattern {forbidden_runtime_pattern}"
            );
        }
    }
}
