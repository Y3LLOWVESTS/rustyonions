//! RO:WHAT — Strict bounded publication-relation projection for public parent/thread reads.
//! RO:WHY — FINAL_BETA Phase 14 needs durable Comment→parent/thread discovery without broadening the general PublicationSummaryV1 timeline contract.
//! RO:INTERACTS — relation-specific Comment summary, shared creator/reference/access/visibility DTOs, svc-index Store, relation keyspace, later Omnigate Comment publish and gateway reads.
//! RO:INVARIANTS — relation source is explicitly Comment; parent/site/thread references are canonical; private/unlisted relations never enter public projection.
//! RO:SECURITY — projection/index metadata only; no raw content bytes, wallet, ledger, receipt, entitlement, moderation mutation, QuickChain, ROX, or Solana authority.
//! RO:TEST — final_beta_phase14_imageboard_relations.rs.

use std::{
    error::Error,
    fmt,
};

use serde::{
    Deserialize,
    Serialize,
};

use crate::publications::{
    PublicationReferencesV1,
};

pub const PUBLICATION_RELATION_SCHEMA: &str =
    "crablink.publication-relation.v1";

pub const PUBLICATION_RELATION_PAGE_SCHEMA: &str =
    "crablink.publication-relation-page.v1";

pub const PUBLICATION_RELATION_DEFAULT_LIMIT: usize =
    50;

pub const PUBLICATION_RELATION_MAX_LIMIT: usize =
    100;

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
pub enum PublicationRelationKind {
    Comment,
}

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
    rename_all = "snake_case"
)]
pub enum PublicationRelationVisibility {
    Public,
    PublicPreview,
    Unlisted,
    Private,
    Deleted,
    Blocked,
    Moderated,
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
pub struct PublicationRelationSummaryV1 {
    pub publication_id:
        String,

    pub kind:
        PublicationRelationKind,

    pub crab_url:
        String,

    #[serde(default)]
    pub title:
        String,

    #[serde(default)]
    pub summary:
        String,

    #[serde(default)]
    pub creator_display:
        Option<String>,

    pub created_at_ms:
        u64,

    pub visibility:
        PublicationRelationVisibility,

    #[serde(default)]
    pub references:
        Option<PublicationReferencesV1>,
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
pub struct PublicationRelationV1 {
    pub schema:
        String,

    pub publication:
        PublicationRelationSummaryV1,

    pub parent_crab_url:
        String,

    #[serde(default)]
    pub thread_crab_url:
        Option<String>,

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
pub struct PublicationRelationPageV1 {
    pub schema:
        String,

    pub items:
        Vec<PublicationRelationV1>,

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
pub struct PublicationRelationPageRequest {
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
pub enum PublicationRelationError {
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
    for PublicationRelationError
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
                formatter.write_str(
                    "invalid publication relation cursor",
                )
            }

            Self::CursorOutOfRange => {
                formatter.write_str(
                    "publication relation cursor is out of range",
                )
            }
        }
    }
}

impl Error
    for PublicationRelationError
{
}

impl PublicationRelationSummaryV1 {
    pub fn validate(
        &self,
    ) -> Result<
        (),
        PublicationRelationError,
    > {
        validate_publication_id(
            &self.publication_id,
        )?;

        if self.kind !=
            PublicationRelationKind::Comment
        {
            return Err(
                invalid(
                    "publication.kind",
                    "relation source must be comment",
                ),
            );
        }

        normalize_typed_crab_url(
            &self.crab_url,
            &[
                "comment",
            ],
            "publication.crabUrl",
        )?;

        validate_optional_text(
            "publication.title",
            &self.title,
            160,
        )?;

        validate_optional_text(
            "publication.summary",
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
                    "publication.title",
                    "title or summary is required",
                ),
            );
        }

        if let Some(
            creator_display,
        ) = self.creator_display
            .as_deref()
        {
            validate_required_text(
                "publication.creatorDisplay",
                creator_display,
                80,
            )?;
        }

        if self.created_at_ms == 0
        {
            return Err(
                invalid(
                    "publication.createdAtMs",
                    "must be greater than zero",
                ),
            );
        }

        if let Some(
            references,
        ) = self.references
            .as_ref()
        {
            let has_manifest =
                references
                    .manifest_cid
                    .is_some();

            let has_content =
                references
                    .content_cid
                    .is_some();

            let has_site =
                references
                    .site_url
                    .is_some();

            if (
                has_manifest
                ||
                has_content
                ||
                has_site
            ) ==
                false
            {
                return Err(
                    invalid(
                        "publication.references",
                        "at least one reference is required",
                    ),
                );
            }

            if let Some(
                cid,
            ) = references
                .manifest_cid
                .as_deref()
            {
                validate_b3_cid(
                    "publication.references.manifestCid",
                    cid,
                )?;
            }

            if let Some(
                cid,
            ) = references
                .content_cid
                .as_deref()
            {
                validate_b3_cid(
                    "publication.references.contentCid",
                    cid,
                )?;
            }

            if let Some(
                site_url,
            ) = references
                .site_url
                .as_deref()
            {
                normalize_relation_site_crab_url(
                    site_url,
                )?;
            }
        }

        Ok(())
    }
}

impl PublicationRelationV1 {
    pub fn validate(
        &self,
    ) -> Result<
        (),
        PublicationRelationError,
    > {
        if self.schema !=
            PUBLICATION_RELATION_SCHEMA
        {
            return Err(
                invalid(
                    "schema",
                    "unsupported relation schema",
                ),
            );
        }

        self.publication
            .validate()?;

        let parent =
            normalize_relation_parent_crab_url(
                &self.parent_crab_url,
            )?;

        let site =
            normalize_relation_site_crab_url(
                &self.site_crab_url,
            )?;

        if let Some(
            references,
        ) = self.publication
            .references
            .as_ref()
        {
            if let Some(
                publication_site,
            ) = references
                .site_url
                .as_deref()
            {
                let publication_site =
                    normalize_relation_site_crab_url(
                        publication_site,
                    )?;

                if publication_site !=
                    site
                {
                    return Err(
                        invalid(
                            "siteCrabUrl",
                            "relation Site does not match publication Site reference",
                        ),
                    );
                }
            }
        }

        if let Some(
            thread,
        ) = self.thread_crab_url
            .as_deref()
        {
            let thread =
                normalize_relation_thread_crab_url(
                    thread,
                )?;

            if parent.ends_with(
                ".image",
            )
                &&
                thread !=
                    parent
            {
                return Err(
                    invalid(
                        "threadCrabUrl",
                        "direct Image reply thread must match Image parent",
                    ),
                );
            }
        }

        Ok(())
    }

    #[must_use]
    pub fn is_public_relation_item(
        &self,
    ) -> bool {
        matches!(
            self.publication.visibility,
            PublicationRelationVisibility::Public
                | PublicationRelationVisibility::PublicPreview
                | PublicationRelationVisibility::Deleted
                | PublicationRelationVisibility::Blocked
                | PublicationRelationVisibility::Moderated
        )
    }
}

impl PublicationRelationPageRequest {
    pub fn new(
        cursor:
            Option<String>,

        limit:
            Option<usize>,
    ) -> Result<
        Self,
        PublicationRelationError,
    > {
        let limit =
            limit.unwrap_or(
                PUBLICATION_RELATION_DEFAULT_LIMIT,
            );

        if (
            1..=
                PUBLICATION_RELATION_MAX_LIMIT
        )
            .contains(
                &limit,
            )
            ==
            false
        {
            return Err(
                invalid(
                    "limit",
                    "must be from 1 through 100",
                ),
            );
        }

        if let Some(
            value,
        ) = cursor.as_deref()
        {
            decode_cursor(
                value,
            )?;
        }

        Ok(
            Self {
                cursor,
                limit,
            },
        )
    }

    pub fn offset(
        &self,
    ) -> Result<
        usize,
        PublicationRelationError,
    > {
        match self.cursor
            .as_deref()
        {
            Some(
                cursor,
            ) => {
                decode_cursor(
                    cursor,
                )
            }

            None => {
                Ok(
                    0,
                )
            }
        }
    }
}

pub fn build_publication_relation_page(
    mut items:
        Vec<PublicationRelationV1>,

    request:
        &PublicationRelationPageRequest,
) -> Result<
    PublicationRelationPageV1,
    PublicationRelationError,
> {
    items.retain(
        |item| {
            item.validate()
                .is_ok()
                &&
            item.is_public_relation_item()
        },
    );

    items.sort_by(
        |left, right| {
            left
                .publication
                .created_at_ms
                .cmp(
                    &right
                        .publication
                        .created_at_ms,
                )
                .then_with(
                    || {
                        left
                            .publication
                            .publication_id
                            .cmp(
                                &right
                                    .publication
                                    .publication_id,
                            )
                    },
                )
        },
    );

    let offset =
        request.offset()?;

    if offset >
        items.len()
    {
        return Err(
            PublicationRelationError::
                CursorOutOfRange,
        );
    }

    let end =
        offset
            .saturating_add(
                request.limit,
            )
            .min(
                items.len(),
            );

    let has_more =
        end <
        items.len();

    let page_items =
        items[
            offset..end
        ]
            .to_vec();

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
        PublicationRelationPageV1 {
            schema:
                PUBLICATION_RELATION_PAGE_SCHEMA
                    .to_owned(),

            items:
                page_items,

            next_cursor,

            has_more,
        },
    )
}

pub fn normalize_relation_parent_crab_url(
    input:
        &str,
) -> Result<
    String,
    PublicationRelationError,
> {
    normalize_typed_crab_url(
        input,
        &[
            "image",
            "article",
            "post",
            "comment",
        ],
        "parentCrabUrl",
    )
}

pub fn normalize_relation_thread_crab_url(
    input:
        &str,
) -> Result<
    String,
    PublicationRelationError,
> {
    normalize_typed_crab_url(
        input,
        &[
            "image",
            "article",
            "post",
            "comment",
        ],
        "threadCrabUrl",
    )
}

pub fn normalize_relation_site_crab_url(
    input:
        &str,
) -> Result<
    String,
    PublicationRelationError,
> {
    let value =
        input.trim();

    if value.starts_with(
        "crab://",
    )
        ==
        false
    {
        return Err(
            invalid(
                "siteCrabUrl",
                "must use crab://",
            ),
        );
    }

    if value !=
        value.to_ascii_lowercase()
    {
        return Err(
            invalid(
                "siteCrabUrl",
                "must be lowercase",
            ),
        );
    }

    let name =
        &value[
            "crab://".len()..
        ];

    if name.is_empty()
        ||
        name.len() >
            128
        ||
        name.contains(
            '/',
        )
        ||
        name.contains(
            '?',
        )
        ||
        name.contains(
            '#',
        )
    {
        return Err(
            invalid(
                "siteCrabUrl",
                "must be a canonical named Site URL",
            ),
        );
    }

    if name
        .chars()
        .all(
            |character| {
                character
                    .is_ascii_lowercase()
                    ||
                character
                    .is_ascii_digit()
                    ||
                matches!(
                    character,
                    '-'
                        | '_'
                        | '.'
                )
            },
        )
        ==
        false
    {
        return Err(
            invalid(
                "siteCrabUrl",
                "contains invalid Site characters",
            ),
        );
    }

    Ok(
        value.to_owned(),
    )
}

fn normalize_typed_crab_url(
    input:
        &str,

    allowed_kinds:
        &[&str],

    field:
        &'static str,
) -> Result<
    String,
    PublicationRelationError,
> {
    let value =
        input.trim();

    if value.starts_with(
        "crab://",
    )
        ==
        false
    {
        return Err(
            invalid(
                field,
                "must use crab://",
            ),
        );
    }

    if value !=
        value.to_ascii_lowercase()
    {
        return Err(
            invalid(
                field,
                "must be lowercase",
            ),
        );
    }

    let target =
        &value[
            "crab://".len()..
        ];

    let Some(
        (
            hash,
            kind,
        ),
    ) = target
        .rsplit_once(
            '.',
        )
    else {
        return Err(
            invalid(
                field,
                "must be a typed crab URL",
            ),
        );
    };

    if hash.len() !=
        64
        ||
        hash
            .bytes()
            .all(
                |byte| {
                    byte.is_ascii_hexdigit()
                        &&
                    byte.is_ascii_uppercase()
                        ==
                        false
                },
            )
            ==
            false
    {
        return Err(
            invalid(
                field,
                "must contain a canonical 64-hex hash",
            ),
        );
    }

    if allowed_kinds
        .iter()
        .any(
            |candidate| {
                candidate ==
                    &kind
            },
        )
        ==
        false
    {
        return Err(
            invalid(
                field,
                "typed asset kind is not accepted",
            ),
        );
    }

    Ok(
        value.to_owned(),
    )
}

fn validate_publication_id(
    value:
        &str,
) -> Result<
    (),
    PublicationRelationError,
> {
    let bytes =
        value.as_bytes();

    if (
        1..=128
    )
        .contains(
            &bytes.len(),
        )
        ==
        false
    {
        return Err(
            invalid(
                "publication.publicationId",
                "must contain 1 through 128 characters",
            ),
        );
    }

    if bytes[
        0
    ]
        .is_ascii_alphanumeric()
        ==
        false
    {
        return Err(
            invalid(
                "publication.publicationId",
                "must begin with ASCII alphanumeric",
            ),
        );
    }

    if bytes
        .iter()
        .all(
            |byte| {
                byte.is_ascii_alphanumeric()
                    ||
                matches!(
                    *byte,
                    b'.'
                        | b'_'
                        | b':'
                        | b'-'
                )
            },
        )
    {
        Ok(())
    } else {
        Err(
            invalid(
                "publication.publicationId",
                "contains unsupported characters",
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
    PublicationRelationError,
> {
    validate_optional_text(
        field,
        value,
        max,
    )?;

    if value
        .trim()
        .is_empty()
    {
        Err(
            invalid(
                field,
                "must not be empty",
            ),
        )
    } else {
        Ok(())
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
    PublicationRelationError,
> {
    if value
        .chars()
        .any(
            char::is_control,
        )
    {
        return Err(
            invalid(
                field,
                "contains control characters",
            ),
        );
    }

    if value
        .chars()
        .count()
        <=
        max
    {
        Ok(())
    } else {
        Err(
            invalid(
                field,
                "exceeds its maximum length",
            ),
        )
    }
}

fn validate_b3_cid(
    field:
        &'static str,

    value:
        &str,
) -> Result<
    (),
    PublicationRelationError,
> {
    let raw =
        value
            .strip_prefix(
                "b3:",
            )
            .unwrap_or(
                "",
            );

    if raw.len() ==
        64
        &&
        raw
            .bytes()
            .all(
                |byte| {
                    byte.is_ascii_digit()
                        ||
                    matches!(
                        byte,
                        b'a'..=b'f'
                    )
                },
            )
    {
        Ok(())
    } else {
        Err(
            invalid(
                field,
                "must be a canonical b3 CID",
            ),
        )
    }
}

fn encode_cursor(
    offset:
        usize,
) -> String {
    format!(
        "r_{offset:08x}",
    )
}

fn decode_cursor(
    cursor:
        &str,
) -> Result<
    usize,
    PublicationRelationError,
> {
    if cursor.len() !=
        10
        ||
        cursor.starts_with(
            "r_",
        )
        ==
        false
    {
        return Err(
            PublicationRelationError::
                InvalidCursor,
        );
    }

    usize::from_str_radix(
        &cursor[
            2..
        ],
        16,
    )
        .map_err(
            |_| {
                PublicationRelationError::
                    InvalidCursor
            },
        )
}

fn invalid(
    field:
        &'static str,

    reason:
        &'static str,
) -> PublicationRelationError {
    PublicationRelationError::
        InvalidField {
            field,
            reason,
        }
}
