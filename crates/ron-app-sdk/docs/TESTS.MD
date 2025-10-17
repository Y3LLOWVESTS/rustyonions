
---

# 🧪 TESTS.md

*Audience: developers, auditors, CI maintainers*
*msrv: 1.80.0 (Tokio/loom compatible)*

---

## 0) Purpose

Define the **testing contract** for `ron-app-sdk`:

* Unit, integration, property, fuzz, chaos/soak (host harness), and performance tests.
* Coverage targets and Bronze→Silver→Gold **acceptance gates**.
* Dev & CI **invocation** commands with feature/transport matrices.
* Guardrails to prevent perf drift, schema drift, and silent security regressions.

The SDK is a **Tier-2 library**; tests are **host-centric** (no daemon). Integration tests use a **mock gateway** (TLS and Tor-sim via SOCKS stub when feature-gated).

---

## 1) Test Taxonomy

### 1.1 Unit Tests

**Scope:** Pure logic and boundaries, fast (<100 ms), **no network**.

**Targets (examples):**

* `config::from_env` and `config::validate` (deadlines, backoff caps, cache bounds).
* `limits::oap_frame_guard` (hard cap 1 MiB; refuse oversize).
* `storage::chunker` (~64 KiB boundaries; last chunk ≤64 KiB).
* `auth::cap_attach` (macaroon always present; never logged).
* `idempotency::key_gen` (stable formatting; entropy sanity).
* `retries::policy` (non-retriable for auth/schema; exponential with cap for 5xx/timeouts).
* `logs::redaction` (secrets/macaroons/idem keys always redacted).

**Location:** `src/**/*` guarded with `#[cfg(test)]`.

**Run:**

```bash
cargo test -p ron-app-sdk --lib
```

---

### 1.2 Integration Tests

**Scope:** End-to-end crate surface via **mock gateway** under `tests/`.

**Must include:**

* **API round-trip:** Encode request → mock → decode response; strict DTOs (`deny_unknown_fields`).
* **Capability flow:** Requests carry macaroons; **Expired/Denied** errors are **not retried**.
* **Limits:** Refuse >1 MiB OAP frames; storage streams in ~64 KiB chunks.
* **Concurrency invariants:** Cancel-safe futures, no task leaks on drop.
* **Retry/Backoff:** Transient 5xx + timeouts retried up to cap; idempotency keys prevent duplicate side-effects.
* **Rate limiting:** Respect `Retry-After`; metric increments (`sdk_rate_limited_total`).
* **Tor path (feature `tor`):** SOCKS stub; higher latency; deadlines honored.

**Suggested files:**

* `tests/mock_gateway.rs` (axum/tower mock + fixtures)
* `tests/integration_api.rs` (CRUD-like samples across planes)
* `tests/integration_limits.rs` (OAP cap, chunking)
* `tests/integration_retries.rs` (5xx/timeout, rate-limit)
* `tests/integration_tor.rs` (behind `--features tor`)

**Run:**

```bash
cargo test -p ron-app-sdk --test '*'
# With Tor feature:
cargo test -p ron-app-sdk --features tor --test integration_tor
```

---

### 1.3 Property-Based Tests

**Scope:** DTO codecs, OAP framing, chunking, idempotency semantics (at SDK boundary).

**Tooling:** `proptest` (preferred) or `quickcheck`.

**Invariants:**

* **No panics** for any generated input within configured bounds.
* **Round-trip**: `decode(encode(x)) == x` for DTOs (within schema).
* **Framing:** `sum(chunks) == payload` and `max(chunk) ≤ 64 KiB`.
* **OAP bound:** `frame_size(x) ≤ 1 MiB` ⇒ accepted; `> 1 MiB` ⇒ rejected early.
* **Idempotency:** Two identical requests with same key → one semantic effect (mock gateway deduplicates by key).

**Run:**

```bash
PROPTEST_CASES=4096 cargo test -p ron-app-sdk --test prop_*
```

---

### 1.4 Fuzz Tests

**Scope:** Wire-adjacent surfaces the SDK validates:

* DTO decoders
* OAP frame parser/validator
* Header builder (capabilities, idempotency)
* Error mappers (server → typed SdkError)

**Tooling:** `cargo fuzz` (libFuzzer).
**Corpus:** Seed with unit/integ artifacts + minimized crashes.

**Acceptance:** **No crashes** in a 4h run locally/nightly.

**Run:**

```bash
cargo fuzz run dto_decode_fuzz -- -max_total_time=60
cargo fuzz run oap_frame_fuzz -- -max_total_time=60
```

---

### 1.5 Chaos / Soak (Host Harness)

**Scope:** Library-level “chaos” via mock gateway fault injection:

* Random 5xx (p=0.2), timeouts (p=0.02), latency jitter.
* Burst rate-limits with `Retry-After`.
* Connection drops (to exercise reconnect/keep-alive).

**Acceptance:** 1h CI soak (PR), 4h nightly soak:

* **Zero** task/file-descriptor leaks.
* **p95 attempts ≤ 3**, **no duplicate side-effects** (idempotent writes).

**Run (example):**

```bash
RUST_LOG=info cargo test -p ron-app-sdk --test chaos_harness -- --ignored --nocapture
```

---

### 1.6 Performance / Load

**Scope:** Latency/throughput under TLS and Tor (feature-gated), plus alloc/op.

**Tools:** `criterion`, flamegraphs for investigations.

**Budgets (relative):**

* p95 **Δ ≤ +10%** vs previous release at same QPS & payload mix.
* Throughput **Δ ≥ −10%** vs previous release.
* Allocations/op **Δ ≤ +15%**.

**Run:**

```bash
cargo bench -p ron-app-sdk
```

Use `critcmp` to compare bench baselines, and store artifacts for regression checks.

---

## 2) Coverage & Gates

### 2.1 Bronze (MVP)

* Unit + integration tests pass on **stable** and **MSRV 1.80**.
* Coverage **≥ 70%** (line); DTO encode/decode units included.
* Fuzz harness **builds** (no runs required in PR).

### 2.2 Silver (Useful Substrate)

* Property-based tests present for DTOs + framing.
* Fuzz **≥ 1h** in nightly CI, zero crashes.
* Coverage **≥ 85%**.
* Chaos harness scripted and **runs for 30m** on PRs (ignored in quick mode).

### 2.3 Gold (Ops-Ready)

* Nightly fuzz **≥ 4h** per target, zero crashes.
* Soak **≥ 4h** nightly (leak-free; stable p95 attempts).
* Coverage **≥ 90%**; critical paths (limits, auth, retries) **100% branch**.
* Perf regressions auto-gated (p95/throughput/alloc deltas).

---

## 3) Invocation Examples

### 3.1 All tests, all targets

```bash
cargo test -p ron-app-sdk --all-targets --features "" -- --nocapture
```

### 3.2 With Tor transport

```bash
cargo test -p ron-app-sdk --features tor --all-targets -- --nocapture
```

### 3.3 Property tests (deeper)

```bash
PROPTEST_CASES=8192 RUST_BACKTRACE=1 cargo test -p ron-app-sdk --test prop_*
```

### 3.4 Fuzz (quick sanity)

```bash
cargo fuzz run dto_decode_fuzz -- -max_total_time=60
```

### 3.5 Loom (concurrency model)

```bash
RUSTFLAGS="--cfg loom" cargo test -p ron-app-sdk --test loom_*
```

### 3.6 Benchmarks

```bash
cargo bench -p ron-app-sdk
```

---

## 4) Observability Hooks

* Tests use a **JSON logger** with mandatory keys:
  `event, endpoint, attempt, deadline_ms, latency_ms, error_kind, transport, corr_id`.
* **Redaction is required**: never log macaroons, idempotency keys, or secrets.
* CorrIDs are **propagated** in integration/chaos tests to correlate retries and results.
* Perf/chaos tests emit Prometheus-style test metrics (optional) for quick diffing.

---

## 5) CI Enforcement

**Matrix:**

* Toolchains: **stable**, **msrv (1.80.0)**.
* Features: `default` and `tor`.
* OS: linux-x86_64 (primary), macOS (smoke).

**Jobs:**

```text
- build: cargo build --workspace --all-features
- fmt:   cargo fmt -- --check
- clippy: cargo clippy --workspace --all-targets -- -D warnings
- test:  cargo test --workspace --all-targets
- coverage:
    - linux: grcov or tarpaulin (line+branch if feasible)
- deny:  cargo deny check advisories bans licenses sources
- public-api (SDK): cargo public-api --diff-git-checks
- fuzz (nightly): cargo fuzz run <target> -- -max_total_time=3600
- soak (nightly): chaos harness --ignored for 4h
- bench (report-only): cargo bench; attach critcmp diffs (no hard gate unless regression > thresholds)
```

Artifacts: coverage HTML, crash reproducers, minimized corpora, criterion baselines, flamegraphs on perf job.

---

## 6) Open Questions (crate-specific to finalize)

* **Loom scope**: Which concurrency invariants are modeled? (e.g., retry backoff scheduler; cancel-safety of in-flight request map).
* **Mandatory fuzz targets**: Minimum set (DTO decode, OAP frame, header builder) vs. optional (error mappers).
* **Perf SLOs**: TLS p95 baseline at reference HW; Tor tolerance factor (e.g., p95(Tor) ≤ 2× p95(TLS)).
* **Cache tests**: If cache is enabled, what are required hit-rate SLOs and eviction semantics under load? Expose `sdk_cache_*` metrics in tests?

---

## 7) File/Folder Hints (suggested layout)

```
ron-app-sdk/
  docs/TESTS.md
  tests/
    mock_gateway.rs
    integration_api.rs
    integration_limits.rs
    integration_retries.rs
    integration_tor.rs
    chaos_harness.rs
    prop_dto_roundtrip.rs
    loom_retry_backoff.rs
  fuzz/
    fuzz_targets/
      dto_decode_fuzz.rs
      oap_frame_fuzz.rs
      header_builder_fuzz.rs
  benches/
    sdk_benches.rs
```

---

## 8) Quick Acceptance Checklist (per PR)

* [ ] Unit + integration tests pass on **stable** and **MSRV**.
* [ ] Coverage ≥ **70%** (Bronze) OR **85%** (Silver) OR **90%** (Gold).
* [ ] No new `clippy` warnings; `cargo deny` clean.
* [ ] Public API diff reviewed (additive-first).
* [ ] If touching DTOs/limits/retries: property tests updated and passing.
* [ ] If touching perf-sensitive code: bench run attached; no regression beyond thresholds.
* [ ] If touching auth/headers: redaction test passes; no secrets in logs.

---

