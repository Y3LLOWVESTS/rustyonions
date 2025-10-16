

---

# ron-accounting — crate scaffold (no code, descriptions only)

```text
ron-accounting/
├─ Cargo.toml
├─ README.md
├─ CHANGELOG.md
├─ LICENSE-APACHE
├─ LICENSE-MIT
├─ .gitignore
├─ .cargo/
│  └─ config.toml
├─ src/
│  ├─ lib.rs
│  ├─ errors.rs
│  ├─ metrics.rs
│  ├─ readiness.rs
│  ├─ normalize.rs
│  ├─ utils/
│  │  ├─ time.rs
│  │  ├─ hashing.rs
│  │  └─ encode.rs
│  ├─ accounting/
│  │  ├─ mod.rs
│  │  ├─ labels.rs
│  │  ├─ dimensions.rs
│  │  ├─ recorder.rs
│  │  ├─ window.rs
│  │  ├─ slice.rs
│  │  └─ rollover.rs
│  ├─ exporter/
│  │  ├─ mod.rs
│  │  ├─ trait.rs
│  │  ├─ router.rs
│  │  ├─ lane.rs
│  │  ├─ worker.rs
│  │  └─ ack_lru.rs
│  ├─ wal/                       # feature = "wal"
│  │  ├─ mod.rs
│  │  ├─ segment.rs
│  │  ├─ replay.rs
│  │  └─ fs.rs
│  └─ config/
│     ├─ mod.rs
│     ├─ schema.rs
│     ├─ validate.rs
│     └─ load.rs
├─ tests/
│  ├─ unit/
│  │  ├─ recording_tests.rs
│  │  ├─ rollover_tests.rs
│  │  ├─ exporter_ordering_tests.rs
│  │  └─ wal_tests.rs
│  ├─ prop/
│  │  ├─ encoding_prop.rs
│  │  └─ labels_prop.rs
│  └─ loom/
│     ├─ router_model.rs
│     └─ shutdown_model.rs
├─ benches/
│  ├─ record.rs
│  └─ seal.rs
├─ examples/
│  ├─ minimal.rs
│  └─ export_to_mock.rs
├─ tests/
│  └─ vectors/
│     └─ ron-accounting/
│        ├─ sliceV1_dagcbor.json
│        ├─ sliceV1_dagcbor.bin
│        ├─ sliceV1_msgpack.bin
│        ├─ sliceV1_digest.txt
│        └─ oap1_rollover_evt.bin
├─ docs/
│  ├─ API.MD
│  ├─ CONFIG.MD
│  ├─ CONCURRENCY.MD
│  ├─ GOVERNANCE.MD
│  ├─ IDB.md
│  ├─ INTEROP.MD
│  ├─ OBSERVABILITY.MD
│  ├─ OLD_README.md
│  ├─ PERFORMANCE.MD
│  ├─ QUANTUM.MD
│  ├─ RUNBOOK.MD
│  ├─ SECURITY.MD
│  ├─ TESTS.MD
│  └─ diagrams/
│     ├─ arch.mmd
│     ├─ sequence.mmd
│     └─ state.mmd
├─ ops/
│  └─ alerts/
│     └─ ron-accounting.yaml
├─ .github/
│  └─ workflows/
│     ├─ clippy.yml
│     ├─ render-mermaid.yml
│     ├─ public-api.yml
│     ├─ coverage.yml
│     └─ perf-gate.yml
└─ scripts/
   ├─ perf_gate.py
   └─ generate_vectors.rs
```

---

## What each file does (tight roles)

### Root & tooling

* **Cargo.toml** — Defines features: `export-serde`, `ledger-wire`, `wal`, `http-adapter` (optional), `metrics`, `otel`; pins MSRV; workspace deps.
* **README.md** — Your finalized crate README (role, SLOs, diagrams, gates). 
* **CHANGELOG.md** — SemVer entries + migration notes (tie to API snapshots).
* **LICENSE-*** — Dual MIT/Apache-2.0.
* **.cargo/config.toml** — Per-crate RUSTFLAGS (deny warnings, loom cfg switch), target runner tweaks.

### Core surfaces (library)

* **src/lib.rs** — Minimal façade & re-exports: `accounting`, `exporter`, `config`, `metrics`, `readiness`, `errors`; feature-gates kept here. (Matches stable API surface.) 
* **src/errors.rs** — `#[non_exhaustive]` error enums + `Result<T>`; variants include `Busy`, `OrderOverflow`, `Timeout`, `DuplicateExport`, `PersistenceFull`, `WalCorrupt`, `SchemaViolation`, `Other`. (Typed taxonomy.) 
* **src/metrics.rs** — One-time Prometheus registration (golden metrics, handles clonable). 
* **src/readiness.rs** — Readiness keys & snapshot (`ReadyKey::{ConfigLoaded,QueuesBoundedOk,ExporterOk,WalOk,BoundaryTickerOk}`). 
* **src/normalize.rs** — Label normalization/templating (e.g., route → `/v1/objects/:id`), the guard against cardinality blow-ups (no PII in labels). 
* **src/utils/time.rs** — UTC boundary math, skew tolerance (≤500ms), monotone boundary counter helpers. 
* **src/utils/hashing.rs** — BLAKE3 helpers for `b3` and chain `prev_b3`, zero-field preimage rule. 
* **src/utils/encode.rs** — Canonical DAG-CBOR / MsgPack encoding wrappers; deny unknown fields; 1 MiB cap helpers. 

### Accounting domain (hot path + slicing)

* **src/accounting/mod.rs** — Public module root; exports `Recorder`, `Window`, `SliceId`, `Row`, `SealedSlice`.
* **src/accounting/labels.rs** — Small builder for `(tenant, method, route, service, region)`; stored compactly; saturating counters. 
* **src/accounting/dimensions.rs** — `const` string dimensions: `bytes`, `requests`, `cpu_units` (future-ready). 
* **src/accounting/recorder.rs** — Sharded, atomic counters; strict bounds; no `.await` on hot path; zero/low alloc path. 
* **src/accounting/window.rs** — Time-sliced ring buffers + rollover policy (fixed windows 1–60m; restart to change). 
* **src/accounting/slice.rs** — `SealedSlice` (opaque), `SliceMeta` (prev_b3, version, amnesia flag), canonical encode + digest. 
* **src/accounting/rollover.rs** — Boundary detection + sealing; optional background ticker with cooperative cancel. (Single small task; library can also expose manual rollover hook.) 

### Exporter pipeline (ordered + idempotent)

* **src/exporter/mod.rs** — Public glue & type aliases for lanes/streams.
* **src/exporter/trait.rs** — `trait Exporter` + `Ack::{Ok,Duplicate}`; extension-trait slot for batch put (future, non-breaking). 
* **src/exporter/router.rs** — Drains `pending_slices` into per-stream ordered lanes; cross-tenant fairness (RR / WFQ). Bounded queues or shed. 
* **src/exporter/lane.rs** — Per `(tenant,dimension)` bounded MPSC with ordering guard + small state (next `seq`, backlog depth).
* **src/exporter/worker.rs** — Single writer per stream; `put()` with jittered backoff, global 10s deadline, dup handling; emits latency histograms. 
* **src/exporter/ack_lru.rs** — Finite replay horizon cache for `(slice_id,b3)` idempotency (opaque newtype). 

### WAL (Macronode only; feature = `wal`)

* **src/wal/mod.rs** — Public handle; quotas & stats; amnesia auto-off gate. 
* **src/wal/segment.rs** — Length-delimited, checksummed records (append-only); atomic rename sealing; fsync policy. 
* **src/wal/replay.rs** — Startup scan; checksum verify; skip corrupt frames; re-enqueue pending; metrics. 
* **src/wal/fs.rs** — Bounded `spawn_blocking` fs ops with deadlines; 0700 perms validation. 

### Config (env/file/flags, fail-closed)

* **src/config/mod.rs** — Public surface (`Config`, `validate`, profile probes).
* **src/config/schema.rs** — Typed structs (accounting/exporter/wal/http-adapter) with serde defaults. 
* **src/config/validate.rs** — Fail-closed rules (windows 60–3600s; shards power-of-two; amnesia ⇒ force WAL disabled; TLS file perms). 
* **src/config/load.rs** — Loader helpers (env → file → defaults precedence); effective-config logging. 

> **Why this split?** Keeps each file short; isolates policy, IO, and hot path per the “bounded everything & no await-locks” rules. 

---

## Tests, benches, examples, vectors

* **tests/unit/*** — Focused invariants:

  * `recording_tests.rs` (monotone in-slice; saturating adds),
  * `rollover_tests.rs` (single rollover per boundary; skew tolerance),
  * `exporter_ordering_tests.rs` (no `N+1` before `N`; shed on overflow),
  * `wal_tests.rs` (replay, checksum skip, quotas). 
* **tests/prop/*** — Property tests:

  * `encoding_prop.rs` (canonical bytes stable; deny unknown fields),
  * `labels_prop.rs` (normalizer never yields PII; cardinality bounded). 
* **tests/loom/*** — Concurrency models (router/worker/shutdown; no lock across `.await`). 
* **benches/*** — Criterion micro-benches (`record.rs`, `seal.rs`) with baselines used by perf gate. 
* **examples/*** — Minimal crate usage; mock exporter sketch (doc-tested in README). 
* **tests/vectors/ron-accounting/** — Canonical interop vectors: DAG-CBOR bytes, digest, OAP/1 frame. (Prevents wire drift.) 

---

## Docs, ops, CI

* **docs/*** — Your 13 crate docs as the source of truth (already provided). Keep Mermaid `.mmd` under `docs/diagrams/` and render in CI. 
* **ops/alerts/ron-accounting.yaml** — PromQL alerts (degraded, backlog, exporter latency, WAL usage, rejects spike). 
* **.github/workflows/**

  * `clippy.yml` (deny warnings & `await_holding_lock`),
  * `render-mermaid.yml` (mmdc step),
  * `public-api.yml` (cargo-public-api diff gate),
  * `coverage.yml` (tarpaulin ≥85% target),
  * `perf-gate.yml` (regression thresholds vs baseline). 
* **scripts/perf_gate.py** — Compares bench JSONs; fails PR on >10% latency regression. 
* **scripts/generate_vectors.rs** — Dev-only tool to (re)mint canonical DTO vectors & digest file. 

---

## Why this structure works (brief rationale)

* **Library-first, tiny files**: hot path (`recorder.rs`) is isolated; IO/adapters/WAL are feature-gated so they never bloat core. 
* **Ordered + idempotent export** is physically embodied in `exporter/{router,lane,worker}.rs`, each < ~250 LOC when implemented, keeping logic testable and composable. 
* **Bounded everything** is enforced by design: one module per queue/cache with clear caps; all `try_send`/shed paths are observable via `metrics.rs`. 
* **Amnesia vs WAL** is not a forked codebase—just a folder behind a feature with hard “amnesia ⇒ no WAL” enforcement in `validate.rs`. 
* **Interop stability** is guarded by the vectors folder + CI snapshot gates, mapping directly to `INTEROP.MD` contract. 
* **Concurrency safety** is modeled (loom) where it matters (router/worker/shutdown), not blanket across the crate. 

---

