---

crate: ryker
path: crates/ryker
role: lib
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
-------------------------

## 1) One-liner

Prototype pricing/monetization utilities for RustyOnions manifests: parse price models, validate payment blocks, and compute request costs.

## 2) Primary Responsibilities

* Translate `naming::manifest::Payment` policies into executable logic (parse price model, compute cost, validate fields).
* Provide lightweight, dependency-minimal validation primitives for wallets and payment blocks.

## 3) Non-Goals

* No payment processing, settlement, exchange-rate handling, or cryptographic verification.
* No networking, storage, or runtime orchestration.
* Not a stable public API (explicitly experimental/scratchpad).

## 4) Public API Surface

* **Re-exports:** none.
* **Key types / functions / traits (from `src/lib.rs`):**

  * `enum PriceModel { PerMiB, Flat, PerRequest }` + `PriceModel::parse(&str) -> Option<Self>`
    Accepted strings (case-insensitive): `"per_mib" | "flat" | "per_request"`.
  * `compute_cost(n_bytes: u64, p: &naming::manifest::Payment) -> Option<f64>`

    * Returns `None` when `p.required == false`.
    * `PerMiB`: `price * (n_bytes / 1,048,576)`.
    * `Flat`/`PerRequest`: returns `price` (bytes do not affect cost).
  * `validate_wallet_string(&str) -> Result<()>`

    * Currently only checks non-empty; comments outline future heuristics (LNURL, BTC, SOL, ETH).
  * `validate_payment_block(p: &Payment) -> Result<()>`

    * Ensures parseable `price_model`, non-empty `wallet`, and `price >= 0.0`.
* **Events / HTTP / CLI:** none.

## 5) Dependencies & Coupling

* **Internal crates:** `naming` (tight) — uses `naming::manifest::{Payment, RevenueSplit}`. Replaceable: **yes**, by defining an internal trait to decouple from concrete `Payment` or relocating logic into `naming`.
* **External (top):**

  * `anyhow` (workspace) — ergonomic errors; low risk.
  * `serde` (workspace) — not used directly in current code; low risk.
  * `workspace-hack` — dedupe shim.
* **Runtime services:** none (pure compute).

## 6) Config & Feature Flags

* No feature flags, no env vars. Behavior is entirely driven by the `Payment` struct inputs.

## 7) Observability

* None. No `tracing` spans, metrics, or structured error taxonomy beyond `anyhow::Error`.

## 8) Concurrency Model

* None. All functions are synchronous, CPU-bound, and side-effect-free.

## 9) Persistence & Data Model

* None. Operates on in-memory `Payment` values from `naming::manifest`.

## 10) Errors & Security

* **Errors:**

  * `validate_*` return `anyhow::Error` on invalid inputs (unknown model, empty wallet, negative price).
  * `compute_cost` uses `Option` to signal “no charge” when policy is not required or parse fails.
* **Security:**

  * No authn/z, no signature checks, no token parsing; only a non-empty wallet string check.
  * No TLS/crypto; not applicable for this pure function crate.
  * PQ-readiness N/A at this layer.

## 11) Performance Notes

* O(1) arithmetic; microseconds per call even at high volumes.
* `PerMiB` math uses `f64`; precision is adequate for display but not settlement-grade accounting.

## 12) Tests

* **Unit tests present:**

  * Cost computation: `per_mib` (2 MiB at \$0.01/MiB ≈ \$0.02), `flat` invariant w\.r.t bytes.
  * Policy gating: `required=false` → `None`.
  * Validation happy-path (`per_request` with non-empty wallet).
* **Gaps to add:**

  * Unknown `price_model` → `parse(None)` and `validate_payment_block` error.
  * Negative `price` → error.
  * Boundary cases (0 bytes; extremely large `n_bytes` for overflow safety).
  * Wallet heuristics unit tests once implemented.

## 13) Improvement Opportunities

### Known gaps / tech debt

* **Float money math:** `f64` is inappropriate for billing-grade arithmetic. Use fixed-precision integers (e.g., micro-units) or `rust_decimal` for currency amounts; define rounding rules.
* **Coupling to `naming`:** Logic is bound to a specific struct; either:

  1. Move this module into `naming` (e.g., `naming::pricing`), or
  2. Extract a small trait (`PaymentPolicy`) consumed by `ryker`, reducing coupling and enabling reuse in other contexts.
* **Error semantics:** Mixed `Option`/`Result` can hide policy typos (e.g., misspelled `price_model` returns `None` and looks like “free”). Consider a stricter mode (feature flag or function variant) that errors on malformed policy vs “not required”.
* **Observability:** Add optional `tracing` spans (e.g., `pricing.compute_cost`) and counters (rejects, unknown model, negative price), guarded by a feature flag to keep the crate lean.
* **Wallet validation:** Implement basic format checks per scheme (LNURL bech32, BTC bech32/base58, SOL length/base58, ETH `0x` + 40 hex) behind feature flags; keep the default permissive.
* **Naming & purpose:** “ryker” is opaque. If promoted beyond scratchpad, consider renaming to `ron-pricing` or `ron-billing` and documenting versioning/compat promises.

### Overlap & redundancy signals

* Overlaps conceptually with any future “billing” logic that might live in `gateway` or `overlay`. Keeping price computation centralized here (or in `naming`) prevents drift.
* If we later add enforcement in services, ensure they call a single shared function (this crate) to avoid duplicated formulas.

### Streamlining

* Provide a **single façade**: `Pricing::from(&Payment).compute(n_bytes)` returning a typed `Amount` newtype with currency + minor units.
* Introduce a **strict mode** API (e.g., `compute_cost_strict`) that errors on malformed policies instead of returning `None`.
* Add currency & rounding policy hooks (banker’s rounding, min charge thresholds).

## 14) Change Log (recent)

* 2025-09-14 — Initial pricing utilities and validations reviewed; unit tests for `per_mib`, `flat`, `required=false`, and validation.

## 15) Readiness Score (0–5 each)

* **API clarity:** 3 — Small and understandable, but experimental name and mixed `Option`/`Result` semantics need polishing.
* **Test coverage:** 3 — Core paths covered; add negative/edge cases.
* **Observability:** 1 — None yet.
* **Config hygiene:** 3 — No config needed; consider feature flags for strictness/validation depth.
* **Security posture:** 2 — Minimal input checks; no scheme validation; fine for prototype.
* **Performance confidence:** 5 — Trivial compute.
* **Coupling (lower is better):** 2 — Tight to `naming::manifest::Payment`; can be improved with a trait or co-location.

