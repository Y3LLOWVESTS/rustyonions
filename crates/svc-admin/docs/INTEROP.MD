````md
# 🔗 INTEROP.md — svc-admin

*Audience: developers, auditors, external SDK authors*  
*msrv: 1.80.0*

---

## 0) Purpose

This document defines the **interop surface** of `svc-admin`:

* Wire protocols & message formats (HTTP/1.1 + TLS).
* DTOs & JSON schemas used between:
  * operator browsers ⇄ `svc-admin` (UI/JSON),
  * `svc-admin` ⇄ RON nodes (admin plane),
  * `svc-admin` ⇄ svc-passport/IdP (passport mode).
* Any bus topics/events (currently *none*; reserved for future config/audit).
* Canonical test vectors for HTTP/JSON interactions.

`svc-admin` is **not** an OAP/1 endpoint and does **not** speak QUIC or custom binary protocols. It is a **pure HTTP/JSON admin-plane client + UI** that sits beside Omni-Gate and visualizes what nodes report.  

This document ensures all inter-crate and external integrations remain consistent with **GMI-1.6 Omni-Gate** and the global “HTTP-only admin plane” story.

---

## 1) Protocols & Endpoints

### 1.1 Ingress Protocols (into svc-admin)

**Supported:**

- HTTP/1.1 (Axum/Hyper) over:
  - `tcp` with optional TLS (via `tokio_rustls::rustls::ServerConfig`), controlled by `[tls]` config / `SVC_ADMIN_TLS_*`.  
- Optional Unix Domain Sockets (UDS) in future; if used, follow hardening rules (0700 dir, 0600 socket).  

**NOT supported:**

- Direct OAP/1 framing.
- QUIC/HTTP3.
- Raw gRPC.

Ingress consumers:

- Operator browsers (SPA, JSON API).
- Monitoring / load probes hitting `/healthz`, `/readyz`, `/metrics`.  

### 1.2 Outbound Protocols (from svc-admin)

`svc-admin` only talks to three classes of peers:  

1. **RON nodes (macronode/micronode/others)** over HTTP(S):

   - `GET /healthz`
   - `GET /readyz`
   - `GET /version`
   - `GET /api/v1/status`
   - `GET /metrics`
   - Future:
     - `POST /api/v1/reload`
     - `POST /api/v1/shutdown`

2. **Passport / IdP endpoints** (passport mode only):

   - JWKS / keyset URLs.
   - Discovery metadata endpoints (`.well-known`).
   - Optional introspection endpoints (if used by the auth library).

3. **Logging / audit sinks**:

   - stdout/stderr structured logs.
   - Future: HTTP/gRPC to `ron-audit` or external collectors.

**Transport invariants:**

- All outbound HTTP calls have explicit timeouts derived from config.  
- TLS to nodes & IdP must use Rustls via `tokio_rustls` only.  
- Request bodies are small JSON; admin API expects **body cap ≈ 1 MiB** and size/decompression caps on `/metrics` bodies (1–4 MiB decompressed).  

### 1.3 Exposed HTTP Endpoints (svc-admin surface)

**Operator-facing (inbound):**  

- SPA / static:

  - `GET /` — SPA index.
  - `GET /assets/*` — static JS/CSS/assets (when embedded).

- UI / admin JSON API:

  - `GET /api/ui-config` — UI config (theme, language, read-only). Driven by `[ui]` + `[ui.dev]` config.  
  - `GET /api/me` — identity and roles for current operator, plus `auth.mode`.  
  - `GET /api/nodes` — configured nodes (`id`, `display_name`, `environment`, `amnesia?`).  
  - `GET /api/nodes/{id}/health` — proxied `/healthz` from node (optional convenience; we may just derive health from status).  
  - `GET /api/nodes/{id}/ready` — proxied `/readyz`.  
  - `GET /api/nodes/{id}/status` — normalized `AdminStatusView` built from `/api/v1/status` and readiness.  
  - `GET /api/nodes/{id}/metrics/summary` — short-window summary over `/metrics`.  
  - `GET /api/nodes/{id}/metrics/facets` — facet metrics view when facet metrics detected.  

- Mutating admin actions (future; gated):

  - `POST /api/nodes/{id}/reload` — only when `actions.enable_reload = true`.  
  - `POST /api/nodes/{id}/shutdown` — only when `actions.enable_shutdown = true`.  

- svc-admin health:

  - `GET /healthz` — liveness.
  - `GET /readyz` — readiness gate.
  - `GET /metrics` — Prometheus metrics for svc-admin itself.

Auth modes for inbound: `auth.mode = "none" | "ingress" | "passport"`, with semantics defined in SECURITY.  

---

## 2) DTOs / Schemas

All interop is **HTTP + JSON** (and text/Prometheus for `/metrics`). No custom binary framing.  

### 2.1 Inbound DTOs (svc-admin responses)

The **canonical JSON shapes** exposed to browsers and tools are:

#### 2.1.1 `UiConfigDto` — `GET /api/ui-config`

Backed by `[ui]` and `[ui.dev]` config.  

```json
{
  "defaultTheme": "system",
  "availableThemes": ["system", "light", "dark", "red-eye"],
  "defaultLanguage": "en-US",
  "availableLanguages": ["en-US"],
  "readOnly": true,
  "dev": {
    "enableAppPlayground": false
  }
}
````

Rules:

* `availableThemes` must be a superset of the configured theme; unknown values are ignored by clients.
* `readOnly = true` **must reflect** `ui.read_only` OR enforced read-only mode (e.g., CLI `--read-only`).

#### 2.1.2 `MeResponse` — `GET /api/me`

```json
{
  "subject": "stevan@example.com",
  "displayName": "Stevan White",
  "roles": ["admin"],
  "authMode": "passport",
  "loginUrl": null
}
```

Semantics:

* `authMode` mirrors `auth.mode`.
* In `auth.mode="none"`, `subject` may be a synthetic `"dev-operator"`; `roles` may be `["dev"]`.
* In `auth.mode="passport"`, if unauthenticated, svc-admin SHOULD 401 with:

  ```json
  {
    "code": "unauth",
    "authMode": "passport",
    "loginUrl": "https://passport.example.com/login?...",
    "message": "Authentication required"
  }
  ```

  where `loginUrl` is constructed from `auth.passport_base_url`, `client_id`, and redirect target.

#### 2.1.3 `NodeSummary` — `GET /api/nodes`

```json
[
  {
    "id": "macronode_dev",
    "displayName": "macronode-dev",
    "environment": "dev",
    "amnesia": false
  }
]
```

* `id` is the TOML section key (`nodes.<id>`).
* `environment` is `"dev" | "staging" | "prod"`.
* `amnesia` is derived from node status payloads (when present); default `false` if unknown.

#### 2.1.4 `AdminStatusView` — `GET /api/nodes/{id}/status`

Normalized view of a node’s status, backed by node `/readyz` + `/api/v1/status`.

```json
{
  "profile": "macronode",
  "version": "0.3.2",
  "planes": [
    {
      "name": "gateway",
      "health": "ok",
      "ready": true,
      "restartCount": 1,
      "notes": "bound: 0.0.0.0:8090"
    },
    {
      "name": "overlay",
      "health": "fail",
      "ready": false,
      "restartCount": 3,
      "notes": "handshake failures; see node logs"
    }
  ],
  "amnesia": false
}
```

Invariants:

* `health` is `"ok" | "fail" | "unknown"`. Node-specific health fields are mapped into this.
* `ready` MUST respect node `/readyz`; svc-admin cannot mark `ready=true` when node says otherwise.
* `restartCount` is best-effort; if not reported by node, default to 0.

#### 2.1.5 Metrics DTOs — `GET /api/nodes/{id}/metrics/*`

These endpoints return **short-window, aggregated metrics** only. They are not substitutes for scraping node `/metrics` directly.

Sketch:

```json
{
  "windowSeconds": 300,
  "updatedAt": "2025-12-04T12:34:56Z",
  "planes": [
    {
      "name": "gateway",
      "httpRps": 123.4,
      "errorRate": 0.002,
      "p95LatencyMs": 45.6
    }
  ]
}
```

Facet variants group by facet label (`facet="kv"`, `"overlay"`, etc.), but remain JSON.

### 2.2 Outbound DTOs (peer expectations)

#### 2.2.1 Node admin APIs

`svc-admin` assumes node admin APIs have the following:

* `GET /healthz` → `200 OK` plain text or JSON; any non-2xx treated as “unhealthy”.
* `GET /readyz` → JSON with `ready: bool`, `missing: [...]`, `degraded: bool`. (Exact schema is node-specific but must be parseable enough to extract `ready`.)
* `GET /api/v1/status` → node-specific status JSON that `svc-admin` maps into `AdminStatusView`.
* `GET /metrics` → Prometheus text format (OpenMetrics acceptable). `svc-admin` parses a sub-set required for visualization.

If a node returns **extra fields**, svc-admin **ignores unknown fields** but never drops critical flags like `ready=false`.

#### 2.2.2 Passport / IdP

`svc-admin` treats IdP as a standard OIDC-ish issuer:

* JWKS endpoint returning `{"keys":[...]}`.
* Access tokens validating issuer/audience/exp/nbf.
* Optional `groups` / `roles` claim mapping to operator roles.

Interop here is delegated to the chosen Rust OIDC/JWT lib; `svc-admin` is responsible for **config correctness** only (issuer, audience, client_id, cookie name).

---

## 3) Bus Topics

Current status: **no direct kernel bus integration**.

* `svc-admin` does not:

  * read or write kernel bus messages,
  * manage actors via bus,
  * consume `KernelEvent` directly.

Future potential:

* Subscribe to `config.updated` or node/plane events via a bus or gRPC streaming service, but that would live in a **separate integration crate** (e.g., `svc-admin-bus-adapter`) to keep `svc-admin` simple and HTTP-only.

Interop invariant:

* Any future bus integration must not introduce **hidden control channels** that can perform actions not visible via the documented HTTP admin APIs.

---

## 4) Canonical Test Vectors

This section defines canonical interop tests as **HTTP exchanges** rather than binary frames.

### 4.1 Operator → svc-admin (`/api/nodes/{id}/status`)

**Request:**

```http
GET /api/nodes/macronode_dev/status HTTP/1.1
Host: admin.local:5300
Accept: application/json
X-Corr-ID: 7c50e5e6-b6af-4df7-9f6b-8d5b4b15df01
Cookie: ron_passport=eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9...
```

**Expected svc-admin behavior:**

1. Auth middleware:

   * Validates passport token (issuer/audience/exp/signature).
2. Upstream calls:

   * `GET https://macronode-dev.internal:8090/readyz` (with macaroon).
   * `GET https://macronode-dev.internal:8090/api/v1/status`.
3. Returns `AdminStatusView` JSON with `ready` derived from `/readyz`.

**Response (happy-path):**

```http
HTTP/1.1 200 OK
Content-Type: application/json
X-Corr-ID: 7c50e5e6-b6af-4df7-9f6b-8d5b4b15df01

{
  "profile": "macronode",
  "version": "0.3.2",
  "planes": [
    {
      "name": "gateway",
      "health": "ok",
      "ready": true,
      "restartCount": 0,
      "notes": null
    }
  ],
  "amnesia": false
}
```

If the node is unreachable, svc-admin returns e.g.:

```http
HTTP/1.1 502 Bad Gateway
Content-Type: application/json

{
  "code": "upstream_unavailable",
  "nodeId": "macronode_dev",
  "message": "Failed to reach node /api/v1/status",
  "details": { "kind": "connect", "timeoutMs": 2000 }
}
```

…and increments `svc_admin_upstream_errors_total{kind="connect"}`.

### 4.2 svc-admin → Node (`/metrics`)

**Request:**

```http
GET /metrics HTTP/1.1
Host: macronode-1.internal
Accept: text/plain
User-Agent: svc-admin/0.1
Authorization: Bearer <macaroon-or-token>
```

**Response (snippet):**

```text
# HELP http_requests_total Total HTTP requests
# TYPE http_requests_total counter
http_requests_total{route="/o/{addr}",method="GET",status="200"} 12345
http_requests_total{route="/o/{addr}",method="GET",status="500"} 10
# HELP ron_node_restarts_total Node-level restart counter
# TYPE ron_node_restarts_total counter
ron_node_restarts_total{plane="overlay"} 3
```

`svc-admin` must:

* respect decompression and size caps (e.g., fail if > 4 MiB decompressed).
* parse only necessary metrics, ignoring others safely.

If body too large:

* Count towards `svc_admin_rejected_total{reason="oversized"}` and `svc_admin_metrics_body_rejected_total{reason="too_large"}` (per OBSERVABILITY), and surface an error in charts.

### 4.3 Auth mode interop — `auth.mode="ingress"`

**Request via ingress:**

```http
GET /api/me HTTP/1.1
Host: admin.prod.ron-core.dev
X-User: jane@example.com
X-Groups: admin,ops
Accept: application/json
```

* svc-admin in `auth.mode="ingress"` maps `X-User` → `subject` and `X-Groups` → `roles`/`groups`.

**Response:**

```json
{
  "subject": "jane@example.com",
  "displayName": "jane@example.com",
  "roles": ["admin", "ops"],
  "authMode": "ingress",
  "loginUrl": null
}
```

---

## 5) Error Taxonomy

Interop-facing error categories for svc-admin:

* `400 BadRequest` — malformed query parameters, invalid node ID, etc.
* `401 Unauthorized` — missing/invalid auth token/headers (when `auth.mode != "none"`).
* `403 Forbidden` — operator authenticated but not allowed to perform the action (e.g. `shutdown`).
* `404 NotFound` — unknown node ID.
* `429 Busy` — internal backpressure when bounded channels are full.
* `502 Bad Gateway` — node-level issues:

  * unreachable,
  * timeout,
  * TLS handshake failures.
* `503 NotReady` — svc-admin itself not ready (readiness gate failed), but process alive.

These map to internal error kinds:

* `Error::Auth(AuthError::Unauth | AuthError::Forbidden)`
* `Error::Upstream(UpstreamError::Connect | Timeout | Parse | NotReady)`
* `Error::Busy` (for backpressure).

Interop invariants:

* Error responses SHOULD have a canonical JSON envelope:

  ```json
  {
    "code": "upstream_unavailable",
    "message": "Failed to reach node /metrics",
    "details": {
      "nodeId": "macronode_prod",
      "kind": "timeout"
    }
  }
  ```

* `code` string values (e.g., `"unauth"`, `"forbidden"`, `"busy"`, `"upstream_unavailable"`) are stable and aligned with metrics labels (e.g. `svc_admin_rejected_total{reason="unauth"}`).

---

## 6) Interop Guarantees

* **HTTP-only framing:**
  `svc-admin` **MUST NOT** invent new binary protocols. All interop is HTTP + JSON (and text/Prometheus).

* **No hidden channels:**
  All control flows go through documented admin endpoints; no backdoor RPCs or shell bridges.

* **SemVer discipline:**

  * Breaking changes to HTTP routes, JSON shapes, or error codes require a **major** bump (or minor bump pre-1.0 with clear migration notes).
  * New fields added as `Option<>`/nullable are backward compatible.

* **Backward compatibility:**

  * Unknown JSON fields from nodes (`/api/v1/status`) are ignored when mapping to `AdminStatusView`.
  * Unknown fields in `svc-admin` responses should be ignored by clients; never repurposed with incompatible semantics.

* **Size and timeout guarantees:**

  * Request body cap: ~1 MiB.
  * `/metrics` decompressed cap: 1–4 MiB (config/chosen in code).
  * Per-request timeouts enforced for node and IdP calls.

* **Auditability:**

  * Canonical HTTP test vectors live under `tests/vectors/interop/svc-admin/*.http` (planned).
  * Security-related flows (unauth, forbidden, upstream_unavailable) are logged with `action`, `result`, `corr_id`, `node_id`.

---

## 7) References

* **Blueprints & Docs:**

  * `docs/IDB.md` — svc-admin role & DTO sketch.
  * `docs/CONFIG.MD` — configuration schema & CLI/env interop.
  * `docs/SECURITY.MD` — inbound/outbound surfaces and threat model.
  * `docs/CONCURRENCY.MD` — HTTP-only framing & async invariants.
  * `docs/OBSERVABILITY.md` — correlation IDs & metrics used in interop flows.

* **Global Interop:**

  * GMI-1.6 Omni-Gate / OAP-1 specs (repo-level docs).
  * RON node admin-plane docs (svc-overlay, svc-gateway, macronode/micronode) for `/healthz`, `/readyz`, `/api/v1/status`, `/metrics`.

---

✅ With this INTEROP spec, `svc-admin`’s **wire-level contracts** are explicit:

* HTTP-only admin-plane proxy,
* stable JSON DTOs for UI/CLI/SDKs,
* predictable error codes and metrics,
* and clear boundaries with RON nodes and identity systems.

```
```
