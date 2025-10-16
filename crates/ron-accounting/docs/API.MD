

````markdown
---
title: API Surface & SemVer Reference — ron-accounting
status: draft
msrv: 1.80.0
last-updated: 2025-10-15
audience: contributors, auditors, API consumers
---

# API.md — ron-accounting

## 0. Purpose

This document captures the **public API surface** of the crate and how we manage it:

- Snapshot of exported functions, types, traits, modules (what external crates see).
- **SemVer discipline**: what is additive vs breaking vs patch.
- Alignment with `CHANGELOG.md` (behavioral vs surface changes).
- CI-enforceable via `cargo public-api` and `cargo-semver-checks`.
- Acts as the **spec** for integrators embedding `ron-accounting`.

> ron-accounting is a **library** first. Optional adapters (HTTP/UDS, OTEL) are exposed behind features. The core keeps a **small, stable** surface focused on recording usage, sealing time-sliced snapshots, and **ordered + idempotent** export.

---

## 1. Public API Surface

> Generated in CI with:
>
> ```bash
> cargo public-api --simplified --deny-changes -p ron-accounting
> ```
>
> Below is the **intended stable surface**. Treat it as a contract; changes require a SemVer-aware PR.

### 1.1 Top-level Modules (re-exported)

```text
pub mod accounting     // hot-path recording & sealing
pub mod exporter       // Exporter trait + reference exporters (feature-gated impls)
pub mod wal            // bounded write-ahead logging (feature "wal")
pub mod config         // Config structs & validation helpers
pub mod metrics        // Metrics handles (registration-by-host pattern)
pub mod readiness      // Readiness snapshot & keys
pub mod errors         // Error/Result taxonomy (#[non_exhaustive] enums)
````

### 1.2 Core Types & Traits (stable)

```text
// accounting
pub struct Recorder;                      // low-overhead counters entry-point
pub struct Window;                        // time-slice view (UTC-aligned)
pub struct SealedSlice { /* opaque */ }   // immutable sealed slice + digest
pub struct SliceId { pub tenant: TenantId, pub dimension: Dimension, pub seq: u64 } // stable fields
pub struct Row { pub key: AccountKey, pub inc: u64 }                                // saturating adds
pub struct AccountKey { pub ns: Namespace, pub id: u128 }
pub type TenantId = u128;
#[non_exhaustive]
pub enum Dimension { Bytes, Requests, Cpu } // extensible; new variants minor
pub struct RolloverHandle;                 // host-triggered/manual sealing
pub struct AckLru;                         // idempotence cache (opaque newtype)

// exporter
pub trait Exporter: Send + Sync {
    /// Put a sealed slice (idempotent). Must return Duplicate on dup.
    fn put<'a>(&'a self, slice: &'a SealedSlice) -> futures::future::BoxFuture<'a, Result<Ack, Error>>;
}
#[non_exhaustive]
pub enum Ack { Ok, Duplicate }
pub struct ExporterRouter;                 // per-stream dispatch (opaque)
pub struct ExportReport { pub status: Ack, pub latency: std::time::Duration } // non-breaking to extend

// wal (feature = "wal")
pub struct Wal;                            // bounded WAL handle
pub struct WalConfig { pub dir: std::path::PathBuf, pub max_bytes: u64, pub max_entries: u64 }
pub struct WalStats { pub size_bytes: u64, pub entries: u64 }

// config
pub struct Config;                         // see docs/CONFIG.md (serde types)
pub fn validate(cfg: &Config) -> anyhow::Result<()>; // fail-closed rules

// metrics
pub struct Metrics { /* clones of prometheus handles */ }
impl Metrics { pub fn new(reg: &prometheus::Registry) -> Self; }

// readiness
#[non_exhaustive]
pub enum ReadyKey { ConfigLoaded, QueuesBoundedOk, ExporterOk, WalOk, BoundaryTickerOk }
pub struct Readiness;
impl Readiness { pub fn snapshot(&self) -> (bool, smallvec::SmallVec<[ReadyKey; 4]>); }

// errors
pub type Result<T> = std::result::Result<T, Error>;
#[non_exhaustive]
pub enum Error {
    Busy,                  // hot-path backpressure (try_send)
    OrderOverflow,         // lane cap hit while holding order
    Timeout,               // I/O or op deadline exceeded
    DuplicateExport,       // downstream idempotent dup
    PersistenceFull,       // WAL quotas exceeded
    WalCorrupt,            // checksum/parse failure (replay)
    SchemaViolation,       // DTO strictness (adapter/marshal)
    Io(std::io::Error),    // bubbled up I/O
    Other(String),         // fallback; avoid matching on message
}
```

### 1.3 Feature-Gated APIs

```text
# feature = "http-adapter"
pub mod http {
    pub struct Server;                       // axum/hyper wrapper
    pub struct BindAddrs { pub bind: std::net::SocketAddr, pub metrics: std::net::SocketAddr }
    pub fn run(server: Server) -> impl Future<Output = anyhow::Result<()>>;
    // Endpoints: /export, /metrics, /readyz, /healthz (documented in OBSERVABILITY.md)
}

# feature = "wal"
pub use wal::{Wal, WalConfig, WalStats};

# feature = "otel"
pub mod otel {
    pub fn init_tracer(service_name: &str) -> anyhow::Result<()>;
}
```

> **Rationale:** Feature flags keep the **core** API minimal and stable. Adapters and telemetry are optional and may evolve faster without breaking core consumers.

---

## 2. SemVer Discipline

### 2.1 Additive (Minor / Non-Breaking)

* Adding **new** free functions, trait impls, modules, or `struct` fields **behind `#[non_exhaustive]`** (or with defaulted builders).
* Adding **enum variants** to `#[non_exhaustive]` enums (`Dimension`, `Ack`, `ReadyKey`, `Error`).
* Adding methods that don’t change existing behavior.
* Adding new features (e.g., `otel`) that are **off by default**.

### 2.2 Breaking (Major)

* Removing, renaming, or moving **public items**.
* Changing function signatures or trait bounds of `Exporter::put`.
* Making formerly `#[non_exhaustive]` enums **exhaustive**, or re-ordering stable fields.
* Changing `Error` discriminants that external code may match without `#[non_exhaustive]`.
* Exposing new required type parameters on public types.

### 2.3 Patch-Level

* Documentation, typos, internal perf changes with identical surface and semantics.
* Extending `Error::Other(String)` messages (non-contractual text).
* Tightening internal validation while keeping same public error **variant**.

> **Behavior vs Surface:** If the public **shape** is unchanged but behavior is meaningfully different (e.g., stricter ordering checks), log it under `CHANGELOG.md` **Behavior** section with migration notes.

---

## 3. Stability Guarantees

* **MSRV:** `1.80.0` (pinned; raising MSRV is a **minor** bump at minimum).
* **Unsafe Rust:** forbidden unless justified and code-reviewed with security tag.
* **No internal leaks:** avoid exposing `tokio` types directly unless intentional (prefer opaque newtypes).
* **Non-exhaustive enums** for all externally matched enums.
* **Opaque internals:** `Recorder`, `SealedSlice`, `ExporterRouter`, `RolloverHandle`, `AckLru` keep fields private to permit optimizations.

---

## 4. Invariants (API-level)

* **Core remains library-first:** The only required trait for integration is `Exporter`; everything else can be embedded without running the HTTP adapter.
* **Idempotence is explicit:** `Ack::{Ok,Duplicate}` is the *only* acknowledged success states for `Exporter::put`.
* **Ordering is implicit in types:** `SliceId.seq: u64` is **monotone** per `(tenant, dimension)`; helpers never expose mutation of `seq`.
* **Config is parsed & validated in-crate:** `config::validate` is the fail-closed gate used by adapters and hosts.
* **Errors are typed:** Catch-all stringy errors stay in `Error::Other`; do not proliferate variants without strong reason.

---

## 5. Tooling

* **cargo public-api** — detects surface diffs.

  * Command: `cargo public-api --simplified --deny-changes -p ron-accounting`
* **cargo semver-checks** — static SemVer compliance (optional but recommended).

  * Command: `cargo semver-checks check-release -p ron-accounting`
* **cargo doc** — published docs, `#![deny(missing_docs)]`.
* **API snapshots** — store text outputs under:

  * `docs/api-history/ron-accounting/<crate-version>.txt`

---

## 6. CI & Gates

* On every PR:

  * Run `cargo public-api` and post the diff as a PR comment.
  * Fail the job if changes are detected **without**:

    * a matching `CHANGELOG.md` entry, **and**
    * an explicit “SemVer: {patch|minor|major}” tag in the PR description.
  * Optionally run `cargo semver-checks` to flag accidental breakage.
* On release:

  * Generate and commit `docs/api-history/.../<version>.txt` snapshot.
  * Ensure `Cargo.toml` version bump aligns with the **kind** of changes.

**Suggested GitHub Actions step (sketch):**

```yaml
- name: Public API diff
  run: |
    cargo install cargo-public-api || true
    cargo public-api --simplified -p ron-accounting > public-api.txt
    git diff --exit-code -- public-api.txt || echo "::warning file=docs/API.md::Public API changed; confirm SemVer & CHANGELOG"
```

---

## 7. Acceptance Checklist (Definition of Done)

* [ ] Current API snapshot generated & stored in `docs/api-history`.
* [ ] SemVer assessment documented in PR (`SemVer: ...`).
* [ ] CI gate passes (`cargo public-api` and, if enabled, `cargo semver-checks`).
* [ ] `CHANGELOG.md` updated for any surface or behavioral change.
* [ ] Docs/tests/examples updated (including `README.md` fragments).
* [ ] **Adapters:** endpoints documented in `docs/OBSERVABILITY.md` + security notes in `docs/SECURITY.md`.

---

## 8. Appendix

### 8.1 Minimal usage examples (stable patterns)

```rust
use ron_accounting::{accounting::Recorder, exporter::{Exporter, Ack}, Result};

async fn export_one<E: Exporter>(rec: &Recorder, exp: &E) -> Result<()> {
    let slice = rec.seal_now()?;                // SealedSlice (opaque)
    let ack = exp.put(&slice).await?;           // Ack::Ok or Ack::Duplicate
    match ack {
        Ack::Ok | Ack::Duplicate => Ok(()),
        // Ack is #[non_exhaustive]; new success states won't break this match.
    }
}
```

```rust
use ron_accounting::{config::Config, config::validate};

fn load_cfg(cfg: &Config) -> anyhow::Result<()> {
    validate(cfg) // Fail-closed; maps to Error variants at adapter boundary
}
```

### 8.2 SemVer annotations we rely on

```rust
#[non_exhaustive]
pub enum Error { /* see §1.2 */ }

#[non_exhaustive]
pub enum Dimension { Bytes, Requests, Cpu /* future-ready: e.g., Gpu, Tokens */ }

#[non_exhaustive]
pub enum ReadyKey { ConfigLoaded, QueuesBoundedOk, ExporterOk, WalOk, BoundaryTickerOk }
```

### 8.3 Planned evolution (roadmap, non-binding)

* Add `Dimension::Tokens` (minor).
* Add `Exporter::put_batch(&[&SealedSlice])` (minor, new method on trait via extension trait to avoid breaking).
* Add `Recorder::seal_at(utc_boundary: std::time::SystemTime)` (minor).
* Add `Error::WalUnavailable` (minor; enum is non-exhaustive).
* Introduce builder types `RecorderBuilder`, `ExporterRouterBuilder` (minor).

> **No planned breaking** changes to existing trait signatures; batching will be delivered via **extension traits** to preserve `Exporter` stability.

### 8.4 References

* Rust SemVer: [https://doc.rust-lang.org/cargo/reference/semver.html](https://doc.rust-lang.org/cargo/reference/semver.html)
* cargo-public-api: [https://github.com/Enselic/cargo-public-api](https://github.com/Enselic/cargo-public-api)
* cargo-semver-checks: [https://github.com/obi1kenobi/cargo-semver-checks](https://github.com/obi1kenobi/cargo-semver-checks)

---

## 9. Current Public API Snapshot (to be auto-generated in CI)

> Replace this fenced block on release with the actual `cargo public-api` output.
> Until then, it serves as a human-curated target surface.

```text
pub mod accounting
pub mod exporter
pub mod wal
pub mod config
pub mod metrics
pub mod readiness
pub mod errors

pub struct accounting::Recorder
pub struct accounting::Window
pub struct accounting::SealedSlice
pub struct accounting::SliceId
pub struct accounting::Row
pub struct accounting::AccountKey
pub type  accounting::TenantId = u128
pub enum  accounting::Dimension

pub trait exporter::Exporter
pub enum  exporter::Ack
pub struct exporter::ExporterRouter
pub struct exporter::ExportReport

pub struct wal::Wal
pub struct wal::WalConfig
pub struct wal::WalStats

pub struct config::Config
pub fn    config::validate(cfg: &Config) -> anyhow::Result<()>

pub struct metrics::Metrics
pub fn    metrics::Metrics::new(reg: &prometheus::Registry) -> metrics::Metrics

pub struct readiness::Readiness
pub enum  readiness::ReadyKey
pub fn    readiness::Readiness::snapshot(&self) -> (bool, smallvec::SmallVec<[ReadyKey; 4]>)

pub enum  errors::Error
pub type  errors::Result<T>
```

---

✅ **Why this matters:**

* Keeps the surface **small, stable, and auditable**.
* Makes SemVer **mechanical** with CI.
* Leaves room to optimize internals (hot-path atomics, sharding, WAL details) **without** breaking consumers.
* Aligns tightly with our IDB: typed errors, explicit idempotence, ordered exports, and library-first ergonomics.

```
```
