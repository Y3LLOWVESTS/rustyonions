//! RO:WHAT — Parser state machine: push bytes, pop frames.
//! RO:WHY  — Make partial reads easy without reinventing decoding logic.
//! RO:INTERACTS — `OapDecoder`; returns `Frame`s as they become available.
//! RO:INVARIANTS — No blocking; honors parser config soft cap; zero `unsafe`.

use bytes::BytesMut;
use tokio_util::codec::Decoder; // bring `decode` into scope

use super::ParserConfig;
use crate::{codec::OapDecoder, Frame};

#[derive(Debug)]
pub struct ParserState {
    dec: OapDecoder,
    buf: BytesMut,
    cfg: ParserConfig,
}

impl ParserState {
    pub fn new(cfg: ParserConfig) -> Self {
        Self {
            dec: OapDecoder::default(),
            buf: BytesMut::new(),
            cfg,
        }
    }

    pub fn with_default() -> Self {
        Self::new(ParserConfig::default())
    }

    /// Feed raw bytes into the parser buffer.
    /// Returns `Ok(())` even if no full frame is available yet.
    pub fn push(&mut self, chunk: &[u8]) -> Result<(), crate::OapDecodeError> {
        self.buf.extend_from_slice(chunk);

        // Soft cap (best-effort): callers decide response; decoding still enforces per-frame caps.
        if let Some(max) = self.cfg.max_buffer_bytes {
            if self.buf.len() > max {
                // Return a decode error to signal backpressure to caller (no new variant needed).
                return Err(crate::OapDecodeError::PayloadOutOfBounds);
            }
        }
        Ok(())
    }

    /// Try to decode a single frame if enough data is buffered.
    pub fn try_next(&mut self) -> Result<Option<Frame>, crate::OapDecodeError> {
        self.dec.decode(&mut self.buf)
    }

    /// Drain all currently-available frames.
    pub fn drain(&mut self) -> Result<Vec<Frame>, crate::OapDecodeError> {
        let mut out = Vec::new();
        while let Some(f) = self.dec.decode(&mut self.buf)? {
            out.push(f);
        }
        Ok(out)
    }

    /// Inspect buffered byte count (useful for tests/telemetry).
    pub fn buffered_len(&self) -> usize {
        self.buf.len()
    }
}
