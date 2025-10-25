//! RO:WHAT — Small helpers for OAP/1 HELLO (app_proto_id=0) request/response.
//! RO:WHY — Normalize what clients/servers exchange during negotiation.
//! RO:INTERACTS — Flags/Frame/Header; used by SDK and services at connect time.
//! RO:INVARIANTS — ver=1; code=200 on success; returns server caps & versions.

use crate::{constants::*, flags::Flags, Frame, Header};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use serde_json;

/// Minimal HELLO request (client → server).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hello {
    /// Optional user-agent/version string.
    pub ua: Option<String>,
}

/// Minimal HELLO reply (server → client).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelloReply {
    pub max_frame: u32,
    pub max_inflight: u16,
    pub flags_supported: u16,
    pub versions: Vec<u16>,
    pub transports: Vec<String>,
}

impl Hello {
    pub fn to_frame(&self, tenant_id: u128, corr_id: u64) -> Frame {
        let payload = serde_json::to_vec(self).expect("serialize hello");
        let header = Header {
            len: 0, // filled by encoder
            ver: OAP_VERSION,
            flags: Flags::REQ,
            code: 0,
            app_proto_id: 0,
            tenant_id,
            cap_len: 0,
            corr_id,
        };
        Frame {
            header,
            cap: None,
            payload: Some(Bytes::from(payload)),
        }
    }
}

impl HelloReply {
    pub fn default_for_server() -> Self {
        Self {
            max_frame: MAX_FRAME_BYTES,
            max_inflight: 64,
            flags_supported: (Flags::REQ
                | Flags::RESP
                | Flags::EVENT
                | Flags::START
                | Flags::END
                | Flags::ACK_REQ
                | Flags::COMP
                | Flags::APP_E2E)
                .bits(),
            versions: vec![OAP_VERSION],
            transports: vec!["tcp+tls".into()],
        }
    }

    pub fn from_frame(frame: &Frame) -> Result<Self, crate::error::OapDecodeError> {
        let Some(payload) = &frame.payload else {
            return Err(crate::error::OapDecodeError::PayloadOutOfBounds);
        };
        serde_json::from_slice(payload)
            .map_err(|e| crate::error::OapDecodeError::Zstd(e.to_string()))
    }

    pub fn to_frame(&self, tenant_id: u128, corr_id: u64) -> Frame {
        let json = serde_json::to_vec(self).expect("serialize hello reply");
        let header = Header {
            len: 0,
            ver: OAP_VERSION,
            flags: Flags::RESP,
            code: crate::error::StatusCode::Ok as u16,
            app_proto_id: 0,
            tenant_id,
            cap_len: 0,
            corr_id,
        };
        Frame {
            header,
            cap: None,
            payload: Some(Bytes::from(json)),
        }
    }
}
