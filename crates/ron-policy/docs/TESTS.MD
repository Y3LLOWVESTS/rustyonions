

---

# 🧪 TESTS.md — ron-policy

*Audience: developers, auditors, CI maintainers*
*msrv: 1.80.0 (Tokio/loom compatible via consumer tests)*

---

## 0) Purpose

Define the **testing contract** for `ron-policy`:

* Unit & integration tests for the **validator** and **evaluator**.
* Property tests for **determinism**, **idempotency**, and **tighten-only** semantics.
* Fuzz tests for **bundle parsing** and **evaluation** (structure-aware).
* Performance benches (Criterion) with **regression gates**.
* Clear Bronze→Silver→Gold acceptance thresholds and CI wiring.

`ron-policy` is **pure** (no I/O, no global state). Tests must keep it that way.

---

## 1) Test Taxonomy

### 1.1 Unit Tests (fast, <100ms)

**Scope:** Functions & modules in isolation.

Covers:

* `validate_bundle(...)` — schema/semantics; `deny_unknown_fields` enforced.
* `build_indices(...)` — precomputation (prefix/trie) produces bounded candidate sets.
* `evaluate(bundle, ctx, clock)` — allow/deny/obligations; zero-alloc allow path where possible.
* Reason rendering — enum→`&'static str`; no heap churn.
* Guardrails — **no panics on valid input**, **no I/O** on hot path.

**Location:** `src/**/*` under `#[cfg(test)]`

Run:

```bash
cargo test -p ron-policy --lib
```

---

### 1.2 Integration Tests (end-to-end lib surface)

**Scope:** End-to-end behavior through public API in `tests/*.rs`.

Must include:

* **Golden vectors**: `(bundle.json, ctx.json) -> Decision` fixtures under `testing/vectors/ron-policy/v1/`.
* **Snapshot semantics**: parse→validate→precompute→evaluate; re-loading the same bundle is idempotent.
* **Backward/Forward compatibility**: known-good bundles from previous minor versions fail **only** when intentionally tightened (e.g., new required field); otherwise still evaluate to the same `Decision`.
* **Tighten-only caps**: inputs that exceed configured caps are **rejected at validation** (never relaxed in tests).

Run:

```bash
cargo test -p ron-policy --test '*'
```

---

### 1.3 Property-Based Tests

**Tooling:** `proptest` (preferred) or `quickcheck`.

**Invariants:**

* **Determinism:** for any `(bundle, ctx, clock)`, repeated calls yield **identical** `Decision`.
  *Property*: `evaluate(b,c,k) == evaluate(b,c,k)`
* **Idempotency of normalize/validate:** `normalize(validate(bundle))` stabilizes in one pass.
  *Property*: `normalize(normalize(b)) == normalize(b)`
* **Tighten-only monotonicity:** adding a rule that narrows scope **cannot** cause a previously denied request to become allowed.
  *Property*: `denies(b,ctx) => denies(tighten(b),ctx)` (monotone in the deny direction)
* **Candidate-set boundedness:** narrowing route classes **does not increase** the candidate rule count chosen by indices.
  *Property*: `|candidates(tighten(b),ctx)| ≤ |candidates(b,ctx)|`
* **No panics:** all generated inputs must not trigger panics; errors are returned as typed results.

Run:

```bash
cargo test -p ron-policy --features proptest -- --nocapture
```

*Optional knobs for reproducibility:*
`PROPTEST_CASES=1024 PROPTEST_MAX_SHRINK_ITERS=100`

---

### 1.4 Fuzz Tests (structure-aware)

**Tooling:** `cargo fuzz` with `libFuzzer`.

Targets (under `fuzz/`):

* `fuzz_bundle_parse`: bytes → serde decode → `validate_bundle`; rejects on unknown fields; must not OOM or hang.
* `fuzz_context_decode`: adversarial contexts; evaluator must never panic.
* `fuzz_evaluate`: generate `(bundle, ctx)` with the `arbitrary` crate for DTOs → `validate` → `build_indices` → `evaluate`; enforce per-iteration **cost caps** (rule count, depth, string lengths).

Corpus:

* Seed with canonical vectors + minimal counter-examples from bugs.
* Minimize regularly; CI uploads new discoveries.

Acceptance:

* CI PR job: **1h** per target, zero crashes.
* Nightly: **≥4h** per target, zero crashes; new inputs auto-minimized and stored.

Run examples:

```bash
cargo fuzz run fuzz_bundle_parse -- -max_total_time=3600
cargo fuzz run fuzz_evaluate -- -max_total_time=3600
```

---

### 1.5 Chaos/Soak Tests (delegated to consumers)

`ron-policy` is a library; chaos pertains to **services** embedding it.

Consumers (e.g., `svc-gateway`) must run:

* **Reload churn** (hot reload every N seconds) → no readiness flaps, decisions stable.
* **Latency injection** around policy spans → p95 eval remains sub-ms.
* **Long soak (≥24h)** → zero memory/file descriptor leaks attributable to policy evaluation.

---

### 1.6 Performance/Load Tests (library-level)

**Tooling:** `criterion`.

Benches (under `benches/`):

* `hot_allow_500` — 500 rules, allow path, target p95 **< 1 ms**
* `hot_deny_500` — deny with reason formatting, p95 **< 1 ms**
* `hot_mixed_1k` — 1,000 rules, mixed allow/deny, p95 **< 1 ms**
* `pathological_caps` — near configured caps; remains within budget or **fails validation** fast

Run:

```bash
cargo bench -p ron-policy
```

Artifacts:
`testing/performance/baselines/ron-policy/msrv-1.80.0/v{SEMVER}/*.json`

---

## 2) Coverage & Acceptance Gates

### 2.1 Bronze (MVP)

* Unit + integration tests **green**.
* Code coverage (lines/branches) **≥ 75%** (Linux) via tarpaulin or grcov.
* Fuzz harnesses **build** (no runs required).

### 2.2 Silver (Useful Substrate)

* Property tests included (determinism, idempotency, tighten-only, bounded candidates).
* CI fuzz run **≥ 1h** per target on PRs; **0** crashes.
* Coverage **≥ 85%**.
* Criterion baselines **stored**; perf job compares to last release.

### 2.3 Gold (Ops-Ready)

* Nightly fuzz **≥ 4h** per target; **0** crashes; new corpus auto-minimized.
* 24h soak in at least one **consumer** shows no leaks; deny/latency dashboards stable.
* Coverage **≥ 90%** (excluding generated/serde glue).
* Perf gates: p95 eval **< 1 ms** (≤1k rule class) and **no >10% regression** vs baseline.

---

## 3) Invocation Examples

### 3.1 All Tests (lib targets)

```bash
cargo test -p ron-policy --all-targets -- --nocapture
```

### 3.2 Property Tests (more cases)

```bash
PROPTEST_CASES=2048 cargo test -p ron-policy --features proptest -- tests::prop_*
```

### 3.3 Fuzz Targets

```bash
cargo fuzz run fuzz_bundle_parse -- -max_total_time=120
cargo fuzz run fuzz_evaluate -- -max_total_time=120
```

### 3.4 Benchmarks

```bash
cargo bench -p ron-policy
```

### 3.5 Deterministic Repro (seeded)

```bash
RUSTFLAGS='-C debuginfo=1' PROPTEST_CASES=1 PROPTEST_SEED=0xDEADBEEF cargo test -p ron-policy -- tests::prop_determinism
```

---

## 4) Observability Hooks

* Tests log **structured events** on failure (enable via `RUST_LOG=debug` and `test-log` or `tracing-test`).
* Each integration test includes a **corr_id** (UUID) in log context to correlate across consumer test runs.
* Criterion exports JSON; CI uploads to `testing/performance/baselines/` and compares p95/p99.

---

## 5) CI Enforcement

**Recommended GitHub Actions matrix**

* Toolchains: `msrv=1.80.0`, `stable`.
* OS: ubuntu-latest (coverage), macOS (sanity).
* Features: default, `policy_smallvec` (if present).

**Jobs**

* **lint:** `cargo fmt -- --check` + `cargo clippy -- -D warnings`
* **unit+integration:** `cargo test --workspace --all-targets`
* **coverage (Linux):** tarpaulin or grcov → fail if `< threshold`
* **fuzz (PR):** 1h per target (`fuzz_bundle_parse`, `fuzz_evaluate`)
* **fuzz (nightly):** 4h per target + upload minimized corpus
* **perf:** `cargo bench` → compare Criterion JSON to baseline; fail on >10% p95 regression
* **security hygiene:** `cargo deny check advisories licenses bans`

---

## 6) Crate-Specific Answers (filled)

**Which invariants are loom-checked?**

* None within `ron-policy` (no async/state). **Loom** is required in **consumer crates** for the atomic snapshot-swap path that loads `ron-policy` bundles.

**Which fuzz targets are mandatory?**

* `fuzz_bundle_parse` and `fuzz_evaluate` are **mandatory**. `fuzz_context_decode` is **recommended**.

**What SLOs are measured in perf tests?**

* p95 **evaluation latency < 1 ms** for ≤1k candidate rules in a route class.
* Zero-alloc on allow path (tracked via heaptrack samples); deny path allocations bounded and stable.
* No >10% regression in ops/s or p95 vs. baselines.

---

## 7) File & Directory Layout (testing assets)

```
ron-policy/
├─ benches/
│  ├─ hot_allow_500.rs
│  ├─ hot_deny_500.rs
│  ├─ hot_mixed_1k.rs
│  └─ pathological_caps.rs
├─ tests/
│  ├─ golden_vectors.rs
│  ├─ integration_api.rs
│  ├─ props.rs
│  └─ compat_semver.rs
├─ fuzz/
│  ├─ fuzz_targets/
│  │  ├─ fuzz_bundle_parse.rs
│  │  ├─ fuzz_context_decode.rs
│  │  └─ fuzz_evaluate.rs
│  └─ dictionaries/
│     └─ policy.dict
└─ testing/
   ├─ performance/
   │  └─ baselines/ron-policy/msrv-1.80.0/v{SEMVER}/...
   └─ vectors/ron-policy/v1/
      ├─ allow_basic.json
      ├─ deny_scoped.json
      ├─ mixed_1k.json
      └─ ctx_samples/*.json
```

---

## 8) Golden Vector Contract

Each vector contains:

* `bundle` (canonical JSON)
* A list of `contexts` with expected `Decision` (allow/deny, reason, obligations)
* Optional `notes` (e.g., “tests tighten-only monotonicity”)

Golden tests **must not change** post-release except to add new vectors. If behavior must change, add a **new versioned vector** and cover both until the deprecation window expires.

---

## 9) Reviewer Checklist (per PR)

* [ ] Unit + integration green; no `.unwrap()` in library codepaths touched.
* [ ] New rules/fields include **golden vectors**.
* [ ] Property tests adjusted if semantics evolved (e.g., tightening monotonicity rules).
* [ ] Fuzz corpus updated/minimized; no new OOM/timeouts.
* [ ] Criterion baselines updated or waiver justified.
* [ ] Coverage meets gate (≥ 85% Silver, ≥ 90% Gold).

---

✅ With this contract, `ron-policy` stays deterministic, panic-free, and sub-ms under load—while CI guards prevent silent perf or semantics drift.
