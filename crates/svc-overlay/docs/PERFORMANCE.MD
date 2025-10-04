

---

# ⚡ PERFORMANCE.md — svc-overlay

---

title: Performance & Scaling — svc-overlay
status: draft
msrv: 1.80.0 (PQ benchmarks require rustls+Kyber feature)
crate_type: service
last-updated: 2025-10-03
audience: contributors, ops, perf testers
-----------------------------------------

# PERFORMANCE.md

## 0. Purpose

Defines the **performance profile** for `svc-overlay`, RustyOnions’ **session & gossip plane** (OAP/1 over TLS/QUIC/Tor via `ron-transport`) that enforces **OAP/1 framing**, bounded fanout, and truthful readiness (overlay ≠ DHT/routing).

This document sets:

* **SLOs** for session handshake, object fetch via overlay RPC, and gossip paths.
* **Bench workloads** and **perf harness**.
* **Scaling knobs** (timeouts, queues, windows).
* **Bottlenecks & triage**.
* **Regression gates** to prevent silent perf drift.

It ties into: **Scaling Blueprint v1.3.1** (roles/SLOs), **Omnigate Build Plan** (Bronze→Gold), and **Perfection Gates** (F/L).

---

## 1. SLOs / Targets

### 1.1 Control-plane (sessions/handshake)

* **Handshake latency (p95)**

  * **Intra-cluster:** < **50 ms**
  * **Inter-region:** < **150 ms**
  * **PQ-hybrid (Kyber/X25519)**: < **75 ms** intra; overhead ≤ **30%** vs classical.

* **Truthful readiness availability:** `readyz_state==1` ≥ **99.9%** monthly.

* **Frame reject rate:** oversize/ratio-cap < **0.5%** of RX frames.

### 1.2 Data-plane (overlay RPC fetch path)

* **Latency (p95, warm path, single hop gateway→overlay)**

  * **≤256 KiB object:** intra < **80 ms**; inter < **200 ms**.
  * **≈1 MiB object (1 frame):** intra < **120 ms**; inter < **300 ms**.

* **Throughput (sustained)**

  * **Bronze baseline:** ≥ **500 req/s** per node on **4 vCPU / 8 GiB**.
  * **Silver target:** ≥ **1k req/s** per node on **8 vCPU / 16 GiB**.
  * **Gold target:** linear horizontal scaling; graceful shedding when saturated (no tail collapse).

* **Error budget**

  * Failures < **0.1%**
  * Quota 429/503 shed < **1%**
  * Bus overflow < **0.01%**

### 1.3 Resource ceilings

* CPU < **70%** per core.
* Memory < **512 MiB** steady state.
* FD usage < **40%** of system limit.
* Timeouts: read/write ≈ **5 s**, idle ≈ **15–30 s**.

### 1.4 Edge / Micronode profile

* Cold start < **300 ms** to accepting sessions.
* Prefer **amnesia=true** (RAM-only) for perf & privacy.

---

## 2. Benchmarks & Harness

### 2.1 Micro-bench (Criterion)

* **OAP/1 encode/decode** (1 KiB control, 64 KiB & 1 MiB data frames).
* **Zstd COMP flag** on/off at 64 KiB.
* **Handshake (HELLO→READY)** under timeout caps.
* **PQ variant**: hybrid handshake (Kyber/X25519); assert ≤30% overhead.

Run:

```
cargo bench -p svc-overlay
```

### 2.2 Integration load

* **overlay-smoke (custom):**

  * S-class: 1 KiB @ 1–5k rps, fanout 1:3.
  * M-class: 64 KiB @ 500–1k rps.
  * L-class: 1 MiB @ 100–300 rps.
* Emulate **gateway→overlay GET loop** with stubs for downstream.
* Soak: **24 h** at Bronze rate; no memory growth or latency drift.
* **Polyglot interop:** include Rust + TS SDK clients; p95 latencies within **10%** of each other.

### 2.3 Profiling & observability

* `cargo flamegraph` for hotspots.
* `tokio-console` for async stalls.
* `hyperfine 'curl -s localhost:9600/readyz'` for admin latency.
* `perf` / `coz` for causal profiling.

### 2.4 CI perf

* Nightly perf job compares to baselines.
* Fail if regression gates (§5) are tripped.

---

## 3. Scaling Knobs

* **Concurrency:** Tokio worker count, per-route semaphores, fair-queue layers.
* **Memory/I/O:** prefer streaming + `bytes::Bytes`; avoid full buffering.
* **Transport caps:**

  * `handshake_timeout = 3s`, `read/write/idle` timeouts.
  * `peer.inflight_frames`, `peer.max_rps`, `peer.send_window_frames`.
  * **max_frame = 1 MiB**, **chunk_bytes ≈64 KiB**.
* **Horizontal:** stateless — add replicas behind gateway.
* **Edge:** lower `chunk_bytes`, disable compression, keep `amnesia=true`.

---

## 4. Bottlenecks & Known Limits

* TLS handshake cost at churn; prefer reuse; QUIC 0-RTT off unless gated.
* Oversize/ratio-cap rejections; monitor `rejected_total{reason}`.
* Downstream latency dominates GETs; overlay must preserve backpressure.
* Copy amplification if not using `Bytes`.
* Admin plane (HTTP) must remain fast under load.

---

## 5. Regression Gates

Fail CI perf job if:

* p95 latency ↑ > **10%**.
* Throughput ↓ > **10%**.
* CPU/mem ↑ > **15%** at same load.

**Policy:** If regression, PR must include a **CHANGELOG entry** with root cause + baseline artifact link.
Waivers only if traced to upstream dep.

---

## 6. Perf Runbook (Triage)

1. Flamegraph → TLS, serialization, zstd.
2. Tokio-console → stalled tasks, `.await` misuse.
3. Metrics → `request_latency_seconds`, `rejected_total{reason=*}`, `bus_lagged_total`.
4. Stress knobs → adjust inflight, window, max_rps.
5. Chaos toggles → disable compression, inject latency, re-profile.
6. Edge cases → ARM/micronode with amnesia=true.

---

## 7. Acceptance Checklist (DoD)

* [ ] SLOs defined (handshake, GET, gossip).
* [ ] PQ-hybrid and SDK-parity benchmarks run.
* [ ] Bench harness in CI.
* [ ] Flamegraph + tokio-console traces captured.
* [ ] Scaling knobs documented in CONFIG.
* [ ] Regression gates enforced nightly.
* [ ] CHANGELOG entries for regressions.
* [ ] Runbook exercised with chaos tests.

---

## 8. Appendix

* **OAP/1 invariants:** `max_frame = 1 MiB`, `chunk_bytes ≈64 KiB`.
* **Admin endpoints:** `/metrics`, `/healthz`, `/readyz`, `/version`.
* **Reference SLOs:** intra GET <80 ms; inter GET <200 ms.
* **Interop SLO:** Rust vs TS SDK latencies within 10%.
* **Profiles:** Micronode (amnesia-first), Macronode (multi-service).
* **History:** append regressions/fixes + root causes.

---
