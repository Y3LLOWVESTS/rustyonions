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