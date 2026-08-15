// RO:WHAT — Public bounded svc-index Explore discovery HTTP read.
// RO:WHY — FINAL_BETA Phase 10 needs one deterministic backend projection for recent public content and public creators.
// RO:INTERACTS — discovery module, AppState.store, later Omnigate Explore route.
// RO:INVARIANTS — strict bounded query; response is public read projection only.
// RO:SECURITY — no mutation, social graph, ranking score, wallet, ledger, receipt, entitlement, QuickChain, ROX, or Solana authority.
// RO:TEST — final_beta_phase10_explore_discovery_projection.rs.

// FINAL_BETA_PHASE10A3B_SVC_INDEX_EXPLORE_HTTP_V1

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
    discovery::{
        build_explore_discovery,
        ExploreDiscoveryError,
        ExploreDiscoveryRequest,
        ExploreDiscoveryV1,
    },
    error::SvcError,
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
pub struct ExploreDiscoveryQuery {
    #[serde(default)]
    pub publication_limit:
        Option<usize>,

    #[serde(default)]
    pub creator_limit:
        Option<usize>,

    #[serde(default)]
    pub site_limit:
        Option<usize>,
}

/// GET /v1/index/explore
pub async fn explore_discovery(
    Query(query):
        Query<ExploreDiscoveryQuery>,
    State(state):
        State<Arc<AppState>>,
) -> Result<
    impl IntoResponse,
    SvcError,
> {
    let discovery =
        explore_discovery_from_store(
            &state.store,
            query,
        )?;

    Ok((
        StatusCode::OK,
        Json(
            discovery,
        ),
    ))
}

pub fn explore_discovery_from_store(
    store: &Store,
    query: ExploreDiscoveryQuery,
) -> Result<
    ExploreDiscoveryV1,
    SvcError,
> {
    let request =
        ExploreDiscoveryRequest::new(
            query.publication_limit,
            query.creator_limit,
            query.site_limit,
        )
        .map_err(
            discovery_bad_request,
        )?;

    build_explore_discovery(
        store,
        &request,
    )
        .map_err(
            discovery_projection_error,
        )
}

fn discovery_bad_request(
    error: ExploreDiscoveryError,
) -> SvcError {
    SvcError::BadRequest(
        error.to_string(),
    )
}

fn discovery_projection_error(
    error: ExploreDiscoveryError,
) -> SvcError {
    match error {
        ExploreDiscoveryError::
            InvalidLimit {
                ..
            } => {
                discovery_bad_request(
                    error,
                )
            }

        ExploreDiscoveryError::
            ConflictingCreator {
                ..
            } => {
                SvcError::Internal(
                    anyhow::Error::new(
                        error,
                    ),
                )
            }
    }
}
