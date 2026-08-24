#[cfg(not(feature = "native-passport"))]
#[test]
fn phase14b_omnigate_fixed_route_admission_is_feature_gated_in_default_build() {
    assert!(
        !cfg!(feature = "native-passport"),
        "default svc-passport builds must not compile native Omnigate fixed-route admission surfaces"
    );
}

#[cfg(feature = "native-passport")]
mod feature_tests {
    use std::{fs, path::PathBuf};

    use svc_passport::native::{
        native_omnigate_fixed_route_admission_posture,
        review_native_omnigate_fixed_route_admission, NativeGatewayOmnigateRouteKind,
        NativeGatewayOmnigateRouteMethod, NativeOmnigateFixedRouteAdmissionDraftV1,
        NativeOmnigateFixedRouteAdmissionReviewError, NativePassportSurface,
        NATIVE_PASSPORT_PHASE14B_LABEL, PHASE14A_ENABLED_SURFACES,
        PHASE14A_GATEWAY_OMNIGATE_ROUTE_CONTRACT_DOMAIN,
        PHASE14A_GATEWAY_OMNIGATE_ROUTE_CONTRACT_VERSION, PHASE14B_ENABLED_SURFACES,
        PHASE14B_FORBIDDEN_ADMISSION_AUTHORITY_FLAGS,
        PHASE14B_OMNIGATE_FIXED_ROUTE_ADMISSION_DOMAIN,
        PHASE14B_OMNIGATE_FIXED_ROUTE_ADMISSION_VERSION,
    };

    fn repo_file(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
    }

    fn admission_draft(
        route_kind: NativeGatewayOmnigateRouteKind,
    ) -> NativeOmnigateFixedRouteAdmissionDraftV1 {
        match route_kind {
            NativeGatewayOmnigateRouteKind::Register => draft_for(
                route_kind,
                NativeGatewayOmnigateRouteMethod::Post,
                "/identity/passport/register",
                "/v1/identity/passport/register",
                "/v1/passport/register",
                512,
                5_000,
                false,
                true,
            ),
            NativeGatewayOmnigateRouteKind::ChallengeIssue => draft_for(
                route_kind,
                NativeGatewayOmnigateRouteMethod::Post,
                "/identity/passport/register/challenge",
                "/v1/identity/passport/register/challenge",
                "/v1/passport/register/challenge",
                512,
                5_000,
                false,
                false,
            ),
            NativeGatewayOmnigateRouteKind::ProofSubmit => draft_for(
                route_kind,
                NativeGatewayOmnigateRouteMethod::Post,
                "/identity/passport/register/proof",
                "/v1/identity/passport/register/proof",
                "/v1/passport/register/proof",
                512,
                5_000,
                false,
                true,
            ),
            NativeGatewayOmnigateRouteKind::DeviceAuthorize => draft_for(
                route_kind,
                NativeGatewayOmnigateRouteMethod::Post,
                "/identity/passport/device/authorize",
                "/v1/identity/passport/device/authorize",
                "/v1/passport/device/authorize",
                512,
                5_000,
                false,
                true,
            ),
            NativeGatewayOmnigateRouteKind::DeviceSessionChallenge => draft_for(
                route_kind,
                NativeGatewayOmnigateRouteMethod::Post,
                "/identity/passport/challenge",
                "/v1/identity/passport/challenge",
                "/v1/passport/challenge",
                512,
                5_000,
                false,
                false,
            ),
            NativeGatewayOmnigateRouteKind::DeviceSessionProof => draft_for(
                route_kind,
                NativeGatewayOmnigateRouteMethod::Post,
                "/identity/passport/prove",
                "/v1/identity/passport/prove",
                "/v1/passport/prove",
                512,
                5_000,
                false,
                true,
            ),
            NativeGatewayOmnigateRouteKind::DeviceRevoke => draft_for(
                route_kind,
                NativeGatewayOmnigateRouteMethod::Post,
                "/identity/passport/device/revoke",
                "/v1/identity/passport/device/revoke",
                "/v1/passport/device/revoke",
                512,
                5_000,
                false,
                true,
            ),
            NativeGatewayOmnigateRouteKind::CapabilityStatus => draft_for(
                route_kind,
                NativeGatewayOmnigateRouteMethod::Get,
                "/identity/passport/capability/status",
                "/v1/identity/passport/capability/status",
                "/v1/passport/capability/status",
                0,
                5_000,
                true,
                false,
            ),
            NativeGatewayOmnigateRouteKind::CapabilityRefresh => draft_for(
                route_kind,
                NativeGatewayOmnigateRouteMethod::Post,
                "/identity/passport/capability/refresh",
                "/v1/identity/passport/capability/refresh",
                "/v1/passport/capability/refresh",
                512,
                5_000,
                true,
                true,
            ),
            NativeGatewayOmnigateRouteKind::CapabilityRevoke => draft_for(
                route_kind,
                NativeGatewayOmnigateRouteMethod::Post,
                "/identity/passport/capability/revoke",
                "/v1/identity/passport/capability/revoke",
                "/v1/passport/capability/revoke",
                512,
                5_000,
                true,
                true,
            ),
            NativeGatewayOmnigateRouteKind::LocalStatus => draft_for(
                route_kind,
                NativeGatewayOmnigateRouteMethod::Get,
                "/identity/passport/status",
                "/v1/identity/passport/status",
                "/v1/passport/status",
                0,
                5_000,
                false,
                false,
            ),
            NativeGatewayOmnigateRouteKind::UsernameClaim => draft_for(
                route_kind,
                NativeGatewayOmnigateRouteMethod::Post,
                "/identity/passport/username/claim",
                "/v1/identity/passport/username/claim",
                "/v1/passport/username/claim",
                512,
                5_000,
                false,
                true,
            ),
            NativeGatewayOmnigateRouteKind::UsernameTransfer => draft_for(
                route_kind,
                NativeGatewayOmnigateRouteMethod::Post,
                "/identity/passport/username/transfer",
                "/v1/identity/passport/username/transfer",
                "/v1/passport/username/transfer",
                512,
                5_000,
                false,
                true,
            ),
            NativeGatewayOmnigateRouteKind::UsernameRelease => draft_for(
                route_kind,
                NativeGatewayOmnigateRouteMethod::Post,
                "/identity/passport/username/release",
                "/v1/identity/passport/username/release",
                "/v1/passport/username/release",
                512,
                5_000,
                false,
                true,
            ),
            NativeGatewayOmnigateRouteKind::ProtectedRead => draft_for(
                route_kind,
                NativeGatewayOmnigateRouteMethod::Get,
                "/identity/passport/protected/read",
                "/v1/identity/passport/protected/read",
                "/v1/passport/protected/read",
                0,
                5_000,
                true,
                false,
            ),
        }
    }

    // Test fixture intentionally keeps every fixed-route contract column
    // explicit at each call site so path/method/body/deadline/proof vectors
    // remain directly auditable rather than hidden behind another state model.
    #[allow(clippy::too_many_arguments)]
    fn draft_for(
        route_kind: NativeGatewayOmnigateRouteKind,
        method: NativeGatewayOmnigateRouteMethod,
        public_gateway_path: &'static str,
        omnigate_path: &'static str,
        downstream_passport_path: &'static str,
        request_body_len_bytes: u32,
        deadline_ms: u32,
        device_bound_capability_present: bool,
        fresh_proof_present: bool,
    ) -> NativeOmnigateFixedRouteAdmissionDraftV1 {
        NativeOmnigateFixedRouteAdmissionDraftV1 {
            contract_domain: PHASE14B_OMNIGATE_FIXED_ROUTE_ADMISSION_DOMAIN,
            contract_version: PHASE14B_OMNIGATE_FIXED_ROUTE_ADMISSION_VERSION,
            route_kind,
            method,
            public_gateway_path,
            omnigate_path,
            downstream_passport_path,
            request_body_len_bytes,
            deadline_ms,
            correlation_id: "corr-phase14b",
            device_bound_capability_present,
            fresh_proof_present,
            mutating_scope_requested: false,
            gateway_proxy_only: true,
            omnigate_orchestration_only: true,
            svc_passport_authority: true,
            fixed_route_catalog_reviewed: true,
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
    fn phase14b_label_and_posture_are_locked() {
        let posture = native_omnigate_fixed_route_admission_posture();

        assert_eq!(
            NATIVE_PASSPORT_PHASE14B_LABEL,
            "NATIVE_PASSPORT_PHASE14B_OMNIGATE_FIXED_ROUTE_ADMISSION"
        );
        assert_eq!(posture.phase_label, NATIVE_PASSPORT_PHASE14B_LABEL);
        assert!(posture.fixed_route_admission_added);
        assert!(posture.fixed_route_catalog_reused);
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
            "dynamic_route_admission",
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
            assert!(PHASE14B_FORBIDDEN_ADMISSION_AUTHORITY_FLAGS.contains(&forbidden));
        }
    }

    #[test]
    fn phase14b_surfaces_extend_phase14a_without_back_mutating_it() {
        assert_eq!(
            PHASE14B_ENABLED_SURFACES.len(),
            PHASE14A_ENABLED_SURFACES.len() + 1
        );
        assert_eq!(
            &PHASE14B_ENABLED_SURFACES[..PHASE14A_ENABLED_SURFACES.len()],
            PHASE14A_ENABLED_SURFACES
        );
        assert_eq!(
            PHASE14B_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::OmnigateFixedRouteAdmissionDto)
        );
        assert_eq!(
            PHASE14A_ENABLED_SURFACES.last(),
            Some(&NativePassportSurface::GatewayOmnigateRouteContractDto)
        );
    }

    #[test]
    fn phase14b_accepts_every_fixed_route_with_required_capability_and_proof_guards() {
        for route_kind in [
            NativeGatewayOmnigateRouteKind::Register,
            NativeGatewayOmnigateRouteKind::ChallengeIssue,
            NativeGatewayOmnigateRouteKind::ProofSubmit,
            NativeGatewayOmnigateRouteKind::DeviceAuthorize,
            NativeGatewayOmnigateRouteKind::DeviceSessionChallenge,
            NativeGatewayOmnigateRouteKind::DeviceSessionProof,
            NativeGatewayOmnigateRouteKind::DeviceRevoke,
            NativeGatewayOmnigateRouteKind::CapabilityStatus,
            NativeGatewayOmnigateRouteKind::CapabilityRefresh,
            NativeGatewayOmnigateRouteKind::CapabilityRevoke,
            NativeGatewayOmnigateRouteKind::LocalStatus,
            NativeGatewayOmnigateRouteKind::UsernameClaim,
            NativeGatewayOmnigateRouteKind::UsernameTransfer,
            NativeGatewayOmnigateRouteKind::UsernameRelease,
            NativeGatewayOmnigateRouteKind::ProtectedRead,
        ] {
            let decision =
                review_native_omnigate_fixed_route_admission(admission_draft(route_kind))
                    .expect("fixed route admission should review");

            assert_eq!(
                decision.contract_domain,
                PHASE14B_OMNIGATE_FIXED_ROUTE_ADMISSION_DOMAIN
            );
            assert_eq!(
                decision.route_contract_domain,
                PHASE14A_GATEWAY_OMNIGATE_ROUTE_CONTRACT_DOMAIN
            );
            assert_eq!(
                decision.route_contract_version,
                PHASE14A_GATEWAY_OMNIGATE_ROUTE_CONTRACT_VERSION
            );
            assert_eq!(decision.route_kind, route_kind);
            assert!(decision.fixed_route_admitted);
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
    fn phase14b_rejects_dynamic_paths_caps_deadlines_and_missing_route_guards() {
        let mut wrong_path = admission_draft(NativeGatewayOmnigateRouteKind::Register);
        wrong_path.public_gateway_path = "/identity/passport/not-fixed";
        assert_eq!(
            review_native_omnigate_fixed_route_admission(wrong_path),
            Err(NativeOmnigateFixedRouteAdmissionReviewError::GatewayPathMismatch)
        );

        let mut wrong_method = admission_draft(NativeGatewayOmnigateRouteKind::ProtectedRead);
        wrong_method.method = NativeGatewayOmnigateRouteMethod::Post;
        assert_eq!(
            review_native_omnigate_fixed_route_admission(wrong_method),
            Err(NativeOmnigateFixedRouteAdmissionReviewError::MethodMismatch)
        );

        let mut too_large = admission_draft(NativeGatewayOmnigateRouteKind::Register);
        too_large.request_body_len_bytes = 16_385;
        assert_eq!(
            review_native_omnigate_fixed_route_admission(too_large),
            Err(NativeOmnigateFixedRouteAdmissionReviewError::BodyCapExceeded)
        );

        let mut too_slow = admission_draft(NativeGatewayOmnigateRouteKind::Register);
        too_slow.deadline_ms = 5_001;
        assert_eq!(
            review_native_omnigate_fixed_route_admission(too_slow),
            Err(NativeOmnigateFixedRouteAdmissionReviewError::DeadlineExceeded)
        );

        let mut no_correlation = admission_draft(NativeGatewayOmnigateRouteKind::Register);
        no_correlation.correlation_id = "";
        assert_eq!(
            review_native_omnigate_fixed_route_admission(no_correlation),
            Err(NativeOmnigateFixedRouteAdmissionReviewError::MissingCorrelationId)
        );

        let mut no_capability = admission_draft(NativeGatewayOmnigateRouteKind::ProtectedRead);
        no_capability.device_bound_capability_present = false;
        assert_eq!(
            review_native_omnigate_fixed_route_admission(no_capability),
            Err(NativeOmnigateFixedRouteAdmissionReviewError::MissingRequiredDeviceBoundCapability)
        );

        let mut no_proof = admission_draft(NativeGatewayOmnigateRouteKind::Register);
        no_proof.fresh_proof_present = false;
        assert_eq!(
            review_native_omnigate_fixed_route_admission(no_proof),
            Err(NativeOmnigateFixedRouteAdmissionReviewError::MissingRequiredFreshProof)
        );
    }

    #[test]
    fn phase14b_rejects_authority_shift_body_logging_live_mounts_and_unsafe_flags() {
        let mut mutating_scope = admission_draft(NativeGatewayOmnigateRouteKind::ProtectedRead);
        mutating_scope.mutating_scope_requested = true;
        assert_eq!(
            review_native_omnigate_fixed_route_admission(mutating_scope),
            Err(NativeOmnigateFixedRouteAdmissionReviewError::MutatingScopeEscalationRequested)
        );

        let mut gateway_authority = admission_draft(NativeGatewayOmnigateRouteKind::Register);
        gateway_authority.gateway_proxy_only = false;
        assert_eq!(
            review_native_omnigate_fixed_route_admission(gateway_authority),
            Err(NativeOmnigateFixedRouteAdmissionReviewError::GatewayClaimsIdentityAuthority)
        );

        let mut omnigate_authority = admission_draft(NativeGatewayOmnigateRouteKind::Register);
        omnigate_authority.omnigate_orchestration_only = false;
        assert_eq!(
            review_native_omnigate_fixed_route_admission(omnigate_authority),
            Err(NativeOmnigateFixedRouteAdmissionReviewError::OmnigateClaimsIdentityAuthority)
        );

        let mut body_logging = admission_draft(NativeGatewayOmnigateRouteKind::Register);
        body_logging.request_body_logging_disabled = false;
        assert_eq!(
            review_native_omnigate_fixed_route_admission(body_logging),
            Err(NativeOmnigateFixedRouteAdmissionReviewError::RequestOrResponseBodyLoggingEnabled)
        );

        let mut caller_url = admission_draft(NativeGatewayOmnigateRouteKind::Register);
        caller_url.caller_controlled_downstream_url = true;
        assert_eq!(
            review_native_omnigate_fixed_route_admission(caller_url),
            Err(NativeOmnigateFixedRouteAdmissionReviewError::CallerControlledDownstreamUrl)
        );

        let mut live_mount = admission_draft(NativeGatewayOmnigateRouteKind::Register);
        live_mount.requests_live_omnigate_mount = true;
        assert_eq!(
            review_native_omnigate_fixed_route_admission(live_mount),
            Err(NativeOmnigateFixedRouteAdmissionReviewError::LiveRouteMountRequested)
        );

        let mutators: &[fn(&mut NativeOmnigateFixedRouteAdmissionDraftV1)] = &[
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
                review_native_omnigate_fixed_route_admission(draft),
                Err(NativeOmnigateFixedRouteAdmissionReviewError::UnsafeAdmissionAuthorityFlag)
            );
        }
    }

    #[test]
    fn phase14b_source_remains_admission_contract_only_without_live_mounts_storage_wallet_ledger_or_secrets(
    ) {
        let source = fs::read_to_string(repo_file("src/native/omnigate_fixed_route_admission.rs"))
            .expect("omnigate fixed route admission source");
        let route_source = fs::read_to_string(repo_file("src/native/gateway_omnigate_routes.rs"))
            .expect("gateway omnigate route source");
        let native_mod =
            fs::read_to_string(repo_file("src/native/mod.rs")).expect("native module source");

        assert!(source.contains("review_native_omnigate_fixed_route_admission"));
        assert!(source.contains("PHASE14A_GATEWAY_OMNIGATE_ROUTE_CATALOG"));
        assert!(route_source.contains("review_native_gateway_omnigate_route_catalog"));
        assert!(native_mod.contains("OmnigateFixedRouteAdmissionDto"));
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
                "Phase 14B fixed route admission source must not gain runtime pattern {forbidden_runtime_pattern}"
            );
        }
    }
}
