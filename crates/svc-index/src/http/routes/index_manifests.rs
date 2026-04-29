//! RO:WHAT — WEB3_2 manifest pointer routes for assets and sites.
//! RO:WHY — Batch 3 foundation: index mutable pointers while storage keeps immutable bytes.
//! RO:INTERACTS — AppState.store, types::{AssetManifestPointer, SiteManifestPointer}.
//! RO:INVARIANTS — no raw byte storage; no wallet/ledger mutation; CIDs canonicalize to b3:<64 lowercase hex>.
//! RO:METRICS — none directly; HTTP metrics are middleware/service-level.
//! RO:CONFIG — store backend via AppState.
//! RO:SECURITY — dev-mode owner refs are stored as references only, not verified here.
//! RO:TEST — http_contract.rs, integration.rs, prop_index.rs.

use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};

use crate::{
    error::SvcError,
    types::{
        normalize_asset_kind, normalize_b3_cid, normalize_optional_ref, normalize_site_name,
        AssetManifestPointer, PutAssetManifestPointer, PutSiteManifestPointer, SiteManifestPointer,
    },
    AppState,
};

const POINTER_VERSION: u16 = 1;

/// PUT /v1/index/assets/:asset_cid/manifest
pub async fn put_asset_manifest(
    Path(asset_cid): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(body): Json<PutAssetManifestPointer>,
) -> Result<impl IntoResponse, SvcError> {
    let asset_cid = normalize_b3_cid(&asset_cid).map_err(bad_request)?;
    let asset_kind = normalize_asset_kind(&body.asset_kind).map_err(bad_request)?;
    let manifest_cid = normalize_b3_cid(&body.manifest_cid).map_err(bad_request)?;

    let owner_passport_subject =
        normalize_optional_ref("owner_passport_subject", body.owner_passport_subject)
            .map_err(SvcError::BadRequest)?;
    let owner_wallet_account =
        normalize_optional_ref("owner_wallet_account", body.owner_wallet_account)
            .map_err(SvcError::BadRequest)?;

    let updated_at_ms = normalize_updated_at_ms(body.updated_at_ms)?;

    let pointer = AssetManifestPointer {
        version: POINTER_VERSION,
        asset_cid,
        asset_kind,
        manifest_cid,
        owner_passport_subject,
        owner_wallet_account,
        updated_at_ms,
    };

    state
        .store
        .put_asset_manifest_pointer(&pointer)
        .map_err(SvcError::Internal)?;

    Ok((StatusCode::ACCEPTED, Json(pointer)))
}

/// GET /v1/index/assets/:asset_cid/manifest
pub async fn get_asset_manifest(
    Path(asset_cid): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, SvcError> {
    let asset_cid = normalize_b3_cid(&asset_cid).map_err(bad_request)?;

    let Some(pointer) = state.store.get_asset_manifest_pointer(&asset_cid) else {
        return Err(SvcError::NotFound);
    };

    Ok((StatusCode::OK, Json(pointer)))
}

/// PUT /v1/index/sites/:name/manifest
pub async fn put_site_manifest(
    Path(name): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(body): Json<PutSiteManifestPointer>,
) -> Result<impl IntoResponse, SvcError> {
    let name = normalize_site_name(&name).map_err(bad_request)?;
    let manifest_cid = normalize_b3_cid(&body.manifest_cid).map_err(bad_request)?;

    let owner_passport_subject =
        normalize_optional_ref("owner_passport_subject", body.owner_passport_subject)
            .map_err(SvcError::BadRequest)?;
    let owner_wallet_account =
        normalize_optional_ref("owner_wallet_account", body.owner_wallet_account)
            .map_err(SvcError::BadRequest)?;

    let updated_at_ms = normalize_updated_at_ms(body.updated_at_ms)?;

    let pointer = SiteManifestPointer {
        version: POINTER_VERSION,
        name,
        manifest_cid,
        owner_passport_subject,
        owner_wallet_account,
        updated_at_ms,
    };

    state
        .store
        .put_site_manifest_pointer(&pointer)
        .map_err(SvcError::Internal)?;

    Ok((StatusCode::ACCEPTED, Json(pointer)))
}

/// GET /v1/index/sites/:name/manifest
pub async fn get_site_manifest(
    Path(name): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, SvcError> {
    let name = normalize_site_name(&name).map_err(bad_request)?;

    let Some(pointer) = state.store.get_site_manifest_pointer(&name) else {
        return Err(SvcError::NotFound);
    };

    Ok((StatusCode::OK, Json(pointer)))
}

fn bad_request(reason: &'static str) -> SvcError {
    SvcError::BadRequest(reason.to_owned())
}

fn normalize_updated_at_ms(value: Option<u64>) -> Result<u64, SvcError> {
    match value {
        Some(0) => Err(SvcError::BadRequest(
            "updated_at_ms must be greater than zero".to_owned(),
        )),
        Some(value) => Ok(value),
        None => now_ms(),
    }
}

fn now_ms() -> Result<u64, SvcError> {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| SvcError::Internal(anyhow::anyhow!(err)))?;

    u64::try_from(elapsed.as_millis())
        .map_err(|_| SvcError::Internal(anyhow::anyhow!("system time overflow")))
}
