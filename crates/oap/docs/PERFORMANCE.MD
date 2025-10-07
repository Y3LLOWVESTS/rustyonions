

---

# ⚡ PERFORMANCE.md — `oap`

---

title: Performance & Scaling
status: draft
msrv: 1.80.0
crate_type: lib
last-updated: 2025-10-06
audience: contributors, ops, perf testers
-----------------------------------------

## 0) Purpose

Defines the **performance profile** of `oap` (OAP/1 framer/validator), a **pure library** (no sockets/TLS/auth).
We specify measurable targets (latency/throughput/allocs), hostile-input behavior, bench rigs, scaling knobs (host-side), CI regression gates, and a triage playbook.

Ties into:

* **Scaling Blueprint** invariants (frame cap 1 MiB; 64 KiB is a host I/O knob).
* **OBSERVABILITY.md** (oap_* metrics, readiness keys, alerts).
* **CONCURRENCY.md** (single-threaded parse; scale via host worker parallelism).
* **SECURITY.md** (reject reasons; hard caps; amnesia mode).
* **GOVERNANCE.md** (SemVer notes if changing perf-relevant invariants).

---

## 1. SLOs / Targets

### 1.1 Library SLOs (decode/encode)

* **Decode p95**: < **20 ms/frame** steady-state across payload classes (≤256 B, 4 KiB, 64 KiB, 1 MiB header-reject path).
* **Encode p95**: < **10 ms/frame** (ACK/END fast-path).
* **Deterministic rejects (100%)** into canonical buckets: `frame_oversize | header_malformed | unknown_envelope | credit_violation | missing_obj | truncated`.
* **Partial-input safety**: truncated inputs must return `NeedMore` without state corruption.
* **Allocations/op**: ≤ **2** small allocs avg (header/env). Payloads are zero-copy (`&[u8]`/`Bytes`).
* **Cold start**: parser/writer init < **1 ms**.

### 1.2 Throughput (reference, 1 core)

* **Tiny frames (≤256 B)**: ≥ **250k frames/s**.
* **64 KiB frames**: ≥ **40k frames/s**.

> Numbers are engineering targets on the CI “perf” runner class; gate to ±10% drift.

### 1.3 Host-observed SLOs (services embedding `oap`)

* **oap_decode_p95_seconds** < **0.020** (intra-region steady state).
* **oap_rejects_total{reason} / oap_frames_total** < **0.5%** (excluding `truncated` that completes within 100 ms).
* **ACK window starvation** (`oap_ack_window_ok=false`) < **5 s** per 5-min window.
* **No `frame_oversize` under normal ops**; any spike triggers immediate degrade.

### 1.4 Profile variants

* **Amnesia mode (Micronode)**: no on-disk corpora; RAM-only buffers. SLOs unchanged; memory ceiling below (§3).
* **PQ-hybrid flag forwarding**: opaque in `HELLO`; **<5%** library overhead budget (should be ~0%—measured to prove).

---

## 2. Benchmarks & Harness

### 2.1 Criterion micro-benches (`/benches`)

* `decode_happy.rs` — `HELLO→START→(DATA×N)→END` with/without `obj`.
* `decode_pathological.rs` — oversize, truncated at every byte boundary, unknown envelope, credit violations.
* `encode_ack.rs` — windowed ACK sequences (algebra & perf).
* Record **ops/s**, **p50/p95**, **allocs/op** (dhat/heaptrack optional on CI), and payload sweeps (≤256 B, 4 KiB, 64 KiB).

### 2.2 Integration load (workspace `/testing/performance`)

* **gwsmoke / bombardier** driving a host (gateway/overlay) that uses `oap`. Export `oap_*` histograms/gauges.
* **ACK window stress**: keep window near zero; verify readiness flips before collapse.

### 2.3 Profiling

* `cargo flamegraph` for CPU hot paths; expect header parse / varint / small-buffer churn.
* `perf` / `coz` for causal profiling.
* Host-side: `tokio-console` to confirm stalls are I/O, not `oap` CPU.

### 2.4 Fuzz / property (CI-required)

* Corpus: truncated at every split point, huge declared lengths, duplicate/unknown headers, replayed END, credit abuse.
* Properties: length-first refusal; `NeedMore` idempotence; ACK monotonicity.

---

## 3. Scaling Knobs

> Protocol constants are fixed (1 MiB max frame). **64 KiB** is a **host I/O chunk** tuning knob, not a protocol size.

**Host-side levers**

* **Parallelism**: spawn N decode workers (bounded). `oap` is CPU-bound & thread-safe; scaling is near-linear until I/O bound.
* **Backpressure**: bounded channels; drop/503 before heap growth.
* **Buffers**: prefer `Bytes`/slices; avoid copying payloads.
* **ACK credit**: maintain a visible gauge; update promptly to prevent starvation.
* **Amnesia**: cap per-conn working-set (see §4); avoid staging buffers >64 KiB unless necessary.
* **PQ**: forwarding of HELLO feature bits is opaque; ensure no extra crypto work in the `oap` hot path.

---

## 4. Bottlenecks & Known Limits

* **Oversize defense (MUST)**: refuse `len > 1 MiB` at header stage; never pre-alloc payload buffers based on untrusted length.
* **Pathological splits (WATCH)**: extreme fragmentation increases header parse overhead; keep p95 decode <20 ms; add a small look-ahead buffer.
* **ACK algebra (HOST)**: violations reject in `oap`; starvation is a **host readiness**/credit update problem.
* **No hashing here (FORBIDDEN)**: object integrity lives where objects materialize (storage/overlay). Don’t add hashing to `oap` hot path.
* **Amnesia ceilings**: in RAM-only profiles, track working-set; alert if resident set for `oap` staging > **128 MiB** per process at target TPS.

Acceptable vs must-fix:

* Acceptable: small p95 drift under chaos (<10%) if backpressure holds.
* Must-fix: any oversize pre-alloc; unbounded inflight; reject mis-classification; starvation without readiness flip.

---

## 5. Regression Gates (CI)

Fail CI if any bench deviates from last **green baseline** on the same runner class by:

* **p95 latency** ↑ > **10%** (decode or encode).
* **Throughput** ↓ > **10%** (tiny or 64 KiB).
* **Alloc/op** ↑ > **15%**.
* **PQ variant** (HELLO with PQ flags) overhead > **5%**.

Baselines live at `testing/performance/baselines/oap/`. Updates require a PR note justifying the change (e.g., added hardening).

**Example GitHub Actions step (illustrative):**

```yaml
- name: Bench (oap)
  run: cargo bench -p oap -- --save-baseline oap-current
- name: Compare to baseline
  run: scripts/perf_compare.sh testing/performance/baselines/oap oap-current --fail-p95=10 --fail-tps=10 --fail-alloc=15
```

---

## 6. Perf Runbook (Triage)

1. **Invariant check**: confirm 1 MiB cap and that 64 KiB is treated as host chunk size only.
2. **Flamegraph**: look for header/varint hotspots; if small-buffer churn dominates, test a slightly larger look-ahead.
3. **tokio-console (host)**: if stalls, it’s likely I/O or quotas—`oap` is CPU-bound.
4. **Metrics**: inspect `oap_rejects_total{reason}` and `oap_decode_p95_seconds`.

   * `frame_oversize` spikes → hostile peer or missing cap.
   * `credit_violation` → host ACK bug.
   * `header_malformed` → version/header mismatch.
5. **Readiness**: if `oap_reject_rate_ok=false` or `oap_ack_window_ok=false`, degrade `/readyz` (brownout) before collapse.
6. **Chaos toggles**: re-run with compression/PQ features off (should not change `oap` CPU); if it does, investigate host layers.
7. **Amnesia drill**: switch amnesia=ON; verify perf parity and resident set ceiling; if improved, you have disk I/O or cache pressure elsewhere.
8. **Escalation thresholds**: page ops if any holds ≥5 min: `oap_decode_p95_seconds > 0.020`, or reject ratio ≥1%, or ack_window_ok=false time ≥30 s/5 min window.

---

## 7. Acceptance Checklist (DoD)

* [ ] SLOs baselined (decode/encode p95; throughput; alloc/op).
* [ ] Criterion benches & payload sweeps checked in.
* [ ] Fuzz/property suites cover truncated/oversize/unknown/credit/replay.
* [ ] Host perf rig exports `oap_*` histograms/gauges and readiness keys.
* [ ] CI gates wired with baseline compare + waiver process.
* [ ] PQ variant bench recorded (<5% overhead).
* [ ] Amnesia drill performed; resident-set ceiling documented.
* [ ] Runbook escalation thresholds validated once.

---

## 8. Appendix

**Protocol constants & versioning**

* Envelopes: `HELLO | START | DATA | ACK | END | ERROR`.
* `MAX_FRAME_BYTES = 1_048_576` (1 MiB) — **protocol-fixed** (changing requires a major).
* Streaming chunk **64 KiB** — **host I/O** tuning; not a protocol size.

**Reference workloads**

* Happy path with/without `obj:"b3:<hex>"`.
* Reject suite: oversize, missing_obj, unknown, truncated, credit violation.

**Perfection Gates**

* **F**: perf regressions barred via CI deltas.
* **L**: scaling validated under chaos (window pressure & fragmentation) with readiness flips before collapse.

**History**

* Track notable regressions/fixes/waivers here.

---

