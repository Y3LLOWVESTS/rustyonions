---

crate: ron-bus
path: crates/ron-bus
role: lib
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
-------------------------

## 1) One-liner

Tiny IPC layer for RustyOnions services: MessagePack-framed request/response envelopes over Unix Domain Sockets with minimal helpers for `listen / recv / send`.

## 2) Primary Responsibilities

* Define the **wire envelope** (`Envelope`) and a few shared reply shapes (e.g., `Ok`, `Bytes`, `NotFound`, `Err`).
* Provide **UDS framing** (length-prefixed MsgPack) and **blocking** helpers: `listen(sock)`, `recv(&mut UnixStream)`, `send(&mut UnixStream, &Envelope)`.
* (Today) Centralize **service RPC enums** (e.g., `StorageReq/Resp`, `IndexReq/Resp`, `OverlayReq/Resp`) so services share one protocol surface.

## 3) Non-Goals

* No async runtime integration (helpers are **blocking** on std UDS).
* No transport security (UDS only; **no TLS**; no network sockets).
* No authorization/verification of capability tokens (only carries `token: Vec<u8>`; verification is a service concern).
* Not the in-process **kernel bus** (`ron-kernel::Bus`); this is **inter-process** IPC.

## 4) Public API Surface

* **Re-exports:** none.
* **Key types / functions / traits:**

  * `pub struct Envelope { service: String, method: String, corr_id: u64, token: Vec<u8>, payload: Vec<u8> }`
  * `pub const RON_BUS_PROTO_VERSION: u32 = 1;`
  * `pub struct CapClaims { sub: String, ops: Vec<String>, exp: u64, nonce: u64, sig: Vec<u8> }` (carried inside `token` as MsgPack, not enforced here)
  * UDS helpers (blocking, Unix-only):

    * `pub fn listen(sock_path: &str) -> io::Result<UnixListener>`
    * `pub fn recv(stream: &mut UnixStream) -> io::Result<Envelope>`
    * `pub fn send(stream: &mut UnixStream, env: &Envelope) -> io::Result<()>`
* **Events / HTTP / CLI:** none.

## 5) Dependencies & Coupling

* **Internal crates:** none (good); however, this crate currently **hosts other services’ RPC enums**, which *functionally* couples `svc-index`, `svc-overlay`, `svc-storage`, `gateway`, etc., to this crate’s releases.

  * Stability: **tight** (any RPC enum change fans out here). Replaceable? **Yes**, with a `ron-proto` split (see §13).
* **External (top 5):**

  * `serde` (workspace) — ubiquitous encoding; low risk.
  * `rmp-serde` (workspace) — MessagePack codec; stable; low risk.
  * `thiserror` (workspace) — error ergonomics; low risk.
  * `workspace-hack` — dep dedupe shim; no runtime impact.
* **Runtime services:** **OS UDS** only; no network, no TLS, no crypto primitives.

## 6) Config & Feature Flags

* No cargo features today.
* No config structs; callers pass socket paths. Services define their own defaults (e.g., `/tmp/ron/svc-storage.sock`).

## 7) Observability

* None built-in. No metrics, tracing, or structured error mapping at the IPC layer.
* Errors bubble as `io::Error` or MsgPack decode errors from `rmp-serde`.

## 8) Concurrency Model

* **Blocking I/O** on `std::os::unix::net::UnixStream`.
* Framing: **u32 length prefix** + MsgPack body; `recv` uses a manual `read_exact` loop; `send` writes a length then bytes.
* **Backpressure:** entirely at the OS socket buffer and the caller’s accept loop; **no internal queues**, limits, or timeouts here.
* **Locks/timeouts/retries:** none implemented; callers must add deadlines and retry policies.

## 9) Persistence & Data Model

* None. No DB integration; IPC envelopes are ephemeral. Any persistence is a service concern.

## 10) Errors & Security

* **Error taxonomy:** thin; essentially `io::Error` for socket and codec failures. Reply conveniences include `NotFound` and `Err{err}` variants but enforcement is up to services.
* **Security:** relies on **process boundary + UDS permissions**. `Envelope.token` can carry `CapClaims` (MsgPack) but this crate does **not** verify signatures/expiry.
* **AuthZ/AuthN:** out of scope here; expected to happen in the service handler.
* **TLS/PQ:** N/A at this layer (local IPC only).

## 11) Performance Notes

* **Hot path:** `recv` → decode → handler → `send`.
* Blocking UDS is fast on a single box, but mixing this inside async services (Axum/Tokio) risks **runtime stalls** unless calls stay off the async threads (use `spawn_blocking`).
* **Serialization:** MessagePack via `rmp-serde` is compact; consider zero-copy `bytes::Bytes` for `payload` to avoid copies on large blobs.
* **Targets (suggested):** p95 `recv+decode` < 30µs, p95 `send+encode` < 40µs for small (≤1 KiB) messages on dev hardware.

## 12) Tests

* **Present:** none visible in the crate; callers (e.g., `svc-storage`, `svc-overlay`) use it in their integration flows.
* **Needed:**

  * Unit: round-trip encode/decode; negative tests for short read, truncated prefix, oversized len.
  * Property/fuzz: random byte streams → ensure graceful decode errors without panics or leaks.
  * Loom: not necessary (blocking I/O), but an invariants test for “no partial frame read returns success” is useful.

## 13) Improvement Opportunities

### Known gaps / tech debt

* **Naming collision / drift risk:** We now have two “bus” concepts:

  * `ron-kernel::Bus` = **in-process broadcast** (Tokio broadcast for `KernelEvent`).
  * `crates/ron-bus` = **inter-process IPC** over UDS.
    This routinely confuses readers and encourages scope creep.

* **Coupling hotspot:** `ron-bus` currently **owns service RPC enums** (Index/Overlay/Storage…). That forces every service-level protocol change through this crate and blurs responsibilities.

* **No timeouts / limits / metrics:** `recv` can block indefinitely; there is no max-frame guard; no `rejected_total{reason}` counters; no structured tracing.

* **Blocking only:** In async services, blocking UDS calls must be quarantined to avoid starving the runtime.

### Streamlining (merge/extract/replace/simplify)

1. **Rename & split (strongly recommended):**

   * Rename this crate to **`ron-ipc`** (or `ron-uds`) to kill the “bus” ambiguity.
   * **Extract all RPC enums** into a new **`ron-proto`** crate with submodules (`proto::index`, `proto::overlay`, `proto::storage`), or one crate per service if we want looser coupling. Keep `ron-ipc` transport-only.

2. **Guards & observability:**

   * Add **max frame size** (e.g., 1 MiB default; configurable by caller); reject with a typed error.
   * Add **read/write timeouts** helpers (or document that callers must wrap with `nix`/`poll`/deadline I/O).
   * Optional **Prometheus hooks**: counters for `ipc_bytes_{in,out}_total`, `ipc_frames_total{dir,service}`, `ipc_decode_errors_total{reason}`.

3. **Async interop:**

   * Offer **Tokio variants** behind a feature flag (`tokio-uds`) using `tokio::net::UnixListener/UnixStream` (no behavior changes).
   * Document that blocking helpers should be used from dedicated threads or `spawn_blocking`.

4. **Versioning on wire:**

   * Today we have `RON_BUS_PROTO_VERSION` but do not put it on the wire. Add a fixed **4-byte magic** (`b"RBUS"`) + `u16 version` ahead of the length prefix to hard-fail mismatches early.

5. **Zero-copy & payload ergonomics:**

   * Switch `Envelope.payload` to `bytes::Bytes` and support **borrowed decode** via `rmp-serde` where possible.

6. **Security posture:**

   * Provide optional **cap verification helpers** (ed25519 verify, `exp`/`nonce` checks) in a *separate* crate (e.g., `ron-cap`) so services can share hardened logic without bloating `ron-ipc`.

### Overlap & redundancy signals

* Overlap with **`ron-kernel::Bus`** in name only: different layers but **same term**, which leads to architectural confusion and doc drift. Fix via rename.
* Overlap with future **`ron-proto`** plan in blueprints: this crate shouldn’t be the “god crate” for all service protocols.

## 14) Change Log (recent)

* 2025-09-14 — Initial review; confirmed blocking UDS helpers (`listen/recv/send`), Envelope shape, and `RON_BUS_PROTO_VERSION = 1`; identified service-enum colocation and naming drift risk.
* 2025-09-05 … (insert if we land the split/rename; otherwise keep this slot empty for next pass)

## 15) Readiness Score (0–5 each)

* **API clarity:** 3 — Envelope and UDS helpers are clear; service RPCs living here muddy boundaries.
* **Test coverage:** 1 — No tests in-crate; relies on downstream integration.
* **Observability:** 1 — No metrics/tracing; opaque errors.
* **Config hygiene:** 2 — Simple, but lacks size/time guards; callers must roll their own.
* **Security posture:** 2 — UDS only; capability bytes carried but not verified; acceptable for localhost IPC, not for network boundaries.
* **Performance confidence:** 3 — Blocking UDS is fast; no guards/benchmarks; potential runtime stalls in async services if misused.
* **Coupling (lower is better):** 2 — Today it’s a coupling hub because it hosts service RPC enums; after `ron-proto` split it would improve to 4–5.

---

### Appendix — Quick API sketch (from the crate)

* **Envelope**

  ```rust
  #[derive(Serialize, Deserialize, Debug, Clone)]
  pub struct Envelope {
      pub service: String,      // e.g., "svc.index"
      pub method: String,       // e.g., "v1.resolve"
      pub corr_id: u64,         // correlation id for RPC
      pub token: Vec<u8>,       // MsgPack<CapClaims> or empty
      pub payload: Vec<u8>,     // MsgPack-encoded method payload
  }
  ```
* **UDS (blocking)**

  ```rust
  pub fn listen(sock_path: &str) -> io::Result<UnixListener>;
  pub fn recv(stream: &mut UnixStream) -> io::Result<Envelope>;
  pub fn send(stream: &mut UnixStream, env: &Envelope) -> io::Result<()>;
  ```
* **CapClaims container** (carried in `token`, verification out of scope here)

  ```rust
  pub struct CapClaims { pub sub: String, pub ops: Vec<String>, pub exp: u64, pub nonce: u64, pub sig: Vec<u8> }
  ```
