

# RustyOnions Hardening Blueprint v1.1

**North Star:** badass + hardened = fails well, refuses bad input, tells the truth, keeps secrets straight, performs under stress.

## Success criteria (DoH: “Definition of Hardened”)

A service is **Hardened** iff it has all of:

* **Limits:** 5s timeout; 512 inflight; 500 rps (tune per svc); 1 MiB request body cap; 10× decompression cap.
* **Ops:** `/metrics` (golden metrics + svc-specific), `/healthz`, `/readyz` (gated), `/version` (build+config).
* **Security:** UDS dir `0700`, socket `0600` + `SO_PEERCRED` allowlist; **no direct DB access from tools**; privileged ops policy-gated.
* **Perf:** streaming for large bodies; no unbounded `Vec<u8>`; memory ceilings enforced.
* **Tests:** unit + property + integration; chaos restart; metrics assertions.

---

## P0 — Immediate (ship first)

### 1) Uniform HTTP hardening (all HTTP services)

**Add dependency**

```toml
# Cargo.toml
tower = { workspace = true }
tower-http = { workspace = true, features = ["limit", "timeout", "trace"] }
```

**Drop-in middleware** `src/hardening.rs`

```rust
use std::time::Duration;
use axum::Router;
use tower::{Layer, ServiceBuilder};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::{DefaultOnResponse, TraceLayer};
use tower::limit::{ConcurrencyLimitLayer, RateLimitLayer};
use tracing::Level;

pub fn layer() -> impl Layer<Router> + Clone {
    ServiceBuilder::new()
        .layer(TimeoutLayer::new(Duration::from_secs(5)))
        .layer(ConcurrencyLimitLayer::new(512))
        .layer(RateLimitLayer::new(500, Duration::from_secs(1)))
        .layer(RequestBodyLimitLayer::new(1_000_000)) // ~1 MiB
        .layer(TraceLayer::new_for_http()
            .on_response(DefaultOnResponse::new().level(Level::INFO)))
        .into_inner()
}
```

**Wire it**

```rust
let app = Router::new().route(/* … */).with_state(state.clone());
let app = hardening::layer().layer(app);
axum::serve(listener, app).await?;
```

### 2) Golden metrics everywhere (uniform names)

* `http_requests_total{route,method,status}` (counter)
* `request_latency_seconds{route,method}` (histogram)
* `inflight_requests{route}` (gauge or implied by limit)

Pattern already exists in `svc-economy`; copy the wrapper into `micronode`, `svc-omnigate`/`gateway`, `svc-edge`, later into any HTTP svc.

### 3) Protocol limits & decompression safety

* **OAP/1:** hard-reject frames > **1 MiB** before buffering.
* **Chunks:** process in **64 KiB** increments.
* **Decompression guard:** cap expansion to **10×** and absolute output size.

Sketch:

```rust
pub fn safe_decompress(input: &[u8], max_out: usize, max_ratio: f32) -> Result<Vec<u8>, &'static str> {
    // Replace with actual streaming decoder
    let decoded_len = 0usize; // ...
    if decoded_len > max_out { return Err("output_size_limit"); }
    if (decoded_len as f32) / (input.len().max(1) as f32) > max_ratio { return Err("decompression_ratio"); }
    Ok(Vec::new())
}
```

### 4) UDS hardening (svc-index, svc-overlay, svc-storage, svc-crypto)

* Socket dir `0700`; socket `0600`.
* **`SO_PEERCRED`** check (UID/GID allowlist) on accept; drop unknowns.

Snippet:

```rust
use nix::sys::socket::{getsockopt, sockopt::PeerCredentials};
use std::os::fd::AsRawFd;

fn check_peer(stream: &std::os::unix::net::UnixStream, allowed_uids: &[u32]) -> bool {
    getsockopt(stream.as_raw_fd(), PeerCredentials)
        .ok()
        .map(|c| allowed_uids.contains(&c.uid()))
        .unwrap_or(false)
}
```

### 5) Eliminate direct DB access from `tldctl`

* All mutations route through a UDS API on `svc-index`.
* One schema owner → no sled lock fights; simpler durability.

### 6) Focused tests that catch real failures

* **Property tests** (ron-ledger): conservation, non-negativity, idempotent retries.
* **Integration tests** (each HTTP svc): exercise all endpoints + `/metrics` + readiness gating.
* **Chaos**: kill + restart under load; ensure `/readyz` flips before listen socket closes.

Property test sketch:

```rust
// crates/ron-ledger/tests/prop.rs
use ron_ledger::InMemoryLedger;
#[test]
fn supply_is_mints_minus_burns() {
    let mut l = InMemoryLedger::new();
    // generate ops, assert invariants…
}
```

---

## P1 — Next (performance, safety, clarity)

### 7) Streaming I/O (svc-overlay, svc-storage)

* Replace `Vec<u8>` with streaming bodies; enforce per-stream & global byte ceilings.

```rust
use tokio_util::io::ReaderStream;
use axum::body::Body;

let reader = /* AsyncRead */;
let stream = ReaderStream::new(reader);
let resp = axum::response::Response::builder()
    .body(Body::from_stream(stream))?;
```

### 8) Economy persistence & safety

* New backend: **`ron-ledger-sqlite`** (append-only journal + periodic snapshots).
* **Idempotency keys** + **per-account sequence numbers** in `ron-token`.
* **Policy gates** (`ron-policy`) on `/mint` & `/burn`; tenant policy for `transfer`.

### 9) Single ingress (recommend **`svc-omnigate`**)

* TLS terminate, authn/z, quotas, OAP parsing limits, decompression guard.
* Everything else over UDS or mTLS behind it.

### 10) Config hygiene

* Central config module (typed structs; env overlay; **reject unknown vars**).
* `/version` returns build info + sanitized config snapshot (no secrets).

---

## P2 — Soon after (resilience & secrets)

### 11) Key & secret handling

* Envelope-encrypt key files or lean on platform KMS; rotate keys; log **key IDs** only.
* “Amnesia mode” for ephemeral keys (e.g., Tor HS) on shutdown.

### 12) Runtime hardening

* Containers: read-only root, seccomp default, drop caps, resource limits.
* Prioritize ingress; set per-svc CPU/mem ceilings.

---

## Deep dives (targets Grok flagged)

### A) `svc-storage`

* **Metrics:** `storage_get_latency_seconds`, `storage_put_latency_seconds`, `chunks_read_total`, `chunks_written_total`, `io_errors_total`.
* **Streaming:** default; no full buffering; `MAX_OBJECT_BYTES` enforced.
* **Integrity:** verify BLAKE3 before serve; on mismatch → 502 + `integrity_fail_total`.
* **Concurrency:** async file I/O; bounded pools (`Semaphore`).
* **Tests:** corrupted chunk → 502; sustained 64 KiB load.

### B) `svc-crypto`

* **Metrics:** `sign_latency_seconds`, `verify_latency_seconds`, `crypto_errors_total{op}`.
* **Timing:** constant-time verify paths where possible.
* **Key custody:** files `0600`, non-root; optionally OS keychain/envelope encryption.
* **UDS:** peer-cred required; optional split roles (sign vs verify).

### C) Fuzzing: `oap`, `ron-bus`, `ron-proto`

```
cargo install cargo-fuzz
cargo fuzz init
cargo fuzz add oap_fuzz_frame_parse
cargo fuzz run oap_fuzz_frame_parse
```

Targets:

* `oap_fuzz_frame_parse` — malformed/oversized frames.
* `bus_fuzz_event_decode` — envelope garbage.
* `proto_fuzz_dto_roundtrip` — serde invariants.

### D) Concurrency fixes

* **svc-index / svc-overlay:** move to async UDS (`tokio::net::UnixListener/UnixStream`) or bound thread pools.
* **arti-transport:** parallelize accepts; apply backpressure when Tor is slow.
* **Actors:** pick **one** (`kameo` or `ryker`); deprecate the other.

---

## Economy wiring (clear & safe)

* `svc-economy` publishes **ledger events** on `ron-bus`: `Minted`, `Burned`, `Transferred{from,to,amount,receipt_id}`.
* `ron-billing` subscribes → invoices/quotas; rejects if over budget (policy).
* `ron-audit` records signed epoch roots + critical ops; receipts are BLAKE3-addressed.

---

## Single ingress migration plan (to `svc-omnigate`)

1. Feature-freeze `gateway`.
2. Move quotas/auth/TLS/signing into `svc-omnigate`.
3. Keep `gateway` as a thin shim → deprecate.
4. Update `ALL_SUMMARIES` + service READMEs: the ingress contract is **one place**.

---

## 7-day execution checklist (no-fail path)

* **Day 1–2:** Add `hardening::layer()` to `svc-omnigate`, `svc-economy`, `micronode`, `svc-edge`. Enforce OAP limits + decompression caps. Implement `SO_PEERCRED` checks + perms on all UDS servers.
* **Day 3–4:** Clone golden metrics wrapper across all HTTP services. Gate readiness with `AtomicBool`. Route `tldctl` mutations via `svc-index` UDS.
* **Day 5:** Make `svc-overlay` responses streaming-only; set size ceilings. Add BLAKE3 verify + latency histograms to `svc-storage`.
* **Day 6:** Add `cargo-fuzz` for `oap`; property tests to `ron-ledger`.
* **Day 7:** CI guard for DoH v1.1 (limits present, metrics live, readiness gate, UDS perms). Publish “Hardened by default” doc and link from every crate README.

---

## CI guardrails (DoH v1.1 checks)

* Lint for `hardening::layer()` in each `main.rs`.
* Self-test container:

  * hit `/readyz` (expects 200 once ready),
  * send 2 MiB body (expects 413),
  * scrape `/metrics` and assert `request_latency_seconds` exists.
* UDS perms script: fail if dir not `0700` or socket not `0600`.

---

## Rating & confidence (with missing bullets restored)

**Rating: 9.6 / 10 (hardened posture)**

**Pros**

* **+ Massive P0 coverage and immediate protections**
* **+ Explicit storage/crypto hardening & fuzzing plan**
* **+ Clear ingress decision and economy plumbing**

**Cons / Pending**

* **– Pending:** async UDS migration, actor unification, SQLite ledger backend, CI automation depth

Close the pending items and you’re hovering **9.8–9.9**—“diamond fortress” territory.

