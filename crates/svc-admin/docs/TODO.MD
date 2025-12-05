

---

## 1) Scaffolding plan (high-level)

**Phase 0 — Crate shell + docs**

* Add `svc-admin` crate under `crates/` with `Cargo.toml`, `README.md`, `CHANGELOG.md`, `build.rs`.
* Wire `lib.rs` to expose `config`, `dto`, `server`, `error` exactly as defined in `API.MD`.
* Keep docs under `crates/svc-admin/docs` in sync with the blueprints (already there via `ALL_DOCS.md`).

**Phase 1 — Core backend modules**

* Implement `config.rs` to match `CONFIG.MD` and surface `Config`, `UiCfg`, `AuthCfg`, `NodeCfg`.
* Implement `dto/*` for `AdminStatusView`, `PlaneStatus`, `UiConfigDto`, `MeResponse`, `NodeSummary`, facet-aware metrics DTOs.
* Implement `error.rs` as the canonical error taxonomy from `API.MD` / `SECURITY.MD`.
* Implement `server.rs`, `router.rs`, `state.rs` as the Axum server bootstrap, router, and shared app state.

**Phase 2 — Node admin client + metrics sampling**

* Implement `nodes/*` to talk to node admin endpoints (`/healthz`, `/readyz`, `/version`, `/metrics`, `/api/v1/status`) and normalize them into DTOs.
* Implement `metrics/*` to:

  * sample node `/metrics`,
  * parse Prometheus exposition,
  * group metrics by facet using `ron_facet_*` and labels (P-16),
  * expose short-lived rolling windows for the UI.

**Phase 3 — Auth, observability, concurrency discipline**

* Implement `auth/*` for `none`, `ingress`, and `passport` modes (P-17).
* Implement `observability.rs` for `tracing`, Prometheus metrics export, and log wiring aligned with `OBSERVABILITY.MD`.
* Implement `interop.rs` for fake-node harnesses and SDK interop tests (`INTEROP.MD`).
* Ensure concurrency and shutdown behavior match `CONCURRENCY.MD`.

**Phase 4 — React SPA skeleton (God-tier DX & GUI)**

* Scaffold `ui/` as a Vite + React + TS app:

  * `/src/routes` for pages (overview, node detail, settings, login).
  * `/src/components` for reusable layout, node cards, facet metrics panels.
  * `/src/i18n` for multi-language support (P-14, P-15).
  * `/src/theme` for theme tokens + theme provider (P-15).
  * `/src/templates` for modular “dashboard templates” the dev can extend (future customization story).
* Use `/src/api` to:

  * talk to `svc-admin`’s `/api/*`,
  * integrate `ron-app-sdk-ts` for the “app-plane playground” area later.

**Phase 5 — UI ↔ backend integration**

* Wire SPA to backend DTOs and `/api/*` shapes from `API.MD`.
* Implement facet-aware metrics views (`FacetMetricsPanel`, `FacetMetricsTemplate`).
* Wire UI config (`/api/ui-config`, `UiConfigDto`) to theme + language switchers and read-only mode.
* Add smoke tests (`tests/http_smoke.rs`) and fake-node tests (`tests/fake_node.rs`) around end-to-end flows.

---

## 2) Proposed file tree (`crates/svc-admin`)

```text
crates/
  svc-admin/
    Cargo.toml
    README.md
    CHANGELOG.md
    build.rs
    .gitignore

    src/
      lib.rs
      cli.rs
      error.rs
      config.rs
      server.rs
      router.rs
      state.rs
      observability.rs
      interop.rs

      auth/
        mod.rs
        none.rs
        ingress.rs
        passport.rs

      nodes/
        mod.rs
        registry.rs
        client.rs
        status.rs

      metrics/
        mod.rs
        sampler.rs
        facet.rs
        prometheus_bridge.rs

      dto/
        mod.rs
        ui.rs
        me.rs
        node.rs
        metrics.rs

      bin/
        svc-admin.rs

    docs/
      API.MD
      CONCURRENCY.MD
      CONFIG.MD
      GOVERNANCE.MD
      IDB.md
      INTEROP.MD
      OBSERVABILITY.MD
      PERFORMANCE.MD
      RUNBOOK.MD
      SECURITY.MD
      TESTS.MD

    ui/
      package.json
      tsconfig.json
      vite.config.ts
      .eslintrc.cjs
      .prettierrc
      index.html

      public/
        favicon.svg
        robots.txt
        locales/
          en-US.json
          es-ES.json

      src/
        main.tsx
        App.tsx

        routes/
          index.tsx
          NodeListPage.tsx
          NodeDetailPage.tsx
          SettingsPage.tsx
          LoginPage.tsx
          NotFoundPage.tsx

        components/
          layout/
            Shell.tsx
            Sidebar.tsx
            TopBar.tsx
            ThemeToggle.tsx
            LanguageSwitcher.tsx
          nodes/
            NodeCard.tsx
            NodeStatusBadge.tsx
            PlaneStatusTable.tsx
          metrics/
            MetricChart.tsx
            FacetMetricsPanel.tsx
          shared/
            LoadingSpinner.tsx
            ErrorBanner.tsx
            EmptyState.tsx

        api/
          adminClient.ts
          ronCorePlaygroundClient.ts

        i18n/
          index.ts
          useI18n.ts

        theme/
          ThemeProvider.tsx
          tokens.ts
          themes.ts

        templates/
          index.ts
          NodeOverviewTemplate.tsx
          FacetMetricsTemplate.tsx
          CustomTemplateRegistry.tsx

        types/
          admin-api.ts

    static/
      README.md

    scripts/
      build-ui.sh
      dev-ui.sh
      lint-ui.sh
      sync-ui-assets.sh

    tests/
      http_smoke.rs
      fake_node.rs
```

---

## 3) File-by-file descriptions

### Top-level (crate root)

* **`Cargo.toml`** – Crate manifest: declares `svc-admin` as a service crate, pins workspace deps (axum, tokio, reqwest, prometheus, tokio-rustls), and enables features like `tls`, `otel`, `passport`.
* **`README.md`** – Human-facing overview you just had me generate: role, invariants, SLOs, interfaces, and roadmap for `svc-admin`.
* **`CHANGELOG.md`** – SemVer-aligned release history documenting API/config/auth changes as required by `API.MD` and `GOVERNANCE.MD`.
* **`build.rs`** – Build script that bundles the compiled SPA (from `ui/dist`) into Rust static assets (e.g., via `include_dir`), so `svc-admin` ships as a single self-contained binary.
* **`.gitignore`** – Ignores `target/`, UI build artifacts (`ui/node_modules`, `ui/dist`), generated static assets, and local configs/logs.

### `src/` core modules

* **`src/lib.rs`** – Crate root; re-exports `config`, `dto`, `server`, and `error` as the small public API snapshot described in `API.MD`, keeping everything else crate-internal.
* **`src/cli.rs`** – Parses CLI arguments (`--config`, `--bind`, `--metrics-bind`, `--log-level`) and merges them with env vars and config file to produce an effective `Config` (aligns with `CONFIG.MD`).
* **`src/error.rs`** – Houses the unified `Error` enum and `Result<T>` alias covering `ConfigError`, `Io`, `Http`, `AuthError`, and `UpstreamError` as specified in the API surface doc.
* **`src/config.rs`** – Implements the Rust config types (`Config`, `UiCfg`, `AuthCfg`, `NodeCfg`, etc.) and loading logic (env + file + CLI) in line with `CONFIG.MD` and `SECURITY.MD`.
* **`src/server.rs`** – Builds shared `AppState`, calls `router::build_router`, starts the Axum HTTP server(s), and wires graceful shutdown/readiness tracking from `CONCURRENCY.MD`.
* **`src/router.rs`** – Defines the Axum router: routes `/`, `/assets/*`, `/api/*`, `/healthz`, `/readyz`, `/metrics` to the appropriate handlers; also wires middlewares (tracing, auth, timeouts).
* **`src/state.rs`** – Defines `AppState` (Arc-wrapped shared state: config, node registry, metrics buffers, auth caches) and helper methods to keep concurrency disciplined (no locks across `.await`).
* **`src/observability.rs`** – Sets up `tracing` subscribers, log format, and Prometheus metric registration matching the canonical metrics listed in `OBSERVABILITY.MD`.
* **`src/interop.rs`** – Helper utilities and types for integration tests / interop harnesses (e.g., fake nodes, harness config) as outlined in `INTEROP.MD`.

### `src/auth/` (auth modes)

* **`src/auth/mod.rs`** – Public facade for auth: exposes traits/functions for `AuthMode`, identity extraction (`MeResponse` source), and role checks used by handlers.
* **`src/auth/none.rs`** – Implements `auth.mode="none"`: generates synthetic dev identities suitable for local/testing, clearly tagged as non-production in logs.
* **`src/auth/ingress.rs`** – Implements `auth.mode="ingress"`: trusts HTTP headers from ingress or reverse proxies (e.g., `X-User`, `X-Roles`) while validating minimal invariants and threat constraints.
* **`src/auth/passport.rs`** – Implements `auth.mode="passport"`: validates passports/JWTs, caches JWKS, extracts roles/subject, and maps them to `MeResponse` while honoring `SECURITY.MD` constraints.

### `src/nodes/` (node admin-plane access)

* **`src/nodes/mod.rs`** – Public entrypoint for node operations: helper functions to fetch `AdminStatusView`, node summaries, and metrics for the HTTP handlers.
* **`src/nodes/registry.rs`** – Manages an in-memory registry of nodes based on config: logical IDs, admin URLs, profiles, environments, and credential references.
* **`src/nodes/client.rs`** – Wraps `reqwest::Client` and performs HTTP calls to node admin endpoints (`/healthz`, `/readyz`, `/version`, `/metrics`, `/api/v1/status`) with observability and timeout controls.
* **`src/nodes/status.rs`** – Normalizes raw node responses into `AdminStatusView` and `PlaneStatus` while enforcing the “truthful readiness” invariant (`/readyz` is the source of truth).

### `src/metrics/` (sampling + facet metrics)

* **`src/metrics/mod.rs`** – Central metrics module that coordinates samplers, in-memory buffers, and DTO conversion for HTTP handlers and Prometheus export.
* **`src/metrics/sampler.rs`** – Background tasks that periodically scrape node `/metrics` and `/api/v1/status`, keeping rolling windows in memory without becoming a time-series DB.
* **`src/metrics/facet.rs`** – Logic for grouping metrics by facet (P-16): parses Prometheus metrics by labels/prefixes (e.g., `ron_facet_<name>*`) and computes facet-level summaries for the GUI.
* **`src/metrics/prometheus_bridge.rs`** – Bridges between scraped metrics and exported Prometheus metrics for `svc-admin` itself (`svc_admin_node_fanout_*`, `svc_admin_admin_actions_total`, etc.).

### `src/dto/` (data transfer objects)

* **`src/dto/mod.rs`** – Re-exports DTO types used in the public API and HTTP handlers; ensures consistent `serde` config and versioning discipline from `API.MD`.
* **`src/dto/ui.rs`** – Implements `UiConfigDto` (`default_theme`, `available_themes`, `default_language`, `available_languages`, `read_only`) for `/api/ui-config`, aligning with P-14/P-15.
* **`src/dto/me.rs`** – Implements `MeResponse` (`subject`, `display_name`, `roles`, `auth_mode`, `login_url`) for `/api/me`, drawing from the active auth mode implementation.
* **`src/dto/node.rs`** – Implements `NodeSummary`, `AdminStatusView`, `PlaneStatus` and related DTOs exposed via `/api/nodes` and `/api/nodes/{id}/status`.
* **`src/dto/metrics.rs`** – DTOs for short-window metrics summaries and facet-aware metrics (e.g., `FacetMetricsSummary`, `PlaneMetricsSummary`) consumed by the SPA.

### `src/bin/` (binary entrypoint)

* **`src/bin/svc-admin.rs`** – Thin CLI wrapper that parses args (`cli::parse`), loads config, initializes logging/metrics, and calls `server::run(config)` per `API.MD`’s guidance.

### `docs/` (IDB & blueprints)

* **`docs/API.MD`** – API surface & SemVer reference for `svc-admin`: canonical public modules, DTOs, HTTP endpoints, and breaking-change rules (already drafted in `ALL_DOCS.md`).
* **`docs/CONCURRENCY.MD`** – Specifies task model, shared state discipline, timeouts, and shutdown ordering for `svc-admin`.
* **`docs/CONFIG.MD`** – Documents config schema (`Config`, environment variables, defaults) and how CLI/env/config interact.
* **`docs/GOVERNANCE.MD`** – Governance & economic integrity for admin-plane behavior (bounded authority, appeal paths, mutating action gates).
* **`docs/IDB.md`** – Invariant-Driven Blueprint: core invariants, role alignment, and how `svc-admin` stays admin-plane only.
* **`docs/INTEROP.MD`** – Interop expectations for SDKs, test harnesses, and external tools that consume `svc-admin`’s API.
* **`docs/OBSERVABILITY.MD`** – Canonical metrics, tracing strategy, sampling, and dashboard recommendations for `svc-admin`.
* **`docs/PERFORMANCE.MD`** – Performance targets, bench methodology, and measurement notes (latency targets, node counts, fan-out behavior).
* **`docs/RUNBOOK.MD`** – Operational runbook for on-call: how to deploy, debug, mitigate, and roll back `svc-admin`.
* **`docs/SECURITY.MD`** – Threat model, auth modes, key handling, and hardening practices specific to `svc-admin`.
* **`docs/TESTS.MD`** – Testing strategy: unit, integration, interop, chaos, and loom tests as they apply to `svc-admin`.

### `ui/` root (SPA project)

* **`ui/package.json`** – Node package manifest: defines dependencies (`react`, `react-dom`, `vite`, `ron-app-sdk-ts`, testing libs) and scripts (`dev`, `build`, `test`, `lint`).
* **`ui/tsconfig.json`** – TypeScript configuration for the SPA, ensuring strict type checking and path aliases (e.g., `@/components`, `@/api`).
* **`ui/vite.config.ts`** – Vite config: dev server setup, proxy to `svc-admin` backend in dev, and optimized build pipeline for `ui/dist`.
* **`ui/.eslintrc.cjs`** – ESLint configuration enforcing consistent code style and catching common issues (DX guardrail).
* **`ui/.prettierrc`** – Prettier configuration for automated code formatting across the SPA.
* **`ui/index.html`** – HTML entrypoint used by Vite; contains the root `<div>` and references to built JS/CSS bundles.

### `ui/public/` (static assets)

* **`ui/public/favicon.svg`** – Default favicon for the SPA; can be themed to match RON branding.
* **`ui/public/robots.txt`** – Robots config (typically minimal, but included for completeness if `svc-admin` is ever reachable via HTTP from outside).
* **`ui/public/locales/en-US.json`** – Default English language pack: key-based strings used by components (`t("overview.title")` etc.), matching the P-14 requirement.
* **`ui/public/locales/es-ES.json`** – Example Spanish (or alt) language pack to validate multi-language pipeline and serve as template for more locales.

### `ui/src/` core SPA files

* **`ui/src/main.tsx`** – SPA entrypoint: mounts React into the DOM, wires `ThemeProvider`, `I18nProvider`, and React router.
* **`ui/src/App.tsx`** – Top-level app component: defines high-level layout and routes container; picks up UI config from `/api/ui-config`.

### `ui/src/routes/` (page-level views)

* **`ui/src/routes/index.tsx`** – Route table wiring React Router (or similar) to the individual page components.
* **`ui/src/routes/NodeListPage.tsx`** – “Nodes overview” page: lists nodes using `NodeSummary`, with status badges and high-level metrics.
* **`ui/src/routes/NodeDetailPage.tsx`** – Node detail page: shows `AdminStatusView`, per-plane status, facet metrics tab (P-16), and time-windowed charts.
* **`ui/src/routes/SettingsPage.tsx`** – Settings GUI: allows operators to tweak local preferences (theme, language) and view effective backend `UiConfigDto`.
* **`ui/src/routes/LoginPage.tsx`** – Login view (when `auth.mode="passport"`): surfaces `loginUrl` from `/api/me` and handles redirect flow.
* **`ui/src/routes/NotFoundPage.tsx`** – Fallback route for unknown paths, keeping SPA behavior graceful.

### `ui/src/components/layout/`

* **`ui/src/components/layout/Shell.tsx`** – Overall layout shell (sidebar + top bar + content), used by all main pages; central place to apply themes and language context.
* **`ui/src/components/layout/Sidebar.tsx`** – Left-hand navigation with links to `Nodes`, `Facets`, `Settings`, and future templates.
* **`ui/src/components/layout/TopBar.tsx`** – Top bar showing environment labels, current user (`/api/me`), and quick actions.
* **`ui/src/components/layout/ThemeToggle.tsx`** – Control that lets users toggle between `system`, `light`, `dark`, and custom themes, wired into `ThemeProvider`.
* **`ui/src/components/layout/LanguageSwitcher.tsx`** – UI control for switching locales; integrates with `i18n` context and `UiConfigDto.available_languages`.

### `ui/src/components/nodes/`

* **`ui/src/components/nodes/NodeCard.tsx`** – Compact card representation of a node, used in list views and dashboards.
* **`ui/src/components/nodes/NodeStatusBadge.tsx`** – Small badge summarizing node health/readiness for quick scanning (e.g., green/amber/red).
* **`ui/src/components/nodes/PlaneStatusTable.tsx`** – Table/grid showing `PlaneStatus` rows (gateway, overlay, storage, facets) for a single node.

### `ui/src/components/metrics/`

* **`ui/src/components/metrics/MetricChart.tsx`** – Generic chart component (wrapping a charting lib) to visualize numeric metrics over time (RPS, error rate, latency).
* **`ui/src/components/metrics/FacetMetricsPanel.tsx`** – Facet-aware metrics view: groups metrics by facet and displays charts/tables tailored to `facet.rs` DTOs (P-16).

### `ui/src/components/shared/`

* **`ui/src/components/shared/LoadingSpinner.tsx`** – Reusable loading indicator for async states (DX: standard skeleton for all pages).
* **`ui/src/components/shared/ErrorBanner.tsx`** – Consistent error presentation for network/config/auth issues.
* **`ui/src/components/shared/EmptyState.tsx`** – “Nothing here yet”/“No nodes configured” UX; used to keep the GUI friendly in edge cases.

### `ui/src/api/`

* **`ui/src/api/adminClient.ts`** – Thin client for calling `svc-admin`’s `/api/*` endpoints, with typed responses from `types/admin-api.ts` and shared error handling.
* **`ui/src/api/ronCorePlaygroundClient.ts`** – Integration point for `ron-app-sdk-ts` to support a future “App Plane Playground” area within the admin UI (e.g., try app requests live).

### `ui/src/i18n/`

* **`ui/src/i18n/index.ts`** – Sets up the i18n context/provider, loads locale JSON files, and exposes a `t()` function to components.
* **`ui/src/i18n/useI18n.ts`** – React hook that gives components access to current locale, translations, and helpers for language switching.

### `ui/src/theme/`

* **`ui/src/theme/ThemeProvider.tsx`** – React context provider handling light/dark/system/custom themes; syncs with `UiConfigDto` and user preferences (e.g., `localStorage`).
* **`ui/src/theme/tokens.ts`** – Design tokens (colors, spacing, typography) that can be reused across themes and templates for consistent styling.
* **`ui/src/theme/themes.ts`** – Concrete theme objects (`light`, `dark`, `red-eye`, etc.) mapping tokens to CSS variables or component styles.

### `ui/src/templates/` (template system for future extensibility)

* **`ui/src/templates/index.ts`** – Entry point and registry for all UI templates; defines how templates are registered and discovered.
* **`ui/src/templates/NodeOverviewTemplate.tsx`** – Default “node overview” dashboard template using core components (cards, charts) and intended as the baseline.
* **`ui/src/templates/FacetMetricsTemplate.tsx`** – Template focused on facet metrics, grouping nodes and planes around facets and their metrics.
* **`ui/src/templates/CustomTemplateRegistry.tsx`** – Mechanism that allows future devs to register custom templates (e.g., via config or build-time extension) without changing core UI code.

### `ui/src/types/`

* **`ui/src/types/admin-api.ts`** – TypeScript definitions mirroring `svc-admin`’s DTOs (`UiConfigDto`, `MeResponse`, `NodeSummary`, `AdminStatusView`, facet metrics DTOs), acting as the contract between backend and SPA.

### `static/`

* **`static/README.md`** – Explains how built SPA assets are placed here (or embedded) and how `build.rs` consumes them; clarifies that files in this directory are generated and not hand-edited.

### `scripts/` (DX / dev tooling)

* **`scripts/build-ui.sh`** – Script to `cd ui`, run `npm install` (first time) and `npm run build`, then copy `ui/dist` into `static/` for embedding.
* **`scripts/dev-ui.sh`** – Starts the Vite dev server and prints instructions for running `svc-admin` in parallel with proxying (DX: one command to get the GUI live).
* **`scripts/lint-ui.sh`** – Runs `npm run lint` and potentially TypeScript checks for the SPA.
* **`scripts/sync-ui-assets.sh`** – Syncs `ui/dist` to `static/` and (optionally) runs `cargo build -p svc-admin` after a successful UI build.

### `tests/` (Rust tests)

* **`tests/http_smoke.rs`** – End-to-end smoke tests: boot a `svc-admin` instance with fake config and assert that `/healthz`, `/readyz`, `/api/ui-config`, `/api/nodes` behave as documented.
* **`tests/fake_node.rs`** – Integration tests using fake nodes that implement the admin endpoints, verifying normalization logic (`AdminStatusView`, facet metrics grouping) and resilience to malformed data.

---

This scaffolding gives you:

* A **tight, invariant-driven Rust backend** that exactly matches the blueprints in `ALL_DOCS.md`.
* A **React + TS SPA** that is ready for multi-language, multi-theme, and facet metrics from day zero.
* A **template system** in the UI so future devs can plug in custom dashboards without rewriting the core.
* A **DX-friendly dev loop** via `scripts/` and clean separation between API types and UI code.
