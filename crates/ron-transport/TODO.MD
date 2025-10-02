

```
crates/ron-transport/
├─ Cargo.toml
├─ README.md
├─ ALL_DOCS.md
├─ .cargo/
│  └─ config.toml
├─ .github/workflows/
│  ├─ ci.yml
│  ├─ perf.yml
│  └─ tla.yml
├─ scripts/
│  ├─ ci/
│  │  ├─ run_tlc.sh
│  │  └─ env_sanitize.sh
│  └─ local/
│     └─ perf_repro.sh
├─ specs/
│  ├─ transport_handshake.tla
│  ├─ transport_handshake.cfg
│  └─ README.md
├─ benches/
│  ├─ bench_throughput.rs
│  └─ bench_latency.rs
├─ examples/
│  ├─ bench_echo.rs
│  ├─ tls_echo.rs
│  ├─ onion_echo.rs
│  └─ quic_echo.rs
├─ tests/
│  ├─ vectors/
│  │  ├─ oap_hello.json
│  │  ├─ comp_bounds.json
│  │  ├─ tor_parity.json
│  │  └─ pq_hybrid_hello.json
│  ├─ integration/
│  │  ├─ tls_handshake_limits.rs
│  │  ├─ arti_bootstrap_ready.rs
│  │  └─ quic_parity.rs
│  ├─ amnesia/
│  │  └─ no_disk_touches.rs
│  ├─ loom/
│  │  └─ single_writer.rs
│  └─ soak/
│     └─ loopback_1MiB.rs
├─ fuzz/
│  └─ fuzz_targets/
│     └─ frame_boundaries.rs
└─ src/
   ├─ lib.rs
   ├─ config.rs
   ├─ limits.rs
   ├─ error.rs
   ├─ reason.rs
   ├─ readiness.rs
   ├─ metrics.rs
   ├─ types.rs
   ├─ util/
   │  ├─ cancel.rs
   │  ├─ timeouts.rs
   │  └─ bytes.rs
   ├─ conn/
   │  ├─ mod.rs
   │  ├─ backpressure.rs
   │  ├─ reader.rs
   │  ├─ writer.rs
   │  └─ rate_limit.rs
   ├─ tcp/
   │  ├─ mod.rs
   │  ├─ listener.rs
   │  └─ dialer.rs
   ├─ tls/
   │  ├─ mod.rs
   │  ├─ client.rs
   │  └─ server.rs
   ├─ arti/              # feature = "arti"
   │  ├─ mod.rs
   │  ├─ client.rs
   │  ├─ service.rs
   │  └─ readiness.rs
   └─ quic/              # feature = "quic"
      ├─ mod.rs
      ├─ client.rs
      └─ server.rs
```

### File-by-file roles (2–3 sentences each)

**Cargo.toml** — Declares the crate as a pure library and defines feature gates (`arti`, `quic`). Pins workspace dependencies and exposes no binaries to keep the surface minimal and composable.

**README.md** — Human-facing quickstart and invariants summary (OAP/1 ceilings, single-writer, TLS discipline). Mirrors examples and links to acceptance gates so ops/devs can reproduce behavior.

**ALL_DOCS.md** — The crate’s combined canon (IDB, SECURITY, QUANTUM, INTEROP, PERFORMANCE, TESTS, RUNBOOK, etc.). Acts as the single source of truth for reviewers and CI policy references.

**.cargo/config.toml** — Local build/test toggles (e.g., enabling loom cfg or quick profiling flags). Keeps developer ergonomics out of Cargo.toml to avoid surprise in CI.

**.github/workflows/ci.yml** — CI matrix across feature sets (`default`, `arti`, `quic`, `arti+quic`). Runs build, tests, clippy-deny, doc build, and specialized jobs (loom/TSan) as needed.

**.github/workflows/perf.yml** — On-demand Criterion benches to capture performance artifacts. Provides stable, comparable outputs without tying SLOs to noisy CI hardware.

**.github/workflows/tla.yml** — Runs lightweight model checks for the TLA+ specs. Prevents specification drift and keeps handshake/readiness invariants executable.

**scripts/ci/run_tlc.sh** — CI entrypoint to run TLC/Apalache with safe time/heap bounds. Fails fast with actionable logs when invariants are violated.

**scripts/ci/env_sanitize.sh** — Ensures env is amnesia-safe for tests (e.g., `AMNESIA=1`, temp dirs). Guards against accidental disk writes in CI.

**scripts/local/perf_repro.sh** — A small, deterministic wrapper to reproduce bench runs locally (fixed warmups/iterations). Helps devs compare apples-to-apples with CI artifacts.

**specs/transport_handshake.tla** — Formal sketch of dial→HELLO→{Ready, NotReady, Timeout} with invariants for canonical reasons and readiness transitions. It’s intentionally tiny to minimize maintenance cost.

**specs/transport_handshake.cfg** — TLC configuration (bounds, invariants enabled). Keeps formal runs reproducible in CI and locally.

**specs/README.md** — One-page explainer for running and interpreting the spec. Sets expectations and scope so we don’t over-invest in formalism.

**benches/bench_throughput.rs** — Criterion harness for sustained throughput using 1 MiB frames and ~64 KiB streaming chunks. Tracks p95/p99 and records artifacts for regression checks.

**benches/bench_latency.rs** — Measures connect→first-byte latency per backend (tcp/tls/arti/quic). Ensures parity across backends and detects jitter regressions early.

**examples/bench_echo.rs** — Minimal end-to-end echo used by README to reproduce perf. Doubles as a sanity harness for size/bounds behavior.

**examples/tls_echo.rs** — Demonstrates TLS 1.3 over TCP using caller-supplied `tokio_rustls::rustls::{ClientConfig,ServerConfig}`. Validates handshake labeling and error-reason mapping.

**examples/onion_echo.rs** — Tor v3 (Arti) echo demonstrating client and onion service modes. Exercises bootstrap readiness transitions and parity with TLS.

**examples/quic_echo.rs** — QUIC echo showing handshake parity and size ceilings identical to TCP/TLS. Kept behind `feature = "quic"` to avoid leakage into default builds.

**tests/vectors/oap_hello.json** — Canonical OAP HELLO vector used by interop tests. Ensures identical behavior across backends without embedding protocol parsing here.

**tests/vectors/comp_bounds.json** — Compression/size-cap vectors to prevent decompression bombs and oversize frames. Serves as ground truth for fuzz and integration tests.

**tests/vectors/tor_parity.json** — Expected Tor/Arti handshake and close-reason parity with TCP/TLS. Shields us from backend-specific drift.

**tests/vectors/pq_hybrid_hello.json** — PQ-hybrid KEX label expectations (e.g., `hybrid_x25519_mlkem768`). Enables safe rollout without label churn.

**tests/integration/tls_handshake_limits.rs** — Verifies TLS timeouts/alerts map to canonical reason strings and histogram buckets. Acts as the main guardrail for observability compatibility.

**tests/integration/arti_bootstrap_ready.rs** — Asserts Arti bootstrap readiness (NotReady→Ready, Retry-After semantics). Catches mis-wired readiness early.

**tests/integration/quic_parity.rs** — Ensures QUIC maintains vector parity and reason canon. Prevents divergent behavior across network stacks.

**tests/amnesia/no_disk_touches.rs** — Proves zero filesystem writes when `AMNESIA=1`. Prevents accidental persistence creeping into the transport layer.

**tests/loom/single_writer.rs** — Model checks the single-writer invariant under concurrency interleavings. Guarantees no cross-await locking and no multi-writer hazards.

**tests/soak/loopback_1MiB.rs** — Long-running soak with 1 MiB frames and capped decompression. Detects FD leaks, stalls, and tail-latency drift.

**fuzz/fuzz_targets/frame_boundaries.rs** — AFL/libFuzzer-style target for frame length, compression caps, and reserved-flag handling. Focused on rejection paths and robustness.

**src/lib.rs** — Public entrypoint with re-exports of safe, minimal types/traits. Does not re-export TLS types; callers pass rustls configs explicitly.

**src/config.rs** — `TransportConfig` DTO (addr/name/max_conns/read/write/idle) with serde-friendly shape. Host services handle any hot-swap; this crate stays lib-pure.

**src/limits.rs** — Centralized ceilings: `MAX_FRAME_BYTES = 1_048_576` and `STREAM_CHUNK_BYTES ≈ 65_536`, plus decompression bounds. Single source of truth for tests and backends.

**src/error.rs** — Internal error taxonomy and mapping to canonical reason strings. Keeps observability stable even as internal errors evolve.

**src/reason.rs** — Append-only list of reason strings used by logs/metrics/tests. Prevents label drift and keeps dashboards durable.

**src/readiness.rs** — Tri-state readiness (`NotReady|Degraded|Ready`) and minimal keys/knobs. Consumed by hosts to expose `/readyz` and by Arti readiness glue.

**src/metrics.rs** — Prometheus registrations with low-cardinality labels (backend, reason). Defines golden histograms/counters used in SLO gating.

**src/types.rs** — Public handles and DTOs (e.g., `Listener`, `Transport`, `ConnStats`, dial/listen options). Designed to be stable and small to avoid semver churn.

**src/util/cancel.rs** — Cooperative cancellation helpers for graceful shutdown. Ensures no blocking drops or zombie tasks.

**src/util/timeouts.rs** — Normalized connect/read/write/idle deadline helpers. Keeps timeout logic consistent across TCP/TLS/Arti/QUIC.

**src/util/bytes.rs** — Owned byte utilities built around `bytes::Bytes`. Preserves zero-copy behavior and reduces needless allocations.

**src/conn/mod.rs** — Connection assembly and task orchestration. Spawns exactly one reader and one writer and publishes error/reason events.

**src/conn/backpressure.rs** — Bounded writer queue with drop counters and queue-depth gauge. Primary defense against unbounded memory growth.

**src/conn/reader.rs** — The single reader task: enforces size/timeout limits and never locks across `.await`. Feeds upper layers with safe, bounded chunks.

**src/conn/writer.rs** — The single writer task: enforces caps and flush discipline. Responsible for applying rate limits and mapping errors to canon reasons.

**src/conn/rate_limit.rs** — Optional token-bucket hooks for ingress/egress shaping (default off). Integrates with metrics to expose effective rates.

**src/tcp/mod.rs** — TCP glue shared by dialer and listener. Keeps platform nuances isolated from core logic.

**src/tcp/listener.rs** — Async accept loop with per-connection budgets. Converts I/O faults into canonical reasons and updates readiness/metrics.

**src/tcp/dialer.rs** — Connect + retry/backoff with deadline discipline. Emits connect/handshake latency metrics for SLO tracking.

**src/tls/mod.rs** — rustls glue that accepts caller-provided configs but never re-exports them. Centralizes any handshake helpers.

**src/tls/client.rs** — TLS 1.3 dial path; records handshake latency and maps alerts to reasons. Accepts `tokio_rustls::rustls::ClientConfig`.

**src/tls/server.rs** — TLS 1.3 accept path mirroring the client’s observability model. Accepts `tokio_rustls::rustls::ServerConfig`.

**src/arti/mod.rs** *(feature = "arti")* — Feature gate and shared types/labels for the Arti backend. Ensures the rest of the crate compiles cleanly without Arti.

**src/arti/client.rs** — Outbound connections via Arti with HELLO parity against TLS/QUIC. Surfaces bootstrap state and reason canon on failures.

**src/arti/service.rs** — Onion service publish/listen behavior and lifecycle. Hooks into readiness to expose NotReady→Ready transitions.

**src/arti/readiness.rs** — Arti-specific readiness keys (bootstrap/publish progress). Consumed by `src/readiness.rs` to produce stable readiness.

**src/quic/mod.rs** *(feature = "quic")* — Feature gate and shared QUIC labels/limits. Keeps QUIC isolated so default builds stay lean.

**src/quic/client.rs** — QUIC dial path mirroring TLS observability and limits. Validates parity via the common vectors.

**src/quic/server.rs** — QUIC accept path with the same ceilings/readiness/error canon. Ensures backend symmetry for operators and dashboards.

---

