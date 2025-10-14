
---

# Proposed file tree (no code yet)

```
crates/ron-ledger/
├─ Cargo.toml
├─ README.md
├─ CHANGELOG.md
├─ LICENSE-APACHE
├─ LICENSE-MIT
├─ src/
│  ├─ lib.rs
│  ├─ api.rs
│  ├─ types.rs
│  ├─ config.rs
│  ├─ error.rs
│  └─ engine/
│     ├─ mod.rs
│     ├─ storage.rs
│     ├─ ledger.rs
│     ├─ accumulator.rs
│     ├─ checkpoint.rs
│     ├─ replay.rs
│     └─ observer.rs
├─ tests/
│  ├─ idempotency_prop.rs
│  ├─ replay_recovery.rs
│  ├─ reject_taxonomy.rs
│  ├─ interop_vectors.rs
│  └─ public_api_semver.rs
├─ benches/
│  └─ micro.rs
├─ fuzz/
│  └─ fuzz_targets/
│     └─ replay_parse.rs
├─ examples/
│  └─ minimal.rs
├─ docs/
│  ├─ IDB.md
│  ├─ CONCURRENCY.md
│  ├─ CONFIG.md
│  ├─ SECURITY.md
│  ├─ OBSERVABILITY.md
│  ├─ PERFORMANCE.md
│  ├─ QUANTUM.md
│  ├─ TESTS.md
│  ├─ RUNBOOK.md
│  ├─ INTEROP.md
│  ├─ API.md
│  └─ diagrams/
│     ├─ arch.mmd
│     ├─ ingest-seq.mmd
│     └─ engine-states.mmd
├─ testing/
│  ├─ vectors/
│  │  ├─ happy.json
│  │  ├─ conflict.json
│  │  ├─ malformed.json
│  │  └─ reversible.json
│  └─ performance/
│     ├─ baselines/
│     │  ├─ micronode.json
│     │  └─ macronode.json
│     └─ payloads/
│        └─ ingest_batch_1k.json
├─ .github/
│  └─ workflows/
│     ├─ ci.yml
│     ├─ semver.yml
│     ├─ render-mermaid.yml
│     └─ fuzz.yml
└─ scripts/
   └─ dev-quickcheck.sh
```

---

## What each file/dir is for (and why it exists)

### Root

* **Cargo.toml** — Crate metadata, features, and minimal deps. Declares `default = ["serde"]`, optional `pq-hybrid`, and `arbitrary` (tests only). Matches the README’s feature posture and library-only scope (no HTTP/metrics deps). 

* **README.md** — The library-scoped, canon-aligned readme you supplied (table of contents, acceptance gates, SLOs, diagrams policy). 

* **CHANGELOG.md** — SemVer and wire stability notes; referenced explicitly in README and ALL_DOCS history.  

* **LICENSE-*** — Dual licensing (MIT/Apache-2.0) per README. 

### `src/` — small, single-purpose modules

* **lib.rs** — Re-exports the public surface so consumers can use `ron_ledger::{engine::Ledger, config::LedgerConfig, api::…, error::…}` as in README examples. Keeps the API stable and minimal (DTOs + traits).  

* **api.rs** — *DTOs only* for request/response structs (`IngestRequest`, `IngestResponse`, `RejectReason`). These are library-internal equivalents to the service’s OpenAPI shapes (no transport types), aligned with the reject taxonomy. 

* **types.rs** — Primitive, versioned types: `Entry`, `Seq`, `Root`, `Kid`, `Checksum`, version tags. This clean split keeps serialization footprints obvious and controlled. 

* **config.rs** — `LedgerConfig` (batch, queue cap, limits, accumulator kind, `engine_mode`, optional `pq_mode` feature seam). Library consumes a struct; embedders map env/CLI to it. 

* **error.rs** — `LedgerError` + `RejectReason` enum (typed, machine-countable). Enumerates `Invalid`, `TooLarge`, `Timeout`, `Conflict` to match metrics and service error mapping. 

* **engine/** — Core logic split into tiny units to keep each file focused:

  * **mod.rs** — Public engine surface & doc comments describing sequencing and single-writer rule. 
  * **storage.rs** — The *only* trait the engine depends on (append WAL, persist checkpoint, stream from `seq`, latest checkpoint). This is intentionally storage-agnostic and small. 
  * **ledger.rs** — Orchestration of ingest → validate invariants → append → compute root → checkpoint (when configured). Single-writer commit path. 
  * **accumulator.rs** — Accumulator abstraction (e.g., Merkle today, Verkle-ready) to produce deterministic state roots per commit. 
  * **checkpoint.rs** — Durable snapshots of `(seq, root, ts)`; version-gated migration hooks. 
  * **replay.rs** — Recovery and idempotent replay from WAL/checkpoint; guarantees deterministic root equality across runs. 
  * **observer.rs** — *Hooks only* (no metrics collector). Emits typed events/callbacks the **service** can wire to Prometheus (`commit_latency_seconds`, `ingest_rejects_total{reason}`, etc.). 

### `tests/` — acceptance gates (prove the invariants)

* **idempotency_prop.rs** — Property tests for I-6 (idempotent ingestion), ordered tie-break, and conservation/non-negativity (I-2) via generators. 

* **replay_recovery.rs** — Crash-recovery monotonicity (I-13): checkpoint + WAL replay must yield the *same* `new_root`. 

* **reject_taxonomy.rs** — Asserts mapping of inputs to `RejectReason::{Invalid,TooLarge,Timeout,Conflict}` remains stable and machine-countable. 

* **interop_vectors.rs** — Runs canonical vectors (`happy.json`, `conflict.json`, etc.) to verify DTO compatibility and cross-impl determinism. 

* **public_api_semver.rs** — Snapshot test wired to `public-api`/`cargo semver-checks` to guard breaking changes in the library surface and serialized DTOs. (Reinforces the README “SemVer / changelog” guarantees.) 

### `benches/`

* **micro.rs** — Criterion microbench for commit latency & batching behavior; reproduces the baseline table from README (NVMe persistent vs. amnesia profiles). 

### `fuzz/`

* **fuzz_targets/replay_parse.rs** — libFuzzer target for entry parsing + replay paths (smoke-run in CI). Covers malformed inputs and ensures no panics in decoding/ordering.  

### `examples/`

* **minimal.rs** — Tiny usage showing `Ledger::new(storage, LedgerConfig::default())` and an ingest; mirrors README example so `cargo test --doc` stays green. 

### `docs/` (source-of-truth blueprints)

* **IDB.md** — Invariant Driven Blueprint (I-1..I-15). The exact, enforceable invariants—append-only, conservation, deterministic roots, crash monotonicity—live here (mirrors `ALL_DOCS` section). 

* **CONCURRENCY.md** — Single-writer commit rule; bounded queues; “no locks across `.await`”; backpressure policy and reasoning. (Even as a library, we keep these rules documented for embedders.) 

* **CONFIG.md** — `LedgerConfig` schema and defaults (batch sizes, limits, engine mode, accumulator kind, `pq_mode` seam). 

* **SECURITY.md** — KID-only policy; no key custody; strict input bounds; denial strategy. 

* **OBSERVABILITY.md** — How to turn observer hooks into metrics at the **service** layer (rejects by reason, commit latency, backlog). 

* **PERFORMANCE.md** — Bench baselines, soak targets, and reproduction steps used by `benches/micro.rs`. 

* **QUANTUM.md** — PQ-hybrid policy and adapter seams (verifier lives outside; library remains crypto-agnostic). 

* **TESTS.md** — Defines acceptance gates (property, fuzz, soak), vectors, and CI wiring that tests/ implements. 

* **RUNBOOK.md** — Operator playbooks that apply to the *service wrapper*. Kept here so the library repo remains the canonical spec even if service is separate. 

* **INTEROP.md / API.md** — Wire schemas and mapping notes for DTOs; align error taxonomy with service OpenAPI to prevent drift. 

* **diagrams/*.mmd** — Mermaid sources for architecture, ingest sequence, and engine states; rendered in CI via `render-mermaid.yml`. 

### `testing/`

* **vectors/** — Golden inputs for cross-impl compatibility and replay determinism (e.g., `happy.json`, `conflict.json`, `reversible.json`). 

* **performance/** — Baseline captures for micronode/macronode profiles and sample payloads to reproduce README targets. 

### CI & Scripts

* **.github/workflows/ci.yml** — Build, fmt, clippy (`-D warnings`), tests, `cargo deny`. Mirrors README “Build/Run/Test” and security gates. 

* **semver.yml** — `public-api`/`cargo semver-checks` to prevent accidental breaking changes in the library surface or DTO wire. 

* **render-mermaid.yml** — Renders `docs/diagrams/*.mmd` to SVGs in CI (as in README). 

* **fuzz.yml** — Smoke-runs fuzz targets (build + minimal run) so regressions surface early. 

* **scripts/dev-quickcheck.sh** — Local guardrail: fmt, clippy, tests, deny (exact command set from README’s quickcheck block). 

---

## Why this layout hits the canon (in short)

* **Library, not service.** No HTTP/metrics in crate; only DTOs, traits, engine, and hooks. Service wrappers (e.g., `svc-ledger`) consume this. 

* **Invariant-driven.** Tests mirror I-1..I-15: append-only, conservation, idempotency, deterministic roots, replay monotonicity. 

* **Single-writer & bounded queues.** The design—and module names—reinforce the single committer and deterministic sequence order. 

* **Reject taxonomy is first-class.** Centralized in `error.rs`, consumed by services for metrics/alerts and stable wire mapping. 

* **PQ-hybrid ready.** A feature-gated seam exists (`pq-hybrid`) but crypto remains in adapters/services; library stays KID-only. 

* **Docs as code.** Mermaid sources and CI renderer are included so diagrams don’t drift. 

---

