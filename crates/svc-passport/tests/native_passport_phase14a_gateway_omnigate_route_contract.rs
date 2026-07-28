#[cfg(not(feature = "native-passport"))]
#[test]
fn phase14a_gateway_omnigate_route_contract_is_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not compile native gateway/Omnigate route contract surfaces"
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use std::{fs, path::PathBuf};

    use svc_passport::native::{
        native_gateway_omnigate_route_contract_posture,
        review_native_gateway_omnigate_problem_envelope,
        review_native_gateway_omnigate_route_catalog, NativeGatewayOmnigateProblemEnvelopeDraftV1,
        NativeGatewayOmnigateRouteKind, NativeGatewayOmnigateRouteReviewError,
        NativePassportSurface, NATIVE_PASSPORT_PHASE14A_LABEL, PHASE13_ENABLED_SURFACES,
        PHASE14A_ENABLED_SURFACES, PHASE14A_FORBIDDEN_GATEWAY_OMNIGATE_AUTHORITY_FLAGS,
        PHASE14A_GATEWAY_OMNIGATE_ROUTE_CATALOG, PHASE14A_GATEWAY_OMNIGATE_ROUTE_CONTRACT_DOMAIN,
        PHASE14A_GATEWAY_OMNIGATE_ROUTE_CONTRACT_VERSION, PHASE14A_MAX_BODY_CAP_BYTES,
        PHASE14A_MAX_DEADLINE_MS,
    };

    fn repo_file(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
    }

    fn safe_problem() -> NativeGatewayOmnigateProblemEnvelopeDraftV1 {
        NativeGatewayOmnigateProblemEnvelopeDraftV1 {
            code: "passport_upstream",
            http_status: 502,
            retryable: true,
            correlation_id: "corr-phase14a",
            public_message: "Passport service is temporarily unavailable.",
            redacted: true,
            includes_raw_request_body: false,
            includes_raw_response_body: false,
            includes_secret_material: false,
            includes_capability_material: false,
            includes_wallet_or_ledger_material: false,
        }
    }

    #[test]
    fn phase14a_label_and_posture_are_locked() {
        let posture = native_gateway_omnigate_route_contract_posture();

        assert_eq!(
            NATIVE_PASSPORT_PHASE14A_LABEL,
            "NATIVE_PASSPORT_PHASE14A_GATEWAY_OMNIGATE_ROUTE_CONTRACT"
        );
        assert_eq!(posture.phase_label, NATIVE_PASSPORT_PHASE14A_LABEL);
        assert!(posture.route_contract_added);
        assert!(posture.fixed_gateway_routes_added);
        assert!(posture.fixed_omnigate_routes_added);
        assert!(posture.downstream_passport_route_mapping_added);
        assert!(posture.body_caps_added);
        assert!(posture.deadlines_added);
        assert!(posture.correlation_id_review_added);
        assert!(posture.redacted_problem_envelope_added);
        assert!(posture.raw_body_logging_disabled);
        assert!(!posture.live_gateway_route_mount_added);
        assert!(!posture.live_omnigate_route_mount_added);
        assert!(!posture.gateway_identity_authority_added);
        assert!(!posture.omnigate_identity_authority_added);
        assert!(!posture.storage_mutation_inside_gateway_or_omnigate_added);
        assert!(!posture.wallet_or_ledger_mutation_added);
        assert!(!posture.secret_key_access_added);
        assert!(!posture.native_secret_implementation_added);
        assert_eq!(PHASE14A_MAX_BODY_CAP_BYTES, 16_384);
        assert_eq!(PHASE14A_MAX_DEADLINE_MS, 5_000);

        for forbidden in [
            "gateway_identity_authority",
            "omnigate_identity_authority",
            "raw_request_body_logging",
            "raw_response_body_logging",
            "secret_echo",
            "capability_material_echo",
            "wallet_or_ledger_material_echo",
            "wallet_spend",
            "ledger_mutation",
        ] {
            assert!(posture.forbidden_authority_flags.contains(&forbidden));
            assert!(PHASE14A_FORBIDDEN_GATEWAY_OMNIGATE_AUTHORITY_FLAGS.contains(&forbidden));
        }
    }

    #[test]
    fn phase14a_surfaces_extend_phase13_without_back_mutating_it() {
        assert_eq!(
            PHASE14A_ENABLED_SURFACES.len(),
            PHASE13_ENABLED_SURFACES.len() + 1
        );
        assert_eq!(
            &PHASE14A_ENABLED_SURFACES[..PHASE13_ENABLED_SURFACES.len()],
            PHASE13_ENABLED_SURFACES
        );
        assert_eq!(
            PHASE14A_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::GatewayOmnigateRouteContractDto)
        );
        assert_eq!(
            PHASE13_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::LocalStatusInspectionDto)
        );
    }

    #[test]
    fn phase14a_reviews_fixed_gateway_omnigate_route_catalog_without_authority_shift() {
        let decision =
            review_native_gateway_omnigate_route_catalog(PHASE14A_GATEWAY_OMNIGATE_ROUTE_CATALOG)
                .expect("fixed route catalog should review");

        assert_eq!(
            decision.contract_domain,
            PHASE14A_GATEWAY_OMNIGATE_ROUTE_CONTRACT_DOMAIN
        );
        assert_eq!(
            decision.contract_version,
            PHASE14A_GATEWAY_OMNIGATE_ROUTE_CONTRACT_VERSION
        );
        assert_eq!(decision.route_count, 13);
        assert!(decision.all_routes_fixed);
        assert!(decision.gateway_proxy_only);
        assert!(decision.omnigate_orchestration_only);
        assert!(decision.svc_passport_authority);
        assert!(decision.body_caps_reviewed);
        assert!(decision.deadlines_reviewed);
        assert!(decision.correlation_ids_required);
        assert!(decision.redacted_problems_required);
        assert!(decision.raw_body_logging_disabled);
        assert!(!decision.mutating_scope_escalation_allowed);
        assert!(!decision.storage_mutated_inside_gateway_or_omnigate);
        assert!(!decision.wallet_or_ledger_mutated);
        assert!(!decision.secret_material_exposed);
        assert!(decision.contract_only);

        let public_paths: Vec<&'static str> = PHASE14A_GATEWAY_OMNIGATE_ROUTE_CATALOG
            .iter()
            .map(|spec| spec.public_gateway_path)
            .collect();

        for required_path in [
            "/identity/passport/register",
            "/identity/passport/challenge",
            "/identity/passport/prove",
            "/identity/passport/device/authorize",
            "/identity/passport/device/revoke",
            "/identity/passport/capability/status",
            "/identity/passport/capability/refresh",
            "/identity/passport/capability/revoke",
            "/identity/passport/status",
            "/identity/passport/username/claim",
            "/identity/passport/username/transfer",
            "/identity/passport/username/release",
            "/identity/passport/protected/read",
        ] {
            assert!(public_paths.contains(&required_path));
        }

        let protected_read = PHASE14A_GATEWAY_OMNIGATE_ROUTE_CATALOG
            .iter()
            .find(|spec| spec.kind == NativeGatewayOmnigateRouteKind::ProtectedRead)
            .expect("protected read route");
        assert!(protected_read.requires_device_bound_capability);
        assert!(!protected_read.mutating_scope_allowed);
    }

    #[test]
    fn phase14a_rejects_route_drift_missing_guards_and_authority_shift() {
        let mut bad_catalog = PHASE14A_GATEWAY_OMNIGATE_ROUTE_CATALOG.to_vec();
        bad_catalog[0].public_gateway_path = "/identity/passport/changed";
        assert_eq!(
            review_native_gateway_omnigate_route_catalog(&bad_catalog),
            Err(NativeGatewayOmnigateRouteReviewError::RouteShapeMismatch)
        );

        let mut bad_catalog = PHASE14A_GATEWAY_OMNIGATE_ROUTE_CATALOG.to_vec();
        bad_catalog[0].body_cap_bytes = PHASE14A_MAX_BODY_CAP_BYTES + 1;
        assert_eq!(
            review_native_gateway_omnigate_route_catalog(&bad_catalog),
            Err(NativeGatewayOmnigateRouteReviewError::RouteShapeMismatch)
        );

        let mut bad_catalog = PHASE14A_GATEWAY_OMNIGATE_ROUTE_CATALOG.to_vec();
        bad_catalog[0].deadline_ms = PHASE14A_MAX_DEADLINE_MS + 1;
        assert_eq!(
            review_native_gateway_omnigate_route_catalog(&bad_catalog),
            Err(NativeGatewayOmnigateRouteReviewError::DeadlineExceeded)
        );

        let mut bad_catalog = PHASE14A_GATEWAY_OMNIGATE_ROUTE_CATALOG.to_vec();
        bad_catalog[0].correlation_id_required = false;
        assert_eq!(
            review_native_gateway_omnigate_route_catalog(&bad_catalog),
            Err(NativeGatewayOmnigateRouteReviewError::MissingCorrelationIdRequirement)
        );

        let mut bad_catalog = PHASE14A_GATEWAY_OMNIGATE_ROUTE_CATALOG.to_vec();
        bad_catalog[0].raw_body_logging_disabled = false;
        assert_eq!(
            review_native_gateway_omnigate_route_catalog(&bad_catalog),
            Err(NativeGatewayOmnigateRouteReviewError::RawBodyLoggingEnabled)
        );

        let mut bad_catalog = PHASE14A_GATEWAY_OMNIGATE_ROUTE_CATALOG.to_vec();
        bad_catalog[0].gateway_proxy_only = false;
        assert_eq!(
            review_native_gateway_omnigate_route_catalog(&bad_catalog),
            Err(NativeGatewayOmnigateRouteReviewError::GatewayClaimsIdentityAuthority)
        );

        let mut bad_catalog = PHASE14A_GATEWAY_OMNIGATE_ROUTE_CATALOG.to_vec();
        bad_catalog[0].omnigate_orchestration_only = false;
        assert_eq!(
            review_native_gateway_omnigate_route_catalog(&bad_catalog),
            Err(NativeGatewayOmnigateRouteReviewError::OmnigateClaimsIdentityAuthority)
        );

        let mut bad_catalog = PHASE14A_GATEWAY_OMNIGATE_ROUTE_CATALOG.to_vec();
        bad_catalog[0].mutating_scope_allowed = true;
        assert_eq!(
            review_native_gateway_omnigate_route_catalog(&bad_catalog),
            Err(NativeGatewayOmnigateRouteReviewError::MutatingScopeEscalationAllowed)
        );
    }

    #[test]
    fn phase14a_reviews_typed_redacted_problem_envelope_without_secret_echo() {
        let problem = review_native_gateway_omnigate_problem_envelope(safe_problem())
            .expect("safe problem envelope should review");

        assert_eq!(problem.code, "passport_upstream");
        assert_eq!(problem.http_status, 502);
        assert!(problem.retryable);
        assert_eq!(problem.correlation_id, "corr-phase14a");
        assert!(problem.redacted);
        assert!(!problem.raw_request_body_exposed);
        assert!(!problem.raw_response_body_exposed);
        assert!(!problem.secret_material_exposed);
        assert!(!problem.capability_material_exposed);
        assert!(!problem.wallet_or_ledger_material_exposed);

        let mut leaking = safe_problem();
        leaking.includes_secret_material = true;
        assert_eq!(
            review_native_gateway_omnigate_problem_envelope(leaking),
            Err(NativeGatewayOmnigateRouteReviewError::ProblemEnvelopeLeaksMaterial)
        );

        let mut leaking = safe_problem();
        leaking.includes_raw_request_body = true;
        assert_eq!(
            review_native_gateway_omnigate_problem_envelope(leaking),
            Err(NativeGatewayOmnigateRouteReviewError::ProblemEnvelopeLeaksMaterial)
        );

        let mut unredacted = safe_problem();
        unredacted.redacted = false;
        assert_eq!(
            review_native_gateway_omnigate_problem_envelope(unredacted),
            Err(NativeGatewayOmnigateRouteReviewError::ProblemEnvelopeNotRedacted)
        );

        let mut no_correlation = safe_problem();
        no_correlation.correlation_id = "";
        assert_eq!(
            review_native_gateway_omnigate_problem_envelope(no_correlation),
            Err(NativeGatewayOmnigateRouteReviewError::ProblemCorrelationIdMissing)
        );
    }

    #[test]
    fn phase14a_source_remains_route_contract_only_without_live_mounts_storage_wallet_ledger_or_secrets(
    ) {
        let source = fs::read_to_string(repo_file("src/native/gateway_omnigate_routes.rs"))
            .expect("gateway omnigate route contract source");
        let native_mod =
            fs::read_to_string(repo_file("src/native/mod.rs")).expect("native module source");

        assert!(source.contains("PHASE14A_GATEWAY_OMNIGATE_ROUTE_CATALOG"));
        assert!(source.contains("review_native_gateway_omnigate_route_catalog"));
        assert!(source.contains("review_native_gateway_omnigate_problem_envelope"));
        assert!(native_mod.contains("GatewayOmnigateRouteContractDto"));
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
                "Phase 14A route contract source must not gain runtime pattern {forbidden_runtime_pattern}"
            );
        }
    }
}
