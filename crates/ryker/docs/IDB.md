---
title: ryker — Actor/Runtime Substrate (Pillar 1)
version: 0.1.0
status: draft
last-updated: 2025-09-18
audience: contributors, ops, auditors
---

# ryker

## 1) Invariants (MUST)
- [I-R1 | Critical] **Bounded mailboxes** only; no unbounded channels in the runtime core.
- [I-R2 | Critical] Tasks are **cancellable**; graceful shutdown must progress via cancellation tokens.
- [I-R3 | Advisory] No `.await` while holding locks in actor hot paths.  
  _Rationale:_ rare, audited exceptions may exist; enforced by lints/xtask by default.

## 2) Design Principles (SHOULD)
- [P-R1] Execution substrate, not orchestrator: **mechanics only**. Policies live in the kernel.
- [P-R2] Prefer message-passing over shared mutability; isolate actor state.
- [P-R3] Named tasks and structured concurrency to improve traceability under kernel supervision.

## 3) Implementation (HOW)
- [C-R1] Helpers around `tokio::task::Builder` and `CancellationToken`; idiomatic `select!` shutdown pattern.
- [C-R2] Mailbox constructors require capacity; **no “unbounded” constructor** (or forbid behind a “deny-unbounded” feature that fails build).
- [C-R3] Backoff adapters (e.g., Tower layer) as building blocks, not policy.

### Ryker snippet (parity with other crates)
```rust
use std::time::Duration;
use tokio::{task::Builder, select};
use tokio_util::sync::CancellationToken;

/// Spawns a named, cancellation-aware task (kernel supervises the JoinHandle).
pub fn spawn_supervised<F>(
    name: &'static str,
    cancel: CancellationToken,
    fut: F,
) -> tokio::task::JoinHandle<()>
where
    F: std::future::Future<Output = ()> + Send + 'static,
{
    Builder::new().name(name).spawn(async move {
        select! {
            _ = cancel.cancelled() => { /* graceful stop */ }
            _ = fut => { /* task finished */ }
        }
    })
}

/// Bounded mailbox example (constructor fails if capacity == 0).
pub fn bounded_mailbox<T: Send + 'static>(capacity: usize) -> anyhow::Result<(async_channel::Sender<T>, async_channel::Receiver<T>)> {
    anyhow::ensure!(capacity > 0, "capacity must be > 0");
    Ok(async_channel::bounded(capacity))
}
```

## 4) Acceptance Gates (PROOF)
- [G-R1 | Build-blocking] **AST/xtask**: ban unbounded channels; forbid `.await` across locks; forbid free `tokio::spawn`.
- [G-R2] **Loom** models for mailbox capacity and cancellation progress (no deadlocks/livelocks).
- [G-R3] **CI**: clippy/test green; **Dep-graph gate** clean.
- [G-R4] **Invariant tags**: at least one test per Critical invariant (`I-R*`).

## 5) Anti-Scope (Forbidden)
- No network/persistence/business code; no HTTP/gRPC handlers.  
  _For HTTP or persistence, see:_ `svc-gateway` / `svc-storage` (etc.).

## 6) References
- Architecture_Blueprint_Refactor.md — Pillar-1 scope, invariants, patterns, gates.
- IDB.md — Template and methodology.
