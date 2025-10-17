

---

title: API Surface & SemVer Reference
status: draft
msrv: 1.80.0
last-updated: 2025-10-17
audience: contributors, auditors, API consumers

# API.md — svc-edge

## 0. Purpose

This document captures the **public API surface** of `svc-edge`:

* The externally-consumed API is **HTTP + CLI**.
* The Rust surface is intentionally minimal (service crate; no stable library API).
* SemVer discipline: what changes break vs. extend.
* CI-enforceable via `cargo public-api` (for any exported symbols) and contract tests for HTTP.
* Serves as the “spec” for consumers, operators, and auditors.

---

## 1. Public API Surface

### 1.1 Crate type & Rust surface

**Crate type:** service (binary).
**Stable Rust API:** **None** (by design). The service exposes **HTTP endpoints** and a **CLI**.
Internal modules remain private. If a future SDK is needed, it will live in a separate crate.

> CI still runs `cargo public-api` to ensure we don’t accidentally leak unstable items.

**Expected `cargo public-api --simplified` (should be empty or near-empty):**

```text
# svc-edge exposes no stable library symbols by design.
# If any `pub` items appear here, they must be gated or explicitly documented.
```

### 1.2 HTTP API (service surface)

#### Endpoints

| Method | Path                   | Description                            | Auth | Notes                                          |
| -----: | ---------------------- | -------------------------------------- | ---- | ---------------------------------------------- |
|    GET | `/edge/assets/{*path}` | Serve bytes from pack/CAS or live fill | No   | Supports `Range`, `If-None-Match`; strong ETag |
|    GET | `/metrics`             | Prometheus metrics exposition          | No*  | Bind to localhost or protect at gateway        |
|    GET | `/healthz`             | Liveness                               | No   | Always 200 if process alive                    |
|    GET | `/readyz`              | Readiness                              | No   | 200 only when all readiness gates pass         |

* Operationally protected by bind or gateway.

#### Behavior contract — `/edge/assets/{*path}`

* **Caching & integrity**

  * `ETag`: **strong**, content-address (BLAKE3).
  * `Cache-Control`: implementation-defined (immutable assets recommended).
  * `Accept-Ranges: bytes`; `Range` supported for single-range; **multi-range disabled** by default.
* **Conditional requests**

  * `If-None-Match` → `304 Not Modified` on ETag match.
* **Partial content**

  * Valid byte range → `206 Partial Content` (+ `Content-Range`).
  * Invalid/unsatisfiable → `416 Range Not Satisfiable`.
* **Live fill (mode=live)**

  * Only from allow-listed hosts (HTTPS).
  * On success: content hashed, written to CAS, served with strong ETag.
  * Retries: exponential backoff + jitter; bounded by config.
* **Failure posture (deterministic)**

  * Queue full / rate/inflight caps → `429 Too Many Requests`.
  * Timeout → `503 Service Unavailable`.
  * Not allow-listed → `403 Forbidden`.
  * Amnesia write attempt → `503` and readiness degraded.
  * Body/decompress cap → `413/400` respectively.
  * All rejects increment `rejected_total{reason=...}` and include `X-Reason` header.

**Canonical headers**

* Request: `Range`, `If-None-Match`, optional `X-Corr-ID`.
* Response: `ETag`, `Accept-Ranges`, `Content-Range` (when 206), `X-Corr-ID`.

#### Examples

```bash
# Warm hit
curl -i http://127.0.0.1:8080/edge/assets/fonts/roboto.woff2

# Conditional
ETAG=$(curl -sI http://127.0.0.1:8080/edge/assets/tile.pmtiles | awk '/^ETag:/ {print $2}')
curl -i -H "If-None-Match: ${ETAG}" http://127.0.0.1:8080/edge/assets/tile.pmtiles

# Range
curl -i -H "Range: bytes=0-65535" http://127.0.0.1:8080/edge/assets/tile.pmtiles
```

### 1.3 CLI (process flags)

|                               Flag |   Alias | Type   | Description                       |                           |   |      |           |
| ---------------------------------: | ------: | ------ | --------------------------------- | ------------------------- | - | ---- | --------- |
|                  `--config <path>` |         | path   | Load config file (low precedence) |                           |   |      |           |
|                 `--bind <ip:port>` |         | socket | HTTP bind                         |                           |   |      |           |
|              `--metrics <ip:port>` |         | socket | Metrics bind                      |                           |   |      |           |
|                   `--mode <offline |  live>` |        | enum                              | Operating mode            |   |      |           |
|                    `--pack <path>` |         | repeat | Add pack path (offline)           |                           |   |      |           |
|               `--allow <hostname>` |         | repeat | Add allow-list host (live)        |                           |   |      |           |
|                      `--rps <num>` |         | u32    | Rate limit                        |                           |   |      |           |
|                 `--inflight <num>` |         | u32    | Max inflight                      |                           |   |      |           |
|                    `--body <size>` |         | size   | Body cap (e.g., 1MiB)             |                           |   |      |           |
|             `--decompress <ratio>` |         | u32    | Decompress ratio cap              |                           |   |      |           |
|          `--decompress-abs <size>` |         | size   | Absolute post-decompress cap      |                           |   |      |           |
|                   `--amnesia[=true | false]` |        | bool                              | RAM-only (no disk writes) |   |      |           |
|               `--hsts / --no-hsts` |         | flag   | HSTS on/off (if TLS)              |                           |   |      |           |
| `--strong-etag / --no-strong-etag` |         | flag   | Strong ETag on/off                |                           |   |      |           |
|   `--multirange / --no-multirange` |         | flag   | Multi-range on/off                |                           |   |      |           |
|                `--retry-base <ms>` |         | u64    | Live-fill retry base              |                           |   |      |           |
|                 `--retry-max <ms>` |         | u64    | Live-fill retry cap               |                           |   |      |           |
|                `--retry-count <n>` |         | u32    | Live-fill max retries             |                           |   |      |           |
|                 `--retry-on <val>` |         | repeat | 503/504/timeout                   |                           |   |      |           |
|           `--jitter / --no-jitter` |         | flag   | Enable/disable jitter             |                           |   |      |           |
|                `--log-format <json |  text>` |        | enum                              | Log format                |   |      |           |
|                `--log-level <trace |   debug | info   | warn                              | error>`                   |   | enum | Log level |

> Full semantics and defaults in `docs/CONFIG.md`.

---

## 2. SemVer Discipline

Because `svc-edge` is a **service crate**, SemVer primarily governs the **HTTP/CLI contract** (not library symbols).

### 2.1 Additive (Minor / Non-Breaking)

* Adding **optional** HTTP headers or metrics/labels.
* Introducing **new endpoints** under a distinct path (e.g., `/edge/debug/...`) default-off.
* Adding CLI flags with **safe defaults** (off/unchanged behavior).
* Expanding enums in JSON responses with `#[non_exhaustive]` semantics (when applicable).
* Adding new `reason` values to metrics **only when** server behavior remains backward-compatible (e.g., splitting an existing reason into two while still returning the same status).

### 2.2 Breaking (Major)

* Changing endpoint paths, required headers, or status code semantics.
* Renaming/removing CLI flags, or changing default values in a way that modifies behavior.
* Enabling multi-range by default (was off).
* Changing ETag semantics (e.g., strong→weak) or CAS hash function without compatibility window.
* Removing/renaming metrics relied on by SLOs (unless migration path provided).
* Any change that alters readiness/health semantics.

### 2.3 Patch

* Performance improvements, internal refactors, or bug fixes that do **not** change surface/behavior.
* Log field additions (non-breaking), spelling fixes, doc updates.

---

## 3. Stability Guarantees

* **MSRV:** 1.80.0 (pinned in workspace).
* **Rust surface:** not a supported public API (service crate).
* **HTTP & CLI contracts** are versioned by SemVer; behavioral changes must be reflected in `CHANGELOG.md`.
* **Unsafe:** forbidden unless explicitly justified (none planned).
* **Hashing & ETag:** BLAKE3 strong ETag is the default and part of the behavior contract; changes require a compatibility period.

---

## 4. Invariants (API-Relevant)

* `/edge/assets/*` always enforces **caps**: timeout ≤ 5s, inflight ≤ 512, RPS ≤ 500, body ≤ 1 MiB, decompress ≤ 10× and ≤ 10 MiB absolute.
* Deterministic failures: 304 / 206 / 416 / 429 / 503 / 403 as specified.
* **Ready means safe**: `200 /readyz` implies assets can be served within SLOs; otherwise `503`.
* **No ambient trust**: no token minting; allow-list governs live egress.
* **Observability contract**: required metrics and logs exist with stable names (see Observability doc).

---

## 5. Tooling

* `cargo public-api --simplified --deny-changes` (guard accidental symbol leaks).
* `cargo semver-checks` (optional) for library-like checks.
* Contract tests for HTTP: status codes, headers, range semantics, retry outcomes, `/readyz` gates.
* Snapshots: store **HTTP response fixtures** and `public-api` symbol listings under `docs/api-history/svc-edge/<version>/`.

---

## 6. CI & Gates

* CI **fails** if `cargo public-api` detects new public symbols (unless explicitly approved).
* CI **fails** if contract tests detect unauthorized behavior changes (HTTP codes/headers/fields).
* Any surface diff → **CHANGELOG entry required**.
* OpenAPI file (below) treated as an artifact; CI can diff it against prior.

---

## 7. Acceptance Checklist (DoD)

* [ ] HTTP contract tests pass (conditional, range, failure posture).
* [ ] `/healthz`, `/readyz`, `/metrics` present and correct.
* [ ] `rejected_total{reason}` emits all canonical reasons.
* [ ] CLI help reflects `docs/CONFIG.md` (flags & defaults).
* [ ] `cargo public-api` produces no unexpected exports.
* [ ] CHANGELOG updated for any behavioral change.
* [ ] `/docs/openapi/svc-edge.yaml` updated and committed.

---

## 8. Appendix A — OpenAPI (concise)

```yaml
openapi: 3.0.3
info:
  title: svc-edge
  version: "1.0.0"
servers:
  - url: http://localhost:8080
paths:
  /edge/assets/{path}:
    get:
      summary: Serve asset bytes from pack/CAS or live fill
      parameters:
        - name: path
          in: path
          required: true
          schema: { type: string }
        - name: Range
          in: header
          required: false
          schema: { type: string, example: "bytes=0-65535" }
        - name: If-None-Match
          in: header
          required: false
          schema: { type: string }
        - name: X-Corr-ID
          in: header
          required: false
          schema: { type: string }
      responses:
        "200":
          description: OK (full)
          headers:
            ETag: { schema: { type: string }, description: Strong ETag (BLAKE3) }
            Accept-Ranges: { schema: { type: string, example: "bytes" } }
        "206":
          description: Partial Content
          headers:
            Content-Range: { schema: { type: string } }
            ETag: { schema: { type: string } }
        "304":
          description: Not Modified
        "416":
          description: Range Not Satisfiable
        "429":
          description: Busy / shedding
          headers:
            X-Reason: { schema: { type: string, enum: [rate_limit] } }
        "403":
          description: Upstream host not allow-listed
        "503":
          description: Timeout / draining / amnesia violation
          headers:
            Retry-After: { schema: { type: integer } }
  /healthz:
    get:
      summary: Liveness
      responses:
        "200": { description: Alive }
  /readyz:
    get:
      summary: Readiness
      responses:
        "200": { description: Ready }
        "503": { description: Degraded / Not ready }
  /metrics:
    get:
      summary: Prometheus metrics
      responses:
        "200": { description: Exposition format }
```

---

## 9. Appendix B — Reason Codes (stable)

The following values are **part of the public behavior contract** and appear in logs/metrics and sometimes as `X-Reason`:

```
amnesia, not_allowed, body_cap, decompress_cap, ratio_cap,
timeout, rate_limit, invalid_range, etag_invalid, draining
```

Adding a new reason is **minor** if HTTP semantics do not change; renaming/removing is **major**.

---

## 10. Appendix C — Compatibility Notes

* **ETag/Hashing:** BLAKE3 chosen for performance; switching algorithms requires a **dual-publish window** (serve both ETags) or a major bump.
* **Multi-range:** Off by default to reduce complexity and attack surface. Enabling by default would be a breaking change.
* **Retry semantics:** Jitter/backoff are tunable; the **upper bound** on retry budget is part of SLO, not strict API.
* **Readiness:** Exact JSON keys in degraded payload are informative; presence of `200/503` semantics is the contract.

---

✅ With this API spec, `svc-edge` presents a **strict, testable contract**: HTTP & CLI behavior are explicit, SemVer-disciplined, and guarded by CI.
