

---

# omnigate — scaffold (no inline comments)

```
crates/omnigate/
├─ Cargo.toml
├─ README.md
├─ CHANGELOG.md
├─ LICENSE-APACHE
├─ LICENSE-MIT
├─ CODEOWNERS
├─ rust-toolchain.toml
├─ deny.toml
├─ .cargo/
│  └─ config.toml
├─ .gitignore
│
├─ src/
│  ├─ main.rs
│  ├─ lib.rs
│  │
│  ├─ config/
│  │  ├─ mod.rs
│  │  ├─ env.rs
│  │  ├─ file.rs
│  │  ├─ validate.rs
│  │  └─ reload.rs
│  │
│  ├─ bootstrap/
│  │  ├─ mod.rs
│  │  ├─ server.rs
│  │  ├─ metrics_server.rs
│  │  └─ health_probe.rs
│  │
│  ├─ admission/
│  │  ├─ mod.rs
│  │  ├─ quotas.rs
│  │  └─ fair_queue.rs
│  │
│  ├─ auth/
│  │  ├─ mod.rs
│  │  ├─ capability.rs
│  │  ├─ passport_client.rs
│  │  └─ revocation.rs
│  │
│  ├─ middleware/
│  │  ├─ mod.rs
│  │  ├─ body_caps.rs
│  │  ├─ decompress_guard.rs
│  │  ├─ corr_id.rs
│  │  ├─ classify.rs
│  │  └─ slow_loris.rs
│  │
│  ├─ routes/
│  │  ├─ mod.rs
│  │  ├─ ops.rs
│  │  └─ v1/
│  │     ├─ mod.rs
│  │     ├─ objects.rs
│  │     ├─ index.rs
│  │     ├─ mailbox.rs
│  │     ├─ dht.rs
│  │     └─ facet/
│  │        ├─ mod.rs
│  │        ├─ media.rs
│  │        ├─ graph.rs
│  │        └─ feed.rs
│  │
│  ├─ hydration/
│  │  ├─ mod.rs
│  │  ├─ planner.rs
│  │  └─ compose.rs
│  │
│  ├─ downstream/
│  │  ├─ mod.rs
│  │  ├─ index_client.rs
│  │  ├─ storage_client.rs
│  │  ├─ mailbox_client.rs
│  │  ├─ dht_client.rs
│  │  ├─ latency.rs
│  │  └─ hedge.rs
│  │
│  ├─ readiness/
│  │  ├─ mod.rs
│  │  ├─ keys.rs
│  │  └─ policy.rs
│  │
│  ├─ metrics/
│  │  ├─ mod.rs
│  │  └─ registry.rs
│  │
│  ├─ errors/
│  │  ├─ mod.rs
│  │  ├─ http_map.rs
│  │  └─ reasons.rs
│  │
│  ├─ observability/
│  │  ├─ mod.rs
│  │  ├─ logging.rs
│  │  └─ tracing_spans.rs
│  │
│  ├─ runtime/
│  │  ├─ mod.rs
│  │  ├─ supervisor.rs
│  │  ├─ worker.rs
│  │  ├─ channels.rs
│  │  └─ shutdown.rs
│  │
│  ├─ pq/
│  │  ├─ mod.rs
│  │  └─ negotiate.rs
│  │
│  ├─ zk/
│  │  ├─ mod.rs
│  │  ├─ receipts.rs
│  │  └─ no_mutate.rs
│  │
│  └─ types/
│     ├─ mod.rs
│     └─ dto.rs
│
├─ benches/
│  ├─ hydration.rs
│  └─ media_range.rs
│
├─ tests/
│  ├─ interop_vectors.rs
│  ├─ readyz_overload.rs
│  ├─ oap_limits.rs
│  ├─ policy_gate.rs
│  ├─ metrics_contract.rs
│  ├─ loom_fanout.rs
│  ├─ hardening.rs
│  └─ zk_read_only.rs
│
├─ fuzz/
│  ├─ fuzz_targets/
│  │  ├─ capability.rs
│  │  ├─ decompress_guard.rs
│  │  └─ headers.rs
│  └─ Cargo.toml
│
├─ docs/
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
│  ├─ mermaid/
│  │  ├─ arch.mmd
│  │  ├─ sequence.mmd
│  │  └─ states.mmd
│  └─ api-history/
│     └─ omnigate/
│        └─ <rev>.txt
│
├─ specs/
│  ├─ readiness.tla
│  ├─ admission.tla
│  └─ README.md
│
├─ configs/
│  ├─ omnigate.toml
│  ├─ staging.toml
│  └─ amnesia.toml
│
├─ testing/
│  ├─ performance/
│  │  ├─ hydrate_mix.sh
│  │  └─ baselines/
│  │     ├─ p95_hydration.json
│  │     └─ p95_range.json
│  ├─ chaos/
│  │  ├─ scenario.yml
│  │  └─ README.md
│  └─ vectors/
│     └─ omnigate/
│        ├─ range_read.json
│        ├─ error_413.json
│        └─ unauth_401.json
│
├─ scripts/
│  ├─ check_boundary.sh
│  ├─ render_mermaid.sh
│  ├─ ci_metrics_guard.sh
│  ├─ inject_faults.sh
│  ├─ soak.sh
│  └─ hnDL_sim.sh
│
└─ .github/
   └─ workflows/
      ├─ ci.yml
      ├─ render-mermaid.yml
      ├─ perf-regression.yml
      ├─ chaos.yml
      └─ public-api.yml
```

---

# File descriptions (separate; one–two sentences each)

**Cargo.toml** — Declares crate metadata, dependencies, and features (`tls`, `pq`, `zk`, `reload`, optional `otel`) consistent with workspace pins.
**README.md** — Canonical contract: scope, interfaces, SLOs, and numbered acceptance gates [G-1…G-7].
**CHANGELOG.md** — SemVer-tracked changes to public surface, behavior, and metrics; reviewers require entries for breaking diffs.
**LICENSE-APACHE / LICENSE-MIT** — Dual licensing per project policy.
**CODEOWNERS** — Enforces reviewers for sensitive surfaces (routes, errors, metrics) to reduce drift.
**rust-toolchain.toml** — Pins toolchain for reproducible CI across environments.
**deny.toml** — Cargo-deny policy (licenses, advisories, bans) to harden supply chain.
**.cargo/config.toml** — Workspace lints and warnings-as-errors; optional target configs.
**.gitignore** — Standard ignores (target/, artifacts/, flamegraphs/).

**src/main.rs** — Thin entry: read config, build the server stack, bind listeners, start tasks.
**src/lib.rs** — Minimal public surface re-exports (Config, Metrics, HealthState, wait_for_ctrl_c) to keep the service bin-oriented.

**src/config/mod.rs** — Aggregates config loaders and exposes the `Config` type.
**src/config/env.rs** — Loads `OMNIGATE_*` environment variables with strong typing.
**src/config/file.rs** — Parses TOML files and merges them with env and defaults.
**src/config/validate.rs** — Enforces invariants (OAP frame ≤1 MiB, decompress ≤10×, sane timeouts).
**src/config/reload.rs** — (feature `reload`) Watches config file, validates, and atomically swaps; emits `ConfigUpdated`.

**src/bootstrap/mod.rs** — Builder glue for listeners, routes, and middleware.
**src/bootstrap/server.rs** — Constructs the application HTTP server and route trees.
**src/bootstrap/metrics_server.rs** — Runs the `/metrics` exporter, optionally on a private bind.
**src/bootstrap/health_probe.rs** — Performs strict boot checks and warms short-TTL caches before marking ready.

**src/admission/mod.rs** — Central admission control hub.
**src/admission/quotas.rs** — Implements token-bucket quotas and maps overflow to `429` with counters.
**src/admission/fair_queue.rs** — Deficit-round-robin classes with bounded queues and inflight/queue depth gauges.

**src/auth/mod.rs** — Capability gate entry for all protected routes.
**src/auth/capability.rs** — Parses/verifies macaroons and caveats; deterministic `401/403` decisions.
**src/auth/passport_client.rs** — Fetches and caches verifier keys from `svc-passport` with short TTL.
**src/auth/revocation.rs** — Maintains a revocation set and checks presented capabilities; exposes cache hit metrics.

**src/middleware/mod.rs** — Common Tower layers for the HTTP stack.
**src/middleware/body_caps.rs** — Hard caps request bodies at 1 MiB and returns `413` early.
**src/middleware/decompress_guard.rs** — Rejects compressed payloads exceeding ratio/absolute limits.
**src/middleware/corr_id.rs** — Propagates/creates correlation IDs and annotates spans.
**src/middleware/classify.rs** — Classifies requests into fairness classes for DRR and metrics labels.
**src/middleware/slow_loris.rs** — Enforces read-budget and defends slow-loris patterns with `408/429`.

**src/routes/mod.rs** — Router registry and API versioning entry.
**src/routes/ops.rs** — Implements `/healthz`, `/readyz` (with degrade detail), and `/metrics`.
**src/routes/v1/mod.rs** — Namespace for stable v1 endpoints and request extractors.
**src/routes/v1/objects.rs** — Proxies object reads/ranges to storage with streaming (~64 KiB chunks) and ETags.
**src/routes/v1/index.rs** — Resolves named indices/manifests; honors deny-unknown-fields at edges.
**src/routes/v1/mailbox.rs** — Provides idempotent send and long-poll receive semantics for mailbox channels.
**src/routes/v1/dht.rs** — Read-only DHT lookups that propagate degraded state.
**src/routes/v1/facet/mod.rs** — Wires facet routes and attaches facet labels.
**src/routes/v1/facet/media.rs** — Media facet endpoints with a p95 range-start SLO target (<100 ms).
**src/routes/v1/facet/graph.rs** — Graph facet endpoints with bounded join budgets.
**src/routes/v1/facet/feed.rs** — Feed facet endpoints with pagination and consistency knobs.

**src/hydration/mod.rs** — Service boundary for planning and composing hydrations.
**src/hydration/planner.rs** — Builds a downstream plan under per-hop budgets without app business logic.
**src/hydration/compose.rs** — Shapes responses using zero-copy `Bytes` and tags downstream latency.

**src/downstream/mod.rs** — Trait-based clients for downstream services.
**src/downstream/index_client.rs** — Typed client for `svc-index` with semaphores and timeouts.
**src/downstream/storage_client.rs** — Typed client for `svc-storage` including range/TTFB capture.
**src/downstream/mailbox_client.rs** — Typed client for `svc-mailbox` with batched reads and idempotence aids.
**src/downstream/dht_client.rs** — Optional read-only DHT lookups.
**src/downstream/latency.rs** — Exposes histograms for `{service,phase}` latency attribution.
**src/downstream/hedge.rs** — Stub for hedged reads to reduce tail latency on cold starts (feature-gated).

**src/readiness/mod.rs** — Orchestrates readiness state reporting.
**src/readiness/keys.rs** — Computes readiness keys (listener, passport, downstream, queues, reload).
**src/readiness/policy.rs** — Encodes fail-open reads/fail-closed writes and shed-before-collapse behavior.

**src/metrics/mod.rs** — Metric handles and helpers.
**src/metrics/registry.rs** — Single registration point for golden metrics with bounded label sets.

**src/errors/mod.rs** — Unified error types for the service.
**src/errors/http_map.rs** — Maps internal errors to HTTP codes (400/401/403/413/416/429/503/504).
**src/errors/reasons.rs** — Stable machine-readable reasons for JSON envelopes and metrics.

**src/observability/mod.rs** — Logging/tracing composition.
**src/observability/logging.rs** — JSON log fields (request_id, cap_id, degraded, fanout, downstream).
**src/observability/tracing_spans.rs** — Span hierarchy and attribute conventions for attribution.

**src/runtime/mod.rs** — Runtime façade for supervised tasks.
**src/runtime/supervisor.rs** — Crash-only supervision with jittered restarts.
**src/runtime/worker.rs** — Request execution units with cooperative cancellation.
**src/runtime/channels.rs** — Bounded channels ensuring no lock is held across `.await`.
**src/runtime/shutdown.rs** — Coordinated shutdown with deadlines and drain.

**src/pq/mod.rs** — Feature-gated PQ readiness toggles and telemetry.
**src/pq/negotiate.rs** — Hooks for hybrid KEM/SIG signaling (off by default).

**src/zk/mod.rs** — Feature-gated ZK receipt plumbing boundary.
**src/zk/receipts.rs** — Stubs for attaching optional proof receipts; increments attempt counters.
**src/zk/no_mutate.rs** — Explicit guard: ZK feature must not perform economic writes; asserts read-only policy.

**src/types/mod.rs** — Narrow, public DTO gateway for ingress.
**src/types/dto.rs** — DTOs with `deny_unknown_fields`; prefer `ron-proto` elsewhere.

**benches/hydration.rs** — Criterion harness benchmarking p95 hydration latency (≤150 ms).
**benches/media_range.rs** — Criterion harness benchmarking p95 range-start latency (<100 ms).

**tests/interop_vectors.rs** — Golden request/response vectors (range, errors) for interop checks.
**tests/readyz_overload.rs** — Validates shed-before-collapse behavior and `/readyz` flipping.
**tests/oap_limits.rs** — Proves frame size and decompression ratio caps.
**tests/policy_gate.rs** — Ensures capability verification precedes any fan-out.
**tests/metrics_contract.rs** — Scrapes `/metrics` and asserts required names/labels exist.
**tests/loom_fanout.rs** — Loom interleavings for fan-out and cancellation (no lock across `.await`).
**tests/hardening.rs** — Exercises slow-loris and compression-bomb defenses.
**tests/zk_read_only.rs** — Asserts ZK paths are strictly read-only with no econ mutations.

**fuzz/fuzz_targets/capability.rs** — Fuzzes capability parsing/validation against hostile inputs.
**fuzz/fuzz_targets/decompress_guard.rs** — Fuzzes ratio/size guards to defuse compression bombs.
**fuzz/fuzz_targets/headers.rs** — Fuzzes header parsing and envelope safety.
**fuzz/Cargo.toml** — Fuzz harness crate configuration.

**docs/API.md** — Source-of-truth for HTTP endpoints and envelopes.
**docs/CONFIG.md** — Configuration surface, defaults, precedence, and reload semantics.
**docs/CONCURRENCY.md** — Concurrency model, rules, and validation scope.
**docs/GOVERNANCE.md** — SemVer and review process guidelines.
**docs/IDB.md** — Invariant-Driven Blueprint for omnigate.
**docs/INTEROP.md** — DTOs, schemas, wire formats, and vectors.
**docs/OBSERVABILITY.md** — Metrics, logs, tracing, and alert rules.
**docs/PERFORMANCE.md** — Perf targets, rigs, and reproducibility notes.
**docs/QUANTUM.md** — PQ readiness plan, exposure, and rollout.
**docs/RUNBOOK.md** — Operational runbook for incidents and maintenance.
**docs/SECURITY.md** — Capability policy, amnesia, inputs, and custody notes.
**docs/TESTS.md** — Test suite taxonomy and CI gates.
**docs/mermaid/arch.mmd** — Architecture diagram source.
**docs/mermaid/sequence.mmd** — Client→gateway→omnigate sequence diagram source.
**docs/mermaid/states.mmd** — Ready→shed→recover lifecycle state diagram source.
**docs/api-history/omnigate/<rev>.txt** — Public surface delta log per release.

**specs/readiness.tla** — TLA+ liveness spec for readiness shedding/recovery.
**specs/admission.tla** — TLA+ fairness spec for DRR admission classes.
**specs/README.md** — How to run TLC locally; notes for quick CI pass.

**configs/omnigate.toml** — Dev defaults for sizes/timeouts and addresses.
**configs/staging.toml** — Stricter staging profile for pre-prod testing.
**configs/amnesia.toml** — Amnesia-by-default profile toggles for Micronode-style operation.

**testing/performance/hydrate_mix.sh** — Load generator that asserts p95 hydration targets.
**testing/performance/baselines/p95_hydration.json** — Golden hydration latency numbers for CI comparison.
**testing/performance/baselines/p95_range.json** — Golden range-start latency numbers for CI comparison.
**testing/chaos/scenario.yml** — Fault-injection plan (latency/5xx) for weekly soak.
**testing/chaos/README.md** — Instructions and expectations for chaos/soak runs.
**testing/vectors/omnigate/range_read.json** — Canonical range request/response vector.
**testing/vectors/omnigate/error_413.json** — Canonical 413 error vector for oversized bodies.
**testing/vectors/omnigate/unauth_401.json** — Canonical 401 error vector for missing/invalid capabilities.

**scripts/check_boundary.sh** — Fails builds on forbidden deps or patterns (e.g., locks across `.await`).
**scripts/render_mermaid.sh** — Batch renders Mermaid sources to SVG artifacts.
**scripts/ci_metrics_guard.sh** — Asserts metric names/labels haven’t drifted.
**scripts/inject_faults.sh** — Local toggles for latency/error injection to mimic chaos runs.
**scripts/soak.sh** — 24-hour soak runner with periodic snapshots.
**scripts/hnDL_sim.sh** — Harvest-now-decrypt-later drill harness for PQ tabletop exercises.

**.github/workflows/ci.yml** — fmt/clippy/tests/deny/fuzz-smoke/loom/public-api, plus minimal TLC run and an aarch64 test lane.
**.github/workflows/render-mermaid.yml** — Renders Mermaid diagrams on push/PR.
**.github/workflows/perf-regression.yml** — Benchmarks vs baselines, enforces ±10% budget, uploads flamegraphs.
**.github/workflows/chaos.yml** — Weekly soak + chaos scenario execution with trend artifacts.
**.github/workflows/public-api.yml** — Guards public API surface; requires CHANGELOG on diffs.

---
