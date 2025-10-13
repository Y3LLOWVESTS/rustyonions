
---

# File tree (proposed)

```
crates/svc-mailbox/
├─ Cargo.toml
├─ README.md
├─ LICENSE-APACHE
├─ LICENSE-MIT
├─ rust-toolchain.toml
├─ benches/
│  └─ enqueue_dequeue.rs
├─ build.rs
├─ configs/
│  └─ svc-mailbox.toml
├─ docs/
│  ├─ IDB.md
│  ├─ CONFIG.md
│  ├─ CONCURRENCY.md
│  ├─ INTEROP.md
│  ├─ OBSERVABILITY.md
│  ├─ PERFORMANCE.md
│  ├─ SECURITY.md
│  ├─ QUANTUM.md
│  ├─ RUNBOOK.md
│  ├─ diagrams/
│  │  ├─ arch.mmd
│  │  ├─ sequence.mmd
│  │  └─ state.mmd
│  └─ vectors/
│     ├─ http_send.json
│     ├─ http_recv.json
│     └─ http_ack.json
├─ examples/
│  └─ client.rs
├─ fuzz/
│  ├─ fuzz_targets/
│  │  ├─ oap_frame_parser.rs
│  │  └─ envelope_deser.rs
│  └─ Cargo.toml
├─ scripts/
│  ├─ dev_run.sh
│  ├─ render_mermaid.sh
│  └─ check_golden_metrics.sh
├─ src/
│  ├─ main.rs
│  ├─ lib.rs
│  ├─ config.rs
│  ├─ error.rs
│  ├─ metrics.rs
│  ├─ observability/
│  │  ├─ health.rs
│  │  └─ tracing.rs
│  ├─ auth/
│  │  ├─ macaroon.rs
│  │  └─ caps_cache.rs
│  ├─ http/
│  │  ├─ mod.rs
│  │  ├─ routes.rs
│  │  ├─ dto.rs
│  │  └─ middleware.rs
│  ├─ grpc/              # stubbed until enabled behind a feature
│  │  ├─ mod.rs
│  │  └─ proto.rs
│  ├─ domain/
│  │  ├─ envelope.rs
│  │  ├─ shard.rs
│  │  ├─ visibility.rs
│  │  ├─ dlq.rs
│  │  └─ idempotency.rs
│  ├─ runtime/
│  │  ├─ supervisor.rs
│  │  ├─ listener.rs
│  │  ├─ worker.rs
│  │  ├─ scanner.rs
│  │  ├─ reprocessor.rs
│  │  └─ shutdown.rs
│  ├─ bus/
│  │  └─ events.rs
│  ├─ pq/
│  │  └─ posture.rs
│  └─ util/
│     ├─ backoff.rs
│     └─ time.rs
├─ tests/
│  ├─ golden_metrics.rs
│  ├─ http_surface.rs
│  ├─ visibility.rs
│  ├─ idempotency.rs
│  └─ readiness.rs
└─ .github/workflows/
   ├─ ci.yml
   └─ render-mermaid.yml
```

---

## What each file/module is for (and why it exists)

### Top level

* **Cargo.toml** — Defines features (`tls`, `otel`, `grpc`), MSRV and minimal deps. Keeps gRPC behind a feature to avoid code bloat until needed (Interop lists HTTP+NDJSON as primary; gRPC is optional).
* **README.md** — Your finalized, role-aware README (already aligned with endpoints, metrics, SLOs). It’s the single-pane starting point.
* **rust-toolchain.toml** — Pins MSRV (1.80.0) to keep reproducibility consistent with docs and CI.
* **LICENSE-*** — Dual licensing as per project convention.
* **build.rs** — Optional: embed git SHA/version into `/metrics` and logs; used by observability dashboards (service.version).
* **configs/svc-mailbox.toml** — A runnable default config showing all keys with sane defaults and comments (env prefix `SVCMBX_` mirrors docs).

### docs/ (treated as code)

* **IDB.md** — Invariant-Driven Blueprint: at-least-once, bounded queues, visibility, idempotency, tamper-evidence. Forms the basis for tests & alerts.
* **CONFIG.md** — Canonical configuration reference and precedence (CLI > env `SVCMBX_*` > file > defaults).
* **CONCURRENCY.md** — Runtime topology (supervisor, listener, shard workers, visibility scanner, cap-cache refresher, DLQ reprocessor) and “no lock across await” rules; includes diagrams. Used to constrain implementation and code review.
* **INTEROP.md** — Protocols, endpoints, DTOs, NDJSON stream, and OAP/1 framing limits (1 MiB frames; ~64 KiB stream chunks). Source of truth for wire contracts and test vectors.
* **OBSERVABILITY.md** — Metrics names, low-cardinality label policy, health/readiness semantics, alert rules, and dashboards. Your golden metrics tests derive from here.
* **PERFORMANCE.md** — Targets & harness notes: enqueue/dequeue p95, mixed RPS, repro guidance for datasets/concurrency. Informs benches.
* **SECURITY.md** — Threat model, macaroon capabilities, TLS 1.3, integrity hashing, amnesia behavior. Mirrors anti-scope guarantees (no ambient trust).
* **QUANTUM.md** — PQ posture milestones M0–M3 and telemetry hooks (e.g., `pq_handshakes_total`). Tied to the `pq/` module and feature flags.
* **RUNBOOK.md** — Saturation, visibility leak, DLQ spike, and cap-rotation runbooks linked from alerts.
* **diagrams/** — Mermaid sources (arch/sequence/state). CI renders SVGs on PR for drift detection.
* **vectors/** — Canonical request/response examples used by doc tests and interop tests; ensures wire compatibility.

### benches/

* **enqueue_dequeue.rs** — Microbench for accept→ready and recv→first-envelope latency; aligns with p95 targets and dashboard histograms.

### fuzz/

* **fuzz_targets/** — Targets for OAP frame parsing and DTO (envelope) deserialization, as called out by your fuzz commands. Keeps parsers robust.

### scripts/

* **dev_run.sh** — One-liner to start the service with env overrides (bind/metrics) matching docs’ quickstart.
* **render_mermaid.sh** — Local diagram rendering to SVG; same as CI recipe.
* **check_golden_metrics.sh** — Greps `/metrics` output in dev to verify golden names exist (mirrors CI “no missing metrics” gate).

### src/ (small, single-purpose modules)

**Entry & surface**

* **main.rs** — Thin binary entry: load config, init tracing/metrics, build HTTP router, spawn runtime tasks, serve, and graceful shutdown. Health endpoints bind from config (metrics addr).
* **lib.rs** — Re-exports stable surface for tests/examples: `Config`, `Metrics`, `Health`, `Routes`, `Supervisor`.

**Cross-cutting**

* **config.rs** — Strongly-typed config (TOML/ENV/CLI) and defaults; enforces frame caps, timeouts, shard sizes, and visibility bounds exactly as documented (env prefix `SVCMBX_`).
* **error.rs** — Typed error taxonomy with retry hints: `AuthError::Denied`, `Busy::Saturated`, `State::Conflict`, `Integrity::BadHash`, etc., mirroring README’s table.
* **metrics.rs** — Registry for golden metrics and histograms (`queue_depth`, `saturation`, `mailbox_*`, `integrity_fail_total`, etc.), plus helpers to increment them with correct labels (topic_class, shard).
* **observability/health.rs** — `/healthz`, `/readyz`, and `/metrics` handlers. Readiness implements “shed writes first under pressure” semantics and reports saturation.
* **observability/tracing.rs** — Tracing setup (JSON logs, fields: `request_id`, `topic_class`, `shard`, `corr_id`; log level from config).

**AuthN/Z**

* **auth/macaroon.rs** — Request capability verification using macaroons; parses KID and validates scope (`send|recv|ack|nack|admin`); denies on unknown/expired. Emits `auth_failures_total{scope}`.
* **auth/caps_cache.rs** — Bounded, TTL’d verifier cache with rotation/eviction counters; task is refreshed by supervisor as per concurrency doc.

**HTTP layer**

* **http/mod.rs** — Axum router composition and middleware wiring (limits, timeouts, decompression ratio cap).
* **http/routes.rs** — Handlers for `/v1/send`, `/v1/recv`, `/v1/recv/stream`, `/ack/{msg_id}`, `/nack/{msg_id}`, `/dlq/reprocess}`; increments histograms/counters and enforces backpressure (429 with Retry-After).
* **http/dto.rs** — Strict DTOs with `deny_unknown_fields` and integrity fields like `payload_hash = b3:<hex>`; schema maps to vectors and INTEROP tables.
* **http/middleware.rs** — Request limits (max body 1 MiB), connection limits, timeouts, correlation ID injection, and capability extraction.

**(Optional) gRPC**

* **grpc/mod.rs** — Feature-gated gRPC server bootstrap to mirror HTTP surface for internal callers.
* **grpc/proto.rs** — Prost-generated or hand-rolled stubs for `ron.mailbox.v1` (OAP/1 framed), behind the `grpc` feature.

**Domain model**

* **domain/envelope.rs** — Canonical `Envelope` with `(msg_id, topic, idem_key, payload_hash, attempt)` and version header; used across IO and workers.
* **domain/shard.rs** — Per-shard, bounded queue abstraction (work queue); exposes `put/recv/ack/nack` with capacity checks and `queue_depth/saturation` metrics. Rejects on full (429) rather than buffering.
* **domain/visibility.rs** — Visibility lease management; monotonic deadlines; redelivery counters. Scanner integration unit-tested here.
* **domain/dlq.rs** — DLQ envelope format `{reason, attempt, last_error}` and operator-gated reprocess semantics.
* **domain/idempotency.rs** — `(topic, idem_key, payload_hash)` checks; returns prior `msg_id` on duplicates (no “exactly-once” claims by design).

**Runtime tasks (tokio)**

* **runtime/supervisor.rs** — Spawns listener, workers (N shards), visibility scanner, cap-cache refresher, metrics/health, optional reprocessor; owns shutdown watch and jittered restarts. Encodes “no lock across await” rules.
* **runtime/listener.rs** — Accepts HTTP requests and pushes units to per-shard `work_tx` (bounded mpsc). Enforces `try_send` → `Busy` policy.
* **runtime/worker.rs** — Per-shard dequeuer: assigns visibility deadlines, emits envelopes, handles ACK/NACK transitions. Cancel-safe loop pattern as in docs.
* **runtime/scanner.rs** — Requeues timed-out inflight messages via `requeue_tx` (bounded, with exponential retry if full) and increments visibility timeout metrics.
* **runtime/reprocessor.rs** — Operator-triggered DLQ → ready flow with policy/backoff; bounded control channel to avoid overload.
* **runtime/shutdown.rs** — Graceful stop orchestration (stop intake → drain → abort stragglers within deadline).

**Bus / events**

* **bus/events.rs** — Loss-tolerant broadcast signals (health/restart), bounded 1024 with drop-oldest semantics; increments `bus_lagged_total` when shedding.

**Post-quantum posture**

* **pq/posture.rs** — Emits posture telemetry (`peer_pq`, `pq_handshakes_total`) and gates readiness when `PQ_ONLY=on`, matching the milestone table. Connects to TLS settings if feature enabled.

**Utilities**

* **util/backoff.rs** — Jittered backoff helpers for scanner/reprocessor and supervisor restarts.
* **util/time.rs** — Duration/size parsing, monotonic clock wrappers, request deadline helpers.

### tests/ (acceptance gates as code)

* **golden_metrics.rs** — Asserts all golden metric names are registered; no duplicates; helps enforce the observability contract in CI.
* **http_surface.rs** — Doc-vector powered tests for `/send`, `/recv`, `/ack`, streaming `/recv/stream`; checks status codes, schema, and idempotent duplicate behavior.
* **visibility.rs** — Property tests for lease expiry/requeue and monotonic deadlines.
* **idempotency.rs** — Ensures `(topic, idem_key, payload_hash)` yields duplicate detection.
* **readiness.rs** — Verifies that readiness flips and *sheds writes first* under saturation.

### .github/workflows/

* **ci.yml** — fmt, clippy (deny warnings), tests, fuzz (time-boxed), deny, benches (optional smoke), docs freshness check (90-day reminder), and golden metrics enforcement.
* **render-mermaid.yml** — Renders all `*.mmd` → SVG in `docs/diagrams/` on PRs to catch drift.

---

## Why this structure works for RustyOnions

* **Maps exactly to runtime topology** (listener → shard workers → scanner, with supervisor & health), so modules stay small and names match the diagrams and docs.
* **Backpressure over buffering** is encoded where it belongs (listener and shard queue), keeping invariants local and testable.
* **Observability is first-class**: one module for metric names & registration, one for health/readiness logic, plus golden-metrics tests and alert/runbook linkage.
* **Config precedence is explicit** and centralized, mirroring the env prefix and schema table so operators don’t hunt across files.
* **Anti-scope is enforced by design**: no storage/DHT creep, no “exactly-once” claims, no global ordering; the modules reflect those hard boundaries.

