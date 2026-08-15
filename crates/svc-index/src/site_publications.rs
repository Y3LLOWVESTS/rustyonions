//! RO:WHAT — Strict Site-keyed root-publication projection for public Site content.
//! RO:WHY — FINAL_BETA Phase 15 needs durable Forum thread discovery without abusing Comment relations or creator-profile authority.
//! RO:INTERACTS — PublicationReferencesV1, PublicationRelationVisibility, Store, svc-index Site-publication HTTP route, later Omnigate root publication writes.
//! RO:INVARIANTS — only root publication kinds are accepted; exact named Site context required; tags remain bounded; private/unlisted records are omitted from public pages.
//! RO:SECURITY — display/index metadata only; creatorDisplay is not verified identity; no wallet, ledger, receipt, entitlement, moderation mutation, QuickChain, ROX, or Solana authority.
//! RO:TEST — final_beta_phase15_site_publication_roots.rs.

use std::{
    error::Error,
    fmt,
};

use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    publications::PublicationReferencesV1,

    relations::{
        normalize_relation_site_crab_url,
        PublicationRelationVisibility,
    },
};

pub const SITE_PUBLICATION_SCHEMA:
    &str =
    "crablink.site-publication.v1";

pub const SITE_PUBLICATION_PAGE_SCHEMA:
    &str =
    "crablink.site-publication-page.v1";

pub const SITE_PUBLICATION_PAGE_DEFAULT_LIMIT:
    usize =
    20;

pub const SITE_PUBLICATION_PAGE_MAX_LIMIT:
    usize =
    100;

pub const SITE_PUBLICATION_MAX_TAGS:
    usize =
    32;

pub const SITE_PUBLICATION_MAX_TAG_LENGTH:
    usize =
    128;

#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
)]
#[serde(
    rename_all = "lowercase"
)]
pub enum SitePublicationKind {
    Post,
    Image,
    Article,
}

impl SitePublicationKind {
    #[must_use]
    pub const fn suffix(
        self,
    ) -> &'static str {
        match self {
            Self::Post =>
                "post",

            Self::Image =>
                "image",

            Self::Article =>
                "article",
        }
    }
}

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
)]
#[serde(
    rename_all = "camelCase",
    deny_unknown_fields
)]
pub struct SitePublicationV1 {
    pub schema:
        String,

    pub publication_id:
        String,

    pub kind:
        SitePublicationKind,

    pub crab_url:
        String,

    #[serde(default)]
    pub title:
        String,

    #[serde(default)]
    pub summary:
        String,

    /// Display-only author text carried from the publication request.
    ///
    /// This is intentionally not a verified username, profile URL, Passport
    /// ownership statement, or creator authority claim.
    #[serde(default)]
    pub creator_display:
        Option<String>,

    pub created_at_ms:
        u64,

    pub visibility:
        PublicationRelationVisibility,

    pub references:
        PublicationReferencesV1,

    /// Bounded publication taxonomy copied from the already-published
    /// manifest/request projection. Forum category tags use this field.
    #[serde(default)]
    pub tags:
        Vec<String>,

    pub site_crab_url:
        String,
}

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
)]
#[serde(
    rename_all = "camelCase",
    deny_unknown_fields
)]
pub struct SitePublicationPageV1 {
    pub schema:
        String,

    pub items:
        Vec<SitePublicationV1>,

    pub next_cursor:
        Option<String>,

    pub has_more:
        bool,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
)]
pub struct SitePublicationPageRequest {
    pub cursor:
        Option<String>,

    pub limit:
        usize,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
)]
pub enum SitePublicationError {
    InvalidField {
        field:
            &'static str,

        reason:
            &'static str,
    },

    InvalidCursor,

    CursorOutOfRange,
}

impl fmt::Display
    for SitePublicationError
{
    fn fmt(
        &self,
        formatter:
            &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            Self::InvalidField {
                field,
                reason,
            } => {
                write!(
                    formatter,
                    "invalid {field}: {reason}",
                )
            }

            Self::InvalidCursor => {
                formatter
                    .write_str(
                        "invalid Site publication cursor",
                    )
            }

            Self::CursorOutOfRange => {
                formatter
                    .write_str(
                        "Site publication cursor is out of range",
                    )
            }
        }
    }
}

impl Error
    for SitePublicationError
{
}

impl SitePublicationV1 {
    pub fn validate(
        &self,
    ) -> Result<
        (),
        SitePublicationError,
    > {
        if self.schema !=
            SITE_PUBLICATION_SCHEMA
        {
            return Err(
                invalid(
                    "schema",
                    "unsupported Site publication schema",
                ),
            );
        }

        validate_publication_id(
            &self.publication_id,
        )?;

        validate_root_crab_url(
            &self.crab_url,
            self.kind,
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

        if self.title
            .trim()
            .is_empty()
            &&
            self.summary
                .trim()
                .is_empty()
        {
            return Err(
                invalid(
                    "title",
                    "title or summary is required",
                ),
            );
        }

        if let Some(
            creator_display,
        ) =
            self.creator_display
                .as_deref()
        {
            validate_required_text(
                "creatorDisplay",
                creator_display,
                80,
            )?;
        }

        if self.created_at_ms == 0
        {
            return Err(
                invalid(
                    "createdAtMs",
                    "must be greater than zero",
                ),
            );
        }

        if self.tags.len() >
            SITE_PUBLICATION_MAX_TAGS
        {
            return Err(
                invalid(
                    "tags",
                    "too many tags",
                ),
            );
        }

        for tag
            in &self.tags
        {
            validate_tag(
                tag,
            )?;
        }

        let site =
            normalize_site_publication_site_crab_url(
                &self.site_crab_url,
            )?;

        let manifest_cid =
            self.references
                .manifest_cid
                .as_deref()
                .ok_or_else(
                    || {
                        invalid(
                            "references.manifestCid",
                            "manifest CID is required",
                        )
                    },
                )?;

        validate_b3_cid(
            "references.manifestCid",
            manifest_cid,
        )?;

        let content_cid =
            self.references
                .content_cid
                .as_deref()
                .ok_or_else(
                    || {
                        invalid(
                            "references.contentCid",
                            "content CID is required",
                        )
                    },
                )?;

        validate_b3_cid(
            "references.contentCid",
            content_cid,
        )?;

        let reference_site =
            self.references
                .site_url
                .as_deref()
                .ok_or_else(
                    || {
                        invalid(
                            "references.siteUrl",
                            "Site reference is required",
                        )
                    },
                )?;

        let reference_site =
            normalize_site_publication_site_crab_url(
                reference_site,
            )?;

        if reference_site !=
            site
        {
            return Err(
                invalid(
                    "siteCrabUrl",
                    "Site projection does not match publication Site reference",
                ),
            );
        }

        Ok(())
    }

    #[must_use]
    pub fn is_public_site_item(
        &self,
    ) -> bool {
        matches!(
            self.visibility,
            PublicationRelationVisibility::Public
                | PublicationRelationVisibility::PublicPreview
                | PublicationRelationVisibility::Deleted
                | PublicationRelationVisibility::Blocked
                | PublicationRelationVisibility::Moderated
        )
    }
}

impl SitePublicationPageRequest {
    pub fn new(
        cursor:
            Option<String>,

        limit:
            Option<usize>,
    ) -> Result<
        Self,
        SitePublicationError,
    > {
        let limit =
            limit
                .unwrap_or(
                    SITE_PUBLICATION_PAGE_DEFAULT_LIMIT,
                );

        if !(
            1..=
            SITE_PUBLICATION_PAGE_MAX_LIMIT
        )
            .contains(
                &limit,
            )
        {
            return Err(
                invalid(
                    "limit",
                    "must be within Site publication page bounds",
                ),
            );
        }

        if let Some(
            cursor,
        ) =
            cursor
                .as_deref()
        {
            decode_cursor(
                cursor,
            )?;
        }

        Ok(
            Self {
                cursor,
                limit,
            },
        )
    }
}

pub fn build_site_publication_page(
    publications:
        Vec<SitePublicationV1>,

    request:
        &SitePublicationPageRequest,
) -> Result<
    SitePublicationPageV1,
    SitePublicationError,
> {
    let mut publications =
        publications
            .into_iter()
            .filter(
                |publication| {
                    publication
                        .validate()
                        .is_ok()
                },
            )
            .filter(
                SitePublicationV1::
                    is_public_site_item,
            )
            .collect::<Vec<_>>();

    publications
        .sort_by(
            |left, right| {
                right
                    .created_at_ms
                    .cmp(
                        &left
                            .created_at_ms,
                    )
                    .then_with(
                        || {
                            left
                                .publication_id
                                .cmp(
                                    &right
                                        .publication_id,
                                )
                        },
                    )
            },
        );

    let offset =
        match request
            .cursor
            .as_deref()
        {
            Some(
                cursor,
            ) => {
                decode_cursor(
                    cursor,
                )?
            }

            None =>
                0,
        };

    if offset >
        publications.len()
    {
        return Err(
            SitePublicationError::
                CursorOutOfRange,
        );
    }

    let end =
        offset
            .saturating_add(
                request.limit,
            )
            .min(
                publications.len(),
            );

    let items =
        publications[
            offset..end
        ]
            .to_vec();

    let has_more =
        end <
        publications.len();

    let next_cursor =
        if has_more {
            Some(
                encode_cursor(
                    end,
                ),
            )
        } else {
            None
        };

    Ok(
        SitePublicationPageV1 {
            schema:
                SITE_PUBLICATION_PAGE_SCHEMA
                    .to_owned(),

            items,

            next_cursor,

            has_more,
        },
    )
}

pub fn normalize_site_publication_site_crab_url(
    value:
        &str,
) -> Result<
    String,
    SitePublicationError,
> {
    normalize_relation_site_crab_url(
        value,
    )
    .map_err(
        |_| {
            invalid(
                "siteCrabUrl",
                "must be an exact named crab:// Site URL",
            )
        },
    )
}

fn validate_publication_id(
    value:
        &str,
) -> Result<
    (),
    SitePublicationError,
> {
    let value =
        value.trim();

    if value.is_empty()
        ||
        value.len() > 128
    {
        return Err(
            invalid(
                "publicationId",
                "must contain 1 to 128 bytes",
            ),
        );
    }

    if value
        .bytes()
        .all(
            |byte| {
                byte
                    .is_ascii_alphanumeric()
                    ||
                    matches!(
                        byte,
                        b'_'
                            | b'-'
                            | b'.'
                            | b':'
                    )
            },
        )
    {
        Ok(())
    } else {
        Err(
            invalid(
                "publicationId",
                "contains unsupported characters",
            ),
        )
    }
}

fn validate_root_crab_url(
    value:
        &str,

    kind:
        SitePublicationKind,
) -> Result<
    (),
    SitePublicationError,
> {
    let Some(
        remainder,
    ) =
        value
            .strip_prefix(
                "crab://",
            )
    else {
        return Err(
            invalid(
                "crabUrl",
                "must use crab://",
            ),
        );
    };

    let Some(
        (
            hash,
            suffix,
        ),
    ) =
        remainder
            .rsplit_once(
                '.',
            )
    else {
        return Err(
            invalid(
                "crabUrl",
                "must be a typed root publication URL",
            ),
        );
    };

    if hash.len() != 64
        ||
        !hash
            .bytes()
            .all(
                |byte| {
                    byte
                        .is_ascii_hexdigit()
                        &&
                        (
                            byte
                                .is_ascii_digit()
                            ||
                            byte
                                .is_ascii_lowercase()
                        )
                },
            )
    {
        return Err(
            invalid(
                "crabUrl",
                "must contain a canonical lowercase 64-hex hash",
            ),
        );
    }

    if suffix !=
        kind.suffix()
    {
        return Err(
            invalid(
                "crabUrl",
                "typed URL suffix does not match root publication kind",
            ),
        );
    }

    Ok(())
}

fn validate_b3_cid(
    field:
        &'static str,

    value:
        &str,
) -> Result<
    (),
    SitePublicationError,
> {
    let Some(
        hash,
    ) =
        value
            .strip_prefix(
                "b3:",
            )
    else {
        return Err(
            invalid(
                field,
                "must use b3:<64 lowercase hex>",
            ),
        );
    };

    if hash.len() == 64
        &&
        hash
            .bytes()
            .all(
                |byte| {
                    byte
                        .is_ascii_hexdigit()
                        &&
                        (
                            byte
                                .is_ascii_digit()
                            ||
                            byte
                                .is_ascii_lowercase()
                        )
                },
            )
    {
        Ok(())
    } else {
        Err(
            invalid(
                field,
                "must use b3:<64 lowercase hex>",
            ),
        )
    }
}

fn validate_optional_text(
    field:
        &'static str,

    value:
        &str,

    max:
        usize,
) -> Result<
    (),
    SitePublicationError,
> {
    if value
        .chars()
        .count()
        <= max
        &&
        !value
            .chars()
            .any(
                char::is_control,
            )
    {
        Ok(())
    } else {
        Err(
            invalid(
                field,
                "contains unsupported or oversized text",
            ),
        )
    }
}

fn validate_required_text(
    field:
        &'static str,

    value:
        &str,

    max:
        usize,
) -> Result<
    (),
    SitePublicationError,
> {
    if value.trim().is_empty()
    {
        return Err(
            invalid(
                field,
                "must not be empty",
            ),
        );
    }

    validate_optional_text(
        field,
        value,
        max,
    )
}

fn validate_tag(
    value:
        &str,
) -> Result<
    (),
    SitePublicationError,
> {
    if value.trim() !=
        value
        ||
        value.is_empty()
        ||
        value
            .chars()
            .count()
            >
            SITE_PUBLICATION_MAX_TAG_LENGTH
        ||
        value
            .chars()
            .any(
                char::is_control,
            )
    {
        return Err(
            invalid(
                "tags",
                "tags must be trimmed, non-empty, bounded text",
            ),
        );
    }

    Ok(())
}

fn encode_cursor(
    offset:
        usize,
) -> String {
    format!(
        "s_{offset:08x}",
    )
}

fn decode_cursor(
    cursor:
        &str,
) -> Result<
    usize,
    SitePublicationError,
> {
    if cursor.len() == 10
        &&
        cursor
            .starts_with(
                "s_",
            )
    {
        usize::from_str_radix(
            &cursor[
                2..
            ],
            16,
        )
        .map_err(
            |_| {
                SitePublicationError::
                    InvalidCursor
            },
        )
    } else {
        Err(
            SitePublicationError::
                InvalidCursor,
        )
    }
}

fn invalid(
    field:
        &'static str,

    reason:
        &'static str,
) -> SitePublicationError {
    SitePublicationError::
        InvalidField {
            field,
            reason,
        }
}
