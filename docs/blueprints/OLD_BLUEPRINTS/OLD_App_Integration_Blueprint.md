Perfected. I updated the blueprint with a **Perfection Pass — Final Refinements (v1.0)** and flipped the right defaults:

# Executive Summary

**What you said:** Apps should have access to a **private, per‑app onion transport** for end‑to‑end (E2E) messaging that never relies on the public Tor network; additionally, apps that need **map tiles and other public assets** should be able to fetch them **through the RustyOnions network** (with caching/offline options), not directly from the clearnet.

**What I proposed:** Run a tiny, reusable **private onion runtime service** (**`svc-oniond`**) as the transport layer, consumed by both **Micronode** (single‑tenant, SDK‑first) and **Omnigate** (multi‑tenant, pro). Keep **E2E** as a **separate library + service** (e.g., `ron-e2e` + `svc-mailbox`) so both products share one audited crypto core. Add a new **Public Asset Plane** service (**`svc-edge`**) that fetches, validates, and **caches whitelisted public assets** (e.g., map tiles), with strict ToS compliance and optional **offline tile packs**.

**In one line:** A **hermetic, app‑scoped onion transport** + **shared E2E core** + **edge cache for public assets**—consumable by Micronode and Omnigate.

---

# Core Principles

* **Isolation by default:** Per‑app onions, keys, caps; no cross‑app traffic mixing.
* **Hermetic tests:** No dependency on public Tor; reproducible E2E.
* **Opaque E2E:** Servers enforce quotas/backpressure but never decrypt APP\_E2E payloads.
* **Least privilege:** Separate caps for onion control, mailbox use, and asset fetching.
* **Offline‑first:** Asset plane supports pre‑baked packs (tiles/fonts/icons) for no‑network runs.
* **Auditability:** Transport, crypto, and policy separated into small, testable components.

---

# Components (High Level)

1. **`svc-oniond` (Private Onion Runtime)**

   * Hosts per‑app private onion services; exposes a control API (UDS/TCP) for CREATE/BIND/STATUS/ROTATE.
   * Backends: `binary-tor` (ControlPort) or `arti` (when ready). Amnesia mode supported.
   * Security: cookie or macaroon auth; onion keys never leave unless cap‑gated export.

2. **E2E Layer**

   * **Library:** `ron-e2e` (envelope/session, `seal/open`, nonce/limits, log redaction helpers).
   * **Adapter:** `ron-e2e-tor` (optional helpers for onion addressing).
   * **Service:** `svc-mailbox` (store‑and‑forward; at‑least‑once delivery; ACK/visibility timeout). Treats payload bytes as opaque.

3. **Service Planes**

   * **Micronode (Starter, single‑tenant):** Can **embed** `svc-oniond` for 1‑command DX; ships opinionated defaults, SDK quickstarts.
   * **Omnigate (Pro, multi‑tenant):** Always **uses external** `svc-oniond`; adds quotas/policies/tenancy and governance.

4. **Public Asset Plane** — **`svc-edge`**

   * A fetch‑validate‑cache service for **whitelisted public assets** (e.g., map tiles, fonts, icons, JS/CSS).
   * Modes:

     * **Live Proxy Mode:** Enforced domain allow‑list; inject provider tokens from secrets; honor ETag/Last‑Modified; respect provider ToS/rate limits.
     * **Offline Packs Mode:** Serve **prebaked PMTiles/MBTiles** (or asset bundles) from local storage; deterministic, zero egress.
   * Storage: content‑addressed cache (dedupe) with TTL + refresh policy; metrics on hit/miss/latency.
   * Exposure: onion endpoints (`/tiles/{z}/{x}/{y}.{ext}`, `/assets/...`) bound via `svc-oniond`.

5. **Existing Core Services** (as needed)

   * `svc-storage` (blob store) + `svc-index` (content index) + **Gateway** (HTTP/OAP/1 surface) remain available for CAS, metrics, readyz.

---

# Data Flows

## A) E2E Message (opaque to the platform)

1. **Client SDK** (`ron-app-sdk`) calls `e2e::seal(pubkey, bytes)` → `Ciphertext`.
2. Send via OAP/1 to **`svc-mailbox`** over local TCP; **`svc-oniond`** exposes it on a private onion to peers/consumers.
3. Receiver pulls from Mailbox, obtains ciphertext, calls `e2e::open(keypair, ciphertext)`.
4. **Servers never decrypt.** They only enforce size/flow/backpressure and return typed errors.

## B) Public Asset Fetch (map tiles)

1. App requests `/tiles/z/x/y.png` (or `/assets/...`) from **Gateway** (onion HTTP endpoint).
2. Gateway forwards to **\`svc-edge\`\`**.

   * **Cache hit:** return bytes from local CAS with correct headers/TTL.
   * **Cache miss (Live Mode):** `svc-edge` checks allow‑list + rate limits, **injects provider token**, fetches over HTTPS, validates ETag, stores in CAS, returns.
   * **Offline Mode:** `svc-edge` serves from **PMTiles/MBTiles** pack mounted locally; never goes to network.

---

## \$1

# Review Enhancements (from Grok)

* **Scalability:** Dynamic onion scaling hooks in `svc-oniond`; autoscale by circuit count/latency.
* **Observability:** End‑to‑end tracing (OpenTelemetry) across Gateway → Mailbox → Edge without payload visibility.
* **Resilience:** Circuit breakers, anti‑DoS (proof‑of‑work on hot paths), exponential backoff in SDKs.
* **Federation (opt‑in):** `svc-oniond` peering API for trusted app networks; mutual caps.
* **Crypto:** Hybrid X25519+Kyber option in `ron-e2e`; signed receipts.
* **Assets:** SRI integrity checks in `svc-edge`; adaptive caching + prefetch hints; offline PMTiles packs.

\$2

* Lock names: **Micronode**, **Omnigate**, **svc-oniond**, **svc-mailbox**, **svc-edge**.
* Create skeleton crates + configs.
* Add a “Hello Map” sample that loads tiles via `svc-edge` and sends an E2E ping via `svc-mailbox` over the private onion.
* Wire CI checks: clippy, fmt, deny, unit tests for each service; smoke test: Micronode one‑command run.

---

# God‑Tier Enhancements (Applied)

**Status:** Integrated Grok’s hardening guidance without changing the token/economy model.

## Scalability & Federation

* **Dynamic scaling hooks** in `svc-oniond`: autoscale by circuits, queue depth, or p95 connect latency; graceful drain during downscale.
* **Opt‑in federation**: `svc-oniond` can peer private onions under mutual caps; default OFF. Peers exchange only transport metadata via a minimal handshake; no content introspection.
* **Geo‑aware routing (optional)**: hint-driven circuit selection for latency, without cross‑app sharing.

## Observability

* **End‑to‑end tracing** (OpenTelemetry): Gateway → Mailbox → Edge spans with correlation IDs; strict redaction.
* **Golden metrics (extended)**: onion setup p95, circuit churn rate, mailbox visibility timeouts, edge cache hit ratio per domain.

## Resilience & Anti‑DoS

* **Circuit breakers** on Mailbox SEND/RECV and Edge fetch; backoff + `Retry‑After`.
* **Proof‑of‑Work (PoW)** gate (configurable) on bursty endpoints to deter abuse.
* **SDK retries with full‑jitter exponential backoff** to avoid thundering herds.
* **Dead‑letter queues** for undeliverable messages with admin purge/requeue.

## E2E (Security Upgrades)

* **PQ‑hybrid option**: X25519+Kyber KEM in `ron-e2e` behind a feature flag; default classical for MVP.
* **Multi‑device support**: envelope supports multiple recipient keys; optional per‑device ratchet.
* **Strict size/limit enforcement** post‑decompress; constant‑time checks for headers.

## Public Assets (Integrity + Performance)

* **SRI integrity** (optional) for assets with known hashes.
* **Adaptive caching** in `svc-edge`: negative caching, prefetch hints, and pack‑first hybrid (serve PMTiles/MBTiles first, live as fallback).
* **Strict allow‑lists** with wildcard support and per‑domain rate limiting; provider tokens injected only for allowed hosts.

## Threat Model & Compliance

* **STRIDE baseline** recorded; mitigation notes for spoofing (caps), DoS (PoW/circuit breakers), info disclosure (redaction), elevation (RBAC on admin APIs).
* **Runtime attestation (optional)** for key handling modules; verifiable builds (cargo‑vet) recommended.

## Benchmarks & SLOs

* **Target SLOs (MVP)**: private onion bring‑up < 5s p95; mailbox enqueue/dequeue p95 < 50ms (local); edge cache hit p95 < 40ms.
* **Load goals**: sustain \~1k msgs/s and \~500 tile req/s per node with graceful degradation.

---

# Updated Config Sketches (Supersedes previous)

## Micronode (`micronode.toml`)

```toml
[onion]
backend = "binary-tor"   # or "arti"
embedded = true           # embed svc-oniond for local dev
amnesia = true            # tmp data dir during dev
federate_peers = []       # opt-in; empty by default

[mailbox]
listen = "127.0.0.1:9410"
max_inflight = 128
visibility_timeout_ms = 30000
ack_deadline_ms = 15000
# Dead-letter queue for undeliverables
dead_letter_dir = "/var/lib/ron/dlq"

[e2e]
# Security profile for dev; enable PQ when ready
pq_hybrid = true

[edge]
mode = "live"            # "live" | "offline" | "hybrid"
allow = ["tile.example.com", "api.maptiler.com"]
cache_dir = "/var/lib/ron/edge-cache"
integrity_sri = true
adaptive_cache = true
# Offline packs for deterministic runs
# packs = ["/data/tiles/region.pmtiles"]
# Prefetch hints for warm cache
prefetch = ["tiles/region/us"]

[gateway]
bind = "127.0.0.1:9080"
expose = [
  { local = "127.0.0.1:9080", onion_port = 80 },
  { local = "127.0.0.1:9410", onion_port = 9410 }
]

[observability]
otel_endpoint = ""   # e.g., http://127.0.0.1:4317

[dos]
proof_of_work = false
```

## Omnigate (`omnigate.toml`)

```toml
[oniond]
addr = "unix:///run/ron/oniond.sock"
scale_min = 1
scale_max = 10
# Autoscale based on circuit count and connect latency
autoscale = { circuits_p95 = 200, connect_ms_p95 = 1200 }

[tenancy]
# tenant/app profiles with caps/quotas (auth only; no economy changes here)

[edge]
allow = ["*.your-org-tiles.com", "fonts.gstatic.com"]
integrity_checks = true
rate_limit_rps = 50
rate_limit_burst = 100
cache_ttl_default = "24h"

[observability]
otel_endpoint = ""   # collector endpoint

[dos]
proof_of_work = true
pow_difficulty = 20  # tuned per env

[security]
attestation = false  # optional TEE attestation for key handling
```

---

# Perfection Pass — Final Refinements (v1.0)

Incorporates Grok’s final critiques while keeping the economics model out of scope. These changes set **PQ‑hybrid as default**, harden federation, automate PoW tuning, add QoS for messaging, and expand interfaces for real‑time and low‑latency ops.

## Final Design Decisions

* **PQ‑hybrid default ON** for `ron-e2e` (X25519+Kyber KEM). Classical‑only remains as a feature flag for constrained builds.
* **Federation = OFF by default**; when enabled, uses **ZK‑augmented handshakes** (challenge‑response with zk‑proof of cap possession) to minimize metadata leak. Policy remains opt‑in + mutual allow‑lists.
* **Automated PoW tuning** on hot endpoints (Mailbox SEND/RECV, Edge live fetch): difficulty adjusts to target p95 CPU budget; safe‑mode caps prevent runaway difficulty.
* **QoS for Mailbox**: 3 priority levels with fair‑queuing; starvation guards; admin‑settable per cap/app.
* **Realtime & Low‑latency I/O**: internal **gRPC** for service‑to‑service control paths; **WebSocket** subscriptions for Mailbox topics.
* **Config validation CLI** and **YAML support** (in addition to TOML) with schema checks.
* **Chaos & fuzzing** requirements baked into DoD.

## API Additions (non‑breaking)

* `svc-oniond`: `POST /federate/zk_handshake {peer_onion, proof}` → `{ok}`; `GET /pow/autotune` → current target; `POST /pow/autotune {on|off}`.
* `svc-mailbox`: `GET /ws/subscribe?topic=…` (WebSocket); `POST /qos {topic, priority}`; `GET /stats/qos`.
* `svc-edge`: `POST /prefetch {prefix}` (unchanged); `GET /sri/{hash}` (unchanged); `GET /rate/domain` for per‑domain limits.

## Config Deltas

### Micronode (`micronode.toml`)

```toml
[e2e]
pq_hybrid = true   # now default

[mailbox]
priority_levels = 3
qos_default = "normal"   # "low" | "normal" | "high"
ws_listen = "127.0.0.1:9420"  # WebSocket for subscriptions

[dos]
proof_of_work = true
pow_auto_tune = true
pow_target_ms = 3       # target CPU ms per challenge
pow_upper_bound = 28    # max difficulty

[federation]
enabled = false
zk_handshake = true     # enforced when enabled
allow = []              # explicit peer list

[api]
grpc_addr = "127.0.0.1:9095"
```

### Omnigate (`omnigate.toml`)

```toml
[dos]
proof_of_work = true
pow_auto_tune = true
pow_target_ms = 5
pow_upper_bound = 30

[mailbox]
priority_levels = 3
qos_default = "normal"
ws_listen = "127.0.0.1:9420"

[federation]
enabled = false
zk_handshake = true
allow = []

[api]
grpc_addr = "127.0.0.1:9095"
```

## Validation & Testing

* **Config linting**: `ronctl validate --file micronode.toml` (TOML) or `ronctl validate --file micronode.yaml` (YAML). Fails on unknown keys, type mismatches, or insecure combos (e.g., federation enabled without zk\_handshake).
* **Chaos tests**: packet loss, delayed ACKs, circuit churn; expect SLO adherence and graceful degradation.
* **Fuzzing**: `ron-e2e` envelope parsing; `svc-mailbox` frame handling; `svc-edge` URL/headers.

## SLOs (confirmed)

* **Onion bring‑up** < 5s p95; **Mailbox** enqueue/dequeue < 50ms p95 (local); **Edge** cache hit < 40ms p95; **WS subscribe** handshake < 150ms p95.

## Notes

* Arti backend tracked for parity; binary‑tor remains default until private‑net feature parity is proven in our tests.
* Federation remains an **advanced** capability; disabled by default and guarded with ZK proofs and mutual allow‑lists.


**What changed (high-impact):**

* **PQ-hybrid is now the default** for `ron-e2e` (X25519+Kyber). Classical-only stays behind a feature flag.
* **Federation stays OFF by default** and now uses a **ZK-augmented handshake** when enabled (mutual allow-lists + zero-knowledge proof of cap possession).
* **Automated PoW tuning** on hot paths (Mailbox SEND/RECV, Edge live fetch) with target CPU budget and safe caps.
* **Mailbox QoS**: 3 priority levels, fair-queuing, starvation guards; admin-settable per app/cap.
* **Realtime I/O**: added **WebSocket** topic subscriptions and **gRPC** internal control channel.
* **Config validation CLI** + **YAML support** (alongside TOML), with schema checks.
* **Chaos + fuzzing** are now part of DoD; SLOs reaffirmed.

**Configs updated in-canvas:**

* `micronode.toml`: `pq_hybrid = true`, QoS + WS + auto-tuned PoW, federation zk settings, `grpc_addr`.
* `omnigate.toml`: autoscale + QoS + WS + auto-tuned PoW, federation zk, `grpc_addr`.

If you want next, I can generate **paste-ready skeleton crates** for:

* `svc-oniond` (binary-tor backend, amnesia, autoscale hooks),
* `svc-mailbox` (SEND/RECV/ACK, visibility timeout, DLQ, QoS, WS, OTEL),
* `svc-edge` (allow-list proxy/cache, SRI, PMTiles offline, adaptive cache).
