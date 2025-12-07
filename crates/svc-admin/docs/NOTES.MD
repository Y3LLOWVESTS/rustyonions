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