### BEGIN NOTE - OCTOBER 30 2025 - 14:30 CST

---

# Carryover Notes — from `svc-index` to `ron-policy`

**Date:** 2025-10-30 (America/Chicago)
**Context:** RON-CORE phase (no Web3). Goal for ron-policy: centralized policy decision + enforcement primitives that other crates (gateway, overlay, index, storage, etc.) can consume uniformly.

## 1) Contracts & Semantics to Keep

* **400 vs 404 discipline**

  * Malformed input → **400** (client error, do not retry).
  * Well-formed miss → **404** (cacheable negative, short TTL).
  * Hit → **200**.
    ron-policy should mirror this split for decision endpoints (e.g., bad subject/resource/action → 400; rule evaluations that result in “deny because not found” → 404 when appropriate, or 403 when found and explicitly denied).
* **DTO stability**

  * Keep field names stable; on 404 return the **same body shape** as 200 (just the status changes). This prevented frontend churn in svc-index and should be the default in ron-policy’s decision APIs.

## 2) Input Validation Patterns

* **Strict, fast validators**

  * svc-index used tight `b3:<64-hex>` checks and returned 400 early.
  * For ron-policy: validate `subject`, `resource`, `action`, `tenant`, `role`, `scope` with small, branch-predictable checks (length caps, char classes). Reject early with 400 to avoid wasted work.
* **Permissive keys, strict IDs**

  * As with `resolve/:key`, some surfaces can be permissive; identity and policy IDs must be strict. Document which is which.

## 3) Error Taxonomy (SvcError → HTTP)

* Keep a compact enum and a single mapping layer:

  * `BadRequest(_)` → 400
  * `NotFound` → 404
  * `Forbidden` → 403 (ron-policy specific; svc-index didn’t need it)
  * `Conflict` → 409 (policy updates)
  * `Internal(_)` → 500
* Add an error kind label for metrics (`err_kind="bad_request|not_found|forbidden|internal"`), like we did for request latency.

## 4) Observability Blueprint (copy this shape)

* **Endpoints**: `/healthz`, `/readyz`, `/version`, `/metrics`.
* **Truthful readiness**: do not return 200 on `/readyz` until the policy engine is warmed (rules parsed, stores opened, caches seeded).
* **Metrics**: reuse names and patterns:

  * `request_latency_seconds` (histogram) for HTTP handlers
  * `policy_evaluations_total` (counter) with labels `{decision="allow|deny|error"}`
  * Optional: `compilation_seconds` histogram for policy bundle loads; `inflight_requests` gauge.
* **No cardinality bombs**: keep labels bounded (tenant, decision, route). Avoid per-user labels.

## 5) Middleware & Auth Patterns to Reuse

* **Header-based guard layer**

  * svc-index’s `AdminTokenLayer` is a working pattern. For ron-policy, generalize to:

    * `X-Admin-Token` for admin plane (off by default).
    * `Authorization: Bearer <jwt>` for PDP calls (feature-gated, minimal dependency surface).
* **Trace + body limits**

  * Keep the `TraceLayer` and a `ContentLengthLimit` equivalent; policy requests can be small but must be bounded.

## 6) Config Surface Consistency

* Mirror env naming from svc-index:

  * `POLICY_BIND` (addr:port)
  * `RON_POLICY_DB` or `POLICY_STORE_URL` (sled/sqlite/postgres depending on choice)
  * `POLICY_ADMIN_TOKEN` (if admin plane enabled)
* Keep your `Config::load()` pattern and inject state via `.with_state(Arc<AppState>)`.

## 7) Storage & Durability (lessons from sled)

* **Flush discipline**

  * If using sled or sqlite for policy bundles/decisions cache: call `.flush()` (sled) or ensure `WAL` and `PRAGMA synchronous=NORMAL` (sqlite) on writes that must survive a quick restart.
* **Dual store plan**

  * Provide an `inmem` feature (like svc-index) for tests and a persistent store for prod.

## 8) Performance Guardrails (what worked)

* **Single-ms hot paths**

  * svc-index achieves ~1–2 ms avg with 60–70k rps on loopback for trivial endpoints.
  * For ron-policy, target: p50 ≤ 2 ms, p99 ≤ 10 ms on decision routes with a warm cache (compile once, evaluate fast).
* **Bench harness**

  * Reuse the `wrk/hey` release script layout and add a Criterion bench for **pure decision evaluation** using a mock store. Report ops/sec for allow/deny checks without HTTP.

## 9) API Shape Suggestions (PDP + Admin)

* **Public (PDP)**

  * `POST /v1/decision` → `{subject, action, resource, context?}` → `{decision:"allow|deny", reasons:[], etag:null}`
  * `GET /v1/decision?subject=…&action=…&resource=…` (idempotent/read-friendly)
  * 400 malformed, 403 forbidden (policy matched deny), 404 not found (no policy applies), 200 allow.
* **Admin (opt-in, token-guarded)**

  * `POST /admin/policy` (create/update bundle)
  * `GET /admin/policy/:id` (fetch)
  * `POST /admin/reload` (force reload)
  * Return 409 if version/hash conflicts; return 202 for async compiles if you add background loading.

## 10) Caching & ETags

* **Negative caching**

  * As with svc-index 404 bodies, allow gateways to cache 404 and 403 for short TTLs.
* **ETag**

  * Compute ETags over compiled policy bundle hash and surface in decision responses; honor `If-None-Match` → 304 for static fetches (e.g., policy listing).
  * Add `etag` field in decision responses for possible replay protection and audit correlation.

## 11) Testing Strategy (copy what paid off)

* **Smoke script**: run fmt → clippy → boot → hit `healthz/readyz/version` + 3 decision cases (400 malformed, 404 no applicable policy, 200 allow).
* **Handler tests**: 3–5 focused tests to assert JSON body stability on 404/403/200.
* **Bench (release)**: the `wrk --latency` trio plus a Criterion microbench for `policy::evaluate()`.

### Commands (no comments)

Run (debug):

```
POLICY_BIND=127.0.0.1:5310 cargo run -p ron-policy
```

Smoke pattern:

```
cargo fmt -p ron-policy
cargo clippy -p ron-policy --no-deps -- -D warnings
cargo build -p ron-policy
```

## 12) Security & Hardening

* **Never log secrets**: scrub headers like `X-Admin-Token`, `Authorization`.
* **Feature-gate JWT**: keep deps minimal; parse only the fields you need.
* **Rate-limit** public PDP endpoints; default off, documented on.
* **Deterministic evaluation**: no time-of-check/time-of-use surprises—snapshot inputs, avoid async calls mid-decision.

## 13) Docs & DX Consistency

* README sections to replicate from svc-index:

  * “How to run” (envs)
  * “HTTP contract” (400/403/404/200 table)
  * “Observability” (endpoints + metric names)
  * “Performance baseline” (post first wrk run)
* Keep RO headers (`RO:WHAT/WHY/INVARIANTS/INTERACTS`) on all modules.

## 14) Acceptance Criteria for ron-policy (Beta)

* Control plane: `healthz/readyz/version/metrics`
* PDP endpoints: 400/403/404/200 semantics locked with stable JSON
* In-mem store + one persistent store, toggled by feature
* Clippy clean with `-D warnings`
* Smoke script passes; `wrk --latency` recorded (p50/p99 targets hit)
* Minimal admin plane behind token (optional but recommended)

---

**Bottom line:** carry forward the **strict input validation**, **truthful readiness**, **400/404 contract discipline**, **bounded metrics labels**, and the **smoke/bench workflow** from svc-index. Ron-policy’s unique adds are the **403/decision model**, **ETag/versioning for bundles**, and a **fast, deterministic evaluator** with the same single-ms ambitions.

### END NOTE - OCTOBER 30 2025 - 14:30 CST

### BEGIN NOTE - OCTOBER 30 2025 - 15:03 CST

Boom—green across the board 🎯

Here’s a tight wrap + immediate next steps to make ron-policy “plug-n-play” with the services:

## Quick victory lap

* `fmt`, `clippy (pedantic/nursery)`, **all tests**: ✅
* Snapshot test replaced with deterministic asserts: ✅
* TOML loader: ✅ (UTF-8 → `toml::from_str`)
* Clippy hygiene (docs, must_use, LazyLock, etc.): ✅

## Sanity extras (optional to run now)

```
cargo run -p ron-policy --example minimal_allow_deny
cargo bench -p ron-policy
```

## How to embed in a service (svc-index / svc-gateway / svc-storage)

### 1) Load + validate bundle once at startup

```rust
use ron_policy::{load_json, Evaluator};
let bytes = std::fs::read("policy.bundle.json")?;
let bundle = load_json(&bytes)?;
let evaluator = Evaluator::new(&bundle)?;
```

### 2) Evaluate per request

```rust
use ron_policy::{Context, ctx::clock::SystemClock};
let clock = SystemClock::default();
let ctx = Context::builder()
    .tenant(tenant_id)        // "acme", or "*"
    .method(req.method().as_str())
    .region(region_code)      // "US", "US-FL", "EU", or "*"
    .body_bytes(body_len)     // u64
    .build(&clock);

let decision = evaluator.evaluate(&ctx)?;
match decision.effect {
    ron_policy::engine::eval::DecisionEffect::Allow => {
        // apply decision.obligations (if any) in your service layer
        // proceed…
    }
    ron_policy::engine::eval::DecisionEffect::Deny => {
        // map to 403/451/etc; surface decision.reason minimally
    }
}
```

### 3) Expose metrics (already registered)

Just ensure your Prometheus exporter is up (svc-* already have it). You’ll see:

* `policy_requests_total`
* `policy_rejected_total{reason="rule_deny|body_too_large|default_deny"}`
* `policy_eval_latency_seconds`

## What to do next (fast wins)

1. **Dev-UX sugar (optional small CLI):** a `ron-policy-cli` that does `eval <bundle.json> <ctx.json>` and prints allow/deny + obligations + trace. Shout if you want me to drop this file in (~80 LOC).
2. **Wiring obligations:** In svc-gateway/svc-index, add tiny interpreters for obligations you care about first:

   * `add-header` (k/v)
   * `log-audit` (string key)
   * `mask-field` (for structured payloads)
3. **Golden vectors:** Add a few more test bundles under `tests/vectors/`:

   * `quota_deny.json` (tag-based throttles; we’ll just evaluate the decision—actual counters live in the service)
   * `residency_eu_only.json`
   * `method_matrix.json` (mix GET/PUT/POST)
4. **Hot-reload (later):** your service can watch the bundle file and `Evaluator::new(&new_bundle)` on change—no changes to ron-policy needed.
5. **Schema gate in CI (optional):** If you already use `cargo-deny` and a JSON schema runner, validate the bundle against `schema/policybundle.schema.json` before baking it into images.

## Manual quick test (copy/paste friendly)

`policy.bundle.json`:

```json
{
  "version": 1,
  "defaults": { "max_body_bytes": 1048576 },
  "rules": [
    { "id": "deny-fl", "when": { "region": "US-FL" }, "action": "deny", "reason": "region denied" },
    { "id": "allow",   "when": {}, "action": "allow", "reason": "fallback allow" }
  ]
}
```

Then:

```
cargo run -p ron-policy --example minimal_allow_deny
```

Edit the example to `.region("US-FL")` to see a deny.

### END NOTE - OCTOBER 30 2025 - 15:03 CST

### BEGIN NOTE - OCTOBER 30 2025 - 16:34 CST
---

# Notes — `ron-policy`

**Date:** 2025-10-30 (America/Chicago)
**Status:** `0.1.0-beta.1` ✅
**Scope:** RON-CORE (no Web3 deps), library-only policy engine with strict DTOs, validator, indexed evaluator, Prometheus metrics, and explainable trace.

---

## 1) What we accomplished (Beta deliverables)

### Design & Invariants

* **Deny-by-default** policy model with explicit **defaults** and **rules**; first-match wins.
* **Strict DTOs**: `#[serde(deny_unknown_fields)]` across all model types → rejects unknown keys (auditability).
* **Stable enums & versioned bundle** (`version: u32`) to prevent silent drift.
* **Explainability**: every decision carries a deterministic **reason** and **trace** (rule hits/misses).
* **Observability**: Prometheus metrics for volume, denials by reason, and latency histogram.
* **No runtime time deps**: a `Clock` trait with `SystemClock` using `std::time` (services may inject others).

### Public API (crate surface)

* **Model:** `PolicyBundle`, `Rule`, `RuleCondition`, `Action`, `Obligation`, `Meta`.
* **Parsing/Validation:**

  * `load_json(&[u8]) -> Result<PolicyBundle, Error>`
  * `load_toml(&[u8]) -> Result<PolicyBundle, Error>`
  * `parse::validate::validate(&PolicyBundle) -> Result<(), Error>`
* **Evaluation:**

  * `Context::builder() -> ContextBuilder` (tenant/method/region/body_bytes/tags)
  * `Evaluator::new(&PolicyBundle) -> Result<Evaluator, Error>`
  * `Evaluator::evaluate(&Context) -> Result<Decision, Error>`
  * `Decision { effect: Allow|Deny, reason: Option<String>, trace: Trace }`
* **Indexing:** `engine::index::RuleIndex` (method-based candidate pruning).
* **Metrics:** `requests_total`, `rejected_total{reason}`, `eval_latency_seconds` (Histogram).
* **Errors:** Stable taxonomy, human-safe, no secret leakage.

### Performance (local baseline)

* **Bench:** `eval:get/us` ~ **1.17 µs/eval** (2019 MBP class), with **trace + Prometheus** instrumentation enabled.
* That’s ~**0.85–0.9 million eval/s/core**; more headroom with trace/metrics gating later.

### Tests & Examples shipped

* **Unit tests** (all green):

  * `unit_model_serde_strict.rs` — strict DTOs, minimal round-trip.
  * `unit_eval_determinism.rs` — default deny path.
  * `unit_churn_protection.rs` — geo/region deny vector.
  * `unit_tighten_only.rs` — defaults caps.
  * `unit_method_matrix.rs` — method index sanity.
  * `unit_tags_all.rs` — require_tags_all semantics.
  * `unit_large_body_default_deny.rs` — default body cap deny.
  * `unit_eu_only.rs` — allow region EU, default deny otherwise.
  * `unit_first_match_wins.rs` — first-match precedence; default fallback.
* **Golden**:

  * `golden_reasons.rs` — stable explain trace snapshot.
* **Vectors** (under `tests/vectors/`):

  * `deny_region.json`, `body_too_large.json`, `decompress_guard.json`, `large_body_default_deny.json`, `eu_only.json`, `method_matrix.json`, `tags_all.json`.
* **Examples**:

  * `minimal_allow_deny.rs` — hello-world eval.
  * `ron_policy_cli.rs` — CLI to load a bundle and evaluate a request (DX smoke).

### Tooling & Hygiene

* **Clippy/Fmt**: clean under `-D warnings`, `pedantic`/`nursery` gates respected.
* **MSRV**: `rust-version = "1.78"` (set in Cargo.toml).
* **Lazy statics**: use `std::sync::LazyLock` (no once_cell).
* **Bench harness**: Criterion set up; baselines supported.

---

## 2) How to verify locally (commands)

### Full tests

```
cargo test -p ron-policy
```

### Run specific tests

```
cargo test -p ron-policy --test unit_method_matrix
cargo test -p ron-policy unit_first_match_wins::first_match_wins_and_default_applies -- --nocapture
```

### Run examples

```
cargo run -p ron-policy --example minimal_allow_deny
```

### CLI smoke on shipped vectors

```
cargo run -p ron-policy --example ron_policy_cli -- \
  --bundle crates/ron-policy/tests/vectors/tags_all.json \
  --tenant acme --method GET --region US --tag paid --tag verified
```

Other quick checks:

```
cargo run -p ron-policy --example ron_policy_cli -- \
  --bundle crates/ron-policy/tests/vectors/large_body_default_deny.json \
  --tenant t --method PUT --region US --body 524288

cargo run -p ron-policy --example ron_policy_cli -- \
  --bundle crates/ron-policy/tests/vectors/eu_only.json \
  --tenant t --method GET --region EU
```

### Benchmarks (Criterion)

```
cargo bench -p ron-policy
# Optional baselines
cargo bench -p ron-policy -- --save-baseline policy-2025-10-30
cargo bench -p ron-policy -- --baseline policy-2025-10-30
```

### Lints/hygiene

```
cargo fmt -p ron-policy
cargo clippy -p ron-policy --no-deps -- -D warnings
```

---

## 3) Integration guidance (svc-index / svc-gateway)

**Where to call it:** middleware layer before primary handler.

**Flow (high level):**

1. **Normalize request → Context**

   * `tenant` (header/path/claims)
   * `method` (uppercase)
   * `region` (geo/residency tag or inferred)
   * `body_bytes` (Content-Length or read count)
   * `tags` (authn/authz labels: `paid`, `verified`, etc.)
2. **Evaluate**: `Evaluator::evaluate(&ctx)`
3. **Enforce**:

   * If `Deny`: early return (e.g., 403/451) + include sanitized `reason`.
   * If `Allow`: continue; optionally apply **Obligations**:

     * `add-header` (k/v)
     * `mask-field` (placeholder, no-op OK in beta)
     * `log-audit` (emit structured event)
4. **Metrics**: expose the engine’s metrics on your service `/metrics`.

**Hot reload**: services own bundle loading/refresh (e.g., from svc-registry/config), then rebuild `Evaluator` and swap behind an `Arc`.

---

## 4) CI expectations

**Workflow (`.github/workflows/policy.yml`):**

* `cargo fmt -- --check`
* `cargo clippy -p ron-policy -- -D warnings`
* `cargo test -p ron-policy`
* (optional) CLI smoke using a vector
* (optional) JSON-Schema validation for `tests/vectors/*.json` against `schema/policybundle.schema.json` (Node `ajv` or `jsonschema` step)

**Tagging:**

```
git commit -m "ron-policy: 0.1.0-beta.1"
git tag ron-policy-v0.1.0-beta.1
git push && git push --tags
```

---

## 5) Current benchmark snapshot

Machine: user’s dev laptop (2019 MBP class), debugged with all tests green.
Criterion (release, bench profile):

```
eval:get/us   [1.1688 µs 1.1728 µs 1.1769 µs]  (~0.85–0.9 M eval/s/core)
```

Notes:

* Trace collection enabled and Prometheus histograms observed → real-world friendly number.
* Further gains possible by gating trace/metrics for “pure hot path” mode.

---

## 6) Troubleshooting (common gotchas)

* **Unknown field in JSON/TOML** → parse/validate error (by design). Fix vector or relax model if intentional (post-beta).
* **Method case**: builder uppercases method; if constructing contexts manually, ensure uppercase before eval.
* **Reason mismatch in tests**: remember **first-match wins** and any `method="*"` rule will preempt defaults.
* **CLI errors**: bundle must be `.json` or `.toml`; flags: `--tenant`, `--method`, `--region`, `--body`, repeated `--tag`.

---

## 7) Post-beta roadmap (prioritized)

**P1 — Integration & Controls**

* **Trace/metrics gating**: `PolicyOpts { trace: bool, metrics: bool }` or feature-gates for extremely hot paths.
* **Index widening**: secondary map `method+tenant`, optionally `region` partitioning, to shrink candidate scans in large bundles.
* **Obligations SDK surface** (library-side traits) to help services wire custom interpreters ergonomically.

**P2 — Tooling & DX**

* **Bundle hot-reload helper** (watcher + atomic swap, behind a feature; services can also own this).
* **Schema CI**: validate all vectors against `policybundle.schema.json` on every PR.
* **More vectors**: quotas/rate buckets, `decompress_guard`, PII masking, header mutation sequences.

**P3 — Perf & Hardening**

* **Pure hot path mode** (no trace strings, counters batched) → expect sub-µs evals.
* **String pooling/interning** for common reasons/ids to reduce alloc pressure in traces.
* **Fuzzing corpus** expansion (bundle parse & eval), plus mutation-based on vectors.

**P4 — Governance**

* **Spec doc** in `specs/governance.md`: clarify versioning, deprecation policy, and compatible change windows (minor/patch).
* **CHANGELOG discipline** per release; expand test matrix when adding fields.

---

## 8) What’s next across the stack

* **svc-gateway / svc-index**: wire the middleware call now (beta-level obligations: header/log/mask).
* **svc-registry** (later): central policy distribution; publish bundle + signature (future).
* **ron-audit** (later): consume `log-audit` obligations for compliance trails.

---

## 9) File map (key artifacts)

* `src/model.rs` — DTOs + serde rules (strict).
* `src/parse/{json,toml,validate}.rs` — loaders + structural validation.
* `src/engine/{eval,index,obligations,metrics}.rs` — evaluator, index, obligations placeholders, Prometheus.
* `src/explain/trace.rs` — deterministic explain.
* `tests/*.rs`, `tests/vectors/*.json` — full coverage of beta scope.
* `benches/eval_throughput.rs` — Criterion bench.
* `examples/{minimal_allow_deny.rs, ron_policy_cli.rs}` — quick start + smoke.

---

**Verdict:** `ron-policy` is **BETA DONE** within RON-CORE scope. The library is stable, fast, test-covered, and ready to be embedded in `svc-index` and `svc-gateway`. Post-beta items are perf tunings, richer indexing, and obligation SDK ergonomics, which can be done incrementally without breaking callers.


### END NOTE - OCTOBER 30 2025 - 16:34 CST




### BEGIN NOTE - APRIL 28 2026 - 23:10 CST

# ron-policy NOTES.MD — ROC Economics / Policy Carryover Notes

Date: 2026-04-29
Crate: `ron-policy`
Workspace path: `crates/ron-policy`
Project: RustyOnions / WEB3 / ROC internal value plane
Session status: **ROC economics TOML foundation complete and first live consumer wired through svc-storage**
Estimated current completion: **88–93% for WEB3 beta policy/economics foundation**, **70–78% for full WEB3_2 product-layer policy scope**

---

## 0. Executive Summary

`ron-policy` became the central policy/config substrate for ROC economics in this session. Before this session, the crate was already a useful allow/deny policy engine, but WEB3_2 economics work had not yet been implemented. The TODO and WEB3_2 documents explicitly called for `configs/roc-economics.toml`, a dedicated `ron-policy` economics module, validation of integer-only money values, basis-point splits, paid action definitions, disabled/unknown action behavior, and deterministic parser/validator tests. 

This session completed that foundation.

We added:

```text id="ovt7nt"
configs/roc-economics.toml

crates/ron-policy/src/economics/mod.rs
crates/ron-policy/src/economics/types.rs
crates/ron-policy/src/economics/load.rs
crates/ron-policy/src/economics/validate.rs

crates/ron-policy/tests/economics_policy.rs
```

We also updated:

```text id="yteh1l"
crates/ron-policy/src/lib.rs
crates/ron-policy/src/engine/metrics.rs
crates/ron-policy/src/engine/eval.rs
crates/ron-policy/tests/unit_eval_determinism.rs
crates/ron-policy/benches/eval_throughput.rs
```

The result is that `ron-policy` can now parse and validate a canonical root economics file, reject malformed/unsafe pricing policy, expose a small lookup API for paid actions, estimate action prices, validate recipient capture plans, and fail closed when explicit economics configuration is invalid.

The final observed `ron-policy` gate was fully green:

```text id="dhjh6e"
cargo fmt                                      ✅
cargo clippy -p ron-policy --all-targets      ✅
cargo test -p ron-policy --test economics_policy ✅
cargo test -p ron-policy --all-targets        ✅
economics_policy tests: 15/15 passed          ✅
full ron-policy test suite                    ✅
eval_throughput bench target                  ✅
```

The terminal output confirmed all 15 economics tests passed and the full `ron-policy` suite passed, including golden reasons, unit policy tests, bench target, and examples. 

Most importantly, this foundation was not left isolated. The next `svc-storage` batch immediately consumed it: `svc-storage` now optionally loads `configs/roc-economics.toml` through `ron-policy` and prices `paid_storage_put` from that policy when `RON_STORAGE_ROC_ECONOMICS_PATH` is set. The `svc-storage` economics tests passed 6/6, proving `ron-policy` is now the first live pricing authority for paid storage. 

---

## 1. Strategic Role in WEB3 / ROC

`ron-policy` is the deterministic policy and economics validation substrate.

Its correct role:

```text id="mgmbj1"
parse strict policy/economics config
validate policy invariants
reject unsafe/unknown/malformed economics
provide deterministic lookup helpers
price supported paid actions
validate capture plans and required recipients
provide allow/deny policy evaluation
explain decisions
support future capability/policy enforcement
```

Its incorrect role — do not regress into this:

```text id="ueo9rz"
not a wallet
not a ledger
not an accounting service
not a reward computation service
not a storage service
not a gateway
not a dashboard
not a mutable config daemon by itself
not an external-chain policy engine
not a ROX/Solana bridge
```

Correct boundary:

```text id="i6cl02"
ron-policy:
  validates and interprets policy/economics

svc-wallet:
  applies wallet/capability policy at mutation boundary

svc-storage / svc-gateway / omnigate:
  call policy/economics to estimate paid actions and validate captures

ron-accounting:
  records usage; does not decide economics truth

svc-rewarder:
  consumes accounting/policy signals for deterministic payouts

ron-ledger:
  durable economic truth
```

The crate remains side-effect-free for economics parsing: caller supplies TOML bytes; the library parses, validates, and returns decisions. It does not read files by itself, call wallet, call ledger, or mutate any service state.

---

## 2. What Was Already Working Before This Session

Before this session, `ron-policy` had:

```text id="9k9u7u"
strict policy bundle DTOs
JSON/TOML policy loading for allow/deny rules
policy validation
deterministic evaluator
rule index
decision traces
obligations
golden reason tests
unit tests for method/region/tags/body defaults
Prometheus policy evaluation metrics
example CLI/minimal allow-deny examples
criterion bench target
```

However, the WEB3_2 economics layer was mostly missing. The TODO explicitly marked `roc-economics config` at roughly **35–45%** before implementation and called out `configs/roc-economics.toml` plus the economics module as the next foundation. 

This session moved it from “not yet done” to a tested and consumed foundation.

---

## 3. What We Accomplished in This Session

### 3.1 Added `configs/roc-economics.toml`

We added the root canonical economics config:

```text id="xn7gpt"
configs/roc-economics.toml
```

It defines:

```text id="j1goxr"
version
unit
default_asset
rounding
remainder_sink
static accounts
dynamic recipient roles
global limits
per-action pricing
per-action max spend
per-action hold multiplier
per-action payout splits
```

The canonical beta actions included:

```text id="ngjq2i"
paid_storage_put
paid_storage_pin
paid_content_view
paid_song_play
site_visit
```

This matches WEB3_2’s recommended first implementation location and action scope for the economics foundation. WEB3_2 explicitly says to put configurable economics in `ron-policy` with `configs/roc-economics.toml`, then later begin b3/crab work in `ron-naming`. 

### 3.2 Added strict economics DTOs

New DTO module:

```text id="bq498e"
crates/ron-policy/src/economics/types.rs
```

Primary types:

```text id="txtwyh"
EconomicsPolicy
EconomicsLimits
ActionEconomics
PricingKind
RoundingMode
PayoutSplit
```

Design choices:

```text id="f0q9m3"
serde deny_unknown_fields
integer minor units only
u64 in config DTOs because toml crate does not support u128
u128 only at checked arithmetic boundaries
basis points use u16/u32 as appropriate
rounding currently supports floor
pricing kinds are snake_case in TOML
rounding is lowercase in TOML
```

Important fix from this session:

```text id="m65ne4"
The toml crate failed to deserialize u128 values.
We changed config DTO money fields to u64 and cast to u128 only during checked math.
```

The terminal output showed the exact failure as `u128 is not supported`, and after the fix all economics tests passed. 

### 3.3 Added strict TOML loader

New loader module:

```text id="oe8o1n"
crates/ron-policy/src/economics/load.rs
```

Public helpers:

```text id="fbp7r4"
load_economics_toml(bytes)
load_economics_toml_str(str)
```

Behavior:

```text id="s14zzl"
parse UTF-8
parse TOML
deserialize strict DTOs
validate policy invariants
return deterministic Error::Parse or Error::Validation
```

Important invariant:

```text id="wejc5h"
No file I/O in ron-policy economics loader.
The caller chooses where bytes come from.
```

This is important because production services may load config from different sources: local file, bundled default, admin endpoint, config service, or in-memory test fixture.

### 3.4 Added economics validator

New validator module:

```text id="u5vub8"
crates/ron-policy/src/economics/validate.rs
```

Validated rules include:

```text id="ul8azv"
version >= 1
unit == "roc_minor"
default_asset == "roc"
rounding == floor
remainder_sink is declared
default_single_action_cap_minor > 0
daily cap >= single action cap
anonymous cap <= default daily cap
all beta actions are present
unknown actions reject
max_hold_multiplier_bps >= 10000
max_spend_minor > 0
minimum_charge_minor > 0
max_spend_minor >= minimum_charge_minor
max_spend_minor <= default_daily_spend_cap_minor
flat actions require price_minor > 0
per-byte actions require price_per_byte_minor > 0
per-second actions require price_per_second_minor > 0
each action has at least one split
split destinations are non-empty
split bps > 0
duplicate split destinations reject
split destinations must be declared account or role
split bps sum exactly to 10_000
```

This matches WEB3_2’s key requirement: no floats, all percentages in basis points, and splits must be deterministic and validated. WEB3_2 explicitly says to use basis points, not floats, and gives `10000 bps = 100%`. 

### 3.5 Added economics lookup/pricing API

New public API exposed from `ron-policy`:

```text id="q9zdfq"
EconomicsPolicy::action_ids()
EconomicsPolicy::enabled_action(action_id)
EconomicsPolicy::price_for(action_id, quantity)
EconomicsPolicy::validate_capture_plan(action_id, recipients, amount_minor)
validate_economics_policy(policy)
```

Current pricing modes:

```text id="3qyh6y"
flat
per_byte_plus_minimum
per_second_plus_minimum
```

Current price behavior:

```text id="79l358"
base price is computed from pricing kind
minimum_charge_minor is applied
max_hold_multiplier_bps is applied
hold amount rounds up by integer arithmetic
hold amount cannot exceed max_spend_minor
disabled actions fail closed
unknown actions fail closed
```

Example proven by tests:

```text id="wqz3s1"
paid_storage_put:
  minimum_charge_minor = 70
  price_per_byte_minor = 1
  max_hold_multiplier_bps = 12000

price_for("paid_storage_put", 1)   = 84
price_for("paid_storage_put", 48)  = 84
price_for("paid_storage_put", 100) = 120
```

### 3.6 Added comprehensive economics tests

New test file:

```text id="kvmr9z"
crates/ron-policy/tests/economics_policy.rs
```

Final test count:

```text id="c82i68"
15 tests
15 passed
0 failed
```

Test coverage:

```text id="d3lkrw"
valid checked-in config loads
deterministic action order is sorted
invalid split sum rejects
unknown split destination rejects
unknown action rejects
disabled action config can load but lookup rejects
float value rejects during parse
negative value rejects during parse
overflow value rejects during parse
missing required action rejects
paid_storage_put uses minimum and hold multiplier
unknown paid action lookup rejects
missing dynamic recipient rejects capture plan
capture plan accepts required dynamic recipient
capture over action cap rejects
```

Final output confirmed all economics tests passed. 

### 3.7 Fixed strict clippy/doc lints

`ron-policy` is strict:

```rust id="8x5hk7"
#![deny(clippy::all, clippy::pedantic, clippy::nursery)]
```

During this session, clippy rejected:

```text id="xq4jhl"
missing backticks in doc comments
too_long_first_doc_paragraph
default constructed unit structs
MSRV-incompatible LazyLock
```

We fixed:

```text id="nhbs4w"
RO header blank lines
backticks around code-like docs
10_000 docs with backticks
SystemClock::default() → SystemClock
LazyLock → OnceLock
```

### 3.8 Preserved existing evaluator behavior

One temporary replacement of `engine/eval.rs` accidentally used a non-existent field:

```text id="425baj"
defaults.allow
```

The actual model uses:

```text id="oxujqj"
defaults.default_action
defaults.max_body_bytes
```

We restored the evaluator shape to match existing crate behavior:

```text id="fbbznj"
borrowed PolicyBundle
RuleIndex
ObligationSet
DecisionTrace
default_action fallback
max_body_bytes default cap
first-match-wins rule matching
```

This was important because economics should not regress the existing policy engine.

### 3.9 Added MSRV-compatible metrics initialization

`std::sync::LazyLock` triggered clippy’s MSRV lint because the crate declares/supports Rust `1.78.0`, while `LazyLock` stabilized later. The fix replaced it with:

```text id="vijy1n"
std::sync::OnceLock
```

This preserved lazy Prometheus metric registration while satisfying the crate’s strict MSRV discipline.

### 3.10 First live consumer wired through svc-storage

After the `ron-policy` economics foundation went green, `svc-storage` was updated to use it:

```text id="1m4bbq"
ron-policy added as svc-storage dependency
svc-storage policy/economics.rs calls ron_policy::load_economics_toml
paid_storage_put can be priced from configs/roc-economics.toml
legacy pricing remains fallback when env var is unset
explicit bad policy path fails closed
explicit unknown action fails closed
```

The `svc-storage` economics tests passed 6/6 and the full `svc-storage` test suite passed. 

This matters because `ron-policy` is no longer just a parser/validator on paper; it is now the first real economics authority consumed by a paid-use crate.

---

## 4. Current Public Economics API

Recommended imports:

```rust id="0ndrl4"
use ron_policy::economics::{
    load_economics_toml_str,
    load_economics_toml,
    validate_economics_policy,
    EconomicsPolicy,
};
```

Root re-exports from `ron-policy` include:

```text id="2x2xbb"
load_economics_toml
validate_economics_policy
ActionEconomics
EconomicsLimits
EconomicsPolicy
PayoutSplit
PricingKind
RoundingMode
```

### 4.1 Loading from bytes

```rust id="l4bdn1"
let policy = ron_policy::load_economics_toml(bytes)?;
```

### 4.2 Loading from string

```rust id="r8v2xa"
let policy = ron_policy::economics::load_economics_toml_str(raw)?;
```

### 4.3 Validating an already-mutated policy

```rust id="gkr7ca"
ron_policy::economics::validate_economics_policy(&policy)?;
```

### 4.4 Deterministic action ordering

```rust id="8ebqla"
let ids = policy.action_ids();
```

Current expected sorted order:

```text id="uemfdm"
paid_content_view
paid_song_play
paid_storage_pin
paid_storage_put
site_visit
```

### 4.5 Pricing

```rust id="tb5zm0"
let hold_amount = policy.price_for("paid_storage_put", bytes_stored)?;
```

Current semantics:

```text id="xu77uw"
quantity means bytes for paid_storage_put
quantity means seconds for paid_storage_pin
quantity is ignored for flat actions
minimum charge applies
hold multiplier applies
integer round-up applies
action max cap applies
```

### 4.6 Capture plan validation

```rust id="muw5gl"
policy.validate_capture_plan("paid_storage_put", &recipients, amount_minor)?;
```

For dynamic split recipients, caller must provide recipients such as:

```text id="v3jquj"
storage_provider -> t:default/provider/storage/w
content_owner    -> t:default/u:artist/w
site_owner       -> t:default/u:site/w
```

---

## 5. Current Config Shape

Current root file:

```text id="8svqfx"
configs/roc-economics.toml
```

Top-level shape:

```toml id="j5mssx"
version = 1
unit = "roc_minor"
default_asset = "roc"
rounding = "floor"
remainder_sink = "treasury"
```

Accounts:

```toml id="q3p015"
[accounts]
treasury = "t:default/treasury"
reward_pool = "t:default/reward_pool"
burn = "t:default/burn"
```

Dynamic roles:

```toml id="lnh5j2"
[roles]
site_owner = "dynamic:site_owner"
content_owner = "dynamic:content_owner"
storage_provider = "dynamic:storage_provider"
edge_provider = "dynamic:edge_provider"
app_owner = "dynamic:app_owner"
curator_pool = "dynamic:curator_pool"
```

Limits:

```toml id="n4xsqn"
[limits]
default_daily_spend_cap_minor = 100000
default_single_action_cap_minor = 10000
anonymous_daily_spend_cap_minor = 1000
```

Required actions:

```text id="mvxluy"
paid_storage_put
paid_storage_pin
paid_content_view
paid_song_play
site_visit
```

Important design rule:

```text id="6b10yf"
Money values are integers.
Percentages are basis points.
No floats.
No negative values.
No unknown fields.
```

---

## 6. Current Test and Gate Status

Final passing command:

```bash id="9t71c4"
cargo fmt
cargo clippy -p ron-policy --all-targets -- -D warnings
cargo test -p ron-policy --test economics_policy
cargo test -p ron-policy --all-targets
```

Observed passing results:

```text id="bks5tb"
ron-policy clippy: passed
economics_policy: 15 passed
golden_reasons: passed
unit_churn_protection: passed
unit_eu_only: passed
unit_eval_determinism: passed
unit_first_match_wins: passed
unit_large_body_default_deny: passed
unit_method_matrix: passed
unit_model_serde_strict: passed
unit_tags_all: passed
unit_tighten_only: passed
eval_throughput bench target: success
examples built as test targets
```

The terminal output explicitly showed the full test suite green. 

Standing `ron-policy` gate:

```bash id="2k7op9"
cargo fmt
cargo clippy -p ron-policy --all-targets -- -D warnings
cargo test -p ron-policy --all-targets
```

Economics-only focused gate:

```bash id="az0a7m"
cargo test -p ron-policy --test economics_policy
```

Downstream consumer gate:

```bash id="sum6k7"
cargo clippy -p svc-storage --all-targets -- -D warnings
cargo test -p svc-storage --test paid_write_economics
```

---

## 7. Current Completion Estimate

After this session:

```text id="rgludb"
core allow/deny policy engine:              85–90%
strict policy DTO/serde foundation:         88–92%
decision tracing / obligations:             82–88%
Prometheus eval metrics:                    78–85%
ROC economics TOML parser:                  94–97%
ROC economics validation:                   94–97%
paid action pricing API:                    88–93%
capture plan validation:                    85–90%
svc-storage consumer integration:           82–88%
WEB3 beta economics policy readiness:       88–93%
full WEB3_2 product policy readiness:       70–78%
```

Interpretation:

```text id="ip4nk0"
For WEB3 beta:
  ron-policy now has the economics foundation needed to stop hardcoding prices.

For full WEB3_2:
  it still needs richer split planning, paid-content/gateway consumers,
  passport/capability policy integration, owner/payout validation, dashboard visibility,
  and more asset-kind-specific policy behavior.
```

---

## 8. Important Invariants to Preserve

Do not regress:

```text id="yv0w3d"
no floating-point money math
money values are integer minor units
percentages are basis points
split bps totals must be exactly 10_000 where required
unknown actions reject
disabled actions fail closed at lookup
unknown split destinations reject
duplicate split destinations reject
negative values reject at parse
floats reject at parse
overflow values reject at parse
config DTOs deny unknown fields
rounding is deterministic
price calculation uses checked arithmetic
hold multiplier uses integer math
capture plan validates dynamic recipients
ron-policy does not mutate wallet
ron-policy does not mutate ledger
ron-policy does not read files in library loader
ron-policy does not call storage/gateway
ron-policy does not compute rewards itself
ron-policy does not include ROX/Solana/external settlement logic
```

Important architectural boundary:

```text id="6h4xjn"
ron-policy validates economics and policy.
Services enforce those decisions.
svc-wallet mutates balances.
ron-ledger records truth.
```

---

## 9. Known Issues Solved During This Session

### 9.1 `u128` TOML deserialization failure

Problem:

```text id="blmt73"
toml could not parse u128 config fields
```

Fix:

```text id="vpvydp"
config DTO money fields use u64
runtime checked arithmetic can cast to u128 where needed
```

### 9.2 Clippy docs were stricter than expected

Problem:

```text id="zoscb1"
clippy::doc_markdown
clippy::too_long_first_doc_paragraph
```

Fix:

```text id="ro2kjz"
blank line after RO:WHAT
backticks around code-like names
backticks around 10_000
shorter first doc paragraphs
```

### 9.3 MSRV conflict with `LazyLock`

Problem:

```text id="l71jye"
std::sync::LazyLock requires newer Rust than the crate MSRV
```

Fix:

```text id="j0perv"
use std::sync::OnceLock
```

### 9.4 Accidental evaluator regression

Problem:

```text id="ulqedk"
temporary eval.rs replacement used defaults.allow,
but model::Defaults has default_action and max_body_bytes
```

Fix:

```text id="w4ivnm"
restore borrowed PolicyBundle + RuleIndex evaluator
use default_action fallback
preserve obligations and traces
```

### 9.5 Missing required action test initially generated unknown action instead

Problem:

```text id="p6y757"
test changed [actions.site_visit] to [actions.site_visit_missing]
validator saw unknown action, not missing required action
```

Fix:

```text id="ywb9x9"
load valid policy
remove site_visit from actions map
call validate_economics_policy(&policy)
assert missing required action
```

---

## 10. What Remains for ron-policy

### Priority A — Economics split planning / allocation output

Currently `ron-policy` validates splits, but it does not yet produce a full allocation plan.

Needed API:

```text id="01wz4r"
policy.split_plan(action_id, recipients, amount_minor)
```

Desired output:

```text id="oo1zsw"
Vec<SplitAllocation>:
  role/destination
  resolved account
  bps
  amount_minor
  residual_sink amount
```

Required rules:

```text id="6bg5vn"
amounts use integer minor units
basis-point math only
floor/rounding policy deterministic
residual goes to configured remainder_sink
total allocated + residual == amount_minor
all dynamic recipients resolved
unknown/missing recipient rejects
```

This is required before paid content/site flows can pay multiple recipients according to configured splits.

### Priority B — Paid content and site visit consumer integration

Current live consumer:

```text id="q0riig"
svc-storage uses paid_storage_put pricing optionally
```

Needed consumers:

```text id="wuztaj"
svc-gateway paid_content_view estimate path
omnigate paid_content_view estimate path
site_visit estimate path
paid_song_play estimate path
paid_storage_pin estimate path
```

WEB3_2 specifically calls for paid content view with economics action, wallet hold, content serve path, capture/release plan, and accounting events. 

### Priority C — Recipient role binding with manifests/index

`ron-policy` knows role names, but not real asset owners.

Needed upstream integration:

```text id="4c0hku"
ron-naming asset manifests provide content_owner/site_owner/app_owner
svc-index exposes owner/payout metadata
gateway/omnigate resolve dynamic recipients
ron-policy validates recipient presence and computes split
wallet captures to resolved accounts
```

Do not put ownership lookup inside `ron-policy`. It should validate and compute based on caller-supplied recipient map.

### Priority D — Capability / passport policy integration

WEB3_2 requires wallet-linked passport permissions and caveats. `ron-policy` should eventually define reusable policy helpers for:

```text id="5xnhm7"
passport kind
main vs alt identity
wallet account caveats
asset caveats
action caveats
per-action spend caps
per-site spend caps
expiry/TTL
audience/service caveats
```

But `ron-policy` should still not manage private keys or passport storage.

### Priority E — Better deterministic error taxonomy

Current errors are string-based `Error::Parse` and `Error::Validation`.

Future improvement:

```text id="v8fjgy"
EconomicsErrorKind:
  ParseToml
  UnsupportedVersion
  InvalidUnit
  InvalidAsset
  UnknownAction
  MissingRequiredAction
  DisabledAction
  UnknownSplitDestination
  DuplicateSplitDestination
  InvalidSplitTotal
  InvalidMoneyAmount
  InvalidBps
  Overflow
  MissingRecipient
  CapExceeded
```

This would improve dashboard/admin output, service metrics, and tests.

### Priority F — Canonical normalized economics output

Tests currently prove deterministic action ordering. Future work should add:

```text id="k4f87z"
canonical JSON representation
canonical hash/CID of economics policy
policy_hash = b3:<64 lowercase hex>
```

This could let services prove exactly which economics config was applied to a payment, receipt, or reward epoch.

### Priority G — Config versioning and migration

Current config version:

```text id="bx1ltp"
version = 1
```

Future needs:

```text id="pqbbl6"
schema version enum
migration notes
backward-compatible parse for v1
clear rejection for unsupported future versions
effective policy version in receipts/manifests
```

### Priority H — Asset-kind-specific policies

Current actions are generic. Future policy may include:

```text id="8cxp8f"
paid_content_view.image
paid_content_view.article
paid_content_view.video
paid_song_play
paid_video_view
paid_comment_boost
site_visit
app_use
```

Need avoid overfitting too early. The current action ID model is enough for beta.

### Priority I — Admin/diff tooling

Useful future CLI/tool:

```text id="cdeabi"
ron-policy validate-economics configs/roc-economics.toml
ron-policy explain-price paid_storage_put --quantity 48
ron-policy split paid_content_view --amount 100 --recipient content_owner=...
ron-policy diff old.toml new.toml
```

This would help operators tune economics safely.

### Priority J — Property tests

Add property tests for:

```text id="i94tgg"
split totals always preserve amount
random valid split lists validate
random invalid split totals reject
random quantity does not overflow
price_for never returns 0 for enabled action
price_for never exceeds max_spend_minor
disabled actions always reject lookup
unknown actions always reject lookup
```

### Priority K — Metrics conventions

`ron-policy` is a library, so consumers should emit metrics. But we should define conventions:

```text id="7v4qzp"
roc_economics_policy_load_total{result}
roc_economics_action_price_total{action,result}
roc_economics_capture_plan_total{action,result}
roc_economics_config_version
roc_economics_validation_error_total{reason}
```

Do not emit high-cardinality action recipient account IDs.

### Priority L — Gateway/omnigate contract tests

Once gateway/omnigate consume `ron-policy`, add cross-crate tests:

```text id="h9k5tn"
paid_content_view rejects unknown action
paid_content_view uses configured flat price
paid_content_view rejects missing content_owner
paid_content_view split plan matches bps
site_visit pays site_owner
song_play pays content_owner/curator/treasury/reward_pool
```

---

## 11. Known Commands

### Full ron-policy gate

```bash id="08rjtm"
cargo fmt
cargo clippy -p ron-policy --all-targets -- -D warnings
cargo test -p ron-policy --all-targets
```

### Economics-only gate

```bash id="7p0t9o"
cargo test -p ron-policy --test economics_policy
```

### Downstream storage consumer gate

```bash id="94a1j5"
cargo fmt
cargo clippy -p svc-storage --all-targets -- -D warnings
cargo test -p svc-storage --test paid_write_economics
cargo test -p svc-storage --all-targets
```

### Paid-storage live smoke with legacy fallback pricing

```bash id="d4vskb"
bash scripts/web3_paid_storage_live_smoke.sh
```

### Future economics-enabled paid-storage smoke

When adding a dedicated economics-enabled script, use:

```bash id="22hjtu"
RON_STORAGE_ROC_ECONOMICS_PATH=configs/roc-economics.toml \
bash scripts/web3_paid_storage_live_smoke.sh
```

Important note:

```text id="r36563"
The hold amount must be high enough for configured pricing.
For the current 48-byte smoke object, paid_storage_put prices to 84 minor units,
not the legacy 48 minor units.
```

---

## 12. Suggested NOTES.MD Commit Summary

```text id="p50ob7"
ron-policy: document ROC economics TOML foundation

- add configs/roc-economics.toml as beta-safe internal ROC economics config
- add economics module with strict DTOs, TOML loader, validator, and lookup helpers
- enforce integer minor units, basis points, required beta actions, split totals,
  disabled/unknown action rejection, overflow/negative/float rejection, and dynamic recipient validation
- fix strict clippy docs/MSRV issues without regressing existing evaluator behavior
- record final green gates: clippy, economics_policy 15/15, full ron-policy test suite
- note first live consumer: svc-storage can price paid_storage_put from validated economics config
```

Longer body:

```text id="gcveij"
ron-policy now owns the first version of the RustyOnions ROC economics policy substrate.
The new economics module parses and validates configs/roc-economics.toml, rejects unsafe
or ambiguous paid-action policy, and exposes deterministic pricing/capture-plan helpers.
This unlocks paid storage, paid content, site visits, songs, and future asset payout splits
without hardcoding economics inside storage/gateway services.

The crate remains side-effect-free: it validates and interprets policy only. It does not mutate
wallet, ledger, accounting, storage, or reward state.
```

---

## 13. Final Status

`ron-policy` should now be considered:

```text id="cy4fid"
Status: ROC economics foundation complete for WEB3 beta
WEB3 beta economics readiness: 88–93%
Full WEB3_2 product policy readiness: 70–78%
Main remaining work: split allocation output, paid-content/gateway consumers,
manifest/owner recipient binding, passport/capability caveats, richer error taxonomy,
canonical policy hash, and admin/diff tooling.
```

Most important achievement:

```text id="z4acv8"
RustyOnions now has a central, validated, integer-only ROC economics policy file
instead of scattering paid-action prices and splits across services.
```

This is a major WEB3_2 foundation step because it unlocks configurable paid storage, paid content, site visits, songs, creator payouts, provider payouts, treasury/reward pool routing, and future dashboard visibility while preserving WEB3 invariants.


### END NOTE - APRIL 28 2026 - 23:10 CST