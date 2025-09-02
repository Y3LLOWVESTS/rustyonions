#![forbid(unsafe_code)]

use bytes::Bytes;

use crate::constants::OAP_VERSION;
use super::flags::OapFlags;

/// OAP/1 frame in host representation.
#[derive(Debug, Clone)]
pub struct OapFrame {
    pub ver: u8,
    pub flags: OapFlags,
    pub code: u16,
    pub app_proto_id: u16,
    pub tenant_id: u128,
    pub cap: Bytes,     // optional; only valid when START set
    pub corr_id: u64,
    pub payload: Bytes, // opaque; may be COMP or APP_E2E
}

impl OapFrame {
    pub fn hello_request() -> Self {
        OapFrame {
            ver: OAP_VERSION,
            flags: OapFlags::empty(), // simple request with empty body
            code: 0,
            app_proto_id: 0, // HELLO
            tenant_id: 0,
            cap: Bytes::new(),
            corr_id: 0,
            payload: Bytes::new(),
        }
    }

    /// Helper for single-shot request (REQ|START|END).
    pub fn oneshot_req(app_proto_id: u16, tenant_id: u128, corr_id: u64, payload: Bytes) -> Self {
        OapFrame {
            ver: OAP_VERSION,
            flags: OapFlags::REQ | OapFlags::START | OapFlags::END,
            code: 0,
            app_proto_id,
            tenant_id,
            cap: Bytes::new(),
            corr_id,
            payload,
        }
    }
}
