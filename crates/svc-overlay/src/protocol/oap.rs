//! RO:WHAT — OAP/1 framing: length-prefixed frames with a kind byte.
//! RO:WHY  — Provide message boundaries over byte-stream transports.
//! RO:INVARIANTS — Max frame size is sourced from ron-proto; parsing is incremental & non-panicking.

use crate::protocol::error::{ProtoError, ProtoResult};
use bytes::{Buf, BufMut, Bytes, BytesMut};

/// Canonical limit from `ron-proto`.
const MAX_FRAME_BYTES: usize = ron_proto::oap::MAX_FRAME_BYTES;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FrameKind {
    /// Application/gossip data frame (payload = opaque).
    Data = 0x01,
    /// Control/handshake or control acks (payload = small).
    Ctrl = 0x02,
}

impl FrameKind {
    fn from_byte(b: u8) -> Option<Self> {
        match b {
            0x01 => Some(FrameKind::Data),
            0x02 => Some(FrameKind::Ctrl),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Frame {
    pub kind: FrameKind,
    pub payload: Bytes,
}

impl Frame {
    /// Encode: 4-byte BE length (kind + payload), then 1-byte kind, then payload.
    pub fn encode_to(&self, out: &mut BytesMut) -> ProtoResult<()> {
        let len = 1usize + self.payload.len();
        if len > MAX_FRAME_BYTES {
            return Err(ProtoError::FrameTooLarge {
                got: len,
                max: MAX_FRAME_BYTES,
            });
        }
        out.reserve(4 + len);
        out.put_u32(len as u32);
        out.put_u8(self.kind as u8);
        out.extend_from_slice(&self.payload);
        Ok(())
    }
}

/// Try to parse a single frame from the buffer; leaves remaining bytes in `buf`.
pub fn try_parse_frame(buf: &mut BytesMut) -> ProtoResult<Option<Frame>> {
    const HDR: usize = 4; // BE length
    if buf.len() < HDR {
        return Ok(None);
    }
    let mut len_bytes = &buf[..HDR];
    let len = len_bytes.get_u32() as usize;

    if len > MAX_FRAME_BYTES {
        return Err(ProtoError::FrameTooLarge {
            got: len,
            max: MAX_FRAME_BYTES,
        });
    }

    if buf.len() < HDR + len {
        // Not enough yet
        return Ok(None);
    }

    buf.advance(HDR);
    let kind_b = buf.get_u8();
    let Some(kind) = FrameKind::from_byte(kind_b) else {
        // Treat as control error; drop this frame safely by consuming payload.
        buf.advance(len - 1);
        return Err(ProtoError::BadPreamble {
            got: [b'F', b'K', kind_b, 0, 0],
        });
    };

    let payload_len = len - 1;
    let payload = buf.split_to(payload_len).freeze();

    Ok(Some(Frame { kind, payload }))
}
