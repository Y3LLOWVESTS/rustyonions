//! RO:WHAT — Exact local prune endpoint for service-node operators.
//!
//! RO:WHY — Expose the real storage, provider, and index-cache coordinator
//! without claiming network-wide deletion or economic mutation.
//!
//! RO:INVARIANTS — canonical exact B3 object only; guarded POST; truthful
//! per-step outcomes; no manifest deletion, wallet, ledger, or reward finality.

#![forbid(unsafe_code)]

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use ron_policy::B3Id;
use serde::Deserialize;

use crate::{services::prune::PruneStatus, types::AppState};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PruneRequest {
    object: String,
}

pub async fn handler(
    State(state): State<AppState>,
    Json(request): Json<PruneRequest>,
) -> impl IntoResponse {
    let object = match request.object.parse::<B3Id>() {
        Ok(object) => object,
        Err(err) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "status": "invalid_object",
                    "error": err.to_string(),
                    "changed": false,
                    "wallet_mutation": false,
                    "ledger_mutation": false,
                    "reward_finality": false
                })),
            )
                .into_response();
        }
    };

    let report = state.runtime.prune_local_object(&object).await;

    let status = match report.status {
        PruneStatus::Pruned | PruneStatus::AlreadyAbsent => StatusCode::OK,
        PruneStatus::Partial => StatusCode::MULTI_STATUS,
        PruneStatus::Failed => StatusCode::SERVICE_UNAVAILABLE,
    };

    (status, Json(report)).into_response()
}

#[cfg(test)]
mod tests {
    use std::{
        sync::Arc,
        time::{Duration, Instant},
    };

    use axum::{
        body::{to_bytes, Bytes},
        response::IntoResponse,
    };
    use serde_json::Value;
    use svc_dht::{types::CrabNodeId, ProviderStore};
    use svc_index::{cache::IndexCache, types::ProvidersResponse};
    use svc_storage::storage::{DynStorage, MemoryStorage};

    use super::*;
    use crate::{
        bench::BenchManager,
        bus::NodeBus,
        config::Config,
        readiness::ReadyProbes,
        types::{AppState, OperatorState, RuntimeStatus},
    };

    const CID: &str = "b3:6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85";

    const NODE_URI: &str =
        "crab://node/00000000000000000000000000000000000000000000000000000000000000a1";

    fn app_state(runtime: Arc<RuntimeStatus>) -> AppState {
        AppState {
            cfg: Arc::new(Config::default()),
            probes: Arc::new(ReadyProbes::new()),
            runtime,
            bus: NodeBus::new(),
            started_at: Instant::now(),
            bench: Arc::new(BenchManager::new("http://127.0.0.1:1".to_string())),
            operator: Arc::new(OperatorState::new(
                false,
                "http://127.0.0.1:5300".to_string(),
                Duration::from_secs(15 * 60),
            )),
        }
    }

    fn providers_response() -> ProvidersResponse {
        ProvidersResponse {
            cid: CID.to_owned(),
            providers: Vec::new(),
            truncated: false,
            etag: None,
        }
    }

    async fn response_json(response: axum::response::Response) -> Value {
        let body = to_bytes(response.into_body(), 1024 * 1024)
            .await
            .expect("bounded response body");

        serde_json::from_slice(&body).expect("response JSON")
    }

    #[tokio::test]
    async fn handler_reports_all_exact_successful_prune_steps() {
        let runtime = Arc::new(RuntimeStatus::new());

        let storage: DynStorage = Arc::new(MemoryStorage::default());

        storage
            .put(CID, Bytes::from_static(b"abc"))
            .await
            .expect("test object put");

        let providers = Arc::new(ProviderStore::new(Duration::from_secs(60)));

        providers
            .add(
                CID.to_owned(),
                NODE_URI.to_owned(),
                Some(Duration::from_secs(60)),
            )
            .expect("local provider record");

        let index = IndexCache::new(60);
        index.put_providers(CID.to_owned(), providers_response());

        let node_id = CrabNodeId::from_uri(NODE_URI).expect("canonical configured node");

        runtime.register_prune_storage(storage);
        runtime.register_prune_provider_store(providers, node_id);
        runtime.register_prune_index_cache(index);

        let response = handler(
            State(app_state(runtime.clone())),
            Json(PruneRequest {
                object: CID.to_string(),
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response_json(response).await;

        assert_eq!(body["status"], "pruned");
        assert_eq!(body["complete"], true);
        assert_eq!(body["changed"], true);
        assert_eq!(body["local_bytes"]["status"], "removed");
        assert_eq!(body["local_bytes"]["bytes"], 3);
        assert_eq!(body["provider"]["status"], "withdrawn");
        assert_eq!(body["index_cache_invalidation"], "invalidated");
        assert_eq!(body["scope"], "local_storage_provider_and_index_cache");
        assert_eq!(body["network_propagation"], false);
        assert_eq!(body["resolve_cache_invalidation"], false);
        assert_eq!(body["manifest_pointer_removal"], false);
        assert_eq!(body["wallet_mutation"], false);
        assert_eq!(body["ledger_mutation"], false);
        assert_eq!(body["reward_finality"], false);
        assert_eq!(runtime.completed_prune_count(), 1);
    }

    #[tokio::test]
    async fn handler_rejects_invalid_object_without_mutation_claims() {
        let runtime = Arc::new(RuntimeStatus::new());

        let response = handler(
            State(app_state(runtime)),
            Json(PruneRequest {
                object: "b3:not-a-valid-object".to_string(),
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = response_json(response).await;

        assert_eq!(body["status"], "invalid_object");
        assert_eq!(body["changed"], false);
        assert_eq!(body["wallet_mutation"], false);
        assert_eq!(body["ledger_mutation"], false);
        assert_eq!(body["reward_finality"], false);
    }

    #[tokio::test]
    async fn handler_maps_missing_runtime_handles_to_service_unavailable() {
        let runtime = Arc::new(RuntimeStatus::new());

        let response = handler(
            State(app_state(runtime)),
            Json(PruneRequest {
                object: CID.to_string(),
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

        let body = response_json(response).await;

        assert_eq!(body["status"], "failed");
        assert_eq!(body["complete"], false);
        assert_eq!(body["changed"], false);
        assert_eq!(body["local_bytes"]["status"], "unavailable");
        assert_eq!(body["provider"]["status"], "unavailable");
        assert_eq!(body["index_cache_invalidation"], "unavailable");
        assert_eq!(body["network_propagation"], false);
    }
}
