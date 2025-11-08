### BEGIN NOTE - NOVEMBER 8 2025 - 10:10 CST

# ron-auth — Carry-Over Notes from **svc-passport** (Blueprint for Cohesive Build)


## 0) TL;DR (what to inherit + why)

* **Keep the contract tiny and boring.** svc-passport proved that a small, stable JSON API with strict limits, solid metrics, and fast negative paths is enough to get God-tier reliability and perf.
* **Reuse invariants verbatim.** Ed25519 only, audience gating, body caps, concurrency caps, structured errors, `/metrics`, `/healthz`, optional `/readyz`.
* **Delegate crypto.** ron-auth should **never** hand-roll crypto; **verify via svc-passport** (or ron-kms via the same trait boundary later). Batch whenever possible.

---

## 1) ron-auth’s role in RON-CORE (scope & boundaries)

**AuthN:** turn a request (passports, API keys, or future login artifacts) into a **Subject** with stable **Claims**.
**AuthZ:** evaluate **Subject × Action × Resource × Context** against policy (via `ron-policy`) and return **Permit | Deny** (with reasons).

**Not in scope (for first beta):**

* Full user directory/IdP, password flows, SSO/OIDC provider.
* Long-term session store with refresh tokens.
* Web3 identities (defer per pivot).

**In scope (beta):**

* **Passport-based AuthN** (single and batch).
* **Policy-driven AuthZ** (call `ron-policy` locally; no remote net I/O).
* **Decision endpoints**: `/v1/auth/whoami`, `/v1/auth/authorize`, `/v1/auth/authorize_batch`, `/v1/auth/introspect`.
* **Fast path caching** (optional, bounded TTL on positive verifies).
* **Ops plane**: `/healthz`, `/readyz`, `/metrics`.
* **Strict limits** and **consistent problems** (same style as svc-passport).

---

## 2) Hard requirements we should copy from **svc-passport**

* **Algorithms:** accept only `Ed25519` envelopes; reject everything else early.
* **Audience:** gated when `[security].require_aud=true` and aud must match configured audience(s) for ron-auth.
* **Body size:** enforce `limits.max_msg_bytes` (413).
* **Batch size:** enforce `limits.max_batch` with 413 on `/authorize_batch`.
* **Concurrency:** `ConcurrencyLimitLayer` on hot paths (env-tunable).
* **Error model:** small set of stable problem codes (`BadAlg`, `BadKid`, `BadAudience`, `Malformed`, `MsgTooLarge`, `BatchTooLarge`, `Denied`, `Internal`).
* **Metrics:** op counters + histograms (names below).
* **Config precedence:** `RON_AUTH_CONFIG` (inline TOML) > `RON_AUTH_CONFIG_FILE` > repo default.
* **JWKS handling:** don’t do crypto key logic in ron-auth—**call svc-passport `/v1/keys`** if you need to hydrate a local VK cache (optional). Prefer calling **svc-passport `/verify(_batch)`** instead of verifying locally.

---

## 3) Proposed API surface (beta)

```
GET  /healthz
GET  /readyz
GET  /metrics

POST /v1/auth/whoami
  -> Input: Envelope (same as svc-passport)
  -> Output: { sub, aud?, iat?, exp?, scopes?, roles?, device?, tenant? }
     (Claims derived from signed msg; if msg is arbitrary JSON, we map known fields conservatively.)

POST /v1/auth/introspect
  -> Input: Envelope or { token: <opaque> } (alias to Envelope for now)
  -> Output: { active: bool, alg: "Ed25519", kid, aud?, sub?, warnings?[] }

POST /v1/auth/authorize
  -> Input: { envelope: Envelope, action: "read|write|...", resource: "uri-or-id", context?: {…} }
  -> Output: { decision: "Permit"|"Deny", reason?: "PolicyRuleId", obligations?: {…} }

POST /v1/auth/authorize_batch
  -> Input: { items: [ { envelope, action, resource, context? }, ... ] }  OR bare array
  -> Output: [ { decision, reason?, obligations? }, ... ]
```

**Why this set:** Minimal surface that many services need immediately (identify subject, check access). Keeps envelopes as the single auth artifact (no second token layer yet).

---

## 4) DTOs (keep them boring & explicit)

* **Envelope**: identical to svc-passport (`{ alg, kid, msg_b64, sig_b64, aud? }`).

* **Subject/Claims** (internal struct):

  ```
  Subject { sub: String, aud: Option<String>, scopes: Vec<String>, roles: Vec<String>,
            tenant: Option<String>, device: Option<String>, iat: Option<i64>, exp: Option<i64> }
  ```

  Only populate fields that are **present in the signed message**. No synthesis.

* **Decision**: `{ decision: "Permit"|"Deny", reason?: String, obligations?: Map }`

---

## 5) Config (mirror svc-passport style)

```toml
[server]
bind = "127.0.0.1:5310"
admin_bind = "127.0.0.1:5311"

[limits]
max_msg_bytes = 4096
max_batch = 512

[security]
require_aud = true
accepted_audiences = ["ron-auth"]  # Default: service logical name

[verify]
# When true, call svc-passport /verify(_batch); when false (future), verify locally via ron-kms.
delegate_to_passport = true
passport_base = "http://127.0.0.1:5307"

[caching]
verify_positive_ttl_ms = 250     # Optional micro-cache for (kid,msg,sig) → true
verify_negative_ttl_ms = 50      # Optional micro-cache for stable negatives
jwks_ttl_s = 60                  # If we ever hydrate VKs locally
```

**Env precedence:**

1. `RON_AUTH_CONFIG` (inline TOML)
2. `RON_AUTH_CONFIG_FILE` (path)
3. Default file in crate

**Tunable envs** (fast overrides):
`RON_AUTH_VERIFY_CONCURRENCY`, `RON_AUTH_AUTHORIZE_CONCURRENCY`, `RON_AUTH_MAX_MSG_BYTES`, `RON_AUTH_MAX_BATCH`.

---

## 6) Inter-crate call patterns (keep cohesive)

* **ron-auth → svc-passport:**

  * Prefer `POST /v1/passport/verify_batch` with the **array form** for best throughput.
  * On startup (optional) hit `/v1/keys` once to warm any VK cache; respect `ETag` and `Cache-Control`.

* **ron-auth ↔ ron-policy:**

  * Build a small in-proc evaluator boundary: `fn authorize(subject:&Subject, action:&str, resource:&str, ctx:&Value) -> Decision`.
  * Keep policy bundles hot-reloaded if `ron-policy` supports it; otherwise load at boot.

* **svc-gateway / omnigate:**

  * They call ron-auth `/authorize(_batch)` before routing/dispatch, or annotate upstream request context with a `Permit`/`Deny` decision and reason ID.

---

## 7) Negative-path & limits (test matrix to port over)

* `alg != Ed25519` → 400 `BadAlg`
* empty `kid` → 400 `BadKid`
* missing required `aud` when `security.require_aud=true` → 400 `BadAudience`
* `issue` doesn’t exist here; **authorize** oversize request → 413 `MsgTooLarge`
* `authorize_batch` length > `limits.max_batch` → 413 `BatchTooLarge`
* malformed base64 in envelope → 400 `Malformed`
* valid envelope but **policy Deny** → 200 with `{ decision:"Deny", reason:"<RuleId>" }` (Deny is not an error)

---

## 8) Metrics (names & cardinalities)

**Counters**

* `auth_ops_total{op="whoami|introspect|authorize|authorize_batch", result="ok|deny|fail", alg="Ed25519|n/a"}`

**Histograms**

* `auth_op_latency_seconds{op}`
* `auth_batch_len` (for `/authorize_batch`)
* Optional: `auth_verify_delegate_seconds` (latency of svc-passport call), `auth_policy_eval_seconds`

**Gauges**

* `auth_ready` (0/1)
* Optional caches: `auth_verify_cache_items`

**Smoke sanity:** ensure at least one increment per op; verify `deny` shows when a synthetic Deny case is tested.

---

## 9) Observability & ops plane

* `/healthz` (liveness): always cheap and fast.
* `/readyz` (readiness): flip **true** after listeners are bound and (optionally) after first successful call to svc-passport `/v1/keys` or `/verify`.
* `/metrics`: Prometheus exposition.

**Logs:** structured with route, decision, rule id (on Deny), `kid`, and a request id. **No** secrets.

---

## 10) Caching (safe, minimal, optional)

* **Positive verify micro-cache** keyed by `(kid, sig_b64, msg_hash)` with **very small TTL** (100–250 ms).

  * Avoids repeat verifies within bursts; bounded memory (LRU).
* **Negative micro-cache** (optional, 50 ms) for obviously bad alg/kid/aud to shed load.
* **JWKS cache**: only if we add local verification later; for Beta keep **delegation** to svc-passport as the default.

---

## 11) Performance expectations (on laptop baseline)

* Delegated verify adds one local HTTP hop when auth sits next to passport. With loopback and small JSON the added latency is typically **<10–20 µs** beyond verify cost.
* Batch authorize should approach **verify_batch** per-item cost + policy eval (µs-level if policy is local and simple).
* Target (laptop, per core):

  * `authorize` ≈ **50–70 µs** typical (verify + trivial policy).
  * `authorize_batch_64` ≈ **40–50 µs/item**.

---

## 12) Test & bench plan (copy svc-passport patterns)

**Unit tests**

* Envelope validation: alg/kid/aud/base64.
* Subject extraction from `msg_b64` (known fields only).
* Policy decisions: Permit and Deny with clear reasons.

**Integration tests**

* **Delegate happy path**: valid envelope → `true` from svc-passport → `Permit` when policy allows.
* **Delegate deny**: force a policy Deny; keep HTTP 200 with `{decision:"Deny"}`.
* **Batch**: `{items:[…]}` and array forms; over-batch → 413.
* **Readiness flip** on boot; shutdown drains.

**Benches (Criterion)**

* `authorize_single` (with real delegate to local svc-passport).
* `authorize_batch_64`.
* Save baselines with `critcmp` compatibility as we did for svc-passport.

**Scripts**

* Mirror `scripts/smoke_passport.sh` as `scripts/smoke_auth.sh`:

  * spawn on fixed/dynamic port, probe `/healthz`, hit `/metrics`, exercise whoami/authorize/authorize_batch, include a Deny case, and stop the process.
  * Accept `RON_AUTH_CONFIG(_FILE)`; optional `AUTH_DYNAMIC_PORT=1`.

---

## 13) Security posture (carry-over)

* **Zero ambient authority:** decisions must be derived from **signed envelopes** (and policy), not from headers you don’t trust.
* **Strict rejects before crypto:** early checks for alg/aud/kid/size; only then call delegate verify.
* **Deny is not error:** keep Deny as a 200 with a clear reason; use HTTP errors for malformed or policy/engine failures only.
* **No secret leakage** in errors or logs.
* **Amnesia mode ready:** be able to run with caches disabled/ephemeral; don’t persist subject info by default.

---

## 14) Nice-to-haves beyond Beta

* **Session facade** (stateless): mint a short-lived **auth decision cookie** (HMAC’d) for web UIs to cut repeat authorizes—documented and opt-in.
* **PEP/PDP split:** optional mode where ron-auth (PDP) is called remotely by gateways (PEP) via gRPC/HTTP with batch.
* **Richer policy context:** geo/time/device risk signals; expose a `context` schema section.
* **Rate limiting/quotas** pre-auth (middleware) for abusive clients.
* **Admin plane**: `/admin/policy/reload`, `/admin/decision_trace` for debugging.

---

## 15) Reusable code/style conventions (keep consistency)

* **Axum 0.7** patterns exactly as in svc-passport: Router<()> + `Extension(Arc<_>)`; `DefaultBodyLimit::max` per route; `ConcurrencyLimitLayer`.
* **Problem envelopes**: same struct shape and code strings.
* **Base64:** `use base64::Engine;` with `general_purpose::STANDARD` (0.22 API).
* **ETag practice:** if any list endpoints appear (e.g., policy list), use **BLAKE3** over exact bytes + `If-None-Match`.
* **Docs:** each file uses `CODECOMMENTS.MD` style headers (RO:WHAT/WHY/INTERACTS/INVARIANTS).

---

## 16) Rollout order (small, safe steps)

1. Scaffold crate with ops plane, config loader, metrics registry, problem types, limits/concurrency, and empty handlers.
2. Implement **delegate verify** client (svc-passport) and whoami.
3. Add policy bridge and `authorize`.
4. Add `authorize_batch` + tests/benches.
5. Smoke script + metrics sanity.
6. `/readyz` + graceful shutdown.
7. README/runbook + baselines.

---

## 17) What to copy directly from **svc-passport**

* Config precedence loader logic (env inline/file/default) and test harness pattern.
* Problem type + helper `problem(StatusCode, code, msg)`.
* Body limit and concurrency env knobs.
* `/metrics` exporter binding and counter/histogram registration.
* The **smoke script** structure (spawn, wait_for_up, hit endpoints, teardown).

---

## 18) Integration expectations with the rest of RON-CORE

* **svc-gateway / omnigate:** call `/v1/auth/authorize(_batch)` early; cache **Permit** decisions briefly at the edge.
* **ron-policy:** load bundle on boot; provide a version in `/admin/attest` so operators can correlate policy rollouts.
* **ron-kms:** future switch (feature flag) to local verification—**do not change the public API** when enabling this.

---

## 19) Acceptance criteria for **ron-auth Beta**

* `cargo fmt && cargo clippy -D warnings && cargo test` green on macOS/Linux.
* `scripts/smoke_auth.sh --spawn` passes (whoami, authorize ok + deny case, batch ok, metrics present).
* Strict limits and negative tests pass.
* Metrics show all ops counters > 0 after smoke; histograms populated.
* Delegation path to svc-passport is resilient (timeouts, clear errors, small retries; no thundering herd).

---

## 20) Quick commands (reference)

**Dev sweep**

```
cargo fmt -p ron-auth
cargo clippy -p ron-auth --no-deps -- -D warnings
cargo test -p ron-auth
```

**Smoke (fixed)**

```
RON_AUTH_CONFIG_FILE=crates/ron-auth/config/default.toml \
crates/ron-auth/scripts/smoke_auth.sh --spawn
```

**Smoke (dynamic)**

```
AUTH_DYNAMIC_PORT=1 RON_AUTH_CONFIG_FILE=crates/ron-auth/config/default.toml \
crates/ron-auth/scripts/smoke_auth.sh --spawn
```

**Benches**

```
cargo bench -p ron-auth -- --measurement-time 10 --sample-size 100
```

---

### Final note

svc-passport gave us the template: tiny surface, strict invariants, solid ops, and speed. ron-auth should look and feel the same—just swapping **“sign/verify”** for **“whoami/authorize”** while **delegating verification to svc-passport** and **localizing policy**. This keeps the core cohesive and makes future productionization (KMS, richer policy, PDP/PEP split) incremental and low-risk.

### END NOTE - NOVEMBER 8 2025 - 10:10 CST