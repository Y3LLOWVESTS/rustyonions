
---

# File tree (final)

```
svc-index/
├─ Cargo.toml
├─ rust-toolchain.toml
├─ README.md
├─ CHANGELOG.md
├─ LICENSE-APACHE
├─ LICENSE-MIT
├─ .editorconfig
├─ .gitignore
├─ deny.toml
├─ .cargo/
│  └─ config.toml
├─ configs/
│  ├─ dev.env.example
│  ├─ prod.env.example
│  └─ Config.toml.example
├─ docs/
│  ├─ README_LINKS.md
│  ├─ IDB.md
│  ├─ CONFIG.md
│  ├─ OBSERVABILITY.md
│  ├─ SECURITY.md
│  ├─ PERFORMANCE.md
│  ├─ QUANTUM.md
│  ├─ RUNBOOK.md
│  ├─ API.md
│  ├─ TESTS.md
│  ├─ arch.mmd
│  ├─ sequence.mmd
│  ├─ state.mmd
│  └─ openapi/
│     └─ svc-index.v1.yaml
├─ benches/
│  ├─ resolve.rs
│  └─ datasets/
│     ├─ keys.txt
│     └─ regions.txt
├─ scripts/
│  ├─ smoke.sh
│  ├─ mermaid_render.sh
│  ├─ bench_resolve.sh
│  ├─ soak.sh
│  └─ chaos_inject.sh
├─ fuzz/
│  ├─ Cargo.toml
│  ├─ fuzz.toml
│  └─ fuzz_targets/
│     ├─ http_resolve.rs
│     └─ parse_manifest.rs
├─ tests/
│  ├─ http_contract.rs
│  ├─ prop_index.rs
│  ├─ loom_index.rs
│  ├─ integration.rs
│  ├─ chaos.rs
│  └─ golden/
│     └─ resolve/
│        ├─ ok_basic.json
│        ├─ not_found.json
│        ├─ providers_ranked.json
│        └─ over_capacity.json
├─ examples/
│  └─ client.rs
├─ src/
│  ├─ lib.rs
│  ├─ main.rs
│  ├─ app.rs
│  ├─ router.rs
│  ├─ error.rs
│  ├─ types.rs
│  ├─ config.rs
│  ├─ logging.rs
│  ├─ telemetry.rs
│  ├─ constants.rs
│  ├─ utils/
│  │  └─ timeouts.rs
│  ├─ state/
│  │  ├─ mod.rs
│  │  ├─ readiness.rs
│  │  ├─ metrics.rs
│  │  └─ shutdown.rs
│  ├─ http/
│  │  ├─ mod.rs
│  │  ├─ extractors/
│  │  │  ├─ corr_id.rs
│  │  │  ├─ capability.rs
│  │  │  └─ limits.rs
│  │  ├─ middleware/
│  │  │  ├─ body_limits.rs
│  │  │  ├─ decompress_guard.rs
│  │  │  ├─ rate_limit.rs
│  │  │  └─ trace_layer.rs
│  │  └─ routes/
│  │     ├─ health.rs
│  │     ├─ metrics.rs
│  │     ├─ version.rs
│  │     ├─ resolve.rs
│  │     ├─ providers.rs
│  │     └─ admin.rs
│  ├─ pipeline/
│  │  ├─ mod.rs
│  │  ├─ resolve.rs
│  │  └─ providers.rs
│  ├─ dht/
│  │  ├─ mod.rs
│  │  ├─ client.rs
│  │  ├─ hedge.rs
│  │  └─ rank.rs
│  ├─ cache/
│  │  ├─ mod.rs
│  │  ├─ manifest.rs
│  │  ├─ providers.rs
│  │  └─ negative.rs
│  ├─ store/
│  │  ├─ mod.rs
│  │  ├─ sled_store.rs
│  │  ├─ keys.rs
│  │  └─ schema.rs
│  ├─ auth/
│  │  ├─ mod.rs
│  │  ├─ caps.rs
│  │  └─ uds_allow.rs
│  ├─ audit/
│  │  ├─ mod.rs
│  │  └─ events.rs
│  ├─ bus/
│  │  ├─ mod.rs
│  │  └─ events.rs
│  └─ net/
│     ├─ mod.rs
│     ├─ listener.rs
│     ├─ uds.rs
│     └─ tls.rs
└─ .github/
   └─ workflows/
      ├─ ci.yml
      ├─ render-mermaid.yml
      ├─ fuzz.yml
      ├─ coverage.yml
      ├─ perf-regress.yml
      └─ chaos.yml
```

---

# Descriptions (every file)

## Root & meta

* **Cargo.toml** — Crate metadata, features (`facets`, `otel`), deps pinned to workspace canon.
* **rust-toolchain.toml** — MSRV pin (1.80.0) for reproducible builds.
* **README.md** — Service contract (scope, SLOs, metrics, endpoints, diagrams, gates).
* **CHANGELOG.md** — SemVer history; breaking changes + migration notes.
* **LICENSE-APACHE / LICENSE-MIT** — Dual license texts.
* **.editorconfig** — Uniform whitespace/encoding across editors.
* **.gitignore** — Ignores `target/`, temp DBs, env files, coverage outputs.
* **deny.toml** — `cargo-deny` policy (licenses/bans/advisories/sources) to lock supply chain.
* **.cargo/config.toml** — Local build defaults (e.g., `-D warnings`, resolver settings).

## configs/

* **dev.env.example** — Dev defaults (localhost binds, liberal logs, safe caps).
* **prod.env.example** — Production-tuned caps (tighter inflight/deadlines, minimal logs).
* **Config.toml.example** — Optional TOML override; mirrors env knobs.

## docs/

* **README_LINKS.md** — Centralized links to crate docs, diagrams, OpenAPI.
* **IDB.md** — Invariants (MUST/MUST NOT) + acceptance proofs (tests/metrics).
* **CONFIG.md** — All config knobs: types, defaults, precedence, examples.
* **OBSERVABILITY.md** — Metrics, logs, traces; scraping and alert guidance.
* **SECURITY.md** — STRIDE, caps, capability model, UDS peer-cred, amnesia mode.
* **PERFORMANCE.md** — Bench harness, datasets, SLO reproduction steps.
* **QUANTUM.md** — PQ posture (TLS hybrid KEM plan; PQ sigs for tokens).
* **RUNBOOK.md** — On-call workflows (symptoms → metrics → fixes).
* **API.md** — HTTP/UDS surface details (headers, cache hints, error taxonomy).
* **TESTS.md** — Per-crate test gates (unit/prop/loom/fuzz/golden/coverage).
* **arch.mmd / sequence.mmd / state.mmd** — Mermaid sources for README diagrams.
* **openapi/svc-index.v1.yaml** — OpenAPI spec for client generation and contract testing.

## benches/

* **resolve.rs** — Criterion bench of resolve path (cache hot/cold, α/β hedging).
* **datasets/keys.txt** — Canonical list of hot & cold keys for reproducible latency.
* **datasets/regions.txt** — Region mix for ranking/provider tests during benches.

## scripts/

* **smoke.sh** — One-shot local smoke: boot, `/healthz`, `/readyz`, basic `resolve`.
* **mermaid_render.sh** — Render all `.mmd` diagrams to `.svg` locally.
* **bench_resolve.sh** — Wrapper to run Criterion with standard flags/datasets.
* **soak.sh** — 15–30 min soak (RPS ramp + jitter) writing histograms to `artifacts/`.
* **chaos_inject.sh** — Flip chaos knobs (e.g., DHT tail %, delay) to repro failures quickly.

## fuzz/

* **Cargo.toml** — Fuzz workspace with dependencies isolated from main crate.
* **fuzz.toml** — Seeds and time budgets; pulls from golden vectors.
* **fuzz_targets/http_resolve.rs** — Fuzz HTTP resolver parsing and hardening caps.
* **fuzz_targets/parse_manifest.rs** — Fuzz manifest DTO deserialization + BLAKE3 validation path.

## tests/

* **http_contract.rs** — Black-box HTTP: status codes, headers, ETag/cache hints, readiness truth.
* **prop_index.rs** — Property tests: idempotent ranking, TTL/freshness monotonicity, deadline budgeting.
* **loom_index.rs** — Loom concurrency scenarios for hedging/cancellation/caches.
* **integration.rs** — E2E boot with mocked DHT + temp sled; full pipeline assertions.
* **chaos.rs** — Fault injection (peer flaps, high tail latency, miss storms) with metrics assertions.
* **golden/resolve/ok_basic.json** — Golden: happy path (deterministic output check).
* **golden/resolve/not_found.json** — Golden: 404 with correct body + headers.
* **golden/resolve/providers_ranked.json** — Golden: provider ranking determinism.
* **golden/resolve/over_capacity.json** — Golden: inflight/size caps → structured 429/413.

## examples/

* **client.rs** — Minimal typed client usage (DTOs via `ron-proto`), rustdoc-tested.

## src/ (top level)

* **lib.rs** — Public surface (re-exports: `Config`, `Metrics`, `HealthState`, `build_router`, `wait_for_ctrl_c()`).
* **main.rs** — Binary entry (load config, init tracing, build state, start listeners, graceful shutdown).
* **app.rs** — App bootstrap wiring (config → state; readiness gates; background tasks).
* **router.rs** — Axum router build (routes + middleware; feature-gated mounts).
* **error.rs** — Error taxonomy + HTTP mapping (NotFound/OverCapacity/TooLarge/Timeout/Unauthorized).
* **types.rs** — Small internal types/newtypes shared across modules.
* **config.rs** — Strongly-typed loader with env-first precedence and validation.
* **logging.rs** — Tracing/log setup; correlating `X-Corr-ID`; target level map.
* **telemetry.rs** — Prometheus registry/exporter, OTEL (feature `otel`), metric init.
* **constants.rs** — Single source for invariants: 1 MiB OAP cap, 10:1 decompress guard, deadlines, inflight limits.
* **utils/timeouts.rs** — Deadline budgeting, jitter helpers (keeps pipeline clean).

## src/state/

* **mod.rs** — `AppState` (clients, caches, metrics, config handles) + constructor.
* **readiness.rs** — Truthful readiness (DB open, DHT reachable, bus health).
* **metrics.rs** — Struct of all counters/histograms/gauges; registration and labels.
* **shutdown.rs** — Ctrl-C handler, graceful drain, task cancellation.

## src/http/

* **mod.rs** — HTTP prelude; route registry function.
* **extractors/corr_id.rs** — Extract/assign correlation ID and inject into spans.
* **extractors/capability.rs** — Capability/macaron parsing & auth error mapping.
* **extractors/limits.rs** — Per-request limit extraction (size/timeout overrides).
* **middleware/body_limits.rs** — Enforce request/response byte caps.
* **middleware/decompress_guard.rs** — Enforce max decompression ratio (zip bomb guard).
* **middleware/rate_limit.rs** — Token/leaky-bucket ingress limiter.
* **middleware/trace_layer.rs** — Uniform tracing spans/fields per route.
* **routes/health.rs** — `/healthz` (fast) and `/readyz` (truthful) endpoints.
* **routes/metrics.rs** — `/metrics` exposition (Prometheus text format).
* **routes/version.rs** — `/version` (git SHA, features, MSRV).
* **routes/resolve.rs** — `GET /resolve/{key}` orchestration + caching headers/ETag.
* **routes/providers.rs** — `GET /providers/{cid}`—ranked provider set; region/freshness filters.
* **routes/admin.rs** — Capability-gated admin ops (reindex/pin/unpin) with audit emits.

## src/pipeline/

* **mod.rs** — Pipeline traits/constants; error types; shared helpers.
* **resolve.rs** — Name/`b3:<hex>` → manifest → providers flow; cache-first; α/β hedging under global deadline; metrics.
* **providers.rs** — Provider set fetch/refine; enforce limits (count, region, freshness).

## src/dht/

* **mod.rs** — DHT client trait and prelude (mockable seam).
* **client.rs** — Thin RPC client to `svc-dht`; translate errors and timeouts.
* **hedge.rs** — α/β hedging, jitter strategy, loser cancellation, histograms.
* **rank.rs** — Deterministic provider ranking (region, freshness, hints).

## src/cache/

* **mod.rs** — Cache traits + key strategy (manifest/providers/negative).
* **manifest.rs** — Manifest-pointer cache (TTL, ETag semantics).
* **providers.rs** — Provider set cache (size/TTL bounded; qualifiers respected).
* **negative.rs** — Negative cache for recent misses to dampen storms.

## src/store/

* **mod.rs** — Store trait (get/put/scan) + schema version gate.
* **sled_store.rs** — Sled-backed store; enforces single-DB-root invariant.
* **keys.rs** — Keyspace layout (prefixes for names, cids, pointers, metadata).
* **schema.rs** — Schema versions + migrations; reject below `min_supported`.

## src/auth/

* **mod.rs** — Auth prelude + errors.
* **caps.rs** — Capability verification policies; admin guard helpers.
* **uds_allow.rs** — Optional UDS allowlist (peer credentials) for local sockets.

## src/audit/

* **mod.rs** — Audit facade; dependency injection for tests.
* **events.rs** — Admin-event construction and bus emission to `ron-audit`.

## src/bus/

* **mod.rs** — Kernel bus integration (common types, wiring hooks).
* **events.rs** — Emit/subscribe to bus events (health, config-update, audit relay).

## src/net/

* **mod.rs** — Networking prelude.
* **listener.rs** — HTTP listener bootstrap (binds, graceful shutdown hooks).
* **uds.rs** — UDS server setup (paths, perms, peer-cred extraction).
* **tls.rs** — TLS server config via `tokio_rustls::rustls::ServerConfig` (not plain rustls).

## .github/workflows/

* **ci.yml** — fmt, clippy (`-D warnings`), unit/prop/doc tests, `cargo-deny`, smoke.
* **render-mermaid.yml** — Render `docs/*.mmd` → `docs/*.svg`, upload artifacts.
* **fuzz.yml** — Nightly/on-demand fuzz jobs with budgets; artifact on crash.
* **coverage.yml** — Coverage via `cargo-llvm-cov`; fail-under (e.g., 85%).
* **perf-regress.yml** — Criterion run vs baseline; fail on >10% p95 regression.
* **chaos.yml** — Inject latency/drop in mocked DHT during tests; assert readiness & rejects.

---

### Why this is “done-done”

* **Modular & maintainable:** Every concern is isolated; handlers/pipeline/store/DHT/cache never sprawl.
* **Testable by design:** Seams for unit/prop/golden/loom/fuzz + integration/chaos; CI gates wire it shut.
* **Canon-aligned:** BLAKE3, OAP caps, amnesia mode, truthful readiness, single sled root, rustls via tokio.
* **Ops-ready:** RUNBOOK, soak/chaos scripts, perf baselines, coverage threshold, deny policy.

