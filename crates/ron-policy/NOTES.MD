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