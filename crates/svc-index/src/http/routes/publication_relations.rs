//! RO:WHAT — Strict bounded HTTP read/write boundary for svc-index publication relations.
//! RO:WHY — FINAL_BETA Phase 14 needs durable Comment parent/thread discovery without an Imageboard-specific backend.
//! RO:INTERACTS — PublicationRelationV1, PublicationRelationPageRequest, Store, router, later Omnigate and gateway adapters.
//! RO:INVARIANTS — GET requires one exact canonical parent; PUT validates the strict relation DTO before persistence; pagination remains bounded.
//! RO:SECURITY — relation metadata only; no raw bytes, wallet, ledger, receipt, entitlement, moderation mutation, QuickChain, ROX, or Solana authority.
//! RO:TEST — final_beta_phase14_publication_relation_http.rs.

// FINAL_BETA_PHASE14A6B_SVC_INDEX_RELATION_HTTP_V1

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
    relations::{
        PublicationRelationError,
        PublicationRelationPageRequest,
        PublicationRelationPageV1,
        PublicationRelationV1,
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
pub struct PublicationRelationQuery {
    pub parent_crab_url:
        String,

    #[serde(default)]
    pub cursor:
        Option<String>,

    #[serde(default)]
    pub limit:
        Option<usize>,
}

/// GET /v1/index/publication-relations?parentCrabUrl=<typed crab URL>
///
/// Returns one exact parent's bounded public relation projection.
pub async fn list_publication_relations(
    Query(query):
        Query<PublicationRelationQuery>,

    State(state):
        State<Arc<AppState>>,
) -> Result<
    impl IntoResponse,
    SvcError,
> {
    let page =
        list_publication_relations_from_store(
            &state.store,
            query,
        )?;

    Ok((
        StatusCode::OK,
        Json(
            page,
        ),
    ))
}

/// PUT /v1/index/publication-relations
///
/// Persists one already-published relation projection after strict DTO
/// validation. This endpoint does not create or mutate the underlying
/// Comment bytes.
pub async fn put_publication_relation(
    State(state):
        State<Arc<AppState>>,

    Json(relation):
        Json<PublicationRelationV1>,
) -> Result<
    impl IntoResponse,
    SvcError,
> {
    let relation =
        put_publication_relation_into_store(
            &state.store,
            relation,
        )?;

    Ok((
        StatusCode::ACCEPTED,
        Json(
            relation,
        ),
    ))
}

pub fn list_publication_relations_from_store(
    store:
        &Store,

    query:
        PublicationRelationQuery,
) -> Result<
    PublicationRelationPageV1,
    SvcError,
> {
    let request =
        PublicationRelationPageRequest::new(
            query.cursor,
            query.limit,
        )
        .map_err(
            relation_bad_request,
        )?;

    store
        .list_publication_relations(
            &query.parent_crab_url,
            &request,
        )
        .map_err(
            relation_bad_request,
        )
}

pub fn put_publication_relation_into_store(
    store:
        &Store,

    relation:
        PublicationRelationV1,
) -> Result<
    PublicationRelationV1,
    SvcError,
> {
    relation
        .validate()
        .map_err(
            relation_bad_request,
        )?;

    store
        .put_publication_relation(
            &relation,
        )
        .map_err(
            SvcError::Internal,
        )?;

    Ok(
        relation,
    )
}

fn relation_bad_request(
    error:
        PublicationRelationError,
) -> SvcError {
    SvcError::BadRequest(
        error.to_string(),
    )
}
