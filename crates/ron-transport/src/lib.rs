//! RO:WHAT — Public entry for ron-transport: config/types and spawn helpers.
//! RO:WHY  — Pillar 10 transport; Concerns: SEC/RES/PERF.
//! RO:INTERACTS — tcp::{listener,dialer}, tls::{server,client}, limits, metrics; kernel Bus/Health.
//! RO:INVARIANTS — single writer per conn; no locks across .await; OAP max_frame=1MiB; chunk≈64KiB.

#![forbid(unsafe_code)]

#[cfg(feature = "arti")]
pub mod arti;
pub mod config;
pub mod conn;
pub mod error;
pub mod limits;
pub mod metrics;
#[cfg(feature = "quic")]
pub mod quic;
pub mod readiness;
pub mod reason;
pub mod tcp;
#[cfg(feature = "tls")]
pub mod tls;
pub mod types;
pub mod util;

// Always-present TLS type alias wrapper (feature-safe).
mod tls_types;
pub use tls_types::TlsServerConfig;

use crate::config::TransportConfig;
use crate::metrics::TransportMetrics;
use crate::readiness::ReadyGate;
use crate::types::TransportEvent;
use crate::util::cancel::Cancel;
use ron_kernel::{Bus, HealthState};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::task::JoinHandle;

/// Public handle for a running transport listener.
pub struct TransportHandle {
    /// The accept-loop task.
    pub task: JoinHandle<()>,
    /// The bound socket address.
    pub addr: SocketAddr,
    /// Cancellation token — request graceful shutdown.
    pub cancel: Cancel,
}

/// Spawn a TCP (optionally TLS) listener and per-connection tasks, returning a shutdown handle.
///
/// Backward-compatible, high-level entry.
pub async fn spawn_transport_with_cancel(
    cfg: TransportConfig,
    metrics: TransportMetrics,
    health: Arc<HealthState>,
    _bus: Bus<TransportEvent>, // reserved; will publish Connected/Disconnected when Bus API is confirmed
    tls: Option<Arc<TlsServerConfig>>,
) -> anyhow::Result<TransportHandle> {
    let gate = ReadyGate::new();
    let cancel = Cancel::new();
    let (task, addr) = tcp::listener::spawn_listener_with_cancel(
        cfg,
        metrics,
        health,
        gate.clone(),
        tls,
        cancel.clone(),
    )
    .await?;
    gate.set_listeners_bound(true);
    Ok(TransportHandle { task, addr, cancel })
}

/// Legacy wrapper that preserves the original return type.
/// Use `spawn_transport_with_cancel` if you want a shutdown handle.
pub async fn spawn_transport(
    cfg: TransportConfig,
    metrics: TransportMetrics,
    health: Arc<HealthState>,
    bus: Bus<TransportEvent>,
    tls: Option<Arc<TlsServerConfig>>,
) -> anyhow::Result<(JoinHandle<()>, SocketAddr)> {
    let handle = spawn_transport_with_cancel(cfg, metrics, health, bus, tls).await?;
    Ok((handle.task, handle.addr))
}
