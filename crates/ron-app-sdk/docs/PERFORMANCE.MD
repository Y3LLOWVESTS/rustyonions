

````markdown
---
title: Performance & Scaling — ron-app-sdk
status: draft
msrv: 1.80.0
crate_type: lib
last-updated: 2025-10-16
audience: contributors, ops, perf testers
---

# PERFORMANCE.md

## 0. Purpose

This document defines the **performance profile** of `ron-app-sdk`:
- Lib-level SLOs (SDK-added latency, retries bound by deadline).
- Benchmarks & workloads it must sustain.
- Perf harness & profiling tools.
- Scaling knobs, bottlenecks, and triage steps.
- Regression gates in CI to prevent silent drift.

Ties into:
- **Scaling Blueprint v1.3.1** (roles, SLOs, runbooks).
- **Omnigate Build Plan** milestones Bronze→Gold.
- **Perfection Gates** (F = perf regressions barred, L = chaos-tested scaling).

---

## 1. SLOs / Targets (Library)

### 1.1 SDK-Added Overhead (hot path, loopback, warm TLS)
- **p50 ≤ 2 ms**, **p95 ≤ 5 ms** per call for payloads ≤ 64 KiB.
- Measured as **call wall time** minus **gateway echo time** (local loopback harness).

### 1.2 End-to-End Behavior Under Faults
- With **20% transient 5xx** + **2% timeouts** injected:
  - **p95 attempts ≤ 3** (bounded by backoff & deadline).
  - **0 duplicate side effects** for idempotent ops (idempotency key respected).

### 1.3 Allocations / Op (target bands)
- Storage GET (≤64 KiB): **≤ 8 allocs/op**, **≤ 4 KiB transient heap**.
- Storage PUT (≤64 KiB): **≤ 12 allocs/op**, **≤ 8 KiB transient heap**.
- Mailbox send: **≤ 10 allocs/op**.

> These are **engineering bands**; lock in exact numbers once benches stabilize and record baselines.

### 1.4 Cold Start
- `RonAppSdk::new(SdkConfig::default())`: **≤ 5 ms** without Tor; Tor path excluded (external bootstrap).

---

## 2. Benchmarks & Harness

### 2.1 Micro-bench (Criterion) — copy/paste

Create `benches/sdk_benches.rs`:

```rust
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput, black_box};
use ron_app_sdk::{RonAppSdk, SdkConfig};
use bytes::Bytes;
use std::time::Duration;

fn bench_storage_put(c: &mut Criterion) {
    // Use a mock/loopback transport provided by tests or a feature-gated dev server.
    let rt = tokio::runtime::Runtime::new().unwrap();
    let payload = Bytes::from(vec![0u8; 64*1024]);

    c.benchmark_group("storage_put_64k")
        .throughput(Throughput::Bytes(payload.len() as u64))
        .bench_function(BenchmarkId::from_parameter("tls_loopback"), |b| {
            b.to_async(&rt).iter(|| async {
                let mut cfg = SdkConfig::default();
                cfg.overall_timeout = Duration::from_millis(5000);
                // cfg.transport = Transport::Tls; cfg.gateway_addr = "https://127.0.0.1:8443".into();
                let sdk = RonAppSdk::new(cfg).await.unwrap();
                // let cap = test_capability(); // helper from dev harness
                // black_box(sdk.storage_put(cap, payload.clone(), Duration::from_secs(5), None).await).unwrap();
                black_box(());
            })
        });
}

criterion_group!(benches, bench_storage_put);
criterion_main!(benches);
````

Run:

```
cargo bench -p ron-app-sdk
```

> Replace mock calls with your loopback gateway once available (dev harness). Keep benches hermetic: no internet, no Tor.

### 2.2 Integration Load (dev harness)

* **Loopback Gateway** (dev/test binary) implementing OAP/1 echo/ok paths:

  * `storage_put` → returns synthetic `AddrB3`.
  * `edge_get` → serves fixed 64 KiB buffer with byte-range.
  * **Fault injection**: 5xx and timeout ratios toggleable via env.
* Drive with `gwsmoke` or a small Rust loadgen (Tokio tasks, bounded concurrency).

### 2.3 Profiling Tools

* **Flamegraph**: identify serde/envelope/copy hot spots.

  ```
  cargo flamegraph -p ron-app-sdk --bench sdk_benches
  ```
* **tokio-console**: watch stalls in integration tests (enable console feature in host).
* **heaptrack / dhat-rs**: allocations/op on benches.
* **perf / coz**: system-level or causal profiling (Linux).

### 2.4 Chaos/Perf Blend

* Inject latency (P90, P99), random 5xx spikes, and I/O timeouts in loopback gateway.
* Verify **retry pressure** (retries/call) and **deadline adherence**.

---

## 3. Scaling Knobs

* **Concurrency (caller-side)**: The SDK is async; throughput scales with the caller’s task count. Use bounded semaphores in the host when necessary.
* **Connection reuse**: Leverage keep-alive/session reuse in `ron-transport` (if exposed). Avoid reconnect-per-call.
* **Chunking**: Storage streams ~64 KiB chunks. Larger chunks reduce syscalls, smaller improve fairness. Stay within OAP limits.
* **Idempotency**: Set idempotency keys on mutations so retries are safe at higher concurrency.
* **Backoff**: `base`, `factor`, `cap`, and `max_attempts` directly impact tail latency under faults; tune per environment.
* **Tor**: Expect higher latency; consider longer deadlines and possibly fewer concurrent inflight ops to avoid queue contention inside SOCKS.

---

## 4. Bottlenecks & Known Limits

* **TLS handshake**: cold handshakes dominate p95 in short-lived clients. Prefer connection reuse; warm up on start.
* **Serde (DTO strictness)**: `deny_unknown_fields` can add overhead—worth it for safety; mitigate with zero-copy `bytes::Bytes` and pre-allocated buffers.
* **BLAKE3 verification**: optional terminal verify on PUTs costs CPU; gate via config (`cache.verify_puts`/explicit verify).
* **Tor bootstrap/unavailability**: adds large, bursty latency; SDK surfaces `TorUnavailable`—do not mask with retries.
* **Large envelopes**: hard cap **1 MiB**; attempts above cap are rejected early (cheap) to avoid wasted I/O.

Classify:

* **Must-fix regressions**: handshake reuse disabled; retries ignoring deadlines; alloc/op jumps > 2×.
* **Acceptable trade-offs**: +3–5% CPU for DTO strictness; ≤1 alloc/op drift when adding observability fields.

---

## 5. Regression Gates (CI)

* Fail PR if any of the following vs. baseline:

  * `sdk_request_latency_seconds{endpoint}` **p95 +10%** or more (bench histogram compare).
  * **Throughput −10%** on loopback storage PUT/GET benches.
  * **Alloc/op +15%** or **peak RSS +15%** in benches.
* Baselines stored under:

  * `testing/performance/baselines/ron-app-sdk/<bench>.json` (Criterion)
  * `testing/performance/baselines/allocations/*.txt` (heaptrack/dhat)
* Waivers:

  * Allowed only with a linked analysis and issue; time-boxed and revisited next release.

**CI snippet (suggested):**

```yaml
name: perf-guardrails
on: [push, pull_request]
jobs:
  bench:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo bench -p ron-app-sdk -- --output-format bencher > bench.txt
      - run: python3 scripts/compare_bench.py bench.txt testing/performance/baselines/bench.txt --p95-delta 0.10

  flamegraph:
    if: github.event_name == 'schedule'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: EmbarkStudios/cargo-flamegraph@v0.4
      - run: cargo flamegraph -p ron-app-sdk --bench sdk_benches
```

---

## 6. Perf Runbook (Triage)

1. **Confirm environment**: CPU governor, frequency scaling, TLS session reuse, debug flags off.
2. **Check flamegraph**: look for top frames in serde (DTO), hashing (BLAKE3), memcpy.
3. **tokio-console**: identify long polls or blocked tasks (unexpected `.await` under lock).
4. **Metrics**: spikes in `sdk_io_timeouts_total{op}` or `sdk_backoff_retries_total{reason}` → investigate gateway or deadlines too tight.
5. **Tune knobs**: enlarge backoff `cap` slightly; increase overall deadline; ensure idempotency for mutations.
6. **Isolation**: compare loopback vs. real gateway to separate SDK overhead from network.
7. **Tor path**: verify SOCKS reachability; if `TorUnavailable` spikes, do not increase retries—fix transport first.

---

## 7. Acceptance Checklist (DoD)

* [ ] Targets set: overhead p50/p95, attempts under faults, alloc/op bands.
* [ ] Criterion benches land and run locally + CI.
* [ ] At least one **flamegraph** captured and stored under `testing/performance/artifacts/`.
* [ ] Scaling knobs documented; defaults justified.
* [ ] Regression gates wired; baselines committed.
* [ ] Chaos/fault injection harness proves retries bounded by deadlines with no duplicate side effects.

---

## 8. Appendix

### 8.1 Reference SLOs (Scaling Blueprint)

* p95 GET < 80 ms intra-region; < 200 ms inter-region (host service perspective).
* Failures < 0.1%; RF observed ≥ RF target.

### 8.2 Reference Workloads

* `gwsmoke` GET/HEAD/RANGE against loopback gateway.
* 24h soak on mailbox send/recv/ack with idempotency ON and induced 5xx/timeout ratios.

### 8.3 Perfection Gates Tie-in

* **Gate F**: perf regressions barred by CI.
* **Gate L**: scaling validated with chaos/fault injection and sustained soak.

### 8.4 History

* Keep a short log of notable perf changes (e.g., “v0.1.0 reduced alloc/op on storage_get by 35% via zero-copy Bytes”).

```

