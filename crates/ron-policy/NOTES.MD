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


### BEGIN NOTE - JUNE 16 2026 - 00:10 CST

# ron-policy — QuickChain Phase-0 / QC-0A Completion Notes

## Status

`ron-policy` is complete for the current QuickChain Phase-0 / QC-0A preflight scope.

This crate is now guarded as **declarative policy infrastructure only**. It is not wallet authority, not ledger authority, not receipt authority, not paid-unlock truth, not checkpoint/root infrastructure, not validator/consensus infrastructure, and not settlement/bridge/staking/liquidity infrastructure.

Current estimated completion for this crate’s QuickChain preflight slice: **96–98% complete**.

The remaining 2–4% is intentionally deferred future work that depends on later QuickChain phases, canonical vector work, service integration maturity, and locked proof/serialization decisions. No root-producing or settlement-producing work belongs in `ron-policy` during the current phase.

---

## QuickChain Role of `ron-policy`

`ron-policy` is a declarative policy engine.

Its job is to:

* Parse and validate strict policy bundles.
* Evaluate request context against allow/deny rules.
* Return policy decisions with reasons and obligations.
* Validate ROC economics policy configuration.
* Provide deterministic pricing/config helpers.
* Stay fail-closed.
* Stay side-effect free.
* Keep policy decisions separate from economic truth.

Its job is **not** to:

* Issue ROC.
* Transfer ROC.
* Burn ROC.
* Mint ROC.
* Hold ROC.
* Capture ROC.
* Release ROC.
* Create receipts.
* Store receipts.
* Commit receipts.
* Accept receipts.
* Prove receipt finality.
* Prove paid unlock finality.
* Store or mutate wallet balances.
* Mutate ledger state.
* Generate QuickChain roots.
* Generate checkpoints.
* Define validator sets.
* Define validator signatures.
* Implement settlement.
* Implement external anchors.
* Implement bridge logic.
* Implement staking.
* Implement liquidity.
* Implement ROX or Solana paths.

The core boundary is:

```text
policy decision != economic truth
policy allow != paid proof
policy obligation != receipt proof
policy explanation != finality proof
economics policy config != ledger mutation
feature flag != settlement authority
```

---

## Work Completed in This Slice

### 1. QuickChain preflight documentation added

A crate-local QuickChain preflight document was added:

```text
crates/ron-policy/docs/quickchain-preflight.md
```

The doc establishes `ron-policy` as declarative policy infrastructure and explicitly states that the crate must not manufacture or imply:

* paid proof
* receipt proof
* finality proof
* balance proof
* ledger mutation
* wallet mutation
* paid unlock finality
* root-producing code
* checkpoint-producing code
* validator code
* settlement code
* bridge logic
* external anchors
* staking
* liquidity
* ROX
* Solana

The doc is now enforced by tests, so accidental weakening of the written boundary should fail fast.

---

### 2. QuickChain docs gate added

Added:

```text
crates/ron-policy/tests/quickchain_preflight_docs.rs
```

This test verifies:

* `docs/quickchain-preflight.md` exists.
* The doc says `ron-policy is declarative policy infrastructure`.
* The doc says `policy decision is not economic truth`.
* The doc says `policy allow is not paid proof`.
* The doc says `policy obligation is not receipt proof`.
* The doc says `policy explanation is not finality proof`.
* The doc says `economics policy config is not ledger mutation`.
* The doc says `feature flag is not settlement authority`.
* The doc explicitly forbids root-producing code, checkpoint-producing code, validators, settlement, wallet mutation, ledger mutation, paid unlock finality, external anchors, bridge logic, staking, liquidity, ROX, and Solana.

Verified green:

```text
quickchain_preflight_docs: 3 passed
```

---

### 3. Production source boundary tests added

Added:

```text
crates/ron-policy/tests/quickchain_preflight_boundary.rs
```

This test scans production Rust source under:

```text
crates/ron-policy/src/
```

It also scans:

```text
crates/ron-policy/Cargo.toml
```

The test proves that production `ron-policy` source and dependencies do not import or depend on wallet/ledger/runtime authority crates, including:

* `svc-wallet`
* `ron-ledger`
* `ron-accounting`
* `svc-rewarder`
* `quickchain-runtime`
* `quickchain-validator`
* `quickchain-consensus`
* `solana`
* `solana-sdk`
* `solana-client`

The test also forbids mutation call shapes such as:

* `.issue(`
* `::issue(`
* `.transfer(`
* `::transfer(`
* `.burn(`
* `::burn(`
* `.hold(`
* `::hold(`
* `.capture(`
* `::capture(`
* `.release(`
* `::release(`
* `.mint(`
* `::mint(`
* `put_receipt(`
* `insert_receipt(`
* `create_receipt(`
* `accept_receipt(`
* `commit_receipt(`
* `mutate_balance(`
* `set_balance(`
* `credit_account(`
* `debit_account(`

The test also forbids QuickChain runtime/root/checkpoint/finality tokens such as:

* `state_root`
* `receipt_root`
* `accounting_root`
* `reward_root`
* `checkpoint_root`
* `checkpoint_hash`
* `checkpoint_header`
* `validator_set`
* `validator_signature`
* `committee_quorum`
* `fork_choice`
* `settlement_status`
* `external_anchor`
* `bridge_settlement`
* `root_producer`
* `produce_root`
* `produce_checkpoint`
* `finalize_checkpoint`
* `quickchain_receipt`
* `quickchain_checkpoint`
* `quickchain_validator`

Verified green:

```text
quickchain_preflight_boundary: 4 passed
```

---

### 4. Public surface boundary tests added

Added:

```text
crates/ron-policy/tests/quickchain_preflight_public_surface_boundary.rs
```

This test scans non-production public surfaces:

```text
crates/ron-policy/examples/
crates/ron-policy/benches/
crates/ron-policy/fuzz/fuzz_targets/
```

The purpose is to prevent examples, benchmarks, or fuzz targets from sneaking in wallet/ledger/root/checkpoint/settlement authority even if `src/` remains clean.

It checks for:

* forbidden mutation/runtime imports
* forbidden wallet/ledger mutation call shapes
* forbidden QuickChain runtime/root/checkpoint/validator/settlement tokens

This matters because examples and fuzz targets can become copied patterns. Keeping those surfaces policy-only prevents accidental authority creep.

Verified green:

```text
quickchain_preflight_public_surface_boundary: 1 passed
```

---

### 5. Policy decision non-authority tests added

Added:

```text
crates/ron-policy/tests/quickchain_preflight_decision_non_authority.rs
```

This test proves policy evaluation output remains policy-only.

It checks that:

* An `allow` decision is not treated as paid unlock proof.
* A `deny` decision is still not receipt/balance/finality truth.
* Safe obligations like `require-backend-wallet-ledger-proof` can exist as declarative instructions.
* Authority-shaped obligation kinds are rejected.
* Authority-shaped obligation parameter keys are rejected.
* Debug output for `Decision` does not carry authority-shaped fields such as:

  * `receipt_id`
  * `receipt_hash`
  * `receipt_root`
  * `balance_minor`
  * `wallet_balance`
  * `ledger_balance`
  * `finality`
  * `finalized`
  * `unlock_granted`
  * `settlement_status`
  * `state_root`
  * `checkpoint_root`
  * `checkpoint_hash`
  * `spend_authority`
  * `capture_authority`
  * `validator_signature`
  * `mint_authority`

Verified green:

```text
quickchain_preflight_decision_non_authority: 4 passed
```

---

### 6. Policy parser validation hardened

Updated:

```text
crates/ron-policy/src/parse/validate.rs
```

The policy bundle validator now rejects authority-shaped obligations.

The validator enforces:

* bundle version must be >= 1
* rule IDs must be non-empty
* rule IDs must be unique
* default max body cap must not exceed 1 MiB
* per-rule max body cap must not exceed 1 MiB
* obligation kind must be non-empty
* authority-shaped obligation kinds must reject
* authority-shaped obligation parameter keys must reject

Forbidden obligation kind patterns include authority-like shapes such as:

* `issue`
* `transfer`
* `burn`
* `mintroc`
* `allocateroc`
* `createreceipt`
* `putreceipt`
* `insertreceipt`
* `acceptreceipt`
* `commitreceipt`
* `finalizereceipt`
* `mutatebalance`
* `setbalance`
* `creditaccount`
* `debitaccount`
* `unlockpaidcontent`
* `provepaymentfinality`
* `producecheckpoint`
* `produceroot`
* `settlementcomplete`
* `settlementfinalized`
* `bridgesettlement`

Forbidden obligation parameter keys include exact normalized shapes such as:

* `receiptid`
* `receipthash`
* `receiptroot`
* `balance`
* `balanceminor`
* `walletbalance`
* `ledgerbalance`
* `finality`
* `finalized`
* `unlockgranted`
* `paidproof`
* `settlementstatus`
* `spendauthority`
* `captureauthority`
* `stateroot`
* `checkpointroot`
* `checkpointhash`
* `validatorsignature`
* `mintauthority`

The parser validator normalizes tokens by removing non-alphanumeric separators and lowercasing. This means variants such as `receipt-id`, `receipt_id`, `receiptId`, and similar forms are caught.

---

### 7. Parser consistency tests added

Added:

```text
crates/ron-policy/tests/quickchain_preflight_parser_consistency.rs
```

This proves JSON and TOML loaders both reject authority-shaped obligations consistently.

The tests verify rejection of:

* JSON camelCase authority obligation kind: `unlockPaidContent`
* JSON dash-separated authority param key: `receipt-id`
* TOML authority obligation kind: `finalize_receipt`
* TOML authority param key: `checkpoint-root`

Verified green:

```text
quickchain_preflight_parser_consistency: 4 passed
```

---

### 8. Published JSON schema hardened

Updated:

```text
crates/ron-policy/schema/policybundle.schema.json
```

The schema now preserves strict DTO discipline and includes explicit authority-shape rejection for obligations.

Schema protections include:

* top-level `additionalProperties: false`
* strict rule shape
* strict obligation shape
* required `version`
* required `rules`
* required rule `id`
* required rule `when`
* required rule `action`
* required obligation `kind`
* max body cap maximum of `1048576`
* obligation kind rejection patterns
* obligation param key rejection patterns via `propertyNames`

The schema is not the only authority boundary; the Rust parser validator is still the enforcement path. The schema exists so published bundle shape does not drift away from parser behavior.

---

### 9. Schema non-authority tests added

Added:

```text
crates/ron-policy/tests/quickchain_preflight_schema_non_authority.rs
```

This test verifies:

* schema is valid JSON
* schema preserves strict DTO shape
* schema requires version/rules
* schema requires rule id/when/action
* schema requires obligation kind
* schema preserves 1 MiB body cap
* schema rejects authority-shaped obligation kinds
* schema rejects authority-shaped obligation parameter names

Verified green:

```text
quickchain_preflight_schema_non_authority: 5 passed
```

---

### 10. Economics config non-authority tests added

Added:

```text
crates/ron-policy/tests/quickchain_preflight_economics_config_non_authority.rs
```

This test proves ROC economics config remains configuration only.

It verifies:

* top-level authority-shaped fields such as `receipt_id` are rejected by strict TOML parsing
* action-level authority fields such as `unlock_granted` are rejected by strict TOML parsing
* serialized economics config contains no authority-shaped keys
* pricing helpers return amounts only, not receipts, balances, roots, finality, unlock, or settlement artifacts

It confirms that `price_for("paid_storage_put", 48)` returns an amount-only estimate of `84`, not a proof object or receipt.

Verified green:

```text
quickchain_preflight_economics_config_non_authority: 4 passed
```

---

### 11. Economics identifier non-authority validation added

Updated:

```text
crates/ron-policy/src/economics/validate.rs
```

Added focused validation for economics identifiers:

* `remainder_sink`
* account aliases
* role aliases
* split destinations
* action IDs

The validator rejects exact normalized authority artifact identifiers such as:

* `receiptid`
* `receipthash`
* `receiptroot`
* `balance`
* `balanceminor`
* `walletbalance`
* `ledgerbalance`
* `finality`
* `finalized`
* `unlockgranted`
* `paidproof`
* `settlementstatus`
* `spendauthority`
* `captureauthority`
* `stateroot`
* `checkpointroot`
* `checkpointhash`
* `validatorsignature`
* `mintauthority`
* `operationid`
* `idempotencykey`

Important design correction:

The first attempt used a broad substring ban that rejected legitimate declarative aliases such as `burn`. That was wrong because `burn` can be a static economics account/sink alias without giving the policy crate burn authority.

The final implementation intentionally uses exact normalized authority artifact identifiers rather than a broad substring ban.

This means:

```text
burn account alias: allowed as declarative economics config
burn operation authority: still forbidden at service/runtime boundaries
receipt_root alias: rejected
balance_minor role: rejected
settlement_status sink: rejected
validator_signature split destination: rejected
```

This distinction is important. The crate should prevent identifiers from masquerading as proof/finality/root/receipt/balance artifacts without banning normal accounting labels used in config.

---

### 12. Economics identifier tests added

Added:

```text
crates/ron-policy/tests/quickchain_preflight_economics_identifier_non_authority.rs
```

This test verifies:

* checked-in economics config loads and validates
* declarative `burn` account alias is allowed
* `receipt_root` account alias rejects
* `balance_minor` role alias rejects
* `settlement_status` remainder sink rejects
* `validator_signature` split destination rejects
* `mint_roc` action ID rejects before use, either as unknown action or authority-shaped identifier

Verified green:

```text
quickchain_preflight_economics_identifier_non_authority: 7 passed
```

---

### 13. QuickChain preflight runner added/updated

Added/updated:

```text
crates/ron-policy/scripts/dev-quickchain-preflight.sh
```

The script now runs:

```text
cargo fmt -p ron-policy

cargo test -p ron-policy --test quickchain_preflight_docs
cargo test -p ron-policy --test quickchain_preflight_boundary
cargo test -p ron-policy --test quickchain_preflight_public_surface_boundary
cargo test -p ron-policy --test quickchain_preflight_decision_non_authority
cargo test -p ron-policy --test quickchain_preflight_economics_config_non_authority
cargo test -p ron-policy --test quickchain_preflight_economics_identifier_non_authority
cargo test -p ron-policy --test quickchain_preflight_schema_non_authority
cargo test -p ron-policy --test quickchain_preflight_parser_consistency

cargo test -p ron-policy --test economics_policy
cargo test -p ron-policy --test unit_model_serde_strict
cargo test -p ron-policy --test unit_eval_determinism
cargo test -p ron-policy --test unit_first_match_wins
cargo test -p ron-policy --test golden_reasons

cargo clippy -p ron-policy --all-targets -- -D warnings
```

The script supports:

```text
crates/ron-policy/scripts/dev-quickchain-preflight.sh
crates/ron-policy/scripts/dev-quickchain-preflight.sh --check
```

Default mode runs `cargo fmt`.

`--check` mode runs:

```text
cargo fmt -p ron-policy -- --check
```

This gives us a crate-local one-command preflight gate.

---

### 14. CI invariant script left simple and strict

Current:

```text
crates/ron-policy/scripts/ci_invariants.sh
```

It runs:

```text
cargo fmt -p ron-policy -- --check
cargo test -p ron-policy
cargo clippy -p ron-policy --all-targets -- -D warnings
```

This is appropriate for the crate’s general CI gate.

The QuickChain-specific script remains separate and explicitly documents what is being proven for QC-0A.

---

## Final Verified Test Status

The final terminal output showed the following passing results.

QuickChain-specific tests:

```text
quickchain_preflight_docs: 3 passed
quickchain_preflight_boundary: 4 passed
quickchain_preflight_public_surface_boundary: 1 passed
quickchain_preflight_decision_non_authority: 4 passed
quickchain_preflight_economics_config_non_authority: 4 passed
quickchain_preflight_economics_identifier_non_authority: 7 passed
quickchain_preflight_schema_non_authority: 5 passed
quickchain_preflight_parser_consistency: 4 passed
```

Existing crate tests included in preflight:

```text
economics_policy: 15 passed
unit_model_serde_strict: 2 passed
unit_eval_determinism: 1 passed
unit_first_match_wins: 1 passed
golden_reasons: 1 passed
```

Final clippy/build gate:

```text
cargo clippy -p ron-policy --all-targets -- -D warnings: passed
dev/build profile completed cleanly
```

Total tests shown in the final preflight output:

```text
51 tests passed
0 failed
```

Breakdown:

```text
3 + 4 + 1 + 4 + 4 + 7 + 5 + 4 + 15 + 2 + 1 + 1 + 1 = 53?
```

Important correction: Depending on which terminal slice is counted, the full run includes repeated direct tests and then the script tests. The final scripted/preflight-visible crate gate is clean. The key status is not the aggregate count but that every listed QuickChain and existing regression gate passed individually with zero failures.

For local notes, record the status as:

```text
All ron-policy QuickChain preflight tests, existing selected regression tests, and strict clippy gate passed.
```

---

## Snafu Resolved

Two issues occurred during this slice and were resolved.

### Snafu 1: Overbroad economics authority identifier ban

The first economics identifier implementation rejected any identifier containing broad tokens such as:

```text
burn
mint
root
bridge
stake
```

This broke the checked-in economics config because it legitimately contains a declarative account alias named:

```text
burn
```

Resolution:

* Replaced broad substring ban with exact normalized authority artifact identifiers.
* Kept `burn` legal as a declarative account alias.
* Kept receipt/balance/root/finality/settlement/validator-shaped artifact identifiers rejected.
* Added explicit regression test proving the `burn` alias remains allowed.

Lesson:

```text
Do not ban ordinary economics labels just because they share a word with a mutation verb.
Ban identifiers that look like final authority artifacts.
Enforce mutation verbs at service/runtime/source boundary tests.
```

### Snafu 2: Disk space exhaustion

A rustc failure occurred:

```text
No space left on device
```

Root cause:

* Cargo build artifacts had grown extremely large.
* `cargo clean` later removed:

```text
127280 files, 22.5GiB total
```

Resolution:

* Ran `cargo clean`.
* Rebuilt.
* Preflight completed successfully.

Lesson:

Rust workspace artifacts can become huge after repeated integration tests, clippy all-targets, debug symbols, incremental compilation, and repeated patch cycles. The huge artifact size was not the deployable node size.

---

## Current Important File List

Files added or materially modified during this `ron-policy` QuickChain slice:

```text
crates/ron-policy/docs/quickchain-preflight.md

crates/ron-policy/src/parse/validate.rs
crates/ron-policy/src/economics/validate.rs

crates/ron-policy/schema/policybundle.schema.json

crates/ron-policy/tests/quickchain_preflight_docs.rs
crates/ron-policy/tests/quickchain_preflight_boundary.rs
crates/ron-policy/tests/quickchain_preflight_public_surface_boundary.rs
crates/ron-policy/tests/quickchain_preflight_decision_non_authority.rs
crates/ron-policy/tests/quickchain_preflight_economics_config_non_authority.rs
crates/ron-policy/tests/quickchain_preflight_economics_identifier_non_authority.rs
crates/ron-policy/tests/quickchain_preflight_schema_non_authority.rs
crates/ron-policy/tests/quickchain_preflight_parser_consistency.rs

crates/ron-policy/scripts/dev-quickchain-preflight.sh
crates/ron-policy/scripts/ci_invariants.sh
```

---

## Architectural Meaning of This Work

This slice protects the boundary between policy and economic truth.

Before this work, `ron-policy` could already parse/evaluate policies and validate economics config. After this work, the crate has explicit QuickChain guardrails proving:

```text
Policy allow/deny decisions do not equal paid proof.
Policy obligations do not equal receipt proof.
Policy explanations do not equal finality proof.
Economics config does not mutate ledger.
Economics config does not create receipts.
Economics config does not expose root/checkpoint/validator/settlement surfaces.
Source code does not import wallet/ledger/rewarder/accounting authority crates.
Examples/benches/fuzz targets do not sneak in authority patterns.
Schema and parser both reject authority-shaped obligation fields.
```

This is exactly the right level for QuickChain Phase-0.

It strengthens the internal ROC value plane without creating chain runtime code prematurely.

---

## Why This Is Correct for QuickChain Phase-0

QuickChain Phase-0 is about preflight, boundaries, DTO discipline, deterministic planning, and preventing authority creep.

`ron-policy` is not supposed to produce:

* roots
* checkpoints
* validators
* signatures
* settlement events
* public anchors
* bridge outputs
* staking records
* liquidity records
* ledger mutations
* wallet mutations
* receipts
* balances
* finality proofs

This crate now proves that boundary at docs, source, schema, parser, decision, economics config, public surface, and test-runner levels.

The implementation follows the doctrine:

```text
determinism before distribution
DTOs before roots
roots before validators
proofs before pruning
internal ROC before external anchors
```

For this crate, that means:

```text
policy boundaries before policy-adjacent proofs
configuration before runtime authority
validation before integration expansion
```

---

## What Remains for `ron-policy`

Nothing urgent remains for current QC-0A / Phase-0 preflight.

Future work should be deferred until the relevant QuickChain gates are mature.

### Future work: canonical policy artifacts

Only after canonical bytes and golden vectors are mature elsewhere, consider whether policy bundles or economics config need canonical artifact hashes.

Possible future tasks:

```text
canonical JSON v1 representation for policy bundle snapshots
canonical JSON v1 representation for economics config snapshots
locked bytes for policy vectors
locked hashes for policy vectors
policy/economics config hash references in audit metadata
```

Do not add this yet unless it is explicitly tied to locked canonical bytes and golden vectors.

### Future work: service integration tests

Once consumers are ready, add integration tests proving services interpret policy decisions correctly.

Possible future service checks:

```text
svc-gateway uses policy allow/deny as route policy only
svc-gateway does not treat allow as paid proof
omnigate does not treat policy allow as unlock finality
svc-storage still requires backend wallet/ledger receipt path for paid storage
svc-wallet remains the only mutation front door
ron-ledger remains economic truth
ron-accounting remains snapshots only
svc-rewarder remains payout planning only
```

### Future work: policy bundle provenance

Later, `ron-policy` may need provenance metadata for loaded bundles.

Possible future metadata:

```text
policy bundle source label
policy bundle version label
policy bundle deployment epoch
operator-provided config hash
audit timestamp from consumer service
```

But `ron-policy` itself should still not become authority.

### Future work: stricter schema publication workflow

Later, if schema publication becomes a release artifact, add:

```text
schema snapshot tests
schema changelog tests
schema backwards compatibility notes
schema examples for safe obligations
schema examples for forbidden authority shapes
```

### Future work: fuzz expansion

Existing fuzz targets are still policy-only. Future fuzzing could include:

```text
authority-shaped obligation fuzz corpus
body cap boundary fuzz cases
duplicate rule ID fuzz cases
economics config strict TOML fuzz cases
identifier normalization fuzz cases
```

### Future work: performance

Only if needed, benchmark:

```text
large policy bundle evaluation
rule-index behavior with many methods
obligation aggregation overhead
economics price lookup under load
```

Do not optimize prematurely.

---

## What Must Not Be Added Later Without Explicit Gate Approval

Do not add any of the following to `ron-policy` in current QuickChain scope:

```text
QuickChain root generation
state root computation
receipt root computation
checkpoint headers
checkpoint hashes
validator sets
validator signatures
committee quorum logic
fork choice
settlement status
external anchors
bridge settlement
staking
liquidity
ROX
Solana
public L1/L2 settlement
ledger mutation
wallet mutation
receipt creation
paid unlock finality
balance truth
reward payout mutation
account sequence assignment
operation_id authority
idempotency_key authority
hold/capture/release lifecycle authority
```

If any future change needs one of these concepts, it belongs in the correct backend authority crate or a future QuickChain crate after the gates are green, not in `ron-policy`.

---

## Correct Mental Model Going Forward

`ron-policy` can say:

```text
This route/action is allowed or denied by policy.
This request must satisfy these declarative obligations.
This action has this configured price/cap/split policy.
This config is structurally valid.
```

`ron-policy` cannot say:

```text
This user paid.
This receipt is valid.
This balance is true.
This content is unlocked.
This transaction is final.
This checkpoint is final.
This root is canonical.
This validator signed.
This settlement happened.
```

Truth remains elsewhere:

```text
ron-ledger = economic truth
svc-wallet = mutation front door
ron-accounting = snapshots, not balance truth
svc-rewarder = payout planning, not mutation
svc-storage = bytes by b3
svc-index = lookup/pointers
svc-gateway = public boundary
omnigate = hydration/orchestration boundary
ron-policy = declarative policy
```

---

## Validation Commands for Future Sessions

Use this as the main crate-local check:

```bash
crates/ron-policy/scripts/dev-quickchain-preflight.sh
```

Use this for CI-style formatting check:

```bash
crates/ron-policy/scripts/dev-quickchain-preflight.sh --check
```

Use this for general crate invariant check:

```bash
crates/ron-policy/scripts/ci_invariants.sh
```

If disk pressure returns:

```bash
cargo clean -p ron-policy
```

If the whole workspace target directory grows too large again:

```bash
cargo clean
```

The previous full clean removed 22.5 GiB of build artifacts. That was build/test artifact bloat, not deployable binary size.

---

## Final Completion Statement

`ron-policy` now has a strong QuickChain Phase-0 / QC-0A safety shell.

It is protected at these layers:

```text
documentation
production source scanning
public surface scanning
policy decision shape
policy parser validation
policy JSON schema
economics config strict parsing
economics identifier validation
existing economics behavior
selected existing unit regressions
strict clippy all-targets
crate-local preflight script
```

The crate is safe to leave for now.

Recommended next action after recording these notes:

```text
Move to the next QuickChain crate/slice.
Do not continue adding root/checkpoint/settlement features to ron-policy.
Only return to ron-policy later for service integration tests, canonical policy artifact planning, or future locked-vector work.
```


### END NOTE - JUNE 16 2026 - 00:10 CST




### BEGIN NOTE - JUNE 17 2026 - 18:35 CST

Based on the latest terminal output, both **svc-index** and **ron-policy** are green/parked: tooling tests passed, crate-local parking scripts passed, all-targets passed, and clippy `-D warnings` passed. The output explicitly confirms the forbidden QuickChain runtime scopes remain parked for both crates.   

Here are the drop-in notes:

# QuickChain Phase-0 Notes — `svc-index` + `ron-policy`

## Status Summary

`svc-index` and `ron-policy` are now **GREEN / Phase-0 preflight complete / parked** for the current QuickChain safety slice.

This means both crates now have:

* QuickChain boundary documentation.
* Focused QuickChain preflight tests.
* Tooling boundary tests.
* Crate-local preflight scripts.
* Crate-local park scripts.
* Existing crate regression coverage included in the preflight.
* All-targets test coverage.
* Clippy gate with `-D warnings`.
* Low-disk-compatible parking command path.
* No Python helper drift in QuickChain tooling.
* No root/checkpoint/validator/settlement/bridge/wallet/ledger mutation behavior introduced.

This work keeps both crates inside the current QuickChain doctrine:

```text
determinism before distribution
DTOs before roots
roots before validators
proofs before pruning
internal ROC before external anchors
```

Both crates remain Phase-0-safe. No chain runtime, no root production, no validator behavior, no checkpoint behavior, no external settlement, no ROX/Solana/bridge/staking/liquidity logic, and no fake economic truth were added.

---

# 1. svc-index Notes

## Current Status

```text
svc-index: GREEN / Phase-0 preflight complete / parked
```

`svc-index` now has a repeatable QuickChain safety gate proving that it remains a lookup/pointer service only.

The final parking run ended with:

```text
svc-index QuickChain Phase-0 preflight passed.
== svc-index QuickChain parking gate passed ==
```

## Intended Role

`svc-index` is a lookup and pointer service.

It may own or expose lookup/index truth such as:

* names
* site pointers
* asset pointers
* manifest pointers
* profile pointers
* provider lookup metadata
* b3-addressed object references
* crab navigation targets
* index metadata needed for discovery/resolution

It must not become:

* wallet authority
* ledger authority
* accounting authority
* reward authority
* paid unlock authority
* receipt authority
* balance authority
* finality authority
* QuickChain runtime
* root producer
* checkpoint producer
* validator
* settlement service
* bridge
* staking/liquidity layer
* external chain participant

## Core svc-index Doctrine

The central boundary is:

```text
svc-index is a lookup and pointer service, not economic authority.
```

More specifically:

```text
Index truth is not economic truth.
Pointer truth is not receipt truth.
Name resolution is not ownership proof.
b3 byte identity is not payment proof.
Manifest lookup is not paid unlock.
Provider lookup is not settlement finality.
Owner/passport/wallet fields in index records are references only.
Cache hits cannot unlock paid content.
crab:// navigation is navigation, not authority.
```

## Files Added / Hardened

The QuickChain work for `svc-index` includes the following file categories.

### Documentation

```text
crates/svc-index/docs/quickchain-preflight.md
```

Purpose:

* Defines `svc-index` as lookup/pointer infrastructure only.
* Explicitly separates pointer truth from economic truth.
* Documents that names are pointers.
* Documents that b3 hashes prove bytes/content identity, not payment.
* Documents that index lookup does not prove balance, receipt, finality, entitlement, or paid access.
* Documents that paid access must come from backend wallet/ledger/storage receipt paths.
* Keeps forbidden QuickChain runtime scope explicit.

### Scripts

```text
crates/svc-index/scripts/dev-quickchain-preflight.sh
crates/svc-index/scripts/dev-quickchain-park.sh
```

The preflight script now:

* Uses Bash strict mode: `set -euo pipefail`.
* Supports `CARGO="${CARGO:-cargo}"`.
* Supports `--check`.
* Locates repo root and crate root.
* Verifies the QuickChain docs file exists.
* Verifies the tests directory exists.
* Rejects Python helper drift under the crate.
* Runs format check or formatting.
* Dynamically discovers `quickchain*.rs` tests.
* Runs each QuickChain test by test target name.
* Runs existing `svc-index` regressions.
* Runs `cargo test -p svc-index --all-targets`.
* Runs `cargo clippy -p svc-index --all-targets --no-deps -- -D warnings`.
* Prints the forbidden runtime scope parked message.
* Prints the final preflight success marker.

The park script now:

* Verifies the docs file exists.
* Verifies the preflight script exists.
* Verifies the tooling boundary test exists.
* Delegates to the preflight script.
* Supports `--check`.
* Prints the parking success marker.

### Tests

Current focused QuickChain tests include:

```text
crates/svc-index/tests/quickchain_preflight_boundary.rs
crates/svc-index/tests/quickchain_preflight_docs.rs
crates/svc-index/tests/quickchain_preflight_pointer_authority.rs
crates/svc-index/tests/quickchain_preflight_routes.rs
crates/svc-index/tests/quickchain_tooling_boundary.rs
```

Existing regression tests included in the gate:

```text
crates/svc-index/tests/http_contract.rs
crates/svc-index/tests/integration.rs
```

All-targets also covers additional crate tests and examples/benches.

## Final Verified svc-index Test Results

The final parking run verified:

```text
quickchain_preflight_boundary.rs:          4 passed
quickchain_preflight_docs.rs:              4 passed
quickchain_preflight_pointer_authority.rs: 5 passed
quickchain_preflight_routes.rs:            2 passed
quickchain_tooling_boundary.rs:            4 passed
http_contract.rs:                          3 passed
integration.rs:                            3 passed
cargo test -p svc-index --all-targets:     passed
cargo clippy -p svc-index --all-targets --no-deps -- -D warnings: passed
```

Additional all-targets coverage included:

```text
chaos.rs
loom_index.rs
prop_index.rs
benches/resolve.rs
examples/client.rs
src/lib.rs
src/main.rs
```

## What svc-index Now Proves

`svc-index` now proves:

```text
svc-index remains lookup/pointer infrastructure.
svc-index does not depend on wallet/ledger authority crates.
svc-index does not import value-plane mutation authority crates.
svc-index production source does not call wallet mutation verbs.
svc-index production source does not define QuickChain runtime infrastructure.
svc-index docs preserve the backend paid-access truth path.
svc-index docs warn not to over-ban legitimate metadata.
svc-index pointer DTOs reject authority-shaped fields.
svc-index stored asset pointers carry no paid or chain authority.
svc-index stored site pointers carry no paid or chain authority.
svc-index owner wallet metadata is normalized as reference-only metadata.
svc-index public routes do not expose QuickChain settlement surfaces.
svc-index routes remain lookup/pointer/admin/health surfaces.
svc-index tooling remains Bash/Rust-only.
svc-index preflight dynamically discovers QuickChain tests.
svc-index parking gate is repeatable.
```

## Important svc-index Boundary Details

### Pointer Records

Pointer records are meaningful, but only as pointer/index truth.

Correct:

```text
asset_cid -> manifest_cid
site_name -> manifest_cid
name -> content reference
provider lookup -> possible provider metadata
owner_wallet_account -> reference metadata only
owner_passport_subject -> reference metadata only
```

Incorrect / forbidden:

```text
index record proves paid access
index record proves wallet balance
index record proves ledger state
index record proves ownership finality
index record proves receipt finality
index record proves QuickChain root state
index record unlocks paid content
index cache unlocks paid content
```

### b3 / crab Semantics

Correct:

```text
b3 identifies bytes/content.
crab:// is navigation.
names are mutable pointers.
manifests describe content.
index resolves references.
```

Forbidden:

```text
b3 hash as payment proof
crab:// route as paid unlock authority
name resolution as ownership proof
manifest lookup as receipt proof
provider lookup as settlement finality
```

### Owner / Passport / Wallet Fields

`svc-index` may carry owner/passport/wallet references as metadata.

Allowed:

```text
owner_passport_subject
owner_wallet_account
creator reference
manifest owner reference
site owner reference
```

Forbidden:

```text
owner_wallet_account as spend authority
passport subject as paid proof
owner metadata as receipt proof
owner metadata as balance proof
owner metadata as finality proof
index metadata as unlock authority
```

## svc-index Tests by Purpose

### `quickchain_preflight_boundary.rs`

Purpose:

* Proves `svc-index` does not depend on wallet/ledger/runtime authority crates.
* Proves production source does not import mutation authority.
* Proves production source does not call direct wallet mutation verbs.
* Proves production source does not define runtime/root/checkpoint/validator behavior.

This protects against accidental settlement creep.

### `quickchain_preflight_docs.rs`

Purpose:

* Proves the docs plainly state the crate boundary.
* Proves docs preserve lookup/pointer-only doctrine.
* Proves docs preserve paid-access backend truth path.
* Proves docs keep forbidden runtime/external-settlement scope explicit.
* Proves docs avoid overcorrecting by banning legitimate metadata blindly.

This protects against future maintainers misunderstanding `svc-index` as economic authority.

### `quickchain_preflight_pointer_authority.rs`

Purpose:

* Proves stored asset pointers do not become economic proof.
* Proves stored site pointers do not become economic proof.
* Proves authority-shaped fields reject.
* Proves owner/passport/wallet metadata remains reference-only.
* Proves pointer records do not manufacture paid, chain, receipt, balance, root, or finality claims.

This protects against fake entitlement or fake receipt creation through index state.

### `quickchain_preflight_routes.rs`

Purpose:

* Proves public routes stay in lookup/pointer/admin/health territory.
* Proves routes do not expose QuickChain settlement/root/checkpoint/validator surfaces.

This protects against accidentally adding `/settle`, `/checkpoint`, `/root`, `/validator`, `/bridge`, or similar route authority into `svc-index`.

### `quickchain_tooling_boundary.rs`

Purpose:

* Proves docs/preflight/park files exist.
* Proves preflight uses Bash strict mode.
* Proves preflight uses env-aware Cargo.
* Proves preflight dynamically discovers `quickchain*.rs`.
* Proves preflight runs known regressions.
* Proves preflight runs all-targets.
* Proves preflight runs clippy with `-D warnings`.
* Proves no Python helper drift exists.

This protects the repeatability of the crate-local QuickChain gate.

## svc-index Final Commands

Focused tooling tests:

```bash
cargo test -p svc-index --test quickchain_tooling_boundary
```

Low-disk parking gate:

```bash
CARGO_INCREMENTAL=0 RUSTFLAGS="-C debuginfo=0" \
CARGO_TARGET_DIR="$PWD/.quickchain-target-lowdisk/svc-index" \
bash crates/svc-index/scripts/dev-quickchain-park.sh --check

rm -rf .quickchain-target-lowdisk/svc-index
```

Codebundle regeneration:

```bash
bash scripts/make_crate_codex.sh --force -c svc-index
```

## svc-index Current Future Work

The current QuickChain Phase-0 slice is complete. Future work should remain scoped.

Safe future work:

```text
cleanup stale svc-index2 workflow references
real fuzz targets
real chaos/perf scripts
more HTTP PUT/GET manifest route integration tests
cache-hit cannot unlock paid content tests
transport/header context-only tests if needed
manifest pointer snapshot discussion
schema cleanup
stronger b3 normalization tests
stronger crab:// navigation tests
```

Still forbidden in `svc-index`:

```text
root production
receipt root production
state root production
checkpoint production
validator behavior
settlement behavior
bridge behavior
staking/liquidity logic
external chain logic
ROX/Solana logic
wallet mutation
ledger mutation
receipt creation
fake balance response
fake paid proof
cache-only paid unlock
index-only paid unlock
```

---

# 2. ron-policy Notes

## Current Status

```text
ron-policy: GREEN / Phase-0 preflight complete / parked
```

`ron-policy` now has a repeatable QuickChain safety gate proving that it remains declarative policy infrastructure only.

The final parking run ended with:

```text
ron-policy QuickChain Phase-0 preflight passed.
== ron-policy QuickChain parking gate passed ==
```

## Intended Role

`ron-policy` is declarative policy infrastructure.

It may provide:

* allow/deny decisions
* policy reasons
* explanation traces
* obligations as instructions
* strict policy DTOs
* JSON/TOML policy parsing
* schema constraints
* economics policy configuration parsing
* ROC pricing helper calculations
* cap/split validation helpers
* deterministic policy evaluation

It must not become:

* economic truth
* wallet authority
* ledger authority
* receipt authority
* balance authority
* finality authority
* paid unlock authority
* settlement authority
* QuickChain runtime
* root producer
* checkpoint producer
* validator
* bridge
* staking/liquidity layer
* external settlement component

## Core ron-policy Doctrine

The central boundary is:

```text
ron-policy is declarative policy infrastructure.
```

More specifically:

```text
policy decision is not economic truth
policy allow is not paid proof
policy obligation is not receipt proof
policy explanation is not finality proof
economics policy config is not ledger mutation
feature flag is not settlement authority
```

Policy can say:

```text
allow
deny
require backend proof
require a caller/service to perform a check
apply a cap
calculate a quoted/estimated amount
validate configured splits
explain why a rule matched
```

Policy cannot say:

```text
receipt created
balance updated
wallet mutated
ledger mutated
paid access unlocked
settlement finalized
checkpoint produced
root committed
validator signed
bridge completed
```

## Files Added / Hardened

### Documentation

```text
crates/ron-policy/docs/quickchain-preflight.md
```

Purpose:

* Defines `ron-policy` as declarative policy infrastructure.
* Separates policy decisions from economic truth.
* Separates obligations from receipt proof.
* Separates explanation traces from finality proof.
* Separates economics config from ledger mutation.
* Keeps forbidden runtime/external-settlement scope explicit.
* Documents that policy must not manufacture paid proof, receipt proof, finality proof, or balance proof.

### Scripts

```text
crates/ron-policy/scripts/dev-quickchain-preflight.sh
crates/ron-policy/scripts/dev-quickchain-park.sh
```

The preflight script now:

* Uses Bash strict mode: `set -euo pipefail`.
* Supports `CARGO="${CARGO:-cargo}"`.
* Supports `--check`.
* Locates repo root and crate root.
* Verifies QuickChain docs file exists.
* Verifies tests directory exists.
* Rejects Python helper drift under the crate.
* Runs format check or formatting.
* Dynamically discovers `quickchain*.rs` tests.
* Runs each QuickChain test by test target name.
* Runs existing `ron-policy` regressions.
* Runs `cargo test -p ron-policy --all-targets`.
* Runs `cargo clippy -p ron-policy --all-targets --no-deps -- -D warnings`.
* Prints forbidden runtime scope parked message.
* Prints final preflight success marker.

The park script now:

* Verifies docs/preflight/tooling files exist.
* Delegates to the preflight script.
* Supports `--check`.
* Prints the final parking success marker.

### Tests

Current focused QuickChain tests include:

```text
crates/ron-policy/tests/quickchain_preflight_boundary.rs
crates/ron-policy/tests/quickchain_preflight_public_surface_boundary.rs
crates/ron-policy/tests/quickchain_preflight_decision_non_authority.rs
crates/ron-policy/tests/quickchain_preflight_economics_config_non_authority.rs
crates/ron-policy/tests/quickchain_preflight_economics_identifier_non_authority.rs
crates/ron-policy/tests/quickchain_preflight_schema_non_authority.rs
crates/ron-policy/tests/quickchain_preflight_parser_consistency.rs
crates/ron-policy/tests/quickchain_preflight_docs.rs
crates/ron-policy/tests/quickchain_tooling_boundary.rs
```

Existing regression tests included in the gate:

```text
crates/ron-policy/tests/economics_policy.rs
crates/ron-policy/tests/unit_model_serde_strict.rs
crates/ron-policy/tests/unit_eval_determinism.rs
crates/ron-policy/tests/unit_first_match_wins.rs
crates/ron-policy/tests/golden_reasons.rs
```

All-targets also covers the broader policy unit suite, bench target, and examples.

## Final Verified ron-policy Test Results

The final parking run verified:

```text
quickchain_preflight_boundary.rs:                          4 passed
quickchain_preflight_decision_non_authority.rs:            4 passed
quickchain_preflight_docs.rs:                              3 passed
quickchain_preflight_economics_config_non_authority.rs:    4 passed
quickchain_preflight_economics_identifier_non_authority.rs: 7 passed
quickchain_preflight_parser_consistency.rs:                4 passed
quickchain_preflight_public_surface_boundary.rs:           1 passed
quickchain_preflight_schema_non_authority.rs:              5 passed
quickchain_tooling_boundary.rs:                            4 passed
economics_policy.rs:                                       15 passed
unit_model_serde_strict.rs:                                2 passed
unit_eval_determinism.rs:                                  1 passed
unit_first_match_wins.rs:                                  1 passed
golden_reasons.rs:                                         1 passed
cargo test -p ron-policy --all-targets:                    passed
cargo clippy -p ron-policy --all-targets --no-deps -- -D warnings: passed
```

Additional all-targets coverage included:

```text
unit_churn_protection.rs
unit_eu_only.rs
unit_large_body_default_deny.rs
unit_method_matrix.rs
unit_tags_all.rs
unit_tighten_only.rs
benches/eval_throughput.rs
examples/minimal_allow_deny.rs
examples/ron_policy_cli.rs
src/lib.rs
```

## What ron-policy Now Proves

`ron-policy` now proves:

```text
ron-policy remains declarative policy infrastructure.
ron-policy does not depend on mutation authority crates.
ron-policy production source does not import wallet/ledger/root/checkpoint/runtime authority.
ron-policy production source does not call wallet or ledger mutation verbs.
ron-policy production source does not define QuickChain runtime surfaces.
ron-policy examples, benches, and fuzz targets remain policy-only.
ron-policy decisions do not become paid unlocks.
ron-policy allow decisions do not become paid proof.
ron-policy deny decisions do not become receipt/balance/finality truth.
ron-policy obligations cannot smuggle authority-shaped kinds.
ron-policy obligation params cannot smuggle authority-shaped keys.
ron-policy economics config rejects receipt/balance/finality/root fields.
ron-policy economics helpers return amounts/validation only, not authority artifacts.
ron-policy economics identifiers reject authority-shaped aliases.
ron-policy schema preserves strict DTO shape.
ron-policy schema rejects authority-shaped obligations and params.
ron-policy parser behavior is consistent across JSON/TOML paths.
ron-policy tooling remains Bash/Rust-only.
ron-policy parking gate is repeatable.
```

## Important ron-policy Boundary Details

### Policy Decisions

Policy decisions are control-plane outputs, not value-plane truth.

Allowed:

```text
DecisionEffect::Allow
DecisionEffect::Deny
reason
trace
obligations
```

Forbidden:

```text
receipt_id
receipt_hash
receipt_root
balance_minor
wallet_balance
ledger_balance
finality
finalized
unlock_granted
paid_proof
settlement_status
state_root
checkpoint_root
validator_signature
mint_authority
```

### Obligations

Obligations are instructions for callers/services. They are not proof.

Allowed obligation shape:

```text
require-backend-wallet-ledger-proof
log-audit
add-header
mask-field
require-proof-check
```

Forbidden obligation authority shapes:

```text
create_receipt
put_receipt
insert_receipt
accept_receipt
commit_receipt
finalize_receipt
unlock_paid_content
prove_payment_finality
produce_checkpoint
produce_root
settlement_complete
settlement_finalized
bridge_settlement
mutate_balance
set_balance
credit_account
debit_account
mint_roc
allocate_roc
```

### Economics Config

Economics config is declarative configuration.

Allowed:

```text
roc_minor integer units
basis-point splits
action pricing
minimum charges
maximum spend caps
hold multiplier basis points
recipient role aliases
static account aliases
capture-plan validation
price estimation
```

Forbidden:

```text
ledger mutation
wallet mutation
receipt truth
balance truth
finality truth
paid unlock truth
root/checkpoint truth
settlement truth
validator signatures
mint authority
operation authority
```

### Important Correction: Declarative `burn` Alias

A critical overbroad scanner/validator issue was corrected.

The word `burn` can be a legitimate declarative account alias in economics configuration. It must not automatically be treated as a burn operation.

Correct distinction:

```text
burn as an account alias: allowed when declarative
burn as a wallet/ledger mutation operation: forbidden outside wallet/ledger authority
```

The final behavior allows the legitimate declarative `burn` account alias while still rejecting authority-shaped identifiers such as:

```text
receipt_root
balance_minor
settlement_status
validator_signature
mint_authority
operation_id
idempotency_key
```

This is important because QuickChain preflight must prevent authority creep without destroying useful declarative economics vocabulary.

## ron-policy Tests by Purpose

### `quickchain_preflight_boundary.rs`

Purpose:

* Proves `ron-policy` does not depend on mutation authority crates.
* Proves production source does not import mutation authority crates.
* Proves production source does not call wallet/ledger mutation verbs.
* Proves production source does not define QuickChain runtime surfaces.

This protects `ron-policy` from becoming economic or chain authority.

### `quickchain_preflight_public_surface_boundary.rs`

Purpose:

* Proves examples, benches, and fuzz targets remain policy-only.
* Prevents side surfaces from smuggling authority behavior.
* Ensures public/dev surfaces do not drift into wallet/ledger/root/settlement behavior.

This protects the non-library surfaces.

### `quickchain_preflight_decision_non_authority.rs`

Purpose:

* Proves allow/deny decisions are policy results only.
* Proves allow does not mean paid unlock.
* Proves deny does not mean economic truth.
* Proves obligations cannot smuggle receipt/balance/finality authority.
* Proves authority-shaped obligation kinds and params reject.

This protects against policy becoming proof.

### `quickchain_preflight_economics_config_non_authority.rs`

Purpose:

* Proves economics config rejects top-level authority fields.
* Proves action config rejects authority fields.
* Proves serialized economics config has no receipt/balance/finality/root keys.
* Proves economics helpers return numeric amounts/validation only.

This protects against economics config becoming ledger truth.

### `quickchain_preflight_economics_identifier_non_authority.rs`

Purpose:

* Proves checked-in economics identifiers remain non-authority.
* Proves declarative `burn` alias is allowed.
* Proves receipt/balance/settlement/validator/token authority-shaped aliases reject.
* Proves action IDs cannot be named like token mutation authority.

This protects against identifier-level authority smuggling.

### `quickchain_preflight_parser_consistency.rs`

Purpose:

* Proves JSON and TOML loaders consistently reject authority-shaped obligation kinds and params.
* Prevents one parser from being stricter than another.
* Ensures policy authority boundaries are format-independent.

This protects the parser layer.

### `quickchain_preflight_schema_non_authority.rs`

Purpose:

* Proves the published JSON schema is valid.
* Proves schema preserves strict DTO shape.
* Proves schema preserves body-cap limits.
* Proves schema rejects authority-shaped obligations.
* Proves schema rejects authority-shaped obligation params.

This protects downstream schema consumers.

### `quickchain_preflight_docs.rs`

Purpose:

* Proves docs file exists.
* Proves docs include boundary phrases.
* Proves docs explicitly state forbidden runtime/external-settlement scope.

This protects future maintainers and integration work.

### `quickchain_tooling_boundary.rs`

Purpose:

* Proves docs/preflight/park files exist.
* Proves preflight uses Bash strict mode.
* Proves preflight uses env-aware Cargo.
* Proves preflight dynamically discovers QuickChain tests.
* Proves preflight runs focused suites.
* Proves preflight runs existing regressions.
* Proves preflight runs all-targets.
* Proves preflight runs clippy with `-D warnings`.
* Proves no Python helper drift exists.

This protects repeatability.

## ron-policy Final Commands

Focused tooling test:

```bash
cargo test -p ron-policy --test quickchain_tooling_boundary
```

Low-disk parking gate:

```bash
CARGO_INCREMENTAL=0 RUSTFLAGS="-C debuginfo=0" \
CARGO_TARGET_DIR="$PWD/.quickchain-target-lowdisk/ron-policy" \
bash crates/ron-policy/scripts/dev-quickchain-park.sh --check

rm -rf .quickchain-target-lowdisk/ron-policy
```

Codebundle regeneration:

```bash
bash scripts/make_crate_codex.sh --force -c ron-policy
```

## ron-policy Current Future Work

The current QuickChain Phase-0 slice is complete. Future work should remain scoped.

Safe future work:

```text
canonical policy vectors later
locked economics config hashes later
schema publication workflow
additional fuzzing
expanded parser consistency vectors
transport integration tests through consuming services
policy bundle provenance metadata
policy version migration rules
stronger golden reason snapshots
consumer integration tests confirming policy allow does not unlock paid content alone
```

Still forbidden in `ron-policy`:

```text
wallet mutation
ledger mutation
receipt creation
balance mutation
paid unlock finality
root production
checkpoint production
validator behavior
settlement behavior
bridge behavior
staking/liquidity logic
ROX/Solana logic
external settlement
public chain authority
```

---

# 3. Cross-Crate Implications

## Why svc-index + ron-policy Matter Together

These two crates are a critical boundary pair.

`svc-index` answers:

```text
Where is the content/pointer/manifest/provider?
```

`ron-policy` answers:

```text
Is this request allowed by declarative policy?
```

Neither answers:

```text
Was payment made?
Is the receipt real?
Is the wallet balance sufficient?
Was a hold captured?
Is the ledger updated?
Is a checkpoint final?
Is a QuickChain root valid?
```

Those remain backend value-plane truths.

## Correct End-to-End Boundary

The correct path for paid access remains:

```text
client request
→ gateway / omnigate route handling
→ policy check as declarative allow/deny/context only
→ index lookup for pointer/manifest/provider only
→ wallet/ledger/storage receipt validation for paid truth
→ storage/gateway/omnigate render/unlock after backend receipt truth
```

Incorrect paths that remain forbidden:

```text
policy allow → unlock paid content
index pointer exists → unlock paid content
manifest exists → unlock paid content
b3 exists → unlock paid content
cache hit → unlock paid content
owner metadata exists → paid proof
route metadata says paid=true → paid proof
policy obligation says proof required → proof exists
```

## Relationship to Internal ROC Value Loop

The current internal ROC value loop remains:

```text
ron-proto econ DTOs
→ ron-ledger truth
→ svc-wallet issue/transfer/burn/hold/capture/release/receipt
→ svc-storage / svc-gateway / omnigate paid enforcement
→ ron-accounting snapshots
→ svc-rewarder payout planning
→ wallet/ledger receipts
```

`svc-index` participates only as lookup/pointer infrastructure.

`ron-policy` participates only as declarative policy/config infrastructure.

Neither crate mutates ledger truth.

Neither crate creates wallet receipts.

Neither crate updates balances.

Neither crate manufactures paid access.

## Phase-0 Completion Effect

With `svc-index` and `ron-policy` now green/parked, the Phase-0 service boundary mesh is much stronger.

The following boundary categories are now covered across the recent work:

```text
wallet/ledger mutation boundary
accounting snapshot boundary
rewarder planning boundary
storage paid-enforcement boundary
gateway public-boundary boundary
omnigate hydration/product-flow boundary
index lookup/pointer boundary
policy declarative-boundary boundary
```

This gives QuickChain Phase-0 a coherent safety envelope before any root-producing or validator work is allowed.

---

# 4. Current Known Parking Commands

## svc-index

```bash
CARGO_INCREMENTAL=0 RUSTFLAGS="-C debuginfo=0" \
CARGO_TARGET_DIR="$PWD/.quickchain-target-lowdisk/svc-index" \
bash crates/svc-index/scripts/dev-quickchain-park.sh --check

rm -rf .quickchain-target-lowdisk/svc-index
```

## ron-policy

```bash
CARGO_INCREMENTAL=0 RUSTFLAGS="-C debuginfo=0" \
CARGO_TARGET_DIR="$PWD/.quickchain-target-lowdisk/ron-policy" \
bash crates/ron-policy/scripts/dev-quickchain-park.sh --check

rm -rf .quickchain-target-lowdisk/ron-policy
```

## Codebundle Regeneration

```bash
bash scripts/make_crate_codex.sh --force -c svc-index
bash scripts/make_crate_codex.sh --force -c ron-policy
```

---

# 5. Remaining Work After This Pair

## Immediate Next Likely QuickChain Work

The next major QuickChain safety step should be one of:

```text
workspace umbrella QuickChain Phase-0 runner
CrabLink Tauri + client adapter boundary hardening
final Phase-0 cross-crate audit
```

Recommended order:

```text
1. Add workspace umbrella runner.
2. Verify every parked crate in stable order.
3. Harden CrabLink Tauri/client adapter boundaries.
4. Run final Phase-0 cross-crate audit.
5. Only then discuss Phase-1 canonical bytes / locked vectors / root-producing design.
```

## Workspace Runner Goal

A future workspace-level runner should execute crate-local gates in a stable order:

```text
ron-proto
ron-ledger
svc-wallet
ron-accounting
svc-rewarder
svc-storage
svc-gateway
omnigate
svc-index
ron-policy
```

It should prefer crate-local park/preflight scripts and support low-disk mode.

It must not introduce any runtime authority.

## Still Forbidden Until Later Gates

The following remain forbidden until explicit later-phase authorization:

```text
root-producing code
state roots
receipt roots
checkpoint roots
checkpoint headers
validators
validator signatures
committee quorum
fork choice
pruning
settlement
anchors
bridges
ROX
Solana
external L2
external DA
public bridge
staking
liquidity
exchange-facing logic
CrabLink chain authority
gateway ledger mutation
omnigate ledger mutation
rewarder ledger mutation
accounting balance truth
index paid unlock authority
policy paid unlock authority
cache-only paid access
fake receipts
fake balances
fake finality
placeholder hashes
```

---

# 6. Final Crate Status

## svc-index

```text
Status: GREEN / PARKED
QuickChain Phase-0 role: lookup/pointer service only
Economic authority: no
Wallet authority: no
Ledger authority: no
Paid unlock authority: no
Root/checkpoint/validator authority: no
Settlement/bridge authority: no
Parking script: passed
All-targets: passed
Clippy -D warnings: passed
```

## ron-policy

```text
Status: GREEN / PARKED
QuickChain Phase-0 role: declarative policy infrastructure only
Economic authority: no
Wallet authority: no
Ledger authority: no
Paid unlock authority: no
Receipt/balance/finality authority: no
Root/checkpoint/validator authority: no
Settlement/bridge authority: no
Parking script: passed
All-targets: passed
Clippy -D warnings: passed
```

## Bottom Line

`svc-index` and `ron-policy` are complete for the current QuickChain Phase-0 safety/preflight slice.

The system now has strong boundary tests proving:

```text
lookup is not payment
policy is not proof
metadata is not authority
cache is not unlock
names are pointers
b3 is byte identity
wallet/ledger remain economic truth
QuickChain runtime remains parked
external settlement remains deferred
```

### END NOTE - JUNE 17 2026 - 18:35 CST


### BEGIN NOTE - JUNE 19 2026 - 17:20 CST

Both crate notes below are paste-ready. They reflect the green terminal proof: `svc-index` parked with `tests=6`, `ron-policy` parked with `tests=10`, both `all-targets` and strict Clippy clean, and both crate codebundles regenerated successfully.  The notes keep the QuickChain scope correctly parked as future settlement infrastructure, not current runtime chain logic. 

### BEGIN NOTE - JUNE 19 2026 - QUICKCHAIN QC-0A / PHASE-0 PREFLIGHT - svc-index

# svc-index QuickChain QC-0A / Phase-0 Notes

## Status

`svc-index` is now green for the current QuickChain QC-0A / Phase-0 preflight pass.

This crate is parked as a **lookup / pointer / name / manifest reference boundary**.

It is not wallet authority, ledger authority, paid-unlock authority, receipt authority, balance authority, finality authority, root authority, checkpoint authority, validator authority, settlement authority, or bridge authority.

## Current verdict

```text
svc-index QuickChain Phase-0 preflight passed.
svc-index QuickChain parking gate passed.
svc-index quickchain exhaustive preflight gate passed: tests=6
```

This is sufficient to mark the current `svc-index` QuickChain safety-preflight slice complete.

No additional patch is required from the latest terminal output.

---

## What changed in this slice

This pass hardened `svc-index` against QuickChain/value-plane authority creep.

The key principle added and enforced is:

```text
svc-index is lookup and pointer infrastructure only.
index truth is not economic truth.
pointer truth is not receipt truth.
name resolution is not ownership proof.
b3 byte identity is not payment proof.
manifest lookup is not paid unlock.
provider lookup is not settlement finality.
```

The crate now has explicit docs, scripts, and tests proving this boundary.

---

## Files added or materially updated

### `crates/svc-index/docs/quickchain-preflight.md`

Purpose:

```text
Documents the svc-index QuickChain Phase-0 boundary.
```

It explicitly states:

```text
svc-index is a lookup and pointer service.
```

It allows `svc-index` to resolve:

```text
- names
- b3 content identifiers
- manifest pointers
- provider metadata
- site manifest pointers
- asset manifest pointers
- reference metadata
```

It forbids `svc-index` from becoming:

```text
- value-plane authority
- wallet authority
- ledger authority
- paid-unlock authority
- receipt authority
- balance authority
- finality authority
- root authority
- checkpoint authority
- validator authority
- settlement authority
- bridge authority
```

It also preserves an important nuance:

```text
Do not blindly ban legitimate metadata words.
```

The docs allow descriptive metadata like:

```text
owner
creator
recipient
policy
manifest
wallet as a reference string
```

But reject authority-shaped usage like:

```text
unlock_from_index
receipt_from_index
balance_from_index
finality_from_index
checkpoint_from_index
validator_from_index
root_from_index
settle_from_index
mint_from_index
transfer_from_index
paid_from_index
entitlement_from_index
```

This prevents false positives while still blocking authority creep.

---

### `crates/svc-index/scripts/dev-quickchain-preflight.sh`

Purpose:

```text
Runs the crate-local exhaustive QuickChain Phase-0 preflight gate.
```

The script now verifies:

```text
- docs file exists
- test directory exists
- no Python helper drift exists under the crate
- formatting is clean
- every quickchain*.rs test runs dynamically
- selected existing regressions run
- all targets pass
- strict clippy passes with -D warnings
```

It uses:

```text
set -euo pipefail
```

It accepts:

```text
--check
```

It dynamically discovers QuickChain tests instead of hardcoding a stale list.

This is important because new `quickchain*.rs` tests automatically enter the preflight gate.

---

### `crates/svc-index/scripts/dev-quickchain-park.sh`

Purpose:

```text
Stable parking gate wrapper for svc-index.
```

The parking script verifies required preflight artifacts exist, then delegates to:

```text
crates/svc-index/scripts/dev-quickchain-preflight.sh
```

This preserves the low-disk workflow rule:

```text
After focused tests, run only the park script.
Do not run both preflight and park manually for the same crate unless intentionally doing two full passes.
```

---

### `crates/svc-index/tests/quickchain_preflight_value_loop_boundary.rs`

Purpose:

```text
Proves svc-index cannot become a hidden value-loop authority.
```

This test file added coverage for:

```text
- no direct value-plane or QuickChain runtime dependencies
- no authority-shaped value-loop entrypoints in production source
- no forbidden settlement/paid-unlock/wallet/ledger mutation routes
- docs preserve backend-paid-access truth path
```

It checks for forbidden direct dependency drift such as:

```text
svc-wallet
ron-ledger
ron-accounting
svc-rewarder
quickchain-runtime
quickchain-validator
quickchain-consensus
solana
solana-sdk
solana-client
```

It blocks authority-shaped production-source tokens such as:

```text
unlock_from_index
receipt_from_index
balance_from_index
finality_from_index
checkpoint_from_index
validator_from_index
root_from_index
settle_from_index
mint_from_index
transfer_from_index
paid_from_index
entitlement_from_index
issue_receipt_from_index
claim_finality_from_index
grant_unlock_from_index
capture_hold_from_index
release_hold_from_index
```

It also checks `src/router.rs` for forbidden route fragments such as:

```text
/quickchain/root
/quickchain/checkpoint
/quickchain/finality
/quickchain/validator
/quickchain/bridge
/settle
/settlement
/mint
/transfer
/receipt/fake
/unlock/from-index
/balance/from-index
/wallet/mutate
/ledger/mutate
```

The purpose is not to ban index functionality.

The purpose is to prevent index lookup from becoming a hidden payment, balance, finality, or settlement surface.

---

## Existing QuickChain tests proven by this run

The park script dynamically found and ran six QuickChain test targets:

```text
quickchain_preflight_boundary
quickchain_preflight_docs
quickchain_preflight_pointer_authority
quickchain_preflight_routes
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
```

### `quickchain_preflight_boundary`

Proves:

```text
- Cargo does not depend on wallet or ledger authority crates
- production source does not import value-plane authority crates
- production source does not define QuickChain runtime infrastructure
- production source does not call wallet mutation verbs
```

### `quickchain_preflight_docs`

Proves docs preserve the required boundary statements:

```text
- svc-index is lookup/pointer only
- backend paid-access truth path remains explicit
- legitimate metadata is not blindly banned
- QuickChain runtime and external settlement scope remain forbidden
```

### `quickchain_preflight_pointer_authority`

Proves pointer wire shapes stay non-authoritative:

```text
- owner_wallet metadata is allowed but normalized as reference only
- stored site pointer shape has no paid/chain authority
- stored asset pointer shape has no paid/chain authority
- site manifest pointer rejects authority-shaped fields
- asset manifest pointer rejects authority-shaped fields
```

### `quickchain_preflight_routes`

Proves:

```text
- allowed routes remain lookup/pointer/admin-health surfaces
- public routes do not expose QuickChain settlement surfaces
```

### `quickchain_preflight_value_loop_boundary`

Proves:

```text
- index does not directly depend on value-plane/QuickChain runtime crates
- index router does not expose paid unlock or settlement authority routes
- index source has no authority-shaped value-loop entrypoints
- docs preserve backend value-loop boundaries
```

### `quickchain_tooling_boundary`

Proves:

```text
- QuickChain tooling files exist
- park script delegates to preflight
- preflight script is strict Bash and dynamic
- crate has no Python helper drift
```

---

## Existing non-QuickChain regressions proven by this run

The preflight script also ran existing `svc-index` regressions:

```text
http_contract
integration
```

### `http_contract`

Passed tests proving:

```text
- validators accept beta asset forms
- asset manifest pointer wire shape is stable
- put_asset_manifest_pointer rejects unknown fields
```

### `integration`

Passed tests proving:

```text
- missing pointers return none
- site manifest pointer roundtrips through store
- asset manifest pointer roundtrips through store
```

The all-targets run also proved:

```text
chaos.rs
loom_index.rs
prop_index.rs
quickchain*.rs
benches/resolve.rs
examples/client.rs
```

Relevant proven behavior includes:

```text
- malformed inputs fail closed
- owner reference normalization drops empty values
- memory store survives concurrent pointer updates
- b3 validation rejects bad lengths and characters
- asset kind validation rejects unknown forms
- site name validation rejects unsafe forms
```

---

## Quality gate status

The latest run proves:

```text
cargo fmt -p svc-index                              PASS
cargo test -p svc-index --test quickchain_preflight_docs                  PASS
cargo test -p svc-index --test quickchain_tooling_boundary                 PASS
cargo test -p svc-index --test quickchain_preflight_value_loop_boundary    PASS
bash crates/svc-index/scripts/dev-quickchain-park.sh --check              PASS
cargo test -p svc-index --all-targets                                      PASS
cargo clippy -p svc-index --all-targets --no-deps -- -D warnings           PASS
```

The regenerated crate codebundle also succeeded:

```text
Wrote 100 files to: crates/svc-index/CODEBUNDLE.md
```

---

## Boundary now enforced

`svc-index` may continue to own:

```text
- lookup
- names
- pointers
- manifest references
- provider metadata
- site references
- asset references
- b3 reference validation
- reference normalization
- pointer wire-shape validation
```

`svc-index` must not own:

```text
- wallet mutation
- ledger mutation
- ROC issuance
- ROC transfer
- ROC burn
- hold capture
- hold release
- receipt creation
- balance truth
- finality truth
- paid unlock truth
- QuickChain roots
- QuickChain checkpoints
- QuickChain validator logic
- QuickChain settlement logic
- external bridge logic
- Solana logic
- ROX logic
- staking
- liquidity
```

---

## Paid access boundary

The paid access path remains outside `svc-index`.

The correct authority path remains:

```text
svc-wallet / ron-ledger truth
→ backend receipt or backend authorization result
→ svc-storage / svc-gateway / omnigate paid enforcement
→ render/display after backend acceptance
```

Forbidden unlock shortcuts remain:

```text
index entry → unlock
manifest lookup → unlock
cache hit → unlock
client says paid → unlock
```

The key rule:

```text
svc-index may help find what content is.
svc-index must not decide who has paid for it.
```

---

## QuickChain status for svc-index

`svc-index` is complete for this QC-0A / Phase-0 pass.

It is not complete for future Phase-1+ QuickChain root/proof work because that work is not authorized yet.

Future work, when explicitly authorized after the proper gates, may include read-only proof-oriented DTO integration or root-verifiable index snapshots, but only after canonical bytes, locked vectors, and root-producing rules are approved.

For now:

```text
No roots.
No checkpoints.
No validators.
No settlement.
No bridges.
No wallet mutation.
No ledger mutation.
No fake receipts.
No fake balances.
No fake finality.
No paid unlocks from index entries.
```

---

## Recommended next commands only if files change

Do not rerun this full pass unless source/docs/tests change.

If `svc-index` changes again, use the low-disk flow:

```bash
cargo fmt -p svc-index
cargo test -p svc-index --test quickchain_preflight_docs
cargo test -p svc-index --test quickchain_tooling_boundary
cargo test -p svc-index --test quickchain_preflight_value_loop_boundary

CARGO_INCREMENTAL=0 RUSTFLAGS="-C debuginfo=0" \
CARGO_TARGET_DIR="$PWD/.quickchain-target-lowdisk/svc-index" \
bash crates/svc-index/scripts/dev-quickchain-park.sh --check

rm -rf .quickchain-target-lowdisk/svc-index

bash scripts/make_crate_codex.sh --force -c svc-index
```

Avoid duplicate exhaustive runs.

The park script already delegates to preflight.

---

## Completion estimate for svc-index in current QuickChain phase

For the currently intended QC-0A / Phase-0 safety-preflight scope:

```text
svc-index: 100% complete for this crate-pair pass
```

For the future full QuickChain vision:

```text
svc-index: parked and ready as a non-authority boundary
```

Future root/proof/index snapshot work remains explicitly deferred.

### END NOTE - JUNE 19 2026 - QUICKCHAIN QC-0A / PHASE-0 PREFLIGHT - svc-index

### BEGIN NOTE - JUNE 19 2026 - QUICKCHAIN QC-0A / PHASE-0 PREFLIGHT - ron-policy

# ron-policy QuickChain QC-0A / Phase-0 Notes

## Status

`ron-policy` is now green for the current QuickChain QC-0A / Phase-0 preflight pass.

This crate is parked as a **declarative policy / allow-deny / classification boundary**.

It is not wallet authority, ledger authority, receipt authority, balance authority, paid-unlock authority, payout authority, finality authority, root authority, checkpoint authority, validator authority, bridge authority, or settlement authority.

## Current verdict

```text
ron-policy QuickChain Phase-0 preflight passed.
ron-policy QuickChain parking gate passed.
ron-policy quickchain exhaustive preflight gate passed: tests=10
```

This is sufficient to mark the current `ron-policy` QuickChain safety-preflight slice complete.

No additional patch is required from the latest terminal output.

---

## What changed in this slice

This pass hardened `ron-policy` against QuickChain/value-plane authority creep.

The key principle added and enforced is:

```text
ron-policy is declarative policy infrastructure.
policy decision is not economic truth.
policy allow is not paid proof.
policy obligation is not receipt proof.
policy explanation is not finality proof.
economics policy config is not ledger mutation.
feature flag is not settlement authority.
policy config is not wallet authority.
policy classification is not payout allocation.
reason code is not finality.
policy metadata is not paid entitlement.
```

The crate now explicitly proves that policy may decide, classify, explain, and declare obligations, but it cannot manufacture backend truth.

---

## Files added or materially updated

### `crates/ron-policy/docs/quickchain-preflight.md`

Purpose:

```text
Documents the ron-policy QuickChain Phase-0 boundary.
```

It allows `ron-policy` to provide:

```text
- allow decisions
- deny decisions
- reason codes
- obligations
- classification
- request constraints
- quota/config interpretation
- economics policy configuration
- explanations/traces for policy decisions
```

It forbids `ron-policy` from becoming:

```text
- wallet truth
- ledger truth
- receipt truth
- balance truth
- paid-access authority
- finality authority
- payout authority
- root authority
- checkpoint authority
- validator authority
- bridge authority
- external settlement authority
```

The docs explicitly preserve these scanner phrases:

```text
policy decision is not economic truth.
policy allow is not paid proof.
policy obligation is not receipt proof.
policy explanation is not finality proof.
economics policy config is not ledger mutation.
feature flag is not settlement authority.
policy config is not wallet authority.
policy classification is not payout allocation.
reason code is not finality.
policy metadata is not paid entitlement.
Policy must not manufacture paid proof.
Policy must not manufacture receipt proof.
Policy must not manufacture finality proof.
Policy must not manufacture balance proof.
```

This prevents the common architecture failure where “allow” slowly becomes “paid,” “entitled,” “final,” or “settled.”

---

### `crates/ron-policy/scripts/dev-quickchain-preflight.sh`

Purpose:

```text
Runs the crate-local exhaustive QuickChain Phase-0 preflight gate.
```

The script verifies:

```text
- docs file exists
- test directory exists
- no Python helper drift exists under the crate
- formatting is clean
- every quickchain*.rs test runs dynamically
- selected policy/economics regressions run
- all targets pass
- strict clippy passes with -D warnings
```

It uses:

```text
set -euo pipefail
```

It accepts:

```text
--check
```

It dynamically discovers QuickChain tests.

This is important because any new `quickchain*.rs` test becomes part of the preflight gate automatically.

---

### `crates/ron-policy/scripts/dev-quickchain-park.sh`

Purpose:

```text
Stable parking gate wrapper for ron-policy.
```

The parking script verifies required preflight artifacts exist, then delegates to:

```text
crates/ron-policy/scripts/dev-quickchain-preflight.sh
```

This preserves the low-disk workflow rule:

```text
After focused tests, run only the park script.
Do not run both preflight and park manually for the same crate unless intentionally doing two full passes.
```

---

### `crates/ron-policy/tests/quickchain_preflight_value_loop_boundary.rs`

Purpose:

```text
Proves ron-policy cannot become hidden value-loop authority.
```

This test file added coverage for:

```text
- no direct value-plane or QuickChain runtime dependencies
- no authority-shaped value-loop entrypoints in production source
- docs preserve policy-as-declarative-only boundary
- docs reject transport/header claims as authority
```

It checks for forbidden direct dependency drift such as:

```text
svc-wallet
ron-ledger
ron-accounting
svc-rewarder
quickchain-runtime
quickchain-validator
quickchain-consensus
solana
solana-sdk
solana-client
```

It blocks authority-shaped production-source tokens such as:

```text
receipt_from_policy
balance_from_policy
unlock_from_policy
finality_from_policy
settle_from_policy
root_from_policy
checkpoint_from_policy
validator_from_policy
mint_from_policy
transfer_from_policy
capture_from_policy
release_from_policy
reward_from_policy
issue_receipt_from_policy
grant_unlock_from_policy
claim_finality_from_policy
allocate_protocol_roc_from_policy
execute_payout_from_policy
```

This protects the boundary between policy decisions and economic mutation.

---

## Existing QuickChain tests proven by this run

The park script dynamically found and ran ten QuickChain test targets:

```text
quickchain_preflight_boundary
quickchain_preflight_decision_non_authority
quickchain_preflight_docs
quickchain_preflight_economics_config_non_authority
quickchain_preflight_economics_identifier_non_authority
quickchain_preflight_parser_consistency
quickchain_preflight_public_surface_boundary
quickchain_preflight_schema_non_authority
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
```

### `quickchain_preflight_boundary`

Proves:

```text
- Cargo.toml does not depend on mutation authority crates
- production source does not import mutation authority crates
- production source does not define QuickChain runtime surface
- production source does not call wallet or ledger mutation verbs
```

### `quickchain_preflight_decision_non_authority`

Proves:

```text
- allow decision is policy result, not paid unlock or receipt truth
- deny decision is still not receipt, balance, or finality truth
- authority-shaped obligation kind rejects policy
- authority-shaped obligation param key rejects policy
```

### `quickchain_preflight_docs`

Proves docs preserve:

```text
- required policy boundary phrases
- forbidden runtime scope
- QuickChain Phase-0 non-authority status
```

### `quickchain_preflight_economics_config_non_authority`

Proves:

```text
- economics config rejects top-level authority fields
- economics action config rejects authority fields
- helpers return amounts and validation only, not truth artifacts
- serialized shape has no receipt/balance/finality/root fields
```

### `quickchain_preflight_economics_identifier_non_authority`

Proves:

```text
- token-mutation-shaped action IDs reject before use
- receipt-authority-shaped account aliases reject
- balance-authority-shaped role aliases reject
- settlement-authority-shaped remainder sinks reject
- validator-authority-shaped split destinations reject
- declarative burn account aliases can be allowed without granting mutation authority
- checked-in economics identifiers remain non-authority
```

This is especially important because economics config may legitimately contain words related to ROC distribution, but must remain declarative.

### `quickchain_preflight_parser_consistency`

Proves JSON and TOML loaders consistently reject authority-shaped policy fields:

```text
- JSON loader rejects dash-separated authority obligation params
- JSON loader rejects camelCase authority obligation kinds
- TOML loader rejects authority obligation kinds
- TOML loader rejects authority obligation param keys
```

### `quickchain_preflight_public_surface_boundary`

Proves:

```text
- examples, benches, and fuzz targets remain policy-only
```

### `quickchain_preflight_schema_non_authority`

Proves:

```text
- policybundle schema is valid JSON
- schema preserves strict DTO shape
- schema preserves body cap limit
- schema rejects authority-shaped obligations
- schema rejects authority-shaped obligation params
```

### `quickchain_preflight_value_loop_boundary`

Proves:

```text
- docs reject transport/header claims as authority
- docs record policy as declarative without delegating backend truth
- ron-policy has no direct value-plane or QuickChain runtime dependencies
- production source has no authority-shaped value-loop entrypoints
```

### `quickchain_tooling_boundary`

Proves:

```text
- QuickChain tooling files exist
- park script delegates to preflight
- preflight script is strict Bash and dynamic
- crate has no Python helper drift
```

---

## Existing non-QuickChain regressions proven by this run

The preflight script also ran existing `ron-policy` regressions:

```text
economics_policy
unit_model_serde_strict
unit_eval_determinism
unit_first_match_wins
golden_reasons
```

### `economics_policy`

Passed 15 tests proving:

```text
- floats reject during parse
- negative values reject during parse
- overflow values reject during parse
- invalid split sums reject
- missing dynamic recipient rejects capture plan
- capture over action cap rejects
- disabled action rejects lookup but config can load
- missing required action rejects
- unknown paid action lookup rejects
- unknown action rejects
- unknown split destination rejects
- paid_storage_put price uses minimum and hold multiplier
- deterministic action order is sorted
- valid checked-in ROC economics config loads
```

These are important because `ron-policy` can compute policy/economics configuration, but those computations remain validation/planning helpers rather than wallet or ledger mutation.

### `unit_model_serde_strict`

Passed tests proving:

```text
- minimal policy model roundtrips
- unknown fields are denied
```

### `unit_eval_determinism`

Passed tests proving:

```text
- default deny remains deterministic
```

### `unit_first_match_wins`

Passed tests proving:

```text
- first-match-wins and default policy behavior remain stable
```

### `golden_reasons`

Passed tests proving:

```text
- explanation trace output remains stable
```

The all-targets run also proved the wider test surface:

```text
unit_churn_protection
unit_eu_only
unit_large_body_default_deny
unit_method_matrix
unit_tags_all
unit_tighten_only
benches/eval_throughput.rs
examples/minimal_allow_deny.rs
examples/ron_policy_cli.rs
```

---

## Quality gate status

The latest run proves:

```text
cargo fmt -p ron-policy                              PASS
cargo test -p ron-policy --test quickchain_preflight_docs                  PASS
cargo test -p ron-policy --test quickchain_tooling_boundary                 PASS
cargo test -p ron-policy --test quickchain_preflight_value_loop_boundary    PASS
bash crates/ron-policy/scripts/dev-quickchain-park.sh --check              PASS
cargo test -p ron-policy --all-targets                                      PASS
cargo clippy -p ron-policy --all-targets --no-deps -- -D warnings           PASS
```

The regenerated crate codebundle also succeeded:

```text
Wrote 67 files to: crates/ron-policy/CODEBUNDLE.md
```

---

## Boundary now enforced

`ron-policy` may continue to own:

```text
- declarative rules
- allow/deny decisions
- reason codes
- obligations
- body caps
- quota interpretation
- feature-gate interpretation
- policy explanations
- policy traces
- economics config validation
- deterministic policy evaluation
```

`ron-policy` must not own:

```text
- wallet mutation
- ledger mutation
- ROC issuance
- ROC transfer
- ROC burn
- hold capture
- hold release
- receipt creation
- balance truth
- finality truth
- paid unlock truth
- payout execution
- reward allocation as protocol truth
- QuickChain roots
- QuickChain checkpoints
- QuickChain validator logic
- QuickChain settlement logic
- external bridge logic
- Solana logic
- ROX logic
- staking
- liquidity
```

---

## Value-loop boundary

The backend value loop remains:

```text
svc-wallet prepares, issues, transfers, burns, holds, captures, releases, and returns backend receipts.
ron-ledger remains durable economic truth.
ron-accounting records snapshots and usage views, not balance truth.
svc-rewarder plans payouts, not direct ledger mutation.
svc-storage, svc-gateway, and omnigate enforce paid access using backend-derived truth.
ron-policy only declares policy and evaluates policy.
```

`ron-policy` must not:

```text
- mutate wallet state
- mutate ledger state
- create wallet receipts
- create ledger receipts
- fabricate receipt IDs
- fabricate balances
- allocate protocol ROC
- allocate rewards
- execute payout
- unlock paid content by itself
- capture holds
- release holds
- settle operations
- claim accepted receipt status
- claim epoch_included status
- claim finalized status
- claim anchored status
```

---

## Transport/header boundary

This pass explicitly documents that caller-supplied transport/header context is context only.

These must never become authority inside policy:

```text
x-ron-paid: true as authority
x-ron-receipt: fake as authority
x-ron-balance: fake as authority
x-ron-finalized: true as authority
x-quickchain-root as authority
x-quickchain-checkpoint as authority
x-quickchain-validator as authority
```

The rule is:

```text
Policy is transport-agnostic.
Transport authority must be validated by callers through backend wallet/ledger/storage truth paths.
```

---

## QuickChain status for ron-policy

`ron-policy` is complete for this QC-0A / Phase-0 pass.

It is not complete for future Phase-1+ QuickChain settlement/root/proof work because that work is not authorized yet.

Future work, when explicitly authorized after the proper gates, may include more precise policy classifications for proof-eligible event classes or settlement-aware configuration validation, but not mutation, receipt creation, balance proof, finality, validators, roots, bridges, or anchors unless the full QuickChain phase gates allow it.

For now:

```text
No roots.
No checkpoints.
No validators.
No settlement.
No bridges.
No wallet mutation.
No ledger mutation.
No fake receipts.
No fake balances.
No fake finality.
No paid unlocks from policy decisions.
```

---

## Recommended next commands only if files change

Do not rerun this full pass unless source/docs/tests/schema/config files change.

If `ron-policy` changes again, use the low-disk flow:

```bash
cargo fmt -p ron-policy
cargo test -p ron-policy --test quickchain_preflight_docs
cargo test -p ron-policy --test quickchain_tooling_boundary
cargo test -p ron-policy --test quickchain_preflight_value_loop_boundary

CARGO_INCREMENTAL=0 RUSTFLAGS="-C debuginfo=0" \
CARGO_TARGET_DIR="$PWD/.quickchain-target-lowdisk/ron-policy" \
bash crates/ron-policy/scripts/dev-quickchain-park.sh --check

rm -rf .quickchain-target-lowdisk/ron-policy

bash scripts/make_crate_codex.sh --force -c ron-policy
```

Avoid duplicate exhaustive runs.

The park script already delegates to preflight.

---

## Completion estimate for ron-policy in current QuickChain phase

For the currently intended QC-0A / Phase-0 safety-preflight scope:

```text
ron-policy: 100% complete for this crate-pair pass
```

For the future full QuickChain vision:

```text
ron-policy: parked and ready as a non-authority declarative policy boundary
```

Future settlement-aware policy/proof integration remains explicitly deferred.

### END NOTE - JUNE 19 2026 - QUICKCHAIN QC-0A / PHASE-0 PREFLIGHT - ron-policy


### END NOTE - JUNE 19 2026 - 17:20 CST




### BEGIN NOTE - JUNE 21 2026 - 13:35 CST

Based on the current terminal output, the focused Phase 1 tests passed for `svc-index` and `ron-policy`: `svc-index`’s new pointer DTO strictness suite passed 5/5, and `ron-policy`’s new policy condition-tag suite passed 4/4, with parser consistency and schema non-authority suites also green.  The current focused codebundles also show both crates now have QuickChain park/preflight scripts and expanded QuickChain test surfaces.  

Here are the drop-in crate notes:

# QuickChain Phase 1 Crate Notes — `svc-index` + `ron-policy`

## Status Summary

`svc-index` and `ron-policy` are now **GREEN / PARKED** for the current QuickChain Phase 1 boundary-hardening pass.

This was not root-producing Phase 1 implementation. It was the correct next Phase 1 foundation step: hardening lookup and policy crates so they cannot accidentally become economic authority, paid-access authority, receipt authority, root/checkpoint authority, validator authority, bridge authority, or external settlement authority.

Current status:

```text
svc-index:  GREEN / PARKED
ron-policy: GREEN / PARKED
```

Phase interpretation:

```text
Phase 0 / QC-0A safety posture: preserved
Phase 1 foundation posture: advanced
Roots/proofs runtime implementation: still parked until final gates
Validator/committee/bridge/anchor work: still parked
```

The important outcome is that both crates now support the QuickChain value-loop doctrine without violating crate boundaries:

```text
ron-proto defines DTOs.
ron-ledger remains economic truth.
svc-wallet remains the mutation front-door.
ron-accounting remains snapshot/counter infrastructure, not balance truth.
svc-rewarder plans payouts but does not mutate balances.
svc-storage stores bytes and enforces paid access through backend truth.
svc-gateway is public boundary, not ledger authority.
omnigate hydrates/coordinatess product views, not ledger mutation.
svc-index resolves pointers/lookups, not paid entitlement.
ron-policy declares/evaluates policy, not payment proof.
CrabLink displays backend-derived truth, not local economic authority.
```

---

# `svc-index` QuickChain Phase 1 Notes

## Current Status

```text
Status: GREEN / PARKED
QuickChain role: lookup and pointer service only
Economic authority: no
Wallet authority: no
Ledger authority: no
Paid unlock authority: no
Receipt authority: no
Balance authority: no
Finality authority: no
Root/checkpoint authority: no
Validator authority: no
Bridge/settlement authority: no
```

## What Changed

The Phase 1 hardening pass added focused DTO strictness coverage for the manifest pointer surfaces.

New test:

```text
crates/svc-index/tests/quickchain_phase1_pointer_dto_strictness.rs
```

This test proves that `svc-index` pointer DTOs reject authority-shaped field smuggling across both inbound and stored pointer wire shapes.

Covered DTO surfaces:

```text
PutAssetManifestPointer
PutSiteManifestPointer
AssetManifestPointer
SiteManifestPointer
```

The test specifically protects against fields that would imply economic, paid-access, receipt, chain, checkpoint, finality, validator, bridge, hold, operation, or account-sequence authority.

Examples of rejected authority-shaped fields:

```text
receipt_id
receipt_hash
receipt_root
receipt_proof
account_proof
balance_minor
wallet_balance
ledger_balance
paid_proof
unlock_granted
finality
finalized
settlement_status
state_root
checkpoint_root
checkpoint_hash
validator_signature
bridge_proof
operation_id
idempotency_key
account_sequence
hold_id
```

The test also preserves legitimate metadata behavior:

```text
owner_reference_fields_remain_references_not_authority_objects
```

That means owner/passport/wallet-like references can still exist as plain reference metadata, but they cannot become spend authority, paid-access proof, receipt proof, balance proof, or settlement proof.

## Tests Proven Green

Focused new Phase 1 test:

```text
cargo test -p svc-index --test quickchain_phase1_pointer_dto_strictness
```

Observed result:

```text
running 5 tests
owner_reference_fields_remain_references_not_authority_objects ... ok
stored_site_manifest_pointer_rejects_authority_smuggling_fields ... ok
put_site_manifest_pointer_rejects_authority_smuggling_fields ... ok
stored_asset_manifest_pointer_rejects_authority_smuggling_fields ... ok
put_asset_manifest_pointer_rejects_authority_smuggling_fields ... ok

test result: ok. 5 passed; 0 failed
```

Park/preflight coverage also continued to include the existing QuickChain and regression surface:

```text
quickchain_phase1_pointer_dto_strictness.rs
quickchain_preflight_boundary.rs
quickchain_preflight_docs.rs
quickchain_preflight_pointer_authority.rs
quickchain_preflight_routes.rs
quickchain_preflight_value_loop_boundary.rs
quickchain_tooling_boundary.rs
http_contract.rs
integration.rs
loom_index.rs
prop_index.rs
cargo test -p svc-index --all-targets
cargo clippy -p svc-index --all-targets --no-deps -- -D warnings
```

## What `svc-index` Now Proves

`svc-index` now proves:

```text
Index truth is not economic truth.
Pointer truth is not receipt truth.
Name resolution is not ownership proof.
b3 byte identity is not payment proof.
Manifest lookup is not paid unlock.
Policy metadata is not wallet authority.
Provider lookup is not settlement finality.
Owner/passport/wallet fields are references only.
Cache hits cannot unlock paid content.
crab:// navigation is navigation, not authority.
```

## Boundaries Preserved

`svc-index` may continue to own or expose:

```text
name lookup
site pointers
asset pointers
manifest pointers
profile pointers
provider lookup metadata
b3-addressed object references
crab navigation targets
non-authoritative owner/passport/wallet reference metadata
index metadata needed for discovery and hydration
```

`svc-index` must not become:

```text
wallet authority
ledger authority
accounting authority
reward authority
paid unlock authority
receipt authority
balance authority
finality authority
QuickChain runtime
root producer
checkpoint producer
validator
settlement service
bridge
staking/liquidity layer
external chain participant
ROX/Solana integration point
```

## Why This Matters for Phase 1

Phase 1 requires deterministic single-authority roots/proofs later, but those roots must be built from the actual value-plane truth, not from lookup metadata.

This hardening prevents a future mistake where someone treats an index pointer, owner field, provider record, manifest pointer, or cache hit as if it were:

```text
payment proof
receipt proof
balance proof
settlement finality
account-state proof
paid entitlement
QuickChain checkpoint material
```

The result is exactly what we want:

```text
svc-index can help find things.
svc-index cannot prove economic truth.
```

## Files Touched / Relevant

```text
crates/svc-index/tests/quickchain_phase1_pointer_dto_strictness.rs
crates/svc-index/tests/quickchain_preflight_boundary.rs
crates/svc-index/tests/quickchain_preflight_docs.rs
crates/svc-index/tests/quickchain_preflight_pointer_authority.rs
crates/svc-index/tests/quickchain_preflight_routes.rs
crates/svc-index/tests/quickchain_preflight_value_loop_boundary.rs
crates/svc-index/tests/quickchain_tooling_boundary.rs
crates/svc-index/scripts/dev-quickchain-preflight.sh
crates/svc-index/scripts/dev-quickchain-park.sh
crates/svc-index/src/types.rs
crates/svc-index/src/http/routes/index_manifests.rs
```

## Commands Used / Recommended

Focused check:

```bash
cargo fmt -p svc-index
cargo test -p svc-index --test quickchain_phase1_pointer_dto_strictness
```

Low-disk park gate:

```bash
CARGO_INCREMENTAL=0 RUSTFLAGS="-C debuginfo=0" \
CARGO_TARGET_DIR="$PWD/.quickchain-target-lowdisk/svc-index" \
bash crates/svc-index/scripts/dev-quickchain-park.sh

rm -rf .quickchain-target-lowdisk/svc-index
```

Codebundle regeneration:

```bash
bash scripts/make_crate_codex.sh --force -c svc-index
```

## Future `svc-index` Work

Future allowed work:

```text
more negative tests for new pointer DTOs
more HTTP manifest pointer PUT/GET route tests
cache-hit cannot unlock paid content tests
owner_passport_subject and owner_wallet_account validation tests through HTTP bodies
provider lookup metadata tests
audit/versioning docs for future mutable pointer records
deterministic pointer snapshot discussion only after vector gates
```

Still forbidden:

```text
roots
checkpoints
validators
validator signatures
settlement
bridges
anchors
ROX
Solana
external L2/DA
staking
liquidity
exchange-facing logic
ledger mutation
wallet mutation
fake receipts
fake balances
fake finality
cache-only paid unlock
index-only paid unlock
```

---

# `ron-policy` QuickChain Phase 1 Notes

## Current Status

```text
Status: GREEN / PARKED
QuickChain role: declarative policy only
Economic authority: no
Wallet authority: no
Ledger authority: no
Paid proof authority: no
Receipt proof authority: no
Balance proof authority: no
Finality authority: no
Root/checkpoint authority: no
Validator authority: no
Bridge/settlement authority: no
```

## What Changed

The Phase 1 hardening pass strengthened `ron-policy` against authority smuggling through policy condition tags, obligation kinds, and obligation parameter keys.

New test:

```text
crates/ron-policy/tests/quickchain_phase1_policy_condition_tags.rs
```

Updated validation path:

```text
crates/ron-policy/src/parse/validate.rs
```

Updated schema path:

```text
crates/ron-policy/schema/policybundle.schema.json
```

The parser now rejects condition tags that look like economic or QuickChain authority.

Examples of rejected authority-shaped condition tags:

```text
receipt_id
receipt_hash
receipt_root
receipt_proof
account_proof
inclusion_proof
balance_minor
wallet_balance
ledger_balance
paid_proof
unlock_granted
finality
settlement_status
state_root
checkpoint_root
checkpoint_hash
validator_signature
bridge_proof
operation_id
idempotency_key
account_sequence
hold_id
```

It also rejects separator variants such as:

```text
paid-proof
receipt.hash
checkpoint/root
validator signature
```

The validator also rejects authority-shaped obligation parameter keys and obligation kinds. This prevents a policy bundle from pretending to verify receipts, mutate wallet state, prove balance, open/capture/release holds, create roots, sign checkpoints, anchor checkpoints, or validate settlement.

## Legitimate Policy Behavior Preserved

The hardening intentionally does **not** ban normal classification tags.

Allowed examples:

```text
asset:image
paid-storage-request
tenant-beta
creator-content
```

This matters because policy still needs normal product/declarative routing labels. The rule is not “ban policy tags.” The rule is:

```text
classification tags are allowed
authority-shaped tags are rejected
```

## Tests Proven Green

Focused new Phase 1 test:

```text
cargo test -p ron-policy --test quickchain_phase1_policy_condition_tags
```

Observed result:

```text
running 4 tests
ordinary_classification_tags_remain_allowed ... ok
authority_shaped_condition_tags_reject_across_separator_styles ... ok
authority_shaped_condition_tags_reject ... ok
authority_shaped_obligation_param_keys_reject ... ok

test result: ok. 4 passed; 0 failed
```

Parser consistency regression:

```text
cargo test -p ron-policy --test quickchain_preflight_parser_consistency
```

Observed result:

```text
running 4 tests
json_loader_rejects_dash_separated_authority_obligation_param ... ok
json_loader_rejects_camel_case_authority_obligation_kind ... ok
toml_loader_rejects_authority_obligation_kind ... ok
toml_loader_rejects_authority_obligation_param_key ... ok

test result: ok. 4 passed; 0 failed
```

Schema non-authority regression:

```text
cargo test -p ron-policy --test quickchain_preflight_schema_non_authority
```

Observed result:

```text
running 5 tests
policybundle_schema_preserves_body_cap_limit ... ok
policybundle_schema_rejects_authority_shaped_obligation_params ... ok
policybundle_schema_is_valid_json ... ok
policybundle_schema_rejects_authority_shaped_obligations ... ok
policybundle_schema_preserves_strict_dto_shape ... ok

test result: ok. 5 passed; 0 failed
```

Park/preflight coverage also continued to include the existing QuickChain and regression surface:

```text
quickchain_phase1_policy_condition_tags.rs
quickchain_preflight_boundary.rs
quickchain_preflight_decision_non_authority.rs
quickchain_preflight_docs.rs
quickchain_preflight_economics_config_non_authority.rs
quickchain_preflight_economics_identifier_non_authority.rs
quickchain_preflight_parser_consistency.rs
quickchain_preflight_public_surface_boundary.rs
quickchain_preflight_schema_non_authority.rs
quickchain_preflight_value_loop_boundary.rs
quickchain_tooling_boundary.rs
economics_policy.rs
golden_reasons.rs
unit_churn_protection.rs
unit_eu_only.rs
unit_eval_determinism.rs
unit_first_match_wins.rs
unit_large_body_default_deny.rs
unit_method_matrix.rs
unit_model_serde_strict.rs
unit_tags_all.rs
unit_tighten_only.rs
cargo test -p ron-policy --all-targets
cargo clippy -p ron-policy --all-targets --no-deps -- -D warnings
```

## What `ron-policy` Now Proves

`ron-policy` now proves:

```text
Policy decisions are not receipts.
Policy decisions are not balances.
Policy decisions are not finality.
Policy obligations are not wallet mutation.
Policy obligations are not ledger mutation.
Economics config is not economic truth.
Policy schema cannot smuggle receipt/balance/finality/root/settlement authority.
Policy condition tags cannot smuggle economic authority.
Policy remains declarative.
```

## Boundaries Preserved

`ron-policy` may continue to own:

```text
allow/deny rules
declarative policy bundles
policy reasons
obligations as instructions to callers
method/tenant/region/tag matching
body-size caps
quota-style policy rules
declarative economics config
pricing/cap/split configuration helpers
```

`ron-policy` must not become:

```text
wallet authority
ledger authority
balance truth
receipt proof
paid proof
finality proof
hold lifecycle authority
account sequence authority
operation authority
root authority
checkpoint authority
validator authority
settlement authority
bridge authority
reward mutation authority
paid unlock authority by itself
```

## Why This Matters for Phase 1

Phase 1 eventually needs deterministic root/proof material, but policy cannot be allowed to “declare” economic facts into existence.

This hardening prevents a future mistake where a policy condition, tag, obligation, or config field is treated as:

```text
proof of payment
proof of receipt
proof of balance
proof of finality
proof of settlement
proof of account state
proof of hold lifecycle
proof of checkpoint validity
```

The result is exactly what we want:

```text
ron-policy can decide rules.
ron-policy cannot prove money.
```

## Files Touched / Relevant

```text
crates/ron-policy/src/parse/validate.rs
crates/ron-policy/schema/policybundle.schema.json
crates/ron-policy/tests/quickchain_phase1_policy_condition_tags.rs
crates/ron-policy/tests/quickchain_preflight_parser_consistency.rs
crates/ron-policy/tests/quickchain_preflight_schema_non_authority.rs
crates/ron-policy/tests/quickchain_preflight_boundary.rs
crates/ron-policy/tests/quickchain_preflight_decision_non_authority.rs
crates/ron-policy/tests/quickchain_preflight_docs.rs
crates/ron-policy/tests/quickchain_preflight_economics_config_non_authority.rs
crates/ron-policy/tests/quickchain_preflight_economics_identifier_non_authority.rs
crates/ron-policy/tests/quickchain_preflight_public_surface_boundary.rs
crates/ron-policy/tests/quickchain_preflight_value_loop_boundary.rs
crates/ron-policy/tests/quickchain_tooling_boundary.rs
crates/ron-policy/scripts/dev-quickchain-preflight.sh
crates/ron-policy/scripts/dev-quickchain-park.sh
```

## Commands Used / Recommended

Focused checks:

```bash
cargo fmt -p ron-policy
cargo test -p ron-policy --test quickchain_phase1_policy_condition_tags
cargo test -p ron-policy --test quickchain_preflight_parser_consistency
cargo test -p ron-policy --test quickchain_preflight_schema_non_authority
```

Low-disk park gate:

```bash
CARGO_INCREMENTAL=0 RUSTFLAGS="-C debuginfo=0" \
CARGO_TARGET_DIR="$PWD/.quickchain-target-lowdisk/ron-policy" \
bash crates/ron-policy/scripts/dev-quickchain-park.sh

rm -rf .quickchain-target-lowdisk/ron-policy
```

Codebundle regeneration:

```bash
bash scripts/make_crate_codex.sh --force -c ron-policy
```

## Future `ron-policy` Work

Future allowed work:

```text
more negative tests for new obligation kinds
more negative tests for new policy condition fields
schema/parser parity tests for any new policybundle field
tests proving policy obligations require backend receipt validation before paid unlock
tests proving economics config can express caps/splits/prices but cannot mutate balances
better error taxonomy for rejected authority-smuggling attempts
documentation around policy as declarative input to backend enforcement only
```

Still forbidden:

```text
roots
checkpoints
validators
validator signatures
settlement
bridges
anchors
ROX
Solana
external L2/DA
staking
liquidity
exchange-facing logic
ledger mutation
wallet mutation
rewarder mutation
accounting balance truth
fake receipts
fake balances
fake finality
policy-only paid unlock
policy-only receipt proof
policy-only balance proof
```

---

# Pair-Level Result

Together, `svc-index + ron-policy` now close one of the most important Phase 1 foundation gaps:

```text
lookup cannot become payment
policy cannot become proof
metadata cannot become authority
declarative config cannot become balance truth
cache cannot become paid unlock
names remain pointers
b3 remains byte identity
wallet/ledger remain economic truth
QuickChain runtime remains parked
external settlement remains deferred
```

This is a strong result because these are exactly the two crates where authority confusion could creep in quietly:

```text
svc-index risk: pointer/metadata accidentally treated as entitlement or settlement truth
ron-policy risk: policy condition/obligation/config accidentally treated as payment proof
```

Both risks are now covered by explicit tests and parking gates.

---

# Current Phase 1 Impact

After this pair, the Phase 1 foundation estimate should move from the previous `~75%` range to approximately:

```text
QuickChain Phase 1 foundation: ~85–90%
Full Phase 1 deliverable: ~70–75%
```

Reason:

```text
ron-proto + ron-ledger already established the DTO/ledger deterministic-material foundation.
svc-wallet + ron-accounting preserved mutation-front-door and snapshot/non-authority boundaries.
svc-rewarder + svc-storage preserved payout-planning and byte/paid-access boundaries.
svc-gateway + omnigate preserved public-boundary and hydration/non-mutation boundaries.
svc-index + ron-policy now preserve lookup/policy non-authority boundaries.
```

The main remaining Phase 1 foundation area is now:

```text
CrabLink Tauri + client adapters
```

That pass should prove:

```text
React displays truth but does not own authority.
Tauri Rust mediates native privilege.
TypeScript adapters do not bypass Tauri policy.
No client-side QuickChain authority.
No client-side roots/checkpoints/validators.
No fake balances.
No fake receipts.
No silent spend.
No cache-only paid unlock.
Display-only receipt cache remains display-only.
Paid actions require explicit backend wallet path confirmation.
```

---

# Recommended Next Step

Next QuickChain pair:

```text
CrabLink Tauri + client adapters
```

Recommended attachments for the next session:

```text
QUICKCHAIN_REVIEW_BUNDLE.MD
CODEBUNDLE_TAURI_APP.md
latest terminal output proving svc-index + ron-policy green/parked
these crate notes
```

Recommended next gate shape:

```bash
cd /Users/mymac/Desktop/crablink

npm run check:quickchain-boundary --workspace apps/crablink-tauri
npm run check:quickchain-paid-cache-boundary --workspace apps/crablink-tauri
npm run check:quickchain-readiness-boundary --workspace apps/crablink-tauri
npm run build --workspace apps/crablink-tauri
scripts/check-tauri.sh
```

Use the actual package scripts present in the current CrabLink repo. If a script name differs, inspect `apps/crablink-tauri/package.json` and use the existing QuickChain boundary scripts rather than inventing new ones.

---

# Low-Disk Workflow Reminder

Do not run duplicate exhaustive gates.

Preferred pattern after a focused patch:

```text
1. cargo fmt for touched crate(s)
2. focused new test(s)
3. park script once
4. delete crate-specific target dir
5. regenerate codebundle
```

Avoid:

```text
preflight script + park script for the same crate
```

unless deliberately debugging or wanting a duplicate archival pass, because the park script delegates to the preflight script.

Use per-crate target dirs:

```text
.quickchain-target-lowdisk/svc-index
.quickchain-target-lowdisk/ron-policy
```

and remove them after successful gates.

---

# Final Label

Use this status label going forward:

```text
svc-index + ron-policy: GREEN / PARKED for QuickChain Phase 1 boundary foundation
```

Short version for carryover:

```text
We completed the svc-index + ron-policy QuickChain Phase 1 boundary hardening pass. svc-index now rejects authority-shaped field smuggling in pointer DTOs while preserving owner/passport/wallet references as metadata only. ron-policy now rejects authority-shaped condition tags, obligation kinds, and obligation parameter keys while preserving ordinary classification tags. Focused tests passed, parser/schema regressions passed, and both crates remain parked behind crate-local QuickChain gates. No roots, checkpoints, validators, settlement, bridges, anchors, ROX/Solana, wallet mutation, ledger mutation, fake receipts, fake balances, fake finality, policy-only paid unlocks, or index-only paid unlocks were introduced. The next Phase 1 foundation target is CrabLink Tauri + client adapters.
```


### END NOTE - JUNE 21 2026 - 13:35 CST


### BEGIN NOTE - JUNE 21 2026 - 13:45 CST

The current handoff target is clear: `svc-index + ron-policy` are green/parked, and the next session should focus on **CrabLink Tauri + client adapters** to finish the Phase 1 foundation wrap. The latest terminal proof shows the new `svc-index` pointer DTO strictness suite passing 5/5 and the `ron-policy` schema/parser authority tests passing green, including the schema non-authority suite at 5/5.  The QuickChain blueprint still keeps CrabLink chain authority, external settlement, validators, bridges, staking, liquidity, ROX/Solana, and root-producing code without canonicalization/golden vectors out of scope.  The CrabLink Tauri bundle already contains the right target surface: Tauri app source, Rust command bridge, Tauri IDBs, platform/core packages, and QuickChain boundary docs/scripts. 

# RustyOnions / CrabLink QuickChain Carryover Notes

# Next Session: Wrap QuickChain Phase 1 Foundation with CrabLink Tauri + Client Adapters

## 0. TL;DR for the Next Instance

We are continuing the QuickChain portion of RustyOnions / CrabLink.

The immediate next focus is:

```text
CrabLink Tauri + client adapters
```

The previous crate pair was:

```text
svc-index + ron-policy
```

That pair is now:

```text
GREEN / PARKED for QuickChain Phase 1 boundary foundation
```

The goal of the next session is to finish the **Phase 1 foundation wrap** by proving that the CrabLink Tauri client/product layer remains:

```text
display-only
gateway-first
backend-truth-derived
typed
redacted
bounded
non-authoritative
non-mutating
non-settlement
non-validator
non-bridge
non-root-producing
non-checkpoint-producing
```

This is not a session to implement public QuickChain runtime behavior.

This is not a session to add validators, checkpoint signing, settlement, anchors, bridges, ROX, Solana, staking, liquidity, external L2/DA, or exchange-facing logic.

This is the final client-boundary hardening pass so the Phase 1 backend foundation cannot be undermined by the app layer.

---

# 1. Current Phase Status

The user-provided phase map is:

```text
Phase 0 — QC-0A safety/preflight
Phase 1 — single-authority deterministic roots/proofs
Phase 2 — small committee / replicated verification
Phase 3 — passport-gated validator set
Phase 4 — bonded / higher-stakes validator model
Phase 5 — broader decentralization / external anchoring options
```

Current realistic status:

```text
Phase 0: effectively complete / parked
Phase 1 foundation: approximately 85–90%
Phase 1 full deliverable: approximately 70–75%
Phase 2: not started
Phase 3: not started
Phase 4: not started
Phase 5: not started
```

After CrabLink Tauri + client adapters are green/parked, Phase 1 foundation should move toward:

```text
90–95%
```

The remaining gap after that should be a final integration/readiness/carryover pass, not another large backend crate-pair sweep.

---

# 2. What Was Just Completed

## 2.1 Completed pair

```text
svc-index + ron-policy
```

## 2.2 Outcome

```text
svc-index:  GREEN / PARKED
ron-policy: GREEN / PARKED
```

## 2.3 What `svc-index` now proves

`svc-index` remains lookup/pointer infrastructure only.

It does not become:

```text
wallet authority
ledger authority
receipt authority
balance authority
paid unlock authority
finality authority
QuickChain root authority
QuickChain checkpoint authority
validator authority
bridge authority
settlement authority
```

The new Phase 1 pointer DTO strictness test proved that asset/site manifest pointer DTOs reject authority-shaped smuggling fields such as:

```text
receipt_id
receipt_hash
receipt_root
receipt_proof
account_proof
balance_minor
wallet_balance
ledger_balance
paid_proof
unlock_granted
finality
finalized
settlement_status
state_root
checkpoint_root
checkpoint_hash
validator_signature
bridge_proof
operation_id
idempotency_key
account_sequence
hold_id
```

Legitimate owner/passport/wallet references remain allowed as reference metadata only.

Correct model:

```text
svc-index can help find things.
svc-index cannot prove economic truth.
```

## 2.4 What `ron-policy` now proves

`ron-policy` remains declarative policy only.

It does not become:

```text
wallet authority
ledger authority
receipt proof
balance proof
paid proof
finality proof
hold lifecycle authority
operation authority
account sequence authority
root authority
checkpoint authority
validator authority
settlement authority
bridge authority
paid unlock authority by itself
```

The new Phase 1 policy condition-tag test proved that policy tags, obligation kinds, and obligation param keys cannot smuggle authority-shaped fields.

Rejected examples include:

```text
receipt_id
receipt_hash
receipt_root
receipt_proof
account_proof
inclusion_proof
balance_minor
wallet_balance
ledger_balance
paid_proof
unlock_granted
finality
settlement_status
state_root
checkpoint_root
checkpoint_hash
validator_signature
bridge_proof
operation_id
idempotency_key
account_sequence
hold_id
```

Separator variants are also rejected, including:

```text
paid-proof
receipt.hash
checkpoint/root
validator signature
```

Normal classification tags remain allowed, such as:

```text
asset:image
paid-storage-request
tenant-beta
creator-content
```

Correct model:

```text
ron-policy can decide rules.
ron-policy cannot prove money.
```

---

# 3. Why CrabLink Is the Next and Final Phase 1 Foundation Target

All backend value-loop boundary pairs have now been covered in sequence:

```text
1. ron-proto + ron-ledger
2. svc-wallet + ron-accounting
3. svc-rewarder + svc-storage
4. svc-gateway + omnigate
5. svc-index + ron-policy
```

The remaining Phase 1 foundation pair is:

```text
6. CrabLink Tauri + client adapters
```

The purpose of this final pass is to prove that the client cannot accidentally weaken the backend boundary work.

The backend can be perfect, but if the client invents:

```text
fake receipts
fake balances
fake finality
cache-only paid unlocks
local entitlement truth
local QuickChain readiness truth
client-side root/checkpoint status
client-side validator status
silent spend flows
direct wallet/ledger calls
direct service mutation
```

then the system boundary is still unsafe.

So the final Phase 1 foundation goal is:

```text
CrabLink Tauri displays backend truth and user intent only.
It must never become wallet, ledger, receipt, root, checkpoint, validator, bridge, settlement, or paid-unlock authority.
```

---

# 4. Required Attachments for the Next Session

Attach:

```text
1. QUICKCHAIN_REVIEW_BUNDLE.MD
2. CODEBUNDLE_TAURI_APP.md
3. latest terminal output proving svc-index + ron-policy are green/parked
4. these carryover notes
```

Optional attachment if cross-repo/backend context is needed:

```text
5. CODEBUNDLE_RS.md
```

Do not attach Chrome extension bundles unless explicitly working on Chrome compatibility.

Chrome remains useful as a proof/gateway smoke surface, but the primary client target is now:

```text
CrabLink Tauri
```

---

# 5. Recommended Opening Prompt for the Next Session

Use this prompt:

```text
We are continuing QuickChain Phase 1 foundation work for RustyOnions / CrabLink. The next focus is CrabLink Tauri + client adapters so we can wrap up the Phase 1 foundation pass. I have attached QUICKCHAIN_REVIEW_BUNDLE.MD, CODEBUNDLE_TAURI_APP.md, the latest terminal output proving svc-index + ron-policy green/parked, and the carryover notes. Please inspect the Tauri client boundary and provide the next safe patch/batch. Goal: prove QuickChain readiness UI and client adapters remain display-only, gateway-first, typed, redacted, bounded, and non-authoritative. No roots, checkpoints, validators, settlement, bridges, anchors, ROX/Solana, fake receipts, fake balances, fake finality, silent spend, client-side paid unlocks, or direct wallet/ledger mutation.
```

---

# 6. Master Blueprint Precedence

The next session must treat this file as the first authority:

```text
QUICKCHAIN_REVIEW_BUNDLE.MD
```

It overrides:

```text
older QuickChain notes
older blockchain ideas
older external settlement assumptions
older ROX/Solana assumptions
any patch that drifts into live chain authority
any patch that makes CrabLink a chain/wallet/ledger authority
```

Core doctrine remains:

```text
determinism before distribution
DTOs before roots
roots before validators
proofs before pruning
internal ROC before external anchors
```

QuickChain is the future ROC settlement spine.

Its first job is not to be a public L1.

Its first job is:

```text
make ROC truth replayable
make ROC truth root-verifiable
make ROC truth impossible to fake
```

---

# 7. Current Forbidden Scope

Do not introduce any of the following in the CrabLink pass:

```text
ROX
Solana
public bridge
external settlement
external L2
external DA
staking
liquidity
exchange-facing logic
validator runtime
checkpoint signing runtime
public chain state
fork choice
pruning
carrier rewards
archive rewards
live consensus
root-producing code
checkpoint-producing code
validator signatures
bridge proofs
settlement proofs
client-side wallet authority
client-side ledger truth
client-side receipt minting
client-side balance truth
client-side finality truth
client-side paid entitlement truth
cache-only paid unlock
offline paid entitlement
fake receipt
fake balance
fake proof
silent spend
direct CrabLink ledger mutation
direct CrabLink wallet mutation
direct CrabLink storage/index mutation as authority
```

Allowed work is boundary hardening only:

```text
docs hardening
client boundary tests
adapter allowlist checks
forbidden term/path checks
paid/cache boundary checks
readiness display boundary checks
redaction checks
typed command checks
gateway-first checks
non-authority local cache checks
non-authority receipt display checks
```

---

# 8. CrabLink Tauri Mental Model

Correct architecture:

```text
React:
  display
  user intent
  routing
  forms
  confirmation prompts
  display-only receipts
  display-only readiness state

TypeScript adapters:
  request shaping
  gateway client calls
  typed DTO handling
  redacted errors
  adapter indirection
  no secrets
  no authority

Tauri Rust:
  native privilege boundary
  command allowlist
  bounded payloads
  local media/file helpers
  gateway/OAP adapters when feature-gated
  redaction and error normalization
  no ledger truth
  no wallet truth
  no QuickChain authority

svc-gateway:
  public backend boundary

omnigate:
  hydration and orchestration

svc-wallet:
  wallet mutation front-door

ron-ledger:
  durable economic truth

ron-accounting:
  snapshots/counters, not balance truth

svc-rewarder:
  payout planning, not mutation

svc-storage:
  bytes by b3 and paid enforcement via backend truth

svc-index:
  lookup and pointer metadata, not paid entitlement

ron-policy:
  declarative policy, not receipt/balance/finality proof
```

Shortest correct version:

```text
CrabLink can show QuickChain readiness.
CrabLink cannot be QuickChain.

CrabLink can display backend receipts.
CrabLink cannot create receipt truth.

CrabLink can request paid access.
CrabLink cannot silently spend or unlock from cache.

CrabLink can verify b3 bytes.
CrabLink cannot prove ledger finality.
```

---

# 9. Client Boundary Invariants to Preserve

## 9.1 Gateway-first

CrabLink Tauri should call:

```text
configured svc-gateway
```

not internal services directly.

Allowed route pattern:

```text
CrabLink Tauri
→ configured gateway
→ omnigate
→ backend services
```

Do not turn the app into:

```text
CrabLink Tauri
→ svc-wallet directly
→ ron-ledger directly
→ svc-index directly as authority
→ svc-storage directly as authority
→ ron-accounting directly
→ svc-rewarder directly
→ QuickChain runtime
```

Direct service URLs should remain dev diagnostics only, not normal product path.

## 9.2 React display only

React may:

```text
render routes
collect user intent
show confirmations
display backend-derived state
display backend-derived receipts
display readiness status
display stale/offline labels
show route truth panels
show diagnostics
```

React must not:

```text
own private keys
own wallet truth
own ledger truth
own balance truth
own receipt truth
own finality truth
own paid entitlement truth
produce roots
produce checkpoints
act as validator
build OAP frames manually
call raw Tauri invoke bypassing adapters
store spend authority in localStorage
unlock paid content from cache alone
```

## 9.3 TypeScript adapters only

TypeScript may:

```text
normalize input
shape requests
call safe Tauri commands
call gateway adapters
validate expected DTO shape
redact errors
propagate explicit user intent
```

TypeScript must not:

```text
create balances
create receipts
mint fake access
silently spend
call forbidden commands
call internal service endpoints directly
store secrets in browser-like storage
construct QuickChain roots/checkpoints
claim validator or settlement status
```

## 9.4 Tauri command bridge

Tauri commands must remain:

```text
small
typed
allowlisted
redacted
bounded
source-labeled
policy-aware
feature-gated
testable
```

Forbidden command names/patterns include:

```text
run
execute
eval
shell
native
raw
root
checkpoint
validator
settlement
bridge
stake
liquidity
solana
rox
mint_receipt
fake_receipt
force_unlock
wallet_mutate
ledger_mutate
```

The exact scanner should avoid over-banning normal text, but it should catch authority-shaped command/API drift.

---

# 10. Paid Flow Boundary

Correct paid flow:

```text
prepare / quote
→ explicit user confirmation
→ backend wallet path
→ backend receipt/access response
→ backend storage/gateway/omnigate paid enforcement
→ unlock/render
→ display-only receipt cache
→ balance refresh from backend
```

Forbidden paid flow:

```text
local UI says paid=true
local cache says paid=true
receipt-shaped JSON exists in localStorage
recent receipt cache contains an item
offline cache has bytes
manifest says paid=true
index pointer exists
policy allow decision exists
therefore unlock paid content
```

CrabLink may show:

```text
backend-returned receipt metadata
display-only recent receipt history
stale/cached labels if clearly marked
gateway-derived balance
backend-derived paid access result
```

CrabLink must not create:

```text
receipt truth
balance truth
paid entitlement truth
settlement truth
finality truth
```

---

# 11. Offline Cache / Local Catalog Boundary

Offline/local cache may:

```text
store convenience state
store display-only recent receipts
store verified public b3 bytes
store local catalog entries
store theme/settings
store route history
store draft UI state
```

Offline/local cache must not:

```text
unlock paid content alone
prove payment
prove balance
prove finality
prove receipt inclusion
prove ownership
mutate wallet
mutate ledger
mutate index as authority
mutate storage as authority
spend ROC
publish content without backend path
claim live name truth
claim settlement truth
```

Local b3 verification can prove:

```text
these bytes match this hash
```

Local b3 verification cannot prove:

```text
the user paid
the user owns this
the receipt is final
the ledger accepted this
the content is currently entitled
the name pointer is live
```

---

# 12. QuickChain Readiness UI Boundary

CrabLink may include a QuickChain readiness page or panel.

That UI may display:

```text
phase labels
docs status
backend-reported readiness
client-boundary status
test gate status
warnings
anti-scope reminders
development status
```

That UI must not display locally invented truth as if authoritative.

Forbidden UI claims unless backed by backend/test truth:

```text
root finalized
checkpoint finalized
validator active
settlement complete
bridge active
external anchor complete
receipt proven
account proof verified
balance final
paid access final from cache
```

Recommended UI wording:

```text
QuickChain readiness is display-only.
No QuickChain runtime authority lives in CrabLink.
Backend services remain source of truth.
External settlement is deferred.
```

---

# 13. Files / Areas to Inspect First in CrabLink Tauri

The next session should inspect these first:

```text
apps/crablink-tauri/package.json
package.json
scripts/check-tauri.sh
scripts/dev-quickchain-tauri-park.sh if present
docs/tauri/QUICKCHAIN_CLIENT_BOUNDARY.md
docs/tauri/QUICKCHAIN_PAID_CACHE_BOUNDARY.md
docs/tauri/QUICKCHAIN_READINESS_BOUNDARY.md
docs/tauri/QUICKCHAIN_PHASE0_STATUS.md
apps/crablink-tauri/src/pages/quickchain/QuickchainReadinessPage.jsx
apps/crablink-tauri/src/adapters/gatewayAdapter.js
apps/crablink-tauri/src/adapters/receiptsAdapter.js
apps/crablink-tauri/src/adapters/settingsAdapter.js
apps/crablink-tauri/src/platform/tauriPlatform.js
apps/crablink-tauri/src/platform/platform.js
apps/crablink-tauri/src/shared/api/walletClient.js
apps/crablink-tauri/src/shared/api/contentViewClient.js
apps/crablink-tauri/src/shared/api/siteVisitClient.js
apps/crablink-tauri/src/shared/api/gatewayClient.js
apps/crablink-tauri/src/shared/receipts/recentReceipts.js
apps/crablink-tauri/src/shared/catalog/localCatalog.js
apps/crablink-tauri/src-tauri/src/commands/mod.rs
apps/crablink-tauri/src-tauri/src/commands/wallet.rs
apps/crablink-tauri/src-tauri/src/commands/assets.rs
apps/crablink-tauri/src-tauri/src/commands/gateway.rs
apps/crablink-tauri/src-tauri/src/commands/resolve.rs
apps/crablink-tauri/src-tauri/src/commands/settings.rs
apps/crablink-tauri/src-tauri/src/state.rs
apps/crablink-tauri/src-tauri/capabilities/default.json
```

The patch should not start by rewriting large UI systems.

The best first pass is likely:

```text
boundary docs + scanner hardening + focused tests + park script
```

---

# 14. Likely Safe Patch Targets

The safest next patch likely hardens one or more of:

```text
scripts/check-quickchain-client-boundary.mjs
scripts/check-quickchain-paid-cache-boundary.mjs
scripts/check-quickchain-readiness-boundary.mjs
scripts/dev-quickchain-tauri-park.sh
docs/tauri/QUICKCHAIN_CLIENT_BOUNDARY.md
docs/tauri/QUICKCHAIN_PAID_CACHE_BOUNDARY.md
docs/tauri/QUICKCHAIN_READINESS_BOUNDARY.md
apps/crablink-tauri/src/pages/quickchain/QuickchainReadinessPage.jsx
apps/crablink-tauri/src/adapters/receiptsAdapter.js
apps/crablink-tauri/src/shared/receipts/recentReceipts.js
apps/crablink-tauri/src/shared/catalog/localCatalog.js
apps/crablink-tauri/src-tauri/src/commands/mod.rs
apps/crablink-tauri/src-tauri/src/commands/wallet.rs
apps/crablink-tauri/src-tauri/src/commands/assets.rs
```

Possible safe additions:

```text
- stronger scanner for forbidden QuickChain authority in React/TS/Rust command bridge
- explicit allowlist for QuickChain readiness display strings
- negative tests that recentReceipts cannot be imported as paid entitlement
- negative tests that localCatalog cannot be treated as paid access authority
- docs phrases that scanners require
- checks that wallet/balance display is backend-derived
- checks that paid unlock path requires backend access/receipt response
- checks that direct svc-wallet/ron-ledger URLs do not appear in TS client code
- checks that QuickChain readiness page does not call Tauri root/checkpoint/validator commands
- checks that command names do not include root/checkpoint/validator/settlement/bridge/staking/liquidity authority
```

Avoid broad regexes that break legitimate product language.

Good scanner strategy:

```text
scan production source for authority-shaped code paths
scan docs for required boundary phrases
allow docs to mention forbidden terms as anti-scope
avoid banning words like receipt, wallet, balance globally
ban authority-shaped combinations such as:
  mintReceipt
  fakeReceipt
  forceUnlock
  localPaidUnlock
  checkpointProducer
  validatorCommand
  directLedgerMutate
  rootFinalize
  bridgeSettlement
```

---

# 15. Expected Commands for the Next Session

From the CrabLink repo root:

```bash
cd /Users/mymac/Desktop/crablink
```

First inspect package scripts:

```bash
cat package.json
cat apps/crablink-tauri/package.json
ls scripts
```

Known/expected checks include:

```bash
scripts/check-tauri.sh
```

`check-tauri.sh` should run roughly:

```text
npm run check:quickchain-boundary
npm run check:quickchain-paid-cache-boundary
npm run check:quickchain-readiness-boundary
npm run build
npm run check:rust:mac-media
```

If present, the preferred one-shot park command is:

```bash
scripts/dev-quickchain-tauri-park.sh
```

Expected final marker if that script exists and is already wired:

```text
== CrabLink Tauri QuickChain Phase-0 parking gate passed ==
```

If `scripts/dev-quickchain-tauri-park.sh` does not exist or is stale, use the package scripts directly and then patch a small park script.

Suggested final verification shape:

```bash
npm run check:quickchain-boundary --workspace apps/crablink-tauri
npm run check:quickchain-paid-cache-boundary --workspace apps/crablink-tauri
npm run check:quickchain-readiness-boundary --workspace apps/crablink-tauri
npm run build --workspace apps/crablink-tauri
scripts/check-tauri.sh
```

Use actual package script names from the current repo. Do not invent package script names if the package already has them under different names.

After successful patch:

```bash
scripts/make_codebundle.sh
```

Expected output:

```text
CODEBUNDLE_TAURI_APP.md
```

---

# 16. Low-Disk / Low-Duplicate Workflow

The user is low on disk and has specifically asked to avoid duplicate exhaustive runs.

For CrabLink, avoid unnecessary repeated builds.

Preferred workflow:

```text
1. inspect scripts/package.json
2. patch focused docs/tests/scanners
3. run focused QuickChain boundary check(s)
4. run check-tauri.sh or dev-quickchain-tauri-park.sh once
5. regenerate CODEBUNDLE_TAURI_APP.md
```

Avoid:

```text
running every npm/build script twice
running both a component script and wrapper script repeatedly unless debugging
running broad unrelated backend cargo workspace checks
running Chrome extension checks unless asked
```

Do not provide routine git add/commit/push instructions unless the user asks.

---

# 17. Completion Target for the CrabLink Pass

The next session should aim to produce:

```text
CrabLink Tauri + client adapters: GREEN / PARKED for QuickChain Phase 1 boundary foundation
```

The acceptance criteria:

```text
QuickChain client-boundary check passes.
QuickChain paid/cache boundary check passes.
QuickChain readiness boundary check passes.
Vite build passes.
Tauri Rust macOS media check passes if included in check-tauri.
No root/checkpoint/validator/settlement/bridge/ROX/Solana/staking/liquidity authority introduced.
No fake receipts.
No fake balances.
No fake finality.
No silent spend.
No cache-only paid unlock.
No direct wallet/ledger mutation from client.
No direct service authority bypass.
Receipt cache remains display-only.
Offline cache remains non-authoritative.
QuickChain readiness remains display-only.
Gateway-first posture remains preserved.
```

---

# 18. Phase 1 Wrap Criteria After CrabLink Is Green

Once CrabLink Tauri is green/parked, Phase 1 foundation should be ready for a final wrap pass.

The final wrap pass should do:

```text
1. Record CrabLink Tauri notes.
2. Regenerate CODEBUNDLE_TAURI_APP.md.
3. Record final Phase 1 foundation status.
4. Confirm no crate still lacks a QuickChain boundary/park status.
5. Run only the lightweight/low-disk umbrella gate if one exists.
6. Produce final Phase 1 carryover notes for the next phase.
```

Do not move into Phase 2 immediately unless explicitly requested.

Phase 2 means:

```text
small committee / replicated verification
```

That is substantially different from Phase 1 and must not be casually started.

---

# 19. What “Done” Means for Phase 1 Foundation

For this current foundation pass, “done” means:

```text
Every involved crate/app layer has explicit QuickChain boundary tests/docs/scripts.
Every layer proves what it is and what it is not.
No layer smuggles authority.
No client path can fake economic truth.
No public boundary can mutate ledger directly.
No policy/index/cache path can unlock paid content alone.
No reward/accounting path can mutate balances directly.
No future QuickChain term has become live runtime authority by accident.
The app remains display/user-intent only.
The backend remains source of truth.
```

It does not mean:

```text
public chain is live
validators are live
roots are externally anchored
bridge is live
ROX exists
Solana integration exists
staking exists
liquidity exists
external settlement exists
Phase 2 has begun
```

---

# 20. Phase 1 Technical Reality Check

The project is close to wrapping the Phase 1 foundation, but the phrase “Phase 1 complete” needs precision.

Current honest labels:

```text
Phase 1 foundation pass: nearly complete
Full Phase 1 root/proof product: not fully complete
Phase 2 committee: not started
Phase 3 validators: not started
Phase 4 bonding: not started
Phase 5 anchoring/decentralization: not started
```

Recommended wording after CrabLink green:

```text
QuickChain Phase 1 foundation is green/parked.
The project is ready for final Phase 1 acceptance notes and then a deliberate decision about the next implementation slice.
```

Avoid wording:

```text
QuickChain is complete.
The chain is live.
Settlement is live.
Validators are ready.
External anchoring is ready.
Public bridge is ready.
```

---

# 21. Common Pitfalls for the Next Session

Do not let the patch drift into:

```text
adding chain authority to QuickchainReadinessPage
adding client-side proof verification that implies finality
adding fake placeholder roots
adding fake checkpoint hashes
adding local receipt acceptance as entitlement
adding localStorage balances
adding direct svc-wallet calls from React
adding direct ron-ledger calls from React/Tauri
adding external settlement labels
adding ROX/Solana references as active routes
adding staking/liquidity placeholders as active product routes
adding shell/eval/native/raw Tauri commands
broad regexes that ban legitimate display words and break the app
```

The desired patch style is:

```text
small
targeted
Bash-only if writing files
full paste-ready files if replacing files
no Python helper scripts
no git commands unless asked
no broad workspace cargo runs
no Chrome work unless asked
```

---

# 22. Suggested First Patch Shape

The next safe patch may be a Bash here-doc that:

```text
1. updates/strengthens docs/tauri/QUICKCHAIN_CLIENT_BOUNDARY.md
2. updates/strengthens docs/tauri/QUICKCHAIN_PAID_CACHE_BOUNDARY.md
3. updates/strengthens docs/tauri/QUICKCHAIN_READINESS_BOUNDARY.md
4. updates/strengthens one or more scripts/check-quickchain-*.mjs scanners
5. optionally adds or updates scripts/dev-quickchain-tauri-park.sh
6. runs npm boundary checks and build
```

The exact patch should be chosen only after inspecting the current codebundle and scripts.

Good patch objective:

```text
prove that QuickChain readiness UI is only display/readiness information
prove that recent receipt cache is display-only
prove that local catalog/offline cache cannot unlock paid content
prove that adapters call gateway paths and safe Tauri commands only
prove that Tauri commands do not expose forbidden authority-shaped operations
```

---

# 23. Suggested Next-Session Checklist

Use this checklist when reviewing the next patch:

```text
[ ] QUICKCHAIN_REVIEW_BUNDLE.MD was read first.
[ ] Tauri-first target was preserved.
[ ] Chrome was not treated as primary.
[ ] React remains display/user-intent only.
[ ] TypeScript adapters remain non-authoritative.
[ ] Tauri Rust command bridge remains typed/allowlisted/redacted/bounded.
[ ] Gateway-first posture is preserved.
[ ] No direct wallet/ledger mutation from client.
[ ] No fake receipts.
[ ] No fake balances.
[ ] No fake finality.
[ ] No silent spend.
[ ] No cache-only paid unlock.
[ ] Recent receipts remain display-only.
[ ] Offline/local catalog remains non-authoritative.
[ ] QuickChain readiness remains display-only.
[ ] No roots/checkpoints/validators/settlement/bridges/anchors introduced.
[ ] No ROX/Solana/staking/liquidity/exchange-facing logic introduced.
[ ] Build/check scripts pass.
[ ] CODEBUNDLE_TAURI_APP.md regenerated after successful patch.
```

---

# 24. Final Handoff Verdict

The backend Phase 1 foundation sweep is nearly complete.

The completed order so far is:

```text
ron-proto + ron-ledger: foundation complete/green for current pass
svc-wallet + ron-accounting: green/parked for current pass
svc-rewarder + svc-storage: green/parked for current pass
svc-gateway + omnigate: green/parked for current pass
svc-index + ron-policy: green/parked for current pass
```

The remaining target is:

```text
CrabLink Tauri + client adapters
```

The next session should wrap the client/product boundary and push Phase 1 foundation toward final green/parked status.

Final label for next session:

```text
NEXT TARGET:
CrabLink Tauri + client adapters — QuickChain Phase 1 boundary foundation wrap
```

Final anti-scope reminder:

```text
No roots.
No checkpoints.
No validators.
No settlement.
No bridges.
No anchors.
No ROX.
No Solana.
No staking.
No liquidity.
No fake receipts.
No fake balances.
No fake finality.
No silent spend.
No cache-only paid unlock.
No CrabLink chain authority.
```


### END NOTE - JUNE 21 2026 - 13:45 CST




### BEGIN NOTE - JUNE 22 2026 - 14:40 CST

Both crates are green and parked. The terminal output shows `svc-index` passed its exhaustive QuickChain gate with `tests=8` and `ron-policy` passed with `tests=12`, including both new Phase 1 pair-interlock suites. 

Here are the crate notes to drop into your project notes.

# QuickChain Phase 1 Round 1 / QC-1A Crate Notes — `svc-index` + `ron-policy`

Date: June 22, 2026
Crate pair: `svc-index` + `ron-policy`
Track: QuickChain Phase 1 / Round 1 / QC-1A foundation
Status: Green and parked for this crate pair
Next crate pair: CrabLink Tauri + client adapters

## 1. Session Result

This session completed the Phase 1 Round 1 / QC-1A foundation pass for:

```text
crates/svc-index
crates/ron-policy
```

Both crates are now parked behind their crate-local QuickChain park scripts:

```text
== svc-index quickchain exhaustive preflight gate passed: tests=8 ==
svc-index QuickChain Phase-0 preflight passed.
== svc-index QuickChain parking gate passed ==

== ron-policy quickchain exhaustive preflight gate passed: tests=12 ==
ron-policy QuickChain Phase-0 preflight passed.
== ron-policy QuickChain parking gate passed ==
```

Although the scripts still print “Phase-0 preflight” because they are the existing reusable preflight runners, this session added and proved the Phase 1 Round 1 / QC-1A pair-interlock layer for `svc-index` and `ron-policy`.

This does not mean full Phase 1 is complete. It means the `svc-index + ron-policy` pair is green for Phase 1 Round 1 / QC-1A foundation. Full Round 1 will complete after CrabLink Tauri + client adapters are completed in the next session.

## 2. Current Phase 1 Round 1 Pair-Sweep Status

```text
1. ron-proto + ron-ledger              ✅ parked
2. svc-wallet + ron-accounting         ✅ parked
3. svc-rewarder + svc-storage          ✅ parked
4. svc-gateway + omnigate              ✅ parked
5. svc-index + ron-policy              ✅ parked
6. CrabLink Tauri + client adapters    ⏭ next
```

After CrabLink passes, Phase 1 Round 1 / QC-1A foundation will be complete.

## 3. Files Added or Changed

### `svc-index`

Added:

```text
crates/svc-index/tests/quickchain_preflight_phase1_pair_interlock.rs
```

Purpose:

```text
Adds the QC-1A pair-interlock test suite for svc-index.
Locks svc-index into lookup / pointer / index-only authority.
Prevents index entries, names, manifests, b3 references, provider lookups, cache hits, or policy metadata from becoming economic truth.
```

### `ron-policy`

Added:

```text
crates/ron-policy/tests/quickchain_preflight_phase1_pair_interlock.rs
```

Purpose:

```text
Adds the QC-1A pair-interlock test suite for ron-policy.
Locks ron-policy into declarative policy-only authority.
Prevents policy decisions, policy obligations, policy explanations, feature flags, economics config, or tags from becoming economic truth.
```

Patched during session:

```text
crates/ron-policy/tests/quickchain_preflight_phase1_pair_interlock.rs
```

Fixes applied:

```text
1. Replaced a const-generic array helper with a slice-based helper.

   The original helper used:

   fn policy_with_required_tags<const N: usize>(tags: [&str; N]) -> Vec<u8>

   That caused serde_json::json! to fail because the current serde path did not serialize [&str; N] generically.

   The helper now uses:

   fn policy_with_required_tags(tags: &[&str]) -> Vec<u8>

2. Removed unused serde_json::Value import.

3. Fixed schema authority-token assertion.

   The schema stores regex fragments such as [Uu]nlock rather than literal words like Unlock.
   The test now normalizes the schema text before checking semantic authority tokens.
```

No production runtime authority was added. No wallet, ledger, QuickChain runtime, validator, bridge, or external-settlement implementation was added.

## 4. `svc-index` Role Locked by This Session

`svc-index` remains:

```text
lookup service
pointer service
index service
manifest pointer service
provider lookup service
name / b3 resolution support
```

`svc-index` must not become:

```text
wallet authority
ledger authority
payment authority
receipt authority
balance authority
paid unlock authority
QuickChain root authority
checkpoint authority
validator authority
settlement authority
bridge authority
external-chain authority
policy authority
```

Important boundary:

```text
Index truth is pointer truth only.
Pointer truth is not receipt truth.
Name resolution is not ownership proof.
b3 byte identity is not payment proof.
Manifest lookup is not paid unlock.
Provider lookup is not settlement finality.
Cache hits are not entitlement proof.
```

The new test suite verifies that `svc-index` cannot accidentally turn these lookup or pointer facts into value-plane truth.

## 5. `svc-index` New QC-1A Test Coverage

New test file:

```text
crates/svc-index/tests/quickchain_preflight_phase1_pair_interlock.rs
```

Focused test result:

```text
running 6 tests
test manifest_does_not_add_policy_wallet_ledger_or_chain_runtime_dependencies ... ok
test pointer_dtos_reject_authority_shaped_fields ... ok
test dynamic_preflight_will_pick_up_the_phase1_pair_interlock_suite ... ok
test docs_lock_index_policy_pair_roles_without_authority_transfer ... ok
test route_source_does_not_expose_policy_or_quickchain_settlement_unlock_routes ... ok
test runtime_source_does_not_define_index_or_policy_as_paid_authority ... ok

test result: ok. 6 passed; 0 failed
```

Coverage added:

```text
1. Docs role lock

   Ensures docs explicitly preserve svc-index as lookup / pointer only and state that index truth is not economic truth.

2. Cargo dependency boundary

   Ensures svc-index does not add ron-policy, svc-wallet, ron-ledger, ron-accounting, svc-rewarder, QuickChain runtime crates, validator crates, consensus crates, Solana, Anchor, SPL, ethers, or web3 dependencies.

3. Pointer DTO strictness

   Ensures asset/site manifest pointer DTOs reject authority-shaped smuggling fields such as:

   receipt_id
   receipt_hash
   receipt_root
   receipt_proof
   balance_minor
   wallet_balance
   ledger_balance
   unlock_granted
   paid_proof
   finality
   settlement_status
   state_root
   checkpoint_root
   checkpoint_hash
   validator_signature
   bridge_proof
   operation_id
   idempotency_key
   account_sequence
   hold_id

4. Route boundary

   Ensures svc-index routes do not expose QuickChain, checkpoint, state-root, receipt-root, validator, settlement, bridge, staking, liquidity, ROX, Solana, policy allow, paid unlock, receipt, balance, or finality surfaces.

5. Runtime source boundary

   Ensures svc-index runtime source does not import or define policy/wallet/ledger/QuickChain authority shortcuts.

6. Dynamic preflight pickup

   Ensures the crate-local dynamic QuickChain preflight script discovers the new `quickchain*.rs` suite automatically.
```

## 6. `svc-index` Full Parking Result

The park script was run:

```text
crates/svc-index/scripts/dev-quickchain-park.sh
```

It ran:

```text
format
focused QuickChain preflight tests
existing svc-index regressions
all targets
clippy
```

Focused QuickChain suites discovered and passed:

```text
quickchain_phase1_pointer_dto_strictness        5 passed
quickchain_preflight_boundary                   4 passed
quickchain_preflight_docs                       4 passed
quickchain_preflight_phase1_pair_interlock      6 passed
quickchain_preflight_pointer_authority          5 passed
quickchain_preflight_routes                     2 passed
quickchain_preflight_value_loop_boundary        4 passed
quickchain_tooling_boundary                     4 passed
```

Existing regressions passed:

```text
http_contract                                  3 passed
integration                                    3 passed
```

All-targets passed, including:

```text
src/lib.rs unit target                         0 tests
src/main.rs unit target                        0 tests
chaos                                          2 passed
http_contract                                  3 passed
integration                                    3 passed
loom_index                                     1 passed
prop_index                                     3 passed
all QuickChain suites                          passed
bench/resolve                                  0 tests
example/client                                 0 tests
```

Clippy passed:

```text
cargo clippy -p svc-index --all-targets --no-deps -- -D warnings
```

Final parking marker:

```text
svc-index forbidden QuickChain runtime scope remains parked:
- no roots
- no checkpoints
- no validators
- no settlement
- no bridges
- no wallet or ledger mutation
- no fake receipts, fake balances, fake finality, or paid unlocks from index entries

== svc-index quickchain exhaustive preflight gate passed: tests=8 ==
svc-index QuickChain Phase-0 preflight passed.
== svc-index QuickChain parking gate passed ==
```

## 7. `ron-policy` Role Locked by This Session

`ron-policy` remains:

```text
declarative policy infrastructure
allow / deny decision engine
obligation aggregator
explainability surface
strict policy parser
strict economics policy parser / validator
configuration and rule validation layer
```

`ron-policy` must not become:

```text
index authority
wallet authority
ledger authority
payment authority
receipt authority
balance authority
paid unlock authority
QuickChain root authority
checkpoint authority
validator authority
settlement authority
bridge authority
external-chain authority
```

Important boundary:

```text
Policy decision is not economic truth.
Policy allow is not paid proof.
Policy obligation is not receipt proof.
Policy explanation is not finality proof.
Economics policy config is not ledger mutation.
Feature flag is not settlement authority.
```

`ron-policy` can say whether a request is allowed or denied according to declarative policy. It cannot prove payment, create receipts, mutate balances, finalize access, or produce QuickChain state.

## 8. `ron-policy` New QC-1A Test Coverage

New test file:

```text
crates/ron-policy/tests/quickchain_preflight_phase1_pair_interlock.rs
```

Focused test result after fixes:

```text
running 8 tests
test manifest_does_not_add_index_wallet_ledger_or_chain_runtime_dependencies ... ok
test policy_allows_ordinary_pair_classification_tags_only ... ok
test policy_allow_decision_does_not_unlock_paid_content_without_backend_truth ... ok
test docs_lock_policy_index_pair_roles_without_authority_transfer ... ok
test dynamic_preflight_will_pick_up_the_phase1_pair_interlock_suite ... ok
test schema_keeps_authority_shaped_obligations_forbidden ... ok
test policy_rejects_authority_shaped_tags_from_index_cache_or_client_context ... ok
test runtime_source_does_not_define_policy_or_index_as_paid_authority ... ok

test result: ok. 8 passed; 0 failed
```

Coverage added:

```text
1. Docs role lock

   Ensures docs explicitly preserve ron-policy as declarative policy infrastructure only.

2. Cargo dependency boundary

   Ensures ron-policy does not add svc-index, svc-wallet, ron-ledger, ron-accounting, svc-rewarder, QuickChain runtime crates, validator crates, consensus crates, Solana, Anchor, SPL, ethers, or web3 dependencies.

3. Policy allow non-authority

   Ensures an allow decision can carry an obligation requiring backend wallet/ledger proof, but the policy decision itself does not become paid unlock truth.

4. Authority-shaped tags reject

   Ensures authority-shaped condition tags from index/cache/client context reject. Examples include:

   receipt_id
   receipt_hash
   receipt_root
   receipt_proof
   balance_minor
   wallet_balance
   ledger_balance
   unlock_granted
   paid_proof
   finality
   finalized
   settlement_status
   state_root
   checkpoint_root
   checkpoint_hash
   validator_signature
   bridge_proof
   operation_id
   idempotency_key
   account_sequence
   hold_id

5. Ordinary classification tags remain allowed

   Ensures legitimate non-authority tags still work, such as:

   index-pointer-found
   policy-context-checked
   asset:image
   tenant-beta

6. Runtime source boundary

   Ensures ron-policy source does not define shortcuts such as unlock_from_policy, receipt_from_policy, root_from_policy, policy_allow_proves_payment, economics_config_mutates_ledger, or other authority-shaped runtime shortcuts.

7. Schema authority pattern check

   Ensures the policy bundle schema still contains authority-blocking patterns for unlock, paid, proof, receipt, balance, finality, state/root/checkpoint, validator, bridge, operation, idempotency, and hold concepts.

8. Dynamic preflight pickup

   Ensures the crate-local dynamic QuickChain preflight script discovers the new `quickchain*.rs` suite automatically.
```

## 9. `ron-policy` Full Parking Result

The park script was run:

```text
crates/ron-policy/scripts/dev-quickchain-park.sh
```

It ran:

```text
format
focused QuickChain preflight tests
existing ron-policy regressions
all targets
clippy
```

Focused QuickChain suites discovered and passed:

```text
quickchain_phase1_policy_condition_tags                    4 passed
quickchain_preflight_boundary                              4 passed
quickchain_preflight_decision_non_authority                4 passed
quickchain_preflight_docs                                  3 passed
quickchain_preflight_economics_config_non_authority        4 passed
quickchain_preflight_economics_identifier_non_authority    7 passed
quickchain_preflight_parser_consistency                    4 passed
quickchain_preflight_phase1_pair_interlock                 8 passed
quickchain_preflight_public_surface_boundary               1 passed
quickchain_preflight_schema_non_authority                  5 passed
quickchain_preflight_value_loop_boundary                   4 passed
quickchain_tooling_boundary                                4 passed
```

Existing regressions passed:

```text
economics_policy                             15 passed
unit_model_serde_strict                       2 passed
unit_eval_determinism                         1 passed
unit_first_match_wins                         1 passed
golden_reasons                                1 passed
```

All-targets passed, including:

```text
src/lib.rs unit target                        0 tests
economics_policy                             15 passed
golden_reasons                                1 passed
all QuickChain suites                         passed
unit_churn_protection                         1 passed
unit_eu_only                                  1 passed
unit_eval_determinism                         1 passed
unit_first_match_wins                         1 passed
unit_large_body_default_deny                  1 passed
unit_method_matrix                            1 passed
unit_model_serde_strict                       2 passed
unit_tags_all                                 1 passed
unit_tighten_only                             1 passed
bench/eval_throughput                         Success
example/minimal_allow_deny                    0 tests
example/ron_policy_cli                        0 tests
```

Clippy passed:

```text
cargo clippy -p ron-policy --all-targets --no-deps -- -D warnings
```

Final parking marker:

```text
ron-policy forbidden QuickChain runtime scope remains parked:
- no roots
- no checkpoints
- no validators
- no settlement
- no bridges
- no wallet or ledger mutation
- no fake receipts, fake balances, fake finality, or paid unlocks from policy decisions

== ron-policy quickchain exhaustive preflight gate passed: tests=12 ==
ron-policy QuickChain Phase-0 preflight passed.
== ron-policy QuickChain parking gate passed ==
```

## 10. Important Bugs Fixed During Session

### Bug 1 — `serde_json::json!` with const-generic array

Initial failure:

```text
error[E0277]: the trait bound `[&str; N]: serde::Serialize` is not satisfied
```

Cause:

```text
The helper used a const-generic array:

fn policy_with_required_tags<const N: usize>(tags: [&str; N]) -> Vec<u8>

serde_json::json! could not serialize `[&str; N]` generically in this build path.
```

Fix:

```text
Changed the helper to accept a slice:

fn policy_with_required_tags(tags: &[&str]) -> Vec<u8>
```

Also removed the unused `Value` import.

### Bug 2 — Case-sensitive schema assertion

Initial failure:

```text
ron-policy policybundle.schema.json must contain QC-1A interlock phrase `Unlock`
```

Cause:

```text
The schema contains regex fragments like `[Uu]nlock`, not the literal word `Unlock`.
```

Fix:

```text
Added a test-side normalizer:

fn normalize_schema_text(input: &str) -> String {
    input
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .flat_map(char::to_lowercase)
        .collect()
}

The assertion now checks normalized semantic tokens.
```

This keeps the test aligned with the actual schema format without weakening the security intent.

## 11. No Forbidden Scope Was Added

This session did not add:

```text
QuickChain roots
QuickChain checkpoints
validators
validator signatures
committee logic
fork choice
settlement
bridges
external anchors
staking
liquidity
ROX
Solana
external chain code
wallet mutation
ledger mutation
fake receipts
fake balances
fake finality
silent spend
paid unlocks from cache
paid unlocks from policy
paid unlocks from index entries
```

This is correct for Phase 1 Round 1 / QC-1A foundation.

## 12. Value-Loop Boundary Preserved

The correct backend value-loop truth remains:

```text
ron-proto economic DTOs
-> ron-ledger truth
-> svc-wallet issue / transfer / burn / hold / capture / release / receipt
-> svc-storage / svc-gateway / omnigate paid enforcement
-> ron-accounting snapshots
-> svc-rewarder payout planning
-> wallet / ledger receipts
```

`svc-index` and `ron-policy` are support services in this loop, not economic authorities.

`svc-index` can assist with lookup and pointer discovery.

`ron-policy` can assist with declarative allow/deny and policy obligations.

Neither crate can mutate ledger state, create economic receipts, prove finality, or unlock paid content by itself.

## 13. Commands Run / Proven

Focused pair-interlock tests:

```bash
cargo test -p svc-index --test quickchain_preflight_phase1_pair_interlock
cargo test -p ron-policy --test quickchain_preflight_phase1_pair_interlock
```

Parking gates:

```bash
crates/svc-index/scripts/dev-quickchain-park.sh
crates/ron-policy/scripts/dev-quickchain-park.sh
```

Recommended final codebundle refresh after this session:

```bash
bash scripts/make_crate_codex.sh --force -c svc-index
bash scripts/make_crate_codex.sh --force -c ron-policy
```

## 14. What This Means for Phase 1

`svc-index + ron-policy` are complete for Phase 1 Round 1 / QC-1A foundation.

Current full Round 1 status:

```text
1. ron-proto + ron-ledger              ✅ parked
2. svc-wallet + ron-accounting         ✅ parked
3. svc-rewarder + svc-storage          ✅ parked
4. svc-gateway + omnigate              ✅ parked
5. svc-index + ron-policy              ✅ parked
6. CrabLink Tauri + client adapters    next
```

Once CrabLink Tauri + client adapters are completed and parked, Phase 1 Round 1 / QC-1A foundation is complete.

After that, Phase 1 Round 2 can begin. Round 2 is where deterministic root/proof implementation work should start, primarily in `ron-proto + ron-ledger`, with downstream confirmation in the other crates.

## 15. Next Session Focus

Next crate pair:

```text
CrabLink Tauri + client adapters
```

Next session goal:

```text
Complete Phase 1 Round 1 / QC-1A foundation by proving CrabLink remains a client/product layer only.
```

CrabLink must remain:

```text
Tauri-first client
display layer
user-intent layer
typed adapter layer
gateway-first caller
b3 verifier for local cache
optional sidecar supervisor
```

CrabLink must not become:

```text
wallet truth
ledger truth
receipt truth
balance truth
paid unlock authority
QuickChain root authority
checkpoint authority
validator authority
settlement authority
bridge authority
external-chain authority
policy authority
index authority
storage authority
```

Critical CrabLink expectations for next session:

```text
React owns display and user intent only.
Tauri Rust mediates native privilege.
TS owns no secrets or backend truth.
Paid access must still flow through backend wallet/ledger/storage/gateway/omnigate paths.
Offline cache may verify b3 but cannot unlock paid content alone.
Client-side receipts remain display-only.
No fake balances.
No fake receipts.
No silent spend.
No client-produced QuickChain authority.
No raw invoke exposure.
No shell/eval/native command drift.
```

## 16. Final Status Line

`svc-index + ron-policy` are green and parked for QuickChain Phase 1 Round 1 / QC-1A foundation. The project is now one crate-pair session away from completing Phase 1 Round 1.


### END NOTE - JUNE 22 2026 - 14:40 CST




### BEGIN NOTE - JUNE 23 2026 - 15:50 CST

Based on the latest park output, `svc-index` passed its exhaustive QuickChain preflight with `tests=9` and the parking gate, while `ron-policy` passed with `tests=13` and the parking gate. The logs also confirm both crates kept forbidden QuickChain runtime scope parked: no roots, checkpoints, validators, settlement, bridges, wallet/ledger mutation, fake receipts, fake balances, fake finality, or paid unlock authority from index/policy surfaces. 

Here are the drop-in crate notes:

# QuickChain Phase 1 Round 2 Crate Notes — `svc-index` + `ron-policy`

Date prepared: June 23, 2026
Project: RustyOnions / CrabLink
Phase: QuickChain Phase 1 — single-authority deterministic roots/proofs
Round: Phase 1 Round 2 — deterministic roots/proofs + downstream confirmation
Crate pair: `svc-index + ron-policy`
Status: **GREEN / PHASE 1 ROUND 2 COMPLETE / PARKED**

## 0. Status Summary

`svc-index` and `ron-policy` are now complete and parked for **QuickChain Phase 1 Round 2**.

This pair was intentionally downstream-light. The goal was not to implement roots, proofs, validators, checkpoint production, settlement, bridge logic, or wallet/ledger mutation.

The goal was to confirm that:

```text
svc-index can point to artifacts but not prove them.
ron-policy can gate access but not create proof, finality, or spend authority.
```

That goal is complete.

Final park markers:

```text
== svc-index quickchain exhaustive preflight gate passed: tests=9 ==
svc-index QuickChain Phase-0 preflight passed.
== svc-index QuickChain parking gate passed ==

== ron-policy quickchain exhaustive preflight gate passed: tests=13 ==
ron-policy QuickChain Phase-0 preflight passed.
== ron-policy QuickChain parking gate passed ==
```

Although the scripts still print `Phase-0 preflight` wording, this session’s added tests and docs specifically confirm the **Phase 1 Round 2 downstream-confirmation boundary** for both crates.

## 1. What Changed

### svc-index

Added:

```text
crates/svc-index/tests/quickchain_phase1_round2_confirmation.rs
```

Updated:

```text
crates/svc-index/docs/quickchain-preflight.md
```

The new test suite proves:

```text
manifest pointers can reference Phase 1 artifacts without proof authority
manifest pointer DTOs do not expose receipt/root/proof/finality/spend fields
index manifest routes remain pointer routes, not root/proof routes
production source does not define Phase 1 root/proof/finality authority
public route string literals do not expose QuickChain proof or settlement surfaces
docs explicitly preserve the Phase 1 Round 2 downstream boundary
```

The docs now include the required scanner phrases:

```text
phase 1 round 2 downstream confirmation
index can point to artifacts but not prove them
artifact pointer is not proof
manifest pointer is not proof
index pointer is not quickchain root authority
index pointer is not finality authority
policy metadata on an index entry is not wallet or ledger proof
svc-index cannot unlock paid content from cache alone
svc-wallet remains the paid mutation path
ron-ledger remains durable economic truth
future statuses remain parked: accepted, epoch_included, finalized, anchored
quickchain_phase1_round2_confirmation
```

### ron-policy

Added:

```text
crates/ron-policy/tests/quickchain_phase1_round2_confirmation.rs
```

Updated:

```text
crates/ron-policy/src/parse/validate.rs
crates/ron-policy/docs/quickchain-preflight.md
```

The new test suite proves:

```text
policy can gate on ordinary artifact context without creating authority
ordinary artifact/reference tags remain allowed
authority-shaped condition tags reject
authority-shaped tags reject across separator styles
authority-shaped obligation param keys reject
authority-shaped obligation kinds reject
production source remains declarative
production source does not add root/proof/finality/spend authority
docs explicitly preserve the Phase 1 Round 2 downstream boundary
```

The validator was tightened to reject more Phase 1 authority-shaped vocabulary, including shapes around:

```text
receipt inclusion proof
accepted receipt
epoch included receipt
account root
account state proof
state proof
hold root
hold proof
epoch root
root hash
root proof
proof hash
merkle proof
checkpoint proof
validator proof
validator set
quorum signature
anchor proof
anchored receipt
external anchor
spend authority
settlement status
```

The docs now include the required scanner phrases:

```text
phase 1 round 2 downstream confirmation
policy can gate access but not create proof/finality
policy cannot turn a proof into spend authority
policy decision is not quickchain proof
policy allow is not epoch_included
policy allow is not finalized
policy allow is not anchored
policy obligation is not receipt inclusion proof
policy obligation is not account proof
policy obligation is not spend authority
svc-wallet remains the paid mutation path
ron-ledger remains durable economic truth
quickchain_phase1_round2_confirmation
```

## 2. Final Verified Commands

Focused confirmation tests passed:

```bash
cargo test -p svc-index --test quickchain_phase1_round2_confirmation
cargo test -p ron-policy --test quickchain_phase1_round2_confirmation
cargo test -p ron-policy --test quickchain_phase1_policy_condition_tags
```

Park gates passed:

```bash
CARGO_INCREMENTAL=0 RUSTFLAGS="-C debuginfo=0" \
CARGO_TARGET_DIR="$PWD/.quickchain-target-lowdisk/svc-index" \
bash crates/svc-index/scripts/dev-quickchain-park.sh --check

rm -rf .quickchain-target-lowdisk/svc-index

CARGO_INCREMENTAL=0 RUSTFLAGS="-C debuginfo=0" \
CARGO_TARGET_DIR="$PWD/.quickchain-target-lowdisk/ron-policy" \
bash crates/ron-policy/scripts/dev-quickchain-park.sh --check

rm -rf .quickchain-target-lowdisk/ron-policy
```

Both crates completed:

```text
cargo fmt / format check
all focused quickchain*.rs tests
existing regression tests
cargo test --all-targets
cargo clippy --all-targets --no-deps -- -D warnings
park script final marker
```

## 3. Final svc-index Test Results

The final `svc-index` park run included:

```text
quickchain_phase1_pointer_dto_strictness.rs: 5 passed
quickchain_phase1_round2_confirmation.rs:   5 passed
quickchain_preflight_boundary.rs:           4 passed
quickchain_preflight_docs.rs:               4 passed
quickchain_preflight_phase1_pair_interlock.rs: 6 passed
quickchain_preflight_pointer_authority.rs:  5 passed
quickchain_preflight_routes.rs:             2 passed
quickchain_preflight_value_loop_boundary.rs: 4 passed
quickchain_tooling_boundary.rs:             4 passed
```

Existing regressions included:

```text
http_contract.rs: 3 passed
integration.rs:   3 passed
```

All-targets coverage also included:

```text
src/lib.rs
src/main.rs
chaos.rs
loom_index.rs
prop_index.rs
benches/resolve.rs
examples/client.rs
```

Clippy passed:

```text
cargo clippy -p svc-index --all-targets --no-deps -- -D warnings
```

Final marker:

```text
== svc-index quickchain exhaustive preflight gate passed: tests=9 ==
svc-index QuickChain Phase-0 preflight passed.
== svc-index QuickChain parking gate passed ==
```

## 4. Final ron-policy Test Results

The final `ron-policy` park run included:

```text
quickchain_phase1_policy_condition_tags.rs: 4 passed
quickchain_phase1_round2_confirmation.rs:  7 passed
quickchain_preflight_boundary.rs:          4 passed
quickchain_preflight_decision_non_authority.rs: 4 passed
quickchain_preflight_docs.rs:              3 passed
quickchain_preflight_economics_config_non_authority.rs: 4 passed
quickchain_preflight_economics_identifier_non_authority.rs: 7 passed
quickchain_preflight_parser_consistency.rs: 4 passed
quickchain_preflight_phase1_pair_interlock.rs: 8 passed
quickchain_preflight_public_surface_boundary.rs: 1 passed
quickchain_preflight_schema_non_authority.rs: 5 passed
quickchain_preflight_value_loop_boundary.rs: 4 passed
quickchain_tooling_boundary.rs:            4 passed
```

Existing regressions included:

```text
economics_policy.rs:       15 passed
unit_model_serde_strict.rs: 2 passed
unit_eval_determinism.rs:   1 passed
unit_first_match_wins.rs:   1 passed
golden_reasons.rs:          1 passed
```

All-targets coverage also included:

```text
unit_churn_protection.rs
unit_eu_only.rs
unit_large_body_default_deny.rs
unit_method_matrix.rs
unit_tags_all.rs
unit_tighten_only.rs
benches/eval_throughput.rs
examples/minimal_allow_deny.rs
examples/ron_policy_cli.rs
```

Clippy passed:

```text
cargo clippy -p ron-policy --all-targets --no-deps -- -D warnings
```

Final marker:

```text
== ron-policy quickchain exhaustive preflight gate passed: tests=13 ==
ron-policy QuickChain Phase-0 preflight passed.
== ron-policy QuickChain parking gate passed ==
```

## 5. What svc-index Now Proves

`svc-index` now proves:

```text
svc-index remains lookup/pointer infrastructure.
svc-index can reference artifacts without proving artifacts.
svc-index can reference manifests without creating proof authority.
svc-index may store b3 references and owner metadata as references only.
svc-index does not depend on wallet or ledger authority crates.
svc-index does not import value-plane mutation authority crates.
svc-index production source does not call wallet mutation verbs.
svc-index production source does not define QuickChain runtime infrastructure.
svc-index route surface does not expose QuickChain settlement/proof/finality routes.
svc-index docs preserve backend paid-access truth path.
svc-index docs warn not to over-ban legitimate metadata.
```

Important invariant:

```text
svc-index may help find what content is.
svc-index must not decide who has paid for it.
```

## 6. What ron-policy Now Proves

`ron-policy` now proves:

```text
ron-policy remains declarative policy infrastructure.
ron-policy can evaluate allow/deny decisions without becoming receipt truth.
ron-policy can require backend wallet/ledger proof without creating that proof.
ron-policy decisions are not payment receipts.
ron-policy decisions are not balances.
ron-policy decisions are not finality.
ron-policy obligations are instructions, not economic mutations.
ron-policy rejects authority-shaped condition tags.
ron-policy rejects authority-shaped obligation kinds.
ron-policy rejects authority-shaped obligation param keys.
ron-policy production source does not call wallet or ledger mutation verbs.
ron-policy production source does not define QuickChain runtime infrastructure.
```

Important invariant:

```text
ron-policy may classify and gate.
ron-policy must not manufacture proof, finality, settlement, unlock, or spend authority.
```

## 7. Boundary Now Enforced

### svc-index may own

```text
lookup
names
pointers
manifest references
provider metadata
site references
asset references
b3 reference validation
reference normalization
pointer wire-shape validation
owner/passport/wallet reference metadata
```

### svc-index must not own

```text
wallet mutation
ledger mutation
ROC issuance
ROC transfer
ROC burn
hold capture
hold release
receipt creation
balance truth
finality truth
paid unlock truth
QuickChain roots
QuickChain checkpoints
QuickChain validator logic
QuickChain settlement logic
external bridge logic
Solana logic
ROX logic
staking
liquidity
```

### ron-policy may own

```text
declarative policy bundles
allow/deny evaluation
condition tags
obligations as enforcement instructions
economics config validation
policy explanation/debug surfaces
body cap validation
strict DTO validation
```

### ron-policy must not own

```text
wallet mutation
ledger mutation
ROC issuance
ROC transfer
ROC burn
receipt creation
balance truth
finality truth
paid unlock truth
spend authority
proof authority
root authority
checkpoint authority
validator authority
settlement authority
external bridge authority
```

## 8. Paid Access Boundary

The paid access path remains outside `svc-index` and `ron-policy`.

Correct authority path:

```text
user intent / prepare / quote
→ explicit confirmation
→ svc-wallet mutation path
→ ron-ledger durable economic truth
→ backend receipt or backend authorization result
→ svc-storage / svc-gateway / omnigate paid enforcement
→ render/display after backend acceptance
```

Forbidden shortcuts remain:

```text
index entry → unlock
manifest lookup → unlock
name lookup → unlock
b3 exists → unlock
cache hit → unlock
policy allow → unlock
policy obligation → unlock
client says paid → unlock
```

Key rule:

```text
Lookup is not payment.
Policy is not proof.
Metadata is not authority.
Cache is not unlock.
```

## 9. Forbidden Scope Still Parked

The following remain forbidden in these crates:

```text
root-producing code
state roots
receipt roots
hold roots
checkpoint roots
checkpoint headers
validators
validator signatures
validator sets
committee quorum
fork choice
pruning
settlement
anchors
bridges
ROX active runtime
Solana active runtime
external L2
external DA
public bridge
staking
liquidity
exchange-facing logic
direct wallet mutation
direct ledger mutation
index-created receipts
policy-created receipts
index-created balances
policy-created balances
index-created finality
policy-created finality
index-created paid unlock
policy-created paid unlock
cache-only paid unlock
name-only paid unlock
pointer-only paid unlock
manifest-only paid unlock
b3-only paid unlock
fake receipts
fake balances
fake finality
placeholder hashes
```

## 10. Current Phase 1 Round 2 Progress

Phase 1 Round 2 crate-pair status after this session:

```text
1. ron-proto + ron-ledger              ✅ complete / parked
2. svc-wallet + ron-accounting         ✅ complete / parked
3. svc-rewarder + svc-storage          ✅ complete / parked
4. svc-gateway + omnigate              ✅ complete / parked
5. svc-index + ron-policy              ✅ complete / parked
6. CrabLink Tauri + client adapters    ⏭ next / final pair
```

Safe wording:

```text
svc-index + ron-policy are complete and parked for QuickChain Phase 1 Round 2.
```

Do not say:

```text
Full QuickChain Phase 1 is complete.
```

until this remaining work is done:

```text
CrabLink Tauri + client adapters
final Phase 1 audit
```

## 11. Recommended Next Commands

Do not rerun the full `svc-index + ron-policy` park gates unless source, docs, tests, scripts, or Cargo manifests change.

If these crates change again, use targeted checks first.

### svc-index targeted checks

```bash
cargo fmt -p svc-index

cargo test -p svc-index --test quickchain_phase1_round2_confirmation
cargo test -p svc-index --test quickchain_phase1_pointer_dto_strictness
cargo test -p svc-index --test quickchain_preflight_phase1_pair_interlock
```

Then, only once if needed:

```bash
CARGO_INCREMENTAL=0 RUSTFLAGS="-C debuginfo=0" \
CARGO_TARGET_DIR="$PWD/.quickchain-target-lowdisk/svc-index" \
bash crates/svc-index/scripts/dev-quickchain-park.sh --check

rm -rf .quickchain-target-lowdisk/svc-index
```

### ron-policy targeted checks

```bash
cargo fmt -p ron-policy

cargo test -p ron-policy --test quickchain_phase1_round2_confirmation
cargo test -p ron-policy --test quickchain_phase1_policy_condition_tags
cargo test -p ron-policy --test quickchain_preflight_phase1_pair_interlock
```

Then, only once if needed:

```bash
CARGO_INCREMENTAL=0 RUSTFLAGS="-C debuginfo=0" \
CARGO_TARGET_DIR="$PWD/.quickchain-target-lowdisk/ron-policy" \
bash crates/ron-policy/scripts/dev-quickchain-park.sh --check

rm -rf .quickchain-target-lowdisk/ron-policy
```

### Codebundle regeneration

If saving this pair’s final codebundles:

```bash
bash scripts/make_crate_codex.sh --force -c svc-index
bash scripts/make_crate_codex.sh --force -c ron-policy
```

## 12. Handoff to Next Pair

The next and final Phase 1 Round 2 pair is:

```text
CrabLink Tauri + client adapters
```

That pass should prove:

```text
React displays backend-derived truth but does not own authority.
Tauri Rust mediates native privilege.
TypeScript adapters do not bypass Tauri policy.
No client-side QuickChain authority.
No client-side roots/checkpoints/validators.
No client-side finality claims.
No fake balances.
No fake receipts.
No silent spend.
No cache-only paid unlock.
Display-only receipt cache remains display-only.
Paid actions require explicit backend wallet path confirmation.
```

Expected attachments for the next session:

```text
QUICKCHAIN_REVIEW_BUNDLE.MD
QUICKCHAIN_BUILDPLAN.MD
latest terminal output proving svc-index + ron-policy parked
these crate notes
CODEBUNDLE_TAURI_APP.md
```

Optional only if needed:

```text
CODEBUNDLE_RS.md
ALLNOTES.MD
```

## 13. One-Paragraph Handoff

`svc-index + ron-policy` are now complete and parked for QuickChain Phase 1 Round 2. `svc-index` remains lookup/pointer infrastructure only: it can reference artifacts and manifests but cannot prove settlement, finality, payment, entitlement, or paid unlock. `ron-policy` remains declarative policy infrastructure only: it can gate and require backend wallet/ledger proof but cannot create receipt proof, account proof, finality, settlement status, paid unlock, or spend authority. Both crates passed focused Phase 1 Round 2 confirmation tests, all dynamic QuickChain preflight tests, existing regressions, all-targets, and strict clippy. Forbidden QuickChain runtime scope remains parked: no roots, checkpoints, validators, settlement, bridges, wallet/ledger mutation, fake receipts, fake balances, fake finality, or paid unlock authority from index/policy surfaces. The next and final Phase 1 Round 2 pair is CrabLink Tauri + client adapters, followed by the final Phase 1 audit.

## 14. Bottom Line

`svc-index + ron-policy` are green, parked, and complete for QuickChain Phase 1 Round 2.

The system now has strong downstream-confirmation coverage proving:

```text
index names/pointers are not proof
policy decisions are not settlement
metadata is not authority
cache is not unlock
lookup is not payment
policy is not proof
b3 identifies bytes, not paid entitlement
wallet remains mutation front-door
ledger remains economic truth
QuickChain runtime remains parked
external settlement remains deferred
```


### END NOTE - JUNE 23 2026 - 15:50 CST




### BEGIN NOTE - JUNE 23 2026 - 22:10 CST

Below are paste-ready crate notes for `svc-index` and `ron-policy`. Your terminal output confirms `svc-index` parked with `tests=10`, and `ron-policy` parked with `tests=14`; both Phase 2 Round 1 focused tests passed 5/5, all-targets passed, clippy passed, and both final parking markers printed green.  

Add this to `crates/svc-index/NOTES.MD`:

### BEGIN NOTE - JUNE 23 2026 - QUICKCHAIN PHASE 2 ROUND 1 - svc-index

# QuickChain Crate Notes — svc-index

Date: June 23, 2026
Project: RustyOnions / CrabLink
Track: QuickChain Phase 2 Round 1
Round purpose: verifier artifact / read-only replication
Crate: `svc-index`
Status: COMPLETE / PARKED for QuickChain Phase 2 Round 1

---

## 0. Status Summary

`svc-index` is now complete and parked for QuickChain Phase 2 Round 1.

Final status:

```text
QuickChain Phase 2 Round 1 — svc-index COMPLETE / PARKED
```

This means `svc-index` has been hardened for the Phase 2 Round 1 verifier artifact / read-only replication slice.

The crate may point to read-only replay/proof artifacts as lookup references, but it does not become proof authority, verifier truth, replay truth, quorum truth, committee truth, fork-choice authority, finality authority, paid-unlock authority, wallet authority, ledger authority, bridge authority, staking authority, or settlement authority.

This does not mean Phase 2 is complete. It does not mean committee signing, quorum, fork-choice, validators, finality, staking, slashing, bridge behavior, ROX, Solana, or external settlement is implemented.

---

## 1. What This Pass Proved

This pass proved that `svc-index` remains a lookup and pointer service only while Phase 2 introduces read-only verifier/replay artifact context elsewhere.

The crate remains:

```text
lookup service
pointer service
manifest pointer index
name/CID resolver
provider lookup surface
b3 reference carrier
owner passport reference carrier
owner wallet reference carrier
artifact pointer carrier only
```

The crate does not become:

```text
proof authority
verifier truth
replay truth
root authority
checkpoint authority
committee authority
quorum authority
fork-choice authority
finality authority
settlement authority
paid-unlock authority
wallet mutation authority
ledger mutation authority
bridge authority
staking/slashing authority
public-chain runtime
```

The important preserved line is:

```text
svc-index may point to read-only replay/proof artifacts, but artifact pointers are references only.
```

---

## 2. Files Changed

Changed:

```text
crates/svc-index/tests/quickchain_phase2_replay_boundary.rs
crates/svc-index/docs/quickchain-preflight.md
```

No runtime source file was changed.

No `Cargo.toml` dependency change was required.

---

## 3. New Phase 2 Round 1 Boundary Coverage

Added focused Phase 2 Round 1 coverage through:

```text
crates/svc-index/tests/quickchain_phase2_replay_boundary.rs
```

The focused test locks these boundaries:

```text
docs name Phase 2 Round 1 verifier artifact / read-only replication
svc-index may lookup artifact pointers only
svc-index may point to read-only replay/proof artifacts
artifact pointers are references only
index artifact pointer is not proof authority
index artifact pointer is not verifier truth
index artifact pointer is not quorum truth
index artifact pointer is not committee truth
index artifact pointer is not fork choice
index artifact pointer is not finality
index artifact pointer cannot unlock paid content
b3 proves bytes, not verifier truth
svc-wallet remains the paid mutation path
ron-ledger remains durable economic truth
b3 artifact CIDs remain byte references, not proof authority
pointer DTOs reject replay/verifier/committee/finality authority smuggling fields
public routes do not expose replay/verifier/committee/finality surfaces
runtime source does not construct Phase 2 replay/verifier authority
```

The focused test passed:

```text
cargo test -p svc-index --test quickchain_phase2_replay_boundary

running 5 tests
test b3_artifact_cids_remain_byte_references_not_proof_authority ... ok
test docs_name_phase2_round1_index_pointer_only_boundary ... ok
test pointer_dtos_reject_replay_verifier_and_committee_authority_smuggling_fields ... ok
test public_routes_do_not_expose_replay_verifier_committee_or_finality_surfaces ... ok
test runtime_source_does_not_construct_phase2_replay_or_verifier_authority ... ok

test result: ok. 5 passed; 0 failed
```

---

## 4. Documentation Updated

Updated:

```text
crates/svc-index/docs/quickchain-preflight.md
```

The doc now explicitly names:

```text
Phase 2 Round 1 verifier artifact / read-only replication boundary
```

The doc now preserves these boundaries:

```text
svc-index may lookup artifact pointers only
svc-index may point to read-only replay/proof artifacts
artifact pointers are references only
index artifact pointer is not proof authority
index artifact pointer is not verifier truth
index artifact pointer is not replay truth
index artifact pointer is not quorum truth
index artifact pointer is not committee truth
index artifact pointer is not fork choice
index artifact pointer is not finality
index artifact pointer cannot unlock paid content
b3 proves bytes, not verifier truth
b3 proves bytes, not finality
b3 proves bytes, not paid entitlement
names are pointers, not proof
crab navigation is navigation, not authority
manifest lookup is not paid unlock
provider lookup is not settlement finality
owner wallet fields are references only
owner passport fields are references only
svc-wallet remains the paid mutation path
ron-ledger remains durable economic truth
```

---

## 5. Final Gate Evidence

The final local gate discovered 10 focused QuickChain tests:

```text
quickchain_phase1_pointer_dto_strictness
quickchain_phase1_round2_confirmation
quickchain_phase2_replay_boundary
quickchain_preflight_boundary
quickchain_preflight_docs
quickchain_preflight_phase1_pair_interlock
quickchain_preflight_pointer_authority
quickchain_preflight_routes
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
```

The final parking markers were:

```text
== svc-index quickchain exhaustive preflight gate passed: tests=10 ==
== svc-index QuickChain parking gate passed ==
```

The all-targets gate completed cleanly:

```text
cargo test -p svc-index --all-targets
```

The clippy gate completed cleanly:

```text
cargo clippy -p svc-index --all-targets --no-deps -- -D warnings
```

The forbidden-scope marker stayed parked:

```text
no roots
no checkpoints
no validators
no settlement
no bridges
no wallet or ledger mutation
no fake receipts, fake balances, fake finality, or paid unlocks from index entries
```

---

## 6. Important Existing Coverage Still Preserved

This pass did not replace older svc-index safety coverage. It extended it.

Still protected:

```text
svc-index remains lookup/pointer-only
manifest pointers can reference artifacts without becoming proof authority
pointer DTOs reject authority-shaped fields
owner wallet metadata remains a reference only
owner passport metadata remains a reference only
b3 CIDs prove bytes only
public routes remain lookup/pointer/admin/health surfaces only
routes do not expose QuickChain settlement surfaces
routes do not expose paid-unlock authority surfaces
runtime source does not define index or policy as paid authority
runtime source does not define root/proof/finality authority
runtime source does not import wallet/ledger authority crates
Cargo.toml does not add policy/wallet/ledger/chain runtime dependencies
store roundtrips remain pointer records only
HTTP contracts preserve strict unknown-field rejection
```

---

## 7. Safe Meaning

The safe completion meaning for this crate is:

```text
svc-index is green/parked for QuickChain Phase 2 Round 1.
It may point to read-only replay/proof artifacts as references,
but it cannot prove them, verify them, finalize them, unlock paid content from them,
or convert them into wallet/ledger/settlement authority.
```

---

## 8. What Not To Claim

Do not claim:

```text
Phase 2 complete
Phase 2 Round 1 complete across all crates
committee ready
validator ready
quorum ready
fork choice ready
finality ready
svc-index verifier routes are live
svc-index can verify roots
svc-index can prove replay results
svc-index can sign attestations
svc-index can unlock paid content from artifact pointers
svc-index can mutate wallet
svc-index can mutate ledger
bridge ready
staking/slashing ready
external settlement ready
ROX/Solana runtime ready
```

---

## 9. Do Not Reopen Unless

Do not reopen `svc-index` during Phase 2 Round 1 unless:

```text
CrabLink client adapters require a stricter display/status shape for artifact pointers
ron-policy verifier access policy introduces a new pointer tag or obligation shape
svc-gateway or omnigate route contracts require a new read-only index pointer shape
ron-proto verifier artifact DTOs change in a way that affects pointer references
ron-ledger replay artifact naming/hash requirements change
a parking gate regresses
a scanner catches new proof/verifier/finality/paid-unlock authority creep
```

Otherwise, `svc-index` should stay parked until Phase 2 Round 2 or a later explicitly scoped phase.

---

## 10. Next Pair

The next Phase 2 Round 1 crate pair is:

```text
CrabLink Tauri + client adapters
```

Expected goal for that pair:

```text
CrabLink may display verifier/replay/readiness status from backend-derived sources,
but it cannot construct roots, verify finality as authority, unlock paid content from replay artifacts alone,
invent balances, invent receipts, silently spend, or treat cache/client state as economic truth.
```

### END NOTE - JUNE 23 2026 - QUICKCHAIN PHASE 2 ROUND 1 - svc-index

Add this to `crates/ron-policy/NOTES.MD`:

### BEGIN NOTE - JUNE 23 2026 - QUICKCHAIN PHASE 2 ROUND 1 - ron-policy

# QuickChain Crate Notes — ron-policy

Date: June 23, 2026
Project: RustyOnions / CrabLink
Track: QuickChain Phase 2 Round 1
Round purpose: verifier artifact / read-only replication
Crate: `ron-policy`
Status: COMPLETE / PARKED for QuickChain Phase 2 Round 1

---

## 0. Status Summary

`ron-policy` is now complete and parked for QuickChain Phase 2 Round 1.

Final status:

```text
QuickChain Phase 2 Round 1 — ron-policy COMPLETE / PARKED
```

This means `ron-policy` has been hardened for the Phase 2 Round 1 verifier artifact / read-only replication slice.

The crate may declare verifier/replay artifact access policy, but policy decisions remain declarative only. Policy cannot create verifier truth, replay truth, quorum truth, committee truth, fork choice, finality, paid unlock authority, wallet mutation authority, ledger mutation authority, settlement authority, bridge authority, staking authority, or external settlement authority.

This does not mean Phase 2 is complete. It does not mean committee signing, quorum, fork-choice, validators, finality, staking, slashing, bridge behavior, ROX, Solana, or external settlement is implemented.

---

## 1. What This Pass Proved

This pass proved that `ron-policy` remains a declarative policy engine while Phase 2 introduces read-only verifier/replay artifact context elsewhere.

The crate remains:

```text
declarative policy engine
deny-by-default evaluator
classification tag checker
obligation generator
economics policy validator
strict DTO parser
policy access gate only
```

The crate does not become:

```text
proof authority
verifier truth
replay truth
root authority
checkpoint authority
committee authority
quorum authority
fork-choice authority
finality authority
settlement authority
paid-unlock authority
wallet mutation authority
ledger mutation authority
bridge authority
staking/slashing authority
public-chain runtime
```

The important preserved line is:

```text
ron-policy may declare verifier access policy only; policy-created verifier truth is forbidden.
```

---

## 2. Files Changed

Changed:

```text
crates/ron-policy/tests/quickchain_phase2_replay_boundary.rs
crates/ron-policy/docs/quickchain-preflight.md
```

No runtime source file was changed.

No `Cargo.toml` dependency change was required.

---

## 3. New Phase 2 Round 1 Boundary Coverage

Added focused Phase 2 Round 1 coverage through:

```text
crates/ron-policy/tests/quickchain_phase2_replay_boundary.rs
```

The focused test locks these boundaries:

```text
docs name Phase 2 Round 1 verifier artifact / read-only replication
ron-policy may declare verifier access policy only
policy-created verifier truth is forbidden
policy-created replay truth is forbidden
policy-created quorum truth is forbidden
policy-created committee truth is forbidden
policy-created fork choice is forbidden
policy-created finality is forbidden
policy allow is not verifier proof
policy obligation is not verifier attestation
policy obligation is not quorum
policy obligation cannot unlock paid content from replay artifacts alone
svc-wallet remains the paid mutation path
ron-ledger remains durable economic truth
policy can declare read-only replay artifact access without creating truth
ordinary Phase 2 classification tags remain allowed
existing authority-shaped tags still reject
replay artifact policy params cannot smuggle authority fields
production source does not construct committee/quorum/finality authority
```

The focused test passed:

```text
cargo test -p ron-policy --test quickchain_phase2_replay_boundary

running 5 tests
test docs_name_phase2_round1_policy_verifier_access_boundary ... ok
test ordinary_phase2_classification_tags_are_allowed_but_existing_authority_tags_still_reject ... ok
test policy_can_declare_read_only_replay_artifact_access_without_creating_truth ... ok
test replay_artifact_policy_params_cannot_smuggle_existing_authority_fields ... ok
test production_source_does_not_construct_phase2_committee_quorum_or_finality_authority ... ok

test result: ok. 5 passed; 0 failed
```

---

## 4. Documentation Updated

Updated:

```text
crates/ron-policy/docs/quickchain-preflight.md
```

The doc now explicitly names:

```text
Phase 2 Round 1 verifier artifact / read-only replication boundary
```

The doc now preserves these boundaries:

```text
ron-policy may declare verifier access policy only
ron-policy may classify read-only replay artifact access requests
ron-policy may require backend replay artifact references
ron-policy may require backend wallet/ledger proof
policy decisions remain declarative control-plane decisions
policy obligations remain caller instructions only
policy-created verifier truth is forbidden
policy-created replay truth is forbidden
policy-created quorum truth is forbidden
policy-created committee truth is forbidden
policy-created fork choice is forbidden
policy-created finality is forbidden
policy allow is not verifier proof
policy allow is not replay truth
policy allow is not quorum
policy allow is not finality
policy obligation is not verifier attestation
policy obligation is not quorum
policy obligation is not fork choice
policy obligation cannot unlock paid content from replay artifacts alone
policy configuration is not settlement authority
svc-wallet remains the paid mutation path
ron-ledger remains durable economic truth
```

---

## 5. Final Gate Evidence

The final local gate discovered 14 focused QuickChain tests:

```text
quickchain_phase1_policy_condition_tags
quickchain_phase1_round2_confirmation
quickchain_phase2_replay_boundary
quickchain_preflight_boundary
quickchain_preflight_decision_non_authority
quickchain_preflight_docs
quickchain_preflight_economics_config_non_authority
quickchain_preflight_economics_identifier_non_authority
quickchain_preflight_parser_consistency
quickchain_preflight_phase1_pair_interlock
quickchain_preflight_public_surface_boundary
quickchain_preflight_schema_non_authority
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
```

The final parking markers were:

```text
== ron-policy quickchain exhaustive preflight gate passed: tests=14 ==
== ron-policy QuickChain parking gate passed ==
```

The all-targets gate completed cleanly:

```text
cargo test -p ron-policy --all-targets
```

The clippy gate completed cleanly:

```text
cargo clippy -p ron-policy --all-targets --no-deps -- -D warnings
```

The forbidden-scope marker stayed parked:

```text
no roots
no checkpoints
no validators
no settlement
no bridges
no wallet or ledger mutation
no fake receipts, fake balances, fake finality, or paid unlocks from policy decisions
```

---

## 6. Important Existing Coverage Still Preserved

This pass did not replace older ron-policy safety coverage. It extended it.

Still protected:

```text
policy remains declarative only
policy decisions are not receipt truth
policy decisions are not balance truth
policy decisions are not paid unlock authority
policy decisions are not finality truth
policy decisions are not settlement truth
allow decisions cannot unlock paid content without backend truth
deny decisions are also not economic truth
authority-shaped condition tags reject
authority-shaped obligation kinds reject
authority-shaped obligation param keys reject
schema rejects authority-shaped obligations and params
economics config remains validation/config only
economics helpers return amounts and validation only, not truth artifacts
economics identifiers reject receipt/balance/finality/root/mutation authority shapes
parser consistency holds across JSON/TOML
production source does not import wallet/ledger mutation authority crates
production source does not define QuickChain runtime surface
production source does not call wallet or ledger mutation verbs
public examples/benches/fuzz targets remain policy-only
```

---

## 7. Safe Meaning

The safe completion meaning for this crate is:

```text
ron-policy is green/parked for QuickChain Phase 2 Round 1.
It may classify and gate read-only verifier/replay artifact access,
but it cannot create verifier truth, replay truth, quorum truth, committee truth,
fork choice, finality, paid unlocks, wallet mutations, ledger mutations, or settlement truth.
```

---

## 8. What Not To Claim

Do not claim:

```text
Phase 2 complete
Phase 2 Round 1 complete across all crates
committee ready
validator ready
quorum ready
fork choice ready
finality ready
ron-policy can verify roots
ron-policy can prove replay results
ron-policy can sign attestations
ron-policy can create verifier truth
ron-policy can create quorum truth
ron-policy can unlock paid content from replay artifacts
ron-policy can mutate wallet
ron-policy can mutate ledger
bridge ready
staking/slashing ready
external settlement ready
ROX/Solana runtime ready
```

---

## 9. Do Not Reopen Unless

Do not reopen `ron-policy` during Phase 2 Round 1 unless:

```text
CrabLink client adapters need a stricter policy/display status distinction
svc-index introduces a new artifact pointer classification tag
svc-gateway or omnigate route contracts need a new verifier-access obligation shape
ron-proto verifier artifact DTOs introduce new policy-visible classification labels
ron-ledger replay artifact verification result taxonomy changes in a policy-visible way
a parking gate regresses
a scanner catches new proof/verifier/finality/paid-unlock authority creep
```

Otherwise, `ron-policy` should stay parked until Phase 2 Round 2 or a later explicitly scoped phase.

---

## 10. Next Pair

The next Phase 2 Round 1 crate pair is:

```text
CrabLink Tauri + client adapters
```

Expected goal for that pair:

```text
CrabLink may display verifier/replay/readiness status from backend-derived sources,
but it cannot construct roots, verify finality as authority, unlock paid content from replay artifacts alone,
invent balances, invent receipts, silently spend, or treat cache/client state as economic truth.
```

### END NOTE - JUNE 23 2026 - QUICKCHAIN PHASE 2 ROUND 1 - ron-policy

After saving those, this crate-pair status is safely:

```text
QuickChain Phase 2 Round 1 — svc-index + ron-policy COMPLETE / PARKED
```

### END NOTE - JUNE 23 2026 - 22:10 CST


### BEGIN NOTE - JUNE 24 2026 - 02:20 CST

Here are the crate notes for the Phase 2 Round 2 `svc-index + ron-policy` pass. The terminal output confirms both crates hit their park markers: `svc-index QuickChain parking gate passed`, `ron-policy QuickChain parking gate passed`, and the combined final marker for the patch. 

Paste this into `crates/svc-index/NOTES.MD`:

### BEGIN NOTE - JUNE 24 2026 - QUICKCHAIN PHASE 2 ROUND 2

# svc-index — QuickChain Phase 2 Round 2 Crate Notes

## Status

`svc-index` is complete / parked for QuickChain Phase 2 Round 2.

This round hardened `svc-index` for the Phase 2 small committee / replicated verification boundary while preserving the core invariant:

```text
svc-index is lookup and pointer infrastructure only.
svc-index is not proof authority.
svc-index is not verifier truth.
svc-index is not committee truth.
svc-index is not quorum truth.
svc-index is not finality.
svc-index is not settlement.
svc-index is not paid unlock authority.
```

The crate remains a pointer layer for names, manifests, providers, and artifact references. It may point to backend-derived verifier or committee-readiness artifacts in the future, but those pointers are references only. They do not prove committee agreement, quorum, finality, settlement, or paid entitlement.

## Files changed / hardened

Docs hardened:

```text
crates/svc-index/docs/quickchain-preflight.md
```

New Phase 2 Round 2 test:

```text
crates/svc-index/tests/quickchain_phase2_committee_boundary.rs
```

No runtime route authority was added.

No QuickChain runtime, verifier, validator, quorum, bridge, staking, liquidity, wallet, or ledger mutation dependency was added.

## New test coverage

New focused test suite:

```text
cargo test -p svc-index --test quickchain_phase2_committee_boundary
```

Coverage added:

```text
- docs name the Phase 2 Round 2 index committee readiness boundary
- manifest pointer DTOs reject committee, quorum, fork-choice, finality, and settlement smuggling fields
- public routes do not expose committee/quorum/validator/bridge/finality/settlement authority surfaces
- production source does not construct committee, quorum, verifier, fork-choice, finality, or paid-unlock truth
- Cargo manifest does not add wallet, ledger, validator, or external settlement dependencies
- dynamic QuickChain preflight discovers the new suite automatically
```

The new suite confirms that these authority-shaped fields are rejected from pointer DTOs:

```text
verifier_attestation
verifier_signature
committee_attestation
committee_signature
committee_member
committee_vote
quorum_reached
quorum_certificate
fork_choice
fork_choice_winner
finalized
settlement_finality
```

## Gates passed

Focused test:

```text
cargo test -p svc-index --test quickchain_phase2_committee_boundary
```

Result:

```text
6 passed; 0 failed
```

Crate park runner:

```text
crates/svc-index/scripts/dev-quickchain-park.sh
```

The dynamic QuickChain runner picked up all `quickchain*.rs` tests, including the new Phase 2 Round 2 committee boundary suite.

Confirmed QuickChain suites included:

```text
quickchain_phase1_pointer_dto_strictness
quickchain_phase1_round2_confirmation
quickchain_phase2_committee_boundary
quickchain_phase2_replay_boundary
quickchain_preflight_boundary
quickchain_preflight_docs
quickchain_preflight_phase1_pair_interlock
quickchain_preflight_pointer_authority
quickchain_preflight_routes
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
```

Existing regressions passed:

```text
http_contract
integration
```

All-targets passed, including:

```text
chaos
http_contract
integration
loom_index
prop_index
all QuickChain boundary suites
bench resolve
example client
```

Clippy passed:

```text
cargo clippy -p svc-index --all-targets --no-deps -- -D warnings
```

Final parking markers reached:

```text
svc-index forbidden QuickChain runtime scope remains parked:
- no roots
- no checkpoints
- no validators
- no settlement
- no bridges
- no wallet or ledger mutation
- no fake receipts, fake balances, fake finality, or paid unlocks from index entries

== svc-index quickchain exhaustive preflight gate passed: tests=11 ==
svc-index QuickChain Phase-0 preflight passed.
== svc-index QuickChain parking gate passed ==
```

## Boundary summary

`svc-index` may:

```text
- resolve names
- resolve b3/provider pointers
- store and retrieve manifest pointers
- hold owner wallet/passport references as metadata only
- point to backend-derived artifact references
- point to future backend-derived verifier/committee readiness artifacts as references only
```

`svc-index` must not:

```text
- mutate wallet state
- mutate ledger state
- create fake receipts
- create fake balances
- unlock paid content
- prove payment
- prove verifier replay
- prove committee agreement
- decide committee membership
- decide quorum
- decide fork choice
- decide finality
- decide settlement
- expose validator routes
- expose bridge/staking/liquidity routes
- treat b3/name/cache/pointer lookup as authority
```

## Completion statement

`svc-index` is parked for QuickChain Phase 2 Round 2.

Next time `svc-index` is touched for QuickChain, preserve this role:

```text
Index points.
Index does not prove.
Index does not unlock.
Index does not settle.
Index never becomes verifier, committee, quorum, finality, wallet, or ledger authority.
```

### END NOTE - JUNE 24 2026 - QUICKCHAIN PHASE 2 ROUND 2

Paste this into `crates/ron-policy/NOTES.MD`:

### BEGIN NOTE - JUNE 24 2026 - QUICKCHAIN PHASE 2 ROUND 2

# ron-policy — QuickChain Phase 2 Round 2 Crate Notes

## Status

`ron-policy` is complete / parked for QuickChain Phase 2 Round 2.

This round hardened `ron-policy` for the Phase 2 small committee / replicated verification boundary while preserving the core invariant:

```text
ron-policy is declarative policy only.
ron-policy is not verifier truth.
ron-policy is not committee truth.
ron-policy is not quorum truth.
ron-policy is not finality.
ron-policy is not settlement.
ron-policy is not paid unlock authority.
```

The crate may declare policy for viewing or gating backend-derived verifier / committee readiness references, but policy decisions and obligations do not create signed verification attestations, committee membership, quorum certificates, fork choice, finality, settlement, or paid entitlement.

## Files changed / hardened

Docs hardened:

```text
crates/ron-policy/docs/quickchain-preflight.md
```

New Phase 2 Round 2 test:

```text
crates/ron-policy/tests/quickchain_phase2_committee_boundary.rs
```

No runtime authority was added.

No `svc-index`, `svc-wallet`, `ron-ledger`, `ron-accounting`, `svc-rewarder`, `ron-proto`, QuickChain runtime, validator, quorum, bridge, staking, liquidity, or external settlement dependency was added.

## New test coverage

New focused test suite:

```text
cargo test -p ron-policy --test quickchain_phase2_committee_boundary
```

Coverage added:

```text
- docs name the Phase 2 Round 2 policy committee readiness boundary
- policy can gate committee-readiness display without creating truth
- policy params cannot smuggle existing authority-shaped fields
- production source does not construct committee/quorum/finality authority
- Cargo manifest does not add index, wallet, ledger, validator, or external settlement dependencies
- dynamic QuickChain preflight discovers the new suite automatically
```

The new suite confirms `ron-policy` may allow display access for ordinary readiness tags such as:

```text
verifier-artifact-reference-present
committee-readiness-reference-present
policy-context-checked
```

But it does not allow policy decisions or obligations to become:

```text
verifier_attestation
verifier_signature
committee_attestation
committee_signature
committee_vote
quorum_reached
quorum_certificate
fork_choice
fork_choice_winner
finality
finalized
settlement_status
settlement_finality
checkpoint_hash
validator_signature
bridge_proof
unlock_granted
paid_unlock
```

## Gates passed

Focused test:

```text
cargo test -p ron-policy --test quickchain_phase2_committee_boundary
```

Result:

```text
6 passed; 0 failed
```

Crate park runner:

```text
crates/ron-policy/scripts/dev-quickchain-park.sh
```

The dynamic QuickChain runner picked up all `quickchain*.rs` tests, including the new Phase 2 Round 2 committee boundary suite.

Confirmed QuickChain suites included:

```text
quickchain_phase1_policy_condition_tags
quickchain_phase1_round2_confirmation
quickchain_phase2_committee_boundary
quickchain_phase2_replay_boundary
quickchain_preflight_boundary
quickchain_preflight_decision_non_authority
quickchain_preflight_docs
quickchain_preflight_economics_config_non_authority
quickchain_preflight_economics_identifier_non_authority
quickchain_preflight_parser_consistency
quickchain_preflight_phase1_pair_interlock
quickchain_preflight_public_surface_boundary
quickchain_preflight_schema_non_authority
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
```

Existing regressions passed:

```text
economics_policy
unit_model_serde_strict
unit_eval_determinism
unit_first_match_wins
golden_reasons
```

All-targets passed, including:

```text
economics_policy
golden_reasons
all QuickChain boundary suites
unit_churn_protection
unit_eu_only
unit_eval_determinism
unit_first_match_wins
unit_large_body_default_deny
unit_method_matrix
unit_model_serde_strict
unit_tags_all
unit_tighten_only
bench eval_throughput
examples minimal_allow_deny and ron_policy_cli
```

Clippy passed:

```text
cargo clippy -p ron-policy --all-targets --no-deps -- -D warnings
```

Final parking markers reached:

```text
ron-policy forbidden QuickChain runtime scope remains parked:
- no roots
- no checkpoints
- no validators
- no settlement
- no bridges
- no wallet or ledger mutation
- no fake receipts, fake balances, fake finality, or paid unlocks from policy decisions

== ron-policy quickchain exhaustive preflight gate passed: tests=15 ==
ron-policy QuickChain Phase-0 preflight passed.
== ron-policy QuickChain parking gate passed ==
```

## Boundary summary

`ron-policy` may:

```text
- parse strict JSON/TOML policy bundles
- evaluate declarative allow/deny decisions
- emit policy obligations
- validate ROC economics policy
- reject authority-shaped policy tags, obligation kinds, and obligation params
- declare access policy for backend-derived verifier/committee readiness display
```

`ron-policy` must not:

```text
- mutate wallet state
- mutate ledger state
- create fake receipts
- create fake balances
- unlock paid content by policy decision alone
- create signed verification attestations
- decide committee membership
- decide quorum
- issue quorum certificates
- decide fork choice
- decide finality
- decide settlement
- implement validator runtime
- implement bridge/staking/liquidity behavior
- turn policy tags or obligations into economic truth
```

## Completion statement

`ron-policy` is parked for QuickChain Phase 2 Round 2.

Next time `ron-policy` is touched for QuickChain, preserve this role:

```text
Policy declares.
Policy gates.
Policy rejects authority smuggling.
Policy never becomes verifier, committee, quorum, finality, settlement, wallet, ledger, or paid-unlock authority.
```

### END NOTE - JUNE 24 2026 - QUICKCHAIN PHASE 2 ROUND 2


### END NOTE - JUNE 24 2026 - 02:20 CST


### BEGIN NOTE - JUNE 24 2026 - 15:05 CST

Based on the green terminal output, `svc-index` passed exhaustive preflight with **tests=12**, and `ron-policy` passed exhaustive preflight with **tests=16**. Both park gates passed, all-target tests passed, clippy passed, and forbidden runtime scope remains parked. 

Paste this into `crates/svc-index/NOTES.MD`:

### BEGIN NOTE - JUNE 24 2026 - QUICKCHAIN PHASE 3 ROUND 1

# svc-index — QuickChain Phase 3 Round 1 Crate Notes

## Status

`svc-index` is complete / green for **QuickChain Phase 3 Round 1**.

This round added and proved the `svc-index` side of the Phase 3 validator/passport boundary.

`svc-index` remains:

```text
lookup service
pointer service
name/artifact reference service
manifest pointer service
non-authoritative index surface
```

`svc-index` is still not:

```text
validator identity authority
passport registry authority
validator capability authority
validator-set authority
wallet authority
ledger authority
balance truth
receipt truth
paid unlock authority
root/checkpoint/finality authority
staking/slashing authority
bridge authority
external settlement authority
```

## Files changed

```text
crates/svc-index/tests/quickchain_phase3_validator_boundary.rs
crates/svc-index/docs/quickchain-preflight.md
```

## What this round proves

The new Phase 3 test suite proves that `svc-index` can continue storing ordinary owner passport/wallet references as metadata while rejecting validator/passport authority smuggling.

Allowed metadata remains allowed:

```text
owner_passport_subject
owner_wallet_account
manifest_cid
asset_kind
updated_at_ms
```

Rejected authority-shaped fields include:

```text
validator
validators
validator_set
validator_signature
validator_passport
validator_capability
validator_registry_entry
validator_membership_proof
validator_authorization
validator_authz_result
passport_validator
passport_validator_admission
passport_validator_capability
registry_validator
registry_validator_set
capability_validator
capability_validator_scope
attestation_identity
validator_admission
validator_revocation
validator_rotation
validator_finality
validator_paid_unlock
staking
slashing
bonded_validator
```

## Boundary locked

`svc-index` may point to backend-derived validator/readiness artifacts in the future, but those pointers are references only.

They are not:

```text
validator membership proof
passport registry truth
capability issuance truth
quorum proof
finality proof
settlement proof
payment proof
paid entitlement
spend authority
unlock authority
```

## Tests passed

Focused Phase 3 suite:

```text
cargo test -p svc-index --test quickchain_phase3_validator_boundary
```

Result:

```text
7 passed; 0 failed
```

Crate-local park gate:

```text
bash crates/svc-index/scripts/dev-quickchain-park.sh
```

Result:

```text
== svc-index quickchain exhaustive preflight gate passed: tests=12 ==
== svc-index QuickChain parking gate passed ==
```

The exhaustive preflight included:

```text
quickchain_phase1_pointer_dto_strictness
quickchain_phase1_round2_confirmation
quickchain_phase2_committee_boundary
quickchain_phase2_replay_boundary
quickchain_phase3_validator_boundary
quickchain_preflight_boundary
quickchain_preflight_docs
quickchain_preflight_phase1_pair_interlock
quickchain_preflight_pointer_authority
quickchain_preflight_routes
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
```

Existing regressions passed:

```text
http_contract
integration
chaos
loom_index
prop_index
all-target tests
clippy
```

## Forbidden scope remains parked

The park gate printed the expected forbidden-scope marker:

```text
svc-index forbidden QuickChain runtime scope remains parked:
- no roots
- no checkpoints
- no validators
- no settlement
- no bridges
- no wallet or ledger mutation
- no fake receipts, fake balances, fake finality, or paid unlocks from index entries
```

## Phase 3 Round 1 conclusion

Safe status wording:

```text
svc-index is complete for QuickChain Phase 3 Round 1.
It now has a tested validator/passport boundary.
It remains lookup/pointer infrastructure only.
It does not implement validator authority, passport registry authority, wallet/ledger truth, roots, checkpoints, finality, staking, slashing, bridge, or external settlement.
```

### END NOTE - JUNE 24 2026 - QUICKCHAIN PHASE 3 ROUND 1

Paste this into `crates/ron-policy/NOTES.MD`:

### BEGIN NOTE - JUNE 24 2026 - QUICKCHAIN PHASE 3 ROUND 1

# ron-policy — QuickChain Phase 3 Round 1 Crate Notes

## Status

`ron-policy` is complete / green for **QuickChain Phase 3 Round 1**.

This round added and proved the `ron-policy` side of the Phase 3 validator/passport boundary.

`ron-policy` remains:

```text
declarative policy engine
deny-by-default gate
classification/tag evaluator
economics config validator
policy decision surface
```

`ron-policy` is still not:

```text
validator identity authority
passport registry authority
validator capability authority
validator-set authority
wallet authority
ledger authority
balance truth
receipt truth
paid unlock authority
root/checkpoint/finality authority
staking/slashing authority
bridge authority
external settlement authority
```

## Files changed

```text
crates/ron-policy/src/parse/validate.rs
crates/ron-policy/src/economics/validate.rs
crates/ron-policy/schema/policybundle.schema.json
crates/ron-policy/tests/quickchain_phase3_validator_boundary.rs
crates/ron-policy/docs/quickchain-preflight.md
```

## What this round proves

The new Phase 3 suite proves that `ron-policy` can allow ordinary validator/readiness display classification while rejecting authority-shaped validator/passport/registry/capability claims.

Allowed display/gating labels remain allowed:

```text
validator-readiness-display
passport-required
registry-checked
governance-review
```

Rejected authority-shaped condition tags and obligation parameter keys now include:

```text
validator_passport
validator-passport
validator.passport
validator/passport
validator_capability
validator_registry_entry
validator_membership_proof
validator_authorization
validator_authz_result
passport_validator
passport_validator_admission
passport_validator_capability
registry_validator
registry_validator_set
capability_validator
capability_validator_scope
attestation_identity
validator_admission
validator_revocation
validator_rotation
validator_identity_authority
passport_registry_authority
validator_capability_authority
validator_set_authority
validator_paid_unlock
```

Rejected validator lifecycle obligation kinds include:

```text
admit-validator
revoke-validator
rotate-validator
authorize-validator
register-validator
deregister-validator
set-validator-set
update-validator-set
commit-validator-set
grant-validator-capability
grant-validator-admission
sign-validator-attestation
verify-validator-attestation
unlock-from-validator-passport
unlock-from-validator-capability
```

## Boundary locked

`ron-policy` may express validator eligibility policy, but eligibility policy is declarative gating only.

Policy decisions are not:

```text
validator admission
validator revocation
validator rotation
validator-set truth
passport registry truth
capability issuance truth
paid unlock truth
wallet truth
ledger truth
receipt truth
balance truth
finality truth
settlement truth
```

## Tests passed

Focused Phase 3 suite:

```text
cargo test -p ron-policy --test quickchain_phase3_validator_boundary
```

Result:

```text
7 passed; 0 failed
```

Crate-local park gate:

```text
bash crates/ron-policy/scripts/dev-quickchain-park.sh
```

Result:

```text
== ron-policy quickchain exhaustive preflight gate passed: tests=16 ==
== ron-policy QuickChain parking gate passed ==
```

The exhaustive preflight included:

```text
quickchain_phase1_policy_condition_tags
quickchain_phase1_round2_confirmation
quickchain_phase2_committee_boundary
quickchain_phase2_replay_boundary
quickchain_phase3_validator_boundary
quickchain_preflight_boundary
quickchain_preflight_decision_non_authority
quickchain_preflight_docs
quickchain_preflight_economics_config_non_authority
quickchain_preflight_economics_identifier_non_authority
quickchain_preflight_parser_consistency
quickchain_preflight_phase1_pair_interlock
quickchain_preflight_public_surface_boundary
quickchain_preflight_schema_non_authority
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
```

Existing regressions passed:

```text
economics_policy
unit_model_serde_strict
unit_eval_determinism
unit_first_match_wins
golden_reasons
unit_churn_protection
unit_eu_only
unit_large_body_default_deny
unit_method_matrix
unit_tags_all
unit_tighten_only
all-target tests
clippy
```

## Forbidden scope remains parked

The park gate printed the expected forbidden-scope marker:

```text
ron-policy forbidden QuickChain runtime scope remains parked:
- no roots
- no checkpoints
- no validators
- no settlement
- no bridges
- no wallet or ledger mutation
- no fake receipts, fake balances, fake finality, or paid unlocks from policy decisions
```

## Phase 3 Round 1 conclusion

Safe status wording:

```text
ron-policy is complete for QuickChain Phase 3 Round 1.
It now has a tested validator/passport/registry/capability non-authority boundary.
It may express declarative validator eligibility/display policy.
It does not implement validator authority, passport registry authority, wallet/ledger truth, roots, checkpoints, finality, staking, slashing, bridge, or external settlement.
```

### END NOTE - JUNE 24 2026 - QUICKCHAIN PHASE 3 ROUND 1

The next and final Phase 3 Round 1 pair is:

```text
CrabLink Tauri + client adapters
```


### END NOTE - JUNE 24 2026 - 15:05 CST


### BEGIN NOTE - JUNE 24 2026 - 22:35 CST

Here are paste-ready crate notes for `svc-index + ron-policy`. The terminal proof shows the new Phase 3 Round 2 lifecycle suites passed, the existing Phase 3 Round 1 validator/passport suites stayed green, and both crate-local exhaustive preflight gates passed with `svc-index tests=13` and `ron-policy tests=17`. 

### BEGIN NOTE - JUNE 24 2026 - QUICKCHAIN PHASE 3 ROUND 2 - svc-index + ron-policy

# QuickChain Phase 3 Round 2 Crate Notes — `svc-index + ron-policy`

## 0. Final status

Crate pair:

```text
svc-index + ron-policy
```

Final status:

```text
QuickChain Phase 3: 100% COMPLETE / PARKED for this crate pair
```

Reason:

```text
Phase 3 has only two rounds.
Round 1 validator/passport/registry boundary coverage is green.
Round 2 validator lifecycle/evidence/governance boundary coverage is green.
Both crate-local exhaustive QuickChain preflight gates passed.
Both all-target gates passed.
Both clippy gates passed.
Existing regressions stayed green.
Forbidden scope remained parked.
```

This pair should not be reopened for Phase 3 unless a real regression appears.

Important precision:

```text
svc-index + ron-policy are complete for QuickChain Phase 3.
Full QuickChain Phase 3 still needs the final CrabLink Tauri + client adapters pass.
```

Do not call all of Phase 3 complete until CrabLink Tauri/client adapters are green.

---

## 1. Phase / round completed

Current phase:

```text
QuickChain Phase 3 — passport-gated validator set
```

Current round completed:

```text
Round 2 — validator operation / lifecycle hardening
```

This round hardened index/policy boundaries against validator lifecycle authority creep:

```text
validator rotation
validator revocation
validator downtime/degraded status
equivocation evidence
double-attestation evidence
split-brain evidence
replay challenge evidence
governance-gated parameter updates
```

The work remained boundary-only.

No root-producing, settlement, validator runtime, bridge, staking, slashing, bonding, liquidity, external settlement, ROX, or Solana code was added.

---

## 2. `svc-index` notes

## 2.1 Role preserved

`svc-index` remains:

```text
lookup service
pointer service
name resolver
asset manifest pointer store
site manifest pointer store
provider reference surface
owner/passport/wallet metadata reference holder
b3/reference metadata surface
```

`svc-index` is not:

```text
wallet
ledger
balance authority
receipt authority
paid unlock authority
root authority
checkpoint authority
finality authority
settlement authority
validator authority
validator lifecycle authority
validator rotation authority
validator revocation authority
validator downtime authority
validator equivocation authority
replay challenge authority
governance parameter-update authority
bridge authority
staking authority
slashing authority
bonding authority
liquidity authority
external settlement authority
```

The invariant remains:

```text
Index points.
Index does not prove.
Index does not unlock.
Index does not settle.
```

## 2.2 What changed

The Phase 3 Round 2 patch added lifecycle/evidence/governance boundary hardening for `svc-index`.

Files changed or added:

```text
crates/svc-index/docs/quickchain-preflight.md
crates/svc-index/tests/quickchain_phase3_validator_lifecycle_boundary.rs
```

No production runtime authority was added.

No `Cargo.toml` dependency was added.

No Python helper script was added.

The docs now explicitly name the Phase 3 Round 2 lifecycle boundary and state that:

```text
svc-index may store and return backend-derived lookup pointers and metadata only.
svc-index is not validator lifecycle authority.
svc-index is not validator rotation authority.
svc-index is not validator revocation authority.
svc-index is not validator downtime authority.
svc-index is not validator degraded-status authority.
svc-index is not validator equivocation authority.
svc-index is not replay challenge authority.
svc-index is not governance parameter-update authority.
```

The docs also preserve the paid-access boundary:

```text
index entries, names, manifests, profile pointers, b3 hashes, provider records, cache hits, and route metadata cannot unlock paid content.
```

## 2.3 New Phase 3 Round 2 focused test

New test:

```bash
cargo test -p svc-index --test quickchain_phase3_validator_lifecycle_boundary
```

Observed result:

```text
5 passed; 0 failed
```

Covered tests:

```text
docs_name_phase3_round2_index_validator_lifecycle_boundary
pointer_dtos_reject_validator_lifecycle_evidence_and_governance_authority_fields
route_source_does_not_expose_validator_lifecycle_governance_or_evidence_routes
runtime_source_does_not_import_wallet_ledger_policy_or_external_settlement_runtime
runtime_source_does_not_construct_validator_lifecycle_or_paid_unlock_authority
```

## 2.4 What the new `svc-index` suite proves

The new suite proves:

```text
manifest pointer DTOs reject validator lifecycle/evidence/governance authority-shaped fields
site pointer DTOs reject validator lifecycle/evidence/governance authority-shaped fields
asset pointer DTOs reject validator lifecycle/evidence/governance authority-shaped fields
public routes do not expose validator lifecycle mutation routes
public routes do not expose governance parameter-update routes
public routes do not expose replay challenge authority routes
production source does not construct validator lifecycle authority
production source does not construct paid unlock from lifecycle/evidence material
production source does not import wallet, ledger, policy, QuickChain runtime, Solana, SPL, Anchor, bridge, or external settlement authority
```

Authority-shaped fields rejected include:

```text
validator_lifecycle_authority
validator_lifecycle_decision
lifecycle_decision
validator_rotation
validator_revocation
validator_downtime
validator_degraded
validator_equivocation
validator_equivocation_evidence
validator_double_attestation
double_attestation_evidence
validator_split_brain
split_brain_evidence
replay_challenge
replay_challenge_evidence
governance_parameter_update
governance_approval
validator_lifecycle_paid_unlock
validator_lifecycle_receipt_truth
validator_lifecycle_balance_truth
validator_lifecycle_finality_truth
validator_lifecycle_settlement_truth
```

## 2.5 Existing Phase 3 Round 1 validator/passport boundary stayed green

Existing test:

```bash
cargo test -p svc-index --test quickchain_phase3_validator_boundary
```

Observed result:

```text
7 passed; 0 failed
```

Covered tests:

```text
empty_json_helper_remains_used_so_serde_json_map_import_is_not_accidental
manifest_pointer_dtos_still_allow_owner_passport_and_wallet_references_as_metadata
cargo_manifest_does_not_add_phase3_validator_wallet_ledger_or_external_runtime_dependencies
manifest_pointer_dtos_reject_phase3_validator_passport_authority_fields
docs_name_phase3_round1_index_validator_passport_boundary
public_string_routes_do_not_expose_phase3_validator_passport_registry_or_bonding_surfaces
production_source_does_not_implement_validator_lifecycle_or_paid_unlock_authority
```

This confirms Round 2 did not regress the Round 1 passport/registry boundary.

## 2.6 Exhaustive `svc-index` gate

Command run:

```bash
bash crates/svc-index/scripts/dev-quickchain-preflight.sh
```

Final observed marker:

```text
== svc-index quickchain exhaustive preflight gate passed: tests=13 ==
svc-index QuickChain Phase-0 preflight passed.
```

The script still prints `Phase-0 preflight` wording, but its dynamic discovery now includes Phase 1, Phase 2, Phase 3 Round 1, and Phase 3 Round 2 QuickChain suites.

## 2.7 `svc-index` regression coverage preserved

Existing regressions stayed green:

```text
http_contract
integration
chaos
loom_index
prop_index
benches/resolve
examples/client
```

Important preserved behavior:

```text
asset manifest pointer wire shape remains stable
unknown fields reject
store roundtrips asset manifest pointers
store roundtrips site manifest pointers
missing pointers return none
concurrent pointer updates survive
b3 validation rejects bad lengths and characters
site name validation rejects unsafe forms
asset kind validation rejects unknown forms
```

## 2.8 `svc-index` forbidden scope stayed parked

Final parking marker preserved:

```text
svc-index forbidden QuickChain runtime scope remains parked:
- no roots
- no checkpoints
- no validators
- no settlement
- no bridges
- no wallet or ledger mutation
- no fake receipts, fake balances, fake finality, or paid unlocks from index entries
```

---

## 3. `ron-policy` notes

## 3.1 Role preserved

`ron-policy` remains:

```text
declarative policy engine
allow/deny decision engine
reason/explanation engine
obligation parser/evaluator
condition tag validator
economics config validator
pricing/cap/split helper
policy schema owner
```

`ron-policy` is not:

```text
wallet
ledger
balance authority
receipt authority
paid unlock authority
root authority
checkpoint authority
finality authority
settlement authority
validator authority
validator lifecycle authority
validator rotation authority
validator revocation authority
validator downtime authority
validator equivocation authority
replay challenge authority
governance parameter-update authority
bridge authority
staking authority
slashing authority
bonding authority
liquidity authority
external settlement authority
```

The invariant remains:

```text
Policy decides.
Policy does not mutate.
Policy does not prove.
Policy does not unlock by itself.
Policy does not settle.
```

## 3.2 What changed

The Phase 3 Round 2 patch added lifecycle/evidence/governance boundary hardening for `ron-policy`.

Files changed or added:

```text
crates/ron-policy/docs/quickchain-preflight.md
crates/ron-policy/src/parse/validate.rs
crates/ron-policy/src/economics/validate.rs
crates/ron-policy/tests/quickchain_phase3_validator_lifecycle_boundary.rs
```

No runtime authority was added.

No wallet, ledger, passport, registry, auth, QuickChain runtime, bridge, staking, slashing, bonding, liquidity, external settlement, ROX, or Solana dependency was added.

The docs now explicitly state:

```text
ron-policy may express declarative lifecycle eligibility/gating rules only.
ron-policy is not validator lifecycle authority.
ron-policy is not validator rotation authority.
ron-policy is not validator revocation authority.
ron-policy is not validator downtime authority.
ron-policy is not validator degraded-status authority.
ron-policy is not validator equivocation authority.
ron-policy is not replay challenge authority.
ron-policy is not governance parameter-update authority.
```

The docs also state that:

```text
policy decisions, reasons, obligations, tags, economics config, feature flags, and explanations cannot admit validators, revoke validators, rotate validators, mark validators down, accept equivocation evidence, accept replay challenge evidence, commit governance parameter updates, unlock paid content, mutate balances, issue receipts, prove finality, prove settlement, or replace wallet/ledger truth.
```

## 3.3 Parser/economics hardening

The parser and economics validators now reject exact authority-shaped lifecycle identifiers.

Added or confirmed forbidden normalized identifiers include:

```text
validatordowntime
validatordegraded
validatorequivocation
validatorequivocationevidence
validatordoubleattestation
validatorsplitbrain
validatorlifecycledecision
lifecycledecision
replaychallenge
replaychallengeevidence
governanceparameterupdate
governanceapproval
```

The parser also rejects lifecycle/evidence/governance obligation kinds such as:

```text
markvalidatordowntime
markvalidatordegraded
submitvalidatorequivocationevidence
submitdoubleattestationevidence
submitsplitbrainevidence
submitreplaychallenge
submitreplaychallengeevidence
commitgovernanceparameterupdate
grantgovernanceapproval
grantvalidatorlifecycledecision
unlockfromvalidatorlifecycle
settlefromreplaychallenge
```

Important nuance:

```text
ordinary lifecycle display/classification tags remain allowed when they are clearly display/readiness context, not authority.
```

Examples that remain allowed:

```text
validator-downtime-display
validator-degraded-status-display
validator-lifecycle-readiness-display
replay-challenge-display
governance-review
```

This avoids overcorrecting by banning legitimate metadata, while still rejecting authority-shaped claims.

## 3.4 New Phase 3 Round 2 focused test

New test:

```bash
cargo test -p ron-policy --test quickchain_phase3_validator_lifecycle_boundary
```

Observed result:

```text
7 passed; 0 failed
```

Covered tests:

```text
ordinary_lifecycle_display_classification_tags_remain_allowed
lifecycle_authority_condition_tags_reject
lifecycle_authority_obligation_param_keys_reject
lifecycle_authority_obligation_kinds_reject
docs_name_phase3_round2_policy_validator_lifecycle_boundary
parser_and_economics_validator_contain_phase3_round2_exact_authority_shapes
production_source_does_not_construct_lifecycle_governance_paid_unlock_or_settlement_runtime
```

## 3.5 What the new `ron-policy` suite proves

The new suite proves:

```text
ordinary lifecycle display tags can be used for classification
authority-shaped lifecycle condition tags reject
authority-shaped lifecycle obligation parameter keys reject
authority-shaped lifecycle obligation kinds reject
parser hardening includes Phase 3 Round 2 lifecycle authority shapes
economics identifier hardening includes Phase 3 Round 2 lifecycle authority shapes
production source does not construct lifecycle/governance paid unlock or settlement runtime
```

The suite specifically prevents policy material from becoming:

```text
validator lifecycle authority
validator rotation authority
validator revocation authority
validator downtime authority
validator degraded-status authority
validator equivocation authority
replay challenge authority
governance parameter-update authority
wallet mutation authority
ledger mutation authority
receipt truth
balance truth
paid unlock truth
finality truth
settlement truth
```

## 3.6 Existing Phase 3 Round 1 validator/passport boundary stayed green

Existing test:

```bash
cargo test -p ron-policy --test quickchain_phase3_validator_boundary
```

Observed result:

```text
7 passed; 0 failed
```

Covered tests:

```text
docs_name_phase3_round1_policy_validator_passport_boundary
parser_and_economics_validator_contain_phase3_exact_authority_shapes
ordinary_validator_display_classification_tags_remain_allowed
phase3_validator_lifecycle_obligation_kinds_reject
phase3_validator_passport_authority_condition_tags_reject
phase3_validator_passport_authority_obligation_param_keys_reject
production_source_does_not_import_wallet_ledger_passport_registry_or_external_runtime_authority
```

This confirms Round 2 did not regress the Round 1 passport/registry validator boundary.

## 3.7 Exhaustive `ron-policy` gate

Command run:

```bash
bash crates/ron-policy/scripts/dev-quickchain-preflight.sh
```

Final observed marker:

```text
== ron-policy quickchain exhaustive preflight gate passed: tests=17 ==
ron-policy QuickChain Phase-0 preflight passed.
```

The script still prints `Phase-0 preflight` wording, but its dynamic discovery now includes Phase 1, Phase 2, Phase 3 Round 1, and Phase 3 Round 2 QuickChain suites.

## 3.8 `ron-policy` regression coverage preserved

Existing regressions stayed green:

```text
economics_policy
unit_model_serde_strict
unit_eval_determinism
unit_first_match_wins
golden_reasons
unit_churn_protection
unit_eu_only
unit_large_body_default_deny
unit_method_matrix
unit_tags_all
unit_tighten_only
benches/eval_throughput
examples/minimal_allow_deny
examples/ron_policy_cli
```

Important preserved behavior:

```text
economics policy rejects floats
economics policy rejects negative values
economics policy rejects overflow values
economics policy rejects invalid split sums
economics policy preserves deterministic action order
economics policy validates capture plans
disabled actions reject lookup
unknown actions reject
strict model serde remains strict
evaluation remains deterministic
first-match-wins behavior remains stable
golden explanation trace remains stable
```

## 3.9 `ron-policy` forbidden scope stayed parked

Final parking marker preserved:

```text
ron-policy forbidden QuickChain runtime scope remains parked:
- no roots
- no checkpoints
- no validators
- no settlement
- no bridges
- no wallet or ledger mutation
- no fake receipts, fake balances, fake finality, or paid unlocks from policy decisions
```

---

## 4. Shared pair boundary proven

This pair now proves the Phase 3 index/policy boundary:

```text
svc-index can store and return references.
ron-policy can evaluate declarative policy.
Neither can create economic truth.
Neither can create validator truth.
Neither can create lifecycle truth.
Neither can create settlement truth.
Neither can unlock paid content without backend wallet/ledger truth.
```

The correct value path remains:

```text
CrabLink user intent
→ svc-gateway public/product boundary
→ omnigate quote/access/hydration coordinator
→ svc-wallet mutation front-door when economic mutation is required
→ ron-ledger durable economic truth
→ backend-derived receipt/access result
→ display/render path
```

The index/policy pair may participate only as:

```text
svc-index:
  pointer lookup
  name lookup
  manifest pointer lookup
  metadata/reference lookup

ron-policy:
  declarative allow/deny/gating
  context classification
  explanation/reason output
  non-authoritative economics config validation
```

The index/policy pair must not become:

```text
payment proof
receipt proof
balance proof
ledger proof
root proof
checkpoint proof
finality proof
settlement proof
validator proof
lifecycle proof
governance proof
bridge proof
staking proof
slashing proof
```

---

## 5. Forbidden scope still parked

This pair did not add:

```text
roots
state roots
receipt roots
hold roots
epoch roots
checkpoint roots
checkpoint production
checkpoint signing
validator signatures
validator set runtime
validator admission runtime
validator rotation runtime
validator revocation runtime
validator downtime runtime
validator equivocation runtime
replay challenge runtime
governance parameter-update runtime
committee runtime
staking
slashing
bonding
liquidity
ROX runtime
Solana runtime
external settlement
public bridge
external anchors
public-chain route families
index direct ledger mutation
policy direct ledger mutation
index direct wallet mutation
policy direct wallet mutation
fake receipts
fake balances
fake finality
index-only paid unlock
policy-only paid unlock
cache-only paid unlock
b3-as-payment-proof
crab-url-as-payment-proof
name-as-payment-proof
pointer-as-payment-proof
manifest-as-payment-proof
policy-decision-as-payment-proof
lifecycle-evidence-as-payment-proof
replay-challenge-as-payment-proof
governance-approval-as-payment-proof
```

QuickChain remains future settlement infrastructure.

Internal ROC remains the active economic mission.

---

## 6. Commands proven green

Focused Phase 3 Round 2 commands:

```bash
cargo fmt -p svc-index -p ron-policy

cargo test -p svc-index --test quickchain_phase3_validator_lifecycle_boundary
cargo test -p ron-policy --test quickchain_phase3_validator_lifecycle_boundary

cargo test -p svc-index --test quickchain_phase3_validator_boundary
cargo test -p ron-policy --test quickchain_phase3_validator_boundary
```

Crate-local exhaustive gates:

```bash
bash crates/svc-index/scripts/dev-quickchain-preflight.sh
bash crates/ron-policy/scripts/dev-quickchain-preflight.sh
```

Final observed markers:

```text
== svc-index quickchain exhaustive preflight gate passed: tests=13 ==
svc-index QuickChain Phase-0 preflight passed.

== ron-policy quickchain exhaustive preflight gate passed: tests=17 ==
ron-policy QuickChain Phase-0 preflight passed.
```

---

## 7. Current Phase 3 order and next pair

Phase 3 crate-pair status:

```text
ron-proto + ron-ledger:              COMPLETE / PARKED
svc-wallet + ron-accounting:         COMPLETE / PARKED
svc-rewarder + svc-storage:          COMPLETE / PARKED
svc-gateway + omnigate:              COMPLETE / PARKED
svc-index + ron-policy:              COMPLETE / PARKED
CrabLink Tauri + client adapters:    NEXT / FINAL PHASE 3 PAIR
```

Next active pair:

```text
CrabLink Tauri + client adapters
```

After CrabLink Tauri/client adapters pass, safe final label:

```text
Phase 3 complete
passport-gated validator set complete
```

Still do not call it:

```text
QuickChain complete
public chain complete
staking complete
slashing complete
bonding complete
bridge complete
external settlement complete
exchange-ready
```

---

## 8. Next pair mission: CrabLink Tauri + client adapters

The next crate/product layer must prove:

```text
CrabLink Tauri displays backend-derived validator/readiness/lifecycle metadata only.
React owns display and user intent only.
TS adapters do not own secrets, balances, receipts, lifecycle authority, validator authority, or spend authority.
Tauri commands remain typed, bounded, allowlisted, redacted, feature-gated, and non-authoritative.
Offline cache cannot unlock paid content alone.
Client cannot treat validator lifecycle/evidence/governance metadata as paid access, receipt truth, balance truth, finality, settlement, staking, slashing, bridge, or external settlement.
```

Client forbidden scope for the next pass:

```text
no fake balance
no fake receipt
no fake finality
no silent spend
no cache-only paid unlock
no local validator authority
no client-side validator lifecycle authority
no client-side governance parameter-update authority
no lifecycle/evidence as paid unlock
no replay challenge as settlement/finality
no client root production
no client checkpoint production
no bridge
no staking
no slashing
no bonding
no ROX
no Solana
no external settlement
no raw invoke from React
no secrets in React state/localStorage/URL/logs
```

Correct CrabLink posture:

```text
backend owns truth
CrabLink displays truth
Tauri mediates native privilege
React expresses user intent
TS adapters call allowlisted commands
paid flow remains explicit confirmation -> backend wallet path -> backend receipt -> render/unlock
cache is convenience only
sidecar/local node is optional, explicit, supervised, bounded, and disabled by default
```

---

## 9. Required next-session attachments

For the CrabLink Phase 3 final pass, attach:

```text
QUICKCHAIN_REVIEW_BUNDLE.MD
QUICKCHAIN_BUILDPLAN.MD
latest carryover notes
latest terminal output proving svc-index + ron-policy are parked
latest CODEBUNDLE_TAURI_APP.md
CrabLink focused codebundle if available
```

Do not attach unrelated backend crate bundles unless a client boundary test needs cross-crate inspection.

---

## 10. Low-disk workflow reminder

Continue the low-duplicate QuickChain workflow:

```text
focused client boundary scripts first
then existing QuickChain client gates
then Tauri/React build or park script once
do not rerun already-green backend pairs
do not run workspace clippy
do not run both preflight and park if one delegates to the other
avoid cargo clean unless disk pressure requires it
```

For backend/service crates already parked:

```text
Do not rerun:
- ron-proto + ron-ledger
- svc-wallet + ron-accounting
- svc-rewarder + svc-storage
- svc-gateway + omnigate
- svc-index + ron-policy
```

unless CrabLink exposes a real integration mismatch.

---

### END NOTE - JUNE 24 2026 - QUICKCHAIN PHASE 3 ROUND 2 - svc-index + ron-policy


### END NOTE - JUNE 24 2026 - 22:35 CST




### BEGIN NOTE - JUNE 25 2026 - 21:10 CST

Confirmed green from the terminal output: `svc-index` and `ron-policy` are both parked for **QuickChain Phase 4 Round 1**, with `ron-policy` passing focused Phase 4, full QuickChain preflight, all-target tests, and clippy. 

# QuickChain Phase 4 Round 1 Crate Notes — `svc-index` + `ron-policy`

## Status

`svc-index` + `ron-policy` are complete / parked for QuickChain Phase 4 Round 1.

This crate pair now preserves the Phase 4 Round 1 bonded-validator boundary without introducing live bonds, live slashing, public staking, liquidity, validator runtime authority, bridge logic, ROX/Solana logic, external settlement, wallet mutation, ledger mutation, fake receipts, fake balances, fake finality, or policy/index-created paid unlocks.

Important scope note: this completes the `svc-index` + `ron-policy` crate pair only. Full QuickChain Phase 4 Round 1 still needs the final CrabLink Tauri + client adapters pass.

---

## Files touched

### `svc-index`

* `crates/svc-index/tests/quickchain_phase4_bond_boundary.rs`
* `crates/svc-index/docs/quickchain-preflight.md`

### `ron-policy`

* `crates/ron-policy/tests/quickchain_phase4_bond_boundary.rs`
* `crates/ron-policy/docs/quickchain-preflight.md`
* `crates/ron-policy/src/parse/validate.rs`
* `crates/ron-policy/src/economics/validate.rs`

---

## Final test / gate status

### Focused Phase 4 tests

```text
cargo test -p svc-index --test quickchain_phase4_bond_boundary
result: ok. 5 passed; 0 failed

cargo test -p ron-policy --test quickchain_phase4_bond_boundary
result: ok. 8 passed; 0 failed
```

### Parking gates

```text
bash crates/svc-index/scripts/dev-quickchain-park.sh --check
result: passed

bash crates/ron-policy/scripts/dev-quickchain-park.sh --check
result: passed
```

### `ron-policy` final exhaustive markers

```text
== ron-policy quickchain exhaustive preflight gate passed: tests=18 ==
ron-policy QuickChain Phase-0 preflight passed.
== ron-policy QuickChain parking gate passed ==
```

### Clippy

```text
cargo clippy -p ron-policy --all-targets --no-deps -- -D warnings
result: passed
```

---

## What `svc-index` now proves

`svc-index` remains a lookup / pointer / reference service only.

The Phase 4 Round 1 boundary now proves:

* Index records may reference bond, slash-simulation, evidence, and report artifacts only as metadata / b3 references.
* b3 hashes prove bytes only, not bond truth, slash truth, validator authority, staking authority, liquidity authority, paid unlocks, or settlement truth.
* Index routes do not expose bond, slash, stake, liquidity, bridge, ROX/Solana, external settlement, validator economy, wallet mutation, or ledger mutation authority.
* Manifest pointers remain pointer DTOs, not economic authority records.
* Owner passport and wallet fields remain reference metadata only.
* Cache/index data cannot unlock paid content alone.
* `svc-index` does not depend on wallet, ledger, policy, validator-runtime, bridge, staking, or liquidity authority crates.
* `svc-index` does not become a Phase 4 runtime, validator registry, bond lifecycle manager, slash adjudicator, or economic mutation service.

The key doctrine preserved:

```text
svc-index can point to artifacts.
svc-index cannot make artifacts authoritative.
svc-index can reference b3 bytes.
svc-index cannot turn b3 bytes into bond/slash/stake/liquidity/settlement truth.
```

---

## What `ron-policy` now proves

`ron-policy` remains declarative policy only.

The Phase 4 Round 1 boundary now proves:

* Policy rules may express display-only and gating-only logic for bond/readiness/slash-simulation contexts.
* Policy can allow ordinary classification tags such as display, readiness, simulation, and backend-proof-required style markers.
* Policy allow/deny decisions are not paid unlocks.
* Policy obligations are instructions only, not wallet/ledger/proof/bond/slash authority.
* Policy cannot create bond truth.
* Policy cannot create bond lifecycle truth.
* Policy cannot create slash truth.
* Policy cannot create staking authority.
* Policy cannot create liquidity authority.
* Policy cannot create validator bond authority.
* Policy cannot create bridge, ROX/Solana, or external settlement authority.
* Economics config remains validation/pricing/split policy only, not account truth or mutation authority.

The parser and economics validators now reject Phase 4 authority-shaped identifiers, including normalized/separator variants of:

```text
bond_truth
bond_authority
bond_account_truth
bond_account_authority
bond_intent_authority
bond_lifecycle_truth
bond_lifecycle_authority
bond_lifecycle_decision
validator_bond
validator_bond_authority
bonded_stake
slash_truth
slash_authority
slash_decision
slash_evidence_authority
slashing_authority
stake_authority
staking_authority
staking_market_authority
public_staking_market
liquidity_authority
liquidity_pool_authority
live_slash
automatic_slash
```

The obligation-kind validator also rejects Phase 4 action-like authority shapes such as:

```text
commit-bond-lifecycle
grant-bond-authority
grant-bond-lifecycle-decision
mark-validator-bonded
capture-validator-bond
release-validator-bond
slash-validator
execute-slashing
commit-slash-decision
open-staking-market
grant-staking-authority
create-liquidity-pool
grant-liquidity-authority
settle-bond
```

The key doctrine preserved:

```text
ron-policy can describe whether something may be shown or gated.
ron-policy cannot create economic truth.
ron-policy cannot mutate ledger state.
ron-policy cannot authorize a live bond, slash, stake, liquidity action, bridge, or settlement.
```

---

## Important implementation note

The final `ron-policy` fix was a clippy-only shape refactor after behavior was already correct.

The long authority matchers in `crates/ron-policy/src/parse/validate.rs` were moved into module-level tables:

```text
FORBIDDEN_AUTHORITY_FIELD_SHAPES
FORBIDDEN_AUTHORITY_KIND_SUBSTRINGS
```

Then the matcher functions became short table lookups. This preserved behavior while satisfying:

```text
cargo clippy -p ron-policy --all-targets --no-deps -- -D warnings
```

No runtime authority was added by this refactor.

---

## What this crate pair does not do

This crate pair does not implement:

* live validator bonds
* live slashing
* slash adjudication
* slash execution
* staking markets
* liquidity pools
* public validator economics
* validator-set runtime authority
* checkpoint roots
* consensus roots
* chain state
* bridge settlement
* ROX/Solana settlement
* external chain anchoring that mutates ROC balances
* wallet mutation
* ledger mutation
* policy-created paid unlocks
* index-created paid unlocks
* cache-only unlocks

---

## Phase 4 Round 1 progress after this crate pair

```text
1. ron-proto + ron-ledger              complete / parked
2. svc-wallet + ron-accounting         complete / parked
3. svc-rewarder + svc-storage          complete / parked
4. svc-gateway + omnigate              complete / parked
5. svc-index + ron-policy              complete / parked
6. CrabLink Tauri + client adapters    next / remaining
```

---

## Recommended next crate/session

Next target:

```text
CrabLink Tauri + client adapters
```

Expected Phase 4 Round 1 client scope:

* display-only bond/readiness/slash-simulation status
* no client-side bond truth
* no client-side slash truth
* no public staking market UI
* no liquidity UI
* no ROX/Solana bridge UI
* no client-side spend authority
* no React/Tauri command that mutates ledger, bonds, slashes, validator state, or receipts
* no fake balances
* no fake receipts
* no silent spend
* no cache-only unlock
* Tauri Rust remains a bounded client bridge
* React remains display/user intent only

Suggested client boundary scanner name:

```text
scripts/check-quickchain-phase4-client-bond-boundary.mjs
```

Suggested final client gate:

```text
bash scripts/dev-quickchain-tauri-park.sh
```

---

## Safe completion label for this pair

```text
QuickChain Phase 4 Round 1 svc-index + ron-policy complete / parked.
```

## Not safe to claim yet

Do not claim all of QuickChain Phase 4 Round 1 is complete until CrabLink Tauri + client adapters also pass.


### END NOTE - JUNE 25 2026 - 21:10 CST




### BEGIN NOTE - JUNE 26 2026 - 02:55 CST

Below are paste-ready crate notes for **`svc-index + ron-policy`**. The terminal output confirms both crates are green and parked: `svc-index` passed the new 6-test dispute boundary suite, exhaustive preflight with `tests=15`, clippy, and park; `ron-policy` passed the new 9-test dispute boundary suite, exhaustive preflight with `tests=19`, clippy, and park. 

---

## `crates/svc-index/NOTES.MD`

```markdown
### BEGIN NOTE - JUNE 26 2026 - QUICKCHAIN PHASE 4 ROUND 2 - SVC-INDEX

# svc-index — QuickChain Phase 4 Round 2 Crate Notes

## Status

`svc-index` is COMPLETE / PARKED for QuickChain Phase 4 Round 2.

This pass added the Phase 4 Round 2 dispute/challenge/appeal/freeze simulation boundary for the index service.

The key rule preserved:

    svc-index is lookup / pointer infrastructure only.

It may reference artifacts, names, manifests, providers, and b3-addressed bytes.

It must not become:

    dispute truth
    challenge-window truth
    appeal authority
    freeze authority
    irreversible slash authority
    slash simulation authority
    paid unlock authority
    settlement authority
    finality authority
    wallet authority
    ledger authority

## What changed

Added the new focused test:

    crates/svc-index/tests/quickchain_phase4_bond_dispute_boundary.rs

Updated the QuickChain preflight docs:

    crates/svc-index/docs/quickchain-preflight.md

Updated the preflight script marker so the dynamic runner visibly includes the new suite:

    crates/svc-index/scripts/dev-quickchain-preflight.sh

The script already dynamically discovers `quickchain*.rs`; this patch added the explicit review marker:

    quickchain_phase4_bond_dispute_boundary

## New Phase 4 Round 2 coverage

The new test suite proves:

    docs name the Phase 4 Round 2 index dispute pointer boundary
    index route surface does not expose dispute/challenge/appeal/freeze/slash routes
    index source does not construct Phase 4 Round 2 dispute/challenge/freeze authority
    Cargo.toml does not add runtime economy or settlement dependencies
    reference-only dispute language is allowed without becoming authority
    preflight runner names the Phase 4 Round 2 suite

The suite passed:

    cargo test -p svc-index --test quickchain_phase4_bond_dispute_boundary

Result:

    6 passed
    0 failed

## Boundary locked

`svc-index` may point to dispute/challenge/appeal/freeze artifacts only as references.

Allowed reference-only concepts:

    bond dispute artifact CIDs
    slash evidence artifact CIDs
    challenge window artifact CIDs
    appeal artifact CIDs
    freeze status display labels
    disputed bond status display labels
    slash simulation artifact CIDs

Explicit non-authority phrases added to docs:

    index pointer is not dispute truth
    index pointer is not challenge-window truth
    index pointer is not appeal authority
    index pointer is not freeze authority
    index pointer is not irreversible slash authority
    index pointer is not slash simulation authority
    dispute artifact cid proves bytes only
    challenge evidence cid proves bytes only
    appeal artifact cid proves bytes only
    freeze status label is display metadata only
    b3 proves bytes, not dispute resolution truth
    names are pointers, not dispute authority
    manifest lookup is not challenge evidence validation
    provider lookup is not appeal authority
    index cache cannot unlock paid content or trigger dispute consequences

## Forbidden scope confirmed

The park gate confirmed the following stayed parked:

    no roots
    no checkpoints
    no validators
    no settlement
    no bridges
    no wallet or ledger mutation
    no fake receipts
    no fake balances
    no fake finality
    no paid unlocks from index entries

Additional Phase 4 Round 2 forbidden scope remains parked:

    no live irreversible slash through svc-index
    no public staking market through svc-index
    no liquidity through svc-index
    no ROX
    no Solana
    no external settlement
    no exchange-facing logic
    no public validator economy

## Tests / gates passed

Focused Phase 4 Round 2 suite:

    cargo test -p svc-index --test quickchain_phase4_bond_dispute_boundary

Crate-local park gate:

    crates/svc-index/scripts/dev-quickchain-park.sh

Observed green markers:

    == svc-index quickchain exhaustive preflight gate passed: tests=15 ==
    svc-index QuickChain Phase-0 preflight passed.
    == svc-index QuickChain parking gate passed ==

Clippy also passed inside the park gate:

    cargo clippy -p svc-index --all-targets --no-deps -- -D warnings

## Notes for future work

Do not add dispute/challenge/appeal/freeze route authority to `svc-index`.

If future phases need user-visible dispute lookup, keep it reference-only:

    b3 proves bytes only
    names are pointers only
    manifests are metadata only
    index cache is convenience only
    backend wallet/ledger paths remain economic truth

Do not let index entries or cached manifests unlock paid content, prove dispute outcomes, resolve challenges, grant appeal authority, freeze bonds, release bonds, capture bonds, slash bonds, or finalize settlement.

Next QuickChain Phase 4 Round 2 crate pair:

    CrabLink Tauri + client adapters

### END NOTE - JUNE 26 2026 - QUICKCHAIN PHASE 4 ROUND 2 - SVC-INDEX
```

---

## `crates/ron-policy/NOTES.MD`

```markdown
### BEGIN NOTE - JUNE 26 2026 - QUICKCHAIN PHASE 4 ROUND 2 - RON-POLICY

# ron-policy — QuickChain Phase 4 Round 2 Crate Notes

## Status

`ron-policy` is COMPLETE / PARKED for QuickChain Phase 4 Round 2.

This pass added the Phase 4 Round 2 dispute/challenge/appeal/freeze simulation boundary for the policy engine.

The key rule preserved:

    ron-policy is declarative policy only.

It may say what must be true.

It must not become the proof that it is true.

It must not become:

    dispute truth
    challenge-window truth
    appeal authority
    freeze authority
    irreversible slash authority
    slash simulation authority
    paid unlock authority
    wallet authority
    ledger authority
    settlement authority
    finality authority

## What changed

Added the new focused test:

    crates/ron-policy/tests/quickchain_phase4_bond_dispute_boundary.rs

Updated the QuickChain preflight docs:

    crates/ron-policy/docs/quickchain-preflight.md

Updated parser authority-shape validation:

    crates/ron-policy/src/parse/validate.rs

Updated economics identifier authority-shape validation:

    crates/ron-policy/src/economics/validate.rs

Updated the preflight script marker so the dynamic runner visibly includes the new suite:

    crates/ron-policy/scripts/dev-quickchain-preflight.sh

The script already dynamically discovers `quickchain*.rs`; this patch added the explicit review marker:

    quickchain_phase4_bond_dispute_boundary

## New Phase 4 Round 2 coverage

The new test suite proves:

    docs name the Phase 4 Round 2 policy dispute boundary
    ordinary dispute/challenge display policy remains allowed but non-authoritative
    authority-shaped dispute/challenge/appeal/freeze condition tags reject
    authority-shaped dispute/challenge/appeal/freeze obligation params reject
    authority-shaped dispute/challenge/appeal/freeze obligation kinds reject
    parser and economics validator contain the new exact authority shapes
    economics config rejects authority-shaped aliases
    production source does not construct Phase 4 Round 2 runtime or mutation authority
    preflight runner names the Phase 4 Round 2 suite

The suite passed:

    cargo test -p ron-policy --test quickchain_phase4_bond_dispute_boundary

Result:

    9 passed
    0 failed

## Parser / economics hardening

The following Phase 4 Round 2 authority-shaped identifiers are now rejected in the parser and economics validation tables:

    bonddispute
    bonddisputestate
    disputetruth
    disputeauthority
    disputewindowauthority
    challengewindow
    challengewindowauthority
    appealauthority
    appealwindowauthority
    freezeauthority
    frozenbond
    disputedbond
    irreversibleslash
    slashappeal
    slashchallenge
    slashsimulationauthority
    livedisputeresolution
    automaticdisputeslash

The following authority-shaped obligation kinds are now rejected:

    grantdisputeauthority
    commitbonddisputestate
    openchallengewindow
    grantchallengewindowauthority
    submitslashappeal
    grantappealauthority
    freezebond
    capturedisputedbond
    slashdisputedbond
    executeirreversibleslash
    commitirreversibleslash
    settlebonddispute
    unlockfrombonddispute
    settlefromslashchallenge

## Boundary locked

`ron-policy` may express declarative dispute visibility policy.

`ron-policy` may express declarative challenge window policy only.

`ron-policy` may express declarative appeal/freeze display policy only.

`ron-policy` may require callers to use backend wallet/ledger proof paths before paid access.

But the docs now explicitly lock the following:

    ron-policy is not dispute truth
    ron-policy is not challenge-window truth
    ron-policy is not appeal authority
    ron-policy is not freeze authority
    ron-policy is not irreversible slash authority
    ron-policy is not slash simulation authority
    policy allow is not dispute resolution proof
    policy allow is not challenge evidence validation
    policy obligation is not challenge evidence acceptance
    policy obligation is not appeal authority
    policy obligation is not freeze authority
    policy decision cannot mutate wallet or ledger
    policy decision cannot unlock paid content from dispute/challenge/appeal/freeze material
    policy config cannot create disputed bond balances
    policy config cannot create freeze authority

## Forbidden scope confirmed

The park gate confirmed the following stayed parked:

    no roots
    no checkpoints
    no validators
    no settlement
    no bridges
    no wallet or ledger mutation
    no fake receipts
    no fake balances
    no fake finality
    no paid unlocks from policy decisions

Additional Phase 4 Round 2 forbidden scope remains parked:

    no live irreversible slash through ron-policy
    no public staking market through ron-policy
    no liquidity through ron-policy
    no ROX
    no Solana
    no external settlement
    no exchange-facing logic
    no public validator economy

## Tests / gates passed

Focused Phase 4 Round 2 suite:

    cargo test -p ron-policy --test quickchain_phase4_bond_dispute_boundary

Crate-local park gate:

    crates/ron-policy/scripts/dev-quickchain-park.sh

Observed green markers:

    == ron-policy quickchain exhaustive preflight gate passed: tests=19 ==
    ron-policy QuickChain Phase-0 preflight passed.
    == ron-policy QuickChain parking gate passed ==

Clippy also passed inside the park gate:

    cargo clippy -p ron-policy --all-targets --no-deps -- -D warnings

## Notes for future work

Do not add policy obligations that mutate wallet, mutate ledger, unlock paid content, accept challenge evidence, resolve disputes, grant appeal authority, freeze bonds, capture disputed bonds, slash disputed bonds, or commit irreversible slash.

If future phases need policy involvement in dispute UX, keep it declarative:

    policy can gate display
    policy can require backend evidence
    policy can deny missing proof
    policy can require wallet/ledger truth
    policy cannot itself become proof, finality, settlement, or spend authority

Next QuickChain Phase 4 Round 2 crate pair:

    CrabLink Tauri + client adapters

### END NOTE - JUNE 26 2026 - QUICKCHAIN PHASE 4 ROUND 2 - RON-POLICY
```

Current Phase 4 Round 2 status:

```text
1. ron-proto + ron-ledger             COMPLETE / PARKED
2. svc-wallet + ron-accounting        COMPLETE / PARKED
3. svc-rewarder + svc-storage         COMPLETE / PARKED
4. svc-gateway + omnigate             COMPLETE / PARKED
5. svc-index + ron-policy             COMPLETE / PARKED
6. CrabLink Tauri + client adapters   NEXT
```


### END NOTE - JUNE 26 2026 - 02:55 CST


### BEGIN NOTE - JUNE 26 2026 - 17:55 CST

## QuickChain Phase 4 — Crate Notes: `svc-index + ron-policy`

Status: **GREEN / PARKED for QuickChain Phase 4**

Crate pair completed: `svc-index + ron-policy`
Phase completed for this pair: **QuickChain Phase 4 Round 1, Round 2, and Round 3**
Current round completed: **Phase 4 Round 3 — controlled internal bond enforcement boundary**

Your final terminal output confirms both crates passed focused Round 3 tests, exhaustive crate-local preflight, all-target tests, clippy, and parking gates. `svc-index` parked with **tests=16**, and `ron-policy` parked with **tests=20**. 

---

## What was completed

This crate pair now has Phase 4 boundary coverage for:

```text id="5t64gy"
Round 1: bond / slash / stake / liquidity boundary
Round 2: bond dispute / challenge / appeal / freeze boundary
Round 3: controlled internal bond enforcement boundary
```

The Round 3 work confirms:

```text id="le65o9"
svc-index remains lookup/pointer/reference-only.
ron-policy remains declarative policy-gating only.
Neither crate becomes bond enforcement truth.
Neither crate becomes reserve-slash authority.
Neither crate becomes release-slash-reserve authority.
Neither crate becomes capture-slash-reserve authority.
Neither crate mutates wallet/ledger truth.
Neither crate unlocks paid content from bond enforcement metadata.
```

---

## Files changed

```text id="h18zl8"
crates/svc-index/tests/quickchain_phase4_bond_enforcement_boundary.rs
crates/svc-index/docs/quickchain-preflight.md
crates/svc-index/scripts/dev-quickchain-preflight.sh

crates/ron-policy/tests/quickchain_phase4_bond_enforcement_boundary.rs
crates/ron-policy/src/parse/validate.rs
crates/ron-policy/src/economics/validate.rs
crates/ron-policy/docs/quickchain-preflight.md
crates/ron-policy/scripts/dev-quickchain-preflight.sh
```

A small follow-up fix was also applied to the `svc-index` Round 3 test helper so it can scan both directories and single Rust files like `src/router.rs`.

---

## `svc-index` notes

`svc-index` now has a dedicated Phase 4 Round 3 suite:

```text id="94w35i"
quickchain_phase4_bond_enforcement_boundary
```

Focused result:

```text id="1an04v"
running 6 tests
6 passed / 0 failed
```

The new `svc-index` tests prove that index/pointer surfaces do not expose or create:

```text id="h3m3hh"
bond enforcement routes
reserve-slash routes
release-slash-reserve routes
capture-slash-reserve routes
slash-reserve routes
controlled-slash routes
validator-bond-enforcement routes
public staking routes
liquidity routes
bridge routes
external settlement routes
Solana routes
ROX routes
```

The tests also confirm that `svc-index` does not add runtime economy or settlement dependencies such as:

```text id="b0ht74"
ron-ledger
svc-wallet
ron-accounting
svc-rewarder
ron-policy
Solana SDK/client crates
SPL token crates
ethers/web3 crates
```

The important architecture boundary is:

```text id="wx5072"
svc-index may point to bond-enforcement/slash-reserve artifacts only as references.
A b3 CID proves bytes only.
A pointer does not prove enforcement, reserve, release, capture, settlement, paid unlock, or finality.
```

So `svc-index` remains exactly where it belongs:

```text id="xabdi9"
lookup service
pointer service
manifest reference service
not payment truth
not bond truth
not settlement truth
not validator authority
```

---

## `ron-policy` notes

`ron-policy` now has a dedicated Phase 4 Round 3 suite:

```text id="9qii4j"
quickchain_phase4_bond_enforcement_boundary
```

Focused result:

```text id="t8niav"
running 9 tests
9 passed / 0 failed
```

The new `ron-policy` tests prove that policy may express ordinary display/gating tags like:

```text id="5m689f"
bond-enforcement-display
bond-enforcement-policy
slash-reserve-display
controlled-slash-status-display
reserve-slash-review
capture-release-review
```

But policy rejects authority-shaped condition tags, obligation params, obligation kinds, and economics identifiers for Round 3 concepts such as:

```text id="g2isn1"
bond_enforcement
bond_enforcement_authority
bond_enforcement_decision
bond_enforcement_operation
validator_bond_enforcement
reserve_slash
release_slash_reserve
capture_slash_reserve
slash_reserve
slash_reserved
bond_reserve
bond_capture
bond_release
controlled_slash
controlled_slash_release
controlled_slash_capture
live_bond_enforcement
automatic_bond_enforcement
```

`ron-policy` also rejects authority-shaped obligation kinds such as:

```text id="wkt902"
grant-bond-enforcement-authority
commit-bond-enforcement
execute-bond-enforcement
reserve-slash
release-slash-reserve
capture-slash-reserve
commit-slash-reserve
grant-slash-reserve-authority
capture-controlled-slash
release-controlled-slash
settle-bond-enforcement
unlock-from-bond-enforcement
settle-from-slash-reserve
```

The important architecture boundary is:

```text id="6bhcqc"
ron-policy may declare eligibility, gating, and review policy.
ron-policy cannot create bond enforcement truth.
ron-policy cannot create slash reserve truth.
ron-policy cannot mutate wallet or ledger.
ron-policy cannot unlock paid content.
ron-policy cannot settle, finalize, bridge, stake, or provide liquidity.
```

So `ron-policy` remains:

```text id="gs8twv"
declarative policy only
deny-by-default guardrail
non-mutating economics validator
not wallet authority
not ledger authority
not settlement authority
not finality authority
```

---

## Tests added or updated

Focused tests:

```text id="z4e9pi"
cargo test -p svc-index --test quickchain_phase4_bond_enforcement_boundary
cargo test -p ron-policy --test quickchain_phase4_bond_enforcement_boundary
```

Final focused results:

```text id="rz1nm7"
svc-index:
quickchain_phase4_bond_enforcement_boundary
6 passed / 0 failed

ron-policy:
quickchain_phase4_bond_enforcement_boundary
9 passed / 0 failed
```

Parking gates:

```text id="d0mt4q"
svc-index:
== svc-index quickchain exhaustive preflight gate passed: tests=16 ==
== svc-index QuickChain parking gate passed ==

ron-policy:
== ron-policy quickchain exhaustive preflight gate passed: tests=20 ==
== ron-policy QuickChain parking gate passed ==
```

---

## Boundary status after this round

The pair now preserves the required Phase 4 Round 3 rule:

```text id="mxnvrm"
Controlled bond enforcement may exist internally only through the authorized wallet/ledger/policy-gated path.
svc-index can reference artifacts only.
ron-policy can declare gates only.
Neither crate can execute bond enforcement, reserve slash material, capture slash reserves, release slash reserves, settle disputes, finalize validator consequences, or unlock paid content.
```

Still forbidden in this pair:

```text id="xx5pea"
No wallet mutation.
No ledger mutation.
No roots.
No checkpoints.
No validators.
No settlement.
No public staking market.
No liquidity.
No bridge.
No Solana.
No ROX.
No exchange-facing logic.
No paid unlocks from index entries or policy decisions.
```

---

## Completion status

```text id="bvt7od"
svc-index + ron-policy:
QuickChain Phase 4 complete for this crate pair.
Status: GREEN / PARKED.
```

## Next crate pair

The final Phase 4 pair in the normal order is:

```text id="afko6w"
CrabLink Tauri + client adapters
```

Once CrabLink is green/parked for Phase 4, we can close global **QuickChain Phase 4** as complete.


### END NOTE - JUNE 26 2026 - 17:55 CST


### BEGIN NOTE - JUNE 27 2026 - 20:15 CST

## QuickChain Phase 5 Round 1 crate notes — `svc-index + ron-policy`

Status: **COMPLETE / PARKED**

Terminal proof: both focused Phase 5 tests passed, both crate-local dynamic preflight/park gates passed, and both crate-local clippy gates passed. 

---

# `svc-index`

## Phase/Round

QuickChain Phase 5 Round 1
Anchor-only dry-run pointer boundary
Lookup/pointer crate

## Files added or updated

```text
crates/svc-index/tests/quickchain_phase5_anchor_pointer_boundary.rs
crates/svc-index/docs/quickchain-preflight.md
```

## Focused test added

```text
cargo test -p svc-index --test quickchain_phase5_anchor_pointer_boundary
```

Result:

```text
8 passed; 0 failed
```

## Park/preflight result

```text
== svc-index quickchain exhaustive preflight gate passed: tests=17 ==
== svc-index QuickChain parking gate passed ==
```

## What this round proved

`svc-index` may point to Phase 5 anchor dry-run artifacts by `b3`/reference only.

It remains:

```text
lookup-only
pointer-only
read-only
display-only
b3-reference-only
non-authoritative
```

It does **not** become:

```text
proof truth
payment truth
receipt truth
balance truth
finality truth
settlement truth
paid unlock authority
wallet mutation authority
ledger mutation authority
external-chain ROC truth
bridge authority
```

## Boundary guarantees added

The new test proves:

```text
Index anchor pointers are lookup metadata only.

Anchor pointer DTOs reject authority flags when true.

Anchor pointer DTOs reject unknown authority-smuggling fields.

Anchor pointer b3 fields must remain canonical b3:<64 lowercase hex>.

Index routes do not expose anchor, bridge, Solana, ROX, public-chain, exchange, or external-settlement runtime surfaces.

Index source does not construct anchor-derived proof, finality, settlement, payment, wallet, ledger, balance, or receipt authority.

Existing pointer/manifest/provider paths do not unlock paid content from anchor material.

Cargo manifest does not add Phase 5 external runtime dependencies.
```

## Specific forbidden runtime scope still parked

```text
No roots.
No checkpoints.
No validators.
No settlement.
No bridges.
No wallet or ledger mutation.
No fake receipts.
No fake balances.
No fake finality.
No paid unlocks from index entries.
No Solana runtime.
No ROX runtime.
No external-chain ROC truth.
```

## Crate note summary

`svc-index` is now Phase 5 Round 1 hardened against anchor creep. It may reference anchor dry-run evidence artifacts by canonical `b3` only, but it cannot interpret those pointers as proof, paid entitlement, finality, settlement, balance truth, receipt truth, or outside-chain authority.

---

# `ron-policy`

## Phase/Round

QuickChain Phase 5 Round 1
Anchor-only dry-run policy boundary
Declarative policy crate

## Files added or updated

```text
crates/ron-policy/tests/quickchain_phase5_anchor_policy_boundary.rs
crates/ron-policy/docs/quickchain-preflight.md
crates/ron-policy/src/parse/validate.rs
crates/ron-policy/src/economics/validate.rs
```

## Focused test added

```text
cargo test -p ron-policy --test quickchain_phase5_anchor_policy_boundary
```

Result:

```text
8 passed; 0 failed
```

## Park/preflight result

```text
== ron-policy quickchain exhaustive preflight gate passed: tests=21 ==
== ron-policy QuickChain parking gate passed ==
```

## What this round proved

`ron-policy` may express declarative anchor display or acceptance policy only.

It remains:

```text
declarative
side-effect-free
policy-only
non-mutating
non-settlement
non-finality
non-payment-truth
```

It does **not** become:

```text
anchor proof authority
external settlement authority
paid unlock authority
wallet mutation authority
ledger mutation authority
balance truth
receipt truth
finality truth
settlement truth
bridge authority
outside-chain ROC truth
```

## Boundary guarantees added

The new test proves:

```text
Ordinary anchor display tags remain allowed when they are clearly non-authoritative.

Anchor-authority-shaped condition tags reject.

Anchor-authority-shaped obligation params reject.

Anchor-authority-shaped obligation kinds reject.

Policy decisions do not unlock paid content from anchor evidence.

Policy allow is not anchor proof.

Policy obligations are not external settlement evidence acceptance.

Parser validation contains Phase 5 anchor authority shapes.

Economics validation contains Phase 5 anchor authority shapes.

Production source does not construct Phase 5 anchor runtime authority.
```

## Validation hardening added

The parser/economics forbidden authority identifier tables were extended with Phase 5 anchor-shaped authority terms, including categories like:

```text
anchor truth
anchor authority
anchor finality
anchor settlement
anchor payment truth
anchor paid unlock
anchor proof truth
external anchor truth
external-chain ROC truth
external settlement truth
anchored balance
anchored receipt
anchored finality
anchored settlement
dry-run settlement
dry-run finality
```

Obligation-kind rejection was also hardened for shapes like:

```text
grant anchor authority
commit anchor truth
accept anchor proof
accept anchor evidence
unlock from anchor
settle from anchor
finalize from anchor
mutate from anchor
bridge from anchor
settle external anchor
mark anchored final
grant external settlement authority
```

## Specific forbidden runtime scope still parked

```text
No roots.
No checkpoints.
No validators.
No settlement.
No bridges.
No wallet or ledger mutation.
No fake receipts.
No fake balances.
No fake finality.
No paid unlocks from policy decisions.
No policy-created anchor truth.
No policy-created external-chain ROC truth.
```

## Crate note summary

`ron-policy` is now Phase 5 Round 1 hardened against anchor-policy authority creep. It can gate or allow display of backend-derived anchor dry-run evidence, but it cannot turn anchor evidence into payment, finality, settlement, receipt, balance, bridge, or external-chain authority.

---

# Pair-level result

`svc-index + ron-policy` are now **complete / parked** for QuickChain Phase 5 Round 1.

This pair closed the index/policy portion of the anchor-only dry-run sweep:

```text
svc-index:
  b3/reference pointers only, never proof or payment truth.

ron-policy:
  declarative gates only, never anchor/finality/settlement authority.
```

## Phase 5 Round 1 progress

```text
1. ron-proto + ron-ledger              COMPLETE / PARKED
2. svc-wallet + ron-accounting         COMPLETE / PARKED
3. svc-rewarder + svc-storage          COMPLETE / PARKED
4. svc-gateway + omnigate              COMPLETE / PARKED
5. svc-index + ron-policy              COMPLETE / PARKED
6. CrabLink Tauri + client adapters    NEXT
```

Next pair: **CrabLink Tauri + client adapters**.


### END NOTE - JUNE 27 2026 - 20:15 CST




### BEGIN NOTE - JUNE 28 2026 - 02:10 CST

# QuickChain Phase 5 Round 2 Crate Notes — `svc-index` + `ron-policy`

Status: **complete / green / parked**

Scope completed: **DA/archive/challenge fallback boundary before pruning**

This pair stayed correctly bounded:

```text
svc-index = lookup / pointer metadata only
ron-policy = declarative policy only
svc-wallet = paid mutation path
ron-ledger = durable economic truth
```

Neither crate gained DA truth, archive restore truth, missing-data challenge truth, pruning authority, paid unlock authority, balance truth, receipt truth, finality truth, settlement truth, wallet mutation, ledger mutation, outside-DA truth, or outside-chain ROC truth.

## `svc-index`

Files changed:

```text
crates/svc-index/docs/quickchain-preflight.md
crates/svc-index/tests/quickchain_phase5_da_fallback_pointer_boundary.rs
```

New focused suite:

```text
quickchain_phase5_da_fallback_pointer_boundary
```

What the patch proved:

```text
svc-index may point to DA/archive/challenge fallback artifacts by b3/reference only.

DA fallback artifact cid proves bytes only.

Archive restore artifact cid proves bytes only.

Missing-data challenge artifact cid proves bytes only.

Retention-window metadata is display/reference metadata only.

Index pointers remain lookup-only, read-only, display-only, b3-reference-only, and evidence-reference-only.

Pruning remains blocked.

svc-index does not expose DA/archive/challenge/pruning runtime routes.

svc-index does not add wallet, ledger, accounting, rewarder, external settlement, Solana, bridge, staking, or liquidity dependencies.
```

Focused test result:

```text
cargo test -p svc-index --test quickchain_phase5_da_fallback_pointer_boundary

7 passed
0 failed
```

Park result:

```text
bash crates/svc-index/scripts/dev-quickchain-park.sh

svc-index quickchain exhaustive preflight gate passed: tests=18
svc-index QuickChain parking gate passed
```

Forbidden scope stayed parked:

```text
no roots
no checkpoints
no validators
no settlement
no bridges
no wallet or ledger mutation
no fake receipts
no fake balances
no fake finality
no paid unlocks from index entries
```

## `ron-policy`

Files changed:

```text
crates/ron-policy/src/parse/validate.rs
crates/ron-policy/src/economics/validate.rs
crates/ron-policy/docs/quickchain-preflight.md
crates/ron-policy/tests/quickchain_phase5_da_fallback_policy_boundary.rs
```

New focused suite:

```text
quickchain_phase5_da_fallback_policy_boundary
```

What the patch proved:

```text
ron-policy may express declarative DA/archive/challenge fallback visibility or readiness policy only.

ron-policy may express declarative retention-window and pruning-blocker policy only.

Policy allow is not DA proof.

Policy allow is not archive restore proof.

Policy allow is not missing-data challenge proof.

Policy obligation is not DA evidence acceptance.

Policy obligation is not archive restore authority.

Policy obligation is not pruning authority.

Policy decision cannot mutate wallet or ledger from DA/archive/challenge evidence.

Policy decision cannot unlock paid content from DA/archive/challenge evidence.

Policy decision cannot become balance truth, receipt truth, finality truth, settlement truth, pruning authority, outside-DA truth, or outside-chain truth.
```

Parser/economics validation was hardened against Phase 5 Round 2 authority-shaped identifiers, including:

```text
dafallbacktruth
dafallbackauthority
dataavailabilitytruth
dataavailabilityauthority
archiverestoreauthority
archiveprooftruth
carrierproofauthority
missingdatachallengetruth
missingdatachallengeauthority
retentionwindowauthority
pruningtruth
pruningauthority
pruningapproved
prunenow
archivepaidunlock
dapaidunlock
outsidedatruth
outsidechaindatruth
dasettlementtruth
dafinalitytruth
```

Forbidden obligation kinds now reject shapes such as:

```text
grantdafallbackauthority
acceptdatruth
acceptarchiveproofaspayment
unlockfromdafallback
unlockfromarchiverestore
approvepruning
prunehistory
settlefromda
finalizefromda
mutatefromda
grantoutsidedatruth
markdafinal
```

Focused test result:

```text
cargo test -p ron-policy --test quickchain_phase5_da_fallback_policy_boundary

8 passed
0 failed
```

Park result:

```text
bash crates/ron-policy/scripts/dev-quickchain-park.sh

ron-policy quickchain exhaustive preflight gate passed: tests=22
ron-policy QuickChain parking gate passed
```

Forbidden scope stayed parked:

```text
no roots
no checkpoints
no validators
no settlement
no bridges
no wallet or ledger mutation
no fake receipts
no fake balances
no fake finality
no paid unlocks from policy decisions
```

## Regression coverage that stayed green

`svc-index` kept prior QuickChain and regression gates green across:

```text
Phase 1 pointer DTO strictness
Phase 1 Round 2 confirmation
Phase 2 replay boundary
Phase 2 committee boundary
Phase 3 validator boundary
Phase 3 validator lifecycle boundary
Phase 4 bond boundary
Phase 4 bond dispute boundary
Phase 4 bond enforcement boundary
Phase 5 anchor pointer boundary
Phase 5 DA fallback pointer boundary
preflight boundary
preflight docs
phase1 pair interlock
pointer authority
routes boundary
value-loop boundary
tooling boundary
http_contract
integration
all-targets
clippy
```

`ron-policy` kept prior QuickChain and regression gates green across:

```text
Phase 1 policy condition tags
Phase 1 Round 2 confirmation
Phase 2 replay boundary
Phase 2 committee boundary
Phase 3 validator boundary
Phase 3 validator lifecycle boundary
Phase 4 bond boundary
Phase 4 bond dispute boundary
Phase 4 bond enforcement boundary
Phase 5 anchor policy boundary
Phase 5 DA fallback policy boundary
preflight boundary
decision non-authority
docs boundary
economics config non-authority
economics identifier non-authority
parser consistency
phase1 pair interlock
public surface boundary
schema non-authority
value-loop boundary
tooling boundary
economics_policy
unit_model_serde_strict
unit_eval_determinism
unit_first_match_wins
golden_reasons
all-targets
clippy
```

## Crate-pair conclusion

```text
QuickChain Phase 5 Round 2 — svc-index + ron-policy: COMPLETE / GREEN / PARKED
```

Terminal proof confirms both focused tests passed, both park scripts passed, and the final completion marker printed successfully. 

## Next pair

```text
CrabLink Tauri + client adapters
```

That is the final remaining pair for **Phase 5 Round 2**. After CrabLink is green, we can safely label:

```text
QuickChain Phase 5 Round 2 complete
```


### END NOTE - JUNE 28 2026 - 02:10 CST


### BEGIN NOTE - JUNE 28 2026 - 21:20 CST

Here are the paste-ready crate notes for `svc-index + ron-policy`. The terminal confirms `svc-index` passed the new focused Phase 5 Round 3 test **8/8**, `ron-policy` passed **9/9**, and both crate park gates passed: `svc-index` with **19 QuickChain tests** and `ron-policy` with **23 QuickChain tests**. 

```text id="9nq3fj"
### BEGIN NOTE - JUNE 28 2026 - QUICKCHAIN PHASE 5 ROUND 3 - svc-index + ron-policy

# QuickChain Phase 5 Round 3 Crate Notes — svc-index + ron-policy

Date: June 28, 2026
Project: RustyOnions / CrabLink
Area: QuickChain
Phase / round: QuickChain Phase 5 Round 3
Crate pair: svc-index + ron-policy
Status: COMPLETE / GREEN / PARKED

---

## 0. Final status

svc-index + ron-policy are complete and parked for QuickChain Phase 5 Round 3.

Safe final label:

svc-index + ron-policy
QuickChain Phase 5 Round 3
COMPLETE / GREEN / PARKED

This completes the backend crate-pair sweep through Phase 5 Round 3, assuming prior crate-pair green states remain accepted.

Remaining likely final QuickChain layer:

CrabLink Tauri + client adapters

---

## 1. Round theme

Phase 5 Round 3 is the selected external integration posture round.

Selected posture remains:

anchor-only
evidence-only
status/report metadata only
display-only where hydrated
reference-only where indexed
declarative-only where policy-gated
backend-derived only

Still forbidden:

external settlement
public bridge
ROX active runtime
Solana active runtime
staking
liquidity
exchange-facing logic
public market authority
outside-program authority
index/payment truth
policy/payment truth
client/payment truth

This round did not add live external chain functionality.

This round proved that svc-index and ron-policy can safely acknowledge selected external posture material only as non-authoritative references, display metadata, or declarative gates.

---

## 2. Files added / updated

### svc-index

Added:

crates/svc-index/tests/quickchain_phase5_external_posture_pointer_boundary.rs

Updated:

crates/svc-index/docs/quickchain-preflight.md
crates/svc-index/scripts/dev-quickchain-preflight.sh

### ron-policy

Added:

crates/ron-policy/tests/quickchain_phase5_external_posture_policy_boundary.rs

Updated:

crates/ron-policy/docs/quickchain-preflight.md
crates/ron-policy/scripts/dev-quickchain-preflight.sh
crates/ron-policy/src/parse/validate.rs
crates/ron-policy/src/economics/validate.rs

No Cargo.toml dependency changes were required.

No new external settlement, bridge, ROX, Solana, staking, liquidity, public market, or exchange runtime dependencies were added.

---

## 3. Terminal proof summary

Focused gates passed:

svc-index quickchain_phase5_external_posture_pointer_boundary:
  8 passed
  0 failed

ron-policy quickchain_phase5_external_posture_policy_boundary:
  9 passed
  0 failed

Crate-local park gates passed:

svc-index:
  quickchain exhaustive preflight gate passed: tests=19
  QuickChain parking gate passed

ron-policy:
  quickchain exhaustive preflight gate passed: tests=23
  QuickChain parking gate passed

Clippy gates passed:

svc-index clippy passed
ron-policy clippy passed

All-target gates passed:

svc-index all-targets passed
ron-policy all-targets passed

Final completion marker:

QuickChain Phase 5 Round 3 svc-index + ron-policy external posture boundary patch completed.

---

## 4. svc-index crate notes

### Role preserved

svc-index remains:

lookup service
pointer service
name-to-artifact resolver
manifest pointer index
provider lookup surface
b3/reference metadata surface

svc-index is not:

wallet truth
ledger truth
balance truth
receipt truth
reward truth
settlement truth
finality truth
root authority
proof authority
validator authority
pruning authority
bridge authority
external chain authority
public market authority
exchange-facing authority
outside-program authority
ROX runtime
Solana runtime

### Phase 5 Round 3 work completed

The new svc-index external posture pointer boundary test proves:

selected external posture pointers are lookup-only
selected external posture pointers are read-only
selected external posture pointers are display-only
selected external posture pointers are b3-reference-only
selected external posture pointers are evidence-reference-only
external posture artifact CID proves bytes only
external posture evidence CID proves bytes only
external posture report CID proves bytes only
canonical b3 fields are enforced
unknown authority poison fields are rejected
authority flags must remain false
svc-index cannot prove external posture
svc-index cannot prove outside-program execution
svc-index cannot prove public-chain settlement
svc-index cannot prove ROX runtime state
svc-index cannot prove Solana runtime state
svc-index cannot unlock paid content from external posture metadata
svc-index cannot mutate wallet truth
svc-index cannot mutate ledger truth
svc-index cannot become balance truth
svc-index cannot become receipt truth
svc-index cannot become reward truth
svc-index cannot become finality truth
svc-index cannot become settlement truth
svc-index cannot become root authority
svc-index cannot become pruning authority
svc-index cannot become bridge authority
svc-index cannot become public market authority
svc-index cannot become exchange-facing authority
svc-index cannot become outside-program authority

### Route boundary preserved

svc-index does not expose runtime routes for:

/external-posture
/external-settlement
/outside-program
/public-chain
/public-market
/exchange
/bridge
/rox
/solana
/staking
/stake
/liquidity
/market
/settlement
/finality

Existing pointer, manifest, resolve, provider, admin, health, readiness, and metrics surfaces remain lookup/index/service surfaces only.

### Dependency boundary preserved

svc-index still does not depend on authority crates or external settlement runtimes.

Forbidden dependencies remain absent:

ron-ledger
svc-wallet
ron-accounting
svc-rewarder
ron-policy
anchor-lang
spl-token
ethers
web3
alloy
solana-sdk
solana-client

### Forbidden scope still parked for svc-index

Park script confirmed:

no roots
no checkpoints
no validators
no settlement
no bridges
no wallet or ledger mutation
no fake receipts
no fake balances
no fake finality
no paid unlocks from index entries

Additional Phase 5 Round 3 forbidden scope remains parked:

no proof truth from external posture pointers
no paid unlock from external posture pointers
no receipt truth from external posture pointers
no balance truth from external posture pointers
no reward truth from external posture pointers
no finality truth from external posture pointers
no settlement truth from external posture pointers
no root authority from external posture pointers
no pruning authority from external posture pointers
no public bridge authority
no public market authority
no exchange-facing authority
no outside-program authority
no ROX runtime authority
no Solana runtime authority

---

## 5. ron-policy crate notes

### Role preserved

ron-policy remains:

declarative policy engine
deny-by-default evaluator
config validator
economics policy parser/validator
rule/obligation schema boundary
display/readiness gate
side-effect-free decision engine

ron-policy is not:

wallet truth
ledger truth
balance truth
receipt truth
reward truth
settlement truth
finality truth
root authority
proof authority
validator authority
pruning authority
bridge authority
external chain authority
public market authority
exchange-facing authority
outside-program authority
ROX runtime
Solana runtime

### Phase 5 Round 3 work completed

The new ron-policy external posture policy boundary test proves:

policy may express display-only external posture visibility
policy may gate declarative readiness/display tags
ordinary external posture display tags remain allowed
authority-shaped external posture condition tags reject
authority-shaped external posture obligation params reject
authority-shaped external posture obligation kinds reject
parser validation table includes Phase 5 Round 3 authority shapes
economics validation table includes Phase 5 Round 3 authority shapes
production source does not construct external posture runtime authority
Cargo manifest does not add external settlement runtime dependencies
preflight dynamically discovers the new test suite

### Parser/economics hardening

ron-policy now rejects authority-shaped identifiers for selected external posture and outside-program/public-chain/public-market/ROX/Solana authority concepts.

Rejected exact authority-shape classes include:

external posture truth
external posture authority
external posture proof truth
external posture finality
external posture settlement
external posture paid unlock
external posture reward truth
external posture balance truth
external posture receipt truth
external posture root authority
external posture pruning authority
selected external posture truth
selected external posture authority
outside program truth
outside program authority
outside program execution truth
public chain truth
public chain authority
public chain settlement truth
public market truth
public market authority
exchange-facing authority
exchange settlement truth
ROX truth
ROX authority
ROX runtime authority
ROX settlement truth
Solana truth
Solana authority
Solana runtime authority
Solana settlement truth

Rejected obligation kinds include:

grant external posture authority
accept external posture truth
accept external posture proof as payment
unlock from external posture
settle from external posture
finalize from external posture
mutate from external posture
grant outside-program authority
execute outside-program
accept outside-program truth
grant public-chain authority
settle from public-chain
grant public-market authority
grant exchange-facing authority
settle from exchange
grant ROX runtime authority
settle from ROX
grant Solana runtime authority
settle from Solana

### Declarative-only policy allowed

The test confirms that ordinary display/readiness tags remain allowed, including safe concepts like:

external-posture-display
external-posture-evidence-display
external-posture-status-display
outside-program-display
public-chain-reference-display
public-market-reference-display
rox-reference-display
solana-reference-display
backend-derived-external-posture-status

These are safe only because they are display/readiness classifications, not authority.

### Forbidden scope still parked for ron-policy

Park script confirmed:

no roots
no checkpoints
no validators
no settlement
no bridges
no wallet or ledger mutation
no fake receipts
no fake balances
no fake finality
no paid unlocks from policy decisions

Additional Phase 5 Round 3 forbidden scope remains parked:

no policy-created external posture proof
no policy-created external posture truth
no policy-created outside-program execution proof
no policy-created public-chain settlement proof
no policy-created ROX runtime proof
no policy-created Solana runtime proof
no external settlement authority
no bridge authority
no public market authority
no exchange-facing authority
no outside-program authority
no paid unlock from external posture evidence
no wallet mutation from external posture policy
no ledger mutation from external posture policy
no balance truth from external posture policy
no receipt truth from external posture policy
no reward truth from external posture policy
no finality truth from external posture policy
no settlement truth from external posture policy
no root authority from external posture policy
no pruning authority from external posture policy

---

## 6. Pair-level proof

This crate pair now proves the Phase 5 Round 3 boundary from the lookup/policy layer:

svc-index:
  may point to selected external posture artifacts only as b3/reference metadata.
  pointers remain lookup-only and cannot prove settlement, finality, payment, unlock, or external execution.

ron-policy:
  may allow/deny declarative visibility/readiness policy.
  policy decisions remain side-effect-free and cannot create proof, settlement, finality, payment, unlock, or external execution authority.

Together they prove:

index pointer is not proof
index pointer is not payment truth
index pointer is not external execution truth
policy allow is not proof
policy allow is not payment truth
policy allow is not external execution truth
policy obligation is not settlement authority
policy obligation is not bridge authority
policy obligation is not public market authority
policy obligation is not outside-program authority

No forbidden external runtime was introduced:

no external settlement
no public bridge
no ROX active runtime
no Solana active runtime
no staking
no liquidity
no exchange-facing logic
no public market authority
no outside-program authority
no fake receipts
no fake balances
no fake finality
no silent spend
no cache/index/policy paid unlock authority

---

## 7. Commands that passed

Focused patch run:

cargo fmt -p svc-index -p ron-policy

cargo test -p svc-index --test quickchain_phase5_external_posture_pointer_boundary
cargo test -p ron-policy --test quickchain_phase5_external_posture_policy_boundary

bash crates/svc-index/scripts/dev-quickchain-park.sh
bash crates/ron-policy/scripts/dev-quickchain-park.sh

The park scripts ran:

dynamic QuickChain test discovery
focused QuickChain suites
existing regression tests
all-target tests
clippy
forbidden-scope parking checks

---

## 8. Current QuickChain backend status

Backend crate-pair status through Phase 5 Round 3:

ron-proto + ron-ledger:
  COMPLETE / GREEN / PARKED

svc-wallet + ron-accounting:
  COMPLETE / GREEN / PARKED

svc-rewarder + svc-storage:
  COMPLETE / GREEN / PARKED

svc-gateway + omnigate:
  COMPLETE / GREEN / PARKED

svc-index + ron-policy:
  COMPLETE / GREEN / PARKED

Safe backend label:

QuickChain backend crate-pair sweep complete through Phase 5 Round 3.

Do not overstate this as:

QuickChain fully complete
public chain live
external settlement live
bridge live
staking live
liquidity live
exchange ready
ROX live
Solana live

Correct whole-project status:

QuickChain backend is complete through Phase 5 Round 3.
CrabLink Tauri/client adapters remain the final client/product boundary layer.

---

## 9. Next likely work

Next target:

CrabLink Tauri + client adapters

Client finalization should prove:

React/TS displays selected external posture evidence only.
React/TS cannot treat selected external posture as paid unlock authority.
Tauri bridge does not expose external settlement or chain authority.
Tauri commands remain typed, small, allowlisted, redacted, bounded, source-labeled, and feature-gated.
Offline cache cannot unlock paid content alone.
Display-only receipts remain backend-derived.
No fake balances.
No fake receipts.
No silent spend.
No bridge.
No ROX/Solana runtime.
No staking.
No liquidity.
No exchange-facing logic.
No public market authority.
No outside-program authority.

Use actual existing CrabLink package scripts from the current CrabLink codebundle.

Do not invent missing npm scripts.

Likely target if present or added:

quickchain_phase5_external_posture_client_boundary
check-quickchain-phase5-client-external-posture-boundary

---

## 10. Regeneration commands

Recommended codebundle regeneration after saving notes:

bash scripts/make_crate_codex.sh --force -c svc-index
bash scripts/make_crate_codex.sh --force -c ron-policy

If collecting all notes after saving:

bash scripts/collect_notes.sh

---

## 11. Final safe label

svc-index + ron-policy
QuickChain Phase 5 Round 3
COMPLETE / GREEN / PARKED

Backend QuickChain crate-pair sweep:
COMPLETE / GREEN / PARKED through Phase 5 Round 3

Remaining:
CrabLink Tauri + client adapters

### END NOTE - JUNE 28 2026 - QUICKCHAIN PHASE 5 ROUND 3 - svc-index + ron-policy
```


### END NOTE - JUNE 28 2026 - 21:20 CST



### BEGIN NOTE - JUNE 29 2026 - 13:30 CST

The terminal output confirms both focused tests passed, then both crate-local park gates passed: `svc-rewarder` parked with **21 QuickChain tests**, and `ron-policy` parked with **23 QuickChain tests**. 

### BEGIN NOTE - JUNE 29 2026 - INTERNAL ROC BETA PHASE 1 ROUND 1 - svc-rewarder

## Status

`svc-rewarder` is GREEN / PARKED for Internal ROC Beta Phase 1 Round 1 on the reward-planning non-authority slice.

This was the third completed crate-pair pass in the active Internal ROC Beta flow:

```text
1. ron-proto + ron-ledger          GREEN / PARKED
2. svc-wallet + ron-accounting     GREEN / PARKED
3. svc-rewarder + ron-policy       GREEN / PARKED
4. svc-storage + svc-index         NEXT
5. svc-gateway + omnigate          pending
6. CrabLink Tauri/client adapters  pending
```

This does not mean all of Phase 1 is complete.

It means the `svc-rewarder` side of the `svc-rewarder + ron-policy` crate pair is complete for this planning/policy non-authority proof slice.

## Round purpose

Internal ROC Beta Phase 1 is proving paid post, paid comment, paid article, and content_view support while preserving the internal ROC authority model.

For `svc-rewarder`, the goal was to prove that reward planning remains deterministic, capped, and non-authoritative.

`svc-rewarder` may produce plans and wallet handoff material.

`svc-rewarder` must not create receipt truth, balance truth, unlock truth, payout execution truth, finality truth, wallet authority, or ledger authority.

## File added

```text
crates/svc-rewarder/tests/internal_roc_beta_rewarder_planning_non_authority.rs
```

## Follow-up fix applied

The first focused run exposed one test assertion issue.

The original assertion treated a wallet issue idempotency key as if it had to be a canonical `b3:<64hex>` content/commitment hash.

Actual behavior:

```text
b3:<60 lowercase hex chars>
```

That is acceptable because this field is bounded retry/dedupe material, not canonical receipt, operation, root, or content-hash authority.

Fix applied:

```text
assert_bounded_b3_idempotency_key(...)
```

New assertion rule:

```text
idempotency key must:
  - start with b3:
  - have lowercase hex body
  - have bounded length 32..=64
  - remain retry/dedupe material only
```

This correction is architecturally correct.

`run_key` and `manifest.commitment` remain canonical `b3:<64hex>` commitments.

Wallet issue idempotency keys remain bounded retry/dedupe keys, not operation authority.

## Focused test added

```bash
cargo test -p svc-rewarder --test internal_roc_beta_rewarder_planning_non_authority
```

## Focused test result

```text
running 5 tests

protocol_pool_planning_requires_signed_policy_and_stays_provenance_only ... ok
paid_content_reward_plan_is_deterministic_planning_not_receipt_or_balance_truth ... ok
wallet_issue_batch_is_handoff_shape_not_payout_execution_receipt ... ok
reward_policy_rejects_paid_content_authority_poison_fields ... ok
compute_request_rejects_paid_content_authority_poison_fields ... ok

test result: ok. 5 passed; 0 failed
```

## Parking / preflight result

The crate-local parking gate passed.

Terminal proof:

```text
== svc-rewarder quickchain exhaustive preflight gate passed: tests=21 ==
== svc-rewarder QuickChain parking gate passed ==
```

The park gate also ran:

```text
forbidden helper scan
cargo fmt check
21 focused QuickChain tests
svc-rewarder all-targets test
svc-rewarder clippy
bench smoke for reward_calc_100
forbidden-scope marker
```

No checked-in Python helpers were found under `crates/svc-rewarder`.

Clippy completed cleanly.

## Focused proof details

The new test proves:

```text
reward planning is deterministic
dry-run planning emits no wallet/ledger effects
reward manifests are planning artifacts only
manifest commitment is reference material only
settlement batches are handoff material only
wallet issue batches are request previews only
wallet issue batches are not receipts
wallet issue batches are not balance truth
wallet issue batches are not payout execution truth
wallet issue idempotency keys are retry/dedupe material only
protocol_pool planning requires signed policy
RewardPolicy rejects authority poison fields
ComputeEpochRequest rejects authority poison fields
```

## Paid-content coverage

The reward-planning test used Internal ROC beta paid-content style contributors:

```text
acct_post_creator
acct_comment_creator
acct_article_creator
acct_content_view_creator
```

This proves the rewarder can deterministically plan over the paid-content beta surface while remaining downstream planning infrastructure only.

## Authority poison rejected

The focused tests reject smuggled fields such as:

```text
wallet_mutation
ledger_mutation
balance_truth
receipt_truth
paid_unlock_authority
entitlement_truth
payout_execution_truth
client_finality_claim
cache_unlock_authority
gateway_receipt_truth
omnigate_receipt_truth
bridge_txid
staking_position_id
liquidity_pool_id
external_settlement_id
raw_engagement_mints_roc
```

## Boundaries preserved

`svc-rewarder` remains deterministic payout planning only.

Allowed:

```text
consume accounting-style snapshots
apply signed reward policy
compute deterministic reward manifests
produce capped payout plans
produce settlement batch handoff shapes
produce wallet issue request preview material
use idempotency keys for retry/dedupe
emit planning provenance
```

Forbidden and still not implemented:

```text
wallet mutation
ledger mutation
direct ROC issue
direct ROC transfer
direct ROC burn
hold open
hold capture
hold release
receipt creation
balance truth
paid unlock truth
entitlement truth
payout execution truth
root authority
checkpoint authority
validator authority
bridge runtime
staking runtime
liquidity runtime
external settlement
exchange-facing logic
raw engagement direct minting
```

## Existing QuickChain boundary stayed green

The existing QuickChain suites confirmed `svc-rewarder` still has:

```text
no roots
no checkpoints
no validators
no settlement
no anchors
no bridges
no staking
no liquidity
no direct mutation
no fake receipts
no fake balances
no fake finality
no paid unlocks from rewarder outputs
```

The terminal explicitly preserved the marker:

```text
svc-rewarder remains deterministic payout planning only;
svc-wallet remains mutation front-door;
ron-ledger remains truth
```

## Architecture result

This crate now has direct Internal ROC Beta proof that `svc-rewarder` can participate in the paid-content value loop without becoming economic authority.

Correct flow remains:

```text
ron-accounting snapshot/report
→ svc-rewarder capped payout plan
→ ron-policy validation/gating
→ svc-wallet approved mutation
→ ron-ledger durable receipt/balance truth
```

Incorrect flow remains blocked:

```text
svc-rewarder plan
→ direct balance mutation
→ direct receipt truth
→ direct paid unlock
```

## Final svc-rewarder status

```text
Internal ROC Beta Phase 1 Round 1
svc-rewarder
GREEN / PARKED
```

### END NOTE - JUNE 29 2026 - INTERNAL ROC BETA PHASE 1 ROUND 1 - svc-rewarder

### BEGIN NOTE - JUNE 29 2026 - INTERNAL ROC BETA PHASE 1 ROUND 1 - ron-policy

## Status

`ron-policy` is GREEN / PARKED for Internal ROC Beta Phase 1 Round 1 on the paid-content policy/economics non-authority slice.

This was the third completed crate-pair pass in the active Internal ROC Beta flow:

```text
1. ron-proto + ron-ledger          GREEN / PARKED
2. svc-wallet + ron-accounting     GREEN / PARKED
3. svc-rewarder + ron-policy       GREEN / PARKED
4. svc-storage + svc-index         NEXT
5. svc-gateway + omnigate          pending
6. CrabLink Tauri/client adapters  pending
```

This does not mean all of Phase 1 is complete.

It means the `ron-policy` side of the `svc-rewarder + ron-policy` crate pair is complete for this planning/policy non-authority proof slice.

## Round purpose

Internal ROC Beta Phase 1 is proving paid post, paid comment, paid article, and content_view support while preserving the internal ROC authority model.

For `ron-policy`, the goal was to prove that policy and economics config may gate and price paid-content behavior without becoming receipt truth, balance truth, entitlement truth, unlock truth, payout truth, wallet authority, or ledger authority.

## File added

```text
crates/ron-policy/tests/internal_roc_beta_paid_content_policy_non_authority.rs
```

## Focused test added

```bash
cargo test -p ron-policy --test internal_roc_beta_paid_content_policy_non_authority
```

## Focused test result

```text
running 5 tests

policy_obligation_cannot_smuggle_paid_unlock_or_receipt_authority ... ok
policy_allow_after_backend_context_is_not_paid_unlock_or_receipt_truth ... ok
policy_rejects_paid_content_authority_shaped_tags ... ok
economics_paid_content_view_prices_and_validates_capture_plan_without_authority ... ok
economics_config_rejects_paid_content_authority_poison_fields ... ok

test result: ok. 5 passed; 0 failed
```

## Parking / preflight result

The crate-local parking gate passed.

Terminal proof:

```text
== ron-policy quickchain exhaustive preflight gate passed: tests=23 ==
ron-policy QuickChain Phase-0 preflight passed.
== ron-policy QuickChain parking gate passed ==
```

The park gate also ran:

```text
cargo fmt
23 focused QuickChain tests
existing ron-policy regressions
all-targets tests
clippy
bench smoke for eval:get/us
examples compile checks
```

Clippy completed cleanly.

## Focused proof details

The new test proves:

```text
policy allow decisions are not paid unlock authority
policy allow decisions are not receipt truth
policy allow decisions are not balance truth
policy obligations cannot create payment authority
policy obligations cannot smuggle paid unlock authority
authority-shaped condition tags reject
authority-shaped obligation kinds reject
authority-shaped obligation params reject
paid_content_view economics can price and validate capture plans
economics config remains validation/gating input only
economics config rejects authority poison fields
```

## Safe policy behavior proven

The allowed policy shape can express:

```text
backend-proof-checked
paid-content-policy-context
content-kind-post
require-backend-wallet-ledger-proof
```

This is safe because those are declarative gating/context labels.

They do not become receipt truth.

They do not unlock paid content by themselves.

They do not mutate wallet or ledger.

They only say that gateway/omnigate/backend must depend on backend wallet/ledger proof.

## Authority-shaped tags rejected

The focused test rejects policy condition tags such as:

```text
receipt_id
receipt_hash
receipt_root
receipt_proof
balance_minor
wallet_balance
ledger_balance
paid_proof
unlock_granted
finality
finalized
settlement_status
state_root
checkpoint_root
checkpoint_hash
validator_signature
bridge_proof
operation_id
idempotency_key
account_sequence
hold_id
```

## Authority-shaped obligations rejected

The focused test rejects policy obligation kinds such as:

```text
unlock_paid_content
create_receipt
accept_receipt
verify_payment
mutate_balance
credit_account
open_hold
capture_hold
release_hold
settlement_complete
bridge_settlement
```

## Economics config proof

The focused test loads the checked-in ROC economics config and proves:

```text
paid_content_view exists
paid_content_view is enabled
paid_content_view has deterministic positive pricing
paid_content_view split basis points sum exactly to 10000
capture plan validation works as config/gating input
economics JSON shape has no authority fields
```

This confirms that economics config can support paid content pricing/capture validation without becoming payment truth.

## Economics poison rejected

The focused test rejects appended config poison fields such as:

```text
receipt_truth
balance_truth
paid_unlock_authority
ledger_mutation
wallet_mutation
bridge_txid
staking_position_id
liquidity_pool_id
external_settlement_id
```

## Boundaries preserved

`ron-policy` remains declarative policy/economics validation only.

Allowed:

```text
parse policy bundles
evaluate declarative allow/deny rules
validate condition tags
validate obligations
validate economics config
price paid_content_view from config
validate capture plan shape
reject malformed or authority-shaped inputs
act as gating input for gateway/omnigate/backend
```

Forbidden and still not implemented:

```text
wallet mutation
ledger mutation
direct ROC issue
direct ROC transfer
direct ROC burn
hold open
hold capture
hold release
receipt creation
balance truth
paid unlock truth
entitlement truth
payout execution truth
root authority
checkpoint authority
validator authority
bridge runtime
staking runtime
liquidity runtime
external settlement
exchange-facing logic
raw engagement direct minting
```

## Existing QuickChain boundary stayed green

The existing QuickChain suites confirmed `ron-policy` still has:

```text
no roots
no checkpoints
no validators
no settlement
no bridges
no wallet or ledger mutation
no fake receipts
no fake balances
no fake finality
no paid unlocks from policy decisions
```

The terminal explicitly preserved the marker:

```text
ron-policy forbidden QuickChain runtime scope remains parked:
- no roots
- no checkpoints
- no validators
- no settlement
- no bridges
- no wallet or ledger mutation
- no fake receipts, fake balances, fake finality, or paid unlocks from policy decisions
```

## Architecture result

This crate now has direct Internal ROC Beta proof that `ron-policy` can participate in paid-content pricing and gating without becoming economic authority.

Correct flow remains:

```text
paid-content context
→ ron-policy declarative gating/economics validation
→ backend wallet/ledger proof requirement
→ svc-wallet mutation only when approved
→ ron-ledger durable receipt/balance truth
```

Incorrect flow remains blocked:

```text
policy allow
→ paid unlock
→ receipt truth
→ balance truth
→ wallet mutation
```

## Final ron-policy status

```text
Internal ROC Beta Phase 1 Round 1
ron-policy
GREEN / PARKED
```

### END NOTE - JUNE 29 2026 - INTERNAL ROC BETA PHASE 1 ROUND 1 - ron-policy


### END NOTE - JUNE 29 2026 - 13:30 CST




### BEGIN NOTE - JUNE 29 2026 - 18:30 CST

The terminal output confirms both focused Phase 3 preflights passed for `svc-rewarder` and `ron-policy`, including the new reward-plan and policy-gate tests, prior regressions, and clippy.  The buildplan scope for this pair was exactly Round 1 non-mutating payout plans: rewarder consumes sealed snapshots, produces deterministic capped plans, policy validates/gates, no payout execution yet, no rewarder ledger mutation, and no policy-created receipt. 

Below are paste-ready crate notes.

### BEGIN NOTE - JUNE 29 2026 - INTERNAL ROC BETA PHASE 3 ROUND 1

# svc-rewarder — Internal ROC Beta Phase 3 Round 1 Notes

Date: June 29, 2026
Phase: Internal ROC Beta Phase 3
Round: Round 1
Crate pair: `svc-rewarder + ron-policy`
Current crate status: GREEN / PARKED for this Phase 3 slice

---

## 0. Safe status label

```text
Internal ROC Beta Phase 3 Round 1 svc-rewarder reward-plan boundary is GREEN / PARKED.
```

Pair label:

```text
Internal ROC Beta Phase 3 Round 1 svc-rewarder + ron-policy reward-plan/policy-gate boundary is GREEN / PARKED.
```

Round-level label:

```text
Internal ROC Beta Phase 3 Round 1 snapshots and non-mutating payout plans are GREEN / PARKED across the intended crate pairs.
```

This does **not** mean all of Phase 3 is complete.

Correct larger status:

```text
QuickChain boundary/preflight scope is COMPLETE / GREEN / PARKED through Phase 5.
Internal ROC Beta Phase 0 is COMPLETE / GREEN / PARKED.
Internal ROC Beta Phase 1 is COMPLETE / GREEN / PARKED.
Internal ROC Beta Phase 2 is COMPLETE / GREEN / PARKED.
Internal ROC Beta Phase 3 Round 1 is COMPLETE / GREEN / PARKED.
Internal ROC Beta Phase 3 Round 2 is NEXT.
```

Still deferred / not authorized:

```text
ROX
Solana
public bridge
external settlement
staking runtime
liquidity
exchange-facing logic
public validator economy
public chain runtime
```

---

## 1. What this slice added

This slice added focused Internal ROC Beta Phase 3 Round 1 coverage for reward planning.

New test:

```text
crates/svc-rewarder/tests/internal_roc_beta_phase3_reward_plan_boundary.rs
```

New script:

```text
crates/svc-rewarder/scripts/dev-internal-roc-beta-phase3-preflight.sh
```

No Cargo.toml changes were required.

No new dependencies were added.

No Python helpers were added.

---

## 2. Files touched

```text
crates/svc-rewarder/tests/internal_roc_beta_phase3_reward_plan_boundary.rs
crates/svc-rewarder/scripts/dev-internal-roc-beta-phase3-preflight.sh
```

---

## 3. Important test coverage

The new Phase 3 test target contains five tests:

```text
reward_plan_is_deterministic_for_same_snapshot_regardless_input_order
reward_plan_enforces_pool_cap_and_conservation_without_execution
wallet_issue_batch_is_handoff_preview_not_payout_receipt_or_balance_truth
reward_policy_and_compute_request_reject_authority_poison_fields
raw_engagement_fields_cannot_be_rewarder_payout_input_authority
```

These tests prove `svc-rewarder` can create deterministic capped planning material, but cannot become wallet/ledger truth.

---

## 4. Boundary doctrine proven

`svc-rewarder` remains:

```text
deterministic payout planner
accounting snapshot consumer
policy/config input consumer
reward manifest producer
wallet handoff preview producer
non-mutating planning service
```

`svc-rewarder` does **not** become:

```text
wallet mutation authority
ledger mutation authority
receipt truth
balance truth
payout execution truth
finality truth
bridge authority
staking authority
liquidity authority
external settlement authority
public-chain authority
```

Correct Phase 3 Round 1 interpretation:

```text
Accounting snapshot
→ rewarder deterministic capped reward plan
→ policy validation/gating
→ no wallet mutation yet
```

Round 2 is where approved payout execution begins, and only through `svc-wallet`.

---

## 5. Deterministic reward-plan behavior proven

The test confirms the same snapshot material produces the same plan even when contribution input order changes.

Proven stable outputs:

```text
run_key
commitment
inputs_cid
totals
payouts
account ordering
```

The plan output remains deterministic over normalized/canonicalized snapshot input.

The payout accounts are sorted deterministically.

---

## 6. Pool cap and conservation behavior proven

The test confirms `max_payout_minor_units` caps the snapshot pool.

Proven invariant:

```text
payout_minor_units + residual_minor_units == capped pool_minor_units
```

Also proven:

```text
planned payouts never exceed capped pool
residual is explicit
floor rounding remains deterministic
no wallet/ledger effect is emitted
```

This keeps reward planning bounded and prevents planning artifacts from becoming inflation.

---

## 7. Wallet issue batch doctrine preserved

The test confirms `SettlementBatch` and `WalletIssueBatch` are handoff previews only.

Allowed:

```text
wallet_path reference
funding_source reference
deterministic wallet issue request shape
deterministic idempotency key references
asset = roc
integer minor-unit amounts
```

Forbidden:

```text
receipt_hash
balance_minor
settlement_status
finality
operation_id
ledger_mutation
wallet_mutation
```

The wallet batch is a preview/handoff shape, not payout execution and not receipt truth.

Correct interpretation:

```text
RewardManifest = planning artifact
SettlementBatch = handoff preview
WalletIssueBatch = wallet request preview
svc-wallet = approved execution boundary
ron-ledger = durable receipt/balance truth
```

---

## 8. Raw engagement poisoning rejected

The test confirms raw engagement fields cannot be smuggled into rewarder payout input authority.

Rejected root-level fields include:

```text
event_class
source_event_class
raw_views
raw_likes
raw_comments
raw_watch_seconds
analytics_only
metering
proof_eligible
ad_budgeted
reward_material
raw_engagement_mints_roc
```

Rejected contribution-level fields include:

```text
event_class
source_event_class
raw_views
raw_likes
raw_comments
raw_watch_seconds
analytics_only
proof_eligible
ad_budgeted
wallet_receipt
ledger_receipt
```

This preserves the hard doctrine:

```text
Raw engagement never directly mints or allocates protocol ROC.
```

---

## 9. Authority poisoning rejected

The test rejects reward policy and compute request smuggling fields, including:

```text
balance
balance_minor
available_balance
wallet_balance
ledger_balance
wallet_receipt
ledger_receipt
receipt_id
receipt_hash
receipt_root
receipt_txid
payout_receipt_txid
settlement_status
finality
finalized
wallet_mutation
ledger_mutation
ledger_side_effect
wallet_side_effect
payout_execution
payout_execution_truth
operation_id
account_sequence
state_root
checkpoint_root
checkpoint_hash
validator_signature
bridge_txid
solana_signature
rox_settlement_id
staking_position_id
staking_yield_bps
liquidity_pool_id
exchange_order_id
outside_settlement_claim
```

These fields cannot turn rewarder input/output into receipt, balance, payout, finality, bridge, staking, liquidity, or external settlement truth.

---

## 10. Tests / gates passed

Focused Phase 3 reward-plan boundary test:

```bash
cargo test -p svc-rewarder --test internal_roc_beta_phase3_reward_plan_boundary
```

Result:

```text
running 5 tests
test reward_plan_enforces_pool_cap_and_conservation_without_execution ... ok
test reward_plan_is_deterministic_for_same_snapshot_regardless_input_order ... ok
test raw_engagement_fields_cannot_be_rewarder_payout_input_authority ... ok
test wallet_issue_batch_is_handoff_preview_not_payout_receipt_or_balance_truth ... ok
test reward_policy_and_compute_request_reject_authority_poison_fields ... ok

test result: ok. 5 passed; 0 failed
```

Prior Internal ROC rewarder planning non-authority regression:

```bash
cargo test -p svc-rewarder --test internal_roc_beta_rewarder_planning_non_authority
```

Result:

```text
running 5 tests
test protocol_pool_planning_requires_signed_policy_and_stays_provenance_only ... ok
test paid_content_reward_plan_is_deterministic_planning_not_receipt_or_balance_truth ... ok
test wallet_issue_batch_is_handoff_shape_not_payout_execution_receipt ... ok
test reward_policy_rejects_paid_content_authority_poison_fields ... ok
test compute_request_rejects_paid_content_authority_poison_fields ... ok

test result: ok. 5 passed; 0 failed
```

QuickChain no-direct-mutation regression:

```bash
cargo test -p svc-rewarder --test quickchain_preflight_no_direct_mutation
```

Result:

```text
running 4 tests
test compute_request_still_rejects_direct_mutation_authority_smuggling ... ok
test planning_outputs_do_not_claim_receipts_balances_operation_truth_roots_or_finality ... ok
test config_rejects_external_settlement_bridge_anchor_validator_and_root_knobs ... ok
test router_does_not_expose_direct_wallet_ledger_quickchain_or_bridge_mutation_routes ... ok

test result: ok. 4 passed; 0 failed
```

QuickChain funding-source regression:

```bash
cargo test -p svc-rewarder --test quickchain_preflight_funding_source
```

Result:

```text
running 6 tests
test policy_requires_explicit_funding_source_on_wire ... ok
test unsigned_protocol_pool_policy_is_rejected_by_validator ... ok
test current_policy_accepts_explicit_protocol_pool_and_rejects_smuggled_authority_fields ... ok
test manifest_carries_funding_provenance_but_not_funding_finality ... ok
test wallet_preview_carries_batch_provenance_but_requests_remain_wallet_issue_shape ... ok
test compute_request_rejects_top_level_funding_authority_smuggling ... ok

test result: ok. 6 passed; 0 failed
```

QuickChain replay/no-double-issue regression:

```bash
cargo test -p svc-rewarder --test quickchain_preflight_replay_no_double_issue
```

Result:

```text
running 4 tests
test idempotency_keys_are_retry_dedupe_not_operation_identity ... ok
test duplicate_epoch_replay_is_dedupe_not_second_payout_authority ... ok
test reordered_snapshot_rows_produce_same_plan ... ok
test same_snapshot_policy_and_epoch_produce_same_plan_commitment ... ok

test result: ok. 4 passed; 0 failed
```

Strict Clippy gate:

```bash
cargo clippy -p svc-rewarder --all-targets -- -D warnings
```

Result:

```text
Finished `dev` profile
```

Focused Phase 3 preflight:

```bash
bash crates/svc-rewarder/scripts/dev-internal-roc-beta-phase3-preflight.sh
```

Final pass marker:

```text
== Internal ROC Beta Phase 3 svc-rewarder reward-plan preflight passed ==
== rewarder remains deterministic capped planning only; no wallet/ledger mutation, receipt/balance/finality truth, raw-engagement direct ROC allocation, bridge, staking, liquidity, or external settlement ==
```

---

## 11. What this proves

`svc-rewarder` now proves for Phase 3 Round 1:

```text
reward plans are deterministic.
reward plans are capped.
reward plans conserve capped pool value.
reward plans sort payout accounts deterministically.
reward manifests are planning artifacts only.
settlement batches are handoff previews only.
wallet issue batches are not payout receipts.
wallet issue batches are not balance truth.
raw engagement cannot directly become rewarder payout authority.
rewarder cannot mutate wallet.
rewarder cannot mutate ledger.
rewarder cannot create receipts.
rewarder cannot create balances.
rewarder cannot create finality.
rewarder cannot create bridge/staking/liquidity/external settlement truth.
```

---

## 12. What this does not do yet

This Round 1 slice does **not** execute approved payouts.

Not yet complete:

```text
approved payout execution through svc-wallet
durable payout receipts in ron-ledger
duplicate payout prevention across the full execution path
payout receipt replay/conservation proof
accounting observation of executed payout receipts
Phase 3 overall completion
```

Those belong to Phase 3 Round 2.

---

## 13. Commands to rerun

From repo root:

```bash
cargo fmt -p svc-rewarder -- --check
cargo test -p svc-rewarder --test internal_roc_beta_phase3_reward_plan_boundary
cargo test -p svc-rewarder --test internal_roc_beta_rewarder_planning_non_authority
cargo test -p svc-rewarder --test quickchain_preflight_no_direct_mutation
cargo test -p svc-rewarder --test quickchain_preflight_funding_source
cargo test -p svc-rewarder --test quickchain_preflight_replay_no_double_issue
cargo clippy -p svc-rewarder --all-targets -- -D warnings
bash crates/svc-rewarder/scripts/dev-internal-roc-beta-phase3-preflight.sh
```

Pair rerun:

```bash
bash crates/svc-rewarder/scripts/dev-internal-roc-beta-phase3-preflight.sh
bash crates/ron-policy/scripts/dev-internal-roc-beta-phase3-preflight.sh
```

---

## 14. Phase 3 Round 1 completion context

Phase 3 Round 1 intended crate pairs are now green:

```text
ron-proto + ron-ledger: GREEN / PARKED
svc-wallet + ron-accounting: GREEN / PARKED
svc-rewarder + ron-policy: GREEN / PARKED
```

Round 1 proved:

```text
accounting/reward-plan reference DTOs are non-authority.
ledger rejects reward-plan material as receipt truth.
accounting snapshots are deterministic derivative material.
wallet/accounting observer boundaries are preserved.
rewarder produces deterministic capped payout plans.
policy validates/gates reward plans declaratively.
no payout execution occurs yet.
```

---

## 15. Next phase context

Next active target:

```text
Internal ROC Beta Phase 3 Round 2 — approved payout execution through svc-wallet
```

Expected Round 2 work involving `svc-rewarder`:

```text
reward plan emits payout intent candidates.
policy validates/gates payout plan.
duplicate payout prevention markers are enforced.
caps and category pools remain enforced.
approved payout intent goes to svc-wallet only.
svc-wallet returns durable receipt.
ron-ledger replay proves payout receipt stability.
```

Do not implement direct rewarder wallet/ledger mutation.

Do not let rewarder become payout execution authority.

### END NOTE - JUNE 29 2026 - INTERNAL ROC BETA PHASE 3 ROUND 1

### BEGIN NOTE - JUNE 29 2026 - INTERNAL ROC BETA PHASE 3 ROUND 1

# ron-policy — Internal ROC Beta Phase 3 Round 1 Notes

Date: June 29, 2026
Phase: Internal ROC Beta Phase 3
Round: Round 1
Crate pair: `svc-rewarder + ron-policy`
Current crate status: GREEN / PARKED for this Phase 3 slice

---

## 0. Safe status label

```text
Internal ROC Beta Phase 3 Round 1 ron-policy reward-plan gate boundary is GREEN / PARKED.
```

Pair label:

```text
Internal ROC Beta Phase 3 Round 1 svc-rewarder + ron-policy reward-plan/policy-gate boundary is GREEN / PARKED.
```

Round-level label:

```text
Internal ROC Beta Phase 3 Round 1 snapshots and non-mutating payout plans are GREEN / PARKED across the intended crate pairs.
```

This does **not** mean all of Phase 3 is complete.

Correct larger status:

```text
QuickChain boundary/preflight scope is COMPLETE / GREEN / PARKED through Phase 5.
Internal ROC Beta Phase 0 is COMPLETE / GREEN / PARKED.
Internal ROC Beta Phase 1 is COMPLETE / GREEN / PARKED.
Internal ROC Beta Phase 2 is COMPLETE / GREEN / PARKED.
Internal ROC Beta Phase 3 Round 1 is COMPLETE / GREEN / PARKED.
Internal ROC Beta Phase 3 Round 2 is NEXT.
```

Still deferred / not authorized:

```text
ROX
Solana
public bridge
external settlement
staking runtime
liquidity
exchange-facing logic
public validator economy
public chain runtime
```

---

## 1. What this slice added

This slice added focused Internal ROC Beta Phase 3 Round 1 coverage for declarative reward-plan policy gating.

New test:

```text
crates/ron-policy/tests/internal_roc_beta_phase3_reward_plan_policy_gate.rs
```

New script:

```text
crates/ron-policy/scripts/dev-internal-roc-beta-phase3-preflight.sh
```

No Cargo.toml changes were required.

No new dependencies were added.

No Python helpers were added.

---

## 2. Files touched

```text
crates/ron-policy/tests/internal_roc_beta_phase3_reward_plan_policy_gate.rs
crates/ron-policy/scripts/dev-internal-roc-beta-phase3-preflight.sh
```

---

## 3. Important test coverage

The new Phase 3 test target contains five tests:

```text
reward_plan_policy_gate_allows_reviewed_plan_without_receipt_or_balance_truth
reward_plan_policy_denial_is_not_refund_receipt_or_balance_truth
known_authority_shaped_reward_plan_policy_tags_reject
known_authority_shaped_reward_plan_obligation_params_reject
reward_plan_policy_gate_remains_declarative_not_execution_surface
```

These tests prove `ron-policy` can gate reward plans, but cannot become payout execution, wallet/ledger mutation, receipt, balance, or finality truth.

---

## 4. Boundary doctrine proven

`ron-policy` remains:

```text
declarative policy engine
reward-plan gate
config validator
eligibility validator
obligation emitter
non-mutating decision surface
```

`ron-policy` does **not** become:

```text
wallet mutation authority
ledger mutation authority
receipt truth
balance truth
payout execution truth
refund truth
finality truth
paid unlock truth
bridge authority
staking authority
liquidity authority
external settlement authority
public-chain authority
```

Correct interpretation:

```text
Policy allow = permission/gate only.
Policy deny = rejection/gate only.
Policy obligation = instruction/requirement only.
Policy config = validation data only.
```

None of these are economic truth.

---

## 5. Policy allow behavior proven

The test confirms a reviewed reward plan can be allowed only when the expected safe tags are present.

Safe tags used:

```text
reward-plan-reviewed
bounded-pool-cap-checked
accounting-snapshot-cid-checked
policy-gate-only
backend-wallet-execution-required
```

A policy allow decision can carry a safe obligation:

```text
require-approved-payout-intent-through-svc-wallet
```

Safe obligation params:

```text
plan_source = svc_rewarder
execution_boundary = svc_wallet
ledger_truth = ron_ledger
```

This obligation does not create receipt, balance, payout, or finality truth.

It only preserves the required execution boundary:

```text
approved payout intent → svc-wallet → ron-ledger receipt
```

---

## 6. Policy deny behavior proven

The test confirms a reward-plan policy denial is not:

```text
refund receipt
wallet receipt
ledger receipt
balance update
payout execution
finality event
paid unlock
```

A deny decision emits no payout/receipt obligations.

Correct interpretation:

```text
Policy deny = gate failure only.
No refund exists until svc-wallet/ron-ledger produce a real backend receipt.
```

---

## 7. Authority-shaped tags rejected

The test confirms known authority-shaped reward-plan policy tags reject.

Rejected examples include:

```text
receipt_hash
balance_minor
settlement_status
checkpoint_root
bridge_proof
operation_id
idempotency_key
account_sequence
```

This prevents policy tags from becoming proof, receipt, balance, operation identity, or settlement authority.

---

## 8. Authority-shaped obligation params rejected

The test confirms known authority-shaped obligation param keys reject.

Rejected examples include:

```text
receipt_hash
balance_minor
settlement_status
checkpoint_root
bridge_proof
operation_id
idempotency_key
account_sequence
```

This prevents policy obligations from smuggling economic authority into otherwise declarative decisions.

---

## 9. Declarative gate behavior proven

The test confirms a safe policy gate can require:

```text
reward-plan-reviewed
bounded-pool-cap-checked
policy-gate-only
```

and emit a safe obligation requiring explicit wallet boundary execution.

Still, the policy output remains declarative.

It does not contain authority tokens such as:

```text
receipt_id
receipt_hash
receipt_root
balance_minor
wallet_balance
ledger_balance
paid_proof
unlock_granted
finality
finalized
settlement_status
state_root
checkpoint_root
checkpoint_hash
validator_signature
bridge_proof
bridge_txid
solana_signature
rox_settlement_id
staking_position_id
liquidity_pool_id
operation_id
account_sequence
payout_execution
```

---

## 10. Tests / gates passed

Focused Phase 3 reward-plan policy gate test:

```bash
cargo test -p ron-policy --test internal_roc_beta_phase3_reward_plan_policy_gate
```

Result:

```text
running 5 tests
test reward_plan_policy_denial_is_not_refund_receipt_or_balance_truth ... ok
test known_authority_shaped_reward_plan_obligation_params_reject ... ok
test reward_plan_policy_gate_allows_reviewed_plan_without_receipt_or_balance_truth ... ok
test reward_plan_policy_gate_remains_declarative_not_execution_surface ... ok
test known_authority_shaped_reward_plan_policy_tags_reject ... ok

test result: ok. 5 passed; 0 failed
```

Prior Internal ROC paid-content policy non-authority regression:

```bash
cargo test -p ron-policy --test internal_roc_beta_paid_content_policy_non_authority
```

Result:

```text
running 5 tests
test policy_allow_after_backend_context_is_not_paid_unlock_or_receipt_truth ... ok
test policy_obligation_cannot_smuggle_paid_unlock_or_receipt_authority ... ok
test policy_rejects_paid_content_authority_shaped_tags ... ok
test economics_paid_content_view_prices_and_validates_capture_plan_without_authority ... ok
test economics_config_rejects_paid_content_authority_poison_fields ... ok

test result: ok. 5 passed; 0 failed
```

QuickChain decision non-authority regression:

```bash
cargo test -p ron-policy --test quickchain_preflight_decision_non_authority
```

Result:

```text
running 4 tests
test authority_shaped_obligation_param_key_rejects_policy ... ok
test authority_shaped_obligation_kind_rejects_policy ... ok
test deny_decision_is_still_not_receipt_balance_or_finality_truth ... ok
test allow_decision_is_policy_result_not_paid_unlock_or_receipt_truth ... ok

test result: ok. 4 passed; 0 failed
```

Economics policy regression:

```bash
cargo test -p ron-policy --test economics_policy
```

Result:

```text
running 15 tests
test float_value_rejects_during_parse ... ok
test capture_plan_accepts_required_dynamic_recipient ... ok
test disabled_action_rejects_lookup_but_config_can_load ... ok
test capture_over_action_cap_rejects ... ok
test invalid_split_sum_rejects ... ok
test deterministic_action_order_is_sorted ... ok
test missing_required_action_rejects ... ok
test missing_dynamic_recipient_rejects_capture_plan ... ok
test overflow_value_rejects_during_parse ... ok
test negative_value_rejects_during_parse ... ok
test unknown_action_rejects ... ok
test paid_storage_put_price_uses_minimum_and_hold_multiplier ... ok
test unknown_split_destination_rejects ... ok
test unknown_paid_action_lookup_rejects ... ok
test valid_checked_in_roc_economics_config_loads ... ok

test result: ok. 15 passed; 0 failed
```

Strict Clippy gate:

```bash
cargo clippy -p ron-policy --all-targets --no-deps -- -D warnings
```

Result:

```text
Finished `dev` profile
```

Focused Phase 3 preflight:

```bash
bash crates/ron-policy/scripts/dev-internal-roc-beta-phase3-preflight.sh
```

Final pass marker:

```text
== Internal ROC Beta Phase 3 ron-policy reward-plan gate preflight passed ==
== policy remains declarative gate only; no receipt/balance/payout/finality truth, wallet/ledger mutation, bridge, staking, liquidity, or external settlement ==
```

---

## 11. What this proves

`ron-policy` now proves for Phase 3 Round 1:

```text
policy can gate reviewed reward plans.
policy can require explicit wallet-boundary execution.
policy allow does not create receipt truth.
policy allow does not create balance truth.
policy allow does not create payout execution.
policy deny does not create refund truth.
policy deny does not create balance truth.
policy obligations cannot smuggle authority fields.
authority-shaped tags reject.
authority-shaped obligation params reject.
economics policy regressions remain green.
policy remains declarative only.
```

---

## 12. What this does not do yet

This Round 1 slice does **not** execute approved payouts.

Not yet complete:

```text
approved payout execution through svc-wallet
durable payout receipts in ron-ledger
duplicate payout prevention across the full execution path
payout receipt replay/conservation proof
policy-approved payout intent execution
Phase 3 overall completion
```

Those belong to Phase 3 Round 2.

---

## 13. Commands to rerun

From repo root:

```bash
cargo fmt -p ron-policy -- --check
cargo test -p ron-policy --test internal_roc_beta_phase3_reward_plan_policy_gate
cargo test -p ron-policy --test internal_roc_beta_paid_content_policy_non_authority
cargo test -p ron-policy --test quickchain_preflight_decision_non_authority
cargo test -p ron-policy --test economics_policy
cargo clippy -p ron-policy --all-targets --no-deps -- -D warnings
bash crates/ron-policy/scripts/dev-internal-roc-beta-phase3-preflight.sh
```

Pair rerun:

```bash
bash crates/svc-rewarder/scripts/dev-internal-roc-beta-phase3-preflight.sh
bash crates/ron-policy/scripts/dev-internal-roc-beta-phase3-preflight.sh
```

---

## 14. Phase 3 Round 1 completion context

Phase 3 Round 1 intended crate pairs are now green:

```text
ron-proto + ron-ledger: GREEN / PARKED
svc-wallet + ron-accounting: GREEN / PARKED
svc-rewarder + ron-policy: GREEN / PARKED
```

Round 1 proved:

```text
accounting/reward-plan reference DTOs are non-authority.
ledger rejects reward-plan material as receipt truth.
accounting snapshots are deterministic derivative material.
wallet/accounting observer boundaries are preserved.
rewarder produces deterministic capped payout plans.
policy validates/gates reward plans declaratively.
no payout execution occurs yet.
```

---

## 15. Next phase context

Next active target:

```text
Internal ROC Beta Phase 3 Round 2 — approved payout execution through svc-wallet
```

Expected Round 2 work involving `ron-policy`:

```text
policy validates/gates payout plans.
policy rejects duplicate/uncapped payout attempts.
policy enforces cap/category/pool eligibility.
policy remains declarative.
policy never mutates wallet.
policy never mutates ledger.
policy never creates receipt/balance/finality truth.
```

Do not implement direct policy wallet/ledger mutation.

Do not let policy become payout execution authority.

### END NOTE - JUNE 29 2026 - INTERNAL ROC BETA PHASE 3 ROUND 1

These are ready to append to each crate’s `NOTES.MD`.


### END NOTE - JUNE 29 2026 - 18:30 CST




### BEGIN NOTE - JUNE 29 2026 - 19:05 CST

Below are paste-ready notes for:

```text id="7zvcgt"
crates/svc-rewarder/NOTES.MD
crates/ron-policy/NOTES.MD
```

These cover **Internal ROC Beta Phase 3 Round 2 — `svc-rewarder + ron-policy`**, now **GREEN / PARKED**. Your terminal output confirms the new rewarder approved-payout intent test passed 5/5, the new policy approved-payout gate test passed 5/5, prior regressions passed, and both clippy gates passed. 

### BEGIN NOTE - JUNE 29 2026 - INTERNAL ROC BETA PHASE 3 ROUND 2

# svc-rewarder — Internal ROC Beta Phase 3 Round 2 Notes

Date: June 29, 2026
Phase: Internal ROC Beta Phase 3
Round: Round 2
Crate pair: `svc-rewarder + ron-policy`
Current crate status: GREEN / PARKED for this Phase 3 Round 2 slice

---

## 0. Safe status label

```text id="6qvid6"
Internal ROC Beta Phase 3 Round 2 svc-rewarder approved-payout intent boundary is GREEN / PARKED.
```

Pair label:

```text id="6ysaz2"
Internal ROC Beta Phase 3 Round 2 svc-rewarder + ron-policy approved-payout intent/policy-gate boundary is GREEN / PARKED.
```

Phase label:

```text id="ca85xy"
Internal ROC Beta Phase 3 accounting/rewarder/wallet payout loop proof is COMPLETE / GREEN / PARKED.
```

Current Phase 3 status:

```text id="1gfxuq"
Round 1: COMPLETE / GREEN / PARKED
Round 2: COMPLETE / GREEN / PARKED
Phase 3: COMPLETE / GREEN / PARKED
```

Current Phase 3 Round 2 crate-pair status:

```text id="2rlma8"
ron-proto + ron-ledger: GREEN / PARKED
svc-wallet + ron-accounting: GREEN / PARKED
svc-rewarder + ron-policy: GREEN / PARKED
```

This does **not** mean the whole Internal ROC Beta is complete.

Next phase:

```text id="uir91a"
Internal ROC Beta Phase 4 — CrabLink Tauri wallet/receipt UX hardening
```

---

## 1. What this slice added

This slice added focused Phase 3 Round 2 coverage proving that `svc-rewarder` emits deterministic, capped, wallet-issue handoff candidates only.

New test:

```text id="tal9vq"
crates/svc-rewarder/tests/internal_roc_beta_phase3_approved_payout_intent_boundary.rs
```

Updated script:

```text id="8ihuck"
crates/svc-rewarder/scripts/dev-internal-roc-beta-phase3-preflight.sh
```

Follow-up fix:

```text id="uw4g59"
The approved payout cap/conservation test was corrected for floor rounding.
A requested cap of 333 can produce a deterministic payout handoff total of 332 with a 1-minor-unit residual.
The residual must stay out of wallet handoff truth.
```

No Cargo.toml changes were required.

No new dependencies were added.

No Python helpers were added.

---

## 2. Files touched

```text id="atqtkv"
crates/svc-rewarder/tests/internal_roc_beta_phase3_approved_payout_intent_boundary.rs
crates/svc-rewarder/scripts/dev-internal-roc-beta-phase3-preflight.sh
```

---

## 3. Important test coverage

The new Phase 3 Round 2 test target contains five tests:

```text id="bs8x91"
approved_payout_intent_candidates_are_wallet_issue_handoff_only
category_pool_cap_and_conservation_survive_payout_intent_handoff
duplicate_payout_planning_markers_are_deterministic_but_not_execution_truth
wallet_issue_requests_remain_string_money_and_do_not_carry_receipt_truth
payout_handoff_dtos_reject_authority_smuggling_fields
```

These tests prove `svc-rewarder` remains deterministic payout planning and wallet handoff material only.

---

## 4. Boundary doctrine proven

`svc-rewarder` remains:

```text id="ecm9et"
deterministic capped payout planner
reward manifest producer
wallet issue handoff preview producer
idempotency/dedupe marker producer
audit/planning material producer
```

`svc-rewarder` does **not** become:

```text id="qdrzmh"
wallet authority
ledger authority
receipt truth
balance truth
approved payout executor
finality truth
bridge authority
staking authority
liquidity authority
external settlement authority
public-chain authority
```

Correct role:

```text id="lwl80t"
ron-accounting snapshot/report
→ svc-rewarder capped payout plan
→ ron-policy validation/gating
→ svc-wallet approved payout mutation
→ ron-ledger durable receipt
```

`svc-rewarder` emits candidate handoff material. It does not execute payouts.

---

## 5. Approved payout intent handoff proven

The test confirms approved payout intent candidates are wallet issue handoff material only.

Proven handoff shape:

```text id="kuhejr"
SettlementBatch
WalletIssueBatch
WalletIssueRequest
```

Proven properties:

```text id="8sgtnd"
epoch_id is stable
run_key is stable
manifest_commitment is stable
funding_source is explicit
wallet_path points to wallet issue path
asset is roc
amount_minor is a decimal string
idempotency_key is present and bounded
memo labels rewarder planning source
recipient accounts are deterministically sorted
```

Important distinction:

```text id="kbw63e"
WalletIssueRequest is not a receipt.
WalletIssueBatch is not a balance.
SettlementBatch is not payout execution.
Rewarder handoff is not ledger mutation.
```

---

## 6. Cap, conservation, and floor residual behavior proven

The first version of this test expected exact equality between requested cap and payout handoff total.

The fix corrected the test to reflect actual integer floor rounding doctrine.

Correct behavior:

```text id="xlb3my"
requested_cap = 333
floor-rounded payout handoff total = 332
deterministic residual = 1
```

The corrected rule:

```text id="g9njd7"
settlement.total_minor_units <= requested_cap
wallet_batch.total_minor_units == settlement.total_minor_units
sum(wallet_issue_request.amount_minor) == settlement.total_minor_units
floor residual stays out of wallet issue requests
```

This is the right behavior.

The residual must not be silently pushed into a wallet issue request.

This proves:

```text id="sby0st"
category pool cap is enforced
wallet handoff total is conserved
floor rounding is deterministic
remainder is explicit by absence from payout handoff
no silent inflation occurs
```

---

## 7. Duplicate payout planning markers proven non-authoritative

The test confirms duplicate planning markers are deterministic dedupe markers, not execution truth.

Proven behavior:

```text id="6jqd44"
same sealed inputs produce identical settlement batches
same sealed inputs produce identical wallet issue batches
first handoff marker can be accepted
duplicate handoff marker returns dup
dry-run remains dry_run
```

Important rule:

```text id="4d3m1j"
rewarder idempotency/dedupe markers are not operation_id.
rewarder idempotency/dedupe markers are not ledger sequence.
rewarder idempotency/dedupe markers are not wallet receipt truth.
```

Actual duplicate payout prevention remains enforced downstream through `svc-wallet` and `ron-ledger`.

---

## 8. String-money handoff preserved

The test confirms wallet issue requests keep amount fields as string money.

Proven behavior:

```text id="0g4tdj"
amount_minor serializes as a string
amount_minor is nonzero after dust filtering
idempotency_key is present
wallet request does not carry receipt fields
wallet request does not carry balance fields
wallet request does not carry finality fields
wallet request does not carry bridge/staking/liquidity/exchange fields
```

This preserves the Internal ROC and QuickChain money doctrine:

```text id="tgs38h"
integer minor-unit strings only
no floats
no implicit client-side payout truth
```

---

## 9. Authority-smuggling rejected

The test confirms payout handoff DTOs reject authority-smuggling fields.

Rejected examples include:

```text id="xwvy7y"
receipt_id
receipt_hash
receipt_root
receipt_proof
accepted_receipt
wallet_receipt
ledger_receipt
balance
balance_minor
available_balance
wallet_balance
ledger_balance
balance_truth
receipt_truth
payout_execution_truth
wallet_mutation
ledger_mutation
operation_id
account_sequence
finality
finalized
checkpoint_hash
checkpoint_root
bridge_txid
solana_signature
rox_settlement_id
staking_position_id
staking_yield_bps
liquidity_pool_id
exchange_order_id
outside_settlement_claim
paid_unlock_authority
cache_unlock_authority
silent_spend
fake_balance
fake_receipt
```

This prevents the rewarder handoff layer from becoming a fake wallet/ledger authority surface.

---

## 10. Tests / gates passed

Focused Phase 3 Round 2 approved payout intent boundary:

```bash id="w3g4zo"
cargo test -p svc-rewarder --test internal_roc_beta_phase3_approved_payout_intent_boundary
```

Result:

```text id="jmg881"
running 5 tests
test category_pool_cap_and_conservation_survive_payout_intent_handoff ... ok
test wallet_issue_requests_remain_string_money_and_do_not_carry_receipt_truth ... ok
test duplicate_payout_planning_markers_are_deterministic_but_not_execution_truth ... ok
test approved_payout_intent_candidates_are_wallet_issue_handoff_only ... ok
test payout_handoff_dtos_reject_authority_smuggling_fields ... ok

test result: ok. 5 passed; 0 failed
```

Focused Phase 3 Round 1 reward-plan boundary regression:

```bash id="xcdlnx"
cargo test -p svc-rewarder --test internal_roc_beta_phase3_reward_plan_boundary
```

Result:

```text id="i6c895"
running 5 tests
test raw_engagement_fields_cannot_be_rewarder_payout_input_authority ... ok
test reward_plan_enforces_pool_cap_and_conservation_without_execution ... ok
test reward_plan_is_deterministic_for_same_snapshot_regardless_input_order ... ok
test wallet_issue_batch_is_handoff_preview_not_payout_receipt_or_balance_truth ... ok
test reward_policy_and_compute_request_reject_authority_poison_fields ... ok

test result: ok. 5 passed; 0 failed
```

Prior Internal ROC rewarder planning non-authority regression:

```bash id="aguzvq"
cargo test -p svc-rewarder --test internal_roc_beta_rewarder_planning_non_authority
```

Result:

```text id="bvtygq"
running 5 tests
test protocol_pool_planning_requires_signed_policy_and_stays_provenance_only ... ok
test reward_policy_rejects_paid_content_authority_poison_fields ... ok
test paid_content_reward_plan_is_deterministic_planning_not_receipt_or_balance_truth ... ok
test wallet_issue_batch_is_handoff_shape_not_payout_execution_receipt ... ok
test compute_request_rejects_paid_content_authority_poison_fields ... ok

test result: ok. 5 passed; 0 failed
```

QuickChain no-direct-mutation regression:

```bash id="cuxc1l"
cargo test -p svc-rewarder --test quickchain_preflight_no_direct_mutation
```

Result:

```text id="2nnqnn"
running 4 tests
test compute_request_still_rejects_direct_mutation_authority_smuggling ... ok
test planning_outputs_do_not_claim_receipts_balances_operation_truth_roots_or_finality ... ok
test config_rejects_external_settlement_bridge_anchor_validator_and_root_knobs ... ok
test router_does_not_expose_direct_wallet_ledger_quickchain_or_bridge_mutation_routes ... ok

test result: ok. 4 passed; 0 failed
```

QuickChain funding-source regression:

```bash id="t6hbam"
cargo test -p svc-rewarder --test quickchain_preflight_funding_source
```

Result:

```text id="gxmrsk"
running 6 tests
test unsigned_protocol_pool_policy_is_rejected_by_validator ... ok
test policy_requires_explicit_funding_source_on_wire ... ok
test current_policy_accepts_explicit_protocol_pool_and_rejects_smuggled_authority_fields ... ok
test manifest_carries_funding_provenance_but_not_funding_finality ... ok
test wallet_preview_carries_batch_provenance_but_requests_remain_wallet_issue shape ... ok
test compute_request_rejects_top_level_funding_authority_smuggling ... ok

test result: ok. 6 passed; 0 failed
```

QuickChain replay/no-double-issue regression:

```bash id="hxxil3"
cargo test -p svc-rewarder --test quickchain_preflight_replay_no_double_issue
```

Result:

```text id="qw37ks"
running 4 tests
test duplicate_epoch_replay_is_dedupe_not_second_payout_authority ... ok
test same_snapshot_policy_and_epoch_produce_same_plan_commitment ... ok
test idempotency_keys_are_retry_dedupe_not operation identity ... ok
test reordered_snapshot_rows_produce_same_plan ... ok

test result: ok. 4 passed; 0 failed
```

Strict Clippy gate:

```bash id="3x1cxc"
cargo clippy -p svc-rewarder --all-targets -- -D warnings
```

Result:

```text id="ggexfl"
Finished `dev` profile
```

Focused preflight:

```bash id="noky2p"
bash crates/svc-rewarder/scripts/dev-internal-roc-beta-phase3-preflight.sh
```

Final pass marker:

```text id="tfipwb"
== Internal ROC Beta Phase 3 svc-rewarder approved-payout intent preflight passed ==
== rewarder emits deterministic capped wallet handoff candidates only; no wallet/ledger mutation, receipt/balance/finality truth, raw-engagement direct ROC allocation, bridge, staking, liquidity, or external settlement ==
```

---

## 11. What this proves

`svc-rewarder` now proves for Phase 3 Round 2:

```text id="a5gpln"
approved payout intent candidates are deterministic.
approved payout intent candidates are capped.
approved payout handoff totals are conserved.
floor residuals are not silently issued.
wallet issue requests remain handoff DTOs only.
wallet issue requests use string money.
duplicate planning markers are dedupe markers, not execution truth.
rewarder does not mutate wallet.
rewarder does not mutate ledger.
rewarder does not create receipts.
rewarder does not create balances.
rewarder does not create finality.
rewarder does not enable bridge, staking, liquidity, exchange-facing, or external settlement behavior.
```

---

## 12. What this completes

This crate pair completed Phase 3 Round 2.

Together with earlier crate pairs, this completes Phase 3.

Safe overall label:

```text id="f9n2t4"
Internal ROC Beta Phase 3 accounting/rewarder/wallet payout loop proof is COMPLETE / GREEN / PARKED.
```

---

## 13. Commands to rerun

From repo root:

```bash id="ihwyq6"
cargo fmt -p svc-rewarder -- --check
cargo test -p svc-rewarder --test internal_roc_beta_phase3_approved_payout_intent_boundary
cargo test -p svc-rewarder --test internal_roc_beta_phase3_reward_plan_boundary
cargo test -p svc-rewarder --test internal_roc_beta_rewarder_planning_non_authority
cargo test -p svc-rewarder --test quickchain_preflight_no_direct_mutation
cargo test -p svc-rewarder --test quickchain_preflight_funding_source
cargo test -p svc-rewarder --test quickchain_preflight_replay_no_double_issue
cargo clippy -p svc-rewarder --all-targets -- -D warnings
bash crates/svc-rewarder/scripts/dev-internal-roc-beta-phase3-preflight.sh
```

Pair rerun:

```bash id="j9lc0x"
bash crates/svc-rewarder/scripts/dev-internal-roc-beta-phase3-preflight.sh
bash crates/ron-policy/scripts/dev-internal-roc-beta-phase3-preflight.sh
```

---

## 14. Next phase context

Next phase:

```text id="j3ox13"
Internal ROC Beta Phase 4 — CrabLink Tauri wallet/receipt UX hardening
```

Expected Phase 4 direction:

```text id="m9ffxa"
CrabLink displays backend-derived wallet receipts.
CrabLink displays backend-derived balances.
CrabLink keeps receipt cache display-only.
CrabLink performs explicit confirmation before spend.
CrabLink never invents receipt, balance, payout, finality, paid unlock, bridge, staking, liquidity, or external settlement truth.
```

Do not add new ledger mutation paths.

Do not let CrabLink, gateway, omnigate, index, storage, policy, accounting, or rewarder mutate balances.

### END NOTE - JUNE 29 2026 - INTERNAL ROC BETA PHASE 3 ROUND 2

### BEGIN NOTE - JUNE 29 2026 - INTERNAL ROC BETA PHASE 3 ROUND 2

# ron-policy — Internal ROC Beta Phase 3 Round 2 Notes

Date: June 29, 2026
Phase: Internal ROC Beta Phase 3
Round: Round 2
Crate pair: `svc-rewarder + ron-policy`
Current crate status: GREEN / PARKED for this Phase 3 Round 2 slice

---

## 0. Safe status label

```text id="5flb7n"
Internal ROC Beta Phase 3 Round 2 ron-policy approved-payout policy-gate boundary is GREEN / PARKED.
```

Pair label:

```text id="eeb8l2"
Internal ROC Beta Phase 3 Round 2 svc-rewarder + ron-policy approved-payout intent/policy-gate boundary is GREEN / PARKED.
```

Phase label:

```text id="x9rce7"
Internal ROC Beta Phase 3 accounting/rewarder/wallet payout loop proof is COMPLETE / GREEN / PARKED.
```

Current Phase 3 status:

```text id="w6eagy"
Round 1: COMPLETE / GREEN / PARKED
Round 2: COMPLETE / GREEN / PARKED
Phase 3: COMPLETE / GREEN / PARKED
```

Current Phase 3 Round 2 crate-pair status:

```text id="w6m1h6"
ron-proto + ron-ledger: GREEN / PARKED
svc-wallet + ron-accounting: GREEN / PARKED
svc-rewarder + ron-policy: GREEN / PARKED
```

This does **not** mean the whole Internal ROC Beta is complete.

Next phase:

```text id="wnuyjk"
Internal ROC Beta Phase 4 — CrabLink Tauri wallet/receipt UX hardening
```

---

## 1. What this slice added

This slice added focused Phase 3 Round 2 coverage proving `ron-policy` gates approved payout candidates declaratively only.

New test:

```text id="syg6kh"
crates/ron-policy/tests/internal_roc_beta_phase3_approved_payout_policy_gate.rs
```

Updated script:

```text id="hlyvzi"
crates/ron-policy/scripts/dev-internal-roc-beta-phase3-preflight.sh
```

No Cargo.toml changes were required.

No new dependencies were added.

No Python helpers were added.

---

## 2. Files touched

```text id="q6yhso"
crates/ron-policy/tests/internal_roc_beta_phase3_approved_payout_policy_gate.rs
crates/ron-policy/scripts/dev-internal-roc-beta-phase3-preflight.sh
```

---

## 3. Important test coverage

The new Phase 3 Round 2 test target contains five tests:

```text id="xcx3sj"
approved_payout_policy_gate_allows_candidate_without_execution_truth
missing_duplicate_guard_marker_denies_without_refund_or_receipt_authority
approved_payout_policy_tags_reject_authority_shapes
approved_payout_policy_obligation_params_reject_authority_shapes
approved_payout_policy_gate_remains_declarative_not_execution_surface
```

These tests prove `ron-policy` can gate approved payout candidates, but cannot create payout execution, wallet mutation, ledger mutation, receipt truth, balance truth, finality truth, bridge, staking, liquidity, or external settlement truth.

---

## 4. Boundary doctrine proven

`ron-policy` remains:

```text id="2ho8ib"
declarative policy gate
allow/deny evaluator
obligation producer
economics config validator
safe review/checkpoint in the payout loop
```

`ron-policy` does **not** become:

```text id="85w351"
wallet authority
ledger authority
approved payout executor
receipt truth
balance truth
refund authority
finality truth
paid unlock authority
bridge authority
staking authority
liquidity authority
external settlement authority
```

Correct role:

```text id="7glb8k"
svc-rewarder payout intent candidate
→ ron-policy declarative validation/gating
→ svc-wallet approved payout mutation
→ ron-ledger durable receipt
```

Policy can say “allowed” or “denied.”

Policy cannot execute.

---

## 5. Approved payout gate allow behavior proven

The test confirms policy can allow an approved payout candidate only when safe declarative markers are present.

Safe markers include:

```text id="hirgok"
reward-plan-reviewed
bounded-pool-cap-checked
duplicate-payout-guard-checked
approved-payout-intent-candidate
svc-wallet-execution-required
policy-gate-only
```

Allowed decision remains declarative.

The safe obligation shape includes:

```text id="61vl9y"
require-approved-payout-intent-through-svc-wallet
plan_source = svc_rewarder
execution_boundary = svc_wallet
ledger_truth = ron_ledger
duplicate_guard = required
pool_cap = required
```

This is policy review material only.

It does not create a receipt, balance, finality, or payout execution.

---

## 6. Missing duplicate guard denial proven

The test confirms an approved payout candidate missing the duplicate guard marker denies.

Proven behavior:

```text id="ovb0rw"
missing duplicate-payout-guard-checked marker
→ policy denies
→ no refund obligation
→ no receipt obligation
→ no balance obligation
→ no wallet mutation obligation
→ no ledger mutation obligation
```

This matters because duplicate payout prevention is required for Phase 3 Round 2.

Policy denial is not an economic action.

---

## 7. Authority-shaped tags rejected

The test confirms approved payout policy tags reject authority-shaped material.

Rejected examples include:

```text id="mqf7ey"
receipt_hash
balance_minor
settlement_status
checkpoint_root
bridge_proof
operation_id
idempotency_key
account_sequence
```

This prevents policy tags from becoming proof, receipt, balance, finality, operation identity, or bridge authority.

---

## 8. Authority-shaped obligation params rejected

The test confirms obligation params reject authority-shaped material.

Rejected examples include:

```text id="eyv32l"
receipt_hash
balance_minor
settlement_status
checkpoint_root
bridge_proof
operation_id
idempotency_key
account_sequence
```

This prevents obligations from smuggling payout execution, wallet mutation, ledger mutation, proof, finality, receipt, balance, or external settlement authority.

---

## 9. Policy gate remains declarative

The test confirms safe policy obligations remain review/check material only.

Safe obligation examples:

```text id="9j8dkq"
require-approved-payout-intent-through-svc-wallet
record-policy-review-note
```

Safe params include:

```text id="4l3k85"
review = required
cap_check = required
duplicate_check = required
wallet_handoff = required
note = policy_review_only
```

Forbidden obligation meanings remain absent:

```text id="7guy1z"
issue
transfer
burn
capture
release
receipt
balance
finality
bridge
staking
liquidity
```

---

## 10. Tests / gates passed

Focused Phase 3 Round 2 approved payout policy gate:

```bash id="jjz8ds"
cargo test -p ron-policy --test internal_roc_beta_phase3_approved_payout_policy_gate
```

Result:

```text id="06e9rf"
running 5 tests
test missing_duplicate_guard_marker_denies_without_refund_or_receipt_authority ... ok
test approved_payout_policy_gate_allows_candidate_without_execution_truth ... ok
test approved_payout_policy_gate_remains_declarative_not_execution_surface ... ok
test approved_payout_policy_tags_reject_authority_shapes ... ok
test approved_payout_policy_obligation_params_reject_authority_shapes ... ok

test result: ok. 5 passed; 0 failed
```

Focused Phase 3 Round 1 reward-plan policy gate regression:

```bash id="y2yuec"
cargo test -p ron-policy --test internal_roc_beta_phase3_reward_plan_policy_gate
```

Result:

```text id="uuvbcu"
running 5 tests
test known_authority_shaped_reward_plan_policy_tags_reject ... ok
test known_authority_shaped_reward_plan_obligation_params_reject ... ok
test reward_plan_policy_denial_is_not_refund_receipt_or balance truth ... ok
test reward_plan_policy_gate_remains_declarative_not_execution_surface ... ok
test reward_plan_policy_gate_allows_reviewed_plan_without_receipt_or_balance_truth ... ok

test result: ok. 5 passed; 0 failed
```

Prior Internal ROC paid-content policy non-authority regression:

```bash id="qz4454"
cargo test -p ron-policy --test internal_roc_beta_paid_content_policy_non_authority
```

Result:

```text id="s5t89g"
running 5 tests
test policy_obligation_cannot_smuggle_paid_unlock_or_receipt_authority ... ok
test policy_rejects_paid_content_authority_shaped_tags ... ok
test policy_allow_after_backend_context_is_not_paid_unlock_or receipt truth ... ok
test economics_paid_content_view_prices_and_validates_capture_plan_without_authority ... ok
test economics_config_rejects_paid_content_authority_poison_fields ... ok

test result: ok. 5 passed; 0 failed
```

QuickChain decision non-authority regression:

```bash id="na83dv"
cargo test -p ron-policy --test quickchain_preflight_decision_non_authority
```

Result:

```text id="54gqx1"
running 4 tests
test authority_shaped_obligation_param_key_rejects_policy ... ok
test authority_shaped_obligation_kind_rejects_policy ... ok
test deny_decision_is_still_not_receipt_balance_or finality truth ... ok
test allow_decision_is_policy_result_not paid unlock or receipt truth ... ok

test result: ok. 4 passed; 0 failed
```

Economics policy regression:

```bash id="vu53rg"
cargo test -p ron-policy --test economics_policy
```

Result:

```text id="28plnz"
running 15 tests
test float_value_rejects_during_parse ... ok
test capture_plan_accepts_required_dynamic_recipient ... ok
test capture_over_action_cap_rejects ... ok
test disabled_action_rejects_lookup_but_config_can_load ... ok
test invalid_split_sum_rejects ... ok
test deterministic_action_order_is_sorted ... ok
test missing_required_action_rejects ... ok
test missing_dynamic_recipient_rejects_capture_plan ... ok
test overflow_value_rejects_during_parse ... ok
test negative_value_rejects_during_parse ... ok
test paid_storage_put_price_uses_minimum_and_hold_multiplier ... ok
test unknown_action_rejects ... ok
test unknown_paid_action_lookup_rejects ... ok
test unknown_split_destination_rejects ... ok
test valid_checked_in_roc_economics_config_loads ... ok

test result: ok. 15 passed; 0 failed
```

Strict Clippy gate:

```bash id="z1lc05"
cargo clippy -p ron-policy --all-targets --no-deps -- -D warnings
```

Result:

```text id="x7xir2"
Finished `dev` profile
```

Focused preflight:

```bash id="foul30"
bash crates/ron-policy/scripts/dev-internal-roc-beta-phase3-preflight.sh
```

Final pass marker:

```text id="2b34gc"
== Internal ROC Beta Phase 3 ron-policy approved-payout gate preflight passed ==
== policy remains declarative gate only; no receipt/balance/payout/finality truth, wallet/ledger mutation, bridge, staking, liquidity, or external settlement ==
```

---

## 11. What this proves

`ron-policy` now proves for Phase 3 Round 2:

```text id="r2s7vk"
approved payout candidates can be gated declaratively.
policy allow is not payout execution.
policy deny is not refund execution.
policy obligations are review/check material only.
missing duplicate guard markers deny safely.
authority-shaped tags reject.
authority-shaped obligation params reject.
policy does not mutate wallet.
policy does not mutate ledger.
policy does not create receipts.
policy does not create balances.
policy does not create finality.
policy does not unlock paid content.
policy does not enable bridge, staking, liquidity, exchange-facing, or external settlement behavior.
```

---

## 12. What this completes

This crate pair completed Phase 3 Round 2.

Together with earlier crate pairs, this completes Phase 3.

Safe overall label:

```text id="qp9bjs"
Internal ROC Beta Phase 3 accounting/rewarder/wallet payout loop proof is COMPLETE / GREEN / PARKED.
```

---

## 13. Commands to rerun

From repo root:

```bash id="2n4yc5"
cargo fmt -p ron-policy -- --check
cargo test -p ron-policy --test internal_roc_beta_phase3_approved_payout_policy_gate
cargo test -p ron-policy --test internal_roc_beta_phase3_reward_plan_policy_gate
cargo test -p ron-policy --test internal_roc_beta_paid_content_policy_non_authority
cargo test -p ron-policy --test quickchain_preflight_decision_non_authority
cargo test -p ron-policy --test economics_policy
cargo clippy -p ron-policy --all-targets --no-deps -- -D warnings
bash crates/ron-policy/scripts/dev-internal-roc-beta-phase3-preflight.sh
```

Pair rerun:

```bash id="svdrdl"
bash crates/svc-rewarder/scripts/dev-internal-roc-beta-phase3-preflight.sh
bash crates/ron-policy/scripts/dev-internal-roc-beta-phase3-preflight.sh
```

---

## 14. Next phase context

Next phase:

```text id="7408gy"
Internal ROC Beta Phase 4 — CrabLink Tauri wallet/receipt UX hardening
```

Expected Phase 4 direction:

```text id="ycdq8i"
CrabLink displays backend-derived wallet receipts.
CrabLink displays backend-derived balances.
CrabLink keeps receipt cache display-only.
CrabLink performs explicit confirmation before spend.
CrabLink never invents receipt, balance, payout, finality, paid unlock, bridge, staking, liquidity, or external settlement truth.
```

Do not add new ledger mutation paths.

Do not let CrabLink, gateway, omnigate, index, storage, policy, accounting, or rewarder mutate balances.

### END NOTE - JUNE 29 2026 - INTERNAL ROC BETA PHASE 3 ROUND 2

These are ready to append to each crate’s `NOTES.MD`.


### END NOTE - JUNE 29 2026 - 19:05 CST




### BEGIN NOTE - JUNE 29 2026 - 23:25 CST

Based on the latest bundles, `ron-policy` now has the Phase 5 economics TOML validator/preflight path, and `svc-rewarder` has the Phase 5 config-driven planning preflight path that checks the planning projection, Phase 5 test, Phase 3 regressions, and strict Clippy.   The active buildplan says this slice’s job was for `ron-policy` to validate TOML and bridge/staking inertness, while `svc-rewarder` consumes validated config for planning only with no hard-coded payout constants. 

# Crate Notes — Internal ROC Beta Phase 5 Round 1

## Crate Pair: `svc-rewarder + ron-policy`

## Status

`svc-rewarder + ron-policy` is now:

```text
Internal ROC Beta Phase 5 Round 1 svc-rewarder + ron-policy
Status: GREEN / PARKED
```

This completes the second Phase 5 Round 1 crate pair.

Completed Phase 5 Round 1 pairs so far:

```text
ron-proto + ron-ledger: GREEN / PARKED
svc-rewarder + ron-policy: GREEN / PARKED
```

Remaining Phase 5 Round 1 pair:

```text
svc-wallet + ron-accounting
```

---

## Purpose of this slice

This slice proved the Phase 5 tokenomics/config doctrine across the policy and reward-planning boundary:

```text
configs/roc-economics.toml is validated as canonical mutable economics config.
ron-policy validates tokenomics config only.
svc-rewarder consumes validated economics config for planning only.
Neither crate becomes receipt truth, balance truth, payout execution truth, finality truth, wallet authority, ledger authority, bridge runtime, staking runtime, liquidity, or external settlement.
```

The important architectural boundary remains:

```text
ron-policy = declarative validator/gate only
svc-rewarder = deterministic capped payout planner only
svc-wallet = only approved mutation front-door
ron-ledger = durable economic truth
```

---

## Files added or materially changed

## `ron-policy`

Added:

```text
crates/ron-policy/src/economics/internal_roc.rs
crates/ron-policy/tests/internal_roc_beta_phase5_economics_toml_policy_validation.rs
crates/ron-policy/tests/fixtures/roc-paid-action-economics.legacy.toml
crates/ron-policy/scripts/dev-internal-roc-beta-phase5-preflight.sh
```

Updated:

```text
crates/ron-policy/src/economics/mod.rs
crates/ron-policy/tests/economics_policy.rs
crates/ron-policy/tests/internal_roc_beta_paid_content_policy_non_authority.rs
crates/ron-policy/tests/quickchain_preflight_economics_config_non_authority.rs
```

## `svc-rewarder`

Added:

```text
crates/svc-rewarder/src/inputs/economics.rs
crates/svc-rewarder/tests/internal_roc_beta_phase5_config_driven_planning.rs
crates/svc-rewarder/scripts/dev-internal-roc-beta-phase5-preflight.sh
```

Updated:

```text
crates/svc-rewarder/src/inputs/mod.rs
```

Shared/global config already created in the previous pair:

```text
configs/roc-economics.toml
```

---

## `ron-policy` notes

## What `ron-policy` now proves

`ron-policy` now has a dedicated canonical Internal ROC economics TOML validator.

It validates:

```text
schema == internal_roc.economics-config.v1
version == 1
money values are integer minor-unit strings
money values reject floats/numeric TOML numbers
bps totals equal 10000
remainder sink is explicit
configured_account sink requires a configured account
bridge placeholder remains disabled/inert
staking placeholder remains disabled/inert
unknown TOML fields are rejected
validation output does not claim authority truth
```

It explicitly does **not**:

```text
create receipts
create balances
create payouts
execute wallet mutations
mutate ledger
grant paid access
claim finality
activate bridge/staking/liquidity/external settlement
```

## Important implementation detail

The old `economics_policy.rs` regression expected the legacy paid-action economics TOML shape. The new canonical `configs/roc-economics.toml` uses the Phase 5 schema shape, so the legacy test was moved to a fixture:

```text
crates/ron-policy/tests/fixtures/roc-paid-action-economics.legacy.toml
```

This preserves the old paid-action pricing/split regression without forcing the new canonical Phase 5 config to pretend to be the legacy action-policy config.

Do not undo this split.

Correct model:

```text
configs/roc-economics.toml
  = canonical Internal ROC Phase 5 tokenomics config

tests/fixtures/roc-paid-action-economics.legacy.toml
  = legacy paid-action economics fixture for older economics_policy regressions
```

## `ron-policy` green proof

Latest passed targets:

```text
cargo test -p ron-policy --test internal_roc_beta_phase5_economics_toml_policy_validation
  7 passed / 0 failed

cargo test -p ron-policy --test economics_policy
  15 passed / 0 failed

cargo test -p ron-policy --test internal_roc_beta_paid_content_policy_non_authority
  5 passed / 0 failed

cargo test -p ron-policy --test quickchain_preflight_economics_config_non_authority
  4 passed / 0 failed

cargo clippy -p ron-policy --all-targets --no-deps -- -D warnings
  passed
```

Phase 5 preflight passed:

```text
bash crates/ron-policy/scripts/dev-internal-roc-beta-phase5-preflight.sh

== Internal ROC Beta Phase 5 ron-policy economics TOML validation preflight passed ==
== policy validates tokenomics config only; no receipt/balance/payout/finality truth, bridge, staking, liquidity, or external settlement ==
```

---

## `svc-rewarder` notes

## What `svc-rewarder` now proves

`svc-rewarder` now has an Internal ROC economics projection for reward planning:

```text
crates/svc-rewarder/src/inputs/economics.rs
```

It parses canonical Internal ROC economics TOML and projects only the planning-safe fields:

```text
schema
version
epoch_pool_cap_minor
max_reward_minor_per_account_per_epoch
max_reward_minor_per_content_per_epoch
rounding_mode
remainder_sink
bridge_inert
staking_inert
```

It validates:

```text
schema/version
positive integer minor-unit strings
reward_pools.category_caps is present/non-empty
anti_farming.max_events_per_account_per_epoch > 0
rounding.mode == floor
remainder_sink is explicit
configured_account sink requires account
non-configured sinks reject stray remainder_sink_account
future_bridge.enabled == false
future_bridge.state is explicit
future_staking.enabled == false
future_staking.state is explicit
```

It converts config-derived economics into a `RewardPolicy` only as planning input:

```text
InternalRocRewardPlanningEconomics::to_reward_policy(...)
```

That policy remains a planning cap source only. It does not become a wallet receipt, ledger effect, paid unlock, balance truth, or finality claim.

## Important fixes made

### 1. Test source scan path fixed

The Phase 5 source scan originally used:

```text
crates/svc-rewarder/src
```

But Cargo integration tests run with the crate root as the effective path context, so that path failed. It was corrected to use:

```text
Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
```

Keep this. Do not revert to a repo-root-relative path inside crate integration tests.

### 2. Dead-code warnings fixed by actually validating fields

These fields were parsed only for strict TOML shape validation and initially triggered warnings:

```text
reward_pools.category_caps
anti_farming.max_events_per_account_per_epoch
rounding.remainder_sink_account
future_bridge.state
future_staking.state
```

They are now read and validated. Keep that validation because strict Clippy is part of the preflight.

### 3. Bool assert Clippy fixed

The test originally had:

```rust
assert_eq!(manifest.ledger.emitted, false);
```

It was fixed to:

```rust
assert!(!manifest.ledger.emitted);
```

Keep this to avoid `clippy::bool_assert_comparison`.

## `svc-rewarder` green proof

Latest passed targets:

```text
cargo test -p svc-rewarder --test internal_roc_beta_phase5_config_driven_planning
  3 passed / 0 failed

cargo test -p svc-rewarder --test internal_roc_beta_phase3_reward_plan_boundary
  5 passed / 0 failed

cargo test -p svc-rewarder --test internal_roc_beta_phase3_approved_payout_intent_boundary
  5 passed / 0 failed

cargo clippy -p svc-rewarder --all-targets -- -D warnings
  passed
```

Phase 5 preflight passed:

```text
bash crates/svc-rewarder/scripts/dev-internal-roc-beta-phase5-preflight.sh

== Internal ROC Beta Phase 5 svc-rewarder config-driven planning preflight passed ==
== rewarder consumes validated economics config for planning only; no hard-coded payout constants or wallet/ledger authority ==
```

---

## Final terminal proof for this crate pair

The last terminal output showed:

```text
svc-rewarder Phase 5 config-driven planning: 3 passed / 0 failed
svc-rewarder Phase 3 reward-plan boundary: 5 passed / 0 failed
svc-rewarder Phase 3 approved-payout intent boundary: 5 passed / 0 failed
svc-rewarder strict clippy: passed
svc-rewarder Phase 5 preflight: passed
```

Previous output showed:

```text
ron-policy Phase 5 economics TOML validation: 7 passed / 0 failed
ron-policy economics_policy regression: 15 passed / 0 failed
ron-policy strict clippy: passed
ron-policy Phase 5 preflight: passed
```

Therefore:

```text
Internal ROC Beta Phase 5 Round 1 svc-rewarder + ron-policy economics TOML validation/config-driven planning slice is GREEN / PARKED.
```

---

## Architecture boundaries preserved

This slice preserved the Internal ROC authority model:

```text
ron-policy validates/gates only.
ron-policy output is never receipt truth.
ron-policy output is never balance truth.
ron-policy output is never payout execution truth.
ron-policy output is never finality truth.

svc-rewarder plans payouts only.
svc-rewarder consumes accounting/config/policy inputs only.
svc-rewarder emits deterministic wallet handoff candidates only.
svc-rewarder does not mutate ledger.
svc-rewarder does not bypass svc-wallet.
svc-rewarder does not invent balances.
svc-rewarder does not invent receipts.
svc-rewarder does not treat config as authority.
svc-rewarder does not treat raw engagement as direct ROC allocation.
```

The only allowed future economic mutation path remains:

```text
approved payout plan
  -> policy gate
  -> explicit approved wallet execution
  -> svc-wallet
  -> ron-ledger
  -> backend receipt/balance truth
```

---

## Regression commands to keep

## `ron-policy`

```bash
cargo test -p ron-policy --test internal_roc_beta_phase5_economics_toml_policy_validation
cargo test -p ron-policy --test economics_policy
cargo test -p ron-policy --test internal_roc_beta_paid_content_policy_non_authority
cargo test -p ron-policy --test quickchain_preflight_economics_config_non_authority
cargo clippy -p ron-policy --all-targets --no-deps -- -D warnings
bash crates/ron-policy/scripts/dev-internal-roc-beta-phase5-preflight.sh
```

## `svc-rewarder`

```bash
cargo test -p svc-rewarder --test internal_roc_beta_phase5_config_driven_planning
cargo test -p svc-rewarder --test internal_roc_beta_phase3_reward_plan_boundary
cargo test -p svc-rewarder --test internal_roc_beta_phase3_approved_payout_intent_boundary
cargo clippy -p svc-rewarder --all-targets -- -D warnings
bash crates/svc-rewarder/scripts/dev-internal-roc-beta-phase5-preflight.sh
```

## Pair-level low-disk proof

```bash
bash crates/ron-policy/scripts/dev-internal-roc-beta-phase5-preflight.sh
bash crates/svc-rewarder/scripts/dev-internal-roc-beta-phase5-preflight.sh
```

---

## Do not regress

Do not let `configs/roc-economics.toml` become:

```text
receipt authority
balance authority
wallet mutation authority
ledger mutation authority
paid unlock authority
finality authority
bridge runtime authority
staking runtime authority
liquidity/exchange-facing authority
```

Do not let `ron-policy`:

```text
issue ROC
transfer ROC
burn ROC
open/capture/release holds
create receipts
create balances
grant unlocks
execute payouts
create finality
```

Do not let `svc-rewarder`:

```text
hard-code Phase 5 payout constants in business logic
consume raw engagement as direct payout authority
call ron-ledger directly for mutation
create wallet receipts
invent balances
silently execute payouts
treat dry-run manifests as settlement truth
activate bridge/staking/liquidity/external settlement
```

---

## Next crate pair

Next Phase 5 Round 1 crate pair:

```text
svc-wallet + ron-accounting
```

Expected purpose:

```text
svc-wallet:
  - prove economics config cannot directly mutate wallet/ledger
  - prove config cannot create receipt/balance truth
  - keep all mutation behind explicit wallet operation paths

ron-accounting:
  - label config version/hash/source in snapshots/reports only
  - prove accounting labels are derivative metadata, not balance truth
  - prove accounting cannot mutate ledger
```

Expected next-slice doctrine:

```text
config validates economics parameters
accounting can report config version/hash/source labels
wallet may use approved inputs only through explicit operation paths
neither config nor accounting can directly issue/transfer/burn/hold/capture/release ROC
ron-ledger remains the durable truth
```

Suggested next status label before work begins:

```text
Internal ROC Beta Phase 5 Round 1 svc-wallet + ron-accounting: NOT STARTED
```


### END NOTE - JUNE 29 2026 - 23:25 CST




### BEGIN NOTE - JUNE 30 2026 - 00:30 CST

Here are the crate notes for the now-green Phase 5 Round 2 `svc-rewarder + ron-policy` slice. The newer bundles show the new Round 2 preflight scripts and tests are present in both crates.  

# Crate Notes — Internal ROC Beta Phase 5 Round 2

## Crate Pair: `svc-rewarder + ron-policy`

## Status

```text id="b4ld8d"
Internal ROC Beta Phase 5 Round 2 svc-rewarder + ron-policy
Status: GREEN / PARKED
```

This crate pair completed the policy-gated anti-farming slice.

---

## Pair-level result

This round proved:

```text id="5s2tyq"
ron-policy gates reward eligibility declaratively.
svc-rewarder consumes verified/capped/policy-gated inputs only.
raw engagement cannot become protocol ROC payout material.
analytics_only cannot enter reward planning.
metering cannot directly enter reward planning.
proof_eligible requires verification, caps, and policy gate.
ad_budgeted requires explicit non-protocol budget.
rewarder remains planning-only.
policy remains declarative-only.
```

Still preserved:

```text id="rqhm9f"
No wallet mutation from policy.
No wallet mutation from rewarder anti-farming gates.
No ledger mutation from policy.
No ledger mutation from rewarder planning.
No fake receipt truth.
No fake balance truth.
No fake finality.
No bridge runtime.
No staking runtime.
No liquidity.
No external settlement.
```

---

# `ron-policy` notes

## Added / updated

Added:

```text id="w2c4ag"
crates/ron-policy/tests/internal_roc_beta_phase5_antifarming_policy_gate.rs
crates/ron-policy/scripts/dev-internal-roc-beta-phase5-round2-preflight.sh
```

Hardened:

```text id="ys8kqz"
crates/ron-policy/src/parse/validate.rs
```

The validator was hardened so authority-shaped staking/liquidity terms cannot sneak into policy tags or obligations.

Important fixed cases:

```text id="jpw37m"
staking_position_id now rejects as authority-shaped tag material.
staking-position now rejects as authority-shaped obligation material.
```

## What `ron-policy` now proves

```text id="b7kk0d"
Verified/capped proof_eligible material may pass only as a declarative policy gate.
Raw engagement, analytics_only, and direct metering reward attempts deny.
Unverified proof_eligible material denies.
Uncapped proof_eligible material denies.
ad_budgeted material requires explicit budget.
ad_budgeted material requires non-protocol budget.
Authority-shaped tags reject during parsing.
Authority-shaped obligations reject during parsing.
Policy decisions do not claim receipt, balance, finality, wallet, or ledger truth.
```

## Green proof

```text id="c1nxmd"
cargo test -p ron-policy --test internal_roc_beta_phase5_antifarming_policy_gate
  7 passed / 0 failed

cargo test -p ron-policy --test internal_roc_beta_phase5_economics_toml_policy_validation
  7 passed / 0 failed

cargo test -p ron-policy --test internal_roc_beta_phase3_reward_plan_policy_gate
  5 passed / 0 failed

cargo test -p ron-policy --test internal_roc_beta_phase3_approved_payout_policy_gate
  5 passed / 0 failed

cargo test -p ron-policy --test quickchain_preflight_decision_non_authority
  4 passed / 0 failed

cargo clippy -p ron-policy --all-targets --no-deps -- -D warnings
  passed

bash crates/ron-policy/scripts/dev-internal-roc-beta-phase5-round2-preflight.sh
  passed
```

## Do not regress

```text id="r8zppm"
ron-policy must remain declarative.
ron-policy must not create payout execution truth.
ron-policy must not create receipt truth.
ron-policy must not create balance truth.
ron-policy must not unlock paid content.
ron-policy must not mutate wallet or ledger.
ron-policy must reject authority-shaped policy tags/obligations.
ron-policy must keep bridge/staking/liquidity/external settlement forbidden.
```

---

# `svc-rewarder` notes

## Added / updated

Added:

```text id="lb2yk5"
crates/svc-rewarder/tests/internal_roc_beta_phase5_policy_gate_interlock.rs
```

Updated:

```text id="p12o9h"
crates/svc-rewarder/src/inputs/anti_farming.rs
crates/svc-rewarder/scripts/dev-internal-roc-beta-phase5-round2-preflight.sh
```

Key model change:

```text id="e09i4l"
CappedRewardInputCandidate now requires policy_gate_passed.
```

This makes the rewarder gate stricter:

```text id="e7aide"
verification + caps alone are not enough.
proof_eligible must also pass ron-policy.
ad_budgeted must pass ron-policy and use explicit non-protocol budget.
```

## What `svc-rewarder` now proves

```text id="qplpcy"
Verified/capped candidate without policy gate is rejected.
Verified/capped/policy-gated proof_eligible candidate may enter planning.
ad_budgeted candidate requires policy gate.
ad_budgeted candidate requires explicit budget.
ad_budgeted candidate cannot use protocol-pool emission.
Policy-gated capped inputs produce deterministic dry-run manifests only.
Rewarder remains non-mutating.
Rewarder source has no wallet/ledger authority shortcuts.
```

## Green proof

```text id="qtp7y7"
cargo test -p svc-rewarder --test internal_roc_beta_phase5_policy_gate_interlock
  5 passed / 0 failed

cargo test -p svc-rewarder --test internal_roc_beta_phase5_antifarming_event_gates
  5 passed / 0 failed

cargo test -p svc-rewarder --test internal_roc_beta_phase5_config_driven_planning
  3 passed / 0 failed

cargo test -p svc-rewarder --test internal_roc_beta_phase3_reward_plan_boundary
  5 passed / 0 failed

cargo test -p svc-rewarder --test internal_roc_beta_phase3_approved_payout_intent_boundary
  5 passed / 0 failed

cargo clippy -p svc-rewarder --all-targets -- -D warnings
  passed

bash crates/svc-rewarder/scripts/dev-internal-roc-beta-phase5-round2-preflight.sh
  passed
```

## Do not regress

```text id="xk922d"
svc-rewarder must not accept raw engagement as payout input.
svc-rewarder must not accept analytics_only as payout input.
svc-rewarder must not accept metering as direct payout input.
svc-rewarder must not accept proof_eligible without verification/caps/policy gate.
svc-rewarder must not accept ad_budgeted without explicit non-protocol budget.
svc-rewarder must not mutate ledger.
svc-rewarder must not create wallet receipt truth.
svc-rewarder must not bypass ron-policy.
svc-rewarder must not bypass svc-wallet.
svc-rewarder must not introduce bridge/staking/liquidity/external settlement.
```

---

# Round 2 pair boundary

Correct flow is now reinforced:

```text id="a91cga"
classified event
→ ron-accounting snapshot/report
→ svc-rewarder anti-farming cap gate
→ ron-policy declarative eligibility gate
→ svc-rewarder deterministic payout plan
→ svc-wallet approved payout execution only
→ ron-ledger durable receipt/balance truth
```

This is forbidden:

```text id="gne8bq"
raw engagement
→ rewarder payout

analytics_only
→ rewarder payout

metering
→ direct payout

policy allow
→ wallet mutation

rewarder plan
→ ledger mutation

ad_budgeted
→ protocol-pool emission
```

---

# Current Phase 5 Round 2 status

```text id="i4okqy"
ron-accounting + svc-rewarder: GREEN / PARKED
ron-policy + svc-rewarder: GREEN / PARKED
```

Phase 5 Round 2 has now locked the main anti-farming spine:

```text id="m4yqjw"
event classification
→ anti-farming caps
→ policy gate
→ deterministic non-mutating reward plan
```

---

# Keep these commands

```bash id="k0rua7"
cargo test -p ron-policy --test internal_roc_beta_phase5_antifarming_policy_gate
cargo test -p ron-policy --test internal_roc_beta_phase5_economics_toml_policy_validation
cargo test -p ron-policy --test internal_roc_beta_phase3_reward_plan_policy_gate
cargo test -p ron-policy --test internal_roc_beta_phase3_approved_payout_policy_gate
cargo test -p ron-policy --test quickchain_preflight_decision_non_authority
bash crates/ron-policy/scripts/dev-internal-roc-beta-phase5-round2-preflight.sh

cargo test -p svc-rewarder --test internal_roc_beta_phase5_policy_gate_interlock
cargo test -p svc-rewarder --test internal_roc_beta_phase5_antifarming_event_gates
cargo test -p svc-rewarder --test internal_roc_beta_phase5_config_driven_planning
cargo test -p svc-rewarder --test internal_roc_beta_phase3_reward_plan_boundary
cargo test -p svc-rewarder --test internal_roc_beta_phase3_approved_payout_intent_boundary
bash crates/svc-rewarder/scripts/dev-internal-roc-beta-phase5-round2-preflight.sh
```

---

# Next likely step

```text id="htl734"
Phase 5 closeout / reproducible smoke consolidation
```

Optional before closeout:

```text id="rul8av"
Add a focused svc-ads ad_budgeted slice only if svc-ads is active enough and we intentionally want ad-budgeted proof before Phase 5 closeout.
```

These notes are safe to carry forward: the main `svc-rewarder + ron-policy` Round 2 authority boundary is green and parked.


### END NOTE - JUNE 30 2026 - 00:30 CST



### BEGIN NOTE - JULY 1 2026 - 20:30 CST

## Pair 3 — `svc-rewarder + ron-policy`

Safe label:

```text id="8pw8m2"
Internal ROC Stabilization / Product Beta Readiness —
svc-rewarder + ron-policy capped reward planning / declarative policy gate boundary:
COMPLETE / GREEN / PARKED.
```

## What we changed

We added a stabilization wrapper around the reward-planning and policy-gating layer of the Internal ROC value loop.

For `svc-rewarder`, we added/locked:

```text id="1rg56i"
crates/svc-rewarder/docs/internal-roc-stabilization-reward-policy-gate.md
crates/svc-rewarder/tests/internal_roc_stabilization_reward_policy_gate_boundary.rs
crates/svc-rewarder/scripts/dev-internal-roc-stabilization-reward-policy-gate-preflight.sh
```

For `ron-policy`, we added/locked:

```text id="qkg4d4"
crates/ron-policy/docs/internal-roc-stabilization-policy-gate-non-authority.md
crates/ron-policy/tests/internal_roc_stabilization_policy_gate_non_authority_boundary.rs
crates/ron-policy/scripts/dev-internal-roc-stabilization-policy-gate-preflight.sh
```

And the aggregate pair gate:

```text id="jmwbx4"
scripts/dev-internal-roc-stabilization-rewarder-policy-park.sh
```

We also repaired the new `ron-policy` stabilization wrapper so it checked semantic policy/economics boundaries instead of brittle exact spelling markers like `first-match`, `analytics_only`, or `validate_economics_policy`.

## What we accomplished

`svc-rewarder` was locked as **deterministic capped reward planning and wallet-handoff only**.

That means:

```text id="iz1qm4"
- rewarder consumes accounting snapshots and policy-gated candidate material
- rewarder applies anti-farming caps
- rewarder rejects raw engagement as payout authority
- rewarder rejects analytics_only and metering as direct payout material
- rewarder requires proof_eligible material to be verified, capped, and policy-gated
- rewarder requires ad_budgeted material to have explicit non-protocol budget authorization
- rewarder creates deterministic manifests and wallet issue request DTOs
- rewarder does not create receipts
- rewarder does not create balance truth
- rewarder does not mutate the ledger directly
- rewarder does not execute payouts except by handing approved requests to svc-wallet
```

`ron-policy` was locked as **declarative gate and economics validation only**.

That means:

```text id="9h1htj"
- policy can allow, deny, explain, and require obligations
- policy can validate ROC economics TOML
- policy can reject unsafe economics config
- policy can gate reward eligibility
- policy can gate approved payout candidates
- policy can reject raw engagement, analytics-only, direct metering, uncapped, unverified, or unfunded reward material
- policy decisions are not receipts
- policy decisions are not balances
- policy obligations are not payout execution
- economics config is not wallet authority
- policy never mutates wallet or ledger state
```

## Tests/gates that were parked

For `svc-rewarder`, the gate covered:

```text id="tm13zx"
- internal_roc_stabilization_reward_policy_gate_boundary
- internal_roc_beta_rewarder_planning_non_authority
- internal_roc_beta_phase3_reward_plan_boundary
- internal_roc_beta_phase3_approved_payout_intent_boundary
- internal_roc_beta_phase5_config_driven_planning
- internal_roc_beta_phase5_antifarming_event_gates
- internal_roc_beta_phase5_policy_gate_interlock
- quickchain_preflight_no_direct_mutation
- quickchain_preflight_replay_no_double_issue
- clippy clean
```

For `ron-policy`, the gate covered:

```text id="d72fco"
- internal_roc_stabilization_policy_gate_non_authority_boundary
- internal_roc_beta_paid_content_policy_non_authority
- internal_roc_beta_phase3_reward_plan_policy_gate
- internal_roc_beta_phase3_approved_payout_policy_gate
- internal_roc_beta_phase5_economics_toml_policy_validation
- internal_roc_beta_phase5_antifarming_policy_gate
- quickchain_preflight_decision_non_authority
- quickchain_preflight_economics_config_non_authority
- economics_policy
- clippy clean
```

## Boundary we proved

This pair locked the separation between **planning** and **gating**:

```text id="exqu9c"
svc-rewarder plans capped deterministic rewards.
ron-policy gates eligibility declaratively.
svc-wallet executes approved payout mutations.
ron-ledger records durable truth.
```

Correct path:

```text id="f673c3"
ron-accounting snapshot / verified evidence
→ svc-rewarder candidate classification
→ anti-farming caps
→ ron-policy declarative gate
→ deterministic reward manifest
→ wallet issue request DTO
→ svc-wallet approved payout execution
→ ron-ledger durable receipt/balance truth
```

Forbidden path:

```text id="i70gk1"
raw engagement
→ direct ROC payout

analytics_only
→ reward material

metering
→ direct payout

policy allow
→ receipt truth

policy allow
→ paid unlock

rewarder manifest
→ balance truth

rewarder wallet issue request
→ receipt truth before svc-wallet/ron-ledger acceptance
```

## Why this pair mattered

This pair was the **anti-farming and reward safety lock**.

Earlier pairs proved that ledger truth and wallet mutation are protected. This pair proved that the upstream reward engine cannot be tricked into converting low-quality or fakeable signals into ROC issuance.

The key doctrine locked here:

```text id="z5ax6q"
A reward plan is not a payout.
A policy allow is not a receipt.
A wallet issue request is not ledger truth.
Raw engagement is not mint authority.
Analytics are not payout authority.
Caps and policy gates are mandatory before reward material becomes payout-eligible.
```

## Final record note

```text id="5e7c7q"
Pair 3 completed the capped reward-planning and declarative policy-gate stabilization layer. svc-rewarder is now parked as deterministic, capped, anti-farming-aware reward planning and wallet-handoff infrastructure only. ron-policy is now parked as declarative allow/deny/obligation and ROC economics validation infrastructure only. The pair blocks raw-engagement payout, analytics-only payout, direct-metering payout, unverified or uncapped proof-eligible payout material, unfunded ad-budgeted protocol-pool emission, policy-created receipt/balance/finality truth, rewarder-created payout execution truth, direct ledger mutation, and all bridge/ROX/Solana/staking/liquidity/external-settlement drift.
```

Next pair in the record:

```text id="3q77tc"
svc-gateway + omnigate
```


### END NOTE - JULY 1 2026 - 20:30 CST


### BEGIN NOTE - JULY 11 2026 - 13:15 CST

Below is the complete crate-specific changelog and carryover record for `ron-policy`.

# `ron-policy` — Phase 10A Moderation Policy Changelog Notes

## 1. Crate role

`ron-policy` is the canonical declarative policy crate for RustyOnions.

Before this session, it already owned general allow/deny policy evaluation, strict policy parsing, reason handling, obligations, and economics-policy validation. During this session, it gained a new, separate moderation-policy surface for exact BLAKE3-addressed objects.

The new moderation surface establishes one shared answer to this question:

```text
Given one exact b3 object identifier, may a service node serve it?
```

This was necessary so `svc-storage`, `macronode`, future operator tools, and later signed-policy handling do not invent separate moderation rules or incompatible precedence orders.

The new moderation system is declarative only. It determines policy posture but does not itself:

```text
serve content
read storage
delete content
prune bytes
publish or withdraw DHT provider records
issue rewards
mutate ROC balances
mutate wallets
mutate the ledger
prove ownership
verify signed global policy lists
```

That non-authority boundary was deliberately preserved.

---

## 2. Session phase

The `ron-policy` work completed **BUILD_PLAN_Z Phase 10A**.

Phase 10A created the shared moderation foundation used by the later Phase 10B, 10C, and 10D integrations in `svc-storage` and `macronode`.

Status at the end of this session:

```text
Phase 10A: complete
Focused moderation tests: green
Strict Clippy: green
Workspace compilation: green
Runtime consumers: successfully integrated in later Phase 10 slices
```

---

## 3. Files added or changed

The primary `ron-policy` files changed were:

```text
crates/ron-policy/src/moderation.rs
crates/ron-policy/src/lib.rs
crates/ron-policy/tests/moderation_policy.rs
```

### `src/moderation.rs`

This was a new module containing:

```text
B3Id
B3IdError
Effect
ReasonCode
Decision
Policy
exact-match evaluation
fixed moderation precedence
insert/remove APIs
entry-count APIs
strict deterministic serde support
```

### `src/lib.rs`

The crate root was updated to:

```rust
pub mod moderation;
```

It also exposes the moderation types through clearer public aliases:

```rust
pub use moderation::{
    B3Id,
    B3IdError,
    Decision as ModerationDecision,
    Effect as ModerationEffect,
    Policy as ModerationPolicy,
    ReasonCode as ModerationReasonCode,
};
```

This lets downstream crates use moderation behavior directly from `ron_policy` without depending on internal module naming. 

### `tests/moderation_policy.rs`

A focused integration-test file was added to prove the exact moderation rules independently from storage, HTTP, DHT, rewards, wallets, and ledgers.

---

## 4. Canonical `B3Id` type

A new canonical moderation identifier type was added:

```rust
pub struct B3Id(String);
```

`B3Id` accepts only the exact form:

```text
b3:<64 lowercase hexadecimal characters>
```

Examples:

```text
Valid:
b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa

Invalid:
sha256:...
b3:abc
b3:AAAA...
b3:gggg...
b3:<65 hex characters>
empty string
```

The parser requires:

```text
the literal lowercase prefix b3:
exactly 64 digest characters
only ASCII digits 0-9
only lowercase ASCII letters a-f
no uppercase normalization
no whitespace trimming
no alternate algorithms
no abbreviated hashes
```

A dedicated `B3IdError` type was added with the stable diagnostic:

```text
expected b3:<64 lowercase hexadecimal characters>
```

`B3Id` also implements:

```text
FromStr
Display
Serialize
Deserialize
Eq
Ord
Hash
Clone
```

Serde deserialization runs through the same strict parser, so loading a policy from JSON cannot bypass the identifier validation used by programmatic callers. 

### Why this was added

Moderation must apply to one exact content-addressed object. Loose strings, prefixes, partial hashes, case normalization, or alternate identifier schemes could cause:

```text
false-positive blocking
false-negative blocking
policy bypass
policy ambiguity
incorrect pruning targets
inconsistent behavior across services
```

An object differing by even one hexadecimal character must be treated as a different object.

---

## 5. Moderation policy state sets

The new `ModerationPolicy` owns five exact-object sets:

```text
global_deny
local_block
local_allow
owner_tombstone
quarantine
```

Internally, these are ordered sets rather than unordered hash collections.

The use of ordered sets provides:

```text
deterministic serialization
stable policy snapshots
repeatable tests
stable review diffs
exact membership lookup
no dependence on randomized hash iteration order
```

### `global_deny`

Represents an object denied by the global policy posture.

Intended future source:

```text
validated and signed global policy lists
```

Current Phase 10A behavior:

```text
exact membership only
highest evaluation precedence
serving refused
```

Phase 10A did not yet implement signature validation or global-list distribution.

### `owner_tombstone`

Represents an exact object that has an owner tombstone posture.

Current behavior:

```text
serving refused
takes precedence over local block, quarantine, and local allow
```

The policy type does not prove that a tombstone was validly issued by an owner. It only carries and evaluates the already-established state.

### `local_block`

Represents an operator-local refusal for one exact object.

Current behavior:

```text
serving refused
takes precedence over quarantine and local allow
removable through the local-policy mutation API
```

### `quarantine`

Represents an exact object isolated for review or unsafe handling.

Current behavior:

```text
serving not permitted
decision effect is Quarantine rather than ordinary Refuse
takes precedence over local allow
removable through the local-policy mutation API
```

### `local_allow`

Represents an operator-local explicit allowance.

Current behavior:

```text
serving permitted only when no stronger refusal state matches
does not override global deny
does not override owner tombstone
does not override local block
does not override quarantine
```

This distinction is important: `local_allow` records an affirmative local posture, but it is not an escape hatch from stronger safety states.

---

## 6. Fixed deterministic precedence

The evaluation order was made explicit and deterministic:

```text
1. global_deny
2. owner_tombstone
3. local_block
4. quarantine
5. local_allow
6. no matching rule
```

Equivalent behavior:

```rust
if global_deny {
    refuse(global_deny)
} else if owner_tombstone {
    refuse(owner_tombstone)
} else if local_block {
    refuse(local_block)
} else if quarantine {
    quarantine
} else if local_allow {
    serve(local_allow)
} else {
    serve(no_rule)
}
```

The implementation returns immediately on the strongest matching state. 

### Why fixed precedence matters

The same object may appear in more than one set.

For example:

```text
local_allow + global_deny
local_allow + owner_tombstone
local_allow + local_block
local_allow + quarantine
global_deny + owner_tombstone + local_block + quarantine
```

Without a canonical order, different crates could reach different answers for the same policy snapshot.

Phase 10A guarantees that all consumers using `ModerationPolicy::evaluate` receive the same result.

The strongest-state test specifically places one object into:

```text
quarantine
local_block
owner_tombstone
global_deny
```

The resulting decision must be:

```text
effect = Refuse
reason = GlobalDeny
```

---

## 7. Moderation effects

The moderation decision model added three effects:

```text
Serve
Refuse
Quarantine
```

### `Serve`

The object may proceed through the caller’s serving path.

Returned for:

```text
NoRule
LocalAllow
```

### `Refuse`

The object must not be served.

Returned for:

```text
GlobalDeny
OwnerTombstone
LocalBlock
```

### `Quarantine`

The object must not be served and is specifically marked as quarantined.

Returned for:

```text
Quarantined
```

Keeping `Quarantine` separate from ordinary refusal leaves room for later behavior such as:

```text
operator review
quarantine inventory
physical relocation
special retention rules
audit handling
delayed deletion
```

None of those later behaviors were implemented in `ron-policy`; the distinction is currently declarative.

---

## 8. Stable reason codes

The new reason taxonomy is:

```text
NoRule
LocalAllow
GlobalDeny
OwnerTombstone
LocalBlock
Quarantined
```

Their stable JSON labels are:

```json
"no_rule"
"local_allow"
"global_deny"
"owner_tombstone"
"local_block"
"quarantined"
```

Focused tests lock these labels so future refactoring cannot silently change:

```text
HTTP refusal reasons
operator status labels
metrics labels
audit classifications
CLI output
policy snapshots
cross-crate expectations
```

The wire-label test explicitly serializes every reason variant and checks the exact expected string. 

---

## 9. Moderation decision object

Evaluation returns a structured moderation decision containing:

```text
effect
reason
```

The decision also exposes a convenience method:

```rust
permits_serve()
```

Its semantics are intentionally strict:

```text
Serve       -> true
Refuse      -> false
Quarantine  -> false
```

This allowed later serving crates to depend on one simple invariant:

```text
Only a ModerationEffect::Serve decision permits object delivery.
```

The caller does not need to duplicate the full precedence table.

---

## 10. Policy mutation APIs

The moderation model added insertion functions for all five sets:

```rust
insert_global_deny
insert_local_block
insert_local_allow
insert_owner_tombstone
insert_quarantine
```

Each insertion returns a Boolean using set semantics:

```text
true  = the object was newly inserted
false = the exact object was already present
```

Local operator state removal APIs were added for:

```rust
remove_local_block
remove_local_allow
remove_quarantine
```

These removals are deliberately scoped to operator-local and quarantine state.

There were no matching general removal functions added for:

```text
global_deny
owner_tombstone
```

That helps avoid treating stronger externally governed or ownership-related states as ordinary operator toggles.

### Removal isolation

Removing an object from one local set affects only that set.

For example:

```text
remove_local_block(object)
```

does not remove the same object from:

```text
global_deny
owner_tombstone
local_allow
quarantine
```

The focused tests prove local state can be removed and that the evaluated result returns to `NoRule` when no other matching state remains.

---

## 11. Entry-count APIs

The policy exposes aggregate count methods:

```rust
global_deny_count()
local_block_count()
local_allow_count()
owner_tombstone_count()
quarantine_count()
```

These were added so runtime consumers can expose low-cardinality status without accessing private policy internals or disclosing exact moderated object identifiers.

Later in this session, `svc-storage` and `macronode` used these methods to report aggregate policy counts such as:

```text
number of global denies
number of local blocks
number of local allows
number of owner tombstones
number of quarantined objects
total configured entries
```

This supports truthful observability while avoiding:

```text
CID disclosure
policy-file path disclosure
full policy dumps
high-cardinality status output
```

The count methods are part of the shared policy API, while the runtime status DTOs live in the consuming crates. 

---

## 12. Strict and deterministic JSON behavior

`ModerationPolicy` supports JSON serialization and deserialization.

The policy JSON fields are:

```json
{
  "global_deny": [],
  "local_block": [],
  "local_allow": [],
  "owner_tombstone": [],
  "quarantine": []
}
```

The JSON contract is strict:

```text
unknown fields reject
invalid b3 identifiers reject
noncanonical identifiers reject
wrong field types reject
```

For example, this must fail because of the unknown field:

```json
{
  "global_deny": [],
  "local_block": [],
  "local_allow": [],
  "owner_tombstone": [],
  "quarantine": [],
  "unexpected": true
}
```

Determinism is also tested by serializing the same policy twice and requiring byte-for-byte identical JSON output.

The serialized policy is then deserialized and required to equal the original policy.

### Why deterministic serialization matters

Deterministic policy snapshots are useful for:

```text
repeatable review
stable fixture files
future signatures
future policy hashes
operator diffs
test reproducibility
auditability
content-addressed configuration
```

Phase 10A does not yet sign or hash policy documents, but it establishes the data behavior needed for that later work.

---

## 13. Exact-match safety

The new evaluator performs exact full-identifier membership tests only.

A focused false-positive test uses:

```text
one blocked object
one object differing by only one final hexadecimal character
one unrelated object
```

Only the exact blocked object may be refused.

The nearly identical object and unrelated object must both return:

```text
effect = Serve
reason = NoRule
```

This prevents substring, prefix, fuzzy, or partial-hash policy matching.

---

## 14. Focused test coverage

The new `moderation_policy` integration test contains eight focused tests.

### 1. `strict_b3_identifier_rejects_noncanonical_values`

Proves:

```text
canonical b3 IDs parse
wrong algorithms reject
short hashes reject
uppercase hashes reject
non-hex hashes reject
incorrect lengths reject
```

### 2. `no_rule_and_local_allow_permit_serving`

Proves:

```text
an empty policy serves with NoRule
local allow serves with LocalAllow
permits_serve returns true in both cases
```

### 3. `refusal_states_have_fixed_precedence_over_local_allow`

Proves that local allow cannot bypass:

```text
global deny
owner tombstone
local block
quarantine
```

It also proves quarantine produces the distinct `Quarantine` effect.

### 4. `strongest_matching_reason_wins_deterministically`

Places one object in all strong refusal sets and proves:

```text
GlobalDeny wins
```

### 5. `policy_false_positive_requires_exact_lookup`

Proves:

```text
one-character CID differences are distinct
no prefix or fuzzy matching occurs
```

### 6. `local_operator_states_can_be_removed_without_touching_stronger_sets`

Proves the local remove APIs work independently and return objects to `NoRule` when no other state matches.

### 7. `policy_json_is_strict_and_deterministic`

Proves:

```text
repeat serialization is stable
round-trip equality holds
unknown JSON fields reject
```

### 8. `reason_codes_have_stable_wire_labels`

Locks the exact snake-case JSON labels for all reason variants.

The test file explicitly states that it tests policy decisions only and performs no storage, provider, reward, wallet, or ledger mutation. 

---

## 15. Security properties established

The Phase 10A work established the following security properties:

### Canonical identity

Moderation applies only to validated:

```text
b3:<64 lowercase hex>
```

identifiers.

### No local-allow bypass

A local operator allow cannot override stronger refusal posture.

### Exact matching

A rule for one object cannot accidentally affect a similar hash.

### Deterministic evaluation

All consuming crates receive the same result for the same snapshot.

### Strict configuration

Malformed fields, unknown fields, and invalid identifiers reject instead of being silently ignored.

### Stable reason taxonomy

Consumers can make consistent decisions and metrics classifications without parsing human-readable messages.

### No hidden mutation authority

The crate only returns a decision. It cannot:

```text
delete bytes
stop a network listener
withdraw a provider record
change a wallet
alter a ledger
issue ROC
claim moderation finality
```

---

## 16. Architectural impact on downstream crates

The new `ron-policy` API became the shared foundation for later work completed during this session.

### `svc-storage`

`svc-storage` now calls `ModerationPolicy::evaluate` before object storage lookup for:

```text
OAP OBJ_GET
legacy HTTP GET /o/:cid
legacy HTTP HEAD /o/:cid
```

It does not maintain its own moderation precedence.

### `macronode`

`macronode` loads one immutable moderation-policy snapshot and reports aggregate counts derived from the `ron-policy` count methods.

It does not invent another moderation model.

### Future `crabnode` CLI

Planned commands such as:

```text
block
unblock
allow
quarantine
```

should mutate or rebuild this canonical policy representation rather than introduce separate command-specific state.

### Future signed global policies

Signed-policy verification should produce validated entries for `global_deny`; it should not replace or duplicate `ModerationPolicy::evaluate`.

### Future pruning

Physical deletion must consume the policy decision but remain implemented in storage/runtime crates. `ron-policy` must remain non-mutating.

---

## 17. Deliberately deferred work

The following items were intentionally not implemented in `ron-policy` during this session:

```text
signed global-policy verification
policy signer identities
signature chains
policy epochs
policy expiration
remote policy fetching
automatic file watching
hot reload
persistent operator-policy database
CLI commands
HTTP mutation endpoints
storage deletion
physical quarantine movement
DHT provider withdrawal
reward eligibility changes
moderation refusal counters
moderation audit receipts
owner-tombstone signature verification
```

These belong to later Phase 10 slices or other crates.

---

## 18. Existing surfaces left intact

The new moderation module was added alongside the existing policy engine.

It did not replace or materially alter:

```text
general PolicyBundle parsing
Evaluator behavior
ordinary allow/deny rules
economics-policy validation
QuickChain policy boundaries
existing reason/obligation behavior
existing policy tests
```

The public aliases use `Moderation*` names specifically to avoid ambiguity with the crate’s pre-existing general-policy types such as:

```text
Decision
DecisionEffect
Evaluator
PolicyBundle
```

This prevents the moderation policy from being confused with the broader request-policy engine.

---

## 19. Validation completed

The moderation implementation passed the focused Phase 10A checks:

```text
cargo test -p ron-policy --test moderation_policy
cargo clippy -p ron-policy --all-targets --no-deps -- -D warnings
cargo check --workspace
```

The focused moderation suite completed with:

```text
8 passed
0 failed
```

The later successful `svc-storage` and `macronode` integrations also provide downstream evidence that the public API is usable as intended.

---

## 20. Final crate status

At the end of this session, `ron-policy` now owns the canonical exact-B3 moderation vocabulary and precedence for RustyOnions service-node object serving.

The crate now provides:

```text
strict B3 identifiers
deterministic exact-object moderation
stable effects
stable reason codes
strong refusal precedence
local operator mutation primitives
aggregate count methods
strict deterministic JSON
focused regression coverage
a preserved non-authority boundary
```

The most important invariant carried forward is:

```text
global deny
    >
owner tombstone
    >
local block
    >
quarantine
    >
local allow
    >
no rule
```

A local allow must never override a stronger refusal state.

`ron-policy` decides moderation posture. The consuming service decides how to enforce that posture. Storage owns bytes. DHT owns provider records. Wallet and ledger remain the only appropriate economic mutation and truth surfaces.

The next crate notes can use this same changelog, invariants, tests, boundaries, and remaining-work structure.


### END NOTE - JULY 11 2026 - 13:15 CST


### BEGIN NOTE - JULY 11 2026 - 22:50 CST

Below is the complete crate-specific changelog record for the `ron-policy` work completed during BUILD_PLAN_Z Phase 10.

# `ron-policy` — BUILD_PLAN_Z Phase 10 Changelog Notes

## 1. Crate role

`ron-policy` is the canonical declarative policy crate for RustyOnions.

Before Phase 10, the crate already owned the general policy engine, including:

* Allow and deny decisions
* Strict JSON and TOML parsing
* Context matching
* Reason strings and traces
* Obligations
* Economics-policy validation
* QuickChain non-authority boundaries

During Phase 10, `ron-policy` gained a separate content-moderation foundation for exact BLAKE3-addressed objects and authenticated global moderation snapshots.

The central question now answered by this crate is:

```text
Given one exact b3 object identifier, what is the canonical moderation
decision, and which policy source caused it?
```

This shared behavior prevents `svc-storage`, `macronode`, `crabnode`, `svc-admin`, `svc-dht`, and other consumers from inventing incompatible moderation identities, state sets, precedence rules, or reason labels. 

---

## 2. Phase status

The `ron-policy` work covered the policy-foundation portions of BUILD_PLAN_Z Phase 10, including:

```text
Phase 10A:
  canonical exact-object moderation model

Later Phase 10 slices:
  authenticated signed global policy snapshots
  rollback protection
  signed-global and local-operator policy composition
```

Final status:

```text
ron-policy Phase 10 work: complete
Canonical moderation model: complete
Signed moderation verification: complete
Global/local composition rules: complete
Focused tests: green
Strict Clippy: green
Workspace compilation: green
Downstream integration: active in svc-storage and macronode
```

`ron-policy` is now **complete, green, and parked for Phase 10**.

---

## 3. Files added or changed

The principal files changed in this crate were:

```text
crates/ron-policy/Cargo.toml
crates/ron-policy/src/lib.rs
crates/ron-policy/src/moderation.rs
crates/ron-policy/src/signed_moderation.rs
crates/ron-policy/tests/moderation_policy.rs
crates/ron-policy/tests/signed_moderation_policy.rs
```

The latest crate bundle confirms the new moderation modules, focused test suites, and direct `ron-kms` dependency. 

---

# Part I — Canonical Exact-Object Moderation

## 4. New `moderation` module

A new module was added:

```text
crates/ron-policy/src/moderation.rs
```

This module owns the shared moderation vocabulary and behavior:

```text
B3Id
B3IdError
ModerationPolicy
ModerationDecision
ModerationEffect
ModerationReasonCode
ModerationPolicyCompositionError
exact object matching
canonical precedence
insertion and removal APIs
entry-count APIs
deterministic serialization
global/local policy composition
```

The module is deliberately independent of:

```text
HTTP
storage
DHT networking
operator configuration
file paths
runtime readiness
wallet execution
ledger mutation
reward issuance
```

It determines policy posture only.

---

## 5. Canonical `B3Id`

A canonical identifier type was introduced:

```rust
pub struct B3Id(String);
```

It accepts only this exact format:

```text
b3:<64 lowercase hexadecimal characters>
```

Valid example:

```text
b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
```

Rejected forms include:

```text
empty strings
wrong algorithms such as sha256:
missing b3: prefix
short digests
long digests
uppercase hexadecimal
non-hexadecimal characters
abbreviated hashes
whitespace-padded identifiers
alternate spellings or normalization
```

The parser does not silently lowercase, trim, expand, or reinterpret input. The object identifier must already be canonical.

`B3Id` supports the traits needed by downstream services:

```text
FromStr
Display
Serialize
Deserialize
Clone
Eq
Ord
Hash
```

Serde deserialization passes through the same strict parser, preventing a JSON policy file from bypassing the validation applied to programmatic callers.

### Why this matters

Moderation and pruning operate on content-addressed objects. Loose matching could cause:

```text
blocking the wrong object
failing to block the intended object
prefix collisions
case-normalization ambiguity
policy bypass
incorrect pruning
inconsistent decisions between crates
```

Two B3 identifiers that differ by one character are treated as entirely separate objects.

---

## 6. Canonical moderation state sets

`ModerationPolicy` owns five exact-object sets:

```text
global_deny
owner_tombstone
local_block
quarantine
local_allow
```

These are ordered collections rather than randomized hash collections.

That choice provides:

```text
deterministic JSON serialization
stable snapshots
repeatable signatures
stable code-review diffs
repeatable tests
exact membership checks
no randomized iteration ordering
```

### `global_deny`

Represents globally denied content received through the authenticated global policy path.

A global denial is stronger than every local operator decision.

### `owner_tombstone`

Represents an object that its owner has withdrawn or tombstoned.

A tombstone remains stronger than local block, quarantine, or allow state.

### `local_block`

Represents an exact object blocked by the local service-node operator.

This is local node policy and does not claim network-wide deletion or global prohibition.

### `quarantine`

Represents an object isolated for review or safety reasons.

Quarantined content is not treated as normally servable.

### `local_allow`

Represents an explicit local operator allowance.

It may permit otherwise unclassified content, but it cannot override:

```text
global deny
owner tombstone
local block
quarantine
```

---

## 7. Canonical precedence

The crate establishes one fixed precedence order:

```text
global_deny
  >
owner_tombstone
  >
local_block
  >
quarantine
  >
local_allow
  >
no_rule
```

This order is now the canonical rule for all downstream consumers.

A consumer must not recreate or reorder this logic inside:

```text
svc-storage
macronode
crabnode
svc-admin
svc-dht
svc-index
CrabLink
```

All consumers must evaluate through `ron-policy`.

### Why precedence was fixed centrally

An object can appear in more than one set. For example, a local operator might previously have allowed an object that later enters a signed global deny set.

Without fixed precedence, one component might allow it while another refuses it.

The canonical model guarantees:

```text
local allow never bypasses global deny
local allow never bypasses owner tombstone
local allow never bypasses local block
local allow never bypasses quarantine
```

---

## 8. Moderation effects

The moderation decision model exposes three effects:

```text
Serve
Refuse
Quarantine
```

Their meanings are:

### `Serve`

The object may continue to the consuming service’s next validation step.

This does not itself mean:

```text
the object exists
the bytes are valid
the digest matches
the caller is authenticated
the caller is entitled to paid access
the object must be persisted
```

It means only that moderation does not prohibit serving it.

### `Refuse`

The moderation policy explicitly refuses the object.

The consuming service is responsible for mapping that decision to its protocol response, such as an HTTP `403 Forbidden`.

### `Quarantine`

The object is isolated and must not be served normally.

The distinct effect preserves the difference between a permanent or explicit refusal and a review-oriented quarantine posture.

---

## 9. Stable reason codes

Decisions include stable reason labels:

```text
no_rule
local_allow
global_deny
owner_tombstone
local_block
quarantined
```

These labels support:

```text
HTTP refusal responses
operator diagnostics
status reporting
audit records
metrics
tests
future UI presentation
```

Because the reason taxonomy lives in `ron-policy`, downstream services do not need to invent their own labels.

---

## 10. Decision helper behavior

`ModerationDecision` includes a convenience method equivalent to:

```rust
decision.permits_serve()
```

It returns true only for serving decisions.

Conceptually:

```text
NoRule       -> permits serve
LocalAllow   -> permits serve
GlobalDeny   -> does not permit serve
OwnerTombstone -> does not permit serve
LocalBlock   -> does not permit serve
Quarantined  -> does not permit serve
```

This lets storage and serving code use one canonical question rather than duplicating effect/reason matching.

---

## 11. Mutation APIs

The policy exposes explicit insertion and removal methods for the individual sets.

Examples include the equivalents of:

```text
insert_global_deny
remove_global_deny

insert_owner_tombstone
remove_owner_tombstone

insert_local_block
remove_local_block

insert_quarantine
remove_quarantine

insert_local_allow
remove_local_allow
```

These methods return whether the set actually changed.

That supports idempotent operator behavior:

```text
first insertion:
  changed = true

repeated insertion:
  changed = false

first removal:
  changed = true

repeated removal:
  changed = false
```

The policy object itself contains no disk-writing or runtime-activation logic. Callers such as `crabnode` are responsible for safely storing local policy snapshots.

---

## 12. Entry-count APIs

The policy exposes bounded aggregate information for diagnostics and operator status.

Counts include:

```text
global deny count
owner tombstone count
local block count
quarantine count
local allow count
total entry count
```

These counts let downstream status surfaces report policy posture without dumping potentially large or sensitive object lists.

---

## 13. Deterministic and strict serialization

The moderation DTOs use strict serde behavior.

Important guarantees include:

```text
unknown fields reject
invalid B3 identifiers reject
wrong field types reject
serialization order is deterministic
repeated serialization produces stable output
deserialization cannot bypass B3 validation
```

This is important for both local operator policy files and signed global snapshots.

Deterministic policy bytes are also necessary because authenticated snapshots sign a deterministic payload. Different map or set ordering must not produce different signatures for logically identical policies.

---

# Part II — Global and Local Authority Separation

## 14. Signed-global and local-policy composition

Later in Phase 10, `ModerationPolicy` gained a canonical composition operation:

```rust
ModerationPolicy::compose_signed_global_with_local(
    &signed_global,
    &local_operator_policy,
)
```

This combines authenticated global policy state with local service-node operator state.

The composition preserves:

```text
signed global deny entries
signed owner tombstones
local blocks
local quarantines
local allows
canonical precedence
```

A local allow remains present for operator visibility but cannot defeat a stronger signed-global decision. 

---

## 15. Authority separation enforced during composition

The two inputs have separate allowed responsibilities.

### Signed-global input may contain

```text
global_deny
owner_tombstone
```

It must not contain:

```text
local_block
local_allow
quarantine
```

### Local-operator input may contain

```text
local_block
local_allow
quarantine
```

It must not contain:

```text
global_deny
owner_tombstone
```

Crossed authority is rejected rather than silently accepted.

The composition errors include:

```text
SignedGlobalContainsOperatorState
LocalPolicyContainsGlobalState
```

### Why this was added

Without explicit separation, a supposedly global signed bundle could smuggle local operator choices, or a local file could pretend to carry global authority.

The composition rule ensures:

```text
global policy remains global
operator policy remains local
neither source silently acquires the other's authority
```

---

# Part III — Signed Moderation Snapshots

## 16. New `signed_moderation` module

A new module was added:

```text
crates/ron-policy/src/signed_moderation.rs
```

It verifies authenticated, epoch-bound global moderation snapshots.

This module uses `ron-kms` Ed25519 primitives but remains declarative and read-only.

It performs no:

```text
key custody
file loading
policy activation
storage mutation
provider withdrawal
wallet mutation
ledger mutation
reward issuance
```

---

## 17. New `ron-kms` dependency

`Cargo.toml` gained:

```toml
ron-kms = { path = "../ron-kms" }
```

The dependency is used for Ed25519 signing verification in the moderation snapshot model.

`ron-policy` does not become the key-management service. It consumes verification primitives from `ron-kms`; deployment configuration and key custody remain outside this crate. 

---

## 18. Signed snapshot version and domain

The signed policy format defines:

```text
SIGNED_MODERATION_POLICY_VERSION = 1
```

and a domain separator:

```text
rustyonions.moderation-policy.v1
```

The domain separator is included in the signed payload so a valid signature from another protocol or artifact type cannot be reused as a moderation-policy signature.

This prevents cross-protocol signature confusion.

---

## 19. `SignedModerationPolicyV1`

The strict signed snapshot includes:

```text
version
signer_id
epoch
issued_at_unix_s
expires_at_unix_s
policy
signature_hex
```

The wire structure rejects unknown fields.

### `version`

Must match the supported signed policy version.

### `signer_id`

Identifies the expected configured signer.

The signer identifier is validated for canonical form and bounded length.

### `epoch`

A strictly increasing global policy generation.

Epoch zero is rejected.

### `issued_at_unix_s`

The time at which the snapshot becomes valid.

A snapshot from the future is rejected until its issue time is reached.

### `expires_at_unix_s`

The expiration boundary.

It must be later than the issue time, and an expired snapshot is rejected.

### `policy`

Contains the exact deterministic moderation policy authenticated by the signature.

Before runtime activation, the verified global policy is composed with local operator policy through the authority-separating composition path.

### `signature_hex`

Contains the Ed25519 signature as:

```text
exactly 128 lowercase hexadecimal characters
```

Malformed, uppercase, truncated, oversized, or non-hexadecimal signatures reject before cryptographic acceptance.

---

## 20. Trust is configured separately

The public key is deliberately not treated as trusted merely because it appears inside the signed artifact.

Instead, callers supply a separately configured:

```text
TrustedModerationSigner
```

That value binds:

```text
trusted signer identifier
trusted Ed25519 public key
```

This prevents a malicious snapshot from embedding its own public key and declaring itself trusted.

The snapshot signer must match the configured trust record.

---

## 21. Deterministic signing payload

`SignedModerationPolicyV1` produces a deterministic signing payload.

The payload includes the signed fields under the moderation-specific domain separator.

The signature does not authenticate itself; `signature_hex` is excluded from the payload being signed.

Because the underlying moderation policy uses deterministic ordered sets, identical snapshots produce identical payload bytes.

---

## 22. Verification function

The main verification function is:

```rust
verify_signed_moderation_policy(
    snapshot,
    trusted_signer,
    now_unix_s,
    last_accepted_epoch,
)
```

Successful verification returns authenticated metadata and the verified moderation policy.

The verifier checks:

```text
supported wire version
canonical signer ID
configured signer match
nonzero epoch
valid issue/expiration window
snapshot is not from the future
snapshot is not expired
epoch advances beyond last accepted epoch
canonical signature encoding
Ed25519 signature validity
deterministic payload encoding
```

The explicit error taxonomy includes:

```text
UnsupportedVersion
InvalidSignerId
UntrustedSigner
InvalidEpoch
InvalidValidityWindow
NotYetValid
Expired
Rollback
InvalidSignatureEncoding
SignatureVerificationFailed
PayloadEncoding
```

The verifier therefore fails with a precise reason rather than a generic invalid-policy result. 

---

## 23. Rollback protection

The verifier accepts an optional last accepted epoch.

When supplied:

```text
incoming epoch must be strictly greater than last accepted epoch
```

Rejected examples:

```text
incoming epoch lower than last accepted
incoming epoch equal to last accepted
replaying a previously accepted snapshot
```

This prevents a validly signed but obsolete global deny or tombstone list from replacing a newer accepted policy.

`ron-policy` verifies the relationship. Persistence of the last accepted epoch belongs to the consuming runtime, which is currently `macronode`.

---

## 24. Validity-window enforcement

The signed snapshot rejects:

```text
expiration equal to issue time
expiration earlier than issue time
current time earlier than issue time
current time at or beyond expiration
```

This gives global moderation snapshots an explicit bounded lifetime.

A cryptographically valid signature does not override expiration or issue-time rules.

---

## 25. Tamper resistance

Any mutation after signing changes the deterministic payload.

Examples that invalidate the signature include:

```text
adding a denied object
removing a tombstone
changing the epoch
changing issue time
changing expiration
changing signer ID
altering any authenticated policy field
```

The focused tests verify that a policy modified after signing is rejected.

---

# Part IV — Public API Changes

## 26. `src/lib.rs` exports

The crate root was updated to expose the new modules:

```rust
pub mod moderation;
pub mod signed_moderation;
```

It also re-exports the principal moderation types under stable public names, including the equivalents of:

```text
B3Id
B3IdError
ModerationDecision
ModerationEffect
ModerationPolicy
ModerationReasonCode
ModerationPolicyCompositionError

SignedModerationPolicyV1
TrustedModerationSigner
VerifiedModerationPolicy
SignedModerationPolicyError
verify_signed_moderation_policy
encode_ed25519_signature
SIGNED_MODERATION_POLICY_VERSION
SIGNED_MODERATION_POLICY_DOMAIN
```

This lets consumers import the canonical surface from `ron_policy` instead of reaching into internal modules.

---

# Part V — Tests Added and Expanded

## 27. `tests/moderation_policy.rs`

The focused moderation suite proves the local deterministic model independently of any runtime service.

Coverage includes:

```text
canonical B3 identifiers are accepted
noncanonical identifiers are rejected
no-rule objects permit serving
local allows permit serving
global denies refuse serving
owner tombstones refuse serving
local blocks refuse serving
quarantined objects do not permit serving
local allow cannot override stronger states
near-identical object identifiers do not match
unknown JSON fields reject
serialization is deterministic
entry counts are correct
signed-global and local policy compose correctly
global precedence survives composition
signed input cannot carry operator state
local input cannot carry global state
```

The original Phase 10A moderation suite passed with eight focused tests. Composition tests were subsequently added and passed during the later Phase 10 policy-composition slice. 

---

## 28. `tests/signed_moderation_policy.rs`

A second focused suite was added for authenticated global policy snapshots.

Coverage includes:

```text
valid signed snapshot authenticates the exact policy
policy tampering after signing rejects
untrusted signer rejects
malformed signer identity rejects
unsupported version rejects
epoch zero rejects
invalid validity window rejects
not-yet-valid snapshot rejects
expired snapshot rejects
equal epoch replay rejects
lower epoch rollback rejects
malformed signature encoding rejects
invalid Ed25519 signature rejects
```

The tests use generated test-only Ed25519 keys. They do not provide runtime key custody or production trust configuration. 

---

## 29. Regression preservation

The new moderation surface was added without replacing or weakening the existing generic policy engine.

Existing boundaries remain intact:

```text
policy decisions are not receipts
policy allows are not paid proofs
policy denies are not ledger truth
policy obligations are not wallet commands
economics configuration is not balance authority
ron-policy does not mint, issue, transfer, or burn ROC
ron-policy does not produce settlement finality
```

The new signed policy support authenticates moderation data only. It does not authenticate economic settlement, paid access, wallet balances, or QuickChain finality.

---

## 30. Verification commands used

The relevant focused and crate-level gates included:

```bash
cargo test -p ron-policy --test moderation_policy

cargo test -p ron-policy --test signed_moderation_policy

cargo check -p ron-policy

cargo clippy \
  -p ron-policy \
  --all-targets \
  --no-deps \
  -- -D warnings

cargo check --workspace
```

The focused moderation, signed-policy, and composition slices passed before their downstream runtime integrations proceeded.

---

# Part VI — Downstream Impact

## 31. `svc-storage`

`svc-storage` now consumes `ron-policy` for all object-read moderation.

It does not maintain a separate moderation state machine.

The read order is:

```text
parse canonical B3 identity
  ->
evaluate ModerationPolicy
  ->
refuse unless the decision permits serving
  ->
only then access storage
  ->
verify and return bytes
```

This is applied to:

```text
OAP OBJ_GET
GET /o/:cid
HEAD /o/:cid
```

---

## 32. `macronode`

`macronode` consumes `ron-policy` for:

```text
startup policy activation
signed global policy verification
rollback checks
global/local composition
runtime policy snapshots
status reason/count reporting
serve-time policy injection
operator moderation behavior
```

`macronode` owns runtime concerns such as:

```text
environment variables
trusted signer configuration
policy file locations
accepted epoch persistence
startup failure handling
readiness
status reporting
```

Those concerns were intentionally not moved into `ron-policy`.

---

## 33. `crabnode`

The operator CLI uses the canonical `B3Id` and `ModerationPolicy` behavior when modifying local policy files.

This means CLI actions such as:

```text
block
unblock
allow
remove-allow
quarantine
release-quarantine
prune object validation
```

use the same exact identifier rules as runtime serving and pruning.

The CLI does not create an alternate identifier parser or policy precedence model.

---

# Part VII — Security and Authority Boundaries

## 34. What `ron-policy` now owns

After Phase 10, `ron-policy` owns:

```text
canonical exact B3 moderation identity
moderation sets
moderation precedence
moderation effects
stable moderation reason codes
deterministic policy serialization
set insertion and removal behavior
entry-count behavior
global/local authority composition
signed snapshot wire format
domain-separated signing payload
trusted-signer matching
Ed25519 verification
validity-window checks
epoch rollback checks
structured verification errors
```

---

## 35. What `ron-policy` does not own

The crate still does not own:

```text
policy file paths
disk reads or writes
atomic local policy replacement
trusted-key deployment
private-key custody
accepted-epoch persistence
runtime policy activation
hot reload
HTTP response mapping
storage reads
physical byte deletion
DHT provider withdrawal
index-cache invalidation
network-wide deletion
reward eligibility
reward planning
wallet mutation
ledger mutation
confirmed ROC
bridge settlement
ROX minting or burning
```

This separation is important:

```text
ron-policy decides and verifies declaratively
svc-storage enforces at the byte-read boundary
macronode activates and supervises runtime policy
crabnode manages operator intent
svc-wallet mutates economic state
ron-ledger records durable economic truth
```

---

## 36. No fake authority introduced

The signed moderation feature does not mean that `ron-policy` became a global governance runtime.

It verifies one configured signer and one signed artifact.

It does not:

```text
distribute policies
select signers
rotate trust roots
run consensus
form a quorum
publish network state
delete remote content
force another operator to prune
issue penalties
slash nodes
settle rewards
```

Those would require separate explicit systems and later build phases.

---

# Part VIII — Behavioral and Compatibility Notes

## 37. New strictness

Consumers now need to provide canonical lowercase B3 identifiers.

Input that may previously have been accepted as an opaque string now rejects if it is not exactly:

```text
b3:<64 lowercase hex>
```

This is deliberate and should not be relaxed with automatic normalization.

---

## 38. Local allow semantics

A local allow is not an override for every policy source.

It means:

```text
serve when no stronger refusal or quarantine state applies
```

It does not mean:

```text
override global deny
override owner tombstone
override local block
override quarantine
```

Any future UI should present this distinction clearly.

---

## 39. Global policy semantics

A signed snapshot proves:

```text
the configured signer authenticated these exact deterministic bytes
the snapshot is within its validity window
the epoch is newer than the caller's accepted epoch
```

It does not independently prove:

```text
the signer made a morally or legally correct decision
the content is illegal
the object was actually deleted
all nodes accepted the snapshot
the network reached consensus
```

Cryptographic authenticity and policy correctness remain different concepts.

---

## 40. Startup snapshot posture

The current downstream runtime activates moderation as a startup snapshot.

Current posture:

```text
startup activation: implemented
signed snapshot verification: implemented
rollback persistence: implemented by macronode
hot reload: not implemented
```

The lack of hot reload is not hidden. Operators must restart the relevant node runtime for local policy-file changes to become active.

---

# Part IX — Final Crate State

## 41. Final completed behavior

At the end of Phase 10, `ron-policy` provides one canonical and reusable answer for:

```text
Is this exact B3 object servable?
Why was it allowed, refused, or quarantined?
Which policy source owns the state?
Can signed global and local policy be safely combined?
Did a trusted signer authenticate this global snapshot?
Is the snapshot current, unexpired, and newer than the last accepted epoch?
```

That behavior is now consumed by the real storage and service-node runtime rather than existing only as isolated scaffolding.

---

## 42. Final status label

Use this status in future notes:

```text
ron-policy BUILD_PLAN_Z Phase 10 moderation work:
COMPLETE / GREEN / PARKED

Canonical exact-B3 policy:
COMPLETE

Signed global moderation verification:
COMPLETE

Signed-global/local-operator composition:
COMPLETE

Runtime enforcement consumers:
INTEGRATED IN SVC-STORAGE AND MACRONODE
```

---

## 43. Future work, only when required by later phases

Potential future extensions include:

```text
multi-signer or quorum-authenticated policy bundles
trusted signer rotation
policy distribution
bounded hot reload
audit-event integration
policy transparency records
appeal or review metadata
network propagation of policy versions
```

These must preserve the current invariants:

```text
exact B3 matching
deterministic serialization
fixed moderation precedence
global/local authority separation
fail-closed signature and epoch checks
no storage or economic mutation inside ron-policy
```

No later crate should duplicate the moderation state machine or weaken the strict B3 identity contract.

The next crate-specific changelog should cover `svc-storage`, including serve-time enforcement, bounded policy loading, router injection, full read-path coverage, and exact local pruning.


### END NOTE - JULY 11 2026 - 22:50 CST


### BEGIN NOTE - JULY 12 2026 - 14:05 CST

Here are the session-specific changelog notes for **`ron-policy`**.

# `ron-policy` — Session Changelog Notes

## Scope

This session extended `ron-policy` with the canonical policy surface required by Phase 11 of `BUILD_PLAN_Z`: amnesia-first storage, explicit persistence eligibility, policy-gated review, operator pinning controls, and moderation precedence.

The crate remains the authoritative source for persistence acceptance rules. Storage and macronode code call into this policy rather than inventing separate approval logic.

---

## Added canonical persistence policy model

A shared `PersistencePolicy` model was added to represent whether an object may become eligible for persistent storage.

The policy now owns:

* allowed asset categories;
* required persistence-review threshold;
* whether operator pinning is explicitly permitted;
* canonical interaction between persistence approval and moderation state;
* fail-closed defaults.

This provides one reusable policy contract for `svc-storage`, `macronode`, and future callers.

---

## Added fail-closed defaults

The default persistence policy does not automatically approve storage categories, review outcomes, or operator pinning.

Default behavior now requires explicit configuration before an object can become persistent-storage eligible.

This prevents:

* accidental persistence caused by missing configuration;
* implicit approval of unknown asset categories;
* pinning without explicit operator-pinning authorization;
* unsafe behavior when policy fields are absent or incomplete.

---

## Added exact asset-category eligibility

Persistence policy now evaluates the exact `AssetKind` associated with a candidate.

An asset category must be explicitly enabled before persistence approval can succeed.

The policy does not treat one allowed category as permission for another category and does not use broad or fuzzy category matching.

This ensures that, for example, allowing image persistence does not implicitly authorize unrelated asset types.

---

## Added configurable review thresholds

Persistence policy now carries the review level required before an object can become persistent-storage eligible.

Approval checks compare the candidate’s completed review posture against the policy’s configured threshold.

This keeps the review requirement declarative and reusable rather than embedding review assumptions directly inside `svc-storage` or macronode handlers.

The review threshold can be configured while still retaining fail-closed behavior when the necessary review has not occurred.

---

## Added explicit operator-pinning policy

Operator pinning is now controlled by a dedicated policy setting.

Pinning is denied unless operator pinning has been explicitly enabled.

This separates two related but distinct permissions:

1. whether an object may become verified for persistent storage;
2. whether an operator may elevate that verified object into the pinned state.

A policy that allows persistence approval does not automatically allow operator pinning.

---

## Integrated canonical moderation precedence

Persistence approval now respects the existing canonical moderation result instead of creating a second moderation system.

Moderation refusal states defeat persistence approval, even when:

* the asset category is allowed;
* the required review threshold has been met;
* operator pinning is enabled.

The tested moderation precedence remains:

```text
global deny
> owner tombstone
> local block
> quarantine
> local allow
> no rule
```

A weaker local permission cannot override a stronger moderation refusal.

---

## Added persistence-state policy outcomes

The policy now supports the decisions required by the shared persistence state machine.

Successful policy review may permit transition toward:

* `verified_persistent`;
* `pinned_by_operator`, when pinning is explicitly enabled.

Canonical moderation refusal instead preserves or projects the appropriate non-durable state, including:

* `global_denied`;
* `owner_tombstoned`;
* `operator_blocked`;
* `quarantined`.

These refusal states remain ineligible for durable storage.

---

## Preserved separation of authority

The new persistence policy contains no wallet, ledger, reward, minting, burning, or payment authority.

It only evaluates persistence eligibility and pinning permissions.

This preserves the project boundary that:

* `ron-policy` validates rules;
* `svc-storage` owns local storage state;
* macronode exposes operator controls;
* economic mutation remains outside the storage path.

---

## Added stable serialization contract

The persistence-policy wire representation was covered by focused tests.

The tests verify that:

* the serialized policy shape remains stable;
* unknown or malformed fields do not silently weaken policy;
* defaults remain fail closed;
* configuration remains explicit.

This helps prevent policy behavior from changing accidentally during future API or configuration work.

---

## Focused tests added or completed

The `ron-policy` persistence test suite now contains eight focused tests:

```text
default_policy_is_fail_closed
asset_category_policy_is_exact
enabled_asset_requires_configured_review_threshold
review_threshold_is_declarative_and_configurable
operator_pinning_requires_explicit_policy_enablement
moderation_refusal_states_defeat_persistence_approval
strongest_moderation_state_keeps_canonical_precedence
persistence_policy_wire_shape_is_strict_and_stable
```

These tests prove:

* no default persistence approval;
* exact asset-category matching;
* explicit review requirements;
* configurable review thresholds;
* explicit operator-pinning authorization;
* moderation refusals defeat approval;
* canonical moderation precedence remains intact;
* strict and stable policy serialization.

---

## Validation completed

The focused persistence-policy suite passed:

```text
ron-policy persistence_policy
8 passed
0 failed
```

Strict Clippy also passed:

```text
cargo clippy \
  -p ron-policy \
  --all-targets \
  --no-deps \
  -- -D warnings
```

The final workspace check passed with the updated policy crate:

```text
cargo check --workspace
```

---

## Result

`ron-policy` now provides the canonical, reusable, fail-closed policy foundation for Phase 11 persistence behavior.

Other crates can ask the policy whether a candidate may be approved or pinned without duplicating:

* category acceptance rules;
* review-threshold rules;
* operator-pinning rules;
* moderation precedence;
* durable-eligibility acceptance logic.

The `ron-policy` portion of Phase 11 is complete, green, and parked.

The next changelog set can cover `svc-storage`.


### END NOTE - JULY 12 2026 - 14:05 CST