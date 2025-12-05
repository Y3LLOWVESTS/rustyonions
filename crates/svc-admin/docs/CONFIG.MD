
````markdown
---
title: Configuration — svc-admin
crate: svc-admin
owner: Stevan White
last-reviewed: 2025-12-04
status: draft
template_version: 1.1
---

# Configuration — svc-admin

This document defines **all configuration** for `svc-admin`, including sources,
precedence, schema (types/defaults), validation, feature flags, live-reload behavior,
and security implications. It complements `README.md`, `docs/SECURITY.md`, and the
svc-admin IDB.

`svc-admin` is a **service crate**: an Axum-based HTTP server that:

- Serves the **admin dashboard SPA** (GUI).
- Exposes its own `/metrics`, `/healthz`, `/readyz`.
- Proxies admin-plane requests to **configured RON nodes** (macronode, micronode, future svc-*).
- Enforces **operator auth** (via svc-passport / ingress / dev-mode) before showing sensitive UI.

> **Crate prefix:**  
> All environment variables use `SVC_ADMIN_` as the prefix.

---

## 1) Sources & Precedence (Authoritative)

Configuration may come from multiple sources. **Precedence (highest wins):**

1. **Process flags** (CLI)  
2. **Environment variables**  
3. **Config file** (e.g., `svc-admin.toml` beside the binary)  
4. **Built-in defaults** (hard-coded)

> When reloading dynamically, the effective config is recomputed under the same precedence.

**Supported file formats:**

- **TOML (preferred)** — `svc-admin.toml`, `Config.toml`, or custom via `--config`.
- **JSON (optional/future)** — if added, must be explicitly documented.

**Default config file resolution order (first hit wins):**

1. `--config <path>` (if provided; if relative, resolved from `$CWD`).
2. `./svc-admin.toml`
3. `./Config.toml`

If no file is found, svc-admin starts with **built-in defaults** and environment overrides.

**Node configuration (targets):**

- **Primary source:** config file (`[nodes.<id>]` sections).
- Environment and CLI flags are used for **global settings** (bind, metrics, polling, UI defaults, auth mode), not for the full per-node list.

---

## 2) Quickstart Examples

### 2.1 Minimal service start (single-node, dev, no auth)

> **Dev-only.** Binds to loopback, `auth.mode=none`, HTTP to node.

```bash
cd /Users/mymac/Desktop/RustyOnions

RUST_LOG=info \
SVC_ADMIN_BIND_ADDR=127.0.0.1:5300 \
SVC_ADMIN_METRICS_ADDR=127.0.0.1:0 \
SVC_ADMIN_AUTH_MODE=none \
cargo run -p svc-admin -- \
  --node http://127.0.0.1:8090
````

This:

* Binds the svc-admin UI on `127.0.0.1:5300` (loopback).
* Exposes Prometheus metrics on an ephemeral port (`127.0.0.1:0` → OS chooses).
* Configures a single target node at `http://127.0.0.1:8090` (macronode or micronode admin).
* Runs with **dev-only auth mode** (`auth.mode=none`), which **MUST NOT** be used with `environment="prod"` or non-loopback binds.

---

### 2.2 Config file (TOML) — recommended

> Example for **dev** with optional app-plane playground and no TLS on svc-admin itself.

```toml
# svc-admin.toml

# Where svc-admin itself listens
bind_addr    = "127.0.0.1:5300"
metrics_addr = "127.0.0.1:0"

max_conns     = 1024
read_timeout  = "5s"
write_timeout = "5s"
idle_timeout  = "60s"

[log]
format = "json"       # "json" | "text"
level  = "info"       # "trace" | "debug" | "info" | "warn" | "error"

[tls]
enabled   = false
# cert_path = "/etc/ron/svc-admin/cert.pem"
# key_path  = "/etc/ron/svc-admin/key.pem"

[polling]
metrics_interval = "5s"       # how often to poll node /metrics
metrics_window   = "5m"       # how long of an in-memory window to keep

[ui]
read_only = true              # default: no mutating admin actions
theme     = "system"          # "system" | "light" | "dark" | "red-eye"
language  = "en-US"           # default locale key

[ui.dev]
enable_app_playground = true  # dev-only app-plane playground (uses ron-app-sdk-ts)

[actions]
enable_reload   = false       # allow POST /api/v1/reload via UI
enable_shutdown = false       # allow POST /api/v1/shutdown via UI

[auth]
mode              = "none"                  # "none" | "ingress" | "passport"
passport_base_url = "https://passport.dev" # only used when mode="passport"
client_id         = "svc-admin-dev"
issuer            = "https://passport.dev"
audience          = "svc-admin-dev"
# ingress_header_user   = "X-User"
# ingress_header_groups = "X-Groups"
# cookie_name           = "ron_passport"

# Node definitions (macronode, micronode, etc.)
[nodes.macronode_dev]
base_url      = "http://127.0.0.1:8090"
display_name  = "macronode-dev"
environment   = "dev"          # "dev" | "staging" | "prod"
insecure_http = true           # allow http for this node (dev only)
# macaroon_path = "/etc/ron/macaroon.macronode-dev"

[nodes.micronode_dev]
base_url      = "http://127.0.0.1:8091"
display_name  = "micronode-dev"
environment   = "dev"
insecure_http = true
```

---

### 2.3 Production-style config (behind svc-passport / ingress)

```toml
# svc-admin.toml (prod-style sketch)

bind_addr    = "0.0.0.0:5300"
metrics_addr = "127.0.0.1:9530"

max_conns     = 4096
read_timeout  = "5s"
write_timeout = "5s"
idle_timeout  = "60s"

[log]
format = "json"
level  = "info"

[tls]
enabled   = true
cert_path = "/etc/ron/svc-admin/cert.pem"
key_path  = "/etc/ron/svc-admin/key.pem"

[polling]
metrics_interval = "10s"
metrics_window   = "10m"

[ui]
read_only = true
theme     = "dark"
language  = "en-US"

[actions]
enable_reload   = false
enable_shutdown = false

[auth]
mode              = "passport"                      # enforce operator auth
passport_base_url = "https://passport.ron-core.dev"
client_id         = "svc-admin"
issuer            = "https://passport.ron-core.dev"
audience          = "svc-admin"
cookie_name       = "ron_passport"

[nodes.macronode_prod]
base_url      = "https://macronode-1.internal:8090"
display_name  = "macronode-1"
environment   = "prod"
insecure_http = false
macaroon_path = "/etc/ron/macaroon.macronode-prod"
```

---

### 2.4 CLI flags (override file/env)

```bash
cargo run -p svc-admin -- \
  --config svc-admin.toml \
  --bind 0.0.0.0:5300 \
  --metrics 127.0.0.1:9530 \
  --read-only \
  --ui-theme dark \
  --ui-language en-US \
  --auth-mode passport \
  --node https://127.0.0.1:8090
```

This:

* Loads `svc-admin.toml`.
* Overrides bind + metrics addresses.
* Forces **read-only UI** even if actions are enabled in the config.
* Overrides UI theme/language defaults.
* Overrides auth mode for this process.
* Adds or overrides a single-node entry for the current run (for quick dev).

> In production, prefer **file + env**, with CLI flags used sparingly for one-off testing.

---

## 3) Schema (Typed, With Defaults)

> **Prefix convention:** All env vars begin with `SVC_ADMIN_`.
>
> Nested keys are flattened in env as `SVC_ADMIN_<SECTION>_<KEY>`, e.g.:
>
> * `SVC_ADMIN_UI_THEME`, `SVC_ADMIN_AUTH_MODE`, `SVC_ADMIN_POLLING_METRICS_INTERVAL`.
>
> **Durations** accept `s`, `ms`, `m`, `h`. **Sizes** accept `B`, `KB`, `MB`, `MiB`.

### 3.1 Global svc-admin keys

| Key / Env Var                                                             | Type            | Default          | Description                                      | Security Notes                                   |
| ------------------------------------------------------------------------- | --------------- | ---------------- | ------------------------------------------------ | ------------------------------------------------ |
| `bind_addr` / `SVC_ADMIN_BIND_ADDR`                                       | socket          | `127.0.0.1:5300` | HTTP/UI bind address for svc-admin               | Public binds require auth + network review       |
| `metrics_addr` / `SVC_ADMIN_METRICS_ADDR`                                 | socket          | `127.0.0.1:0`    | Prometheus metrics bind for svc-admin itself     | Prefer localhost; external scrape via gateway    |
| `max_conns` / `SVC_ADMIN_MAX_CONNS`                                       | u32             | `1024`           | Max concurrent client connections                | Prevents FD exhaustion                           |
| `read_timeout` / `SVC_ADMIN_READ_TIMEOUT`                                 | duration        | `5s`             | Per-client read timeout                          | DoS mitigation                                   |
| `write_timeout` / `SVC_ADMIN_WRITE_TIMEOUT`                               | duration        | `5s`             | Per-client write timeout                         | DoS mitigation                                   |
| `idle_timeout` / `SVC_ADMIN_IDLE_TIMEOUT`                                 | duration        | `60s`            | Keep-alive idle shutdown                         | Resource hygiene                                 |
| `log.format` / `SVC_ADMIN_LOG_FORMAT`                                     | enum(text,json) | `json`           | Log format                                       | JSON recommended in prod for aggregation         |
| `log.level` / `SVC_ADMIN_LOG_LEVEL`                                       | enum            | `info`           | `trace`..`error`                                 | Avoid `trace` in prod                            |
| `tls.enabled` / `SVC_ADMIN_TLS_ENABLED`                                   | bool            | `false`          | Enable TLS on svc-admin listener                 | Use tokio-rustls only                            |
| `tls.cert_path` / `SVC_ADMIN_TLS_CERT_PATH`                               | path            | `""`             | PEM cert path for svc-admin                      | Secrets on disk; perms 0600                      |
| `tls.key_path` / `SVC_ADMIN_TLS_KEY_PATH`                                 | path            | `""`             | PEM key path for svc-admin                       | Zeroize in memory; never log                     |
| `polling.metrics_interval` / `SVC_ADMIN_METRICS_INTERVAL`                 | duration        | `5s`             | Poll interval for node `/metrics`                | Too low = load on nodes; too high = stale UI     |
| `polling.metrics_window` / `SVC_ADMIN_METRICS_WINDOW`                     | duration        | `5m`             | In-memory window for metrics visualization       | No long-term TSDB; bounded memory                |
| `ui.read_only` / `SVC_ADMIN_READ_ONLY`                                    | bool            | `true`           | Force UI to be read-only                         | MUST default true; prod posture                  |
| `ui.theme` / `SVC_ADMIN_UI_THEME`                                         | enum            | `"system"`       | `"system"`, `"light"`, `"dark"`, `"red-eye"`     | Only affects UX; no security effect              |
| `ui.language` / `SVC_ADMIN_UI_LANGUAGE`                                   | string (locale) | `"en-US"`        | Default language pack key                        | i18n; keys must map to existing pack             |
| `ui.dev.enable_app_playground` / `SVC_ADMIN_UI_DEV_ENABLE_APP_PLAYGROUND` | bool            | `false`          | Enable dev app-plane playground (ron-app-sdk-ts) | Dev-only; MUST be `false` in prod                |
| `actions.enable_reload` / `SVC_ADMIN_ENABLE_RELOAD`                       | bool            | `false`          | Allow config reload actions via UI               | Powerful; require policy + audit                 |
| `actions.enable_shutdown` / `SVC_ADMIN_ENABLE_SHUTDOWN`                   | bool            | `false`          | Allow shutdown actions via UI                    | Highest risk; enable only with strict governance |

### 3.2 Auth config keys

| Key / Env Var                                                         | Type         | Default          | Description                                              | Security Notes                                               |
| --------------------------------------------------------------------- | ------------ | ---------------- | -------------------------------------------------------- | ------------------------------------------------------------ |
| `auth.mode` / `SVC_ADMIN_AUTH_MODE`                                   | enum         | `"none"`         | `"none"` | `"ingress"` | `"passport"`                    | `"none"` dev-only; prod MUST use `"ingress"` or `"passport"` |
| `auth.passport_base_url` / `SVC_ADMIN_AUTH_PASSPORT_BASE_URL`         | URL string   | `""`             | Base URL of svc-passport (for redirects/links)           | Required when `auth.mode="passport"`                         |
| `auth.client_id` / `SVC_ADMIN_AUTH_CLIENT_ID`                         | string       | `"svc-admin"`    | OIDC/Passport client_id for svc-admin                    | Tied to passport config                                      |
| `auth.issuer` / `SVC_ADMIN_AUTH_ISSUER`                               | string (URL) | `""`             | Expected issuer claim for passports                      | Required when `auth.mode="passport"`                         |
| `auth.audience` / `SVC_ADMIN_AUTH_AUDIENCE`                           | string       | `"svc-admin"`    | Expected audience claim for passports                    | Required when `auth.mode="passport"`                         |
| `auth.cookie_name` / `SVC_ADMIN_AUTH_COOKIE_NAME`                     | string       | `"ron_passport"` | Name of cookie carrying passport token (if cookie-based) | Sensitive; must not be guessable in multi-tenant setups      |
| `auth.ingress_header_user` / `SVC_ADMIN_AUTH_INGRESS_HEADER_USER`     | string       | `"X-User"`       | Header used by ingress to convey user identity           | Only used when `auth.mode="ingress"`                         |
| `auth.ingress_header_groups` / `SVC_ADMIN_AUTH_INGRESS_HEADER_GROUPS` | string       | `"X-Groups"`     | Header used by ingress to convey group/role info         | Only used when `auth.mode="ingress"`                         |

### 3.3 Node-specific keys (config file only; env/CLI are global)

| Key (TOML)                   | Type            | Default      | Description                                            | Security Notes                                                 |
| ---------------------------- | --------------- | ------------ | ------------------------------------------------------ | -------------------------------------------------------------- |
| `nodes.<id>.base_url`        | URL (string)    | **required** | Base URL of the node’s admin plane (e.g., `https://…`) | Use HTTPS in prod; HTTP allowed only with `insecure_http=true` |
| `nodes.<id>.display_name`    | string          | `<id>`       | Human-friendly name for the node                       | Cosmetic only                                                  |
| `nodes.<id>.environment`     | enum            | `"dev"`      | `"dev"`, `"staging"`, or `"prod"`                      | Drives UX hints + some validation; no auth effect by itself    |
| `nodes.<id>.insecure_http`   | bool            | `false`      | Allow HTTP (no TLS) to this node                       | Only for localhost/dev; forbid in prod                         |
| `nodes.<id>.macaroon_path`   | path (optional) | `""`         | Path to capability token/macaroon for this node        | Treat as secret; never log contents                            |
| `nodes.<id>.default_timeout` | duration        | `5s`         | Per-node proxy timeout (admin calls)                   | DoS protection, avoid “hung UI”                                |

> **Note:** Per-node secrets (macaroons/tokens) MUST live in config files or secret mounts, not in logs.
> If future env overrides for per-node secrets are added, they MUST be documented with a clear naming convention.

---

## 4) Validation Rules (Fail-Closed)

On startup or reload, svc-admin builds an effective `Config` snapshot and applies **strict validation**:

### 4.1 Global

* `bind_addr` and `metrics_addr`:

  * Must parse as valid `SocketAddr`.
  * Ports `<1024` require OS privileges; config should warn in logs.

* `max_conns` **> 0**.

* `polling.metrics_interval` **>= 250ms** (below that is rejected as too aggressive).

* `polling.metrics_window` **>= 10s** and **>= metrics_interval**.

* `log.level` and `log.format` must be valid enum values.

* If `tls.enabled=true`:

  * Both `cert_path` and `key_path` must be present, must exist, and must be readable.
  * Key file must not be world-readable (on Unix: mode not including “others” read/write/exec).

* `ui.theme`:

  * Must be `"system"`, `"light"`, `"dark"`, or `"red-eye"`; else config is rejected.

* `ui.language`:

  * Must match a supported pack key (e.g., `"en-US"`). If unknown:

    * either rejected, or mapped to `"en-US"` with a loud warning (implementation choice; default behavior should be documented).

* `ui.dev.enable_app_playground`:

  * Allowed in all envs at config level, but runtime SHOULD log a warning or refuse to enable if **all** nodes are `"prod"`.

### 4.2 Auth

* `auth.mode` must be one of `"none"`, `"ingress"`, `"passport"`.

* If `auth.mode = "passport"`:

  * `passport_base_url`, `issuer`, and `client_id` MUST be non-empty.
  * `audience` MUST be non-empty (default `"svc-admin"` is acceptable).
  * Optional `cookie_name` must be a valid cookie token name (no spaces, control chars).

* If `auth.mode = "ingress"`:

  * `ingress_header_user` and `ingress_header_groups` must be non-empty, safe header names.

* If **any node** has `environment="prod"` **and**:

  * `auth.mode = "none"` **and** `bind_addr` is not loopback (`127.0.0.1`/`::1`), then:

    * **Fail fast**: svc-admin MUST refuse to start in this configuration.
  * `auth.mode = "none"` but `bind_addr` is loopback:

    * Allowed but MUST log a clear `SECURITY_WARNING` that dev-mode auth is used against prod nodes.

### 4.3 Node entries

For each `[nodes.<id>]`:

* `base_url`:

  * Must be a valid URL with `http` or `https` scheme.
  * If scheme is `http`, `insecure_http` MUST be `true`; otherwise startup fails.

* `display_name`:

  * If omitted, defaults to `<id>`.

* `environment`:

  * Must be one of `"dev"`, `"staging"`, `"prod"`. Unknown values are rejected.

* `macaroon_path` (if set):

  * File must exist and be readable.
  * svc-admin must NOT read or log contents during validation; only existence is checked.

* `default_timeout`:

  * Must be > 0 and not absurdly large (e.g., we may cap at 5 minutes to catch misconfig).

### 4.4 On validation failure

svc-admin MUST:

* Log a **structured error** (JSON) with:

  * which key is invalid,
  * why,
  * **without** including secret values.
* **Exit non-zero** (for service) rather than starting in a partially invalid state.

For dynamic reload (if supported), invalid config MUST leave the **previous valid snapshot** in place and reject the new one with a clear log.

---

## 5) Dynamic Reload (If Supported)

Dynamic reload is **optional** but recommended; design assumes:

### 5.1 Triggers

* SIGHUP signal, or
* HTTP admin endpoint on svc-admin itself (e.g., `POST /admin/reload`), or
* Bus event `ConfigUpdated { version: <u64> }` if svc-admin joins RON’s bus.

### 5.2 Reload semantics

* **Safe to reload in-place:**

  * `log.level`, `log.format`
  * `polling.metrics_interval`, `polling.metrics_window`
  * `ui.read_only`, `ui.theme`, `ui.language`, `ui.dev.enable_app_playground`
  * `actions.*` flags (if we choose to allow runtime action toggles)
  * Node list (`nodes.*`) **if** we rebind only the internal proxy configuration, not the svc-admin listener.
  * `auth.mode` and other auth fields (if token validation stack supports hot-swap), with care.

* **Disruptive or restart-required:**

  * `bind_addr`, `metrics_addr` — require socket rebind.
  * `tls.*` for svc-admin — require reinitializing TLS acceptor.

A “disruptive” reload SHOULD be done as “drain and restart” rather than hot-swap.

### 5.3 Atomicity

* Parse & validate a **new `Config` instance** off to the side.
* On success, swap an `Arc<Config>` in a single, non-blocking step.
* No `.await` is held while holding any config lock.

### 5.4 Audit

* On reload, svc-admin SHOULD emit:

  * a structured log entry, e.g. `{"event":"config_reload","old_version":X,"new_version":Y}`.
  * If integrated with `ron-audit`, an audit event with:

    * node id = `svc-admin`,
    * who triggered it (if HTTP admin endpoint used),
    * success/failure.

---

## 6) CLI Flags (Canonical)

> Flags are optional; prefer file + env in production.

Canonical flags (final naming implemented with `clap` or similar):

```text
--config <path>                  # Load svc-admin.toml (baseline config)
--bind <ip:port>                 # Override bind_addr
--metrics <ip:port>              # Override metrics_addr
--max-conns <num>                # Override max_conns
--read-timeout <dur>             # Override read_timeout
--write-timeout <dur>            # Override write_timeout
--idle-timeout <dur>             # Override idle_timeout
--log-format <json|text>
--log-level <trace|debug|info|warn|error>

# UI/behavior
--read-only                      # Force UI read-only mode
--ui-theme <system|light|dark|red-eye>
--ui-language <locale>           # e.g. en-US, es-ES
--allow-reload                   # Equivalent to actions.enable_reload=true
--allow-shutdown                 # Equivalent to actions.enable_shutdown=true
--enable-app-playground          # Equivalent to ui.dev.enable_app_playground=true

# Auth
--auth-mode <none|ingress|passport>
--auth-passport-base-url <url>
--auth-client-id <id>
--auth-issuer <issuer-url>
--auth-audience <aud>
--auth-cookie-name <name>
--auth-ingress-header-user <header-name>
--auth-ingress-header-groups <header-name>

# Quick single-node dev override
--node <url>                     # Add/override a simple node entry for this run
--node-env <dev|staging|prod>    # Environment label for --node (default: dev)
--node-insecure-http             # Allow http for --node (dev only)

# TLS for svc-admin
--tls                            # Shorthand for tls.enabled=true
--tls-cert <path>
--tls-key <path>
```

Rules:

* CLI flags MUST NOT silently introduce new behavior not representable in the config schema; they are just overrides.
* Truly one-off testing flags (e.g., `--dev-demo`) MUST be clearly labeled and default off.

---

## 7) Feature Flags (Cargo)

| Feature     | Default | Effect                                                                               |
| ----------- | ------: | ------------------------------------------------------------------------------------ |
| `tls`       |     off | Enables tokio-rustls dependencies and `tls.*` config keys                            |
| `cli`       |      on | Enable CLI parsing (clap/structopt) for flags above                                  |
| `embed-spa` |      on | Embed SPA assets in the binary (disable for dev builds that serve from disk)         |
| `dev-tools` |      on | Include dev-only endpoints (e.g., fake-node hooks, extra logging, playground wiring) |

Notes:

* `tls` feature MUST be consistent with workspace TLS policy (tokio-rustls, not native-tls).
* `embed-spa` off may expect a `SPA_DIST_DIR` env var or config key for local dev static assets.
* `dev-tools` MUST be safe to disable for production builds (no dev-only endpoints).

---

## 8) Security Implications

Key points:

* **Public binds (`0.0.0.0`):**

  * Exposes the admin UI. In prod, this MUST be behind:

    * an authenticated reverse proxy or
    * svc-passport-based auth (mode `"passport"`).
  * `auth.mode="none"` with non-loopback bind MUST be rejected in validation.

* **TLS for svc-admin:**

  * Use `tokio_rustls::rustls::ServerConfig` only.
  * Cert/key paths are secrets; restrict filesystem permissions.
  * No automatic cert discovery; all paths must be explicit.

* **Auth modes:**

  * `auth.mode="none"`:

    * Dev-only; only acceptable when:

      * all nodes are non-prod **or**
      * bind_addr is loopback.
  * `auth.mode="ingress"`:

    * svc-admin trusts headers from ingress.
    * Ingress is responsible for validating end-user identity.
  * `auth.mode="passport"`:

    * svc-admin validates tokens directly against svc-passport’s issuer/audience/keys.

* **Node credentials (macaroons/tokens):**

  * Stored in `nodes.<id>.macaroon_path` or equivalent *only*.
  * Never log them.
  * Treat them like root access to the node’s admin plane.

* **Read-only default:**

  * `ui.read_only=true` + `actions.enable_* = false` is the default posture.
  * Any deviation MUST be intentional and documented in ops runbooks.

* **Insecure HTTP:**

  * `nodes.<id>.insecure_http=true` is only acceptable for localhost/dev.
  * svc-admin MUST reject `http` URLs in prod configs unless explicitly allowed.

* **Metrics polling:**

  * Too aggressive polling can serve as an amplification vector (driving load on nodes).
  * Config must keep metrics intervals sane (`>=250ms`, usually 2–10s).

* **App playground:**

  * `ui.dev.enable_app_playground` exposes app-plane tooling.
  * In prod, it SHOULD be disabled; if enabled, it MUST be gated by strong operator auth and clearly labeled “dev-only”.

See `docs/SECURITY.md` for a full threat model.

---

## 9) Compatibility & Migration

* **Backwards compatibility:**

  * New keys MUST have safe defaults (preserving prior behavior).
  * Example: adding `ui.theme`, `ui.language`, and `auth.mode` with defaults (`"system"`, `"en-US"`, `"none"`) preserves dev behavior but is gated in prod by validation rules.

* **Renames:**

  * When renaming keys (e.g., `metrics_bind` → `metrics_addr`), keep:

    * an alias for env vars (e.g., `SVC_ADMIN_METRICS_BIND`) for at least one minor version,
    * a deprecation warning in logs when legacy keys are used.

* **Breaking changes:**

  * Any schema change that alters semantics of existing config keys or makes previously valid configs invalid MUST be paired with:

    * a **major version bump**, and
    * clear migration notes in `CHANGELOG.md` + `docs/MIGRATION.md`.

**Deprecation table (to maintain over time):**

| Old Key                 | New Key                    | Removal Target | Notes                                |
| ----------------------- | -------------------------- | -------------: | ------------------------------------ |
| `metrics_bind`          | `metrics_addr`             |         v1.2.0 | Accept both for 1.x; warn on old key |
| `nodes.<id>.token_path` | `nodes.<id>.macaroon_path` |         v1.3.0 | Unified naming with `ron-policy`     |

---

## 10) Reference Implementation (Rust)

> Minimal example for `Config` with file + env + CLI (serde-based).
> This snippet is intended to live in `src/config.rs` for `svc-admin`.

```rust
use std::{collections::BTreeMap, net::SocketAddr, path::PathBuf, time::Duration};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsCfg {
    pub enabled: bool,
    pub cert_path: Option<PathBuf>,
    pub key_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollingCfg {
    #[serde(with = "humantime_serde", default = "default_metrics_interval")]
    pub metrics_interval: Duration,
    #[serde(with = "humantime_serde", default = "default_metrics_window")]
    pub metrics_window: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogCfg {
    #[serde(default = "default_log_format")]
    pub format: String, // "json" | "text"
    #[serde(default = "default_log_level")]
    pub level: String,  // "trace" | "debug" | "info" | "warn" | "error"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiDevCfg {
    #[serde(default)]
    pub enable_app_playground: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiCfg {
    #[serde(default = "default_read_only")]
    pub read_only: bool,
    #[serde(default = "default_ui_theme")]
    pub theme: String,      // "system" | "light" | "dark" | "red-eye"
    #[serde(default = "default_ui_language")]
    pub language: String,   // "en-US", "es-ES", ...
    #[serde(default)]
    pub dev: UiDevCfg,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionsCfg {
    #[serde(default)]
    pub enable_reload: bool,
    #[serde(default)]
    pub enable_shutdown: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthCfg {
    #[serde(default = "default_auth_mode")]
    pub mode: String,               // "none" | "ingress" | "passport"
    pub passport_base_url: Option<String>,
    pub client_id: Option<String>,
    pub issuer: Option<String>,
    pub audience: Option<String>,
    #[serde(default = "default_cookie_name")]
    pub cookie_name: String,
    #[serde(default = "default_ingress_header_user")]
    pub ingress_header_user: String,
    #[serde(default = "default_ingress_header_groups")]
    pub ingress_header_groups: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCfg {
    pub base_url: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default = "default_environment")]
    pub environment: String, // "dev" | "staging" | "prod"
    #[serde(default)]
    pub insecure_http: bool,
    #[serde(default)]
    pub macaroon_path: Option<PathBuf>,
    #[serde(with = "humantime_serde", default = "default_node_timeout")]
    pub default_timeout: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_bind_addr")]
    pub bind_addr: SocketAddr,
    #[serde(default = "default_metrics_addr")]
    pub metrics_addr: SocketAddr,
    #[serde(default = "default_max_conns")]
    pub max_conns: u32,
    #[serde(with = "humantime_serde", default = "default_5s")]
    pub read_timeout: Duration,
    #[serde(with = "humantime_serde", default = "default_5s")]
    pub write_timeout: Duration,
    #[serde(with = "humantime_serde", default = "default_60s")]
    pub idle_timeout: Duration,

    #[serde(default)]
    pub tls: TlsCfg,
    #[serde(default)]
    pub polling: PollingCfg,
    #[serde(default)]
    pub log: LogCfg,
    #[serde(default)]
    pub ui: UiCfg,
    #[serde(default)]
    pub actions: ActionsCfg,
    #[serde(default)]
    pub auth: AuthCfg,
    #[serde(default)]
    pub nodes: BTreeMap<String, NodeCfg>,
}

// Defaults

fn default_bind_addr() -> SocketAddr {
    "127.0.0.1:5300".parse().expect("valid default bind addr")
}
fn default_metrics_addr() -> SocketAddr {
    "127.0.0.1:0".parse().expect("valid default metrics addr")
}
fn default_max_conns() -> u32 { 1024 }
fn default_5s() -> Duration { Duration::from_secs(5) }
fn default_60s() -> Duration { Duration::from_secs(60) }
fn default_metrics_interval() -> Duration { Duration::from_secs(5) }
fn default_metrics_window() -> Duration { Duration::from_secs(300) } // 5m
fn default_log_format() -> String { "json".to_string() }
fn default_log_level() -> String { "info".to_string() }
fn default_read_only() -> bool { true }
fn default_environment() -> String { "dev".to_string() }
fn default_node_timeout() -> Duration { Duration::from_secs(5) }
fn default_ui_theme() -> String { "system".to_string() }
fn default_ui_language() -> String { "en-US".to_string() }
fn default_auth_mode() -> String { "none".to_string() }
fn default_cookie_name() -> String { "ron_passport".to_string() }
fn default_ingress_header_user() -> String { "X-User".to_string() }
fn default_ingress_header_groups() -> String { "X-Groups".to_string() }

impl Default for TlsCfg {
    fn default() -> Self {
        Self {
            enabled: false,
            cert_path: None,
            key_path: None,
        }
    }
}

impl Default for PollingCfg {
    fn default() -> Self {
        Self {
            metrics_interval: default_metrics_interval(),
            metrics_window: default_metrics_window(),
        }
    }
}

impl Default for LogCfg {
    fn default() -> Self {
        Self {
            format: default_log_format(),
            level: default_log_level(),
        }
    }
}

impl Default for UiDevCfg {
    fn default() -> Self {
        Self {
            enable_app_playground: false,
        }
    }
}

impl Default for UiCfg {
    fn default() -> Self {
        Self {
            read_only: default_read_only(),
            theme: default_ui_theme(),
            language: default_ui_language(),
            dev: UiDevCfg::default(),
        }
    }
}

impl Default for ActionsCfg {
    fn default() -> Self {
        Self {
            enable_reload: false,
            enable_shutdown: false,
        }
    }
}

impl Default for AuthCfg {
    fn default() -> Self {
        Self {
            mode: default_auth_mode(),
            passport_base_url: None,
            client_id: Some("svc-admin".to_string()),
            issuer: None,
            audience: Some("svc-admin".to_string()),
            cookie_name: default_cookie_name(),
            ingress_header_user: default_ingress_header_user(),
            ingress_header_groups: default_ingress_header_groups(),
        }
    }
}

// Validation

impl Config {
    /// Validate config according to the rules in the blueprint.
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.max_conns == 0 {
            anyhow::bail!("max_conns must be > 0");
        }

        if self.polling.metrics_interval < Duration::from_millis(250) {
            anyhow::bail!("metrics_interval must be >= 250ms");
        }

        if self.polling.metrics_window < self.polling.metrics_interval {
            anyhow::bail!("metrics_window must be >= metrics_interval");
        }

        if self.tls.enabled {
            match (&self.tls.cert_path, &self.tls.key_path) {
                (Some(c), Some(k)) if c.exists() && k.exists() => {}
                _ => anyhow::bail!("TLS enabled but cert_path/key_path missing or unreadable"),
            }
        }

        // UI theme validation
        match self.ui.theme.as_str() {
            "system" | "light" | "dark" | "red-eye" => {}
            other => anyhow::bail!("invalid ui.theme: {other}"),
        }

        // Auth mode validation
        match self.auth.mode.as_str() {
            "none" | "ingress" | "passport" => {}
            other => anyhow::bail!("invalid auth.mode: {other}"),
        }

        if self.auth.mode == "passport" {
            if self.auth.passport_base_url.as_deref().unwrap_or("").is_empty() {
                anyhow::bail!("auth.mode=passport requires auth.passport_base_url");
            }
            if self.auth.issuer.as_deref().unwrap_or("").is_empty() {
                anyhow::bail!("auth.mode=passport requires auth.issuer");
            }
            if self.auth.client_id.as_deref().unwrap_or("").is_empty() {
                anyhow::bail!("auth.mode=passport requires auth.client_id");
            }
            if self.auth.audience.as_deref().unwrap_or("").is_empty() {
                anyhow::bail!("auth.mode=passport requires auth.audience");
            }
        }

        let bind_is_loopback = self.bind_addr.ip().is_loopback();

        // Node validations (and prod+auth interactions)
        let mut has_prod_node = false;

        for (id, node) in &self.nodes {
            if !(node.base_url.starts_with("http://")
                || node.base_url.starts_with("https://"))
            {
                anyhow::bail!("node[{id}]: base_url must start with http:// or https://");
            }
            if node.base_url.starts_with("http://") && !node.insecure_http {
                anyhow::bail!(
                    "node[{id}]: http:// base_url requires insecure_http=true (dev only)"
                );
            }
            if !matches!(
                node.environment.as_str(),
                "dev" | "staging" | "prod"
            ) {
                anyhow::bail!(
                    "node[{id}]: environment must be 'dev', 'staging', or 'prod'"
                );
            }
            if node.environment == "prod" {
                has_prod_node = true;
            }
            if let Some(path) = &node.macaroon_path {
                if !path.exists() {
                    anyhow::bail!("node[{id}]: macaroon_path does not exist: {}", path.display());
                }
            }
            if node.default_timeout == Duration::from_secs(0) {
                anyhow::bail!("node[{id}]: default_timeout must be > 0");
            }
        }

        if has_prod_node && self.auth.mode == "none" && !bind_is_loopback {
            anyhow::bail!(
                "auth.mode=none is not allowed with prod nodes and non-loopback bind_addr"
            );
        }

        Ok(())
    }
}
```

---

## 11) Test Matrix

| Scenario                                                                  | Expected Outcome                                                             |
| ------------------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| Missing `svc-admin.toml`                                                  | Start with built-in defaults + env; log a warning                            |
| Invalid `bind_addr` env/flag                                              | Fail fast with clear error; exit non-zero                                    |
| TLS enabled but cert/key missing                                          | Fail fast; clear error; do not start                                         |
| `metrics_interval` < 250ms                                                | Fail fast; config rejected                                                   |
| Node with `http://` base_url, `insecure_http=false`                       | Fail fast; log that insecure_http must be true for http                      |
| Node with unknown environment `"qa"`                                      | Fail fast; config rejected                                                   |
| Node macaroon path missing                                                | Fail fast; do not attempt to start                                           |
| `auth.mode="passport"` without issuer/client_id/audience/base_url         | Fail fast; config rejected                                                   |
| At least one node `environment="prod"` + `auth.mode="none"` + public bind | Fail fast; config rejected; security warning in logs                         |
| All nodes `environment="dev"` + `auth.mode="none"` + loopback bind        | Start with dev auth mode; log dev-mode warning                               |
| SIGHUP (if implemented)                                                   | Reload config; if valid, swap snapshot; if invalid, retain old config        |
| UI read-only + `actions.*` true                                           | UI still refuses to present mutating actions (read_only trumps actions)      |
| `ui.theme` invalid                                                        | Fail fast; config rejected                                                   |
| Single-node dev (`--node http://127.0.0.1`)                               | Start with one node in config; UI shows that node; validation still enforced |

---

## 12) Mermaid — Config Resolution Flow

```mermaid
flowchart TB
  A[Defaults] --> D[Merge]
  B[Config file (svc-admin.toml)] --> D
  C[Env Vars (SVC_ADMIN_*)] --> D
  E[CLI Flags] --> D
  D --> V{Validate}
  V -- ok --> R[Runtime Config Snapshot]
  V -- fail --> X[Exit / Keep Old Snapshot]
  style R fill:#0369a1,stroke:#0c4a6e,color:#fff
```

---

## 13) Operational Notes

* Keep **production `svc-admin.toml`** under version control (private repo or secret store).
* In container deployments:

  * Prefer **env vars** for top-level tuning.
  * Mount `svc-admin.toml` and macaroon files as **read-only secrets**.
* Restrict svc-admin’s bind address:

  * Typically `127.0.0.1:5300` behind a reverse proxy, or
  * A private VPC subnet; never the public Internet without strong auth.
* Treat node macaroon/token files as **equivalent to root access** on those nodes.
* Configure `auth.mode` thoughtfully:

  * `none` for local/dev only.
  * `ingress` when an upstream (e.g., Envoy/NGINX) already enforces auth.
  * `passport` when svc-admin validates passports itself.
* Keep `ui.dev.enable_app_playground` off in prod unless you explicitly want app-plane dev tooling in the admin console and have strong RBAC.
* Update this document whenever:

  * New config keys are added.
  * Defaults change.
  * Any breaking config behavior is introduced.

```
```
