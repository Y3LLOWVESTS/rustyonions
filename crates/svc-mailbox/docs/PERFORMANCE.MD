
---

# ⚡ PERFORMANCE.md — Template 


---
title: Performance & Scaling Template
status: draft
msrv: 1.80.0
crate_type: service|lib
last-updated: YYYY-MM-DD
audience: contributors, ops, perf testers
---

# PERFORMANCE.md

## 0. Purpose

This document defines the **performance profile** of the crate:
- Service-level objectives (SLOs) or lib-level throughput metrics.
- Benchmarks & workloads it must sustain.
- Perf harness & profiling tools.
- Scaling knobs, bottlenecks, and triage steps.
- Regression gates to prevent silent perf drift.

It ties directly into:
- **Scaling Blueprint v1.3.1** (roles, SLOs, runbooks).
- **Omnigate Build Plan** milestones Bronze→Gold.
- **Perfection Gates** (F = perf regressions barred, L = scaling chaos-tested).

---

## 1. SLOs / Targets

### For Services
- **Latency:**  
  - p95 GET intra-region < X ms  
  - p95 GET inter-region < Y ms  
  - PUT p95 < Z ms  

- **Throughput:**  
  - ≥ N req/s (per node).  
  - Graceful backpressure when >N.  

- **Error Budget:**  
  - Failures <0.1%  
  - Quota 429/503 <1%  
  - Bus overflow <0.01%  

- **Resource ceilings:**  
  - CPU < A% per core at target load.  
  - Memory < B GiB steady state.  
  - FD usage < C% of system limit.  

- **Edge/Mobile (if applicable):**  
  - Cold start < D ms.  
  - Power draw < E% per 1k ops.  

### For Libraries
- **Ops/sec:** e.g., BLAKE3 hashes/sec.  
- **Allocations/op:** measured via `cargo bench + heaptrack`.  
- **Throughput per thread:** scaling with cores.  
- **Cold start:** crate init < F ms.  

---

## 2. Benchmarks & Harness

- **Micro-bench:** Criterion (`cargo bench`) for hot paths.  
- **Integration load tests:** `testing/performance/*` rigs (wrk, bombardier, gwsmoke).  
- **Profiling:**  
  - `cargo flamegraph` for hotspots.  
  - `tokio-console` for async stalls.  
  - `hyperfine` for CLI latency.  
  - `perf` / `coz` for causal profiling.  

- **Chaos/perf blend:** latency injection, slow-loris, compression bombs.  
- **CI Integration:** nightly perf runs vs baselines.

---

## 3. Scaling Knobs

Document the main levers:

- **Concurrency:** Tokio tasks, semaphores, Tower caps.  
- **Memory:** buffer pools, chunk size (64 KiB vs 1 MiB).  
- **I/O:** streaming vs full-buffer; zero-copy (`bytes::Bytes`).  
- **Horizontal:** add replicas for stateless services.  
- **Vertical:** increase CPU pools (hash/compression).  
- **Edge/Mobile:** adjust chunk size, disable heavy features.  

---

## 4. Bottlenecks & Known Limits

- List current hot spots (e.g., sled write amp, TLS handshake).  
- Flag acceptable vs. must-fix.  
- Tie to Omnigate milestones (Bronze baseline vs. Gold scalability).  

---

## 5. Regression Gates

- CI must fail if:  
  - p95 latency ↑ >10%.  
  - Throughput ↓ >10%.  
  - CPU/mem regress >15%.  

- Baselines stored in `testing/performance/baselines/`.  
- Escape hatch: allow waivers if regression traced to upstream dep.  

---

## 6. Perf Runbook (Triage)

Steps when perf SLOs are breached:

1. **Check flamegraph:** TLS handshake, hashing, serialization hotspots.  
2. **Inspect tokio-console:** task stalls, blocked I/O.  
3. **Review metrics:** `*_latency_seconds`, `bus_overflow_dropped_total`.  
4. **Stress knobs:** increase semaphores, tweak buffer size.  
5. **Chaos toggle:** disable compression/jitter and re-run.  
6. **Edge cases:** test ARM/mobile baseline.  

---

## 7. Acceptance Checklist (DoD)

- [ ] SLOs defined for this crate.  
- [ ] Bench harness runs locally + CI.  
- [ ] Flamegraph/console traces collected at least once.  
- [ ] Scaling knobs documented.  
- [ ] Regression gates wired into CI.  
- [ ] Perf runbook section updated.  

---

## 8. Appendix

- **Reference SLOs (Scaling Blueprint):**  
  - p95 GET <80ms intra-region; <200ms inter-region.  
  - Failures <0.1%; RF observed ≥ RF target.  

- **Reference workloads:**  
  - gwsmoke GET/HEAD/RANGE.  
  - Soak test 24h on echo+mailbox.  

- **Perfection Gates tie-in:**  
  - Gate F = perf regressions barred.  
  - Gate L = scaling validated under chaos.  

- **History:**  
  - Record past regressions/fixes to build institutional knowledge.  

```

---
