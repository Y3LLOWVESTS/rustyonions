### BEGIN NOTE - svc-overlay - OCTOBER 25 2025 - 15:00 CST

>TLDR: We accidentally skipped building ron-transport and began building svc-overlay instead so we are leaving notes here so we can come back and continue working on svc-overlay after ron-transport is completed. 

Absolutely—here’s a tidy, “pick-up-where-we-left-off” note pack for **svc-overlay**, so we can jump to **ron-transport** now and return later without losing context.

# svc-overlay — parking lot notes (return plan)

## Snapshot: what’s in place

* **Project wiring**

  * Crate scaffold matches TODO tree (admin/, readiness/, gossip/, protocol/, transport/, conn/, etc.).
  * `Cargo.toml` currently **does not** require `ron-transport` to avoid workspace build breaks.
* **Admin plane**

  * Routes: `/healthz` (liveness), `/readyz` (truthful readiness with “missing” list), `/version` (build info).
  * Metrics endpoint path prepared; two options:

    * via `ron-metrics` exporter (earlier variant), or
    * via `observe.rs` Prometheus recorder (alternate).
  * Either way, metrics are separated from data plane and cheap to serve.
* **Readiness truth model**

  * Gate structs implemented with setters for:

    * `listeners_bound`, `metrics_bound`, `cfg_loaded`, `queues_ok`, `shed_rate_ok`, `fd_headroom`
    * Reserved optionals: `pq_ready`, `tor_bootstrap`
  * `/readyz` returns `503` + list of missing gates until listener and transport are bound.
* **Config**

  * Validated defaults aligned with docs:

    * `oap_max_frame = 1 MiB`, default send/recv window = `16/16`, admin bind addresses.
  * Env loading hooks exist; CLI is a minimal stub (expand later).
* **Runtime skeleton**

  * `supervisor.rs`: service loop + clean shutdown.
  * `listener.rs`: stub (to be wired to transport).
  * `gossip/engine.rs`: bounded mpsc lane scaffolded.
  * `protocol/*`: OAP/1 placeholders (frames/handshake/flags/errors).
  * `conn/*`: reader/writer/supervisor shells exist for split responsibilities.
* **Observability**

  * Names reserved for golden metrics (`overlay_*`) and HTTP metrics (`http_requests_total`, `request_latency_seconds`, etc.).
  * Ready to emit once we wire listener/transport and message paths.

## Known gaps (by design, to unblock ron-transport first)

* No real **transport** yet (TLS/QUIC/Tor is delegated to ron-transport).
* No **listener bind** or per-connection task wiring (awaiting ron-transport API).
* OAP/1 **handshake implementation** is stubbed.
* Metrics counters/gauges/histograms are defined/reserved, but most increments are no-ops without transport/gossip traffic.
* Queue-depth sampler is stubbed; `/readyz` “queues_ok” is forced true for now.

## Return plan after ron-transport is ready

When we come back, follow this order. Each bullet is a small, shippable PR.

1. **Bind listener + flip readiness**

   * Implement `listener.rs` to bind the socket using ron-transport’s `Transport::bind()` (TLS by default; QUIC/Tor behind features).
   * On successful bind, set `listeners_bound = true`.
   * Graceful stop via shutdown signal.

2. **Per-connection lifecycle (reader/writer split)**

   * In `conn/supervisor.rs`, accept from transport, spawn:

     * `conn/reader.rs` (frame parse → channel to engine),
     * `conn/writer.rs` (bounded egress; one writer per conn).
   * Enforce timeouts from ron-transport’s `TransportConfig` (read/write/idle); no locks held across `.await`.

3. **OAP/1 handshake**

   * Implement `protocol/handshake.rs`: version check + capability flags + PQ posture negotiation (hybrid when enabled).
   * Emit structured errors via `protocol/error.rs`.

4. **Gossip lane integration**

   * Connect reader outputs → `gossip::engine` → broadcast with backpressure (drop counts increment metrics).
   * Add admission checks via `auth/macaroon.rs` for “send gossip” capability.

5. **Metrics wiring (overlay_*)**

   * Counters: `overlay_sessions_open_total{reason}`, `overlay_frames_total{dir,kind}`, `overlay_bytes_total{dir}`, `overlay_queue_dropped_total{queue,reason}`.
   * Gauges: `overlay_sessions_active`, `overlay_queue_depth`.
   * Hists: `overlay_frame_size_bytes{dir}` with buckets up to 1 MiB.
   * Hook increments in accept path, reader parse, writer send, and gossip ingress/egress.

6. **Readiness sampler**

   * Implement `readiness/sampler.rs`:

     * Track queue depth/backpressure → set `queues_ok`.
     * File descriptor headroom probe → set `fd_headroom`.
     * Shed-rate guard (reject rate over threshold) → set `shed_rate_ok = false` to flip `/readyz` red under sustained saturation.

7. **Config/CLI hardening**

   * Finish env/flag mapping (timeouts, windows, max frame, PQ mode, Tor/QUIC features).
   * Validate all bounds (no zero windows, reasonable timeouts, 1 MiB frame max).

8. **Contract tests & benches**

   * `tests/http_contract.rs` for `/healthz` `/readyz` `/version` schemas.
   * Integration: handshake, streaming, and readiness under pressure.
   * Benches: `oap_codec.rs`, `handshake.rs` using ron-transport I/O.

## Dependencies on ron-transport (what we need from it)

* **APIs**

  * `Transport::bind(addr, config) -> Listener`
  * `Listener::accept() -> TransportConn`
  * `TransportConn::split() -> (Reader, Writer)` or accessors for AsyncRead/AsyncWrite
  * Config struct with **read/write/idle timeouts**, buffer caps, optional TLS/PQ/Tor flags.
  * Feature flags: `tls` (default), `pq` (hybrid KEX), `quic`, `arti` (Tor via Arti).
* **Semantics**

  * One-writer discipline easy to enforce (Send + !Sync writer handle preferred).
  * Bounded read frame (1 MiB hard cap) and clear error taxonomy for oversize frames, timeouts, bad TLS, PQ mismatch.
  * Clean shutdown: half-close and cancellation propagate without hangs.

## Risks / gotchas to remember

* Don’t reintroduce `ron-transport` as a hard dep until that crate exists; use optional feature or add path later.
* Maintain low-cardinality metrics labels (avoid peer IDs as labels).
* `/readyz` must be **truthful**: set gates only when the underlying condition is actually satisfied (no early greens).
* Avoid holding locks across `.await`; use bounded channels for backpressure.
* Keep the admin plane on a separate bind from data plane in production (same process is fine; separate sockets).

## Quick resume checklist

When we return to svc-overlay:

* [ ] Restore `ron-transport` in `Cargo.toml` (path dep; add feature gating).
* [ ] Implement `listener.rs` using ron-transport and flip `listeners_bound`.
* [ ] Fill `conn/*` + `protocol/handshake.rs`.
* [ ] Wire gossip engine and metrics increments.
* [ ] Implement `readiness/sampler.rs` probes.
* [ ] Expand `Config` + CLI/env mapping and revalidate bounds.
* [ ] Run `cargo clippy -- -D warnings`, all tests, and smoke `/readyz` under load.

---

### END NOTE - OCTOBER 25 2025 - 15:00 CST

### BEGIN NOTE - OCTOBER 25 2025 - 19:45 CST

### CARRY OVER NOTES FROM ron-transport THAT ARE IMPORTANT FOR THIS CRATE

Absolutely—here are the focused “carryover notes” from `ron-transport` to guide and de-risk the build of `svc-overlay`. Drop this straight into your notes.

# Carryover Notes from `ron-transport` → `svc-overlay`

## 1) What `svc-overlay` can rely on (contracts & invariants)

* **Transport model**

  * Byte-stream sockets (TCP) with optional **TLS** termination via `tokio-rustls`.
  * Per-connection **single writer** (queue) and a reader loop. No concurrent writers—`svc-overlay` should **not** attempt parallel writes on the same connection handle.

* **Resource limits & timeouts**

  * Max connections via `Semaphore` (`TransportConfig.max_conns`).
  * **Read timeout** (per read) and **Idle timeout** (no traffic).
  * When idle triggers, transport closes the connection **gracefully** (FIN / TLS `close_notify`).

* **Graceful shutdown**

  * Public API: `spawn_transport_with_cancel(...) -> TransportHandle { task, addr, cancel }`.
  * Calling `cancel.cancel()` requests the accept loop to exit; live connections finish naturally. Overlay should expose a higher-level shutdown that forwards to transport.

* **Metrics available**

  * `connections{service=name}`
  * `bytes_in{service=name}`
  * `bytes_out{service=name}`
  * `rejected_total{service=name, reason}`
  * `latency_seconds{service=name}` (per-connection lifetime)
  * Overlay should **re-use** the `service=name` label value to keep dashboards aligned across layers.

* **Features & versions**

  * TLS gated behind feature `tls` (uses `tokio-rustls 0.26`, `rustls-pemfile 2.2`).
  * Transport is intentionally **protocol-agnostic** (no HTTP framing). Overlay provides framing/semantics (OAP/1).

## 2) Integration points `svc-overlay` should implement

* **Framing (OAP/1)**

  * Overlay adds message boundaries on top of byte streams (length-prefix or chunked). Transport provides a raw `AsyncRead/AsyncWrite` abstraction via reader/writer tasks.
  * Keep **max frame** ≤ 1 MiB (transport’s cap assumption). Validate in overlay to fail early.

* **Backpressure**

  * Transport writer uses a bounded MPSC queue (size 64). Overlay should:

    * Avoid enqueuing arbitrarily large bursts.
    * Consider **awaiting** writer send futures or implementing per-peer quotas (rate limit window) to avoid enqueue blowups.

* **Connection lifecycle events**

  * Transport reserves a `Bus<TransportEvent>` but doesn’t emit yet.
  * Overlay should prepare to **subscribe/forward**:

    * `Connected { peer, name }`
    * `Disconnected { peer, name, reason }`
  * As soon as the Bus publish API is finalized, transport will emit; overlay can consume for routing, state cleanup, and metrics.

* **Error taxonomy & retries**

  * Read timeouts → overlay should treat as **soft disconnect** (no retry loops inside transport).
  * Over-capacity rejection → overlay can apply **admission control** upstream (e.g., drop dial attempts or backoff).
  * TLS handshake failure → **surfaced as accept error**; overlay should log and meter by reason.

* **Health & readiness**

  * Transport sets readiness when bound. Overlay should:

    * Block “serve traffic” readiness until **both**: transport bound **and** overlay subsystems initialized (routing tables, key material, caches).
    * Export a composite health endpoint that includes transport readiness.

* **Shutdown choreography**

  * On overlay shutdown:

    1. Stop accepting overlay-level work (ingress gate).
    2. Drain in-flight overlay messages.
    3. Call `transport.cancel.cancel()` and `await transport.task`.
    4. Release resources (metrics registries, caches).

## 3) Data shapes & types (use as canonical)

* `TransportConfig` (overlay should clone and extend):

  * `addr: SocketAddr`
  * `name: &'static str` (label used in metrics—overlay should keep this stable)
  * `read_timeout: Duration`
  * `idle_timeout: Duration`
  * `max_conns: usize`

* `TransportHandle`

  * `task: JoinHandle<()>`
  * `addr: SocketAddr`
  * `cancel: Cancel`

* Events (reserved; expect overlay consumption soon):

  * `TransportEvent::Connected { peer: SocketAddr, name: &'static str }`
  * `TransportEvent::Disconnected { peer: SocketAddr, name: &'static str, reason: DisconnectReason }`

## 4) How `svc-overlay` should structure its I/O with transport

* **Reader path**

  * Overlay should read from transport’s per-connection reader (already running). Implementation detail: transport currently drives the reader loop internally; overlay defines **what to do with bytes**:

    * Provide a **decode/framing** stage (OAP/1) pulling from the bytes stream.
    * Enforce frame length limits (≤ 1 MiB), checksum if required by OAP/1.
    * Surface parsed messages to overlay services.

* **Writer path**

  * Overlay calls transport’s **writer handle** to send framed bytes.
  * Count application-level metrics separately (e.g., `oap_frames_out`) and let transport continue counting `bytes_out`.

* **Concurrency discipline**

  * One writer handle per connection; overlay can **fan-in** multiple overlay tasks by enqueuing to that writer. Keep message ordering guarantees in overlay (if required by OAP/1).

## 5) Metrics alignment (do this in `svc-overlay`)

* Re-export/augment transport metrics:

  * Add `oap_frames_in/out`, `oap_rejects{reason}`, `oap_decode_errors`.
* Share `service=name` label value with transport so Grafana dashboards can aggregate across layers.
* Consider tagging peers: `{peer="ip:port"}` sparingly (cardinality!). Prefer sampling or per-reason totals.

## 6) Configuration flow (from overlay down)

* Overlay config should embed or reference `TransportConfig`:

  * `overlay.toml` → `[transport] { addr, max_conns, read_timeout, idle_timeout, name }`
* TLS:

  * When `feature = "tls"`, overlay loads PEM cert/key (or receives a `TlsServerConfig`), then passes `Some(Arc<TlsServerConfig>)` to transport.
  * Provide a toggle to run **plain** for local dev.

## 7) Operational playbook (devs & CI)

* **Local**

  * HTTP smoke (human-visible): `cargo run -p ron-transport --example http_echo` then `http_echo_smoke.sh`.
  * TLS: `mk_self_signed.sh` once, then `smoke_tls.sh`.
* **Tests**

  * `cargo test -p ron-transport`
  * `cargo test -p ron-transport --features tls`
  * TLS test auto-skips if certs aren’t present.
* **When overlay integrates**:

  * Add overlay integration tests that:

    * Spin up transport + overlay.
    * Send a framed OAP/1 message over raw TCP/TLS.
    * Assert overlay response + transport `bytes_in/out` bump.

## 8) Known safe defaults to carry forward

* `read_timeout = 5s`, `idle_timeout = 30s` (tune per overlay workload).
* `max_conns` conservative start (e.g., 1024) with reject metrics observed.
* Writer queue size `64` per connection (match transport). If overlay bursts are larger, implement overlay-side batch/flush policy rather than inflating queue size.

## 9) Failure modes & guidance

* **Curl timeouts against transport** are expected (transport is not HTTP). Use `http_echo` example to validate human-visible flows.
* **Unexpected EOF on TLS** indicates a non-graceful close (older binary). Transport current code **does** send `close_notify`; ensure the right process is running.
* **Over-capacity**: transport will drop new accepts; overlay should expose admission control or queue-length saturation signals upstream.

## 10) Small TODOs for `svc-overlay` startup milestone

1. **Adopt `TransportHandle`**

   * Start/stop lifecycle aligned with overlay runtime.
2. **Implement OAP/1 framing**

   * Decoder with length checks; encoder mirroring max-frame.
3. **Wire transport metrics into overlay dashboards**

   * Add overlay counters; keep `service=name` aligned.
4. **Plumb Bus events (once available)**

   * Update state/routing tables on `Connected/Disconnected`.
5. **Integration tests**

   * End-to-end OAP frame round-trip over TCP and TLS (uses transport underneath).
6. **Graceful shutdown demo**

   * SIGINT → overlay stop → transport cancel → await all tasks.

---

### TL;DR for engineers

* Treat transport as a **reliable framed-message substrate** once overlay adds OAP/1 framing.
* Respect single-writer semantics; don’t bypass the writer queue.
* Bubble config, metrics, and shutdown coherently across overlay ↔ transport.
* Prepare to consume `TransportEvent` bus notifications ASAP.

### END NOTE - OCTOBER 25 2025 - 19:45 CST

### BEGIN NOTE - OCTOBER 25 2025 - 19:48 CST

Perfect—here’s a compact, return-ready packet you can paste into the repo. I’ll give you a drop-in doc plus a quick executive summary.

# Executive summary (for now)

* We paused **svc-overlay** mid-scaffold and completed **ron-transport** first.
* svc-overlay currently has: admin plane (`/healthz`, `/readyz`, `/version`), truthful readiness gate with setters, config defaults (1 MiB frame, 16/16 windows), supervisor/shutdown skeleton, and stubs for protocol/gossip/conn/transport. Metrics names are reserved, not fully wired.
* Next time: bind transport, implement OAP/1 framing + handshake, wire per-connection reader/writer with single-writer discipline, turn on overlay_* metrics, add readiness sampler, finish CLI/env, then integration tests over TCP+TLS.

---

# docs/RETURNING-TO-OVERLAY.md (paste-ready)

````markdown
# Returning to `svc-overlay` (post `ron-transport`)

This note captures where we paused, what we finished in `ron-transport`, and the exact steps to finish `svc-overlay` safely and fast.

---

## 0) State snapshot — svc-overlay today

**Implemented**
- Admin plane: `/healthz` (liveness), `/readyz` (truthful; lists missing gates), `/version`.
- Readiness gate: setters for `listeners_bound`, `metrics_bound`, `cfg_loaded`, `queues_ok`, `shed_rate_ok`, `fd_headroom` (+ reserved: `pq_ready`, `tor_bootstrap`).
- Config skeleton: `oap_max_frame = 1 MiB`, send/recv windows = `16/16`, admin binds; env loader; CLI stub.
- Runtime shell: `supervisor` + graceful shutdown; `listener` stub; gossip engine scaffold; protocol/conn stubs.
- Metrics: names reserved for `overlay_*` + HTTP metrics; exporter path in place, wiring pending.

**Intentionally missing (blocked by transport before)**
- Transport binding/accept loop.
- OAP/1 framing + handshake.
- Per-connection reader/writer tasks and quotas.
- Readiness sampler & full metrics wiring.

---

## 1) Carryover from `ron-transport` (contracts you can rely on)

**Transport model**
- Byte-stream sockets (TCP) with optional **TLS** (`tokio-rustls`).
- **Single writer** per connection (bounded MPSC queue). Overlay **must not** multi-write concurrently.

**Limits & timeouts**
- `TransportConfig`: `max_conns`, `read_timeout`, `idle_timeout`. Idle close is graceful (`FIN`/`close_notify`).

**Lifecycle**
- `spawn_transport_with_cancel(..) -> TransportHandle { task, addr, cancel }`.
- `cancel.cancel()` stops accept; live conns finish.

**Metrics (already emitted by transport)**
- `connections{service}`
- `bytes_in{service}`, `bytes_out{service}`
- `rejected_total{service,reason}`
- `latency_seconds{service}` (per-conn)
> Use the same `service=name` label in overlay to align dashboards.

**Features**
- `tls` feature for TLS; transport is protocol-agnostic (overlay adds OAP/1 framing).

---

## 2) Integration plan — small, shippable steps

### Step A — Bind listener + flip readiness
- Add `TransportConfig` into overlay config section; call `spawn_transport_with_cancel`.
- On successful bind, set `listeners_bound = true`.
- Expose a stop path that calls `cancel.cancel()` and awaits the transport task.

**Done when**: `cargo run -p svc-overlay` → `/readyz` goes green (200) immediately after bind.

### Step B — OAP/1 framing + handshake
- Implement length-prefixed decoder with **≤ 1 MiB** hard cap; reject oversize early.
- Define frame kinds + handshake caps; negotiate PQ posture (hybrid when compiled).
- Error taxonomy: oversize, version mismatch, timeout → structured metrics and clean disconnect.

**Done when**: round-trip echo frame test passes; oversize frame test fails with correct code & metric.

### Step C — Per-connection reader/writer
- `conn/supervisor.rs`: accept → spawn `reader.rs` (decode → engine) and `writer.rs` (bounded egress). Enforce **single writer**.
- Respect `read_timeout`/`idle_timeout` from `TransportConfig`.
- Admission: macaroon capability for “send gossip” (stub ok initially).

**Done when**: backpressure behaves (bounded queue), and connections cleanly close on idle.

### Step D — Gossip integration + quotas
- Wire reader → `gossip::engine` fanout; enforce fanout quota and drop policy.
- Increment drop metrics (`overlay_queue_dropped_total{queue,reason}`).

**Done when**: load test triggers measurable drops without memory growth.

### Step E — Observability & readiness sampler
- Wire `overlay_*` counters/gauges/histograms:
  - `overlay_sessions_active`
  - `overlay_sessions_open_total{reason}`
  - `overlay_frames_total{dir,kind}`
  - `overlay_bytes_total{dir}`
  - `overlay_frame_size_bytes{dir}`
  - `overlay_queue_depth`
  - `overlay_queue_dropped_total{queue,reason}`
- Implement sampler to set:
  - `queues_ok` (depth under ceiling)
  - `fd_headroom` (simple proc check)
  - `shed_rate_ok` (reject rate below threshold)
- Keep `/readyz` **truthful** under pressure.

**Done when**: chaos test flips `/readyz` to 503 during sustained shed.

### Step F — CLI/env + docs + tests
- Finalize env/CLI mapping; validate bounds (no zero windows; 1 MiB max frame).
- Contract tests for HTTP endpoints; integration for handshake & streaming (TCP & TLS).
- Benches for frame codec and handshake.

---

## 3) Config & types to standardize

```rust
// Overlay embeds transport cfg; keep `service` stable for metrics
pub struct TransportCfg {
  pub addr: SocketAddr,
  pub name: &'static str,       // metrics label
  pub read_timeout: Duration,
  pub idle_timeout: Duration,
  pub max_conns: usize,
  // tls: optional Arc<TlsServerConfig> behind a feature flag
}

pub struct OverlayCfg {
  pub oap_max_frame: usize,     // ≤ 1 MiB
  pub send_window_frames: u32,  // default 16
  pub recv_window_frames: u32,  // default 16
  pub transport: TransportCfg,
  pub amnesia: bool,
}
````

Defaults to carry over:

* `read_timeout=5s`, `idle_timeout=30s`, `max_conns=1024`.
* Writer queue size `64` (match transport).

---

## 4) Metrics alignment

**Transport emits:** `connections`, `bytes_{in,out}`, `rejected_total{reason}`, `latency_seconds`.
**Overlay adds:** `overlay_*` family above, plus `oap_decode_errors`, `oap_rejects{reason}`.
**Labeling:** always carry `service=name`; avoid high-cardinality peer labels (sample if needed).

---

## 5) Shutdown choreography (overlay)

1. Close ingress/admission (stop accepting new app work).
2. Drain in-flight frames (bounded).
3. `transport.cancel.cancel()`; await `transport.task`.
4. Report `/readyz` degraded if draining exceeds N seconds; finally return green → down during final tear-down.

---

## 6) Risks & guardrails

* Never multi-write on the same connection; obey single-writer discipline.
* Keep `/readyz` **honest**; don’t set gates until conditions are actually met.
* Enforce frame size cap in decoder (≤ 1 MiB).
* Keep metric labels low-cardinality; no peer IDs in labels.
* No locks across `.await`; use bounded channels for backpressure.

---

## 7) Acceptance checklist for M1 (“Transport-backed svc-overlay”)

* [ ] `/readyz` is green after listener bind; flips to red under pressure via sampler.
* [ ] OAP/1 handshake & framing implemented; oversize frames rejected with metrics.
* [ ] Per-conn reader/writer tasks respect timeouts; idle closes are graceful.
* [ ] overlay_* metrics populated; dashboards show transport + overlay aligned by `service`.
* [ ] TCP & TLS integration tests pass; benches for frame codec & handshake run in CI.
* [ ] Graceful shutdown flow demonstrated (SIGINT → cancel → await → clean exit).

---

### END NOTE - OCTOBER 25 2025 - 19:48 CST

### BEGIN NOTE - OCTOBER 26 2025 - 12:36 CST


awesome—here’s a tight, drop-in “carryover notes” packet so we can hand the baton and keep sprinting on **svc-overlay** next session.

# Carryover Notes — `svc-overlay`

## TL;DR (where we are)

* ✅ Admin HTTP is up with `/healthz`, `/readyz`, `/version`, `/metrics` (Prometheus text).
* ✅ Tracing is JSON, env-filtered; logs show listener + connection lifecycle.
* ✅ OAP/1 framing implemented (length-prefixed, 1-byte kind; max comes from `ron-proto`).
* ✅ Temporary **plain TCP** overlay listener binds and negotiates OAP/1 handshake, echoes `DATA` frames, and emits overlay metrics.
* ✅ Minimal **gossip engine** online (bounded MPSC + background task) and hooked to data path (best-effort publish).
* ✅ Build is **clean** with `-D warnings` and runs end-to-end echo.

---

## How to build/run/smoke (copy/paste)

### Fast CI-style checks (current crate only)

```
cargo fmt -p svc-overlay
cargo clippy -p svc-overlay --no-deps -- -D warnings
cargo build -p svc-overlay
```

### Run the server (Terminal A)

```
cargo run -p svc-overlay
```

Expected logs:

* `admin server listening on 127.0.0.1:9600`
* `overlay TCP listener bound (temporary)` (from `listener::plain`)
* `overlay supervisor running`

### Echo smoke (Terminal B)

```
cargo run -p svc-overlay --example libapi_embed
```

Expected:

* `negotiated: ver=1, caps=Caps(GOSSIP_V1)`
* `echo from overlay: hello, overlay!`

### Probe admin endpoints

```
curl -s localhost:9600/healthz | jq
curl -s -o /dev/null -w "%{http_code}\n" localhost:9600/readyz
curl -s localhost:9600/version | jq
curl -s localhost:9600/metrics | head -n 40
```

Look for these metrics moving after the echo:

* `overlay_frames_in_total`
* `overlay_frames_out_total`
* `overlay_bytes_in_total`
* `overlay_bytes_out_total`
* `gossip_ingress_total`
* `overlay_conn_lifetime_seconds_bucket`…

---

## What we accomplished (with pointers)

### Observability

* Prometheus recorder with stable buckets (`observe.rs`):

  * latency: `request_latency_seconds` (5ms..5s)
  * frame sizes: `overlay_frame_size_bytes` (512B..1MiB)
* Metric helpers (`emit::*`) for future HTTP routes.
* JSON tracing (`bootstrap::init_tracing`) with `EnvFilter`.

### Admin surface (`src/admin/*`)

* `/healthz`, `/readyz`, `/version`, `/metrics` wired via Axum.
* `ReadyProbe` truthfully gates readiness.

### Protocol (`src/protocol/*`)

* OAP/1 framing (`oap.rs`), `FrameKind::{Data, Ctrl}`, safe incremental parser, canonical max from `ron-proto`.
* Minimal `handshake` + `flags::Caps` (GOSSIP_V1).

### Listener & I/O (`src/listener/*`)

* **Feature-switchable** listener module:

  * `listener/plain.rs` (active): TCP accept → OAP/1 handshake → echo loop with metrics and gossip publish.
  * `listener/ron.rs` (placeholder): exports the same API, ready for `ron-transport`.
  * `features.transport_ron` registered; **off by default**.
* Metrics emitted on accept, frames in/out, bytes in/out, handshake failures by coarse reason.

### Gossip (`src/gossip/*`)

* `GossipEngine::start(cap)` spawns worker, counts ingress bytes, exposes `publish(Bytes)`.
* Global publish hook (OnceCell) installed via bootstrap helper.
* Listener publishes `DATA` frames best-effort (drops when full).

### Bootstrap (`src/bootstrap.rs`)

* Admin server spawner.
* `start_gossip_engine(capacity)` helper to bring the engine online.

---

## Current invariants/contracts to preserve

* **Single writer per connection** (we preserve this discipline; don’t parallelize writes).
* OAP/1 **max frame size** comes from `ron-proto::oap::MAX_FRAME_BYTES` (1 MiB today).
* **Label cardinality** kept low in metrics (`kind`, `reason` only).
* **Readiness** returns 503 until `listeners_bound && metrics_bound && cfg_loaded && queues_ok && shed_rate_ok && fd_headroom` (and any optional probes like PQ/Tor).

---

## What’s left (priority/order + done-when)

### 1) `ron-transport` integration (behind feature) — **High**

**Where:** `src/listener/ron.rs`
**What:** Replace current delegation with real transport:

* Build `TransportConfig` from overlay `Config` (addr, max_conns, timeouts, name).
* Call `spawn_transport_with_cancel(...)` and drive accepted streams through the same OAP/1 handshake + data path (reuse our parser/writer).
* Forward graceful shutdown (expose `ListenerHandle` that owns `JoinHandle` + cancel).

**Done-when:**

* `cargo run -p svc-overlay --features transport_ron` passes echo smoke (TCP).
* Handshake errors are metered via `overlay_handshake_fail_total{reason}`.
* Readiness waits for transport bound.

> Optional follow-ups: TLS (`--features tls` in both crates), QUIC/Tor later.

### 2) Gossip fanout (topics + subscribers) — **High**

**Where:** `src/gossip/engine.rs`
**What:** Add:

* Topic key (`Bytes` prefix or simple `String`) extracted from payload (for now, allow “raw” topic passed by caller).
* Per-topic bounded subscriber queues (MPSC), `subscribe(topic)` API → returns receiver.
* Backpressure/drop policy: drop newest with `gossip_dropped_total{reason="full"}`.
* Unit tests for fanout ordering, drop behavior, and capacity limits.

**Done-when:**

* Example demonstrates a subscriber receiving echoed `DATA` messages by topic.
* Metrics: `gossip_subscribers`, `gossip_fanout_total`, `gossip_dropped_total{reason}`.

### 3) AuthN/Z (macaroons/capability tokens) — **Medium**

**Where:** `src/auth/*`, call from listener before processing `DATA`.
**What:**

* Parse/authenticate bearer macaroon (or capability token) from the handshake/control frame.
* Minimal policy: allowlist of permitted `cap` for GOSSIP publish.
* Meter rejects: `overlay_oap_rejects_total{reason}`.

**Done-when:**

* Unauthorized handshake or publish is rejected with structured metric & log.
* Tests cover accept/reject paths.

### 4) Readiness truthiness under load — **Medium**

**Where:** `readiness/*` + listener/gossip wiring.
**What:**

* Wire `queues_ok` from gossip engine saturation (e.g., threshold on drops).
* Wire `fd_headroom` via a simple sampler (platform-gated if needed).
* Add a periodic sampler (`readiness/sampler.rs`) that updates the probe.

**Done-when:**

* Under synthetic pressure, `/readyz` flips 503 with `missing=["queues_ok"]` and recovery returns 200.

### 5) Config load/validate/watch — **Medium**

**Where:** `config.rs` (+ tie into `ron-kernel` config cell if present).
**What:**

* Read TOML, validate (`addr`, resource limits, TLS toggles).
* Hook to kernel watcher if available; otherwise file-load on startup.
* When config applies, update `ReadyProbe.cfg_loaded = true`.

**Done-when:**

* Bad config yields clear error at boot; good config sets readiness.
* Unit test for validation.

### 6) Post-quantum posture plumbing — **Low**

**Where:** `pq/*`, handshake flags.
**What:**

* Plumb PQ flags into handshake path; expose negotiated posture in logs/metrics.
* Tests that assert flag presence end-to-end (no crypto—just control bits).

**Done-when:**

* `pq_embed` example shows expected negotiation logs and counters.

### 7) Test suite & benches — **High coverage goal**

**Where:** `tests/`, `benches/`, `fuzz/`
**What to add:**

* `integration/overlay_oap_streaming.rs`: multi-frame round-trip.
* `overlay_admin_roundtrip.rs`: hit admin endpoints; verify JSON shape.
* Fuzzers (`fuzz_targets/oap_frame_parse.rs`): malformed/oversized/split frames.
* Bench: `benches/handshake.rs` + `benches/oap_codec.rs` already scaffolded—fill with realistic payload sizes.

**Done-when:**

* CI runs `cargo test -p svc-overlay` clean.
* Fuzz corpus includes `valid_frame`, `oversize_frame`, `split_frame` (already scaffolded).

---

## Useful commands / toggles

### Build with ron-transport feature (placeholder works; real wiring TBD)

```
cargo run -p svc-overlay --features transport_ron
```

### Trace more verbosely

```
RUST_LOG=svc-overlay=debug,ron-transport=warn,tower_http=warn cargo run -p svc-overlay
```

### Quick Prometheus spot-check

```
curl -s localhost:9600/metrics | rg 'overlay_|gossip_'
```

---

## File map highlights (what to open next)

* `src/listener/plain.rs` — data path + metrics + gossip publish (working).
* `src/listener/ron.rs` — **replace with real transport wiring**.
* `src/gossip/engine.rs` — **expand to topics/fanout**.
* `src/protocol/oap.rs` — framing & parser (stable).
* `src/admin/*` — handlers and readiness.
* `src/bootstrap.rs` — tracing/admin, `start_gossip_engine(…)`.

---

## Non-goals (for now)

* No DHT/routing; overlay ≠ router.
* No persistence; gossip is in-memory fanout.
* QUIC/Tor support deferred until `ron-transport` features land here.

---

## Risks & watch-outs

* **Metrics crate versions**: we’re using macros from `metrics 0.24`; keep exporter compatibility in mind (we pinned buckets via builder).
* **Cardinality creep**: avoid per-peer labels; we kept labels coarse.
* **Backpressure**: bounded MPSC is in place; add policy before increasing capacities.

---

## Definition of Done (crate)

* Transport integration behind feature works for TCP (and TLS w/ flag).
* Gossip fanout with topics + subscribers and clear drop policy.
* Readiness accurately reflects overload (queues) and config state.
* Auth path (macaroons/caps) enforced for publish.
* Test suite covers protocol parsing, admin JSON, and basic load.
* Docs in `docs/specs/OAP-1.md` and `docs/README.md` match behavior.
* `cargo clippy -D warnings` and examples all green.

---

If you want, I can immediately:

* implement **transport wiring** in `listener/ron.rs`, or
* add **gossip fanout** (topics + subscribe API) with a small example.


### END NOTE - OCTOBER 26 2025 - 12:36 CST

### BEGIN NOTE - OCTOBER 26 2025 - 18:07 CST

# svc-overlay — Carryover Notes (Exhaustive)

## 0) TL;DR (where we’re at)

* **Working:** admin plane (`/healthz`, `/readyz`, `/version`, `/metrics`), Prometheus exporter with real registry, live overlay TCP listener, minimal OAP/1 handshake, echo for Data frames, golden metrics wired, smoke + round-trip scripts, example OAP client.
* **Validated:** local builds pass `fmt` + `clippy -D warnings`; metrics surface and move; successful client round-trip.
* **Still to do:** swap temp listener to `ron-transport`, finalize readiness sampler, finish OAP/1 codec hardening, add tests/benches, integrate with supervisor lifecycle, finalize metrics set and labels.

---

## 1) What we accomplished

### Admin plane

* Implemented **admin server** with routes:

  * `GET /healthz` → `{"alive":true}`
  * `GET /readyz` → `{"ready":true}` or `503` with `{"ready":false,"missing":[...]}`
  * `GET /version` → static build info captured at compile time (via `build.rs`)
  * `GET /metrics` → **Prometheus** exposition (non-empty; includes build info)
* **Compatibility:** Restored `admin::router(probe, ver)` and `admin::ReadyProbe` API expected by your bootstrap/supervisor.
* **Build metadata:** `build.rs` injects `GIT_SHA` and `BUILD_TS` (std-only timestamp) so `/version` shows meaningful info.

### Metrics (Prometheus, backed by a real registry)

* **Exporter:** `admin/metrics.rs` exposes Prometheus text; uses `default_registry()`.
* **Self-initializing:** always returns data (no more `# no recorder`).
* **Golden metrics implemented:**

  * Gauges:

    * `overlay_sessions_active` — concurrent sessions (accept to close).
    * `overlay_build_info{version,git}` = `1`.
  * Counters:

    * `overlay_connections_total` — accept() count.
    * `overlay_frames_in_total{kind="data|ctrl"}`
    * `overlay_frames_out_total{kind="data"}`
    * `overlay_bytes_in_total`, `overlay_bytes_out_total`
    * `overlay_handshake_fail_total{reason="timeout|caps|preamble|io|other"}`
  * Histograms:

    * `overlay_accept_latency_seconds` — **accept → post-handshake** latency.
    * `overlay_conn_lifetime_seconds` — connection lifetime (accept → close).
* Wiring in `listener/plain.rs` ensures:

  * `overlay_sessions_active`: inc on accept; dec on **all** exit paths.
  * `overlay_accept_latency_seconds`: observed **exactly once** per accepted socket (RAII guard; fires on success or failure).

### Overlay listener & protocol path

* **Temporary plain `TcpListener`** bound from config; logs and gates readiness.
* **Minimal OAP/1 handshake** (`handshake()`), then **echo** for Data frames.
* **Gossip integration:** Data frames are `publish()`ed best-effort into the minimal gossip engine.
* **Error handling:** parse/write errors terminate the conn with proper metrics/logging.

### Example client + scripts

* **Example:** `examples/oap_client.rs`

  * Does a real handshake and one Data frame round-trip; verifies echo.
* **Scripts:**

  * `scripts/smoke_overlay.sh` — one-button smoke: build → run → open sockets (nc) → scrape metrics → shutdown.
  * `scripts/roundtrip_overlay.sh` — one-button full round-trip: launch server, run the **OAP client** N times, scrape metrics, shutdown.
* **Makefile targets (optional):** convenience wrappers for run/smoke/roundtrip.
* **CI suggestion:** GitHub Actions workflow to fmt/clippy/build & run round-trip (headless).

---

## 2) File changes you can point to (by path)

* `crates/svc-overlay/src/admin/mod.rs`

  * Admin router; **exports** `ReadyProbe`, `BuildInfo`; maintains readiness gates.
  * Routes: `/healthz`, `/readyz`, `/version`, `/metrics`.
* `crates/svc-overlay/src/admin/version.rs`

  * `BuildInfo`, `current_build_info()`, handler for `/version` (reads `AdminState`).
* `crates/svc-overlay/src/admin/metrics.rs`

  * Prometheus exporter; registers gauges/histograms; `overlay_metrics::*` helpers.
  * Ensures **non-empty** metrics via `overlay_build_info`.
* `crates/svc-overlay/src/listener/plain.rs`

  * Accept loop; per-conn task with handshake + echo; **RAII accept timer**; session gauge fidelity; counters/histograms.
* `crates/svc-overlay/build.rs`

  * Injects `GIT_SHA` (short 12) and `BUILD_TS` (unix:secs) via env.
* `crates/svc-overlay/examples/oap_client.rs`

  * Minimal client; successful handshake; send/echo Data frame.
* `crates/svc-overlay/scripts/smoke_overlay.sh`

  * Build, run, open sockets, scrape metrics, shutdown.
* `crates/svc-overlay/scripts/roundtrip_overlay.sh`

  * Build server + **only** `oap_client`; run server; wait for admin & TCP; run client N times; scrape metrics; shutdown.

---

## 3) How to run things (exact commands)

### Launch svc-overlay directly

```
cargo run -p svc-overlay
```

### Probe admin endpoints

```
curl -i http://127.0.0.1:9600/healthz
curl -i http://127.0.0.1:9600/readyz
curl -sSf http://127.0.0.1:9600/version
curl -sSf http://127.0.0.1:9600/metrics | head -n 40
```

### One-button smoke (accepts, timeouts, gauge/histogram ticks)

```
chmod +x crates/svc-overlay/scripts/smoke_overlay.sh
crates/svc-overlay/scripts/smoke_overlay.sh
```

### One-button round-trip (server + client + metrics)

```
chmod +x crates/svc-overlay/scripts/roundtrip_overlay.sh
crates/svc-overlay/scripts/roundtrip_overlay.sh 3
```

### Run the example OAP client manually (server must be running)

```
cargo run -p svc-overlay --example oap_client
```

### Optional Makefile targets (if added at repo root)

```
make overlay.run
make overlay.smoke
make overlay.roundtrip
```

---

## 4) Readiness gates (what `/readyz` means today)

* `listeners_bound` — listener bound and accepting.
* `metrics_bound` — exporter up (toggle point available; currently implied by admin start).
* `cfg_loaded` — config validated/applied (toggle available).
* `queues_ok` — (sampler placeholder) TX queues not saturated.
* `shed_rate_ok` — backpressure under control.
* `fd_headroom` — file descriptor headroom.
* **Current behavior:** returns `200` when all are true; otherwise `503` with `missing` gates.
* **Where gates flip:** `ReadyProbe` setters and `probe.set(|s| ...)` shim. The listener flips `listeners_bound=true`.

---

## 5) Metrics reference (names, semantics)

### Gauges

* `overlay_sessions_active`

  * Incremented on accept; decremented on **all** exits (handshake fail, parse error, close).
* `overlay_build_info{version,git} = 1`

  * Ensures `/metrics` is never empty; labels are build metadata.

### Counters

* `overlay_connections_total`

  * Count of successful `accept()` calls.
* `overlay_frames_in_total{kind="data|ctrl"}`

  * Number of frames parsed inbound.
* `overlay_frames_out_total{kind="data"}`

  * Number of frames written (currently echo for Data).
* `overlay_bytes_in_total`, `overlay_bytes_out_total`

  * Byte counts.
* `overlay_handshake_fail_total{reason="timeout|caps|preamble|io|other"}`

  * Classified handshake failures.

### Histograms

* `overlay_accept_latency_seconds`

  * Time from `accept()` to **post-handshake** code path; observed once per accepted socket (success or failure), via RAII guard.
* `overlay_conn_lifetime_seconds`

  * Accept → socket close, measured on EOF.

> **Planned split:** consider separating `overlay_accept_to_handshake_seconds` vs. `overlay_handshake_seconds` to isolate network accept queueing vs. protocol work.

---

## 6) Troubleshooting quickies

* **`/metrics` empty or missing:** fixed by exporter; if you see emptiness, ensure service is the new build. `overlay_build_info` guarantees non-empty.
* **`curl` shows nothing but service is up:** some clients buffer; `head` shows content. Prometheus format is plain text.
* **`nc` causes “Broken pipe” in logs:** expected—`nc` isn’t OAP/1. We still record latency and manage the session gauge.
* **`oap_client` “Connection refused”:** server isn’t running—use `roundtrip_overlay.sh` or start server first.
* **`cargo build --examples` fails on `pq_embed`:** we only build `oap_client`; if needed, gate `pq_embed` behind a feature.

---

## 7) What remains (targeted next steps)

### Transport & lifecycle

* **Swap temp `TcpListener` → `ron-transport`** (optionally TLS) once API is aligned.
* **Supervisor integration:** formal handle lifecycle, graceful shutdown, backpressure signaling.

### Readiness sampler (real signals)

* Implement periodic sampler to set:

  * `queues_ok` from gossip/tx queue depths vs. watermarks.
  * `fd_headroom` via process limits (conservative for macOS dev, robust in Linux).
  * `metrics_bound` once admin plane is serving; `cfg_loaded` after validated/apply.

### Protocol hardening (OAP/1)

* Strict preamble + versioning fallbacks; capability negotiation; length guards & DoS limits; ctrl frame handling; structured error codes.
* Add **property tests** for frame parse/encode, and **fuzz hooks** for preamble/lengths.

### Tests & benches

* **Unit tests:** handshake paths (success/failure), parser, writer, gossip publish, admin handlers.
* **Integration tests:** spawn server in-proc, run OAP client, assert metrics deltas.
* **Benches:** simple conn throughput and echo latency with `criterion`.

### Observability polish

* Add labels (e.g., `reason` on failures already present); consider `peer_class` when we classify clients later.
* Export a service `build_info` metric via a common label schema across services.

### Polish & hygiene

* Clippy stays clean (`-D warnings`).
* Feature-gated examples (`pq_embed`) to avoid CI noise.
* Document config schema; surface effective config via `/configz` (read-only) if desired.

---

## 8) Acceptance criteria for “svc-overlay MVP done”

* Admin plane healthy: `/healthz 200`, `/readyz 200` under normal conditions, `/metrics` non-empty, `/version` shows real sha & timestamp.
* Listener uses **ron-transport**; successful OAP/1 handshake with real clients; echo path validated end-to-end.
* Readiness gates reflect real state; under induced backpressure, `/readyz` flips to `503` with correct `missing` gates list.
* Golden metrics present and correct; gauges/counters/histograms move predictably; round-trip & smoke scripts pass.
* Test suite: unit + integration; CI green on fmt/clippy/tests.

---

## 9) Scripts (what they do / where they are)

### `crates/svc-overlay/scripts/smoke_overlay.sh`

* **Does:** `fmt` + `clippy` → run server → wait for `/healthz` → open/close raw TCP sockets via `nc` → scrape key metrics at each step → stop server.
* **Use:**

```
chmod +x crates/svc-overlay/scripts/smoke_overlay.sh
crates/svc-overlay/scripts/smoke_overlay.sh
```

### `crates/svc-overlay/scripts/roundtrip_overlay.sh`

* **Does:** `fmt` + `clippy` → build server + **example oap_client** → run server → wait for admin **and** TCP bind → run client N times → scrape metrics → stop server.
* **Use:**

```
chmod +x crates/svc-overlay/scripts/roundtrip_overlay.sh
crates/svc-overlay/scripts/roundtrip_overlay.sh 3
```

---

## 10) Optional convenience (if adopted)

### Makefile (root)

```
overlay.run:
	cargo run -p svc-overlay

overlay.smoke:
	crates/svc-overlay/scripts/smoke_overlay.sh

overlay.roundtrip:
	crates/svc-overlay/scripts/roundtrip_overlay.sh 3
```

### GitHub Actions (CI)

* Fmt + clippy
* Build `oap_client` example only
* Run round-trip script in headless mode
  (See suggested `svc-overlay.yml` in prior message.)

---

## 11) Open questions to align before next patch

* Should we **split** handshake metrics into accept→start vs. handshake duration?
* Which **labels** (if any) do we standardize across services (e.g., `service`, `instance`, `git`) vs. embedding in metric names?
* Required **transport features** for MVP (TLS now vs. later)?
* Should `/configz` (read-only) and `/debugz` be part of the admin surface in MVP?
* Any **compat constraints** for Prometheus buckets (histogram layout) we should match to existing dashboards?

---

## 12) Quick estimate snapshot (for downstream planning)

* **Progress:** ~60% (admin + metrics solid; temp listener + minimal OAP path working; scripts + example client done).
* **Remaining:** transport swap, readiness sampler, protocol hardening, tests/benches, polish.

---

### END NOTE - OCTOBER 26 2025 - 18:07 CST


### BEGIN NOTE - OCTOBER 26 2025 - 21:53 CST

## TL;DR — What’s left for `svc-overlay` so we can jump to `svc-dht`?

Minimal tie-offs for a **ron-core beta**:

1. **Gate the stray example** so `--examples` builds cleanly (no-op at runtime).
2. **Lock the public surface** (one-liner doc): endpoints, env knobs, feature flag.
3. **Sanity CI step** (fmt + clippy + roundtrip) so we don’t regress while working on DHT.

That’s it. Graceful shutdown, deep tests, and the real `ron-transport` swap can land later without blocking DHT.

---

# Paste into `NOTES.MD`

## svc-overlay — status + go/no-go notes (beta)

### What we’ve accomplished

* **Listener & handshake**: Accepts TCP, negotiates v1, `Caps(GOSSIP_V1)`, errors/timeouts recorded.
* **Frame path**: Parses OAP frames, echoes `Data`, publishes to gossip stub.
* **Metrics**: Prometheus ready; stable `overlay_*` (sessions, accept latency, conn lifetime, handshake fail, peer tx queue depth, drops, build_info).
* **Admin HTTP**: `/healthz`, `/readyz`, `/metrics` live; readiness flips based on TX queue depth.
* **Transport facade**: One codepath today (Tokio TCP); future flip to `ron-transport` is a local change in `src/transport/mod.rs` only.
* **Runtime tunables**:

  * `RON_OVERLAY_HANDSHAKE_MS` (default 2000ms)
  * `RON_OVERLAY_TX_WATERMARK` (default 96 of 128 queue)
* **Roundtrip script**: Green.
* **Soak (45s @ 24 conc)**: 7,872 launches, **0 failures**, `/readyz = true`.
  (We gated `pq_embed` below to fix the example build noise.)

### Beta gate ✅/❌

* Build (no warnings, default features): ✅
* Roundtrip smoke: ✅
* Light soak (45–60s, conc 16–24): ✅
* Minimal docs (endpoints + env knobs): ✅ (below)
* Examples build clean / or gated: ✅ (change below)
* Non-blocking items we’re punting (ok for beta):

  * Graceful shutdown drain/quiesce (can land later)
  * Integration tests asserting metrics under load
  * Real `ron-transport` integration (facade ready when API finalized)

### What’s left to flip “done” (quick)

1. **Gate stray example (compile noise fix)**
   In `crates/svc-overlay/Cargo.toml`, add:

   ```toml
   [[example]]
   name = "pq_embed"
   required-features = ["pq"]
   ```

   (Now `cargo build -p svc-overlay --examples` skips `pq_embed` unless you pass `--features pq`.)

   *Alternative*: add a stub `fn main(){}` to `examples/pq_embed.rs`—but gating is cleaner.

2. **Tiny doc block (public surface / how to run)**
   Add to `crates/svc-overlay/README.md`:

   ```md
   ## Admin
   - /healthz   : liveness
   - /readyz    : 503 until listeners bound AND tx queue depth < watermark
   - /metrics   : Prometheus, `overlay_*` instruments

   ## Env tunables
   - RON_OVERLAY_HANDSHAKE_MS (u64 ms, default 2000, clamps 100..=30000)
   - RON_OVERLAY_TX_WATERMARK (i64, default 96, clamps 1..=127)

   ## Features
   - use_ron_transport : reserved; today still maps to Tokio TCP via transport facade

   ## Quick smoke
   cargo run -p svc-overlay
   crates/svc-overlay/scripts/roundtrip_overlay.sh 2
   ```

3. **One-shot CI helper (optional, but fast)**
   Create `scripts/ci_overlay.sh` so we can reuse locally and in CI:

   ```
   #!/usr/bin/env bash
   set -euo pipefail
   cargo fmt --all --check
   cargo clippy -p svc-overlay --no-deps -- -D warnings
   crates/svc-overlay/scripts/roundtrip_overlay.sh 2
   ```

   (Make executable: `chmod +x scripts/ci_overlay.sh`)

---

## Hand-off requirements for `svc-dht` to start now

With the above two quick tweaks, `svc-overlay` gives DHT everything it needs:

* **Stable listening surface**: TCP listener, framed IO, handshake result available in logs/metrics.
* **Simple “session” semantics**: you can connect from a DHT client over the overlay and send frames (our echo path proves this).
* **Observability hooks**: per-conn lifetime, handshake failures, backpressure, readiness—all visible.

### DHT-first overlay contracts (no code changes needed today)

* Keep using **OAP `FrameKind::Data`** for DHT messages initially (we’ll add a `Ctrl`/`Dht` variant later if needed).
* Expect **best-effort** delivery per connection; overlay currently drops on full TX (counter increments). DHT should treat transport as lossy and retry.
* Overlay handshake `Caps(GOSSIP_V1)` is present but not yet used by DHT; okay for beta.

---

## Next: `svc-dht` bootstrap plan (actionable, minimal)

1. **Crate skeleton**

   ```
   cargo new crates/svc-dht --bin
   ```

   * `svc-dht` depends on `ron-proto` (message types) and (for now) **uses overlay as a remote** (no in-proc embedding yet).

2. **Protocol messages (ron-proto)**

   * Add minimal `DhtMsg` enum (Put, Get, Found, Ping/Pong) with CBOR/JSON via serde.
   * Serialize `DhtMsg` into OAP `Frame.payload` for transport (reusing the overlay echo path to deliver).

3. **Client shim**

   * Small helper in `svc-dht` to open a TCP connection to an overlay peer and push/pull `FrameKind::Data`.
   * Reuse the same framing as `oap_client` example in overlay.

4. **DHT operations (MVP)**

   * `put(key, value)` → broadcast (or limited fanout) via overlay sessions.
   * `get(key)`       → query peers; first response wins; naive timeout.
   * In-memory table (HashMap) for now; persistence later.

5. **Demo roundtrip**

   * New script `crates/svc-dht/scripts/roundtrip_dht.sh`:

     * start `svc-overlay`
     * launch two `svc-dht` nodes (A,B)
     * `put` on A, `get` on B, assert value

6. **Observability**

   * Counters: `dht_put_total`, `dht_get_total`, `dht_found_total`, `dht_timeout_total`
   * Histogram: `dht_get_latency_seconds`

This lets us iterate on the DHT logic while overlay stays stable underneath.

---

## One small code tweak you can apply now (optional)

If you want the example build to be quiet immediately (even without the Cargo gating), add this **tiny stub**:

**`crates/svc-overlay/examples/pq_embed.rs`**

```rust
/* placeholder example, gated by `pq` feature in Cargo.toml */
fn main() {
    // no-op; kept to reserve path for future PQ handshake demo
}
```

(But the Cargo `[[example]]` gating above is enough; you don’t need both.)

---

### Final call

* Do the example gating + README snippet.
* We’re **beta-ready** on `svc-overlay`.
* Let’s spin up `svc-dht` with the bootstrap plan above. If you want, I can generate the `svc-dht` crate scaffold and the initial `DhtMsg` types next.


### END NOTE - OCTOBER 26 2025 - 21:53 CST
