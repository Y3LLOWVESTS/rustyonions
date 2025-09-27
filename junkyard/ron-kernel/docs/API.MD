
---

# 📖 API.md — ron-kernel

---

title: API Surface & SemVer Reference
status: draft
msrv: 1.80.0
last-updated: 2025-09-27
audience: contributors, auditors, API consumers
-----------------------------------------------

# API.md

## 0. Purpose

This document captures the **public API surface** of `ron-kernel`:

* Snapshot of exported functions, types, traits, modules (the **frozen** microkernel surface).
* SemVer discipline (what breaks vs. what extends).
* CI-enforceable via `cargo public-api` (and optional `cargo semver-checks`).
* Canonical reference for downstream crates and node profiles.

---

## 1. Public API Surface

> The kernel’s public surface is intentionally **tiny and frozen**. Any drift requires a **major** bump.

Generate snapshot:

```bash
cargo public-api -p ron-kernel --simplified
```

### 1.1 Current Surface (canonical, human-readable)

```text
# Re-exports (the only stable, supported surface)
pub struct Bus
pub enum  KernelEvent
pub struct Metrics
pub struct HealthState
pub struct Config
pub async fn wait_for_ctrl_c() -> Result<(), std::io::Error>
```

### 1.2 Canonical contracts (shape & intent)

* `pub struct Bus`
  In-process **broadcast bus** (bounded), cloneable handle. Lag/drop are observable via metrics (implementation detail; not part of the surface).

* `pub enum KernelEvent`  *(shape is frozen; see SemVer rules)*

  * `Health        { service: String, ok: bool }`
  * `ConfigUpdated { version: u64 }`
  * `ServiceCrashed{ service: String, reason: String }`  *(reason is required, structured string)*
  * `Shutdown`

* `pub struct Metrics`

  * `fn new() -> Metrics`
  * `fn health(&self) -> &HealthState`
  * `async fn serve(self, addr: std::net::SocketAddr) -> (tokio::task::JoinHandle<()>, std::net::SocketAddr)`
    *(consumes `self` by design)*
    — Exposes **golden metrics** used across RustyOnions (e.g., `bus_lagged_total`, `service_restarts_total`, `request_latency_seconds`) without policy enforcement.

* `pub struct HealthState`

  * `fn set(&self, ready: bool)`
  * `fn snapshot(&self) -> bool`        // or richer snapshot in future
  * `fn all_ready(&self) -> bool`

* `pub async fn wait_for_ctrl_c() -> Result<(), std::io::Error>`
  Cooperative shutdown helper.

**Notes**

* TLS types are not exposed here; services consume `tokio_rustls::rustls::ServerConfig`.
* No transport/overlay/storage APIs leak into the kernel surface.

---

## 2. SemVer Discipline

**Strict SemVer** with a **frozen** core surface.

### 2.1 Additive (Minor / Non-Breaking)

* Docs, tests, internal changes.
* Adding methods to **opaque** types when it cannot change behavior or trait guarantees (rare).
* Adding metrics labels/fields (affects emitted metrics, not this Rust surface) — document in CHANGELOG.

> We **do not** promise additive variants on `KernelEvent`; it is a fixed contract.

### 2.2 Breaking (Major)

* Any change to the re-export list (`Bus`, `KernelEvent`, `Metrics`, `HealthState`, `Config`, `wait_for_ctrl_c`).
* Renaming/removing exports, changing visibility/paths.
* Signature/ownership changes (e.g., making `Metrics::serve` borrow instead of consume).
* Changing `KernelEvent` shape (renames, add/remove fields, add new variants).
* Changing return types (e.g., `wait_for_ctrl_c` error type).

### 2.3 Patch-Level (Non-Breaking)

* Doc fixes, examples, internal perf/resilience with identical surface.
* Bug fixes that do not alter observable API.

---

## 3. Stability Guarantees

* **MSRV:** `1.80.0` (MSRV bumps require a major version bump or clearly documented policy exception).
* **No `unsafe`** in public API (`#![forbid(unsafe_code)]`).
* No leaking incidental runtime types (e.g., TCP streams, axum state).
* `Metrics::serve` **consumes** `self` intentionally; do not assume interior mutability.

---

## 4. Invariants

* **Kernel role (scope)**

  * Public surface is **only**: `Bus`, `KernelEvent`, `Metrics`, `HealthState`, `Config`, `wait_for_ctrl_c`.
  * No overlay/DHT/storage/ledger/app logic in this crate.

* **Concurrency (contextual, not surface)**

  * **One broadcast receiver per task**, bounded bus, **no locks across `.await`** on supervisory paths.

* **Context hooks (not surface)**

  * **Amnesia Mode** and **PQ posture** are surfaced via `Config` and passed through; the kernel **does not** enforce crypto or persistence policy.

* **Review & CI**

  * API diffs **must** be reviewed; CI runs `cargo public-api` and fails on unacknowledged changes.
  * All public items **must** have rustdoc (`#![deny(missing_docs]` in CI).

---

## 5. Tooling

* **cargo public-api** — authoritative surface diff

  ```bash
  cargo public-api -p ron-kernel --simplified
  ```
* **cargo semver-checks** — SemVer advisor (optional but recommended)

  ```bash
  cargo install cargo-semver-checks
  cargo semver-checks check-release -p ron-kernel
  ```
* **cargo doc** — docs + doctests

  ```bash
  cargo doc -p ron-kernel --no-deps
  cargo test -p ron-kernel --doc
  ```
* **API snapshots** (optional)

  * Store under `/docs/api-history/ron-kernel/<version>.txt`.

---

## 6. CI & Gates

* **Public API gate**

  ```yaml
  - name: public-api
    run: cargo public-api -p ron-kernel --deny changed
  ```
* **Missing docs gate**

  ```yaml
  - name: docs
    run: RUSTDOCFLAGS="--deny missing_docs" cargo doc -p ron-kernel --no-deps
  ```
* **SemVer advisor (optional)**

  ```yaml
  - name: semver-checks
    run: cargo semver-checks check-release -p ron-kernel || true
  ```
* **CHANGELOG hygiene**

  * If `cargo public-api` reports a surface change, the PR **must** include a matching CHANGELOG entry with SemVer rationale.

---

## 7. Acceptance Checklist (Definition of Done)

* [ ] API snapshot generated (`cargo public-api -p ron-kernel --simplified`) and attached to PR.
* [ ] CI gate passes **or** change is explicitly acknowledged as **major** (version bump + migration notes).
* [ ] CHANGELOG updated for any surface change (added/removed/renamed items, signature changes).
* [ ] Rustdoc coverage: **no missing docs** on public items.
* [ ] Doctests/examples updated and passing.
* [ ] Downstream crates audited for breakage (bus/event consumers, profiles).

---

## 8. Appendix

**References**

* Rust SemVer guidelines — [https://doc.rust-lang.org/cargo/reference/semver.html](https://doc.rust-lang.org/cargo/reference/semver.html)
* cargo-public-api — [https://github.com/Enselic/cargo-public-api](https://github.com/Enselic/cargo-public-api)
* cargo-semver-checks — [https://github.com/obi1kenobi/cargo-semver-checks](https://github.com/obi1kenobi/cargo-semver-checks)

**Perfection Gates tie-in**

* **Gate G:** No undocumented public items.
* **Gate H:** Breaking changes require a major bump.
* **Gate J:** CHANGELOG alignment with any public diff.

**History (examples)**

* **v1.0.0** — Initial frozen API: re-exports `Bus`, `KernelEvent` (4 variants), `Metrics`, `HealthState`, `Config`, `wait_for_ctrl_c`; `Metrics::serve(self, addr) -> (JoinHandle<()>, SocketAddr)`; `HealthState::{set,snapshot,all_ready}`.

**Contributor note**
Propose any change to `KernelEvent` or the re-export list via RFC first. The small, stable surface is what keeps the microkernel drift-proof.
