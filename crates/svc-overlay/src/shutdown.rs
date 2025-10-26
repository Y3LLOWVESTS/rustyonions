//! RO:WHAT — Shutdown coordination
pub async fn wait_for_shutdown() {
    let _ = tokio::signal::ctrl_c().await;
}
