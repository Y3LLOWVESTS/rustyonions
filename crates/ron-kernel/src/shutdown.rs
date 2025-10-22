//! RO:WHAT — Helper to await Ctrl+C for cooperative shutdown.
//! RO:WHY  — Common pattern for binaries to align with kernel readiness and graceful stop.
//! RO:INTERACTS — May be used to trigger KernelEvent::Shutdown by callers (kernel doesn't emit it automatically).
//! RO:INVARIANTS — async-signal safe; no blocking in Drop.

/// Wait for a Ctrl+C signal.
pub async fn wait_for_ctrl_c() {
    let _ = tokio::signal::ctrl_c().await;
}
