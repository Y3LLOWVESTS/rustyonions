//! RO:WHAT — Worker trait + spawner glue.
//! RO:WHY  — Lets us run heterogeneous background tasks under a supervisor.

use std::{future::Future, pin::Pin, sync::Arc};
use tokio::{
    sync::{broadcast, mpsc},
    task::JoinHandle,
};
use tracing::info;

use super::channels::{SupervisorMsg, WorkerMsg};
use super::shutdown::Shutdown;

/// Vtable for a managed worker.
pub trait Worker: Send + Sync + 'static {
    fn name(&self) -> &'static str;

    fn run(
        &self,
        shutdown: Shutdown,
        commands: broadcast::Receiver<SupervisorMsg>,
        up_tx: mpsc::Sender<WorkerMsg>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send>>;
}

pub type DynWorker = Arc<dyn Worker>;

pub fn spawn_worker(
    w: DynWorker,
    shutdown: Shutdown,
    commands: broadcast::Receiver<SupervisorMsg>,
    up_tx: mpsc::Sender<WorkerMsg>,
) -> JoinHandle<()> {
    let name = w.name();
    tokio::spawn(async move {
        // Let the supervisor know we’re starting.
        let _ = up_tx.send(WorkerMsg::Started(name)).await;

        // Run the worker future to completion.
        let fut = (*w).run(shutdown, commands, up_tx.clone());
        fut.await;

        info!(worker = name, "worker exited");
        // Best-effort notify stop.
        let _ = up_tx.send(WorkerMsg::Stopped(name)).await;
    })
}
