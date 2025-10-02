

```markdown
# ⚡ PERFORMANCE.md — `ron-transport`

---
title: Performance & Scaling
status: draft
msrv: 1.80.0
crate_type: lib
last-updated: 2025-09-29
owner: Stevan White
concerns: [PERF, RES, SEC]
audience: contributors, ops, perf testers
---

## 0) Purpose

Defines the **performance profile** for `ron-transport`, the library that carries **OAP/1** frames over **TLS/TCP**, **Tor v3 (Arti)**, and **QUIC**:

- Lib-level throughput/latency targets and **service-level SLOs** for hosts.
- Benchmarks & workloads it must sustain.
- Harness & profiling tools (Criterion, flamegraph, tokio-console, heaptrack).
- Scaling knobs, known bottlenecks, and a triage runbook.
- **Regression gates** to prevent silent perf drift.

Ties into:
- **Scaling Blueprint v1.3.1** (roles, SLOs, runbooks)  
- **Hardening Blueprint** (bounded decompression, frame caps)  
- **Perfection Gates** (F = perf regressions barred, L = scaling chaos-tested)

---

## 1) SLOs / Targets

> `ron-transport` is a **lib**; SLOs below are what **host services** should achieve when using this crate with OAP/1 defaults (frame cap = **1_048_576** bytes; streaming chunk ≈ **65_536** bytes). Results are normalized per core and per node. Keep strict serde & caps upstream to avoid variability.

### 1.1 Reference nodes
- **Ref-A (dev):** 8 vCPU / 16 GiB / NVMe, x86-64  
- **Ref-B (prod):** 16 vCPU / 32 GiB / NVMe, x86-64

### 1.2 Latency (service-facing, end-to-end over transport)

| Path | SLO p95 (Ref-A) | SLO p95 (Ref-B) | SLO p99 (both) | Notes |
|---|---:|---:|---:|---|
| **TLS/TCP, resumed** (16 KiB) | ≤ **12 ms** | ≤ **8 ms** | ≤ **25 ms** | Enable session tickets; OCSP stapling. |
| **TLS/TCP, cold** (handshake) | ≤ **80 ms** | ≤ **60 ms** | ≤ **150 ms** | Cold path; prefer resumption. |
| **QUIC, 1-RTT** (16 KiB) | ≤ **10 ms** | ≤ **7 ms** | ≤ **20 ms** | 0-RTT only if replay-safe. |
| **Tor v3** (HELLO) | ≤ **900 ms** | ≤ **700 ms** | ≤ **1500 ms** | Gate on bootstrap & HS publish. |
| **Inter-region TLS** (16 KiB) | ≤ **60–120 ms** | ≤ **40–90 ms** | ≤ **200 ms** | WAN dependent; QUIC on lossy paths. |

### 1.3 Throughput (node-level, steady state, non-compressed)

| Workload | Ref-A Target | Ref-B Target | Notes |
|---|---:|---:|---|
| Small frames (1–4 KiB), TLS | **≥ 15k fps** | **≥ 30k fps** | fps = frames/s (REQ→RESP). |
| 64 KiB streaming, TLS | **≥ 2.0 Gb/s** | **≥ 3.5 Gb/s** | Coalesce writes; use `Bytes`. |
| QUIC, 64 KiB streaming | **≥ 2.2 Gb/s** | **≥ 3.8 Gb/s** | Many streams; avoid app HOL. |
| Tor v3, small frames | **≥ 1.2k fps** | **≥ 2.0k fps** | Network dependent. |

### 1.4 Error budget / resource ceilings

- **Failures (5xx transport)** < **0.1%**; **429/503** (quota/backpressure) < **1%**.  
- **Bus overflow** (host) < **0.01%**.  
- **CPU** < **75%** per core at target load; **Mem** steady < **1.5 GiB** per node (transport state).  
- **FDs** < **60%** of soft limit.

---

## 2) Benchmarks & Harness

### 2.1 Micro-bench (Criterion)

Targets hot paths: frame encode/decode, header parse, bounded inflate guard, copy-free write.

```

cargo bench -p ron-transport

```

Suggested benches:
- `oap_frame_encode_16k`
- `oap_frame_decode_16k`
- `oap_header_parse`
- `bounded_inflate_guard_1mib`
- `write_coalesce_64k_chunks`

### 2.2 Integration load (TLS/QUIC/Tor)

```

cargo run -p transport-bench --bin tls_echo_server -- --listen 0.0.0.0:8443
cargo run -p transport-bench --bin tls_echo_client -- --target [https://127.0.0.1:8443](https://127.0.0.1:8443) --conns 200 --rps 15000 --payload 4096

cargo run -p transport-bench --bin quic_echo_server -- --listen 0.0.0.0:8443
cargo run -p transport-bench --bin quic_echo_client -- --target quic://127.0.0.1:8443 --conns 200 --streams 512 --payload 65536

cargo run -p transport-bench --bin tor_echo_server --features arti -- --hs 127.0.0.1:8443
cargo run -p transport-bench --bin tor_echo_client --features arti -- --onion abc...onion:443 --conns 64 --rps 1200 --payload 2048

```

External tools (optional):
```

bombardier -c 200 -l -m POST -d 60s [https://127.0.0.1:8443/echo](https://127.0.0.1:8443/echo)
hyperfine 'cargo run -q -p transport-bench -- --payload 16384 --rps 1000'

```

### 2.3 Profiling

Flamegraph:
```

cargo install flamegraph
cargo flamegraph -p ron-transport --bench oap_frame_decode_16k

```

Async stalls (host):
```

RUSTFLAGS="--cfg tokio_unstable" 
RUST_LOG=debug 
TOKIO_CONSOLE_BIND=127.0.0.1:6669 
cargo run -p svc-gateway

```

**Memory profiling (required):**
```

heaptrack cargo bench -p ron-transport

# open the result:

heaptrack_gui heaptrack.cargo-bench.*.zst

````
Fallbacks: `valgrind --tool=massif`, or `dhat-rs` for allocation hot paths.

### 2.4 Chaos/perf blend

- **Loss/latency injection:** `tc netem` for 1–5% loss, 50–150 ms RTT.  
- **Slow-loris:** many idle half-open connections.  
- **Compression bombs:** verify inflate bound rejects > 8× frame cap with **413** (no OOM).

---

## 3) Scaling Knobs

| Area | Knob | Default/Canon | Guidance |
|---|---|---|---|
| **Concurrency** | Accept semaphore | 2× CPU cores | Prevent accept storms; smooth handshakes. |
|  | In-flight frame permits | 64 | Prefer backpressure over buffering. |
| **Buffers** | Write coalescing | 64 KiB | Align to streaming chunk; `bytes::Bytes`/`BytesMut`. |
|  | Read buffer | 64–128 KiB | Avoid tiny reads; match BDP. |
| **I/O** | TCP_NODELAY | on | Low latency for small frames. |
|  | SO_SNDBUF/SO_RCVBUF | 512 KiB | Increase for high-BDP (WAN/QUIC). |
| **TLS** | Session resumption | on | Tickets (rotate daily); OCSP stapling on. |
|  | Cipher suites | TLS 1.3 AEADs | Keep 1.3-only; short chains. |
| **QUIC** | Max streams | 512 (Ref-A) / 1024 (Ref-B) | Many small streams > one big; autotune windows. |
|  | 0-RTT | off (default) | Enable only for replay-safe ops. |
| **Tor** | Bootstrap timeout | 60–120 s | Gate readiness; expose progress metric. |
| **Compression** | Inflate bound | ≤ 8× frame cap | Reject with **413** if exceeded; never speculative inflate. |

*Feature flags:* QUIC knobs apply with `cfg(feature="quic")`; Tor knobs with `cfg(feature="arti")`; compression knobs only if enabled.

---

## 4) Bottlenecks & Known Limits

- **TLS handshake CPU**: cert verify & key exchange spikes; rely on **resumption** and short chains.  
- **Small write amplification**: many <4 KiB writes → syscalls; **coalesce to 64 KiB**.  
- **Allocation churn**: avoid `Vec<u8>` grow/shrink in hot paths; use `Bytes`/small slabs.  
- **QUIC loss sensitivity**: add **concurrent streams** on lossy links to avoid app HOL.  
- **Tor bootstrap/publish**: slow on first run; always **readiness-gate**.  
- **Compression bombs**: attacker-controlled `COMP`; **bound inflate before allocate**.

**Milestones**
- **Bronze**: Meets Ref-A targets locally; no regressions.  
- **Silver**: Meets Ref-B; chaos (loss 1–3%, 100 ms RTT) passes.  
- **Gold**: Mixed TLS/QUIC/Tor, 24 h soak, stable CPU/mem, p99 within SLO.

---

## 5) Regression Gates (CI)

Compare **median of 3 runs** to baseline; fail if thresholds breach:

| Metric | Gate |
|---|---|
| p95 small-frame latency (TLS) | +10% |
| p99 small-frame latency (TLS) | +10% |
| p95 64 KiB streaming latency | +10% |
| Node throughput (fps / Gb/s) | −10% |
| CPU @ target RPS | +15% |
| Mem steady | +15% |

Baselines live in `testing/performance/baselines/ron-transport.json` (validated by `perf-baseline.schema.json`).

Example baseline:
```json
{
  "ref": "Ref-A",
  "tls_small_p95_ms": 12.0,
  "tls_small_p99_ms": 25.0,
  "tls_stream_64k_gbps": 2.0,
  "quic_stream_64k_gbps": 2.2,
  "cpu_pct_at_target": 70.0,
  "mem_gib": 1.2
}
````

Comparison:

```
cargo run -p perf-ci --bin check -- \
  --crate ron-transport \
  --baselines testing/performance/baselines/ron-transport.json \
  --schema testing/performance/baselines/perf-baseline.schema.json \
  --report testing/performance/reports/ron-transport-$(date +%F).json \
  --gate +10p95,+10p99,-10throughput,+15cpu,+15mem --runs 3 --stat median
```

*Noise note:* On noisy runners widen gates by +5% temporarily with a waiver PR comment referencing this section.

---

## 6) Perf Runbook (Triage)

1. **Reproduce quickly**

   ```
   cargo run -p transport-bench --bin tls_echo_server -- --listen 0.0.0.0:8443
   cargo run -p transport-bench --bin tls_echo_client -- --target https://127.0.0.1:8443 --conns 200 --rps 15000 --payload 4096
   ```
2. **Flamegraph**: inspect `rustls` hot spots, memcpy, allocs.

   ```
   cargo flamegraph -p ron-transport --bench oap_frame_decode_16k
   ```
3. **tokio-console (host)**: find stalls/backpressure, long polls.
4. **Metrics** (host): `*_latency_seconds`, `rejected_total{reason}`, inflight; watch 413/429/503 bursts.
5. **Knob sweep**: increase write coalesce (64→128 KiB), accept semaphore, enable TLS resumption, bump QUIC streams.
6. **Chaos off**: disable compression/jitter; re-measure; re-enable progressively.
7. **Network check**: `tc -s qdisc`, `ss -tinap`, MTU/path; try QUIC on lossy links.
8. **Memory check**: `heaptrack cargo bench -p ron-transport` → open in GUI; verify steady buffer reuse, no growth over 24 h soak.
9. **Tor readiness**: confirm `tor_bootstrap_progress` and HS publish metrics hit 100% before load.
10. **Document**: append findings in **History** (§8).

---

## 7) Acceptance Checklist (DoD)

* [ ] SLOs set for TLS/QUIC/Tor, Ref-A/Ref-B (p95 **and** p99).
* [ ] Criterion micro-benches land with stable IDs.
* [ ] Integration harness runs locally (TLS) and in CI (TLS/QUIC; Tor behind feature).
* [ ] Flamegraph, tokio-console, and **heaptrack** captures checked in at least once.
* [ ] Scaling knobs documented & wired to host configs.
* [ ] Regression gates active with **schema-validated** baselines; median-of-3 policy.
* [ ] 24 h soak (Silver) passes without leak or drift; chaos cases captured.
* [ ] Grafana dashboards updated (`dashboards/transport.json`: p95/p99, fps, 413/429/503, CPU/mem).

---

## 8) Appendix

**Reference SLOs (Scaling Blueprint):**

* p95 GET < **80 ms** intra-region; < **200 ms** inter-region.
* Failure rate < **0.1%**; RF(observed) ≥ RF(target).

**Reference workloads:**

* `tls_echo_{server,client}` — small frame RTTs and throughput.
* `quic_echo_{server,client}` — multi-stream streaming.
* Soak test 24 h with mixed small/64 KiB frames at 60–70% CPU.

**Perfection Gates tie-in:**

* **Gate F** — perf regressions barred by §5 CI gates.
* **Gate L** — scaling validated under chaos (loss/latency/slow-loris).

**History:**

* 2025-Q4: Coalescing from 8 KiB→64 KiB improved TLS small-frame p95 16→11 ms on Ref-A.
* 2025-Q4: Session tickets enabled; cold handshake p95 −30% on Ref-B.

````

---

## New file: `testing/performance/baselines/perf-baseline.schema.json`

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "ron-transport perf baseline",
  "type": "object",
  "required": [
    "ref",
    "tls_small_p95_ms",
    "tls_small_p99_ms",
    "tls_stream_64k_gbps",
    "quic_stream_64k_gbps",
    "cpu_pct_at_target",
    "mem_gib"
  ],
  "properties": {
    "ref": { "type": "string", "enum": ["Ref-A", "Ref-B"] },
    "tls_small_p95_ms": { "type": "number", "minimum": 0 },
    "tls_small_p99_ms": { "type": "number", "minimum": 0 },
    "tls_stream_64k_gbps": { "type": "number", "minimum": 0 },
    "quic_stream_64k_gbps": { "type": "number", "minimum": 0 },
    "cpu_pct_at_target": { "type": "number", "minimum": 0, "maximum": 100 },
    "mem_gib": { "type": "number", "minimum": 0 }
  },
  "additionalProperties": true
}
````

---

