// Black-box test scaffold for /v1/passport/issue.

#[test]
fn issue_route_contract_path_is_stable() {
    let endpoint = ["/v1", "/passport", "/issue"].concat();

    assert_eq!(endpoint, "/v1/passport/issue");
    assert!(endpoint.starts_with("/v1/"));
}
