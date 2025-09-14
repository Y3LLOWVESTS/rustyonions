---

crate: naming
path: crates/naming
role: lib
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
-------------------------

## 1) One-liner

Defines the **canonical content address syntax** for RustyOnions (`b3:<64-hex>.<tld>`), plus helpers to parse/validate/normalize addresses and to read/write **Manifest v2** metadata.

## 2) Primary Responsibilities

* **Addressing:** Parse and format RustyOnions addresses (BLAKE3-256 only), normalize case/prefix, and verify hex length/characters.
* **Typing:** Enumerate and parse **TLD types** (e.g., `image`, `video`, `post`, …) to keep addresses semantically scoped.
* **Manifests:** Provide a stable **Manifest v2** schema and a writer for `Manifest.toml` (core fields, optional encodings/payments/relations/ext).

## 3) Non-Goals

* **No resolution** from address → providers (that’s the index service).
* **No byte serving/storage** (overlay/storage handle bytes).
* **No network/API** surface; this is a pure library with light filesystem I/O for manifest writing only.

## 4) Public API Surface

* **Re-exports:** none currently.
* **Key types / functions / traits:**

  * `hash` module:

    * `const B3_LEN: usize = 32`, `const B3_HEX_LEN: usize = 64`
    * `fn b3(bytes: &[u8]) -> [u8; 32]`
    * `fn b3_hex(bytes: &[u8]) -> String`
    * `fn parse_b3_hex(s: &str) -> Option<[u8; 32]>`
  * `tld` module:

    * `enum TldType { Image, Video, Audio, Post, Comment, News, Journalist, Blog, Map, Route, Passport, ... }`
    * `impl FromStr` and `Display` with lower-case mapping; `TldParseError`
  * **Address** (duplicated—see “Improvement Opportunities”):

    * In `address.rs`: `struct Address { hex: String, tld: TldType }` with `impl FromStr` expecting `<hex>.<tld>` and `AddressParseError`.
    * In `lib.rs`: a second `struct Address { hex: String, tld: String }` with comments allowing optional `b3:` and canonical `Display` as `b3:<hex>.<tld>`.
  * `manifest` module:

    * `struct ManifestV2 { schema_version, tld, address, hash_algo, hash_hex, bytes, created_utc, mime, stored_filename, original_filename, encodings[], payment?, relations?, license?, ext{} }`
    * `struct Encoding { coding, level, bytes, filename, hash_hex }`
    * `struct Payment { required, currency, price_model, price, wallet, ... }`
    * `struct Relations { reply_to?, thread?, source?, ... }`
    * `fn write_manifest(dir: &Path, &ManifestV2) -> Result<PathBuf>`
* **Events / HTTP / CLI:** none.

## 5) Dependencies & Coupling

* **Internal crates:** none required at compile time (good isolation). Intended consumers: `gateway`, `svc-index`, `svc-overlay` (loose coupling).
* **External crates (top 5 and why):**

  * `blake3` — content addressing; **low risk**, active.
  * `serde`, `thiserror`, `anyhow` — data model & errors; **low risk**, standard.
  * `toml` — manifest encoding; **low risk**.
  * `mime` / `mime_guess` — present in `Cargo.toml`, likely for future MIME derivation; **currently unused** (see debt).
  * `uuid`, `chrono`, `base64` — also declared; **currently unused** in visible code (drift).
* **Runtime services:** none (pure CPU & small FS writes for manifests). No network/crypto services beyond BLAKE3 hashing.

## 6) Config & Feature Flags

* **Env vars:** none.
* **Cargo features:** none declared. (Opportunity: feature-gate optional `mime_guess`, `uuid`, etc., or remove until used.)

## 7) Observability

* No metrics/logging inside the crate (appropriate for a pure library). Downstream services should tag failures when parsing/validating addresses or manifests.

## 8) Concurrency Model

* Not concurrent; pure functions and data types. No async, no background tasks. Thread-safe by construction (no globals).

## 9) Persistence & Data Model

* **Address:** `b3:<64-hex>.<tld>` canonical form (lower-case hex, lower-case TLD). Accepting input with or without `b3:` is implied by comments, but behavior diverges between the two `Address` impls (see below).
* **Manifest v2:** TOML document with **stable core** (schema\_version=2, tld, address, hash, size, mime, filenames, timestamp) and **optional sections**:

  * `encodings[]` (e.g., zstd/br precompressions)
  * `payment?` (simple micropayment hints)
  * `relations?` (reply/thread/source)
  * `license?` (SPDX or freeform)
  * `ext{}` (namespaced TLD-specific extras)
* **Retention/Artifacts:** `write_manifest()` writes `Manifest.toml` into a bundle directory; no reads provided yet (writer-only API).

## 10) Errors & Security

* **Errors:**

  * `AddressParseError { Invalid, Tld(TldParseError) }`
  * `TldParseError { Unknown(String) }`
  * `manifest::write_manifest` returns `anyhow::Error` on serialization/FS failures.
* **Security posture:**

  * Strict hash length and hex-char checks (`parse_b3_hex`) and lower-case normalization reduce ambiguity.
  * No signatures/PKI here (by design). No unsafe code.
  * Recommend constant-time equality when comparing digests (callers).

## 11) Performance Notes

* Hot paths are string parsing and hex validation; all O(n) over short inputs—**negligible** vs I/O.
* BLAKE3 helpers are thin wrappers; rely on upstream performance.

## 12) Tests

* Present: `hash` unit tests (hex length, lowercase guarantee).
* Missing/needed:

  * `Address` round-trips (with and without `b3:` prefix; mixed case input).
  * TLD parsing (valid set + unknown TLD rejection).
  * Manifest round-trip: build → write → parse (when a reader is added).
  * Property tests: random hex strings of non-64 length must be rejected; case normalization.

## 13) Improvement Opportunities

### A) **Eliminate duplication and drift (highest priority)**

* There are **two different `Address` definitions**:

  * `address.rs`: `tld: TldType` with structured TLD + strong parse errors.
  * `lib.rs`: `tld: String` with comments about accepting `b3:`; uses `anyhow`.
* **Action:** Keep **one canonical `Address`** in `address.rs` (typed TLD), re-export it from `lib.rs`, and implement:

  * `impl Display` → `b3:<hex>.<tld>`
  * `impl FromStr` that accepts **both** `b3:<hex>.<tld>` and `<hex>.<tld>` and normalizes to canonical.
  * `fn parse_loose(s) -> Result<Address, AddressParseError>` (optional) if we need a forgiving parser for UX, else keep `FromStr` strict with optional prefix allowed.
* Add `pub mod tld; pub mod hash; pub mod address; pub mod manifest;` in `lib.rs` and remove the stray `Address` duplicate there.

### B) **Tighten validation & helpers**

* Provide `Address::from_bytes(tld: TldType, payload: &[u8]) -> Address` using `b3_hex`.
* Provide `Address::verify_payload(&self, payload: &[u8]) -> bool` (hash checks).
* Add `fn validate_manifest(v: &ManifestV2) -> Result<()>` to ensure:

  * `v.hash_algo == "b3"`, `len(hash_hex)==64`, lowercase.
  * `v.address == format!("b3:{hash_hex}.{tld}")`.
  * `mime` aligns with `tld` (soft check) using `mime_guess` if we keep it.

### C) **Dependencies hygiene**

* Remove or feature-gate currently unused deps (`uuid`, `chrono`, `mime`, `mime_guess`, `base64`).
* If we keep `chrono`, switch `created_utc` to `DateTime<Utc>` in API and serialize to RFC3339 via serde (or keep as `String` but justify).

### D) **Manifest ergonomics**

* Add `fn read_manifest(path: &Path) -> Result<ManifestV2>`.
* Provide builders: `ManifestV2::new_basic(addr, bytes, mime, filenames...)` and `with_encoding(...)`, `with_payment(...)`, etc.
* Document the `ext{}` namespace conventions per TLD (short guide in README).

### E) **MIME ↔ TLD policy**

* If MIME is involved, create a small mapping:

  * e.g., `image/* → TldType::Image`, `video/* → Video`, `audio/* → Audio`, `application/geo+json → Map/Route`, etc.
* Keep it **non-authoritative** (advisory only), exposed via helper fns.

### F) **Consistency & docs**

* README shows intent (“gateway/overlay use canonical form”) but is truncated; expand with examples:

  * Valid/invalid addresses, acceptance of `b3:` prefix, and canonical rendering.

## 14) Change Log (recent)

* **2025-09-14** — First deep review; identified **Address type duplication** and unused dependencies; documented Manifest v2 structure and validation needs.
* **2025-09-13..09-06** — (Implied) Iterations on TLD set and manifest fields; added `encodings`, `payment`, `relations`, and `ext` blocks.

## 15) Readiness Score (0–5 each)

* **API clarity:** 2 — useful pieces exist, but **duplicate `Address`** types and missing `lib.rs` re-exports create confusion.
* **Test coverage:** 2 — some `hash` tests; missing address/TLD/manifest round-trips.
* **Observability:** 3 — not needed here; downstream services should log failures.
* **Config hygiene:** 2 — unused deps suggest drift; no features to scope them.
* **Security posture:** 4 — strict hex checks, no unsafe; recommend adding manifest/address cross-validation helpers.
* **Performance confidence:** 5 — tiny string/hex ops; BLAKE3 is fast; no hot-path risks.
* **Coupling (lower is better):** 1 — clean, standalone; consumed by multiple services without runtime ties.

