//! RO:WHAT — Bounded per-connection TX queue with single-writer task.
//! RO:WHY  — Enforces single-writer discipline and exposes queue depth/drops.
//! RO:INTERACTS — conn::writer::write_frame, admin::metrics::overlay_metrics
//! RO:INVARIANTS — one writer per connection; bounded mpsc; no locks across .await
//! RO:METRICS — overlay_peer_queue_depth, overlay_peer_dropped_total
//! RO:TEST — covered indirectly by roundtrip; unit tests can enqueue/close

use bytes::BytesMut;
use tokio::io::AsyncWrite;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{debug, warn};

use crate::admin::metrics::overlay_metrics;
use crate::protocol::oap::Frame;

/// Message to the writer task.
pub enum TxMsg {
    Frame(Frame),
    Close,
}

/// Handle to enqueue frames for a single connection.
#[derive(Clone)]
pub struct TxSender {
    tx: mpsc::Sender<TxMsg>,
}

impl TxSender {
    pub fn capacity(&self) -> usize {
        self.tx.capacity()
    }

    pub fn try_send(&self, frame: Frame) -> Result<(), Frame> {
        match self.tx.try_send(TxMsg::Frame(frame)) {
            Ok(_) => {
                overlay_metrics::set_peer_tx_depth(self.tx.max_capacity() - self.tx.capacity());
                Ok(())
            }
            Err(mpsc::error::TrySendError::Full(TxMsg::Frame(f))) => {
                overlay_metrics::inc_peer_tx_dropped();
                Err(f)
            }
            Err(mpsc::error::TrySendError::Closed(TxMsg::Frame(f))) => Err(f),
            Err(_) => unreachable!("only Frame variants used here"),
        }
    }

    pub async fn send(&self, frame: Frame) -> Result<(), Frame> {
        // Avoid use-after-move by staging in an Option.
        let mut slot = Some(frame);
        match self.tx.send(TxMsg::Frame(slot.take().unwrap())).await {
            Ok(_) => {
                overlay_metrics::set_peer_tx_depth(self.tx.max_capacity() - self.tx.capacity());
                Ok(())
            }
            Err(mpsc::error::SendError(TxMsg::Frame(f))) => Err(f),
            Err(_) => unreachable!("only Frame variants used here"),
        }
    }
}

/// Spawn a writer task which OWNS the AsyncWrite half (single-writer discipline).
pub fn spawn_writer<W>(mut wr: W, bound: usize) -> (TxSender, JoinHandle<()>)
where
    W: AsyncWrite + Unpin + Send + 'static,
{
    let (tx, mut rx) = mpsc::channel::<TxMsg>(bound);
    let handle = tokio::spawn(async move {
        let mut scratch = BytesMut::with_capacity(8 * 1024);
        while let Some(msg) = rx.recv().await {
            match msg {
                TxMsg::Frame(frame) => {
                    if let Err(e) =
                        crate::conn::writer::write_frame(&mut wr, &frame, &mut scratch).await
                    {
                        warn!(error=?e, "writer: write failed — closing");
                        break;
                    }
                    overlay_metrics::set_peer_tx_depth(rx.max_capacity() - rx.capacity());
                }
                TxMsg::Close => {
                    debug!("writer: close requested");
                    break;
                }
            }
        }
    });
    (TxSender { tx }, handle)
}
