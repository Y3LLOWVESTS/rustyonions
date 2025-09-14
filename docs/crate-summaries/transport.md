---

crate: transport
path: crates/transport
role: lib
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
-------------------------

## 1) One-liner

Async TCP/TLS listener/dialer layer that enforces connection limits and per-socket timeouts, integrates with kernel health/metrics, and hands framed bytes to higher layers without embedding app semantics. &#x20;

## 2) Primary Responsibilities

* Bind and serve a socket (optionally TLS) with **max connections** and **idle/read/write timeouts** as configured. &#x20;
* Provide a clean async accept loop that spawns per-connection tasks and exposes the bound address/handle to the caller. &#x20;
* Surface health/metrics for open conns, bytes, errors, and backpressure drops via the project’s observability conventions.&#x20;

## 3) Non-Goals

* No routing, quotas, or protocol parsing (OAP/1 lives above; gateway/omnigate do edge policy).&#x20;
* No persistence or chunk storage (overlay/storage own that).&#x20;
* No app/business logic inside transport (kernel stays tiny: **transport + supervision + metrics + bus only**).&#x20;

## 4) Public API Surface

* **Re-exports:** Tokio primitives and `tokio_rustls::rustls` TLS config types are accepted but not re-exported (see TLS note below).&#x20;
* **Key types / functions (expected, per project invariants):**

  * `TransportConfig { addr, name, max_conns, read_timeout, write_timeout, idle_timeout }` (maps 1:1 to `[[transport]]` TOML in templates).&#x20;
  * `spawn_transport(cfg, metrics, health, bus, tls: Option<tokio_rustls::rustls::ServerConfig>) -> Result<(JoinHandle<()>, SocketAddr)>`. TLS type choice is a project invariant.&#x20;
* **Events / hooks:** On fatal listener errors publish restart/crash reasons to the Bus; reject on overload per global policy. &#x20;

## 5) Dependencies & Coupling

* **Internal crates → why, stability, replaceable?**

  * `ron-kernel` (health/metrics/bus) — reports readiness and emits `ServiceCrashed{reason}`; coupling *loose*; replaceable *no* (core). &#x20;
* **External crates (top 5; pins/features)**

  * `tokio` (async runtime), `tokio-rustls` (TLS), `bytes` (zero-copy), `tracing` (logs), `prometheus` (metrics). All mainstream/maintained → moderate risk.&#x20;
* **Runtime services:** Network sockets (TCP), optional TLS keypair/cert, optional Tor SOCKS if the embedding app chooses to route via Tor.&#x20;

## 6) Config & Feature Flags

* **Config struct:** mirrors repo templates—`max_conns`, `idle_timeout_ms`, `read_timeout_ms`, `write_timeout_ms`.&#x20;
* **TLS inputs:** PEM files (prod templates show `tls_cert_file`, `tls_key_file`) mapped into `rustls::ServerConfig`.&#x20;
* **Defaults we ship with (baseline):** handshake **2s**, read idle **15s**, write **10s**.&#x20;
* **Features:** none required for core; Tor/arti and QUIC are future/optional at higher layers.&#x20;

## 7) Observability

* **Metrics:** requests/4xx/5xx, latency histograms, queue depths, restarts, bytes in/out, **open conns**, backpressure drops.&#x20;
* **Tracing:** correlation IDs propagated across RPC boundaries.&#x20;
* **Readiness/health:** tied to accept loop liveliness and connection budget.&#x20;

## 8) Concurrency Model

* Tokio multi-threaded runtime; acceptor task + per-connection tasks; **semaphores** for dials/inbound; deadlines on awaits; cancellation on parent drop; bounded queues to avoid collapse.&#x20;
* Global concurrency cap via Tower-style middleware when embedded in HTTP stacks.&#x20;

## 9) Persistence & Data Model

* None (in-memory counters only). Transport is stateless by design.&#x20;

## 10) Errors & Security

* **Error taxonomy:** Listener bind/accept errors escalate through supervision (backoff + restart intensity caps). Overload returns **429/503** at edges using higher-level components. &#x20;
* **TLS:** `tokio_rustls::rustls::ServerConfig` is the required type; mTLS/Noise for non-public planes is handled by services that build atop transport. &#x20;
* **Secrets:** no secrets in logs; rotation and amnesia mode belong to upper layers.&#x20;
* **PQ-readiness:** transport stays stable for OAP/1; PQ changes are planned in **OAP/2** (feature-gated) above transport.&#x20;

## 11) Performance Notes

* **Hot path:** accept → TLS handshake (if any) → framed I/O → shutdown; zero-copy (`bytes::Bytes`) on hot paths.&#x20;
* **Targets:** internal API **p95 < 40 ms**, **p99 < 120 ms** under nominal budgets.&#x20;
* **Budgets:** inbound queue ≈ **\~1k msgs/listener**; tune `max_conns` and timeouts to maintain SLOs.&#x20;

## 12) Tests

* **Recommended patterns:** hermetic duplex I/O (`tokio::io::duplex`) for transport round-trip; chaos tests (slow-loris/packet loss) in harness; property tests for framed boundaries. &#x20;
* **Interop suite:** transport must pass the matrix that runs over **TCP+TLS** & **Tor**, comparing bytes/timings.&#x20;

## 13) Improvement Opportunities

* **QUIC (exploratory):** optional feature for low-latency paths once OAP/1 parity is proven. (Keep transport pluggable.)&#x20;
* **Tor/arti adapter:** align with private onion runtime (`svc-oniond`) when promoted; expose a SOCKS dialer hook cleanly.&#x20;
* **Metrics hardening:** add per-listener inflight gauges and handshake failure counters mapped to reasons.&#x20;
* **Config polish:** publish a tiny schema/doc for `TransportConfig` with the shipped defaults.&#x20;

## 14) Change Log (recent)

* **2025-09-05** — Project invariants pinned: **rustls TLS type**, health/ready/metrics required across services; timeouts/defaults documented. &#x20;

## 15) Readiness Score (0–5 each)

* **API clarity:** 3.5 — Responsibilities and config shape are clear; publish the exact `TransportConfig`/`spawn_transport` signature.&#x20;
* **Test coverage:** 2.5 — Patterns and harness guidance exist; formalized transport tests (duplex/chaos) should land. &#x20;
* **Observability:** 3.0 — Metrics slate defined; ensure gauges/counters are wired before Silver.&#x20;
* **Config hygiene:** 3.5 — Templates show timeouts/TLS; codify schema + defaults in the crate. &#x20;
* **Security posture:** 3.0 — Correct TLS stack and secret-handling posture via upper layers; PQ defers to OAP/2. &#x20;
* **Performance confidence:** 3.0 — Budgets and SLOs are explicit; needs sustained soak + handshake error histograms. &#x20;
* **Coupling (lower is better):** 2.5 — Thin integration to kernel (bus/metrics/health); otherwise protocol-agnostic.&#x20;

