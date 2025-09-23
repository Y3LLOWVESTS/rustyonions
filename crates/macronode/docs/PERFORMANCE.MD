

# ⚡ PERFORMANCE.md — macronode

---

title: Performance & Scaling Profile — macronode
status: draft
msrv: 1.80.0
crate\_type: service
last-updated: 2025-09-23
audience: contributors, ops, perf testers
-----------------------------------------

# PERFORMANCE.md

## 0. Purpose

This file defines macronode’s **performance profile**:

* Service-level objectives (latency, throughput, error budgets).
* Benchmarks & workloads to sustain.
* Perf harness & profiling tools.
* Scaling knobs, bottlenecks, triage steps.
* Regression gates to block silent drift.

It ties into:

* **Scaling Blueprint v1.4** (roles, SLOs, runbooks).
* **Omnigate Build Plan** (Bronze → Gold perf milestones).
* **Perfection Gates** (Gate F = regressions barred, Gate L = chaos scaling validated).

---

## 1. SLOs / Targets

### Latency

* **p95 GET (intra-region):** < **80 ms**
* **p95 GET (inter-region):** < **200 ms**
* **PUT p95 (any region):** < **150 ms**

### Throughput

* ≥ **10k req/s per macronode instance** sustained.
* Graceful **backpressure** once above 10k; no collapse.

### Error Budget

* **5xx errors:** <0.1%
* **429/503 rejects:** <1%
* **Bus overflow:** <0.01%

### Resource Ceilings

* **CPU:** < 70% per core at target load.
* **Memory:** < 16 GiB steady state (large profile).
* **FD usage:** < 70% of system soft limit.

### Cold Start / Edge

* Cold start to readyz: < 2s.
* Power draw budget: < 5% CPU per 1k ops (ARM/mobile builds).

---

## 2. Benchmarks & Harness

* **Micro-benchmarks:** Criterion (`cargo bench`) on:

  * OAP/1 frame encode/decode
  * Manifest digest validation (BLAKE3)
  * Capability token verification (macaroon parse + PQ verify)

* **Integration load tests:** `testing/performance/*`:

  * `wrk` / `bombardier` for HTTP GET/PUT
  * `gwsmoke` for full North–South path (gateway→omnigate→index/storage/mailbox)
  * DHT lookup rigs (`test_dht.sh`)

* **Profiling:**

  * `cargo flamegraph` → CPU hotspots
  * `tokio-console` → async stalls, lagging tasks
  * `hyperfine` → CLI latency (`ronctl`)
  * `perf` / `coz` → causal profiling

* **Chaos/Perf blend:**

  * Latency injection, slow-loris, decompression bombs, quota storms.

* **CI Integration:** nightly perf runs vs baselines; artifacts uploaded.

---

## 3. Scaling Knobs

* **Concurrency:**

  * Tokio worker threads (`TOKIO_WORKERS`)
  * Tower service concurrency caps (`tower::limit`)
  * Semaphore limits in DHT/index facets

* **Memory:**

  * Buffer pools (`bytes::Bytes`)
  * Chunk size = 64 KiB (streaming)
  * Frame cap = 1 MiB (OAP/1)

* **I/O:**

  * Streaming IO over full-buffer
  * Zero-copy for storage reads

* **Horizontal scale:** add replicas behind LB (`svc-gateway`).

* **Vertical scale:** more CPU cores for hashing/compression.

* **Amnesia Mode (Micronode):** disables persistence, reduces IO overhead, increases RAM churn.

---

## 4. Bottlenecks & Known Limits

* **TLS handshakes:** CPU-bound; batch with ALPN/TLS resumption.
* **Digest checks (BLAKE3):** scale with cores; pinned threadpool may bottleneck.
* **DHT lookups:** hop inflation under churn; target p99 ≤ 5 hops.
* **Storage replication:** fanout quotas needed; replication storms can saturate disk IO.
* **Mailbox shards:** lock contention under 50k+ msgs/s; semaphore tuning required.

**Must-fix:** bus lag >0.01% sustained, replication > fanout quota, mailbox stalls.
**Acceptable:** brief TLS spikes on cold start, memory spikes under amnesia.

---

## 5. Regression Gates

CI fails if:

* p95 latency ↑ >10% vs baseline.
* Throughput ↓ >10%.
* CPU/mem regress >15%.
* Error budgets exceeded in soak tests (24h).

Baselines in `testing/performance/baselines/`.
Escape hatch: waiver only if regression traced to upstream dep (with issue link).

---

## 6. Perf Runbook (Triage)

When perf SLOs breached:

1. **Flamegraph:** look for TLS, hashing, serialization hotspots.
2. **tokio-console:** task stalls, blocked IO, semaphore waits.
3. **Metrics:** check `*_latency_seconds`, `bus_lagged_total`, `rejected_total{reason}`.
4. **Knobs:** tune semaphores, adjust buffer pools, lift concurrency caps.
5. **Chaos toggle:** disable compression/quotas and rerun.
6. **Edge tests:** ARM/mobile to compare.
7. **Escalate:** if DHT hop > 5 or replication RF < target, run repair (`ronctl repair`).

---

## 7. Acceptance Checklist (DoD)

* [ ] SLOs defined (latency, throughput, error, resource).
* [ ] Bench harness runs in CI.
* [ ] Flamegraph & tokio-console traces captured at least once per release.
* [ ] Scaling knobs documented.
* [ ] Regression gates wired into CI.
* [ ] Perf runbook triage updated.

---

## 8. Appendix

* **Reference SLOs (Scaling Blueprint):**

  * GET p95 <80 ms intra-region, <200 ms inter-region.
  * Failures <0.1%, Quota <1%.
  * RF observed ≥ RF target.

* **Reference workloads:**

  * `gwsmoke` GET/HEAD/RANGE
  * 24h soak on echo+mailbox
  * DHT lookup storm (1e6 queries)

* **Perfection Gates tie-in:**

  * Gate F = perf regressions barred in CI.
  * Gate L = chaos scaling validated (latency injection, storm tests).

* **History:**

  * 2025-08: Optimized BLAKE3 threading, 30% gain.
  * 2025-09: Fixed TLS resumption bug; handshake CPU cut by 20%.

---

✅ With this template, macronode has a **concrete, measurable perf profile** and CI gates to prevent drift as scaling grows from single-node → regional mesh → global deployment.

