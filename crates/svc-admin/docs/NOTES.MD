### BEGIN NOTE - DECEMBER 4 2025 - 22:30 CST

Here are full carryover notes for **svc-admin** as of now.

---

## 0. Quick status snapshot

* Crate: `crates/svc-admin`
* Build: ✅ `cargo test -p svc-admin --tests`
* Runtime: ✅ `cargo run -p svc-admin --bin svc-admin`
* Current listeners:

  * UI/API: `127.0.0.1:5300`
  * Health/metrics: `127.0.0.1:5310`
* Confirmed behavior via curl:

  * `GET /healthz` → `ok`
  * `GET /readyz` → `{"ready":true}`
  * `GET /api/ui-config` → real config-derived DTO
  * `GET /api/me` → dev stub identity
  * `GET /api/nodes` → registry-derived `NodeSummary` (example node)
  * `GET /api/nodes/example-node/status` → placeholder `AdminStatusView` with `profile:"macronode", version:"0.0.0", planes:[]`

We’ve completed a **Phase 1 vertical slice**: svc-admin boots, has real config, a dual-port Axum server, basic health/readiness, UI config & identity endpoints, and a config-driven node registry exposed via `/api/nodes` and `/api/nodes/:id/status`.

---

## 1. Crate-level setup & dependencies

### 1.1 `Cargo.toml`

* `svc-admin` is a normal service crate (not published):

  * `publish = false`
  * `edition = 2021`
  * `license = "MIT OR Apache-2.0"`
* Binary + lib:

  * `[lib]` name `svc_admin`, `src/lib.rs`
  * `[[bin]]` name `svc-admin`, `src/bin/svc-admin.rs`
* Features:

  * `default = []` (no accidental “feature includes non-optional deps” issues)
  * `tls = ["tokio-rustls"]`
  * `otel = []` (placeholder)
  * `passport = []` (placeholder)
* Core deps (aligned with workspace HTTP stack):

  * `tokio = { version = "1", features = ["rt-multi-thread", "macros", "time", "signal"] }`
  * `axum = { version = "0.7", default-features = false, features = ["tokio", "http1", "http2", "json"] }`
  * `tower`, `tower-http = { version = "0.6", features = ["trace", "cors"] }`
  * `reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls-native-roots"] }`
  * `tokio-rustls = { version = "0.26", optional = true }`
  * Observability: `tracing`, `tracing-subscriber = { features = ["fmt", "env-filter"] }`, `prometheus = "0.14"`
  * Serialization: `serde` (+derive), `serde_json`
  * Error handling: `anyhow`, `thiserror`
* Dev-deps mirror runtime (tokio + reqwest) so tests can run HTTP calls against the service.

**Takeaway:** svc-admin is now wired to the same modern, rustls-only HTTP stack as the rest of RON-CORE, with clean, minimal features and no native-tls.

---

## 2. Config system

### 2.1 Types

`src/config.rs` defines:

* `Config`:

  * `server: ServerCfg`
  * `auth: AuthCfg`
  * `ui: UiCfg`
  * `nodes: NodesCfg` (config-driven node registry)
* `ServerCfg`:

  * `bind_addr: String` (UI/API)
  * `metrics_addr: String` (health/metrics)
* `AuthCfg`:

  * `mode: String` (`"none" | "ingress" | "passport"`)
* `UiCfg`:

  * `default_theme: String`
  * `default_language: String`
  * `read_only: bool`
* `NodeCfg`:

  * `base_url: String` (admin plane base URL)
  * `display_name: Option<String>`
  * `environment: String` (e.g., `"dev"`, `"staging"`, `"prod"`)
  * `insecure_http: bool`
* `NodesCfg = BTreeMap<String, NodeCfg>`:

  * `key = node_id` (logical name like `"example-node"`)

### 2.2 Loading semantics (`Config::load()`)

* **Guardrail**:

  * If `SVC_ADMIN_CONFIG` is set, we **fail fast** with `Error::Config`, because file-based config isn’t implemented yet. This prevents silently ignoring a path that ops might think is used.
* Env keys and defaults:

  * `SVC_ADMIN_BIND_ADDR` → default `"127.0.0.1:5300"`
  * `SVC_ADMIN_METRICS_ADDR` → default `"127.0.0.1:5310"`
  * `SVC_ADMIN_AUTH_MODE` → default `"none"`

    * Validated via `load_auth_mode` against `["none","ingress","passport"]`.
  * `SVC_ADMIN_UI_DEFAULT_THEME` → default `"light"`
  * `SVC_ADMIN_UI_DEFAULT_LANGUAGE` → default `"en-US"`
  * `SVC_ADMIN_UI_READ_ONLY` → parsed via `load_bool`, default `true`

    * Accepts `true/false`, `1/0`, `yes/no`, `y/n` (case-insensitive).
* Node seed (dev convenience):

  * We **seed one node** by default:

    * ID: `"example-node"`
    * `base_url` from `SVC_ADMIN_EXAMPLE_NODE_URL` or default `"http://127.0.0.1:9000"`
    * `display_name = Some("Example Node")`
    * `environment` from `SVC_ADMIN_EXAMPLE_NODE_ENV` or `"dev"`
    * `insecure_http = true`
  * This ensures `/api/nodes` and `/api/nodes/example-node/status` always return meaningful data even with zero config, which is perfect for local dev & early UI work.

### 2.3 Helper functions

* `load_addr(key, default) -> Result<String>`:

  * Reads env, falls back to default.
  * Tries to parse as `SocketAddr` to fail early on invalid addresses.
  * Returns the original string if parsing succeeds.
* `load_bool(key, default) -> Result<bool>`:

  * Accepts `1/0`, `true/false`, `yes/no`, `y/n` (case-insensitive).
  * Returns default if env var is unset.
* `load_auth_mode(key, default) -> Result<String>`:

  * Enforces allowed auth modes (`none`, `ingress`, `passport`).
  * Fails with `Error::Config` on invalid values.

### 2.4 CLI integration

* `src/cli.rs`:

  * `pub fn parse_args() -> anyhow::Result<Config>` simply calls `Config::load()`.
  * For now there are no actual CLI flags; future work is to introduce `clap` and implement precedence: CLI → env → file → defaults.

**Takeaway:** Config is robust for dev-preview: env-driven, validated addresses and auth mode, one seeded node, and clear guardrails for future TOML support.

---

## 3. Observability & server bootstrap

### 3.1 `observability.rs`

* `init_tracing()`:

  * Uses `tracing_subscriber::fmt` with `EnvFilter`.
  * `EnvFilter` comes from `RUST_LOG` if set, or defaults to something like:

    * `info,svc_admin=info,axum=warn,tower_http=warn`
  * Calls `.init()` via `SubscriberInitExt` (trait imported correctly).
* This ensures *every* svc-admin run (including tests that spawn it) has structured logs.

### 3.2 `server.rs`

* `pub async fn run(config: Config) -> Result<()>`:

  * Calls `observability::init_tracing()` once.
  * Builds `AppState::new(config.clone())`, wraps it in `Arc`.
  * Builds the router via `router::build_router(state.clone())`.
  * Parses the two bind addrs from config:

    * `bind_addr` (UI/API)
    * `metrics_addr` (health/metrics)
  * Binds:

    * `main_listener = TcpListener::bind(bind_addr).await?`
    * `metrics_listener = TcpListener::bind(metrics_addr).await?`
  * Logs:

    * `svc-admin listening for UI/API bind_addr=...`
    * `svc-admin listening for health/metrics metrics_addr=...`
  * Spawns **metrics listener**:

    * `tokio::spawn(axum::serve(metrics_listener, metrics_app))`
    * For now reuses the same router (`metrics_app = app.clone()`), so all routes are available on both ports.
  * Serves the **main listener** with graceful shutdown:

    * `axum::serve(main_listener, app).with_graceful_shutdown(shutdown_signal())`
  * `shutdown_signal()` uses `tokio::signal::ctrl_c()` and logs `"shutdown signal received"`.

* `pub async fn run_server(config: Config) -> Result<()>`:

  * Alias for `run`, matching the name used in docs/README.

**Takeaway:** server boot/shutdown is clean, dual-port, and uses the shared router. We still need to split out a dedicated metrics router later (for Prometheus exposition + minimal surface on metrics port).

---

## 4. Shared state & node registry

### 4.1 `state.rs`

* `pub struct AppState`:

  * `config: Config`
  * `nodes: NodeRegistry`
* `impl AppState::new(config: Config) -> Self`:

  * Constructs `NodeRegistry::new(&config.nodes)` from the loaded config.
  * Stores the Config and registry.

### 4.2 `nodes/registry.rs`

* `pub struct NodeRegistry`:

  * Wraps `BTreeMap<String, NodeCfg>` (copied from `NodesCfg`).
* `pub fn new(cfg: &NodesCfg) -> Self`:

  * Clones the map from config to local in-memory registry.
* `pub fn list_summaries(&self) -> Vec<NodeSummary>`:

  * Iterates over `nodes` map.
  * Produces `NodeSummary`:

    * `id`: key
    * `display_name`: `cfg.display_name.unwrap_or(id.clone())`
    * `profile: None` for now
* `pub async fn get_status(&self, id: &str) -> Option<AdminStatusView>`:

  * Looks up `NodeCfg` by id.
  * Uses `status::build_status_placeholder()` to create fallback `AdminStatusView`.
  * Overwrites:

    * `view.id = id.to_string()`
    * `view.display_name = cfg.display_name.unwrap_or(id.to_string())`
  * Returns `Some(view)` on success, `None` if the node isn’t in the registry.
* `pub fn contains(&self, id: &str) -> bool`:

  * Convenience check used for 404 conditions in future.

**Takeaway:** we now have a real, config-driven node registry as the source of truth for `/api/nodes` and `/api/nodes/:id/status`.

---

## 5. DTOs & HTTP surface

### 5.1 `router.rs`

Current routes:

```text
/healthz                     GET
/readyz                      GET
/api/ui-config               GET
/api/me                      GET
/api/nodes                   GET
/api/nodes/:id/status        GET
```

Handlers:

* `healthz` → `"ok"`
* `readyz` → `{"ready": true}` (hardcoded for now, but plumbed with `AppState` so we can gate on readiness later).
* `ui_config`:

  * Uses `dto::ui::UiConfigDto::from_cfg(&state.config)`.
* `me`:

  * Returns `dto::me::MeResponse::dev_default()`.
* `nodes`:

  * Calls `state.nodes.list_summaries()` and returns `Vec<NodeSummary>`.
* `node_status`:

  * Takes `Path<String>` (`id`) and `State<Arc<AppState>>`.
  * Calls `state.nodes.get_status(&id).await`.

    * `Some(view)` → `200 OK` with `AdminStatusView`.
    * `None` → `404 NOT_FOUND`.

### 5.2 DTOs (inferred from behavior)

* `UiConfigDto`:

  * Response example:

    ```json
    {
      "default_theme":"light",
      "available_themes":["light","dark"],
      "default_language":"en-US",
      "available_languages":["en-US","es-ES"],
      "read_only":true
    }
    ```

  * Implementation: `from_cfg(&Config)` merges:

    * `default_theme` from config.
    * `default_language` from config.
    * Hardcoded `available_themes = ["light","dark"]` and `available_languages = ["en-US","es-ES"]` for now.
* `MeResponse`:

  * Response example:

    ```json
    {
      "subject":"dev-operator",
      "display_name":"Dev Operator",
      "roles":["admin"],
      "auth_mode":"none",
      "login_url":null
    }
    ```

  * `dev_default()` is hardcoded for now and assumes `auth.mode="none"`.
* `NodeSummary`:

  * `/api/nodes` response snippet:

    ```json
    [
      {
        "id": "example-node",
        "display_name": "Example Node",
        "profile": null
      }
    ]
    ```

  * `profile` is `Option<String>` and currently `None` (null).
* `AdminStatusView` (placeholder from `/api/nodes/example-node/status`):

  ```json
  {
    "id":"example-node",
    "display_name":"Example Node",
    "profile":"macronode",
    "version":"0.0.0",
    "planes":[]
  }
  ```

  * `status::build_status_placeholder()` likely sets:

    * `id = "example-node"` initially (then overwritten by registry)
    * `display_name = "Example Node"`
    * `profile = "macronode"`
    * `version = "0.0.0"`
    * `planes = []`

**Takeaway:** The API contract for the core endpoints is real and consumable. We still need to document them fully in `docs/API.MD` (if not already) and keep DTOs in sync with that doc.

---

## 6. Tests

### 6.1 `tests/http_smoke.rs`

* Bootstraps a **real** svc-admin instance in-process with a custom `Config`:

  * `bind_addr = "127.0.0.1:5300"`
  * `metrics_addr = "127.0.0.1:5310"`
  * Auth/UI set to simple defaults.
  * `nodes` map is explicitly filled with one `NodeCfg` (example-node).
* Spawns `server::run(cfg)` in a background task.
* Sleeps 200ms (simple stability hack; we may want a more robust ready probe later).
* Calls `reqwest::get("http://127.0.0.1:5310/healthz")` and asserts body is `"ok"`.

Result:

* The test passes cleanly (confirmed in your terminal output).
* fake_node integration test file exists but **defines zero tests** right now.

**Takeaway:** We have a simple but solid end-to-end smoke test around server boot and `/healthz` on the metrics/health port. We still need tests for `/readyz`, `/api/ui-config`, `/api/me`, `/api/nodes`, `/api/nodes/:id/status`, and eventually fake-node interop.

---

## 7. What remains (big-ticket items)

Here’s what’s still outstanding, grouped roughly by the phases in your TODO.

### 7.1 Phase 1 / 1.5 – Finish core backend shape

* **Config:**

  * Implement real **TOML file loading** (`SVC_ADMIN_CONFIG` and/or `--config`).
  * Add any remaining fields promised in `docs/CONFIG.MD` (timeouts, TLS, per-node auth tokens, etc.).
  * Implement precedence: CLI → env → file → defaults.
* **Error taxonomy (`error.rs`):**

  * Ensure the `Error` enum and `Result` alias cover:

    * Config errors
    * IO / HTTP
    * Auth errors
    * Upstream node errors
  * Align with `docs/API.MD` and `docs/SECURITY.MD`.

### 7.2 Phase 2 – Node admin client + real status & metrics sampling

* **Node HTTP client (`nodes/client.rs`):**

  * Create a `NodeClient` that:

    * Constructs URLs based on `NodeCfg.base_url`.
    * Hits admin endpoints: `/healthz`, `/readyz`, `/version`, `/metrics`, `/api/v1/status`.
    * Applies timeouts and TLS/insecure_http logic per config.
  * Handle HTTP/JSON errors and map to `Error::UpstreamNode` (or similar).
* **Node status normalization (`nodes/status.rs`):**

  * Replace `build_status_placeholder()` with real logic that:

    * Ingests raw responses from nodes.
    * Normalizes them into `AdminStatusView` and `PlaneStatus` objects.
    * Enforces “truthful readiness” invariant: `/readyz` is the source of truth.
* **Metrics sampling (`metrics/*`):**

  * Implement:

    * `metrics/sampler.rs`: background tasks that scrape `/metrics` and `/api/v1/status` periodically.
    * `metrics/facet.rs`: group metrics by facet (`ron_facet_*` labels).
    * `metrics/prometheus_bridge.rs`: take sampled metrics & facet summaries and expose them as Prometheus metrics for svc-admin itself.
  * Expose facet-aware DTOs (`dto::metrics`) for the SPA to render.

### 7.3 Phase 3 – Auth modes & observability hardening

* **Auth (`auth/*`):**

  * `auth::none`: retain or refine dev-mode identity for local/dev only.
  * `auth::ingress`: trust specific headers from an ingress/proxy (`X-User`, `X-Roles`), with strict validation and threat model from `SECURITY.MD`.
  * `auth::passport`: validate JWT/passport tokens, cache JWKS, map claims to `MeResponse`.
  * Update handlers (`/api/me`, mutating endpoints later) to respect roles/scopes.
* **Observability:**

  * Implement `GET /metrics` for svc-admin’s own Prometheus metrics.
  * Decide whether `/metrics` is only on the metrics port, or on both.
  * Add structured logging for admin actions (when we add mutating APIs).
  * Wire OpenTelemetry (`otel` feature) if/when required.

### 7.4 Phase 4 – UI / SPA (Vite + React + TS)

Right now the `ui/` tree is largely scaffold + stubs. We haven’t touched it in this slice.

Planned work:

* Set up **Vite + React + TS** in `ui/`:

  * `package.json` with scripts: `dev`, `build`, `lint`, `test`.
  * `tsconfig.json`, `vite.config.ts`, ESLint + Prettier configs.
* Implement SPA core:

  * `src/main.tsx`, `src/App.tsx` wiring Router, ThemeProvider, I18n provider.
* Layout components:

  * `Shell`, `Sidebar`, `TopBar`, `ThemeToggle`, `LanguageSwitcher`.
* Pages:

  * `NodeListPage` using `/api/nodes`.
  * `NodeDetailPage` using `/api/nodes/:id/status` and facet metrics DTOs once they’re real.
  * `SettingsPage` consuming `/api/ui-config`.
  * `LoginPage` consuming `/api/me` with `login_url` when in passport mode.
* Shared components: loading spinner, error banner, empty state.

### 7.5 Phase 5 – UI ↔ backend integration & tests

* **API client (`ui/src/api/adminClient.ts`):**

  * Typed fetch functions for:

    * `getUiConfig()`
    * `getMe()`
    * `listNodes()`
    * `getNodeStatus(id)`
    * Later: facet metrics, admin actions.
* **Types (`ui/src/types/admin-api.ts`):**

  * Keep TS types in lock-step with Rust DTOs.
* **Templates & facet panels:**

  * `FacetMetricsPanel`, `FacetMetricsTemplate` consuming future facet DTOs.
* **Tests:**

  * Integration tests in `tests/` that:

    * Bring up a fake node.
    * Confirm normalization into `AdminStatusView` & metrics DTOs.
    * Validate auth flows (`none`, `ingress`, `passport`) once implemented.

---

## 8. How to run and test in the future

### 8.1 Run svc-admin locally

From repo root:

```bash
cargo run -p svc-admin --bin svc-admin
```

Optional env overrides:

```bash
SVC_ADMIN_BIND_ADDR=127.0.0.1:5300 \
SVC_ADMIN_METRICS_ADDR=127.0.0.1:5310 \
SVC_ADMIN_AUTH_MODE=none \
SVC_ADMIN_UI_DEFAULT_THEME=dark \
SVC_ADMIN_UI_DEFAULT_LANGUAGE=en-US \
SVC_ADMIN_UI_READ_ONLY=true \
SVC_ADMIN_EXAMPLE_NODE_URL=http://127.0.0.1:9000 \
SVC_ADMIN_EXAMPLE_NODE_ENV=dev \
cargo run -p svc-admin --bin svc-admin
```

Then from another terminal:

```bash
curl http://127.0.0.1:5310/healthz
curl http://127.0.0.1:5300/readyz
curl http://127.0.0.1:5300/api/ui-config
curl http://127.0.0.1:5300/api/me
curl http://127.0.0.1:5300/api/nodes
curl http://127.0.0.1:5300/api/nodes/example-node/status
```

### 8.2 Run tests

```bash
cargo test -p svc-admin --tests
```

Currently runs:

* `tests/http_smoke.rs` (1 passing test).
* `tests/fake_node.rs` (empty; we’ll populate later).

---

## 9. Suggested next moves (when we resume svc-admin)

If we pick up where we left off, here’s a good, high-impact next sequence:

1. **Finish `NodeRegistry` vertical:**

   * Add a real `status::NodeClient` that hits `/healthz`, `/readyz`, `/version`, `/metrics`, `/api/v1/status`.
   * Replace `build_status_placeholder()` with real aggregation logic.
   * Add tests around `AdminStatusView` normalization (including edge cases: node down, partial data, malformed metrics).
2. **Implement `metrics/sampler.rs` + `metrics/facet.rs`:**

   * Scrape `/metrics` and group `ron_facet_*` metrics per facet.
   * Store a small, rolling window in memory (e.g., last N samples).
   * Expose a cheap `/api/nodes/:id/facet-metrics` DTO.
3. **Add `/metrics` endpoint for svc-admin itself**:

   * Export metrics about:

     * node fan-out success/failures
     * admin actions (later)
     * sampling lag, etc.
4. **Wire `auth.mode` into `/api/me` and add basic `auth::none` vs. `auth::ingress` skeleton**:

   * Even a minimal ingress-mode stub would be enough to start testing the shape.

Once that’s done, we’ll be in a really good spot to jump into the SPA and start making **svc-admin actually look like the God-tier dashboard we’ve specced out**, with real data behind `/api/nodes` and `/api/nodes/:id/status`.

These notes should be enough for the next instance to rehydrate context and continue building svc-admin without losing any of the decisions we’ve made so far.


### END NOTE - DECEMBER 4 2025 - 22:30 CST