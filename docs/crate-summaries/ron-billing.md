---

crate: ron-billing
path: crates/ron-billing
role: lib
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
-------------------------

## 1) One-liner

Lightweight billing primitives: compute request cost from a manifest `Payment` policy and validate basic billing fields (model, wallet, price) without performing settlement.

## 2) Primary Responsibilities

* Provide a strict but minimal **pricing model** (`PriceModel`) and **cost calculator** (`compute_cost`) over byte counts and manifest `Payment` policy.
* Validate **payment blocks** for sanity (`validate_payment_block`) and **wallet strings** for presence/format baseline.

## 3) Non-Goals

* No currency FX, tax/VAT, or monetary rounding strategy beyond raw `f64` math.
* No settlement, invoicing, balances, receipts, or billing storage.
* No network lookups (LNURL resolution, chain RPC), KYC, or fraud detection.
* No quota/rate limiting (that’s `ron-policy`), and no business rules for revenue splits beyond what the manifest already declares.

## 4) Public API Surface

* Re-exports: none (crate stands alone; `ryker` may re-export during migration).
* Key types / functions / traits:

  * `enum PriceModel { PerMiB, Flat, PerRequest }` with `PriceModel::parse(&str) -> Option<Self>`.
  * `fn compute_cost(n_bytes: u64, policy: &naming::manifest::Payment) -> Option<f64>`

    * Returns `None` when `policy.required == false`; else cost in `policy.currency`.
  * `fn validate_wallet_string(&str) -> anyhow::Result<()>` (presence/shape checks only).
  * `fn validate_payment_block(&Payment) -> anyhow::Result<()>` (model parseable, non-negative price, non-empty wallet).
* Events / HTTP / CLI: none.

## 5) Dependencies & Coupling

* Internal crates:

  * `naming` (for `manifest::Payment`): **loose/medium** coupling; used for type shape. Replaceable: **yes** (if `Payment` moves to `ron-proto`, switch imports).
* External crates (top):

  * `anyhow` (error bubble-up) — permissive, low risk; consider custom error to reduce footprint.
* Runtime services: none. No network, storage, OS, or crypto at runtime.

## 6) Config & Feature Flags

* Env vars / config structs: none.
* Cargo features: none (current); could add features later for wallet scheme validators (e.g., `lnurl`, `btc`, `eth`) without pulling heavy deps by default.

## 7) Observability

* No metrics or logs are emitted by this crate (pure functions).
* Guidance: callers (e.g., `svc-omnigate`) should instrument cost paths (e.g., `billing_cost_total`, `billing_cost_bytes_total`) if needed.

## 8) Concurrency Model

* None. Pure, synchronous functions; no tasks, channels, or retries.

## 9) Persistence & Data Model

* None. Operates on caller-supplied `Payment`; returns ephemeral results.
* No artifacts or retention.

## 10) Errors & Security

* Error taxonomy: `anyhow::Error` for validation failures (unknown `price_model`, empty wallet, negative price). All are **terminal** for the given input (no retry semantics inside this crate).
* Security: does **not** touch secrets; validation is shallow by design (does not authenticate wallets nor verify ownership).
* Risk notes:

  * Using `f64` for money invites rounding drift; caller must round/ceil per product policy before charging.
  * Accepts `wallet` as opaque; callers must not treat “validated” as “billable” without scheme-specific checks.

## 11) Performance Notes

* O(1) arithmetic; effectively zero overhead.
* `PerMiB` uses `n_bytes / (1024*1024)` as MiB; if you require decimal MB pricing, add an alternate model to avoid confusion.

## 12) Tests

* Current unit tests (expected baseline):

  * Cost computation for `PerMiB` and `Flat`; `required=false` returns `None`.
  * Payment block validation success path.
* Recommended additions:

  * Property tests across random sizes and prices (ensure monotonicity and non-negative results).
  * Edge cases: `n_bytes=0`, extremely large `n_bytes` (u64 near max), `price=0`, tiny `price` values (sub-cent).
  * Wallet heuristics per scheme (behind features) when added.
  * Ensure `compute_cost` is **pure** (same inputs → same output).

## 13) Improvement Opportunities

* **Monetary type:** introduce `Money`/`Decimal` (e.g., `rust_decimal`) and explicit **rounding mode** to avoid `f64` issues; or return integer minor units (cents/sats).
* **Model expressiveness:** add `PerByte`, `PerSecond`, `PerRequestPlusMiB`, and **caps/floors** (`min_charge`, `max_charge`) to match real billing.
* **Schema decoupling:** depend on `ron-proto::Payment` once it exists to centralize DTOs; keep this crate manifest-agnostic beyond shared types.
* **Wallet validators (opt-in):** feature-gate lightweight checks:

  * `lnurl`: bech32 prefix/length;
  * `btc`: base58/bech32 checksum length;
  * `eth`: `0x` + 40 hex with optional EIP-55 checksum;
  * `sol`: base58 length.
    All kept best-effort and non-networked.
* **Custom error enum:** replace `anyhow` with `BillingError` for clear caller behavior and no backtrace cost in hot paths.
* **Docs & examples:** add table examples for common sizes (e.g., 10 KiB, 1 MiB, 100 MiB) per model; call out rounding guidance.
* **Avoid hidden coupling:** don’t let business logic creep in (keep revenue-split math out; that belongs in a settlement layer).

## 14) Change Log (recent)

* 2025-09-14 — Extracted from `ryker` into its own crate; API preserved (`PriceModel`, `compute_cost`, `validate_*`) for compatibility.

## 15) Readiness Score (0–5 each)

* API clarity: **4** (tiny, obvious; add docs/rounding guidance)
* Test coverage: **3** (good unit coverage baseline; add properties/edge cases)
* Observability: **1** (none by design; fine for a pure lib)
* Config hygiene: **5** (no env/features yet; simple)
* Security posture: **4** (no secrets; shallow wallet checks—document limits)
* Performance confidence: **5** (O(1) math)
* Coupling (lower is better): **3** (depends on `naming::manifest::Payment`; move to `ron-proto` later)
