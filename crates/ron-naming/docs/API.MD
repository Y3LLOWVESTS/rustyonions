

# 📖 API.md — Template


---
title: API Surface & SemVer Reference
status: draft
msrv: 1.80.0
last-updated: YYYY-MM-DD
audience: contributors, auditors, API consumers
---

# API.md

## 0. Purpose

This document captures the **public API surface** of the crate:

- Snapshot of exported functions, types, traits, modules.
- SemVer discipline: what changes break vs. extend.
- Alignment with CHANGELOG.md (behavioral vs. surface changes).
- CI-enforceable via `cargo public-api` (or similar).
- Acts as the “spec” for external consumers of this crate.

---

## 1. Public API Surface

Generated via:

```bash
cargo public-api --simplified --deny-changes
````

### Current Surface

```text
# Example output (replace with actual run)
pub struct Bus
pub enum KernelEvent
pub fn wait_for_ctrl_c() -> Result<(), std::io::Error>
...
```

* For **services:** Document CLI/HTTP/gRPC/REST endpoints as well as Rust surface.
* For **libs:** List modules, traits, and re-exports.

---

## 2. SemVer Discipline

### Additive (Minor/Non-Breaking)

* Adding new functions, traits, modules (non-default).
* Expanding enums with `#[non_exhaustive]`.
* Adding fields with `#[non_exhaustive]`.

### Breaking (Major)

* Removing or renaming exports.
* Changing function signatures or trait bounds.
* Changing error types or return values.
* Making previously stable enums exhaustive.

### Patch-Level

* Doc-only fixes.
* Internal impl changes with identical API.
* Performance improvements without surface change.

---

## 3. Stability Guarantees

* MSRV: `1.80.0`.
* Public API changes tracked in this doc + CHANGELOG.
* Unsafe code forbidden unless explicitly justified.
* No leaking of internal types (e.g., `tokio::net::TcpStream`) unless intentional.

---

## 4. Invariants

* Exports MUST align with crate role:

  * **Kernel crates:** only re-export Bus, KernelEvent, Metrics, Config.
  * **SDK crates:** stable client API; error enums `#[non_exhaustive]`.
  * **Service crates:** no direct public API beyond CLI/HTTP surface.

* API diffs MUST be reviewed in PR → CI gate enforces `cargo public-api`.

* All public items MUST be documented (`#![deny(missing_docs)]`).

---

## 5. Tooling

* **cargo public-api** → detects surface diffs.
* **cargo semver-checks** (optional) → ensures semver compliance.
* **cargo doc** + doc tests for examples.
* **API snapshots** stored in `/docs/api-history/{crate}/{version}.txt`.

---

## 6. CI & Gates

* PR pipeline runs `cargo public-api`.
* Fail if any breaking changes are unacknowledged.
* Auto-open PR comment with diff of added/removed symbols.
* Sync with CHANGELOG: if surface diff exists, CHANGELOG entry MUST exist.

---

## 7. Acceptance Checklist (DoD)

* [ ] Current API snapshot generated & stored.
* [ ] SemVer discipline reviewed.
* [ ] CI gate passes (`cargo public-api`).
* [ ] CHANGELOG updated for any surface changes.
* [ ] Docs/tests updated for new/changed APIs.
* [ ] Service crates: endpoints documented in `/docs/openapi/`.

---

## 8. Appendix

* **References:**

  * Rust SemVer guidelines: [https://doc.rust-lang.org/cargo/reference/semver.html](https://doc.rust-lang.org/cargo/reference/semver.html)
  * cargo-public-api: [https://github.com/Enselic/cargo-public-api](https://github.com/Enselic/cargo-public-api)
  * cargo-semver-checks: [https://github.com/obi1kenobi/cargo-semver-checks](https://github.com/obi1kenobi/cargo-semver-checks)

* **Perfection Gates tie-in:**

  * Gate G: No undocumented API surface.
  * Gate H: Breaking changes require major version bump.
  * Gate J: CHANGELOG alignment enforced.

* **History Section:**

  * Record notable API shifts (e.g., KernelEvent variants).



---

✅ This **API.md template** is:  
- **Scannable** (snapshot + diffs).  
- **Enforceable** (CI gates).  
- **Aligned** with blueprints (SemVer discipline, Perfection Gates).  
- **Adaptive** (covers libs, SDKs, services).  

