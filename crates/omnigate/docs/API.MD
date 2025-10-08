---

title: API Surface & SemVer Reference — omnigate
status: draft
msrv: 1.80.0
last-updated: 2025-10-07
audience: contributors, auditors, API consumers
-----------------------------------------------

# API.md — omnigate (impl: `svc-gateway`)

## 0. Purpose

This document captures the **public API surface** of the gateway service:

* Snapshot of exported **Rust items** (if any), **CLI flags**, and **HTTP endpoints**.
* SemVer discipline (what is breaking vs. additive).
* CHANGELOG alignment and CI enforcement (`cargo public-api`, diff gates).
* Serves as the **spec** for external consumers.

> Note: omnigate is a **service crate**. Its primary public API is **HTTP** (+ CLI). The Rust surface is intentionally minimal and not intended for downstream linkage.

---

## 1. Public API Surface

### 1.1 Rust surface (bin-oriented)

The binary does not expose a stable library API. Internal modules are `pub(crate)` unless required by integration tests. The only intended “public” Rust boundary is the **binary entrypoint**.

If/when a small helper library is exposed (e.g., types for tests), document it here and snapshot with `cargo public-api`.

#### Current (expected) `cargo public-api --simplified` output

```text
# omnigate is a binary crate; no stable public items are exported.
# If a companion lib target exists, paste its symbol list here.
```

> CI will still run `cargo public-api` to ensure no unintended symbols leak.

### 1.2 CLI surface (user-facing)

```
omnigate
  --config <path>                 Load Config.toml (TOML/JSON)
  --bind <ip:port>                Ingress bind (default 127.0.0.1:9080)
  --metrics <ip:port>             Metrics bind (default 127.0.0.1:9090)
  --max-conns <num>               Max concurrent connections
  --read-timeout <dur>            e.g., 5s, 250ms
  --write-timeout <dur>
  --idle-timeout <dur>
  --tls                           Enable TLS
  --tls-cert <path>               PEM cert path (overridden by OMNIGATE_TLS_CERT_PEM)
  --tls-key <path>                PEM key path  (overridden by OMNIGATE_TLS_KEY_PEM)
  --uds <path>                    Unix Domain Socket path (optional)
  --quota-default-rps <num>
  --quota-burst-rps <num>
  --per-ip-rps <num>
  --per-cap-rps <num>
  --fq-quantum <size>             e.g., 64KiB
  --fq-weight-<CLASS> <num>       anon|authed|internal
  --downstream-timeout <dur>
  --strict-boot                   Fail boot if any downstream health check fails
  --log-format <json|text>
  --log-level <trace|debug|info|warn|error>
```

Environment variables mirror flags (prefix `OMNIGATE_…`). See `docs/CONFIG.md` for authoritative schema.

### 1.3 HTTP API surface

The gateway provides a small, stable ingress surface (versioned under **`/v1`**). It **brokers** requests to internal services (index/storage/mailbox/overlay/dht/passport) and enforces admission (quotas/DRR) and auth (macaroon capabilities).

#### Common

* **Scheme**: `https://` (recommended) or `http://` in dev.
* **Auth**: capability token in header (default `Authorization: Bearer <macaroon>`; header name configurable).
* **Correlation**: accepts `X-Corr-ID`, returns `X-Corr-ID`; emits `X-Request-ID` per hop.
* **Compression**: request decompression is allowed with **ratio cap ≤ 10×** and **absolute body cap ≤ 1MiB**.
* **Content Types**: `application/json` (primary); opaque payloads proxied as-is for storage.
* **Errors**: JSON `{ "error": "<code>", "reason": "<human-friendly>", "corr_id": "<id>" }`.
* **Rate limiting / backpressure**: `429 Too Many Requests` with optional `Retry-After` seconds.

##### Standard endpoints (operational)

* `GET /healthz` → `200 OK` liveness.
* `GET /readyz` → `200 OK` ready, or `503` with body `{ready:false, degraded:true, missing:[…], retry_after:N}`.
* `GET /metrics` → Prometheus text (binds to localhost/private).

##### Application endpoints (representative; keep stable names/forms)

> These are **representative** path shapes used by the BFF hydration flows. Actual downstream composition may evolve; contract stability is maintained at the gateway boundary.

* **Objects (read-only via gateway):**

  * `GET /v1/o/{content_id}` → fetch object (proxy to storage).

    * Headers:

      * Request: `Range` (optional), `Accept`
      * Response: `Content-Length`, `ETag`, `Cache-Control`, `Accept-Ranges`
    * Status: `200`, `206`, `404`, `416`
* **Index lookups (naming/metadata):**

  * `GET /v1/index/{name}` → resolve to content_id / metadata (proxy to index).

    * Query: `?version=latest|<semver>`
    * Status: `200`, `404`
* **Mailbox (signals/messages):**

  * `GET /v1/mailbox/{channel}` → receive (long poll with timeout budget).
  * `POST /v1/mailbox/{channel}` → send (idempotency-key supported).

    * Status: `200`, `202`, `400`, `401`, `403`, `404`, `409`
* **Overlay/DHT service lookups (read-only via gateway):**

  * `GET /v1/dht/lookup?key=<hex>` → returns service records.

    * Status: `200`, `404`, `503(degraded)`

> **Mutations** (writes) can be proxied but may be **shed first** under degraded mode per readiness policy.

#### Headers

* **Request (accepted)**:

  * `Authorization` (macaroon) — or configured `OMNIGATE_MACAROON_HEADER`.
  * `X-Corr-ID` — correlation id (ULID/UUID). If missing, generated and echoed.
  * `X-Request-ID` — optional; gateway sets one if absent.
  * `Accept`, `Accept-Encoding`, `Range`, `If-None-Match` (proxied).
* **Response (always)**:

  * `X-Corr-ID`, `X-Request-ID`
  * Optional: `Retry-After` on 429/503
  * Storage reads: `ETag`, `Content-Range`, `Content-Length`, `Accept-Ranges`

#### Error codes (canonical)

| HTTP | `error` code             | When                                          |
| ---- | ------------------------ | --------------------------------------------- |
| 400  | `bad_request`            | Malformed input, invalid headers              |
| 401  | `unauth`                 | Missing/invalid macaroon                      |
| 403  | `forbidden`              | Capability lacks required caveats/scope       |
| 404  | `not_found`              | Object/index/mailbox missing                  |
| 413  | `body_cap`               | Body exceeds 1MiB cap                         |
| 415  | `unsupported_media_type` | Codec not allowed                             |
| 416  | `range_not_satisfiable`  | Invalid range                                 |
| 429  | `quota` or `busy`        | Admission/DRR throttling                      |
| 500  | `internal`               | Unhandled downstream error                    |
| 502  | `bad_gateway`            | Downstream non-success / protocol mismatch    |
| 503  | `degraded`               | Readiness failing; strict boot/reload failure |
| 504  | `downstream_timeout`     | Exceeded downstream deadline                  |

**Mapping to metrics:** every non-2xx increments `rejected_total{reason=<code>}` (see Observability).

---

## 2. SemVer Discipline

### 2.1 Service (HTTP) contract

* **Versioning strategy**: Path-based (`/v1/...`). Non-breaking **additive** changes (new optional query params/headers, new endpoints) **do not** bump major.
* **Breaking changes** require bump to `/v2`:

  * Removing/renaming endpoints or fields.
  * Changing semantics of existing fields/headers.
  * Tightening validation that rejects previously-accepted requests.
  * Changing default auth header name without alias window.

**Additive (Non-Breaking):**

* New endpoints under `/v1` that do not alter existing behavior.
* New response fields (must be optional/ignorable by clients).
* New `reason` values **only if** documented and added to metrics/logs lists.

**Patch-level:**

* Response formatting fixes, docstrings, perf improvements that don’t change wire behavior.

### 2.2 Rust (binary/lib) SemVer

* **Binary-only**: no stable Rust API is promised.
* If a companion lib is exposed in the future:

  * Mark enums `#[non_exhaustive]`, and structs with additive fields also `#[non_exhaustive]`.
  * Introduce traits with sealed pattern to prevent breaking downstream impls.

---

## 3. Stability Guarantees

* **MSRV**: 1.80.0.
* **No `unsafe`** in public-facing code (crate-level `#![forbid(unsafe_code)]`).
* **Config names** are part of operational API; renames require deprecation aliases ≥ 1 minor.
* **Headers**: `X-Corr-ID` and `Authorization` (or configured alternative) are stable.
* **Body limits & decompress caps** are part of the security contract (see Security/Config).

---

## 4. Invariants

* Gateway remains **stateless** (beyond counters/cache of verifier keys).
* **Admission first**: requests are admitted/rejected before heavy work (auth/decompress/downstream).
* **Deterministic rejection**: identical request shape under same load produces same status & `reason`.
* **No PII** in logs/labels; correlation IDs are synthetic.

---

## 5. Tooling

* **cargo public-api**: detect accidental library symbol exposure.

  ```bash
  cargo public-api --simplified --deny-changes
  ```
* **cargo semver-checks** (if companion lib appears later).
* **OpenAPI** (optional export): lightweight spec in `/docs/openapi/omnigate.v1.yaml` (kept in sync with this doc).

---

## 6. CI & Gates

* Pipeline runs:

  * `cargo public-api` (bin should have no exported items; companion lib, if any, must match snapshot).
  * Golden tests for **error mapping** (`status` ↔ `reason` ↔ metrics/logs).
  * Contract tests for `/healthz`, `/readyz`, representative `/v1` endpoints.
* Fails if:

  * Public API (lib) diff is detected without CHANGELOG entry.
  * New `reason` value added without doc & metric wiring.
  * OpenAPI diff (if present) lacks CHANGELOG entry.

---

## 7. Acceptance Checklist (DoD)

* [ ] Snapshot of current **Rust public API** stored under `/docs/api-history/omnigate/<rev>.txt` (even if empty).
* [ ] HTTP endpoints documented here and (optionally) in `/docs/openapi/omnigate.v1.yaml`.
* [ ] CI gate (`cargo public-api`) green.
* [ ] CHANGELOG updated for any surface change (CLI flags, headers, endpoint semantics).
* [ ] Integration tests cover new/changed endpoints & error mapping.

---

## 8. Appendix

### 8.1 Examples

**Fetch object (range read)**

```bash
curl -sS https://gw.example.com/v1/o/ba5e... \
  -H "Authorization: Bearer <cap>" \
  -H "X-Corr-ID: 01J9Z7..." \
  -H "Range: bytes=0-65535" -D-
```

**Index resolve**

```bash
curl -sS "https://gw.example.com/v1/index/app-config?version=latest" \
  -H "Authorization: Bearer <cap>"
```

**Mailbox send (idempotent)**

```bash
curl -X POST https://gw.example.com/v1/mailbox/alerts \
  -H "Authorization: Bearer <cap>" \
  -H "Idempotency-Key: 01J9Z7..." \
  -H "Content-Type: application/json" \
  --data '{"msg":"deploy-started"}'
```

### 8.2 Error body schema

```json
{
  "error": "quota",
  "reason": "rate limited (global)",
  "corr_id": "01J9Z7K9WZJ6Y8..."
}
```

### 8.3 References

* Rust SemVer: [https://doc.rust-lang.org/cargo/reference/semver.html](https://doc.rust-lang.org/cargo/reference/semver.html)
* cargo-public-api: [https://github.com/Enselic/cargo-public-api](https://github.com/Enselic/cargo-public-api)
* cargo-semver-checks: [https://github.com/obi1kenobi/cargo-semver-checks](https://github.com/obi1kenobi/cargo-semver-checks)

### 8.4 History

* `2025-10-07`: Initial v1 contract recorded; explicit canonical `reason` set; clarified `/v1` path versioning and readiness semantics.

---

**Summary:**
omnigate’s API is **HTTP-first**, small, and rigorously versioned. Rejections are deterministic and observable; additive growth is easy; breaking changes are explicit via `/v2`. The binary intentionally avoids exporting a public Rust surface; any future lib must obey `#[non_exhaustive]` and SemVer gates.
