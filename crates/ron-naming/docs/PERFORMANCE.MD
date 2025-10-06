

---

# ⚡ PERFORMANCE.md — ron-naming

---

title: Performance & Scaling
status: draft
msrv: 1.80.0
crate_type: lib (+ offline CLI `tldctl`)
last-updated: 2025-10-06
audience: contributors, ops, perf testers
-----------------------------------------

# PERFORMANCE.md

## 0. Purpose

This document defines the **performance profile** for `ron-naming` as a **pure library** (schemas, normalization, canonical encodings) and the **offline CLI `tldctl`** used for local validation/vector generation. There is **no online resolution** here; runtime lookups/serving live in `svc-index` (Pillar 9 boundary).

It ties directly into:

* **Scaling Blueprint** guardrails (frame/chunk bounds referenced as system context).
* **Perfection Gates** (Gate F: perf regressions barred; Gate L: scaling validated).
* **IDB invariants**: deterministic outputs, no network/DB side-effects, one normalization path.

---

## 1. SLOs / Targets

### Baseline Environments (to make numbers reproducible)

* **CI Baseline (authoritative for gates):** GitHub-hosted `ubuntu-latest`, 2 vCPU, 7 GB RAM, x86_64, `RUSTFLAGS="-C target-cpu=x86-64-v3"`.
* **Local Reference (informational):** Modern 8-core laptop (e.g., Apple M2/M3 or Intel i7-12700H+), `-C target-cpu=native`.

### Library (single-threaded by default; `parallel` feature uses Rayon for batch ops)

* **Normalization throughput (labels):**

  * ASCII-heavy: ≥ **1.5M labels/sec/core** (local ref), ≥ **500k/sec/core** (CI baseline).
  * Mixed Unicode (NFKC + casefold): ≥ **400k/sec/core** (local ref), ≥ **150k/sec/core** (CI).
* **Parsing/Encoding (small DTOs ≤512 B):**

  * JSON→DTO: ≥ **200k docs/sec/core** (local), ≥ **80k/sec/core** (CI).
  * TOML→DTO: ≥ **80k docs/sec/core** (local), ≥ **30k/sec/core** (CI).
  * Canonical CBOR encode: ≥ **300k ops/sec/core** (local), ≥ **120k/sec/core** (CI).
* **Allocations/op (amortized):**

  * Normalize ASCII: **0** heap allocs (fast path).
  * Normalize Unicode: ≤ **2** short-string allocs/label.
  * JSON→DTO: ≤ **3** transient allocs/doc.
* **Cold init:** crate tables/setup < **5 ms** (no I/O).

### Optional PQ / Verify Feature (only if the crate exposes a `verify` capability)

* **Signature verify throughput (per core):**

  * Ed25519: ≥ **50k verifies/sec** (CI).
  * Hybrid PQ (e.g., Dilithium2+Ed25519): ≥ **10k verifies/sec** (CI).
* **Target deltas:** Hybrid ≤ **+3×** latency vs Ed25519 alone for small payloads.

> If `verify` is not part of `ron-naming`, this subsection is **N/A**; keep the bench harness behind a feature flag so it’s ready if we add verify later.

### CLI (`tldctl`, offline streaming)

* **Latency (single op):** `tldctl normalize example.com` p95 < **2 ms** warm (CI).
* **Throughput (batch):**

  * `--stdin` single-thread: ≥ **450k labels/sec** (local), ≥ **160k/sec** (CI).
  * `--stdin --parallel` (Rayon): ≥ **1.2M/sec** (local, 8 cores), scales ~linearly by core; ≥ **320k/sec** (CI, 2 vCPU).
* **Resource ceilings:**

  * RSS steady < **200 MiB** at 1M-name batch (streaming + bounded buffers).
  * CPU < **85%** of a core (single-thread), scales with threads when `--parallel` used.

**Standards/context (system-wide, not enforced here):**

* OAP/1 max frame **1 MiB**; streaming chunk **64 KiB**. Useful for corpus sizing and DTO micro-bench context.

---

## 2. Benchmarks & Harness

* **Criterion micro-benches (`cargo bench`):**

  * `bench_normalize_ascii`, `bench_normalize_unicode`
  * `bench_parse_json_small`, `bench_parse_toml_small`
  * `bench_cbor_encode_small`
  * `bench_verify_ed25519`, `bench_verify_pq_hybrid` *(behind `verify` feature)*
* **CLI latency/throughput (hyperfine):**

  * `hyperfine 'tldctl normalize example.org'`
  * `hyperfine --warmup 3 'cat testing/corpora/100k.txt | tldctl vectorize --stdin'`
  * `hyperfine --warmup 3 'cat testing/corpora/100k.txt | tldctl vectorize --stdin --parallel'`
* **CPU profiling:** `cargo flamegraph` (hotspots: Unicode tables, TOML parse, CBOR encode).
* **Alloc profiling:** `heaptrack` (Linux) or `valgrind --tool=dhat` (alloc sites).
* **Determinism/property tests:** proptest corpus for idempotence & canonicalization; run alongside benches to catch perf + correctness drift.
* **Chaos/perf blend (CLI):** very long labels, mixed scripts (CJK/RTL), invalid code points/confusables, slow/stdin backpressure.

**CI Integration**

* Nightly perf workflow:

  * Run Criterion; compare JSON to baselines.
  * Run hyperfine batch scripts; append CSV to baselines.
  * Upload flamegraphs on deltas > thresholds.
* Optional: `tokio-console` is N/A (no async hot paths); Rayon parallelism is profiled via flamegraph/coz.

**Perf Flow (visual)**

```mermaid
flowchart TD
  A[Input labels] -->|ASCII fast-path| B[Normalize ≥1.5M/s/core (local)]
  A -->|NFKC+Casefold| C[Normalize ≥400k/s/core (local)]
  B --> D[Alloc ≤2/label (Unicode) / 0 (ASCII)]
  C --> D
  E[CLI batch stdin] -->|single-thread| G[≥450k/s (local)]
  E -->|--parallel (Rayon)| F[≥1.2M/s (local 8c)]
  style D fill:#0b7285,stroke:#083344,color:#fff
```

---

## 3. Scaling Knobs

* **Concurrency:** default single-thread; `parallel` feature enables Rayon in batch CLI paths only.
* **Memory:** bounded reader (64–256 KiB); `SmallVec` for ≤63-byte labels; optional `intern` feature (LRU-capped) for hot TLD forms.
* **I/O:** streaming stdin/stdout; avoid full-file buffering; prefer zero-copy slices for DTO serialization.
* **Build profile:** `-C target-cpu=x86-64-v3` in CI; optional `-C target-cpu=native` locally; thin-LTO toggle for release benches.
* **Amnesia toggle (CLI):** if `RON_AMNESIA=1`, zeroize transient buffers at process exit.

---

## 4. Bottlenecks & Known Limits

* **Unicode normalization dominates** CPU on mixed-script corpora; keep ASCII fast-path hot and consider SIMD table precomputation in Silver.
* **TOML parse is slower** than JSON; prefer JSON for bulk DTO ingest.
* **Line/label size limits:** enforce IDN-style constraints (label ≤ **63 octets**, full name ≤ **253 octets** after canonicalization); reject oversize with typed errors.
* **No network / no sled by design:** anything resembling “resolve” is out-of-scope (belongs in `svc-index`). This preserves CPU-bound, deterministic perf.

**Milestones**

* **Bronze:** targets above met; baselines captured; gates enforced.
* **Silver:** SIMD/arena experiments for normalization; reduced allocs/label.
* **Gold:** large corpus parallel runs, cross-platform baselines (aarch64/x86_64), stabilized variance ≤5%.

---

## 5. Regression Gates

CI fails if (vs previous CI baseline on identical runner; geomean across 5 runs):

* **Normalization p95/op:** ↑ > **10%** (+ an allowed noise band **±5%**).
* **Parse/encode throughput:** ↓ > **10%** (±5% noise).
* **Allocations/op:** ↑ > **15%**.
* **CLI batch throughput:** ↓ > **10%** (±5% noise).

**Baselines**

* Stored under `testing/performance/baselines/ron-naming/`:

  * Criterion JSON snapshots (per-bench).
  * Hyperfine CSV (`cli_batch_throughput.csv`).
  * Example JSON snippet:

    ```json
    {
      "normalize_ascii": { "throughput_ops_per_sec": 520000, "p95_ms": 0.0017 },
      "normalize_unicode": { "throughput_ops_per_sec": 165000, "p95_ms": 0.0056 }
    }
    ```

**Waivers**

* Allowed only with attached flamegraph + heaptrack evidence and a clear upstream cause (e.g., dependency bump). Must include an issue link and a plan to recover.

**Rollback**

* If a regression is confirmed, revert to the last green tag `ron/ron-naming/vX.Y.Z` and re-run perf to re-establish the baseline.

---

## 6. Perf Runbook (Triage)

1. **Confirm corpus** and proportions (ASCII vs Unicode); ensure same file & order.
2. **Flamegraph** micro-benches → confirm hotspots (normalizer, parser, encoder).
3. **Heaptrack** the CLI batch to locate alloc spikes; check SmallVec & interning behavior.
4. **Flip knobs**: `--parallel`, buffer 128→256 KiB, enable/disable `intern`, release+LTO.
5. **Chaos shape**: inject long labels, confusables, invalid code points; verify typed errors and stable throughput.
6. **PQ path (if enabled)**: run `bench_verify_*` and compare deltas to prior.
7. **File an issue** with bench diffs, flamegraph, heaptrack; propose fix & expected gain; link to PR.
8. **Rollback** if needed (see §5).

---

## 7. Acceptance Checklist (DoD)

* [ ] SLOs defined with explicit CI/local baselines.
* [ ] Criterion benches cover normalize (ASCII/Unicode), parse (JSON/TOML), encode (CBOR).
* [ ] Optional PQ verify benches gated behind feature (or marked N/A).
* [ ] Hyperfine scripts for CLI single & batch (parallel + single).
* [ ] Baselines checked in under `testing/performance/baselines/ron-naming/`.
* [ ] CI perf comparison + variance window wired; gates fail on >10–15% drifts.
* [ ] Property tests assert **idempotence & determinism** across perf runs.
* [ ] Perf runbook validated once per milestone.
* [ ] Lib/service boundary re-asserted (no resolve/network/sled here).

---

## 8. Appendix

**Reference workloads**

* `testing/corpora/`:

  * `ascii_100k.txt` (DNS-like, a–z, 0–9, hyphen)
  * `unicode_mixed_100k.txt` (CJK/RTL + Latin)
  * `long_labels.txt` (edge-case max lengths)

**Example scripts**

```bash
# Micro-bench (CI/local)
cargo bench -p ron-naming

# CLI single op
hyperfine 'tldctl normalize example.org'

# CLI batch (single-thread)
hyperfine --warmup 3 'cat testing/corpora/unicode_mixed_100k.txt | tldctl vectorize --stdin'

# CLI batch (parallel)
hyperfine --warmup 3 'cat testing/corpora/unicode_mixed_100k.txt | tldctl vectorize --stdin --parallel'
```

**Perfection Gates**

* **Gate F:** perf regressions barred by CI baselines (with explicit variance band).
* **Gate L:** scaling validated for batch CLI, with flamegraphs and alloc reports attached.

**History**

* Maintain a running log of perf wins/regressions in `CHANGELOG.md` under a “Performance” heading (with links to bench diffs/PRs).

---

