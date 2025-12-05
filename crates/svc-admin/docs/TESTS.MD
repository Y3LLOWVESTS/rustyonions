````md
# 🧪 TESTS.md — svc-admin

*Audience: developers, auditors, CI maintainers*  
*msrv: 1.80.0 (Tokio/loom compatible)*

---

## 0) Purpose

This document defines the **test contract** for `svc-admin`:

- What we test: unit, integration, property, fuzz, chaos, performance.
- What we guarantee: HTTP/API behavior, config invariants, concurrency & shutdown, security surfaces.
- How we run it: commands for devs & CI and what “good” looks like (Bronze → Silver → Gold).

`svc-admin` is an **admin-plane service** (UI + JSON API + HTTP proxy) that:

- reads config & auth mode,
- talks to **node admin endpoints** (`/healthz`, `/readyz`, `/version`, `/metrics`, `/api/v1/status`),
- exposes **normalized views** (AdminStatusView, metrics summaries) to operators.

Tests must prove:

- We **never lie** about node readiness or status.
- We **never crash** on hostile or malformed admin responses.
- We stay correct under **slow nodes**, **partial failure**, and **auth/IdP issues**.
- We respect **CONFIG**, **SECURITY**, **CONCURRENCY**, **OBSERVABILITY**, and **PERFORMANCE** docs for this crate.

---

## 1) Test Taxonomy

### 1.1 Unit Tests

**Scope:** single modules & functions; fast (< 50–100 ms); no real network; pure or mostly pure logic.

**Core areas:**

- **Config parsing & defaults**  
  - TOML → `Config` struct:
    - Correct defaults for bind/metrics addresses.
    - `ui.read_only` default & overrides.
    - `auth.mode` parsing and invalid mode rejection.
    - Node definitions (`nodes.*`) mapping into internal registry.

- **DTO construction / mapping**  
  - Raw node status JSON → `AdminStatusView`:
    - `ready=false` in node data can **never** become `ready=true` in view.
    - `amnesia` flags propagate correctly.
    - Missing fields -> sane defaults (`restartCount=0`, `notes=None`).
  - Raw `/metrics` text → summary DTOs:
    - Golden metrics recognized.
    - Unknown series ignored, not fatal.
    - Negative or NaN values treated safely.

- **Auth & mode selection**  
  - `auth.mode = "none" | "ingress" | "passport"`:
    - correct guard behavior for each:
      - “none” → always accept (dev only).
      - “ingress” → header-based identity mapping.
      - “passport” → token-based identity with “unauth”/“forbidden” decisions.

- **Readiness gate logic**  
  - All required readiness keys must be set before `ready=true`.
  - Negative conditions (e.g. metrics sampler failing) drive readiness to false.

**Location:**

- `src/config.rs` unit tests.
- `src/dto/*.rs` unit tests for mapping.
- `src/auth/*.rs` unit tests for modes/guards.
- `src/readiness.rs` unit tests for gate semantics.

**Run:**

```bash
cargo test -p svc-admin --lib
````

---

### 1.2 Integration Tests

**Scope:** end-to-end crate surface via HTTP and top-level entrypoints (router/server), **with fake nodes**.

We want “operator-visible” behavior tested:

* HTTP routes & JSON contracts (`API.md`).
* Config -> router -> outbound node calls -> AdminStatusView & metrics.
* Concurrency invariants: clean shutdown, bounded queues (see `CONCURRENCY.md`).
* Error mapping: upstream errors → stable JSON error envelopes & metrics.

**Patterns:**

* Use **in-process HTTP servers** (e.g., `axum::Router` on ephemeral ports) as fake nodes:

  * Provide endpoints:

    * `/healthz`, `/readyz`, `/version`, `/api/v1/status`, `/metrics`.
  * Configurable behavior:

    * healthy vs unhealthy vs slow vs malformed.
* Start svc-admin with:

  * test config pointing at fake nodes.
  * ephemeral UI & metrics ports (bind to `127.0.0.1:0` and introspect actual ports).

**Must-have integration tests:**

1. **Happy-path UI/API:**

   * `GET /` returns SPA index (or placeholder).
   * `GET /api/ui-config` returns expected theme & read-only flags from config.
   * `GET /api/me` for:

     * `auth.mode="none"` (dev identity),
     * `auth.mode="ingress"` (headers present),
     * `auth.mode="passport"` (mock token accepted).

2. **Node status round-trip:**

   * Given node admin responds with known JSON, `GET /api/nodes/{id}/status` returns correct `AdminStatusView`.
   * `ready` matches node `/readyz`.
   * Per-plane health mapping is correct.

3. **Partial node failure:**

   * Some fake nodes return 500 or are unreachable.
   * svc-admin:

     * still returns 200 for `/api/nodes` and `/api/nodes/{id}/status` where possible,
     * returns 5xx or error envelopes only for affected node(s),
     * **does not** mark unaffected nodes as failed.

4. **Metrics summarization:**

   * Fake `/metrics` with:

     * a few golden series (http RPS, error rates, latency).
     * extra noise metrics.
   * `GET /api/nodes/{id}/metrics/summary` returns expected aggregated values.

5. **Readiness & shutdown:**

   * Start svc-admin, wait until `/readyz = 200`.
   * Send SIGINT or call server shutdown path.
   * Assert:

     * `/readyz` flips to non-ready during drain.
     * No panics; all background tasks finish.

**Location:**

* `tests/http_smoke.rs`
* `tests/status_roundtrip.rs`
* `tests/metrics_summary.rs`
* `tests/shutdown.rs`

**Run:**

```bash
cargo test -p svc-admin --tests
# or explicitly:
cargo test -p svc-admin --test http_smoke
```

---

### 1.3 Property-Based Tests

**Scope:** input-space heavy or schema-mapping code; we care about:

* No panics on arbitrary inputs.
* Round-trip / idempotent behavior where applicable.
* Invariants from `CONFIG.md`, `CONCURRENCY.md`, `INTEROP.md`.

**Candidates:**

1. **Config loader:**

   * Generate random but structurally valid TOML fragments:

     * `ui.*`, `auth.*`, `nodes.*` sections.
   * Properties:

     * loader never panics; returns `Result<Config, Error>`.
     * valid values round-trip through `Config` → TOML → `Config`.

2. **AdminStatusView mapper:**

   * Generate random node status JSON (within schema envelope).
   * Property:

     * `ready=false` in raw data ===> `ready` never becomes true in view.
     * unknown fields are ignored; we don’t panic.

3. **Metrics parser:**

   * Generate random sequences of Prometheus-like lines + random noise.
   * Properties:

     * parser never panics,
     * total counted requests & errors are non-negative,
     * ignoring unknown series yields same golden outputs.

**Tooling:**

* `proptest` preferred (shrinking).
* Possibly `quickcheck` for simpler properties.

**Location:**

* `tests/prop_config.rs`
* `tests/prop_status.rs`
* `tests/prop_metrics.rs`

**Run:**

```bash
cargo test -p svc-admin --test prop_*
```

---

### 1.4 Fuzz Tests

`svc-admin` is HTTP/JSON/text facing; fuzzing is valuable for:

* metric text parsing,
* admin response parsing,
* config loader hardening.

**Targets (examples):**

1. `svc_admin_metrics_parser_fuzz`:

   * Input: arbitrary byte slices.
   * Feed into metrics parser as if it were `/metrics` body (subject to size limits).
   * Assertions:

     * no panics,
     * we bail out early on oversized input,
     * we don’t allocate unbounded memory.

2. `svc_admin_status_decoder_fuzz`:

   * Input: arbitrary JSON up to some limit.
   * Attempt to deserialize into raw node status and map to `AdminStatusView`.
   * Assertions:

     * errors are handled cleanly; no panics.

3. `svc_admin_config_loader_fuzz`:

   * Input: arbitrary bytes as TOML candidate.
   * `Config::from_toml_str` must never panic.

**Tooling:**

* `cargo fuzz`.

**Run (examples):**

```bash
# Setup (once)
cargo fuzz init

# Run individual fuzzers
cargo fuzz run svc_admin_metrics_parser_fuzz -- -max_total_time=60
cargo fuzz run svc_admin_status_decoder_fuzz -- -max_total_time=60
cargo fuzz run svc_admin_config_loader_fuzz -- -max_total_time=60
```

Nightly/longer runs for Gold; see §2.

---

### 1.5 Chaos / Soak Tests

**Scope:** `svc-admin` as a **service** with live HTTP listeners, contacting fake nodes or staging nodes.

Scenarios:

1. **Metrics slowness:**

   * Fake nodes delay `/metrics` responses close to timeout.
   * Ensure:

     * svc-admin doesn’t deadlock or starve,
     * stale metrics are indicated,
     * `/readyz` remains 200 if svc-admin itself is fine.

2. **Partial node outages:**

   * Randomly kill or disconnect subset of nodes.
   * Ensure:

     * svc-admin logs upstream errors,
     * UI shows per-node issues,
     * svc-admin doesn’t crash or flap readiness.

3. **Auth chaos (passport mode):**

   * IdP intermittently fails or returns invalid JWK set.
   * Ensure:

     * auth errors logged,
     * operators see 401/403 appropriately,
     * svc-admin remains up & ready (albeit with limited access).

4. **Process restarts:**

   * Periodically kill svc-admin while traffic is ongoing.
   * Confirm clean startup, no port leaks or FD leaks.

**Acceptance (for Gold):**

* 24h soak under mixed traffic and chaos injection:

  * zero memory leaks (RSS stable ± small drift),
  * FDs stable (< 60–70% of limit),
  * no panics,
  * `/readyz` consistent except during planned restarts.

---

### 1.6 Performance / Load Tests

See `PERFORMANCE.md` for SLOs; tests must validate:

* Local-only endpoints (UI + `/api/ui-config`, `/api/me`, `/api/nodes`) meet latency targets under small load.
* Node-backed endpoints (status, metrics summary) meet targets given well-behaved nodes.

**Harness:**

* `testing/performance/svc-admin/svc_admin_smoke.sh`:

  * Use `wrk`/`bombardier` against:

    * `/api/ui-config`,
    * `/api/me`,
    * `/api/nodes`,
    * `/api/nodes/{id}/status`.

* `testing/performance/svc-admin/svc_admin_fanout.sh`:

  * Use fake nodes to simulate N=10,25,50 nodes.
  * Measure:

    * RPS,
    * p95 latency,
    * CPU/mem.

**Run:**

```bash
# Dev smoke
./testing/performance/svc-admin/svc_admin_smoke.sh

# Fanout scaling test
./testing/performance/svc-admin/svc_admin_fanout.sh
```

---

## 2) Coverage & Gates

### 2.1 Bronze (MVP)

`svc-admin` can be merged to main & used in dev/staging when:

* ✅ Unit + integration tests pass:

  ```bash
  cargo test -p svc-admin --all-targets
  ```

* ✅ Minimal property tests exist (config loader & status mapping).

* ✅ Fuzz harnesses build (`cargo fuzz build`).

* ✅ Code coverage ≥ **70%** on `src/` (via `grcov` or `cargo-tarpaulin`).

### 2.2 Silver (Useful Substrate)

For **pre-Beta** / “team-usable in staging”:

* ✅ Property tests for:

  * config loader,
  * status mapping,
  * metrics parser.
* ✅ Fuzz runs **≥ 1h** in CI (nightly) across:

  * `svc_admin_metrics_parser_fuzz`,
  * `svc_admin_status_decoder_fuzz`,
  * `svc_admin_config_loader_fuzz`.
* ✅ Chaos test scripts exist and are runnable against a staging stack.
* ✅ Coverage ≥ **85%**, especially for:

  * config & readiness,
  * DTO mapping,
  * auth & guards,
  * error mapping.

### 2.3 Gold (Ops-Ready)

For **production-ready** svc-admin:

* ✅ Property test suite is stable and part of regular CI.
* ✅ Fuzz runs **≥ 4h nightly** on core fuzz targets (metrics, status, config).
* ✅ At least one **24h soak/chaos** test per quarter across:

  * svc-admin,
  * a small node fleet or fake-node cluster,
  * IdP/ingress.
* ✅ Coverage ≥ **90%** on:

  * crate’s core logic,
  * all error branches for HTTP handlers.
* ✅ Performance regression tracked release-to-release (see `PERFORMANCE.md`).
* ✅ Known failure modes are covered by one or more tests.

---

## 3) Invocation Examples

### 3.1 All Tests (dev-friendly)

```bash
# All unit + integration tests for this crate
cargo test -p svc-admin --all-targets -- --nocapture
```

### 3.2 Specific Test Modules

```bash
# Config tests only
cargo test -p svc-admin config

# HTTP smoke tests only
cargo test -p svc-admin --test http_smoke
```

### 3.3 Fuzz Targets

```bash
# Short fuzz run for quick dev feedback
cargo fuzz run svc_admin_metrics_parser_fuzz -- -max_total_time=60
cargo fuzz run svc_admin_status_decoder_fuzz -- -max_total_time=60
cargo fuzz run svc_admin_config_loader_fuzz -- -max_total_time=60
```

### 3.4 Loom (Concurrency Model Tests)

Focus on:

* readiness gate state machine,
* metrics sampler lifecycle,
* graceful shutdown.

```bash
# Example (once loom tests exist)
RUSTFLAGS="--cfg loom" \
  cargo test -p svc-admin --test loom_readiness
```

Loom tests should live under `tests/loom_*` and target small, deterministic components (not the whole HTTP stack).

### 3.5 Benchmarks

```bash
# Criterion benches (behind a feature to avoid pulling in dev-only deps)
cargo bench -p svc-admin --features benches
```

---

## 4) Observability Hooks

To improve debuggability:

* Tests SHOULD use `tracing` with a test subscriber that:

  * logs JSON or structured logs on failure,
  * includes `corr_id`, `node_id`, `auth_mode`, `route` where applicable.

* Integration tests should:

  * assert presence of `X-Corr-ID` headers or correlation IDs in logs.
  * when tests fail, print last few log lines / metrics snapshots.

* A shared test utility module (e.g., `tests/util/mod.rs`) can expose:

  ```rust
  fn init_test_tracing() { /* install tracing-subscriber for tests */ }
  ```

This connects directly with `OBSERVABILITY.md` and ensures failures in CI can be triaged quickly.

---

## 5) CI Enforcement

### 5.1 Core CI Jobs

At the workspace level, but relevant for `svc-admin`:

* **Test & Lints:**

  ```bash
  cargo test --workspace --all-targets
  cargo fmt -- --check
  cargo clippy --workspace --all-targets -- -D warnings
  ```

* **Security & deps:**

  ```bash
  cargo deny check advisories
  cargo deny check licenses
  ```

* **Public API drift** (for Rust surface):

  ```bash
  cargo public-api -p svc-admin --simplified --deny-changes
  ```

* **Coverage (nightly or gated branch):**

  * `grcov` or `cargo-tarpaulin` capturing svc-admin line & branch coverage.

### 5.2 Fuzz & Chaos (Nightly / Scheduled)

* Nightly fuzz:

  ```bash
  cargo fuzz run svc_admin_metrics_parser_fuzz -- -max_total_time=3600
  cargo fuzz run svc_admin_status_decoder_fuzz  -- -max_total_time=3600
  cargo fuzz run svc_admin_config_loader_fuzz   -- -max_total_time=3600
  ```

* Scheduled chaos/perf job:

  ```bash
  ./testing/performance/svc-admin/svc_admin_smoke.sh
  ./testing/performance/svc-admin/svc_admin_fanout.sh
  # plus chaos harness for fake nodes / IdP
  ```

Fail CI if:

* fuzz run finds a crash or timeout,
* chaos/perf harness regresses beyond thresholds defined in `PERFORMANCE.md`.

---

## 6) Open Questions (per svc-admin)

* **Loom invariants:**

  * Which concurrency-critical components are modeled with loom?

    * Candidate: readiness gate + metrics sampler + shutdown coordination.
  * How often do loom tests run (PR vs nightly)?

* **Mandatory fuzz targets:**

  * Is `svc_admin_metrics_parser_fuzz` considered mandatory for production?
  * Are we fuzzing any **auth-mode logic** (e.g., corrupted headers/tokens)?

* **Perf SLOs in tests:**

  * Which SLOs from `PERFORMANCE.md` are enforced as **hard CI gates** vs informational?
  * Do we maintain recorded perf baselines under `testing/performance/baselines/svc-admin/` and auto-compare?

* **Staging vs production test parity:**

  * Do we require staging chaos tests before prod upgrades (e.g., at least one partial-node-failure test)?

---

✅ With this `TESTS.md`, `svc-admin` declares a clear **testing contract**:

* What kinds of tests exist,
* How to run them,
* What thresholds define Bronze/Silver/Gold quality,

so that the admin plane stays trustworthy even as the rest of RON-CORE evolves.

```
```
