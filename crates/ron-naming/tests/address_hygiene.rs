use ron_naming::{normalize::normalize_fqdn_ascii, Address};

#[test]
fn parse_b3_ok() {
    let addr =
        Address::parse("b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")
            .unwrap();
    assert!(matches!(addr, Address::Content { .. }));
}

#[test]
fn parse_name_with_version() {
    let addr = Address::parse("files.example@1.2.3").unwrap();
    match addr {
        Address::Name { fqdn, version } => {
            assert_eq!(fqdn.0, "files.example");
            assert_eq!(version.unwrap().0.to_string(), "1.2.3");
        }
        _ => panic!("expected name"),
    }
}

#[test]
fn normalize_then_parse() {
    let nf = normalize_fqdn_ascii(" Café.EXAMPLE ").unwrap();
    let addr = Address::parse(&nf.0 .0).unwrap();
    matches!(addr, Address::Name { .. });
}
