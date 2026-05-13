use ron_naming::{AssetKind, AssetKindParseError};
use std::str::FromStr;

#[test]
fn asset_kind_suffixes_are_stable() {
    let expected = [
        (AssetKind::Image, "image"),
        (AssetKind::Song, "song"),
        (AssetKind::Music, "music"),
        (AssetKind::Podcast, "podcast"),
        (AssetKind::Article, "article"),
        (AssetKind::Post, "post"),
        (AssetKind::Comment, "comment"),
        (AssetKind::Video, "video"),
        (AssetKind::Stream, "stream"),
        (AssetKind::Profile, "profile"),
        (AssetKind::Passport, "passport"),
        (AssetKind::Alt, "alt"),
        (AssetKind::Page, "page"),
        (AssetKind::Site, "site"),
        (AssetKind::App, "app"),
        (AssetKind::Manifest, "manifest"),
    ];

    assert_eq!(AssetKind::ALL.len(), expected.len());

    for (kind, suffix) in expected {
        assert_eq!(kind.suffix(), suffix);
        assert_eq!(kind.as_str(), suffix);
        assert_eq!(kind.to_string(), suffix);
        assert!(kind.is_beta_supported());
        assert_eq!(AssetKind::from_suffix(suffix).unwrap(), kind);
    }
}

#[test]
fn asset_kind_parses_case_insensitively() {
    assert_eq!(AssetKind::from_suffix("IMAGE").unwrap(), AssetKind::Image);
    assert_eq!(AssetKind::from_suffix("Song").unwrap(), AssetKind::Song);
    assert_eq!(AssetKind::from_str("ARTICLE").unwrap(), AssetKind::Article);
    assert_eq!(AssetKind::from_str("PoSt").unwrap(), AssetKind::Post);
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
