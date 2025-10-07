

````markdown
---
title: Omnigate — Invariant-Driven Blueprint (IDB)
version: 1.0.0
status: reviewed
last-updated: 2025-10-06
audience: contributors, ops, auditors
---

> **Alias Notice:** *Omnigate* is the architectural **contract** (invariants, SLOs, acceptance gates).
> The **production implementation** lives in **`crates/svc-gateway`**; all gates below apply to that crate.

# Omnigate — IDB

## 1. Invariants (MUST)

- [I-1] **Edge role / stateless BFF:** Acts as the unified north–south ingress (TLS termination, quotas, DRR fair-queueing, HTTP↔OAP brokering). **Stateless** beyond counters/telemetry; no business logic.
- [I-2] **OAP/1 bounds:** Enforce protocol limits at the edge: `max_frame = 1 MiB`; streaming body chunks ≈ **64 KiB**; oversize requests respond **413** with structured error.
- [I-3] **Content addressing:** Never serve bytes without verifying content-address (`ETag`/addr form like `b3:<hex>`). Do not hash framed envelopes themselves.
- [I-4] **Readiness semantics:** `/readyz` must prefer **read availability** while **failing writes first** under pressure (shed-before-collapse). Expose degradation reasons.
- [I-5] **Backpressure visibility:** DRR class queues (per tenant/capability) and drop/reject reasons are observable (golden metrics + labels).
- [I-6] **Capability-first mutation:** No ambient trust. Every mutating call presents a capability (e.g., macaroon/token) that is validated and propagated downstream.
- [I-7] **Amnesia honor (Micronode):** With amnesia ON, no hidden disk spill of request/response bodies or keys. Only sanctioned ephemera (counters/metrics) persist.
- [I-8] **Pillar boundary:** Omnigate remains within Pillar-6 (Ingress & Edge). No storage/index/DHT/transport loops implemented here; delegate to owners.
- [I-9] **Facet SLOs (read-heavy paths, intra-AZ):**
  - Graph/Feed hydration (single object + bounded neighbor fan-out): **p95 ≤ 150 ms**, payload ≤ **256 KiB**.
  - Media byte-range (≤128 KiB window): **p95 ≤ 100 ms**.
  - Search autocomplete (≤10 suggestions): **p95 ≤ 75 ms**.
  SLOs apply to ingress composition latency; per-hop latencies must be attributed, not aggregated away.
- [I-10] **Decompression guard:** Enforce absolute body cap and **bounded decompression ratio ≤10×**; reject when exceeded with structured **413**.

## 2. Design Principles (SHOULD)

- [P-1] **Thin composition:** Hydrate client views by *composing* downstream services (index/storage/mailbox), never caching domain objects locally.
- [P-2] **Early admission control:** Apply auth/quotas/body caps **before** heavy work (parsing, decompression, upstream dials).
- [P-3] **Graceful degrade:** Prefer partial availability (reads) during overload; return explicit reject reasons `{quota|degraded|oversize|timeout}`.
- [P-4] **DTO hygiene:** External JSON DTOs deny unknown fields; shared wire types live in `ron-proto`.
- [P-5] **Transport neutrality:** Posture and metadata only; Tor/QUIC/mTLS live in `ron-transport`. No transport loops in Omnigate.
- [P-6] **Facet composition without state:** Compose Graph/Feed/Search/Media with per-hop budgets and explicit timeouts; no ranking or per-tenant in-process caches.

## 3. Implementation (HOW)

> The following are copy-paste-or-adapt patterns used in `svc-gateway`.

- [C-1] **Axum/Tower hardening sketch**
  ```rust
  // Apply early caps; values align with invariants.
  use tower::{ServiceBuilder, limit::ConcurrencyLimitLayer, timeout::TimeoutLayer};
  use tower_http::limit::RequestBodyLimitLayer;
  use std::time::Duration;

  let svc = ServiceBuilder::new()
      .layer(ConcurrencyLimitLayer::new(512))                 // ingress concurrency cap
      .layer(TimeoutLayer::new(Duration::from_secs(5)))      // end-to-end request timeout
      .layer(RequestBodyLimitLayer::new(1 * 1024 * 1024))    // 1 MiB body cap
      .into_inner();
````

* [C-2] **Fair-queue DRR classification (tenant/capability)**

  ```rust
  // Pseudocode: classify into queue classes; export per-class gauges.
  enum Class { Free, Pro, System }
  fn classify(req: &Request) -> Class { /* capability/tenant → class */ }
  // drr.schedule(class, req) provides fairness + backpressure.
  ```
* [C-3] **Readyz with write-shedding**

  ```rust
  enum Mode { Healthy, DegradedReadOnly { reason: &'static str }, Unready { reason: &'static str } }
  fn readyz() -> (StatusCode, Json<Status>) {
      // Reads remain 200 in DegradedReadOnly; writes return 429/503 with Retry-After.
      /* reflect queue thresholds, downstream probes, and headroom */
  }
  ```
* [C-4] **HTTP↔OAP broker**

  ```rust
  // Map REST verbs to OAP envelopes, stream bodies in ~64 KiB chunks,
  // attach idempotency keys on mutations, propagate capabilities end-to-end.
  ```
* [C-5] **Observability contract**

  ```
  http_requests_total{route,method,status}
  request_latency_seconds{route,method}        # histogram (p50/p95/p99)
  inflight_requests{route}                     # or implied by concurrency limit
  rejected_total{reason}                       # quota|degraded|oversize|timeout
  drr_inflight{class} / drr_deficit{class}
  amnesia{"on"|"off"}
  upstream_latency_seconds{hop="index|storage|mailbox"}  # attribution, not aggregation
  ```
* [C-6] **Decompression guard**

  * Enforce both **absolute** post-inflate cap and **ratio ≤10×**.
  * Reject with **413** and `{reason="oversize"}` when exceeded; never continue partial inflate.
* [C-7] **Statelessness**

  * No local object caches or index tables; use `svc-edge`/`svc-storage` for bytes and `svc-index` for resolution.

## 4. Acceptance Gates (PROOF)

> Each gate is CI-enforceable against `svc-gateway`; suggested test filenames in parentheses.

* [G-1] **OAP limits honored:** Payloads > 1 MiB are rejected with **413**; streaming occurs near 64 KiB chunk size. *(tests/I02_oap_limits_1mib.rs)*
* [G-2] **Readiness under overload:** Synthetic load flips `/readyz` to **503** while reads continue **200** and writes return **429/503** until pressure clears; reasons are reported. *(tests/I04_readyz_overload.rs)*
* [G-3] **Backpressure metrics:** DRR gauges and `rejected_total{reason}` change under load; labels present and stable. *(tests/I05_metrics_drr_rejects.rs)*
* [G-4] **Amnesia matrix:** With Micronode + amnesia=ON, filesystem inspection shows no persisted bodies/keys; with amnesia=OFF, only configured paths exist. *(tests/I07_amnesia_fs_scan.rs)*
* [G-5] **Boundary audit:** Static checks ensure no storage/index/DHT/transport loops in the crate; only typed clients are used. *(ci/scripts/check_boundary.sh)*
* [G-6] **Facet SLO benches (read-heavy):**

  * Graph/Feed hydration p95 ≤ 150 ms @ payload P50 128 KiB; bounded fan-out ≤ 3 downstream calls.
  * Media byte-range p95 ≤ 100 ms @ 128 KiB window.
  * Search autocomplete p95 ≤ 75 ms @ 10 results.
    Gate fails on **>10% regression** or SLO breach; failed runs publish flamegraphs. *(benches/B06_facets_slo.rs)*
* [G-7] **Capability parser fuzz:** 1h nightly fuzz for capability/macaron envelopes: no panics/OOM/UB; memory stable. *(fuzz/fuzz_capability.rs)*
* [G-8] **Attribution required:** `upstream_latency_seconds{hop}` must be present; a CI rule fails if only aggregate latency is emitted without per-hop labels. *(ci/rules/metrics_attribution.yml)*
* [G-9] **Decompression guard tests:** Corpus includes high-ratio inputs; exceeding ratio or post-inflate cap returns **413** quickly with `{reason="oversize"}`; no partial inflate continuation. *(tests/I10_decompress_guard.rs)*

## 5. Anti-Scope (Forbidden)

* Business/domain logic, ranking, or moderation workflows (belong in `svc-mod`/facets).
* Local persistent caches or in-process per-tenant state (use dedicated cache services if required).
* Storage/index/DHT/transport implementations (owners: `svc-storage`, `svc-index`, `svc-dht`, `ron-transport`).
* Ambient authentication (must validate explicit capabilities; no trust by origin alone).

## 6. References

* `FULL_PROJECT_BLUEPRINT.MD` — OAP/1 wire limits; content addressing; readiness semantics.
* `SCALING_BLUEPRINT.MD` — DRR fair-queueing, shed-before-collapse, SLO posture.
* `HARDENING_BLUEPRINT.MD` — Structured errors, body caps, decompression guard.
* `APP_INTEGRATION_BLUEPRINT.MD` — Facet composition (Graph/Feed/Search/Media) via ingress.
* `12_PILLARS.MD` — Pillar-6 (Ingress & Edge) mapping; role boundaries.
* `SIX_CONCERNS.MD` — SEC/RES/PERF/GOV/DX spine and cross-checks.
* `OMNIGATE_BLUEPRINT.MD` (if separate) — Narrative design for Omnigate role.
* `DEVELOPER_SUITE_BLUEPRINT.MD` — DX expectations & SDK interop for ingress.

```

**Notes**
- I kept values consistent with your canon: **1 MiB** OAP frame, **~64 KiB** streaming chunks, **≤10×** decompression ratio, write-first shedding, DRR fairness, capability gating, and amnesia on Micronode.
- The HOW section uses short, practical snippets (not full programs), per your “copy-paste ergonomics” goal.
- The PROOF section maps every new facet and hardening requirement to explicit, CI-friendly tests/benches/scripts.
