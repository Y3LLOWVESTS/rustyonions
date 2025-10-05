

---

# `crates/svc-storage/` — Final Scaffold (no code)

```
crates/svc-storage/
├─ Cargo.toml
├─ README.md
├─ CHANGELOG.md
├─ LICENSE-APACHE
├─ LICENSE-MIT
├─ .gitignore
├─ .editorconfig
├─ rust-toolchain.toml
├─ .github/
│  └─ workflows/
│     ├─ ci.yml
│     ├─ contract-apis.yml
│     ├─ perf.yml
│     ├─ concurrency-guardrails.yml
│     ├─ redteam-fuzz.yml
│     ├─ render-mermaid.yml
│     ├─ quantum.yml
│     └─ coverage.yml
├─ configs/
│  ├─ svc-storage.example.toml
│  ├─ pq-hybrid.toml
│  ├─ validate.sh
│  └─ profiles/
│     ├─ micronode.toml
│     └─ macronode.toml
├─ docs/
│  ├─ ALL_DOCS.md
│  ├─ README.linkmap.md
│  ├─ API.md
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
│  ├─ FACETS.md
│  ├─ ECONOMICS.md
│  ├─ arch.mmd
│  ├─ sequence.mmd
│  ├─ state.mmd
│  ├─ specs/
│  │  ├─ OAP-1.md
│  │  └─ vectors/
│  │     ├─ chunking-64k.txt
│  │     ├─ frame-1MiB.txt
│  │     └─ range-requests.txt
│  ├─ openapi/
│  │  ├─ svc-storage.yaml
│  │  └─ history/
│  │     └─ v0.1.0-http.json
│  └─ api-history/
│     └─ svc-storage/
│        └─ v0.1.0-libapi.txt
├─ src/
│  ├─ main.rs
│  ├─ prelude.rs
│  ├─ version.rs
│  ├─ readiness.rs
│  ├─ errors.rs
│  ├─ types.rs
│  ├─ config.rs
│  ├─ metrics.rs
│  ├─ amnesia.rs
│  ├─ bus.rs
│  ├─ tls/
│  │  ├─ mod.rs
│  │  ├─ server_config.rs
│  │  └─ pq.rs
│  ├─ uds/
│  │  ├─ mod.rs
│  │  ├─ server.rs
│  │  └─ peercred.rs
│  ├─ http/
│  │  ├─ mod.rs
│  │  ├─ server.rs
│  │  ├─ error.rs
│  │  ├─ extractors.rs
│  │  ├─ middleware.rs
│  │  └─ routes/
│  │     ├─ mod.rs
│  │     ├─ get_object.rs
│  │     ├─ head_object.rs
│  │     ├─ put_object.rs
│  │     ├─ post_object.rs
│  │     ├─ health.rs
│  │     ├─ ready.rs
│  │     ├─ metrics.rs
│  │     └─ version.rs
│  ├─ storage/
│  │  ├─ mod.rs
│  │  ├─ cas.rs
│  │  ├─ fs.rs
│  │  ├─ io.rs
│  │  ├─ compression.rs
│  │  ├─ cache.rs
│  │  ├─ placement.rs
│  │  ├─ replication.rs
│  │  ├─ erasure.rs
│  │  ├─ repair.rs
│  │  ├─ hedged.rs
│  │  └─ pq_envelope.rs
│  ├─ auth/
│  │  ├─ mod.rs
│  │  └─ macaroon.rs
│  └─ policy/
│     ├─ mod.rs
│     ├─ quotas.rs
│     ├─ residency.rs
│     └─ economics.rs
├─ benches/
│  ├─ read_path.rs
│  ├─ write_path.rs
│  └─ etag_range.rs
└─ testing/
   ├─ integration/
   │  ├─ http_get_head_put.rs
   │  ├─ range_tests.rs
   │  ├─ error_caps.rs
   │  └─ profile_matrix.rs
   ├─ performance/
   │  ├─ fixtures/
   │  │  └─ zip_bomb.bin
   │  ├─ scripts/
   │  │  ├─ range.lua
   │  │  └─ media_facet.sh
   │  ├─ run_load.sh
   │  └─ compare_baselines.sh
   └─ fuzz/
      ├─ range_fuzz/
      ├─ decomp_fuzz/
      └─ addr_fuzz/
```

---

## File-by-file roles (tight, modular, future-proof)

### Root & Repo Hygiene

* **Cargo.toml** — Service crate manifest; enables feature flags (`amnesia`, `pq-hybrid`, `uds`) and pins workspace deps/MSRV. No public Rust API exposed.
* **README.md** — Human-readable contract: role, endpoints, invariants (1 MiB body cap, ~64 KiB chunks, ≤10× safe decompression), ops knobs, SLOs.
* **CHANGELOG.md** — SemVer & compatibility log; marks HTTP/metrics breaking changes explicitly.
* **LICENSE-APACHE / LICENSE-MIT** — Dual-license policy.
* **.gitignore** — Standard Rust/CI/artifacts ignore list.
* **.editorconfig** — Consistent whitespace, charset, line endings across editors.
* **rust-toolchain.toml** — Locks MSRV (e.g., 1.80.0) to stop toolchain drift locally and in CI.

### CI / Quality Gates (`.github/workflows/`)

* **ci.yml** — Build, fmt, clippy (deny-wall), tests, cargo-deny; fails on warnings.
* **contract-apis.yml** — Diffs `openapi/svc-storage.yaml` and `docs/api-history` to enforce contract discipline; optional `cargo public-api` to confirm “no public Rust API.”
* **perf.yml** — Runs load tests (range-heavy profile) and compares p95/p99 to baselines; fails if regression > threshold.
* **concurrency-guardrails.yml** — Loom/sanitizers; forbids locks across `.await` and common foot-guns.
* **redteam-fuzz.yml** — CI fuzz budget for ranges, decompression, address parsing.
* **render-mermaid.yml** — Renders `arch.mmd`, `sequence.mmd`, `state.mmd` for docs PRs.
* **quantum.yml** — Matrix job toggling `pq-hybrid` feature to smoke-test PQ config paths & transport selection.
* **coverage.yml** — Collects coverage (e.g., grcov) and gates at ≥85% for core modules per TESTS.md.

### Configs (`configs/`)

* **svc-storage.example.toml** — Canonical config sample; every knob documented (timeouts, caps, directories, RPS/Inflight ceilings, hedging).
* **pq-hybrid.toml** — Example enabling PQ-hybrid transport/auth envelopes; demonstrates how PQ does **not** change object identity (BLAKE3).
* **validate.sh** — Lightweight CI helper that validates TOML schema and forbids unsafe combinations (e.g., `amnesia=true` with persistent paths).
* **profiles/micronode.toml** — Single-tenant profile defaults: amnesia ON, conservative limits, in-memory read cache only.
* **profiles/macronode.toml** — Multi-tenant profile defaults: persistent data dirs, replication/repair knobs, hedged reads tuned for tail latency.

### Docs (`docs/`)

* **ALL_DOCS.md** — Combined canonical reference (drift guard).
* **README.linkmap.md** — Quick links map for contributors (what to read when).
* **API.md** — Rust (internal) types/DTOs and stability policy (service exposes HTTP contract, not Rust API).
* **CONFIG.md** — Every setting with type, default, scope, and validation notes.
* **GOVERNANCE.md** — Policy sources (residency, quotas), decision boundaries, update channels.
* **IDB.md** — Invariants/MUSTs/Anti-scope; HOW principles; PROOF gates and testable metrics.
* **INTEROP.md** — Wire contracts: `GET/HEAD /o/{b3}`, `PUT /o/{b3}`, `POST /o`, plus `/metrics`, `/healthz`, `/readyz`, `/version`; headers/validators.
* **OBSERVABILITY.md** — Golden signals (`requests_total`, `latency_seconds`, `rejected_total{reason}`, `integrity_fail_total`, `rf_target/observed`), exemplar dashboards.
* **PERFORMANCE.md** — SLOs (byte-range p95), hedged read policy, chunk sizing rationale.
* **QUANTUM.md** — PQ posture: BLAKE3 identity unaffected; PQ-hybrid transport/auth envelopes feature-gated; rollout plan and tests.
* **RUNBOOK.md** — Ops playbook: startup, readiness DAG, common incidents (disk full, hot shard), repair pacing, feature flags.
* **SECURITY.md** — Threat model (STRIDE), capability-guarded writes, safe decompression, UDS peercred, TLS guidance.
* **TESTS.md** — Bronze/Silver/Gold gates; coverage targets; fuzz/soak/chaos matrices; canonical repros.
* **FACETS.md** — Media + Trust & Safety SLOs (range-heavy traffic patterns, reject taxonomy, ceilings).
* **ECONOMICS.md** — Read-only settlement signals via bus events (bytes stored/read, RF achieved); **no** on-chain or ledger logic here.
* **arch.mmd / sequence.mmd / state.mmd** — Mermaid diagrams (rendered in CI).
* **specs/OAP-1.md** — Service-local OAP/1 notes; pointers to normative spec.
* **specs/vectors/** — **Canonical test vectors** for chunking, framing, and byte-range semantics.
* **openapi/svc-storage.yaml** — HTTP contract source of truth.
* **openapi/history/v0.1.0-http.json** — Frozen first contract snapshot for diffing.
* **api-history/svc-storage/v0.1.0-libapi.txt** — Confirms empty/stable Rust surface (service boundary).

### Source — crate composition (`src/`)

> **Design rule:** tiny files, single responsibility, feature-gated where appropriate. The service exposes **HTTP**; Rust API is internal.

Core

* **main.rs** — Composes config → metrics → transport(s) → routes; sets readiness to **fail-closed** for writes under degradation.
* **prelude.rs** — Shared imports/aliases to keep modules lean.
* **version.rs** — `/version` data (build, features booleans like `amnesia`, `pq-hybrid`).
* **readiness.rs** — Readiness DAG; toggles `/readyz` and coordinates with amnesia/persistence constraints.
* **errors.rs** — Structured error envelope; consistent mapping to 4xx/5xx and retry hints.
* **types.rs** — Local DTOs/types for handlers, config, metrics tags (no public API).
* **config.rs** — Env+TOML merge, defaults, and strong validation (caps, decompression, chunk size).
* **metrics.rs** — Registers golden signals + storage gauges (`rf_target`, `rf_observed`, integrity/reject counters).
* **amnesia.rs** — Enforces “amnesia mode” (no disk spill); flips readiness if spill would occur.
* **bus.rs** — Emits/consumes kernel events (Health/Shutdown/ConfigUpdated), and settlement hooks for economics policy.

TLS & UDS

* **tls/mod.rs** — TLS module surface.
* **tls/server_config.rs** — `tokio_rustls::rustls::ServerConfig` builder; TLS 1.3 suites per policy.
* **tls/pq.rs** — **Feature-gated (`pq-hybrid`)** configuration stub for PQ-hybrid suites (design only; no impl).
* **uds/mod.rs** — UDS surface.
* **uds/server.rs** — UDS listener wiring for intra-node traffic.
* **uds/peercred.rs** — SO_PEERCRED extraction/validation for local peer identity.

HTTP

* **http/mod.rs** — HTTP surface wiring.
* **http/server.rs** — Axum/Hyper composition; timeouts, concurrency caps, graceful shutdown.
* **http/error.rs** — Maps internal errors → HTTP status + envelope.
* **http/extractors.rs** — Range parsing; capability token extraction (macaroon).
* **http/middleware.rs** — **1 MiB** body cap; safe decompression (≤10× ratio + absolute cap); rate/Inflight ceilings.
* **http/routes/mod.rs** — Route registry.
* **http/routes/get_object.rs** — `GET /o/{b3}` with validators and Range semantics; `ETag: "b3:<hex>"`.
* **http/routes/head_object.rs** — `HEAD /o/{b3}` for validators/length without body.
* **http/routes/put_object.rs** — Idempotent `PUT /o/{b3}`; digest verify-on-write; capability required.
* **http/routes/post_object.rs** — `POST /o` “compute-address” helper returning `b3:<hex>`.
* **http/routes/health.rs** — Liveness.
* **http/routes/ready.rs** — Readiness (fail-closed logic for writes).
* **http/routes/metrics.rs** — Prometheus exposition.
* **http/routes/version.rs** — Build/feature booleans (non-fingerprintable).

Storage Engine (CAS)

* **storage/mod.rs** — Storage surface (trait bounds kept small for testability).
* **storage/cas.rs** — CAS primitives; identity/ETag tie to BLAKE3; idempotency rules.
* **storage/fs.rs** — Disk layout and path derivation across `data_dirs`.
* **storage/io.rs** — Streaming I/O in **~64 KiB** chunks; bounded buffers; backpressure interface.
* **storage/compression.rs** — Safe decompression guard (ratio + absolute cap).
* **storage/cache.rs** — Simple, pluggable read-through cache interface (feature-ready).
* **storage/placement.rs** — Object placement planning hooks (single-node now; RF hooks later).
* **storage/replication.rs** — Replication control (stubbed policy; gauges updates).
* **storage/erasure.rs** — EC planning hooks (feature-gated later).
* **storage/repair.rs** — Repair pacing knobs; readiness integration.
* **storage/hedged.rs** — Hedged read policy to pull down p99 tail.
* **storage/pq_envelope.rs** — Feature-gated design stub for optional PQ-aware client-side encryption envelope (does **not** affect object identity).

Auth & Policy

* **auth/mod.rs** — Auth surface.
* **auth/macaroon.rs** — Capability verification for writes (no ambient bypass).
* **policy/mod.rs** — Policy surface.
* **policy/quotas.rs** — Quota/backpressure logic; increments `rejected_total{reason}` consistently.
* **policy/residency.rs** — Residency/region policy validation; config updates.
* **policy/economics.rs** — **Read-only** settlement signals (bytes stored/read, RF achieved) emitted on bus; no ledger logic inside.

### Benches (`benches/`)

* **read_path.rs** — Criterion: steady-state p95 GET, cold/warm cache.
* **write_path.rs** — Criterion: PUT latency under 1 MiB cap, with verification cost included.
* **etag_range.rs** — Criterion: conditional GET + Range with strong validators.

### Tests / Fuzz / Perf (`testing/`)

* **integration/http_get_head_put.rs** — Round-trip for `GET/HEAD/PUT/POST`, validators, idempotency.
* **integration/range_tests.rs** — Byte-range correctness (single/multi, boundaries, 416 taxonomy).
* **integration/error_caps.rs** — 413/429/503/Decompression guard rejections; retry hints.
* **integration/profile_matrix.rs** — Executes full suite against **micronode** and **macronode** profiles (parity guarantee).
* **performance/fixtures/zip_bomb.bin** — Controlled expansion corpus for safe decompression tests.
* **performance/scripts/range.lua** — Range-heavy load profile for perf rigs.
* **performance/scripts/media_facet.sh** — Media facet profile (seeky workloads, CDN-like patterns).
* **performance/run_load.sh** — One-shot perf runner with thresholds.
* **performance/compare_baselines.sh** — Compares current p95/p99 to stored baselines.
* **fuzz/range_fuzz/** — Fuzzer corpus/target for Range parser.
* **fuzz/decomp_fuzz/** — Fuzzer for decompression guard edge cases.
* **fuzz/addr_fuzz/** — Fuzzer for BLAKE3 address parsing/validation.

---

## Canon-critical invariants this scaffold enforces (documented & testable)

* **Immutability by contract:** CAS has **no DELETE** route; mutability belongs to higher policy services.
* **Identity equals BLAKE3:** `ETag == "b3:<hex>"`; enabling `pq-hybrid` never changes object identity or DTOs.
* **Safety ceilings:** 1 MiB request body cap; safe decompression ≤10× ratio **and** absolute byte cap; ~64 KiB streaming chunks.
* **Fail-closed readiness:** Any condition that risks unsafe writes (e.g., spill under amnesia) flips readiness and 503s mutating routes.
* **Observability:** Golden metrics + RF gauges are first-class and wired to policy/repair hooks (ready for dashboards).
* **Interop discipline:** OpenAPI + vectors + API history gates ensure wire compatibility and drift-free evolution.

