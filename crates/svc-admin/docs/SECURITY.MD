
````markdown
---
title: Security Notes — svc-admin
crate: svc-admin
owner: Stevan White
last-reviewed: 2025-12-04
status: draft
---

# Security Documentation — svc-admin

This document defines the **threat model**, **security boundaries**, and **hardening requirements** specific to `svc-admin`.  
It complements the repo-wide Hardening, Interop, and RON-CORE blueprints and the svc-admin IDB, Config, and Concurrency docs.

`svc-admin` is an **operator-facing admin GUI** over the RON-CORE admin plane. It amplifies power, so it must be treated as a **high-value, high-risk** component: compromise here can reveal topology, metrics, and (if enabled) trigger admin actions on nodes.

`svc-admin` is **not** a full identity provider. Multi-user auth is handled by upstream identity systems (SSO / IdP / svc-passport + ingress). svc-admin consumes identity and **enforces policy**, but does not store passwords or invent its own user database.

---

## 1) Threat Model (STRIDE)

### 1.1 STRIDE Matrix

| Category | Threats                                                                                   | Relevant in `svc-admin`? | Mitigation (current / planned)                                                                                                                                                                                                                                                                                   |
|----------|-------------------------------------------------------------------------------------------|---------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **S**poofing | Client impersonation; fake operator; fake node endpoints                                     | **Yes**                  | TLS (tokio-rustls) for svc-admin; TLS to nodes; macaroon/token auth for node calls; deploy behind SSO/identity proxy; `auth.mode` (`none` \| `ingress` \| `passport`) with ingress headers or validated tokens; no local password DB; rely on svc-passport / IdP for user identity and session management.   |
| **T**ampering | Manipulated HTTP traffic; modified config; UI tampering via XSS/CSRF                         | **Yes**                  | HTTPS; HSTS at ingress; strict config file perms; SPA served with CSP, XFO, and `X-Content-Type-Options`; no `eval`/dynamic script injection; React-encoded HTML; CSRF defenses if svc-admin ever issues its own cookies; preference for stateless APIs behind an authenticated ingress/proxy.                   |
| **R**epudiation | “I didn’t click that” for admin actions; missing audit trail                                  | **Yes**                  | Structured JSON logs with operator identity (from SSO/headers/claims), node ID, action, and `corr_id`; designed integration with `ron-audit` or audit sink; log-level + retention policy; non-read-only actions explicitly logged; SPA surfaces “who you are” (subject/roles) for operators.                     |
| **I**nformation Disclosure | Node topology leaks; metrics containing sensitive labels; token or credential exposure         | **Yes**                  | Do not log secrets; redact sensitive headers; short-horizon in-memory metrics only; no long-term TSDB by default; optional redaction rules for known-sensitive label keys; TLS for svc-admin and nodes; auth failures/logs include node ID and reason, not raw token; config + secrets kept on least-privileged FS. |
| **D**enial of Service | Flooding svc-admin; abusing `/metrics` fanout; slow clients/polls; expensive auth checks | **Yes**                  | Concurrency caps; body/size caps; per-request timeouts; rate-limiting via tower-http or ingress; bounded metrics buffers; backpressure on internal channels; cheap auth path (no blocking I/O in middleware); bounded timeouts and retries for metrics and any auth-key/JWKS refresh.                           |
| **E**levation of Privilege | Viewer gaining mutating powers; over-broad macaroons; bypass of auth mode              | **Yes**                  | Read-only by default; config-gated actions (`actions.enable_*`); least-privilege macaroons; `auth.mode` + roles/claims enforced at handler layer for mutation endpoints; role separation (viewer/admin) delegated to SSO/svc-passport groups; no local RBAC DB; invariants: no unauthenticated mutating endpoints. |

### 1.2 Scope & Non-scope

- We **assume nodes are honest** about their own status and metrics; svc-admin visualizes what nodes report.
- We **do not** attempt to protect against:
  - a malicious operator with full access to svc-admin’s host/OS,
  - an attacker with arbitrary code execution on the host.
  Those are treated as infrastructure compromises.

---

## 2) Security Boundaries

### 2.1 Inbound

svc-admin receives:

- HTTP(S) requests from **operator browsers**:

  - `GET /` — static SPA (admin dashboard UI).
  - `GET /api/ui-config` — UI configuration (themes, languages, auth mode, etc.).
  - `GET /api/nodes` — configured node list (IDs, display names, env tags).
  - `GET /api/nodes/{id}/health` — proxied `/healthz` from node.
  - `GET /api/nodes/{id}/ready` — proxied `/readyz`.
  - `GET /api/nodes/{id}/status` — proxied `/api/v1/status`.
  - `GET /api/nodes/{id}/metrics/summary` — derived summary over `/metrics`.
  - `GET /api/nodes/{id}/metrics/facets` — facet metrics view when facet metrics are detected.
  - Future **mutating** endpoints (guarded by config + auth):

    - `POST /api/nodes/{id}/reload`
    - `POST /api/nodes/{id}/shutdown`
    - Future: plane-level actions like “drain overlay listeners” / “self-test” etc.

- svc-admin’s own health and metrics:

  - `GET /healthz` — liveness.
  - `GET /readyz` — readiness gate (fail-closed when invariants break).
  - `GET /metrics` — svc-admin’s own Prometheus metrics.

**Auth & identity modes:**

- `auth.mode = "none"`:

  - Dev-only; svc-admin enforces no operator identity beyond network boundaries.
  - Must only be used on localhost or locked-down environments.

- `auth.mode = "ingress"`:

  - svc-admin trusts headers set by an upstream auth/ingress (e.g., `X-User`, `X-Groups`).
  - No local session or password handling; identity is provided by SSO / svc-passport at the edge.

- `auth.mode = "passport"`:

  - svc-admin validates a token (e.g., JWT / passport) per request:
    - verifies issuer, audience, expiry, and signature using cached keys/JWKS.
    - derives roles/permissions from token claims.
  - No local password DB; tokens are minted by svc-passport / IdP.

svc-admin **does not expose** arbitrary “debug” or dev-only endpoints without explicit config gating. Any “playground” or dev tools (e.g., app-plane console) are hard-gated behind `ui.dev` settings and non-prod environments.

### 2.2 Outbound

svc-admin calls:

- **RON nodes** over HTTP(S):

  - `/healthz`, `/readyz`, `/version`, `/api/v1/status`, `/metrics`.
  - Future: `/api/v1/reload`, `/api/v1/shutdown`, other documented admin endpoints.

- **Logging / audit sinks**:

  - stdout/stderr for structured logs.
  - future: HTTP/gRPC to `ron-audit` service or centralized log collector.

- **Passport/IdP endpoints (passport mode only, if configured)**:

  - Optional JWKS / keyset URLs for token validation.
  - Optional discovery metadata endpoints.

svc-admin does **not** talk directly to databases, K/V stores, or external message buses.

### 2.3 Trust Zone

- `svc-admin` is expected to run in a **trusted operator zone**:

  - same cluster / VPC / host network as RON nodes, or
  - tightly controlled operator subnet / VPN.

- The UI is **not meant for anonymous internet users**:

  - even dev deployments should be:
    - bound to localhost, or
    - behind an authenticated gateway (VPN, SSO, bastion).

### 2.4 Assumptions

- Nodes enforce their own:

  - TLS, auth, quotas, and `/readyz` semantics.

- Kernel invariants hold:

  - Health/metrics/exporters behave as documented.

- File system, if used:

  - Config files are **per-service**, with correct ownership (`svc-admin` user) and restrictive permissions (`0600`/`0640`).

- Reverse proxy / ingress:

  - If present (nginx, Traefik, Envoy, etc.), is configured to:
    - terminate TLS with modern ciphersuites **or** pass-through to svc-admin TLS,
    - inject identity headers only after successful auth,
    - scrub/override conflicting identity headers from outside.

---

## 3) Key & Credential Handling

### 3.1 Types of keys / secrets

- **svc-admin TLS keys** (if terminating TLS itself with tokio-rustls):

  - X.509 certificate + private key.

- **Node credentials**:

  - macaroons / bearer tokens / API keys to talk to RON nodes’ admin plane.

- **Auth/Passport integration**:

  - Public keys / JWKS for issuer (not secret, but security-sensitive configuration).
  - Optional client secret if svc-admin ever acts as an OAuth client (avoid unless necessary).

- **(Future) Session / CSRF secrets**:

  - Only if svc-admin introduces any local cookie-based UX; by default we prefer stateless token/headers and external sessions.

### 3.2 Storage

- TLS keys:

  - Loaded from file paths specified in config (`tls.cert_path`, `tls.key_path`).
  - Files MUST be owned by the `svc-admin` OS user and **not world-readable** (`0600`).

- Node credentials:

  - Stored as **paths or secret references** in config when possible (e.g., `nodes.<id>.macaroon_path`).
  - In memory:
    - loaded once at startup or reload,
    - held in dedicated structs,
    - never logged.

- Passport / JWKS configuration:

  - Issuer URL, audience, and JWKS endpoint are configuration inputs.
  - Keys themselves are public but still integrity-sensitive; they should be cached in memory with a bounded refresh interval and not modified by arbitrary data.

- Amnesia mode:

  - When a node reports **amnesia**, svc-admin still retains its configuration and credentials unless it is itself started in an amnesia/ephemeral mode.
  - If a future `svc-admin` amnesia mode is introduced:

    - node credentials SHOULD be provided by env/secret mounts only,
    - config and secrets MUST NOT be persisted to disk beyond what’s strictly required.

### 3.3 Rotation & Revocation

- Node tokens/macaroons:

  - SHOULD be rotated **≤ 30 days**, controlled by ops / `ron-policy`.
  - Rotation via:
    - updating node config or secret store,
    - reloading svc-admin config,
    - ensuring nodes reject old tokens/macaroons.

- TLS keys:

  - Follow standard certificate rotation practices (cert-manager, ACME, etc.).

- Auth/Passport keys:

  - `svc-admin` relies on issuer/JWKS rotation from svc-passport or upstream IdP.
  - Refresh is **read-only**; stale keys affect login/validation but do not leak secrets.

### 3.4 Zeroization

- For secrets we do manage (e.g., node macaroons):

  - Use `zeroize::Zeroizing` or equivalent to reduce residual time in memory.
  - Avoid unnecessary copies (pass by reference where possible).

- We accept that not all copies (e.g., inside libs, OS buffers) can be fully controlled; the goal is to minimize lifetime and duplication.

---

## 4) Hardening Checklist (from Blueprint, adapted to svc-admin)

svc-admin MUST conform to the workspace Hardening blueprint, with the following **concrete expectations**:

- [ ] **5s timeout on outbound node calls**  

  - Node HTTP calls MUST use explicit timeouts (connect + read), defaulting to ~3–5s and tunable via config.
  - Metrics polling uses shorter timeouts (e.g., 3s) and bounded retries.

- [ ] **Concurrency cap on inbound requests**

  - HTTP server configured (via Hyper/Axum and OS settings) to limit concurrent connections (`max_conns`).
  - Optionally also limit concurrency per endpoint (e.g., metrics, status) if needed.

- [ ] **Optional RPS caps**

  - For internet-facing or multi-tenant operator environments, use tower-http or the ingress to enforce RPS caps (e.g., 500 RPS).

- [ ] **Request body cap: 1 MiB**

  - svc-admin expects only small JSON bodies (if any).
  - Use `RequestBodyLimitLayer` or equivalent at the Axum layer.

- [ ] **Decompression ratio cap and size caps**

  - If compression is enabled, enforce:
    - decompression ratio ≤ 10×,
    - absolute cap on decompressed `/metrics` bodies (1–4 MiB).

- [ ] **UDS sockets (if used)**

  - If svc-admin is ever exposed via Unix domain sockets:
    - socket dir `0700`, socket `0600`,
    - optional `SO_PEERCRED` checks to restrict peers.

- [ ] **No direct DB access**

  - svc-admin does **not** talk directly to DBs; it uses only admin-plane HTTP.

- [ ] **Chaos test / restart under load**

  - Under synthetic UI load:
    - restart svc-admin and ensure `/readyz` flips appropriately,
    - verify it recovers without leaking secrets or leaving stale state.

- [ ] **Auth mode sanity**

  - `auth.mode="none"` **must not** be used in production.
  - In non-`none` modes, mutating endpoints MUST require valid authenticated identity and proper roles/claims.

---

## 5) Observability for Security

### 5.1 Metrics

svc-admin SHOULD emit security-relevant metrics, e.g.:

- `svc_admin_rejected_total{reason="unauth"|"forbidden"|"oversized"|"timeout"}`  
- `svc_admin_auth_failures_total{scope="ui"|"node"|"passport"}`  
  - `scope="ui"`: operator auth errors (invalid token/headers).
  - `scope="node"`: node rejects svc-admin’s credentials.
  - `scope="passport"`: failures contacting issuer/JWKS.
- `svc_admin_tls_handshake_failures_total{peer="node"|"client"}`  
- `svc_admin_csrf_failures_total` (if/when CSRF layer exists)  
- `svc_admin_upstream_errors_total{kind="connect"|"timeout"|"parse"}`  

These metrics aid detection of:

- brute-force / credential misuse,
- misconfigured nodes or expired certs,
- broken auth flows,
- attempts to abuse size/time limits.

### 5.2 Logs

All logs MUST be structured (JSON) with at least:

- `service = "svc-admin"`
- `level`
- `timestamp`
- `corr_id` (per incoming HTTP request)
- `peer_addr` (client IP, where available/appropriate)
- `node_id` (for outbound node calls, if applicable)
- `action` (`"proxy_health"`, `"proxy_ready"`, `"proxy_status"`, `"proxy_metrics"`, `"reload"`, `"shutdown"`, `"auth_validate"`, etc.)
- `result` (`"ok"`, `"timeout"`, `"unauth"`, `"forbidden"`, `"error"`)

For authenticated operators, logs SHOULD include:

- `subject` (canonical user id / email / UUID from SSO / token),
- `roles` or `groups` (coarsely, e.g., `"admin"`, `"viewer"`), if available.

**Never log:**

- Raw `Authorization` headers or cookies.
- macaroons/tokens or decoded sensitive claims (beyond minimal subject/roles).
- TLS private keys or certificate material.
- Full raw `/metrics` payloads (only sizes/counts).

### 5.3 Health gates

- `/healthz`:

  - Basic liveness: process up, config parsed.

- `/readyz`:

  - Fail-closed when **critical invariants** break, e.g.:

    - config invalid or inconsistent,
    - metrics sampler panicking in a loop (hard errors),
    - auth subsystem misconfigured (e.g., `auth.mode="passport"` but issuer config invalid).

  - svc-admin may stay ready when **nodes** are unhealthy; per-node errors are surfaced in the UI instead of failing svc-admin itself.

---

## 6) Dependencies & Supply Chain

**Security-sensitive dependencies:**

- `tokio-rustls` — TLS for svc-admin (if not terminated by proxy).
- `hyper` / `axum` — HTTP server.
- `tower-http` — timeouts, body limits, compression, possibly rate-limits.
- `reqwest` — outbound HTTPS client to nodes and optionally passport/issuer endpoints.
- `serde` / `serde_json` — deserialization of node status, auth config, and JWKs.
- `prometheus` or similar — svc-admin’s own metrics export.
- `zeroize` (if used) — memory zeroization for secrets.
- Auth-related crates (if we implement local token verification), e.g.:

  - JWT / JOSE / OIDC client crates (final choice TBD, but must be mainstream and audited),
  - or a thin passport client library from our own workspace.

**Pinned versions:**

- All above are pinned at the **workspace root** `Cargo.toml` to avoid version skew or accidental multiple major versions.

**Supply chain controls:**

- `cargo-deny` enforced in CI:

  - denies known-vulnerable versions,
  - enforces license policy,
  - disallows unknown registries except explicit allowlist.

- SBOM:

  - At release time, svc-admin SHOULD include or reference a dependency list/SBOM (SPDX/CycloneDX) under `docs/sbom/` or a global SBOM for the workspace.

**No homegrown crypto:**

- svc-admin does not implement its own crypto primitives.
- All signing/verification uses standard, widely maintained libraries (via rustls, JOSE, etc.).

---

## 7) Formal & Destructive Validation

**Property tests:**

- Config:

  - property checks on config parser ensure:

    - invalid TLS paths / missing certs → fail-fast,
    - invalid enum values (`auth.mode`, `ui.theme`, `environment`) → reject or default safely.

- Node responses:

  - property tests for `/api/v1/status` mapping ensure:

    - `deny_unknown_fields` catches unexpected schema drift where appropriate,
    - malformed JSON does not crash svc-admin.

**Auth flows (unit / integration):**

- `auth.mode="none"`:

  - requests pass through; tests assert this is **only** enabled via config and emits a warning in logs for non-dev envs.

- `auth.mode="ingress"`:

  - missing/invalid identity headers → 401/403.
  - valid headers → request reaches handler with expected `subject` and `roles`.

- `auth.mode="passport"`:

  - valid token (correct issuer/aud/audience, not expired) → accepted.
  - expired, tampered, or wrong-audience token → 401/403.
  - token validation must **not** perform blocking network calls; JWKS refresh is tested separately.

**Fuzzing:**

- Metrics parser:

  - fuzz Prometheus text parsing to ensure no panics or blow-ups with weird content.

- Status parser:

  - fuzz JSON structures within size limits; ensure safe failures.

- (Optional) Token claims parsing:

  - fuzz around claim boundaries (times, numeric types) if we write wrappers around auth libs.

**Loom / concurrency checks:**

- As covered in Concurrency doc, but with an eye on:

  - metrics store + HTTP handler + shutdown,
  - `AuthState` read/write patterns (if we keep a local JWK cache).

**Chaos tests:**

- Under load:

  - break node connectivity and verify svc-admin returns `upstream_unavailable` without crashing.

- In passport mode:

  - temporarily break JWKS endpoint; ensure svc-admin uses cached keys and logs degraded state, but does not hang in auth middleware.

**TLA+ (optional/future):**

- For complex multi-node readiness visualizations or multi-issuer auth flows, we MAY define a small TLA+/PlusCal sketch. For now, out of scope.

---

## 8) Security Contacts

- **Maintainer:** Stevan White  
- **Security contact email:** `security@rustyonions.dev` (planned / placeholder)  
- **Disclosure policy:** See repo root `SECURITY.md` (responsible disclosure, coordinated fixes, embargo windows).

---

## 9) Migration & Upgrades

- Any **breaking change to auth or credential handling** MUST:

  - be treated as a **major version bump** for svc-admin,
  - be called out explicitly in `CHANGELOG.md` and `docs/UPGRADING.md`.

Examples:

- Introducing or changing `auth.mode` semantics.
- Switching from ingress-only auth to mandatory passport validation for certain deployments.
- Changing how node credentials are stored (e.g., moving from inline config to secret store references).

- **Deprecations:**

  - If a config key is deprecated (e.g., `insecure_http`), keep a compatibility path for ≥ 1 minor release, log a warning, and document removal targets.

---

## 10) Mermaid — Security Flow Diagram (REQUIRED)

```mermaid
flowchart LR
  subgraph Operators
    UI[Browser (Operator)]
  end

  UI -->|HTTPS + IdP/SSO / svc-passport| A[svc-admin<br/>auth.mode=none/ingress/passport]

  subgraph Nodes[RON Nodes]
    N1[macronode]
    N2[micronode]
  end

  A -->|TLS + macaroon/token| N1
  A -->|TLS + macaroon/token| N2

  A -.->|Reject unauth/oversized/timeout| ERR[HTTP Error + Security Metrics]

  style A fill:#b91c1c,stroke:#7f1d1d,color:#fff
````

**Text description:**
Operator browsers authenticate via SSO / svc-passport and reach svc-admin over HTTPS. Depending on `auth.mode`, svc-admin either trusts ingress headers or validates tokens itself. svc-admin calls RON nodes’ admin APIs over TLS using macaroons/tokens scoped to least privilege. Unauthorized, oversized, or timed-out requests are rejected with HTTP errors and increment appropriate security metrics.

---

```
```
