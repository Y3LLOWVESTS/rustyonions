//! RO:WHAT — Bounded svc-index HTTP reads for creator publication projections.
//! RO:WHY — FINAL_BETA Phase 6 requires Omnigate to read one canonical creator timeline from svc-index.
//! RO:INTERACTS — AppState.store and the canonical publication projection models.
//! RO:INVARIANTS — public read projection only; non-public records return no public detail; unknown query fields reject.
//! RO:SECURITY — no wallet, ledger, receipt, entitlement, follow, settlement, key, PIN, recovery, or capability authority.
//! RO:TEST — final_beta_phase6_creator_publication_http.rs.

// FINAL_BETA_PHASE6B2_SVC_INDEX_PUBLICATION_HTTP_V1

use std::sync::Arc;

use axum::{
    extract::{
        Path,
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
    publications::{
        PublicationPageRequest,
        PublicationPageV1,
        PublicationProjectionError,
        PublicationSummaryV1,
    },
    store::Store,
    AppState,
};

#[derive(
    Debug,
    Clone,
    Default,
    Deserialize,
    PartialEq,
    Eq,
)]
#[serde(
    deny_unknown_fields,
    rename_all = "camelCase"
)]
pub struct CreatorPublicationQuery {
    #[serde(default)]
    pub cursor: Option<String>,

    #[serde(default)]
    pub limit: Option<usize>,
}

/// GET /v1/index/creators/:username/publications
pub async fn list_creator_publications(
    Path(username): Path<String>,
    Query(query): Query<CreatorPublicationQuery>,
    State(state): State<Arc<AppState>>,
) -> Result<
    impl IntoResponse,
    SvcError,
> {
    let page =
        list_creator_publications_from_store(
            &state.store,
            &username,
            query,
        )?;

    Ok((
        StatusCode::OK,
        Json(page),
    ))
}

/// GET /v1/index/creators/:username/publications/:publication_id
/// PUT /v1/index/creators/:username/publications/:publication_id
///
/// Stores one validated creator-publication projection after canonical
/// publication identity has already been established by the publisher.
///
/// Route creator and publication identity must exactly match the
/// PublicationSummaryV1 body before Store mutation.
pub async fn put_creator_publication(
    Path((
        username,
        publication_id,
    )): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
    Json(publication): Json<PublicationSummaryV1>,
) -> Result<
    StatusCode,
    SvcError,
> {
    put_creator_publication_into_store(
        &state.store,
        &username,
        &publication_id,
        &publication,
    )?;

    Ok(
        StatusCode::NO_CONTENT,
    )
}

pub async fn get_creator_publication(
    Path((
        username,
        publication_id,
    )): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> Result<
    impl IntoResponse,
    SvcError,
> {
    let publication =
        get_creator_publication_from_store(
            &state.store,
            &username,
            &publication_id,
        )?;

    Ok((
        StatusCode::OK,
        Json(publication),
    ))
}

pub fn list_creator_publications_from_store(
    store: &Store,
    username: &str,
    query: CreatorPublicationQuery,
) -> Result<
    PublicationPageV1,
    SvcError,
> {
    let request =
        PublicationPageRequest::new(
            query.cursor,
            query.limit,
        )
        .map_err(
            publication_bad_request,
        )?;

    store
        .list_creator_publications(
            username,
            &request,
        )
        .map_err(
            publication_bad_request,
        )
}

pub fn put_creator_publication_into_store(
    store: &Store,
    username: &str,
    publication_id: &str,
    publication: &PublicationSummaryV1,
) -> Result<
    (),
    SvcError,
> {
    publication
        .validate()
        .map_err(
            publication_bad_request,
        )?;

    if publication.creator.username !=
        username
    {
        return Err(
            SvcError::BadRequest(
                "publication creator username does not match route"
                    .to_owned(),
            ),
        );
    }

    if publication.publication_id !=
        publication_id
    {
        return Err(
            SvcError::BadRequest(
                "publication id does not match route"
                    .to_owned(),
            ),
        );
    }

    store
        .put_creator_publication(
            publication,
        )
        .map_err(
            SvcError::Internal,
        )?;

    Ok(())
}

pub fn get_creator_publication_from_store(
    store: &Store,
    username: &str,
    publication_id: &str,
) -> Result<
    PublicationSummaryV1,
    SvcError,
> {
    let publication =
        store
            .get_creator_publication(
                username,
                publication_id,
            )
            .map_err(
                publication_bad_request,
            )?
            .filter(
                PublicationSummaryV1::
                    is_public_timeline_item,
            )
            .ok_or(
                SvcError::NotFound,
            )?;

    Ok(publication)
}

fn publication_bad_request(
    error: PublicationProjectionError,
) -> SvcError {
    SvcError::BadRequest(
        error.to_string(),
    )
}
