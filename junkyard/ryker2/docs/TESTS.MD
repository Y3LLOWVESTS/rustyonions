

````markdown
# 🧪 TESTS.md — ryker

*Audience: developers, auditors, CI maintainers*  
*msrv: 1.80.0 (Tokio/loom compatible)*

---

## 0) Purpose

Define the **test contract** for `ryker` (actor & mailbox runtime library):

* Unit, integration, property, loom, fuzz (parsers/state), chaos/soak (via host harness), performance.
* Explicit coverage goals & Bronze→Silver→Gold acceptance gates.
* Canonical invocation for devs & CI.

---

## 1) Test Taxonomy

### 1.1 Unit Tests (fast, <100ms)
Scope: pure logic, small structs, parsing, validation.

**Required units**
- `SpanLevel::from_str` happy/edge/error cases.
- `parse_size` (B/KiB/MiB; case & whitespace variants).
- `RykerConfig::default()` invariants.
- `RykerConfig::validate()` errors (mailbox_capacity=0, yield<batch, base>cap, amnesia mismatch, deadline range).
- Env override macro: one test per env key; restore env after.

Run:
```bash
cargo test -p ryker --lib
````

### 1.2 Integration Tests (end-to-end crate surface)

Scope: config→runtime→mailbox behavior; hot/cold reload semantics; backpressure.

**Must include**

* **Env→Config snapshot:** env set → `from_env_validated()` → assert effective fields match.
* **Deprecation alias:** `RYKER_JITTER` maps to `decorrelated_jitter` with WARN noted.
* **Hot vs cold reload:** fairness/deadline apply immediately; capacity/max_msg_bytes only for new mailboxes.
* **Backpressure path:** bounded mailbox returns Busy on `try_send` when full; metrics counters increment.
* **Deadline outcome:** handler exceeding deadline yields timeout outcome metric.
* **Amnesia guard:** `enabled=true` requires `zeroize_on_drop=true` → validation error; zeroize behavior smoke check.

Run:

```bash
cargo test -p ryker --test '*'
```

### 1.3 Property-Based Tests

Tooling: `proptest` (preferred).

**Invariants**

* **Size parser:** for any positive integer `n`, `format!("{n}KiB")` parses to `n*1024`; reject negatives/overflow.
* **Duration parser:** round-trip `humantime` strings within tolerance; reject empty/garbage.
* **Mailbox** (model): never duplicates or drops messages except by explicit policy (full/drop reason), preserves FIFO per mailbox.
* **Config monotonicity:** if `yield_every_n_msgs >= batch_messages` before, and both increase by same delta, validation still passes.

Run:

```bash
cargo test -p ryker --features proptest
```

### 1.4 Loom (concurrency interleavings; ignored by default)

Scope: producer/consumer + shutdown; watch ordering & cancellation.

**Tests**

* `loom_mailbox_basic`: N producers, 1 consumer; assert no deadlock, all sent==received or dropped-by-policy.
* `loom_shutdown_no_drop`: on shutdown signal, consumer drains until empty or deadline; no double-drop.
* `loom_backpressure_consistency`: under full queue, producers see Busy deterministically (no send that “vanishes”).

Run:

```bash
RUSTFLAGS="--cfg loom" cargo test -p ryker --tests -- --ignored
```

### 1.5 Fuzz Tests (parsers/state machines)

`cargo-fuzz` targets (lib-appropriate; no network wire here):

* `fuzz_parse_config_toml`: arbitrary TOML → either valid snapshot or structured error (no panic, no UB).
* `fuzz_parse_config_json`: same for JSON path.
* `fuzz_env_pairs`: random (key,value) env maps; ensure strict parse or typed error; never hang.
* `fuzz_mailbox_ops`: randomized sequences of enqueue/try_send/recv/shutdown; assert internal invariants (no negative counts, no leaks).

Acceptance (local/CI nightly): zero crashes; coverage increases corpus over time.

Run:

```bash
cargo fuzz run fuzz_parse_config_toml -- -max_total_time=60
```

### 1.6 Chaos/Soak (host harness)

Scope: run via `testing/host_echo_actor/` that embeds ryker.

Inject:

* Actor panic storm (supervision backoff respected).
* Queue saturation (Busy mapping stable; no deadlocks).
* Config reload flaps (hot/cold fields behave as documented).

Acceptance: 24h soak with **zero** FD/mem leaks; bounded Busy rate; consistent shutdown drains.

### 1.7 Performance/Load (see PERFORMANCE.md)

* Criterion micro-benches: enqueue/dequeue/batch.
* Span-overhead A/B: `info` vs `debug` cost ratio ≤ 5.0 p95.
* Alloc gates: enqueue path amortized 0 alloc/op.

Run:

```bash
cargo bench -p ryker
```

---

## 2) Coverage & Gates

### 2.1 Bronze (MVP)

* Unit + integration pass.
* Coverage (line) ≥ **70%** (exclude benches/loom).
* Fuzz targets build & smoke-run (≤60s each).

### 2.2 Silver (Useful Substrate)

* Property tests enabled and passing.
* Loom suite runs in CI weekly (ignored by default on PRs).
* Fuzz ≥ **1h** nightly, zero crashes.
* Coverage ≥ **85%**.

### 2.3 Gold (Ops-Ready)

* Fuzz ≥ **4h** nightly; corpus under version control.
* Chaos/soak 24h monthly on host harness.
* Coverage ≥ **90%** overall and ≥ **95%** for `config` & `mailbox` modules.
* Perf regression gates green (see PERFORMANCE.md §5).

---

## 3) Invocation Examples

### 3.1 All tests (unit + integration)

```bash
cargo test -p ryker --all-targets -- --nocapture
```

### 3.2 Property tests only

```bash
cargo test -p ryker --features proptest -- proptest
```

### 3.3 Loom suite

```bash
RUSTFLAGS="--cfg loom" cargo test -p ryker --tests -- --ignored loom_
```

### 3.4 Fuzz (quick pass)

```bash
cargo fuzz run fuzz_parse_config_toml -- -max_total_time=60
cargo fuzz run fuzz_mailbox_ops -- -max_total_time=60
```

### 3.5 Benches + compare to baseline

```bash
cargo bench -p ryker -- --save-baseline current
cargo criterion compare current testing/performance/baselines/ryker
```

---

## 4) Observability Hooks

* Tests emit **structured JSON** logs via `tracing-subscriber` when `RUST_LOG` is set; failures attach `corr_id`.
* Metrics scrapes available from host harness; perf tests assert on Prometheus samples (busy/latency histograms).
* Ensure **no PII** in test logs; redact message contents.

---

## 5) CI Enforcement

**PR pipeline**

* `cargo clippy -D warnings -W clippy::await_holding_lock`
* `cargo test --workspace --all-targets`
* `cargo public-api --simplified --deny-changes`
* `cargo deny check advisories bans licenses`
* `cargo fmt -- --check`

**Nightly**

* Fuzz 1–4h across targets (parallel jobs), crash artifact upload.
* Loom suite (ignored on PR) with timeout guard.
* Coverage (grcov or tarpaulin) threshold: track Bronze/Silver/Gold.
* Perf regression job (criterion delta & alloc gates) per PERFORMANCE.md §5.

Artifacts:

* `tests/vectors/` (env→config fixtures, JSON snapshots)
* `fuzz/corpus/*`
* `testing/performance/baselines/ryker/*`

---

## 6) Open Questions (ryker-specific)

* Which additional mailbox invariants should be loom-checked (e.g., fairness starvation bounds)?
* What explicit **zeroize** verification do we adopt for amnesia (heap poisoning / Miri assist)?
* Do we gate **span cost** per release train or per PR (perf job runtime trade-off)?
* Should we add a **golden diff** for `RykerConfig` serde JSON/TOML snapshots over versions?

---

✅ This testing contract keeps `ryker` **correct, concurrent, and fast**. It hardens parsers, validates reload semantics, proves mailbox safety under interleavings, and bars perf regressions.

```
```
