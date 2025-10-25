//! RO:WHAT — Optional tracing helpers (no-op without `tracing` feature).
//! RO:WHY  — Keep span names stable per docs; host chooses exporter.
//! RO:INVARIANTS — never log message bodies/PII.

#[inline]
pub fn span_enqueue(actor: &str, depth: usize) {
    #[cfg(feature = "tracing")]
    tracing::trace!(target="ryker", actor=%actor, queue_depth=%depth, "ryker.mailbox.enqueue");
}

#[inline]
pub fn span_handle(actor: &str, outcome: &str, deadline_ms: u64) {
    #[cfg(feature = "tracing")]
    tracing::trace!(target="ryker", actor=%actor, outcome=%outcome, deadline_ms=%deadline_ms, "ryker.actor.handle");
}

#[inline]
pub fn span_config_reload() {
    #[cfg(feature = "tracing")]
    tracing::info!(target = "ryker", "ryker.config.reload");
}
