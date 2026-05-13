use ron_naming::{normalize_handle, normalize_username, RonUsername, UsernameParseError};

#[test]
fn username_normalizes_handle_and_username() {
    let username = RonUsername::parse("@Skinny.Crabby").unwrap();

    assert_eq!(username.as_str(), "skinny.crabby");
    assert_eq!(username.handle(), "@skinny.crabby");
    assert_eq!(username.crab_url(), "crab://@skinny.crabby");
    assert_eq!(username.to_string(), "skinny.crabby");
    assert_eq!(
        normalize_username("Skinny-Crabby").unwrap(),
        "skinny-crabby"
    );
    assert_eq!(normalize_handle("skinny_crabby").unwrap(), "@skinny_crabby");
}

#[test]
fn username_rejects_reserved_names() {
    let err = RonUsername::parse("@site").unwrap_err();

    assert_eq!(err.code(), "reserved");
    assert!(matches!(
        &err,
        UsernameParseError::Reserved { name } if name == "site"
    ));
}

#[test]
fn username_rejects_confusing_shapes() {
    assert!(matches!(
        RonUsername::parse("ab"),
        Err(UsernameParseError::TooShort { .. })
    ));
    assert!(matches!(
        RonUsername::parse("-alice"),
        Err(UsernameParseError::InvalidStart)
    ));
    assert!(matches!(
        RonUsername::parse("alice_"),
        Err(UsernameParseError::InvalidTrailingPunctuation)
    ));
    assert!(matches!(
        RonUsername::parse("alice..bob"),
        Err(UsernameParseError::ConsecutiveDots)
    ));
    assert!(matches!(
        RonUsername::parse("alice bob"),
        Err(UsernameParseError::InvalidCharacter)
    ));
}
