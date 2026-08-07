// RO:WHAT — Canonical svc-index creator-publication read projection and bounded page model.
// RO:WHY — FINAL_BETA Phase 6 needs one backend-derived creator timeline projection for profiles, Home, Explore, and reviewed templates.
// RO:INTERACTS — store::Store, store::keys, serde JSON records, and later gateway and omnigate read adapters.
// RO:INVARIANTS — public read projection only; unknown wire fields reject; pages are bounded; non-public records never appear in creator timelines.
// RO:SECURITY — no wallet, ledger, receipt, entitlement, follow, settlement, key, PIN, recovery, or capability authority.
// RO:TEST — final_beta_phase6_creator_publication_projection.rs.

use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};

pub const FINAL_BETA_PHASE6B1_SVC_INDEX_CREATOR_PROJECTION: &str =
    "FINAL_BETA_PHASE6B1_SVC_INDEX_CREATOR_PROJECTION_V1";

pub const PUBLICATION_SUMMARY_SCHEMA: &str =
    "crablink.publication-summary.v1";

pub const PUBLICATION_PAGE_SCHEMA: &str =
    "crablink.publication-page.v1";

pub const PUBLICATION_PAGE_DEFAULT_LIMIT: usize = 20;
pub const PUBLICATION_PAGE_MAX_LIMIT: usize = 50;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PublicationKind {
    Post,
    Article,
    Image,
    Video,
    Audio,
    Podcast,
    Music,
    Lyrics,
    Code,
    Game,
    Site,
    Stream,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PublicationVisibility {
    Public,
    Unlisted,
    Private,
    Deleted,
    Blocked,
    Moderated,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PublicationAccess {
    Free,
    Paid,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PublicationThumbnailKind {
    Image,
    Video,
    Audio,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PublicationCreatorV1 {
    pub username: String,
    pub display_name: String,
    pub profile_url: String,

    #[serde(default)]
    pub avatar_cid: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PublicationThumbnailV1 {
    pub kind: PublicationThumbnailKind,
    pub cid: String,
    pub alt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PublicationReferencesV1 {
    #[serde(default)]
    pub manifest_cid: Option<String>,

    #[serde(default)]
    pub content_cid: Option<String>,

    #[serde(default)]
    pub site_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PublicationSummaryV1 {
    pub schema: String,
    pub publication_id: String,
    pub kind: PublicationKind,
    pub crab_url: String,

    #[serde(default)]
    pub title: String,

    #[serde(default)]
    pub summary: String,

    pub creator: PublicationCreatorV1,
    pub published_at: String,
    pub updated_at: String,
    pub visibility: PublicationVisibility,
    pub access: PublicationAccess,

    #[serde(default)]
    pub thumbnail: Option<PublicationThumbnailV1>,

    #[serde(default)]
    pub references: Option<PublicationReferencesV1>,

    #[serde(default)]
    pub pinned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PublicationPageV1 {
    pub schema: String,
    pub items: Vec<PublicationSummaryV1>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicationPageRequest {
    pub cursor: Option<String>,
    pub limit: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublicationProjectionError {
    InvalidField {
        field: &'static str,
        reason: &'static str,
    },
    InvalidCursor,
    CursorOutOfRange,
}

impl fmt::Display for PublicationProjectionError {
    fn fmt(
        &self,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            Self::InvalidField { field, reason } => {
                write!(
                    formatter,
                    "invalid {field}: {reason}",
                )
            }
            Self::InvalidCursor => {
                formatter.write_str(
                    "invalid publication cursor",
                )
            }
            Self::CursorOutOfRange => {
                formatter.write_str(
                    "publication cursor is out of range",
                )
            }
        }
    }
}

impl Error for PublicationProjectionError {}

impl PublicationSummaryV1 {
    pub fn validate(
        &self,
    ) -> Result<(), PublicationProjectionError> {
        validate_exact(
            "schema",
            &self.schema,
            PUBLICATION_SUMMARY_SCHEMA,
        )?;

        validate_publication_id(
            &self.publication_id,
        )?;

        validate_crab_url(
            "crab_url",
            &self.crab_url,
        )?;

        validate_optional_text(
            "title",
            &self.title,
            160,
        )?;

        validate_optional_text(
            "summary",
            &self.summary,
            500,
        )?;

        if self.title.trim().is_empty()
            && self.summary.trim().is_empty()
        {
            return Err(invalid(
                "title",
                "title or summary is required",
            ));
        }

        validate_username(
            &self.creator.username,
        )?;

        validate_required_text(
            "display_name",
            &self.creator.display_name,
            80,
        )?;

        validate_crab_url(
            "profile_url",
            &self.creator.profile_url,
        )?;

        let expected_profile =
            format!(
                "crab://@{}",
                self.creator.username,
            );

        if self.creator.profile_url != expected_profile {
            return Err(invalid(
                "profile_url",
                "must match creator username",
            ));
        }

        if let Some(avatar_cid) =
            self.creator.avatar_cid.as_deref()
        {
            validate_b3_cid(
                "avatar_cid",
                avatar_cid,
            )?;
        }

        validate_timestamp(
            "published_at",
            &self.published_at,
        )?;

        validate_timestamp(
            "updated_at",
            &self.updated_at,
        )?;

        if self.updated_at < self.published_at {
            return Err(invalid(
                "updated_at",
                "must not precede published_at",
            ));
        }

        if let Some(thumbnail) = self.thumbnail.as_ref() {
            validate_b3_cid(
                "thumbnail.cid",
                &thumbnail.cid,
            )?;

            validate_required_text(
                "thumbnail.alt",
                &thumbnail.alt,
                180,
            )?;
        }

        if let Some(references) = self.references.as_ref() {
            let has_manifest =
                references.manifest_cid.is_some();

            let has_content =
                references.content_cid.is_some();

            let has_site =
                references.site_url.is_some();

            if !(
                has_manifest
                    || has_content
                    || has_site
            ) {
                return Err(invalid(
                    "references",
                    "at least one reference is required",
                ));
            }

            if let Some(cid) =
                references.manifest_cid.as_deref()
            {
                validate_b3_cid(
                    "references.manifest_cid",
                    cid,
                )?;
            }

            if let Some(cid) =
                references.content_cid.as_deref()
            {
                validate_b3_cid(
                    "references.content_cid",
                    cid,
                )?;
            }

            if let Some(site_url) =
                references.site_url.as_deref()
            {
                validate_crab_url(
                    "references.site_url",
                    site_url,
                )?;
            }
        }

        Ok(())
    }

    pub fn is_public_timeline_item(
        &self,
    ) -> bool {
        self.visibility
            == PublicationVisibility::Public
    }
}

impl PublicationPageRequest {
    pub fn new(
        cursor: Option<String>,
        limit: Option<usize>,
    ) -> Result<Self, PublicationProjectionError> {
        let limit =
            limit.unwrap_or(
                PUBLICATION_PAGE_DEFAULT_LIMIT,
            );

        if !(
            1..=PUBLICATION_PAGE_MAX_LIMIT
        )
        .contains(&limit)
        {
            return Err(invalid(
                "limit",
                "must be from 1 through 50",
            ));
        }

        if let Some(value) = cursor.as_deref() {
            decode_cursor(value)?;
        }

        Ok(Self {
            cursor,
            limit,
        })
    }

    pub fn offset(
        &self,
    ) -> Result<usize, PublicationProjectionError> {
        match self.cursor.as_deref() {
            Some(cursor) => decode_cursor(cursor),
            None => Ok(0),
        }
    }
}

pub fn build_publication_page(
    mut items: Vec<PublicationSummaryV1>,
    request: &PublicationPageRequest,
) -> Result<PublicationPageV1, PublicationProjectionError> {
    items.retain(
        |item| {
            item.validate().is_ok()
                && item.is_public_timeline_item()
        },
    );

    items.sort_by(
        |left, right| {
            right
                .pinned
                .cmp(&left.pinned)
                .then_with(
                    || {
                        right
                            .published_at
                            .cmp(&left.published_at)
                    },
                )
                .then_with(
                    || {
                        left
                            .publication_id
                            .cmp(&right.publication_id)
                    },
                )
        },
    );

    let offset = request.offset()?;

    if offset > items.len() {
        return Err(
            PublicationProjectionError::CursorOutOfRange,
        );
    }

    let end =
        offset
            .saturating_add(request.limit)
            .min(items.len());

    let has_more = end < items.len();
    let page_items = items[offset..end].to_vec();

    let next_cursor =
        if has_more {
            Some(encode_cursor(end))
        } else {
            None
        };

    Ok(PublicationPageV1 {
        schema:
            PUBLICATION_PAGE_SCHEMA.to_owned(),
        items:
            page_items,
        next_cursor,
        has_more,
    })
}

pub fn normalize_username(
    input: &str,
) -> Result<String, PublicationProjectionError> {
    let username =
        input.trim().to_ascii_lowercase();

    validate_username(
        &username,
    )?;

    Ok(username)
}

pub fn normalize_publication_id(
    input: &str,
) -> Result<String, PublicationProjectionError> {
    let publication_id =
        input.trim().to_owned();

    validate_publication_id(
        &publication_id,
    )?;

    Ok(publication_id)
}

fn validate_username(
    value: &str,
) -> Result<(), PublicationProjectionError> {
    let bytes = value.as_bytes();

    if !(3..=32).contains(&bytes.len()) {
        return Err(invalid(
            "username",
            "must contain 3 through 32 characters",
        ));
    }

    if !(
        bytes[0].is_ascii_lowercase()
            || bytes[0].is_ascii_digit()
    ) {
        return Err(invalid(
            "username",
            "must begin with lowercase ASCII or a digit",
        ));
    }

    if bytes.iter().all(
        |byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || *byte == b'_'
                || *byte == b'-'
        },
    ) {
        Ok(())
    } else {
        Err(invalid(
            "username",
            "contains unsupported characters",
        ))
    }
}

fn validate_publication_id(
    value: &str,
) -> Result<(), PublicationProjectionError> {
    let bytes = value.as_bytes();

    if !(1..=128).contains(&bytes.len()) {
        return Err(invalid(
            "publication_id",
            "must contain 1 through 128 characters",
        ));
    }

    if !bytes[0].is_ascii_alphanumeric() {
        return Err(invalid(
            "publication_id",
            "must begin with ASCII alphanumeric",
        ));
    }

    if bytes.iter().all(
        |byte| {
            byte.is_ascii_alphanumeric()
                || matches!(
                    *byte,
                    b'.' | b'_' | b':' | b'-',
                )
        },
    ) {
        Ok(())
    } else {
        Err(invalid(
            "publication_id",
            "contains unsupported characters",
        ))
    }
}

fn validate_exact(
    field: &'static str,
    value: &str,
    expected: &'static str,
) -> Result<(), PublicationProjectionError> {
    if value == expected {
        Ok(())
    } else {
        Err(invalid(
            field,
            "unexpected schema",
        ))
    }
}

fn validate_required_text(
    field: &'static str,
    value: &str,
    max: usize,
) -> Result<(), PublicationProjectionError> {
    validate_optional_text(
        field,
        value,
        max,
    )?;

    if value.trim().is_empty() {
        Err(invalid(
            field,
            "must not be empty",
        ))
    } else {
        Ok(())
    }
}

fn validate_optional_text(
    field: &'static str,
    value: &str,
    max: usize,
) -> Result<(), PublicationProjectionError> {
    if value.chars().any(char::is_control) {
        return Err(invalid(
            field,
            "contains control characters",
        ));
    }

    if value.chars().count() <= max {
        Ok(())
    } else {
        Err(invalid(
            field,
            "exceeds its maximum length",
        ))
    }
}

fn validate_crab_url(
    field: &'static str,
    value: &str,
) -> Result<(), PublicationProjectionError> {
    validate_required_text(
        field,
        value,
        1024,
    )?;

    if value.starts_with("crab://") {
        Ok(())
    } else {
        Err(invalid(
            field,
            "must use crab://",
        ))
    }
}

fn validate_b3_cid(
    field: &'static str,
    value: &str,
) -> Result<(), PublicationProjectionError> {
    let raw =
        value
            .strip_prefix("b3:")
            .unwrap_or("");

    if raw.len() == 64
        && raw.bytes().all(
            |byte| {
                byte.is_ascii_digit()
                    || matches!(
                        byte,
                        b'a'..=b'f',
                    )
            },
        )
    {
        Ok(())
    } else {
        Err(invalid(
            field,
            "must be a canonical b3 CID",
        ))
    }
}

fn validate_timestamp(
    field: &'static str,
    value: &str,
) -> Result<(), PublicationProjectionError> {
    let bytes = value.as_bytes();

    if bytes.len() != 24 {
        return Err(invalid(
            field,
            "must be canonical UTC milliseconds",
        ));
    }

    let separators = [
        (4, b'-'),
        (7, b'-'),
        (10, b'T'),
        (13, b':'),
        (16, b':'),
        (19, b'.'),
        (23, b'Z'),
    ];

    for (index, expected) in separators {
        if bytes[index] != expected {
            return Err(invalid(
                field,
                "must be canonical UTC milliseconds",
            ));
        }
    }

    for (index, byte) in bytes.iter().enumerate() {
        if matches!(
            index,
            4 | 7 | 10 | 13 | 16 | 19 | 23,
        ) {
            continue;
        }

        if !byte.is_ascii_digit() {
            return Err(invalid(
                field,
                "must contain numeric UTC components",
            ));
        }
    }

    let year =
        parse_component(value, 0, 4, field)?;

    let month =
        parse_component(value, 5, 7, field)?;

    let day =
        parse_component(value, 8, 10, field)?;

    let hour =
        parse_component(value, 11, 13, field)?;

    let minute =
        parse_component(value, 14, 16, field)?;

    let second =
        parse_component(value, 17, 19, field)?;

    if year >= 1970
        && (1..=12).contains(&month)
        && day >= 1
        && day <= days_in_month(year, month)
        && hour <= 23
        && minute <= 59
        && second <= 59
    {
        Ok(())
    } else {
        Err(invalid(
            field,
            "contains an invalid UTC calendar component",
        ))
    }
}

fn parse_component(
    value: &str,
    start: usize,
    end: usize,
    field: &'static str,
) -> Result<u32, PublicationProjectionError> {
    value[start..end]
        .parse::<u32>()
        .map_err(
            |_| {
                invalid(
                    field,
                    "contains an invalid numeric component",
                )
            },
        )
}

fn days_in_month(
    year: u32,
    month: u32,
) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(
    year: u32,
) -> bool {
    (
        year % 4 == 0
            && year % 100 > 0
    ) || year % 400 == 0
}

fn encode_cursor(
    offset: usize,
) -> String {
    format!(
        "p_{offset:08x}",
    )
}

fn decode_cursor(
    cursor: &str,
) -> Result<usize, PublicationProjectionError> {
    if cursor.len() == 10
        && cursor.starts_with("p_")
    {
        usize::from_str_radix(
            &cursor[2..],
            16,
        )
        .map_err(
            |_| {
                PublicationProjectionError::InvalidCursor
            },
        )
    } else {
        Err(
            PublicationProjectionError::InvalidCursor,
        )
    }
}

fn invalid(
    field: &'static str,
    reason: &'static str,
) -> PublicationProjectionError {
    PublicationProjectionError::InvalidField {
        field,
        reason,
    }
}
