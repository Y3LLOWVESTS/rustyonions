
---

````markdown
---
title: API Surface & SemVer Reference — ron-audit
status: draft
msrv: 1.80.0
last-updated: 2025-10-07
audience: contributors, auditors, API consumers
---

# API.md — ron-audit

## 0. Purpose

This document captures the **public API surface** of `ron-audit`:

- Snapshot of exported *functions, types, traits, modules* (Rust surface only; this is a library, not a service).
- SemVer discipline: what is **additive**, what is **breaking**, and which **feature flags** gate optional APIs.
- CI enforceability via `cargo public-api` and (optionally) `cargo semver-checks`.
- Canon alignment: all exports reflect the IDB invariants (BLAKE3 chaining, dedupe, bounds, checkpoints).

> `ron-audit` is **lib-only**. No HTTP/CLI endpoints are part of this crate’s API surface. Hosts (e.g., `svc-gateway`, `svc-storage`) wire the library and expose endpoints/metrics.

---

## 1. Public API Surface

Generate (at release and every PR):

```bash
cargo public-api -p ron-audit --simplified --deny-changes
````

### Current Surface (authoritative snapshot)

> Replace this block with the output of `cargo public-api` from your branch before merging.
> The list below is the **intended** stable surface for v1.x and should match the tool output.

```text
# re-exports (minimal, convenience only)
pub use ron_proto::audit::AuditRecord;

# core traits & errors
pub trait AuditSink
pub trait AuditStream
pub struct ChainState { pub head: String, pub seq: u64 }
pub enum AppendError { Full, Tamper, SizeExceeded, Io, Schema }

# canonicalization & hashing
pub mod canon {
    pub fn canonicalize_without_self_hash(rec: &AuditRecord) -> Result<Vec<u8>, CanonError>
    pub enum CanonError { NonCanonicalInput, FloatDisallowed, KeyOrder, Utf8Nfc }
}
pub mod hash {
    pub fn b3_no_self(rec: &AuditRecord) -> Result<String, CanonError>
    pub fn dedupe_key(rec: &AuditRecord) -> Result<[u8; 32], CanonError>
}

# verification helpers
pub mod verify {
    pub fn verify_record(rec: &AuditRecord) -> Result<(), VerifyError>
    pub fn verify_link(prev: &AuditRecord, next: &AuditRecord) -> Result<(), VerifyError>
    pub fn verify_chain<'a, I: IntoIterator<Item=&'a AuditRecord>>(iter: I) -> Result<(), VerifyError>
    pub enum VerifyError { HashMismatch, PrevMismatch, Canon(CanonError) }
}

# bounds & policy
pub mod bounds {
    #[derive(Clone, Copy)]
    pub struct Limits { pub max_attrs_bytes: usize, pub max_record_bytes: usize }
    pub fn check(rec: &AuditRecord, lim: Limits) -> Result<(), BoundsError>
    pub enum BoundsError { AttrsTooLarge, RecordTooLarge }
}

# feature: wal
#[cfg(feature = "wal")]
pub mod wal {
    pub struct WalSink { /* opaque */ }
    impl AuditSink for WalSink { /* ... */ }
    pub struct WalConfig { pub dir: std::path::PathBuf, pub rotate_bytes: u64, pub fsync: FsyncPolicy }
    pub enum FsyncPolicy { Always, OnCheckpoint, Never }
}

# feature: export
#[cfg(feature = "export")]
pub mod export {
    pub struct Manifest { /* opaque */ }
    pub struct Checkpoint {
        pub range: (u64, u64),
        pub root: String,           // "b3:<hex>"
        pub signer_key_id: String,
        pub alg: String,            // tag only; signing done by host
        pub sig: Vec<u8>,           // host-provided
    }
    pub trait ExportSink: Send + Sync {
        fn write_manifest(&self, m: &Manifest) -> Result<(), std::io::Error>;
        fn write_checkpoint(&self, c: &Checkpoint) -> Result<(), std::io::Error>;
    }
}

# optional helper for hosts (off by default)
#[cfg(feature = "cli")]
pub mod config {
    pub use ron_audit_config_types::*;
}
```

> **Notes**
>
> * We **intentionally** re-export `AuditRecord` from `ron-proto` for ergonomics. Its SemVer stability is tracked in §2 (re-export policy).
> * `WalSink`, `Manifest`, `Checkpoint` are **feature-gated** (`wal`, `export`) to minimize footprint for Micronode (amnesia) use.

---

## 2. SemVer Discipline

### 2.1 Additive (MINOR / Non-Breaking)

* New modules **behind a new feature flag** (default-off).
* New methods with default impls on traits if trait object safety is preserved.
* New `AppendError` / `VerifyError` **variants** only if marked `#[non_exhaustive]` (recommended).
* New helper fns in `canon`, `hash`, `verify`, `bounds`.

### 2.2 Breaking (MAJOR)

* Removing/renaming any exported item (fn/type/trait/mod).
* Changing function signatures, trait method sets, or object safety.
* Changing canonicalization semantics (key order, float policy, NFC) — this is **API by contract**; any change requires **major**.
* Changing `AuditRecord` re-export target or replacing it with a local type.
* Making previously `#[non_exhaustive]` enums exhaustive.
* Altering feature-flag defaults (e.g., enabling `wal` by default).

### 2.3 Patch-Level

* Doc fixes, internal perf changes that preserve observable behavior.
* Implementation changes that don’t affect types, signatures, or canonicalization.
* Tightened error messages/text without changing error **types**.

### 2.4 Re-Export Policy for `AuditRecord`

* We re-export `ron_proto::audit::AuditRecord` for convenience.

  * **Breaking**: structural changes in `AuditRecord` that are not backward-compatible (major bump in `ron-proto`) → require a **matching major** in `ron-audit`.
  * **Non-breaking**: additive fields under `#[serde(deny_unknown_fields)]` are **not** allowed; additions must go into `attrs`. Structural additions imply a major in `ron-proto`.

---

## 3. Stability Guarantees

* **MSRV**: `1.80.0` for all v1.x.
* `#![forbid(unsafe_code)]` — no public API depends on unsafe unless explicitly justified (requires owner sign-off).
* No leaking of internal implementation types (e.g., file handles, tokio types) through public signatures.
* Canonicalization is **law**: string NFC, fixed field order, no floats, deterministic JSON map order.
* Feature flags are **additive** and default-off; enabling an optional feature must not affect unrelated APIs.

---

## 4. Invariants (API-Visible Contracts)

* `AuditSink::append` is **append-only**; on success returns the new chain head (`self_hash` of `rec`).
* `AuditStream::state` exposes a **snapshot**; consumers must not infer global ordering beyond `(writer_id, stream, seq)`.
* `hash::b3_no_self` computes `"b3:<hex>"` over canonicalized bytes **excluding** `self_hash`.
* `hash::dedupe_key` is stable across versions for an identical canonical record; replays using the same key MUST be idempotent.
* `verify::*` never mutates inputs and returns **typed** errors (`VerifyError`).
* `bounds::check` enforces `attrs ≤ 1 KiB` and canonical record size `≤ 4 KiB` by default limits (callers may pass stricter).

---

## 5. Tooling

* **cargo public-api** — hard gate on surface drift.
* **cargo semver-checks** — optional, to catch subtle SemVer violations.
* **cargo doc** — public items must be documented (`#![deny(missing_docs)]` in lib).
* **API snapshots** stored at: `docs/api-history/ron-audit/<version>.txt` (exact `cargo public-api` output).

Commands:

```bash
cargo public-api -p ron-audit --simplified > docs/api-history/ron-audit/next.txt
git diff -- docs/api-history/ron-audit/  # review before bumping
```

---

## 6. CI & Gates

* **PR pipeline**:

  * `cargo public-api -p ron-audit --simplified --deny-changes`
  * `cargo doc -p ron-audit` with `#![deny(missing_docs)]`
  * (optional) `cargo semver-checks check-release`
* **CHANGELOG.md** rule:

  * If `cargo public-api` reports differences, the PR **must** include a CHANGELOG entry with **SemVer classification** (patch/minor/major) and rationale.
* **IDB binding**:

  * Any change affecting canonicalization or bounds **must** reference the IDB section and include updated *canonicalization test vectors*.

---

## 7. Acceptance Checklist (DoD)

* [ ] Current API snapshot generated and added to `docs/api-history/ron-audit/<new-version>.txt`.
* [ ] `cargo public-api` gate passes (or intentional diffs explained + CHANGELOG updated).
* [ ] All public items documented and rustdoc builds clean.
* [ ] Feature-flag matrix compiled at least once (`--features wal`, `--features export`, combos).
* [ ] If `AuditRecord` shape changed (via `ron-proto`): coordinated **major** plan captured.

---

## 8. Appendix

### 8.1 References

* Rust SemVer: [https://doc.rust-lang.org/cargo/reference/semver.html](https://doc.rust-lang.org/cargo/reference/semver.html)
* cargo-public-api: [https://github.com/Enselic/cargo-public-api](https://github.com/Enselic/cargo-public-api)
* cargo-semver-checks: [https://github.com/obi1kenobi/cargo-semver-checks](https://github.com/obi1kenobi/cargo-semver-checks)

### 8.2 Perfection Gates (tie-in)

* **Gate G:** No undocumented public items.
* **Gate H:** Canon changes ⇒ major bump; vectors updated.
* **Gate J:** CHANGELOG alignment with `cargo public-api` diff.

### 8.3 History

* **v1.0.0 → v1.0.1:** Added bounds/canonicalization helpers; no surface break.
* **v1.0.1 → v1.0.2:** Clarified multi-writer ordering; added `verify_chain`; feature-gated `export::Checkpoint`.

```
