---

crate: svc-storage
path: crates/svc-storage
role: service
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
-------------------------

## 1) One-liner

Durable, content-addressed DAG store with rolling-hash chunking and Reed–Solomon erasure that serves objects to the overlay in 64 KiB streams, verifying BLAKE3 before returning bytes.  &#x20;

## 2) Primary Responsibilities

* Store objects as a manifest/DAG (DAG-CBOR) and chunks, addressed by `b3:<hex>` (BLAKE3-256).&#x20;
* Provide read-path primitives (`GET`, `HAS`) over an internal API so Overlay can stream **64 KiB** chunks to Gateway/clients. &#x20;
* Maintain durability & availability via erasure coding and background repair.&#x20;

## 3) Non-Goals

* No public HTTP surface, quotas, or tenant policy (that’s Gateway/Omnigate).&#x20;
* No provider discovery or routing (Index/DHT handle resolve/announce).&#x20;
* No application-level behavior or decryption; services verify bytes and keep APP\_E2E opaque.&#x20;

## 4) Public API Surface

* **Re-exports:** none (service binary).
* **Service endpoints (internal):** read-path API offering `GET(hash=b3:<hex>)` and `HAS(hash)`; streaming in 64 KiB chunks (implementation detail distinct from OAP/1 `max_frame=1 MiB`). &#x20;
* **Manifests/DAG:** objects modeled as manifests (DAG-CBOR) with chunk references.&#x20;
* **Admin plane (repo invariant):** `/healthz`, `/readyz`, `/metrics` exposed across services.&#x20;

## 5) Dependencies & Coupling

* **Internal crates → why; stability; replaceable?**

  * **svc-overlay** (caller): streams objects to edge; *loose coupling* over RPC/UDS; replaceable=yes.&#x20;
  * **svc-index** (peer): stores/serves provider records; storage itself does not resolve; *loose*; replaceable=yes.&#x20;
* **External crates (likely, per blueprint standards):**

  * `blake3` (addressing/integrity), `reed-solomon-erasure` (parity/repair), `tokio` & `bytes` (async/zero-copy), telemetry stack (Prometheus, tracing). These are mainstream/maintained → moderate risk. &#x20;
* **Runtime services:** local disk (object/chunk store), OS (files/sockets), internal RPC (UDS/TCP) from Overlay, crypto (BLAKE3 digest). &#x20;

## 6) Config & Feature Flags

* **Store root path** used by pack/tools and services (e.g., `.onions` in quick-start), must align across writer and storage to avoid phantom 404s. &#x20;
* **Streaming/erasure knobs:** 64 KiB streaming chunk is an implementation detail; erasure coding parameters and **repair pacing ≤ 50 MiB/s** per cluster (operational cap). &#x20;
* **Cargo features:** none called out yet (PQ and federation features live elsewhere / future).&#x20;

## 7) Observability

* **Endpoints:** `/healthz`, `/readyz`, `/metrics` present per repo invariant.&#x20;
* **Golden metrics (target set):** `requests_total{code}`, `bytes_{in,out}_total`, `latency_seconds`, `inflight`, plus storage histograms; to be standardized across services.&#x20;

## 8) Concurrency Model

* **Runtime:** Tokio multi-threaded; bounded CPU pool for hashing/erasure/compression; cancellations propagate (parent drops cancel children).&#x20;
* **Backpressure:** Semaphores at dials/inbound/CPU; bounded queues; deadlines on awaits.&#x20;
* **Zero-copy:** use `bytes::Bytes`, vectored I/O on hot paths.&#x20;

## 9) Persistence & Data Model

* **Addressing/integrity:** `b3:<hex>` (BLAKE3-256) over plaintext object/manifest root; full digest **MUST** be verified before returning bytes.&#x20;
* **Layout:** Manifest (DAG-CBOR) → chunk files (rolling-hash 1–4 MiB) with parity shards; background repair jobs maintain RF. &#x20;

## 10) Errors & Security

* **Error taxonomy (converging):** canonical 2xx/4xx/5xx with `Retry-After` on 429/503; storage should map internal conditions accordingly once adopted repo-wide.&#x20;
* **Mutual auth / secrets (non-public planes):** mTLS/Noise on internal planes; amnesia mode and secret handling policies apply across services. &#x20;
* **Integrity:** services verify BLAKE3 equality for full digests prior to serving bytes.&#x20;
* **PQ-readiness:** OAP/1 unchanged; PQ lands later (OAP/2/SDK), not a storage service concern yet.&#x20;

## 11) Performance Notes

* **Hot path:** manifest resolve → chunk reads → stream **64 KiB** → return; keep internal API p95 < 40 ms, p99 < 120 ms. &#x20;
* **Repair pacing:** cap erasure repair at ≤ 50 MiB/s per cluster to avoid cache thrash.&#x20;

## 12) Tests

* **Now:** local end-to-end read path proven (Gateway→Index→Overlay→Storage 200 OK).&#x20;
* **Planned (M2):** implement `GET`/`HAS` streaming w/ BLAKE3 verify; minimal tileserver example; latency histograms.&#x20;
* **Future:** property/fuzz on chunker & manifest traversal; chaos (IO stalls, partial reads) as part of red-team.&#x20;

## 13) Improvement Opportunities

* **Land the read path fully** (GET/HAS + 64 KiB streaming + verify on read) and ship the tileserver example.&#x20;
* **Golden metrics & dashboards**: align counters/histograms with Gateway/Overlay.&#x20;
* **Erasure/repair ops:** enforce pacing; surface repair/backlog metrics.&#x20;
* **Drift killers:** CI greps for BLAKE3 terms and OAP/1 vs 64 KiB chunking to prevent spec drift.&#x20;
* **DX:** keep store root path consistent between packers and storage to eliminate “phantom 404s”.&#x20;

## 14) Change Log (recent)

* **2025-09-05** — M2 tickets defined for **Storage read-path (GET/HAS, 64 KiB streaming)** with tileserver example.&#x20;
* **2025-09-05** — Local **Gateway→Index→Overlay→Storage** manifest fetch validated via `gwsmoke`.&#x20;

## 15) Readiness Score (0–5 each)

* **API clarity:** 3.0 — Role and interfaces are clear; needs a short in-repo API note.&#x20;
* **Test coverage:** 2.5 — E2E smoke proven; unit/property/chaos tests pending for chunker/erasure. &#x20;
* **Observability:** 3.0 — Health/ready/metrics baseline exists; golden set not fully wired. &#x20;
* **Config hygiene:** 3.0 — Clear store-root pattern; alignment required across tools/services; streaming/erasure knobs defined. &#x20;
* **Security posture:** 3.0 — BLAKE3 verification mandated; internal plane auth policies documented; PQ slated for OAP/2. &#x20;
* **Performance confidence:** 3.0 — Targets set; streaming and repair pacing specified; needs sustained tests. &#x20;
* **Coupling (lower is better):** 3.0 — Loosely coupled to Overlay/Index over internal RPC; clear boundaries.&#x20;

---
