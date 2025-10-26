//! RO:WHAT — Read loop: accumulates bytes, decodes OAP frames.

use bytes::BytesMut;
use tokio::io::{AsyncRead, AsyncReadExt};
use tracing::{debug, trace};

use super::error::ConnResult;
use crate::protocol::oap::{try_parse_frame, FrameKind}; // <- keep only ConnResult

/// Blocking-ish read loop that parses frames and logs; returns on EOF or error.
/// Transport-agnostic: any AsyncRead works (TCP/TLS/QUIC streams that implement it).
pub async fn run_reader<R>(mut rd: R) -> ConnResult<()>
where
    R: AsyncRead + Unpin,
{
    let mut buf = BytesMut::with_capacity(8 * 1024);

    loop {
        // Try parse any already-buffered frames first.
        while let Some(frame) = try_parse_frame(&mut buf)? {
            match frame.kind {
                FrameKind::Data => {
                    debug!(len = frame.payload.len(), "oap/data frame");
                }
                FrameKind::Ctrl => {
                    debug!(len = frame.payload.len(), "oap/ctrl frame");
                }
            }
            trace!(buf_len = buf.len(), "post-parse buffer");
        }

        // Refill buffer. read_buf appends into BytesMut.
        let n = rd.read_buf(&mut buf).await?;
        if n == 0 {
            // EOF
            return Ok(());
        }
        trace!(read = n, buf_len = buf.len(), "read bytes");
    }
}
