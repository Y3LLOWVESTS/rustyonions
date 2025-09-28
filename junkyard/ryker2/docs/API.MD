
**File:** `crates/ryker/docs/API.md`

````markdown
---
title: API Surface & SemVer Reference — ryker
status: draft
msrv: 1.80.0
last-updated: 2025-09-28
audience: contributors, auditors, API consumers
concerns: [RES, PERF]
---

# API.md

## 0. Purpose

This document captures the **public API surface** of `ryker` and our **SemVer discipline**:

- Snapshot of exported functions, types, traits, and modules.
- What changes are **breaking** vs. **additive** vs. **patch-only**.
- CI gates: `cargo public-api` (+ optional `cargo semver-checks`), doc build, deny/audit.
- Source of truth for external consumers and release reviews (CHANGELOG alignment).

> Scope: `ryker` is a **library crate** (embedded actor/mailbox runtime). It does **not** own a runtime or sockets/HTTP surface. Any CLI helpers are feature-gated and **not** part of the library API.

---

## 1. Public API Surface

### 1.1 Generate the authoritative snapshot

```bash
cargo install cargo-public-api
cargo public-api --simplified --deny-changes -p ryker
cargo public-api --simplified -p ryker > docs/api-history/ryker/<next-version>.txt
git add docs/api-history/ryker/<next-version>.txt
````

Quick diff vs previous tag (handy in PRs):

```bash
cargo public-api --diff-git-checkouts "$(git describe --tags --abbrev=0 @^)" @ -p ryker
```

> Machine snapshot lives in `docs/api-history/ryker/` and is enforced by CI. The curated baseline below is human-readable guidance; keep them in sync.

### 1.2 Curated Baseline (human-readable)

> Namespaces shown as `ryker::<module>::Type`. Hyperlinks point to generated docs.rs for reference.

#### Top-level modules

* [`ryker::config`](https://docs.rs/ryker/latest/ryker/config/) — configuration schema & loader
* [`ryker::runtime`](https://docs.rs/ryker/latest/ryker/runtime/) — runtime constructor and actor/mailbox building blocks
* [`ryker::mailbox`](https://docs.rs/ryker/latest/ryker/mailbox/) — mailbox types and builder
* [`ryker::prelude`](https://docs.rs/ryker/latest/ryker/prelude/) — ergonomic re-exports
* [`ryker::errors`](https://docs.rs/ryker/latest/ryker/errors/) — typed error(s) surfaced by config/runtime

#### Example key types

* [`RykerConfig`](https://docs.rs/ryker/latest/ryker/config/struct.RykerConfig.html)
* [`Runtime`](https://docs.rs/ryker/latest/ryker/runtime/struct.Runtime.html)
* [`Mailbox`](https://docs.rs/ryker/latest/ryker/mailbox/struct.Mailbox.html)
* [`MailboxBuilder`](https://docs.rs/ryker/latest/ryker/mailbox/struct.MailboxBuilder.html)

### 1.3 API Map (Mermaid)

```mermaid
flowchart TD
  subgraph ryker
    C[config] -->|RykerConfig| R[runtime::Runtime]
    C -->|SpanLevel, Errors| E[errors]
    R -->|mailbox<T>() ->| MB[mailbox::MailboxBuilder<T>]
    MB -->|build() ->| M[mailbox::Mailbox<T>]
    P[prelude] --> C
    P --> R
    P --> M
  end
  style ryker fill:#0ea5e9,stroke:#0c4a6e,color:#fff
```

**Text alt (accessibility):** The `ryker` crate exposes modules `config`, `runtime`, `mailbox`, `errors`, and `prelude`. `RykerConfig` flows into `Runtime`, which can construct `MailboxBuilder`, which builds a `Mailbox`. The `prelude` re-exports key items.

---

## 2. SemVer Discipline

### 2.1 Additive (Minor / Non-Breaking)

* Adding new **modules**, **structs**, **methods**, **traits** (not enabled by default features).
* Adding fields/variants to **`#[non_exhaustive]`** items.
* Adding **feature-gated** APIs (feature is opt-in).

### 2.2 Breaking (Major)

* Removing/renaming any public item or changing visibility/hierarchy.
* Changing function signatures, trait bounds, or return/error **types**.
* Invalidating config without a deprecation window.
* Moving an existing public API **behind** a feature flag.
* Altering `SpanLevel` variants (it is **exhaustive**).

**Deprecation policy (Rust surface):**

```rust
#[deprecated(since = "0.5.0", note = "use Runtime::mailbox instead")]
pub fn actor_mailbox(...) -> ...
```

* Mark old items for **≥1 minor** release.
* Document in CHANGELOG under *Deprecated*; remove in the next **major**.
* For **env keys**: maintain aliases + WARN for ≥1 minor (see CONFIG.md).

### 2.3 Patch-Level

* Docs/examples; internal perf refactors with identical API.
* Clearer error **messages** while preserving **types/variants**.
* Accepting **additional** value formats within documented ranges.

---

## 3. Stability Guarantees

* **MSRV**: `1.80.0`.
* **Unsafe**: no `unsafe` in public API.
* **Docs**: `#![deny(missing_docs)]` for all public items.
* **No leaky internals**: avoid exposing unstable external types unless intentionally stable.
* **PQ / Amnesia notes**:

  * **No crypto API** in ryker; **PQ = N/A**.
  * **Amnesia** affects runtime behavior (zeroize, no persistence) but does **not** alter the API surface.

---

## 4. Invariants (API Contract)

* Library-only: no network, no `/healthz`, no `/metrics`.
* Constructors/builders are **non-blocking**; validation is strict and **fail-closed**.
* Mailboxes are **bounded**; producers see explicit **Busy** at capacity.
* Deadlines/fairness are observable and never silently mutated.
* Env/alias rules are **documented** in CONFIG.md.

---

## 5. Tooling

* `cargo public-api` — detect public surface diffs.
* `cargo semver-checks` — SemVer analysis.

  ```bash
  cargo install cargo-semver-checks
  cargo semver-checks check -p ryker
  ```
* `cargo doc` — rustdoc must build clean (all features).
* Snapshots: `docs/api-history/ryker/{version}.txt`.

---

## 6. CI & Gates

* **Public surface**: `cargo public-api --simplified -p ryker --deny-changes`.
* **Doc build**: `cargo doc --no-deps --all-features -D warnings`.
* **SemVer checks**: `cargo semver-checks check -p ryker`.
* **Supply chain**: `cargo deny check` and `cargo audit`.
* **Governance**: If removals/renames are detected, label PR **breaking-change** and require maintainer approval. Optionally, automation can bump Cargo.toml version on such PRs.
* **Auto-diff**: CI posts a comment showing added/removed symbols when diffs exist.

---

## 7. Acceptance Checklist (DoD)

* [ ] Current API snapshot generated & stored.
* [ ] Rustdoc builds clean (`cargo doc --all-features -D warnings`).
* [ ] SemVer discipline reviewed; if breaking, **major bump** rationale recorded.
* [ ] CI gate passes (`cargo public-api`).
* [ ] CHANGELOG updated for any surface changes.
* [ ] Docs/tests updated for new/changed APIs.
* [ ] Property tests for invariants (e.g., Busy on overflow) in place.
* [ ] Service crates only: endpoints documented separately (N/A for ryker).

---

## 8. Appendix

### 8.1 References

* Rust SemVer: [https://doc.rust-lang.org/cargo/reference/semver.html](https://doc.rust-lang.org/cargo/reference/semver.html)
* cargo-public-api: [https://github.com/Enselic/cargo-public-api](https://github.com/Enselic/cargo-public-api)
* cargo-semver-checks: [https://github.com/obi1kenobi/cargo-semver-checks](https://github.com/obi1kenobi/cargo-semver-checks)

### 8.2 Perfection Gates

* **Gate G:** No undocumented API surface.
* **Gate H:** Breaking changes require major bump.
* **Gate J:** CHANGELOG alignment enforced.

### 8.3 History

* vX.Y.Z — Introduced `RykerReloadHook` and `SpanLevel::from_str`.
* vX.Y.(Z+1) — Tightened config validation; no surface change.
* vY.0.0 — (Example) Moved `MailboxBuilder` under `ryker::mailbox`.

> In CI, diffs from `cargo public-api` are auto-posted as PR comments. Maintainers update this section when notable changes occur.

```

---

