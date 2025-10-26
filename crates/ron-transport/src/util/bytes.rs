//! RO:WHAT — Byte helpers (cap checks).
//! RO:INVARIANTS — enforce MAX_FRAME_BYTES before alloc.

use bytes::BytesMut;

pub fn reserve_capped(buf: &mut BytesMut, want: usize, cap: usize) -> Result<(), &'static str> {
    if want > cap {
        return Err("cap_exceeded");
    }
    buf.reserve(want);
    Ok(())
}
