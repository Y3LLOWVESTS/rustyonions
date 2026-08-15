// RO:WHAT — Deterministic svc-index Explore discovery projection.
// RO:WHY — FINAL_BETA Phase 10 needs one real public discovery read built from canonical publication records.
// RO:INTERACTS — Store creator-publication keyspace, PublicationSummaryV1, Explore HTTP route, later Omnigate and gateway adapters.
// RO:INVARIANTS — recent content is public-only and newest-first; creators are deduplicated and username-ordered; all limits are bounded.
// RO:SECURITY — read projection only; no social graph, engagement ranking, paid ranking, wallet, ledger, receipt, entitlement, QuickChain, ROX, or Solana authority.
// RO:TEST — final_beta_phase10_explore_discovery_projection.rs.

// FINAL_BETA_PHASE10A3B_SVC_INDEX_DISCOVERY_PROJECTION_V1

use std::collections::BTreeMap;

use serde::{
    Deserialize,
    Serialize,
};
use thiserror::Error;

use crate::{
    publications::{
        PublicationCreatorV1,
        PublicationSummaryV1,
    },
    store::Store,
};

pub const EXPLORE_DISCOVERY_SCHEMA: &str =
    "crablink.explore-discovery.v1";

pub const EXPLORE_PUBLICATION_DEFAULT_LIMIT: usize =
    12;

pub const EXPLORE_PUBLICATION_MAX_LIMIT: usize =
    24;

pub const EXPLORE_CREATOR_DEFAULT_LIMIT: usize =
    12;

pub const EXPLORE_CREATOR_MAX_LIMIT: usize =
    24;

pub const EXPLORE_SITE_DEFAULT_LIMIT: usize =
    8;

pub const EXPLORE_SITE_MAX_LIMIT: usize =
    16;

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
pub struct ExploreTemplateSiteSummaryV1 {
    pub site_url: String,
    pub title: String,

    #[serde(default)]
    pub summary: Option<String>,

    pub creator: PublicationCreatorV1,
    pub template_id: String,
    pub updated_at: String,
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
pub struct ExploreDiscoveryV1 {
    pub schema: String,
    pub recent_publications:
        Vec<PublicationSummaryV1>,
    pub public_creators:
        Vec<PublicationCreatorV1>,
    pub template_sites:
        Vec<ExploreTemplateSiteSummaryV1>,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
)]
pub struct ExploreDiscoveryRequest {
    pub publication_limit: usize,
    pub creator_limit: usize,
    pub site_limit: usize,
}

#[derive(
    Debug,
    Error,
    PartialEq,
    Eq,
)]
pub enum ExploreDiscoveryError {
    #[error(
        "invalid {field}: must be from 1 through {maximum}"
    )]
    InvalidLimit {
        field: &'static str,
        maximum: usize,
    },

    #[error(
        "conflicting public creator projection for {username}"
    )]
    ConflictingCreator {
        username: String,
    },
}

impl ExploreDiscoveryRequest {
    pub fn new(
        publication_limit: Option<usize>,
        creator_limit: Option<usize>,
        site_limit: Option<usize>,
    ) -> Result<
        Self,
        ExploreDiscoveryError,
    > {
        Ok(Self {
            publication_limit:
                normalize_limit(
                    publication_limit,
                    EXPLORE_PUBLICATION_DEFAULT_LIMIT,
                    EXPLORE_PUBLICATION_MAX_LIMIT,
                    "publicationLimit",
                )?,

            creator_limit:
                normalize_limit(
                    creator_limit,
                    EXPLORE_CREATOR_DEFAULT_LIMIT,
                    EXPLORE_CREATOR_MAX_LIMIT,
                    "creatorLimit",
                )?,

            site_limit:
                normalize_limit(
                    site_limit,
                    EXPLORE_SITE_DEFAULT_LIMIT,
                    EXPLORE_SITE_MAX_LIMIT,
                    "siteLimit",
                )?,
        })
    }
}

pub fn build_explore_discovery(
    store: &Store,
    request: &ExploreDiscoveryRequest,
) -> Result<
    ExploreDiscoveryV1,
    ExploreDiscoveryError,
> {
    let mut all_publications =
        store
            .list_public_creator_publications_for_discovery();

    all_publications.sort_by(
        |left, right| {
            right
                .published_at
                .cmp(
                    &left.published_at,
                )
                .then_with(
                    || {
                        left
                            .creator
                            .username
                            .cmp(
                                &right
                                    .creator
                                    .username,
                            )
                    },
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

    let recent_publications =
        all_publications
            .iter()
            .take(
                request.publication_limit,
            )
            .cloned()
            .collect::<Vec<_>>();

    let mut creators =
        BTreeMap::<
            String,
            PublicationCreatorV1,
        >::new();

    for publication in &all_publications {
        let username =
            publication
                .creator
                .username
                .clone();

        match creators.get(
            &username,
        ) {
            Some(existing) => {
                if existing
                    == &publication.creator
                {
                    continue;
                }

                return Err(
                    ExploreDiscoveryError::
                        ConflictingCreator {
                            username,
                        },
                );
            }

            None => {
                creators.insert(
                    username,
                    publication
                        .creator
                        .clone(),
                );
            }
        }
    }

    let public_creators =
        creators
            .into_values()
            .take(
                request.creator_limit,
            )
            .collect::<Vec<_>>();

    // The current SiteManifestPointer does not contain
    // template ID plus public creator display metadata.
    //
    // Do not infer public site-card truth from owner
    // passport or wallet references. A later reviewed
    // site-display projection will populate this array.
    let template_sites =
        Vec::with_capacity(
            request
                .site_limit
                .min(
                    EXPLORE_SITE_MAX_LIMIT,
                ),
        );

    Ok(
        ExploreDiscoveryV1 {
            schema:
                EXPLORE_DISCOVERY_SCHEMA
                    .to_owned(),

            recent_publications,

            public_creators,

            template_sites,
        },
    )
}

fn normalize_limit(
    value: Option<usize>,
    fallback: usize,
    maximum: usize,
    field: &'static str,
) -> Result<
    usize,
    ExploreDiscoveryError,
> {
    let value =
        value.unwrap_or(
            fallback,
        );

    if (
        1..=maximum
    )
        .contains(
            &value,
        )
    {
        return Ok(
            value,
        );
    }

    Err(
        ExploreDiscoveryError::
            InvalidLimit {
                field,
                maximum,
            },
    )
}
