//! RO:WHAT — High-level OAP frame value (header + optional cap + payload).
//! RO:WHY — Keep parsing/encoding logic separate from transport loops; safe, owned bytes.
//! RO:INTERACTS — Header/Flags; codec reads/writes `Frame` to/from wire.
//! RO:INVARIANTS — Owned buffers; bounds validated; START carries capability bytes only.

use crate::{Header};
use bytes::Bytes;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    pub header: Header,
    pub cap: Option<Bytes>,
    pub payload: Option<Bytes>,
}

impl Frame {
    pub fn new(header: Header, cap: Option<Bytes>, payload: Option<Bytes>) -> Self {
        Self { header, cap, payload }
    }

    pub fn payload_len(&self) -> usize {
        self.payload.as_ref().map(|b| b.len()).unwrap_or(0)
    }

    pub fn cap_len(&self) -> usize {
        self.cap.as_ref().map(|b| b.len()).unwrap_or(0)
    }
}
