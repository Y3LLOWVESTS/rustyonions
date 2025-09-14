---

crate: svc-index
path: crates/svc-index
role: service
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
-------------------------

## 1) One-liner

Local Unix-socket index service that maps RustyOnions content addresses to bundle directories and answers resolve/put requests for peers.

## 2) Primary Responsibilities

* Maintain the **address → directory** mapping (open the index DB; get/set entries).
* Serve **IPC RPCs** over UDS (MsgPack) for `Resolve`, `PutAddress`, and `Health`.
* Emit concise **structured logs** (JSON via `tracing-subscriber`) and stay simple/robust under kernel supervision.

## 3) Non-Goals

* No HTTP; no direct file I/O of bundle contents (that’s `gateway` / `svc-storage`).
* No overlay/network lookups, caching, or fetch from remote peers.
* No authorization policy or capability enforcement beyond local UDS trust.

## 4) Public API Surface

This is a **service**; its “API” is the on-wire RPC (via `ron-bus`).

* **Re-exports:** none.
* **Key RPC DTOs (from `ron_bus::api`):**

  * `IndexReq::{ Health, Resolve { addr }, PutAddress { addr, dir } }`
  * `IndexResp::{ HealthOk, Resolved { dir }, NotFound, PutOk, Err { err } }`
* **Service wiring (in `src/main.rs`):**

  * `listen(sock_path) -> UnixListener` (blocking), accept loop, **thread-per-connection**.
  * `recv(&mut UnixStream) -> Envelope` → decode `IndexReq` via `rmp_serde`.
  * Handle request via `index::Index` methods:

    * `get_bundle_dir(&Address) -> Result<Option<PathBuf>>`
    * `put_address(&Address, PathBuf) -> Result<()>`
  * Respond with `Envelope { service: "svc.index", method: "v1.ok", corr_id, payload=MsgPack(IndexResp), token=[] }`.
* **Utilities:** `to_vec_or_log<T: Serialize>(..) -> Vec<u8>` (MsgPack encode; log on error).
* **Events / HTTP / CLI:** none (health is an RPC variant, not HTTP).

## 5) Dependencies & Coupling

* **Internal crates**

  * `index` (tight): provides the actual DB engine/operations for the mapping. Replaceable: **yes** (e.g., swap sled/other store behind the `index` crate).
  * `naming` (tight): `Address` parsing/normalization (prevents format drift). Replaceable: **no** in practice (all address semantics live there).
  * `ron-bus` (tight): IPC envelopes + UDS helpers. Replaceable: **yes** if/when we split to `ron-proto` + `ron-ipc` (recommended).
* **External crates (top)**

  * `serde`, `rmp-serde`: MsgPack encode/decode (stable, low risk).
  * `tracing`, `tracing-subscriber` (json, env-filter): structured logs (low risk).
  * `regex`: present in Cargo; not used in this file—likely used in `index` crate or is a leftover (watch for dead dep).
* **Runtime services:** OS UDS and filesystem (index DB path).

## 6) Config & Feature Flags

* **Env vars**

  * `RON_INDEX_SOCK` (default **`/tmp/ron/svc-index.sock`**): UDS path to bind.
  * `RON_INDEX_DB` (default **`.data/index`**): DB path opened by `index::Index::open`.
* **Cargo features:** none declared here.
* **Notes:** Defaults are sensible, but the **DB path must match other services** (e.g., gateway, overlay, storage) or you’ll see spurious `NotFound` on resolve.

## 7) Observability

* **Logs:** JSON logs via `tracing-subscriber` + `EnvFilter` (e.g., `info!` on start, resolve/put; `error!` on decode/DB errors).
* **Metrics:** none.
* **Health/Readiness:** RPC `Health → HealthOk`. No Prometheus endpoints.

## 8) Concurrency Model

* **Blocking UDS** listener; **one OS thread per client** (`std::thread::spawn` in accept loop).
* **Backpressure:** OS socket buffers only. No in-process queues, limits, or rate control.
* **Timeouts/Retries:** none at service layer. A stuck client could hold a thread.
* **Shared state:** `Arc<index::Index>`; internal locking is managed by the `index` crate.

## 9) Persistence & Data Model

* **Store:** handled by `index::Index` (likely a sled-backed KV at `${RON_INDEX_DB}`).
* **Key/Value shape (inferred):** `Address` → `PathBuf` (bundle directory).
* **Retention:** stable mapping; no TTL/GC logic in the service.

## 10) Errors & Security

* **Error taxonomy (on RPC):**

  * Decode/IO errors → log and drop client (no response) or respond with `IndexResp::Err`.
  * Missing mapping → `IndexResp::NotFound`.
  * Put failures → `IndexResp::Err { err }`.
* **Security model:** local **UDS only**, no authn/z; relies on file permissions and process boundaries.
* **Hardening gaps:** no peer-credential checks (`SO_PEERCRED`/`LOCAL_PEERCRED`), no allowlist of caller UIDs/GIDs, no max-frame guard (DoS risk).
* **TLS/PQ:** N/A (local IPC).

## 11) Performance Notes

* Hot path is tiny: decode → `index::Index` get/put → encode.
* **Targets (suggested):**

  * p95 `Resolve` (hit) ≤ 200 µs; `NotFound` ≤ 150 µs (DB miss).
  * p95 `PutAddress` ≤ 300 µs.
* Watch for **thread explosion** under many idle clients; consider a small thread-pool or async UDS in future.

## 12) Tests

* **Present (in this crate):** none visible.
* **Recommended:**

  * **Integration (UDS)**: spawn service on a temp socket + temp DB; exercise `Health/Resolve/PutAddress` happy/err paths.
  * **DB path sanity:** set `RON_INDEX_DB` to a temp dir and verify cross-service compatibility (same path → resolves succeed).
  * **Fuzz/property:** random bytes into `recv`/MsgPack decode should never panic or OOM; malformed Address strings must be rejected.
  * **Concurrency:** parallel `PutAddress` and `Resolve` workloads to smoke-test locking in `index::Index`.

## 13) Improvement Opportunities

### Known gaps / tech debt

* **No access control**: any local process can talk to the socket. Add **peer-cred allowlist** (UID/GID) and strict socket perms.
* **No resource guards**: add **max frame size** and **per-request deadline** (e.g., 2s) to avoid stuck handlers.
* **Metrics absent**: add counters (`index_requests_total{op,ok}`), histograms (`index_latency_seconds{op}`), and byte totals.
* **Name/API ownership**: RPC enums live in `ron-bus::api`; this couples service evolution to that crate.

### Overlap & redundancy signals

* **Address normalization** correctly lives in `naming::Address`—keep it there to prevent drift across services.
* Mapping ownership is **centralized** here; remove any ad-hoc maps from other crates (e.g., gateway) to avoid duplication.

### Streamlining (merge/extract/replace/simplify)

1. **Split protocol from transport**: move `IndexReq/Resp` to a new `ron-proto::index` and keep `ron-bus` transport-only.
2. **Guardrails**: add `MAX_REQ_BYTES` env (default 1 MiB) and per-request timeout.
3. **Authn**: optional shared-secret HMAC on envelopes or peer-cred allowlist.
4. **Observability**: feature-gated Prometheus metrics; keep default lean.
5. **Async option**: optional `tokio` UDS backend to avoid thread-per-client under load.

## 14) Change Log (recent)

* 2025-09-14 — Reviewed: blocking UDS accept loop; JSON logs; RPCs `Health/Resolve/PutAddress`; envs `RON_INDEX_SOCK`, `RON_INDEX_DB`; identified guardrails & auth gaps.

## 15) Readiness Score (0–5 each)

* **API clarity:** 4 — RPC surface is small and obvious.
* **Test coverage:** 2 — Needs integration and negative tests.
* **Observability:** 2 — Good JSON logs; no metrics/health endpoints.
* **Config hygiene:** 3 — Sensible defaults; document DB/socket interplay clearly.
* **Security posture:** 2 — Local-only but open; add peer-cred checks and socket perms guidance.
* **Performance confidence:** 4 — Minimal hot path; thread model is fine at current scale.
* **Coupling (lower is better):** 3 — Tied to `index`, `naming`, and `ron-bus::api`; splitting protocol would improve.

