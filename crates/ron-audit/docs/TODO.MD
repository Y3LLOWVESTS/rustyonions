
# ron-audit — scaffolded file tree (no code)

```
crates/ron-audit/
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
├─ docs/
│  ├─ IDB.md
│  ├─ SECURITY.md
│  ├─ CONCURRENCY.md
│  ├─ CONFIG.md
│  ├─ INTEROP.md
│  ├─ OBSERVABILITY.md
│  ├─ PERFORMANCE.md
│  ├─ GOVERNANCE.md
│  ├─ QUANTUM.md
│  ├─ RUNBOOK.md
│  └─ api-history/
│     └─ ron-audit/
│        └─ v1.0.2.txt
├─ src/
│  ├─ lib.rs
│  ├─ errors.rs
│  ├─ dto.rs
│  ├─ canon/
│  │  ├─ mod.rs
│  │  ├─ rules.rs
│  │  └─ vectors.rs
│  ├─ hash/
│  │  ├─ mod.rs
│  │  └─ b3.rs
│  ├─ verify/
│  │  ├─ mod.rs
│  │  ├─ record.rs
│  │  └─ chain.rs
│  ├─ bounds/
│  │  └─ mod.rs
│  ├─ sink/
│  │  ├─ mod.rs
│  │  ├─ traits.rs
│  │  ├─ ram.rs           # feature = "wal" OFF path still uses this
│  │  ├─ wal.rs           # feature = "wal"
│  │  └─ export.rs        # feature = "export"
│  ├─ stream/
│  │  └─ mod.rs
│  ├─ privacy/
│  │  └─ mod.rs
│  ├─ metrics/
│  │  └─ mod.rs
│  └─ prelude.rs
├─ tests/
│  ├─ append_only.rs
│  ├─ canonicalization.rs
│  ├─ bounds.rs
│  ├─ idempotency.rs
│  ├─ multi_writer_ordering.rs
│  ├─ privacy_policies.rs
│  ├─ export_checkpoints.rs
│  └─ api_compat.rs
├─ fuzz/
│  ├─ fuzz_targets/
│  │  ├─ fuzz_record_roundtrip.rs
│  │  └─ fuzz_canon_vectors.rs
│  └─ Cargo.toml
├─ loom/
│  └─ chain_loom.rs
├─ benches/
│  ├─ hash_b3.rs
│  ├─ verify_chain.rs
│  └─ wal_batching.rs
└─ testing/
   ├─ perf/
   │  └─ perf_plan.md
   ├─ chaos/
   │  └─ chaos_plan.md
   └─ vectors/
      ├─ record_small.json
      ├─ record_max.json
      └─ manifest_example.json
```

## What each file/dir is for (brief, modular roles)

### Top-level

* **Cargo.toml** — lib-only crate; features `wal`, `export`, and (future) `pq-sign` adapters remain additive and off-by-default (SemVer gates) .
* **README.md** — concise purpose/surface; makes lib-only boundary explicit (no sockets/HTTP/CLI) and shows Micronode (RAM) vs Macronode (WAL/export) posture  .
* **CHANGELOG.md** — ties to `cargo public-api` snapshots; every surface change is SemVer-classified and justified .
* **deny.toml / rust-toolchain.toml / .cargo/config.toml** — keep MSRV 1.80.0 consistent and lints strict for public docs (`#![deny(missing_docs)]`) in CI .

### docs/ (canonical, already written in your combined file—these filenames keep one-doc-per-topic discipline)

* **IDB.md** — normative invariants: append-only, canonical DTO with `deny_unknown_fields`, chaining, idempotent dedupe, profile-aware amnesia, per-writer ordering, strict payload bounds .

* **SECURITY.md** — threat model + boundaries; no secrets or keys in this crate; hosts sign checkpoints; queue/bounds/timeouts hardening checklist   .

* **CONCURRENCY.md** — small-lib “lite mode” rules; never hold locks across `.await`; shed-not-block philosophy belongs to hosts but tested via acceptance gating .

* **CONFIG.md** — host-supplied posture/env matrix (`AUDIT_*` knobs) and validation rules; Micronode forbids WAL; export cadence/dirs; privacy salt file hygiene   .

* **INTEROP.md** — defines DTOs/manifests/checkpoints, on-disk WAL/export formats, canonicalization rules, and bus events; provides test vectors for external tools .

* **OBSERVABILITY.md / PERFORMANCE.md** — governance counters, SLOs, perf gates; no >10% regressions; freeze & recovery SLAs wired into ops narratives  .

* **GOVERNANCE.md** — integrity power bounds (N-of-M actions), redactions via tombstones, appeals path, and Perfection Gates tie-ins  .

* **QUANTUM.md** — signer/verifier adapter traits (host-provided keys; PQ-ready envelope) under features; cross-sign rotation timeline (roadmap)  .

* **RUNBOOK.md** — ops drills (latency, disk-full, storm, bitrot), freeze ≤60s and restore ≤15m, healthy metrics snapshots and example logs  .

* **api-history/ron-audit/** — stores `cargo public-api` snapshots to enforce surface stability in CI .

### src/ (small modules; no background tasks)

* **lib.rs** — crate prelude/re-exports: `AuditSink`, `AuditStream`, `canon`, `hash`, `verify`, `bounds`, `privacy`, error types. Public surface is intentionally small and SemVer-guarded  .
* **errors.rs** — typed errors (`AppendError`, `VerifyError`, `BoundsError`) referenced as API contracts (no stringly errors)  .
* **dto.rs** — imports or type-aliases the canonical `AuditRecord` schema (from `ron-proto`) and any crate-local thin wrappers; DTOs are `serde(deny_unknown_fields)` per canon .
* **canon/** — normalization rules (NFC strings, field order, no floats) + frozen test vectors so implementations can re-derive bytes identically .
* **hash/** — BLAKE3 helpers: `b3_no_self(&record) -> "b3:<hex>"` and stable `dedupe_key`; these are API-visible invariants .
* **verify/** — record and chain validation (prev/self_hash linkage, canonicality check); functions are pure and non-mutating .
* **bounds/** — centralized limits (`attrs ≤ 1 KiB`, canonical record ≤ 4 KiB) with helpers used by hosts before append .
* **sink/**

  * **traits.rs** — `AuditSink`/`AuditStream` abstractions (append-only; return new head; expose snapshot-only state) .
  * **ram.rs** — in-memory chain state for Micronode, bounded memory profile (no disk).
  * **wal.rs** *(feature = "wal")* — host-managed WORM/WAL bindings and batching knobs (still “lib-only”: no threads spawned here).
  * **export.rs** *(feature = "export")* — deterministic manifest/checkpoint helpers (no key custody here; only envelope fields) .
* **stream/** — per-(writer,stream) ordering helpers and deterministic total-order key derivation used by hosts on ingest .
* **privacy/** — IP handling policies (`drop`, prefix, salted-hash) and attr redaction helpers aligned with SECURITY.md .
* **metrics/** — types/names for governance counters and verify/shedding gauges that **hosts** will register and increment (crate itself does not own a Prometheus registry) .
* **prelude.rs** — a small “use-this” import surface for hosts to keep their files tidy.

### tests/

Acceptance gates are codified as small, focused tests:

* **append_only.rs** — prove `(prev → self_hash)` continuity and that mutations yield verification failure (fatal) .
* **canonicalization.rs** — vectors for canon bytes and stable IDs .
* **bounds.rs** — enforce size limits and reject floats per spec .
* **idempotency.rs** — dedupe key invariants .
* **multi_writer_ordering.rs** — prove stream ordering rules under merge key .
* **privacy_policies.rs** — verify PII/IP handling modes and salt requirements (validation failures on misconfig) .
* **export_checkpoints.rs** — manifest/root determinism and cross-derivation from WAL .
* **api_compat.rs** — snapshot surface checks against `docs/api-history`.

### fuzz/

* **fuzz_record_roundtrip.rs** — DTO → canon → verify → roundtrip stays stable; rejects unknown fields.
* **fuzz_canon_vectors.rs** — exercises tricky unicode/floats/ordering edge cases.

### loom/

* **chain_loom.rs** — tiny loom model for concurrent append imposing single-writer-per-stream semantics (ensures we never “re-order” within a stream) — complements repo-wide concurrency rules .

### benches/

* **hash_b3.rs** — baseline BLAKE3 throughput.
* **verify_chain.rs** — chain-verify p95 targets.
* **wal_batching.rs** — macro-only path; informs host defaults (batch size/fsync cadence) .

### testing/

* **perf/** & **chaos/** plans — operational drills for latency injection, disk-full, storms, and bitrot with success criteria and dashboards to attach in RUNBOOK (Gate J) .
* **vectors/** — canonical sample records and manifests for public, reproducible verification by SDKs/tools .

---

## Why this layout fits the canon

* **Lib-only boundary, no endpoints**: enforced by README and SECURITY; all ports/metrics belong to *hosts* (e.g., `svc-gateway`)  .
* **Evidence invariants** (append-only, canonicalization, chaining, bounds, idempotency) are isolated into dedicated, tiny modules with acceptance tests, matching the IDB and API contracts  .
* **Profiles** (Micronode vs Macronode) map cleanly to `sink/ram.rs` and feature-gated `sink/wal.rs` + `sink/export.rs` with config validation matrices and forbidden combinations (e.g., amnesia + WAL) .
* **Governance/PQ-readiness** live in docs + adapter traits (no key custody here; hosts provide signers), keeping public API small and future-proof .
* **CI teeth**: `docs/api-history` + `cargo public-api` gates ensure SemVer discipline, with CHANGELOG links required on any surface drift .

