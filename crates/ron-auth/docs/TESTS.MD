
````markdown
# 🧪 TESTS.md — `ron-auth`

*Audience: developers, auditors, CI maintainers*  
*MSRV: 1.80.0 (loom-capable toolchain; not required for this crate)*

---

## 0) Purpose

Define the **test contract** for `ron-auth`:

- Unit, integration, property, fuzz, chaos drills (host-embedded), and performance.
- Explicit coverage goals & Bronze→Silver→Gold acceptance gates.
- Invocation commands for devs & CI.
- Golden vectors for **allow/deny** decisions and **stable reason strings**.

> `ron-auth` is a **pure verification library** (no I/O, no background tasks). Most chaos/soak is validated **in hosts** (e.g., `svc-gateway`) that wrap `verify()` and export metrics.

---

## 1) Test Taxonomy

### 1.1 Unit Tests (fast, pure)
**Scope:** token decode/parse helpers, caveat evaluation logic, reason-string normalization, decision shaping.  
**Location:** `src/**`, gated by `#[cfg(test)]`.  
**Run:**
```bash
cargo test -p ron-auth --lib -- --nocapture
````

**Must cover (examples):**

* Base64URL decode errors → `parse.b64` reason.
* CBOR parse errors / missing fields → `parse.cbor` / `schema.*`.
* Bounds enforcement: `max_token_bytes`, `max_caveats` → `parse.bounds`.
* Deterministic reason taxonomy (stable strings).
* Caveat evaluation primitives: `exp/nbf/path`, boolean conjunctions, custom caveat dispatch (unknown → deny).

---

### 1.2 Integration Tests (end-to-end)

**Scope:** full crate surface via `verify(token_b64, RequestCtx, MacKeyProvider, SigAdapter?) → Decision`.
**Location:** `tests/*.rs`.
**Run:**

```bash
cargo test -p ron-auth --test '*'
```

**Mandatory suites:**

* **`allow_deny_vectors.rs`** — replay **golden vectors** (see §3.5) across tenants/KIDs; assert `Decision::Allow` vs `Decision::Deny{reason}` and **exact** reason strings.
* **`config_bounds.rs`** — validate ranges & defaults; oversize tokens/too many caveats → `parse.bounds`.
* **`rotation_window.rs`** — provider exposes **current + N previous** KIDs; removing the previous immediately → `kid.unknown`.
* **`custom_caveats.rs`** — with/without registrar: known custom caveat passes when satisfied; unknown custom caveat **denies by default**.
* **`amnesia_mode.rs`** — tokens containing `Amnesia(true)` must **deny** unless host signals amnesia in `RequestCtx`.
* **`pq_envelope.rs`** *(cfg `pq-hybrid`)* — signature envelope verified via `SigAdapter` (classical vs PQ); ensure decision parity and record perf delta (no hard gate here).

---

### 1.3 Property-Based Tests

**Tooling:** `proptest` (preferred) or `quickcheck`.
**Run:**

```bash
cargo test -p ron-auth --lib --features proptest -- --nocapture
```

**Invariants & strategies:**

* **No panics** for any input up to `max_token_bytes` and `max_caveats` (fuzzable byte vectors constrained by config).
* **Linear slope**: latency scales ~linearly with caveat count (assert within a tolerance window in debug metrics mode, optional).
* **MAC integrity**: flipping any byte of payload or tag ⇒ `mac.mismatch` (never `Allow`).
* **Order sensitivity**: reordering caveats changes MAC chain ⇒ **deny** (`mac.mismatch` or related); order-preserving replay with same inputs ⇒ **allow**.
* **Schema stability**: adding unknown top-level fields ⇒ `schema.unknown_field` (stable reason).

> Generators produce realistic caveat sets (`exp`, `nbf`, `path`, `tenant`, and random custom namespaces). Shrinkers bias toward minimal counterexamples (1-byte flip, 1 unknown field, etc.).

---

### 1.4 Fuzz Tests

**Tooling:** `cargo-fuzz` with ASan/UBSan where available.
**Targets & goals:**

* `token_parser_fuzz` — arbitrary bytes → verify parse path; **no panics/OOM**; must terminate within time budget.
* `b64_decode_fuzz` — malformed Base64URL cases (padding/URL alphabet).
* `cbor_parse_fuzz` — indefinite-length items, nested maps/arrays, truncated items.
* `custom_caveat_name_fuzz` — untrusted namespace bytes → deny without panic.
* *(optional)* `pq_envelope_fuzz` (cfg `pq-hybrid`) — corrupted envelopes ⇒ deny safely.

**Corpus:** seed with golden negatives (oversize, truncated, invalid schema) and real-world samples (redacted).
**Acceptance:** CI quick pass (≤60s) per PR; **nightly** job 1h (Silver) / 4h (Gold) with **0 crashes**.

**Run:**

```bash
cargo fuzz run token_parser_fuzz -- -max_total_time=60
```

---

### 1.5 Chaos / Soak (host-embedded)

`ron-auth` has no processes/FDs to soak; validate **via a tiny host harness** (under `testing/harness/`) that wraps `verify()` and exposes `/metrics`:

Inject in the harness:

* **Key rotation churn:** hot-swap KID windows; assert clean `kid.unknown` denies (no panics).
* **Hostile input replay:** malformed Base64URL/CBOR; confirm `parse.*` reasons.
* **Amnesia toggling:** enable/disable amnesia; tokens with `Amnesia(true)` obey policy.

**Acceptance:** 30–60 min soak ⇒ zero panics, steady memory, stable deny taxonomy.

**Run (example):**

```bash
cargo test -p ron-auth -- --ignored runbook_chaos_hot_swap_kid
cargo test -p ron-auth -- --ignored runbook_chaos_hostile_inputs
cargo test -p ron-auth -- --ignored runbook_chaos_amnesia_mode
```

---

### 1.6 Performance / Load

Benchmarks live under `benches/` (Criterion). They enforce PERF SLOs and detect regression slope with caveat count.

**Benches:**

* `verify_small_token` (1–3 caveats; cold/warm).
* `verify_many_caveats` (16/32/64; check linear slope).
* `sig_adapter_delta` *(cfg `pq-hybrid`)* — record delta vs classical.

**Run:**

```bash
cargo bench -p ron-auth
```

---

## 2) Coverage & Gates

### 2.1 Bronze (MVP)

* Unit + integration tests pass.
* Code coverage **≥ 70%** (line).
* Fuzz harness **builds**; at least one corpus seeded.
* Golden vectors present (allow + deny) with reason parity checks.

### 2.2 Silver (Useful Substrate)

* Property tests in place for parse/MAC invariants.
* Fuzz run ≥ **1h** nightly; **0 crashes**.
* Coverage **≥ 85%**.
* Chaos harness scripted; three drills implemented as `--ignored` tests.

### 2.3 Gold (Ops-Ready)

* Fuzz run ≥ **4h** nightly; **0 crashes**.
* Soak/chaos 60–120 min in CI harness; **no leaks**; deny taxonomy stable.
* Coverage **≥ 90%** (line) and **≥ 85%** (branch) on stable rustc.
* Perf regression tracked release-to-release; ≤10% tolerated unless waived.

---

## 3) Invocation Examples

### 3.1 All tests (crate)

```bash
cargo test -p ron-auth --all-targets -- --nocapture
```

### 3.2 Specific integration suite

```bash
cargo test -p ron-auth --test allow_deny_vectors -- --nocapture
```

### 3.3 Fuzz quick pass (local)

```bash
cargo fuzz run token_parser_fuzz -- -max_total_time=60
```

### 3.4 Benchmarks

```bash
cargo bench -p ron-auth
```

### 3.5 Golden vectors replay

Vectors live under:

```
testing/vectors/ron-auth/v1/
  allow_*.json
  deny_*.json
  pq_*.json           # only when feature pq-hybrid is enabled
```

Each vector MUST include:

```json
{
  "tenant": "acme",
  "kid": "ed25519:2025-09-01",
  "key_hex": "… (for test-only determinism) …",
  "token_b64": "eyJ…",
  "ctx": { "amnesia": false, "path": "/o/foo", "now": 1736037000 },
  "expected": { "decision": "allow" }
}
```

or for denies:

```json
{ "expected": { "decision": "deny", "reason": "parse.cbor" } }
```

**Rules:** reason strings are **stable**; tests must match **exact** text.

---

## 4) Observability Hooks (tests)

* Integration & chaos harness must emit **structured JSON** on failures: include `result`, `reason`, `corr_id`, redacted `token_digest8`.
* If using the host harness, export Prometheus counters/histograms named:

  * `ron_auth_verify_total`
  * `ron_auth_deny_total{reason}`
  * `ron_auth_parse_error_total{kind}`
  * `ron_auth_unknown_kid_total`
  * `ron_auth_duration_us_bucket`
  * `ron_auth_token_size_bytes`

Dashboards should show:

* Deny taxonomy distribution over time.
* Unknown KID spikes during rotation drills.
* p95 duration vs caveat count slope.

---

## 5) CI Enforcement

**Minimum:**

* PR: `cargo test -p ron-auth --all-targets`
* Lints/format: `cargo fmt -- --check && cargo clippy --no-deps -D warnings`
* Advisories: `cargo deny check advisories bans licenses sources`

**Coverage (choose one):**

* `cargo llvm-cov --lcov --output-path lcov.info` (recommended)
* `cargo tarpaulin --out Xml`

**Fuzz (nightly):**

* Quick 60s per target on PR label `fuzz-ok`.
* Nightly 1h (Silver) / 4h (Gold) with artifact upload of new crashing inputs (should be none) and corpus growth.

**Perf guard (optional but recommended):**

* Run Criterion; compare to `testing/performance/baselines/ron-auth/` and fail on >10% regression.

**Chaos harness (quarterly):**

* Run `--ignored` drills from §1.5 on a schedule (Gate J/K).

---

## 6) Open Questions / Per-Crate Decisions

* **Loom usage:** None at lib level (no internal concurrency). Loom reserved for host hot-swap tests (e.g., ArcSwap of providers) in host repos.
* **Mandatory fuzz targets:** `token_parser_fuzz`, `cbor_parse_fuzz`, `b64_decode_fuzz` (**required**); `custom_caveat_name_fuzz` (**recommended**); `pq_envelope_fuzz` (**when feature enabled**).
* **Perf SLOs (cross-ref PERF.md):** p95 ≤ **60 µs + (8 µs × caveats)** at 4 KiB tokens (conservative). Track host-parallel throughput (8-core ≥ **1.0M verifies/s**).

---

## 7) Repository Layout (tests & tooling)

```
crates/ron-auth/
├─ src/
│  └─ … (unit tests inside modules with #[cfg(test)])
├─ tests/
│  ├─ allow_deny_vectors.rs
│  ├─ config_bounds.rs
│  ├─ rotation_window.rs
│  ├─ custom_caveats.rs
│  ├─ amnesia_mode.rs
│  └─ pq_envelope.rs            # cfg(feature = "pq-hybrid")
├─ benches/
│  ├─ verify_small_token.rs
│  ├─ verify_many_caveats.rs
│  └─ sig_adapter_delta.rs      # cfg(feature = "pq-hybrid")
├─ fuzz/
│  ├─ fuzz_targets/
│  │  ├─ token_parser_fuzz.rs
│  │  ├─ cbor_parse_fuzz.rs
│  │  ├─ b64_decode_fuzz.rs
│  │  └─ custom_caveat_name_fuzz.rs
│  └─ Cargo.toml
└─ testing/
   ├─ vectors/ron-auth/v1/*.json
   ├─ performance/baselines/ron-auth/*.json
   └─ harness/ (optional mini-host for chaos/soak)
```

---

## 8) Bronze→Silver→Gold Acceptance Gate Checklist

**Bronze**

* [ ] Unit + integration pass; golden vectors implemented.
* [ ] Coverage ≥ 70% (line); fuzz harness compiles.

**Silver**

* [ ] Property tests for parse/MAC invariants.
* [ ] Nightly fuzz 1h (0 crashes); chaos harness scripted.
* [ ] Coverage ≥ 85% (line).
* [ ] Perf guard wired; baselines stored.

**Gold**

* [ ] Nightly fuzz 4h (0 crashes); soak/chaos 60–120 min clean.
* [ ] Coverage ≥ 90% (line), ≥ 85% (branch).
* [ ] Perf tracked per release; ≤10% regression or approved waiver.
* [ ] Quarterly drills (rotation, hostile inputs, amnesia) logged and linked in RUNBOOK.

---

✅ This testing contract makes `ron-auth` **reproducible, falsifiable, and drift-resistant** across contributors and CI. It aligns with your invariants (bounds, stable reasons, pure-lib ethos) and keeps ops hooks (metrics/chaos) with the host wrappers where they belong.

```
```
