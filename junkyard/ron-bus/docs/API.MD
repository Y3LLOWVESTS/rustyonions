

**Path:** `crates/ron-bus/docs/API.md`

````markdown
---
title: API Surface & SemVer Reference — ron-bus
status: draft
msrv: 1.80.0
last-updated: 2025-09-27
audience: contributors, auditors, API consumers
---

# API.md — ron-bus

## 0. Purpose

This document captures the **public API surface** of `ron-bus` and the rules for changing it:

- Snapshot of exported types/functions/modules.
- **SemVer discipline** (what’s additive vs. breaking).
- CI enforcement (`cargo public-api`, optional `cargo semver-checks`).
- Aligns with `CHANGELOG.md` and the invariants in `docs/IDB.md`.

> Canon note: **monomorphic surface** — no public generics on `Bus` or its primary methods.

---

## 1. Public API Surface (Authoritative Spec)

Generated in CI with:

```bash
cargo public-api -p ron-bus --simplified
````

> Until code lands, this section defines the **intended** public symbols. Once implemented, CI must match this list (or the diff must be acknowledged via SemVer + CHANGELOG).

### Current Surface (intended)

```text
# Root items
pub struct Bus
pub enum Event
pub struct BusConfig
pub enum BusError

# Bus impl
impl Bus {
    pub fn new(cfg: impl Into<BusConfig>) -> Result<Self, BusError>
    pub fn sender(&self) -> tokio::sync::broadcast::Sender<Event>
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<Event>
    pub fn capacity(&self) -> usize
}

# Event (kernel-aligned, non-exhaustive)
#[non_exhaustive]
pub enum Event {
    Health { service: String, ok: bool },
    ConfigUpdated { version: u64 },
    ServiceCrashed { service: String, reason: String },
    Shutdown,
}

# Config (host-constructed, serde-enabled)
#[non_exhaustive]
pub struct BusConfig {
    pub capacity: u32,
    pub overflow_warn_rate_per_min: u32,
    pub metrics_namespace: String,
    pub emit_amnesia_label: bool,
}
impl Default for BusConfig {}
impl BusConfig {
    pub fn validate(&self) -> Result<(), String>;
}

# Errors (non-exhaustive)
#[non_exhaustive]
pub enum BusError {
    Config(String),           // invalid BusConfig
    Closed,                   // channel closed (no subscribers)
}

# Optional feature gates (public only as features, not symbols)
# - "tracing"   : add spans around publish/recv (no API shape change)
# - "loom"      : dev-only testing cfg (no API surface)
```

**Re-exports (if any, kept minimal):**

* None by default. Kernel crate may re-export `ron_bus::Bus` and `ron_bus::Event` from its own surface.

**Not public:**

* No public traits.
* No generic type parameters on `Bus` or its primary methods.
* No macros/proc-macros.

---

## 2. SemVer Discipline

### Additive (Minor / Non-Breaking)

* Adding new inherent methods on `Bus` **without** changing existing signatures.
* Adding **new `Event` variants** (enum is `#[non_exhaustive]`).
* Adding fields to `BusConfig` (struct is `#[non_exhaustive]`) with sane defaults.
* Adding optional features (off by default).

### Breaking (Major)

* Removing/renaming any existing public item.
* Changing `Bus` method signatures or return types.
* Making `Event` exhaustive or changing existing variant fields.
* Changing `BusConfig` field semantics or defaults in a way that alters behavior.
* Introducing generics on `Bus` or its primary methods (forbidden by canon).

### Patch

* Documentation or comment-only changes.
* Internal refactors that do not change symbols or documented behavior.
* Performance improvements that preserve semantics.

---

## 3. Stability Guarantees

* **MSRV:** `1.80.0` (pinned workspace-wide).
* **No `unsafe`** in this crate (`#![forbid(unsafe_code)]`).
* **Monomorphic surface:** no public generics on `Bus` (enforced in CI via `cargo public-api` diff checks).
* **No public leakage of internals:** The return types deliberately expose Tokio broadcast handles, but the `Bus` type itself owns the capacity and semantics.
* All public items must have rustdoc (`#![deny(missing_docs)]`).

---

## 4. Invariants (API-Level)

* The public API must reflect the **bounded, lossy, observable** semantics:

  * Construction via `Bus::new` enforces capacity ≥ 2 (validation errors via `BusError::Config`).
  * `subscribe()` returns a **unique** receiver per call; examples show **one receiver per task**.
  * Overflow is observable via host metrics, not via API changes.
* **Event** and **BusConfig** are `#[non_exhaustive]` to allow additive growth.
* The crate must remain **I/O-free and task-free** — API does not grow to include network endpoints, threads, or persistence.

---

## 5. Tooling

* **cargo public-api**

  * Baseline snapshots stored under `/docs/api-history/ron-bus/<version>.txt`.
  * PRs must show diffs; breaking diffs require SemVer bump rationale.
* **cargo semver-checks** (recommended)

  * Guards against accidental breaking changes.
* **cargo doc**

  * Examples must follow concurrency rules (no shared `Receiver`, handle `Lagged(n)`).

---

## 6. CI & Gates

* **Gate A (Surface Lock):**

  ```bash
  cargo public-api -p ron-bus --simplified
  ```

  Fail PR if surface changes without matching CHANGELOG entry + SemVer plan.

* **Gate B (SemVer Checks):**

  ```bash
  cargo semver-checks -p ron-bus
  ```

  Required for non-patch releases.

* **Gate C (Lint Wall):**

  * `-D warnings`
  * Deny `clippy::await_holding_lock`, `clippy::unwrap_used`, `clippy::expect_used`.

* **Gate D (Docs):**

  * `#![deny(missing_docs)]` and doctests must pass.

---

## 7. Acceptance Checklist (DoD)

* [ ] Current API snapshot generated & saved in `/docs/api-history/ron-bus/<ver>.txt`.
* [ ] `CHANGELOG.md` updated with surface and behavior notes.
* [ ] `cargo public-api` gate green (or acknowledged with SemVer bump).
* [ ] (Optional) `cargo semver-checks` green for non-breaking PRs.
* [ ] Rustdoc complete; examples reflect **one receiver per task** and `Lagged(n)` handling.
* [ ] No public generics on `Bus` or its primary methods.

---

## 8. Appendix

### References

* Rust SemVer: [https://doc.rust-lang.org/cargo/reference/semver.html](https://doc.rust-lang.org/cargo/reference/semver.html)
* cargo-public-api: [https://github.com/Enselic/cargo-public-api](https://github.com/Enselic/cargo-public-api)
* cargo-semver-checks: [https://github.com/obi1kenobi/cargo-semver-checks](https://github.com/obi1kenobi/cargo-semver-checks)

### Perfection Gates tie-in

* **Gate G:** No undocumented API surface (rustdoc coverage).
* **Gate H:** Breaking changes require **major** version bump + migration notes.
* **Gate J:** CHANGELOG alignment enforced by CI.

### History (initialize at v1.0.0)

* v1.0.0 — Initial monomorphic surface:

  * `Bus::{new,sender,subscribe,capacity}`
  * `Event::{Health,ConfigUpdated,ServiceCrashed,Shutdown} (non-exhaustive)`
  * `BusConfig::{capacity,overflow_warn_rate_per_min,metrics_namespace,emit_amnesia_label} (non-exhaustive)`
  * `BusError::{Config,Closed}`

```

