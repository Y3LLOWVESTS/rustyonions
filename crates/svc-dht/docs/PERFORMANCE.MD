```markdown
---
title: Performance & Scaling — svc-dht
status: draft
msrv: 1.80.0
crate_type: service
last-updated: 2025-10-11
audience: contributors, ops, perf testers
---

# ⚡ PERFORMANCE.md — svc-dht

## 0. Purpose

This document defines the **performance profile** of `svc-dht`:
- Service-level objectives (SLOs) for lookup latency, hop-count, and readiness.
- Benchmarks & workloads it must sustain (per-node & cluster views).
- Perf harness & profiling tools (repeatable, CI-enforced).
- Scaling knobs, known bottlenecks, and triage steps.
- Regression gates to prevent silent drift.

It ties into:
- **Scaling Blueprint v1.3.1** (roles, SLOs, runbooks).
- **Omnigate Build Plan** milestones Bronze→Gold.
- **Perfection Gates** (F = perf regressions barred, L = scaling chaos-tested).
- **IDB** invariants (hop budget, hedging) and **OBSERVABILITY.md** (metrics-as-contract).

---

## 1. SLOs / Targets (Service)

> SLOs apply to steady-state, intra-region traffic unless noted. SLI sources are the canonical metrics in OBSERVABILITY (§5.1).

### Latency
- **Lookup (FIND_VALUE/FIND_NODE)**  
  - **p95** < **200 ms** intra-region  
  - **p99** < **400 ms** intra-region  
  - Inter-region guidance: **p95** < **500 ms**
- **Hop Count**  
  - **p99** **≤ 5** hops (hard SLO; alerts fire otherwise)

### Throughput (per node)
- **Sustain** ≥ **2,500 RPC/s** mixed workload (60% FIND_VALUE, 35% FIND_NODE, 5% PROVIDE)  
- **Burst** absorb 5,000 RPC/s for 60s with **graceful backpressure** (Busy/NotReady; no panics)

### Error Budget
- 5xx (HTTP ops plane) **< 0.1%**
- `429|503` combined **< 1%**
- `queue_dropped_total / (queue_dropped_total + processed)` **< 0.01%**
- `tls_handshake_failures_total` rate **near-zero** (investigate if > 1/min)

### Resource Ceilings @ target load (per node)
- **CPU**: < **70%** average across cores; no core pegged > 90% for > 5m
- **Memory**: < **1.0 GiB** steady; no unbounded growth (GC/fragmentation checked)
- **FD usage**: < **60%** of limit
- **GC/allocs**: < **2 allocations** per hot-path RPC (decode → route → encode), amortized via arenas/`Bytes`

### Bootstrap / Readiness
- Cold start **median ≤ 120s**, **p95 ≤ 300s** to `readyz=200`
- `dht_ready_bucket_fill_pct` reaches configured threshold within **300s**

### Edge Profile (Micronode)
- Cold start **≤ 800 ms** to begin dialing seeds
- Steady power < **1.5 W** @ 500 RPC/s (guidance), chunk size 32–64 KiB

---

## 2. Benchmarks & Harness

### 2.1 Micro-benchmarks (Criterion)
- Focus: **CBOR encode/decode**, **BLAKE3 derivations**, **signature verification**, **node distance & sort**, **ASN entropy calc**.
- Location: `benches/`  
- Run:
```

cargo bench -p svc-dht

```

### 2.2 Integration / Load
- **Synthetic peer swarm** (Tokio): spawns N peers, randomized RTT/jitter, drives mixed RPCs:
```

cargo run -p svc-dht-perf -- --peers 2000 --rps 3000 --alpha 3 --beta 1 --k 20 --duration 300s

```
- **wrk/bombardier** for HTTP ops plane:
```

bombardier -c 128 -d 60s [http://127.0.0.1:9108/metrics](http://127.0.0.1:9108/metrics)

```
- **gwsmoke** (if available): scripted DHT probes with histograms.

### 2.3 Profiling
- Flamegraph (CPU hotspots):
```

cargo flamegraph -p svc-dht --bin svc-dht -- --alpha 3 --beta 1

```
- Tokio Console (async stalls):
```

RUSTFLAGS="--cfg tokio_unstable" 
TOKIO_CONSOLE_BIND=127.0.0.1:6669 
cargo run -p svc-dht -- --profile macronode

```
- `perf` / `coz` (optional causal profiling):
```

sudo perf record -g -- cargo run -p svc-dht -- --profile macronode

````

### 2.4 Chaos/Perf Blend
- Latency injection & slow-loris at transport; compression bomb checks; random task abort in lookup workers to validate hedging and recovery.

### 2.5 CI Integration (nightly)
- Nightly perf runs compare against baselines under `testing/performance/baselines/*.json`
- Artifacts: flamegraphs (.svg), tokio-console records, metrics snapshots (.prom)

---

## 3. Scaling Knobs

| Knob | Default | Range | Effect |
|------|---------|-------|--------|
| `alpha` (α) | 3 | 1–8 | Parallel closest-probe fanout; ↑ reduces tail, ↑ load |
| `beta` (β) | 1 | 0–4 | Hedged probes; cuts tail; cancel losers aggressively |
| `k` | 20 | 16–32 | Return set size & bucket width; ↑ memory & sorting cost |
| `hop_budget` | 5 | 3–8 | Max hops; protects against pathologies |
| `rpc_timeout` | 1500ms | 800–2500ms | Probe deadline; tunes tail vs retries |
| `hedge_after` | 250ms | 100–600ms | Hedging delay; too low == self-DDOS |
| `queue_cap.work` | 512 | 256–2048 | Backpressure depth; too high == latency |
| `bucket_refresh_interval` | 30s | 10–120s | Churn resilience vs background load |
| `asn_max_pct` | 40% | 25–60% | Eclipse resistance vs reachability |
| `asn_entropy_min` | 1.5 bits | 1.0–2.0 | Diversity floor |
| `frame_max` | 1 MiB | fixed | DoS guardrail |
| `chunk_size` | 64 KiB | 32–128 KiB | I/O balance (edge tuning) |

**Horizontal scaling:** stateless service—scale replicas linearly until shared infra (metrics/seed lists) bottlenecks.  
**Vertical scaling:** cpu-bound on verify/sort; memory bound on buckets & inflight maps.

---

## 4. Bottlenecks & Known Limits

### Current hotspots (ranked)
1. **Signature verification** (`Ed25519`, `ML-DSA-87`) on PROVIDE bursts  
 *Mitigation:* batch verify (where safe), move to worker pool, cache good publishers.
2. **TLS/QUIC handshake churn** during sudden peer rotations  
 *Mitigation:* connection pooling, keep-alives, conservative idle timeouts.
3. **CBOR encode/decode** allocations (large `NodeInfo` vectors)  
 *Mitigation:* pre-size vectors, `Bytes` zero-copy, arena/stack-alloc for small structs.
4. **Distance sort** (xor metric) on large candidate sets  
 *Mitigation:* partial selection (`select_nth_unstable`), keep K-heap.
5. **ASN diversity computation** (entropy per bucket)  
 *Mitigation:* incremental rolling metrics; O(1) updates on insert/evict.

### Must-fix vs Acceptable
- **Must-fix**: sustained violation of p99 hops, verify path > 30% CPU, GC/alloc regression > 20%  
- **Acceptable**: microbench variance ±5%, single-core spike < 60s under burst

### Milestones (Omnigate)
- **Bronze**: 1,500 RPC/s, p95 < 250ms, CPU < 70%  
- **Silver**: 2,500 RPC/s, p95 < 200ms, hop p99 ≤ 5, bootstrap p95 ≤ 300s  
- **Gold**: 4,000 RPC/s, p95 < 150ms, burst 6,000/s for 2 min without error budget burn

---

## 5. Regression Gates (CI)

- **Fail build** if any of:
- `lookup p95` ↑ **> 10%** vs baseline
- `throughput` ↓ **> 10%** vs baseline
- `CPU avg` or `alloc/op` ↑ **> 15%**
- `hop p99` > **5**
- `bootstrap p95` > **300s**
- Baselines: `testing/performance/baselines/{intra,inter,edge}.json`
- Waivers: allowed only with root-cause (e.g., upstream TLS change), CHANGELOG note, and signed-off issue.

**Example CI snippet**
```yaml
- name: perf-run
run: cargo run -p svc-dht-perf -- --peers 2000 --rps 3000 --duration 180s --json out.json
- name: perf-compare
run: cargo run -p perf-compare -- --baseline testing/performance/baselines/intra.json --actual out.json --fail-p95=10 --fail-rps=10 --fail-cpu=15
````

---

## 6. Perf Runbook (Triage)

1. **Validate scenario**

   * Confirm load mix (FIND_VALUE/FIND_NODE/PROVIDE) and RPS match plan.
   * Check environment noise (noisy neighbors, CPU throttling).
2. **Inspect flamegraph**

   * Hot stacks: verify, distance sort, CBOR, TLS; confirm instruction hotspots.
3. **Tokio Console**

   * Look for long `.await` stalls, queue backpressure, task churn.
4. **Metrics deep-dive**

   * `dht_lookup_latency_seconds`, `dht_lookup_hops_histogram`
   * `dht_rpcs_total{code=timeout|busy|not_ready}`, `queue_dropped_total{queue}`
5. **Tune scaling knobs**

   * Start with `alpha/beta/hedge_after/rpc_timeout`, then queue caps.
   * For bursts: lower `hedge_after` cautiously; increase `alpha` only if CPU headroom exists.
6. **Chaos toggles**

   * Disable compression; raise TLS session resumption; test with connection reuse.
7. **Edge path**

   * Reduce `chunk_size` to 32 KiB; lower `k` to 16; enable amnesia for RAM-only.
8. **If still failing**

   * Capture pprof/flamegraph and console traces; open a **Perf Incident** with artifacts and exact commit SHA.

---

## 7. Acceptance Checklist (DoD)

* [ ] SLOs set (latency, hops, throughput, readiness).
* [ ] Bench harness runs locally & in CI (nightly).
* [ ] Flamegraph + tokio-console traces captured for **Silver** load.
* [ ] Scaling knobs documented with safe ranges.
* [ ] Regression gates wired; baselines stored and referenced.
* [ ] Runbook updated with recent incident learnings.

---

## 8. Appendix

### 8.1 Reference SLOs (Scaling Blueprint)

* p95 lookup **< 200ms** intra; p99 hops **≤ 5**
* Failures **< 0.1%**; `429|503` **< 1%**

### 8.2 Reference Workloads

* **Intra-region**: RTT 1–3ms, jitter ±1ms, loss < 0.1%
* **Inter-region**: RTT 20–60ms, jitter ±5ms, loss < 0.5%
* **Soak**: 24h at 1,500 RPC/s; monitor drift and memory

### 8.3 Tooling Cheatsheet

```
# quick local perf
cargo run -p svc-dht-perf -- --peers 1000 --rps 2000 --duration 120s

# record flamegraph
cargo flamegraph -p svc-dht --bin svc-dht

# run criterion micro-benches
cargo bench -p svc-dht

# live async insights
RUSTFLAGS="--cfg tokio_unstable" TOKIO_CONSOLE_BIND=127.0.0.1:6669 cargo run -p svc-dht
```

### 8.4 History

* (keep a short log of regressions/fixes with commit SHAs and a one-line root cause)

---

```
```
