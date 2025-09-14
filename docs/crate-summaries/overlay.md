---

crate: overlay
path: crates/overlay
role: lib
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
-------------------------

## 1) One-liner

A legacy library that bundles a simple TCP overlay protocol (PUT/GET), a sled-backed blob store, and convenience client/server helpers into one crate.

## 2) Primary Responsibilities

* Implement a minimal overlay wire protocol (PUT → hash; GET ← blob) over TCP.
* Persist and retrieve binary blobs in a local sled database keyed by BLAKE3 hex.
* Provide helpers to run a listener and to act as a simple client (PUT/GET).

## 3) Non-Goals

* No HTTP surface, no OAP/ALPN/TLS, and no capability/tenant enforcement.
* Not a scalable content distribution service (no chunking, no replication).
* Not a metrics/quotas layer; only basic tracing logs.

## 4) Public API Surface

* Re-exports: `pub use store::Store;`
* Key types / functions / traits:

  * `Store` (sled wrapper): `Store::open<P: AsRef<Path>>(path) -> Result<Self>`, `put(&self, key: &[u8], val: Vec<u8>)`, `get(&self, key: &[u8]) -> Result<Option<Vec<u8>>>`
  * Protocol (TCP helpers):

    * `run_overlay_listener(bind: SocketAddr, store_db: impl AsRef<Path>) -> anyhow::Result<()>` — spawns a background TCP server task.
    * `client_put(addr: &str, path: &Path) -> anyhow::Result<String>` — PUT file, returns BLAKE3 hex.
    * `client_get(addr: &str, hash_hex: &str, out: &Path) -> anyhow::Result<()>` — GET by hash to file.
  * Internal error enum exists (`OverlayError`) but the crate’s public functions mostly return `anyhow::Result`, so the typed error is not propagated publicly.
* Events / HTTP / CLI: none (pure library; logging via `tracing`).

## 5) Dependencies & Coupling

* Internal crates → why, stability, replaceable?

  * (None) — pure library; consumers (e.g., gateway or svc-\* crates) call into this.
  * **Replaceable:** yes; the whole crate is a thin layer that could be superseded by `svc-overlay` (network) + `svc-storage` (DB).
* External crates (top 5; pins/features) → why, risk

  * `tokio` (full) — async TCP I/O & tasks; mature/low risk.
  * `sled` — embedded KV store; stable but maintenance is conservative; watch for long-standing issues (crash safety, compaction).
  * `blake3` — fast hashing for content addressing; low risk.
  * `anyhow` / `thiserror` — error ergonomics; low risk.
  * `tracing` — logging; low risk.
  * (Present but **unused** in code): `lru`, `transport`. These indicate drift/tech debt.
* Runtime services: file-system DB via sled; plain TCP. No TLS/crypto beyond hashing.

## 6) Config & Feature Flags

* Env vars: none.
* Config structs: none; the caller supplies `SocketAddr` and DB path.
* Cargo features: none (no conditional compilation for TLS/OAP/chunking).

## 7) Observability

* Logs: `tracing::info!` on bind and `tracing::warn!` when listener task exits with error.
* No metrics, health/readiness, or structured error counters.
* No request IDs/spans; correlation relies on peer address and op type.

## 8) Concurrency Model

* Server: `run_overlay_listener` spawns a background task; inside it, a `TcpListener` accepts connections; each connection is handled on its own `tokio::spawn`.
* Backpressure: none at protocol level; server reads entire request payload into memory based on a declared 8-byte length prefix.
* Locks/timeouts/retries: none explicit; sled provides internal concurrency; network ops rely on default socket timeouts (i.e., effectively unbounded).
* Failure model: background task’s `JoinHandle` is dropped; errors are only logged (no restart/supervision here).

## 9) Persistence & Data Model

* DB: sled (single tree, default namespace).
* Keys: `hash_hex` (BLAKE3, lowercase hex).
* Values: raw blob bytes.
* Durability: `Store::put` calls `db.flush()` after insert (safer for tests/scripts, slower in bulk).
* TTL/GC/retention: none; unlimited growth until external pruning.

## 10) Errors & Security

* Error taxonomy:

  * I/O, sled errors, UTF-8, early EOF, unknown opcode, and a placeholder `InvalidChunkSize` (unused by current protocol).
  * Public API returns `anyhow::Result`, so callers cannot match on the typed `OverlayError`.
* Security posture:

  * Plain TCP with no TLS, ALPN, or authn/z.
  * No rate limiting or quota.
  * No validation of caller identity; unauthenticated writes/reads possible if exposed.
  * PQ-readiness N/A (no TLS/keys).
* Input bounds:

  * PUT: trusts an 8-byte big-endian length prefix (no max bound enforced) → potential memory blow or DoS if exposed.

## 11) Performance Notes

* Hot paths: PUT hashing (BLAKE3 over full payload) and sled `insert + flush`; GET reads full value and writes it out.
* Current behavior is fully in-memory per request: allocates a `Vec<u8>` of exact length; no streaming/chunking.
* SLOs: none defined in code; practical latency will be dominated by FS and blob size; with `flush()` on every PUT, throughput will be poor under sustained load.

## 12) Tests

* No unit or integration tests included in the crate snapshot.
* No fuzz/property tests, no loom concurrency checks, no E2E harness.
* Manual testing implied via client helpers (`client_put`/`client_get`).

## 13) Improvement Opportunities

**Known gaps / tech debt**

* Remove **unused deps** (`lru`, `transport`) or wire them up (e.g., LRU read-cache).
* The `OverlayError` type is not exposed/used by the public API; either propagate it or delete it to avoid confusion.
* No bounds on PUT size; add a configurable max (e.g., 1 MiB like OAP defaults) to prevent DoS.
* `run_overlay_listener` drops its `JoinHandle`; provide a handle or supervision hook for clean shutdown and error handling.

**Overlap & redundancy signals**

* This crate duplicates responsibilities now planned for `svc-overlay` (network protocol) and `svc-storage` (DB). Keeping all three risks drift.
* It also diverges from our OAP+TLS direction: raw TCP here vs. axum/OAP in newer services.

**Streamlining (merge/extract/replace/simplify)**

* **Option A (recommended):** Freeze this crate as **legacy** (tests + docs only), and route all new code to:

  * `svc-overlay` for network protocol (ideally OAP over TLS/ALPN).
  * `svc-storage` for persistence, with sled/SQLite abstraction and quotas.
* **Option B:** If keeping:

  * Add `max_put_bytes` bound; stream PUT (hash while writing) and avoid full-buffer allocs (`tokio::io::copy` + rolling BLAKE3).
  * Add `GET` streaming (length prefix + framed chunks).
  * Introduce metrics: `requests_total{op}`, `bytes_in/out_total`, `errors_total{reason}`, `latency_seconds{op}`, `inflight`.
  * Add feature flags: `tcp` (current), `oap` (reframe), `tls` (via `tokio-rustls`), `cache` (enable LRU), `metrics`.
  * Expose structured errors (`OverlayError`) in public API.
  * Replace per-PUT `flush()` with batched/periodic fsyncs or make it configurable.

## 14) Change Log (recent)

* 2025-09-14 — Deep audit; documented legacy status, identified unused deps and security/perf gaps; proposed retirement in favor of `svc-overlay` + `svc-storage`.

## 15) Readiness Score (0–5 each)

* API clarity: **3** (surface is tiny and easy to call, but error typing is inconsistent and protocol undocumented externally).
* Test coverage: **1** (no tests observed).
* Observability: **1** (logs only; no metrics/health).
* Config hygiene: **2** (caller-provided bind/path is clean; no bounds/features/env).
* Security posture: **1** (plain TCP; unauthenticated; no quotas/bounds).
* Performance confidence: **2** (works for small blobs; `flush()` per PUT hurts throughput; no streaming).
* Coupling (lower is better): **3** (self-contained, but overlaps with the newer service split; two unused deps signal drift).

