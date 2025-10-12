
````markdown
---
title: svc-interop — Invariant-Driven Blueprint (IDB)
version: 0.2.2
status: reviewed
last-updated: 2025-10-12
audience: contributors, ops, auditors, integrators
---

# svc-interop — Invariant-Driven Blueprint

## 0. Purpose

`svc-interop` is the bridge layer that connects RustyOnions to external ecosystems (REST, GraphQL, webhooks, and foreign SDKs) **without importing ambient external authority** or violating content-addressed, capability-first invariants. It performs **capability translation**, enforces **hardening limits** and **observability**, keeps economic **truth** separated from counters, and ensures **reversible** contracts so the core can function even when bridges are disabled.

---

## 1. Invariants (MUST)

- [I-1] **Reversible bridges only.** Disabling any adapter must leave core objects/capabilities valid; no bridge-specific mutation of core data models.
- [I-2] **No external auth trust.** Never treat third-party tokens/sessions as ambient authority. Always translate to a RustyOnions capability (macaroon) before action.
- [I-3] **Capabilities everywhere.** All inbound actions carry a short-lived capability issued/validated via `svc-passport`/`ron-auth`; keys are under `ron-kms`.
- [I-4] **Protocol bounds.** Respect OAP/1 rules: `max_frame = 1 MiB`. Treat `64 KiB` as streaming chunk size for large payloads. DTOs use `#[serde(deny_unknown_fields)]`.

### Hashing & Signatures Policy (new)
- [I-4a] **Internal hashing is BLAKE3-256.** All internal content addressing, deduplication, and audit hashes MUST be **BLAKE3-256**. This includes object addresses (`"b3:<hex>"`) and the `b3_hash` recorded in audit events.
- [I-4b] **SHA-256 only at the edge for third parties.** SHA-256 (e.g., HMAC-SHA256) MAY be used **transiently and exclusively** to verify third-party webhook signatures (GitHub/Slack/Stripe, etc.) **at ingress**. SHA-256 digests/signatures MUST NOT be stored or used as internal identifiers and MUST NOT propagate beyond the verification step. The only persisted evidence is a boolean `verified=true/false` and a `sig_alg` label (e.g., `"hmac-sha256"`).

- [I-5] **Economic separation.** Interop emits accounting/ledger **events** but never mutates `ron-ledger` truth directly.
- [I-6] **Observability + readiness.** `/metrics`, `/healthz`, `/readyz`, `/version` exist. Under incident load, `/readyz` degrades **bridging** first; read paths stay green as long as safe.
- [I-7] **Hardening defaults.** Request timeout 5s; ≤512 inflight; ≤500 rps/instance; body cap 1 MiB; decompression guard ≤10× with absolute cap; deterministic error mapping.
- [I-8] **Amnesia honored.** In Micronode amnesia mode: no on-disk persistence, ephemeral logs/queues, secrets zeroized per policy.
- [I-9] **Backpressure first.** Apply quotas/rate limits **before** heavy work; never create unbounded queues; early reject with reason-coded responses.
- [I-10] **Canon discipline.** No pillar drift or new crates. `svc-interop` stays scoped to bridging (P12 + concerns: DX, SEC, PERF).

### Expanded Invariants
- [I-11] **Streaming above frame.** Any payload >1 MiB MUST stream in `64 KiB` chunks. Server buffers ≤2 chunks/request (≤128 KiB) in memory; otherwise backpressure or 413.
- [I-12] **Auditability.** Every bridge action emits an audit event: {redacted capability ID, provider, provider request ID, **BLAKE3-256 payload hash**, decision allow/deny, reason code, latency bucket}. Audit sinks obey amnesia mode.
- [I-13] **TTL concretes.** Translated capabilities TTL ≤60s; single-audience scope (service + action). Refresh requires revalidation with `svc-passport`.
- [I-14] **Origin pinning.** Webhooks enforce signed provider descriptors + host allow-list. Replay window ≤5 minutes with idempotency keys.
- [I-15] **Privacy-by-default.** No external PII is stored unless explicitly authorized by policy (with retention TTL). Otherwise redact at edge.
- [I-16] **Deterministic error taxonomy.** All rejects map to a closed set: `{BAD_ORIGIN, EXPIRED_TOKEN, SCOPE_MISMATCH, BODY_LIMIT, DECOMP_LIMIT, RATE_LIMIT, BACKPRESSURE, DOWNSTREAM_UNAVAILABLE, POLICY_BLOCKED}`.
- [I-17] **No synchronous coupling.** Do not perform blocking external calls on internal critical paths. If p95 external call >100 ms, route via mailbox/work queue.
- [I-18] **SLO guardrails.** p95 bridge latency intra-region ≤120 ms; success ≥99.9% for healthy providers under nominal load. Enforce with rate limiting and brownout.

---

## 2. Design Principles (SHOULD)

- [P-1] **Adapter isolation.** Provider codegen/glue lives in per-provider modules; core DTOs remain provider-agnostic.
- [P-2] **Idempotency at edges.** Treat inbound webhooks/jobs as at-least-once; require idempotency keys; mailbox handles retries and visibility timeouts.
- [P-3] **Declarative config.** Enabling adapters/origins requires signed policy/registry entries. Rollbacks are atomic and auditable.
- [P-4] **DX ergonomics, reversible contracts.** Keep REST/GraphQL façades minimal and reversible to OAP/1 and object addresses.
- [P-5] **Fail well.** Prefer explicit 429/503 with `Retry-After`. Degrade writes/bridges first.
- [P-6] **Narrow surface.** Few composable endpoints beat many specialized ones; new endpoints require explicit policy and versioning.
- [P-7] **Provider isolation.** Each adapter ships its own limits, signature validation, and test vectors; failures cannot cascade across providers.
- [P-8] **Schema stability.** Version external DTOs; additive-only by default; breaking changes require dual-accept windows and deprecation plan.
- [P-9] **First-class observability.** Emit RED/USE metrics and reason-coded rejects sufficient for SRE triage without logs.

---

## 3. Implementation (HOW)

> Notes: Use `axum 0.7.x`, `tower`, and `tower-http` layers. DTOs derive `Serialize/Deserialize` with `deny_unknown_fields`. Object addresses are **BLAKE3** (`"b3:<hex>"`).

### 3.1 Router & Hardening Layers (skeleton)

```rust
use axum::{routing::{get, post}, Router, middleware::from_fn};
use tower::{ServiceBuilder, limit::ConcurrencyLimitLayer, timeout::TimeoutLayer};
use tower_http::limit::RequestBodyLimitLayer;
use std::time::Duration;

pub fn router() -> Router {
    let limits = ServiceBuilder::new()
        .layer(ConcurrencyLimitLayer::new(512))            // ≤512 inflight
        .layer(RequestBodyLimitLayer::new(1 * 1024 * 1024))// 1 MiB body cap
        .layer(TimeoutLayer::new(Duration::from_secs(5))); // 5s timeout

    Router::new()
        .route("/put", post(put_handler))
        .route("/o/:addr", get(get_handler))
        .route("/webhooks/:provider", post(webhook_handler))
        .layer(limits)
        .layer(from_fn(capability_translation))  // I-2, I-3
        .layer(from_fn(decompression_guard))     // I-7
        .layer(from_fn(origin_pin))              // I-14
        .layer(from_fn(rate_limit_brownout))     // I-9, I-18
}
````

### 3.2 DTO Hygiene (deny unknown + stable versions)

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PutObjectV1 {
    pub addr: String,         // "b3:<hex>"
    pub content_type: String, // e.g., "application/json"
    #[serde(with = "serde_bytes")]
    pub body: Vec<u8>,
    pub idempotency_key: String,
    pub ts_ms: u64,
}
```

### 3.3 Streaming >1 MiB (64 KiB chunks, ≤2 buffered)

```rust
use axum::{body::Body, response::IntoResponse};
use futures_util::StreamExt;

async fn put_handler(req: axum::http::Request<Body>) -> impl IntoResponse {
    let mut stream = req.into_body().into_data_stream();
    let mut buffered: usize = 0;
    while let Some(chunk) = stream.next().await {
        let bytes = match chunk { Ok(b) => b, Err(_) => return err("DECOMP_LIMIT", 413) };
        if bytes.len() > 64 * 1024 { return err("BODY_LIMIT", 413); }
        buffered += bytes.len();
        if buffered > 128 * 1024 { return err("BACKPRESSURE", 429); }
        // verify + forward chunk to internal storage pipe …
        buffered -= bytes.len();
    }
    ok()
}
```

### 3.4 Reason-coded errors (closed taxonomy)

```rust
#[derive(Debug, serde::Serialize)]
struct ErrorBody { code: &'static str, message: &'static str }

fn err(code: &'static str, status: axum::http::StatusCode)
  -> (axum::http::StatusCode, axum::Json<ErrorBody>)
{
    (status, axum::Json(ErrorBody { code, message: code }))
}
```

### 3.5 Dual-step verify → hash (edge SHA-256, internal BLAKE3)

* **Step 1 (edge verify):** Verify the provider signature **exactly** as documented (e.g., HMAC-SHA256 for GitHub/Slack/Stripe) against the raw body/base string.
* **Step 2 (internalize):** Compute `b3_hash = blake3::hash(body)` and use that for **all internal flows** (addressing, dedup, audits). Persist **no SHA-256 digests**.

```rust
use hmac::{Hmac, Mac};
use sha2::Sha256;
use blake3;

fn verify_sig_hmac_sha256(prefix_hex: &str, body: &[u8], secret: &[u8]) -> bool {
    // e.g., "sha256=<hex>" → strip "sha256=" then hex-decode
    let expected_hex = prefix_hex.splitn(2, '=').nth(1).unwrap_or_default();
    let expected = match hex::decode(expected_hex) { Ok(v) => v, Err(_) => return false };
    let mut mac = Hmac::<Sha256>::new_from_slice(secret).expect("hmac key");
    mac.update(body);
    mac.verify_slice(&expected).is_ok()
}

#[derive(Debug, serde::Serialize)]
struct Audit {
    provider: &'static str,
    verified: bool,        // true if edge verify succeeded
    sig_alg: &'static str, // evidence only (e.g., "hmac-sha256")
    b3_hash: String,       // canonical internal hash
}
fn ingest_webhook(sig_header: &str, body: &[u8], secret: &[u8]) -> Audit {
    let verified = verify_sig_hmac_sha256(sig_header, body, secret);
    let b3 = blake3::hash(body);
    Audit {
        provider: "github",
        verified,
        sig_alg: "hmac-sha256",
        b3_hash: format!("b3:{}", hex::encode(b3.as_bytes())),
    }
}
```

### 3.6 Webhook Receiver (idempotent, pinned origin)

* Check provider origin (allow-list + signed descriptor).
* Compute `idem_key = HMAC(provider_secret, provider_request_id || b3(payload))`.
* Enqueue to `svc-mailbox` with visibility timeout; worker performs internal action; ACK or DLQ with audit evidence.

### 3.7 Audit Events (amnesia-aware)

* Emit `{cap_id_redacted, provider, req_id, **b3_hash**, decision, reason, latency_bucket}` to audit sink.
* In amnesia mode, route sink to in-memory ring buffer; zeroize on interval.

---

## 4. Acceptance Gates (PROOF)

* [G-1] **Security tests.** External token → capability translation unit tests: missing/expired/unknown origins → `{401,403}` with correct codes.
* [G-2] **Hardening checks.** CI enforces: 1 MiB body cap; 5s timeout; ≤512 inflight; ≤500 rps; decompression guard ≤10×; reason-coded 413/429/503 under stress.
* [G-3] **Amnesia matrix.** CI runs amnesia {ON,OFF}. With ON, zero on-disk artifacts; metrics include `amnesia="on"`.
* [G-4] **Interop vectors.** Golden vectors for REST ↔ OAP GET/PUT, GraphQL hydration via omnigate, webhook at-least-once with idempotency; DTOs deny unknown fields.
* [G-5] **Observability.** `/metrics`, `/healthz`, `/readyz` verified. `/readyz` browns out bridges first; dashboards show `rejected_total{reason}`.
* [G-6] **Canon/concern labels.** PRs tagged `pillar:12` and `concern:DX,SEC,PERF`; schema guard checks for DTO compatibility.
* [G-7] **Fuzzing.** libFuzzer targets for webhook parsers, signature verifiers, and DTO decoders; reject overlong headers, mixed encodings, decompression bombs.
* [G-8] **Chaos & brownout.** Fault injection (timeouts, 5xx bursts, slowloris). Read paths maintain ≥99% success while bridges brown out.
* [G-9] **Performance SLO check.** Criterion + k6/hyperfine: p95 ≤120 ms for PUT/GET intra-region at 500 rps with ≤1% error. Publish latency histograms; saturate gracefully at caps.
* [G-10] **Streaming conformance.** >1 MiB requests must chunk at 64 KiB; ≥65 KiB frames → 413 `BODY_LIMIT`; >2 chunks buffered → 429 `BACKPRESSURE`.
* [G-11] **Audit trail integrity.** Golden tests assert audit event fields present/redacted correctly; amnesia=ON writes to in-mem sink only.
* [G-12] **Policy & origin pinning.** Negative vectors: unknown origin, expired signature, clock skew >5m, undeclared endpoint → 403 `POLICY_BLOCKED`.
* [G-13] **Hashing policy enforcement.** Tests assert: (a) internal addresses & audit hashes are **BLAKE3-256**; (b) any SHA-256 usage is confined to edge verification; (c) no SHA artifacts are persisted or propagated internally.

---

## 5. Anti-Scope (Forbidden)

* Importing/forwarding external sessions as ambient authority (no pass-through auth).
* Writing directly to `ron-ledger` truth paths. Interop emits events only.
* Adding DHT/overlay logic here (belongs to `svc-dht`/`svc-overlay`).
* Violating OAP/1 bounds or weakening DTO hygiene.
* **Stateful bridges** caching external session state beyond 60s TTL.
* Adding gRPC/QUIC endpoints without policy entry and versioned schema.
* **Synchronous external calls** on critical internal transitions when p95 >100 ms.
* **Persisting or reusing SHA-256 digests** beyond edge verification. All internal hashes MUST be BLAKE3-256.

---

## 6. SLOs (Perf & Availability)

* **Availability:** 99.9% monthly for bridging endpoints; read paths remain ≥99.95% during provider incidents.
* **Latency (intra-region):** p50 ≤40 ms, p95 ≤120 ms, p99 ≤250 ms at 500 rps/instance.
* **Error budget policy:** Brownout bridges when 5-minute rolling failure rate >1% or upstream p95 >500 ms.

---

## 7. Dependencies & Adjacent Services

* **Depends on:** `ron-proto` (DTOs), `ron-kms` (key custody), `ron-auth`/`svc-passport` (capabilities), `svc-mailbox` (job/retry), `svc-index`/`svc-storage` (object I/O), `svc-metrics` (Prometheus).
* **Does not own:** discovery (`svc-dht`), overlay sessions (`svc-overlay`), truth ledger (`ron-ledger`), user identity (`svc-passport`).

---

## 8. Provider Matrix

| Provider         | Ingress Type | Auth Check | Signature                                                    | Limits | Replay Window | Idem Key | Status |
| ---------------- | ------------ | ---------- | ------------------------------------------------------------ | ------ | ------------- | -------- | ------ |
| GitHub           | Webhook      | Secret     | `X-Hub-Signature-256` (HMAC-SHA256, prefix `sha256=`)        | 1 MiB  | 5m            | yes      | pilot  |
| Stripe           | Webhook      | Secret     | `Stripe-Signature` (`v1`; **test** also `v0`)                | 1 MiB  | 5m            | yes      | pilot  |
| Slack (REST)     | REST         | Bearer     | —                                                            | 1 MiB  | n/a           | yes      | alpha  |
| Slack (Webhooks) | Webhook      | Secret     | `X-Slack-Signature` (HMAC-SHA256, prefix `v0=` over ts+body) | 1 MiB  | 5m            | yes      | alpha  |

---

## 9. References

* Scaling & Hardening blueprints (limits, brownout, decompression guard).
* Pillars & Six Concerns (P12 placement; DX/SEC/PERF obligations).
* OAP/1 contract (1 MiB frame, 64 KiB streaming chunk).
* Capability & KMS docs (macaroons; custody; TTLs).
* Micronode amnesia policy (RAM-only; zeroization).
* Hash policy: **BLAKE3-256 for all internal uses**; **SHA-256 only for third-party edge verification**.

---

## 10. Appendix — Axum Version Pin (Workspace Policy)

Code examples target **axum 0.7.x** by design. The RustyOnions workspace pins axum at 0.7.9 to keep the tower/hyper feature matrix unified across the 33 crates. A future, coordinated bump to axum 0.8.x will be executed at the workspace root with an ADR and cross-crate refactor plan (not piecemeal). The migration note should cover extractor changes and any middleware API shifts.

---

## Changelog

* **v0.2.2 (2025-10-12)**

  * Added explicit **hashing & signatures policy**: [I-4a] BLAKE3-256 for all internal hashes; [I-4b] SHA-256 allowed **only** at the edge for third-party verification and never persisted internally.
  * Added **3.5 Dual-step verify → hash** section with example code.
  * Added **G-13 Hashing policy enforcement** and Anti-Scope bullet forbidding persisted SHA-256.
* **v0.2.1 (2025-10-12)**

  * Fixed Provider Matrix details (Stripe v1/test v0; Slack split; GitHub `sha256=` prefix). Added Axum 0.7.x appendix.
* **v0.2.0 (2025-10-12)**

  * Added Purpose, expanded invariants (I-11..I-18), Design Principles (P-6..P-9), streaming rules, closed error taxonomy, amnesia-aware audits.
  * Added PROOF gates (G-7..G-12), SLOs, Dependencies, Provider Matrix.
  * Included Rust snippets (router/hardening, DTO hygiene, streaming, reason-coded errors).

```

