---

title: API Surface & SemVer Reference — ron-policy
status: draft
msrv: 1.80.0
last-updated: 2025-10-06
audience: contributors, auditors, API consumers
-----------------------------------------------

# API.md — ron-policy (v0.1.3)

## 0. Purpose

This document captures the **public API surface** of `ron-policy`:

* Snapshot of exported functions, types, traits, and modules.
* SemVer discipline: what changes are **additive** vs **breaking**.
* Alignment with `CHANGELOG.md` (behavioral vs surface changes).
* CI-enforceable via `cargo public-api` and `cargo semver-checks`.
* Serves as the **spec** for external consumers (services, SDKs).

`ron-policy` is a **pure library**: it validates **policy bundles** and performs **deterministic evaluation**. No I/O, no crypto, no background tasks.

---

## 1. Public API Surface

> Generate a fresh snapshot on each release:
>
> ```bash
> VERSION=$(cargo pkgid -p ron-policy | sed 's/.*#//')
> cargo public-api -p ron-policy --simplified > docs/api-history/ron-policy/${VERSION}.txt
> cargo public-api -p ron-policy --simplified --deny-changes
> ```

### 1.1 Current Surface (intended, stable)

```text
# Crate root
pub mod model

// --- Core DTOs (serde; JSON canonical) ---
pub use model::PolicyBundle;
pub use model::Residency;
pub use model::PlacementPrefs;
pub use model::QuotaRule;
pub use model::RouteRule;
pub use model::Features;
pub use model::Meta;
pub use model::Decision;
pub use model::Obligation;   // (non_exhaustive in code)
pub use model::Scope;
pub use model::AppliesTo;
pub use model::Limit;        // (non_exhaustive in code)

// --- Evaluation ---
pub struct RequestCtx<'a> {
    pub route: &'a str,
    pub body_len: u64,
    pub target_region: &'a str,
    pub has_cap: bool,
    pub peer_id: &'a str,
    pub cap_audience: Option<&'a str>,
}
pub fn evaluate(bundle: &PolicyBundle, ctx: &RequestCtx<'_>, now_ms: u64) -> Decision;

// --- Config & Validation ---
pub struct Config;
pub struct LimitsCfg;
pub struct PerfCfg;
pub enum  CanonicalWire { Json, Toml }
pub struct SchemaCfg;

pub struct Validation<'a> { /* new(&'a Config) */ }
impl<'a> Validation<'a> {
    pub fn new(cfg: &'a Config) -> Self;
    pub fn validate_config(&self) -> anyhow::Result<()>;
    pub fn validate_bundle(&self, bundle: &PolicyBundle) -> Result<(), ValidationError>;
}

pub fn validate_bundle_diff(old: &PolicyBundle, new: &PolicyBundle) -> Result<(), ValidationError>;

// --- Errors (non_exhaustive) ---
#[non_exhaustive]
pub enum ValidationError {
    TightenOnly,     // relaxes global caps
    Churn,           // widening without break_change
    UnknownField,    // schema strictness
    Schema(String),  // detailed message
    PerfBudget,      // violates local perf intent
    Other(String),
}

// --- Helpers ---
pub fn parse_bundle_json(src: &str) -> Result<PolicyBundle, serde_json::Error>;
pub fn parse_bundle_toml(src: &str) -> Result<PolicyBundle, toml::de::Error>;
pub fn recompute_b3(canonical_json: &str) -> String;

// --- Reason codes (stable strings) ---
pub const REASON_OK: &str;
pub const REASON_REGION_DENIED: &str;
pub const REASON_REGION_NOT_ALLOWED: &str;
pub const REASON_CAP_REQUIRED: &str;
pub const REASON_BODY_TOO_LARGE: &str;
pub const REASON_DECOMPRESS_GUARD: &str;
// validation-only causes:
pub const REASON_TIGHTEN_ONLY: &str;
pub const REASON_CHURN: &str;
pub const REASON_SCHEMA: &str;
pub const REASON_PERF: &str;
pub const REASON_OTHER: &str;

// --- Feature-gated (additive) ---
#[cfg(feature = "schema")]
pub fn emit_json_schema() -> String;

// --- Optional prelude (DX; additive) ---
pub mod prelude {
    pub use crate::{PolicyBundle, RequestCtx, Decision, evaluate};
}
```

**Notes**

* DTOs derive: `Serialize + Deserialize + Clone + Debug + PartialEq` and use `#[serde(deny_unknown_fields)]`.
* Growth-prone enums are **`#[non_exhaustive]`** in code: `ValidationError`, `Obligation`, `Limit`.
* `evaluate` is **pure** and **sync**; callers supply `now_ms`.

### 1.2 Module Structure

```text
ron_policy
├── model        // DTOs & enums (serde)
├── eval         // evaluate() & RequestCtx
├── config       // Config, LimitsCfg, PerfCfg, SchemaCfg, CanonicalWire
└── validate     // Validation, validate_bundle_diff, ValidationError
```

---

## 2. SemVer Discipline

### 2.1 Additive (Minor / Non-Breaking)

* Adding new free functions (no signature changes).
* Adding new **enum variants** to `#[non_exhaustive]` enums (e.g., `Obligation`, `Limit`, `ValidationError`).
* Adding **feature-gated** APIs (e.g., `emit_json_schema`).
* Adding new **reason code constants** (existing literals remain unchanged).

### 2.2 Breaking (Major)

* Removing/renaming any public symbol listed in §1.1.
* Changing function signatures or argument/return types.
* Making previously `#[non_exhaustive]` enums **exhaustive**.
* Changing JSON DTO shapes in a non–backward-compatible way.
* Renaming/removing **reason code string literals**.
* Bumping **MSRV** below `1.80.0`.

### 2.3 Patch-Level

* Docs, internal refactors, perf improvements with **identical** results & error taxonomy.
* New validations that are opt-in or only affect **invalid** inputs.

### 2.4 Deprecation Policy

* Use `#[deprecated(since = "x.y.z", note = "...")]` for ≥1 minor before removal.
* CHANGELOG must include **Deprecated** and later **Removed** sections.

---

## 3. Stability Guarantees

* **MSRV:** `1.80.0` across the 0.x series (MSRV bumps are **breaking**).
* **Determinism:** `evaluate` yields identical `Decision` for identical `(bundle, ctx, now_ms)` across patch/minor versions.
* **No I/O / No crypto:** Public APIs perform no network/filesystem or crypto operations.
* **No panics on valid inputs:** Public functions return typed errors; panics are considered bugs.
* **Serde compatibility:** Field names/types remain stable; DTO additivity is handled by **bundle versioning**, not by accepting unknown fields (those are rejected by design).

---

## 4. Invariants (API-Level)

* **Pure boundary:** No secrets; only declarative **obligations** (e.g., proof hints).
* **Tighten-only:** Validators enforce global upper bounds (1 MiB body, 10× decompress).
* **Churn safety:** Widening residency/limits requires `metadata.break_change = true` + runbook link; otherwise `ValidationError::Churn`.
* **Reason codes:** String literals in Appendix A are **stable identifiers**.

---

## 5. Tooling

* **cargo-public-api** — detect surface diffs and generate snapshots.
* **cargo-semver-checks** — verify SemVer compliance.
* **cargo-doc (rustdoc JSON)** — optional snapshot for richer diffs:

  ```bash
  RUSTDOCFLAGS="--output-format json" cargo doc -p ron-policy
  ```
* **cargo-fuzz** — fuzz parsers (`parse_bundle_json`, `parse_bundle_toml`).

---

## 6. CI & Gates

```yaml
name: api-surface
on: [pull_request]
jobs:
  public_api:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo install cargo-public-api
      - run: cargo public-api -p ron-policy --simplified --deny-changes

  semver_checks:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo install cargo-semver-checks
      - run: cargo semver-checks check-release -p ron-policy

  rustdoc_json:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: RUSTDOCFLAGS="--output-format json" cargo doc -p ron-policy

  fuzz_smoke:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo install cargo-fuzz
      - run: cargo fuzz run parse_bundle -- -max_total_time=30
```

**Policy:** Fail PR if public API diff exists **without** a matching `CHANGELOG.md` entry and reviewer approval.

---

## 7. Acceptance Checklist (DoD)

* [ ] API snapshot generated → `docs/api-history/ron-policy/<version>.txt`.
* [ ] CI `cargo public-api` and `cargo semver-checks` green.
* [ ] `CHANGELOG.md` updated for any surface/behavior change.
* [ ] All public items documented (`#![deny(missing_docs)]` satisfied).
* [ ] **Reason code literals** verified by test (see below).
* [ ] Fuzz target exists for parsers; smoke-run wired in CI.
* [ ] Deprecations annotated with `#[deprecated]` (if any).

**Reason literal stability test (paste-ready):**

```rust
#[test]
fn reason_literals_are_stable() {
    assert_eq!(ron_policy::REASON_OK, "ok");
    assert_eq!(ron_policy::REASON_REGION_DENIED, "region.denied");
    assert_eq!(ron_policy::REASON_REGION_NOT_ALLOWED, "region.not_allowed");
    assert_eq!(ron_policy::REASON_CAP_REQUIRED, "cap.required");
    assert_eq!(ron_policy::REASON_BODY_TOO_LARGE, "body.too_large");
    assert_eq!(ron_policy::REASON_DECOMPRESS_GUARD, "decompress.guard");
    assert_eq!(ron_policy::REASON_TIGHTEN_ONLY, "tighten_only");
    assert_eq!(ron_policy::REASON_CHURN, "churn");
    assert_eq!(ron_policy::REASON_SCHEMA, "schema");
    assert_eq!(ron_policy::REASON_PERF, "perf");
    assert_eq!(ron_policy::REASON_OTHER, "other");
}
```

---

## 8. Appendix A — Reason Code Set (stable literals)

```rust
pub const REASON_OK: &str                 = "ok";
pub const REASON_REGION_DENIED: &str      = "region.denied";
pub const REASON_REGION_NOT_ALLOWED: &str = "region.not_allowed";
pub const REASON_CAP_REQUIRED: &str       = "cap.required";
pub const REASON_BODY_TOO_LARGE: &str     = "body.too_large";
pub const REASON_DECOMPRESS_GUARD: &str   = "decompress.guard";

// Validation-only causes:
pub const REASON_TIGHTEN_ONLY: &str       = "tighten_only";
pub const REASON_CHURN: &str              = "churn";
pub const REASON_SCHEMA: &str             = "schema";
pub const REASON_PERF: &str               = "perf";
pub const REASON_OTHER: &str              = "other";
```

Adding new reasons is **minor**; renaming/removing existing ones is **major**.

---

## 9. Appendix B — Minimal Usage Examples

### 9.1 Validation & Snapshot Swap

```rust
use ron_policy::{Config, Validation, PolicyBundle};

let cfg = Config::default();
Validation::new(&cfg).validate_config()?;

// Load from registry (service I/O, not in this crate)
let new_bundle: PolicyBundle = load_bundle_from_registry()?;

// Tighten-only + churn checks happen here:
Validation::new(&cfg).validate_bundle(&new_bundle)?;

// Atomically swap Arc<PolicyBundle> snapshot in the service.
```

### 9.2 Deterministic Evaluation

```rust
use ron_policy::{evaluate, model::Decision, PolicyBundle, RequestCtx};

let decision: Decision = evaluate(&bundle, &RequestCtx {
    route: "GET /o/*",
    body_len: 512,
    target_region: "us-east-1",
    has_cap: true,
    peer_id: "p1",
    cap_audience: Some("u1"),
}, 1_764_950_400_000);

assert!(decision.allow);
```

---

## 10. Appendix C — API Evolution Guardrails (DTOs)

* DTOs are **wire contracts**. Any incompatible change to shape/semantics is **major**.
* Keep `serde(deny_unknown_fields)` enabled; evolve via **bundle versioning** and rollouts, not by ignoring fields.
* Growth-friendly enums:

  ```rust
  #[non_exhaustive] pub enum Obligation { /* … */ }
  #[non_exhaustive] pub enum Limit      { /* … */ }
  #[non_exhaustive] pub enum ValidationError { /* … */ }
  ```
* MSRV adherence: dependencies pinned so the crate builds on `1.80.0`.

---

## 11. History (human-curated)

* **0.1.0** — Baseline: DTOs, `evaluate`, `Validation`, `validate_bundle_diff`, reason codes.
* **0.1.1** — Added `Obligation::RequireProof { proof_kind, scope }` (additive). Added perf budget config (`PerfCfg`).
* **0.1.2** — Introduced `metadata.break_change` gating, `CanonicalWire`, and `emit_json_schema` (feature `schema`). Expanded `ValidationError`.
* **0.1.3** — Marked `Obligation`/`Limit`/`ValidationError` as `#[non_exhaustive]`; added `prelude` module; added **no-panic** guarantee; added parser fuzz & rustdoc JSON gates; locked **reason literal** stability test.

---

### References

* Rust SemVer: [https://doc.rust-lang.org/cargo/reference/semver.html](https://doc.rust-lang.org/cargo/reference/semver.html)
* cargo-public-api: [https://github.com/Enselic/cargo-public-api](https://github.com/Enselic/cargo-public-api)
* cargo-semver-checks: [https://github.com/obi1kenobi/cargo-semver-checks](https://github.com/obi1kenobi/cargo-semver-checks)

---

**Perfection Gates tie-in**

* **Gate G:** No undocumented API surface (public-api snapshot).
* **Gate H:** Breaking changes require a major bump (+ migration notes).
* **Gate J:** CHANGELOG entry required for any surface or behavior change.

---
