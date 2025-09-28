
**Path:** `crates/ron-bus/docs/PERFORMANCE.md`

```markdown
# ⚡ PERFORMANCE.md — ron-bus

---
title: Performance & Scaling — ron-bus
status: draft
msrv: 1.80.0
crate_type: lib
last-updated: 2025-09-27
audience: contributors, ops, perf testers
---

# PERFORMANCE.md

## 0. Purpose

This document defines the **performance profile** of `ron-bus`:

- Lib-level throughput metrics and scaling expectations.
- Benchmarks & workloads it must sustain.
- Perf harness & profiling tools.
- Scaling knobs, bottlenecks, and triage steps.
- Regression gates to prevent silent perf drift.

It ties directly into:
- **Scaling Blueprint v1.3.1** (roles, SLOs, runbooks).
- **Omnigate Build Plan** milestones Bronze→Gold.
- **Perfection Gates** (F = perf regressions barred, L = scaling chaos-tested).

---

## 1. SLOs / Targets (Library-Specific)

- **Throughput:** ≥ **1M events/sec** fan-out on commodity 8-core (baseline).  
- **Latency:** publish→recv **<50µs** p95 under load (single producer, 4 consumers).  
- **Allocations:** ≤1 alloc/op (bounded; ideally zero-copy `Bytes`).  
- **Subscriber scaling:** linear up to 32 subscribers before perf taper.  
- **Overflow semantics:** `bus_overflow_dropped_total` <0.01% of events at target load.  
- **Cold start:** Bus::new(capacity=256) <1µs.

---

## 2. Benchmarks & Harness

- **Micro-benchmarks:** Criterion (`cargo bench`) for:
  - `Bus::send` throughput.
  - Subscriber `recv` latency.
  - Lagged/overflow handling cost.

- **Integration tests:**  
  - Multi-producer, multi-subscriber soak for 24h.  
  - Backpressure scenarios (slow consumer).  

- **Profiling tools:**  
  - `cargo flamegraph` → CPU hotspots.  
  - `tokio-console` → verify no async stalls (in host subscriber tasks).  
  - `heaptrack` / `valgrind massif` → allocation profile.  
  - `perf stat` → IPC/branch misses under load.

- **Chaos/perf blend:**  
  - Induce lagged subscribers to verify bounded overflow.  
  - Subscriber churn (join/leave at 100Hz).  

- **CI Integration:** nightly Criterion vs baselines (`target/criterion/` compared).

---

## 3. Scaling Knobs

- **Channel capacity:** default 256, configurable; higher = more buffering but ↑ latency.  
- **Subscribers:** scale fan-out; each adds overhead proportional to channel depth.  
- **Message size:** use `Bytes`/`Arc<T>` to avoid copies; keep messages ≤64 KiB (OAP chunk guidance).  
- **Overflow policy:** drop oldest; tune `overflow_warn_rate` in host.  
- **Runtime tuning (host):** Tokio worker threads; task pinning for recv loops.

---

## 4. Bottlenecks & Known Limits

- **tokio::broadcast internals:** O(N) clone per subscriber; scaling limit ~64 subscribers.  
- **Lagged receivers:** slow consumers trigger drop accounting; metric update cost is non-zero.  
- **Alloc pressure:** large payloads (`Vec<u8>`) copy on send; must prefer `Bytes`.  
- **CPU cache:** hot loop is single branch + clone; cache thrash at >1M msgs/sec.

---

## 5. Regression Gates

- CI must fail if:  
  - Throughput ↓ >10% vs baseline.  
  - Publish→recv latency ↑ >20% p95.  
  - Allocations/op >1.  
  - Overflow % >0.1% in standard soak.

- Baselines stored in `testing/performance/baselines/ron-bus/`.  
- Escape hatch: allow waiver if regression traced to upstream Tokio changes.

---

## 6. Perf Runbook (Triage)

When perf SLOs are breached:

1. **Check flamegraph:** look for copies in `send` path.  
2. **Verify subscriber code:** ensure receivers use `Bytes`/`Arc<T>` not cloning payloads.  
3. **Inspect `bus_overflow_dropped_total`:** sustained increase → slow consumer.  
4. **Adjust channel capacity:** raise temporarily to absorb burst, but monitor latency.  
5. **Reproduce with Criterion:** isolate regression to crate vs. host usage.  
6. **Chaos test:** churn subscribers to detect leaks or lag miscounts.  

---

## 7. Acceptance Checklist (DoD)

- [ ] SLOs defined (throughput, latency, allocs, overflow).  
- [ ] Criterion harness implemented in `/benches/`.  
- [ ] Baseline results captured in `/testing/performance/baselines/`.  
- [ ] Perf regression CI wired.  
- [ ] Runbook validated with at least one chaos scenario.  

---

## 8. Appendix

- **Reference SLOs (Scaling Blueprint):**  
  - Broadcast bus: p95 <50µs, ≥1M events/sec, overflow <0.01%.  
  - Subscriber churn tolerance: join/leave at 100Hz without degradation.  

- **Reference workloads:**  
  - `cargo bench --bench bus_perf` with 4 producers × 8 subscribers.  
  - 24h soak with 10^9 events; check counters at end.  

- **Perfection Gates tie-in:**  
  - Gate F = no un-explained perf regressions.  
  - Gate L = chaos scaling validated (lagged subscriber churn).  

- **History:**  
  - [2025-09-27] Initial targets: 1M events/sec, 50µs p95, <0.01% overflow.  

---
```

---
