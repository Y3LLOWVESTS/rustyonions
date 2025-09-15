# ron-kms

---

crate: ron-kms
path: crates/ron-kms
role: lib
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
-------------------------

## 1) One-liner

KMS trait + dev in-memory backend for origin key derivation (HKDF), secret sealing, and rotation hooks—kept separate from envelope/auth logic.

## 2) Primary Responsibilities

* Derive per-origin/per-instance keys from a node master key (HKDF) and expose stable `derive_origin_key(..)` semantics.
* Seal/unseal small secrets for services (dev in-memory backend now; pluggable later).
* Provide rotation/audit hooks (emit key lifecycle events via the bus/audit layer).

## 3) Non-Goals

* Envelope formats or request verification (lives in auth/verify crate or service edge).
* Long-term persistent key stores or cloud KMS integrations inside this core crate.
* Access control policy (authn/z) and multi-tenant boundary enforcement.

## 4) Public API Surface

* Re-exports: none.
* Key types / functions / traits:

  * `Kms` trait (core operations: derive, seal/unseal, list/rotate).
  * `InMemoryKms` dev implementation (non-persistent).
  * `derive_origin_key(origin, instance, epoch)` → stable key bytes/handle.
  * `seal(bytes) -> Sealed`, `unseal(Sealed) -> bytes`.
  * Events (conceptual): `KeyRotated`, `KeyDerived` (for audit stream; emitted via caller).
* Events / HTTP / CLI: none in core; a thin Axum service can be added behind a feature in a follow-up crate.

## 5) Dependencies & Coupling

* Internal crates → `ron-proto` (types/ids only, loose; replaceable with adapter: yes).
* External crates (top 5; pins/features) → `hkdf` (HKDF-SHA256), `sha2`, `zeroize` (for secret wiping), `parking_lot` (RwLock), `thiserror`.

  * Low maintenance risk; permissive licenses; small, stable APIs.
* Runtime services: OS RNG via `rand` (for nonces); otherwise none.

## 6) Config & Feature Flags

* Cargo features:

  * `core` (default): HKDF derivation + seal/unseal.
  * (Optional, off by default) `signing-hmac`, `signing-ed25519`: expose raw signing primitives if a caller insists, but envelope usage should live in auth.
* No environment variables. Backend selection is compile-time; future backends (e.g., `kms-file`, `kms-sled`, `kms-axum`) land as separate crates/features.

## 7) Observability

* Not yet implemented. Recommended metrics:

  * `kms_derivations_total{origin}`, `kms_seal_total{ok}`, `kms_unseal_total{ok}`, `kms_errors_total{kind}`.
  * Latency histograms for derive/seal/unseal.
  * Rotation counters (`kms_rotations_total{scope}`).

## 8) Concurrency Model

* Single process, in-memory store guarded by `Arc<RwLock<..>>`.
* Short critical sections (clone entry → operate outside lock).
* No async tasks/channels here; CPU-bound operations, no backpressure lane.
* Timeouts/retries: N/A (callers should implement retries & budgets).

## 9) Persistence & Data Model

* Ephemeral dev storage: `HashMap<KeyId, KeyEntry>` where `KeyEntry` carries algo + material.
* No schema/DB; production backends (sled/file/cloud) to be added as separate implementations.
* Retention/rotation: policy is caller-driven; KMS exposes hooks/ids to rotate and emits events.

## 10) Errors & Security

* Error taxonomy: `UnknownKey`, `Unsupported`, `Crypto`, `Policy` (for future rotation/ACL violations).
* Security posture:

  * HKDF for derivation (context-tagged info inputs: origin/instance/epoch).
  * `zeroize` on secret buffers where feasible; avoid long-lived clones.
  * KID strategy: prefer opaque random IDs (or public-key-derived IDs for asymmetric) to avoid leaking secret digests.
  * PQ-readiness: algo-agile boundary—future PQ KDFs/AEADs can be introduced without API breaks.
  * Authn/z is out-of-scope; assume trusted in-process callers or wrap with an authenticated service.

## 11) Performance Notes

* Hot paths: HKDF derive and small AEAD seal/unseal (when enabled by backend).
* Expected micro-latency per op; lock contention negligible given short RwLock scopes.
* Scaling: multiple independent KMS instances are cheap; persistent backends will define real throughput bounds.

## 12) Tests

* Unit: HKDF known-answer tests; seal/unseal round-trip; rotation sanity.
* Future: properties (derivations differ across origin/instance/epoch), fuzz unseal inputs, cross-impl vectors vs reference libs.

## 13) Improvement Opportunities

* Backends: `ron-kms-file` (JSON+age), `ron-kms-sled` (encrypted-at-rest), `ron-kms-cloud` (provider adapters).
* Stronger secret-handling: mandatory `zeroize`, `secrecy::SecretVec`, page-locking (where OS allows).
* Rotation policy helpers and scheduled rotation jobs.
* Prometheus metrics + structured tracing behind `observability` feature.
* Hardening docs: threat model, SOC2-style control mapping.

## 14) Change Log (recent)

* 2025-09-14 — Refactor: removed envelope/sign responsibilities; added HKDF-based origin key derivation and sealing surface; clarified feature gating.

## 15) Readiness Score (0–5 each)

* API clarity: 4
* Test coverage: 3
* Observability: 1
* Config hygiene: 5
* Security posture: 4 (good primitives; production hardening pending)
* Performance confidence: 4
* Coupling (lower is better): 2 (low; depends lightly on `ron-proto`)
