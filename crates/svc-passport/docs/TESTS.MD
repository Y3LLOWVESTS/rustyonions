````markdown
# 🧪 TESTS.md — svc-passport

*Audience: developers, auditors, CI maintainers*  
*msrv: 1.80.0 (Tokio/loom compatible)*

---

## 0) Purpose

Define the **testing contract** for `svc-passport` (Pillar 3 Identity): unit, integration, property, fuzz, chaos/soak, and performance tests, with explicit coverage goals and Bronze→Silver→Gold acceptance gates. Tests assert invariants from `IDB.md`, SLOs from `PERFORMANCE.md` (issuance p95 ≤ 40 ms; verify p95 ≤ 10 ms; revocation p99 ≤ 5 s; cold-start p95 ≤ 150 ms), and hardening caps from `SECURITY.md`.

---

## 1) Test Taxonomy

### 1.1 Unit Tests

**Scope:** Pure logic and small components (<100 ms).  
**Examples (non-exhaustive):**
- Passport claims/caveats validation (TTL, audience, issuer).
- Macaroon/“passport” envelope encode/decode helpers.
- Revocation set diffs & epoch bump math.
- Config parsing + defaults (limits.rps, pq.mode, cache TTL).
- Error taxonomy mapping (HTTP 4xx/5xx).

**Location:** inline `#[cfg(test)]` in source files under `src/`.  
**Command:**
```bash
cargo test -p svc-passport --lib
````

---

### 1.2 Integration Tests

**Scope:** End-to-end service surface in `tests/*.rs`.
**Must include:**

* **API round-trips:** `/v1/passport/issue`, `/verify`, `/revoke` (happy/edge: expired, bad audience, oversized body → 413, malformed JSON).
* **Config reload semantics:** SIGHUP or dynamic reload toggles (`limits.rps`, `pq.mode`, cache TTL) apply without restart (see CONFIG.md).
* **Concurrency invariants:** graceful shutdown completes inflight; bounded issuance semaphore prevents stampedes; backpressure returns 429/503 not timeouts (see CONCURRENCY.md).
* **Revocation propagation:** publish revocation/epoch on bus → verify refuses within p99 ≤ **5 s**.
* **PQ mode transitions:** `mirror → require → enforce` do not break interop; classical-only verifiers never accept PQ-only envelopes.

**Location:** `tests/`:

* `http_issue_verify.rs`
* `config_reload.rs`
* `concurrency_backpressure.rs`
* `revocation_bus.rs`
* `pq_modes.rs`

**Command:**

```bash
cargo test -p svc-passport --test '*'
```

---

### 1.3 Property-Based Tests

**Scope:** Codecs, parsers, policy evaluators, small state machines.
**Tooling:** `proptest` (preferred).

**Invariants:**

* **No panics** on arbitrary inputs (envelope parser, caveat list up to cap).
* **Round-trip:** `decode(encode(x)) == x` for passports (modulo normalization).
* **Idempotency:** verifying the same passport twice with same context is stable.
* **Monotonic revocation:** once revoked at epoch N, verification at epoch ≥ N must refuse.

**Location:** `tests/prop_*` or `src/*/mod.rs` with `proptest!`.
**Examples:**

* `prop_envelope_roundtrip.rs`
* `prop_revocation_monotonic.rs`

**Command:**

```bash
cargo test -p svc-passport --features proptest
```

---

### 1.4 Fuzz Tests

**Scope:** External-facing decoders/handlers and JSON bodies.
**Tooling:** `cargo fuzz` (libFuzzer).

**Targets (minimum):**

* `fuzz_targets/envelope_parse.rs` — random bytes → passport parser.
* `fuzz_targets/http_issue_body.rs` — mutated JSON → issue-request validator.
* `fuzz_targets/verify_context.rs` — headers/claims/caveats combiner.
* **PQ Hybrid Target:** `fuzz_targets/hybrid_sig_verify.rs` — mutates classical+PQ signature blobs and key IDs.

  * No panics on malformed hybrids.
  * `mirror/require/enforce` never accept malformed hybrids.
  * Classical-only verifiers refuse PQ-only envelopes.

**Corpus:**

* Seed with minimized failures from CI + samples from integration tests (`testing/fuzz/corpus/*`).
* **Dictionary** `fuzz/dictionaries/passport.dict` includes:

  ```
  aud="..."
  exp=1234567890
  nbf=1234567890
  caveats="..."
  sig="..."
  kid="..."
  pq="mirror"
  pq="require"
  pq="enforce"
  ```

**Acceptance:**

* CI (Silver): ≥ **1h** fuzz each selected target; zero crashes.
* Nightly (Gold): ≥ **4h**; zero crashes, OOMs, or timeouts.

**Commands:**

```bash
cargo fuzz run envelope_parse -- -max_total_time=3600
cargo fuzz run http_issue_body -- -max_total_time=3600
cargo fuzz run verify_context -- -max_total_time=3600
cargo fuzz run hybrid_sig_verify -- -max_total_time=3600
```

---

### 1.5 Chaos / Soak Tests

**Scope:** Service behavior under faults and time.

**Inject:**

* **Process crashes:** send SIGKILL; service restarts cleanly; emits `ServiceCrashed`.
* **Bus lag/drops:** delayed revocation events; no deadlocks; propagation p99 ≤ **5 s** after recovery.
* **KMS latency:** +50/100/200 ms; issuance p95 rises but bounded; no unbounded queue growth.
* **Ingress slow-loris / oversized body:** early **429** and **413** (body cap **1 MiB**).
* **24h soak:** memory RSS slope **< 2%/24h**; FD count stable.

**ARM/Edge Variant**

* Run soak on ARM64 node; record cold-start delta (expect ≤ **+50 ms**) and RSS slope.
* Record **power/thermal** hints if available (RAPL/`perf stat` on x86, SoC sensors on ARM).
* Gate (advisory): no throttling events during 10m sustained load at reference RPS; document if unavoidable.
* Gate: ARM soak passes with same leak thresholds.

**Inter-Region Simulation (WAN)**

* Use `tc/netem` to inject +50 ms / +100 ms RTT on verify paths.

  * Expect: verify p95 ≤ **120 ms**, p99 ≤ **200 ms**; no queue growth; no false alerts.

**Harness:** `testing/chaos/*` scripts (Docker/K8s ready).

---

### 1.6 Performance / Load Tests

**Scope:** Throughput, latency, quotas per `PERFORMANCE.md`.

**Workloads:**

* Issuance heavy (80/20 issue/verify) — steady + spike.
* Verify heavy (95/5) — intra-AZ and inter-region (WAN).
* PQ modes: `mirror` vs `enforce`.

**SLO checks (4c/8GB reference):**

* Issuance p95 ≤ **40 ms**, p99 ≤ **100 ms**.
* Verify p95 ≤ **10 ms**, p99 ≤ **25 ms**.
* Revocation propagation p99 ≤ **5 s**.
* Cold-start p95 ≤ **150 ms**.

**Profiles & Scripts:**

* `testing/performance/profiles/issue_80_20_local.json`
* `testing/performance/profiles/verify_95_5_wan.json` (adds `tc` RTT)
* `testing/performance/profiles/pq_enforce_issue.json`
* Baselines JSON in `testing/performance/baselines/*.json`

**Commands:**

```bash
cargo bench -p svc-passport
bash testing/performance/run_profiles.sh profiles/verify_95_5_wan.json
```

---

## 2) Coverage & Gates

**Coverage Tooling (branch coverage):**

```bash
RUSTFLAGS='-Cinstrument-coverage' LLVM_PROFILE_FILE='svc-%p-%m.profraw' cargo test -p svc-passport
grcov . -s . --binary-path ./target/debug/ -t lcov --branch -o lcov.info
```

**Gate Table**

| Tier   | Line Cov | Branch Cov | Fuzz Time       | Soak          | Perf Gates                                     | Tools                            |
| ------ | -------- | ---------- | --------------- | ------------- | ---------------------------------------------- | -------------------------------- |
| Bronze | ≥70%     | ≥60%       | build ok        | optional      | none                                           | grcov/tarpaulin, cargo-fuzz      |
| Silver | ≥85%     | ≥75%       | ≥1h/target (CI) | scripted      | sanity profiles pass                           | grcov (branch), cargo-fuzz       |
| Gold   | ≥90%     | ≥85%       | ≥4h/nightly     | 24h **clean** | CI fails on p95 ↑>10%, thrpt ↓>10%, cold ↑>20% | grcov, cargo-fuzz, cargo-mutants |

Additionally (Gold): critical modules coverage minima — `issue` ≥ **95%**, `verify` ≥ **95%**, `revocation` ≥ **95%**, `config` ≥ **90%**, `errors` ≥ **90%**.

---

## 3) Invocation Examples

### 3.1 All Tests (crate)

```bash
cargo test -p svc-passport --all-targets -- --nocapture
```

### 3.2 Specific Integration

```bash
cargo test -p svc-passport --test http_issue_verify -- --ignored
```

### 3.3 Fuzz Target (local quick)

```bash
cargo fuzz run envelope_parse -- -max_total_time=60
```

### 3.4 Loom (concurrency model checks)

```bash
RUSTFLAGS="--cfg loom" cargo test -p svc-passport --test loom_*
```

### 3.5 Benches

```bash
cargo bench -p svc-passport
```

---

## 4) Observability Hooks

* Tests emit **structured JSON logs** with `corr_id`, `endpoint`, `mode` (pq/cache), timings, and key decisions; failures attach the last 50 log lines.
* Perf runs scrape Prometheus; dump metrics snapshots into `target/test-artifacts/*`; attach Grafana snapshot links to CI artifacts.

**Alert Firing (canaries)**

* Integration harness can intentionally raise issuance p95 over threshold for ~5m to assert:

  * `PassportIssueLatencyHigh` fires and includes a link to RUNBOOK §6.
  * Burn-rate **fast** (2h ≥ 14×) triggers; **slow** (24h ≥ 6×) does not.
* Mark these tests `#[ignore]`; run in nightly only.

**Burn-rate PromQL (examples)**

```
(sum(rate(http_requests_total{job="svc-passport",status=~"5..|429|503"}[2h])) /
 sum(rate(http_requests_total{job="svc-passport"}[2h]))) / 0.001 > 14

(sum(rate(http_requests_total{job="svc-passport",status=~"5..|429|503"}[24h])) /
 sum(rate(http_requests_total{job="svc-passport"}[24h]))) / 0.001 > 6
```

*Adjust SLO budget (0.001 = 0.1%) and labels to your metrics.*

---

## 5) CI Enforcement

**GitHub Actions job matrix (summary):**

* `test`: `cargo test --workspace --all-targets` (stable + nightly).
* `fmt`: `cargo fmt -- --check`; `clippy -D warnings`.
* `deny`: `cargo deny check advisories bans sources licenses`.
* `coverage`: instrumented run + `grcov --branch`; enforce gate table thresholds.
* `coverage-alt`: `cargo tarpaulin --engine llvm --out Xml` (sanity cross-check).
* `fuzz-nightly`: selected fuzzers with `-max_total_time=14400`.
* `perf-nightly`: `testing/performance/run_profiles.sh`; compare to baselines; fail on regressions.
* `perf-wan`: run `verify_95_5_wan.json` with `tc/netem`; upload Grafana snapshot + JSON results.
* `chaos-weekly`: fault injection on small K8s cluster (KMS latency, slow-loris, bus lag).
* `mutation-gold`: run mutation tests on guarded modules; **fail if any survivors**.

**Mutation CI (example):**

```bash
cargo mutants --workspace --no-shuffle --in-place --exclude-tests \
  -p svc-passport -v --timeout 120 --threads $(nproc)
test -f target/mutants/surviving.txt && exit 1 || true
```

---

## 6) svc-passport Specifics

**Loom-checked invariants (in `tests/loom_*`):**

* **Bounded issuance semaphore:** never exceeds configured max; no permit leaks on cancellation.
* **Graceful shutdown:** all tasks observe shutdown signal; no events after `Shutdown`.
* **Backpressure path:** under load, handler returns 429 quickly; no unbounded queue growth.
* **Revocation race:** verify vs revocation ordering never permits “accept after revoke”.

**Mandatory fuzz targets:**

* `envelope_parse`, `http_issue_body`, `verify_context`, `hybrid_sig_verify`.

**Performance SLOs measured:**

* Issuance p95/p99, Verify p95/p99, Revocation p99, Cold-start p95.
* Error budgets: 5xx < **0.1%**, 429/503 < **1%** under spike.
* Soak slope: RSS < **2%/24h**, FD stable ±5.

**Proptest Strategies:**

* Caveats: `prop::collection::vec(caveat_strategy(), 0..=CAP)`.
* `exp/nbf` jitter: ±(0..=120 s) around now to stress boundaries.
* Envelope sizes: ladder at **[1 KiB, 8 KiB, 64 KiB, 512 KiB, 1 MiB]**.

**Mutation Testing (Gold — REQUIRED):**

* Tool: `cargo-mutants` (preferred) or `mutagen`.
* Scope: guards in `issue`, `verify`, `revocation` (aud/exp/nbf checks; epoch/revocation enforcement).
* Gate: **zero surviving mutants** across scoped modules (enforced by `mutation-gold` CI job).

**Test data size limits:**

* Max envelope/body fuzz size: **1 MiB** (mirrors ingress cap).
* Caveats per passport capped at policy limit; strategies cover `[0, cap]`.

---

## 7) Layout & Artifacts

```
svc-passport/
├─ benches/
│  └─ passport_benches.rs
├─ tests/
│  ├─ http_issue_verify.rs
│  ├─ config_reload.rs
│  ├─ concurrency_backpressure.rs
│  ├─ revocation_bus.rs
│  ├─ pq_modes.rs
│  ├─ prop_envelope_roundtrip.rs
│  ├─ prop_revocation_monotonic.rs
│  └─ loom_shutdown.rs
├─ fuzz/
│  ├─ fuzz_targets/
│  │  ├─ envelope_parse.rs
│  │  ├─ http_issue_body.rs
│  │  ├─ verify_context.rs
│  │  └─ hybrid_sig_verify.rs
│  └─ dictionaries/
│     └─ passport.dict
└─ testing/
   ├─ performance/
   │  ├─ run_profiles.sh
   │  ├─ profiles/
   │  │  ├─ issue_80_20_local.json
   │  │  ├─ verify_95_5_wan.json
   │  │  └─ pq_enforce_issue.json
   │  └─ baselines/
   │     └─ issuance-4c8g.json
   └─ chaos/
      ├─ inject_kms_latency.sh
      ├─ slow_loris.sh
      ├─ bus_lag.sh
      └─ wan_tc.sh
```

---

## 8) Developer Tips

* Reproduce flakies:

  ```bash
  cargo test -p svc-passport -- --test-threads=1 --nocapture RUST_LOG=info,svc_passport=debug
  ```
* Simulate WAN locally:

  ```bash
  sudo tc qdisc add dev lo root netem delay 50ms 5ms distribution normal
  # run tests / perf profile
  sudo tc qdisc del dev lo root
  ```
* Quick branch coverage (HTML):

  ```bash
  RUSTFLAGS='-Cinstrument-coverage' LLVM_PROFILE_FILE='svc-%p-%m.profraw' cargo test -p svc-passport
  grcov . -s . --binary-path ./target/debug/ -t html --branch -o target/coverage/html
  ```
* Minimize fuzz corpora before committing:

  ```bash
  cargo fuzz cmin envelope_parse
  cargo fuzz tmin envelope_parse
  ```

---

✅ This contract is **auditable, reproducible, and enforceable**. CI integrates these gates—coverage, fuzz/chaos, perf, mutation—to prevent silent drift and uphold `PERFECTION_GATES` A/F/J/K/L for a critical identity service.

```
```
