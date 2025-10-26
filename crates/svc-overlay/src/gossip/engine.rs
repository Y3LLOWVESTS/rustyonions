//! RO:WHAT — Minimal gossip engine: bounded ingress queue + background worker.
//! RO:WHY  — Provide a place to route/process `Data` frames beyond the echo demo.
//! RO:INVARIANTS — Non-blocking publish; backpressure via bounded channel; best-effort drop on full.

use bytes::Bytes;
use once_cell::sync::OnceCell;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{debug, warn};

/// Global publishing hook (optional). Listener can publish without holding an Engine instance.
static GLOBAL_TX: OnceCell<mpsc::Sender<Bytes>> = OnceCell::new();

#[derive(Clone)]
pub struct GossipEngine {
    tx: mpsc::Sender<Bytes>,
}

impl GossipEngine {
    /// Start the engine with bounded capacity and spawn the worker task.
    pub fn start(capacity: usize) -> (Self, JoinHandle<()>) {
        let (tx, mut rx) = mpsc::channel::<Bytes>(capacity);
        let me = Self { tx: tx.clone() };

        let task = tokio::spawn(async move {
            // Minimal worker: log and count. Later: route, dedupe, fanout.
            while let Some(msg) = rx.recv().await {
                debug!(len = msg.len(), "gossip: received message");
                metrics::counter!("gossip_ingress_total").increment(1);
                metrics::counter!("gossip_ingress_bytes_total").increment(msg.len() as u64);
                // TODO: plumb to per-topic queues or peers.
            }
            // Channel closed → shutdown path.
            warn!("gossip: worker exiting (channel closed)");
        });

        (me, task)
    }

    /// Install this engine as the global publisher target.
    pub fn install_global(&self) {
        let _ = GLOBAL_TX.set(self.tx.clone());
    }

    /// Try to publish a message (drops if queue is full).
    pub fn try_publish(&self, msg: Bytes) -> bool {
        match self.tx.try_send(msg) {
            Ok(()) => true,
            Err(mpsc::error::TrySendError::Full(_)) => {
                metrics::counter!("gossip_dropped_total", "reason" => "full").increment(1);
                false
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                metrics::counter!("gossip_dropped_total", "reason" => "closed").increment(1);
                false
            }
        }
    }
}

/// Publish through the global hook (if installed).
pub fn publish(msg: Bytes) -> bool {
    if let Some(tx) = GLOBAL_TX.get() {
        match tx.try_send(msg) {
            Ok(()) => {
                metrics::counter!("gossip_ingress_total").increment(1);
                true
            }
            Err(mpsc::error::TrySendError::Full(_)) => {
                metrics::counter!("gossip_dropped_total", "reason" => "full").increment(1);
                false
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                metrics::counter!("gossip_dropped_total", "reason" => "closed").increment(1);
                false
            }
        }
    } else {
        metrics::counter!("gossip_dropped_total", "reason" => "unset").increment(1);
        false
    }
}
