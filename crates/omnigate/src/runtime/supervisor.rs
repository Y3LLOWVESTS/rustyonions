//! RO:WHAT — Spawns and coordinates all workers.
//! RO:WHY  — Single place that can trigger graceful shutdown and fan-out commands.

use tokio::{sync::broadcast, task::JoinHandle};
use tracing::info;

use super::channels::{mk_supervisor_bus, MpscRx, SupervisorMsg};
use super::shutdown::{pair as shutdown_pair, ShutdownTrigger};
use super::worker::{spawn_worker, DynWorker};

pub struct SupervisorHandle {
    pub join: JoinHandle<()>,
    pub tx_cmd: broadcast::Sender<SupervisorMsg>,
    pub shutdown: ShutdownTrigger,
    pub up_rx: MpscRx<super::channels::WorkerMsg>,
}

pub fn spawn_supervisor(workers: Vec<DynWorker>, worker_backlog: usize) -> SupervisorHandle {
    let (tx_cmd, _rx_cmd, up_tx, up_rx) = mk_supervisor_bus(worker_backlog);
    let tx_cmd_for_task = tx_cmd.clone();

    let (shutdown, trigger) = shutdown_pair();

    let join = tokio::spawn(async move {
        // spawn all workers
        let mut joins: Vec<JoinHandle<()>> = Vec::with_capacity(workers.len());
        for w in workers {
            let rx = tx_cmd_for_task.subscribe();
            let j = spawn_worker(w, shutdown.clone(), rx, up_tx.clone());
            joins.push(j);
        }

        // Wait for shutdown, then ask everyone to stop.
        shutdown.cancelled().await;
        let _ = tx_cmd_for_task.send(SupervisorMsg::Stop);

        // Drain joins.
        for j in joins {
            let _ = j.await;
        }
        info!("supervisor exited");
    });

    SupervisorHandle {
        join,
        tx_cmd,
        shutdown: trigger,
        up_rx,
    }
}
