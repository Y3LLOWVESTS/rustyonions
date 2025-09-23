

```
crates/macronode/
├─ Cargo.toml
├─ README.md
├─ LICENSE
├─ .gitignore
├─ src/
│  ├─ main.rs
│  ├─ errors.rs
│  ├─ types.rs
│  ├─ observability/
│  │  ├─ mod.rs
│  │  ├─ metrics.rs
│  │  └─ logging.rs
│  ├─ config/
│  │  ├─ mod.rs
│  │  ├─ schema.rs
│  │  ├─ load.rs
│  │  ├─ env_overlay.rs
│  │  ├─ cli_overlay.rs
│  │  ├─ validate.rs
│  │  └─ hot_reload.rs
│  ├─ cli/
│  │  ├─ mod.rs
│  │  ├─ args.rs
│  │  ├─ run.rs
│  │  ├─ version.rs
│  │  ├─ check.rs
│  │  ├─ config_print.rs
│  │  ├─ config_validate.rs
│  │  └─ doctor.rs
│  ├─ supervisor/
│  │  ├─ mod.rs
│  │  ├─ lifecycle.rs
│  │  ├─ backoff.rs
│  │  ├─ crash_policy.rs
│  │  ├─ shutdown.rs
│  │  └─ health_reporter.rs
│  ├─ readiness/
│  │  ├─ mod.rs
│  │  └─ deps.rs
│  ├─ http_admin/
│  │  ├─ mod.rs
│  │  ├─ router.rs
│  │  ├─ middleware/
│  │  │  ├─ mod.rs
│  │  │  ├─ request_id.rs
│  │  │  ├─ timeout.rs
│  │  │  ├─ auth.rs
│  │  │  └─ rate_limit.rs
│  │  ├─ handlers/
│  │  │  ├─ version.rs
│  │  │  ├─ healthz.rs
│  │  │  ├─ readyz.rs
│  │  │  ├─ metrics.rs
│  │  │  ├─ status.rs
│  │  │  ├─ reload.rs
│  │  │  └─ shutdown.rs
│  ├─ services/
│  │  ├─ mod.rs
│  │  ├─ registry.rs
│  │  ├─ spawn.rs
│  │  ├─ svc_gateway.rs
│  │  ├─ svc_overlay.rs
│  │  ├─ svc_index.rs
│  │  ├─ svc_storage.rs
│  │  ├─ svc_mailbox.rs
│  │  └─ svc_dht.rs
│  ├─ bus/
│  │  ├─ mod.rs
│  │  └─ events.rs
│  ├─ facets/
│  │  ├─ mod.rs
│  │  ├─ permits.rs
│  │  └─ quotas.rs
│  ├─ security/
│  │  ├─ mod.rs
│  │  ├─ tls.rs
│  │  ├─ macaroon.rs
│  │  └─ amnesia.rs
│  ├─ pq/                 # feature = "pq"
│  │  ├─ mod.rs
│  │  └─ hybrid.rs
│  └─ util/
│     ├─ sizes.rs
│     └─ dur.rs
├─ tests/
│  ├─ admin_smoke.rs
│  ├─ readiness_drain.rs
│  └─ metrics_contract.rs
├─ benches/
│  └─ admin_paths_latency.rs
├─ fuzz/
│  └─ oap_frame_parser/   # skeleton target, no code yet
├─ scripts/
│  ├─ dump_http_surface.sh
│  ├─ dump_metrics_names.sh
│  └─ render_mermaid.sh
├─ docs/
│  ├─ ALL_DOCS.md
│  ├─ API.md
│  ├─ CONFIG.md
│  ├─ CONCURRENCY.md
│  ├─ GOVERNANCE.md
│  ├─ IDB.md
│  ├─ INTEROP.md
│  ├─ OBSERVABILITY.md
│  ├─ PERFORMANCE.md
│  ├─ QUANTUM.md
│  ├─ RUNBOOK.md
│  ├─ SECURITY.md
│  ├─ TESTS.md
│  ├─ api-history/
│  │  ├─ macronode/
│  │  │  ├─ cli-vX.Y.Z.txt
│  │  │  ├─ http-vX.Y.Z.json
│  │  │  └─ metrics-vX.Y.Z.txt
│  └─ openapi/
│     └─ macronode.v1.yaml
└─ .github/
   └─ workflows/
      ├─ ci.yml
      └─ render-mermaid.yml
```

### What each file does (brief, single-purpose roles)

* `Cargo.toml` — Declares a **service crate** with binary target, feature flags (`cli`, `tls`, optional `pq`), and pins matching your stack; ensures no stable Rust API is exported (CI guard).&#x20;
* `README.md` — Operator/developer overview, SLOs, endpoints, and run commands; mirrors your finalized README canon.&#x20;
* `LICENSE` — Dual MIT/Apache-2.0 as per repo norm.
* `.gitignore` — Standard Rust target/coverage, plus local artifacts.

#### `src/` (runtime + small modules)

* `main.rs` — Thin entrypoint: parses CLI, loads config, wires supervisor + admin HTTP as per contract (`/version`, `/healthz`, `/readyz`, `/metrics`).&#x20;
* `errors.rs` — Centralized error types for config/admin/supervisor paths; keeps handlers small and typed.
* `types.rs` — Common DTOs (e.g., version payload), strongly typed sizes/durations used across modules.

##### Observability

* `observability/mod.rs` — Glue for tracing + metrics init.
* `observability/metrics.rs` — Registers canonical metrics (request totals, latency histograms, ready state, restarts) and exposes helpers; names match the contract and stability rules.&#x20;
* `observability/logging.rs` — Structured logging init (JSON/text, levels), request-id propagation.

##### Config (file/env/CLI + hot reload)

* `config/mod.rs` — Public facade returning an `Arc<Config>` snapshot.
* `config/schema.rs` — Typed schema for all keys (binds, timeouts, limits, TLS, services, facet permits).&#x20;
* `config/load.rs` — Reads TOML/JSON, applies defaults.
* `config/env_overlay.rs` — Applies `MACRO_*`/`RON_*` env overlays in documented precedence.&#x20;
* `config/cli_overlay.rs` — Maps CLI flags to config overlays; used by `run` and `config print/validate`.
* `config/validate.rs` — **Fail-closed** validation (ports, TLS files, sizes, service names).&#x20;
* `config/hot_reload.rs` — SIGHUP or bus-driven snapshot swap; emits `ConfigUpdated` event.

##### CLI (operator surface; no business logic)

* `cli/mod.rs` — Command router.
* `cli/args.rs` — Defines `run/version/check/config/doctor` subcommands/flags as specified.&#x20;
* `cli/run.rs` — Starts runtime with selected services, observability, and admin HTTP.
* `cli/version.rs` — Prints `{service, version, git_sha, build_ts, rustc, msrv}`.&#x20;
* `cli/check.rs` — Validates environment (ports, perms, time/entropy) and config.&#x20;
* `cli/config_print.rs` — Prints **effective** redacted config.
* `cli/config_validate.rs` — Off-line validation of a supplied file.
* `cli/doctor.rs` — Diagnostics bundle (fs/dns/time drift/ports).&#x20;

##### Supervision & lifecycle

* `supervisor/mod.rs` — Orchestrates long-lived tasks and service spawns; owns join handles and state machine.&#x20;
* `supervisor/lifecycle.rs` — Start→ready→drain→stop ordering; ensures Metrics server exits last; enforces grace windows.&#x20;
* `supervisor/backoff.rs` — Jittered exponential backoff primitives for task restarts (caps as documented).&#x20;
* `supervisor/crash_policy.rs` — Degrade/readiness rules when restart loops exceed thresholds.&#x20;
* `supervisor/shutdown.rs` — Graceful drain choreography and final abort; updates gauges and `Retry-After`.&#x20;
* `supervisor/health_reporter.rs` — Periodic `KernelEvent::Health{service, ok}` emission + ready aggregation.&#x20;

##### Readiness

* `readiness/mod.rs` — In-process readiness state (unready/degraded/ready); exposes `/readyz` source of truth.&#x20;
* `readiness/deps.rs` — Pluggable checks (config loaded, listeners bound, services alive).

##### Admin HTTP (axum 0.7)

* `http_admin/mod.rs` — Server bootstrap on `--http-addr` (or loopback default); HTTP/1.1 + HTTP/2; TLS gated.&#x20;
* `http_admin/router.rs` — Routes: `/version`, `/healthz`, `/readyz`, `/metrics`, `POST /api/v1/{reload,shutdown}`, optional `/api/v1/status`.&#x20;
* `http_admin/middleware/request_id.rs` — Generate/propagate `X-Request-Id`.
* `http_admin/middleware/timeout.rs` — Per-endpoint deadlines (read/write/total).&#x20;
* `http_admin/middleware/auth.rs` — Loopback-by-default; mTLS/macaroons for non-loopback admin; **no CORS**.&#x20;
* `http_admin/middleware/rate_limit.rs` — Shed with `429` under pressure; set `Retry-After`.&#x20;
* `http_admin/handlers/version.rs` — Minimal static JSON per README.&#x20;
* `http_admin/handlers/healthz.rs` — Liveness check.&#x20;
* `http_admin/handlers/readyz.rs` — Readiness (200 or 503 + Retry-After).&#x20;
* `http_admin/handlers/metrics.rs` — Prometheus text exposition; stable names.&#x20;
* `http_admin/handlers/status.rs` — Optional snapshot: uptime/build/resources/warnings (feature-gated).&#x20;
* `http_admin/handlers/reload.rs` — Triggers config snapshot swap; emits audit event.&#x20;
* `http_admin/handlers/shutdown.rs` — Initiates graceful shutdown (audit-required).&#x20;

##### Services (composition only; no business logic here)

* `services/mod.rs` — Facade for enabling/disabling canonical services by config list.&#x20;
* `services/registry.rs` — Known service IDs and feature flags; denies unknowns (keeps canon honest).&#x20;
* `services/spawn.rs` — Spawns and supervises each selected service; returns join handles; enforces caps and drain order.
* `services/svc_gateway.rs` … `svc_dht.rs` — Thin in-proc adapters/wrappers and health hooks for each composed service role (gateway, overlay, index, storage, mailbox, DHT).&#x20;

##### Bus (events)

* `bus/mod.rs` — ron-bus wrapper: subscribe/publish, lag handling, queue caps.
* `bus/events.rs` — Exact typed events used here (Health, ConfigUpdated, ServiceCrashed, Shutdown), matching README.&#x20;

##### Facets (controls for hot spots)

* `facets/mod.rs` — Facet registry (Feed/Graph/Search/Index/Media/Mailbox).
* `facets/permits.rs` — CPU-scaled semaphores (`NxCPU` parsing), as documented in Config.&#x20;
* `facets/quotas.rs` — (Optional) per-tenant token buckets to keep tail latencies in SLO.

##### Security

* `security/mod.rs` — Security glue: auth decisions, redaction helpers.
* `security/tls.rs` — rustls server config builder; rejects weak ciphers; handshake deadline.&#x20;
* `security/macaroon.rs` — Capability tokens loader/validator; no logging of secrets.&#x20;
* `security/amnesia.rs` — Enforces amnesia mode toggles (no disk writes, zeroize on shutdown).&#x20;

##### Post-Quantum (optional, feature-gated)

* `pq/mod.rs` — PQ glue; toggles hybrid mode.
* `pq/hybrid.rs` — Stubs to integrate KEX/sign adapters later; handshake deadline cap noted in Concurrency doc.&#x20;

##### Utilities

* `util/sizes.rs` — Byte-size parsing (`1MiB`, `64KiB`) for limits.
* `util/dur.rs` — Duration parsing helpers for `5s`, `60s`, etc.

#### Tests/benches/fuzz

* `tests/admin_smoke.rs` — `/version|/healthz|/readyz|/metrics` status/type smoke tests; enforce response shape.&#x20;
* `tests/readiness_drain.rs` — Drain ordering SLOs (p95≤3s/p99≤5s) under chaos.&#x20;
* `tests/metrics_contract.rs` — Metric name stability (deny removal/rename without deprecation window).&#x20;
* `benches/admin_paths_latency.rs` — p90/p99 latency harness for admin plane SLOs.&#x20;
* `fuzz/oap_frame_parser/` — Placeholder target directory per Concurrency §7 (frame bounds/ratio guard).&#x20;

#### Scripts

* `scripts/dump_http_surface.sh` — Emits HTTP surface snapshot to `docs/api-history/macronode/http-vX.Y.Z.json`.&#x20;
* `scripts/dump_metrics_names.sh` — Emits metric names to `docs/api-history/macronode/metrics-vX.Y.Z.txt`.&#x20;
* `scripts/render_mermaid.sh` — Batch-render `.mmd` → `.svg` for docs/diagrams per README tooling.&#x20;

#### Docs (checked in; you already have them)

* `docs/*.md` — The 12 canonical docs: API, CONFIG, CONCURRENCY, IDB, GOVERNANCE, INTEROP, OBSERVABILITY, PERFORMANCE, QUANTUM, RUNBOOK, SECURITY, TESTS.&#x20;
* `docs/api-history/macronode/*` — Snapshots of CLI help, HTTP surface, and metrics names for SemVer tracking.&#x20;
* `docs/openapi/macronode.v1.yaml` — Minimal OpenAPI stub for admin endpoints (`/version`, `/healthz`, `/readyz`, `/metrics`, optional admin ops).&#x20;

#### CI

* `.github/workflows/ci.yml` — Clippy/deny/tests + public-API diff + perf micro-gate on admin paths (small RPS), matching README and API gates.&#x20;
* `.github/workflows/render-mermaid.yml` — Renders mermaid diagrams into SVG on PRs, as in README tooling.&#x20;

---

### Why this structure fits your canon

* **Service crate, no Rust API**: We treat this as a binary-only, operator surface; CI denies public items (tree keeps modules private behind `main`).&#x20;
* **Admin plane exactness**: Routes/middleware map 1:1 to `/version`, `/healthz`, `/readyz`, `/metrics`, with auth/backpressure/timeouts per spec.&#x20;
* **Concurrency & drain**: Supervisor, backoff, shutdown, and readiness modules mirror your runtime topology and SLOs.&#x20;
* **Config precedence & hot-reload**: Schema + overlays + fail-closed validation + reload emission as documented.&#x20;
* **Metrics stability**: Dedicated module and snapshot scripts uphold the “names frozen” rule and deprecation window.&#x20;
* **Governance boundaries**: `services/registry.rs` denies drift (unknown crates), keeping macronode honest about what it composes.&#x20;
