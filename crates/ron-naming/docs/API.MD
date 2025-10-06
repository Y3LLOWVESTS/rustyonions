

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
