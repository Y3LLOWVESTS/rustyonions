//! RO:WHAT — Bounded HTTP read/write boundary for Site-keyed root-publication projections.
//! RO:WHY — FINAL_BETA Phase 15 needs durable Forum thread roots without widening Comment relations or claiming creator identity.
//! RO:INTERACTS — SitePublicationV1, SitePublicationPageRequest, Store, svc-index router.
//! RO:INVARIANTS — GET requires one exact Site; PUT validates before persistence; pagination remains bounded.
//! RO:SECURITY — index metadata only; creatorDisplay is display text, not identity authority; no wallet, ledger, receipt, entitlement, moderation mutation, QuickChain, ROX, or Solana authority.
//! RO:TEST — final_beta_phase15_site_publication_roots.rs.

use std::sync::Arc;

use axum::{
    extract::{
        Query,
        State,
    },

    http::StatusCode,

    response::IntoResponse,

    Json,
};

use serde::Deserialize;

use crate::{
    error::SvcError,

    site_publications::{
        SitePublicationError,
        SitePublicationPageRequest,
        SitePublicationPageV1,
        SitePublicationV1,
    },

    store::Store,

    AppState,
};

#[derive(
    Debug,
    Clone,
    Deserialize,
    PartialEq,
    Eq,
)]
#[serde(
    deny_unknown_fields,
    rename_all = "camelCase"
)]
pub struct SitePublicationQuery {
    pub site_crab_url:
        String,

    #[serde(default)]
    pub cursor:
        Option<String>,

    #[serde(default)]
    pub limit:
        Option<usize>,
}

/// GET /v1/index/site-publications?siteCrabUrl=<named Site>
pub async fn list_site_publications(
    Query(
        query,
    ):
        Query<SitePublicationQuery>,

    State(
        state,
    ):
        State<Arc<AppState>>,
) -> Result<
    impl IntoResponse,
    SvcError,
> {
    let page =
        list_site_publications_from_store(
            &state.store,
            query,
        )?;

    Ok(
        (
            StatusCode::OK,
            Json(
                page,
            ),
        ),
    )
}

/// PUT /v1/index/site-publications
///
/// Stores one already-published root projection. This endpoint does not
/// create content bytes, establish creator identity, or mutate moderation.
pub async fn put_site_publication(
    State(
        state,
    ):
        State<Arc<AppState>>,

    Json(
        publication,
    ):
        Json<SitePublicationV1>,
) -> Result<
    impl IntoResponse,
    SvcError,
> {
    let publication =
        put_site_publication_into_store(
            &state.store,
            publication,
        )?;

    Ok(
        (
            StatusCode::ACCEPTED,
            Json(
                publication,
            ),
        ),
    )
}

pub fn list_site_publications_from_store(
    store:
        &Store,

    query:
        SitePublicationQuery,
) -> Result<
    SitePublicationPageV1,
    SvcError,
> {
    let request =
        SitePublicationPageRequest::new(
            query.cursor,
            query.limit,
        )
        .map_err(
            site_publication_bad_request,
        )?;

    store
        .list_site_publications(
            &query.site_crab_url,
            &request,
        )
        .map_err(
            site_publication_bad_request,
        )
}

pub fn put_site_publication_into_store(
    store:
        &Store,

    publication:
        SitePublicationV1,
) -> Result<
    SitePublicationV1,
    SvcError,
> {
    publication
        .validate()
        .map_err(
            site_publication_bad_request,
        )?;

    store
        .put_site_publication(
            &publication,
        )
        .map_err(
            SvcError::Internal,
        )?;

    Ok(
        publication,
    )
}

fn site_publication_bad_request(
    error:
        SitePublicationError,
) -> SvcError {
    SvcError::BadRequest(
        error
            .to_string(),
    )
}
