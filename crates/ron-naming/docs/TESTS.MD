

---

# 🧪 TESTS.md — ron-naming

*Audience: developers, auditors, CI maintainers*
*msrv: 1.80.0 (no async runtime required; Loom generally N/A)*

---

## 0) Purpose

Define the **test contract** for `ron-naming` (Pillar 9 library for schemas, normalization, canonical encodings) and its **offline CLI** `tldctl`.

Covers:

* Unit, integration (CLI & DTO round-trips), property, fuzz, soak (CLI), and performance.
* Explicit coverage goals & **Bronze → Silver → Gold** acceptance gates.
* Invocation commands for devs & CI, plus required corpora/baselines.

---

## 1) Test Taxonomy

### 1.1 Unit Tests

**Scope:** Pure functions and modules (fast, <100ms).
**Key areas:**

* `normalize::*` — ASCII fast-path and Unicode (NFKC + casefold) branches.
* `parse::{json,toml}` — DTO parse/serialize invariants.
* `encode::cbor` — canonical, deterministic bytestreams (no nondeterminism).
* DTO validation (label length ≤ 63 octets; FQDN ≤ 253 octets post-canonicalization).
* Error taxonomy (typed errors; no panics on malformed input).

**Location:** `src/**/*` with `#[cfg(test)]`.

**Run:**

```bash
cargo test -p ron-naming --lib -- --nocapture
```

---

### 1.2 Integration Tests

**Scope:** End-to-end crate surface and CLI behavior (`tests/*.rs`).
**Must include:**

* **API round-trip:** DTO → JSON/TOML → DTO (lossless) and DTO → CBOR → DTO; byte-for-byte canonical CBOR across runs.
* **Normalization invariants:** idempotence (`normalize(normalize(x)) == normalize(x)`), casefold + NFKC correctness on mixed-script corpora.
* **CLI streaming:** `stdin`→`stdout` pipeline correctness on newline-delimited corpora; return codes and stderr JSON on errors.
* **Boundary checks (anti-scope):** no network, no filesystem writes unless explicitly invoked by a test harness.
* **Config knobs that exist for CLI:** `--parallel` behavior is observationally identical to single-thread (order may differ but outputs must be set-equal if order-independent mode is used).

**Run:**

```bash
cargo test -p ron-naming --test '*'
```

---

### 1.3 Property-Based Tests

**Tooling:** `proptest` (preferred) or `quickcheck`.
**Targets & invariants:**

* **Normalization:**

  * *Idempotence:* `N(N(x)) == N(x)`
  * *Casefold stability:* `N(lower(x)) == N(x)` for Latin subset; constrained for full Unicode.
  * *No panics:* any valid UTF-8 string (and selected ill-formed sequences handled as errors) never panic.
* **DTO round-trip:**

  * JSON/TOML/CBOR round-trip equivalence (structural equality).
  * CBOR canonical form stable across process runs.
* **Label constraints:**

  * Reject labels >63 octets post-canonicalization; surface typed error codes.
* **Confusable handling (if present):**

  * Confusables mapping stays deterministic; “safe confusables” test vectors must not drift.

**Run:**

```bash
cargo test -p ron-naming --features proptest -- --nocapture
```

---

### 1.4 Fuzz Tests

**Tooling:** `cargo fuzz` (libFuzzer).
**Targets (minimally):**

* `normalize_fuzz` — arbitrary UTF-8 + selected invalid UTF-8 → must not panic; errors are typed.
* `json_parse_fuzz` — random JSON → parse→encode→parse stability or typed error.
* `toml_parse_fuzz` — random TOML → same invariants.
* `cbor_decode_fuzz` — random CBOR → decoder must not panic; reject gracefully.

**Corpus:**
Seed from `testing/corpora/`:

* `ascii_*.txt`, `unicode_mixed_*.txt`, `long_labels.txt`, `malformed_utf8.bin`, `confusables.txt`

**Acceptance:**
Nightly **≥1h** (Silver) / **≥4h** (Gold) with **zero crashes/UB**.

**Run (examples):**

```bash
cargo fuzz run normalize_fuzz -- -max_total_time=3600
cargo fuzz run json_parse_fuzz -- -max_total_time=3600
```

---

### 1.5 Chaos / Soak Tests (CLI-oriented)

`ron-naming` is not a service, so chaos focuses on **long-running CLI pipelines**:

**Scenarios:**

* 24h **soak**: `cat 10M_lines.txt | tldctl vectorize --stdin` (with/without `--parallel`).
  Validate stable RSS (no leak), steady throughput variance (<±5%), and 0 unexpected exits.
* **Size/Compression bombs:** extremely long labels and max-length FQDNs; ensure bounded memory and typed rejects.
* **Fault injection:** kill/restart `tldctl` mid-pipeline in a shell script; resume at next line (external concern).

**Acceptance:**

* 24h soak: **no leaks**, **no panics**, **no throughput collapse**.

---

### 1.6 Performance / Load Tests

Benchmarks live alongside tests but are **separate** from unit pass/fail:

* **Criterion** microbenches:

  * `bench_normalize_ascii`, `bench_normalize_unicode`
  * `bench_parse_json_small`, `bench_parse_toml_small`
  * `bench_cbor_encode_small`
* **CLI latency/throughput:** hyperfine scripts for single-op and batch via stdin; with and without `--parallel`.
* **Regression gates:** as specified in `PERFORMANCE.md` (CI baseline deltas; fail on >10–15% drift with variance band).

**Run:**

```bash
cargo bench -p ron-naming
hyperfine 'tldctl normalize example.org'
hyperfine --warmup 3 'cat testing/corpora/unicode_mixed_100k.txt | tldctl vectorize --stdin'
```

---

## 2) Coverage & Gates

### Coverage Tooling

* `grcov` (LLVM) **or** `cargo tarpaulin` for line/branch coverage (choose one in CI; grcov recommended on Linux x86_64).

### 2.1 Bronze (MVP)

* Unit + integration tests pass (Linux + macOS).
* Coverage **≥ 70%** lines.
* Fuzz harness builds; smoke fuzz run (≤5 min) executes.

### 2.2 Silver (Useful Substrate)

* Property tests on normalization and DTO round-trip.
* Nightly fuzz **≥ 1h** across all targets; zero crashes.
* Coverage **≥ 85%** lines, **≥ 75%** branches on normalization & parsing modules.
* CLI soak script present (smoke 30 min) and green.

### 2.3 Gold (Ops-Ready)

* Nightly fuzz **≥ 4h**, corpus minimized and tracked in repo.
* 24h CLI soak passes with **no RSS growth** and **<±5%** throughput variance.
* Coverage **≥ 90%** lines overall; critical modules **≥ 95%**.
* Performance regression tracked release-to-release; baselines updated with justification.

---

## 3) Invocation Examples

### 3.1 All tests (unit + integration)

```bash
cargo test -p ron-naming --all-targets -- --nocapture
```

### 3.2 Property tests only

```bash
cargo test -p ron-naming --features proptest --tests prop_ -- --nocapture
```

### 3.3 Fuzz targets

```bash
cargo fuzz run normalize_fuzz -- -max_total_time=60
cargo fuzz run json_parse_fuzz -- -max_total_time=60
```

### 3.4 Benches (Criterion) + CLI perf

```bash
cargo bench -p ron-naming
hyperfine --warmup 3 'tldctl normalize example.org'
hyperfine --warmup 3 'cat testing/corpora/100k.txt | tldctl vectorize --stdin'
```

### 3.5 Soak (example)

```bash
set -euo pipefail
seq 10000000 | awk '{print "name" $1 ".example"}' \
 | ./target/release/tldctl vectorize --stdin --parallel \
 | pv -l > /dev/null
```

---

## 4) Observability Hooks

* **Structured logs** (stderr) on failures with fields: `level`, `module`, `err_kind`, `offset`, `duration_ms`, `corpus_id`.
* **Correlation IDs**: tests may add `corr_id=<uuid>`; emitted on error paths for cross-test triage.
* **Artifacts**: CI uploads:

  * Fuzz crashes/minimized repros (if any).
  * Criterion JSON baselines + hyperfine CSVs.
  * Flamegraphs (on perf deltas).
  * Heaptrack reports (if alloc spike detected).

---

## 5) CI Enforcement

**Jobs (typical GitHub Actions matrix):**

* **Test:** `cargo test --workspace --all-targets`
* **Lint:** `cargo fmt -- --check` and `cargo clippy -- -D warnings`
* **Supply chain:** `cargo deny check advisories bans licenses sources`
* **Coverage:** grcov (Linux x86_64), uploaded as artifact + badge
* **Fuzz (nightly):** `cargo fuzz run ... -max_total_time=3600` (Silver) / `14400` (Gold)
* **Perf (nightly):**

  * `cargo bench` → compare to baselines
  * Hyperfine scripts → compare CSV to baselines
  * Fail thresholds per `PERFORMANCE.md` (10–15% + variance band)

---

## 6) Open Questions (crate-specific)

* **Loom applicability:** N/A (no async or lock-heavy concurrency in the lib). If an internal queue appears later, define a minimal loom harness.
* **Mandatory fuzz set:** `normalize_fuzz`, `json_parse_fuzz`, `toml_parse_fuzz`, `cbor_decode_fuzz`. Add `confusables_fuzz` if confusable logic expands.
* **Perf SLOs:** Use `PERFORMANCE.md` CI baselines as **authoritative** (x86-64-v3 runners); local native numbers are informative only.
* **PQ/verify:** If a `verify` feature lands here (or in a sibling lib), add property/fuzz and benches for hybrid signatures; otherwise N/A.

---

## 7) Repository Layout (tests/perf/fuzz)

```
crates/ron-naming/
├─ src/
│  ├─ normalize/…              # unit tests inline
│  ├─ parse/{json,toml}.rs
│  └─ encode/cbor.rs
├─ tests/
│  ├─ integration_api.rs       # round-trips + invariants
│  ├─ integration_cli.rs       # stdin→stdout, errors
│  └─ prop_normalize.rs        # property tests (proptest)
├─ fuzz/
│  ├─ fuzz_targets/
│  │  ├─ normalize_fuzz.rs
│  │  ├─ json_parse_fuzz.rs
│  │  ├─ toml_parse_fuzz.rs
│  │  └─ cbor_decode_fuzz.rs
│  └─ Cargo.toml
├─ benches/
│  ├─ normalize_bench.rs
│  ├─ parsing_bench.rs
│  └─ encode_bench.rs
├─ testing/
│  ├─ corpora/
│  │  ├─ ascii_100k.txt
│  │  ├─ unicode_mixed_100k.txt
│  │  ├─ long_labels.txt
│  │  └─ malformed_utf8.bin
│  └─ performance/
│     └─ baselines/
│        ├─ criterion/*.json
│        └─ cli/*.csv
```

---
