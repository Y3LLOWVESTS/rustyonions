

---

# svc-interop — scaffold (no code yet)

```
svc-interop/
├─ Cargo.toml
├─ README.md
├─ CHANGELOG.md
├─ LICENSE-APACHE
├─ LICENSE-MIT
├─ Config.toml.example
├─ .env.example
├─ src/
│  ├─ main.rs
│  ├─ lib.rs
│  ├─ config/
│  │  └─ mod.rs
│  ├─ error.rs
│  ├─ hashing.rs
│  ├─ metrics.rs
│  ├─ readiness.rs
│  ├─ supervisor.rs
│  ├─ shutdown.rs
│  ├─ queues.rs
│  ├─ audit/
│  │  ├─ mod.rs
│  │  └─ sink.rs
│  ├─ middleware/
│  │  ├─ mod.rs
│  │  ├─ corr_id.rs
│  │  ├─ capability.rs
│  │  ├─ origin_pin.rs
│  │  ├─ rate_limit.rs
│  │  └─ decompress_guard.rs
│  ├─ routes/
│  │  ├─ mod.rs
│  │  ├─ health.rs
│  │  ├─ ready.rs
│  │  ├─ metrics.rs
│  │  ├─ version.rs
│  │  ├─ put.rs
│  │  ├─ get_object.rs
│  │  └─ webhooks/
│  │     ├─ mod.rs
│  │     ├─ github.rs
│  │     ├─ stripe.rs
│  │     └─ slack_webhook.rs
│  ├─ clients/
│  │  ├─ mod.rs
│  │  ├─ passport.rs
│  │  ├─ storage.rs
│  │  ├─ index.rs
│  │  └─ mailbox.rs
│  ├─ dto/
│  │  ├─ mod.rs
│  │  └─ types.rs
│  ├─ pq/
│  │  └─ mod.rs
│  └─ telemetry/
│     ├─ mod.rs
│     ├─ tracing.rs
│     └─ logging.rs
├─ tests/
│  ├─ smoke_http.rs
│  ├─ readiness.rs
│  ├─ reason_codes.rs
│  ├─ hashing_policy.rs
│  ├─ vectors.rs
│  └─ webhooks/
│     ├─ github_ping.rs
│     ├─ stripe_v1.rs
│     └─ slack_v0.rs
├─ tests/vectors/
│  ├─ vectors.json
│  └─ generator.rs
├─ benches/
│  └─ hashing_bench.rs
├─ fuzz/
│  ├─ fuzz_targets/
│  │  ├─ dto_parse.rs
│  │  ├─ webhook_sig.rs
│  │  └─ decompression.rs
│  └─ Cargo.toml
├─ docs/
│  ├─ arch.mmd
│  ├─ sequence.mmd
│  ├─ state.mmd
│  ├─ openapi/
│  │  └─ svc-interop.openapi.yaml
│  ├─ api-history/
│  │  ├─ http/
│  │  │  └─ v1.yaml
│  │  └─ rust/
│  │     └─ svc-interop/
│  │        └─ v1.txt
│  ├─ IDB.md
│  ├─ CONCURRENCY.md
│  ├─ CONFIG.md
│  ├─ INTEROP.md
│  ├─ OBSERVABILITY.md
│  ├─ PERFORMANCE.md
│  ├─ QUANTUM.md
│  ├─ RUNBOOK.md
│  └─ SECURITY.md
├─ .github/
│  └─ workflows/
│     ├─ ci.yml
│     ├─ render-mermaid.yml
│     └─ concurrency-guardrails.yml
├─ ops/
│  ├─ alerts/
│  │  └─ prometheus-rules.yaml
│  └─ dashboards/
│     └─ svc-interop.json
├─ scripts/
│  ├─ dev-run.sh
│  ├─ regen_vectors.rs
│  ├─ openapi_diff.sh
│  └─ perf_mixed.sh
└─ Makefile.toml
```

---

## What each file/dir is for (tight, actionable)

### Top level

* **Cargo.toml** — Crate metadata, features (`libapi`, `tls`, `pq`, `blake3-simd`, `cli`), workspace pins; no giant `[dependencies]`—just what this crate owns.
* **README.md** — Your already-filled, **role-aware** service README (architecture, sequences, states, SLOs, metrics, gates).
* **CHANGELOG.md** — SemVer history (HTTP + Rust/libapi diffs, reason-code adds, breaking notes).
* **LICENSE-*** — Dual licensing: MIT / Apache-2.0.
* **Config.toml.example** — Minimal runnable config with safe defaults & comments mirroring `docs/CONFIG.md`.
* **.env.example** — Env-first posture: bind/metrics, provider secrets *by name* (no secrets committed).

### src/ (small files, single concern each)

* **main.rs** — Thin bootstrap: parse config/flags, init tracing/metrics, start supervisor; *no business logic* here.
* **lib.rs** — Gated `libapi` surface: re-export `Service`, `ServiceOptions`, `run()`; deny missing docs for any public items when `libapi` is on.

#### Core building blocks

* **config/mod.rs** — Typed config + defaults + validation (fail-closed rules: body caps, decompression ratio, hashing policy BLAKE3-only internal, amnesia guards).
* **error.rs** — Central error taxonomy → stable wire `reason` mapping; no HTTP in business logic.
* **hashing.rs** — **BLAKE3-256** canonical helpers; explicit **edge-only** HMAC-SHA256 verification adapters (no SHA digests leave this module).
* **metrics.rs** — Registry & canonical series (request latency, rejected_total{reason}, queue_depth, webhook_verify_total, etc.) with low-cardinality labels.
* **readiness.rs** — Liveness/readiness keys, brownout flag; JSON payload builder for `/readyz`.
* **supervisor.rs** — Task tree owner: spawn listener/workers/sentinel/exporters; restart policy/backoff; crash-loop guard.
* **shutdown.rs** — Cooperative cancellation primitives (watch channel, drain deadlines).
* **queues.rs** — Bounded mpsc/broadcast constructors + gauge emitters + consistent `Busy` semantics.

#### Audit

* **audit/mod.rs** — Audit event model (redacted), emit helpers (reason, corr_id, b3_hash).
* **audit/sink.rs** — Sinks: memory|stdout|file (file forbidden in amnesia); drop-oldest with `audit_drop_total`.

#### Middleware (each is a few, focused lines)

* **middleware/mod.rs** — Layer wiring helpers (tower stack composer).
* **middleware/corr_id.rs** — `X-Corr-ID` extract/generate + response echo.
* **middleware/capability.rs** — Capability translation (macaroon verify/mint) for mutating routes; TTL ≤ 60s.
* **middleware/origin_pin.rs** — Host allow-list + provider signature timestamp/skew checks.
* **middleware/rate_limit.rs** — Token bucket limiter (max/burst) with `RATE_LIMIT` reason.
* **middleware/decompress_guard.rs** — Ratio cap + absolute cap; `DECOMP_LIMIT` reason on overflow.

#### Routes (each endpoint in its own file)

* **routes/mod.rs** — Axum router wiring; no handler bodies here.
* **routes/health.rs** — `/healthz` fast liveness (no deps).
* **routes/ready.rs** — `/readyz` reads readiness keys + brownout state.
* **routes/metrics.rs** — `/metrics` Prometheus exposition (bind to localhost by config).
* **routes/version.rs** — `/version` build/scm/rustc info.
* **routes/put.rs** — `POST /put` ingress; stream >1MiB in 64KiB chunks, compute **BLAKE3**.
* **routes/get_object.rs** — `GET /o/:addr` address-validated fetch; strict `b3:<hex>` parsing.
* **routes/webhooks/mod.rs** — Common glue for providers + dispatch based on `:provider`.
* **routes/webhooks/github.rs** — GitHub adapter (HMAC-SHA256 `sha256=<hex>` verify **only at edge**), dedup/idempotency envelope + audit emit.
* **routes/webhooks/stripe.rs** — Stripe adapter (v1/v0 header parsing), edge-verify + envelope.
* **routes/webhooks/slack_webhook.rs** — Slack v0 signature + ts; replay window checking.

#### Downstream clients (each service in a tiny client)

* **clients/mod.rs** — Re-exports + typed error mapping for RPC outcomes.
* **clients/passport.rs** — `svc-passport` capability mint/verify (deadline + retries for idempotent ops).
* **clients/storage.rs** — `svc-storage` blob put/get by **BLAKE3** address.
* **clients/index.rs** — `svc-index` lookups/metadata paths.
* **clients/mailbox.rs** — `svc-mailbox` enqueue/ack with visibility timeouts; at-least-once contract.

#### DTOs & types

* **dto/mod.rs** — Public DTO module: deny-unknown-fields, wire-stable error body, Put JSON shape.
* **dto/types.rs** — Internal manifests (e.g., `ObjectManifestV2`), minimal shared structs.

#### PQ hooks (feature-gated, tiny)

* **pq/mod.rs** — Counters/labels for PQ adoption; optional toggles (hybrid KEX signal); no crypto implementations here.

#### Telemetry glue

* **telemetry/mod.rs** — Bring-up for logs + tracing + OTEL (feature).
* **telemetry/tracing.rs** — Span names/fields helpers, parent_or_else sampling.
* **telemetry/logging.rs** — JSON formatter + redaction rules.

### tests/ (small, focused integration & golden)

* **smoke_http.rs** — Boot service with defaults; asserts `/healthz`, `/metrics`, `/version`.
* **readiness.rs** — Simulate deps down/brownout → `/readyz` flips; read stays green while write browns out.
* **reason_codes.rs** — Every non-2xx path returns wire-stable `{error,reason,corr_id}`; closed taxonomy check.
* **hashing_policy.rs** — Proves **BLAKE3-only internal**; ensures no persisted SHA256 artifacts.
* **vectors.rs** — Confirms generated test vectors match docs; reuses `tests/vectors/vectors.json`.
* **webhooks/**… — Provider-specific conformance (GitHub ping, Stripe v1/v0, Slack v0), edge verify only.

### tests/vectors/

* **vectors.json** — Canonical BLAKE3 digests/idempotency examples; regenerated by CI.
* **generator.rs** — Deterministic vector builder; single source of truth for hashes/fixtures.

### benches/

* **hashing_bench.rs** — Criterion microbench (BLAKE3 64KiB..1MiB); basis for perf regression gate.

### fuzz/

* **fuzz_targets/dto_parse.rs** — Fuzz JSON DTOs (deny-unknown-fields, length caps).
* **fuzz_targets/webhook_sig.rs** — Fuzz base strings/headers for signature verifiers.
* **fuzz_targets/decompression.rs** — Zip-bomb/ratio guards.
* **Cargo.toml** — `cargo-fuzz` config; tiny target deps only.

### docs/

* **arch.mmd** — Flowchart (edge → interop → deps + metrics bus).
* **sequence.mmd** — Common path sequence (webhook → verify → b3 → enqueue/audit).
* **state.mmd** — Service lifecycle/brownout states.
* **openapi/svc-interop.openapi.yaml** — Canonical HTTP contract snapshot (diffed in CI).
* **api-history/http/v1.yaml** — Frozen HTTP snapshot for SemVer diffs.
* **api-history/rust/svc-interop/v1.txt** — `cargo public-api` snapshot when `libapi` feature is on.
* **IDB.md, CONCURRENCY.md, CONFIG.md, INTEROP.md, OBSERVABILITY.md, PERFORMANCE.md, QUANTUM.md, RUNBOOK.md, SECURITY.md** — Your combined crate docs, kept co-located for auditors and CI gates.

### .github/workflows/

* **ci.yml** — fmt/clippy/tests/deny/public-api/openapi-lint/coverage; fails on drift or missing docs for pub items (when `libapi`).
* **render-mermaid.yml** — Auto-render `*.mmd` → `*.svg` on PRs; artifacts for diffs.
* **concurrency-guardrails.yml** — Loom (PR only), fuzz build, clippy `await_holding_lock` gate.

### ops/

* **alerts/prometheus-rules.yaml** — SLO alerts (latency p95, 5xx rate, brownout, queue pressure, webhook verify fails).
* **dashboards/svc-interop.json** — Golden dashboard (RED/USE, queues, brownout, verify counts, RPC p95).

### scripts/

* **dev-run.sh** — Local launcher with sane env vars; no secrets; prints endpoints.
* **regen_vectors.rs** — Rebuilds `tests/vectors/vectors.json` deterministically.
* **openapi_diff.sh** — Diffs `docs/openapi/*.yaml` against latest snapshot; intended for CI step too.
* **perf_mixed.sh** — Local load: 50/30/20 GET/webhook/PUT mix to sanity-check SLOs.

### Makefile.toml

* Task aliases (cargo-make): `lint`, `test`, `fuzz:build`, `perf:local`, `docs:render`, `api:check`.

---

## Why this shape (quick rationale)

* **Small files, single purpose:** Each HTTP route, middleware, provider adapter, and client has a dedicated file—easy to read, test, and replace.
* **Edge-only SHA-256:** All SHA-256 logic is isolated to webhook adapters; everything else speaks **BLAKE3** internally.
* **Proof-first:** Tests/golden vectors/openapi & public-api snapshots lock the contract. CI gates catch drift.
* **Separation of concerns:** `routes` (I/O shape) vs `clients` (deps) vs `middleware` (policy/limits) vs `audit/metrics/readiness` (ops).

