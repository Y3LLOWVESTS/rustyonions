

# 🧪 TESTS.md — macronode

*Audience: developers, auditors, CI maintainers*
*msrv: 1.80.0 (Tokio/loom compatible)*

---

## 0) Purpose

Define the **test contract** for `macronode`:

* Unit, integration, property, fuzz, chaos, and performance tests.
* Explicit coverage goals & Bronze→Silver→Gold acceptance gates.
* One-command invocations for devs & CI.
* Canonical interop vectors (OAP/1 frames, manifests, capabilities) exercised in CI.

---

## 1) Test Taxonomy

### 1.1 Unit Tests

**Scope:** Pure functions/modules (encode/decode, helpers), fast (<100 ms).
**Location:** `src/**/*` with `#[cfg(test)]` blocks.
**Run:**

```bash
cargo test -p macronode --lib
```

**Must-cover (examples):**

* OAP/1 envelope encode/decode helpers (no panics, exact round-trip).
* Capability caveat evaluation (deny unknown *critical* caveats).
* DAG-CBOR manifest validators (size/chunk invariants).
* HTTP error envelope construction (code → status mapping).

---

### 1.2 Integration Tests

**Scope:** End-to-end crate surface in `tests/*.rs`.
**Mandatory suites:**

* **API round-trip:** `GET /o/{addr}`, `HEAD /o/{addr}`, `POST /put` (with/without capability).
* **OAP/1 verbs:** `OBJ_GET`, `OBJ_PUT`, `CFG_SNAP`, `HEALTH_PING` (REQ→RESP end-to-end).
* **Config reload:** hot-reload triggers `ConfigUpdated` and rebinds transports (see `CONFIG.md`).
* **Concurrency invariants:** graceful shutdown ordering, backpressure, bus lag visibility (see `CONCURRENCY.md`).
* **Readiness semantics:** `/readyz` flips to 200 only when all readiness keys are green; degraded paths return 503 with JSON reason.

**Run:**

```bash
cargo test -p macronode --test '*'
```

---

### 1.3 Property-Based Tests

**Tooling:** `proptest` (preferred) or `quickcheck`.
**Targets & invariants:**

* **OAP/1 framing:** arbitrary header + payloads → decode → encode → byte-identical (no panics).
* **DAG-CBOR manifests:** arbitrary chunk partitions reconstruct to the same root BLAKE3; `sum(chunk.len)==size`.
* **Capability tokens (macaroons):** arbitrary caveat order is semantically idempotent; TTL windows respected.
* **HTTP header propagation:** `X-Corr-ID` always round-trips and is valid ULID/UUID.

**Run:**

```bash
cargo test -p macronode --lib --features proptest
```

---

### 1.4 Fuzz Tests

**Tooling:** `cargo fuzz`.
**Mandatory fuzz targets (crate-specific):**

* `oap_envelope_fuzz` — OAP/1 header/body parser (flags/len/verb).
* `manifest_cbor_fuzz` — DAG-CBOR manifest decoder/validator.
* `capability_token_fuzz` — macaroon/signed token parser & caveat interpreter.
* `http_path_fuzz` — router path & query parser for `/o/{addr}` and `/put`.
* `sse_line_fuzz` — SSE event framing (no unbounded growth / no panics).

**Acceptance:** ≥ 4h nightly fuzz, **zero crashes**; CI keeps and grows corpus.
**Example:**

```bash
cargo fuzz run oap_envelope_fuzz -- -max_total_time=60
```

---

### 1.5 Chaos / Soak Tests

**Scope:** Service-level resilience.
**Faults injected:**

* **Crash storms:** kill child tasks/process; supervisor must restart with jittered backoff; `/readyz` degrades during storm.
* **Bus pressure:** overflow broadcast queues; assert `bus_lagged_total` increments and no deadlocks.
* **Disk pressure:** simulate disk-full/slow I/O on storage path (macronode profile); reads fail-open, writes fail-closed.
* **Network:** slow-loris & TLS handshake floods (throttle without collapse).

**Acceptance:** 24h soak with **zero FD leaks**, bounded RSS, and no stuck tasks.

**Run (example harness):**

```bash
testing/performance/soak.sh --hours 24 --scenario macronode-default
```

---

### 1.6 Performance / Load Tests

**Scope:** Throughput, latency, quotas; aligns with `PERFORMANCE.md`.
**Tools:** `criterion`, `wrk`/`bombardier`, `tokio-console`, `cargo flamegraph`.
**Example metrics:** p95 GET intra-AZ < 80 ms; inter-AZ < 200 ms; PUT p95 < 150 ms; 5xx < 0.1%; 429/503 < 1%.
**Run:**

```bash
cargo bench -p macronode
testing/performance/http_load.sh --rps 12000 --duration 300s
```

---

## 2) Coverage & Gates

### 2.1 Bronze (MVP)

* Unit + integration tests pass.
* Coverage ≥ **70%** (lines).
* Fuzz harness builds and smoke-runs for 60s/target.
* Basic perf run (benchmarks) completes with artifacts.

### 2.2 Silver (Useful Substrate)

* Property tests present for OAP/1 + manifests + capability tokens.
* Fuzz in CI ≥ **1h** cumulative (rotating targets).
* Coverage ≥ **85%**.
* Chaos scripts exist and run for ≥ 60m locally.
* `/readyz` degraded-mode tests included.

### 2.3 Gold (Ops-Ready)

* Nightly fuzz ≥ **4h**, **zero crashes**.
* 24h soak in CI (scheduled), **zero FD/mem leaks**, no stuck tasks.
* Coverage ≥ **90%** (lines) and ≥ **80%** branches in protocol and capability modules.
* Perf regressions tracked release-to-release (see §5).

---

## 3) Invocation Examples

### 3.1 All tests (crate)

```bash
cargo test -p macronode --all-targets -- --nocapture
```

### 3.2 Specific integration test

```bash
cargo test -p macronode --test api_roundtrip -- --nocapture
```

### 3.3 Fuzz target (local quick run)

```bash
cargo fuzz run manifest_cbor_fuzz -- -max_total_time=120
```

### 3.4 Loom (concurrency model)

```bash
RUSTFLAGS="--cfg loom" cargo test -p macronode --test loom_*
```

### 3.5 Benches

```bash
cargo bench -p macronode
```

---

## 4) Observability Hooks

* Tests emit structured JSON logs on failure (JSONL with `ts`,`level`,`service`,`event`,`reason`,`corr_id`,`latency_ms`).
* `corr_id` is generated at test ingress and **propagated** across HTTP, OAP, and bus events.
* Failure artifacts:

  * `/target/test-logs/macronode/*.jsonl`
  * Flamegraphs (SVG) in `/target/criterion/**/report/`.
  * pprof snapshots (optional) in `/target/profiles/`.

---

## 5) CI Enforcement

**Recommended GitHub Actions jobs (per push + nightly):**

* **Unit/Integration:**
  `cargo test --workspace --all-targets -- --nocapture`
* **Lint/Format:**
  `cargo fmt -- --check` and `cargo clippy -- -D warnings`
* **Advisories & licenses:**
  `cargo deny check advisories bans licenses sources`
* **Coverage:** (one of)
  `grcov` or `cargo tarpaulin --workspace --engine llvm --timeout 600 --out Xml`
* **Fuzz (nightly):**
  matrix over targets (`oap_envelope_fuzz`, `manifest_cbor_fuzz`, `capability_token_fuzz`, `http_path_fuzz`, `sse_line_fuzz`) with `-max_total_time=14400`
* **Soak (scheduled):**
  run `testing/performance/soak.sh --hours 24`
* **Perf baselines:**
  upload `testing/performance/baselines/*.json`; fail if p95↑>10% or throughput↓>10%.

Artifacts saved from CI (logs, flamegraphs, baselines) must be retained ≥ 30 days.

---

## 6) Open Questions — **macronode answers (filled)**

**Q: Which invariants are loom-checked?**
**A:**

* Readiness DAG (config→listeners→bus→deps): no deadlock, correct ordering.
* Bus publish/subscribe: single consumer per receiver; no double-await; no lock across `.await`.
* Graceful shutdown: drains listeners, then cancels workers, then closes bus; `/readyz` must flip to 503 before stop.
* Backpressure: bounded queues/semaphores never block forever; timeouts propagate.

**Q: Which fuzz targets are mandatory?**
**A:** `oap_envelope_fuzz`, `manifest_cbor_fuzz`, `capability_token_fuzz`, `http_path_fuzz`, `sse_line_fuzz`. (Add corpus seeds from integration captures.)

**Q: What SLOs are measured in perf tests?**
**A:** p95 GET intra-AZ < 80 ms; p95 GET inter-AZ < 200 ms; PUT p95 < 150 ms; 5xx < 0.1%; 429/503 < 1%; RF(observed) ≥ RF(target). Throughput target ≥ 10k req/s per instance.

---

## 7) Canonical Vectors & Layout

```
macronode/
  tests/
    api_roundtrip.rs
    readiness.rs
    oap_verbs.rs
    loom_shutdown.rs
  fuzz/
    fuzz_targets/
      oap_envelope_fuzz.rs
      manifest_cbor_fuzz.rs
      capability_token_fuzz.rs
      http_path_fuzz.rs
      sse_line_fuzz.rs
  tests/vectors/
    oap1_obj_get_roundtrip.hex
    oap1_obj_get_roundtrip.json
    hello_world.bytes
    hello_world.manifest.cbor
    hello_world.manifest.json
    cap_ok.json
    cap_tampered.json
    cap_missing_pq_leg.json
  testing/performance/
    http_load.sh
    soak.sh
    baselines/
      http_get_intra.json
      http_get_inter.json
      http_put.json
```

---

## 8) Bronze→Silver→Gold DoD Checklist

* [ ] Unit & integration tests (Bronze).
* [ ] Coverage ≥ 70% (Bronze), ≥ 85% (Silver), ≥ 90% (Gold).
* [ ] Property tests for OAP/1, manifests, capabilities (Silver).
* [ ] Nightly fuzz ≥ 4h, zero crashes (Gold).
* [ ] 24h soak, zero FD/mem leaks (Gold).
* [ ] Perf baselines wired, regressions blocked (Bronze→Gold).
* [ ] Failure artifacts uploaded (logs, flamegraphs).

---

✅ With this contract, `macronode` testing is **repeatable, measurable, and enforced by CI**—covering logic correctness, protocol safety, resilience, and performance so regressions can’t sneak in.
