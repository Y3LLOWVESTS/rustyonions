//! RO:WHAT — Deterministic OAP/1 `OBJ_GET` serving and verification.
//!
//! RO:WHY — Phases 9 and 10 require policy-gated, moderation-gated,
//! chunk-bounded, digest-verified object flow.
//!
//! RO:INTERACTS — OAP codecs, `ron-proto` object DTOs, storage, and
//! `ron-policy` request and moderation decisions.
//!
//! RO:INVARIANTS — frame ≤1 MiB; DATA chunk ≤64 KiB; full BLAKE3 match;
//! request policy and object moderation both run before storage lookup.
//!
//! RO:SECURITY — request DTO carries no IP/socket/route; denied, blocked,
//! tombstoned, and quarantined objects are never read or served.
//!
//! RO:TEST — `tests/oap_object_fetch.rs`.

#![forbid(unsafe_code)]

use std::sync::Arc;

use bytes::Bytes;
use oap::{
    Flags, Frame, FrameBuilder, OapDecodeError, OapEncodeError, OapWriter, ParserState, StatusCode,
    MAX_FRAME_BYTES, STREAM_CHUNK_SIZE,
};
use ron_policy::{
    ctx::clock::SystemClock, B3Id, Context, DecisionEffect, Evaluator, ModerationPolicy,
    ModerationReasonCode, PolicyBundle,
};
use ron_proto::oap::data::Data;
use ron_proto::oap::end::End;
use ron_proto::oap::object::{ObjGet, ObjStreamStart, OBJ_GET_APP_PROTO_ID};
use ron_proto::ContentId;
use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;

use crate::errors::StorageError;
use crate::storage::DynStorage;

/// Policy tag proving this slice does not accept direct or IP-bearing routes.
pub const PRIVACY_AWARE_LOCAL_TAG: &str = "privacy-aware-local";

/// Errors returned by deterministic OBJ_GET serving or verification.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum OapObjectError {
    #[error("OAP frame too large: {len} > {max}")]
    FrameTooLarge { len: u32, max: u32 },

    #[error("OAP DATA chunk too large: {len} > {max}")]
    ChunkTooLarge { len: usize, max: usize },

    #[error("invalid OBJ_GET request: {0}")]
    InvalidRequest(&'static str),

    #[error("invalid OBJ_GET response: {0}")]
    InvalidResponse(&'static str),

    #[error("object not found")]
    NotFound,

    #[error("object service is not ready")]
    NotReady,

    #[error("policy denied OBJ_GET: {reason}")]
    PolicyDenied { reason: String },

    #[error("invalid policy configuration: {0}")]
    PolicyConfig(String),

    #[error("policy evaluation failed: {0}")]
    PolicyEvaluation(String),

    #[error("invalid moderation object identifier: {0}")]
    ModerationIdentity(String),

    #[error("moderation refused OBJ_GET: {reason:?}")]
    ModerationDenied { reason: ModerationReasonCode },

    #[error("object digest mismatch: expected {expected}, got {actual}")]
    DigestMismatch { expected: String, actual: String },

    #[error("storage failure: {0}")]
    Storage(String),

    #[error("OAP codec failure: {0}")]
    Codec(String),

    #[error("OBJ_GET JSON failure: {0}")]
    Json(String),
}

/// Local deterministic OAP object service.
///
/// Request-policy evaluation and exact-b3 moderation are separate gates.
/// Both gates run before the storage backend is consulted.
pub struct LocalOapObjectService {
    store: DynStorage,
    policy: PolicyBundle,
    moderation: Arc<ModerationPolicy>,
    ready: bool,
}

impl LocalOapObjectService {
    /// Build a service using a validated policy bundle.
    pub fn new(store: DynStorage, policy: PolicyBundle) -> Result<Self, OapObjectError> {
        Evaluator::new(&policy).map_err(|err| OapObjectError::PolicyConfig(err.to_string()))?;

        Ok(Self {
            store,
            policy,
            moderation: Arc::new(ModerationPolicy::default()),
            ready: true,
        })
    }

    /// Build the canonical local privacy-aware OAP read service.
    ///
    /// The policy only allows `OBJ_GET` requests carrying the internal
    /// `privacy-aware-local` context tag. The wire request itself contains no
    /// IP, socket, route, provider, wallet, ledger, or reward fields.
    pub fn privacy_aware_local(store: DynStorage) -> Result<Self, OapObjectError> {
        let policy = ron_policy::load_json(
            br#"{
                "version": 1,
                "rules": [
                    {
                        "id": "allow-private-local-obj-get",
                        "when": {
                            "method": "OBJ_GET",
                            "require_tags_all": ["privacy-aware-local"]
                        },
                        "action": "allow",
                        "reason": "local private object read"
                    }
                ]
            }"#,
        )
        .map_err(|err| OapObjectError::PolicyConfig(err.to_string()))?;

        Self::new(store, policy)
    }

    /// Set deterministic readiness for local tests and future runtime wiring.
    #[must_use]
    pub const fn with_ready(mut self, ready: bool) -> Self {
        self.ready = ready;
        self
    }

    /// Install the canonical exact-b3 moderation policy.
    ///
    /// The supplied policy is declarative only. This method does not delete
    /// storage, publish provider records, or perform economic mutation.
    #[must_use]
    pub fn with_moderation_policy(mut self, moderation: ModerationPolicy) -> Self {
        self.moderation = Arc::new(moderation);
        self
    }

    /// Install an already shared immutable moderation snapshot.
    ///
    /// The HTTP runtime uses this path so potentially large moderation sets
    /// are not cloned for every OAP object request.
    #[must_use]
    pub fn with_shared_moderation_policy(mut self, moderation: Arc<ModerationPolicy>) -> Self {
        self.moderation = moderation;
        self
    }

    /// Process one OAP OBJ_GET request and return a bounded response stream.
    ///
    /// Policy evaluation deliberately occurs before any storage lookup.
    pub async fn serve_obj_get(&self, request: Frame) -> Result<Vec<Frame>, OapObjectError> {
        let request = normalize_frame(request)?;
        validate_request_frame(&request)?;

        if !self.ready {
            return Err(OapObjectError::NotReady);
        }

        let get: ObjGet = decode_payload(
            &request,
            OapObjectError::InvalidRequest("missing or invalid OBJ_GET payload"),
        )?;

        self.enforce_policy(&request)?;
        self.enforce_moderation(&get.obj)?;

        let object = self
            .store
            .get_full(get.obj.as_str())
            .await
            .map_err(map_storage_error)?;

        verify_digest(&get.obj, &object)?;

        build_response_stream(
            &get.obj,
            object,
            request.header.tenant_id,
            request.header.corr_id,
        )
    }

    /// Process exactly one encoded OAP OBJ_GET request and return the encoded
    /// START/DATA/END response stream.
    pub async fn serve_obj_get_wire(&self, wire: &[u8]) -> Result<Bytes, OapObjectError> {
        let mut requests = decode_wire_frames(wire)?;

        if requests.len() != 1 {
            return Err(OapObjectError::InvalidRequest(
                "wire body must contain exactly one OBJ_GET frame",
            ));
        }

        let request = requests.pop().ok_or(OapObjectError::InvalidRequest(
            "wire body did not contain an OBJ_GET frame",
        ))?;

        let responses = self.serve_obj_get(request).await?;
        encode_frame_stream(&responses)
    }

    fn enforce_policy(&self, request: &Frame) -> Result<(), OapObjectError> {
        let evaluator = Evaluator::new(&self.policy)
            .map_err(|err| OapObjectError::PolicyConfig(err.to_string()))?;

        let payload_bytes = request.payload.as_ref().map_or(0_u64, |payload| {
            u64::try_from(payload.len()).unwrap_or(u64::MAX)
        });

        let clock = SystemClock;

        let context = Context::builder()
            .tenant(format!("{:032x}", request.header.tenant_id))
            .method("OBJ_GET")
            .region("local")
            .body_bytes(payload_bytes)
            .tag("oap")
            .tag("object-read")
            .tag(PRIVACY_AWARE_LOCAL_TAG)
            .build(&clock);

        let decision = evaluator
            .evaluate(&context)
            .map_err(|err| OapObjectError::PolicyEvaluation(err.to_string()))?;

        if decision.effect == DecisionEffect::Deny {
            return Err(OapObjectError::PolicyDenied {
                reason: decision
                    .reason
                    .unwrap_or_else(|| "deny-by-default".to_string()),
            });
        }

        Ok(())
    }

    fn enforce_moderation(&self, object: &ContentId) -> Result<(), OapObjectError> {
        let object = object
            .as_str()
            .parse::<B3Id>()
            .map_err(|err| OapObjectError::ModerationIdentity(err.to_string()))?;

        let decision = self.moderation.evaluate(&object);

        if !decision.permits_serve() {
            return Err(OapObjectError::ModerationDenied {
                reason: decision.reason,
            });
        }

        Ok(())
    }
}

/// Build a canonical single-frame OBJ_GET request.
pub fn build_obj_get_request(
    obj: ContentId,
    tenant_id: u128,
    corr_id: u64,
) -> Result<Frame, OapObjectError> {
    let payload = encode_json(&ObjGet { obj })?;

    normalize_frame(
        FrameBuilder::request(OBJ_GET_APP_PROTO_ID, tenant_id, corr_id)
            .start()
            .end()
            .payload(payload)
            .build(),
    )
}

/// Encode one canonical OAP frame for transport.
pub fn encode_frame_wire(frame: Frame) -> Result<Bytes, OapObjectError> {
    encode_frame_stream(std::slice::from_ref(&frame))
}

/// Decode and verify an encoded OAP object response stream.
pub fn verify_obj_stream_wire(expected: &ContentId, wire: &[u8]) -> Result<Bytes, OapObjectError> {
    let frames = decode_wire_frames(wire)?;
    verify_obj_stream(expected, &frames)
}

/// Verify a complete local OBJ_GET response and return the authenticated bytes.
///
/// Verification checks every frame through the real OAP encoder/decoder,
/// validates sequence and chunk limits, then calculates the complete BLAKE3
/// digest over the reconstructed object.
pub fn verify_obj_stream(expected: &ContentId, frames: &[Frame]) -> Result<Bytes, OapObjectError> {
    if frames.len() < 2 {
        return Err(OapObjectError::InvalidResponse(
            "stream requires START and END frames",
        ));
    }

    let start_frame = normalize_frame(frames[0].clone())?;
    validate_start_frame(&start_frame)?;

    let start: ObjStreamStart = decode_payload(
        &start_frame,
        OapObjectError::InvalidResponse("missing or invalid object stream START payload"),
    )?;

    if &start.obj != expected {
        return Err(OapObjectError::InvalidResponse(
            "START object does not match requested object",
        ));
    }

    let advertised_chunk = usize::try_from(start.chunk_bytes)
        .map_err(|_| OapObjectError::InvalidResponse("invalid chunk size"))?;

    if advertised_chunk == 0 || advertised_chunk > STREAM_CHUNK_SIZE {
        return Err(OapObjectError::ChunkTooLarge {
            len: advertised_chunk,
            max: STREAM_CHUNK_SIZE,
        });
    }

    let total_bytes = usize::try_from(start.total_bytes).map_err(|_| {
        OapObjectError::InvalidResponse("announced object length does not fit this platform")
    })?;

    let tenant_id = start_frame.header.tenant_id;
    let corr_id = start_frame.header.corr_id;
    let end_index = frames.len() - 1;

    let mut reconstructed = Vec::new();
    let mut expected_seq = 0_u64;

    for raw_frame in &frames[1..end_index] {
        let frame = normalize_frame(raw_frame.clone())?;

        validate_data_frame(&frame, tenant_id, corr_id)?;

        let data: Data = decode_payload(
            &frame,
            OapObjectError::InvalidResponse("missing or invalid DATA payload"),
        )?;

        if &data.obj != expected {
            return Err(OapObjectError::InvalidResponse(
                "DATA object does not match requested object",
            ));
        }

        if data.seq != expected_seq {
            return Err(OapObjectError::InvalidResponse(
                "DATA sequence is not contiguous",
            ));
        }

        if data.bytes.len() > advertised_chunk || data.bytes.len() > STREAM_CHUNK_SIZE {
            return Err(OapObjectError::ChunkTooLarge {
                len: data.bytes.len(),
                max: advertised_chunk.min(STREAM_CHUNK_SIZE),
            });
        }

        let next_len = reconstructed
            .len()
            .checked_add(data.bytes.len())
            .ok_or(OapObjectError::InvalidResponse("object length overflow"))?;

        if next_len > total_bytes {
            return Err(OapObjectError::InvalidResponse(
                "DATA exceeds announced object length",
            ));
        }

        reconstructed.extend_from_slice(&data.bytes);

        expected_seq = expected_seq
            .checked_add(1)
            .ok_or(OapObjectError::InvalidResponse("DATA sequence overflow"))?;
    }

    let end_frame = normalize_frame(frames[end_index].clone())?;
    validate_end_frame(&end_frame, tenant_id, corr_id)?;

    let end: End = decode_payload(
        &end_frame,
        OapObjectError::InvalidResponse("missing or invalid END payload"),
    )?;

    if !end.ok || end.error.is_some() {
        return Err(OapObjectError::InvalidResponse(
            "END reported an unsuccessful object stream",
        ));
    }

    if end.seq_end != expected_seq {
        return Err(OapObjectError::InvalidResponse(
            "END sequence does not match DATA count",
        ));
    }

    if reconstructed.len() != total_bytes {
        return Err(OapObjectError::InvalidResponse(
            "reconstructed length does not match START",
        ));
    }

    let bytes = Bytes::from(reconstructed);
    verify_digest(expected, &bytes)?;

    Ok(bytes)
}

fn build_response_stream(
    obj: &ContentId,
    object: Bytes,
    tenant_id: u128,
    corr_id: u64,
) -> Result<Vec<Frame>, OapObjectError> {
    let total_bytes = u64::try_from(object.len()).map_err(|_| {
        OapObjectError::InvalidResponse("object length does not fit OAP stream metadata")
    })?;

    let start = ObjStreamStart {
        obj: obj.clone(),
        total_bytes,
        chunk_bytes: u32::try_from(STREAM_CHUNK_SIZE).expect("64 KiB must fit u32"),
    };

    let mut frames = Vec::new();

    frames.push(normalize_frame(
        FrameBuilder::response(OBJ_GET_APP_PROTO_ID, tenant_id, corr_id, StatusCode::Ok)
            .start()
            .payload(encode_json(&start)?)
            .build(),
    )?);

    let mut seq = 0_u64;

    for chunk in object.chunks(STREAM_CHUNK_SIZE) {
        let data = Data {
            obj: obj.clone(),
            seq,
            bytes: chunk.to_vec(),
        };

        frames.push(normalize_frame(
            FrameBuilder::response(
                OBJ_GET_APP_PROTO_ID,
                tenant_id,
                corr_id,
                StatusCode::Partial,
            )
            .payload(encode_json(&data)?)
            .build(),
        )?);

        seq = seq
            .checked_add(1)
            .ok_or(OapObjectError::InvalidResponse("DATA sequence overflow"))?;
    }

    let end = End {
        seq_end: seq,
        ok: true,
        error: None,
    };

    frames.push(normalize_frame(
        FrameBuilder::response(OBJ_GET_APP_PROTO_ID, tenant_id, corr_id, StatusCode::Ok)
            .end()
            .payload(encode_json(&end)?)
            .build(),
    )?);

    Ok(frames)
}

fn validate_request_frame(frame: &Frame) -> Result<(), OapObjectError> {
    let required = Flags::REQ | Flags::START | Flags::END;
    let forbidden = Flags::RESP | Flags::EVENT | Flags::COMP | Flags::APP_E2E;

    if !frame.header.flags.contains(required) || frame.header.flags.intersects(forbidden) {
        return Err(OapObjectError::InvalidRequest(
            "OBJ_GET must be one REQ|START|END frame",
        ));
    }

    if frame.header.app_proto_id != OBJ_GET_APP_PROTO_ID {
        return Err(OapObjectError::InvalidRequest(
            "unexpected app protocol identifier",
        ));
    }

    if frame.header.code != 0 {
        return Err(OapObjectError::InvalidRequest(
            "request status code must be zero",
        ));
    }

    if frame.payload.is_none() {
        return Err(OapObjectError::InvalidRequest(
            "OBJ_GET payload is required",
        ));
    }

    Ok(())
}

fn validate_start_frame(frame: &Frame) -> Result<(), OapObjectError> {
    let required = Flags::RESP | Flags::START;
    let forbidden = Flags::REQ | Flags::EVENT | Flags::END | Flags::COMP | Flags::APP_E2E;

    if !frame.header.flags.contains(required) || frame.header.flags.intersects(forbidden) {
        return Err(OapObjectError::InvalidResponse(
            "first response must be RESP|START",
        ));
    }

    validate_response_identity(frame)?;

    if frame.header.code != StatusCode::Ok as u16 {
        return Err(OapObjectError::InvalidResponse("START status must be 200"));
    }

    Ok(())
}

fn validate_data_frame(frame: &Frame, tenant_id: u128, corr_id: u64) -> Result<(), OapObjectError> {
    let forbidden =
        Flags::REQ | Flags::EVENT | Flags::START | Flags::END | Flags::COMP | Flags::APP_E2E;

    if !frame.header.flags.contains(Flags::RESP) || frame.header.flags.intersects(forbidden) {
        return Err(OapObjectError::InvalidResponse(
            "DATA response flags are invalid",
        ));
    }

    validate_response_identity(frame)?;
    validate_stream_identity(frame, tenant_id, corr_id)?;

    if frame.header.code != StatusCode::Partial as u16 {
        return Err(OapObjectError::InvalidResponse("DATA status must be 206"));
    }

    Ok(())
}

fn validate_end_frame(frame: &Frame, tenant_id: u128, corr_id: u64) -> Result<(), OapObjectError> {
    let required = Flags::RESP | Flags::END;
    let forbidden = Flags::REQ | Flags::EVENT | Flags::START | Flags::COMP | Flags::APP_E2E;

    if !frame.header.flags.contains(required) || frame.header.flags.intersects(forbidden) {
        return Err(OapObjectError::InvalidResponse(
            "last response must be RESP|END",
        ));
    }

    validate_response_identity(frame)?;
    validate_stream_identity(frame, tenant_id, corr_id)?;

    if frame.header.code != StatusCode::Ok as u16 {
        return Err(OapObjectError::InvalidResponse("END status must be 200"));
    }

    Ok(())
}

fn validate_response_identity(frame: &Frame) -> Result<(), OapObjectError> {
    if frame.header.app_proto_id != OBJ_GET_APP_PROTO_ID {
        return Err(OapObjectError::InvalidResponse(
            "unexpected app protocol identifier",
        ));
    }

    if frame.cap.is_some() {
        return Err(OapObjectError::InvalidResponse(
            "response stream must not carry capability bytes",
        ));
    }

    Ok(())
}

fn validate_stream_identity(
    frame: &Frame,
    tenant_id: u128,
    corr_id: u64,
) -> Result<(), OapObjectError> {
    if frame.header.tenant_id != tenant_id || frame.header.corr_id != corr_id {
        return Err(OapObjectError::InvalidResponse(
            "tenant or correlation ID changed during stream",
        ));
    }

    Ok(())
}

fn encode_frame_stream(frames: &[Frame]) -> Result<Bytes, OapObjectError> {
    let mut writer = OapWriter::with_default();

    for frame in frames {
        writer
            .encode_to_buf(frame.clone())
            .map_err(map_encode_error)?;
    }

    Ok(writer.take_buf())
}

fn decode_wire_frames(wire: &[u8]) -> Result<Vec<Frame>, OapObjectError> {
    if wire.is_empty() {
        return Err(OapObjectError::InvalidResponse("OAP wire stream is empty"));
    }

    let mut parser = ParserState::with_default();
    let mut frames = Vec::new();

    // Feed bounded pieces so a multi-frame object response can exceed the
    // parser's soft aggregate buffer cap without any individual frame
    // exceeding the canonical 1 MiB protocol limit.
    for chunk in wire.chunks(STREAM_CHUNK_SIZE) {
        parser.push(chunk).map_err(map_decode_error)?;
        frames.extend(parser.drain().map_err(map_decode_error)?);
    }

    if parser.buffered_len() != 0 {
        return Err(OapObjectError::InvalidResponse(
            "OAP wire stream ended with an incomplete frame",
        ));
    }

    if frames.is_empty() {
        return Err(OapObjectError::InvalidResponse(
            "OAP wire stream contained no frames",
        ));
    }

    Ok(frames)
}

fn normalize_frame(frame: Frame) -> Result<Frame, OapObjectError> {
    let mut writer = OapWriter::with_default();

    writer.encode_to_buf(frame).map_err(map_encode_error)?;

    let wire = writer.take_buf();
    let mut parser = ParserState::with_default();

    parser.push(&wire).map_err(map_decode_error)?;

    let decoded =
        parser
            .try_next()
            .map_err(map_decode_error)?
            .ok_or(OapObjectError::InvalidResponse(
                "encoded frame was incomplete",
            ))?;

    if parser.buffered_len() != 0 {
        return Err(OapObjectError::InvalidResponse(
            "encoded frame left trailing bytes",
        ));
    }

    Ok(decoded)
}

fn encode_json<T: Serialize>(value: &T) -> Result<Bytes, OapObjectError> {
    serde_json::to_vec(value)
        .map(Bytes::from)
        .map_err(|err| OapObjectError::Json(err.to_string()))
}

fn decode_payload<T: DeserializeOwned>(
    frame: &Frame,
    missing: OapObjectError,
) -> Result<T, OapObjectError> {
    let payload = frame.payload.as_ref().ok_or(missing)?;

    serde_json::from_slice(payload).map_err(|err| OapObjectError::Json(err.to_string()))
}

fn verify_digest(expected: &ContentId, bytes: &[u8]) -> Result<(), OapObjectError> {
    let actual = format!("b3:{}", blake3::hash(bytes).to_hex());

    if actual != expected.as_str() {
        return Err(OapObjectError::DigestMismatch {
            expected: expected.to_string(),
            actual,
        });
    }

    Ok(())
}

fn map_storage_error(err: StorageError) -> OapObjectError {
    match err {
        StorageError::NotFound => OapObjectError::NotFound,
        other => OapObjectError::Storage(other.to_string()),
    }
}

fn map_encode_error(err: OapEncodeError) -> OapObjectError {
    match err {
        OapEncodeError::FrameTooLarge { len, max } => OapObjectError::FrameTooLarge { len, max },
        other => OapObjectError::Codec(other.to_string()),
    }
}

fn map_decode_error(err: OapDecodeError) -> OapObjectError {
    match err {
        OapDecodeError::FrameTooLarge { len, max } => OapObjectError::FrameTooLarge { len, max },
        other => OapObjectError::Codec(other.to_string()),
    }
}

/// Compile-time relationship guard for the two canonical bounds.
const _: () = {
    assert!(STREAM_CHUNK_SIZE < MAX_FRAME_BYTES as usize);
};
