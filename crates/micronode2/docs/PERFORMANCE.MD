
---

# ⚡ PERFORMANCE.md — micronode

---

title: Performance & Scaling Profile — micronode
status: draft
msrv: 1.80.0
crate\_type: service
last-updated: 2025-09-23
audience: contributors, ops, perf testers
-----------------------------------------

## 0. Purpose

This document defines the **performance profile** of `micronode`:

* Service SLOs, throughput, and error budgets.
* Benchmarks & workloads it must sustain.
* Perf harness & profiling tools.
* Scaling knobs, bottlenecks, and triage steps.
* Regression gates to prevent drift.

It ties directly into:

* **Scaling Blueprint v1.4** (roles, SLOs, runbooks).&#x20;
* **Omnigate Build Plan** milestones Bronze→Gold.&#x20;
* **Perfection Gates** (F = perf regressions barred, L = scaling chaos-tested).&#x20;

Hardened defaults also apply: **5s timeout; 512 inflight; 500 rps; 1 MiB body cap; decompression ≤ 10×**; ops surfaces `/metrics`, `/healthz`, `/readyz`, `/version`.&#x20;

---

## 1. SLOs / Targets

### Service SLOs (inherit canon; single-tenant profile)

* **Latency:**

  * **p95 GET intra-region < 80 ms**; **inter-region < 200 ms**.&#x20;
  * **PUT p95 < 150 ms** (any region).&#x20;

* **Throughput:**

  * Must sustain at least the **hardened RPS cap** with headroom; default **≥ 500 req/s per instance** (cap is tune-per-service).&#x20;
  * Bronze→Gold plans may lift caps after validation; **graceful backpressure** beyond target (no collapse).&#x20;

* **Error Budget:**

  * 5xx **< 0.1%**; 429/503 **< 1%**; bus overflow **< 0.01%**.&#x20;

* **Resource ceilings (per instance @ target load):**

  * **CPU < 70%/core**, **FD usage < 70%** of soft limit. Micronode memory budgets are profile-small; honor **no unbounded buffers**. &#x20;

* **Cold start / Edge (if built for mobile/edge):**

  * **readyz < 2 s**; power budget ≈ **< 5% CPU per 1k ops** (ARM builds).&#x20;

**Hard limits (non-negotiable, enforced by hardening layer):** 5s / 512 / 500 rps / 1 MiB body cap / **≤ 10×** decompression.&#x20;

---

## 2. Benchmarks & Harness

* **Micro-benchmarks (Criterion):**

  * OAP/1 frame encode/decode; BLAKE3 manifest validation; macaroon + PQ verify path.&#x20;
* **Integration load tests (`testing/performance/*`):**

  * `wrk`/`bombardier` (HTTP GET/PUT), `gwsmoke` N-S path (gateway→index/storage/mailbox).&#x20;
* **Profiling stack:**

  * `cargo flamegraph`, `tokio-console`, `hyperfine`, `perf`/`coz` (causal).&#x20;
* **Chaos/perf blend:**

  * latency injection, slow-loris, decompression bombs, quota storms.&#x20;
* **CI integration:** nightly perf runs vs baselines (artifacts uploaded).&#x20;

---

## 3. Scaling Knobs

* **Concurrency:** Tokio workers (`TOKIO_WORKERS`), Tower concurrency caps (`tower::limit`), per-facet semaphores.&#x20;
* **Memory:** buffer pools (`bytes::Bytes`), **64 KiB streaming chunk size**, **1 MiB OAP frame cap**.&#x20;
* **I/O:** prefer streaming over full-buffer; enable zero-copy reads.&#x20;
* **Horizontal:** add replicas behind gateway LB (stateless).&#x20;
* **Vertical:** more cores for hashing/compression lanes.&#x20;
* **Amnesia mode (default for micronode):** no persistence → less I/O, more RAM churn; readiness **degrades before spill**. &#x20;

---

## 4. Bottlenecks & Known Limits

* **TLS handshakes** (CPU-bound): prefer session resumption/ALPN batching.&#x20;
* **BLAKE3 digest checks:** scale with cores; watch pinned threadpools.&#x20;
* **Mailbox shards** (when feature-on): potential lock contention ≥ 50k msgs/s; tune semaphores.&#x20;
* **Must-fix thresholds:** sustained bus lag > 0.01%, replication fanout overruns, mailbox stalls. Accept brief TLS/cold-start spikes and amnesia-mode memory variance.&#x20;

**Hardened path guarantees:** I/O caps + integrity-fail → **502** + `integrity_fail_total`.&#x20;

---

## 5. Regression Gates

CI **fails** if any of the below regress beyond baseline:

* p95 latency **↑ > 10%**, throughput **↓ > 10%**, CPU/Mem **↑ > 15%**; error budgets busted in 24h soak.&#x20;
* Baselines live in `testing/performance/baselines/`; waivers only if traced to upstream dep (link issue).&#x20;

---

## 6. Perf Runbook (Triage)

1. **Flamegraph:** TLS, hashing, serialization hotspots.&#x20;
2. **tokio-console:** task stalls, blocked I/O, semaphore waits.&#x20;
3. **Metrics:** inspect `*_latency_seconds`, `bus_lagged_total`, `rejected_total{reason}`.&#x20;
4. **Knobs:** adjust semaphores, buffer pools, concurrency caps.&#x20;
5. **Chaos toggle:** disable compression/quotas to isolate cause; re-run.&#x20;
6. **Edge tests:** compare ARM/mobile baseline; watch power.&#x20;

---

## 7. Acceptance Checklist (DoD)

* [ ] SLOs defined (latency/throughput/error/resource).&#x20;
* [ ] Bench harness runs locally + CI; artifacts stored.&#x20;
* [ ] Flamegraph / tokio-console traces captured at least once per release.&#x20;
* [ ] Scaling knobs documented (concurrency/memory/I-O/horizontal/vertical).&#x20;
* [ ] Regression gates wired into CI with baselines.&#x20;
* [ ] Perf runbook section updated and linked from RUNBOOK.&#x20;

---

## 8. Appendix

* **Reference SLOs (Scaling Blueprint):** p95 GET **< 80 ms** intra-region; **< 200 ms** inter-region; failures **< 0.1%**; `rf_observed ≥ rf_target`.&#x20;
* **Reference workloads:** `gwsmoke` GET/HEAD/RANGE; **24h soak** on echo+mailbox.&#x20;
* **Hardening defaults (DoH v2.0):** 5s/512/500rps/1 MiB/≤10×; ops surfaces present; **streaming I/O**; no unbounded buffers.&#x20;
* **Perfection Gates tie-in:** Gate **F** = perf regressions barred; Gate **L** = scaling validated under chaos.&#x20;
* **Amnesia Mode (spec):** RAM-only caches; logs suppressed; time-boxed keys; **degrade readiness** instead of spill (micronode default).&#x20;

---

**Notes for operators**

* If POST `/put` > 1 MiB (non-stream) or decompression exceeds bounds, expect **413**/refusal; switch to **64 KiB streaming**.&#x20;
* Integrity failures on read return **502** and increment `integrity_fail_total`.&#x20;

**Hardened defaults are shared via a small helper layer** (Axum/Tower) so caps/latencies are uniform across services.&#x20;

---
