

````markdown
---
title: svc-gateway — Ingress IDB (OAP/1 over HTTP)
version: 1.0.0
status: draft
last-updated: 2025-10-02
audience: contributors, ops, auditors
---

# svc-gateway — Invariant-Driven Blueprinting (IDB)

> Role: hardened, stateless **HTTP ↔ OAP/1 ingress**. Terminates TLS, enforces quotas/DRR, applies caps **before** heavy work, exposes health/readiness, and forwards to internal services. No hydration—`omnigate` owns that.

---

## 1. Invariants (MUST)

- **[I-1] Canon & role.** Gateway is the HTTP↔OAP ingress. It terminates TLS, enforces **fair-queue DRR**, applies **quotas** and **caps** at the edge, exposes `/metrics`, `/healthz`, `/readyz`, and remains **stateless** aside from counters.
- **[I-2] OAP/1 bounds.** Enforce protocol limits: `max_frame = 1 MiB` (413 if exceeded). Stream request bodies in ~`64 KiB` chunks; never buffer unbounded.
- **[I-3] Content addressing on reads.** When proxying CAS reads, set strong `ETag = "b3:<hex>"` and verify digest before serving bytes; preserve integrity headers end-to-end.
- **[I-4] Hardening defaults.** Timeouts = `5s`, inflight ≤ `512`, rate limit ≤ `500 rps/instance`, body cap `1 MiB`. Decompression guarded (see I-11). Reject early with structured errors.
- **[I-5] Readiness semantics.** `/readyz` **sheds writes first** under pressure; prefer read-only service continuity. Backpressure/DRR state must be observable.
- **[I-6] Capabilities only.** All mutating routes require **capability tokens** (e.g., macaroons). No ambient trust.
- **[I-7] No kernel creep.** No kernel bus, DHT, overlay, ledger, or storage logic inside gateway. It only **consumes** internal services.
- **[I-8] PQ-neutral TLS.** Use `tokio_rustls` types and allow PQ-hybrid flags to pass through; never down-convert negotiated suites.
- **[I-9] Golden metrics.** Expose at minimum:  
  `http_requests_total{route,method,status}`,  
  `request_latency_seconds{route,method}`,  
  `inflight_requests{route}`,  
  `rejected_total{reason}`,  
  plus DRR/quotas metrics below.
- **[I-10] Amnesia mode honor.** In “amnesia=ON” profiles, avoid on-disk spill; use RAM-only buffers/ring logs; zeroize secrets on drop.
- **[I-11] Decompression guard is mandatory.** If decoding (gzip/deflate/br), enforce **ratio cap ≤ 10:1** and **absolute decoded cap ≤ 8 MiB**. Reject 413 on breach. No streaming decode without guard.
- **[I-12] Configurable but safe defaults.** Hardening knobs are tunable via config, but defaults are safe. Lowering below defaults requires `danger_ok=true` and is disallowed in prod builds.
- **[I-13] No custom auth paths.** No passwords, cookies, or session stickiness. Only bearer-style capability tokens.
- **[I-14] Deterministic error taxonomy.** Map edge failures to a fixed set:  
  `400` malformed, `401` missing/invalid cap, `403` cap denied,  
  `413` too large / decoded too large, `415` unsupported media,  
  `429` quota/DRR, `503` degraded.

---

## 2. Design Principles (SHOULD)

- **[P-1] Fail fast at the edge.** Apply caps/quotas/DRR before any downstream I/O; use 429/503 with `Retry-After`.
- **[P-2] Small, composable layers.** Compose tower layers for timeout, concurrency, rate limiting, body cap, tracing; keep handlers thin.
- **[P-3] Predictable degrade.** Prefer read-only availability; `/readyz` truthfully reflects shedding; logs and metrics tell the truth.
- **[P-4] DTO hygiene.** Uphold strict schemas (deny unknowns) when unwrapping OAP envelopes for policy checks.
- **[P-5] Stateless scale.** No session state; horizontal scale behind LB.
- **[P-6] Observability is non-optional.** Every limiter/queue/backpressure decision emits a metric and a trace span with a reason code.
- **[P-7] Bytes first, policy early.** Validate size/encoding first, then capabilities, then dial downstream.

---

## 3. Implementation (HOW)

### [C-1] Hardening stack (Axum + tower)
```rust
use tower::{ServiceBuilder, limit::ConcurrencyLimitLayer};
use tower_http::{
  timeout::TimeoutLayer,
  limit::RequestBodyLimitLayer,
  request_id::MakeRequestUuidLayer,
  decompression::DecompressionLayer,
};
use std::time::Duration;

pub fn hardening() -> impl tower::Layer<axum::Router> {
    ServiceBuilder::new()
      .layer(TimeoutLayer::new(Duration::from_secs(5)))      // I-4
      .layer(ConcurrencyLimitLayer::new(512))                // I-4
      .layer(RequestBodyLimitLayer::new(1 << 20))            // 1 MiB (I-2,I-4)
      .layer(MakeRequestUuidLayer::default())
      .layer(DecompressionLayer::new())                      // Pair with I-11 guard
}
````

### [C-2] DRR sketch (deterministic & observable)

```rust
struct RequestCtx { /* tenant_id, cost_units(), ... */ }
struct TenantQ { id: String, deficit: u32, quantum: u32, queue: Vec<RequestCtx> }

fn pick_next(tenants: &mut [TenantQ]) -> Option<RequestCtx> {
    for t in tenants.iter_mut() {
        t.deficit = t.deficit.saturating_add(t.quantum);
        if let Some(req) = t.queue.first().cloned() {
            let cost = req.cost_units().max(1);
            if t.deficit >= cost {
                t.deficit -= cost;
                let _ = t.queue.remove(0);
                // emit metrics: gateway_drr_queue_depth{tenant}, gateway_quota_exhaustions_total{tenant}
                return Some(req);
            }
        }
    }
    None
}
```

### [C-3] Structured rejects (stable DX)

```rust
#[derive(serde::Serialize)]
struct Reject { code: u16, reason: &'static str, retry_after: Option<u32> }

fn too_many_requests(seconds: u32)
-> (axum::http::StatusCode, [(&'static str, String);1], axum::Json<Reject>) {
    use axum::http::StatusCode as S;
    (S::TOO_MANY_REQUESTS,
     [("Retry-After", seconds.to_string())],
     axum::Json(Reject{ code: 429, reason: "quota", retry_after: Some(seconds) }))
}
```

### [C-4] Readiness that sheds writes first

```rust
pub async fn readyz(state: AppState) -> impl axum::response::IntoResponse {
    if state.shed_writes() {
        return (axum::http::StatusCode::SERVICE_UNAVAILABLE,
                [("Retry-After","1")],
                "degraded: shedding writes").into_response();
    }
    "ready"
}
```

### [C-5] OAP/1 enforcement at ingress

* Validate `Content-Length` ≤ `1 MiB` for framed payloads; 413 otherwise.
* Stream in ~`64 KiB` reads; map envelope errors to: `400/413/429/503`.

### [C-6] Metrics taxonomy (minimum)

* HTTP: `http_requests_total{route,method,status}`, `request_latency_seconds{route,method}`, `inflight_requests{route}`
* Quotas/DRR: `gateway_quota_exhaustions_total{tenant}`, `gateway_drr_queue_depth{tenant}`
* Readiness/backpressure: `rejected_total{reason}`, `gateway_degraded{bool}`
* Version/health endpoints: `/metrics`, `/healthz`, `/readyz`, `/version`

### [C-7] TLS / PQ-neutral plumbing

* Use `tokio_rustls` server config; surface PQ flags; never strip them.

### [C-8] Amnesia toggle

* When enabled: RAM-only request buffers; ring-buffer tracing; redact PII; zeroize caps.

### [C-9] Decompression guard helper (ratio + absolute)

```rust
use axum::body::Body;
use http::Request;
use tokio::io::AsyncReadExt;
use tokio_util::io::StreamReader;

const DECODE_ABS_CAP: usize = 8 * 1024 * 1024; // 8 MiB
const DECODE_RATIO_MAX: usize = 10;            // 10:1

pub async fn read_with_guard(
    req: Request<Body>,
    claimed_len: Option<usize>
) -> Result<Vec<u8>, (http::StatusCode, &'static str)> {
    let mut in_bytes: usize = claimed_len.unwrap_or(0);
    let mut out = Vec::new();
    let mut reader = StreamReader::new(
        req.into_body().map_err(|_| std::io::Error::from(std::io::ErrorKind::Other))
    );
    let mut buf = [0u8; 64 * 1024]; // ~64 KiB
    loop {
        let n = reader.read(&mut buf).await.map_err(|_| (http::StatusCode::BAD_REQUEST, "read"))?;
        if n == 0 { break; }
        in_bytes = in_bytes.saturating_add(n);
        out.extend_from_slice(&buf[..n]);
        if out.len() > DECODE_ABS_CAP { return Err((http::StatusCode::PAYLOAD_TOO_LARGE, "decoded-cap")); }
        if in_bytes > 0 && out.len() > in_bytes.saturating_mul(DECODE_RATIO_MAX) {
            return Err((http::StatusCode::PAYLOAD_TOO_LARGE, "decoded-ratio"));
        }
    }
    Ok(out)
}
```

### [C-10] Config surface (keys + defaults)

* `gateway.timeout_ms = 5000`
* `gateway.max_inflight = 512`
* `gateway.rate_limit_rps = 500`
* `gateway.body_cap_bytes = 1_048_576`   # 1 MiB
* `gateway.decode_abs_cap_bytes = 8_388_608`   # 8 MiB
* `gateway.decode_ratio_max = 10`
* `gateway.amnesia = true|false`
* `gateway.danger_ok = false`   # required to weaken defaults (non-prod only)

---

## 4. Acceptance Gates (PROOF)

* **[G-1] Limits test.** 2 MiB body → **413**. 600 rps for 60s → **429** with `Retry-After`. `rejected_total{reason}` increments as expected.
* **[G-2] Readiness behavior.** Under induced pressure (restart storm or quota exhaustion), `/readyz` → **503** and writes are shed; reads continue if possible.
* **[G-3] OAP conformance.** Fuzz/vectors confirm 1 MiB frame cap, ~64 KiB streaming, stable error mapping: `400/413/429/503`.
* **[G-4] Capability enforcement.** Mutating routes without valid token → **401/403**; audit log includes reason and route.
* **[G-5] Observability gates.** Prometheus scrape exposes required metrics; dashboard shows latency histograms, queue depth, quota exhaustions, reject reasons.
* **[G-6] Amnesia matrix.** With `amnesia=ON`, integration proves zero on-disk spill; with OFF, logs rotate safely.
* **[G-7] CI concerns.** Deny `await_holding_lock`, `unwrap_used`; run sanitizer job; test both single-thread and multi-thread runtimes.
* **[G-8] Perf baseline lock.** At 400 rps, **p95 ≤ 150 ms** for 64 KiB requests on the Bronze reference host; CI fails if p95 regresses >20% vs baseline JSON.
* **[G-9] Decompression bomb test.** 100 KiB gzip expanding to ~12 MiB → **413** with `reason="decoded-cap"`; no OOM; stable memory profile.
* **[G-10] Error taxonomy conformance.** Vector tests assert status/reason pairs per [I-14].
* **[G-11] Config safety guard.** With `danger_ok=false`, setting `max_inflight=10_000` fails startup with a clear diagnostic; with `danger_ok=true` in `dev`, it starts.

---

## 5. Anti-Scope (Forbidden)

* ❌ View hydration, templating, BFF logic (belongs to `omnigate`).
* ❌ Overlay/DHT/session management (`svc-overlay`, `svc-dht` own these).
* ❌ Ledger/economic semantics (wallet, rewards, ads).
* ❌ Persistent state beyond counters/metrics; no sessions/cookies.
* ❌ Kernel or transport event loops (`ron-kernel`, `ron-transport` own these).
* ❌ WebSocket business logic. WS pass-through only under same caps/limits; no stateful chat/render logic.

---

## 6. References (non-normative)

* RustyOnions canon: 33-crate atlas, 12 Pillars, Six Concerns.
* Hardening & Scaling blueprints: timeouts, caps, DRR, degrade-first.
* Concurrency & Observability guides: runtime discipline, golden metrics set.
* OAP/1 and CAS conventions: 1 MiB frames, BLAKE3 `b3:<hex>` integrity.

```

