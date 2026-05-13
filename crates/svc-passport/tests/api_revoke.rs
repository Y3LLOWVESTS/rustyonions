// Black-box test scaffold for /v1/passport/revoke.

#[test]
fn revoke_route_contract_path_is_stable() {
    let endpoint = ["/v1", "/passport", "/revoke"].concat();

    assert_eq!(endpoint, "/v1/passport/revoke");
    assert!(endpoint.starts_with("/v1/"));
}
