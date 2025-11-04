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