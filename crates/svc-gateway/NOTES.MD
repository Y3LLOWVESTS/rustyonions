### BEGIN NOTE - NOVEMBER 3 2025 - 13:33 CST
(carry over notes from omnigate for svc-gateway to enhance build efficiency and prevent drift)

Here’s a laser-focused carry-over pack so we can stand up **svc-gateway** fast, with zero wheel-reinvention.

# What to carry over from Omnigate (and how it maps to svc-gateway)

## 1) Proven patterns you should copy verbatim

These shaved the most time off Omnigate—reuse them 1:1:

* **Layer order discipline (Axum “last-added runs first”)**

  * Extension data needed by inner layers (e.g., policy bundles, capability maps) must be added **after** you apply your inner middleware layers, so the inner layer can read it.
  * In svc-gateway, keep `layers/*` (timeouts, body caps, decode guard, rate limits, DRR, tarpit, auth, corr) as the **inner** stack and add any `Extension<...>` (e.g., residency/policy bundles) **after** that.

* **Truthful readiness with anti-flap**

  * Reuse the readiness policy: trip on (a) sustained inflight over threshold, or (b) sustained error-rate (429/503) over a rolling window; hold degraded for N seconds; count transitions.
  * Bridge inflight with an RAII guard in your admission/queues (we fixed this in Omnigate—copy the approach).

* **Stable JSON error envelopes + Retry-After**

  * Map 429 and degraded 503 to include `Retry-After` (seconds). Keep a single `Problem { code, message, retryable, retry_after_ms?, reason? }` envelope.
  * This keeps docs, tests, and clients sane across services.

* **Smoke scripts that prove the big paths**

  * Policy smoke (403/415/413 + metric deltas).
  * Readiness smoke (degrade under load; hold, then recover).
  * These will catch 95% of “order, wiring, thresholds” mistakes in minutes.

* **Integration tests that prove readiness deterministically**

  * One test trips via **inflight** (using a `/sleep` route or equivalent).
  * One test trips via **error-rate** (we forced 503s with a tiny queue; you can also force 429s via tiny quotas).

## 2) Concrete module-to-module mapping (Omnigate → svc-gateway)

| Omnigate piece                                                                  | What to port                                                                                       | svc-gateway target                                                                                                 |
| ------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| `middleware/…` (corr_id, policy layer, body caps, decompress guard, slow-loris) | Patterns + order (classification → corr → policy → body caps → decode guard → timeouts/tarpit)     | `src/layers/{corr.rs,body_caps.rs,decode_guard.rs,timeouts.rs,tarpit.rs,auth.rs,drr.rs,rate_limit.rs}`             |
| `admission/fair_queue.rs` (RAII inflight gauge)                                 | The **RAII enter/leave** wiring into readiness gauges; the “interactive headroom” idea if you want | `src/layers/drr.rs` (if DRR owns concurrency) or `src/admission/quotas.rs` + `src/layers/rate_limit.rs` (if split) |
| `errors/http_map.rs`                                                            | The **Problem** shape + `Retry-After` addition for 429 and degraded 503                            | `src/errors.rs`                                                                                                    |
| `metrics/gates.rs`                                                              | The readiness gauge/counter names; init registration                                               | `src/observability/metrics.rs` (keep names stable)                                                                 |
| Readiness policy + sampler                                                      | Trip logic (inflight + error-rate + hold), and the background sampler that rolls 429/503/drops     | `src/readiness/mod.rs` (core), `src/readiness/keys.rs` (metric names/constants)                                    |
| Policy bundle normalization (strict+lenient parse)                              | If svc-gateway needs a bundle (residency/abuse), copy the lenient → strict normalize trick         | `src/policy/{mod.rs,residency.rs,abuse.rs}` (plus `Extension` layering order)                                      |
| Admin plane endpoints                                                           | `/healthz`, `/readyz`, `/metrics`, `/version`                                                      | `src/routes/{health.rs,ready.rs,metrics.rs,version.rs}`                                                            |
| Env toggles (`OMNIGATE_DEV_READY`, `AMNESIA`)                                   | Same semantics for **gateway** (`SVC_GATEWAY_DEV_READY`, `SVC_GATEWAY_AMNESIA`)                    | `src/config/{amnesia.rs,env.rs}` and used in `main.rs`                                                             |

> Efficiency tip: **do include the Omnigate CODEBUNDLE** during implementation. You won’t copy files 1:1, but you’ll copy **patterns and short code paths** (readiness, error envelope, RAII inflight, layer order). It’s the fastest path and avoids logic drift.

## 3) Config knobs to carry forward (TOML fields)

* **Server/admin**: `bind`, `metrics_addr`, `amnesia`.
* **Admission**:

  * `global_quota { qps, burst }`
  * `ip_quota { enabled, qps, burst, buckets[]? }` (optional CIDR buckets)
  * `drr/drr_like { max_inflight, headroom?, class_weights? }`
  * `body { max_content_length, reject_on_missing_length }`
  * `decompression { allow[], deny_stacked }`
* **Policy**:

  * `enabled, bundle_path, fail_mode`
  * (Gateway may add `residency.default_region`, `abuse.blocklist_path`, etc.)
* **Readiness**:

  * `max_inflight_threshold`
  * `error_rate_429_503_pct`
  * `window_secs`
  * `hold_for_secs`

Keep the **same metric names** for the readiness gauges/counters so dashboards can be reused:

* Gauges: `ready_inflight_current`, `ready_error_rate_pct`, `ready_queue_saturated`
* Counters: `ready_trips_total{reason}`, `ready_state_changes_total{to}`

## 4) Tests and scripts to stand up first (fastest proof of life)

### Must-have integration tests (copy and adapt)

* `tests/integration/readyz_degrade.rs`

  * Test 1: **inflight trip** (low threshold + `/routes/ready.rs` sleeps or `/routes/objects.rs` does a small delay)
  * Test 2: **error-rate trip** by:

    * (A) Tiny DRR cap → 503 drops **or**
    * (B) Tiny quotas → 429s
* `tests/integration/caps_limits.rs`

  * 413 on oversize (`body_caps`)
  * 415 on stacked encodings (`decode_guard`)
  * Optional: 400 on unknown content-encoding if you want stricter behavior
* `tests/integration/taxonomy_stability.rs`

  * Map a small set of gateway-specific error codes → stable envelope (`Problem{code,...}`) and status
* `tests/interop_vectors.rs`

  * Round-trip OAP/1 framing vectors and manifest hash vectors (you already have vectors scaffolded)

### Smoke scripts (bash)

* `scripts/dev_run.sh`

  * Runs server with the sample config (prints bind + echoes env toggles)
* `scripts/soak_test.sh`

  * Long-ish wrk/hey soak to watch readiness behavior and error rates
* `scripts/chaos_burst.sh`

  * Sudden burst to trip DRR/quotas; greps the key metrics
* `scripts/export_metrics.sh`

  * Dumps a filtered Prometheus snapshot (e.g., `ready_|*_reject_total|*_shortcircuits_total`)

**Why these prove functionality**
They touch every pillar you just locked down in Omnigate—layer order, capacity guards, truthful readiness, and metric visibility—**but in minutes**.

## 5) Readiness design in svc-gateway (exact expectations)

* **Trip conditions**: identical to Omnigate: inflight **or** error-rate.
* **Error-rate inputs**: 429 (rate limiter), 503 (DRR/tarpit shed), and optionally **abuse/policy** short-circuit 403/451→ you can choose to count only 503/429 to keep semantics tidy (recommended).
* **Sources** to increment in the sampler:

  * `rate_limit` layer: `GATEWAY_RL_EXHAUSTED_TOTAL{scope="global|ip|class"}`
  * `drr` layer: `GATEWAY_DRR_DROPS_TOTAL`
  * `tarpit` layer (if you use 503): `GATEWAY_TARPIT_DROPS_TOTAL`
* **Gauges** set points:

  * `ready_inflight_current` updated by DRR or a tiny “inflight bridge” layer (RAII guard)
  * `ready_queue_saturated` toggled when DRR/tarpit is saturated
* **Hold & transitions**:

  * Use the **same counters** to show `to="degraded"` and `to="ready"` once per transition.

## 6) Error taxonomy (keep stability between services)

* Reuse the **Problem** envelope; keep short, stable `code` strings:

  * `policy_denied`, `oversize_body`, `stacked_encoding`, `rate_limited`, `overloaded`
* Map to statuses:

  * 403, 413, 415, 429, 503 — and never leak internal reasons in messages.
* Tests:

  * `taxonomy_stability.rs` ensures codes and fields are present and unchanged.

## 7) Observability and dashboards

* Export the **same** readiness and gate metrics names, then clone your Omnigate dashboard JSON and tweak titles/targets to `svc-gateway`. Put it at `docs/dashboards/gateway_golden.json`.
* Grep snippets you’ll use during local dev:

  ```
  curl -s http://127.0.0.1:<metrics_port>/metrics | egrep 'ready_|shortcircuits|reject_total|drops_total|exhausted_total'
  ```

## 8) Performance baseline (don’t overthink yet)

* Criterion bench like Omnigate’s `middleware_ping` is enough for now:

  * In-process GET /objects hot path (no forwarding) → establish µs-level overhead target.
* Later, add **forward/** client stubs (overlay, index, storage). Use reqwest’s rustls and keep timeouts sane.

## 9) “Nice to have” post-Beta (queue for polish)

* `/ops/vars` — dump effective caps (DRR inflight, quotas, body limits, decode allowlist).
* **Per-class DRR** (anon/auth/admin) with weights (your `layers/drr.rs` already hints this).
* Optional **slow-recovery** readiness (require N good checks after hold expires).
* Optional **EWMA** in error-rate sampler (less spiky than a tumbling window).
* **OpenAPI** doc in `docs/API.md` for public endpoints (`/o/{addr}` and ranged GET).
* Fuzz targets (already scaffolded): `oap_frame.rs`, `taxonomy_mapper.rs`.

## 10) Quick start for svc-gateway (day-1)

Terminal A:

```
cargo run -p svc-gateway --bin svc-gateway -- --config configs/svc-gateway.sample.toml
```

Terminal B:

```
scripts/dev_run.sh         # if you want a wrapper
scripts/chaos_burst.sh     # prove readiness degrade
scripts/soak_test.sh       # steady state
cargo test -p svc-gateway --test readyz_degrade --test caps_limits --test taxonomy_stability
```

## 11) Final “yes/no” on including Omnigate’s CODEBUNDLE

**Yes, include it.** You’ll copy short files and patterns (readiness, error envelope, sampler, RAII inflight, layer ordering) and rename types. This avoids conceptual drift and gives you **fast, correct** scaffolding. We’re not duplicating business logic—just reusing *infrastructure glue* that we already validated.

---


### END NOTE - NOVEMBER 3 2025 - 13:33 CST


### BEGIN NOTE - NOVEMBER 3 2025 - 19:27 CST
---

# svc-gateway — Carry-Over Notes 

## 0) What’s currently failing (and why)

### The error you posted (now)

```
error[E0277]: the trait bound `impl IntoResponse: Handler<_, _>` is not satisfied
  --> crates/svc-gateway/src/main.rs:20:32
   |
20 | .route("/metrics", get(routes::metrics::expose(metrics.clone())))
   |                            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

**Root cause:** In Axum 0.7, `get(...)` expects a *handler* (a function/closure that implements `Handler<T, S>`), **not** the *result* of calling your function. You’re invoking `expose(...)` (and `health()`, `ready(...)`) at route definition time, which produces `impl IntoResponse`. That value doesn’t implement `Handler`.

**Fix pattern:** Pass a handler function (or closure) to `get()`. If a handler needs state, use `State<AppState>` extractor. Do **not** call it during route building.

---

## 1) Dependency hygiene (and what we already fixed)

* **Tower version conflicts:** We unified on **tower = 0.5.2** across the workspace (and added `[patch.crates-io] tower = "=0.5.2"` at the root). `cargo tree -p svc-gateway | rg '^tower v'` now shows only 0.5.2 (good).
* **Axum/Tower bounds:** Axum 0.7 requires route/stack layers whose resulting service type satisfies `Service<Request<Body>> + Clone + Send + 'static`.

  * We replaced the custom correlation middleware with **tower-http `request-id` layers** that already satisfy these bounds.

**Keep these pins:**

* `axum = 0.7.x` (workspace-pinned)
* `tower = 0.5.2` (workspace-pinned + crates-io patch)
* `tower-http = 0.6.6`

---

## 2) Router/layers architecture (agreed order)

**Outer → inner (last-added runs first on request path):**

```
corr (request-id set+propagate)  ← route-level (Router::route_layer)
auth (future)
tarpit (optional)
drr (fair queue)
rate_limit
decode_guard (decompression ratio & absolute caps)
body_caps (max body bytes)
timeouts (read timeout)
handler
```

We’re applying **corr** at the *route* level (works well with Axum’s bounds) and the rest with `.layer(...)` at the router level.

---

## 3) Paste-ready fixes (full files)

### A) `crates/svc-gateway/src/layers/corr.rs`  ✅ (kept from earlier fix)

```rust
//! RO:WHAT   Correlation/Request-ID stack for routes.
//! RO:WHY    Stable request tracing across ingress and backend calls.
//! RO:INTERACTS  axum::Router.route_layer(); tower-http::request_id::*
//! RO:INVARIANTS  - Never blocks
//!                - Always sets an ID if missing
//!                - Propagates x-request-id downstream
//! RO:SECURITY    Header-only; no secrets.
//! RO:NOTES       Use at *route* scope (Router::route_layer), not global .layer().

use tower::layer::util::Stack;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};

/// Route-level layer: set `x-request-id` if missing, and propagate it.
/// Satisfies Axum's bounds: `Service<Request<Body>> + Clone + Send + 'static`.
pub fn route_layer() -> Stack<PropagateRequestIdLayer, SetRequestIdLayer<MakeRequestUuid>> {
    let set = SetRequestIdLayer::x_request_id(MakeRequestUuid);
    let prop = PropagateRequestIdLayer::x_request_id();
    Stack::new(prop, set)
}
```

### B) `crates/svc-gateway/src/routes/metrics.rs`  ✅

```rust
//! RO:WHAT  /metrics endpoint handler (Prometheus exposition).
//! RO:WHY   Export metrics for scraping/SRE.
//! RO:INVARIANTS  Non-blocking; returns text/plain.

use axum::{extract::State, response::IntoResponse};
use ron_metrics::Metrics;

use crate::state::AppState;

pub async fn handler(State(state): State<AppState>) -> impl IntoResponse {
    // ron-metrics exposes a text exposition method; if you have a helper, call it here.
    state.metrics.expose_text()
}
```

> If your `Metrics` type uses a different API (e.g., `expose()` returning `impl IntoResponse` or a `String`), adapt the one-liner inside accordingly:
>
> * `state.metrics.expose()` or
> * `state.metrics.expose_text()` or
> * `state.metrics.export_prom_text()`.

### C) `crates/svc-gateway/src/routes/health.rs`  ✅

```rust
//! RO:WHAT  /healthz — liveness probe.
//! RO:WHY   Simple process up check. Always 200 "ok".

use axum::response::{IntoResponse, Response};

pub async fn handler() -> Response {
    "ok".into_response()
}
```

### D) `crates/svc-gateway/src/routes/ready.rs`  ✅

```rust
//! RO:WHAT  /readyz — readiness gate.
//! RO:WHY   Truthful readiness: only 200 once listeners/cfg/etc are ready.

use axum::{extract::State, http::StatusCode, response::IntoResponse};
use crate::state::AppState;

pub async fn handler(State(state): State<AppState>) -> impl IntoResponse {
    if state.readiness.all_ready() {
        (StatusCode::OK, "ready")
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "not ready")
    }
}
```

> If your `Readiness` API differs, adapt the predicate (some projects use `snapshot().ready()` or similar).

### E) `crates/svc-gateway/src/routes/mod.rs`  ✅ (unified routes table)

```rust
//! Router assembly + handlers.
//! RO:ORDER  last-added layer runs first on request path.

use axum::{routing::get, Router};

use crate::{layers, state::AppState};

pub mod health;
pub mod metrics;
pub mod ready;

// Object routes can be added later once handlers are ready:
// pub mod objects;
// pub mod objects_range;

pub fn build_router(state: AppState) -> Router {
    Router::new()
        // Core plane:
        .route("/metrics", get(metrics::handler))
        .route("/healthz", get(health::handler))
        .route("/readyz", get(ready::handler))
        .with_state(state.clone())
        // Correlation IDs at route scope (satisfies Axum's bounds).
        .route_layer(layers::corr::route_layer())
        // Inner stack (last-added runs first).
        .layer(layers::drr::DrrLayer {
            max_inflight: crate::consts::DEFAULT_MAX_CONNS,
            metrics: state.metrics.clone(),
        })
        .layer(layers::rate_limit::RateLimitLayer { rps: crate::consts::DEFAULT_RPS })
        .layer(layers::decode_guard::DecodeLayer::new(
            crate::consts::DEFAULT_DECODE_RATIO_MAX,
            crate::consts::DEFAULT_DECODE_ABS_CAP_BYTES,
        ))
        .layer(layers::body_caps::layer(crate::consts::DEFAULT_BODY_CAP_BYTES))
        .layer(layers::timeouts::layer(crate::consts::DEFAULT_READ_TIMEOUT_SECS))
}
```

> Note: I commented the object routes so we can land the core plane first. Re-enable when those handlers are implemented as proper Axum handlers (see §5).

### F) `crates/svc-gateway/src/main.rs`  ✅ (ensure we call `build_router`)

```rust
//! svc-gateway bin
//! RO:WHAT   Gateway binary entry
//! RO:WHY    Binds listener, installs routes, serves HTTP.

mod consts;
mod layers;
mod routes;
mod state;

use axum::serve;
use std::{net::SocketAddr, time::Duration};
use tokio::net::TcpListener;
use tracing::{info, Level};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // basic tracing
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).with_ansi(true).init();

    // Build app state (wire your real constructors here)
    let state = state::AppState::new_default();

    // Build router via routes module (handlers are passed by function path, not called)
    let app = routes::build_router(state.clone());

    let addr: SocketAddr = "127.0.0.1:5305".parse()?;
    let listener = TcpListener::bind(addr).await?;
    info!(%addr, "svc-gateway listening");

    // Axum serve
    serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    use tokio::signal::unix::{signal, SignalKind};
    let mut sigint = signal(SignalKind::interrupt()).expect("install SIGINT");
    let mut sigterm = signal(SignalKind::terminate()).expect("install SIGTERM");
    tokio::select! {
        _ = sigint.recv() => {}
        _ = sigterm.recv() => {}
    }
}
```

> If you already have a fancier `state::AppState` ctor or config loader, plug it in above. The key change is: **we pass handler functions to `get()`** and **we build the router in one place**.

---

## 4) Verification script (deterministic)

After pasting the files above:

```bash
# from workspace root
rm -f Cargo.lock
cargo clean

# sanity: show Tower is unified
cargo tree -p svc-gateway | rg '^tower v'

# build bin
cargo build -p svc-gateway

# run
cargo run -p svc-gateway &
APP_PID=$!

# wait a moment
sleep 0.5

# probe endpoints
curl -i http://127.0.0.1:5305/healthz
curl -i http://127.0.0.1:5305/readyz
curl -s http://127.0.0.1:5305/metrics | head -n 20

kill $APP_PID
```

Expected:

* `/healthz` → `200 ok`
* `/readyz` → `503 not ready` (until your readiness flips true)
* `/metrics` → Prometheus text (non-empty)

---

## 5) Re-enabling object routes (how to avoid `Handler` errors)

When you wire `objects`/`objects_range`, make each handler an Axum handler function:

```rust
// crates/svc-gateway/src/routes/objects.rs
use axum::{extract::{Path, State}, response::IntoResponse};
use crate::state::AppState;

pub async fn get_object(State(state): State<AppState>, Path(addr): Path<String>) -> impl IntoResponse {
    // ... do work with `state`, return impl IntoResponse
}
```

Then route them **by function path** (don’t call them!):

```rust
.route("/o/:addr", axum::routing::get(objects::get_object))
```

If you need to thread other resources into handlers, put them into `AppState`, then extract `State(AppState)` in the handler signature.

---

## 6) Known rough edges / TODOs

* **decode_guard.rs warning (`dead_code`)**
  You derived `Clone` and store `ratio_max`/`abs_cap` but don’t read them in `call()`. Once you enforce decompression caps, these warnings will disappear. For now, either use them or `#[allow(dead_code)]` on those fields to keep CI green.
* **DRR / rate_limit / tarpit stubs**
  The layer types must be `Clone + Send + 'static` and return a `Service<Request<Body>>`. If you implement custom layers, use the same pattern as corr fix: `#[derive(Clone)]`, bounds `S: Service<Request<Body>> + Clone + Send + 'static` and `S::Future: Send + 'static`.
* **Readiness**
  Ensure `state.readiness.all_ready()` (or the equivalent API) flips true when listeners/config/whatever you gate on are ready. Hook it up in your supervisor/bootstrap.
* **Metrics**
  Confirm the `Metrics` API used in `metrics::handler` is correct (`expose_text()` vs `expose()` etc.). Adjust the one-liner accordingly.
* **CI guardrails**
  Add a quick smoke job that builds + curls `/healthz` and `/metrics`; gate merges.

---

## 7) Minimal invariants checklist (to keep us honest)

* **Handlers are functions**: Every `get(...)`/`post(...)` receives a function or closure that compiles to a `Handler<T,S>`. No calling at route build time.
* **State via extractors**: Use `State<AppState>` in handler signatures; use `.with_state(state)` once on the router.
* **Layer bounds**: Custom layers implement `Clone` and target `Request<Body>`; futures are `Send + 'static`.
* **Route-level corr**: Keep request-id at `.route_layer(...)` (fewer bound headaches).
* **No locks across `.await`**: Especially in layers; prefer header-only tweaks.

---

## 8) Common pitfalls we already avoided / fixed

* **Tower 0.4 vs 0.5 split** → resolved with workspace pin + crates.io patch.
* **Custom corr layer failing `route_layer` bounds** → replaced with `tower-http` request-id stack.
* **E0277 on `Handler`** → *this* carry fixes by passing handler functions instead of invoked responses.

---

## 9) What’s next (short path to green)

1. Paste the files in §3.
2. Run the verification in §4.
3. Re-enable `/o/:addr` routes following §5 pattern.
4. Wire real readiness flips and decompression guard logic to eliminate the `dead_code` warnings.
5. Add a tiny `scripts/smoke_gateway.sh` that builds, runs on `127.0.0.1:5305`, curls the three core endpoints, and kills the PID. Gate in CI.

You’re close. The big blockers were type-bound mismatches (Tower/Axum) and handler invocation vs handler function. With these fixed, the rest is feature work.


### END NOTE - NOVEMBER 3 2025 - 19:27 CST

### BEGIN NOTE - NOVEMBER 3 2025 - 20:48 CST


here’s a clean, exhaustive carry-over packet for **svc-gateway** so the next instance can pick up fast, fix the remaining build/clippy issues, and ship a green baseline.

# What svc-gateway is (scope + shape)

* Thin, high-performance HTTP gateway sitting in front of RON services.
* Responsibilities (MVP):

  * Basic admin plane: `GET /healthz`, `GET /readyz`, `GET /metrics`, `GET /version`.
  * Per-route safety layers: read timeouts, body size caps, decompression ratio/absolute caps, request-id set/propagate, (optionally) rate limiting/tarpit/auth later.
  * Observability: Prometheus metrics via `ron-metrics` default registry; structured logs via `tracing`.
  * Readiness: `ReadyState` toggles degraded/ready consistent with kernel semantics (200/503 contract).
* Non-goals (for MVP): AuthZ/Caps, tenancy, forwarding/proxying to overlay/storage (placeholders exist), TLS termination (feature-gated for later).

# Current file layout (expected)

```
crates/svc-gateway/
├─ Cargo.toml
└─ src/
   ├─ lib.rs                     # crate root (library)
   ├─ consts.rs                  # constants (caps, timeouts, rps)
   ├─ errors.rs                  # Problem envelope + helpers
   ├─ state.rs                   # AppState { cfg, readiness, metrics }
   ├─ config/mod.rs              # Config load/env defaults (MVP)
   ├─ headers/etag.rs            # ETag helper
   ├─ observability/metrics.rs   # register() -> MetricsHandles
   ├─ readiness/mod.rs           # ReadyState API
   ├─ admission/{...}            # stubs: quotas, classifier, capabilities
   ├─ forward/{overlay_client.rs, storage_client.rs}  # stubs
   ├─ layers/
   │  ├─ mod.rs                  # order commentary
   │  ├─ timeouts.rs             # Tower TimeoutLayer wrapper
   │  ├─ body_caps.rs            # RequestBodyLimitLayer wrapper
   │  ├─ decode_guard.rs         # custom decompression guard (MVP)
   │  ├─ corr.rs                 # set + propagate x-request-id (route-layer)
   │  ├─ {auth.rs, tarpit.rs}    # stubs
   └─ routes/
      ├─ mod.rs                  # build_router(&AppState) -> Router
      ├─ health.rs               # handler()
      ├─ ready.rs                # handler()
      ├─ metrics.rs              # handler()
      └─ version.rs              # handler()
```

> **Important architecture rule:** If you add a binary (`src/main.rs`), **do not** re-`mod` these files inside the bin. The bin must depend on the **library** (`svc_gateway::...`). Re-including source files from the bin causes “no `consts` in root” and similar namespace errors.

# Known error classes we hit + their fixes

1. **`tower_layer` not found / unresolved**

   * We accidentally imported `tower_layer::util::Stack`.
   * **Fix:** Don’t use the `tower_layer` crate. We apply two request-id layers with two `route_layer(...)` calls (see below).

2. **`tower::timeout::TimeoutLayer` gated / not found**

   * Tower’s `timeout` feature wasn’t enabled.
   * **Fix in `Cargo.toml`:**

     * `tower = { version = "0.5", features = ["util","timeout"] }`

3. **`PropagateRequestIdLayer::new()` requires `HeaderName`**

   * We called `.new()` with no args.
   * **Fix:** `PropagateRequestIdLayer::new(HeaderName::from_static("x-request-id"))`.

4. **`DecodeLayer` not implementing `Layer<Route>` (E0277)**

   * We used `.layer(...)` on the **router**; Axum expects `Layer<Route>`.
   * **Fix:** use `.route_layer(...)` for per-route composition. Our custom Tower layers plug in fine there.

5. **`DEFAULT_RPS` type mismatch (`usize` vs `u64`)**

   * Config used `u64` but const was `usize`.
   * **Fix:** make `DEFAULT_RPS: u64` so construction is clean and avoid 32-bit truncation lint.

6. **Clippy: identity op / doc_markdown / must_use / wildcard_imports**

   * Examples:

     * `1 * 1024 * 1024` → `1_048_576`.
     * Backtick bare identifiers in docs (e.g., `KernelEvent`, `bus_soa`).
     * Add `#[must_use]` on getters/builders returning values used by callers.
     * Replace `use crate::consts::*;` with explicit names.
   * **Fix:** apply these stylings across the crate (see “Clippy checklist” below).

7. **Bin vs lib namespace collisions (`crate::consts` not found from bin)**

   * If a bin re-includes the same module files, the paths differ (`crate` is the bin, not lib).
   * **Fix:** keep all modules in the **lib**; bin should call `svc_gateway::routes::build_router(...)` etc., not `mod routes;`.

# Minimal interface contracts (so next edits don’t drift)

## `routes::build_router`

* Signature: `pub fn build_router(state: &AppState) -> axum::Router`
* Must register:

  * `/healthz` → 200 OK plain or JSON with snapshot (simple OK is fine)
  * `/readyz` → 200 OK when ready; 503 with `{missing:[...]}`
  * `/metrics` → Prometheus text
  * `/version` → JSON `{ "name": "svc-gateway", "version": "<crate-version>" }` (or compatible)
* Route layers (order, inner → outer):

  1. set request id if missing (`SetRequestIdLayer::x_request_id(MakeRequestUuid)`)
  2. propagate request id (`PropagateRequestIdLayer::new("x-request-id")`)
  3. read timeout (`TimeoutLayer::new(Duration::from_secs(DEFAULT_READ_TIMEOUT_SECS))`)
  4. decode guard (`DecodeLayer::new(DEFAULT_DECODE_RATIO_MAX, DEFAULT_DECODE_ABS_CAP_BYTES)`)
  5. body cap (`RequestBodyLimitLayer` using `DEFAULT_BODY_CAP_BYTES`)

## `readiness::ReadyState`

* Methods needed now:

  * `pub fn new() -> Self` (and `impl Default for ReadyState`)
  * `pub fn set_degraded(&self, d: bool)` (semicolon at end; satisfies clippy)
  * `pub fn is_degraded(&self) -> bool`
* `ready.rs` should read `if !state.readiness.is_degraded()` as “ready”. Return 200 vs 503 accordingly.

## `observability::metrics`

* `pub struct MetricsHandles { /* inner registry handles */ }`
* `pub fn register() -> anyhow::Result<MetricsHandles>`

  * Must register collectables with Prometheus default registry; avoid leaking crate-private types.
* `state::AppState::new_default()` calls `metrics::register().expect("register metrics")`.

## `consts`

* Keep these exact names to avoid drift:

  * `DEFAULT_MAX_CONNS: usize`
  * `DEFAULT_BODY_CAP_BYTES: usize` (1_048_576)
  * `DEFAULT_DECODE_RATIO_MAX: usize` (10)
  * `DEFAULT_DECODE_ABS_CAP_BYTES: usize` (≥ 8–16 MiB)
  * `DEFAULT_READ_TIMEOUT_SECS: u64`
  * `DEFAULT_WRITE_TIMEOUT_SECS: u64`
  * `DEFAULT_IDLE_TIMEOUT_SECS: u64`
  * `DEFAULT_RPS: u64`

# Cargo/toml pins that matter

* `tower = { version = "0.5", features = ["util","timeout"] }`
* `tower-http = { version = "0.6", features = ["trace","request-id","util","cors","limit","timeout","compression-full","decompression-full","set-header","catch-panic"] }`
* `axum = "0.7.*"` with `["http1","http2","json","tokio"]`
* `prometheus = "0.14"` (align with ron-metrics expectations)
* Optional: `tokio-rustls` behind `tls` feature; unused for MVP.

# Build/run recipe (lib-only and with a bin)

**A) Library-only sanity**

```
cargo fmt -p svc-gateway
cargo clippy -p svc-gateway --no-deps -- -D warnings
cargo build -p svc-gateway
```

This should compile just the **library target** cleanly. If you see bin errors, a `src/main.rs` exists—either remove it temporarily or fix it per (B).

**B) With a minimal bin (`src/main.rs`)**

* The bin should be tiny:

  * load config (or default),
  * construct `AppState`,
  * `let app = svc_gateway::routes::build_router(&state);`
  * bind a listener & serve.
* **Do not** `mod` any library files from the bin. Only import from `svc_gateway::*`.

# Clippy checklist to zero warnings

* `identity_op`: replace `1 * 1024 * 1024` → `1_048_576`.
* `doc_markdown`: backtick identifiers/types/macros/headers: `KernelEvent`, `bus_soa`, `Router::route_layer`.
* `must_use_candidate` / `return_self_not_must_use`: add `#[must_use]` on ctors/getters/builders returning values.
* `wildcard_imports`: replace `use crate::consts::*;` with explicit names.
* `semicolon_if_nothing_returned`: end setters with `;`.
* `needless_pass_by_value`: `build_router(state: &AppState)` (by ref).
* `cast_possible_truncation`: keep count types in `u64` (e.g., RPS).
* `unused_async`: remove `async` on functions without `.await` (e.g., placeholder clients).
* `match_same_arms`: merge identical arms (decode_guard content-encoding checks).
* `too_many_lines`: if clippy complains (e.g., large builders), consider small helpers; otherwise allow temporarily.
* `new_without_default`: implement `Default` for small state structs like `ReadyState`.

# Route handlers quick contract

* `health::handler`:

  * Return `200 OK` with `"ok"` (or JSON) and maybe snapshot later.
* `ready::handler`:

  * If `!is_degraded` → 200 + `"ready"`.
  * Else → 503 + JSON `{ "missing": ["dependency_name"...] }` (placeholder list is OK for MVP).
* `metrics::handler`:

  * Gather from Prometheus **default** registry; return text/plain exposition.
* `version::handler`:

  * Returns service name + crate version; no blocking calls.

# Layer order (final)

1. `corr::set_request_id_layer()`
2. `corr::propagate_request_id_layer()`
3. `timeouts::layer(DEFAULT_READ_TIMEOUT_SECS)`
4. `decode_guard::DecodeLayer::new(DEFAULT_DECODE_RATIO_MAX, DEFAULT_DECODE_ABS_CAP_BYTES)`
5. `body_caps::layer(DEFAULT_BODY_CAP_BYTES)`

> Apply these with **`route_layer(...)`** (not `.layer(...)`) on the router.

# Typical breakpoints & what they mean

* **E0433/E0432** in `routes/mod.rs` about `crate::consts`:

  * Indicates the file is being compiled as part of a **bin** crate that doesn’t have `consts` in its root (i.e., you re-modded lib code from the bin).
  * **Action:** Remove `mod routes;` etc. from the bin and call into the library (`svc_gateway::routes`…), or compile the library target alone.
* **E0277** “trait bound `Layer<Route>` not satisfied”:

  * You applied a custom Tower layer via `.layer(...)` on the router.
  * **Action:** switch to `.route_layer(...)`.
* **`tower::timeout` missing**:

  * **Action:** enable Tower `timeout` feature in Cargo.toml as above.
* **PropagateRequestIdLayer::new() missing arg**:

  * **Action:** pass a `HeaderName` (see above).
* **Clippy “doc_markdown”**:

  * Backtick identifiers in docs; it’s noisy but easy to fix.

# Minimal smoke expectations when it’s green

* `cargo clippy -p svc-gateway --no-deps -- -D warnings` → **no warnings**.
* If a bin is present and you run it:

  * `GET /healthz` → `200`
  * `GET /readyz` → `503` (until you flip `ReadyState` or wire readiness)
  * `GET /metrics` → Prometheus text
  * `GET /version` → JSON with service/version
  * Responses include an `x-request-id` (fresh or propagated).

# Short “next edits” queue

1. Ensure `Cargo.toml` has `tower = { version="0.5", features=["util","timeout"] }`.
2. Make `consts::DEFAULT_RPS: u64`; keep other consts as above.
3. Confirm `routes::build_router(&AppState)` uses **`route_layer`** for all custom layers (timeouts, decode_guard, caps, request-id).
4. Fix `corr.rs` to split **set** and **propagate** layers and pass `HeaderName`.
5. If a bin exists, remove any `mod` duplication; call into library API only.
6. Run lib-only flow (fmt/clippy/build). Triage any lingering clippy items (doc backticks, must_use).

# Optional (nice-to-have before beta)

* Add `/readyz` details: maintain a small `Vec<&'static str>` inside `ReadyState` representing missing deps (e.g., overlay, storage). Populate it later; for now an empty vector when ready is fine.
* Add `rate_limit` layer (Tower’s `ConcurrencyLimitLayer` or `RateLimitLayer`) once config is finalized.
* Expose a tiny `cfg/env` doc in `README.md` with examples (env precedence, sample TOML, defaults).
* Add `trace` middleware when we bring in `tracing` spans (request span + IDs).

---


### END NOTE - NOVEMBER 3 2025 - 20:48 CST



### BEGIN NOTE - NOVERMBER 3 2025 - 22:00 CST


# svc-gateway Carry-Over Notes (Incremental Plan to Beta)

## Snapshot (current state)

* **Build**: clean with `axum 0.7`, `tower-http 0.6`, pedantic clippy on.
* **Binary**: `main.rs` uses `axum::serve`, graceful shutdown on `CTRL+C`.
* **Config**: `Config::load()` returns defaults (env parsing TBD).
* **State**: `AppState::new(cfg, metrics)` exists and is used by router.
* **Routes**: `routes::build_router(&AppState)` compiles (minimal endpoints).
* **Metrics**: `observability::metrics::register()` sets up:

  * `http_requests_total`, `request_latency_seconds`,
  * `inflight_requests`, `rejected_total`,
  * readiness gauges (`ready_*`).
* **Layers**: stubs present (`corr`, `timeouts`, `decode_guard`)—kept **small**.
* **Headers**: `etag::etag_from_b3()` fixed (clippy-clean).
* **Lint posture**: `#![deny(clippy::all, clippy::pedantic)]`—expect doc/backtick/must_use nudges.

> Baseline tag recommendation: create a git tag now
> `git tag -a svc-gateway-baseline-clean -m "svc-gateway: clean pedantic build (no features)"; git push --tags`

---

## Guiding principles (to avoid rework)

1. **One tiny feature per PR** (or commit), with its own smoke test.
2. **Route-scoped layers only** until the end. Avoid global `.layer()` until we’re sure trait bounds are happy.
3. **No type gymnastics**: prefer ready-made `tower`/`tower-http` layers where possible.
4. **Gates first** (health/ready, metrics), **guards second** (timeouts, body limits), **policy last** (rate-limit/DRR/auth).
5. **Keep pedantic clippy on** so drift is caught early.

---

## Minimal dev loop

```
cargo fmt -p svc-gateway
cargo clippy -p svc-gateway --no-deps -- -D warnings
cargo build  -p svc-gateway
RUST_LOG=info,target=svc_gateway=debug cargo run -p svc-gateway
```

Quick probes:

```
curl -i http://127.0.0.1:5304/healthz
curl -s http://127.0.0.1:5304/readyz
curl -s http://127.0.0.1:5304/metrics | head
```

---

## Incremental plan (tiny, meaningful slices)

> Each step: add code → run trio (fmt/clippy/build) → run → `curl` probe → commit/tag.

### 0) Lock a minimal admin plane

**Goal**: Always have `/healthz`, `/readyz`, `/metrics`.

* **Add**: `routes::healthz()` → `200 ok` text; `routes::readyz()` → **temporary** `503` if `danger_ok=false` to prove the dial.
* **Add**: `routes::metrics()` exposing Prometheus default registry (already wired).
* **Router**: only mount these three to start. No layers yet.
* **Acceptance**: `healthz=200`, `metrics` serves, `readyz` chosen status matches toggle in `Config.safety.danger_ok`.

### 1) Correlation layer (route-scoped, no generics)

**Goal**: Add/propagate `x-request-id`, basic trace fields.

* Implement `layers::corr::layer()` returning `axum::middleware::from_fn(mw)` with **signature**:

  ```rust
  pub async fn mw(req: Request, next: Next) -> Response
  ```

  (No generics on `Next` in axum 0.7.)
* In the handler: attach/request ID (generate UUID if missing), set response header.
* **Mount**: `.route_layer(corr::layer())` only on `/healthz` and `/readyz` first.
* **Acceptance**: `curl -i` shows `x-request-id` on responses; logs include ID.

> **Gotcha**: `Next` is not generic in axum 0.7. If you see `E0107/E0308`, you used `Next<B>`—drop the generic.

### 2) HTTP metrics middleware (simple)

**Goal**: Count requests and latency by `route`/`method`.

* Add a tiny middleware (or use `tower_http::trace` if you prefer) that:

  * increments `http_requests_total{route,method,status}`,
  * observes `request_latency_seconds{route,method}` using `Instant::now()`.
* **Mount** it on `/healthz` only first to validate labels.
* **Acceptance**: Hit `/healthz` a few times, scrape `/metrics`, verify counters/histogram buckets grow for that route.

### 3) Timeouts (read/overall) — **route-scoped**

**Goal**: Enforce timeouts without touching global server config yet.

* Use `tower::timeout::TimeoutLayer` for a **request** timeout (overall).
* Keep per-socket read/write/idle timeouts in `Config.server` but don’t enforce them yet (we’ll push to transport later if we need).
* **Mount**: `.route_layer(TimeoutLayer::new(Duration::from_secs(cfg.server.read_timeout_secs)))` **only** on `/readyz`.
* **Acceptance**: Create a test handler that sleeps **past** timeout behind a feature flag and confirm you get `408/504` mapping (map error to `StatusCode::REQUEST_TIMEOUT` or `GATEWAY_TIMEOUT`).

> **Gotcha**: Axum route layering requires `Layer::Service: Service<Request> + Clone + Send + 'static`. Prefer `tower`/`tower-http` layers or `middleware::from_fn` (they satisfy bounds). If you see E0277, your layer type likely doesn’t implement one of those.

### 4) Body cap (simple Content-Length gate first)

**Goal**: Reject over-large bodies early.

* Start with a cheap guard: if `Content-Length > max_body_bytes`, immediately `413 Payload Too Large`.
* Add metric increment: `rejected_total{reason="body_cap"}`.
* **Do not** implement streaming cap yet (that’s trickier).
* **Acceptance**: `curl -H "Content-Length: 999999999"` (or send a big body) gets 413; metrics tick.

### 5) Decode guard (placeholder → then real)

**Goal**: Prevent zip-bomb style decompression.

* **Phase A (placeholder)**: store params (ratio/abs cap) in state; middleware **does nothing** but increments `rejected_total{reason="decode_cap"}` if a test flag is set. This ensures trait bounds are satisfied and we have the label path.
* **Phase B (later)**: when we add actual decompression (gzip/zstd/brotli), wrap the decoder stream and enforce ratio/abs bytes.

### 6) Readiness truth table

**Goal**: Make `/readyz` meaningful.

* Define minimal readiness snapshot in state:

  * `inflight_current` (gauge),
  * `error_rate_pct` (computed over sliding window via a small ring buffer),
  * `queue_saturated` (bool, stubbed false for now).
* `/readyz` returns `200` when all gates under thresholds, else `503`.
* **Acceptance**: Flip thresholds in config, watch `/readyz` change.

### 7) Simple **global** concurrency cap (cheap backpressure)

**Goal**: Fail fast when saturation.

* Add `tower::limit::ConcurrencyLimitLayer` with `cfg.server.max_conns` (or a slightly smaller per-route number) to **/readyz** only first.
* On `Service::poll_ready` errors, map to `503` and increment `rejected_total{reason="concurrency_limit"}`.
* **Acceptance**: Use `wrk`/`hey` to exceed concurrency; verify rejections and metric label.

### 8) Rate limit (token bucket) — initial

**Goal**: Tenant-agnostic RPS cap as a guardrail.

* Use `tower::limit::RateLimitLayer` (global buckets) as a start.
* Later we can add per-tenant classification in `admission::classifier` to derive labels.
* **Acceptance**: Bomb a route; see 429s and `rejected_total{reason="rate_limit"}`.

### 9) Config: minimal ENV loader

**Goal**: Override defaults via env (`SVC_GATEWAY_*`).

* Implement `Config::load()` using `std::env`:

  * `SVC_GATEWAY_BIND_ADDR`, `METRICS_ADDR`, `MAX_CONNS`, `READ_TIMEOUT_SECS`, etc.
* Document variables in `configs/.env.example`.
* **Acceptance**: Change a value via env, see log print final config on boot.

### 10) Forwarders (overlay/storage) — **stubbed** endpoints

**Goal**: Read-only fetch path smoke.

* Add routes `/v1/media/:addr` that:

  * **Phase A**: returns `501 Not Implemented` with a JSON problem envelope.
  * **Phase B later**: call `forward::overlay_client::get_bytes` (still stub), stream body, set `ETag`.
* **Acceptance**: Endpoint exists, correct status/headers, metrics counted.

### 11) Admission hooks (placeholders)

**Goal**: Plumbing for later policy.

* `admission::capabilities::has_cap(cap) -> bool` (currently returns true).
* `admission::quotas::allow(tenant) -> bool` (currently true).
* `admission::classifier::classify(path) -> &str` returns static class like `"public"`.
* Wire **only** into `/v1/media/:addr` route as experiments (log decision).
* **Acceptance**: Logs include “cap=pass/deny”, metrics label for “rejected_total{reason="quota"}” increases when we flip a test flag.

### 12) Docs + runbook crumbs

**Goal**: Keep clippy pedantic green with doc sections.

* For each `Result` returning function, add a short `# Errors` doc block (even if “I/O or config parse errors.”).
* Avoid wildcard imports; fix doc backticks (clippy nags).
* Keep READMEs short; defer deep docs to `/docs/`.

---

## Beta definition (what “beta” means for svc-gateway)

* **Admin plane**: `/healthz`, `/readyz`, `/metrics` truthful, with gauges and thresholds config-driven.
* **Guards**: working **route-scoped** timeouts, body cap, rate limit; decode guard placeholder acceptable.
* **Observability**: counters + latency histograms per route; `rejected_total` labeled by reason; correlation ID propagated.
* **Config**: defaults + env override; logs reflect effective config on boot.
* **Binary**: graceful shutdown; no panics on fast path; clean `clippy -D warnings`.
* **Perf sanity**: `/healthz` can handle 10–20k RPS on local dev without 5xx (not a hard perf goal yet).

---

## Troubleshooting cribsheet

* **`Next<B>` errors (E0107/E0308)** → axum 0.7 uses `Next` **without** generics; signature is `fn(req: Request, next: Next)`.
* **`route_layer` E0277 bounds** → your layer type must implement `Service<Request> + Clone + Send + 'static`. Prefer `tower`’s layers or `middleware::from_fn`.
* **`axum::Server` not found** → in axum 0.7 use `axum::serve(listener, app)`, not `Server::bind`.
* **Clippy pedantic doc nags** → add missing backticks, `# Errors` sections, remove wildcard imports.
* **Global vs route layer** → keep to `.route_layer(...)` during bring-up; switch to global only after traits are clearly satisfied.

---

## Suggested PR/commit sequence

1. **PR-01**: Baseline admin plane (+ corr on `/healthz`)
2. **PR-02**: HTTP metrics middleware for `/healthz`
3. **PR-03**: Timeout layer on `/readyz` (+ error mapping)
4. **PR-04**: Body cap (Content-Length gate)
5. **PR-05**: Readiness truth table + gauges update logic
6. **PR-06**: Concurrency limit on `/readyz`
7. **PR-07**: Global rate limit layer (coarse)
8. **PR-08**: ENV config loader + `.env.example`
9. **PR-09**: Media route stub + admission placeholders
10. **PR-10**: Decode guard placeholder + metric path

Each PR should include: unit test where easy, `scripts/smoke_gateway.sh` (one-liner curls), and a short “How to verify” block.

---

## Tiny smoke script (drop in `crates/svc-gateway/scripts/smoke_gateway.sh`)

```bash
#!/usr/bin/env bash
set -euo pipefail

echo "fmt+clippy+build…"
cargo fmt -p svc-gateway
cargo clippy -p svc-gateway --no-deps -- -D warnings
cargo build -p svc-gateway

pkill -f svc-gateway || true
RUST_LOG=info,target=svc_gateway=debug cargo run -p svc-gateway &

sleep 0.5
echo "healthz:";  curl -si http://127.0.0.1:5304/healthz | head -n 1
echo "readyz:";   curl -si http://127.0.0.1:5304/readyz  | head -n 1
echo "metrics:";  curl -s  http://127.0.0.1:5304/metrics | head -n 10
```

---

## Nice-to-have (post-beta)

* Swap the body cap to a true **streaming** cap for unknown/incorrect `Content-Length`.
* Real decode guard around gzip/zstd/brotli streams with ratio/abs enforcement.
* Per-tenant classification + per-class buckets for rate limiting.
* `/version` + build info endpoint (git SHA, features).
* Structured JSON logs with correlation ID and latency.

---



### END NOTE - NOVEMBER 3 2025 - 22:00 CST








### BEGIN NOTE - NOVEMBER 4 2025 - 11:25 CST


# Next steps to ship beta (tiny slices, in order)

## 1) Config loader (env → struct) + boot log

**What:** Implement `Config::load()` (read env vars you already use) and log the effective config once on boot.
**Vars to capture now:**

* `BIND_ADDR`
* `SVC_GATEWAY_DEV_ROUTES`
* `SVC_GATEWAY_MAX_BODY_BYTES`
* `SVC_GATEWAY_RL_RPS`, `SVC_GATEWAY_RL_BURST`
* `SVC_GATEWAY_READY_TIMEOUT_MS`, `SVC_GATEWAY_READY_MAX_INFLIGHT`, `SVC_GATEWAY_READY_SLEEP_MS` (dev-only), `SVC_GATEWAY_DEV_READY` (dev-only)

**Acceptance:**

* `cargo run -p svc-gateway` prints a single JSON line (or clear structured line) with the effective config.
* Changing env shows up correctly.

**Smoke:**

```
BIND_ADDR=127.0.0.1:5310 SVC_GATEWAY_MAX_BODY_BYTES=2048 cargo run -p svc-gateway
```

## 2) README + .env.example crumbs

**What:** Document the above env knobs briefly and add `configs/.env.example`.
**Acceptance:**

* `README.md` has a short “Config” table.
* `configs/.env.example` includes commented defaults.
* `scripts/run_gateway.sh` prints which env it’s using (it already does most).

## 3) Decode-guard placeholder (label only)

**What:** Add a no-op `layers/decode_guard.rs` that **only** increments `gateway_rejections_total{reason="decode_cap"}` when `SVC_GATEWAY_DEV_DECODE_REJECT=1`. Do not implement real decompression yet. Mount it only on the dev POST route for now.
**Acceptance:**

* With `SVC_GATEWAY_DEV_DECODE_REJECT=1`, `POST /dev/echo` returns `415 Unsupported Media Type` (or `400`) and metric increases.
* With the flag off, normal behavior.

**Probe:**

```
SVC_GATEWAY_DEV_ROUTES=1 SVC_GATEWAY_DEV_DECODE_REJECT=1 crates/svc-gateway/scripts/test_body_cap.sh
```

## 4) Admission placeholders (log-only)

**What:** Add `admission/` stubs:

* `classifier::class(&Request) -> &'static str` → returns `"public"` for now.
* `caps::has(_cap: &str) -> bool` → `true`.
* `quotas::allow(_tenant: &str) -> bool` → `true`.
  Wire them into **dev** routes (log the decision; do not enforce).
  **Acceptance:**
* Requests log `class=public cap=ok quota=ok`.
* Metrics: `gateway_rejections_total{reason="quota"}` only moves if `SVC_GATEWAY_DEV_QUOTA_REJECT=1` (optional).

## 5) Promote guards to production paths (scaffold only)

**What:** Add a placeholder “real” write path (e.g., `POST /v1/media` that returns `501` for now) and mount **body cap** there so the guard is attached to a real surface. Keep rate-limit on dev for now.
**Acceptance:**

* `POST /v1/media` returns `501 Not Implemented`.
* Oversized POST → `413` via body cap, counted in metrics.

**Probe:**

```
printf 'hi' | curl -i -X POST --data-binary @- http://127.0.0.1:5304/v1/media
head -c 2097153 </dev/zero | tr '\0' 'A' | curl -i -X POST --data-binary @- http://127.0.0.1:5304/v1/media
```

## 6) Readiness truth table polishing

**What:** Keep the sampler but add two cheap “real” signals:

* `inflight_current`: add a tiny middleware that bumps an atomic before/after the request on the **/readyz** route only (sampler already publishes gauges).
* `error_rate_pct`: count 5xx & 429 in a small ring buffer inside the sampler task; compute simple percentage over last N ticks. (Keep N small, e.g., 20.)
  **Acceptance:**
* Hammer `/dev/rl` to generate 429s; watch `gateway_ready_error_rate_pct` reflect non-zero.
* `/readyz` flips to `503` if `error_rate_pct > threshold` (e.g., >5%).
* This preserves your tiny blast radius and no `AppState` churn.

## 7) CI knobs + clippy pedantic + deny SHA crates

**What:** In workspace `deny.toml`, add a ban stanza for `sha-1`, `sha1`, `sha2`, `sha3` crates, while allowing `blake3`. (We won’t touch TLS libs here; this is app-level guardrails.)
**Acceptance:**

* `cargo deny check` passes; fails if any app crate tries to add SHA crates.
* `cargo clippy -D warnings` stays green.

**Probe:**

```
cargo deny check -A sources
```

## 8) Scripts: one “all-guards” smoke

**What:** Add `scripts/smoke_all.sh` that spins gateway with:

* dev routes on,
* small body cap,
* RL (low RPS),
* concurrency max=1,
* ready sleep=300ms, timeout=1000ms,
  then does: `/healthz`, two `/readyz` in parallel (expect 200/503), small/large `/dev/echo`, burst `/dev/rl`, and prints the three key metric lines (`gateway_http_requests_total`, `gateway_request_latency_seconds`, `gateway_rejections_total`).
  **Acceptance:**
* Script ends with all expected statuses and metric lines.

---

## Beta checklist (after completing the above)

* **Admin plane**: `/healthz`, `/readyz`, `/metrics`, `/version` (done ✅).
* **Guards**:

  * Timeout (readyz) ✅
  * Concurrency cap (readyz) ✅
  * Body-size cap (dev + /v1/media) ✅
  * Rate-limit (dev) ✅
  * Decode-guard placeholder (to add per step 3) ⏳
* **Admission**: stubs + logs (per step 4) ⏳
* **Observability**:

  * Correlation ID ✅
  * Per-route HTTP counters & histograms ✅
  * Gateway-prefixed readiness gauges wired ✅
  * Rejection reasons counted (body_cap, rate_limit, decode_cap, quota) ✅ (after step 3–4)
* **Config**: `Config::load()` + boot log ⏳
* **Docs**: README env table + `.env.example` ⏳
* **CI**: clippy pedantic on ✅; cargo-deny bans for SHA ⏳

At this point you’re ~80–85% to our defined beta. Steps 1–4 are the only “code” left; 5–8 are glue/tests/docs. If you want, I can drop **full, paste-ready files** for step 1 (Config loader + boot log) as the next move.


### END NOTE - NOVEMBER 4 2025 - 11:25 CST

### BENCHMARKS RESULTS - NOVEMBER 4 2025 - 11:45 CST

On a 2019 MacBook Pro (Intel i5-8257U 1.4 GHz, loopback, plain HTTP), svc-gateway delivered ~50k req/s on `/healthz` and ~48k req/s on guarded `/readyz`, with our middleware histograms showing ~**26 µs** average server-side service time per request (correlation + http_metrics enabled). A 30s hammer of `/dev/rl` sustained ~52k req/s attempted with ~1.57M **429** rejections as expected, and the body-cap test returned **413** (note: most `/dev/echo` requests showed 429 because the dev router currently applies rate-limit to the whole subrouter). `wrk` reported ~2.5–2.6 ms average client-perceived latency at 4 threads/128 conns, which includes queueing and network stack overhead; our on-box service time remains microsecond-class, confirming a cheap hot path and fast-fail guards.


### END BENCHMARK RESULTS - NOVEMBER 4 2025 - 11:45 CST






### BEGIN NOTE - NOVEMBER 5 2025 - 9:35 CST


Here’s a clean, drop-in set of carry-over notes for **svc-gateway**.

# svc-gateway — Carry-over Notes (Beta Push)

## TL;DR (status & perf)

* **Status:** healthy; admin plane wired; fast path + guards working; lock-free rate-limit in place; metrics surfaced; clippy **pedantic** clean.
* **Laptop used:** **MacBook Pro (2019), 1.4 GHz i5**, macOS, plugged in for final runs.
* **Benchmarks (wrk, 8T/256C, 30s):**

  * `/healthz`: **~51.7k RPS**, client-seen avg ~4.9 ms; **server-side** avg ≈ **23.9 µs** (Prom hist sum/count = 37.161 s / 1,553,352).
  * `/readyz` (guards on, forced ready): **~47.8k RPS**, similar tails.
  * `/dev/rl` (hot rejection path via RL): **~50.7k RPS** of 429s; **server-side** avg ≈ **21.5 µs** (32.823 s / 1,523,942).
* **Takeaway:** Gateway overhead is tiny; it won’t be the bottleneck for beta. Numbers are elite for this hardware with full middleware + metrics.

---

## What we built (high level)

* **Admin plane:** `/healthz`, `/readyz`, `/metrics` (Prometheus).
* **Middleware & guards:**

  * Correlation ID (`corr::mw`) + HTTP metrics (`observability::http_metrics::mw`) on `/healthz`.
  * **Readiness**: request timeout + concurrency cap on `/readyz`.
  * **Body cap** on `POST /dev/echo` → returns **413** on oversize.
  * **Rate limit** (lock-free token bucket with atomics) on `GET /dev/rl` → returns **429** (optional tarpit).
* **Observability:**

  * Counters/histograms:

    * `gateway_http_requests_total{method,route,status}`
    * `gateway_request_latency_seconds{method,route}` (histogram)
    * `gateway_rejections_total{reason="rate_limit"|"body_cap"}`
  * **Ready gauges:** inflight, error-rate, queue-saturated (prewarmed label series).
* **Router assembly:** layers kept minimal per route; dev routes are gated by env.

---

## Environment knobs (used by scripts)

* **Bind:** `BIND_ADDR` (default `127.0.0.1:5304`)
* **Dev routes:** `SVC_GATEWAY_DEV_ROUTES=1` (enable `/dev/*`)
* **Dev metrics prewarm:** `SVC_GATEWAY_DEV_METRICS=1` (optional helper for dashboards)
* **Force ready in benches:** `SVC_GATEWAY_DEV_READY=1`
* **Readiness guards:**
  `SVC_GATEWAY_READY_TIMEOUT_MS` (default 1000)
  `SVC_GATEWAY_READY_MAX_INFLIGHT` (default 1024)
  `SVC_GATEWAY_READY_SLEEP_MS` (0 for benches)
* **Body cap:** `SVC_GATEWAY_MAX_BODY_BYTES` (default 1 MiB)
* **Rate limit (global, lock-free):**
  `SVC_GATEWAY_RL_RPS` (tokens/sec)
  `SVC_GATEWAY_RL_BURST` (extra tokens)
  `SVC_GATEWAY_RL_TARPIT_MS` (0 for benches)
* **Logging:** `RUST_LOG=info,svc_gateway=debug`

> Note: older `SVC_GATEWAY_RPS` was replaced by `SVC_GATEWAY_RL_RPS` (+ burst/tarpit).

---

## Scripts (what they do & how to run)

### 1) Quick run (build, start, show admin plane)

```bash
crates/svc-gateway/scripts/run_gateway.sh
# Uses BIND_ADDR, SVC_GATEWAY_DEV_ROUTES, SVC_GATEWAY_DEV_READY, etc.
# Prints /healthz, /readyz and shows first metric lines; leaves the process running.
```

### 2) RL sanity (deterministic 200 then 429/429)

```bash
SVC_GATEWAY_DEV=1 SVC_GATEWAY_DEV_METRICS=1 \
crates/svc-gateway/scripts/sanity_rate_limit.sh
# Primed build, starts gateway with RL=1 rps, burst=1, tarpit=0ms
# Hits /dev/rl three times; expects: 200, then 429,429
# Prints rate_limit counters to prove correctness.
```

### 3) RL functional smoke (bursting; with watchdog)

```bash
# Optional knobs: PAR=100 REQ_TIMEOUT=3 CONNECT_TIMEOUT=1
SVC_GATEWAY_RL_RPS=3 SVC_GATEWAY_RL_BURST=5 \
crates/svc-gateway/scripts/test_rate_limit.sh
# Starts gateway, fires N concurrent GETs to /dev/rl, prints tallies and RL counters.
```

### 4) Throughput bench (hey or wrk)

```bash
THREADS=8 CONNS=256 DUR=30s SVC_GATEWAY_DEV=1 SVC_GATEWAY_DEV_METRICS=1 SVC_GATEWAY_DEV_READY=1 \
crates/svc-gateway/benches/bench_gateway.sh
# Runs healthz, readyz, dev_rl (hot rejection), and body-cap echo (413).
# Emits per-stage results and a metric snapshot for later analysis.
```

---

## Test/bench results we observed (reproducible patterns)

* **Sanity RL @ RPS=1, burst=1:** `200, 429, 429` and:

  * `gateway_http_requests_total{route="dev_rl",status="200"} == 1`
  * `gateway_http_requests_total{route="dev_rl",status="429"} == 2`
  * `gateway_rejections_total{reason="rate_limit"} == 2`
* **Benches (plugged in):**

  * `/healthz`: ~**51.7k RPS**; **service time ≈ 23.9 µs** (hist sum/count).
  * `/readyz` (guards): ~**47.8k RPS**; tails similar.
  * `/dev/rl` (mostly 429): ~**50.7k RPS**, **service time ≈ 21.5 µs**.
* **Benches (on battery):** a little lower (~50.2k/48.3k/7–9k on earlier RL attempt); plugging in lifts RPS and smooths tails.

> **How we compute service time:** from Prometheus histogram:
> `avg_service_time = <gateway_request_latency_seconds_sum> / <..._count>` → report in µs.

---

## Notable fixes & hardening we landed

* **Lock-free RL**: replaced mutex with atomics (OnceCell bucket); added **burst** + **tarpit** knobs; clamped env; fixed corner overflow (night-time second rollover) and made refill branchless + race-safe.
* **Metrics “prewarm”**: upfront label-series touch to avoid cold-start holes in dashboards.
* **Doc/clippy fixes**: pedantic clean (unused imports, doc_markdown backticks, let-unit, bool→int, similar_names, must_use candidates).
* **Portable scripts**: removed `mapfile` dependency (macOS bash), added watchdogs/timeouts, deterministic `curl` probes, and clean log tails on failure.

---

## What’s left to ship **beta**

**A. Docs & demos**

* Update crate **README.md** with: admin plane, knobs, metrics table, and sample dashboards.
* Add **RUNBOOK.md** (systemd unit, graceful shutdown, `SIGINT`/`SIGTERM`, log targets).
* Add **CONFIG.md** with env → meaning → default → impact.

**B. CI “fortress” (repo-consistent)**

* Ensure workflows include: `fmt`, `clippy -D warnings`, `test`, `deny`, `audit`, `geiger`.
* Add a tiny **perf guard** job (nightly or on label) that runs `bench_gateway.sh` and asserts **no >10% regression** on `/healthz` RPS vs a stored baseline (tolerances configurable).

**C. Tests**

* Unit tests for: body cap 413, RL math (refill, burst, clamp), correlation header propagation.
* Integration tests: readiness timeout vs. sleep hook; concurrency cap refuses beyond budget.
* Property test (quickcheck) for RL invariants (tokens never negative; burst respected).

**D. Ops polish**

* Add container/Dockerfile + example **systemd** unit with hardening flags (read-only rootfs, no new privs, seccomp profile).
* Optional: expose `/version` (git SHA, build time); ensure it appears in logs.

**E. Optional perf nits (safe to defer)**

* Pre-allocate small header maps on fast paths (avoid per-req alloc churn).
* **metrics_buf** path (thread-local counters with periodic flush) in `ron-metrics` to shave ~5–10% CPU under heavy scrape.
* Consider a tiny **retry-after** budget algorithm for RL (helps clients back off deterministically).

---

## Repro recipe (copy-paste)

```bash
# 0) Build once (will be re-used by scripts)
cargo fmt -p svc-gateway
cargo clippy -p svc-gateway --no-deps -- -D warnings
cargo build -p svc-gateway

# 1) Sanity RL (deterministic)
SVC_GATEWAY_DEV=1 SVC_GATEWAY_DEV_METRICS=1 \
crates/svc-gateway/scripts/sanity_rate_limit.sh

# 2) Throughput (plugged in)
THREADS=8 CONNS=256 DUR=30s \
SVC_GATEWAY_DEV=1 SVC_GATEWAY_DEV_METRICS=1 SVC_GATEWAY_DEV_READY=1 \
crates/svc-gateway/benches/bench_gateway.sh

# 3) Quick manual poke
curl -i http://127.0.0.1:5304/healthz
curl -i http://127.0.0.1:5304/readyz
curl -s http://127.0.0.1:5304/metrics | head
```

---

## Why this is beta-ready

* **Correctness**: guards enforce caps; rejection paths are observable (`gateway_rejections_total`), and deterministic tests pass.
* **Performance**: **~50k RPS** with **~24 µs** server-side service time on a **2019 1.4 GHz i5** is *well above* our SLO needs; gateway won’t be the bottleneck.
* **Safety & DX**: no `unsafe`; pedantic clippy; scripts reproducible across macOS; metrics prewarmed.


### END NOTE - NOVEMBER 5 2025 - 9:35 CST



### BEGIN NOTE - NOVEMBER 5 2025 - 11:17 CST
# SVC-GATEWAY BETA WRAP UP NOTES

# svc-gateway — Beta Carry-Over Notes

## TL;DR

* **Status:** ✅ **BETA** (functional + fast). Admin plane live; hot paths and guard paths proven; label-prewarmed metrics; scripts reproducible; clippy **pedantic** clean.
* **Hardware (your runs):** MacBook Pro 13" (2019), 1.4 GHz i5.
* **Throughput (wrk, 8T/256C, 30s):**

  * `/healthz`: ~**51–52k RPS**
  * `/readyz` (guards on): ~**48k RPS**
  * `/dev/rl` (429 hot reject): ~**50k RPS**
  * Server-side service-time (Prom hist sum/count): low **tens of µs**.
* **Conclusion:** Gateway overhead is tiny; it won’t be a bottleneck for Core Plane.

---

## What we built

### Routes (stable)

* `GET /healthz` — liveness; correlation id + request metrics.
* `GET /readyz` — readiness; wraps timeout + concurrency cap (truthful by default).
* `GET /metrics` — Prometheus exposition.
* `GET /version` — `{ name, version, built_at_unix }` (no SHA policy).

### Dev routes (guarded by env: `SVC_GATEWAY_DEV_ROUTES=1`)

* `POST /dev/echo` — JSON echo with **body cap** (413 on oversize).
* `GET /dev/rl` — **lock-free token bucket** path for rate-limit (200 then 429s).

### Middleware / Guards

* **Correlation ID** (`x-request-id`).
* **HTTP metrics** per route (counters, latency histogram); label prewarm to avoid cold-start gaps.
* **Readiness guards** on `/readyz`:

  * request timeout (`SVC_GATEWAY_READY_TIMEOUT_MS`, default 1000)
  * concurrency cap (`SVC_GATEWAY_READY_MAX_INFLIGHT`, default 1024)
* **Body cap** on `POST /dev/echo`: `SVC_GATEWAY_MAX_BODY_BYTES` (default 1 MiB).
* **Decode guard** (dev): rejects stacked encodings (415), rejects compressed bodies (415) and caps decoded size (413). Tracks `decode_cap`, `stacked_encoding`.
* **Lock-free rate-limit** (dev): `SVC_GATEWAY_RL_RPS`, `SVC_GATEWAY_RL_BURST`, `SVC_GATEWAY_RL_TARPIT_MS`.

### Metrics (Prometheus)

* `gateway_http_requests_total{method,route,status}`
* `gateway_request_latency_seconds{method,route}` (histogram)
* `gateway_rejections_total{reason="rate_limit|body_cap|decode_cap|stacked_encoding"}`
* Readiness gauges (exported and prewarmed):

  * `gateway_ready_inflight_current`
  * `gateway_ready_error_rate_pct` (plumbed sampler; **not** gating `/readyz` yet)
  * `gateway_ready_queue_saturated` (stubbed gauge, exported)

### Build/Version stamping

* `build.rs` sets `SVC_GATEWAY_BUILD_TS` to UNIX seconds; `/version` returns it as `built_at_unix`.

---

## Key environment knobs

**Core**

* `BIND_ADDR` (default `127.0.0.1:5304`)
* `METRICS_BIND_ADDR` (if split; else `/metrics` on main)

**Dev toggles**

* `SVC_GATEWAY_DEV_ROUTES` = `1|true|yes|on` to expose `/dev/*`
* `SVC_GATEWAY_DEV_METRICS` = `1|…` to add http_metrics layer on `/dev/*`
* `SVC_GATEWAY_DEV_READY` = `1` to force readiness in benches/scripts (no sleep)

**Readiness guards**

* `SVC_GATEWAY_READY_TIMEOUT_MS` (default 1000)
* `SVC_GATEWAY_READY_MAX_INFLIGHT` (default 1024)
* `SVC_GATEWAY_READY_SLEEP_MS` (dev hook; default 0)

**Caps**

* `SVC_GATEWAY_MAX_BODY_BYTES` (default 1_048_576)
* `SVC_GATEWAY_DECODE_ABS_CAP_BYTES` (decode guard cap; default conservative)

**Rate limit (dev hot path)**

* `SVC_GATEWAY_RL_RPS`
* `SVC_GATEWAY_RL_BURST`
* `SVC_GATEWAY_RL_TARPIT_MS` (0 for benches)

**Logging**

* `RUST_LOG=info,svc_gateway=debug` (scripts set a reasonable default)

Accepted truthy values: `1`, `true`, `yes`, `on` (case-insensitive).

---

## Layout / files you’ll touch

* `crates/svc-gateway/src/routes/`
  `health.rs`, `ready.rs`, `metrics.rs`, `version.rs`, `dev/{mod,echo,rl}.rs`, `mod.rs`
* `crates/svc-gateway/src/layers/`
  `corr.rs`, `timeouts.rs`, `concurrency.rs`, `body_caps.rs`, `decode_guard.rs`, `rate_limit.rs`
* `crates/svc-gateway/src/observability/`
  `http_metrics.rs`, `readiness.rs`
* `crates/svc-gateway/src/config/`
  `mod.rs`, `env.rs`
* `crates/svc-gateway/build.rs` — build timestamp
* `crates/svc-gateway/scripts/`
  `run_gateway.sh`, `stop_gateway.sh`, `sanity_rate_limit.sh`, `test_rate_limit.sh`, `decode_guard_smoke.sh`, `bench_gateway.sh`, `test_body_cap.sh`

---

## How to run it (day-to-day)

### Quick boot (dev routes on)

```
SVC_GATEWAY_DEV_ROUTES=1 SVC_GATEWAY_DEV_METRICS=1 \
crates/svc-gateway/scripts/run_gateway.sh
```

Outputs `/healthz`, `/readyz`, metrics head; writes PID to `target/gateway.pid`.

Stop:

```
crates/svc-gateway/scripts/stop_gateway.sh
```

### Manual pokes

```
curl -is http://127.0.0.1:5304/healthz
curl -is http://127.0.0.1:5304/readyz
curl -s  http://127.0.0.1:5304/version | jq .
curl -s  http://127.0.0.1:5304/metrics | head -n 40
```

---

## Tests & smokes

### Rate-limit sanity (deterministic)

```
SVC_GATEWAY_DEV=1 SVC_GATEWAY_DEV_METRICS=1 \
crates/svc-gateway/scripts/sanity_rate_limit.sh
```

Expect: 200, then 429, 429 and counters:

* `gateway_http_requests_total{route="dev_rl",status="200"} == 1`
* `gateway_http_requests_total{route="dev_rl",status="429"} == 2`
* `gateway_rejections_total{reason="rate_limit"} == 2`

### Body-cap smoke

```
crates/svc-gateway/scripts/test_body_cap.sh
```

Expect: small payload → 200, oversize → 413, and `gateway_rejections_total{reason="body_cap"} >= 1`.

### Decode-guard smoke

```
crates/svc-gateway/scripts/decode_guard_smoke.sh
```

Expect:

* gzip encoded body → 415 (or 413 if decoded cap trips)
* stacked encodings → 415
* declared > cap → 413
  Metrics: `decode_cap` and `stacked_encoding` increment.

---

## Benchmarking

Pre-req: `wrk` (or `hey`) installed locally.

Fast 10s run (matches what you posted):

```
THREADS=4 CONNS=64 DUR=10s \
SVC_GATEWAY_DEV_ROUTES=1 SVC_GATEWAY_DEV_METRICS=1 SVC_GATEWAY_DEV_READY=1 \
crates/svc-gateway/benches/bench_gateway.sh
```

Full 30s baseline (on power, recommended):

```
THREADS=8 CONNS=256 DUR=30s \
SVC_GATEWAY_DEV_ROUTES=1 SVC_GATEWAY_DEV_METRICS=1 SVC_GATEWAY_DEV_READY=1 \
crates/svc-gateway/benches/bench_gateway.sh
```

Read service times from Prometheus histograms:

* Service time (seconds) ≈ `gateway_request_latency_seconds_sum{route="X"}` / `..._count{route="X"}`
* Multiply by 1e6 for µs.

Expected ballpark on your 2019 i5:

* `/healthz` ~51–52k RPS; `/readyz` (guards on) ~48k; `/dev/rl` ~50k (mostly 429).
* Hist-avg service time: ~20–40 µs depending on route.

---

## Ship flow

Build + run:

```
cargo fmt -p svc-gateway
cargo clippy -p svc-gateway --no-deps -- -D warnings
cargo build -p svc-gateway
```

Version check:

```
SVC_GATEWAY_DEV_ROUTES=1 SVC_GATEWAY_DEV_METRICS=1 \
crates/svc-gateway/scripts/run_gateway.sh

curl -s http://127.0.0.1:5304/version | jq .
# built_at_unix should be non-zero
```

Tag & push:

```
git add -A
git commit -m "svc-gateway: beta"
git tag -a svc-gateway-v0.1.0-beta -m "svc-gateway beta"
git push && git push --tags
```

Release build:

```
cargo build -p svc-gateway --release
```

---

## Troubleshooting (fast)

* **`/version` shows `built_at_unix: 0`**
  Ensure `crates/svc-gateway/build.rs` exists and `Cargo.toml` has `build = "build.rs"`. Then:

  ```
  cargo clean -p svc-gateway
  cargo build -p svc-gateway
  ```
* **Scripts say “Couldn’t connect to 127.0.0.1:5304” briefly**
  That’s the exponential probe; it keeps trying until ready. If it never flips, `tail -n 200 target/run-logs/gateway.log`.
* **No dev routes visible**
  `SVC_GATEWAY_DEV_ROUTES=1` must be set in the same shell that launches the process.
* **Bench variance**
  Plug in power; close background CPU hogs; keep `wrk` and gateway on the same host; pin `THREADS/CONNS` as above.

---

## Beta acceptance (met)

* Clippy pedantic: `cargo clippy -p svc-gateway -- -D warnings` ✅
* Deterministic smokes for RL/body/decode ✅
* Bench baselines at/near your posted numbers ✅
* `/healthz`, `/readyz`, `/metrics`, `/version` stable & documented in code ✅
* Build stamp present (`built_at_unix > 0`) ✅

---

## Nice-to-have (post-beta, low risk)

1. **Readiness error-rate gate (toggle-off default)**

   * Env: `SVC_GATEWAY_READY_ERR_WINDOW_S`, `SVC_GATEWAY_READY_ERR_THRESHOLD_PCT`
   * If recent 429/413/415% ≥ threshold, return 503 from `/readyz`.

2. **CI “fortress”** (copy from other crates)

   * Jobs: fmt, clippy, test, cargo-deny, cargo-audit (+ optional geiger).
   * Optional perf guard: run `bench_gateway.sh` on label/nightly, assert ≤10% regression.

3. **Containerization + unit file**

   * Slim Dockerfile (distroless or alpine), health/readiness endpoints in compose/k8s.
   * Example systemd unit with hardening flags (no new privs, read-only FS, seccomp).

4. **Admin niceties**

   * `/metrics` on a separate bind (optional), `/version` in boot logs, `/readyz` verbosity knob.

5. **Policy hook (future)**

   * Plug a policy gate before dev routes (or before future public routes) to demo the composition story (ron-policy).

---

## Future (beyond gateway)

* When `svc-gateway` fronts **omnigate** or other services, keep the current invariants:

  * No ambient authority; caps enforced close to ingress
  * Predictable rejection paths with metrics (`gateway_rejections_total`)
  * Minimal per-route layer stacks; only opt-in metrics where needed for benches

---

## Quick appendix — examples

**/version**

```
curl -s http://127.0.0.1:5304/version | jq .
{
  "name": "svc-gateway",
  "version": "0.1.0",
  "built_at_unix": 1762362641
}
```

**rate-limit sanity**

```
SVC_GATEWAY_DEV=1 SVC_GATEWAY_DEV_METRICS=1 \
crates/svc-gateway/scripts/sanity_rate_limit.sh
# hit 1 -> 200
# hit 2 -> 429
# hit 3 -> 429
```

**decode guard**

```
printf hi | gzip | curl -s -i -X POST --data-binary @- \
  -H 'Content-Encoding: gzip' http://127.0.0.1:5304/dev/echo | sed -n '1,20p'
# 413 decoded_cap (or 415 encoded_body_unsupported depending on cap)
```

---


### END NOTE - NOVEMBER 5 2025 - 11:17 CST






### BEGIN NOTE - NOVEMBER 22 2025 - 19:42 CST 

# 🧅 **svc-gateway — Wrap-Up Summary (Completed Work)**

**Status:** Fully functional App-Plane Gateway · All Tests Green · Clippy Pedantic Clean
**Date:** Latest Completion

The **svc-gateway** crate is now a fully operational, production-grade HTTP ingress layer for the RustyOnions cluster. It sits at the system’s edge and forwards all external **/app/*** traffic into Omnigate’s app-plane while enforcing the correct RON-CORE architecture, invariants, and future-forward app contract requirements.

This wrap-up summarizes what we accomplished in this crate.

---

# ✅ 1. **App Plane Hop — Fully Implemented**

### ✔ End-to-end /app/* → /v1/app/* proxy

The gateway now forwards all application-level traffic to Omnigate exactly as defined in the App Contract:

```
client → svc-gateway (/app/*)
       → ingress stack
       → omnigate (/v1/app/*)
       → app code via ron-app-sdk
```

### ✔ Method preservation

All HTTP verbs preserved:

* GET
* POST
* PUT
* DELETE
* PATCH

### ✔ Query string preservation

We fixed Axum Path extractor behavior and now forward the original query string correctly:

```
/app/kv/get?foo=1 → /v1/app/kv/get?foo=1
```

### ✔ Body passthrough

Binary-safe body forwarding using `Bytes`.

### ✔ Header propagation

Critical headers pass through untouched:

* `Authorization`
* `X-RON-Token`
* `X-RON-Passport`
* `X-Correlation-ID`
* `X-Request-Id` (generated if missing)

### ✔ Hop-by-hop header filtering

We correctly skip:

* Host
* Connection
* TE
* Trailer
* Transfer-Encoding
* Upgrade

Matching HTTP/1.1 RFC rules.

---

# ✅ 2. **Transport Failure Semantics**

We implemented correct error classification:

### ✔ Upstream 4xx / 5xx bodies pass through unchanged

If Omnigate returns:

* 400 Problem JSON
* 403 Problem JSON
* 500 Problem JSON

the gateway returns **exactly** the same status + body back to the client.

### ✔ Transport failures (connect/read) → 502 RON Problem JSON

When upstream is unreachable, connection is refused, or upstream read fails:

```
502 Bad Gateway
{
  "code": "upstream_unavailable",
  "message": "...",
  "retryable": true
}
```

This is the correct RON-CORE SDK-facing behavior.

---

# ✅ 3. **Ingress Pipeline Integration**

svc-gateway now applies the cluster ingress stack to /app/*:

* **Correlation Layer**
  Injects or preserves `x-request-id` and `x-correlation-id`, binds spans.

* **HTTP Metrics Layer**
  Produces:

  * `gateway_http_requests_total`
  * `gateway_http_request_duration_seconds`

* **Readiness Gate**
  If degraded: `/app/* → 503`.

* **Body Caps** (for dev routes)

* **Rate Limit Layer** (for dev routes)

* **Timeout Layer** (for readiness paths)

These layers make `/app/*` safe, observable, and consistent with RON-CORE node standards.

---

# ✅ 4. **Config Integration**

We added clean environment-driven discovery:

### ✔ `SVC_GATEWAY_OMNIGATE_BASE_URL`

Defines where app-plane requests are forwarded.

### ✔ `SVC_GATEWAY_BIND_ADDR`

Defines the public listener.

### ✔ Works with `Config::load()`

All config merges preserve RON-CORE’s blueprint architecture.

---

# ✅ 5. **Robust Integration Test Suite**

We added a complete test harness:

### ✔ Dummy Omnigate server

Implements:

* ping
* headers echo
* method echo
* body echo
* problem400 / problem403 / problem500

### ✔ Full /app/* coverage

Tests verify:

* happy-path forwarding
* header propagation
* method round-trip
* body + query preservation
* problem envelope passthrough
* 502 on upstream failure

### ✔ Race-free startup

We hardened the test harness with readiness polling to avoid intermittent failures.

### ✔ Metrics registration fixed

We introduced a test-only `OnceCell` guard to avoid Prometheus duplicate registration panics.

Result:
**5/5 app-proxy tests passing consistently.**

---

# ✅ 6. Clippy Pedantic Cleanliness

We satisfied all strict RON-CORE lint rules:

* `clippy::all`
* `clippy::pedantic`
* `#![forbid(unsafe_code)]`

Final cleanup included:

* renaming `url` → `upstream_url` to avoid `similar_names`
* ensuring `#[must_use]` correctness
* removing unused imports
* no panics or expects in core paths
* stable routing semantics

---

# 🎉 Final Status: svc-gateway Complete

The **svc-gateway crate is now fully finished for the App Plane milestone.**

You now have:

* A verified and reliable ingress surface
* Correct transport semantics for SDKs
* Fully tested forwarding behavior
* Observability hooks and readiness gates
* A stable foundation for **ron-app-sdk-ts**

svc-gateway is production-grade and ready for the next phase of RON-CORE development.

---


### END NOTE - NOVEMBER 22 2025 - 19:42 CST




### BEGIN NOTE - APRIL 29 2026 - 01:00 CST

# NOTE — `svc-gateway` Session Notes

## Current status

`svc-gateway` is now a real **edge-facing WEB3 paid storage ingress**. In this session it moved from general app/admin gateway behavior into a working product-facing route surface for both paid-storage preflight and paid-storage write execution:

```text
GET  /paid/o/estimate
POST /paid/o
PUT  /paid/o
```

The important architectural correction we made is this:

```text
client
→ svc-gateway
→ omnigate
→ svc-storage
```

`svc-gateway` no longer bypasses `omnigate` for paid storage product paths. That means `svc-gateway` remains the edge ingress, `omnigate` remains the BFF/product layer, and `svc-storage` remains the paid-use enforcement point.

The final live smoke proved the whole chain:

```text
svc-gateway
→ omnigate
→ svc-storage
→ svc-wallet
→ ron-accounting
```

The smoke completed product-facing estimate, wallet issue/hold, gateway paid write, wallet capture/release, accounting export, object readback, and replay rejection. The final proof line was `WEB3 edge paid storage write smoke green`, with `capture_minor: 84`, `release_minor: 16`, `accounting_rows: 3`, and `replay_rejection: 402 payment_required`. 

---

# 1. What `svc-gateway` is responsible for now

For WEB3 paid storage, `svc-gateway` now owns the public edge route shape:

```text
/paid/o/estimate
/paid/o
```

Its responsibilities are:

```text
1. Expose short, client-facing paid storage routes.
2. Forward paid estimate requests to omnigate.
3. Forward paid write requests to omnigate.
4. Preserve selected authorization/payment/accounting/correlation headers.
5. Preserve body bytes for paid writes.
6. Preserve query strings for estimates.
7. Pass successful upstream JSON bodies through unchanged.
8. Pass upstream app errors through unchanged.
9. Convert omnigate transport failures into 502 Problem JSON.
10. Continue applying gateway correlation/metrics middleware to product paths.
```

Its responsibilities are **not**:

```text
1. No direct wallet mutation.
2. No ledger mutation.
3. No storage-byte persistence.
4. No pricing calculation.
5. No wallet receipt verification.
6. No accounting export.
7. No CID derivation.
8. No paid context-idem derivation.
```

This keeps the stack modular:

```text
svc-gateway:
  edge ingress, correlation, routing, coarse product path exposure

omnigate:
  BFF/product path and future passport/asset UX layer

svc-storage:
  paid storage enforcement, body hashing, b3 CID, wallet receipt verification,
  capture/release, accounting event export

svc-wallet:
  balance mutation, committed receipts, idempotency

ron-accounting:
  usage events and snapshots
```

---

# 2. Files changed or materially involved this session

## Direct `svc-gateway` files

```text
crates/svc-gateway/src/config/env.rs
crates/svc-gateway/src/config/mod.rs
crates/svc-gateway/src/state.rs
crates/svc-gateway/src/routes/mod.rs
crates/svc-gateway/src/routes/paid_storage.rs
crates/svc-gateway/tests/paid_storage_estimate_proxy.rs
crates/svc-gateway/tests/paid_storage_write_proxy.rs
```

## Existing `svc-gateway` files exercised indirectly

```text
crates/svc-gateway/src/routes/app.rs
crates/svc-gateway/src/routes/health.rs
crates/svc-gateway/src/routes/ready.rs
crates/svc-gateway/src/routes/dev.rs
crates/svc-gateway/src/observability/*
crates/svc-gateway/src/layers/*
crates/svc-gateway/tests/app_proxy.rs
crates/svc-gateway/tests/smoke.rs
```

## Root scripts that now exercise `svc-gateway`

```text
scripts/web3_paid_estimate_edge_smoke.sh
scripts/web3_paid_storage_edge_write_smoke.sh
```

The most important one is:

```text
scripts/web3_paid_storage_edge_write_smoke.sh
```

It starts the real service chain and proves `svc-gateway` can act as the public product ingress for paid storage.

---

# 3. Major accomplishment: added `/paid/o/estimate`

## What was added

`svc-gateway` now exposes:

```text
GET /paid/o/estimate?bytes=N
```

This route proxies to:

```text
omnigate /v1/paid/o/estimate?bytes=N
```

and `omnigate` then proxies to:

```text
svc-storage /paid/o/estimate?bytes=N
```

## Why this matters

This is the public-facing preflight route for paid storage. A dashboard, app, CLI, or future browser extension can ask the edge layer:

```text
How much will this storage write cost?
```

without knowing about:

```text
omnigate port
svc-storage port
ron-policy config
internal pricing action
storage implementation details
```

The route is read-only. It does not create a wallet hold, mutate ledger state, store object bytes, or export accounting.

## Proven behavior

The route was proven in three ways:

```text
1. Gateway integration tests using a dummy omnigate.
2. Estimate-only live edge smoke.
3. Full edge paid-write live smoke.
```

In the final live smoke, `svc-gateway /paid/o/estimate?bytes=48` returned:

```json
{
  "schema": "svc-storage.paid-storage-estimate.v1",
  "route": "/paid/o",
  "action": "paid_storage_put",
  "asset": "roc",
  "bytes": 48,
  "amount_minor": "84",
  "minimum_hold_minor": "84",
  "pricing_mode": "roc-economics",
  "economics_policy_path": "configs/roc-economics.toml"
}
```

That proves the full edge estimate path:

```text
svc-gateway
→ omnigate
→ svc-storage
→ configs/roc-economics.toml
```

with the expected 48-byte price of 84 ROC minor units. 

---

# 4. Major accomplishment: changed estimate routing to go through `omnigate`

At first, the gateway estimate proxy directly targeted `svc-storage`.

That worked technically, but it was not the clean WEB3 product boundary. We corrected it so the route now goes through `omnigate`:

```text
client
→ svc-gateway /paid/o/estimate
→ omnigate /v1/paid/o/estimate
→ svc-storage /paid/o/estimate
```

This is important because future WEB3_2 product behavior belongs in the BFF layer:

```text
passport context
wallet-linked permissions
asset page hydration
paid access UX
crab:// flow support
dashboard response shaping
```

If gateway bypasses omnigate, the product layer gets split and messy. After this session, gateway paid-storage routes are now aligned with the intended architecture.

The gateway estimate proxy tests stayed green after the switch to omnigate: `paid_storage_estimate_proxy_happy_path`, `paid_storage_estimate_proxy_passes_omnigate_errors_through`, and `paid_storage_estimate_upstream_connect_failure_yields_problem_502` all passed. 

---

# 5. Major accomplishment: added `/paid/o` write route

## What was added

`svc-gateway` now exposes:

```text
POST /paid/o
PUT  /paid/o
```

These proxy to:

```text
omnigate /v1/paid/o
```

and `omnigate` proxies to:

```text
svc-storage /paid/o
```

## Why this matters

This gives clients a single public edge endpoint for paid storage execution:

```text
POST /paid/o
```

The client no longer needs to talk directly to `svc-storage`, and it no longer needs to know the internal topology.

## What it forwards

The gateway write route forwards:

```text
Authorization
Accept
Content-Type
x-ron-*
x-correlation-id
x-request-id
Idempotency-Key
body bytes
HTTP method POST/PUT
```

It intentionally filters:

```text
Host
Connection
Proxy-Authorization
TE
Trailer
Transfer-Encoding
Upgrade
Content-Length
```

Filtering `Content-Length` is correct because `reqwest` recalculates the length after the body is forwarded.

## What it does not do

The gateway write route does not:

```text
compute b3 CID
validate wallet receipts
create wallet holds
capture/release ROC
export accounting
store bytes
derive context-idem
price storage
```

Those remain downstream responsibilities.

---

# 6. Error behavior

`svc-gateway` now has stable paid-storage proxy behavior.

## Upstream application errors

If `omnigate` returns an application error, gateway passes it through.

Examples:

```text
400 bad_request
402 payment_required
```

This matters because `svc-storage` owns payment/security semantics and `omnigate` should preserve them. Gateway should not reinterpret storage’s economic failures.

## Upstream transport failure

If gateway cannot connect to `omnigate`, it returns a gateway Problem JSON via the existing `errors::upstream_unavailable` path.

The paid write proxy test covers:

```text
SVC_GATEWAY_OMNIGATE_BASE_URL=http://127.0.0.1:1
POST /paid/o
→ 502
→ code = upstream_unavailable
→ retryable = true
```

The final gateway write proxy tests prove the transport failure case stayed green. 

---

# 7. Tests added for `svc-gateway`

## `paid_storage_estimate_proxy.rs`

This file now proves:

```text
1. GET /paid/o/estimate forwards to omnigate /v1/paid/o/estimate.
2. Query string is preserved.
3. Successful JSON response is passed through.
4. Authorization/x-ron/correlation headers are preserved.
5. Gateway correlation layer generates x-request-id where expected.
6. Omnigate-side bad request passes through as 400.
7. Omnigate unavailable maps to 502 Problem JSON.
8. Hop-by-hop headers are not forwarded.
```

The final passing run showed:

```text
paid_storage_estimate_proxy_happy_path ... ok
paid_storage_estimate_proxy_passes_omnigate_errors_through ... ok
paid_storage_estimate_upstream_connect_failure_yields_problem_502 ... ok
```



## `paid_storage_write_proxy.rs`

This file now proves:

```text
1. POST /paid/o forwards to omnigate /v1/paid/o.
2. PUT /paid/o forwards to omnigate /v1/paid/o.
3. Body bytes are preserved exactly.
4. Authorization header is preserved.
5. Idempotency-Key is preserved.
6. x-ron payment headers are preserved.
7. x-ron wallet receipt headers are preserved.
8. x-ron accounting context headers are preserved.
9. Gateway-generated x-request-id reaches upstream.
10. Omnigate-side 400 passes through unchanged.
11. Omnigate unavailable maps to 502 Problem JSON.
12. Hop-by-hop headers are not forwarded.
```

The final passing run showed:

```text
paid_storage_write_proxy_post_happy_path ... ok
paid_storage_write_proxy_put_happy_path ... ok
paid_storage_write_proxy_passes_omnigate_errors_through ... ok
paid_storage_write_upstream_connect_failure_yields_problem_502 ... ok
```



## `app_proxy.rs` regression test

Existing app proxy tests remained green:

```text
app_proxy_upstream_connect_failure_yields_problem_502
app_proxy_happy_path
app_proxy_preserves_auth_and_corr_headers_and_generates_request_id
app_proxy_preserves_methods_body_and_query
app_proxy_passes_problem_bodies_unchanged
```

This proved the new paid-storage routes did not regress the existing `/app/*` route behavior. The app proxy tests are shown passing in the final gateway test run. 

## `smoke.rs`

The basic `svc-gateway` smoke test remained green, proving the crate’s baseline test surface still passes after the new paid routes. 

---

# 8. Clippy/test fixes completed during the session

## 8.1 Dependency clippy issue

The first full gateway clippy command without `--no-deps` failed because path dependencies `ron-kernel` and `ron-metrics` have pre-existing clippy/pedantic lint debt. That failure was not caused by the gateway paid-route changes. 

The focused gate used:

```bash
cargo clippy -p svc-gateway --all-targets --no-deps -- -D warnings
```

That is the right crate-focused clippy command until the dependency lint debt is cleaned up.

## 8.2 `doc_markdown` fixes

Gateway has `#![deny(clippy::all, clippy::pedantic)]`, so RO headers must satisfy pedantic doc lint rules. Initial comments in `state.rs`, `config/mod.rs`, `config/env.rs`, `routes/mod.rs`, and `routes/paid_storage.rs` triggered missing-backtick `doc_markdown` errors. The output showed clippy flagging things like `config::Config`, `observability::metrics`, `readiness::ReadyState`, test filenames, and env vars as needing backticks. 

We fixed the headers by wrapping code-like names in backticks:

```text
`config::Config`
`observability::metrics`
`readiness::ReadyState`
`SVC_GATEWAY_OMNIGATE_BASE_URL`
`tests/paid_storage_estimate_proxy.rs`
```

## 8.3 Removed unused `put` imports

The route uses:

```rust
post(write).put(write)
```

That does not require importing `put`.

Clippy rejected the unused import:

```rust
routing::{get, post, put}
```

We removed `put` from route/test imports and left:

```rust
routing::{get, post}
```

## 8.4 `manual_let_else` fix

Clippy rejected a `match` used to convert Axum `Method` into `reqwest::Method`:

```rust
let reqwest_method = match reqwest::Method::from_bytes(...) {
    Ok(method) => method,
    Err(_) => return errors::upstream_unavailable("bad_method"),
};
```

Gateway is pedantic-clean, so we rewrote it as:

```rust
let Ok(reqwest_method) = reqwest::Method::from_bytes(method.as_str().as_bytes()) else {
    return errors::upstream_unavailable("bad_method");
};
```

## 8.5 Fixed POST body-length expectation

The gateway paid-write proxy POST body:

```text
RustyOnions paid write through gateway
```

has length:

```text
38
```

The initial test expected `37`. The test correctly failed with actual `38`, and we updated the expected value. The final gateway write proxy test then passed.  

---

# 9. Config/state changes

## `config::Upstreams`

`svc-gateway` now has upstream config for both:

```text
omnigate_base_url
storage_base_url
```

The current paid-storage route uses `omnigate_base_url`.

`storage_base_url` remains in config because the earlier direct-storage proxy existed and because it may still be useful for diagnostics or future non-product storage admin paths. But product paid-storage routes should continue to go through `omnigate`.

## Env vars

The gateway env loader now supports:

```text
SVC_GATEWAY_BIND_ADDR
BIND_ADDR
SVC_GATEWAY_MAX_BODY_BYTES
SVC_GATEWAY_DECODE_ABS_CAP_BYTES
SVC_GATEWAY_OMNIGATE_BASE_URL
SVC_GATEWAY_STORAGE_BASE_URL
```

For WEB3 product routing, the important one is:

```text
SVC_GATEWAY_OMNIGATE_BASE_URL
```

The final live smoke used gateway bound on:

```text
127.0.0.1:15304
```

and omnigate upstream:

```text
http://127.0.0.1:15305
```

The final live smoke summary printed:

```text
gateway estimate: http://127.0.0.1:15304/paid/o/estimate?bytes=48
gateway write:    http://127.0.0.1:15304/paid/o
```



## `AppState`

`AppState` now includes a shared `reqwest::Client` for omnigate and one for storage:

```text
omnigate_client
storage_client
```

The paid-storage product path uses:

```text
omnigate_client
```

This keeps clients reusable and avoids rebuilding HTTP clients per request.

---

# 10. Route inventory after this session

## Gateway paid estimate

```text
GET /paid/o/estimate?bytes=N
```

Proxy target:

```text
omnigate GET /v1/paid/o/estimate?bytes=N
```

Behavior:

```text
read-only
preserve query
preserve selected headers
pass success body through
pass upstream app errors through
transport failure -> 502
```

## Gateway paid write

```text
POST /paid/o
PUT  /paid/o
```

Proxy target:

```text
omnigate POST /v1/paid/o
omnigate PUT  /v1/paid/o
```

Behavior:

```text
preserve body bytes
preserve x-ron payment headers
preserve wallet receipt headers
preserve accounting context headers
preserve idempotency key
pass success body through
pass upstream app errors through
transport failure -> 502
```

## Existing gateway app proxy

Still supported:

```text
/app/*
```

The app proxy regression tests stayed green.

---

# 11. Live smoke proof involving `svc-gateway`

## Estimate edge smoke

The estimate edge smoke proved:

```text
svc-gateway /paid/o/estimate
→ omnigate /v1/paid/o/estimate
→ svc-storage /paid/o/estimate
→ configs/roc-economics.toml
→ 48 bytes = 84 ROC minor units
```

This established that gateway can expose a product-facing preflight route.

## Full edge paid-write smoke

The final edge write smoke proved the full path:

```text
svc-gateway /paid/o/estimate
→ omnigate /v1/paid/o/estimate
→ svc-storage /paid/o/estimate

svc-wallet issue
svc-wallet hold

svc-gateway /paid/o
→ omnigate /v1/paid/o
→ svc-storage /paid/o
→ svc-wallet capture/release
→ ron-accounting export
```

The final smoke proved:

```text
CID: b3:730812d549a71a900fba05b821b29c440e9b32c21a51e54ecbc3af7eb6132b57
object bytes: 48
capture_minor: 84
release_minor: 16
payer_balance: 16
payee_balance: 84
accounting_rows: 3
replay_rejection: 402 payment_required
```



This is the strongest proof that `svc-gateway` is now a real WEB3 edge ingress.

---

# 12. Current `svc-gateway` WEB3 readiness estimate

For `svc-gateway` specifically:

```text
Paid storage estimate edge route: 95–98%
Paid storage write edge route:    94–97%
WEB3 paid-storage edge layer:     93–96%
General gateway readiness:        88–92%
WEB3_2 product UX readiness:      65–72%
```

Why the paid-storage edge layer is high:

```text
1. Estimate route exists.
2. Write route exists.
3. Both route through omnigate.
4. Tests cover success and failure paths.
5. App proxy regression stayed green.
6. Live multi-service smoke proved real end-to-end execution.
```

Why WEB3_2 remains lower:

```text
1. Passport routes are not implemented.
2. Wallet-linked passport permissions are not wired.
3. Typed b3 asset pages are not wired through gateway.
4. crab:// UX routing is not implemented.
5. Paid content/site/app access beyond storage is not implemented.
6. Product response hydration needs polish.
```

---

# 13. What is complete for `svc-gateway`

## Complete or beta-complete

```text
1. Edge-facing paid estimate route added.
2. Edge-facing paid write route added.
3. Paid estimate now goes through omnigate.
4. Paid write now goes through omnigate.
5. Header forwarding allowlist implemented.
6. Hop-by-hop headers filtered.
7. Body pass-through for POST/PUT proven.
8. Query-string preservation proven.
9. Omnigate app errors pass through.
10. Omnigate transport failure maps to 502.
11. Gateway correlation/request-id behavior preserved.
12. App proxy regression tests remain green.
13. Gateway smoke test remains green.
14. Full live edge paid-storage smoke proves the route in real service topology.
```

## Current proof quality

`svc-gateway` now has:

```text
1. Focused paid estimate proxy tests.
2. Focused paid write proxy tests.
3. Existing app proxy regression tests.
4. Basic smoke test.
5. Full live multi-service WEB3 edge write smoke.
```

That is strong beta-level proof for the paid-storage edge path.

---

# 14. What remains for `svc-gateway`

## 14.1 Product response shaping

Right now `svc-gateway` passes through storage/omnigate responses mostly unchanged.

For a polished beta UX, gateway may eventually need a stable external envelope.

Potential future response:

```json
{
  "schema": "svc-gateway.paid-storage-write.v1",
  "cid": "b3:<hash>",
  "asset": "roc",
  "charged_minor": "84",
  "released_minor": "16",
  "payment_status": "settled",
  "accounting_status": "exported",
  "receipts": {
    "capture": "b3:<hash>",
    "release": "b3:<hash>"
  },
  "links": {
    "read": "/o/b3:<hash>",
    "asset": "crab://b3/<hash>.asset"
  }
}
```

This should be considered carefully. Gateway may remain pure pass-through and leave shaping to omnigate. If shaping is added, it should be versioned and not break existing storage response tests.

## 14.2 Live route documentation

The project should document the new route chain:

```text
GET  /paid/o/estimate?bytes=N
POST /paid/o
PUT  /paid/o
```

with the internal targets:

```text
/v1/paid/o/estimate on omnigate
/v1/paid/o on omnigate
/paid/o/estimate on svc-storage
/paid/o on svc-storage
```

Add examples for:

```text
estimate
wallet hold
paid write
readback
replay rejection
```

## 14.3 Config cleanup

`svc-gateway` now carries both:

```text
omnigate_base_url
storage_base_url
```

For product paid-storage routes, `omnigate_base_url` is correct. The codebase should clarify whether `storage_base_url` remains for diagnostics/admin or should be removed later.

Recommended path:

```text
Keep storage_base_url for now.
Document that product WEB3 routes should not use it.
Audit future code so gateway does not bypass omnigate.
```

## 14.4 Gateway-to-omnigate timeout policy

The current route uses the shared `reqwest::Client` default behavior.

Future hardening should add:

```text
connect timeout
request timeout
body size caps for paid writes
read timeout
retry policy for estimates
no blind retry for paid writes unless idempotency is fully guaranteed
upstream latency metrics
```

Paid writes should be treated carefully because they can trigger wallet settlement downstream.

## 14.5 Route-specific metrics

The route currently inherits gateway HTTP metrics/correlation layers, but route-specific WEB3 metrics would help beta dashboards.

Suggested metrics:

```text
gateway_paid_estimate_requests_total
gateway_paid_estimate_upstream_errors_total
gateway_paid_write_requests_total
gateway_paid_write_upstream_errors_total
gateway_paid_write_bytes_total
gateway_paid_write_status_total
gateway_paid_storage_proxy_latency_seconds
```

Labels should be bounded:

```text
route
status_class
method
upstream = omnigate
```

Avoid high-cardinality labels like CID, account, or txid.

## 14.6 Passport/capability integration

WEB3_2 will need gateway-visible passport flows. Gateway may become the first place clients present:

```text
RON Passport
wallet-linked permission token
anonymous passport token
site/app authorization token
```

Gateway may not validate everything itself, but it should preserve or normalize the headers needed by omnigate.

Future work:

```text
1. Define canonical passport headers.
2. Preserve them through gateway.
3. Add tests proving they reach omnigate.
4. Add rejection behavior if gateway must enforce coarse auth.
5. Avoid putting passport-specific business logic into low-level gateway plumbing.
```

## 14.7 Typed b3 / crab route ingress

WEB3_2 expects URLs such as:

```text
crab://b3/<hash>.image
crab://b3/<hash>.song
crab://b3/<hash>.article
crab://b3/<hash>.video
crab://b3/<hash>.comment
```

Gateway may need HTTP-compatible ingress routes for the browser extension/dashboard while `crab://` support matures.

Possible future routes:

```text
GET /b3/:hash
GET /b3/:hash.:kind
GET /crab/b3/:hash
GET /asset/b3/:hash
```

These should route to `omnigate`, not `svc-storage`, because asset pages require metadata, manifests, ownership, payout routing, and provenance—not raw byte storage only.

## 14.8 Paid content/site/app access

Beyond paid storage, gateway will likely expose:

```text
GET  /paid/content/:id/estimate
POST /paid/content/:id/view
GET  /paid/site/:id/estimate
POST /paid/site/:id/access
GET  /paid/app/:id/estimate
POST /paid/app/:id/access
```

These should route to `omnigate` or the relevant product service, not directly to ledger/wallet/storage. Gateway should remain edge ingress.

## 14.9 CI gate strategy

Current dependency lint debt means full clippy without `--no-deps` can still fail due to `ron-kernel` / `ron-metrics` lint issues. For gateway-focused work, use:

```bash
cargo clippy -p svc-gateway --all-targets --no-deps -- -D warnings
```

Future cleanup should eventually make this pass:

```bash
cargo clippy -p svc-gateway --all-targets -- -D warnings
```

but that requires cleaning dependency crates, not only gateway.

---

# 15. Developer notes for next session

## Do not regress these invariants

```text
1. Gateway paid-storage routes must go through omnigate.
2. Gateway must not call svc-wallet directly for paid storage.
3. Gateway must not call ron-ledger directly.
4. Gateway must not implement pricing.
5. Gateway must not validate storage wallet receipts.
6. Gateway must preserve body bytes exactly.
7. Gateway must preserve x-ron payment/accounting headers.
8. Gateway must filter hop-by-hop headers.
9. Gateway must preserve query strings on estimate routes.
10. Gateway must pass through upstream application errors.
11. Gateway must map transport failure to 502 upstream_unavailable.
12. Existing /app proxy behavior must not regress.
```

## Important env vars

```text
SVC_GATEWAY_BIND_ADDR
BIND_ADDR
SVC_GATEWAY_OMNIGATE_BASE_URL
SVC_GATEWAY_STORAGE_BASE_URL
SVC_GATEWAY_MAX_BODY_BYTES
SVC_GATEWAY_DECODE_ABS_CAP_BYTES
SVC_GATEWAY_DEV_ROUTES
SVC_GATEWAY_DEV_METRICS
```

For current WEB3 paid storage product path:

```text
SVC_GATEWAY_OMNIGATE_BASE_URL=http://127.0.0.1:15305
```

## Important routes

```text
GET  /paid/o/estimate?bytes=N
POST /paid/o
PUT  /paid/o
```

## Important upstream routes

```text
GET  omnigate /v1/paid/o/estimate?bytes=N
POST omnigate /v1/paid/o
PUT  omnigate /v1/paid/o
```

## Important proof values from this session

```text
object bytes: 48
CID: b3:730812d549a71a900fba05b821b29c440e9b32c21a51e54ecbc3af7eb6132b57
gateway estimate amount: 84
hold amount: 100
capture amount: 84
release amount: 16
payer final balance: 16
payee final balance: 84
accounting rows: 3
replay rejection: 402 payment_required
```

---

# 16. Suggested next `svc-gateway` work

## Option A — Route documentation and examples

Document the new route flow clearly for beta users and future browser-extension work.

High value because the current path is now real and demoable.

## Option B — Gateway/omnigate product response contract

Decide whether gateway should remain pure pass-through or whether omnigate should shape product responses. My recommendation:

```text
gateway remains mostly pass-through
omnigate owns product response shaping
gateway documents stable external path
```

## Option C — Add route-specific metrics

Add bounded metrics for paid estimate/write traffic.

This is useful for dashboards and beta observability.

## Option D — Add request body caps to paid write route

Ensure `/paid/o` inherits or explicitly applies appropriate body-size limits.

This matters before exposing beta ingress more broadly.

## Option E — Start passport header preservation tests

Before implementing passport validation, add tests proving gateway preserves future passport headers to omnigate.

Example headers:

```text
x-ron-passport
x-ron-wallet-account
x-ron-permission
x-ron-spend-limit
```

## Option F — Add b3/crab route ingress stubs

Add edge routes that forward typed asset page requests to omnigate.

This begins WEB3_2 route work without putting asset logic into gateway.

---

# 17. Bottom line

`svc-gateway` is now a functioning WEB3 edge ingress for paid storage.

It now supports:

```text
paid storage estimate through edge
paid storage write through edge
gateway → omnigate → storage routing
POST and PUT write proxy
authorization/payment/accounting header preservation
body preservation
query preservation
upstream error pass-through
transport 502 mapping
app proxy regression green
live multi-service edge smoke green
```

The most important accomplishment is that the public path now works:

```text
client
→ svc-gateway /paid/o/estimate
→ omnigate /v1/paid/o/estimate
→ svc-storage /paid/o/estimate

client
→ svc-gateway /paid/o
→ omnigate /v1/paid/o
→ svc-storage /paid/o
→ svc-wallet / ron-accounting
```

That is exactly the shape needed for a beta-facing “pay for storage with ROC” demo, and it sets up the next WEB3_2 work cleanly: passports, crab links, typed b3 pages, and paid content/site/app access.


### END NOTE - APRIL 29 2026 - 01:00 CST



### BEGIN NOTE - APRIL 29 2026 - 13:00 CST


Here are the **svc-gateway changelog/carry-over notes only**.

---

# `svc-gateway` — WEB3_2 Product Route Exposure / Public Ingress Notes

**Session date:** 2026-04-29
**Crate:** `crates/svc-gateway`
**Status:** Batch complete / green
**Purpose of this session:** Expose the WEB3_2 product-proof routes through the public gateway ingress and proxy them to `omnigate` without adding business logic to the gateway.

---

## 0. Summary

In this session, `svc-gateway` was extended to expose public-facing product routes for the WEB3_2 typed asset/site flow.

The gateway now exposes routes such as:

```text
GET  /crab/resolve?url=crab://<64hex>.<asset_kind>
GET  /b3/<64hex>.<asset_kind>
POST /paid/o/prepare
POST /assets/image/prepare
POST /assets/image
POST /sites/prepare
POST /sites
```

These routes proxy to `omnigate` equivalents:

```text
/v1/crab/resolve
/v1/b3/<64hex>.<asset_kind>
/v1/paid/o/prepare
/v1/assets/image/prepare
/v1/assets/image
/v1/sites/prepare
/v1/sites
```

Important boundary preserved:

```text
svc-gateway is proxy-only for this batch.
svc-gateway does not parse manifests.
svc-gateway does not hydrate asset pages.
svc-gateway does not store bytes.
svc-gateway does not price storage.
svc-gateway does not issue wallet holds.
svc-gateway does not capture or release holds.
svc-gateway does not mutate ron-ledger.
```

This completed the server-side public ingress chain:

```text
client / browser extension later
→ svc-gateway
→ omnigate
→ svc-index + svc-storage
→ typed asset page response
```

---

## 1. Files added or updated

### Updated

```text
crates/svc-gateway/src/routes/mod.rs
crates/svc-gateway/src/routes/objects.rs
crates/svc-gateway/src/routes/objects_range.rs
```

### Added

```text
crates/svc-gateway/src/routes/product.rs
crates/svc-gateway/tests/product_routes_proxy.rs
```

---

## 2. Product route module added

A new route module was added:

```text
crates/svc-gateway/src/routes/product.rs
```

This module owns gateway exposure for WEB3_2 product routes.

It is intentionally thin.

It does:

```text
accept incoming HTTP route
preserve allowed headers
preserve query strings
preserve request bodies
proxy to omnigate
copy safe upstream response headers
return upstream status/body
map upstream connection/read failure to 502 Problem
```

It does **not**:

```text
parse crab URLs
validate asset kinds
hydrate manifests
fetch object bytes
call svc-index directly
call svc-storage directly
call svc-wallet
call ron-ledger
calculate payouts
perform ownership verification
```

That keeps the gateway inside its proper lane.

---

## 3. Routes exposed

## 3.1 `GET /crab/resolve`

Gateway route:

```text
GET /crab/resolve?url=crab://<64hex>.<asset_kind>
```

Proxies to:

```text
GET /v1/crab/resolve?url=crab://<64hex>.<asset_kind>
```

Purpose:

```text
Public edge route for resolving a canonical crab asset URL.
```

Example:

```text
GET /crab/resolve?url=crab://0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.image
```

This is one of the key future browser-extension routes.

The extension can eventually translate:

```text
crab://0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.image
```

into a local gateway request:

```text
http://127.0.0.1:<gateway-port>/crab/resolve?url=crab://0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.image
```

---

## 3.2 `GET /b3/:asset`

Gateway route:

```text
GET /b3/<64hex>.<asset_kind>
```

Proxies to:

```text
GET /v1/b3/<64hex>.<asset_kind>
```

Purpose:

```text
Public HTTP equivalent of the canonical crab asset URL.
```

Example:

```text
GET /b3/0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.image
```

Important:

```text
Gateway does not validate the <64hex>.<kind> segment.
omnigate owns product parsing and deterministic rejection.
```

---

## 3.3 `POST /paid/o/prepare`

Gateway route:

```text
POST /paid/o/prepare
```

Proxies to:

```text
POST /v1/paid/o/prepare
```

Purpose:

```text
Expose paid object/storage prepare through public gateway ingress.
```

Boundary:

```text
Gateway does not create wallet holds.
Gateway does not calculate price.
Gateway does not verify receipts.
Gateway only forwards request to omnigate.
```

---

## 3.4 `POST /assets/image/prepare`

Gateway route:

```text
POST /assets/image/prepare
```

Proxies to:

```text
POST /v1/assets/image/prepare
```

Purpose:

```text
Prepare route for future paid image upload flow.
```

This is route exposure only. Full image-upload business logic remains future work.

---

## 3.5 `POST /assets/image`

Gateway route:

```text
POST /assets/image
```

Proxies to:

```text
POST /v1/assets/image
```

Purpose:

```text
Future image upload route exposure.
```

Boundary:

```text
Gateway does not hash bytes.
Gateway does not store bytes.
Gateway does not write manifests.
Gateway only proxies to omnigate.
```

---

## 3.6 `POST /sites/prepare`

Gateway route:

```text
POST /sites/prepare
```

Proxies to:

```text
POST /v1/sites/prepare
```

Purpose:

```text
Future crab://site launch prepare route exposure.
```

---

## 3.7 `POST /sites`

Gateway route:

```text
POST /sites
```

Proxies to:

```text
POST /v1/sites
```

Purpose:

```text
Future site creation / launch route exposure.
```

Boundary:

```text
Gateway does not create site manifests.
Gateway does not mutate index directly.
Gateway does not charge ROC directly.
Gateway only proxies.
```

---

## 4. Router integration

`crates/svc-gateway/src/routes/mod.rs` was updated to include:

```rust
pub mod product;
```

And product routes were mounted with the same gateway-level route layers used for other product paths:

```text
correlation middleware
HTTP metrics middleware
```

The router now merges:

```text
health/version routes
ready route
dev routes
app proxy routes
paid storage proxy routes
WEB3_2 product routes
metrics route
```

Product routes are mounted directly at the public gateway surface:

```text
/crab/resolve
/b3/:asset
/paid/o/prepare
/assets/image/prepare
/assets/image
/sites/prepare
/sites
```

---

## 5. Proxy behavior added

The product proxy implementation:

```text
builds an upstream URL from SVC_GATEWAY_OMNIGATE_BASE_URL
preserves path/query as needed
forwards selected request headers
forwards request body bytes
sends request through state.omnigate_client
copies safe upstream response headers
returns upstream status
returns upstream body
maps connect/read failures to upstream_unavailable Problem
```

Upstream base URL comes from existing gateway config:

```text
SVC_GATEWAY_OMNIGATE_BASE_URL
```

This matches the existing app/paid proxy pattern.

---

## 6. Header forwarding behavior

The product proxy forwards only selected headers.

Forwarded headers include:

```text
Authorization
Accept
Content-Type
x-ron-*
x-correlation-id
x-request-id
Idempotency-Key
```

This matters for future WEB3_2 flows because product requests may carry:

```text
x-ron-passport
x-ron-wallet-account
x-ron-permission
x-ron-spend-limit
idempotency-key
authorization
```

Hop-by-hop and unsafe/proxy headers are not forwarded.

Filtered request headers include:

```text
Host
Connection
Proxy-Authorization
TE
Trailer
Transfer-Encoding
Upgrade
Content-Length
```

This keeps the gateway proxy behavior safer and avoids leaking transport-level headers downstream.

---

## 7. Response header behavior

The product proxy copies safe upstream response headers.

Filtered response headers include:

```text
Transfer-Encoding
Content-Length
Connection
```

This prevents gateway/body mismatches and hop-by-hop leakage.

---

## 8. Error behavior

If `omnigate` cannot be reached, gateway returns a Problem response:

```text
502 Bad Gateway
code = upstream_unavailable
retryable = true
```

Reasons include:

```text
omnigate_connect
omnigate_read
bad_method
```

This matches the existing app/paid proxy error behavior.

Important:

```text
Gateway does not invent business-domain errors.
If omnigate returns a problem body, gateway passes it through.
If transport fails, gateway maps it to upstream_unavailable.
```

---

## 9. Tests added

New test file:

```text
crates/svc-gateway/tests/product_routes_proxy.rs
```

The tests spin up:

```text
dummy omnigate server
real svc-gateway router
real HTTP requests through gateway
```

This proves the gateway is not just compiling but actually proxying product routes.

---

## 9.1 `crab_resolve_proxy_preserves_query_and_headers`

Covers:

```text
GET /crab/resolve?url=...&view=asset
```

Verifies gateway proxies to:

```text
/v1/crab/resolve
```

And preserves:

```text
query url
query view
Authorization
x-ron-passport
x-ron-wallet-account
x-ron-permission
x-ron-spend-limit
x-request-id
```

Also verifies:

```text
Connection header is not forwarded.
x-request-id / correlation behavior is present.
```

This is important for the future browser-extension path.

---

## 9.2 `b3_asset_proxy_targets_omnigate_b3_route`

Covers:

```text
GET /b3/<hash>.image
```

Verifies gateway proxies to:

```text
/v1/b3/<hash>.image
```

Also verifies:

```text
method = GET
body length = 0
```

This proves the HTTP equivalent of crab asset URLs is exposed correctly.

---

## 9.3 `paid_prepare_proxy_preserves_body_and_idempotency`

Covers:

```text
POST /paid/o/prepare
```

Verifies gateway proxies to:

```text
/v1/paid/o/prepare
```

And preserves:

```text
request body
Authorization
Content-Type
Idempotency-Key
```

This protects retry-safe paid prepare flows.

---

## 9.4 `image_and_site_product_routes_target_omnigate`

Covers:

```text
POST /assets/image/prepare
POST /assets/image
POST /sites/prepare
POST /sites
```

Verifies these map to:

```text
/v1/assets/image/prepare
/v1/assets/image
/v1/sites/prepare
/v1/sites
```

Also verifies body bytes are forwarded.

This gives us public route exposure for upcoming image and site demo batches.

---

## 9.5 `product_route_upstream_connect_failure_yields_problem_502`

Covers:

```text
omnigate unavailable
```

Verifies gateway returns:

```text
502 Bad Gateway
code = upstream_unavailable
retryable = true
```

This ensures transport failure behavior is deterministic.

---

## 10. Existing stub cleanup

During strict clippy, two older stub route files failed pedantic lints.

Updated:

```text
crates/svc-gateway/src/routes/objects.rs
crates/svc-gateway/src/routes/objects_range.rs
```

Fixes applied:

```text
format!("object stub for {}", addr)
→ format!("object stub for {addr}")

format!("range read stub for {}", addr)
→ format!("range read stub for {addr}")

removed unused async from stub handlers

added #[must_use] to stub handler return functions

fixed doc markdown backticks:
headers::etag::etag_from_b3
→ `headers::etag::etag_from_b3`
```

These were not directly part of WEB3_2 product route behavior, but they were necessary to pass the strict clippy gate.

---

## 11. Product route clippy cleanup

`product.rs` also needed clippy/pedantic cleanup.

Fixes included:

```text
WEB3_2 in docs
→ `WEB3_2`

removed #[must_use] from router() because Router is already must_use

kept docs aligned with project RO header style
```

After this, the targeted gateway clippy gate passed.

---

## 12. Validation gates passed

Final `svc-gateway` gates passed:

```text
cargo fmt
cargo clippy -p svc-gateway --all-targets --no-deps -- -D warnings
cargo test -p svc-gateway --test product_routes_proxy
cargo test -p svc-gateway --all-targets
```

Final observed result:

```text
svc-gateway clippy: clean
product_routes_proxy: 5/5 passing
svc-gateway all-target tests: all passing
```

Passing test groups included:

```text
app_proxy.rs
paid_storage_estimate_proxy.rs
paid_storage_write_proxy.rs
product_routes_proxy.rs
smoke.rs
```

New product route tests:

```text
product_routes_proxy.rs    5/5 passing
```

---

## 13. Invariants preserved

This work preserved WEB3 / WEB3_2 and RustyOnions crate-boundary invariants:

```text
svc-gateway remains edge ingress.
svc-gateway remains proxy-only for product routes.
svc-gateway does not store raw bytes.
svc-gateway does not hash uploaded bytes.
svc-gateway does not store manifests.
svc-gateway does not write index pointers.
svc-gateway does not call svc-index directly.
svc-gateway does not call svc-storage directly.
svc-gateway does not call svc-wallet.
svc-gateway does not mutate ron-ledger.
svc-gateway does not calculate ROC prices.
svc-gateway does not verify wallet receipts.
svc-gateway does not execute payouts.
```

The product chain now remains properly layered:

```text
svc-gateway
  public ingress/proxy

omnigate
  product BFF/hydration/composition

svc-index
  manifest pointer lookup

svc-storage
  raw bytes / object metadata

svc-wallet
  wallet mutation and receipts

ron-ledger
  durable economic truth
```

---

## 14. Architectural boundary notes

`svc-gateway` now owns:

```text
public route exposure
edge correlation/metrics layering
safe header forwarding
body/query proxying
transport error mapping
public HTTP path stability
```

`svc-gateway` still does **not** own:

```text
crab URL parsing
asset manifest DTO semantics
manifest pointer persistence
asset page hydration
object byte storage
wallet holds
wallet captures
wallet releases
ledger mutation
creator payout execution
browser extension UI
```

Those remain in:

```text
ron-naming     — crab URL grammar
ron-proto      — asset/manifest/page DTOs
svc-index      — manifest pointer persistence
omnigate       — asset page hydration and BFF composition
svc-storage    — content-addressed byte storage
svc-wallet     — wallet mutation and receipt API
ron-ledger     — append-only economic truth
browser extension — user-facing crab navigation UX later
```

---

## 15. Completion assessment for `svc-gateway`

For the WEB3_2 product-proof TODO, `svc-gateway` Batch 5 is:

```text
100% complete for this session’s scope
```

Current `svc-gateway` WEB3_2 readiness:

```text
GET /crab/resolve gateway route                 complete
GET /b3/<hash>.<kind> gateway route             complete
POST /paid/o/prepare route                      complete
POST /assets/image/prepare route                complete
POST /assets/image route                        complete
POST /sites/prepare route                       complete
POST /sites route                               complete
omnigate proxying                               complete
selected header forwarding                      complete
hop-by-hop header filtering                     complete
body/query preservation                         complete
upstream unavailable → 502 Problem              complete
targeted product proxy tests                    complete
strict clippy gate                              green
all-target tests                                green
```

---

## 16. Remaining future work for `svc-gateway`

No blocker remains from this session.

Future enhancements may include:

```text
Add route docs/OpenAPI for the new product paths.
Add dashboard-visible route metrics for crab/b3 requests.
Add stricter body caps per product route.
Add gateway-level capability prechecks if policy requires them.
Add browser-extension-local defaults once extension MVP starts.
Add product route smoke script that starts gateway + omnigate + index + storage.
Add public docs showing crab:// URL translation through gateway.
```

Potential future product routes:

```text
GET  /site/:name
GET  /crab/site/:name
POST /assets/:kind/prepare
POST /assets/:kind
POST /content/view/prepare
POST /content/view/capture
GET  /receipts/:tx_id
```

But gateway should continue to proxy these rather than implementing business behavior.

---

## 17. Session takeaway

`svc-gateway` now exposes the WEB3_2 product routes needed by clients and the future browser extension.

Before this batch, the system had:

```text
ron-naming   — crab grammar
ron-proto    — asset DTOs
svc-index    — manifest pointers
omnigate     — asset-page resolver
```

After this batch, the route chain is externally reachable through the gateway:

```text
client / browser extension
  → svc-gateway /crab/resolve or /b3/<hash>.<kind>
  → omnigate /v1/crab/resolve or /v1/b3/<hash>.<kind>
  → svc-index manifest pointer lookup
  → svc-storage availability check
  → typed asset-page response
```

This is a major product-proof milestone because it means the backend now has a clean public path for:

```text
crab://<64hex>.<asset_kind>
```

without violating crate boundaries.

`svc-gateway` Batch 5 is complete and green.


### END NOTE - APRIL 29 2026 - 13:00 CST


### BEGIN NOTE - APRIL 29 2026 - 17:50 CST
>THE FOLLOWING ARE DUAL NOTES FOR BOTH OMNIGATE AND SVC-GATEWAY:

Below are two **drop-in crate notes**: one for `omnigate`, one for `svc-gateway`. They capture what we accomplished, what was proven, architectural invariants, tests, smoke gates, known caveats, and remaining work. The final green gate passed end-to-end, including `omnigate` 5-test site launch coverage, `svc-gateway` 6-test product route proxy coverage, safe stack smoke, and mutation-enabled site create/resolve stack smoke. 

---

# Drop-in note for `crates/omnigate/NOTES.md`

````markdown
# omnigate WEB3_2 Product Layer Notes — Batch 8/9/10 Carryover

## Current status

`omnigate` is now one of the central WEB3_2 product-layer crates. In this session, it moved from having partial product proof routes to having a green-gated backend foundation for:

- typed `b3` / `crab://` asset-page resolution
- image asset prepare/upload coordination
- paid object prepare/proxy flows
- static site prepare
- static site manifest creation
- static site name pointer write
- static site resolver/hydrator
- live stack smoke through `svc-gateway → omnigate → svc-storage/svc-index`

The final green gate passed:

```text
cargo fmt
cargo clippy -p omnigate --all-targets --no-deps -- -D warnings
cargo test -p omnigate --test site_launch
cargo test -p omnigate --all-targets
scripts/web3_product_stack_smoke.sh
RON_RUN_SITE_CREATE=1 scripts/web3_product_stack_smoke.sh
scripts/web3_product_green_gate.sh
````

The final `site_launch.rs` test target now runs 5 tests and passes all of them, including the resolver/hydration proof:

```text
site_prepare_returns_wallet_hold_template
site_prepare_rejects_bad_site_name
site_prepare_maps_storage_estimate_errors
site_create_stores_manifest_and_index_pointer
site_resolve_hydrates_manifest_from_index_and_storage
```

The full `omnigate` all-targets suite also passed, including asset-page resolver tests, image asset prepare/upload tests, paid storage prepare/proxy tests, readiness tests, policy tests, middleware tests, metrics tests, and site launch tests.

## Session work summary

The main `omnigate` work centered on completing the static site product backend and then proving it through both cargo tests and live stack smoke.

### Site route surface now implemented/proven

The site route family is now:

```text
POST /v1/sites/prepare
POST /v1/sites
GET  /v1/sites/:name
```

These are mounted through the v1 router and are intended to sit behind public `svc-gateway` routes:

```text
POST /sites/prepare → omnigate POST /v1/sites/prepare
POST /sites         → omnigate POST /v1/sites
GET  /sites/:name   → omnigate GET  /v1/sites/:name
```

### `POST /v1/sites/prepare`

This route is a read-only/product-preflight route.

It validates:

* strict JSON body
* safe beta site name
* safe file paths
* nonzero total byte count or valid file byte sum

It then calls `svc-storage` paid estimate route:

```text
GET /paid/o/estimate?bytes=<total_bytes>
```

It returns a wallet-hold template and next-step hints, but does not call `svc-wallet`.

Important invariant:

```text
site_prepare does not mutate ledger, wallet, storage objects, or index pointers.
```

It returns schema:

```text
omnigate.site-prepare.v1
```

The response includes:

* normalized `site_name`
* action, asset, total bytes
* owner/passport/account hints
* paid storage estimate summary
* wallet hold template
* site manifest preview
* next routes for wallet hold, site submit, and later resolve

### `POST /v1/sites`

This route creates a static site manifest and writes the site name pointer.

It validates:

* strict JSON body
* safe beta site name
* canonical root document CID in `b3:<64 lowercase hex>` form
* canonical CIDs inside `asset_map`
* canonical CIDs inside `route_map`

It then builds a site manifest JSON object with:

* `version`
* `site_name`
* `root_document_cid`
* `asset_map`
* `route_map`
* `owner`
* `payout`
* `metadata`
* `provenance`
* `storage`
* `receipts`

It stores the generated manifest through `svc-storage`:

```text
POST /o
```

Then it writes the site pointer through `svc-index`:

```text
PUT /v1/index/sites/:site_name/manifest
```

It returns schema:

```text
omnigate.site-create.v1
```

A successful response must report:

```json
{
  "manifest": {
    "status": "stored",
    "manifest_cid": "b3:<64 lowercase hex>"
  },
  "index_pointer": {
    "status": "stored"
  }
}
```

Important invariant:

```text
site_create coordinates manifest object storage and pointer write, but it still does not call wallet or mutate ledger directly.
```

Wallet/ledger mutation remains outside this route. The route may include receipt refs and a provided hold txid in the manifest receipts, but it does not create/capture/release holds itself.

### `GET /v1/sites/:name`

This route was the key missing resolver/hydrator piece added in this session.

It performs a read-only chain:

```text
GET /v1/sites/:name
→ GET svc-index /v1/index/sites/:name/manifest
→ GET svc-storage /o/<manifest_cid>
→ parse site manifest JSON
→ return hydrated site page DTO
```

It returns schema:

```text
omnigate.site-page.v1
```

A successful hydrated response includes:

```json
{
  "manifest": {
    "status": "present",
    "hydration_status": "hydrated"
  }
}
```

It also returns:

* `site_name`
* `root_document_cid`
* owner summary
* payout summary
* metadata
* route map
* asset map
* receipts
* links, including `crab://<site_name>`, `/v1/sites/<site_name>`, and `/o/<manifest_cid>`

## Important bugs fixed during this session

### 1. Ownership/partial move in `site_create`

`site_create` initially moved fields from `request`, then tried to borrow the full request later for manifest construction.

Fix:

* clone `request.owner_passport_subject`
* clone `request.owner_wallet_account`

This preserved ownership and allowed `request` to be borrowed afterward.

### 2. Site-name error ordering

A test expected `../bad` to fail as `double_dot`, but the name validator originally returned `edge_dot`.

Fix:

* check `name.contains("..")` before edge-dot checks

This made traversal-like input fail with the stable reason:

```text
double_dot
```

### 3. `created_at_ms` overflowing literal in tests

Large timestamp literals in tests needed explicit `u64` suffixes to avoid `i32` overflow.

Fix:

```rust
1_776_000_000_000u64
```

### 4. Manifest payout lifetime issue

`SitePayoutSummary.default_action` was originally `&'static str`, but resolver hydration wanted to use a string borrowed from a hydrated manifest.

Fix:

```rust
default_action: String
```

The resolver now owns the default action string safely.

### 5. Live smoke `site_manifest_bad_json`

The mutation-enabled live site smoke initially failed with:

```json
{
  "code": "site_manifest_bad_json",
  "message": "site manifest object was not valid JSON",
  "retryable": true,
  "reason": "site_manifest_bad_json"
}
```

Root cause:

* The real generated manifest included `provenance` and `storage`.
* `SiteManifestDocument` used `#[serde(deny_unknown_fields)]`.
* The resolver DTO did not include `provenance` and `storage`, so strict parsing rejected a manifest generated by the same route family.

Fix:

* Add optional fields to the resolver DTO:

```rust
#[serde(default)]
provenance: Option<Value>,

#[serde(default)]
storage: Option<Value>,
```

This preserves strict DTO parsing while accepting fields the manifest generator intentionally emits.

### 6. Live smoke stale pointer issue mitigation

During live smoke debugging, the stack script was updated to use a unique per-run site name and an isolated per-run `svc-index` DB. This avoids resolving stale site-name pointers from earlier test runs.

Although this fix primarily lives in scripts, it matters for `omnigate` because it proved the resolver against fresh site pointer state.

## Current route and DTO invariants

### Content IDs

Internal content IDs remain canonical:

```text
b3:<64 lowercase hex>
```

The resolver rejects:

* missing `b3:` prefix
* uppercase hex
* wrong length
* non-hex characters

### Public crab asset URLs

The current public asset URL format is:

```text
crab://<64 lowercase hex>.<asset_kind>
```

The old `crab://b3/<hash>.image` path form is rejected for current/new product UX.

### Site names

Site names are human pointers, not canonical content IDs.

The current validator rejects:

* empty names
* control characters
* `.` and `..`
* double-dot traversal patterns
* leading or trailing dot
* slash or backslash
* `@`
* spaces
* unsupported characters
* names longer than 253 chars

Current allowed byte set:

```text
a-z
0-9
.
-
_
```

Names are normalized to lowercase.

### Storage ownership

`svc-storage` owns raw bytes by content ID only.

`omnigate` may store generated manifest bytes, but storage does not own:

* site routing
* name semantics
* owner lookup
* payout lookup
* manifest interpretation
* crab URL parsing

### Index ownership

`svc-index` owns mutable pointers such as:

```text
site name → site manifest CID
asset CID → asset manifest CID
```

`omnigate` writes/reads these pointers but does not own durable index storage itself.

### Wallet/ledger ownership

`omnigate` does not directly mutate wallet or ledger.

Allowed:

* return wallet hold templates
* include provided receipt refs / hold txid refs in generated manifests
* proxy or coordinate product DTOs

Not allowed:

* create wallet holds directly
* capture/release holds directly
* mutate `ron-ledger`
* bypass `svc-wallet`

This preserves the WEB3 invariant:

```text
svc-wallet is the normal mutation front-door.
ron-ledger is durable economic truth.
omnigate is product/BFF coordination and hydration only.
```

## Smoke and test coverage now passing

### Unit/integration tests

Important passing test groups:

```text
tests/site_launch.rs
tests/asset_page_resolver.rs
tests/image_asset_prepare.rs
tests/paid_storage_prepare.rs
tests/paid_storage_estimate_proxy.rs
tests/paid_storage_write_proxy.rs
tests/policy_gate.rs
tests/policy_metrics.rs
tests/readiness_error_rate.rs
tests/readiness_inflight.rs
tests/ready_truth.rs
tests/middleware_contract.rs
tests/metrics_contract.rs
tests/zk_receipts.rs
```

### `site_launch.rs`

Now proves:

```text
site prepare happy path
bad site name rejection
storage estimate error mapping
site manifest storage + pointer write
site resolver hydration from index + storage
```

### Live stack smoke

The live smoke proved:

```text
client/script
→ svc-gateway
→ omnigate
→ svc-storage
→ svc-index
→ back through gateway
```

Safe smoke passed:

```text
paid object prepare
image prepare
site prepare
b3 image asset page
crab:// image resolver
```

Mutation-enabled site smoke passed:

```text
/sites/prepare
/sites create
/sites/<unique-smoke-site>
```

The final green gate ended with:

```text
WEB3_2 product green gate passed
```

## Policy behavior

Production policy remains fail-closed.

Earlier live smoke hit:

```json
{
  "code": "POLICY_DENY",
  "message": "Access denied",
  "retryable": false,
  "reason": "default"
}
```

That was expected because product smoke was testing product route plumbing without a production policy bundle.

The stack smoke script now generates a local `omnigate-smoke.toml` that disables policy only for the local smoke run.

Important distinction:

```text
Production/default behavior: policy fails closed.
Local smoke behavior: policy disabled only in generated smoke config.
```

Do not weaken production policy to make tests pass.

## Files touched or relevant

Primary crate files:

```text
crates/omnigate/src/routes/v1/sites.rs
crates/omnigate/src/routes/v1/mod.rs
crates/omnigate/tests/site_launch.rs
```

Relevant scripts/docs added outside the crate:

```text
scripts/web3_product_stack_smoke.sh
scripts/web3_crab_image_smoke.sh
scripts/web3_crab_site_smoke.sh
scripts/web3_product_green_gate.sh
docs/WEB3_2_PRODUCT_ROUTE_CONTRACTS.md
docs/WEB3_2_PRODUCT_SMOKE_RUNBOOK.md
```

## Remaining work for omnigate

### 1. Paid image upload live smoke with real wallet hold

The image upload coordinator is tested and the prepare route is live-smoked, but the default live stack smoke intentionally skips paid upload:

```text
skip: paid upload; set RON_RUN_PAID_UPLOAD=1 with RON_IMAGE_FILE and RON_WALLET_HOLD_TXID to enable
```

Remaining:

* obtain a real hold txid from `svc-wallet`
* run `RON_RUN_PAID_UPLOAD=1`
* verify image bytes are stored
* verify image manifest is stored
* verify asset CID → manifest CID pointer is written
* verify `crab://<hash>.image` hydrates the uploaded image manifest

### 2. Product policy bundle

The local smoke disables policy. Production beta should have a policy bundle that allows intended product routes under correct capabilities and denies everything else.

Remaining:

* add explicit policy entries for product prepare/read routes
* add stricter policy entries for mutation routes
* test with policy enabled
* verify fail-closed defaults
* add route-specific policy metrics expectations

### 3. Site launch UX refinements

Current site create is a dev-mode manifest/pointer coordinator. It assumes static file CIDs already exist.

Remaining:

* decide whether omnigate should ever coordinate bundled site upload, or whether upload remains a higher-level client/gateway workflow
* support richer route maps
* support content-type metadata for site assets
* support app/site manifest conventions
* support owner/curator/provenance metadata extensions
* support paid site access/view route later

### 4. Shared DTO extraction

Some DTOs are local to `omnigate` route files. Once stable, consider moving reusable product DTOs into `ron-proto::web3` or `ron-proto::product`.

Candidate DTOs:

```text
site prepare request/response
site create request/response
site page response
image prepare response
asset page response
manifest pointer response
```

Do this only after route contracts settle.

### 5. Browser extension integration

`omnigate` is ready for the Chrome extension MVP.

The extension should call the public `svc-gateway` routes, not internal `omnigate` routes.

Important gateway-facing routes:

```text
GET  /crab/resolve?url=...
GET  /b3/:hash.kind
POST /paid/o/prepare
POST /assets/image/prepare
POST /assets/image
POST /sites/prepare
POST /sites
GET  /sites/:name
```

### 6. Observability details

Current route behavior inherits normal middleware/metrics coverage. Future work should add or verify product-specific counters for:

```text
omnigate_product_prepare_total
omnigate_asset_page_resolve_total
omnigate_site_create_total
omnigate_site_resolve_total
omnigate_manifest_hydration_failures_total
```

Only add new metrics if consistent with existing metrics style.

## Completion estimate for omnigate WEB3_2 product work

Current estimate:

```text
omnigate WEB3_2 product backend: ~94–96% complete
```

What is complete:

```text
asset page resolver
paid prepare route
image prepare route
image upload coordinator test path
site prepare route
site create route
site resolver/hydrator
gateway-facing live smoke compatibility
strict error taxonomy for major failure paths
```

What remains:

```text
paid image upload live smoke with real hold
production policy bundle for product routes
deeper DTO extraction/cleanup
browser extension integration feedback
optional richer static site/app support
```

````

---

# Drop-in note for `crates/svc-gateway/NOTES.md`

```markdown
# svc-gateway WEB3_2 Product Route Notes — Batch 8/9/10 Carryover

## Current status

`svc-gateway` now exposes the public WEB3_2 product route surface needed by the Chrome extension and client-facing beta flows.

The gateway is intentionally thin and proxy-only. Product semantics remain in `omnigate`.

Final green gate passed:

```text
cargo fmt
cargo clippy -p svc-gateway --all-targets --no-deps -- -D warnings
cargo test -p svc-gateway --test product_routes_proxy
cargo test -p svc-gateway --all-targets
scripts/web3_product_stack_smoke.sh
RON_RUN_SITE_CREATE=1 scripts/web3_product_stack_smoke.sh
scripts/web3_product_green_gate.sh
````

The dedicated product route proxy test now runs 6 tests and passes all 6.

The full live product stack smoke passed through:

```text
client/script
→ svc-gateway
→ omnigate
→ svc-storage
→ svc-index
→ back through gateway
```

The final green gate ended with:

```text
WEB3_2 product green gate passed
```

## Session work summary

This session completed and proved the public gateway exposure for the WEB3_2 backend product routes.

The major change was to ensure `svc-gateway` exposes and proxies the product routes that the Chrome extension will use.

## Public product route surface now proven

`svc-gateway` exposes:

```text
GET  /crab/resolve
GET  /b3/:asset
POST /paid/o/prepare
POST /assets/image/prepare
POST /assets/image
POST /sites/prepare
POST /sites
GET  /sites/:name
```

These proxy to `omnigate` internal routes:

```text
GET  /v1/crab/resolve
GET  /v1/b3/:asset
POST /v1/paid/o/prepare
POST /v1/assets/image/prepare
POST /v1/assets/image
POST /v1/sites/prepare
POST /v1/sites
GET  /v1/sites/:name
```

## Route-by-route notes

### GET `/crab/resolve?url=...`

Purpose:

```text
Resolve a crab:// URL through omnigate and return the corresponding product DTO.
```

Current proven use:

```text
crab://<64 lowercase hex>.image
```

The gateway preserves the query string and forwards to:

```text
/v1/crab/resolve?url=...
```

The product route proxy test proves query preservation and selected header forwarding.

### GET `/b3/:asset`

Purpose:

```text
Public typed b3 asset page route.
```

Expected path format:

```text
/b3/<64 lowercase hex>.<asset_kind>
```

Example:

```text
/b3/0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.image
```

Gateway does not validate product semantics. It proxies to `omnigate`, which owns asset parsing and error behavior.

### POST `/paid/o/prepare`

Purpose:

```text
Public paid object prepare route.
```

Gateway forwards to:

```text
/v1/paid/o/prepare
```

Gateway does not:

* price the object
* create a wallet hold
* mutate wallet
* mutate ledger
* store bytes

It only proxies the request and response.

### POST `/assets/image/prepare`

Purpose:

```text
Public image upload prepare route.
```

Gateway forwards to:

```text
/v1/assets/image/prepare
```

This is intended for the Chrome extension / creator UI to estimate cost and receive a wallet hold template.

### POST `/assets/image`

Purpose:

```text
Public paid image upload coordination route.
```

Gateway forwards to:

```text
/v1/assets/image
```

This route is not run by default in live smoke because it requires a real wallet hold txid and local image file.

Gateway remains proxy-only.

### POST `/sites/prepare`

Purpose:

```text
Public static site launch prepare route.
```

Gateway forwards to:

```text
/v1/sites/prepare
```

This route returns the wallet hold template and next-step hints.

### POST `/sites`

Purpose:

```text
Public static site create route.
```

Gateway forwards to:

```text
/v1/sites
```

The route creates/stores a site manifest and writes a site pointer through `omnigate`.

Gateway does not parse the manifest or pointer response.

### GET `/sites/:name`

Purpose:

```text
Public static site resolver/hydrator route.
```

Gateway forwards to:

```text
/v1/sites/:name
```

This was the important public route added/proven in this session.

It enables the browser/extension/public clients to resolve:

```text
crab://<site_name>
```

through an HTTP call to:

```text
GET /sites/:name
```

The gateway does not normalize, validate, or hydrate the site name. `omnigate` owns those semantics and fail-closed validation.

## Gateway invariants preserved

### Proxy-only behavior

The gateway remains a public edge proxy.

Allowed:

```text
- expose stable client/browser paths
- forward selected headers
- forward bodies
- preserve query strings
- return upstream response bodies/statuses
- convert connect/read failures into gateway Problem bodies
```

Not allowed:

```text
- no product DTO hydration
- no manifest parsing
- no pricing math
- no storage object ownership
- no site pointer ownership
- no wallet mutation
- no ledger mutation
- no app-specific business logic
```

This keeps the crate boundary clean:

```text
svc-gateway = public edge proxy
omnigate = product/BFF semantics
svc-storage = immutable raw bytes by content ID
svc-index = mutable product pointers
svc-wallet = economic mutation front-door
ron-ledger = durable economic truth
```

### Header forwarding

The product proxy forwards selected request headers:

```text
Authorization
Accept
Content-Type
Idempotency-Key
x-correlation-id
x-request-id
x-ron-*
```

It filters hop-by-hop and unsafe transport headers:

```text
Host
Connection
Proxy-Authorization
TE
Trailer
Transfer-Encoding
Upgrade
Content-Length
```

The product route tests specifically prove that `connection` is not forwarded and `x-ron-*` headers are preserved.

### Error behavior

When `omnigate` cannot be reached, gateway returns structured `upstream_unavailable` Problem responses with HTTP 502.

The product route proxy test includes:

```text
product_route_upstream_connect_failure_yields_problem_502
```

Gateway also passes upstream product errors through when appropriate.

## Tests added/updated/proven

Primary test file:

```text
crates/svc-gateway/tests/product_routes_proxy.rs
```

Now covers 6 product proxy tests:

```text
crab_resolve_proxy_preserves_query_and_headers
b3_asset_proxy_targets_omnigate_b3_route
paid_prepare_proxy_preserves_body_and_idempotency
image_and_site_product_routes_target_omnigate
site_resolve_proxy_targets_omnigate_site_hydrator
product_route_upstream_connect_failure_yields_problem_502
```

The final test output proved:

```text
running 6 tests
6 passed
```

Full gateway test suite also passed:

```text
app_proxy
paid_storage_estimate_proxy
paid_storage_write_proxy
product_routes_proxy
smoke
```

## Live stack smoke proof

The live stack smoke proved that public gateway routes work against actual running local services:

```text
svc-index
svc-storage
omnigate
svc-gateway
```

The safe smoke passed:

```text
/p paid object prepare through /paid/o/prepare
/assets/image/prepare
/sites/prepare
/b3/<hash>.image
/crab/resolve?url=crab://<hash>.image
```

The mutation-enabled site smoke passed:

```text
/sites/prepare
/sites create
/sites/<unique-smoke-site>
```

This proves gateway can sit in front of `omnigate` for the site create/resolve product loop.

Final mutation-enabled proof:

```text
ok: /sites/prepare
ok: /sites create
ok: /sites/<unique-smoke-site>
WEB3 crab site smoke passed
WEB3 product stack smoke passed
```

## Files touched or relevant

Primary crate files:

```text
crates/svc-gateway/src/routes/product.rs
crates/svc-gateway/tests/product_routes_proxy.rs
```

Relevant supporting files outside the crate:

```text
scripts/web3_product_stack_smoke.sh
scripts/web3_extension_backend_smoke.sh
scripts/web3_asset_page_smoke.sh
scripts/web3_crab_image_smoke.sh
scripts/web3_crab_site_smoke.sh
scripts/web3_product_green_gate.sh
docs/WEB3_2_PRODUCT_ROUTE_CONTRACTS.md
docs/WEB3_2_PRODUCT_SMOKE_RUNBOOK.md
```

## Current Chrome extension handoff contract

The Chrome extension should treat `svc-gateway` as its backend API.

Important routes for the extension MVP:

```text
GET  /crab/resolve?url=...
GET  /b3/:hash.kind
POST /paid/o/prepare
POST /assets/image/prepare
POST /assets/image
POST /sites/prepare
POST /sites
GET  /sites/:name
```

The extension should attach identity/account headers:

```text
x-ron-passport
x-ron-wallet-account
x-ron-permission
x-ron-spend-limit
x-correlation-id
```

Where appropriate, it should also attach:

```text
Authorization
Idempotency-Key
Content-Type
```

The gateway already has tests proving selected header forwarding.

## Known caveats

### Paid image upload live path still skipped

The default smoke intentionally skips:

```text
RON_RUN_PAID_UPLOAD=1
```

Reason:

```text
Paid upload requires a real image file and a real wallet hold txid.
```

This is not a gateway failure. The gateway route exists and is tested as a proxy. The full paid upload live flow should wait until we have a wallet hold fixture or a real `svc-wallet`-created hold txid.

### Local product smoke disables omnigate policy

The product stack smoke script starts `omnigate` with a generated local smoke config that disables policy. This is only for route/product proof.

Production policy must remain fail-closed.

Do not change gateway to bypass or weaken production policy.

### Gateway does not validate site names

This is intentional. `omnigate` owns site-name validation.

Gateway simply proxies:

```text
GET /sites/:name → GET /v1/sites/:name
```

## Remaining work for svc-gateway

### 1. Chrome extension integration testing

Once the extension scaffold exists, test it against:

```text
SVC_GATEWAY_BIND_ADDR=127.0.0.1:8090
```

Expected extension calls:

```text
GET /crab/resolve?url=crab://...
GET /b3/:hash.kind
GET /sites/:name
POST /sites/prepare
POST /assets/image/prepare
```

### 2. Production policy-aligned route tests

Current proxy tests use dummy `omnigate`. Product stack smoke disables `omnigate` policy to prove route plumbing.

Remaining:

* run product route stack smoke with a real product policy bundle
* prove allowed capability paths
* prove denied paths fail closed
* verify useful `POLICY_DENY` body reaches the client

### 3. Paid image upload live edge smoke

Remaining once wallet hold is available:

```text
RON_RUN_PAID_UPLOAD=1
RON_IMAGE_FILE=/absolute/path/to/test.png
RON_WALLET_HOLD_TXID=<real_hold_txid>
scripts/web3_product_stack_smoke.sh
```

This should prove:

```text
client → gateway /assets/image
gateway → omnigate /v1/assets/image
omnigate → storage object put
omnigate → manifest put
omnigate → index pointer put
response returns crab://<hash>.image
```

### 4. Route contract stability

Before beta, preserve these public paths unless explicitly versioning them:

```text
/crab/resolve
/b3/:asset
/paid/o/prepare
/assets/image/prepare
/assets/image
/sites/prepare
/sites
/sites/:name
```

Avoid breaking the extension once it depends on them.

### 5. Observability follow-up

Gateway already inherits route metrics/correlation layers, but product-specific dashboards may need route labels or counters for:

```text
gateway_product_proxy_total
gateway_product_upstream_errors_total
gateway_product_latency_seconds
```

Add only if consistent with existing metrics style.

## Completion estimate for svc-gateway WEB3_2 product exposure

Current estimate:

```text
svc-gateway WEB3_2 product exposure: ~95–97% complete
```

Complete:

```text
public product routes
crab resolver proxy
b3 asset proxy
paid prepare proxy
image prepare/upload proxy
site prepare/create/resolve proxy
header forwarding tests
query/body preservation tests
upstream failure tests
full live stack smoke through gateway
```

Remaining:

```text
paid image upload live test with real hold
policy-enabled product route stack test
Chrome extension integration feedback
optional product-specific metrics refinement
```

```
```



### END NOTE - APRIL 29 2026 - 17:50 CST




### BEGIN NOTE - APRIL 30 2026 - 00:30 CST

Below is a **single dual note** you can paste into both:

```text
crates/svc-gateway/NOTES.MD
crates/omnigate/NOTES.MD
```

---

# BEGIN NOTE — APRIL 30 2026 — CrabLink Identity/Wallet Gateway + Omnigate Façade

## Scope of this note

This note covers the paired work completed across:

```text
svc-gateway
omnigate
```

These two crates were updated together to support the first live CrabLink browser-extension identity flow:

```text
CrabLink Extension
→ svc-gateway public browser routes
→ omnigate WEB3_2 identity/wallet façade
→ future svc-passport / svc-wallet / ron-ledger backing
```

This work intentionally **did not** modify:

```text
svc-passport
svc-wallet
ron-ledger
ron-proto
ron-naming
svc-storage
svc-index
ron-accounting
svc-rewarder
```

Those are future integration targets.

---

## Executive summary

We successfully added the first extension-facing identity and wallet display route surface for CrabLink.

The browser extension can now:

```text
- detect that the RustyOnions gateway is online
- check readiness
- ask whether a passport is present
- bootstrap a dev-safe passport label
- load a linked wallet account label
- fetch a wallet balance display DTO
- run route diagnostics against the live local stack
```

The working live path is now:

```text
CrabLink Extension
→ http://127.0.0.1:8090
→ svc-gateway
→ omnigate
→ identity/wallet façade
```

This was proven live by the persistent CrabLink dev stack:

```text
svc-index    → healthy
svc-storage  → healthy
omnigate     → healthy
svc-gateway  → healthy and ready
identity smoke → passed
CrabLink popup → ONLINE
```

The extension now displays:

```text
Gateway:  http://127.0.0.1:8090
Passport: passport:main:dev
Wallet:   acct_dev
ROC:      0 ROC
Status:   ONLINE
```

This is a real end-to-end browser-to-backend milestone.

---

## Files changed in `svc-gateway`

### Updated

```text
crates/svc-gateway/src/routes/product.rs
```

### Added

```text
crates/svc-gateway/tests/identity_routes_proxy.rs
```

### New public gateway routes

`svc-gateway` now exposes these CrabLink/browser-facing routes:

```text
GET  /identity/me
POST /identity/passport/bootstrap
GET  /wallet/:account/balance
```

These routes proxy to omnigate:

```text
GET  /v1/identity/me
POST /v1/identity/passport/bootstrap
GET  /v1/wallet/:account/balance
```

### Existing WEB3_2 product routes preserved

We also confirmed the existing product routes still work:

```text
GET  /crab/resolve?url=<crab_url>
GET  /b3/:asset
POST /paid/o/prepare
POST /assets/image/prepare
POST /assets/image
POST /sites/prepare
POST /sites
GET  /sites/:name
```

---

## `svc-gateway` responsibilities after this work

`svc-gateway` remains a **thin public edge/proxy**.

It owns:

```text
- stable public browser/client route paths
- HTTP proxying to omnigate
- preserving query strings
- forwarding selected request headers
- preserving POST request bodies
- returning upstream response bodies
- mapping omnigate connection/read failures to structured 502 problems
```

It does **not** own:

```text
- passport key creation
- passport persistence
- wallet creation
- wallet balance truth
- ROC issuance
- ledger mutation
- manifest hydration
- asset ownership
- storage writes
- pricing logic
```

Gateway must remain proxy-only for these product routes.

---

## Gateway header forwarding behavior

For the CrabLink identity and wallet routes, `svc-gateway` forwards selected headers including:

```text
Authorization
Accept
Content-Type
Idempotency-Key
x-correlation-id
x-request-id
x-ron-*
```

It filters hop-by-hop / transport headers such as:

```text
Host
Connection
Proxy-Authorization
TE
Trailer
Transfer-Encoding
Upgrade
Content-Length
```

This preserves the clean edge boundary: browser state and request context can reach omnigate, but gateway does not interpret private identity or wallet semantics.

---

## Gateway tests added

The new test file:

```text
crates/svc-gateway/tests/identity_routes_proxy.rs
```

proves:

```text
- GET /identity/me proxies to /v1/identity/me
- POST /identity/passport/bootstrap proxies to /v1/identity/passport/bootstrap
- GET /wallet/:account/balance proxies to /v1/wallet/:account/balance
- Authorization header is preserved
- x-ron-passport is preserved
- x-ron-wallet-account is preserved
- x-correlation-id is preserved
- x-request-id is generated/preserved as expected
- Idempotency-Key is preserved for bootstrap
- hop-by-hop headers are not forwarded
- upstream connection failure returns a structured 502 problem
```

The test result was green:

```text
running 4 tests
test identity_route_upstream_connect_failure_yields_problem_502 ... ok
test passport_bootstrap_proxy_preserves_body_and_idempotency ... ok
test wallet_balance_proxy_targets_omnigate_wallet_balance ... ok
test identity_me_proxy_preserves_passport_headers ... ok

test result: ok. 4 passed
```

---

## Files changed in `omnigate`

### Updated

```text
crates/omnigate/src/routes/v1/mod.rs
```

### Added

```text
crates/omnigate/src/routes/v1/identity.rs
crates/omnigate/src/routes/v1/wallet.rs
```

### New omnigate v1 routes

`omnigate` now exposes:

```text
GET  /v1/identity/me
POST /v1/identity/passport/bootstrap
GET  /v1/wallet/:account/balance
```

These are currently **dev-safe façade routes**, not final `svc-passport` / `svc-wallet` backed routes.

---

## `omnigate` responsibilities after this work

`omnigate` currently owns the temporary WEB3_2 identity/wallet browser façade.

It owns:

```text
- returning safe passport display labels
- returning safe wallet account display labels
- returning a wallet display DTO
- returning capability hints for the CrabLink UI
- making clear when a response is not ledger-backed
- preventing fake positive ROC balances
- preventing uncaveated spend authority
```

It does **not** own:

```text
- private key custody
- local browser key material
- real passport creation/persistence yet
- real wallet mutation yet
- ROC issuance yet
- ledger mutation
- durable economic truth
```

This keeps `omnigate` aligned with its BFF/view-hydrator role.

---

## Current `GET /v1/identity/me` behavior

The route returns identity context from request headers.

Expected request context:

```text
x-ron-passport: passport:main:dev
x-ron-wallet-account: acct_dev
```

Current response shape:

```json
{
  "schema": "crablink.identity.me.v1",
  "passport": {
    "subject": "passport:main:dev",
    "kind": "main",
    "display_name": "Local Dev Passport",
    "created_at": null,
    "source": "request_header"
  },
  "wallet": {
    "account": "acct_dev",
    "linked": true,
    "source": "request_header"
  },
  "capabilities": {
    "can_view_balance": true,
    "can_prepare_paid_actions": true,
    "can_spend": false
  },
  "warnings": [
    "identity view is header-derived until svc-passport is wired"
  ]
}
```

Important invariant:

```text
can_spend=false
```

The browser is not granted wallet spend authority by this route.

---

## Current `POST /v1/identity/passport/bootstrap` behavior

The route currently returns dev-safe labels only.

It accepts a request like:

```json
{
  "kind": "main",
  "label": "CrabLink main passport",
  "client": "crablink-chrome",
  "request_starter_grant": true,
  "create_wallet": true
}
```

Current response shape:

```json
{
  "schema": "crablink.identity.bootstrap.v1",
  "passport": {
    "subject": "passport:main:dev",
    "kind": "main",
    "display_name": "CrabLink main passport",
    "created_at": null,
    "source": "omnigate_dev_bootstrap"
  },
  "wallet": {
    "account": "acct_dev",
    "linked": true,
    "source": "omnigate_dev_bootstrap"
  },
  "starter_grant": {
    "issued": false,
    "amount_minor_units": "0",
    "receipt_id": null,
    "reason": "svc_wallet_integration_pending"
  },
  "capabilities": {
    "can_view_balance": true,
    "can_prepare_paid_actions": true,
    "can_spend": false
  },
  "warnings": [
    "dev bootstrap returns labels only",
    "starter ROC is not issued until svc-wallet integration is wired",
    "wallet spend authority is not granted by this response"
  ]
}
```

Important invariant:

```text
starter_grant.issued=false
amount_minor_units="0"
can_spend=false
```

This avoids fake ROC issuance.

---

## Current `GET /v1/wallet/:account/balance` behavior

The route currently returns a truthful placeholder balance:

```json
{
  "schema": "crablink.wallet.balance.v1",
  "account": "acct_dev",
  "unit": "ROC",
  "available_minor_units": "0",
  "held_minor_units": "0",
  "display": "0 ROC",
  "as_of": null,
  "ledger_backed": false,
  "source": "omnigate_dev_wallet_view.v1",
  "reason": "svc_wallet_integration_pending",
  "warnings": [
    "display-only dev placeholder",
    "real balance must come from svc-wallet and ron-ledger",
    "do not treat this route as spend authority"
  ]
}
```

Important invariant:

```text
ledger_backed=false
available_minor_units="0"
```

A non-ledger-backed response must not fake a positive balance.

---

## Live scripts added outside the crates

These scripts were added or heavily updated in the RustyOnions repo and are relevant to both crates:

```text
scripts/web3_identity_stack_smoke.sh
scripts/web3_crablink_dev_stack.sh
```

### `scripts/web3_identity_stack_smoke.sh`

This script proves the extension-facing identity/wallet routes live through `svc-gateway`.

It validates:

```text
GET  /healthz
GET  /readyz
GET  /identity/me
POST /identity/passport/bootstrap
GET  /wallet/acct_dev/balance
```

It also validates:

```text
- identity DTO has expected passport subject
- identity DTO has expected wallet account
- bootstrap DTO has expected passport and wallet labels
- bootstrap does not grant spend authority
- bootstrap does not fake starter ROC
- wallet balance is integer minor units
- non-ledger-backed balance is not fake-positive
```

The final successful output was:

```text
WEB3 identity stack smoke passed
```

### `scripts/web3_crablink_dev_stack.sh`

This script starts a persistent local WEB3_2 stack for browser testing:

```text
svc-index
svc-storage
omnigate
svc-gateway
```

It keeps the stack online at:

```text
http://127.0.0.1:8090
```

It also runs the identity smoke automatically.

The successful output was:

```text
CrabLink dev stack is online
gateway: http://127.0.0.1:8090
```

This allowed the Chrome extension to show:

```text
ONLINE
passport:main:dev
acct_dev
0 ROC
```

---

## Documentation added outside the crates

The following route-contract doc was added:

```text
docs/WEB3_2_IDENTITY_ROUTE_CONTRACTS.md
```

It defines:

```text
- GET /identity/me contract
- POST /identity/passport/bootstrap contract
- GET /wallet/:account/balance contract
- starter ROC grant rule
- gateway header forwarding contract
- no fake ROC rule
- no browser private key rule
- no uncaveated spend authority rule
```

This doc should remain the contract until we replace the dev façade with real `svc-passport` / `svc-wallet` backing.

---

## Tests and gates passed

The following commands passed successfully:

```text
cargo fmt -p omnigate -p svc-gateway
cargo test -p svc-gateway --test identity_routes_proxy
cargo test -p svc-gateway --test product_routes_proxy
cargo clippy -p svc-gateway --all-targets --no-deps -- -D warnings
cargo clippy -p omnigate --no-deps -- -D warnings
```

Additional product gate passed:

```text
scripts/web3_product_green_gate.sh
```

This proved:

```text
- omnigate clippy passed
- omnigate site_launch tests passed
- omnigate all-target tests passed
- svc-gateway clippy passed
- svc-gateway product route proxy tests passed
- svc-gateway all-target tests passed
- safe WEB3 product stack smoke passed
- mutation-enabled site create/resolve stack smoke passed
```

Successful product smoke included:

```text
WEB3 product stack smoke passed
WEB3_2 product green gate passed
```

Successful identity smoke included:

```text
GET /healthz -> HTTP 200
GET /readyz -> HTTP 200
GET /identity/me -> HTTP 200
POST /identity/passport/bootstrap -> HTTP 200
GET /wallet/acct_dev/balance -> HTTP 200
WEB3 identity stack smoke passed
```

---

## CrabLink integration proven

The separate CrabLink repo now successfully talks to this RustyOnions route surface.

CrabLink proved:

```text
- extension structure checks pass
- JavaScript syntax checks pass
- local gateway smoke passes
- /healthz passes
- /readyz passes
- /identity/me passes
- /wallet/acct_dev/balance passes
- /b3/<sample>.image passes
- /crab/resolve passes
- bootstrap mutation route is skipped by default in read-only diagnostics
- Chrome package zip builds
```

This matters for `svc-gateway` and `omnigate` because it proves their new extension-facing routes are not theoretical. The browser client is already consuming them.

---

## Architectural invariants preserved

This work preserved the WEB3 / WEB3_2 invariants:

```text
- ROC remains internal only
- no ROX/Solana/external settlement
- no floating-point money math
- no fake positive ROC balance
- no direct ledger mutation from gateway
- no direct ledger mutation from omnigate
- svc-wallet remains the future normal mutation front-door
- ron-ledger remains durable economic truth
- gateway remains proxy-only
- omnigate remains a BFF/façade/hydrator, not durable truth
- passport labels are not wallet authority
- browser stores no private keys
- browser stores no seed phrases
- browser performs no silent spending
- capability/policy checks must fail closed in production
```

---

## Current limitations

The current implementation is intentionally not final.

Known limitations:

```text
- passport bootstrap is dev-label-only
- no real svc-passport persistence yet
- no real local encrypted passport vault yet
- no password/PIN unlock flow yet
- no alt anonymous passport management yet
- no real svc-wallet balance call yet
- no 1776 ROC starter grant yet
- no ledger-backed receipt for passport starter grants yet
- wallet balance is currently ledger_backed=false
- starter_grant currently reports issued=false
```

The current façade is safe because it is explicit about those limitations.

---

## Next major backend step

The next requested feature is:

```text
Every newly created passport should automatically start with 1776 ROC.
```

This should **not** be implemented in CrabLink as a fake local balance.

The correct future flow is:

```text
CrabLink
→ svc-gateway POST /identity/passport/bootstrap
→ omnigate /v1/identity/passport/bootstrap
→ svc-passport create/load local passport identity
→ svc-wallet issue 1776 ROC
→ ron-ledger commit issue event
→ svc-wallet returns receipt
→ omnigate returns starter_grant.issued=true + receipt id
→ CrabLink refreshes /wallet/:account/balance
→ balance displays 1776 ROC
```

The starter grant must be backend truth, not extension display state.

---

## Next files likely to change

Future work will probably touch:

```text
crates/omnigate/src/config/*
crates/omnigate/src/downstream/*
crates/omnigate/src/routes/v1/identity.rs
crates/omnigate/src/routes/v1/wallet.rs
scripts/web3_crablink_dev_stack.sh
scripts/web3_identity_stack_smoke.sh
```

And likely bring in these services:

```text
svc-passport
svc-wallet
ron-ledger
```

`svc-gateway` may not need much more for the 1776 ROC step if it continues proxying the same public routes.

---

## Next implementation plan: 1776 ROC starter grant

Recommended Batch 8:

```text
1. Add svc-wallet to scripts/web3_crablink_dev_stack.sh.
2. Ensure svc-wallet has a configured ledger path for the local dev stack.
3. Add omnigate wallet downstream config:
   - OMNIGATE_WALLET_BASE_URL
   - config field for wallet base URL
   - downstream wallet client helper
4. Update omnigate /v1/wallet/:account/balance:
   - call svc-wallet balance endpoint
   - return ledger_backed=true when successful
   - preserve placeholder fallback only for explicitly configured dev mode, if desired
5. Update omnigate /v1/identity/passport/bootstrap:
   - create/load passport label for now
   - derive/link wallet account
   - call svc-wallet issue endpoint for 1776 ROC
   - use idempotency key so repeated bootstrap does not double-issue
   - return starter_grant.issued=true
   - return receipt_id from svc-wallet
6. Update identity smoke:
   - bootstrap should return starter_grant.issued=true
   - amount_minor_units should be "1776"
   - receipt_id must be present
   - balance route should report 1776 after bootstrap
7. Keep the no-fake-money invariant:
   - if wallet issue fails, bootstrap should fail or report issued=false
   - extension must not invent the 1776 balance
```

Important decision still needed:

```text
Should 1776 mean:
A) 1776 minor units, displayed as 1776 ROC in the current dev model
or
B) 1776 full ROC with a future decimal scale
```

Current safest choice:

```text
Use integer amount_minor_units="1776" and display "1776 ROC" for dev bootstrap.
```

Do not introduce floating-point math.

---

## Risks to watch next

### Double issuance risk

The biggest risk in the 1776 ROC batch is issuing 1776 ROC multiple times if the user clicks Create repeatedly.

Mitigation:

```text
- use Idempotency-Key
- use deterministic account bootstrap key
- store/check bootstrap receipt by passport subject or account
- repeated bootstrap should return the same receipt or no-op status
```

### Boundary drift risk

Do not let gateway or omnigate mutate `ron-ledger` directly.

Correct mutation path:

```text
omnigate
→ svc-wallet
→ ron-ledger
```

Incorrect path:

```text
omnigate
→ ron-ledger
```

### Browser trust risk

Do not treat the browser as trusted.

The browser can request bootstrap, but backend policy must decide:

```text
- whether bootstrap is allowed
- whether starter grant is allowed
- how many times it is allowed
- which wallet account receives it
```

### Dev façade lingering risk

The dev façade is safe now because it is explicit. Once real backing exists, old placeholder behavior should either be removed or guarded behind obvious dev-only config.

---

## Current completion estimate for this paired slice

For the `svc-gateway + omnigate` CrabLink identity surface:

```text
Public route exposure:             95% complete
Gateway proxy behavior:            95% complete
Omnigate dev façade:               90% complete
Live dev stack support:            90% complete
CrabLink browser integration:      85% complete
Real svc-passport backing:          0% complete
Real svc-wallet balance backing:    0% complete
1776 ROC starter grant backing:     0% complete
```

Overall for this paired identity/wallet route slice:

```text
~65–70% complete
```

Reason: the browser/gateway/omnigate route surface is working and proven, but the final backend truth path still needs `svc-passport`, `svc-wallet`, and `ron-ledger`.

---

## TL;DR carryover

We added a real extension-facing identity/wallet route layer:

```text
CrabLink
→ svc-gateway
→ omnigate
```

Routes now working:

```text
GET  /identity/me
POST /identity/passport/bootstrap
GET  /wallet/:account/balance
```

Current implementation is a safe dev façade:

```text
passport:main:dev
acct_dev
0 ROC
ledger_backed=false
can_spend=false
starter_grant.issued=false
```

Everything compiled, tested, linted, smoked, and worked in the Chrome extension.

Next goal:

```text
Replace dev façade with real backend truth:
svc-passport-backed passport bootstrap
svc-wallet-backed balance
1776 ROC starter grant through svc-wallet → ron-ledger receipt
```

Do not fake 1776 ROC in the extension. The extension should only display backend truth.

# END NOTE — APRIL 30 2026 — CrabLink Identity/Wallet Gateway + Omnigate Façade


### END NOTE - APRIL 30 2026 - 00:30 CST




### BEGIN NOTES - MAY 2 2026 - 13:55 CST

Below is a **dual drop-in note set** for `omnigate` and `svc-gateway`. It covers what changed, what was proven, the live proof artifacts, and what remains. The green test run showed both `omnigate` and `svc-gateway` clippy/tests passing, and the paid image proof showed `omnigate.image-asset-upload.v1` plus hydrated `omnigate.asset-page.v1` resolving correctly.  

````markdown
### BEGIN NOTE - MAY 02 2026 - CRABLINK WALLET HOLD + PAID IMAGE PRODUCT PROOF

# CARRY-OVER NOTES — omnigate + svc-gateway WEB3_2 CrabLink Wallet Hold / Paid Image Proof

**Date:** 2026-05-02  
**Scope:** `omnigate`, `svc-gateway`, with live proof through `svc-wallet`, `svc-storage`, and `svc-index`  
**Status:** Green compile/test for touched RustyOnions crates; CrabLink can now reach live ROC wallet state and a real paid image asset page  
**Verdict:** Major product-proof milestone achieved. The browser-facing WEB3_2 stack has crossed from “prepare/preflight only” into “explicit wallet hold + paid content upload + crab:// asset resolution.”

---

## 0. TL;DR

We worked on two RustyOnions crates in this session:

```text
crates/omnigate
crates/svc-gateway
````

We also worked heavily in the separate CrabLink Chrome extension repo, but these notes are for the RustyOnions crate notes.

The key backend achievement:

```text
CrabLink
→ svc-gateway /wallet/hold
→ omnigate /v1/wallet/hold
→ svc-wallet /v1/hold
→ real ROC wallet hold receipt
```

Then we manually proved the next paid product path:

```text
ROC wallet hold receipt
→ svc-gateway /assets/image
→ omnigate image asset coordinator
→ svc-storage paid object write
→ manifest stored
→ svc-index asset pointer stored
→ crab://<hash>.image resolves
→ hydrated omnigate.asset-page.v1 rendered in CrabLink
```

This is a major WEB3_2 product checkpoint.

---

# PART A — omnigate notes

## 1. omnigate — what changed

### 1.1 Added/updated wallet façade route

We added/updated the `omnigate` wallet route surface so browser clients do not need direct access to `svc-wallet`.

Public flow:

```text
CrabLink
→ svc-gateway POST /wallet/hold
→ omnigate POST /v1/wallet/hold
→ svc-wallet POST /v1/hold
```

The route belongs in:

```text
crates/omnigate/src/routes/v1/wallet.rs
```

and is mounted from:

```text
crates/omnigate/src/routes/v1/mod.rs
```

The important endpoint now exposed by `omnigate`:

```text
POST /v1/wallet/hold
```

Existing wallet display route also remains:

```text
GET /v1/wallet/:account/balance
```

### 1.2 Preserved service boundaries

`omnigate` does **not** mutate the ledger directly.

The invariant remains:

```text
svc-wallet remains the economic mutation front-door.
ron-ledger remains durable append-only truth.
omnigate is a façade/coordinator, not the wallet implementation.
```

This is important. We did not cheat the architecture to make the demo work.

### 1.3 Wallet hold request shape

The wallet hold request accepted by `omnigate` mirrors the `svc-wallet` transfer/hold request shape:

```json
{
  "from": "acct_dev",
  "to": "escrow_paid_write",
  "asset": "roc",
  "amount_minor": "1024",
  "nonce": 1,
  "memo": "CrabLink site hold for crab://site",
  "idempotency_key": "crablink-preview:site:awebsitename"
}
```

Rules enforced in `omnigate` before forwarding:

```text
from must be non-empty
to must be non-empty
asset must normalize to roc
amount_minor must be a positive integer string
nonce is forwarded to svc-wallet
memo is trimmed/optional
idempotency_key is trimmed/optional
x-ron-wallet-account header, when present, must match request.from
```

### 1.4 Wallet hold response shape observed

The live hold response from `svc-wallet` through gateway/omnigate looked like:

```json
{
  "txid": "tx_14b0998645b92302c222931ed90c567e",
  "op": "hold",
  "from": "acct_dev",
  "to": "escrow_paid_write",
  "asset": "roc",
  "amount_minor": "1024",
  "nonce": 1,
  "idem": "crablink-preview:site:awebsitename",
  "ts": 1777704690070,
  "ledger_seq_start": 2,
  "ledger_seq_end": 3,
  "ledger_root": "ab2551be5dc15a435ff9d97ed62b643c0304ffb4921f4f3150305a800feccded",
  "receipt_hash": "b3:86c620beca061320570baf5cbc8ba7ee2f5d51bd072523195252e10e1ff8d34e"
}
```

This receipt was successfully replayed idempotently without deducting again.

### 1.5 Environment knobs

The wallet façade uses:

```text
OMNIGATE_WALLET_BASE_URL=http://127.0.0.1:8088
OMNIGATE_WALLET_BEARER=dev
```

The correct live startup pattern is:

```bash
cd /Users/mymac/Desktop/RustyOnions
RUST_LOG=info SVC_WALLET_ADDR=127.0.0.1:8088 cargo run -p svc-wallet
```

Then in another terminal:

```bash
cd /Users/mymac/Desktop/RustyOnions
OMNIGATE_WALLET_BASE_URL=http://127.0.0.1:8088 \
OMNIGATE_WALLET_BEARER=dev \
scripts/web3_crablink_dev_stack.sh
```

### 1.6 Balance façade behavior

The wallet balance route returns a CrabLink-facing DTO:

```json
{
  "schema": "crablink.wallet.balance.v1",
  "account": "acct_dev",
  "unit": "ROC",
  "available_minor_units": "1776",
  "held_minor_units": "0",
  "display": "1776 ROC",
  "as_of": null,
  "ledger_backed": true,
  "source": "svc_wallet.v1",
  "reason": null,
  "warnings": []
}
```

Observed later balances:

```text
after 1024 site hold: 752 ROC
after 68 image hold: 684 ROC
```

Important note:

```text
held_minor_units remains "0" in the current display DTO even though holds reduce available balance.
```

This means the current wallet route presents holds as deducted/unavailable but does not yet expose reserved/held amounts separately. This is acceptable for the current milestone but should be clarified later.

---

## 2. omnigate — paid image upload proof

### 2.1 Successful paid image upload route

The successful upload went through:

```text
POST /assets/image
```

Gateway forwards this to `omnigate`, and `omnigate` coordinates:

```text
paid proof verification
→ paid storage write
→ manifest creation/storage
→ asset pointer indexing
→ response with crab:// URL
```

Successful response schema:

```text
omnigate.image-asset-upload.v1
```

### 2.2 Successful paid proof headers learned

The live backend required these paid proof headers:

```text
x-ron-paid-op: hold
x-ron-paid-asset: roc
x-ron-paid-estimate-minor: <amount>
x-ron-wallet-txid: <txid>
x-ron-wallet-receipt-hash: <receipt_hash>
x-ron-wallet-from: <payer>
x-ron-wallet-to: <escrow>
```

Important correction discovered live:

```text
Wrong header:
x-ron-wallet-hold-txid

Correct header:
x-ron-wallet-txid
```

The backend also required:

```text
x-ron-paid-op
```

Once the full header contract was supplied, the upload succeeded.

### 2.3 Successful image upload artifact

Tiny PNG proof:

```text
path: artifacts/crablink-manual/tiny-crablink.png
size: 68 bytes
```

Hold receipt used for image upload:

```json
{
  "txid": "tx_f227fae07360b37dfefe31cafeec3fdb",
  "op": "hold",
  "from": "acct_dev",
  "to": "escrow_paid_write",
  "asset": "roc",
  "amount_minor": "68",
  "nonce": 2,
  "idem": "crablink-preview:image:tiny-crablink.png",
  "ledger_seq_start": 4,
  "ledger_seq_end": 5,
  "ledger_root": "bdf0cb74e42952166eae66ed6f16b1813de99ff8480e5cbce3da91bbf22364af",
  "receipt_hash": "b3:8c05f9c22010c9118d21e38d2cda75cb9c7a0d4d85c5c5b86a938f991ed73e9e"
}
```

Upload result:

```text
schema: omnigate.image-asset-upload.v1
asset_kind: image
asset_cid: b3:984128e643b594b1ff15ed2c40cf1d589616b9ddb7b212d00e91670997c1b8e4
crab_url: crab://984128e643b594b1ff15ed2c40cf1d589616b9ddb7b212d00e91670997c1b8e4.image
manifest_cid: b3:e81539db791855a113757e2d84bfd53d40721f232208ff79879ab0eb77e1f6f5
owner: passport:main:dev / acct_dev
payout recipient: acct_dev
wallet_txid: tx_f227fae07360b37dfefe31cafeec3fdb
wallet_receipt_hash: b3:8c05f9c22010c9118d21e38d2cda75cb9c7a0d4d85c5c5b86a938f991ed73e9e
```

### 2.4 Resolved asset page proof

The returned URL:

```text
crab://984128e643b594b1ff15ed2c40cf1d589616b9ddb7b212d00e91670997c1b8e4.image
```

resolved to:

```text
schema: omnigate.asset-page.v1
asset_kind: image
manifest.status: present
manifest.hydration_status: hydrated
storage.available: true
storage.size_bytes: 68
owner.passport_subject: passport:main:dev
owner.wallet_account: acct_dev
payout.recipient_account: acct_dev
metadata.title: Tiny CrabLink Test Image
metadata.description: First terminal-proven CrabLink paid image upload.
metadata.tags: crablink, tiny, proof
receipts[0].tx_id: tx_f227fae07360b37dfefe31cafeec3fdb
receipts[0].receipt_kind: paid_storage
receipts[0].amount_minor_units: 68
```

This asset page loaded successfully inside the CrabLink browser extension.

---

## 3. omnigate — quality gates run

The user ran:

```bash
cargo fmt
cargo clippy -p omnigate --all-targets --no-deps -- -D warnings
cargo test -p omnigate --all-targets
```

Final status:

```text
omnigate clippy: green
omnigate tests: green
```

Important passing test areas:

```text
routes::v1::identity unit tests
asset_page_resolver tests
builtin_page_resolver tests
image_asset_prepare tests
paid_storage_prepare tests
paid_storage_write_proxy tests
site_launch tests
readiness tests
policy tests
middleware tests
metrics contract tests
```

### 3.1 Compile/clippy issues fixed during this session

We fixed:

```text
String vs &str mismatch in wallet balance DTO construction
clippy::result-large-err for Result<_, Response>
```

The `result-large-err` issue was fixed by replacing large `Response` error variants with a small `WalletHoldProblem` struct and converting it to `Response` only at the route boundary.

---

## 4. omnigate — what remains

### Priority A — Add direct wallet façade tests

Add tests for:

```text
POST /v1/wallet/hold happy path forwards to svc-wallet
x-ron-wallet-account mismatch returns 400
asset != roc returns 400
amount_minor non-integer returns 400
amount_minor zero returns 400
upstream svc-wallet connect failure returns 502/problem JSON
idempotency-key header is forwarded
request id/correlation headers are forwarded where expected
```

The route is live-proven, but it needs dedicated contract tests.

### Priority B — Document wallet hold route contract

Update WEB3_2 route contract docs with:

```text
POST /v1/wallet/hold
POST /wallet/hold through gateway
request shape
response shape
error taxonomy
security assumptions
idempotency behavior
```

### Priority C — Clarify balance held semantics

Current balance:

```text
available decreases after hold
held_minor_units remains 0
```

Decide whether:

```text
held_minor_units should expose reserved holds
```

or whether the field should be renamed/removed until the ledger/wallet model exposes active holds clearly.

### Priority D — Harden paid proof docs

Document the exact paid proof headers:

```text
x-ron-paid-op
x-ron-paid-asset
x-ron-paid-estimate-minor
x-ron-wallet-txid
x-ron-wallet-receipt-hash
x-ron-wallet-from
x-ron-wallet-to
```

Make sure CrabLink and docs do not use the wrong `x-ron-wallet-hold-txid` header.

### Priority E — Prepare for image upload UI

CrabLink will next call `/assets/image` from the browser after a successful hold. Ensure omnigate’s image route remains strict:

```text
requires paid proof
requires correct txid/receipt hash headers
requires image content type
stores bytes by b3 content ID
stores manifest
stores index pointer
returns crab_url
does not accept spoofed ownership without passport/wallet context
```

### Priority F — Site creation proof

The next backend product proof after image UI should be site creation:

```text
crab://site
→ prepare
→ hold
→ POST /sites
→ resolve crab://<site_name>
```

Before UI work, inspect and document:

```text
crates/omnigate/src/routes/v1/sites.rs
crates/omnigate/tests/site_launch.rs
```

Confirm exact site-create request shape and paid-proof expectations.

---

# PART B — svc-gateway notes

## 5. svc-gateway — what changed

### 5.1 Added public wallet hold route

We added/updated the product gateway route:

```text
POST /wallet/hold
```

This proxies to:

```text
omnigate POST /v1/wallet/hold
```

The relevant file:

```text
crates/svc-gateway/src/routes/product.rs
```

### 5.2 Public product route surface after this session

The `svc-gateway` product router now exposes:

```text
GET  /identity/me
POST /identity/passport/bootstrap

GET  /wallet/:account/balance
POST /wallet/hold

GET  /crab/resolve?url=...
GET  /b3/:asset

POST /paid/o/prepare

POST /assets/image/prepare
POST /assets/image

POST /sites/prepare
POST /sites
GET  /sites/:name
```

### 5.3 Preserved gateway boundary

`svc-gateway` remains proxy-only.

It does not:

```text
parse manifests
hydrate asset pages
price storage
create wallet holds directly
mutate ledger
store bytes
write index pointers
capture/release ROC
```

Its job is:

```text
stable browser/client path
selected header forwarding
body forwarding
query forwarding
hop-by-hop header filtering
return upstream response body/status
```

### 5.4 Header forwarding behavior

The product proxy forwards selected headers:

```text
Authorization
Accept
Content-Type
x-ron-*
x-correlation-id
x-request-id
idempotency-key
```

It filters hop-by-hop and unsafe headers such as:

```text
Host
Connection
Proxy-Authorization
TE
Trailer
Transfer-Encoding
Upgrade
Content-Length
```

This is important for gateway correctness and avoiding proxy bugs.

### 5.5 Wallet hold route live proof

The user successfully created a wallet hold from CrabLink through:

```text
CrabLink full-tab browser
→ svc-gateway POST /wallet/hold
→ omnigate POST /v1/wallet/hold
→ svc-wallet POST /v1/hold
```

The balance moved from:

```text
1776 ROC
```

to:

```text
752 ROC
```

after the `1024` site hold.

Then another image hold of `68` moved balance to:

```text
684 ROC
```

This proves the public gateway route reaches the wallet path correctly.

---

## 6. svc-gateway — paid image upload path

### 6.1 Existing route used successfully

The already-exposed public route:

```text
POST /assets/image
```

successfully proxied to `omnigate` and returned:

```text
omnigate.image-asset-upload.v1
```

This proved `svc-gateway` correctly passed:

```text
raw image body
Content-Type: image/png
Authorization
idempotency-key
x-ron-paid-* headers
x-ron-wallet-* headers
x-ron-passport
x-ron-wallet-account
x-ron-asset-title
x-ron-asset-description
x-ron-asset-tags
```

### 6.2 Successful upload returned final crab URL

```text
crab://984128e643b594b1ff15ed2c40cf1d589616b9ddb7b212d00e91670997c1b8e4.image
```

The gateway also successfully resolved:

```text
GET /crab/resolve?url=crab://984128e643b594b1ff15ed2c40cf1d589616b9ddb7b212d00e91670997c1b8e4.image
```

to:

```text
omnigate.asset-page.v1
```

This proves the browser-facing public route layer is working for the core WEB3_2 image flow.

---

## 7. svc-gateway — quality gates run

The user ran:

```bash
cargo fmt
cargo clippy -p svc-gateway --all-targets --no-deps -- -D warnings
cargo test -p svc-gateway --all-targets
```

Final status:

```text
svc-gateway clippy: green
svc-gateway tests: green
```

Important passing tests:

```text
tests/app_proxy.rs
tests/identity_routes_proxy.rs
tests/paid_storage_estimate_proxy.rs
tests/paid_storage_write_proxy.rs
tests/product_routes_proxy.rs
tests/smoke.rs
```

### 7.1 Compile/clippy issues fixed during this session

We fixed `clippy::doc_markdown` errors in the product route header comments by wrapping path-like and project-specific terms in backticks.

Examples:

```text
SVC_GATEWAY_OMNIGATE_BASE_URL
tests/product_routes_proxy.rs
tests/identity_routes_proxy.rs
CrabLink
```

---

## 8. svc-gateway — what remains

### Priority A — Add direct `/wallet/hold` proxy test

The route is live-proven, but a dedicated gateway test should be added.

Add coverage to either:

```text
crates/svc-gateway/tests/identity_routes_proxy.rs
```

or:

```text
crates/svc-gateway/tests/product_routes_proxy.rs
```

Test expectations:

```text
POST /wallet/hold targets /v1/wallet/hold upstream
body is preserved
Authorization is forwarded
Content-Type is forwarded
idempotency-key is forwarded
x-ron-passport is forwarded
x-ron-wallet-account is forwarded
x-correlation-id is forwarded
upstream errors pass through
connect failure yields structured 502 problem
```

### Priority B — Update route contracts

Update docs to include:

```text
POST /wallet/hold
```

in the public gateway route list.

Document that the gateway route is a proxy only:

```text
client → svc-gateway /wallet/hold → omnigate /v1/wallet/hold → svc-wallet /v1/hold
```

### Priority C — Add optional paid image upload smoke

The manual terminal flow should be converted into an optional smoke path.

Possible script location:

```text
crablink/scripts/smoke-local-gateway.sh
```

or RustyOnions:

```text
scripts/web3_crablink_dev_stack.sh
scripts/web3_product_stack_smoke.sh
```

Suggested opt-in env:

```text
CRABLINK_SMOKE_RUN_UPLOAD=1
```

Default should remain non-mutating unless explicitly enabled.

The upload smoke should:

```text
create tiny PNG
create wallet hold
upload image with paid proof headers
assert omnigate.image-asset-upload.v1
resolve returned crab_url
assert omnigate.asset-page.v1
assert receipt present
```

### Priority D — Keep gateway route surface stable for CrabLink

CrabLink now depends on these public paths:

```text
/healthz
/readyz
/identity/me
/identity/passport/bootstrap
/wallet/:account/balance
/wallet/hold
/crab/resolve?url=...
/b3/:hash.kind
/sites/prepare
/assets/image/prepare
/assets/image
```

Future changes must preserve these paths or provide migration notes.

### Priority E — Site create route verification

Before wiring CrabLink site-create UI, confirm the public gateway `/sites` route forwards all required paid proof headers and body correctly.

Existing route:

```text
POST /sites
```

Needed next:

```text
manual terminal proof for site create
then UI wiring
then gateway test if missing
```

---

# PART C — joint proof / live commands

## 9. Live proof startup commands

### Terminal A

```bash
cd /Users/mymac/Desktop/RustyOnions
RUST_LOG=info SVC_WALLET_ADDR=127.0.0.1:8088 cargo run -p svc-wallet
```

### Terminal B

```bash
cd /Users/mymac/Desktop/RustyOnions
OMNIGATE_WALLET_BASE_URL=http://127.0.0.1:8088 \
OMNIGATE_WALLET_BEARER=dev \
scripts/web3_crablink_dev_stack.sh
```

### Backend health checks

```bash
curl -i http://127.0.0.1:8090/healthz
curl -i http://127.0.0.1:8090/readyz

curl -s 'http://127.0.0.1:8090/crab/resolve?url=crab://site' | jq .
curl -s 'http://127.0.0.1:8090/crab/resolve?url=crab://image' | jq .

curl -s http://127.0.0.1:8090/wallet/acct_dev/balance \
  -H 'Authorization: Bearer dev' \
  -H 'x-ron-passport: passport:main:dev' \
  -H 'x-ron-wallet-account: acct_dev' | jq .
```

---

## 10. Successful paid image upload command

Reference command that succeeded:

```bash
cd /Users/mymac/Desktop/RustyOnions

HOLD_TXID=$(jq -r '.txid' artifacts/crablink-manual/tiny-hold.json)
HOLD_HASH=$(jq -r '.receipt_hash' artifacts/crablink-manual/tiny-hold.json)
HOLD_FROM=$(jq -r '.from' artifacts/crablink-manual/tiny-hold.json)
HOLD_TO=$(jq -r '.to' artifacts/crablink-manual/tiny-hold.json)
HOLD_AMOUNT=$(jq -r '.amount_minor' artifacts/crablink-manual/tiny-hold.json)

curl -s http://127.0.0.1:8090/assets/image \
  -X POST \
  -H 'Authorization: Bearer dev' \
  -H 'Content-Type: image/png' \
  -H 'Accept: application/json' \
  -H 'x-ron-paid-op: hold' \
  -H 'x-ron-paid-asset: roc' \
  -H "x-ron-paid-estimate-minor: ${HOLD_AMOUNT}" \
  -H "x-ron-wallet-txid: ${HOLD_TXID}" \
  -H "x-ron-wallet-receipt-hash: ${HOLD_HASH}" \
  -H "x-ron-wallet-from: ${HOLD_FROM}" \
  -H "x-ron-wallet-to: ${HOLD_TO}" \
  -H 'x-ron-passport: passport:main:dev' \
  -H 'x-ron-wallet-account: acct_dev' \
  -H 'x-ron-asset-title: Tiny CrabLink Test Image' \
  -H 'x-ron-asset-description: First terminal-proven CrabLink paid image upload.' \
  -H 'x-ron-asset-tags: crablink,tiny,proof' \
  -H 'idempotency-key: crablink-upload:image:tiny-crablink.png' \
  --data-binary @artifacts/crablink-manual/tiny-crablink.png \
  | tee artifacts/crablink-manual/tiny-upload.json | jq .
```

Resolve:

```bash
CRAB_URL=$(jq -r '.crab_url // .links.crab // .asset.crab_url // empty' artifacts/crablink-manual/tiny-upload.json)

curl -s "http://127.0.0.1:8090/crab/resolve?url=${CRAB_URL}" \
  -H 'Authorization: Bearer dev' \
  -H 'x-ron-passport: passport:main:dev' \
  -H 'x-ron-wallet-account: acct_dev' | jq .
```

---

# PART D — cross-crate next-step plan

## 11. Immediate next work

### 11.1 CrabLink UI next

The next implementation should wire the already-proven terminal path into the CrabLink full-tab UI:

```text
Prepare Image
→ Confirm ROC Hold
→ Submit Image Upload
→ show final crab://<hash>.image
→ auto-open the new asset page
→ refresh balance
```

This will require CrabLink changes, not necessarily new RustyOnions crate changes unless gaps appear.

### 11.2 omnigate next

Add dedicated tests for `/v1/wallet/hold`.

### 11.3 svc-gateway next

Add dedicated proxy tests for `/wallet/hold`.

### 11.4 Docs next

Update route contracts to include:

```text
/wallet/hold
paid image upload proof headers
image upload flow
returned asset page fields
```

### 11.5 Smoke next

Add optional mutating smoke:

```text
CRABLINK_SMOKE_RUN_UPLOAD=1
```

The default smoke should not consume ROC unless explicitly requested.

---

## 12. Success statement

This session proved the first real RustyOnions / CrabLink creator economy loop:

```text
pay ROC
→ upload image
→ receive b3 content ID
→ receive crab:// image URL
→ resolve hydrated asset page
→ show ownership, payout, storage, manifest, and receipt metadata
```

This is a major WEB3_2 milestone.

The remaining work is no longer proving whether the architecture can support the product loop. It can. The remaining work is polishing, testing, documenting, and wiring the proven backend path into the browser UI.



### END NOTES - MAY 2 2026 - 13:55 CST



### BEGIN NOTE - MAY 3 2026 - 13:30 CST

Below is a paste-ready crate note for `crates/svc-gateway/NOTES.MD`.

Context anchors: before this session, the Rust codebundle still showed `svc-gateway/src/routes/objects.rs` as a temporary `/o/{addr}` object stub, and the router composition shown there did not yet mount a real object proxy path into the public router.  The existing gateway product-route notes already established the correct boundary: `svc-gateway` must stay proxy-only, forwarding selected headers/bodies/query strings and never owning hydration, pricing, storage, wallet mutation, ledger mutation, or product business logic.  The broader NEXT_LEVEL direction remains to harden image/site flows and asset-page rendering before expanding into `.post/.comment/.article`. 

---

# svc-gateway NOTES — May 03 2026 — Raw Object Proxy for CrabLink Asset Previews

## 0. Executive summary

This session made `svc-gateway` usable for browser-native raw asset previews by wiring the public raw-object read path:

```text
GET  /o/b3:<64hex>
HEAD /o/b3:<64hex>
```

through `svc-gateway` to `svc-storage`.

Before this work, CrabLink could successfully resolve and display a hydrated `omnigate.asset-page.v1` page for a paid image asset, but the actual image preview could not load because the public gateway path `/o/b3:<hash>` either returned `404`, hit a temporary stub, or failed to reach the correct storage upstream.

After this work and correct dev-stack configuration, CrabLink successfully loaded a real image asset preview through:

```text
CrabLink asset page
→ img src http://127.0.0.1:8090/o/b3:<hash>
→ svc-gateway
→ svc-storage
→ raw image bytes
→ browser preview
```

This is a major product milestone because it closes the browser-facing loop:

```text
pay ROC
→ upload image
→ receive b3 CID
→ receive crab://<hash>.image URL
→ resolve asset page
→ show metadata/ownership/payout/storage
→ load the actual image preview through the public gateway
```

The successful current test image URL is:

```text
crab://2e24f045f01a1bc77c57a94d622365e6b291936fcdd3ae64b45b0578e99c2058.image
```

## 1. Files changed in this crate

Primary files changed:

```text
crates/svc-gateway/src/routes/objects.rs
crates/svc-gateway/src/routes/mod.rs
```

No `Cargo.toml` changes were required.

No wallet, ledger, storage, index, omnigate, or policy crate code was changed in this session.

## 2. Starting problem

CrabLink image asset pages were resolving, but the actual image preview was broken.

The asset page DTO was valid and contained a raw route like:

```text
/o/b3:e76e12c9017bbf4d83471b5f134cf2afdad301123b29abf0627a1bc69070a754
```

The page showed:

```text
storage.available = true
size_bytes = 287972
links.raw = /o/b3:<hash>
warnings = []
```

But the browser preview could not display the image.

The direct tests proved the failure was at the gateway/storage raw object boundary:

```text
GET/HEAD http://127.0.0.1:8090/o/b3:<hash>  → 404 or 502
GET/HEAD http://127.0.0.1:5303/o/b3:<hash>  → 200 OK, content-length present
```

This meant:

```text
svc-storage had the bytes.
omnigate had the asset metadata.
CrabLink was using the right raw route.
svc-gateway was the missing/failing layer.
```

## 3. What was implemented

### 3.1 Replaced temporary object stub with storage proxy

`crates/svc-gateway/src/routes/objects.rs` was changed from a temporary object stub into a real proxy module.

New responsibility:

```text
GET  /o/:addr → svc-storage /o/:addr
HEAD /o/:addr → svc-storage /o/:addr
```

Important behavior:

```text
- Accepts the object address from the public gateway path.
- Builds the upstream storage URL from cfg.upstreams.storage_base_url.
- Proxies GET and HEAD to svc-storage.
- Copies upstream status code.
- Copies upstream response body.
- Copies safe response headers.
- Adds application/octet-stream as a fallback Content-Type for GET when storage does not provide one.
- Maps storage connect failures to gateway upstream_unavailable / 502 behavior.
- Maps storage body-read failures to upstream_unavailable / 502 behavior.
```

### 3.2 Mounted `/o/:addr` in the gateway router

`crates/svc-gateway/src/routes/mod.rs` was updated so the public router now includes object routes:

```text
/o/:addr
```

with:

```text
GET  objects::get_object
HEAD objects::head_object
```

The route was wrapped with the same kind of public gateway concerns as other public routes:

```text
correlation middleware
HTTP metrics middleware
```

This preserves observability and request tracing.

### 3.3 Header filtering was preserved

The object proxy forwards request headers carefully and filters unsafe/hop-by-hop headers.

Forwarding behavior remains aligned with the gateway proxy discipline:

```text
Allowed / expected to pass through:
- Authorization
- Accept
- x-ron-*
- x-correlation-id
- x-request-id
- idempotency-key when present
```

Headers that must not be forwarded as hop-by-hop or transport-specific state include:

```text
Host
Connection
Proxy-Authorization
TE
Trailer
Transfer-Encoding
Upgrade
```

Response header copying also filters hop-by-hop headers.

### 3.4 Gateway boundary preserved

This change intentionally does **not** make `svc-gateway` own storage semantics.

`svc-gateway` still does not:

```text
- store bytes
- verify BLAKE3 itself
- parse manifests
- resolve names
- hydrate asset pages
- price uploads
- create wallet holds
- capture/release ROC
- mutate wallet
- mutate ledger
- write index pointers
```

It only exposes a stable browser/client path and proxies to the correct owner service.

Ownership remains:

```text
svc-gateway = public edge/proxy
svc-storage = raw immutable bytes by b3 CID
omnigate = asset/site hydration and product DTOs
svc-index = name/manifest pointers
svc-wallet = economic mutation front-door
ron-ledger = durable economic truth
CrabLink = thin gateway-only browser client
```

## 4. Debugging path and proof

### 4.1 Initial symptom

CrabLink opened a valid asset page:

```text
crab://e76e12c9017bbf4d83471b5f134cf2afdad301123b29abf0627a1bc69070a754.image
```

but clicking image preview produced a broken image icon.

### 4.2 Direct gateway test initially failed

Test:

```bash
HASH="e76e12c9017bbf4d83471b5f134cf2afdad301123b29abf0627a1bc69070a754"
CID="b3:${HASH}"

curl -I "http://127.0.0.1:8090/o/${CID}" \
  -H 'Authorization: Bearer dev' \
  -H 'x-ron-passport: passport:main:dev' \
  -H 'x-ron-wallet-account: acct_dev'
```

Observed before final stack fix:

```text
HTTP/1.1 502 Bad Gateway
```

and later:

```text
HTTP/1.1 404 Not Found
```

### 4.3 Direct storage test proved bytes existed

Test:

```bash
curl -I "http://127.0.0.1:5303/o/${CID}" \
  -H 'Authorization: Bearer dev' \
  -H 'x-ron-passport: passport:main:dev' \
  -H 'x-ron-wallet-account: acct_dev'
```

Observed:

```text
HTTP/1.1 200 OK
etag: "<hash>"
content-length: 287972
```

This proved the object existed in `svc-storage`.

### 4.4 Root cause of 502

The new gateway proxy was trying to reach the wrong storage upstream because the dev stack was not launched with the correct storage base URL.

The real CrabLink dev storage instance was:

```text
http://127.0.0.1:5303
```

The gateway needed:

```text
SVC_GATEWAY_STORAGE_BASE_URL=http://127.0.0.1:5303
```

Without that, `/o/:addr` could be mounted correctly but still fail to reach the storage instance that actually had the object.

### 4.5 Port collision during restart

A failed restart showed:

```text
error: svc-gateway bind address 127.0.0.1:8090 is already in use by another process.
```

Meaning Chrome was still talking to the old gateway process. Important operational lesson:

```text
If /o behavior does not change after code/config changes, check for stale gateway process on 8090.
```

Useful check:

```bash
lsof -nP -iTCP:8090 -sTCP:LISTEN
```

### 4.6 Correct dev-stack startup shape

The correct local dev startup now needs both the wallet and storage upstream config, and also the ROC economics config so image prepare does not fall back to legacy byte-based pricing:

```bash
cd /Users/mymac/Desktop/RustyOnions

ECON_PATH="$(pwd)/configs/roc-economics.dev.toml"

env \
  OMNIGATE_WALLET_BASE_URL=http://127.0.0.1:8088 \
  OMNIGATE_WALLET_BEARER=dev \
  RON_STORAGE_ROC_ECONOMICS_PATH="$ECON_PATH" \
  RON_STORAGE_ROC_ECONOMICS_ACTION=paid_storage_put \
  SVC_GATEWAY_STORAGE_BASE_URL=http://127.0.0.1:5303 \
  scripts/web3_crablink_dev_stack.sh
```

Important gotcha found during the session:

```text
Do not write a line-continuation backslash followed by a space.
```

This was bad:

```bash
OMNIGATE_WALLET_BEARER=dev \ 
```

Use the `env \` form above to avoid that shell issue.

## 5. Quality gates run

The user ran the following from the RustyOnions repo:

```bash
cargo fmt -p svc-gateway
cargo clippy -p svc-gateway --all-targets --no-deps -- -D warnings
cargo test -p svc-gateway --test product_routes_proxy
```

Final result:

```text
cargo fmt -p svc-gateway                               green
cargo clippy -p svc-gateway --all-targets --no-deps    green
cargo test -p svc-gateway --test product_routes_proxy  green
```

`product_routes_proxy` result:

```text
running 6 tests
6 passed
```

The six product proxy tests that remained green:

```text
crab_resolve_proxy_preserves_query_and_headers
b3_asset_proxy_targets_omnigate_b3_route
paid_prepare_proxy_preserves_body_and_idempotency
image_and_site_product_routes_target_omnigate
site_resolve_proxy_targets_omnigate_site_hydrator
product_route_upstream_connect_failure_yields_problem_502
```

During implementation, strict clippy found and we fixed:

```text
clippy::doc_markdown
clippy::manual_let_else
```

This was important because `svc-gateway` uses strict/pedantic linting.

## 6. Product proof achieved through svc-gateway

After the gateway raw object route and dev-stack config were correct, a new paid image upload produced:

```text
crab://2e24f045f01a1bc77c57a94d622365e6b291936fcdd3ae64b45b0578e99c2058.image
```

CrabLink then resolved the asset page through `svc-gateway`, and the preview loaded through:

```text
GET /o/b3:2e24f045f01a1bc77c57a94d622365e6b291936fcdd3ae64b45b0578e99c2058
```

This proves:

```text
GET /crab/resolve?url=crab://<hash>.image  works for metadata/page hydration
GET /o/b3:<hash>                           works for raw image preview bytes
POST /assets/image/prepare                 works for non-mutating prepare
POST /wallet/hold                          works through the public wallet proxy path
POST /assets/image                         works for paid image upload
```

Only `svc-gateway` code changed for the raw byte preview part. CrabLink UI work made it visible and professional, but the RustyOnions crate fix was the gateway raw object proxy.

## 7. Important local dev config lesson

There are now **two separate local dev config requirements** that must both be present:

### 7.1 Correct storage upstream for raw preview

Required for image preview:

```text
SVC_GATEWAY_STORAGE_BASE_URL=http://127.0.0.1:5303
```

Without this, gateway `/o/b3:<hash>` may return:

```text
502 Bad Gateway
```

even when `svc-storage` has the object.

### 7.2 Correct ROC economics for 25 ROC dev image pricing

Required for expected 25 ROC image upload price:

```text
RON_STORAGE_ROC_ECONOMICS_PATH=/Users/mymac/Desktop/RustyOnions/configs/roc-economics.dev.toml
RON_STORAGE_ROC_ECONOMICS_ACTION=paid_storage_put
```

Without this, `POST /assets/image/prepare` may fall back to legacy byte pricing and return absurd estimates like:

```text
bytes = 121927
amount = 121927
minimum_hold = 121927
```

That is not a CrabLink issue and not a gateway pricing issue. It means the local storage/economics config was not loaded.

Still, `svc-gateway` users will experience it through gateway routes, so the dev stack script should make these env vars hard to forget.

## 8. What remains for svc-gateway

### 8.1 Add dedicated object proxy tests

Current `product_routes_proxy` remained green, but there should be a dedicated test file for the new raw object proxy path.

Recommended test file:

```text
crates/svc-gateway/tests/objects_proxy.rs
```

Recommended test coverage:

```text
GET /o/b3:<hash> proxies to svc-storage /o/b3:<hash>
HEAD /o/b3:<hash> proxies to svc-storage /o/b3:<hash>
GET copies upstream status code
GET copies upstream body bytes exactly
HEAD does not require/return body
ETag from storage is preserved
Content-Length from storage is preserved
Content-Type from storage is preserved
Content-Type fallback is application/octet-stream when absent
404 from storage passes through as 404
500 from storage behavior is deterministic
storage connect failure returns structured 502/upstream_unavailable
hop-by-hop request headers are not forwarded
hop-by-hop response headers are not copied
x-ron-* headers are forwarded where appropriate
Authorization is forwarded where appropriate
x-correlation-id / x-request-id behavior remains stable
```

Acceptance:

```text
cargo test -p svc-gateway --test objects_proxy
cargo clippy -p svc-gateway --all-targets --no-deps -- -D warnings
```

### 8.2 Decide whether to implement `/o/:addr` range support

`crates/svc-gateway/src/routes/objects_range.rs` still exists as a range-read placeholder/stub area.

Future work:

```text
GET /o/:addr with Range header
→ forward Range to svc-storage
→ preserve 206 Partial Content
→ preserve Content-Range
→ preserve Accept-Ranges
→ preserve ETag
→ preserve 416 Range Not Satisfiable
```

This matters later for:

```text
video previews
music/audio assets
large image/media reads
browser seek support
svc-edge integration
```

Do not overbuild this before image/site flows are stable, but it is the natural next backend read-path improvement.

### 8.3 Update dev stack script so the storage base URL is automatic

The session exposed that `scripts/web3_crablink_dev_stack.sh` can launch services on:

```text
svc-storage 127.0.0.1:5303
svc-gateway 127.0.0.1:8090
```

while `svc-gateway` may still default to a different storage upstream unless explicitly configured.

Recommended script fix:

```text
When launching svc-gateway inside scripts/web3_crablink_dev_stack.sh,
always set:
SVC_GATEWAY_STORAGE_BASE_URL=http://127.0.0.1:5303
```

Also print this value in the stack banner so debugging is obvious.

### 8.4 Add script-level port conflict diagnostics

The restart failed once because an old gateway still held port `8090`.

Recommended script improvement:

```text
Before launching, check:
127.0.0.1:8090 gateway
127.0.0.1:9090 omnigate
127.0.0.1:5303 storage
127.0.0.1:5304 index
127.0.0.1:8088 wallet if expected
```

If a port is occupied, print:

```text
port
owning PID
process name
suggested safe next action
```

This would prevent accidentally testing against stale gateways.

### 8.5 Add route-contract docs for `/o/b3:<hash>`

Update product route docs to include:

```text
GET  /o/b3:<64hex>
HEAD /o/b3:<64hex>
```

Contract:

```text
Purpose:
  Public raw immutable object read path by canonical b3 CID.

Owner:
  svc-gateway exposes/proxies; svc-storage owns bytes.

Not responsible for:
  manifest hydration, name resolution, pricing, wallet, ledger.

Expected success:
  200 OK
  body bytes on GET
  no required body on HEAD
  ETag when storage provides it
  Content-Length when storage provides it
  Content-Type when storage provides it or application/octet-stream fallback

Expected failures:
  404 object not found
  502 storage unavailable/connect/read failure
```

### 8.6 Consider stricter address validation later

Current gateway behavior is proxy-oriented. It should not own product semantics, but it may still be appropriate for gateway hardening to validate obvious path hygiene before proxying.

Possible future validation:

```text
Allow:
  b3:<64 lowercase hex>

Reject:
  empty addr
  path traversal
  slash-containing addr after /o/
  non-b3 raw paths unless explicitly supported
```

This should be done carefully so `svc-gateway` does not duplicate `svc-storage` semantics unnecessarily. If added, keep it as ingress hygiene, not product/business validation.

### 8.7 Improve metrics/prewarming for object route

The object route inherits HTTP metrics middleware, but dashboards may need explicit route labels/prewarm for:

```text
/o/:addr
```

Future improvement:

```text
prewarm http metric labels for GET /o/:addr and HEAD /o/:addr
track upstream storage connect/read failures by reason
track object proxy status class
```

Potential reason labels:

```text
storage_connect
storage_read
bad_method
```

### 8.8 Confirm full svc-gateway test suite after object proxy tests

Current session confirmed:

```text
clippy green
product_routes_proxy green
```

After adding object proxy tests, run a broader gate:

```bash
cargo fmt -p svc-gateway
cargo clippy -p svc-gateway --all-targets --no-deps -- -D warnings
cargo test -p svc-gateway --all-targets
```

And then live smoke:

```bash
scripts/web3_product_stack_smoke.sh
RON_RUN_SITE_CREATE=1 scripts/web3_product_stack_smoke.sh
```

Plus CrabLink smoke once the extension-side script includes upload:

```bash
cd /Users/mymac/Desktop/crablink
scripts/check-chrome.sh && scripts/package-chrome.sh
scripts/smoke-local-gateway.sh
CRABLINK_SMOKE_RUN_BOOTSTRAP=1 scripts/smoke-local-gateway.sh
CRABLINK_SMOKE_RUN_UPLOAD=1 scripts/smoke-local-gateway.sh
```

### 8.9 Do not move gateway into product semantics

As NEXT_LEVEL progresses toward `.post`, `.comment`, `.article`, `.music`, `.video`, etc., keep this crate disciplined.

`svc-gateway` should keep adding stable public routes and safe proxy behavior, not product ownership.

For new asset kinds, gateway should generally not need bespoke logic beyond routing/proxying:

```text
crab://<hash>.post      → /crab/resolve?url=...
crab://<hash>.comment   → /crab/resolve?url=...
crab://<hash>.article   → /crab/resolve?url=...
crab://<hash>.music     → /crab/resolve?url=...
crab://<hash>.video     → /crab/resolve?url=...
```

Hydration belongs in `omnigate`. Raw bytes belong in `svc-storage` or later `svc-edge`. Names/pointers belong upstream in `svc-index`/`ron-naming`.

## 9. Known gotchas from this session

### 9.1 `curl -i -o file` writes headers into the output file

During debugging, this made `file` and `xxd` show:

```text
HTTP/1.1 502 Bad
```

at the beginning of the saved file.

Use this form instead:

```bash
curl -sS -D /tmp/headers.txt \
  -o /tmp/body.bin \
  "http://127.0.0.1:8090/o/${CID}" \
  -H 'Authorization: Bearer dev' \
  -H 'x-ron-passport: passport:main:dev' \
  -H 'x-ron-wallet-account: acct_dev'

cat /tmp/headers.txt
file /tmp/body.bin
xxd -l 16 /tmp/body.bin
```

### 9.2 Old objects may disappear after dev stack restart

If the stack uses temp/in-memory storage, a previously uploaded `crab://<hash>.image` may still resolve as an asset shell or from index state, but raw bytes may be missing after restart.

The reliable proof is:

```bash
curl -I http://127.0.0.1:5303/o/b3:<hash>
curl -I http://127.0.0.1:8090/o/b3:<hash>
```

Both should return `200` for current-live objects.

### 9.3 Gateway `502` vs `404` tells different stories

Interpretation:

```text
Gateway 502, storage direct 200:
  gateway is mounted but points at wrong/unreachable storage upstream.

Gateway 404, storage direct 404:
  object is missing from current storage instance.

Gateway 404, storage direct 200:
  gateway route/proxy path or upstream base URL/path construction is wrong.

Gateway 200, browser broken:
  check content type, CSP, CSS, or image renderer.
```

## 10. Current status

`svc-gateway` is in a stronger place after this session.

Green/proven:

```text
- product route proxy tests still pass
- clippy strict gate passes
- gateway /o route now exists and proxies to storage when configured correctly
- CrabLink can load an actual image preview through gateway /o
- gateway remains proxy-only
- no wallet/ledger/storage ownership leaked into gateway
```

Still needed:

```text
- dedicated object proxy tests
- range proxy support or explicit deferral
- dev stack script must always set SVC_GATEWAY_STORAGE_BASE_URL
- route-contract docs should include /o/b3:<hash>
- smoke script should include raw object preview/read verification
- port-conflict diagnostics should be improved
```

## 11. Suggested next svc-gateway task

Recommended next focused batch:

```text
Batch: svc-gateway object proxy hardening
```

Scope:

```text
1. Add crates/svc-gateway/tests/objects_proxy.rs.
2. Test GET /o/b3:<hash>.
3. Test HEAD /o/b3:<hash>.
4. Test 404 pass-through.
5. Test 502 storage connect failure.
6. Test header filtering.
7. Add /o/b3:<hash> to route-contract docs.
8. Update web3_crablink_dev_stack.sh to export SVC_GATEWAY_STORAGE_BASE_URL=http://127.0.0.1:5303.
9. Optionally update CrabLink/RustyOnions smoke to check GET /o on the returned image CID.
```

Acceptance:

```text
cargo fmt -p svc-gateway
cargo clippy -p svc-gateway --all-targets --no-deps -- -D warnings
cargo test -p svc-gateway --test objects_proxy
cargo test -p svc-gateway --test product_routes_proxy
manual CrabLink open of crab://2e24f045f01a1bc77c57a94d622365e6b291936fcdd3ae64b45b0578e99c2058.image shows image preview automatically
```

## 12. Success statement

This session moved `svc-gateway` from “product DTO proxy only” to “product DTO proxy plus raw content-byte read path,” which is essential for browser-quality CrabLink asset pages.

The route now supports the actual user-facing pattern:

```text
crab://<hash>.image
→ hydrated asset page via /crab/resolve
→ image preview via /o/b3:<hash>
```

That is exactly the clean gateway role for RustyOnions WEB3_2 and NEXT_LEVEL: public stable paths at the edge, but no business logic or durable truth inside the gateway.


### END NOTE - MAY 3 2026 - 13:30 CST


### BEGIN NOTE - MAY 6 2026 - 17:55 CST

# RustyOnions Carryover Notes

## Set 6 — `svc-gateway` Crate Notes

Drop this into the `svc-gateway` crate notes.

---

# `svc-gateway` Crate Carryover Notes

## NEXT_LEVEL / WEB3_2 Public Profile Proxy Edge Layer

## 1. Crate role

`svc-gateway` is the public HTTP edge for CrabLink and browser/client-facing RustyOnions product routes.

It should expose stable external paths while keeping internal service topology hidden behind the gateway.

Correct boundary:

```text
CrabLink
→ svc-gateway
→ omnigate
→ svc-passport / svc-wallet / svc-storage / svc-index
```

For NEXT_LEVEL identity/profile work, `svc-gateway` is responsible for:

```text
Public browser-safe route names.
Forwarding selected request headers.
Preserving request bodies.
Preserving query strings.
Passing Omnigate responses through.
Mapping Omnigate connect/read failures to structured 502.
Keeping CrabLink gateway-only.
```

It is **not** responsible for:

```text
Username ownership.
Profile persistence.
Passport creation semantics.
Wallet mutation.
Ledger mutation.
ROC issuance.
Storage writes.
Manifest parsing.
Profile hydration logic.
Reputation/moderation scoring.
Alt identity policy.
```

Important invariant:

```text
svc-gateway remains proxy-only.
```

---

## 2. Session summary

In this session, `svc-gateway` was extended to expose public profile claim/read routes for CrabLink.

New public gateway routes:

```text
POST /identity/passport/profile/claim
GET  /identity/passport/profile/:username
```

These proxy to Omnigate routes:

```text
POST /v1/identity/passport/profile/claim
GET  /v1/identity/passport/profile/:username
```

Which then proxy to `svc-passport` routes:

```text
POST /v1/passport/profile/claim
GET  /v1/passport/profile/:username
```

Main accomplishments:

```text
Added public profile claim route to gateway.
Added public profile lookup route to gateway.
Preserved idempotency-key and x-ron-* identity headers.
Preserved x-correlation-id and x-request-id.
Filtered hop-by-hop headers.
Preserved upstream 404 profile_not_found.
Preserved upstream 409 username_unavailable.
Mapped Omnigate connect/read failure to structured 502.
Fixed /identity/me query forwarding.
Expanded identity_routes_proxy.rs tests.
Fixed final Clippy collapsible-if lint.
Got svc-gateway tests green.
Got svc-gateway focused Clippy green.
```

---

## 3. Files changed / added

Primary files touched:

```text
crates/svc-gateway/src/routes/product.rs
crates/svc-gateway/tests/identity_routes_proxy.rs
```

No `Cargo.toml` change was needed.

---

## 4. Public routes now exposed

`product.rs` now exposes these product-facing routes:

```text
GET  /identity/me
POST /identity/passport/bootstrap
POST /identity/passport/profile/claim
GET  /identity/passport/profile/:username
GET  /wallet/:account/balance
POST /wallet/hold
GET  /crab/resolve?url=...
GET  /b3/:asset
POST /paid/o/prepare
POST /assets/image/prepare
POST /assets/image
POST /sites/prepare
POST /sites
GET  /sites/:name
```

The new profile routes are:

```text
POST /identity/passport/profile/claim
GET  /identity/passport/profile/:username
```

These are the **routes CrabLink should call next**.

CrabLink should **not** call:

```text
http://127.0.0.1:9090/v1/identity/passport/profile/claim
http://127.0.0.1:5307/v1/passport/profile/claim
```

CrabLink should call the gateway base URL only, typically:

```text
http://127.0.0.1:8090/identity/passport/profile/claim
http://127.0.0.1:8090/identity/passport/profile/:username
```

---

## 5. New profile route handlers

Added to `crates/svc-gateway/src/routes/product.rs`:

```rust
passport_profile_claim()
passport_profile_get()
```

### `passport_profile_claim`

Public route:

```text
POST /identity/passport/profile/claim
```

Proxies to Omnigate:

```text
POST /v1/identity/passport/profile/claim
```

Purpose:

```text
Expose profile username claim to CrabLink/browser clients through the gateway.
```

Important behavior:

```text
Forwards JSON request body unchanged.
Forwards selected identity/auth/idempotency headers.
Does not reserve usernames itself.
Does not persist profile truth.
Does not create passports.
Does not mutate wallets.
Does not touch ledger.
```

### `passport_profile_get`

Public route:

```text
GET /identity/passport/profile/:username
```

Proxies to Omnigate:

```text
GET /v1/identity/passport/profile/:username
```

Purpose:

```text
Expose read-only public profile lookup to CrabLink/browser clients through the gateway.
```

Important behavior:

```text
Forwards selected headers.
Does not hydrate profiles itself.
Does not interpret username ownership.
Does not fake profile responses.
```

---

## 6. `/identity/me` query forwarding fix

Before this session, `GET /identity/me?debug=1` dropped the query string when forwarding to Omnigate.

This was fixed.

Current handler:

```rust
pub async fn identity_me(
    State(state): State<AppState>,
    uri: Uri,
    headers: HeaderMap,
) -> Response {
    let upstream_path = with_query("/v1/identity/me", uri.query());

    proxy_to_omnigate(
        &state,
        Method::GET,
        &upstream_path,
        headers,
        Bytes::new(),
    )
    .await
}
```

Important lesson:

```text
Gateway GET proxy routes that accept query parameters must use with_query().
```

Existing query-preserving routes include:

```text
/identity/me
/crab/resolve
```

If future GET routes need query support, follow this pattern.

---

## 7. Header forwarding behavior

Gateway product proxy routes forward selected headers.

Forwarded request headers:

```text
Authorization
Accept
Content-Type
x-ron-*
x-correlation-id
x-request-id
idempotency-key
```

Filtered request headers:

```text
Host
Connection
Proxy-Authorization
TE
Trailer
Transfer-Encoding
Upgrade
Content-Length
```

Function:

```rust
fn should_forward_header(name: &HeaderName) -> bool
```

Current behavior:

```rust
fn should_forward_header(name: &HeaderName) -> bool {
    if is_hop_by_hop_or_host(name) || name == header::CONTENT_LENGTH {
        return false;
    }

    name == header::AUTHORIZATION
        || name == header::ACCEPT
        || name == header::CONTENT_TYPE
        || name.as_str().starts_with("x-ron-")
        || name.as_str() == "x-correlation-id"
        || name.as_str() == "x-request-id"
        || name.as_str() == "idempotency-key"
}
```

Important note:

```text
idempotency-key is matched as a string, not as header::IDEMPOTENCY_KEY.
```

The standard `http::header` module does not provide `IDEMPOTENCY_KEY`.

---

## 8. Response header copying behavior

Gateway copies response headers from Omnigate except problematic hop/body framing headers.

Filtered response headers:

```text
Transfer-Encoding
Content-Length
Connection
```

Function:

```rust
fn should_copy_response_header(name: &HeaderName) -> bool {
    name != header::TRANSFER_ENCODING
        && name != header::CONTENT_LENGTH
        && name != header::CONNECTION
}
```

Purpose:

```text
Avoid invalid body framing after the gateway rebuilds the response body.
```

---

## 9. Proxy function behavior

Core function:

```rust
async fn proxy_to_omnigate(
    state: &AppState,
    method: Method,
    upstream_path: &str,
    headers: HeaderMap,
    body: Bytes,
) -> Response
```

Behavior:

```text
Builds upstream URL from state.cfg.upstreams.omnigate_base_url.
Converts Axum/http Method into reqwest Method.
Forwards selected request headers.
Sends raw body bytes.
Preserves upstream status.
Copies allowed upstream response headers.
Copies upstream body bytes.
Maps bad method to upstream_unavailable("bad_method").
Maps send/connect failure to upstream_unavailable("omnigate_connect").
Maps body read failure to upstream_unavailable("omnigate_read").
```

Important invariant:

```text
Gateway does not inspect or mutate profile JSON.
Gateway does not inspect or mutate wallet JSON.
Gateway does not inspect or mutate site/image/paid route JSON except for proxying.
```

---

## 10. Error behavior

Gateway uses existing structured error helper:

```rust
errors::upstream_unavailable(reason)
```

For profile route upstream failure:

```text
HTTP 502
code: upstream_unavailable
retryable: true
```

Reasons used:

```text
bad_method
omnigate_connect
omnigate_read
```

Profile semantic errors from Omnigate are preserved:

### Missing profile

```text
HTTP 404
code: profile_not_found
retryable: false
```

### Username unavailable

```text
HTTP 409
code: username_unavailable
retryable: false
```

Important behavior:

```text
Gateway does not remap Omnigate semantic errors.
Gateway only maps transport/upstream availability failures.
```

---

## 11. New/expanded integration test file

File:

```text
crates/svc-gateway/tests/identity_routes_proxy.rs
```

This test spins up:

```text
dummy Omnigate on random local port
real svc-gateway on random local port
```

Then uses `reqwest` to call public gateway routes and confirm they proxy correctly.

The tests serialize environment mutation with:

```rust
static ENV_LOCK: Mutex<()> = Mutex::const_new(());
```

Environment variables used in tests:

```text
SVC_GATEWAY_OMNIGATE_BASE_URL
SVC_GATEWAY_BIND_ADDR
```

`clear_env()` removes both after each test.

---

## 12. `identity_routes_proxy.rs` tests

Current tests:

```text
identity_me_proxy_preserves_passport_headers
passport_bootstrap_proxy_preserves_body_and_idempotency
passport_profile_claim_proxy_preserves_body_headers_and_idempotency
passport_profile_lookup_proxy_targets_omnigate_profile_route
passport_profile_upstream_errors_pass_through
wallet_balance_proxy_targets_omnigate_wallet_balance
identity_route_upstream_connect_failure_yields_problem_502
```

All passed.

### `identity_me_proxy_preserves_passport_headers`

Verifies:

```text
GET /identity/me?debug=1 routes to /v1/identity/me?debug=1
debug=1 query string is preserved
Authorization is forwarded
x-ron-passport is forwarded
x-ron-wallet-account is forwarded
x-correlation-id is forwarded
x-request-id is forwarded
Connection is not forwarded
```

This test caught the original missing query-forwarding bug.

### `passport_bootstrap_proxy_preserves_body_and_idempotency`

Verifies:

```text
POST /identity/passport/bootstrap routes to /v1/identity/passport/bootstrap
JSON body is preserved
idempotency-key is forwarded
x-ron-passport is forwarded
x-ron-wallet-account is forwarded
```

### `passport_profile_claim_proxy_preserves_body_headers_and_idempotency`

Verifies:

```text
POST /identity/passport/profile/claim routes to /v1/identity/passport/profile/claim
JSON body is preserved
Authorization is forwarded
Content-Type is forwarded
idempotency-key is forwarded
x-ron-passport is forwarded
x-ron-wallet-account is forwarded
x-correlation-id is forwarded
x-request-id is forwarded
Connection is not forwarded
```

### `passport_profile_lookup_proxy_targets_omnigate_profile_route`

Verifies:

```text
GET /identity/passport/profile/skinnycrabby routes to /v1/identity/passport/profile/skinnycrabby
Authorization is forwarded
x-ron-passport is forwarded
x-ron-wallet-account is forwarded
x-correlation-id is forwarded
x-request-id is forwarded
Connection is not forwarded
```

### `passport_profile_upstream_errors_pass_through`

Verifies:

```text
GET /identity/passport/profile/missing preserves 404 profile_not_found
POST /identity/passport/profile/claim with requested_username=taken preserves 409 username_unavailable
```

### `wallet_balance_proxy_targets_omnigate_wallet_balance`

Verifies:

```text
GET /wallet/acct_dev/balance routes to /v1/wallet/acct_dev/balance
Authorization is forwarded
x-ron-passport is forwarded
x-ron-wallet-account is forwarded
```

### `identity_route_upstream_connect_failure_yields_problem_502`

Verifies:

```text
If SVC_GATEWAY_OMNIGATE_BASE_URL points to a dead port:
GET /identity/passport/profile/skinnycrabby returns 502
code = upstream_unavailable
retryable = true
```

---

## 13. Test dummy Omnigate behavior

The dummy Omnigate in `identity_routes_proxy.rs` handles:

```text
/v1/identity/me
/v1/identity/passport/bootstrap
/v1/identity/passport/profile/claim
/v1/identity/passport/profile/:username
/v1/wallet/:account/balance
```

It echoes:

```text
method
path
query
body_len
body
authorization
passport
wallet_account
idempotency_key
x_request_id
x_correlation_id
connection
```

Special error behavior:

```text
path contains "problem400" → 400
path contains "missing" → 404 profile_not_found
path contains "taken" → 409 username_unavailable
profile claim body requested_username="taken" → 409 username_unavailable
```

Important bug fixed:

The dummy Omnigate originally only checked path for `"taken"`, but the conflict test sent:

```json
{
  "passport_subject": "passport:main:taken2",
  "requested_username": "taken"
}
```

to:

```text
/v1/identity/passport/profile/claim
```


### END NOTE - MAY 6 2026 - 17:55 CST




### BEGIN NOTE - MAY 14 2026 - 13:50 CST

Confirmed: the RustyOnions crates changed in this backend slice were:

```text
crates/svc-gateway
crates/omnigate
```

We also changed CrabLink extension files in the separate `crablink` repo, but those are **not RustyOnions crates**. For RustyOnions crate notes, the dual note set below should go into/near the relevant crate `NOTES.MD` sections or your running carryover notes.

Your latest green gate is strong: `omnigate` clippy passed, `.post` tests passed, new `.comment` tests passed, and the full `omnigate --all-targets` suite passed. 

---

# NOTE 1 — `svc-gateway` RustyOnions crate

````markdown
### BEGIN NOTE - MAY 14 2026 - NEXT_LEVEL TEXT ASSET GATEWAY EXPOSURE

# Carryover Notes — svc-gateway NEXT_LEVEL text asset route exposure

## 0. Summary

This slice updated `crates/svc-gateway` to expose public browser-facing proxy routes for NEXT_LEVEL text assets:

```text
POST /assets/post/prepare
POST /assets/post
POST /assets/comment/prepare
POST /assets/comment
POST /assets/article/prepare
POST /assets/article
````

The important architectural outcome is that `svc-gateway` remains exactly what it should be: a public edge/proxy layer. It does **not** implement asset semantics, pricing, wallet mutation, storage writes, manifest construction, or index mutation.

The new routes forward to Omnigate:

```text
POST /v1/assets/post/prepare
POST /v1/assets/post
POST /v1/assets/comment/prepare
POST /v1/assets/comment
POST /v1/assets/article/prepare
POST /v1/assets/article
```

This was intentionally implemented before Omnigate `.comment`/`.article` semantics so CrabLink can use stable public gateway routes while backend implementation lands one primitive at a time.

## 1. Files changed

Primary file:

```text
crates/svc-gateway/src/routes/product.rs
```

Test file:

```text
crates/svc-gateway/tests/product_routes_proxy.rs
```

No `Cargo.toml` changes were required.

## 2. Product-route behavior added

`svc-gateway` now exposes the following public product route family:

```text
GET  /identity/me
POST /identity/passport/bootstrap
POST /identity/passport/profile/claim
GET  /identity/passport/profile/:username

GET  /wallet/:account/balance
POST /wallet/hold

GET  /crab/resolve?url=...
GET  /b3/:asset

POST /paid/o/prepare

POST /assets/image/prepare
POST /assets/image

POST /assets/post/prepare
POST /assets/post

POST /assets/comment/prepare
POST /assets/comment

POST /assets/article/prepare
POST /assets/article

POST /sites/prepare
POST /sites
GET  /sites/:name
```

The text asset route additions were the key part of this batch.

## 3. Header forwarding contract

The gateway proxy forwards only selected safe/product headers. Important preserved headers include:

```text
Authorization
Accept
Content-Type
Idempotency-Key
x-correlation-id
x-request-id
x-ron-*
```

It explicitly filters hop-by-hop / host-style headers such as:

```text
Host
Connection
Transfer-Encoding
Upgrade
Content-Length
Proxy-Authorization
TE
Trailer
```

This matters because CrabLink paid publish flows depend on the modern paid proof header family:

```text
x-ron-paid-op: hold
x-ron-paid-asset: roc
x-ron-paid-estimate-minor: <amount>
x-ron-wallet-txid: <txid>
x-ron-wallet-receipt-hash: <receipt_hash>
x-ron-wallet-from: <payer>
x-ron-wallet-to: <escrow>
x-ron-asset-kind: post|comment|article
```

The tests prove those headers survive the gateway hop.

## 4. Important invariants preserved

`svc-gateway` remains proxy-only.

It does **not**:

```text
- price assets
- create wallet holds
- mutate svc-wallet
- mutate ron-ledger
- store bytes
- create manifests
- write svc-index pointers
- parse post/comment/article body semantics
- fabricate CIDs or receipts
```

It only exposes stable public routes and forwards to Omnigate.

This preserves the RustyOnions value-plane boundary:

```text
CrabLink
→ svc-gateway
→ omnigate
→ svc-storage / svc-index / svc-wallet façade where appropriate
→ svc-wallet remains the economic mutation front-door
→ ron-ledger remains durable economic truth
```

## 5. Tests added/updated

Primary test file:

```text
crates/svc-gateway/tests/product_routes_proxy.rs
```

The test suite verifies:

```text
- /crab/resolve preserves query + important headers
- /b3/:asset proxies to /v1/b3/:asset
- /paid/o/prepare preserves JSON body and idempotency key
- image/site/text asset routes proxy to the correct Omnigate paths
- /assets/post, /assets/comment, /assets/article preserve paid proof headers
- /sites/:name proxies to Omnigate site resolver
- upstream connect failures become deterministic 502 Problem bodies
```

A clippy doc warning was fixed by wrapping `CrabLink` in backticks in a doc comment.

## 6. Green gates achieved

These were run and passed after the gateway batch:

```bash
cargo fmt -p svc-gateway
cargo clippy -p svc-gateway --all-targets --no-deps -- -D warnings
cargo test -p svc-gateway --test product_routes_proxy
cargo test -p svc-gateway --all-targets
```

## 7. Current status

`svc-gateway` is complete for the current NEXT_LEVEL text route exposure layer.

The public route surface is ready for:

```text
.post      implemented in Omnigate and proven end-to-end
.comment   implemented in Omnigate and proven in tests; live CrabLink publish succeeded
.article   gateway route exposed but Omnigate implementation still pending
```

## 8. Known-good proof artifacts from this session

The user restarted the local dev stack, so the current fresh proof URLs are:

```text
Post:
crab://b23f4c579201e17ab391dd3bff54635718a0b4c1371782ef87115b50f80bb1d3.post

Comment:
crab://ad0fd74aa4c20095c3a08ae9f8e111b68ccff6537ed5f8fb769fa43f782d8f63.comment
```

The `.comment` asset page successfully loaded through CrabLink and displayed:

```text
kind: comment
HTTP: 200
receipts: 1
site: crab://ron2
parent: crab://b23f4c579201e17ab391dd3bff54635718a0b4c1371782ef87115b50f80bb1d3.post
raw body rendered from b3-backed content object
```

## 9. Next svc-gateway work

No immediate svc-gateway changes are needed for `.comment`.

Future gateway work likely includes:

```text
POST /assets/article/prepare
POST /assets/article
```

Those public routes are already exposed, so next work is likely Omnigate `.article` semantics first, then only adjust gateway tests if the article route contract changes.

### END NOTE - MAY 14 2026 - NEXT_LEVEL TEXT ASSET GATEWAY EXPOSURE

````

---

# NOTE 2 — `omnigate` RustyOnions crate

```markdown
### BEGIN NOTE - MAY 14 2026 - NEXT_LEVEL POST AND COMMENT TEXT ASSETS

# Carryover Notes — omnigate NEXT_LEVEL post/comment text asset implementation

## 0. Summary

This slice implemented the first two real NEXT_LEVEL text asset primitives in `crates/omnigate`:

```text
.post
.comment
````

The implementation follows the already-proven image/site pipeline:

```text
CrabLink
→ svc-gateway public route
→ Omnigate text asset route
→ svc-storage paid write
→ storage returns b3 CID
→ Omnigate builds manifest
→ Omnigate stores manifest object
→ Omnigate writes svc-index asset→manifest pointer
→ Omnigate returns crab://<hash>.<kind>
→ CrabLink resolves typed asset page
→ CrabLink reads raw /o/b3:<hash> content object for title/body
```

The key product result: `.post` and `.comment` can now be minted as independent b3-backed assets and displayed as typed CrabLink asset pages. `.post` can also be embedded into a site using `<crab-post>`.

## 1. Files changed

Primary route module:

```text
crates/omnigate/src/routes/v1/text_assets.rs
```

Route aggregator:

```text
crates/omnigate/src/routes/v1/mod.rs
```

Tests:

```text
crates/omnigate/tests/text_asset_publish.rs
crates/omnigate/tests/comment_asset_publish.rs
```

Existing clippy cleanup was also needed in:

```text
crates/omnigate/tests/passport_profile_routes.rs
```

No `Cargo.toml` changes were required.

## 2. Route aggregator changes

`crates/omnigate/src/routes/v1/mod.rs` now includes:

```rust
pub mod text_assets;
```

and mounts text assets under:

```text
/v1/assets
```

The active routes are:

```text
POST /v1/assets/post/prepare
POST /v1/assets/post
POST /v1/assets/comment/prepare
POST /v1/assets/comment
```

Article is intentionally not active yet in Omnigate, even though gateway public route exposure already exists.

## 3. New text asset route module

`crates/omnigate/src/routes/v1/text_assets.rs` now owns the post/comment text asset flow.

### Active asset kinds

The module introduced a staged internal enum:

```text
TextAssetKind::Post
TextAssetKind::Comment
```

Article was intentionally left for a later batch to preserve the implementation order:

```text
post → comment → article
```

## 4. Post route contract

### Prepare

```text
POST /v1/assets/post/prepare
```

Accepted strict JSON fields include:

```text
title
body
site_context_crab_url
parent_crab_url
creator_display
language
post_kind / postKind
visibility
rights_mode / rightsMode
moderation_mode / moderationMode
content_warning / contentWarning
tags
payer_account / payerAccount
owner_passport_subject / ownerPassportSubject
client_idempotency_key / clientIdempotencyKey
```

Important rules:

```text
- title required for post
- body required
- site_context_crab_url required
- site_context_crab_url must be a named crab:// site URL
- parent_crab_url optional for post
- payer_account required
- owner_passport_subject required
```

### Publish

```text
POST /v1/assets/post
```

Requires modern paid proof headers:

```text
x-ron-paid-op
x-ron-paid-asset
x-ron-paid-estimate-minor
x-ron-wallet-txid
x-ron-wallet-receipt-hash
x-ron-wallet-from
x-ron-wallet-to
x-ron-asset-kind: post
```

Publishes a canonical content envelope:

```text
schema: ron.post-content.v1
kind: post
asset_kind: post
format: text/plain; charset=utf-8
title
body
metadata
relations
site_connection
parent_reference
thread_reference
creator_display
created_at_ms
```

## 5. Comment route contract

### Prepare

```text
POST /v1/assets/comment/prepare
```

Accepted strict JSON fields include:

```text
title
body
site_context_crab_url
parent_crab_url / parentCrabUrl / target_crab_url / targetCrabUrl
thread_context_crab_url / threadContextCrabUrl
creator_display / creatorDisplay
language
comment_kind / commentKind
visibility
rights_mode / rightsMode
moderation_mode / moderationMode
content_warning / contentWarning
tags
payer_account / payerAccount
owner_passport_subject / ownerPassportSubject
client_idempotency_key / clientIdempotencyKey
```

Important rules:

```text
- body required
- site_context_crab_url required
- parent_crab_url required
- payer_account required
- owner_passport_subject required
- title optional; defaults to "Comment"
- site_context_crab_url must be a named crab:// site URL
- parent_crab_url must be a typed crab://<hash>.<kind> asset URL
```

### Parent/target relationship

For comments, `parent_crab_url` is required. The first live test used:

```text
parent_crab_url:
crab://b23f4c579201e17ab391dd3bff54635718a0b4c1371782ef87115b50f80bb1d3.post
```

Allowed parent kind list currently includes:

```text
post
comment
article
image
video
music
song
podcast
stream
thread
film
```

This was intentionally made future-ready so comments can eventually attach to posts, articles, images, videos, podcasts, streams, etc. However, only `.post` and `.comment` have proven backend publish support right now.

### Publish

```text
POST /v1/assets/comment
```

Requires modern paid proof headers:

```text
x-ron-paid-op
x-ron-paid-asset
x-ron-paid-estimate-minor
x-ron-wallet-txid
x-ron-wallet-receipt-hash
x-ron-wallet-from
x-ron-wallet-to
x-ron-asset-kind: comment
```

Publishes a canonical content envelope:

```text
schema: ron.comment-content.v1
kind: comment
asset_kind: comment
format: text/plain; charset=utf-8
title: Comment by default
body
metadata.comment_kind
metadata.language
metadata.tags
relations.site
relations.parent
relations.target
site_connection
parent_reference
thread_reference
creator_display
created_at_ms
```

## 6. Storage/index coordination

For both `.post` and `.comment`, Omnigate coordinates the same flow:

```text
1. validate strict JSON
2. build canonical JSON content envelope
3. call svc-storage GET /paid/o/estimate?bytes=<canonical_bytes>
4. return wallet hold template to the client
5. require paid proof headers on publish
6. call svc-storage POST /paid/o with content envelope bytes
7. parse returned content CID
8. require canonical b3:<64 lowercase hex>
9. build manifest JSON
10. store manifest object through svc-storage POST /o
11. parse returned manifest CID
12. write svc-index pointer:
    PUT /v1/index/assets/<hash>/manifest
13. return crab://<hash>.<kind> plus links/manifest/index summary
```

## 7. Invariants preserved

Omnigate still does **not** mutate wallet or ledger directly.

The implementation does **not**:

```text
- call ron-ledger
- directly mutate wallet balances
- fake receipts
- fabricate CIDs
- silently charge ROC
- write storage without paid proof on publish
- store bytes outside svc-storage
- write index state anywhere except svc-index pointer route
```

The publish path requires the caller to supply the wallet hold proof created through the wallet path.

The economics invariant remains:

```text
svc-wallet is the economic mutation front-door.
ron-ledger is the durable economic truth.
omnigate coordinates product flow but does not own monetary truth.
```

## 8. Error taxonomy and strict DTO behavior

The route uses `#[serde(deny_unknown_fields)]` on the text asset request DTO. This means frontend clients must not send local manifest/debug fields.

Earlier frontend clients were rejected until they were updated to avoid fields like:

```text
schema
asset_kind
bytes
content_type
relations
content
manifest_hint
```

Important problem codes/reasons include:

```text
invalid_post_prepare_request
invalid_post_publish_request
invalid_comment_prepare_request
invalid_comment_publish_request
bad_json
missing_site_context
invalid_site_context
missing_parent_reference
invalid_parent_reference
invalid_parent_kind
missing_payer_account
missing_owner_passport_subject
missing_paid_proof
invalid_paid_proof
storage_estimate_missing_amount
storage_upload_missing_cid
storage_upload_invalid_cid
manifest_encode_failed
index_pointer_encode_failed
```

Clippy previously flagged `Result<_, Response>` as `result_large_err`, so local validation helpers now return the smaller `RouteProblemSpec` and only convert to Axum `Response` at the handler boundary.

## 9. Tests added/updated

### Existing post tests

```text
crates/omnigate/tests/text_asset_publish.rs
```

This test suite verifies:

```text
post_asset_prepare_returns_hold_template
post_asset_prepare_requires_site_context
post_asset_publish_coordinates_paid_storage_manifest_and_index_pointer
post_asset_publish_requires_paid_proof_headers
post_asset_prepare_upstream_connect_failure_yields_problem_502
```

The post test had to remain compatible with `post_kind`, so `TextAssetRequest` now accepts both:

```text
post_kind
postKind
```

### New comment tests

```text
crates/omnigate/tests/comment_asset_publish.rs
```

This test suite verifies:

```text
comment_asset_prepare_returns_hold_template
comment_asset_prepare_requires_parent_reference
comment_asset_publish_coordinates_paid_storage_manifest_and_index_pointer
comment_asset_publish_requires_paid_proof_headers
```

The tests spin up dummy storage and index services and then run real Omnigate routes in-process.

They prove:

```text
- strict JSON validation
- comment body envelope creation
- site relation required
- parent post relation required
- paid proof headers required
- storage paid write gets correct content bytes
- manifest object is written
- index pointer is written
- returned crab://<hash>.comment URL is correct
```

## 10. Clippy cleanup

During this slice, clippy issues fixed included:

```text
- result_large_err in text_assets.rs helper functions
- dead_code from unused TextAssetKind::prepare_route
- unnecessary_filter_map in passport_profile_routes.rs
```

## 11. Green gates achieved

The final green gate from this slice:

```bash
cargo fmt -p omnigate
cargo clippy -p omnigate --all-targets --no-deps -- -D warnings
cargo test -p omnigate --test text_asset_publish
cargo test -p omnigate --test comment_asset_publish
cargo test -p omnigate --all-targets
```

All passed.

The full `omnigate --all-targets` run included and passed:

```text
text_asset_publish.rs
comment_asset_publish.rs
image_asset_prepare.rs
asset_page_resolver.rs
site_launch.rs
named_site_crab_resolver.rs
passport_profile_routes.rs
paid_storage_prepare.rs
paid_storage_write_proxy.rs
paid_storage_estimate_proxy.rs
readiness tests
middleware tests
policy tests
zk tests
benches smoke target
```

## 12. Live CrabLink proof from this session

After restarting the stack, the user minted a fresh post:

```text
crab://b23f4c579201e17ab391dd3bff54635718a0b4c1371782ef87115b50f80bb1d3.post
```

Then the user minted a fresh comment:

```text
crab://ad0fd74aa4c20095c3a08ae9f8e111b68ccff6537ed5f8fb769fa43f782d8f63.comment
```

The comment asset page showed:

```text
Comment Asset
HTTP 200
receipts: 1
kind: comment
content schema: ron.comment-content.v1
content kind: comment
language: en
site: crab://ron2
parent: crab://b23f4c579201e17ab391dd3bff54635718a0b4c1371782ef87115b50f80bb1d3.post
title: no title / default title behavior visible in UI
body: test comment test test test comment test
```

This proves the `.comment` object is independently b3-backed and can preserve both site context and parent post reference.

## 13. Known issues / follow-up notes

### 13.1 Comment default title display

The current live comment page shows “no title” in some UI spots even though the backend content envelope defaults comment title to `"Comment"` if no title is supplied.

This may be a frontend display/normalization issue, not necessarily a backend issue. Check CrabLink’s typed asset viewer and comment content summary normalization before changing backend.

### 13.2 Parent type policy is permissive

The backend currently permits future parent kinds such as:

```text
article
video
music
podcast
stream
film
```

That is future-ready, but product UX should still label unsupported parent types honestly until those assets are backend-proven.

### 13.3 Article still pending

Gateway route exposure exists for:

```text
POST /assets/article/prepare
POST /assets/article
```

But Omnigate `.article` implementation is not done yet. The next backend primitive should probably be `.article`.

## 14. Recommended next backend step

Implement Omnigate `.article` using the generalized text asset pattern, but with article-specific rules:

```text
title required
body required
site_context_crab_url required
optional excerpt/summary
optional hero image reference
optional linked assets
optional parent/thread relation depending on use case
payer_account required
owner_passport_subject required
paid proof required on publish
```

Recommended new test:

```text
crates/omnigate/tests/article_asset_publish.rs
```

Recommended route additions in existing text module:

```text
POST /v1/assets/article/prepare
POST /v1/assets/article
```

## 15. Recommended next CrabLink step

Before article backend, the UI path may need one of these:

```text
- add <crab-comment> embed hydration, like <crab-post>
- polish comment typed asset page label from “Post content truth” to “Comment content truth”
- make comment title/default display cleaner
```

But backend-wise, `.post` and `.comment` are now proven.

### END NOTE - MAY 14 2026 - NEXT_LEVEL POST AND COMMENT TEXT ASSETS

````

---

# Short carryover checkpoint

```markdown
## RustyOnions / CrabLink checkpoint — 2026-05-14

Backend crates changed:
- crates/svc-gateway
- crates/omnigate

CrabLink repo also changed separately:
- post client strict DTO
- comment client strict DTO
- typed text asset viewer
- <crab-post> site embed hydration

Fresh known-good proof URLs from current stack:
- Post: crab://b23f4c579201e17ab391dd3bff54635718a0b4c1371782ef87115b50f80bb1d3.post
- Comment: crab://ad0fd74aa4c20095c3a08ae9f8e111b68ccff6537ed5f8fb769fa43f782d8f63.comment

Green gates:
- svc-gateway clippy/tests passed after route exposure batch
- omnigate clippy passed
- omnigate text_asset_publish passed
- omnigate comment_asset_publish passed
- omnigate --all-targets passed

Current product proof:
- .post can mint, resolve, display raw title/body, and embed into a site with <crab-post>
- .comment can mint, resolve, display raw body, preserve site context, and preserve parent post reference

Next likely work:
1. Add <crab-comment> embed hydration in CrabLink.
2. Polish comment typed asset wording/default title display.
3. Implement Omnigate .article prepare/publish.
4. Then align CrabLink article client strict JSON and typed article display.
````


### END NOTE - MAY 14 2026 - 13:50 CST




### BEGIN NOTE - MAY 15 2026 - 13:30 CST


# CARRY-OVER NOTES — svc-gateway + omnigate paid `site_visit` proof

**Date:** 2026-05-15  
**Crates documented:** `svc-gateway`, `omnigate`  
**Milestone:** First paid named-site visit loop proven end-to-end.  
**Status:** SUCCESS — `crab://ron7` paid `site_visit` quote/pay works through gateway → omnigate → svc-wallet → ron-ledger, and CrabLink/browser confirms real ROC balance movement.

---

## 0. TL;DR

We successfully implemented and proved the first real **paid named-site visit** path:

```text
Visitor B / acct_visitor_b
→ visits crab://ron7
→ pays 10 ROC for site_visit
→ Creator A / acct_dev receives 10 ROC
→ svc-wallet returns a transfer receipt
→ ron-ledger commits durable receipt/root truth
→ CrabLink/browser shows real backend balance changes
````

This proves that a named CrabLink site can now be monetized by real internal ROC movement, not fake UI state.

The important route pair is now alive:

```text
POST /sites/:name/visit/quote
POST /sites/:name/visit/pay
```

Public route flow:

```text
CrabLink / curl
→ svc-gateway public route
→ omnigate /v1 route
→ site manifest/index/storage lookup
→ payout recipient validation
→ svc-wallet /v1/transfer
→ ron-ledger receipt/root
→ response back to CrabLink
```

This is a major WEB3/NEXT_LEVEL milestone because it proves:

```text
Visitor spend:       real
Creator receipt:     real
Wallet receipt:      real
Ledger root:         real
CrabLink display:    backend truth
Gateway role:        proxy-only
Omnigate role:       coordinator/hydrator
svc-wallet role:     mutation front-door
ron-ledger role:     durable truth
```

---

## 1. Files changed / touched

### omnigate

Primary files:

```text
crates/omnigate/src/routes/v1/site_visit.rs
crates/omnigate/src/routes/v1/mod.rs
```

What changed:

```text
- Added paid site_visit quote route.
- Added paid site_visit pay route.
- Mounted:
  POST /v1/sites/:name/visit/quote
  POST /v1/sites/:name/visit/pay
- Quote route resolves named site, loads site manifest, validates payout recipient, and returns fixed dev quote.
- Pay route resolves named site, validates payout recipient, rejects self-payment, and sends svc-wallet transfer.
- Pay route returns omnigate.site-visit-payment.v1 with wallet receipt fields.
```

Important implementation shape:

```text
quote:
  route site name
  → svc-index site pointer
  → svc-storage manifest object
  → validate manifest payout.default_action == site_visit
  → validate recipient_account
  → return amount_minor = "10"

pay:
  route site name
  → load same site context
  → validate payer != recipient
  → validate amount_minor == configured site_visit price
  → POST svc-wallet /v1/transfer
  → return wallet receipt + site_visit receipt wrapper
```

Important env/defaults:

```text
OMNIGATE_SITE_VISIT_PRICE_MINOR default = 10
OMNIGATE_WALLET_BASE_URL default = http://127.0.0.1:8088
OMNIGATE_WALLET_BEARER default = dev
OMNIGATE_STORAGE_BASE_URL / OMNIGATE_DOWNSTREAM_STORAGE_BASE_URL
OMNIGATE_INDEX_BASE_URL / OMNIGATE_DOWNSTREAM_INDEX_BASE_URL
```

### svc-gateway

Primary file:

```text
crates/svc-gateway/src/routes/product.rs
```

What changed:

```text
- Added public product proxy routes:
  POST /sites/:name/visit/quote
  POST /sites/:name/visit/pay

- Proxies to omnigate:
  POST /v1/sites/:name/visit/quote
  POST /v1/sites/:name/visit/pay
```

Important gateway invariant preserved:

```text
svc-gateway does not price.
svc-gateway does not parse manifests.
svc-gateway does not mutate wallet.
svc-gateway does not mutate ledger.
svc-gateway only proxies selected headers/body to omnigate.
```

---

## 2. Bugs encountered and fixed

### Bug 1 — 404 route missing

Initial curl:

```text
POST /sites/ron7/visit/quote
→ HTTP/1.1 404 Not Found
```

Meaning:

```text
svc-gateway did not yet expose the public route.
```

Fix:

```text
Added /sites/:name/visit/quote and /sites/:name/visit/pay to svc-gateway product router.
Mounted matching /v1 routes in omnigate.
```

### Bug 2 — `site_manifest_bad_json`

After route registration, quote/pay reached Omnigate but failed with:

```json
{
  "code": "site_manifest_bad_json",
  "message": "site manifest object was not valid JSON",
  "retryable": true,
  "reason": "site_manifest_bad_json"
}
```

Root cause:

```text
site_visit.rs had a strict duplicate SiteManifestDocument DTO that did not accept all fields generated by the site creation route.
```

Real site manifests include fields such as:

```text
asset_map
route_map
owner
payout
metadata
provenance
storage
receipts
```

Fix:

```text
Updated the local SiteManifestDocument in site_visit.rs to accept the generated site manifest shape.
```

Follow-up note:

```text
Long-term, avoid maintaining duplicate strict manifest parsers in multiple route files.
Prefer a shared DTO/helper or parse as serde_json::Value and extract only the fields required for site_visit validation.
```

### Bug 3 — running stack used old binary

After patching, `cargo test` and `cargo clippy` were green, but live curl still returned the old error until the stack was restarted.

Lesson:

```text
cargo test/clippy compile patched code, but do not restart the already-running omnigate/svc-gateway processes.
After route changes, restart the RustyOnions dev stack before live curl/browser tests.
```

### Dev stack state note

The local dev stack may flush prior site/index/storage state on restart depending on artifact directory/config.

Practical testing rule:

```text
After stack restart, check whether the named site still exists:
curl -i -sS http://127.0.0.1:8090/sites/ron7

If missing, recreate the site in CrabLink with Creator A / acct_dev.
```

---

## 3. Tests / gates run

The following commands were run and passed during this batch:

```text
cargo fmt -p omnigate
cargo fmt -p svc-gateway
cargo test -p omnigate --test site_launch
cargo test -p svc-gateway --test product_routes_proxy
cargo clippy -p omnigate --all-targets --no-deps -- -D warnings
cargo clippy -p svc-gateway --all-targets --no-deps -- -D warnings
```

Observed test results:

```text
omnigate site_launch:
  5 passed / 0 failed

svc-gateway product_routes_proxy:
  7 passed / 0 failed

omnigate clippy:
  clean

svc-gateway clippy:
  clean
```

Important note:

```text
Existing gateway product route tests did not yet explicitly verify the new site_visit quote/pay proxy routes.
Add that as follow-up test coverage.
```

---

## 4. Live route health proof

Health checks before proof:

```text
curl -sS http://127.0.0.1:8090/healthz
→ ok

curl -sS http://127.0.0.1:9090/healthz
→ {"config":true,"omnigate":true}
```

This confirmed gateway and omnigate were live.

---

## 5. Site resolver proof

`crab://ron7` resolved as a valid paid site.

Important resolver fields:

```text
schema = omnigate.site-page.v1
site_name = ron7
root_document_cid = b3:056cd088f6bb9ae45b11b1cd4786a2eb9b62f576e35f7132b33a74e4f07dac96
manifest_cid = b3:918dad927484a43c2f716691794e24e7a608c951f38b9a6341a39f558284a4a9
owner.passport_subject = passport:main:dev
owner.wallet_account = acct_dev
payout.default_action = site_visit
payout.recipient_account = acct_dev
metadata.title = Reference Graph Smoke Site
```

This confirmed the site was correctly configured for the proof:

```text
Creator A owns the site.
Creator A payout recipient is acct_dev.
Visitor B should pay acct_dev for site_visit.
```

---

## 6. Quote proof

Command shape:

```text
POST http://127.0.0.1:8090/sites/ron7/visit/quote

Headers:
  Authorization: Bearer dev
  Content-Type: application/json
  x-ron-passport: passport:main:visitor-b
  x-ron-wallet-account: acct_visitor_b
  Idempotency-Key: crablink-test-site-visit-quote-ron7d

Body:
  site_name = ron7
  crab_url = crab://ron7
  action = site_visit
  quantity = 1
  payer_account = acct_visitor_b
  visitor_wallet_account = acct_visitor_b
  visitor_passport_subject = passport:main:visitor-b
  recipient_account = acct_dev
  max_amount_minor = 10
```

Result:

```text
HTTP/1.1 200 OK
schema = omnigate.site-visit-quote.v1
ok = true
action = site_visit
amount_minor = "10"
asset = "roc"
display_amount = "10 ROC"
payer_account = "acct_visitor_b"
visitor_wallet_account = "acct_visitor_b"
visitor_passport_subject = "passport:main:visitor-b"
recipient_account = "acct_dev"
site_name = "ron7"
crab_url = "crab://ron7"
quote_id = "site-visit-b19f93aa1d951d55"
quote_hash = "85e1c00714793cc3"
```

Nested quote policy confirmed:

```text
policy.source = omnigate.site_visit.v1
policy.fixed_dev_price_minor = 10
policy.price_env = OMNIGATE_SITE_VISIT_PRICE_MINOR
policy.wallet_front_door = svc-wallet /v1/transfer
```

This proves:

```text
svc-gateway public route works.
omnigate route works.
site manifest lookup works.
payout recipient lookup works.
fixed 10 ROC dev pricing works.
quote DTO is CrabLink-compatible.
```

---

## 7. Pay proof

Command shape:

```text
POST http://127.0.0.1:8090/sites/ron7/visit/pay

Headers:
  Authorization: Bearer dev
  Content-Type: application/json
  x-ron-passport: passport:main:visitor-b
  x-ron-wallet-account: acct_visitor_b
  Idempotency-Key: crablink-test-site-visit-pay-ron7d

Body:
  site_name = ron7
  crab_url = crab://ron7
  action = site_visit
  payer_account = acct_visitor_b
  visitor_wallet_account = acct_visitor_b
  visitor_passport_subject = passport:main:visitor-b
  recipient_account = acct_dev
  amount_minor = 10
  asset = roc
  client_idempotency_key = crablink-test-site-visit-pay-ron7d
```

Result:

```text
HTTP/1.1 200 OK
schema = omnigate.site-visit-payment.v1
ok = true
action = site_visit
site_name = ron7
crab_url = crab://ron7
amount_minor = "10"
asset = "roc"
payer_account = "acct_visitor_b"
visitor_wallet_account = "acct_visitor_b"
recipient_account = "acct_dev"
nonce = 1
txid = tx_6f04012c31be95d392824308c440a6d2
receipt_hash = b3:c02be0ecd068408ca18d7a5dd7bad7bf987b05ed0c3b6b15a222880f263aec8a
```

Wallet receipt:

```text
op = transfer
from = acct_visitor_b
to = acct_dev
asset = roc
amount_minor = 10
nonce = 1
idem = crablink-test-site-visit-pay-ron7d
ledger_seq_start = 5
ledger_seq_end = 6
ledger_root = 3062a104f536661a279358cecb016d641ea77dff5990bd6a1dc4f2c762b36726
txid = tx_6f04012c31be95d392824308c440a6d2
receipt_hash = b3:c02be0ecd068408ca18d7a5dd7bad7bf987b05ed0c3b6b15a222880f263aec8a
```

Site visit receipt wrapper included:

```text
kind = site_visit
manifest_cid = b3:918dad927484a43c2f716691794e24e7a608c951f38b9a6341a39f558284a4a9
root_document_cid = b3:056cd088f6bb9ae45b11b1cd4786a2eb9b62f576e35f7132b33a74e4f07dac96
wallet_txid = tx_6f04012c31be95d392824308c440a6d2
wallet_receipt_hash = b3:c02be0ecd068408ca18d7a5dd7bad7bf987b05ed0c3b6b15a222880f263aec8a
```

This proves:

```text
Omnigate correctly called svc-wallet /v1/transfer.
svc-wallet performed the transfer.
ron-ledger committed durable receipt/root truth.
The response contains enough proof for CrabLink to unlock paid content.
```

---

## 8. Browser / CrabLink proof

Browser confirmation:

```text
Passport B / Visitor B lost 10 ROC.
Passport A / Creator A received 10 ROC.
```

This is the first confirmed real browser-observed paid named-site visit transfer.

This matters because CrabLink is not editing balances locally. The browser balance movement came from backend wallet truth.

---

## 9. Architectural invariants preserved

### svc-gateway invariants

Preserved:

```text
- Gateway is public route surface only.
- Gateway proxies quote/pay to Omnigate.
- Gateway does not inspect site manifests.
- Gateway does not calculate price.
- Gateway does not decide payout recipient.
- Gateway does not call svc-wallet directly for this product flow.
- Gateway does not mutate ron-ledger.
- Gateway preserves selected x-ron-* and idempotency headers.
```

### omnigate invariants

Preserved:

```text
- Omnigate coordinates product flow.
- Omnigate resolves site context through index/storage.
- Omnigate validates payout recipient from site manifest.
- Omnigate does not directly mutate ron-ledger.
- Omnigate sends mutation through svc-wallet.
- Quote route is read-only.
- Pay route is explicit and receipt-returning.
```

### wallet/ledger invariants

Preserved:

```text
- svc-wallet remains the economic mutation front-door.
- ron-ledger remains durable economic truth.
- Transfer uses integer minor units only.
- Receipt includes nonce, idempotency, txid, receipt_hash, ledger root.
- No fake balances.
- No fake receipts.
```

### CrabLink invariants

Preserved:

```text
- CrabLink calls public gateway routes.
- CrabLink does not call internal services directly.
- CrabLink does not mutate balances locally.
- CrabLink only unlocks paid content after backend proof.
- User-visible payment requires explicit action.
```

---

## 10. Important current limitations / beta caveats

This is a successful proof, but not production final.

Known limitations:

```text
- Site visit price is currently fixed/dev-configured via OMNIGATE_SITE_VISIT_PRICE_MINOR.
- Pricing should later move into ron-policy / configs/roc-economics.toml.
- Quote hash is simple/dev-style and should later become canonical signed/hashed quote material.
- Pay route currently revalidates amount against current config, not a persisted quote object.
- Nonce handling is early/dev; wallet expected nonce recovery may need hardening.
- Site manifest parsing is duplicated locally in site_visit.rs.
- Dev stack may flush site/index/storage state between restarts.
- No dedicated site_visit test file yet.
- Gateway product proxy tests should explicitly cover quote/pay routes.
- No full smoke script yet for quote → pay → balance delta.
```

Do not treat this as final chain/economy architecture. It is the first proof of the product loop.

---

## 11. Recommended immediate follow-up

### A. Add reproducible smoke script

Create a smoke script that proves:

```text
1. Create or require site crab://ron7 or generated smoke name.
2. Confirm site resolver returns payout.default_action = site_visit.
3. Check acct_visitor_b balance before.
4. Check acct_dev balance before.
5. POST /sites/:name/visit/quote.
6. Assert amount_minor == 10.
7. POST /sites/:name/visit/pay.
8. Assert schema == omnigate.site-visit-payment.v1.
9. Assert wallet_receipt.op == transfer.
10. Assert wallet_receipt.from == acct_visitor_b.
11. Assert wallet_receipt.to == acct_dev.
12. Assert wallet_receipt.amount_minor == 10.
13. Check acct_visitor_b balance after decreased by 10.
14. Check acct_dev balance after increased by 10.
```

Suggested script name:

```text
scripts/web3_paid_site_visit_smoke.sh
```

### B. Add omnigate tests

Suggested tests:

```text
crates/omnigate/tests/site_visit.rs
```

Test cases:

```text
site_visit_quote_returns_10_roc_for_paid_site
site_visit_quote_rejects_recipient_mismatch
site_visit_quote_rejects_missing_payer
site_visit_pay_rejects_self_payment
site_visit_pay_uses_wallet_transfer
site_visit_pay_returns_wallet_receipt
site_visit_pay_rejects_amount_mismatch
```

### C. Add svc-gateway proxy tests

Extend:

```text
crates/svc-gateway/tests/product_routes_proxy.rs
```

Add assertions:

```text
POST /sites/ron7/visit/quote → /v1/sites/ron7/visit/quote
POST /sites/ron7/visit/pay → /v1/sites/ron7/visit/pay
headers/body/idempotency preserved
```

### D. Update route contracts docs

Update:

```text
docs/WEB3_2_PRODUCT_ROUTE_CONTRACTS.md
```

Add:

```text
POST /sites/:name/visit/quote
POST /sites/:name/visit/pay
```

Document proof DTOs:

```text
omnigate.site-visit-quote.v1
omnigate.site-visit-payment.v1
```

### E. Improve manifest context loading

Longer-term refactor:

```text
- Avoid duplicate strict manifest DTO in site_visit.rs.
- Share site manifest loading/hydration helper with sites.rs.
- Or parse manifest as Value and extract only:
  version
  site_name
  root_document_cid
  owner.wallet_account
  owner.passport_subject
  payout.default_action
  payout.recipient_account
  metadata.title
```

### F. Move pricing to policy/economics config

Current proof uses:

```text
OMNIGATE_SITE_VISIT_PRICE_MINOR = 10
```

Future target:

```text
ron-policy / configs/roc-economics.toml
action = site_visit
amount_minor = 10 dev default
basis-point splits
creator/storage/provider/treasury split support
```

---

## 12. Manual reproduction commands

Health:

```bash
curl -sS http://127.0.0.1:8090/healthz
curl -sS http://127.0.0.1:9090/healthz
```

Check site:

```bash
curl -i -sS "http://127.0.0.1:8090/sites/ron7"
```

Quote:

```bash
curl -i -sS \
  -X POST "http://127.0.0.1:8090/sites/ron7/visit/quote" \
  -H "Authorization: Bearer dev" \
  -H "Content-Type: application/json" \
  -H "x-ron-passport: passport:main:visitor-b" \
  -H "x-ron-wallet-account: acct_visitor_b" \
  -H "Idempotency-Key: crablink-test-site-visit-quote-ron7d" \
  --data '{"site_name":"ron7","crab_url":"crab://ron7","action":"site_visit","quantity":1,"payer_account":"acct_visitor_b","visitor_wallet_account":"acct_visitor_b","visitor_passport_subject":"passport:main:visitor-b","recipient_account":"acct_dev","max_amount_minor":"10","client_idempotency_key":"crablink-test-site-visit-quote-ron7d"}'
```

Pay:

```bash
curl -i -sS \
  -X POST "http://127.0.0.1:8090/sites/ron7/visit/pay" \
  -H "Authorization: Bearer dev" \
  -H "Content-Type: application/json" \
  -H "x-ron-passport: passport:main:visitor-b" \
  -H "x-ron-wallet-account: acct_visitor_b" \
  -H "Idempotency-Key: crablink-test-site-visit-pay-ron7d" \
  --data '{"site_name":"ron7","crab_url":"crab://ron7","action":"site_visit","payer_account":"acct_visitor_b","visitor_wallet_account":"acct_visitor_b","visitor_passport_subject":"passport:main:visitor-b","recipient_account":"acct_dev","amount_minor":"10","asset":"roc","client_idempotency_key":"crablink-test-site-visit-pay-ron7d"}'
```

Balance check:

```bash
curl -sS "http://127.0.0.1:8090/wallet/acct_visitor_b/balance" | python3 -m json.tool
curl -sS "http://127.0.0.1:8090/wallet/acct_dev/balance" | python3 -m json.tool
```

Expected balance effect:

```text
acct_visitor_b decreases by 10 ROC
acct_dev increases by 10 ROC
```

---

## 13. Why this milestone matters

This is the first confirmed paid named-site consumption proof.

Before this, RustyOnions had strong proofs for:

```text
paid image upload
site creation
site resolution
reference-graph site rendering
post/comment/article/image embeds
wallet-backed balances
```

This milestone adds:

```text
paid site visit
visitor-to-creator transfer
creator payout proof
browser-observed ROC deduction/credit
```

This moves RustyOnions/CrabLink closer to a real creator economy:

```text
create site
publish assets
attach payout policy
visitor pays to access/use/view
creator receives ROC
receipt proves it
ledger replays it
```

---

## 14. Do not regress

Future changes must not break:

```text
POST /sites/:name/visit/quote
POST /sites/:name/visit/pay
quote amount 10 ROC in dev config
payer acct_visitor_b
recipient acct_dev
wallet receipt op=transfer
CrabLink unlock after backend proof
gateway proxy-only boundary
wallet as mutation front-door
ledger as durable truth
```

Add tests/smoke coverage before refactoring this path.

---

## 15. Completion estimate impact

This milestone raises confidence in the internal WEB3/NEXT_LEVEL beta loop.

Updated rough interpretation:

```text
RustyOnions internal ROC / WEB3 beta loop:
  now closer to high-80s percent complete for local beta proof

CrabLink + RustyOnions combined paid creator proof:
  significantly advanced because site visits now move real ROC

Remaining before calling this robust:
  automated smoke
  tests
  policy/economics integration
  receipt UI polish
  paid content view generalization
  accounting/rewarder integration
```

---

## 16. Final checkpoint phrase

```text
2026-05-15:
Paid named-site visit is proven.

crab://ron7 quoted 10 ROC for site_visit.
acct_visitor_b paid acct_dev.
svc-wallet returned transfer receipt.
ron-ledger returned ledger root.
CrabLink/browser confirmed Passport B lost 10 ROC and Passport A received 10 ROC.

This is the first real visitor-to-creator ROC payment loop.
```



### END NOTE - MAY 15 2026 - 13:30 CST




### BEGIN NOTE - MAY 16 2026 - 14:40 CST

Below are two separate crate-specific carryover notes you can paste into your running notes. They are focused only on the changes made in **`omnigate`** and **`svc-gateway`** during this backend/content-view session.

---

# OMNIGATE Carryover Notes — Paid Site Visit + Paid Content View

## 1. Crate role reminder

`omnigate` remains a **product coordinator / façade**. It must not become economic truth and must not mutate `ron-ledger` directly.

Correct role:

```text
CrabLink / svc-gateway
→ omnigate product route
→ svc-index / svc-storage / svc-wallet
→ wallet receipt / ledger root returned
```

`omnigate` may coordinate paid product actions, but actual money movement must go through:

```text
svc-wallet
```

and durable economic truth must remain in:

```text
ron-ledger
```

Hard invariant preserved this session:

```text
No direct ledger mutation from omnigate.
No fake receipts.
No silent spend.
No floating-point money math.
```

---

# 2. Major Omnigate work completed this session

## 2.1 Paid named-site visit backend was locked

We already had the site-visit product path available, but this session turned it into a repeatable proof with tests and live smoke.

Relevant route family:

```text
POST /v1/sites/:name/visit/quote
POST /v1/sites/:name/visit/pay
```

Public gateway equivalent:

```text
POST /sites/:name/visit/quote
POST /sites/:name/visit/pay
```

Behavior now proven:

```text
Visitor B / acct_visitor_b
→ pays 10 ROC
→ Creator A / acct_dev receives 10 ROC
→ svc-wallet returns transfer receipt
→ response includes txid / receipt_hash / ledger_root
```

Latest repeated live runs showed `acct_visitor_b` decreasing by 10 and `acct_dev` increasing by 10, with committed txid, receipt hash, and ledger root. This proved the route is not just mocked UI behavior. The live preflight logs also showed the site-visit smoke passing while the stack stayed up. 

## 2.2 Omnigate site-visit contract tests added

File added:

```text
crates/omnigate/tests/site_visit.rs
```

Purpose:

```text
Verify omnigate site_visit quote/pay behavior without requiring the live stack.
```

The test creates dummy upstreams for:

```text
svc-index
svc-storage
svc-wallet
```

Covered cases:

```text
quote_returns_10_roc_and_manifest_payout_recipient
pay_recovers_wallet_nonce_and_returns_wallet_receipt
quote_rejects_recipient_mismatch
```

Important behavior locked:

```text
quote:
  read-only
  loads site manifest pointer
  loads site manifest
  derives payout recipient from manifest
  returns fixed dev price
  does not call wallet

pay:
  validates recipient
  validates payer
  calls svc-wallet /v1/transfer
  handles nonce conflict by retrying expected nonce
  returns wallet receipt
```

The nonce recovery behavior is important because earlier live site creation/paid flows sometimes exposed expected nonce errors. The test ensures Omnigate can recover when wallet says something like:

```text
nonce conflict: expected 8
```

## 2.3 Omnigate content-view backend route added

File added:

```text
crates/omnigate/src/routes/v1/content_view.rs
```

Purpose:

```text
Add backend quote/pay support for paid b3 asset views.
```

New route family:

```text
POST /v1/content/view/quote
POST /v1/content/view/pay
```

This route family is for b3-backed asset pages such as:

```text
crab://<64hex>.article
crab://<64hex>.post
crab://<64hex>.comment
crab://<64hex>.image
crab://<64hex>.video
crab://<64hex>.song
crab://<64hex>.music
```

Primary route flow:

```text
content_view quote/pay
→ parse canonical crab://<hash>.<kind>
→ load asset pointer from svc-index
→ load manifest object from svc-storage
→ validate asset_cid and asset_kind
→ derive payout recipient from manifest payout/owner
→ quote or pay through svc-wallet
```

## 2.4 Omnigate v1 router updated

File modified:

```text
crates/omnigate/src/routes/v1/mod.rs
```

Change:

```rust
pub mod content_view;
```

and route mount:

```rust
.nest("/content", content_view::router())
```

This exposes:

```text
/v1/content/view/quote
/v1/content/view/pay
```

## 2.5 Omnigate content-view tests added

File added:

```text
crates/omnigate/tests/content_view.rs
```

Purpose:

```text
Test paid b3 asset content_view quote/pay contract with dummy upstreams.
```

Dummy upstreams:

```text
dummy svc-index:
  GET /v1/index/assets/:hash/manifest

dummy svc-storage:
  GET /o/:manifest_cid

dummy svc-wallet:
  POST /v1/transfer
```

Covered tests:

```text
quote_returns_manifest_recipient_and_does_not_call_wallet
pay_recovers_wallet_nonce_and_returns_wallet_receipt
quote_rejects_recipient_mismatch
```

Latest output confirmed all three Omnigate content-view tests passed. 

---

# 3. Omnigate content-view contract details

## 3.1 Quote endpoint

Endpoint:

```text
POST /v1/content/view/quote
```

Typical request:

```json
{
  "asset_crab_url": "crab://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.article",
  "payer_account": "acct_visitor_b",
  "viewer_wallet_account": "acct_visitor_b",
  "viewer_passport_subject": "passport:main:visitor-b",
  "recipient_account": "acct_creator",
  "max_amount_minor": "5",
  "client_idempotency_key": "content-view-quote-test"
}
```

Quote response schema:

```text
omnigate.content-view-quote.v1
```

Quote behavior:

```text
- parse asset target
- require canonical crab://<64 lowercase hex>.<kind>
- reject crab://b3/<hash>.<kind> slash form
- load asset manifest pointer from index
- load asset manifest from storage
- validate asset_cid
- validate asset_kind
- derive payout recipient from manifest
- reject recipient mismatch
- return amount_minor as string
- do not call wallet
```

Current default dev price:

```text
5 ROC minor units
```

Environment knob:

```text
OMNIGATE_CONTENT_VIEW_PRICE_MINOR
```

Default if unset:

```text
5
```

## 3.2 Pay endpoint

Endpoint:

```text
POST /v1/content/view/pay
```

Typical request:

```json
{
  "asset_crab_url": "crab://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.article",
  "payer_account": "acct_visitor_b",
  "viewer_wallet_account": "acct_visitor_b",
  "viewer_passport_subject": "passport:main:visitor-b",
  "recipient_account": "acct_creator",
  "amount_minor": "5",
  "asset": "roc",
  "quote_id": "content-view-test-quote",
  "quote_hash": "quotehash",
  "client_idempotency_key": "content-view-pay-test"
}
```

Payment response schema:

```text
omnigate.content-view-payment.v1
```

Pay behavior:

```text
- parse asset target
- load/validate asset manifest pointer
- load/validate manifest object
- validate payout action
- validate recipient account
- reject self-payment
- validate amount matches current content_view price
- call svc-wallet /v1/transfer
- recover from wallet nonce conflict when expected nonce is present
- return wallet receipt
- expose txid / receipt_hash / ledger_root
```

Wallet transfer memo format:

```text
crablink content_view crab://<hash>.<kind>
```

## 3.3 Asset target validation

Accepted asset kinds:

```text
image
article
post
comment
video
music
song
```

Accepted canonical URL shape:

```text
crab://<64 lowercase hex>.<asset_kind>
```

Rejected:

```text
crab://b3/<hash>.<kind>
URLs with path separators
URLs with control characters
URLs with query strings
URLs with fragments
uppercase hashes
wrong hash length
unsupported kind
```

This follows the current RustyOnions rule that public typed asset URLs use:

```text
crab://<64hex>.<kind>
```

while internal CIDs use:

```text
b3:<64hex>
```

## 3.4 Manifest payout behavior

The route derives payout recipient from manifest data.

Expected manifest fields:

```json
{
  "version": 1,
  "asset_cid": "b3:<hash>",
  "asset_kind": "article",
  "owner": {
    "passport_subject": "passport:main:creator",
    "wallet_account": "acct_creator"
  },
  "payout": {
    "default_action": "content_view",
    "recipient_account": "acct_creator"
  },
  "metadata": {
    "title": "Paid Article Fixture",
    "description": "A paid content_view test fixture.",
    "tags": ["article", "content_view"],
    "content_type": "application/json; charset=utf-8"
  }
}
```

Accepted payout actions:

```text
content_view
asset_view
view
<asset_kind>_view
```

Examples:

```text
article_view
post_view
comment_view
image_view
```

Fallback behavior:

```text
If payout.recipient_account is missing, route may fall back to owner.wallet_account.
If neither exists, route fails closed.
```

## 3.5 Important manifest DTO lesson

Initial issue:

```text
AssetManifestDocument and ManifestPayout were too strict.
```

The route initially used:

```rust
#[serde(deny_unknown_fields)]
```

on manifest DTOs. That caused normal/evolving manifest fields to fail deserialization and returned `502`.

Fields that caused tolerance problems:

```text
receipts
receipt_refs
provenance
storage
payout.splits
```

Fix:

```text
Keep request DTOs strict.
Make stored manifest DTOs tolerant.
Explicitly validate only critical economic fields.
Let Serde ignore unknown manifest fields by default.
```

Do **not** reintroduce `deny_unknown_fields` on stored manifest objects unless all real manifest fields are represented.

## 3.6 Important JSON macro lesson

Initial issue:

```text
recursion limit reached while expanding serde_json::json_internal!
```

Cause:

```text
One huge nested json! response object in content_view quote.
```

Fix:

```text
Split JSON construction into smaller objects:
policy
asset_page
quote
next
payment
receipt
response
```

Do not fix this by raising the crate recursion limit.

---

# 4. Omnigate files changed in detail

## 4.1 `crates/omnigate/src/routes/v1/content_view.rs`

Added full route implementation.

Main public functions:

```rust
pub fn router<S>() -> Router<S>
pub async fn content_view_quote(headers: HeaderMap, body: Bytes) -> Response
pub async fn content_view_pay(headers: HeaderMap, body: Bytes) -> Response
```

Important internal helpers:

```rust
load_content_view_context
fetch_asset_pointer
fetch_asset_manifest
send_wallet_content_view_transfer
content_view_payment_response
wallet_transfer_problem
parse_asset_target
parse_crab_asset_url
canonicalize_hash
normalize_asset_kind
is_content_view_action
content_view_payer_account
content_view_price_minor
default_content_view_nonce
wallet_base_url
wallet_bearer
storage_base_url
index_base_url
content_view_quote_id
content_view_idempotency_key
simple_hex_hash
is_canonical_b3_cid
normalize_minor_units_owned
decimal_gt
clean_optional
parse_expected_nonce
should_forward_header
is_hop_by_hop_or_host
value_string
grab
now_ms
problem
```

Current known limitations:

```text
pricing is fixed dev env price, not yet full roc-economics.toml-driven
single recipient payout is enforced, not split payout execution
quote_id / quote_hash are deterministic lightweight route-level values, not durable ledger truth
content_view has backend tests but not live stack smoke yet
CrabLink UI is not wired yet
```

## 4.2 `crates/omnigate/src/routes/v1/mod.rs`

Added module:

```rust
pub mod content_view;
```

Added router mount:

```rust
.nest("/content", content_view::router())
```

This is how `/v1/content/view/quote` and `/v1/content/view/pay` are exposed.

## 4.3 `crates/omnigate/tests/content_view.rs`

Added content-view integration-style route tests with dummy upstream services.

Important constants:

```rust
const HASH: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const ASSET_CID: &str = "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const MANIFEST_CID: &str = "b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const LEDGER_ROOT: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const RECEIPT_HASH: &str = "b3:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
```

The dummy wallet intentionally first returns:

```text
nonce conflict: expected 8
```

then accepts nonce `8`, proving nonce recovery.

## 4.4 `crates/omnigate/tests/site_visit.rs`

Added site-visit test coverage.

Important tests:

```text
quote_returns_10_roc_and_manifest_payout_recipient
pay_recovers_wallet_nonce_and_returns_wallet_receipt
quote_rejects_recipient_mismatch
```

This is the site-level counterpart to content-view.

---

# 5. Omnigate gates now green

Run:

```bash
cd /Users/mymac/Desktop/RustyOnions

cargo test -p omnigate --test site_visit
cargo test -p omnigate --test content_view
```

Expected:

```text
site_visit:    3 passed
content_view:  3 passed
```

Strict content-view gate:

```bash
RON_RUN_CLIPPY=1 bash scripts/web3_nextlevel_content_view_green_gate.sh
```

Latest status:

```text
PASS
```

---

# 6. Omnigate follow-up work

## 6.1 Highest priority next Omnigate follow-up

After CrabLink UI wiring, add live smoke:

```text
scripts/web3_paid_content_view_smoke.sh
```

Purpose:

```text
Use real stack services.
Ensure a known b3 article/post/comment asset exists.
Quote 5 ROC.
Pay from visitor to creator.
Assert payer -5.
Assert creator +5.
Assert txid / receipt_hash / ledger_root.
```

This should be modeled after:

```text
scripts/web3_paid_site_visit_smoke.sh
```

## 6.2 Future economics config integration

Current content-view pricing is:

```text
OMNIGATE_CONTENT_VIEW_PRICE_MINOR=5
```

Future target:

```toml
[actions.content_view]
enabled = true
pricing_kind = "flat"
price_minor = 5
```

in:

```text
configs/roc-economics.dev.toml
```

Do not do this until CrabLink UI is wired and current backend route remains green.

## 6.3 Future split payout support

Current content-view route only uses:

```text
payout.recipient_account
```

Future WEB3_2/NEXT_LEVEL likely needs:

```text
creator split
curator split
storage/provider split
treasury residual
```

But do not implement split settlement inside Omnigate directly. It should still call wallet through controlled route(s), or later use policy/rewarder planning, depending on architecture.

---

# SVC-GATEWAY Carryover Notes — Paid Site Visit + Paid Content View Proxy Routes

## 1. Crate role reminder

`svc-gateway` is the public browser/client edge.

It must remain:

```text
proxy-only for product routes
```

Correct role:

```text
CrabLink
→ svc-gateway public route
→ omnigate /v1 route
```

Gateway must not:

```text
parse product manifests
choose payout recipients
calculate prices
call svc-wallet directly for product payment
mutate ron-ledger
store bytes
write index pointers
compute accounting
compute rewards
```

Gateway may:

```text
expose clean public HTTP paths
preserve selected request headers
preserve request body
filter hop-by-hop headers
copy safe response headers
return upstream body/status
```

This boundary remained intact.

---

# 2. Major svc-gateway work completed this session

## 2.1 Paid site-visit proxy route was tested

File added:

```text
crates/svc-gateway/tests/site_visit_routes_proxy.rs
```

Purpose:

```text
Verify gateway exposes site_visit quote/pay as proxy-only routes.
```

Public routes:

```text
POST /sites/:name/visit/quote
POST /sites/:name/visit/pay
```

Proxied upstream:

```text
POST /v1/sites/:name/visit/quote
POST /v1/sites/:name/visit/pay
```

Test verifies:

```text
method preserved
path rewritten correctly to /v1/...
body preserved
authorization header preserved
x-ron-passport preserved
x-ron-wallet-account preserved
idempotency-key preserved
x-request-id preserved
x-correlation-id preserved
connection header not forwarded
```

Important Host-header lesson:

```text
Gateway must not forward inbound Host.
But reqwest/hyper must synthesize a new upstream Host header.
```

The original test incorrectly expected Host to be null. We fixed the test to expect:

```rust
assert_eq!(body["host"], omnigate_addr.to_string());
```

while still asserting:

```rust
assert!(body["connection"].is_null());
```

## 2.2 Paid content-view proxy routes added

File modified:

```text
crates/svc-gateway/src/routes/product.rs
```

New public routes:

```text
POST /content/view/quote
POST /content/view/pay
```

Upstream paths:

```text
POST /v1/content/view/quote
POST /v1/content/view/pay
```

Added handlers:

```rust
pub async fn content_view_quote(...)
pub async fn content_view_pay(...)
```

The handlers do only this:

```text
proxy_to_omnigate(&state, Method::POST, "/v1/content/view/quote", headers, body)
proxy_to_omnigate(&state, Method::POST, "/v1/content/view/pay", headers, body)
```

No product logic was added to gateway.

## 2.3 Paid content-view proxy test added

File added:

```text
crates/svc-gateway/tests/content_view_routes_proxy.rs
```

Purpose:

```text
Verify /content/view/quote and /content/view/pay proxy to omnigate and preserve context.
```

Covered public routes:

```text
/content/view/quote
/content/view/pay
```

Expected upstream routes:

```text
/v1/content/view/quote
/v1/content/view/pay
```

The test uses dummy Omnigate echo server.

It verifies:

```text
method = POST
path = expected /v1/content/view/*
body_len > 0
authorization preserved
x-ron-passport preserved
x-ron-wallet-account preserved
idempotency-key preserved
x-request-id preserved
x-correlation-id preserved
upstream Host synthesized correctly
Connection header not forwarded
```

Latest output confirmed this test passes. 

## 2.4 Product route docs/comments updated

File modified:

```text
crates/svc-gateway/src/routes/product.rs
```

Updated module header to include:

```text
content-view
```

in the product route exposure description.

Updated route list to include:

```text
POST /content/view/quote
POST /content/view/pay
```

Fixed Clippy doc-markdown issue:

```text
CrabLink
```

needed to be:

```text
`CrabLink`
```

Latest strict content-view gate passed after this fix. The previous failure was only a doc comment in `svc-gateway/src/routes/product.rs`, not a kernel issue. 

---

# 3. svc-gateway `product.rs` current route surface

The product router now exposes:

```text
GET  /identity/me
POST /identity/passport/bootstrap
POST /identity/passport/profile/claim
GET  /identity/passport/profile/:username

GET  /wallet/:account/balance
POST /wallet/hold

GET  /crab/resolve?url=...
GET  /b3/:asset

POST /paid/o/prepare

POST /assets/image/prepare
POST /assets/image

POST /assets/post/prepare
POST /assets/post

POST /assets/comment/prepare
POST /assets/comment

POST /assets/article/prepare
POST /assets/article

POST /content/view/quote
POST /content/view/pay

POST /sites/prepare
POST /sites
GET  /sites/:name

POST /sites/:name/visit/quote
POST /sites/:name/visit/pay
```

Key helpers remain:

```rust
proxy_to_omnigate
with_query
should_forward_header
should_copy_response_header
is_hop_by_hop_or_host
```

---

# 4. svc-gateway header forwarding contract

Allowed/preserved request headers include:

```text
authorization
accept
content-type
x-ron-*
x-correlation-id
x-request-id
idempotency-key
```

Filtered inbound headers include:

```text
host
connection
proxy-authorization
te
trailer
transfer-encoding
upgrade
content-length
```

Response headers copied unless unsafe:

```text
not transfer-encoding
not content-length
not connection
```

Important nuance:

```text
The inbound Host header is filtered.
The outbound HTTP client may synthesize a new Host header for the upstream request.
This is correct HTTP behavior.
```

The proxy tests encode this.

---

# 5. svc-gateway files changed in detail

## 5.1 `crates/svc-gateway/src/routes/product.rs`

Added content-view route exposure.

New routes in router:

```rust
.route("/content/view/quote", post(content_view_quote))
.route("/content/view/pay", post(content_view_pay))
```

New handlers:

```rust
pub async fn content_view_quote(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    proxy_to_omnigate(
        &state,
        Method::POST,
        "/v1/content/view/quote",
        headers,
        body,
    )
    .await
}
```

```rust
pub async fn content_view_pay(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    proxy_to_omnigate(
        &state,
        Method::POST,
        "/v1/content/view/pay",
        headers,
        body,
    )
    .await
}
```

No pricing, wallet, ledger, manifest, or storage logic added.

## 5.2 `crates/svc-gateway/tests/content_view_routes_proxy.rs`

Added proxy-only route test.

Key test:

```text
content_view_quote_and_pay_proxy_to_omnigate_and_preserve_context
```

Verifies route-level product boundary.

## 5.3 `crates/svc-gateway/tests/site_visit_routes_proxy.rs`

Added site-visit proxy-only route test.

Key test:

```text
site_visit_quote_and_pay_proxy_to_omnigate_and_preserve_context
```

This is the named-site counterpart to content-view proxy testing.

---

# 6. svc-gateway gates now green

Run:

```bash
cd /Users/mymac/Desktop/RustyOnions

cargo test -p svc-gateway --test site_visit_routes_proxy
cargo test -p svc-gateway --test content_view_routes_proxy
```

Expected:

```text
site_visit_routes_proxy:     1 passed
content_view_routes_proxy:   1 passed
```

Strict scoped content-view gate:

```bash
RON_RUN_CLIPPY=1 bash scripts/web3_nextlevel_content_view_green_gate.sh
```

Latest status:

```text
PASS
```

The latest terminal output showed both scoped Clippy steps passing and both content-view tests passing. The `ron-kernel` “Checking” lines appeared only because Cargo still typechecks dependencies; the failure was resolved without touching kernel. 

---

# 7. svc-gateway follow-up work

## 7.1 Do not add product logic to gateway

For the next CrabLink content-view UI work, do **not** add extra gateway logic unless a route is missing.

Gateway already exposes:

```text
/content/view/quote
/content/view/pay
```

CrabLink should call these public paths.

## 7.2 Add proxy tests if new public paths appear

If future backend adds:

```text
/content/view/live-smoke
/assets/:kind/:hash/view/quote
/assets/:kind/:hash/view/pay
```

or similar, add gateway proxy tests immediately.

## 7.3 Keep gateway route docs current

`docs/WEB3_2_PRODUCT_ROUTE_CONTRACTS.md` should be updated whenever a new gateway public product route becomes real and tested.

## 7.4 Watch for Clippy doc-markdown issues

`svc-gateway` has strict lints:

```rust
#![deny(clippy::all, clippy::pedantic)]
```

So route comments may fail Clippy for unbackticked product names. For docs/comments, prefer:

```text
`CrabLink`
`Omnigate`
`svc-wallet`
`ron-ledger`
`content_view`
```

where appropriate.

---

# 8. Cross-crate interaction notes

## 8.1 `content_view` route ownership

Ownership split:

```text
svc-gateway:
  public browser/client route
  proxy-only
  no product logic

omnigate:
  product route coordinator
  resolves manifest pointer
  loads manifest
  validates payout context
  calls svc-wallet

svc-index:
  asset hash → manifest CID pointer

svc-storage:
  stores manifest object and content bytes

svc-wallet:
  transfer front-door

ron-ledger:
  durable economic truth
```

## 8.2 `site_visit` route ownership

Ownership split:

```text
svc-gateway:
  /sites/:name/visit/quote
  /sites/:name/visit/pay
  proxy-only

omnigate:
  site manifest lookup
  payout recipient validation
  calls svc-wallet transfer

svc-wallet:
  balance movement and receipt

ron-ledger:
  append-only durable truth
```

## 8.3 Pattern established for future paid routes

Future paid routes should follow the same template:

```text
1. Gateway public proxy route
2. Gateway proxy test
3. Omnigate route implementation
4. Omnigate contract test with dummy upstreams
5. Optional wallet-level receipt replay test
6. Live smoke only after contract tests are green
7. CrabLink UI only after backend route contract is stable
```

Do not jump directly to UI before backend contract tests exist.

---

# 9. Current recommended next task

The next session should **not** make more Omnigate/Gateway changes first unless the CrabLink wiring exposes a missing response shape.

Best next task:

```text
Wire CrabLink paid content_view UI for article asset pages first.
```

CrabLink should call:

```text
POST /content/view/quote
POST /content/view/pay
```

through the gateway.

It should not call:

```text
/v1/content/view/quote
/v1/content/view/pay
svc-wallet
svc-index
svc-storage
ron-ledger
```

UI acceptance criteria:

```text
Open crab://<hash>.article
quote card appears
price is shown
recipient/creator info shown if available
no automatic spend
user clicks Pay
CrabLink calls /content/view/pay
receipt returns
article unlocks only after receipt
balance refreshes
txid shown
receipt_hash shown
ledger_root shown
Developer JSON remains available
```

---

# 10. Do-not-regress checklist for these two crates

## Omnigate do-not-regress

```text
POST /v1/sites/:name/visit/quote
POST /v1/sites/:name/visit/pay
POST /v1/content/view/quote
POST /v1/content/view/pay
manifest-derived payout recipient
recipient mismatch rejection
payer/header mismatch rejection
self-payment rejection for content_view
wallet /v1/transfer only
nonce conflict recovery
wallet receipt passthrough
txid / receipt_hash / ledger_root response fields
integer string amount_minor
strict request DTOs
tolerant stored manifest DTOs
no direct ledger mutation
```

## svc-gateway do-not-regress

```text
POST /sites/:name/visit/quote
POST /sites/:name/visit/pay
POST /content/view/quote
POST /content/view/pay
proxy-only behavior
selected header forwarding
body forwarding
query forwarding where relevant
hop-by-hop header filtering
no manifest parsing
no pricing
no wallet mutation
no ledger mutation
no storage writes
no accounting/rewarder logic
```

## Shared do-not-regress

```text
ron-kernel untouched
QuickChain locked
CrabLink only calls gateway public routes
money remains integer minor units
receipt_hash remains b3:<64 lowercase hex>
ledger_root remains present when wallet returns it
no silent spend
```


### END NOTE - MAY 16 2026 - 14:40 CST


### BEGIN NOTE - JUNE 14 2026 - 22:45 CST

I based these notes on the current `svc-gateway` bundle and the QuickChain blueprint/carryover context, especially the green preflight script and the new `quickchain_preflight_*` tests.

# svc-gateway QuickChain Phase-0 Crate Notes

## Status

`svc-gateway` is now effectively parked for the current QuickChain Phase-0 / preflight slice.

Current estimate:

```text
svc-gateway QuickChain Phase-0 / preflight completion: ~93–96%
```

The focused preflight gate is green:

```text
svc-gateway QuickChain preflight gate passed
```

This means `svc-gateway` now has a crate-local safety cage proving that it remains a public route/proxy/admission boundary and does not drift into wallet truth, ledger truth, receipt truth, storage truth, QuickChain authority, bridge authority, validator authority, finality authority, staking authority, liquidity authority, or cache-only paid-access authority.

The crate is ready to pause/park for this phase unless we decide to do optional polish later.

## Mission for this crate

The mission for `svc-gateway` was:

```text
Add QuickChain Phase-0/preflight docs, script, and focused tests proving svc-gateway stays a public boundary and does not become economic or chain authority.
```

The gateway sits between external/client traffic and RustyOnions internal services. Because of that, it is a dangerous place for fake claims to enter the system.

The crate needed to prove:

```text
client / CrabLink / Tauri / browser traffic
→ svc-gateway public boundary
→ omnigate / svc-storage / backend service paths
→ backend-derived response
→ render/unlock/display only from backend truth
```

The crate must not prove or claim:

```text
wallet balance truth
ledger truth
receipt truth
QuickChain roots
checkpoint hashes
validator signatures
bridge settlement
external settlement
staking state
liquidity state
paid unlock authority from cache
paid unlock authority from client headers
```

That mission is now covered by docs, focused tests, shared proxy header filtering, and a crate-local preflight script.

## What was added or changed

### Added `crates/svc-gateway/docs/quickchain-preflight.md`

A new crate-local QuickChain Phase-0 preflight document was added.

Purpose:

```text
Document svc-gateway's QuickChain boundary contract.
Keep future contributors from accidentally turning gateway into economic or chain authority.
Give tests a written contract to enforce.
```

The doc states that `svc-gateway` is:

```text
public route boundary
admission boundary
proxy boundary
correlation/metrics boundary
fail-closed edge
```

The doc states that `svc-gateway` is not:

```text
wallet truth
ledger truth
storage truth
receipt truth
balance truth
QuickChain authority
chain runtime
root producer
checkpoint writer
validator
bridge
anchor writer
external settlement actor
staking actor
liquidity actor
```

The doc preserves the key QuickChain Phase-0 rules:

```text
wallet remains mutation front-door
ron-ledger remains durable economic truth
accounting is not balance truth
rewarder does not mutate ledger
gateway does not mutate ledger
gateway does not create fake balances
gateway does not create fake receipts
gateway does not claim finality
gateway does not produce roots
gateway does not unlock paid content from client/cache claims alone
```

It also explicitly keeps future QuickChain work parked outside the gateway:

```text
canonical bytes and locked vectors
state/account Merkle roots
receipt roots
checkpoint signing
validator-set logic
external DA
public anchors
bridges
staking or liquidity
CrabLink chain authority
gateway/omnigate ledger mutation
```

### Added `crates/svc-gateway/scripts/dev-quickchain-preflight.sh`

A focused local preflight script was added and made executable.

Current role:

```text
Run only the svc-gateway QuickChain boundary tests plus selected proxy regressions.
Run clippy only for svc-gateway using -p svc-gateway and --no-deps.
Print a clear pass marker when the gate is green.
```

Current script shape:

```bash
cargo test -p svc-gateway --test quickchain_preflight_boundary
cargo test -p svc-gateway --test quickchain_preflight_docs
cargo test -p svc-gateway --test quickchain_preflight_no_fake_receipts
cargo test -p svc-gateway --test quickchain_preflight_cache_boundary

cargo test -p svc-gateway --test app_proxy
cargo test -p svc-gateway --test paid_storage_estimate_proxy
cargo test -p svc-gateway --test paid_storage_write_proxy
cargo test -p svc-gateway --test product_routes_proxy

cargo clippy -p svc-gateway --all-targets --no-deps -- -D warnings

echo "svc-gateway QuickChain preflight gate passed"
```

Important note:

```text
Cargo may still print "Checking ron-kernel", "Checking ron-proto", etc. because those are dependencies.
That is normal dependency checking/compilation.
The clippy command is still targeted at svc-gateway with -p svc-gateway --no-deps.
```

### Added `crates/svc-gateway/src/headers/proxy.rs`

A new shared proxy header filtering module was added.

Purpose:

```text
Centralize request/response header filtering for gateway proxy routes.
Prevent client-supplied or upstream-supplied QuickChain/economic authority headers from crossing the public boundary as trusted claims.
```

Main exported helpers:

```rust
should_forward_passthrough_header(...)
should_forward_product_header(...)
should_copy_response_header(...)
```

The helper strips ordinary hop-by-hop headers, including:

```text
Host
Connection
Proxy-Authenticate
Proxy-Authorization
TE
Trailer
Transfer-Encoding
Upgrade
Content-Length
```

The helper also strips QuickChain/economic/authority-looking headers, including:

```text
x-ron-account-sequence
x-ron-operation-id
x-ron-state-root
x-ron-receipt-root
x-ron-accounting-root
x-ron-reward-root
x-ron-checkpoint-root
x-ron-checkpoint-hash
x-ron-data-availability-root
x-ron-validator-set
x-ron-validator-signature
x-ron-finality
x-ron-finalized
x-ron-anchored
x-ron-anchor
x-ron-bridge
x-ron-external-settlement
x-ron-staking
x-ron-liquidity
x-ron-balance
x-ron-entitlement
x-ron-unlock-authorized
```

It also denies authority-looking prefixes:

```text
x-ron-quickchain-
x-ron-validator-
x-ron-bridge-
x-ron-anchor-
x-ron-checkpoint-
x-ron-root-
x-ron-ledger-
```

Safe product request headers remain allowed:

```text
Authorization
Accept
Content-Type
x-correlation-id
x-request-id
idempotency-key
safe x-ron-* metadata that is not authority-looking
```

Important semantic rule:

```text
idempotency-key is retry identity only.
It is not operation_id.
It is not chain authority.
It is not spend authority.
```

### Updated `src/headers/mod.rs`

The new proxy module was exported:

```rust
pub mod proxy;
```

### Updated proxy route modules to use shared header filters

The following route modules were updated to use `headers::proxy` helpers instead of local ad hoc filtering:

```text
src/routes/app.rs
src/routes/objects.rs
src/routes/paid_storage.rs
src/routes/product.rs
```

This prevents each route from inventing its own inconsistent header policy.

### Updated `src/routes/app.rs`

`/app/*` remains a proxy to omnigate:

```text
gateway /app/{tail}
→ omnigate /v1/app/{tail}
```

Behavior now covered:

```text
preserves method
preserves body bytes
preserves path tail
preserves query string
forwards safe headers only
strips hop-by-hop headers
strips caller-supplied QuickChain authority/finality headers
passes upstream 4xx/5xx bodies through
maps upstream transport failures to 502 Problem JSON
filters response headers before returning to caller
```

### Updated `src/routes/objects.rs`

`/o/:addr` remains a raw object proxy to `svc-storage`:

```text
gateway /o/:addr
→ svc-storage /o/:addr
```

Behavior now covered:

```text
GET proxy
HEAD proxy
safe request header forwarding
safe response header copying
no storage writes at gateway
no wallet mutation
no ledger mutation
no receipt creation
no paid access authority
default application/octet-stream for GET if upstream does not provide Content-Type
```

The object route remains a byte/object path, not economic truth.

### Updated `src/routes/paid_storage.rs`

Paid storage public routes remain proxy-only:

```text
gateway /paid/o/estimate
→ omnigate /v1/paid/o/estimate

gateway /paid/o
→ omnigate /v1/paid/o
```

Covered behavior:

```text
estimate route is read-only
write route is proxy-only
gateway does not quote as authority
gateway does not create holds
gateway does not capture payment
gateway does not release payment
gateway does not write storage directly
gateway does not mint receipts
gateway does not mutate ledger
gateway forwards only safe product headers
gateway filters response authority headers
```

### Updated `src/routes/product.rs`

Product route proxy behavior was hardened.

Product routes remain public CrabLink/WEB3-facing edge routes backed by omnigate:

```text
identity routes
wallet display/hold proxy routes
crab resolve
b3 asset resolve
paid prepare
image prepare/upload
video prepare/upload
music prepare/upload
podcast prepare/upload
stream prepare/publish/status/segment routes
post/comment/article prepare/publish
content view quote/pay
site prepare/create/resolve/visit quote/pay
chat resolve/prepare/create/messages/moderation
```

Important boundaries documented and preserved:

```text
gateway does not create passports
gateway does not create keys
gateway does not create wallets
gateway does not reserve usernames
gateway does not own wallet balance truth
gateway does not mutate wallet directly
gateway does not price storage/content
gateway does not create wallet holds
gateway does not validate media semantics as authority
gateway does not store bytes
gateway does not write index pointers
gateway does not mint stream truth
gateway does not verify paid access by itself
gateway does not issue paid unlocks
```

The product proxy now uses `should_forward_product_header` and `should_copy_response_header`.

The clippy `manual_let_else` issue in media upload handlers was fixed by using `let Ok(body) = ... else { ... }`.

Doc markdown issues in the proxy helper docs were fixed so clippy pedantic passes.

## Tests added

### Added `tests/quickchain_preflight_boundary.rs`

Purpose:

```text
Prove gateway is a public boundary, not QuickChain runtime or economic authority.
```

Covered tests:

```text
route_source_keeps_wallet_surface_limited_to_proxy_hold_and_display_balance
gateway_source_does_not_import_wallet_ledger_or_quickchain_runtime_authority
unsupported_quickchain_ledger_bridge_root_routes_fail_closed_at_gateway
product_proxy_strips_caller_supplied_quickchain_authority_headers
```

What this proves:

```text
gateway has only limited wallet surface suitable for proxy/display semantics
gateway does not import mutation authority from wallet/ledger/QuickChain runtime crates
unsupported QuickChain/root/ledger/bridge routes fail closed
product proxy strips caller-supplied authority headers
```

Key protected concepts:

```text
ledger mutation
wallet mutation authority
QuickChain runtime authority
root endpoints
bridge endpoints
validator endpoints
checkpoint endpoints
fake finality
fake authority headers
```

### Added `tests/quickchain_preflight_docs.rs`

Purpose:

```text
Keep docs, script, and gateway boundary doctrine aligned.
```

Covered tests:

```text
docs_state_gateway_is_proxy_boundary_not_chain_runtime
docs_name_allowed_and_forbidden_phase_zero_scope
docs_define_header_boundary_and_operation_identity_rule
docs_list_focused_preflight_and_proxy_suites
docs_keep_future_quickchain_work_parked_outside_gateway
```

What this proves:

```text
the QuickChain preflight doc exists
the doc states gateway is proxy boundary, not chain runtime
the doc preserves allowed/forbidden Phase-0 scope
the doc preserves operation identity rules
the doc lists focused preflight suites and runner
the doc keeps root/checkpoint/validator/bridge/future work parked outside gateway
```

Important exact concepts preserved by docs test:

```text
x-ron-operation-id
x-ron-account-sequence
x-ron-state-root
x-ron-receipt-root
x-ron-checkpoint-hash
x-ron-validator-*
x-ron-bridge-*
x-ron-quickchain-*
operation_id is backend-assigned
idempotency-key is retry identity only
account_sequence is ledger-assigned
gateway cannot decide paid access from caller-supplied claims alone
```

### Added `tests/quickchain_preflight_no_fake_receipts.rs`

Purpose:

```text
Prove gateway cannot smuggle fake economic or chain authority through request or response headers.
```

Covered tests:

```text
response_headers_cannot_smuggle_economic_or_chain_authority
request_headers_cannot_smuggle_economic_or_chain_authority
backend_receipt_metadata_remains_display_context_not_gateway_authority
```

What this proves:

```text
request headers cannot smuggle fake balances
request headers cannot smuggle fake unlocks
request headers cannot smuggle roots
request headers cannot smuggle finality
request headers cannot smuggle validator approval
request headers cannot smuggle bridge settlement
request headers cannot smuggle staking/liquidity/external settlement
response headers cannot return gateway-endorsed fake authority
backend receipt metadata may pass only as display/backend-validation context
```

Important distinction captured:

```text
x-ron-receipt-id
x-ron-receipt-hash
x-ron-quote-id
```

These are allowed as metadata/display/backend-validation context, but not as gateway-owned economic authority.

This distinction matters because the gateway must be allowed to relay backend-derived receipt context, but must not invent or bless it.

### Added `tests/quickchain_preflight_cache_boundary.rs`

Purpose:

```text
Prove cache/ETag/transport metadata cannot unlock paid content.
```

Covered tests:

```text
cache_headers_are_transport_metadata_not_unlock_authority
cache_or_client_unlock_signals_are_stripped_when_they_claim_authority
route_source_does_not_unlock_paid_content_from_cache_or_query_claims
```

What this proves:

```text
Cache-Control remains transport metadata
ETag remains transport metadata
If-None-Match remains transport metadata
If-Modified-Since remains transport metadata
If-Unmodified-Since remains transport metadata
cache metadata is not paid unlock truth
client/cache unlock headers are stripped if they claim authority
route source does not define paid=true / unlock=true / cache_unlocked shortcuts
```

The source scan was fixed to use:

```rust
env!("CARGO_MANIFEST_DIR")
```

Reason:

```text
Cargo integration tests run with the package working directory, not necessarily workspace root.
```

The fixed test scans:

```text
<svc-gateway crate root>/src/routes
```

rather than:

```text
crates/svc-gateway/src/routes
```

which failed when run from the crate context.

## Existing proxy regression tests now included in the QuickChain gate

The final preflight script also runs existing proxy regressions:

```text
app_proxy
paid_storage_estimate_proxy
paid_storage_write_proxy
product_routes_proxy
```

These are important because the QuickChain hardening changed proxy header behavior. The regressions prove we did not break normal product traffic while stripping authority headers.

### `app_proxy`

Green tests:

```text
app_proxy_happy_path
app_proxy_preserves_auth_and_corr_headers_and_generates_request_id
app_proxy_passes_problem_bodies_unchanged
app_proxy_preserves_methods_body_and_query
app_proxy_upstream_connect_failure_yields_problem_502
```

What it proves:

```text
/app routes still proxy correctly
methods/body/query survive
safe auth/correlation headers survive
upstream problem bodies pass through
upstream connection failure becomes 502 Problem JSON
```

### `paid_storage_estimate_proxy`

Green tests:

```text
paid_storage_estimate_proxy_happy_path
paid_storage_estimate_proxy_passes_omnigate_errors_through
paid_storage_estimate_upstream_connect_failure_yields_problem_502
```

What it proves:

```text
paid storage estimate is still proxied to omnigate
gateway does not take pricing authority
omnigate errors pass through
connect failure fails closed as 502
```

### `paid_storage_write_proxy`

Green tests:

```text
paid_storage_write_proxy_post_happy_path
paid_storage_write_proxy_put_happy_path
paid_storage_write_proxy_passes_omnigate_errors_through
paid_storage_write_upstream_connect_failure_yields_problem_502
```

What it proves:

```text
paid storage writes are still proxied
POST and PUT paths work
gateway does not perform write semantics itself
gateway does not create storage truth
gateway does not mutate wallet/ledger
upstream failure fails closed
```

### `product_routes_proxy`

Green tests:

```text
crab_resolve_proxy_preserves_query_and_headers
b3_asset_proxy_targets_omnigate_b3_route
paid_prepare_proxy_preserves_body_and_idempotency
image_site_and_text_product_routes_target_omnigate
site_resolve_proxy_targets_omnigate_site_hydrator
text_asset_publish_routes_preserve_paid_proof_headers
chat_product_routes_target_omnigate
product_route_upstream_connect_failure_yields_problem_502
```

What it proves:

```text
CrabLink product routes remain wired
b3 asset routes target omnigate correctly
paid prepare keeps body/idempotency semantics
image/site/text routes target omnigate
site resolve targets omnigate hydrator
paid proof metadata can be preserved where safe
chat routes target omnigate
upstream failures fail closed
```

## Final green terminal state

The latest terminal run showed:

```text
quickchain_preflight_cache_boundary: 3 passed
quickchain_preflight_no_fake_receipts: 3 passed
quickchain_preflight_docs: 5 passed
quickchain_preflight_boundary: 4 passed
app_proxy: 5 passed
paid_storage_estimate_proxy: 3 passed
paid_storage_write_proxy: 4 passed
product_routes_proxy: 8 passed
cargo clippy -p svc-gateway --all-targets --no-deps -- -D warnings: passed
svc-gateway QuickChain preflight gate passed
```

This is the expected parking state for `svc-gateway`.

## Architectural boundary now proven

`svc-gateway` may do these things:

```text
expose public routes
normalize public paths
proxy selected requests
forward safe headers
strip hop-by-hop headers
strip authority-looking headers
preserve safe auth/correlation/idempotency metadata
enforce body caps
enforce route-level rate/concurrency/timeout controls
return upstream status/body after safe filtering
return gateway-safe errors
emit bounded metrics
attach request/correlation IDs
fail closed on missing upstream dependencies
```

`svc-gateway` must not do these things:

```text
mint ROC
issue ROC
transfer ROC
burn ROC
hold ROC as gateway authority
capture ROC
release ROC
mutate ron-ledger
mutate wallet state directly
create wallet receipts
invent receipt hashes
invent balances
invent payment finality
invent storage truth
invent index truth
invent QuickChain roots
invent checkpoints
invent validator approval
invent bridge settlement
accept client QuickChain DTOs as authority
accept client-supplied roots as authority
accept client-supplied finality as authority
accept client-supplied unlock headers as authority
unlock paid content from cache alone
claim external settlement
claim staking or liquidity status
```

## Important semantics captured

### Gateway is not wallet truth

The gateway may expose a browser-facing wallet route, such as balance display or hold proxy routes, but only as proxy paths to backend service flow.

The gateway must not calculate balances, store balances, or invent balances.

### Gateway is not ledger truth

The gateway must not import or directly mutate `ron-ledger`.

The ledger remains durable economic truth.

### Gateway is not receipt truth

The gateway may relay backend receipt metadata as display/backend-validation context.

The gateway must not fabricate receipts, receipt roots, receipt hashes, or finality.

### Gateway is not QuickChain runtime

The gateway must not create roots, checkpoints, validators, anchors, bridges, or external settlement state.

### Gateway cache is not paid unlock truth

Cache headers and ETags are transport metadata only.

They may optimize delivery.

They must never authorize access.

### Idempotency key is not operation identity

`idempotency-key` may be forwarded as retry identity.

It is not `operation_id`.

It is not ledger sequence.

It is not account sequence.

It is not chain authority.

### Backend receipt metadata is allowed only as context

Allowed metadata examples:

```text
x-ron-receipt-id
x-ron-receipt-hash
x-ron-quote-id
```

These may be forwarded/copied as backend-derived display or validation context.

They are not gateway authority.

## Files added

```text
crates/svc-gateway/docs/quickchain-preflight.md
crates/svc-gateway/src/headers/proxy.rs
crates/svc-gateway/tests/quickchain_preflight_boundary.rs
crates/svc-gateway/tests/quickchain_preflight_docs.rs
crates/svc-gateway/tests/quickchain_preflight_no_fake_receipts.rs
crates/svc-gateway/tests/quickchain_preflight_cache_boundary.rs
crates/svc-gateway/scripts/dev-quickchain-preflight.sh
```

## Files modified

```text
crates/svc-gateway/src/headers/mod.rs
crates/svc-gateway/src/routes/app.rs
crates/svc-gateway/src/routes/objects.rs
crates/svc-gateway/src/routes/paid_storage.rs
crates/svc-gateway/src/routes/product.rs
```

## Notable bug fixes during the session

### Fixed Markdown/codeblock escape issue in generated patch

Earlier output had nested code fences that broke shell copy/paste.

Resolved by providing shell patches without nested Markdown fences inside the shell block.

### Fixed docs test missing exact script marker

`quickchain_preflight_docs` required this exact string:

```text
`scripts/dev-quickchain-preflight.sh`
```

The docs were patched to contain the exact backticked marker.

### Fixed clippy doc_markdown issues

`src/headers/proxy.rs` originally triggered clippy pedantic documentation errors for unbackticked identifiers such as:

```text
routes::app
routes::objects
routes::paid_storage
routes::product
QuickChain
quickchain_preflight_boundary
app_proxy
paid_storage_*_proxy
product_routes_proxy
```

The docs/comments were cleaned up so `cargo clippy -p svc-gateway --all-targets --no-deps -- -D warnings` passes.

### Fixed clippy manual_let_else issues

`src/routes/product.rs` upload handlers originally triggered clippy `manual_let_else`.

The code was changed to use `let Ok(body) = ... else { ... }` form.

### Fixed cache-boundary source-scan path

`quickchain_preflight_cache_boundary.rs` originally looked for:

```text
crates/svc-gateway/src/routes
```

That failed because integration tests run from the crate package context.

Fixed to use:

```rust
Path::new(env!("CARGO_MANIFEST_DIR")).join("src/routes")
```

After that fix, the test passed.

## Current acceptance command

Run from repository root:

```bash
crates/svc-gateway/scripts/dev-quickchain-preflight.sh
```

Expected final output:

```text
svc-gateway QuickChain preflight gate passed
```

Manual focused gate:

```bash
cargo fmt -p svc-gateway
cargo test -p svc-gateway --test quickchain_preflight_boundary
cargo test -p svc-gateway --test quickchain_preflight_docs
cargo test -p svc-gateway --test quickchain_preflight_no_fake_receipts
cargo test -p svc-gateway --test quickchain_preflight_cache_boundary
cargo test -p svc-gateway --test app_proxy
cargo test -p svc-gateway --test paid_storage_estimate_proxy
cargo test -p svc-gateway --test paid_storage_write_proxy
cargo test -p svc-gateway --test product_routes_proxy
cargo clippy -p svc-gateway --all-targets --no-deps -- -D warnings
```

## Why svc-gateway is parkable now

`svc-gateway` is parkable because it now satisfies the core parking checklist:

```text
QuickChain docs exist and describe gateway boundary.
Focused preflight tests exist and pass.
Preflight script runs the focused gate.
Fake receipts/balances/finality are rejected or source-guarded.
Direct ledger/wallet mutation is absent from gateway authority.
Paid access remains backend-derived.
Cache cannot unlock paid content alone.
Product/app/paid proxy regressions remain green.
Clippy -D warnings passes for svc-gateway.
Codebundle was regenerated after patches.
```

The crate now has a meaningful safety cage against the most dangerous gateway mistakes:

```text
fake client paid claims
fake receipt headers
fake balance headers
fake root headers
fake finality headers
fake validator headers
fake bridge headers
cache-only unlocks
gateway-side wallet mutation
gateway-side ledger mutation
gateway-owned chain authority
```

## What remains for svc-gateway later

The remaining work is not needed before moving to `omnigate`, but should be tracked.

### 1. Optional object-route proxy regression

Existing `/o/:addr` logic is hardened through shared header helpers, but there is no focused `objects_proxy` test in the current preflight script.

Future optional test:

```text
objects_proxy
```

Should prove:

```text
GET /o/:addr proxies to svc-storage
HEAD /o/:addr proxies to svc-storage
range headers, if added later, stay bounded
hop-by-hop headers are stripped
authority-looking x-ron headers are stripped
response authority-looking headers are stripped
Content-Type fallback remains safe
gateway does not unlock paid content from object cache
```

### 2. Optional identity/content/site proxy suites in preflight script

The codebase already contains additional proxy tests such as:

```text
identity_routes_proxy
content_view_routes_proxy
site_visit_routes_proxy
```

These are not currently included in `dev-quickchain-preflight.sh`.

Future optional choice:

```text
Either keep current script lean,
or include these once their runtime cost and stability are acceptable.
```

Do not add them blindly if the script becomes too slow for the focused QuickChain gate.

### 3. Optional observability boundary test

A future test could assert that metrics/log labels do not include high-cardinality or authority-sensitive values.

Should forbid metric labels containing:

```text
wallet account IDs
payer IDs
escrow IDs
receipt hashes
wallet transaction IDs
CIDs as labels
private capability tokens
state roots
receipt roots
checkpoint hashes
validator IDs
bridge proof IDs
```

Current metrics are mostly generic HTTP/readiness/reject metrics. This is optional future hardening.

### 4. Optional gateway storage proxy preflight test

A future `quickchain_preflight_gateway_storage_proxy.rs` could explicitly prove object routes speak only in b3/content-addressed object terms and not economic terms.

Potential checks:

```text
/o/:addr does not create receipt fields
/o/:addr does not create balance fields
/o/:addr does not claim paid unlock
/o/:addr does not claim state root / receipt root / finality
/o/:addr is raw object proxy only
```

### 5. Optional response-body fake-claim scan

We currently guard request/response authority headers and route source cache unlock snippets.

A future test could additionally scan gateway response body construction for forbidden economic truth fields, while allowing tests/docs to contain those strings as guardrails.

Forbidden production response-body fields would include:

```text
balance_minor
available_minor
held_minor
receipt_root
state_root
checkpoint_hash
validator_signature
finalized
anchored
bridge_settled
unlock_authorized
paid: true
```

This should be careful to avoid false positives in docs/tests.

### 6. Optional route inventory snapshot

A future test could snapshot route intent and make accidental new authority routes fail.

Forbidden gateway-owned public routes:

```text
/quickchain
/checkpoint
/checkpoints
/state-root
/receipt-root
/validator
/validators
/bridge
/anchor
/anchors
/staking
/liquidity
/settlement
/finality
/ledger
/ledger/mutate
/wallet/issue
/wallet/transfer
/wallet/burn
/wallet/capture
/wallet/release
```

Caution:

```text
/wallet/hold exists as a proxy route to omnigate in current product surface.
Do not ban that route unless the product design changes.
Instead, ensure it remains proxy-only and not direct gateway mutation.
```

### 7. Optional runbook polish

The docs can later include a short operator runbook:

```text
how to run the preflight gate
what each test proves
what a failure usually means
what not to fix by weakening the test
how to add new gateway routes safely
```

### 8. Optional CI integration

The focused script could eventually be wired into workspace CI.

Do not run full workspace clippy here until unrelated crates are clean.

Preferred local gate remains:

```bash
crates/svc-gateway/scripts/dev-quickchain-preflight.sh
```

### 9. Future gateway production hardening unrelated to QuickChain

Not required for this QuickChain parking point, but future gateway work may include:

```text
more complete config file loading
remote gateway mode hardening
TLS mode polish
auth/admission policy integration
production rate-limit tuning
range/media proxy hardening
structured redaction policy
public API stability checks
fuzz target implementation
perf guard implementation
```

Keep these separate from QuickChain Phase-0 boundary work.

## What must not be added to svc-gateway

Do not add the following to `svc-gateway` during current QuickChain work:

```text
QuickChain roots
checkpoint creation
checkpoint signing
validator set logic
bridge logic
anchor publishing
external settlement
ROX
Solana
staking
liquidity
exchange-facing logic
ledger mutation
wallet mutation authority
receipt generation
balance calculation
reward distribution
pruning
DA/challenge/archive behavior
CrabLink chain authority
cache-authorized paid unlocks
```

If a future route appears to need any of those, the route probably belongs behind the gateway in `omnigate`, `svc-wallet`, `ron-ledger`, `ron-accounting`, `svc-rewarder`, `svc-storage`, `svc-index`, or `ron-policy`, not in `svc-gateway`.

## Relationship to other QuickChain crates

Current mental model:

```text
ron-proto = DTOs and strict shape contracts
ron-ledger = durable economic truth
svc-wallet = economic mutation front-door
ron-accounting = snapshots, not balances
svc-rewarder = payout planning, not mutation
svc-storage = bytes by b3, not receipts/balances
svc-gateway = public route/proxy/admission boundary
omnigate = hydration/product coordination, not ledger truth
svc-index = lookup metadata, not balance truth
ron-policy = declarative policy, not mutation
CrabLink/Tauri = display/user intent, not authority
```

`svc-gateway` now fits its role:

```text
public boundary
safe proxy
fail-closed edge
not economic truth
not QuickChain authority
```

## Next crate

The next recommended crate is:

```text
omnigate
```

Why:

```text
omnigate sits immediately behind svc-gateway.
omnigate coordinates hydration/product views.
omnigate is close to paid access decisions.
omnigate touches content view, site visit, stream-lite, chat, paid storage, wallet-facing flows, storage/index paths, and product composition.
omnigate must not become ledger truth.
omnigate must not invent receipts.
omnigate must not invent balances.
omnigate must not mutate ron-ledger directly.
omnigate must not unlock paid content from cache or caller claims.
```

Expected next mission:

```text
Add QuickChain Phase-0/preflight docs, script, and focused tests proving omnigate is a hydration/product coordination boundary, not wallet truth, ledger truth, receipt truth, root authority, finality authority, bridge authority, validator authority, or cache-only paid unlock authority.
```

Likely omnigate tests:

```text
quickchain_preflight_boundary
quickchain_preflight_no_direct_ledger_mutation
quickchain_preflight_paid_access
quickchain_preflight_no_fake_receipts
quickchain_preflight_cache_boundary
quickchain_preflight_docs
existing product/paid route regressions
```

## Parking statement

As of this note, `svc-gateway` has completed its current QuickChain Phase-0/preflight mission.

The crate should be considered:

```text
green
parkable
safe to move past for now
ready for future optional polish later
```

The project should move to `omnigate` next unless a new regression appears in `svc-gateway`.

### END NOTE — svc-gateway QuickChain Phase-0/preflight parking notes


### END NOTE - JUNE 14 2026 - 22:45 CST



### BEGIN NOTE - JUNE 17 2026 - 17:55 CST

# QuickChain Phase 0 / Preflight Notes — `svc-gateway` + `omnigate`

## Session Status

This session completed and parked the QuickChain Phase 0/preflight boundary work for both:

* `svc-gateway`
* `omnigate`

Both crates are now green for the current QuickChain parking slice. The work performed here does **not** implement QuickChain roots, checkpoints, validators, bridges, staking, pruning, settlement, external anchors, or public-chain authority. This work intentionally hardens the boundary around those future concepts and proves these crates do not accidentally become chain runtimes, ledger mutators, fake receipt issuers, or paid-access bypass points.

The important architectural result is that `svc-gateway` and `omnigate` are now explicitly documented, tested, and scripted as **transport / hydration / coordination surfaces only**, while economic truth remains owned by the internal ROC backend path:

`ron-proto econ DTOs -> ron-ledger truth -> svc-wallet mutation front door -> svc-storage / svc-gateway / omnigate paid enforcement -> ron-accounting snapshots -> svc-rewarder payout planning`

These crates now preserve the doctrine:

* Backend owns truth.
* Wallet / ledger own economic mutation.
* Gateway and Omnigate coordinate and proxy.
* Cache is convenience only.
* b3 proves bytes, not payment.
* crab:// is navigation, not authority.
* QuickChain is future settlement infrastructure, not current runtime.
* No fake balances.
* No fake receipts.
* No transport-header authority.
* No public QuickChain authority routes.

---

# `svc-gateway` QuickChain Phase 0 Notes

## Current Status

`svc-gateway` is parked and green for the current QuickChain Phase 0/preflight slice.

The crate is now treated as a public boundary / gateway surface. It may accept public traffic, normalize and route requests, forward safe context, and proxy to Omnigate / RustyOnions services. It must **not** become a ledger, wallet, checkpoint producer, validator, bridge, authority root, balance source, receipt fabricator, or QuickChain runtime.

The successful parking result means the crate currently satisfies the intended Phase 0 safety posture:

* No direct QuickChain runtime authority.
* No direct ledger mutation authority.
* No fake receipt generation.
* No fake balance generation.
* No paid unlock from client headers.
* No cache-only unlock.
* No gateway-created finality.
* No public root/checkpoint/validator/bridge routes.
* Gateway remains a public transport boundary, not economic truth.

## Role of `svc-gateway`

`svc-gateway` is the public entry boundary for client/product traffic. In the CrabLink/Tauri model, normal production flow is:

`CrabLink Tauri -> configured svc-gateway -> omnigate -> RustyOnions services`

The gateway is allowed to:

* Receive public client requests.
* Normalize request shape.
* Apply transport-level controls.
* Forward requests to Omnigate or other configured services.
* Preserve safe correlation/idempotency context.
* Strip or reject unsafe authority claims from clients.
* Fail closed when unsupported QuickChain/economic authority routes are requested.
* Keep the client/API edge clean and reviewable.

The gateway is **not** allowed to:

* Mutate `ron-ledger`.
* Create wallet holds.
* Capture/release holds.
* Mint ROC.
* Burn ROC.
* Transfer ROC.
* Invent balances.
* Invent receipts.
* Mark settlement final.
* Produce roots.
* Produce checkpoints.
* Run validators.
* Expose bridge routes.
* Treat `x-ron-*` or QuickChain-like request headers as economic authority.
* Let client cache state unlock paid content.
* Treat b3 hashes as payment proof.
* Treat crab:// navigation as economic authority.

## Boundary Hardened

The gateway now has a specific QuickChain preflight boundary around public routes and proxy behavior.

The main boundary invariant is:

`svc-gateway is a transport/public boundary, not a chain authority and not a ledger/wallet mutation service.`

This is important because `svc-gateway` sits at the highest-risk boundary: it is closest to public clients, therefore it must aggressively refuse any smuggled client-side economic authority.

The gateway must only forward or proxy to backend truth sources. It must not interpret user-supplied headers, local cache claims, local receipt cache entries, or client DTOs as proof of payment.

## Docs Added / Hardened

A QuickChain preflight documentation file was added or hardened under the crate docs area.

Purpose of the docs:

* Define `svc-gateway` as a public route/proxy boundary.
* State that it is not a chain runtime.
* State that it is not a validator.
* State that it is not a bridge.
* State that it is not a checkpoint/root producer.
* State that it is not a wallet.
* State that it is not a ledger.
* State that it does not own ROC balance truth.
* State that it does not own receipt truth.
* State that paid access must come from backend wallet/ledger receipt validation.
* State that transport headers are not economic authority.
* State that cache is convenience only.
* State that b3 hashes prove bytes, not payment.
* Preserve QuickChain operation identity rules:

  * `operation_id` is backend-assigned durable ledger operation identity.
  * `idempotency_key` is a retry key, not authority.
  * `account_sequence` is ledger-assigned.
  * `hold_id` identifies one hold lifecycle.
  * expiry is epoch-based.

The docs were made scanner-friendly so the tests can enforce exact architectural phrases over time. This is deliberate: the crate documentation is now part of the preflight gate, not just a human note.

## Scripts Added / Hardened

Two crate-local scripts were added or hardened:

### `crates/svc-gateway/scripts/dev-quickchain-preflight.sh`

This is the main exhaustive QuickChain preflight runner for the crate.

It performs the crate-specific gate:

* Verifies required docs exist.
* Verifies test directory exists.
* Rejects Python helper drift under the crate.
* Runs formatting check.
* Dynamically discovers `quickchain*.rs` tests.
* Runs all focused QuickChain preflight tests.
* Runs relevant paid/product/proxy regressions.
* Runs all-target tests.
* Runs clippy with warnings denied.
* Prints explicit forbidden-scope parking message.

The important thing is that the script dynamically discovers QuickChain tests, so new `quickchain*.rs` files will automatically become part of the crate parking gate.

### `crates/svc-gateway/scripts/dev-quickchain-park.sh`

This is the parking wrapper.

It verifies the core QuickChain docs/scripts/tests exist, delegates to the preflight script, and prints the final parking marker once all gates pass.

The parking script makes it clear when `svc-gateway` is safe to leave and move to the next crate.

## Tests Added / Hardened

The `svc-gateway` QuickChain preflight suite now includes seven focused QuickChain test files:

1. `quickchain_preflight_boundary.rs`
2. `quickchain_preflight_cache_boundary.rs`
3. `quickchain_preflight_docs.rs`
4. `quickchain_preflight_no_fake_receipts.rs`
5. `quickchain_preflight_paid_access.rs`
6. `quickchain_preflight_transport_authority.rs`
7. `quickchain_tooling_boundary.rs`

These tests are intentionally not a QuickChain implementation. They are a safety fence around the crate.

## `svc-gateway` Test Coverage Meaning

### Boundary Tests

The boundary tests verify that `svc-gateway` stays in its proper role.

They guard against:

* Importing direct ledger mutation authority.
* Depending on future QuickChain runtime crates.
* Publishing public QuickChain authority routes.
* Accidentally exposing root/checkpoint/validator/bridge behavior.
* Treating gateway as a chain runtime.
* Treating gateway as wallet or ledger truth.

### Docs Tests

The docs tests verify that the crate’s own documentation names the correct role and forbidden roles.

They make sure the docs preserve:

* Gateway is a public transport/proxy boundary.
* Gateway is not ledger truth.
* Gateway is not wallet truth.
* Gateway is not chain runtime.
* Gateway is not validator.
* Gateway is not bridge.
* Gateway is not settlement.
* Operation identity rules remain correct.
* Idempotency key remains retry-only.

This matters because future contributors may see “QuickChain” and accidentally begin wiring root/checkpoint/bridge authority into the gateway. The docs gate is there to prevent that drift.

### No Fake Receipts Tests

The no-fake-receipts tests guard against any future shortcut such as:

* Constructing a “receipt” locally.
* Returning a dev/demo receipt as if it were backend truth.
* Returning fake finality.
* Inventing balances.
* Inventing paid access.
* Constructing `root`, `checkpoint`, `validator`, or settlement-looking authority fields in production paths.

This is one of the most important safety gates because gateway code often becomes tempting glue code. The test ensures glue code does not become economic truth.

### Paid Access Tests

The paid-access tests verify that gateway paid paths stay in the correct prepare/quote/proxy role.

The required invariant is:

`prepare/quote -> explicit user confirmation -> backend wallet path -> backend receipt -> unlock/render`

`svc-gateway` may participate in request routing, but it must not silently spend, silently unlock, or locally authorize paid access.

These tests protect against:

* Paid unlock bypass shortcuts.
* Hardcoded “paid: true” behavior.
* Cache-only unlock.
* Header-only unlock.
* Client-provided receipt acceptance without backend validation.
* Gateway-created receipt truth.

### Cache Boundary Tests

The cache boundary tests verify that local/client/cache/materialized responses do not become paid entitlement authority.

Important invariants:

* Cache is convenience only.
* Cached bytes do not prove payment.
* b3 proves bytes, not economic authorization.
* Manifest hydration does not prove payment.
* Local receipt cache does not prove payment without backend validation.
* Offline cache can improve display and performance but cannot unlock paid content by itself.

### Transport Authority Tests

The transport authority tests verify that public request headers do not become economic authority.

They guard against unsafe use of headers such as:

* fake `x-ron-*` authority claims
* fake QuickChain finality headers
* fake receipt headers
* fake paid unlock headers
* fake root/checkpoint headers
* fake wallet/balance headers

The gateway may forward carefully allowlisted safe context, such as correlation or idempotency context, but it must strip or refuse authority-looking headers from clients.

### Tooling Boundary Tests

The tooling tests verify the crate has the required docs and scripts, uses the dynamic test discovery flow, and avoids Python helper drift under the crate.

This matches the current project preference: Bash-only crate-local QuickChain gate scripts, no Python patch/helper drift.

## `svc-gateway` Runtime/Product Regressions Preserved

The QuickChain hardening did not replace or bypass existing product behavior. The gateway’s existing public/proxy product behavior remains covered by regressions.

Important preserved behavior:

* Gateway still proxies product traffic.
* Gateway still routes to Omnigate as the product/hydration coordinator.
* Gateway still respects configured upstreams.
* Gateway still fails closed on unsupported paths.
* Gateway still keeps transport-level behavior separate from economic truth.
* Gateway still avoids local wallet/ledger mutation.

## What Changed Conceptually in `svc-gateway`

Before this slice, the gateway was functionally acting as the public edge, but the QuickChain boundary had not been fully made executable as tests/scripts/docs.

After this slice, the gateway has an explicit “do not cross this line” QuickChain safety fence.

The crate is now protected against several dangerous classes of future regression:

1. **Authority creep**
   Gateway cannot quietly grow into a chain runtime.

2. **Receipt fabrication**
   Gateway cannot invent receipts or paid unlock proof.

3. **Header smuggling**
   Client-provided authority-looking headers cannot become truth.

4. **Cache confusion**
   Cached bytes or manifests cannot become entitlement proof.

5. **Settlement creep**
   Root/checkpoint/validator/bridge code remains parked.

6. **Testing drift**
   New QuickChain tests are automatically discovered by the crate preflight script.

## Current `svc-gateway` Completion

For the current QuickChain Phase 0/preflight objective, `svc-gateway` should be treated as complete and parked.

Approximate crate status:

* Current Phase 0/preflight slice: 100%
* Full future QuickChain runtime role: intentionally 0%, because gateway should not become that role
* Future integration role: pending later client/backend proof work, not root/checkpoint/validator work

## What Remains for `svc-gateway` Later

Later work should only happen after the upstream QuickChain phases authorize it.

Allowed later work:

* More route-level negative tests if new gateway routes are added.
* More denylist/allowlist coverage for authority-looking headers.
* Better typed error envelopes for rejected authority headers.
* More integration tests proving paid access only unlocks after backend receipt validation.
* Gateway/Tauri end-to-end proof that display-only cached receipts cannot unlock paid content alone.
* Gateway observability for rejected authority-smuggling attempts.
* Gateway docs alignment when `svc-index`, `ron-policy`, and CrabLink Tauri adapters are hardened.

Still forbidden in `svc-gateway`:

* No roots.
* No checkpoints.
* No validators.
* No bridges.
* No external settlement.
* No staking.
* No liquidity.
* No public chain state.
* No ledger mutation.
* No wallet mutation.
* No fake receipts.
* No fake balances.
* No client cache unlock authority.

---

# `omnigate` QuickChain Phase 0 Notes

## Current Status

`omnigate` is now parked and green for the current QuickChain Phase 0/preflight slice.

The terminal output proves the full Omnigate gate passed:

* `cargo fmt -p omnigate`
* `cargo test -p omnigate --test quickchain_preflight_docs`
* `crates/omnigate/scripts/dev-quickchain-preflight.sh`
* `crates/omnigate/scripts/dev-quickchain-park.sh`
* all seven focused QuickChain test suites passed
* paid/product route regressions passed
* all-target tests passed
* clippy passed with warnings denied
* final parking marker printed

The terminal output ended with:

`omnigate QuickChain preflight gate passed`

`== omnigate quickchain exhaustive preflight gate passed: tests=7 ==`

`== omnigate QuickChain parking gate passed ==`

The crate codebundle was regenerated after the successful parking run.

## Role of `omnigate`

`omnigate` is the product-facing hydration and coordination layer. It composes product flows, resolves/hydrates content views, coordinates paid storage/product interactions, and fronts certain app/page/product routes.

Its role is:

* Hydrator.
* Coordinator.
* Product route layer.
* Manifest/page composition layer.
* Backend-service coordinator.
* Explicit paid-flow coordinator.
* Fail-closed public API surface.

Its role is **not**:

* Ledger.
* Wallet.
* Chain runtime.
* QuickChain root producer.
* Checkpoint producer.
* Validator.
* Bridge.
* External settlement service.
* Balance authority.
* Receipt authority by itself.
* Entitlement authority from local cache.
* Authority source from request headers.

## Main Architecture Preserved

The Omnigate paid product flow remains aligned with the internal ROC economy:

1. Product route prepares or quotes.
2. User explicitly confirms payment/hold/spend through the wallet path.
3. `svc-wallet` talks to ledger truth.
4. Backend receipt comes back from wallet/ledger path.
5. Omnigate uses backend truth to unlock/render/coordinate.
6. Any cached/display receipt remains display-only unless validated against backend truth.

Omnigate can coordinate the flow, but it cannot become the source of economic truth.

## Docs Added / Hardened

A QuickChain preflight document was added/hardened for `omnigate`.

The docs now explicitly state the crate’s role and forbidden boundaries:

* Omnigate is hydration and product coordination boundary.
* Omnigate is not a chain runtime.
* Omnigate is not a validator.
* Omnigate is not a bridge.
* Omnigate is not a checkpoint/root producer.
* Omnigate is not a wallet.
* Omnigate is not a ledger.
* Omnigate does not mutate ROC balances.
* Omnigate does not create economic truth.
* Omnigate does not fabricate receipts.
* Omnigate does not treat transport headers as authority.
* Omnigate does not treat cache as entitlement proof.
* Omnigate does not treat b3 hashes as payment proof.
* Omnigate does not treat manifests as payment proof.
* Omnigate does not treat local receipt cache as backend truth.

The final docs fix added/ensured exact scanner phrases for cache/content boundaries, including the essential idea:

* cache is convenience only
* cache cannot unlock paid content
* b3 hashes are content truth, not economic truth
* b3 proves bytes
* b3 does not prove payment
* manifest hydration does not prove payment
* local receipt cache does not prove payment without backend validation
* backend wallet/ledger receipt validation remains required for paid unlock

This fixed the earlier docs scanner failure and completed the crate gate.

## Scripts Added / Hardened

Two crate-local QuickChain scripts now define the Omnigate parking workflow.

### `crates/omnigate/scripts/dev-quickchain-preflight.sh`

This is the exhaustive Omnigate QuickChain gate.

It performs:

* Required docs check.
* Required tests directory check.
* Python helper drift rejection.
* `cargo fmt -p omnigate -- --check`.
* Dynamic discovery of `quickchain*.rs` tests.
* Execution of all focused QuickChain suites.
* Execution of paid/product route regressions.
* `cargo test -p omnigate --all-targets`.
* `cargo clippy -p omnigate --all-targets --no-deps -- -D warnings`.
* Final forbidden-scope parked message.

The script names the known focused suites for reviewability while still discovering tests dynamically. This is the right balance: humans can see the intended gate, and the script still catches new `quickchain*.rs` test files automatically.

### `crates/omnigate/scripts/dev-quickchain-park.sh`

This is the parking wrapper.

It verifies that the required docs/script/tooling files exist, delegates to the preflight gate, and prints:

`== omnigate QuickChain parking gate passed ==`

This makes Omnigate’s parked status explicit and repeatable.

## Tests Added / Hardened

Omnigate now has seven focused QuickChain preflight test suites:

1. `quickchain_preflight_boundary.rs`
2. `quickchain_preflight_cache_boundary.rs`
3. `quickchain_preflight_docs.rs`
4. `quickchain_preflight_no_fake_receipts.rs`
5. `quickchain_preflight_paid_access.rs`
6. `quickchain_preflight_transport_authority.rs`
7. `quickchain_tooling_boundary.rs`

These test suites are intentionally defensive. They do not implement QuickChain. They prevent Omnigate from accidentally implementing the wrong parts too early.

## Omnigate Test Coverage Meaning

### `quickchain_preflight_boundary.rs`

This suite proves Omnigate remains a hydrator/coordinator and not a chain runtime.

It verifies:

* Route docs keep Omnigate as hydrator/coordinator.
* Omnigate Cargo config does not depend on `ron-ledger` or future QuickChain runtime crates.
* Wallet surface remains display balance and explicit hold front door only.
* Public v1 router does not publish QuickChain authority routes.
* Omnigate source does not import direct ledger or chain authority.

This prevents accidental dependency/route creep.

### `quickchain_preflight_cache_boundary.rs`

This suite proves that cache/content addressing cannot become paid entitlement authority.

It verifies:

* Docs state cache and b3 are not paid entitlement authority.
* Stream and chat sources fail closed for cached paid media.
* Paid-access routes do not parse transport cache headers as unlock authority.
* Production source does not claim cache, b3, or manifest proves payment.

This is critical for CrabLink/Tauri offline cache design. Offline/local cache may improve UX, but it must not unlock paid content alone.

### `quickchain_preflight_docs.rs`

This suite enforces the crate docs as part of the architecture gate.

It verifies:

* Docs state Omnigate hydration/product coordination boundary.
* Docs name forbidden authority roles.
* Docs preserve operation identity rules.
* Docs define cache/content addressing as non-economic truth.
* Preflight script lists focused tests and clippy gate.

This converts architectural doctrine into testable crate-local requirements.

### `quickchain_preflight_no_fake_receipts.rs`

This suite guards against fake economic truth.

It verifies:

* Wallet balance fallback is explicitly non-authoritative.
* Paid routes use wallet receipt values without dev receipt fabrication.
* Production source does not construct root or validator authority fields.
* Production source does not construct obvious fake receipts or finality.

This is a major safety win. Omnigate coordinates visible product behavior, so it is one of the easiest places for fake receipts or fake paid flags to creep in. This suite prevents that.

### `quickchain_preflight_paid_access.rs`

This suite guards paid route behavior.

It verifies:

* Paid storage route stays prepare/estimate/proxy only.
* Content view and site visit payment routes name the wallet front-door invariant.
* Stream and chat paid paths fail closed on backend truth.
* Production source does not contain paid/unlock bypass shortcuts.

This preserves the core paid flow:

`prepare/quote -> explicit confirmation -> backend wallet path -> backend receipt -> unlock/render`

### `quickchain_preflight_transport_authority.rs`

This suite prevents authority-smuggling through HTTP headers.

It verifies:

* Docs state transport headers are not economic authority.
* Proxy-like routes filter `x-ron-*` headers before downstream forwarding.
* Route sources do not accept fake `x-ron` authority headers.
* Receipt/paid authority must come from backend paths, not request headers.
* Production `x-ron` headers do not define QuickChain authority tokens.

This protects against client-side spoofing of receipt/payment/finality.

### `quickchain_tooling_boundary.rs`

This suite verifies the crate-local QuickChain tooling is present and disciplined.

It verifies:

* Docs and runner files exist.
* Preflight script dynamically discovers QuickChain tests and runs hard gates.
* Preflight script lists known boundary suites for reviewability.
* Park script delegates to preflight and prints parking marker.
* No Python helper drift exists under the crate.

This aligns with the current project patch/test preference: Bash scripts, deterministic crate gates, no extra Python helper drift.

## Product / Paid Route Regressions Preserved

The Omnigate preflight script also ran paid/product route regressions after the focused QuickChain tests.

The following route regressions passed:

* `content_view`
* `site_visit`
* `streams`
* `chat_routes`
* `paid_storage_estimate_proxy`
* `paid_storage_prepare`
* `paid_storage_write_proxy`

Important preserved behavior:

### Content View

* Quote returns manifest recipient and does not call wallet.
* Pay recovers wallet nonce and returns wallet receipt.
* Quote rejects recipient mismatch.

This preserves separation between quote and pay.

### Site Visit

* Quote returns expected ROC amount and manifest payout recipient.
* Pay recovers wallet nonce and returns wallet receipt.
* Quote rejects recipient mismatch.

This preserves paid site visit semantics without Omnigate becoming wallet truth.

### Streams

* Latest segment requires stream-bound wallet receipt.

This protects stream paid access from cache/header bypass.

### Chat Routes

* Chat create/resolve/quote works.
* Paid send requires quote.
* Free chat send/list/latest works.
* Moderation fails closed.

This preserves chat product behavior while preventing paid send shortcutting.

### Paid Storage Estimate Proxy

* Happy path passes.
* Storage errors pass through.
* Upstream connect failure yields problem 502.

This keeps Omnigate as proxy/coordinator, not storage authority.

### Paid Storage Prepare

* Returns hold template from storage estimate.
* Maps storage estimate errors.
* Rejects bad JSON without storage call.
* Upstream connect failure yields problem 502.

This preserves prepare-only behavior.

### Paid Storage Write Proxy

* PUT and POST happy paths pass.
* Storage errors pass through.
* Upstream connect failure yields problem 502.

This keeps paid storage enforcement owned by storage/wallet/ledger path, not Omnigate-local shortcut logic.

## All-Targets Regression Coverage Preserved

The Omnigate parking gate also ran all-target tests and clippy.

The all-target run covered broad product and infrastructure behavior, including:

* App/builtin page routes.
* Asset page resolver.
* Named site crab resolver.
* Article asset publish.
* Comment asset publish.
* Image asset prepare.
* Text asset publish.
* Site launch.
* Passport profile routes.
* Policy gate.
* Policy metrics.
* Middleware contract.
* Admission contract.
* Readiness behavior.
* Metrics contract.
* ZK receipt/read-only placeholders.
* Product route regressions.
* QuickChain preflight suites.
* Benches with zero-test harnesses where applicable.

The successful all-target run is important because the QuickChain hardening did not break the existing Omnigate product surface.

## Clippy Gate Preserved

`cargo clippy -p omnigate --all-targets --no-deps -- -D warnings` passed.

This is important because the preflight gate is not just a test gate. It also enforces warning-free compile hygiene for the crate’s targets.

## What Changed Conceptually in `omnigate`

Before this slice, Omnigate already coordinated product flows, but the QuickChain boundaries were not fully executable as local docs/scripts/tests.

After this slice, Omnigate now has a hard preflight fence around the most important future-risk boundaries:

1. **Hydration boundary**
   Omnigate hydrates and coordinates; it does not become a chain runtime.

2. **Wallet/ledger boundary**
   Omnigate can call/proxy wallet-facing flows but does not mutate ledger truth directly.

3. **Receipt boundary**
   Omnigate does not fabricate receipts, roots, finality, or balances.

4. **Paid-access boundary**
   Paid unlock requires backend truth, not cache, not headers, not local display state.

5. **Cache/content boundary**
   b3 and manifests prove content identity, not payment.

6. **Transport boundary**
   Client headers cannot smuggle QuickChain or wallet authority.

7. **Tooling boundary**
   Omnigate now has a repeatable parking script and dynamically discovered focused tests.

## Current Omnigate Completion

For the current QuickChain Phase 0/preflight objective, `omnigate` should be treated as complete and parked.

Approximate crate status:

* Current Phase 0/preflight slice: 100%
* Full future QuickChain runtime role: intentionally 0%, because Omnigate should not become a runtime
* Future integration role: pending later backend receipt/root/proof phases, but not authorized yet

## What Remains for `omnigate` Later

Later work should be strictly scoped and should not cross into root/checkpoint/validator/bridge implementation until the proper upstream QuickChain gates are green.

Allowed later work:

* Add more paid route negative tests as new product routes are introduced.
* Add more tests that prove display-only receipt cache cannot unlock paid content alone.
* Add deeper integration tests against `svc-wallet` once backend receipt validation is formalized.
* Add typed receipt-display DTOs once `ron-proto` economic DTOs are fully locked for the relevant phase.
* Add product-route observability around rejected fake paid headers.
* Add CrabLink/Tauri end-to-end proof that Omnigate remains backend-truth-driven.
* Add bounded media/range tests where paid media rendering needs stronger proof.
* Add route-specific policy checks after `ron-policy` QuickChain preflight is parked.
* Add index lookup non-authority checks after `svc-index` QuickChain preflight is parked.

Still forbidden in `omnigate`:

* No root production.
* No checkpoint production.
* No validator logic.
* No bridge logic.
* No external settlement.
* No staking.
* No liquidity.
* No public chain state.
* No ledger mutation.
* No direct wallet mutation.
* No raw engagement payout allocation.
* No fake receipts.
* No fake balances.
* No local-cache paid unlock.
* No transport-header authority.

---

# Combined Result of This Crate Pair

This crate pair is especially important because `svc-gateway` and `omnigate` sit between clients and the backend value plane.

Together, they now enforce a clean split:

## `svc-gateway`

Public boundary / proxy edge.

It answers:

* Is this request shaped safely enough to forward?
* Which backend/product surface should receive it?
* Which headers/context are safe?
* Which authority-like claims must be stripped or refused?

It does **not** answer:

* Did the user pay?
* What is the true balance?
* Is this receipt final?
* What is the state root?
* Is this checkpoint valid?

## `omnigate`

Hydration / product coordination layer.

It answers:

* How should a product route be prepared?
* Which storage/index/wallet-facing backend paths are involved?
* How should a product page/content view be hydrated?
* How should paid flows coordinate with backend wallet receipt paths?

It does **not** answer by itself:

* Did ledger mutate?
* Is the balance true?
* Is the receipt final?
* Is this QuickChain state root valid?
* Is this checkpoint valid?
* Has external settlement occurred?

## Backend Truth Remains Elsewhere

Truth remains:

* `ron-ledger` for economic truth.
* `svc-wallet` as the mutation front door.
* `svc-storage` for paid byte enforcement.
* `ron-accounting` for snapshots, not balance truth.
* `svc-rewarder` for payout planning, not mutation.
* future QuickChain for settlement/proof infrastructure only after gates allow it.

---

# Why This Session Matters

This session is a major architectural hardening step because it prevents the two most exposed product-facing crates from becoming accidental authorities.

The most dangerous future failure modes were addressed directly:

* A client sends a fake paid header.
* A client sends a fake receipt header.
* A client relies on cached bytes to unlock paid content.
* A route returns a fake receipt for demo convenience.
* A gateway route accidentally exposes `/quickchain/root`.
* Omnigate starts constructing validator/finality/root-looking fields.
* b3 hashes are mistaken for payment proof.
* manifest hydration is mistaken for entitlement proof.
* local receipt cache is mistaken for backend validation.
* idempotency key is mistaken for operation authority.
* public routing is mistaken for ledger mutation authority.

The new docs/tests/scripts turn those risks into repeatable gates.

---

# Current QuickChain Progress Impact

With `svc-gateway` and `omnigate` parked, the major public/product boundary pair is now protected.

Current completed/parked QuickChain Phase 0 slices include:

* `ron-proto` DTO foundation work
* `ron-ledger` atomic execution / snapshot / replay preflight work
* `svc-wallet` preflight boundary work
* `ron-accounting` snapshot/accounting boundary work
* `svc-rewarder` payout-planning boundary work
* `svc-storage` paid-storage/content boundary work
* `svc-gateway` public gateway boundary work
* `omnigate` hydration/product coordination boundary work

The next coherent crate pair remains:

1. `svc-index`
2. `ron-policy`

This is the correct next pair because:

* `svc-index` must be locked down as lookup/pointer service only, not truth/authority.
* `ron-policy` must be locked down as declarative policy only, not wallet/ledger/rewarder/settlement authority.

---

# Next Crate Pair: `svc-index` + `ron-policy`

## `svc-index` Next Goals

For `svc-index`, the next QuickChain preflight slice should prove:

* Index is lookup/pointer service only.
* Names and pointers are not authority.
* Index does not mutate ledger.
* Index does not create wallet receipts.
* Index does not prove payment.
* Index does not unlock paid content.
* Index does not own b3 bytes.
* Index does not own balance truth.
* Index does not produce roots/checkpoints/validators/bridges.
* Index routes fail closed on authority-looking QuickChain paths.
* b3 hashes remain content identity only.
* crab:// names remain navigation/pointers only.
* Pointer resolution does not equal entitlement.

Expected additions:

* `docs/quickchain-preflight.md`
* `scripts/dev-quickchain-preflight.sh`
* `scripts/dev-quickchain-park.sh`
* focused `quickchain*.rs` tests
* product/index regressions in the preflight script
* no Python helper drift

## `ron-policy` Next Goals

For `ron-policy`, the next QuickChain preflight slice should prove:

* Policy is declarative policy only.
* Policy may classify/allow/deny intent.
* Policy does not mutate ledger.
* Policy does not mutate wallet.
* Policy does not allocate rewards.
* Policy does not mint ROC.
* Policy does not transfer ROC.
* Policy does not create receipts.
* Policy does not create roots/checkpoints/validators/bridges.
* Policy decisions are not economic truth by themselves.
* Policy cannot replace wallet/ledger receipt validation.
* Policy may label event classes later, but does not create balances.

Expected additions:

* `docs/quickchain-preflight.md`
* `scripts/dev-quickchain-preflight.sh`
* `scripts/dev-quickchain-park.sh`
* focused `quickchain*.rs` tests
* policy evaluator regressions in the preflight script
* no Python helper drift

---

# Commands Confirmed Useful

For codebundle regeneration, the working command format remains:

```bash
bash scripts/make_crate_codex.sh --force -c svc-gateway
bash scripts/make_crate_codex.sh --force -c omnigate
```

For the just-completed Omnigate gate, the successful command sequence was:

```bash
cargo fmt -p omnigate

cargo test -p omnigate --test quickchain_preflight_docs
crates/omnigate/scripts/dev-quickchain-preflight.sh
crates/omnigate/scripts/dev-quickchain-park.sh

bash scripts/make_crate_codex.sh --force -c omnigate
```

For future crate work, keep using Bash-only patches and crate-local preflight scripts.

---

# Final Parked State

`svc-gateway`: parked for current QuickChain Phase 0/preflight slice.

`omnigate`: parked for current QuickChain Phase 0/preflight slice.

Together, they now preserve the intended public/product boundary:

* Gateway routes and proxies.
* Omnigate hydrates and coordinates.
* Wallet/ledger own economic truth.
* Storage enforces paid bytes.
* Accounting snapshots but is not balance truth.
* Rewarder plans but does not mutate.
* Cache is convenience only.
* b3 is content truth only.
* crab:// is navigation only.
* QuickChain remains future settlement/proof infrastructure.
* No roots/checkpoints/validators/bridges/external settlement are implemented here.

This is a clean stopping point for the pair. The next session can start directly on **`svc-index` + `ron-policy`** with the same Phase 0 parking pattern.


### END NOTE - JUNE 17 2026 - 17:55 CST



### BEGIN NOTE - JUNE 19 2026 - 16:40 CST

The latest terminal output confirms both crates are **green + parked** with `tests=8` for each crate and final parking markers for both `svc-gateway` and `omnigate`. 

# QuickChain Phase 0 / QC-0A Notes — `svc-gateway` + `omnigate`

## Session summary

This session completed the QuickChain Phase 0 / QC-0A pass for the `svc-gateway` and `omnigate` pair.

The goal of this pass was not to add QuickChain runtime behavior. The goal was to harden the gateway/hydration boundary so that the paid-access value loop can safely pass through `svc-gateway` and `omnigate` without either crate becoming wallet truth, ledger truth, receipt truth, balance truth, finality truth, root authority, checkpoint authority, validator authority, bridge authority, or external settlement authority.

The new work locks the value-loop boundary:

```text
client intent
  -> svc-gateway public boundary
  -> omnigate quote/access/hydration coordination
  -> svc-wallet mutation front-door
  -> ron-ledger durable economic truth
  -> accepted backend receipt
  -> paid unlock/render using backend-derived truth
```

This pair is now green and parked for the current QuickChain Phase 0 pass.

---

# `svc-gateway` notes

## Crate role in QuickChain Phase 0

`svc-gateway` remains the public route/proxy/admission boundary.

For QuickChain Phase 0, `svc-gateway` is allowed to:

* receive public client intent;
* route/proxy product, paid-access, identity, storage, and app requests;
* preserve safe request context;
* forward allowed headers/body/query data;
* strip caller-supplied fake authority headers;
* proxy paid access workflows toward `omnigate`;
* expose backend-derived/display-only receipt context when appropriate;
* fail closed when unsupported chain, bridge, root, checkpoint, validator, or finality routes are attempted.

For QuickChain Phase 0, `svc-gateway` is explicitly not allowed to:

* mutate ROC balances;
* call `ron-ledger` directly as economic authority;
* act as wallet authority;
* fabricate receipts;
* fabricate balances;
* decide paid entitlement from cache, client headers, b3 hashes, crab URLs, manifests, or query strings;
* produce roots;
* produce checkpoints;
* act as a validator;
* expose bridge routes;
* expose external settlement routes;
* promote an accepted backend receipt into QuickChain finality;
* treat `accepted` as `epoch_included`, `finalized`, or `anchored`.

The key doctrine for this crate is:

```text
svc-gateway is a public boundary and proxy surface, not economic truth.
```

---

## What changed in `svc-gateway`

### 1. Added pair-level QuickChain value-loop boundary coverage

A new focused QuickChain test suite was added:

```text
crates/svc-gateway/tests/quickchain_preflight_value_loop_boundary.rs
```

This test suite verifies that the gateway documentation and preflight tooling explicitly lock the public gateway role into the full paid-access value loop without transferring economic authority into the gateway.

The test suite covers:

```text
docs_lock_public_to_product_value_loop_without_authority_transfer
docs_separate_current_accepted_receipt_from_future_quickchain_status
docs_keep_gateway_out_of_receipt_balance_and_finality_truth
preflight_runner_discovers_and_names_value_loop_suite
```

The purpose of this test suite is to ensure that future changes cannot quietly turn the gateway into a wallet, ledger, receipt, balance, root, checkpoint, validator, bridge, settlement, or finality authority.

---

### 2. Added documentation scanner phrases for the gateway value-loop boundary

The `svc-gateway` QuickChain preflight documentation was expanded with a pair-level value-loop section.

The section documents the intended route of authority:

```text
svc-gateway public route boundary
  -> omnigate product hydration/access coordination
  -> svc-wallet mutation front-door
  -> ron-ledger durable economic truth
```

It also documents the current paid-access path:

```text
client intent
  -> svc-gateway public boundary
  -> omnigate quote/access/hydration coordinator
  -> svc-wallet hold/transfer/capture/release/receipt path
  -> ron-ledger accepted receipt
  -> paid unlock/render using backend-derived truth
```

The documentation now explicitly states:

```text
gateway and omnigate may coordinate paid access, but neither is wallet, ledger, receipt, balance, root, checkpoint, validator, bridge, external settlement, or finality authority.
```

This phrase is important because it locks the paired boundary between the public ingress layer and the product hydration/access coordination layer.

---

### 3. Separated current accepted backend receipt from future QuickChain statuses

The gateway docs and tests now distinguish the current backend receipt state from future QuickChain settlement statuses.

The current system may use an accepted backend receipt for local paid unlock/render, but the gateway must not treat that receipt as future QuickChain finality.

The locked distinction is:

```text
accepted backend receipt can unlock local paid content.
accepted is not finalized.
accepted is not epoch_included.
accepted is not anchored.
```

The future statuses remain parked:

```text
accepted
epoch_included
finalized
anchored
```

This matters because the current internal ROC MVP uses backend wallet/ledger truth, while QuickChain settlement status remains future infrastructure.

The gateway must not blur those layers.

---

### 4. Locked gateway non-authority over receipt, balance, and finality truth

The gateway docs and tests now explicitly state:

```text
gateway is not receipt truth.
gateway is not balance truth.
gateway is not settlement finality.
gateway never converts accepted wallet/ledger receipt into QuickChain finality.
current paid unlock is backend-derived local access, not future QuickChain epoch inclusion.
```

This prevents future route handlers, headers, proxy metadata, or cache logic from turning the gateway into a source of economic truth.

---

### 5. Confirmed forbidden scope remains parked

The gateway preflight/park output confirms that forbidden QuickChain scope remains parked:

```text
no roots
no checkpoints
no validators
no bridges
no external settlement
no fake receipts
no fake balances
```

This is the correct Phase 0 state.

No root-producing code was introduced.

No checkpoint-producing code was introduced.

No validator code was introduced.

No bridge code was introduced.

No external settlement code was introduced.

No wallet or ledger mutation authority was moved into `svc-gateway`.

---

### 6. Confirmed dynamic QuickChain test discovery includes the new suite

The gateway preflight tooling now discovers the new value-loop boundary test as part of the `quickchain*.rs` dynamic suite.

The preflight runner still verifies:

```text
find "$TEST_DIR"
-name 'quickchain*.rs'
test -p "$PKG" --test "$test_name"
```

This means newly added `quickchain*.rs` test files are automatically picked up by the exhaustive preflight script.

The `quickchain_tooling_boundary` suite also confirms that the preflight and park scripts remain reviewable and that the park script delegates to preflight.

---

## `svc-gateway` tests and gates passed

The following focused test passed:

```text
cargo test -p svc-gateway --test quickchain_preflight_value_loop_boundary
```

The following exhaustive QuickChain preflight suites passed:

```text
quickchain_preflight_boundary
quickchain_preflight_cache_boundary
quickchain_preflight_docs
quickchain_preflight_no_fake_receipts
quickchain_preflight_paid_access
quickchain_preflight_transport_authority
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
```

The gateway product/proxy regressions passed, including:

```text
app_proxy
paid_storage_estimate_proxy
paid_storage_write_proxy
product_routes_proxy
site_visit_routes_proxy
```

The gateway all-targets pass completed successfully.

The gateway clippy gate completed successfully.

The final successful markers were:

```text
== svc-gateway quickchain exhaustive preflight gate passed: tests=8 ==
== svc-gateway QuickChain parking gate passed ==
```

---

## `svc-gateway` current status

`svc-gateway` is green and parked for this QuickChain Phase 0 pass.

Current status:

```text
svc-gateway QuickChain Phase 0 status: complete for current pair pass.
```

The crate remains a public boundary/proxy layer. It does not own QuickChain authority, wallet authority, ledger truth, receipt truth, balance truth, or finality truth.

---

## `svc-gateway` remaining future work

No additional Phase 0 work is needed in `svc-gateway` for this pair pass unless a later cross-crate umbrella runner discovers drift.

Future work belongs to later phases and should remain parked until the appropriate gates are green.

Future parked work includes:

* no root-producing implementation until canonical bytes and golden vectors are locked;
* no checkpoint-producing implementation until root semantics are finalized;
* no validator or committee behavior until later QuickChain phases;
* no bridge or external settlement behavior in the current internal ROC MVP;
* no gateway-side wallet or ledger mutation;
* no gateway-side paid unlock based on cache, b3, crab URL, manifest, or caller-supplied headers.

If future product routes are added, they must preserve the same rule:

```text
svc-gateway may route intent, but it must not become economic truth.
```

---

# `omnigate` notes

## Crate role in QuickChain Phase 0

`omnigate` remains the product hydration/access coordination boundary.

For QuickChain Phase 0, `omnigate` is allowed to:

* hydrate product pages;
* coordinate quote/access flows;
* coordinate paid content workflows through backend truth;
* call the proper backend services;
* use wallet/ledger-derived accepted receipts for local paid unlock/render;
* proxy or coordinate storage/index/policy/wallet paths as designed;
* fail closed when backend truth is unavailable;
* preserve the distinction between accepted backend receipt and future QuickChain finality.

For QuickChain Phase 0, `omnigate` is explicitly not allowed to:

* mutate ledger truth directly;
* become wallet authority;
* fabricate receipts;
* fabricate balances;
* fabricate finality;
* treat cache, b3, crab URL, manifest, stream segment, chat message, or client header as payment proof;
* generate QuickChain roots;
* generate checkpoints;
* act as a validator;
* expose bridge authority;
* expose external settlement authority;
* promote `accepted` into `epoch_included`, `finalized`, or `anchored`.

The key doctrine for this crate is:

```text
omnigate hydrates and coordinates product access, but it is not economic truth.
```

---

## What changed in `omnigate`

### 1. Added pair-level QuickChain value-loop boundary coverage

A new focused QuickChain test suite was added:

```text
crates/omnigate/tests/quickchain_preflight_value_loop_boundary.rs
```

This test suite verifies that `omnigate` remains a product hydration/access coordinator in the paid-access value loop and does not become wallet, ledger, receipt, balance, root, checkpoint, validator, bridge, settlement, or finality authority.

The test suite covers:

```text
docs_lock_gateway_to_omnigate_to_wallet_to_ledger_value_loop
docs_separate_accepted_receipt_from_future_finality
docs_keep_omnigate_out_of_receipt_balance_and_finality_truth
preflight_runner_discovers_and_names_value_loop_suite
```

The purpose of this test suite is to prevent future hydration or paid-access code from quietly converting `omnigate` into economic authority.

---

### 2. Added documentation scanner phrases for the omnigate value-loop boundary

The `omnigate` QuickChain preflight documentation was expanded with a pair-level value-loop section.

The section documents the intended authority path:

```text
svc-gateway public route boundary
  -> omnigate product hydration/access coordination
  -> svc-wallet mutation front-door
  -> ron-ledger durable economic truth
```

It also documents the product-side version of the boundary:

```text
omnigate product hydration/access coordination
  -> svc-wallet mutation front-door
  -> ron-ledger durable economic truth
```

The current paid-access path is now documented as:

```text
client intent
  -> svc-gateway public boundary
  -> omnigate quote/access/hydration coordinator
  -> svc-wallet hold/transfer/capture/release/receipt path
  -> ron-ledger accepted receipt
  -> paid unlock/render using backend-derived truth
```

This makes it clear that `omnigate` coordinates the product flow but does not own the underlying economic mutation or settlement truth.

---

### 3. Separated accepted backend receipt from future QuickChain finality

The `omnigate` docs and tests now explicitly distinguish current accepted backend receipts from future QuickChain settlement statuses.

The current accepted backend receipt can be used for local paid access, but it is not a QuickChain finality marker.

The locked distinction is:

```text
accepted backend receipt can unlock local paid content.
accepted is not finalized.
accepted is not epoch_included.
accepted is not anchored.
omnigate cannot promote accepted receipt to epoch_included, finalized, or anchored.
```

This is especially important in `omnigate` because product hydration/access flows are close to user-visible paid content. The test suite ensures that user-visible access does not become fake finality.

---

### 4. Locked omnigate non-authority over receipt, balance, and finality truth

The `omnigate` docs and tests now explicitly state:

```text
omnigate is not receipt truth.
omnigate is not balance truth.
omnigate is not settlement finality.
```

This ensures that any product route, paid route, storage route, stream route, chat route, site route, or asset route added later must keep wallet/ledger-derived truth separate from product hydration.

---

### 5. Confirmed forbidden scope remains parked

The omnigate preflight/park output confirms that forbidden QuickChain scope remains parked:

```text
no roots
no checkpoints
no validators
no bridges
no external settlement
no fake receipts
no fake balances
```

This is the correct Phase 0 state.

No root-producing code was introduced.

No checkpoint-producing code was introduced.

No validator code was introduced.

No bridge code was introduced.

No external settlement code was introduced.

No direct `ron-ledger` economic mutation authority was moved into `omnigate`.

---

### 6. Confirmed dynamic QuickChain test discovery includes the new suite

The omnigate preflight tooling now discovers the new value-loop boundary test as part of the `quickchain*.rs` dynamic suite.

The preflight runner still verifies:

```text
find "$TEST_DIR"
-name 'quickchain*.rs'
test -p "$PKG" --test "$test_name"
```

The `quickchain_tooling_boundary` suite confirms that the preflight and park scripts remain reviewable and that the park script delegates to preflight.

---

## `omnigate` tests and gates passed

The following focused test passed:

```text
cargo test -p omnigate --test quickchain_preflight_value_loop_boundary
```

The following exhaustive QuickChain preflight suites passed:

```text
quickchain_preflight_boundary
quickchain_preflight_cache_boundary
quickchain_preflight_docs
quickchain_preflight_no_fake_receipts
quickchain_preflight_paid_access
quickchain_preflight_transport_authority
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
```

The omnigate paid/product route regressions passed, including:

```text
content_view
site_visit
streams
chat_routes
paid_storage_estimate_proxy
paid_storage_prepare
paid_storage_write_proxy
```

The omnigate all-targets pass completed successfully.

The omnigate clippy gate completed successfully.

The final successful markers were:

```text
== omnigate quickchain exhaustive preflight gate passed: tests=8 ==
== omnigate QuickChain parking gate passed ==
```

---

## `omnigate` current status

`omnigate` is green and parked for this QuickChain Phase 0 pass.

Current status:

```text
omnigate QuickChain Phase 0 status: complete for current pair pass.
```

The crate remains a product hydration/access coordinator. It does not own QuickChain authority, wallet authority, ledger truth, receipt truth, balance truth, or finality truth.

---

## `omnigate` remaining future work

No additional Phase 0 work is needed in `omnigate` for this pair pass unless a later cross-crate umbrella runner discovers drift.

Future work belongs to later phases and should remain parked until the appropriate gates are green.

Future parked work includes:

* no root-producing implementation until canonical bytes and golden vectors are locked;
* no checkpoint-producing implementation until root semantics are finalized;
* no validator or committee behavior until later QuickChain phases;
* no bridge or external settlement behavior in the current internal ROC MVP;
* no direct ledger mutation from `omnigate`;
* no product unlock based on cache, b3, crab URL, manifest, stream segment, chat message, or caller-supplied headers;
* no conversion of accepted backend receipts into QuickChain finality.

If future product or paid-access routes are added, they must preserve the same rule:

```text
omnigate may coordinate access, but it must not become economic truth.
```

---

# Pair-level notes: `svc-gateway` + `omnigate`

## Why this pair mattered

This pair is one of the most important Phase 0 boundaries because it is the visible path between the CrabLink client and the backend economic system.

Users interact through CrabLink/Tauri or browser clients. Those clients reach the backend through `svc-gateway`. Product pages and paid access are hydrated/coordinated by `omnigate`. That means these two crates sit directly in the path where fake authority would be most tempting or most dangerous.

This pass ensures that:

```text
client-visible access does not equal economic truth.
gateway routing does not equal economic truth.
omnigate hydration does not equal economic truth.
cache possession does not equal economic truth.
b3 possession does not equal economic truth.
crab:// navigation does not equal economic truth.
accepted backend receipt does not equal QuickChain finality.
```

The real economic path remains:

```text
svc-wallet -> ron-ledger
```

`svc-wallet` remains the mutation front-door.

`ron-ledger` remains durable economic truth.

---

## Boundary now locked by this pair

The pair now explicitly locks the following invariant:

```text
gateway and omnigate may coordinate paid access, but neither is wallet, ledger, receipt, balance, root, checkpoint, validator, bridge, external settlement, or finality authority.
```

This is the key pair-level invariant from the session.

It protects the internal ROC MVP from accidentally drifting into premature QuickChain settlement behavior.

---

## Accepted receipt versus future QuickChain settlement status

This session also locked the distinction between current backend-paid access and future QuickChain settlement states.

Current state:

```text
accepted backend receipt
```

Future parked states:

```text
epoch_included
finalized
anchored
```

Meaning:

```text
accepted can unlock local paid content when it comes from backend wallet/ledger truth.
accepted cannot be treated as finalized.
accepted cannot be treated as epoch_included.
accepted cannot be treated as anchored.
gateway cannot promote accepted to finality.
omnigate cannot promote accepted to finality.
```

This distinction should remain throughout the rest of Phase 0.

---

## What did not change

This session did not add QuickChain runtime behavior.

Specifically, this session did not add:

```text
roots
checkpoints
validators
bridges
external settlement
staking
liquidity
exchange-facing logic
public chain authority
gateway ledger mutation
omnigate ledger mutation
fake receipts
fake balances
cache-only paid unlocks
client-header paid unlocks
b3-as-payment-proof behavior
crab-url-as-payment-proof behavior
manifest-as-payment-proof behavior
```

This is intentional and correct for Phase 0.

---

## Final test/park status

Final status for this pair:

```text
svc-gateway: green + parked
omnigate:    green + parked
```

Both crates passed:

```text
focused value-loop tests
dynamic QuickChain preflight discovery
all QuickChain boundary suites
product/paid route regressions
all-targets tests
clippy
park scripts
```

Each crate now reports:

```text
quickchain exhaustive preflight gate passed: tests=8
QuickChain parking gate passed
```

---

# Low-disk workflow note

During this session, both preflight and park scripts were run for the same crates. This duplicated the heavy exhaustive gates because the park scripts already delegate to the preflight scripts internally.

Going forward, avoid running both preflight and park for the same crate unless intentionally requesting two full passes.

Preferred low-disk flow after a focused patch is:

```text
cargo fmt -p <crate-a>
cargo fmt -p <crate-b>

cargo test -p <crate-a> --test <focused_new_test>
cargo test -p <crate-b> --test <focused_new_test>

crates/<crate-a>/scripts/dev-quickchain-park.sh
crates/<crate-b>/scripts/dev-quickchain-park.sh
```

For this pair, the preferred low-disk flow would have been:

```text
cargo fmt -p svc-gateway
cargo fmt -p omnigate

cargo test -p svc-gateway --test quickchain_preflight_value_loop_boundary
cargo test -p omnigate --test quickchain_preflight_value_loop_boundary

crates/svc-gateway/scripts/dev-quickchain-park.sh
crates/omnigate/scripts/dev-quickchain-park.sh
```

Do not also run:

```text
crates/svc-gateway/scripts/dev-quickchain-preflight.sh
crates/omnigate/scripts/dev-quickchain-preflight.sh
```

unless a deliberate duplicate full preflight pass is needed.

---

# Recommended next step

The next planned QuickChain Phase 0 pair is:

```text
svc-index + ron-policy
```

After that, the remaining Phase 0 pair is:

```text
CrabLink Tauri + client adapters
```

Once those are complete, the crate-by-crate Phase 0 sweep should be functionally complete. A final light wrap pass should then confirm:

```text
all touched crates have preflight/park markers;
forbidden scope remains parked everywhere;
no roots/checkpoints/validators/bridges/external settlement were introduced;
vector status is clearly labeled sketch / locked_bytes / locked_hash;
low-disk readiness workflow is documented;
Phase 0 carryover notes are complete before Phase 1 planning.
```

Only after that should Phase 1 root/proof planning begin.

---

# Current QuickChain completion estimate after this pair

After `svc-gateway + omnigate` are green and parked:

```text
QuickChain Phase 0 / QC-0A safety-preflight completion: approximately 70–75%.
Full QuickChain multi-phase vision completion: approximately 25–30%.
```

After `svc-index + ron-policy` and `CrabLink Tauri + client adapters` are completed and parked, Phase 0 should be close to:

```text
90–95% complete
```

The remaining 5–10% would be the final wrap/readiness pass rather than another major crate pair.


### END NOTE - JUNE 19 2026 - 16:40 CST


### BEGIN NOTE - JUNE 21 2026 - 12:55 CST

Both crates are now green/parked for this QuickChain service-boundary sweep. `svc-gateway` ended with the forbidden-scope marker, preflight passed, `tests=8`, and the parking marker printed.  The gateway run also proved the focused cache/docs/no-fake-receipts/paid-access suites and product/proxy regressions stayed green.   Omnigate is also parked: the latest run added/verified the 6-test transport-authority suite, kept cache/docs/no-fake-receipts/paid-access/value-loop/tooling suites green, passed clippy, and printed the `tests=8` parking marker.  

Paste the following into your crate notes, either as a shared note in both crates or split by section.

# QuickChain Phase 1 Foundation Notes — `svc-gateway` + `omnigate`

## BEGIN NOTE - JUNE 21 2026 - `svc-gateway` + `omnigate` QuickChain Phase 1 Foundation / Service-Boundary Parking Notes

## 0. Executive summary

The `svc-gateway` + `omnigate` crate pair is now green and parked for the current QuickChain Phase 1 foundation / QC-1A service-boundary sweep.

This was not root-producing implementation.

This was not validator implementation.

This was not bridge, staking, liquidity, ROX, Solana, external-settlement, or public-chain implementation.

The purpose of this sweep was to harden the public/product access path so the gateway and hydrator layers cannot quietly become:

```text
wallet truth
ledger truth
receipt truth
balance truth
root authority
checkpoint authority
validator authority
bridge authority
external settlement authority
cache entitlement authority
finality authority
```

The public/product path is now explicitly constrained as:

```text
client intent
-> svc-gateway public boundary / route proxy / admission surface
-> omnigate quote-access-hydration coordinator
-> svc-wallet economic mutation front-door
-> ron-ledger durable economic truth
-> backend-derived accepted receipt
-> paid unlock/render using backend truth
```

Current status:

```text
svc-gateway: GREEN / parked
omnigate:    GREEN / parked
pair status: GREEN / parked for this QuickChain service-boundary slice
```

Approximate status:

```text
svc-gateway current QuickChain Phase 1 foundation boundary slice: 100% parked
omnigate current QuickChain Phase 1 foundation boundary slice:    100% parked
full future QuickChain runtime role for either crate:              intentionally 0%
pair-level Phase 1 service-boundary readiness:                    parked
```

Interpretation:

```text
The crates are complete for the current service-boundary hardening pass.
They are not supposed to become future root producers or validators.
Later work may add proof/status display or integration tests, but ledger/root/finality truth must remain outside these crates.
```

---

# 1. Master doctrine preserved

QuickChain remains future settlement infrastructure.

The active doctrine remains:

```text
determinism before distribution
DTOs before roots
roots before validators
proofs before pruning
internal ROC before external anchors
```

The current RustyOnions authority model remains:

```text
ron-proto = DTOs and wire shapes
ron-ledger = durable economic truth
svc-wallet = economic mutation front-door
ron-accounting = snapshots and measurement, not balance truth
svc-rewarder = payout planning, not mutation
svc-storage = bytes by b3, not economic truth
svc-gateway = public route/proxy/admission boundary
omnigate = product hydration/access/quote coordinator
ron-policy = declarative policy, not mutation
svc-index = lookup/pointer service, not entitlement proof
CrabLink = display and user intent only
```

Still forbidden in `svc-gateway` and `omnigate`:

```text
no roots
no checkpoints
no validators
no bridges
no anchors
no external settlement
no staking
no liquidity
no ROX
no Solana
no exchange-facing logic
no public chain state
no direct ledger mutation
no direct wallet mutation
no fake balances
no fake receipts
no fake finality
no cache-only paid unlock
no client-header paid unlock
no b3-as-payment-proof
no manifest-as-payment-proof
```

---

# 2. Pair-level result

The pair now proves the current public/product value path stays inside the intended boundary.

Correct value loop:

```text
prepare / quote
-> explicit user confirmation
-> backend wallet path
-> backend accepted receipt / access authorization
-> gateway / omnigate enforce paid access from backend truth
-> storage serves bytes by b3 / range
-> client renders / unlocks
-> display-only receipt cache
-> balance refresh from backend wallet / ledger truth
```

Incorrect loops explicitly blocked:

```text
client says paid=true -> unlock
client sends x-ron-paid -> unlock
client sends x-ron-receipt -> unlock
client sends x-ron-finalized -> treat as finality
client sends x-ron-state-root -> treat as root proof
client sends x-ron-validator-* -> treat as validator proof
client sends x-ron-bridge-* -> treat as settlement proof
cache hit -> unlock paid content
b3 verifies bytes -> unlock paid content
manifest exists -> unlock paid content
hydrated view -> creates receipt truth
gateway proxy response -> creates balance truth
```

The pair-level success is important because these crates sit directly on the customer/product path. If either crate became economic authority, QuickChain would inherit a broken trust model before root/proof work even begins.

---

# 3. `svc-gateway` notes

## 3.1 Current `svc-gateway` status

`svc-gateway` is green and parked for this QuickChain boundary sweep.

Final observed gateway parking marker:

```text
svc-gateway QuickChain preflight gate passed
== svc-gateway quickchain exhaustive preflight gate passed: tests=8 ==
== svc-gateway QuickChain parking gate passed ==
```

Final forbidden-scope marker:

```text
[svc-gateway quickchain] forbidden scope remains parked:
no roots, no checkpoints, no validators, no bridges,
no external settlement, no fake receipts, no fake balances
```

## 3.2 `svc-gateway` role

`svc-gateway` is:

```text
public route boundary
route proxy
admission surface
public product path front door
header/body/query forwarding policy point
fail-closed edge boundary
```

`svc-gateway` is not:

```text
wallet
ledger
receipt issuer
balance authority
chain runtime
root producer
checkpoint writer
validator
bridge
external settlement service
finality source
cache entitlement authority
```

## 3.3 Gateway boundary hardened

The gateway now has an executable QuickChain safety fence around public route behavior.

The important guarantees are:

```text
gateway does not import wallet/ledger/QuickChain runtime authority
unsupported quickchain/ledger/bridge/root routes fail closed
wallet surface remains limited to proxy/hold/display-balance style behavior
product proxy strips caller-supplied QuickChain authority headers
cache/client unlock signals are stripped when they claim authority
request headers cannot smuggle economic or chain authority
response headers cannot smuggle economic or chain authority
backend receipt metadata is context, not gateway authority
client paid unlock headers are never gateway authority
accepted receipt remains distinct from future finalized/epoch_included/anchored status
```

## 3.4 Gateway focused QuickChain suites now green

Focused QuickChain suites proven green:

```text
quickchain_preflight_boundary: 4 passed
quickchain_preflight_cache_boundary: 3 passed
quickchain_preflight_docs: 5 passed
quickchain_preflight_no_fake_receipts: 3 passed
quickchain_preflight_paid_access: 3 passed
quickchain_preflight_transport_authority: 3 passed
quickchain_preflight_value_loop_boundary: 4 passed
quickchain_tooling_boundary: 5 passed
```

Important tests covered:

```text
route_source_keeps_wallet_surface_limited_to_proxy_hold_and_display_balance
gateway_source_does_not_import_wallet_ledger_or_quickchain_runtime_authority
unsupported_quickchain_ledger_bridge_root_routes_fail_closed_at_gateway
product_proxy_strips_caller_supplied_quickchain_authority_headers
cache_or_client_unlock_signals_are_stripped_when_they_claim_authority
cache_headers_are_transport_metadata_not_unlock_authority
route_source_does_not_unlock_paid_content_from_cache_or_query_claims
backend_receipt_metadata_remains_display_context_not_gateway_authority
request_headers_cannot_smuggle_economic_or_chain_authority
response_headers_cannot_smuggle_economic_or_chain_authority
client_paid_unlock_headers_are_never_gateway_authority
backend_receipt_metadata_is_context_not_gateway_paid_authority
route_source_does_not_define_cache_or_client_paid_unlock_shortcuts
hop_by_hop_and_host_headers_are_never_forwarded_or_copied
identity_and_retry_headers_are_context_not_authority
ordinary_cache_headers_remain_transport_metadata_only
docs_keep_gateway_out_of_receipt_balance_and_finality_truth
docs_lock_public_to_product_value_loop_without_authority_transfer
docs_separate_current_accepted_receipt_from_future_quickchain_status
```

## 3.5 Gateway product/proxy regressions stayed green

The gateway hardening did not break normal product proxy behavior.

Green product/proxy regression suites included:

```text
app_proxy: 5 passed
paid_storage_estimate_proxy: 3 passed
paid_storage_write_proxy: 4 passed
product_routes_proxy: 8 passed
site_visit_routes_proxy: 1 passed
```

Representative preserved behavior:

```text
app proxy preserves methods, body, query, auth, correlation headers, and generated request IDs
app proxy passes problem bodies unchanged
paid storage estimate/write proxies pass omnigate errors through
paid storage estimate/write proxies handle upstream connect failures as problem 502
image/site/text routes target omnigate
b3 asset proxy targets omnigate b3 route
chat product routes target omnigate
site resolve proxy targets omnigate site hydrator
paid prepare proxy preserves body and idempotency
crab resolve proxy preserves query and headers
site visit quote/pay proxies to omnigate and preserves context
```

## 3.6 Gateway all-targets and clippy green

The gateway parking gate also ran:

```text
cargo test -p svc-gateway --all-targets
cargo clippy -p svc-gateway --all-targets --no-deps -- -D warnings
```

Both completed successfully.

## 3.7 Gateway scripts/tooling hardened

Gateway crate-local QuickChain scripts now define the repeatable parking workflow:

```text
crates/svc-gateway/scripts/dev-quickchain-preflight.sh
crates/svc-gateway/scripts/dev-quickchain-park.sh
```

The preflight script behavior:

```text
checks docs exist
checks test directory exists
rejects checked-in Python helper drift
runs cargo fmt -p svc-gateway -- --check
dynamically discovers quickchain*.rs tests
runs every discovered focused QuickChain test
runs product/proxy regressions
runs cargo test -p svc-gateway --all-targets
runs cargo clippy -p svc-gateway --all-targets --no-deps -- -D warnings
prints forbidden-scope marker
prints dynamic test-count marker
```

The park script behavior:

```text
checks docs/script/tooling files exist
delegates to dev-quickchain-preflight.sh
prints == svc-gateway QuickChain parking gate passed ==
```

This gives the crate a low-drift, future-safe local gate.

## 3.8 Gateway files touched / relevant

Relevant gateway files for this slice:

```text
crates/svc-gateway/docs/quickchain-preflight.md
crates/svc-gateway/scripts/dev-quickchain-preflight.sh
crates/svc-gateway/scripts/dev-quickchain-park.sh
crates/svc-gateway/tests/quickchain_preflight_boundary.rs
crates/svc-gateway/tests/quickchain_preflight_cache_boundary.rs
crates/svc-gateway/tests/quickchain_preflight_docs.rs
crates/svc-gateway/tests/quickchain_preflight_no_fake_receipts.rs
crates/svc-gateway/tests/quickchain_preflight_paid_access.rs
crates/svc-gateway/tests/quickchain_preflight_transport_authority.rs
crates/svc-gateway/tests/quickchain_preflight_value_loop_boundary.rs
crates/svc-gateway/tests/quickchain_tooling_boundary.rs
```

Product/proxy regression files exercised:

```text
crates/svc-gateway/tests/app_proxy.rs
crates/svc-gateway/tests/paid_storage_estimate_proxy.rs
crates/svc-gateway/tests/paid_storage_write_proxy.rs
crates/svc-gateway/tests/product_routes_proxy.rs
crates/svc-gateway/tests/site_visit_routes_proxy.rs
crates/svc-gateway/tests/content_view_routes_proxy.rs
crates/svc-gateway/tests/identity_routes_proxy.rs
crates/svc-gateway/tests/smoke.rs
```

## 3.9 Gateway later work

Not required before moving on, but worth tracking later:

```text
object-route-specific proxy regression test
route inventory snapshot for forbidden authority routes
observability/redaction/cardinality boundary test
response body fake-claim scan
more route-level negative tests as new gateway routes are added
gateway/Tauri end-to-end proof that display-only cached receipts cannot unlock paid content alone
metrics for rejected authority-smuggling attempts
typed error envelopes for rejected authority headers
range/media proxy hardening
TLS/config/admission production polish
CI integration of focused QuickChain park gate
```

Still do not add to gateway:

```text
root creation
checkpoint writing
validator APIs
bridge settlement
external anchor settlement
staking
liquidity
direct wallet mutation
direct ledger mutation
gateway-owned receipt issuance
gateway-owned balance truth
cache-only paid unlock
```

---

# 4. `omnigate` notes

## 4.1 Current `omnigate` status

`omnigate` is green and parked for this QuickChain boundary sweep.

Final observed omnigate parking marker:

```text
omnigate QuickChain preflight gate passed
== omnigate quickchain exhaustive preflight gate passed: tests=8 ==
== omnigate QuickChain parking gate passed ==
```

Final forbidden-scope marker:

```text
[omnigate quickchain] forbidden scope remains parked:
no roots, no checkpoints, no validators, no bridges,
no external settlement, no fake receipts, no fake balances
```

The `tests=8` count reflects the added/shared transport-authority/header-policy suite in this latest pass.

## 4.2 `omnigate` role

`omnigate` is:

```text
product coordinator
hydrator
quote/access coordinator
manifest/page composition layer
backend service coordinator
paid-flow coordinator
fail-closed public API surface behind gateway
```

`omnigate` is not:

```text
wallet
ledger
balance authority
receipt authority by itself
chain runtime
root producer
checkpoint writer
validator
bridge
external settlement service
cache entitlement authority
transport-header authority
finality source
```

## 4.3 Omnigate shared header policy added

A shared route header policy was added for omnigate v1 product routes:

```text
crates/omnigate/src/routes/v1/header_policy.rs
```

The policy allows normal product context headers while rejecting caller-supplied authority-shaped headers.

Allowed product context examples:

```text
x-ron-wallet-account
x-ron-passport
x-ron-wallet-txid
x-ron-wallet-receipt-hash
x-ron-paid-op
x-ron-paid-asset
asset metadata headers
```

These are context for product coordination and backend verification. They are not finality, balance truth, ledger truth, root truth, checkpoint truth, validator authority, bridge authority, or external settlement authority.

Forbidden caller-supplied authority examples:

```text
x-ron-operation-id
x-ron-account-sequence
x-ron-receipt
x-ron-paid
x-ron-unlocked
x-ron-entitlement
x-ron-balance
x-ron-ledger
x-ron-root
x-ron-state-root
x-ron-receipt-root
x-ron-hold-root
x-ron-epoch-root
x-ron-checkpoint
x-ron-finalized
x-ron-finality
x-ron-epoch-included
x-ron-anchored
x-ron-settlement
x-ron-external-settlement
x-ron-validator
x-ron-bridge
x-ron-spend-authority
x-ron-capture-authority
x-ron-quickchain-*
x-ron-validator-*
x-ron-bridge-*
x-ron-checkpoint-*
x-ron-root-*
x-ron-proof-*
```

The important distinction:

```text
x-ron-wallet-receipt-hash may travel as backend/product verification context.
x-ron-state-root / x-ron-finalized / x-ron-validator-* / x-ron-bridge-* / x-ron-quickchain-* must not be forwarded or interpreted as authority.
```

## 4.4 Omnigate proxy helpers updated

The v1 product proxy helpers now use the shared QuickChain-aware header policy instead of directly wildcard-forwarding broad `x-ron-*` headers.

Updated route files:

```text
crates/omnigate/src/routes/v1/assets.rs
crates/omnigate/src/routes/v1/chat.rs
crates/omnigate/src/routes/v1/content_view.rs
crates/omnigate/src/routes/v1/paid.rs
crates/omnigate/src/routes/v1/profile.rs
crates/omnigate/src/routes/v1/site_visit.rs
crates/omnigate/src/routes/v1/sites.rs
crates/omnigate/src/routes/v1/text_assets.rs
```

Module registration:

```text
crates/omnigate/src/routes/v1/mod.rs
```

New/updated test:

```text
crates/omnigate/tests/quickchain_preflight_transport_authority.rs
```

Docs hardened:

```text
crates/omnigate/docs/quickchain-preflight.md
```

## 4.5 Omnigate focused QuickChain suites now green

Focused QuickChain suites proven green:

```text
quickchain_preflight_boundary: 5 passed
quickchain_preflight_cache_boundary: 4 passed
quickchain_preflight_docs: 5 passed
quickchain_preflight_no_fake_receipts: 4 passed
quickchain_preflight_paid_access: 4 passed
quickchain_preflight_transport_authority: 6 passed
quickchain_preflight_value_loop_boundary: 4 passed
quickchain_tooling_boundary: 5 passed
```

Important tests covered:

```text
omnigate_cargo_does_not_depend_on_ron_ledger_or_quickchain_runtime_crates
route_docs_keep_omnigate_as_hydrator_and_coordinator_not_chain_runtime
wallet_surface_is_display_balance_and_explicit_hold_front_door_only
public_v1_router_does_not_publish_quickchain_authority_routes
omnigate_source_does_not_import_direct_ledger_or_chain_authority
stream_and_chat_sources_explicitly_fail_closed_for_cached_paid_media
docs_state_cache_and_b3_are_not_paid_entitlement_authority
paid_access_routes_do_not_parse_transport_cache_headers_as_unlock_authority
production_source_does_not_claim_cache_b3_or_manifest_proves_payment
wallet_balance_fallback_is_explicitly_non_authoritative
paid_routes_use_wallet_receipt_values_without_dev_receipt_fabrication
production_source_does_not_construct_root_or_validator_authority_fields
production_source_does_not_construct_obvious_fake_receipts_or_finality
paid_storage_route_stays_prepare_estimate_or_proxy_only
content_view_and_site_visit_pay_routes_name_wallet_front_door_invariant
stream_and_chat_paid_paths_fail_closed_on_backend_truth
production_source_does_not_contain_paid_or_unlock_bypass_shortcuts
shared_header_policy_blocks_quickchain_authority_headers
docs_state_transport_headers_are_not_economic_authority
proxy_like_routes_use_shared_header_policy_for_x_ron_forwarding
route_sources_do_not_accept_fake_x_ron_authority_headers
receipt_or_paid_authority_must_come_from_backend_paths_not_request_headers
production_x_ron_headers_do_not_define_quickchain_authority_tokens_outside_policy
docs_keep_omnigate_out_of_receipt_balance_and_finality_truth
docs_lock_gateway_to_omnigate_to_wallet_to_ledger_value_loop
docs_separate_accepted_receipt_from_future_finality
```

## 4.6 Omnigate paid/product regressions stayed green

The omnigate hardening did not break paid/product route behavior.

Green paid/product route regressions included:

```text
content_view: 3 passed
site_visit: 3 passed
streams: 1 passed
chat_routes: 2 passed
paid_storage_estimate_proxy: 3 passed
paid_storage_prepare: 4 passed
paid_storage_write_proxy: 4 passed
```

Representative preserved behavior:

```text
content view quote rejects recipient mismatch
content view pay recovers wallet nonce and returns wallet receipt
site visit quote returns manifest payout recipient
site visit pay recovers wallet nonce and returns wallet receipt
stream latest segment requires stream-bound wallet receipt
chat paid send requires quote
paid storage estimate/prepare/write proxy paths pass upstream errors safely
paid object prepare returns hold template from storage estimate
paid write proxy supports POST and PUT happy paths
```

## 4.7 Omnigate all-targets and clippy green

The omnigate parking gate also ran:

```text
cargo test -p omnigate --all-targets
cargo clippy -p omnigate --all-targets --no-deps -- -D warnings
```

Both completed successfully.

All-targets also re-ran broad product coverage, including:

```text
article asset publish
asset page resolver
builtin page resolver
chat routes
comment asset publish
content view
image asset prepare
middleware contract
named site crab resolver
paid storage estimate
paid storage prepare
paid storage write
passport profile routes
policy gate
site launch
site visit
streams
text asset publish
zk receipt/read-only placeholder targets
```

## 4.8 Omnigate scripts/tooling hardened

Omnigate crate-local QuickChain scripts now define the repeatable parking workflow:

```text
crates/omnigate/scripts/dev-quickchain-preflight.sh
crates/omnigate/scripts/dev-quickchain-park.sh
```

The preflight script behavior:

```text
checks docs exist
checks test directory exists
rejects checked-in Python helper drift
runs cargo fmt -p omnigate -- --check
dynamically discovers quickchain*.rs tests
runs every discovered focused QuickChain test
runs paid/product route regressions
runs cargo test -p omnigate --all-targets
runs cargo clippy -p omnigate --all-targets --no-deps -- -D warnings
prints forbidden-scope marker
prints dynamic test-count marker
```

The park script behavior:

```text
checks docs/script/tooling files exist
delegates to dev-quickchain-preflight.sh
prints == omnigate QuickChain parking gate passed ==
```

## 4.9 Omnigate later work

Not required before moving on, but worth tracking later:

```text
proof/finality display model once upstream QuickChain proof DTOs and vectors are mature
route inventory snapshot for future proof/status endpoints, if any
hydration-specific negative tests for new product pages
better typed error envelopes for rejected authority headers
observability for rejected authority-smuggling attempts
more e2e tests with CrabLink Tauri display-only receipts
production wallet/access transport hardening
policy/config polish for paid product flows
CI integration of focused QuickChain park gate
```

Still do not add to omnigate:

```text
root creation
checkpoint writing
validator APIs
bridge settlement
external anchor settlement
staking
liquidity
direct wallet mutation
direct ledger mutation
omnigate-owned receipt issuance
omnigate-owned balance truth
cache-only paid unlock
manifest-only paid unlock
b3-only paid unlock
transport-header paid unlock
```

---

# 5. Pair-level architecture preserved

The combined pair now has the right division of responsibility:

```text
svc-gateway receives public client traffic.
svc-gateway proxies and filters.
svc-gateway fails closed.
svc-gateway does not decide wallet/ledger truth.

omnigate hydrates and coordinates.
omnigate prepares/quotes paid flows.
omnigate displays backend-derived receipt/access context.
omnigate fails closed when backend truth is missing.
omnigate does not create wallet/ledger truth.

svc-wallet remains the mutation front-door.
ron-ledger remains durable economic truth.
QuickChain future roots/proofs/finality remain outside gateway and omnigate.
```

The safe current paid flow is:

```text
client request
-> svc-gateway public route/proxy/admission boundary
-> omnigate quote/access/hydration coordinator
-> svc-wallet hold/transfer/capture/release/receipt path
-> ron-ledger accepted receipt
-> omnigate/gateway paid unlock/render from backend-derived truth
-> client display-only cache/receipt panel
```

Accepted-vs-finalized wording remains preserved:

```text
accepted backend receipt may unlock local paid content
accepted is not finalized
accepted is not epoch_included
accepted is not anchored
accepted is not external settlement
```

---

# 6. Commands that proved the pair

## `svc-gateway`

Final command:

```bash
bash crates/svc-gateway/scripts/dev-quickchain-park.sh
```

The park script delegated to preflight and ran:

```text
fmt check
dynamic quickchain test discovery
8 focused quickchain*.rs tests
product/proxy regressions
all-targets
clippy -D warnings
forbidden-scope marker
parking marker
```

## `omnigate`

Final command sequence:

```bash
cargo fmt -p omnigate
cargo test -p omnigate --test quickchain_preflight_transport_authority
bash crates/omnigate/scripts/dev-quickchain-park.sh
```

The park script delegated to preflight and ran:

```text
fmt check
dynamic quickchain test discovery
8 focused quickchain*.rs tests
paid/product route regressions
all-targets
clippy -D warnings
forbidden-scope marker
parking marker
```

---

# 7. Regenerate focused codebundles after notes

Recommended regeneration commands:

```bash
bash scripts/make_crate_codex.sh --force -c svc-gateway
bash scripts/make_crate_codex.sh --force -c omnigate
```

Do this after notes are updated so the next session sees the docs/tests/scripts/notes state.

---

# 8. Low-disk reminder

Do not run duplicate full gates.

Preferred workflow:

```text
focused test(s) first
then park script once
do not run preflight and park back-to-back for the same crate unless deliberately duplicating
```

Low-disk gateway gate:

```bash
CARGO_INCREMENTAL=0 RUSTFLAGS="-C debuginfo=0" \
CARGO_TARGET_DIR="$PWD/.quickchain-target-lowdisk/svc-gateway" \
bash crates/svc-gateway/scripts/dev-quickchain-park.sh

rm -rf .quickchain-target-lowdisk/svc-gateway
```

Low-disk omnigate gate:

```bash
CARGO_INCREMENTAL=0 RUSTFLAGS="-C debuginfo=0" \
CARGO_TARGET_DIR="$PWD/.quickchain-target-lowdisk/omnigate" \
bash crates/omnigate/scripts/dev-quickchain-park.sh

rm -rf .quickchain-target-lowdisk/omnigate
```

---

# 9. Current QuickChain Phase 1 completion impact

This pair closes one of the most dangerous Phase 1 foundation gaps: the public/product route boundary.

Before this pair, the main risk was that the public gateway or hydrator could accidentally grow into:

```text
receipt authority
balance authority
paid unlock authority
cache entitlement authority
future root/checkpoint/finality display authority
transport-header authority
bridge/external settlement surface
```

After this pair, those paths are guarded by:

```text
docs
source scans
route-boundary tests
header-policy tests
cache-boundary tests
no-fake-receipt tests
paid-access tests
value-loop tests
tooling tests
product regressions
all-targets
clippy
parking markers
```

Updated Phase 1 foundation progress estimate:

```text
QuickChain Phase 1 foundation: approximately 75% complete
```

Remaining Phase 1 foundation pairs after this:

```text
svc-index + ron-policy
CrabLink Tauri + client adapters
final integration/carryover pass
```

---

# 10. Next crate pair

The next pair should be:

```text
svc-index + ron-policy
```

Reason:

```text
svc-index must remain lookup/pointer infrastructure, not paid entitlement or proof authority.
ron-policy must remain declarative policy/evaluation, not wallet, ledger, receipt, balance, root, checkpoint, validator, bridge, or settlement authority.
```

Expected next-session focus:

```text
index pointer authority
policy decision non-authority
accepted-vs-finalized wording
no paid unlock from index lookup alone
no receipt/balance/finality from policy decisions
no root/checkpoint/validator/bridge/external settlement creep
crate-local park gates
```

### END NOTE - JUNE 21 2026 - 12:55 CST


### BEGIN NOTE - JUNE 22 2026 - 14:15 CST

The run proves both crates are now parked: each new QC-1A pair-interlock suite passed **5/5**, then `svc-gateway` and `omnigate` each printed the final QuickChain parking marker with `tests=9`. The buildplan still labels this as **Phase 1 Round 1 / QC-1A foundation**, not full Phase 1 completion; the purpose is to prepare crates for roots/proofs without letting services become root authority.  

Paste this into the crate notes.

### BEGIN NOTE - JUNE 22 2026 - QUICKCHAIN PHASE 1 ROUND 1 / QC-1A - svc-gateway + omnigate

# QuickChain Phase 1 Round 1 / QC-1A Notes — `svc-gateway` + `omnigate`

## 0. Current Status

```text
svc-gateway: GREEN / parked for QuickChain Phase 1 Round 1 / QC-1A foundation
omnigate:    GREEN / parked for QuickChain Phase 1 Round 1 / QC-1A foundation
```

This pair is now parked for the current QC-1A foundation pass.

This is not full Phase 1 completion.

This is not root-producing implementation.

This is not proof engine implementation.

This is not validator, committee, bridge, staking, liquidity, ROX, Solana, external settlement, or public-chain runtime implementation.

Safe label:

```text
svc-gateway + omnigate are green and parked for QuickChain Phase 1 Round 1 / QC-1A foundation.
```

Do not call this:

```text
Full Phase 1 complete
root engine complete
proof engine complete
validator ready
settlement ready
QuickChain complete
```

---

## 1. Buildplan Context

The active phase/round reference is:

```text
QUICKCHAIN_BUILDPLAN.MD
```

The master QuickChain blueprint remains:

```text
QUICKCHAIN_REVIEW_BUNDLE.MD
```

Current round:

```text
Phase 1 Round 1 — QC-1A pre-root/root-material foundation
```

Round purpose:

```text
Prepare every crate for roots/proofs without letting any service become root authority.
```

For this crate pair, the buildplan goal was:

```text
svc-gateway + omnigate:
  - paid enforcement stays backend-derived
  - no gateway/omnigate ledger mutation
  - no fake paid unlock
  - no QuickChain state authority
  - no route-level finality claims
```

This pair now satisfies that current QC-1A boundary/interlock goal.

---

## 2. What Was Added

Two new pair-interlock test files were added:

```text
crates/svc-gateway/tests/quickchain_preflight_phase1_pair_interlock.rs
crates/omnigate/tests/quickchain_preflight_phase1_pair_interlock.rs
```

These tests are intentionally boundary/interlock tests only.

They do not implement roots, checkpoints, validators, settlement, bridges, anchors, staking, liquidity, or external chain behavior.

They harden the public/product access path so the gateway and hydration layers cannot quietly become authority over:

```text
wallet truth
ledger truth
receipt truth
balance truth
root truth
checkpoint truth
finality truth
validator authority
bridge authority
external settlement authority
cache entitlement authority
client/header-paid authority
```

---

## 3. `svc-gateway` Notes

## 3.1 Gateway Role Preserved

`svc-gateway` remains:

```text
public boundary
route/proxy layer
public enforcement layer
header/filtering boundary
fail-closed public API surface
backend-derived paid/display response carrier
```

`svc-gateway` may:

```text
terminate public API ingress
route requests
proxy requests
apply public-boundary checks
apply bounded transport/header policy
fail closed
forward selected backend-derived context
return bounded HTTP/problem responses
preserve safe request context such as auth/correlation/idempotency where allowed
```

`svc-gateway` must not:

```text
mint ROC
issue ROC
transfer ROC
burn ROC
hold ROC
capture ROC
release ROC
mutate ron-ledger
invent wallet receipts
invent balances
invent paid access
invent finality
invent roots
invent checkpoints
act as validator
act as committee member
act as bridge
act as external settlement authority
unlock paid content from client/cache/header claims alone
treat b3 as payment proof
treat crab:// navigation as economic authority
treat transport headers as trust roots
create route-level finality claims
create QuickChain state authority
```

Boundary sentence locked by this pass:

```text
svc-gateway is the public boundary; it may route, proxy, and fail closed, but it must not become wallet, ledger, receipt, balance, root, finality, validator, bridge, cache, or external settlement authority.
```

---

## 3.2 Gateway Pair-Interlock Suite

New focused test:

```text
cargo test -p svc-gateway --test quickchain_preflight_phase1_pair_interlock
```

Result:

```text
5 passed
0 failed
```

Tests proved:

```text
docs_lock_gateway_into_qc1a_pair_interlock_role
manifest_does_not_add_chain_or_ledger_runtime_dependencies
runtime_source_does_not_define_direct_economic_or_quickchain_authority
public_routes_do_not_expose_live_quickchain_or_settlement_families
dynamic_preflight_will_pick_up_the_phase1_pair_interlock_suite
```

Meaning:

```text
Gateway docs explicitly lock the QC-1A role.
Cargo.toml does not add chain/ledger runtime dependencies.
Runtime source does not define direct ledger/economic/QuickChain authority.
Public routes do not expose live quickchain/checkpoint/root/validator/bridge/settlement/staking/liquidity/ROX/Solana route families.
The dynamic preflight runner discovers the new quickchain*.rs suite automatically.
```

---

## 3.3 Gateway Existing QuickChain Suites Stayed Green

The park script dynamically discovered and ran all focused `quickchain*.rs` tests.

Green focused QuickChain suites included:

```text
quickchain_preflight_boundary:              4 passed
quickchain_preflight_cache_boundary:        3 passed
quickchain_preflight_docs:                  5 passed
quickchain_preflight_no_fake_receipts:      3 passed
quickchain_preflight_paid_access:           3 passed
quickchain_preflight_phase1_pair_interlock: 5 passed
quickchain_preflight_transport_authority:   3 passed
quickchain_preflight_value_loop_boundary:   4 passed
quickchain_tooling_boundary:                5 passed
```

The gateway preflight reported:

```text
== svc-gateway quickchain exhaustive preflight gate passed: tests=9 ==
== svc-gateway QuickChain parking gate passed ==
```

---

## 3.4 Gateway Product/Proxy Regressions Stayed Green

The gateway hardening did not break normal product proxy behavior.

Green product/proxy regression suites included:

```text
app_proxy:                     5 passed
paid_storage_estimate_proxy:   3 passed
paid_storage_write_proxy:      4 passed
product_routes_proxy:          8 passed
site_visit_routes_proxy:       1 passed
```

The gateway `--all-targets` pass also confirmed additional proxy/product suites stayed green, including:

```text
content_view_routes_proxy: 1 passed
identity_routes_proxy:     7 passed
smoke:                     1 passed
```

Representative preserved behavior:

```text
app proxy preserves methods, body, query, auth, correlation headers, and generated request IDs
app proxy passes problem bodies unchanged
paid storage estimate/write proxies pass omnigate errors through
paid storage estimate/write proxies map upstream connect failures to problem 502
image/site/text product routes target omnigate
b3 asset proxy targets omnigate b3 route
chat product routes target omnigate
site resolve proxy targets omnigate site hydrator
paid prepare proxy preserves body and idempotency
crab resolve proxy preserves query and headers
site visit quote/pay proxies to omnigate and preserve context
wallet balance route targets the omnigate wallet-balance display path
identity/passport profile routes preserve allowed headers and upstream errors
```

---

## 3.5 Gateway All-Targets and Clippy Green

The gateway park gate ran:

```text
cargo test -p svc-gateway --all-targets
cargo clippy -p svc-gateway --all-targets --no-deps -- -D warnings
```

Both completed successfully.

Final gateway marker:

```text
[svc-gateway quickchain] forbidden scope remains parked: no roots, no checkpoints, no validators, no bridges, no external settlement, no fake receipts, no fake balances
svc-gateway QuickChain preflight gate passed
== svc-gateway quickchain exhaustive preflight gate passed: tests=9 ==
== svc-gateway QuickChain parking gate passed ==
```

---

## 4. `omnigate` Notes

## 4.1 Omnigate Role Preserved

`omnigate` remains:

```text
hydration coordinator
product/access coordinator
BFF/view composition boundary
backend-derived paid-access coordinator
display/access composition layer
```

`omnigate` may:

```text
hydrate product views
coordinate quote/access flows
call backend services through approved paths
compose client-facing responses
carry backend-derived receipts/display context
fail closed when backend truth is absent
enforce policy/readiness/admission boundaries
coordinate with wallet front-door paths without becoming the wallet
```

`omnigate` must not:

```text
mint ROC
issue ROC
transfer ROC
burn ROC
hold ROC directly as ledger truth
capture ROC directly as ledger truth
release ROC directly as ledger truth
mutate ron-ledger
invent receipts
invent balances
invent paid access
invent finality
invent roots
invent checkpoints
act as validator
act as committee member
act as bridge
act as external settlement authority
unlock paid content from hydration cache
unlock paid content from display receipt cache
unlock paid content from index pointers
unlock paid content from policy decisions alone
treat b3 as payment proof
treat manifests as payment proof
treat client headers as paid authority
create route-level finality claims
create QuickChain state authority
```

Boundary sentence locked by this pass:

```text
omnigate is a hydration/access coordinator; it may compose backend-derived product responses, but it must not become wallet, ledger, receipt, balance, root, finality, validator, bridge, cache, policy, index, or external settlement authority.
```

---

## 4.2 Omnigate Pair-Interlock Suite

New focused test:

```text
cargo test -p omnigate --test quickchain_preflight_phase1_pair_interlock
```

Result:

```text
5 passed
0 failed
```

Tests proved:

```text
docs_lock_omnigate_into_qc1a_pair_interlock_role
manifest_does_not_add_chain_or_ledger_runtime_dependencies
runtime_source_does_not_define_direct_economic_or_quickchain_authority
public_routes_do_not_expose_live_quickchain_or_settlement_families
dynamic_preflight_will_pick_up_the_phase1_pair_interlock_suite
```

Meaning:

```text
Omnigate docs explicitly lock the QC-1A role.
Cargo.toml does not add chain/ledger runtime dependencies.
Runtime source does not define direct ledger/economic/QuickChain authority.
Public routes do not expose live quickchain/checkpoint/root/validator/bridge/settlement/staking/liquidity/ROX/Solana route families.
The dynamic preflight runner discovers the new quickchain*.rs suite automatically.
```

---

## 4.3 Omnigate Existing QuickChain Suites Stayed Green

The park script dynamically discovered and ran all focused `quickchain*.rs` tests.

Green focused QuickChain suites included:

```text
quickchain_preflight_boundary:              5 passed
quickchain_preflight_cache_boundary:        4 passed
quickchain_preflight_docs:                  5 passed
quickchain_preflight_no_fake_receipts:      4 passed
quickchain_preflight_paid_access:           4 passed
quickchain_preflight_phase1_pair_interlock: 5 passed
quickchain_preflight_transport_authority:   6 passed
quickchain_preflight_value_loop_boundary:   4 passed
quickchain_tooling_boundary:                5 passed
```

The omnigate preflight reported:

```text
== omnigate quickchain exhaustive preflight gate passed: tests=9 ==
== omnigate QuickChain parking gate passed ==
```

---

## 4.4 Omnigate Paid/Product Regressions Stayed Green

The omnigate hardening did not break normal paid/product behavior.

Green paid/product regression suites included:

```text
content_view:                3 passed
site_visit:                  3 passed
streams:                     1 passed
chat_routes:                 2 passed
paid_storage_estimate_proxy: 3 passed
paid_storage_prepare:        4 passed
paid_storage_write_proxy:    4 passed
```

The full all-targets run also kept broader product/hydration suites green, including:

```text
article_asset_publish:       4 passed
asset_page_resolver:         15 passed
builtin_page_resolver:       6 passed
comment_asset_publish:       4 passed
dto_serialization:           1 passed
image_asset_prepare:         5 passed
metrics_contract:            1 passed
middleware_contract:         5 passed
named_site_crab_resolver:    2 passed
passport_profile_routes:     5 passed
policy_gate:                 2 passed
policy_metrics:              1 passed
readiness_error_rate:        1 passed
readiness_inflight:          1 passed
ready_truth:                 1 passed
site_launch:                 5 passed
text_asset_publish:          5 passed
zk_receipts:                 2 passed
```

Representative preserved behavior:

```text
content-view quote returns manifest recipient without wallet call
content-view pay recovers wallet nonce and returns wallet receipt
site-visit quote returns ROC amount and manifest payout recipient
site-visit pay recovers wallet nonce and returns wallet receipt
stream latest segment requires stream-bound wallet receipt
chat paid send requires quote
chat moderation fails closed
paid storage prepare returns hold template from storage estimate
paid storage write proxy passes storage errors through
site creation stores manifest and index pointer
site resolve hydrates manifest from index and storage
asset resolver preserves canonical crab/b3 route behavior
passport profile routes preserve upstream behavior
policy/readiness/middleware contracts remain green
```

---

## 4.5 Omnigate All-Targets and Clippy Green

The omnigate park gate ran:

```text
cargo test -p omnigate --all-targets
cargo clippy -p omnigate --all-targets --no-deps -- -D warnings
```

Both completed successfully.

Final omnigate marker:

```text
[omnigate quickchain] forbidden scope remains parked: no roots, no checkpoints, no validators, no bridges, no external settlement, no fake receipts, no fake balances
omnigate QuickChain preflight gate passed
== omnigate quickchain exhaustive preflight gate passed: tests=9 ==
== omnigate QuickChain parking gate passed ==
```

---

## 5. Shared Pair Boundary Proven

This pair now proves the public/product boundary remains:

```text
client intent
→ svc-gateway public route/proxy/enforcement boundary
→ omnigate quote/access/hydration coordinator
→ svc-wallet mutation front-door
→ ron-ledger durable economic truth
→ backend-derived receipt/access result
→ paid unlock/render/display path
```

The pair explicitly does not create:

```text
gateway ledger truth
omnigate ledger truth
gateway receipt truth
omnigate receipt truth
gateway balance truth
omnigate balance truth
gateway root truth
omnigate root truth
gateway checkpoint truth
omnigate checkpoint truth
gateway finality truth
omnigate finality truth
gateway validator authority
omnigate validator authority
gateway bridge authority
omnigate bridge authority
gateway settlement authority
omnigate settlement authority
cache entitlement authority
client/header entitlement authority
```

Backend-derived enforcement remains the rule.

No invented trust was introduced.

No local/client/cache/header-only paid unlock was introduced.

No route-level finality was introduced.

No gateway/omnigate mutation authority was introduced.

---

## 6. Forbidden Scope Still Parked

This pair did not add:

```text
roots
state roots
receipt roots
hold roots
epoch roots
checkpoints
checkpoint signing
validator signatures
validator sets
committee replication
staking
liquidity
ROX runtime
Solana runtime
external settlement
public bridge
external anchors
public-chain route families
gateway direct ledger mutation
omnigate direct ledger mutation
gateway direct wallet mutation outside backend wallet path
omnigate direct wallet mutation outside backend wallet path
fake receipts
fake balances
fake finality
cache-only paid unlock
policy-only paid unlock
index-pointer paid unlock
b3-as-payment-proof
crab-url-as-payment-proof
transport-header-as-payment-proof
```

This remains a QC-1A boundary/interlock hardening pass only.

---

## 7. Commands Proven Green

Focused tests:

```bash
cargo test -p svc-gateway --test quickchain_preflight_phase1_pair_interlock
cargo test -p omnigate --test quickchain_preflight_phase1_pair_interlock
```

Park scripts:

```bash
crates/svc-gateway/scripts/dev-quickchain-park.sh
crates/omnigate/scripts/dev-quickchain-park.sh
```

Final observed markers:

```text
== svc-gateway quickchain exhaustive preflight gate passed: tests=9 ==
== svc-gateway QuickChain parking gate passed ==

== omnigate quickchain exhaustive preflight gate passed: tests=9 ==
== omnigate QuickChain parking gate passed ==
```

---

## 8. Low-Disk Workflow Note

The correct workflow for this pair was followed:

```text
focused tests first
then each crate’s park script once
no duplicate preflight + park run for the same crate
```

This matches the low-disk QuickChain workflow:

```text
Do not run both dev-quickchain-preflight.sh and dev-quickchain-park.sh for the same crate unless deliberately needing two full passes.
```

The park scripts already delegate to the preflight gate.

---

## 9. Future Work for These Crates

These crates are parkable for QC-1A Round 1, but future work remains after core root/proof material is actually implemented in the proper crates.

Future `svc-gateway` work may include:

```text
confirming future proof-status display routes are read-only
confirming future root/proof APIs are proxy/display only if exposed at all
stronger header-policy regression tests as proof DTOs mature
stronger paid-flow integration tests once receipt/proof status exists
ensuring no public route claims finality until backend proof verification exists
gateway display of future proof status without becoming proof authority
```

Still forbidden for `svc-gateway` until explicitly authorized by later phase gates:

```text
direct ledger mutation
direct wallet mutation
root production
checkpoint production
validator participation
bridge settlement
external anchors
staking
liquidity
route-level finality claims
fake receipts
fake balances
cache-only paid unlock
```

Future `omnigate` work may include:

```text
future read-only proof-status hydration once proof DTOs exist
future display-only receipt/proof panels sourced from backend truth
stronger wallet receipt lookup hardening
stronger paid-access failure-mode tests
future proof inclusion display without proof authority
future accounting/rewarder context display without mutation authority
future policy/index/cache boundary tests as proof status matures
```

Still forbidden for `omnigate` until explicitly authorized by later phase gates:

```text
direct ledger mutation
direct wallet mutation
root production
checkpoint production
validator participation
bridge settlement
external anchors
staking
liquidity
hydration-level finality claims
fake receipts
fake balances
cache-only paid unlock
policy-only paid unlock
index-pointer paid unlock
```

---

## 10. Parked Status

`svc-gateway` is parked for the current QuickChain Phase 1 Round 1 / QC-1A foundation pass.

`omnigate` is parked for the current QuickChain Phase 1 Round 1 / QC-1A foundation pass.

This crate pair can now be treated as complete for the current QC-1A boundary/interlock sweep.

Recommended regeneration commands after saving notes:

```bash
bash scripts/make_crate_codex.sh --force -c svc-gateway
bash scripts/make_crate_codex.sh --force -c omnigate
```

Next crate pair in the buildplan order:

```text
svc-index + ron-policy
```

Next pair mission:

```text
Prove svc-index remains lookup/pointer/index only.
Prove ron-policy remains declarative policy only.
No names/pointers/policy decisions as proof.
No cache/name/policy as paid-unlock authority.
No policy-created finality.
No index-created finality.
No ledger mutation.
No roots/checkpoints/validators/bridges/external settlement.
```


### END NOTE - JUNE 22 2026 - 14:15 CST


### BEGIN NOTE - JUNE 23 2026 - 15:05 CST

The green run proves the pair hit the buildplan completion definition: focused Round 2 confirmation tests passed for both crates, dynamic discovery picked them up, both park gates passed, and the pair is now safe to mark **Phase 1 complete / parked** while full Phase 1 still waits on `svc-index + ron-policy` and CrabLink Tauri/client adapters.  The buildplan still keeps `svc-gateway + omnigate` non-authoritative: they may enforce/publicly expose flows, but they must not mutate ledger truth or own balances, roots, finality, or settlement truth. 

Here are paste-ready crate notes:

### BEGIN NOTE - JUNE 23 2026 - QUICKCHAIN PHASE 1 ROUND 2 - svc-gateway + omnigate

# QuickChain Phase 1 Notes — `svc-gateway` + `omnigate`

## 0. Final status

`svc-gateway` and `omnigate` are now **QuickChain Phase 1 complete / parked**.

This means both Phase 1 rounds for this crate pair are complete:

```text
QuickChain Phase 1 — svc-gateway + omnigate
Round 1: COMPLETE
Round 2: COMPLETE
Pair status: PHASE 1 COMPLETE / PARKED
```

Do not call this:

```text
Full QuickChain Phase 1 complete
QuickChain complete
validator ready
settlement ready
bridge ready
external settlement ready
```

Full QuickChain Phase 1 is not complete until the remaining pairs are done:

```text
svc-index + ron-policy
CrabLink Tauri + client adapters
final Phase 1 audit, if needed
```

The next planned major work on `svc-gateway + omnigate` should be in **Phase 2**, unless a later Phase 1 crate pair exposes a concrete integration issue that requires a small downstream fix.

---

## 1. What this session accomplished

This session completed QuickChain Phase 1 Round 2 downstream confirmation for:

```text
svc-gateway
omnigate
```

The goal was not to add root-producing code.

The goal was to prove that these two downstream services can remain safe while the real deterministic root/proof work stays in the authorized core path.

The patch added Phase 1 Round 2 confirmation coverage:

```text
crates/svc-gateway/tests/quickchain_phase1_round2_confirmation.rs
crates/omnigate/tests/quickchain_phase1_round2_confirmation.rs
```

The patch also appended Phase 1 Round 2 downstream-confirmation language to:

```text
crates/svc-gateway/docs/quickchain-preflight.md
crates/omnigate/docs/quickchain-preflight.md
```

No Cargo.toml dependency changes were required.

No new runtime authority was introduced.

No root/checkpoint/validator/bridge/external settlement code was introduced.

---

## 2. Why this was downstream confirmation only

According to the QuickChain buildplan, Phase 1 Round 2 is core-heavy and downstream-light.

The core deterministic root/proof implementation belongs primarily in:

```text
ron-proto
ron-ledger
```

For `svc-gateway + omnigate`, Round 2 only needed confirmation that they can expose, route, hydrate, and enforce backend-derived state without becoming QuickChain authority.

The confirmed boundary is:

```text
svc-gateway:
  public route/admission/proxy/enforcement boundary only

omnigate:
  product hydration / quote-access coordination / backend-derived display only
```

Both crates remain downstream services. They are not balance truth, receipt truth, root truth, finality truth, or settlement truth.

---

## 3. `svc-gateway` final QuickChain Phase 1 role

`svc-gateway` is the public boundary.

Allowed role:

```text
public route surface
admission/proxy boundary
paid enforcement boundary
backend-derived receipt/status forwarding
safe response shaping
policy-aware route enforcement
bounded transport headers
redacted errors
fail-closed product proxying
```

Forbidden role:

```text
wallet authority
ledger authority
balance truth
receipt truth
root authority
checkpoint authority
validator authority
bridge authority
external settlement authority
public chain authority
client entitlement authority
cache-only unlock authority
route-level finality authority
```

`svc-gateway` may forward backend-derived evidence, but it must not create that evidence.

`svc-gateway` may proxy paid access flows, but it must not decide wallet or ledger truth.

`svc-gateway` may display accepted receipt status, but it must not promote `accepted` into `epoch_included`, `finalized`, or `anchored`.

---

## 4. `omnigate` final QuickChain Phase 1 role

`omnigate` is the product coordinator and hydrator.

Allowed role:

```text
hydration coordinator
quote/access coordinator
backend-derived paid access orchestration
site/content hydration
route-level product composition
receipt/status display material
safe upstream/downstream adapter behavior
fail-closed paid/product coordination
```

Forbidden role:

```text
wallet authority
ledger authority
durable economic truth
balance truth
receipt truth
root authority
checkpoint authority
validator authority
bridge authority
external settlement authority
public chain authority
cache entitlement authority
manifest-only paid unlock authority
b3-only paid unlock authority
transport-header paid unlock authority
```

`omnigate` may coordinate a paid flow, but it must not become the paid truth source.

`omnigate` may use backend wallet/ledger receipt values to unlock or hydrate paid content, but it must not fabricate receipts, balances, or finality.

---

## 5. Phase 1 Round 2 confirmation tests added

### `svc-gateway`

Added:

```text
crates/svc-gateway/tests/quickchain_phase1_round2_confirmation.rs
```

This test suite confirms:

```text
docs name Phase 1 Round 2 downstream confirmation
gateway paid enforcement remains backend-derived
gateway is not wallet truth
gateway is not ledger truth
gateway is not QuickChain root authority
gateway is not finality authority
gateway cannot unlock paid content from cache alone
svc-wallet remains the paid mutation path
ron-ledger remains durable economic truth
accepted backend receipt can unlock local paid content
accepted is not finalized
accepted is not epoch_included
accepted is not anchored
future statuses remain parked
gateway never converts accepted wallet/ledger receipt into QuickChain finality
gateway must label status honestly and must not fabricate status
source has no direct ledger or QuickChain authority imports
route literals do not expose forbidden QuickChain/external runtime paths
preflight runner dynamically discovers the new test
```

### `omnigate`

Added:

```text
crates/omnigate/tests/quickchain_phase1_round2_confirmation.rs
```

This test suite confirms:

```text
docs name Phase 1 Round 2 downstream confirmation
omnigate hydration remains backend-derived
omnigate is not wallet truth
omnigate is not ledger truth
omnigate is not QuickChain root authority
omnigate is not finality authority
omnigate cannot unlock paid content from cache alone
svc-wallet remains the paid mutation path
ron-ledger remains durable economic truth
accepted backend receipt can unlock local paid content
accepted is not finalized
accepted is not epoch_included
accepted is not anchored
omnigate cannot promote accepted receipt to epoch_included, finalized, or anchored
future statuses remain parked
omnigate must label status honestly and must not fabricate status
source has no direct ledger or QuickChain authority imports
route literals do not expose forbidden QuickChain/external runtime paths
preflight runner dynamically discovers the new test
```

---

## 6. Final green commands run

The focused Round 2 tests were run first:

```bash
cargo test -p svc-gateway --test quickchain_phase1_round2_confirmation
cargo test -p omnigate --test quickchain_phase1_round2_confirmation
```

Then the crate-local park gates were run once:

```bash
crates/svc-gateway/scripts/dev-quickchain-park.sh
crates/omnigate/scripts/dev-quickchain-park.sh
```

The focused tests passed for both crates:

```text
svc-gateway quickchain_phase1_round2_confirmation:
  5 passed
  0 failed

omnigate quickchain_phase1_round2_confirmation:
  5 passed
  0 failed
```

The final parking markers printed:

```text
== svc-gateway QuickChain parking gate passed ==
== omnigate QuickChain parking gate passed ==
```

---

## 7. `svc-gateway` gate summary

`svc-gateway` passed:

```text
cargo fmt check
dynamic quickchain test discovery
quickchain_phase1_round2_confirmation
quickchain_preflight_boundary
quickchain_preflight_cache_boundary
quickchain_preflight_docs
quickchain_preflight_no_fake_receipts
quickchain_preflight_paid_access
quickchain_preflight_phase1_pair_interlock
quickchain_preflight_transport_authority
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
product/proxy regressions
cargo test -p svc-gateway --all-targets
cargo clippy -p svc-gateway --all-targets --no-deps -- -D warnings
final park gate
```

Representative product/proxy regressions stayed green:

```text
app_proxy
paid_storage_estimate_proxy
paid_storage_write_proxy
product_routes_proxy
site_visit_routes_proxy
content_view_routes_proxy
identity_routes_proxy
smoke
```

Representative preserved behavior:

```text
app proxy preserves methods, body, query, auth, correlation headers, and generated request IDs
app proxy passes problem bodies unchanged
paid storage estimate/write proxies pass omnigate errors through
paid storage estimate/write proxies handle upstream connect failures as problem 502
product routes target omnigate
b3 asset proxy targets omnigate b3 route
chat product routes target omnigate
site resolve proxy targets omnigate site hydrator
paid prepare proxy preserves body and idempotency
site visit quote/pay proxies to omnigate and preserve context
identity and wallet display routes remain proxy/display surfaces
```

Final `svc-gateway` marker:

```text
svc-gateway QuickChain preflight gate passed
== svc-gateway quickchain exhaustive preflight gate passed: tests=10 ==
== svc-gateway QuickChain parking gate passed ==
```

---

## 8. `omnigate` gate summary

`omnigate` passed:

```text
cargo fmt check
dynamic quickchain test discovery
quickchain_phase1_round2_confirmation
quickchain_preflight_boundary
quickchain_preflight_cache_boundary
quickchain_preflight_docs
quickchain_preflight_no_fake_receipts
quickchain_preflight_paid_access
quickchain_preflight_phase1_pair_interlock
quickchain_preflight_transport_authority
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
paid/product route regressions
cargo test -p omnigate --all-targets
cargo clippy -p omnigate --all-targets --no-deps -- -D warnings
final park gate
```

Representative paid/product regressions stayed green:

```text
content_view
site_visit
streams
chat_routes
paid_storage_estimate_proxy
paid_storage_prepare
paid_storage_write_proxy
article_asset_publish
asset_page_resolver
builtin_page_resolver
comment_asset_publish
image_asset_prepare
middleware_contract
named_site_crab_resolver
passport_profile_routes
policy_gate
site_launch
text_asset_publish
zk receipt/read-only placeholder targets
```

Representative preserved behavior:

```text
content view quote rejects recipient mismatch
content view pay recovers wallet nonce and returns wallet receipt
site visit quote returns manifest payout recipient
site visit pay recovers wallet nonce and returns wallet receipt
stream latest segment requires stream-bound wallet receipt
chat paid send requires quote
paid storage estimate/prepare/write proxy paths pass upstream errors safely
paid object prepare returns hold template from storage estimate
paid write proxy supports POST and PUT happy paths
site launch stores manifest and index pointer
site resolve hydrates manifest from index and storage
asset page resolver keeps canonical crab/b3 behavior
passport profile routes proxy safely to svc-passport
```

Final `omnigate` marker:

```text
omnigate QuickChain preflight gate passed
== omnigate quickchain exhaustive preflight gate passed: tests=10 ==
== omnigate QuickChain parking gate passed ==
```

---

## 9. Pair-level architecture preserved

The combined pair now has the right division of responsibility:

```text
svc-gateway receives public client traffic.
svc-gateway proxies and filters.
svc-gateway fails closed.
svc-gateway does not decide wallet/ledger truth.

omnigate coordinates product hydration.
omnigate coordinates quote/access flows.
omnigate talks to backend services.
omnigate fails closed.
omnigate does not decide wallet/ledger truth.

svc-wallet remains the paid mutation front door.
ron-ledger remains durable economic truth.
backend wallet/ledger receipts remain accepted local unlock truth.
future QuickChain epoch/finality/anchor labels remain parked unless backed by future authorized proof paths.
```

This keeps the internal ROC value loop intact:

```text
ron-proto economic DTOs
-> ron-ledger durable economic truth
-> svc-wallet issue/transfer/burn/hold/capture/release/receipt
-> svc-storage/svc-gateway/omnigate paid enforcement
-> ron-accounting snapshots
-> svc-rewarder payout planning
-> wallet/ledger receipts
-> CrabLink display/user intent only
```

---

## 10. Receipt and status language preserved

Accepted paid access:

```text
accepted:
  svc-wallet / ron-ledger accepted the mutation.
  sufficient for local paid unlock.
```

Future settlement labels:

```text
epoch_included:
  receipt appears in a future epoch receipt root.

finalized:
  future challenge/finality threshold reached.

anchored:
  optional external anchor proves checkpoint commitment existed externally.
```

Rules:

```text
paid content unlock:
  accepted backend receipt is enough.

creator-visible pending earnings:
  accepted or epoch_included may be displayed honestly.

withdrawable/finalized rewards:
  require epoch_included/finalized according to policy.

external bridge/export/anchor claims:
  require finalized or anchored.

gateway/omnigate display:
  must label status honestly and must not fabricate status.
```

---

## 11. Still forbidden in `svc-gateway + omnigate`

The following remain forbidden until the correct later phase explicitly authorizes them:

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
gateway-owned receipt issuance
omnigate-owned receipt issuance
gateway-owned balance truth
omnigate-owned balance truth
gateway-owned finality truth
omnigate-owned finality truth
cache-only paid unlock
manifest-only paid unlock
b3-only paid unlock
transport-header paid unlock
client-header paid unlock
fake receipts
fake balances
fake finality
placeholder hashes
```

---

## 12. Scripts/tooling status

Both crates now have repeatable crate-local QuickChain parking workflows:

```text
crates/svc-gateway/scripts/dev-quickchain-preflight.sh
crates/svc-gateway/scripts/dev-quickchain-park.sh

crates/omnigate/scripts/dev-quickchain-preflight.sh
crates/omnigate/scripts/dev-quickchain-park.sh
```

The preflight scripts:

```text
check docs exist
check test directory exists
reject checked-in Python helper drift
run cargo fmt --check
dynamically discover quickchain*.rs tests
run every discovered focused QuickChain test
run product/proxy or paid/product regressions
run cargo test --all-targets
run cargo clippy --all-targets --no-deps -- -D warnings
print forbidden-scope marker
print dynamic test-count marker
```

The park scripts:

```text
check docs/script/tooling files exist
delegate to dev-quickchain-preflight.sh
print final parking marker
```

---

## 13. What not to reopen

Do not reopen this pair in Phase 1 unless there is a concrete failure from a later pair or final audit.

Do not rerun exhaustive gates for this pair immediately after the green park run.

Do not rerun already-green earlier pairs unless they were patched.

Do not run workspace-wide clippy unless doing the final phase audit.

Do not add Python helper scripts.

Do not add QuickChain runtime authority to satisfy scanner tests.

---

## 14. Current QuickChain Phase 1 status after this pair

Complete / parked:

```text
1. ron-proto + ron-ledger
2. svc-wallet + ron-accounting
3. svc-rewarder + svc-storage
4. svc-gateway + omnigate
```

Remaining:

```text
5. svc-index + ron-policy
6. CrabLink Tauri + client adapters
```

Recommended next crate pair:

```text
svc-index + ron-policy
```

---

## 15. Bottom line

`svc-gateway + omnigate` are complete for QuickChain Phase 1.

They are green, parked, and correctly non-authoritative.

The pair now proves:

```text
public routing is not settlement
hydration is not settlement
paid enforcement is backend-derived
accepted receipt is local unlock truth
future finality labels are not fabricated
gateway is not wallet/ledger/root/finality authority
omnigate is not wallet/ledger/root/finality authority
cache/header/index/storage metadata cannot unlock paid content alone
no external settlement/ROX/Solana/bridge/staking/liquidity path exists
```

Next planned major work for these crates should be Phase 2, unless a later Phase 1 pair or final audit reveals a concrete integration issue.

### END NOTE - JUNE 23 2026 - QUICKCHAIN PHASE 1 ROUND 2 - svc-gateway + omnigate


### END NOTE - JUNE 23 2026 - 15:05 CST


### BEGIN NOTE - JUNE 23 2026 - 21:35 CST


Below are paste-ready crate notes for `svc-gateway` and `omnigate`. Your terminal output shows both focused Phase 2 tests passed, both dynamic park gates discovered the new test, both all-targets and clippy gates passed, and both parking markers printed green. 

Add this to `crates/svc-gateway/NOTES.MD`:

### BEGIN NOTE - JUNE 23 2026 - QUICKCHAIN PHASE 2 ROUND 1 - svc-gateway

# QuickChain Crate Notes — svc-gateway

Date: June 23, 2026
Project: RustyOnions / CrabLink
Track: QuickChain Phase 2 Round 1
Round purpose: verifier artifact / read-only replication
Crate: `svc-gateway`
Status: COMPLETE / PARKED for QuickChain Phase 2 Round 1

---

## 0. Status Summary

`svc-gateway` is now complete and parked for QuickChain Phase 2 Round 1.

Final status:

```text
QuickChain Phase 2 Round 1 — svc-gateway COMPLETE / PARKED
```

This means `svc-gateway` has been hardened for the Phase 2 Round 1 read-only verifier/replay artifact slice.

It does not mean Phase 2 is complete. It does not mean committee signing, quorum, fork-choice, finality, validator readiness, staking, slashing, bridge behavior, ROX, Solana, or external settlement is implemented.

---

## 1. What This Pass Proved

This pass proved that `svc-gateway` can safely remain the public proxy/admission/route boundary while Phase 2 introduces read-only replay/proof/verifier artifacts elsewhere.

The crate remains:

```text
public gateway
proxy boundary
admission boundary
transport/header filter
product route forwarder
paid route forwarder
display/routing context carrier only
```

The crate does not become:

```text
verifier truth
replay truth
root authority
checkpoint authority
committee authority
quorum authority
fork-choice authority
finality authority
settlement authority
wallet mutation authority
ledger mutation authority
paid-unlock authority by replay artifact
bridge authority
staking/slashing authority
public-chain runtime
```

The important preserved line is:

```text
svc-gateway may route/display read-only verifier or replay metadata, but paid unlock still requires backend wallet/ledger truth.
```

---

## 2. Files Changed

Changed:

```text
crates/svc-gateway/src/headers/proxy.rs
crates/svc-gateway/tests/quickchain_phase2_replay_boundary.rs
crates/svc-gateway/docs/quickchain-preflight.md
```

No `Cargo.toml` dependency change was required.

---

## 3. Header Boundary Hardening

`crates/svc-gateway/src/headers/proxy.rs` was hardened so authority-shaped QuickChain Phase 2 headers do not pass through gateway product/proxy/response paths as trusted metadata.

The added blocked header families include:

```text
x-ron-proof-*
x-ron-replay-*
x-ron-verifier-*
x-ron-committee-*
x-ron-quorum
```

This extends the earlier protection around root/checkpoint/validator/ledger/bridge/finality-shaped metadata.

The purpose is not to prevent future backend verifier routes forever. The purpose is to prevent caller-supplied or upstream-smuggled headers from becoming gateway-endorsed verifier, replay, quorum, committee, finality, or paid-unlock authority.

---

## 4. New Phase 2 Round 1 Boundary Coverage

Added focused Phase 2 Round 1 coverage through:

```text
crates/svc-gateway/tests/quickchain_phase2_replay_boundary.rs
```

The focused test locks these boundaries:

```text
gateway docs name Phase 2 Round 1 verifier artifact / read-only replication
gateway may expose read-only proof/replay artifact routes if needed
gateway replay metadata is display and routing context only
gateway is not verifier truth
gateway is not replay truth
gateway is not quorum truth
gateway is not committee truth
gateway does not sign verifier attestations
gateway does not decide fork choice
gateway does not claim finality
gateway cannot unlock paid content from replay artifacts alone
paid unlock still requires backend wallet/ledger truth
authority-shaped replay/proof/verifier headers are stripped
public routes do not expose verifier/committee/finality/bridge/staking families
Cargo.toml does not gain direct ledger/wallet/external settlement dependencies
gateway source does not construct finality or committee authority statuses
```

The focused test passed:

```text
cargo test -p svc-gateway --test quickchain_phase2_replay_boundary

running 5 tests
verifier_replay_headers_are_transport_metadata_not_gateway_authority ... ok
docs_name_phase2_round1_gateway_read_only_replay_boundary ... ok
gateway_does_not_gain_direct_ledger_wallet_or_external_settlement_dependencies ... ok
gateway_routes_do_not_expose_verifier_committee_or_finality_authority_paths ... ok
gateway_source_does_not_construct_phase2_authority_status ... ok

test result: ok. 5 passed; 0 failed
```

---

## 5. Documentation Updated

Updated:

```text
crates/svc-gateway/docs/quickchain-preflight.md
```

The doc now explicitly names:

```text
Phase 2 Round 1 verifier artifact / read-only replication boundary
```

The doc now preserves these boundaries:

```text
svc-gateway may expose read-only proof/replay artifact routes if needed
gateway replay metadata is display and routing context only
replay artifacts are not gateway authority
proof artifacts are not gateway authority
verifier results are not gateway authority
committee attestations are not gateway authority
quorum claims are not gateway authority
fork-choice claims are not gateway authority
finality claims are not gateway authority
gateway is not verifier truth
gateway is not replay truth
gateway is not quorum truth
gateway is not committee truth
gateway does not sign verifier attestations
gateway does not decide fork choice
gateway does not claim finality
gateway cannot unlock paid content from replay artifacts alone
paid unlock still requires backend wallet/ledger truth
svc-wallet remains the paid mutation path
ron-ledger remains durable economic truth
```

---

## 6. Final Gate Evidence

The final local gate discovered 11 focused QuickChain tests:

```text
quickchain_phase1_round2_confirmation
quickchain_phase2_replay_boundary
quickchain_preflight_boundary
quickchain_preflight_cache_boundary
quickchain_preflight_docs
quickchain_preflight_no_fake_receipts
quickchain_preflight_paid_access
quickchain_preflight_phase1_pair_interlock
quickchain_preflight_transport_authority
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
```

The final parking markers were:

```text
== svc-gateway quickchain exhaustive preflight gate passed: tests=11 ==
== svc-gateway QuickChain parking gate passed ==
```

The all-targets gate completed cleanly:

```text
cargo test -p svc-gateway --all-targets
```

The clippy gate completed cleanly:

```text
cargo clippy -p svc-gateway --all-targets --no-deps -- -D warnings
```

The forbidden-scope marker stayed parked:

```text
no roots
no checkpoints
no validators
no bridges
no external settlement
no fake receipts
no fake balances
```

---

## 7. Important Existing Coverage Still Preserved

This pass did not replace older gateway safety coverage. It extended it.

Still protected:

```text
gateway remains proxy boundary, not chain runtime
gateway source does not import wallet, ledger, or QuickChain runtime authority
unsupported QuickChain/ledger/bridge/root routes fail closed
product proxy strips caller-supplied QuickChain authority headers
cache headers are transport metadata, not unlock authority
client/cache unlock signals are stripped when authority-shaped
routes do not unlock paid content from cache or query claims
backend receipt metadata is context, not gateway authority
request/response headers cannot smuggle economic or chain authority
client paid unlock headers are never gateway authority
public routes do not expose live QuickChain or settlement families
runtime source does not define direct economic or QuickChain authority
identity/retry/cache headers remain context only
gateway stays out of receipt, balance, and finality truth
accepted receipt is separated from future QuickChain statuses
```

---

## 8. Safe Meaning

The safe completion meaning for this crate is:

```text
svc-gateway is green/parked for QuickChain Phase 2 Round 1.
It can carry or route read-only replay/proof/verifier context if future routes need it,
but it cannot become verifier, replay, quorum, committee, fork-choice, finality, settlement,
wallet, ledger, bridge, staking, or paid-unlock authority.
```

---

## 9. What Not To Claim

Do not claim:

```text
Phase 2 complete
Phase 2 Round 1 complete across all crates
committee ready
validator ready
quorum ready
fork choice ready
finality ready
gateway verifier routes are live
gateway can verify roots
gateway can sign attestations
gateway can unlock paid content from replay artifacts
gateway can mutate wallet
gateway can mutate ledger
bridge ready
staking/slashing ready
external settlement ready
ROX/Solana runtime ready
```

---

## 10. Do Not Reopen Unless

Do not reopen `svc-gateway` during Phase 2 Round 1 unless:

```text
a later crate-pair exposes a real mismatch in verifier artifact routing/display boundaries
svc-index or ron-policy introduces a replay artifact pointer/policy shape requiring gateway confirmation
CrabLink client adapters need a stricter gateway boundary signal
svc-wallet receipt or paid-admission contracts change
ron-ledger economic truth or receipt format changes
omnigate route contracts change
QuickChain verifier artifact schemas change in ron-proto
a parking gate regresses
a source/header scanner catches new authority creep
```

Otherwise, `svc-gateway` should stay parked until Phase 2 Round 2 or a later explicitly scoped phase.

---

## 11. Next Pair

The next Phase 2 Round 1 crate pair is:

```text
svc-index + ron-policy
```

Expected goal for that pair:

```text
svc-index may point to read-only replay/proof artifacts but cannot become proof authority.
ron-policy may declare verifier access policy but cannot create verifier truth, settlement truth, finality, or paid-unlock authority.
```

### END NOTE - JUNE 23 2026 - QUICKCHAIN PHASE 2 ROUND 1 - svc-gateway

Add this to `crates/omnigate/NOTES.MD`:

### BEGIN NOTE - JUNE 23 2026 - QUICKCHAIN PHASE 2 ROUND 1 - omnigate

# QuickChain Crate Notes — omnigate

Date: June 23, 2026
Project: RustyOnions / CrabLink
Track: QuickChain Phase 2 Round 1
Round purpose: verifier artifact / read-only replication
Crate: `omnigate`
Status: COMPLETE / PARKED for QuickChain Phase 2 Round 1

---

## 0. Status Summary

`omnigate` is now complete and parked for QuickChain Phase 2 Round 1.

Final status:

```text
QuickChain Phase 2 Round 1 — omnigate COMPLETE / PARKED
```

This means `omnigate` has been hardened for the Phase 2 Round 1 read-only verifier/replay artifact slice.

It does not mean Phase 2 is complete. It does not mean committee signing, quorum, fork-choice, finality, validator readiness, staking, slashing, bridge behavior, ROX, Solana, or external settlement is implemented.

---

## 1. What This Pass Proved

This pass proved that `omnigate` can safely remain the hydration/product coordination boundary while Phase 2 introduces read-only replay/proof/verifier artifacts elsewhere.

The crate remains:

```text
hydration coordinator
product route coordinator
paid route coordinator
backend-derived display surface
wallet/ledger receipt consumer
storage/index/policy orchestration layer
read-only context carrier only
```

The crate does not become:

```text
verifier truth
replay truth
root authority
checkpoint authority
committee authority
quorum authority
fork-choice authority
finality authority
settlement authority
wallet mutation authority
ledger mutation authority
paid-unlock authority by replay artifact
bridge authority
staking/slashing authority
public-chain runtime
```

The important preserved line is:

```text
omnigate may hydrate/display read-only verifier or replay metadata, but paid unlock still requires backend wallet/ledger truth.
```

---

## 2. Files Changed

Changed:

```text
crates/omnigate/tests/quickchain_phase2_replay_boundary.rs
crates/omnigate/docs/quickchain-preflight.md
```

No `Cargo.toml` dependency change was required.

---

## 3. New Phase 2 Round 1 Boundary Coverage

Added focused Phase 2 Round 1 coverage through:

```text
crates/omnigate/tests/quickchain_phase2_replay_boundary.rs
```

The focused test locks these boundaries:

```text
omnigate docs name Phase 2 Round 1 verifier artifact / read-only replication
omnigate may expose read-only proof/replay artifact views if needed
omnigate replay metadata is display and hydration context only
omnigate is not verifier truth
omnigate is not replay truth
omnigate is not quorum truth
omnigate is not committee truth
omnigate does not sign verifier attestations
omnigate does not decide fork choice
omnigate does not claim finality
omnigate cannot unlock paid content from replay artifacts alone
paid unlock still requires backend wallet/ledger truth
public v1 route registry does not expose verifier/committee/finality/bridge/staking families
v1 route source does not construct verifier results, replay results, committee attestations, quorum, fork-choice, finality, anchored, or settlement-finality statuses
Cargo.toml does not gain direct ledger or external settlement dependencies
existing paid routes remain backend wallet/ledger-derived, not replay-artifact unlocks
```

The focused test passed:

```text
cargo test -p omnigate --test quickchain_phase2_replay_boundary

running 5 tests
omnigate_does_not_gain_direct_ledger_or_external_settlement_dependencies ... ok
docs_name_phase2_round1_omnigate_read_only_replay_boundary ... ok
omnigate_routes_do_not_expose_verifier_committee_or_finality_authority_paths ... ok
existing_paid_routes_remain_backend_wallet_ledger_derived_not_replay_artifact_unlocks ... ok
omnigate_source_does_not_construct_phase2_authority_results ... ok

test result: ok. 5 passed; 0 failed
```

---

## 4. Documentation Updated

Updated:

```text
crates/omnigate/docs/quickchain-preflight.md
```

The doc now explicitly names:

```text
Phase 2 Round 1 verifier artifact / read-only replication boundary
```

The doc now preserves these boundaries:

```text
omnigate may expose read-only proof/replay artifact views if needed
omnigate replay metadata is display and hydration context only
replay artifacts are not omnigate authority
proof artifacts are not omnigate authority
verifier results are not omnigate authority
committee attestations are not omnigate authority
quorum claims are not omnigate authority
fork-choice claims are not omnigate authority
finality claims are not omnigate authority
omnigate is not verifier truth
omnigate is not replay truth
omnigate is not quorum truth
omnigate is not committee truth
omnigate does not sign verifier attestations
omnigate does not decide fork choice
omnigate does not claim finality
omnigate cannot unlock paid content from replay artifacts alone
paid unlock still requires backend wallet/ledger truth
svc-wallet remains the paid mutation path
ron-ledger remains durable economic truth
```

---

## 5. Final Gate Evidence

The final local gate discovered 11 focused QuickChain tests:

```text
quickchain_phase1_round2_confirmation
quickchain_phase2_replay_boundary
quickchain_preflight_boundary
quickchain_preflight_cache_boundary
quickchain_preflight_docs
quickchain_preflight_no_fake_receipts
quickchain_preflight_paid_access
quickchain_preflight_phase1_pair_interlock
quickchain_preflight_transport_authority
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
```

The final parking markers were:

```text
== omnigate quickchain exhaustive preflight gate passed: tests=11 ==
== omnigate QuickChain parking gate passed ==
```

The all-targets gate completed cleanly:

```text
cargo test -p omnigate --all-targets
```

The clippy gate completed cleanly:

```text
cargo clippy -p omnigate --all-targets --no-deps -- -D warnings
```

The forbidden-scope marker stayed parked:

```text
no roots
no checkpoints
no validators
no bridges
no external settlement
no fake receipts
no fake balances
```

---

## 6. Paid/Product Regression Coverage Passed

The park script also ran paid/product route regressions successfully:

```text
content_view
site_visit
streams
chat_routes
paid_storage_estimate_proxy
paid_storage_prepare
paid_storage_write_proxy
```

This is important because Phase 2 Round 1 must not accidentally weaken paid access or create replay-artifact-only unlocks.

The successful regression coverage preserved:

```text
content view quote/pay still uses wallet-derived receipt path
site visit quote/pay still uses wallet-derived receipt path
streams latest segment still requires stream-bound wallet receipt
chat paid send still requires quote
paid storage estimate/prepare/write remain backend/storage/wallet-derived paths
upstream errors pass through safely
no replay/proof/verifier artifact becomes paid unlock authority
```

---

## 7. Important Existing Coverage Still Preserved

This pass did not replace older omnigate safety coverage. It extended it.

Still protected:

```text
omnigate remains hydrator and coordinator, not chain runtime
Cargo.toml does not depend on ron-ledger or QuickChain runtime crates
public v1 router does not publish QuickChain authority routes
wallet surface is display balance and explicit hold front door only
source does not import direct ledger or chain authority
cache and b3 are not paid entitlement authority
paid access routes do not parse cache headers as unlock authority
production source does not claim cache/b3/manifest proves payment
wallet balance fallback is explicitly non-authoritative
paid routes use wallet receipt values without dev receipt fabrication
source does not construct root or validator authority fields
source does not construct obvious fake receipts or finality
paid storage remains prepare/estimate/proxy only
content view and site visit pay routes name wallet front-door invariant
stream and chat paid paths fail closed on backend truth
source does not contain paid or unlock bypass shortcuts
x-ron header policy blocks QuickChain authority headers
transport headers are not economic authority
receipt or paid authority must come from backend paths, not request headers
accepted receipt is separated from future finality status
```

---

## 8. Safe Meaning

The safe completion meaning for this crate is:

```text
omnigate is green/parked for QuickChain Phase 2 Round 1.
It can hydrate or display read-only replay/proof/verifier context if future routes need it,
but it cannot become verifier, replay, quorum, committee, fork-choice, finality, settlement,
wallet, ledger, bridge, staking, or paid-unlock authority.
```

---

## 9. What Not To Claim

Do not claim:

```text
Phase 2 complete
Phase 2 Round 1 complete across all crates
committee ready
validator ready
quorum ready
fork choice ready
finality ready
omnigate verifier views are live
omnigate can verify roots
omnigate can sign attestations
omnigate can unlock paid content from replay artifacts
omnigate can mutate wallet
omnigate can mutate ledger
bridge ready
staking/slashing ready
external settlement ready
ROX/Solana runtime ready
```

---

## 10. Do Not Reopen Unless

Do not reopen `omnigate` during Phase 2 Round 1 unless:

```text
a later crate-pair exposes a real mismatch in verifier artifact hydration/display boundaries
svc-index or ron-policy introduces a replay artifact pointer/policy shape requiring omnigate confirmation
CrabLink client adapters need a stricter omnigate status shape
svc-wallet receipt or paid-admission contracts change
ron-ledger economic truth or receipt format changes
svc-gateway route/header contracts change
QuickChain verifier artifact schemas change in ron-proto
a parking gate regresses
a source/header scanner catches new authority creep
```

Otherwise, `omnigate` should stay parked until Phase 2 Round 2 or a later explicitly scoped phase.

---

## 11. Next Pair

The next Phase 2 Round 1 crate pair is:

```text
svc-index + ron-policy
```

Expected goal for that pair:

```text
svc-index may point to read-only replay/proof artifacts but cannot become proof authority.
ron-policy may declare verifier access policy but cannot create verifier truth, settlement truth, finality, or paid-unlock authority.
```

### END NOTE - JUNE 23 2026 - QUICKCHAIN PHASE 2 ROUND 1 - omnigate

### END NOTE - JUNE 23 2026 - 21:35 CST 


### BEGIN NOTE - JUNE 24 2026 - 02:00 CST

Based on the final terminal output, `svc-gateway` hit the full parking marker after clippy passed, and `omnigate` had already parked in the previous run. 

Paste this into `crates/svc-gateway/NOTES.MD`:

### BEGIN NOTE - JUNE 24 2026 - QUICKCHAIN PHASE 2 ROUND 2

# svc-gateway — QuickChain Phase 2 Round 2 Crate Notes

## Status

`svc-gateway` is complete / parked for QuickChain Phase 2 Round 2.

This round hardened `svc-gateway` as a Phase 2 public boundary for future committee/verifier metadata while preserving the core invariant:

```text
svc-gateway is a public ingress/proxy/admission surface.
svc-gateway is not ledger truth.
svc-gateway is not wallet mutation authority.
svc-gateway is not a QuickChain verifier.
svc-gateway is not committee authority.
svc-gateway is not quorum/finality/settlement authority.
```

The crate now has explicit Phase 2 Round 2 coverage proving that committee, quorum, finality, replay, verifier, bridge, staking, liquidity, and external-settlement claims cannot be smuggled through public headers, public route names, direct dependencies, or source-level status construction.

## Files changed / hardened

Primary runtime hardening:

```text
crates/svc-gateway/src/headers/proxy.rs
```

This header proxy policy now rejects authority-looking QuickChain / chain / committee / quorum / finality headers before forwarding upstream. The deny surface includes exact and prefix families such as:

```text
x-quickchain-*
x-qc-*
x-ron-quickchain-*
x-ron-replay-*
x-ron-verifier-*
x-ron-committee-*
x-ron-quorum-*
x-ron-validator-*
x-ron-checkpoint-*
x-ron-root-*
x-ron-ledger-*
x-ron-proof-*
x-ron-settlement-*
x-ron-anchor-*
x-ron-bridge-*
```

It still allows ordinary gateway/product context headers that are needed for current ROC paid product flows, such as:

```text
authorization
accept
content-type
x-correlation-id
x-request-id
idempotency-key
x-ron-wallet-account
x-ron-passport
x-ron-wallet-txid
x-ron-wallet-receipt-hash
x-ron-paid-op
x-ron-paid-asset
```

New Phase 2 Round 2 test:

```text
crates/svc-gateway/tests/quickchain_phase2_committee_boundary.rs
```

Docs hardened:

```text
crates/svc-gateway/docs/quickchain-preflight.md
```

## New test coverage

New focused test suite:

```text
cargo test -p svc-gateway --test quickchain_phase2_committee_boundary
```

Coverage added:

```text
- docs name the Phase 2 Round 2 gateway committee readiness boundary
- gateway filters committee/quorum/finality/external QuickChain authority headers
- gateway still allows backend product context without promoting it to finality
- gateway does not gain direct ledger/wallet/validator/external settlement dependencies
- gateway routes do not expose committee/quorum/validator/bridge authority paths
- gateway source does not construct committee or finality status truth
```

This locks `svc-gateway` into the correct future Phase 2 position:

```text
backend-derived verifier or committee labels may eventually be routed/displayed,
but gateway itself must not create, sign, decide, finalize, settle, reward, anchor, or unlock.
```

## Gates passed

Focused Phase 2 Round 2 test:

```text
cargo test -p svc-gateway --test quickchain_phase2_committee_boundary
```

Result:

```text
6 passed; 0 failed
```

Park runner:

```text
crates/svc-gateway/scripts/dev-quickchain-park.sh
```

The dynamic QuickChain discovery runner picked up the new suite and reran the existing boundary tests.

Confirmed passing suites included:

```text
quickchain_phase1_round2_confirmation
quickchain_phase2_committee_boundary
quickchain_phase2_replay_boundary
quickchain_preflight_boundary
quickchain_preflight_cache_boundary
quickchain_preflight_docs
quickchain_preflight_no_fake_receipts
quickchain_preflight_paid_access
quickchain_preflight_phase1_pair_interlock
quickchain_preflight_transport_authority
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
```

Product/proxy regressions passed, including:

```text
app_proxy
paid_storage_estimate_proxy
paid_storage_write_proxy
product_routes_proxy
site_visit_routes_proxy
content_view_routes_proxy
identity_routes_proxy
smoke
```

Hard gates passed:

```text
cargo fmt -p svc-gateway
cargo test -p svc-gateway --all-targets
cargo clippy -p svc-gateway --all-targets --no-deps -- -D warnings
```

Final parking marker reached:

```text
svc-gateway QuickChain preflight gate passed
== svc-gateway quickchain exhaustive preflight gate passed: tests=12 ==
== svc-gateway QuickChain parking gate passed ==
```

## Small repair made during parking

The initial park run failed only on a clippy `doc_markdown` lint in:

```text
crates/svc-gateway/src/headers/proxy.rs
```

The fix was to wrap `QuickChain` in backticks inside the RO header comment. After that, `cargo fmt`, the dynamic QuickChain preflight, all-targets, regressions, and clippy passed.

## Boundary summary

`svc-gateway` may:

```text
- expose stable public browser/client routes
- proxy selected requests to omnigate
- preserve ordinary product context headers
- strip authority-looking headers
- route backend-derived product responses
- return backend-derived/display-only receipt context
```

`svc-gateway` must not:

```text
- mutate ron-ledger
- bypass svc-wallet
- create fake balances
- create fake receipts
- unlock paid content from cache/client/header claims
- expose live QuickChain authority routes
- create verifier attestations
- decide committee membership
- decide quorum
- decide finality
- decide fork choice
- anchor externally
- bridge externally
- implement staking/liquidity/slashing
- act as validator or settlement runtime
```

## Completion statement

`svc-gateway` is parked for QuickChain Phase 2 Round 2.

Next time `svc-gateway` is touched for QuickChain, preserve this role:

```text
Gateway routes/display metadata only.
Gateway strips authority smuggling.
Gateway never becomes settlement authority.
Gateway never becomes wallet/ledger/validator/quorum/finality authority.
```

### END NOTE - JUNE 24 2026 - QUICKCHAIN PHASE 2 ROUND 2

Paste this into `crates/omnigate/NOTES.MD`:

### BEGIN NOTE - JUNE 24 2026 - QUICKCHAIN PHASE 2 ROUND 2

# omnigate — QuickChain Phase 2 Round 2 Crate Notes

## Status

`omnigate` is complete / parked for QuickChain Phase 2 Round 2.

This round hardened `omnigate` as a product hydrator / coordination boundary for future verifier or committee metadata while preserving the core invariant:

```text
omnigate hydrates product views.
omnigate coordinates backend calls.
omnigate is not ledger truth.
omnigate is not wallet mutation authority.
omnigate is not a QuickChain verifier.
omnigate is not committee authority.
omnigate is not quorum/finality/settlement authority.
```

The crate now has explicit Phase 2 Round 2 coverage proving that committee, verifier, quorum, finality, replay, validator, bridge, staking, liquidity, and external-settlement claims cannot be smuggled through headers, public route names, paid routes, source-level hydration logic, or direct dependencies.

## Files changed / hardened

Primary runtime hardening:

```text
crates/omnigate/src/routes/v1/header_policy.rs
```

The shared header policy now rejects authority-looking QuickChain / chain / committee / quorum / finality headers before route code treats forwarded `x-ron-*` context as safe product context.

The deny surface includes exact and prefix families such as:

```text
x-quickchain-*
x-qc-*
x-ron-quickchain-*
x-ron-replay-*
x-ron-verifier-*
x-ron-committee-*
x-ron-quorum-*
x-ron-validator-*
x-ron-checkpoint-*
x-ron-root-*
x-ron-ledger-*
x-ron-proof-*
x-ron-settlement-*
x-ron-anchor-*
x-ron-bridge-*
```

The policy still preserves ordinary product/value-loop context headers needed for current ROC flows, such as:

```text
x-ron-wallet-account
x-ron-passport
x-ron-wallet-txid
x-ron-wallet-receipt-hash
x-ron-paid-op
x-ron-paid-asset
```

New Phase 2 Round 2 test:

```text
crates/omnigate/tests/quickchain_phase2_committee_boundary.rs
```

Docs hardened:

```text
crates/omnigate/docs/quickchain-preflight.md
```

## New test coverage

New focused test suite:

```text
cargo test -p omnigate --test quickchain_phase2_committee_boundary
```

Coverage added:

```text
- docs name the Phase 2 Round 2 omnigate committee readiness boundary
- omnigate header policy blocks committee/quorum/finality/external QuickChain headers
- omnigate routes do not expose committee/quorum/validator/bridge authority paths
- omnigate does not gain direct ledger/validator/external settlement dependencies
- existing paid routes remain backend wallet/ledger-derived, not committee unlocks
- omnigate hydration and paid routes do not construct committee or finality status truth
```

This locks `omnigate` into the correct future Phase 2 position:

```text
backend-derived verifier or committee labels may eventually be hydrated/displayed,
but omnigate itself must not create, sign, decide, finalize, settle, reward, anchor, or unlock.
```

## Gates passed

Focused Phase 2 Round 2 test:

```text
cargo test -p omnigate --test quickchain_phase2_committee_boundary
```

Result:

```text
6 passed; 0 failed
```

Park runner:

```text
crates/omnigate/scripts/dev-quickchain-park.sh
```

The dynamic QuickChain discovery runner picked up the new suite and reran the existing boundary tests.

Confirmed passing suites included:

```text
quickchain_phase1_round2_confirmation
quickchain_phase2_committee_boundary
quickchain_phase2_replay_boundary
quickchain_preflight_boundary
quickchain_preflight_cache_boundary
quickchain_preflight_docs
quickchain_preflight_no_fake_receipts
quickchain_preflight_paid_access
quickchain_preflight_phase1_pair_interlock
quickchain_preflight_transport_authority
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
```

Paid/product route regressions passed, including:

```text
content_view
site_visit
streams
chat_routes
paid_storage_estimate_proxy
paid_storage_prepare
paid_storage_write_proxy
```

All-targets passed, including route, hydration, product, policy, readiness, site, stream, text, and zk read-only suites.

Hard gates passed:

```text
cargo fmt -p omnigate
cargo test -p omnigate --all-targets
cargo clippy -p omnigate --all-targets --no-deps -- -D warnings
```

Final parking marker reached:

```text
omnigate QuickChain preflight gate passed
== omnigate quickchain exhaustive preflight gate passed: tests=12 ==
== omnigate QuickChain parking gate passed ==
```

## Boundary summary

`omnigate` may:

```text
- hydrate product views
- coordinate backend calls
- proxy paid storage prepare/write flows
- call backend wallet-facing routes through the approved front-door path
- display backend-derived receipt and paid-access context
- preserve product context headers after policy filtering
- eventually hydrate backend-derived verifier/committee labels as display-only metadata
```

`omnigate` must not:

```text
- mutate ron-ledger directly
- bypass svc-wallet
- create fake balances
- create fake receipts
- unlock paid content from cache/client/header claims
- expose live QuickChain authority routes
- create signed verifier attestations
- decide committee membership
- decide quorum
- decide finality
- decide fork choice
- anchor externally
- bridge externally
- implement staking/liquidity/slashing
- act as validator or settlement runtime
```

## Completion statement

`omnigate` is parked for QuickChain Phase 2 Round 2.

Next time `omnigate` is touched for QuickChain, preserve this role:

```text
Omnigate hydrates backend-derived truth.
Omnigate filters authority smuggling.
Omnigate coordinates product access.
Omnigate never becomes wallet/ledger/validator/quorum/finality authority.
```

### END NOTE - JUNE 24 2026 - QUICKCHAIN PHASE 2 ROUND 2


### END NOTE - JUNE 24 2026 - 02:00 CST