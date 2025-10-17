

---

# ron-app-sdk — scaffold (no code)

```
crates/ron-app-sdk/
├─ Cargo.toml
├─ README.md
├─ CHANGELOG.md
├─ LICENSE-APACHE
├─ LICENSE-MIT
├─ .gitignore
├─ rust-toolchain.toml
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
│  ├─ api-history/
│  │  └─ ron-app-sdk/
│  │     └─ v0.1.0.txt
│  └─ diagrams/
│     ├─ arch.mmd
│     ├─ sequence_put.mmd
│     └─ lifecycle.mmd
├─ src/
│  ├─ lib.rs
│  ├─ context.rs
│  ├─ errors.rs
│  ├─ config/
│  │  └─ mod.rs
│  ├─ retry.rs
│  ├─ idempotency.rs
│  ├─ tracing.rs
│  ├─ metrics.rs
│  ├─ cache/
│  │  ├─ mod.rs
│  │  └─ lru.rs
│  ├─ transport/
│  │  └─ mod.rs
│  ├─ planes/
│  │  ├─ storage.rs
│  │  ├─ edge.rs
│  │  ├─ mailbox.rs
│  │  └─ index.rs
│  ├─ ready.rs
│  └─ types.rs
├─ examples/
│  └─ demo.rs
├─ tests/
│  ├─ i_1_profile_parity.rs
│  ├─ i_2_caps_required.rs
│  ├─ i_3_oap_bounds.rs
│  ├─ i_4_content_addressing.rs
│  ├─ i_5_retries_deadlines.rs
│  ├─ i_6_dto_strictness.rs
│  ├─ i_7_transport_agnostic.rs
│  ├─ i_8_deadlines_everywhere.rs
│  ├─ i_9_error_taxonomy.rs
│  ├─ i_10_semver_snapshot.rs
│  ├─ i_11_no_persistence.rs
│  ├─ i_12_canon_deps.rs
│  ├─ prop_retry_bounds.rs
│  ├─ prop_oap_frames.rs
│  ├─ interop_vectors.rs
│  └─ vectors/
│     ├─ oap1_min_req.bin
│     ├─ oap1_min_req.json
│     └─ capability_example.json
├─ benches/
│  └─ sdk_benches.rs
├─ fuzz/
│  ├─ fuzz_targets/
│  │  ├─ oap_frame_parser.rs
│  │  └─ dto_roundtrip.rs
│  └─ Cargo.toml
├─ scripts/
│  ├─ compare_bench.py
│  └─ gen_api_snapshot.sh
└─ .github/
   └─ workflows/
      ├─ ci.yml
      ├─ render-mermaid.yml
      ├─ perf-guardrails.yml
      └─ api-semver.yml
```

---

## What each file does (and why)

### Top level

* **Cargo.toml** — Declares crate, features (`default=["tls"]`, `tor`, `metrics`, `pq`, `pq-hybrid`), MSRV, and workspace deps; pins policy-approved crates. Drives SemVer and feature gating called out in API/IDB. 
* **README.md** — The paste-ready README you finalized (role, architecture, metrics, SLOs, diagrams, examples). Single source for contributors. 
* **CHANGELOG.md** — Release notes; includes migration notes when API/behavior changes (ties to `api-history`). 
* **LICENSE-*** — Dual-license headers per project standard.
* **rust-toolchain.toml** — Locks MSRV to `1.80.0` for consistent builds; enforced in CI. 

### docs/ (canon blueprints in-repo)

* **API.md** → Public surface spec + SemVer rules; contains snapshot procedure (`cargo public-api`). Output synced to `docs/api-history/…`. 
* **CONCURRENCY.md** → Invariants (no lock across `.await`, bounded retries, cancel-safe), validation strategy, Loom scope. 
* **CONFIG.md** → Typed config schema, env keys, defaults, validation, and reload semantics; mirrors `src/config`. 
* **GOVERNANCE.md** → Idempotency/limit SLAs, authority boundaries, change control & hotfix process. 
* **IDB.md** → Invariants I-1..I-12 and DoD gates; maps to tests under `tests/`. 
* **INTEROP.md** → OAP/1 limits, endpoints, DTO strictness, canonical vectors placed in `tests/vectors/`. 
* **OBSERVABILITY.md** → Metric names, tracing fields, redaction rules; pairs with `src/metrics.rs`/`src/tracing.rs`. 
* **PERFORMANCE.md** → SLOs, benches, flamegraph workflow, CI regression gates. 
* **QUANTUM.md** → PQ posture (hybrid KEX), feature flags, rollout, and downgrade metrics. 
* **RUNBOOK.md** → Operational guidance for hosts (readiness mapping, failure modes). 
* **SECURITY.md** → Threat model, capability handling, logging redaction, supply chain. 
* **TESTS.md** → Strategy for unit/prop/fuzz/Loom; coverage floors; chaos tests. 
* **diagrams/*.mmd** → Mermaid sources: architecture, common PUT sequence, client lifecycle (rendered in CI). 
* **api-history/ron-app-sdk/v0.1.0.txt** → First `cargo public-api` snapshot (keeps SemVer honest). 

### src/ (small, focused modules)

* **lib.rs** — Tiny façade: re-exports public types, constructs the client, wires metrics/tracing hooks, and exposes planes. No heavy logic here; keeps API surface crisp per API.md. 
* **context.rs** — `SdkContext { profile, amnesia }` metadata accessors; never used to branch semantics (I-1). 
* **errors.rs** — `SdkError` (non-exhaustive) and mapping from wire/transport; centralizes retry classification (NoRetry/Retriable). 
* **config/mod.rs** — Typed `SdkConfig` + `from_env()` + `validate()`; env precedence and fail-closed rules exactly as CONFIG.md. 
* **retry.rs** — Full-jitter backoff calculator and deadline helpers; guarantees attempts ≤ max and sleeps within caller’s overall deadline (I-5/I-8). 
* **idempotency.rs** — Key generation/formatting and header/envelope injection helpers for mutations; never generates two keys for the same logical op (governance I-G1). 
* **tracing.rs** — Span/field helpers (`endpoint`, `attempt`, `deadline_ms`, `corr_id` propagation), redaction policy toggles. 
* **metrics.rs** — Optional Prometheus handle with golden counters/histograms (SDK-prefixed, low-cardinality labels). 
* **cache/mod.rs** — Ephemeral, bounded TTL cache interface; never bypasses capability checks; may verify BLAKE3 on terminal PUT success. 
* **cache/lru.rs** — Small local LRU implementation or thin wrapper to a no-std LRU crate; metrics for hits/evictions. 
* **transport/mod.rs** — Thin adapter over `ron-transport` to select TLS vs Tor, inject timeouts, and enforce OAP frame cap pre-flight (1 MiB). No server/listener code. 
* **planes/storage.rs** — Public async fns: `storage_get/put` with content addressing and optional terminal verify; streams ≈64 KiB; respects idempotency & retries. 
* **planes/edge.rs** — `edge_get` with optional byte ranges; tight DTO/wire mapping; no surprises. 
* **planes/mailbox.rs** — `send/recv/ack` trio; at-least-once semantics via idempotency keys and conflict mapping. 
* **planes/index.rs** — `index_resolve` for logical key → `AddrB3`; simple request/response flow. 
* **ready.rs** — `ReadyReport` plus lightweight checks (`config`, `transport`, `tor?`) to help hosts implement `/readyz`. 
* **types.rs** — Re-exports of `ron-proto` DTOs (`Capability`, `IdemKey`, `AddrB3`, etc.) to present a single import surface and keep DTO hygiene centralized. 

### examples/

* **demo.rs** — Minimal end-to-end use (builds only with examples feature): config parse, client `new()`, a GET/PUT roundtrip; doubles as docs test in CI. 

### tests/ (traceability: IDB → files)

* **i_*.rs** — One file per invariant from IDB.md (I-1..I-12). Names mirror invariants; assertions prove behavior (e.g., frame cap, strict DTO, no persistence). A small `build.rs` (repo-level) can enforce coverage of all invariants listed. 
* **prop_*.rs** — Property tests: retry sleep sum < overall deadline; frames around 1 MiB bound; idempotency dedup under injected faults. 
* **interop_vectors.rs** — Runs canonical wire vectors from `tests/vectors/` to ensure stable mapping and strict decode. 
* **vectors/** — Canonical OAP/1 and capability examples referenced by INTEROP.md; used by both unit and property tests. 

### benches/

* **sdk_benches.rs** — Criterion micro-bench for hot path (e.g., storage put/get via loopback transport). CI compares baselines and enforces ±10–15% guardrails per PERFORMANCE.md. 

### fuzz/

* **fuzz_targets/** — `oap_frame_parser` and `dto_roundtrip` fuzz targets for envelope/DTO safety; minimal runtime, run on PRs with time caps. 
* **Cargo.toml** — Separate fuzz workspace for `cargo-fuzz`.

### scripts/

* **compare_bench.py** — Small helper to diff Criterion outputs vs baselines in CI; fails on p95 deltas beyond budget. 
* **gen_api_snapshot.sh** — Calls `cargo public-api` and writes to `docs/api-history/…`; used in release CI. 

### .github/workflows/

* **ci.yml** — Build/test/lint/coverage (`-D warnings`, proptests, minimal-versions, MSRV) plus `cargo deny`, `public-api`, and `semver-checks`. 
* **render-mermaid.yml** — Renders `docs/diagrams/*.mmd` to SVG on PRs; artifacts attached. 
* **perf-guardrails.yml** — Runs benches, compares to baselines, uploads flamegraphs on schedule. 
* **api-semver.yml** — Fails if public surface changes without a changelog & version bump. 

---

## Notes on modularity & maintainability

* **One concern per file.** The *planes* directory isolates Storage/Edge/Mailbox/Index; swapping or evolving a plane doesn’t ripple across the crate. 
* **Adapters stay thin.** Transport/metrics/tracing modules are tiny shims; core logic (retries, deadlines, idempotency) lives in dedicated files and is testable in isolation. 
* **Docs enforce code.** Each docs blueprint has a direct counterpart in `src/` and a matching test in `tests/`, per BUILDPLAN and IDB traceability. 
