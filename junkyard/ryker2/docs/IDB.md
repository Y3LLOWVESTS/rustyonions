

```markdown
---
title: Ryker — Actor & Mailbox Runtime (IDB)
version: 1.1.0
status: draft
last-updated: 2025-09-27
audience: contributors, ops, auditors
---

# Ryker — Actor & Mailbox Runtime (IDB)

## 1. Invariants (MUST)

- [I-1] **Bounded queues only.** All mailboxes/channels are finite and enforce backpressure (no unbounded queues anywhere).
- [I-2] **Single consumer per mailbox.** Exactly one receiver task owns a mailbox; any fan-out/broadcast happens on the kernel Bus, not here.
- [I-3] **No locks across `.await`.** Never hold a lock across `.await` in supervisors, actors, or mailbox I/O.
- [I-4] **Crash-only + cancel-safe.** Tasks must be cancellation-safe; failures trigger crash-only restarts with jittered backoff; teardown must not block.
- [I-5] **Loss visibility.** On overflow/drop, increment counters and emit structured logs.
- [I-6] **Zero-copy payloads where possible.** Prefer `bytes::Bytes` or equivalent for owned byte payloads.
- [I-7] **Scope-bounded.** No app/business logic, no network/TLS, no persistence; orchestration primitives only.
- [I-8] **Amnesia honored.** When amnesia mode is on, do not persist hidden state; zeroize transients on drop.
- [I-9] **Runtime-agnostic API.** Public API must not leak specific Tokio flavor/handles.
- [I-10] **No free-spawn.** Actor tasks must be spawned via `Supervisor::spawn` only (traceable, restartable). Direct `tokio::spawn` is forbidden for actor lifecycles.
- [I-11] **Deadline propagation.** Request/response operations carry a deadline (or timeout). Actors must respect and not exceed it.
- [I-12] **Memory budgets.** Each mailbox enforces `capacity` and `max_msg_bytes`; enforce a per-actor budget = `capacity × max_msg_bytes`, dropping with reasoned telemetry rather than risking OOM.
- [I-13] **Cooperative execution.** No blocking syscalls or busy loops in actor loops; yield cooperatively under sustained work.
- [I-14] **Deterministic close semantics.** Mailbox close is observable; receivers terminate promptly after draining without busy-spin.

## 2. Design Principles (SHOULD)

- [P-1] **Small surface, strong patterns.** Provide a minimal set of primitives: `Supervisor`, `Mailbox<T>`, typed req/resp helpers.
- [P-2] **Explicit capacity & deadlines.** Callers specify mailbox capacity and timeouts; no hidden defaults in hot paths.
- [P-3] **Observability by default.** Queue depth snapshots, overflow counters, restart counters, last-error cause.
- [P-4] **Typed messages.** Prefer enum messages per actor; avoid ad-hoc `Any`/string routing.
- [P-5] **Escalate with context.** Panics/fatal errors bubble to supervisor with structured reason.
- [P-6] **Fairness over raw throughput.** Use small batches and cooperative yields to reduce starvation under load.
- [P-7] **Crypto-agnostic boundary.** Ryker never handles keys/crypto; PQ/ZK lives in `ron-kms` / `ron-pq-*`.
- [P-8] **Error taxonomy.** Use small, structured enums (e.g., `ActorExit::{Normal, Cancelled, Failed{cause}}`, `MailboxError::{Full, TooLarge, Closed, Deadline}`) for clarity and metrics labels.
- [P-9] **Span hygiene.** Each actor and message is traced with a named span and correlation id when present.

## 3. Implementation (HOW)

- [C-1] **Mailbox (bounded).** Wrap `tokio::sync::mpsc`:
  - `capacity: usize` set by caller,
  - `max_msg_bytes: usize` optional bound,
  - `try_send` (fast path) + `send_with_deadline`,
  - counters for `overflow_total` (reasons: `capacity|too_large|closed|deadline`),
  - `queue_depth()` snapshot for scrapes.
- [C-2] **Actor loop.** Single consumer pulls from `Mailbox<T>`; `handle(msg)` is `async` but never holds locks across `.await`. Prefer compact state + `Bytes`.
- [C-3] **Request/Response.** Pattern = `mpsc` to actor + `oneshot` for reply; enforce per-call deadline and propagate cancel.
- [C-4] **Supervisor::spawn().** Wraps `tokio::spawn` to:
  - tag tasks with `name`,
  - convert panics to failures,
  - restart with capped exponential backoff + jitter,
  - count `actor_restarts_total{actor}`,
  - stop after `N` rapid failures with structured error.
- [C-5] **Cancellation.** Cooperative cancellation via token or `select!`; shutdown drains minimally and exits promptly.
- [C-6] **No broadcast here.** For fanout, use the kernel Bus (lossy, bounded); Ryker mailboxes remain single-consumer.
- [C-7] **Metrics families (harmonized).**
  - `ryker_mailbox_overflow_total{actor,reason}`
  - `ryker_actor_restarts_total{actor}`
  - `ryker_mailbox_queue_depth{actor}` (sampled)
  - `ryker_actor_poll_duration_seconds{actor}` (optional)
- [C-8] **Amnesia hook.** Constructors accept `amnesia: bool`; avoid caching across restarts and zeroize on drop when true.
- [C-9] **Backoff algorithm.** Use decorrelated jitter (e.g., `base=100ms`, `cap=5s`), record next-delay in logs/metrics.
- [C-10] **Deadline wiring.** `Request<T>` carries `deadline: Instant`; helpers `send_with_deadline`/`recv_with_deadline` enforce and annotate drops with `reason="deadline"`.
- [C-11] **Instrumentation hook.** Lightweight `MailboxObserver` trait invoked on `overflow/close/drain`; default forwards to metrics to avoid tight coupling.
- [C-12] **Tracing spans.** Actor task runs within `tracing::span!(INFO, "actor", name, instance_id)`; per-message spans annotate `kind`, `cid` (correlation id), and timing.

## 4. Acceptance Gates (PROOF)

**Concurrency/Correctness**
- [G-1] **No-locks-across-await lint.** CI denies holding locks across `.await` (regex lint + curated false-positive exceptions).
- [G-2] **Flavor matrix.** Tests pass on both Tokio current-thread and multi-thread runtimes.
- [G-3] **Backpressure test.** Fill mailbox to capacity; assert `try_send` fails and `ryker_mailbox_overflow_total{reason="capacity"}` increments.
- [G-4] **Crash-only restart.** Actor that panics N times restarts with increasing delays; counter increments; exceeds rapid-fail threshold → supervisor stops and surfaces structured failure.
- [G-5] **Cancel-safety.** Inject cancellation mid-I/O; assert prompt exit, no deadlocks, and no locks across `.await`.
- [G-6] **Amnesia compliance.** With amnesia=true, no state crosses restarts; zeroization hook runs (heap inspector/tests).
- [G-7] **Observability sanity.** Metrics exist with correct names/labels; queue depth snapshots are sane under load.

**Discipline/Drift**
- [G-8] **Spawn discipline.** CI forbids `tokio::spawn(` usage for actor lifecycles outside `ryker::Supervisor::spawn` (regex + allowlist for non-actor tasks where justified).
- [G-9] **Loom smoke.** A minimal loom test exercises send/receive/cancel; no ordering violations or leaks detected.
- [G-10] **API stability.** `cargo-public-api` gate protects semver on `Supervisor`, `Mailbox`, and error enums.
- [G-11] **Backoff property.** Property test verifies backoff within `[base, cap]`, decorrelated jitter, monotone non-decreasing under steady failures.
- [G-12] **Memory budget guard.** Oversized messages rejected with `MailboxError::TooLarge`; counters increment; stress test shows no runaway heap.
- [G-13] **Doc-test runnable.** Actor loop snippet from README/IDB compiles and runs as a doc-test.

## 5. Anti-Scope (Forbidden)

- Unbounded queues or hidden buffers (including “accidental” buffering in adapters).
- Multi-receiver/broadcast mailboxes (use Bus instead).
- App/business logic, RPC codecs, protocol parsing.
- Network/TLS types, storage backends, or persistence.
- Public types that force a specific Tokio runtime flavor or executor handle.
- Work-stealing pools or runtime tuning knobs embedded inside Ryker.

## 6. References

- **Pillar 1 — Kernel & Orchestration** (role, boundaries, invariants).
- **Concurrency & Aliasing Blueprint** (bounded channels, no locks across `.await`, cancellation).
- **Hardening Blueprint** (amnesia mode, zeroization, crash-only restart posture).
- **Microkernel Blueprint** (acceptance checklist; golden metrics alignment).
- **Complete Crate List / Crate Map** (Ryker’s placement and coupling limits).
```

