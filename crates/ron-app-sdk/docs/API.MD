

````markdown
---
title: API Surface & SemVer Reference
status: draft
msrv: 1.80.0
last-updated: 2025-10-16
audience: contributors, auditors, API consumers
---

# API.md — ron-app-sdk

## 0. Purpose

This document captures the **public API surface** of `ron-app-sdk`:

- Snapshot of exported functions, types, traits, modules.
- SemVer discipline: what changes break vs. extend.
- Behavioral stability: deadlines, retries, error taxonomy, and config defaults.
- CI-enforceable via `cargo public-api` and `cargo semver-checks`.
- The canonical “spec” for external consumers (apps using the SDK).

---

## 1. Public API Surface

> Generate with:
>
> ```bash
> cargo public-api -p ron-app-sdk --simplified --deny-changes
> ```
>
> - Commit the output under `docs/api-history/ron-app-sdk/<version>.txt`.
> - Keep the snapshot below in sync each release.

### 1.1 Proposed Surface (v0.1.0) — until first `cargo public-api` run

> This section reflects the **intended** public API we’ve locked in across the other blueprints. Replace with real output once the crate compiles.

```text
# Top-level types
pub struct RonAppSdk
pub struct SdkContext { pub profile: NodeProfile, pub amnesia: bool }

# Configuration (public, constructible)
pub struct SdkConfig
pub enum Transport { Tls, Tor }
pub enum Jitter { Full, None }
pub enum Redaction { Safe, None }
pub enum PqMode { Off, Hybrid }
pub struct Timeouts { pub connect: Duration, pub read: Duration, pub write: Duration }
pub struct RetryCfg { pub base: Duration, pub factor: f32, pub cap: Duration, pub max_attempts: u32, pub jitter: Jitter }
pub struct IdemCfg { pub enabled: bool, pub key_prefix: Option<String> }
pub struct CacheCfg { pub enabled: bool, pub max_entries: usize, pub ttl: Duration, pub verify_puts: bool }
pub struct TracingCfg { pub propagate: bool, pub redaction: Redaction }
pub struct TorCfg { pub socks_addr: String }

# Errors (stable, non-exhaustive)
#[non_exhaustive]
pub enum SdkError {
  DeadlineExceeded,
  Transport(std::io::ErrorKind),
  Tls,
  TorUnavailable,
  OapViolation { reason: &'static str },
  CapabilityExpired,
  CapabilityDenied,
  SchemaViolation { path: String, detail: String },
  NotFound,
  Conflict,
  RateLimited { retry_after: Option<std::time::Duration> },
  Server(u16),
  Unknown(String),
}

# Client facade (async)
impl RonAppSdk {
  pub async fn new(cfg: SdkConfig) -> Result<RonAppSdk, SdkError>;
  pub fn context(&self) -> SdkContext;

  // Mailbox plane
  pub async fn mailbox_send(&self, cap: Capability, msg: Mail, deadline: Duration, idem: Option<IdemKey>) -> Result<Receipt, SdkError>;
  pub async fn mailbox_recv(&self, cap: Capability, deadline: Duration) -> Result<Vec<Mail>, SdkError>;
  pub async fn mailbox_ack(&self, cap: Capability, ack: Ack, deadline: Duration) -> Result<(), SdkError>;

  // Edge plane (byte-range friendly)
  pub async fn edge_get(&self, cap: Capability, path: &str, range: Option<ByteRange>, deadline: Duration) -> Result<bytes::Bytes, SdkError>;

  // Storage plane (content addressed)
  pub async fn storage_get(&self, cap: Capability, addr_b3_hex: &str, deadline: Duration) -> Result<bytes::Bytes, SdkError>;
  pub async fn storage_put(&self, cap: Capability, blob: bytes::Bytes, deadline: Duration, idem: Option<IdemKey>) -> Result<AddrB3, SdkError>;

  // Index plane
  pub async fn index_resolve(&self, cap: Capability, key: &IndexKey, deadline: Duration) -> Result<AddrB3, SdkError>;
}

# Config helpers
impl SdkConfig {
  pub fn from_env() -> anyhow::Result<SdkConfig>;
  pub fn validate(&self) -> anyhow::Result<()>;
  pub fn with_overrides<F: FnOnce(&mut SdkConfig)>(self, f: F) -> SdkConfig;
}

# Context & enums
pub enum NodeProfile { Micronode, Macronode }

# Re-exports (facade to canon)
pub use ron_proto::{Capability, IdemKey, Mail, Ack, Receipt, IndexKey, AddrB3, ByteRange};
````

> **Feature gating (public surface impact):**
>
> * `tor` feature exposes `Transport::Tor` and `TorCfg` (present but no-ops if disabled).
> * `metrics` feature adds a `metrics()` getter returning an opaque handle (not required for core use).
> * `cli` (if any example/bin exists) is **out of library surface** and documented separately.

---

## 2. SemVer Discipline

### 2.1 Additive (Minor / Non-Breaking)

* New **methods** on `RonAppSdk` that don’t change existing signatures.
* New **variants** on `SdkError` (enum is `#[non_exhaustive]`).
* New **fields** on config structs **if** they derive `Default` and are optional or have safe defaults.
* New re-exports (e.g., additional DTOs from `ron-proto`) so long as names don’t shadow existing exports.

### 2.2 Breaking (Major)

* Removing or renaming any public item (types, functions, re-exports).
* Changing function **signatures** (params, return types) or trait bounds.
* Making an enum exhaustive (removing `#[non_exhaustive]`) or retyping variants.
* Tightening visibility (e.g., pub→crate) of anything already public.
* Changing config defaults in a way that meaningfully alters **behavioral contracts** (deadline policy, retry caps) without a coordinated migration.

### 2.3 Patch-Level

* Documentation improvements, typos, examples.
* Performance improvements that preserve behavioral semantics.
* Internal refactors, private module layout, dependency bumps with identical API.

---

## 3. Stability Guarantees

* **MSRV**: `1.80.0` (bumps are **minor** and called out in CHANGELOG).
* **No `unsafe`** in public API (any `unsafe` usage requires justification and audit).
* **Error taxonomy** is stable: string matching is discouraged; use enum variants.
* **Transport neutrality**: public API does not expose transport internals (no `TcpStream`/`TlsStream` in signatures).
* **Deadlines present** across all network methods; no unbounded I/O paths are exposed.

---

## 4. Invariants (API-Level)

* `RonAppSdk` methods are **async** and accept a **deadline** (or derive one from config) — this is part of the contract.
* **Idempotency**: all mutating methods accept an optional `IdemKey`; the SDK auto-generates if `idempotency.enabled`.
* **One SDK, two profiles**: behavior at the SDK surface is identical for Micronode/Macronode; profile is available via `SdkContext` only (metadata).
* **DTO hygiene**: all public DTOs come from `ron-proto`; unknown fields are rejected internally, not surfaced as successful results.
* **Content addressing**: storage APIs operate on/return `AddrB3` (BLAKE3) values.

---

## 5. Behavioral Compatibility

> These are **not** symbol changes but still observable by consumers; we document and version them carefully.

* **Retry policy defaults** (base/factor/cap/max_attempts) — changing them is **behavioral**; treat as **minor** only with explicit release notes.
* **Timeout defaults** — same rule as retries.
* **Redaction defaults** — must remain `Safe`; changing to `None` would be breaking for compliance.
* **Transport defaults** — `Transport::Tls` remains default; adding Tor has **no effect** unless selected.

---

## 6. Tooling

* `cargo public-api` — detect surface diffs:

  ```bash
  cargo public-api -p ron-app-sdk --simplified --deny-changes
  ```
* `cargo semver-checks` — enforce SemVer adherence:

  ```bash
  cargo semver-checks check-release -p ron-app-sdk
  ```
* `cargo doc --no-deps -p ron-app-sdk` — docs CI; **all public items must be documented** (`#![deny(missing_docs)]` suggested).

---

## 7. CI & Gates

* **PR gate** runs:

  * `cargo public-api` (diff posted in PR comment; job fails on unacknowledged changes).
  * `cargo semver-checks` (fails on breaking changes without a major bump).
* **CHANGELOG coupling**:

  * If a surface diff exists, a CHANGELOG entry is **mandatory**.
  * Major changes must include **migration notes** and examples.
* **Snapshot archival**:

  * On publish, write `docs/api-history/ron-app-sdk/<version>.txt` from `cargo public-api`.

---

## 8. Acceptance Checklist (DoD)

* [ ] Current API snapshot generated & stored under `docs/api-history/`.
* [ ] `cargo public-api` passes in CI (no unexpected diffs).
* [ ] `cargo semver-checks` passes or change is marked **breaking** with major bump.
* [ ] CHANGELOG updated (surface + behavioral notes).
* [ ] Rustdoc examples compile (`cargo test --doc`).
* [ ] New/changed APIs have observability coverage (spans/metrics) in examples/tests.

---

## 9. Appendix

### 9.1 References

* Rust SemVer: [https://doc.rust-lang.org/cargo/reference/semver.html](https://doc.rust-lang.org/cargo/reference/semver.html)
* cargo-public-api: [https://github.com/Enselic/cargo-public-api](https://github.com/Enselic/cargo-public-api)
* cargo-semver-checks: [https://github.com/obi1kenobi/cargo-semver-checks](https://github.com/obi1kenobi/cargo-semver-checks)

### 9.2 Perfection Gates tie-in

* **Gate G**: No undocumented public items (docs must exist).
* **Gate H**: Breaking changes → major version bump + migration.
* **Gate J**: CHANGELOG + API snapshot alignment enforced.

### 9.3 API Evolution Playbook

* **Add a new plane method?**

  * Prefer adding on `RonAppSdk` with a clear `endpoint` name and deadline param.
  * Add spans/metrics; document examples; minor bump.

* **Extend `SdkError`?**

  * Add a new variant (enum is non-exhaustive); minor bump; update mapping table in `SECURITY.md`.

* **Add config?**

  * Add struct field with safe default; minor bump; update `CONFIG.md` table.

* **Deprecate a method?**

  * Keep symbol; add `#[deprecated(note = "...", since = "x.y.z")]`; provide a replacement; remove in next **major**.

### 9.4 History

* *v0.1.0* — Initial surface with async client facade, strict DTOs, stable error taxonomy, and config builders.

```

