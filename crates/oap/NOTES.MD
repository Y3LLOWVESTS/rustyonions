### NOTES FOR oap - OCTOBER 24 2025 - 19:47 CST

Here’s a comprehensive snapshot of the **oap** crate—what we built, why it matters, how it hangs together, and what’s left to button up.

# OAP crate — comprehensive notes

## 1) What OAP is (scope & purpose)

* **Purpose:** Provide a **safe, bounded, versioned framing layer** (OAP/1) with a Tokio codec for stream transports. It’s the stable envelope used by SDKs/gateways/services inside RON-CORE.
* **Non-goals:** No transport management, auth semantics, or policy here. Capabilities (macaroons/scopes/etc.) are carried as **opaque bytes** on `START` frames; interpretation lives above OAP.

## 2) Protocol invariants implemented

* **Max frame size:** `MAX_FRAME_BYTES = 1 MiB` (hard bound, enforced in decode/encode).
* **Header layout:** fixed-size header (`Header::WIRE_SIZE`) with fields: `len, ver, flags, code, app_proto_id, tenant_id (u128), cap_len, corr_id`.
* **START + capabilities rule:** `cap_len > 0` is only legal when `Flags::START` is set (decoder rejects otherwise).
* **Compression flag:** `Flags::COMP` triggers optional **zstd** decompression (feature-gated, bounded to ≤ `MAX_DECOMPRESS_EXPANSION`).
* **No `unsafe` code:** crate-wide `#![forbid(unsafe_code)]`.

## 3) Public modules and their roles

* `constants.rs` — protocol-level bounds and version (`OAP_VERSION`), chunk sizes, etc.
* `flags.rs` — `bitflags` for OAP control bits (REQ/RESP/START/END/ACK_REQ/EVENT/COMP/APP_E2E).
* `header.rs` — read/write of the on-wire header (little parsing helpers).
* `frame.rs` — `Frame { header, cap: Option<Bytes>, payload: Option<Bytes> }`.
* `codec.rs` — **Tokio `Decoder`/`Encoder`** for `Frame`. Enforces size rules, capability bounds, and optional zstd inflate with strict output cap.
* `hello.rs` — minimal **HELLO** request/reply helpers for negotiation (`app_proto_id=0`), JSON-encoded payload.
* `envelope.rs` — user ergonomics:

  * `FrameBuilder` for common flows (request/response, START w/ cap, ACK, END, payload).
  * `Capability` wrapper (opaque bytes with u16-size check).
  * ACK/END/EVENT helpers: `wants_ack`, `is_terminal`, `is_fire_and_forget`.
  * Convenience helpers: `hello_request`, `hello_reply_default`.
* `metrics.rs` — **stable metrics vocabulary** (no direct Prometheus dep):

  * `OutcomeClass` (Success/ClientError/ServerError/DecodeError/Oversize),
  * `reason()`, `is_success/is_client_err/is_server_err`,
  * `outcome_from_status()`, `outcome_from_decode()`, `labels_for_outcome()`.
* `prelude.rs` — curated re-exports for ergonomic `use oap::prelude::*`.
* `seq.rs` — lock-free **monotonic correlation-id** generator (seeded).
* `parser/` — incremental parser facade for partial reads:

  * `ParserConfig { max_buffer_bytes: Option<usize> }` (soft cap),
  * `ParserState` with `push`, `try_next`, `drain`, `buffered_len`.
* `writer/` — buffered frame writer for async sinks:

  * `WriterConfig { flush_hint_bytes }` defaulting to stream chunk size,
  * `OapWriter` with `encode_to_buf`, `take_buf`, `write_frame`, `flush`.

## 4) Error taxonomy (stable, typed)

* `StatusCode` (narrow HTTP-like set): 200, 206, 400/401/403/404, 413, 429, 500, 503.
* `OapDecodeError`: truncated/bad header, bad flags, wrong START/cap usage, oversize, bounds, zstd errors, and `Io(#[from] std::io::Error)`.
* `OapEncodeError`: oversize, cap OOB, `Io`.
* `OapError`: sum-type of decode/encode; plus `From<std::io::Error>` to allow `?` from async I/O in writer.

## 5) Feature flags

* `zstd` (optional): bounded inflate enabled only when requested. Without it, any `COMP` frame yields `ZstdFeatureNotEnabled`.

## 6) Tests (now substantial and green)

* **Vectors & symmetry:** `tests/vectors.rs` cover hello/hello-reply/start-with-cap/data/no-cap and oversize. The tests normalize expected headers (`len`, `cap_len`) to match encoder’s canonicalization.
* **ACK algebra:** `ack_algebra.rs` validates `wants_ack`, `is_terminal`, `is_fire_and_forget` with representative frames.
* **Config & parser:** `config_validation.rs` triggers parser soft-cap; minimal builder sanity.
* **Conformance:** `conformance.rs` checks header size invariant and `cap_requires_start_flag`.
* **Metrics mapping:** `metrics_mapping.rs` checks outcome classification and labels; decode error mapping (oversize & bad flags).
* **Partial reads:** `parser_partial_read.rs` feeds small chunks and drains two frames.
* **Writer roundtrip:** `writer_roundtrip.rs` uses `OapWriter` then decodes back to a frame.
* **Seq monotonicity:** `seq_monotonic.rs` validates uniqueness across a sample.
* **Split-need-more semantics:** `split_need_more.rs` ensures decoder returns `None` before full header, then succeeds.

> Result: `cargo test -p oap` passes in your workspace with and without `--features zstd`.

## 7) Benches (present)

* You mentioned existing bench scripts: `decode_happy.rs`, `decode_pathological.rs`, `encode_ack.rs`. They remain; wiring to Criterion is optional and still TBD (see “What’s left”).

## 8) API ergonomics & exports

* `lib.rs` exports core types and TODO-aligned modules **plus** parser/writer:

  * Re-exports of helpers in `envelope` and `metrics`.
  * `prelude::*` includes codec, header/frame, helpers, constants, seq.
  * Parser/writer exports: `ParserState`, `ParserConfig`, `OapWriter`, `WriterConfig`.

## 9) Security posture

* **No `unsafe`**, strict bounds on decode/encode, bounded decompression with expansion guard, capability bytes treated as opaque (no parsing here). DTOs for higher layers (e.g., `ron-proto`) are expected to use strict serde settings (e.g., `deny_unknown_fields`)—outside this crate.

## 10) Performance posture

* Based on `bytes`/`BytesMut`; encode avoids unnecessary copies; decode zero-copies header re-parse from the split frame. Decompression is bounded and chunked (16 KiB temp buf). Writer uses a configurable flush hint (default 64 KiB stream chunk).

---

# What we still need / recommended next steps

## A) Documentation polish

* **Module-level docs**: Add example snippets to `envelope`, `parser`, `writer` so `cargo doc` is copy-paste friendly.
* **README**: crate-local `README.md` (or keep `README.snip.md`) summarizing invariants, feature flags, and quickstart examples.
* **Doc cfg badges:** reflect `zstd` feature in docs (e.g., `#[cfg_attr(feature="zstd", doc = "…")]`).

## B) CI & quality gates

* **Clippy & fmt gates:** Ensure `cargo clippy --all-features -D warnings` and `cargo fmt --all --check` pass in CI.
* **MSRV policy (if any):** Document and test-minimal Rust version in CI.

## C) Criterion benches (optional but valuable)

* Convert `decode_happy.rs`, `decode_pathological.rs`, `encode_ack.rs` into Criterion benches in `benches/` with parameterized frame sizes (4 KiB → 1 MiB), with and without `zstd`.
* Track throughput and allocations; confirm no regression as crate evolves.

## D) Fuzzing (strong safety ROI)

* Add `cargo-fuzz` target(s):

  * **`decode_frame`**: feed arbitrary bytes into `OapDecoder::decode`; assert it never panics and respects bounds.
  * **`header_roundtrip`**: structured fuzz for header read/write.
* Add a **corpus** seeded with our golden vectors and pathological cases (e.g., huge `len`, `cap_len > len`, `FLAGS:COMP` with junk data).

## E) More conformance/golden vectors

* RESP|END with **empty payload**.
* EVENT with/without `ACK_REQ` and with payload.
* START with **large but valid** capability (near `u16::MAX`) to prove boundary behavior.
* COMP payload that expands near the expansion limit (negative test for `DecompressBoundExceeded`).

## F) Integration points (future)

* **ron-metrics**: concrete adapter mapping `metrics::labels_for_outcome` to metric counters/histograms. (Right now we only expose the labels.)
* **ron-proto**: illustrative DTOs (serde) and a tiny example that packs/unpacks them inside `Frame.payload`—still keeping OAP opaque to specific schema.
* **ron-bus / ron-kernel**: confirm the OAP framing integrates with the bus and kernel abstractions (naming and flags alignment).

## G) API stability & versioning

* Consider `#[non_exhaustive]` for error enums if we expect to add variants.
* Promote a `CHANGELOG.md` and semver policy (OAP/1 vs crate `0.x` API).
* Review `prelude` content to ensure it’s stable and minimal (avoid re-export churn).

## H) Backpressure & buffering semantics

* The parser’s **soft cap** currently returns `PayloadOutOfBounds` as a signal. Decide:

  * Keep as-is (documented) *or*
  * Add a dedicated error variant (e.g., `BufferSoftCapExceeded`) for clarity.
* Writer’s flush policy: document that **flush happens on size**, not time; encourage callers to call `flush()` at natural boundaries.

## I) Examples (dev UX)

* `examples/`:

  * **producer_consumer.rs**: Duplex stream; `OapWriter` on one end, `ParserState` on the other.
  * **hello_roundtrip.rs**: send HELLO → receive HELLO reply with defaults.

## J) Publish readiness (if we plan to)

* Ensure `Cargo.toml` metadata (description, license files, categories) is correct.
* Optional: feature docs in `Cargo.toml` (what `zstd` does).
* Verify `cargo publish --dry-run` in CI.

## K) Cross-crate references

* Audit any references to external blueprints/notes (e.g., RON-CORE, HARDENING, SIX_CONCERNS). Ensure **comments match** our final invariants and we’re not promising features we don’t ship here.

---

# TL;DR

* ✅ **Functional & tested**: codec, frames, flags, header, hello helpers, envelope builder, metrics helpers, seq, parser, writer.
* ✅ **Safety**: strict bounds; no `unsafe`; optional bounded inflate; START/cap rules enforced.
* ✅ **Tests**: coverage across invariants, ACK algebra, metrics mapping, partial reads, writer roundtrip, seq monotonicity, golden vectors.
* 🔜 **Next**: docs polish, CI gates, benches→Criterion, fuzzing, more vectors, examples, metrics adapter, integration with `ron-metrics`/`ron-bus`/`ron-kernel`, and publish hygiene.


