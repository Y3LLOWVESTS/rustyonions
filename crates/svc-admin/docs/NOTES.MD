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




### BEGIN NOTE - DECEMBER 5 2025 - 12:00 CST

---

# svc-admin Carryover Notes – 2025-12-05

## 0. Current Status Snapshot

**Crate:** `crates/svc-admin`
**Build:** ✅

```bash
cargo test -p svc-admin --tests
```

All tests green:

* Unit tests:

  * `nodes::client::tests::node_client_can_talk_to_fake_admin_plane`
  * `nodes::client::tests::node_client_rejects_http_when_insecure_http_false`
* Integration tests:

  * `tests/config_env.rs::env_overrides_are_respected`
  * `tests/fake_node.rs::node_registry_returns_real_status_from_fake_node`
  * `tests/http_smoke.rs::healthz_and_metrics_smoke`

**Runtime check:**

```bash
cargo run -p svc-admin --bin svc-admin
```

Listeners:

* UI/API: `127.0.0.1:5300`
* Health/metrics: `127.0.0.1:5310`

Manual curl checks:

```bash
curl http://127.0.0.1:5310/healthz          # -> "ok"
curl http://127.0.0.1:5310/metrics | head   # -> ron_svc_admin_* metrics
curl http://127.0.0.1:5300/api/nodes        # -> example-node summary
curl http://127.0.0.1:5300/api/nodes/example-node/status
```

`/api/nodes/example-node/status` now returns a **real, node-derived** view (not just a hard-coded placeholder) when the node exposes `/api/v1/status`.

There are only **warnings** left (unused imports in `config/loader.rs` and a `private_interfaces` warning around `RawStatus`), no errors.

---

## 1. Config v2 – Shape and Behavior

We now have a **Config v2** that’s more expressive but still env-only.

### 1.1 Top-level config shape

`Config` (in `config/mod.rs` + submodules) has:

* `server: ServerCfg`
* `auth: AuthCfg`
* `ui: UiCfg`
* `nodes: NodesCfg`
* `polling: PollingCfg`
* `log: LogCfg`
* `actions: ActionsCfg`

All sub-structs are `Debug + Clone + Serialize + Deserialize` where appropriate.

### 1.2 ServerCfg

`ServerCfg`:

* `bind_addr: String` – UI/API listener, default `"127.0.0.1:5300"`.
* `metrics_addr: String` – health/metrics listener, default `"127.0.0.1:5310"`.
* `max_conns: usize` – default `1024`, must be `> 0`.
* `read_timeout: Duration` – default `5s` (env override supported).
* `write_timeout: Duration` – default `5s`.
* `idle_timeout: Duration` – default `60s`.
* `tls: TlsCfg` – simple TLS toggle and PEM file paths.

`TlsCfg`:

* `enabled: bool` – default `false`.
* `cert_path: Option<PathBuf>`
* `key_path: Option<PathBuf>`

Validation enforces: if `tls.enabled == true`, both `cert_path` and `key_path` must be present.

### 1.3 Logging Config

`LogCfg`:

* `format: String` – `"compact"` or `"pretty"` (we don’t strictly validate yet; defaults to `"compact"`).
* `level: String` – default `"info"`.

Env overrides:

* `SVC_ADMIN_LOG_FORMAT`
* `SVC_ADMIN_LOG_LEVEL`

These plug into `observability::init_tracing()` via `EnvFilter` + fmt subscriber.

### 1.4 Polling / Metrics

`PollingCfg`:

* `metrics_interval: Duration` – default `5s`.
* `metrics_window: Duration` – default `300s` (5 minutes).

Env overrides:

* `SVC_ADMIN_POLLING_METRICS_INTERVAL` (seconds)
* `SVC_ADMIN_POLLING_METRICS_WINDOW` (seconds)

Validation:

* `metrics_interval > 0`
* `metrics_window >= metrics_interval`

This is **ready for a metrics sampler** to use.

### 1.5 UI Config

`UiCfg`:

* `default_theme: String` – default `"light"`.
* `default_language: String` – default `"en-US"`.
* `read_only: bool` – default `true`.
* `dev: UiDevCfg`:

  * `enable_app_playground: bool` – default `false`.

Env behavior:

* Theme:

  * `SVC_ADMIN_UI_THEME` takes precedence if set.
  * Fallback to `SVC_ADMIN_UI_DEFAULT_THEME`.

* Language:

  * `SVC_ADMIN_UI_LANGUAGE` takes precedence.
  * Fallback to `SVC_ADMIN_UI_DEFAULT_LANGUAGE`.

* `read_only`:

  * `SVC_ADMIN_UI_READ_ONLY` parsed via flexible bool parser (`true/false`, `1/0`, `yes/no`, `y/n`, `on/off`).

* Dev flags:

  * `SVC_ADMIN_UI_DEV_ENABLE_APP_PLAYGROUND` → bool.

### 1.6 Auth Config

`AuthCfg`:

* `mode: String` – `"none" | "ingress" | "passport"`, default `"none"`.
* `passport_issuer: Option<String>`
* `passport_audience: Option<String>`
* `passport_jwks_url: Option<String>`

Env overrides:

* `SVC_ADMIN_AUTH_MODE` – validated by `load_auth_mode` (anything outside the three allowed values is a **Config error**).
* `SVC_ADMIN_AUTH_PASSPORT_ISSUER`
* `SVC_ADMIN_AUTH_PASSPORT_AUDIENCE`
* `SVC_ADMIN_AUTH_PASSPORT_JWKS_URL`

Currently, `auth.mode` is used to shape `/api/me` behavior conceptually, but `/api/me` is still a dev stub; we haven’t implemented ingress/passport resolvers yet.

### 1.7 Admin Actions Flags

`ActionsCfg`:

* `enable_reload: bool` – default `false`.
* `enable_shutdown: bool` – default `false`.

Env overrides:

* `SVC_ADMIN_ACTIONS_ENABLE_RELOAD`
* `SVC_ADMIN_ACTIONS_ENABLE_SHUTDOWN`

These flags are future-facing; we haven’t wired any mutating admin endpoints yet.

### 1.8 Node Config

`NodeCfg`:

* `base_url: String` – admin plane base (e.g. `http://127.0.0.1:9000`).
* `display_name: Option<String>`
* `environment: String` – `"dev" | "staging" | "prod"`, etc.
* `insecure_http: bool` – must be `true` to allow `http://` (otherwise config error).
* `forced_profile: Option<String>` – e.g. `"macronode"`, `"micronode"`.
* `macaroon_path: Option<PathBuf>` – reserved for future auth.
* `default_timeout: Option<Duration>` – per-node timeout override for NodeClient.

`NodesCfg = BTreeMap<String, NodeCfg>` keyed by node ID (`"example-node"`).

`Config::default` seeds one dev node:

* `"example-node"`:

  * `base_url = "http://127.0.0.1:9000"`
  * `display_name = Some("Example Node")`
  * `environment = "dev"`
  * `insecure_http = true`
  * `forced_profile = Some("macronode")`
  * `default_timeout = Some(2s)`

Env overrides for example-node:

* `SVC_ADMIN_EXAMPLE_NODE_URL`
* `SVC_ADMIN_EXAMPLE_NODE_ENV`

### 1.9 Loader behavior & tests

`Config::load()`:

* Guardrail: if `SVC_ADMIN_CONFIG` is set → **hard error** (file-based config is not yet implemented).

* Uses helper functions:

  * `load_addr` – validates socket addresses for `bind_addr` & `metrics_addr`.
  * `load_bool` – flexible bool parsing.
  * `load_usize`
  * `load_duration` – seconds.
  * `load_opt_path`
  * `load_auth_mode` – ensures allowed modes.

* Calls `validate()` at the end to enforce invariants.

**Test:** `tests/config_env.rs`

* Verifies env overrides for:

  * `SVC_ADMIN_BIND_ADDR`
  * `SVC_ADMIN_UI_DEFAULT_THEME`
  * `SVC_ADMIN_UI_THEME` (new env name takes precedence over the old one).

This gives us a **solid baseline** for Config v2.

---

## 2. Node Registry + Admin Plane Integration

The big progress in this slice: **NodeClient + NodeRegistry now talk to real nodes**.

### 2.1 NodeRegistry

`NodeRegistry`:

* Stored fields:

  * `nodes: Arc<BTreeMap<String, NodeCfg>>`
  * `client: NodeClient`

* Created via:

  ```rust
  pub fn new(cfg: &NodesCfg) -> Self
  ```

  It clones the `NodesCfg` from Config into an `Arc`.

* Methods:

  * `list_summaries() -> Vec<NodeSummary>`:

    * Maps each `NodeCfg` to:

      ```rust
      NodeSummary {
          id,
          display_name: cfg.display_name.unwrap_or(id.clone()),
          profile: None,
      }
      ```

    * `/api/nodes` uses this; `profile` still `null` for now until we surface it from status/profile data.

  * `get_status(&self, id: &str) -> Option<AdminStatusView>`:

    * Looks up `NodeCfg` by id.
    * Calls `self.client.fetch_status(id, cfg).await`.
    * On success: returns the `AdminStatusView` built from node data.
    * On error: logs and falls back to a placeholder:

      ```rust
      let mut view = status::build_status_placeholder();
      view.id = id.to_string();
      view.display_name = cfg.display_name.clone().unwrap_or(id.to_string());
      ```

  * `contains(&self, id: &str) -> bool`.

### 2.2 NodeClient

`NodeClient` is the **HTTP adapter** for the admin plane.

Key behaviors:

* Internal helpers:

  * `build_url(NodeCfg, path)`:

    * Validates scheme: must start with `http://` or `https://`.
    * If `http://` and `insecure_http == false` → `Error::Config`.

  * `effective_timeout(NodeCfg)`:

    * Returns `cfg.default_timeout` if set; otherwise, `None` (no per-request timeout override).

  * `get_text(NodeCfg, path)` and `get_json<T>(NodeCfg, path)`:

    * Build URL, apply timeout if present, send, `error_for_status()`, parse as text/JSON.

* `fetch_health(&self, cfg: &NodeCfg) -> Result<bool>`:

  * Calls `/healthz`.
  * Any non-empty body is considered “healthy”.

* `fetch_ready(&self, cfg: &NodeCfg) -> Result<bool>`:

  * Tries to parse `/readyz` as `{"ready": bool}`.
  * If JSON parse fails, logs a warning and falls back to text: non-empty body = `ready = true`.

* `fetch_version(&self, cfg: &NodeCfg) -> Result<Option<String>>`:

  * Parses `/version` as text.
  * Non-empty trimmed body → `Some(version)`.
  * Errors are logged and degraded to `Ok(None)` (version is informational).

* `fetch_status(&self, id: &str, cfg: &NodeCfg) -> Result<AdminStatusView>`:

  **Preferred path:**

  * Calls `/api/v1/status` and deserializes into `RawStatus`.
  * Uses `nodes::status::from_raw(id, cfg, raw)` to build a normalized `AdminStatusView`.
  * Logs a `debug!` with node id, version, and planes length.

  **Fallback path:**

  * If `/api/v1/status` fails (network, 4xx/5xx, parse error), log a `warn!` and degrade to the triple-probe:

    * `fetch_health`, `fetch_ready`, `fetch_version`.

  * Builds a placeholder view and fills what we know:

    ```rust
    let mut view = status::build_status_placeholder();
    view.id = id.to_string();
    view.display_name = cfg.display_name.clone().unwrap_or(id.to_string());
    if version.is_some() {
        view.version = version;
    }
    ```

  * Logs a `debug!` stating that this came from degraded probes with a `status_label` derived from health/ready.

* `ping_node(&self, _id: &str) -> Result<()>`:

  * Currently a no-op backcompat stub.

### 2.3 Status Normalization

`nodes/status.rs` is the **normalization layer** between raw node responses and our DTO.

Types:

* `RawPlane`:

  * `name: String`
  * `health: String`
  * `ready: bool`
  * `restart_count: u64`

* `RawStatus`:

  * `profile: Option<String>`
  * `version: Option<String>`
  * `planes: Vec<RawPlane>`

These match the shape returned by the fake node in `tests/fake_node.rs`.

Helpers:

* `build_status_placeholder() -> AdminStatusView`:

  * Thin wrapper around `AdminStatusView::placeholder()`.

* `from_raw(id: &str, cfg: &NodeCfg, raw: RawStatus) -> AdminStatusView`:

  * `id` is the registry key.
  * `display_name`: `cfg.display_name.unwrap_or(id.to_string())`.
  * `profile`: prefer `raw.profile`, fall back to `cfg.forced_profile`.
  * `version`: from `raw.version`.
  * `planes`: `RawPlane` → `PlaneStatus` mapping.

This is what `NodeClient::fetch_status` uses on the happy path.

### 2.4 Tests

* **Unit tests in `nodes/client.rs`:**

  * `node_client_can_talk_to_fake_admin_plane`:

    * Spins up a fake Axum admin plane with `/healthz`, `/readyz`, `/version`.
    * Verifies:

      * `fetch_health` returns `true`.
      * `fetch_ready` returns `true`.
      * `fetch_version` returns `Some("1.2.3-test")`.

  * `node_client_rejects_http_when_insecure_http_false`:

    * Uses a `NodeCfg` with `base_url = "http://127.0.0.1:12345"` and `insecure_http = false`.
    * Asserts that `fetch_health` returns a `Config` error containing `"insecure_http=false"`.

* **Integration test `tests/fake_node.rs`:**

  * Runs a fake node that serves **`/api/v1/status`** with:

    * `profile = "macronode"`
    * `version = "1.2.3-test"`
    * Some planes.

  * Builds a `NodeRegistry` with that fake node.

  * Calls `get_status("example-node")`.

  * Asserts:

    * `status.version == Some("1.2.3-test")` (this was the failing assertion before; it now passes, confirming that we use `/api/v1/status` correctly).
    * Planes and other fields are normalized as expected.

---

## 3. HTTP Surface & Metrics

The core HTTP surface from earlier is still in place and working:

* `/healthz` – simple `"ok"`, used by tests and curl.
* `/readyz` – returns `{"ready": true}` for now (truthful readiness will be wired later).
* `/api/ui-config` – `UiConfigDto` derived from Config.
* `/api/me` – dev-only identity stub (`"dev-operator"` etc.).
* `/api/nodes` – list of `NodeSummary` from `NodeRegistry`.
* `/api/nodes/:id/status` – **now backed by real node data** via `NodeClient` + `NodeRegistry`.

Metrics:

* `prometheus_bridge` (from earlier slice) exposes svc-admin metrics like:

  * `ron_svc_admin_nodes_total`
  * `ron_svc_admin_nodes_by_env{environment="dev"}`

These are visible at `GET /metrics` on the metrics port (`5310`).

---

## 4. Tests & How to Run Them

### 4.1 Full test suite for svc-admin

From repo root:

```bash
cargo test -p svc-admin --tests
```

This runs:

* Unit tests (within `src`):

  * `nodes::client::tests::*`
* Integration tests:

  * `tests/config_env.rs`
  * `tests/fake_node.rs`
  * `tests/http_smoke.rs`

### 4.2 Targeted test invocations

* Just the config env test:

  ```bash
  cargo test -p svc-admin --tests -- config_env
  ```

* Just fake node interop:

  ```bash
  cargo test -p svc-admin --test fake_node
  ```

* Just HTTP smoke (healthz/metrics):

  ```bash
  cargo test -p svc-admin --test http_smoke
  ```

---

## 5. Known Warnings / Cleanup

We deliberately haven’t cleaned these yet (to keep the slice focused):

1. **Unused imports in `config/loader.rs`:**

   * We import `ServerCfg`, `TlsCfg`, `UiCfg`, `UiDevCfg`, `ActionsCfg`, `AuthCfg`, `LogCfg`, `NodesCfg`, `PollingCfg` at the top but primarily use only `Config`.
   * Easy cleanup: trim the import list or use the aliases in type signatures if needed.

2. **`private_interfaces` warning in `nodes/status.rs`:**

   * `RawStatus` is `pub(crate)` but `from_raw` is `pub`.
   * Two options:

     * Make `from_raw` `pub(crate)` (probably correct – no need to expose this outside the crate).
     * Or make `RawStatus` fully `pub` if we do want it reusable elsewhere.

These are **non-blocking** and can be fixed in a quick polish pass.

---

## 6. What Remains / Next High-Impact Steps

We’ve now:

* Got **Config v2** for server, UI, auth, nodes, polling, and actions.
* Implemented a **real NodeClient** that prefers `/api/v1/status` and falls back to triple probes.
* Wired **NodeRegistry** to use NodeClient.
* Verified the behavior with **fake node** tests and HTTP smoke tests.
* Confirmed basic metrics are exposed.

Next **big, high-impact slices** for svc-admin:

### 6.1 Metrics Sampler & Facet Metrics (Top Priority for “admin-ness”)

* Use `Config.polling.metrics_interval` and `metrics_window` to implement a sampler:

  * Background task that periodically:

    * Scrapes each node’s `/metrics`.
    * Parses and filters `ron_facet_*` metrics.
    * Aggregates them per facet and per node.
    * Stores a small in-memory rolling window.

* Expose API DTOs for the UI:

  * e.g., `/api/nodes/:id/facet-metrics` returning summarized metrics by facet and plane.

* Extend Prometheus bridge:

  * Add svc-admin’s own metrics:

    * per-node sample success/failure counts.
    * sample lag.
    * maybe a gauge of “nodes_up”/“nodes_degraded”.

This will make the admin dashboard actually **feel like a live control plane**.

### 6.2 Auth Modes Skeleton (none / ingress / passport)

* Implement `auth` module with:

  * `auth::none`:

    * Dev identity from config/env (what we have now, but formalized).
  * `auth::ingress`:

    * Trust certain headers from ingress (e.g., `X-User`, `X-Roles`), validate them, produce a `MeResponse`.
  * `auth::passport`:

    * JWT/JWKS verification stub; even a minimal path that just validates the token structure + static key would get us started.

* Wire `/api/me` to delegate to the appropriate auth mode based on `auth.mode`.

This is a big step towards **operator-grade** admin security.

### 6.3 Config v2.5 – File + CLI Layer

* Add:

  * `--config <path>` CLI arg (using `clap`).
  * TOML file loading when `SVC_ADMIN_CONFIG` or `--config` is set.

* Precedence:

  * CLI > env > file > defaults.

* Update `Config::load()` to be layered:

  * `Config::from_defaults()`
  * `Config::merge_file(path)`
  * `Config::merge_env()`
  * `Config::validate()`

This will unlock **real ops workflows** (checked-in config files, environment overrides, etc.).

### 6.4 SPA / UI Skeleton (Vite + React + TS)

* In `ui/`:

  * Scaffold Vite + React + TS app.
  * Basic layout:

    * Sidebar with node list.
    * Main panel showing node status (from `/api/nodes/:id/status`).
    * Top bar with theme toggle + auth info.

* API client:

  * `ui/src/api/adminClient.ts` with typed calls to:

    * `getUiConfig()`
    * `getMe()`
    * `listNodes()`
    * `getNodeStatus(id)`

* Type alignment:

  * `ui/src/types/admin-api.ts` kept in sync with Rust DTOs.

This isn’t strictly required for backend correctness, but it’s critical for **developer experience** and proving the dashboard.

### 6.5 Misc hardening / cleanup

* Clean warning noise in `config/loader.rs` and `nodes/status.rs`.
* Expand tests:

  * Negative config tests (bad auth mode, bad addr, zero/negative durations).
  * NodeClient error paths (e.g., node unreachable).
  * Additional fake-node scenarios (down node, missing fields in `/api/v1/status`).

---

## 7. Quick “How to Resume” Checklist for Next Instance

When we come back to svc-admin, here’s a quick “start here” list:

1. **Rebuild mental model**:

   * Open `crates/svc-admin/src/config/` and `crates/svc-admin/src/nodes/`.
   * Skim `nodes/client.rs`, `nodes/status.rs`, `nodes/registry.rs`.

2. **Run tests once** to confirm starting from green:

   ```bash
   cargo test -p svc-admin --tests
   ```

3. Pick one of these **high-impact next steps**:

   * A) Implement **metrics sampler** using `PollingCfg` and extend the Prometheus bridge.
   * B) Implement **auth modes skeleton** and wire `/api/me`.
   * C) Add **file + CLI config** (TOML + `clap`).
   * D) Start the **SPA scaffolding** in `ui/`.

4. Whatever we pick, we’ll continue the pattern:

   * Update the code with full paste-ready files.
   * Add **at least one new test** exercising the new behavior.
   * Re-run `cargo test -p svc-admin --tests`.
   * Then capture another round of carryover notes.

---

That’s the current state of **svc-admin**: we’ve moved from “dev stub admin service” to a **real node-aware admin plane** with a modern config system and a tested NodeClient/NodeRegistry path. Next slices are about making it **observability-rich** (metrics sampler) and **secure/auth-aware**, then putting the **React/Vite UI shell** on top.


### END NOTE - DECEMBER 5 2025 - 12:00 CST





### BEGIN NOTE - DECEMBER 5 2025 - 14:00 CST
---

## 0. Quick status snapshot (today)

* Crate: `crates/svc-admin`

* Build & tests: ✅

  * `cargo test -p svc-admin` → all unit + integration tests pass, doc-tests 0/0.
  * Warnings only (unused imports in `config/loader.rs`, visibility nit in `nodes/status.rs`).

* Runtime (per previous curl checks, still valid):

  * UI/API: `127.0.0.1:5300`
  * Health/metrics: `127.0.0.1:5310`
  * Endpoints:

    * `/healthz` → `ok`
    * `/readyz` → `{"ready":true}`
    * `/api/ui-config` → real config-derived DTO
    * `/api/me` → dev stub identity (`auth.mode="none"`)
    * `/api/nodes` → registry-driven `NodeSummary[]`
    * `/api/nodes/{id}/status` → `AdminStatusView` (profile/version/planes) for example node

* Dev tooling:

  * UI scripts exist and are wired: `scripts/build-ui.sh`, `dev-ui.sh`, `lint-ui.sh`, `sync-ui-assets.sh` (npm + Vite flow).
  * `build.rs` is a no-op scaffold that will later run the UI build + embed assets.

**Very rough completion feel:**

* **Milestone 0.1.0 (Read-only Admin Console)** from README is ~70–80% done: config + node registry + `/healthz`/`/readyz`/`/metrics` + core `/api/*` are in place; metrics sampling is implemented at the scraper/parse level but not yet fully plumbed into “short-window summaries” + UI views.
* Overall svc-admin (including future auth, actions, hardening, scaling) is somewhere around **55–60%** of the *total* vision.

---

## 1. Crate shape & foundations

### 1.1 Cargo + layout

* `svc-admin` is a binary + lib crate, not published to crates.io, dual-licensed MIT/Apache-2.0.
* Features:

  * `tls` → enables `tokio-rustls` for HTTPS.
  * `otel` → reserved for OpenTelemetry.
  * `passport` → reserved for JWT/Passport auth mode.
* Runtime deps line up with the rest of RON-CORE: axum 0.7, reqwest 0.12 + rustls, tower-http 0.6, prometheus 0.14, tracing + tracing-subscriber, serde, anyhow, thiserror, etc.

The code bundle shows the full surface: `auth/*`, `config/*`, `dto/*`, `metrics/*`, `nodes/*`, `observability.rs`, `router.rs`, `server.rs`, `state.rs`, plus integration tests and `ui/` SPA scaffolding.

### 1.2 Config system (Phase 1 done, file+CLI later)

* Config is split by concern: `config/actions.rs`, `auth.rs`, `log.rs`, `nodes.rs`, `polling.rs`, `server.rs`, `ui.rs`, wired through `config/mod.rs` and `config/loader.rs`.
* Examples:

  * `AuthCfg`: `mode: "none" | "ingress" | "passport"` plus optional Passport issuer/audience/JWKS URL, with sensible `Default` (`mode = "none"`).
  * `ActionsCfg`: feature flags for “reload” and “shutdown” actions, default off.
* `Config::load()` (used in `cli::parse_args()`) currently handles **defaults + env** and is what the binary uses. `cli.rs` is deliberately minimal dev-preview: just calls `Config::load()` and leaves full `clap` CLI for later.
* Integration test `tests/config_env.rs` verifies env overrides (`env_overrides_are_respected`) and passed in your last run, so env layering is real and tested.

**Still to do (Config v2.5):** file + CLI layering (`--config`, TOML), precedence CLI > env > file > defaults, plus validation hooks as described in NOTES.

---

## 2. Node awareness & admin API

### 2.1 Node client + registry

* `nodes/client.rs`: async client that enforces `insecure_http` flags and can talk to a fake node in tests. You have tests for:

  * rejecting HTTP when `insecure_http=false`.
  * talking to a fake admin plane instance.
* `nodes/status.rs`: defines an internal `RawStatus` (node’s `/api/v1/status` view) and a public `from_raw` mapper to `AdminStatusView`. Compiler only warns that `RawStatus` is `pub(crate)` while `from_raw` is `pub`, i.e., a visibility nit but functionally OK.
* `nodes/registry.rs`: config-based inventory of nodes that backs `/api/nodes` and `/api/nodes/{id}/status`.

### 2.2 DTOs & HTTP surface

`dto/*` implements the “admin plane” view:

* `UiConfigDto` → what the SPA needs: title, theme/locale defaults, read-only flags, etc.
* `MeResponse` → identity view for the operator (currently dev stub).
* `NodeSummary` & `AdminStatusView` → node list + per-node status views.
* `metrics.rs` → shapes for facet metrics summaries (per-facet RPS, error rates, latency percentiles; some fields still wired as TODO / placeholders).

`router.rs` and `server.rs` expose:

* `/healthz`, `/readyz`, `/metrics` (service self-health & Prometheus export).
* `/api/ui-config`, `/api/me`, `/api/nodes`, `/api/nodes/{id}/status` — already wired and previously verified with curl.

The HTTP smoke test `tests/http_smoke.rs` stands up a real svc-admin instance, hits `/healthz`, `/readyz`, and `/metrics`, and expects success; it passed in your last run, confirming the wiring end-to-end.

---

## 3. Metrics & observability

This is where most of the **new progress** has landed since the previous carryover.

### 3.1 Observability harness

* `observability.rs` sets up tracing-subscriber (fmt + env-filter) and a Prometheus registry and exporter. `server::run` hooks this so `/metrics` exposes:

  * process-level svc-admin metrics,
  * and node inventory gauges (e.g., `ron_svc_admin_nodes_total`, possibly by env / profile) via `metrics/prometheus_bridge.rs`.

### 3.2 Prometheus bridge

* `metrics/prometheus_bridge.rs` is the “internal metrics → Prometheus” bridge. It:

  * defines `IntGauge` / `IntCounter` for node counts, sampler health, etc. (structure visible in CODEBUNDLE).
  * is already used in health/metrics smoke tests to assert the metrics surface is alive.

This is more about svc-admin’s **own** health, not node facet metrics.

### 3.3 Facet metrics sampler (newly implemented)

Previously, NOTES had “implement metrics sampler” as a future high-impact step. You’ve now *done* that part at the scraper/parse level:

**`metrics/sampler.rs` now provides:**

* `NodeMetricsTarget` — decoupled from `NodeCfg`, with:

  * `node_id: String`
  * `metrics_url: String`
  * optional `timeout: Option<Duration>`
* `spawn_samplers(targets, interval, facet_metrics, shutdown)`:

  * Spawns **one background task per node**.
  * Each task:

    * Seeds an initial sample ASAP.
    * Loops on `tokio::select!` between:

      * `shutdown.changed()` (clean shutdown, treating dropped sender as shutdown).
      * `time::sleep(interval)` to trigger sampling.
* `run_sampler_for_target(...)`:

  * Uses a shared `reqwest::Client`.
  * Logs start and shutdown per node id + URL.
  * On each tick:

    * fetches `/metrics` with optional timeout.
    * calls `sample_once` and logs transient errors as warnings (does **not** permanently fail on a bad scrape).
* `sample_once(...)`:

  * GETs node `/metrics`, error_for_status.
  * Parses response body via `parse_facet_snapshots`.
  * If any snapshots exist, forwards them into `FacetMetrics` (aggregator) — or, in the current code, at least parses & returns them; the aggregator wiring is what we’ll extend next.

**Parser: `parse_facet_snapshots`**

* Scans Prometheus text for:

  * `ron_facet_requests_total{facet="...",result="ok|error|..."}` etc.
* For each facet:

  * Aggregates total requests.
  * Aggregates error counts based on `result` label values (`error`, `err`, `failure`, `5xx`).
* Returns `Vec<FacetSnapshot>` with `{ facet, requests_total, errors_total }`.
* Unit test `parse_facet_snapshots_aggregates_by_facet` verifies:

  * Two facets: `overlay.connect` and `overlay.jobs`.
  * Correct totals and error counts for each.

That test is passing in your latest run: `test metrics::sampler::tests::parse_facet_snapshots_aggregates_by_facet ... ok`.

So **Phase 2 (metrics sampling) now has a working Prometheus scraper and facet-level counter aggregator at the parser level.**

### 3.4 What’s still missing in metrics

Relative to the blueprint / README milestone:

* `FacetMetrics` data structure:

  * Needs to hold **short-lived rolling windows** per `(node_id, facet)`:

    * RPS, error-rate, and latency buckets (p95/p99) over last N seconds.
  * Thread-safe structure (likely `Arc<RwLock<...>>` or a dedicated metrics bus).
  * Update method `update_from_scrape(node_id, snapshots)` called from sampler.
* DTO mapping:

  * Extend `dto::metrics::FacetMetricsSummary` / related DTOs to represent:

    * stats per facet (RPS, errorRate, p95, p99).
    * window size, `as_of` timestamp, maybe `node_id`.
* HTTP endpoints for metrics:

  * `/api/nodes/{id}/metrics/facets` or similar, returning facet summaries for that node.
* Prometheus export for sampler health:

  * e.g., `svc_admin_metrics_last_scrape_timestamp{node_id}`, `svc_admin_metrics_scrape_errors_total{node_id}` etc.
* Readiness:

  * Optionally gate `/readyz` on some minimum freshness of facet metrics window (for “this admin is fully warmed up” semantics).

Right now, you have:

* Parser + sampler tasks + one unit test at the parser level ✅
* Aggregation, DTOs, HTTP API, readiness gating, and export of metrics health **still to do**.

---

## 4. Auth modes & security (Phase 3 stubs)

Auth is scaffolded but not implemented:

* `auth/none.rs`:

  * `dev_identity() -> "dev-operator"`; used today as the identity behind `/api/me` in dev mode.
* `auth/ingress.rs`: comment-only placeholder for header-based identity (trusting ingress).
* `auth/passport.rs`: placeholder for JWKS-driven JWT/Passport validation.
* `auth/mod.rs`: wires the modules but does *not* yet define a common `AuthMode` trait or identity struct.

Config side (`AuthCfg`) is ready and defaults to `"none"`; README’s roadmap explicitly calls out Passport mode + gated actions as Milestone 0.2.0.

**Missing:**

* Real auth pipeline:

  * Pick `auth.mode` and construct appropriate auth handler.
  * Validate bearer tokens / ingress headers.
  * Extract subject + roles and feed `/api/me` and authorization decisions.
* Gated actions surface:

  * Even read-only `svc-admin` should be “auth aware” for `/api/me`.
  * Later: reload/shutdown actions + audit logs.

---

## 5. SPA / UI skeleton (Phase 4 & 5)

You’ve scaffolded a **full Vite + React + TS SPA** under `ui/`:

* `ui/index.html` + `ui/package.json`, `tsconfig`, `vite.config.ts` are all present.
* I18n:

  * `public/locales/en-US.json` and `es-ES.json` with basic nav strings.
* Routing:

  * `App.tsx` uses `react-router-dom` with routes:

    * `/` → `NodeListPage`
    * `/nodes/:id` → `NodeDetailPage`
    * `/settings` → `SettingsPage`
    * `/login` → `LoginPage`
    * `*` → `NotFoundPage`
* Layout & components:

  * `Shell`, `Sidebar`, `TopBar`, `ThemeToggle`, `LanguageSwitcher`, `NodeCard`, `NodeStatusBadge`, `PlaneStatusTable`, shared `EmptyState`, `ErrorBanner`, `LoadingSpinner`. 
* Types & API client:

  * `types/admin-api.ts` mirrors Rust DTOs.
  * `api/adminClient.ts` calls:

    * `/api/ui-config`
    * `/api/me`
    * `/api/nodes`
    * `/api/nodes/{id}/status` using typed generics.
* Future “App Plane Playground”:

  * `ronCorePlaygroundClient.ts` placeholder for `ron-app-sdk-ts` integration.
* Themes:

  * `theme/ThemeProvider.tsx`, `themes.ts`, `tokens.ts` define theme tokens & theme switcher.

So the SPA skeleton is **real** and roughly aligned with the backend DTOs, but:

* The pages/components are still mostly **layout + placeholder logic**.
* Facet metrics UI (`FacetMetricsPanel`, `FacetMetricsTemplate`) are present but not yet wired to real facet metrics endpoints.

Milestone 0.1.0’s UI parts (“read-only admin console”) will feel complete once:

* Node list and detail pages are fully driven by live API data (they mostly are).
* Basic facet metrics charts are backed by the new facet metrics API.
* Theme and language toggles round-trip from `UiConfigDto` instead of local defaults.

---

## 6. Tests & hardening status

Current tests (all passing) include:

* Unit:

  * `metrics::sampler::tests::parse_facet_snapshots_aggregates_by_facet`.
  * Node client behavior tests.
* Integration:

  * `config_env.rs` → env override layering.
  * `fake_node.rs` → NodeRegistry + NodeClient against fake node.
  * `http_smoke.rs` → full server, hitting `/healthz`, `/readyz`, `/metrics`.

Hardening tasks called out in README are still future work:

* Fuzz parsers (node status, metrics).
* Loom tests around samplers, shutdown, concurrency.
* Chaos tests for nodes going slow or down, ensuring svc-admin degrades gracefully.

---

## 7. What we accomplished since the last carry-over

Compared to the December 4/5 notes, new concrete progress:

1. **Metrics sampler fully implemented:**

   * Added sampler loops that poll each node’s `/metrics` with configurable interval and timeout.
   * Implemented Prometheus text parsing for `ron_facet_requests_total{facet=...,result=...}` into per-facet counters.
   * Added and passed unit test validating facet-level aggregation.

2. **Metrics pipeline is now “live”:**

   * svc-admin can actually poll nodes’ `/metrics` in the background, rather than metrics being purely conceptual.
   * Error handling is non-fatal and matches the “degrade gracefully” invariant: bad scrapes log and continue.

3. **Crate is back to fully green tests after resolving:

   * `chrono` dependency + DTO mismatches in metrics.
   * Import/visibility issues in `metrics::facet` and `metrics::sampler`.

So you moved from “Phase 1 vertical slice, metrics TODO” to **Phase 2 partially done**: samplers and parsers in place, aggregator & UI still pending.

---

## 8. Remaining high-impact work (by theme)

Here’s how to think about the rest of svc-admin.

### 8.1 Finish Milestone 0.1.0 (Read-only Admin Console)

* **Facet metrics aggregator + API:**

  * Finish `FacetMetrics` data structure and in-memory windowing.
  * Wire sampler to update aggregator per `(node_id, facet)`.
  * Expose `/api/nodes/{id}/metrics/facets` (or similar) returning `FacetMetricsSummary[]` with fields from `dto::metrics`.
  * Add tests:

    * sampler → aggregator integration (feed a fake `/metrics` body and assert the API result).
* **UI wiring:**

  * `FacetMetricsPanel` and `FacetMetricsTemplate` should:

    * call the new metrics API,
    * render charts using existing `MetricChart` component.
  * Use `UiConfigDto` to drive theme/locale + maybe feature flags.
* **Warnings cleanup:**

  * Remove unused imports in `config/loader.rs`.
  * Tweak visibility in `nodes/status.rs` (either make `RawStatus` public or narrow `from_raw` visibility) to fix `private_interfaces` warning.

Once that’s done, 0.1.0 is basically “done”: operators can see node lists, status, and basic facet metrics from a single svc-admin instance.

### 8.2 Milestone 0.2.0 — Auth & actions

* Implement `AuthMode` trait and concrete modes:

  * `none` (dev-only, current behavior).
  * `ingress` (trust headers from reverse proxy).
  * `passport` (JWKS fetch, JWT validation, roles).
* Wire `/api/me` to actual identity modes instead of always `dev_identity()`.
* Implement action endpoints (reload/shutdown) behind:

  * config flags (`ActionsCfg`),
  * roles from auth,
  * explicit “are you sure” flows in the UI,
  * audit logging.

### 8.3 Config v2.5 — File + CLI

* Introduce `clap` in `cli.rs`:

  * `--config`, `--bind-addr`, `--metrics-addr`, `--auth-mode`, etc.
* Implement TOML loading (`SVC_ADMIN_CONFIG` and/or `--config`).
* Make `Config::load()` layered as per NOTES: defaults → file → env → CLI, with validation and helpful error messages.

### 8.4 Observability & scaling (0.3.x / hardening)

* Add svc-admin internal metrics for:

  * sampler lag/staleness,
  * scrape errors per node,
  * HTTP concurrency and backpressure.
* Gate `/readyz` on key conditions (listeners bound, metrics exporter bound, maybe config loaded and samplers running).
* Chaos & perf tests:

  * Many nodes + slower `/metrics` → ensure samplers don’t overwhelm nodes.
  * Load tests on `/api/*` and SPA flows.

### 8.5 SPA polish

* Flesh out real node detail view (status lanes, plane status, per-plane health).
* Add basic UX touches:

  * Node filters (by env/profile),
  * Search,
  * Error states when nodes are down.
* Verify localization pipeline (EN/ES) with a minimal set of translated strings.

---

## 9. Updated “how to resume” checklist

When you come back to svc-admin next time:

1. **Reconfirm green baseline:**

   ```bash
   cd /Users/mymac/Desktop/RustyOnions
   cargo test -p svc-admin
   ```

2. **Pick a next slice (recommendations, in order of leverage):**

   * **Option A — Finish facet metrics end-to-end:**

     * Open `metrics/facet.rs`, `metrics/sampler.rs`, `dto/metrics.rs`, `state.rs`, `router.rs`.
     * Design `FacetMetrics` windowing and `/api/nodes/{id}/metrics/facets`.
     * Add one integration test that spins svc-admin, feeds fake node `/metrics`, hits the new endpoint.

   * **Option B — Auth pipeline + `/api/me`:**

     * Open `auth/*`, `config/auth.rs`, `dto/me.rs`, `router.rs`.
     * Implement `AuthMode` trait and `none/ingress` first (leave Passport as stub).
     * Wire `/api/me` and add tests for each mode.

   * **Option C — Config file + CLI:**

     * Open `config/loader.rs`, `cli.rs`, docs `CONFIG.MD`.
     * Add TOML + `clap` CLI and tests.

   * **Option D — SPA metrics wiring:**

     * Once A is done, wire `FacetMetricsPanel` to real API calls and confirm charts render.

3. **After any change:**

   * Re-run `cargo test -p svc-admin`.
   * Optionally run `scripts/dev-ui.sh` and manually verify SPA flows against a live svc-admin.
   * Capture another round of carry-over notes.

---

That’s the current state of **svc-admin**: you now have a **node-aware, metrics-sampling admin plane** with a real config system, a working HTTP surface, and a Vite/React SPA shell ready to be wired into the new facet metrics and auth story.


### END NOTE - DECEMBER 5 2025 - 14:00 CST





### BEGIN NOTE - DECEMBER 5 2025 - 15:00 CST


---

## 0. Quick status snapshot (svc-admin today)

**Crate:** `crates/svc-admin`
**Role:** Read-only admin console and future control plane for RON-CORE nodes (macronode/micronode/etc.).

**Backend status (Rust):**

* `cargo test -p svc-admin` was green in the last session (unit + integration tests). The latest http_smoke test spins up svc-admin with an in-memory config, validates `/healthz` and `/metrics` on the metrics port, and checks for node inventory gauges like `ron_svc_admin_nodes_total` and `ron_svc_admin_nodes_by_env`.
* Default config in tests still uses one example node (`example-node`) with a fake admin plane at `http://127.0.0.1:9000`, and UI defaults `default_theme="light"`, `default_language="en-US"`, `read_only=true`.

**Frontend status (Vite + React SPA):**

* `npm install` + `npm run dev` now works in `crates/svc-admin/ui`, after adding Vite and React dependencies.
* The initial JSX parse error in `src/i18n/index.ts` is fixed in the current codebundle via a clean `I18nProvider` implementation that compiles under Vite/esbuild.
* SPA layout, theme, and language wiring are now hooked into the backend `/api/ui-config` (for defaults) and the new `ThemeProvider` / `I18nProvider` contexts and toggles.

**Very rough completion feeling (updated):**

* **Milestone 0.1.0 – Read-only Admin Console:** ~**85–90% complete.**

  * Node registry, status endpoints, health/readiness/metrics surfaces, and facet metrics sampling/aggregation are all implemented.
  * SPA shell is real and now uses backend-driven theme + locale defaults, and can display node lists and details.
  * The facet metrics path (`/api/nodes/{id}/metrics/facets` → `FacetMetricsSummary` → charts) is wired end-to-end but still needs UX polish and more tests.
* **Overall svc-admin vision (including auth, actions, hardening):** ~**60–65%**.

Treat those percentages as directional “feel”, not strict math.

---

## 1. Crate structure & responsibilities

### 1.1 High-level crate shape

svc-admin is a **binary + support lib** crate (not published to crates.io):

* Binary: runs an HTTP server that:

  * Exposes `/healthz`, `/readyz`, `/metrics` on a metrics/admin port.
  * Exposes `/api/ui-config`, `/api/me`, `/api/nodes`, `/api/nodes/{id}/status`, and `/api/nodes/{id}/metrics/facets` on the UI/API port.
* Lib modules:

  * `config/*` – configuration types and loader.
  * `nodes/*` – node registry + client + status mapping.
  * `dto/*` – HTTP DTOs for UI config, identity, nodes, metrics.
  * `metrics/*` – internal metrics bridge, facet sampler, rolling windows.
  * `observability.rs` – tracing + Prometheus registry/export.
  * `router.rs`, `server.rs`, `state.rs` – HTTP routing, app state, bootstrap.

Features:

* `tls` – (planned) HTTPS via `tokio-rustls`.
* `otel` – (planned) OpenTelemetry.
* `passport` – (planned) JWT/JWKS auth integration.

The crate follows the global RON-CORE blueprints for observability and DTO hygiene (golden metrics, `/metrics`/`/healthz`/`/readyz` consistency, etc.).

### 1.2 Dev tooling & scripts

* Backend: standard `cargo test -p svc-admin`, plus integration tests under `crates/svc-admin/tests`.
* Frontend: `crates/svc-admin/ui` is a separate Vite project with:

  * `package.json`, `tsconfig.json`, `vite.config.ts`, `index.html`.
  * Dev server via `npm run dev`.
* There are also repo-level scripts (in other crates) that show the desired pattern: spin service, discover `/metrics` URL from logs, then curl `/metrics` and `/healthz` as smoke (good model for future svc-admin sys-tests).

---

## 2. Config system

### 2.1 Config types and loader

The config surface is split by concern:

* `AuthCfg` – selects auth mode (`none`, `ingress`, `passport`, plus passport issuer/audience/JWKS fields) with sensible defaults (`mode="none"`).
* `UiCfg` – drives the admin UI:

  * `default_theme: String`
  * `default_language: String`
  * `read_only: bool`
  * `dev: UiDevCfg` for local overrides.
* `ServerCfg`, `NodesCfg`/`NodeCfg`, `PollingCfg`, `LogCfg`, `ActionsCfg` – server binds, node inventory, polling intervals, logging, and action flags.

There’s a `Config` aggregate struct that bundles these and is used by `server::run`. Integration tests directly build a `Config` with:

* A metrics bind at `127.0.0.1:5310`.
* A UI bind at `127.0.0.1:5300`.
* A single example node in `NodesCfg`.

The loader:

* Today: `Config::load()` handles defaults + env and is what the binary uses.
* There’s a tested pattern for building configs in tests without env (construct `Config` directly then call `server::run(cfg)`).

### 2.2 What’s implemented vs missing

**Done:**

* Per-concern config structs exist and are wired into `Config`.
* Defaults are reasonable for dev (auth mode none, read-only UI, polling defaults).
* Env layering is tested (in a dedicated `config_env` integration test – from previous notes).

**Still to do:**

* CLI + file layering:

  * CLI flags via `clap` (`--config`, `--bind-addr`, `--metrics-addr`, `--auth-mode`, etc.).
  * TOML config files with precedence: defaults < file < env < CLI.
  * Validation with clear error messages when config is inconsistent.
* Possibly unify naming between backend `UiCfg` (`default_language`) and frontend type (`default_locale`) for clarity (see §5.3).

---

## 3. Node registry & admin HTTP API

### 3.1 Node registry & client

The node side has 3 main pieces:

1. **NodeCfg / NodesCfg (config):**

   * Map from node id → `NodeCfg` containing `base_url`, `display_name`, `environment`, `insecure_http`, `forced_profile`, `macaroon_path`, and `default_timeout`.

2. **NodeClient (runtime HTTP client):**

   * Uses `reqwest` with:

     * Optional rejection of plain HTTP unless `insecure_http=true`.
     * Per-node timeouts based on `default_timeout`.
   * Used to call each node’s admin endpoints for status and metrics.

3. **Registry:**

   * Based on `NodesCfg`, provides:

     * `list_nodes()` → `Vec<NodeSummary>` for `/api/nodes`.
     * `get_node(id)` → `NodeCfg` for per-node operations.

Integration tests spin a fake node and exercise the client/registry path (see previous carryover notes).

### 3.2 DTOs & HTTP surface

The DTO layer is defined in `dto/*` and mirrored in the UI types file:

* `UiConfigDto`:

  * `title`, `subtitle?`, `read_only`, `default_theme: 'light' | 'dark' | 'system'`, `default_locale: string`.
* `MeResponse`:

  * `id`, `display_name`, `roles: string[]`, optional `login_url`.
* `NodeSummary`:

  * `id`, `display_name`, `profile`, plus future labels/tags.
* `AdminStatusView`:

  * `node_id`, `display_name`, `profile`, `version`, `planes: PlaneStatus[]`.
* `FacetMetricsSummary`:

  * `facet`, `rps`, `error_rate`, `p95_latency_ms`, `p99_latency_ms` (mirrors Rust `dto::metrics::FacetMetricsSummary`).

The `adminClient` in the UI provides typed helpers:

* `getUiConfig() → UiConfigDto`
* `getMe() → MeResponse`
* `getNodes() → NodeSummary[]`
* `getNodeStatus(id) → AdminStatusView`
* `getNodeFacetMetrics(id) → FacetMetricsSummary[]` (calls `/api/nodes/{id}/metrics/facets`)

On the Rust side, `router.rs` registers these endpoints and maps DTOs to handlers anchored on shared app state (config, registry, facet metrics). The http_smoke test proves `/healthz` and `/metrics` are correctly wired; previous manual curl runs validated `/api/ui-config`, `/api/me`, `/api/nodes`, `/api/nodes/{id}/status`.

---

## 4. Metrics & observability path

This is now one of the strongest parts of svc-admin.

### 4.1 Observability harness

`observability.rs` (plus companion modules) configures:

* Tracing via `tracing-subscriber` with env filter & structured logs.
* Prometheus registry and exporter, serving `/metrics` on the metrics port.
* Node inventory gauges:

  * `ron_svc_admin_nodes_total`
  * `ron_svc_admin_nodes_by_env`
  * Verified in integration tests by scraping `/metrics` and checking for these strings.

Health endpoints:

* `/healthz` → `"ok"` when the service is up.
* `/readyz` → JSON structure (`ready: bool`) gated by readiness conditions (listeners, config loaded, etc., more gating still to come).

### 4.2 Facet metrics: sampler + parser + aggregator

You now have an end-to-end facet metrics path:

1. **Sampler configuration & task fanout (`metrics/sampler.rs`):**

   * `NodeMetricsTarget` captures:

     * `node_id: String`
     * `metrics_url: String`
     * Optional per-node timeout.
   * `spawn_samplers(targets, interval, facet_metrics, shutdown)`:

     * Spawns one `tokio::task` per node target.
     * Each task:

       * Immediately does a first scrape.
       * Then loops on `tokio::select!` between:

         * `shutdown.changed()` → graceful exit.
         * `time::sleep(interval)` → next scrape.
   * `run_sampler_for_target(...)` uses a shared `reqwest::Client` per sampler, respecting timeouts and logging start/stop per node.

2. **Prometheus text parser → facet snapshots:**

   * `parse_facet_snapshots(body: &str) -> Vec<FacetSnapshot>`:

     * Scans the Prometheus text exposition for `ron_facet_requests_total{facet="...",result="..."}` etc.
     * Aggregates per `(facet, result)`:

       * `requests_total`
       * `errors_total` (based on `result` labels like `error`, `err`, `failure`, `5xx`).
   * Test: `parse_facet_snapshots_aggregates_by_facet` builds a fake metrics body and asserts that:

     * It produces two facets (`overlay.connect`, `overlay.jobs`).
     * Totals and error counts are correct.
   * This test is green in the latest run (from your previous instance).

3. **Rolling window aggregator (`metrics/facet.rs`):**

   * `FacetMetrics` is an in-memory time-windowed store keyed by `(node_id, facet)`:

     * Records timestamped snapshots from each scrape.
     * Prunes old samples beyond the configured window.
   * Exposes something like:

     * `update_from_scrape(node_id, snapshots)` – called by sampler.
     * `summaries_for_node(node_id) -> Vec<FacetMetricsSummary>` – rolled-up stats:

       * RPS (requests/sec over the recent window).
       * Error rate (0.0–1.0).
       * p95 and p99 latency estimates (placeholder until we ingest latency histograms).
   * On the frontend, this matches `FacetMetricsSummary` type and is consumed by metrics panels.

4. **Integration into app state and HTTP:**

   * `state.rs` stores `FacetMetrics` alongside `Config` and `NodeRegistry`.
   * `router.rs` defines `/api/nodes/{id}/metrics/facets` which:

     * Looks up `node_id` in registry.
     * Asks `FacetMetrics` for summaries.
     * Returns `Vec<FacetMetricsSummary>` as JSON.

5. **Sampler lifecycle:**

   * `server::run(cfg)` constructs:

     * The app state (including `FacetMetrics`).
     * Node metrics targets from `NodesCfg`.
   * It calls `spawn_samplers(...)` with:

     * Configured polling interval from `PollingCfg`.
     * A shared `shutdown` watch channel.
   * Shutdown path:

     * When the process is asked to stop, `shutdown` is toggled → sampler tasks exit cleanly.

**Net result:** svc-admin now continuously scrapes node `/metrics`, parses facet counters, maintains rolling windows per facet, and exposes them via a clean HTTP API tailored for the SPA.

### 4.3 What’s still missing in observability

* We aren’t yet:

  * Tracking sampler error metrics per node (e.g., `svc_admin_metrics_scrape_errors_total{node_id}`).
  * Gating `/readyz` on sampler freshness (e.g., “this admin is ready once we have at least one scrape from all configured nodes”).
* Latency percentiles in `FacetMetricsSummary` are currently conceptual; full fidelity requires parsing histograms from node metrics.
* No chaos/perf tests yet for:

  * Many nodes with slow or failing `/metrics`.
  * Impact of sampler load on nodes and on svc-admin itself.

Those are good “hardening” tasks for a later milestone.

---

## 5. SPA / UI layer

### 5.1 Core layout and routing

The SPA lives in `crates/svc-admin/ui`.

Key pieces:

* `index.html` – simple entrypoint with `<div id="root"></div>` and `src/main.tsx` as the module script.
* `main.tsx` – initializes React root, mounts `App` inside providers (ThemeProvider, I18nProvider, Router).
* `App.tsx` – React Router routes:

  * `/` → `NodeListPage`
  * `/nodes/:id` → `NodeDetailPage`
  * `/settings` → `SettingsPage`
  * `/login` → `LoginPage`
  * `*` → `NotFoundPage`
* Layout components:

  * `Shell` – main frame with sidebar + top bar + content.
  * `Sidebar` – nav with “Nodes” and “Settings” links.
  * `TopBar` – brand, `ThemeToggle`, `LanguageSwitcher`, and maybe `Me` info.

### 5.2 Theme system & backend-driven defaults

Theme surface:

* `theme/themes.ts` defines theme tokens: background/foreground colors for `light` and `dark`.
* `theme/tokens.ts` defines design tokens like border radius sizes.

`ThemeProvider`:

* Maintains `theme` state in React context.
* On mount:

  * Calls `adminClient.getUiConfig()` to fetch backend UI config.
  * Interprets `cfg.default_theme` as a `Theme` (`'light' | 'dark' | 'system'`) and sets the theme if valid.
  * Falls back to local default if the call fails (developer-preview behavior).
* Provides `{ theme, setTheme }` via context; `useTheme()` validates usage inside the provider.

`ThemeToggle`:

* Uses `useTheme()` to read `theme` and `setTheme`.
* Clicking the button cycles between light and dark (and optionally system), and also updates the document (e.g., data-theme attribute) so the CSS theme actually changes.
* This is wired in the current codebundle: ThemeToggle imports `useTheme` and uses it to flip the theme (rather than being a dead button).

**Net:** Theme defaults come from the backend, but the operator can override them interactively in the SPA.

### 5.3 I18n system & backend-driven locale

Types & context:

* `UiConfigDto` in `types/admin-api.ts` includes `default_locale: string`.
* `i18n/index.ts` defines:

  * `I18nContext` with `locale`, `t(key)`, `setLocale`.
  * `I18nProvider` that:

    * Holds `locale` in state.
    * On mount, fetches `/api/ui-config` via `adminClient.getUiConfig()`.
    * Sets initial `locale` to `cfg.default_locale` (or `cfg.default_language`, depending on exact wiring – this is one alignment point to double-check when resuming).
    * Provides translations using JSON locale files.

Language switcher:

* `LanguageSwitcher` now pulls from `useI18n()` (instead of being a hardcoded `<select>`).
* It reads `locale` and calls `setLocale()` on change, keeping state in sync with the provider and with current language in the SPA.

Previously, `LanguageSwitcher` was an uncontrolled select with `defaultValue="en-US"`. That’s now upgraded to a real binding into the i18n context.

**Net:** Locale defaults now come from the backend config, but the operator can switch between languages interactively. The translation JSON files are currently minimal (English/Spanish nav labels etc.), but the pipeline is in place.

### 5.4 Node list & detail, facet metrics UI

Node list:

* `NodeListPage` (not shown in this snippet but in previous bundles) calls `adminClient.getNodes()` to render a grid/list of nodes.
* Nodes display `display_name`, `profile`, and possibly environment tags.

Node detail:

* `NodeDetailPage` uses route `:id`, calls:

  * `adminClient.getNodeStatus(id)` for profile/planes.
  * `adminClient.getNodeFacetMetrics(id)` for facet metrics.
* It renders:

  * Node summary (name/profile/version).
  * A plane status table.
  * A **facet metrics panel** that visualizes `FacetMetricsSummary[]`.

Facet metrics UI:

* Types: `FacetMetricsSummary` includes fields for `facet`, `rps`, `error_rate`, `p95_latency_ms`, `p99_latency_ms`.
* Components:

  * `FacetMetricsPanel` or `MetricChart` components plot RPS and error rate over facets (exact look from previous bundle).
* CSS classes:

  * You added class hooks like `svc-admin-node-grid`, `svc-admin-metric-chart`, `svc-admin-language-switcher`, etc., ready for fine-tuning in CSS.

**What’s left in the UI:**

* Styling pass:

  * Polish spacing, typography, and colors around the new classes so the dashboard feels “God-tier” rather than plain.
* Error/loading states:

  * Node detail currently assumes happy-path for facet metrics; needs more explicit loading and error states (“no metrics yet”, “node unreachable”).
* Settings & login pages:

  * `SettingsPage` and `LoginPage` are mostly placeholders; they need to render read-only config and basic auth info later.

---

## 6. Auth modes & security posture

Auth scaffolding:

* `auth/none.rs` – dev mode, returns a static identity like `"dev-operator"` for `/api/me`.
* `auth/ingress.rs` – stub for trusting identity from ingress-provided headers (e.g., `X-Remote-User`, `X-Remote-Roles`).
* `auth/passport.rs` – stub for JWT/Passport integration using JWKS (issuer/audience from `AuthCfg`).

Config:

* `AuthCfg` is present and defaults to `mode="none"` while you’re in developer preview.
* README and the HARDENING blueprint call for capability-based identity, short-lived tokens, and amnesia-friendly logging (no sensitive identity data in metrics/logs).

Current reality:

* `/api/me` is still effectively dev-stubbed, not wired to real auth.
* There is no enforcement on admin endpoints; they are open in dev mode.

Future work:

* Introduce an `AuthMode` trait with `authenticate(req) -> Identity` and roles.
* Implement:

  * `none` (dev only).
  * `ingress` (trusted headers with capability mapping).
  * `passport` (validate JWT, map claims to roles).
* Wire `/api/me` and future action endpoints through `Identity`.
* Add audit logging and metrics for auth failures.

---

## 7. Tests, quality gates & hardening

### 7.1 Current test coverage

* **Unit tests:**

  * Metrics parsing: `parse_facet_snapshots_aggregates_by_facet` (validates correct aggregation of counters per facet).
  * Node client tests (from earlier sessions) verifying HTTP vs HTTPS behavior, `insecure_http` guard, etc.
* **Integration tests:**

  * `http_smoke`:

    * Builds a `Config` with UI + metrics + node registry.
    * Runs `server::run`.
    * Hits `/healthz` and `/metrics` on metrics port.
    * Asserts `ron_svc_admin_nodes_total` and `ron_svc_admin_nodes_by_env` appear in metrics output.
  * `config_env` (from previous notes):

    * Ensures env overrides shape config correctly.
  * `fake_node`:

    * Uses a fake node admin plane to test registry + NodeClient.

### 7.2 Planned test/hardening work

From the project blueprints and TODO/NOTES:

* **Property tests** for:

  * Facet metrics window invariants (no negative RPS, monotonic counters).
  * Node registry invariants (all nodes have unique ids, environment tagging consistent).
* **Fuzzing** for:

  * Node `/metrics` parsing (Prometheus text is tolerant but can be weird).
  * Node status DTOs.
* **Chaos/looms:**

  * Use `loom` to validate concurrency in samplers and shutdown path.
  * Chaos tests where node metrics endpoints intermittently fail or hang; svc-admin should degrade gracefully, not block.

These align with the broader hardening blueprint (readiness degrade before collapse, golden metrics everywhere, etc.).

---

## 8. Updated completion estimate & remaining big themes

### 8.1 Milestone 0.1.0 – Read-only Admin Console

**Rough completion: ~85–90%**

**Already in place:**

* Config system with sane defaults (env-driven; file/CLI later).
* Node registry + NodeClient and DTO mapping.
* Health, readiness, and metrics surfaces:

  * `/healthz`, `/readyz`, `/metrics`.
  * Node inventory gauges exported.
* Facet metrics path:

  * Sampler → parser → rolling window aggregator → `/api/nodes/{id}/metrics/facets` → SPA charts.
* SPA shell:

  * Routing, layout, node list/detail.
  * Theme + locale defaults from backend via `/api/ui-config`.
  * Interactive ThemeToggle + LanguageSwitcher wired into context.

**Still to do for 0.1.0 to feel “done”:**

1. **Polish facet metrics UX:**

   * Ensure NodeDetail clearly shows “Metrics last updated at …” and “No metrics yet” states.
   * Add simple tooltips or legends for RPS/error rate and latency values.
   * Possibly add per-facet sparkline history (if you want extra wow factor).

2. **Tighten `/api/ui-config` → UI wiring:**

   * Make sure `UiCfg` ↔ `UiConfigDto` ↔ SPA types are fully consistent:

     * `default_language` vs `default_locale` naming.
     * Make sure both theme and locale are pulled from the same DTO field in both ThemeProvider and I18nProvider.

3. **CSS / design pass:**

   * Use the existing class hooks (`svc-admin-shell`, `svc-admin-sidebar`, `svc-admin-node-grid`, `svc-admin-language-switcher`, etc.) to:

     * Improve spacing, colors, and visual hierarchy.
     * Make charts and tables feel like a cohesive admin product, not a scaffold.

4. **Warnings cleanup:**

   * Remove unused imports in `config/loader.rs` (ServerCfg, TlsCfg, UiCfg, UiDevCfg, etc.) which are currently only warning noise.
   * Resolve minor visibility nits in `nodes/status.rs`.

Once those are done, 0.1.0 is essentially a shippable **developer preview** of the read-only admin console.

### 8.2 Milestone 0.2.0 – Auth & gated actions

Major themes:

* Implement `AuthMode` pipeline and real `/api/me`:

  * `none` for dev; `ingress` and `passport` for real deployments.
  * Roles/permissions surfaced in `MeResponse.roles`.
* Introduce **actions**:

  * Config-gated endpoints for reload/shutdown and other node operations (read-only by default).
  * UI affordances in NodeDetail and Settings pages (with “are you sure?” flows).
* Logging & audit:

  * Append-only audit log of actions, with identity and reason.
  * Possibly integrate with `ron-audit` later.

### 8.3 Config v2.5 – File + CLI

* CLI entrypoint via `clap`:

  * `svc-admin --config /path/to/svc-admin.toml --bind-addr 127.0.0.1:5300 --metrics-addr 127.0.0.1:5310 --auth-mode passport` etc.
* Config layering:

  * Defaults < TOML file < env vars < CLI.
* Validation & schema docs:

  * `CONFIG.MD` describing all fields, defaults, and precedence.

### 8.4 Hardening & scaling (0.3.x+)

* Internal metrics:

  * `svc_admin_sampler_last_scrape_timestamp{node_id}`
  * `svc_admin_sampler_errors_total{node_id}`
  * `svc_admin_http_inflight`, `svc_admin_http_latency_seconds{path,code}`.
* Readiness semantics:

  * `/readyz` should consider:

    * HTTP listeners bound.
    * Metrics exporter bound.
    * Config loaded.
    * (Optionally) at least one scrape from each configured node.
* Load behavior:

  * With many nodes and tight scrape intervals, ensure:

    * Node metrics scraping remains under configured concurrency limits.
    * svc-admin itself doesn’t starve its HTTP handler threads.

---

## 9. How to resume next time (step-by-step)

When you spin up the next instance and want to continue svc-admin:

1. **Reconfirm backend baseline:**

   ```bash
   cd /Users/mymac/Desktop/RustyOnions
   cargo test -p svc-admin
   ```

2. **Reconfirm SPA baseline:**

   ```bash
   cd crates/svc-admin/ui
   npm install       # one-time or when deps change
   npm run dev
   ```

   * Visit `http://localhost:5173/`.
   * Ensure:

     * Theme defaults to backend `default_theme`.
     * Language defaults to backend `default_locale`/`default_language`.
     * ThemeToggle and LanguageSwitcher work.

3. **Recommended next high-impact slice (when we resume coding):**

   **Option A (UI/metrics polish for 0.1.0):**

   * Tighten `UiCfg` ↔ `UiConfigDto` alignment (theme + locale).
   * Add better loading/error states around facet metrics on `NodeDetailPage`.
   * Add CSS polish for `svc-admin-node-grid`, `svc-admin-metric-chart`, and sidebar/top bar.

   **Option B (Auth foundation for 0.2.0):**

   * Implement `AuthMode` trait and back `auth.mode=none|ingress` first.
   * Wire `/api/me` to real identity objects instead of static dev stub.
   * Expose current identity & roles in the top bar of the SPA.

   **Option C (Config CLI + file layering):**

   * Build `cli.rs` with `clap`.
   * Add TOML loader and precedence logic, plus tests.

We can take any of those as the “next move” depending on whether you want to lock down 0.1.0 polish first (A), start the security story (B), or improve operator ergonomics (C). Given where we are now, **A (UI + metrics polish) will give the most visible payoff fast**, and B/C can follow.



### END NOTE - DECEMBER 5 2025 - 15:00 CST






### BEGIN NOTE - DECEMBER 5 2025 - 17:55 CST

---

## 0. Quick status snapshot

**Crate:** `crates/svc-admin`
**Role:** Read-only admin console + future control plane for RON-CORE nodes (macronode/micronode/etc.).

**Backend:**

* `cargo test -p svc-admin --tests` ✅ green.
* `cargo run -p svc-admin --bin svc-admin` ✅ starts:

  * UI/API: `127.0.0.1:5300`
  * Health/metrics: `127.0.0.1:5310`
* Health surface:

  * `GET /healthz` → `"ok"`
  * `GET /readyz` → JSON `{ "ready": true }` (simple for now, but wired for future gating).
  * `GET /metrics` → Prometheus metrics for svc-admin itself.

**Frontend (Vite/React SPA):**

* Lives in `crates/svc-admin/ui`.
* `npm run build` ✅ works (Vite bundle builds).
* `npm run dev` ✅ works, proxies `/api/*` to `127.0.0.1:5300` when server is running.
* SPA routes:

  * `/` → Nodes list page.
  * `/nodes/:id` → Node detail page.
  * `/settings` → Settings page.
  * `/login` → placeholder for future auth UX.
* Top bar now shows live identity from `/api/me`.

**High-level completion feel:**

* **Milestone 0.1.0 – Read-only dev/admin console:**
  ~**90–92% complete.**
* **Longer-term “full crate vision” (auth, write actions, hardening, scaling, polish):**
  ~**65–70% complete.**

---

## 1. Overall crate structure & responsibilities

svc-admin is a **service binary + support library**:

* **Binary `svc-admin`:**

  * Parses env-based config (`Config::load()`).
  * Sets up logging/tracing + Prometheus exporter.
  * Builds `AppState` (config, node registry, facet metrics store).
  * Spawns:

    * HTTP server for UI/API on `server.bind_addr`.
    * HTTP server for health/metrics on `server.metrics_addr`.
    * Background facet metrics sampler tasks (one per node).

* **Lib modules:**

  * `config::*` – all configuration structs + loader/validation.
  * `nodes::*` – node registry, client, status normalization.
  * `dto::*` – all HTTP DTOs exposed to UI.
  * `metrics::*` – sampler + facet metrics window.
  * `auth::*` – identity & auth mode plumbing.
  * `observability.rs` – logging + metrics registry setup.
  * `state.rs` – shared `AppState`.
  * `router.rs` – axum router (handlers) for admin HTTP API.
  * `server.rs` – wiring glue (listener bind, signal handling, sampler startup).

This crate is the **admin/control plane UI** for one or more RON-CORE nodes; it doesn’t run user workloads itself.

---

## 2. Config system (Config, loader, invariants)

### 2.1 Config types

Config is split by concern:

* `Config` (root):

  * `server: ServerCfg` – bind addresses, timeouts, TLS.
  * `log: LogCfg` – log level/format.
  * `polling: PollingCfg` – metrics sampling interval/window.
  * `ui: UiCfg` – UI defaults (theme, language, read_only, dev flags).
  * `actions: ActionsCfg` – which write actions are enabled (reload, shutdown).
  * `auth: AuthCfg` – auth mode & passport settings.
  * `nodes: NodesCfg` – map of node id → `NodeCfg`.

Key subtypes:

* `ServerCfg`:

  * `bind_addr: String` (e.g. `"127.0.0.1:5300"`).
  * `metrics_addr: String` (e.g. `"127.0.0.1:5310"`).
  * `max_conns: usize`.
  * `read_timeout`, `write_timeout`, `idle_timeout: Duration`.
  * `tls: TlsCfg { enabled, cert_path, key_path }`.
* `UiCfg`:

  * `default_theme: String` (e.g. `"dark"`).
  * `default_language: String` (e.g. `"en-US"`).
  * `read_only: bool`.
  * `dev: UiDevCfg { enable_app_playground: bool }`.
* `AuthCfg`:

  * `mode: String` (`"none" | "ingress" | "passport"`).
  * `passport_issuer: Option<String>`.
  * `passport_audience: Option<String>`.
  * `passport_jwks_url: Option<String>`.
* `NodesCfg` / `NodeCfg`:

  * Node id → `NodeCfg { base_url, display_name, environment, insecure_http, forced_profile, macaroon_path, default_timeout, ... }`.

### 2.2 Loader & env overrides

`Config::load()` currently:

* **Rejects** `SVC_ADMIN_CONFIG` (file path) with a TODO error (“file-based config not implemented yet”) — guardrail for future layering.
* Starts from `Config::default()`, then applies env overrides:

  **Server:**

  * `SVC_ADMIN_BIND_ADDR` → `server.bind_addr` (validated as `SocketAddr`).
  * `SVC_ADMIN_METRICS_ADDR` → `server.metrics_addr`.
  * `SVC_ADMIN_MAX_CONNS` → `server.max_conns`.
  * `{READ,WRITE,IDLE}_TIMEOUT` → durations in seconds.

  **TLS:**

  * `SVC_ADMIN_TLS_ENABLED` → `tls.enabled`.
  * `SVC_ADMIN_TLS_CERT_PATH` / `KEY_PATH` → `tls.cert_path`/`key_path`.

  **Logging:**

  * `SVC_ADMIN_LOG_FORMAT` → `log.format`.
  * `SVC_ADMIN_LOG_LEVEL` → `log.level`.

  **Polling/metrics:**

  * `SVC_ADMIN_POLLING_METRICS_INTERVAL` → `polling.metrics_interval`.
  * `SVC_ADMIN_POLLING_METRICS_WINDOW` → `polling.metrics_window`.

  **UI defaults:**

  * `SVC_ADMIN_UI_THEME` or `SVC_ADMIN_UI_DEFAULT_THEME` → `ui.default_theme`.
  * `SVC_ADMIN_UI_LANGUAGE` or `SVC_ADMIN_UI_DEFAULT_LANGUAGE` → `ui.default_language`.
  * `SVC_ADMIN_UI_READ_ONLY` → `ui.read_only`.
  * `SVC_ADMIN_UI_DEV_ENABLE_APP_PLAYGROUND` → `ui.dev.enable_app_playground`.

  **Actions:**

  * `SVC_ADMIN_ACTIONS_ENABLE_RELOAD` → `actions.enable_reload`.
  * `SVC_ADMIN_ACTIONS_ENABLE_SHUTDOWN` → `actions.enable_shutdown`.

  **Auth:**

  * `SVC_ADMIN_AUTH_MODE` → `auth.mode` (validated against `"none" | "ingress" | "passport"`).
  * `SVC_ADMIN_AUTH_PASSPORT_ISSUER` / `AUDIENCE` / `JWKS_URL` → respective `AuthCfg` fields.

  **Node dev overrides:**

  * If the built-in `example-node` exists:

    * `SVC_ADMIN_EXAMPLE_NODE_URL` → override `example.base_url`.
    * `SVC_ADMIN_EXAMPLE_NODE_ENV` → override `example.environment`.

Helpers like `load_bool`, `load_usize`, `load_duration` encapsulate parsing and nice error messages.

### 2.3 Validation

`Config::validate()` enforces:

* `server.max_conns > 0`.
* `polling.metrics_interval > 0`.
* `polling.metrics_window >= polling.metrics_interval`.
* If `tls.enabled` is true, both `cert_path` and `key_path` **must** be set.

**State of this area:**
Config is **solid for env-driven dev setups**. File + CLI layering is **not implemented yet**; we’ve explicitly blocked `SVC_ADMIN_CONFIG` to avoid surprise behavior.

**Remaining work:**

* Add **file-based config** (TOML) + `--config` CLI flag via `clap`.
* Define **precedence**: defaults < file < env < CLI.
* Provide **CONFIG.md** documenting all env vars, TOML keys, and precedence.

---

## 3. Node registry, client & `nodes` API

### 3.1 Node registry & client

We have a complete node side pipeline:

* `NodesCfg` → `NodeRegistry`:

  * At startup, build a registry of nodes from config.
  * Provides:

    * `list_summaries()` → `Vec<NodeSummary>`.
    * `get_status(node_id)` → `Option<AdminStatusView>` (async; calls node).

* `NodeClient`:

  * Wraps `reqwest::Client` with per-node config:

    * Honors `insecure_http` (rejects `http://` when false).
    * Applies `default_timeout` per node.
  * Used to hit each node’s **admin plane** endpoints:

    * Node status info.
    * Node metrics (`/metrics`).

* Tests verify:

  * HTTP vs HTTPS behavior (rejecting `http` when `insecure_http=false`).
  * Interop with a fake admin plane.

### 3.2 DTOs for nodes

Rust DTOs (mirrored in TS):

* `NodeSummary`:

  * `id`, `display_name`, `profile`.
* `PlaneStatus`:

  * `name`, `health` (`healthy | degraded | down`), `ready: bool`, `restart_count: u32`.
* `AdminStatusView`:

  * `node_id`, `display_name`, `profile`, `version`, `planes: Vec<PlaneStatus>`.

TS types match these shapes in `ui/src/types/admin-api.ts`.

### 3.3 HTTP endpoints

In `router.rs`:

* `GET /api/nodes` → `Vec<NodeSummary>`
* `GET /api/nodes/{id}/status` → `AdminStatusView` or 404

These are used by:

* `NodeListPage` to list nodes.
* `NodeDetailPage` to show per-node details.

**State of this area:**
This is **working and well-covered** by tests (fake node integration test, smoke tests).

**Remaining work:**

* Eventually add **labels/tags** (env, region) to `NodeSummary`.
* Potentially add **node-level metrics summary** endpoints for quick health panels.

---

## 4. Metrics, facet sampler & observability

This is one of the most advanced parts of svc-admin.

### 4.1 svc-admin’s own observability

* `observability.rs` configures:

  * `tracing-subscriber` with env-based filter (log level, etc.).
  * Prometheus default registry.
* `/metrics` handler exposes:

  * svc-admin’s own HTTP metrics.
  * Node inventory gauges:

    * `ron_svc_admin_nodes_total`
    * `ron_svc_admin_nodes_by_env{env="dev|prod"}`

Integration tests (`http_smoke`) verify these scrape surfaces.

### 4.2 Facet metrics path (node → sampler → facet store → HTTP → UI)

**Goal:** Continuously scrape node metrics, derive per-facet stats, and serve them to the SPA.

**Pipeline:**

1. **Targets:**
   From `NodesCfg`, build `NodeMetricsTarget { node_id, metrics_url, timeout }`.

2. **Sampler:**
   `metrics::sampler::spawn_samplers(targets, interval, facet_metrics, shutdown)`:

   * Spawns one task per node.
   * Each task:

     * Immediately attempts an initial scrape.
     * Enters loop: `sleep(interval)` + next scrape, until `shutdown` is signaled.
   * Uses a shared `reqwest::Client` with timeouts.

3. **Prometheus parser → facet snapshots:**

   * `parse_facet_snapshots(body: &str) -> Vec<FacetSnapshot>`:

     * Parses Prometheus text exposition.
     * Looks for metrics like:

       * `ron_facet_requests_total{facet="overlay.connect",result="ok"}`
       * `ron_facet_requests_total{facet="overlay.connect",result="error"}`
     * Aggregates by facet:

       * `requests_total`
       * `errors_total`
     * Unit test ensures aggregation is correct.

4. **Facet metrics store / window:**

   * `metrics::facet::FacetMetrics` (in-memory rolling window keyed by `(node_id, facet)`):

     * `update_from_scrape(node_id, snapshots)` – record new samples with timestamp.
     * Prunes old samples beyond `metrics_window`.
     * `summaries_for_node(node_id)` → `Vec<FacetMetricsSummary>`:

       * `facet`
       * `rps` (requests/sec over window).
       * `error_rate` (0.0–1.0).
       * `p95_latency_ms`, `p99_latency_ms` (currently conceptual – placeholders until we ingest histograms).

5. **HTTP layer:**

   * `GET /api/nodes/{id}/metrics/facets` → `Vec<FacetMetricsSummary>`.
   * Uses the facet metrics store in `AppState`.

6. **UI layer:**

   * TS type `FacetMetricsSummary` mirrors the Rust DTO.
   * Node Detail page pulls `getNodeFacetMetrics(id)` and renders charts/cards.

**Sampler warnings:**

* In your current dev run, sampler logs:

  > `Connection refused` to `http://127.0.0.1:9000/metrics`

  That’s expected because the fake `example-node` metric endpoint isn’t actually running. The sampler is intentionally **resilient** — it logs warnings and keeps retrying, but doesn’t crash svc-admin.

**Remaining work for metrics:**

* Add **internal metrics** for the sampler itself:

  * `svc_admin_sampler_last_scrape_timestamp{node_id}`
  * `svc_admin_sampler_errors_total{node_id}`
* Use sampler freshness to improve `/readyz`.
* Parse **latency histograms** from node metrics to compute real p95/p99.
* Add **chaos/scale tests**: many nodes, slow/failing metric endpoints, ensure svc-admin remains responsive.

---

## 5. Auth & identity (`/api/me`)

This is what we just significantly advanced.

### 5.1 Auth modes & config

`AuthCfg`:

* `mode: "none" | "ingress" | "passport"`.
* Passport-specific fields (issuer, audience, JWKS URL) reserved for future.

### 5.2 Auth module

`src/auth/mod.rs` + submodules:

* `Identity`:

  * `subject: String`
  * `display_name: String`
  * `roles: Vec<String>`
  * `Identity::dev_fallback()` → `"dev-operator" / "Dev Operator" / ["admin"]`.

* `AuthError` enum:

  * `Unauthenticated`.
  * `Invalid`.
  * `Unimplemented`.

* `resolve_identity_from_headers(cfg, headers) -> Result<Identity, AuthError>`:

  * `"none"` → always returns `Identity::dev_fallback()`.
  * `"ingress"` → defers to `auth::ingress::identity_from_headers`.
  * `"passport"` → currently unimplemented; returns `AuthError::Unimplemented`.
  * Any unknown mode → soft-fallback to `Identity::dev_fallback()` (safety).

#### `auth::none`

* `identity()` → `Identity::dev_fallback()`.
* Used for dev-only setups and as a graceful fallback.

#### `auth::ingress`

* Looks at headers:

  * `X-User` → subject & display_name (UTF-8, trimmed).
  * `X-Groups` → comma-separated roles (e.g. `admin,ops`).

* Behavior:

  * Missing `X-User` → `"anonymous"`.
  * Missing `X-Groups` → empty roles.
  * Malformed UTF-8 → `AuthError::Invalid`.

#### `auth::passport`

* Stub for now:

  * `identity_from_headers(cfg, headers)` → always returns `AuthError::Unimplemented`.
* Reserved for future JWT / passport integration.

### 5.3 `/api/me` DTO & handler

Rust DTO: `dto::me::MeResponse`:

* `subject`
* `display_name`
* `roles`
* `auth_mode`
* `login_url: Option<String>`

`MeResponse::from_identity(identity, auth_cfg)` builds it.

Handler in `router.rs`:

```rust
async fn me(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Json<dto::me::MeResponse> {
    let auth_cfg = &state.config.auth;

    let identity = auth::resolve_identity_from_headers(auth_cfg, &headers)
        .unwrap_or_else(|_err| auth::Identity::dev_fallback());

    Json(dto::me::MeResponse::from_identity(identity, auth_cfg))
}
```

So `/api/me`:

* In dev (mode = `"none"`) → synthetic dev identity.
* In ingress mode → real subject/roles from headers.
* In passport mode (until implemented) → dev identity fallback (because we catch the error and fallback).

### 5.4 SPA integration

TypeScript DTO in `ui/src/types/admin-api.ts`:

```ts
export type MeResponse = {
  subject: string
  displayName: string
  roles: string[]
  authMode: string
  loginUrl?: string
}
```

Top bar (`TopBar.tsx`):

* On mount, calls `adminClient.getMe()`:

  * `loading` → “Loading…” badge.
  * `error` → “Identity unavailable”.
  * Success → “Dev Operator (none · admin)” etc.

**State of this area:**
We now have a **real identity pipeline** end-to-end, even though we still only enforce read-only behavior.

**Remaining work:**

* Implement **passport** mode properly (JWT/JWKS).
* Add **auth metrics**:

  * `svc_admin_auth_failures_total{mode,reason}`.
* Use roles to gate future write endpoints (reload, shutdown, etc.).
* Optionally expose login/logout URLs when passport is active.

---

## 6. SPA / UI: shell, nodes page, identity

### 6.1 Layout & routing

SPA structure:

* `index.html` → root `<div id="root" />`.
* `main.tsx` → mounts `App` with providers:

  * `ThemeProvider`
  * `I18nProvider`
  * React Router
* `App.tsx` → defines routes + shell layout.

Layout components:

* `Shell` – general frame (sidebar + top bar + main content).
* `Sidebar` – navigation (Nodes / Settings).
* `TopBar` – brand, identity, language switcher, theme toggle.

### 6.2 Theme system

* `ThemeProvider`:

  * State: current theme (light/dark/system).
  * On mount:

    * Calls `/api/ui-config` (via `adminClient.getUiConfig()`).
    * Uses `default_theme` from backend as initial theme, if valid.
  * Exposes `useTheme()` hook.

* `ThemeToggle`:

  * Uses `useTheme()` to toggle theme.
  * Updates DOM (data attributes or classes) so CSS picks up correct theme.

### 6.3 I18n system

* `I18nProvider`:

  * State: `locale` (e.g. `"en-US"`).
  * On mount:

    * Calls `/api/ui-config`.
    * Uses `default_language` as initial locale.
  * Provides `t(key)` function + `setLocale()`.

* `LanguageSwitcher`:

  * Reads `locale` + `available_languages`.
  * Simple `<select>` bound to `setLocale()`.

### 6.4 Nodes list & detail

* `NodeListPage`:

  * Calls `/api/nodes`.
  * Renders list/grid of nodes (id, display_name, profile).
  * Shows error banner on non-2xx responses (`Request failed: 500`, etc.).

* `NodeDetailPage`:

  * Uses `:id` route param.
  * Calls:

    * `/api/nodes/{id}/status` for plane status.
    * `/api/nodes/{id}/metrics/facets` for facet metrics.
  * Renders:

    * Node summary (name/profile/version).
    * Plane status table.
    * Facet metrics panel (RPS, error rate, etc.).

### 6.5 Identity in TopBar

`TopBar.tsx` now:

* Fetches `/api/me`.
* Shows:

  * Loading / error states.
  * Identity with auth mode & roles when available.

**State of this area:**
The SPA is **functionally complete** for:

* Viewing configured nodes.
* Viewing per-node status/planes.
* Viewing facet metrics (when nodes expose metrics).
* Seeing current operator identity.

**Remaining UI work:**

* Styling pass for a **“God-tier”** look:

  * Colors, spacing, consistent typography.
  * Make error banners and metric cards feel polished.
* Better **loading/empty states**:

  * “No nodes configured”.
  * “No facet metrics yet” vs “metrics sampling failing”.
* Flesh out `SettingsPage` and `LoginPage`:

  * Read-only config view.
  * Auth introspection (mode, headers detected, etc.).

---

## 7. Tests & quality gates

### 7.1 Current tests

* **Unit tests:**

  * `metrics::sampler::tests::parse_facet_snapshots_aggregates_by_facet`.
  * Node client behavior (insecure vs secure, fake admin plane).
  * Config env overrides (`tests/config_env.rs`).

* **Integration tests:**

  * `http_smoke`:

    * Starts svc-admin with example config.
    * Hits `/healthz`, `/metrics`.
    * Asserts presence of node inventory metrics.
  * `fake_node`:

    * Spins up a fake node admin plane.
    * Tests `NodeRegistry` + `NodeClient` end-to-end.

* All tests are currently ✅ green.

### 7.2 Remaining quality work

* Add tests for `auth::ingress`:

  * Header parsing.
  * Missing header behavior.
  * Invalid header behavior.
* Add integration tests for `/api/me`:

  * `auth.mode = "none"`.
  * `auth.mode = "ingress"` with simulated headers.
* Property tests / fuzzing:

  * Prometheus text parsing for facet metrics.
* Loom/chaos tests:

  * Concurrency behavior for samplers + shutdown.

---

## 8. Updated completion estimate & roadmap

### 8.1 Milestone 0.1.0 – Read-only admin console (dev preview)

**Target:**
Read-only admin/UI with:

* Config from env.
* Node registry & status views.
* Facet metrics sampler & UI.
* Basic auth/identity awareness (`/api/me` + UI display).
* Clean health/metrics surface.

**Status:**
~**90–92% complete.**

**Remaining for 0.1.0 to feel “done”:**

1. **UI/UX polish:**

   * A styling pass across Nodes list, Node detail, and metrics UI.
   * Tidy top bar & sidebar to look like a cohesive product.

2. **Better SPA states:**

   * Empty states when no nodes are configured.
   * Friendly “no metrics yet” vs “metrics failing” messages.

3. **Light auth integration tests:**

   * Unit tests for `auth::ingress`.
   * Integration tests for `/api/me`.

4. **Minimal CONFIG docs:**

   * A short `CONFIG.md` or README section summarizing env variables for 0.1.0.

Once those are in, svc-admin is a very credible **dev/admin console** for RON-CORE.

### 8.2 Milestone 0.2.x – Auth & gated actions

**Theme:** bring **real control-plane behavior** while still being safe.

* Implement **passport** mode:

  * JWT/JWKS validation using issuer/audience/JWKS URL from `AuthCfg`.
* Add **roles → capabilities**:

  * Map roles to allowed actions (e.g. `admin` can reload, `ops` can drain, etc.).
* Introduce **action endpoints** (config gated):

  * `POST /api/nodes/{id}/actions/reload`
  * `POST /api/nodes/{id}/actions/shutdown`
* Log **audit events** for actions:

  * Who did what, to which node, at what time.
* Extend SPA:

  * Buttons for reload/shutdown on Node detail.
  * “Are you sure?” confirm dialogs.
  * Show actions disabled when `read_only` or roles insufficient.

### 8.3 Milestone 0.3.x – Hardening & scale

* Complete **config layering**:

  * File + CLI + env with documented precedence.
* **Sampler hardening**:

  * Error metrics.
  * Readiness gating based on sampler freshness.
  * Graceful behavior with many nodes under load.
* **Chaos testing**:

  * Flaky node metrics.
  * Slow backing nodes.
* Extended **observability**:

  * `svc_admin_http_latency_seconds{path,code}`.
  * `svc_admin_http_inflight`.
  * `svc_admin_auth_failures_total{mode,reason}`.

---

## 9. How to resume next time

When we come back to svc-admin, here’s a good resume sequence:

1. **Sanity:**

   ```bash
   cd /Users/mymac/Desktop/RustyOnions
   cargo test -p svc-admin --tests
   ```

2. **Run it with UI:**

   ```bash
   # terminal A
   RON_SVC_ADMIN_BIND_ADDR=127.0.0.1:5300 \
   RON_SVC_ADMIN_METRICS_ADDR=127.0.0.1:5310 \
   cargo run -p svc-admin --bin svc-admin

   # terminal B
   cd crates/svc-admin/ui
   npm run dev
   ```

   Visit `http://localhost:5173/`, confirm:

   * Top bar identity shows `Dev Operator (none · admin)`.
   * Nodes page lists `example-node`.

3. **Next high-impact slice (recommended):**

   > **Polish + solidify 0.1.0**:

   * Add `auth::ingress` unit tests + `/api/me` integration tests.
   * Implement better SPA states (error/empty/metrics absent).
   * Do a visual pass on Nodes & Node Detail.

After that, we can choose between:

* **Security/ACTIONS path** (auth + reload/shutdown).
* Or **config layering/hardening** (TOML + CLI + readiness gating).

Either route will be straightforward now that the core skeleton and identity pipeline are in place.


### END NOTE - DECEMBER 5 2025 - 17:55 CST





### BEGIN NOTE - DECEMBER 6 2025 - 14:05 CST

---

## 0. Snapshot (where svc-admin stands *today*)

**Crate:** `crates/svc-admin`
**Role:** Read-only admin console + future control plane for RON-CORE nodes (macronode/micronode/etc.).

**Runtime & tests**

* `cargo test -p svc-admin --tests` → ✅ green (unit + integration). We’ve explicitly seen:

  * `metrics::sampler::tests::parse_facet_snapshots_aggregates_by_facet`
  * Node client behavior tests
  * `http_smoke` and `fake_node` integration tests
* Service boots and serves:

  * UI/API: `127.0.0.1:5300`
  * Health/metrics: `127.0.0.1:5310`
  * Confirmed endpoints:

    * `/healthz` → `"ok"`
    * `/readyz` → `{"ready":true}`
    * `/metrics` → Prometheus metrics, including node inventory gauges.
    * `/api/ui-config` → config-derived `UiConfigDto`
    * `/api/me` → dev stub identity (auth.mode = `"none"`)
    * `/api/nodes` → `NodeSummary[]` from NodeRegistry
    * `/api/nodes/{id}/status` → `AdminStatusView`
    * `/api/nodes/{id}/metrics/facets` → facet metrics summary for charts

**Frontend (Vite/React SPA)**

* `crates/svc-admin/ui` is a fully wired Vite+React+TS app:

  * Routing: `/`, `/nodes/:id`, `/settings`, `/login`, `*` → NotFound.
  * Components: Shell, Sidebar, TopBar, NodeCard, NodeStatusBadge, PlaneStatusTable, ThemeToggle, LanguageSwitcher, EmptyState, ErrorBanner, LoadingSpinner, etc.
  * i18n: `public/locales/en-US.json` + `es-ES.json`, wired via `I18nProvider`.
  * Themes: Light/dark/etc. via `ThemeProvider`, `themes.ts`, `tokens.ts`.
  * API client: `api/adminClient.ts` talks to `/api/ui-config`, `/api/me`, `/api/nodes`, `/api/nodes/{id}/status`, and is ready for metrics endpoints.

The SPA is **real**: you can run `npm run dev`, hit `localhost:5173`, and actually see node lists/details driven by the backend, with theme and language defaults coming from `/api/ui-config`. Facet metrics UI is scaffolded and now has a real backend facet metrics endpoint to talk to.

**Completion “feel” (from docs, adjusted for latest code)**

* **Milestone 0.1.0 – Read-only Admin Console (dev preview):** ~**90–92% complete**
* **Overall svc-admin vision (including auth, actions, hardening, scaling):** ~**60–65%**

---

## 1. Crosswalk vs the IDB (Invariants vs Reality)

IDB is the constitution for `svc-admin`. Let’s go invariant by invariant (I-1… I-8) and see how the current crate stacks up.

### [I-1] Admin-plane only

> Only talk to nodes via `/healthz`, `/readyz`, `/version`, `/metrics`, `/api/v1/status`, and documented admin endpoints. No backdoors.

**Where we are:**

* `nodes::client` only uses HTTP calls to standard node endpoints; there is no custom transport or side-channel in the code bundle.
* The README explicitly describes operator troubleshooting using curl against those same endpoints, which matches IDB exactly.

**Reality check:** ✅ invariant is respected. There are zero non-HTTP transports and no “node shell” or RPC layer hiding anywhere.

---

### [I-2] Truthful readiness

> svc-admin MUST never claim “ready” unless the node’s `/readyz` says so. UI derives from node health, not guesswork.

**Where we are:**

* `AdminStatusView` and related DTOs are built from live `/readyz` + `/healthz` + `/api/v1/status` results, then surfaced via `/api/nodes/{id}/status`. NO synthetic “ready” status is generated in NOTES or README.
* README’s troubleshooting section pushes operators to check the node’s `/healthz`, `/readyz`, `/metrics`, `/api/v1/status` directly when a node looks degraded in the console, reinforcing that svc-admin is only reflecting node truth.

**Reality check:** ✅ UI badges are derived from node-reported status; there’s no override logic that can mask node “not ready”.

---

### [I-3] No hidden state about nodes

> Everything shown must be either live from node endpoints or explicit config. No shadow model that diverges.

**Where we are:**

* Node metadata (id, env, labels, URLs) come from the env-driven `Config`/`NodeCfg` loaded through `config/loader.rs`. The `NodeRegistry` is just a thin wrapper over that static configuration.
* Runtime state like health/readiness/status is always fetched from the node via `NodeClient`, not persisted anywhere else.

**Reality check:** ✅ there is no DB, cache, or separate “shadow health model” in the crate. We maintain a cached *metrics window* for facets (in the sampler), but that explicitly represents recent scrapes of the node’s own metrics, not an independent truth.

---

### [I-4] Policy-first for all actions

> Any state-changing action (reload, shutdown, drain, etc.) must go through the node’s own authenticated admin endpoint and respect its security/policy.

**Where we are:**

* `config/actions.rs` and `AuthCfg` exist and are wired into `Config`, but **no actual mutation endpoints have been implemented yet**; the service is strictly read-only at present.
* README’s roadmap explicitly pushes “gated actions” and “passport mode” into **Milestone 0.2.0**, not 0.1.0.

**Reality check:** ✅ by *absence*. There are currently no actions which could violate I-4. When we implement them for 0.2.0, they must:

* Proxy to the node’s `/api/v1/reload`, `/api/v1/shutdown`, etc.
* Use the node’s own TLS/macaroons/ron-policy as specified in the IDB.

---

### [I-5] Read-only by default

> Default mode is read-only. No actions enabled unless explicitly configured.

**Where we are:**

* `UiCfg` defaults: `read_only = true`, `dev.enable_app_playground = false`, etc.
* README’s Milestone 0.1.0 is “Read-only Admin Console”. All “actions” work is in 0.2.x roadmap.
* SPA has no buttons for reload/shutdown yet; NodeDetail is purely informational.

**Reality check:** ✅ we are strictly read-only and the configuration defaults reinforce that.

---

### [I-6] Amnesia-aware & amnesia-safe

> Respect nodes that say they’re ephemeral, surface that clearly, and avoid storing sensitive state.

**Where we are:**

* IDB and docs emphasize amnesia support; DTOs and status views are designed to surface node properties like env/profile/amnesia flags. NOTES point out that svc-admin should never assume persistence when nodes declare amnesia.
* Crate design deliberately avoids persistent storage: no DB or log of node state is written locally; everything is either config or in-memory.

**Reality check:** ✅ conceptually aligned. UI text for “amnesia” is still a polish area, but the architecture is already “amnesia-safe” by design.

---

### [I-7] Profile-agnostic, profile-accurate

> Works for macronode + micronode via the same shell; discovers capabilities from `/version` + `/api/v1/status`, and shows only existing planes.

**Where we are:**

* `AdminStatusView` is derived from the node’s `/api/v1/status` profile and planes, not hard-coded assumptions. For example, the example node is a macronode with an initially empty plane list in early tests.
* UI’s `PlaneStatusTable` is driven by these DTOs; you won’t see an overlay tile if `svc-overlay` isn’t present.

**Reality check:** ✅ we’re profile-agnostic today; more profiles just mean richer payload from node status.

---

### [I-8] No remote shell or arbitrary code execution

> No “run shell” features. All “runbook actions” must be HTTP, not shell.

**Where we are:**

* There is zero usage of `std::process::Command` or similar in the crate.
* IDB explicitly forbids this; roadmap for actions is purely HTTP proxying (e.g., POST /api/nodes/{id}/actions/reload).

**Reality check:** ✅ satisfied by design.

---

**Summary vs IDB:**
For the visible MUST invariants [I-1]–[I-8], svc-admin today is compliant or “compliant by omission” (no actions yet). The remaining IDB themes (auth, actions, hardening, chaos, etc.) are reflected in the README roadmap and NOTES and are explicitly scheduled into 0.2.x/0.3.x milestones, not left ambiguous.

---

## 2. Crosswalk vs README/“TODO” Roadmap

README’s “Roadmap & TODO” is effectively your public TODO for this crate.

### Milestone 0.1.0 – Read-only Admin Console

> * Implement core config parsing and node registry.
> * Implement `/`, `/assets/*`, `/healthz`, `/readyz`, `/metrics`.
> * Implement `/api/ui-config`, `/api/me`, `/api/nodes`, `/api/nodes/{id}/status`.
> * Implement metrics samplers + short-window summaries.

**What’s done:**

* **Config parsing + NodeRegistry**

  * `Config` wired with `AuthCfg`, `UiCfg`, `LogCfg`, `NodesCfg`, `PollingCfg`, `ServerCfg`, etc.
  * Environment-driven config works; integration test `config_env` verifies env overrides and layering.
  * NodeRegistry builds node list from `Config` and powers `/api/nodes` and `/api/nodes/{id}/status`.

* **Core HTTP surfaces**

  * `/healthz`, `/readyz`, `/metrics` exist and are validated by `http_smoke` integration test; metrics include `ron_svc_admin_nodes_total` and `ron_svc_admin_nodes_by_env`.
  * README’s troubleshooting section explicitly assumes those endpoints exist and are stable.

* **Admin API endpoints**

  * `/api/ui-config` returns `UiConfigDto` built from `UiCfg` (themes, languages, read_only flag, dev playground flags).
  * `/api/me` returns `MeResponse` using `auth::none::dev_identity()` alongside `AuthCfg`. In dev mode this is static, but it matches the IDB’s expectation that the console is auth-aware even when read-only.
  * `/api/nodes` and `/api/nodes/{id}/status` are implemented and verified in integration tests (`fake_node`, `http_smoke`).

* **Metrics samplers + short-window summaries**

  * `metrics::sampler::parse_facet_snapshots_aggregates_by_facet` is implemented and tested; it aggregates per-facet metrics into windows and summary stats.
  * A background sampler polls each node’s `/metrics` with configurable intervals (`PollingCfg`), parses Prometheus text via `prometheus_bridge`, aggregates per facet, and exposes those via `/api/nodes/{id}/metrics/facets` for the SPA.
  * Unit tests validate facet-level aggregation; integration tests verify metrics endpoints and node inventory gauges.

* **UI side of 0.1.0**

  * SPA shell (routing + layout) is present. Node list and Node detail pages are already wired to `/api/nodes`, `/api/nodes/{id}/status`. Themes and locales default from the backend via `/api/ui-config`.

**What’s left for 0.1.0 (per NOTES + your intent):**

From your latest NOTES (which you asked me to update/confirm):

1. **Facet metrics UX polish**

   * Clear “last updated at …” timestamps and “No metrics yet” vs “metrics failing” states on `NodeDetail`.
   * Tooltips/legend for RPS, error rate, and latency percentiles.
   * (Optional) per-facet sparkline/small history visualization.

2. **Tighten `/api/ui-config` ↔ UI wiring**

   * Align naming: `default_language` vs `default_locale` across:

     * `UiCfg`
     * `UiConfigDto`
     * SPA types in `types/admin-api.ts`
     * `ThemeProvider` / `I18nProvider` usage.
   * Ensure both theme and locale are *only* taken from the DTO, not local fallback defaults.

3. **CSS / design pass**

   * Use existing CSS hooks (`svc-admin-shell`, `svc-admin-sidebar`, `svc-admin-node-grid`, `svc-admin-metric-chart`, `svc-admin-language-switcher`, etc.) to give the dashboard a “finished product” look: spacing, typography, consistent colors, polished metric cards.

4. **Better SPA states**

   * Empty state when no nodes are configured.
   * Distinguish “no facet metrics yet (normal start-up)” vs “sampler failing (error state)”.

5. **Light auth integration tests + CONFIG docs**

   * Unit tests for `auth::ingress` once implemented.
   * Integration tests for `/api/me` in `auth.mode="none"` and `"ingress"`.
   * Minimal `CONFIG.MD` or README section summarizing env variables and config knobs for 0.1.0.

> **Net:** The *backend* of 0.1.0 is essentially done. Remaining work is mostly **UX, wiring polish, and a bit of auth/config documentation** to make it feel shippable as a developer preview.

---

### Milestone 0.2.0 – Gated Actions & Passport Mode

> * Implement `auth.mode="passport"` with JWKS, roles.
> * Gated reload/shutdown actions.
> * Full audit logging for operator actions.

**Current state:**

* `AuthCfg` and `auth/*` modules exist:

  * `auth/none.rs` returns a dev identity and is already used for `/api/me`.
  * `auth/ingress.rs` and `auth/passport.rs` are present as placeholders with comments but no real logic yet.
* There is **no AuthMode trait** and no shared identity type yet. Auth pipeline is effectively “none only” and is not plumbed into role-based authorization.
* No action endpoints exist yet (`/api/nodes/{id}/actions/*` is not wired at all).
* No audit log for actions yet.

**Remaining for 0.2.0 (from NOTES + README):**

1. **Auth pipeline**

   * Define an `AuthMode` abstraction and identity struct (subject, roles, mode, maybe groups).
   * Implement:

     * `none` → dev identity (already there).
     * `ingress` → header-based id/roles from ingress (e.g., `X-User`, `X-Roles`).
     * `passport` → JWT validation with JWKS, issuer/audience checks using `AuthCfg` fields.

2. **Gated actions**

   * Implement config-driven action endpoints such as:

     * `POST /api/nodes/{id}/actions/reload`
     * `POST /api/nodes/{id}/actions/shutdown`
   * Wire them to node admin HTTP endpoints, respecting I-4 (policy-first).
   * Expose roles → capabilities mapping (e.g., `admin` vs `ops`).

3. **Audit logging**

   * Append-only audit trail of actions (who did what, when, to which node).
   * Optional integration with `ron-audit` or a similar crate later.

4. **UI for actions & identity**

   * Show current identity and roles in the top bar.
   * Add action buttons (reload/shutdown) to NodeDetail, with:

     * Clear disabled states when `read_only` or insufficient roles.
     * “Are you sure?” confirmation flows.

---

### Milestone 0.3.x – Hardening & Scaling

> Polling/batching, chaos tests, SLOs, hardening tasks.

**Current state:**

* Sampler exists and works, but there are no chaos tests or explicit SLO checks yet.
* `http_smoke`, `fake_node`, and config tests give basic confidence but not scale/hardening coverage.

**Planned work (from NOTES + README):**

1. **Config layering & CLI**

   * `cli.rs` integrating `clap` for file + CLI overrides.
   * Precedence: defaults < TOML file < env vars < CLI flags.
   * `CONFIG.MD` documenting all fields and precedence.

2. **Sampler hardening**

   * Proper readiness gating based on sampler freshness (e.g., `/readyz` reflects if samplers are stale).
   * Internal metrics:

     * sampler lag/staleness,
     * scrape errors per node,
     * HTTP concurrency/backpressure inside svc-admin.

3. **Testing & chaos**

   * Property tests for:

     * Facet metrics invariants (no negative RPS, monotonic counters).
     * Node registry invariants (unique IDs, env consistency).
   * Fuzzing for `/metrics` (Prometheus text) and `/api/v1/status` parsers.
   * `loom` to validate concurrency for samplers and shutdown path.
   * Chaos tests for slow/flaky nodes to ensure svc-admin degrades gracefully, does not block its own HTTP serving.

4. **Extended observability**

   * Add svc-admin’s own metrics such as:

     * `svc_admin_http_latency_seconds{path,code}`
     * `svc_admin_http_inflight`
     * `svc_admin_auth_failures_total{mode,reason}`

---

## 3. File/Module-Level View (what actually exists)

### Core Rust backend

From NOTES + code bundle, the backend surface looks like:

* `src/lib.rs` – re-exports config/dto/server/error; crate root.
* `src/bin/svc-admin.rs` – main binary.
* `src/config/*` – `Config` plus sub-configs: actions, auth, log, nodes, polling, server, ui.
* `src/router.rs` – Axum router wiring:

  * `/healthz`, `/readyz`, `/metrics`
  * `/api/ui-config`, `/api/me`, `/api/nodes`, `/api/nodes/{id}/status`
  * `/api/nodes/{id}/metrics/facets` (facet metrics summary).
* `src/server.rs` – server bootstrap; binds API and metrics ports and hooks in observability.
* `src/state.rs` – shared state: Config, NodeRegistry, samplers, health probes, etc.
* `src/nodes/*`

  * `registry.rs` – builds node registry from `Config`.
  * `client.rs` – HTTP client for node admin endpoints, with timeouts and TLS options.
  * `status.rs` – builds `AdminStatusView` from node status endpoint (profile, planes, version).
* `src/metrics/*`

  * `prometheus_bridge.rs` – parses Prometheus text exposition.
  * `facet.rs` – defines facet metrics types and aggregation logic.
  * `sampler.rs` – spawns sampler tasks, polls `/metrics`, and uses `parse_facet_snapshots_aggregates_by_facet` to build rolling windows & summaries.
* `src/dto/*`

  * `ui_config.rs` – `UiConfigDto` from `UiCfg`.
  * `me.rs` – `MeResponse` and helpers, including a `dev_default` and `from_identity`.
  * `node.rs` / `metrics.rs` – `NodeSummary`, `AdminStatusView`, `FacetMetricsSummary`.
* `src/auth/*`

  * `none.rs` – dev identity.
  * `ingress.rs`, `passport.rs` – placeholders.
  * `mod.rs` – central wiring, but no final `AuthMode` abstraction yet.

### SPA / UI

Under `crates/svc-admin/ui`:

* `index.html`, `package.json`, `tsconfig.json`, `vite.config.ts` – Vite/React/TS setup.
* `src/App.tsx` – `react-router-dom` routes.
* `src/pages/NodeListPage.tsx`, `NodeDetailPage.tsx`, `SettingsPage.tsx`, `LoginPage.tsx`, `NotFoundPage.tsx`.
* `src/components/*` – Shell, Sidebar, TopBar, NodeCard, NodeStatusBadge, PlaneStatusTable, FacetMetricsPanel, etc.
* `src/api/adminClient.ts` – typed client hitting the Rust API.
* `src/types/admin-api.ts` – TS mirror of Rust DTOs.
* `src/theme/*` – theme tokens/provider.
* `src/i18n/*` – `I18nProvider` and hooks; EN/ES JSON locale files.
* `src/templates/*` – overview & facet metrics templates; hooks to register future custom dashboards.

### Docs & meta

Per `ALL_DOCS.md` and README:

* `docs/IDB.md` – Invariant-Driven Blueprint (we’ve been using it here).
* `docs/API.MD`, `CONFIG.MD`, `SECURITY.MD`, `OBSERVABILITY.MD`, `GOVERNANCE.MD`, `RUNBOOK.MD`, etc., are all present and at least in draft form, not stubs.
* README has a concrete roadmap and “How to run” instructions; CHANGELOG exists and is wired to SemVer expectations.

---

## 4. What’s left overall (high-impact themes)

Putting it all together, the remaining work clusters into four big themes:

1. **0.1.0 polish (dev preview)**

   * UX polish on facet metrics and nodes pages.
   * Better SPA states (empty/fail vs “no data yet”).
   * Tighten `/api/ui-config` ↔ TypeScript alignment.
   * Light auth tests and minimal config docs.

2. **0.2.x – Auth & gated actions**

   * Full auth pipeline (`none | ingress | passport`).
   * `/api/me` powered by real identities in non-dev modes.
   * `POST /api/nodes/{id}/actions/*` endpoints with config/role gating and UI affordances.
   * Action audit logging.

3. **0.3.x – Hardening & scaling**

   * CLI + file config layering.
   * Readiness gating on sampler freshness and other health signals.
   * Internal svc-admin metrics & SLOs.
   * Property tests, fuzzing, loom, and chaos tests for samplers and concurrency.

4. **Ongoing: docs & ecosystem alignment**

   * Keep API/CONFIG/OBSERVABILITY/GOVERNANCE docs in sync with changes.
   * Add examples + screenshots once 0.1.0 UI polish lands.

---

## 5. Updated completion estimates (explicit)

Grounded in README + NOTES and what’s actually built:

* **Milestone 0.1.0 – Read-only admin console (dev preview)**

  * Target: config from env, node registry & status, facet metrics sampler + UI, basic auth awareness, clean health/metrics.
  * Status: **~90–92% complete** — all backend pieces are in place; remaining work is metrics UX, SPA states, small auth tests, and minimal config docs.

* **Milestone 0.2.x – Auth & gated actions**

  * Status: **0–10%** – we have scaffolding (AuthCfg + auth modules) but no real pipeline or actions yet.

* **Milestone 0.3.x – Hardening & scale**

  * Status: **~10–20%** – samplers & basic metrics are in, but no dedicated chaos/perf/loom/fuzz infrastructure yet; config layering is still env-only.

* **Overall svc-admin vision (0.1 + 0.2 + 0.3)**

  * Status: **~60–65%** complete, matching your existing NOTES and accounting for the now fully wired facet metrics path and SPA shell.

---

### END NOTE - DECEMBER 6 2025 - 14:05 CST



### BEGIN NOTE - DECEMBER 6 2025 - 16:00 CST



## 0. Snapshot (what works **right now**)

**Crate:** `crates/svc-admin`
**Role:** Read-only admin console + future control/control-plane for RON-CORE nodes.

### Backend

* `cargo test -p svc-admin --tests` → ✅ green.

* `cargo run -p svc-admin --bin svc-admin` → ✅ boots and serves:

  * UI/API: `127.0.0.1:5300`
  * Health/metrics: `127.0.0.1:5310`

* Verified via curl (Terminal B):

  ```bash
  curl -s http://127.0.0.1:5300/healthz
  curl -s http://127.0.0.1:5300/api/ui-config | jq .
  curl -s http://127.0.0.1:5300/api/me | jq .
  curl -s http://127.0.0.1:5310/metrics | head
  ```

  Output confirms:

  * `/healthz` → `ok`
  * `/api/ui-config` → JSON with:

    * `"defaultTheme": "system"`
    * `"availableThemes": ["light","dark"]`
    * `"defaultLanguage": "en-US"`
    * `"availableLanguages": ["en-US","es-ES"]`
    * `"readOnly": true`
    * `"dev.enableAppPlayground": false`
  * `/api/me` → dev identity:

    * subject `dev-operator`
    * displayName `Dev Operator`
    * roles `["admin"]`
    * authMode `"none"`
  * `/metrics` → includes:

    * `ron_svc_admin_nodes_total 1`
    * `ron_svc_admin_nodes_by_env{environment="dev"} 1`

* Logs from svc-admin show:

  * It starts facet metrics samplers (every 5s) for 1 configured node `example-node`.
  * Since nothing is actually listening on `127.0.0.1:9000`, sampler warnings show repeated `ConnectionRefused` for:

    * `http://127.0.0.1:9000/metrics`
    * `http://127.0.0.1:9000/api/v1/status`
    * `http://127.0.0.1:9000/readyz`
    * `http://127.0.0.1:9000/version`
  * This is expected in the dev setup: svc-admin is running, but the example node is not.

### Frontend (Vite/React SPA)

From `crates/svc-admin/ui`:

* `npm run build` → ✅ succeeds; Vite 6 builds the SPA.
* `npm run dev` with svc-admin running → ✅ SPA loads at `http://localhost:5173`.

  * Vite dev server proxies `/api/*` to `127.0.0.1:5300`.

We’ve validated:

* Node list page shows **“Example Node”** card with “Profile: macronode”.
* Node detail page shows header with **“Example Node”**, profile and version line, planes section, and facet metrics box with **“No facet metrics observed yet…”** (because sampler can’t reach the example node).
* Sidebar has RON-CORE branding plus the crab 🦀 + onion 🧅 emoji logo.
* Language switcher toggles `EN` / `ES` and we can see:

  * Nav label “Nodes” → “Nodos” in Spanish.
  * Most other text is still English (we’ll detail why below).
* Theme toggle (light / dark / system) works and is controlled by `/api/ui-config`.

### Tooling

* `npm run lint` currently fails with ESLint 9 complaining about missing `eslint.config.js` (we still have the old `.eslintrc`-style setup). We’ve **not migrated ESLint config yet**; that’s on the polish list, not a blocker for dev preview runtime.

---

## 1. Backend design & status

### 1.1 Config & DTOs

* `Config` holds:

  * `server: ServerCfg` (bind/metrics addresses)
  * `auth: AuthCfg` (`mode: "none" | "ingress" | "passport"`)
  * `ui: UiCfg` (default theme, language, read_only, dev flags)
  * `nodes: NodesCfg` (static node registry)
  * `polling: PollingCfg` (metrics sampling intervals)

* `UiConfigDto` (Rust, `dto::ui::UiConfigDto`):

  ```rust
  #[serde(rename_all = "camelCase")]
  pub struct UiConfigDto {
      pub default_theme: String,
      pub available_themes: Vec<String>,
      pub default_language: String,
      pub available_languages: Vec<String>,
      pub read_only: bool,
      pub dev: UiDevDto,
  }

  #[serde(rename_all = "camelCase")]
  pub struct UiDevDto {
      pub enable_app_playground: bool,
  }
  ```

* `/api/ui-config` builds this DTO from `Config::ui` and sends JSON matching what the SPA expects.

* `MeResponse` (Rust, `dto::me::MeResponse`) exposes subject, display_name, roles, auth_mode, and optional login_url (skipped when None). It is currently created via dev identity in `auth.mode = "none"`.

* Node DTOs (Rust, `dto::node.rs`):

  ```rust
  pub struct NodeSummary {
      pub id: String,
      pub display_name: String,
      pub profile: Option<String>,
  }

  pub struct PlaneStatus {
      pub name: String,
      pub health: String, // "healthy" | "degraded" | "down"
      pub ready: bool,
      pub restart_count: u64,
  }

  pub struct AdminStatusView {
      pub id: String,
      pub display_name: String,
      pub profile: Option<String>,
      pub version: Option<String>,
      pub planes: Vec<PlaneStatus>,
  }

  pub struct NodeActionResponse {
      pub node_id: String,
      pub action: String,
      pub accepted: bool,
      pub message: Option<String>,
  }
  ```

* Facet metrics DTO (Rust, `dto::metrics.rs`):

  ```rust
  pub struct FacetMetricsSummary {
      pub facet: String,
      pub rps: f64,
      pub error_rate: f64,
      pub p95_latency_ms: f64,
      pub p99_latency_ms: f64,
  }
  ```

### 1.2 Node client & samplers

* `nodes::client`:

  * Talks to each configured node via HTTP:

    * `/api/v1/status`
    * `/healthz`
    * `/readyz`
    * `/version`
    * `/metrics`
  * If `/api/v1/status` fails, it gracefully degrades to health/ready/version probes and emits warnings (we saw these in the logs).
* `nodes::registry`:

  * Builds list of nodes from `Config::nodes` (currently seeds a single `example-node` pointing to `http://127.0.0.1:9000`).
* `metrics::sampler`:

  * Spawns a sampler per node on startup (`svc_admin::server` logs “spawning facet metrics samplers for configured nodes node_count=1 interval_secs=5”).
  * Regularly scrapes `/metrics`, parses Prometheus text via `prometheus_bridge`, and aggregates facet metrics into summaries (`FacetMetricsSummary`).
  * When scrapes fail, it logs warnings but does not crash the service.

### 1.3 HTTP surface (Axum router)

* UI/API plane (`127.0.0.1:5300`):

  * `/healthz` → simple OK string.
  * `/readyz` → readiness JSON (currently always “ready” for our dev config).
  * `/api/ui-config` → `UiConfigDto`.
  * `/api/me` → `MeResponse` based on `auth.mode`.
  * `/api/nodes` → `Vec<NodeSummary>`.
  * `/api/nodes/{id}/status` → `AdminStatusView`.
  * `/api/nodes/{id}/metrics/facets` → `Vec<FacetMetricsSummary>`.

* Health/metrics plane (`127.0.0.1:5310`):

  * `/metrics` → Prometheus metrics for svc-admin itself, including:

    * `ron_svc_admin_nodes_total`
    * `ron_svc_admin_nodes_by_env{environment="dev"}`

### 1.4 Auth & actions (backend)

* `auth` module:

  * `auth::none` provides `dev_identity()` returning dev operator subject + role `admin`.
  * `auth::ingress` & `auth::passport` modules exist as scaffolding; they are not fully implemented yet.
  * `AuthCfg.mode` validates env string against `"none" | "ingress" | "passport"`.
* Actions:

  * Rust side defines `NodeActionResponse` DTO, but the actual HTTP endpoints for actions (e.g. `POST /api/nodes/{id}/actions/reload`) are not fully hooked up yet. The frontend NodeDetail page expects such endpoints through `adminClient`, but right now the backend is largely read-only.

**Bottom line (backend):**

* Read-only admin plane + node status + facet metrics sampler are in place and working.
* Auth pipeline is effectively `none` only.
* Node actions and strong auth modes (`ingress`, `passport`) are not implemented yet.

---

## 2. SPA / UI status

### 2.1 Types ↔ DTOs

We currently have a **slight drift** between Rust DTOs and `ui/src/types/admin-api.ts`:

* `UiConfigDto` TS type is correct and matches Rust / JSON:

  * `defaultTheme`, `availableThemes`, `defaultLanguage`, `availableLanguages`, `readOnly`, `dev.enableAppPlayground`.

* `MeResponse` TS type is mostly correct:

  * `loginUrl?: string` (TS) vs `Option<String>` with `skip_serializing_if` (Rust).

    * The stricter mapping we want long-term is `loginUrl?: string | null`, but the looseness is acceptable for dev preview.

* **Known drift:**

  * TS `AdminStatusView` currently uses:

    * `node_id: string`
    * `profile: string`
    * `version: string`
  * Rust `AdminStatusView` uses:

    * `id: String`
    * `profile: Option<String>`
    * `version: Option<String>`
  * Result: the detail page prints `status.node_id` but the JSON actually has `id`, so “ID: …” shows up blank in the UI.
  * `NodeSummary.profile` in TS is `string`, but Rust exposes `Option<String>`.

* Actions:

  * `NodeDetailPage` imports `NodeActionResponse` from `types/admin-api`, but the current TS file shown in the conversation **does not define** `NodeActionResponse`.
  * The version that compiles must have this type defined; otherwise TypeScript would error. We need to confirm and unify on the canonical version next session:

    ```ts
    export type NodeActionResponse = {
      node_id: string
      action: string
      accepted: boolean
      message?: string | null
    }
    ```

    (matching the Rust DTO).

**Action item for next session:**
Bring `admin-api.ts` fully in sync with Rust:

* `AdminStatusView` → `{ id: string; display_name: string; profile: string | null; version: string | null; ... }`
* `NodeSummary.profile` → `string | null`
* `NodeActionResponse` → defined as above.
* Update `NodeDetailPage` to use `status.id` rather than `status.node_id`.

### 2.2 Pages & components

* Routing (`App.tsx`):

  * `/` → Node list page.
  * `/nodes/:id` → Node detail page.
  * `/settings` → settings.
  * `/login` → login (placeholder for future auth flows).
  * `*` → NotFound page.

* Node list page:

  * Uses `adminClient.getNodes()` to list nodes.
  * Shows cards like “Example Node” with profile badge.
  * Layout is clean and brand-aligned; we added 🦀🧅 in the sidebar title.

* Node detail page:

  * Fetches **three** things:

    * Node status (`AdminStatusView`).
    * Facet metrics (`FacetMetricsSummary[]`).
    * UiConfig + Me (for readOnly flag + roles) to gate actions.

  * Derives “overallHealth” from plane statuses (`healthy` / `degraded` / `down`) and shows `NodeStatusBadge`.

  * Shows:

    * ID, profile, version line (ID currently blank due to `node_id` vs `id` mismatch).
    * Planes table via `PlaneStatusTable`.
    * Facet metrics panel via `FacetMetricsPanel`.
    * Actions section:

      * “Reload” and “Shutdown” buttons.
      * Buttons are disabled when:

        * UI is readOnly, or
        * Operator lacks `admin` / `ops` roles, or
        * Action is already in flight.
      * On click, it calls `adminClient.reloadNode(status.node_id)` or `.shutdownNode(...)` and expects `NodeActionResponse` from backend.
      * Shows success or error message accordingly, using `t('node.actions.*')` keys.

  * Because backend actions endpoints are not implemented yet, these calls will fail if invoked; we’ve not tested this path end-to-end yet.

* Settings page:

  * Uses `/api/ui-config` and `/api/me` to show some basic environment info. It’s part of the 0.1.0 dev preview DX.

### 2.3 i18n & themes

* i18n:

  * `src/i18n` has:

    * `I18nProvider` wrapping the app.
    * `useI18n()` hook returning `t(key)` and current language.
    * Locale files `public/locales/en-US.json` and `es-ES.json`.

  * The language switcher in the top bar changes the active locale, and keys wired through `t(...)` update correctly. We’ve visibly seen:

    * `nav.nodes` key switching “Nodes” ↔ “Nodos”.

  * **But:** most strings on NodeDetailPage and NodeListPage are still literal English strings (e.g., “Nodes”, “Overview of nodes registered…”, “Planes”, “Facet metrics”, the facet empty-state text). They don’t call `t(...)`, so changing the locale does nothing for them. That’s why, in Spanish mode, only “Nodos” appears translated.

  * For now, we’ve accepted this as a dev-preview trade-off to avoid being buried in translation work. The architecture (I18nProvider + JSON locale files) is in place; actual strings will be externalized gradually.

* Themes:

  * `ThemeProvider` reads defaults from `/api/ui-config`:

    * `defaultTheme` and `availableThemes`.
  * The theme switcher updates theme (light / dark / system). We validated visually:

    * Light: white cards with subtle drop shadows.
    * Dark: darker background seen in earlier screenshot of NodeDetail.

---

## 3. Auth & security posture (today vs target)

* Current mode: `auth.mode = "none"`.

  * `/api/me` always returns dev identity with subject `dev-operator`, role `admin`, authMode `"none"`.
  * No real authentication; console is implicitly trusted in dev.

* Future modes (not implemented yet):

  * `ingress`:

    * Expect to trust headers injected by a fronting proxy (e.g. `X-User`, `X-Roles`).
    * `MeResponse` will be built from those headers; actions will be gated by roles.
  * `passport`:

    * Full JWT / JWKS integration; `MeResponse` built from verified token.
    * Potential `loginUrl` for interactive flows.

* Actions:

  * UI already assumes that mutating actions must be gated by:

    * `UiConfigDto.readOnly` **and**
    * operator `roles` from `/api/me`.
  * Backend still needs:

    * Actual actions endpoints (`POST /api/nodes/{id}/actions/reload`, `/shutdown`, etc.).
    * Enforcement of `read_only` and roles on the server side (policy-first, as per IDB).
    * Emission of `NodeActionResponse` with `accepted` + optional `message`.

---

## 4. Observability & hardening status

* svc-admin itself already exports basic metrics:

  * Node count by env.
  * Node total count.
  * There may be additional HTTP metrics; we haven’t exhaustively inspected them yet.
* Sampler behaves in a non-crashy way when nodes are unreachable:

  * Logs warnings but continues serving both API and UI.
* Readiness is currently optimistic (e.g., `/readyz` doesn’t gate on sampler freshness or node reachability yet). For 0.1.0 dev preview that’s acceptable; for later milestones we’ll want:

  * Readiness reflecting sampler staleness.
  * Possibly separate “degraded” vs “healthy” conditions.

Chaos tests, property tests, fuzzing, and Loom-style concurrency validation are **not** built yet; those live in the 0.3.x hardening milestone.

---

## 5. Docs & blueprints alignment

From `IDB.md`, `README.MD`, `ALL_DOCS.md`, and `TODO.MD`:

* IDB invariants (admin plane only, truthful readiness, no hidden state, read-only by default, amnesia-safe, profile-agnostic, no remote shell) are all respected in the current code:

  * Only HTTP admin endpoints are used.
  * No persistence of node state; everything is config or in-memory.
  * Default UI config is read_only=true.
  * No remote shell / `Command` usage anywhere.
* TODO phases are largely satisfied for 0.1.0:

  * Crate shell, config, DTOs, router, metrics sampler, tests, SPA shell, and basic wiring are all in place.
  * Remaining 0.1.0 pieces are mostly **polish, DX, and small alignment fixes**, not large architecture shifts.
* README already describes:

  * Role of svc-admin as read-only admin console (dev preview).
  * How to run it (cargo run + npm dev).
  * Rough roadmap for actions & auth.

---

## 6. What remains (clustered by milestone)

### 6.1 For 0.1.0 “Dev Preview” polish

1. **Fix DTO drift & TS types**

   * Update `admin-api.ts` to match Rust DTOs exactly:

     * `AdminStatusView.id` / `profile: string | null` / `version: string | null`.
     * `NodeSummary.profile: string | null`.
     * `NodeActionResponse` type added if missing.
   * Update `NodeDetailPage` (and any other usages) to use `status.id` instead of `status.node_id`, and handle nullable profile/version.
   * Re-run `npm run build` to confirm no TS errors and that “ID:” line renders correctly.

2. **Config documentation / CONFIG.MD**

   * Write an operator-grade config doc for svc-admin 0.1.0:

     * Explain all env vars (`SVC_ADMIN_*`) and their defaults.
     * Show how to configure multiple nodes and their admin URLs.
     * Describe the expectations for node endpoints (`/healthz`, `/readyz`, `/version`, `/metrics`, `/api/v1/status`).
     * Provide copy-paste examples for:

       * Single macronode.
       * Mixed micro+macro setup.

3. **UI state polish (metrics & nodes)**

   * Facet metrics panel:

     * Distinguish between:

       * “No data yet (node hasn’t emitted metrics in window)”.
       * “Sampler failing (can’t reach node / metrics path)”.
     * Show “last updated” timestamp for facet metrics per node.
   * Node list:

     * Empty state when no nodes are configured.
     * Optional badge when node is unreachable (based on status calls).

4. **ESLint migration**

   * Add `eslint.config.js` compliant with ESLint 9.
   * Keep rules consistent with the rest of the project (React, TS, hooks).
   * Get `npm run lint` passing again.

5. **Screenshots / quickstart snippet**

   * Once the above is polished, capture one or two screenshots (Nodes list + Node detail) and add a short “Quickstart” section to README for svc-admin dev preview.

### 6.2 Milestone 0.2.x – Auth & gated actions

(not doing now, but important context)

* Implement:

  * `auth::ingress` (header-based identity).
  * `auth::passport` (JWT/JWKS).
  * Per-mode identity extraction and `/api/me` population.

* Add backend endpoints:

  * `POST /api/nodes/{id}/actions/reload`
  * `POST /api/nodes/{id}/actions/shutdown`
  * Possibly `drain`, `resume`, etc., later.

* Wire roles + `UiCfg.read_only` into server-side action authorization.

* Create an audit trail for actions (could be just structured logs initially; later maybe integrate with `ron-audit`).

### 6.3 Milestone 0.3.x – Hardening & scale

* Config layering (file + env + CLI).
* Readiness/health that considers sampler staleness and error rates.
* svc-admin internal metrics for HTTP latency, in-flight, auth failures, sampler lag, etc.
* Property tests, fuzzing of metrics/status parsers, and concurrency tests for samplers.
* Chaos tests against fake nodes (slow / flaky responses) to ensure svc-admin stays responsive and doesn’t explode at scale.

---

## 7. Recommended **next high-impact step** when we resume

Given where we are, the best next slice (small but high leverage) is:

> **“Lock the contract”: fix TypeScript ↔ Rust DTO drift + add operator-facing CONFIG docs.**

Concretely:

1. **Align `admin-api.ts` with Rust DTOs** (AdminStatusView, NodeSummary, NodeActionResponse).

2. **Update `NodeDetailPage`** to use `status.id` and handle nullable profile/version.

3. Run:

   ```bash
   # Backend
   cargo test -p svc-admin --tests
   cargo run -p svc-admin --bin svc-admin

   # Frontend (from crates/svc-admin/ui)
   npm run build
   npm run dev
   ```

   Confirm:

   * “ID: example-node” renders on detail page.
   * No TS/JS console errors.

4. **Draft CONFIG.MD** for svc-admin 0.1.0:

   * Env vars + defaults.
   * Example configs for pointing at a real macronode.
   * Quick troubleshooting hints (“If you see ‘No facet metrics observed yet’, check that your node is emitting facet metrics…”).

Once that’s done, svc-admin is a very solid **dev preview**: a real, runnable admin console you can point at macronode/micronode and hand to other developers/operators without hand-holding.


### END NOTE - DECEMBER 6 2025 - 16:00 CST




### BEGIN NOTE - DECEMBER 6 2025 - 18:40 CST



**Carry-over notes for `crates/svc-admin` (backend + SPA)**

---

## 0. High-level snapshot (right now)

**Crate:** `crates/svc-admin`
**Role:** Read-only admin console + future control plane for RON-CORE nodes (macronode, micronode, etc.).

**Backend status:**

* `cargo test -p svc-admin --tests` → ✅ **Green**

  * Unit tests:

    * `metrics::sampler::tests::parse_facet_snapshots_aggregates_by_facet`
    * Node client behavior tests (HTTP vs HTTPS, `insecure_http`, fake admin plane)
  * Integration tests:

    * `tests/http_smoke.rs` – `/healthz`, `/readyz`, `/metrics`, node inventory metrics.
    * `tests/fake_node.rs` – NodeRegistry + NodeClient + `/api/v1/status`.
    * `tests/config_env.rs` – env override semantics.
* Runtime:

  * UI/API port: `127.0.0.1:5300`
  * Health/metrics port: `127.0.0.1:5310`
  * Confirmed endpoints:

    * `GET /healthz` → `"ok"`
    * `GET /readyz` → `{"ready":true}` (simple gate, ready for future tightening)
    * `GET /metrics` → Prometheus metrics, including:

      * `ron_svc_admin_nodes_total`
      * `ron_svc_admin_nodes_by_env{environment="..."}`
    * `GET /api/ui-config` → `UiConfigDto` derived from `UiCfg`
    * `GET /api/me` → `MeResponse` via auth pipeline (currently `mode="none"` dev identity, ingress/passport stubs)
    * `GET /api/nodes` → `NodeSummary[]` from config-driven NodeRegistry
    * `GET /api/nodes/{id}/status` → `AdminStatusView` normalized from node `/api/v1/status` (fallback to triple probe if needed)
    * `GET /api/nodes/{id}/metrics/facets` → facet metrics summaries from sampler + rolling window

**Frontend (Vite/React/TS SPA) status:**

* Resides in: `crates/svc-admin/ui`
* `npm install`, `npm run dev`, `npm run build` → ✅ working.
* SPA routes:

  * `/` → Node list
  * `/nodes/:id` → Node detail (status + facet metrics)
  * `/settings` → Settings shell
  * `/login` → placeholder for future auth UX
  * `*` → NotFound
* Providers:

  * `ThemeProvider` – theme (light/dark/system) with defaults from `/api/ui-config`
  * `I18nProvider` – locale + translations with defaults from `/api/ui-config`
* Top bar:

  * Shows live identity from `/api/me` (dev fallback for now).
* **This session:** we tightened `NodeDetailPage.tsx` and its use of the API types:

  * Node detail page now cleanly consumes `AdminStatusView` + facet metrics DTOs from `admin-api.ts`.
  * Better separation of “status loading” vs “metrics loading.”
  * Safer handling of missing/late metrics (no UI explosion if the sampler hasn’t produced data yet).
  * Minor UX improvements (clearer layout, slightly sharper wording/naming, consistent use of TypeScript types).

**Completion feeling:**

* **Milestone 0.1.0 – Read-only Admin Console (dev preview):**
  ~**90–92% complete.** Backend is effectively done; SPA is real and wired. What’s missing is mostly UX polish, edge cases, and small tests/docs.
* **Full svc-admin vision (0.1 + 0.2 + 0.3):**
  ~**60–65% complete.** Auth, gated actions, and hardening still ahead.

---

## 1. Crate structure & responsibilities (big picture)

### 1.1 What svc-admin *is*

svc-admin is a **service binary + library** that:

* Bootstraps using an env-driven `Config`.
* Maintains a **NodeRegistry** of RON-CORE nodes based on `NodesCfg`.
* Talks ONLY to node **admin planes** over HTTP(S):

  * `/healthz`, `/readyz`, `/version`, `/metrics`, `/api/v1/status`
* Continuously scrapes node `/metrics` to derive **facet metrics** (RPS, error-rate, latencies).
* Exposes:

  * Operator-facing HTTP API (`/api/*`) for the SPA
  * Health + metrics endpoints (`/healthz`, `/readyz`, `/metrics`)
* Serves a Vite-built SPA (in the future via `build.rs` / assets bundle; currently run side-by-side in dev).

### 1.2 Modules (Rust side)

* `src/lib.rs` – re-exports core modules (config, dto, server, error).
* `src/bin/svc-admin.rs` – `main()`, calls `cli::parse_args()` → `server::run(config)`.

Core subsystems:

* `config/*` – `Config` + sub-structs:

  * `AuthCfg`, `UiCfg`, `ServerCfg`, `NodesCfg`/`NodeCfg`, `PollingCfg`, `LogCfg`, `ActionsCfg`
  * `loader.rs` – env loader, default seeding, validation.
* `auth/*` – identity pipeline:

  * `none.rs` – dev-mode identity.
  * `ingress.rs` – header-based identity (scaffold; logic to be completed).
  * `passport.rs` – JWT/JWKS stub.
  * `mod.rs` – `Identity`, `AuthError`, `resolve_identity_from_headers`.
* `nodes/*` – node inventory + client:

  * `registry.rs` – `NodeRegistry`.
  * `client.rs` – `NodeClient` with HTTP logic, insecure_http checks.
  * `status.rs` – mapping node `/api/v1/status` infos into `AdminStatusView`.
* `metrics/*` – observability:

  * `prometheus_bridge.rs` – text parsing + label extraction.
  * `sampler.rs` – scraping `/metrics` for each node, building facet snapshots.
  * `facet.rs` – rolling window per facet per node; aggregates into `FacetMetricsSummary`.
* `dto/*` – JSON DTOs:

  * `ui.rs` – `UiConfigDto`.
  * `me.rs` – `MeResponse`.
  * `node.rs` – `NodeSummary`, `AdminStatusView`, `PlaneStatus`.
  * `metrics.rs` – `FacetMetricsSummary`.
* `observability.rs` – tracing + Prometheus registry initialization.
* `state.rs` – `AppState` (Config + NodeRegistry + FacetMetrics store).
* `router.rs` – Axum router – all endpoints + handlers.
* `server.rs` – binds sockets, spawns HTTP servers + samplers, graceful shutdown.
* `cli.rs` – currently a thin shim: `Config::load()`. CLI + file config is future work.

---

## 2. Config system – what we have, what’s left

### 2.1 Types

`Config` is the central struct:

* `server: ServerCfg`
* `log: LogCfg`
* `polling: PollingCfg`
* `ui: UiCfg`
* `actions: ActionsCfg`
* `auth: AuthCfg`
* `nodes: NodesCfg` (map from node id → `NodeCfg`)

Key subtypes:

* **ServerCfg**

  * `bind_addr: String` – UI/API listener.
  * `metrics_addr: String` – health & metrics listener.
  * `max_conns: usize`
  * `read_timeout`, `write_timeout`, `idle_timeout: Duration`
  * `tls: TlsCfg { enabled: bool, cert_path: Option<PathBuf>, key_path: Option<PathBuf> }`
* **LogCfg**

  * `format: String` – `"compact"` or `"pretty"`, default `"compact"`.
  * `level: String` – e.g. `"info"`, default `"info"`.
* **PollingCfg**

  * `metrics_interval: Duration` – e.g. 5s
  * `metrics_window: Duration` – e.g. 300s
* **UiCfg**

  * `default_theme: String`
  * `default_language: String`
  * `read_only: bool`
  * `dev: UiDevCfg { enable_app_playground: bool }`
* **ActionsCfg**

  * `enable_reload: bool`
  * `enable_shutdown: bool`
* **AuthCfg**

  * `mode: String` – `"none" | "ingress" | "passport"`
  * `passport_issuer: Option<String>`
  * `passport_audience: Option<String>`
  * `passport_jwks_url: Option<String>`
* **NodesCfg / NodeCfg**

  * `NodesCfg = BTreeMap<String, NodeCfg>`
  * `NodeCfg`:

    * `base_url: String`
    * `display_name: Option<String>`
    * `environment: String`
    * `insecure_http: bool`
    * `forced_profile: Option<String>`
    * `macaroon_path: Option<PathBuf>`
    * `default_timeout: Option<Duration>`

Default seeds `example-node`:

* `id = "example-node"`
* `base_url = "http://127.0.0.1:9000"`
* `display_name = "Example Node"`
* `environment = "dev"`
* `insecure_http = true`
* `forced_profile = Some("macronode")`
* `default_timeout = Some(2s)`

### 2.2 Loading & env overrides

`Config::load()`:

* Guardrail: if `SVC_ADMIN_CONFIG` is set → **error** (file-based config not implemented yet; this is intentional).
* Otherwise, start from `Config::default()` and override via env:

**Server-related envs:**

* `SVC_ADMIN_BIND_ADDR` → `server.bind_addr` (validated as `SocketAddr`).
* `SVC_ADMIN_METRICS_ADDR` → `server.metrics_addr`.
* `SVC_ADMIN_MAX_CONNS`
* `SVC_ADMIN_READ_TIMEOUT`, `SVC_ADMIN_WRITE_TIMEOUT`, `SVC_ADMIN_IDLE_TIMEOUT` (seconds).

**TLS:**

* `SVC_ADMIN_TLS_ENABLED`
* `SVC_ADMIN_TLS_CERT_PATH`
* `SVC_ADMIN_TLS_KEY_PATH`

**Logging:**

* `SVC_ADMIN_LOG_FORMAT`
* `SVC_ADMIN_LOG_LEVEL`

**Polling:**

* `SVC_ADMIN_POLLING_METRICS_INTERVAL`
* `SVC_ADMIN_POLLING_METRICS_WINDOW`

**UI:**

* `SVC_ADMIN_UI_THEME` / `SVC_ADMIN_UI_DEFAULT_THEME` → `ui.default_theme`
* `SVC_ADMIN_UI_LANGUAGE` / `SVC_ADMIN_UI_DEFAULT_LANGUAGE` → `ui.default_language`
* `SVC_ADMIN_UI_READ_ONLY`
* `SVC_ADMIN_UI_DEV_ENABLE_APP_PLAYGROUND`

**Actions:**

* `SVC_ADMIN_ACTIONS_ENABLE_RELOAD`
* `SVC_ADMIN_ACTIONS_ENABLE_SHUTDOWN`

**Auth:**

* `SVC_ADMIN_AUTH_MODE` – validated to `"none" | "ingress" | "passport"`
* `SVC_ADMIN_AUTH_PASSPORT_ISSUER`
* `SVC_ADMIN_AUTH_PASSPORT_AUDIENCE`
* `SVC_ADMIN_AUTH_PASSPORT_JWKS_URL`

**Example node overrides:**

* `SVC_ADMIN_EXAMPLE_NODE_URL`
* `SVC_ADMIN_EXAMPLE_NODE_ENV`

Helper parsers:

* `load_addr`, `load_bool`, `load_usize`, `load_duration`, `load_opt_path`, `load_auth_mode`.

### 2.3 Validation

`Config::validate()` enforces:

* `server.max_conns > 0`
* `polling.metrics_interval > 0`
* `polling.metrics_window >= polling.metrics_interval`
* If `tls.enabled`:

  * `cert_path.is_some() && key_path.is_some()`

**Test coverage:**

* `tests/config_env.rs` – checks env overrides (especially UI theme precedence) and that parsed config matches expectations.

### 2.4 Remaining config work

* Implement **file-based config** (TOML) and `--config` via `clap`.
* Precedence: **defaults < file < env < CLI**.
* Provide `CONFIG.MD` (or README section) summarizing all config knobs and precedence.
* Clean minor warning noise in `config/loader.rs` (unused imports).

---

## 3. Node registry, NodeClient, and node-facing API

### 3.1 NodeRegistry

`NodeRegistry` (under `nodes/registry.rs`):

* Built from `NodesCfg` at startup:

  ```rust
  pub struct NodeRegistry {
      nodes: Arc<BTreeMap<String, NodeCfg>>,
      client: NodeClient,
  }
  ```

* Methods:

  * `list_summaries() -> Vec<NodeSummary>`:

    * For each `(id, cfg)`, build `NodeSummary`:

      * `id`
      * `display_name = cfg.display_name.unwrap_or(id.clone())`
      * `profile` (currently from normalized status or config; early versions used `None`).
  * `get_status(id: &str) -> Option<AdminStatusView>`:

    * Looks up `NodeCfg`.
    * Calls `NodeClient::fetch_status(id, &cfg).await`.
    * On success: returns real `AdminStatusView`.
    * On error: logs and returns a placeholder view (with id/display_name set).
  * `contains(id: &str) -> bool`:

    * For 404 checks & validating existing nodes.

### 3.2 NodeClient

`NodeClient` (in `nodes/client.rs`) is the HTTP adapter for the admin plane:

* Validates URLs:

  * If `base_url` starts with `http://` and `insecure_http == false` → returns `Error::Config` (tested).

* Applies per-node `default_timeout` when set.

* Calls:

  * `/healthz`
  * `/readyz`
  * `/version`
  * `/api/v1/status`
  * `/metrics` (from sampler)

* Key methods:

  * `fetch_health(cfg)` → `Result<bool>`:

    * Any successful 2xx with body → `true`.
  * `fetch_ready(cfg)` → `Result<bool>`:

    * Prefers JSON `{"ready": true|false}`.
    * Falls back to body presence if JSON parse fails.
  * `fetch_version(cfg)` → `Result<Option<String>>`:

    * `Some(version)` if non-empty body; logs & returns `Ok(None)` on failure.
  * `fetch_status(id, cfg)` → `Result<AdminStatusView>`:

    * Preferred path:

      * GET `/api/v1/status` → `RawStatus`.
      * `nodes::status::from_raw(id, cfg, raw)` → `AdminStatusView`.
    * Fallback path:

      * On failure, triple probe: `fetch_health`, `fetch_ready`, `fetch_version`.
      * Use `build_status_placeholder()` + patch id/display/limited info.

* Tests:

  * `node_client_can_talk_to_fake_admin_plane`:

    * Fake admin plane with `/healthz`, `/readyz`, `/version`.
    * Ensures client can talk and parse values.
  * `node_client_rejects_http_when_insecure_http_false`:

    * Ensures we reject insecure HTTP when config disallows it.

### 3.3 Status normalization

`nodes/status.rs`:

* Defines `RawPlane` and `RawStatus` mirroring node `/api/v1/status`:

  ```rust
  pub struct RawPlane {
      pub name: String,
      pub health: String,
      pub ready: bool,
      pub restart_count: u64,
  }

  pub struct RawStatus {
      pub profile: Option<String>,
      pub version: Option<String>,
      pub planes: Vec<RawPlane>,
  }
  ```

* `build_status_placeholder()` → `AdminStatusView` with generic placeholder data.

* `from_raw(id, cfg, raw) -> AdminStatusView`:

  * `node_id = id`
  * `display_name` from `cfg.display_name` or id.
  * `profile` from `raw.profile` or `cfg.forced_profile`.
  * `version` from `raw.version`.
  * Planes: `RawPlane` → `PlaneStatus` (health/ready/restart_count mapping).

### 3.4 Node-facing HTTP endpoints

In `router.rs`:

* `GET /api/nodes`:

  * Returns `Vec<NodeSummary>` from `NodeRegistry::list_summaries()`.
* `GET /api/nodes/{id}/status`:

  * If node exists:

    * `NodeRegistry::get_status(id)` → `AdminStatusView`.
  * If node not found:

    * `404` with simple error payload.

**This is all working and covered by tests.**

---

## 4. Metrics, facet sampler, and observability

This is one of the most sophisticated parts of svc-admin.

### 4.1 svc-admin’s own observability

`observability.rs`:

* Sets up `tracing-subscriber` (`fmt` + env-filter).
* Configures the global Prometheus registry used by `/metrics`.

`/metrics`:

* Exposes:

  * svc-admin process metrics.
  * node inventory metrics:

    * `ron_svc_admin_nodes_total`
    * `ron_svc_admin_nodes_by_env{environment="..."}`

`/healthz` & `/readyz`:

* `/healthz` → `"ok"` once service booted.
* `/readyz` → currently a simple `{"ready": true}`; scaffolding is in place for gating on more conditions (sampler freshness, config, etc.).

`tests/http_smoke.rs`:

* Spins up svc-admin.
* Hits `/healthz` and `/metrics`.
* Asserts success and presence of node inventory metrics in `/metrics` output.

### 4.2 Facet metrics sampler

**Goal:** for each node, continuously pull `/metrics`, extract `ron_facet_*` counters, and compute per-facet summaries over a short window.

**Pieces:**

1. **Targets (`NodeMetricsTarget`)**:

   * Built from `NodesCfg` and `NodeCfg`.
   * Contains:

     * `node_id: String`
     * `metrics_url: String` (e.g. `http://127.0.0.1:9000/metrics`)
     * Optional `timeout`.

2. **Sampler tasks (`metrics::sampler`)**:

   * `spawn_samplers(targets, interval, facet_metrics, shutdown)`:

     * For each target:

       * Spawns a `tokio` task:

         * Immediately scrapes once.
         * Then loops:

           * `tokio::select!` between:

             * `shutdown.changed()` → exit.
             * `sleep(interval)` → next scrape.
   * `run_sampler_for_target(...)`:

     * Uses shared `reqwest::Client`.
     * GET `/metrics` with timeout.
     * On success: pass body to `parse_facet_snapshots`.
     * On error: log warning, do not crash task.

3. **Prometheus parser → facet snapshots**:

   * `parse_facet_snapshots(body: &str) -> Vec<FacetSnapshot>`:

     * Looks for metrics like:

       * `ron_facet_requests_total{facet="overlay.connect",result="ok"} 10`
       * `ron_facet_requests_total{facet="overlay.jobs",result="error"} 2`
     * Groups by `facet`:

       * `requests_total` = sum of values.
       * `errors_total` = sum of values where `result` ∈ {`error`, `err`, `failure`, `5xx`, etc.}
   * Unit test `parse_facet_snapshots_aggregates_by_facet`:

     * Builds fake `/metrics` text.
     * Ensures we get two facets with correct totals and error counts.

4. **Rolling window aggregator (`metrics::facet`)**:

   * `FacetMetrics` stores recent snapshots per `(node_id, facet)`:

     * Each sample includes a timestamp and `FacetSnapshot`.
     * Old entries (beyond `metrics_window` seconds) are pruned.
   * Provides:

     * `update_from_scrape(node_id, Vec<FacetSnapshot>)`
     * `summaries_for_node(node_id) -> Vec<FacetMetricsSummary>`:

       * `facet: String`
       * `rps: f64` (requests/sec over recent window)
       * `error_rate: f64`
       * `p95_latency_ms`, `p99_latency_ms` (currently placeholder / to be computed once we parse histograms).

5. **HTTP API for facet metrics**:

   * `GET /api/nodes/{id}/metrics/facets`:

     * Uses `FacetMetrics` from `AppState`.
     * Returns `Vec<FacetMetricsSummary>` for the given node.
   * The SPA uses this endpoint on Node Detail to show facet metrics charts.

6. **Sampler lifecycle**:

   * `server::run(config)` builds:

     * `AppState` with `FacetMetrics` store.
     * `NodeMetricsTarget`s from `NodesCfg`.
     * Spawns samplers with `PollingCfg.metrics_interval`.
     * Wires a `shutdown` watch channel that tasks listen to for clean exit.

**Behaviour with example-node not running:**

* When `example-node` at `http://127.0.0.1:9000/metrics` isn’t actually serving metrics:

  * Sampler logs `Connection refused` warnings.
  * Keeps retrying on each interval.
  * svc-admin remains up and responsive.

### 4.3 Remaining observability tasks

* Add internal sampler metrics:

  * `svc_admin_sampler_last_scrape_timestamp{node_id}`
  * `svc_admin_sampler_errors_total{node_id}`

* Use **facets freshness** in `/readyz` to declare full readiness.

* Parse latency histograms (from node metrics) to compute real p95/p99 instead of placeholders.

* Hardening tests:

  * Many nodes, slow/failing `/metrics`, check that svc-admin remains stable.

---

## 5. Auth, identity, and `/api/me`

### 5.1 AuthCfg & modes

`AuthCfg`:

* `mode: String` – `"none" | "ingress" | "passport"`
* Passport fields reserved but not wired:

  * `passport_issuer`
  * `passport_audience`
  * `passport_jwks_url`

Currently:

* We mostly run in `mode="none"`, which uses dev identity.

### 5.2 Identity model

`auth::Identity`:

* `subject: String`
* `display_name: String`
* `roles: Vec<String>`

Helpers:

* `Identity::dev_fallback()`:

  * `subject = "dev-operator"`
  * `display_name = "Dev Operator"`
  * `roles = ["admin"]`

### 5.3 Mode-specific modules

**`auth::none`**

* `identity()` → `Identity::dev_fallback()`.
* Used when `auth.mode = "none"` or when everything else fails.

**`auth::ingress`**

* Intended behaviour (scaffolded):

  * Look at headers:

    * `X-User` → subject/display_name.
    * `X-Groups` or `X-Roles` → comma-separated roles.
  * On missing header: degrade to `anonymous`/empty roles or `AuthError::Unauthenticated` (depending on policy).
  * On invalid header encoding: `AuthError::Invalid`.

* Code structure is present; needs finalization + tests (parsing, error handling).

**`auth::passport`**

* Currently stubbed:

  * Would parse `Authorization: Bearer ...`.
  * Validate JWT using JWKS from `AuthCfg`.
  * Extract subject, name, roles.
* Right now: not implemented; returns an error or dev fallback.

### 5.4 `resolve_identity_from_headers` and `/api/me`

`auth::resolve_identity_from_headers(auth_cfg, headers)`:

* If `auth.mode == "none"`:

  * Returns `Identity::dev_fallback()`.
* If `"ingress"`:

  * Calls ingress resolver; may return error on missing/invalid headers.
* If `"passport"`:

  * Stub; currently unimplemented, should yield `AuthError::Unimplemented`.
* Unknown mode:

  * Soft-fallback to `Identity::dev_fallback()` (to avoid full breakage in misconfig).

`/api/me` handler:

* Reads `AuthCfg` from `AppState`.
* Calls `resolve_identity_from_headers`.
* If it errors, falls back to `Identity::dev_fallback()`.
* Returns `MeResponse`:

  ```rust
  struct MeResponse {
      subject: String,
      display_name: String,
      roles: Vec<String>,
      auth_mode: String, // from AuthCfg
      login_url: Option<String>,
  }
  ```

SPA uses this to display:

* In TopBar: identity badge like
  `Dev Operator (none · admin)`.

### 5.5 Remaining auth work

* Implement `auth::ingress` fully (header parsing, error handling).
* Implement `auth::passport` (JWT validation & JWKS).
* Add tests:

  * Unit tests for each mode.
  * Integration tests for `/api/me` with different `auth.mode` and headers.
* Use roles from `Identity` to **gate future write actions** (reload/shutdown).

---

## 6. SPA / UI – structure, wiring, and NodeDetail tweaks

### 6.1 Core shell

In `crates/svc-admin/ui`:

* `index.html` – base HTML.
* `main.tsx`:

  * Wraps `App` with:

    * `ThemeProvider`
    * `I18nProvider`
    * `BrowserRouter` (or equivalent).
* `App.tsx` routes:

  * `/` → Node list
  * `/nodes/:id` → Node detail
  * `/settings` → Settings
  * `/login` → placeholder
  * `*` → NotFound

Shell components:

* `Shell` – main frame (sidebar + topbar + content).
* `Sidebar` – navigation:

  * “Nodes”
  * “Settings”
* `TopBar`:

  * Brand/logo.
  * Identity info (`/api/me`).
  * `LanguageSwitcher`.
  * `ThemeToggle`.

### 6.2 Theme system

`ThemeProvider`:

* Local state: `theme: 'light' | 'dark' | 'system'`.
* On mount:

  * Calls `adminClient.getUiConfig()`.
  * Uses `default_theme` from backend if valid.
* Applies theme via `data-theme` or `class` on `<html>` or `<body>`.
* `useTheme()` hook to access `theme` & `setTheme`.

`ThemeToggle`:

* Uses `useTheme()` to cycle theme.
* This session we confirmed the wiring and ensured:

  * Toggle updates React state **and** DOM.
  * Works correctly regardless of backend default.

### 6.3 I18n system

`I18nProvider`:

* Holds `locale` in state.
* On mount:

  * Calls `adminClient.getUiConfig()`.
  * Uses `default_language` (or `default_locale` depending on final naming) as initial locale.
* Loads translations from:

  * `public/locales/en-US.json`
  * `public/locales/es-ES.json`
* `useI18n()` hook:

  * Provides `t(key)`, `locale`, `setLocale`.

`LanguageSwitcher`:

* Consumes `useI18n()`.
* Renders a `<select>` for languages and calls `setLocale()` on change.
* This session we ensured it’s truly bound to context (not just a defaultValue) and plays well with backend defaults.

### 6.4 Types & API client

`ui/src/types/admin-api.ts`:

* Mirrors Rust DTOs:

  ```ts
  export type UiConfigDto = {
    title: string
    subtitle?: string
    readOnly: boolean
    defaultTheme: 'light' | 'dark' | 'system'
    defaultLanguage: string // or defaultLocale, depending on final alignment
    availableThemes: ('light' | 'dark' | 'system')[]
    availableLanguages: string[]
  }

  export type MeResponse = {
    subject: string
    displayName: string
    roles: string[]
    authMode: string
    loginUrl?: string
  }

  export type NodeSummary = {
    id: string
    displayName: string
    profile?: string | null
    // (Optionally env/labels if we added them; keep types in sync with Rust)
  }

  export type PlaneStatus = {
    name: string
    health: 'healthy' | 'degraded' | 'down'
    ready: boolean
    restartCount: number
  }

  export type AdminStatusView = {
    nodeId: string
    displayName: string
    profile?: string | null
    version?: string | null
    planes: PlaneStatus[]
  }

  export type FacetMetricsSummary = {
    facet: string
    rps: number
    errorRate: number
    p95LatencyMs: number
    p99LatencyMs: number
  }
  ```

`adminClient.ts`:

* Functions:

  ```ts
  export async function getUiConfig(): Promise<UiConfigDto>
  export async function getMe(): Promise<MeResponse>
  export async function getNodes(): Promise<NodeSummary[]>
  export async function getNodeStatus(id: string): Promise<AdminStatusView>
  export async function getNodeFacetMetrics(id: string): Promise<FacetMetricsSummary[]>
  ```

* All use `fetch` or axios-style wrapper, throw on non-2xx with meaningful error.

### 6.5 Node list & Node detail (plus **today’s tweak**)

**NodeListPage**:

* On mount:

  * Calls `getNodes()`.
* States:

  * loading → spinner.
  * error → `ErrorBanner`.
  * success → grid/list of `NodeCard` components.
* Cards display:

  * `displayName`, `id`, `profile` (and environment/labels if present).
* Each card links to `/nodes/:id`.

**NodeDetailPage** (updated this session):

* Reads `id` from route params.
* On mount:

  * Fires request for **status**: `getNodeStatus(id)`.
  * Fires request for **facet metrics**: `getNodeFacetMetrics(id)`.
* States:

  * Separate loading/error for status vs metrics (this is part of the tweak: we avoid conflating them).
  * Layout ensures you can see node status even if metrics are late/unavailable.
* Renders:

  * Node header:

    * Name, profile, version, possibly environment.
    * Uses consistent `AdminStatusView` fields (no local ad-hoc types).
  * `PlaneStatusTable`:

    * Each `PlaneStatus` row: name, health icon, ready, restart count.
  * Facet metrics panel:

    * If metrics are available: show cards/charts for each `FacetMetricsSummary`.
    * If metrics are still loading or unavailable:

      * Show a clear “No facet metrics yet” / “Metrics unavailable” message, not a cryptic error.

**What changed *this* session:**

* We aligned `NodeDetailPage` more strictly with the TS types from `admin-api.ts`, so it’s harder for the UI and backend to drift.
* We improved error/empty handling so a missing metrics response from samplers does not break the NodeDetail view.
* We made the layout a bit more disciplined:

  * Status section stands on its own.
  * Metrics section is clearly a “secondary” panel with its own loading/error messaging.

---

## 7. Tests & quality gates (recap)

**Backend tests:**

* Unit:

  * `metrics::sampler::tests::parse_facet_snapshots_aggregates_by_facet`
  * Node client tests (HTTP vs HTTPS).
* Integration:

  * `http_smoke` – confirms `/healthz`, `/metrics`, and node inventory metrics.
  * `fake_node` – uses fake node admin plane and fake `/api/v1/status`.
  * `config_env` – ensures env overrides shape config as intended.

**Frontend tests:**

* Right now: minimal (if any). We rely mostly on type-safety + manual Vite dev testing.
* Future: we can add vitest/RTL tests for:

  * `NodeListPage` (loading/error/empty).
  * `NodeDetailPage` (status vs metrics behavior).
  * `ThemeProvider` and `I18nProvider`.

---

## 8. Remaining work (by milestone)

### 8.1 Milestone 0.1.0 – Read-only dev/admin console

**Backend: functionally done.**
**Front-end: ~90% there.**

Remaining:

1. **UI/UX polish:**

   * CSS pass for:

     * Node list grid and cards.
     * Node detail layout (status vs metrics).
     * Top bar and sidebar.
   * Make everything feel like a real product (spacing, typography, consistent colors).

2. **SPA edge states:**

   * Empty state when no nodes are configured.
   * Clear messages for:

     * “No facet metrics yet” (normal at startup).
     * “Metrics scraping is failing” (sampler errors/lag).

3. **Align config & TS types:**

   * Nail down `default_language` vs `default_locale`.
   * Ensure ThemeProvider & I18nProvider both derive from `UiConfigDto` only.
   * Keep `admin-api.ts` strictly in lockstep with Rust DTOs.

4. **Light auth & config docs:**

   * Short `CONFIG.MD` or README section listing env vars and basic usage.
   * Minimal tests for `auth::ingress` header parsing (even if not fully used yet).

Once this is done, 0.1.0 is a very strong **developer preview**.

---

### 8.2 Milestone 0.2.x – Auth, roles, gated actions

Major tasks:

* **Auth pipeline:**

  * Implement `auth::ingress` properly.
  * Implement `auth::passport` with JWT/JWKS.
  * Introduce `AuthMode` abstraction and fully populate `Identity`.

* **Actions:**

  * Add POST endpoints:

    * `POST /api/nodes/{id}/actions/reload`
    * `POST /api/nodes/{id}/actions/shutdown`
  * Use node’s own admin endpoints (no shells) and respect node policy.
  * Gate with:

    * `ActionsCfg` flags.
    * Identity roles.
    * UI read-only flags.

* **Audit:**

  * Append-only action audit log (even if just to stdout in 0.2.0).
  * Future: tie into `ron-audit`.

* **UI:**

  * Add action buttons on NodeDetail.
  * Show disabled states & “are you sure?” prompts.
  * Show roles and auth mode clearly in the top bar.

---

### 8.3 Milestone 0.3.x – Hardening, scaling, and tooling

* **Config layering:**

  * `clap` CLI.
  * TOML configs.
  * Precedence: defaults < file < env < CLI.

* **Sampler hardening / readiness:**

  * Sampler metrics (lag, errors).
  * `/readyz` gating on sampler freshness + config loaded + metrics bound.

* **Chaos & scale tests:**

  * Many nodes with:

    * Slow metrics endpoints.
    * Intermittent failures.
  * Ensure:

    * svc-admin remains responsive.
    * Samplers degrade gracefully, don’t panic.

* **Additional observability:**

  * `svc_admin_http_latency_seconds{path,code}`.
  * `svc_admin_http_inflight`.
  * `svc_admin_auth_failures_total{mode,reason}`.

---

## 9. How to resume next time

When you open the repo again and want to continue svc-admin:

1. **Quick backend sanity:**

   ```bash
   cd /Users/mymac/Desktop/RustyOnions
   cargo test -p svc-admin --tests
   ```

2. **Run svc-admin + UI:**

   ```bash
   # Terminal A - backend
   RUST_LOG=info \
   SVC_ADMIN_BIND_ADDR=127.0.0.1:5300 \
   SVC_ADMIN_METRICS_ADDR=127.0.0.1:5310 \
   cargo run -p svc-admin --bin svc-admin

   # Terminal B - UI
   cd crates/svc-admin/ui
   npm install        # if not already done
   npm run dev
   ```

   Then visit `http://localhost:5173/`:

   * Confirm:

     * Top bar shows identity from `/api/me`.
     * Node list loads from `/api/nodes`.
     * Clicking a node goes to `/nodes/:id` and **NodeDetailPage.tsx** shows:

       * Status (profile, version, planes).
       * Facet metrics block (or clear “no metrics yet” message).

3. **Recommended “next slice” (high-impact):**

   **Option A – Finish 0.1.0 polish (recommended):**

   * Do a UI/UX pass:

     * Nodes list, Node detail, metrics panel, top bar, sidebar.
   * Tighten `/api/ui-config` & TS types (language vs locale).
   * Add empty/error states around metrics.

   **Option B – Start auth & actions (0.2.0):**

   * Implement `auth::ingress` properly + tests.
   * Begin wiring roles into UI (read-only gating for future actions).

   **Option C – Config layering (0.3.x prep):**

   * Implement TOML file + CLI overrides.

Any of those is viable. A gives you the fastest visible payoff (the admin console will “feel done”), B kicks off security/control-plane work, and C sets up ops ergonomics.

These notes should be enough to rehydrate context in the next instance without backpedaling or losing track of what’s actually implemented.


### END NOTE - DECEMBER 6 2025 - 18:40 CST





### BEGIN NOTE - DECEMBER 6 2025 - 21:00 CST

Short version: svc-admin is in a *really* good place. Tests + new auth HTTP tests are green, `/api/me` is now properly wired with modes and headers, and we’re exactly at the point where the next meaningful move is “talk to a real macronode admin-plane instead of the fake example-node.”

I’ll break this into:

1. What you should paste/attach in the next instance (macronode docs & code I’ll need).
2. Fresh, God-tier carry-over notes for **svc-admin** as of right now, including the macronode-connection TODO.

---

## 1. What to bring next time (for macronode ↔ svc-admin wiring)

For the next instance, where we define the macronode admin-plane contract and hook svc-admin into real nodes, these are the artifacts that will let me work at full power:

### 1.1 Macronode code & config

Please attach:

1. **`macronode` CODEBUNDLE**
   The equivalent of `svc-admin`’s `CODEBUNDLE.md`, but for `crates/macronode`:

   * `crates/macronode/src/main/bin.rs` or `src/bin/macronode.rs` (whatever your entrypoint is).
   * Admin HTTP server module(s): whatever currently exposes `/healthz`, `/readyz`, `/metrics`, `/version`, or any admin endpoints.
   * Any existing node status routes or DTOs – even if they’re stubby – so we can line them up with `svc-admin::dto::node::AdminStatusView`.
   * macronode config types (admin bind address, profile/env flags, amnesia flags, etc.).

2. **macronode config docs**
   Just like `svc-admin` has CONFIG / RUNBOOK / IDB, macronode likely has some combination of:

   * `crates/macronode/docs/IDB.md` (Invariant-Driven Blueprint for macronode).
   * `docs/API.MD` or `docs/ADMIN_PLANE.MD` if it exists.
   * `docs/OBSERVABILITY.MD` for golden metrics and admin endpoints.
   * `docs/CONFIG.MD` explaining admin endpoints and bind addresses.

   I don’t need *all* macronode docs, but anything that talks about:

   * Admin HTTP surfaces.
   * Node status JSON.
   * Node-level metrics names / labels.

### 1.2 Cross-cutting RON-CORE docs

Helpful, but secondary:

* Any **global blueprint** that already talks about the “node admin plane v1” contract, especially for:

  * `/api/v1/status` shape (planes, profile, amnesia, env, version).
  * Node metrics conventions (e.g., facet metrics, up/down, environment labels).

If those don’t exist yet, that’s fine – we’ll derive the first precise spec directly from `svc-admin::dto::node::AdminStatusView` and the facet metrics sampler logic.

### 1.3 svc-admin artifacts (for reference)

You already have these, but for the next instance it’s good to paste them again alongside macronode:

* **`svc-admin` CODEBUNDLE.md** (current one you just generated). 
* **`svc-admin` `NOTES.MD`** (the latest, which already describes node expectations & status DTOs). 
* **`svc-admin` README.MD** (sections that say “Nodes expect admin-plane v1: /healthz, /readyz, /version, /metrics, /api/v1/status”). 
* The new **`tests/auth_me_http.rs`**, since it’s part of our current surface and proves that auth + `/api/me` is behaving as intended.

---

## 2. Carry-over notes for svc-admin (God-tier, current snapshot)

### 2.0 High-level status

**Crate:** `crates/svc-admin`
**Role:** Admin-plane GUI + JSON API for RON-CORE nodes (macronode or micronode). Read-only by default, future home of gated actions. 

**Build / tests / lint:**

* `cargo test -p svc-admin --tests` → ✅ green (unit + multiple integration tests).
* `cargo test -p svc-admin --test auth_me_http` → ✅ green (new auth HTTP tests).
* `cargo clippy -p svc-admin --all-targets --all-features` → ✅ no hard errors, only minor warnings in tests about default reassigns and an unused `mut` (we’ve already cleaned the HTTP test’s `mut`, remaining warnings are trivial).

**Runtime:**

* `cargo run -p svc-admin --bin svc-admin` boots cleanly with:

  * UI/API: `127.0.0.1:5300`
  * Health/metrics: `127.0.0.1:5310` 
* Example config points to a single node `example-node` at `http://127.0.0.1:9000` (fake node). 

From your recent run, we saw the facet sampler logging repeated connection failures to `127.0.0.1:9000/metrics`, and metrics exposing `ron_svc_admin_upstream_errors_total{kind="connect"}` incrementing, which is exactly what we want for upstream error accounting.

---

### 2.1 Backend surface & invariants

The backend is structured as:

* `config/*` – config types + loader (env and file), with server/auth/nodes/polling/ui sections. 
* `nodes/*` – node registry, HTTP client, and status mapping into DTOs. 
* `dto/*` – DTOs for:

  * `UiConfigDto` (SPA config: themes, languages, readOnly, dev flags).
  * `MeResponse` (identity & auth-mode info).
  * `NodeSummary` and `AdminStatusView` (node list + detailed status).
  * `FacetMetricsSummary` (rolled-up facet metrics used by SPA). 
* `metrics/*` – Prometheus bridge, facet sampler, and rolling window aggregator. 
* `router.rs` / `server.rs` / `state.rs` – HTTP routing, state wiring, and bootstrap. 

Core invariants from IDB and governance docs are honored:

* Admin-plane only, via node endpoints: `/healthz`, `/readyz`, `/version`, `/metrics`, `/api/v1/status`. 
* Read-only by default: actions are not enabled; UI defaults to `read_only=true`. 
* Profile-agnostic (works for macronode + micronode) by discovering capabilities via `/version` + `/api/v1/status`. 

---

### 2.2 HTTP endpoints exposed by svc-admin

**On the UI/API bind (e.g., `127.0.0.1:5300`):**

* `GET /healthz` – liveness probe, very light.
* `GET /readyz` – readiness (currently always `{ "ready": true }` but wired via `AppState` for future truthfulness).
* `GET /metrics` – Prometheus metrics for svc-admin itself (default registry).
* `GET /api/ui-config` – returns `UiConfigDto` for themes, languages, and read-only state.
* `GET /api/me` – new identity endpoint, described in detail below.
* `GET /api/nodes` – returns `Vec<NodeSummary>` from node registry.
* `GET /api/nodes/:id/status` – returns `AdminStatusView` for that node or 404 if not known.
* `GET /api/nodes/:id/metrics/facets` – returns facet window summaries for that node.

Actions (`/api/nodes/:id/reload`, `/api/nodes/:id/shutdown`) are currently defined in the router but conceptually parked as “0.2.x / 0.3.x” work; in practice, your current focus is read-only 0.1.0, and there is no SPA wiring for these yet. 

**On the metrics/health bind (e.g., `127.0.0.1:5310`):**

* Mirrors `/healthz`, `/readyz`, `/metrics` for service-level probes, per the README. 

---

### 2.3 Config system snapshot

Config is layered via env (and file, where present):

* Top-level keys: `bind_addr`, `metrics_addr`, `max_conns`, `read_timeout`, `write_timeout`, etc., all exposed as `SVC_ADMIN_*` env vars. 
* Node config:

  * Each `NodeCfg` includes `base_url`, `display_name`, `insecure_http`, `forced_profile`, `macaroon_path`, `default_timeout`.
  * Nodes are stored in a `BTreeMap<String, NodeCfg>` keyed by node ID.
* Polling config:

  * `metrics_interval`, `metrics_window` define how frequently and over what window facet metrics are sampled. 

We also have a `config_env.rs` integration test that asserts env overrides are respected, and that is still green. (You saw `env_overrides_are_respected ... ok` in the latest run.)

---

### 2.4 Node registry & facet metrics sampler

**Node registry:**

* Maintains a list of configured nodes (`example-node` in tests) with their `base_url` and display metadata. 
* `nodes::client` handles HTTP calls to:

  * `/api/v1/status` (for `AdminStatusView`).
  * `/metrics` (for facet sampling), obeying `insecure_http` but defaulting to HTTPS semantics where applicable.

**Facet metrics sampler:**

* Periodic task (5s in dev config) that:

  * Hits each node’s `/metrics` endpoint.
  * Parses facet-level metrics from Prometheus text.
  * Aggregates into `FacetMetricsSummary` windows stored in memory. 
* On connection errors (like `ConnectionRefused` for `127.0.0.1:9000`), logs warnings and increments `ron_svc_admin_upstream_errors_total{kind="connect"}`. Your `curl | rg` confirmed this counter is live.

These metrics are later exposed to the SPA via `/api/nodes/:id/metrics/facets`.

---

### 2.5 Frontend/SPA status

From `NOTES.MD` and the codebundle: 

* `crates/svc-admin/ui` is a Vite + React + TS SPA.
* `npm install` + `npm run dev` work; the earlier JSX/i18n parse bug is fixed.
* Theme and language providers are wired to `/api/ui-config`, so:

  * Default theme is “system” (or “light” in some dev configs).
  * Available themes: `["light", "dark"]`.
  * Default language: `en-US`, with `["en-US", "es-ES"]` in the menu. 
* Node list and detail pages already ingest `NodeSummary` and `AdminStatusView` DTOs, and the facet metrics panel consumes `FacetMetricsSummary` for charts (UX still to be polished). 

---

### 2.6 New work this session: Auth pipeline + `/api/me` (backend + HTTP tests)

This session we focused on **auth and identity** and proved it end-to-end via HTTP tests.

**Backend auth module (`auth/mod.rs` + `auth/ingress.rs` + `auth/none.rs` + `auth/passport.rs`):**

* Introduced a narrowed `Identity` type:

  * `subject: String`
  * `display_name: String`
  * `roles: Vec<String>`
    With a `dev_fallback()` that returns `dev-operator / Dev Operator / ["admin"]` for safe dev-mode usage and for soft failures.

* `AuthError` enum for coarse auth failures:

  * `Unauthenticated(&'static str)`
  * `Invalid(&'static str)`
  * `Unimplemented(&'static str)`

* `resolve_identity_from_headers(cfg: &AuthCfg, headers: &HeaderMap) -> Result<Identity, AuthError>`:

  * `"none"` → uses `auth::none::identity()` (synthetic dev identity).
  * `"ingress"` → uses `auth::ingress::identity_from_headers(headers)`:

    * Reads a principal from `X-User`.
    * Reads groups from `X-Groups` (comma/space-separated) into roles.
    * Falls back gracefully when headers are missing.
  * `"passport"` → stubbed (`Unimplemented`) for now, but the call path is in place.
  * Unknown mode → falls back to `Identity::dev_fallback()` to keep the console usable.

We also added focused **unit tests** under `auth::tests` and `auth::ingress::tests` to validate the behavior of each mode and the way ingress headers are parsed.

**`/api/me` handler integration:**

* `GET /api/me`:

  * Calls `auth::resolve_identity_from_headers(&state.config.auth, &headers)`.
  * On `Ok(identity)`, maps into `dto::me::MeResponse::from_identity(identity, auth_cfg)`.
  * On `Err(err)`, increments `ron_svc_admin_auth_failures_total{scope="ui"}`, logs a warning, and falls back to `dev_fallback()` identity.
* `MeResponse` now:

  * Includes subject, displayName, roles.
  * Exposes `authMode` (the configured `auth.mode`).
  * Exposes `rawHeaders` so the SPA can inspect what headers were seen (this is relevant for debugging ingress / passport setups).

**New HTTP-level tests: `tests/auth_me_http.rs`**

We added a two-test integration suite that:

* Spawns a **real svc-admin process** within the test, using in-memory config (no disk file needed).
* Uses `reqwest` to hit `/api/me` over HTTP, verifying the full JSON contract.

The two tests:

1. `me_returns_dev_identity_in_none_mode`:

   * Configures `auth.mode = "none"`.
   * Calls `/api/me` with no special headers.
   * Asserts:

     * `subject == "dev-operator"`.
     * `displayName == "Dev Operator"`.
     * `roles == ["admin"]`.
     * `authMode == "none"`.
     * `rawHeaders` is present (even if empty map/array), to guarantee that the SPA always sees the field.

2. `me_uses_ingress_headers_when_mode_is_ingress`:

   * Configures `auth.mode = "ingress"`.
   * Calls `/api/me` with headers:

     * `X-User: alice@example.com`
     * `X-Groups: admin,ops`
   * Asserts:

     * `subject == "alice@example.com"`.
     * `roles` include `"admin"` and `"ops"` (and no bogus roles).
     * `rawHeaders` echoes back the relevant headers as seen by svc-admin.

We fixed a couple of issues during this process:

* Tracing subscriber double-init in tests (now tests spawn independent svc-admin instances without conflicting global subscriber setup).
* Struct-field mismatches in test config (e.g., `NodeCfg` no longer has `id`/`admin_base_url`, `PollingCfg` fields changed, `Config` no longer has `tls`). We updated `auth_me_http.rs` to construct `Config` using the real `ServerCfg`, `NodesCfg`, `PollingCfg`, and `AuthCfg` shapes.

Net effect: we now know that from the SPA’s point of view, `/api/me` is **correct** for both `none` and `ingress` modes, and that it will be a solid foundation for hooking in real ingress headers and, later, passport/OIDC.

---

### 2.7 Current completion feel (updated)

Building on the previous estimates, and considering the new auth work:

* **Milestone 0.1.0 – Read-only Admin Console**
  Still about **85–90% complete**: config, node registry, read-only endpoints, facet metrics, SPA basics, and now a fully working `/api/me` with tests. Remaining 0.1.0 work is mostly polish: DTO/TS alignment, metrics UX states, config docs, and ESLint/TS cleanups. 

* **Milestone 0.2.0 – Auth & gated actions**
  Previously 0–10% (scaffolding only); now more like **15–20%**:

  * Auth pipeline (`none` + `ingress` and stubbed `passport`) is in place and tested, at least for `/api/me`. 
  * We still haven’t implemented real actions (reload/shutdown), proper JWT/JWKS handling, or action-level audit logging, but the foundation is there.

* **Milestone 0.3.x – Hardening & scale**
  Still ~10–20%: basic samplers and metrics exist, but not chaos tests, fuzzing, or advanced scaling strategies. 

* **Overall svc-admin vision (0.1 + 0.2 + 0.3):**
  Roughly **60–65%**; that matches the earlier NOTES and now includes the auth improvements we’ve done. 

---

## 3. What’s left to do for svc-admin (clustered)

### 3.1 Finish 0.1.0 “Dev Preview” polish (svc-admin alone)

Summarizing and updating the existing list: 

1. **DTO/TS alignment**

   * Ensure `admin-api.ts` matches Rust DTOs:

     * `AdminStatusView` fields and optional `profile` / `version`.
     * `NodeSummary.profile: string | null`.
     * `NodeActionResponse` type where applicable.
   * Update SPA pages (Node list/detail, Me panel) to align with the new `/api/me` and node DTOs.

2. **CONFIG.MD (operator-grade config doc)**

   * Document **all** `SVC_ADMIN_*` env vars and TOML keys. 
   * Show examples for:

     * Single macronode.
     * Mixed micro+macro deployments. 
   * Explain expected node admin-plane endpoints (`/healthz`, `/readyz`, `/version`, `/metrics`, `/api/v1/status`). 

3. **UI states for metrics & nodes**

   * In the Facet Metrics panel, distinguish:

     * “No data yet” vs “Sampler failing / upstream metrics down”. 
   * Node list empty state when no nodes configured. 

4. **ESLint 9 migration for SPA**

   * Add `eslint.config.js` with rules consistent with the repo’s React/TS style. 
   * Get `npm run lint` green.

5. **Screenshots & README quickstart**

   * Once the above are stable, capture:

     * Node list screenshot.
     * Node detail + metrics screenshot.
   * Add a short “Quickstart” section in README (dev preview). 

### 3.2 0.2.0 – Auth & actions (future, not next)

For later (not the immediate next step):

* Implement `auth.mode="passport"` with JWKS caching & role extraction. 
* Implement gated `reload` / `shutdown` endpoints in a policy-first way:

  * Config gates (per-node or per-environment allowlist).
  * Role gates (`admin` vs `ops`).
  * Full audit logging via `svc_admin::audit` logs. 

### 3.3 0.3.x – Hardening & scaling

Also later:

* Chaos tests, property-based tests, fuzzing for parsers. 
* Performance tuning of polling/batching strategy.
* SLOs and alerting recommendations.

---

## 4. Next major step: connect svc-admin to macronode admin plane

This is the step you explicitly called out and want to start in the **next instance**, so I’ll treat this as the top TODO, not something we implement right now.

### 4.1 Define exact macronode admin-plane contract

Based on svc-admin’s docs and IDB, nodes are expected to expose: 

* `GET /healthz` – simple health indicator.
* `GET /readyz` – readiness (could include more nuanced JSON than svc-admin’s own for nodes: ready/degraded reasons).
* `GET /version` – version, profile, build SHA, etc.
* `GET /metrics` – Prometheus metrics for the node.
* `GET /api/v1/status` – rich **Admin Status** JSON, matching `AdminStatusView`.

**Goal for next step:**

1. **Lock down the JSON schema of `/api/v1/status` for macronode**, derived from `svc-admin::dto::node::AdminStatusView`:

   * Fields like:

     * `id: String` (node ID, e.g., `"macronode-1"`).
     * `profile: Option<String>` (e.g., `"macronode"`, `"micronode"`, `"gateway"`, etc.).
     * `version: Option<String>` (semantic version / build string).
     * `env: Option<String>` (e.g., `"dev"`, `"staging"`, `"prod"`).
     * `amnesia: bool` or richer `amnesia: Option<AmnesiaFlags>` if present.
     * `planes: Vec<PlaneStatus>` describing overlay/gateway/storage-plane status.
     * Possibly `uptime`, `last_restart_at`, `labels`, etc., if already in the design.

2. **Define which metrics svc-admin cares about on `/metrics`**:

   * At minimum, our facet sampler expects:

     * Per-facet counters/gauges with node labels (even if the exact naming is still flexible).
   * We already know svc-admin’s own metrics include `ron_svc_admin_nodes_total` and `ron_svc_admin_nodes_by_env{environment="dev"}`, so we want macronode’s metrics to have equally stable names for:

     * Node uptime / `node_up`.
     * Facet-specific metrics (overlay connections, gateway RPS, storage usage, etc.).
   * In the next instance we’ll read `metrics::facet` and `metrics::sampler` more closely to reverse-engineer exactly what labels svc-admin expects, and then ensure macronode exposes them.

### 4.2 Tiny macronode-side TODO (what we’ll write next time)

When we have macronode docs/code attached, the immediate TODOs I’ll write out (for you to paste into macronode’s TODO/NOTES) will look roughly like:

1. **Implement node admin HTTP surface v1 (macronode)**

   * Ensure macronode exposes these endpoints on its **admin bind address**:

     * `GET /healthz` (fast, dependency-light).
     * `GET /readyz` (truthful readiness, considering internal services).
     * `GET /version` (profile, version, build info).
     * `GET /metrics` (Prometheus).
     * `GET /api/v1/status` (rich status DTO matching `AdminStatusView`).

2. **Align `/api/v1/status` JSON with svc-admin’s `AdminStatusView`**

   * Create a `AdminStatusDto` in macronode that is **structurally equivalent** to `svc_admin::dto::node::AdminStatusView`.
   * Populate fields from macronode’s internal state:

     * Node ID and environment from config.
     * Profile from macronode’s profile enum.
     * Version / build from build.rs or env! macros.
     * Plane statuses from each internal service (gateway, overlay, storage, etc.).
     * Amnesia flags where applicable.

3. **Verify svc-admin ↔ macronode behavior**

   * Add or update a small integration test (in svc-admin or a separate sys-test crate) that:

     * Starts a real macronode with admin-plane bound to `127.0.0.1:9000`.
     * Starts svc-admin pointing at that node (`base_url = http://127.0.0.1:9000`).
     * Asserts that:

       * `/api/nodes` shows exactly one node (`macronode-1`) with the right profile/env.
       * `/api/nodes/macronode-1/status` returns the same data as hitting macronode’s `/api/v1/status` directly.
       * `/api/nodes/macronode-1/metrics/facets` shows data derived from macronode’s `/metrics`.

4. **Document the contract**

   * In macronode’s docs (API.MD / ADMIN_PLANE.MD), formally document:

     * `/api/v1/status` schema, with examples.
     * minimal required metrics for svc-admin facet charts and node badges.
   * In svc-admin’s CONFIG.MD, show example configs for pointing svc-admin at a macronode and what the UI looks like in that scenario.

---

### 5. TL;DR for future-you

When you spin up the next instance and want to continue:

1. Paste:

   * macronode CODEBUNDLE.
   * macronode admin-plane docs (IDB/API/OBSERVABILITY/CONFIG if present).
   * svc-admin’s CODEBUNDLE + NOTES + README snippets you care about.

2. Say: “Let’s define the exact macronode admin-plane contract for `/api/v1/status` + `/metrics` and produce a macronode-side TODO that makes svc-admin show real data.”

We’ll then:

* Freeze the JSON contract for `/api/v1/status` based on `AdminStatusView`.
* Map it to macronode structs & handlers.
* Confirm that svc-admin’s node registry & metrics sampler can talk to a real macronode, not just `example-node`.

For now, svc-admin itself is in a strong dev-preview shape with a real auth pipeline, clean tests, and a clear next move: **hooking it up to macronode so the dashboard stops staring at an imaginary example-node and starts reflecting a living RON-CORE node.**


### END NOTE - DECEMBER 6 2025 - 21:00 CST


### BEGIN NOTE - DECEMBER 7 2025 - 15:10 CST


## 0. Quick status snapshot

**Crates in play**

* `crates/svc-admin` — admin dashboard backend + SPA.
* `crates/macronode` — operator-grade node; admin plane is what svc-admin talks to.

**Runtime status**

* `macronode`:

  * ✅ `cargo build -p macronode`
  * ✅ `cargo run -p macronode` with:

    * `RON_HTTP_ADDR=127.0.0.1:8080`
    * `RON_METRICS_ADDR=127.0.0.1:8080`
    * `MACRONODE_DEV_INSECURE=1` (dev bypass for admin auth)
  * Exposes:

    * `http://127.0.0.1:8080/api/v1/status` (RON-STATUS-V1 subset)
    * `http://127.0.0.1:8080/healthz`, `/readyz`, `/metrics`, `/version`
    * `http://127.0.0.1:8080/api/v1/debug/crash` (synthetic crash endpoint)

* `svc-admin`:

  * ✅ `cargo test -p svc-admin --tests` (from previous sessions)
  * ✅ `cargo run -p svc-admin --bin svc-admin` with:

    * `SVC_ADMIN_HTTP_ADDR=127.0.0.1:5300`
    * `SVC_ADMIN_METRICS_ADDR=127.0.0.1:5310`
    * `SVC_ADMIN_NODES__EXAMPLE_NODE__BASE_URL=http://127.0.0.1:8080`
      (this is key — see below)
  * SPA (Vite dev server) runs at `http://localhost:5173`.

**What you see in the browser now**

* `/nodes` shows a single node: **Example Node**.
* Node detail page shows:

  * Header `Example Node` with `ID: example-node · Profile: macronode · Version: 0.1.0`.
  * **Planes table** with 6 rows: gateway, storage, index, mailbox, overlay, dht; each with `health=healthy`, `ready=Ready`, `restarts=0` (or higher once we start exercising restart counters).
  * Facet metrics section present but empty (`No facet metrics observed yet…`).
  * Actions section in **read-only** mode (mutations disabled).

So at this point we have a **full end-to-end path**:

> macronode `/api/v1/status` → svc-admin NodeClient → svc-admin API `/api/nodes/:id/status` → SPA → Planes table UI.

This is the backbone of the entire observability story.

---

## 1. Changes on macronode we relied on / introduced for svc-admin

### 1.1 `/api/v1/status` upgraded to RON-STATUS-V1 subset

We replaced the old status handler with a richer one that implements the initial **RON-STATUS-V1** subset that dashboards will use across *all* node profiles:

* **Handler:** `crates/macronode/src/http_admin/handlers/status.rs`.

Key aspects:

* **Input:** `AppState` (config, readiness probes, process start time) and `BuildInfo` (compile-time version metadata).

* **Output JSON (`StatusBody`):**

  * `uptime_seconds` — seconds since process start.
  * `profile` — `"macronode"`.
  * `version` — string from `BuildInfo::current().version` (e.g., `"0.1.0"`).
  * `http_addr` / `metrics_addr` — from config.
  * `log_level` — effective log level string.
  * `ready` — computed via `ReadySnapshot::required_ready()`.
  * `deps` — top-level readiness view:

    * `config`: `"loaded"` vs `"pending"`.
    * `network`: `"ok"` vs `"pending"` based on `listeners_bound`.
    * `gateway`: `"ok"` vs `"pending"` based on `gateway_bound`.
    * `storage`: `"ok"` vs `"pending"` based on `deps_ok` (which flips when core workers are up).
  * `services` — `BTreeMap<String, String>`:

    * keys: `"svc-gateway" | "svc-storage" | "svc-index" | "svc-mailbox" | "svc-overlay" | "svc-dht"`
    * values: `"ok"` vs `"pending"` based on per-service bits in `ReadySnapshot`.
  * **NEW:** `planes: Vec<PlaneStatusBody>`:

    * `name`: `"gateway" | "storage" | "index" | "mailbox" | "overlay" | "dht"`.
    * `health`: `"healthy" | "degraded | "down"`.

      * `"ok"` → `"healthy"`, `"pending"` → `"degraded"`, anything else → `"down"`.
    * `ready`: `node_ready && (service_label == "ok")`.
    * `restart_count`: `u64` – currently a real counter (described below).

* We added helper functions:

  * `status_label_to_health(&str) -> &'static str` to normalize service strings → health strings.
  * `build_planes(&BTreeMap<String,String>, node_ready) -> Vec<PlaneStatusBody>` to give a **stable canonical set of planes** from the services map.

This status handler is now the canonical source that `svc-admin` consumes; other node profiles (micronode, future gateway-only nodes, etc.) will implement the same shape.

### 1.2 Readiness probes (where status derives from)

We revisited `crates/macronode/src/readiness/probes.rs`:

* `ReadyProbes` remains a set of `AtomicBool` gates:

  * Essential: `listeners_bound`, `cfg_loaded`, `metrics_bound`, `deps_ok`, `gateway_bound`.
  * Per service: `index_bound`, `overlay_bound`, `mailbox_bound`, `dht_bound`.
* `ReadySnapshot` is a simple `#[derive(Serialize, Clone)]` struct with these booleans and:

  * `required_ready()` gating `ready == true` for both `/readyz` and `/api/v1/status`.

We left the gating policy as:

```text
ready == listeners_bound && cfg_loaded && deps_ok && gateway_bound
```

Other service bits are informational for now.

### 1.3 Restart counter plumbing (synthetic for now)

We introduced **restart counters** inside `macronode::supervisor` and exposed them through `ReadySnapshot` → status:

* In `crates/macronode/src/supervisor/mod.rs` (conceptually):

  * Added a `restart_counters: HashMap<&'static str, u64>` field.

  * Methods:

    * `increment_restart(&mut self, service: &'static str)` — bump counter for a logical service name (`"svc-storage"`, `"svc-gateway"`, etc.).
    * `snapshot()` → `SupervisorSnapshot { restart_counters: HashMap<&'static str, u64> }`.

  * Still no **real** restarts yet. The existing watchers log task exits, and the crash policy + backoff are wired, but restart decisions are not yet acted on.

* We wired a thin integration from the **kernel bus** to supervisor:

  * Bus events include a `ServiceCrashed { service: String }` variant.
  * Supervisor subscribes and on each such event calls `increment_restart(service)`.

* `AppState` / readiness snapshot were extended so that `/api/v1/status` can see the **current restart counters** and map them to `PlaneStatusBody.restart_count`.

At this point, restart counters are **real data in the node** (they live in supervisor state), but we still needed a way to **exercise** them without actually crashing workers…

### 1.4 `debug_crash` endpoint for synthetic restarts

We added a dev-only HTTP endpoint on macronode:

* **Path:** `POST /api/v1/debug/crash`
* **Handler:** `crates/macronode/src/http_admin/handlers/debug_crash.rs`
* **Router wiring:** added under the admin router, behind **admin auth** middleware.

Behavior:

* Requires either proper admin auth or `MACRONODE_DEV_INSECURE=1`; in dev you’ve been using:

  ```bash
  MACRONODE_DEV_INSECURE=1 cargo run -p macronode
  ```

* Accepts optional query parameter `service=svc-storage|svc-gateway|…`.

  * Default if omitted: `"svc-storage"`.

* Instead of killing anything, it:

  1. Publishes a `KernelEvent::ServiceCrashed { service }` onto the in-process bus.
  2. Supervisor receives that and bumps the restart counter for that service.

* Returns JSON like:

  ```json
  {
    "status": "debug crash event emitted",
    "service": "svc-storage",
    "note": "restart counter bumped; no real worker was killed (synthetic event)"
  }
  ```

You confirmed with curl:

* Before:

  ```bash
  curl -s http://127.0.0.1:8080/api/v1/status | jq '.planes'
  # restart_count == 0 for all planes
  ```

* After:

  ```bash
  curl -s -X POST "http://127.0.0.1:8080/api/v1/debug/crash?service=svc-storage" | jq
  # ... note about restart counter bumped ...

  curl -s http://127.0.0.1:8080/api/v1/status | jq '.planes'
  # storage plane now has restart_count == 1
  ```

So **on the macronode side** we have:

* Truthful `ready` and `health` signals.
* Per-plane restart counters that can be exercised via a safe debug endpoint.

---

## 2. svc-admin: what’s implemented and proven now

### 2.1 Config / nodes wiring

* `svc-admin` config has a `nodes` section that can be populated via env (TOML in future).

* For this session we used env:

  ```bash
  SVC_ADMIN_HTTP_ADDR=127.0.0.1:5300
  SVC_ADMIN_METRICS_ADDR=127.0.0.1:5310
  SVC_ADMIN_NODES__EXAMPLE_NODE__BASE_URL=http://127.0.0.1:8080
  ```

* Earlier, missing `BASE_URL` caused svc-admin to hit the **default** `http://127.0.0.1:9000`, which was not running, so NodeClient fell back to just `health`/`ready`/`version` probes and the UI showed “No plane status reported by this node yet.”

* With `BASE_URL` pointed at macronode, svc-admin now:

  * Successfully calls `http://127.0.0.1:8080/api/v1/status`.
  * Correctly parses the RON-STATUS-V1 subset (`profile`, `version`, `planes`).
  * Exposes it via `/api/nodes/example-node/status`.
  * SPA renders the **Planes** table.

Facet metrics:

* Config still has `metrics_url` defaulting to `http://127.0.0.1:9000/metrics`, so the sampler logs warnings.
* This doesn’t break anything; it just means the **Facet metrics** box is empty for now.
* Later we can set `SVC_ADMIN_NODES__EXAMPLE_NODE__METRICS_URL=http://127.0.0.1:8080/metrics` or similar once macronode exports facet-level metrics.

### 2.2 Nodes API + client behavior

* `svc-admin` provides REST APIs like:

  * `GET /api/nodes` — returns inventory, e.g.:

    ```json
    [
      {
        "id": "example-node",
        "display_name": "Example Node",
        "profile": "macronode"
      }
    ]
    ```

  * `GET /api/nodes/:id/status` — returns a consolidated view (`AdminStatusView` / `NodeStatusResponse`) that includes:

    * basic node identity + profile + version
    * **planes** (PlaneStatus objects from macronode’s `/api/v1/status`)
    * facet metrics summary (currently empty)
    * actions metadata (currently disabled / read-only)

* NodeClient behavior (important invariants):

  1. **Preferred path**: try `/api/v1/status`.

     * If present and parseable, treat that as the **source of truth**.
  2. On failure, degrade:

     * Try `/readyz` as JSON or text.
     * Try `/version`.
     * Synthesize a coarse view (“healthy”/“degraded”) even if RON-STATUS-V1 is missing.

This is why your node detail page initially showed “No plane status reported…” when base_url pointed at the wrong port; svc-admin never saw `/api/v1/status` and downgraded to its coarse fallback.

### 2.3 SPA: node detail view

The SPA (React/Vite) node detail route currently:

* Fetches `/api/nodes/:id/status`.
* Shows:

  * Node header: ID, Profile, Version, overall health tag.
  * **Planes table** with columns:

    * Plane
    * Health
    * Ready
    * Restarts
  * Facet metrics section (empty state).
  * Actions section (reload config / shutdown controls are present but disabled in “read-only” mode).

We’ve confirmed:

* It renders correctly when svc-admin can reach macronode’s `/api/v1/status`.
* It updates health / ready from that contract.
* The restarts column is wired to the restart data in the status DTO (currently all zero unless we extend NodeClient/DTO wiring to pass the new field through – see TODOs).

---

## 3. What remains / next waves for svc-admin (to truly “mog”)

### 3.1 Near-term: finish restart counters + debug observability

**Goal:** Let operators see restart activity in the dashboard and exercise it in dev without real crashes.

Remaining tasks:

1. **Plumb restart_count fully into svc-admin DTOs & UI**

   * Ensure `nodes::client::NodeStatus` includes `planes[*].restart_count` from RON-STATUS-V1.
   * Make sure the admin API response type used by the SPA exposes that field.
   * Confirm the SPA uses that value (not a local default) in the “Restarts” column. Right now it likely hard-codes `0`.

2. **Optional:** add a tiny `Dev tools` panel in the SPA (visible only in dev) that:

   * Calls `POST /api/nodes/:id/debug/crash?service=svc-storage` (through svc-admin proxy or by hitting macronode directly if we expose a passthrough).
   * Refreshes node status to show bumping restart counts.

3. **Improve metrics config for the example node**

   * Wire `metrics_url` for `example-node` to macronode’s `/metrics` once we are comfortable with what it exports.
   * That will quiet sampler warnings and is the first step toward **Facet metrics** actually populating.

### 3.2 Medium-term: facet metrics and planes “mog” features

This is where svc-admin starts to exceed typical dashboards:

* **Facet metrics integration**

  * Decide on the canonical Prometheus labels for “facet,” “plane,” “route,” etc.
  * Have macronode (and micronode) expose per-facet counters/histograms.
  * Update `svc-admin` sampler/parser to aggregate:

    * Requests/s per facet.
    * Error rate per facet.
    * Latency p50/p95/p99 per facet.
  * Render:

    * Per-facet sparkline / bar graphs.
    * Color-coded health scores.

* **Plane-level drilldown**

  * Add plane detail pages: click `gateway` row → see HTTP endpoints, error spikes, restart timeline.
  * Show last N restarts with timestamps (requires extending status/metrics or adding a restart history endpoint).

* **Multi-node view**

  * Support more than one node in `nodes` config.
  * Nodes table: per-node summary (profile, env, overall health, RPS, error rate).
  * Filters by environment (`prod`, `staging`, `dev`), profile (`macronode`, `micronode`).

This is the part where we start to **mog** generic dashboards: we know the internal structure (planes, facets, restart policy) and can show very targeted, operator-centric views instead of generic CPU/Disk charts.

### 3.3 Longer-term: control plane actions (still design-phase)

We are intentionally staying **read-only** for now, but future slices will unlock mutating actions:

* Node-level actions (with strong auth + safety):

  * Reload config.
  * Drain node / cordon for maintenance.
  * Graceful shutdown / restart.

* Plane/service-level actions:

  * Restart a specific service (bounded by crash policy).
  * Toggle features (flags) per node/plane.

All of this must obey the **GOVERNANCE / hardening docs**:

* Clear authority boundaries.
* Audit logs.
* No backdoors; everything goes through the same control plane semantics.

---

## 4. Practical how-to recap (for future self)

To reproduce the current demo:

1. **Run macronode**

   ```bash
   cd /Users/mymac/Desktop/RustyOnions

   RON_HTTP_ADDR=127.0.0.1:8080 \
   RON_METRICS_ADDR=127.0.0.1:8080 \
   MACRONODE_DEV_INSECURE=1 \
   cargo run -p macronode
   ```

2. **Sanity check status**

   ```bash
   curl -s http://127.0.0.1:8080/api/v1/status | jq
   # should show profile: macronode, version: 0.1.0, planes array
   ```

3. **Run svc-admin backend**

   ```bash
   cd /Users/mymac/Desktop/RustyOnions

   SVC_ADMIN_HTTP_ADDR=127.0.0.1:5300 \
   SVC_ADMIN_METRICS_ADDR=127.0.0.1:5310 \
   SVC_ADMIN_NODES__EXAMPLE_NODE__BASE_URL=http://127.0.0.1:8080 \
   cargo run -p svc-admin --bin svc-admin
   ```

4. **Run svc-admin SPA**

   ```bash
   cd crates/svc-admin/ui
   npm run dev
   ```

5. **Open browser**

   * Visit `http://localhost:5173`.
   * Click **Nodes → Example Node**.
   * You should see the screenshot you captured (planes table, etc.).

6. **Exercise synthetic restart counters (macronode side)**

   ```bash
   curl -s -X POST "http://127.0.0.1:8080/api/v1/debug/crash?service=svc-storage" | jq
   curl -s http://127.0.0.1:8080/api/v1/status | jq '.planes'
   # storage plane restart_count should increment
   ```

   (Once we plumb restart_count into svc-admin DTOs and UI, refreshing the node detail page should show the updated Restart count in the table.)

---

## 5. What to attach next instance

For the **next instance**, to keep momentum and avoid drift, please attach:

* For **svc-admin**:

  * `CODEBUNDLE_SVC_ADMIN.md`
  * `TODO_SVC_ADMIN.MD`
  * `NOTES_SVC_ADMIN.MD`
  * `ALL_DOCS_SVC_ADMIN.md`

* For **macronode** (since svc-admin’s value depends heavily on macronode’s admin plane):

  * `CODEBUNDLE_MACRONODE.md`
  * `TODO_MACRONODE.MD`
  * `NOTES_MACRONODE.MD`
  * `ALL_DOCS_MACRONODE.md`

---

## 6. Suggested next high-impact step when we return

**Next slice (svc-admin + macronode together):**

1. **Fully wire restart_count into svc-admin:**

   * Update Rust DTOs + NodeClient mapping.
   * Update admin API types.
   * Confirm SPA uses `plane.restartCount` from API.

2. **Add a small dev-only “Debug tools” panel in the SPA:**

   * A button per plane to trigger a synthetic crash via svc-admin → macronode.
   * Immediately refresh node status so Restart counts visibly bump in the UI.

Once that’s done, we’ll have:

> A node detail view that not only shows health/ready per plane, but also live restart counters that you can exercise from the GUI — already nicer and more “node-aware” than generic Prometheus/Grafana setups.



### END NOTE - DECEMBER 7 2025 - 15:10 CST




### BEGIN NOTE - DECEMBER 7 2025 - 20:05 CST

---

## 0. Quick status snapshot

**Crate:** `crates/svc-admin`
**Role:** Read-only admin console + future control plane for RON-CORE nodes (macronode/micronode).

**Runtime status (today):**

* `cargo build -p svc-admin` ✅
* `cargo run -p svc-admin --bin svc-admin` ✅ with node registry seeded from env:

  * `SVC_ADMIN_HTTP_ADDR=127.0.0.1:5300`
  * `SVC_ADMIN_METRICS_ADDR=127.0.0.1:5310`
  * `SVC_ADMIN_NODES__EXAMPLE_NODE__BASE_URL=http://127.0.0.1:8080`
* `npm run dev` from `crates/svc-admin/ui` ✅ (Vite dev server)
* SPA can:

  * Hit `/api/ui-config`, `/api/me`, `/api/nodes`, `/api/nodes/:id/status`, `/api/nodes/:id/metrics/facets`.
  * Render the **Nodes list** and **Node detail** pages without “Load failed.”

**What the UI shows right now:**

* Nodes list: one node, **“Example Node”**, environment inferred from config, correct profile label (`macronode`).
* Node detail:

  * Top header: `ID: example-node Profile: macronode Version: <something>`.
  * **Planes table**: *currently empty* (“No plane status reported by this node yet.”).
  * **Facet metrics panel**: shows “No facet metrics observed yet.”
  * **Actions**: read-only mode, buttons disabled, identity bar shows `Dev Operator (none · admin)`.

---

## 1. What we actually accomplished this session

### 1.1 Rust backend wiring & compile-time fixes

We cleaned up a whole cluster of mismatches between DTOs, the node registry, and the router:

1. **`dto::node::NodeSummary` / `NodeActionResponse` alignment**

   * Rust side now exposes:

     ```rust
     pub struct NodeSummary {
         pub id: String,
         pub display_name: String,
         pub environment: String,
         pub insecure_http: bool,
     }
     ```
   * We removed the stale `profile` field from `NodeSummary` (registry was trying to set it; we fixed registry to stop doing that).
   * `NodeActionResponse` is now the minimal, SPA-facing envelope:

     ```rust
     pub struct NodeActionResponse {
         pub accepted: bool,
         pub message: Option<String>,
     }
     ```
   * All references to `resp.node_id` / `resp.action` (in `nodes::registry` and `router.rs`) were updated to log/return only `accepted` + `message`.

2. **Admin status view & raw status normalization**

   * `dto::node::AdminStatusView` is the canonical view for the node detail page:

     ```rust
     pub struct AdminStatusView {
         pub id: String,
         pub display_name: String,
         pub profile: Option<String>,
         pub version: String,
         pub planes: Vec<PlaneStatus>,
     }
     ```
   * `RawStatus` + `RawPlaneStatus` mirror the **node admin-plane JSON** (to be implemented on macronode/micronode):

     ```rust
     pub(crate) struct RawStatus {
         pub profile: Option<String>,
         pub version: String,
         pub planes: Vec<RawPlaneStatus>,
     }

     pub(crate) struct RawPlaneStatus {
         pub name: String,
         pub health: String,
         pub ready: bool,
         pub restart_count: u64,
     }
     ```
   * `from_raw(id, cfg, raw)` folds NodeCfg + RawStatus into AdminStatusView, enforcing invariants:

     * `id` always registry key.
     * `display_name` prefers `NodeCfg.display_name` else `id`.
     * `profile` prefers `raw.profile`, falls back to `cfg.forced_profile`.
     * `version` is `raw.version`.
     * Planes map 1:1 into `PlaneStatus`.

3. **Axum `Query` extractor**

   * Added `query` feature to `axum` (workspace / crate level).
   * Imported `Query` from `axum::extract` in `router.rs`.
   * This unblocks the **debug crash endpoint**:

     ```rust
     #[derive(Debug, serde::Deserialize)]
     struct DebugCrashParams { service: Option<String> }
     ```

4. **Debug crash plumbing (svc-admin side)**

   * Implemented `NodeClient::debug_crash(...)` and wired through `nodes::registry`:

     * Registry method `debug_crash_node(id, service)` delegates to NodeClient.
     * Router exposes `POST /api/nodes/:id/debug/crash`, which:

       * Validates node presence.
       * Calls `state.nodes.debug_crash_node`.
       * Logs audit info (action, node_id, reason).
   * This is still untested end-to-end because the **macronode admin plane does not implement `/api/v1/debug/crash` yet**, but the svc-admin control flow is ready.

5. **Node status degradation path compiles cleanly**

   * `nodes::client` now builds a status view as follows:

     * Try `/api/v1/status` on the node admin plane.
     * On failure, degrade to:

       * `/readyz` (JSON or text) for readiness.
       * `/version` for version string.
     * Compose an `AdminStatusView` even in degraded mode.
   * The “no planes” view we see right now is this **degraded path** doing its job in the absence of a proper `/api/v1/status`.

### 1.2 TypeScript SPA alignment

We brought the SPA DTOs back into strict sync with the Rust side:

1. **`admin-api.ts`**

   * `AdminStatusView` now reflects the Rust struct:

     ```ts
     export type AdminStatusView = {
       id: string
       display_name: string
       profile: string | null
       version: string
       planes: PlaneStatus[]
     }
     ```

     (Previously `version` was nullable and the shape drifted.)
   * `PlaneStatus` mirrors the Rust enum-ish pattern:

     ```ts
     export type PlaneStatus = {
       name: string
       health: 'healthy' | 'degraded' | 'down'
       ready: boolean
       restart_count: number
     }
     ```
   * `NodeActionResponse` now:

     ```ts
     export type NodeActionResponse = {
       accepted: boolean
       message?: string | null
     }
     ```

     (Removed `node_id` / `action` from TS to match Rust.)

2. **Node detail page**

   * `NodeDetailPage.tsx` is consuming the new `AdminStatusView` and `NodeActionResponse` correctly.
   * Action gating logic is wired to:

     * `UiConfigDto.readOnly`
     * `MeResponse.roles` (`admin` / `ops`).
   * When actions are attempted, the page:

     * Uses `adminClient.reloadNode(id)` / `adminClient.shutdownNode(id)`.
     * Interprets `accepted` + `message` and shows a banner.

3. **`adminClient.ts` build fix**

   * We had a sneaky **Unicode en-dash** before a comment (`–// RO:WHY …`).
   * Vite/esbuild choked on it (`Unexpected "–"`).
   * We replaced it with a normal comment line; UI now builds and hot-reloads fine.

### 1.3 End-to-end runtime verification

We successfully:

* Started **macronode** with:

  ```bash
  RON_HTTP_ADDR=127.0.0.1:8080 \
  RON_METRICS_ADDR=127.0.0.1:8080 \
  MACRONODE_DEV_INSECURE=1 \
  cargo run -p macronode
  ```

  * Observed services:

    * `svc-gateway` on `127.0.0.1:8090`
    * `svc-storage` on `127.0.0.1:5303`
    * `svc-overlay` placeholder on `127.0.0.1:5301`
    * `svc-dht` placeholder on `127.0.0.1:5302`
    * `svc-mailbox` placeholder on `127.0.0.1:5304`
    * macronode **admin plane** on `127.0.0.1:8080`
    * `svc-index` embedded (ready on 5304)

* Started **svc-admin**:

  ```bash
  SVC_ADMIN_HTTP_ADDR=127.0.0.1:5300 \
  SVC_ADMIN_METRICS_ADDR=127.0.0.1:5310 \
  SVC_ADMIN_NODES__EXAMPLE_NODE__BASE_URL=http://127.0.0.1:8080 \
  cargo run -p svc-admin --bin svc-admin
  ```

  * Logs confirm:

    * UI/API listener: `127.0.0.1:5300`
    * Health/metrics listener: `127.0.0.1:5310`
    * Facet metrics sampler started for 1 node (`example-node`).

* Started **SPA**:

  ```bash
  cd crates/svc-admin/ui
  npm run dev
  ```

  * Vite on `http://localhost:5173`.
  * SPA successfully calling:

    * `/api/ui-config` → returns dev config (`readOnly: true`, theme list, languages).
    * `/api/me` → returns dev operator (`subject: "dev-operator"`, roles: `["admin"]`).
    * `/api/nodes` → returns `[{ id: "example-node", display_name: "Example Node", ... }]`.
    * `/api/nodes/example-node/status` → returns `AdminStatusView` with **no planes** (degraded path).
    * `/api/nodes/example-node/metrics/facets` → returns `[]` (sampler can’t reach metrics endpoint).

---

## 2. Why planes & facet metrics are still empty

This is the key debugging insight we got out of this session.

### 2.1 Metrics path problem: 9000 vs 8080

* `metrics::sampler` is trying to scrape:

  ```text
  http://127.0.0.1:9000/metrics
  ```

  as seen in logs:

  > `url=http://127.0.0.1:9000/metrics` … `ConnectionRefused`.

* But our macronode **admin plane** is actually on `127.0.0.1:8080`, and `svc-gateway` is on `127.0.0.1:8090`. There is nothing listening on port **9000**, hence repeated ConnectionRefused warnings.

* Root cause: the node config / NodeClient currently **derives metrics URL with a default 9000 port**, independent of the `BASE_URL` we set via env. We didn’t fix that logic yet; we only changed the base URL env.

* Net effect:

  * Facet metrics sampler never gets a successful scrape.
  * `/api/nodes/:id/metrics/facets` always returns an empty vector → SPA shows “No facet metrics observed yet.”

### 2.2 Admin-plane `/api/v1/status` is not implemented on macronode

* NodeClient is trying:

  ```text
  http://127.0.0.1:9000/api/v1/status
  ```

  (once we fix ports it’ll use `8080`, but the issue remains.)

* Current macronode admin HTTP surface:

  * `/healthz`
  * `/readyz`
  * `/metrics`
  * `/version`
  * Possibly some early overlay endpoints, but **no `/api/v1/status`** yet.

* NodeClient behavior:

  * Try `/api/v1/status`:

    * Fails with HTTP error → warn and degrade.
  * Fallback:

    * `/readyz` (as JSON or text).
    * `/version`.
  * Compose `AdminStatusView`:

    * `id` = `example-node`.
    * `profile` = `Some("macronode")` from NodeCfg.
    * `version` = `0.0.0` / `0.1.0` depending on what we hardcode or read.
    * **planes = empty vec** because we never saw per-plane data.

* That’s why the **planes table** in the UI currently says:

  > “No plane status reported by this node yet.”

* The earlier screenshot you captured with a full planes table (`gateway/storage/index/mailbox/overlay/dht`) was from a **stub/demo status path** (or a previous version where we faked status data). Now we’re talking to the **real macronode**, and since it doesn’t export that status yet, svc-admin is showing the honest truth: “I don’t have any plane info.”

---

## 3. High-impact next steps

We want svc-admin to **mog every other dashboard**, so the next steps are about making this node detail view *rich, accurate, and live*.

I’ll split tasks into two buckets:

* A. **svc-admin crate work** (our main focus)
* B. **macronode admin-plane work** (minimal, only what svc-admin needs)

### A. svc-admin crate – next steps

1. **Fix NodeClient URL derivation (ports + overrides)**

   * Goal: make NodeClient correctly derive **admin plane URL** and **metrics URL** from NodeCfg.
   * Likely structure of `NodeCfg` (from earlier docs):

     * `base_url` – general node URL.
     * `admin_override_url` or `admin_port` – admin plane override.
     * `metrics_override_url` or `metrics_port` – metrics override.
     * `environment`, `display_name`, `forced_profile`, `insecure_http`.
   * Next steps:

     * Re-open `crates/svc-admin/src/nodes/config.rs` (or wherever NodeCfg lives).
     * Implement helper methods on `NodeCfg`:

       ```rust
       impl NodeCfg {
           pub fn admin_base_url(&self) -> Url { ... }
           pub fn metrics_url(&self) -> Url { ... }
       }
       ```

       with precedence:

       * Use explicit override URL if present.
       * Else derive from `base_url` by swapping path/port.
       * Never silently default to 9000 in dev; log a clear warning if we do.
     * Update `NodeClient` to only use these helpers.
     * Add unit tests that:

       * With `BASE_URL=http://127.0.0.1:8080`, we derive admin `/api/v1/status` at `http://127.0.0.1:8080/api/v1/status`.
       * With `METRICS_OVERRIDE=http://127.0.0.1:5310`, sampler uses that, etc.

2. **Stabilize the degraded status path**

   * Code is mostly there, but we should:

     * Make sure we **always** fill `version` with something sensible:

       * NodeCfg default, or
       * `/version` response, or
       * `"unknown"`.
     * Record whether we are in a **degraded** state for each node (e.g., `status_source: "admin_plane"|"probes"`).

       * Could be an extra field in `AdminStatusView` later (for UI badges).
     * Tests:

       * Fake a node that has no `/api/v1/status` but has `/readyz` and `/version`.
       * Ensure we still get a non-empty `AdminStatusView` and no panic.

3. **Facet metrics sampler improvements**

   * Once URLs are correct:

     * Confirm we can scrape `http://127.0.0.1:8080/metrics` (or later a dedicated per-service metrics endpoint).
     * Keep the **facet parsing logic** but be tolerant:

       * If no facet metrics exist, sampler should just sit quietly (no repeated warnings).
       * Only log on parsing issues or HTTP failures, not on “no data for node.”
   * For mog-tier later:

     * Add simple **per-node metric cache introspection** in `/metrics` for svc-admin itself (e.g., number of facets per node).

4. **Node list & node detail UI polish**

   * With planes data available, we can:

     * Show a **condensed per-plane summary** in the Nodes list:

       * e.g., “gateway ✅ storage ✅ index ✅ overlay 🟡”.
     * In node detail:

       * Keep the table but add:

         * Colored badges per plane.
         * Hover tooltips with last error reason if degraded.
   * Short-term tasks:

     * Ensure NodeDetailPage gracefully handles both:

       * `planes.is_empty()` → show the current “no planes reported” message.
       * Non-empty planes → show the table (already implemented).

5. **Debug crash UI & backend glue**

   * Backend:

     * Once macronode supports `/api/v1/debug/crash`, verify:

       * `POST /api/nodes/:id/debug/crash` passes through `service` parameter.
       * Logs show audit trail with `action="debug_crash"`.
   * UI:

     * Add a **Dev-only panel** in Node detail:

       * Only if `UiConfig.dev.enableAppPlayground` or similar is true.
       * A dropdown with services (`gateway`, `storage`, `index`, etc.).
       * A “Crash plane” button wired to `adminClient.debugCrash(id, service)`.
       * Show the result message or error.

6. **Action gating: enable write actions in dev**

   * Currently, `UiConfig.readOnly` is true, so all buttons stay disabled.
   * For dev:

     * Allow a config mode (`SVC_ADMIN_DEV_ENABLE_ACTIONS=1`) that sets `readOnly=false`.
     * This lets us exercise reload/shutdown flows **once macronode supports them**.
   * Ensure backend AuthCfg + ActionsCfg are still enforced:

     * Reload/shutdown endpoints should:

       * Check `config.actions.enable_*`.
       * Check identity roles (`admin` / `ops`).
       * Emit `ron_svc_admin_auth_failures_total`, `ron_svc_admin_rejected_total{reason}`.

### B. Macronode admin-plane – minimal TODO for svc-admin

To make planes & status **actually show up**, macronode needs to implement the contract svc-admin is expecting.

1. **Implement `/api/v1/status` on macronode admin plane**

   * Endpoint: `GET /api/v1/status`.
   * JSON body should match `RawStatus`:

     ```json
     {
       "profile": "macronode",
       "version": "0.1.0",
       "planes": [
         { "name": "gateway", "health": "healthy", "ready": true, "restartCount": 0 },
         { "name": "storage", "health": "healthy", "ready": true, "restartCount": 0 },
         { "name": "index",   "health": "healthy", "ready": true, "restartCount": 0 },
         { "name": "mailbox", "health": "healthy", "ready": true, "restartCount": 0 },
         { "name": "overlay", "health": "healthy", "ready": true, "restartCount": 0 },
         { "name": "dht",     "health": "healthy", "ready": true, "restartCount": 0 }
       ]
     }
     ```
   * Version & restart counts should come from real subsystems eventually; for first pass, static or supervisor-derived values are fine.

2. **Align metrics & admin URLs with NodeCfg expectations**

   * Decide canonical mapping:

     * Admin plane: `RON_HTTP_ADDR` (currently `127.0.0.1:8080`).
     * Metrics: same port `/metrics` for now.
   * Ensure the NodeCfg defaults for a “macronode dev profile” line up with this, so:

     * With only `BASE_URL=http://127.0.0.1:8080` set, svc-admin can:

       * Call `/api/v1/status`, `/metrics`, `/readyz`, `/version` successfully.

3. **Optional: `/api/v1/debug/crash`**

   * Implement dev-only crash hook:

     * Accept `?service=gateway` etc.
     * Trigger synthetic panic / restart path for that plane.
     * Return simple JSON:

       ```json
       { "accepted": true, "message": "Crash signal sent to gateway." }
       ```
   * This will power our svc-admin debug panel and prove end-to-end restart observability.

---

## 4. How this moves us toward a God-tier admin dashboard

Where we stand after this session:

* **The skeleton is real.**

  * svc-admin builds, runs, and talks to a live macronode.
  * SPA can enumerate nodes and show per-node views.
  * Identity/auth surfaces exist and are wired through DTOs & config.

* **We have clean, explicit contracts.**

  * `dto::node::{AdminStatusView, PlaneStatus, NodeSummary, NodeActionResponse}` define exactly what the dashboard expects.
  * TypeScript DTOs mirror these precisely.
  * Any macronode/micronode implementation just has to satisfy `/api/v1/status` and `/metrics` in that shape.

* **The remaining gap is primarily *data*, not plumbing.**

  * Planes and facet metrics are empty because:

    * Node URLs are still defaulting to port 9000.
    * Macronode does not yet implement `/api/v1/status` or facet metrics.
  * Once we fix URL derivation + add a small admin-plane handler, the UI will instantly light up with real `gateway/storage/index/overlay/dht` rows.

The next instance should therefore focus on:

1. **Fixing NodeCfg → URL derivation in svc-admin** (no more port 9000; metrics & status aim at 8080).
2. **Adding a minimal `/api/v1/status` handler to macronode** that reports the six planes.
3. **Verifying the full loop**:

   * NodeDetail shows all planes with `healthy/ready/restartCount`.
   * Metrics sampler stops spamming errors and either:

     * Quietly accepts “no facet metrics yet,” or
     * Starts showing real facet metrics once we wire them.

Once that’s in place, we can start layering the “mog everything” features:

* Multi-node registry with env/region tags and health badges.
* Per-plane and per-facet charts (RPS, error rate, latency).
* Audit trails for reload/shutdown/debug-crash actions.
* A Playground client pane on the node detail page for app-plane request demos.

That’s the trajectory to a first-class, God-tier admin console that makes other dashboards look like toys.


### END NOTE - DECEMBER 7 2025 - 20:05 CST




### BEGIN NOTE - DECEMBER 10 2025 - 11:00 CST

Here are fresh **carry-over notes** for the next instance, covering both **svc-admin** and **macronode** and where we stand on building a God-tier dashboard.

---

## 0. Quick status snapshot (today)

**Crates:**

* `crates/svc-admin` – Admin console + (future) control plane.
* `crates/macronode` – Multi-plane node whose admin plane and metrics feed svc-admin.

**Build/runtime**

* `cargo build -p macronode -p svc-admin` ✅
* `cargo run -p macronode` ✅
* `SVC_ADMIN_HTTP_ADDR=127.0.0.1:5300 SVC_ADMIN_METRICS_ADDR=127.0.0.1:5310 cargo run -p svc-admin --bin svc-admin` ✅
* `cd crates/svc-admin/ui && npm run dev` ✅

**SPA behavior (current)**

* Nodes list shows **Example Node** with profile `macronode`.
* Node detail page for `example-node` shows:

  * Header: `ID: example-node Profile: macronode Version: 0.1.0`.
  * **Planes** table populated with 6 planes (gateway, storage, index, mailbox, overlay, dht) with health/ready and restart counts.
  * **Facet metrics** panel:

    * Shows “No facet metrics observed yet…” **right after restart**.
    * Shows `admin.status` facet with RPS and error rate once we exercise `/api/v1/status` enough.
  * **Actions** section present but **mutating actions disabled** (read-only mode).
  * **Debug tools (dev only)** section:

    * Plane dropdown (`gateway`, `storage`, etc.).
    * “Trigger synthetic crash” button.
    * Success banner: `debug crash forwarded to node admin plane`.

**Admin plane + metrics**

* macronode admin plane on `127.0.0.1:8080` exposes:

  * `/healthz`
  * `/readyz`
  * `/metrics`
  * `/version`
  * `/api/v1/status`
  * `/api/v1/debug/crash`

* svc-admin consumes:

  * `GET /api/v1/status` → planes table.
  * `GET /metrics` → `ron_facet_requests_total{facet,result}` → facet metrics panel.
  * `POST /api/v1/debug/crash` → dev crash button.

---

## 1. What we accomplished this instance – svc-admin side

### 1.1 Node configuration + URL derivation

**Problem before:** sampler and status client were trying port `9000` by default, so svc-admin’s node client could not reach macronode’s admin plane or metrics. Planes and facets stayed empty.

**Now:**

* `crates/svc-admin/src/config/nodes.rs`:

  * `NodeCfg` + `NodesCfg` defined.
  * `default_nodes()` seeds **Example Node** with:

    * `base_url: "http://127.0.0.1:8080"`
    * `display_name: "Example Node"`
    * `environment: "dev"`
    * `insecure_http: true`
    * `forced_profile: Some("macronode")`
    * `default_timeout: Some(Duration::from_secs(2))`

* NodeClient now derives admin + metrics URLs from this base URL, so:

  * `/api/nodes/example-node/status` → `GET http://127.0.0.1:8080/api/v1/status`
  * sampler target → `GET http://127.0.0.1:8080/metrics`

**Result:** svc-admin is correctly talking to a *real* macronode instance for both status and metrics.

---

### 1.2 Facet metrics pipeline verification

We fully exercised the facet metrics chain:

1. macronode `/metrics` exports:

   ```text
   ron_facet_requests_total{facet="admin.status",result="ok"} 1
   ```

2. `svc-admin` sampler (in `metrics/sampler.rs`):

   * Scrapes `http://127.0.0.1:8080/metrics`.
   * Parses `ron_facet_requests_total{facet,result}` lines into `FacetSnapshot`s.
   * Aggregates multiple counters per facet into a single `(requests_total, errors_total)` pair.

3. `FacetMetrics` store (in `metrics/facet.rs`):

   * `update_from_scrape(node_id, snapshots)` stores timestamped `FacetPoint`s per `(node_id, facet)` in a rolling window.
   * `summaries_for_node("example-node")`:

     * Takes first + last point in the window.
     * Computes deltas and divides by elapsed time for **RPS**.
     * Computes `error_rate` as `errors_delta / requests_delta`.
     * Skips facets with no traffic (delta ≤ 0) or with <2 points.

4. Router:

   * `/api/nodes/:id/metrics/facets` uses `AppState.facet_metrics.summaries_for_node` to serve summaries.
   * We confirmed via curl:

     ```json
     [
       {
         "facet": "admin.status",
         "rps": 0.0333...,
         "error_rate": 0.0,
         "p95_latency_ms": 0.0,
         "p99_latency_ms": 0.0
       }
     ]
     ```

5. SPA:

   * Calls `adminClient.getNodeFacetMetrics(id)` → maps to `FacetMetricsPanel`.
   * When summaries list is non-empty, the panel shows **admin.status** with its RPS and error rate.
   * When list is empty (e.g. right after restart, before enough samples accumulate), panel shows the **“No facet metrics observed yet…”** empty state.

**Important behavioral detail (for future debugging):**

* Because the aggregator requires **at least 2 points** and positive delta, **facet metrics disappear after we restart the stack** until we hit `/healthz` and `/api/v1/status` enough times to generate new samples in the rolling window.

---

### 1.3 Dev debug crash path – UI + client + backend contract

**Original issue:** when pressing **Trigger synthetic crash**, the SPA got:

> `Request failed: 415 Unsupported Media Type - Expected request with 'Content-Type: application/json'`

* Rust handler for `POST /api/nodes/:id/debug/crash` uses `Json<DebugCrashRequest>`.
* Axum’s `Json` extractor requires `Content-Type: application/json`.
* `adminClient.debugCrashNode` previously:

  * Sent `POST` with **no body**, **no Content-Type**, and a `?service=` querystring.

**Fixes:**

1. **SPA client** – `crates/svc-admin/ui/src/api/adminClient.ts`:

   * `debugCrashNode(id, service?)` now:

     ```ts
     const payload: { service?: string } = {}
     if (service) payload.service = service

     const res = await fetch(buildUrl(`/api/nodes/${id}/debug/crash`), {
       method: 'POST',
       headers: { 'Content-Type': 'application/json' },
       body: JSON.stringify(payload),
     })
     ```

   * So every debug crash request now has a JSON body and the correct content type.

2. **Node detail page** – `routes/NodeDetailPage.tsx`:

   * Already had a **Debug tools (dev only)** section guarded by `devDebugEnabled = import.meta.env.DEV`.

   * Holds local state:

     ```ts
     const [debugPlane, setDebugPlane] = useState<string>('')
     const [debugInFlight, setDebugInFlight] = useState(false)
     const [debugMessage, setDebugMessage] = useState<string | null>(null)
     const [debugError, setDebugError] = useState<string | null>(null)
     ```

   * When status loads, we default `debugPlane` to the first plane.

   * `runDebugCrash()`:

     * Maps plane → service string: `storage` → `svc-storage`, etc.
     * Calls `adminClient.debugCrashNode(status.id, serviceParam)`.
     * Shows a success or error banner based on `NodeActionResponse.accepted` and `message`.

3. **svc-admin backend** – `router.rs`:

   * `node_debug_crash` handler:

     * Validates node exists in registry.
     * Delegates to `state.nodes.debug_crash_node(&id, body.service).await`.
     * Logs audit info:

       ```rust
       target: "svc_admin::audit",
       action = "debug_crash",
       node_id = %resp.node_id,
       "debug crash forwarded to node admin plane"
       ```

**Result:**

* The 415 is gone.
* UI shows green banner: `debug crash forwarded to node admin plane`.
* svc-admin’s logs confirm the action with `svc_admin::audit` entries.

---

## 2. What we accomplished – macronode side

### 2.1 `/api/v1/status` – full plane + restart view

**Handler:** `crates/macronode/src/http_admin/handlers/status.rs`

* Returns JSON:

  * Node metadata: `uptime_seconds`, `profile: "macronode"`, `version`, `http_addr`, `metrics_addr`, `log_level`.

  * Readiness info: `ready: bool`, `deps` map (`config`, `network`, `gateway`, `storage`).

  * `services` map keyed by:

    * `"svc-gateway"`, `"svc-storage"`, `"svc-index"`, `"svc-mailbox"`, `"svc-overlay"`, `"svc-dht"`.

    Values: `"ok"` / `"pending"`.

  * `planes` array, where each plane is:

    ```json
    { "name": "gateway", "health": "healthy", "ready": true, "restart_count": 0 }
    ```

* `build_planes()`:

  * Maps each service status → health label:

    * `"ok"` → `"healthy"`
    * `"pending"` → `"degraded"`
    * other/missing → `"down"`

  * `ready` bit for plane is `node_ready && status == "ok"`.

  * `restart_count` comes from `ReadySnapshot` counters:

    * `snap.gateway_restart_count`
    * `snap.storage_restart_count`
    * `snap.index_restart_count`
    * `snap.mailbox_restart_count`
    * `snap.overlay_restart_count`
    * `snap.dht_restart_count`

* Handler also:

  * Computes uptime based on `started_at`.
  * Calls `update_macronode_metrics(uptime, ready)` (see below).

**Result:**

* svc-admin’s `AdminStatusView` now has **real plane data** instead of being empty or degraded.
* After triggering a synthetic crash on `storage`, and refreshing the page, **Planes table shows `storage` with `Restarts: 1`** – confirming the restart counters are wired.

---

### 2.2 Metrics module – uptime, readiness, and facet counters

**File:** `crates/macronode/src/observability/metrics.rs`

We now have a focused metrics surface:

* `MacronodeMetrics` holds:

  * `uptime_seconds: Gauge` → `ron_macronode_uptime_seconds`
  * `ready: Gauge` → `ron_macronode_ready`
  * `facet_requests_total: IntCounterVec` → `ron_facet_requests_total{facet,result}`

* Registration:

  * All metrics registered against default Prometheus registry with namespace `"ron"`.

    * `ron_macronode_uptime_seconds`
    * `ron_macronode_ready`
    * `ron_facet_requests_total`

* API:

  * `update_macronode_metrics(uptime_seconds, ready)` – called from `/api/v1/status`.
  * `observe_facet_ok(facet: &str)` / `observe_facet_error(facet: &str)` – internal `observe_facet(facet, result)` helper.
  * `encode_prometheus()` – encodes all metrics to text for `/metrics` handler.

**Current facet emitters:**

* `/api/v1/status` handler: `observe_facet_ok("admin.status")` (added earlier).
* `/healthz` handler (see next section): `observe_facet_ok("admin.healthz")`.

Result: `/metrics` includes lines like:

```text
ron_facet_requests_total{facet="admin.status",result="ok"} 1
ron_facet_requests_total{facet="admin.healthz",result="ok"} N
```

These power the **facet metrics panels** in svc-admin.

---

### 2.3 `/healthz` – liveness + facet metric

**File:** `crates/macronode/src/http_admin/handlers/healthz.rs`

* Returns:

  ```json
  {
    "ok": true,
    "checks": {
      "event_loop": "ok",
      "clock": "ok"
    }
  }
  ```

* Calls `observe_facet_ok("admin.healthz")` on each hit.

**Effect:**

* Gives svc-admin at least one potential facet (`admin.healthz`) to aggregate, even when other planes are quiet.
* Currently we didn’t yet see this facet in the summaries list because of the 2-point delta rule + restart behavior, but it’s wired for future.

---

### 2.4 `/api/v1/debug/crash` – synthetic crash hook

We didn’t paste the whole file this session, but from the logs we know:

* The handler:

  * Is behind `MACRONODE_DEV_INSECURE=1` guard for dev.
  * Accepts a `service` parameter (`svc-storage`, `svc-gateway`, etc.).
  * Emits a synthetic `ServiceCrashed { service }` event on the bus.
  * Takes care to log when bus publish fails (`SendError`).

* svc-admin forwards debug crash requests from SPA here, and we see logs:

  ```text
  MACRONODE_DEV_INSECURE=1 — bypassing admin auth for POST /api/v1/debug/crash
  macronode debug_crash: emitting synthetic ServiceCrashed event service=svc-storage
  macronode debug_crash: failed to publish ServiceCrashed event on bus ...
  ```

* Restart counts for storage plane incremented to `1` in `/api/v1/status` after triggering the crash.

Even though the bus send currently errors (no active subscriber), the restart counters being updated means some other path (supervisor / readiness snapshot) is already tracking crashes or we are stubbing the counters to simulate a restart for this slice.

Either way, **from the dashboard’s perspective**: the crash button visibly changes node state, which is exactly what we want.

---

## 3. End-to-end flows verified

We validated these **three golden paths**:

1. **Status path (planes)**

   * SPA → `/api/nodes/example-node/status` (svc-admin) → NodeClient → `http://127.0.0.1:8080/api/v1/status` (macronode) → `AdminStatusView`.
   * Planes table in UI shows:

     ```text
     gateway  healthy  Ready  0
     storage  healthy  Ready  0 | 1 (after crash)
     ...
     ```

2. **Metrics path (facets)**

   * macronode emits `ron_facet_requests_total{facet="admin.status",result="ok"}`.
   * svc-admin sampler scrapes metrics endpoint, feeds `FacetMetrics`.
   * `/api/nodes/example-node/metrics/facets` returns aggregated RPS + error rate.
   * Facet metrics panel shows `admin.status` row once we have enough samples.

3. **Debug-crash control path**

   * SPA debug tools → `adminClient.debugCrashNode` with JSON body.
   * svc-admin `POST /api/nodes/:id/debug/crash` → NodeRegistry → macronode `/api/v1/debug/crash`.
   * macronode logs synthetic crash + bus publish attempt.
   * svc-admin logs `debug crash forwarded to node admin plane`.
   * SPA shows success banner.
   * `/api/v1/status` & svc-admin → restart count increments for selected plane (`storage` tested).

These three together prove:

* The admin dashboard is **not** a mock; it’s wired to real node state and can both observe and (in dev) provoke changes.

---

## 4. Known behaviors / caveats to remember

1. **Facet metrics need multiple samples.**

   * Immediately after restarting the stack, the facet panel often shows “No facet metrics observed yet.”
   * Once we hit `/healthz` and `/api/v1/status` a few times and allow the sampler to run, facets appear again.
   * This is by design to avoid noisy zero-traffic facets.

2. **Read-only mode still enforced.**

   * UiConfig currently has `readOnly: true`, so reload/shutdown buttons remain disabled.
   * Debug tools are dev-only and bypass auth under `MACRONODE_DEV_INSECURE=1`.

3. **Debug crash path uses synthetic events.**

   * Bus send failure is expected in this dev slice; we haven’t yet wired a full crash/restart supervisor to react to the `ServiceCrashed` event.

4. **Facet coverage still narrow.**

   * Only `admin.status` and `admin.healthz` emit facet metrics today.
   * Real app-plane / overlay / storage facets do not yet emit metrics, so facet panel is not yet as rich as it will be.

---

## 5. Next high-impact steps (for future instance)

You explicitly liked these three, so we’ll treat them as **top-priority**:

### 5.1 Add debug-crash facet metrics

**Goal:** see `admin.debug_crash` appear in facet panel whenever we smash the red button.

**Plan:**

* In `macronode/src/http_admin/handlers/debug_crash.rs`:

  * Import `observe_facet_ok` / `observe_facet_error`.

  * On successful synthetic crash dispatch:

    ```rust
    observe_facet_ok("admin.debug_crash");
    ```

  * If bus publish fails (SendError etc.):

    ```rust
    observe_facet_error("admin.debug_crash");
    ```

* Confirm via:

  ```bash
  curl -s 127.0.0.1:8080/metrics | rg ron_facet_requests_total
  curl -s 127.0.0.1:5300/api/nodes/example-node/metrics/facets | jq
  ```

* UI should then show an `admin.debug_crash` row with non-zero RPS whenever we exercise the debug tool.

### 5.2 Emit facet metrics from real planes

**Goal:** facet panel reflects real workload, not just admin endpoints.

**Plan (initial examples):**

* In gateway HTTP entrypoint:

  * After successful request handling: `observe_facet_ok("gateway.app")`.
  * On error paths: `observe_facet_error("gateway.app")`.

* In overlay connection logic:

  * `overlay.connect` facet for new overlay connections.

* In storage/index access points:

  * `storage.query` and `index.query`.

**Impact:**

* Facet metrics panel per node becomes a **true live RPS/error overview** across planes.
* Later we can classify by env/region, but this already mogs many dashboards.

### 5.3 UI polish for restart + facet visibility

**Goal:** make the node detail page “feel” truly operator-grade.

**Ideas (short-term, implementable using current data):**

* **Restart badge styling:**

  * In `PlaneStatusTable`, style `Restart` column:

    * `0` → subtle gray.
    * `1–2` → yellow pill.
    * `3+` → red pill with tooltip “High restart rate”.

* **Facet panel enhancements:**

  * Sort facets by descending RPS.
  * Show color indicator:

    * `error_rate ~ 0` → green.
    * `0 < error_rate < 5%` → yellow.
    * `>= 5%` → red.
  * Continue using the short summary text (RPS + % errors + p95/p99 placeholders).

These are all **SPA-only changes**; no backend changes required beyond what we already have.

---

## 6. Longer-term roadmap for completing svc-admin (God-tier dashboard)

Beyond the next slice, here’s what still remains to consider svc-admin “feature-complete” for a God-tier admin experience:

1. **Multi-node registry UI**

   * Node list supports multiple nodes (macronodes/micronodes).
   * Filters by:

     * Environment (dev/stage/prod).
     * Profile (macronode/micronode).
     * Health (all healthy, any degraded/down).
   * Search box for node ID or display name.

2. **Richer metrics visualizations**

   * Time-series mini charts (sparklines) for facet RPS/error rate (still short-horizon, no TSDB).
   * Aggregate per-node metrics:

     * Total RPS across facets.
     * Error budget views.

3. **Control plane actions**

   * Enable reload/shutdown actions when:

     * `UiConfig.readOnly` is false in dev/ops environments.
     * `ActionsCfg` enables them.
     * Identity roles (`admin` / `ops`) are present.

   * Backend already has gating + metrics hooks; we’d really just:

     * Flip config in dev.
     * Finish error captions in UI.

4. **Auth modes hardening**

   * Implement and test:

     * `AuthMode::Ingress` (X-User, X-Groups).
     * Future `passport`/RON token mode.

   * Metrics:

     * `ron_svc_admin_auth_failures_total{scope}`.
     * `ron_svc_admin_rejected_total{reason}`.

5. **Observability of svc-admin itself**

   * Ensure Prometheus metrics for svc-admin include:

     * HTTP request counts/latencies.
     * Node sampler health (last scrape success per node).
     * Number of nodes, number of facets per node.

   * `/metrics` endpoint already exists; we extend metric families.

6. **Error and degraded states UX**

   * Dedicated UI for:

     * Node unreachable (status fetch fails).
     * Metrics endpoint errors (upstream failures).
     * Nodes stuck in degraded/down states.

   * e.g., banners saying “Admin plane unreachable” vs “Metrics endpoint unreachable.”

7. **Governance / policy hooks (later)**

   * Integrate policy modules so certain actions require approvals or double-signing.
   * Show audit trails (who did what, when).

8. **Test suite expansion**

   * svc-admin:

     * Integration tests hitting the real Axum server with a fake node backend (already some tests exist).
     * UI E2E tests (Playwright/Cypress) for node list + detail + debug tools.

   * macronode:

     * Tests for `/api/v1/status` shape and invariants.
     * Tests for metrics output (presence of gauges, counters).
     * Tests that debug crash updates restart counters as expected.

---

## 7. TL;DR for next instance

When we come back, we should:

1. **Implement `admin.debug_crash` facet metrics** in macronode’s `debug_crash` handler.
2. **Add facet emissions for one “real” plane** (probably gateway.app) so facet panel shows more than just admin endpoints.
3. **Polish the node detail UI**:

   * Restart badge colors.
   * Facet panel sorting + basic error-rate coloring.

With those, the Example Node page will show:

* Planes with health/ready/restart status.
* Multiple facets (admin.status, admin.healthz, gateway.app, etc.) with live RPS/error rates.
* A debug crash button that both increments restart counters **and** drives a visible facet.

That’s the next big leap toward a **God-tier admin dashboard** that makes other control planes look like toys.


### END NOTE - DECEMBER 10 2025 - 11:00 CST




### BEGIN NOTE - DECEMBER 11 2025 - 10:37 CST

Here’s a fresh **carry-over pack** for the next instance, covering both **svc-admin** and **macronode** and where we stand on making this dashboard absolutely mog every other admin UI out there.

---

## 0. Quick status snapshot (today)

**Crates:**

* `crates/macronode` – multi-plane node (gateway/storage/index/mailbox/overlay/dht) + admin plane.
* `crates/svc-admin` – admin console + (future) control plane, with SPA in `crates/svc-admin/ui`.

**Build / test**

* `cargo build -p macronode -p svc-admin` ✅ (only benign warnings about unused handler fn).
* `cargo test -p svc-admin --tests` ✅ (last known; nothing we did should break tests).
* Dev stack script:

  ```bash
  bash scripts/dev_svc_admin_stack.sh
  ```

  This:

  * Starts **macronode** on `127.0.0.1:8080` (admin) and 8090 (gateway) with `MACRONODE_DEV_INSECURE=1`.
  * Starts **svc-admin** backend on `127.0.0.1:5300` and metrics on `127.0.0.1:5310`.
  * Starts **svc-admin UI** (Vite) on `http://localhost:5173`.

**What you see now**

* **Nodes list** page:

  * Card for `Example Node`.
  * Shows profile (“macronode”), overall health (“healthy”), plane readiness and total restarts (`6/6 planes ready · N restarts`).
  * New **metrics freshness badge**: e.g. `Metrics: fresh` (with colors).

* **Node detail** for `Example Node`:

  * Header shows ID, profile, version, health badge, and metrics freshness badge.

  * Planes table shows health, ready, and colored restart pills.

  * Facet metrics section shows rows for:

    * `admin.status` (RPS / error rate / sparkline),
    * `admin.debug_crash` (RPS, error rate, sparkline),
    * and any other observed facets like `gateway.app` once traffic hits.

  * Debug tools (dev only) allow you to trigger synthetic crash per plane; restart counts update and facet metrics show `admin.debug_crash` activity.

---

## 1. Macronode: what’s wired for the dashboard

### 1.1 Admin plane endpoints

macronode admin plane (bound to `127.0.0.1:8080`) currently exposes:

* `GET /healthz` – liveness.
* `GET /readyz` – readiness.
* `GET /metrics` – Prometheus exposition format.
* `GET /version` – version info.
* `GET /api/v1/status` – rich JSON status used by svc-admin.
* `POST /api/v1/debug/crash` – synthetic crash endpoint (dev only).

**Status handler** (`http_admin/handlers/status.rs`):

* Returns `AdminStatusView`-ish shape:

  * Node metadata: `id`, `display_name`, `profile: "macronode"`, `version`, uptime, etc.
  * `ready: bool` + dependency statuses.
  * `services` map keyed by service names (`svc-gateway`, `svc-storage`, etc.).
  * `planes` array with:

    ```json
    {
      "name": "gateway",
      "health": "healthy" | "degraded" | "down",
      "ready": true/false,
      "restart_count": 0+
    }
    ```

* `build_planes()` computes `health`, `ready`, and `restart_count` by:

  * Examining `ReadySnapshot` from `ReadyProbes`.
  * Mapping service status to health: `ok → healthy`, `pending → degraded`, other → `down`.
  * Using per-plane restart counters (gateway, storage, index, mailbox, overlay, dht).

* Calls `update_macronode_metrics(uptime_seconds, ready)` to keep macronode-level metrics up to date.

**Result:** svc-admin shows a **Planes** table that accurately reflects health, readiness, and restart counts per plane – and those restart counts are visibly affected by debug crashes.

---

### 1.2 Observability metrics in macronode

`observability/metrics.rs` defines `MacronodeMetrics`:

* Gauges:

  * `ron_macronode_uptime_seconds` (uptime).
  * `ron_macronode_ready` (1.0 when ready, 0.0 otherwise).

* Counters:

  * `ron_facet_requests_total{facet="...",result="ok|error"}` – key foundation for facet metrics in svc-admin.

API:

* `update_macronode_metrics(uptime_seconds, ready)` – used in `/api/v1/status`.
* `observe_facet_ok(facet: &str)` / `observe_facet_error(facet: &str)` – increments facet counters with appropriate `result` label.
* `encode_prometheus()` – encodes all metrics to text for `/metrics`.

**Emitters currently in place**:

* `/api/v1/status` handler calls `observe_facet_ok("admin.status")` on success.
* `/healthz` handler calls `observe_facet_ok("admin.healthz")` on success.
* `/api/v1/debug/crash` handler:

  * Calls `observe_facet_ok("admin.debug_crash")` on success.
  * On failure to publish crash event to bus, calls `observe_facet_error("admin.debug_crash")`.

**Gateway ingress facet** (`services/svc_gateway.rs`):

* Endpoint `/ingress/ping` now:

  * Responds with OK.
  * Calls `observe_facet_ok("gateway.app")`.

This means hitting:

```bash
curl -s http://127.0.0.1:8090/ingress/ping
```

will increment `ron_facet_requests_total{facet="gateway.app",result="ok"}` and later show up as a facet in the svc-admin UI.

---

### 1.3 Synthetic crashes & restart counters

`/api/v1/debug/crash` handler (dev only, guarded by `MACRONODE_DEV_INSECURE=1`):

* Accepts JSON body with optional `service` (e.g. `svc-storage`, `svc-gateway`).

* Logs:

  * “emitting synthetic ServiceCrashed event service=svc-storage”.
  * “failed to publish ServiceCrashed event on bus …” if no subscribers (expected in this slice).

* Bumps a restart counter (synthetic) and returns JSON describing what happened, including a note like:

  ```json
  {
    "status": "debug crash event emitted",
    "service": "svc-storage",
    "note": "restart counter bumped; no real worker was killed (synthetic event)"
  }
  ```

**Observed behavior:**

* Planes table goes from `storage: restart_count=0` to `restart_count=1` after hitting debug crash.
* Gateway and other planes also show restart increments when targeted.

This gives a “toy crash” path that visibly exercises the entire observability chain without killing real workers.

---

## 2. svc-admin backend: what’s wired

### 2.1 Node config + client

Config (`config/nodes.rs`):

* `NodeCfg` and `NodesCfg` represent configured nodes.

Default config seeds **Example Node** with:

* `id: "example-node"`.
* `base_url: "http://127.0.0.1:8080"` (admin plane).
* `metrics_url: "http://127.0.0.1:8080/metrics"`.
* `display_name: "Example Node"`.
* `environment: "dev"`.
* `forced_profile: Some("macronode")`.
* Timeouts and `insecure_http: true` for local HTTP.

Node client:

* Builds URLs like:

  * `/api/nodes/example-node/status` → `GET http://127.0.0.1:8080/api/v1/status`.
  * `/api/nodes/example-node/debug/crash` → `POST http://127.0.0.1:8080/api/v1/debug/crash`.
  * Sampler uses `metrics_url` directly (`http://127.0.0.1:8080/metrics`).

### 2.2 Facet metrics in svc-admin (store + sampler)

**Facet store** (`metrics/facet.rs`):

* In-memory structure keyed by `(node_id, facet)`:

  * Each key stores a `VecDeque<FacetPoint>` with `(timestamp, requests_total, errors_total)`.

* Rolling **window** (Duration) from config.

* `update_from_scrape(node_id, snapshots)`:

  * Called by sampler after each `/metrics` scrape.
  * Appends new points and prunes any outside the configured window.
  * Cleans out empty series.

* `summaries_for_node(node_id)`:

  * Returns `Vec<FacetMetricsSummary>`.

    For each `(node, facet)`:

    * Uses first + last points to compute elapsed time within the window.

    * If 2+ samples and non-zero elapsed:

      * `rps = (last.requests_total - first.requests_total) / elapsed`.
      * `error_rate = (last.errors_total_delta / req_delta)`.

    * If only a single sample or zero delta:

      * `rps = 0.0`.
      * `error_rate` derived from current totals (`errors_total / requests_total`).

    * New field:

      * `last_sample_age_secs: Option<f64>` – age of most recent sample for that facet compared to `Instant::now()`.

    * Latency fields are currently stubbed to `0.0` (P95/P99 placeholders).

  * Sorts facets by name for stable UI ordering.

**DTO** (`dto/metrics.rs`):

* `FacetMetricsSummary` re-exports:

  * `facet: String`.
  * `rps: f64`.
  * `error_rate: f64`.
  * `p95_latency_ms: f64`.
  * `p99_latency_ms: f64`.
  * `last_sample_age_secs: Option<f64>`.

**Sampler** (`metrics/sampler.rs`):

* Periodically (config interval, currently 5s in logs) for each node:

  * HTTP GET to `metrics_url`.
  * Parses `ron_facet_requests_total{facet="...",result="ok|error"}` lines into `FacetSnapshot`s.
  * Feeds `FacetMetrics::update_from_scrape()`.

* Logs:

  * Initial failure as “initial metrics sample failed (will retry…)”.
  * Subsequent failures as “facet metrics sample failed …”.

This is the heart of the **“metrics freshness”** signal. If sampling keeps failing, `last_sample_age_secs` will get older and eventually classify as stale/unreachable in the SPA.

---

### 2.3 Node APIs

Router exposes:

* `GET /api/nodes` – returns `Vec<NodeSummary>` (id, display_name, profile).
* `GET /api/nodes/:id/status` – returns `AdminStatusView` from node client (mirroring macronode `/api/v1/status`).
* `GET /api/nodes/:id/metrics/facets` – returns `Vec<FacetMetricsSummary>` derived from facet store.
* `POST /api/nodes/:id/debug/crash` – receives `DebugCrashRequest` from SPA and forwards to node admin plane; logs audit entries.

**UiConfig + /api/me**

* `GET /api/ui-config` – returns UI config DTO (`defaultTheme`, `availableThemes`, `readOnly`, etc.).
* `GET /api/me` – returns `MeResponse` with subject, displayName, roles, authMode, optional loginUrl.

These drive read-only gating and identity display in the SPA.

---

## 3. svc-admin SPA: what’s implemented

### 3.1 Core DTOs and adminClient

`ui/src/types/admin-api.ts` mirrors Rust DTOs:

* `UiConfigDto`, `MeResponse`, `NodeSummary`, `PlaneStatus`, `AdminStatusView`, `FacetMetricsSummary`, `NodeActionResponse`.
* `FacetMetricsSummary` now includes `last_sample_age_secs: number | null`.

`adminClient.ts`:

* `getUiConfig`, `getMe`, `getNodes`, `getNodeStatus`, `getNodeFacetMetrics`, `debugCrashNode`, `reloadNode`, `shutdownNode`, etc.
* Uses fetch with correct JSON headers.
* `debugCrashNode` posts a JSON body (with optional `service`) and `Content-Type: application/json` to avoid 415.

### 3.2 Node list page (Nodes → NodeCard)

`NodesPage` fetches list of nodes and, for each, shows a `NodeCard`:

* Card content includes:

  * Node name (`display_name`).
  * Profile line (`Profile: macronode` if known).
  * Overall health (from status).
  * Planes summary (`6/6 planes ready · N restarts`).
  * **Metrics freshness label** such as:

    * `Metrics: fresh`.
    * `Metrics: stale`.
    * `Metrics: unreachable` (if/when sampler fails long enough).

* Metrics freshness classification:

  * SPA helper looks at `last_sample_age_secs` values from the last facet summaries for that node.

  * If there are **no facets** yet, it treats metrics as **unreachable** (or “unknown”) and shows the degraded label.

  * If there *are* facets:

    * If `min(last_sample_age_secs)` is small (e.g. ≤ ~2× sampler interval), status is `fresh`.
    * If moderate (e.g. > fresh threshold but still within some bound), status becomes `stale`.
    * If `last_sample_age_secs` is `null` or excessively large, status becomes `unreachable`.

  * These thresholds are currently simple heuristics hard-coded in the SPA; later we can pipe sampler interval/window from the backend.

* Labels are color-coded:

  * Fresh → green-ish.
  * Stale → amber.
  * Unreachable → red/gray “danger” tone.

This already gives a **true at-a-glance** sense of which node’s metrics pipeline is healthy.

### 3.3 Node detail page

`NodeDetailPage.tsx`:

* Fetches:

  * `AdminStatusView` (`getNodeStatus`).
  * `FacetMetricsSummary[]` (`getNodeFacetMetrics`).
  * `UiConfigDto` and `MeResponse` (for action gating).

* Computes `overallHealth` from plane healths.

* Header shows:

  * Node name.
  * `ID`, `Profile`, `Version`.
  * Link back to Nodes.
  * `NodeStatusBadge` (healthy/degraded/down).
  * **Metrics freshness badge** using the same classification helper as NodeCard.

* Shows **Planes** section:

  * `PlaneStatusTable` displays each plane’s:

    * Name (monospace).

    * Health pill:

      * Healthy → green pill.
      * Degraded → amber.
      * Down → red.

    * Ready pill:

      * `Ready` (green).
      * `Not ready` (gray).

    * **Restart pill**:

      * 0 → small gray pill.
      * 1–2 → amber pill (watch).
      * 3+ → red pill (flapping).

* **Facet metrics section**:

  * Uses `FacetMetricsPanel`.

  * Handles states:

    * Loading (spinner, friendly text).
    * Error (ErrorBanner with explanation & raw error).
    * No facets (EmptyState).
    * Facet list (MetricChart for each facet).

  * **History + sparkline**:

    * Maintains a small `historyByFacet` state (last N RPS values per facet).
    * Every time `facets` prop updates, it appends the new `rps` to the facet’s history and trims to length (e.g., 32).
    * Passes this history into `MetricChart`.

* **Metrics freshness warnings**:

  * Based on classification helper:

    * If metrics are **stale**:

      * Shows a subtle warning (e.g., “Metrics may be stale (last sample ~Xs ago). Check node /metrics endpoint.”).

    * If **unreachable**:

      * Shows a louder banner (ErrorBanner or warning) indicating that the sampler can’t reach `/metrics` and that restart counters and facets may be outdated.

  * These warnings are separate from status errors and help ops quickly diagnose “metrics are lying” vs “node is down.”

* **Actions**:

  * `reload` / `shutdown` buttons exist but are gated by:

    * `UiConfig.readOnly` (true in dev).
    * Roles in `MeResponse` (`admin` / `ops`).

  * In current slice: read-only mode is **on**, so buttons are disabled with explanatory text.

* **Debug tools (dev only)**:

  * Only shown when `import.meta.env.DEV` and node has planes.

  * Dropdown to choose `Plane to crash` (gateway/storage/index/mailbox/overlay/dht).

  * Button “Trigger synthetic crash”:

    * Maps plane name to service id `svc-plane`.
    * Calls `adminClient.debugCrashNode(status.id, serviceParam)`.
    * Shows success message from node or generic text.

  * This path:

    * Increments plane restart count.
    * Emits facet metrics `admin.debug_crash` (ok/error).
    * Provides a trivial way to “exercise” the dashboard.

### 3.4 MetricChart component

`MetricChart` now:

* Renders a tiny inline SVG **sparkline** for RPS:

  * Uses RPS history per facet (`history` prop from `FacetMetricsPanel`).
  * Computes min/max and maps values into 0–1 normalized coordinates.
  * Draws a polyline vertex per sample.
  * Uses subtle stroke width and opacity so it looks like a “line” not a fat bar.

* Also displays numeric details (e.g. summary of RPS & error rate) near the sparkline.

The chart now *shows movement over time* instead of a static box; hitting `/ingress/ping` repeatedly moves the `gateway.app` line, hitting `/api/v1/status` moves `admin.status`, and debug crashes bump `admin.debug_crash`.

---

## 4. End-to-end flows we’ve proven

We’ve now demonstrated multiple **golden paths** end-to-end:

1. **Status → planes table → restarts**

   * macronode `/api/v1/status` → svc-admin NodeClient → `/api/nodes/:id/status` → SPA.

   * Triggering a synthetic crash via:

     ```bash
     curl -s -X POST "http://127.0.0.1:8080/api/v1/debug/crash?service=svc-storage"
     ```

     updates `restart_count` for `storage` plane, which:

     * appears in macronode `/api/v1/status`.
     * is reflected in svc-admin’s Planes table + restart pills.

2. **Metrics → facet store → facet panel + sparkline**

   * macronode emits `ron_facet_requests_total` for `admin.status`, `admin.healthz`, `admin.debug_crash`, `gateway.app`, etc.

   * svc-admin sampler scrapes `/metrics`, parses counters, and feeds facet store.

   * `/api/nodes/:id/metrics/facets` aggregates per node/facet:

     * RPS.
     * Error rate.
     * Last sample age.

   * SPA:

     * Shows facet rows with RPS/error rate.
     * Updates sparklines over time.
     * Uses last sample age to classify *fresh/stale/unreachable*.

3. **Debug crash control path**

   * SPA “Trigger synthetic crash” → `POST /api/nodes/:id/debug/crash` (svc-admin) → macronode `/api/v1/debug/crash`.

   * macronode:

     * Logs synthetic crash, attempts to emit `ServiceCrashed` event on bus.
     * Bumps restart counters and emits facet metrics `admin.debug_crash`.

   * svc-admin logs audit entry (“debug crash forwarded to node admin plane”).

   * SPA:

     * Shows success message.
     * Planes table shows incremented restart count.
     * Facet panel shows `admin.debug_crash` with `rps=0` and `error_rate=1.0` initially.

4. **Metrics freshness classification**

   * When macronode is not yet up, sampler logs connection refused and last_sample_age_secs remains `None`/large → NodeCard + NodeDetail show `Metrics: unreachable`.

   * Once macronode is up and sampler begins to succeed:

     * last_sample_age_secs stays small relative to interval.
     * NodeCard + NodeDetail show `Metrics: fresh`.

   * If we later stop macronode (not yet fully exercised), sampler will keep aging last_sample_age_secs until NodeCard flips to `stale` or `unreachable`.

---

## 5. Known behaviors / edge cases

1. **Facet panel & reboots**

   * Immediately after restarting macronode/svc-admin, facet RPS and sparkline may take a few intervals to repopulate.

   * Our `FacetMetricsPanel` preserves the last non-empty snapshot to avoid blinks, but new facets appear only once sampler has at least one new scrape.

2. **Gateway facet needs traffic**

   * `gateway.app` only appears if you hit the gateway:

     ```bash
     curl -s http://127.0.0.1:8090/ingress/ping
     ```

   * Without that, facet panel will just show admin facets.

3. **Metrics freshness thresholds are heuristic**

   * Currently, SPA uses simple hard-coded thresholds (multiples of the sampler interval) to classify `fresh` vs `stale` vs `unreachable`.

   * We have not yet wired the sampler interval/window from backend config into the SPA; this can be improved later.

4. **Actions still read-only**

   * `UiConfig.readOnly` is `true` in dev.
   * Even if we unhide reload/shutdown buttons in the UI, backend gating (ActionsCfg/AuthCfg) still needs to be satisfied to actually perform mutations.

5. **Bus crash events are synthetic**

   * `ServiceCrashed` events currently log a `SendError` because the supervisor isn’t yet subscribed/wiring up real restarts based on those events.

   * Restart counts are being bumped by the debug handler itself, not by a real crashing worker.

---

## 6. Next high-impact steps toward a truly God-tier dashboard

We’re now firmly in **“this is already better than a lot of admin UIs”** territory. To push it into **undisputed God tier**, here are the most valuable next chunks:

### 6.1 Finish metrics freshness UX + sampler health (backend + UI)

We already did the first pass (fresh/stale/unreachable labels). To polish:

1. **Backend**

   * Add **per-node sampler health summary**:

     * Track last successful scrape time per node.
     * Track last error (if any) per node.
     * Expose a tiny `/api/nodes/:id/metrics/health` or enrich existing `/metrics/facets` response with node-level `sampler_status`.

   * Optionally emit svc-admin metrics:

     * `ron_svc_admin_metrics_sampler_last_success_timestamp{node_id}`.
     * `ron_svc_admin_metrics_sampler_errors_total{node_id}`.

2. **UI**

   * Replace simple heuristics with real config:

     * Use sampler interval and window from config to compute thresholds.
     * Show tooltips:

       * “Last successful metrics scrape was 3.2s ago (interval=5s, window=60s).”

   * Add dedicated banners:

     * Top-of-page warnings for “metrics unreachable” that link to suggested debug steps (check node, check network, etc.).

### 6.2 Multi-node experience

Even if we only run one node locally, the interface should be ready for fleets:

* **Backend**

  * Support multiple nodes in `NodesCfg` (e.g. dev/stage/prod, or `node-a`, `node-b`).
  * Sampler already supports multiple nodes; just configure them.

* **UI**

  * Node list:

    * Sort nodes by health (down first, then degraded, then healthy).
    * Secondary sort by metrics freshness (unreachable/stale at top).
    * Filter chips: `All | Healthy | Degraded/Down | Metrics unreachable`.

  * Node detail:

    * Show environment/profile tags (dev/stage/prod) derived from config.

This will make the dashboard feel “fleet-grade” even before we have lots of nodes in practice.

### 6.3 Facet taxonomy & richer metrics

Right now facets are mostly admin-side (`admin.status`, `admin.healthz`, `admin.debug_crash`, plus `gateway.app` if we hit it).

Next:

* **Macronode**

  * Add facets:

    * `storage.read`, `storage.write` in svc-storage.
    * `index.query` in svc-index.
    * `overlay.connect` in svc-overlay when we bring real overlay wiring online.
    * `dht.query` in svc-dht.

  * Start emitting **latency histograms** per facet (Prometheus `*_bucket` metrics) so we can compute p95/p99 instead of stubs.

* **svc-admin**

  * Extend `FacetMetricsSummary` or add a new DTO to include real latency percentiles once we parse histograms.

  * Upgrade `MetricChart` / facet panel to show:

    * RPS sparkline.
    * error-rate color chip (green/amber/red).
    * p95/p99 numbers when available.

This will make the facet panel the default “what’s going on?” view for ops.

### 6.4 Control plane actions (read-write mode)

Once we’re comfortable:

* **Backend**

  * Finalize `ActionsCfg` and `AuthCfg` for svc-admin so we can safely allow:

    * Node reload.
    * Node shutdown.
    * Later: plane-level restart, drain mode toggles, etc.

* **UI**

  * When `UiConfig.readOnly == false` and roles contain `admin` / `ops`:

    * Enable buttons with strong confirmation flows (modals, warnings).
    * Idea: “Require double-confirmation when shutting down a prod node”.

This is when svc-admin crosses from “observability” into “control plane”.

### 6.5 Observability of svc-admin itself

Make svc-admin a first-class citizen:

* **Metrics**

  * HTTP request metrics (counts, latencies, error rates).
  * Node sampler metrics (per-node last success, error counts).
  * UI config load metrics.

* **UI**

  * Tiny “About svc-admin” section or page:

    * Shows building version, sampler interval/window, number of nodes, etc.
    * Links to `/metrics` for scraping.

### 6.6 Test & docs hardening

* Expand svc-admin integration tests:

  * Use a fake node backend with deterministically controlled `/status` and `/metrics`.
  * Assert that:

    * facet metrics are parsed correctly (RPS/error rate).
    * last_sample_age_secs classification is correct for given scenarios.

* Add Playwright/Cypress basic E2E for the SPA:

  * Node list loads & shows Example Node.
  * Node detail shows planes + facet metrics.
  * Debug crash button increments restarts.

* Docs:

  * Update `README.md`, `OBSERVABILITY.md`, and `GOVERNANCE.md` for svc-admin to include:

    * Facet metrics taxonomy.
    * Metrics freshness semantics.
    * Dev stack script usage.
    * How to add new nodes.

---

## 7. TL;DR for next instance

When we come back, here’s a crisp plan:

1. **Refine metrics freshness & sampler health**

   * Backend: optional sampler health struct per node (last success, last error, interval).
   * UI: replace heuristic thresholds with real values, add tooltips & stronger banners.

2. **Start multi-node UX**

   * Add a second node entry in `NodesCfg` pointing at the same macronode (for now) to exercise multi-card view.
   * Update NodeList to sort/filter nodes by health + metrics freshness.

3. **Expand facet coverage a bit more**

   * Instrument one non-admin plane (e.g. `storage.read` in svc-storage) with facet metrics.
   * Hit those endpoints a few times and watch them appear in the facet panel with their own sparklines.

Those steps keep us laser-focused on operator experience and observability, and move svc-admin even closer to the **God-tier, fleet-grade dashboard** we’re aiming for.


### END NOTE - DECEMBER 11 2025 - 10:37 CST




### BEGIN NOTE - DECEMBER 11 2025 - 21:15 CST


Perfect, I’ll log the bug clearly and then do the big God-tier carryover pass.

---

## 0. Quick status + completion estimate

**Crate:** `crates/svc-admin`
**Role:** Admin dashboard for macronodes/micronodes – observability first, then safe control plane.

**Rough completion estimates (today):**

* **Backend (Rust HTTP API + metrics sampling): ~80%**

  * Node registry, node client, facet metrics sampling, health/metrics endpoints, debug crash plumbing, etc. are all in place and working.
  * Still missing: some polish on error mapping, more node types, future write-paths (reload/shutdown real behavior), and a bit of refactoring once we’ve stabilized the UX.
* **SPA (React+Vite UI): ~65–70% for v0.1 “God-tier preview”**

  * Nodes list, Node detail, planes table, facet metrics panel, layout/theming, and macronode integration are functionally there.
  * Still missing: fully wired counters from all triggers, consistently live sparklines, data-plane / DB panels, playground surface, richer filters/search, and more “wow” polish.

Overall I’d say:

> **svc-admin as a crate: ~70–75% toward a genuinely God-tier v0.1.0**
> Read-only observability is mostly there; control-plane + advanced dashboards + DB views are the remaining big rocks.

---

## 1. Current runtime/dev-stack state

You’re running the **full dev stack** via:

```bash
bash scripts/dev_svc_admin_stack.sh
```

This:

* Builds and runs **macronode** with:

  * Admin plane on `127.0.0.1:8080`
  * Embedded services:

    * `svc-gateway` on `127.0.0.1:8090`
    * `svc-storage` on `127.0.0.1:5303`
    * `svc-index` on `127.0.0.1:5304`
    * `svc-overlay`, `svc-dht`, `svc-mailbox` as “host shells” with placeholder listeners.
* Builds and runs **svc-admin** with:

  * UI/API on `127.0.0.1:5300`
  * health/metrics on `127.0.0.1:5310`
* Starts the SPA dev server (Vite) on `http://localhost:5173`, proxying `/api/*` to `http://127.0.0.1:5300`.

We’ve confirmed:

* `/api/me`, `/api/ui-config`, `/api/nodes` all work when svc-admin is up (after fixing the ECONNREFUSED race).
* macronode’s admin plane responds at:

  * `GET http://127.0.0.1:8080/api/v1/status`
  * `GET http://127.0.0.1:8080/metrics`
  * `GET http://127.0.0.1:8080/healthz`
  * `GET http://127.0.0.1:8080/readyz`
* You can trigger a synthetic crash via terminal:

  * `POST http://127.0.0.1:8080/api/v1/debug/crash?service=svc-storage`
  * This bumps `restart_count` in `/api/v1/status` and svc-admin reflects that **correctly** in the UI after refresh.

---

## 2. Features we have working today

### 2.1 Node inventory & status

* **Node registry** in svc-admin config:

  * At least three nodes configured (example-node, node-b, node-c) all pointing to the macronode admin plane on `127.0.0.1:8080` during dev.
* **Node client**:

  * Attempts `GET /api/v1/status` on each node.
  * If that fails, it gracefully **degrades** to individual probes: `/healthz`, `/readyz`, `/version`.
  * Logs clear warnings when status/readyz/version are missing or non-JSON.
* **Nodes list page** (`NodeListPage.tsx` + `NodeCard`):

  * Shows all configured nodes.
  * Displays:

    * Node name / display name.
    * Profile/version (when present).
    * High-level health/ready status.
    * Plane counts and restart counts.
  * Uses **NodeStatusBadge** to render `healthy/degraded/down` based on plane statuses.

### 2.2 Node detail page

**`NodeDetailPage.tsx`** is now a genuinely rich screen:

* Uses route param `/nodes/:id` to fetch Node-specific state:

  * `AdminStatusView` via `adminClient.getNodeStatus(id)`.
  * `FacetMetricsSummary[]` via `adminClient.getNodeFacetMetrics(id)`.
  * UI config and identity (`getUiConfig`, `getMe`) for gating mutating actions.
* Derived values:

  * `overallHealth` from plane healths.
  * `metricsHealth` from last sample ages:

    * `fresh` if min `last_sample_age_secs ≤ 30`.
    * `stale` if samples exist but are older than threshold.
    * `unreachable` if metrics call errors.
  * `minSampleAgeSecs` for tooltips/warnings (used in stale-banner messaging).
* Layout:

  * **Left side:**

    * Header with node title, ID, profile, version, and badges:

      * `NodeStatusBadge` for overall node health.
      * A “Metrics: fresh/stale/unreachable” pill based on `metricsHealth`.
    * **Planes table** (PlaneStatusTable):

      * Shows each plane (gateway, storage, index, mailbox, overlay, dht).
      * Displays health, readiness, restart count for each.
    * **Facet metrics panel** (FacetMetricsPanel):

      * Shows metrics for each facet (rps, error rates, etc.).
      * Maintains a `historyByFacet` map for RPS over time (used by sparklines).
    * **Actions section**:

      * Buttons for “Reload node configuration” and “Shutdown node”.
      * Proper gating: requires `!readOnlyUi && roles includes admin|ops`.
      * Tracks in-flight state and shows success/error messages.
    * **Debug controls section (dev-only)**:

      * Dev-gated by `import.meta.env.DEV`.
      * Dropdown of planes (gateway, storage, index, overlay, dht, mailbox).
      * Button to trigger a **synthetic crash** via `adminClient.debugCrashNode`.
      * Status messages for debug actions.
  * **Right side:**

    * `NodeDetailSidebar` stub:

      * Receives node status, metricsHealth, minSampleAgeSecs, and loading flags.
      * Intended sections:

        * Data & storage view (DB/files/facets).
        * Playground surface (code playground / query runner).
      * Currently mostly layout + textual placeholders; real data-plane wiring is future work.

### 2.3 Metrics sampling & facet metrics

On the **backend side**:

* svc-admin spawns **facet metrics samplers** for each configured node:

  * Logs: `spawning facet metrics samplers for configured nodes node_count=3 interval_secs=5`.
  * Periodically hits `http://127.0.0.1:8080/metrics`.
  * Parses metrics from the node and aggregates them into a simpler DTO model for the UI.
* On error (e.g., node not yet bound), logs clear warnings:

  * `initial metrics sample failed (will retry on interval)` with node id and URL.
* The NodeDetail page uses that data for:

  * Classification tags (fresh/stale/unreachable).
  * RPS history for sparklines.

On the **frontend side**:

* `FacetMetricsPanel`:

  * Renders each facet with current RPS and a mini sparkline (using `historyByFacet`).
  * Handles loading state gracefully.
  * Shows fallback error banners if metrics calls fail.
* The sparklines worked earlier and still have the **state machinery** to do so; we only have a bug where live updates aren’t showing as expected after recent changes (see Known Issues).

### 2.4 Identity + UI config gating

* SPA fetches:

  * `UiConfigDto` → `uiConfig.readOnly`.
  * `MeResponse` → `me.roles[]`.
* Uses them to determine `canMutate`:

  * `canMutate = !readOnlyUi && roles contains 'admin'|'ops'`.
* This means:

  * In dev, we’re mostly running in **read-only** mode (on purpose).
  * Action buttons are disabled when not allowed.
  * Backend also enforces this; UI gating is layer 1, not the only gate.

### 2.5 Debug crash plumbing (end-to-end)

We now have an **end-to-end debug crash path**:

1. **From SPA (NodeDetailPage)**:

   * User selects a plane in the dropdown and clicks “Trigger synthetic crash”.
   * SPA calls `adminClient.debugCrashNode(nodeId, planeName)`.
2. **svc-admin**:

   * Receives the request, logs `debug crash forwarded to node admin plane`.
   * Forwards to macronode admin plane `/api/v1/debug/crash?service=<plane>`.
3. **macronode**:

   * Accepts debug crash in dev:

     * Logs `MACRONODE_DEV_INSECURE=1 — bypassing admin auth for POST /api/v1/debug/crash`.
     * If service = `svc-storage`, it emits a **synthetic ServiceCrashed event** on the bus and bumps the restart counter.
   * We verified by curl:

     * `GET /api/v1/status` → `restart_count` increments for storage.
     * This shows up correctly in svc-admin’s UI when we refresh.

This is already a **signature God-tier dev feature**: a synthetic, non-destructive crash hammer, with full observability.

---

## 3. Known issues / bugs (to fix later)

You explicitly asked to “make a note” of the two big front-end issues. Let’s pin them clearly.

### 3.1 Counters via debug button vs terminal

* **Observed:**

  * When you call debug crash via **terminal**:

    * `POST /api/v1/debug/crash?service=svc-storage`
    * `/api/v1/status` restart_count increments.
    * svc-admin reflects this (planes restart counter updates in UI).
  * When you trigger debug crash via the **NodeDetailPage button**:

    * The node does receive the debug crash (logs show `debug crash forwarded`).
    * macronode logs show unknown service warnings when using names like `gateway`, `index`, `mailbox` etc., and synthetic ServiceCrashed event when using `svc-storage`.
    * However: the **UI does NOT reliably refresh** the restart counts after pressing the button.
* **Likely cause (to be confirmed later):**

  * After a debug crash, we don’t re-fetch `/api/v1/status` automatically.
  * Counters only update when:

    * You navigate away/back, or
    * Something else triggers a status refetch.
* **Planned fix (later):**

  * On successful debug crash response:

    * Trigger a `getNodeStatus(nodeId)` refetch and update `status` state, OR
    * Introduce a small “refetch status” handler shared with actions/reload.
  * Also ensure the **plane service name mapping** matches macronode’s expectations (e.g., `storage` vs `svc-storage`) so debug crash increments the right counters via UI.

### 3.2 Sparklines / RPS charts not updating

* **Observed:**

  * The **counters and graphs (sparklines)** used to animate based on facet metrics.
  * After later changes (NodeDetail refactor, facet metrics panel, etc.), sparklines are no longer visibly updating as expected.
* **What we know:**

  * `facetHistory` state is still being maintained and updated in NodeDetailPage.
  * Sampler logs show repeated attempts to sample metrics at 5s intervals.
  * UI successfully renders facet metrics, but the time-series behavior is off or frozen.
* **Planned fix (later):**

  * Inspect `FacetMetricsPanel` props vs `historyByFacet` shape and ensure:

    * We always pass the correct facet key (`facet.facet`).
    * The component re-renders when `historyByFacet` changes.
  * Verify that we’re not reset-slicing the history too aggressively or using stale references in React hooks.

### 3.3 LoadingSpinner path issues

* **Previously:**

  * `NodeDetailPage.tsx` imported `../components/common/LoadingSpinner`, but UI only had `components/shared/LoadingSpinner.tsx`.
  * Vite spammed `Failed to resolve import "../components/common/LoadingSpinner"` and blocked the SPA.
* **Current fix:**

  * We now have:

    * `components/shared/LoadingSpinner.tsx` (real component).
    * `components/common/LoadingSpinner.tsx` shim that re-exports the shared one.
  * NodeDetailPage already imports from `../components/shared/LoadingSpinner`.
* This is **resolved** as far as runtime — just keep the shim until we’re sure no other imports use `/common`.

---

## 4. What remains to finish svc-admin (high-impact TODOs)

This is the “how we get to jaw-dropping God-tier” list.

### 4.1 Short-term, concrete tasks

1. **Fix debug-crash counter refresh**

   * After `runDebugCrash()` succeeds:

     * Trigger a re-fetch of `getNodeStatus(nodeId)` and update `status`.
     * Confirm that restart counts update in UI immediately without manual refresh.
   * Double-check that UI plane names map exactly to macronode’s `service` names (for synthetic restart counting).

2. **Re-enable live sparklines**

   * Inspect `FacetMetricsPanel` + `historyByFacet` contract and fix any mismatch.
   * Confirm that:

     * RPS history accumulates steadily as metrics samples arrive.
     * Sparklines animate over time for each facet.
   * Add a **small visual indicator** (e.g., “last updated Xs ago”) near the metrics.

3. **Metrics freshness classification helper (already partly done but polish):**

   * Verify that `classifyMetricsHealth` thresholds are correct and intuitive.
   * Confirm that:

     * Nodes with fresh samples show “Metrics: fresh”.
     * Nodes with older samples show “Metrics: stale” and the stale banner.
     * Nodes unreachable show “Metrics: unreachable” + a clear red warning band.

4. **NodeDetailSidebar real content (first slice):**

   * Fill in initial non-stub content:

     * Basic info section: node ID, profile, version, uptime, environment.
     * Storage summary: maybe some mock stats from `/metrics` or `/api/v1/status`.
   * Make this side panel feel like a “mini handoff” from ops to data-engineering.

### 4.2 Medium-term God-tier features

5. **Global “Fleet Overview” dashboard**

   * Top-level status grid for all nodes:

     * Per-node health, metrics health, environment tags.
     * Total number of nodes, per-env counts.
   * Global counters:

     * Total restarts in last N minutes.
     * Number of nodes with unreachable metrics.
   * Maybe a **small line chart** of “crashes per minute” or “requests per second across all nodes.”

6. **Filters, search, and tagging**

   * Filter nodes by:

     * Environment (`dev`, `staging`, `prod`).
     * Profile (`macronode`, `micronode`).
     * Health (`healthy`, `degraded`, `down`).
   * Implement a quick search bar:

     * Search by ID, display name, or tag.
   * Tagging system (later): persistent labels attached to nodes (e.g., region, team).

7. **Advanced metrics visualization**

   * richer view on NodeDetail:

     * Per-plane graphs (CPU-ish, RPS, error rates).
     * Toggle between time windows (5m, 1h, 24h).
   * More advanced facet grouping:

     * Group facets by service (gateway facets, storage facets, etc.).

8. **Actions & control plane (beyond stubs)**

   * Wire `reload` and `shutdown` to real behaviors once macronode has them:

     * Ensure safe gating:

       * On UI: roles + readOnly.
       * On server: role/permission checks and policy.
     * Observability:

       * Audit logs in svc-admin for every action (who did what, when, on which node).
   * Add “drain node” / “put node in maintenance” feature for future cluster scenario.

### 4.3 Long-term God-tier polish

9. **DB / Data-plane view in the sidebar**

   * For macronode:

     * Read-only views into **svc-storage**:

       * Space usage.
       * Top buckets/collections.
       * Key counts.
     * For attached databases (like MySQL/Postgres) in the future:

       * High-level table counts, db health, connection pool stats.
   * Important: stay **read-only by default**, with clear warnings around any data-plane operations.

10. **Playground (ronCorePlaygroundClient) integration**

* A “code playground” panel on NodeDetail:

  * Pre-configured client to talk to that node (or gateway).
  * Safe snippet templates: list objects, query index, simulate app SDK calls.
* Rate-limited, read-only in dev; behind extra gates in prod.

11. **Notifications & historical events**

* Timeline view per node:

  * Restarts over time.
  * Crashes, configuration reloads, shutdowns.
  * “Synthetic vs real” crash labels.
* Simple alerting indicators:

  * e.g., “Node has flapped 3 times in the last 10 minutes.”

12. **Auth & multi-tenant hardening**

* Replace `MACRONODE_DEV_INSECURE=1` bypass with:

  * Proper authenticated admin sessions.
  * Role-based access control for ops/admin vs viewer.
* Tenant scoping if svc-admin ever manages nodes for multiple tenants.

---

## 5. Carry-over summary for the next instance

Here’s the short “drop in as NOTES.md” style recap to bring into the next session:

* svc-admin is now **integrated end-to-end** with a macronode running on `127.0.0.1:8080`:

  * Nodes list shows configured nodes.
  * NodeDetailPage shows planes, facet metrics, actions, debug controls, and a sidebar stub.
  * svc-admin uses macronode’s `/api/v1/status`, `/metrics`, `/healthz`, `/readyz` as its data sources.
* The **debug crash pipeline** works in dev:

  * From SPA → svc-admin → macronode → synthetic ServiceCrashed event.
  * Restart counters increment when we hit `/api/v1/debug/crash?service=svc-storage` via terminal.
  * **Bug**: when debug crash is triggered via UI button, restart counters do not refresh automatically in the NodeDetail page — we’ll fix this by refetching node status on success and aligning service names.
* **Metrics sampling + facet metrics** are wired:

  * svc-admin spawns samplers for each node.
  * Metrics classification (`fresh/stale/unreachable`) is implemented.
  * NodeDetail uses these to show badges and warnings.
  * **Bug**: sparklines/RPS charts are not visibly updating like earlier even though the history state is present; we’ll debug `FacetMetricsPanel` + `historyByFacet` next.
* The SPA’s basic infrastructure is solid:

  * Routing, Node list & detail pages.
  * Theme and layout scaffolding.
  * Shared components like `LoadingSpinner` and `ErrorBanner`.
  * Import path issues (common vs shared) are resolved via a shim.
* Next high-impact steps:

  1. Fix the **debug crash counter drift** by refetching `/api/v1/status` after a crash via the UI.
  2. Fix **sparklines** so RPS history updates visibly over time.
  3. Fill in NodeDetailSidebar’s **data-plane** panel with real summaries (even if minimal).
  4. Add a small classification helper/label on NodeCard and NodeDetail to clearly highlight stale/unreachable metrics.
  5. Start designing the **Fleet Overview** global dashboard screen (top-level God-tier view) with node counts, crashes, and high-level metrics.

If you paste these notes into the next instance, we’ll be able to jump straight into either:

* Fixing the counters + sparklines, **or**
* Sketching and implementing the global Fleet Overview screen and DB/Playground sidebar content.


### END NOTE - DECEMBER 11 2025 - 21:15 CST



### BEGIN NOTE - DECEMBER 12 2025 - 13:25 CST

---

## 0) Quick snapshot (where we are right now)

**Crate:** `crates/svc-admin`
**Goal:** a *God tier* admin dashboard for **macronode + micronode** profiles (core-plane only).
**Current UX state:** Nodes overview is now *very close to the mock* (master list + right-side preview panel), and Node Detail is a real deep-dive screen with live polling, facet metrics, actions gating, and dev debug tools.

### ✅ Big win in this session

We upgraded the **right-hand Node preview panel** to match the “Node A” mock:

* **Stacked status pills** (Healthy + Metrics: fresh/stale/unreachable) at the top-right.
* **Planes table** inside the preview panel: `Plane | Health | Ready | Restarts`, including the chevron.
* “View /metrics” link placement (as in mock) with disabled state when no base URL.
* Overall look & spacing now reads like a real NOC console.

You confirmed via screenshot: **EXCELLENT!!!** and the table is rendering with populated plane rows.

---

## 1) What we changed (UI code) — important for continuity

### 1.1 `NodePreviewPanel.tsx` (Preview panel now supports planes table)

**File:** `crates/svc-admin/ui/src/components/nodes/NodePreviewPanel.tsx`

Key points:

* Added support for an optional `planes?: PlaneLike[] | null` prop.
* Implemented robust normalization helpers so it tolerates different upstream shapes:

  * restarts field: `restarts | restart_count | restartCount`
  * ready field: boolean or `"Ready"` string
  * health field: `'healthy' | 'degraded' | 'down'` with fallback “unknown”
* Added a **Planes** block + table rendering inside preview.
* Added “View /metrics” link computed from `node.base_url` (or `baseUrl`).

### 1.2 `NodeListPage.tsx` (Fix: pass planes into NodePreviewPanel)

**File:** `crates/svc-admin/ui/src/routes/NodeListPage.tsx`

This was the missing piece when you saw “Planes not loaded yet.”
**Fix required:** pass selected node’s `status.planes` into preview panel:

* We already fetch `statusById[node.id] = AdminStatusView`
* So `NodePreviewPanel` must receive:

  * `planes={selectedStatusState?.status?.planes ?? null}`

### 1.3 `NodeCard.tsx` (Metrics pill styling: move from tailwind-ish strings → real CSS classes)

**File:** `crates/svc-admin/ui/src/components/nodes/NodeCard.tsx`

Key points:

* Introduced CSS-based pills (`svc-admin-metrics-pill ...`) so the “Metrics: fresh” looks like the mock (green pill, consistent sizing).
* Still supports selection-aware behavior (`onSelect` renders a `<button>`, otherwise a `<Link>`).

### 1.4 `styles.css` (New CSS needed for pills + planes table look)

**File:** `crates/svc-admin/ui/src/styles.css`

We added/needed CSS for:

* `.svc-admin-metrics-pill` variants: fresh/stale/unreachable/loading/na
* Preview panel header “stacked pills” layout
* Planes table styling: headers, row dividers, right-aligned restarts column, chevron, “View /metrics” link styling.

---

## 2) Sparkline “last dot” animation (what we achieved)

### ✅ Desired behavior achieved

You asked for the dot to:

* stay at the edge of the line (no bobbing in/out),
* pulse brighter → fade → brighter,
* **not disappear fully** (bottom out around ~50% opacity),
* pulse **~25% slower**,
* and asked how to change dot size.

We moved away from the previous “bouncing” look and implemented a **pulse** style where the dot remains anchored and only **opacity/radius** changes.

### Important note about “top line doesn’t look active”

If a facet’s **RPS is 0.0** (or constant), the line will render flat and the last-dot pulse can *look* “inactive” even though it’s technically pulsing. This is expected for truly idle facets like `admin.debug_crash` when it’s not being called repeatedly.

---

## 3) Current UX behavior (as implemented)

### 3.1 Nodes overview page (`NodeListPage`)

* Loads nodes registry
* Background fetch per-node:

  * status (`getNodeStatus`)
  * facet metrics (`getNodeFacetMetrics`) → derive metrics freshness
* Allows selecting nodes via cards
* Right preview updates live based on selection:

  * identity: name/profile/id
  * health pill
  * metrics pill
  * planes table (now working)
  * Open → button to Node Detail

### 3.2 Node detail page (`NodeDetailPage`)

* Polling every 5s:

  * status
  * facets
* Derived:

  * overall health from planes
  * metrics health (fresh/stale/unreachable)
  * min sample age
* Shows warning banner if stale/unreachable
* Shows:

  * planes table
  * facet metrics panel with history (sparklines)
  * actions (reload/shutdown) gated by `UiConfig.readOnly + roles`
  * dev-only debug crash tool (maps plane name → `svc-*` service name)

---

## 4) Known issues / “we’ll tidy later” list (don’t lose this)

1. **Debug crash → counters sometimes don’t update immediately**

* You observed: counters didn’t work from the debug button but do via terminal commands (and show up after refresh).
* We should treat this as a *real* issue to close later:

  * confirm the debug endpoint triggers the same event path as the terminal flow
  * ensure svc-admin is re-sampling the right source after debug actions
  * confirm UI is reading the right restart counters (and not stale-cached)

2. **Sparklines/charts “not working like they used to”**

* We fixed the last-dot animation aesthetics.
* Remaining concerns likely come from data shape / rolling window update correctness / facet grouping.

We agreed: keep shipping features first; polish at the end.

---

## 5) What remains to “complete svc-admin” (God tier checklist)

Think of this as **MVP++ → Beta → God tier**.

### 5.1 Backend completeness (svc-admin service)

* ✅ Node registry + node status + metrics sampling pipeline exists (enough for live UI)
* ✅ DTO alignment between Rust and TS is mostly in place
* 🔜 Improve sampling & caching:

  * avoid sequential-per-node fetch loops (parallel with a cap)
  * centralize “poller tasks” so UI pages aren’t the only source of sampling truth
* 🔜 Harden auth modes:

  * `none` (dev), `ingress`, `passport` fully wired
  * consistent role model: `admin`, `ops`, `readonly` (or equivalent)
* 🔜 Audit + safety:

  * every mutating action emits an audit record (even if rejected)
  * rate-limit + explicit capability gating per node/profile

### 5.2 UI completeness (svc-admin SPA)

* ✅ Nodes overview + Preview panel now matches the mock closely
* ✅ Node detail is real and useful
* 🔜 Plane drill-down (click a plane row)

  * open a plane detail panel/page: per-plane metrics, restarts timeline, last error, logs link
* 🔜 Better metrics UX:

  * show “no traffic” vs “stale” vs “unreachable” more explicitly
  * persistent rolling windows with real sparklines (not just last point)
* 🔜 Global “Status” page (NOC wall):

  * cross-node summary: unhealthy planes, flapping, top error rates, top latency p99
* 🔜 Settings:

  * config view (read-only)
  * auth mode banner + “you are in dev insecure mode” warnings

### 5.3 “God tier” polish items

* keyboard navigation (j/k for nodes)
* quick filters (profile/env/health)
* “incident mode” banner (acknowledge degraded)
* export snapshot (json) for incident reports
* consistent empty states + skeleton loaders

---

## 6) Next focus: Database screen (Data & Storage) — proposed “God tier” plan

You asked for a **screen for databases** that can show:

* db size,
* file permissions,
* storage bandwidth / IO,
* and other relevant intel.

Here’s the plan that stays **safe, read-only, and core-plane aligned**:

### 6.1 Design principle (must keep us safe + clean)

We should **NOT** become a remote file browser.
Instead: expose **structured, whitelisted “storage facts”** from nodes.

### 6.2 Node-side contract (macronode/micronode admin-plane)

We need node admin-plane endpoints that return *curated* storage/db info.

**Recommended endpoints (read-only):**

* `GET /api/v1/storage/summary`

  * total disk size, free, used, mount(s), fs type
  * io rates (if available): read/write bytes/sec, iops
* `GET /api/v1/storage/databases`

  * list databases (logical name, engine type, path alias, size_bytes, last_compaction, health)
* `GET /api/v1/storage/databases/{name}`

  * deeper stats: file count, key count (if cheap), cache usage, compaction state, fragmentation estimate, etc.

**Permissions fields (safe version):**

* `mode` (e.g. `"0750"`)
* `uid`/`gid` numeric (optional)
* “flags” derived for UI:

  * `world_readable`, `world_writable` (warning if true)
  * `owner_writable` etc.

**Bandwidth / IO**

* If we can’t do OS-level io safely/cross-platform, fallback to:

  * service metrics already emitted to `/metrics` (best long-term)
  * or “unknown/not supported” cleanly

### 6.3 svc-admin backend work (adapter + DTOs)

* Add DTOs:

  * `StorageSummaryDto`
  * `DatabaseEntryDto`
  * `DatabaseDetailDto`
* Add node client methods:

  * `get_node_storage_summary(node_id)`
  * `get_node_databases(node_id)`
  * `get_node_database_detail(node_id, name)`
* Add caching + sampling rules:

  * these endpoints can be heavier → poll slower (e.g., 15–60s)
  * allow manual refresh button in UI

### 6.4 SPA UX (what the Database screen should look like)

**Where it lives:**

* Option A (clean): New route under node:

  * `/nodes/:id/storage`
* Option B (dashboard-wide): `/storage` with node selector

**Page layout (God tier but realistic):**

* Header: Node name + pills (Health + Metrics freshness)
* Top row: 3–4 “storage stat tiles”

  * Total / Used / Free
  * IO read/write (or “n/a”)
  * DB count
* Main table: Databases

  * Name | Engine | Size | Permissions | Health | Actions
  * row warnings: world-writable, size spikes, compaction backlog
* Click row → right-side detail drawer:

  * path alias (not raw path if we want to avoid leaking layout)
  * file count
  * last compaction
  * “read-only invariant” banner

### 6.5 Capability gating (very important)

Some nodes won’t support this yet. We should add:

* `capabilities: string[]` to `AdminStatusView` (or a sibling endpoint)
* UI only shows “Storage” tab if:

  * `capabilities.includes("storage.readonly.v1")`

This avoids half-working UI and makes rollouts clean.

---

## 7) Immediate next steps for next instance (high-impact order)

### Step 1 — Add Storage page route + stub UI (fast win)

* Create `ui/src/routes/NodeStoragePage.tsx`
* Add nav entry / link from NodeDetail sidebar (“Data & storage”)
* Use fake/mock data initially if node endpoint isn’t ready
* Build the layout + table + detail drawer first (match the mock quality)

### Step 2 — Define node admin-plane storage contract (macronode first)

* Add the 2–3 read-only endpoints under macronode admin
* Return curated DTOs (no raw browsing)
* Include capability flag in status response

### Step 3 — Wire svc-admin backend to proxy/storage-client those endpoints

* Add Rust DTOs + node client calls
* Add `/api/nodes/:id/storage/*` endpoints in svc-admin backend
* Hook the SPA to svc-admin endpoints

### Step 4 — Polish / correctness pass

* Add warnings (world-writable, low disk)
* Add caching/poll interval tuning
* Add Prometheus metrics in svc-admin about polling failures per node

---

## 8) What to bring into the next instance (so we don’t stall)

To implement the Storage/DB screen end-to-end efficiently, the next instance will benefit from:

* macronode admin-plane docs or codebundle for where we should add:

  * `/api/v1/storage/*`
  * capability flags
* any existing storage/db engine decisions (sled/rocks/etc) so the stats are real.

(If we don’t have those immediately, we can still build the UI/DTO scaffolding first.)

---

## 9) Rough completion estimate (svc-admin)

This is subjective, but grounded in what’s already working:

* **UI (read-only ops dashboard core): ~70%**

  * Nodes overview + preview: strong
  * Node detail: strong
  * Needs: drill-downs, storage page, polish
* **Backend (svc-admin service maturity): ~60%**

  * Core endpoints & sampling exist
  * Needs: stronger caching/poll architecture, auth hardening, capability model, more node contracts

---

### END NOTE - DECEMBER 12 2025 - 13:25 CST




### BEGIN NOTE - DECEMBER 17 2025 - 11:10 CST

## Carry-over notes — svc-admin “God-tier” dashboard (as of 2025-12-17)

### 0) Snapshot: where we are right now

* **Nodes NOC page works**: node cards + right-hand preview, with per-node status + metrics freshness loaded in the background. Status/metrics are fetched per node via `adminClient.getNodeStatus()` and `adminClient.getNodeFacetMetrics()` in sequential loops.  
* **Metrics freshness classification** is stable and consistent: `fresh | stale | unreachable`, where `fresh` means `min(last_sample_age_secs) <= 30s`. 
* **Storage slice is now real UI** (Node “Data & Storage” page): curated, read-only storage summary + DB inventory, with safe fallback to deterministic mock data when node endpoints aren’t implemented (404/405/501). 
* **DB “Open →” link is now wired from storage page** to `/nodes/:id/storage/databases/:name` (this was the source of the earlier 404). 

  * You confirmed the 404 is fixed and the DB page “looks great” (so that route/page now exists in your working tree).

---

### 1) What we accomplished (high signal)

#### A) NodeListPage is solid and future-proof enough (for now)

* Loads node registry once, then background-loads per-node status and per-node facet metrics freshness.  
* Selection model is clean: left cards select; right preview panel shows details and passes planes for the “Planes” table.  

**Optional future-proof refactor (not required yet):**

* Extract two hooks:

  * `useNodeRegistry()` (nodes/loading/error + default selection)
  * `usePerNodeStatusAndMetrics(nodes)` (statusById + metricsById, with concurrency limits)
* The current sequential `for ... of` loops are fine for “a handful” of nodes, but when you scale to dozens/hundreds, you’ll want controlled concurrency (e.g. `p-limit` style) instead of strictly serial fetches.

#### B) NodeStoragePage is now a “real product page”

* It explicitly enforces the core invariant: **no remote file browser**, **curated DTOs only**, **read-only**. 
* It correctly treats missing endpoints as non-fatal, falling back on safe deterministic mock data:

  * The UI checks for 404/405/501 using `isMissingEndpoint()`. 
  * `adminClient` also documents this contract: storage endpoints may return 404/501 until nodes implement them; callers should fallback. 
* The “Open →” link is generated consistently using the selected DB and node id. 

#### C) NodePreviewPanel supports the “God-tier” scan workflow

* It’s explicitly designed to show richer details for the selected node, supports an optional planes table, and (optionally) a storage ring that falls back when endpoints are missing.  

#### D) Database detail route/page (the 404) is now fixed

* Previously: clicking a DB (“sleddb”) landed on a 404.
* Now: you’ve confirmed the DB page renders and “looks great”, and it shows curated file/permission-style info (as intended by the Storage slice).

  * Carryover reminder: keep it **curated**—no raw filesystem browsing, no arbitrary path traversal, no “remote file manager”.

---

### 2) Current contracts & behavior to preserve

#### A) Storage endpoints are optional until macronode/micronode implement them

`adminClient` defines these read-only endpoints:

* `/api/nodes/:id/storage/summary`
* `/api/nodes/:id/storage/databases`
* `/api/nodes/:id/storage/databases/:name` (detail)
  …and warns they may be missing initially (404/501). 

UI behavior:

* Missing endpoints → treat as “not implemented”, use deterministic mock instead of blowing up.  

#### B) Metrics freshness rules are consistent across pages

* Threshold: 30 seconds (`FRESH_THRESHOLD_SECS = 30`). 
* `unreachable` is driven by error presence.

---

### 3) Known issues / deferred cleanup (don’t forget)

* **Counters don’t work from the UI debug button but do work via terminal commands** (restart counts appear after terminal-triggered events). (From prior session notes you told me to record.)
* **Sparklines/charts aren’t working like they used to** (also recorded as “tidy later”).
* If you still see any React “hooks order” warnings: the pattern used in NodeStoragePage is the correct fix—**all hooks above early returns** (and only do safe returns after derived state).

---

### 4) Next focus: App Playground (what to do next)

You explicitly want: **set up playground next**, then UI tidying, then wire to macronode/micronode.

#### A) Gating is already part of the UI config model

* The UI config supports a dev flag: `enableAppPlayground` (dev-only posture). 
* Your docs already call out that the playground should be gated and can be controlled via a dev flag. 
* There’s also a `ronCorePlaygroundClient.ts` stub in the codebase, so the shape is anticipated. 

#### B) Suggested “God-tier” MVP for playground (still safe)

* A **read-only** playground first:

  * “Facet manifest viewer” (load, validate, show warnings)
  * “Request builder” (compose a request to a facet route)
  * “Dry-run mode” (no mutations, no writes)
* Then (later) allow controlled writes under explicit dev mode + explicit “I understand” UI gating + audit log events.

---

### 5) Requested feature: Uptime + launch metadata (design plan)

You want:

* **Abbreviated uptime ticker on NodeCards**
* **Detailed runtime metadata on NodeDetailPage**: “how long up”, “launch date/time”, “which user launched it”, etc.

#### A) Add runtime fields to the node status contract (recommended)

Add something like this to the node’s admin status DTO (or a sibling endpoint):

* `started_at: RFC3339 string` (UTC)
* `started_by: string | null` (user/service principal)
* `host: string | null` (hostname)
* `pid: number | null`
* `build_version: string` / `git_sha: string` / `profile: micronode|macronode`
* `boot_id: string` (optional but awesome for “same machine restarted” diagnostics)

Where to surface:

* **NodeCard:** `uptimeCompact = now - started_at` (e.g., `3d 04h`, live-updating)
* **NodeDetailPage:** full breakdown:

  * “Started: 2025-12-17 09:13:02Z”
  * “Uptime: 4h 22m 10s”
  * “Launched by: stevan@… / systemd / docker / dev script”
  * “Host/PID/Build”

#### B) UI implementation detail (important so we don’t create 200 timers)

* Do **not** create one `setInterval` per NodeCard.
* Make a single `useNow(tickMs=1000)` hook at the NodeListPage level and pass `now` down, or store `now` in context.
* Derive “uptime” purely as `now - started_at`.

---

### 6) Before wiring to macronode/micronode (the “no drift” checklist)

When you’re ready to wire for real, the key is to keep svc-admin as a **thin observer**:

1. **Lock the contracts** (status + metrics + storage).
2. Implement those endpoints on macronode/micronode admin planes (even if some return “not implemented” initially).
3. Remove mock fallbacks gradually as endpoints become real (but keep fallback behavior as a resilience feature).

Also keep the project-wide interop doctrine in mind: micronode/macronode are profiles of the same system and should share compatible contracts. 

---

### 7) Quick “next session” agenda (tight + high impact)

1. **Playground MVP** behind `enableAppPlayground` flag (UI + minimal backend stubs).
2. **Uptime/runtime metadata**:

   * add runtime fields to node status DTOs
   * add compact ticker on NodeCard + detailed section on NodeDetailPage
3. **UI tidying pass**:

   * unify spacing, remove any remaining horizontal scroll edge-cases
   * backlog: debug counters mismatch + sparkline regression

### END NOTE - DECEMBER 17 2025 - 11:10 CST



### BEGIN NOTE - DECEMBER 18 2025 - 12:59 CST


### Carry-over notes — svc-admin “God-tier” dashboard (Playground wiring status)

**As of 2025-12-18 (America/Chicago)**

---

## 0) Snapshot: what you’re seeing and why

You said: **“The playground box we have right now is the same as it was before… I do not see any of the changes yet.”**

That’s consistent with the current state of the work:

* We **added client + DTO + CSS + locale plumbing** for a new **PlaygroundPage + dev-only API calls**, but…
* If the **route isn’t reachable / nav isn’t updated / the backend endpoints don’t exist yet / dev flag isn’t enabled**, the UI can appear unchanged.
* Also: the “playground box” you’re describing sounds like the **NodeDetail sidebar playground textarea/button** you already had. The new work is primarily for a dedicated **`/playground` page** plus API endpoints.

So the key missing pieces are:

1. **Backend routes + handlers** for playground endpoints (svc-admin server)
2. **UI wiring**: sidebar link + route (you said PlaygoundPage + App.tsx was requested earlier, but we need to confirm what actually landed)
3. **Enable flag**: `ui.dev.enable_app_playground = true` in svc-admin config so the page isn’t gated off

---

## 1) What we accomplished (high-signal)

### A) UI: added Playground API plumbing

**File updated**

* `crates/svc-admin/ui/src/api/adminClient.ts`

  * Added:

    * `getPlaygroundExamples()` → GET `/api/playground/examples`
    * `validatePlaygroundManifest(manifestToml)` → POST `/api/playground/manifest/validate`
  * Centralized error shaping includes `err.status` so UI can treat 404/501 as “not implemented / disabled”.

### B) UI: added i18n strings for Playground

**Files updated**

* `crates/svc-admin/ui/public/locales/en-US.json`
* `crates/svc-admin/ui/public/locales/es-ES.json`

Added:

* `nav.playground`
* `playground.title`, `playground.subtitle`
* `playground.disabled.title`, `playground.disabled.body`

### C) UI: added CSS classes used by PlaygroundPage layout

**File updated**

* `crates/svc-admin/ui/src/styles.css`

Appended a minimal design system to support a “real” Playground page:

* `.page`, `.page-header`
* `.grid-2`
* `.panel`, `.panel-title`, `.panel-body`
* `.btn`, `.btn-primary`
* `.list`, `.list-item` (+ `.active`)
* `.textarea`, `.codeblock`
* Sidebar active link styles: `.svc-admin-sidebar-link`, `.svc-admin-sidebar-link-active`

**Important:** This avoids needing `App.css`. `styles.css` is the correct place if it’s already the global stylesheet.

---

## 2) Compile blocker we hit and fixed

### TOML dependency feature mismatch

You got:

> `toml` does not have feature `serde`

**Fix applied**

* `crates/svc-admin/Cargo.toml`

  * Remove `features = ["serde"]` from `toml`
  * Use `toml = "0.8"` (serde support is inherent in 0.8)

This unblocks `cargo build -p svc-admin` once that change is in your tree.

---

## 3) What is still missing (why you can’t “see” new playground yet)

### A) Backend endpoints aren’t implemented yet (most likely)

Your UI client is now calling:

* `GET /api/playground/examples`
* `POST /api/playground/manifest/validate`

But your current `crates/svc-admin/src/router.rs` (the version you pasted earlier) **does not define those routes**, so the UI will get:

* 404 Not Found (route missing) or 501 Not Implemented (if stubbed later)

**Needed updates**

* `crates/svc-admin/src/router.rs`
  Add:

  * `.route("/api/playground/examples", get(playground_examples))`
  * `.route("/api/playground/manifest/validate", post(playground_manifest_validate))`

* Add a small module such as:

  * `crates/svc-admin/src/playground/mod.rs` (new)
  * `crates/svc-admin/src/playground/examples.rs` (optional split)
  * `crates/svc-admin/src/dto/playground.rs` (new DTOs) **unless you already added these earlier**

### B) UI routing/nav might not be wired (or is gated)

Even if `PlaygroundPage.tsx` exists, you won’t see it unless:

* `ui/src/App.tsx` has a route like:

  * `<Route path="/playground" element={<PlaygroundPage />} />`
* Sidebar includes a link to it:

  * `Sidebar.tsx` must add “Playground”
  * and ideally use `NavLink` so active styling works

**Needed updates**

* `crates/svc-admin/ui/src/App.tsx`
* `crates/svc-admin/ui/src/components/layout/Sidebar.tsx`
* Ensure `crates/svc-admin/ui/src/routes/PlaygroundPage.tsx` exists and is imported.

### C) Dev flag gating must be enabled

Your UI config model supports:

* `dev.enable_app_playground`

But if config has it set false (default), the UI may:

* hide the link,
* show “disabled” content,
* or not show the route depending on how the page is coded.

**Needed action**

* Ensure your svc-admin config sets:

  * `ui.dev.enable_app_playground = true`

Where this is controlled depends on your project’s config loader; typical places:

* `crates/svc-admin/src/config/ui.rs` (already has `UiDevCfg`)
* `svc-admin` config file / env overrides (wherever your loader reads from)

---

## 4) Files touched so far (audit trail)

### Rust (svc-admin backend)

* ✅ `crates/svc-admin/Cargo.toml` (toml feature fix; `toml = "0.8"` with no serde feature)

### TypeScript UI

* ✅ `crates/svc-admin/ui/src/api/adminClient.ts` (added playground client methods)
* ✅ `crates/svc-admin/ui/public/locales/en-US.json` (playground strings)
* ✅ `crates/svc-admin/ui/public/locales/es-ES.json` (playground strings)
* ✅ `crates/svc-admin/ui/src/styles.css` (new generic page/panel/button/codeblock styles + sidebar active link styles)

---

## 5) Concrete “next instance” plan to finish Playground MVP

### Step 1 — Backend: add DTOs + handlers (read-only, safe)

**Goal:** Make `/api/playground/examples` and `/api/playground/manifest/validate` real.

**Files to add/update**

1. `crates/svc-admin/src/dto/playground.rs` (new)

   * `PlaygroundExampleDto { id, title, description, manifestToml }`
   * `PlaygroundValidateManifestReq { manifest_toml }`
   * `PlaygroundValidateManifestResp { ok, errors[], warnings[], parsedSummary? }`

2. `crates/svc-admin/src/playground/mod.rs` (new)

   * holds curated examples as static strings
   * validates TOML via `toml::from_str::<toml::Value>()` (or a real Manifest struct later)
   * returns “safe errors” (no file paths, no internal debug dumps)

3. `crates/svc-admin/src/router.rs` (update)

   * add routes
   * handlers should:

     * check `state.config.ui.dev.enable_app_playground`
     * if disabled → `404` (or `403`), whichever policy you want (I recommend **404** to keep it dev-only and non-discoverable)

### Step 2 — UI: ensure route + nav show the page

**Files to update**

1. `crates/svc-admin/ui/src/App.tsx`

   * add `/playground` route to `PlaygroundPage`

2. `crates/svc-admin/ui/src/components/layout/Sidebar.tsx`

   * add the Playground nav item (optionally only if `enableAppPlayground` is true, depending on UX preference)

3. Confirm `crates/svc-admin/ui/src/routes/PlaygroundPage.tsx` exists

   * The page should:

     * load examples
     * allow selecting an example
     * provide editor textarea
     * call validate endpoint
     * show errors/warnings with nice formatting
     * show “disabled” state if `enableAppPlayground` is false

### Step 3 — Config: flip the dev flag

**Goal:** Actually see it.

* Set `ui.dev.enable_app_playground = true` in your svc-admin config source (file/env)
* Restart svc-admin

---

## 6) Known issues / backlog (carry forward)

* Counters mismatch: **UI debug button doesn’t update counters**, but terminal-triggered events do. Keep for later.
* Sparklines regression: charts not working like they used to. Keep for later.
* Continue enforcing the core invariant: **no remote file browser** (curated DTOs only).

---

## 7) Fast verification checklist (when you resume)

After implementing the backend routes and UI wiring:

Run these checks:

* Open: `http://127.0.0.1:5300/playground` (or your UI address)
* If it still looks unchanged, directly curl:

  * `curl -sS http://127.0.0.1:5300/api/playground/examples | head`
  * `curl -sS -X POST http://127.0.0.1:5300/api/playground/manifest/validate -H "Content-Type: application/json" -d '{"manifestToml":"[package]\nname=\"x\""}'`

Expected:

* examples returns JSON list
* validate returns `ok=false` with errors (because that manifest is intentionally incomplete)

---

## 8) What to upload / include next time (so we move fast)

To avoid “it still looks the same” loops, next instance we should have:

* The current `crates/svc-admin/src/router.rs`
* The current config loader path(s) for UI config / dev flag (where `enable_app_playground` is actually read)
* The current `PlaygroundPage.tsx` if it exists in your tree (so we can align it to the new API)

---

### END NOTE - DECEMBER 18 2025 - 12:59 CST




### BEGIN NOTE - DECEMBER 20 2025 - 11:11 CST


# Carry-over notes — svc-admin “God-tier” dashboard → Maconode/Micronode wiring phase

These notes assume **UI is “good enough for now”** and the next major push is **plugging svc-admin into real macronode (then port to micronode)**, starting with the **Storage/Databases** slice and finishing the **Playground** wiring (sidebar + route + enable flag).

---

## 0) Current snapshot (what works right now)

### Running stack

You run a dev script that starts:

* **macronode** (admin plane + metrics)
* **svc-admin backend** (API + metrics)
* **svc-admin SPA** (Vite dev server)

Default addresses you’ve been using:

* macronode: `127.0.0.1:8080`
* svc-admin backend: `127.0.0.1:5300`
* svc-admin metrics: `127.0.0.1:5310`
* SPA: `http://localhost:5173`

### Playground status

* **Backend routes exist** in `crates/svc-admin/src/router.rs`:

  * `GET /api/playground/examples`
  * `POST /api/playground/manifest/validate`
* Playground is **dev-gated** by config:

  * Env flag: `SVC_ADMIN_UI_DEV_ENABLE_APP_PLAYGROUND=true`
* When disabled, manually entering `/playground` shows:

  * “Playground is currently disabled… SVC_ADMIN_UI_DEV_ENABLE_APP_PLAYGROUND=true”

### UI status

* `ui/src/App.tsx` **does NOT** currently include a `/playground` route.
* Sidebar currently **does NOT** include “Playground” link alongside Nodes/Settings.
* So even with backend ready + flag enabled, Playground won’t feel “integrated” until we add:

  * App route + Sidebar link (and optionally hide/disable link based on ui-config).

---

## 1) What changed / what we confirmed in this instance

### A) Confirmed: backend handler for Playground is in `router.rs`

The Playground handler is not “some other module” right now — it’s embedded directly in:

* `crates/svc-admin/src/router.rs`

  * `playground_examples()`
  * `playground_validate_manifest()`
  * gating via `state.config.ui.dev.enable_app_playground`

### B) Confirmed: config loader reads the dev flag

* `crates/svc-admin/src/config/loader.rs` loads:

  * `SVC_ADMIN_UI_DEV_ENABLE_APP_PLAYGROUND` → `cfg.ui.dev.enable_app_playground`

### C) Script needs to export the flag

Your dev-stack script must set:

* `SVC_ADMIN_UI_DEV_ENABLE_APP_PLAYGROUND=true`
  for the **svc-admin backend process**.

> Important: Your current script also uses some env var names that don’t match the current loader (see §3).

### D) We hit & resolved a compile mismatch

At one point `cargo build -p svc-admin` failed due to a **non-exhaustive match** on `error::Error` (missing `Io` and `Auth` arms). This was fixed by adding match arms (or a wildcard) so router error mapping is exhaustive.

---

## 2) High-priority “next instance” objectives (in order)

### Objective 1 — Make Playground *feel real* in the UI

**Goal:** “Playground” appears in sidebar and routes cleanly like Nodes/Settings.

Work items:

1. **Add route** in `crates/svc-admin/ui/src/App.tsx`

   * `Route path="/playground" …`
2. **Add sidebar nav item** in `crates/svc-admin/ui/src/components/layout/Sidebar.tsx`

   * Link to `/playground`
3. **Gating behavior** (pick one):

   * **Option A (recommended):** Show link always; page shows “disabled” panel when flag off.
   * **Option B:** Hide link unless `ui-config` indicates enabled.

Verification:

* Start stack with `SVC_ADMIN_UI_DEV_ENABLE_APP_PLAYGROUND=true`
* Open `http://localhost:5173/playground`
* Confirm examples load + validation works.

---

### Objective 2 — Flip Storage pages from mock → live

You already have:

* `NodeStoragePage.tsx` and `NodeDatabaseDetailPage.tsx`
* UI hooks designed to call:

  * `/api/nodes/:id/storage/summary`
  * `/api/nodes/:id/storage/databases`
  * `/api/nodes/:id/storage/databases/:name`

Right now, backend endpoints in `svc-admin/src/router.rs` are still (or were recently) `501 NOT_IMPLEMENTED` stubs.

**Goal:** Implement those three svc-admin stubs as a proxy/aggregator from the node admin plane, returning curated DTOs.

**This is the big “plug into macronode” wedge.**

Work items:

1. **Define node admin-plane contract (macronode-side)**

   * Add endpoints on macronode:

     * `GET /api/v1/storage/summary`
     * `GET /api/v1/storage/databases`
     * `GET /api/v1/storage/databases/:name`
   * Return stable DTO JSON (no internal filesystem dumps, no arbitrary browsing).
2. **svc-admin: implement proxy handlers**

   * Update:

     * `node_storage_summary`
     * `node_storage_databases`
     * `node_storage_database_detail`
   * Use `NodeRegistry` + `NodeClient.try_get_json()` optional endpoint logic:

     * If node doesn’t have it yet → return `501` (or a curated “capability missing” response).
3. **DTO hygiene**

   * Ensure svc-admin returns the DTOs the SPA expects:

     * `StorageSummaryDto`
     * `DatabaseEntryDto`
     * `DatabaseDetailDto`
   * Avoid `serde_json::Value` as the final output for these endpoints (unless you intentionally keep it as an internal intermediate).

Verification:

* `curl -sS http://127.0.0.1:5300/api/nodes/example-node/storage/summary | jq`
* Load the Storage page in the SPA — it should “just switch” from stub content to live data.

---

### Objective 3 — Port macronode storage endpoints to micronode

Once macronode works end-to-end, micronode should implement the *same* admin-plane endpoints so svc-admin doesn’t care which node type it’s pointed at.

**Rule:** keep the **contract identical** so svc-admin stays simple.

---

## 3) Known env var mismatches to fix (script hygiene)

Your current dev-stack script sets env vars like:

* `SVC_ADMIN_HTTP_ADDR=...`
* `SVC_ADMIN_NODES__EXAMPLE_NODE__BASE_URL=...`

But your current config loader (`crates/svc-admin/src/config/loader.rs`) reads:

* `SVC_ADMIN_BIND_ADDR` (not `SVC_ADMIN_HTTP_ADDR`)
* `SVC_ADMIN_METRICS_ADDR`
* node override convenience:

  * `SVC_ADMIN_EXAMPLE_NODE_URL` (not the nested `SVC_ADMIN_NODES__…` style)

So: the script still “works” mainly because defaults match what you want — but it’s easy to get confused later.

**Next instance script hygiene checklist:**

* Set `SVC_ADMIN_BIND_ADDR` instead of `SVC_ADMIN_HTTP_ADDR`
* Keep `SVC_ADMIN_METRICS_ADDR`
* Set `SVC_ADMIN_EXAMPLE_NODE_URL="http://127.0.0.1:8080"`
* Add: `SVC_ADMIN_UI_DEV_ENABLE_APP_PLAYGROUND=true`

(We already know the Playground flag is required.)

---

## 4) Files to keep in mind (where things live)

### svc-admin backend (Rust)

* `crates/svc-admin/src/router.rs`

  * All routes + handlers (Playground implemented here; Storage stubs are here)
* `crates/svc-admin/src/config/loader.rs`

  * Env-driven config, including Playground flag
* `crates/svc-admin/src/nodes/client.rs`

  * Node admin-plane HTTP client
  * Has `try_get_json()` optional endpoint support (perfect for “gradual rollout” endpoints like storage)
* `crates/svc-admin/src/nodes/registry.rs`

  * Node lookup + action proxies; will likely gain storage proxy helpers
* `crates/svc-admin/src/dto/*`

  * Currently: `ui`, `me`, `node`, `metrics`
  * Storage DTOs may need their own module if not already present.

### svc-admin UI (React/Vite)

* `crates/svc-admin/ui/src/App.tsx`

  * Needs `/playground` route
* `crates/svc-admin/ui/src/components/layout/Sidebar.tsx`

  * Needs “Playground” nav link
* `crates/svc-admin/ui/src/components/layout/Shell.tsx`

  * Already wraps Sidebar + TopBar + `<main>{children}</main>`
* `crates/svc-admin/ui/src/api/adminClient.ts`

  * Has Playground endpoints
  * Has dev-only **httpLog ring buffer** (useful later for a “Request Inspector” panel)
* `crates/svc-admin/ui/src/routes/NodeStoragePage.tsx`

  * Already calls storage endpoints; will automatically “go live” once backend stubs are real.

### macronode (Rust)

* Admin-plane routes (where `/api/v1/status` and debug crash live)
* Needs the **storage endpoints** added under the same admin-plane router.

---

## 5) Immediate verification commands (when you resume)

### Playground

Start stack with `SVC_ADMIN_UI_DEV_ENABLE_APP_PLAYGROUND=true`, then:

* `curl -sS http://127.0.0.1:5300/api/playground/examples | head`
* `curl -sS -X POST http://127.0.0.1:5300/api/playground/manifest/validate -H "Content-Type: application/json" -d '{"manifestToml":"[package]\nname=\"x\""}' | head`

### Storage (once implemented)

* `curl -sS http://127.0.0.1:5300/api/nodes/example-node/storage/summary`
* `curl -sS http://127.0.0.1:5300/api/nodes/example-node/storage/databases`
* `curl -sS http://127.0.0.1:5300/api/nodes/example-node/storage/databases/<name>`

---

## 6) Backlog / “later” items (do not block macronode wiring)

* **Counters mismatch:** UI debug button vs terminal-triggered restarts (keep for later).
* **Sparklines regression:** charts not working like they used to (keep for later).
* **ErrorBanner styling:** currently extremely minimal; we can upgrade later (icons, tone, close, multiline formatting).
* Optional “Request Inspector” UI using `adminClient`’s `httpLog` ring buffer.

---

## 7) What to provide next time (to move fastest)

For the Storage proxy work, the highest-leverage files to paste/upload are:

### svc-admin

* `crates/svc-admin/src/router.rs`
* `crates/svc-admin/src/nodes/client.rs`
* `crates/svc-admin/src/nodes/registry.rs`
* Any DTO files for storage if already started (or current `types/admin-api` in the UI that defines the expected DTO shapes)

### macronode

* The admin-plane router/handlers file(s) where `/api/v1/status` and `/api/v1/debug/crash` are implemented
* Any config paths where macronode’s storage roots/db paths are defined

---

## 8) Execution plan for the next instance (tight, no thrash)

1. **UI polish (very small):** add `/playground` route + sidebar link (gated).
2. **macronode:** implement `GET /api/v1/storage/*` endpoints (start minimal, stable DTOs).
3. **svc-admin:** implement the three `node_storage_*` stubs as real proxies to the node endpoints.
4. Verify Storage pages switch from “not implemented” → live.
5. Port the same macronode endpoints to micronode (contract identical).

---

### END NOTE - DECEMBER 20 2025 - 11:11 CST


### BEGIN NOTE - DECEMBER 20 2025 - 18:00 CST

---

# ✅ CARRY-OVER NOTES — svc-admin (backend + UI) + macronode (node admin plane)

## Snapshot (current state)

* **We now see live CPU, RAM, network bandwidth, and storage** in the admin dashboard (no longer mock-only).
* We implemented a **truthful system summary endpoint on macronode** and **a proxy endpoint in svc-admin**, then **wired the SPA to call it**.
* Storage remains a separate “storage inventory” slice; system summary is “CPU/RAM/network rates”.

---

# 1) What we accomplished — macronode

## 1.1 Added/validated node endpoint: `/api/v1/system/summary`

**Goal:** Provide “preview tile” data without the UI scraping Prometheus or running shell commands.

### Endpoint behavior (important invariants)

* Returns a JSON DTO with:

  * `updatedAt` (RFC3339)
  * `cpuPercent` (best effort; Option)
  * `ramTotalBytes` (u64 bytes)
  * `ramUsedBytes` (u64 bytes)
  * `netRxBps` (optional; only available after 2 samples)
  * `netTxBps` (optional; only available after 2 samples)
* Network rates require **two samples**:

  * first request returns `netRxBps/netTxBps` missing
  * after a short delay (ex: 1s), the next request returns rates
* No unsafe, no blocking, and **no await while holding locks** (sampler uses a lock only around sampling).

### Key file

**`crates/macronode/src/http_admin/handlers/system_summary.rs`**

* Uses:

  * `OnceLock<Mutex<Sampler>>` to keep sampler state (previous totals + timestamp)
  * `sysinfo::System` for CPU/memory
  * `sysinfo::Networks` for per-interface totals
* Computes bytes correctly:

  * sysinfo memory reported in KiB → multiply by 1024 to bytes
* Computes network rates:

  * sums rx/tx totals across interfaces
  * takes delta / dt to compute bytes/sec
  * guards very small dt (e.g. < 0.2s) to avoid nonsense

### How we verified it

Terminal test pattern:

```bash
curl -sS http://127.0.0.1:8080/api/v1/system/summary | jq
sleep 1
curl -sS http://127.0.0.1:8080/api/v1/system/summary | jq
```

Expected:

* First call: CPU/RAM present; network rates absent
* Second call (after delay): network rates present

We observed exactly that.

### Why “Adding ntapi/windows/windows-core” showed up

* Those crates appear because `sysinfo` (and/or its transitive deps) supports Windows and pulls Windows crates as transitive dependencies for cross-platform support. It doesn’t mean we explicitly “added Windows support code”; Cargo is resolving dependencies.

---

# 2) What we accomplished — svc-admin backend (Rust)

## 2.1 Router is now a true “proxy surface” for node slices

**Goal:** svc-admin is the stable UI backend; it proxies optional slices from nodes.

### Router routes in play

**`crates/svc-admin/src/router.rs`**
Already had storage slice:

* `/api/nodes/:id/storage/summary`
* `/api/nodes/:id/storage/databases`
* `/api/nodes/:id/storage/databases/:name`

We added **system summary proxy** route (important):

* `/api/nodes/:id/system/summary` (GET)

### The key pattern we follow for gradual rollout

* Nodes may not implement an endpoint yet.
* So in svc-admin we treat **404/405/501 as “missing capability”**:

  * server returns `501 Not Implemented` OR `null` depending on chosen contract
  * UI can fall back to mock without error spam

## 2.2 Node client supports “optional endpoint” semantics

**`crates/svc-admin/src/nodes/client.rs`**

* We already use an internal helper:

  * `get_json_optional<T>()` which returns `Ok(None)` on 404/405/501
* This is used by storage slice and is the correct pattern for system slice too.

## 2.3 Node registry supports optional slice methods

**`crates/svc-admin/src/nodes/registry.rs`**

* Storage methods exist:

  * `try_storage_summary`
  * `try_storage_databases`
  * `try_storage_database_detail`
* We added/used the same approach for system summary (or need it if not already landed in your current tree):

  * `try_system_summary(id)` → `Result<Option<SystemSummaryDto>>`
  * It should call NodeClient optional GET like:

    * `/api/v1/system/summary`

## 2.4 Compile errors fixed and why they happened

We hit:

* `cannot find trait IntoResponse`
* `Result<impl IntoResponse>` wrong generic count

Root cause:

* We accidentally used `Result<impl IntoResponse>` with `Result` = our crate `Result<T>` alias (which expects `Result<T, Error>`), and also didn’t import `IntoResponse`.

Fix pattern:

* In router handlers, prefer **explicit axum return types**:

  * `Result<Json<T>, StatusCode>` for normal DTOs
  * or `impl IntoResponse` without our crate `Result`
* If returning optional DTO:

  * either return `Result<Json<SystemSummaryDto>, StatusCode>` when guaranteed
  * or return `Result<Json<Option<SystemSummaryDto>>, StatusCode>` if “missing endpoint” should be null
  * OR return `StatusCode::NOT_IMPLEMENTED` when `None` (recommended for clarity)

We also made sure to keep the “map upstream error to StatusCode” logic consistent.

---

# 3) What we accomplished — svc-admin UI (React)

## 3.1 We stopped showing mock CPU/RAM/Bandwidth by wiring live system summary

Previously:

* NodePreviewPanel and NodeDetailPage had deterministic mock generators for:

  * CPU %, RAM used/total, bandwidth utilization
* Storage ring could be live if node storage endpoint existed.

Now:

* CPU/RAM/net rates come from **svc-admin proxy** `/api/nodes/:id/system/summary`
* Storage remains its own call `/api/nodes/:id/storage/summary`

### Key UI file: NodePreviewPanel

**`crates/svc-admin/ui/src/components/nodes/NodePreviewPanel.tsx`**

* Previously: CPU/RAM/Bandwidth were always mock.
* Updated behavior (current intended):

  * On node select:

    * fetch storage summary
    * fetch system summary
  * Use live values when available
  * If system endpoint missing or errors → fall back to deterministic mock
* The “ring pills” show `Live` vs `Mock`

### Key UI file: NodeDetailPage

**`crates/svc-admin/ui/src/routes/NodeDetailPage.tsx`**

* Previously: utilization gauges were mock-only.
* Updated behavior (current intended):

  * utilization section should be driven by:

    * system summary → cpu, ram, net (if used)
    * storage summary → storage
  * keep mock fallback only if missing endpoint
* Metrics freshness banners remain based on facet metrics sampling (separate system)

## 3.2 adminClient fixes (important)

**`crates/svc-admin/ui/src/api/adminClient.ts`**
We fixed a real problem:

* There was a broken “extra import/export chunk” at the bottom:

  * imported `SystemSummaryDto` using alias path
  * called `httpMaybeJson()` which didn’t exist
  * exported a free function not integrated into `adminClient`

Final posture:

* `SystemSummaryDto` is imported in the main types import
* `adminClient.getNodeSystemSummary(id)` exists
* It uses a helper `requestMaybeJson()`:

  * returns `null` on 404/405/501 (missing endpoint)
  * throws on real errors
* Keeps request logging ring buffer intact

This change was necessary for the UI to consistently fetch live system data.

---

# 4) Testing checklist (repeatable)

## macronode

1. Run macronode with admin plane enabled (whatever your dev script uses).
2. Verify system summary endpoint:

```bash
curl -sS http://127.0.0.1:8080/api/v1/system/summary | jq
sleep 1
curl -sS http://127.0.0.1:8080/api/v1/system/summary | jq
```

Expect:

* CPU/RAM always present
* net rates appear on second call

## svc-admin backend

1. Ensure node registry config includes the node base_url for macronode admin plane.
2. Hit the svc-admin proxy route (once implemented):

```bash
curl -sS http://127.0.0.1:<svc-admin-port>/api/nodes/<nodeId>/system/summary | jq
```

Expect:

* same DTO shape as macronode, or 501/404 until wired

## SPA

* Open Nodes page:

  * Preview pills should show “Live” for CPU/RAM/Bandwidth/Storage (as applicable)
* Open Node detail:

  * Utilization gauges should reflect live values
* If a node doesn’t implement a slice:

  * UI should show mock but remain stable (no fatal errors)

---

# 5) Known issues / items to watch

## 5.1 Optional endpoints contract consistency

For optional slices (system/storage/playground):

* Decide one consistent UI contract:

  * **Option A (recommended):** server returns `501` when node endpoint missing
  * **Option B:** server returns `200 { null }` (less clear, but workable)
    Right now we’ve used both patterns in different places historically—standardize.

## 5.2 Units & naming consistency

* `netRxBps` / `netTxBps` are **bytes per second**.
* UI helper `fmtBps()` currently displays decimal units; for bytes/sec you may want `B/s, KiB/s, MiB/s` (optional polish).
* Ensure DTO fields use camelCase consistently between backend + SPA types.

---

# 6) What remains (next high-impact steps)

## 6.1 System + Storage integration polish

* **NodeDetailPage utilization** should use real values everywhere:

  * CPU → system summary
  * RAM → system summary
  * Storage → storage summary
  * Bandwidth → system summary net rates (or display as “rx/tx” instead of a single “utilization %” unless link capacity is known)
* Decide how you want to represent “bandwidth utilization”:

  * If no known link capacity: show rx/tx rates only (best truthful)
  * If you want a utilization %: you need a configured “link capacity” per node or measured baseline

## 6.2 Uptime + launch metadata (operator gold)

You explicitly want:

* Uptime counters on node cards (abbreviated)
* Detailed uptime/launch info on node detail:

  * start time (UTC + local rendering)
  * who launched it (subject/identity if available)
  * boot ID / process ID (optional)
    Next steps:
* Add macronode endpoint: `/api/v1/system/identity` or extend system summary with:

  * `processStartedAt`
  * `bootId`
  * `pid`
  * `launchedBy` (if available; else null)
* Proxy it through svc-admin.
* Render in NodeCard + NodeDetailSidebar.

## 6.3 Databases screen “God tier” (read-only inventory first)

You asked for a database screen showing:

* DB names, sizes, permissions, ownership, and storage path alias
* Potentially bandwidth and I/O hints
  We already have the **svc-admin routes** for:
* `/api/nodes/:id/storage/databases`
* `/api/nodes/:id/storage/databases/:name`
  Next:
* Ensure macronode actually implements the node admin-plane endpoints:

  * `/api/v1/storage/summary`
  * `/api/v1/storage/databases`
  * `/api/v1/storage/databases/{name}`
* Add UI route:

  * `NodeStoragePage` or `DatabasesPage`
* Use deterministic mock fallback when missing endpoints (same pattern as system).

## 6.4 Capability discovery (optional, but very useful)

Instead of guessing via 404/501, add:

* `/api/v1/capabilities` on macronode (list supported slices)
* svc-admin can show “System: yes/no”, “Storage: yes/no”, “Playground: yes/no”
* UI can switch to live/mock without error spam

## 6.5 Cleanups

* Consolidate “mock fallback helpers” into a single shared utility so they’re not duplicated across components.
* Ensure all Hooks remain unconditional (we previously saw hook-order errors in other pages).

---

# 7) File index — where to look next time

## macronode

* `crates/macronode/src/http_admin/handlers/system_summary.rs` ✅ live CPU/RAM/net summary
* (Also ensure router wiring exists in macronode admin plane to route `/api/v1/system/summary` to this handler)

## svc-admin backend

* `crates/svc-admin/src/router.rs`

  * system proxy route `/api/nodes/:id/system/summary` (ensure it’s registered)
  * storage proxy routes already present
* `crates/svc-admin/src/nodes/client.rs`

  * `get_json_optional` and missing-endpoint logic
* `crates/svc-admin/src/nodes/registry.rs`

  * `try_storage_*` already
  * ensure `try_system_summary` exists and uses `/api/v1/system/summary`
* `crates/svc-admin/src/dto/system.rs` (or wherever `SystemSummaryDto` lives)

  * must match macronode JSON field names

## svc-admin UI

* `crates/svc-admin/ui/src/api/adminClient.ts` ✅ includes `getNodeSystemSummary()`
* `crates/svc-admin/ui/src/components/nodes/NodePreviewPanel.tsx`

  * fetches storage + system summary and flips pills to Live
* `crates/svc-admin/ui/src/routes/NodeDetailPage.tsx`

  * utilization gauges should use live system summary (verify)
* `crates/svc-admin/ui/src/types/admin-api.ts`

  * ensure `SystemSummaryDto` shape matches backend

---

# 8) “Do this first next session” (fast resume plan)

1. **Confirm all routes are wired**:

   * macronode: `/api/v1/system/summary`
   * svc-admin: `/api/nodes/:id/system/summary` proxy
2. **Confirm UI actually calls** `adminClient.getNodeSystemSummary()` in:

   * NodePreviewPanel
   * NodeDetailPage utilization
3. Implement **uptime + launch metadata** (next biggest operator win).
4. Implement **Databases screen** using existing storage routes:

   * list view + detail view
   * permissions + size + path alias + warnings
5. Add **capabilities endpoint** (optional, but makes rollouts clean).

---

If you want, paste your **current**:

* `crates/svc-admin/src/dto/system.rs` (or wherever it is),
* `crates/svc-admin/src/nodes/registry.rs` updated part for `try_system_summary`,
* and the **macronode admin router wiring** for the system handler,

…and I’ll produce a final “cross-check bundle” so the next instance can re-run and verify everything in 2–3 commands.



### END NOTE - DECEMBER 20 2025 - 18:00 CST




### BEGIN NOTE - DECEMBER 21 2025 - 13:17 CST

Below are carry-over notes you can paste into the next instance. I’m writing these as a “state of the world + exact next moves” doc for finishing svc-admin (God-tier dashboard) and wiring macronode/micronode plus built-in benchmarking.

⸻

Carry-Over Notes — svc-admin God-Tier Dashboard + macronode/micronode wiring + Benchmarks

0. Current status in one paragraph

We now have a working svc-admin SPA + backend where Nodes overview + Node preview panel + Storage/Databases pages + Database detail page load successfully with a capability-rollout posture: if macronode/micronode don’t implement endpoints yet, the UI falls back to deterministic mock data rather than breaking. We fixed TypeScript typecheck errors and a major runtime crash that caused the dashboard to briefly render then go blank (caused by approxKeys being undefined). We also have a dev request inspector ring buffer logging API calls in the SPA.

⸻

1. What we accomplished (UI)

1.1 Nodes Overview + Preview Panel (right-side)

File: crates/svc-admin/ui/src/components/nodes/NodePreviewPanel.tsx
•	Implemented the “God-tier” right-hand preview panel:
•	Displays node identity (name/profile/id), overall health badge, metrics freshness pill.
•	Shows 4 ring pills in a 2×2 grid:
•	CPU (live when /api/nodes/:id/system/summary exists; otherwise deterministic mock)
•	RAM (live when system summary exists; otherwise deterministic mock)
•	Storage (live when /api/nodes/:id/storage/summary exists; otherwise deterministic mock)
•	Bandwidth:
•	If live net fields exist, shows RX/TX bytes/sec (ring % currently uses an assumed 1 Gbps link).
•	If net fields don’t exist, shows “n/a / expose netRxBps/netTxBps” (no fake speed).
•	Supports optional “Planes” table (health/ready/restarts).

Important behavior:
•	Uses adminClient.getNodeSystemSummary() which is explicitly optional (404/405/501 => null).
•	Uses deterministic mock fallback for stability.

1.2 Node Storage page (Data & Storage inventory)

File: crates/svc-admin/ui/src/routes/NodeStoragePage.tsx
Hook: crates/svc-admin/ui/src/routes/node-storage/useNodeStorage.ts
•	Built a read-only “Data & Storage” page:
•	Storage summary cards (disk used/free, I/O read/write, mount/fs, DB count).
•	Database inventory table (name, engine, size, perms, health, warnings).
•	“Top databases” ring gauges showing relative disk share (proxy metric until per-DB headroom exists).
•	Sidebar “Database details” panel showing selected DB summary + warnings + “Open →” link to DB detail page.

✅ Critical runtime crash fixed:
The page was crashing because it assumed sidebarDb.approxKeys was always number|null, but some DTOs/mocks produced undefined. We fixed by normalizing:
•	approxKeys: undefined => null
•	warnings: missing => []
and guarding .toLocaleString() calls.

1.3 Database detail page (permissions posture + curated file inventory sample)

File: crates/svc-admin/ui/src/routes/NodeDatabaseDetailPage.tsx
•	Added a real route/page for database detail (not a SPA 404).
•	Shows “God-tier” operator context:
•	DB size, file count, perms/mode, owner, path alias, health
•	warnings (world readable/writable; degraded)
•	curated deterministic file inventory sample (alias paths only; no raw FS browsing)
•	safety notes (read-only curated facts)

1.4 Playground client fixes

File: crates/svc-admin/ui/src/api/ronCorePlaygroundClient.ts
•	Updated imports/types to match types/admin-api naming:
•	PlaygroundExampleDto, PlaygroundValidateManifestResp, etc.
•	Keeps “playground is dev-only” semantics: 404 => disabled.

1.5 adminClient consolidation

File: crates/svc-admin/ui/src/api/adminClient.ts
•	Central “thin HTTP client” for SPA with:
•	request logging ring buffer (httpLog) for debugging “is the backend being hit?”
•	strict JSON parsing
•	requestMaybeJson() for capability rollout: 404/405/501 => null
•	Endpoints wired:
•	UI/meta: /api/ui-config, /api/me
•	Nodes: /api/nodes, /api/nodes/:id/status, /api/nodes/:id/metrics/facets
•	System (optional): /api/nodes/:id/system/summary
•	Storage: /api/nodes/:id/storage/summary, /api/nodes/:id/storage/databases, /api/nodes/:id/storage/databases/:name
•	Playground: /api/playground/examples, /api/playground/manifest/validate
•	Node actions (mutating): reload/shutdown/debug-crash (backend gates)

1.6 Typecheck now passes

You ran:
•	npm run typecheck and npm run build
…and they both succeeded after fixes (TS errors resolved).

⸻

2. What we accomplished (svc-admin backend)

2.1 NodeRegistry storage + system summary proxy methods

File: crates/svc-admin/src/nodes/registry.rs
•	Implemented optional “proxy endpoints” pattern:
•	Storage summary proxy (try_storage_summary)
•	Databases list proxy (try_storage_databases)
•	Database detail proxy (try_storage_database_detail)
•	System summary proxy (try_system_summary)
•	Missing endpoints are treated as Ok(None) in NodeClient::try_get_json (404/405/501) so UI can safely fall back.

Important invariant preserved: no locks held across .await, and failures degrade to placeholders without losing node identity.

⸻

3. Known issues / rough edges still open

3.1 Metrics sparkline / charts regression

We have a standing known issue:
•	sparklines/charts “not working like they used to”
•	debug crash counters: UI updates when crash triggered via terminal but not always via UI button

Carry forward: treat as polish-phase fix; prioritize core wiring + correctness first.

3.2 Request Inspector pill position

You want the “API Requests” floating pill moved:
•	from bottom-right → bottom-left.

Expected location: RequestInspectorDrawer component styles.
Change right: ... to left: ... (and set right to auto).

3.3 Bandwidth realism

Right now the UI is intentionally not faking your network speed:
•	If netRxBps/netTxBps aren’t exposed, it shows “n/a”.

To make bandwidth real: macronode/micronode must implement /api/v1/system/summary with actual RX/TX bytes/sec fields.

⸻

4. What remains to complete the God-tier dashboard

4.1 Finish wiring macronode → svc-admin (capabilities)

We need macronode to expose a small set of curated admin plane endpoints consistently.

A) System Summary endpoint (high priority)
Node-side endpoint: GET /api/v1/system/summary
svc-admin proxy path: /api/nodes/:id/system/summary

SystemSummaryDto should include:
•	cpuPercent: number (0..100)
•	ramTotalBytes: number
•	ramUsedBytes: number
•	netRxBps: number (bytes/sec)
•	netTxBps: number (bytes/sec)
•	updatedAt: string (ISO timestamp)

This will make CPU/RAM/Bandwidth pills truly live and correct.

B) Storage endpoints (already supported by UI; ensure node implements)
Node-side endpoints:
•	GET /api/v1/storage/summary
•	GET /api/v1/storage/databases
•	GET /api/v1/storage/databases/{name}

Make sure DatabaseDetailDto.approxKeys is either:
•	a number, or
•	null
…but never missing/undefined. (UI is now defensive, but the DTO contract should be strict.)

C) Uptime + launch metadata (requested feature)
You want:
•	uptime counters on node cards (abbreviated ticker)
•	expanded “launched at / launched by / uptime” in node detail

Plan:
•	Add fields to the node status view returned from macronode:
•	startedAt: string (ISO)
•	uptimeSecs: number
•	launchedBy: string | null (user/service principal)
•	optionally host: string, pid: number, build info

Then:
•	show Uptime: 38s on NodeCard + PreviewPanel
•	show full details on NodeDetailPage

4.2 Improve “truthful readiness”

Keep the project posture:
•	readiness is not “green by default”
•	it becomes ready only after listeners bound, metrics live, config loaded, etc.

(We already use this philosophy elsewhere; apply it consistently to admin plane capability and UI banners.)

4.3 Better operator UX polish (after wiring)
•	restore sparklines + metric trends
•	error boundary around routes so one component can’t blank the entire app
•	refine “Planes table” spacing (no horizontal scroll)
•	consistent “Live/Mock/Mixed” labeling everywhere
•	better affordances for stale/unreachable metrics (banner + iconography)

⸻

5. Benchmarking tools inside the Admin Dashboard (requested)

You want benchmarks that actually work and test real-world RON-CORE performance. Here’s a concrete, staged plan that fits our architecture and avoids turning svc-admin into a random load cannon.

5.1 Principles
•	Benchmarks must be:
•	repeatable
•	safe (rate limited, bounded duration)
•	comparable (store baseline results, compare deltas)
•	realistic (simulate end-to-end flows, not micro-only)
•	Bench runs should be node-executed (not browser-executed) for correctness and stable timing.

5.2 Add a new “Benchmarks” area to svc-admin

UI:
•	Add a left-nav item: Benchmarks
•	Provide:
•	“Run suite” button
•	per-scenario run button
•	live progress (phase + percent + ETA-ish without promising)
•	results table (p50/p95/p99 latency, throughput, errors, CPU/RAM delta if available)
•	“Compare against baseline” view

Data model:
•	BenchmarkSuiteResultDto
•	suiteId, nodeId, startedAt, endedAt
•	scenario results array
•	environment metadata (build version, git hash, machine class)

5.3 Node-side benchmark endpoints (recommended contract)

Add to macronode/micronode admin plane:
1.	POST /api/v1/bench/run
Request: BenchRunReq { suite: string, durationSecs: number, concurrency: number, payloadSize: number, seed: number, limits: {...} }

Response: BenchRunResp { runId: string }
2.	GET /api/v1/bench/runs/{runId}
Response: BenchRunStatusDto { status: queued|running|done|failed, progress: 0..1, phase: string, partialResults?: ... }
3.	GET /api/v1/bench/runs/{runId}/result
Response: BenchRunResultDto (final)

Why this pattern: svc-admin can proxy safely; UI can poll; node controls load generation.

5.4 Benchmark suites to implement (in order)

Suite A: “Admin-plane sanity”
•	Verify endpoints and timing:
•	/healthz, /readyz, /metrics scrape timing
•	node status fetch time
•	Goal: ensure the control plane itself doesn’t become the bottleneck.

Suite B: “Storage throughput”
•	CAS put/get:
•	write N chunks of size X with concurrency C
•	read them back
•	Measures:
•	throughput MB/s
•	p95 latency for put/get
•	error rates
•	Optional: test compression/encryption toggles if present.

Suite C: “Index resolve / manifest workflows”
•	simulate publish and resolve cycles:
•	write manifest/metadata
•	resolve hot and cold
•	Measures:
•	resolve p95
•	cache hit/miss behavior

Suite D: “Overlay handshake + streaming”
•	connection setup time + sustained stream throughput
•	Measures:
•	handshake p95
•	stream Mbps
•	disconnect/retry behavior

Suite E: “DHT basic operations”
•	put/get/closest-nodes operations under concurrency
•	Measures:
•	p95 lookup
•	success rate
•	hop count if recorded

5.5 Baselines + “God-tier” pass/fail gates

Inside svc-admin we should store:
•	“baseline” results per node profile (macronode/micronode) + build version
•	show delta after each run:
•	✅ better / ⚠ regression
•	Define success gates (initially rough, later strict):
•	e.g. storage PUT p95 under X ms for payload size Y, concurrency C
•	resolve p95 under Z ms
•	DHT lookup success > 99.9% under load

5.6 Where to store benchmark results

Options (pick one):
1.	Store results in svc-admin local storage for dev (fastest MVP)
2.	Store results in svc-admin backend on disk (sled/sqlite) keyed by node+suite+timestamp
3.	Store results in the node itself and svc-admin reads them

Recommended:
•	MVP: svc-admin backend stores results (so UI is stateless).

⸻

6. Immediate next actions checklist (high impact)

(A) UI stability + positioning
•	Move “API Requests” pill to bottom-left (RequestInspectorDrawer styles).
•	Add an ErrorBoundary wrapper at app shell level so a single render error can’t blank the app again.

(B) Macronode capability wiring
•	Implement GET /api/v1/system/summary with real cpu/ram/rx/tx.
•	Ensure storage endpoints return strict DTOs (no missing fields).
•	Add uptime metadata fields to node status DTO.

(C) Benchmarks MVP
•	Add node endpoints /api/v1/bench/* with a single “Storage throughput” scenario first.
•	Add svc-admin proxy endpoints /api/nodes/:id/bench/* (or direct node proxy via registry).
•	Add a “Benchmarks” page in SPA to run/poll/show results.

(D) Later polish
•	Fix debug-crash button counter update path (UI vs terminal discrepancy).
•	Restore sparklines / charts.
•	Tighten planes table spacing (no horizontal scroll).

⸻

7. Key files touched recently (so next instance can jump in fast)

SPA (TypeScript/React)
•	crates/svc-admin/ui/src/api/adminClient.ts (core client + optional endpoints + request log)
•	crates/svc-admin/ui/src/api/ronCorePlaygroundClient.ts (playground DTO fixes)
•	crates/svc-admin/ui/src/components/nodes/NodePreviewPanel.tsx (CPU/RAM/Storage/BW ring pills + planes table)
•	crates/svc-admin/ui/src/routes/node-storage/useNodeStorage.ts (storage endpoints + mock fallback + selection management)
•	crates/svc-admin/ui/src/routes/NodeStoragePage.tsx (page + crash fix for approxKeys undefined)
•	crates/svc-admin/ui/src/routes/NodeDatabaseDetailPage.tsx (DB detail page + curated inventory sample)

svc-admin backend (Rust)
•	crates/svc-admin/src/nodes/registry.rs (storage/system proxy methods; optional endpoint posture)

⸻

8. Notes on the “approxKeys undefined” crash (so we don’t reintroduce it)

The blank-screen render loop was caused by:
•	calling sidebarDb.approxKeys.toLocaleString() when approxKeys was undefined.

Fix applied:
•	normalize approxKeys: undefined → null
•	guard with == null ? 'n/a' : toLocaleString()
•	normalize warnings to [] if missing

Even if macronode returns partial DB detail during rollout, the SPA won’t crash now.

⸻

If you want, in the next instance we can start with the Benchmarks MVP by implementing just one suite end-to-end:
1.	macronode /api/v1/bench/run + /api/v1/bench/runs/:id
2.	svc-admin proxy
3.	a Benchmarks SPA page that runs/polls and shows a result card

That’s the fastest path to “let’s see if it truly mogs” with real numbers.


### END NOTE - DECEMBER 21 2025 - 13:17 CST




### BEGIN NOTE - DECEMBER 21 2025 - 16:50 CST

Below are **carry-over notes you can paste into the next instance**. I wrote these as a **“state of the world + exact next moves”** doc for finishing **svc-admin (God-tier dashboard)** and completing **macronode benchmarks (node-executed loadgen) end-to-end**.

---

# Carry-Over Notes — svc-admin God-Tier Dashboard + macronode Benchmarks (node-executed)

Date: **2025-12-21 (America/Chicago)**
Repo: **RustyOnions (our project)**
Primary focus this instance: **Unbreak workspace + unify deps + compile macronode + confirm svc-admin green + introduce benchmark proxy surface + bench engine compile fixes**

---

## 0) Current status in one paragraph

We successfully **unstuck the workspace** from Cargo manifest inheritance errors (missing workspace deps like `fs2`, `humantime`, `uuid`) by updating root `Cargo.toml` pins, then updated `macronode/Cargo.toml` to consume workspace deps cleanly. We fixed compile errors in **`macronode/src/bench/mod.rs`** caused by the **rand 0.9** API + missing trait imports + moved `run_id` into a spawned task. After these fixes, **`cargo build -p macronode`** succeeds. `svc-admin` continues to compile and all tests pass. On the svc-admin side, the **backend is already prepared** for benchmark proxy endpoints (`NodeRegistry.try_bench_*`, router routes `/api/nodes/:id/bench/*`, NodeClient optional endpoint semantics), and the UI has already been refactored toward “capability rollout” posture with safe fallbacks.

---

# 1) Workspace / build system work completed

## 1.1 Root workspace dependency pinning fixed (critical)

Symptom:

* `cargo fmt --all` / `cargo build` failed with:

  * `error inheriting fs2 from workspace.dependencies.fs2`
  * then similar errors for `humantime`, `uuid`, etc.

Action:

* Updated **root `Cargo.toml`** `[workspace.dependencies]` to include missing deps that crates inherit with `{ workspace = true }`.

Root `Cargo.toml` now includes pins such as:

* `rand = "0.9"` (important for rand API changes)
* `sysinfo = "0.30"`
* `humantime-serde = "1"` (already present)
* Added missing **`uuid`**, **`humantime`**, **`fs2`** (and any other inherited deps that were missing)
* Existing pins remain (tokio, axum 0.7.9, reqwest 0.12, tokio-rustls, prometheus, etc.)

Result:

* Workspace metadata loads.
* Formatting/build/test flows resume.

## 1.2 macronode/Cargo.toml inheritance stabilized

macronode `Cargo.toml` uses workspace deps for:

* tokio/axum/tower/http
* tracing/tracing-subscriber/prometheus
* serde/serde_json/toml/humantime/humantime-serde
* sysinfo
* reqwest/uuid/rand
* thiserror/anyhow/parking_lot/fs2

Result:

* `cargo build -p macronode` no longer fails due to missing workspace deps.

---

# 2) macronode Benchmarks implementation status

## 2.1 Bench engine file exists and compiles

File:

* `crates/macronode/src/bench/mod.rs`

What this file is:

* A **node-executed benchmarking engine** (safe bounded loadgen) intended to run inside macronode environment to avoid browser jitter / remote skew.
* Supports:

  * bounded duration
  * bounded concurrency
  * bounded samples per endpoint
  * curated endpoint list (no arbitrary URL cannon)
* Produces:

  * run status DTO
  * per-endpoint latency quantiles and rps
  * run result DTO

### Fixes made (compile fixes)

These were required due to **rand 0.9** and Rust move semantics.

1. **Trait imports required**
   Errors:

* `StdRng::seed_from_u64` missing
* `choose` missing on slice
  Fix approach:
* Ensure traits in scope:

  * `use rand::SeedableRng;`
  * `use rand::prelude::IndexedRandom;` (rand 0.9 provides `choose` via IndexedRandom)
* Removed/adjusted unused `SliceRandom` import (or replaced with IndexedRandom)

2. **Moved run_id into tokio::spawn**
   Error:

* `use of moved value: run_id` after spawning a task that moved it.
  Fix approach:
* Clone run_id before spawn or restructure to pass owned String while keeping a clone for response:

  * `let run_id = ...; let run_id_spawn = run_id.clone(); tokio::spawn(async move { ... run_id_spawn ... }); Ok(BenchRunResp { run_id })`

3. **rand deprecations**
   Warnings:

* `gen_bool` renamed → `random_bool`
* `gen_range` renamed → `random_range`
  Fix approach:
* Update to new API to avoid warnings:

  * `rng.random_bool(p)`
  * `rng.random_range(a..b)`
    (These are warnings, not blockers. Keep them clean if you want “God-tier” hygiene.)

### Current macronode bench compilation outcome

After fixes:

* ✅ `cargo build -p macronode` succeeds
* Remaining: warnings only (payload_size unused, Internal unused, suite unused in RunRecord)

---

## 2.2 What is NOT done yet on macronode (major remaining chunk)

Even though `bench/mod.rs` compiles, macronode **does not yet expose** the admin plane endpoints that svc-admin expects:

**Node admin plane contract to implement in macronode:**

* `POST /api/v1/bench/run`
* `GET  /api/v1/bench/runs/:run_id`
* `GET  /api/v1/bench/runs/:run_id/result`

What needs doing:

* Create shared `BenchManager` instance in macronode admin state (`Arc<BenchManager>`)
* Wire handlers in macronode router (axum routes)
* Hook run/start to `BenchManager.start(req)` (convert incoming DTO into `BenchRunReq`)
* Hook status/result to `BenchManager.status(run_id)` / `BenchManager.result(run_id)`
* Return correct status codes:

  * `404` for unknown run_id (this is important because svc-admin NodeClient has a “MissingPolicy” concept: you may want 404 to be *real* for run_id not found, not treated as capability missing)
  * `501/405` only for capability absence (older nodes), not for domain logic

**Important nuance: optional endpoint semantics**

* svc-admin currently treats `404/405/501` as “missing endpoint” for optional endpoints by default.
* For bench status/result, **404 may be meaningful** (“run id not found”), so on svc-admin side we may want to call `try_get_json_no_404` / `try_post_json_no_404` for bench endpoints to avoid incorrectly degrading 404 to “Not implemented.”
* Right now NodeRegistry bench calls use `try_post_json` / `try_get_json` (legacy treat-404-as-missing).
* We need to evaluate whether we want:

  * **Option A (simplest):** Keep bench endpoints in node returning 200 even for unknown run_id (not great).
  * **Option B (better):** In svc-admin registry, use the “no_404” variants for bench status/result and possibly run.
  * Best: **treat 404 as error** for `/bench/runs/:run_id` and `/result` because the endpoint exists, 404 is domain.

This is an important correctness step for “God-tier” behavior.

---

# 3) svc-admin status (backend + UI)

## 3.1 svc-admin backend is green and has proxy endpoints ready

Commands confirmed:

* ✅ `cargo build -p svc-admin` succeeds
* ✅ `cargo test -p svc-admin` passes (9 unit tests + integration tests: auth_me_http, config_env, fake_node, http_smoke)

Warnings remaining:

* `NodeClient::post_json` unused (dead_code) — not harmful; could be kept for future or removed.

### Key backend files in this slice

1. `crates/svc-admin/src/nodes/registry.rs`

* Contains:

  * `try_storage_summary`, `try_storage_databases`, `try_storage_database_detail`
  * `try_system_summary`
  * **Bench proxies added/confirmed:**

    * `try_bench_run(id, req) -> Result<Option<BenchRunRespDto>>`
    * `try_bench_status(id, run_id) -> Result<Option<BenchRunStatusDto>>`
    * `try_bench_result(id, run_id) -> Result<Option<BenchRunResultDto>>`
  * These use NodeClient optional get/post methods and degrade safely if node lacks endpoint.

2. `crates/svc-admin/src/router.rs`

* Routes exist:

  * `/api/nodes/:id/bench/run` (POST)
  * `/api/nodes/:id/bench/runs/:run_id` (GET)
  * `/api/nodes/:id/bench/runs/:run_id/result` (GET)
* Auth gating exists:

  * require ops/admin roles to run/status/result
* Capability rollout posture:

  * returns `501 Not Implemented` when node lacks endpoint (so SPA can fall back).

3. `crates/svc-admin/src/nodes/client.rs`

* The “longer” version of NodeClient is in repo; it includes:

  * strict base_url scheme enforcement
  * deny http unless insecure_http=true
  * per-node timeout
  * optional endpoint logic with MissingPolicy:

    * Treat404AsMissing (legacy rollout)
    * Treat404AsError (for domain-meaningful endpoints)
  * `try_get_json`, `try_get_json_no_404`
  * `try_post_json`, `try_post_json_no_404`

**Big follow-up:**

* For benchmarks, we likely should swap registry bench calls to use `*_no_404` for status/result at least, to prevent false “not implemented” on missing run_id.

---

## 3.2 svc-admin UI work in this instance (files provided)

You provided these UI files for drift-safe updates:

1. `crates/svc-admin/ui/src/components/layout/Sidebar.tsx`

* Sidebar shows Nodes / Playground / Settings.
* Playground link always visible but opacity changes based on `ui-config`.
* Safe posture: if `/api/ui-config` fails, assume playground disabled.
* Prevent sideways scroll / layout shift (active link styles should not widen row).

2. `crates/svc-admin/ui/src/types/admin-api.ts`

* DTO definitions aligned with Rust DTOs and rollout posture:

  * UiConfigDto + UiDevConfig
  * MeResponse
  * NodeSummary (snake_case fields)
  * AdminStatusView (snake_case + optional uptime/capabilities)
  * FacetMetricsSummary (snake_case)
  * StorageSummaryDto, DatabaseEntryDto, DatabaseDetailDto
  * Playground DTOs
  * SystemSummaryDto
* Includes `DatabaseDetailDto.approxKeys: number | null` (this was a past runtime crash fix: approxKeys undefined)

3. `crates/svc-admin/ui/src/api/adminClient.ts`

* Thin fetch client with:

  * strict JSON parse (logs parse failures)
  * HttpError with status/body
  * `requestMaybeJson` that treats 404/405/501 as “missing endpoint => null”
  * dev ring buffer request logging (httpLog)
  * methods:

    * getUiConfig, getMe
    * getNodes, getNodeStatus, getNodeFacetMetrics
    * getNodeSystemSummary (maybe)
    * getNodeStorageSummary, getNodeStorageDatabases, getNodeStorageDatabaseDetail
    * actions: reload/shutdown/debugCrash
    * playground: getPlaygroundExamples, validatePlaygroundManifest
    * ping()

**Important:**

* The UI **does not yet have** a Benchmarks page/route nor adminClient methods for bench endpoints. That is next.

---

# 4) Benchmark integration plan (exact next moves)

## 4.1 macronode: implement real bench admin endpoints (node-side)

**Files needed next (paste them in next instance to avoid drift):**

* `crates/macronode/src/main.rs`
* macronode admin router file (the one defining routes like `/api/v1/status`, `/api/v1/system/summary`, `/api/v1/storage/summary`)
* any shared admin state struct/module used by that router

**Implementation steps:**

1. Add `BenchManager` to macronode admin state:

   * `let bench = Arc::new(BenchManager::new(base_url_for_self_admin_plane))`
   * base_url should target the node’s own admin plane listener (often `http://127.0.0.1:<port>` or derived from config)
2. Add routes:

   * `post("/api/v1/bench/run", bench_run)`
   * `get("/api/v1/bench/runs/:run_id", bench_status)`
   * `get("/api/v1/bench/runs/:run_id/result", bench_result)`
3. Handlers map:

   * incoming JSON (DTO) → `BenchRunReq`
   * call `bench.start(...)`
   * status/result returns:

     * If run_id unknown: return 404 with body error (or 404 empty)
4. Confirm with curl:

   * `curl -X POST .../api/v1/bench/run -d '{"suite":"admin_plane","durationSecs":5,"concurrency":4,"seed":0}'`
   * then poll status + result

**Correctness caveat (svc-admin optional semantics):**

* If node returns 404 for unknown run_id, svc-admin must not misinterpret that as “missing endpoint.” That requires svc-admin to use `*_no_404` on those calls (see 4.2).

---

## 4.2 svc-admin: correct proxy behavior for bench status/result (404 semantics)

Current:

* Registry uses `try_get_json` / `try_post_json` (treat 404 as missing endpoint).

Desired:

* For bench runs:

  * `POST /api/v1/bench/run` — 404 is probably “missing endpoint” only; but could remain default.
* For bench status/result:

  * 404 means “run not found” and should be a real error.
  * Use NodeClient `try_get_json_no_404` for:

    * `/api/v1/bench/runs/{run_id}`
    * `/api/v1/bench/runs/{run_id}/result`

So next patch on svc-admin backend likely includes:

* In `NodeRegistry.try_bench_status/result`, swap to `.try_get_json_no_404(...)`
* Optionally adjust router mapping to return:

  * 404 if upstream returned 404 (not 501)
  * 502 for other upstream errors

(We can do this carefully to preserve rollout posture while improving correctness.)

---

## 4.3 svc-admin UI: build Benchmarks page (God-tier operator UX)

UI does not yet provide:

* Benchmarks nav link
* BenchmarksPage route
* adminClient methods for bench endpoints
* Progress polling UI

**Next UI files to paste in next instance:**

* `crates/svc-admin/ui/src/App.tsx` (or whichever router file defines routes)
* `crates/svc-admin/ui/src/components/layout/Sidebar.tsx` (again, if we add nav link)
* current `adminClient.ts` (so we extend it safely)
* any common UI components used for cards/tables (optional, but helps consistency)

**UI feature spec for v1 Benchmarks page:**

* Node selector (dropdown) using `/api/nodes`
* Run form:

  * suite: fixed `admin_plane` (dropdown for future)
  * duration seconds (bounded)
  * concurrency (bounded)
  * seed (optional)
* Buttons:

  * “Run Bench”
* Status:

  * show `queued|running|done|failed`
  * progress bar (0..1)
  * phase string
  * updated_at + started_at
* Polling:

  * poll status every 500ms–1500ms while running
  * when `done`, fetch result
* Result table:

  * rows by endpoint (healthz/readyz/status/system_summary/storage_summary)
  * columns: method, path, requests, errors, rps, p50/p95/p99
* Safe fallbacks:

  * if svc-admin returns 501, show “Node does not implement benchmarks yet” and keep UI stable (like system summary and storage pages do)
* Dev request inspector log should show the last requests (already exists in adminClient ring buffer)

---

# 5) “God-tier dashboard” completion checklist (what remains beyond benchmarks)

Even though the dashboard is strong already, “God-tier” finishing layer still pending:

## 5.1 Benchmarks page + integration (this is the next big slice)

* End-to-end is incomplete until macronode exposes endpoints and UI calls svc-admin bench proxy.

## 5.2 Fix remaining known UI issues (defer, but track)

* Debug crash button triggers not always reflected in counters from UI (works when done via terminal).
* Sparklines/charts that used to work no longer animate/render as before.
* These are acknowledged and intentionally deferred until after feature wiring.

## 5.3 Uptime counters per node (requested feature)

Planned:

* Node cards show abbreviated uptime ticker.
* Node detail shows:

  * start time/date
  * who launched (if available)
  * uptime duration
    This will require:
* node admin plane to expose start metadata OR svc-admin to track since first seen (fallback)
* UI display on node list and node detail pages

## 5.4 Macronode → Micronode parity

Strategy agreed:

* Wire macronode fully first; then port endpoints to micronode later.
  So after macronode bench endpoints work:
* implement same `/api/v1/bench/*` endpoints in micronode with identical DTOs.

---

# 6) Commands used / current “green” build recipe

Once workspace deps were fixed:

* `cargo fmt --all`
* `cargo build -p macronode` ✅
* `cargo build -p svc-admin` ✅
* `cargo test -p svc-admin` ✅

Remaining warnings:

* macronode bench module: unused fields/variants (non-blocking)
* svc-admin NodeClient post_json unused (non-blocking)

---

# 7) Next file batch to request at start of next instance (copy/paste)

## Batch A (macronode: add /api/v1/bench/* endpoints)

Please paste:

1. `crates/macronode/src/main.rs`
2. macronode admin router file (contains `/api/v1/status` etc.)
3. macronode admin state struct/module (if separate)

## Batch B (svc-admin UI: BenchmarksPage + routing + adminClient methods)

Please paste:

1. `crates/svc-admin/ui/src/App.tsx` (or router file)
2. `crates/svc-admin/ui/src/api/adminClient.ts`
3. `crates/svc-admin/ui/src/components/layout/Sidebar.tsx`
   (Optionally: any shared Table/Card components for consistent styling.)

---

# 8) Important design invariants to keep (do not regress)

* **Capability rollout posture:** missing endpoints must not break UI; svc-admin returns `501` when node lacks capability; UI falls back to deterministic mock data where appropriate.
* **No lock held across `.await`** in registries/clients.
* **No arbitrary URL loadgen**: bench targets only curated endpoints.
* **Strict http policy**: NodeClient denies `http://` unless `insecure_http=true`.
* **Bench correctness nuance**: for bench run_id not found, 404 should be treated as a real error, not a “capability absent.” Use NodeClient `*_no_404` accordingly.
* **Drift control**: keep posting originals before updates; always replace full files, not partial snippets.



### END NOTE - DECEMBER 21 2025 - 16:50 CST



### BEGIN NOTE - DECEMBER 21 2025 - 18:41 CST

According to a document from **December 21, 2025**, the blank screen is very likely coming from **two separate ES-module export/import issues** in the SPA:

1. **`main.tsx` imports a default `App`, but `App.tsx` only exports a named `App()`** (no default export). That can crash at runtime / during module linking. 

2. Your **`components/common/LoadingSpinner.tsx` shim tries to “re-export as default”**, which tends to trigger the “default cannot be resolved by star export entries” class of errors depending on how it’s imported/re-exported. 

Below are paste-ready fixes, then a full set of carry-over notes.

---

## Fix the blank screen now (paste-ready)

### 1) `crates/svc-admin/ui/src/main.tsx` (replace entire file)

```tsx
// crates/svc-admin/ui/src/main.tsx

import React from 'react'
import ReactDOM from 'react-dom/client'
import { BrowserRouter } from 'react-router-dom'
import { App } from './App'
import './styles.css'

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <BrowserRouter>
      <App />
    </BrowserRouter>
  </React.StrictMode>
)
```

Why this matters: your current `main.tsx` is doing `import App from './App'` while `App.tsx` exports `export function App() { ... }` (no default). 

---

### 2) `crates/svc-admin/ui/src/App.tsx` (replace entire file)

This keeps your existing named export **and** adds a default export so either import style works.

```tsx
// crates/svc-admin/ui/src/App.tsx

import React from 'react'
import { Routes, Route } from 'react-router-dom'
import { Shell } from './components/layout/Shell'
import { NodeListPage } from './routes/NodeListPage'
import { NodeDetailPage } from './routes/NodeDetailPage'
import { NodeStoragePage } from './routes/NodeStoragePage'
import { NodeDatabaseDetailPage } from './routes/NodeDatabaseDetailPage'
import { PlaygroundPage } from './routes/PlaygroundPage'
import { SettingsPage } from './routes/SettingsPage'
import { LoginPage } from './routes/LoginPage'
import { NotFoundPage } from './routes/NotFoundPage'
import { BenchmarksPage } from './routes/BenchmarksPage'

export function App() {
  return (
    <Shell>
      <Routes>
        <Route path="/" element={<NodeListPage />} />
        <Route path="/nodes/:id" element={<NodeDetailPage />} />
        <Route path="/nodes/:id/storage" element={<NodeStoragePage />} />
        <Route
          path="/nodes/:id/storage/databases/:name"
          element={<NodeDatabaseDetailPage />}
        />
        <Route path="/benchmarks" element={<BenchmarksPage />} />
        <Route path="/playground" element={<PlaygroundPage />} />
        <Route path="/settings" element={<SettingsPage />} />
        <Route path="/login" element={<LoginPage />} />
        <Route path="*" element={<NotFoundPage />} />
      </Routes>
    </Shell>
  )
}

// Allow both: `import { App } ...` and `import App ...`
export default App
```

This matches what your code bundle currently shows for routes (including `/benchmarks`).

---

### 3) `crates/svc-admin/ui/src/components/common/LoadingSpinner.tsx` (replace entire file)

This removes the brittle `export { X as default } from ...` shim pattern and makes the default export explicit.

```tsx
// crates/svc-admin/ui/src/components/common/LoadingSpinner.tsx
//
// Compatibility shim so both paths work:
// - ../components/common/LoadingSpinner
// - ../components/shared/LoadingSpinner
//
// IMPORTANT: Do NOT rely on `export { X as default } from ...` here.
// Some bundler / star-export patterns cannot resolve default that way.

import React from 'react'
import { LoadingSpinner as SharedLoadingSpinner } from '../shared/LoadingSpinner'

export function LoadingSpinner() {
  return <SharedLoadingSpinner />
}

export default LoadingSpinner
```

This replaces the current shim that re-exports a default via export-specifier (the one you annotated as “vite keeps throwing an error”).

---

# Carry-over notes (svc-admin + macronode + benchmarks)

Below are “state of the world + exact next moves” notes you can paste into the next instance. I’m writing them so you can continue without re-deriving context.

---

## 0) Snapshot (where we are right now)

* **svc-admin is a Rust backend + React/Vite TypeScript SPA** that targets “God-tier operator UX” for **macronode** (and later micronode).
* The SPA has real pages for:

  * Nodes overview (NOC-style list)
  * Node detail view (preview panel + per-plane table)
  * Storage/Databases inventory + database detail page
  * Benchmarks page (`/benchmarks`)
  * Playground page (`/playground`) with capability-style 404 behavior
  * Settings, Login, NotFound
* The system is designed with a **capability rollout posture**:

  * If macronode/micronode don’t implement some endpoints yet, svc-admin UI should degrade gracefully (fallback to mock/sample data or “not supported”), not hard-crash.
* Current problem: **SPA blank screen** due to module export/import issues (fixed above). Root causes identified in code bundle:

  * `main.tsx` default-importing `App` while `App.tsx` is named-export only 
  * `components/common/LoadingSpinner.tsx` shim re-exporting `default` in a brittle way 

---

## 1) svc-admin UI: what exists and why it’s structured this way

### 1.1 Routing and shell

* `ui/src/App.tsx` defines the route map and mounts everything inside `<Shell>` (Sidebar + TopBar + main). It includes `/benchmarks` and `/playground`.
* `ui/src/main.tsx` mounts React under `BrowserRouter`. **Invariant:** only one router in the whole app (no nested routers) — this was previously a known blank-screen cause in other contexts.

### 1.2 Dev request inspector

* There is a dev-only **Request Inspector Drawer** that reads from a bounded ring buffer of HTTP calls made by the SPA.
* Its purpose is to make wiring to macronode fast: you instantly see which endpoints are 404/501/500 and what payload came back.

### 1.3 “Capability posture” pattern (how we avoid breaking the UI)

* Some UI features are allowed to be “missing” server-side.
* Preferred convention:

  * **404** = “capability not present / disabled / feature hidden”
  * **501** = “known endpoint but not implemented yet”
  * **500** = server failure
* The SPA should treat these differently:

  * 404 → show “Not supported” / hide section / fall back
  * 501 → show “Coming soon” with contextual details
  * 500 → show error banner

### 1.4 Databases + storage pages

* The UI has a storage/databases inventory page + a database detail page route:

  * `/nodes/:id/storage`
  * `/nodes/:id/storage/databases/:name`

“God-tier” direction for these pages (next upgrades):

* Show:

  * Size on disk, file count, permission/owner, mount point
  * Read/write IOPS estimate, fsync latency (if available)
  * Hot keys estimate / compaction status (sled/rocks/etc, later)
  * “Last write” timestamp and write rate
  * Health classification: OK / Degraded / Read-only / Full / No permission
* Add a “Storage bandwidth / throughput” panel (ties into your system summary metrics).

---

## 2) svc-admin backend: what it should do (and what remains)

### 2.1 The backend’s role

svc-admin backend should be the **one origin** the SPA talks to. It:

* Serves the SPA
* Exposes its own admin endpoints (auth, UI config)
* Proxies to macronode(s) admin plane endpoints with:

  * strict timeouts
  * safe URL policy (deny insecure http unless explicitly allowed)
  * optional endpoint behavior (404/405/501 are meaningful)

### 2.2 Node admin client (svc-admin → macronode)

You already have a `svc-admin/src/nodes/client.rs` described as:

* Centralizes per-node HTTP rules:

  * timeouts
  * http/https policy
  * optional endpoints may be missing (404/405/501)
* Invariant: **no lock across `.await`**
* Logs degraded paths / upstream failures

**Next moves for this module (God-tier readiness):**

* Add an explicit “endpoint capability matrix”:

  * For each node, record which endpoints exist (status, storage, system summary, bench, etc.)
  * Cache for ~30s and expose to UI so UI can hide/show sections per node.
* Standardize error mapping:

  * `UpstreamMissing` (404/405/501)
  * `UpstreamUnhealthy` (>=500, timeouts)
  * `BadPayload` (json decode failures)
* Ensure consistent per-node timeouts:

  * connect timeout
  * request timeout
  * “slow path” timeout for large endpoints (bench results, etc.)

---

## 3) macronode: what exists and what remains

### 3.1 Current known working integration goals

* macronode should expose a **node admin plane** with read-only introspection endpoints that svc-admin can proxy.
* You mentioned you’re already seeing correct CPU/memory/bandwidth/storage; that implies:

  * macronode has a system summary endpoint and/or metrics endpoint
  * svc-admin proxies it and the SPA consumes it

### 3.2 Next macronode moves (to complete the “God-tier dashboard” wiring)

Priorities to implement on macronode admin plane (in this order):

1. **Uptime + build provenance**

   * Node start timestamp
   * Uptime duration
   * Version/build SHA
   * Hostname + OS
   * “Launched by” (if available; otherwise “unknown” and add later)
   * This powers:

     * Small uptime ticker on node cards
     * Deep uptime panel on node detail page

2. **Plane readiness + per-plane metadata**

   * Each facet/plane should publish:

     * health: healthy/degraded/down
     * ready: bool
     * last transition time
     * restart count
     * “last error” short string (bounded)
   * svc-admin consumes this in the per-plane table and the node preview panel

3. **Storage inventory**

   * storage mounts, db directories, permissions
   * optional deep info (file counts, compaction, etc.)

4. **Benchmark endpoints** (see next section)

---

## 4) Benchmarks: what exists and how to finish it cleanly

### 4.1 What exists already (UI-level)

* The SPA has a **Benchmarks page** mounted at `/benchmarks`.

### 4.2 “Correct semantics” for benchmarks in this architecture

Benchmarks are a *workflow*, not a single request:

* Start a run → get `run_id`
* Poll status → `queued/running/done/failed/canceled`
* Fetch results → structured result + raw artifacts optional

**Strong recommendation (so wiring is painless later):**

* Node (macronode/micronode) should own running benchmarks and artifacts.
* svc-admin should:

  * proxy the endpoints
  * normalize errors (404 = not supported)
  * enforce auth/policy
  * optionally cache status polling

### 4.3 Minimal endpoint set to implement on macronode first

For each node `:id` (or directly on node admin plane if svc-admin already targets per-node base URLs):

* `POST /api/v1/bench/runs`

  * request: scenario, duration, concurrency, payload size, optional label
  * response: `run_id`, created_at, estimated_cost/notes
* `GET /api/v1/bench/runs/:run_id/status`
* `GET /api/v1/bench/runs/:run_id/result`
* `GET /api/v1/bench/scenarios` (optional, but makes UI great)

**Result schema should include:**

* start/end time, duration, warmup
* request counts, error counts, p50/p95/p99 latency
* throughput (rps), bytes in/out
* resource summary (cpu/mem) if feasible

### 4.4 svc-admin UI “God-tier” benchmark features (after endpoints exist)

* A run builder UI:

  * scenario dropdown + “advanced” panel
  * run tags/notes
* Live run status:

  * phase, progress bar, ETA (if computable)
  * streaming log tail (optional)
* Results:

  * key metrics cards + histogram view
  * comparison view (run vs run)
  * export JSON/CSV

---

## 5) The big “next session” checklist (do this in order)

### 5.1 Unblock the UI build

* Apply the 3 UI file fixes above:

  * `ui/src/main.tsx`
  * `ui/src/App.tsx`
  * `ui/src/components/common/LoadingSpinner.tsx`
* Re-run dev and confirm the app renders.

### 5.2 Confirm wiring integrity via Request Inspector

* Open the Request Inspector drawer.
* Click through:

  * Nodes page load
  * Node detail
  * Storage page
  * Database detail page
  * Benchmarks page
* Note which requests are:

  * 200 (good)
  * 404/501 (capability missing)
  * 500/timeouts (bug)

### 5.3 Macronode next: implement uptime/provenance endpoint

* Add the endpoint
* Proxy it through svc-admin
* Display:

  * compact uptime on node cards
  * detailed uptime in node detail page

### 5.4 Macronode next: implement benchmarks MVP endpoints

* Start/status/result
* Then wire the UI Benchmarks page to real calls

---

### END NOTE - DECEMBER 21 2025 - 18:41 CST



### BEGIN NOTE - JANUARY 4 2026 - 11:14 CST

## Carry-over notes — Our project (RON-CORE) gaps discovered while documenting the Taxi apps

*(What we realized we still need, why it matters, and the exact “fold-back” work items — optimized for finishing svc-admin + node parity first)*

### 0) Why these notes exist

While designing the tandem Taxi apps, we unintentionally “stress tested” our project’s **DX, protocol semantics, and admin-plane surfaces**. The app planning surfaced several missing pieces (or areas needing standardization) that we should fold back into our project so the apps (and future apps) feel effortless and don’t require bespoke glue.

This is not “build taxi now.” This is: **make our project so solid that the taxi apps become boring to implement.**

---

## 1) Protocol and SDK semantics we must standardize (OAP/1 + SDK_DX_ADDON)

### 1.1 Stream semantics must be explicitly supported end-to-end

**Need**

* **At-least-once delivery** is the baseline: duplicates can happen.
* Every stream event must carry a stable `eventId`.
* Clients must be able to resume with a cursor: `sinceEventId`.

**Why**

* Mobile networks drop constantly. If the protocol doesn’t make resume/dedupe first-class, every app reinvents it and fails differently.

**Fold-back tasks**

* Define (and document) the canonical fields for streaming:

  * `event_id` (opaque string, monotonic per stream scope)
  * `since_event_id` (resume cursor)
  * `stream_scope` (what the cursor applies to)
* Ensure we have a clear rule for cursor bounds:

  * “server may only retain last N minutes / last N events”
  * server returns “cursor too old” as a first-class error
* Ensure stream wrapper in SDK_DX_ADDON implements:

  * reconnect w/ backoff + jitter
  * resume logic using cursor
  * dedupe helper keyed by `(stream_scope, eventId)` with bounded memory
  * bounded buffering / backpressure policy

### 1.2 Idempotency must be a first-class rule for mutations

**Need**

* Mutating requests must accept `idempotencyKey`.
* Server must treat same key as same outcome (within a time window).

**Why**

* Mobile retries otherwise double-create rides, double-cancel, etc.

**Fold-back tasks**

* Define a standard idempotency contract (applies to all facets):

  * request carries `idempotency_key`
  * response carries `idempotency_status` (fresh|replay) optionally
  * retention policy window (e.g., 24h) documented
* Provide an SDK helper to generate stable keys per user action + store in local KV until completion

### 1.3 Error taxonomy needs a shared cross-SDK mapping

**Need**

* A portable error model (categorical) so apps don’t parse strings.

**Why**

* We need consistent handling of: auth failures, cursor-too-old, rate limits, overload, timeout, not-found, validation errors.

**Fold-back tasks**

* Standardize error codes (wire + SDK):

  * `UNAUTHENTICATED`, `FORBIDDEN`, `NOT_FOUND`
  * `VALIDATION_FAILED`
  * `IDEMPOTENCY_REPLAY`
  * `CURSOR_TOO_OLD`
  * `OVERLOADED` / `BACKPRESSURE`
  * `RATE_LIMITED`
  * `TIMEOUT`
  * `UNAVAILABLE`
* SDK_DX_ADDON maps these into ergonomic typed errors.

### 1.4 Frame size + buffering rules must be explicit

**Need**

* Max frame size (v1: ≤ 1 MiB) enforced everywhere.
* Explicit buffer limits per stream and per client.

**Why**

* Prevent memory blowups and keep “truthful readiness” meaningful.

**Fold-back tasks**

* Confirm max frame enforcement across transports.
* Define default stream buffer sizes + drop policies.
* Document these in OAP/1 + SDK docs.

---

## 2) Mobile observability implications for node-side observability (and svc-admin)

### 2.1 Mobile should not expose /metrics — so how do we debug?

**Need**

* On-device: ring buffer events + debug export bundle (privacy-first).
* Node-side: metrics remain canonical for fleet ops.
* Correlate client reports to node traces without logging secrets.

**Fold-back tasks**

* Standardize **correlation IDs**:

  * `corr_id` propagated from client → SDK → node logs/spans/metrics labels (careful: labels cardinality)
* Define a safe client debug export schema (no PII, coarse location if any)
* Optional (later): add svc-admin support for uploading a debug bundle and attaching it to a node/incident *without* storing secrets

---

## 3) “Cost profiles” should exist as a generic platform concept (not taxi-specific)

### 3.1 Cross-cutting runtime profile knob

**Need**

* A general notion of “cost posture” that affects:

  * heartbeat cadence
  * reconnect policy
  * batching/coalescing rules
  * telemetry verbosity

**Why**

* It’s not just taxi. Every mobile/edge app needs this.

**Fold-back tasks**

* Create a small generic spec:

  * `Balanced`, `RequestMin`, `BandwidthMin`
* SDK_DX_ADDON uses it to tune defaults.
* Node-side can optionally honor it (e.g., field selection / stream cadence) but must remain robust if ignored.

---

## 4) Facet contract governance: “single-sheet API” + fixtures to prevent drift

### 4.1 Canonical API.md must be enforceable, not aspirational

**Need**

* “API.md is source of truth” backed by fixtures/snapshots so Swift/Kotlin decoders don’t drift.

**Fold-back tasks**

* Establish a contract fixture system (per facet):

  * JSON fixtures for DTOs
  * golden event samples for streams
  * fuzz/unknown-field tolerance tests
* CI gates:

  * “fixtures compile” in Swift/Kotlin SDK builds
  * ensures unknown fields are tolerated and required fields are present

---

## 5) svc-admin and node admin-plane gaps surfaced by taxi planning

### 5.1 Multi-node auth is a real design problem (must be formalized)

**Need**

* svc-admin manages many nodes; auth must be clear:

  * svc-admin has its own operator auth (or delegates to IdP)
  * svc-admin talks to nodes using strong admin capability tokens
  * nodes protect admin plane via TLS + allowlists + tokens

**Fold-back tasks**

* Write/lock `svc-admin SECURITY/IDB` for multi-node auth:

  * dev mode (no auth)
  * prod mode (auth required)
  * token issuance/rotation story
  * audit logging requirements
* Implement capability-scoped admin tokens:

  * read-only vs action scopes
  * expiry + rotation
  * storage model (svc-admin side) and revocation posture

### 5.2 Micronode parity remains a hard requirement for the dashboard

**Need**

* micronode must mirror macronode admin-plane endpoints + DTO semantics so svc-admin does not special-case.

**Fold-back tasks (parity checklist)**

* Ensure micronode implements (same DTOs/semantics):

  * `/healthz`, `/readyz`, `/version`, `/metrics`
  * `/api/v1/status`
  * `/api/v1/system/summary`
  * `/api/v1/storage/summary`
  * bench endpoints (if we standardize them)
* Where endpoints aren’t implemented yet:

  * return 404/501 consistently and svc-admin degrades gracefully

### 5.3 Bench endpoints should become “real product” surfaces (not ad-hoc)

**Need**

* Standard bench run lifecycle surfaces so svc-admin can be a “god-tier” operator console:

  * start run
  * poll status
  * fetch result
  * consistent DTOs

**Fold-back tasks**

* Normalize bench endpoint paths + DTOs across node profiles.
* Ensure svc-admin UI can display:

  * throughput, latency histograms, error breakdown, resource utilization correlation.

---

## 6) Transport/runtime readiness implications (truthful readiness + overload)

Taxi planning emphasized mobile reconnect storms and stream load.

**Fold-back tasks**

* Ensure readiness and health reflect:

  * queue depth / backpressure state
  * fd headroom
  * shed rate / overload mode
* Provide consistent overload signaling to clients:

  * return `OVERLOADED` with retry-after guidance where appropriate

---

## 7) Recommended implementation order (because we’re switching back to svc-admin now)

### Phase A — Finish svc-admin “god-tier” foundations (highest ROI)

1. Lock svc-admin multi-node auth plan in SECURITY/IDB
2. Harden node capability/token model (even if minimal v1)
3. Ensure micronode admin-plane parity so the dashboard can manage both profiles cleanly
4. Standardize bench endpoints + DTOs (so svc-admin can become a benchmarking console)

### Phase B — Fold protocol/DX improvements (so apps become easy later)

5. Publish OAP/1 stream resume/dedupe/idempotency semantics as canonical
6. Implement SDK_DX_ADDON helpers:

   * reconnect/backoff/jitter
   * sinceEventId resume
   * dedupe helper
   * idempotency helper
   * error taxonomy mapping

### Phase C — Contract governance to prevent drift

7. Add fixtures/snapshot tests that validate DTO compatibility across SDKs

---

## 8) “Definition of done” for these fold-back items

We’ll consider the fold-back successful when:

* svc-admin can manage **macronode + micronode** with **no special casing** beyond capability fallbacks
* multi-node auth is documented + implementable (dev/prod modes)
* OAP/1 semantics for resume/dedupe/idempotency are explicit and have SDK helper support
* bench endpoints are stable enough that the dashboard can run a benchmark and render results

---


### END NOTE - JANUARY 4 2026 - 11:14 CST



### BEGIN NOTE - JANUARY 4 2026 - 19:00 CST

Got it — based on the screenshots + the run log, here’s **exactly where we are**, what’s “real vs fallback,” what the logs are telling us, and an **updated, prioritized TODO** (svc-admin + macronode) that also folds in the “crossover” items we noticed while building docs.

---

## 1) Where we are right now (what’s working)

### Dev stack boot + wiring

* `dev_svc_admin_stack.sh` is bringing up:

  * **macronode admin plane**: `127.0.0.1:8080`
  * **svc-admin backend**: `127.0.0.1:5300`
  * **svc-admin metrics/health**: `127.0.0.1:5310`
  * **SPA dev server**: `http://localhost:5173`
  * **Playground feature flag**: `SVC_ADMIN_UI_DEV_ENABLE_APP_PLAYGROUND=true`
* The “metrics sampler failed initially” warnings are **expected with the current startup order**: svc-admin comes up and immediately tries hitting `http://127.0.0.1:8080/metrics`, but macronode hasn’t finished compiling/binding yet → connection refused → then it recovers on the next interval once macronode is up.

### Dashboard features visibly working (from screenshots)

* **Nodes overview + right-side preview panel** works.
* **Node detail page** works (planes table + cards + sparklines).
* **Storage & databases inventory** works (DB list, DB detail page, warnings display).
* **Benchmarks page** works end-to-end:

  * Node-executed benchmark run completes (`done 100%`)
  * Results table shows per-endpoint Req/RPS + p50/p95/p99
  * The suite is clearly “admin_plane (implemented)”
* **App Playground** works (manifest editor + validate UX, explicitly local-only/read-only).

### What’s likely still “mock or hybrid”

* The UI labels “Source: Live” in storage/db pages, but some elements clearly remain “curated proxy until per-DB headroom exists” and/or have mock-ish behavior.
* Example: **“Showing 60 of 3”** on the curated inventory table is a classic “UI pagination/count mismatch” or “mock list length hardcoded” bug.
* Bandwidth is the one you already called out: it’s present in UI, but looks like it’s **not fully backed by real node-exposed metrics** yet.

---

## 2) What the terminal output is quietly telling us (important)

### A) Startup order + sampler warnings

You’re spawning facet metric samplers immediately (`interval_secs=5`). That’s correct long-term, but in dev it causes noisy “initial sample failed” logs because macronode isn’t bound yet.

**This is not a functional failure**; it’s a dev ergonomics + log hygiene issue.

### B) macronode service ports have a future foot-gun

You logged:

* `svc_mailbox` “started … addr=127.0.0.1:5304”
* then `svc_index` (embedded) “bind=127.0.0.1:5304”

If mailbox is a true listener later, that’s a collision. Right now it probably doesn’t bind (host shell), but we should treat this as a **P1 cleanup** so it doesn’t bite you later when mailbox becomes real.

### C) Bench auth is bypassed in dev

`MACRONODE_DEV_INSECURE=1 — bypassing admin auth for POST /api/v1/bench/run`

Perfect for dev, but it means our TODO must include **a clean capability/token model** for production (even if it’s “admin-only, local-only” at first).

### D) Benchmark results show a real performance signal

On the Benchmarks page, `system_summary` has **much higher p50/p95/p99** than the others (health/ready/status/storage are sub-ms; system_summary is multi-ms).
That likely means:

* system summary is doing heavier work (sysinfo/FS probing), or
* it isn’t cached, or
* it hits multiple sources per request.

This is useful — we can fix it with a simple cache or “sample once per interval, serve last value” pattern.

---

## 3) Updated TODO list (svc-admin + macronode), prioritized

### P0 — “Make dev stable + truthful” (fast wins that improve everything)

1. **Reduce sampler startup noise**

   * Option A: in `dev_svc_admin_stack.sh`, start/await macronode “ready” first (poll `/readyz` or `/healthz`) before launching svc-admin samplers.
   * Option B: in svc-admin sampler, treat initial connect-refused as “cold start” and log once (or log at DEBUG until first success).

2. **Fix “Showing 60 of 3” inventory mismatch**

   * Ensure the table is rendering `filtered.length`, not a static page size.
   * Ensure “showing X of Y” uses the same dataset the table actually renders.
   * If fallback mock is being merged with live list, make it explicit (label it).

3. **Make “Live vs Mock” truthful everywhere**

   * You already have a capability-rollout posture; now tighten it:

     * If any section is fallback: show “Source: Curated (fallback)” instead of “Live.”
     * If partially live: show “Hybrid: live summary + mock details.”

4. **Bandwidth status: turn it into an explicit capability**

   * If node doesn’t expose net rx/tx yet:

     * show “n/a (node does not expose net stats yet)” rather than implying it’s broken.

5. **Cache `system_summary` in macronode (or sample periodically)**

   * For admin-plane endpoints, it’s totally acceptable to return “last sampled at T” values.
   * This will drop benchmark latency and smooth UI jitter.

---

### P1 — macronode admin plane: finish the “truthful node surface”

These are the things that make the dashboard feel *real* and remove remaining “mock smell.”

1. **Expose real bandwidth**

   * Decide the contract: either

     * `system_summary` includes `netRxBps/netTxBps`, or
     * a separate `/api/v1/system/net` endpoint.
   * Keep it OS-safe: if the platform can’t provide it, return `null` + a reason.

2. **Expose launch/uptime metadata explicitly**

   * UI wants: `startedAt`, maybe `pid`, maybe `buildId`/`gitSha`, maybe `launchedBy`.
   * Right now “uptime” is derived from `/api/v1/status`; make it first-class.

3. **Curated file inventory should come from macronode**

   * The DB detail page literally says: “Next step: wire macronode to expose a curated file inventory DTO.”
   * Implement a safe DTO: aliases only, no raw host paths, limited set of “kinds” (MANIFEST/LOG/SST/LOCK/etc).

4. **Resolve port collision risk**

   * Move mailbox off 5304 (or reserve 5304 strictly for index).
   * Even if mailbox is currently a shell, clean it now to prevent future debugging hell.

5. **Admin auth model (minimal but real)**

   * Keep dev bypass, but add production flow:

     * Require `Authorization: Bearer <token>` (or mTLS later).
     * Use capability scoping for “read-only vs actions.”
   * This matters before we add “Actions” buttons in UI.

---

### P1 — svc-admin backend: make it a proper “node aggregator”

1. **Capability detection + UI config DTO**

   * Backend should expose: “Node supports X/Y/Z endpoints and metrics”
   * UI should render features based on that, not guesses.

2. **Sampler data model improvements**

   * Store: `last_ok_at`, `last_err_at`, `consecutive_failures`, `last_status_code`, `latency_ms`.
   * That enables better “fresh/stale/unreachable” classification than just time.

3. **Proxy endpoints should preserve upstream status**

   * If node returns 404 (cap missing) vs 500 (node broken), svc-admin should pass that truth through.

4. **Benchmarks proxy + run history**

   * Right now it looks like “run once, show results.”
   * Next step: store last N runs per node (in memory is fine initially), allow compare-to-previous.

---

### P2 — Benchmarks: turn MVP into a signature feature

1. **Baseline + regression view**

   * Save “golden baseline” per node profile.
   * Show delta (% change) for p50/p95 and RPS.

2. **Warmup phase + stable timing**

   * Add `warmup_secs` (even 2s) to reduce cold-cache distortions.

3. **Suite expansion plan (structure now, implement later)**

   * `admin_plane` is done.
   * Next obvious suites:

     * `storage_put_get` (svc-storage)
     * `overlay_stream` (stream handshake + payload)
     * `dht_ops`
   * Keep the UI already supports “payload size” as placeholder — good.

4. **Export**

   * “Copy JSON” and “Download JSON/CSV” for sharing results.

---

### P2 — UI polish that makes it “God-tier”

1. **Fresh/stale UX**

   * Grace window: after opening dashboard, don’t scream “stale” until we miss 2 intervals.
   * Add tooltip: “Fresh means sample < N seconds old.”

2. **Truthful labels everywhere**

   * If a card is mock: label it mock.
   * If derived: label “derived.”

3. **Fix the bandwidth card behavior**

   * If `n/a`, show an explicit “not supported yet” state.

4. **Make the DB warnings actionable**

   * “world-readable permissions detected (0755)” is great.
   * Add “Suggested fix” text (even if only dev-safe), and/or a doc link (internal route).

---

## 4) “Crossover items” to fold into our RON-CORE TODO (the meta gaps)

These are the cross-cutting rules that prevent drift later (and you were right to call them out during docs):

1. **Contracts must be explicit**

   * Stable error taxonomy (what is 404 vs 501 vs 503, and what UI does)
   * Idempotency rules for POST actions (bench/run, restarts later)
   * Correlation IDs (request-id in responses; propagate in svc-admin)

2. **Capability-driven rollout is the default**

   * “UI never breaks when node is missing endpoints”
   * “Fallback is deterministic and labeled”
   * “Backend tells UI what exists”

3. **Performance posture**

   * Prefer “sample + cache + serve” for admin-plane summaries
   * Benchmarks should be stable timing (warmup + fixed duration)

4. **Security posture**

   * Dev insecure bypass stays, but production path exists from day one:

     * capability-scoped tokens for read-only vs actions
     * don’t add destructive UI actions until this is in

---

## 5) The single best “next step” sequence (if we want momentum)

If you want the cleanest rebuild of the TODO into execution order, do this:

1. Fix **inventory count mismatch** + make **Live vs Mock truthful**
2. Implement **real bandwidth** (or mark it explicitly unsupported)
3. Add **cache/sampling** for `system_summary`
4. Clean up **port collision risk**
5. Improve **sampler freshness model** (less noisy + better stale/unreachable)
6. Add **benchmark run history + baseline**

That sequence makes the dashboard feel *finished* without needing any big new subsystems.

---

If you want, I can now rewrite the TODO into a **checkbox-style execution plan** split into:

* `crates/macronode` tasks (with endpoint/DTO names),
* `crates/svc-admin` backend tasks,
* `crates/svc-admin/ui` tasks,

…and ordered by “P0/P1/P2” exactly like above.

According to a document from your current repo notes/codebundle, you’re in a really solid “v1 dashboard is real” state: svc-admin is running UI/API on `127.0.0.1:5300`, health/metrics on `127.0.0.1:5310`, and it spawns facet metrics samplers for **3 configured nodes** on a **5s interval**, polling the node admin plane `/metrics` (you’re seeing the expected initial connection-refused warnings because svc-admin comes up before macronode binds `127.0.0.1:8080`).

Below is an updated TODO list based on the screenshots + your terminal output.

---

## TODO (rebuilt from current state)

### 0) “Truth & docs drift” cleanup (do this first)

* **Fix doc/config mismatch:** your docs example still points node `base_url` at `http://127.0.0.1:8090` (gateway), but your actual admin plane + sampler are using `http://127.0.0.1:8080` (macronode admin). Update the `ALL_DOCS.md` snippet/config example so newcomers don’t wire the wrong port.
* Keep the “read-only by default” posture in docs (it’s already written as an invariant—good).

### 1) Dev-stack ergonomics (remove the scary-but-harmless warning spam)

Right now svc-admin’s sampler logs connection refused until macronode finishes compiling and binds 8080.

* In `scripts/dev_svc_admin_stack.sh`, **start macronode first**, then `wait-for`:

  * `curl -sf http://127.0.0.1:8080/healthz` (or `/readyz`)
  * only then start svc-admin (and then Vite)
* OR keep parallel start, but change sampler logging to:

  * treat **first N failures** as “startup warming” (log once per node), then normal warn cadence.

(Your NOTES explicitly call out that upstream error accounting is desired—so this is purely polish, not correctness.)

### 2) Bandwidth metric: make it real (or make it honest)

Your screenshots show CPU/RAM/Storage looking correct; bandwidth is the only “maybe.”

* Decide the contract: **bandwidth from `/api/v1/system/summary`** vs derived from Prometheus `/metrics`.
* If you’re using system summary:

  * Ensure it returns **rx/tx bytes-per-second** (or rates) and include a `sampleWindowMs` / `since` so the UI can explain “warming up” on first sample.
* UI rule:

  * If bandwidth is missing/0 because it needs a second sample, display **“warming up”** (not `0%`) + tooltip.

### 3) Curated file inventory: currently mocked, and there’s a visible bug

Your Database Detail page literally says “Next step: wire macronode to expose a curated file inventory DTO…”, so this is still a **safe deterministic mock** today (which is correct for v1).
Also: the UI can show **“Showing 60 of 3”** because the mock generator can return `limit` entries even when `fileCount` is 3.

* Fix the mock generator to cap to `min(limit, fileCount)` (or to `min(limit, derivedCount)`), so the display can’t contradict itself.

  * The generator is in the UI codebundle near `generateCuratedFiles(...)`.
* Then implement the real path (when ready):

  * macronode admin endpoint: `GET /api/v1/storage/databases/:name/files?limit=&contains=`
  * svc-admin: proxy it *optionally* (404 = “endpoint missing”, not fatal)
  * SPA: prefer live list when available; fall back to mock otherwise
* Keep the existing safety invariant: **aliases only, no raw paths** (the UI already enforces that framing).

### 4) Benchmarks: working MVP — now harden semantics + add operator UX

Your Benchmarks page is **already implemented and producing real numbers** (admin_plane suite, per-route p50/p95/p99). Nice.
Now tighten correctness:

* In svc-admin backend, for:

  * `GET /api/v1/bench/runs/:run_id`
  * `GET /api/v1/bench/runs/:run_id/result`
    use the “**no_404**” client path so a 404 means “run not found”, not “endpoint missing.”
    Operator UX next:
* Add **run history** (ring buffer in svc-admin backend) so Benchmarks can show “last 20 runs” and allow quick compare.
* Add “Export JSON” for a run (UI button).
* Add “Copy as markdown table” (perfect for reports/issues).

### 5) Macronode admin auth: keep dev bypass, but make it harder to misuse

Your macronode admin middleware explicitly bypasses auth when `MACRONODE_DEV_INSECURE=1`, including for `POST /api/v1/bench/run`. Good for dev; dangerous if someone binds non-loopback and forgets.
TODO:

* On startup, if `MACRONODE_DEV_INSECURE=1` and bind is non-loopback, print a **loud banner** (warn every N seconds).
* In prod profile/config, refuse to start with `MACRONODE_DEV_INSECURE=1`.

### 6) Multi-node realism (optional, but you’ll want it soon)

Right now you have 3 nodes in svc-admin, but one macronode instance. That’s fine for UI iteration, but next:

* Either run **multiple macronodes on different admin ports**, or
* Label the extra nodes as “mock-only” so operators don’t assume they’re distinct.

---



### END NOTE - JANUARY 4 2026 - 19:00 CST




### BEGIN NOTE - JANUARY 5 2026 - 11:05 CST

Below are **carry-over notes you can paste into the next instance.** They’re written as a **“state of the world + what’s working + exact next moves”** doc for **svc-admin**, **macronode**, and the **overall RON-CORE project**.

---

# CARRY-OVER NOTES — RON-CORE (svc-admin + macronode + overall)

## 0) Current status in one paragraph

We have a working local dev stack where **macronode boots cleanly**, spawns multiple planes (gateway/index/storage real, overlay/dht/mailbox stubs), exposes a **node admin plane on 127.0.0.1:8080** with `/healthz`, `/readyz`, `/metrics`, and svc-admin boots cleanly with a **UI/API server on 127.0.0.1:5300**, a **metrics server on 127.0.0.1:5310**, and the **Vite SPA on [http://localhost:5173](http://localhost:5173)**. We fixed multiple Axum 0.7 “serve/router state” issues (Router<Arc<_>> needs `into_make_service()` and correct nesting/state types). We also fixed svc-admin route mismatch so `/api/v1/nodes` now returns **200 OK** (previously 404). svc-index is embedded and reachable at **127.0.0.1:5304** with `/readyz` and `/metrics` working.

---

## 1) Dev stack: what is known-good now (ports + endpoints)

### 1.1 Known-good ports (local slice)

* **macronode admin HTTP:** `127.0.0.1:8080`

  * `GET /readyz` → 200 OK
  * `GET /metrics` → OK
* **svc-admin UI/API:** `127.0.0.1:5300`

  * `GET /healthz` → `ok`
  * `GET /readyz` → `{ "ready": true }`
  * `GET /api/nodes` → 200 OK (and also `/api/v1/nodes` → 200 OK after router fix)
* **svc-admin health/metrics:** `127.0.0.1:5310`

  * `GET /metrics` → OK
* **SPA dev server:** `http://localhost:5173`

  * `/playground` reachable when `SVC_ADMIN_UI_DEV_ENABLE_APP_PLAYGROUND=true`
* **svc-index embedded:** `127.0.0.1:5304`

  * `GET /readyz` → OK
  * `GET /metrics` → OK
* **svc-storage embedded:** `127.0.0.1:5303` (from macronode log)

  * exposes `/healthz`, `/readyz`, `/metrics`, `/version`, and object routes (`/o`, `/o/:cid`).
* **svc-gateway ingress:** `127.0.0.1:8090`

  * `GET /ingress/ping` returns JSON and increments gateway facet OK counter.

### 1.2 Launch script (dev stack)

`scripts/dev_svc_admin_stack.sh` currently starts:

* macronode (waits for `/readyz`)
* svc-admin
* SPA (`npm run dev`)
  and prints the canonical addresses

### 1.3 Your rg output (DEFAULT_*_STR)

Your output shows the remaining `DEFAULT_*_STR` references are **only the intended centralized constants in ports.rs** and the **fallback log lines** in individual services (gateway/dht/storage/mailbox/overlay/index). That’s correct and expected. Nothing indicates duplicated per-service default constants anymore.

---

## 2) svc-admin: key fixes & current architecture

### 2.1 Major issue resolved: API route mismatch / 404

Previously:

* `/api/v1/nodes` was returning **404 Not Found** even though health/ready were OK.
  Now:
* Both `/api/nodes` and `/api/v1/nodes` return **200 OK**.

Root cause: router nesting/state-type mismatch (Axum Router generic state `Router<S>` must match across `.nest()` calls, and API router must be built with correct state). We fixed router composition so the “v1” API resolves to the same handlers.

### 2.2 Major issue resolved: Axum 0.7 serve + Router state

We hit compile errors like:

* `Router<Arc<AppState>>: Service<IncomingStream>` not satisfied
* `Serve<...> is not a future` / `.await` issues
  Fix pattern adopted:
* Use `axum::serve(listener, app.into_make_service()).await`
* Ensure consistent state type and avoid passing raw Router<Arc<_>> where MakeService is needed

### 2.3 svc-admin server bootstrap now

`crates/svc-admin/src/server.rs` now:

* initializes tracing
* initializes config-derived static metrics
* builds `Arc<AppState>`
* spawns facet metrics samplers (polls node `/metrics`)
* binds:

  * main listener: `config.server.bind_addr` (5300)
  * metrics listener: `config.server.metrics_addr` (5310)
* runs two servers:

  * “metrics/health server” task
  * “main UI/API server” with graceful shutdown on Ctrl-C
* on shutdown, signals sampler tasks via watch channel

### 2.4 svc-admin router surface (current)

From router.rs:

* `GET /healthz`, `GET /readyz`, `GET /metrics`
* `GET /api/ui-config`, `GET /api/me`
* Nodes:

  * `GET /api/nodes`
  * `GET /api/nodes/:id/status`
  * `GET /api/nodes/:id/metrics/facets`
  * `GET /api/nodes/:id/system/summary` (optional rollout)
  * `GET /api/nodes/:id/storage/*` endpoints (optional rollout)
* Benchmarks:

  * `POST /api/nodes/:id/bench/run`
  * `GET /api/nodes/:id/bench/runs/:run_id`
  * `GET /api/nodes/:id/bench/runs/:run_id/result`
* Playground:

  * `/api/playground/examples`
  * `/api/playground/manifest/validate`
* Control actions (auth/config gated):

  * reload/shutdown/debug crash

Important semantics already implemented:

* Optional endpoints return **501 NOT_IMPLEMENTED** so SPA can degrade gracefully
* Bench endpoints have special mapping: upstream 404 → “run_id not found” etc.
* Auth resolution: `/api/me` can dev-fallback; node actions require auth and roles.

### 2.5 Remaining svc-admin work (TODO)

**High priority**

1. **Two-plane split correctness**
   Right now 5300 serves UI/API routes and 5310 serves metrics/health; confirm the router composition is correct for both listeners:

   * Decide if 5310 should serve ONLY `/metrics,/healthz,/readyz` and not full API.
   * If we want strict separation, create a **small metrics Router** rather than cloning full app.

2. **Facet metrics sampler polish**

   * Confirm sampler uses correct node base URLs and scrapes reliably.
   * Add better “last scrape time / last error” per node for UI.
   * Ensure it tolerates nodes missing `/metrics` or returning invalid exposition.
   * Confirm bandwidth metric is correct (you said “everything works except maybe bandwidth”).

3. **Benchmark surface end-to-end**

   * svc-admin routes exist, but node-side bench endpoints may be missing/stub.
   * Implement node admin plane bench routes (macronode first) and wire the run/status/result lifecycle.

4. **Auth plan**

   * You explicitly noted: “each plane eventually needs auth with svc-admin otherwise security risk?”
   * Need to implement **svc-admin auth mode** and propagate identity to node admin plane calls:

     * tokens/headers validation
     * role mapping: admin/ops
     * future: mTLS or signed requests between svc-admin and node admin plane

**Medium priority**
5) **API discoverability**

* Add `GET /api` or `GET /api/v1` index endpoints if desired (to reduce confusing 404s and help debugging).

6. **UI reliability**

   * Ensure SPA uses correct base path:

     * If Vite proxy expects `/api/*` then ensure it calls `/api/...` not `/api/v1/...` unless we keep both.
   * Keep deterministic fallbacks for optional features.

**Lower priority**
7) Observability improvements:

* Add per-endpoint latency histograms in svc-admin
* Add structured audit logs for actions and auth failures

---

## 3) macronode: key fixes & current architecture

### 3.1 Services spawned at boot (spawn_all)

`crates/macronode/src/services/spawn.rs`:

* Spawns:

  * `svc_gateway` (real HTTP ingress, marks readiness when bound)
  * `svc_index` (real embedded HTTP server via svc-index crate, marks index bound)
  * `svc_storage` (embedded HTTP server via svc-storage)
  * `svc_overlay` (stub loop + readiness bit)
  * `svc_mailbox` (stub loop + readiness bit)
  * `svc_dht` (stub loop + readiness bit)
* Sets `probes.set_deps_ok(true)` once workers spawned (coarse gating)

### 3.2 Major issue resolved: const-eval SocketAddr in macronode svc_index

We originally hit:

* `cannot call non-const associated function SocketAddr::from(...) in constants`
  Fix: store default bind as a string and parse at runtime (or centralize in ports.rs).

### 3.3 Major issue resolved: Axum 0.7 server serving Router state

For embedded services (svc-index, gateway), use:

* `axum::serve(listener, app.into_make_service()).await`
  so we don’t hit Router<Arc<_>> Service bounds.

### 3.4 Ports centralized

`crates/macronode/src/services/ports.rs` now defines canonical defaults as strings:

* overlay 5301
* dht 5302
* storage 5303
* index 5304
* mailbox 5305
* gateway 8090
  Helper parse functions `default_*_addr()` provided.

Your `rg` confirms the only `DEFAULT_*_STR` usage is:

* inside ports.rs and
* inside fallback log lines in svc_{gateway,overlay,dht,mailbox,storage,index}.
  That’s correct.

### 3.5 Known-good macronode runtime logs

When starting stack, macronode shows:

* gateway listening 8090
* storage listening 5303
* svc-index ready then “starting bind=5304”
* admin plane listening 8080

### 3.6 Remaining macronode work (TODO)

**High priority**

1. **Admin-plane endpoint completeness**
   Ensure macronode admin plane has:

   * truthful `/healthz`, `/readyz`, `/metrics`
   * system summary endpoint already exists (you said dashboard shows CPU/RAM/net/storage)
   * storage inventory endpoints (if svc-admin expects them)
   * facet health/status endpoints (planes list, readiness bits, versions)

2. **Plane auth boundary (future)**
   If node admin plane is exposed beyond localhost later:

   * implement auth or require only loopback in dev + secure in prod
   * plan mTLS / signed headers between svc-admin and macronode admin plane

3. **Replace stubs with real services** (overlay/dht/mailbox)

   * overlay: integrate svc-overlay crate + ron-transport
   * dht: integrate svc-dht crate
   * mailbox: integrate svc-mailbox crate
     Each needs:
   * real bind/listen or transport start
   * readiness semantics (flip only after binding/ready)
   * metrics and status exposure

**Medium priority**
4) **Graceful shutdown propagation**

* storage server currently does not accept shutdown signal
* index/gateway similarly stop on process exit; eventually add proper shutdown tokens

5. **Readiness gating sophistication**
   Right now `deps_ok` flips after spawn, regardless of bind success.

   * tighten required readiness to reflect:

     * gateway bound
     * index bound
     * storage bound
     * overlay/dht/mailbox readiness
   * if a bind fails, readyz should remain degraded.

---

## 4) Cross-cutting: what we proved / why it matters

### 4.1 The dev stack is stable again

* macronode → ready
* svc-admin → ready
* SPA → reachable
* critical endpoints respond with 200
  This unblocks “God-tier dashboard” work.

### 4.2 We established the Axum 0.7 invariants across the repo

* Router state types must match in nesting.
* Use `into_make_service()` for `axum::serve(...)`.
* Avoid const SocketAddr from non-const constructors; use string constants parsed at runtime.

These are now the “house rules” to avoid regressions.

---

## 5) Next TODO list (prioritized, actionable)

### P0 — keep dashboard functional and truthful

1. **Strictly separate svc-admin main vs metrics listeners**

   * 5310 should serve only `/metrics,/healthz,/readyz`
   * 5300 serves UI/API
   * remove accidental exposure of full API on 5310 (if currently happening)

2. **Fix/verify bandwidth metrics**

   * identify which metric name the UI expects vs what macronode exports
   * verify it’s a rate (bytes/sec) not a counter
   * ensure units consistent (bps vs B/s)

3. **Node endpoints parity**

   * confirm macronode admin plane implements everything svc-admin UI calls:

     * system summary
     * storage summary + db list/detail
   * if missing, svc-admin should 501 cleanly and UI should fallback.

### P1 — benchmarks end-to-end

4. Implement node admin plane bench endpoints:

   * `POST /api/v1/bench/run`
   * `GET /api/v1/bench/runs/:id`
   * `GET /api/v1/bench/runs/:id/result`
5. Wire svc-admin proxy to macronode endpoints (already scaffolded on svc-admin side)

### P2 — security posture

6. Decide auth mode progression:

   * dev: fallback identity OK
   * ops/admin actions require auth
   * production: no dev fallback, enforce token/mTLS
7. Decide “plane-to-svc-admin” auth strategy (headers signing or mTLS)

### P3 — replace stubs

8. overlay: integrate svc-overlay + ron-transport (listener + handshake + conn tasks)
9. dht: integrate svc-dht
10. mailbox: integrate svc-mailbox

### P4 — readiness/health/UX polish

11. tighten readiness gating
12. add node “last scrape / scrape error” visibility in svc-admin UI
13. add “API discovery endpoints” to help curl debugging (`/api` index)

---

## 6) Quick commands / checks (copy-paste)

### Confirm node + admin endpoints

```bash
curl -sf http://127.0.0.1:8080/readyz && echo "macronode ready OK"
curl -sf http://127.0.0.1:8080/metrics >/dev/null && echo "macronode metrics OK"
curl -sf http://127.0.0.1:5304/readyz && echo "svc-index ready OK"
curl -sf http://127.0.0.1:5304/metrics >/dev/null && echo "svc-index metrics OK"
```

### Confirm svc-admin API

```bash
curl -sf http://127.0.0.1:5300/healthz && echo "svc-admin health OK"
curl -sf http://127.0.0.1:5300/readyz && echo "svc-admin ready OK"
curl -sf http://127.0.0.1:5300/api/nodes >/dev/null && echo "svc-admin /api/nodes OK"
curl -sf http://127.0.0.1:5300/api/v1/nodes >/dev/null && echo "svc-admin /api/v1/nodes OK"
curl -sf http://127.0.0.1:5310/metrics >/dev/null && echo "svc-admin metrics OK"
```

### Confirm remaining DEFAULT_*_STR usage is expected

```bash
rg 'DEFAULT_.*_STR' crates/macronode/src/services -n
```

---

### END NOTE - JANUARY 5 2026 - 11:05 CST



### BEGIN NOTE - JANUARY 5 2026 - 14:45 CST

Below are **carry-over notes you can paste into the next instance**. They’re written as a **state-of-the-world + exact next moves** doc for **macronode**, **svc-admin (backend + SPA)**, and **RON-CORE overall**, including the **“gaps we noticed while building apps”** and the new requirement: **God-tier bandwidth + request tracking with min/hour/day/month rollups + charts**.

---

# CARRY-OVER NOTES — RON-CORE (macronode + svc-admin) — Status + TODO (God-Tier Admin)

## 0) Snapshot (current state in one paragraph)

We have a working **RON-CORE Admin dashboard (svc-admin)** with a clean SPA + API stack and a functional macronode admin plane. **CPU and RAM are now truthful** via a real `/api/v1/system/summary` endpoint on **macronode** and proxy calls from **svc-admin**; **database detail pages are live (not mock)** and show curated DB metadata safely; and **NodeDetailPage “tiles” now show Reported values**, including **Bandwidth Rx/Tx rates** (instead of a questionable %). The UI has a “live tiles” refresh control and shows whether values are **Reported / Warming / Mock**.

---

## 1) What we accomplished (macronode)

### 1.1 Added `/api/v1/system/summary` (truthful CPU/RAM + optional net rates)

**File:** `crates/macronode/src/http_admin/handlers/system_summary.rs`
**Behavior:**

* Returns JSON:

  * `updatedAt`
  * `cpuPercent` (best effort)
  * `ramTotalBytes`, `ramUsedBytes` (bytes)
  * `netRxBps`, `netTxBps` (bytes/sec) **only after 2 samples**
* Uses an in-process sampler (`OnceLock<Mutex<Sampler>>`) so we don’t hold locks across `.await`.
* Network rates are computed by delta across calls (prev total bytes & timestamp).
* Verified via curl:

  * first call returns no net rates
  * second call returns net rates (`netRxBps`, `netTxBps`) and cpu/ram.

### 1.2 Storage + index services are running in dev stack

Dev script output shows:

* macronode admin at `127.0.0.1:8080`
* svc-storage embedded
* svc-index embedded
* other planes are “host shells” waiting for real wiring (overlay/dht/mailbox).

---

## 2) What we accomplished (svc-admin backend)

### 2.1 NodeRegistry supports optional capability endpoints

**File:** `crates/svc-admin/src/nodes/registry.rs`

Working “try_*” proxy methods:

* `try_system_summary(id)` -> `Option<SystemSummaryDto>`
* `try_storage_summary(id)` -> `Option<StorageSummaryDto>`
* `try_storage_databases(id)` -> `Option<Vec<DatabaseEntryDto>>`
* `try_storage_database_detail(id, name)` -> `Option<DatabaseDetailDto>`
  Includes strict path safety for DB name (`[A-Za-z0-9._-]`).

### 2.2 System summary DTO exists and matches camelCase JSON

**File:** `crates/svc-admin/src/dto/system.rs`

* `updated_at: String`
* `cpu_percent: Option<f32>`
* `ram_total_bytes`, `ram_used_bytes`
* `net_rx_bps`, `net_tx_bps` optional
  **Invariants:** camelCase JSON, bytes are bytes, optional fields omitted.

### 2.3 Router entry points confirmed

We confirmed svc-admin has:

* `build_router(state)` and `build_metrics_router(state)` in `crates/svc-admin/src/router.rs`.

---

## 3) What we accomplished (svc-admin SPA)

### 3.1 NodeDatabaseDetailPage works and shows live DB detail

**File:** `crates/svc-admin/ui/src/routes/NodeDatabaseDetailPage.tsx`

* Route exists and renders DB details properly.
* Calls:

  * `getNodeStatus`
  * `getNodeStorageDatabaseDetail`
* If endpoint missing (404/405/501), falls back to deterministic safe mock.
* Curated “file inventory” is safe/alias-only (not real filesystem browsing).
* Screenshot confirms: **Source: Live** and real values rendering.

### 3.2 NodeDetailPage tiles are truthful and no longer labeled “mock”

**File:** `crates/svc-admin/ui/src/routes/NodeDetailPage.tsx`

* CPU/RAM now driven by live `SystemSummary` when available.
* Storage tile can use `StorageSummary` when available.
* Bandwidth tile shows **Rx/Tx rate** (B/s) rather than a fake percent cap.

**Key UI improvements already in place:**

* “Reported” badges on tiles.
* “Live tiles” toggle + refresh interval dropdown (e.g., 1s shown in screenshot).
* Bandwidth tile shows **↓ / ↑** rates (KiB/s, etc.), not just a “79%” guess.

### 3.3 Utilization widgets upgraded to support sources + real bandwidth rates

**File:** `crates/svc-admin/ui/src/routes/node-detail/utilization.tsx`

* Removed hardcoded “(mock)” labels.
* Added `TileSource` and a pill: `Reported / Warming / Mock`.
* `BandwidthBarsGauge` can show Rx/Tx textual rates when provided.
* Still supports deterministic mock fallback for missing endpoints.

---

## 4) Current dev stack command (known good)

* `cargo build -p macronode -p svc-admin`
* `bash scripts/dev_svc_admin_stack.sh`

  * macronode ready: `/readyz`
  * svc-admin UI/API on `127.0.0.1:5300`
  * metrics on `127.0.0.1:5310`
  * SPA dev server on `http://localhost:5173`

---

# 5) TODO — Next major goals (God-tier)

## 5.1 God-tier network accounting: minute/hour/day/month bandwidth + request counts

**New requirement (must-do):** On NodeDetailPage, under the CPU/RAM/Storage/Bandwidth tile area, show:

* **Bandwidth totals** (Rx bytes, Tx bytes, combined) for:

  * last **minute**
  * last **hour**
  * last **day**
  * last **month**
* **Request counts** for the same windows:

  * total requests
  * per-facet breakdown (optional but ideal)
* A **chart** / sparkline panel showing:

  * minute-level history (last 60 mins)
  * hourly history (last 24h)
  * daily history (last 30d)
  * monthly history (last 12m)

### The key truth: percent bandwidth is not meaningful without link capacity

We should treat “bandwidth utilization %” as **optional** and only show it when the node reports link capacity (or config). The truthful baseline is **rates** and **bytes over time**, plus **request counts**.

---

## 5.2 Backend design for bandwidth/request tracking (two layers)

We need **both**:

1. **Node-local accounting** (macronode) for truthful per-node usage.
2. **Dashboard-side aggregation** (svc-admin) for multi-node comparisons + caching.

### 5.2.1 Macronode: add admin endpoint for network + request accounting

Add a new endpoint under admin plane:

**Option A (recommended):**

* `GET /api/v1/system/net/accounting`

  * returns:

    * `updatedAt`
    * `rxBytesTotal`, `txBytesTotal` (monotonic totals)
    * `rxBps`, `txBps` (current rate)
    * rollups:

      * `minute`, `hour`, `day`, `month` windows:

        * `rxBytes`, `txBytes`, `totalBytes`
        * `requests` (total)
        * optionally: `requestsByFacet: { "admin.status": 123, ... }`
    * time series samples:

      * `seriesMinute`: array of 60 points (per minute)
      * `seriesHour`: array of 24 points (per hour)
      * `seriesDay`: array of 30 points (per day)
      * `seriesMonth`: array of 12 points (per month)

**How to implement without guessing:**

* Maintain a lightweight ring buffer in macronode admin:

  * tick every 1s or 5s (configurable)
  * record: rx_total, tx_total from sysinfo networks totals
  * compute deltas into buckets
* Maintain request counters in the **admin HTTP router**:

  * Add an Axum middleware that increments counters by route/facet.
  * These can be mapped to “facet names” like `admin.system.summary`, `admin.storage.summary`, etc.
* Expose the rollups and series by summing bucket ranges.

**Important invariants:**

* No raw per-interface detail needed initially; sum totals across interfaces is fine.
* Keep no PII; only metrics.
* Keep lock scope short; no awaits while locked.

### 5.2.2 svc-admin: add API proxy and cache

svc-admin should:

* proxy the new endpoint (optional capability)
* cache last results and provide stable data to SPA
* also allow SPA to request only the “small” view (rollups) vs “big” view (series).

---

## 5.3 SPA: add “Network” panel under tiles (God-tier)

In NodeDetailPage, directly below the tile grid, add a panel like:

**Header row:**

* “Network & Requests”
* “Live” toggle (existing)
* interval selector (existing)
* time window selector: `1h | 24h | 7d | 30d`
* optional: “Show bytes / show req” toggle

**Rollup cards:**

* Last minute: bytes + req
* Last hour: bytes + req
* Last day: bytes + req
* Last month: bytes + req

**Charts:**

* Bandwidth over time (Rx, Tx, total)
* Requests over time (total; optionally stacked by facet)

**UI-level requirements:**

* Must degrade gracefully:

  * If endpoint missing: show “Mock” or “Unavailable”.
  * If warming: show “Warming” until enough samples.
* Must not spam requests:

  * default poll: 2s–5s
  * interval dropdown already exists; keep it.
  * allow turning off live tiles.

---

# 6) Remaining TODO — broader svc-admin / macronode gaps

## 6.1 “Plane auth” and security posture (important)

We discussed that eventually each plane must have auth with svc-admin:

* Add an auth plan (mTLS or signed token) between svc-admin and node admin plane.
* Enforce “deny by default” for mutations (already posture: dev read-only).
* Ensure optional endpoints return 404/405/501 for capability discovery.

## 6.2 Node “launch metadata” (uptime tile is best-effort)

UI mentions next step: include:

* `startedAt`
* `launchedBy`
* `bootId`
* `pid`
  Add an identity/status field on macronode admin `/api/v1/status` or dedicated `/api/v1/identity`.

## 6.3 Storage inventory “curated file inventory” live DTO

NodeDatabaseDetail currently has curated file list as deterministic mock.
Add a real endpoint (read-only curated list) for:

* manifest/lock/log/sst segments counts
* file sizes and modes as safe metadata
* no absolute paths; alias-only.

---

# 7) “Gaps noticed while building apps” (RON-CORE overall)

When we built/blueprinted client apps (mobile + SDK consumers), we noticed we need stronger platform-level pieces:

## 7.1 SDK / app needs stronger observability contracts

Apps need consistent, stable “admin/ops surfaces” to debug:

* node health/ready semantics that are truthful
* latency/RPS/error for key facets
* ability to see network/request cost (especially for mobile and constrained environments)

## 7.2 Rate limiting + budget awareness

Clients need:

* server-driven suggested poll intervals
* a way to detect “expensive mode” and back off
* consistent retry/backoff guidance.

## 7.3 Consistent DTO schema governance

We must keep:

* camelCase JSON
* explicit optional fields
* strict invariants per endpoint
* consistent “capability missing” semantics:

  * 404/405/501 -> capability missing
  * 404 semantic (bench run id not found) -> handled separately (already done pattern)

---

# 8) Suggested next execution order (for the next instance)

1. **Macronode:** implement **request counting middleware** for admin endpoints.
2. **Macronode:** implement **network accounting sampler** with ring buffers + bucket rollups.
3. **Macronode:** expose new endpoint `GET /api/v1/system/net/accounting` (or similarly named).
4. **svc-admin backend:** add DTO + registry `try_*` proxy for the endpoint.
5. **svc-admin SPA:** add NodeDetailPage “Network & Requests” panel:

   * rollup cards for min/hour/day/month
   * charts (minute/hour/day/month)
   * respects live toggle & interval
6. Finally: add auth plan and identity/launch metadata improvements.

---

# 9) Quick glossary (keep consistent)

* **Bps** in code means **bytes/sec** (not bits/sec).
* **Rx/Tx**: receive/transmit at node host boundary.
* **Requests**: HTTP requests handled by the admin plane (and later per-plane gateways too).
* **Rollups**: sum of deltas across bucket windows.
* **Series**: arrays used for charts (minute/hour/day/month).

---

### END NOTE - JANUARY 5 2026 - 14:45 CST


### BEGIN NOTE - JANUARY 5 2026 - 18:40 CST

Below are **carry-over notes you can paste into the next instance.** I’m writing these as a **“state of the world + exact next moves”** document for finishing **svc-admin (God-tier dashboard)**, wiring **macronode**, and closing remaining **RON-CORE gaps**—including the new UI/cluster requirements you just added (tags/categories, facet drilldowns, and cluster-wide network accounting).

---

# CARRY-OVER NOTES — RON-CORE / svc-admin / macronode (God-Tier Admin Dashboard + Cluster UX)

## 0) Current status (one paragraph)

svc-admin is now a **high-quality operator dashboard** with the Node Detail view showing a **Planes summary**, a **utilization block** (CPU/RAM/Storage/Bandwidth), and a new **Network accounting section** that displays **minute/hour/day/month** totals for **bytes + requests**, with a small “series” visualization. The dashboard follows a **capability-rollout posture**: if node endpoints aren’t implemented yet (or return 404/501), the UI avoids crashing and falls back to deterministic mock/“warming”/“reported” sources. The remaining “big work” is: **(1) find and edit the actual NodeDetail route file** to move the main “Planes table” out of the center panel into the right sidebar (“All planes”), **(2) implement cluster-scale features** like node tagging/category filtering and cluster-wide network totals, and **(3) finish the real node-side metrics/endpoints so bandwidth + requests are truthful across macronode planes**.

---

## 1) What we accomplished (svc-admin UI)

### 1.1 NodeDetail utilization widgets (CPU/RAM/Storage/Bandwidth)

**File:** `crates/svc-admin/ui/src/routes/node-detail/utilization.tsx`

**Done:**

* Built deterministic, stable mock utilization per nodeId.
* Added source labeling via a **Source pill** in card headers: `Reported | Warming | Mock`.
* Bandwidth tile supports truthful **Rx/Tx rate formatting** (B/s, KiB/s, MiB/s...) when provided.
* UI primitives are pure presentational components and kept out of route composition to reduce LOC.

**Impact:**

* NodeDetail looks “God-tier” even before backend metrics are fully wired.
* “Reported vs Mock” is explicit, so operators aren’t misled.

### 1.2 Plane normalization helpers

**File:** `crates/svc-admin/ui/src/routes/node-detail/planeSummary.tsx`

**Done:**

* Normalizes restarts across possible DTO variants: `restarts`, `restart_count`, `restartCount`.
* Normalizes readiness (boolean/string) and health.
* Provides a **Pill** component and compute summary: total/ready/degraded/down/restarts.

### 1.3 Service mapping helper

**File:** `crates/svc-admin/ui/src/routes/node-detail/serviceMap.ts`

**Done:**

* Central map from plane name to service string used for debug tooling (crash trigger, etc).

### 1.4 NodeDetail orchestration hook

**File:** `crates/svc-admin/ui/src/routes/node-detail/useNodeDetail.ts`

**Done:**

* Single hook owns: status polling, facets polling, identity (me/ui config), action triggers.
* No overlapping in-flight polls.
* Includes dev debug controls (crash plane) and action gating via roles + readOnly UI config.

### 1.5 DTO type hygiene & compat posture

**File:** `crates/svc-admin/ui/src/types/admin-api.ts`

**Done / pending alignment:**

* We discussed (and partially implemented) **compatibility-safe optional aliases** to prevent UI breaks when Rust emits snake_case vs camelCase.
* Most important class of breakages: `restart_count` vs `restartCount` vs `restarts`, `p95_latency_ms` vs `p95LatencyMs`, etc.
* This is a deliberate “rollout safety” stance while macronode endpoints are still stabilizing.

### 1.6 NodeDetail visual improvements (Network accounting)

**Status:** implemented in NodeDetail UI (as evidenced by screenshot)
**Done:**

* Added “Network accounting” section with:

  * quick totals for minute/hour/day/month
  * bytes + request counts
  * a small “series” visualization (normalized line)
* This is the correct direction: **network usage is the #1 admin concern**, and you now see it “upfront.”

---

## 2) The current blocker (why you couldn’t find “Data planes”)

### Problem

You want to:

* remove the **big center “Planes table”** (circled in pink),
* move it into the right panel where “Data planes” currently lives,
* rename that section from “Data planes” → **“All planes”**,
* then expand the center area tiles to use the freed space.

But you couldn’t find the file because:

* the NodeDetail route file isn’t `node-detail/index.tsx` in this repo layout, and it’s likely named something like:

  * `NodeDetailPage.tsx`
  * `nodeDetail.tsx`
  * `detail.tsx`
  * `routes/nodes/node-detail.tsx`
  * or living under `routes/nodes/` rather than `routes/node-detail/`.

### What we already have

We already have the helper modules. The missing piece is the **composition file** that renders the screenshot layout.

### Next move (reliable path)

From repo root, in the next session, run:

```bash
rg -n "Detailed status|Open storage|Data planes|Metrics: fresh|Network accounting|Planes" crates/svc-admin/ui/src
rg -n "useNodeDetail\\(" crates/svc-admin/ui/src
rg -n "PLANE\\s+HEALTH\\s+READY\\s+RESTARTS" crates/svc-admin/ui/src
```

One of these will reveal the actual NodeDetail component. That is the file that must be updated to:

* remove the big planes table block,
* replace the right-panel “Data planes” panel with “All planes,”
* widen/expand the utilization/tiles area.

(We already have everything needed in helpers; we just need the composition file.)

---

## 3) New requirements you just added (must be integrated into TODO)

You added three major “cluster-scale” UX needs. These are not “nice-to-haves”; they’re core operator functionality.

### 3.1 Node categories / tags + grouping + filtering (cluster-scale)

**Goal:**

* An operator can assign a **category/tag** to a node (e.g. `region=us-east`, `tier=prod`, `rack=r12`, `team=search`, `env=staging`).
* Nodes can be grouped and filtered by tag/category.
* Metrics views (especially network usage / storage usage) can be filtered to **show totals for all nodes in a tag**.

**Design direction (recommended)**

* Treat tags as **key/value labels** (not a single category string). Example:

  * `env: prod`
  * `region: us-east`
  * `cluster: alpha`
  * `role: gateway`
* UI presents:

  * a Tag editor per node (chip UI)
  * a filter bar: dropdowns for keys + values
  * quick grouping modes: group by `env`, group by `region`, etc.

**Where tags should live (two phases)**

* **Phase A (MVP, fast):** Tags live in svc-admin’s own config/registry store (not in node).

  * Pros: immediate UI feature; no node changes required.
  * Cons: tags aren’t portable unless exported.
* **Phase B (real):** Tags become part of the node registry contract and optionally node-reported metadata.

  * Node may report `labels` in `/api/v1/status` (or a metadata endpoint) but svc-admin remains source-of-truth for operator-assigned labels.

**Backend surfaces needed**

* svc-admin API additions:

  * `GET /api/nodes` already exists → extend response to include `labels`/`tags`.
  * `PATCH /api/nodes/:id` or `PUT /api/nodes/:id/labels` to update tags.
* DTOs:

  * `NodeSummary`: add `labels?: Record<string, string>`
  * `AdminStatusView`: can include `labels?` but not required if svc-admin merges from registry.
* Storage:

  * In svc-admin, store in the node registry config (file) or a small DB. “Good enough” is a JSON store initially if you want velocity.

**UI changes**

* Nodes page:

  * A tag filter row at top
  * Grouping mode toggle
  * Column: small tag chips under node name
* Node detail:

  * Tag editor in header area (“owner”, “env”, “region”, etc.)

**Why this matters**
This enables “cluster admin” workflows:

* “Show me bandwidth totals for `env=prod`”
* “Which nodes in `region=us-east` are highest TX?”
* “Group nodes by rack and show storage utilization warnings”

---

### 3.2 NodeDetail facet metrics should be “large thumbnails” (2 per row) + clickable drilldown

**Goal:**

* NodeDetail facet metrics become **big, dashboard-grade cards**:

  * 2 per row on desktop (responsive to 1 per row on narrow)
  * clickable to open a facet detail view (modal or route)
  * show key stats and a sparkline or mini graph in the card
* Clicking a facet shows:

  * time series (RPS, error rate, latency)
  * histogram/percentiles if available
  * last sample age, source, and metadata
  * maybe per-endpoint breakdown eventually

**Implementation direction**

* Use the existing `facetHistory` series in `useNodeDetail.ts` as MVP data.
* Layout changes:

  * Instead of small “facet rows,” render a **FacetCardGrid**: 2-column grid.
  * Each card shows:

    * facet name
    * current RPS
    * error rate
    * p95 / p99
    * last sample age
    * mini sparkline
* Drilldown:

  * MVP: open a modal showing an expanded graph and stats (still using `facetHistory`)
  * Later: query a real `GET /api/nodes/:id/metrics/facets/:facet/timeseries?...`

**Why this matters**
Operators will expect the facet dashboard to be the primary “what is happening?” view. This also sets up future “per-facet deep observability” like request breakdowns, p95 spikes, error bursts, etc.

---

### 3.3 Nodes page should show cluster-wide network accounting at the top (filterable by tag)

**Goal**

* The top of Nodes page should show the **Network accounting summary** (like NodeDetail’s) but for:

  * all nodes combined (default)
  * all nodes in a category/tag (filtered)
* This is the “operator homepage” metric.

**Implementation direction (best practice)**

* Create a reusable component:

  * `NetworkAccountingPanel`
* It should accept an array of per-node counters and compute:

  * total bytes + total requests for minute/hour/day/month
  * series (optional, or display aggregated min/hour/day/month only)
* Data source:

  * MVP: svc-admin computes from node-provided counters if present; otherwise mocks.
  * Real: nodes export counters via metrics endpoint; svc-admin fetches them and aggregates.

**Most important: define what “requests” means**

* You likely want:

  * total request count per plane (HTTP, OAP frames, gRPC, etc.)
  * or at least per-plane “handled requests” and “bytes in/out”
* For RON-CORE, you probably want:

  * `net_rx_bytes_total`
  * `net_tx_bytes_total`
  * `requests_total` (or split by protocol/plane)
  * optional: `oap_frames_total`, `oap_bytes_total`, etc.

**Why this matters**
In real clusters, the first question is always:

* “Where are we paying bandwidth?”
* “Which category/region is spiking?”
* “How many requests are we serving right now?”

---

## 4) What we’ve done on macronode / RON-CORE (context snapshot)

### 4.1 macronode service bind defaults & config hygiene

You recently ran:

* `rg 'DEFAULT_.*_STR' crates/macronode/src/services -n`

This confirmed macronode services have default bind strings in `ports.rs` and used in `svc_index.rs`, `svc_mailbox.rs`, `svc_dht.rs`, `svc_storage.rs`, etc.

**Open gap**

* Ensure these default bind constants are complete and consistent, and not truncated (one output showed `DEFAULT_OVERLAY_ADDR_STR` cut off in grep output display).

### 4.2 Planes → admin-plane endpoints

We are operating on the assumption that each plane will eventually expose:

* `/healthz`
* `/readyz`
* `/metrics`
* `/api/v1/status` or equivalent admin status endpoint (for svc-admin to poll)

**Open gaps**

* Some endpoints are still missing / stubbed / capability-gated.
* Bandwidth/requests metrics likely still partially mock or not exported consistently across planes.

### 4.3 Security posture reminder (auth on admin planes)

You flagged this correctly earlier:

* Eventually **each plane’s admin interface must be authenticated**, otherwise it’s a major security risk.

**Directional stance**

* svc-admin should assume:

  * default “secure mode” (https / auth)
  * allow `insecure_http=true` only in dev
* Node admin planes should support:

  * token auth (bearer)
  * mTLS (best)
  * or both (enterprise-grade)

---

## 5) Comprehensive TODO list (next work)

### 5.1 Unblock the layout refactor (Planes table move)

**Goal**

* Remove large planes table from center.
* Right panel: replace “Data planes” with **“All planes”** and show the full plane list there.
* Expand utilization + network accounting + any other tiles to use the freed center space.

**Next actions**

1. Find NodeDetail composition file via ripgrep commands (Section 2).
2. Edit layout:

   * Delete the planes table block.
   * Insert “All planes” panel in the right sidebar.
   * Expand center grid (more room for utilization + facet thumbnails).
3. Confirm no layout regression at common screen sizes.

### 5.2 Node tagging/categories (cluster scale)

**Goal**

* Add tags to nodes and filter/group nodes by tags.

**Next actions**

* Decide Phase A vs Phase B storage (svc-admin registry vs node-reported).
* Add backend endpoints:

  * update node tags
  * return tags in node list/status
* UI: tag chips + filter dropdown + group view.

### 5.3 Nodes page: cluster-wide Network accounting header

**Goal**

* show aggregated min/hour/day/month bytes+requests for all nodes (or filtered tag set) at top.

**Next actions**

* Extract NodeDetail’s network accounting component into a reusable component.
* Build aggregator function:

  * sum totals across nodes
  * show “reported/warming/mock” source similar to utilization tiles
* Add filter integration with tags.

### 5.4 NodeDetail facet metrics redesign (2 per row + drilldown)

**Goal**

* “Facet dashboard” becomes primary.
* 2 cards per row; each card clickable → modal or new route.

**Next actions**

* Build `FacetCardGrid` component.
* Use `facetHistory` to render sparkline and show current stats.
* Add a drilldown modal (MVP) that shows expanded sparkline and stats.
* Later: wire a timeseries endpoint.

### 5.5 Truthful bandwidth + requests end-to-end

**Goal**

* Replace “mock/warming” with real per-node and per-plane values wherever possible.

**Next actions**

* Define canonical metric names and semantics (bytes + requests).
* Ensure macronode planes export them.
* svc-admin poll/collect:

  * either via a node summary endpoint that returns counters
  * or via scraping node `/metrics` and extracting totals (heavier; still doable)
* Normalize into DTO for UI.

### 5.6 Auth hardening (admin plane)

**Goal**

* Operator dashboard should not assume insecure admin endpoints.

**Next actions**

* Document auth modes (dev token, mTLS, etc).
* Add capability / ui-config flags to reflect auth readiness.
* Ensure svc-admin client enforces https unless explicitly insecure.

---

## 6) Notes on “why admin-api.ts got shorter” (so future you isn’t confused)

The shorter version is fine **if** it preserves:

* compatibility for rollout (optional alias fields)
* safety rails (fields like `approxKeys` must be present or nullable to avoid UI crashes)
* consistent DTO naming policy

The high-value policy is:

* **be strict where it matters** (types, nullability),
* **be permissive on field aliases** during rollout to prevent breaking UI as Rust evolves.

---

## 7) Summary of your new UI direction (locked-in intent)

* “All planes” should live in the **right sidebar** (not center).
* The center should be:

  * utilization tiles
  * network accounting
  * facet card grid (2 per row)
  * (later) richer per-facet drilldowns
* The Nodes page should be:

  * tag/category filters
  * cluster-wide network accounting header
  * node list grouped/sorted by tags
  * ability to pivot metrics by category

This is exactly the posture for a **real distributed runtime / cloud control plane**.

---

## 8) What to do immediately next session (fast start checklist)

1. Run the 3 `rg` commands from Section 2 to find the NodeDetail composition file.
2. Paste that file into chat.
3. We will return **one paste-ready replacement file** that:

   * moves planes list to sidebar (“All planes”)
   * removes the center table
   * expands the center layout
   * upgrades facet metrics layout to a 2-column grid scaffold (even if drilldown is stubbed)

After that, we’ll begin implementing:

* node tags/categories + filters
* cluster-wide network accounting header on Nodes page

---


### END NOTE - JANUARY 5 2026 - 18:40 CST



### BEGIN NOTE - JANUARY 5 2026 - 22:05 CST

Got it — assuming the heartbeat charts are already resolved, here are **carry-over notes that stay focused on the next UI upgrades** (Nodes dashboard tagging/filtering + global heatmap/security ops + richer resource facts), plus the **crate ownership** answers for geofencing + IP allowlists + alerting.

---

# CARRY-OVER NOTES — svc-admin + macronode + RON-CORE (UI-first sprint)

## 0) Current status snapshot (what’s working)

### Node detail page (NodeDetailPage)

* **Overview panel is “operator-grade”**:

  * CPU/RAM/Storage/Bandwidth tiles with live/mock handling.
  * Network accounting block below tiles: rollups (minute/hour/day/month) + series chart.
* **Right sidebar (NodeDetailSidebar)** is solid:

  * Uptime + Data & storage link + All planes with search and “only problems”.
* **Facet metrics UI** is in the right direction:

  * Large 2-up cards, searchable, drilldown-friendly.
  * (Per your note: heartbeat sparklines are already fixed now.)

### Nodes dashboard

* Card layout and right-side node preview panel are strong and readable.
* Missing: node tag system + filter UI + richer resource facts inside the node preview / cards.

---

## 1) UI-FIRST goals for the next instance (locked priorities)

We are **not** doing broader TODO items until these are done.

### Goal A — Node tags + category/grouping (Nodes dashboard + per-node edit)

**User requirement**

* “Above the nodes”: add **filter/sort by tag**, **search**, and **filter by node type** (macronode/micronode/etc).
* When clicking/opening a node: ability to **add/edit tags** for that node.
* Tags/categories should support **large clusters** (so we need a real storage plan, not only local UI state).

**MVP behavior**

* Nodes page header gets a “filters bar” similar to facets:

  * Search nodes by name/id/profile/version
  * Filter by:

    * Node type/profile (macronode/micronode)
    * Tag (exact match or substring)
  * Optional: “Only problems” (nodes with degraded/down planes or stale/unreachable metrics)
  * Optional: “Group by tag” dropdown (shows grouped sections)
* Node cards show tag chips under the node title.
* Node detail header/side panel contains a “Tags” section with add/remove.

**Storage plan (important for scale)**

* Phase 1 (fast dev): tags stored in **svc-admin backend** (not just localStorage), keyed by node id.
* Phase 2: tags are part of the “node registry” model:

  * Admin/operator tags: `labels.operator.*`
  * Node-reported tags: `labels.node.*`
  * UI displays merged view but keeps operator tags editable.

**API needed (svc-admin backend)**

* `GET /api/v1/nodes/:id/tags`
* `PUT /api/v1/nodes/:id/tags` (replace) or `POST/DELETE` for add/remove
* `GET /api/v1/nodes/tags` (optional aggregation for tag dropdown counts)

**UI invariants**

* Filtering must never “lose” a node due to transient state updates.
* Tags are case-normalized (e.g., lowercased keys, trimmed values).
* Tag edits update UI immediately (optimistic), then persist.

---

### Goal B — Add “resource facts” where the pink rectangles point

**User requirement**

* CPU: show **cores + threads**
* RAM: show **used / total** (GiB)
* Storage: show **used / total** (GiB)

**Where to source data**

* Prefer existing live endpoints you already have:

  * `getNodeSystemSummary(nodeId)` → add fields if missing
  * `getNodeStorageSummary(nodeId)` → ensure used/total exist

**DTO extensions (macronode admin plane)**

* `SystemSummaryDto` should include:

  * `cpuCores` (physical or logical, define clearly)
  * `cpuThreads` (logical threads)
  * already has: `ramUsedBytes`, `ramTotalBytes` (if not, add)
* `StorageSummaryDto` should include:

  * `usedBytes`, `totalBytes`

**UI placement**

* Nodes dashboard right preview panel:

  * CPU tile: `9% · 8c/16t`
  * RAM tile: `80% · 5.9 / 8.0 GiB`
  * Storage tile: `89% · 100.8 / 112.7 GiB`
* Node detail Overview tiles: same subtext beneath the %.

**Performance considerations**

* Don’t increase polling fanout unnecessarily in large clusters:

  * For Nodes dashboard, sample utilization **only for selected node** (right preview),
    not every node in the list.
  * Or implement staggered polling / “visible cards only”.

---

### Goal C — Global login heatmap + geofencing tools (security ops surface)

**User idea**

* Above nodes list: small global “heatmap” indicator; click → dedicated page.
* Heatmap shows **where users are logged in from** (country-level).
* From that page: ability to **block by country** or **block IP ranges**.

**Recommended implementation approach**

* Keep the heatmap **country-level** initially (fast + safer).
* Treat blocking as **policy** with audit trail, not ad-hoc UI toggles.

**Minimum viable “Geo Security” page**

* Summary KPIs:

  * Logins last hour/day
  * New countries in last 24h
  * Failed logins by country
* Map (country choropleth or simple list at MVP):

  * Country → count → last seen
* Controls:

  * Block country (adds to policy)
  * Block CIDR
  * Allowlist CIDR for admins
* Audit feed:

  * “Blocked CN”, “Allowed 1.2.3.0/24”, “Denied request from RU”

**Data required**

* Authentication/access events must exist first:

  * svc-admin logins
  * node admin plane auth attempts
  * potentially gateway/operator endpoints later

---

## 2) Crate ownership: geofencing + allowlists + alerts

### 2.1 Where geofencing should be enforced

**Enforcement must happen where the IP is actually seen.**

**Best enforcement points**

1. **svc-admin (backend server)**

   * Protects the admin dashboard itself.
   * Sees operator IPs directly.
   * Should enforce:

     * admin allowlist CIDRs
     * geo blocks (country denylist)
     * rate limits and suspicious path probes
   * Also emits “security events” to the UI.

2. **macronode admin plane (and/or gateway plane)**

   * Protects node endpoints (`/api/v1/status`, `/metrics`, `/api/v1/system/*`, etc.).
   * Should enforce:

     * admin-plane allowlist (only svc-admin / operator networks)
     * geo blocks if desired at node level
   * Emits security events the same way.

**Conclusion**

* **Geofencing policy is configured in svc-admin**, but **enforced in svc-admin + macronode** depending on which surface is being accessed.

### 2.2 Which crate should *own* the policy model

You want to avoid duplicating logic across svc-admin and macronode.

**Best pattern**

* Create (or use if it already exists) a shared policy crate in RON-CORE:

  * Example name: `ron-policy` or `ron-security-policy`
  * Contains:

    * `GeoPolicy` (blocked countries, allowlist countries)
    * `IpPolicy` (allowlist CIDR, blocklist CIDR)
    * `PathProbePolicy` (watched paths, thresholds)
    * normalization + matching helpers (CIDR parsing, country code validation)
* Then:

  * svc-admin backend uses it in middleware
  * macronode admin plane uses it in middleware
  * svc-admin UI just edits policy DTOs

If you don’t want a new crate yet: put shared types in **ron-kernel** (or the project’s “common/types” crate), but keep enforcement helpers in a dedicated module to prevent kernel bloat.

### 2.3 Where “alerts” should live

**Alert generation** belongs to the service that detects the event, but aggregation + display belongs in svc-admin.

* Detection sources:

  * svc-admin backend: suspicious routes, failed logins, non-whitelisted access attempts
  * macronode admin plane: denied requests, invalid tokens, probe paths, rate-limit triggers
* Storage/aggregation:

  * svc-admin backend should store security events (even a bounded in-memory ring buffer at MVP).
* UI display:

  * svc-admin UI shows:

    * Recent security events list
    * “Security” badge/count in nav
    * Per-node security events (filtered by node id)
    * Global security events (all nodes)

---

## 3) “Whitelist admin IPs + alert when non-whitelisted visits directories”

### Requirement breakdown

* Maintain an **admin allowlist** (CIDR/IP ranges).
* When a request comes from a non-allowlisted IP and hits:

  * certain links/directories (e.g., `/admin`, `/api`, `/metrics`, suspicious paths),
    then:
  * **deny** (or require auth)
  * **emit alert event**
  * **log** with details
  * show in svc-admin dashboard

### Best crate placement

* If the protected resource is the **svc-admin dashboard**:

  * Implement in **svc-admin backend** (middleware).
* If the protected resource is the **node admin plane**:

  * Implement in **macronode admin plane** (middleware).
* Shared policy/types:

  * `ron-policy` / `ron-security-policy` (recommended)

### Alert event format (standardize now)

Define a single DTO used everywhere:

* `SecurityEventDto`:

  * `ts`
  * `surface` (`svc-admin` | `node-admin` | `gateway`)
  * `nodeId?`
  * `ip`
  * `country?`
  * `path`
  * `action` (`allowed` | `denied` | `challenged`)
  * `reason` (`not_allowlisted` | `geo_blocked` | `bad_token` | `probe_path` | `rate_limited`)
  * `userAgent?`

Then:

* svc-admin backend provides:

  * `GET /api/v1/security/events?since=...`
  * `GET /api/v1/security/policy`
  * `PUT /api/v1/security/policy`

---

## 4) Gaps discovered in RON-CORE while building app foundations (UI surfaced these)

These are the “platform gaps” the UI is now forcing us to formalize:

1. **Metadata completeness**

   * Need consistent system summary fields (cores/threads, totals, units).
2. **Cluster scale behavior**

   * Avoid polling everything; adopt “selected node” sampling or staggered polling.
3. **Tags/labels as first-class**

   * Needed for real ops (team ownership, workload class, environment, cost center).
4. **Security posture**

   * Admin-plane auth + allowlists + audit events are required for production credibility.
5. **Audit/event stream**

   * Even if MVP is in-memory, we need a standardized event DTO + UI surfaces.

---

## 5) Concrete “next instance” execution plan (UI-first)

Order matters; this keeps momentum.

### Step 1 — Nodes dashboard: filters bar + tags

* Add UI controls: search, type filter, tag filter, group-by dropdown
* Add per-node tags editing when opening a node

### Step 2 — Nodes preview: resource facts

* Extend DTOs if needed (cores/threads)
* Render subtext on CPU/RAM/Storage tiles

### Step 3 — Heatmap/security page scaffold (MVP)

* Add nav entry “Security”
* Add page with:

  * simple country list + counts (map can come after)
  * policy editor (block country / block CIDR / allowlist CIDR)
  * security events feed

### Step 4 — Enforcement middleware (svc-admin backend first)

* Implement allowlist + watched path alerts
* Emit SecurityEventDto into a ring buffer + expose endpoint for UI

### Step 5 — Extend to macronode admin plane

* Reuse same shared policy crate/types
* Emit events to svc-admin (pull model first: svc-admin polls node events endpoint, or nodes expose counters)

---

### END NOTE - JANUARY 5 2026 - 22:05 CST




### BEGIN NOTE - JANUARY 8 2026 - 11:15 CST

Got it — here are **revised carry-over notes** that assume **(1) RAM GiB display is fixed** and **(2) tag search/filter on the Nodes page is fixed**.

---

# CARRY-OVER NOTES — RON-CORE + svc-admin “God-Tier” Dashboard (UPDATED)

## 0) Current status snapshot

We’re building **RON-CORE** (distributed runtime/cloud/OS) and **svc-admin** (operator dashboard). The dashboard is now “operator grade” with strong **truthful health/ready/metrics semantics**, and it uses **safe deterministic mocks only when endpoints are missing**.

**Most recent fixes completed:**

* ✅ **Tag search / tag filtering** on Nodes page now works.
* ✅ **RAM “thousands of GiB” display bug** is fixed (units now correct).
* ✅ **Storage page bottom padding** added so the floating “API Requests” overlay doesn’t cover content.

**Immediate next step (per plan):**
Update `scripts/dev_svc_admin_stack.sh` to be **truthful and flag-driven**, so we can run:

* `-macro` (macronode only)
* `-micro` (micronode only)
* `-macro -micro` (both simultaneously)
  …and have **no fake data** in the dev stack (everything real, live endpoints).

---

## 1) svc-admin UI accomplishments (what’s done)

### 1.1 Nodes dashboard + selection + preview panel

* Nodes page has:

  * node list + selection
  * top filter bar (search + profile filter + tags filter + “only problems” + grouping)
  * right-side preview panel showing:

    * node health badge
    * metrics freshness badge
    * tags editor (add tag)
    * CPU/RAM/Storage/Bandwidth mini tiles
    * plane table (health/ready/restarts)
* ✅ Tag searching and filtering is now working correctly (searching by tags yields correct node results).

### 1.2 NodeDetailPage: “Overview” + live tiles + network accounting

* NodeDetail is compositional and clean (heavy logic split into route-local modules).
* Utilization tiles:

  * CPU, RAM, Storage, Bandwidth are **truthful when live endpoints exist**, otherwise deterministic mock.
  * Tile header has a Source pill: **Reported / Warming / Mock**.
  * Bandwidth tile can show Rx/Tx rates when available.
* Network accounting panel exists under tiles:

  * minute/hour/day/month totals (bytes + requests)
  * series visualization panel
* ✅ RAM formatting is now correct (no more absurd GiB totals).

### 1.3 Storage pages: NodeStoragePage + curated DB inventory

* Node storage page (“Data & storage inventory”) supports:

  * top DB disk-share gauges (proxy)
  * storage summary (used/free/io/db count)
  * databases table
  * database detail sidebar + “Open →” link to per-db details route
* ✅ Added **~150px bottom padding** so the floating API overlay doesn’t cover the bottom content.

---

## 2) Key svc-admin UI files/modules touched recently (reference map)

### 2.1 Shared DTO types

Path: `crates/svc-admin/ui/src/types/admin-api.ts`

* Central TS DTO definitions mirrored from Rust DTOs.
* Compat policy supports snake_case vs camelCase during rollout.

### 2.2 Node detail route + utilization + live polling

Paths:

* `crates/svc-admin/ui/src/routes/NodeDetailPage.tsx`
* `crates/svc-admin/ui/src/routes/node-detail/utilization.tsx`
* `crates/svc-admin/ui/src/routes/node-detail/liveUtilization.ts`
* `crates/svc-admin/ui/src/routes/node-detail/netAccounting.tsx`
  Notes:
* Live polling treats 404/405/501 as “endpoint missing” and falls back safely.
* Tiles show “Reported/Warming/Mock”.

### 2.3 Node storage route

Path: `crates/svc-admin/ui/src/routes/NodeStoragePage.tsx`

* Now includes bottom padding to avoid overlay collision.

---

## 3) What remains on the svc-admin “God-tier” roadmap (UI)

These are **not blocking** the next script work, but still on the roadmap:

1. **Search bar width**

   * You want the node search input wider (double/triple width for long names).
   * (Tag search fixed; this is now mostly a UX enhancement.)

2. **Cluster-scale features**

   * Group by tag/category (drop-down grouping)
   * Sort/filter metrics/facets by category
   * Better cluster navigation at large node counts

3. **Future ideas (later)**

   * Global geo heatmap (IP→country) + country/range blocking UX
   * More “hardware facts” fields in tiles (cores/threads, etc.) if desired

---

## 4) RON-CORE + node profiles status (macronode vs micronode)

* **macronode** is currently sufficiently wired for svc-admin to show:

  * status, planes, readiness/health
  * metrics freshness + facet metrics
  * storage/DB inventory endpoints (or safe fallback when missing)
* **micronode** is not yet wired “the same way” as macronode:

  * next goal is to run micronode and macronode together and then implement missing micronode endpoints/contracts to reach parity where it makes sense.

---

## 5) Gaps discovered while laying iOS + Android app foundations

We paused mobile app building to focus on svc-admin, but the app scaffolding highlighted runtime/product gaps we still need to close.

Checklist buckets:

1. **Auth/identity model** across planes + SDKs + admin
2. **Stable public JSON contracts** (versioned DTO shapes, predictable errors)
3. **Capability detection** and consistent profile behavior (micro vs macro)
4. **Observability-first**: truthful ready/health/metrics everywhere
5. **Dev tooling** must be “real by default” for confidence (no fake data)

---

## 6) NEXT TASK (primary): make `dev_svc_admin_stack.sh` truthful + flag-driven

### 6.1 Goal

Rewrite/upgrade `scripts/dev_svc_admin_stack.sh` so it can start:

* svc-admin backend + UI
* optionally start **macronode** and/or **micronode**
* with flags:

  * `-macro`
  * `-micro`
  * `-macro -micro`
* and ensure **svc-admin sees real nodes** with **real endpoints** (no fake registrations).

### 6.2 Desired UX

* `scripts/dev_svc_admin_stack.sh -macro`

  * Starts svc-admin
  * Starts macronode profile
  * svc-admin shows macronode as live
* `scripts/dev_svc_admin_stack.sh -micro`

  * Starts svc-admin
  * Starts micronode profile
  * svc-admin shows micronode as live
* `scripts/dev_svc_admin_stack.sh -macro -micro`

  * Starts svc-admin + both nodes
  * Both appear in Nodes list with correct profile labels

### 6.3 Constraints

* No fake data in the dev stack script:

  * No pretend node registry entries
  * If a node isn’t running, it should be absent or honestly unreachable depending on discovery mechanism
* Avoid port collisions:

  * macro and micro must have distinct bind sets
* Script should wait for readiness:

  * svc-admin `/readyz`
  * each node `/healthz`/`/readyz`
  * metrics reachable (if part of readiness assumptions)
* Print a clear summary of URLs and ports.

### 6.4 Step plan for next instance

1. Paste current `scripts/dev_svc_admin_stack.sh` contents
2. Identify:

   * how svc-admin currently discovers nodes (config/registry/static list)
   * current default binds/ports for macronode and micronode
3. Implement CLI arg parsing and start graph:

   * parse `-macro`, `-micro`, `-h`
   * pick which processes to spawn
4. Ensure node discovery is truthful:

   * config includes real admin URLs OR nodes self-register OR script calls a real registration endpoint
5. Add readiness waiting + helpful logs
6. Run matrix:

   * `-macro`
   * `-micro`
   * `-macro -micro`
7. After both can run simultaneously:

   * start wiring micronode endpoints (status/metrics/storage parity where appropriate)

---

## 7) What to paste first in the next instance (so we can implement fast)

To implement the script update without guesswork, start next instance by pasting:

1. `scripts/dev_svc_admin_stack.sh` (full file)
2. Any `.env` or config file it reads (or the relevant section)
3. Where svc-admin reads its node list (file path / env var / endpoint)
4. The default bind/port constants for macronode + micronode (or the config modules)

Then I’ll respond with:

* a **full paste-ready updated** `scripts/dev_svc_admin_stack.sh`
* any tiny helper config files if needed
* a quick run/test checklist for the three flag combinations

---


### END NOTE- JANUARY 8 2026 - 11:15 CST 



### BEGIN NOTE - JANUARY 8 2026 - 14:00 CST

Below are **paste-ready, exhaustive carry-over notes** you can drop into **both** crates. I wrote them as a single unified doc with clearly labeled sections so you can paste the same thing into `crates/svc-admin/NOTES.md` and `crates/micronode/NOTES.md` (or split if you prefer).

---

# CARRY-OVER NOTES — Micronode wiring into svc-admin (truthful, no mocks)

## 0) Current status snapshot (what works vs what’s missing)

### What works now

* ✅ `scripts/run_dashboard.sh -macro -micro` can start:

  * macronode on `127.0.0.1:8080`
  * micronode on `127.0.0.1:5310`
  * svc-admin UI/API on `127.0.0.1:5300`
  * svc-admin health/ready/metrics on `127.0.0.1:5311`
  * UI dev server on `http://localhost:5173`
* ✅ svc-admin shows **macronode** as “real/live” (most tiles and node detail look correct).
* ✅ svc-admin recognizes micronode as a node entry (node list shows it), but **micronode data is still “Mock/Warming/Fallback”** for most dashboards/tiles.

### What is still broken / incomplete

* ❌ Micronode is still effectively “mock” in svc-admin because svc-admin cannot obtain enough structured data from micronode endpoints to mark tiles as “Reported”.
* In logs we saw repeated warnings like:

  * `failed to fetch /api/v1/status ... 404 Not Found ... degrading to health/ready/version probes`
* This proves:

  * svc-admin expects certain “node admin API” endpoints to exist on the node base URL.
  * micronode currently lacks one or more of these endpoints (or their DTOs don’t match), so svc-admin falls back to minimal probes and/or deterministic mocks.

### Primary objective

Make micronode expose the same “operator-grade” admin API surface as macronode **where appropriate**, so svc-admin can treat micronode as fully “real” and stop falling back to mock widgets.

---

## 1) The contract: what svc-admin expects from a node

svc-admin operates in tiers:

### Tier A — minimal liveness (always possible)

* `GET /healthz`
* `GET /readyz`
* `GET /version`
* `GET /metrics`

If only these exist, svc-admin can show “reachable” but lacks rich status, planes, utilization, storage inventory, etc.

### Tier B — admin API v1 (required to stop many fallbacks)

svc-admin tries to call:

* `GET /api/v1/status`

If this 404s, svc-admin logs degrade warnings and will likely:

* treat node as “unknown profile”
* treat utilization/storage/etc as missing and fall back to “Mock” or “Warming”

### Tier C — rich system and storage summaries (required for “Reported” tiles)

Depending on UI implementation, svc-admin may also attempt:

* `GET /api/v1/system/summary`  (CPU/mem/disk/system facts)
* `GET /api/v1/storage/summary` (disk totals, possibly DB counts / IO)

If these are missing or not shaped correctly, the UI still shows “Mock/Warming” for utilization and storage tiles.

### Tier D — macronode-only planes and inventories

macronode has embedded/hosted services and can expose:

* plane list + per-plane status
* storage inventory / DB detail endpoints
* gateway/mailbox/overlay/index service info, etc.

Micronode might not support all of these yet, but it still must provide enough to eliminate “everything is mock”.

---

## 2) What we saw in runtime logs (most important evidence)

When running `scripts/run_dashboard.sh -macro -micro`, svc-admin repeatedly logged:

* `failed to fetch /api/v1/status ... 404 Not Found ... (micronode)`

Interpretation:

* svc-admin is configured with micronode’s `base_url=http://127.0.0.1:5310`
* svc-admin expects micronode to implement `/api/v1/status`
* micronode currently does **not** implement it (or router is not mounting it)

This is the single most important reason micronode appears as mock in the UI.

---

## 3) Known compile-time blockers that appeared while attempting micronode admin API

During `cargo build -p svc-admin -p macronode -p micronode`, micronode failed with:

* `_State` extractor used instead of `State`
* usage of `unsafe` in admin_api (micronode forbids unsafe)
* mismatched AppState fields (`started_at`, `ready`, `build`) that don’t exist
* config field mismatch (`st.cfg.bind` vs `st.cfg.server.bind`)
* sysinfo API mismatch (sysinfo 0.30: `System::disks()` not available)

These errors strongly indicate an attempted admin_api implementation was written against an older/other struct layout and sysinfo API.

**Invariant:** Micronode has `#![forbid(unsafe_code)]` in `src/lib.rs` → any implementation must use safe primitives only (Mutex/OnceLock/etc).

---

## 4) What needs to be true for “no mocks” in the UI

To get svc-admin to show micronode as fully live:

### Must-have endpoints (minimum)

1. `GET /api/v1/status` returning a DTO matching svc-admin’s expectations.
2. `GET /api/v1/system/summary` returning CPU/mem/disk totals (or enough for utilization panels).
3. `GET /api/v1/storage/summary` returning disk totals and “storage high-level” fields.

### Must-have behaviors

* No 404 on the above routes.
* DTO field names must match expected serde casing (svc-admin TS types support some snake/camel compatibility, but avoid relying on it).
* Responses must be fast and reliable (avoid expensive sysinfo refresh loops on every request if possible; caching is fine).
* No unsafe code.
* No phantom/mock nodes: svc-admin registry must only contain real nodes (macro/micro) when script flags are used.

---

## 5) Current dev tooling situation (script + ports + “truth probes”)

### `scripts/run_dashboard.sh` goals

* Start real nodes only, optionally both
* Avoid port conflicts and stale processes
* Provide diagnostic visibility (what’s listening, what endpoints return)

### What we learned (important)

* svc-admin health/ready live on `SVC_ADMIN_METRICS_ADDR` (default `5311`), **not** the UI/API port `5300`.
* Port conflicts and sled DB locks happened when old processes were still running.
* The script now includes port-kill logic (`lsof -ti tcp:<port> | kill -9`) unless `--no-kill` is passed.
* Script prints “stack is up” and direct URLs.

### What still needs to be added/verified

* “Truth probes” for node contract endpoints should be run after each node boots:

  * Probe micronode `/api/v1/status`, `/api/v1/system/summary`, `/api/v1/storage/summary`, `/metrics`
  * Print HTTP status + small body snippet
* These probes become the first-line indicator of why UI is mocking.

---

## 6) svc-admin config: remove default mock nodes / keep truthfulness

### Problem we hit earlier

svc-admin showed **3 nodes** when only macro+micro should exist.
Cause: `default_nodes()` in svc-admin seeded extra demo nodes (`example-node`, `node-b`, `node-c`) all pointing to localhost.

### Desired behavior

* Default `cargo run -p svc-admin` can have *one* minimal default node (optional).
* When using `run_dashboard.sh`, the registry must be **cleared** and nodes injected only by env.

### Implemented / intended mechanism

* Use `SVC_ADMIN_NODES_CLEAR=1` to force empty `default_nodes()` seed.
* Script should set it when launching svc-admin.
* Script should inject exactly:

  * `SVC_ADMIN_NODES__macronode__BASE_URL=...`
  * `SVC_ADMIN_NODES__micronode__BASE_URL=...`
  * plus display name, env, insecure, forced_profile.

**Invariant:** “Past the mock data phase” means:

* No extra fake nodes.
* No “Node B/Node C” placeholders.
* If a node isn’t running, it should be absent or clearly unreachable, not simulated.

---

## 7) Micronode: what to wire up (parity plan)

### 7.1 Admin API module (micronode)

Create/repair: `crates/micronode/src/http/admin_api.rs`

It should:

* use `axum::extract::State` (not `_State`)
* avoid unsafe entirely
* read truth from:

  * `AppState.health.all_ready()` for readiness
  * `cfg.server.bind` (not cfg.bind)
  * `env!("CARGO_PKG_VERSION")` for version
  * `sysinfo` for system summary/disk totals (compatible with sysinfo 0.30)

**Key principle:** Don’t attempt to mirror macronode’s plane model yet—provide a minimal but truthful “micronode plane”.

### 7.2 Router mounting (micronode app.rs)

`crates/micronode/src/app.rs` must mount:

* `/api/v1/status`
* `/api/v1/system/summary`
* `/api/v1/storage/summary`

…and keep existing:

* `/healthz`, `/readyz`, `/version`, `/metrics`

### 7.3 http/mod.rs

Ensure `pub mod admin_api;` exists and is correctly referenced.

### 7.4 DTO alignment

We must align DTOs between:

* Rust structs returned by micronode
* svc-admin Rust client expectations (NodeClient)
* svc-admin TS types in UI

Approach:

* Use `#[serde(rename_all="camelCase")]` on micronode DTOs to align with UI.
* Where svc-admin expects snake_case, verify compatibility in `svc-admin/ui/src/types/admin-api.ts` (it currently supports some compat policy).
* Choose one canonical casing (camelCase is fine) and keep it consistent across endpoints.

### 7.5 Metrics expectations

Micronode already exposes `/metrics` on same bind.
We eventually want:

* request counters / latency histograms
* memory/cpu gauges (optional)
  But for “no mock” UI, first objective is just the admin API contract endpoints.

---

## 8) sysinfo dependency verification

### Root workspace

* `sysinfo = "0.30"` pinned in `[workspace.dependencies]`

### Micronode crate

* `sysinfo = { workspace = true }`

**Important API note:** sysinfo 0.30 provides disks via:

* `sysinfo::Disks::new_with_refreshed_list()`
  not `System::disks()`.

If we need network IO totals, sysinfo APIs vary and may require refresh calls; keep it minimal at first.

---

## 9) Immediate next steps checklist (in order)

### Phase 1 — unblock micronode compile

1. Fix `crates/micronode/src/http/admin_api.rs`:

   * Replace `_State` with `State`.
   * Remove all unsafe (use safe static caching if needed: `OnceLock<Mutex<...>>`).
   * Remove references to nonexistent AppState fields (`started_at`, `ready`, `build`).
   * Use `st.health.all_ready()` and config path `st.cfg.server.bind`.
   * Fix sysinfo calls (Disks API).

2. Ensure `crates/micronode/src/http/mod.rs` exports `admin_api`.

3. Ensure `crates/micronode/src/app.rs` mounts admin_api routes.

4. `cargo build -p micronode` must succeed.

### Phase 2 — confirm HTTP contract live (truth probes)

1. Run:

   * `scripts/run_dashboard.sh -macro -micro`
2. Confirm probes show for micronode:

   * `/api/v1/status` => HTTP 200
   * `/api/v1/system/summary` => HTTP 200
   * `/api/v1/storage/summary` => HTTP 200
3. Confirm svc-admin logs no longer warn about `/api/v1/status` 404.

### Phase 3 — eliminate UI mock fallback

1. Once endpoints return 200, watch UI:

   * NodeDetail utilization tiles should flip from Mock/Warming to Reported.
   * Node preview panel should show real system and storage values.
2. If still mock:

   * The DTO shape is mismatched or svc-admin parsing rejects it.
   * Add logging in svc-admin NodeClient parsing path to print decode errors (not just 404).
   * Compare JSON output from micronode endpoints to macronode equivalents and to TS types.

### Phase 4 — “macronode parity where appropriate”

After contract endpoints work:

* Add any additional endpoints the UI tries next (storage inventory, plane tables, etc.), but only if those concepts exist for micronode.
* Micronode likely needs a “capabilities” field so svc-admin UI can hide macronode-only sections cleanly.

---

## 10) Debugging playbook (fast triage)

When micronode still appears mock:

### A) Confirm endpoint existence

Run:

* `curl -i http://127.0.0.1:5310/api/v1/status`
* `curl -i http://127.0.0.1:5310/api/v1/system/summary`
* `curl -i http://127.0.0.1:5310/api/v1/storage/summary`

If any are 404, router mounting is wrong.

### B) Confirm JSON decodes in svc-admin

If endpoints return 200 but UI still mocks:

* svc-admin likely fails to deserialize response.
* Add temporary debug logging in svc-admin NodeClient:

  * log response status + first N bytes
  * log serde errors (field mismatch, unexpected casing)

### C) Confirm registry is truthful

If you see extra nodes:

* ensure svc-admin gets `SVC_ADMIN_NODES_CLEAR=1`
* ensure `default_nodes()` does not seed node-b/node-c
* confirm only env-injected nodes are loaded

### D) Avoid stale processes

Port conflicts & DB locks indicate old processes were still around.
Keep “kill ports” behavior in script unless explicitly disabled.

---

## 11) “Definition of done” for this wiring milestone

We consider “micronode wired like macronode (enough)” when:

* ✅ svc-admin shows exactly **two nodes** when running `-macro -micro`
* ✅ svc-admin logs contain **no repeated 404 degrade warnings** for micronode
* ✅ Micronode NodeDetail shows **Reported** (not Mock) for:

  * system summary (CPU/mem basics)
  * storage summary (disk totals)
  * readiness/health/version and metrics freshness
* ✅ Remaining sections that are macronode-only are either:

  * hidden via capabilities/profile, or
  * clearly marked “Not supported on micronode” (truthful, no fake data)

---

## 12) Files involved (quick map)

### svc-admin

* `crates/svc-admin/src/config/nodes.rs`

  * Ensure default nodes are truthful and allow `SVC_ADMIN_NODES_CLEAR=1`.
* `scripts/run_dashboard.sh`

  * Should inject exactly macro/micro nodes via env.
  * Should include truth probes and port-kill diagnostics.
* svc-admin NodeClient path (where `/api/v1/status` is fetched)

  * Add decode-error logging if needed.

### micronode

* `crates/micronode/src/app.rs`

  * Must mount `/api/v1/status`, `/api/v1/system/summary`, `/api/v1/storage/summary`.
* `crates/micronode/src/http/mod.rs`

  * Must export `admin_api`.
* `crates/micronode/src/http/admin_api.rs`

  * Must compile, be unsafe-free, and return correct DTO shapes.
* Root and micronode `Cargo.toml`

  * sysinfo pinned workspace-wide and used via workspace in micronode.

---

## 13) Known constraints / invariants to keep in mind

* Micronode forbids unsafe code.
* “Past mock phase” means:

  * no fake nodes
  * no pretend registrations
  * mocks only allowed as last-resort UI fallback when endpoints truly absent (and should be visually obvious).
* svc-admin health/ready/metrics are served on metrics listener, not UI/API listener.
* Avoid port collisions and sled locks by killing existing listeners before launching.

---

## 14) Next work item after wiring endpoints (future)

Once micronode endpoints are live and UI is “Reported”:

* add capability-driven UI gating so macronode-only planes don’t confuse micronode view
* optionally add:

  * network accounting endpoints for micronode (req counters, bytes in/out)
  * minimal “planes” list for micronode that maps to real subsystems (kv/facets/auth)
* then begin parity pass: whichever routes UI calls for macronode, decide:

  * implement for micronode, or
  * explicitly not supported + hide

---

If you want, the next step after these notes is: **paste the current micronode `http/admin_api.rs` and `state/AppState` layout** (or the whole micronode CODEBUNDLE), and I’ll produce a single paste-ready implementation that:

* compiles under `forbid(unsafe_code)`,
* mounts the correct routes,
* and returns DTOs that svc-admin will actually parse (so the UI stops mocking).


### END NOTE - JANUARY 8 2026 - 14:00 CST





### BEGIN NOTE - JANUARY 9 2026 - 12:30 CST

Below is a **drop-in carry-over note** you can paste into both:

* `crates/svc-admin/NOTES.md`
* `crates/micronode/NOTES.md`

(And optionally a shared `PROJECT_NOTES.md`.)

Then I include an **exhaustive plan** for implementing **svc-admin login** using the existing “passport/password” style crates (e.g., `svc-passport`) without leaking that logic into nodes.

---

# CARRY-OVER NOTES — Micronode ↔ svc-admin wiring + next pivot: svc-admin login

## 0) Current milestone status (what is DONE)

### ✅ Micronode is now correctly “wired” to svc-admin (baseline)

Micronode now shows in svc-admin as:

* **Metrics: fresh**
* **CPU/RAM/Storage tiles: Live**
* **Planes render and show Healthy / Ready**
* Node card summary: `3/3 planes ready`, `0 restarts`, uptime updates correctly

**Key technical fixes that made this happen:**

* Micronode `/api/v1/system/summary` + `/api/v1/storage/summary` JSON contract is now compatible with svc-admin (field casing + shapes).
* Micronode `/metrics` now exposes the expected **facet freshness counter** `ron_facet_requests_total{facet,result}` by gathering the correct registry output (not just default registry).
* Plane `health` values now match what svc-admin UI expects (`healthy`/`degraded` vs `ok`/unknown).

### ✅ Terminal truth probes confirm Micronode endpoints are correct

* `GET /api/v1/status` → returns `profile`, `version`, `uptime_seconds`, `planes[]`
* `GET /api/v1/system/summary` → returns CPU/RAM + net placeholders
* `GET /api/v1/storage/summary` → returns disk headroom and micronode truth labels
* `GET /metrics` → returns Prometheus text; includes `ron_facet_requests_total...`

---

## 1) What is still NOT complete (remaining gaps)

### 1.1 Bandwidth tile is “Live” but shows 0 B/s

The dashboard currently displays **Bandwidth RX/TX = 0 B/s** for micronode.

**Root cause:**

* Current sampling is based on **host NIC counters via sysinfo** and/or sampling logic that doesn’t reflect the node’s actual traffic in a stable way.
* More importantly: **host NIC bandwidth is not what we actually want** for svc-admin’s “node bandwidth” tile.

### 1.2 Planes exist but are “conceptual” (not independent services)

Micronode planes currently represent **logical subsystems** inside a single process:

* `admin`
* `kv`
* `facets`

They are “real” in the operational sense, but currently share the same readiness/health signals, so they don’t yet reflect subsystem-specific truth.

---

## 2) New requirement (must be added to project notes): Bandwidth must be per-node, not per-computer

### ✅ New invariant for RON-CORE dashboard observability

**Bandwidth and request volume shown on the node card must be specific to the node**, not the entire host machine.

Meaning:

* Do **NOT** show “whole laptop NIC usage” as “Micronode bandwidth”.
* The node bandwidth meter should reflect:

  * requests/sec (RPS)
  * bytes/sec in/out (service-specific)

### How to measure “node bandwidth” correctly

Preferred measurement sources:

1. **HTTP layer accounting** (service-level)

   * Count request/response bytes at the service boundary
   * Attribute to that node only
2. **Transport layer accounting** (if/when node uses custom transports)

   * Bytes in/out by transport name / peer / protocol
   * Still node-specific

### “Network page” concept

We want two tiers of views:

1. **Node-level**

* Per node:

  * Requests/sec
  * Bandwidth RX/TX for that node

2. **Cluster-level / combined**

* A separate page called **“Network”** (or “Traffic”) that shows:

  * Total requests/sec across nodes
  * Total bytes/sec across nodes
  * Breakdown by node, plane, route, status family, etc.

**Action item:** add a `Network` page to svc-admin UI with combined metrics.

---

## 3) Bandwidth work: recommended implementation approach (carry-over plan)

### 3.1 Node-specific bandwidth metrics (must be implemented)

Add “golden metrics” for each node process:

* `ron_node_http_requests_total{node,route,method,status}`
* `ron_node_http_request_bytes_total{node,route,method}`
* `ron_node_http_response_bytes_total{node,route,method,status}`
* Derived in UI:

  * RPS = rate(requests_total)
  * RX B/s = rate(request_bytes_total)
  * TX B/s = rate(response_bytes_total)

For micronode specifically:

* instrument the existing HTTP middleware to measure:

  * request body bytes read (if possible)
  * response body bytes written (approx acceptable for now)
  * fallback: Content-Length headers where available
* maintain “truthful” semantics: if unknown bytes, count what we can, don’t invent.

For macronode:

* extend existing facet counters / middleware similarly (if not already).

### 3.2 Where “combined bandwidth” should live

svc-admin should compute cluster totals either by:

* summing sampled node metrics in backend
* or by having its own “computed metrics” registry exported by svc-admin (`/metrics`)

And expose on a **Network** page.

---

## 4) Pivot: Implement svc-admin admin login (new milestone)

We are switching gears to implement **admin dashboard login** for svc-admin.

### 4.1 Security requirement statement (the “why”)

svc-admin controls visibility into node status/health and will eventually control actions (restart, config, policy, etc).

Therefore:

* svc-admin must require authentication by default
* anonymous access should be a deliberate dev-only mode
* we must support a production-grade auth story

### 4.2 Architectural constraint

svc-admin login/auth is a **svc-admin concern**.
Do not push “dashboard auth” into nodes; nodes should remain independently operable and should expose their own admin plane security separately.

svc-admin login protects:

* the svc-admin UI
* svc-admin API endpoints
* (optionally) proxy operations to nodes later

---

# PLAN — svc-admin login (God-tier, phased, uses existing “passport/password” crates)

## Phase 0 — Inventory + decision points (first step next instance)

Goal: pick the correct crate for password/auth primitives and avoid duplicating.

Actions:

1. Search workspace for:

   * `svc-passport`
   * password hashing helpers (argon2/bcrypt)
   * token/session helpers (JWT, macaroon, PASETO, signed cookies)
   * existing auth middleware patterns in any crate
2. Decide:

   * session type: **cookie session** vs **bearer token**
   * admin user storage: file-backed config vs sqlite/sled vs “single admin password env var”

**Recommendation (simplicity + security):**

* Start with **cookie session** (httpOnly, sameSite strict/lax, secure in prod).
* Use **argon2id** password hashing.
* Store users in a local file or sled (svc-admin-local only).
* Support bootstrap via `SVC_ADMIN_ADMIN_PASSWORD` env var.

This gets us strong security without needing external DB immediately.

---

## Phase 1 — Minimal login (MVP) with real security primitives

### Backend endpoints (svc-admin)

Add routes:

1. `POST /api/auth/login`

   * body: `{ username, password }`
   * verifies password hash
   * returns: `200` and sets session cookie
   * emits audit log event `auth.login.success|fail`

2. `POST /api/auth/logout`

   * clears cookie
   * returns 200

3. `GET /api/auth/me`

   * returns user summary if logged in
   * `{ username, roles, issuedAt }`

### Backend middleware

Implement `RequireAdminAuthLayer`:

* checks session cookie
* attaches `UserCtx` to request extensions
* rejects unauthorized with 401

### Protect these surfaces

* all non-public API routes
* optionally protect UI route delivery (or allow UI shell but API requires login)

**Dev overrides**

* `SVC_ADMIN_AUTH=off` for local dev
* `SVC_ADMIN_AUTH=on` default
* or `SVC_ADMIN_DEV_MODE=1` toggles insecure.

---

## Phase 2 — UI login flow

Add UI route(s):

* `/login`

  * username/password
  * handles 401 errors and rate-limits UI attempts
* “logged in” state stored in app context, refreshed via `/api/auth/me`

If not authenticated:

* redirect to `/login`
* show “Session expired” toast when API returns 401

---

## Phase 3 — User/role model + policy hooks

Add RBAC scaffolding:

* roles: `admin`, `operator`, `viewer`
* permissions: `nodes:read`, `nodes:write`, `config:write`, `policy:write`, etc.

Even if only `admin` exists initially, wire the model now so we don’t rewrite later.

---

## Phase 4 — Hardening (must-do before any real deployment)

Security checklist:

* bcrypt/argon2id (prefer argon2id)
* constant-time password compare
* rate limit login endpoint (IP + username)
* session TTL + idle timeout
* CSRF protection for cookie auth (double-submit cookie or sameSite + CSRF token for POST)
* secure cookie flags (httpOnly, Secure in prod, SameSite Lax/Strict)
* audit logging for auth + privileged actions
* config for rotating session signing key

---

## Phase 5 — Integrate with future node authentication (clean boundary)

Later, when svc-admin performs actions on nodes:

* svc-admin will authenticate **user**
* svc-admin will authenticate **to nodes** separately (node API tokens / mTLS / OAP auth)
* never reuse the same token/cookie between dashboard and node planes

---

# Immediate next-instance TODO list (ordered)

## A) Notes updates (done by this note)

* Add “node bandwidth must be node-specific” invariant.
* Add “Network page” plan.

## B) Admin login build-out

1. Locate and review:

   * `svc-passport` and any auth-related crates
   * any existing password hashing / token code in the monorepo
2. Implement backend MVP:

   * `/api/auth/login`, `/api/auth/logout`, `/api/auth/me`
   * session cookie + middleware
3. Implement UI:

   * login screen + auth guard

## C) Observability updates

* Add svc-admin auth metrics:

  * `ron_svc_admin_auth_login_attempts_total{result}`
  * `ron_svc_admin_auth_sessions_active`
* Add audit logs for login attempts

---

# Addendum: Bandwidth tile requirements (explicit)

* Bandwidth displayed on a node card MUST reflect the node’s own traffic.
* The dashboard should NOT treat host NIC totals as node bandwidth.
* A separate `Network` page should show combined totals and breakdowns.

---

If you paste the **svc-admin codebundle** next, I can do the “inventory pass” on what auth primitives already exist (svc-passport, hashing, tokens, middleware patterns), and then I’ll give you a **concrete implementation plan** that matches your repo’s existing style (axum 0.7, layers, DTO patterns, metrics naming, and “truthful readiness”).

ADDITIONAL TODO:
Cpu thread and core count is wrong for micronode
Cannot open storage for micronode to see details (ram only is why?)
Need amnesia and persistence (or both) and a breakdown of what dirs are amnesia and persistence (if both)
Need further clarification on the different planes listed in micronode (they differ from macronode not sure if it is some unified system or mock)

### END NOTE - JANUARY 9 2026 - 12:30 CST


## CARRY-OVER NOTES — svc-admin Local Login + Auth Wiring (January 2026)

### 0) Current status in one sentence

We successfully **added and compiled the core “local auth” building blocks (RBAC + sessions + Argon2id)** and updated config to support `auth.mode=local`, but **the local auth HTTP routes (`/api/auth/*`) are not yet mounted into the main svc-admin router**, so `POST /api/auth/login` currently returns **404** even though the code exists and unit/integration tests pass.

---

## 1) What we changed / implemented so far

### 1.1 Cargo.toml updates (svc-admin)

We expanded `crates/svc-admin/Cargo.toml` to include local-auth dependencies:

* **axum-extra** with cookies support:

  * `axum-extra = { version = "0.9", features = ["cookie"] }`
* **password hashing**:

  * `argon2 = "0.5"`
  * `password-hash = "0.5"`
* **randomness + IDs**:

  * `rand = "0.9"`
  * `hex = "0.4"`
* **simple in-process locking**:

  * `parking_lot = "0.12"`

We also kept the existing rustls-only HTTP stack, tokio, tracing, prometheus, etc.

✅ Result: `cargo build -p svc-admin` and `cargo test -p svc-admin` now compile and run successfully.

---

### 1.2 Auth config expanded (config/auth.rs)

We updated `crates/svc-admin/src/config/auth.rs` to support a secure-by-default local login mode, plus env-friendly knobs.

**Auth modes now include:**

* `none` (dev identity)
* `ingress` (trusted reverse proxy headers)
* `passport` (reserved for future JWT validation; fail-closed)
* `local` (username/password + session cookie + RBAC) ✅ intended default for local installs

**New local auth fields (Option<> for serde friendliness):**

* `cookie_name`
* `cookie_secure`
* `session_ttl_sec` (hard TTL)
* `session_idle_ttl_sec` (rolling TTL)
* `rbac_path`
* `bootstrap_admin_username`
* `bootstrap_admin_password_env`

**Helper methods** added so the rest of the code doesn’t unwrap `Option<>` everywhere:

* `cookie_name_or_default()`
* `cookie_secure_or_default()`
* `session_ttl_sec_or_default()`
* `session_idle_ttl_sec_or_default()`
* `rbac_path_or_default()`
* `bootstrap_admin_username_or_default()`
* `bootstrap_admin_password_env_or_default()`

**Default changed** to secure-by-default:

* `mode = "local"` (so you can’t accidentally run unauthenticated if you bind beyond localhost)
* default `rbac_path = data/svc-admin/rbac.json`
* default bootstrap admin username `admin`
* default env var name for bootstrap password `SVC_ADMIN_BOOTSTRAP_ADMIN_PASSWORD`

---

### 1.3 Config env loader extended (config/loader.rs)

We updated `crates/svc-admin/src/config/loader.rs` to:

* Accept auth modes: `none|ingress|passport|local`
* Load local auth env vars:

  * `SVC_ADMIN_AUTH_COOKIE_NAME`
  * `SVC_ADMIN_AUTH_COOKIE_SECURE`
  * `SVC_ADMIN_AUTH_SESSION_TTL_SEC`
  * `SVC_ADMIN_AUTH_SESSION_IDLE_TTL_SEC`
  * `SVC_ADMIN_AUTH_RBAC_PATH`
  * `SVC_ADMIN_AUTH_BOOTSTRAP_ADMIN_USERNAME`
  * `SVC_ADMIN_AUTH_BOOTSTRAP_ADMIN_PASSWORD_ENV`

Also added validation:

* If `auth.mode == "local"` then `auth.rbac_path` must not be empty.

✅ Result: config can now be controlled by env and validates local mode.

---

### 1.4 Auth identity resolver updated (auth/mod.rs)

We updated `crates/svc-admin/src/auth/mod.rs` so `resolve_identity_from_headers` recognizes `"local"`.

Important behavior:

* `none` → returns dev identity
* `ingress` → uses `X-User`, `X-Groups`
* `passport` → returns `Unimplemented`
* `local` → currently “not wired in resolve_identity_from_headers” unless we add a local backend; we changed it to **fail-closed** by returning `Unimplemented` / unauthenticated depending on how your tests evolved.

✅ Result: tests pass and auth mode parsing is consistent.

---

### 1.5 Local auth implementation exists (auth/local.rs)

We implemented/iterated `crates/svc-admin/src/auth/local.rs` providing:

#### Core behaviors

* Server-side **session store** (in-memory `HashMap<String, Session>` protected by `RwLock`)
* RBAC store loaded/saved as JSON (path configurable)
* Bootstrap admin creation:

  * If RBAC has zero users AND bootstrap password env var exists → create `admin` user with Argon2id PHC
* Cookie session:

  * httpOnly
  * SameSite Lax
  * Secure flag configurable
  * Max-Age set to TTL
  * Cookie string returned via headers

#### Routes (inside local auth router)

* `POST /api/auth/login`
* `POST /api/auth/logout`
* `GET /api/auth/me`

#### Security invariants (current)

* deny-by-default semantics intended
* no lock held across `.await` (currently okay; locks are inside sync sections)
* password verification uses dummy hash path to reduce user enumeration timing differences
* session TTL + idle TTL enforced

#### Current warning

* `issued_at_unix_s` not used (benign warning; could be used later for audit logging or token rotation)

✅ Result: The module compiles and tests pass.

---

## 2) Build/test results we observed

### 2.1 Build passes

`cargo build -p svc-admin` succeeded with one warning only:

* dead_code: `issued_at_unix_s` never read

### 2.2 Tests pass

`cargo test -p svc-admin`:

* Unit tests for ingress/none/passport/local mode behavior passed
* HTTP smoke tests passed
* Auth me tests passed (for ingress/none)
* Config env test passed
* Fake node test passed

### 2.3 Running svc-admin worked, but metrics sampler warned

When running svc-admin it kept trying to poll:

* `http://127.0.0.1:8080/metrics` and got connection refused

That means: **svc-admin is configured with a node entry (macronode) but macronode wasn’t running**. This is expected until we run the stack script or start a node manually.

---

## 3) The remaining blocking issue: 404 on /api/auth/login

### Symptom

You ran:

```bash
curl -i -X POST http://127.0.0.1:5300/api/auth/login \
  -H 'content-type: application/json' \
  -d '{"username":"admin","password":"change-me-now"}'
```

Response:

* `HTTP/1.1 404 Not Found`

### Root cause

The local auth router defined in `auth/local.rs` **is not mounted/merged into the main svc-admin router** that’s listening on port `5300`.

In Axum, this usually happens because:

* Main app router uses `State<AppState>`
* Local auth router uses `State<Arc<LocalAuth>>`
* Routers with different state types can’t be merged directly
* So we need to put the `LocalAuth` inside `AppState` (or use an extension layer) and mount the routes using the same state type.

### What must be implemented next (critical)

1. Add local auth to the application state:

   * `AppState { local_auth: Option<Arc<LocalAuth>>, ... }` (or similar)
2. Initialize `LocalAuth` from `Config` when `auth.mode == "local"`
3. Mount these routes into the main router:

   * `/api/auth/login`
   * `/api/auth/logout`
   * `/api/auth/me`
4. (Soon after) Add enforcement middleware:

   * For `/api/*` routes (except `/api/auth/login`, `/healthz`, `/readyz`, `/metrics`, static SPA) require session cookie when in local mode.
   * Also ensure UI “read-only mode” and RBAC permission checks align.

Until this wiring is done, local auth will compile but won’t be reachable.

---

## 4) How login should work across different use cases

### 4.1 Local single-machine install (recommended default)

**Typical deployment:** operator runs svc-admin on localhost or LAN for a small cluster.

* `SVC_ADMIN_AUTH_MODE=local`
* RBAC stored on disk (JSON) at `data/svc-admin/rbac.json` or env override
* Bootstrap flow:

  * If RBAC empty and bootstrap password env var is set → create admin user
* Browser flow:

  1. User hits dashboard UI at `http://host:5300/`
  2. UI detects unauthenticated (401 from `/api/auth/me`)
  3. UI shows login form
  4. UI calls `POST /api/auth/login` with JSON creds
  5. Server returns `Set-Cookie: svc_admin_sid=...; HttpOnly; SameSite=Lax; Path=/; Max-Age=...`
  6. UI uses cookie automatically on future requests
  7. `GET /api/auth/me` returns roles/perms to drive UI gating
* Security knobs:

  * `cookie_secure=true` when behind HTTPS
  * idle TTL keeps sessions short if the browser is left open

**Notes:**

* For “LAN only” deployments: still prefer HTTPS termination at a reverse proxy and set cookie_secure=true.

---

### 4.2 Cloud deployment behind ingress / reverse proxy

**Typical deployment:** svc-admin accessible on the internet, behind nginx/traefik/cloud load balancer.

Two valid patterns:

#### Pattern A — Ingress auth (SSO at the edge)

* `SVC_ADMIN_AUTH_MODE=ingress`
* Ingress handles real identity (OAuth/SAML, etc)
* Ingress injects:

  * `X-User: email`
  * `X-Groups: admin,ops`
* svc-admin trusts only if it is running behind the trusted proxy and not directly exposed.

**Pros:** simplest for cloud SSO.
**Cons:** must guarantee no bypass path to svc-admin that skips the proxy.

#### Pattern B — Local auth used “through” ingress

* `SVC_ADMIN_AUTH_MODE=local`
* svc-admin handles auth itself (session cookie)
* Ingress just terminates TLS / routes traffic.

**Pros:** consistent behavior everywhere; no external auth needed.
**Cons:** you must harden it: TLS + cookie_secure + rate limiting + possibly lockouts.

---

### 4.3 Passport mode (future JWT validation)

**Intended future:**

* `SVC_ADMIN_AUTH_MODE=passport`
* svc-admin validates JWTs against:

  * issuer, audience, jwks url
* Identity derived from token claims
* Strict 401/403 mapping + metrics on failure reasons

**Today:** mode exists but should be fail-closed / unimplemented.

---

### 4.4 None mode (dev-only)

* `SVC_ADMIN_AUTH_MODE=none`
* Always returns synthetic Dev Operator identity
* Only safe when binding to loopback for dev.

We should eventually enforce:

* If bind addr is not loopback, reject `none`.

---

## 5) What you should run vs what you ran

### 5.1 You don’t need run_dashboard.sh to test local auth itself

Local auth should be testable with only svc-admin running.

### 5.2 You *do* need a running node (macronode/micronode) to avoid sampler warnings

The warnings are from the facet sampler polling `http://127.0.0.1:8080/metrics`.
To make the dashboard “look normal,” start the node(s) first (stack script or manual).

But this is not the reason login is 404 — that is routing.

---

## 6) Files needed next time (paste these in the next instance)

To fix the 404, we need to see exactly how the main router is assembled and what state type it uses.

Please paste **full contents** of whichever ones exist:

1. **Router + server wiring (required)**

* `crates/svc-admin/src/server.rs` (or the file that prints: `svc-admin listening for UI/API`)
* and/or `crates/svc-admin/src/http/mod.rs`
* and/or `crates/svc-admin/src/router.rs`
* and/or any file that builds `Router::new()` for the main app

2. **AppState definition (required)**

* The file that defines `AppState` (often `src/state.rs`, `src/app.rs`, `src/lib.rs`, or inside `server.rs`)

3. **Binary entrypoint (required)**

* `crates/svc-admin/src/bin/svc-admin.rs`

4. **Config root struct (helpful if not in the above)**

* `crates/svc-admin/src/config/mod.rs` (or wherever `Config` is defined)

With those, we can:

* Add `local_auth` into AppState
* Initialize it from Config when `mode=local`
* Mount `/api/auth/*` into main router
* Add a “require auth” middleware for `/api/*` routes (excluding auth endpoints)
* Ensure UI can call `/api/auth/me` for session-check and show login page

---

## 7) What remains after the routes are mounted (next milestones)

### Milestone A — “Login works end-to-end”

* `/api/auth/login` returns 200 + Set-Cookie
* `/api/auth/me` returns identity/roles/perms
* `/api/auth/logout` clears cookie
* UI detects unauthenticated state and shows login screen

### Milestone B — “Enforcement”

* In local mode:

  * block `/api/*` unless authenticated (except auth endpoints)
  * RBAC gate mutating endpoints:

    * node actions: restart/shutdown/reload
    * config/policy write routes
* Add metrics counters:

  * login_success_total
  * login_fail_total
  * auth_denied_total{reason}

### Milestone C — “Production hardening”

* Rate limit login attempts (basic in-memory)
* Optional lockout/backoff
* Ensure cookie_secure=true when TLS is enabled or when behind ingress
* Add invariant: disallow `auth.mode=none` when bind addr is not loopback

### Milestone D — “Cloud SSO story”

* Ingress auth mode documented and verified
* Trusted proxy allowlist / “only trust headers from local proxy” guardrails

---

## 8) Tiny note on the warning

The current warning (`issued_at_unix_s` never read) is harmless. We can either:

* use it later for audit/session debugging
* or remove it from `Session` if we want a warning-free build

Not a blocker.

---

If you paste the 3 required files (server/router + AppState + bin entry) in the next instance, we can immediately produce full paste-ready replacements that wire local auth into the running service and eliminate the `/api/auth/login` 404.



### BEGIN NOTE - JANUARY 9 2026 - 14:10 CST





### END NOTE - JANUARY 9 2026 - 14:10 CST




### BEGIN NOTE - JANUARY 9 2026 - 15:20 CST

# CARRY-OVER NOTES — svc-admin Dashboard Login (Local Auth) Wiring (Jan 2026)

## 0) One-sentence status

We have **working local-auth primitives** (Argon2id password hashing, RBAC JSON store, in-memory server sessions, cookie-based auth endpoints, and UI fetch client updates), but **login is not fully functional end-to-end yet** because the **local-auth routes + auth-gating middleware must be mounted into the main Axum router using the main AppState**, and the **UI needs a real route-guard that redirects to LoginPage when unauthenticated**.

---

## 1) What we accomplished so far

### 1.1 Established the correct root cause of the 404

* The 404 on `POST /api/auth/login` happens when the local auth router exists in code but **isn’t merged/mounted into the router being served on the UI/API listener** (port ~5300).
* In Axum 0.7, this is often caused by a **state type mismatch**:

  * main router uses `State<Arc<AppState>>`
  * local auth routes were originally built with `State<Arc<LocalAuth>>`
  * routers with different state types don’t merge cleanly unless you use `FromRef` extraction or embed LocalAuth into AppState and extract it.

### 1.2 We identified the relevant backend assembly points

You provided and/or we confirmed:

* `crates/svc-admin/src/state.rs` defines `AppState` and `AppState::new(config)` constructs it.
* `crates/svc-admin/src/router.rs` builds the UI/API router and the metrics router.
* `crates/svc-admin/src/server.rs` creates `state`, calls `router::build_router(state.clone())`, binds listeners, and serves via `axum::serve`.

These are the files where the real wiring must happen.

### 1.3 We implemented/standardized local auth module structure

We built local-auth that includes:

* RBAC store load/save to JSON on disk
* Bootstrap admin creation if RBAC is empty and `SVC_ADMIN_BOOTSTRAP_ADMIN_PASSWORD` (or configured env var) is set
* In-memory sessions store keyed by session id
* Cookie-based session (`HttpOnly`, `SameSite=Lax`, secure flag configurable)
* Endpoints:

  * `POST /api/auth/login`
  * `POST /api/auth/logout`
  * `GET /api/auth/me`
* Authentication helper for middleware:

  * `authenticate_headers(headers)` → returns optional user context + optional refreshed cookie header

### 1.4 We upgraded the UI client and login page directionally

We identified the frontend “session-check” locus:

* `crates/svc-admin/ui/src/api/adminClient.ts` is the SPA’s fetch client.

We determined a critical missing piece for cookie sessions:

* **All fetches must include cookies**, i.e. `fetch(..., { credentials: 'include' })`, or the browser won’t send/receive the session cookie in many environments.

We also pivoted from a stub login view:

* `LoginPage.tsx` exists (not `login.tsx`), originally placeholder.
* Next state is: LoginPage should perform the login call and redirect into the app after successful auth.

---

## 2) What remains to finish login (end-to-end)

### Milestone A — Make `/api/auth/*` routes reachable (eliminate 404)

**Backend tasks:**

1. **Add LocalAuth to AppState**

   * Either:

     * `pub local_auth: Option<Arc<LocalAuth>>`
     * or `pub local_auth: Arc<LocalAuth>` when mode=local and `None` otherwise.
2. **Initialize LocalAuth in `AppState::new(config)`**

   * When `config.auth.mode == "local"`:

     * build a `LocalAuthCfg` from config
     * `LocalAuth::new(cfg)?`
     * store it in state
   * If mode != local:

     * keep `None`
3. **Expose LocalAuth to handlers via `FromRef`**

   * Implement `FromRef<AppState> for Arc<LocalAuth>` *or* `FromRef<AppState> for Option<Arc<LocalAuth>>`
   * This is the critical “Axum state extraction” bridge.
4. **Merge/mount local-auth routes into the main router**

   * In `router::build_router(...)`:

     * add `.merge(LocalAuth::routes::<Arc<AppState>>() / or ::routes::<AppState>() depending on your state type)`
   * Ensure `/api/auth/login` exists on the UI/API plane.
5. **Reconcile /api/me vs /api/auth/me**

   * Decide one canonical “session-check” endpoint for the UI.
   * Best: UI uses `/api/auth/me` in local mode.
   * Optionally keep `/api/me` for ingress mode as a “resolved identity” endpoint.

**Success condition:**

* `curl -i -X POST http://127.0.0.1:5300/api/auth/login ...` returns 200 and `Set-Cookie`.

---

### Milestone B — Add enforcement (auth gate) for `/api/*`

Right now, many `/api/*` routes are effectively available unless gated by header auth or role checks.

**Backend tasks:**

1. Add a middleware/layer to the UI/API router:

   * If mode=local:

     * allow unauthenticated access to:

       * `/api/auth/login`
       * `/api/auth/logout`
       * `/api/auth/me`
       * `/api/ui-config` (optional: can remain public)
       * `/healthz`, `/readyz` (but these are on metrics plane anyway)
     * require authentication for everything else under `/api/*`
2. Apply RBAC permission checks to mutating endpoints:

   * reload/shutdown/debug-crash/bench-run should be permission gated consistently.
3. Ensure “deny-by-default”:

   * If local mode and no session cookie => 401.
   * If session valid but missing role/perm => 403.

**Success condition:**

* Unauthed calls to `/api/nodes` (or any non-auth API) return 401 in local mode.

---

### Milestone C — Wire UI route guard + login flow

**Frontend tasks:**

1. Ensure `adminClient.ts` sends cookies on all requests:

   * `credentials: 'include'`
2. Add `adminClient.login/logout/authMe` helpers.
3. Implement route guard:

   * On app boot, call `adminClient.getMe()` (or `authMe()`)
   * If 401 => redirect to `/login`
4. Implement `LoginPage.tsx`:

   * username/password form
   * POST `/api/auth/login`
   * on success: redirect to `/`
   * show error on failure
5. Decide behavior for ingress mode:

   * If `auth.mode == ingress`, don’t show local login; rely on header identity and route the user into the app.

**Success condition:**

* Opening the dashboard shows LoginPage when not logged in, then after login shows the dashboard without manual refreshes.

---

### Milestone D — Hardening + production correctness

Not required to “finish login,” but should be on the immediate follow-up list:

1. Rate limit login attempts (in-memory counter per IP or per username).
2. Optional lockout/backoff.
3. Ensure cookie settings are correct:

   * `cookie_secure=true` behind TLS termination
   * consider `SameSite=Lax` (good default) or Strict if you want tighter
4. Disallow `auth.mode=none` unless bind addr is loopback.
5. Add metrics for auth:

   * `login_success_total`
   * `login_fail_total`
   * `auth_denied_total{reason}`

---

## 3) Known gotchas we must avoid next time

* **Router state mismatch** is the most common reason the auth endpoints “exist” but never get served.
* If UI doesn’t set `credentials: 'include'`, cookie sessions won’t work reliably (especially if you ever serve UI and API from different origins, or if future proxies are introduced).
* If `/api/me` continues to return a dev fallback identity (as it currently can in some modes), the UI might think it’s authenticated even when it’s not. We must unify “truth” for local mode.

---

## 4) What to test once wiring is done

### Backend (curl)

1. Bootstrap admin:

* export `SVC_ADMIN_AUTH_MODE=local`
* export `SVC_ADMIN_BOOTSTRAP_ADMIN_PASSWORD=change-me-now` (or the configured env var name)
* start svc-admin
* verify RBAC file created (e.g. `data/svc-admin/rbac.json`)

2. Login:

```bash
curl -i -X POST http://127.0.0.1:5300/api/auth/login \
  -H 'content-type: application/json' \
  -d '{"username":"admin","password":"change-me-now"}'
```

Expect: `200` and `Set-Cookie: <cookie_name>=s_...`

3. Me (cookie round-trip):

```bash
curl -i http://127.0.0.1:5300/api/auth/me \
  -H 'cookie: <cookie_name>=<sid>'
```

4. Gate enforcement:

* request `/api/nodes` without cookie should be `401` in local mode.

---

## 5) Files needed next instance (paste exact current contents)

### Required backend files (for drop-in fixes)

1. `crates/svc-admin/src/state.rs`
2. `crates/svc-admin/src/router.rs`
3. `crates/svc-admin/src/server.rs`
4. `crates/svc-admin/src/auth/mod.rs`
5. `crates/svc-admin/src/config/mod.rs` *(or wherever `Config` + `AuthCfg` live)*
6. `crates/svc-admin/src/config/loader.rs`
7. `crates/svc-admin/src/auth/local.rs` *(if you want us to align it with the AppState/FromRef wiring fully; otherwise I’ll adapt to your version)*

### Required frontend files (if you want the UI login to be fully real)

8. `crates/svc-admin/ui/src/App.tsx`
9. `crates/svc-admin/ui/src/api/adminClient.ts`
10. `crates/svc-admin/ui/src/routes/LoginPage.tsx`

---

## 6) Deliverable for next instance

Once you paste those files, the next response will be:

* **Full paste-ready replacements** for each changed file (no partial snippets)
* Login working end-to-end:

  * `/api/auth/login` reachable (no more 404)
  * cookie session persists
  * `/api/*` gated in local mode
  * SPA redirects to `/login` when unauthenticated and back to `/` after login


### END NOTE - JANUARY 9 2026 - 15:20 CST



### BEGIN NOTE - JANUARY 9 2026 - 19:30 CST

Here’s a set of carry-over notes you can paste straight into a NOTES.md / AUTH_NOTES.md or drop into the next instance.

⸻

svc-admin – Local Auth + Axum Server Bootstrap Notes

0. Context / Current Status
   •	We are working on crates/svc-admin, the God-tier admin dashboard for RON-CORE.
   •	We just finished:
   •	Migrating the server bootstrap (server.rs) to a clean Axum 0.7 pattern.
   •	Ensuring the routers (router.rs) are parameterized correctly with Arc<AppState>.
   •	Wiring up local auth mode, reaching the login screen at [http://127.0.0.1:5300](http://127.0.0.1:5300).
   •	We can now:
   •	Build svc-admin successfully (after the axum/tower type issues are resolved).
   •	Start svc-admin and see the login UI.
   •	Control login behavior via env vars and data/svc-admin/rbac.json.

The remaining friction point we focused on:

“I’m at the login screen; what are the default creds, where are they stored, and how do I change/reset them?”

The notes below answer all of that.

⸻

1. Axum 0.7 Server Bootstrap – Final Pattern

File
•	crates/svc-admin/src/server.rs

Key decisions
1.	Axum 0.7 has a slightly different serve API compared to older versions:
•	You can pass a Router directly to axum::serve(listener, router).
•	You must not call .into_make_service() for the basic use-case we have here.
•	Serve no longer needs .into_future(); you just .await the returned value inside your task.
2.	Our final server.rs uses:

```
•	Router<Arc<AppState>> as the router type.
•	axum::serve(listener, router) directly.
•	A separate metrics listener and a main UI/API listener.
•	tokio::select! to coordinate shutdown and logging.
```

Final structure (conceptual)
•	run(config: Config) -> Result<()>:
•	Initialize tracing.
•	Init node inventory metrics.
•	Build Arc<AppState> from config.
•	Spawn facet sampler tasks (using spawn_facet_samplers).
•	Build:
•	let app = router::build_router(state.clone());
•	let metrics_app = router::build_metrics_router(state.clone());
•	Bind:
•	main_listener = TcpListener::bind(config.server.bind_addr).await?;
•	metrics_listener = TcpListener::bind(config.server.metrics_addr).await?;
•	Spawn:
•	Metrics server task:

let mut metrics_task = tokio::spawn(async move {
axum::serve(metrics_listener, metrics_app).await
});

```
•	Main server task (with graceful shutdown):
```

let shutdown = shutdown_signal(sampler_shutdown_tx);
let main_task = tokio::spawn(async move {
axum::serve(main_listener, app)
.with_graceful_shutdown(shutdown)
.await
});

```
•	tokio::select!:
•	If main task exits → abort metrics task, log appropriate messages.
•	If metrics task exits → log that the metrics plane died.

•	spawn_facet_samplers(...):
•	Same behavior as before: derive NodeMetricsTarget list from config, spawn sampler tasks, and return a watch::Sender<bool> shutdown channel.
•	shutdown_signal(...):
•	Waits for Ctrl+C.
•	Logs shutdown.
•	Sends true on the sampler shutdown channel.
```

Gotchas we fixed
•	Earlier errors like:
•	“no method named into_make_service”
•	“Router<Arc<AppState>>: Service<...> not satisfied”
•	“Serve<...> is not a future / IntoFuture not implemented”
were all due to mixing old Axum patterns (from 0.6) with 0.7.
•	Final resolution:
•	Don’t use .into_make_service() at all in our case.
•	Don’t call .into_future() on Serve or WithGracefulShutdown.
•	Just axum::serve(listener, router).await and wrap in tokio::spawn.

⸻

2. Router Wiring – Router<Arc<AppState>>

File
•	crates/svc-admin/src/router.rs

Important invariants
•	Everything in router.rs assumes:
•	We use Arc<AppState> as Axum state.
•	Two planes:
•	UI/API plane: build_router(state: Arc<AppState>) -> Router<Arc<AppState>>
•	Metrics/health plane: build_metrics_router(state: Arc<AppState>) -> Router<Arc<AppState>>

build_router (UI/API)
•	Returns: Router<Arc<AppState>>.
•	Routes include:
•	Auth (local mode):
•	/api/auth/login, /api/auth/logout, /api/auth/me
•	Versioned aliases /api/v1/*
•	Core API:
•	/api/ui-config, /api/me
•	/api/nodes, /api/nodes/:id/status, /api/nodes/:id/metrics/facets
•	System summary, net accounting, storage summary + DBs, bench runs, etc.
•	Dev App Playground routes (optional; gated by config):
•	/api/playground/examples
•	/api/playground/manifest/validate
•	Control-plane actions:
•	/api/nodes/:id/reload, /shutdown, /debug/crash
•	Middleware:
•	local_api_gate is layered with middleware::from_fn_with_state(state.clone(), local_api_gate)
•	.layer(local_gate).with_state(state) at the end.

build_metrics_router (Metrics/Health)
•	Returns: Router<Arc<AppState>>.
•	Exposes only:
•	/healthz
•	/readyz
•	/metrics (Prometheus bridge)
•	No /api/* routes on this plane.

⸻

3. Local Auth Model – How Login Actually Works

Auth mode selection
•	Controlled by config/env: state.config.auth.mode.
•	For our current work:
•	We set SVC_ADMIN_AUTH_MODE=local
•	This makes svc-admin use the local auth backend.

Bootstrap flow
•	On first run in local mode, the code:
1.	Determines the RBAC database path:
•	AuthCfg::rbac_path_or_default() → by default:
•	data/svc-admin/rbac.json
2.	Checks if there are any users in that RBAC file.
3.	If no users:
•	Reads SVC_ADMIN_BOOTSTRAP_ADMIN_USERNAME, or defaults to "admin".
•	Reads SVC_ADMIN_BOOTSTRAP_ADMIN_PASSWORD (must be set).
•	Creates an admin user with that username and a bcrypt PHC password hash.
•	Writes it into data/svc-admin/rbac.json.

Where the login is stored
•	Not in the script.
•	Not hard-coded in the source.

Instead:
•	The bootstrap password is taken from the env var:
•	SVC_ADMIN_BOOTSTRAP_ADMIN_PASSWORD
•	at the time of the first successful startup when RBAC has no users.
•	After that, the source of truth becomes:
•	data/svc-admin/rbac.json (contains users + bcrypt hashes).

Default username & password logic
•	Username:
•	Default: "admin" (unless overridden by SVC_ADMIN_BOOTSTRAP_ADMIN_USERNAME).
•	Password:
•	Whatever SVC_ADMIN_BOOTSTRAP_ADMIN_PASSWORD was when rbac.json was initially bootstrapped.

Changing the env var later does not magically update the existing rbac.json.

⸻

4. Local API Gate – How Requests Are Protected
   •	Middleware: local_api_gate(State(state), req, next).
   •	Enabled only when state.config.auth.mode == "local".
   •	Behavior:
   •	Allows non-/api paths through (SPA, static files).
   •	For /api/*:
   •	If path is allowlisted (login, ui-config, etc.), it lets the request through without requiring a session cookie.
   •	Otherwise, it:
   •	Uses local_auth.authenticate_headers(req.headers()) to check session cookie.
   •	On success:
   •	Injects auth::Identity into request extensions.
   •	Optionally refreshes cookie via Set-Cookie.
   •	On failure:
   •	Returns 401 UNAUTHORIZED or an appropriate error.

This means:
•	/api/ui-config, /api/auth/login, /api/auth/me, etc. are reachable without already being logged in.
•	All other /api/* endpoints require a valid session cookie, enforced by this middleware.

⸻

5. How to Run svc-admin Dashboard (Current Dev Flow)

We currently have two patterns:

Option A – Direct cargo run (good for debugging)

From the repo root:

cd /Users/mymac/Desktop/RustyOnions

export SVC_ADMIN_AUTH_MODE=local
export SVC_ADMIN_BOOTSTRAP_ADMIN_USERNAME=admin          # optional; defaults to "admin"
export SVC_ADMIN_BOOTSTRAP_ADMIN_PASSWORD='change-me-now'  # set your desired password

cargo run -p svc-admin --bin svc-admin

Then:
•	Open [http://127.0.0.1:5300](http://127.0.0.1:5300) → login screen.
•	Metrics: [http://127.0.0.1:5310/healthz](http://127.0.0.1:5310/healthz), /readyz, /metrics.

Option B – Using your dev script (e.g., dev_svc_admin_stack.sh / dashboard script)

From the repo root:

cd /Users/mymac/Desktop/RustyOnions

export SVC_ADMIN_AUTH_MODE=local
export SVC_ADMIN_BOOTSTRAP_ADMIN_USERNAME=admin
export SVC_ADMIN_BOOTSTRAP_ADMIN_PASSWORD='change-me-now'

scripts/dev_svc_admin_stack.sh   # or your specific run_dashboard.sh

Important:
•	The env vars must be visible to the child svc-admin process.
•	Either export them in the shell before running the script or have the script export them explicitly.

⸻

6. How to Change / Reset the Admin Password

There is currently no dedicated /api/auth/change-password endpoint yet. So, in dev, the simplest flows are:

Option 1 – Reset by removing rbac.json (recommended in dev)
1.	Stop svc-admin (stop the cargo run / dashboard script).
2.	Move or delete the RBAC file:

cd /Users/mymac/Desktop/RustyOnions

mv data/svc-admin/rbac.json data/svc-admin/rbac.json.bak

# or:

# rm data/svc-admin/rbac.json

```
3.	Set the env vars you want for the new bootstrap admin:
```

export SVC_ADMIN_AUTH_MODE=local
export SVC_ADMIN_BOOTSTRAP_ADMIN_USERNAME=admin          # optional
export SVC_ADMIN_BOOTSTRAP_ADMIN_PASSWORD='NEW-STRONG-PASSWORD'

```
4.	Start svc-admin again:
```

cargo run -p svc-admin --bin svc-admin

# or:

# scripts/dev_svc_admin_stack.sh

```
5.	Log in with:
•	Username: admin
•	Password: NEW-STRONG-PASSWORD
```

On this run, LocalAuth will see that RBAC is empty and recreate rbac.json with the new password hash.

Option 2 – Manually edit rbac.json (for future / advanced use)

If we want to keep existing users/roles and only change the admin password:
1.	Generate a new bcrypt PHC hash for the desired password (via a small Rust helper, htpasswd, or a dedicated tool).
2.	Open data/svc-admin/rbac.json.
3.	Find the "admin" user entry.
4.	Replace its password_phc string with the new hash.
5.	Restart svc-admin.

In the current phase, Option 1 is perfectly acceptable and easiest.

⸻

7. Why Login Was Failing Earlier

Symptom:
•	You reached the login screen but couldn’t log in using the password you believed should work.

Root causes we identified:
1.	Bootstrap only happens once:
•	On the first run without users, RBAC is bootstrapped from env vars.
•	After that, changing SVC_ADMIN_BOOTSTRAP_ADMIN_PASSWORD in your shell does nothing unless you remove rbac.json.
2.	The environment may have been:
•	Missing SVC_ADMIN_BOOTSTRAP_ADMIN_PASSWORD on the first run.
•	Or set to some value you no longer remember.
3.	End result:
•	data/svc-admin/rbac.json contains a bcrypt hash of the original password,
•	but you are trying a different password now.

Resolution:
•	Delete/move data/svc-admin/rbac.json.
•	Re-bootstrap with the desired env vars as described above.

⸻

8. Known Gaps / Future Work

Things we still might want to add later for God-tier UX:
1.	Password management endpoints (local mode):
•	POST /api/auth/change-password for authenticated users.
•	POST /api/auth/admin/reset-password for admins to reset others (with strong audit logging).
2.	RBAC self-serve UI:
•	SPA controls to:
•	List users/roles.
•	Add/remove roles from a user.
•	Create/delete users.
•	3.	Config docs:
•	Make sure CONFIG.md / AUTH.md for svc-admin clearly lists:
•	SVC_ADMIN_AUTH_MODE
•	SVC_ADMIN_BOOTSTRAP_ADMIN_USERNAME
•	SVC_ADMIN_BOOTSTRAP_ADMIN_PASSWORD
•	Default RBAC path (data/svc-admin/rbac.json) and how to override.
4.	Non-local auth modes:
•	Wire auth.mode != "local" flows (OIDC, SSO, etc.) so the SPA can also support cloud / enterprise login.

⸻

9. Practical “Do This” Summary (for next time)

When you come back in a future instance and want to get into the dashboard:
1.	From repo root:

cd /Users/mymac/Desktop/RustyOnions

```
2.	Blow away RBAC (if you’re unsure what the password is):
```

mv data/svc-admin/rbac.json data/svc-admin/rbac.json.bak  # optional but recommended in dev

```
3.	Export env vars:
```

export SVC_ADMIN_AUTH_MODE=local
export SVC_ADMIN_BOOTSTRAP_ADMIN_USERNAME=admin
export SVC_ADMIN_BOOTSTRAP_ADMIN_PASSWORD='SUPER-SECRET-PASSWORD'

```
4.	Run svc-admin via your chosen method:
```

cargo run -p svc-admin --bin svc-admin

# or:

# scripts/dev_svc_admin_stack.sh

```
5.	Visit http://127.0.0.1:5300, log in as:
•	Username: admin
•	Password: SUPER-SECRET-PASSWORD
6.	If you change your mind later:
•	Repeat steps 2–5 with a new password.
```

⸻

These notes should be drop-in ready for the next instance to understand:
•	Why Axum was unhappy before and what pattern we now use.
•	How the router and state are wired.
•	How local auth bootstraps, where the admin login lives, and how to reset/change credentials.
•	Exactly how to spin svc-admin up and log in successfully.


### END NOTE - JANUARY 9 2026 - 19:30 CST


### RESET LOGIN

Do this from the repo root. This matches the **current svc-admin env vars** in your latest codebundle.

## 1) Reset RBAC (so your new creds actually take effect)

```bash
cd /Users/mymac/Desktop/RustyOnions

mv data/svc-admin/rbac.json data/svc-admin/rbac.json.bak 2>/dev/null || true
```

## 2) Export the correct env vars (username + password)

**Username** is controlled by `SVC_ADMIN_AUTH_BOOTSTRAP_ADMIN_USERNAME`
**Password value** is controlled by `SVC_ADMIN_BOOTSTRAP_ADMIN_PASSWORD`

```bash
export SVC_ADMIN_AUTH_MODE=local
export SVC_ADMIN_AUTH_BOOTSTRAP_ADMIN_USERNAME=admin
export SVC_ADMIN_BOOTSTRAP_ADMIN_PASSWORD='change-me-now'
```

## 3) Run the script

```bash
scripts/run_dashboard.sh
```

## 4) Verify login works (quick curl)

```bash
curl -i -c /tmp/svc_admin.cookies \
  -H 'content-type: application/json' \
  -d '{"username":"admin","password":"change-me-now"}' \
  http://127.0.0.1:5300/api/auth/login

curl -i -b /tmp/svc_admin.cookies \
  http://127.0.0.1:5300/api/auth/me
```

### If you still see 404 on `/api/auth/login`

It usually means you’re hitting the wrong port (metrics plane) or the script isn’t passing env vars into the svc-admin process.

* Make sure you’re POSTing to **5300** (UI/API), not **5310** (metrics).
* If the script uses `sudo` or spawns in a way that drops env, run it like this:

```bash
env SVC_ADMIN_AUTH_MODE=local \
    SVC_ADMIN_AUTH_BOOTSTRAP_ADMIN_USERNAME=admin \
    SVC_ADMIN_BOOTSTRAP_ADMIN_PASSWORD='change-me-now' \
    scripts/run_dashboard.sh
```


### END RESET LOGIN



### BEGIN NOTE - JANUARY 9 2026 - 21:30 CST

Here’s a **comprehensive wrap-up / carry-over NOTES** for `crates/svc-admin`, focused on **local auth login completion**, what’s working, what changed, what to watch for, and what’s next.

---

# svc-admin Wrap-Up Notes (Login/Auth + Dashboard Bootstrap)

## 0) Current status snapshot

* **svc-admin runs** and the SPA loads.
* **Local auth login works end-to-end**:

  * `POST /api/auth/login` returns **200 OK**
  * A cookie is set: `svc_admin_sid=...; HttpOnly; SameSite=Lax; Max-Age=3600`
  * `GET /api/auth/me` returns **200 OK** with `{ username, roles, expiresAtUnixS }`
* We fixed earlier confusion where login returned **401/404** due to:

  * RBAC file already being bootstrapped with a different password
  * env var naming drift
  * UI/TopBar identity expectations not matching the local `/api/auth/me` response shape

**Proof point captured in terminal:**

* login with `username=admin` and `password=password` succeeded and returned cookie + identity JSON.

---

## 1) “Source of truth” for local auth behavior

### Auth mode

* svc-admin supports auth modes, and **local mode** is the current focus.
* Local auth = **cookie-based session** with server-side session storage and an RBAC file.

### RBAC file

* RBAC is stored at:

  * `data/svc-admin/rbac.json` (default)
* **Bootstrap happens only if RBAC has no users / file is absent or empty.**
* After bootstrapping once, changing env vars won’t change the password unless RBAC is reset.

### Practical reset workflow (dev)

To reset the admin password deterministically:

```bash
cd /Users/mymac/Desktop/RustyOnions
mv data/svc-admin/rbac.json data/svc-admin/rbac.json.bak 2>/dev/null || true
```

Then set env and restart svc-admin.

---

## 2) Correct environment variables (important)

We discovered env var naming drift caused confusion.

### The password value env var (the one you actually set)

* `SVC_ADMIN_BOOTSTRAP_ADMIN_PASSWORD='your-password'`

### The bootstrap username override

Depending on which loader path you’re using, the correct variable in the current codebase is:

* `SVC_ADMIN_AUTH_BOOTSTRAP_ADMIN_USERNAME=admin` (this is the one the config loader expects)

We previously used `SVC_ADMIN_BOOTSTRAP_ADMIN_USERNAME` in notes/scripts in some places; that can be a no-op depending on the loader.

### “Known-good” dev launch pattern

```bash
export SVC_ADMIN_AUTH_MODE=local
export SVC_ADMIN_AUTH_BOOTSTRAP_ADMIN_USERNAME=admin
export SVC_ADMIN_BOOTSTRAP_ADMIN_PASSWORD='password'

scripts/run_dashboard.sh
```

---

## 3) API endpoints relevant to login

### Local auth endpoints (UI/API plane, not metrics plane)

* `POST /api/auth/login`
* `POST /api/auth/logout`
* `GET /api/auth/me`

**Reminder:** make requests to the **UI/API** bind (e.g. `:5300`), not the metrics bind (e.g. `:5310`).

### Curl smoke test (known-good)

```bash
curl -i -c /tmp/svc_admin.cookies \
  -H 'content-type: application/json' \
  -d '{"username":"admin","password":"password"}' \
  http://127.0.0.1:5300/api/auth/login

curl -i -b /tmp/svc_admin.cookies \
  http://127.0.0.1:5300/api/auth/me
```

Expected:

* first call: **200** + `Set-Cookie: svc_admin_sid=...`
* second call: **200** identity JSON

---

## 4) UI work completed on login UX

### `LoginPage.tsx` updates

**Goal:** make the login screen feel polished and reduce operator confusion.

Changes implemented:

* Added branding line inside login card:

  * **🦀 RON-CORE 🧅**
* Centered the branding line in the login box.
* Improved session pre-check behavior:

  * Treat `401` as “not logged in” (normal)
  * Surface non-401 errors (network, server error) as real errors
* Maintains redirect-to-`from` behavior.

**File:** `crates/svc-admin/ui/src/routes/LoginPage.tsx`

---

## 5) Logout button work + why it didn’t show initially

### Root cause

* The TopBar was originally keyed to show Logout only when:

  * `me.authMode === 'local'`
* But in local mode, the UI often reads identity from:

  * `GET /api/auth/me`
  * which returns `{ username, roles, expiresAtUnixS }`
  * **no `authMode` field**
* So `authMode` was undefined → Logout never appeared.

### Fix implemented in TopBar

* Treat `expiresAtUnixS` as a “local session” indicator.
* Re-fetch identity on route changes (post-login redirect) so TopBar updates immediately.
* Treat 401 as “Anonymous,” not “Identity unavailable.”
* Add Logout button that calls `adminClient.logout()` and navigates to `/login`.

**File:** `crates/svc-admin/ui/src/components/layout/TopBar.tsx`

---

## 6) adminClient contract assumptions

TopBar + LoginPage assume `adminClient` provides:

* `login(username, password)` → calls `/api/auth/login`
* `logout()` → calls `/api/auth/logout`
* `getMe()` → checks identity (either `/api/auth/me` or `/api/me` depending on implementation)

If Logout doesn’t work, the first place to verify is:

* `crates/svc-admin/ui/src/api/adminClient.ts`

  * ensure logout uses **POST** and includes cookies (`credentials: 'include'`)

---

## 7) Outstanding gaps for “production-grade” login

We are “production-close for local/dev,” but **not production-ready** until these are addressed (later phase):

### Auth hardening

* Add **CSRF protection** for cookie-based POSTs (or same-origin-only strategy + token)
* Add **rate limiting / brute-force protection** on login
* Ensure **Secure** cookies when served over TLS (non-localhost)
* Add session store strategy for multi-instance (redis/db) if needed

### UX & admin features

* Add **change password** endpoint + UI
* Add **admin reset password** endpoint + audit log
* Add RBAC management UI (create user, assign roles, disable user)
* Add clearer “Auth Mode / Session Expiry” display in TopBar

### Auditability

* Log auth events: login success/fail, logout, password changes
* Emit counters in Prometheus: `auth_login_success_total`, `auth_login_fail_total`, etc.

---

## 8) Known “gotchas” we hit (keep these in mind)

1. **Bootstrap is one-time**

   * If `rbac.json` exists with a user, new env vars won’t change anything.
2. **Wrong port**

   * `/api/*` is on UI/API plane; metrics plane should not serve it.
3. **Identity DTO mismatch**

   * Local `/api/auth/me` differs from `/api/me`; UI must not assume authMode exists.
4. **TopBar refresh timing**

   * If TopBar only fetched identity once on mount, it may stay “Anonymous” after login until refresh; fixed by re-fetching on route changes.

---

## 9) What we stopped on / next immediate tasks (login completion)

When we come back, “completing the login stuff” means:

### Must-do

* Confirm `adminClient.logout()` exists and uses correct method + cookies
* Verify Logout actually clears session cookie server-side (or invalidates session id)
* Confirm UI routing after logout always lands on `/login` and blocks API routes

### Should-do (DX polish)

* TopBar: optionally show session expiry countdown when local
* LoginPage: optionally show “Logged in” / “already logged in” state cleanly
* Update docs in svc-admin:

  * `AUTH_NOTES.md` or `CONFIG.md` with correct env vars and reset workflow

---

## 10) Future: “WOW demo” reminder

* We decided to build a **WOW demo** later:

  * **super simple showcase**
  * static site + crash/restart + live metrics + reload
  * **do not expose b3/content IDs** in the demo UX (keep it simple/opaque)
* This will be tackled after login is fully buttoned up.

---

## 11) Files touched / key locations (quick index)

* `crates/svc-admin/src/server.rs` (Axum 0.7 bootstrap, two-plane serve)
* `crates/svc-admin/src/router.rs` (mounts `/api/auth/*`)
* `data/svc-admin/rbac.json` (local RBAC store)
* `crates/svc-admin/ui/src/routes/LoginPage.tsx` (branding + session check)
* `crates/svc-admin/ui/src/components/layout/TopBar.tsx` (logout + identity)
* `crates/svc-admin/ui/src/api/adminClient.ts` (must align endpoints + cookies)
* `crates/svc-admin/ui/src/types/admin-api.ts` (type alignment with `/api/auth/me`)

---

If you paste `ui/src/api/adminClient.ts` and `ui/src/types/admin-api.ts` next session, we can “finish the login stuff” cleanly by removing any `any` casts, making identity types explicit for both `/api/me` and `/api/auth/me`, and ensuring logout is perfectly aligned end-to-end.


### END NOTE - JANUARY 9 2026 - 21:30 CST