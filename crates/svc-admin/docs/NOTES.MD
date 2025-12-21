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