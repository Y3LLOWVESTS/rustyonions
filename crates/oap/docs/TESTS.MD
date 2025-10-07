

---

# 🧪 TESTS.md — `oap`

*Audience: developers, auditors, CI maintainers*
*msrv: 1.80.0 (Tokio/loom compatible — but loom is mostly N/A for this crate)*

---

## 0) Purpose

Define the **test contract** for `oap`:

* Unit, integration, property, fuzz (mandatory), chaos/soak (via hosts), and performance.
* Explicit coverage goals & Bronze→Silver→Gold acceptance gates.
* Canonical commands for local dev & CI.

**Scope reminder:** `oap` is a **pure protocol library** (no sockets/FS/DB), with fixed OAP/1 constants: **max frame = 1 MiB**; envelope set = `HELLO|START|DATA|ACK|END|ERROR`. Hosts export metrics and readiness using `oap_*` instruments.   

---

## 1) Test Taxonomy

### 1.1 Unit Tests (fast, <100 ms)

**Focus:** pure parsing/encoding helpers, header validation, typed errors, and config validation.

* **Must cover (non-exhaustive):**

  * Length-first refusal: reject `len > 1 MiB` before any payload allocation. 
  * Strict headers on (default): unknown/duplicate/malformed → typed reject. 
  * `ParserConfig::validate` bounds for `ack_window_frames ∈ [1,1024]`. 
  * PQ HELLO flags are **opaque** when allowed; rejected at init if policy forbids.  

* **Run:**

  ```bash
  cargo test -p oap --lib
  ```

### 1.2 Integration Tests (`tests/*.rs`)

**Focus:** crate surface (`Parser`, `Writer`, `Envelope`, `OapError`), canonical vectors, behavior under split inputs, and metric calls via a test handle.

* **Must include:**

  * **Envelope round-trips** for `HELLO→START→(DATA×N)→END`, `ACK` sequences.
  * **NeedMore semantics:** truncated inputs at every boundary return `NeedMore`, never corrupt state. 
  * **Config reload semantics (host-style):** constructing a new `ParserConfig` and swap (atomic, no `.await`). 
  * **Metrics mapping:** increments `oap_frames_total{kind}` / `oap_rejects_total{reason}` using a fake metrics facade. 

* **Run:**

  ```bash
  cargo test -p oap --test '*'
  ```

### 1.3 Property-Based Tests

**Tooling:** `proptest` (preferred) or `quickcheck`.

* **Invariants to encode:**

  * **Round-trip**: `encode(decode(bytes)) == bytes` for valid frames; rejects do not panic.
  * **Deterministic rejects:** malformed inputs always map to canonical reasons: `frame_oversize|unknown_envelope|header_malformed|credit_violation|missing_obj|truncated`. 
  * **Idempotency:** repeated `NeedMore` calls with added bytes eventually yield one accept/reject; never multiple advances. 
  * **ACK algebra:** `ack_up_to` is monotone; credit violations reject. (Host timing is out of scope; algebra only.) 

### 1.4 Fuzz Tests (mandatory)

**Tooling:** `cargo fuzz` with libFuzzer targets; sanitizers enabled on nightly job.

* **Targets (suggested):**

  * `parser_fuzz` — bytes → `decode()`; assert no UB/panic; check canonical rejects.
  * `header_fuzz` — header-only corpus; stresses strict header rules & oversize refusal. 
  * `ack_fuzz` — sequences mixing `ACK`/`DATA` enforcing monotone `ack_up_to`.

* **Corpus:**

  * Seed with **canonical vectors** and **recorded rejects** from CI/hosts (minimized). 
  * Include oversize frames, unknown envelopes, duplicate headers, truncated at each byte, missing `obj` when required. 

* **Acceptance:** **≥4h** nightly fuzz run, **zero crashes**; CI keeps minimized findings as regression corpus.

* **Run examples:**

  ```bash
  cargo fuzz run parser_fuzz -- -max_total_time=60
  cargo fuzz run header_fuzz -- -runs=500000
  ```

### 1.5 Chaos/Soak Tests (host-level)

`oap` is a lib; chaos/soak run **in hosts** (gateway/overlay) embedding `oap`.

* Inject: truncated streams, oversize floods, ACK starvation, extreme fragmentation.
* **Acceptance:** 24h soak, zero leaks, `/readyz` browns out before collapse; alerts fire per SLOs. 

### 1.6 Performance/Load Tests

* **Lib micro-bench:** Criterion benches for `decode`/`encode` across sizes (≤256 B, 4 KiB, 64 KiB), reporting p50/p95 and allocs/op.
* **Host rigs:** measure `oap_read_frame_seconds` / `oap_frames_total` under load; enforce p95 < 20 ms and reject-rate SLO.  

---

## 2) Coverage & Gates

### 2.1 Bronze (MVP)

* Unit + integration pass; coverage ≥ **70%**.
* Fuzz targets build; minimal corpus present.
* Config validation paths covered (good & bad `ack_window_frames`). 

### 2.2 Silver (Useful Substrate)

* Property suites merged (round-trip, `NeedMore`, ACK algebra).
* Fuzz **≥1h** in CI; corpus saved/minimized.
* Coverage ≥ **85%**.
* Host chaos scripts exist (reject-rate, oversize, starvation); metrics keys exported. 

### 2.3 Gold (Ops-Ready)

* Nightly fuzz **≥4h**; zero crashes.
* 24h host soak, alerts wired (reject rate, decode p95, oversize, starvation).  
* Coverage ≥ **90%** (logic paths; unsafe blocks must be 100% covered or justified).
* Perf regression tracked vs baselines (`oap_read_frame_seconds` p95 < 20 ms; reject-rate < 0.5%). 

---

## 3) Invocation Examples

### 3.1 All Tests

```bash
cargo test -p oap --all-targets -- --nocapture
```

### 3.2 Fuzz Target

```bash
cargo fuzz run parser_fuzz -- -max_total_time=60
```

### 3.3 Loom (concurrency model)

`oap` has **no internal tasks/locks across `.await`**, so loom is **mostly N/A**; keep a small loom suite for any shared test helpers that use locks. 

```bash
RUSTFLAGS="--cfg loom" cargo test -p oap --test loom_*
```

### 3.4 Benchmarks

```bash
cargo bench -p oap
```

---

## 4) Observability Hooks

* Test harnesses **must** assert correct metric emission via a fake metrics facade (`oap_frames_total`, `oap_rejects_total`, `oap_read_frame_seconds`). 
* On failure, emit structured JSON logs in the host tests (never raw DATA or PQ bitmasks). 

---

## 5) CI Enforcement

**Gates (workspace CI):**

* Unit/integration: `cargo test --workspace --all-targets`.
* Lints: `cargo fmt -- --check`, `cargo clippy -D warnings`.
* Security/advisories: `cargo deny check advisories`.
* Coverage: `grcov` or `tarpaulin` on `oap` crate.
* Fuzz: nightly job (`cargo fuzz run ...`) with min-crash regression corpora.
* API stability (bonus): `cargo public-api`, `cargo semver-checks` (detect surface drift). 

**Perf regression (tie-in):** compare Criterion JSON vs stored baselines; fail if p95 ↑>10% or TPS ↓>10%. (Matches PERFORMANCE gates.)

---

## 6) Open Questions (crate-specific resolutions)

* **Loom coverage:** minimal (helpers only); core parser is single-threaded & CPU-bound. 
* **Mandatory fuzz targets:** `parser_fuzz`, `header_fuzz`, `ack_fuzz` (nightly).
* **Perf SLOs to enforce:** `oap_read_frame_seconds` p95 < **20 ms**; reject rate < **0.5%**; ACK starvation time at zero < **5 s/5 m**. 

---

## 7) Canonical Test Vectors (appendix)

Include under `tests/vectors/`:

* Valid: minimal `HELLO`, `START`, `DATA` with and without `obj:"b3:<hex>"`, `END`, `ACK{up_to}` sequences.
* Invalid: oversize `len=1 MiB+1`, unknown envelope, duplicate headers, truncated at every byte, `DATA` without `obj` where required, `ACK` rollback.  

---

## 8) Developer QoL

* **Fast shard:** `cargo test -p oap --lib -q` during TDD.
* **ASan/UBSan:** enable on nightly fuzz CI job.
* **Miri (spot checks):** `cargo +nightly miri test -p oap` for UB sanity.

---

## 9) Bronze→Silver→Gold Checklist (paste in PRs)

* [ ] Unit + integration passing; vectors updated (Bronze).
* [ ] Property suites merged; fuzz ≥1h CI; coverage ≥85% (Silver).
* [ ] Nightly fuzz ≥4h; 24h host soak; coverage ≥90%; perf gates green (Gold).
* [ ] API diffs reviewed if public surface changed (public-api + semver checks). 

---

### Notes for Auditors

* **Protocol constants & envelope set are frozen for OAP/1**; tests must never “configure” the 1 MiB cap or introduce new envelope kinds without a major bump. 
* **Metrics names are canonical** and low-cardinality; reasons must be one of the enumerated values.  

---

