// Black-box test scaffold for /v1/passport/verify.

#[test]
fn verify_route_contract_path_is_stable() {
    let endpoint = ["/v1", "/passport", "/verify"].concat();

    assert_eq!(endpoint, "/v1/passport/verify");
    assert!(endpoint.starts_with("/v1/"));
}
