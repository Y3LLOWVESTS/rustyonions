//! RO:WHAT — OAP/1 fixed header (without [cap] and [payload]) and (de)serialization helpers.
//! RO:WHY — Deterministic, endian-stable wire header; validates size/field bounds before alloc.
//! RO:INTERACTS — Flags; codec uses this to parse before reading variable sections.
//! RO:INVARIANTS — Length checked ≤ MAX_FRAME_BYTES; version=1; cap_len for START frames only.

use crate::constants::{MAX_FRAME_BYTES, OAP_VERSION};
use crate::flags::Flags;
use bytes::{Buf, BufMut, BytesMut};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Header {
    /// Total frame length in bytes (header + cap + payload).
    pub len: u32,
    /// Protocol version (must be 1).
    pub ver: u16,
    /// Bit flags.
    pub flags: Flags,
    /// Status or app code (e.g., 2xx/4xx/5xx or app-defined).
    pub code: u16,
    /// Application protocol id (0 = control/HELLO).
    pub app_proto_id: u16,
    /// Tenant id (128-bit).
    pub tenant_id: u128,
    /// Capability section length in bytes (only valid/allowed with START).
    pub cap_len: u16,
    /// Correlation id (64-bit).
    pub corr_id: u64,
}

impl Header {
    pub const WIRE_SIZE: usize = 4 + 2 + 2 + 2 + 2 + 16 + 2 + 8;

    pub fn validate(&self) -> Result<(), crate::error::OapDecodeError> {
        if self.ver != OAP_VERSION {
            return Err(crate::error::OapDecodeError::BadVersion(self.ver));
        }
        if self.len == 0 || self.len > MAX_FRAME_BYTES {
            return Err(crate::error::OapDecodeError::FrameTooLarge {
                len: self.len,
                max: MAX_FRAME_BYTES,
            });
        }
        if self.cap_len > 0 && !self.flags.contains(Flags::START) {
            return Err(crate::error::OapDecodeError::CapOnNonStart);
        }
        Ok(())
    }

    pub fn put_to(&self, dst: &mut BytesMut) {
        dst.put_u32(self.len);
        dst.put_u16(self.ver);
        dst.put_u16(self.flags.bits());
        dst.put_u16(self.code);
        dst.put_u16(self.app_proto_id);
        dst.put_u128(self.tenant_id);
        dst.put_u16(self.cap_len);
        dst.put_u64(self.corr_id);
    }

    pub fn read_from(src: &mut bytes::Bytes) -> Result<Self, crate::error::OapDecodeError> {
        use crate::error::OapDecodeError as E;
        if src.len() < Self::WIRE_SIZE {
            return Err(E::TruncatedHeader);
        }
        let len = src.get_u32();
        let ver = src.get_u16();
        let flags_bits = src.get_u16();
        let code = src.get_u16();
        let app_proto_id = src.get_u16();
        let tenant_id = src.get_u128();
        let cap_len = src.get_u16();
        let corr_id = src.get_u64();
        let flags = Flags::from_bits(flags_bits).ok_or(E::BadFlags(flags_bits))?;
        let hdr = Header {
            len,
            ver,
            flags,
            code,
            app_proto_id,
            tenant_id,
            cap_len,
            corr_id,
        };
        hdr.validate()?;
        Ok(hdr)
    }
}
