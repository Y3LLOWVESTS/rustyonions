//! RO:WHAT — Message channels between supervisor and workers.
//! RO:WHY  — Broadcast down (supervisor→workers), MPSC up (workers→supervisor).

use tokio::sync::{broadcast, mpsc};

#[derive(Debug, Clone)]
pub enum SupervisorMsg {
    /// Ask all workers to stop gracefully.
    Stop,
    /// Future: reload config, etc.
    Nop,
}

#[derive(Debug)]
pub enum WorkerMsg {
    Started(&'static str),
    Stopped(&'static str),
}

pub use broadcast::{Receiver as BcastRx, Sender as BcastTx};
pub use mpsc::{Receiver as MpscRx, Sender as MpscTx};

/// Build the control plane channels.
/// - `worker_backlog`: size of the per-worker upstream MPSC buffer.
pub fn mk_supervisor_bus(
    worker_backlog: usize,
) -> (
    BcastTx<SupervisorMsg>,
    BcastRx<SupervisorMsg>,
    MpscTx<WorkerMsg>,
    MpscRx<WorkerMsg>,
) {
    let (tx_cmd, rx_cmd) = broadcast::channel(16);
    let (up_tx, up_rx) = mpsc::channel(worker_backlog);
    (tx_cmd, rx_cmd, up_tx, up_rx)
}
