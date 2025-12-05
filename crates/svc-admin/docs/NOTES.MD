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