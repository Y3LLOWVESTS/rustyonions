#![forbid(unsafe_code)]

use bytes::BytesMut;
use bytes::{Buf, Bytes};

use super::OapCodec;
use crate::constants::OAP_VERSION;
use crate::errors::{Error, Result};
use crate::oap::flags::OapFlags;
use crate::oap::frame::OapFrame;

#[inline]
fn need_bytes(src: &BytesMut, n: usize) -> bool {
    src.len() >= n
}

pub(super) fn decode_frame(codec: &mut OapCodec, src: &mut BytesMut) -> Result<Option<OapFrame>> {
    // Need at least len (4 bytes)
    if !need_bytes(src, 4) {
        return Ok(None);
    }

    let len = {
        let b = &src[..4];
        u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize
    };

    if len > codec.max_frame {
        return Err(Error::Decode(format!(
            "frame too large: {} > {}",
            len, codec.max_frame
        )));
    }

    // Total bytes needed = 4 + len
    if !need_bytes(src, 4 + len) {
        return Ok(None);
    }

    // Consume len prefix
    src.advance(4);
    let mut frame = src.split_to(len);

    // Fixed header sizes
    const FIXED: usize = 1 + 2 + 2 + 2 + 16 + 2 + 8;
    if frame.len() < FIXED {
        return Err(Error::Decode("truncated header".into()));
    }

    let ver = frame.get_u8();
    if ver != OAP_VERSION {
        return Err(Error::Protocol(format!("unsupported version {}", ver)));
    }

    let flags_bits = frame.get_u16_le();
    let flags = OapFlags::from_bits_truncate(flags_bits);

    let code = frame.get_u16_le();
    let app_proto_id = frame.get_u16_le();

    // u128 tenant (little-endian)
    let mut tenant_bytes = [0u8; 16];
    frame.copy_to_slice(&mut tenant_bytes);
    let tenant_id = u128::from_le_bytes(tenant_bytes);

    let cap_len = frame.get_u16_le() as usize;
    let corr_id = frame.get_u64_le();

    if cap_len > frame.len() {
        return Err(Error::Decode("cap_len exceeds frame".into()));
    }

    let cap = if cap_len > 0 {
        if !flags.contains(OapFlags::START) {
            return Err(Error::Protocol(
                "cap present on non-START frame (invalid)".into(),
            ));
        }
        frame.split_to(cap_len).freeze()
    } else {
        Bytes::new()
    };

    // Remaining is payload
    let payload = frame.freeze();

    Ok(Some(OapFrame {
        ver,
        flags,
        code,
        app_proto_id,
        tenant_id,
        cap,
        corr_id,
        payload,
    }))
}
