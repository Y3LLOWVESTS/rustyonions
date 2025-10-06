
---

# 📁 File Tree — `crates/ron-metrics`

```text
crates/
└─ ron-metrics/
   ├─ Cargo.toml
   ├─ README.md
   ├─ CHANGELOG.md
   ├─ LICENSE-APACHE
   ├─ LICENSE-MIT
   ├─ CODEOWNERS
   ├─ .gitignore
   ├─ rust-toolchain.toml
   ├─ deny.toml
   ├─ .cargo/
   │  └─ config.toml
   ├─ src/
   │  ├─ lib.rs
   │  ├─ config.rs
   │  ├─ metrics.rs
   │  ├─ registry.rs
   │  ├─ labels.rs
   │  ├─ readiness.rs
   │  ├─ health.rs
   │  ├─ errors.rs
   │  ├─ build_info.rs
   │  ├─ pq.rs
   │  ├─ zk.rs
   │  └─ exposer/
   │     ├─ mod.rs
   │     ├─ http.rs
   │     ├─ middleware.rs
   │     ├─ uds.rs
   │     └─ tls.rs
   ├─ exporters/
   │  ├─ mod.rs
   │  └─ otel.rs
   ├─ examples/
   │  └─ exposer.rs
   ├─ tests/
   │  ├─ integration_http_endpoints.rs
   │  ├─ taxonomy_labels.rs
   │  ├─ readiness_semantics.rs
   │  ├─ public_api.rs
   │  ├─ loom_shutdown.rs
   │  └─ vectors/
   │     └─ interop/ron-metrics/
   │        ├─ readyz-ready.json
   │        ├─ readyz-degraded.json
   │        ├─ metrics-small.prom
   │        └─ metrics-large.prom
   ├─ benches/
   │  ├─ exposer_bench.rs
   │  └─ hotpath_bench.rs
   ├─ docs/
   │  ├─ API.md
   │  ├─ CONCURRENCY.md
   │  ├─ CONFIG.md
   │  ├─ GOVERNANCE.md
   │  ├─ IDB.md
   │  ├─ INTEROP.md
   │  ├─ OBSERVABILITY.md
   │  ├─ PERFORMANCE.md
   │  ├─ QUANTUM.md
   │  ├─ RUNBOOK.md
   │  ├─ SECURITY.md
   │  ├─ mmd/
   │  │  ├─ arch.mmd
   │  │  ├─ sequence.mmd
   │  │  └─ state.mmd
   │  ├─ api-history/
   │  │  └─ ron-metrics/
   │  │     └─ 0001.initial.txt
   │  └─ sbom/
   │     └─ .gitkeep
   ├─ scripts/
   │  ├─ check-taxonomy.sh
   │  └─ render-mermaid.sh
   └─ .github/
      └─ workflows/
         ├─ ci.yml
         ├─ coverage.yml
         ├─ public-api.yml
         ├─ mermaid.yml
         ├─ sanitizer.yml
         └─ perf-smoke.yml
```

---

# 📄 Per-File Descriptions

## Repo meta & policy

* **Cargo.toml** — Declares the crate, features (`otel`, `tls`, `pq`, `zk`), and workspace-pinned deps; sets MSRV and categories; no code gen.
* **README.md** — Contract of behavior and interfaces (endpoints, labels, SLOs, acceptance gates, diagrams); single source of truth for users/operators.
* **CHANGELOG.md** — SemVer history and migration notes (especially metric/label changes and endpoint semantics).
* **LICENSE-APACHE / LICENSE-MIT** — Dual licensing files for redistribution/audit compliance.
* **CODEOWNERS** — Enforces review from designated maintainers for changes to public API/metrics taxonomy.
* **.gitignore** — Ignores build artifacts, coverage, bench outputs, SBOMs, and rendered diagrams.
* **rust-toolchain.toml** — Pins the Rust toolchain to stabilize CI and local dev parity.
* **deny.toml** — `cargo-deny` policy for licenses/advisories/banned crates; keeps the supply chain clean.

## Cargo config

* **.cargo/config.toml** — Common build flags and lints; treats warnings as errors in CI and can set default run profiles.

## Library source

* **src/lib.rs** — Minimal façade that re-exports public items (`Metrics`, `Config`, readiness DTOs) and wires crate-level docs; no heavy logic.
* **src/config.rs** — Typed configuration loader/validator (TCP addr vs UDS path, timeouts, TLS file paths, OTLP endpoint); rejects unsafe combos early.
* **src/metrics.rs** — Owns metric family registration (golden counters/histograms/gauges) and provides handles used by the exposer; ensures one-time registry init.
* **src/registry.rs** — Helpers around `prometheus::Registry` to prevent duplicate registration and to namespace families consistently.
* **src/labels.rs** — Defines and validates base labels (`service`, `instance`, `build_version`, `amnesia`) and enforces low-cardinality rules.
* **src/readiness.rs** — Readiness JSON DTO and evaluation logic; encodes **fail-open reads / fail-closed writes** policy with `Retry-After` guidance.
* **src/health.rs** — Liveness snapshot glue (simple yes/no) separate from readiness; avoids coupling basic up/down to dependency state.
* **src/errors.rs** — Small, ops-focused error taxonomy (Busy/Timeout/Canceled/Lagging) used across endpoints and middleware.
* **src/build_info.rs** — Injects build metadata (version/commit/date) from env to populate the `build_version` label deterministically.
* **src/pq.rs** — Declares PQ-related metric families (e.g., `pq_kex_failures_total`) so dashboards remain stable even when PQ is inactive.
* **src/zk.rs** — Declares ZK metric families (e.g., verify failures/latencies) with predefined buckets; present as zeroes when ZK is unused.

### HTTP exposer (isolated)

* **src/exposer/mod.rs** — Assembles the Axum router for `GET /metrics`, `GET /healthz`, `GET /readyz`; mounts middleware and wiring to `Metrics`.
* **src/exposer/http.rs** — Handlers and method guards (GET-only), response headers (`no-store`, `nosniff`), and content shaping; keeps path logic tiny.
* **src/exposer/middleware.rs** — Per-route timeouts/concurrency caps and inflight gauges; keeps `/metrics` responsive under load.
* **src/exposer/uds.rs** — UDS binding with secure mode defaults (0700 dir / 0600 socket) and safety checks (no symlinks).
* **src/exposer/tls.rs** — TLS wiring specifically using `tokio_rustls::rustls::ServerConfig`; isolates certificate loading and acceptor creation.

## Exporters (optional)

* **exporters/mod.rs** — Feature-gate seam for exporters; keeps core API unchanged regardless of enabled exporters.
* **exporters/otel.rs** — OTLP exporter mapping (1:1 with Prometheus families/labels) when `otel` feature is enabled.

## Examples

* **examples/exposer.rs** — Minimal, doc-tested example showing `Metrics::new().serve(...)` and printing the bound address; used for quick local checks.

## Tests (acceptance gates)

* **tests/integration_http_endpoints.rs** — Verifies `/metrics`, `/healthz`, `/readyz` status codes, headers, and body shapes end-to-end.
* **tests/taxonomy_labels.rs** — Enforces suffix discipline (`*_seconds|*_bytes|*_total`) and presence of base labels on all first-party families.
* **tests/readiness_semantics.rs** — Asserts **fail-open reads / fail-closed writes**, including `Retry-After` on 503 and degraded/missing keys behavior.
* **tests/public_api.rs** — Guards against public symbol drift (compares to stored snapshot) to maintain SemVer stability.
* **tests/loom_shutdown.rs** — Loom-gated test that supervises shutdown paths and ensures no lock is held across `.await` in critical sections.
* **tests/vectors/interop/ron-metrics/readyz-ready.json** — Canonical sample for a healthy readiness response used by tests and external SDKs.
* **tests/vectors/interop/ron-metrics/readyz-degraded.json** — Canonical degraded example with `missing` keys and `retry_after` used by tests.
* **tests/vectors/interop/ron-metrics/metrics-small.prom** — Small, deterministic Prometheus output used for taxonomy and parser tests.
* **tests/vectors/interop/ron-metrics/metrics-large.prom** — Large registry variant to test scraper performance and size limits.

## Benches (SLO proof)

* **benches/exposer_bench.rs** — Criterion benchmarks for `/metrics` exposition latency across registry sizes; backs the p95 SLO gate.
* **benches/hotpath_bench.rs** — Measures counter/histogram update hot path for allocation/lock behavior to prevent regressions.

## Docs (living design)

* **docs/API.md** — Public Rust surface and stability rules; documents expected signatures and invariants for `Metrics`/exposer.
* **docs/CONCURRENCY.md** — States runtime rules (no locks across `.await`), threading, and loom strategy for safety.
* **docs/CONFIG.md** — Enumerates env/flags schema; defines TCP vs UDS vs TLS modes and safe defaults.
* **docs/GOVERNANCE.md** — Review process and ownership; clarifies how metric/label changes are approved and versioned.
* **docs/IDB.md** — Invariant-Driven Blueprint: MUSTs, Principles, and PROOF gates that the tests/CI enforce.
* **docs/INTEROP.md** — Readiness JSON schema, Prometheus exposition rules, and OTLP mapping; includes canonical vectors.
* **docs/OBSERVABILITY.md** — Golden metrics, base labels, dashboards, and alerting best practices tied to this crate.
* **docs/PERFORMANCE.md** — SLO targets, reproduction methods, and perf regression policy for merges.
* **docs/QUANTUM.md** — PQ posture (observability scope) and how PQ metrics are represented without forcing PQ adoption.
* **docs/RUNBOOK.md** — Operator procedures, common incidents, and diagnostic steps for unhealthy scrapes or readiness 503s.
* **docs/SECURITY.md** — Threat model, GET-only stance, TLS type invariant, and label cardinality guardrails.
* **docs/mmd/arch.mmd** — Mermaid flowchart of the exposer with Prometheus/OTLP; rendered in CI for visual audit.
* **docs/mmd/sequence.mmd** — Mermaid sequence diagram of a scrape, deadlines, and success path.
* **docs/mmd/state.mmd** — Mermaid state diagram for lifecycle: Idle → Running → Degraded → Shutdown.
* **docs/api-history/ron-metrics/0001.initial.txt** — Baseline `cargo public-api` snapshot to detect breaking changes in CI.
* **docs/sbom/.gitkeep** — Placeholder to store generated SBOMs (CycloneDX) for releases.

## Scripts

* **scripts/check-taxonomy.sh** — CI helper script that asserts suffix discipline and base labels in `/metrics` output.
* **scripts/render-mermaid.sh** — Local helper that renders all `.mmd` files to `.svg` to preview diagrams offline.

## GitHub Actions

* **.github/workflows/ci.yml** — Main CI: fmt, clippy (deny warnings), tests, and `cargo-deny`; fast feedback on every PR.
* **.github/workflows/coverage.yml** — Enforces ≥80% branches on core modules via `cargo llvm-cov`; blocks regressions.
* **.github/workflows/public-api.yml** — Fails on public symbol diffs using `cargo public-api`; keeps SemVer honest.
* **.github/workflows/mermaid.yml** — Renders Mermaid diagrams to SVG artifacts so reviewers can see topology diffs.
* **.github/workflows/sanitizer.yml** — Optional nightly TSan run to catch data races in the exposer/middleware.
* **.github/workflows/perf-smoke.yml** — Non-blocking smoke that hammers `/metrics` and surfaces latency artifacts for manual review.

---

