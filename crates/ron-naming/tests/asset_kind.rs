use ron_naming::{AssetKind, AssetKindParseError};
use std::str::FromStr;

#[test]
fn asset_kind_suffixes_are_stable() {
    let expected = [
        (AssetKind::Image, "image"),
        (AssetKind::Song, "song"),
        (AssetKind::Article, "article"),
        (AssetKind::Video, "video"),
        (AssetKind::Comment, "comment"),
        (AssetKind::Page, "page"),
        (AssetKind::Manifest, "manifest"),
    ];

    for (kind, suffix) in expected {
        assert_eq!(kind.suffix(), suffix);
        assert_eq!(kind.as_str(), suffix);
        assert_eq!(kind.to_string(), suffix);
        assert!(kind.is_beta_supported());
    }
}

#[test]
fn asset_kind_parses_case_insensitively() {
    assert_eq!(AssetKind::from_suffix("IMAGE").unwrap(), AssetKind::Image);
    assert_eq!(AssetKind::from_suffix("Song").unwrap(), AssetKind::Song);
    assert_eq!(AssetKind::from_str("ARTICLE").unwrap(), AssetKind::Article);
}

#[test]
fn asset_kind_rejects_empty() {
    assert!(matches!(
        AssetKind::from_suffix(""),
        Err(AssetKindParseError::Empty)
    ));
}

#[test]
fn asset_kind_rejects_unknown() {
    assert!(matches!(
        AssetKind::from_suffix("binary"),
        Err(AssetKindParseError::Unsupported { kind }) if kind == "binary"
    ));
}
