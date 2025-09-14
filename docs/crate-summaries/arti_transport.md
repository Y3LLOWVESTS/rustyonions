---

crate: arti\_transport
path: crates/arti\_transport
role: lib
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
-------------------------

## 1) One-liner

SOCKS5-based outbound (Tor/Arti) plus a tiny Tor control-port client to publish a v3 hidden service and hand connections to the core transport `Handler`.

## 2) Primary Responsibilities

* Dial arbitrary `host:port` through a SOCKS5 proxy (Tor/Arti) and return a `ReadWrite` stream.
* Publish a Tor v3 hidden service (ephemeral by default, persistent if configured) and accept inbound connections, piping each to the provided `Handler`.
* Track I/O with shared byte counters (`accounting::Counters` / `CountingStream`).

## 3) Non-Goals

* No direct async runtime integration (uses blocking `std::net` + one background thread).
* No integrated Prometheus/HTTP metrics export, rate limiting, or access control.
* No Tor process management (expects an already-running Tor/Arti with SOCKS & control ports).

## 4) Public API Surface

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

## 5) Dependencies & Coupling

* **Internal crates:**

  * `transport` → Provides `Transport`, `ReadWrite`, `Handler`. **Tight** (core trait boundary). Replaceable: **no** (by design).
  * `accounting` → Byte counters wrapper. **Loose** (can be replaced with any tallying). Replaceable: **yes**.
* **External crates (top 5):**

  * `socks = "0.3"` → SOCKS5 client used for outbound. Mature, MIT/Apache; moderate maintenance risk (simple, but not high-churn).
  * `anyhow` (workspace) → error plumbing; low risk.
  * `tracing = "0.1"` → logging facade (currently underused in this crate); low risk.
  * (No TLS libs; control port is plain TCP to localhost.)
* **Runtime services:** Requires Tor/Arti SOCKS (e.g., `127.0.0.1:9050`) and ControlPort (e.g., `127.0.0.1:9051`) reachable on localhost. No DB or crypto done locally (keys handled as strings passed to Tor).

## 6) Config & Feature Flags

* **Env vars:**

  * `RO_HS_KEY_FILE` (optional): path to persist the private key for the v3 HS.

    * **Unset** → Ephemeral onion (`Flags=DiscardPK`).
    * **Set & file exists** → Reuse the exact Tor key string (`ED25519-V3:...`).
    * **Set & file missing** → Request NEW key and write it to the path.
* **Constructor args:** `socks_addr`, `tor_ctrl_addr`, `connect_timeout`.
* **Cargo features:** none.

## 7) Observability

* `accounting::Counters` wraps all streams (in/out totals + 60-minute ring).
* Logs: primarily `eprintln!` in accept errors; `tracing` is present but not consistently used here.
* No `/healthz` or readiness endpoints (library).

## 8) Concurrency Model

* **Outbound:** blocking connect via `socks::Socks5Stream`; per-stream socket timeouts set to `connect_timeout`.
* **Inbound HS:** creates a `TcpListener` on `127.0.0.1:0`; publishes ADD\_ONION; spawns **one background thread** that:

  * Loops over `ln.incoming()` and, for each connection, wraps with `CountingStream` and calls `handler(stream)`.
  * Note: calls `handler` **inline** on the accept thread (no per-connection thread/pool). A long-running `handler` will serialize accepts.
* **Ctrl-port:** blocking `TcpStream` client; sends `PROTOCOLINFO`, parses AUTH methods, then `AUTHENTICATE` (SAFECOOKIE or NULL).

## 9) Persistence & Data Model

* No database. Optional persistence of the **exact** Tor private key string into `RO_HS_KEY_FILE` (if configured). Retention: indefinite (left to filesystem).

## 10) Errors & Security

* **Error taxonomy:** Network/IO errors from SOCKS or control port bubble up via `anyhow::Result`. HS publish failures (5xx from Tor) are surfaced with contextual messages. No internal retry/backoff.
* **AuthN/AuthZ:** Auth to control port via Tor’s SAFECOOKIE (preferred) or NULL (if Tor permits). No app-layer auth on inbound connections.
* **Secrets:** The HS private key (if persisted) is stored as a plaintext line (as Tor emits it). **File permissions are not forced** here—caller should ensure secure perms (0600).
* **TLS:** None at this layer (SOCKS/control are plaintext localhost).
* **PQ-readiness:** N/A in this crate (delegated to Tor; onion keys are Ed25519).

## 11) Performance Notes

* **Hot paths:** Per-IO accounting increments; SOCKS connect; HS accept → `handler`.
* **Targets:** Suitable for low/medium throughput; overhead minimal except when `handler` blocks.
* **Bottlenecks / wins:**

  * Single accept thread + inline `handler` → backpressure if `handler` is slow. Consider spawning per-conn worker threads or handing off to a channel/async runtime.
  * Switch `eprintln!` to `tracing` with structured fields (service id, peer addr, duration).
  * Optional: move to non-blocking/Tokio for better scalability.

## 12) Tests

* **Unit:** none present in this crate zip.
* **Integration/E2E:** none here (E2E expected in higher layers’ smoke tests).
* **Fuzz/loom:** none.

## 13) Improvement Opportunities

* **Concurrency:** Offload each accepted connection to a worker (thread or async task) so accept loop stays hot.
* **Observability:** Use `tracing::{info,warn,error}`; attach counters snapshots to periodic logs; expose simple adapters for Prometheus (optional module).
* **Security:** When persisting HS key, ensure the file is created with `0600` (platform-appropriate) and avoid leaving it world-readable.
* **Resilience:** Add limited retries/backoff for control-port operations (authenticate, ADD\_ONION, DEL\_ONION).
* **Config hygiene:** Consider optional envs for SOCKS/CTRL addresses or a tiny Config struct to avoid hardcoding in call sites.
* **Error clarity:** Normalize Tor control errors (250/550 parsing) into typed errors (retryable vs terminal).
* **Overlap signals:** This crate is the Tor/Arti transport; avoid duplicating HS logic elsewhere (e.g., gateway/node overlay). If another crate publishes onions, factor a shared `tor_ctrl` util.

## 14) Change Log (recent)

* 2025-09-14 — Initial `Transport` impl with SOCKS outbound, HS publish via control port; added byte accounting on all streams; ephemeral/persistent HS modes via `RO_HS_KEY_FILE`; best-effort `DEL_ONION` on drop.

## 15) Readiness Score (0–5 each)

* **API clarity:** 3 — Small, clear surface; needs doc comments and examples.
* **Test coverage:** 1 — No unit/integration tests yet.
* **Observability:** 2 — Counters present; tracing underused; no exporter.
* **Config hygiene:** 3 — Constructor args + one env; could consolidate in a config struct.
* **Security posture:** 3 — SAFECOOKIE support; improve key file permissions/handling.
* **Performance confidence:** 3 — Fine for modest load; accept loop serialization is a known limiter.
* **Coupling (lower is better):** 2 — Intentional tie to `transport`; other deps are lightweight and swappable.
