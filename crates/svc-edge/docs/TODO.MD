
---

# `svc-edge` — 

```
crates/
└─ svc-edge/
   ├─ Cargo.toml
   ├─ README.md
   ├─ CHANGELOG.md
   ├─ LICENSE-APACHE
   ├─ LICENSE-MIT
   ├─ .gitignore
   ├─ .cargo/
   │  └─ config.toml
   ├─ configs/
   │  └─ svc-edge.toml
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
   │  ├─ openapi/
   │  │  └─ svc-edge.yaml
   │  ├─ _matrix.md
   │  ├─ mmd/
   │  │  ├─ arch.mmd
   │  │  ├─ sequence.mmd
   │  │  └─ state.mmd
   │  └─ svg/
   │     ├─ arch.svg
   │     ├─ sequence.svg
   │     └─ state.svg
   ├─ src/
   │  ├─ lib.rs
   │  ├─ bin/
   │  │  └─ svc-edge.rs
   │  ├─ cli.rs
   │  ├─ config.rs
   │  ├─ errors.rs
   │  ├─ metrics.rs
   │  ├─ state.rs
   │  ├─ readiness.rs
   │  ├─ supervisor.rs
   │  ├─ admission/
   │  │  ├─ body_cap.rs
   │  │  ├─ decompress_guard.rs
   │  │  ├─ inflight_cap.rs
   │  │  ├─ rps_limit.rs
   │  │  └─ timeout.rs
   │  ├─ routes/
   │  │  ├─ mod.rs
   │  │  ├─ assets.rs
   │  │  ├─ health.rs
   │  │  ├─ ready.rs
   │  │  └─ prometheus.rs
   │  ├─ adapters/
   │  │  ├─ pack.rs
   │  │  ├─ cas.rs
   │  │  ├─ live_fill.rs
   │  │  └─ tls.rs
   │  ├─ http/
   │  │  ├─ headers.rs
   │  │  ├─ range.rs
   │  │  └─ etag.rs
   │  ├─ work/
   │  │  ├─ queue.rs
   │  │  ├─ worker.rs
   │  │  └─ shutdown.rs
   │  ├─ security/
   │  │  ├─ cors.rs
   │  │  ├─ hsts.rs
   │  │  └─ audit.rs
   │  └─ util/
   │     ├─ bytes.rs
   │     ├─ size_parse.rs
   │     └─ backoff.rs
   ├─ tests/
   │  ├─ i_1_hardening_ingress.rs
   │  ├─ i_4_http_semantics.rs
   │  ├─ i_5_content_address.rs
   │  ├─ i_6_amnesia.rs
   │  ├─ i_8_size_bounds.rs
   │  ├─ i_9_observability_contract.rs
   │  ├─ i_10_deterministic_failures.rs
   │  ├─ i_11_pack_integrity.rs
   │  ├─ http_contract.rs
   │  ├─ readiness_logic.rs
   │  ├─ concurrency_backpressure.rs
   │  ├─ fuzz_headers.rs
   │  └─ fixtures/
   │     ├─ packs/
   │     │  └─ sample.pmtiles
   │     └─ http/
   │        ├─ warm_hit.response
   │        └─ range_206.response
   ├─ benches/
   │  ├─ bench_range.rs
   │  ├─ bench_blake3.rs
   │  └─ bench_pack_read.rs
   ├─ scripts/
   │  ├─ dev-run.sh
   │  ├─ render-mermaid.sh
   │  └─ perf-smoke.sh
   └─ .github/
      └─ workflows/
         ├─ ci.yaml
         ├─ render-mermaid.yaml
         ├─ perf-guard.yaml
         └─ public-api.yaml
```

---

## What each file/dir is for (one-liners, grouped by area)

### Top-level

* **Cargo.toml** — Crate metadata, features (`cli`, `tls`, `pq`…), dependency pins, bench/dev-deps.
* **README.md** — Your role-aware readme; links to docs and diagrams; curl examples; SLO table.
* **CHANGELOG.md** — SemVer changes; HTTP/CLI contract diffs and migration notes.
* **LICENSE-APACHE / LICENSE-MIT** — Dual-license texts.
* **.cargo/config.toml** — Local dev flags (e.g., `-D warnings`) and target configs.
* **configs/svc-edge.toml** — A runnable config example (offline vs live; caps; allow-list).

### Docs (kept as sources + rendered)

* **docs/*.MD** — The crate’s canonical docs (your combined ALL_DOCS broken out per domain) so PRs can diff each policy/contract.
* **docs/openapi/svc-edge.yaml** — Concise OpenAPI for `/edge/assets`, health/ready, metrics.
* **docs/mmd/*.mmd** → **docs/svg/*.svg** — Mermaid sources + rendered SVG (CI renders automatically).
* **docs/_matrix.md** — IDB invariant → test file traceability matrix.

### Binary + minimal library surface

* **src/lib.rs** — Minimal re-exports (`Config`, `Metrics`, `HealthState`, `wait_for_ctrl_c`, etc.); no public “SDK” surface.
* **src/bin/svc-edge.rs** — Main: boot `supervisor`, parse CLI/env, install router/middleware, serve, handle shutdown.

### Configuration & state

* **src/cli.rs** — Flag parsing and help text mirroring `docs/CONFIG.md` (env → flags → file precedence).
* **src/config.rs** — Typed config, defaults, and `validate()` enforcing caps & amnesia rules (size/time parsers live in `util/`).
* **src/state.rs** — `AppState` snapshot (Arc of `Config`, `Metrics`, handles, allow-list snapshot).
* **src/readiness.rs** — Readiness computation and JSON payload for `/readyz` (ready/degraded + reasons).

### Supervisor & concurrency (small, focused files)

* **src/supervisor.rs** — Spawns listener + workers, wires shutdown (`watch`), exposes bind/join orchestration.
* **src/work/queue.rs** — Bounded `mpsc` work queue (capacity constant and metrics hooks).
* **src/work/worker.rs** — Live-fill worker loop (retry w/ jitter, deadline-aware, cooperative cancel).
* **src/work/shutdown.rs** — Helpers for draining/aborting with timeouts and counters.

### Admission control (Tower/Axum middleware chain)

* **src/admission/body_cap.rs** — Hard cap on request body; 413 on overflow.
* **src/admission/decompress_guard.rs** — Ratio + absolute decompression guards; 400/413 with reason tags.
* **src/admission/inflight_cap.rs** — Concurrency limiter; ties into `edge_inflight` gauges.
* **src/admission/rps_limit.rs** — Token-bucket per-instance RPS guard; emits `rate_limit` rejects.
* **src/admission/timeout.rs** — Per-request deadline (e.g., 5s), ensuring cancel-safety.

### HTTP surface & handlers

* **src/routes/mod.rs** — Router construction, middleware ordering, route constants.
* **src/routes/assets.rs** — `GET /edge/assets/{*path}`: cache hit path, range/conditional handling, miss → enqueue live fill.
* **src/routes/health.rs** — `GET /healthz`: process liveness.
* **src/routes/ready.rs** — `GET /readyz`: real readiness (packs/CAS verified, caps stable).
* **src/routes/prometheus.rs** — `GET /metrics`: Prometheus exposition handler.

### HTTP helpers (wire-correctness, tiny files)

* **src/http/headers.rs** — Corr-ID propagation, common header constants/utilities.
* **src/http/range.rs** — Strict single-range parsing/validation and `Content-Range` construction.
* **src/http/etag.rs** — Strong ETag model (BLAKE3, string encoding/validation).

### Adapters (IO/edge boundary)

* **src/adapters/pack.rs** — PMTiles/MBTiles read adapter: key→byte-range; verify on mount.
* **src/adapters/cas.rs** — Content-addressed store interface (idempotent writes, dedup counters).
* **src/adapters/live_fill.rs** — Allow-list check → HTTPS GET → integrity validate → CAS write → return bytes.
* **src/adapters/tls.rs** — Optional TLS server config (feature-gated) & PQ-hybrid hooks (future).

### Security & governance

* **src/security/cors.rs** — CORS policy (deny-by-default; allow-lists from config).
* **src/security/hsts.rs** — HSTS header injection (when TLS).
* **src/security/audit.rs** — Optional audit sink emitter for live-fill and rejects (tamper-evident records).

### Observability

* **src/metrics.rs** — One struct that registers all counters/gauges/histograms; helper inc/observe fns.
* **src/errors.rs** — `thiserror` enums mapped to HTTP codes + `reason` strings; ensures deterministic rejects.

### Utilities (keep tiny and re-usable)

* **src/util/bytes.rs** — Small buffer/Bytes helpers for zero-copy slices.
* **src/util/size_parse.rs** — Human size parser (KiB/MiB); used by config and guards.
* **src/util/backoff.rs** — Exponential backoff + jitter calculator (normative formula).

### Tests (mirror the IDB + contract)

* **tests/i_*_*.rs** — One file per invariant from `docs/IDB.md` (hardening caps, HTTP semantics, amnesia, etc.).
* **tests/http_contract.rs** — End-to-end for `/edge/assets` (`200/206/304/416/429/503`) + headers (ETag/Range).
* **tests/readiness_logic.rs** — Gates flip correctly for packs verified, CAS reachable, shedding state.
* **tests/concurrency_backpressure.rs** — Bounded queue enq policy (`try_send` → 429 + metrics).
* **tests/fuzz_headers.rs** — Parser fuzz harnesses for `Range`/`If-None-Match`.
* **tests/fixtures/** — Reproducible inputs & golden responses.

### Benches (perf proof points)

* **benches/bench_range.rs** — Range parser benchmarks.
* **benches/bench_blake3.rs** — ETag hashing throughput.
* **benches/bench_pack_read.rs** — Random-access read on packs (chunk sizes).

### Scripts & CI

* **scripts/dev-run.sh** — Local dev launcher with sensible defaults (amnesia on, offline pack).
* **scripts/render-mermaid.sh** — mmdc wrapper to render all `.mmd` → `.svg`.
* **scripts/perf-smoke.sh** — Quick `bombardier`/`wrk` harness.
* **.github/workflows/ci.yaml** — fmt, clippy `-D warnings`, tests, coverage, deny, udeps.
* **.github/workflows/render-mermaid.yaml** — Render diagrams in CI.
* **.github/workflows/perf-guard.yaml** — Nightly perf regression check vs baselines.
* **.github/workflows/public-api.yaml** — Guard against accidental public Rust API surface.

---

## Why this layout works for `svc-edge`

* **Tiny, composable modules:** Each guard/adapter/route is its own ~100–200 line file max. Swapping implementations or tightening caps is trivial.
* **Direct traceability:** Every IDB invariant maps to a **test file**; `_matrix.md` enforces coverage before merge.
* **Operational clarity:** Admission chain is explicit and ordered; readiness has a single authoritative module.
* **Safety by design:** Backpressure is its own concern (queue, rps, inflight) so “reject early, never melt” is enforceable.
* **Docs as code:** Mermaid sources live in `docs/mmd/`; CI renders to `docs/svg/` so engineers and auditors both win.
