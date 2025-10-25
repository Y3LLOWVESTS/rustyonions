//! RO:WHAT — Buffered OAP frame writer for async sinks.
//! RO:WHY  — Provide a simple way to encode frames and flush to `AsyncWrite`.
//! RO:INTERACTS — `OapEncoder`; tokio `AsyncWrite`.
//! RO:INVARIANTS — No locks across `.await`; zero `unsafe`.

pub mod config;

use bytes::BytesMut;
use tokio::io::{AsyncWrite, AsyncWriteExt};
use tokio_util::codec::Encoder; // bring `encode` into scope

use crate::{codec::OapEncoder, Frame};
pub use config::WriterConfig;

/// Buffered OAP writer: encodes frames into an internal buffer,
/// then writes/flushes to an `AsyncWrite`.
#[derive(Debug)]
pub struct OapWriter {
    enc: OapEncoder,
    buf: BytesMut,
    cfg: WriterConfig,
}

impl OapWriter {
    pub fn new(cfg: WriterConfig) -> Self {
        Self {
            enc: OapEncoder::default(),
            buf: BytesMut::new(),
            cfg,
        }
    }

    pub fn with_default() -> Self {
        Self::new(WriterConfig::default())
    }

    /// Encode a frame into the internal buffer (does not perform I/O).
    pub fn encode_to_buf(&mut self, frame: Frame) -> Result<(), crate::OapEncodeError> {
        self.enc.encode(frame, &mut self.buf)
    }

    /// Take the internal buffer as bytes (leaves buffer empty).
    pub fn take_buf(&mut self) -> bytes::Bytes {
        std::mem::take(&mut self.buf).freeze()
    }

    /// Encode and write a frame to an async sink, flushing if buffer exceeds `flush_hint_bytes`.
    pub async fn write_frame<S: AsyncWrite + Unpin>(
        &mut self,
        sink: &mut S,
        frame: Frame,
    ) -> Result<(), crate::OapError> {
        // Encoding errors -> OapEncodeError -> OapError (From impl)
        self.enc.encode(frame, &mut self.buf)?;

        if self.buf.len() >= self.cfg.flush_hint_bytes {
            // I/O errors -> std::io::Error -> OapError (From impl)
            sink.write_all(&self.buf).await?;
            self.buf.clear();
            sink.flush().await?;
        }
        Ok(())
    }

    /// Force-flush any buffered bytes to the sink.
    pub async fn flush<S: AsyncWrite + Unpin>(
        &mut self,
        sink: &mut S,
    ) -> Result<(), crate::OapError> {
        if !self.buf.is_empty() {
            sink.write_all(&self.buf).await?;
            self.buf.clear();
        }
        sink.flush().await?;
        Ok(())
    }
}
