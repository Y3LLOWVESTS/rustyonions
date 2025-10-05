
---

```markdown
# ⚡ PERFORMANCE.md — `ron-auth`

---
title: Performance & Scaling
status: draft
msrv: 1.80.0
crate_type: lib
last-updated: 2025-10-04
audience: contributors, ops, perf testers
---

## 0) Purpose

Defines the **performance profile** for `ron-auth`, a **pure verification library** (no I/O, no background tasks):

- Lib-level SLOs (latency/allocations) + host-parallel throughput guidance.
- Benchmarks & workloads it must sustain.
- Perf harness & profiling tools.
- Scaling knobs, known bottlenecks, and a triage runbook.
- CI regression gates to prevent silent drift.

Ties into **Scaling Blueprint v1.3.1**, **Omnigate Bronze→Gold**, and **Perfection Gates** (Gate **F** = perf regressions barred; Gate **L** = scaling under chaos validated).

> **Amnesia Mode:** The library is stateless by design and holds no keys. Hosts are responsible for key zeroization and any amnesia toggles on the request context.

---

## 1) SLOs / Targets (Library)

### Latency (p95 per verification)
- **Conservative baseline (portable across HW):**  
  p95 ≤ **60 µs + (8 µs × caveats)** for 4 KiB tokens.  
  (E.g., 10 caveats → p95 ≤ 140 µs.)

- **Typical modern x86 (informative note):**  
  Expect **10–20 µs base + ~1–2 µs/caveat** on high-end desktops; we still enforce the conservative SLO above to avoid over-optimism across fleets.

### Allocations / op
- ≤ **2 allocations** steady (decode + minimal workspace), verified by heaptrack.

### Throughput (host-parallel guidance)
- On an **8-core x86_64 workstation** (3.5 GHz class), `--release`, 4 KiB tokens, 10 caveats, saturating request-level parallelism:  
  **≥ 1.0M verifies/sec** sustained.  
- On **CI (ubuntu-latest, 2 vCPU)**: track **relative deltas**, not absolutes; regressions >10% fail.

### Reliability / budgets (host-observed)
- Parse errors ≈ 0 in steady state (spikes indicate client or parser drift).
- Unknown KID ≈ 0 in steady state (spikes indicate rotation drift).

### Resource ceilings / bounds
- `max_token_bytes` default **4096**; validated range **[512, 16384]**.
- `max_caveats` default **64**; validated range **[1, 1024]**.  
These are **MUST** bounds to keep perf predictable and abuse-resistant.

### PQ-hybrid envelope (feature `pq-hybrid`)
- No hard SLO yet; **measure & publish** overhead vs. classical signatures. Target keeping overheads within deployment budgets as the QUANTUM plan matures.

---

## 2) Benchmarks & Harness

**Micro-bench (Criterion):**
- `verify_small_token` (1–3 caveats; cold/warm).
- `verify_many_caveats` (16/32/64 caveats; enforces slope).
- `verify_skewed` (`exp/nbf`, realistic `RequestCtx`).
- `verify_unknown_kid` (constant-time deny path).
- `verify_custom_caveats` (registrar present; unknowns deny).

**PQ benches (`pq-hybrid`):**
- `sig_adapter_ed25519` vs `sig_adapter_dilithium2` — report the delta (not a hard gate…yet).

**Integration load (host wrapper):**
- Wrap `verify()` with Prometheus histograms/counters; drive with `bombardier`/`wrk`. Collect latency distributions and error taxonomy.

**Profiling tools:**
- `cargo flamegraph` for CPU hotspots (decode/parse/MAC chain).
- `tokio-console` (if wrapper is async) to catch stalls around the call boundary.
- `heaptrack` to verify ≤2 allocations/op.
- `hyperfine` for quick end-to-end CLI latency of the wrapper, if any.

_Run locally:_
```

cargo bench -p ron-auth
cargo flamegraph -p ron-auth --bench verify_many_caveats
hyperfine 'cargo bench -p ron-auth --bench verify_small_token'

````

---

## 3) Scaling Knobs

- **Bounds:** tune `max_caveats` and `max_token_bytes` within validated ranges to cap per-op cost.
- **Key provider:** implement `MacKeyProvider` as an **O(1)** KID→handle map; during rotations, atomically swap (current + N previous) to avoid `unknown_kid` spikes.
- **Zero-copy:** decode once; avoid buffer churn in host wrappers (retain ≤2 alloc/op).
- **Parallelism:** scale throughput via host-level concurrency; verifier is `Send + Sync` and re-entrant.
- **PQ toggles:** enable only where needed; always record deltas per algorithm.

---

## 4) Bottlenecks & Known Limits

- **Decode + CBOR parse** (~size-sensitive, but bounded by `max_token_bytes`).
- **Keyed BLAKE3 MAC chain** (O(n_caveats); latency scales linearly with caveats).
- **KID lookup** (must be constant-time hashmap or similar; spikes appear as `unknown_kid`).
- **PQ envelope verify** (optional) — higher CPU cost; track and report.

> **Deliberate choice:** no internal caches in the lib; determinism and short codepaths beat micro-caching. Host may cache at higher layers.

---

## 5) Regression Gates (CI)

CI fails if any hold:

- p95 latency ↑ **>10%** at any tested caveat count, or slope deviates materially from the linear target.
- Host-parallel throughput (on the same runner type) ↓ **>10%**.
- Allocations/op > **2** steady or become unstable.
- Error counters (`parse_error_total`, `unknown_kid_total`) exceed steady thresholds under load.

**Baselines:** commit JSON/CSV snapshots under `testing/performance/baselines/ron-auth/`.  
**Waivers:** allowed only when traced to an upstream dependency with a filed issue and documented CHANGELOG note.

> **Minimal CI guard (example)**  
> Add `.github/workflows/perf.yml`:
>
> ```yaml
> name: perf
> on:
>   workflow_dispatch: {}
>   schedule: [{cron: "0 8 * * 1"}]  # weekly
> jobs:
>   bench:
>     runs-on: ubuntu-latest
>     steps:
>       - uses: actions/checkout@v4
>       - uses: dtolnay/rust-toolchain@stable
>       - run: cargo bench -p ron-auth
>       - name: Compare against baselines
>         run: bash scripts/perf_guard.sh target/criterion testing/performance/baselines/ron-auth || exit 1
> ```
>
> And `scripts/perf_guard.sh` compares current Criterion estimates to baselines and exits non-zero on >10% regressions.

---

## 6) Perf Runbook (Triage)

1. **Bounds sanity:** Ensure `max_token_bytes` / `max_caveats` match intended tier. Tighten if abuse indicated.
2. **Metrics first:** Check latency histogram p95, `parse_error_total`, `unknown_kid_total`, `deny_total{reason=…}`.
3. **Flamegraph:** Attribute time among decode/parse, MAC chain, KID lookup, and (if enabled) `SigAdapter` verify.
4. **Key provider audit:** Confirm O(1) lookups and rotation window (current + N prev). Fix drift if `unknown_kid` spikes.
5. **Allocator churn:** Run heaptrack; keep ≤2 alloc/op. Hunt stray copies in wrappers.
6. **PQ toggle pivot:** If recently enabled and deltas breach budget, disable or reduce surface area; file QUANTUM follow-up.
7. **Chaos replay:** Re-run with compression/jitter disabled; verify slope stays linear with caveats.

---

## 7) Acceptance Checklist (DoD)

- [ ] SLOs captured (p95 ≤ 60 µs + 8 µs×caveats; ≤2 alloc/op; host-parallel ≥ 1.0M ops/s on 8-core).  
- [ ] Criterion benches run locally + CI; baselines stored and compared.  
- [ ] Flamegraph and heaptrack traces collected at least once per minor.  
- [ ] Scaling knobs documented (bounds, KID map, PQ features).  
- [ ] Regression gates wired into CI with failure thresholds.  
- [ ] Runbook updated and linked from alerts.

---

## 8) Appendix

### A. Hardware Baselines (fingerprint & expectations)

| Tier | CPU example | Cores | Note | Enforcement |
|----|----|----:|----|----|
| CI | `ubuntu-latest` (2 vCPU) | 2 | Relative deltas only | ✔ |
| Dev | Ryzen 7 / i7 (3.5 GHz) | 8 | ≥ 1.0M ops/s host-parallel | ✔ |
| Edge | Apple M-class / ARM | 4 | Track & report (no hard SLO) | ✔ |

Capture runner fingerprint (CPU model, rustc version, flags) alongside baseline artifacts.

### B. Hot-Path Sketch (Mermaid)

```mermaid
flowchart LR
  A[Base64URL decode] --> B[CBOR parse]
  B --> C[MAC chain (keyed BLAKE3) per caveat]
  C --> D[Evaluate caveats (exp/nbf/path/...)]
  D --> E[Decision: Allow / Deny{reason}]
````

### C. PQ posture

* Features gated (`pq-hybrid`); interop intact; report per-algorithm overheads and keep within deployment budgets as QUANTUM milestones progress.

### D. History

* Record notable regressions + fixes; include perf notes in CHANGELOG even when the public API is unchanged.

```

---
