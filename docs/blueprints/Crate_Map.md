
# RustyOnions – Crate Map (Core + Services + Nodes + SDK)

**Date:** 2025-09-14
**Architecture style:** Microkernel + plane separation

* **Node plane:** peer/overlay side (Gateway, Overlay, Index, Storage, Crypto).
* **App plane:** developer/app side (Omnigate, Micronode, special nodes).
* **Core:** ron-kernel only (IPC, supervision, health/metrics surface).
* **Security libs (small crates):** ron-proto, ron-auth, ron-kms, ron-policy, ron-audit, ryker.
* **Transports:** pluggable behind a single trait; TLS type fixed by kernel.

---

## Topology at a glance

```
 (Peers)                                  (Apps/Sites/SDKs)
    |                                           |
┌───▼──────┐                              ┌─────▼──────┐
│ svc-gateway │  (Node ingress)           │ svc-omnigate │  (App BFF/API)
└─┬───────┬─┘                              └──┬───────┬──┘
  │ OAP/1 │                                     │ REST/GraphQL/WS
  │       │                                     │
  │   ┌───▼────────────┐         ┌──────────────▼───────────┐
  │   │  ron-kernel     │  Bus   │        Micronode(s)       │
  │   │ (Bus, Health)   ├────────┤   (per-tenant edge runt.) │
  │   └───┬───────┬────┘         └───────────┬───────────────┘
  │       │       │                          │
┌─▼───┐ ┌─▼────┐ ┌▼────────┐            ┌────▼────┐    ┌──────────┐
│Index│ │Overlay│ │Storage  │            │ map-node │    │ cdn-node │
└─────┘ └──────┘ └─────────┘            └─────────┘    └──────────┘

       (All cross-service DTOs/errors: ron-proto; auth envelopes: ron-auth;
       quotas: ron-policy; keys: ron-kms; audit: ron-audit; actor/supervisor: ryker)
```

---

# Index of crates

| Crate                 | Role                                                             | Plane    | Kind    | Depends on                                           | Key Outputs                                                      |
| --------------------- | ---------------------------------------------------------------- | -------- | ------- | ---------------------------------------------------- | ---------------------------------------------------------------- |
| **ron-kernel**        | Microkernel: Bus, Health/Ready, Metrics, Config, Ctrl-C          | Core     | lib     | (axum/tokio/prometheus), **no app crypto/DB**        | IPC + observability surfaces                                     |
| **ron-proto**         | Single source of truth for DTOs/errors, addressing, OAP/1 consts | Shared   | lib     | serde, rmp-serde, blake3                             | `Error`, `B3Digest`, `OAP1_MAX_FRAME=1MiB`, `STREAM_CHUNK=64KiB` |
| **ron-auth**          | Zero-trust envelopes + HMAC verify                               | Shared   | lib     | hmac/sha2, uuid, smallvec                            | `Envelope<H,P>`, `verify_envelope()`                             |
| **ron-kms**           | KMS trait + dev backend (sealed storage, HKDF)                   | Shared   | lib     | hkdf/sha2, zeroize                                   | `Kms` trait, `derive_origin_key()`                               |
| **ron-policy**        | Token bucket quotas (peer/app)                                   | Shared   | lib     | time                                                 | `TokenBucket`                                                    |
| **ron-audit**         | Tamper-evident audit (Ed25519 + chain)                           | Shared   | lib     | ed25519-dalek, blake3                                | `Auditor::append()`                                              |
| **ryker**             | Actor/supervisor: bounded queues, jittered restarts              | Shared   | lib     | tokio, tracing                                       | `Supervisor::spawn_supervised()`                                 |
| **transport**         | Transport trait + wiring                                         | Shared   | lib     | tokio, rustls (via tokio-rustls)                     | `Transport` trait, `spawn_transport()`                           |
| **arti-transport**    | Tor/Arti backend impl                                            | Node     | lib     | transport, arti                                      | `Transport for Tor`                                              |
| **tcp-transport**     | Plain TCP backend impl                                           | Node     | lib     | transport                                            | `Transport for TCP`                                              |
| **quic-transport**    | QUIC backend impl (future)                                       | Node     | lib     | transport, quinn                                     | `Transport for QUIC`                                             |
| **svc-gateway**       | Node ingress (peer OAP/1 + admin ops)                            | Node     | service | kernel, proto, auth, policy, transport, ryker, audit | Quotas/DoS, peer auth, admin APIs                                |
| **svc-overlay**       | Routing/forwarding service                                       | Node     | service | kernel, proto, auth, policy, ryker                   | Route reqs, stream 64KiB chunks                                  |
| **svc-index**         | Provider/manifest index, metadata verify                         | Node     | service | kernel, proto, auth, ryker                           | Resolve/announce, signed metadata                                |
| **svc-storage**       | Content storage/DAG + (optional) repair                          | Node     | service | kernel, proto, ryker, policy                         | Get/put blobs, erasure hooks                                     |
| **svc-crypto**        | Centralized crypto ops, key sealing                              | Node     | service | kernel, proto, kms, ryker                            | Sign/verify, seal/unseal, rotate                                 |
| **svc-omnigate**      | App BFF/API (REST/GraphQL/WS), per-tenant quotas                 | App      | service | kernel, proto, auth, policy, ryker, audit            | App APIs, tenant auth, compose data                              |
| **micronode**         | Per-tenant edge/runtime (offline-first client)                   | App/Edge | lib/bin | proto, auth (client), policy                         | Local cache + sync, tenant APIs                                  |
| **map-node**          | Static **map** asset node (vector/tiles cache)                   | App/Edge | service | kernel, proto, policy, ryker, audit                  | OSM/vtile cache, CDN hints                                       |
| **cdn-node**          | Public asset relay/CDN node (“for hire”)                         | App/Edge | service | kernel, proto, policy, ryker, audit                  | Signed cache/relay, billing hooks                                |
| **ron-app-sdk**       | Developer SDK (typed calls to Omnigate)                          | App      | lib     | proto                                                | Strongly-typed client for apps                                   |
| **tldctl**            | CLI/operator tool (bus/admin via gateway)                        | Ops      | bin     | proto, auth                                          | Operate node, inspect status                                     |
| **node**              | Compositor binary (kernel + selected services)                   | Deploy   | bin     | kernel + selected svcs                               | Launch profile (dev/prod/hardened)                               |
| **svc-observability** | Central metrics/health server (kernel delegates)                 | Shared   | service | kernel, ryker                                        | /metrics,/healthz,/readyz                                        |

> **Naming note:** retain “gateway” for node ingress; “omnigate” for app BFF; Micronode is single-tenant. Map/CDN nodes are app-plane **specialized edges**.

---

# Allowed-Imports Matrix (hard fence)

* **ron-kernel** may import: tokio, axum, prometheus, serde; **must not** import DB/crypto beyond TLS **type** (`tokio_rustls::rustls::ServerConfig`).
* **Services** must import DTOs only from **ron-proto**; auth envelope from **ron-auth**; quotas from **ron-policy**; keys via **ron-kms**; auditing via **ron-audit**; supervision via **ryker**.
* **Transport impls** must respect the kernel TLS type (no alternate TLS types).
* **SDK/CLI** **never** directly touch service internals/DB; talk via Omnigate/Gateway protocols only.

---

# Detailed crate roles

## Core

### ron-kernel (core/lib)

* **One-liner:** Tiny microkernel that provides lossy broadcast Bus, health/readiness/metrics HTTP, config & graceful shutdown, and fixes the TLS type for all transports.
* **Responsibilities:**

  1. IPC (broadcast bus) with bounded capacity; overflow → drop + metric + `KernelEvent::ServiceCrashed{reason="bus-overflow"}`.
  2. Observability (`/metrics`, `/healthz`, `/readyz`) and `HealthState`.
  3. Public API **re-exports**: `Bus`, `KernelEvent::{Health,ConfigUpdated,ServiceCrashed,Shutdown}`, `Metrics`, `HealthState`, `Config`, `wait_for_ctrl_c()`.
* **Non-goals:** No app logic; no DB; no hashing/signatures (payload-agnostic).
* **Connections:** All services depend on the Bus & health. Kernel may **delegate** observability to `svc-observability` when present.

## Security & policy libraries (small, auditable)

### ron-proto (lib)

* **Purpose:** Single source of truth for **all** cross-service DTOs, `Error`, addressing (`B3Digest`), and OAP constants.
* **Invariants:** `OAP1_MAX_FRAME = 1MiB`; `STREAM_CHUNK = 64KiB`. JSON-schema snapshots for DTOs in CI.

### ron-auth (lib)

* **Purpose:** Zero-trust **Envelope** for every cross-service message; HMAC-SHA256 over a canonical rmp-serde encoding; `iat/exp`, `scopes`, `nonce`.
* **Usage:** Verified at **every** Bus/UDS ingress; required scopes enforced by plane (Node/App).
* **Extensibility:** Reserve fields for macaroons/capabilities later.

### ron-kms (lib)

* **Purpose:** KMS trait with **origin key derivation** (HKDF over Node Master Key), **seal/unseal**, rotation.
* **Backends:** `MemKms` (dev), `osx-keychain`/`linux-keyring`/`pkcs11` (future).
* **Policy:** Default **24h rotation**; fire `KeyRotated` kernel events (no secrets).

### ron-policy (lib)

* **Purpose:** Token-bucket quotas used by Gateway (peer/IP keys) and Omnigate (tenant/API keys).
* **Metrics:** Provide standard labels so Prometheus dashboards line up.

### ron-audit (lib)

* **Purpose:** Signed, chained audit log with hourly checkpoints. Log security events (auth fail, admin ops, key rotations).
* **Backends:** In-memory; optional fs sink; future remote sink.

### ryker (lib)

* **Purpose:** Unify the concurrency story: supervised spawns with bounded mpsc, jittered backoff, graceful shutdown.

## Transports

### transport (lib)

* **Purpose:** Trait + `spawn_transport(cfg, metrics, health, bus, tls_override)`; **TLS type** = `tokio_rustls::rustls::ServerConfig`.
* **Impls:** `arti-transport` (Tor/HS), `tcp-transport`, `quic-transport` (future).
* **Config:** timeouts, idle, max conns; backpressure reporting.

## Node plane services

### svc-gateway (service)

* **Role:** **Only** ingress for node/peer traffic + operator admin API.
* **Duties:**

  * Terminate OAP/1; authenticate peers; apply peer quotas (ron-policy).
  * Admin/control (drain, shutdown, snap) behind operator auth.
  * Publish bus events; verify envelopes on ingress.
* **Not:** no tenant UX or app session logic.

### svc-overlay (service)

* **Role:** Route/forward content; maintain in-memory routing table; chunked streaming (64KiB).
* **Security:** envelope scopes like `overlay:route` required.

### svc-index (service)

* **Role:** Provider announcements, lookups, metadata signature verification.
* **Data:** persistent store (sled/rocks trait behind an interface).
* **Future:** optional `svc-dht` extraction if Kademlia gets big.

### svc-storage (service)

* **Role:** Get/put for content DAG; optional background repair with pacing (e.g., 50MiB/s).
* **Security:** local sealing of sensitive blobs (rare); normal content is public but integrity is BLAKE3.

### svc-crypto (service)

* **Role:** Central signing/verification + sealing; integrates with ron-kms; rotates keys; provides `sign_manifest`, `verify_manifest`.

## App plane services & runtimes

### svc-omnigate (service)

* **Role:** App/API BFF. REST/GraphQL/WS; per-tenant quotas; session/auth (OAuth/OIDC/JWT, API keys).
* **Behavior:** Composes responses by fanning out to Index/Overlay/Storage; returns typed DTOs (ron-proto).
* **Security:** Enforce **app scopes**; issue **short-lived** tokens for Micronode.

### micronode (lib/bin)

* **Role:** Single-tenant runtime (edge-adjacent or embedded) for offline-first reads and local caching.
* **Behavior:** Sync with Omnigate; use app tokens; offer local APIs to the app.
* **Security:** Min secrets; sealed at rest; periodic purge (amnesia mode optional).

### map-node (service) — **Special App/Edge node**

* **Role:** Serve **static map assets** (vector tiles, styles, sprites, fonts); cache upstream OSM/open tiles; optionally pre-seeded packs.
* **Why separate:** To keep ToS and cache semantics isolated; avoid coupling map logic into Omnigate.
* **Interfaces:**

  * Public **HTTP** GET for tiles/styles;
  * Admin API for seeding/ttl/prefetch regions;
  * Bus listener for warm-up events (e.g., “hot area” hints).
* **Policy:** Strong rate limits; referrer checks; per-tenant cache partitions; opt-in **privacy filters** (no IP logging in amnesia mode).

### cdn-node (service) — **Special App/Edge node**

* **Role:** **For-hire public asset relay** (images, scripts, fonts, app bundles) with signed caching, quotas, and optional billing hooks.
* **Why separate:** Clear commercial boundary, auditable, scalable independently.
* **Interfaces:**

  * Public **HTTP** GET;
  * Signed URL support (HMAC or Ed25519 URLs);
  * Reporting API (usage exports).
* **Security/policy:**

  * Tenant allowlist + path constraints;
  * Per-tenant token buckets;
  * Abuse detection (sudden spikes → auto throttle);
  * **No exit** semantics (relay only whitelisted sources);
  * Optional watermarking for images (feature-gated).
* **Billing (future):** Emit signed usage to a settlement queue; not in kernel.

## SDK, CLI, Compose

### ron-app-sdk (lib)

* **Role:** Typed SDK for apps to call Omnigate + Micronode; hides DTO details; retries with budgets; observability hooks.

### tldctl (bin)

* **Role:** Operator CLI; connects to Gateway admin and kernel health; “no direct DB”.

### node (bin)

* **Role:** The process that **assembles** the chosen services by profile:

  * **Dev:** kernel + (gateway, overlay, index, storage, omnigate) in-proc (fast), `MemKms`.
  * **Hardened:** split processes via UDS for omnigate/storage; sandbox; OS KMS.
  * **Edge:** map-node/cdn-node deployed separately, registered via bus.

### svc-observability (service)

* **Role:** Central metrics/health server; kernel can **delegate** to this; hosts `/metrics`, `/healthz`, `/readyz`, plus dashboards.

---

# Connections & contracts (who talks to whom)

* **All cross-service calls** carry `ron-auth::Envelope` and are **verified** at the receiving boundary.
* **Gateway ↔ Overlay/Index/Storage:** OAP/1 frames → bus DTOs; overlay streams 64KiB chunks; storage returns content; index provides metadata.
* **Omnigate ↔ Index/Overlay/Storage:** app composite reads; tenant quotas check at Omnigate; Micronode tokens minted here.
* **Micronode ↔ Omnigate:** short-lived app tokens; background sync schedules.
* **Map-node, CDN-node:** registered as **edge services**; Omnigate can route app requests to them or hand out signed URLs.

---

# Security posture per plane

* **Node plane (Gateway, Overlay, Index, Storage, Crypto):**

  * **Peer** auth/quota at Gateway; envelope scopes required (`overlay:route`, `index:announce`, …).
  * In **hardened** profile: UDS + `SO_PEERCRED`, seccomp/cgroups, minimal caps.
* **App plane (Omnigate, Micronode, Map/CDN nodes):**

  * **Tenant** auth (JWT/API keys), per-tenant quotas; signed URLs for CDN; app scopes enforced at ingress.
  * Tokens short-lived; revoke/rotate policies via ron-kms.

**Audit:** Every auth failure, admin op, key event → `ron-audit` record (signed + chained).

---

# Config & feature flags (high level)

* **ron-kernel:** `METRICS_ADDR`, `READY_GATES` (optional), TLS type fixed (no feature to change).
* **transport:** timeouts, max conns, idle; `--features arti-transport/quic-transport` toggles backends.
* **ron-kms:** `dev-mem` (default for dev); `osx-keychain/linux-keyring` in prod; rotation period (env/Config).
* **svc-gateway:** peer quotas (policy JSON), admin credentials (sealed), OAP settings.
* **svc-omnigate:** tenant registry, JWT/OIDC config, per-tenant defaults, CORS.
* **map-node/cdn-node:** cache tiers, TTLs, upstream allowlists, signed-URL policy keys.
* **svc-observability:** port, scrape configs, dashboards (optional bundle).

---

# Observability (golden metrics everywhere)

* `request_latency_seconds{service,route,plane}`
* `rejected_total{service,reason}`
* `quota_drop_total{key}`
* `bus_overflow_dropped_total`
* **Health/readiness**: consistent; Omnigate **not Ready** until downstreams Ready and Micronode pool has capacity.

---

# Persistence & data model (quick pointers)

* **svc-index:** provider manifests keyed by content IDs; trait for storage (sled/rocks).
* **svc-storage:** content DAG (b3 hash keys, chunked 64KiB); optional erasure metadata.
* **Micronode:** small local cache (per-tenant namespace).
* **Map-node/CDN-node:** cache stores with per-tenant partitions and TTL; export usage counters periodically.

---

# Risks & fences (CI must enforce)

1. Kernel must not import DB or signature/hashing crates (beyond TLS types).
2. No `#[derive(Serialize, Deserialize)]` DTOs outside **ron-proto**.
3. No unbounded channels; no `unwrap/expect` in non-test.
4. Assert OAP/1 **1 MiB frame** vs **64 KiB chunk** never conflated.
5. No unscoped envelopes—unit tests reject `scopes: []`.
6. Cargo/audit + git-secrets + SLSA provenance in CI.

---

# Roadmap notes (security first)

* **Now:** land `ron-proto`, `ron-auth`, `ron-kms`, `ron-policy`, `ron-audit`, `ryker`. Switch services to envelopes + quotas.
* **Soon:** default UDS/sandbox for Omnigate/Storage in hardened profile; SO\_PEERCRED checks.
* **Next:** map-node & cdn-node MVP (HTTP GET, cache TTLs, signed URLs), with Omnigate routing.
* **Later:** PQ hybrid signatures in provider manifests; OAP/2 header reservation; capabilities/macaroons.

---

## Quick per-crate readiness table (0–5 subjective)

| Crate             | API | Tests | Obs | Config | Security | Perf | Coupling |
| ----------------- | --: | ----: | --: | -----: | -------: | ---: | -------: |
| ron-kernel        |   5 |     4 |   4 |      3 |        3 |    3 |    **2** |
| ron-proto         |   4 |     3 |   3 |      5 |        4 |    5 |        1 |
| ron-auth          |   4 |     3 |   3 |      4 |        5 |    4 |        1 |
| ron-kms           |   4 |     3 |   3 |      4 |        4 |    4 |        1 |
| ron-policy        |   4 |     4 |   3 |      4 |        4 |    5 |        1 |
| ron-audit         |   4 |     3 |   4 |      4 |        5 |    4 |        1 |
| ryker             |   4 |     3 |   3 |      4 |        4 |    4 |        1 |
| transport/\*      |   4 |     3 |   3 |      4 |        4 |    4 |        2 |
| svc-gateway       |   4 |     3 |   4 |      4 |        4 |    4 |        3 |
| svc-overlay       |   4 |     3 |   3 |      4 |        4 |    4 |        3 |
| svc-index         |   4 |     3 |   3 |      4 |        4 |    4 |        3 |
| svc-storage       |   4 |     3 |   3 |      4 |        4 |    4 |        3 |
| svc-crypto        |   4 |     3 |   3 |      4 |        5 |    4 |        3 |
| svc-omnigate      |   4 |     3 |   4 |      4 |        4 |    4 |        3 |
| micronode         |   3 |     3 |   3 |      4 |        4 |    4 |        2 |
| map-node          |   3 |     2 |   3 |      4 |        4 |    4 |        2 |
| cdn-node          |   3 |     2 |   3 |      4 |        4 |    4 |        2 |
| svc-observability |   4 |     3 |   5 |      4 |        4 |    5 |        2 |

> “Coupling” lower is better (kernel anchors the project low).

