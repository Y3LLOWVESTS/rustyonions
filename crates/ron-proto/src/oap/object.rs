//! RO:WHAT — Strict DTOs and app-protocol identifier for OAP object fetch.
//! RO:WHY — Give OBJ_GET and its stream preamble one canonical cross-crate wire shape.
//! RO:INTERACTS — ContentId, OAP Data/End DTOs, svc-storage local object flow.
//! RO:INVARIANTS — b3 IDs are strict; no IP/socket/provider-route fields; DTO-only.
//! RO:SECURITY — Request carries object identity only, never client or residential IP.
//! RO:TEST — tests/oap_object_fetch_dto.rs.

use serde::{Deserialize, Serialize};

use crate::id::ContentId;

/// Application protocol identifier for the OAP/1 OBJ_GET flow.
///
/// All request, stream-start, DATA, and END frames in one object flow use this
/// same app protocol identifier. Frame flags and payload DTOs distinguish the
/// individual stages.
pub const OBJ_GET_APP_PROTO_ID: u16 = 0x0101;

/// Request one canonical b3-addressed object.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ObjGet {
    pub obj: ContentId,
}

/// Announces object and chunk bounds before DATA frames are emitted.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ObjStreamStart {
    pub obj: ContentId,
    pub total_bytes: u64,
    pub chunk_bytes: u32,
}
