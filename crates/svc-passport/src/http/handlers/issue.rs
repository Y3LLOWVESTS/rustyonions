//! RO:WHAT — Issue passports + key discovery + admin rotation/attest.
//! RO:INVARIANTS — deterministic envelope; KID from current signer; TTL bounded.

use axum::{extract::State, Json};
use serde_json::json;
use std::sync::Arc;

use crate::{
    dto::issue::{IssueRequest, IssueResponse},
    error::Error,
    kms::client::KmsClient,
    state::issuer::IssuerState,
    token::encode,
    Config,
};

pub async fn issue(
    State((cfg, issuer, _health)): State<(Config, Arc<IssuerState>, crate::health::Health)>,
    Json(req): Json<IssueRequest>,
) -> Result<Json<IssueResponse>, Error> {
    let payload = encode::canonical_payload(&cfg, &req)?;
    let (kid, sig) = issuer.sign(&payload).await?;
    let env = encode::envelope(&payload, &kid, &sig)?;
    Ok(Json(IssueResponse { envelope: env }))
}

pub async fn keys(
    State((_cfg, issuer, _)): State<(Config, Arc<IssuerState>, crate::health::Health)>,
) -> Result<Json<serde_json::Value>, Error> {
    let jwks = issuer.jwks().await?;
    Ok(Json(jwks))
}

pub async fn rotate(
    State((_cfg, issuer, _)): State<(Config, Arc<IssuerState>, crate::health::Health)>,
) -> Result<Json<serde_json::Value>, Error> {
    let new_kid = issuer.rotate().await?;
    Ok(Json(json!({ "kid": new_kid })))
}

pub async fn attest(
    State((_cfg, issuer, _)): State<(Config, Arc<IssuerState>, crate::health::Health)>,
) -> Result<Json<serde_json::Value>, Error> {
    let att = issuer.attest().await?;
    Ok(Json(att))
}
