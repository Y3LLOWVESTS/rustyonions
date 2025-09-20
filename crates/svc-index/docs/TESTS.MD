
---

# 🧪 TESTS.md (Template)

*Audience: developers, auditors, CI maintainers*
*msrv: 1.80.0 (Tokio/loom compatible)*

---

## 0) Purpose

Define the **test contract** for this crate:

* Unit, integration, property, fuzz, chaos, performance.
* Explicit coverage goals & Bronze→Silver→Gold acceptance gates.
* Invocation commands for devs & CI.

---

## 1) Test Taxonomy

### 1.1 Unit Tests

* Scope: Functions/modules, fast (<100ms), pure logic.
* Location: `src/*` annotated with `#[cfg(test)]`.
* Run with:

  ```bash
  cargo test -p <crate> --lib
  ```

### 1.2 Integration Tests

* Scope: End-to-end crate surface (`tests/*.rs`).
* Must include:

  * API round-trip (REQ→RESP).
  * Config reload semantics【CONFIG.md】.
  * Concurrency invariants (shutdown, backpressure)【CONCURRENCY.md】.
* Run with:

  ```bash
  cargo test -p <crate> --test '*'
  ```

### 1.3 Property-Based Tests

* Scope: Parsers, codecs, protocol state machines.
* Tooling: `proptest` or `quickcheck`.
* Invariants:

  * No panics.
  * Round-trip encode/decode.
  * Idempotency of state transitions.

### 1.4 Fuzz Tests

* Scope: Wire-facing crates (overlay, gateway, oap).
* Tooling: `cargo fuzz`.
* Corpus seeded from CI + real traffic.
* Acceptance: 4h fuzz run, zero crashes.

### 1.5 Chaos/Soak Tests

* Scope: Service crates.
* Inject:

  * Process crashes (must restart cleanly).
  * Bus lag/drops (must not deadlock).
  * Disk full, slow I/O.
* Acceptance: 24h soak = zero FD/memory leaks.

### 1.6 Performance/Load Tests

* Scope: Throughput, latency, quotas.
* Example metrics: p95 < 80 ms intra-AZ.
* Tools: `criterion`, custom harness in `testing/*`.

---

## 2) Coverage & Gates

### 2.1 Bronze (MVP)

* Unit + integration tests pass.
* Code coverage ≥ 70%.
* Fuzz harness builds.

### 2.2 Silver (Useful Substrate)

* Property tests included.
* Fuzz run ≥ 1h in CI.
* Coverage ≥ 85%.
* Chaos tests scripted.

### 2.3 Gold (Ops-Ready)

* Fuzz run ≥ 4h nightly.
* Chaos/soak 24h in CI.
* Coverage ≥ 90%.
* Performance regression tracked release-to-release.

---

## 3) Invocation Examples

### 3.1 All Tests

```bash
cargo test -p <crate> --all-targets -- --nocapture
```

### 3.2 Fuzz Target

```bash
cargo fuzz run parser_fuzz -- -max_total_time=60
```

### 3.3 Loom (concurrency model)

```bash
RUSTFLAGS="--cfg loom" cargo test -p <crate> --test loom_*
```

### 3.4 Benchmarks

```bash
cargo bench -p <crate>
```

---

## 4) Observability Hooks

* Each test must log structured JSON on failure (see `OBSERVABILITY.md`).
* CorrIDs propagate through test harness to trace failures across crates.

---

## 5) CI Enforcement

* GitHub Actions:

  * `cargo test --workspace --all-targets`.
  * `cargo deny check advisories`.
  * `cargo fmt -- --check`.
  * Fuzz job (nightly).
  * Coverage job (grcov or tarpaulin).

---

## 6) Open Questions (to fill per crate)

* Which invariants are loom-checked?
* Which fuzz targets are mandatory?
* What SLOs are measured in perf tests?

---

✅ With this template, every crate declares its **testing contract**, ensuring reproducibility and preventing silent drift in test discipline.
