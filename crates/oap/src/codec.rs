//! RO:WHAT — Tokio `Encoder`/`Decoder` for OAP/1 frames (length-prefixed, bounded).
//! RO:WHY — Interop needs a safe, reusable codec with strict limits and optional zstd inflate.
//! RO:INTERACTS — bytes, tokio-util::codec; uses Header/Flags and error taxonomy.
//! RO:INVARIANTS — Enforce MAX_FRAME_BYTES; START-only cap; optional COMP → bounded inflate ≤ 8×.

use bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder};

use crate::constants::MAX_FRAME_BYTES;
use crate::error::{OapDecodeError as DE, OapEncodeError as EE};
use crate::flags::Flags;
use crate::{Frame, Header};

#[cfg(feature = "zstd")]
use bytes::Bytes;
#[cfg(feature = "zstd")]
use crate::constants::MAX_DECOMPRESS_EXPANSION;

#[derive(Debug, Default)]
pub struct OapDecoder;

#[derive(Debug, Default)]
pub struct OapEncoder;

impl Decoder for OapDecoder {
    type Item = Frame;
    type Error = DE; // must be From<std::io::Error>; satisfied via Io(#[from]).

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // Need at least the fixed header.
        if src.len() < Header::WIRE_SIZE {
            return Ok(None);
        }

        // Peek a copy for header parsing without moving yet.
        let mut peek = src.clone().freeze();
        let hdr = Header::read_from(&mut peek)?;

        // If we don't have the whole frame yet, wait.
        if src.len() < hdr.len as usize {
            return Ok(None);
        }

        // Split off exactly the frame to work on.
        let mut frame_bytes = src.split_to(hdr.len as usize).freeze();

        // Re-read header from the real slice (advance).
        let _ = Header::read_from(&mut frame_bytes)?; // validated already

        // Cap section
        let cap = if hdr.cap_len > 0 {
            if !hdr.flags.contains(Flags::START) {
                return Err(DE::CapOnNonStart);
            }
            if frame_bytes.len() < hdr.cap_len as usize {
                return Err(DE::CapOutOfBounds);
            }
            Some(frame_bytes.split_to(hdr.cap_len as usize))
        } else {
            None
        };

        // Remaining is payload (may be empty)
        let payload = if frame_bytes.is_empty() {
            None
        } else {
            Some(frame_bytes)
        };

        // Optional bounded decompression when COMP flag set
        #[cfg(feature = "zstd")]
        let mut payload = payload;

        if hdr.flags.contains(Flags::COMP) {
            #[cfg(not(feature = "zstd"))]
            {
                return Err(DE::ZstdFeatureNotEnabled);
            }
            #[cfg(feature = "zstd")]
            {
                if let Some(body) = payload.take() {
                    let max_out = (MAX_FRAME_BYTES * MAX_DECOMPRESS_EXPANSION) as usize;
                    let mut dec = zstd::stream::read::Decoder::new(std::io::Cursor::new(body))
                        .map_err(|e| DE::Zstd(e.to_string()))?;
                    use std::io::Read;
                    let mut out = Vec::new();
                    let mut buf = [0u8; 16 * 1024];
                    loop {
                        let n = dec.read(&mut buf)?;
                        if n == 0 { break; }
                        out.extend_from_slice(&buf[..n]);
                        if out.len() > max_out {
                            return Err(DE::DecompressBoundExceeded);
                        }
                    }
                    payload = Some(Bytes::from(out));
                }
            }
        }

        Ok(Some(Frame { header: hdr, cap, payload }))
    }
}

impl Encoder<Frame> for OapEncoder {
    type Error = EE; // must be From<std::io::Error>; satisfied via Io(#[from]).

    fn encode(&mut self, item: Frame, dst: &mut BytesMut) -> Result<(), Self::Error> {
        // Sanity on lengths vs declared header.len
        let cap_len = item.cap.as_ref().map(|b| b.len()).unwrap_or(0);
        let payload_len = item.payload.as_ref().map(|b| b.len()).unwrap_or(0);
        if cap_len > u16::MAX as usize {
            return Err(EE::CapOutOfBounds);
        }
        let total_len = Header::WIRE_SIZE + cap_len + payload_len;
        if total_len > MAX_FRAME_BYTES as usize {
            return Err(EE::FrameTooLarge { len: total_len as u32, max: MAX_FRAME_BYTES });
        }

        // Write header (with corrected cap_len & len)
        let mut hdr = item.header;
        hdr.cap_len = cap_len as u16;
        hdr.len = total_len as u32;

        // Reserve and emit
        dst.reserve(total_len);
        hdr.put_to(dst);
        if let Some(cap) = item.cap {
            dst.extend_from_slice(&cap);
        }
        if let Some(payload) = item.payload {
            dst.extend_from_slice(&payload);
        }
        Ok(())
    }
}
