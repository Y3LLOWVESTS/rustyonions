
# ron-kms — Scaffolded File Tree (structure only)

```
crates/
└─ ron-kms/
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
   ├─ src/
   │  ├─ lib.rs
   │  ├─ types.rs
   │  ├─ error.rs
   │  ├─ metrics.rs
   │  ├─ config.rs
   │  ├─ traits/
   │  │  ├─ mod.rs
   │  │  ├─ keystore.rs
   │  │  ├─ signer.rs
   │  │  ├─ verifier.rs
   │  │  ├─ kem.rs
   │  │  └─ hybrid.rs
   │  ├─ sealed/
   │  │  ├─ mod.rs
   │  │  ├─ header.rs
   │  │  ├─ aead.rs
   │  │  ├─ anti_rollback.rs
   │  │  └─ store.rs
   │  ├─ backends/
   │  │  ├─ mod.rs
   │  │  ├─ memory.rs
   │  │  ├─ file.rs
   │  │  └─ pkcs11.rs              # feature-gated
   │  ├─ ops/
   │  │  ├─ create.rs
   │  │  ├─ rotate.rs
   │  │  ├─ attest.rs
   │  │  ├─ sign.rs
   │  │  ├─ verify.rs
   │  │  ├─ wrap.rs
   │  │  └─ unwrap.rs
   │  ├─ pq/
   │  │  ├─ mod.rs
   │  │  ├─ mlkem.rs               # feature "mlkem"
   │  │  ├─ mldsa.rs               # feature "mldsa"
   │  │  └─ slhdsa.rs              # feature "slhdsa"
   │  ├─ util/
   │  │  ├─ zeroize.rs
   │  │  ├─ ct.rs                  # constant-time helpers
   │  │  └─ time.rs                # monotonic + UTC stamps
   │  └─ prelude.rs
   ├─ docs/
   │  ├─ IDB.md
   │  ├─ INTEROP.md
   │  ├─ QUANTUM.md
   │  ├─ SECURITY.md
   │  ├─ PERFORMANCE.md
   │  ├─ CONFIG.md
   │  ├─ OBSERVABILITY.md
   │  ├─ CONCURRENCY.md
   │  ├─ API.md
   │  ├─ RUNBOOK.md
   │  ├─ GOVERNANCE.md
   │  ├─ PQ_MIGRATION.md
   │  ├─ arch.mmd
   │  └─ arch.svg
   ├─ tests/
   │  ├─ unit/
   │  │  ├─ sealed_header_test.rs
   │  │  ├─ rotate_test.rs
   │  │  ├─ attest_test.rs
   │  │  └─ zeroize_test.rs
   │  ├─ interop_kats.rs
   │  └─ vectors/
   │     ├─ ed25519/
   │     ├─ mlkem/
   │     └─ mldsa/
   ├─ benches/
   │  ├─ sign_bench.rs
   │  ├─ verify_bench.rs
   │  ├─ encap_bench.rs
   │  └─ decap_bench.rs
   ├─ fuzz/
   │  ├─ Cargo.toml
   │  └─ fuzz_targets/
   │     ├─ sealed_header.rs
   │     └─ dto_sign.rs
   ├─ xtask/
   │  └─ src/main.rs               # helpers: KAT pack, perf gates, diagram build
   └─ testing/                      # dev-only harnesses; no library code depends on these
      └─ kms-dev-server/           # separate crate (HTTP) mapping DTOs ↔ ron-kms traits
         ├─ Cargo.toml
         └─ src/main.rs
```

## What each file/folder is for (and why it exists)

### Top level

* **Cargo.toml** — Pure library. Enables features `mlkem`, `mldsa`, `slhdsa`, `soft-seal`, `with-metrics`. No HTTP deps (service lives in `testing/kms-dev-server` or a separate `svc-kms`) .
* **README.md** — Your canonical role statement and governance/SemVer guardrails; reiterates “no network code in this crate,” harness/service out of tree; includes env matrix and CI snippets  .
* **deny.toml / rust-toolchain.toml / .cargo/config.toml** — Lock toolchain and deny bad deps to keep custody core minimal and reproducible (per canon hardening & supply-chain gates) .
* **LICENSE-APACHE / LICENSE-MIT / CODEOWNERS / CHANGELOG.md** — Standard governance hygiene per repo norms.

### `src/` (library-only, small files)

* **lib.rs** — Re-exports public API surface (traits/types/prelude), matching “library-only” stance.
* **types.rs** — `KeyId/Kid`, `Alg`, sealed record header enums/IDs, DTO-adjacent types kept local to the crate (pure types; shared DTOs that cross crates stay in `ron-proto`) .
* **error.rs** — Error taxonomy exactly like your README table (e.g., `NoSuchKey`, `SealedCorrupt`, `AlgUnavailable`, `Expired`, `Entropy`) .
* **metrics.rs** — Register **once** and expose handles for `kms_ops_total`, `kms_failures_total`, `kms_op_latency_seconds`; used across ops paths (aligns with your acceptance gates) .
* **config.rs** — Parses env knobs used by convenience constructors: `RON_KMS_STORE`, `RON_KMS_PQ_MODE`, `RON_KMS_ROTATE_DAYS`, `RON_KMS_AMNESIA`, `RON_KMS_METRICS`, plus PKCS#11 vars; includes PQ rollout toggles (`KMS_PQ_PRODUCE_HYBRID`, `KMS_PQ_VERIFY_MODE`, `KMS_PQ_ROLLOUT_PERCENT`)  .
* **prelude.rs** — Tiny prelude exporting common traits/types for ergonomic use by callers.

#### Traits (stable core surface)

* **traits/mod.rs** — Module glue for public traits.
* **traits/keystore.rs** — “create/import/rotate/get/attest” interface that backs the custody lifecycle in your docs (deterministic audit chain) .
* **traits/signer.rs, verifier.rs** — Stateless sign/verify trait surfaces; deterministic API for Ed25519/ML-DSA bundles per policy (hybrid emission) .
* **traits/kem.rs, hybrid.rs** — KEM/Decap + Hybrid KEM (X25519+ML-KEM) abstraction; aligns to “hybrid by default” wrap/unwrap path in endpoints .

#### Sealed custody primitives

* **sealed/mod.rs** — Exposes sealed-store API.
* **sealed/header.rs** — Versioned sealed blob header (`magic="RONK"`, `version`, `alg`, `hybrid`, `created_at`, `not_after`, `kid`, `pubkey_len`, `aead_nonce`, `aead_tag`) — parse/emit; rejects unknown/malformed (KATs cover it) .
* **sealed/aead.rs** — Domain-separated AEAD with explicit nonce policy and replay/nonce-reuse guards per kid#vN .
* **sealed/anti_rollback.rs** — Anti-rollback markers/checkpoints referenced in your docs (attested continuity) .
* **sealed/store.rs** — Trait defining the sealed store interface; used by backends (RAM/file/HSM). Enforces **non-exportable** semantics (handles for HSM/TPM; sealed blobs for file) and amnesia rules (RAM-only when on) .

#### Backends (pluggable, feature-gated where needed)

* **backends/memory.rs** — RAM-only store (micronode default when amnesia=ON) .
* **backends/file.rs** — Soft-seal file store (fsync on mutate; sealed blobs at rest) .
* **backends/pkcs11.rs** — HSM adapter behind a feature, using object handles (non-exportable); offloads any blocking to `spawn_blocking` per hardening checklist .

#### Ops (tiny orchestrators invoking traits/backends)

* **ops/create.rs / rotate.rs** — KID scheme and versioning (`tenant/purpose/alg/uuid#vN`), idempotent rotation via client op_id; record attested continuity .
* **ops/attest.rs** — Optional attestation bundle stamped with slot/firmware evidence when available .
* **ops/sign.rs / verify.rs** — Policy-driven hybrid behavior (emit dual sigs; verify AND/OR/PQ-only); no network or macaroon logic here—capability decisions live above this layer .
* **ops/wrap.rs / unwrap.rs** — AEAD+Hybrid KEM envelopes; default Hybrid KEM path per endpoints table .

#### PQ modules

* **pq/** — Algorithm adapters: **ML-KEM**, **ML-DSA**, **SLH-DSA** (all behind features). Exposes only trait impls; config controls policy (Classical / Hybrid / PQ-only) with rollout %, AND/OR verify toggles and metrics labeling, matching your PQ plan .

#### Utilities

* **util/zeroize.rs** — Zeroize helpers; page-lock when allowed; “no Debug/Display for secrets” discipline .
* **util/ct.rs** — Constant-time compares for tags/sigs per security mechanics .
* **util/time.rs** — Monotonic + UTC stamping; skew tolerance notes (used in headers/attest) .

### `docs/` (drift-proofed)

* **INTEROP.md** — Points to canonical DTOs and OAP norms; **library has no wire code**; harness/service mirror the same DTOs and caps (1 MiB body cap; deny_unknown_fields; correlation, etc.) .
* **QUANTUM.md / PQ_MIGRATION.md** — Phased migration (Classical → Hybrid-OR → Hybrid-AND → PQ-only) and config/metrics knobs called out in README/ALL_DOCS .
* **SECURITY.md / HARDENING hooks** — Global limits (5s/512/500 rps/1 MiB/10× decompress) are documented for **services** that wrap this library; KMS library documents custody controls (zeroize, sealed, amnesia) .
* **PERFORMANCE.md** — Perf gates and drift thresholds (e.g., +20% Ed25519 verify, +25% ML-KEM decap), and flamegraph guidance, as in README .
* **RUNBOOK.md** — Rotation, rollback from sealed corruption, HSM slot/PIN issues, amnesia hygiene; mirrors Troubleshooting section .
* **CONCURRENCY.md** — Reiterates “no blocking in async” and lock discipline; loom targets (opt-in) .
* **arch.mmd / arch.svg** — The small flowchart from README rendered to SVG via CI .

### Tests, benches, fuzz

* **tests/unit/** — Zeroization; sealed header parse/emit; rotate without loss; attest continuity; constant-time checks—maps to acceptance gates you listed .
* **tests/interop_kats.rs & tests/vectors/** — KATs for Ed25519/ML-KEM/ML-DSA and cross-impl verification per your compatibility section .
* **benches/** — p95 latency budgets for sign/verify/encap/decap; ties into perf drift gates (Ed25519 ±20%; ML-KEM ±25%) .
* **fuzz/** — Targets: sealed header decoder, DTO strict parsing (deny_unknown_fields) per acceptance gates .

### Dev harness (out-of-tree, not linked by library)

* **testing/kms-dev-server/** — Minimal Axum HTTP harness mapping DTOs ↔ traits; used for curl, soak, fuzz orchestration; explicitly **not** part of the library build, matching your “no server in library” rule  .

---

## Why this scaffold matches the canon

* **Library-only** boundary is enforced (no HTTP in crate; harness/service separate). Your README calls this out explicitly and even shows how to run the dev harness as a separate crate .
* **Pure custody + PQ-hybrid** traits mirror your endpoints table (sign/verify/wrap/unwrap, hybrid by policy) while keeping capability/policy enforcement out of the library (policy lives in `ron-policy`; caps in `ron-auth`/`svc-passport`) .
* **Sealed at rest, non-exportable, amnesia-aware** backends line up with your key handling and storage rules (object handles in HSM; RAM-only under amnesia) .
* **Hardening defaults & limits** are documented and applied in harness/service layers, not inside the library, per the Hardening Blueprint (timeouts, caps, decompression guard, etc.) .
* **Perf, fuzz, loom, KATs** directories directly satisfy your acceptance gates and CI hooks (perf drift thresholds, fuzz targets, loom for lock-ordering, vectors for interop)  .
* **Pillar alignment** keeps ron-kms in **Pillar 3 (Identity & Keys)** as a library—no scope creep into gateway/transport (per project canon list) .
