---

crate: svc-omnigate
path: crates/svc-omnigate
role: service
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
-------------------------

## 1) One-liner

Multi-tenant gateway that fronts RustyOnions services, enforcing quotas/backpressure, exposing health/metrics/readiness, and brokering app traffic over the OAP/1 protocol to storage/index/mailbox while staying app-agnostic.&#x20;

## 2) Primary Responsibilities

* Enforce per-tenant capacity (token buckets), overload discipline (429/503 + Retry-After), and capacity-aware `/readyz`.&#x20;
* Terminate the external surface (HTTP/OAP/1), route to internal services (index/storage/mailbox), and keep protocol limits distinct from storage chunking. &#x20;
* Export “golden metrics” for requests, bytes, latency, rejects/quotas, and inflight.&#x20;

## 3) Non-Goals

* No app-specific behavior or business logic (keep kernel/services neutral; use SDK + app\_proto\_id for app needs).&#x20;
* No PQ/QUIC/ZK changes to OAP/1 at this layer (future-tracks in OAP/2/R\&D).&#x20;

## 4) Public API Surface

* **Re-exports:** None required for consumers; this is a service binary exposing endpoints.
* **HTTP Endpoints (expected):**

  * `/healthz` (liveness), `/readyz` (capacity-aware readiness), `/metrics` (Prometheus). &#x20;
  * Object read path (via gateway surface proved by gwsmoke; stable ETag/Range/encoding semantics).&#x20;
* **Protocol:** OAP/1 framing and error mapping; HELLO advertises limits; max\_frame = 1 MiB (distinct from storage 64 KiB streaming). &#x20;
* **Error envelope (target):** `{code,message,retryable,corr_id}`; 400/404/413/429/503 + Retry-After.&#x20;

## 5) Dependencies & Coupling

* **Internal crates (via RPC/UDS/HTTP):**

  * `svc-index`, `svc-storage`, `svc-overlay` (read path; content-addressed GET/HAS/stream). Coupling: *loose* (over network/UDS); replaceable=yes.&#x20;
  * `svc-mailbox` (SEND/RECV/ACK; later DELETE/SUBSCRIBE/visibility). Coupling: *loose*; replaceable=yes.&#x20;
  * `ron-kernel` for service bus events & invariants; OAP/1 codec defined outside kernel (no app logic). Coupling: *loose*.&#x20;
* **External crates (expected by repo invariants):** Axum 0.7 (HTTP), Tokio (async), Prometheus client (metrics), tokio-rustls (TLS), Serde/rmp-serde (wire). Risk moderate (well-maintained); TLS uses rustls.&#x20;
* **Runtime services:** Network (HTTP/UDS), Storage (blob/index), OS (sockets), Crypto (caps/tokens; APP\_E2E opaque).&#x20;

## 6) Config & Feature Flags

* **Env & files:** `RON_QUOTA_PATH` (per-tenant rate config), `RON_NODE_URL` / `RON_CAP` for SDK/clients; capacity gating via `/readyz`. &#x20;
* **Service config (Omnigate):** TOML with DoS PoW toggles, QoS for mailbox (WS subscribe), optional federation (off by default, ZK handshake), gRPC control addr.&#x20;
* **Spec source control:** Local `/docs/specs/OAP-1.md` mirrors GMI-1.6 to prevent drift.&#x20;

## 7) Observability

* **Metrics (golden set):** `requests_total{code}`, `bytes_{in,out}_total`, `latency_seconds`, `rejected_total{reason}`, `inflight`, `quota_exhaustions_total` (and cache/range counters when serving objects). &#x20;
* **Health/Readiness:** `/healthz` + capacity-aware `/readyz` that gates load.&#x20;
* **Tracing:** Correlation IDs end-to-end; SDK propagates `corr_id`.&#x20;

## 8) Concurrency Model

* **Tasks:** HTTP acceptor; per-tenant token-bucket middleware; backend client pools for index/storage/mailbox; metrics exporter loop.
* **Backpressure/overload:** Token buckets; 429/503 with Retry-After; bounded inflight; compression guardrails to cap decompressed size/ratio.  &#x20;
* **Timeouts/retries:** Error taxonomy + SDK jittered retries respecting Retry-After.&#x20;

## 9) Persistence & Data Model

* **Omnigate:** Stateless beyond in-memory counters/quotas/metrics. Durable content and messages live in backing services (index/storage/mailbox). Read path ensures 64 KiB streaming and BLAKE3 verification in storage.&#x20;

## 10) Errors & Security

* **Error taxonomy:** Canonical JSON envelope; map 400/404/413 (decompress/ratio caps), 429 (quota), 503 (capacity). Include `corr_id` and `Retry-After` where applicable.&#x20;
* **AuthN/Z:** Capability tokens/macaroons managed outside kernel; Omnigate enforces quotas per tenant and optional PoW on hot paths. &#x20;
* **TLS:** rustls stack per project invariant; APP\_E2E payloads remain opaque to services.&#x20;
* **PQ-readiness (road-map):** PQ-hybrid in E2E layer, not altering OAP/1; federation guarded with ZK handshake when enabled. &#x20;

## 11) Performance Notes

* **SLO targets (dev-box/intra-AZ):** p50 < 10 ms, p95 < 40 ms, p99 < 120 ms for service plane; mailbox p95 < 50 ms local; cache hit p95 < 40 ms. &#x20;

## 12) Tests

* **Status:** gwsmoke shows Gateway→Index→Overlay→Storage read path working (ETag/Range/encoding behaviors validated); health/ready/metrics exist; Bronze ring needs golden metrics + red-team. &#x20;
* **Planned/required:** OAP parser proptests + fuzz corpus; compression bomb suite; golden error tests; SDK retry tests. &#x20;

## 13) Improvement Opportunities

* **Close Bronze:** Ship golden metrics, JSON error envelope, and capacity `/readyz` invariants with tests; finalize Mailbox ops (DELETE/SUBSCRIBE/visibility).&#x20;
* **Config hygiene:** Externalize quotas to `RON_QUOTA_PATH`; document default tenant fallbacks.&#x20;
* **Drift guards:** Keep OAP/1 spec a stub that mirrors GMI-1.6; add CI greps for max\_frame vs 64 KiB chunking and BLAKE3 addressing. &#x20;
* **Redundancy watch:** If a legacy `gateway` crate exists, de-duplicate roles with `svc-omnigate` (same surface/metrics/quotas) or merge under one name to avoid split ownership.&#x20;

## 14) Change Log (recent)

* **2025-09-05** — Project plan sets Bronze focus: quotas/readyz, golden metrics, red-team, error taxonomy; gwsmoke verified read path. &#x20;
* **2025-09-01** — Interop blueprint (GMI-1.6) finalized; OAP/1 spec is authoritative reference for Omnigate surface.&#x20;

## 15) Readiness Score (0–5 each)

* **API clarity:** 3.5 (OAP/1/HELLO and HTTP surfaces are clear; error envelope pending finalization).&#x20;
* **Test coverage:** 3 (gwsmoke + read-path checks; need fuzz/property/bronze red-team).&#x20;
* **Observability:** 3.5 (health/readyz live; golden metrics partially listed but not all landed).&#x20;
* **Config hygiene:** 3 (quota file & SDK env planned; Omnigate TOML templates exist). &#x20;
* **Security posture:** 3 (caps/PoW hooks and compression guards planned; federation off by default; APP\_E2E opaque). &#x20;
* **Performance confidence:** 3 (SLOs defined; read path proven locally; storage streaming targets identified).&#x20;
* **Coupling (lower is better):** 3 (loose to services via RPC/UDS; shared invariants prevent drift).&#x20;

