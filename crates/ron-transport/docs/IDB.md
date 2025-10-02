

---

````markdown
---
title: ron-transport — Invariant-Driven Blueprint (IDB)
version: 0.1.2
status: draft
last-updated: 2025-10-01
audience: contributors, ops, auditors
crate: ron-transport
pillar: 10 — Overlay, Transport & Discovery
owners: [Stevan White]
---

# 🪓 Invariant-Driven Blueprinting (IDB) — `ron-transport`

`ron-transport` is the **unified transport library** for RustyOnions. It abstracts TCP/TLS and (optionally) Tor via **Arti** behind cargo features. The old `svc-arti-transport` was **merged into this crate**; Arti is now a **feature** (`arti`). The DHT lives in `svc-dht` and overlay sessions/gossip in `svc-overlay`; **no DHT/overlay logic here**.  
`ron-transport` must honor OAP/1 bounds as **I/O limits** but does not parse OAP frames.

---

## 1. Invariants (MUST)

- **[I-1] Boundary & canon.** `ron-transport` is a **library** exposing dialers/listeners and typed connections. It must not implement overlay sessions, gossip, or DHT. It exists as one crate in **Pillar 10** of the 33-crate canon.
- **[I-2] Feature-gated backends.** Provide `tcp` and `tls` by default; **Arti/Tor lives under `--features arti`**. The legacy `svc-arti-transport` must not reappear.
- **[I-3] TLS type discipline.** TLS configs use **`tokio_rustls::rustls::{ClientConfig, ServerConfig}`** only (so `TlsAcceptor::from` works). No direct `rustls::*` types in public API.
- **[I-4] Single writer per connection.** Each socket has exactly **one writer task**; reader/writer halves split cleanly.
- **[I-5] No locks across `.await`.** Never hold a lock across async suspension in transport paths.
- **[I-6] Timeouts & ceilings mandatory.** Every connection enforces **read/write/idle timeouts** and **per-peer/global caps**; quotas are checked **before work**.
- **[I-7] Owned bytes on hot path.** Use `bytes::Bytes/BytesMut` for I/O payloads; no borrowed slices escape.
- **[I-8] OAP/1 limits as I/O caps.** Clamp reads to **≤ 1 MiB** per frame, stream in ~**64 KiB** chunks. No OAP parsing here.
- **[I-8b] Decompression guard.** Apply a **compression inflation guard ≤ 8×**; additionally enforce the **absolute 1 MiB frame cap**.  
  *Rationale:* 8× (stricter than the global 10× default) reduces per-connection amplification risk and memory spikes on framed links.
- **[I-9] Observability first-class.** Expose metrics:  
  `transport_dials_total{backend,result}` • `transport_accepts_total{backend}` • `handshake_latency_seconds{backend}` • `bytes_in_total/bytes_out_total{backend}` • `conn_inflight{backend}` • `rejected_total{reason}`
- **[I-10] PQ-ready, crypto-neutral.** Never hard-code non-PQ algorithms; accept PQ/hybrid toggles from config when the stack supports them. Tor circuits remain classical unless Arti exposes PQ circuits in future.
- **[I-11] Amnesia mode honored.** With `amnesia=ON`, no persistent artifacts (including Arti caches) and zeroized ephemeral state.
- **[I-12] Rate limits enforced.** Both per-peer and global **connection + I/O rate limits** (token-bucket style) must exist and be observable.
- **[I-13] Error taxonomy stable.** All errors come from a closed, `#[non_exhaustive]` enum with machine-parsable `kind()`.
- **[I-14] Readiness states explicit.** Crate exposes tri-state: `Ready`, `Degraded(reason)`, `NotReady(reason)`. Arti bootstrap → `Degraded("arti_bootstrap")`.
- **[I-15] Metrics buckets fixed.** Canonical buckets:  
  - `handshake_latency_seconds`: `[0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1, 2]`  
  - `io_chunk_bytes`: powers-of-two up to **1 MiB** (e.g., 1 KiB … 1 MiB)  
  - (Optional) `throughput_bytes_per_sec`: `[1k, 2k, 4k, 8k, 16k, 32k, 64k, 128k, 256k, 512k, 1m]`
- **[I-16] Six Concerns compliance.** This crate maps to **SEC, RES, PERF** and must satisfy CI gates.
- **[I-17] In-proc Arti only.** No external Tor processes are spawned or managed by this crate; Arti runs in-proc under the `arti` feature.
- **[I-18] No `unsafe` on hot paths.** Any necessary `unsafe` must be cold-path, locally justified, and covered by tests.

---

## 2. Design Principles (SHOULD)

- **[P-1] Thin abstraction, stable traits.** Small `Dialer`/`Listener`/`Conn` traits; backend hidden via features.
- **[P-2] Fail fast, shed early.** Timeouts, quotas, structured rejects (`timeout`, `tls_error`, `peer_limit`, `arti_bootstrap`, `too_large`, `rate_limited`).
- **[P-3] Zero-copy where it matters.** Prefer `Bytes`/`writev`/`readv`.
- **[P-4] Backend symmetry.** TCP, TLS, Arti expose the same shapes & metrics taxonomy.
- **[P-5] Profile-aware defaults.** Micronode: shorter timeouts, stricter caps, amnesia **ON**. Macronode: scale via config.
- **[P-6] Test under both Tokio schedulers** (`current_thread` and `multi_thread`) and run **TSan** in CI.
- **[P-7] Prefer backpressure to buffering.** Reject early; expose reasons in metrics.
- **[P-8] Deterministic cancellation.** Every task tied to a `CancellationToken`; clearly bounded shutdown.

---

## 3. Implementation (HOW)

### [C-1] Config Surface

```rust
pub enum Backend { Tcp, Tls, Arti }

pub struct TransportConfig {
    pub backend: Backend,                    // Tcp | Tls | Arti
    pub name: &'static str,                  // instance label
    pub addr: std::net::SocketAddr,          // listener bind
    pub max_conns: usize,                    // hard cap
    pub read_timeout: std::time::Duration,
    pub write_timeout: std::time::Duration,
    pub idle_timeout: std::time::Duration,
    pub tls: Option<std::sync::Arc<tokio_rustls::rustls::ServerConfig>>,   // server-side
    pub tls_client: Option<std::sync::Arc<tokio_rustls::rustls::ClientConfig>>, // dialer-side
    // Quotas & rate limiting (enforced before work)
    pub max_frame_bytes: usize,              // default 1 MiB
    pub io_rate_bytes_per_sec: Option<u64>,  // per-conn
    pub global_rate_bytes_per_sec: Option<u64>,
}
```

> Dialer-side options (e.g., `ClientConfig`) are injected at construction time via a builder and **also** use `tokio_rustls::rustls::ClientConfig`.

### [C-2] Reader/Writer Split

Use `tokio::io::split(stream)`; spawn **one writer task**; cancel-safe with `CancellationToken`.

### [C-3] Deadlines, Quotas & Backpressure

Wrap I/O in `tokio::time::timeout`; enforce quotas with `tokio::sync::Semaphore` and pre-read length guards; expose rejects in metrics (`rejected_total{reason="timeout|too_large|rate_limited|peer_limit|arti_bootstrap"}`).

### [C-4] Rate Limiter (token bucket)

```rust
pub enum TransportErrorKind { Timeout, PeerLimit, RateLimit, TooLarge, Tls, Arti, IoClosed }

pub struct RateLimiter { /* capacity, tokens, fill rate… */ }

impl RateLimiter {
    pub fn try_consume(&self, n: u64, now_ns: u64) -> bool { /* pacing logic */ }
}
```

### [C-5] Error Taxonomy

```rust
#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum TransportError {
    #[error("timeout during {op}")] Timeout { op: &'static str },
    #[error("peer limit exceeded")] PeerLimit,
    #[error("rate limited")]       RateLimited,
    #[error("message too large")]  TooLarge,
    #[error("tls handshake failed")] Tls,
    #[error("arti not ready")]     ArtiBootstrap,
    #[error("io closed")]          IoClosed,
}
impl TransportError {
    pub fn kind(&self) -> &'static str { /* stable string mapping */ }
}
```

### [C-6] Readiness Contract

```rust
#[derive(Clone, Debug)]
pub enum ReadyState {
    Ready,
    Degraded { reason: &'static str }, // e.g., "arti_bootstrap"
    NotReady { reason: &'static str }, // e.g., "bind_failed"
}
pub trait TransportHealth { fn readiness(&self) -> ReadyState; }
```

### [C-7] Arti Backend

Compiled only with `--features arti`. Bootstrap status = `Degraded("arti_bootstrap")`; dials fail with `TransportError::ArtiBootstrap` until bootstrap completes. Under **amnesia**, Arti uses in-memory state only; no disk persistence.

### [C-8] Testing Hooks

* `mock::Duplex` dialer for in-proc tests.
* `LoopbackListener` for integration tests.
* `loom` configs for small, deterministic interleavings of writer/inbox/cancel paths.

---

## 4. Acceptance Gates (PROOF)

- **[G-1] Unit/property tests.**
  - Single-writer enforcement.
  - Timeouts cause `Timeout` + `rejected_total{reason="timeout"}`.
  - Borrowed buffers never escape (compile-time + drop tests).

- **[G-2] OAP/1 ceilings & decompression.**
  - Frames > **1 MiB** → `TooLarge`, reject metric increments.
  - Streaming ~ **64 KiB** passes.
  - Decompression inflation tests show **≤ 8×** expansion enforced.

- **[G-3] Feature matrix CI.**
  - Build/test: `--no-default-features`, `--features tls`, `--features arti`, `--features tls,arti`.

- **[G-4] Concurrency gates.**
  - Clippy denies `await_holding_lock`.
  - Tests pass under **Tokio current_thread** and **multi_thread**.
  - **Loom** model(s) for writer/inbox/cancel pass.
  - **TSan** green.
  - Cancellation: kill conn mid-transfer → all tasks stop ≤ **100 ms**.

- **[G-5] Rate limiting proof.**
  - Write loops at 10× throughput pace; verify pacing within ±20% and `rejected_total{reason="rate_limited"}` increments.

- **[G-6] Readiness proof.**
  - Arti backend starts **Degraded("arti_bootstrap")** until bootstrap completes; flips to `Ready` within **1 s**; emits a single transition event.

- **[G-7] Observability smoke.**
  - Dials/accepts increase counters; handshake latency buckets fill; rejects carry stable `reason` strings.
  - JSON logs include `{conn_id, corr_id, backend, reason}` fields.

- **[G-8] PQ posture.**
  - With PQ-hybrid TLS available in the stack, crate compiles unchanged; enabling is **config-only**.
  - **Hybrid KEX vector**: when a PQ-hybrid profile is enabled (e.g., X25519+Kyber via rustls config), the handshake succeeds and records `handshake_latency_seconds{backend="tls", pq="hybrid"} > 0`, with compatibility smoke tests passing.

- **[G-9] Canon audit.**
  - CI grep denies `svc-arti-transport`; crate exists once in canon, Pillar 10.

- **[G-10] Soak & chaos.**
  - 24h soak on reference rig: no FD/mem leaks, steady buckets/throughput.
  - Chaos: induced dial failures / bootstrap stalls produce bounded retries upstream and stable rejects here.

---

## 5. Anti-Scope (Forbidden)

- ❌ Overlay sessions, gossip, or DHT logic.
- ❌ Running/managing external Tor daemons (Arti lib only, in-proc).
- ❌ Parsing OAP frames (only enforce I/O caps).
- ❌ Using `rustls::ServerConfig` directly (must be `tokio_rustls::rustls::*`).
- ❌ Unbounded queues; locks across `.await`; multi-writer sockets.
- ❌ `unsafe` in hot paths.
- ❌ Policy loops (retry/backoff/jitter) — that lives upstream.

---

## 6. References

- **12 Pillars (2025):** Pillar 10, transport boundaries.
- **Full Project Blueprint v2.0:** OAP/1 limits, Arti merge.
- **Concurrency & Aliasing Blueprint v1.3:** Single-writer, no-locks, owned bytes.
- **Hardening Blueprint v2.0:** Timeouts, rejects, amnesia, decompression guards.
- **Scaling Blueprint v1.4:** Micronode vs Macronode defaults; soak.
- **Six Concerns:** SEC/RES/PERF gates.
- **TLS 1.3 RFC & PQ hybrids.**
- **Zeroize crate docs** for amnesia.

---

### Reviewer Quick-Checklist

- [ ] Library-only, no overlay/DHT.
- [ ] Arti gated by `--features arti`; no `svc-arti-transport` remnants.
- [ ] TLS config types = `tokio_rustls::rustls::{ClientConfig, ServerConfig}`.
- [ ] Single-writer per socket; cancel-safe.
- [ ] OAP/1 caps enforced (≤ 1 MiB, ~ 64 KiB chunks) **and** 8× decompression guard (+ rationale).
- [ ] Metrics taxonomy present; buckets fixed; rejects visible with stable reasons.
- [ ] Rate limiting enforced and observable.
- [ ] Readiness states explicit (`Ready/Degraded/NotReady`) with Arti bootstrap semantics.
- [ ] Amnesia honored; PQ toggles are pass-through; **hybrid KEX test vector** passes when enabled.
- [ ] CI: Clippy wall, feature matrix, **Loom + TSan**, 24h soak green.

---

**Why this IDB matters:**  
It fixes the crate’s **laws** (invariants), codifies **preferences**, embeds **patterns** developers can copy-paste, and makes each invariant **provable** via gates. It also guards the borders with **anti-scope**. The result: `ron-transport` stays lean, safe, PQ-ready, and drift-proof, perfectly aligned with the canon.
````

---

