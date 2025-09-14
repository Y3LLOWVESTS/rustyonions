---

crate: common
path: crates/common
role: lib
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
-------------------------

## 1) One-liner

Shared foundation crate: hashing (BLAKE3), address formatting/parsing, `NodeId`, and a small `Config` loader used across services.

## 2) Primary Responsibilities

* Provide stable utilities: BLAKE3 helpers (`b3_hex`, `b3_hex_file`), address helpers (`format_addr`, `parse_addr`, `shard2`), and `NodeId`.
* Define a repo-wide `Config` with defaults and disk loader (TOML/JSON).
* Re-export commonly used helpers so higher layers don’t duplicate logic.

## 3) Non-Goals

* No networking, async runtime, or service hosting.
* No metrics/HTTP endpoints; no persistence beyond reading config files.
* No cryptographic key management beyond hashing helpers.

## 4) Public API Surface

* **Re-exports:** `b3_hex`, `b3_hex_file`, `format_addr`, `parse_addr`, `shard2`.
* **Key types / functions / traits:**

  * `NodeId([u8; 32])` (`Serialize`, `Deserialize`, `Clone`, `Copy`, `Eq`, `Hash`)

    * `NodeId::from_bytes(&[u8]) -> Self` (BLAKE3 of input → 32 bytes)
    * `NodeId::from_text(&str) -> Self`
    * `NodeId::to_hex(&self) -> String`
    * `NodeId::as_bytes(&self) -> &[u8; 32]`
    * `impl FromStr for NodeId` (expects 32-byte hex; errors otherwise)
    * `impl fmt::Debug` prints hex
  * `Config` (serde-friendly)

    * Fields:
      `data_dir: PathBuf`,
      `overlay_addr: SocketAddr`,
      `dev_inbox_addr: SocketAddr`,
      `socks5_addr: String`,
      `tor_ctrl_addr: String`,
      `chunk_size: usize`,
      `connect_timeout_ms: u64`,
      `hs_key_file: Option<PathBuf>`
    * `impl Default` with sane localhost defaults (9050/9051, etc.)
    * `Config::connect_timeout() -> Duration`
    * `Config::load(path: impl AsRef<Path>) -> anyhow::Result<Self>` (TOML or JSON)
  * Hash/address helpers (in `hash` module):

    * `b3_hex(bytes: &[u8]) -> String` (64-hex)
    * `b3_hex_file(path: &Path) -> io::Result<String>` (streams with 1 MiB buffer)
    * `format_addr(hex64: &str, tld: &str, explicit_algo_prefix: bool) -> String`

      * Yields `b3:<hex64>.<tld>` when `explicit_algo_prefix` is true; otherwise `<hex64>.<tld>`
    * `parse_addr(s: &str) -> Option<(String /*hex64*/, String /*tld*/)>`

      * Accepts with or without `b3:` prefix; lowercases hex; validates 64 hex chars
    * `shard2(hex64: &str) -> &str` (first two hex chars for sharding)
* **Events / HTTP / CLI:** None.

## 5) Dependencies & Coupling

* **Internal crates:** none (intentionally base/leaf). Replaceable: **yes**.
* **External crates (top 5; via `Cargo.toml`):**

  * `blake3` — hashing; mature, Apache/MIT; low risk.
  * `serde`, `serde_json`, `toml` — config and DTOs; very stable; low risk.
  * `anyhow` — ergonomic error contexts at API edges; low risk.
  * `hex` — encoding; tiny, stable.
  * `thiserror` — available for typed errors (not heavily used yet).
  * (`workspace-hack` is build-graph hygiene only.)
* **Runtime services:** Filesystem only (config read; optional file hashing). No network or crypto services.

## 6) Config & Feature Flags

* **Env vars:** none.
* **Config structs:** `Config` (see fields above). `Default` uses localhost for overlay/inbox, Tor SOCKS (9050) and control (9051), `chunk_size = 1<<16`, and `connect_timeout_ms = 5000`.
* **Cargo features:** none.
* **Effect:** Deterministic behavior; TOML/JSON auto-detect in `Config::load`.

## 7) Observability

* None built-in (no metrics/log/health). Intended to be imported by services that expose metrics/logging.

## 8) Concurrency Model

* None; purely synchronous helpers and types. No internal threads, channels, or locks.

## 9) Persistence & Data Model

* **Persistence:** None (reads config from disk when asked).
* **Data model:** Simple types (`NodeId`, `Config`) and address strings; address sharding via first two hex chars.

## 10) Errors & Security

* **Errors:** `anyhow::Error` for `Config::load` (read/parse issues). `NodeId::from_str` enforces exact 32-byte hex. Address parsing returns `Option` for invalid formats.
* **Security:** No secrets, no TLS. Hashing is BLAKE3 (not a password hash; fine for content addressing).
* **PQ-readiness:** N/A (no key exchange or signatures here).

## 11) Performance Notes

* `b3_hex`/`b3_hex_file` are O(n) with minimal overhead; file hashing uses a 1 MiB buffer to reduce syscalls.
* `shard2` is constant time; address format/parse do small string ops.
* Suitable for hot paths in overlay/gateway; no global state or locks.

## 12) Tests

* **In-crate:** none in this archive.
* **Recommended additions:**

  * Golden tests for `parse_addr`/`format_addr` (with/without `b3:`; invalid hex; bad TLD).
  * `Config::load` round-trips for TOML/JSON; missing fields → defaults or errors as intended.
  * `NodeId` determinism (bytes/text) and `FromStr` failure modes.
  * `b3_hex_file` on small/large files (0B, 1B, >1 MiB).

## 13) Improvement Opportunities

* **Typed address:** Introduce a `struct ObjAddr { hex64, tld }` with `Display/FromStr` to replace raw tuples and reduce mistakes.
* **Strengthen parsing:** Validate TLD charset (e.g., `[a-z0-9]{1,16}`) and reject mixed-case hex; already lowercased but codify constraints.
* **Error taxonomy:** Use `thiserror` for `ConfigError`, `AddrParseError` instead of `anyhow` for library callers.
* **Docs/examples:** Add rustdoc examples for all public fns; explain sharding convention and `b3:` prefix semantics.
* **Config cohesion:** Consider a `common::prelude` and/or a single `Config` read path (env var for path, `RO_CONFIG`), plus `Display` for human-friendly dumps.
* **Test clock/file abstractions:** Not needed here except to make file hashing tests cleaner (temp files) and config loading predictable.

## 14) Change Log (recent)

* 2025-09-14 — Added `Config::load` (TOML/JSON), `Default` values; exported hash/address helpers; introduced `NodeId` with hex/parse helpers.

## 15) Readiness Score (0–5 each)

* **API clarity:** 3 — Compact and sensible; add rustdoc + typed address to reach 4–5.
* **Test coverage:** 1 — No in-crate tests yet.
* **Observability:** 0 — By design (library only).
* **Config hygiene:** 4 — Defaults + loader are clean; could add env override and stricter validation.
* **Security posture:** 4 — No secrets; safe helpers; clarify non-use of hashing for authentication.
* **Performance confidence:** 4 — Tiny, CPU-bound hashing/string ops; unlikely bottleneck.
* **Coupling (lower is better):** 1 — Leaf utility crate; widely depended on, but itself loosely coupled.
