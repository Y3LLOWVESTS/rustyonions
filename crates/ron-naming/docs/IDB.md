
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

