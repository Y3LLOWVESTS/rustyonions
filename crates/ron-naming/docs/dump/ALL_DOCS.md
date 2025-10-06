# Combined Markdown

_Source directory_: `crates/ron-naming/docs`  
_Files combined_: 13  
_Recursive_: 0

---

### Table of Contents

- API.MD
- CONCURRENCY.MD
- CONFIG.MD
- GOVERNANCE.MD
- IDB.md
- INTEROP.MD
- OBSERVABILITY.MD
- OLD_README.md
- PERFORMANCE.MD
- QUANTUM.MD
- RUNBOOK.MD
- SECURITY.MD
- TESTS.MD

---

## API.MD
_File 1 of 13_



---

````markdown
---
title: API Surface & SemVer Reference — ron-naming
status: draft
msrv: 1.80.0
last-updated: 2025-10-06
audience: contributors, auditors, API consumers
---

# API.md

## 0. Purpose

This document captures the **public API surface** of `ron-naming`:

- Snapshot of exported functions, types, traits, modules (library) + CLI commands/flags (tldctl).
- SemVer discipline and change classes with CI gates.
- Alignment with the IDB invariants and CONFIG contract.

> Scope recap (from IDB): `ron-naming` is a **library for naming schemas, normalization, validation, and signed governance artifacts**, with a **thin, offline CLI** (`tldctl`) for authoring/linting/packing/signing/verifying. No runtime lookups; **no network/DB/DHT** here. 

---

## 1. Public API Surface

> Source of truth for intent and invariants: IDB §1 **Invariants**; verifier traits and core DTOs are defined there for stability. 

### 1.1 Library (Rust) — Modules, Types, Traits, Fns (intended stable set for 0.1.x)

```text
# Modules (top-level)
ron_naming::types
ron_naming::normalize
ron_naming::address
ron_naming::wire      # canonical (de)serializers: JSON/CBOR
ron_naming::verify    # Verifier/Signer traits; helpers (feature "verify")
ron_naming::version   # policy/Unicode/IDNA table version info

# Core DTOs (types)
pub struct CanonicalName(String)                       # normalized, NFC+IDNA, lowercased
pub struct Label(String)                               # single normalized label
pub enum   NameRef { Human { name: CanonicalName }, Address { b3: String } }
pub struct TldEntry { pub tld: Label, pub owner_key_id: String, pub rules_version: u32 }
pub struct TldMap   { pub version: u64, pub entries: Vec<TldEntry> }
pub struct SignedTldMap<S> { pub body: TldMap, pub signatures: Vec<S> }

# Normalization (single entrypoint)
pub struct NormalizationOptions { /* policy bundle + strict/mixed-script knobs */ }
pub fn normalize_name(input: &str, opts: &NormalizationOptions) -> Result<CanonicalName, ValidationError>

# Address hygiene
pub fn is_b3_addr(s: &str) -> bool                      # true if "b3:<64-lower-hex>"

# Canonical encodings (wire)
pub enum WireFormat { Json, Cbor }
pub fn to_canonical_bytes<T: Serialize>(t: &T, fmt: WireFormat) -> Result<Vec<u8>, WireError>
pub fn from_bytes<T: DeserializeOwned>(buf: &[u8], fmt: WireFormat) -> Result<T, WireError>

# Sign/Verify (feature "verify")
pub trait Verifier<Sig> { fn verify(&self, canonical_body: &[u8], sig: &Sig) -> bool; }
pub trait Signer<Sig>   { fn sign  (&self, canonical_body: &[u8]) -> Result<Sig, SignError>; }
pub fn verify_signed_map<V,S>(v: &V, m: &SignedTldMap<S>) -> bool
  where V: Verifier<S>, S: Clone;

# Version info
pub struct VersionInfo { pub unicode: &'static str, pub idna: &'static str, pub confusables: &'static str }
pub fn version_info() -> VersionInfo
````

**Provenance in IDB (normative snippets):**

* Pure library + feature-gated CLI boundary and normalization pipeline/invariants.
* Canonical wire forms + BLAKE3 address guard.
* DTO & trait sketches (Verifier/Signer; SignedTldMap).

### 1.2 CLI (tldctl, feature `cli`) — Commands & Flags (offline only)

**Subcommands (stable contract):**
`lint <in>`, `pack <in> [--out <out>]`, `sign <in> [--out <out>]`, `verify <in>`, `show <in>`
(Inputs via explicit file path or `-` for stdin; outputs default to stdout or `--out` path.)

**Global & domain flags (canonical):**
`--config`, `--format json|cbor`, `--pretty`, `--in`, `--out`, normalization flags (`--strict`, `--allow-mixed-scripts`, `--allowed-scripts`, `--policy-bundle`, `--custom-tables-dir`), signing/verification flags (`--sign`, `--sign-profile`, `--m`, `--n`, `--key-id ...`, `--verify`, `--pq-mode off|hybrid`), logging flags (`--log-format`, `--log-level`).

**Config precedence and env prefixes:** flags > env (`TLDCTL_*` > `RON_NAMING_*`) > file > defaults.

**CLI design constraints:** one-shot, **no network**, deterministic outputs, machine-friendly exit codes (0/2/3/64+).

---

## 2. SemVer Discipline

**Additive (Minor / Non-Breaking):**

* New modules/functions/traits behind a non-default feature (e.g., `verify`, `pq`).
* Extending enums/structs guarded with `#[non_exhaustive]` patterns (if introduced).
* New CLI subcommands/flags that do not alter existing behavior.

**Breaking (Major):**

* Removing/renaming public items or changing function signatures/trait bounds.
* Tightening validation behavior in ways that alter canonical outputs (wire compatibility).
* Changing DTO field names or serde attributes that break round-trip invariants.
* Making previously extensible enums/structs exhaustive.

**Patch:**

* Docs, internal performance changes, bug fixes that preserve canonical bytes and error taxonomy.

> CI gate: `cargo public-api` diff must be acknowledged for any change (IDB acceptance gate G-6).

---

## 3. Stability Guarantees

* **MSRV:** 1.80.0; enabling `cli` must **not** raise MSRV or violate workspace pins.
* **No unsafe:** library `#![forbid(unsafe_code)]`; clippy denies `unwrap_used/expect_used`.
* **DTO hygiene:** `#[serde(deny_unknown_fields)]` on all public DTOs.
* **Canonical stability:** JSON/CBOR encodings are stable & size-bounded; address format is `"b3:<64-hex>"`.
* **No network/DB/DHT:** applies to both lib and CLI (offline, explicit I/O only).

---

## 4. Invariants (API-Relevant)

* **One normalization path** (`normalize_name`) shared by lib, tests, and CLI.
* **Address hygiene** helper (`is_b3_addr`) rejects near-misses (case, length, prefix).
* **Verifier/Signer traits** allow PQ backends (e.g., Dilithium) via `ron-kms` features; library stores **no keys**.
* **CLI subcommand set** is fixed to `lint|pack|sign|verify|show`; exit codes documented.

---

## 5. Tooling

* **cargo public-api** — detect surface diffs; run with `--simplified` in CI. (Acceptance gate G-6.)
* **cargo semver-checks** — optional layer to validate semantic compatibility.
* **cargo doc** — examples mirror `tldctl` workflows.
* **API snapshots** — store under `docs/api-history/ron-naming/vX.Y.Z-libapi.txt` (+ `...-cli.txt` for flags/subcommands).

---

## 6. CI & Gates

* PR pipeline runs `cargo public-api`; any diff triggers a bot comment with added/removed symbols.
* Failing gates without CHANGELOG notes are rejected.
* Unicode/IDNA/confusables **pins** require updated vectors alongside any API that depends on them.

---

## 7. Acceptance Checklist (DoD)

* [ ] Current API snapshot generated & stored under `/docs/api-history/ron-naming/`.
* [ ] SemVer classification performed (additive / breaking / patch).
* [ ] CI gate passes (`cargo public-api`).
* [ ] CHANGELOG updated for any surface or behavioral change.
* [ ] Docs/tests updated (normalize, wire, verify).
* [ ] CLI surface (subcommands/flags) re-snapshotted if `cli` changed.

---

## 8. Appendix

**References (canon):**

* IDB §1 Invariants & DTO/Verifier sketches; §4 Acceptance gates; §5 Anti-scope.
* CONFIG — CLI precedence, flags, features; sample config; PQ/verify interactions.
* Interop & Hardening blueprints — crate split (naming vs. index), OAP constants context.

**Perfection Gates tie-in:**

* Gate G — Public API documented & snapshotted.
* Gate H — Breaking changes acknowledged (major bump).
* Gate J — CHANGELOG alignment enforced.

**History Section (create as versions land):**

* v0.1.0 — Initial public DTOs (`CanonicalName`, `Label`, `TldMap`, `SignedTldMap<S>`), normalization entrypoint, address hygiene helper, canonical (de)serializers, verifier/signature traits behind `verify`; `tldctl` with `lint|pack|sign|verify|show`.

```

---


---

## CONCURRENCY.MD
_File 2 of 13_

---

title: Concurrency Model — ron-naming
crate: ron-naming
owner: Stevan White
last-reviewed: 2025-10-06
status: draft
template_version: 1.1
msrv: 1.80.0
tokio: "N/A (library is sync); CLI uses blocking I/O only"
loom: "0.7+ (dev-only; minimal models)"
lite_mode: "This crate is a small library + optional one-shot CLI; §§2,6,7 are N/A"
-----------------------------------------------------------------------------------

# Concurrency Model — ron-naming

`ron-naming` is a **pure, synchronous library** for name normalization/validation and DTOs for TLD governance artifacts, plus an **optional one-shot CLI** (`tldctl`) that wraps those functions for offline authoring (lint/pack/sign/verify). There are **no background tasks, no async runtime, no channels**, and **no shared mutable state** beyond short-lived local variables. This document makes that explicit and provides copy-paste patterns to keep it that way.

> Golden rule (still applies): if any future async enters this crate, **never hold a lock across `.await`** and prefer message passing over shared mutability.

---

## 0) Lite Mode

This crate qualifies for **Lite Mode**:

* Completed: **§1 Invariants**, **§3 Channels**, **§4 Locks**, **§5 Timeouts**, **§10 Validation**, **§11 Code Patterns**, **§15 CI & Lints**.
* Marked **N/A**: **§2 Runtime**, **§6 Shutdown**, **§7 I/O framing** (no network), **§8 Error taxonomy (RPC flavor)** trimmed, **§9 Metrics** trimmed, **§12 Config hooks** summarized, **§14 diagrams** simplified.

---

## 1) Invariants (MUST)

* **[I-1] No async runtime required.** Library APIs are **pure/sync**. The CLI is **one-shot**, uses **blocking std I/O**, and must not spawn background tasks.
* **[I-2] No global mutable state.** No `static mut`, no `lazy_static` with interior mutability for live mutation. Policy/Unicode tables are **read-only** after construction.
* **[I-3] Single-writer rule for caches (if introduced).** Any optional cache must be **construct-then-freeze** (e.g., `OnceCell<Arc<Table>>`). No post-publish mutation.
* **[I-4] Reentrancy & thread safety.** All public functions are **reentrant** and **`Send + Sync` friendly** (no shared interior mutability visible to callers).
* **[I-5] Determinism under concurrency.** Given the same inputs and policy tables, results are **bit-for-bit identical** regardless of caller concurrency.
* **[I-6] No blocking on shared locks.** If any lock is ever introduced (discouraged), guard sections must be **micro-scoped** and **never** cross FFI or hypothetical `.await`.
* **[I-7] Bounded memory.** No unbounded queues/buffers. Parsers validate sizes; canonical encoders allocate proportionally to input with hard caps.
* **[I-8] Cancel/Abort safety.** CLI operations are **idempotent** and can be aborted at process level without leaving partial in-memory state (outputs go to temp path then atomically rename if we ever add file writing beyond stdout).
* **[I-9] No task leaks.** (Trivial here) The CLI must **not** detach background work.
* **[I-10] Platform independence.** No reliance on thread-locals for correctness.

---

## 2) Runtime Topology — **N/A**

* No Tokio runtime, listeners, workers, or supervisors.

---

## 3) Channels & Backpressure

There are **no channels** in the library or CLI by design.

| Name   | Kind | Capacity | Producers → Consumers | Backpressure | Drop Semantics |
| ------ | ---- | -------: | --------------------- | ------------ | -------------- |
| *None* | —    |        — | —                     | —            | —              |

Guidelines retained (should this ever change):

* Prefer **bounded** `mpsc` with `try_send()` and an explicit `Busy` error over buffering.
* If a broadcast is ever introduced (unlikely here), track lag/drop metrics.

---

## 4) Locks & Shared State

**Allowed (only if needed later):**

* `OnceCell<Arc<T>>` to build read-only policy/Unicode tables at first use; no mutation after set.
* `Arc<StateSnapshot>` copies shared read-only views between threads.

**Forbidden:**

* Holding any lock across blocking I/O or hypothetical `.await`.
* Nested locks; lock hierarchies.
* Global mutable registries.

**Current state:** No locks used.

---

## 5) Timeouts, Retries, Deadlines

* **Library:** Not applicable (pure/sync, in-memory transformations).
* **CLI:** Uses standard blocking file/stdin/stdout; no network/RPC. Timeouts are not needed; if they are ever introduced (e.g., for very large inputs), they must be **opt-in** and enforced with a **total deadline** per command.

Backoff/retry policies are **N/A** (no external I/O other than local files/stdio).

---

## 6) Cancellation & Shutdown — **N/A**

* No long-running tasks. Process exit is immediate. If we ever write to files, we use **temp file + atomic rename** to ensure partial writes are not observed.

---

## 7) I/O & Framing — **N/A (no sockets)**

* CLI reads/writes **files/stdin/stdout** only. Encodings are **CBOR/JSON** (canonicalized deterministically in the library). No streaming network framing.

---

## 8) Error Taxonomy (Concurrency-Relevant)

Minimal, tailored to this crate:

| Error                 | When                                  | Retry? | Notes                              |
| --------------------- | ------------------------------------- | ------ | ---------------------------------- |
| `Error::Busy`         | (future) if a bounded queue existed   | N/A    | Not used presently                 |
| `Error::Timeout`      | Only if an optional deadline is added | N/A    | Not used presently                 |
| `Error::Canceled`     | CLI interrupted by user (signal)      | N/A    | Leaves no partial in-memory state  |
| `Error::Poisoned`     | (future) if locks are added           | N/A    | Avoid by not using poisoning locks |
| `Error::SizeExceeded` | Input exceeds documented cap          | N/A    | Validation guard, not concurrency  |

---

## 9) Metrics (Concurrency Health) — **Trimmed**

* **Not emitted here.** If the workspace has global metrics, the CLI may log structured events only (no Prometheus endpoints).

---

## 10) Validation Strategy

**Unit / Property (library)**

* **Determinism under threads:** Spawn N threads, concurrently call `normalize_name` on the same corpus; assert outputs are identical and stable (no allocation panics, no races).
* **Idempotence:** `normalize(normalize(x)) == normalize(x)` across diverse Unicode corpora (ASCII/Latin/Cyrillic/CJK/Emoji/mixed scripts).
* **Immutability checks:** If `OnceCell` is used for tables, verify `set()` is called at most once (negative tests ensure double-set fails predictably).

**Loom (dev-only; minimal)**

* Model a hypothetical **single-writer initialization** of a `OnceCell` while multiple readers call `get_or_init` concurrently; assert no deadlock/livelock and visibility after set.
* Model a **no-mutation after publish** invariant using a small state struct with a read handle cloned before/after initialization.

**Fuzz**

* Feed arbitrary Unicode into normalization; assert **no panics**, errors are typed, and resulting normalized strings meet policy (confusables/mixed scripts as configured).

**Golden vectors**

* Canonical encodings (CBOR/JSON) of `TldMap`/`SignedTldMap` are **byte-stable** across runs/platforms.

---

## 11) Code Patterns (Copy-Paste)

**Read-only table with single initialization**

```rust
use once_cell::sync::OnceCell;
use std::sync::Arc;

static POLICY_TABLES: OnceCell<Arc<PolicyTables>> = OnceCell::new();

pub fn policy_tables() -> &'static Arc<PolicyTables> {
    POLICY_TABLES.get_or_init(|| {
        // Build from embedded/bundled data; no external I/O.
        Arc::new(PolicyTables::from_builtin())
    })
}
```

**No global mutability; pass snapshots explicitly**

```rust
pub fn normalize_with(input: &str, tables: &PolicyTables) -> Result<CanonicalName, NameError> {
    // Pure function: depends only on input + provided tables.
    normalize_impl(input, tables)
}

pub fn normalize(input: &str) -> Result<CanonicalName, NameError> {
    normalize_with(input, policy_tables())
}
```

**Atomic output (future-proofing if writing files)**

```rust
use std::{fs, io, path::Path};

pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, bytes)?;
    // On Unix, this is atomic if same filesystem.
    fs::rename(tmp, path)?;
    Ok(())
}
```

**Thread-safe reentrancy test scaffold**

```rust
#[test]
fn normalize_is_thread_safe() {
    use std::sync::Arc;
    use std::thread;

    let inputs = Arc::new(vec!["ExAmPle", "café", "РУС", "😀", "xn--caf-dma"]);
    let mut handles = Vec::new();

    for _ in 0..8 {
        let inputs = inputs.clone();
        handles.push(thread::spawn(move || {
            for s in inputs.iter() {
                let a = crate::normalize(s).unwrap();
                let b = crate::normalize(&a.0).unwrap();
                assert_eq!(a, b, "idempotence failed on {s}");
            }
        }));
    }
    for h in handles { h.join().unwrap(); }
}
```

---

## 12) Configuration Hooks (Quick Reference)

* None affect concurrency: config toggles normalization strictness, policy bundles, signing behavior (via external KMS profile), PQ mode flags for signature algorithms.
* If a future cache size were configurable, it must have a **hard cap** and **documented memory bound**.

See `docs/CONFIG.md` for the full schema.

---

## 13) Known Trade-offs / Nonstrict Areas

* **OnceCell vs eager build:** We choose `OnceCell` to avoid startup cost; the first call pays the build. This adds minimal synchronization but no ongoing contention.
* **No timeouts in CLI:** Simplicity beats complexity; very large inputs should be handled by upstream tooling (or we add an **opt-in** deadline later, with tests).

---

## 14) Mermaid Diagrams (Lightweight)

### 14.1 Initialization & Use (no tasks/channels)

```mermaid
flowchart LR
  A[First normalize()] --> B{POLICY_TABLES set?}
  B -- no --> C[Build from builtin]
  C --> D[Store OnceCell<Arc<_>>]
  B -- yes --> D
  D --> E[Normalize input (pure)]
```

**Text description:** The first call constructs policy tables from built-ins and stores them in a `OnceCell<Arc<_>>`. Subsequent calls reuse the read-only snapshot.

---

## 15) CI & Lints (Enforcement)

* **Clippy (crate-level):**

  * `-D clippy::unwrap_used`
  * `-D clippy::expect_used`
  * `-D clippy::await_holding_lock` *(kept on even though we’re sync; guards future async drift)*
  * `-D warnings`

* **Loom job (optional, PR-only):**

  * Build tests with `--cfg loom` and run the minimal `OnceCell` model.

* **Fuzz job:**

  * `cargo fuzz` target for normalization inputs; assert no panics and policy compliance.

* **Golden vectors:**

  * Check CBOR/JSON stability for `TldMap`/`SignedTldMap` across platforms.

---

### Definition of Done (Concurrency)

* No background tasks, channels, or locks are introduced.
* Any initialization is one-time and read-only via `OnceCell<Arc<_>>`.
* Determinism and idempotence validated under parallel callers.
* CI enforces lints; (optional) Loom model guards future drift.


---

## CONFIG.MD
_File 3 of 13_


---

````markdown
---
title: Configuration — ron-naming
crate: ron-naming
owner: Stevan White
last-reviewed: 2025-10-06
status: draft
template_version: 1.1
audience: contributors, ops, auditors
concerns: [SEC, GOV]
---

# Configuration — ron-naming

This document defines **all configuration** for `ron-naming`, including sources,
precedence, schema (types/defaults), validation, feature flags, (non-)reload
behavior, and security implications. It complements `README.md`, `IDB.md`,
and `SECURITY.md`.

> **Tiering**
>
> - **Library (default role):** Pure functions accept parameters; they **do not** read global config.  
> - **CLI (`tldctl`, feature `cli`):** Reads config from flags/env/file to drive **offline** authoring, linting, packing, signing, verifying of naming artifacts.  
> - **Service settings (ports, /healthz):** **N/A** for this crate.

---

## 1) Sources & Precedence (Authoritative)

Configuration can be supplied by the user when invoking the **CLI**. The **library
does not load config**; callers pass values directly via function parameters.

**Precedence (highest wins) for CLI:**

1. **Process flags** (CLI args to `tldctl`)  
2. **Environment variables**  
3. **Config file** (TOML/JSON via `--config`)  
4. **Built-in defaults** (documented below)

**Env prefixes:**

- Common: `RON_NAMING_…`
- CLI-only (takes precedence over common): `TLDCTL_…`  
  Example: `TLDCTL_FORMAT=cbor` overrides `RON_NAMING_FORMAT=json`.

**File formats:** TOML (preferred), JSON (optional).

**Path resolution for `--config` (if relative):** `./` then `$CWD`.

**Reload:** Not applicable (one-shot CLI). Library has no reload concept.

---

## 2) Quickstart Examples

### 2.1 Lint a TLD set (stdin → stdout, JSON)
```bash
echo '{ "version":1, "entries":[{"tld":"example","owner_key_id":"ops@k1","rules_version":1}] }' \
| TLDCTL_FORMAT=json cargo run -p ron-naming --features cli --bin tldctl -- lint -
````

### 2.2 Pack deterministically to CBOR

```bash
TLDCTL_FORMAT=cbor cargo run -p ron-naming --features cli --bin tldctl -- \
  pack ./tlds.json --out ./tldmap.cbor
```

### 2.3 Sign with an external KMS profile (detached multisig envelope)

```bash
TLDCTL_SIGN=true \
TLDCTL_SIGN_PROFILE=profile://ops-m-of-n \
TLDCTL_SIGN_THRESHOLD_M=2 \
TLDCTL_SIGN_N=3 \
cargo run -p ron-naming --features "cli,verify" --bin tldctl -- \
  sign ./tldmap.cbor --out ./signed-tldmap.cbor
```

### 2.4 Verify a signed map with PQ backend enabled

```bash
TLDCTL_PQ_MODE=hybrid \
cargo run -p ron-naming --features "cli,verify,pq" --bin tldctl -- \
  verify ./signed-tldmap.cbor
```

### 2.5 Use a config file + env overrides

```bash
cargo run -p ron-naming --features cli --bin tldctl -- \
  --config ./Config.toml pack - --out ./tldmap.cbor
```

---

## 2.a Full sample `Config.toml` (ready to commit)

```toml
# CLI behavior
format = "cbor"        # json|cbor
pretty = false         # pretty JSON (no effect for CBOR)
in_path = "-"          # "-" means stdin
out_path = "-"         # "-" means stdout

# Normalization policy
[normalize]
strict = true
allow_mixed_scripts = false
allowed_scripts = ["Latin"]   # optional whitelist
policy_bundle = "builtin"     # builtin|minimal|extended|custom
custom_tables_dir = ""        # used when bundle="custom"

# Signing / verification
[sign]
enabled = false
profile = "profile://ops-m-of-n"
threshold_m = 2
n = 3
key_ids = ["ops-k1","ops-k2","ops-k3"]

[pq]
mode = "off"                  # off|hybrid

[log]
format = "json"               # json|text
level  = "info"               # trace|debug|info|warn|error
```

### 2.b Env export snippet (ops convenience)

```bash
export TLDCTL_FORMAT=cbor
export TLDCTL_NORM_STRICT=true
export TLDCTL_SIGN=false
export TLDCTL_LOG_FORMAT=json
export TLDCTL_LOG_LEVEL=info
```

---

## 3) Schema (Typed, With Defaults)

> **Durations/Sizes:** Not applicable here (no network/server).
> **OAP tie-in:** Normalized names and governance artifacts are **small** by design; CLI/library must keep serialized outputs **well below** OAP/1 `max_frame = 1 MiB`.
> **Env names:** Prefer `TLDCTL_…` for CLI; `RON_NAMING_…` also accepted as a lower-precedence alias.

| Key / Env Var                                     | Type                                          | Default      | Applies to | Description                                                                             | Security Notes                                |
| ------------------------------------------------- | --------------------------------------------- | ------------ | ---------- | --------------------------------------------------------------------------------------- | --------------------------------------------- |
| `format` / `TLDCTL_FORMAT`                        | enum(`json`,`cbor`)                           | `cbor`       | CLI        | Output encoding for `pack`/`sign`/`show`. Input auto-detected or forced via `--format`. | JSON may leak whitespace; CBOR is compact.    |
| `pretty` / `TLDCTL_PRETTY`                        | bool                                          | `false`      | CLI        | Pretty-print JSON output (no effect for CBOR).                                          | Disable in scripts to keep outputs canonical. |
| `in_path` / `TLDCTL_IN`                           | path (`-` = stdin)                            | `-`          | CLI        | Default input path for commands accepting a single input.                               | Validate path; avoid shell glob surprises.    |
| `out_path` / `TLDCTL_OUT`                         | path (`-` = stdout)                           | `-`          | CLI        | Default output path; `-` keeps output on stdout.                                        | Overwrites only with explicit `--out`.        |
| `normalize.strict` / `TLDCTL_NORM_STRICT`         | bool                                          | `true`       | CLI/Lib    | Reject confusables/mixed-script by policy (default-deny).                               | Reduces spoof risks.                          |
| `normalize.allow_mixed_scripts` / `…_ALLOW_MIX`   | bool                                          | `false`      | CLI/Lib    | Permit mixed scripts when `strict=false`.                                               | Increases spoof risks; use sparingly.         |
| `normalize.allowed_scripts` / `…_ALLOWED_SCRIPTS` | list<string>                                  | `[]`         | CLI/Lib    | Optional allow-list (e.g., `["Latin"]`).                                                | Narrow scope to reduce risk.                  |
| `normalize.policy_bundle` / `…_POLICY_BUNDLE`     | enum(`builtin`,`minimal`,`extended`,`custom`) | `builtin`    | CLI/Lib    | Which built-in table set to use.                                                        | Custom bundles must be reviewed.              |
| `normalize.custom_tables_dir` / `…_TABLES_DIR`    | path                                          | `""`         | CLI/Lib    | Directory containing custom policy tables (only if `policy_bundle="custom"`).           | Treat as untrusted input.                     |
| `unicode.version_pin` / `…_UNICODE_VERSION`       | string                                        | pinned build | CLI/Lib    | Build-time Unicode/IDNA version pin (read-only at runtime; surfaced for diagnostics).   | Pin updates require vectors + review.         |
| `sign.enabled` / `TLDCTL_SIGN`                    | bool                                          | `false`      | CLI        | Enable signing flow (`sign` command).                                                   | Keys never stored by CLI.                     |
| `sign.profile` / `TLDCTL_SIGN_PROFILE`            | string URI                                    | `""`         | CLI        | KMS/HSM profile, e.g., `profile://ops-m-of-n`.                                          | No secrets in URI; use profile indirection.   |
| `sign.threshold_m` / `TLDCTL_SIGN_THRESHOLD_M`    | u8                                            | `2`          | CLI        | M in M-of-N multisig.                                                                   | Validate `1 ≤ M ≤ N`.                         |
| `sign.n` / `TLDCTL_SIGN_N`                        | u8                                            | `3`          | CLI        | N in M-of-N multisig.                                                                   |                                               |
| `sign.key_ids` / `TLDCTL_SIGN_KEY_IDS`            | list<string>                                  | `[]`         | CLI        | Logical key IDs included in the envelope metadata.                                      | Do not log.                                   |
| `verify.enabled` / `TLDCTL_VERIFY`                | bool                                          | `true`       | CLI        | Enable verification checks in `verify` command.                                         |                                               |
| `pq.mode` / `TLDCTL_PQ_MODE`                      | enum(`off`,`hybrid`)                          | `off`        | CLI/Lib    | Select verifier/signing hybrid mode (e.g., Ed25519 + Dilithium).                        | Interop gating required.                      |
| `log.format` / `TLDCTL_LOG_FORMAT`                | enum(`json`,`text`)                           | `json`       | CLI        | CLI log output format.                                                                  | JSON preferred for CI.                        |
| `log.level` / `TLDCTL_LOG_LEVEL`                  | enum                                          | `info`       | CLI        | `trace`..`error`.                                                                       | Avoid `trace` in CI; may leak paths.          |

> The library surfaces policy/Unicode versions via functions like `version_info()`.
> Consumers should treat them as **diagnostic values**, not mutable config.

---

## 4) Validation Rules (Fail-Closed)

Applied by the CLI at startup (and by library constructors where relevant):

* `format ∈ {json,cbor}`; `pretty` valid only when `format=json`.
* If `normalize.policy_bundle="custom"` then `normalize.custom_tables_dir` **must exist** and contain required table files.
* If `normalize.strict=true`, deny mixed scripts regardless of `allow_mixed_scripts`.
* If `sign.enabled=true` then:

  * `sign.profile ≠ ""`,
  * `1 ≤ threshold_m ≤ n`,
  * if `key_ids` provided, **length ≥ n** and unique.
* `pq.mode="hybrid"` requires the selected `Verifier/Signer` backend to advertise support; otherwise **fail** with a clear message.
* `in_path` and `out_path` are validated; `out_path="-"` is permitted; writing to an existing file requires explicit `--out` (no silent overwrite).
* Unknown keys in config files are **rejected** (serde `deny_unknown_fields`).

**On violation**: CLI prints a structured error to stderr and exits with code `64` (usage) or `2/3` for validation/signature failures. Library APIs return typed errors; **no panics** on user input.

---

## 5) Dynamic Reload

**Not supported.** `tldctl` is a one-shot CLI; library is stateless/pure.

---

## 6) CLI Flags (Canonical)

> Flags override env and file settings.

```
# Global
--config <path>                 # Load TOML/JSON config (lowest precedence)
--format <json|cbor>
--pretty                        # Pretty JSON (no effect for CBOR)
--in <path|->                   # Default stdin/out shorthands
--out <path|->

# Normalization
--strict                        # Enforce strict policy (default true)
--no-strict                     # Disable strict policy
--allow-mixed-scripts           # Only if not strict
--allowed-scripts <CSV>
--policy-bundle <builtin|minimal|extended|custom>
--custom-tables-dir <path>

# Signing / Verification
--sign                          # Enable signing flow
--sign-profile <uri>            # e.g., profile://ops-m-of-n
--m <num>                       # threshold M
--n <num>                       # total N
--key-id <id> ...               # repeatable
--verify                        # Enable verify flow (default true)
--pq-mode <off|hybrid>

# Logging
--log-format <json|text>
--log-level <trace|debug|info|warn|error>
```

> Subcommands: `lint <in>`, `pack <in> [--out <out>]`, `sign <in> [--out <out>]`,
> `verify <in>`, `show <in>`.

---

## 7) Feature Flags (Cargo)

| Feature        | Default | Effect                                                                          |
| -------------- | ------: | ------------------------------------------------------------------------------- |
| `cli`          |     off | Builds the `tldctl` binary and pulls arg-parsing/logging deps for the CLI only. |
| `verify`       |     off | Enables signature verification helpers and KMS integration points.              |
| `pq`           |     off | Wires PQ verifier/signers (e.g., Dilithium) via `ron-kms` backend.              |
| `large-tables` |     off | Bundles extended Unicode/confusables tables (larger binary).                    |

> Enabling `cli` **must not** raise MSRV or violate workspace dependency pins.

---

## 8) Security Implications

* **Keys & secrets**: The CLI never stores keys; it references an external **profile** (KMS/HSM). No secret material should appear in logs or on disk via this tool.
* **Normalization strictness**: Relaxing policy (allowing mixed scripts) increases spoof risks; defaults are deny-by-policy.
* **Custom policy bundles**: Treat directories as untrusted input; pin and review changes; include provenance in PRs.
* **Canonical outputs**: Prefer CBOR in pipelines; JSON pretty is for humans.
* **PQ hybrid**: Interop must be planned (publish mode in release notes) to avoid network partitions later when services enforce maps.
* **Amnesia-compatibility**: Lib is stateless; CLI is one-shot with explicit I/O only. In “amnesia mode,” there is **no implicit state** to purge beyond user-provided files.

---

## 9) Compatibility & Migration

* **Additive keys** are introduced with safe defaults (`strict=true`, `format=cbor`).
* **Renames** ship with env aliases for ≥1 minor; CLI emits a deprecation warning.
* **Breaking changes** (e.g., policy semantics) require a major version bump and documented migration steps (including regeneration of golden vectors).

**Deprecation table**

| Old Key              | New Key                       | Removal Target | Notes               |
| -------------------- | ----------------------------- | -------------: | ------------------- |
| `policy_bundle_path` | `normalize.custom_tables_dir` |         v0.2.0 | Name clarified      |
| `sign.threshold`     | `sign.threshold_m`            |         v0.2.0 | Disambiguate from N |

---

## 10) Reference Implementation (Rust)

> Minimal structures used by the **CLI**. The **library** takes params directly and does not read env/file.

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct NormalizeCfg {
    #[serde(default = "default_true")]
    pub strict: bool,
    #[serde(default)]
    pub allow_mixed_scripts: bool,
    #[serde(default)]
    pub allowed_scripts: Vec<String>,
    #[serde(default = "default_policy_bundle")]
    pub policy_bundle: PolicyBundle,
    #[serde(default)]
    pub custom_tables_dir: Option<PathBuf>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PolicyBundle { Builtin, Minimal, Extended, Custom }
fn default_policy_bundle() -> PolicyBundle { PolicyBundle::Builtin }
fn default_true() -> bool { true }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignCfg {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub profile: Option<String>,
    #[serde(default = "default_m")]
    pub threshold_m: u8,
    #[serde(default = "default_n")]
    pub n: u8,
    #[serde(default)]
    pub key_ids: Vec<String>,
}
fn default_m() -> u8 { 2 }
fn default_n() -> u8 { 3 }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PqCfg {
    #[serde(default = "default_pq_mode")]
    pub mode: PqMode,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PqMode { Off, Hybrid }
fn default_pq_mode() -> PqMode { PqMode::Off }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LogCfg {
    #[serde(default = "default_log_fmt")]
    pub format: LogFormat,
    #[serde(default = "default_log_level")]
    pub level: LogLevel,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat { Json, Text }
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel { Trace, Debug, Info, Warn, Error }
fn default_log_fmt() -> LogFormat { LogFormat::Json }
fn default_log_level() -> LogLevel { LogLevel::Info }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CliCfg {
    #[serde(default = "default_format")]
    pub format: OutputFormat,      // json|cbor
    #[serde(default)]
    pub pretty: bool,              // json only
    #[serde(default = "default_in")]
    pub in_path: String,           // "-" = stdin
    #[serde(default = "default_out")]
    pub out_path: String,          // "-" = stdout
    #[serde(default)]
    pub normalize: NormalizeCfg,
    #[serde(default)]
    pub sign: SignCfg,
    #[serde(default)]
    pub pq: PqCfg,
    #[serde(default)]
    pub log: LogCfg,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat { Json, Cbor }
fn default_format() -> OutputFormat { OutputFormat::Cbor }
fn default_in() -> String { "-".to_string() }
fn default_out() -> String { "-".to_string() }

impl CliCfg {
    pub fn validate(&self) -> anyhow::Result<()> {
        if matches!(self.format, OutputFormat::Json) == false && self.pretty {
            // pretty ignored for CBOR; not an error
        }
        let norm = &self.normalize;
        if matches!(norm.policy_bundle, PolicyBundle::Custom) && norm.custom_tables_dir.is_none() {
            anyhow::bail!("custom policy bundle requires custom_tables_dir");
        }
        let s = &self.sign;
        if s.enabled {
            if s.profile.as_deref().unwrap_or("").is_empty() {
                anyhow::bail!("sign.enabled=true but sign.profile missing");
            }
            if !(1..=s.n).contains(&s.threshold_m) {
                anyhow::bail!("sign.threshold_m must satisfy 1 ≤ M ≤ N");
            }
        }
        Ok(())
    }
}
```

---

## 11) Test Matrix

| Scenario                                    | Expected Outcome                         |
| ------------------------------------------- | ---------------------------------------- |
| Unknown key in `Config.toml`                | Fail with “unknown field”                |
| `policy_bundle=custom` without tables dir   | Fail with clear message                  |
| `sign.enabled=true` but no `sign.profile`   | Fail with clear message                  |
| `threshold_m > n`                           | Fail with clear message                  |
| `--format json --pretty`                    | Pretty JSON output                       |
| `verify` on tampered signature              | Exit code `3` (signature failed)         |
| Mixed-script input when `strict=true`       | `lint` fails; exit code `2`              |
| PQ hybrid with backend lacking PQ           | Fail with interop capability error       |
| `out_path` is existing file without `--out` | No overwrite unless explicitly specified |

---

## 12) Mermaid — Config Resolution Flow (CLI)

```mermaid
flowchart TB
  F[Flags] --> M[Merge]
  E[Env (TLDCTL_ > RON_NAMING_)] --> M
  C[Config file (TOML/JSON)] --> M
  D[Defaults] --> M
  M --> V{Validate}
  V -- ok --> RUN[Execute subcommand]
  V -- fail --> ERR[Exit non-zero]
  style RUN fill:#0ea5e9,stroke:#075985,color:#fff
```

---

## 13) Operational Notes

* Keep policy bundles and signed artifacts in **version control**; treat them as code (review, provenance).
* Prefer **CBOR** for machine pipelines and **JSON** only for human inspection or diffs.
* For PQ rollout, publish the planned `pq.mode` in release notes so consuming services can align.
* The CLI is **offline by design**; any request to add network behavior should be rejected or moved to a service crate (e.g., `svc-registry`, `svc-index`).
* **Amnesia:** Since the lib is stateless and the CLI is one-shot with explicit I/O only, amnesia mode is trivially satisfied (no implicit state).

```
```


---

## GOVERNANCE.MD
_File 4 of 13_



---

# 🏛 GOVERNANCE.md — ron-naming

---

title: Governance & Integrity
status: draft
msrv: 1.80.0
last-updated: 2025-10-06
audience: contributors, ops, auditors, SDK authors
crate-type: policy (naming schemas & normalization)
---------------------------------------------------

## 0. Purpose

This document defines the **rules of engagement** for `ron-naming`:

* Transparent, auditable decision-making for **naming schemas**, **normalization**, and **canonical encodings**.
* Enforcement of **naming invariants** (determinism, idempotence, bounded label/length, stable DTOs).
* Clear **authority boundaries** with adjacent crates/services (e.g., `svc-index` for runtime resolution).
* **Compatibility SLAs** for downstream consumers (services, SDKs, `tldctl` users).

It ties into:

* **Hardening Blueprint** (bounded authority; signed releases; supply-chain hygiene).
* **Interop Blueprint** (DTO stability & test vectors).
* **Perfection Gates A–O** with emphasis on **Gate I** (invariants can’t be weakened silently) and **Gate M** (appeal paths & dispute handling).

---

## 1. Invariants (MUST)

Non-negotiable **naming** rules enforced by code and CI:

* **[I-N1] Determinism:** Same input ⇒ same canonical output (platform/arch independent).
* **[I-N2] Idempotence:** `normalize(normalize(x)) == normalize(x)`.
* **[I-N3] Bounds:** Post-canonicalization: label length ≤ **63 octets**; FQDN length ≤ **253 octets**; reject otherwise with typed errors.
* **[I-N4] Encoding Canon:** Canonical CBOR/JSON encodings are **stable** and **round-trip lossless**.
* **[I-N5] No side effects:** Library and `tldctl` perform **no network/DB I/O**; purely computational.
* **[I-N6] Auditability:** All schema/normalization changes are RFC’d, reviewed, signed, and shipped with **golden test vectors** and perf baselines.
* **[I-N7] Backward compatibility:** Public DTOs and normalization semantics are **SemVer-governed**; breaking changes require a **major** and migration notes.
* **[I-N8] Anti-scope:** Runtime **resolution** is not implemented here (belongs to `svc-index`).

---

## 2. Roles & Authority

### Roles

* **Maintainers (N-Core):** Own `ron-naming` design/implementation; gate RFCs and releases.
* **Policy Stewards (P-Stewards):** Curate higher-level naming **policy** drafts (live in `ron-policy`); propose inputs to N-Core.
* **Interop Liaisons:** Ensure DTO/test-vector alignment across SDKs/services.
* **Auditors:** External reviewers verifying invariants and release provenance.

### Authority Boundaries

* **N-Core** can **define/alter normalization** and **DTOs**, but **cannot** add runtime I/O or resolution.
* **P-Stewards** can **propose** rules (e.g., confusables strategy, future variants) but cannot merge code without N-Core sign-off.
* **Services (e.g., `svc-index`)** may **reject** inputs that violate current invariants; they **cannot** extend/override normalization rules upstream.
* All impactful actions require **capability-based Git permissions** (review approvals) and **signed tags** on release.

---

## 3. Rules & SLAs

### Compatibility & Stability

* **Public API/DTO stability:**

  * Minor/patch releases: **no breaking changes**; only additive or bug-fix semantics.
  * Major release: may change normalization semantics or DTO shape **only with** migration notes + upgraded golden vectors.

* **Test Vectors SLA:**

  * A canonical vector corpus (`/testing/corpora` + `/testing/vectors/*.json`) is shipped each release.
  * Any change to vectors must include rationale, RFC link, and SemVer label (breaking/additive/fix).

* **`tldctl` Coupling:**

  * `tldctl` is **version-locked** to the library. New flags must not change defaults silently.
  * Deprecations: mark in **N+1**, remove no earlier than **N+3** minor releases.

* **Release Cadence & Notification:**

  * Patch ≤ 2 weeks for critical correctness bugs; Minor on demand; Major with **30-day** heads-up (RFC accepted, migration notes published).

### Observability & Provenance

* **Signed releases:** All tags are **cryptographically signed**; SBOM and checksums published.
* **Provenance trail:** RFC ID → PRs → CI artifacts → signed tag → CHANGELOG with vectors & perf links.

---

## 4. Governance Process

### Proposal Lifecycle (RFC)

1. **Draft** (problem statement, invariants impacted, test-vector diffs, perf impact).
2. **Public Review** (≥ 5 business days; SDKs/services may file compatibility concerns).
3. **Decision** (N-Core quorum ≥ 2 maintainers; at least 1 Interop Liaison sign-off if DTOs change).
4. **Implementation** (feature branch with vectors, tests, perf baselines).
5. **Staging** (release-candidate tag; downstream try-builds).
6. **Release** (signed tag + CHANGELOG + migration notes).

**Default posture:** If quorum not reached in **T = 10 business days**, **reject** (can resubmit).

### Emergency Fixes

* **Correction Patch (CP-N):** For correctness/security regressions only; skip full RFC but require dual-maintainer approval and postmortem within **72h**.
* **No semantic expansions** via CP-N.

### Parameterization

* **Confusables / Unicode tables / normalization knobs** are code-constants here; any change requires RFC (no runtime/config toggles).

---

## 5. Audit & Observability

* **Audit artifacts:**

  * RFC docs (in repo or governance site).
  * Signed release tags + checksums.
  * Golden vectors (before/after), Criterion baselines, hyperfine CSV.
  * `cargo-public-api` diff for public items.

* **Metrics (repo/process level, optional):**

  * `governance_rfc_total{status}` – Draft/Accepted/Rejected.
  * `golden_vector_changes_total{kind}` – additive/breaking/fix.
  * `semver_releases_total{level}` – major/minor/patch.

* **Red-team drills:**

  * Attempt to slip a breaking semantic under a “patch” label → CI gate must fail (public-API diff and vector delta checks).
  * Attempt to introduce I/O (network/fs) → anti-scope lint must fail.

---

## 6. Config & Custody

* **Runtime config:** N/A (no network/DB). `tldctl` supports flags only; no secrets.

* **Key custody (release signing):**

  * Maintainer keys in **KMS/HSM** (preferred) or hardware tokens.
  * Rotation every **90 days** or post-incident.
  * Public keys published; verify in CI and by downstream packaging.

* **Supply-chain hygiene:**

  * `cargo-deny` (advisories/licenses/sources) must be green.
  * Reproducible builds where feasible; SBOM (CycloneDX) attached to releases.

---

## 7. Appeal Path

* **Who may appeal:** SDK authors, service owners, external integrators.
* **How:**

  1. Open an **Appeal Issue** referencing the RFC/commit; attach failing cases (inputs, expected vs actual).
  2. N-Core triage within **3 business days**.
  3. If blocked in production, request **Compatibility Waiver** (temporary) with scoped mitigation.
  4. Resolution via: revert, targeted patch, or scheduled major with migration aids.
* **Transparency:** Appeals and outcomes are linked in the CHANGELOG for that release.

---

## 8. Acceptance Checklist (DoD)

* [ ] Invariants **[I-N1]…[I-N8]** enforced by tests and CI gates.
* [ ] RFC lifecycle documented for any semantic change; vectors updated.
* [ ] Public API diffs checked (`cargo-public-api`) and labeled with correct SemVer.
* [ ] Release is **signed**; SBOM + checksums published.
* [ ] Perf baselines & golden vectors attached; CI perf deltas within thresholds.
* [ ] Appeal path documented; at least one red-team drill run per quarter.

---

## 9. Appendix

* **Blueprints:** Interop (DTO stability), Hardening (bounded authority, signing), Scaling/Perf (bench baselines & gates).
* **References:** Unicode normalization (NFKC + casefold), IDN length rules, canonical CBOR spec notes for determinism.
* **History:** Maintain a table of semantic changes (version, RFC, vectors changed, migration notes).

---

### Notes specific to `tldctl`

* Treated as an **offline companion** to `ron-naming` with **no runtime authority**.
* New flags default to **non-breaking** behavior; breaking defaults allowed **only** in a major release with migration guides and dual-path compatibility (`--legacy` window).
* CLI help and `--version --vectors-hash` must reflect library version and the hash of golden vectors for easy provenance checks.

---

With this governance in place, `ron-naming` remains **stable, auditable, and predictable** for all downstream consumers while preserving the strict lib boundary (no I/O, no runtime resolution) and ensuring any semantic evolution is **measured, reviewable, and revertible**.


---

## IDB.md
_File 5 of 13_


---

````markdown
---
title: ron-naming — Invariant-Driven Blueprint (IDB)
version: 0.1.1
status: draft
last-updated: 2025-10-06
audience: contributors, ops, auditors
msrv: 1.80.0
concerns: [SEC, GOV]
pillar: 9   # Content Addressing & Naming
---

# ron-naming — IDB

> **Scope:** Library for naming **schemas, normalization, validation, and signed governance artifacts**.  
> Includes an **optional, thin CLI** (`tldctl`) that wraps the library for local authoring/linting/packing/signing.  
> **No runtime lookups** (that’s `svc-index`). **No network/DB/DHT** here.

---

## 1) Invariants (MUST)

- **[I-1] Boundary & roles.** `ron-naming` is a **pure library** + **optional CLI**.  
  - Library: types, normalization, validation, deterministic (de)serialization, canonical digesting.  
  - CLI (`tldctl`): **feature-gated** (`cli`) and **only** calls library APIs to lint/pack/sign/verify artifacts with **explicit I/O** (stdin/stdout/paths).  
  - **Never**: network calls, DB/DHT access, resolver logic, implicit persistence.

- **[I-2] Deterministic normalization pipeline.** `normalize_name(input) -> CanonicalName` applies, in order:  
  1) Unicode **NFC**,  
  2) **IDNA/UTS-46** non-transitional processing (with explicit error channels),  
  3) **lowercase** case-fold,  
  4) **disallowed codepoints** rejected per policy tables,  
  5) **confusables/mixed-script** policy enforced (default-deny),  
  6) **idempotence** holds: `normalize(normalize(x)) == normalize(x)`.

- **[I-3] Canonical wire forms.** Canonical names are UTF-8; ASCII channels use **Punycode** (`xn--`). DTOs use `serde` with `#[serde(deny_unknown_fields)]`. Encodings (CBOR/JSON) are **stable** and **size-bounded** (document limits).

- **[I-4] Addressing invariant.** Any content address references use **BLAKE3** `"b3:<64-hex>"`. This crate validates format only (no hashing of payloads).

- **[I-5] TLD governance artifacts (tldctl folded here).**  
  - **`TldMap`** (authoritative set; versioned; order-invariant digest).  
  - **`SignedTldMap`** (detached multi-sig envelope).  
  - Canonical digest computed over canonical encoding of the **body**.  
  - No “econ/registry policy” decisions here; those live in policy/registry services.

- **[I-6] Verifier abstraction.** Signature verification/signing is defined via small traits; implementations may be backed by `ron-kms`/HSM. The **library stores no keys** and never prompts for secrets.

- **[I-7] Hardening hygiene.**  
  - No panics on user input; error types are explicit and non-allocating where viable.  
  - DTO size limits enforced; Unicode/IDNA **table versions are pinned** and must be updated intentionally with vectors.  
  - `#![forbid(unsafe_code)]`; clippy denies `unwrap_used`, `expect_used`.

- **[I-8] Public API stability.** Public DTOs/APIs are **semver-disciplined**; breaking changes require a major bump and an approved API diff.

- **[I-9] Placement & split.** Pillar 9 (Naming): **schemas/types/validation** are here; **runtime resolution/serving** is in `svc-index`. Keep the split crisp.

- **[I-10] Amnesia compatibility.** The library is stateless; the CLI reads/writes only what the operator specifies. There is **no retained state** to erase beyond user files.

- **[I-11] Workspace conformance.** MSRV **1.80.0**; dependencies respect workspace pins; enabling `cli` must not raise MSRV.

---

## 2) Design Principles (SHOULD)

- **[P-1] Schemas first.** Prefer declarative DTOs + validators over imperative flows; keep policy tables as **data** (regenerated, not hard-coded).
- **[P-2] One normalization path.** Expose a **single** normalization entrypoint used by everything (lib, tests, CLI).
- **[P-3] PQ-ready signatures.** `Verifier`/`Signer` traits **must** allow PQ backends (e.g., Dilithium) via `ron-kms` features; Ed25519 is acceptable for bootstrap but not a long-term assumption.
- **[P-4] Boring CLI.** The CLI mirrors library semantics: explicit files/stdio, deterministic outputs, machine-friendly exit codes. Think “lint/pack/sign/verify/show”—nothing else.
- **[P-5] Minimal feature surface.** Features:  
  - `cli` (pulls arg-parsing + bin target),  
  - `verify` (enables signature verification helpers),  
  - `pq` (wires PQ backends),  
  - `large-tables` (ships extended Unicode policy tables).

---

## 3) Implementation (HOW)

> The following are copy-paste-ready idioms; real code may live across modules.

### [C-1] Core DTOs

```rust
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalName(String);

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Label(String); // single, normalized label

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag="kind", deny_unknown_fields)]
pub enum NameRef {
    Human { name: CanonicalName },
    Address { b3: String }, // must match "b3:<64 hex>", lower-case
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TldEntry {
    pub tld: Label,
    pub owner_key_id: String, // logical key id; actual key custody is external
    pub rules_version: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TldMap {
    pub version: u64,        // monotonic; required to increase on changes
    pub entries: Vec<TldEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedTldMap<S> {
    pub body: TldMap,        // canonical-encoded for digest
    pub signatures: Vec<S>,  // detached signatures
}
````

### [C-2] Normalization API (single entrypoint)

```rust
pub fn normalize_name(input: &str) -> Result<CanonicalName, NameError> {
    let nfc = input.nfc().collect::<String>();
    let (uts46, uts46_errors) = idna::Config::default()
        .use_std3_ascii_rules(true)
        .non_transitional_processing(true)
        .to_unicode(&nfc);
    if !uts46_errors.is_empty() { return Err(NameError::Idna(uts46_errors)); }
    let lower = uts46.to_lowercase();
    policy::reject_disallowed(&lower)?;
    policy::reject_confusables(&lower)?;
    if lower != normalize_once(&lower)? { return Err(NameError::NonIdempotent); }
    Ok(CanonicalName(lower))
}
```

### [C-3] BLAKE3 address guard

```rust
pub fn is_b3_addr(s: &str) -> bool {
    s.len() == 67 && s.starts_with("b3:")
        && s.as_bytes()[3..].iter()
            .all(|b| matches!(b, b'0'..=b'9'|b'a'..=b'f'))
}
```

### [C-4] Verifier/Signer traits (PQ-capable)

```rust
pub trait Verifier<Sig> {
    fn verify(&self, canonical_body: &[u8], sig: &Sig) -> bool;
}
pub trait Signer<Sig> {
    fn sign(&self, canonical_body: &[u8]) -> Result<Sig, SignError>;
}

pub fn verify_signed_map<V,S>(v: &V, m: &SignedTldMap<S>) -> bool
where V: Verifier<S>, S: Clone {
    let body = canonical_encode(&m.body); // deterministic CBOR/JSON
    m.signatures.iter().all(|s| v.verify(&body, s))
}
```

### [C-5] CLI contract (feature `cli`)

* Commands: `lint`, `pack`, `sign`, `verify`, `show`.
* I/O rules: inputs via explicit file paths or stdin (`-`), outputs to stdout (default) or `--out <path>`.
* Exit codes: `0` success, `2` validation failed, `3` signature failed, `64+` CLI misuse.

### [C-6] Compile/CI glue

* `#![forbid(unsafe_code)]` in lib; `clippy.toml` disallows `unwrap_used`, `expect_used`.
* `cargo-deny` clean (licenses/bans/advisories/sources).
* `cargo-public-api` gate on public surface.

---

## 4) Acceptance Gates (PROOF)

**Unit / Property**

* **[G-1] Idempotence:** `normalize(normalize(x)) == normalize(x)` for corpora (ASCII, Latin-1, Cyrillic, CJK, Emoji, mixed scripts).
* **[G-2] Round-trip:** JSON and CBOR round-trip exact values; unknown fields rejected.
* **[G-3] Address hygiene:** near-miss fuzzing fails (`b3-`, uppercase hex, wrong length).
* **[G-4] TldMap digesting:** order changes do **not** alter digest; entry changes **do** alter digest.

**Fuzzing**

* **[G-5] Name fuzzer:** arbitrary Unicode → normalize never panics; errors are typed.

**Tooling / CI**

* **[G-6] Public API guard:** `cargo-public-api` diff acknowledged for any change.
* **[G-7] Unicode/IDNA pins:** bumps fail CI unless vectors/regenerated tables accompany PR.
* **[G-8] Workspace pins:** enabling `cli` doesn’t raise MSRV or violate workspace dependency policies.
* **[G-9] Perf sanity:** normalization p95 ≤ **50µs** for short labels on baseline dev hardware (Criterion tracked; informational gate).

**CLI (feature `cli`)**

* **[G-10] CLI conformance:**

  * `tldctl --help` exits 0; subcommands validate arguments.
  * Golden tests for `lint/pack/sign/verify/show` (use a **mock signer**).
  * **No network** attempted (assertion in tests); I/O restricted to stdin/stdout/explicit paths.
  * Canonical bytes stable across runs (snapshot tests).

---

## 5) Anti-Scope (Forbidden)

* ❌ Resolvers, caches, databases, DHT, or network I/O.
* ❌ Economic/governance policy decisions (beyond **schemas** and **signature envelopes**).
* ❌ Alternate hash/address formats in public types (BLAKE3 only).
* ❌ Hidden state or implicit persistence in CLI or lib.
* ❌ Raising MSRV or introducing non-workspace-pinned deps without approval.

---

## 6) References

* **Pillar 9** — Content Addressing & Naming: lib/runtime split (naming vs index).
* **Six Concerns** — SEC (validation, signatures), GOV (artifact governance).
* **Hardening Blueprint** — DTO hygiene, input limits, deny-unknown.
* **Full Project Blueprint** — BLAKE3 addressing, OAP constants, workspace pinning.
* **MERGE TODO** — `tldctl` folded into `ron-naming` as a thin, offline CLI; net crate count unchanged.

---

### Definition of Done (for this blueprint)

* Invariants lock the lib/CLI boundary, deterministic normalization, canonical encodings, BLAKE3 format, PQ-ready verifier, amnesia compatibility, and workspace/MSRV constraints.
* Gates cover **idempotence, round-trip, fuzz, API/Unicode pins, perf sanity, and CLI conformance with no network**.
* Anti-scope prevents drift into runtime/econ/network/alternate hashing.

```



---

## INTEROP.MD
_File 6 of 13_



````markdown
# 🔗 INTEROP.md — ron-naming

*Audience: developers, auditors, external SDK authors*  
*msrv: 1.80.0*

---
## 0) Purpose

Define the **interop surface** of `ron-naming`:

* Canonical **DTOs & schemas** for names and governance artifacts.  
* **Normalization** and **address hygiene** rules all crates/SDKs must follow.  
* **Canonical encodings** (JSON/CBOR) and **test vectors**.  
* CLI (`tldctl`) **file/stdio contract** for offline authoring.  

> `ron-naming` is **wire-neutral** (pure library + optional offline CLI). Network protocols (HTTP/OAP/QUIC), TLS, and readiness belong to host services (e.g., `svc-index`) that embed this crate.

---

## 1) Protocols & Endpoints

**Ingress protocols:** N/A (library).  
**Exposed endpoints:** N/A (library).

**Transport invariants (context for hosts):**
- **OAP/1** `max_frame = 1 MiB`.  
- **Storage** streaming chunk ≈ **64 KiB** (storage detail; unrelated to naming).  
- **TLS** type at hosts: `tokio_rustls::rustls::ServerConfig`.

> Host crates using `ron-naming` (e.g., `svc-index`) must enforce these on *their* ingress surfaces.

---

## 2) DTOs / Schemas

### 2.1 Core types (Rust)

```rust
pub struct CanonicalName(String);          // NFC + IDNA processed, lowercase, policy-checked
pub struct Label(String);                  // single normalized label

pub enum NameRef {
  Human   { name: CanonicalName },
  Address { b3: String },                  // "b3:<64-lower-hex>"
}

pub struct TldEntry {
  pub tld: Label,
  pub owner_key_id: String,                // logical key identifier (not a secret)
  pub rules_version: u32,                  // policy bundle / table version
}

pub struct TldMap {
  pub version: u64,                        // monotonic map version
  pub entries: Vec<TldEntry>,
}

pub struct SignedTldMap<Sig> {
  pub body: TldMap,                        // canonicalized body
  pub signatures: Vec<Sig>,                // detached; multi-sig supported
}
````

**Normalization & hygiene (library functions)**

```rust
pub struct NormalizationOptions { /* policy bundle + strict/mixed-script knobs */ }

pub fn normalize_name(
  input: &str,
  opts: &NormalizationOptions
) -> Result<CanonicalName, ValidationError>;

pub fn is_b3_addr(s: &str) -> bool;       // accepts only "b3:<64-lower-hex>"
```

**Canonical encodings**

```rust
pub enum WireFormat { Json, Cbor }

pub fn to_canonical_bytes<T: Serialize>(
  t: &T,
  fmt: WireFormat
) -> Result<Vec<u8>, WireError>;

pub fn from_bytes<T: DeserializeOwned>(
  buf: &[u8],
  fmt: WireFormat
) -> Result<T, WireError>;
```

> All public DTOs are `serde` types and must use `#[serde(deny_unknown_fields)]` in their definitions (strict schema).

**Verifier/Signer (feature `verify`)**

```rust
pub trait Verifier<Sig> {
  fn verify(&self, canonical_body: &[u8], sig: &Sig) -> bool;
}

pub trait Signer<Sig> {
  fn sign(&self, canonical_body: &[u8]) -> Result<Sig, SignError>;
}

pub fn verify_signed_map<V, S>(v: &V, m: &SignedTldMap<S>) -> bool
where
  V: Verifier<S>,
  S: Clone;
```

Backends (Ed25519, PQ, HSM/KMS) plug in behind these traits.

### 2.5 Signature Envelope (Interop Format)

To keep polyglot compatibility and PQ-hybrid clarity, signatures are explicit, detached, and ordered:

```json
{
  "body": { /* TldMap */ },
  "signatures": [
    {
      "alg": "Ed25519",         // or "Dilithium2", "Falcon512"
      "kid": "k:ops-2025",      // logical key id; no secret material present
      "policy": { "required": true },  // optional; host m-of-n policy lives outside
      "sig": "base64url…"       // opaque bytes; canonical over CBOR(body)
    },
    {
      "alg": "Dilithium2",
      "kid": "k:pq-2025",
      "sig": "base64url…"
    }
  ]
}
```

**Verification rule:** `verify_signed_map` operates over **canonical CBOR of `body`**. Hybrid ≠ concat—each signature is independent; host policy decides m-of-n.

---

## 3) CLI (`tldctl`) — Offline Interop Contract

**Subcommands (stable):**

* `lint <in>` — validate & normalize; exit non-zero on error.
* `pack <in> [--out <out>]` — emit canonical bytes (JSON or CBOR).
* `sign <in> [--out <out>]` — produce `SignedTldMap` using external signer.
* `verify <in>` — verify detached signatures.
* `show <in>` — pretty print parsed/normalized view.

**I/O model:**

* Inputs via explicit path or `-` (stdin).
* Outputs default to stdout or `--out <path>`.
* No network or DB; one-shot execution; machine-friendly exit codes.

**Flags (subset most relevant for interop):**

* `--format {json|cbor}`, `--pretty`
* Normalization policy: `--strict`, `--allow-mixed-scripts`,
  `--allowed-scripts ...`, `--policy-bundle <dir>`
* Signing/verify: `--sign-profile <id>`, `--key-id <id>`, `--pq-mode {off|hybrid}`
* Logging: `--log-format json|text`, `--log-level info|debug|trace`

**Config precedence:** `flags > env (TLDCTL_* > RON_NAMING_*) > file > defaults`.

### 3.5 CLI Examples (Offline, Deterministic)

```bash
# Normalize + lint (stdin → stdout), strict policy
tldctl lint - --strict < ./fixtures/name.json

# Pack canonical CBOR and write to file
tldctl pack ./fixtures/tldmap.json --format cbor --out ./out/tldmap.cbor

# Sign with PQ hybrid (detached multi-sig), then verify
tldctl sign ./out/tldmap.cbor --sign-profile ops --pq-mode hybrid --out ./out/signed.cbor
tldctl verify ./out/signed.cbor
```

Exit codes: `0=ok, 2=usage, 3=validation, 4=signature_invalid, 64+=unexpected`.

---

## 4) Canonical Test Vectors

> Store under `tests/vectors/ron-naming/` and mirror in `/docs/api-history/ron-naming/` on releases.

### 4.1 Normalization

* **Input (UTF-8):** `"Exämple.COM"`
* **Policy:** strict, NFC, IDNA UTS-46
* **Output `CanonicalName`:** `"exämple.com"` (lowercase, NFC)
* **JSON (pretty):**

  ```json
  { "name": "exämple.com" }
  ```

### 4.2 Address hygiene

* **Input:** `"b3:4f0c...<64 hex total>..."` → `is_b3_addr == true`
* **Near-miss rejects:** `"B3:..."` (case), `"b3:...63hex"`, `"b3:...65hex"`, `"b2:..."`.

### 4.3 TldMap (JSON canonical)

```json
{
  "version": 7,
  "entries": [
    { "tld": "example", "owner_key_id": "k:ops-2025", "rules_version": 3 }
  ]
}
```

**CBOR canonical (hex dump excerpt):**

```
a2                                      # map(2)
  67                                    # text(7) "version"
  07                                    # 7
  67                                    # text(7) "entries"
  81                                    # array(1)
    a3                                  # map(3)
      63                                # text(3) "tld"
      67 65 78 61 6d 70 6c 65           # "example"
      6b                                # text(11) "owner_key_id"
      6a 6b 3a 6f 70 73 2d 32 30 32 35  # "k:ops-2025"
      6d                                # text(12) "rules_version"
      03
```

### 4.4 SignedTldMap

* **Body canonical bytes:** from `to_canonical_bytes(body, Cbor)`
* **Signatures:** detached; example:

  ```json
  {
    "body": { "...": "TldMap as above ..." },
    "signatures": [
      { "alg": "Ed25519", "kid": "k:ops-2025", "sig": "base64…" },
      { "alg": "Dilithium2", "kid": "k:pq-2025", "sig": "base64…" }
    ]
  }
  ```
* **Verification:** `verify_signed_map` → `true` only if required policy passes (e.g., m-of-n).

### 4.5 Vector Coverage & CI Gates

* **Coverage targets:** at least one vector per class:

  * **ASCII**, **Latin w/ diacritics**, **CJK**, **RTL**, **Emoji-in-name (rejected)**,
    **Mixed-script (rejected)**, **Confusable (rejected)**.
* **CI job:** regenerate vectors → compare canonical bytes → fail on diff unless PR includes:

  1. table/version bump rationale,
  2. updated vectors,
  3. SemVer note (minor/major) if canonical bytes changed.

---

## 5) Error Taxonomy (library/CLI)

**Validation/normalization (library):**
`unknown_field`, `oversize`, `unicode_confusable`, `mixed_script`, `bad_address_format`,
`canonical_mismatch`, `rules_unsupported_version`.

**CLI (`tldctl`) exit codes:**
`0` success; `2` usage error; `3` validation failure; `4` signature invalid; `64+` unexpected/internal.

> Hosts should map library errors to structured HTTP/OAP errors in their own interop specs (e.g., `oversize → 413`).

---

## 6) Bus Topics & Observability Tie-In

`ron-naming` publishes **no** bus events itself.
Hosts SHOULD emit:

* `<svc>.naming.policy_loaded` (Gauge bool) — becomes `naming_policy_loaded` metric.
* `<svc>.naming.parse_error{reason}` (Counter) — mirrors `naming_manifest_parse_errors_total{reason}`.
* `<svc>.naming.selftest_ok` (Gauge bool) — set after boot round-trip.

This aligns events ↔ metrics to make dashboards and alerts trivial.

---

## 7) Interop Guarantees

* **Canonical encodings** are stable across minor versions; any change that alters canonical bytes requires a **major** version.
* **DTO hygiene:** unknown fields are **rejected** on read (strict schema).
* **No network/DB side effects** in either lib or CLI.
* **Verifier/Signer traits** provide a stable boundary for Ed25519 and PQ backends; adding new algorithms behind features is **additive**.
* **Vectors are normative**; release CI re-generates and compares canonical outputs.
* **Amnesia embedding:** When hosts run with amnesia ON, policy tables and vectors are loaded from read-only bundles at boot; no persistent caches are assumed.

---

## 8) SDK Notes (polyglot)

When porting to TS/Python/Swift:

* Mirror DTOs exactly; enforce lowercasing + IDNA/NFC in **one** place.
* Deny unknown fields; match the address regex `^b3:[0-9a-f]{64}$`.
* Use CBOR canonical form with deterministic map ordering for signatures.
* Provide a single `normalizeName(input, options)` entrypoint to avoid divergence.

**TypeScript sketch:**

```ts
export type CanonicalName = string;

export function isB3Addr(s: string): boolean {
  return /^b3:[0-9a-f]{64}$/.test(s);
}

export function normalizeName(input: string, opts: {
  strict?: boolean;
  allowedScripts?: string[];
}): CanonicalName {
  // delegate to platform IDNA/NFC lib; ensure lowercase + policy
  // exact behavior must match Rust vectors (round-trip tests in CI)
  throw new Error("impl in SDK; verify against vectors");
}
```

---

## 9) Interop Gates (CI-Enforceable)

* **API & schema drift:** `cargo public-api --deny-changes` and schema compile tests.
* **Vectors:** `regen-vectors && compare-vectors` must pass; diffs require explicit approval + SemVer note.
* **Determinism:** Two runs of `pack` over the same input (same tables) must byte-match.
* **No-network guard:** `tldctl` executes under egress sandbox; any socket attempt fails CI.

---

## 10) References

* **GMI-1.6 Omni-Gate** (host interop baseline)
* **OBSERVABILITY.md** — metrics/spans a host should emit when calling the library
* **API.md** — surface & SemVer gates
* **SECURITY.md** — confusable/mixed-script policy, signing/verification model

---

## 11) Diagram

```mermaid
flowchart LR
  subgraph Dev/Ops
    A[tldctl (offline)] -->|pack/sign/verify| B{Canonical bytes}
  end
  B --> C[Vectors (JSON/CBOR)]
  C --> D[SDK ports (TS/Py/Swift)]
  D --> E[Hosts (svc-index)]
  E -->|serve OAP/HTTP| Users
  style B fill:#b91c1c,stroke:#7f1d1d,color:#fff
```

```



---

## OBSERVABILITY.MD
_File 7 of 13_



---

```markdown
# 📈 OBSERVABILITY.md — ron-naming

*Audience: developers, operators, auditors*  
*msrv: 1.80.0 (Tokio/loom compatible)*

> Role recap: `ron-naming` is a **library** for naming schemas, normalization, and canonical encodings (no network, no DB).  
> It also exposes an **optional offline CLI** (`tldctl`, feature `cli`) for authoring/linting/packing/signing/verifying governance artifacts.  
> Runtime surfaces like `/metrics`, `/healthz`, `/readyz` are provided by **host services** (e.g., `svc-index`) that embed this crate.

---

## 0) Purpose

Define **what is observable** for ron-naming and **how hosts should expose it**:

* Library-level signals for **validation/normalization** work
* CLI (`tldctl`) signals for **artifact authoring** workflows
* Conventions for logs, tracing, and correlation
* Recommended **host-facing** metrics, health semantics, and alerts

---

## 1) Metrics (Prometheus-style)

> The library itself does not run a server. Metrics are **emitted by hosts** that call into ron-naming (e.g., `svc-index`, a build tool, or a test harness).  
> `tldctl` is offline; it **does not** scrape/serve Prometheus but can print JSON events that hosts can ingest.

### 1.1 Library “golden” signals (to be exported by hosts)

- `naming_manifest_validation_seconds{operation,ok}` (Histogram)  
  - `operation ∈ {normalize,validate,encode,decode}`  
  - Bucket policy should capture typical artifact sizes (small/medium/large).
- `naming_manifest_parse_errors_total{reason}` (Counter)  
  - `reason` examples: `unknown_field`, `oversize`, `unicode_confusable`, `mixed_script`, `bad_address_format`.
- `naming_address_hygiene_total{result}` (Counter)  
  - `result ∈ {accepted,rejected}` for `b3:<hex>` guards and near-miss rejects.
- `naming_canonical_bytes_total{format}` (Counter)  
  - Count of canonical encodings emitted by hosts (e.g., `json`, `cbor`).

> Registration discipline: **register once** at host init; pass handles into code paths that call the library; avoid duplicate registration.  

### 1.2 CLI (`tldctl`) signals (offline)

`tldctl` emits **structured JSON events** to stderr (one line per event) that a wrapper can ship to logs/metrics:

- `event="tldctl_op"` with fields: `op ∈ {lint,pack,sign,verify,show}`, `ok`, `duration_ms`, `input_bytes`, `output_bytes`.  
- Aggregators may transform these into counters/histograms:
  - `tldctl_ops_total{op,ok}` (Counter)  
  - `tldctl_op_duration_seconds{op}` (Histogram)

### 1.3 What **not** to do

- No direct network exporters inside ron-naming.  
- No implicit global registries; hosts **own** registration and lifetimes.

---

## 2) Health & Readiness

> ron-naming has **no server**; health is **host-defined**. Use the following guidance when embedding:

### 2.1 Endpoints (host responsibility)

- `/healthz` — simple process liveness (host).  
- `/readyz` — returns `200` only after the host has loaded config, attached bus/listeners, and validated any naming policies it relies on.

### 2.2 Readiness keys (when naming policy is material to serving)

- `naming_policy_loaded` — policy tables (Unicode/IDNA/confusables) pinned and validated.  
- `naming_vectors_loaded` — golden vectors parsed and ready for spot checks.  
- `naming_selftest_ok` — host performed a quick round-trip (normalize→encode→decode) at boot.

### 2.3 Failure semantics (host convention)

- Reads can be **fail-open** only if policy isn’t critical for the route; otherwise **fail-closed** with `503` and JSON body:  
  `{ "degraded": true, "missing": ["naming_policy_loaded"], "retry_after": 1 }`.

---

## 3) Logs

### 3.1 Structure & fields

- JSON Lines; one event per line; UTC timestamps.
- Recommended fields produced by hosts and/or `tldctl`:
  - `ts` (ISO-8601), `level` (`INFO|WARN|ERROR|DEBUG|TRACE`)
  - `service` (host service name or `tldctl`)
  - `event` (e.g., `naming.validate`, `naming.normalize`, `tldctl_op`)
  - `reason` (align with metrics labels when a reject/err happens)
  - `corr_id` (ULID/UUID propagated by host)
  - `duration_ms`, `bytes_in`, `bytes_out`
  - `content_id` (if an artifact has a `b3:<hex>` address)
  - `op` (for `tldctl`: `lint|pack|sign|verify|show`)

### 3.2 Redaction discipline

- No PII; no secrets; never log private keys or capability tokens.
- If logging config snapshots, redact secret keys; log **key IDs** only when signatures are referenced.

### 3.3 Error taxonomy (examples)

- `unknown_field`, `oversize`, `unicode_confusable`, `mixed_script`, `bad_address_format`,
  `canonical_mismatch`, `signature_invalid`, `vectors_mismatch`.

---

## 4) Tracing & Correlation

- Use `tracing` + `tracing-subscriber` (JSON) in hosts; `tldctl` can emit spans in JSON when `--trace` is set.
- Span naming:
  - `lib.ron_naming.normalize`, `lib.ron_naming.validate`
  - `cli.tldctl.lint|pack|sign|verify|show`
  - Hosts wrap these with higher-level spans like `svc.index.resolve`
- Correlation:
  - In hosts: inject/propagate `X-Corr-ID`; include `corr_id` in log events and span fields.
  - In CLI mode: generate a fresh `corr_id` per invocation (printed in the first event) so shells/wrappers can stitch multi-step workflows.
- OTEL:
  - Exporters live in hosts; gate with `otel` feature flags at the host level.

---

## 5) Alerts & SLOs

> Alerts target **hosts** running naming-critical routes (e.g., `svc-index`) and CI/workflows that use `tldctl`. Suggestion set:

### 5.1 CI/Build-time SLOs (applicable to both hosts and CLI pipelines)

- **Golden vectors** pass rate = 100% (hard gate).  
- **Canonical stability**: no diff in canonical bytes for unchanged fixtures.  
- **Validation performance**: p95 `naming_manifest_validation_seconds{operation=validate}` below a budget for representative sizes (document per repo).

### 5.2 Runtime SLOs (hosts using ron-naming on hot paths)

- Policy availability: `naming_policy_loaded` true ≥ 99.99% during serving windows.  
- Validation latency: host-defined p95/p99 budgets per route (e.g., `resolve`).

### 5.3 Alert examples

- `increase(rate(naming_manifest_parse_errors_total{reason=~"unknown_field|confusable"}[5m]))` above baseline for 15m → **WARN** (data quality drift or abuse).  
- Boot readiness missing key: `absent(naming_selftest_ok == 1)` for > 60s after process start → **CRIT** (bad deploy).  
- Canonical bytes drift in CI: diff detected on protected fixtures → **CRIT** (compat regression).

### 5.4 Runbooks

- Link to `RUNBOOK.md` with steps: confirm policy versions, re-run vectors, inspect last schema change, bisect confusables/mixed-script rejects.

---

## 6) CI / Enforcement

- **Vectors & pins:** Unicode/IDNA/confusable tables are **pinned**; CI regenerates and compares canonical outputs; any delta requires explicit approval and updated vectors.  
- **No network invariant:** `tldctl` must not attempt network calls; CI runs with a network sandbox and fails if egress is observed.  
- **Metrics discipline:** host crates register metrics once; a CI grep/lint prevents duplicate registration names.  
- **Tracing discipline:** spans exist for every public library entrypoint used on hot paths; smoke tests assert presence of span fields (`corr_id`, `operation`, `ok`).  
- **Docs freshness:** review this file on a cadence (e.g., quarterly) and stamp the header date.

---

## 7) Reference Implementations (how to wire it)

- **Host (Rust, Axum):** wrap calls to `normalize/validate` with latency timers; increment counters on typed error categories; expose `/metrics`.  
- **CLI pipeline:** run `tldctl ... --json` and pipe stderr to your log shipper; convert `event="tldctl_op"` lines into counters/histograms in your collector.

---

## 8) Known Non-Goals

- ron-naming will not open sockets, publish `/metrics`, or own readiness endpoints.  
- No global singletons or background tasks; all observability is **call-scoped** and **host-owned**.

---
```


---

## OLD_README.md
_File 8 of 13_

# naming

## 1. Overview
- **What it is:** A library crate that defines and validates RustyOnions content addresses (e.g., `b3:<hash>.<ext>`).  
- **Role in RustyOnions:** Ensures that all services (`gateway`, `svc-index`, `svc-overlay`, etc.) use a canonical, well-formed representation of bundle addresses.

---

## 2. Responsibilities
- **What this crate does:**
  - [x] Parses string addresses into structured types.  
  - [x] Validates address format (hash algorithm, extension).  
  - [x] Normalizes addresses to a canonical form.  

- **What this crate does *not* do:**
  - [x] Does not resolve addresses to directories (that’s `svc-index`).  
  - [x] Does not fetch or serve content (that’s `svc-overlay` / `gateway`).  
  - [x] Does not perform hashing itself (delegates to `common`).  

---

## 3. APIs / Interfaces
- **Rust API:**
  - `parse_addr(addr: &str) -> Result<Address>` — Parse a string into an address type.  
  - `Address` struct with fields for hash type, hash bytes, and extension.  
  - `to_string(&self) -> String` — Convert back to canonical string form.  

---

## 5. Configuration
- **Environment variables:** None.  
- **Command-line flags:** None.  

---

## 8. Integration
- **Upstream (who calls this):**  
  - `svc-index` (to validate addresses before storing mappings).  
  - `gateway` and `svc-overlay` (to normalize user input addresses).  

- **Downstream (what it depends on):**  
  - `common` (for hash validation helpers).  

- **Flow:**  
  ```text
  client → gateway (HTTP addr) → naming::parse_addr() → normalized address
```

---

## PERFORMANCE.MD
_File 9 of 13_



---

# ⚡ PERFORMANCE.md — ron-naming

---

title: Performance & Scaling
status: draft
msrv: 1.80.0
crate_type: lib (+ offline CLI `tldctl`)
last-updated: 2025-10-06
audience: contributors, ops, perf testers
-----------------------------------------

# PERFORMANCE.md

## 0. Purpose

This document defines the **performance profile** for `ron-naming` as a **pure library** (schemas, normalization, canonical encodings) and the **offline CLI `tldctl`** used for local validation/vector generation. There is **no online resolution** here; runtime lookups/serving live in `svc-index` (Pillar 9 boundary).

It ties directly into:

* **Scaling Blueprint** guardrails (frame/chunk bounds referenced as system context).
* **Perfection Gates** (Gate F: perf regressions barred; Gate L: scaling validated).
* **IDB invariants**: deterministic outputs, no network/DB side-effects, one normalization path.

---

## 1. SLOs / Targets

### Baseline Environments (to make numbers reproducible)

* **CI Baseline (authoritative for gates):** GitHub-hosted `ubuntu-latest`, 2 vCPU, 7 GB RAM, x86_64, `RUSTFLAGS="-C target-cpu=x86-64-v3"`.
* **Local Reference (informational):** Modern 8-core laptop (e.g., Apple M2/M3 or Intel i7-12700H+), `-C target-cpu=native`.

### Library (single-threaded by default; `parallel` feature uses Rayon for batch ops)

* **Normalization throughput (labels):**

  * ASCII-heavy: ≥ **1.5M labels/sec/core** (local ref), ≥ **500k/sec/core** (CI baseline).
  * Mixed Unicode (NFKC + casefold): ≥ **400k/sec/core** (local ref), ≥ **150k/sec/core** (CI).
* **Parsing/Encoding (small DTOs ≤512 B):**

  * JSON→DTO: ≥ **200k docs/sec/core** (local), ≥ **80k/sec/core** (CI).
  * TOML→DTO: ≥ **80k docs/sec/core** (local), ≥ **30k/sec/core** (CI).
  * Canonical CBOR encode: ≥ **300k ops/sec/core** (local), ≥ **120k/sec/core** (CI).
* **Allocations/op (amortized):**

  * Normalize ASCII: **0** heap allocs (fast path).
  * Normalize Unicode: ≤ **2** short-string allocs/label.
  * JSON→DTO: ≤ **3** transient allocs/doc.
* **Cold init:** crate tables/setup < **5 ms** (no I/O).

### Optional PQ / Verify Feature (only if the crate exposes a `verify` capability)

* **Signature verify throughput (per core):**

  * Ed25519: ≥ **50k verifies/sec** (CI).
  * Hybrid PQ (e.g., Dilithium2+Ed25519): ≥ **10k verifies/sec** (CI).
* **Target deltas:** Hybrid ≤ **+3×** latency vs Ed25519 alone for small payloads.

> If `verify` is not part of `ron-naming`, this subsection is **N/A**; keep the bench harness behind a feature flag so it’s ready if we add verify later.

### CLI (`tldctl`, offline streaming)

* **Latency (single op):** `tldctl normalize example.com` p95 < **2 ms** warm (CI).
* **Throughput (batch):**

  * `--stdin` single-thread: ≥ **450k labels/sec** (local), ≥ **160k/sec** (CI).
  * `--stdin --parallel` (Rayon): ≥ **1.2M/sec** (local, 8 cores), scales ~linearly by core; ≥ **320k/sec** (CI, 2 vCPU).
* **Resource ceilings:**

  * RSS steady < **200 MiB** at 1M-name batch (streaming + bounded buffers).
  * CPU < **85%** of a core (single-thread), scales with threads when `--parallel` used.

**Standards/context (system-wide, not enforced here):**

* OAP/1 max frame **1 MiB**; streaming chunk **64 KiB**. Useful for corpus sizing and DTO micro-bench context.

---

## 2. Benchmarks & Harness

* **Criterion micro-benches (`cargo bench`):**

  * `bench_normalize_ascii`, `bench_normalize_unicode`
  * `bench_parse_json_small`, `bench_parse_toml_small`
  * `bench_cbor_encode_small`
  * `bench_verify_ed25519`, `bench_verify_pq_hybrid` *(behind `verify` feature)*
* **CLI latency/throughput (hyperfine):**

  * `hyperfine 'tldctl normalize example.org'`
  * `hyperfine --warmup 3 'cat testing/corpora/100k.txt | tldctl vectorize --stdin'`
  * `hyperfine --warmup 3 'cat testing/corpora/100k.txt | tldctl vectorize --stdin --parallel'`
* **CPU profiling:** `cargo flamegraph` (hotspots: Unicode tables, TOML parse, CBOR encode).
* **Alloc profiling:** `heaptrack` (Linux) or `valgrind --tool=dhat` (alloc sites).
* **Determinism/property tests:** proptest corpus for idempotence & canonicalization; run alongside benches to catch perf + correctness drift.
* **Chaos/perf blend (CLI):** very long labels, mixed scripts (CJK/RTL), invalid code points/confusables, slow/stdin backpressure.

**CI Integration**

* Nightly perf workflow:

  * Run Criterion; compare JSON to baselines.
  * Run hyperfine batch scripts; append CSV to baselines.
  * Upload flamegraphs on deltas > thresholds.
* Optional: `tokio-console` is N/A (no async hot paths); Rayon parallelism is profiled via flamegraph/coz.

**Perf Flow (visual)**

```mermaid
flowchart TD
  A[Input labels] -->|ASCII fast-path| B[Normalize ≥1.5M/s/core (local)]
  A -->|NFKC+Casefold| C[Normalize ≥400k/s/core (local)]
  B --> D[Alloc ≤2/label (Unicode) / 0 (ASCII)]
  C --> D
  E[CLI batch stdin] -->|single-thread| G[≥450k/s (local)]
  E -->|--parallel (Rayon)| F[≥1.2M/s (local 8c)]
  style D fill:#0b7285,stroke:#083344,color:#fff
```

---

## 3. Scaling Knobs

* **Concurrency:** default single-thread; `parallel` feature enables Rayon in batch CLI paths only.
* **Memory:** bounded reader (64–256 KiB); `SmallVec` for ≤63-byte labels; optional `intern` feature (LRU-capped) for hot TLD forms.
* **I/O:** streaming stdin/stdout; avoid full-file buffering; prefer zero-copy slices for DTO serialization.
* **Build profile:** `-C target-cpu=x86-64-v3` in CI; optional `-C target-cpu=native` locally; thin-LTO toggle for release benches.
* **Amnesia toggle (CLI):** if `RON_AMNESIA=1`, zeroize transient buffers at process exit.

---

## 4. Bottlenecks & Known Limits

* **Unicode normalization dominates** CPU on mixed-script corpora; keep ASCII fast-path hot and consider SIMD table precomputation in Silver.
* **TOML parse is slower** than JSON; prefer JSON for bulk DTO ingest.
* **Line/label size limits:** enforce IDN-style constraints (label ≤ **63 octets**, full name ≤ **253 octets** after canonicalization); reject oversize with typed errors.
* **No network / no sled by design:** anything resembling “resolve” is out-of-scope (belongs in `svc-index`). This preserves CPU-bound, deterministic perf.

**Milestones**

* **Bronze:** targets above met; baselines captured; gates enforced.
* **Silver:** SIMD/arena experiments for normalization; reduced allocs/label.
* **Gold:** large corpus parallel runs, cross-platform baselines (aarch64/x86_64), stabilized variance ≤5%.

---

## 5. Regression Gates

CI fails if (vs previous CI baseline on identical runner; geomean across 5 runs):

* **Normalization p95/op:** ↑ > **10%** (+ an allowed noise band **±5%**).
* **Parse/encode throughput:** ↓ > **10%** (±5% noise).
* **Allocations/op:** ↑ > **15%**.
* **CLI batch throughput:** ↓ > **10%** (±5% noise).

**Baselines**

* Stored under `testing/performance/baselines/ron-naming/`:

  * Criterion JSON snapshots (per-bench).
  * Hyperfine CSV (`cli_batch_throughput.csv`).
  * Example JSON snippet:

    ```json
    {
      "normalize_ascii": { "throughput_ops_per_sec": 520000, "p95_ms": 0.0017 },
      "normalize_unicode": { "throughput_ops_per_sec": 165000, "p95_ms": 0.0056 }
    }
    ```

**Waivers**

* Allowed only with attached flamegraph + heaptrack evidence and a clear upstream cause (e.g., dependency bump). Must include an issue link and a plan to recover.

**Rollback**

* If a regression is confirmed, revert to the last green tag `ron/ron-naming/vX.Y.Z` and re-run perf to re-establish the baseline.

---

## 6. Perf Runbook (Triage)

1. **Confirm corpus** and proportions (ASCII vs Unicode); ensure same file & order.
2. **Flamegraph** micro-benches → confirm hotspots (normalizer, parser, encoder).
3. **Heaptrack** the CLI batch to locate alloc spikes; check SmallVec & interning behavior.
4. **Flip knobs**: `--parallel`, buffer 128→256 KiB, enable/disable `intern`, release+LTO.
5. **Chaos shape**: inject long labels, confusables, invalid code points; verify typed errors and stable throughput.
6. **PQ path (if enabled)**: run `bench_verify_*` and compare deltas to prior.
7. **File an issue** with bench diffs, flamegraph, heaptrack; propose fix & expected gain; link to PR.
8. **Rollback** if needed (see §5).

---

## 7. Acceptance Checklist (DoD)

* [ ] SLOs defined with explicit CI/local baselines.
* [ ] Criterion benches cover normalize (ASCII/Unicode), parse (JSON/TOML), encode (CBOR).
* [ ] Optional PQ verify benches gated behind feature (or marked N/A).
* [ ] Hyperfine scripts for CLI single & batch (parallel + single).
* [ ] Baselines checked in under `testing/performance/baselines/ron-naming/`.
* [ ] CI perf comparison + variance window wired; gates fail on >10–15% drifts.
* [ ] Property tests assert **idempotence & determinism** across perf runs.
* [ ] Perf runbook validated once per milestone.
* [ ] Lib/service boundary re-asserted (no resolve/network/sled here).

---

## 8. Appendix

**Reference workloads**

* `testing/corpora/`:

  * `ascii_100k.txt` (DNS-like, a–z, 0–9, hyphen)
  * `unicode_mixed_100k.txt` (CJK/RTL + Latin)
  * `long_labels.txt` (edge-case max lengths)

**Example scripts**

```bash
# Micro-bench (CI/local)
cargo bench -p ron-naming

# CLI single op
hyperfine 'tldctl normalize example.org'

# CLI batch (single-thread)
hyperfine --warmup 3 'cat testing/corpora/unicode_mixed_100k.txt | tldctl vectorize --stdin'

# CLI batch (parallel)
hyperfine --warmup 3 'cat testing/corpora/unicode_mixed_100k.txt | tldctl vectorize --stdin --parallel'
```

**Perfection Gates**

* **Gate F:** perf regressions barred by CI baselines (with explicit variance band).
* **Gate L:** scaling validated for batch CLI, with flamegraphs and alloc reports attached.

**History**

* Maintain a running log of perf wins/regressions in `CHANGELOG.md` under a “Performance” heading (with links to bench diffs/PRs).

---



---

## QUANTUM.MD
_File 10 of 13_



---

title: Post-Quantum (PQ) Readiness & Quantum Proofing
status: draft
msrv: 1.80.0
last-updated: 2025-10-06
audience: contributors, security auditors, ops
crate: ron-naming
crate-type: lib
pillar: 9
owners: [Stevan White]

# QUANTUM.md

## 0) Purpose

Define how `ron-naming`—a **pure library** (schemas/normalization/canonical encodings) with an **offline CLI** `tldctl`—remains safe under a PQ future.
Because this crate **does not perform network handshakes, token verification, or at-rest encryption**, our PQ focus is:

* **Provenance** of releases and **golden test vectors** (supply-chain integrity).
* **Compatibility hooks** for downstream crates that *do* perform crypto.
* Guardrails to avoid accidentally introducing classical-only crypto here.

Scope: algorithms (if any), release signing, optional vector attestation, tests, rollout plan, and Harvest-Now-Decrypt-Later (HNDL) exposure.

---

## 1) Exposure Assessment (What’s at risk?)

* **Public-key usage (breakable by Shor):**

  * **In-crate:** **None at runtime.** No TLS, no KEX, no signatures during normalization/encoding.
  * **Around the crate:** Git tag **release signing** (maintainer keys) and optional **attestation signatures** on golden vectors. If classical PKI breaks, attackers could spoof releases/vectors.
* **Symmetric/Hash (Grover-affected only):**

  * **In-crate:** Hashing may be used **only for internal IDs/tests** (e.g., vector IDs). We standardize on **BLAKE3-256** (256-bit security margin; Grover-style requires ~2^128). No AEAD is used here.
* **Data at rest / long-lived artifacts:**

  * **Golden vectors**, **baselines**, **SBOMs**, **CHANGELOG**—all public artifacts; **no secrets**.
  * Retention window: **years** (repo history). **HNDL risk: Low**—there’s nothing private to decrypt; provenance can be attacked if signatures are classical-only (mitigated below).
* **Transport/Session lifetime:** N/A (no transport).
* **Worst-case blast radius if classical PKI breaks:**

  * An adversary could distribute a **maliciously modified crate/tag** or **altered golden vectors**, tricking downstreams. Runtime resolution/data privacy is unaffected (those live in other crates).

> **HNDL:** Low. There are no confidential payloads produced by this crate; risk concentrates on **authenticity of releases**.

---

## 2) Current Crypto Profile (Today)

* **Algorithms in use (runtime):** **None.**
* **Provenance (release process):**

  * Git tag **signatures** by maintainers (classical, e.g., Ed25519 via git tooling).
  * **SBOM + checksums** attached to releases.
  * Optional: **hashes (BLAKE3-256)** for golden vector bundles.
* **Libraries linked for crypto:** None required by the lib itself; only dev/release tooling.
* **Key custody (release signing):** Maintainer keys in KMS/HSM or hardware tokens; rotation policy 90 days or post-incident.
* **Interfaces that carry crypto:** N/A (library); `tldctl` does not produce or verify tokens/certs.

---

## 3) Target PQ Posture (Where we’re going)

For **ron-naming**, “PQ readiness” means **provenance hardening** and **clean handoff** to PQ-capable neighbors:

* **Provenance / Attestation**

  * Dual-track release signing: maintain classical signatures **and** add an **independent transparency mechanism** (e.g., transparency log or two-maintainer co-signature).
  * Optional **PQ signature** on **golden vector archives** (e.g., ML-DSA/Dilithium) stored alongside classical sigs so auditors can verify with either.
* **Downstream Interop Hooks**

  * Keep the crate **crypto-free**. Expose **stable DTOs** so `ron-auth`, `ron-kms`, `svc-passport`, and transports can layer **hybrid KEX** and **PQ signatures** in their domains without changes here.
* **Backwards compatibility**

  * PQ attestations are **additive**; classical signatures remain until the workspace reaches **M3 (Gold)** posture elsewhere.
  * `tldctl` remains offline/crypto-free; it may print **vector hash + provenance info** to help auditors verify artifacts signed elsewhere.

---

## 4) Feature Flags & Config (How to turn it on)

```toml
# Cargo features (no runtime crypto here; these are NO-OP placeholders to prevent drift)
[features]
pq = []               # Reserved: enables PQ attestation *metadata* emission (no crypto)
pq-attest = []        # Reserved: emits vector bundle hash & external-signature URIs
pq-hard-fail = []     # Reserved: CI-only, fail if classical-only provenance detected
```

```ini
# (No runtime config) Example CI/env toggles
RON_NAMING_ATTEST_HASH=blake3-256     # hash algo for vector bundles (default)
RON_NAMING_ATTEST_URI=https://…/sig   # pointer to external PQ signature produced by release infra
```

> All actual **signing** is performed by release infrastructure (e.g., `ron-kms` or CI HSM), **not** by this crate.

---

## 5) Migration Plan (Milestones)

* **M1 (Bronze) — Documentation & Guardrails**

  * Declare **no in-crate crypto**; add **anti-drift CI check** that fails if crypto crates are linked.
  * Hash and publish **golden vector bundles** (BLAKE3-256).
  * Release signing must be **mandatory** (classical), with keys documented and rotated.

* **M2 (Silver) — Attestation & Transparency**

  * Add **PQ attestation** (e.g., Dilithium) for **vector bundles** via external signer; publish sidecar `.sig-pq`.
  * Add **transparency log** entry or multi-maintainer co-signature requirement for releases.
  * CI verifies: (a) classical sig valid, (b) PQ attestation available & well-formed, (c) vector hash matches.

* **M3 (Gold) — Default & Policy**

  * Documentation and RUNBOOK require auditors to **verify PQ attestation** for vectors/releases.
  * Downstream crates (auth/transport/ledger) default to **hybrid**; `ron-naming` remains crypto-free but **references** required PQ verification steps in its release notes.
  * Quarterly **PQ drill**: simulate classical break (treat classical sigs as untrusted); auditors must be able to complete verification via PQ attestation/transparency only.

---

## 6) Invariants (MUST)

* **[PQ-N1] Crypto-free core:** No KEX, signatures, AEAD in `ron-naming` runtime.
* **[PQ-N2] Provenance required:** Every release and golden vector bundle must have **verifiable provenance** (at minimum classical signing + checksums).
* **[PQ-N3] PQ path available:** Provide a **PQ attestation** (external signer) or transparency proof for vector bundles by **M2**.
* **[PQ-N4] Anti-downgrade:** CI must fail if PQ attestation is marked “required” in workspace policy and is missing.
* **[PQ-N5] Stable DTOs:** Changes to DTOs/normalization are SemVer-gated; no hidden crypto material ever embedded.
* **[PQ-N6] No secrets:** CLI/library must never read secrets or private keys.

---

## 7) Observability (Metrics, Logs, Readiness)

* **CLI logs (`tldctl`):** on `--about`/`--vectors-info`, print: `hash_algo`, `vectors_hash`, `provenance_uri`, `pq_attestation_present={true|false}`.
* **CI artifacts:** attach `vectors.tar.zst`, `vectors.tar.zst.b3`, `vectors.tar.zst.sig` (classical), `vectors.tar.zst.sig-pq` (PQ).
* **Dashboards (repo level):** counts of releases with PQ attestation; last rotation date for maintainer keys.

---

## 8) Testing & Verification

* **Unit/property:** unaffected (no crypto).
* **Provenance checks (CI):**

  * Validate **BLAKE3 hash** of vector bundle; confirm presence of classical sig; optionally verify external **PQ signature** (out-of-band tool).
  * Negative tests: corrupt vectors → hash mismatch → CI fails.
* **Security drill (“classical break”):**

  * Treat classical sig as untrusted; verify **only** via PQ attestation/transparency; ensure auditors can reproduce verification with published tools.
* **No fuzz targets here** for crypto; fuzz stays focused on normalization/parse/encode paths.

---

## 9) Risks & Mitigations

* **Release spoofing if classical breaks:** Mitigate with **PQ attestation** and/or a **transparency log** requiring multiple identities.
* **Supply-chain library creep:** Prevent by **CI denylist** of crypto crates in `ron-naming`; crypto belongs to other crates.
* **Operational confusion:** Document clearly that `ron-naming` is **crypto-free**; PQ controls live in **release process** and **downstream services**.
* **Verifier tooling fragmentation:** Publish a **single verifier script** (repo `tools/verify_vectors.sh`) that checks hash + classical sig + optional PQ attestation.

---

## 10) Acceptance Checklist (DoD)

* [ ] README/SECURITY/RUNBOOK explicitly state **no runtime crypto** in `ron-naming`.
* [ ] Release process signs tags and publishes **SBOM + checksums**.
* [ ] Golden vector bundles produced with **BLAKE3-256**; hashes published.
* [ ] CI includes **anti-drift** (no crypto deps), vector hash check, and presence of signatures.
* [ ] **M2:** PQ attestation available for vectors; verifier script documented.
* [ ] Quarterly **PQ drill** executed; results logged.

---

## 11) Role Presets (how this crate aligns)

* **kernel/lib:** Crypto-agnostic; expose no keys, no handshakes.
* **transport/identity/econ (neighbors):** Implement **hybrid KEX**, **PQ signatures**, **pq_only** policies **outside** this crate; `ron-naming` DTOs remain stable to avoid re-signing churn.

---

## 12) Appendix

* **Algorithms:**

  * Hash for vector bundles: **BLAKE3-256** (deterministic, fast; 256-bit security level).
  * PQ attestation (external): **ML-DSA (Dilithium)** or **SLH-DSA (SPHINCS+)** as provided by release infra.
* **Libraries:** none linked in this crate; external signing handled by CI/KMS.
* **Interop notes:** Downstreams (e.g., `ron-auth`, `ron-kms`, transports) will negotiate **hybrid**; this crate stays unchanged.
* **Change log (provenance):**

  * 2025-Q4: Added vector hashing and signed bundles (classical).
  * 2026-Q1: Added optional PQ attestation; published verifier script.

---

**Bottom line:** `ron-naming` remains **crypto-free at runtime** and therefore has **negligible HNDL risk**. Our PQ readiness centers on **provenance**—making sure auditors can **trust what they build and test**, even if classical PKI weakens—while leaving heavy cryptographic negotiation and enforcement to the crates designed for it.


---

## RUNBOOK.MD
_File 11 of 13_



---

title: RUNBOOK — ron-naming
owner: Stevan White
msrv: 1.80.0
last-reviewed: 2025-10-06
audience: operators, SRE, auditors

# 🛠️ RUNBOOK — ron-naming

## 0) Purpose

Operational manual for **ron-naming** (Pillar 9 library for schemas/normalization/canonical encodings) and its **offline CLI `tldctl`** used for local validation, normalization, and vector generation.
There is **no online resolution** here (that belongs to `svc-index`).
This runbook satisfies **PERFECTION_GATES**: **K** (Continuous Vigilance) and **L** (Black Swan Economics).

---

## 1) Overview

* **Name:** `ron-naming`
* **Role:** Pillar 9 **library** + **offline CLI** (`tldctl`) for normalization, parsing, canonical encodings, test vectors
* **Criticality Tier:** **2** (supporting) — foundational for correctness; not on the request path
* **Runtime Dependencies (CLI):** none (stdin/stdout only)
* **Build/Dev Dependencies:** Rust toolchain (msrv 1.80.0); standard workspace deps
* **Ports Exposed:** **None** (not a service)
* **Data Flows:** `stdin | file` → `tldctl` → `stdout` (JSON/CBOR/TOML as configured)
* **Version Constraints:** stable **SemVer**; consumers pin `ron-naming >= X.Y` per workspace policy

---

## 2) Startup / Shutdown

### Build & Use (CLI)

```bash
# Build release binary for the CLI
cargo build -p ron-naming --bin tldctl --release

# Normalize a single name
./target/release/tldctl normalize example.org

# Vectorize a corpus from stdin (newline-delimited)
cat testing/corpora/unicode_mixed_100k.txt | ./target/release/tldctl vectorize --stdin

# Parallel batch processing (if built with the `parallel` feature)
cat testing/corpora/unicode_mixed_100k.txt | ./target/release/tldctl vectorize --stdin --parallel
```

**Environment knobs:**

* `RUST_LOG=info|debug|trace` — structured logs to stderr
* `RON_AMNESIA=1` — zeroize transient buffers on exit (defensive hardening)
* `RUSTFLAGS="-C target-cpu=native"` — local perf runs; CI uses x86-64-v3

**Verification (sanity):**

```bash
./target/release/tldctl selftest
# exit code 0 and "selftest_ok" in output indicates a healthy build
```

### Shutdown

* Normal: process exits after finishing stdin; non-interactive
* Long-running pipelines: `Ctrl-C` to terminate cleanly (ensures buffered output is flushed)

---

## 3) Health & Readiness

There are **no /healthz or /readyz endpoints**. Use **self-tests** and smoke commands:

* **Self-test:** `tldctl selftest` → exit code 0 if tables/pipeline load correctly
* **Smoke (ASCII):** `tldctl normalize example.org` → emits canonical form without error
* **Smoke (Unicode):**

  ```bash
  printf "Ｅxample.ｏrg\n" | tldctl vectorize --stdin
  ```

  Expect normalized output; failures indicate Unicode table/normalization issues

---

## 4) Common Failure Modes

| Symptom                                 | Likely Cause                              | Evidence / Where to Look                  | Resolution                                                 | Alert/Action    |
| --------------------------------------- | ----------------------------------------- | ----------------------------------------- | ---------------------------------------------------------- | --------------- |
| Non-zero exit on `selftest`             | Build/runtime mismatch; corrupted tables  | stderr logs (`RUST_LOG=debug`)            | Rebuild release; run `cargo clean`; re-run `selftest`      | Block release   |
| Very slow batch throughput              | Built without native flags or `parallel`  | perf logs; hyperfine baseline regressions | Rebuild with `-C target-cpu=native`; enable `--parallel`   | File perf issue |
| “Invalid label length” errors           | Oversized label (>63 octets)              | stderr structured error                   | Validate inputs; split labels; reject per policy           | N/A             |
| Unicode confusable/invalid codepoints   | Malformed corpus                          | stderr typed errors; proptest failures    | Fix corpus; keep malformed set for fuzz/regression         | N/A             |
| Memory usage spikes on huge single line | Unbounded input line; not newline-delim   | RSS growth; heaptrack                     | Enforce newline-delimited input; pre-chunk source files    | N/A             |
| CI perf gate failure                    | Upstream dep changed; code path regressed | Criterion JSON diff; flamegraph artifacts | Investigate hotspot; file waiver with evidence or rollback | Paging in CI    |

---

## 5) Diagnostics

**Logs (structured, stderr):**

```bash
RUST_LOG=debug ./target/release/tldctl normalize example.org 2>debug.log
grep -E "level=|error=|duration=" debug.log
```

**Perf snapshots:**

```bash
# Microbench (Criterion)
cargo bench -p ron-naming

# CLI latency/throughput (hyperfine)
hyperfine 'tldctl normalize example.org'
hyperfine --warmup 3 'cat testing/corpora/100k.txt | tldctl vectorize --stdin'
```

**Flamegraph & allocations:**

```bash
cargo flamegraph -p ron-naming --bin tldctl -- normalize example.org
heaptrack ./target/release/tldctl -- vectorize --stdin < testing/corpora/100k.txt
```

**Determinism/property:**

```bash
cargo test -p ron-naming --features proptest
```

---

## 6) Recovery Procedures

1. **Build/Config Drift**

   * Symptom: `selftest` fails or unexpected normalization
   * Action: `cargo clean && cargo build -p ron-naming --bin tldctl --release`
   * Verify: `tldctl selftest` → 0

2. **Perf Regression**

   * Symptom: hyperfine/Criterion below baselines
   * Action: run flamegraph + heaptrack; try `--parallel`, bump chunk 128→256 KiB; re-bench
   * If confirmed: open issue with artifacts; consider rollback to last green tag `ron/ron-naming/vX.Y.Z`

3. **Corpus Issues**

   * Symptom: frequent input errors on pipelines
   * Action: validate corpus (newline-delimited; UTF-8); segregate malformed lines; re-run

4. **Toolchain Breakage**

   * Symptom: build fails after toolchain bump
   * Action: pin stable toolchain for msrv; re-sync workspace versions; re-run `cargo-deny` and benches

5. **Amnesia/Security Posture**

   * Symptom: sensitive data remained in memory longer than desired (rare; lab paranoia mode)
   * Action: set `RON_AMNESIA=1`; re-run workload; confirm zeroization hooks executed on exit (log note)

---

## 7) Backup / Restore

* **Statefulness:** None. `ron-naming` and `tldctl` are **stateless**.
* **What to preserve:** test corpora, perf baselines (`testing/performance/baselines/`), and CI artifacts (flamegraphs/CSV).
* **Restore:** re-checkout repo + baselines; rebuild; re-run `selftest` and perf smoke.

---

## 8) Upgrades

1. Land PR with CHANGELOG entry (include perf notes if any).
2. CI must be green on: benches vs baselines, clippy, cargo-deny.
3. Tag `ron/ron-naming/vX.Y.Z`.
4. Rebuild `tldctl`; run `selftest` + two smoke commands (ASCII + Unicode).
5. If downstream consumers (e.g., `svc-index`) rely on DTOs, confirm no breaking changes (SemVer).

---

## 9) Chaos & Black-Swan Testing (Quarterly)

* **Fuzz/Property:** run proptests with confusables/invalid code points.
* **Compression/size bombs:** very long labels and max-length names; ensure graceful reject + stable memory.
* **Throughput soak:** 1M-line batch via `--stdin` (parallel on/off); watch RSS & throughput variance.
* **Outcome:** attach artifacts (logs, flamegraphs) to the quarterly drill report. Gate **J/K/L** must pass.

---

## 10) Scaling Notes

* **Parallelism:** `--parallel` (Rayon) for batch; scales ~linearly by core.
* **Buffers:** try 64→128→256 KiB to trade memory for throughput.
* **I/O:** always prefer streaming (`--stdin`); avoid full-file buffering scripts.
* **Targets:** see PERFORMANCE.md §1 (CI vs local baselines). Treat CI numbers as authoritative for gates.

---

## 11) Security Ops

* **No secrets** in inputs or logs; `tldctl` writes results to stdout, diagnostics to stderr.
* **Amnesia:** enable `RON_AMNESIA=1` for zeroization on exit.
* **PQ posture:** if a `verify` feature is enabled in the future, track its perf & key handling in QUANTUM.md; for now, N/A.
* **Supply chain:** `cargo-deny` must be green; lockfile updates reviewed.

---

## 12) References

* `PERFORMANCE.md` — SLOs, baselines, gates, scripts
* `CONCURRENCY.md` — single-thread default; optional Rayon for batch
* `OBSERVABILITY.md` — what to log/measure for perf triage (CLI context)
* `SECURITY.md` — malformed input handling, zeroization notes
* `IDB.md` — invariants (deterministic normalization; no network)
* Blueprints: Hardening, Concurrency & Aliasing, Scaling, Omnigate

---

## ✅ Perfection Gates Checklist

* [ ] **K (Continuous Vigilance):** selftest + smoke wired in pre-release; logs/artefacts retained
* [ ] **L (Black Swan Economics):** quarterly chaos drill (fuzz, size bombs, soak) passed
* [ ] Perf CI gates green (≤10–15% drift within variance window)
* [ ] No service creep (no ports, no network, no sled)
* [ ] CHANGELOG entry includes any perf deltas and mitigation notes

---


---

## SECURITY.MD
_File 12 of 13_


````markdown
---
title: Security Notes — ron-naming
crate: ron-naming
owner: Stevan White
last-reviewed: 2025-10-06
status: draft
---

# Security Documentation — ron-naming

This document defines the **threat model**, **security boundaries**, and **hardening requirements** specific to `ron-naming`.  
It complements the repo-wide Hardening Blueprint and Interop/IDB docs. The crate exposes **two artifacts**:

1) **Library** (default): schemas, normalization, validation, canonical encodings, address hygiene.  
2) **CLI `tldctl`** (feature `cli`): thin, **offline** wrapper for authoring/linting/packing/signing/verifying governance artifacts using the library. **No network or DB/DHT**. :contentReference[oaicite:0]{index=0} :contentReference[oaicite:1]{index=1}

---

## 1) Threat Model (STRIDE)

> Scope is tailored to **pure parsing/validation** (library) and **offline authoring** (CLI). Resolution/runtime auth lives in `svc-index` and friends.

| Category | Example threats | Relevant? | Mitigations / Notes |
|---|---|---:|---|
| **S**poofing | Names that look alike (confusables), forged governance artifacts | Yes | Unicode normalization + IDNA/UTS-46; strict confusable/mixed-script policy; BLAKE3 address format guard; detached, multi-sig envelopes via pluggable verifiers. :contentReference[oaicite:2]{index=2} :contentReference[oaicite:3]{index=3} |
| **T**ampering | Modified `TldMap` bytes, reordered entries | Yes | Canonical encodings (CBOR/JSON) with `deny_unknown_fields`; order-invariant digest for maps; canonical digest over **body**. :contentReference[oaicite:4]{index=4} :contentReference[oaicite:5]{index=5} |
| **R**epudiation | “We never published that map” | Partial | Audit/attestation lives in `ron-audit`; library provides typed structures & canonical bytes to feed audit systems. (Design choice: keep GOV in dedicated crates per Pillar model.) :contentReference[oaicite:6]{index=6} |
| **I**nfo Disclosure | PII leakage embedded in artifacts | Yes | Schema review + CI pinning of Unicode/IDNA tables; deny unknown fields; amnesia mode when embedded (no spill). :contentReference[oaicite:7]{index=7} |
| **D**oS | Oversized/cyclic inputs; path explosions in CLI | Yes | Hard caps on serialized sizes (<< OAP 1 MiB frame); explicit path validation; no network; offline, one-shot CLI. :contentReference[oaicite:8]{index=8} :contentReference[oaicite:9]{index=9} |
| **E**oP | Using `tldctl` to bypass capability checks | Yes | `tldctl` generates/verifies artifacts **only**; runtime capability enforcement happens in services (e.g., svc-index). No ambient authority here. :contentReference[oaicite:10]{index=10} |

---

## 2) Security Boundaries

- **Inbound (Library):** function inputs only; **no** config or I/O side-effects. **Inbound (CLI):** explicit files/stdin and flags/env (feature-gated). :contentReference[oaicite:11]{index=11}  
- **Outbound (Library):** canonical bytes; `b3:<hex>` addressing string checks; **no** network. **Outbound (CLI):** stdout/file outputs; **no** network attempts (tested). :contentReference[oaicite:12]{index=12}  
- **Downstream dependents:** `svc-index` for resolution, `svc-storage` for content, `svc-dht` for discovery; runtime quotas/readiness/capabilities enforced *there*, not here. :contentReference[oaicite:13]{index=13}  
- **Trust Zone:** same-tenant developer/ops usage; non-privileged by default (no keys stored).  
- **Assumptions (canon):** 33-crate canon, overlay/DHT split, `tldctl` folded into `ron-naming`. :contentReference[oaicite:14]{index=14}

---

## 3) Key & Credential Handling

- **Custody:** The library **stores no keys**. Signing/verification routes through small traits; backing implementations can delegate to `ron-kms`/HSM. :contentReference[oaicite:15]{index=15}  
- **CLI usage:** `tldctl` may **reference** key IDs/profiles and request detached signatures from an external provider; inputs/outputs are explicit paths or stdio. (No network in default profile.) :contentReference[oaicite:16]{index=16} :contentReference[oaicite:17]{index=17}  
- **PQ stance:** Optional `pq` feature selects PQ-capable verifiers (e.g., Dilithium) via the verifier abstraction; hybrid mode must be explicitly enabled and supported by the backend. :contentReference[oaicite:18]{index=18}

---

## 4) Hardening Requirements (Applicability-Tagged)

> These are **obligations**, not a completion checklist. Apply “Lib” or “CLI” as indicated.

- **Lib:** `#![forbid(unsafe_code)]`, deny `unwrap/expect` in clippy; deterministic normalization; strict DTOs with `#[serde(deny_unknown_fields)]`; address format guard (`b3:<64-hex>`). :contentReference[oaicite:19]{index=19} :contentReference[oaicite:20]{index=20} :contentReference[oaicite:21]{index=21}  
- **Lib:** Size limits on encoded artifacts documented so they remain **well below** OAP/1 `max_frame = 1 MiB`. :contentReference[oaicite:22]{index=22}  
- **CLI:** One-shot execution; **no network**; I/O restricted to stdin/stdout/explicit paths; exit codes for usage/validation/signature failures; unknown config keys rejected. :contentReference[oaicite:23]{index=23} :contentReference[oaicite:24]{index=24}  
- **CLI:** Configuration precedence (flags > env > file > defaults); dedicated `TLDCTL_*` env names. :contentReference[oaicite:25]{index=25}  
- **Both:** Unicode/IDNA table versions **pinned**; updates require vectors and CI gates. :contentReference[oaicite:26]{index=26}

---

## 5) Observability for Security (when embedded)

- **Metrics (suggested names):**  
  - `naming_manifest_parse_errors_total{reason}`  
  - `naming_manifest_validation_seconds` (histogram)  
  These are emitted by hosts that embed the library or wrap the CLI; `ron-naming` itself does not run a server.  
- **Logs:** structured JSON lines with `service`, `reason`, `corr_id`, and when relevant `content_id`.  
- **Readiness:** Resolution/serving readiness lives in `svc-index`/`svc-storage`; apply hardening defaults there (timeouts 5s, 512 inflight, 1 MiB, decompress ≤10×). :contentReference[oaicite:27]{index=27}

---

## 6) Dependencies & Supply Chain

- **Security-sensitive deps:** `serde` (strict), `blake3` (address checking), `hex` (encoding), optional arg-parsing/logging only under `cli`. (Enabling `cli` must not raise MSRV or violate workspace pins.) :contentReference[oaicite:28]{index=28}  
- **Pinning & policy:** versions pinned at workspace root; `cargo-deny` and SBOM in CI. (See Full Project Blueprint hardening defaults & CI.) :contentReference[oaicite:29]{index=29}

---

## 7) Formal, Property & Destructive Validation

- **Property tests (Lib):** idempotence (`normalize(normalize(x)) == normalize(x)`), JSON/CBOR round-trip, address hygiene near-miss rejects, TldMap digest invariants. :contentReference[oaicite:30]{index=30}  
- **Fuzz (Lib):** arbitrary Unicode into normalization — no panics, typed errors, policy respected. :contentReference[oaicite:31]{index=31}  
- **CLI conformance:** golden tests for `lint/pack/sign/verify/show`; assert **no network** attempted; canonical bytes stable across runs. :contentReference[oaicite:32]{index=32}  
- **Loom:** N/A today (sync lib); minimal models retained only for future single-writer init patterns. :contentReference[oaicite:33]{index=33}

---

## 8) Security Contacts

- **Maintainer:** Stevan White  
- **Disclosure policy:** Use the repo-root security process (GitHub Security Advisories).  

---

## 9) Migration & Upgrades

- **Schema evolution:** any breaking change in wire structs or canonicalization rules requires a **major version bump** and a migration note with golden vectors.  
- **Table updates:** bumping Unicode/IDNA tables must land with vector updates and CI gates; `cli` feature must not raise MSRV. :contentReference[oaicite:34]{index=34} :contentReference[oaicite:35]{index=35}

---

## 10) Mermaid — Security Flow Diagram

```mermaid
flowchart LR
  subgraph Dev/Ops Workstation
    A[tldctl (offline)] -->|lint/pack/sign/verify| B(ron-naming lib)
  end
  B -->|canonical bytes & b3:<hex>| C[Artifact store (svc-storage)]
  B -.->|reject invalid schema/addr| E[Error + typed reason]
  D[svc-index (runtime)] -->|resolve name→manifest| C
  style B fill:#b91c1c,stroke:#7f1d1d,color:#fff
````

---

### Appendix — Canon Hooks (where these constraints come from)

* `tldctl` is folded into `ron-naming`; runtime lookups remain in `svc-index`. 
* Pillar 9: library/runtime split (naming vs index); Six Concerns mapping: **SEC, GOV**. 
* Hardening defaults for services (timeouts, caps, UDS perms) are global; libraries adhere via DTO hygiene and size bounds.  

```


---

## TESTS.MD
_File 13 of 13_



---

# 🧪 TESTS.md — ron-naming

*Audience: developers, auditors, CI maintainers*
*msrv: 1.80.0 (no async runtime required; Loom generally N/A)*

---

## 0) Purpose

Define the **test contract** for `ron-naming` (Pillar 9 library for schemas, normalization, canonical encodings) and its **offline CLI** `tldctl`.

Covers:

* Unit, integration (CLI & DTO round-trips), property, fuzz, soak (CLI), and performance.
* Explicit coverage goals & **Bronze → Silver → Gold** acceptance gates.
* Invocation commands for devs & CI, plus required corpora/baselines.

---

## 1) Test Taxonomy

### 1.1 Unit Tests

**Scope:** Pure functions and modules (fast, <100ms).
**Key areas:**

* `normalize::*` — ASCII fast-path and Unicode (NFKC + casefold) branches.
* `parse::{json,toml}` — DTO parse/serialize invariants.
* `encode::cbor` — canonical, deterministic bytestreams (no nondeterminism).
* DTO validation (label length ≤ 63 octets; FQDN ≤ 253 octets post-canonicalization).
* Error taxonomy (typed errors; no panics on malformed input).

**Location:** `src/**/*` with `#[cfg(test)]`.

**Run:**

```bash
cargo test -p ron-naming --lib -- --nocapture
```

---

### 1.2 Integration Tests

**Scope:** End-to-end crate surface and CLI behavior (`tests/*.rs`).
**Must include:**

* **API round-trip:** DTO → JSON/TOML → DTO (lossless) and DTO → CBOR → DTO; byte-for-byte canonical CBOR across runs.
* **Normalization invariants:** idempotence (`normalize(normalize(x)) == normalize(x)`), casefold + NFKC correctness on mixed-script corpora.
* **CLI streaming:** `stdin`→`stdout` pipeline correctness on newline-delimited corpora; return codes and stderr JSON on errors.
* **Boundary checks (anti-scope):** no network, no filesystem writes unless explicitly invoked by a test harness.
* **Config knobs that exist for CLI:** `--parallel` behavior is observationally identical to single-thread (order may differ but outputs must be set-equal if order-independent mode is used).

**Run:**

```bash
cargo test -p ron-naming --test '*'
```

---

### 1.3 Property-Based Tests

**Tooling:** `proptest` (preferred) or `quickcheck`.
**Targets & invariants:**

* **Normalization:**

  * *Idempotence:* `N(N(x)) == N(x)`
  * *Casefold stability:* `N(lower(x)) == N(x)` for Latin subset; constrained for full Unicode.
  * *No panics:* any valid UTF-8 string (and selected ill-formed sequences handled as errors) never panic.
* **DTO round-trip:**

  * JSON/TOML/CBOR round-trip equivalence (structural equality).
  * CBOR canonical form stable across process runs.
* **Label constraints:**

  * Reject labels >63 octets post-canonicalization; surface typed error codes.
* **Confusable handling (if present):**

  * Confusables mapping stays deterministic; “safe confusables” test vectors must not drift.

**Run:**

```bash
cargo test -p ron-naming --features proptest -- --nocapture
```

---

### 1.4 Fuzz Tests

**Tooling:** `cargo fuzz` (libFuzzer).
**Targets (minimally):**

* `normalize_fuzz` — arbitrary UTF-8 + selected invalid UTF-8 → must not panic; errors are typed.
* `json_parse_fuzz` — random JSON → parse→encode→parse stability or typed error.
* `toml_parse_fuzz` — random TOML → same invariants.
* `cbor_decode_fuzz` — random CBOR → decoder must not panic; reject gracefully.

**Corpus:**
Seed from `testing/corpora/`:

* `ascii_*.txt`, `unicode_mixed_*.txt`, `long_labels.txt`, `malformed_utf8.bin`, `confusables.txt`

**Acceptance:**
Nightly **≥1h** (Silver) / **≥4h** (Gold) with **zero crashes/UB**.

**Run (examples):**

```bash
cargo fuzz run normalize_fuzz -- -max_total_time=3600
cargo fuzz run json_parse_fuzz -- -max_total_time=3600
```

---

### 1.5 Chaos / Soak Tests (CLI-oriented)

`ron-naming` is not a service, so chaos focuses on **long-running CLI pipelines**:

**Scenarios:**

* 24h **soak**: `cat 10M_lines.txt | tldctl vectorize --stdin` (with/without `--parallel`).
  Validate stable RSS (no leak), steady throughput variance (<±5%), and 0 unexpected exits.
* **Size/Compression bombs:** extremely long labels and max-length FQDNs; ensure bounded memory and typed rejects.
* **Fault injection:** kill/restart `tldctl` mid-pipeline in a shell script; resume at next line (external concern).

**Acceptance:**

* 24h soak: **no leaks**, **no panics**, **no throughput collapse**.

---

### 1.6 Performance / Load Tests

Benchmarks live alongside tests but are **separate** from unit pass/fail:

* **Criterion** microbenches:

  * `bench_normalize_ascii`, `bench_normalize_unicode`
  * `bench_parse_json_small`, `bench_parse_toml_small`
  * `bench_cbor_encode_small`
* **CLI latency/throughput:** hyperfine scripts for single-op and batch via stdin; with and without `--parallel`.
* **Regression gates:** as specified in `PERFORMANCE.md` (CI baseline deltas; fail on >10–15% drift with variance band).

**Run:**

```bash
cargo bench -p ron-naming
hyperfine 'tldctl normalize example.org'
hyperfine --warmup 3 'cat testing/corpora/unicode_mixed_100k.txt | tldctl vectorize --stdin'
```

---

## 2) Coverage & Gates

### Coverage Tooling

* `grcov` (LLVM) **or** `cargo tarpaulin` for line/branch coverage (choose one in CI; grcov recommended on Linux x86_64).

### 2.1 Bronze (MVP)

* Unit + integration tests pass (Linux + macOS).
* Coverage **≥ 70%** lines.
* Fuzz harness builds; smoke fuzz run (≤5 min) executes.

### 2.2 Silver (Useful Substrate)

* Property tests on normalization and DTO round-trip.
* Nightly fuzz **≥ 1h** across all targets; zero crashes.
* Coverage **≥ 85%** lines, **≥ 75%** branches on normalization & parsing modules.
* CLI soak script present (smoke 30 min) and green.

### 2.3 Gold (Ops-Ready)

* Nightly fuzz **≥ 4h**, corpus minimized and tracked in repo.
* 24h CLI soak passes with **no RSS growth** and **<±5%** throughput variance.
* Coverage **≥ 90%** lines overall; critical modules **≥ 95%**.
* Performance regression tracked release-to-release; baselines updated with justification.

---

## 3) Invocation Examples

### 3.1 All tests (unit + integration)

```bash
cargo test -p ron-naming --all-targets -- --nocapture
```

### 3.2 Property tests only

```bash
cargo test -p ron-naming --features proptest --tests prop_ -- --nocapture
```

### 3.3 Fuzz targets

```bash
cargo fuzz run normalize_fuzz -- -max_total_time=60
cargo fuzz run json_parse_fuzz -- -max_total_time=60
```

### 3.4 Benches (Criterion) + CLI perf

```bash
cargo bench -p ron-naming
hyperfine --warmup 3 'tldctl normalize example.org'
hyperfine --warmup 3 'cat testing/corpora/100k.txt | tldctl vectorize --stdin'
```

### 3.5 Soak (example)

```bash
set -euo pipefail
seq 10000000 | awk '{print "name" $1 ".example"}' \
 | ./target/release/tldctl vectorize --stdin --parallel \
 | pv -l > /dev/null
```

---

## 4) Observability Hooks

* **Structured logs** (stderr) on failures with fields: `level`, `module`, `err_kind`, `offset`, `duration_ms`, `corpus_id`.
* **Correlation IDs**: tests may add `corr_id=<uuid>`; emitted on error paths for cross-test triage.
* **Artifacts**: CI uploads:

  * Fuzz crashes/minimized repros (if any).
  * Criterion JSON baselines + hyperfine CSVs.
  * Flamegraphs (on perf deltas).
  * Heaptrack reports (if alloc spike detected).

---

## 5) CI Enforcement

**Jobs (typical GitHub Actions matrix):**

* **Test:** `cargo test --workspace --all-targets`
* **Lint:** `cargo fmt -- --check` and `cargo clippy -- -D warnings`
* **Supply chain:** `cargo deny check advisories bans licenses sources`
* **Coverage:** grcov (Linux x86_64), uploaded as artifact + badge
* **Fuzz (nightly):** `cargo fuzz run ... -max_total_time=3600` (Silver) / `14400` (Gold)
* **Perf (nightly):**

  * `cargo bench` → compare to baselines
  * Hyperfine scripts → compare CSV to baselines
  * Fail thresholds per `PERFORMANCE.md` (10–15% + variance band)

---

## 6) Open Questions (crate-specific)

* **Loom applicability:** N/A (no async or lock-heavy concurrency in the lib). If an internal queue appears later, define a minimal loom harness.
* **Mandatory fuzz set:** `normalize_fuzz`, `json_parse_fuzz`, `toml_parse_fuzz`, `cbor_decode_fuzz`. Add `confusables_fuzz` if confusable logic expands.
* **Perf SLOs:** Use `PERFORMANCE.md` CI baselines as **authoritative** (x86-64-v3 runners); local native numbers are informative only.
* **PQ/verify:** If a `verify` feature lands here (or in a sibling lib), add property/fuzz and benches for hybrid signatures; otherwise N/A.

---

## 7) Repository Layout (tests/perf/fuzz)

```
crates/ron-naming/
├─ src/
│  ├─ normalize/…              # unit tests inline
│  ├─ parse/{json,toml}.rs
│  └─ encode/cbor.rs
├─ tests/
│  ├─ integration_api.rs       # round-trips + invariants
│  ├─ integration_cli.rs       # stdin→stdout, errors
│  └─ prop_normalize.rs        # property tests (proptest)
├─ fuzz/
│  ├─ fuzz_targets/
│  │  ├─ normalize_fuzz.rs
│  │  ├─ json_parse_fuzz.rs
│  │  ├─ toml_parse_fuzz.rs
│  │  └─ cbor_decode_fuzz.rs
│  └─ Cargo.toml
├─ benches/
│  ├─ normalize_bench.rs
│  ├─ parsing_bench.rs
│  └─ encode_bench.rs
├─ testing/
│  ├─ corpora/
│  │  ├─ ascii_100k.txt
│  │  ├─ unicode_mixed_100k.txt
│  │  ├─ long_labels.txt
│  │  └─ malformed_utf8.bin
│  └─ performance/
│     └─ baselines/
│        ├─ criterion/*.json
│        └─ cli/*.csv
```

---

