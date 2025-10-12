
---

# 🧪 TESTS.md — svc-interop

*Audience: developers, auditors, CI maintainers*
*msrv: 1.80.0 (Tokio/loom compatible)*

---

## 0) Purpose

Define the **test contract** for `svc-interop`:

* Unit, integration, property, fuzz, chaos/soak, and performance tests.
* Explicit coverage goals & Bronze→Silver→Gold acceptance gates.
* One-command invocation for devs & CI.
* Enforce invariants: **edge-only SHA-256 verification**, **BLAKE3-256 internal**, **reversible bridges**, **default-deny policy**, **bounded queues with backpressure**, **stable error envelope `{error,reason,corr_id}`**.

---

## 1) Test Taxonomy

### 1.1 Unit Tests

**Scope:** Pure logic and small modules; <100ms each.

**Examples (non-exhaustive):**

* DTO validation & error mapping to stable reason codes (`MALFORMED`, `BODY_LIMIT`, `RATE_LIMIT`, `BACKPRESSURE`, `DOWNSTREAM_UNAVAILABLE`, `POLICY_BLOCKED`, `DECOMP_LIMIT`).
* Hashing policy guards: reject propagation of SHA-256 beyond edge; assert BLAKE3-256 for internal addressing.
* Rate/Quota math (token-bucket), retry budget arithmetic.
* Audit envelope composition (no secrets; has `corr_id`, `decision`, `reason`).
* Config parsing with `deny_unknown_fields`, defaults, and bounds checks.

**Layout & run:**

* In-file tests under `src/**` with `#[cfg(test)]`.

```bash
cargo test -p svc-interop --lib
```

---

### 1.2 Integration Tests

**Scope:** End-to-end under `tests/*.rs`; spin up the HTTP app with test config.

**Must include:**

1. **API Round-trip:**
   `/healthz`, `/readyz`, at least one bridge endpoint (e.g., `/webhooks/providerX`). Signed webhook fixture → capability validate → internal action mock → audit event emitted → stable error envelope on failure.

2. **Config Reload Semantics:**
   Hot-reload or restart applies new `limits.*`, brownout thresholds, and provider secret rotation (dual-secret acceptance window).

3. **Concurrency Invariants:**
   Draining shutdown (no request lost, audit flushes or is explicitly dropped under amnesia). Bounded queues: when full, 429 Busy `reason="BACKPRESSURE"` (no deadlocks). No locks across `.await` in adapters (spot-check via loom, §3.4 & §8.3).

4. **Reason Code Contract:**
   Table-driven tests mapping failure conditions → HTTP status + `reason` + JSON envelope (snapshots in §7.2).

**Run:**

```bash
cargo test -p svc-interop --test '*'
```

**Fixtures:**
`tests/fixtures/` with provider payloads + signature headers (good/bad, skewed clock, replay).

---

### 1.3 Property-Based Tests

**Tooling:** `proptest` (preferred) or `quickcheck`.

**Invariants:**

* **No panics** for inputs within declared limits.
* **Round-trip encode/decode** for signed headers; benign header permutation tolerated.
* **Idempotency** of replay protection within window (same payload → single accept, duplicates → deny with stable reason).
* **Monotonic policy:** tightening policy must not permit previously denied operations.

```bash
cargo test -p svc-interop --features proptest -- --nocapture
```

---

### 1.4 Fuzz Tests

**Scope:** Wire-facing surfaces and parsers:

* `webhook_parser_fuzz` — headers & HMAC base string derivation.
* `json_dto_fuzz` — payload DTOs with `deny_unknown_fields`.
* `reason_envelope_fuzz` — error mapping stays valid UTF-8 JSON and includes `corr_id`.
* (Recommended) `policy_eval_fuzz` — capability envelope + caveats.

**Tooling:** `cargo fuzz` (libFuzzer). Corpus seeded from `tests/fixtures`, prior minimizations, edge-case bodies near `body_limit`, and compressed payloads near `decompression.max_ratio`.

**Acceptance:**
CI nightly ≥1h, pre-release ≥4h. Zero crashes; zero ASAN/UBSAN issues.

```bash
cargo fuzz run webhook_parser_fuzz -- -max_total_time=3600
cargo fuzz run json_dto_fuzz -- -max_total_time=3600
```

---

### 1.5 Chaos / Soak Tests

**Scope:** Verify **brownout** (writes degrade first), no deadlocks/leaks.

**Faults to inject:**

* Upstream latency +300ms and 5% 5xx on index/storage.
* Queue saturation (reduce mpsc capacity) → expect `BACKPRESSURE` 429s, no stalls.
* Process crash (kill -9) → supervised restart, `/readyz` green within target.
* Clock skew ±10 minutes on one node → signature failures with stable reason; recovery after NTP sync.

**Acceptance:**
**Soak 24h**: zero FD/memory leaks, no unbounded queue growth, no increasing `audit_drop_total`. Read SLOs ≥99%; post-fault recovery ≤5m.

Harness: `testing/chaos/` scripts + `ronctl chaos inject`.

---

### 1.6 Performance / Load Tests

**Tools:**

* `criterion` for micro-benches (hashing, envelope build).
* HTTP load in `testing/load/` (Rust harness or `wrk`/`hey`).

**SLO checks:**
Intra-AZ p95 ≤ 120 ms, p99 ≤ 300 ms. `rejected_total{reason!="MALFORMED"}` / `requests_total` ≤ 1% over 5m. `audit_drop_total` remains 0 at target RPS. Busy/Backpressure only when limits are deliberately reached.

```bash
cargo bench -p svc-interop
testing/load/run_local.sh http://127.0.0.1:8080
```

---

## 2) Coverage & Acceptance Gates

### 2.1 Bronze (MVP)

* Unit + integration pass.
* Line coverage ≥ **70%** (llvm-cov).
* Fuzz harness builds; 5-minute smoke per target.
* Chaos smoke: latency fault only.

### 2.2 Silver (Useful Substrate)

* Property-based tests for parsers & policy.
* Nightly fuzz ≥ **1h** per target; zero crashes.
* Coverage ≥ **85%** lines and ≥ **70%** branches.
* Chaos scripts: latency, 5% error bursts, queue saturation.
* Loom model(s) for at least one critical queue/worker path.

### 2.3 Gold (Ops-Ready)

* Nightly fuzz ≥ **4h** across all targets with ASAN; zero crashes/leaks.
* 24h soak green: no leaks, no queue runaway, SLOs stable.
* Coverage ≥ **90%** lines and ≥ **80%** branches for critical modules (adapters, policy, error mapping).
* Performance regression guardrails: fail CI on p95 drift > **10 ms** or reject_rate + **0.5%** vs baseline (unless waivered).
* Mutation testing: ≥ **85%** mutants killed on `adapters/*` and `error/*`.

---

## 3) Invocation Examples

### 3.1 All Tests (dev)

```bash
cargo test -p svc-interop --all-targets -- --nocapture
```

### 3.2 Faster Runner (optional)

```bash
cargo nextest run -p svc-interop
```

### 3.3 Fuzz Targets

```bash
cargo fuzz run webhook_parser_fuzz -- -max_total_time=60
cargo fuzz run json_dto_fuzz -- -max_total_time=60
```

### 3.4 Loom (concurrency model checks)

```bash
RUSTFLAGS="--cfg loom" cargo test -p svc-interop --test loom_*
```

### 3.5 Benchmarks

```bash
cargo bench -p svc-interop
```

---

## 4) Observability Hooks

* Tests **must** emit structured JSON logs on failure with `corr_id`, `reason`, `provider`, and test name.
* Integration harness injects a deterministic `corr_id` to correlate requests, logs, metrics, and audit events.
* The load harness scrapes `/metrics` during runs to assert SLOs and reject ratios.

---

## 5) CI Enforcement

**GitHub Actions (suggested):**

* Build & Tests:

  * `cargo test --workspace --all-targets --locked`
  * `cargo fmt --all -- --check`
  * `cargo clippy --workspace -- -D warnings`
* Supply chain:

  * `cargo deny check advisories bans licenses sources`
* Coverage (llvm-cov):

  * `cargo llvm-cov --workspace --json --output-path cov.json`
  * gate lines/branches by tier (see §2)
* Fuzz (nightly):

  * `cargo fuzz run <each_target> -- -max_total_time=3600`
* Chaos smoke (scheduled):

  * `testing/chaos/smoke_latency.sh` against containerized svc
* Perf guardrail:

  * compare `testing/load/out/run.json` to `testing/load/baseline.json`; fail on drift
* Prometheus rules:

  * `promtool check rules ops/prometheus/interop_rules.yaml`

---

## 6) Crate-Specific Mandates

**Loom-checked invariants (minimum):**

* `adapters::work_queue` producer uses `try_send`; on full returns typed Busy; **no deadlock** with consumers.
* **Graceful shutdown** drains watchers; no hangs from partially dropped channels.
* `audit::ring_buffer` has **no starvation** under interleaved producers.

**Mandatory fuzz targets:**

* `webhook_parser_fuzz`, `json_dto_fuzz`, `reason_envelope_fuzz`.
* (Recommended) `policy_eval_fuzz`.

**Performance SLO measurements (must record):**

* p50/p95/p99 under 1× and 3× target RPS.
* `rejected_total` ratio over 5m (exclude `MALFORMED`).
* `audit_drop_total` (must be zero at target RPS).
* CPU% and RSS trends; confirm no growth during 30-minute steady load.

---

## 7) File/Folder Layout (tests & tooling)

```
svc-interop/
├─ src/
│  └─ ... (unit tests inline with #[cfg(test)])
├─ tests/
│  ├─ api_roundtrip.rs
│  ├─ config_reload.rs
│  ├─ reason_codes.rs
│  ├─ concurrency_shutdown.rs
│  ├─ loom_model_queue.rs            # behind cfg(loom)
│  ├─ fixtures/
│  │  ├─ providerX_event.json
│  │  ├─ providerX_sig_good.txt
│  │  ├─ providerX_sig_bad.txt
│  │  └─ ...
│  └─ golden/
│     ├─ reason_BODY_LIMIT.json
│     ├─ reason_DECOMP_LIMIT.json
│     ├─ reason_RATE_LIMIT.json
│     ├─ reason_BACKPRESSURE.json
│     ├─ reason_DOWNSTREAM_UNAVAILABLE.json
│     ├─ reason_POLICY_BLOCKED.json
│     └─ reason_MALFORMED.json
├─ fuzz/
│  ├─ fuzz_targets/
│  │  ├─ webhook_parser_fuzz.rs
│  │  ├─ json_dto_fuzz.rs
│  │  └─ reason_envelope_fuzz.rs
│  └─ corpus/
├─ benches/
│  ├─ hashing.rs
│  └─ envelope.rs
├─ testing/
│  ├─ chaos/
│  │  ├─ smoke_latency.sh
│  │  ├─ inject_errors.sh
│  │  └─ skew_clock.sh
│  └─ load/
│     ├─ run_local.sh
│     └─ baseline.json
└─ ops/
   └─ prometheus/
      └─ interop_rules.yaml
```

---

## 8) Precision Upgrades (Hard Gates)

### 8.1 Tooling Quickstart

```bash
cargo install cargo-nextest
cargo install cargo-llvm-cov
cargo install cargo-fuzz
cargo install cargo-mutants
```

### 8.2 Coverage via `cargo-llvm-cov`

```bash
cargo llvm-cov --workspace --lcov --output-path coverage.lcov
cargo llvm-cov --open
cargo llvm-cov --json --output-path cov.json
jq '.data[0].totals.lines.pct' cov.json | awk '{exit !($1 >= 85)}'   # Silver example gate
```

### 8.3 Sanitizers Matrix (tests + fuzz)

```bash
export RUSTFLAGS="-Zsanitizer=address -C debuginfo=1"
export RUSTDOCFLAGS="-Zsanitizer=address"
export ASAN_OPTIONS=detect_leaks=1
cargo +nightly test -Zbuild-std --target x86_64-unknown-linux-gnu
cargo fuzz run webhook_parser_fuzz --sanitizer=address -- -max_total_time=3600
```

### 8.4 Mutation Testing (Gold+)

```bash
cargo mutants --workspace --timeout 20 --tests "nextest"
```

Gate: ≥ **85%** mutants killed on `adapters/*` and `error/*`.

### 8.5 Cross-Target Matrix (Gate N)

```bash
rustup target add aarch64-unknown-linux-gnu
cargo test -p svc-interop --target aarch64-unknown-linux-gnu
```

(Optional) add `x86_64-unknown-linux-musl` for static-link checks.

### 8.6 Golden Files for Reason Codes

Snapshot the error envelope for each reason in `tests/golden/*.json`.
Gate: any diff requires PR label `api-break` and reviewer ACK.

### 8.7 Differential Test: Edge SHA-256 vs Internal BLAKE3

Paired test must assert:

1. Edge accepts SHA-256-signed payloads;
2. No SHA-256 appears in internal spans/logs/audits;
3. Persisted address is BLAKE3-256.
   Gate: fail if internal outputs match `/sha-?256/i`.

### 8.8 Replay & Clock-Skew Properties

Property test generates `(payload, t, Δ)`; accept if `|Δ| ≤ 5m`, otherwise stable deny reason.
Integration confirms **dual-secret** acceptance during rotation overlap.

### 8.9 Perf Baseline Artifact

`testing/load/baseline.json` vs `testing/load/out/run.json`:

```bash
jq -n --argfile base testing/load/baseline.json --argfile run testing/load/out/run.json '
  ($run.p95_ms - $base.p95_ms) <= 10 and
  ($run.reject_rate <= ($base.reject_rate + 0.005))' | grep true
```

### 8.10 Flake Quarantine Policy

* Mark with `#[ignore = "flake: <issue#>"]`; auto-open ticket.
* CI fails if >3 flakes exist or any flake >14 days old.

### 8.11 Prometheus Rule Validation

```bash
promtool check rules ops/prometheus/interop_rules.yaml
```

### 8.12 Loom Model Targets (Concrete)

Run at least these models:

* `adapters::work_queue::producer_try_send_no_deadlock`
* `shutdown::drain_all_watchers_without_block`
* `audit::ring_buffer_no_starvation`

```bash
RUSTFLAGS="--cfg loom" cargo test -p svc-interop --test loom_model_queue -- --nocapture
```

---

## 9) Open Questions (to finalize)

* Which adapters/providers are **MUST-test** in CI vs optional (feature-flagged)?
* Exact **dual-secret** rotation overlap per provider (default 24h?).
* Enforce gzip/zstd decompression ratio tests per provider path?
* Which runners handle nightly 4h fuzz (self-hosted vs ephemeral)?

---

