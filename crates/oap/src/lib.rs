#![forbid(unsafe_code)]
// OAP/1 tiny codec + DATA packing helpers (b3:<hex>), per Microkernel blueprint:
// - max_frame = 1 MiB (protocol default)
// - DATA payload layout: [u16 header_len][header JSON][raw body]
// - header MUST include obj:"b3:<hex>" (BLAKE3-256 of the *plaintext* body)

use bytes::{BufMut, Bytes, BytesMut};
use serde_json::Value as Json;
use std::{convert::TryFrom, io};
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub const OAP_VERSION: u8 = 0x1;
pub const DEFAULT_MAX_FRAME: usize = 1 << 20; // 1 MiB

#[inline]
fn json_vec(value: serde_json::Value, ctx: &'static str) -> Vec<u8> {
    match serde_json::to_vec(&value) {
        Ok(v) => v,
        Err(e) => {
            // Non-panicking path: log to stderr, return empty payload.
            // (Keeps public API stable and avoids crashing on serialization failure.)
            eprintln!("oap: failed to serialize {} payload: {}", ctx, e);
            Vec::new()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FrameType {
    Hello = 0x01,
    Start = 0x02,
    Data = 0x03,
    End = 0x04,
    Ack = 0x05,
    Error = 0x06,
}

impl TryFrom<u8> for FrameType {
    type Error = OapError;
    // NOTE: Returning OapError explicitly avoids ambiguity with the `Error` variant.
    fn try_from(b: u8) -> Result<Self, OapError> {
        Ok(match b {
            0x01 => FrameType::Hello,
            0x02 => FrameType::Start,
            0x03 => FrameType::Data,
            0x04 => FrameType::End,
            0x05 => FrameType::Ack,
            0x06 => FrameType::Error,
            other => return Err(OapError::UnknownType(other)),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OapFrame {
    pub ver: u8,
    pub typ: FrameType,
    pub payload: Bytes,
}

#[derive(Debug, Error)]
pub enum OapError {
    #[error("io: {0}")]
    Io(#[from] io::Error),

    #[error("invalid version: {0}")]
    InvalidVersion(u8),

    #[error("unknown frame type: 0x{0:02x}")]
    UnknownType(u8),

    #[error("payload too large: {len} > max_frame {max}")]
    PayloadTooLarge { len: usize, max: usize },

    #[error("header too large: {0} > u16::MAX")]
    HeaderTooLarge(usize),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("DATA decode short header")]
    DataShortHeader,
}

impl OapFrame {
    pub fn new(typ: FrameType, payload: impl Into<Bytes>) -> Self {
        Self {
            ver: OAP_VERSION,
            typ,
            payload: payload.into(),
        }
    }
}

/// Write a single frame to the stream: ver(1) typ(1) len(4) payload(len)
pub async fn write_frame<W: AsyncWrite + Unpin>(
    w: &mut W,
    frame: &OapFrame,
    max_frame: usize,
) -> Result<(), OapError> {
    let len = frame.payload.len();
    if frame.ver != OAP_VERSION {
        return Err(OapError::InvalidVersion(frame.ver));
    }
    if len > max_frame {
        return Err(OapError::PayloadTooLarge {
            len,
            max: max_frame,
        });
    }
    w.write_u8(frame.ver).await?;
    w.write_u8(frame.typ as u8).await?;
    w.write_u32(len as u32).await?;
    if len > 0 {
        w.write_all(&frame.payload).await?;
    }
    w.flush().await?;
    Ok(())
}

/// Read a single frame from the stream (validates version and size).
pub async fn read_frame<R: AsyncRead + Unpin>(
    r: &mut R,
    max_frame: usize,
) -> Result<OapFrame, OapError> {
    let ver = r.read_u8().await?;
    if ver != OAP_VERSION {
        return Err(OapError::InvalidVersion(ver));
    }
    let typ = FrameType::try_from(r.read_u8().await?)?;
    let len = r.read_u32().await? as usize;
    if len > max_frame {
        return Err(OapError::PayloadTooLarge {
            len,
            max: max_frame,
        });
    }
    let mut buf = vec![0u8; len];
    if len > 0 {
        r.read_exact(&mut buf).await?;
    }
    Ok(OapFrame {
        ver,
        typ,
        payload: Bytes::from(buf),
    })
}

/// Compute canonical object id "b3:<hex>" for plaintext bytes.
pub fn b3_of(bytes: &[u8]) -> String {
    let hash = blake3::hash(bytes);
    let hex = hex::encode(hash.as_bytes());
    format!("b3:{hex}")
}

/// DATA packing: `[u16 header_len][header JSON][raw body]`
/// Ensures `obj:"b3:<hex>"` is present in header (adds if missing).
pub fn encode_data_payload(mut header: Json, body: &[u8]) -> Result<Bytes, OapError> {
    if !header.is_object() {
        // Promote non-object to object with single "meta" field
        header = serde_json::json!({ "meta": header });
    }
    let obj = header
        .get("obj")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| b3_of(body));

    // Insert/overwrite obj
    if let Some(map) = header.as_object_mut() {
        map.insert("obj".into(), Json::String(obj));
    }

    let hdr_bytes = serde_json::to_vec(&header)?;
    if hdr_bytes.len() > u16::MAX as usize {
        return Err(OapError::HeaderTooLarge(hdr_bytes.len()));
    }

    let mut out = BytesMut::with_capacity(2 + hdr_bytes.len() + body.len());
    out.put_u16(hdr_bytes.len() as u16);
    out.extend_from_slice(&hdr_bytes);
    out.extend_from_slice(body);
    Ok(out.freeze())
}

/// Decode a DATA payload into (header JSON, body bytes).
pub fn decode_data_payload(payload: &[u8]) -> Result<(Json, Bytes), OapError> {
    if payload.len() < 2 {
        return Err(OapError::DataShortHeader);
    }
    let hdr_len = u16::from_be_bytes([payload[0], payload[1]]) as usize;
    if payload.len() < 2 + hdr_len {
        return Err(OapError::DataShortHeader);
    }
    let hdr = &payload[2..2 + hdr_len];
    let body = &payload[2 + hdr_len..];
    let header_json: Json = serde_json::from_slice(hdr)?;
    Ok((header_json, Bytes::copy_from_slice(body)))
}

/// Helper: build a DATA frame and enforce `max_frame`.
pub fn data_frame(header: Json, body: &[u8], max_frame: usize) -> Result<OapFrame, OapError> {
    let payload = encode_data_payload(header, body)?;
    if payload.len() > max_frame {
        return Err(OapError::PayloadTooLarge {
            len: payload.len(),
            max: max_frame,
        });
    }
    Ok(OapFrame::new(FrameType::Data, payload))
}

/// Small helpers to build common frames
pub fn hello_frame(proto_id: &str) -> OapFrame {
    let payload = json_vec(serde_json::json!({ "hello": proto_id }), "hello");
    OapFrame::new(FrameType::Hello, payload)
}
pub fn start_frame(topic: &str) -> OapFrame {
    let payload = json_vec(serde_json::json!({ "topic": topic }), "start");
    OapFrame::new(FrameType::Start, payload)
}
pub fn end_frame() -> OapFrame {
    OapFrame::new(FrameType::End, Bytes::new())
}
pub fn ack_frame(credit_bytes: u64) -> OapFrame {
    let payload = json_vec(serde_json::json!({ "credit": credit_bytes }), "ack");
    OapFrame::new(FrameType::Ack, payload)
}
pub fn quota_error_frame(reason: &str) -> OapFrame {
    let payload = json_vec(serde_json::json!({ "code":"quota", "msg": reason }), "err");
    OapFrame::new(FrameType::Error, payload)
}

// --- Tests are in /tests to keep this crate lean ---
