

---

# `svc-gateway` — Final Scaffold (no code yet)

```
svc-gateway/
├─ Cargo.toml
├─ README.md
├─ CHANGELOG.md
├─ LICENSE-APACHE
├─ LICENSE-MIT
├─ .gitignore
├─ .editorconfig
├─ .github/
│  └─ workflows/
│     ├─ ci.yml
│     ├─ api-stability.yml
│     ├─ fuzz-nightly.yml
│     └─ perf-guard.yml
├─ configs/
│  ├─ svc-gateway.sample.toml
│  └─ .env.example
├─ docs/
│  ├─ IDB.md
│  ├─ INTEROP.md
│  ├─ CONFIG.md
│  ├─ SECURITY.md
│  ├─ OBSERVABILITY.md
│  ├─ PERFORMANCE.md
│  ├─ RUNBOOK.md
│  ├─ QUANTUM.md
│  ├─ API.md
│  ├─ POLICY.md
│  └─ diagrams/
│     ├─ architecture.mmd
│     ├─ sequence_ingress.mmd
│     └─ readiness_state.mmd
├─ docs/dashboards/
│  └─ gateway_golden.json
├─ scripts/
│  ├─ dev_run.sh
│  ├─ soak_test.sh
│  ├─ chaos_burst.sh
│  └─ export_metrics.sh
├─ examples/
│  ├─ curl.http
│  └─ httpie.http
├─ benches/
│  ├─ README.md
│  └─ baseline.json
├─ tests/
│  ├─ vectors/
│  │  ├─ oap1_frame_roundtrip.json
│  │  ├─ manifest_digest.json
│  │  └─ error_taxonomy.json
│  └─ integration/
│     ├─ readyz_degrade.rs
│     ├─ caps_limits.rs
│     ├─ interop_vectors.rs
│     ├─ taxonomy_stability.rs
│     └─ loom_readiness.rs
├─ fuzz/
│  ├─ README.md
│  └─ targets/
│     ├─ oap_frame.rs
│     └─ taxonomy_mapper.rs
└─ src/
   ├─ main.rs
   ├─ lib.rs
   ├─ consts.rs
   ├─ errors.rs
   ├─ result.rs
   ├─ state.rs
   ├─ cli/
   │  └─ mod.rs
   ├─ config/
   │  ├─ mod.rs
   │  ├─ env.rs
   │  ├─ safety.rs
   │  └─ amnesia.rs
   ├─ observability/
   │  ├─ mod.rs
   │  ├─ metrics.rs
   │  ├─ tracing.rs
   │  └─ logging.rs
   ├─ headers/
   │  ├─ mod.rs
   │  └─ etag.rs
   ├─ tls/
   │  └─ mod.rs
   ├─ pq/
   │  ├─ mod.rs
   │  └─ policy.rs
   ├─ readiness/
   │  ├─ mod.rs
   │  └─ keys.rs
   ├─ policy/
   │  ├─ mod.rs
   │  ├─ residency.rs
   │  └─ abuse.rs
   ├─ layers/
   │  ├─ mod.rs
   │  ├─ timeouts.rs
   │  ├─ body_caps.rs
   │  ├─ decode_guard.rs
   │  ├─ rate_limit.rs
   │  ├─ drr.rs
   │  ├─ tarpit.rs
   │  ├─ auth.rs
   │  └─ corr.rs
   ├─ admission/
   │  ├─ mod.rs
   │  ├─ classifier.rs
   │  ├─ capabilities.rs
   │  ├─ quotas.rs
   │  ├─ taxonomy.rs
   │  ├─ residency.rs
   │  └─ payments.rs          # feature = "econ"
   ├─ routes/
   │  ├─ mod.rs
   │  ├─ health.rs
   │  ├─ ready.rs
   │  ├─ metrics.rs
   │  ├─ version.rs
   │  ├─ objects.rs           # GET /o/{addr}
   │  └─ objects_range.rs     # GET /o/{addr} with Range
   └─ forward/
      ├─ mod.rs
      ├─ overlay_client.rs
      ├─ index_client.rs
      └─ storage_client.rs
```

---

## What each file/dir is for

### Top-level

* **Cargo.toml** — Crate metadata + features (`tls`, `pq`, `cli`, `econ`). Deny unsafe, pin deps, and expose only the minimal public API.
* **README.md** — Human entry point: role, endpoints, SLOs, diagrams, run commands.
* **CHANGELOG.md** — SemVer-aligned, user-visible changes; taxonomy changes must be called out.
* **LICENSE-APACHE / LICENSE-MIT** — Dual license.
* **.gitignore / .editorconfig** — Hygiene and consistent formatting.

### CI workflows (`.github/workflows/`)

* **ci.yml** — Lints (clippy -Dwarnings), fmt, tests, cargo-deny, docs.
* **api-stability.yml** — `cargo-public-api` + `semver-checks`; blocks accidental breaking changes.
* **fuzz-nightly.yml** — Scheduled fuzz on `fuzz/targets/*`.
* **perf-guard.yml** — Runs benches and compares against `benches/baseline.json`; fails on p95/p99 drift.

### Configs

* **configs/svc-gateway.sample.toml** — Declarative knobs: timeouts, body cap, decode cap/ratio, RPS limit, DRR weights, amnesia, PQ/TLS policy toggles.
* **configs/.env.example** — Minimal env for local run (binds, metrics addr).

### Docs

* **IDB.md** — Invariants (caps before work; shed writes first), principles, gates, anti-scope.
* **INTEROP.md** — Protocols (HTTP/TLS, OAP/1 bounds), endpoints, DTOs, vectors.
* **CONFIG.md** — All knobs with safe defaults; `danger_ok` / non-prod escape hatch.
* **SECURITY.md** — STRIDE, macaroon caps, no on-disk keys, amnesia posture.
* **OBSERVABILITY.md** — Golden metrics, labels, dashboards, alerting hints.
* **PERFORMANCE.md** — SLOs, workloads, harness, regression gates.
* **RUNBOOK.md** — Incident playbooks, scaling, chaos guidance.
* **QUANTUM.md** — PQ posture (hybrid policy), HNDL risk, rollout.
* **API.md** — Route contracts, status codes, headers (`ETag`, `Retry-After`).
* **POLICY.md** — Residency/geo allow-lists, method allow-lists per tenant/class, abuse categories; admission reads from here.
* **diagrams/** — Mermaid: architecture, ingress sequence, readiness state.
* **dashboards/gateway_golden.json** — Example dashboard definition (panels/queries documented, vendor-neutral).

### Scripts

* **dev_run.sh** — Minimal local runner (reads `.env`, sample config).
* **soak_test.sh** — Long-running load to validate SLOs/heat behavior.
* **chaos_burst.sh** — Burst & downstream slowness to prove “shed writes first” and 429 behavior.
* **export_metrics.sh** — Quick scrape/export during manual checks.

### Examples

* **curl.http / httpie.http** — Paste-ready requests covering `GET /o/{addr}`, `Range`, `POST /put`, `/healthz`, `/readyz`, `/metrics`, with example headers and expected statuses.

### Benches

* **benches/README.md** — How to run + interpret benches; where baseline comes from.
* **benches/baseline.json** — Golden numbers; perf guard compares against this file.

### Tests

* **tests/vectors/** — Canonical interop vectors: OAP/1 frame round-trip, manifest (BLAKE3), error taxonomy.
* **tests/integration/readyz_degrade.rs** — Proves readiness sheds writes first under pressure.
* **tests/integration/caps_limits.rs** — Proves 1 MiB body cap, decode cap ≤ 8 MiB and ≤ 10× ratio.
* **tests/integration/interop_vectors.rs** — Asserts protocol invariants against vectors.
* **tests/integration/taxonomy_stability.rs** — Freezes reason→status mapping; fails on drift.
* **tests/integration/loom_readiness.rs** — Small loom scenario for readiness/admission interleavings (no locks across `.await`).

### Fuzz

* **fuzz/README.md** — Scope & corpus sources.
* **fuzz/targets/oap_frame.rs** — Fuzz length/envelope bounds (1 MiB invariant).
* **fuzz/targets/taxonomy_mapper.rs** — Fuzz error mapping; ensures deterministic, bounded outputs.

---

## Source Layout (small, single-purpose modules)

### Root

* **src/main.rs** — Wires CLI → config → TLS → router; attaches layers in the correct order; exposes control plane routes.
* **src/lib.rs** — Minimal public types for tests/examples; keep binary logic out.
* **src/consts.rs** — OAP/1 & transport bounds (1 MiB frames, ~64 KiB chunks), timeouts.
* **src/errors.rs** — Deterministic error taxonomy + canonical HTTP mapping.
* **src/result.rs** — Local Result alias + helpers.
* **src/state.rs** — Shared handles: metrics, readiness registry, forward clients; no durable state.

### CLI

* **src/cli/mod.rs** — Optional flags (binds, pq policy, enforce-payments when `econ` feature enabled).

### Config

* **src/config/mod.rs** — Config struct & validation entry points; safe defaults.
* **src/config/env.rs** — Env loaders for quick dev experience.
* **src/config/safety.rs** — Hard safety checks (caps, timeouts); rejects unsafe combos in prod.
* **src/config/amnesia.rs** — RAM-only posture, zeroization cadence, timers; ensures no disk spill.

### Observability

* **src/observability/mod.rs** — One import point.
* **src/observability/metrics.rs** — Golden counters/gauges/histograms (requests_total, latency_seconds, inflight, rejected_total{reason}, DRR queue depths, readiness flags).
* **src/observability/tracing.rs** — Tracing subscriber, trace context propagation.
* **src/observability/logging.rs** — JSON, UTC timestamps, redaction rules.

### Headers

* **src/headers/mod.rs** — Normalize/validate tracing & cache headers (traceparent, x-request-id, cache-control).
* **src/headers/etag.rs** — Helpers for ETag formatting (`"b3:<hex>"`) and validation.

### TLS / PQ

* **src/tls/mod.rs** — TLS 1.3 config via tokio_rustls::rustls; no native-tls fallback.
* **src/pq/mod.rs** — Feature-gated stubs for PQ posture (no crypto here).
* **src/pq/policy.rs** — Allowed TLS suites, hybrid toggle, and “suite drift” checks (used by config/ and CI).

### Readiness

* **src/readiness/mod.rs** — Readiness registry; exposes reasons; supports degrade.
* **src/readiness/keys.rs** — Canonical keys (downstream_ok, quotas_ok, decode_guard_ok, tls_ok).

### Policy (explicit, testable)

* **src/policy/mod.rs** — Policy trait & lightweight types; pure-logic, no I/O.
* **src/policy/residency.rs** — Region/geo allow/deny decisions driven by `docs/POLICY.md`.
* **src/policy/abuse.rs** — Abuse categories and scoring (credential stuffing, scraping, burst-writes) for tarpit/rate-limit decisions.

### Layers (Tower/Axum style)

* **src/layers/mod.rs** — Layer wiring hub.
* **src/layers/timeouts.rs** — Read/write/connect timeouts.
* **src/layers/body_caps.rs** — Enforce 1 MiB body cap (early 413).
* **src/layers/decode_guard.rs** — Enforce decoded cap ≤ 8 MiB and ≤ 10× ratio.
* **src/layers/rate_limit.rs** — Global RPS limit; emits `Retry-After`.
* **src/layers/drr.rs** — Per-tenant fair-queue (deficit round robin) + gauges.
* **src/layers/tarpit.rs** — Exponential delays for flagged abuse categories (separate from rate-limit).
* **src/layers/auth.rs** — Macaroon capability verification (no ambient trust).
* **src/layers/corr.rs** — Correlation-ID extraction/injection.

### Admission (decide before doing work)

* **src/admission/mod.rs** — Admission orchestrator; composes classifiers & policy checks.
* **src/admission/classifier.rs** — Cheap request classifier (read/write/admin/media); informs DRR/quotas/policy.
* **src/admission/capabilities.rs** — Capability checks and caveats (scope, TTL, paths).
* **src/admission/quotas.rs** — Tenant quotas; emits 429 with `Retry-After`.
* **src/admission/taxonomy.rs** — Central mapping to deterministic reasons/status codes.
* **src/admission/residency.rs** — Thin adapter to `policy::residency` (no logic creep).
* **src/admission/payments.rs** — (feature = "econ") enforce paid writes/prepaid quotas; emits econ-reasoned rejections.

### Routes (tiny handlers; layers do the heavy lifting)

* **src/routes/mod.rs** — Router assembly; attaches layers and mounts sub-routers.
* **src/routes/health.rs** — `/healthz` liveness.
* **src/routes/ready.rs** — `/readyz` with degrade signals; **sheds writes first**.
* **src/routes/metrics.rs** — `/metrics` Prometheus exposition.
* **src/routes/version.rs** — `/version` build metadata.
* **src/routes/objects.rs** — `GET /o/{addr}`; validates addressing; returns ETag.
* **src/routes/objects_range.rs** — `GET /o/{addr}` with strict `Range` support (read-only, bounded).

### Forward (downstream facades; keep thin)

* **src/forward/mod.rs** — Facade selectors & shared types.
* **src/forward/overlay_client.rs** — Primary forwarder to overlay; preserves correlation and maps errors deterministically.
* **src/forward/index_client.rs** — Optional: resolve helpers (addr → location) if interop requires a hint before overlay fetch.
* **src/forward/storage_client.rs** — Optional: read-only media proxy; range-reads only; keeps overlay client focused.

---

### Why this nails your canon (quick hits)

* **Policy made explicit** → fewer hidden rules in admission; easy to test.
* **Tarpit separate from rate-limit** → better Trust & Safety, less collateral damage.
* **Classifier first** → precise DRR/quotas; avoids over-enforcement.
* **PQ policy guard** → CI-enforceable TLS suite contract without adding crypto.
* **Range route (bounded)** → practical UX for media reads while staying stateless.
* **Stability gates** → taxonomy freeze, API/semver checks, perf guard, fuzz nightly, loom interleavings.

