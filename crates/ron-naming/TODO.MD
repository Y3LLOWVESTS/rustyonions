

> Core scope recap: `ron-naming` is a **pure library** for naming schemas/normalization/encodings with a **thin, offline CLI (`tldctl`)** for authoring/linting/packing/signing/verifying—no network, no DB, no DHT, no runtime endpoints. `svc-index` owns lookups.  

---

# ron-naming — scaffold (no code)

```
crates/ron-naming/
├─ Cargo.toml
├─ README.md
├─ CHANGELOG.md
├─ LICENSE-APACHE
├─ LICENSE-MIT
├─ CODEOWNERS
├─ .cargo/
│  └─ config.toml
├─ rust-toolchain.toml
├─ deny.toml
├─ src/
│  ├─ lib.rs
│  ├─ types.rs
│  ├─ normalize.rs
│  ├─ address.rs
│  ├─ version.rs
│  ├─ wire/
│  │  ├─ mod.rs
│  │  ├─ json.rs
│  │  └─ cbor.rs
│  ├─ verify/              # gated behind feature = "verify"
│  │  └─ mod.rs
│  └─ bin/
│     └─ tldctl.rs         # gated behind feature = "cli"
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
│  ├─ OLD_README.md
│  └─ diagrams/
│     ├─ arch.mmd
│     └─ arch.svg
├─ examples/
│  ├─ normalize_roundtrip.rs
│  └─ encode_decode.rs
├─ tests/
│  ├─ normalize_idempotence.rs
│  ├─ dto_wire_vectors.rs
│  ├─ address_hygiene.rs
│  └─ cli_contract.rs       # `--features cli` only; spawns tldctl in-process
├─ benches/
│  ├─ normalize_bench.rs
│  └─ encode_bench.rs
├─ testdata/
│  ├─ vectors/
│  │  ├─ names_ascii.json
│  │  ├─ names_unicode_mixed.json
│  │  ├─ tldmap_minimal.json
│  │  ├─ tldmap_minimal.cbor
│  │  └─ vectors.manifest.json
│  └─ signatures/
│     ├─ vectors.b3.txt
│     └─ vectors.attestation.txt
└─ scripts/
   ├─ hash_vectors.sh
   ├─ verify_vectors_attestation.sh
   └─ render_mermaid.sh
```

---

## What each file does (and why)

### Top-level

* **`Cargo.toml`**
  Declares the **lib-first** crate with two opt-in features:

  * `cli` → builds `src/bin/tldctl.rs` (offline only),
  * `verify` → enables signature/verifier traits (no runtime crypto, used for detached verification flows).
    Pins MSRV and serde features; **no network deps** to enforce the “pure lib” boundary.  

* **`README.md`**
  Canonical overview, mermaid diagrams, scope/anti-scope, SemVer/DoD hooks. Matches your finished README, including `tldctl` as a first-class offline CLI and pillar 9 alignment. 

* **`CHANGELOG.md`**
  SemVer log of API/schema/encoding/CLI changes plus migration notes and updated vectors. 

* **`LICENSE-APACHE`, `LICENSE-MIT`**
  Dual-licensing per workspace policy.

* **`CODEOWNERS`**
  OPO/SPR reviewers for contract changes (API and CLI surfaces).

* **`.cargo/config.toml`**
  Workspace lint walls; e.g., deny warnings, consistent RUSTFLAGS for doc tests.

* **`rust-toolchain.toml`**
  Optional pin for CI parity with MSRV.

* **`deny.toml`**
  cargo-deny policy (licenses/advisories) to keep the lib “crypto-free at runtime”. 

---

### `src/` (library: small, surgical modules)

* **`lib.rs`**
  Re-exports **only** the stable surfaces: DTOs, normalization entrypoint, address helpers, wire (de)serialization, and `version` info. Forbids `unsafe_code`. No background tasks; pure functions only.  

* **`types.rs`**
  DTOs that define the naming schema surface: `CanonicalName`, `Label`, `NameRef`, `TldEntry`, `TldMap`, `SignedTldMap<_>`. Enforced `#[serde(deny_unknown_fields)]` at types using wire modules. 

* **`normalize.rs`**
  Deterministic normalization pipeline (`NFC` → `IDNA/UTS-46` non-transitional → lowercase → policy checks for disallowed/confusables → idempotence), exported as a single `normalize_name(...)` entrypoint with `NormalizationOptions`. 

* **`address.rs`**
  Address hygiene helpers like `is_b3_addr(&str) -> bool` enforcing `"b3:<64-lower-hex>"` form only (no hashing of payloads here).  

* **`version.rs`**
  Surfaces policy bundle/Unicode/IDNA table versions so CLI and auditors can print exact provenance, aligned with the API table in docs. 

* **`wire/`**

  * `mod.rs` — common traits/errors for canonical encoders.
  * `json.rs` — canonical JSON encode/decode with stable options.
  * `cbor.rs` — canonical CBOR encode/decode.
    Provides `to_canonical_bytes` / `from_bytes` and the `WireFormat` enum used by CLI/tests. 

* **`verify/`** *(feature = "verify")*
  Detached **Verifier/Signer traits** and hooks needed by `tldctl sign|verify` while keeping runtime crypto out of the lib’s default build. (Attestations are external; this is interface glue only.)  

* **`bin/tldctl.rs`** *(feature = "cli")*
  Offline CLI that wraps library calls for `lint|pack|sign|verify|show`. Reads **stdin/stdout/paths**, supports `--config` (TOML/JSON), and prints deterministic exit codes. **No network or service endpoints.**  

---

### `docs/` (your finished set)

* **`API.md`, `IDB.md`, `INTEROP.md`, `CONFIG.md`, `SECURITY.md`, `PERFORMANCE.md`, `OBSERVABILITY.md`, `CONCURRENCY.md`, `GOVERNANCE.md`, `RUNBOOK.md`, `QUANTUM.md`, `TESTS.md`, `OLD_README.md`**
  Already combined in `ALL_DOCS.md`; the scaffold mirrors these as single sources of truth. Interop reiterates **library-only** (no endpoints) and carries the OAP constants for hosts (context) without coupling.  

* **`diagrams/arch.mmd` & `arch.svg`**
  Mermaid source and rendered artifact referenced from README; CI snippet in README shows rendering automation. 

---

### `examples/`

* **`normalize_roundtrip.rs`**
  Small doctest-style example exercising `normalize_name` idempotence and confusables policy.

* **`encode_decode.rs`**
  Demonstrates `to_canonical_bytes`/`from_bytes` across JSON/CBOR with DTOs.

> Examples are part of the **DX/Interop concern**—quick adoption and verifiable vectors. 

---

### `tests/`

* **`normalize_idempotence.rs`**
  Property-style fixture: `normalize(normalize(x)) == normalize(x)` across ASCII/Unicode sets. Ties directly to IDB invariant [I-2]. 

* **`dto_wire_vectors.rs`**
  Round-trips DTOs against **golden JSON/CBOR** vectors; denies unknown fields; asserts deterministic bytes. 

* **`address_hygiene.rs`**
  Validates `"b3:<hex>"` acceptance and rejects malformed forms, aligning with addressing invariants. 

* **`cli_contract.rs`** *(cfg(feature = "cli"))*
  Asserts `tldctl` subcommands, exit codes, `--version --vectors-hash`, and **no-network** conformance using temp dirs/stdin/stdout. 

> Concurrency DoD: **no background tasks/locks/channels**—tests validate determinism/idempotence under parallel callers. 

---

### `benches/`

* **`normalize_bench.rs`, `encode_bench.rs`**
  Perf sanity for normalization and wire encodes to catch regressions—small, deterministic, CI-visible (e.g., compare p95 across runs).

---

### `testdata/`

* **`vectors/`**
  Canonical vector files (ASCII, mixed-script names; minimal `tldmap` JSON/CBOR) consumed by tests and the CLI. Also a `vectors.manifest.json` enumerating files and checksums.

* **`signatures/`**
  `vectors.b3.txt` holds the **BLAKE3-256** hash of the vector bundle for provenance; `attestation.txt` (optional) is a detached signature (e.g., Dilithium/SPHINCS+) produced by release infra—**verified outside runtime**. 

---

### `scripts/`

* **`hash_vectors.sh`**
  Reproducibly computes and writes `vectors.b3.txt` (library remains crypto-free at runtime, provenance via build/release). 

* **`verify_vectors_attestation.sh`**
  Calls external verifier (CI/local) to check PQ attestation for the vector bundle (optional but documented in DoD). 

* **`render_mermaid.sh`**
  Local helper mirroring the README CI step for diagram rendering. 

---

## Alignment & guardrails (why this structure is “canon-correct”)

* **Strict library boundary + optional offline CLI** (no endpoints, no network/DB) — matches IDB and README scope.  
* **DTOs/encodings & normalization as the public surface**; deterministic, size-bounded wire forms; deny unknown fields.  
* **OAP constants present only as host context**; `ron-naming` stays wire-neutral; `svc-index` enforces ingress at runtime. 
* **Six Concerns mapping** places `ron-naming` under **SEC/GOV** (policy/semantics, not runtime perf paths), which is why there’s no services code here. 
* **No concurrency foot-guns**: zero background tasks, no channels/locks; init via `OnceCell` if needed; tests prove idempotence/determinism. 
* **Vectors + provenance**: golden bundles hashed with BLAKE3; optional PQ attestation handled by release infra, not runtime. 

---

