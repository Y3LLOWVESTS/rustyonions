# 🧪 TESTS.md — svc-edge

*Audience: developers, auditors, CI maintainers*
*msrv: 1.80.0 (Tokio/loom compatible)*

---

## 0) Purpose

This file defines the **testing contract** for `svc-edge`—a stateless edge asset server with strong ETag semantics, byte-range support, optional live-fill from allow-listed HTTPS origins, and strict backpressure.
We specify the full test taxonomy (unit → integration → property → fuzz → chaos/soak → performance), explicit **coverage goals**, and **Bronze→Silver→Gold** acceptance gates, plus **dev & CI invocations**.

High-risk areas we must continuously test:

* **ETag/Range correctness** (200/206, 304, 416 behavior; strong ETag stability).
* **Backpressure** (bounded queues, `max_inflight`, RPS caps → reject instead of meltdown).
* **Amnesia mode** (no persistence; reject operations requiring durable writes).
* **Live fill** (allow-list enforcement; retry/jitter bounded; integrity check).
* **Observability** (golden metrics present; corr_id propagation).
* **Startup/readiness gates** (packs verified, deps reachable).

---

## 1) Test Taxonomy

### 1.1 Unit Tests

**Scope:** Pure logic, header parsing, policy checks, small helpers (<100 ms).
**Location:** `src/**` under `#[cfg(test)]`.
**Focus areas & examples:**

* **Range parser**: valid/invalid ranges; satisfiable vs unsatisfiable → 206 vs 416.
* **ETag generator**: determinism across identical bytes; changes on mutation.
* **Allow-list**: host/port/case/IDNA normalization; subdomain pitfalls.
* **Retry policy**: capped backoff; jitter bounds.
* **Content-type sniff** (if any): minimal signatures only; no panics on short inputs.

Run:

```bash
cargo test -p svc-edge --lib
```

### 1.2 Integration Tests

**Scope:** End-to-end crate surface (`tests/*.rs`) using the real HTTP stack.
Spin up `svc-edge` on ephemeral ports with a minimal config and fixture packs.

**Must include:**

* **API Round-Trip**

  * `GET /edge/assets/{path}` → 200/206 with `Accept-Ranges`, `ETag`, `Last-Modified` (if applicable).
  * Conditional GET: `If-None-Match` → 304 when tag matches.
  * Range: `Range: bytes=0-99`, `bytes=100-`, multi-range (if supported) → 206 with correct `Content-Range`.
* **CONFIG reload semantics**

  * Change allow-list; SIGHUP or hot-reload → new origins honored, old denied.
* **CONCURRENCY invariants**

  * Saturate with N clients > `max_inflight`; observe rejects not collapse.
  * Graceful shutdown drains in-flight, no leaked tasks/sockets.
* **Amnesia mode**

  * Live-fill attempts in amnesia → deterministic reject; `/readyz` stays stable.
* **Live fill (happy path)**

  * Upstream served via local test server; allow-listed; success increases `edge_live_fill_success_total`.
* **Error surfaces**

  * Upstream 4xx/5xx; timeouts; malformed headers → appropriate 4xx/5xx here, no panics.

Run:

```bash
cargo test -p svc-edge --test '*'
```

**Fixtures (recommendation):**
`tests/fixtures/` with:

* `small.bin` (1 KiB), `medium.bin` (64 KiB), `large.bin` (5–20 MiB)
* A tiny PMTiles/MBTiles or generic pack format the crate supports (if not, use raw files).

### 1.3 Property-Based Tests

**Scope:** Parsers, state machines, and invariants that must hold for all inputs.
**Tooling:** `proptest` (preferred) or `quickcheck`.

**Invariants:**

* **Range parsing:**

  * Generative ranges → either satisfiable (206) or unsatisfiable (416) with exact `Content-Range` math.
  * Normalization idempotent: normalize(normalize(r)) == normalize(r).
* **ETag & bytes:**

  * Equal byte slices → equal ETags; unequal slices → ETag differs with overwhelming likelihood (strong tag).
* **Allow-list:**

  * For any host H, allow(H) XOR deny(H) must be true (no tri-state leak).

Run:

```bash
cargo test -p svc-edge --features proptest -- --nocapture
```

### 1.4 Fuzz Tests

**Scope:** Wire-facing surfaces and parsers:

* HTTP header parsing (Range, If-None-Match, Content-Range).
* ETag/metadata decoding (if any).
* Live-fill URL & host validation pipeline (scheme/host/port/path normalization).

**Tooling:** `cargo fuzz` (libFuzzer).
**Targets (examples):**

* `fuzz_targets/range_header.rs`
* `fuzz_targets/if_none_match.rs`
* `fuzz_targets/allowlist_url.rs`

**Corpus:** Start with seeds from integration tests; augment from production exemplars (scrubbed).
**Acceptance:** **4 h** fuzz run, **0 crashes**, **0 OOMs** (Gold gate).

Commands:

```bash
cargo fuzz run range_header -- -runs=0 -max_total_time=3600
cargo fuzz run allowlist_url -- -runs=0 -max_total_time=3600
```

### 1.5 Chaos / Soak Tests

**Scope:** Service behavior under adverse conditions (external to unit/integ).

* **Process crashes**: kill -9; confirm clean restart and readiness in ≤ 3 min.
* **Network faults**: latency, loss, timeouts on live-fill (Toxiproxy / netem).
* **Backpressure**: sustained RPS slightly above limits; ensure rejects (429) rather than rising latencies → meltdown.
* **Disk pressure** (when packs are local): simulate slow I/O / full disk; service must fail fast and remain responsive on other paths.
* **Leak checks**: file descriptors, memory.

**Acceptance:** **24 h** soak: no FD growth trend, RSS plateau within ±10%, p95 ≤ baseline +10%, rejects < 1% steady-state.

Scripts/examples live in `testing/chaos/` (see RUNBOOK appendices).

### 1.6 Performance / Load Tests

**Scope:** Throughput, latency, quotas vs. SLOs.
**Tools:**

* Micro: `criterion` for hot functions (ETag, range calculations).
* Macro: `k6`, `wrk`, or `hey` against real server; optional `criterion` custom HTTP harness in `testing/perf/`.

**SLO targets (local, warm-hit):**

* p95 < **40 ms**, p99 < **80 ms**; error rate < **0.1%**; shedding < **1%** (15 m).

**Profiles to test:**

* **Small-file burst** (1–8 KiB; 95% hit).
* **Mixed sizes** (16 KiB–2 MiB).
* **Range patterns** (0-99, 100-, tail ranges).
* **Live-fill ratio** (5–10% miss to allowed origin).

Commands:

```bash
# Example: 200 concurrent for 2 minutes, range requests
hey -z 2m -c 200 -H "Range: bytes=0-8191" http://127.0.0.1:8080/edge/assets/small.bin
```

---

## 2) Coverage & Gates

### 2.1 Bronze (MVP)

* Unit + integration tests pass on x64 Linux/macOS.
* Line coverage **≥ 70%** (excluding generated code & OS shims).
* At least **1 fuzz target** builds.
* `/healthz` and `/readyz` integration checks included.

### 2.2 Silver (Useful Substrate)

* Property tests present for **Range**, **ETag**, **Allow-list**.
* Fuzz runs in CI (≥ 1 h total across targets).
* Coverage **≥ 85%** lines, **≥ 70%** branches on parser modules.
* Basic **chaos** scripts (latency injection + kill-9) in `testing/chaos/`.
* Performance smoke: single-node warm-hit p95 **≤ 50 ms** with 100 rps.

### 2.3 Gold (Ops-Ready)

* Nightly fuzz **≥ 4 h** across targets, **0 crashes**.
* **24 h soak** CI job: FD/mem stable; rejects < 1%; 5xx < 0.1%.
* Coverage **≥ 90%** lines, **≥ 80%** branches for header/path/allow-list modules.
* Performance regression tracked release-to-release with stored baselines (JSON artifact).
* ARM64 profile job: warm-hit p95 **≤ 40 ms** or documented delta with perf/Watt note.

---

## 3) Invocation Examples

### 3.1 All tests (fast path)

```bash
cargo test -p svc-edge --all-targets -- --nocapture
```

### 3.2 Specific integration test

```bash
cargo test -p svc-edge --test api_roundtrip -- --nocapture
```

### 3.3 Property tests

```bash
cargo test -p svc-edge --features proptest -- --nocapture
```

### 3.4 Fuzz target (1 minute smoke)

```bash
cargo fuzz run range_header -- -max_total_time=60
```

### 3.5 Loom (concurrency model)

> Loom tests should avoid real I/O; isolate executor primitives & backpressure channels.

```bash
RUSTFLAGS="--cfg loom" cargo test -p svc-edge --test loom_*
```

### 3.6 Benchmarks

```bash
cargo bench -p svc-edge
```

---

## 4) Observability Hooks

* All tests emit **structured JSON logs** on failure with `corr_id`, `route`, `status`, `reason`, `latency_ms`, `bytes_out`, `mode`.
* Integration/perf tests **scrape `/metrics`** snapshots before/after to assert golden signals exist:

  * `http_requests_total{status}`
  * `request_latency_seconds_bucket`
  * `rejected_total{reason}`
  * `edge_live_fill_{attempts,success}_total`
  * `io_timeouts_total{op}`
* Test harness attaches snapshots to artifacts (`testing/artifacts/*`) for auditors (Gold).

---

## 5) CI Enforcement

**Suggested GitHub Actions matrix (linux-x64, linux-arm64, macOS x64):**

* **Lint & format:** `cargo fmt -- --check`, `cargo clippy -- -D warnings`
* **Advisories:** `cargo deny check advisories bans licenses sources`
* **Tests:** `cargo test --workspace --all-targets`
* **Property (feature):** `cargo test -p svc-edge --features proptest`
* **Coverage:** `grcov` or `tarpaulin --engine llvm --out Xml`
* **Fuzz (nightly):** `cargo fuzz run ... -max_total_time=14400` (matrix over targets)
* **Soak (nightly):** launch server + Toxiproxy/netem job for 24 h (self-hosted runner)
* **Perf (release-only):** run perf profile; publish baseline JSON; diff vs last tag

Artifacts to keep:

* `coverage.xml`
* `fuzz/*.log`, `fuzz/artifacts/*`
* `/metrics` snapshots before/after perf & chaos
* `perf_baseline.json` (p50/p95/p99, error%, rejects%)

---

## 6) Open Questions (svc-edge binding)

* **Loom scope:** Which concurrency paths are loom-checked?

  * Proposal: backpressure queue (bounded mpsc), inflight counters, graceful shutdown sequencing.
* **Mandatory fuzz targets:**

  * `range_header`, `if_none_match`, `allowlist_url` (required); add `content_range_encoder` if present.
* **Perf SLOs to lock:**

  * Confirm p95 < 40 ms & p99 < 80 ms **warm-hit** intra-AZ are our canonical SLOs for Gold.
* **Multi-range support:** If implemented, define tests for content-type boundaries and `multipart/byteranges`.

---

## 7) Reference Test Plan (file layout)

```
testing/
  chaos/
    toxiproxy_scenarios.json
    linux_netem_latency.sh
    kill9_restart.sh
  perf/
    k6_script.js
    baseline.json        # updated in CI
  fixtures/
    small.bin
    medium.bin
    large.bin
  promql/
    rejects_5m.promql
    p95_latency.promql
tests/
  api_roundtrip.rs
  range_semantics.rs
  concurrency_backpressure.rs
  live_fill_allowlist.rs
  amnesia_mode.rs
  readiness_gates.rs
fuzz/
  fuzz_targets/
    range_header.rs
    if_none_match.rs
    allowlist_url.rs
benches/
  etag.rs
  range_math.rs
```

---

## 8) Example Assertions (snippets to copy into tests)

```rust
// Range semantics
assert_eq!(status, 206);
assert_eq!(headers["accept-ranges"], "bytes");
assert!(headers["content-range"].starts_with("bytes 0-99/"));

// Conditional GET
assert_eq!(res.status(), 304);
assert_eq!(res.headers().get("etag"), Some(prev_etag));

// Backpressure under load
assert!(reject_rate <= 0.01, "shedding > 1% over 15m");

// Live fill success ratio
let ratio = successes as f64 / attempts.max(1) as f64;
assert!(ratio >= 0.99, "live fill success < 99%");
```

---

## 9) Developer Checklist (pre-PR)

* [ ] Unit + integration tests green locally.
* [ ] Property tests pass (if feature-gated).
* [ ] New headers/fields covered by tests & PromQL.
* [ ] Added/updated fixtures if API changed.
* [ ] Updated perf baseline (only for code touching hot path).
* [ ] Added fuzz seeds when new parser logic appears.

---

✅ With this contract, `svc-edge` maintains **repeatable rigor**: correctness of ETag/Range, deterministic shedding, safe amnesia behavior, and live-fill integrity—guarded by unit/prop/fuzz, verified via metrics, and enforced via CI gates up to **Gold**.
