//! RO:WHAT — Typed WEB3_2 asset kind vocabulary for b3-backed crab links.
//! RO:WHY — Pillar 9, Concerns: DX/SEC/GOV; keeps asset suffix parsing pure and deterministic.
//! RO:INTERACTS — crab::CrabLink, ron-proto asset DTOs, NEXT_LEVEL text/media/profile asset routes.
//! RO:INVARIANTS — beta asset kinds are explicit; suffixes canonicalize to lowercase; no IO or async.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — rejects unknown asset suffixes fail-closed.
//! RO:TEST — tests/asset_kind.rs and tests/crab_links.rs.

use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

/// WEB3_2 beta asset kinds supported by typed `crab://<hash>.<kind>` links.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetKind {
    /// Image or visual artwork asset.
    Image,
    /// Song or audio asset.
    Song,
    /// Music/audio asset.
    Music,
    /// Podcast/audio-show asset.
    Podcast,
    /// Article or long-form text asset.
    Article,
    /// Short post/feed object asset.
    Post,
    /// Comment or discussion object asset.
    Comment,
    /// Video asset.
    Video,
    /// Live/segmented stream descriptor asset.
    Stream,
    /// Public profile manifest/page asset.
    Profile,
    /// Public/semi-public passport proof page asset.
    Passport,
    /// Pseudonymous alt identity page asset.
    Alt,
    /// Web page or app page asset.
    Page,
    /// Site manifest or root pointer asset.
    Site,
    /// Application/app manifest object.
    App,
    /// Manifest object asset.
    Manifest,
}

impl AssetKind {
    /// All beta-supported WEB3_2 asset kinds in stable order.
    pub const ALL: [AssetKind; 16] = [
        AssetKind::Image,
        AssetKind::Song,
        AssetKind::Music,
        AssetKind::Podcast,
        AssetKind::Article,
        AssetKind::Post,
        AssetKind::Comment,
        AssetKind::Video,
        AssetKind::Stream,
        AssetKind::Profile,
        AssetKind::Passport,
        AssetKind::Alt,
        AssetKind::Page,
        AssetKind::Site,
        AssetKind::App,
        AssetKind::Manifest,
    ];

    /// Return the canonical suffix used in typed crab links.
    #[must_use]
    pub const fn suffix(self) -> &'static str {
        match self {
            AssetKind::Image => "image",
            AssetKind::Song => "song",
            AssetKind::Music => "music",
            AssetKind::Podcast => "podcast",
            AssetKind::Article => "article",
            AssetKind::Post => "post",
            AssetKind::Comment => "comment",
            AssetKind::Video => "video",
            AssetKind::Stream => "stream",
            AssetKind::Profile => "profile",
            AssetKind::Passport => "passport",
            AssetKind::Alt => "alt",
            AssetKind::Page => "page",
            AssetKind::Site => "site",
            AssetKind::App => "app",
            AssetKind::Manifest => "manifest",
        }
    }

    /// Return the canonical string form.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.suffix()
    }

    /// Parse an asset suffix, normalizing ASCII case.
    pub fn from_suffix(input: &str) -> Result<Self, AssetKindParseError> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(AssetKindParseError::Empty);
        }

        match trimmed.to_ascii_lowercase().as_str() {
            "image" => Ok(AssetKind::Image),
            "song" => Ok(AssetKind::Song),
            "music" => Ok(AssetKind::Music),
            "podcast" => Ok(AssetKind::Podcast),
            "article" => Ok(AssetKind::Article),
            "post" => Ok(AssetKind::Post),
            "comment" => Ok(AssetKind::Comment),
            "video" => Ok(AssetKind::Video),
            "stream" => Ok(AssetKind::Stream),
            "profile" => Ok(AssetKind::Profile),
            "passport" => Ok(AssetKind::Passport),
            "alt" => Ok(AssetKind::Alt),
            "page" => Ok(AssetKind::Page),
            "site" => Ok(AssetKind::Site),
            "app" => Ok(AssetKind::App),
            "manifest" => Ok(AssetKind::Manifest),
            other => Err(AssetKindParseError::Unsupported {
                kind: other.to_owned(),
            }),
        }
    }

    /// Return true when the kind is part of the current beta surface.
    #[must_use]
    pub const fn is_beta_supported(self) -> bool {
        true
    }
}

impl fmt::Display for AssetKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.suffix())
    }
}

impl FromStr for AssetKind {
    type Err = AssetKindParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_suffix(s)
    }
}

/// Deterministic parser errors for asset suffixes.
#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq)]
pub enum AssetKindParseError {
    /// Empty suffix.
    #[error("empty asset kind")]
    Empty,
    /// Unknown or unsupported suffix.
    #[error("unsupported asset kind: {kind}")]
    Unsupported {
        /// Unsupported kind after ASCII lowercase normalization.
        kind: String,
    },
}
