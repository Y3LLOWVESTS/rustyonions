//! RO:WHAT — Example worker used by tests and as a template.
//! RO:WHY  — Minimal worker that exits on shutdown or Stop.

use std::{future::Future, pin::Pin, sync::Arc};
use tokio::sync::{broadcast, mpsc};
use tracing::info;

use super::channels::{SupervisorMsg, WorkerMsg};
use super::shutdown::Shutdown;
use super::worker::Worker;

#[derive(Default)]
pub struct TickWorker;

impl TickWorker {
    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}

impl Worker for TickWorker {
    fn name(&self) -> &'static str {
        "tick"
    }

    fn run(
        &self,
        shutdown: Shutdown,
        mut commands: broadcast::Receiver<SupervisorMsg>,
        up_tx: mpsc::Sender<WorkerMsg>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        Box::pin(async move {
            loop {
                tokio::select! {
                    _ = shutdown.cancelled() => {
                        break;
                    }
                    Ok(msg) = commands.recv() => {
                        if let SupervisorMsg::Stop = msg {
                            break;
                        }
                    }
                }
            }
            let _ = up_tx.send(WorkerMsg::Stopped("tick")).await;
            info!("tick worker exiting");
        })
    }
}
