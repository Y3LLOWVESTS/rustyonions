
---

# Proposed file tree (scaffold only)

```
crates/
└─ svc-registry/
   ├─ Cargo.toml
   ├─ README.md
   ├─ LICENSE-APACHE
   ├─ LICENSE-MIT
   ├─ .gitignore
   ├─ .env.example
   ├─ Config.toml.sample
   ├─ build.rs
   ├─ src/
   │  ├─ main.rs
   │  ├─ lib.rs
   │  ├─ build_info.rs
   │  ├─ error.rs
   │  ├─ result.rs
   │  ├─ shutdown.rs
   │  ├─ config/
   │  │  ├─ mod.rs
   │  │  ├─ model.rs
   │  │  ├─ load.rs
   │  │  ├─ validate.rs
   │  │  └─ reload.rs
   │  ├─ observability/
   │  │  ├─ mod.rs
   │  │  ├─ metrics.rs
   │  │  ├─ tracing.rs
   │  │  ├─ logging.rs
   │  │  └─ endpoints.rs
   │  ├─ http/
   │  │  ├─ mod.rs
   │  │  ├─ routes.rs
   │  │  ├─ sse.rs
   │  │  ├─ middleware/
   │  │  │  ├─ limits.rs
   │  │  │  ├─ timeouts.rs
   │  │  │  ├─ corr_id.rs
   │  │  │  └─ auth.rs
   │  │  └─ responses.rs
   │  ├─ auth/
   │  │  ├─ macaroon.rs
   │  │  └─ uds.rs
   │  ├─ interop/
   │  │  ├─ dto.rs
   │  │  ├─ event_shapes.rs
   │  │  └─ openapi_stub.rs
   │  ├─ governance/
   │  │  ├─ mod.rs
   │  │  ├─ quorum.rs
   │  │  ├─ approvals.rs
   │  │  ├─ supersede.rs
   │  │  └─ signer_set.rs
   │  ├─ pipeline/
   │  │  ├─ mod.rs
   │  │  ├─ propose.rs
   │  │  ├─ approve.rs
   │  │  ├─ commit.rs
   │  │  ├─ bus_publish.rs
   │  │  ├─ checkpoint.rs
   │  │  ├─ retention.rs
   │  │  └─ deep_verify.rs
   │  ├─ storage/
   │  │  ├─ mod.rs
   │  │  ├─ types.rs
   │  │  ├─ head.rs
   │  │  ├─ log.rs
   │  │  ├─ checkpoint.rs
   │  │  ├─ sled_store.rs
   │  │  └─ sqlite_store.rs
   │  ├─ bus/
   │  │  ├─ mod.rs
   │  │  └─ events.rs
   │  ├─ readiness/
   │  │  └─ gate.rs
   │  └─ pq/
   │     ├─ mod.rs
   │     ├─ verify_dilithium.rs
   │     ├─ verify_falcon.rs
   │     └─ policy.rs
   ├─ benches/
   │  ├─ blake3_payload.rs
   │  └─ verify_approvals.rs
   ├─ tests/
   │  ├─ http_contract.rs
   │  ├─ invariants.rs
   │  ├─ concurrency_loom.rs
   │  ├─ chaos_ready.rs
   │  └─ vectors/
   │     ├─ proposal.json
   │     ├─ approvals_ed25519.json
   │     ├─ approvals_pq_mixed.json
   │     └─ descriptor_set_v1.json
   ├─ fuzz/               (cargo-fuzz)
   │  ├─ fuzz_targets/
   │  │  ├─ dto_decode.rs
   │  │  └─ approval_payload.rs
   │  └─ corpus/
   ├─ docs/
   │  ├─ arch.mmd
   │  ├─ sequence.mmd
   │  ├─ state.mmd
   │  ├─ openapi/
   │  │  └─ registry.yaml
   │  └─ api-history/
   │     └─ svc-registry-1.0.0.txt
   ├─ scripts/
   │  ├─ run_local.sh
   │  ├─ render_mermaid.sh
   │  └─ perf_sweep.sh
   └─ .github/
      └─ workflows/
         ├─ ci.yml
         ├─ render-mermaid.yml
         └─ perf.yml
```

---

## What each file/dir is for (and why it exists)

### Workspace & top level

* **Cargo.toml** — Crate manifest (features: `tls`, `pq`, `pq-hybrid`, `pq-verify`, `kameo`, `cli`). Keeps service Rust surface tiny per API.md; HTTP is the canonical interface. 
* **README.md** — Your role-aware, diagrams-rich ops doc (keep arch/sequence/state diagrams in sync with `/docs/*.mmd`). 
* **Config.toml.sample / .env.example** — Sane defaults mirroring the config schema (SVCR_* envs, limits, TLS, UDS, retention, bus) so devs can copy-paste quickly. 
* **build.rs** — Embeds build metadata (name, version, commit, build_time) for `GET /version`. 

### `src/` (small, focused modules)

* **main.rs** — Binary entrypoint: init config → observability → spawn runtime topology (listener, commit pipeline, publisher, checkpointor, pruner, deep verifier) → block on shutdown. No business logic. 
* **lib.rs** — Re-exports minimal surfaced types for tests (`Config`, `Metrics`, `Error`, `BuildInfo`) while keeping internals private, per your “service-first HTTP, tiny Rust surface” rule. 
* **build_info.rs** — Type holding the exposed version/commit/schema info surfaced at `/version`. 
* **error.rs** — Canonical domain error enum mapping to the wire envelope (`code`, `message`, `corr_id`, `details`) so HTTP and logs use the same taxonomy. 
* **result.rs** — Local `type Result<T> = std::result::Result<T, crate::Error>` for consistency.
* **shutdown.rs** — Shared cooperative shutdown channel and helpers (watch/propagate/drain/abort), aligning with your graceful drain semantics. 

#### `src/config/`

* **model.rs** — Strongly typed config struct with nested sections (timeouts, limits, tls, uds, storage, retention, otel, amnesia, pq, bus, deprecation). Mirrors the documented keys and defaults. 
* **load.rs** — Merge layers (file/env/flags), precedence, and “atomic snapshot then apply” logic used by reload; no secrets logged. 
* **validate.rs** — Fail-closed invariants (e.g., non-zero limits, tmpfs rule for amnesia, TLS key existence, retention thresholds). 
* **reload.rs** — SIGHUP/bus-driven dynamic reload, applies non-disruptive keys hot and drains/rebinds on disruptive ones; increments `registry_reload_*` metrics. 
* **mod.rs** — Public `Config` interface and helpers (lint warnings surfaced in logs/metrics). 

#### `src/observability/`

* **metrics.rs** — Single registration point for all golden + registry-specific metrics; provides cheap clone handles. Exposed at `/metrics`. 
* **tracing.rs** — OpenTelemetry initialization, span naming (`svc.registry.{propose,approve,commit,...}`), correlation propagation (`X-Corr-ID`). 
* **logging.rs** — JSON logs, redaction, and required fields (ts, level, service, event, corr_id, route, err.code, version, payload_b3). 
* **endpoints.rs** — `/healthz` (liveness) and `/readyz` (write/read readiness with structured JSON, `Retry-After`), plus the `/version` surface. 

#### `src/http/`

* **routes.rs** — Axum router wiring of documented endpoints:
  `GET /version`, `/healthz`, `/readyz`, `/registry/head`, `/registry/{version}`, `/registry/stream`, `POST /registry/proposals`, `POST /registry/approvals/{id}`, `POST /registry/commit/{id}`, and rare `POST /registry/supersede`. 
* **sse.rs** — SSE stream for lossy updates; integrates with broadcast channel and forces clients to reconcile on gaps. 
* **middleware/**

  * **limits.rs** — Body cap, decompression ratio cap, and reject-don’t-buffer behavior that increments `rejected_total{reason}`. 
  * **timeouts.rs** — Read/write/idle timeouts as Tower layers. 
  * **corr_id.rs** — Extract/set `X-Corr-ID` and echo to responses and bus events. 
  * **auth.rs** — Capability (macaroon) extraction and scope checks for write paths. 
* **responses.rs** — Shared JSON responders for success/error with the typed error envelope (`code`, `message`, `corr_id`, `details`). 

#### `src/auth/`

* **macaroon.rs** — Verifier for capability tokens and scopes (`registry:propose|approve|commit`), short-lived, attenuable. 
* **uds.rs** — `SO_PEERCRED` allow-list checks when bound to a Unix socket. 

#### `src/interop/`

* **dto.rs** — Glue around `ron-proto` DTOs (no local DTOs); guards `deny_unknown_fields` for write paths. 
* **event_shapes.rs** — Kernel bus event JSON shapes (`registry.updated`, `kernel.health`, `kernel.config.updated`). 
* **openapi_stub.rs** — Loader/sanity check that `/docs/openapi/registry.yaml` stays in sync with the router (used by contract tests). 

#### `src/governance/`

* **quorum.rs** — M-of-N quorum evaluation, CAS assumptions, expiry windows; produces deterministic errors like `QuorumFailed`. 
* **approvals.rs** — Verify approvals over `payload_b3` (canonical payload hash with fixed context string), reject duplicates/revocations. 
* **supersede.rs** — Administrative supersede mechanics (`supersede_of`, reason, CAS on HEAD) with audit emission. 
* **signer_set.rs** — Signer lifecycle (rotation/revocation) validation at commit time. 
* **mod.rs** — Role/authority boundary façade (operators can freeze writes; can’t bypass quorum). 

#### `src/pipeline/` (single writer discipline)

* **propose.rs** — Accept proposal, compute/confirm `payload_b3`, enqueue to commit pipeline (bounded mpsc; `Busy` on full). 
* **approve.rs** — Validate signer and add approval to the pending proposal state. 
* **commit.rs** — The **only** task allowed to advance `HEAD` (CAS), append `SignedArtifact` to log, and broadcast `committed`. Enforces append-only and monotonic versions. 
* **bus_publish.rs** — Non-blocking publish of update events; counts drops (`bus_lagged_total`). 
* **checkpoint.rs** — Periodic signed checkpoints to enable safe pruning; exported freshness metric. 
* **retention.rs** — Policy-driven prune (by age/count/size) only for segments fully covered by a checkpoint. 
* **deep_verify.rs** — Background integrity verification (time-sliced) of the audit chain; never blocks hot paths. 
* **mod.rs** — Wires bounded channels, capacities, and backpressure policy (reject on request paths, drop-to-latest on broadcast). 

#### `src/storage/`

* **types.rs** — Storage traits (`RegistryStore`) covering `put_proposal`, `append_approval`, `commit_signed_artifact`, `get_head`, `get_version`, `stream_from`. 
* **head.rs** — HEAD snapshot/get/set with compare-and-set semantics for monotonic versioning. 
* **log.rs** — Append-only signed artifact log; fsync policy on critical sections. 
* **checkpoint.rs** — Durable checkpoint records and retention guards. 
* **sled_store.rs / sqlite_store.rs** — Backend adapters (feature-gated), both crash-safe and append-only. 
* **mod.rs** — Backend selection + small glue.

#### `src/bus/`

* **events.rs** — Event constructors for `registry.updated` and health/config heartbeats; includes correlation ID. 
* **mod.rs** — Bounded broadcast handles and subscribe/publish helpers. 

#### `src/readiness/`

* **gate.rs** — Aggregates subsystem health (storage, bus, commit, checkpointor, config) for `/readyz`’s nuanced JSON. 

#### `src/pq/` (feature-gated)

* **verify_dilithium.rs / verify_falcon.rs** — PQ signature verification adapters behind `pq-verify`, labelled metrics for verify counts/latency. 
* **policy.rs** — Mixed quorum evaluation (`k_pq` slice) and readiness guard if `pq_only=true` but capacity is insufficient. 
* **mod.rs** — Facade toggling PQ posture per config/features. 

### Benches / Tests / Fuzz / Docs / Scripts

* **benches/** — Criterion micro-benches for BLAKE3 payload hashing and approval verify; feed perf gates. 
* **tests/http_contract.rs** — Spins the service and asserts endpoint existence, headers, status codes, error envelope, and `/version` deprecations table. (CI Gate B). 
* **tests/invariants.rs** — Property tests: no commit without quorum; monotonic HEAD; idempotent commit path; strict DTO hygiene on writes. 
* **tests/concurrency_loom.rs** — Loom model: single writer; no lock across `.await`; shutdown is observed. 
* **tests/chaos_ready.rs** — Write-plane sheds first; `/readyz` flips correctly under storage/bus degradation. 
* **tests/vectors/** — Canonical interop vectors (proposal, approvals ed25519 & PQ, descriptor set) used across tests and SDKs. 
* **fuzz/** — `cargo-fuzz` targets for DTO decode and approval payloads; corpus lives here and grows via CI artifacts. 
* **docs/arch.mmd, sequence.mmd, state.mmd** — Mermaid sources that must exist per README; rendered in CI for drift-free diagrams. 
* **docs/openapi/registry.yaml** — Authoritative spec excerpt mirrored in API.md; kept in lockstep with router and contract tests. 
* **docs/api-history/** — Public Rust surface snapshots (expected to be minimal) kept for diffs via `cargo public-api`. (CI Gate A). 
* **scripts/run_local.sh** — Local dev runner with sensible defaults (localhost binds, metrics on loopback).
* **scripts/render_mermaid.sh** — Developer helper to render all `.mmd` to `.svg` locally; CI mirrors this. 
* **scripts/perf_sweep.sh** — Quick load sweep to reproduce SLOs and compare against stored baselines. 
* **.github/workflows/**

  * **ci.yml** — fmt/clippy/tests/deny + contract tests + coverage ≥ 90%; perf guard fail if p95 regresses >10%. 
  * **render-mermaid.yml** — Renders all `.mmd` on push/PR and publishes artifacts. 
  * **perf.yml** — Nightly perf job comparing JSON baselines; uploads flamegraphs & traces on regressions. 

---

## Why this layout hits your “God tier” gates

* **Invariants first:** The `pipeline/commit.rs` single-writer and CAS-on-HEAD layout operationalizes your append-only, quorum-gated chain; request paths reject fast (`Busy`) instead of queueing. 
* **Interop is law:** `http/routes.rs`, `interop/dto.rs`, `docs/openapi/registry.yaml`, and `tests/http_contract.rs` form the contract triangle; wire schemas are additive only. 
* **Observability baked-in:** Golden metrics + health/readiness JSON live in `observability/*` and `readiness/gate.rs`, with stable names/labels for dashboards/alerts. 
* **Security/governance:** Capability scopes and signer verification live in `auth/` + `governance/`; emergency **freeze** is policy, not a bypass. Supersede is explicit and signed. 
* **PQ-ready without churn:** `pq/` isolates adapters and policy so we can flip to hybrid/verify/mixed-quorum safely, exposing clear readiness and metrics. 
* **Small files, clear seams:** Each concern is in its own short module; hot paths avoid locks across `.await` and expose clear unit/property/loom targets. 

