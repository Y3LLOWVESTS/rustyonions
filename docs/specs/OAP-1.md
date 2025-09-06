# OAP/1 — RustyOnions Object Access Protocol (Stub, locked defaults)

_Status: Draft/Locked Defaults • Last updated: 2025-09-05 • Scope: wire-level defaults and invariants_

This document fixes the **normative** defaults for OAP/1 as used by the RustyOnions services plane. It is intentionally small.
Implementation details (chunk sizes, backpressure strategies, etc.) are **not** part of the wire protocol.

## 1) Normative defaults (MUST)
- **Framing:** length‑prefixed frames over a reliable byte stream (e.g., TCP/TLS).
- **Frame types:** `HELLO`, `START`, `DATA`, `END`, `ACK`, `ERROR`.
- **Default max frame size:** **`max_frame = 1 MiB`** (negotiable downward via `HELLO`).
- **Object identity in DATA:** The DATA frame **header MUST include** `obj:"b3:<hex>"`,
  where `<hex>` is the 64‑hex **BLAKE3‑256** digest of the **plaintext body**.
- **No per‑frame hashing:** Frames themselves are not hashed; content authenticity is enforced by verifying `b3:<hex>` against the assembled object bytes.
- **Case and encoding:** `b3:<hex>` is lowercase hex; headers use UTF‑8 JSON objects.

## 2) Frame layouts (logical)
All frames are: `u32 length | u8 type | payload`. The `payload` is a UTF‑8 JSON document unless noted.

- **HELLO**
  - Client → Server.
  - Example: `{ "proto":"OAP/1", "accept_max_frame": 1048576 }`
  - Server MAY respond with `{ "ok":true, "max_frame":1048576 }` (or lower).

- **START**
  - Signals the beginning of a logical object transaction or mailbox request.
  - Example: `{ "op":"get", "addr":"b3:<hex>.tld", "corr_id":"abc123" }`

- **DATA**
  - **Header+Body packing**: payload is a JSON header followed by **raw bytes**:
    - `payload = json_header_bytes || body_bytes`
    - Header **must** include: `{"obj":"b3:<hex>", "offset":0, "total_len":N}`
  - The stream of `DATA` frames carries the full body in order.

- **END**
  - Signals end of the current operation. Example: `{ "status":"ok" }`

- **ACK**
  - Acknowledges receipt in mailbox flows, or confirms server receipt. Example: `{ "ok":true }`

- **ERROR**
  - Error envelope (see ErrorEnvelope.md). Example:
    `{ "code":"quota_exhausted", "message":"...", "retryable":true, "corr_id":"..." }`

## 3) Object integrity
- Receivers **MUST** verify that the reconstructed body’s BLAKE3‑256 equals the `obj` header’s `<hex>`.
- If the digest does not match, the server **MUST** send `ERROR{ code:"bad_digest", ... }` and abort the stream.

## 4) Negotiation
- Only **`max_frame`** is negotiated in OAP/1 M1. Implementations MAY choose a smaller bound but not larger than 1 MiB, unless explicitly configured out-of-band.

## 5) Examples (non‑normative)
- DATA header example (JSON, pretty-printed):
```json
{
  "obj": "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
  "offset": 0,
  "total_len": 12345,
  "content_type": "application/octet-stream"
}
```
- START GET example:
```json
{ "op":"get", "addr":"b3:abcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcd.text", "corr_id":"r-42" }
```

## 6) Security & robustness notes
- Apply server‑side quotas per tenant before accepting large frames.
- Bound `json_header_bytes` parsing to `max_frame`.
- Enforce idle/read/write timeouts consistent with service config.
- Reject compression bomb candidates early (size/ratio guards) with `ERROR{ code:"rejected", reason:"compression_bomb" }`.

## 7) Drift control
- CI **MUST** grep for `max_frame = 1 MiB` in this document to prevent accidental drift.
- CI **MUST** grep for `obj:"b3:<hex>"` in examples/tests to ensure addressing remains BLAKE3‑256.
