//! RO:WHAT — Per-connection reader task (frame-capped, timed).
//! RO:INVARIANTS — cap before alloc; owned bytes; cancel-safe; idle/read timeouts.
//! RO:DESIGN — Generic over any AsyncRead, so it supports TcpStream and TlsStream.

use bytes::BytesMut;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::time::{timeout, Duration, Instant};

use crate::limits::MAX_FRAME_BYTES;

#[derive(Debug, Default, Clone)]
pub struct ReaderStats {
    pub bytes_in: u64,
}

pub async fn run<R>(
    mut rd: R,
    read_timeout: Duration,
    idle_timeout: Duration,
) -> std::io::Result<ReaderStats>
where
    R: AsyncRead + Unpin,
{
    let mut buf = BytesMut::with_capacity(8 * 1024);
    let mut stats = ReaderStats::default();
    let mut last = Instant::now();

    loop {
        // Per-op read timeout.
        let n = match timeout(read_timeout, rd.read_buf(&mut buf)).await {
            Ok(Ok(0)) => return Ok(stats), // peer closed
            Ok(Ok(n)) => n,
            Ok(Err(e)) => return Err(e),
            Err(_elapsed) => {
                // If we've been entirely idle longer than idle_timeout, close.
                if last.elapsed() > idle_timeout {
                    return Ok(stats);
                }
                continue; // allow another attempt until idle threshold trips
            }
        };

        last = Instant::now();
        stats.bytes_in += n as u64;

        if buf.len() > MAX_FRAME_BYTES {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "frame too large",
            ));
        }

        // MVP: drain; upper layers will parse OAP frames.
        if !buf.is_empty() {
            buf.clear();
        }
    }
}
