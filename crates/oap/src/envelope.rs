//! RO:WHAT — Ergonomic envelope helpers: capability wrapper and frame builders.
//! RO:WHY  — Reduce boilerplate for callers while enforcing OAP invariants at build time.
//! RO:INTERACTS — Uses Frame/Header/Flags/StatusCode plus HELLO DTOs from `hello.rs`.
//! RO:INVARIANTS — No I/O; the Encoder normalizes `len`/`cap_len`; caps only valid with START.

use bytes::Bytes;

use crate::{
    flags::Flags, hello::{Hello, HelloReply},
    Frame, Header, StatusCode, OAP_VERSION,
};

/// Opaque capability bytes carried on `START` frames.
/// Semantics (macaroons, scopes, etc.) live above OAP.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Capability(Bytes);

impl Capability {
    pub fn new(bytes: Bytes) -> Self { Self(bytes) }
    pub fn as_bytes(&self) -> &Bytes { &self.0 }
    pub fn into_bytes(self) -> Bytes { self.0 }
    /// Size guard for START frames (u16 fit).
    pub fn fits_u16(&self) -> bool { self.0.len() <= u16::MAX as usize }
}

/// True if a sender requests an ACK.
pub fn wants_ack(flags: Flags) -> bool { flags.contains(Flags::ACK_REQ) }
/// True if a frame marks the logical end of a request/stream.
pub fn is_terminal(flags: Flags) -> bool { flags.contains(Flags::END) }
/// True if a frame is fire-and-forget (EVENT without ACK).
pub fn is_fire_and_forget(flags: Flags) -> bool { flags.contains(Flags::EVENT) && !wants_ack(flags) }

/// Minimal, chainable builder for common envelopes.
/// The encoder will set `len`/`cap_len` on write.
#[derive(Debug, Clone)]
pub struct FrameBuilder {
    header: Header,
    cap: Option<Bytes>,
    payload: Option<Bytes>,
}

impl FrameBuilder {
    /// Begin a request for `app_proto_id`.
    pub fn request(app_proto_id: u16, tenant_id: u128, corr_id: u64) -> Self {
        Self {
            header: Header {
                len: 0, ver: OAP_VERSION, flags: Flags::REQ, code: 0,
                app_proto_id, tenant_id, cap_len: 0, corr_id,
            },
            cap: None, payload: None,
        }
    }

    /// Begin a response for `app_proto_id` with a status code.
    pub fn response(app_proto_id: u16, tenant_id: u128, corr_id: u64, code: StatusCode) -> Self {
        Self {
            header: Header {
                len: 0, ver: OAP_VERSION, flags: Flags::RESP, code: code as u16,
                app_proto_id, tenant_id, cap_len: 0, corr_id,
            },
            cap: None, payload: None,
        }
    }

    /// Mark as START and attach capability bytes (REQ is set if not already).
    pub fn start_with_cap(mut self, cap: Bytes) -> Self {
        self.header.flags |= Flags::START | Flags::REQ;
        self.cap = Some(cap);
        self
    }

    /// Attach opaque payload.
    pub fn payload(mut self, p: Bytes) -> Self {
        self.payload = Some(p);
        self
    }

    /// Request ACK.
    pub fn want_ack(mut self) -> Self {
        self.header.flags |= Flags::ACK_REQ;
        self
    }

    /// Mark as END.
    pub fn end(mut self) -> Self {
        self.header.flags |= Flags::END;
        self
    }

    /// Build a `Frame`.
    pub fn build(self) -> Frame {
        Frame { header: self.header, cap: self.cap, payload: self.payload }
    }
}

/// Convenience: HELLO request frame (app_proto_id=0).
pub fn hello_request(ua: Option<&str>, tenant_id: u128, corr_id: u64) -> Frame {
    let h = Hello { ua: ua.map(str::to_owned) };
    h.to_frame(tenant_id, corr_id)
}

/// Convenience: HELLO reply frame (app_proto_id=0), using default server caps.
pub fn hello_reply_default(tenant_id: u128, corr_id: u64) -> Frame {
    HelloReply::default_for_server().to_frame(tenant_id, corr_id)
}
