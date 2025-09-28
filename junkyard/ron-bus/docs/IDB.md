
---

**Path:** `crates/ron-bus/docs/IDB.md`

````markdown
---
title: ron-bus — In-Process Broadcast Bus (IDB)
version: 1.0.0
status: draft
last-updated: 2025-09-27
msrv: 1.80.0
audience: contributors, ops, auditors
pillars: [P1 Kernel & Orchestration]
concerns: [RES, PERF]
canon: 33-crate fixed set
---

# ron-bus — In-Process Broadcast Bus (IDB)

## 1. Invariants (MUST)

- [I-1] **Bounded broadcast.** All channels are **bounded**; overflow is **lossy** and **observable** via metrics. Bus operations must not block kernel-critical paths.
- [I-1b] **Monomorphic surface.** `Bus` and its public API are **non-generic** and stable; no generic parameters in public types or trait bounds.
- [I-2] **One receiver per task.** Each consumer task uses its **own** `broadcast::Receiver`. Sharing a receiver across tasks is forbidden.
- [I-3] **No locks across `.await`.** Library and examples MUST NOT hold any lock across `.await`, especially on supervisory/hot paths.
- [I-4] **Overflow accounting.** On drop/overflow, increment `bus_overflow_dropped_total` and expose lag/queue depth signals; never panic in steady state.
- [I-5] **Kernel contract compatibility.** Bus semantics must support the frozen kernel API that re-exports `Bus` and emits `KernelEvent::*` without API drift.
- [I-6] **Crash-only friendliness.** Subscriber failures **cannot wedge publishers**; backpressure remains bounded and observable; the bus never performs blocking I/O.
- [I-7] **Amnesia-neutral.** The bus has **no persistence**. Behavior is identical with amnesia on/off; only metrics may include an `amnesia` label. (Aligns with Micronode RAM-only default.)
- [I-8] **Unsafe forbidden.** `#![forbid(unsafe_code)]` at crate root; CI denies `await_holding_lock`, `unwrap_used`, `expect_used`.

## 2. Design Principles (SHOULD)

- [P-1] **Monomorphic & minimal.** Keep `Bus` simple (fan-out, subscribe, capacity helpers) and avoid runtime coupling. Integrate with `ron-metrics` but don’t redefine metric taxonomies.
- [P-2] **Owned data at edges.** Prefer `Bytes` or `Arc<T>` for payloads; never borrow stack buffers into async loops.  
  - [P-2a] Prefer `Bytes`/`Arc<T>` over `Vec<u8>` to enable cheap clones and reduce reallocations under fan-out (zero-copy where possible).
- [P-3] **Deterministic teardown.** Cooperative cancellation and drop-safe constructs ensure subscribers exit cleanly on kernel shutdown.
- [P-4] **Zero global state.** No ad-hoc singletons; the bus handle is passed or cloned explicitly.
- [P-5] **Observable from day one.** Provide overflow/lag/queue-depth metrics so `/readyz` can degrade before collapse.

## 3. Implementation (HOW)

### [C-1] Core surface (sketch)

```rust
// lib.rs (sketch)
#![forbid(unsafe_code)]
use tokio::sync::broadcast;

#[derive(Clone, Debug)]
pub enum Event {
    Health { service: String, ok: bool },
    ConfigUpdated { version: u64 },
    ServiceCrashed { service: String, reason: String },
    Shutdown,
}

pub struct Bus {
    tx: broadcast::Sender<Event>,
    cap: usize,
}

impl Bus {
    pub fn new(capacity: usize) -> Self {
        let (tx, _rx) = broadcast::channel(capacity);
        Self { tx, cap: capacity }
    }
    pub fn sender(&self) -> broadcast::Sender<Event> { self.tx.clone() }
    pub fn subscribe(&self) -> broadcast::Receiver<Event> { self.tx.subscribe() }
    pub fn capacity(&self) -> usize { self.cap }
}
````

### [C-2] One-receiver-per-task idiom

```rust
let mut rx = bus.subscribe(); // unique per task
tokio::spawn(async move {
    while let Ok(ev) = rx.recv().await {
        handle(ev).await;
    }
});
```

> Never share one `Receiver` across multiple tasks.

### [C-3] Overflow & lag metrics

```rust
// Example hooks; actual registration lives in ron-metrics
use prometheus::{IntCounter, IntGauge};
static BUS_OVERFLOW_DROPPED_TOTAL: OnceLock<IntCounter> = OnceLock::new();
static BUS_QUEUE_DEPTH: OnceLock<IntGauge> = OnceLock::new();

fn record_lagged(n: u64) {
    BUS_OVERFLOW_DROPPED_TOTAL.get().unwrap().inc_by(n);
}

fn set_queue_depth(depth: i64) {
    BUS_QUEUE_DEPTH.get().unwrap().set(depth);
}
```

### [C-4] No-lock-across-await guard

```rust
// BAD
let mut g = state.lock().await;
g.push(ev);
some_io().await;

// GOOD
{ state.lock().await.push(ev); }
some_io().await;
```

### [C-5] Cancel-safe shutdown

```rust
use tokio_util::sync::CancellationToken;

let cancel = CancellationToken::new();
let child = cancel.child_token();

let task = tokio::spawn(async move {
    tokio::select! {
        _ = child.cancelled() => { /* drain & exit */ }
        _ = subscriber_loop() => {}
    }
});

// elsewhere: on kernel shutdown
cancel.cancel();
```

> Optional: a `tracing` cargo feature may add spans around publish/recv; default **off** to keep the core minimal.

### [C-6] (Optional) Visual — overflow path

```mermaid
flowchart LR
  P[Publisher send()] -->|ok| RX1[Receiver A]
  P -->|ok| RX2[Receiver B]
  P -->|lagged n| O[Overflow Counter ++, QueueDepth update]
```

## 4. Acceptance Gates (PROOF)

* [G-1] **Unit & property tests (semantics).**

  * Fan-out correctness (N subscribers receive M events; loss only under intentional overflow).
  * **No deadlocks**; **no lock across `.await`** (Clippy denies).
* [G-2] **Overflow visibility.** Force saturation at capacity=8; assert `bus_overflow_dropped_total > 0` while publishers remain non-blocking.
* [G-3] **One-receiver-per-task.** Integration test that shares a receiver across tasks must fail (compile-time or runtime assertion). Canonical pattern test must pass.
* [G-4] **Kernel contract smoke.** Publish all `Event` variants (incl. `ServiceCrashed`) and verify end-to-end observation by ≥1 subscriber.
* [G-5] **Loom interleavings (lite).** Minimal loom test exploring send/recv/drop interleavings without violating [I-1]…[I-3].
* [G-6] **CI teeth.**

  * Clippy wall: deny `await_holding_lock`, `unwrap_used`, `expect_used`.
  * Thread/Address sanitizers green on CI.
  * Tokio runtime matrix (multi-thread & current-thread) passes.
* [G-7] **Metrics registration discipline.** Metrics are registered **once** (e.g., `OnceLock`); duplicate registration must fail tests or be prevented at runtime.
* [G-8] **Amnesia neutrality.** Running with `amnesia=on|off` does not change bus behavior; only a metrics label may differ; asserted in tests.
* [G-9] **Public API lock.** `cargo public-api -p ron-bus` shows no generics on public `Bus` types; diff must be empty unless an intentional SemVer bump.
* [G-10] **SemVer discipline.** `cargo-semver-checks` passes for non-breaking changes; breaking changes require an intentional major version bump.

## 5. Anti-Scope (Forbidden)

* ❌ **Unbounded channels** or implicit infinite buffers.
* ❌ **Sharing a `broadcast::Receiver`** between tasks or cloning receivers across tasks.
* ❌ **Holding locks across `.await`** in any example or helper.
* ❌ **Custom metric taxonomies** or logging formats conflicting with `ron-metrics` golden set.
* ❌ **Persistence, transport, or network I/O** inside `ron-bus` — remain a pure in-process IPC library.
* ❌ **Generics in the public `Bus` type** or its primary public methods.

## 6. References

* Full Project & Microkernel Blueprints (kernel API, bounded bus, golden metrics)
* Concurrency & Aliasing rules (no lock across await; one-receiver-per-task)
* Six Concerns (Resilience & Performance mapping)
* Hardening gates (CI lints, sanitizers, SemVer discipline)

```
---
