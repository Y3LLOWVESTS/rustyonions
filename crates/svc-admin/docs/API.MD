

````md
---
title: API Surface & SemVer Reference — svc-admin
status: draft
msrv: 1.80.0
last-updated: 2025-12-04
audience: contributors, auditors, API consumers
---

# API.md — svc-admin

## 0. Purpose

This document captures the **public API surface** of `svc-admin`:

- Snapshot of exported **Rust types** (for internal integration / tests).
- Description of the **HTTP/JSON surface** that the SPA and external tools call.
- CLI flags and config keys that shape runtime behavior.
- SemVer discipline for what counts as a breaking vs. additive change.
- Alignment with `CHANGELOG.md`, `IDB.md`, `CONFIG.MD`, and `SECURITY.MD`. :contentReference[oaicite:0]{index=0}

`svc-admin` is primarily a **service crate**:

- The **canonical public API** is:
  - its **CLI** (`svc-admin` binary),
  - its **HTTP endpoints** (`/`, `/api/...`, `/healthz`, `/readyz`, `/metrics`),
  - the **JSON shapes** used by the React SPA and any external admin tooling.
- The Rust library surface exists mainly to support:
  - composition in tests and harnesses (fake-node / multi-node),
  - reuse of DTOs and config parsing.

---

## 1. Public API Surface

### 1.1 Rust API Surface (planned / enforced)

The intention is that `svc-admin` exposes a **small, well-defined library surface** for configuration, DTOs, and server bootstrap. Everything else is internal.

Expected `cargo public-api --simplified` snapshot (illustrative; keep this in sync with real output once the crate is implemented):

```text
# Root

pub mod config
pub mod dto
pub mod server
pub mod error

# config

pub struct Config
pub struct UiCfg
pub struct AuthCfg
pub struct NodeCfg
pub struct NodesCfg
pub fn load_config_from_env_and_args() -> Result<Config, error::Error>

# dto

/// Projection of a node's admin status as seen by svc-admin.
pub struct AdminStatusView {
    pub profile: String,          // "macronode" | "micronode" | etc.
    pub version: String,          // node semantic version
    pub planes: Vec<PlaneStatus>, // per-plane health/ready info
    pub amnesia: bool,
}

pub struct PlaneStatus {
    pub name: String,             // "gateway", "overlay", "kv", "facets", ...
    pub health: String,           // "ok" | "fail" | "unknown"
    pub ready: bool,
    pub restart_count: u64,
    pub notes: Option<String>,
}

pub struct UiConfigDto {
    pub default_theme: String,        // "system" | "light" | "dark" | "red-eye"
    pub available_themes: Vec<String>,
    pub default_language: String,     // "en-US", "es-ES", ...
    pub available_languages: Vec<String>,
    pub read_only: bool,
}

pub struct MeResponse {
    pub subject: String,              // user id / principal
    pub display_name: Option<String>,
    pub roles: Vec<String>,           // "viewer" | "admin" | ...
    pub auth_mode: String,            // "none" | "ingress" | "passport"
    pub login_url: Option<String>,    // present on 401 in passport mode
}

pub struct NodeSummary {
    pub id: String,
    pub display_name: String,
    pub environment: String,          // "dev" | "staging" | "prod" | ...
    pub amnesia: bool,
}

pub struct MetricsSummaryDto { /* short-window counters / histograms */ }
pub struct FacetMetricsDto   { /* grouped per-facet summaries        */ }

# server

pub struct AppState { /* shared state (Config, clients, caches) */ }

pub async fn build_router(state: AppState) -> axum::Router;
pub async fn run(config: Config) -> Result<(), error::Error>;

# error

pub enum Error {
    Config(ConfigError),
    Io(std::io::Error),
    Http(reqwest::Error),
    Auth(AuthError),
    Upstream(UpstreamError),
    // #[non_exhaustive] planned for 0.2.0+
}

pub type Result<T> = std::result::Result<T, Error>;
````

Notes:

* `AdminStatusView` and `PlaneStatus` exactly reflect the IDB DTOs and are used as the **canonical normalized status view**. 
* DTO modules should be `serde`-driven and used for both Rust and JSON, with `#[serde(deny_unknown_fields)]` where appropriate to catch schema drift. 
* `server::run` is the main entrypoint for the binary; the actual `bin/svc-admin.rs` should be a thin wrapper that calls this.

**Discipline:** anything that is **not** explicitly listed here should be treated as **internal** (subject to change without SemVer guarantees), even if it happens to be `pub` for crate-internal reasons.

### 1.2 CLI Surface

The `svc-admin` binary exposes a small set of stable CLI flags that map to the config schema in `CONFIG.MD`. 

Current / planned flags:

* `svc-admin --help`
* `svc-admin --version`
* `svc-admin --config <PATH>` — override default `svc-admin.toml` path.
* `svc-admin --bind-addr <ADDR:PORT>` — override `bind_addr` in config (optional; may remain experimental).
* `svc-admin --metrics-addr <ADDR:PORT>` — override `metrics_addr` in config (optional).
* `svc-admin --log-level <LEVEL>` — override `log.level` (`error|warn|info|debug|trace`).

These are considered **public API** once documented and relied upon by deployment scripts.

Env vars:

* All env vars documented in `CONFIG.MD` (`SVC_ADMIN_*`) are part of the **configuration API surface** and must be treated as such (additive-only changes, deprecate before removal). 

### 1.3 HTTP / JSON API Surface

This is the **primary API** for consumers (the SPA, CLI tools, scripts, future plugins). It is divided into:

* **Service health / metrics:**

  * `GET /healthz`
  * `GET /readyz`
  * `GET /metrics`

* **SPA & static assets:**

  * `GET /` → SPA index.
  * `GET /assets/*` → static JS/CSS/assets (when embedded).

* **Admin JSON API (svc-admin-local):**

  | Method | Path                              | Description                                                  |
  | ------ | --------------------------------- | ------------------------------------------------------------ |
  | GET    | `/api/ui-config`                  | Returns `UiConfigDto` with theme, language, read-only flags. |
  | GET    | `/api/me`                         | Returns `MeResponse` describing the current operator.        |
  | GET    | `/api/nodes`                      | List `NodeSummary` from config.                              |
  | GET    | `/api/nodes/{id}/status`          | Returns `AdminStatusView` for a node.                        |
  | GET    | `/api/nodes/{id}/metrics/summary` | Returns `MetricsSummaryDto` (short-window, aggregated).      |
  | GET    | `/api/nodes/{id}/metrics/facets`  | Returns `FacetMetricsDto` grouped by facet.                  |

* **Future / controlled mutating endpoints (feature-gated):** 

  | Method | Path                       | Description                       | Status         |
  | ------ | -------------------------- | --------------------------------- | -------------- |
  | POST   | `/api/nodes/{id}/reload`   | Ask node to reload its config.    | future / gated |
  | POST   | `/api/nodes/{id}/shutdown` | Ask node to shut down gracefully. | future / gated |

  These map to node admin endpoints (`/api/v1/reload`, `/api/v1/shutdown`) and MUST remain behind **config + auth + audit** gates.

#### 1.3.1 JSON shapes (sketch)

* `GET /api/ui-config` → `UiConfigDto`:

  ```json
  {
    "defaultTheme": "system",
    "availableThemes": ["system", "light", "dark", "red-eye"],
    "defaultLanguage": "en-US",
    "availableLanguages": ["en-US"],
    "readOnly": true
  }
  ```

  Backed by `ui.*` and `ui.dev.*` config. 

* `GET /api/me` → `MeResponse`:

  ```json
  {
    "subject": "stevan@example.com",
    "displayName": "Stevan White",
    "roles": ["admin"],
    "authMode": "passport",
    "loginUrl": null
  }
  ```

  * In `auth.mode="none"`, returns a synthetic dev identity. 
  * In `auth.mode="passport"`, 401 responses MAY include `{"loginUrl": "https://passport.local/login?..."}`
    to drive SPA sign-in. 

* `GET /api/nodes` → `NodeSummary[]`:

  ```json
  [
    {
      "id": "macronode-dev",
      "displayName": "macronode-dev",
      "environment": "dev",
      "amnesia": false
    }
  ]
  ```

  Derived from `[nodes.*]` config entries and amnesia flags observed via `AdminStatusView`. 

* `GET /api/nodes/{id}/status` → `AdminStatusView`:

  ```json
  {
    "profile": "macronode",
    "version": "0.1.0",
    "planes": [
      {
        "name": "gateway",
        "health": "ok",
        "ready": true,
        "restartCount": 0,
        "notes": null
      }
    ],
    "amnesia": false
  }
  ```

  The mapping from `/readyz` + `/api/v1/status` payloads into this normalized view is part of the **contract** and covered by tests. 

* `GET /api/nodes/{id}/metrics/summary` and `/metrics/facets`:

  * Both return *short-window* summaries only, not raw Prometheus text or time-series DBs.
  * `summary` focuses on per-plane health (RPS, error rates, p95s).
  * `facets` groups metrics by facet label or `ron_facet_*` prefix. 

Exactly serialization details will be documented in `docs/openapi/svc-admin.yaml` once the paths are implemented; this `API.md` is the human-readable overview.

---

## 2. SemVer Discipline

### 2.1 Additive (Minor / Non-Breaking)

For `svc-admin`, the following are considered **additive, non-breaking** changes (minor version bump):

* Adding new **HTTP endpoints**, as long as existing endpoints’ semantics are unchanged.
* Adding new **optional JSON fields** to existing responses (must be `Option<>` / nullable in DTOs).
* Adding new **DTO types** or new variants to `Error` marked as `#[non_exhaustive]`.
* Adding new **metrics** or labels (as long as existing labels remain and semantics don’t invert).
* Adding optional CLI flags or env vars with safe defaults.

### 2.2 Breaking (Major)

Changes that require a **major** version bump:

* Removing / renaming HTTP endpoints or changing method/route (e.g., `GET` → `POST`).
* Changing the **meaning** or required fields of a JSON response in a way that would break existing clients:

  * renaming keys,
  * changing types (e.g., `string` → `number`),
  * dropping fields without a deprecation path.
* Changing `auth.mode` semantics or default behavior in a way that would surprise operators (e.g., making `passport` mandatory without a backward-compatible path). 
* Changing how **config keys** behave (e.g., `ui.read_only` defaulting to `false` instead of `true`). 
* Removing or heavily altering key Rust types like `AdminStatusView`, `PlaneStatus`, or `Config` in a way that breaks downstream consumers.

### 2.3 Patch-Level

Patch releases (e.g., `0.1.1`) may include:

* Doc-only fixes and comments.
* Internal implementation refactors, performance improvements, and bugfixes that **do not** change:

  * HTTP endpoints,
  * JSON schemas,
  * CLI flags or env var names,
  * public Rust types / signatures.
* New metrics that are clearly optional / “extra” and don’t alter existing charts.

---

## 3. Stability Guarantees

* **MSRV:** `1.80.0` (Tokio/loom-compatible), as per crate header.
* **HTTP surface:** once the crate reaches `0.1.0` and the SPA is wired, we treat the HTTP/JSON contract as **“stable but pre-1.0”**:

  * we avoid breaking changes without good reason,
  * if we must break something, we:

    * bump minor (`0.x` → `0.(x+1)`),
    * document it explicitly in `CHANGELOG.md` and this file.
* **Rust library surface:** is **secondary**; only the types listed in §1.1 are considered “public API” for SemVer; everything else is subject to change.
* **Unsafe code:** forbidden, consistent with `IDB.md` (`Zero unsafe in Rust backend`). 
* Internal types (e.g., `reqwest::Client`, auth caches, metrics stores) MUST NOT leak into public signatures unless intentionally part of the contract.

---

## 4. Invariants

* **Role alignment:**

  * `svc-admin` is an **admin-plane GUI + proxy**, not an orchestrator or data plane. 
  * API surface MUST reflect that: mostly read-only views; mutating actions are explicit and heavily gated. 

* **Admin-plane only:**

  * All node interactions go through documented admin endpoints:

    * `/healthz`, `/readyz`, `/version`, `/metrics`, `/api/v1/status`, and future `/api/v1/reload` / `/api/v1/shutdown`. 
  * No bespoke or hidden channels; this shapes the Rust and HTTP API surface.

* **Truthful readiness:**

  * `AdminStatusView` and `/api/nodes/{id}/status` MUST never mark a node as “ready” when its `/readyz` is negative. 

* **Short-horizon metrics:**

  * `svc-admin` must not grow its own TSDB; metrics endpoints expose only short, in-memory windows derived from `/metrics`. 

* **Auth / policy first:**

  * Any non-read-only admin action goes through node-level auth and is fully documented in API / logs. 

* **No remote shell:**

  * No API (Rust or HTTP) may expose anything that maps to local or remote shell execution. Actions must translate to **HTTP requests** only, consistent with “Visual runbook over curl.” 

* **Non-orchestration:**

  * No endpoint should imply global cluster orchestration (e.g., “rebalance cluster”, “move shards”). That belongs to separate control-plane services. 

---

## 5. Tooling

To keep the API surface honest and traceable:

* `cargo public-api -p svc-admin --simplified`

  * run in CI,
  * snapshot stored under `docs/api-history/svc-admin/{version}.txt`.
* `cargo semver-checks -p svc-admin` (optional, once integrated)

  * ensures we don’t accidentally make breaking changes while claiming minor/patch bumps.
* `cargo doc -p svc-admin --all-features`

  * all public items must be documented (`#![deny(missing_docs)]`).
* `docs/openapi/svc-admin.yaml`

  * machine-readable OpenAPI spec for HTTP surface, generated/maintained from DTOs and routes.

Snapshots should be updated **only** when:

* a deliberate API change is made, and
* `CHANGELOG.md` is updated accordingly.

---

## 6. CI & Gates

The following gates apply to `svc-admin`:

* **Public API diff check:**

  * CI step runs:

    ```bash
    cargo public-api -p svc-admin --simplified --deny-changes
    ```

  * Any diff must be:

    * explicitly acknowledged in the PR,
    * accompanied by a `CHANGELOG.md` entry,
    * and, if breaking, accompanied by a version bump.

* **HTTP surface test coverage:**

  * Integration tests (including the fake-node harness) must assert:

    * endpoints exist (`/api/ui-config`, `/api/me`, `/api/nodes/...`),
    * JSON shapes can be deserialized into the published DTOs,
    * readiness semantics are truthful and stable. 

* **Config / security gates:**

  * For changes touching `auth.mode`, TLS config, or node credential handling, CI must ensure:

    * security tests in `SECURITY.MD` are still green, including fuzzing and auth flows. 

* **Workspace-level Perfection Gates:**

  * `svc-admin` participates in global gates:

    * no undocumented public items,
    * no unsafe usage,
    * observability endpoints (`/metrics`, `/healthz`, `/readyz`) exposed and documented.

---

## 7. Acceptance Checklist (DoD) for API Changes

For any PR that changes `svc-admin`’s API (Rust or HTTP):

* [ ] `cargo public-api -p svc-admin` run locally; snapshot updated if needed.
* [ ] If public surface changed:

  * [ ] `CHANGELOG.md` includes a clear description (what changed, why).
  * [ ] `docs/openapi/svc-admin.yaml` updated (for HTTP changes).
  * [ ] This `API.md` updated if semantics or contracts changed.
* [ ] New/changed endpoints have tests (unit + integration) and, where applicable, fake-node harness coverage.
* [ ] For breaking changes:

  * [ ] Version bumped appropriately (major for true breaks).
  * [ ] Migration steps documented (if needed).

---

## 8. Appendix

### 8.1 References

* Rust SemVer guidelines: [https://doc.rust-lang.org/cargo/reference/semver.html](https://doc.rust-lang.org/cargo/reference/semver.html)
* `cargo-public-api`: [https://github.com/Enselic/cargo-public-api](https://github.com/Enselic/cargo-public-api)
* `cargo-semver-checks`: [https://github.com/obi1kenobi/cargo-semver-checks](https://github.com/obi1kenobi/cargo-semver-checks)
* svc-admin IDB: high-level invariants and DTO definitions. 
* svc-admin CONFIG: configuration API keys and env vars. 
* svc-admin SECURITY: auth modes and health gates. 

### 8.2 Perfection Gates tie-in

* **Gate G:** No undocumented API surface
  → enforced via `#![deny(missing_docs)]` and doc coverage in `cargo doc`.
* **Gate H:** Breaking changes require major version bump
  → enforced by human review + `cargo public-api` diffs + version policy.
* **Gate J:** CHANGELOG alignment
  → any detected `public-api` diff without matching `CHANGELOG` entry fails CI.

### 8.3 History

* **0.1.0 (planned):**

  * Initial stable HTTP surface:

    * `/`, `/assets/*`
    * `/healthz`, `/readyz`, `/metrics`
    * `/api/ui-config`, `/api/me`, `/api/nodes`, `/api/nodes/{id}/status`, `/api/nodes/{id}/metrics/*`
  * Rust DTOs: `AdminStatusView`, `PlaneStatus`, `UiConfigDto`, `MeResponse`, `NodeSummary`, `MetricsSummaryDto`, `FacetMetricsDto`.
  * Config surface as per `svc-admin.toml` example. 

(As we implement `svc-admin`, update this history section with concrete versions and links to the corresponding API snapshots.)

```
```
