use svc_passport::native_plan::{
    current_route, planned_native_v1_route, PassportRoutePosture, CANONICAL_PASSPORT_PACKAGE_OWNER,
    CURRENT_ROUTE_AUTHORITIES, LEGACY_DEV_PASSPORT_SUBJECT, NATIVE_PASSPORT_PHASE0A_LABEL,
    OWNER_FACTS, PLANNED_NATIVE_V1_ROUTE_AUTHORITIES,
};

#[test]
fn phase0a_label_and_canonical_owner_are_locked() {
    assert_eq!(
        NATIVE_PASSPORT_PHASE0A_LABEL,
        "NATIVE_PASSPORT_PHASE0A_SOURCE_AUTHORITY_RECONCILIATION"
    );
    assert_eq!(CANONICAL_PASSPORT_PACKAGE_OWNER, "svc-passport");
    assert_eq!(LEGACY_DEV_PASSPORT_SUBJECT, "passport:main:dev");
}

#[test]
fn current_legacy_routes_do_not_claim_native_v1_authority() {
    assert!(
        !CURRENT_ROUTE_AUTHORITIES.is_empty(),
        "route inventory must not be empty"
    );

    for route in CURRENT_ROUTE_AUTHORITIES {
        assert!(
            !route.native_v1_authority,
            "{} {} is current source posture and must not claim Native Passport V1 authority in Phase 0A",
            route.method,
            route.path
        );
        assert!(
            route.posture != PassportRoutePosture::PlannedNativeV1,
            "current route inventory must not mix planned Native V1 routes"
        );
    }
}

#[test]
fn known_current_routes_are_classified_explicitly() {
    let issue = current_route("POST", "/v1/passport/issue").expect("issue route classified");
    assert_eq!(issue.posture, PassportRoutePosture::CompatibilityOnly);
    assert!(!issue.native_v1_authority);
    assert!(issue.note.contains("legacy token issue path"));

    let profile_claim = current_route("POST", "/v1/passport/profile/claim")
        .expect("profile claim route classified");
    assert_eq!(
        profile_claim.posture,
        PassportRoutePosture::CompatibilityOnly
    );
    assert!(!profile_claim.native_v1_authority);
    assert!(profile_claim.note.contains("proof-gated"));

    let debug =
        current_route("GET", "/v1/passport/profile/_debug").expect("debug route classified");
    assert_eq!(debug.posture, PassportRoutePosture::DevelopmentOnly);
    assert!(!debug.native_v1_authority);

    let rotate = current_route("POST", "/admin/rotate").expect("admin rotate classified");
    assert_eq!(rotate.posture, PassportRoutePosture::AdminOnly);
    assert!(!rotate.native_v1_authority);
}

#[test]
fn native_v1_routes_are_planned_not_current() {
    for path in [
        "/v1/passport/challenge",
        "/v1/passport/prove",
        "/v1/passport/capability/refresh",
        "/v1/passport/capability/revoke",
        "/v1/passport/status/:passport_id",
    ] {
        let method = if path == "/v1/passport/status/:passport_id" {
            "GET"
        } else {
            "POST"
        };

        assert!(
            current_route(method, path).is_none(),
            "{method} {path} must not appear in current-route inventory before implementation"
        );

        let planned =
            planned_native_v1_route(method, path).expect("planned Native Passport V1 route");
        assert_eq!(planned.posture, PassportRoutePosture::PlannedNativeV1);
        assert!(planned.native_v1_authority);
    }

    assert_eq!(PLANNED_NATIVE_V1_ROUTE_AUTHORITIES.len(), 5);
}

#[test]
fn owner_facts_block_duplicate_or_wrong_authority() {
    let svc_passport = OWNER_FACTS
        .iter()
        .find(|fact| fact.owner == "svc-passport")
        .expect("svc-passport owner fact");
    assert!(svc_passport.responsibility.contains("single canonical"));
    assert!(svc_passport.forbidden.contains("new Passport crate"));

    let ron_ledger = OWNER_FACTS
        .iter()
        .find(|fact| fact.owner == "ron-ledger")
        .expect("ron-ledger owner fact");
    assert!(ron_ledger.responsibility.contains("economic truth"));
    assert!(ron_ledger.forbidden.contains("@username"));

    let svc_index = OWNER_FACTS
        .iter()
        .find(|fact| fact.owner == "svc-index")
        .expect("svc-index owner fact");
    assert!(svc_index.responsibility.contains("projections"));
    assert!(svc_index.forbidden.contains("ownership finality"));
}
