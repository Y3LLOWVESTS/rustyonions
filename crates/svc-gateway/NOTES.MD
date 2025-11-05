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