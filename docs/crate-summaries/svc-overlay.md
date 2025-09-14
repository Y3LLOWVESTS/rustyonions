---

crate: svc-overlay
path: crates/svc-overlay
role: service
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
-------------------------

## 1) One-liner

Thin overlay service that answers RPC `Health` and `Get{addr,rel}` over a local UDS “bus,” resolving content addresses via `svc-index` and returning file bytes from `svc-storage`.

## 2) Primary Responsibilities

* Accept `OverlayReq::Get` and return `OverlayResp::{Bytes,NotFound,Err}`.
* Resolve `addr` → directory via `svc-index`, then read a file (default `payload.bin` if `rel==""`) from `svc-storage`.
* Provide a lightweight health probe (`OverlayReq::Health → OverlayResp::HealthOk`).

## 3) Non-Goals

* No public HTTP surface, quotas, or request shaping (that’s `svc-omnigate`/gateway).
* No durable storage, manifests, DHT routing, or chunk management (owned by `svc-storage` / future DHT).
* No application-level crypto semantics (APP E2E stays opaque above the service plane).

## 4) Public API Surface

* **Wire (bus/UDS, MessagePack via `rmp-serde`):**

  * **Request enums (from `ron_bus::api`):** `OverlayReq::{Health, Get{addr:String, rel:String}}`.
  * **Response enums:** `OverlayResp::{HealthOk, Bytes{data:Vec<u8>}, NotFound, Err{err:String}}`.
  * **Envelope fields set by this service:** `service="svc.overlay"`, `method ∈ {"v1.ok","v1.not_found","v1.err"}`, `corr_id` echoed, `token=[]`.
* **No Rust re-exports intended for external consumers (service binary).**

## 5) Dependencies & Coupling

* **Internal crates (via API types, not direct linking):**

  * `ron-bus` — Envelope + UDS helpers (`listen/recv/send`) and shared request/response enums. *Loose; replaceable=yes.*
  * Communicates with **`svc-index`** (`IndexReq::Resolve → IndexResp::{Resolved,NotFound,Err}`) and **`svc-storage`** (`StorageReq::Read{dir,rel} → StorageResp::{File,NotFound,Err}`) over UDS. *Loose; replaceable=yes.*
* **External crates (workspace-pinned):** `anyhow`, `serde`, `rmp-serde`, `tracing`, `tracing-subscriber`.
* **Runtime services:** Unix domain sockets only (POSIX); no network listeners; local filesystem reads happen in `svc-storage`, not here. No TLS at this hop.

## 6) Config & Feature Flags

* **Env vars:**

  * `RON_OVERLAY_SOCK` (default `/tmp/ron/svc-overlay.sock`) — where this service listens.
  * `RON_INDEX_SOCK` (default `/tmp/ron/svc-index.sock`) — where to reach `svc-index`.
  * `RON_STORAGE_SOCK` (default `/tmp/ron/svc-storage.sock`) — where to reach `svc-storage`.
* **Cargo features:** none declared.
* **Filesystem:** creates parent directory for its UDS path if missing (macOS tmpdirs friendliness).

## 7) Observability

* **Logs:** structured `tracing` at `info`/`error` with high-signal records (`%addr`, `%rel`, bytes length, and error contexts).
* **Health:** RPC health handler (`OverlayReq::Health`). No HTTP `/healthz` and no Prometheus metrics in this crate (yet).
* **Correlation:** echoes `corr_id` from inbound Envelope.

## 8) Concurrency Model

* **Server pattern:** blocking UDS listener; **per-connection thread** (`std::thread::spawn`) that handles exactly one request/response pair (receive → match → reply).
* **Backpressure:** OS accept queue + thread scheduling; no explicit inflight limits besides OS resources.
* **Timeouts/retries:** none at this layer; callers (gateway/SDK) should apply deadlines and retries.
* **Sync I/O:** uses `UnixStream` (no Tokio); simple, portable across POSIX, but each connection ties up a thread.

## 9) Persistence & Data Model

* **Stateless.** Holds no durable state; passes through `addr`/`rel` and returns raw bytes.
* **Default file name:** if `rel==""`, reads `payload.bin` within the directory resolved for `addr`.

## 10) Errors & Security

* **Error taxonomy (current):** maps failures to `OverlayResp::Err{err}`; “not found” is explicit (`NotFound`). No structured retry hints (e.g., `Retry-After`) yet.
* **Security:** local UDS hop, no auth; `Envelope.token` not used. Capability tokens/macaroons (if any) would be enforced at the gateway/omnigate layer.
* **Integrity:** does not perform digest verification; relies on upstream index/storage contracts. (Blueprint calls for BLAKE3 addressing/verification—**not yet implemented here**.)
* **Platform:** `std::os::unix` dependency means no Windows build without a compatibility layer.

## 11) Performance Notes

* **Hot path:** `Get → (index resolve) → (storage read) → respond Bytes`.
* **Current behavior:** reads entire file into memory (`Vec<u8>`) before replying—good for small assets, risky for large payloads (latency and memory spikes).
* **Targets (pragmatic for local UDS):** p50 single-file GET under a few ms; introduce chunked/streamed reads to preserve p95 under load.

## 12) Tests

* **In-crate:** none.
* **System/E2E:** validated indirectly when the gateway smoke test exercises overlay via index/storage.
* **Recommended next:** unit tests for request decoding/response mapping; integration tests against stubbed index/storage; golden tests for edge cases (`rel==""`, long `rel`, not-found, storage/index error). Add property tests for “echo `corr_id`” and “never panic on decode/encode.”

## 13) Improvement Opportunities

* **Streaming:** return framed/chunked responses instead of a single `Vec<u8>` to avoid memory bloat and enable large objects.
* **Time bounds:** add read/connect timeouts to index/storage calls, and per-request deadlines.
* **Metrics:** add Prometheus counters/histograms (requests, bytes, latency, errors) to align with observability standards elsewhere.
* **Integrity:** optionally verify BLAKE3 of returned bytes (configurable), or at least assert size/hint from index when available.
* **Concurrency:** move to Tokio (or a bounded thread-pool) for fewer threads under load; bound inflight work and introduce backpressure signals to callers.
* **Security hooks:** plumb `Envelope.token` through and perform capability checks once repo-wide policy is ready.
* **Windows portability:** abstract UDS behind a transport trait to unlock Windows named pipes.

## 14) Change Log (recent)

* **2025-09-14** — Draft UDS service with `Health` + `Get` end-to-end to index/storage; logging hardened; no metrics yet.

## 15) Readiness Score (0–5 each)

* **API clarity:** 2.5 — Minimal, clear request/response; lacks streaming & structured error envelope.
* **Test coverage:** 1.0 — No unit/integration tests in-crate.
* **Observability:** 2.0 — Good logs; no metrics/readiness endpoints.
* **Config hygiene:** 2.5 — Socket paths are env-configurable; sensible defaults; no feature flags.
* **Security posture:** 2.0 — Local UDS only; no auth; integrity verification deferred.
* **Performance confidence:** 2.5 — Fine for small files; needs streaming/backpressure before scale.
* **Coupling (lower is better):** 3.0 — Loose, via bus enums and UDS; clean boundaries with index/storage.

