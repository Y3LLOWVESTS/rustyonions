

---

````markdown
---
title: svc-edge — Invariant-Driven Blueprint (IDB)
version: 1.2.0
status: reviewed
last-updated: 2025-10-17
audience: contributors, ops, auditors
---

# svc-edge — IDB

## 1. Invariants (MUST)
- [I-1] **Hardened ingress profile** (CI-enforced):
  - timeout ≤ 5s; max inflight ≤ 512; rate cap ≤ 500 RPS/instance;
  - body cap ≤ 1 MiB; decompression safety ≤ 10× with absolute cap;
  - streaming I/O only (no unbounded buffering).
- [I-2] **TLS & headers**:
  - TLS termination active; `Strict-Transport-Security` on; `Referrer-Policy: no-referrer`;
  - CORS explicit (deny-by-default).
- [I-3] **Modes are exact**:
  - `Hit` (serve from pack/CAS), `Miss/Live` (allow-list + bucket → HTTPS → ETag validate → CAS write), or `Offline Pack` (PMTiles/MBTiles, zero egress). No hidden paths.
- [I-4] **HTTP caching correctness**:
  - Strong `ETag` (content hash) for immutable assets; conditional requests return `304` when valid;
  - `Accept-Ranges: bytes` and `206` partial content supported; multi-range disabled unless explicitly enabled.
- [I-5] **Content-address integrity**:
  - CAS keys are **BLAKE3** of bytes; writes are idempotent; double-writes safe; corruption impossible without hash mismatch.
- [I-6] **Amnesia compliance**:
  - With `security.amnesia=true`, no disk persistence; attempted writes → reject and flip `/readyz` to degraded.
- [I-7] **Zero ambient trust**:
  - Protected paths require capability verification upstream; anonymous assets still quota-governed and rate-limited.
- [I-8] **Interop/size bounds honored**:
  - Frame/chunk/size ceilings match project canon; DTOs (if any) use `deny_unknown_fields`.
- [I-9] **Observability is canonical**:
  - `/metrics`, `/healthz`, `/readyz` present; golden series include latency histograms, inflight gauges, cache hit/miss, and
    `edge_rejects_total{reason in [rate_limit, timeout, decompress_cap, body_cap, not_allowed, amnesia, invalid_range, etag_invalid]}`.
- [I-10] **Deterministic failure posture**:
  - On quota/timeout/invalid, return 429/503/4xx deterministically; never “best-effort” partials.
- [I-11] **Pack integrity at mount**:
  - PMTiles/MBTiles verified (size, magic, optional signature/hash); unreadable or mismatched → instance not ready.

## 2. Design Principles (SHOULD)
- [P-1] **Offline-first ergonomics** (packs for dev/demos; flip to live via allow-list).
- [P-2] **Minimal surface area** (serve bytes; no discovery, indexing, business logic).
- [P-3] **Degrade first** (shed early and loudly; never serve corrupt/partial).
- [P-4] **Determinism over cleverness** (ETag-gated fills; no speculative writes).
- [P-5] **Portable configs** (same config for Micronode/Macronode; only capacity knobs differ).

## 3. Implementation (HOW)
- [C-1] **Config (TOML)**:
  ```toml
  [edge]
  mode   = "offline"                      # "offline" | "live"
  packs  = ["./data/world.pmtiles"]       # offline sources
  allow  = ["fonts.gstatic.com","api.maptiler.com"]  # live allow-list

  [ingress]
  timeout_secs = 5
  max_inflight = 512
  rps_limit    = 500
  body_bytes   = 1048576
  decompress_max_ratio = 10

  [security]
  amnesia = true
  hsts = true
  cors = { allow_origins = [], allow_methods = [], allow_headers = [] }

  [retry]
  live_fill = { strategy="exp_backoff", base_ms=50, max_ms=800, max_retries=3, jitter=true, retry_on=[503, 504, "timeout"] }
````

* [C-2] **Tower/Axum hardening order**:
  `RequestBodyCap` → `SafeDecompress` → `RateLimit` → `ConcurrencyLimit` → `Timeout` → handlers.
  Emit: `http_requests_total{method,route,code}`, `request_latency_seconds_*`, `edge_inflight`, `edge_cache_{hits,misses}_total`, `edge_rejects_total{reason}`.
* [C-3] **Routes**:

  * `GET /edge/assets/{*path}` → pack/CAS with `Range` + conditional support.
  * `GET /healthz`, `GET /readyz`, `GET /metrics`.
* [C-4] **Live fill algorithm (with retry/backoff)**:

  ```
  if host(path) not in allow-list -> reject("not_allowed")
  if token_bucket.take().is_err() -> reject("rate_limit")
  attempt = 0
  loop {
    resp = https_get(path, timeout=5s)
    if resp.is_timeout() or resp.status in {503,504} and attempt < max_retries {
      sleep(jittered_backoff(attempt)); attempt += 1; continue
    }
    if !resp.ok -> reject("upstream")
    if !valid_strong_etag(resp) -> reject("etag_invalid")
    write_to_cas(resp.bytes, blake3(resp.bytes))
    return 200 with ETag/Cache-Control/Accept-Ranges
  }
  ```
* [C-5] **Readiness computation**:

  * `ready = true` only if all: quotas below shedding thresholds; packs mounted & verified (offline); CAS reachable (live);
    amnesia constraints satisfied; recent fill error rate under alert threshold.
* [C-6] **Error taxonomy → codes/metrics/logs**:

  * **Transient**: upstream timeout, 503/504, CAS backpressure → 503 (+retry if allowed); increment `reason`.
  * **Permanent**: not allow-listed, invalid range, body/decompress cap, invalid ETag → 4xx; increment `reason`.
  * Structured JSON logs: `{reason, route, code, duration_ms, bytes, etag?, hash?}`; optional audit hook enabled below.
* [C-7] **Security headers & CORS**:

  * Default deny; whitelists explicit; HSTS always-on if TLS.
* [C-8] **Audit hook (optional)**:

  * When `audit.enabled=true`, emit tamper-evident audit records for rejects and live fills
    (include content hash, source host, size, operator id if present).

## 4. Acceptance Gates (PROOF)

* [G-1] **Perf SLO**:

  * Local warm cache: p95 < 40 ms, p99 < 80 ms for `GET /edge/assets/...` at 10–50 parallel clients (includes `206` paths).
* [G-2] **Hardening conformance**:

  * Assert timeout/inflight/rps/body/decompress settings; negative tests: zip-bomb, over-large body, slowloris → 4xx/429/503 with correct `reason`.
* [G-3] **Amnesia gate**:

  * With `amnesia=true`, live fills rejected; no disk artifacts; `/readyz=degraded`; metrics/logs confirm `reason="amnesia"`.
* [G-4] **TLS/CORS/HSTS gate**:

  * E2E verifies TLS termination; HSTS present; CORS deny by default; explicit allow works per config.
* [G-5] **HTTP semantics gate**:

  * `If-None-Match` → `304` on match; invalid `Range` → `416`; multi-range disabled → `416` unless `edge.allow_multi_range=true`.
* [G-6] **Interop & hygiene gate**:

  * DTO `deny_unknown_fields`; frame/chunk/size limits enforced; same config boots on Micronode/Macronode.
* [G-7] **Fuzz/chaos**:

  * Header/value fuzz; malformed `Range`; corrupt gzip; under CPU/RAM stress `readyz` flips before saturation; no corrupt bytes served.
* [G-8] **Metrics contract gate**:

  * Scenario checks: warm hits bump `edge_cache_hits_total`; quota trips bump `edge_rejects_total{reason="rate_limit"}`; latency buckets move as expected.
* [G-9] **Audit hook gate (if enabled)**:

  * Live fill emits audit record with BLAKE3, size, source host; rejects emit reason-coded audit entries.

## 5. Anti-Scope (Forbidden)

* No business logic, token minting, or policy engines (use upstream services).
* No discovery/DHT/indexing (lives in `svc-dht`, `svc-index`, `svc-storage`).
* No unbounded buffering; no bypass of ingress caps.
* No disk persistence under amnesia; no speculative writes; no cache-invalidation API (owned elsewhere).

## 6. Threat Model (STRIDE-lite mapping)

| Threat            | Vector                           | Counter(s)               |
| ----------------- | -------------------------------- | ------------------------ |
| Slowloris/DoS     | Dribbled headers / infinite body | [I-1], [G-2]             |
| Zip bomb          | Over-decompression               | [I-1], [G-2]             |
| Cache poisoning   | Stale/forged content on fill     | [I-4], [I-5], [C-4]      |
| Abuse via anon    | Hot-path scraping                | [I-1], [I-7], rate/quota |
| CS leak / CORS    | Cross-origin fetch misuse        | [I-2], [C-7], [G-4]      |
| Integrity at rest | Corrupt packs/CAS                | [I-5], [I-11], [G-6]     |
| Silent partials   | Half-responses under pressure    | [I-10], [P-3], [G-7]     |

## 7. Scaling Notes (targets, not hard gates)

* Single instance on modest hardware should sustain **~3–5k RPS** of warm hits with p95 < 40 ms (range 1–64 KiB),
  assuming local CAS and kernel pins. Scale linearly via horizontal replicas; place CAS locally for hot assets.
* Expect **>90% hit ratio** in steady state for static packs; live fill retry budget capped (C-4) to prevent herding.

## 8. References

* Hardening, Scaling, and App Integration blueprints; Six Concerns; 12 Pillars; project observability canon.

```
