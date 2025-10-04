---

title: Security Notes — svc-index
crate: svc-index
owner: Stevan White
last-reviewed: 2025-10-03
status: draft
-------------

# Security Documentation — `svc-index`

This document defines the **threat model**, **security boundaries**, and **hardening requirements** specific to `svc-index`.
It complements the repo-wide Hardening Blueprint and Interop Blueprint and is aligned with the crate’s IDB, CONFIG, and Concurrency docs. See also the Six Concerns mapping where `svc-index` is held to **SEC, PERF, RES, GOV** gates. 

---

## 1) Threat Model (STRIDE)

`svc-index` is a **thin, read-optimized resolver** that maps **names and `b3:<hex>` CIDs → manifests → provider sets**. It **does not** store blob bytes and **does not** implement DHT internals; all provider discovery flows through `svc-dht`. This scope shapes the threats and mitigations below. 

| Category                   | Threats                                                                    | Relevant in `svc-index`? | Mitigation                                                                                                                                                                                      |
| -------------------------- | -------------------------------------------------------------------------- | -----------------------: | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **S**poofing               | Fake client or peer identity; unauthenticated admin/facet calls            |                    **Y** | Capability tokens (macaroons) for admin & facets; optional UDS with **SO_PEERCRED** allow-list; TLS at ingress profile when public.                                                             |
| **T**ampering              | Mutated manifests or provider data; index DB corruption                    |                    **Y** | Content addressing with **BLAKE3**; verify full digest before returning any manifest-derived data; single “truth” DB root; structured rejects.                                                  |
| **R**epudiation            | Missing/auditable trails for admin mutations                               |                    **Y** | Emit audit events via `ron-audit` for every mutating admin path; structured JSON logs with correlation IDs.                                                                                     |
| **I**nformation Disclosure | Leaking capability secrets, PII in logs, or internal topology              |                    **Y** | Never log secrets; JSON logs only; amnesia mode (RAM-only) available; cautious CORS and tracing endpoints.                                                                                      |
| **D**enial of Service      | Floods, slow-loris, compression bombs, CPU spikes from search/graph facets |                    **Y** | Hard **timeouts**, **inflight caps**, **1 MiB** body cap, **≤10×** decompression ratio, per-IP connection cap, optional token-bucket rate limiting and global quotas; truthful **/readyz**.     |
| **E**levation of Privilege | Using unscoped creds to perform admin/facet ops                            |                    **Y** | Capability verification **before enqueue**; optional role allow-lists; fail-closed design for writes/administration.                                                                            |

> **Protocol & size invariants**: OAP/1 **max_frame = 1 MiB** (wire); storage **streaming chunk ≈ 64 KiB** (I/O guidance). Do not conflate the two. `svc-index` responses must respect these bounds.  

---

## 2) Security Boundaries

* **Inbound (typical HTTP/UDS API surface):**

  * `GET /resolve/{name|b3}` → manifest pointer (+ cache)
  * `GET /providers/{b3}` → ranked provider set (region/freshness/latency class)
  * `POST /admin/{task}` → controlled mutations (reindex, backfill, pin/unpin) → **audited** via `ron-audit`
  * `GET /healthz`, `GET /readyz`, `GET /metrics`, `GET /version`
    (Admin/facets require capabilities; writes **fail-closed** when degraded.)   

* **Outbound / Dependencies:**

  * **`svc-dht`** (provider discovery with α/β hedging, deadlines) 
  * **`ron-audit`** (mutations generate audit events) 
  * **Kernel** bus/metrics/health wiring; **`ron-metrics`** Prometheus surface (taxonomy incl. rejects/latency).  
  * **`ron-auth`/`svc-passport`** for capability verification (macaroons). 

* **Trust Zone / Profiles:**

  * **Micronode**: embedded, RAM-first (**amnesia on** by default).
  * **Macronode**: behind `svc-gateway` quotas/backpressure; `svc-index` may bind publicly if reviewed. Public binds require strict caps.   

* **Assumptions:**

  * Kernel invariants hold (Bus, Metrics, Health, Config).
  * Downstream services publish `/readyz` and enforce their own quotas.
  * DB/filesystem is **per-service** and isolated; **single `RON_INDEX_DB` root** configured.  

---

## 3) Key & Credential Handling

* **Types of secrets/keys in scope:**

  * **Capability tokens** (macaroons) for admin/facet access.
  * **TLS certificates/keys** (only if `svc-index` is directly public).
  * **UDS allow-lists** (UIDs) when `uds.enabled = true`.
  * Optional verifier/issuer integration via `ron-auth`/`svc-passport`.   

* **Storage & custody:**

  * Prefer sealed custody via **`ron-kms`**; **never log** token material; **amnesia** mode rejects persistent spill, purges on a schedule. 

* **Rotation & revocation:**

  * Rotate capability/token material **≤ 30 days**; document revocation path in ops runbooks (issuer-side). 

* **Zeroization:**

  * Wrap secrets in types that **zeroize on drop** (local policy; see Hardening defaults). 

---

## 4) Hardening Checklist (svc-index profile)

* [ ] **Timeouts**: 5s request default (read/write timeouts) and **end-to-end lookup budget** (~1200 ms) for α/β+DHT hedging.  
* [ ] **Concurrency**: inflight cap **512**; per-IP cap **256**.  
* [ ] **RPS/Admission**: token bucket rate-limit (default off) and optional global quotas enforced **before heavy work**.  
* [ ] **Body/Compression**: **1 MiB** body cap; decompression **≤ 10×** + absolute cap; reject 2 MiB self-test with **413**.  
* [ ] **UDS** (if enabled): directory **0700**, socket **0600**, **SO_PEERCRED** allow-list; CI check perms. 
* [ ] **Capability gating**: verify **before enqueue** (admission controller). 
* [ ] **Cache integrity**: verify **BLAKE3** CID before returning manifest-derived data. 
* [ ] **Truthful readiness**: `/readyz` returns 200 only after config, DB, DHT client, and bus are live. 
* [ ] **Amnesia mode**: RAM-only; scheduled purge; metrics label `amnesia_mode{status=…}` present. 
* [ ] **No unsafe**; **cargo-deny** green (licenses/advisories/bans). 

---

## 5) Observability for Security

* **Metrics (augment svc-index taxonomy):**

  * `rejected_total{reason="unauth"|"over_capacity"|"body_cap"|"decompress_cap"}`
  * `auth_failures_total{service="svc-index"}`
  * `index_resolve_latency_seconds{type}`; `index_dht_lookup_ms`
  * `index_cache_{hits,misses}_total`
  * `tls_handshake_failures_total` (if public TLS)
  * `amnesia_mode{status="on|off"}` (gauge)
    These align with the crate’s golden taxonomy and Six Concerns’ hardening gates.  

* **Logs:** Structured **JSON** only; include `service`, `route`, `status`, `reason`, `corr_id`, `peer_addr`; never log secrets. 

* **Health gates:** `/readyz` **fail-closed** when degraded; shed load transparently and surface reject reasons in metrics.  

---

## 6) Dependencies & Supply Chain

* **Security-sensitive deps (non-exhaustive):**
  `tokio-rustls` (TLS 1.3, if used), `ron-proto` (DTOs with `deny_unknown_fields`), `serde` (strict schemas), `rmp-serde`/CBOR (if enabled), `ron-auth` (capabilities), `svc-dht` client. 

* **Pinned versions:** workspace root controls pins; changes require CI and doc updates.

* **Controls:** `cargo-deny` in CI; **SBOM** emitted at release under `docs/sbom/`. 

---

## 7) Formal & Destructive Validation

* **Property tests:** name→cid→providers round-trip; DTOs enforce `serde(deny_unknown_fields)`. 
* **Fuzzing:** `b3:<hex>` parser; DTO (JSON/CBOR) fuzz targets. 
* **Loom/Concurrency:** assert **no lock/iterator across `.await`**; cancel-safety and shutdown joins.  
* **Chaos & Exhaustion:** OOM/FD exhaustion; DHT flaps; verify amnesia purge cadence. 
* **Hardening self-tests:** 2 MiB body → **413**; 600+ concurrent → inflight cap reject; `/readyz` blocks until DB+DHT+bus up. 
* **TLA+ (optional):** spec sketch for readiness DAG and admission/backpressure (ties into Six Concerns). 

---

---

## 8) Migration & Upgrades

* Changes to auth, capability formats, or key custody are **security-relevant** and require:

  * **CHANGELOG.md** entries + **major** semver bump if breaking.
  * A documented migration path and **EoS window** for deprecated keys/flags. 

---

## 09) Mermaid — Security Flow Diagram (REQUIRED)

```mermaid
flowchart LR
  subgraph Client/Caller
    X[SDK/Caller]
  end

  X -->|HTTP/UDS\ncapability (macaroon)| A(svc-index)
  A -->|resolve {name|b3}| B[Cache/DB]
  A -->|provider lookup (α/β+hedge)| D[[svc-dht]]
  A -.->|admin only| E[(ron-audit)]
  A -.->|metrics| M[(Prometheus)]

  A -.->|reject unauth/oversized/over-capacity| R[(Error + Metrics)]

  style A fill:#b91c1c,stroke:#7f1d1d,color:#fff
  style D fill:#334155,stroke:#0ea5e9,color:#fff
  style E fill:#334155,stroke:#22c55e,color:#fff
```

---

### Appendix A — Canonical Defaults (for quick reference)

* **OAP/1**: `max_frame = 1 MiB`; **storage stream ≈ 64 KiB**; DTOs deny-unknown. 
* **Hard limits**: 5s timeout; 512 inflight; 500 rps; 1 MiB body; **≤10×** decompress; **/metrics /healthz /readyz /version**. 
* **UDS**: dir **0700**, socket **0600**, **SO_PEERCRED** allow-list. 
* **Amnesia mode**: RAM-only caches; purge on cadence; metrics label present. 

> **Definition of Done (SEC, svc-index):** All checklist items are wired, capability gating is **before** work admission, rejects are observable, and `/readyz` is truthful under stress. 


