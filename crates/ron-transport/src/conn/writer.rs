//! RO:WHAT — Per-connection single-writer task with backpressure.
//! RO:WHY  — Enforce single-writer discipline; count bytes_out; await I/O for backpressure.
//! RO:DESIGN — Generic over any AsyncWrite so it works for TcpStream and TlsStream.

use bytes::Bytes;
use tokio::{
    io::AsyncWrite,
    io::AsyncWriteExt,
    sync::mpsc::{self, error::SendError, Sender},
};

/// Handle to enqueue bytes for the connection's writer task.
#[derive(Clone)]
pub struct WriterHandle {
    tx: Sender<Bytes>,
}

impl WriterHandle {
    pub async fn send(&self, b: Bytes) -> Result<(), SendError<Bytes>> {
        self.tx.send(b).await
    }
}

/// Spawn a writer task for any AsyncWrite (TcpStream, TlsStream, ...).
/// Returns a handle for sending bytes and the writer task JoinHandle.
///
/// The writer task:
/// - writes each chunk fully (`write_all`)
/// - flushes periodically (on every message in MVP)
/// - increments `bytes_out` metrics
/// - exits cleanly when channel closes
pub fn spawn_writer<W>(
    mut w: W,
    name: &'static str,
    metrics: crate::metrics::TransportMetrics,
) -> (WriterHandle, tokio::task::JoinHandle<()>)
where
    W: AsyncWrite + Unpin + Send + 'static,
{
    // Bounded queue prevents unbounded memory under slow receivers.
    let (tx, mut rx) = mpsc::channel::<Bytes>(64);
    let jh = tokio::spawn(async move {
        while let Some(chunk) = rx.recv().await {
            if chunk.is_empty() {
                continue;
            }
            if let Err(e) = w.write_all(&chunk).await {
                tracing::debug!(error=%e, "writer: write_all failed");
                break;
            }
            // Count bytes_out
            metrics
                .bytes_out
                .with_label_values(&[name])
                .inc_by(chunk.len() as u64);

            // Flush to minimize tail latency (can batch/tune later).
            if let Err(e) = w.flush().await {
                tracing::debug!(error=%e, "writer: flush failed");
                break;
            }
        }

        // Attempt graceful shutdown for protocols that support it (e.g., TLS close_notify).
        let _ = w.shutdown().await;
    });

    (WriterHandle { tx }, jh)
}
