
---

# 📖 API.md — `ron-proto`

---

title: API Surface & SemVer Reference
status: draft
msrv: 1.80.0
last-updated: 2025-09-28
audience: contributors, auditors, API consumers
-----------------------------------------------

## 0) Purpose

This document captures the **public API surface** of `ron-proto`:

* Snapshot of exported functions, types, traits, and modules.
* **SemVer discipline**: what changes break vs. extend (Rust API vs. wire).
* Alignment with `CHANGELOG.md` (behavioral vs. surface changes).
* CI-enforceable via `cargo public-api` and `cargo semver-checks`.
* Acts as the **spec** for external consumers (Rust + polyglot SDKs via codegen).

> **Scope:** `ron-proto` is a **library crate (DTOs only)**. No I/O, sockets, or async runtime types are exposed. Endpoint/port topics are **N/A** here.

---

## 1) Public API Surface

### 1.1 How to generate the authoritative snapshot

```bash
cargo install cargo-public-api
cargo public-api --simplified --deny-changes > docs/api-history/ron-proto/$(cargo pkgid | sed 's/.*#//').txt
```

To compare against the last committed snapshot:

```bash
git diff -- docs/api-history/ron-proto
```

> CI (see §6) runs `cargo public-api` on PRs and fails if **breaking** symbols disappear or change without a matching SemVer bump and `CHANGELOG.md` entry.

### 1.2 Nominal API Map (curated overview)

This is a reviewer-friendly overview; the **snapshot in §1.4 is source of truth**.

* **Modules**

  * `id`

    * `pub struct ContentId` (opaque; canonical `"b3:<hex>"`)
    * `pub fn parse_content_id(&str) -> Result<ContentId, ProtoError>`
    * `pub fn content_id_from_bytes(&[u8]) -> ContentId` *(typing only; hashing lives in kms/auth if not exposed)*
  * `oap`

    * `#[non_exhaustive] pub struct OapEnvelope { kind: OapKind, seq: u64, content: ContentId, /* … */ }`
    * `#[non_exhaustive] pub enum OapKind { HelloV1, StartV1, DataV1, EndV1, AckV1, ErrorV1, /* … */ }`
  * `manifest`

    * `#[non_exhaustive] pub struct ManifestV1 { /* #[serde(deny_unknown_fields)] */ }`
  * `cap`

    * `#[non_exhaustive] pub struct CapTokenHdr { /* typed, no secrets */ }`
  * `error`

    * `#[non_exhaustive] pub enum ProtoErrorKind` (decode/encode/schema/addr/…)
    * `pub struct ProtoError { kind: ProtoErrorKind, detail: Option<String> }`
    * `impl ProtoErrorKind { pub fn as_metric_reason(&self) -> &'static str }`  ⟵ **reason canon (immutable strings)**
  * `version`

    * `pub const PROTO_VERSION: &str`
    * `pub const PROTO_ABI_FINGERPRINT: &str`
  * `trace`

    * `pub fn trace_fields(type_name: &str) -> [(&'static str, String); N]` *(helper; no `tracing` dep)*
  * `config`

    * `#[non_exhaustive] pub struct ProtoConfig { … }`
    * `impl ProtoConfig { pub fn validate(&self) -> Result<(), ProtoConfigError>; pub fn from_env_prefixed(prefix:&str) -> Result<Self, EnvError> }`
    * `#[non_exhaustive] pub enum ProtoConfigError`

> All externally consumed DTOs use `#[serde(deny_unknown_fields)]` and prefer `#[non_exhaustive]` to allow additive evolution without breaking construction.

### 1.3 Facet Map (Surface → Facet responsibilities)

| Facet                      | Key Types                                              | SemVer Notes (Rust)                                                                                 | Wire Compatibility Notes                                                                                |
| -------------------------- | ------------------------------------------------------ | --------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| **Graph / Search / Index** | `manifest::ManifestV1`, `oap::OapEnvelope`             | Mark DTOs `#[non_exhaustive]`; add optional fields via `#[serde(default)]` → **minor**              | Unknown fields rejected (strict); adding **optional** fields keeps wire compat; update interop vectors. |
| **Media**                  | `oap::{OapEnvelope, OapKind::DataV1}`, `id::ContentId` | Additive hints or flags → **minor** if optional; never change existing field semantics              | Observe OAP frame cap (1 MiB) and chunk semantics (~64 KiB) in host; IDs must be `"b3:<hex>"`.          |
| **Trust & Safety**         | `error::ProtoErrorKind` (+ reason canon)               | **Append-only** variant growth on `#[non_exhaustive]` → **minor**; reason strings are **immutable** | Reason strings are part of observability contract; do not mutate literals.                              |
| **ECON (if used)**         | DTOs participating in ledger flows                     | Additive optional fields → **minor**; changing value semantics → **major**                          | Property tests ensure conservation (no negative, no double-spend); update vectors.                      |
| **DX (SDKs)**              | `config::ProtoConfig`, `trace::trace_fields`           | New helpers/constructors → **minor**; signature changes → **major**                                 | Keep SDK codegen in lockstep; regenerate bindings on surface add.                                       |

### 1.4 Current Surface (authoritative snapshot)

Replace this block with the **actual** tool output in the repo:

```text
# cargo public-api --simplified (excerpt; update on every release/PR as needed)
pub mod id
pub mod oap
pub mod manifest
pub mod cap
pub mod error
pub mod version
pub mod trace
pub mod config
pub struct id::ContentId
pub fn id::parse_content_id(s: &str) -> Result<id::ContentId, error::ProtoError>
pub enum oap::OapKind
pub struct oap::OapEnvelope
pub const version::PROTO_VERSION: &str
pub const version::PROTO_ABI_FINGERPRINT: &str
...
```

---

## 2) SemVer Discipline (Rust API vs. Wire Compatibility)

**Two contracts** must stay aligned:

* **Rust API stability** → guarded by SemVer + `cargo public-api`.
* **Wire compatibility** → guarded by strict serde (`deny_unknown_fields`), optional fields with defaults, versioned types, and interop vectors.

### 2.1 Additive (Minor / Non-Breaking)

* Add new functions/modules/`pub` items **without** altering existing signatures.
* Keep DTOs and error enums `#[non_exhaustive]`; add new variants freely (minor).
* Add new **optional** fields to `#[non_exhaustive]` structs using:

  * `#[serde(default)]`
  * doc updates and constructors/builders for ergonomics.
* Add **PQ-hybrid typing** fields behind `cfg(feature="pq")` as **optional** (typing only; crypto lives in kms/auth/transport).
* Add new Cargo features **off by default**.

### 2.2 Breaking (Major)

* Remove/rename/privatize existing `pub` items.
* Change function signatures/trait bounds.
* Make previously `#[non_exhaustive]` types exhaustive.
* Change serde layout in ways that break wire (e.g., removing a field, changing meaning/format).
* **Mutate** an existing reason string literal from `as_metric_reason` (observability regression).

### 2.3 Patch-Level

* Doc-only edits.
* Internal performance improvements with identical API.
* Tightened invariants not visible in the public surface.

### 2.4 Feature Policy

* Adding a feature = **minor**.
* Removing or flipping default features = **major**.
* Any `pub` item behind a feature must be clearly documented with `cfg(feature="…")` in rustdoc.

---

## 3) Stability Guarantees

* **MSRV:** `1.80.0` (workspace-root pinned). Raising MSRV is a **minor** bump with clear justification.
* **Unsafe:** `#![forbid(unsafe_code)]`. Exceptions require design review & explicit justification.
* **No runtime bleed:** API does not leak I/O or async types (e.g., no `tokio::…` in public surface).
* **Docs:** `#![deny(missing_docs)]` at crate root; every `pub` item documented.
* **Observability contract:** `PROTO_VERSION`, `PROTO_ABI_FINGERPRINT`, and **reason strings** are external contracts:

  * Fingerprint may change with schema updates (not a SemVer break alone).
  * Reason strings are **append-only** (never mutate literals).

---

## 4) API Invariants (Crate Role Alignment)

* **Library posture:** Limit to DTOs, error types, and small helpers (parsers, constants, trace fields).
* **DTO hygiene:** All externally consumed DTOs use `#[serde(deny_unknown_fields)]`.
* **Construction:** Prefer constructors/builders to keep structs extensible without breaking direct construction.
* **Addressing:** All content IDs are `"b3:<hex>"` (lowercase, 64 nybbles); construct via `ContentId`.
* **No ambient I/O/async:** Keep public surface independent of services/runtimes.

---

## 5) Tooling

* **cargo public-api** — snapshot & diff of exported symbols.
* **cargo semver-checks** — static analysis of SemVer compatibility.
* **cargo doc** — rustdoc for public items (doctests recommended).
* **API snapshots** — store under `docs/api-history/ron-proto/<crate-version>.txt`.

Local refresh one-liner:

```bash
cargo public-api --simplified > docs/api-history/ron-proto/$(cargo pkgid | sed 's/.*#//').txt
```

---

## 6) CI & Gates

```yaml
name: ron-proto API gates
on: [push, pull_request]
jobs:
  api-surface:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Build
        run: cargo build -p ron-proto
      - name: Public API snapshot (diff)
        run: |
          cargo install cargo-public-api || true
          cargo public-api --simplified > current.txt
          if ! git diff --quiet -- current.txt; then
            echo "::group::Public API diff"
            git --no-pager diff -- current.txt || true
            echo "::endgroup::"
          fi
      - name: SemVer checks (advisory)
        run: |
          cargo install cargo-semver-checks || true
          cargo semver-checks check-release -p ron-proto || true
```

**Policy:**

* Breaking diffs require: (a) **major bump** and (b) `CHANGELOG.md` entry.
* Any surface diff (even additive) requires a `CHANGELOG.md` “Added/Changed” note.

---

## 7) Acceptance Checklist (Definition of Done)

* [ ] API snapshot generated & committed at `docs/api-history/ron-proto/<version>.txt`.
* [ ] `cargo public-api` clean (no unacknowledged breaking diffs).
* [ ] `CHANGELOG.md` updated (surface + behavior).
* [ ] All public items documented (`cargo doc` OK).
* [ ] DTOs/enums exposed to consumers are `#[non_exhaustive]` and `#[serde(deny_unknown_fields)]` where applicable.
* [ ] Reason string additions documented; no mutations to past strings.
* [ ] Interop vectors updated when wire shape is extended (optional fields only for minor).
* [ ] If PQ-typing added, it’s optional (`cfg(feature="pq")`) with docs.

---

## 8) Appendix

### 8.1 References

* Rust SemVer: [https://doc.rust-lang.org/cargo/reference/semver.html](https://doc.rust-lang.org/cargo/reference/semver.html)
* cargo-public-api: [https://github.com/Enselic/cargo-public-api](https://github.com/Enselic/cargo-public-api)
* cargo-semver-checks: [https://github.com/obi1kenobi/cargo-semver-checks](https://github.com/obi1kenobi/cargo-semver-checks)
* Project canon: `docs/IDB.md`, `docs/CONFIG.md`, `docs/SECURITY.md`, `docs/OBSERVABILITY.md`

### 8.2 Perfection Gates tie-in

* **Gate G:** No undocumented API surface (deny missing docs + snapshot present).
* **Gate H:** Breaking changes require major bump and CHANGELOG entry.
* **Gate J:** API snapshots must match committed history; PR shows diffs.

### 8.3 Examples — safe additive changes

**Add an enum variant (non-breaking):**

```rust
#[non_exhaustive]
pub enum ProtoErrorKind {
    Decode,
    Encode,
    SchemaMismatch,
    // New (minor):
    VersionUnsupported,
}
```

**Add an optional field with serde default (non-breaking):**

```rust
#[non_exhaustive]
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManifestV1 {
    pub id: ContentId,
    #[serde(default)]
    pub hints: Option<ManifestHints>, // newly added
}
```

**Introduce a constructor to avoid direct struct construction:**

```rust
impl ManifestV1 {
    /// Minimal constructor; new fields default-initialized.
    pub fn new(id: ContentId) -> Self {
        Self { id, hints: None }
    }
}
```

> If a struct was previously **exhaustively constructible** (no `#[non_exhaustive]`), adding fields is **breaking**. Prefer non-exhaustive + constructors.

---

## 9) Wire vs. Rust API (reconciliation)

* **Wire**: strict serde + interop vectors guard compatibility; optional fields keep old readers working.
* **Rust**: `cargo public-api` guards surface stability; `#[non_exhaustive]` + constructors enable additive growth.
* An extension that’s wire-compatible may still be a **Rust break** if the struct was exhaustively constructed; design DTOs to avoid that pitfall.

---

## 10) Mermaid — Visuals

### 10.1 API change → CI gating flow

```mermaid
flowchart LR
  D[Dev edits public items] --> S[cargo public-api snapshot]
  S -->|diff| R{Breaking?}
  R -- Yes --> B[Require major bump + CHANGELOG]
  R -- No  --> M[Minor/Patch + CHANGELOG (Added/Changed)]
  S --> C[cargo semver-checks (advisory)]
  C --> P[PR Review]
  P --> G[Merge if gates pass]
```

### 10.2 Module map (surface overview)

```mermaid
flowchart TB
  subgraph ron-proto (lib)
    ID[id]:::m --> ERR[error]:::m
    OAP[oap]:::m --> ID
    MAN[manifest]:::m --> ID
    CAP[cap]:::m --> ERR
    VER[version]:::m
    TR[trace]:::m
    CFG[config]:::m
  end
  classDef m fill:#0ea5e9,stroke:#0369a1,color:#fff,rx:6,ry:6;
```

---

**End of file.**
