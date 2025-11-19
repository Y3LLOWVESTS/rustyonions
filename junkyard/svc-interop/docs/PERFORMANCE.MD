

````markdown
---
title: Performance & Scaling — svc-interop
status: draft
msrv: 1.80.0
crate_type: service
last-updated: 2025-10-12
audience: contributors, ops, perf testers
---

# PERFORMANCE.md — svc-interop

## 0. Purpose

Defines the **performance profile** for `svc-interop`:
- SLOs (latency/availability/error budget) and per-node throughput targets.
- Benchmarks & representative workloads (webhooks, PUT, GET by BLAKE3).
- Perf harness, profiling tools, and automated regression gates.
- Scaling knobs (config & runtime), known bottlenecks, and triage playbook.

Ties to:
- **Scaling Blueprint v1.3.1** (roles, SLOs, runbooks).
- **Omnigate Build Plan** milestones Bronze→Gold.
- **Perfection Gates** (F = perf regressions barred, L = scaling chaos-tested).

---

## 1. SLOs / Targets (Service)

### 1.1 Latency (intra-region; steady state, p95)
- `GET /o/:addr` (read): **≤ 80 ms**
- `POST /put` (normalized): **≤ 120 ms**
- `POST /webhooks/:provider`: **≤ 120 ms**

> Inter-region guidance: add 100–150 ms to the above p95 targets.

### 1.2 Throughput (per node)
- Sustained: **≥ 400 req/s** mixed (50% GET, 30% webhook, 20% PUT)
- Burst handling: **token bucket 500 RPS** with **250 burst**; graceful backpressure (429/503 with `reason="BACKPRESSURE|RATE_LIMIT"`)

### 1.3 Availability & Error Budget (monthly)
- Bridge endpoints (PUT/webhooks): **≥ 99.9%**
- Read endpoints (GET): **≥ 99.95%**
- 5xx error rate: **< 0.1%**
- 429/503 due to quotas/backpressure: **< 1%** of total requests
- Bus overflow drops: **< 0.01%** of events

### 1.4 Resource ceilings (per node @ target load)
- CPU: **< 70%** avg per core
- Memory: **< 512 MiB** RSS steady, **< 768 MiB** p95
- FD usage: **< 60%** of ulimit-n
- GC/alloc: no sustained allocator growth; fragmentation alarms disabled if <5%

---

## 2. Benchmarks & Harness

### 2.1 Workloads (representative)
- **R1 — Read hot path**: `GET /o/b3:<hex>` N% cache hits (tune via storage gateway)
- **W1 — Webhook verify**: GitHub/Stripe/Slack signatures (HMAC-SHA256 at edge) + BLAKE3 internal hash + enqueue/audit
- **P1 — PUT**: 64 KiB..1 MiB payloads; identity/gzip/zstd encodings; BLAKE3; mailbox enqueue

### 2.2 Driver tools
- **bombardier** (HTTP load), **wrk** (Lua if needed), **k6** (scenarios & thresholds)
- **criterion** (microbench for hashing/DTO parse)
- **tokio-console** (async stalls), **cargo flamegraph** (CPU hotspots)
- **perf/callgrind** (Linux), **coz** (causal profiling)

### 2.3 Example harness

_run a 10-minute mixed test against localhost:_

```bash
bombardier -c 200 -d 10m -l \
  -m GET http://127.0.0.1:8080/o/b3:7f1a... \
  -m POST:http://127.0.0.1:8080/put@./testing/payloads/256k.json \
  -m POST:http://127.0.0.1:8080/webhooks/github@./testing/webhooks/github-ping.json
````

*k6 scenario with SLO thresholds (snippet):*

```js
import http from 'k6/http';
import { Trend, check } from 'k6';
export let options = {
  scenarios: {
    mixed: {
      executor: 'constant-arrival-rate',
      rate: 400, timeUnit: '1s', duration: '10m', preAllocatedVUs: 200, maxVUs: 400
    }
  },
  thresholds: {
    'http_req_duration{route:/o/:addr}': ['p(95)<80'],
    'http_req_duration{route:/put}': ['p(95)<120'],
    'http_req_duration{route:/webhooks/:provider}': ['p(95)<120'],
    'http_req_failed': ['rate<0.001']
  }
};
export default function () {
  const r = Math.random();
  if (r < 0.5) { http.get('http://127.0.0.1:8080/o/b3:7f1a...'); }
  else if (r < 0.8) { http.post('http://127.0.0.1:8080/webhooks/github', JSON.stringify({zen:"..." }), { headers: {'Content-Type':'application/json','X-Hub-Signature-256':'sha256=...'} }); }
  else { http.post('http://127.0.0.1:8080/put', JSON.stringify({payload:"Zm9vYmFy"}), { headers: {'Content-Type':'application/json'} }); }
}
```

*criterion microbench (hash & DTO parse):*

```rust
use criterion::{criterion_group, criterion_main, Criterion};
fn bench_b3(c: &mut Criterion) {
    let data = vec![0u8; 1<<20]; // 1 MiB
    c.bench_function("blake3_1MiB", |b| b.iter(|| blake3::hash(&data)));
}
criterion_group!(benches, bench_b3);
criterion_main!(benches);
```

### 2.4 Profiling quickstart

* CPU hotspots: `cargo flamegraph -p svc-interop --bin svc-interop`
* Async stalls: run with `RUSTFLAGS="--cfg tokio_unstable"` and attach `tokio-console`
* Linux low-overhead: `perf record -F 99 -g -- target/release/svc-interop ...`

---

## 3. Scaling Knobs (and where to set them)

| Knob                       | Purpose                    | Default   | How to tune                                                  | Source          |
| -------------------------- | -------------------------- | --------- | ------------------------------------------------------------ | --------------- |
| `limits.max_rps`           | global rate limit per node | 500       | Lower if CPU spikes; raise cautiously after profiling        | `CONFIG.md`     |
| `limits.burst_rps`         | short-term burst           | 250       | Match LB burstiness; keep ≤ 50% of `max_rps`                 | `CONFIG.md`     |
| `max_conns`                | concurrent conns cap       | 512       | Align with file limits + LB keep-alive strategy              | `CONFIG.md`     |
| `work mpsc capacity`       | queue depth for jobs       | 512       | If `busy_rejections_total` rises with low CPU, consider +128 | Concurrency     |
| `read/write/idle timeouts` | DoS protection             | 5s/5s/60s | Reduce on high tail latency; ensure downstreams match        | `CONFIG.md`     |
| `stream chunk`             | streaming granularity      | 64 KiB    | Larger for fewer syscalls; ensure ≤ 2 buffered               | IDB/Interop     |
| `audit sink`               | audit throughput           | memory    | File sink only out of amnesia; increase mpsc cap if dropped  | Security/Config |
| `otel sampling`            | tracing overhead           | 1% info   | Raise during investigations; revert in steady state          | Observability   |

Horizontal scale: the service is stateless; add replicas behind the gateway.
Vertical scale: if CPU-bound on hashing, enable `blake3-simd` feature and favor AVX2 hosts.

---

## 4. Bottlenecks & Known Limits

* **Edge verification (HMAC-SHA256) + BLAKE3**: double hashing per webhook. Mitigation: keep BLAKE3 (internal) and only verify SHA256 at edge; reuse canonicalized bytes across both steps to avoid copies.
* **Small payload overhead** (<4 KiB): per-request fixed costs (routing, JSON serde). Mitigation: batch PUTs via mailbox where possible; use `bytes::Bytes` zero-copy paths.
* **Compression guard**: zstd high compression levels can raise CPU; keep provider/content encodings to identity/gzip at the edge; cap ratio to 10×.
* **TLS handshake** (if not terminated upstream): spikes p95 under connection churn. Mitigation: long keep-alive, LB re-use, prefer upstream termination.
* **Queue saturation**: `work` mpsc full triggers Busy (good) but can cause oscillation. Mitigation: brownout thresholds, quick retry with jitter your callers side.
* **Disk-backed audit** (if enabled): slow FS → `audit_drop_total` increments. Mitigation: memory sink or buffered writer; avoid disk in amnesia mode.

Bronze target = meets SLO at 400 rps/node.
Gold target = 800 rps/node with p95 ≤ 120 ms by scaling replicas and enabling SIMD BLAKE3.

---

## 5. Regression Gates (CI)

* **Fail PR** if any holds true vs baseline:

  * p95 latency ↑ **> 10%** for `/o`, `/put`, or `/webhooks`
  * Throughput ↓ **> 10%** at fixed error budget (≤0.1% 5xx)
  * CPU or RSS ↑ **> 15%** at the same load
* Baselines live in `testing/performance/baselines/` with JSON:

  ```json
  {"route":"/o/:addr","metric":"p95_ms","value":72}
  ```
* Nightly job runs the k6 scenario for 10m; compares to last green.
* Waiver process: require linked issue + evidence (e.g., dependency upgrade regression) and time-boxed exception.

---

## 6. Perf Runbook (Triage)

1. **Confirm load realism**: traffic mix ~50/30/20 (GET/webhook/PUT), RPS ≤ 500.
2. **Check dashboards**:

   * `request_latency_seconds{route}` p95, `http_requests_total{status}`, `rejected_total{reason}`.
   * `queue_depth{queue="work"}` and `queue_dropped_total{queue="audit"}`.
3. **Flamegraph**: if >25% in hashing/serde → enable `blake3-simd`, move heavy serde to borrowed views, ensure `deny_unknown_fields` doesn’t force extra copies.
4. **tokio-console**: look for tasks waiting on I/O; confirm no lock held across `.await` (lint + runtime asserts).
5. **Knob sweep**:

   * Lower `max_rps` by 10–20% if CPU headroom < 20%.
   * Increase `work` queue to 640 if Busy spikes with CPU < 50%.
   * Adjust chunk to 128 KiB for large streaming payloads (still ≤ 2 buffered).
6. **Chaos toggles**:

   * Disable compression (edge) and compare tails.
   * Force upstream `passport` latency +200 ms → ensure brownout engages and read paths remain green.
7. **File system & TLS**:

   * If audit to disk, move to memory sink to isolate FS latency.
   * If terminating TLS in-proc, try upstream termination to measure delta.

---

## 7. Acceptance Checklist (DoD)

* [ ] SLOs defined and published (dashboards reference this doc).
* [ ] Perf harness (k6/bombardier + criterion) runs locally and in CI.
* [ ] At least one flamegraph + tokio-console trace captured and archived.
* [ ] Scaling knobs documented and wired to config flags.
* [ ] Regression gates compare against baselines and fail PRs on drift.
* [ ] Runbook validated during a game-day or chaos test.

---

## 8. Appendix

### 8.1 Prometheus snippets (copy-paste)

**p95 GET latency (ms)**

```promql
histogram_quantile(
  0.95,
  sum by (le) (rate(request_latency_seconds_bucket{route="/o/:addr"}[5m]))
) * 1000
```

**Error budget (5xx rate)**

```promql
sum(rate(http_requests_total{status=~"5.."}[5m]))
/
clamp_min(sum(rate(http_requests_total[5m])), 1e-9)
```

**Backpressure rate**

```promql
sum(rate(rejected_total{reason=~"BACKPRESSURE|RATE_LIMIT"}[5m]))
/
clamp_min(sum(rate(http_requests_total[5m])), 1e-9)
```

**Queue pressure**

```promql
avg_over_time(queue_depth{queue="work"}[5m]) > 0.8 * 512
```

### 8.2 Reference workloads

* `gwsmoke` for basic GET/HEAD/RANGE.
* 24h soak (400 rps mixed) with hourly brownout injection.

### 8.3 Perfection Gates tie-in

* **Gate F:** CI blocks perf regressions per §5.
* **Gate L:** scaling validated under chaos (brownout + downstream latency).

### 8.4 History (fill as we iterate)

* 2025-10-12: Initial baselines captured on c6i.large @ 400 rps mixed, p95 GET 68 ms, PUT 104 ms, webhook 97 ms, CPU 58%.

```

