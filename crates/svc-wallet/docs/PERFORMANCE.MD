# ⚡ PERFORMANCE.md — `svc-wallet`

---

title: Performance & Scaling
status: draft
msrv: 1.80.0
crate_type: service
last-updated: 2025-10-16
audience: contributors, ops, perf testers
-----------------------------------------

## 0. Purpose

This document defines the **performance profile** of `svc-wallet`:

* Service-level objectives (SLOs)
* Benchmarks & workloads to sustain
* Perf harness & profiling tools
* Scaling knobs, bottlenecks, and triage steps
* Regression gates to prevent silent perf drift

Direct ties:

* **Scaling Blueprint v1.3.1** (roles, SLOs, runbooks)
* **Omnigate Build Plan** (Bronze → Gold)
* **Perfection Gates** (F = perf regressions barred, L = scaling chaos-tested)

---

## 1. SLOs / Targets

> Assumptions unless noted: single node, 4 vCPU / 8 GiB RAM, intra-region peers, TLS terminated at service (rustls), ledger/auth/policy healthy.

### Latency (intra-region)

* **GET /v1/balance**

  * p95 ≤ **120 ms**, p99 ≤ **250 ms**
* **GET /v1/tx/{txid}**

  * p95 ≤ **120 ms**, p99 ≤ **250 ms**
* **POST /v1/{issue,transfer,burn}** (with ledger commit)

  * p95 ≤ **180 ms**, p99 ≤ **350 ms**
* **/readyz**:

  * p99 ≤ **30 ms** (when green)

### Throughput (sustained, 10 min window)

* **Read-heavy** (95% GET, 5% POST): ≥ **4,000 rps** per node
* **Write-mix** (70% GET, 30% POST): ≥ **2,500 rps** per node (≥ **1,500 rps** writes)
* **Replay burst** (1% idem replays): no latency inflation > **10%** vs write-mix

### Error Budget

* 5xx < **0.1%** of requests
* 429/503 (shed) < **1%** sustained; short spikes allowed during chaos
* Bus overflow (`bus_lagged_total`) < **0.01%** of event volume

### Resource Ceilings (at target load)

* **CPU**: < **75%** average per core
* **Memory**: < **512 MiB** RSS steady state (excluding kernel page cache)
* **FDs**: < **60%** of system limit
* **GC/alloc**: < **2 allocations** per hot request on average (reads)

### Startup

* Cold start (listeners bound, readyz true under healthy deps) < **1.5 s**

---

## 2. Benchmarks & Harness

### 2.1 Workloads (canonical)

1. **Read-Heavy**

* 95% `GET /v1/balance?account=Ai&asset=ron` (randomized keyspace), 5% `GET /v1/tx/{txid}`

2. **Write-Mix**

* 20% `POST /v1/transfer`, 5% `POST /v1/issue`, 5% `POST /v1/burn`, 70% reads
* Transfers random in a fixed account pool; NONCE strictly increasing per account

3. **Replay & Backpressure**

* Same as Write-Mix + 1% POST replays (same `Idempotency-Key`)
* Injection of queue pressure until `429` appears; verify graceful shedding

4. **Adversarial Transport**

* 10% gzip payloads near decompression ratio cap
* 1% oversized to confirm `413` path cost

### 2.2 Tools

* **Load**: `k6`, `vegeta`, `bombardier`, `wrk` (pick any two)
* **Profiles**: `cargo flamegraph`, `pprof-rs` (optional), `tokio-console`
* **OS**: `perf`, `bcc` tools (eBPF), `pidstat`/`vmstat`
* **Chaos**: `tc qdisc` (latency/loss), fault-injection flags in test harness

### 2.3 Example invocations

#### bombardier (write-mix, 4k rps target, 10m)

```
bombardier -c 256 -r 4000 -d 10m \
  -H "Authorization: Bearer $MACAROON" \
  -H "Idempotency-Key: $(uuidgen)" \
  -m POST -f transfer.json https://wallet/v1/transfer
```

#### vegeta (mixed)

```
echo "GET https://wallet/v1/balance?account=acc_1&asset=ron" | vegeta attack -duration=10m -rate=3000 | vegeta report
```

#### k6 (JS script)

```
k6 run testing/performance/k6/write_mix.js
```

#### flamegraph (local)

```
cargo flamegraph -p svc-wallet --bin svc-wallet -- --bind 0.0.0.0:8080
```

#### tokio-console

```
RUSTFLAGS="--cfg tokio_unstable" TOKIO_CONSOLE_BIND=127.0.0.1:6669 \
cargo run -p svc-wallet -- --bind 0.0.0.0:8080
```

---

## 3. Scaling Knobs

> Tune from **Config** (see CONFIG.md) and environment.

**Concurrency**

* Listener concurrency limiter (Tower): `max_conns` (default 1024)
* Worker pool size (committers): `min(num_cpus, max_conns/2)`; override via `WALLET_WORKERS`
* Bounded work queue cap: `512` (reject at 429 when full)

**Retries & Breakers**

* Per-RPC timeout (ledger/auth/policy): `≤1s`
* Retries: idempotent ops only, backoff (50ms → 1s, max 3)
* Circuit breaker: open threshold `20` errors/rolling window; half-open probes `10`

**Memory & Buffers**

* `limits.max_body_bytes` = `1 MiB`
* Gzip decompression ratio cap = `10x`
* Prefer `bytes::Bytes` and zero-copy body handling

**Connection Pools**

* Reuse `reqwest::Client` (HTTP/2 if via edge); pool size ~ `2 × workers`

**Horizontal/Vertical**

* Horizontal scale is primary (stateless service)
* Vertical: add vCPU up to **16** before inter-node coordination outperforms local scaling (due to ledger RPC)

**SDK / Edge**

* Encourage client idempotency to reduce retries under pressure
* Push TLS/mTLS termination to edge when allowed to offload handshakes

---

## 4. Bottlenecks & Known Limits

**Dominant**

* **Ledger RPC latency** (commit path): the critical path for writes; variance here dominates p95
* **Capability verification** (macaroons via `ron-auth`): cache misses cost extra RTT

**Hotspots**

* JSON encode/decode (serde) on large payloads
* TLS handshakes during spike/open-loop tests (prefer keepalive)
* BLAKE3 for receipt hashing (minor cost, but shows in micro)

**Contention**

* Nonce reservation sharding: ensure DashMap shards scale with account cardinality
* Idempotency store (LRU+TTL): hot keys can bounce between shards if hashing is poor

**Acceptable (Bronze)**

* `wallet_idem_replays_total` spikes during client retries

**Must Fix (pre-Gold)**

* p99 write > 350 ms under healthy upstreams
* `queue_dropped_total{queue="work"}` sustained growth at < 60% target load

---

## 5. Regression Gates

> Stored baselines under `testing/performance/baselines/` by scenario.

**Fail CI if any:**

* p95 GET ↑ > **10%** vs baseline (same build flags)
* p95 POST ↑ > **10%**
* Sustained throughput ↓ > **10%**
* CPU or RSS ↑ > **15%** at matched load
* `429|503` rate ↑ > **0.5 pp** absolute at matched load

**Tolerances**

* Allow a single waiver per quarter with root cause analysis (dependency bump, CVE mitigation) and temporary new baseline

**Automation**

```
testing/performance/run.sh --scenario write_mix --duration 10m --rate 2500 --baseline testing/performance/baselines/write_mix.json
```

---

## 6. Perf Runbook (Triage)

1. **Confirm readiness & caps**

   * `/readyz` status, breaker states, queue depth gauges

2. **Hotspot identification**

   * `cargo flamegraph` (focus on `ledger::commit`, serde, rustls)
   * `tokio-console`: long polls, tasks blocked on I/O

3. **Metrics sweep**

   * `request_latency_seconds{route,method}`
   * `upstream_latency_seconds{svc,op}`
   * `circuit_open_total{svc}`, `backoff_retries_total{svc}`
   * `busy_rejections_total{endpoint}`, `queue_depth{queue}`

4. **Hypothesis tests**

   * Increase worker pool by +50% (keep queue at 512); measure p95
   * Push ledger client pool from 2×workers → 3×workers
   * Disable gzip accept (server) to isolate decompression overhead

5. **OS & Network**

   * `ss -s`, `ulimit -n`, `pidstat -dru` for syscall patterns
   * `tc qdisc` to emulate upstream latency/jitter; observe breaker behavior

6. **Fixes**

   * Tune breaker thresholds (avoid flapping)
   * Add cache for auth verification (with strict TTL)
   * Reduce allocs: switch hot JSON paths to `serde_json::from_slice` and borrow where possible

7. **Re-baseline**

   * Once green, update baselines with reasoned justification in PR

---

## 7. Acceptance Checklist (DoD)

* [ ] SLOs documented and agreed (this file)
* [ ] Bench harness runs locally + in CI nightlies
* [ ] Flamegraph and tokio-console traces captured at least once per release line
* [ ] Scaling knobs enumerated and wired to CONFIG
* [ ] Regression gates implemented; baselines stored
* [ ] Perf runbook followed and updated after each triage
* [ ] Observability panels updated (Perf Overview, Upstream Health, Backpressure)

---

## 8. Appendix

### 8.1 Reference SLOs (from Scaling Blueprint)

* p95 GET < **80 ms** intra-region; < **200 ms** inter-region (stretch goals)
* Failures < **0.1%**; RF observed ≥ RF target

### 8.2 Reference Workloads (repo structure)

```
testing/
  performance/
    k6/
      write_mix.js
      read_heavy.js
    baselines/
      write_mix.json
      read_heavy.json
    data/
      accounts_10k.txt
      idempotency_keys.txt
```

### 8.3 Example k6 script (write_mix.js)

```js
import http from 'k6/http';
import { check, sleep } from 'k6';
import { uuidv4 } from 'https://jslib.k6.io/k6-utils/1.4.0/index.js';

export const options = {
  scenarios: {
    write_mix: {
      executor: 'constant-arrival-rate',
      rate: 2500, timeUnit: '1s', duration: '10m',
      preAllocatedVUs: 300, maxVUs: 600
    }
  }
};

const BASE = __ENV.BASE || 'https://wallet';
const MAC = __ENV.MACAROON;

export default function () {
  const idem = uuidv4();
  const payload = JSON.stringify({
    from: 'acc_src', to: 'acc_dst', asset: 'ron', amount_minor: '250000', nonce: 42
  });
  const params = {
    headers: {
      'Authorization': `Bearer ${MAC}`,
      'Content-Type': 'application/json',
      'Idempotency-Key': idem,
    },
    timeout: '5s',
  };
  const res = http.post(`${BASE}/v1/transfer`, payload, params);
  check(res, { 'status is 200/409/429/503': r => [200,409,429,503].includes(r.status) });
  sleep(0.001);
}
```

### 8.4 Chaos helpers

**Add 80 ms RTT and 1% loss to ledger upstream (docker/dev):**

```
tc qdisc add dev eth0 root netem delay 40ms 10ms 25% loss 0.5%
```

**Remove:**

```
tc qdisc del dev eth0 root
```

### 8.5 Perfection Gates tie-in

* **Gate F:** CI blocks merges if perf regressions exceed thresholds
* **Gate L:** Nightlies run chaos and scaling tests; `/readyz` flip behavior verified

### 8.6 History

* 2025-10-16: Initial SLOs & harness defined; baselines established for read-heavy and write-mix

---

**Bottom line:** `svc-wallet` must sustain low-latency reads, predictable write commits under bounded variance, and graceful shedding under pressure. The harness, knobs, and gates here make that *provable* and *repeatable*.
