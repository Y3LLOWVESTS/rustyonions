use ron_proto::ContentId;

#[test]
fn content_id_parses_and_displays() {
    let s = "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let cid: ContentId = s.parse().unwrap();
    assert_eq!(cid.to_string(), s);
    assert_eq!(cid.as_str(), s);
}

#[test]
fn content_id_rejects_bad_prefix() {
    let s = "b2:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    assert!(s.parse::<ContentId>().is_err());
}

#[test]
fn content_id_rejects_bad_len() {
    let s = "b3:0123456789abcdef"; // too short
    assert!(s.parse::<ContentId>().is_err());
}

#[test]
fn content_id_rejects_uppercase() {
    let s = "b3:0123456789ABCDEF0123456789abcdef0123456789abcdef0123456789abcdef";
    assert!(s.parse::<ContentId>().is_err());
}
