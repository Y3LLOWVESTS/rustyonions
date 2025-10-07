

---

# 🛠️ RUNBOOK — `oap`

---

title: RUNBOOK — oap
owner: Stevan White
msrv: 1.80.0
last-reviewed: 2025-10-06
audience: operators, SRE, auditors
----------------------------------

## 0) Purpose

Operational manual for **oap**: how hosts wire it up, how to read oap-fed health signals, what to do under failure, and how to scale without violating invariants. Satisfies **PERFECTION_GATES** K (Continuous Vigilance) and L (Black Swan economics).
**Scope:** `oap` is a **pure protocol library** (no sockets/FS/DB); all endpoints/quotas live in **host services** (e.g., gateway/overlay).
**Polyglot note:** Non-Rust hosts (FFI/SDKs) must export the same readiness keys and metrics; watch FFI overhead in hot paths.

---

## 1) Overview

* **Name:** `oap`
* **Role:** OAP/1 **framing + validation** library; converts byte slices → typed envelopes or typed rejects.
* **Criticality Tier:** **2 (supporting)** — embedded by Tier-1 services (gateway/overlay).
* **Dependencies (lib):** `bytes` (zero-copy); optional parser combinators. **Deliberately excludes hashing/TLS** to prevent scope creep.
* **Ports Exposed:** **None** by `oap`. Metrics/health endpoints are host-owned (typical: `metrics_addr=:9909`).
* **Data Flows:** **Ingress:** host (after TLS/auth) → `oap::decode`. **Egress:** `Envelope` | `Reject(reason)` → host.
* **Version Constraints:** OAP/1 constants frozen: `MAX_FRAME_BYTES = 1 MiB`; envelopes = `HELLO|START|DATA|ACK|END|ERROR`. Changing caps is **major**.

---

## 2) Startup / Shutdown

`oap` ships as a lib; you start/stop the **host** that embeds it.

### Startup (host example)

```bash
RUST_LOG=info ./target/release/svc-gateway --config /etc/ron/gateway.toml --amnesia=on
curl -fsS http://127.0.0.1:9909/readyz
```

**Verify on startup**

* Logs: “oap parser initialized”, config validated, **oap_parser_ready=true**.
* `/readyz`: `200 OK` when all readiness inputs (including OAP keys) are healthy.

### Shutdown

* SIGINT/SIGTERM to host → drain connections → exit. `oap` holds no threads/sockets; nothing to flush.

---

## 3) Health & Readiness

`oap` provides **readiness inputs**; hosts surface them via `/readyz`.

**Keys (all must be true to serve):**

* `oap_parser_ready` — config/grammar valid.
* `oap_reject_rate_ok` — rolling reject ratio below SLO (default < **1%** / 5m).
* `oap_ack_window_ok` — advertised window not starved (not 0 for > **5s** within 5m).
* `oap_backpressure_ok` — inflight below bound; no queue growth.

**Tip:** Degrade (503) **before** collapse when keys fail; brown-out is success.

---

## 4) Common Failure Modes

| Symptom               | Likely Cause                              | Metric / Log                                                   | Resolution                                                                                                    | Alert Threshold         |
| --------------------- | ----------------------------------------- | -------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- | ----------------------- |
| `/readyz` flapping    | High reject ratio                         | `oap_rejects_total{reason}` rising; `oap_reject_rate_ok=false` | Inspect reasons: `header_malformed` ⇒ client mismatch; `frame_oversize` ⇒ hostile peer; quarantine/rate-limit | Reject ratio >0.5% (5m) |
| Rising decode p95     | Pathological split buffers / CPU pressure | `oap_read_frame_seconds` p95 > 20ms                            | Enable buffer pooling; ensure zero-copy payload; small look-ahead                                             | p95 > 0.02s (5m)        |
| ACK starvation        | Host credit updates lag                   | `oap_ack_window==0` under load                                 | Fix ACK update path; scale workers; lower inflight caps                                                       | window=0 >5s per 5m     |
| Heap growth / OOM     | Unbounded host queues                     | Inflight/queue metrics rising                                  | Enforce bounded channels; drop early; verify 1 MiB refusal at header                                          | Any sustained rise      |
| Unexpected accepts    | Strictness disabled                       | Config shows `strict_headers=false`                            | Re-enable strict headers in prod; retest                                                                      | Any in prod             |
| PQ handshake mismatch | Disallowed HELLO flags                    | `reason="bad_hello_pq"` (host-mapped)                          | Align host policy; disable or allow flags consistently                                                        | Any spike               |

---

## 5) Diagnostics

**Metrics**

```bash
curl -s http://127.0.0.1:9909/metrics | grep -E 'oap_(frames|rejects|read_frame_seconds|ack_window|inflight)'
```

Expect:

* Counters: `oap_frames_total{kind}`, `oap_rejects_total{reason}`
* Histograms: `oap_read_frame_seconds_*`
* Gauges: `oap_ack_window`, `oap_inflight_frames`

**Logs**
Structured JSON w/ `service`, `corr_id`, `tenant`, and **redacted** OAP fields. Never log raw `DATA` or PQ bitmasks.

**Tracing & Perf**

* Use host tracing for correlation.
* Perf hotspots: run `cargo flamegraph -p <host>`; oap hotspots should be header parse/varint, not I/O.

---

## 6) Recovery Procedures

1. **Parser Config Error**
   *Symptom:* `oap_parser_ready=false` at startup.
   *Action:* Validate config; ensure `ack_window_frames ∈ [1,1024]`; restore strict defaults; restart host.

2. **High Reject Rate**
   *Symptom:* `/readyz` degrades on reject-rate.
   *Action:* Check reason distribution; rollback incompatible client build; confirm envelope grammar/versions.

3. **Frame Oversize Spike (DoS/broken peer)**
   *Symptom:* `oap_rejects_total{reason="frame_oversize"}` increases.
   *Action:* Quarantine peer; verify early header-stage refusal; tighten ingress ACLs.

4. **ACK Starvation**
   *Symptom:* `oap_ack_window==0` while traffic continues.
   *Action:* Inspect ACK propagation; increase worker parallelism; reduce inflight; ensure readiness flips.

5. **Heap/Backpressure Emergency**
   *Symptom:* Queue growth, GC churn, OOM risk.
   *Action:* Enforce bounded channels; drop on ingress before queue; confirm zero-copy payloads.

6. **Amnesia RS (RAM-only) Breach**
   *Symptom:* Resident set > **128 MiB** at target TPS in Micronode.
   *Action:* Reduce inflight; verify no temp files; shrink staging buffers; re-test with amnesia=on.

---

## 7) Backup / Restore

N/A for `oap` (no state). Hosts with state (index/storage) own their backup/restore. `oap` must remain amnesia-friendly.

---

## 8) Upgrades

* **SemVer discipline:** protocol invariants changes (e.g., 1 MiB cap) are **major**.
* Update **conformance vectors** + **fuzz corpus** with any behavior change.
* CI gates (`cargo public-api`, semver checks) must pass.
* **PQ opacity:** HELLO feature bits remain opaque; upgrading must not alter `oap` state machine.

---

## 9) Chaos Testing

**Quarterly drills (Gate J/K):**

* **Hostile frames under load:** truncated/oversize/unknown/credit abuse → **deterministic rejects**; `/readyz` browns out before collapse.
* **ACK chaos:** interleave ACK/DATA; verify monotone `ack_up_to`; no starvation >5s/5m.
* **Amnesia chaos:** run with `--amnesia=on` (RAM-only), 30-minute soak; verify resident set < **256 MiB** post-restart, no temp files, perf parity (±10%).

---

## 10) Scaling Notes

* `oap` spawns **no tasks**; scale via **host worker parallelism** (bounded).
* **Rule of thumb:** workers ≤ **2× cores**; beyond that, watch context-switches & cache misses.
* Keep payload **zero-copy**; treat **64 KiB** as a host I/O chunk (not a protocol size).
* Plan capacity so that at **~10k conns** on 4c/8GB, oap CPU stays **<20%** if workers are bounded and backpressure is enforced.

---

## 11) Security Ops

* **Never** log secrets, raw `DATA`, or PQ bitmasks.
* Keep **strict header validation** in prod; relax only in interop labs.
* **PQ HELLO flags**: if disallowed by policy, reject at init; do not alter `oap` state.
* **Micronode amnesia:** ensure hosts avoid any on-disk staging; RAM-only.

---

## 12) References

* CONFIG.md, SECURITY.md, OBSERVABILITY.md, CONCURRENCY.md, TESTS.md, INTEROP.md
* Blueprints: Hardening, Concurrency & Aliasing, Omnigate

---

## ✅ Perfection Gates Checklist

* [ ] **Gate A:** oap metrics green (`oap_frames_total`, `oap_rejects_total`, `oap_read_frame_seconds`, `oap_ack_window`).
* [ ] **Gate J:** Chaos drill passes (deterministic rejects; readiness flips before collapse).
* [ ] **Gate K:** Alerts wired (reject-rate/latency/oversize/ACK starvation/backpressure).
* [ ] **Gate N:** ARM/edge validation recorded (amnesia parity).
* [ ] **Gate O:** Security hardening: length-first refusal; strict headers; zero-copy; no hashing in hot path.
* [ ] **Gate P (PQ):** PQ HELLO flags remain opaque (no state/perf change); overhead < **5%** vs baseline.

---

### Appendix — PromQL Snippets

**High reject rate**

```promql
sum(rate(oap_rejects_total[5m])) / clamp_min(sum(rate(oap_frames_total[5m])), 1) > 0.005
```

**Decode p95 regression**

```promql
histogram_quantile(0.95, sum(rate(oap_read_frame_seconds_bucket[5m])) by (le)) > 0.02
```

**ACK starvation**

```promql
avg_over_time(oap_ack_window[5m]) == 0 and sum(rate(oap_frames_total[5m])) > 0
```

**Oversize frames present**

```promql
increase(oap_rejects_total{reason="frame_oversize"}[15m]) > 0
```

**Backpressure brown-out**

```promql
avg_over_time(oap_inflight_frames[5m]) / <inflight_capacity> > 0.8
```

**FFI overhead (polyglot hosts) — watch CPU time in readiness exporter**

```promql
rate(process_cpu_seconds_total{job="gateway",component="ffi_ready"}[5m]) > 0.1
```

---
