---
title: ron-kernel — Kernel & Orchestration (Pillar 1)
version: 0.1.0
status: draft
last-updated: 2025-09-18
audience: contributors, ops, auditors
---

# ron-kernel

## 1) Invariants (MUST)
- [I-K1 | Critical] Never hold a lock across `.await` in supervisory paths.
- [I-K2 | Critical] Child panics are contained; apply jittered backoff restart. **SLO:** median restart < 1s, p99 < 5s.
- [I-K3 | Critical] No silent loss of critical signals. Integrates with bus guarantees (bounded queues + NACK/DLQ).
- [I-K4 | Critical] No “free” tasks: all runtime work is supervised and cancellation-capable.
- [I-K5 | Advisory] Kernel exports health/ready endpoints and emits metrics for restarts/supervision outcomes.  
  _Rationale:_ schema can evolve; enforcement checks presence vs. strict shape.

## 2) Design Principles (SHOULD)
- [P-K1] Keep the kernel small and policy-free: supervise, don’t decide.
- [P-K2] Explicit capability-passing (no ambient authority) for network/storage/keys.
- [P-K3] Prefer actor isolation and bounded mailboxes; graceful shutdown via cancellation.

## 3) Implementation (HOW)
- [C-K1] Supervision surface:
```rust
pub trait KernelService {
    async fn start(&self, cancel: CancellationToken) -> anyhow::Result<()>;
    async fn shutdown(&self, grace: std::time::Duration) -> anyhow::Result<()>;
}
```
- [C-K2] `supervised_spawn` wrappers using `tokio::task::Builder` for named tasks + jittered backoff.
- [C-K3] Graceful shutdown: `select!` on `CancellationToken` + per-service `shutdown(grace)`.
- [C-K4] Metrics hooks (via ron-metrics): `child_restarts_total`, `restart_seconds` histogram; health/ready checks.

## 4) Acceptance Gates (PROOF)
- [G-K1 | Build-blocking] **AST/xtask**: forbid `.await` while holding `Mutex/RwLock`; forbid free `tokio::spawn`; require `#[non_exhaustive]`; no `pub` struct fields.
- [G-K2] **Chaos test**: crash a supervised child; verify SLO (median <1s, p99 <5s) and `restart_seconds` histogram has samples.
- [G-K3] **Dep-graph gate**: upload DAG artifact; fail on cycles.
- [G-K4] **CI**: `clippy -D warnings`, `cargo test --workspace`; optional **miri/loom** for hot concurrency paths.
- [G-K5] **Invariant tags**: at least one test per Critical invariant (`I-K*`).
- [G-K6] **Metrics presence**: assert `child_restarts_total` and `restart_seconds` are exported.
- [G-K7 | Optional] **Formal sketch**: keep a TLA+ stub for supervision liveness/backoff (non-blocking in “Full” CI profile).

## 5) Anti-Scope (Forbidden)
- No business logic, persistence, or network protocol handling (HTTP/gRPC/etc.). Kernel supervises; services implement domain behavior.

## 6) References
- Architecture_Blueprint_Refactor.md — Pillar 1 scope, SLOs, gates.
- IDB.md — Template and methodology.
