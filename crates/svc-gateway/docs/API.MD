

---

````markdown
---
title: API Surface & SemVer Reference — svc-gateway
status: draft
msrv: 1.80.0
last-updated: 2025-10-03
audience: contributors, auditors, API consumers
---

# API.md

## 0. Purpose

This document captures the **public API surface** of `svc-gateway`:

- Snapshot of exported Rust items and the **HTTP wire contract**.
- SemVer discipline: what changes **break** vs **extend**.
- CI-enforceable via `cargo public-api` (Rust) and `openapi-diff` (HTTP).
- Acts as the “spec” for external consumers.

`svc-gateway` is a **service crate** (ingress). The Rust public API is intentionally small; the primary surface is **HTTP**.

```mermaid
flowchart LR
  C[Client] -->|HTTP/1.1| G[svc-gateway]
  C -. X-Corr-ID .-> G
  G -->|cap checks + limits| G
  G -->|validated request| D[Downstream]
  G -.->|JSON error + Retry-After| C
  style G fill:#0ea5e9,stroke:#0c4a6e,color:#fff
````

---

## 1. Public API Surface

### 1.1 Rust Surface (intended contract)

> Keep small, stable, and testable. This lets other crates embed or integration-test the service.

```text
# Target public items (contract — assert via cargo public-api)
pub mod config {
    pub struct Config;                 // serde-loadable, validated
}
pub mod readiness {
    pub struct ReadinessSnapshot;      // { degraded: bool, missing: Vec<&'static str> }
    pub const READY_KEYS: &'static [&'static str];
}
pub mod telemetry {
    pub fn init_tracing(service: &str) -> Result<(), Box<dyn std::error::Error>>;
}
pub mod metrics {
    pub struct Metrics;                // registered handles; Clone
}
pub mod service {
    pub async fn run(cfg: config::Config) -> anyhow::Result<()>;
    pub const VERSION: &'static str;   // build info (git, semver)
}
pub mod errors {
    #[non_exhaustive]
    pub enum Error {
        Busy, Timeout, Unauthorized, Forbidden,
        TooLarge, Unsupported, Degraded,
        Io(std::io::Error),
        Other(String)
    }
}
```

**Doctest examples (non-normative)**

````rust
/// Create config, initialize tracing, and run service.
/// ```no_run
/// use svc_gateway::{config::Config, telemetry, service};
/// telemetry::init_tracing("svc-gateway").unwrap();
/// let cfg = Config::default(); // load/merge env+file in real main()
/// tokio_test::block_on(service::run(cfg)).unwrap();
/// ```
````

Assert in CI:

```bash
cargo public-api --simplified --deny-changes -p svc-gateway
```

---

### 1.2 HTTP Surface (primary contract)

All endpoints are served over **HTTP/1.1 or HTTP/2 (TLS 1.3)**.

#### 1.2.1 Base Path

* The default base path is `/api/…`.
* Deployments MAY set `BASE_PATH` (e.g., `/gateway/v1`) **without breaking** the contract *provided* per-route semantics, headers, and error taxonomy remain unchanged.
* If base path changes, reflect it in OpenAPI and CHANGELOG (minor if additive, major if removing old path).

#### 1.2.2 Control & Telemetry

| Method | Path       |  Status | Body       | Notes                                                |
| -----: | ---------- | ------: | ---------- | ---------------------------------------------------- |
|    GET | `/healthz` |     200 | `"ok"`     | Liveness only. Never carries secrets.                |
|    GET | `/readyz`  | 200/503 | JSON       | `503` on degrade. See schema below.                  |
|    GET | `/metrics` |     200 | text/plain | Prometheus exposition. Usually bound to localhost.   |
|    GET | `/version` |     200 | JSON       | `{ "version": "...", "git": "...", "build": "..." }` |

`/readyz` JSON (stable):

```json
{ "degraded": false, "missing": [] }
```

When degraded:

```json
{ "degraded": true, "missing": ["policy_loaded"], "retry_after": 1 }
```

#### 1.2.3 Ingress API (edge rules applied)

* **Caps & Limits:** read/write timeouts **5s**; ≤**512** inflight; ≤**500 rps/instance**; body cap **1 MiB**; decode guard **10:1** and **≤ 8 MiB** post-decode.
* **Auth:** capability tokens (macaroons) on **mutations**; no sessions/cookies.
* **Deterministic errors (reason registry):**

| HTTP | Reason          | When                                |
| ---: | --------------- | ----------------------------------- |
|  400 | `malformed`     | Bad framing/headers/body            |
|  401 | `unauth`        | Missing/invalid capability token    |
|  403 | `forbidden`     | Capability present but insufficient |
|  413 | `body_cap`      | Raw body exceeds 1 MiB              |
|  413 | `decoded-cap`   | Post-decode > 8 MiB                 |
|  413 | `decoded-ratio` | Expansion ratio > 10:1              |
|  415 | `unsupported`   | Unsupported media/encoding          |
|  429 | `quota`         | Rate/DRR quota exceeded             |
|  429 | `busy`          | Queue full/backpressure             |
|  503 | `degraded`      | Readiness shedding writes           |

Error payload (stable):

```json
{ "code": 429, "reason": "quota", "retry_after": 2 }
```

#### 1.2.4 Required Headers (contract)

* **Request:**

  * `Authorization: Bearer <capability>` (mutations)
  * `Content-Encoding: gzip|deflate|br` (if used; subject to guards)
  * `X-Corr-ID: <uuid/ulid>` (optional; generated if absent)

* **Response:**

  * `X-Corr-ID` (echoed or created)
  * `Retry-After: <seconds>` (for 429/503)
  * `ETag: "b3:<hex>"` (on content-addressed reads, when applicable)

---

## 2. SemVer Discipline

We track **two** surfaces:

1. **Rust API** — Cargo SemVer
2. **HTTP API** — wire contract rules below

### 2.1 Rust API

**Additive (minor)**

* New `pub` items that don’t change existing signatures.
* New enum variants/fields only on `#[non_exhaustive]`.
* Optional config fields with safe defaults.

**Breaking (major)**

* Remove/rename public items; change signatures/bounds.
* Change error types/variants in breaking ways.
* Make previously `#[non_exhaustive]` items exhaustive.

**Patch**

* Impl/perf/doc changes with identical surface.

### 2.2 HTTP API

**Compatible (additive)**

* Add new routes or optional response fields.
* Add new error **reasons** (registry is **additive only**).
* Tighten validation against previously undefined behavior.

**Breaking**

* Remove/rename routes or change success schema.
* Change status code/`reason` mapping for existing failures.
* Remove or semantically alter a required header.

> **Reason Registry Stability:** Reasons are **append-only**; existing meanings never change. CI checks that any emitted `reason` is in the registry.

---

## 3. Stability Guarantees

* **MSRV:** 1.80.0 (workspace-pinned).
* **Unsafe code:** forbidden unless explicitly justified.
* **No internal type leaks**: public Rust signatures avoid deep stack types.
* **Docs required:** `#![deny(missing_docs)]` for public items; OpenAPI in `/docs/openapi/`.

---

## 4. Invariants

* **Service crate**: Rust surface stays minimal (config, readiness, telemetry, metrics, `run()`, `VERSION`).
* **HTTP**: Predictable degrade (`/readyz` truth), deterministic error taxonomy, `X-Corr-ID` propagation, `Retry-After` on backpressure, `ETag` on CAS reads.

---

## 5. Tooling

* **Rust surface**

  ```bash
  cargo install cargo-public-api
  cargo public-api --simplified -p svc-gateway > docs/api-history/svc-gateway/<NEW_VERSION>.txt
  ```

* **HTTP surface**

  * Store spec at `/docs/openapi/svc-gateway.yaml`.
  * Diff against previous:

    ```bash
    openapi-diff docs/openapi/svc-gateway-prev.yaml docs/openapi/svc-gateway.yaml --fail-on-incompatible
    ```

* **Optional**

  * `cargo semver-checks` for stricter Rust SemVer analysis.
  * `cargo doc` with doctests for `Config` and error taxonomy examples.

---

## 6. CI & Gates

* **Rust surface gate**

  ```yaml
  - name: public-api
    run: |
      cargo install cargo-public-api || true
      cargo public-api --simplified -p svc-gateway > public-api.txt
      git diff --exit-code -- public-api.txt || (echo "Public API changed; update API.md/CHANGELOG" && exit 1)
  ```

* **HTTP surface gate**

  ```yaml
  - name: openapi-diff
    run: |
      openapi-diff docs/openapi/svc-gateway-prev.yaml docs/openapi/svc-gateway.yaml --fail-on-incompatible
  ```

* **Reason registry sanity**

  ```yaml
  - name: error-reason-registry
    run: cargo test -p svc-gateway --test error_reasons_golden
  ```

---

## 7. Acceptance Checklist (DoD)

* [ ] Current **Rust** API snapshot stored under `/docs/api-history/svc-gateway/<version>.txt`.
* [ ] **OpenAPI** updated and diffed; base path documented if changed.
* [ ] CI gates green (`public-api`, `openapi-diff`, reason registry test).
* [ ] **CHANGELOG.md** updated for any surface changes.
* [ ] Docs/tests updated for new/changed APIs.
* [ ] Error taxonomy table matches implementation & metrics (`rejected_total{reason=…}`).

---

## 8. Appendix

### 8.1 Error Object (JSON Schema)

```json
{
  "$id": "https://rustyonions.dev/schemas/gateway-error.json",
  "type": "object",
  "required": ["code", "reason"],
  "properties": {
    "code":   { "type": "integer", "enum": [400,401,403,413,415,429,503] },
    "reason": { "type": "string",
                "enum": ["malformed","unauth","forbidden","body_cap","decoded-cap","decoded-ratio","unsupported","quota","busy","degraded"] },
    "retry_after": { "type": "integer", "minimum": 0 }
  },
  "additionalProperties": false
}
```

### 8.2 `/readyz` Response (JSON Schema)

```json
{
  "$id": "https://rustyonions.dev/schemas/gateway-readyz.json",
  "type": "object",
  "required": ["degraded","missing"],
  "properties": {
    "degraded": { "type": "boolean" },
    "missing":  { "type": "array", "items": { "type": "string" } },
    "retry_after": { "type": "integer", "minimum": 0 }
  },
  "additionalProperties": false
}
```

### 8.3 Minimal OpenAPI Skeleton (headers included)

```yaml
openapi: 3.0.3
info:
  title: svc-gateway API
  version: "1.0.0"
paths:
  /healthz:
    get:
      summary: Liveness probe
      responses:
        "200": { description: OK, content: { text/plain: { schema: { type: string } } } }
  /readyz:
    get:
      summary: Readiness probe (degrade-aware)
      responses:
        "200":
          description: Ready
          headers:
            X-Corr-ID: { $ref: "#/components/headers/XCorrID" }
          content:
            application/json: { schema: { $ref: "#/components/schemas/Readyz" } }
        "503":
          description: Degraded
          headers:
            X-Corr-ID: { $ref: "#/components/headers/XCorrID" }
            Retry-After: { $ref: "#/components/headers/RetryAfter" }
          content:
            application/json: { schema: { $ref: "#/components/schemas/Readyz" } }
  /metrics:
    get:
      summary: Prometheus metrics
      responses:
        "200": { description: Prometheus text }
  /version:
    get:
      summary: Build metadata
      responses:
        "200":
          description: Version JSON
          headers:
            X-Corr-ID: { $ref: "#/components/headers/XCorrID" }
components:
  headers:
    XCorrID:
      schema: { type: string }
      description: Correlation ID propagated end-to-end
    RetryAfter:
      schema: { type: integer, minimum: 0 }
      description: Seconds to wait before retrying
    ETag:
      schema: { type: string }
      description: Strong ETag (b3:<hex>) on content-addressed reads
  schemas:
    Error:  { $ref: "gateway-error.json" }
    Readyz:
      type: object
      required: [degraded, missing]
      properties:
        degraded: { type: boolean }
        missing:  { type: array, items: { type: string } }
        retry_after: { type: integer, minimum: 0 }
```

### 8.4 Examples

**Successful GET**

```bash
curl -H "X-Corr-ID: 01J..." https://gateway.example/api/objects/123
```

**429 with Retry-After**

```bash
curl -i -X POST https://gateway.example/api/put -d @big.json
# HTTP/1.1 429 Too Many Requests
# Retry-After: 2
# X-Corr-ID: 01J...
# { "code": 429, "reason": "quota", "retry_after": 2 }
```

**413 decoded-cap**

```bash
# Response: 413 + { "code": 413, "reason": "decoded-cap" }
```

---

## 9. History

* **1.0.0 (draft)** — Minimal Rust surface (`Config`, `run`, `Metrics`, `ReadinessSnapshot`, `init_tracing`, `VERSION`), stable HTTP control endpoints, deterministic error taxonomy/headers, base-path note, and CI gates for Rust+HTTP surfaces.

```
---

