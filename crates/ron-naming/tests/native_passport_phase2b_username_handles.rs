use ron_naming::{
    is_handle_v1, is_username_v1, HandleV1, PassportUsernameParseError, UsernameV1, HANDLE_PREFIX,
    NATIVE_PASSPORT_PHASE2B_LABEL, RESERVED_USERNAME_LABELS, USERNAME_MAX_LEN, USERNAME_MIN_LEN,
};

#[test]
fn phase2b_username_constants_are_locked() {
    assert_eq!(
        NATIVE_PASSPORT_PHASE2B_LABEL,
        "NATIVE_PASSPORT_PHASE2B_RON_NAMING_USERNAME_REUSE"
    );
    assert_eq!(HANDLE_PREFIX, "@");
    assert_eq!(USERNAME_MIN_LEN, 3);
    assert_eq!(USERNAME_MAX_LEN, 32);
    assert!(RESERVED_USERNAME_LABELS.contains(&"admin"));
    assert!(RESERVED_USERNAME_LABELS.contains(&"wallet"));
    assert!(RESERVED_USERNAME_LABELS.contains(&"ledger"));
    assert!(RESERVED_USERNAME_LABELS.contains(&"passport"));
}

#[test]
fn usernames_and_handles_normalize_to_canonical_lowercase() {
    let username = UsernameV1::parse("Crab_User7").expect("username parses");
    let handle = HandleV1::parse("@Crab_User7").expect("handle parses");

    assert_eq!(username.as_str(), "crab_user7");
    assert_eq!(username.handle(), "@crab_user7");
    assert_eq!(username.to_string(), "crab_user7");

    assert_eq!(handle.as_str(), "@crab_user7");
    assert_eq!(handle.username(), "crab_user7");
    assert_eq!(handle.to_string(), "@crab_user7");

    assert!(is_username_v1("Crab_User7"));
    assert!(is_handle_v1("@Crab_User7"));
}

#[test]
fn handle_parser_accepts_bare_username_but_stores_at_handle() {
    let handle = HandleV1::parse("creator7").expect("bare username can become handle");

    assert_eq!(handle.as_str(), "@creator7");
    assert_eq!(handle.username(), "creator7");
}

#[test]
fn usernames_reject_reserved_labels_and_authority_looking_names() {
    for reserved in [
        "admin",
        "@admin",
        "root",
        "site",
        "wallet",
        "ledger",
        "passport",
        "device",
        "capability",
        "treasury",
    ] {
        assert_eq!(
            UsernameV1::parse(reserved).unwrap_err(),
            PassportUsernameParseError::Reserved
        );
        assert_eq!(
            HandleV1::parse(reserved).unwrap_err(),
            PassportUsernameParseError::Reserved
        );
    }
}

#[test]
fn usernames_reject_whitespace_legal_name_shapes_and_confusing_separators() {
    assert!(matches!(
        UsernameV1::parse("John Smith"),
        Err(PassportUsernameParseError::InvalidCharacter { ch: ' ' })
    ));
    assert!(matches!(
        UsernameV1::parse("crab-link"),
        Err(PassportUsernameParseError::InvalidCharacter { ch: '-' })
    ));
    assert_eq!(
        UsernameV1::parse("_creator").unwrap_err(),
        PassportUsernameParseError::InvalidBoundary
    );
    assert_eq!(
        UsernameV1::parse("creator_").unwrap_err(),
        PassportUsernameParseError::InvalidBoundary
    );
    assert_eq!(
        UsernameV1::parse("creator__one").unwrap_err(),
        PassportUsernameParseError::ConfusingSeparator
    );
}

#[test]
fn usernames_reject_short_long_and_non_ascii_values() {
    assert_eq!(
        UsernameV1::parse("ab").unwrap_err(),
        PassportUsernameParseError::TooShort {
            actual: 2,
            min: USERNAME_MIN_LEN
        }
    );

    let too_long = "a".repeat(USERNAME_MAX_LEN + 1);
    assert_eq!(
        UsernameV1::parse(&too_long).unwrap_err(),
        PassportUsernameParseError::TooLong {
            actual: USERNAME_MAX_LEN + 1,
            max: USERNAME_MAX_LEN
        }
    );

    assert!(matches!(
        UsernameV1::parse("créator"),
        Err(PassportUsernameParseError::InvalidCharacter { ch: 'é' })
    ));
}
