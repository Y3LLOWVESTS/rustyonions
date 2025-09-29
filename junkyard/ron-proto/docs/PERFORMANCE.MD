
---

# ⚡ PERFORMANCE.md — ron-proto

---

title: Performance & Scaling
status: draft
msrv: 1.80.0
crate_type: lib
last-updated: 2025-09-28
audience: contributors, ops, perf testers
-----------------------------------------

# PERFORMANCE.md

## 0. Purpose

This document defines the **performance profile** of `ron-proto`:

* Library-level throughput for serialization, hashing, and validation.
* Benchmarks & workloads for DTO encode/decode.
* Perf harness & profiling tools.
* Scaling knobs (allocs, chunk size) and bottlenecks (Serde, BLAKE3).
* Regression gates to prevent silent perf drift.

It ties directly into:

* **Scaling Blueprint v1.3.1** (roles, SLOs, runbooks).
* **Omnigate Build Plan** milestones Bronze→Gold.
* **Perfection Gates** (F = perf regressions barred, L = scaling chaos-tested).

**Amnesia Compatibility:** `ron-proto` is stateless by design. All benchmarks and memory targets MUST hold under RAM-only operation (no reliance on OS page cache or disk spill). Any optional buffer pools must gracefully shrink to a low-water mark under memory pressure.

---

## 1. SLOs / Targets (Library)

### Measurement Baseline

Results are recorded for two representative hosts:

* **x86_64-AVX2** (e.g., 8c/16t, 3.5 GHz class)
* **arm64-NEON** (e.g., Apple M2)

All SLOs are enforced as **% change vs. the crate’s frozen baselines** in
`testing/performance/baselines/ron-proto.json` for each host profile.

### Size-Binned Targets (single thread unless noted)

**BLAKE3 (hashing throughput)**

| Input size bin | Target metric                  | Gate (vs. baseline) |
| -------------- | ------------------------------ | ------------------- |
| ≤ 1 KiB        | ops/sec (tiny input regime)    | -10% max regression |
| 64 KiB         | bytes/sec (streaming regime)   | -10% max regression |
| 1 MiB          | bytes/sec (large-block regime) | -10% max regression |

**CBOR (ObjectManifestV2 encode+decode)**

| Manifest shape                        | Target metric          | Gate               |
| ------------------------------------- | ---------------------- | ------------------ |
| 2 chunks (128 KiB object)             | ops/sec; ≤ 3 allocs/op | -10% / +20% allocs |
| 16 chunks (1 MiB object)              | ops/sec; ≤ 3 allocs/op | -10% / +20% allocs |
| 1024 chunks (64 MiB logical manifest) | ops/sec; ≤ 5 allocs/op | -10% / +20% allocs |

**Parallel scaling**

* Multi-threaded encode/decode MUST scale ≥ **0.8× N** up to N=8 worker threads
  on baseline hosts (diminishing returns beyond memory-bandwidth saturation).

**Cold start**

* ron-proto init ≤ **2 ms** on both baselines (no global heavy statics).

**Error budget / correctness (hard floors)**

* 0% acceptance of malformed CBOR or mismatched `ContentId`.
* Chunk size invariants MUST be enforced (64 KiB logical segmentation).

---

## 2. Benchmarks & Harness

* **Micro-bench (Normative):** Criterion (`cargo bench`) for:

  * `ContentId::parse`
  * `ObjectManifestV2` encode/decode
  * `OAP/1` envelope parse/serialize

* **Integration tests:**

  * Use `/tests/vectors/*` to validate throughput at scale (10k frames round-trip).

* **Profiling tools:**

  * `cargo flamegraph` — hotspot in Serde or CBOR.
  * `perf` or `coz` — causal profiling of ContentId parse loops.
  * `heaptrack` — allocations per op.

* **CI Integration:**

  * Nightly perf run; regression gates compared to baselines in `testing/performance/baselines/ron-proto.json`.

---

## 3. Scaling Knobs

* **Concurrency:** Scales by running encode/decode in parallel across cores; no internal locks.
* **Memory:**

  * Buffer pools: prefer `bytes::Bytes` for zero-copy slices.
  * Chunk size = 64 KiB fixed (interop invariant; not tunable here).
* **I/O:** *N/A (crate is lib-only).*
* **Edge/Mobile:**

  * Can disable CBOR canonicalization checks in dev/test builds for faster iteration.

---

## 4. Bottlenecks & Known Limits

* **Serde DAG-CBOR encode/decode:** dominates CPU in manifests with >1k chunks.
* **BLAKE3 hashing:** scales well with SIMD but limited by memory bandwidth; slower for tiny (<1 KiB) inputs due to per-call overhead.
* **Known Limit:** `u32 len` in OAP/1 envelope caps payload < 4 GiB; acceptable (interop invariant).
* **Bronze milestone:** baseline criterion benches green.
* **Gold milestone:** polyglot SDKs match throughput within 10% of Rust baseline.

---

## 5. Regression Gates

* CI must fail if (per host profile, per size bin):

  * BLAKE3 bytes/sec or ops/sec drops >10% vs baseline.
  * CBOR encode+decode ops/sec drops >10%.
  * Allocations/op rises >20%.

* Baselines stored in `testing/performance/baselines/`.

* Escape hatch: waivers allowed if regression traced to upstream Serde or blake3 crate update.

---

## 6. Perf Runbook (Triage)

Steps when perf SLOs are breached:

1. **Check flamegraph:** locate Serde/CBOR hotspots.
2. **Inspect allocations:** run heaptrack; confirm pool usage.
3. **Cross-check BLAKE3:** ensure SIMD path enabled; fallbacks slower.
4. **Re-run Criterion:** compare to baselines.
5. **Stress test:** 1M vector round-trips; confirm deterministic behavior.
6. **Edge case:** test ARM64 (M1/M2) for mobile perf parity.
7. **Tiny vs. big input regression:**

   * If tiny-input hashing regresses but large-block does not:

     * Check instruction path (SIMD enabled?).
     * Inspect function-call overhead vs core loop.
     * Batch tiny inputs in bench harness to separate per-call setup from hash speed.

---

## 7. Acceptance Checklist (DoD)

* [ ] Library SLOs defined and documented.
* [ ] Criterion benches implemented (`contentid`, `manifest_v2`, `oap1_envelope`).
* [ ] Flamegraph/heaptrack traces captured once per release.
* [ ] Scaling knobs documented (buffer pools, concurrency).
* [ ] Regression gates wired into CI.
* [ ] Runbook updated with triage steps.

---

## 8. Appendix

* **Reference SLOs (Scaling Blueprint):**

  * p95 GET <80ms intra-region; <200ms inter-region (service-side; `ron-proto` feeds DTO parse costs into these).
  * Failures <0.1%.

* **Reference workloads:**

  * `/tests/vectors/manifest_roundtrip.cbor` (1 MB object, 16 chunks).
  * `/tests/vectors/frame_echo.cbor` (OAP/1 HELLO).

* **Perfection Gates tie-in:**

  * Gate F = perf regressions barred.
  * Gate L = scaling validated under chaos (simulated via fuzz vectors).

* **History:**

  * 2025-09-28: Initial perf doc drafted; baselines collected on AVX2 host.

---

✅ With this template, `ron-proto` locks down **DTO performance invariants**: serialization/hashing are benchmarked, regressions caught in CI, and scaling behavior is explicit. Drift is barred; SDKs and services can rely on stable perf characteristics.

---
