//! RO:WHAT — Read-only paid storage price estimate handler for GET /paid/o/estimate.
//! RO:WHY — Pillar 12; Concerns: ECON/DX/GOV. Clients need preflight pricing before wallet holds.
//! RO:INTERACTS — policy::economics; shares pricing with /paid/o settlement planning.
//! RO:INVARIANTS — no wallet, ledger, accounting, storage, or manifest mutation; integer minor units only.
//! RO:METRICS — none currently; this endpoint is side-effect-free.
//! RO:CONFIG — RON_STORAGE_ROC_ECONOMICS_PATH, RON_STORAGE_ROC_ECONOMICS_ACTION.
//! RO:SECURITY — exposes policy-derived price only; no account IDs, receipts, CIDs, or secrets.
//! RO:TEST — tests/paid_write_estimate.rs.

use axum::{http::StatusCode, http::Uri, response::IntoResponse, Json};
use serde::Serialize;

use crate::policy::economics::paid_storage_price_estimate_from_env;

const PAID_ROUTE: &str = "/paid/o";
const ESTIMATE_SCHEMA: &str = "svc-storage.paid-storage-estimate.v1";

#[derive(Debug, Serialize)]
struct PaidEstimateResp {
    schema: &'static str,
    route: &'static str,
    action: String,
    asset: String,
    bytes: u64,
    amount_minor: String,
    minimum_hold_minor: String,
    pricing_mode: &'static str,
    economics_policy_path: Option<String>,
}

#[derive(Debug, Serialize)]
struct PaidEstimateErrorBody {
    error: &'static str,
    reason: String,
}

/// Return a side-effect-free paid-storage estimate.
///
/// This endpoint is intentionally read-only. It must not create wallet holds,
/// mutate ledger state, write storage bytes, or export accounting events.
pub async fn handler(uri: Uri) -> impl IntoResponse {
    let bytes_stored = match parse_bytes_from_uri(&uri) {
        Ok(bytes_stored) => bytes_stored,
        Err(reason) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(PaidEstimateErrorBody {
                    error: "bad_request",
                    reason,
                }),
            )
                .into_response();
        }
    };

    let estimate = match paid_storage_price_estimate_from_env(bytes_stored) {
        Ok(estimate) => estimate,
        Err(reason) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(PaidEstimateErrorBody {
                    error: "config_error",
                    reason,
                }),
            )
                .into_response();
        }
    };

    (
        StatusCode::OK,
        Json(PaidEstimateResp {
            schema: ESTIMATE_SCHEMA,
            route: PAID_ROUTE,
            action: estimate.action_id,
            asset: estimate.asset,
            bytes: estimate.bytes_stored,
            amount_minor: estimate.amount_minor.to_string(),
            minimum_hold_minor: estimate.amount_minor.to_string(),
            pricing_mode: estimate.pricing_mode,
            economics_policy_path: estimate.economics_policy_path,
        }),
    )
        .into_response()
}

fn parse_bytes_from_uri(uri: &Uri) -> Result<u64, String> {
    let Some(query) = uri.query() else {
        return Err("missing required query parameter: bytes".to_string());
    };

    for pair in query.split('&') {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));

        if key.trim() == "bytes" {
            return parse_bytes(Some(value));
        }
    }

    Err("missing required query parameter: bytes".to_string())
}

fn parse_bytes(value: Option<&str>) -> Result<u64, String> {
    let Some(value) = value else {
        return Err("missing required query parameter: bytes".to_string());
    };

    let value = value.trim();
    if value.is_empty() {
        return Err("bytes cannot be empty".to_string());
    }

    value
        .parse::<u64>()
        .map_err(|_| "bytes must be an unsigned integer".to_string())
}
