

---

title: RUNBOOK — ron-naming
owner: Stevan White
msrv: 1.80.0
last-reviewed: 2025-10-06
audience: operators, SRE, auditors

# 🛠️ RUNBOOK — ron-naming

## 0) Purpose

Operational manual for **ron-naming** (Pillar 9 library for schemas/normalization/canonical encodings) and its **offline CLI `tldctl`** used for local validation, normalization, and vector generation.
There is **no online resolution** here (that belongs to `svc-index`).
This runbook satisfies **PERFECTION_GATES**: **K** (Continuous Vigilance) and **L** (Black Swan Economics).

---

## 1) Overview

* **Name:** `ron-naming`
* **Role:** Pillar 9 **library** + **offline CLI** (`tldctl`) for normalization, parsing, canonical encodings, test vectors
* **Criticality Tier:** **2** (supporting) — foundational for correctness; not on the request path
* **Runtime Dependencies (CLI):** none (stdin/stdout only)
* **Build/Dev Dependencies:** Rust toolchain (msrv 1.80.0); standard workspace deps
* **Ports Exposed:** **None** (not a service)
* **Data Flows:** `stdin | file` → `tldctl` → `stdout` (JSON/CBOR/TOML as configured)
* **Version Constraints:** stable **SemVer**; consumers pin `ron-naming >= X.Y` per workspace policy

---

## 2) Startup / Shutdown

### Build & Use (CLI)

```bash
# Build release binary for the CLI
cargo build -p ron-naming --bin tldctl --release

# Normalize a single name
./target/release/tldctl normalize example.org

# Vectorize a corpus from stdin (newline-delimited)
cat testing/corpora/unicode_mixed_100k.txt | ./target/release/tldctl vectorize --stdin

# Parallel batch processing (if built with the `parallel` feature)
cat testing/corpora/unicode_mixed_100k.txt | ./target/release/tldctl vectorize --stdin --parallel
```

**Environment knobs:**

* `RUST_LOG=info|debug|trace` — structured logs to stderr
* `RON_AMNESIA=1` — zeroize transient buffers on exit (defensive hardening)
* `RUSTFLAGS="-C target-cpu=native"` — local perf runs; CI uses x86-64-v3

**Verification (sanity):**

```bash
./target/release/tldctl selftest
# exit code 0 and "selftest_ok" in output indicates a healthy build
```

### Shutdown

* Normal: process exits after finishing stdin; non-interactive
* Long-running pipelines: `Ctrl-C` to terminate cleanly (ensures buffered output is flushed)

---

## 3) Health & Readiness

There are **no /healthz or /readyz endpoints**. Use **self-tests** and smoke commands:

* **Self-test:** `tldctl selftest` → exit code 0 if tables/pipeline load correctly
* **Smoke (ASCII):** `tldctl normalize example.org` → emits canonical form without error
* **Smoke (Unicode):**

  ```bash
  printf "Ｅxample.ｏrg\n" | tldctl vectorize --stdin
  ```

  Expect normalized output; failures indicate Unicode table/normalization issues

---

## 4) Common Failure Modes

| Symptom                                 | Likely Cause                              | Evidence / Where to Look                  | Resolution                                                 | Alert/Action    |
| --------------------------------------- | ----------------------------------------- | ----------------------------------------- | ---------------------------------------------------------- | --------------- |
| Non-zero exit on `selftest`             | Build/runtime mismatch; corrupted tables  | stderr logs (`RUST_LOG=debug`)            | Rebuild release; run `cargo clean`; re-run `selftest`      | Block release   |
| Very slow batch throughput              | Built without native flags or `parallel`  | perf logs; hyperfine baseline regressions | Rebuild with `-C target-cpu=native`; enable `--parallel`   | File perf issue |
| “Invalid label length” errors           | Oversized label (>63 octets)              | stderr structured error                   | Validate inputs; split labels; reject per policy           | N/A             |
| Unicode confusable/invalid codepoints   | Malformed corpus                          | stderr typed errors; proptest failures    | Fix corpus; keep malformed set for fuzz/regression         | N/A             |
| Memory usage spikes on huge single line | Unbounded input line; not newline-delim   | RSS growth; heaptrack                     | Enforce newline-delimited input; pre-chunk source files    | N/A             |
| CI perf gate failure                    | Upstream dep changed; code path regressed | Criterion JSON diff; flamegraph artifacts | Investigate hotspot; file waiver with evidence or rollback | Paging in CI    |

---

## 5) Diagnostics

**Logs (structured, stderr):**

```bash
RUST_LOG=debug ./target/release/tldctl normalize example.org 2>debug.log
grep -E "level=|error=|duration=" debug.log
```

**Perf snapshots:**

```bash
# Microbench (Criterion)
cargo bench -p ron-naming

# CLI latency/throughput (hyperfine)
hyperfine 'tldctl normalize example.org'
hyperfine --warmup 3 'cat testing/corpora/100k.txt | tldctl vectorize --stdin'
```

**Flamegraph & allocations:**

```bash
cargo flamegraph -p ron-naming --bin tldctl -- normalize example.org
heaptrack ./target/release/tldctl -- vectorize --stdin < testing/corpora/100k.txt
```

**Determinism/property:**

```bash
cargo test -p ron-naming --features proptest
```

---

## 6) Recovery Procedures

1. **Build/Config Drift**

   * Symptom: `selftest` fails or unexpected normalization
   * Action: `cargo clean && cargo build -p ron-naming --bin tldctl --release`
   * Verify: `tldctl selftest` → 0

2. **Perf Regression**

   * Symptom: hyperfine/Criterion below baselines
   * Action: run flamegraph + heaptrack; try `--parallel`, bump chunk 128→256 KiB; re-bench
   * If confirmed: open issue with artifacts; consider rollback to last green tag `ron/ron-naming/vX.Y.Z`

3. **Corpus Issues**

   * Symptom: frequent input errors on pipelines
   * Action: validate corpus (newline-delimited; UTF-8); segregate malformed lines; re-run

4. **Toolchain Breakage**

   * Symptom: build fails after toolchain bump
   * Action: pin stable toolchain for msrv; re-sync workspace versions; re-run `cargo-deny` and benches

5. **Amnesia/Security Posture**

   * Symptom: sensitive data remained in memory longer than desired (rare; lab paranoia mode)
   * Action: set `RON_AMNESIA=1`; re-run workload; confirm zeroization hooks executed on exit (log note)

---

## 7) Backup / Restore

* **Statefulness:** None. `ron-naming` and `tldctl` are **stateless**.
* **What to preserve:** test corpora, perf baselines (`testing/performance/baselines/`), and CI artifacts (flamegraphs/CSV).
* **Restore:** re-checkout repo + baselines; rebuild; re-run `selftest` and perf smoke.

---

## 8) Upgrades

1. Land PR with CHANGELOG entry (include perf notes if any).
2. CI must be green on: benches vs baselines, clippy, cargo-deny.
3. Tag `ron/ron-naming/vX.Y.Z`.
4. Rebuild `tldctl`; run `selftest` + two smoke commands (ASCII + Unicode).
5. If downstream consumers (e.g., `svc-index`) rely on DTOs, confirm no breaking changes (SemVer).

---

## 9) Chaos & Black-Swan Testing (Quarterly)

* **Fuzz/Property:** run proptests with confusables/invalid code points.
* **Compression/size bombs:** very long labels and max-length names; ensure graceful reject + stable memory.
* **Throughput soak:** 1M-line batch via `--stdin` (parallel on/off); watch RSS & throughput variance.
* **Outcome:** attach artifacts (logs, flamegraphs) to the quarterly drill report. Gate **J/K/L** must pass.

---

## 10) Scaling Notes

* **Parallelism:** `--parallel` (Rayon) for batch; scales ~linearly by core.
* **Buffers:** try 64→128→256 KiB to trade memory for throughput.
* **I/O:** always prefer streaming (`--stdin`); avoid full-file buffering scripts.
* **Targets:** see PERFORMANCE.md §1 (CI vs local baselines). Treat CI numbers as authoritative for gates.

---

## 11) Security Ops

* **No secrets** in inputs or logs; `tldctl` writes results to stdout, diagnostics to stderr.
* **Amnesia:** enable `RON_AMNESIA=1` for zeroization on exit.
* **PQ posture:** if a `verify` feature is enabled in the future, track its perf & key handling in QUANTUM.md; for now, N/A.
* **Supply chain:** `cargo-deny` must be green; lockfile updates reviewed.

---

## 12) References

* `PERFORMANCE.md` — SLOs, baselines, gates, scripts
* `CONCURRENCY.md` — single-thread default; optional Rayon for batch
* `OBSERVABILITY.md` — what to log/measure for perf triage (CLI context)
* `SECURITY.md` — malformed input handling, zeroization notes
* `IDB.md` — invariants (deterministic normalization; no network)
* Blueprints: Hardening, Concurrency & Aliasing, Scaling, Omnigate

---

## ✅ Perfection Gates Checklist

* [ ] **K (Continuous Vigilance):** selftest + smoke wired in pre-release; logs/artefacts retained
* [ ] **L (Black Swan Economics):** quarterly chaos drill (fuzz, size bombs, soak) passed
* [ ] Perf CI gates green (≤10–15% drift within variance window)
* [ ] No service creep (no ports, no network, no sled)
* [ ] CHANGELOG entry includes any perf deltas and mitigation notes

---
