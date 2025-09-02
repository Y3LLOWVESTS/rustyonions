#![forbid(unsafe_code)]

use bytes::{BufMut, BytesMut};

use crate::errors::{Error, Result};
use crate::oap::flags::OapFlags;
use crate::oap::frame::OapFrame;
use super::OapCodec;

pub(super) fn encode_frame(codec: &mut OapCodec, item: OapFrame, dst: &mut BytesMut) -> Result<()> {
    // Validate START/cap invariants before writing.
    if !item.cap.is_empty() && !item.flags.contains(OapFlags::START) {
        return Err(Error::Protocol(
            "cap present on non-START frame (invalid)".into(),
        ));
    }

    let cap_len = item.cap.len();
    let body_len = 1 + 2 + 2 + 2 + 16 + 2 + 8 + cap_len + item.payload.len();

    if body_len > codec.max_frame {
        return Err(Error::Decode(format!(
            "frame too large to encode: {} > {}",
            body_len, codec.max_frame
        )));
    }

    // Reserve: 4 for len + body_len
    dst.reserve(4 + body_len);

    // len
    dst.put_u32_le(body_len as u32);

    // ver
    dst.put_u8(item.ver);

    // flags, code, app id
    dst.put_u16_le(item.flags.bits());
    dst.put_u16_le(item.code);
    dst.put_u16_le(item.app_proto_id);

    // tenant (u128 LE)
    dst.put_slice(&item.tenant_id.to_le_bytes());

    // cap len
    dst.put_u16_le(cap_len as u16);

    // corr_id
    dst.put_u64_le(item.corr_id);

    // cap (if any)
    if cap_len > 0 {
        dst.put_slice(&item.cap);
    }

    // payload
    dst.put_slice(&item.payload);

    Ok(())
}
