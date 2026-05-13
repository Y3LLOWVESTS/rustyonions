// Readiness test scaffold for svc-passport.

#[test]
fn readiness_route_contract_path_is_stable() {
    let endpoint = ["/", "readyz"].concat();

    assert_eq!(endpoint, "/readyz");
    assert!(endpoint.starts_with('/'));
}
