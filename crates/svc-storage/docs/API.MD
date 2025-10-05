---

title: API Surface & SemVer Reference — svc-storage
status: draft
msrv: 1.80.0
last-updated: 2025-10-04
audience: contributors, auditors, API consumers
-----------------------------------------------

# API.md — svc-storage

## 0. Purpose

This document captures the **public API surface** of `svc-storage`:

* Snapshot of exported functions, types, traits, modules (Rust surface).
* **Service API**: CLI flags and **HTTP** endpoints (primary contract).
* SemVer discipline: what changes break vs. extend.
* CI-enforceable via `cargo public-api` (Rust) and OpenAPI diff (HTTP).
* Acts as the “spec” for external consumers of this crate.

`svc-storage` is a **service crate**. The **HTTP surface is the API**; the Rust library surface is intentionally minimal.

---

## 1. Public API Surface

### 1.1 Rust Surface (intentionally minimal)

`svc-storage` is shipped primarily as a binary. Any library module exists only to share code with the binary and is **not a supported public API**.

**Policy:** keep the Rust public API **empty** (or private via `pub(crate)`), so the service’s API contract remains the **HTTP** surface.

Generate with:

```bash
cargo public-api -p svc-storage --simplified --deny-changes
```

#### Current Surface (expected)

```text
# Expected: no public items (service API is HTTP-only).
# If any public items appear here, they must be justified and documented below.
```

> If a minimal Rust surface is required (e.g., for integration testing), it must be **explicitly marked unstable** via `#[doc(hidden)]` and guarded behind a non-default feature (e.g., `testing`).

---

### 1.2 CLI (Binary Flags)

The binary accepts flags (see `docs/CONFIG.md` for full schema). Stable flags include:

```
--config <path>
--bind <ip:port>
--metrics <ip:port>
--max-conns <num>
--read-timeout <dur>   # e.g., 5s
--write-timeout <dur>
--idle-timeout <dur>
--limits-max-body <size>
--limits-decompress-ratio <n>
--tls
--tls-cert <path>
--tls-key <path>
--uds <path>
--region <label>
--hedge-delay <dur>
--max-rps <num>
--max-inflight <num>
--log-format <json|text>
--log-level <trace|debug|info|warn|error>
```

Flags are **additive-stable**; renames/removals are **breaking** (see SemVer rules below).

---

### 1.3 HTTP Surface (Primary Contract)

All endpoints are **JSON or binary** over HTTP/1.1.
**Headers you should expect/emit:**

* Responses: `ETag: "b3:<hex>"`, `Accept-Ranges: bytes`, `Cache-Control`, `Vary: Accept-Encoding`
* Requests: `Range`, `If-Match`, `If-None-Match`, `Content-Encoding` (safe decompression caps enforced)
* Auth (privileged ops): `Authorization: Macaroon <token>`

#### 1.3.1 GET Object

```
GET /o/{b3}
```

* **Path param**: `b3` — `b3:<64-hex>` (BLAKE3-256; case-insensitive hex)
* **Query**: none (hints are out-of-scope; discovery lives elsewhere)
* **Request headers** (optional):

  * `Range: bytes=<start>-<end>` (inclusive)
  * `If-None-Match`, `If-Match` (operate on the **strong** ETag)
  * `Accept-Encoding` (client-accepts transfer compression; response may be compressed per policy)
* **Responses**:

  * `200 OK` — full object
  * `206 Partial Content` — range served
    Headers: `Content-Range`, `Content-Length`, `ETag`, `Accept-Ranges`
  * `304 Not Modified` — when `If-None-Match` satisfied
  * `404 Not Found` — unknown address
  * `416 Range Not Satisfiable` — bad range
  * `429 Too Many Requests` — backpressure
  * `503 Service Unavailable` — not ready/degraded (see `/readyz`)

**Notes**

* The **ETag equals** the canonical content address: `ETag: "b3:<hex>"`.
* Content is served in ~**64 KiB** streaming chunks (implementation detail, subject to tuning).
* Integrity is verified before serving and during streaming; mismatches produce **5xx** with `integrity_fail_total{reason="checksum"}` incremented.

**Example**

```bash
curl -v https://storage.example/o/b3:9fa1d2c3... \
  -H 'Range: bytes=0-65535' \
  -H 'X-Corr-ID: <ulid>'
```

---

#### 1.3.2 HEAD Object

```
HEAD /o/{b3}
```

* Same semantics as `GET`, but returns headers only (ETag, Content-Length) and supports `Range` to validate bounds.
* **Responses**: `200`, `206`, `404`, `416`, `429`, `503`.

---

#### 1.3.3 PUT Object (Idempotent by Address)

```
PUT /o/{b3}
```

* **Auth**: required (`Authorization: Macaroon …`)
* **Path param**: `b3` — address of **canonical bytes** (not compressed form).
* **Request headers** (optional):

  * `Content-Encoding: gzip|zstd` — transfer compression; server **safely** decompresses (≤10× ratio + absolute cap) before hashing/storing.
* **Body**: canonical bytes (or compressed transfer of canonical bytes).
* **Responses**:

  * `201 Created` — newly stored; `Location: /o/{b3}`
  * `200 OK` — already existed (idempotent)
  * `400 Bad Request` — malformed address
  * `401/403` — missing/invalid capability
  * `409 Conflict` — content does not match `{b3}` when verified
  * `413 Payload Too Large` — exceeds `max_body_bytes`
  * `429 Too Many Requests` — backpressure
  * `503 Service Unavailable` — degraded/unready

**Notes**

* Server verifies the decompressed (if any) body against `{b3}` before commit.
* Decompression caps enforced: **ratio ≤10×** and **absolute output cap** (configurable). Violations → `413` with counters bumped.

**Example**

```bash
curl -v -X PUT https://storage.example/o/b3:9fa1d2c3... \
  -H 'Authorization: Macaroon ...' \
  --data-binary @object.bin
```

---

#### 1.3.4 POST Object (Digest-on-Write Helper)

```
POST /o
```

* **Auth**: required (`Authorization: Macaroon …`)
* **Body**: canonical bytes (or compressed transfer body).
* **Behavior**: server computes BLAKE3 on canonical bytes; stores if absent; returns address.
* **Responses**:

  * `201 Created` — with body: `{ "address": "b3:<hex>" }` and `Location: /o/{b3}`
  * `200 OK` — already existed; same body
  * Other errors: as in `PUT /o/{b3}` (`401/403/413/429/503`)

**Notes**

* Prefer `PUT /o/{b3}` for strictly idempotent client-side workflows. `POST /o` is a convenience endpoint.

**Example**

```bash
curl -v -X POST https://storage.example/o \
  -H 'Authorization: Macaroon ...' \
  --data-binary @object.bin
```

---

#### 1.3.5 Admin: Repair Hints (optional feature `admin`)

```
POST /admin/repair
```

* **Auth**: required; admin-scoped capability.
* **Body** (JSON, optional):

  * `{ "address": "b3:<hex>" }` — request targeted check/repair of a specific object, or omit for background.
* **Responses**:

  * `202 Accepted` — repair enqueued
  * `401/403` — unauthorized
  * `429/503` — backpressure/degraded

> This endpoint **does not** expose discovery or listing; it only hints the existing repair subsystem. It is safe to disable in hardened deployments.

---

#### 1.3.6 Observability & Version

```
GET /metrics     # Prometheus exposition
GET /healthz     # liveness
GET /readyz      # readiness (fail-closed on writes/invariants)
GET /version     # { "service": "svc-storage", "version": "...", "git": "...", "features": [...] }
```

---

### 1.4 Error Model

All non-2xx responses (other than `304`) include a **machine-readable JSON** body:

```json
{
  "error": "busy|unauth|payload_too_large|decompress_cap|ratio_cap|timeout|not_found|conflict",
  "message": "<human readable>",
  "corr_id": "<ulid>"
}
```

* `Retry-After` header accompanies `429/503` where appropriate.
* `corr_id` echoes/propagates your `X-Corr-ID` or is generated server-side.

---

### 1.5 OpenAPI (source of truth)

An OpenAPI document is maintained at:

```
/docs/openapi/svc-storage.yaml
```

**CI gate:** PRs that change routes/status codes/headers must update the OpenAPI and pass an **OpenAPI diff** job (breaking vs. additive classification).

---

## 2. SemVer Discipline

SemVer applies to **two dimensions**:

1. **Rust library surface** (minimal by policy)
2. **HTTP API surface** (primary)

### 2.1 Additive (Minor / Non-breaking)

* New **endpoints** (e.g., `HEAD /o/{b3}`), **optional** request/response headers.
* New **fields** in JSON responses that are **optional** and have safe defaults.
* New **query parameters** that are optional and ignored if unknown.
* New **metrics** and **log fields**.

### 2.2 Breaking (Major)

* Removing or renaming endpoints/paths or **changing required parameters**.
* Changing semantics of existing status codes or header meanings.
* Removing JSON fields or making optional fields required.
* Changing body caps or decompression caps **downward** in a way that rejects previously valid traffic (policy exception may allow safety-driven decreases with major/minor depending on governance).

### 2.3 Patch-Level

* Doc-only updates.
* Performance improvements that do not change semantics or caps.
* Error messages text-only changes (reason codes unchanged).

---

## 3. Stability Guarantees

* **MSRV**: `1.80.0`.
* **HTTP**: stability of documented routes, status codes, and header semantics.
* **Rust**: keep public API empty; if exposing items, mark `#[non_exhaustive]` and document SemVer impact.
* **Unsafe**: forbidden unless justified; never part of public surface.
* **No leaking** internal types (e.g., `tokio::net::TcpStream`) over any stable boundary.

---

## 4. Invariants ↔ API Mapping

* **Content addressing**: `ETag: "b3:<hex>"` on `GET`/`HEAD`; `PUT /o/{b3}` verifies digest; `POST /o` returns digest.
* **Bounds & caps**: `413` for `max_body_bytes`, safe decompression enforced; `416` for bad `Range`.
* **Backpressure**: `429` with `Retry-After`; metric `rejected_total{reason="busy"}` increments.
* **Readiness**: use `503` for fail-closed **writes** and degraded states; `/readyz` truthfully reflects dependency state.
* **Auth**: capability tokens (macaroons) required on **PUT/POST** and admin endpoints; `401/403` on failure.

---

## 5. Tooling

* `cargo public-api -p svc-storage --simplified --deny-changes` (Rust surface)
* `cargo semver-checks` (optional)
* **OpenAPI diff** (e.g., `oasdiff`) to classify HTTP changes
* `cargo doc` + doc tests (examples in this file)
* Store API snapshots in: `/docs/api-history/svc-storage/{version}.txt` (Rust) and `/docs/openapi/history/` (HTTP)

---

## 6. CI & Gates

* **Public API gate**: Rust surface must remain empty (or acknowledged changes).
* **OpenAPI gate**: PR fails if HTTP changes are breaking and version bump/changelog not provided.
* **CHANGELOG alignment**: Any surface diff requires a CHANGELOG entry under **Added/Changed/Removed**.
* **Integration tests**: validate `GET`/`HEAD`/`PUT`/`POST` behavior and status codes, including `Range`, `If-*` headers, caps, and backpressure.

---

## 7. Acceptance Checklist (DoD)

* [ ] Current **Rust API** snapshot generated & stored (empty or justified).
* [ ] **OpenAPI** updated to match endpoints/headers/status codes.
* [ ] SemVer classification done; version bumped appropriately.
* [ ] CI gates pass (`cargo public-api`, OpenAPI diff).
* [ ] CHANGELOG updated for any API surface changes.
* [ ] Examples and docs updated (this file, README).
* [ ] `/metrics`, `/healthz`, `/readyz`, `/version` endpoints present and documented.

---

## 8. Appendix

### 8.1 Example cURL Recipes

**GET (range) with caching**

```bash
curl -v https://storage.example/o/b3:9fa1d2c3... \
  -H 'Range: bytes=0-65535' \
  -H 'If-None-Match: "b3:9fa1d2c3..."'
```

**HEAD (verify existence/length)**

```bash
curl -I https://storage.example/o/b3:9fa1d2c3...
```

**PUT (idempotent by address)**

```bash
curl -v -X PUT https://storage.example/o/b3:9fa1d2c3... \
  -H 'Authorization: Macaroon ...' \
  --data-binary @object.bin
```

**POST (digest-on-write helper)**

```bash
curl -v -X POST https://storage.example/o \
  -H 'Authorization: Macaroon ...' \
  --data-binary @object.bin
```

### 8.2 References

* Rust SemVer: [https://doc.rust-lang.org/cargo/reference/semver.html](https://doc.rust-lang.org/cargo/reference/semver.html)
* cargo-public-api: [https://github.com/Enselic/cargo-public-api](https://github.com/Enselic/cargo-public-api)
* cargo-semver-checks: [https://github.com/obi1kenobi/cargo-semver-checks](https://github.com/obi1kenobi/cargo-semver-checks)

---

**API Definition of Done:**
The Rust surface is empty (or intentionally, clearly documented) and guarded by CI; the HTTP contract (GET/HEAD/PUT/POST, observability, admin repair) is fully documented and versioned via OpenAPI; invariants (content addressing, caps, readiness, auth) are encoded in status codes and headers; consumers can rely on stable semantics across minor/patch releases.
