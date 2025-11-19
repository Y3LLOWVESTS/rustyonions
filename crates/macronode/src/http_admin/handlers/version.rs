//! RO:WHAT — `/version` handler for Macronode.
//! RO:WHY  — Provide build provenance and HTTP API version.

use axum::{response::IntoResponse, Json};
use serde::Serialize;

use crate::types::BuildInfo;

#[derive(Serialize)]
struct ApiInfo<'a> {
    http: &'a str,
}

#[derive(Serialize)]
struct VersionBody<'a> {
    service: &'a str,
    version: &'a str,
    git_sha: &'a str,
    build_ts: &'a str,
    rustc: &'a str,
    msrv: &'a str,
    api: ApiInfo<'a>,
}

pub async fn handler() -> impl IntoResponse {
    let info = BuildInfo::current();

    let body = VersionBody {
        service: info.service,
        version: info.version,
        git_sha: info.git_sha,
        build_ts: info.build_ts,
        rustc: info.rustc,
        msrv: info.msrv,
        api: ApiInfo { http: "v1" },
    };

    Json(body)
}
