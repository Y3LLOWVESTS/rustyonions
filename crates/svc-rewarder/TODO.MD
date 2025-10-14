

---

# `crates/svc-rewarder/` — Proposed File Tree & Roles

```
svc-rewarder/
├─ Cargo.toml
├─ README.md
├─ configs/
│  └─ svc-rewarder.toml
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
│  ├─ TESTS.md
│  ├─ openapi/
│  │  └─ svc-rewarder.json
│  ├─ schemas/
│  │  ├─ compute.request.v1.json
│  │  └─ manifest.v1.json
│  └─ mermaid/
│     ├─ arch.mmd
│     ├─ sequence.mmd
│     └─ state.mmd
├─ src/
│  ├─ main.rs
│  ├─ lib.rs
│  ├─ prelude.rs
│  ├─ config/
│  │  ├─ mod.rs
│  │  ├─ types.rs
│  │  ├─ load.rs
│  │  └─ validate.rs
│  ├─ http/
│  │  ├─ mod.rs
│  │  ├─ routes.rs
│  │  ├─ handlers.rs
│  │  ├─ dto.rs
│  │  └─ error.rs
│  ├─ core/
│  │  ├─ mod.rs
│  │  ├─ compute.rs
│  │  ├─ invariants.rs
│  │  └─ algebra.rs
│  ├─ inputs/
│  │  ├─ mod.rs
│  │  ├─ accounting.rs
│  │  ├─ policy.rs
│  │  ├─ ledger_snapshot.rs
│  │  ├─ cid.rs
│  │  └─ cache.rs
│  ├─ outputs/
│  │  ├─ mod.rs
│  │  ├─ manifest.rs
│  │  ├─ artifacts.rs
│  │  ├─ intents.rs
│  │  └─ attestation.rs
│  ├─ metrics/
│  │  └─ mod.rs
│  ├─ readiness/
│  │  ├─ mod.rs
│  │  └─ health.rs
│  ├─ bus/
│  │  ├─ mod.rs
│  │  └─ events.rs
│  ├─ security/
│  │  ├─ mod.rs
│  │  ├─ caps.rs
│  │  ├─ tls.rs
│  │  └─ pq.rs
│  ├─ concurrency.rs
│  ├─ errors.rs
│  ├─ telemetry.rs
│  └─ util/
│     ├─ bytes.rs
│     └─ timeouts.rs
├─ tests/
│  ├─ unit/
│  │  ├─ invariants.rs
│  │  ├─ idempotency.rs
│  │  └─ config.rs
│  ├─ integration/
│  │  ├─ http_compute.rs
│  │  ├─ readiness.rs
│  │  └─ egress_dedupe.rs
│  └─ vectors/
│     └─ v1/
│        ├─ happy-001.json
│        ├─ dup-epoch.json
│        └─ quarantine-overflow.json
├─ benches/
│  └─ reward_calc.rs
└─ xtask/
   └─ mod.rs
```

## What each file/dir is for (and why it exists)

### Root & docs

* **`Cargo.toml`** — Declares service targets, features (`tls`, `pq-hybrid`, `pq-sign`, `cli`), and workspace deps. Keeps feature flags explicit for PQ toggles and TLS. 
* **`README.md`** — Your refined, consumer-facing contract: overview, interfaces, SLOs, diagrams, and quick start. Mirrors the HTTP surface and SLO gates. 
* **`configs/svc-rewarder.toml`** — Example config with sane defaults (binds, timeouts, caps). Used in smoke runs and CI fixtures. 
* **`docs/*.MD`** — The 12 canonical docs collated under the crate (one per topic), plus:

  * `openapi/svc-rewarder.json` — machine-readable HTTP contract (CI-diffed). 
  * `schemas/*.json` — JSON Schemas for request & manifest for SDK validation and doctests. 
  * `mermaid/*.mmd` — source for the three mandatory diagrams, rendered in CI. 

### Executable surfaces

* **`src/main.rs`** — Thin bootstrap: parse config (optionally via CLI), construct server, run tokio runtime, wire shutdown. All logic lives behind library modules to keep `main.rs` tiny. 
* **`src/lib.rs`** — Re-exports: `Config`, `Metrics`, `HealthState`, `Bus`, `KernelEvent`, helpers. Enforces a tidy public surface per IDB. 
* **`src/prelude.rs`** — Small `pub use` collection for common types (Result, tracing macros, anyhow/thiserror), minimizing import noise (DX concern). 

### Configuration

* **`src/config/types.rs`** — Strongly-typed config structs (binds, limits, timeouts, rewarder knobs, ingress endpoints, amnesia, PQ). Defaults map to documented values; serde + humantime parsing. 
* **`src/config/load.rs`** — Merge precedence (defaults ← file ← env ← CLI). URL & path validation and friendly errors. Hot-reload hooks (SIGHUP) emit bus event. 
* **`src/config/validate.rs`** — Central fail-closed checks (body cap ≥1 KiB, decompression ratio ≥1, PQ policy, amnesia/tmpfs rules). Keeps validation decoupled and testable. 
* **`src/config/mod.rs`** — Exposes `Config` API and reload entry points. 

### HTTP layer

* **`src/http/routes.rs`** — Route table for v1 (`/rewarder/epochs/{id}/compute`, `/epochs/{id}`, health/ready/metrics/version/openapi/schema`). Keeps endpoint plumbing separate from handlers. 
* **`src/http/handlers.rs`** — Thin adapters that: validate frame/size caps, parse DTOs, hand off to scheduler/compute, and shape responses (including 409 quarantine vs 200/202). 
* **`src/http/dto.rs`** — Request/response DTOs with `serde(deny_unknown_fields)` matching the OpenAPI/Schema (stable wire contract). 
* **`src/http/error.rs`** — Uniform error envelope (`error.code`, `details.reason`, `corr_id`) + status mapping per taxonomy. 
* **`src/http/mod.rs`** — Axum server builder (middleware: tracing, limits, corr-id propagation). Keeps HTTP concerns isolated. 

### Core (pure math & invariants)

* **`src/core/compute.rs`** — Deterministic reward calculus: transforms sealed inputs + policy into allocation set; pure, no I/O, easy to bench/fuzz. 
* **`src/core/invariants.rs`** — Enforces conservation (`Σ payouts ≤ pool`), non-negative, overflow checks; returns `Quarantine` classification on breach. 
* **`src/core/algebra.rs`** — Money-safe arithmetic (minor units / fixed-point helpers). Strictly integer math per IDB. 
* **`src/core/mod.rs`** — Facade combining compute + invariants with a small result type that the outer pipeline consumes. 

### Inputs (sealed snapshots)

* **`src/inputs/accounting.rs`** — Fetches/validates accounting snapshot via content address or URL; enforces size/decompression caps. 
* **`src/inputs/policy.rs`** — Retrieves signed policy doc; verifies `policy_id/hash` pair and signature; caches per TTL. 
* **`src/inputs/ledger_snapshot.rs`** — Optional reads of ledger state if the algorithm needs it (sealed/cached). 
* **`src/inputs/cid.rs`** — Parser and helpers for `b3:<hex>` content addresses and canonicalization (deterministic run key). 
* **`src/inputs/cache.rs`** — Small in-memory cache (bounded, TTL) for snapshots/policies to hit SLOs safely. 
* **`src/inputs/mod.rs`** — Unified interface to obtain a sealed `Snapshot` used by `core`. 

### Outputs (artifacts & intents)

* **`src/outputs/manifest.rs`** — Canonical manifest struct; computes `commitment` hash over sorted JSON; version-gated schema. 
* **`src/outputs/artifacts.rs`** — Writes `run.json`, `commitment.txt`, optional zk proof files; respects amnesia mode (tmpfs / cleanup). 
* **`src/outputs/intents.rs`** — Emits idempotent settlement intent to `ron-ledger` with `run_key`; classifies `accepted|dup|error`. Retries only for idempotent paths with budgeted deadlines. 
* **`src/outputs/attestation.rs`** — Signs manifest (Ed25519 + optional PQ ML-DSA) and attaches attestations; integrates with KMS/HSM where configured. 
* **`src/outputs/mod.rs`** — Facade returning a small `EgressOutcome` consumed by handlers. 

### Metrics, readiness, bus, security

* **`src/metrics/mod.rs`** — One place to register **all** counters/gauges/histograms (golden: `reward_compute_latency_seconds`, `ledger_intents_total{result}`, `rejected_total{reason}`, queue depth, degraded flags). Panics on double-register to enforce uniqueness. 
* **`src/readiness/health.rs`** — `/healthz`, `/readyz` implementations; readiness keys (`ledger_ok`, `policy_registry_ok`, `queue_ok`, etc.) and degraded JSON body with `retry_after`. 
* **`src/readiness/mod.rs`** — Exposes the readiness gate API; flips to NOT READY under backpressure or dep outage. 
* **`src/bus/events.rs`** — Minimal event shapes (`run.started`, `run.quarantined`, `run.completed`) with low cardinality; broadcast publisher. 
* **`src/bus/mod.rs`** — In-proc bus wiring (tokio broadcast) for lifecycle and audit notifications. 
* **`src/security/caps.rs`** — Capability (macaroon) verification and scope/caveat checks; never forwards ambient JWTs downstream. 
* **`src/security/tls.rs`** — rustls server config builder; TLS 1.3 only; deploy-pinned ciphers. 
* **`src/security/pq.rs`** — PQ toggles (hybrid KEX, PQ signatures), downgrade detection, and readiness policy when `pq_only`. 

### Control plane & cross-cutting

* **`src/concurrency.rs`** — Declares bounded channels (`work_req`, `work`, `results`, `intents`) and the supervisor/worker topology; spawn patterns and shutdown fanout. 
* **`src/errors.rs`** — Error taxonomy enums and `thiserror` impls mapped to HTTP envelope; `Busy`, `Timeout`, `Quarantine`, etc. 
* **`src/telemetry.rs`** — Tracing layers (JSON logs), span fields, corr-id propagation, optional OTEL exporter behind feature flag. 
* **`src/util/bytes.rs`** — Small helpers for framed I/O and safe decompression guard (ratio caps). 
* **`src/util/timeouts.rs`** — Deadline/timeout helpers for outbound calls and end-to-end budgets. 

### Tests & benches

* **`tests/unit/invariants.rs`** — Property tests proving conservation/non-negative/no overflow across randomized vectors. 
* **`tests/unit/idempotency.rs`** — Same `(epoch, policy_hash, inputs_cid)` ⇒ identical manifest + `run_key`; second emit is `dup`. 
* **`tests/unit/config.rs`** — Fail-closed validations (TLS with missing keys, amnesia/tmpfs rules, URL schemes). 
* **`tests/integration/http_compute.rs`** — End-to-end compute happy path; 200/202; schema round-trip against `schemas/compute.request.v1.json`. 
* **`tests/integration/readiness.rs`** — Induced dep outages/backpressure flip `/readyz` to 503 with correct `missing` keys. 
* **`tests/integration/egress_dedupe.rs`** — Proves `accepted` then `dup` behavior at the ledger boundary. 
* **`tests/vectors/v1/*.json`** — Canonical good/bad/dedup vectors (frozen and signed) used by CI and perf baselines. 
* **`benches/reward_calc.rs`** — Criterion microbench of the pure calculus; tracks p95/p99 and allocs/op for S/M/L scales. 

### Developer tooling

* **`xtask/mod.rs`** — Small automation: render Mermaid to SVG, validate OpenAPI diff, check presence of golden metrics, and enforce “no floats in money paths” grep. Keeps CI steps scriptable. 

---

## Why this structure works (tie-backs)

* **Stable HTTP surface** is isolated under `src/http/*`, matching the OpenAPI contract and error schema; compute stays pure for determinism and perf. 
* **Concurrency & backpressure** are explicit and centrally declared, matching the supervisor/queues topology and acceptance gates. 
* **Observability** has a single metrics registrar and readiness gate, so `/metrics`, `/healthz`, `/readyz` are consistent and CI-verifiable. 
* **Security & PQ** are first-class modules (caps, TLS, PQ), enforcing capability boundaries and PQ downgrade policy without leaking into business logic. 
* **Tests** mirror the IDB: invariants, idempotency, golden vectors, and readiness behavior; **benches** target the hot pure loop for perf regressions. 

