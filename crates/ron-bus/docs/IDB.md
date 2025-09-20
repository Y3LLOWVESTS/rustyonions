---
title: ron-bus — In-Process Event Bus (Pillar 1)
version: 0.1.0
status: draft
last-updated: 2025-09-18
audience: contributors, ops, auditors
---

# ron-bus

## 1) Invariants (MUST)
- [I-B1 | Critical] All topics are **bounded**; backpressure is enforced. No unbounded fan-out.
- [I-B2 | Critical] **No silent loss** of critical events. When bounds are exceeded, surface NACK/DLQ or explicit error.
- [I-B3 | Advisory] Subscriber isolation: one slow consumer must not starve others.  
  _Rationale:_ isolation strategy can vary by impl; property/loom proofs suffice.

## 2) Design Principles (SHOULD)
- [P-B1] Strictly **in-process** signal fabric. Not a durable queue or a network broker.
- [P-B2] Prefer copy-on-write/Arc-slice for payloads to minimize contention.
- [P-B3] Strongly typed events (`trait Event`) to avoid “stringly typed” channels.

## 3) Implementation (HOW)
- [C-B1] Bus surface:
```rust
pub trait Event: Send + Sync + 'static {}

pub trait Bus<E: Event> {
    fn publish(&self, evt: E) -> Result<(), BusError>;
    fn subscribe(&self) -> Subscription<E>; // bounded mailbox per subscriber
}
```
- [C-B2] Non-blocking `publish` succeeds only within capacity; otherwise return NACK/error and record metrics.
- [C-B3] Property-test harness simulates fan-out with bounded queues; validates “no silent drops” and isolation.
- [C-B4] Metrics (via ron-metrics): `bus_dropped_total`, **`bus_lagged_total` (REQUIRED)**, and `bus_nack_total` counters; `bus_queue_depth` gauge.

## 4) Acceptance Gates (PROOF)
- [G-B1 | Build-blocking] Property tests cover bounded fan-out, NACK/DLQ paths, and subscriber isolation.
- [G-B2] **Loom** model(s) for hot paths (publish/subscribe contention); enforce “no lock-across-await” in critical sections.
- [G-B3] **CI**: clippy/test green; **Dep-graph gate** clean.
- [G-B4] **Metrics presence**: assert `bus_dropped_total`, **`bus_lagged_total`**, `bus_nack_total`, and `bus_queue_depth` are exported.
- [G-B5] **Invariant tags**: at least one test per Critical invariant (`I-B*`).

## 5) Anti-Scope (Forbidden)
- No durability semantics, no external transport, no business routing policies.  
  _For HTTP or network distribution, see:_ `svc-gateway` / `omnigate`.

## 6) References
- Architecture_Blueprint_Refactor.md — Bus interface, invariants, gates.
- IDB.md — Template and methodology.
