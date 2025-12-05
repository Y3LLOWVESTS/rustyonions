

````markdown
---
title: svc-admin (Admin Plane GUI)
version: 0.1.0
status: draft
last-updated: 2025-12-04
audience: contributors, ops, auditors
---

# svc-admin (Admin Plane GUI)

## 1. Invariants (MUST)

- [I-1] **Admin-plane only.**  
  svc-admin MUST interact with nodes *only* via existing HTTP admin surfaces:
  - `/healthz`
  - `/readyz`
  - `/version`
  - `/metrics`
  - `/api/v1/status`
  - and explicitly defined admin endpoints (e.g., `/api/v1/reload`, `/api/v1/shutdown`, `/api/v1/selftest`) when they exist.  
  It MUST NOT invent new backdoor protocols or privileged channels.

- [I-2] **Truthful readiness.**  
  svc-admin MUST never claim a node or plane is “ready” if that node’s `/readyz` reports “not ready” or degraded.  
  All UI status badges and colors MUST be derived from node-reported health/readiness, not inferred or overridden.

- [I-3] **No hidden state about nodes.**  
  All operational state shown in the UI MUST be either:
  - live from node endpoints, or
  - from explicit svc-admin config (e.g., a local TOML with labels, auth, env tags).  
  svc-admin MUST NOT maintain its own shadow model of node health, config, or topology that can diverge from the nodes.

- [I-4] **Policy-first for all actions.**  
  Any action that can change node state (reload, shutdown, future drain, etc.) MUST:
  - go through the node’s authenticated admin endpoint, and
  - be fully subject to that node’s security and policy system (macaroons, `ron-policy`, TLS, etc.).  
  svc-admin MUST NOT bypass or weaken node-level authz.

- [I-5] **Read-only by default.**  
  The default operating mode of svc-admin MUST be read-only:
  - No state-changing buttons are active unless an operator explicitly configures allowed actions and credentials for the target node(s).
  - In the absence of explicit configuration, all mutation controls MUST be disabled or hidden.

- [I-6] **Amnesia-aware and amnesia-safe.**  
  When a node is running in an “amnesia” or ephemeral mode, svc-admin MUST:
  - surface this clearly in the UI, and
  - avoid storing sensitive node state persistently on disk by default.  
  Amnesia flags reported by nodes MUST be treated as truth; svc-admin MUST NOT assume persistence when a node says it is ephemeral.

- [I-7] **Profile-agnostic, profile-accurate.**  
  svc-admin MUST work against both **macronode** and **micronode** profiles using the same UI shell:
  - It MUST discover capabilities via `/version` and `/api/v1/status`.
  - It MUST only show planes/services that actually exist on that node (e.g., no “overlay” tile on a profile that doesn’t expose it).

- [I-8] **No remote shell or arbitrary code execution.**  
  svc-admin MUST NOT execute arbitrary shell commands, scripts, or binaries on behalf of the operator (locally or remotely).  
  “Runbook actions” MUST map to **documented HTTP endpoints** only, not to shell invocations.

- [I-9] **Short-horizon metrics only.**  
  svc-admin’s internal metrics visualization MUST be based on a short, in-memory window (e.g., last N samples) collected from `/metrics`.  
  It MUST NOT implement its own long-lived time-series database or silently store large historical datasets by default.

- [I-10] **No orchestration role.**  
  svc-admin MUST NOT assume responsibility for scheduling, load balancing, or cluster-level orchestration.  
  It is an *administration and observability* panel, not a control plane.

- [I-11] **Zero unsafe in Rust backend.**  
  The Rust (Axum) backend for svc-admin MUST NOT rely on `unsafe` code. Any exceptional case requiring `unsafe` MUST be treated as a design bug.

- [I-12] **Deterministic mapping from admin data → UI.**  
  For each tile, badge, and status color, there MUST be a deterministic mapping from admin-plane data (health, ready, metrics, restart counters, etc.).  
  This mapping MUST be documented and testable (no “magic heuristics”).

- [I-13] **Least-privilege configuration.**  
  Node credentials (tokens, macaroons, etc.) configured in svc-admin MUST be scoped to the minimal set of admin actions required (view vs control).  
  svc-admin MUST NOT assume cluster-wide god tokens by default.

- [I-14] **Auditable admin actions.**  
  Any non-read-only admin action triggered via svc-admin MUST be:
  - visible in svc-admin’s own logs / event stream, and
  - designed to integrate with `ron-audit` or an equivalent audit service, so that an external auditor can reconstruct who did what, when.

- [I-15] **Operator auth via trusted identity (svc-passport / ingress).**  
  In non-dev environments, svc-admin MUST only serve its SPA and JSON APIs to authenticated operators whose identity and roles are derived from a **trusted identity source**:
  - svc-passport (preferred), or
  - an ingress/gateway that itself uses svc-passport or an equivalent IdP.  
  svc-admin MUST NOT implement its own username/password database or bespoke password flows; it MUST rely on signed tokens/claims from the identity layer.

---

## 2. Design Principles (SHOULD)

- [P-1] **“Visual runbook over curl.”**  
  Every action and insight in svc-admin SHOULD be explainable as a small set of `curl` commands against the node’s admin plane.  
  The UI SHOULD feel like a polished, annotated version of the node’s README + smoke script.

- [P-2] **3 a.m. incident-first UX.**  
  The primary design target is an operator on-call at 3 a.m.:
  - The first screen SHOULD answer, within seconds: “Is this node healthy?” and “If not, which plane is failing and why?”
  - Navigation SHOULD be shallow (1–2 clicks to the failing plane’s detail view).

- [P-3] **Single-node focus first, fleet later.**  
  The first iterations of svc-admin SHOULD focus on deeply understanding one node at a time.  
  Multi-node / cluster dashboards SHOULD only appear after single-node views are stable, truthful, and widely used.

- [P-4] **Profile-sensitive, layout-stable.**  
  The overall layout (header, nav, core sections) SHOULD remain consistent between macronode and micronode.  
  Differences SHOULD appear as:
  - additional or missing tiles,
  - extra tabs/sections on detail views,  
  not as entirely different UX flows.

- [P-5] **Explainability over cleverness.**  
  For every red/yellow badge, svc-admin SHOULD be able to explain the reasoning in human language (e.g., “Ready=FALSE because overlay plane health=FAIL and last check timed out”).  
  Tooltips and side panels SHOULD prefer concrete cause chains over vague labels.

- [P-6] **Prometheus and Grafana coexistence.**  
  svc-admin SHOULD coexist smoothly with Prometheus/Grafana:
  - Use plain text Prometheus metrics from `/metrics`.
  - Where appropriate, provide links or canned Grafana queries/dashboards rather than re-implementing full TSDB features.

- [P-7] **Static SPA + Axum proxy.**  
  svc-admin SHOULD be built as:
  - a React + TypeScript single-page app (SPA), bundled with Vite, and
  - served by an Axum backend which also proxies admin requests to nodes.  
  Static assets SHOULD be embedded in the binary to ease deployment.

- [P-8] **Strong typing, thin layers.**  
  The Rust backend SHOULD:
  - define strongly typed DTOs for `/api/v1/status`, readiness, and metrics projections, and
  - keep the proxy and mapping logic thin and testable, without heavy frameworks.

- [P-9] **Graceful degradation.**  
  When a node is unreachable, misconfigured, or partially broken:
  - svc-admin SHOULD present clear error states (e.g., “Cannot reach /metrics: TLS handshake failed”).
  - It SHOULD still render what information it *can* fetch, rather than failing the entire page.

- [P-10] **Environment-aware UX (config-driven).**  
  svc-admin SHOULD treat environments (dev, staging, prod) differently in UX, but **only via explicit configuration** (e.g., `environment = "dev"` and `ui.dev.*` flags in `svc-admin.toml`), never via heuristics on hostnames or URLs:
  - Dev: more experimental tools (KV inspector, facet tester, app-plane playground) visible when explicitly enabled.
  - Prod: fewer mutating controls, more emphasis on read-only observability and auditing.

- [P-11] **Small, composable UI components.**  
  The SPA SHOULD treat planes, metrics charts, and status tiles as composable components:
  - Each component SHOULD be testable independently (Storybook / component tests).
  - The overall dashboard SHOULD assemble those components rather than using a monolithic page.

- [P-12] **Theming and responsive layout.**  
  Given real-world operator usage, svc-admin SHOULD support:
  - **design-token-based themes** (CSS variables) with at least three built-in themes: `"light"`, `"dark"`, and `"red-eye"` (night/low-blue mode),
  - a responsive layout that remains usable on laptops, large monitors, and tablet screens.

- [P-13] **Modular layouts and extension points.**  
  The SPA layout SHOULD be defined in terms of named regions/slots (e.g., `header`, `sidebar`, `overview-main`, `detail-main`, `footer-tools`), and panels SHOULD be pluggable into those slots.  
  This allows future customization and alternative layouts (e.g., custom dashboards) without rewriting the core data layer.

- [P-14] **i18n-ready labeling.**  
  All user-facing text in the SPA SHOULD be referenced via stable message keys and language packs (e.g., `t("overview.title")`), not hard-coded English strings in components.  
  A default locale (e.g., `en-US`) SHOULD be provided, and the design SHOULD anticipate future locale packs.

- [P-15] **Configurable themes and languages.**  
  svc-admin SHOULD expose configuration keys for:
  - default theme (`ui.theme = "system" | "light" | "dark" | "red-eye"`),
  - default language (`ui.language = "en-US"`),  
  while still allowing per-operator preferences stored client-side (e.g., in `localStorage`).

- [P-16] **Facet-aware metrics views.**  
  When nodes expose **facet metrics** via the shared Prometheus registry (e.g., metrics labeled with `facet="<facet_name>"` or prefixed by `ron_facet_*`), svc-admin SHOULD:
  - surface a dedicated **Facets tab/button** in the UI, and
  - group and visualize these metrics by facet, without requiring additional node changes.

- [P-17] **Batteries-included login UX backed by svc-passport.**  
  svc-admin SHOULD present a clear “Sign in” / “Operator login” experience, but all authentication MUST be backed by:
  - svc-passport (preferred) issuing signed passports/tokens, or
  - an ingress that itself relies on svc-passport or an equivalent IdP.  
  The SPA SHOULD interact with auth via a simple `/api/me` + redirect-to-login pattern, not by collecting passwords directly.

---

## 3. Implementation (HOW)

- [C-1] **High-level architecture**

  - `svc-admin` crate (Rust):
    - Uses Axum as the HTTP server.
    - Serves static SPA assets at `/` (embedded using something like `include_dir` or equivalent).
    - Exposes backend API endpoints for the SPA, e.g.:
      - `GET /api/nodes` → list configured nodes (from a local config file or env vars).
      - `GET /api/nodes/{id}/health` → proxy to `<node>/healthz`.
      - `GET /api/nodes/{id}/ready` → proxy to `<node>/readyz`.
      - `GET /api/nodes/{id}/status` → proxy to `<node>/api/v1/status`.
      - `GET /api/nodes/{id}/metrics` → proxy to `<node>/metrics` and optionally parse key metrics for quick charts.
      - `GET /api/nodes/{id}/metrics/summary` → short-horizon, pre-processed view for charts.
      - `GET /api/nodes/{id}/metrics/facets` → facet-grouped metrics view.
      - `GET /api/ui-config` → UI config (theme, languages, feature flags).
      - `GET /api/me` → current operator identity + roles (from svc-passport / ingress).

  - React SPA (TypeScript):
    - Talks only to the `svc-admin` backend (`/api/...`) for admin-plane data and auth (`/api/me`).
    - Optionally uses `ron-app-sdk-ts` for **dev-only app-plane tooling** (e.g., playground console), gated by config.
    - Renders:
      - Node selector + environment/amnesia badges.
      - Overview dashboard (readiness pill, plane tiles).
      - Detail views for each plane.
      - Metrics charts using a light charting library (e.g. Recharts).
      - A **Facets** tab/button for facet metrics when present.
      - A top-right identity widget (`Signed in as …`, theme toggle, language selector).

- [C-2] **Node and UI configuration model**

  Config file (e.g., `svc-admin.toml`):

  ```toml
  # svc-admin.toml

  bind_addr    = "127.0.0.1:5300"
  metrics_addr = "127.0.0.1:0"

  [log]
  format = "json"
  level  = "info"

  [ui]
  read_only = true
  theme     = "system"   # "system" | "light" | "dark" | "red-eye"
  language  = "en-US"

  [ui.dev]
  enable_app_playground = true  # uses ron-app-sdk-ts (dev only)

  [polling]
  metrics_interval = "5s"
  metrics_window   = "5m"

  [auth]
  mode              = "passport"               # "none" | "ingress" | "passport"
  passport_base_url = "https://passport.local" # used in redirects / links
  client_id         = "svc-admin"              # for svc-passport / IdP
  issuer            = "https://passport.local"
  audience          = "svc-admin"

  [nodes.macronode-dev]
  base_url      = "http://127.0.0.1:8090"
  display_name  = "macronode-dev"
  environment   = "dev"
  insecure_http = true
  # macaroon_path = "/etc/ron/macaroon.macronode-dev"

  [nodes.micronode-dev]
  base_url      = "http://127.0.0.1:8091"
  display_name  = "micronode-dev"
  environment   = "dev"
  insecure_http = true
  # macaroon_path = "/etc/ron/macaroon.micronode-dev"
````

Backend pattern:

* Load config once at startup into a typed `Config` struct (see Config blueprint).
* Expose a `NodeConfig` map, `UiCfg`, and `AuthCfg` in Axum state.
* Ensure that *all* outbound node HTTP requests use this config (no ad-hoc URLs).

- [C-3] **Health / readiness / status DTOs**

  In Rust:

  ```rust
  /// Projection of the node's admin status as seen by svc-admin.
  #[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
  #[serde(deny_unknown_fields)]
  pub struct AdminStatusView {
      pub profile: String,          // "macronode" | "micronode" | etc.
      pub version: String,          // node semantic version
      pub planes: Vec<PlaneStatus>, // per-plane health/ready info
      pub amnesia: bool,
  }

  #[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
  #[serde(deny_unknown_fields)]
  pub struct PlaneStatus {
      pub name: String,             // "gateway", "overlay", "kv", "facets", ...
      pub health: String,           // "ok" | "fail" | "unknown"
      pub ready: bool,
      pub restart_count: u64,
      pub notes: Option<String>,    // optional explanatory string from node
  }
  ```

  * Provide mapping code from raw `/api/v1/status` payloads into this normalized view.
  * Maintain 1:1 tests from real or fixture status payloads to `AdminStatusView`.

- [C-4] **Metrics polling and caching**

  * Backend:

    * `GET /api/nodes/{id}/metrics/summary`:

      * On each request (or at polling intervals), fetch `/metrics` from the node.
      * Parse a small, whitelisted subset of metrics (e.g., request counters, latency histograms, restart counters).
      * Maintain an in-memory ring buffer per node/plane with the last N samples (bounded by `polling.metrics_window`).

    * `GET /api/nodes/{id}/metrics/facets`:

      * Same underlying metrics source, but:

        * filters series by `facet` label or `ron_facet_*` prefix,
        * groups them by facet name for the SPA.

  * Frontend:

    * Polls `metrics/summary` every X seconds for the active node.
    * When the **Facets** tab is active, polls `metrics/facets` (or reuses cached data) and renders facet-specific charts.

- [C-5] **Status mapping → UI components**

  React components, e.g.:

  * `<ReadinessPill status={adminStatusView}>`
  * `<PlaneGrid planes={adminStatusView.planes}>`
  * `<PlaneDetail plane={planeStatus}>`
  * `<MetricsChart data={metricsSeries}>`
  * `<FacetMetricsPanel facets={facetMetricsSummary}>`
  * `<IdentityWidget me={me} onSignIn={...} onSignOut={...} />`

  Each component:

  * Has a small prop type.
  * Encodes the deterministic mapping from status/metrics to colors/icons.
  * Reads labels via i18n keys and colors via theme tokens.

- [C-6] **Amnesia display**

  * `AdminStatusView` includes `amnesia: bool`.
  * UI:

    * Shows a prominent badge near the node name: `Amnesia: ON` / `OFF`.
    * Adds annotations where relevant (e.g., logs: “Node reports amnesia; logs may not be persisted to disk”).

- [C-7] **Auth / policy integration**

  **Operator auth (svc-passport / ingress):**

  * Auth modes:

    * `auth.mode = "none"`:

      * Dev-only; svc-admin SHOULD only bind to loopback.
      * `/api/me` returns a synthetic “dev” identity.
    * `auth.mode = "ingress"`:

      * svc-admin trusts identity headers set by an upstream ingress (e.g., `X-User`, `X-Groups`).
      * `/api/me` reflects those claims and maps them to local roles.
    * `auth.mode = "passport"`:

      * svc-admin expects a passport token (cookie or bearer) issued by svc-passport.
      * Validates signature/expiry/issuer/audience using configured keys.
      * If missing or invalid, returns `401` and (optionally) a `login_url` pointing to svc-passport.

  * SPA flow:

    * On load, SPA calls `/api/me`.
    * If `200`, shows identity + role and enables viewer/admin UI accordingly.
    * If `401` with `login_url`, shows a **Sign in** button that redirects to svc-passport.

  **Node admin auth:**

  * All proxy requests from svc-admin to nodes:

    * MUST attach the node’s configured auth token or macaroon in the correct header.
    * MUST use HTTPS by default; `insecure_http = true` is required to allow HTTP (dev only).

  * Future actions:

    * `POST /api/nodes/{id}/reload` (svc-admin endpoint) → `POST <node>/api/v1/reload`.
    * `POST /api/nodes/{id}/shutdown` → `POST <node>/api/v1/shutdown`.

  * These actions MUST be gated by:

    * svc-admin config (`actions.enable_*`),
    * operator roles from `/api/me` (viewer vs admin),
    * node-level authn/authz,
    * and MUST emit audit events.

- [C-8] **Packaging & deployment**

  * Binary:

    * `svc-admin` single binary, static-link friendly.
    * Contains embedded SPA assets when `embed-spa` feature is enabled.

  * Config:

    * Default path `svc-admin.toml` in working directory, overrideable via `--config` or env.

  * Deployment:

    * Run behind a reverse proxy or gateway if desired (often backed by svc-passport), but MUST work stand-alone over TLS.

- [C-9] **Testing harness**

  * “Fake-node” harness:

    * Exposes `/healthz`, `/readyz`, `/metrics`, `/version`, `/api/v1/status` with controllable responses.
    * Can be used to simulate facet metrics (`facet` labels) and amnesia flags.

  * Integration tests:

    * Spin up fake-node + svc-admin.
    * Assert correct JSON admin views and metrics summaries.

- [C-10] **Code comments and CI alignment**

  * Every Rust module in svc-admin SHOULD follow `CODECOMMENTS.MD`, e.g.:

    ```rust
    //! RO:WHAT       — svc-admin HTTP proxy for node admin plane.
    //! RO:WHY        — Provide a stable JSON API for the SPA dashboard over /healthz,/readyz,/status,/metrics.
    //! RO:INTERACTS  — NodeConfig, reqwest-based proxy client, SPA static assets, svc-passport / ingress.
    //! RO:INVARIANTS — No unsafe; no new protocols; all state derived from node endpoints, config, or trusted identity claims.
    ```

  * CI:

    * `cargo fmt`, `cargo clippy --all-targets --all-features`, and full tests for svc-admin.
    * Integrated into workspace CODECHECK flows.

- [C-11] **Theming (light/dark/red-eye) and tokens**

  * CSS / Tailwind design tokens:

    * Define color variables such as `--color-bg`, `--color-bg-alt`, `--color-text`, `--color-accent`, `--color-danger`, etc.
    * Provide three theme scopes:

      * `data-theme="light"`
      * `data-theme="dark"`
      * `data-theme="red-eye"` (very dark, low-blue, warm text).

  * Components:

    * MUST use design tokens (CSS vars or mapped Tailwind classes), not raw hex values.

  * Theme selection:

    * Default from `ui.theme` config (`system`/`light`/`dark`/`red-eye`).
    * Per-operator override stored in `localStorage` and applied via `document.documentElement.dataset.theme`.

- [C-12] **i18n and language packs**

  * SPA defines a simple translation function, e.g.:

    ```ts
    import { useI18n } from "./i18n";

    const { t } = useI18n();
    <h1>{t("overview.title")}</h1>;
    ```

  * Language packs:

    ```ts
    // en-US.ts
    export const enUS = {
      "overview.title": "Node Overview",
      "overview.status.ready": "Ready",
      "overview.status.notReady": "Not Ready",
      "planes.gateway": "Gateway Plane",
      "planes.overlay": "Overlay Plane",
      "theme.light": "Light",
      "theme.dark": "Dark",
      "theme.redEye": "Red-eye",
      "auth.signIn": "Sign in",
      "auth.signedInAs": "Signed in as {{user}}",
      // ...
    } as const;
    ```

  * Backend exposes `/api/ui-config` with:

    * `defaultLanguage`, `availableLanguages`, `defaultTheme`.

  * SPA:

    * Picks initial language from config or browser locale (if supported).
    * Allows per-operator override in the UI.

- [C-13] **Facet metrics tab/button**

  * Backend:

    * Discovers facet metrics by:

      * scanning metrics for `facet` label, or
      * matching `ron_facet_*` prefixes.
    * Provides an aggregated response from `/api/nodes/{id}/metrics/facets`:

      * per-facet summary (e.g., requests/second, error rate, latency).

  * Frontend:

    * Shows a **Facets** tab or button when facet metrics are detected for the selected node.
    * Clicking it:

      * loads facet metrics (if not already cached),
      * renders a per-facet grid of charts and status badges.
    * If no facet metrics exist, the tab/button is disabled or hidden with a clear explanation.

---

## 4. Acceptance Gates (PROOF)

* [G-1] **Readiness truth tests.**
  For a set of fixture `/readyz` + `/api/v1/status` responses (healthy, degraded, failing):

  * Backend tests MUST assert that `AdminStatusView` and the derived JSON for the SPA:

    * never mark nodes as ready when `/readyz` is negative, and
    * correctly identify which planes are blocking readiness.

* [G-2] **Metrics parsing property tests.**
  Given arbitrary but valid Prometheus-text metrics:

  * Property tests MUST show that:

    * parsing either succeeds with a defined “summary view” or cleanly rejects malformed metrics.
    * no panic or misclassification occurs due to unexpected metrics.
    * facet metrics are detected only when `facet` labels or `ron_facet_*` metrics are actually present.

* [G-3] **No-shell guarantee.**
  A static analysis / grep-based gate MUST confirm:

  * no use of `std::process::Command` or similar shell-exec primitives in svc-admin’s codebase.
  * If any such usage exists (e.g., for dev-only scripts), it MUST be under `#[cfg(test)]` or clearly isolated and documented, and NOT reachable from the running server.

* [G-4] **Security regression tests.**

  * Integration tests MUST verify:

    * svc-admin refuses to proxy to nodes over HTTP unless `insecure_http = true` is set in config.
    * auth headers are correctly attached to node calls when configured.
    * `svc-admin` returns appropriate HTTP errors (401/403/502) when a node rejects or is unreachable, and the SPA surfaces them without crashing.

* [G-5] **Profile coverage tests.**

  * Fixtures for:

    * macronode-like `/api/v1/status` (multiple planes),
    * micronode-like `/api/v1/status` (KV + facets only).

  * Tests MUST assert that:

    * macro fixtures produce multiple plane tiles.
    * micro fixtures hide macro-only planes and show KV/facets correctly.
    * the SPA’s component tree does not break when planes are missing.

* [G-6] **3 a.m. UX walkthrough.**

  A manual or scripted checklist for reviewers MUST verify that, for a node configured as:

  * healthy → the overview clearly shows “Ready” and all planes green.
  * broken plane (e.g., overlay) → the overview highlights the failing plane and provides an immediate explanation of the failure chain.
  * unreachable → a clear error banner is shown, not a blank or spinning page.

* [G-7] **Zero unsafe & lint gates.**

  CI MUST enforce:

  * `cargo clippy --all-targets --all-features` with the same pedantic settings used elsewhere in RON-CORE.
  * A `cargo geiger` or similar check (if adopted) showing zero `unsafe` usage in svc-admin.
  * All tests (unit, integration, property) green.

* [G-8] **SPA integration tests.**

  Using Playwright/Cypress or a similar framework (once wired):

  * End-to-end tests MUST hit the SPA, load the overview page against a fake-node, and assert:

    * readiness pill text/color,
    * presence of plane tiles,
    * basic chart rendering with metrics data.

* [G-9] **Audit hook validation (for actions).**

  Once mutations (reload/shutdown) are added:

  * Tests MUST verify that every such action:

    * triggers an outbound call to the node’s admin endpoint, and
    * emits a structured log/audit event in svc-admin’s logs that includes node id, action, user (if applicable), and timestamp.

* [G-10] **Theming, i18n, and facet metrics behavior.**

  * Theming:

    * Automated or manual tests MUST confirm that changing theme (light/dark/red-eye) updates component visuals without breaking layout.

  * i18n:

    * At least one non-English test pack (e.g., `es-ES`) MUST be wire-able in dev mode, and UI text MUST come from translation keys, not hard-coded strings.

  * Facet metrics:

    * With a fake-node exposing facet metrics (via `facet` label), the **Facets** tab/button MUST appear and show grouped metrics.
    * With no facet metrics, the tab/button MUST be absent or clearly disabled, without errors.

* [G-11] **Auth mode & passport integration tests.**

  * For `auth.mode = "none"`:

    * svc-admin MUST bind only to loopback (in tests).
    * `/api/me` MUST return a synthetic dev identity and mark the mode as non-production.

  * For `auth.mode = "ingress"`:

    * Tests MUST set identity headers on requests (e.g., `X-User`, `X-Groups`).
    * `/api/me` MUST reflect those claims and map them to local roles.

  * For `auth.mode = "passport"`:

    * Tests MUST simulate valid and invalid passport tokens:

      * valid → `/api/me` returns 200 with identity/roles.
      * invalid/expired/missing → `/api/me` returns 401 with a `login_url` field.
    * Admin-only UI actions MUST be inaccessible when `/api/me` reports no admin role.

---

## 5. Anti-Scope (Forbidden)

svc-admin MUST NOT:

* Become a general-purpose **orchestrator** (no scheduling, no placement, no autoscaling decisions).
* Implement or embed its own **time-series database** or long-term metrics storage beyond a short, in-memory buffer.
* Execute **shell commands**, **scripts**, or **remote code** as part of its normal operation.
* Maintain an internal **health model** that overrides or contradicts node-reported `/healthz` and `/readyz`.
* Offer **Web3** or token/ledger/rewarder views; RON-CORE’s Web3-related crates are explicitly out-of-scope for the RON-CORE pivot.
* Store node credentials (tokens, macaroons) in cleartext logs or unencrypted dumps.
* Introduce a separate **user database** or full-blown identity system; if multi-user auth is needed, it MUST integrate with existing identity mechanisms (e.g., svc-passport, OIDC, SSO) rather than inventing one.
* Directly edit node configuration files on disk over SSH, NFS, or other file system access mechanisms.
* Depend on non-standard or exotic front-end frameworks that are hard to maintain (stick to React + TypeScript).
* Hard-code user-facing strings or colors in components; those MUST flow through i18n message keys and theme tokens.

---

## 6. References

* RON-CORE master blueprint (`RON_CORE.MD`)
* MICROKERNEL blueprint (kernel, supervisor, bus, planes)
* SCALING blueprint (planes, readiness, golden metrics)
* HARDENING blueprint (security posture, amnesia, TLS)
* SIX_CONCERNS blueprint (Governance, Observability, Security, etc.)
* Configuration blueprint for `svc-admin` (Config schema, reload behavior)
* ALLCODEBUNDLES.MD (current implementations of:

  * `ron-kernel` (KernelEvent, supervisor, bus),
  * `ron-metrics` (Prometheus export, HealthState),
  * `svc-overlay`, `svc-dht`, `svc-storage`, `svc-index`, `ron-policy`, etc.)
* `svc-passport` (planned) IDB — canonical identity / passport provider for operator auth.
* External analogues for inspiration (not as hard dependencies):

  * Kubernetes Dashboard (read-only cluster views, RBAC)
  * Consul UI (service & health views)
  * Prometheus + Grafana (metrics + dashboards)

> **Facet metrics note:**
> Facets SHOULD expose metrics via the shared Prometheus registry with a `facet` label (or well-defined prefix such as `ron_facet_*`), so svc-admin can group and visualize facet-level metrics without additional node-specific code.

```
```
