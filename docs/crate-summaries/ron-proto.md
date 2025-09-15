# ron-proto

---

crate: ron-proto
path: crates/ron-proto
role: lib
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
-------------------------

## 1) One-liner

Single source of truth for cross-service DTOs/errors, addressing (BLAKE3-based digests), and OAP/1 protocol constants with JSON/optional MessagePack codecs.

## 2) Primary Responsibilities

* Define stable protocol DTOs and error types shared across services.
* Provide addressing primitives (e.g., `B3Digest`) and canonical constants (`OAP1_MAX_FRAME = 1 MiB`, `STREAM_CHUNK = 64 KiB`).
* Offer wire helpers for JSON and optional rmp-serde encoding/decoding.

## 3) Non-Goals

* Cryptography (key storage, signing, verification) and envelope semantics.
* Networking, persistence, or HTTP/CLI surfaces.
* Policy decisions (authn/z, key rotation).

## 4) Public API Surface

* Re-exports: none.
* Key types / functions:

  * Addressing: `B3Digest` (BLAKE3 digest newtype) and helpers (parse/format; hex `b3:` style).
  * DTO modules for planes/services (overlay, index, storage, gateway, etc.).
  * Error types for protocol/domain errors.
  * Wire: `wire::{to_json, from_json, to_msgpack, from_msgpack}` (MessagePack behind `rmp` feature).
* Events / HTTP / CLI: none.

## 5) Dependencies & Coupling

* Internal crates → none (intentionally decoupled; consumers depend on `ron-proto`).
* External crates (top 5; pins/features) → `serde` (derive), `rmp-serde` (optional), `blake3`, `hex`, `thiserror`.

  * Low risk, permissive licenses; minimal API churn expected.
* Runtime services: none (pure CPU/heap).

## 6) Config & Feature Flags

* Cargo features:

  * `rmp` (default): enable MessagePack (compact wire format); without it JSON-only.
* No environment variables; behavior is compile-time via features.

## 7) Observability

* None currently (no logs/metrics). Future counters: (de)serialize successes/failures per format.

## 8) Concurrency Model

* Pure value types; no interior mutability or async tasks. No backpressure concerns.

## 9) Persistence & Data Model

* None; defines wire/domain shapes only. Addressing uses BLAKE3 digests (`B3Digest`) and hex text form.

## 10) Errors & Security

* Error taxonomy: `ProtoError::{Serde, DeSerde, Unsupported}` (terminal for the attempted operation).
* Security:

  * No signing/envelopes here—kept in auth/KMS layers.
  * Digest use is explicit and algorithm-named (`B3Digest`) to avoid ambiguity and enable future upgrades.

## 11) Performance Notes

* Hot paths: (de)serialization and BLAKE3 digest computation for addressing.
* MessagePack recommended for compactness; JSON remains ergonomic for debugging.

## 12) Tests

* Unit: JSON/MsgPack round-trips for representative DTOs; digest parse/format round-trips.
* Future: property tests for DTO backward-compat behaviors and fuzzing decode paths.

## 13) Improvement Opportunities

* Add CBOR helpers (optional).
* Stronger ID newtypes (e.g., typed resource IDs) and schema doc generation.
* Compat shims for versioned DTO migrations (serde `#[serde(other)]` guards).

## 14) Change Log (recent)

* 2025-09-14 — Refactor: removed envelope/signing; adopted BLAKE3 addressing (`B3Digest`); added OAP/1 constants and clarified wire helpers.

## 15) Readiness Score (0–5 each)

* API clarity: 4
* Test coverage: 3
* Observability: 1
* Config hygiene: 5
* Security posture: 4 (clear boundaries; no crypto here)
* Performance confidence: 4
* Coupling (lower is better): 1
