//! RO:WHAT — Binary HTTP adapter for the OAP/1 `OBJ_GET` service.
//!
//! RO:WHY — Expose the validated OAP model through the real storage listener.
//!
//! RO:INTERACTS — `AppState`, shared moderation policy, OAP service, and Axum.
//!
//! RO:INVARIANTS — one request frame; ≤1 MiB request; request policy and
//! moderation before storage; verified b3 response.
//!
//! RO:SECURITY — no IP/provider/economic DTO fields; no wallet or ledger
//! mutation; immutable moderation is injected by the router.
//!
//! RO:TEST — `tests/oap_http_transport.rs`.

use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    extract::State,
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Extension,
};
use ron_policy::ModerationPolicy;

use crate::{
    http::extractors::AppState,
    oap_object::{LocalOapObjectService, OapObjectError},
};

const OAP_CONTENT_TYPE: &str = "application/oap";

/// Accept one encoded OAP `OBJ_GET` request and return an encoded stream.
pub async fn handler(
    State(app): State<AppState>,
    Extension(moderation): Extension<Arc<ModerationPolicy>>,
    body: Body,
) -> Response {
    let wire = match to_bytes(body, oap::MAX_FRAME_BYTES as usize).await {
        Ok(wire) => wire,
        Err(_) => {
            return (
                StatusCode::PAYLOAD_TOO_LARGE,
                "OAP request exceeded the 1 MiB frame limit",
            )
                .into_response();
        }
    };

    let service = match LocalOapObjectService::privacy_aware_local(app.store.clone()) {
        Ok(service) => service.with_shared_moderation_policy(moderation),
        Err(err) => return error_response(err),
    };

    match service.serve_obj_get_wire(&wire).await {
        Ok(response_wire) => {
            let mut response = (StatusCode::OK, response_wire).into_response();

            response.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static(OAP_CONTENT_TYPE),
            );

            response
        }
        Err(err) => error_response(err),
    }
}

fn error_response(err: OapObjectError) -> Response {
    if let OapObjectError::ModerationDenied { reason } = &err {
        super::moderation_observability::observe_refusal(
            super::moderation_observability::ModerationReadRoute::OapObjGet,
            *reason,
        );
    }

    let status = match &err {
        OapObjectError::FrameTooLarge { .. } | OapObjectError::ChunkTooLarge { .. } => {
            StatusCode::PAYLOAD_TOO_LARGE
        }

        OapObjectError::InvalidRequest(_)
        | OapObjectError::InvalidResponse(_)
        | OapObjectError::Codec(_)
        | OapObjectError::Json(_) => StatusCode::BAD_REQUEST,

        OapObjectError::NotFound => StatusCode::NOT_FOUND,
        OapObjectError::NotReady => StatusCode::SERVICE_UNAVAILABLE,
        OapObjectError::PolicyDenied { .. } | OapObjectError::ModerationDenied { .. } => {
            StatusCode::FORBIDDEN
        }

        OapObjectError::PolicyConfig(_)
        | OapObjectError::PolicyEvaluation(_)
        | OapObjectError::ModerationIdentity(_)
        | OapObjectError::DigestMismatch { .. }
        | OapObjectError::Storage(_) => StatusCode::INTERNAL_SERVER_ERROR,
    };

    (status, err.to_string()).into_response()
}
