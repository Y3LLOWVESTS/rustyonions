---

crate: oap
path: crates/oap
role: lib
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
-------------------------

## 1) One-liner

A tiny, dependency-light codec + helpers for **OAP/1** that parse/serialize frames, enforce bounds, and provide canonical DATA packing (with `obj:"b3:<hex>"`) used by the SDK, gateway, and services. &#x20;

## 2) Primary Responsibilities

* Implement the **OAP/1** wire format and state machine with strict bounds and errors, plus HELLO negotiation helpers. &#x20;
* Provide canonical **DATA packing** helpers that embed `obj:"b3:<hex>"` in a framed header; both sides use the same logic to prevent drift. &#x20;
* Ship normative **test vectors** and parser tests/fuzz to guarantee interop (SDK parity, conformance suite). &#x20;

## 3) Non-Goals

* Not a transport or TLS layer (uses kernel/transport or SDK for I/O); no business logic, economics, or service-level quotas here.&#x20;
* Not a capability system or verifier (macaroons are referenced/encoded, but verification/enforcement lives at services/gateway).&#x20;

## 4) Public API Surface

* **Re-exports:** OAP constants (version, limits), status codes, flags; canonical test vectors (A–T).&#x20;
* **Key types / functions / traits:**

  * `OapFrame { len, ver, flags, code, app_id, tenant, caplen, corr_id, cap, payload }` with `encode/decode`.&#x20;
  * `Flags` bitset (e.g., `REQ`, `RESP`, `START`, `END`, `ACK_REQ`, `APP_E2E`, `COMP`).&#x20;
  * `HelloInfo { max_frame, max_inflight, supported_flags, version }` and `hello()` probe.&#x20;
  * `data_frame()` + `encode_data_payload` / `decode_data_payload` placing `obj:"b3:<hex>"` into the header. &#x20;
  * `Error` enum with typed reasons (bad\_frame, oversize, unsupported\_flag, decompress\_too\_large, unauthorized, quota, timeout). &#x20;
* **Events / HTTP / CLI:** none directly; consumed by gateway/services; vectors runnable via `ronctl test oap --vectors`.&#x20;

## 5) Dependencies & Coupling

* **Internal crates:**

  * *ron-kernel*: none at compile-time (keep codec independent); used by gateway that consumes oap. Tight runtime coupling avoided by design. Replaceable: **yes** (codec could be swapped with a generated one).&#x20;
  * *gateway/sdk*: depend **on** `oap`, not vice-versa (to avoid layering inversions).&#x20;
* **External crates (expected top 5; minimal pins/features):**

  * `bytes` (frame I/O without copies), `serde`/`serde_json` (HELLO/DATA header JSON), optional `zstd` (COMP), optional `tokio` (demo I/O), `uuid` (tenant). Risks: low/maintained; zstd guarded.&#x20;
* **Runtime services:** none (pure codec). TLS and Tor belong to transport/gateway; ALPN/TLS posture is normative input.&#x20;

## 6) Config & Feature Flags

* **Env/config:** n/a for the core crate (limits negotiated via HELLO). `max_frame` defaults to **1 MiB** unless HELLO lowers it.&#x20;
* **Cargo features:** `comp` (zstd compression; enforce 8× bound), `pq-preview` (future macaroons PQ verifier compatibility, no wire change), `tcp-demo` (tokio helpers). &#x20;

## 7) Observability

* The crate itself is logic-only; metrics are emitted at gateway/services. Golden metrics include `requests_total`, `bytes_{in,out}_total`, `rejected_total{reason}`, `latency_seconds`, `inflight`, `quota_exhaustions_total`.&#x20;
* SDK/gateway should expose HELLO timing and parse errors; kernel emits health/service events.&#x20;

## 8) Concurrency Model

* Pure functions for encode/decode; no interior mutability.
* For I/O demos, **ACK\_REQ** + server window with `max_inflight` backpressure; timeouts/retries are caller policy (SDK/gateway). &#x20;

## 9) Persistence & Data Model

* None (stateless codec). OAP embeds `tenant` and optional capability bytes; DATA header includes `obj:"b3:<hex>"` for content addressing. &#x20;

## 10) Errors & Security

* **Error taxonomy:** oversize (payload > negotiated `max_frame`), malformed header, unsupported version/flag, decompressed size bound exceeded (≤ 8× `max_frame`), unauthorized (cap), quota (service). &#x20;
* **Security posture:** TLS 1.3 with ALPN `ron/1` (transport), no TLS compression/renegotiation; `APP_E2E` means packet body is opaque to kernel/services. PQ readiness planned without changing OAP. &#x20;

## 11) Performance Notes

* Hot path is frame parse/serialize and DATA header (small JSON) read/write; use `bytes::Bytes` to minimize copies.
* System SLOs for OAP under load: **p95 < 40 ms**, **p99 < 120 ms** (integration target); enforce 1 MiB `max_frame` and 64 KiB streaming at storage as distinct knobs. &#x20;

## 12) Tests

* **Unit:** vectors A–T (HELLO, REQ|START, cap present/absent, COMP bounded, error cases).&#x20;
* **Integration:** conformance harness over TCP+TLS and Tor; must match SDK bytes/timings.&#x20;
* **Fuzz/property:** parser fuzz (≥ 1000h cumulative over time) and proptests in CI; persist and replay corpus. &#x20;
* **Formal (optional):** TLA+ sketch of state transitions.&#x20;

## 13) Improvement Opportunities

* **Eliminate duplication risk:** Some plans had a parser in `ron-kernel/overlay/protocol.rs`; consolidate into `oap` as the single source of truth and make kernel/gateway depend on it. &#x20;
* **Create `ron-proto` crate:** Move constants/status codes/headers/vectors there to prevent drift across SDKs/services.&#x20;
* **Observability hooks:** Expose lightweight counters inside `oap` behind a feature (e.g., parse errors by reason) to feed golden metrics upstream.&#x20;
* **Leakage harness:** Add padding/jitter toggles and doc guidance (cross-plane leakage checks).&#x20;

## 14) Change Log (recent)

* **2025-09-14** — Drafted deep-dive and alignment with **GMI-1.6** invariants (1 MiB `max_frame`, DATA packing with `b3:`).&#x20;
* **2025-09-13** — Acceptance checklists added to ensure `max_frame` alignment and rejected reasons.&#x20;

## 15) Readiness Score (0–5 each)

* **API clarity:** 4 (wire format & helpers are crisp; finalize code-level docs once `ron-proto` lands).&#x20;
* **Test coverage:** 3 (vectors specified; need CI proptests/fuzz soaking).&#x20;
* **Observability:** 3 (golden metrics defined upstream; add error counters in codec if useful).&#x20;
* **Config hygiene:** 5 (negotiated via HELLO; no env in core).&#x20;
* **Security posture:** 4 (TLS 1.3 + ALPN; APP\_E2E opaque; PQ path planned). &#x20;
* **Performance confidence:** 3 (SLO targets defined; need harness results).&#x20;
* **Coupling (lower is better):** 4 (pure library; ensure kernel doesn’t re-implement parser).&#x20;

