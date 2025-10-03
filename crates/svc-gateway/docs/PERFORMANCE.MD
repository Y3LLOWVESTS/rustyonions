
---

````markdown
# ⚡ PERFORMANCE.md — `svc-gateway`

---
title: Performance & Scaling — svc-gateway
status: draft
msrv: 1.80.0
crate_type: service
last-updated: 2025-10-03
audience: contributors, ops, perf testers
---

# PERFORMANCE.md

## 0. Purpose

This document defines the **performance profile** for `svc-gateway`, the hardened, stateless HTTP↔OAP/1 ingress:

- Service-level objectives (SLOs) and guardrails.
- Benchmarks & workloads to sustain.
- Perf harness & profiling tools.
- Scaling knobs, known bottlenecks, and triage steps.
- Regression gates to prevent silent perf drift.

It ties into:
- **Scaling Blueprint** SLOs & runbooks. :contentReference[oaicite:0]{index=0}
- **Hardening defaults** (timeouts, caps, DRR/backpressure). :contentReference[oaicite:1]{index=1} :contentReference[oaicite:2]{index=2}
- **svc-gateway IDB** invariants (DRR, quota-first, OAP/1 bounds, golden metrics). :contentReference[oaicite:3]{index=3}

---

## 1. SLOs / Targets

### 1.1 Latency (read path)
- **Public GET start latency** (time to first byte):
  - **Intra-region p95 < 80 ms**. :contentReference[oaicite:4]{index=4}
  - **Inter-region p95 < 200 ms**. :contentReference[oaicite:5]{index=5}

### 1.2 Latency (write/mutate path)
- For representative 64 KiB requests on the Bronze host, **p95 ≤ 150 ms at 400 rps** (baseline gate). :contentReference[oaicite:6]{index=6}

### 1.3 Throughput & backpressure
- **Design cap (per instance):** **500 rps**; apply DRR/quotas **before** heavy work, shed load early. :contentReference[oaicite:7]{index=7} :contentReference[oaicite:8]{index=8}
- **Target sustained throughput:** operate at **≤ 80%** of configured cap to preserve tail latency; scale horizontally beyond that (LB + health eject on `/readyz`). :contentReference[oaicite:9]{index=9}

### 1.4 Error budget & rejects
- **5xx < 0.1%**; **429/503 < 1%** (sustained, 5m). :contentReference[oaicite:10]{index=10}
- `/readyz` must **shed writes first** under pressure; DRR/backpressure observable. :contentReference[oaicite:11]{index=11}

### 1.5 Resource ceilings (reference)
- **Inflight ≤ 512**, **body cap = 1 MiB**, **decode ratio ≤ 10×** with **absolute decoded cap ≤ 8 MiB**. :contentReference[oaicite:12]{index=12} :contentReference[oaicite:13]{index=13}
- **Timeouts**: request timeout **5 s** (read/write), idle connections per service policy. :contentReference[oaicite:14]{index=14}

### 1.6 Protocol bounds (interop-critical)
- **OAP/1**: **max_frame = 1 MiB**; stream I/O in ~**64 KiB** chunks (no unbounded buffering). :contentReference[oaicite:15]{index=15} :contentReference[oaicite:16]{index=16}

---

## 2. Benchmarks & Harness

### 2.1 Micro-benchmarks (Criterion)
- **Hot paths**:
  - Header parse & route dispatch (Axum/Tower).
  - Capability verification fast-path (token parsing only).
  - Optional decode guard path (gzip/deflate/br gate checks).
- Run with `cargo bench` and export JSON for trend analysis.

### 2.2 Integration load tests
- **Load mix** (representative):
  - **GET /o/{addr}** (16 KiB, 64 KiB, 256 KiB) with/without Range.
  - **PUT/POST** small writes (8–64 KiB) behind caps/quota.
  - **Negative cases**: 1.5 MiB body → expect **413**, 600 rps spike → **429** with `Retry-After`. :contentReference[oaicite:17]{index=17}
- **Tools**: `bombardier`, `wrk`, `hey` for HTTP; scripts under `testing/performance/`.

Example (soak at 80% cap):
```bash
bombardier -c 128 -d 15m -l -m GET http://$BIND/o/$ADDR
````

Spike/limit tests:

```bash
bombardier -c 256 -r 600 -n 30000 -m POST http://$BIND/api/put --body ./payload_64k.bin
```

### 2.3 Profiling

* **Flamegraph** for hotspots; **tokio-console** for stalls; optional `perf`/`coz` for causal profiling.
* Collect artifacts for the worst tail (p99+) run in CI’s nightly perf job.

### 2.4 Chaos/perf blend

* Slow-loris, decompression bomb vectors, restart storms with LB health eject; verify `/readyz` degrades and DRR keeps serving reads.  

---

## 3. Scaling Knobs

* **Concurrency**: Tower concurrency limit, semaphore on inflight (≤ 512), DRR queue weights per route. 
* **Rate limiting**: instance RPS cap (≤ 500), token bucket per principal; reject early (429) before heavy work.  
* **I/O**: prefer streaming bodies (~64 KiB chunks) and **owned bytes** to avoid copies; hard **1 MiB** body cap.  
* **Decode guards**: ratio ≤ 10× and absolute ≤ 8 MiB; refusal path is 413 with structured `reason`.  
* **Horizontal scale**: add replicas; LB should health-eject on `/readyz` within ≤ 15 s in small clusters. 
* **Amnesia mode**: RAM-only buffers/logs; ensure no on-disk spill (micronode default). 

---

## 4. Bottlenecks & Known Limits

* **TLS handshake & header parse**: CPU spikes at cold start / connection churn; amortize with keep-alive and LB tuning.
* **Decode guard checks**: gzip/deflate/br headers & ratio accounting on inbound can show up in p99; ensure short-circuit on cap breach. 
* **Write path backends**: downstream `omnigate`/storage/index dominate tail latency; gateway must shed writes first. 
* **OAP bounds**: hard **1 MiB** frame/body caps—anything larger is 413; do not buffer unbounded. 
* **DRR fairness**: mis-tuned weights can starve low-volume tenants; monitor reject/backpressure metrics. 

---

## 5. Regression Gates

* **Per-PR guard** (micro/short runs):

  * Fail if **p95 latency ↑ > 10%** or **throughput ↓ > 10%** vs previous commit’s short-run baseline.
* **Nightly Bronze baseline** (64 KiB payload @ 400 rps):

  * Fail if **p95 > 150 ms × 1.2** (i.e., >20% regression) or **reject rate > 1%**. Baselines live under `testing/performance/baselines/`. 
* **Resource guards**:

  * Alert on sustained inflight > 90% of 512 cap; decoded rejections > 10/min. 
* **Escape hatch**:

  * Temporary waiver allowed if regression is traced to an upstream dependency and documented with flamegraphs & notes; must pass within 7 days.

---

## 6. Perf Runbook (Triage)

1. **Check metrics dashboard**: `request_latency_seconds`, `http_requests_total`, `rejected_total{reason}`, DRR queue depth; confirm if rejects are quota/decoded/overload. 
2. **Flamegraph**: look for handshake, header parse, decode-guard, capability parse hotspots.
3. **tokio-console**: identify stalls (blocked I/O, wide critical sections).
4. **Toggle knobs**:

   * Lower concurrency limit to protect tail; adjust DRR weights for noisy tenants.
   * Disable decode (if safe) to localize bottleneck; re-run A/B.
5. **Chaos checks**: induce 600 rps spike and verify early 429s; ensure `/readyz` flips to degrade (writes first). 
6. **Downstream verification**: sample calls to `omnigate`/storage/index; if slow, hedge or temporarily prefer cached reads (if policy allows).
7. **Amnesia hygiene**: on micronode, verify zero on-disk spill and memory boundedness. 

---

## 7. Acceptance Checklist (DoD)

* [ ] SLOs declared (GET start latency, error budget, rejects). 
* [ ] Bench harness runs locally + CI; spike/limits tests assert 413/429/503 taxonomy. 
* [ ] Flamegraph & tokio-console traces captured for worst tail.
* [ ] Scaling knobs (concurrency, RPS, decode caps) documented & tunable. 
* [ ] Regression gates wired to baselines (`testing/performance/baselines/*.json`). 
* [ ] `/readyz` degrade-first semantics verified (writes shed first). 

---

## 8. Appendix

### 8.1 Reference SLOs (Scaling Blueprint)

* **GET start latency**: **p95 < 80 ms** intra-region; **< 200 ms** inter-region. **5xx < 0.1%**, **429/503 < 1%**. 

### 8.2 Reference workloads

* `GET /o/{addr}` with 16/64/256 KiB bodies; range requests.
* **Soak**: 24 h at 80% of cap; **Spike**: 120% for 60 s → early 429s; **Decode bomb**: 100 KiB gzip → ~12 MiB → expect 413 and no OOM. 

### 8.3 Protocol constants (interop)

* **OAP/1** `max_frame = 1 MiB`; **storage streaming ~64 KiB**; content addressing via BLAKE3 (`"b3:<hex>"`). 

### 8.4 Perfection Gates tie-in

* **Gate F**: perf regressions barred by CI baselines.
* **Gate L**: scaling validated under chaos (degrade-first ingress). 

```

---
```
