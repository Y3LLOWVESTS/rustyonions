# RustyOnions – Crate Map (FINAL)
_Generated: 2025-09-15 01:18:45_

# **Architecture style:** Microkernel + plane separation

* **Node plane:** peer/overlay side (svc-gateway, svc-overlay, svc-index, svc-storage, crypto/observability edges).
* **App plane:** developer/app side (svc-omnigate, Micronode, specialized edge nodes like map-node, cdn-node).
* **Core:** **ron-kernel** (health/metrics/config, supervision) + **ron-bus** (pub/sub event bus).
* **Security & policy libs:** ron-proto, ron-auth, ron-kms, ron-policy, ron-audit, ryker.
* **Transports:** pluggable behind a single trait (transport); TLS type fixed by kernel (tokio-rustls).

---

## Topology at a glance

```
 (Peers)                                      (Apps/Sites/SDKs)
    |                                               |
┌───▼──────────┐                              ┌─────▼──────────┐
│  svc-gateway │  (Node ingress)              │  svc-omnigate  │  (App BFF/API)
└───┬──────────┘                              └────┬───────────┘
    │ OAP/1                                          │ REST/GraphQL/WS
    │                                                │
    │    ┌──────────────────────┐    Bus (ron-bus)   │
    │    │      ron-kernel      │◄───────────────────┤
    │    │ (health, metrics)    │                    │
    │    └───┬───────────┬──────┘                    │
    │        │           │                           │
┌───▼───┐ ┌──▼─────┐  ┌──▼──────┐               ┌────▼─────┐    ┌──────────┐
│Index │ │Overlay  │  │ Storage │               │ map-node │    │ cdn-node │
│(svc) │ │ (svc)   │  │  (svc)  │               │  (svc)   │    │  (svc)   │
└──────┘ └─────────┘  └─────────┘               └──────────┘    └──────────┘

(Shared across planes)
- DTOs/errors: ron-proto    - Auth envelopes: ron-auth   - Quotas: ron-policy
- Keys/KMS: ron-kms         - Audit trail: ron-audit     - Actors/supervisor: ryker
- Transport backends: arti-transport / tcp-transport / quic-transport (future)
```

> **Naming note:** We retain legacy names where they appear in **ALL_SUMMARIES.md** to keep the count accurate. Standardization guidance: `gateway`→`svc-gateway`, `index`→`svc-index`, `overlay`→`svc-overlay`, and `arti_transport`→`arti-transport` for code identifiers.

## Index of crates (as listed in ALL_SUMMARIES.md)

| Crate | Role (from summaries) | One‑liner | Plane | Depends on | Key Outputs |
| --- | --- | --- | --- | --- | --- |
| **accounting** | lib | Tiny, std-only byte accounting: a `CountingStream<S>` wrapper and shared `Counters` that track total bytes in/out and rolling per-minute buckets (60-slot ring). | — | serde, time | Ring buffers; minute windows; snapshot/rotation |
| **arti\_transport** | lib | SOCKS5-based outbound (Tor/Arti) plus a tiny Tor control-port client to publish a v3 hidden service and hand connections to the core transport `Handler`. | — | — | — |
| **common** | lib | Shared foundation crate: hashing (BLAKE3), address formatting/parsing, `NodeId`, and a small `Config` loader used across services. | — | anyhow/thiserror, serde | Common helpers/macros |
| **gateway** | lib | HTTP façade and OAP/1 ingress for RustyOnions: serves `b3:<hex>.<tld>` bundles over Axum with ETag/Range/precompressed negotiation, optional payment enforcement, quotas, and Prometheus metrics; plus a lightweight OAP server for framed ingest/telemetry. | — | — | — |
| **index** | lib | A tiny library that maintains and queries the mapping from a content address (`b3:<hex>`) to currently known providers, enforcing TTLs and signatures, so callers can resolve where to fetch bytes without speaking the DHT directly. &#x20; | — | — | — |
| **kameo** | lib | A lightweight, in-process actor toolkit (mailboxes, supervisors, and typed request/response) for building RustyOnions services with predictable concurrency, backpressure, and restarts. | — | — | — |
| **naming** | lib | Defines the **canonical content address syntax** for RustyOnions (`b3:<64-hex>.<tld>`), plus helpers to parse/validate/normalize addresses and to read/write **Manifest v2** metadata. | — | serde | Strong IDs; normalization rules |
| **node** | lib (primarily a CLI binary with a tiny placeholder lib) | A developer-facing CLI that runs a single-process RustyOnions **overlay listener** and offers **PUT/GET** helpers (TCP and optional Tor), plus a tiny JSON stats socket for quick local testing. | Deploy | ron-kernel, ron-bus, selected svc-* | Launch profiles (dev/prod/hardened) |
| **oap** | lib | A tiny, dependency-light codec + helpers for **OAP/1** that parse/serialize frames, enforce bounds, and provide canonical DATA packing (with `obj:"b3:<hex>"`) used by the SDK, gateway, and services. &#x20; | — | ron-proto, serde | OAP/1 codec; frame/limit enforcement |
| **overlay** | lib | A legacy library that bundles a simple TCP overlay protocol (PUT/GET), a sled-backed blob store, and convenience client/server helpers into one crate. | — | — | — |
| **ron-app-sdk** | sdk | A lightweight, async Rust client SDK for apps to speak **OAP/1** to a RustyOnions node/gateway—handling TLS, HELLO negotiation, capability tokens, and streaming PUT/GET of content-addressed blobs. | — | ron-proto | Typed client SDK |
| **ron-audit** | lib | Tamper-evident audit logging: append signed, hash-chained records (Ed25519 + BLAKE3) with an optional filesystem sink. | — | ed25519-dalek, blake3 | — |
| **ron-auth** | lib | Zero-trust message envelopes for internal IPC: sign and verify structured headers/payloads using HMAC-SHA256 with scopes, nonces, and time windows, backed by pluggable key derivation. | — | hmac/sha2 or ed25519-dalek, uuid | — |
| **ron-billing** | lib | Lightweight billing primitives: compute request cost from a manifest `Payment` policy and validate basic billing fields (model, wallet, price) without performing settlement. | — | accounting, naming, time | Usage records; billable events; roll-ups |
| **ron-bus** | lib | Tiny IPC layer for RustyOnions services: MessagePack-framed request/response envelopes over Unix Domain Sockets with minimal helpers for `listen / recv / send`. | — | tokio, tracing, rmp-serde | `Bus`, core events; broadcast |
| **ron-kernel** | core | A tiny, stable microkernel that provides the project’s event bus, metrics/health surfaces, transport glue, and shutdown/supervision hooks—while exporting a frozen public API for the rest of the system.&#x20; | — | — | — |
| **ron-kms** | lib | KMS trait + dev in-memory backend for origin key derivation (HKDF), secret sealing, and rotation hooks—kept separate from envelope/auth logic. | — | hkdf/sha2, zeroize | — |
| **ron-proto** | lib | Single source of truth for cross-service DTOs/errors, addressing (BLAKE3-based digests), and OAP/1 protocol constants with JSON/optional MessagePack codecs. | — | serde, rmp-serde, blake3 | — |
| **ryker** | lib | A tiny supervisor for async tasks that restarts failing jobs with exponential backoff + jitter, with a temporary compatibility shim that re-exports legacy billing helpers. | — | — | — |
| **svc-crypto** | service | Local IPC “crypto concierge” that signs, verifies, hashes, and key-manages for other RustyOnions services via a simple Unix socket RPC. | Node | ron-bus, ron-proto, ryker, tracing (+ auth/policy as needed) | Sign/verify; seal/unseal; rotate |
| **svc-index** | service | Local Unix-socket index service that maps RustyOnions content addresses to bundle directories and answers resolve/put requests for peers. | Node | ron-bus, ron-proto, ryker, tracing (+ auth/policy as needed) | Resolve/announce; signed metadata |
| **svc-omnigate** | service | Multi-tenant gateway that fronts RustyOnions services, enforcing quotas/backpressure, exposing health/metrics/readiness, and brokering app traffic over the OAP/1 protocol to storage/index/mailbox while staying app-agnostic.&#x20; | App | ron-bus, ron-proto, ryker, tracing (+ auth/policy as needed) | App APIs; tenancy; audit hooks |
| **svc-overlay** | service | Thin overlay service that answers RPC `Health` and `Get{addr,rel}` over a local UDS “bus,” resolving content addresses via `svc-index` and returning file bytes from `svc-storage`. | Node | ron-bus, ron-proto, ryker, tracing (+ auth/policy as needed) | Routing; 64KiB streaming |
| **svc-storage** | service | Durable, content-addressed DAG store with rolling-hash chunking and Reed–Solomon erasure that serves objects to the overlay in 64 KiB streams, verifying BLAKE3 before returning bytes. &#x20; | Node | ron-bus, ron-proto, ryker, tracing (+ auth/policy as needed) | Get/put blobs; repair hooks |
| **tldctl** | lib | Library + thin CLI for **packing local files into content-addressed bundles** (BLAKE3 `b3:<hex>`) with manifest and precompressed variants, writing to the store root and index DB, and printing the canonical address. &#x20; | Ops | ron-proto, ron-auth | Operator CLI |
| **transport** | lib | Async TCP/TLS listener/dialer layer that enforces connection limits and per-socket timeouts, integrates with kernel health/metrics, and hands framed bytes to higher layers without embedding app semantics. &#x20; | — | tokio, tokio-rustls | `Transport` trait; `spawn_transport()` |

## Status snapshot

| Crate | Maturity | Last Reviewed | Owner | Path |
| --- | --- | --- | --- | --- |
| accounting | draft | 2025-09-14 | Stevan White | `crates/accounting` |
| arti\_transport | draft | 2025-09-14 | Stevan White | `crates/arti\_transport` |
| common | draft | 2025-09-14 | Stevan White | `crates/common` |
| gateway | draft | 2025-09-14 | Stevan White | `crates/gateway` |
| index | draft | 2025-09-14 | Stevan White | `crates/index` |
| kameo | draft | 2025-09-14 | Stevan White | `crates/kameo` |
| naming | draft | 2025-09-14 | Stevan White | `crates/naming` |
| node | draft | 2025-09-14 | Stevan White | `crates/node` |
| oap | draft | 2025-09-14 | Stevan White | `crates/oap` |
| overlay | draft | 2025-09-14 | Stevan White | `crates/overlay` |
| ron-app-sdk | draft | 2025-09-14 | Stevan White | `crates/ron-app-sdk` |
| ron-audit | draft | 2025-09-14 | Stevan White | `crates/ron-audit` |
| ron-auth | draft | 2025-09-14 | Stevan White | `crates/ron-auth` |
| ron-billing | draft | 2025-09-14 | Stevan White | `crates/ron-billing` |
| ron-bus | draft | 2025-09-14 | Stevan White | `crates/ron-bus` |
| ron-kernel | draft | 2025-09-14 | Stevan White | `crates/ron-kernel` |
| ron-kms | draft | 2025-09-14 | Stevan White | `crates/ron-kms` |
| ron-proto | draft | 2025-09-14 | Stevan White | `crates/ron-proto` |
| ryker | draft | 2025-09-14 | Stevan White | `crates/ryker` |
| svc-crypto | draft | 2025-09-14 | Stevan White | `crates/svc-crypto` |
| svc-index | draft | 2025-09-14 | Stevan White | `crates/svc-index` |
| svc-omnigate | draft | 2025-09-14 | Stevan White | `crates/svc-omnigate` |
| svc-overlay | draft | 2025-09-14 | Stevan White | `crates/svc-overlay` |
| svc-storage | draft | 2025-09-14 | Stevan White | `crates/svc-storage` |
| tldctl | draft | 2025-09-14 | Stevan White | `crates/tldctl` |
| transport | draft | 2025-09-14 | Stevan White | `crates/transport` |

---

## accounting

- **path:** `crates/accounting` · **role:** lib · **owner:** Stevan White · **maturity:** draft · **last-reviewed:** 2025-09-14

### 1) One-liner
Tiny, std-only byte accounting: a `CountingStream<S>` wrapper and shared `Counters` that track total bytes in/out and rolling per-minute buckets (60-slot ring).

### 2) Primary Responsibilities
* Provide a thread-safe in-memory counter for bytes read/written (totals + 60-minute ring).
* Offer a zero-alloc snapshot API for quick export to logs/metrics.
* Make it trivial to instrument any `Read`/`Write` with `CountingStream<S>`.

### 3) Non-Goals
* No Prometheus/HTTP export layer (consumers must expose).
* No quotas/rate-limiting or policy enforcement.
* No persistence or cross-process aggregation.

### 4) Public API Surface
* **Re-exports:** (none)
* **Key types / functions / traits:**

  * `CountingStream<S>`: wraps any `S: Read/Write` and increments counters on IO.

    * `CountingStream::new(inner, counters)`
    * `CountingStream::into_inner()`
    * `CountingStream::counters() -> Counters`
    * Implements `Read` and `Write` (increments on successful ops).
  * `Counters(Arc<Mutex<State>>)`:

    * `Counters::new()`
    * `add_in(u64)`, `add_out(u64)`
    * `snapshot() -> Snapshot`
    * `reset_minutes()` (preserves totals, clears ring)
  * `Snapshot`:

    * `total_in`, `total_out`
    * `per_min_in: [u64; 60]`, `per_min_out: [u64; 60]`
* **Events / HTTP / CLI:** None.

### 5) Dependencies & Coupling
* **Internal crates:** none (intentionally standalone). Stability: **tight isolation**, replaceable: **yes**.
* **External crates (top):**

  * `anyhow` (workspace): not required by current code path; low risk, MIT/Apache.
  * `parking_lot` (workspace): declared; current code uses `std::sync::Mutex`; consider switching. Mature, MIT.
  * `workspace-hack`: build graph hygiene only.
* **Runtime services:** Uses OS time (`SystemTime`) to bucket by epoch minute; no network, storage, or crypto.

### 6) Config & Feature Flags
* **Env vars:** none.
* **Config structs:** none.
* **Cargo features:** none.
* Effect: behavior is deterministic apart from system clock.

### 7) Observability
* In-memory counters + `snapshot()`; no direct logging or metrics export.
* No `/healthz`/`/readyz`; not a service.
* Intended for consumers (gateway/services) to scrape/emit.

### 8) Concurrency Model
* Shared state behind `Arc<Mutex<State>>` (single small critical section per update).
* Minute rotation occurs on write/read accounting when the observed minute changes; intermediate buckets zeroed.
* No async tasks/channels; backpressure is the caller’s concern.
* No internal timeouts/retries; purely local mutation.

---

## arti\_transport

- **path:** `crates/arti\_transport` · **role:** lib · **owner:** Stevan White · **maturity:** draft · **last-reviewed:** 2025-09-14

### 1) One-liner
SOCKS5-based outbound (Tor/Arti) plus a tiny Tor control-port client to publish a v3 hidden service and hand connections to the core transport `Handler`.

### 2) Primary Responsibilities
* Dial arbitrary `host:port` through a SOCKS5 proxy (Tor/Arti) and return a `ReadWrite` stream.
* Publish a Tor v3 hidden service (ephemeral by default, persistent if configured) and accept inbound connections, piping each to the provided `Handler`.
* Track I/O with shared byte counters (`accounting::Counters` / `CountingStream`).

### 3) Non-Goals
* No direct async runtime integration (uses blocking `std::net` + one background thread).
* No integrated Prometheus/HTTP metrics export, rate limiting, or access control.
* No Tor process management (expects an already-running Tor/Arti with SOCKS & control ports).

### 4) Public API Surface
* **Re-exports:** none.
* **Key types / functions / traits:**

  * `ArtiTransport`: concrete `Transport` impl.

    * `pub fn new(socks_addr: String, tor_ctrl_addr: String, connect_timeout: Duration) -> Self`
    * `pub fn counters(&self) -> Counters`
  * `impl Transport for ArtiTransport` (from local `transport` crate):

    * `fn connect(&self, addr: &str) -> Result<Box<dyn ReadWrite + Send>>`

      * Dials via `socks::connect_via_socks()`, wraps with `CountingStream`, sets read/write timeouts.
    * `fn listen(&self, handler: Handler) -> Result<()>`

      * Calls `hs::publish_and_serve()` to ADD\_ONION and spawn accept loop; passes each accepted stream to `handler`.
* **Events / HTTP / CLI:** none (library-only).

### 5) Dependencies & Coupling
* **Internal crates:**

  * `transport` → Provides `Transport`, `ReadWrite`, `Handler`. **Tight** (core trait boundary). Replaceable: **no** (by design).
  * `accounting` → Byte counters wrapper. **Loose** (can be replaced with any tallying). Replaceable: **yes**.
* **External crates (top 5):**

  * `socks = "0.3"` → SOCKS5 client used for outbound. Mature, MIT/Apache; moderate maintenance risk (simple, but not high-churn).
  * `anyhow` (workspace) → error plumbing; low risk.
  * `tracing = "0.1"` → logging facade (currently underused in this crate); low risk.
  * (No TLS libs; control port is plain TCP to localhost.)
* **Runtime services:** Requires Tor/Arti SOCKS (e.g., `127.0.0.1:9050`) and ControlPort (e.g., `127.0.0.1:9051`) reachable on localhost. No DB or crypto done locally (keys handled as strings passed to Tor).

### 6) Config & Feature Flags
* **Env vars:**

  * `RO_HS_KEY_FILE` (optional): path to persist the private key for the v3 HS.

    * **Unset** → Ephemeral onion (`Flags=DiscardPK`).
    * **Set & file exists** → Reuse the exact Tor key string (`ED25519-V3:...`).
    * **Set & file missing** → Request NEW key and write it to the path.
* **Constructor args:** `socks_addr`, `tor_ctrl_addr`, `connect_timeout`.
* **Cargo features:** none.

### 7) Observability
* `accounting::Counters` wraps all streams (in/out totals + 60-minute ring).
* Logs: primarily `eprintln!` in accept errors; `tracing` is present but not consistently used here.
* No `/healthz` or readiness endpoints (library).

### 8) Concurrency Model
* **Outbound:** blocking connect via `socks::Socks5Stream`; per-stream socket timeouts set to `connect_timeout`.
* **Inbound HS:** creates a `TcpListener` on `127.0.0.1:0`; publishes ADD\_ONION; spawns **one background thread** that:

  * Loops over `ln.incoming()` and, for each connection, wraps with `CountingStream` and calls `handler(stream)`.
  * Note: calls `handler` **inline** on the accept thread (no per-connection thread/pool). A long-running `handler` will serialize accepts.
* **Ctrl-port:** blocking `TcpStream` client; sends `PROTOCOLINFO`, parses AUTH methods, then `AUTHENTICATE` (SAFECOOKIE or NULL).

---

## common

- **path:** `crates/common` · **role:** lib · **owner:** Stevan White · **maturity:** draft · **last-reviewed:** 2025-09-14

### 1) One-liner
Shared foundation crate: hashing (BLAKE3), address formatting/parsing, `NodeId`, and a small `Config` loader used across services.

### 2) Primary Responsibilities
* Provide stable utilities: BLAKE3 helpers (`b3_hex`, `b3_hex_file`), address helpers (`format_addr`, `parse_addr`, `shard2`), and `NodeId`.
* Define a repo-wide `Config` with defaults and disk loader (TOML/JSON).
* Re-export commonly used helpers so higher layers don’t duplicate logic.

### 3) Non-Goals
* No networking, async runtime, or service hosting.
* No metrics/HTTP endpoints; no persistence beyond reading config files.
* No cryptographic key management beyond hashing helpers.

### 4) Public API Surface
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

### 5) Dependencies & Coupling
* **Internal crates:** none (intentionally base/leaf). Replaceable: **yes**.
* **External crates (top 5; via `Cargo.toml`):**

  * `blake3` — hashing; mature, Apache/MIT; low risk.
  * `serde`, `serde_json`, `toml` — config and DTOs; very stable; low risk.
  * `anyhow` — ergonomic error contexts at API edges; low risk.
  * `hex` — encoding; tiny, stable.
  * `thiserror` — available for typed errors (not heavily used yet).
  * (`workspace-hack` is build-graph hygiene only.)
* **Runtime services:** Filesystem only (config read; optional file hashing). No network or crypto services.

### 6) Config & Feature Flags
* **Env vars:** none.
* **Config structs:** `Config` (see fields above). `Default` uses localhost for overlay/inbox, Tor SOCKS (9050) and control (9051), `chunk_size = 1<<16`, and `connect_timeout_ms = 5000`.
* **Cargo features:** none.
* **Effect:** Deterministic behavior; TOML/JSON auto-detect in `Config::load`.

### 7) Observability
* None built-in (no metrics/log/health). Intended to be imported by services that expose metrics/logging.

### 8) Concurrency Model
* None; purely synchronous helpers and types. No internal threads, channels, or locks.

---

## gateway

- **path:** `crates/gateway` · **role:** lib · **owner:** Stevan White · **maturity:** draft · **last-reviewed:** 2025-09-14

### 1) One-liner
HTTP façade and OAP/1 ingress for RustyOnions: serves `b3:<hex>.<tld>` bundles over Axum with ETag/Range/precompressed negotiation, optional payment enforcement, quotas, and Prometheus metrics; plus a lightweight OAP server for framed ingest/telemetry.

### 2) Primary Responsibilities
* Serve immutable bundle artifacts via `GET/HEAD /o/:addr/*rel` by fetching bytes from `svc-overlay` over UDS (ron-bus envelope).
* Enforce basic edge policies: conditional GET (ETag), byte ranges, precompressed selection (`.br`, `.zst`), per-tenant token-bucket quotas, and optional `402` per `Manifest.toml`.
* Expose health/readiness and `/metrics` (Prometheus); provide an OAP/1 server (`oapd`) that validates BLAKE3 digests and emits kernel events.

### 3) Non-Goals
* No direct storage or on-disk indexing; all object lookup comes via services.
* No TLS termination, auth, or complex access control (beyond quotas/payment guard).
* No write path for bundles (OAP server is protocol echo/validate + eventing, not persistence).

### 4) Public API Surface
* **Re-exports:**

  * `routes::router()` → `axum::Router<()>` (stateless; state injected at serve-time).
  * `state::AppState` (shared clients and flags).
  * `oap::OapServer`.
* **Key types / functions / traits:**

  * `routes::router()`: mounts `/o/:addr/*tail`, `/healthz`, `/readyz`, `/metrics`, and a JSON 404 fallback; applies metrics middleware.
  * `routes::object::serve_object(Extension<AppState>, Path<(addr, rel)>, ...) -> IntoResponse`:

    * ETag (`"b3:<hex>"`) derivation from address, If-None-Match short-circuit (`304`).
    * Accept-Encoding negotiation for `.br`/`.zst` sibling artifacts.
    * Single-range (`bytes=a-b`) parsing and `206` slicing; invalid ranges → `416`.
    * Content-Type guess from extension (html/js/css/wasm/img/…).
    * Optional `402 Payment Required` via `pay_enforce` on `Manifest.toml`.
  * `state::AppState { index: Arc<IndexClient>, overlay: Arc<OverlayClient>, enforce_payments: bool }`.
  * `overlay_client::OverlayClient::from_env_or(...).get_bytes(addr, rel) -> Result<Option<Vec<u8>>>` (UDS, rmp-serde framed).
  * `index_client::IndexClient::resolve_dir(addr) -> Result<PathBuf>` (UDS helper; not in the hot GET path).
  * `quotas::{check(tenant) -> Option<retry_after_secs>}` (global token-bucket).
  * `metrics::{metrics_handler, record_metrics, ...}` (Prometheus registry + middleware).
  * `oap::OapServer::serve(addr) -> (JoinHandle, bound_addr)`; framed OAP/1: `Hello` → `Start{topic}` → many `Data{hdr+body}` (b3 verified) → `End`, with windowed ACKs and concurrency gating.
* **Events / HTTP / CLI:**

  * HTTP: `/o/:addr/*rel`, `/healthz`, `/readyz`, `/metrics`.
  * CLI (bin): `gateway` (bind address, `--enforce-payments`), `gateway-oapd` (OAP server with ack window/max-frame/concurrency).
  * Bus events: emits `KernelEvent::{Health,ConfigUpdated,ServiceCrashed}` from OAP server.

### 5) Dependencies & Coupling
* **Internal crates → why / stability / replaceable?**

  * `oap` (tight): frame codec + helpers (`OapFrame`, `read_frame`, `ack_frame`, `b3_of`). Replaceable: **no** (protocol boundary).
  * `ron-kernel` (loose): event bus types; used for OAP telemetry. Replaceable: **no** (kernel contract).
* **External crates (top 5; pins via workspace):**

  * `axum` (0.7.x via workspace): HTTP server/routing. Risk: low; active.
  * `tokio`: async runtime. Risk: low.
  * `tower` / `tower-http` (trace/compression): middleware. Risk: low.
  * `prometheus`: metrics registry/encoders. Risk: low; text encode stable.
  * `rmp-serde` / `serde`: message framing for UDS calls. Risk: low.
  * Others used: `tracing`, `clap` (bins), `reqwest` in tests.
* **Runtime services:**

  * **Network:** TCP listener for HTTP; optional OAP TCP listener.
  * **Storage:** none (delegated to services).
  * **OS:** UDS clients to `svc-overlay` (`RON_OVERLAY_SOCK`) and `svc-index` (`RON_INDEX_SOCK`).
  * **Crypto:** BLAKE3 verification via `oap` helper (OAP path only).

### 6) Config & Feature Flags
* **Env vars:**

  * `RON_OVERLAY_SOCK` (default `/tmp/ron/svc-overlay.sock`).
  * `RON_INDEX_SOCK` (default `/tmp/ron/svc-index.sock`).
  * `RON_QUOTA_RPS` (float), `RON_QUOTA_BURST` (float) for token-bucket quotas.
* **Config structs:** `AppState` assembled in `main.rs`; router itself is stateless.
* **Cargo features:** `legacy-pay` (wraps older enforcer type; default off).
* **Effect:** Env controls sockets/quotas; CLI controls bind address and payment enforcement.

### 7) Observability
* **Metrics (Prometheus):**

  * `requests_total{code}`, `bytes_out_total`, `request_latency_seconds`,
    `cache_hits_total` (304s), `range_requests_total` (206s),
    `precompressed_served_total{encoding}`, `quota_rejections_total` (429s).
  * `/metrics` handler exports text format; middleware records status/latency/bytes.
* **Health/readiness:** `/healthz` (liveness), `/readyz` (checks UDS reachability to overlay/index with 300ms timeout).
* **Logging:** `tracing` used (e.g., request summaries in object route, server bind info). Some error paths still use best-effort mapping helpers.

### 8) Concurrency Model
* **HTTP path:** Axum/Tokio. Handlers are `async`, but UDS clients use **blocking** `std::os::unix::net::UnixStream` I/O; this can pin a runtime worker thread under load. No per-request concurrency limiter; backpressure is via Tokio + quotas (if enabled).
* **Quotas:** global `OnceLock<Quotas>`; per-tenant token buckets stored behind a `tokio::sync::Mutex<HashMap<...>>`; time arithmetic via `Instant`.
* **OAP server:** One `TcpListener`, per-connection task gated by a `Semaphore` (`concurrency_limit`). In-task loop enforces `max_frame`, ACK windowing, and frame sequencing. Control errors publish `ServiceCrashed`.
* **Retries/Timeouts:** HTTP path defers to service timeouts; readiness uses 300ms connect timeout. OAP path responds protocol errors and closes.

---

## index

- **path:** `crates/index` · **role:** lib · **owner:** Stevan White · **maturity:** draft · **last-reviewed:** 2025-09-14

### 1) One-liner
A tiny library that maintains and queries the mapping from a content address (`b3:<hex>`) to currently known providers, enforcing TTLs and signatures, so callers can resolve where to fetch bytes without speaking the DHT directly. &#x20;

### 2) Primary Responsibilities
* Resolve `b3:<hex>` → provider list (node\_id + addr), honoring TTL and limits.&#x20;
* Accept provider announcements and validate Ed25519 signatures + expirations.&#x20;
* Offer a fast, local cache and persistence backing used by `svc-index` and tools like `tldctl pack`. &#x20;

### 3) Non-Goals
* Running a network-facing API (that’s `svc-index`; this crate is the embeddable core).&#x20;
* Implementing DHT lookups or peer gossip (delegated to `svc-dht`/discovery).&#x20;
* Storing or serving object bytes (that’s Storage/Overlay).&#x20;

### 4) Public API Surface
* **Re-exports:** (none yet; keep minimal)
* **Key types / fns (proposed for clarity):**

  * `Provider { node_id: String, addr: SocketAddr, expires_at: u64, sig: Vec<u8> }` (matches announce schema).&#x20;
  * `Index::announce(hash: &str, provider: Provider) -> Result<()>` (validates TTL/signature).&#x20;
  * `Index::resolve(hash: &str, limit: usize) -> Vec<Provider>` (sorted/stable, cap by `limit`).&#x20;
  * `Index::gc(now_epoch: u64)` (purges expired records).
* **Events / HTTP / CLI:** none in the lib; `svc-index` exposes `GET /resolve` and `POST /announce` using this crate.&#x20;

### 5) Dependencies & Coupling
* **Internal crates:**

  * *ron-kernel* (loose; for health/metrics patterns if embedded) — replaceable: **yes**.&#x20;
  * *svc-index* (service wrapper uses this lib; tight but directional).&#x20;
* **External crates (expected top 5):**

  * `sled` (embedded KV store for provider sets). Risk: moderate (maintenance), permissive license. (Implied by `RON_INDEX_DB` workflows.)&#x20;
  * `blake3` (addressing; verification alignment). Low risk.&#x20;
  * `ed25519-dalek` (signature verification on announces). Moderate risk; widely used.&#x20;
  * `serde` (+`rmp-serde`/`rmpv` for compact storage) — low risk. (Used broadly across services.)&#x20;
  * `parking_lot` (locks) — low risk.
* **Runtime services:** local disk (Sled DB), system clock (TTL), optional bus for health. &#x20;

### 6) Config & Feature Flags
* **Env vars:** `RON_INDEX_DB` → path to the Sled database used by pack/index/services (must match across tools).&#x20;
* **Cargo features (suggested):**

  * `verify-sig` (on by default) toggles Ed25519 checks.&#x20;
  * `inmem` switches to an in-memory map for tests.
* **Constants alignment:** `b3:<hex>` addressing, OAP/1 `max_frame = 1 MiB` (docs alignment; not used directly here). &#x20;

### 7) Observability
* Expected to surface counters via the host service (`svc-index`): `requests_total{code}`, `bytes_{in,out}_total`, `rejected_total{reason}`, `latency_seconds`, `inflight`.&#x20;
* Health/readiness endpoints exist at the service layer; the lib should expose cheap `stats()` for wiring.&#x20;

### 8) Concurrency Model
* Library is synchronous over a `RwLock` around the store (Sled is thread-safe); no background tasks required.
* Backpressure and timeouts live in the service layer; the lib enforces `limit` and TTL checks to keep ops O(log n) per key. (Service backpressure/429 rules per blueprint.)&#x20;

---

## kameo

- **path:** `crates/kameo` · **role:** lib · **owner:** Stevan White · **maturity:** draft · **last-reviewed:** 2025-09-14

### 1) One-liner
A lightweight, in-process actor toolkit (mailboxes, supervisors, and typed request/response) for building RustyOnions services with predictable concurrency, backpressure, and restarts.

### 2) Primary Responsibilities
* Provide a minimal actor runtime: `Actor` trait, `Addr<T>` handles, bounded mailboxes, and a typed `ask` pattern.
* Enforce supervision & backoff policies (crash isolation, restart limits, jitter) with metrics/log signals.
* Offer ergonomic helpers to integrate actors into our kernel (spawn, graceful shutdown, bus/health hooks).

### 3) Non-Goals
* Not a distributed actor system (no remote transport, routing tables, sharding).
* Not a replacement for the kernel bus (kameo is in-proc; bus is cross-service).
* Not a general scheduler/executor (relies on Tokio; no custom runtime).

### 4) Public API Surface
* **Re-exports:** (keep minimal) `tokio::task::JoinHandle`, `tokio::sync::{mpsc, oneshot}` when feature-gated.
* **Key types / functions / traits (expected/stable surface):**

  * `trait Actor`: `type Msg; async fn handle(&mut self, msg: Self::Msg, ctx: &mut Ctx<Self>) -> Result<()>;`
  * `struct Addr<A: Actor>`: `send(msg)`, `try_send(msg)`, `ask<R>(msg) -> Result<R>`, `close()`.
  * `struct Ctx<A: Actor>`: access to timers, spawn\_child, stop, actor name/id, supervised restarts.
  * `struct Supervisor`: strategies (`Always`, `OnPanic`, `Never`), `Backoff{min,max,jitter,reset_after}`.
  * `spawn_actor(actor, MailboxCfg) -> (Addr<A>, JoinHandle<()>)`.
  * `MailboxCfg { capacity, overflow: DropNewest|DropOldest|Reject, warn_hwm }`.
  * `AskCfg { timeout, buffer }` for typed request/response via oneshot.
* **Events / HTTP / CLI:** none; logs/metrics published for the host service to expose.

### 5) Dependencies & Coupling
* **Internal crates:**

  * *ron-kernel* (loose): optional integration to publish `KernelEvent::{ServiceCrashed,Health}` and use `Metrics` counters; replaceable: **yes**.
  * *ron-bus* (loose): not required; bus remains inter-service IPC, while kameo is in-proc.
* **External crates (top 5; likely pins/features):**

  * `tokio` (rt, sync/mpsc, time); low risk, core dependency.
  * `tracing` (spans, errors, actor ids); low risk, widely maintained.
  * `thiserror` or `anyhow` (error taxonomy); low risk.
  * `parking_lot` (fast locks for registry/counters); low risk.
  * `futures`/`async-trait` (if trait methods are async); moderate risk (dyn overhead), standard.
* **Runtime services:** none beyond process resources & clock. No network/crypto/storage in this crate.

### 6) Config & Feature Flags
* **Cargo features:**

  * `metrics` (default): register Prometheus counters/histograms in the hosting service.
  * `tracing` (default): structured spans with actor name/id and message types.
  * `bus-hooks`: emit `KernelEvent` signals via the kernel bus on crash/restart.
  * `loom-tests`: enable loom models in tests only.
* **Env vars:** none directly; consumers decide sampling/verbosity via standard `RUST_LOG` and service config.

### 7) Observability
* **Metrics (by actor label):**

  * `kameo_messages_total{actor,kind=received|handled|rejected}`
  * `kameo_mailbox_depth{actor}` (gauge) and high-watermark.
  * `kameo_handle_latency_seconds{actor}` (histogram).
  * `kameo_restarts_total{actor,reason}`; `kameo_failures_total{actor,kind=panic|error}`.
* **Health/readiness:** expose a cheap `stats()`/`is_idle()` for wiring into `/readyz` at the service layer.
* **Tracing:** span per message handle; include cause chain on failure; supervisor restart spans with backoff.

### 8) Concurrency Model
* **Execution:** one Tokio task per actor; single-threaded `handle` guarantees (no concurrent `handle` on same actor).
* **Mailboxes:** bounded `tokio::mpsc` per actor; overflow policy selected via `MailboxCfg`.
* **Backpressure:** callers use `send` (awaits when full) or `try_send` (fail fast) based on path criticality; `ask` uses bounded in-flight with timeout.
* **Supervision:** parent/child tree; on panic or error return, supervisor applies backoff (exponential + jitter) until `max_restarts` within `window` trips to `Stopped`.
* **Cancellation/Shutdown:** cooperative: `close()` stops intake; drain tail up to budget; `on_stop()` hook for cleanup.

---

## naming

- **path:** `crates/naming` · **role:** lib · **owner:** Stevan White · **maturity:** draft · **last-reviewed:** 2025-09-14

### 1) One-liner
Defines the **canonical content address syntax** for RustyOnions (`b3:<64-hex>.<tld>`), plus helpers to parse/validate/normalize addresses and to read/write **Manifest v2** metadata.

### 2) Primary Responsibilities
* **Addressing:** Parse and format RustyOnions addresses (BLAKE3-256 only), normalize case/prefix, and verify hex length/characters.
* **Typing:** Enumerate and parse **TLD types** (e.g., `image`, `video`, `post`, …) to keep addresses semantically scoped.
* **Manifests:** Provide a stable **Manifest v2** schema and a writer for `Manifest.toml` (core fields, optional encodings/payments/relations/ext).

### 3) Non-Goals
* **No resolution** from address → providers (that’s the index service).
* **No byte serving/storage** (overlay/storage handle bytes).
* **No network/API** surface; this is a pure library with light filesystem I/O for manifest writing only.

### 4) Public API Surface
* **Re-exports:** none currently.
* **Key types / functions / traits:**

  * `hash` module:

    * `const B3_LEN: usize = 32`, `const B3_HEX_LEN: usize = 64`
    * `fn b3(bytes: &[u8]) -> [u8; 32]`
    * `fn b3_hex(bytes: &[u8]) -> String`
    * `fn parse_b3_hex(s: &str) -> Option<[u8; 32]>`
  * `tld` module:

    * `enum TldType { Image, Video, Audio, Post, Comment, News, Journalist, Blog, Map, Route, Passport, ... }`
    * `impl FromStr` and `Display` with lower-case mapping; `TldParseError`
  * **Address** (duplicated—see “Improvement Opportunities”):

    * In `address.rs`: `struct Address { hex: String, tld: TldType }` with `impl FromStr` expecting `<hex>.<tld>` and `AddressParseError`.
    * In `lib.rs`: a second `struct Address { hex: String, tld: String }` with comments allowing optional `b3:` and canonical `Display` as `b3:<hex>.<tld>`.
  * `manifest` module:

    * `struct ManifestV2 { schema_version, tld, address, hash_algo, hash_hex, bytes, created_utc, mime, stored_filename, original_filename, encodings[], payment?, relations?, license?, ext{} }`
    * `struct Encoding { coding, level, bytes, filename, hash_hex }`
    * `struct Payment { required, currency, price_model, price, wallet, ... }`
    * `struct Relations { reply_to?, thread?, source?, ... }`
    * `fn write_manifest(dir: &Path, &ManifestV2) -> Result<PathBuf>`
* **Events / HTTP / CLI:** none.

### 5) Dependencies & Coupling
* **Internal crates:** none required at compile time (good isolation). Intended consumers: `gateway`, `svc-index`, `svc-overlay` (loose coupling).
* **External crates (top 5 and why):**

  * `blake3` — content addressing; **low risk**, active.
  * `serde`, `thiserror`, `anyhow` — data model & errors; **low risk**, standard.
  * `toml` — manifest encoding; **low risk**.
  * `mime` / `mime_guess` — present in `Cargo.toml`, likely for future MIME derivation; **currently unused** (see debt).
  * `uuid`, `chrono`, `base64` — also declared; **currently unused** in visible code (drift).
* **Runtime services:** none (pure CPU & small FS writes for manifests). No network/crypto services beyond BLAKE3 hashing.

### 6) Config & Feature Flags
* **Env vars:** none.
* **Cargo features:** none declared. (Opportunity: feature-gate optional `mime_guess`, `uuid`, etc., or remove until used.)

### 7) Observability
* No metrics/logging inside the crate (appropriate for a pure library). Downstream services should tag failures when parsing/validating addresses or manifests.

### 8) Concurrency Model
* Not concurrent; pure functions and data types. No async, no background tasks. Thread-safe by construction (no globals).

---

## node

- **path:** `crates/node` · **role:** lib (primarily a CLI binary with a tiny placeholder lib) · **owner:** Stevan White · **maturity:** draft · **last-reviewed:** 2025-09-14

### 1) One-liner
A developer-facing CLI that runs a single-process RustyOnions **overlay listener** and offers **PUT/GET** helpers (TCP and optional Tor), plus a tiny JSON stats socket for quick local testing.

### 2) Primary Responsibilities
* **Serve**: start an overlay listener bound to a local TCP address (or Tor HS when enabled) using the embedded store path.
* **Client ops**: PUT a local file and GET a blob by `b3:<hex>` from a listener for demos/smoke tests.
* **Dev stats**: expose minimal, ad-hoc JSON counters (bytes in/out, store size) for local inspection.

### 3) Non-Goals
* Not a microkernel-managed “service”; no supervision tree or bus integration.
* Not the production gateway (no HTTP API surface, no auth, no multi-tenant features).
* Not a persistence authority beyond the overlay’s own sled store; no index, no naming resolution.

### 4) Public API Surface
* **Re-exports**: none (the lib target is intentionally empty/placeholder).
* **Key functions (binary/CLI)**

  * `Serve { bind, transport, store_db }` → runs overlay listener (`TCP` today; legacy Tor path in old commands).
  * `Put { to, path }` → connect and upload file; prints content hash.
  * `Get { from, hash, out }` → fetch by hex hash; write to file.
* **Legacy (unwired code under `src/commands/*`)**

  * `serve(config, transport = "tcp"|"tor")` using `TcpTransport` or `ArtiTransport`, spawns a tiny metrics socket on `dev_inbox_addr`.
  * `put/get(..., transport)`, `tor_dial`, `init` (writes a default `config.toml`), `stats_json`.
* **Events / HTTP / CLI**: CLI only (clap). The dev stats socket speaks a minimal HTTP/1.1 response with a JSON body; there’s **no** Prometheus endpoint here.

### 5) Dependencies & Coupling
* **Internal crates**

  * `overlay` (tight, replaceable: **yes**): provides `run_overlay_listener`, `client_put/get`, `Store` (sled) and transport-agnostic variants in legacy path.
  * `common` (loose, replaceable: **yes**): reads `Config` (used by legacy commands).
  * `transport` (loose, replaceable: **yes**): TCP transport; Tor control helpers in legacy path.
* **External crates (top)**

  * `tokio` (multithread + macros + net + signal) — runtime for Ctrl-C and async clients.
  * `clap` — CLI parser (derive).
  * `tracing`, `tracing-subscriber` — logs and `RUST_LOG` env filter.
  * `color-eyre`/`anyhow` — human-friendly error context.
  * *(Legacy code)* `arti_transport` — Tor SOCKS + HS publish; currently **not in Cargo.toml**, indicating drift.
* **Runtime services**:

  * **Network**: TCP listener on `bind`; optional Tor SOCKS + control when using the legacy path.
  * **Storage**: sled DB directory (overlay store).
  * **OS**: signals (Ctrl-C), threads for periodic counter logs in legacy serve.

### 6) Config & Feature Flags
* **CLI flags (current `main.rs`)**: `--bind`, `--transport` (accepts only `tcp`), `--store-db`.
* **Legacy config (via `common::Config`)**:

  * `data_dir`, `overlay_addr`, `dev_inbox_addr`, `socks5_addr`, `tor_ctrl_addr`, `chunk_size`, `connect_timeout_ms`, optional `hs_key_file` (via `RO_HS_KEY_FILE` env).
* **Cargo features**: none today (opportunity: `tor`, `metrics`, `legacy-commands`).
* **Env**: `RUST_LOG` for logging; `RO_HS_KEY_FILE` honored by legacy Tor serve.

### 7) Observability
* **Current**: `tracing` logs; if using legacy `serve`, a tiny TCP socket on `dev_inbox_addr` returns JSON: `{ store: {n_keys,total_bytes}, transport: {total_in,total_out} }`.
* **Gaps**: no Prometheus metrics, no standardized `/healthz`/`/readyz` hooks, and the JSON stats endpoint is non-standard and ad-hoc.

### 8) Concurrency Model
* **Serve path (current)**: overlay listener runs in its own async stack (inside `overlay`); the CLI awaits Ctrl-C via `tokio::signal::ctrl_c()`.
* **Legacy path**: spawns a thread every 60s to log counters; simple accept loop for the stats socket (blocking `TcpListener`).
* **Backpressure**: bounded by overlay/transport internals; CLI PUT/GET calls are synchronous from the user’s perspective.
* **Timeouts/Retries**: client ops rely on overlay/transport defaults; legacy Tor path sets connect timeout via config.

---

## oap

- **path:** `crates/oap` · **role:** lib · **owner:** Stevan White · **maturity:** draft · **last-reviewed:** 2025-09-14

### 1) One-liner
A tiny, dependency-light codec + helpers for **OAP/1** that parse/serialize frames, enforce bounds, and provide canonical DATA packing (with `obj:"b3:<hex>"`) used by the SDK, gateway, and services. &#x20;

### 2) Primary Responsibilities
* Implement the **OAP/1** wire format and state machine with strict bounds and errors, plus HELLO negotiation helpers. &#x20;
* Provide canonical **DATA packing** helpers that embed `obj:"b3:<hex>"` in a framed header; both sides use the same logic to prevent drift. &#x20;
* Ship normative **test vectors** and parser tests/fuzz to guarantee interop (SDK parity, conformance suite). &#x20;

### 3) Non-Goals
* Not a transport or TLS layer (uses kernel/transport or SDK for I/O); no business logic, economics, or service-level quotas here.&#x20;
* Not a capability system or verifier (macaroons are referenced/encoded, but verification/enforcement lives at services/gateway).&#x20;

### 4) Public API Surface
* **Re-exports:** OAP constants (version, limits), status codes, flags; canonical test vectors (A–T).&#x20;
* **Key types / functions / traits:**

  * `OapFrame { len, ver, flags, code, app_id, tenant, caplen, corr_id, cap, payload }` with `encode/decode`.&#x20;
  * `Flags` bitset (e.g., `REQ`, `RESP`, `START`, `END`, `ACK_REQ`, `APP_E2E`, `COMP`).&#x20;
  * `HelloInfo { max_frame, max_inflight, supported_flags, version }` and `hello()` probe.&#x20;
  * `data_frame()` + `encode_data_payload` / `decode_data_payload` placing `obj:"b3:<hex>"` into the header. &#x20;
  * `Error` enum with typed reasons (bad\_frame, oversize, unsupported\_flag, decompress\_too\_large, unauthorized, quota, timeout). &#x20;
* **Events / HTTP / CLI:** none directly; consumed by gateway/services; vectors runnable via `ronctl test oap --vectors`.&#x20;

### 5) Dependencies & Coupling
* **Internal crates:**

  * *ron-kernel*: none at compile-time (keep codec independent); used by gateway that consumes oap. Tight runtime coupling avoided by design. Replaceable: **yes** (codec could be swapped with a generated one).&#x20;
  * *gateway/sdk*: depend **on** `oap`, not vice-versa (to avoid layering inversions).&#x20;
* **External crates (expected top 5; minimal pins/features):**

  * `bytes` (frame I/O without copies), `serde`/`serde_json` (HELLO/DATA header JSON), optional `zstd` (COMP), optional `tokio` (demo I/O), `uuid` (tenant). Risks: low/maintained; zstd guarded.&#x20;
* **Runtime services:** none (pure codec). TLS and Tor belong to transport/gateway; ALPN/TLS posture is normative input.&#x20;

### 6) Config & Feature Flags
* **Env/config:** n/a for the core crate (limits negotiated via HELLO). `max_frame` defaults to **1 MiB** unless HELLO lowers it.&#x20;
* **Cargo features:** `comp` (zstd compression; enforce 8× bound), `pq-preview` (future macaroons PQ verifier compatibility, no wire change), `tcp-demo` (tokio helpers). &#x20;

### 7) Observability
* The crate itself is logic-only; metrics are emitted at gateway/services. Golden metrics include `requests_total`, `bytes_{in,out}_total`, `rejected_total{reason}`, `latency_seconds`, `inflight`, `quota_exhaustions_total`.&#x20;
* SDK/gateway should expose HELLO timing and parse errors; kernel emits health/service events.&#x20;

### 8) Concurrency Model
* Pure functions for encode/decode; no interior mutability.
* For I/O demos, **ACK\_REQ** + server window with `max_inflight` backpressure; timeouts/retries are caller policy (SDK/gateway). &#x20;

---

## overlay

- **path:** `crates/overlay` · **role:** lib · **owner:** Stevan White · **maturity:** draft · **last-reviewed:** 2025-09-14

### 1) One-liner
A legacy library that bundles a simple TCP overlay protocol (PUT/GET), a sled-backed blob store, and convenience client/server helpers into one crate.

### 2) Primary Responsibilities
* Implement a minimal overlay wire protocol (PUT → hash; GET ← blob) over TCP.
* Persist and retrieve binary blobs in a local sled database keyed by BLAKE3 hex.
* Provide helpers to run a listener and to act as a simple client (PUT/GET).

### 3) Non-Goals
* No HTTP surface, no OAP/ALPN/TLS, and no capability/tenant enforcement.
* Not a scalable content distribution service (no chunking, no replication).
* Not a metrics/quotas layer; only basic tracing logs.

### 4) Public API Surface
* Re-exports: `pub use store::Store;`
* Key types / functions / traits:

  * `Store` (sled wrapper): `Store::open<P: AsRef<Path>>(path) -> Result<Self>`, `put(&self, key: &[u8], val: Vec<u8>)`, `get(&self, key: &[u8]) -> Result<Option<Vec<u8>>>`
  * Protocol (TCP helpers):

    * `run_overlay_listener(bind: SocketAddr, store_db: impl AsRef<Path>) -> anyhow::Result<()>` — spawns a background TCP server task.
    * `client_put(addr: &str, path: &Path) -> anyhow::Result<String>` — PUT file, returns BLAKE3 hex.
    * `client_get(addr: &str, hash_hex: &str, out: &Path) -> anyhow::Result<()>` — GET by hash to file.
  * Internal error enum exists (`OverlayError`) but the crate’s public functions mostly return `anyhow::Result`, so the typed error is not propagated publicly.
* Events / HTTP / CLI: none (pure library; logging via `tracing`).

### 5) Dependencies & Coupling
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

### 6) Config & Feature Flags
* Env vars: none.
* Config structs: none; the caller supplies `SocketAddr` and DB path.
* Cargo features: none (no conditional compilation for TLS/OAP/chunking).

### 7) Observability
* Logs: `tracing::info!` on bind and `tracing::warn!` when listener task exits with error.
* No metrics, health/readiness, or structured error counters.
* No request IDs/spans; correlation relies on peer address and op type.

### 8) Concurrency Model
* Server: `run_overlay_listener` spawns a background task; inside it, a `TcpListener` accepts connections; each connection is handled on its own `tokio::spawn`.
* Backpressure: none at protocol level; server reads entire request payload into memory based on a declared 8-byte length prefix.
* Locks/timeouts/retries: none explicit; sled provides internal concurrency; network ops rely on default socket timeouts (i.e., effectively unbounded).
* Failure model: background task’s `JoinHandle` is dropped; errors are only logged (no restart/supervision here).

---

## ron-app-sdk

- **path:** `crates/ron-app-sdk` · **role:** sdk · **owner:** Stevan White · **maturity:** draft · **last-reviewed:** 2025-09-14

### 1) One-liner
A lightweight, async Rust client SDK for apps to speak **OAP/1** to a RustyOnions node/gateway—handling TLS, HELLO negotiation, capability tokens, and streaming PUT/GET of content-addressed blobs.

### 2) Primary Responsibilities
* Provide a **safe, ergonomic client** to perform OAP requests (e.g., `put`, `get`, `hello`) with connection lifecycle, retries, and backpressure.
* Implement **OAP/1 framing** via the `oap` crate, including canonical DATA header packing (`obj: "b3:<hex>"`), capabilities, and negotiated limits.
* Offer **streaming I/O** APIs (file/reader/writer) so large payloads do not load fully into memory; compute BLAKE3 on the fly for PUT.

### 3) Non-Goals
* Not a server, proxy, or transport daemon; it’s a **client library** only.
* No business policy enforcement (quotas, authz rules); it **carries** capability credentials but does not adjudicate them.
* No persistent cache by default (optional, pluggable is fine but out of core scope).

### 4) Public API Surface
*(naming may differ slightly; organized by intent rather than exact symbol names)*

* **Re-exports:**

  * From `oap`: frame constants/flags, error types used in results (so apps don’t import `oap` directly).
  * From hashing: `blake3::Hash` (or a thin newtype like `ObjId`).

* **Key types / functions / traits:**

  * `Client` / `RonClient`

    * `new(config: Config) -> Result<Self>`
    * `hello(&self) -> Result<HelloInfo>` (populates negotiated `max_frame`, flags).
    * `put(&self, reader: impl AsyncRead, len: u64) -> Result<ObjId>` (streams, hashes, optional compression).
    * `put_file<P: AsRef<Path>>(&self, path: P) -> Result<ObjId>` convenience.
    * `get(&self, id: &ObjId) -> Result<Bytes>` (bounded by policy; small objects).
    * `get_to_writer(&self, id: &ObjId, w: impl AsyncWrite) -> Result<()>` (streaming for large objects).
  * `Config` (builder): endpoint(s), TLS mode/roots, timeouts, in-flight limit, `tenant`, `app_id`, `capability` (macaroon or token), `enable_comp`.
  * `ObjId` (newtype around BLAKE3 hex).
  * `SdkError` (rich error enum with `RetryAdvice`, see §10).
  * `SdkMetrics` (optional feature) or trait-based hooks for instrumentation.

* **Events / HTTP / CLI:**

  * None directly (library), but **feature-gated** dev helpers like `sdk::bin::put` for local testing are fine.

### 5) Dependencies & Coupling
* **Internal crates**

  * `oap` (tight by design): single source of truth for framing/limits. **Replaceable:** yes in principle, but it’s the protocol—so keep tight.
  * `ron-kernel` **not** a dependency (avoid layering inversion).
  * `gateway` / `svc-*` consume the SDK indirectly (at app edges), not vice-versa.

* **External crates (top set; keep minimal)**

  * `tokio` (net + io-util) — async runtime.
  * `tokio-rustls` + `rustls-pemfile` (TLS 1.3, ALPN `ron/1`).
  * `bytes` (zero-copy buffers).
  * `blake3` (content IDs).
  * `serde`/`serde_json` (HELLO payload and optional DATA header JSON).
  * Optional: `zstd` (COMP), `webpki-roots`/`rustls-native-certs` (roots), `thiserror` (errors).
    *Risks:* keep TLS stack versions aligned with workspace pins; `zstd` behind a feature to keep the default build small.

* **Runtime services**

  * **Network:** TCP/TLS client sockets (and optionally SOCKS5 if Tor is used—feature-gated).
  * **Storage/OS/Crypto:** none persistent; uses OS to open files for convenience helpers and `blake3` for hashing.

### 6) Config & Feature Flags
* **Env vars:** none required; allow reading TLS roots or endpoint from env in helpers/tests (`RON_ENDPOINT`, `RON_TLS_INSECURE=1` for dev only).
* **Config struct:**

  * `endpoint: Endpoint` (`host:port`), `alpn: "ron/1"`, `tls: TlsMode` (`SystemRoots | WebPki | InsecureDev`), `tenant`, `app_id`, `capability: Option<Vec<u8>>`,
  * networking knobs: `connect_timeout`, `read_timeout`, `write_timeout`, `hello_timeout`, `inflight_limit`, `retry: RetryPolicy { max_retries, base_backoff, max_backoff }`,
  * `max_frame_hint` (upper bound to clamp local allocations until HELLO arrives).
* **Cargo features:**

  * `comp` (zstd COMP frames; enforce ≤8× decompressed bound),
  * `native-roots` / `webpki-roots`,
  * `tor-socks` (dial via SOCKS5),
  * `metrics` (expose counters/histograms via callbacks),
  * `tracing-logs` (lightweight spans/logs).

### 7) Observability
* **Client-side golden metrics (emit through trait/callback if `metrics` enabled):**

  * `requests_total{op,code}`; `bytes_{in,out}_total{op}`; `latency_seconds{op}`; `inflight{op}`; `retries_total{op,reason}`; `rejected_total{reason}` (oversize, auth, quota).
* **Logging:** `tracing` spans per request with correlation id (OAP corr\_id) and app/tenant tags (redact secrets).
* **Health:** `hello()` result exposed so apps can log negotiated limits and supported flags at startup.

### 8) Concurrency Model
* **Connection management:** a small **async pool** (N connections) honoring `inflight_limit` per connection (via OAP flags/ACK if used) and global **semaphore** for total in-flight.
* **Backpressure:** acquire permit → encode → send → await response or ACK; if pool exhausted, new requests **wait** (bounded queue) or fail fast based on `RetryPolicy`.
* **Timeouts & retries:** per-stage timeouts (connect/hello/read/write). Retries only for **idempotent** ops (GET, HELLO) and only on transient errors (timeout, connection lost before write finished). PUT retries require idempotence with content-hash verification (safe if server side is “put-if-absent by obj id”).
* **Locks:** avoid long-held mutexes; use `parking_lot` or `tokio::sync::Mutex` around small state; channel fan-out for response routing keyed by `corr_id`.

---

## ron-audit

- **path:** `crates/ron-audit` · **role:** lib · **owner:** Stevan White · **maturity:** draft · **last-reviewed:** 2025-09-14

### 1) One-liner
Tamper-evident audit logging: append signed, hash-chained records (Ed25519 + BLAKE3) with an optional filesystem sink.

### 2) Primary Responsibilities
* Build an **append-only chain** of audit records: `prev_hash → body → hash`, then **sign** the new `hash` with Ed25519.
* Provide a minimal API to **append** semantic events with a timestamp and small JSON payload, and (optionally) **persist** each record to disk.

### 3) Non-Goals
* Not a general logging/metrics framework; no query/index layer.
* No key storage/rotation policy (belongs in `ron-kms`/ops).
* No transport/remote sinks in-core (HTTP/Kafka/S3, etc. should be separate).
* No end-user PII redaction or schema validation (callers must filter/sanitize).

### 4) Public API Surface
* Re-exports: none.
* Key types / functions:

  * `struct Auditor` (in-memory signer + chain head):

    * `fn new() -> Self` — generates a fresh Ed25519 key; `prev_hash = [0;32]`.
    * `fn with_dir(self, dir: impl Into<PathBuf>) -> Self` *(feature = "fs")* — enable per-record persistence to `dir`.
    * `fn verifying_key(&self) -> &VerifyingKey`.
    * `fn append(&mut self, kind: &'static str, data: serde_json::Value) -> Result<AuditRecord>`.
  * `struct AuditBody { ts: i64, kind: &'static str, data: serde_json::Value }`.
  * `struct AuditRecord { prev_hash: [u8;32], hash: [u8;32], sig: Vec<u8>, body: AuditBody }`.
* Events / HTTP / CLI: none in this crate.

### 5) Dependencies & Coupling
* Internal crates → none at runtime; **loose** coupling to the wider system (any service can use it). Replaceable: **yes** (API is small).
* External crates (top):

  * `ed25519-dalek` — signatures; mature; constant-time ops; keep pin current.
  * `blake3` — fast hash for chaining; mature.
  * `serde`/`rmp-serde` — deterministic encoding for chaining; low risk.
  * `time` — UTC timestamps.
  * `anyhow` — error aggregation (could be replaced with a custom error).
  * `serde_json` — event payload type.
* Runtime services: OS filesystem *(only with `fs` feature)*; otherwise memory-only.

### 6) Config & Feature Flags
* Features:

  * `fs` (off by default): enable persistence (`with_dir`, per-record `.bin` files); writes `rmp-serde` bytes.
* Env/config structs: none here (callers choose directory path when enabling `fs`).

### 7) Observability
* The crate itself does not emit metrics/logs.
* Recommended: callers increment counters like `audit_append_total{kind}` / `audit_write_fail_total` and export `audit_chain_head` (short b3 hash) as an info metric.

### 8) Concurrency Model
* `append(&mut self, …)` mutates internal `prev_hash`; API is **single-writer** by design.
* For multi-threaded apps, wrap `Auditor` in `Arc<Mutex<…>>` or provide a dedicated single-threaded task receiving messages via mpsc.
* No built-in backpressure or batching; each append is immediate.

---

## ron-auth

- **path:** `crates/ron-auth` · **role:** lib · **owner:** Stevan White · **maturity:** draft · **last-reviewed:** 2025-09-14

### 1) One-liner
Zero-trust message envelopes for internal IPC: sign and verify structured headers/payloads using HMAC-SHA256 with scopes, nonces, and time windows, backed by pluggable key derivation.

### 2) Primary Responsibilities
* Provide a **self-authenticating envelope** type that canonically encodes header+payload and authenticates them with an HMAC tag.
* **Verify** envelopes at every receiving boundary (time window + origin + scopes + tag), and **sign** envelopes at send time using a KMS-backed key derivation.
* Define the **`KeyDeriver`** trait to plug in key management/rotation (via `ron-kms` or other backends).

### 3) Non-Goals
* No key storage, rotation scheduling, or secret management (that lives in **`ron-kms`**).
* No capability/macaroon logic or policy evaluation (lives in **`ron-policy`/service layer**).
* No transport/TLS, no network or DB I/O, no replay database (receivers enforce replay windows if needed).
* Not a JWT/OIDC library; this is **intra-cluster IPC** authn/z, not end-user auth.

### 4) Public API Surface
* Re-exports: none.
* Key types / functions / traits:

  * `enum Plane { Node, App }` — plane tagging for routing and policy.
  * `struct Envelope<H, P>` — generic over header/payload (`H: Serialize, P: Serialize`), fields:

    * `plane`, `origin_svc: &'static str`, `origin_instance: Uuid`, `tenant_id: Option<String>`,
      `scopes: SmallVec<[String; 4]>`, `nonce: [u8;16]`, `iat: i64`, `exp: i64`, `header: H`, `payload: P`, `tag: [u8;32]`.
  * `trait KeyDeriver { fn derive_origin_key(&self, svc: &str, instance: &Uuid, epoch: u64) -> [u8;32]; }`
  * `fn sign_envelope(kd: &dyn KeyDeriver, svc: &str, instance: &Uuid, epoch: u64, env: Envelope<H,P>) -> Envelope<H,P>`
  * `fn verify_envelope(kd: &dyn KeyDeriver, expected_svc: &str, epoch: u64, required_scopes: &[&str], env: &Envelope<H,P>) -> Result<(), VerifyError>`
  * `fn verify_envelope_from_any(kd: &dyn KeyDeriver, allowed_senders: &[&str], epoch: u64, required_scopes: &[&str], env: &Envelope<H,P>) -> Result<(), VerifyError>`
  * `fn generate_nonce() -> [u8;16]`
  * `enum VerifyError { Expired, MissingScope(String), WrongOrigin, BadTag, Crypto }`
* Events / HTTP / CLI: none.

### 5) Dependencies & Coupling
* Internal crates → why / stability / replaceable?

  * **`ron-kms` (indirect)**: typical implementer of `KeyDeriver` (derive per-origin keys; sealing & rotation policy). Stability **loose** (trait-level). Replaceable **yes** (any KMS can implement).
  * **`ron-proto` (recommended, not required)**: carries DTOs used as `header`/`payload`; coupling is **loose** (generic `Serialize`). Replaceable **yes**.
* External crates (top 5) → why / risk:

  * `serde`, `rmp-serde` — canonical messagepack encoding for MAC input; mature, low risk.
  * `hmac`, `sha2` — HMAC-SHA256; mature, constant-time verify.
  * `time` — iat/exp handling; mature.
  * `uuid` — per-instance identity; mature.
  * `smallvec` — efficient `scopes`; low risk.
  * (also `rand` for nonce; `thiserror` for error types).
* Runtime services: none (no network/storage/OS calls; pure CPU + time source).

### 6) Config & Feature Flags
* Env vars / config structs: none in this crate.
* Cargo features: none today; future candidates:

  * `pq` (switch/mix to SHA3/KMAC or PQ signatures if design evolves),
  * `replay-cache` (optional in-mem Bloom/ring buffer helpers, though better as a separate crate).

### 7) Observability
* The crate itself logs nothing (pure functions).
* **Recommended in receivers:** increment `auth_success_total{svc}` / `auth_fail_total{svc,reason}` and log minimal context (never secrets). Expose latency buckets for verification if it becomes hot.

### 8) Concurrency Model
* None — all APIs are synchronous and thread-safe (stateless functions over inputs).
* `KeyDeriver` is `Send + Sync` and may perform internal synchronization/IO in the implementer (but recommended to be in-memory and cheap).

---

## ron-billing

- **path:** `crates/ron-billing` · **role:** lib · **owner:** Stevan White · **maturity:** draft · **last-reviewed:** 2025-09-14

### 1) One-liner
Lightweight billing primitives: compute request cost from a manifest `Payment` policy and validate basic billing fields (model, wallet, price) without performing settlement.

### 2) Primary Responsibilities
* Provide a strict but minimal **pricing model** (`PriceModel`) and **cost calculator** (`compute_cost`) over byte counts and manifest `Payment` policy.
* Validate **payment blocks** for sanity (`validate_payment_block`) and **wallet strings** for presence/format baseline.

### 3) Non-Goals
* No currency FX, tax/VAT, or monetary rounding strategy beyond raw `f64` math.
* No settlement, invoicing, balances, receipts, or billing storage.
* No network lookups (LNURL resolution, chain RPC), KYC, or fraud detection.
* No quota/rate limiting (that’s `ron-policy`), and no business rules for revenue splits beyond what the manifest already declares.

### 4) Public API Surface
* Re-exports: none (crate stands alone; `ryker` may re-export during migration).
* Key types / functions / traits:

  * `enum PriceModel { PerMiB, Flat, PerRequest }` with `PriceModel::parse(&str) -> Option<Self>`.
  * `fn compute_cost(n_bytes: u64, policy: &naming::manifest::Payment) -> Option<f64>`

    * Returns `None` when `policy.required == false`; else cost in `policy.currency`.
  * `fn validate_wallet_string(&str) -> anyhow::Result<()>` (presence/shape checks only).
  * `fn validate_payment_block(&Payment) -> anyhow::Result<()>` (model parseable, non-negative price, non-empty wallet).
* Events / HTTP / CLI: none.

### 5) Dependencies & Coupling
* Internal crates:

  * `naming` (for `manifest::Payment`): **loose/medium** coupling; used for type shape. Replaceable: **yes** (if `Payment` moves to `ron-proto`, switch imports).
* External crates (top):

  * `anyhow` (error bubble-up) — permissive, low risk; consider custom error to reduce footprint.
* Runtime services: none. No network, storage, OS, or crypto at runtime.

### 6) Config & Feature Flags
* Env vars / config structs: none.
* Cargo features: none (current); could add features later for wallet scheme validators (e.g., `lnurl`, `btc`, `eth`) without pulling heavy deps by default.

### 7) Observability
* No metrics or logs are emitted by this crate (pure functions).
* Guidance: callers (e.g., `svc-omnigate`) should instrument cost paths (e.g., `billing_cost_total`, `billing_cost_bytes_total`) if needed.

### 8) Concurrency Model
* None. Pure, synchronous functions; no tasks, channels, or retries.

---

## ron-bus

- **path:** `crates/ron-bus` · **role:** lib · **owner:** Stevan White · **maturity:** draft · **last-reviewed:** 2025-09-14

### 1) One-liner
Tiny IPC layer for RustyOnions services: MessagePack-framed request/response envelopes over Unix Domain Sockets with minimal helpers for `listen / recv / send`.

### 2) Primary Responsibilities
* Define the **wire envelope** (`Envelope`) and a few shared reply shapes (e.g., `Ok`, `Bytes`, `NotFound`, `Err`).
* Provide **UDS framing** (length-prefixed MsgPack) and **blocking** helpers: `listen(sock)`, `recv(&mut UnixStream)`, `send(&mut UnixStream, &Envelope)`.
* (Today) Centralize **service RPC enums** (e.g., `StorageReq/Resp`, `IndexReq/Resp`, `OverlayReq/Resp`) so services share one protocol surface.

### 3) Non-Goals
* No async runtime integration (helpers are **blocking** on std UDS).
* No transport security (UDS only; **no TLS**; no network sockets).
* No authorization/verification of capability tokens (only carries `token: Vec<u8>`; verification is a service concern).
* Not the in-process **kernel bus** (`ron-kernel::Bus`); this is **inter-process** IPC.

### 4) Public API Surface
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

### 5) Dependencies & Coupling
* **Internal crates:** none (good); however, this crate currently **hosts other services’ RPC enums**, which *functionally* couples `svc-index`, `svc-overlay`, `svc-storage`, `gateway`, etc., to this crate’s releases.

  * Stability: **tight** (any RPC enum change fans out here). Replaceable? **Yes**, with a `ron-proto` split (see §13).
* **External (top 5):**

  * `serde` (workspace) — ubiquitous encoding; low risk.
  * `rmp-serde` (workspace) — MessagePack codec; stable; low risk.
  * `thiserror` (workspace) — error ergonomics; low risk.
  * `workspace-hack` — dep dedupe shim; no runtime impact.
* **Runtime services:** **OS UDS** only; no network, no TLS, no crypto primitives.

### 6) Config & Feature Flags
* No cargo features today.
* No config structs; callers pass socket paths. Services define their own defaults (e.g., `/tmp/ron/svc-storage.sock`).

### 7) Observability
* None built-in. No metrics, tracing, or structured error mapping at the IPC layer.
* Errors bubble as `io::Error` or MsgPack decode errors from `rmp-serde`.

### 8) Concurrency Model
* **Blocking I/O** on `std::os::unix::net::UnixStream`.
* Framing: **u32 length prefix** + MsgPack body; `recv` uses a manual `read_exact` loop; `send` writes a length then bytes.
* **Backpressure:** entirely at the OS socket buffer and the caller’s accept loop; **no internal queues**, limits, or timeouts here.
* **Locks/timeouts/retries:** none implemented; callers must add deadlines and retry policies.

---

## ron-kernel

- **path:** `crates/ron-kernel` · **role:** core · **owner:** Stevan White · **maturity:** draft · **last-reviewed:** 2025-09-14

### 1) One-liner
A tiny, stable microkernel that provides the project’s event bus, metrics/health surfaces, transport glue, and shutdown/supervision hooks—while exporting a frozen public API for the rest of the system.&#x20;

### 2) Primary Responsibilities
* Define and **freeze** the kernel’s public API (Bus, events, metrics/health, config, shutdown helper).&#x20;
* Offer **lossy broadcast** bus primitives and light supervision/backpressure behaviors that keep core paths non-blocking.&#x20;
* Expose **observability** endpoints (`/metrics`, `/healthz`, `/readyz`) that other services rely on for SLOs and runbooks.&#x20;

### 3) Non-Goals
* No app/business semantics (Mailbox, quotas, SDK ergonomics, payment). Those live outside the kernel.&#x20;
* No protocol spec surface (OAP/1 codec lives in its own crate; kernel only exports stable re-exports & helpers).&#x20;
* No persistent storage logic (DHT/Index/Storage are services; kernel stays stateless beyond in-memory counters).&#x20;

### 4) Public API Surface
* **Re-exports (frozen):** `Bus`, `KernelEvent::{Health {service, ok}, ConfigUpdated {version}, ServiceCrashed {service, reason}, Shutdown}`, `Metrics`, `HealthState`, `Config`, `wait_for_ctrl_c()`.&#x20;
* **Key types / functions / traits:**

  * `Bus`: broadcast-based event channel; `new(capacity)`, `publish`, `subscribe` (+ helpers like `recv_with_timeout`, `recv_matching`).&#x20;
  * `KernelEvent`: canonical system events including structured crash reasons.&#x20;
  * `Metrics`: Prometheus counters/histograms + HTTP service for `/metrics`, `/healthz`, `/readyz`.&#x20;
  * `HealthState`: shared registry behind `/healthz`/`/readyz`.&#x20;
  * `Config` + `wait_for_ctrl_c()`: configuration holder and graceful shutdown helper (public re-export).&#x20;
  * **Transport hook (exposed at kernel root):** uses `tokio_rustls::rustls::ServerConfig`—type choice is part of the contract.&#x20;
* **Events / HTTP / CLI:** kernel itself exposes HTTP for `/metrics`, `/healthz`, `/readyz`; events published on `Bus` include `Health`, `ConfigUpdated`, `ServiceCrashed{reason}`, `Shutdown`.&#x20;

### 5) Dependencies & Coupling
* **Internal crates:** None required at runtime; kernel is intentionally standalone to avoid coupling creep. It coordinates with sibling crates (oap, gateway, sdk, ledger) via the frozen API.&#x20;
* **External crates (top focus):**

  * **tokio** (async runtime, channels) — core concurrency; widely maintained.
  * **axum 0.7** (HTTP for metrics/health) — aligned with workspace stack; handlers `.into_response()` pattern is enforced across services.&#x20;
  * **prometheus** (metrics registry/types) — counters/histograms exposed at `/metrics`.&#x20;
  * **tokio-rustls 0.26** (TLS types) — **contractually** the kernel’s TLS type for transports.&#x20;
  * **serde** (DTOs for JSON health/readiness replies).
    *Risk posture:* all are mature, widely used crates; the TLS type selection is explicitly locked to avoid drift.&#x20;
* **Runtime services:** Network (TCP/TLS listeners), OS (signals for shutdown). No direct DB or crypto beyond TLS type selection.&#x20;

### 6) Config & Feature Flags
* **Config:** The crate exports a `Config` type and `/readyz` capacity gates are expected at the service layer; kernel provides the plumbing and health surfaces.&#x20;
* **Features:** Kernel stays minimal; optional adapters (e.g., accounting) are feature-gated in **other crates** (ledger), not here—by design.&#x20;

### 7) Observability
* **Endpoints:** `/metrics`, `/healthz`, `/readyz` exposed by the kernel HTTP server; overload paths return **429/503** downstream when integrated.&#x20;
* **Golden metrics:** kernel-level `bus_lagged_total`, `service_restarts_total`, `request_latency_seconds`; service-level tie-ins for `rejected_total{reason}`.&#x20;

### 8) Concurrency Model
* **Bus:** `tokio::sync::broadcast` with **bounded capacity** (default \~4096) to remain lossy under pressure; **never blocks** kernel critical paths. On overflow, increment a counter and emit a throttled `ServiceCrashed{service:"bus-overflow", reason:...}`.&#x20;
* **Service control:** request/response via bounded `mpsc` + oneshot replies (pattern guidance in blueprint).&#x20;
* **Backpressure:** enforced at boundaries; **no unbounded queues** anywhere.&#x20;

---

## ron-kms

- **path:** `crates/ron-kms` · **role:** lib · **owner:** Stevan White · **maturity:** draft · **last-reviewed:** 2025-09-14

### 1) One-liner
KMS trait + dev in-memory backend for origin key derivation (HKDF), secret sealing, and rotation hooks—kept separate from envelope/auth logic.

### 2) Primary Responsibilities
* Derive per-origin/per-instance keys from a node master key (HKDF) and expose stable `derive_origin_key(..)` semantics.
* Seal/unseal small secrets for services (dev in-memory backend now; pluggable later).
* Provide rotation/audit hooks (emit key lifecycle events via the bus/audit layer).

### 3) Non-Goals
* Envelope formats or request verification (lives in auth/verify crate or service edge).
* Long-term persistent key stores or cloud KMS integrations inside this core crate.
* Access control policy (authn/z) and multi-tenant boundary enforcement.

### 4) Public API Surface
* Re-exports: none.
* Key types / functions / traits:

  * `Kms` trait (core operations: derive, seal/unseal, list/rotate).
  * `InMemoryKms` dev implementation (non-persistent).
  * `derive_origin_key(origin, instance, epoch)` → stable key bytes/handle.
  * `seal(bytes) -> Sealed`, `unseal(Sealed) -> bytes`.
  * Events (conceptual): `KeyRotated`, `KeyDerived` (for audit stream; emitted via caller).
* Events / HTTP / CLI: none in core; a thin Axum service can be added behind a feature in a follow-up crate.

### 5) Dependencies & Coupling
* Internal crates → `ron-proto` (types/ids only, loose; replaceable with adapter: yes).
* External crates (top 5; pins/features) → `hkdf` (HKDF-SHA256), `sha2`, `zeroize` (for secret wiping), `parking_lot` (RwLock), `thiserror`.

  * Low maintenance risk; permissive licenses; small, stable APIs.
* Runtime services: OS RNG via `rand` (for nonces); otherwise none.

### 6) Config & Feature Flags
* Cargo features:

  * `core` (default): HKDF derivation + seal/unseal.
  * (Optional, off by default) `signing-hmac`, `signing-ed25519`: expose raw signing primitives if a caller insists, but envelope usage should live in auth.
* No environment variables. Backend selection is compile-time; future backends (e.g., `kms-file`, `kms-sled`, `kms-axum`) land as separate crates/features.

### 7) Observability
* Not yet implemented. Recommended metrics:

  * `kms_derivations_total{origin}`, `kms_seal_total{ok}`, `kms_unseal_total{ok}`, `kms_errors_total{kind}`.
  * Latency histograms for derive/seal/unseal.
  * Rotation counters (`kms_rotations_total{scope}`).

### 8) Concurrency Model
* Single process, in-memory store guarded by `Arc<RwLock<..>>`.
* Short critical sections (clone entry → operate outside lock).
* No async tasks/channels here; CPU-bound operations, no backpressure lane.
* Timeouts/retries: N/A (callers should implement retries & budgets).

---

## ron-proto

- **path:** `crates/ron-proto` · **role:** lib · **owner:** Stevan White · **maturity:** draft · **last-reviewed:** 2025-09-14

### 1) One-liner
Single source of truth for cross-service DTOs/errors, addressing (BLAKE3-based digests), and OAP/1 protocol constants with JSON/optional MessagePack codecs.

### 2) Primary Responsibilities
* Define stable protocol DTOs and error types shared across services.
* Provide addressing primitives (e.g., `B3Digest`) and canonical constants (`OAP1_MAX_FRAME = 1 MiB`, `STREAM_CHUNK = 64 KiB`).
* Offer wire helpers for JSON and optional rmp-serde encoding/decoding.

### 3) Non-Goals
* Cryptography (key storage, signing, verification) and envelope semantics.
* Networking, persistence, or HTTP/CLI surfaces.
* Policy decisions (authn/z, key rotation).

### 4) Public API Surface
* Re-exports: none.
* Key types / functions:

  * Addressing: `B3Digest` (BLAKE3 digest newtype) and helpers (parse/format; hex `b3:` style).
  * DTO modules for planes/services (overlay, index, storage, gateway, etc.).
  * Error types for protocol/domain errors.
  * Wire: `wire::{to_json, from_json, to_msgpack, from_msgpack}` (MessagePack behind `rmp` feature).
* Events / HTTP / CLI: none.

### 5) Dependencies & Coupling
* Internal crates → none (intentionally decoupled; consumers depend on `ron-proto`).
* External crates (top 5; pins/features) → `serde` (derive), `rmp-serde` (optional), `blake3`, `hex`, `thiserror`.

  * Low risk, permissive licenses; minimal API churn expected.
* Runtime services: none (pure CPU/heap).

### 6) Config & Feature Flags
* Cargo features:

  * `rmp` (default): enable MessagePack (compact wire format); without it JSON-only.
* No environment variables; behavior is compile-time via features.

### 7) Observability
* None currently (no logs/metrics). Future counters: (de)serialize successes/failures per format.

### 8) Concurrency Model
* Pure value types; no interior mutability or async tasks. No backpressure concerns.

---

## ryker

- **path:** `crates/ryker` · **role:** lib · **owner:** Stevan White · **maturity:** draft · **last-reviewed:** 2025-09-14

### 1) One-liner
A tiny supervisor for async tasks that restarts failing jobs with exponential backoff + jitter, with a temporary compatibility shim that re-exports legacy billing helpers.

### 2) Primary Responsibilities
* Provide `spawn_supervised(..)` to run an async factory with automatic restart on error (exp backoff capped at 10s + jitter).
* (Transitional) Re-export legacy billing symbols (`PriceModel`, `compute_cost`, payment validators) via an opt-in compatibility feature.

### 3) Non-Goals
* No business logic (billing, pricing) long-term — those live in `ron-billing`.
* No service orchestration, readiness, or metrics endpoints (that’s kernel/services).
* No actor framework, channels, or backpressure primitives beyond simple restart policy.

### 4) Public API Surface
* Re-exports (behind feature `billing-compat`, enabled by default right now):
  `PriceModel`, `compute_cost`, `validate_payment_block`, `validate_wallet_string` (from `ron-billing`), plus a deprecated marker module `_billing_compat_note`.
* Key functions:
  `spawn_supervised(name: &'static str, factory: impl FnMut() -> impl Future<Output = anyhow::Result<()>>) -> JoinHandle<()>`
* Traits/Types: none public besides what’s re-exported; backoff `jitter(..)` is private.
* Events / HTTP / CLI: none.

### 5) Dependencies & Coupling
* Internal crates →

  * `ron-billing` (optional via `billing-compat`): keeps old `ryker::…` billing imports working during migration. Stability: **loose** (intended to remove). Replaceable: **yes** (turn off feature and update imports).
* External crates (top) →

  * `tokio` (rt, macros, time): async runtime & timers. Mature, low risk.
  * `tracing`: structured logs. Mature, low risk.
  * `rand`: jitter generation. Mature, low risk.
  * `anyhow` (**should be added**): used in function signature; currently missing in `Cargo.toml` (see §13).
* Runtime services: none (no network/storage/crypto). Uses OS timer/PRNG via `rand`.

### 6) Config & Feature Flags
* Cargo features:

  * `billing-compat` (**default ON**): re-exports billing APIs from `ron-billing` to avoid breakage; plan to disable by default after migration and then remove.
* Env vars / config structs: none.

### 7) Observability
* Logs via `tracing`: logs start/complete/fail with error, and sleeps before restart.
* No metrics emitted (e.g., restart counters) and no readiness signal.
* Recommendation: add counters/gauges (`restarts_total{task}`, `backoff_ms{task}`, `last_error{task}` as a label/message) or expose hooks so services can increment metrics.

### 8) Concurrency Model
* Spawns a Tokio task that repeatedly calls a user-supplied async factory.
* On `Err`, sleeps with exponential backoff (start 200ms, \*2 up to 10s) plus random jitter, then restarts.
* On `Ok(())` the loop exits and the supervised task **does not** restart.
* No channels/locks here; no inbuilt cancellation/shutdown token — caller should cancel the `JoinHandle` or make the factory observe a stop signal.

---

## svc-crypto

- **path:** `crates/svc-crypto` · **role:** service · **owner:** Stevan White · **maturity:** draft · **last-reviewed:** 2025-09-14

### 1) One-liner
Local IPC “crypto concierge” that signs, verifies, hashes, and key-manages for other RustyOnions services via a simple Unix socket RPC.

### 2) Primary Responsibilities
* Provide a stable **sign/verify** and **hash/derive** surface to peers over local IPC (UDS).
* **Manage node/app keys** (create, load, rotate, zeroize) with tight FS permissions and optional “amnesia” (ephemeral) mode.
* Validate **capability tokens** (e.g., `CapClaims`) used across services (expiry/nonce/sig) without leaking key material.

### 3) Non-Goals
* No network-facing API, TLS endpoints, or remote HSM/KMS integration (local-only).
* No payment/receipt encoding, billing, or rate computation (belongs in `ryker`/gateway).
* No PQ algorithms by default (optional/behind features only).

### 4) Public API Surface
> Scope is the *service’s* RPC surface (over `ron-bus`/UDS) plus any exported helper types.

* **Re-exports:** (expected minimal) `sha2`, `base64`, `hex` are *not* re-exported; callers use RPC instead of linking libs directly.
* **Key RPCs / DTOs (expected)**

  * `CryptoReq::Sign { key_id, alg, msg } -> CryptoResp::Signature { sig }`
  * `CryptoReq::Verify { key_id?, alg, msg, sig, pk? } -> CryptoResp::Verified { ok }`
  * `CryptoReq::Hash { alg, data } -> CryptoResp::Digest { digest }`
  * `CryptoReq::Random { n } -> CryptoResp::Bytes { data }`
  * `CryptoReq::Keygen { alg, label, persist } -> CryptoResp::KeyInfo { key_id, public }`
  * `CryptoReq::ListKeys -> CryptoResp::Keys { entries }`
  * `CryptoReq::TokenVerify { token_bytes } -> CryptoResp::Token { ok, reason? }`
* **Events / HTTP / CLI:** likely none; startup logs only. (If there’s a small `bin`, it probably just binds the UDS and serves.)

### 5) Dependencies & Coupling
* **Internal crates:**

  * `ron-bus` (tight) — IPC envelope and UDS helpers; replaceable with a split `ron-ipc` if we rename. \[replaceable: yes]
  * Possibly `ron-kernel` for health/metrics bus events (loose). \[replaceable: yes]
* **External (top 5, expected):**

  * `ed25519-dalek` (v2) — signatures; maintained; MIT/Apache; low risk.
  * `sha2` (0.10), `blake3` (optional) — hashing; low risk.
  * `rand`/`rand_chacha` + `getrandom` — CSPRNG; critical; low risk.
  * `zeroize` — secret zeroization; low risk, important.
  * `anyhow`/`thiserror` — error ergonomics; low risk.
  * (Optional) `pqcrypto`/`oqs` behind feature flags — high maintenance risk; large footprints.
* **Runtime services:** OS filesystem (key store), OS RNG; no DB required unless we use sled for metadata.

### 6) Config & Feature Flags
* **Env/config (suggested/typical):**

  * `RON_CRYPTO_SOCK` (default: `/tmp/ron/svc-crypto.sock`).
  * `RON_CRYPTO_KEYDIR` (default: `$XDG_DATA_HOME/ron/keys` or `/var/lib/ron/keys`).
  * `RON_CRYPTO_AMNESIA=1` → in-mem keys, no persistence.
  * `RON_CRYPTO_ALLOWED_ALGS=ed25519,sha256,...` → whitelist for policy.
  * `RON_CRYPTO_MAX_REQ_BYTES` → guardrail against DoS.
* **Cargo features:**

  * `pq` → enable PQ signatures/hashes (Dilithium/Kyber via oqs).
  * `blake3` → enable BLAKE3 hashing.
  * `metrics` → Prometheus counters/histograms.
  * `serde-pubkey` → (if serializing public keys in JSON for tooling).

### 7) Observability
* **Metrics (if `metrics`):**

  * `crypto_requests_total{op,alg,ok}`
  * `crypto_bytes_total{dir=ingress|egress}`
  * `crypto_latency_seconds{op,alg}` histogram
  * `crypto_key_load_failures_total{reason}`
* **Health:** report ready only after key store scan completes and UDS is bound.
* **Tracing:** span per request with `corr_id`, `op`, `alg`, `key_id`; never log secrets/material.

### 8) Concurrency Model
* UDS accept loop (blocking or tokio), one task/thread per client.
* **Backpressure:** OS socket buffers + per-request size checks; optional bounded job channel if requests are offloaded to worker pool.
* **Locks:** `parking_lot::RwLock` around keystore map; `zeroize` on drop.
* **Timeouts/retries:** Deadline per request (e.g., 2s) to avoid hung clients; caller retriable on `io::ErrorKind::TimedOut`.
* **Isolation:** Signing operations are cheap (ed25519), so inline is OK; heavy PQ ops (if enabled) should go to a worker pool.

---

## svc-index

- **path:** `crates/svc-index` · **role:** service · **owner:** Stevan White · **maturity:** draft · **last-reviewed:** 2025-09-14

### 1) One-liner
Local Unix-socket index service that maps RustyOnions content addresses to bundle directories and answers resolve/put requests for peers.

### 2) Primary Responsibilities
* Maintain the **address → directory** mapping (open the index DB; get/set entries).
* Serve **IPC RPCs** over UDS (MsgPack) for `Resolve`, `PutAddress`, and `Health`.
* Emit concise **structured logs** (JSON via `tracing-subscriber`) and stay simple/robust under kernel supervision.

### 3) Non-Goals
* No HTTP; no direct file I/O of bundle contents (that’s `gateway` / `svc-storage`).
* No overlay/network lookups, caching, or fetch from remote peers.
* No authorization policy or capability enforcement beyond local UDS trust.

### 4) Public API Surface
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

### 5) Dependencies & Coupling
* **Internal crates**

  * `index` (tight): provides the actual DB engine/operations for the mapping. Replaceable: **yes** (e.g., swap sled/other store behind the `index` crate).
  * `naming` (tight): `Address` parsing/normalization (prevents format drift). Replaceable: **no** in practice (all address semantics live there).
  * `ron-bus` (tight): IPC envelopes + UDS helpers. Replaceable: **yes** if/when we split to `ron-proto` + `ron-ipc` (recommended).
* **External crates (top)**

  * `serde`, `rmp-serde`: MsgPack encode/decode (stable, low risk).
  * `tracing`, `tracing-subscriber` (json, env-filter): structured logs (low risk).
  * `regex`: present in Cargo; not used in this file—likely used in `index` crate or is a leftover (watch for dead dep).
* **Runtime services:** OS UDS and filesystem (index DB path).

### 6) Config & Feature Flags
* **Env vars**

  * `RON_INDEX_SOCK` (default **`/tmp/ron/svc-index.sock`**): UDS path to bind.
  * `RON_INDEX_DB` (default **`.data/index`**): DB path opened by `index::Index::open`.
* **Cargo features:** none declared here.
* **Notes:** Defaults are sensible, but the **DB path must match other services** (e.g., gateway, overlay, storage) or you’ll see spurious `NotFound` on resolve.

### 7) Observability
* **Logs:** JSON logs via `tracing-subscriber` + `EnvFilter` (e.g., `info!` on start, resolve/put; `error!` on decode/DB errors).
* **Metrics:** none.
* **Health/Readiness:** RPC `Health → HealthOk`. No Prometheus endpoints.

### 8) Concurrency Model
* **Blocking UDS** listener; **one OS thread per client** (`std::thread::spawn` in accept loop).
* **Backpressure:** OS socket buffers only. No in-process queues, limits, or rate control.
* **Timeouts/Retries:** none at service layer. A stuck client could hold a thread.
* **Shared state:** `Arc<index::Index>`; internal locking is managed by the `index` crate.

---

## svc-omnigate

- **path:** `crates/svc-omnigate` · **role:** service · **owner:** Stevan White · **maturity:** draft · **last-reviewed:** 2025-09-14

### 1) One-liner
Multi-tenant gateway that fronts RustyOnions services, enforcing quotas/backpressure, exposing health/metrics/readiness, and brokering app traffic over the OAP/1 protocol to storage/index/mailbox while staying app-agnostic.&#x20;

### 2) Primary Responsibilities
* Enforce per-tenant capacity (token buckets), overload discipline (429/503 + Retry-After), and capacity-aware `/readyz`.&#x20;
* Terminate the external surface (HTTP/OAP/1), route to internal services (index/storage/mailbox), and keep protocol limits distinct from storage chunking. &#x20;
* Export “golden metrics” for requests, bytes, latency, rejects/quotas, and inflight.&#x20;

### 3) Non-Goals
* No app-specific behavior or business logic (keep kernel/services neutral; use SDK + app\_proto\_id for app needs).&#x20;
* No PQ/QUIC/ZK changes to OAP/1 at this layer (future-tracks in OAP/2/R\&D).&#x20;

### 4) Public API Surface
* **Re-exports:** None required for consumers; this is a service binary exposing endpoints.
* **HTTP Endpoints (expected):**

  * `/healthz` (liveness), `/readyz` (capacity-aware readiness), `/metrics` (Prometheus). &#x20;
  * Object read path (via gateway surface proved by gwsmoke; stable ETag/Range/encoding semantics).&#x20;
* **Protocol:** OAP/1 framing and error mapping; HELLO advertises limits; max\_frame = 1 MiB (distinct from storage 64 KiB streaming). &#x20;
* **Error envelope (target):** `{code,message,retryable,corr_id}`; 400/404/413/429/503 + Retry-After.&#x20;

### 5) Dependencies & Coupling
* **Internal crates (via RPC/UDS/HTTP):**

  * `svc-index`, `svc-storage`, `svc-overlay` (read path; content-addressed GET/HAS/stream). Coupling: *loose* (over network/UDS); replaceable=yes.&#x20;
  * `svc-mailbox` (SEND/RECV/ACK; later DELETE/SUBSCRIBE/visibility). Coupling: *loose*; replaceable=yes.&#x20;
  * `ron-kernel` for service bus events & invariants; OAP/1 codec defined outside kernel (no app logic). Coupling: *loose*.&#x20;
* **External crates (expected by repo invariants):** Axum 0.7 (HTTP), Tokio (async), Prometheus client (metrics), tokio-rustls (TLS), Serde/rmp-serde (wire). Risk moderate (well-maintained); TLS uses rustls.&#x20;
* **Runtime services:** Network (HTTP/UDS), Storage (blob/index), OS (sockets), Crypto (caps/tokens; APP\_E2E opaque).&#x20;

### 6) Config & Feature Flags
* **Env & files:** `RON_QUOTA_PATH` (per-tenant rate config), `RON_NODE_URL` / `RON_CAP` for SDK/clients; capacity gating via `/readyz`. &#x20;
* **Service config (Omnigate):** TOML with DoS PoW toggles, QoS for mailbox (WS subscribe), optional federation (off by default, ZK handshake), gRPC control addr.&#x20;
* **Spec source control:** Local `/docs/specs/OAP-1.md` mirrors GMI-1.6 to prevent drift.&#x20;

### 7) Observability
* **Metrics (golden set):** `requests_total{code}`, `bytes_{in,out}_total`, `latency_seconds`, `rejected_total{reason}`, `inflight`, `quota_exhaustions_total` (and cache/range counters when serving objects). &#x20;
* **Health/Readiness:** `/healthz` + capacity-aware `/readyz` that gates load.&#x20;
* **Tracing:** Correlation IDs end-to-end; SDK propagates `corr_id`.&#x20;

### 8) Concurrency Model
* **Tasks:** HTTP acceptor; per-tenant token-bucket middleware; backend client pools for index/storage/mailbox; metrics exporter loop.
* **Backpressure/overload:** Token buckets; 429/503 with Retry-After; bounded inflight; compression guardrails to cap decompressed size/ratio.  &#x20;
* **Timeouts/retries:** Error taxonomy + SDK jittered retries respecting Retry-After.&#x20;

---

## svc-overlay

- **path:** `crates/svc-overlay` · **role:** service · **owner:** Stevan White · **maturity:** draft · **last-reviewed:** 2025-09-14

### 1) One-liner
Thin overlay service that answers RPC `Health` and `Get{addr,rel}` over a local UDS “bus,” resolving content addresses via `svc-index` and returning file bytes from `svc-storage`.

### 2) Primary Responsibilities
* Accept `OverlayReq::Get` and return `OverlayResp::{Bytes,NotFound,Err}`.
* Resolve `addr` → directory via `svc-index`, then read a file (default `payload.bin` if `rel==""`) from `svc-storage`.
* Provide a lightweight health probe (`OverlayReq::Health → OverlayResp::HealthOk`).

### 3) Non-Goals
* No public HTTP surface, quotas, or request shaping (that’s `svc-omnigate`/gateway).
* No durable storage, manifests, DHT routing, or chunk management (owned by `svc-storage` / future DHT).
* No application-level crypto semantics (APP E2E stays opaque above the service plane).

### 4) Public API Surface
* **Wire (bus/UDS, MessagePack via `rmp-serde`):**

  * **Request enums (from `ron_bus::api`):** `OverlayReq::{Health, Get{addr:String, rel:String}}`.
  * **Response enums:** `OverlayResp::{HealthOk, Bytes{data:Vec<u8>}, NotFound, Err{err:String}}`.
  * **Envelope fields set by this service:** `service="svc.overlay"`, `method ∈ {"v1.ok","v1.not_found","v1.err"}`, `corr_id` echoed, `token=[]`.
* **No Rust re-exports intended for external consumers (service binary).**

### 5) Dependencies & Coupling
* **Internal crates (via API types, not direct linking):**

  * `ron-bus` — Envelope + UDS helpers (`listen/recv/send`) and shared request/response enums. *Loose; replaceable=yes.*
  * Communicates with **`svc-index`** (`IndexReq::Resolve → IndexResp::{Resolved,NotFound,Err}`) and **`svc-storage`** (`StorageReq::Read{dir,rel} → StorageResp::{File,NotFound,Err}`) over UDS. *Loose; replaceable=yes.*
* **External crates (workspace-pinned):** `anyhow`, `serde`, `rmp-serde`, `tracing`, `tracing-subscriber`.
* **Runtime services:** Unix domain sockets only (POSIX); no network listeners; local filesystem reads happen in `svc-storage`, not here. No TLS at this hop.

### 6) Config & Feature Flags
* **Env vars:**

  * `RON_OVERLAY_SOCK` (default `/tmp/ron/svc-overlay.sock`) — where this service listens.
  * `RON_INDEX_SOCK` (default `/tmp/ron/svc-index.sock`) — where to reach `svc-index`.
  * `RON_STORAGE_SOCK` (default `/tmp/ron/svc-storage.sock`) — where to reach `svc-storage`.
* **Cargo features:** none declared.
* **Filesystem:** creates parent directory for its UDS path if missing (macOS tmpdirs friendliness).

### 7) Observability
* **Logs:** structured `tracing` at `info`/`error` with high-signal records (`%addr`, `%rel`, bytes length, and error contexts).
* **Health:** RPC health handler (`OverlayReq::Health`). No HTTP `/healthz` and no Prometheus metrics in this crate (yet).
* **Correlation:** echoes `corr_id` from inbound Envelope.

### 8) Concurrency Model
* **Server pattern:** blocking UDS listener; **per-connection thread** (`std::thread::spawn`) that handles exactly one request/response pair (receive → match → reply).
* **Backpressure:** OS accept queue + thread scheduling; no explicit inflight limits besides OS resources.
* **Timeouts/retries:** none at this layer; callers (gateway/SDK) should apply deadlines and retries.
* **Sync I/O:** uses `UnixStream` (no Tokio); simple, portable across POSIX, but each connection ties up a thread.

---

## svc-storage

- **path:** `crates/svc-storage` · **role:** service · **owner:** Stevan White · **maturity:** draft · **last-reviewed:** 2025-09-14

### 1) One-liner
Durable, content-addressed DAG store with rolling-hash chunking and Reed–Solomon erasure that serves objects to the overlay in 64 KiB streams, verifying BLAKE3 before returning bytes.  &#x20;

### 2) Primary Responsibilities
* Store objects as a manifest/DAG (DAG-CBOR) and chunks, addressed by `b3:<hex>` (BLAKE3-256).&#x20;
* Provide read-path primitives (`GET`, `HAS`) over an internal API so Overlay can stream **64 KiB** chunks to Gateway/clients. &#x20;
* Maintain durability & availability via erasure coding and background repair.&#x20;

### 3) Non-Goals
* No public HTTP surface, quotas, or tenant policy (that’s Gateway/Omnigate).&#x20;
* No provider discovery or routing (Index/DHT handle resolve/announce).&#x20;
* No application-level behavior or decryption; services verify bytes and keep APP\_E2E opaque.&#x20;

### 4) Public API Surface
* **Re-exports:** none (service binary).
* **Service endpoints (internal):** read-path API offering `GET(hash=b3:<hex>)` and `HAS(hash)`; streaming in 64 KiB chunks (implementation detail distinct from OAP/1 `max_frame=1 MiB`). &#x20;
* **Manifests/DAG:** objects modeled as manifests (DAG-CBOR) with chunk references.&#x20;
* **Admin plane (repo invariant):** `/healthz`, `/readyz`, `/metrics` exposed across services.&#x20;

### 5) Dependencies & Coupling
* **Internal crates → why; stability; replaceable?**

  * **svc-overlay** (caller): streams objects to edge; *loose coupling* over RPC/UDS; replaceable=yes.&#x20;
  * **svc-index** (peer): stores/serves provider records; storage itself does not resolve; *loose*; replaceable=yes.&#x20;
* **External crates (likely, per blueprint standards):**

  * `blake3` (addressing/integrity), `reed-solomon-erasure` (parity/repair), `tokio` & `bytes` (async/zero-copy), telemetry stack (Prometheus, tracing). These are mainstream/maintained → moderate risk. &#x20;
* **Runtime services:** local disk (object/chunk store), OS (files/sockets), internal RPC (UDS/TCP) from Overlay, crypto (BLAKE3 digest). &#x20;

### 6) Config & Feature Flags
* **Store root path** used by pack/tools and services (e.g., `.onions` in quick-start), must align across writer and storage to avoid phantom 404s. &#x20;
* **Streaming/erasure knobs:** 64 KiB streaming chunk is an implementation detail; erasure coding parameters and **repair pacing ≤ 50 MiB/s** per cluster (operational cap). &#x20;
* **Cargo features:** none called out yet (PQ and federation features live elsewhere / future).&#x20;

### 7) Observability
* **Endpoints:** `/healthz`, `/readyz`, `/metrics` present per repo invariant.&#x20;
* **Golden metrics (target set):** `requests_total{code}`, `bytes_{in,out}_total`, `latency_seconds`, `inflight`, plus storage histograms; to be standardized across services.&#x20;

### 8) Concurrency Model
* **Runtime:** Tokio multi-threaded; bounded CPU pool for hashing/erasure/compression; cancellations propagate (parent drops cancel children).&#x20;
* **Backpressure:** Semaphores at dials/inbound/CPU; bounded queues; deadlines on awaits.&#x20;
* **Zero-copy:** use `bytes::Bytes`, vectored I/O on hot paths.&#x20;

---

## tldctl

- **path:** `crates/tldctl` · **role:** lib · **owner:** Stevan White · **maturity:** draft · **last-reviewed:** 2025-09-14

### 1) One-liner
Library + thin CLI for **packing local files into content-addressed bundles** (BLAKE3 `b3:<hex>`) with manifest and precompressed variants, writing to the store root and index DB, and printing the canonical address.  &#x20;

### 2) Primary Responsibilities
* **Pack**: build a bundle (`Manifest.toml`, `payload.bin`, optional `.br`/`.zst`) and compute the BLAKE3 address (`b3:<hex>.<tld>`). &#x20;
* **Persist**: write bundle to a **store root** (e.g., `.onions`) and update **Index DB** at the configured sled path.&#x20;
* **Report**: print the final address to STDOUT for downstream use (e.g., Gateway GET).&#x20;

### 3) Non-Goals
* No HTTP service surface, quotas, or readiness/metrics (this is a local packer, not a server). (Implied by CLI usage only.)&#x20;
* No DHT/provider routing, overlay streaming, or storage durability; those live in services (`svc-index`, `svc-overlay`, `svc-storage`).&#x20;
* No application-level crypto; it uses **content addressing** only (BLAKE3 `b3:<hex>`).&#x20;

### 4) Public API Surface
* **Re-exports:** none (internal lib consumed by its own bin + tests using “reuse the packing routine”).&#x20;
* **Key functions (implied by usage & docs):**

  * `pack(input, tld, index_db, store_root) -> PackedAddr` (builds bundle, writes to store/index, returns `b3:<hex>.<tld>`).&#x20;
  * Helpers for **precompression** (`.br`, `.zst`) and manifest emission (`Manifest.toml`).&#x20;
* **CLI (bin target):** `tldctl pack --tld <text|…> --input <file> --index-db <path> --store-root <dir>`; prints the computed address.&#x20;

### 5) Dependencies & Coupling
* **Internal crates**

  * **svc-index** (writes/reads the **sled** Index DB path used by services). *Coupling: medium (direct DB access today); replaceable: yes (shift to UDS daemon call).* &#x20;
  * **svc-storage** (indirect—consumes bundles later; no direct link at pack time). *Loose; replaceable: yes.*&#x20;
* **External crates (likely top set, by features used):**

  * `blake3` (addressing), `zstd` / `brotli` (precompression), `toml`/`serde` (manifest I/O), `sled` (Index DB). *All mainstream; moderate risk.* &#x20;
* **Runtime services:** **Filesystem** (store root), **sled DB** (index); **no network/TLS**.&#x20;

### 6) Config & Feature Flags
* **Env compatibility (used in docs/scripts):**

  * `RON_INDEX_DB` — sled DB location used by pack and services.&#x20;
  * `OUT_DIR` / `--store-root` — bundle root (e.g., `.onions`).&#x20;
* **CLI args:** `--tld`, `--input`, `--index-db`, `--store-root`.&#x20;
* **Features:** none documented.

### 7) Observability
* **Logs:** standard CLI logs (stderr).
* **No metrics or health endpoints** (library/CLI only); services provide `/healthz` `/readyz` `/metrics`.&#x20;

### 8) Concurrency Model
* **CLI pipeline** (compute → precompress → write → index update) runs **synchronously**; no server tasks/backpressure here. (Implied by one-shot CLI script flow.)&#x20;

---

## transport

- **path:** `crates/transport` · **role:** lib · **owner:** Stevan White · **maturity:** draft · **last-reviewed:** 2025-09-14

### 1) One-liner
Async TCP/TLS listener/dialer layer that enforces connection limits and per-socket timeouts, integrates with kernel health/metrics, and hands framed bytes to higher layers without embedding app semantics. &#x20;

### 2) Primary Responsibilities
* Bind and serve a socket (optionally TLS) with **max connections** and **idle/read/write timeouts** as configured. &#x20;
* Provide a clean async accept loop that spawns per-connection tasks and exposes the bound address/handle to the caller. &#x20;
* Surface health/metrics for open conns, bytes, errors, and backpressure drops via the project’s observability conventions.&#x20;

### 3) Non-Goals
* No routing, quotas, or protocol parsing (OAP/1 lives above; gateway/omnigate do edge policy).&#x20;
* No persistence or chunk storage (overlay/storage own that).&#x20;
* No app/business logic inside transport (kernel stays tiny: **transport + supervision + metrics + bus only**).&#x20;

### 4) Public API Surface
* **Re-exports:** Tokio primitives and `tokio_rustls::rustls` TLS config types are accepted but not re-exported (see TLS note below).&#x20;
* **Key types / functions (expected, per project invariants):**

  * `TransportConfig { addr, name, max_conns, read_timeout, write_timeout, idle_timeout }` (maps 1:1 to `[[transport]]` TOML in templates).&#x20;
  * `spawn_transport(cfg, metrics, health, bus, tls: Option<tokio_rustls::rustls::ServerConfig>) -> Result<(JoinHandle<()>, SocketAddr)>`. TLS type choice is a project invariant.&#x20;
* **Events / hooks:** On fatal listener errors publish restart/crash reasons to the Bus; reject on overload per global policy. &#x20;

### 5) Dependencies & Coupling
* **Internal crates → why, stability, replaceable?**

  * `ron-kernel` (health/metrics/bus) — reports readiness and emits `ServiceCrashed{reason}`; coupling *loose*; replaceable *no* (core). &#x20;
* **External crates (top 5; pins/features)**

  * `tokio` (async runtime), `tokio-rustls` (TLS), `bytes` (zero-copy), `tracing` (logs), `prometheus` (metrics). All mainstream/maintained → moderate risk.&#x20;
* **Runtime services:** Network sockets (TCP), optional TLS keypair/cert, optional Tor SOCKS if the embedding app chooses to route via Tor.&#x20;

### 6) Config & Feature Flags
* **Config struct:** mirrors repo templates—`max_conns`, `idle_timeout_ms`, `read_timeout_ms`, `write_timeout_ms`.&#x20;
* **TLS inputs:** PEM files (prod templates show `tls_cert_file`, `tls_key_file`) mapped into `rustls::ServerConfig`.&#x20;
* **Defaults we ship with (baseline):** handshake **2s**, read idle **15s**, write **10s**.&#x20;
* **Features:** none required for core; Tor/arti and QUIC are future/optional at higher layers.&#x20;

### 7) Observability
* **Metrics:** requests/4xx/5xx, latency histograms, queue depths, restarts, bytes in/out, **open conns**, backpressure drops.&#x20;
* **Tracing:** correlation IDs propagated across RPC boundaries.&#x20;
* **Readiness/health:** tied to accept loop liveliness and connection budget.&#x20;

### 8) Concurrency Model
* Tokio multi-threaded runtime; acceptor task + per-connection tasks; **semaphores** for dials/inbound; deadlines on awaits; cancellation on parent drop; bounded queues to avoid collapse.&#x20;
* Global concurrency cap via Tower-style middleware when embedded in HTTP stacks.&#x20;

---
