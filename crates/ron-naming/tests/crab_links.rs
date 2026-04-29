use ron_naming::{AssetKind, CrabLink, CrabNamespace, CrabParseError};

const H: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const H_UPPER: &str = "0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF";

#[test]
fn parses_typed_b3_asset_links_without_b3_path_prefix() {
    let cases = [
        ("image", AssetKind::Image),
        ("song", AssetKind::Song),
        ("article", AssetKind::Article),
        ("video", AssetKind::Video),
        ("comment", AssetKind::Comment),
        ("page", AssetKind::Page),
        ("manifest", AssetKind::Manifest),
    ];

    for (suffix, kind) in cases {
        let link = CrabLink::parse(&format!("crab://{H}.{suffix}")).unwrap();
        assert_eq!(link.scheme(), "crab");
        assert_eq!(link.namespace(), CrabNamespace::B3);
        assert_eq!(link.raw_hash_hex(), Some(H));
        assert_eq!(link.asset_kind(), Some(kind));
        assert_eq!(link.canonical_b3_cid().unwrap().0, format!("b3:{H}"));
        assert_eq!(link.canonical_string(), format!("crab://{H}.{suffix}"));
    }
}

#[test]
fn lowercases_b3_hash_and_asset_suffix_for_canonical_string() {
    let link = CrabLink::parse(&format!("crab://{H_UPPER}.IMAGE")).unwrap();

    assert_eq!(link.raw_hash_hex(), Some(H));
    assert_eq!(link.asset_kind(), Some(AssetKind::Image));
    assert_eq!(link.canonical_b3_cid().unwrap().0, format!("b3:{H}"));
    assert_eq!(link.canonical_string(), format!("crab://{H}.image"));
}

#[test]
fn parses_named_site_link() {
    let link = CrabLink::parse("crab://site/SeaLobsta.COM").unwrap();

    assert_eq!(link.namespace(), CrabNamespace::Site);
    assert_eq!(link.name().unwrap().0, "sealobsta.com");
    assert_eq!(link.canonical_string(), "crab://site/sealobsta.com");
}

#[test]
fn parses_explicit_name_link() {
    let link = CrabLink::parse("crab://name/Café.Example").unwrap();

    assert_eq!(link.namespace(), CrabNamespace::Name);
    assert_eq!(link.name().unwrap().0, "xn--caf-dma.example");
    assert_eq!(link.canonical_string(), "crab://name/xn--caf-dma.example");
}

#[test]
fn parses_root_name_link() {
    let link = CrabLink::parse("crab://Sealobsta").unwrap();

    assert_eq!(link.namespace(), CrabNamespace::RootName);
    assert_eq!(link.name().unwrap().0, "sealobsta");
    assert_eq!(link.canonical_string(), "crab://sealobsta");
}

#[test]
fn preserves_safe_query_parameters_in_sorted_canonical_order() {
    let link = CrabLink::parse(&format!("crab://{H}.image?z=2&a=1&receipt=tx:abc")).unwrap();

    assert_eq!(link.query_params().get("a").map(String::as_str), Some("1"));
    assert_eq!(link.query_params().get("z").map(String::as_str), Some("2"));
    assert_eq!(
        link.query_params().get("receipt").map(String::as_str),
        Some("tx:abc")
    );
    assert_eq!(
        link.canonical_string(),
        format!("crab://{H}.image?a=1&receipt=tx:abc&z=2")
    );
}

#[test]
fn normalizes_percent_encoding_in_query() {
    let link = CrabLink::parse("crab://site/sealobsta?tag=hello%2fworld").unwrap();

    assert_eq!(
        link.query_params().get("tag").map(String::as_str),
        Some("hello%2Fworld")
    );
    assert_eq!(
        link.canonical_string(),
        "crab://site/sealobsta?tag=hello%2Fworld"
    );
}

#[test]
fn rejects_invalid_hash_length_when_it_is_not_name_like() {
    let err = CrabLink::parse("crab://abc.image").unwrap();

    assert_eq!(err.namespace(), CrabNamespace::RootName);
    assert_eq!(err.name().unwrap().0, "abc.image");
}

#[test]
fn rejects_invalid_hash_characters_for_64_char_asset_target() {
    let bad = "g123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let err = CrabLink::parse(&format!("crab://{bad}.image")).unwrap_err();

    assert!(matches!(
        err,
        CrabParseError::InvalidHash {
            reason: "hash contains non-hex characters"
        }
    ));
}

#[test]
fn rejects_unknown_asset_kind() {
    let err = CrabLink::parse(&format!("crab://{H}.binary")).unwrap_err();

    assert!(matches!(
        err,
        CrabParseError::InvalidAssetKind { kind } if kind == "binary"
    ));
}

#[test]
fn rejects_missing_asset_kind_for_bare_hash() {
    let err = CrabLink::parse(&format!("crab://{H}")).unwrap_err();

    assert!(matches!(err, CrabParseError::MissingAssetKind));
}

#[test]
fn rejects_embedded_credentials() {
    let err = CrabLink::parse("crab://user@site/sealobsta").unwrap_err();

    assert!(matches!(err, CrabParseError::EmbeddedCredentials));
}

#[test]
fn rejects_path_traversal() {
    let err = CrabLink::parse("crab://site/../sealobsta").unwrap_err();

    assert!(matches!(err, CrabParseError::PathTraversal));
}

#[test]
fn rejects_encoded_path_traversal() {
    let err = CrabLink::parse("crab://site/%2e%2e").unwrap_err();

    assert!(matches!(err, CrabParseError::PathTraversal));
}

#[test]
fn rejects_unsafe_control_characters() {
    let err = CrabLink::parse("crab://site/sealobsta\n").unwrap_err();

    assert!(matches!(err, CrabParseError::UnsafeControlCharacter));
}

#[test]
fn rejects_unsupported_namespace_with_path() {
    let err = CrabLink::parse("crab://pay/acct_creator").unwrap_err();

    assert!(matches!(
        err,
        CrabParseError::UnsupportedNamespace { namespace } if namespace == "pay"
    ));
}

#[test]
fn rejects_old_b3_slash_prefix_as_noncanonical() {
    let err = CrabLink::parse(&format!("crab://b3/{H}.image")).unwrap_err();

    assert!(matches!(
        err,
        CrabParseError::UnsupportedNamespace { namespace } if namespace == "b3"
    ));
}

#[test]
fn rejects_duplicate_query_keys() {
    let err = CrabLink::parse("crab://site/sealobsta?a=1&a=2").unwrap_err();

    assert!(matches!(
        err,
        CrabParseError::DuplicateQueryKey { key } if key == "a"
    ));
}

#[test]
fn deterministic_error_codes_are_stable() {
    let err = CrabLink::parse("https://example.com").unwrap_err();

    assert_eq!(err.code(), "invalid_scheme");
}
