

title: Security Notes — svc-rewarder
crate: svc-rewarder
owner: Stevan White
last-reviewed: 2025-10-14
status: draft

---

# Security Documentation — `svc-rewarder`

This doc defines the **threat model**, **security boundaries**, **controls**, and **validation plan** for `svc-rewarder`, which deterministically computes reward distributions from sealed inputs and emits **idempotent**, **conservation-safe** settlement intents to the ledger.

**Posture:** zero custody • capability-gated • conservation-first • idempotent by construction • fail-closed under pressure.

---

## 0) Purpose & Canon Alignment

* **Role:** Deterministic reward compute → idempotent ledger intents.
* **Primary risks:** Unauthorized issuance, replay/double-spend, precision/rounding leaks, dependency compromise, input tampering, algorithmic DoS.
* **Six Concerns:** **ECON** (value safety), **PERF** (bounded cost), **SEC** (caps & custody), **OPS** (fail-closed), **GOV** (policy signatures), **DX** (clear contracts).
* **Pillars:** Economics & Wallets (P12).
* **Assume:** Kernel Bus/Metrics/Health/Config invariants; downstreams expose `/readyz` & quotas.

---

## 1) Threat Model (STRIDE) — with Residual Risk

| Cat         | Concrete Threats (svc-rewarder)                | Rel? | Mitigations                                                                                                                                            | **Residual Risk** |
| ----------- | ---------------------------------------------- | ---: | ------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------- |
| **S**       | Spoofed caller, forged internal client         |    Y | TLS 1.3 (rustls); optional **mTLS** for econ plane; macaroons w/ caveats (tenant, path, method, TTL, IP); UDS + `SO_PEERCRED` allow-list               | **Low**           |
| **T**       | Input/policy tampering; manifest/DB corruption |    Y | **Content addressing** (BLAKE3 CIDs); CAS invariants; sealed RO manifests; run commitment `(epoch_id, policy_hash, inputs_cid)`; atomic writes         | **Low**           |
| **R**       | Repudiation of runs/outputs                    |    Y | Structured JSON logs (`corr_id`, `tenant`, `run_key`, `policy_hash`); signed run manifests; append-only audit sink; token `nbf/exp`                    | **Low**           |
| **I**       | PII/secret leakage in logs/artifacts           |    Y | **Amnesia mode** (tmpfs), strict DTOs `deny_unknown_fields`, PII minimization, header redaction, `zeroize`                                             | **Low**           |
| **D**       | Flood/slow-loris/zip bombs; dependency stalls  |    Y | **5s** timeout; **512** inflight; **500 RPS**; **1 MiB** body; decompress ≤ **10×** + absolute cap; bounded queues; degrade `/readyz`; circuit breaker | **Med→Low**       |
| **E**       | Privilege escalation / cross-tenant access     |    Y | Fine-grained macaroon scopes; tenant isolation; deny-by-default routing; no ambient creds                                                              | **Low**           |
| **Pivot**   | Compromised rewarder mints ledger intents      |    Y | **Egress macaroons** scoped to `ledger.intent.write` with tenant+`run_key` caveats; ledger verifies **commitment** & idempotency                       | **Low**           |
| **Alg-DoS** | Pathological epoch → excessive CPU             |    Y | **Cost estimator**; **MAX_EPOCH_COST_MS** guard; force `dry_run` or reject on estimate breach; per-run CPU budget                                      | **Med→Low**       |

---

## 2) Security Boundaries (Surfaces)

**Ingress (HTTP/1.1 + TLS; optional UDS):**

* `POST /rewarder/epochs/{epoch_id}/compute`
  Body: `{ inputs_cid, policy_id, policy_hash, dry_run?, notes? }`
  Auth: macaroon (`rewarder.run`, tenant-scoped)
  Idempotent run-key: `hash(epoch_id || policy_hash || inputs_cid)`
* `GET /rewarder/epochs/{epoch_id}` → sealed manifest
* `GET /rewarder/policy/{policy_id}` → signed policy (audit)

**Egress (downstream):**

* **ron-ledger**: `POST /intents` (idempotency=run_key), scope `ledger.intent.write`
* **ron-accounting**: sealed snapshots (ro), scope `accounting.snapshot.read`
* **policy/registry**: signed policy fetch (ro)

**Trust Zones & Gateway Integration:**

* Default: **internal economics plane**, strongly recommend **mTLS**.
* If internet-exposed: place behind gateway/WAF (global RPS, slow-loris mitigation, IP allow-lists). Service still enforces **per-tenant** caps.
* **No custody** of user funds; only service signing key for attestations.

---

## 3) Data Classification & Lifetimes

* **Inputs:** Confidental (opaque refs). Persist **CIDs** + minimal metadata.
* **Policy:** Public/verifiable (signed & versioned).
* **Manifests:** Internal-Confidential (aggregate totals; avoid per-user values unless required).
* **Retention:** Policy window (e.g., 90d). **Amnesia=ON** → RAM-only; no disk.
* **Non-amnesia at-rest encryption:** Encrypt artifacts with **XChaCha20-Poly1305**; per-file unique nonce; key held in memory (Zeroizing) and rotated with service signing key.

---

## 4) Key & Credential Handling

* **Types:** TLS key/cert; **macaroon root/derivation keys** per-tenant; service **signing key** (Ed25519; optional PQ dual-sig); optional PQ-KEM material for hybrid TLS.
* **Storage & Rotation:** OS 0600; macaroons rotate ≤30d; TLS certs with overlap; publish verification keys before switch; amnesia keeps keys ephemeral.
* **Egress macaroon attenuation:** `tenant=T, scope=ledger.intent.write, path=/intents, method=POST, aud=ron-ledger, exp≤30d, max_uses=10000`.
* **Revocation:** Publish **key epoch** to deny-list; all services reject old epochs within 60s.
* **Zeroization:** wrap secrets with `zeroize::Zeroizing`.
* **Redaction:** never log `Authorization`, `Cookie`, tokens, salts.

---

## 5) Economic Invariants (Security-Critical)

* **Conservation:** `Σ payouts ≤ pool_total` (allow fees/rounding).
* **Non-negativity:** no negative payouts/balances.
* **Determinism:** same `(epoch_id, policy_hash, inputs_cid)` ⇒ same manifest & intents.
* **Idempotency:** replays with same run-key produce **no new** effects.
* **Precision:** integer minor units (`i128`), **checked arithmetic** only (`checked_add`, etc.); overflow ⇒ **quarantine**.
* **Order independence:** aggregation independent of input order.
* **Dust policy:** amounts < 1 minor unit accumulate in `residual` bucket; logged & surfaced.

---

## 6) Hardening Checklist (defaults)

* [ ] **Timeouts:** 5s/request; 2s downstream connect; attempt budget 10s; bounded retries with jitter.
* [ ] **Concurrency:** 512 inflight; bounded work queue; `/readyz` degrades under pressure.
* [ ] **Rate limits:** 500 RPS/instance (token bucket; burst 200); also per-tenant caps.
* [ ] **Payload caps:** 1 MiB request; decompress ratio ≤10×; post-inflate cap ≤8 MiB.
* [ ] **Headers:** HSTS; `X-Content-Type-Options: nosniff`; CORS off unless strict allow-list.
* [ ] **AuthZ:** macaroon scopes per route; no ambient creds; strict tenant isolation.
* [ ] **UDS mode:** dir `0700`, socket `0600`, `SO_PEERCRED` allow-list; disable net bind.
* [ ] **Build:** `#![forbid(unsafe_code)]`, `deny(warnings)`; reproducible CI builds.
* [ ] **Secrets:** via env/file/agent; never CLI args; redact in logs/metrics.
* [ ] **Chaos:** restart-under-load; dependency outage drills; slow-loris & zip-bomb tests.
* [ ] **Container:** read-only rootfs; `no-new-privileges`; drop Linux capabilities (keep only `CAP_NET_BIND_SERVICE` if needed).
* [ ] **Seccomp/AppArmor:** restricted profiles (deny `execve`, `ptrace`).
* [ ] **Resource limits:** bounded `ulimit -n`; cap per-peer connections.

---

## 7) Observability for Security

**Metrics (Prometheus):**

* `reward_runs_total{status="ok|quarantined|fail"}`
* `reward_compute_latency_seconds` (histogram)
* `rejected_total{reason="unauth|oversize|decompress_cap|schema|stale|invariant"}`
* `ledger_intents_total{result="accepted|dup|error"}`
* `readyz_degraded{cause="backpressure|dep_outage|quota"}`
* `tls_handshake_failures_total`
* `auth_failures_total{scope}`

**Logs (JSON):** `ts, level, service, tenant, corr_id, run_key, policy_hash, inputs_cid, reason, scope, peer_addr`.
**Tracing:** propagate `corr_id`; tag spans with `tenant`, `run_key`; record compute cost estimates.
**Health gates:** `/healthz` liveness; `/readyz` **fail-closed** with `cause`.

**Alert rules (examples):**

```promql
ALERT AuthFailuresSpike
  IF sum(rate(auth_failures_total[5m])) by (tenant) > 10
  FOR 10m
  LABELS { severity="high" }
  ANNOTATIONS { summary="Auth failures spike for {{ $labels.tenant }}" }

ALERT DoSCapTriggered
  IF sum(rate(rejected_total{reason=~"oversize|decompress_cap"}[5m])) > 50
  FOR 5m
  LABELS { severity="medium" }

ALERT ReadyDegraded
  IF max_over_time(readyz_degraded[10m]) > 0
  FOR 10m
  LABELS { severity="high" }
```

---

## 8) Interfaces & Validation (DTOs)

* **Serde** with `#[serde(deny_unknown_fields)]`.
* **Validation:** `epoch_id` format; CID length/type; `policy_hash` algo; integer bounds; `dry_run` boolean only.
* **Content-Type:** `application/json` only.
* **Compression:** accept gzip/zstd; enforce ratio caps; refuse nested archives.
* **Contract:** generate **JSON Schema** via `schemars` for all DTOs; publish `GET /openapi.json` and `GET /schema/compute.json`; CI validates examples.

---

## 9) Dependency & Supply Chain Controls

* Workspace-pinned versions; TLS via `tokio-rustls` 0.26.x.
* CI gates: `cargo-deny`, `cargo audit`, `cargo clippy --deny warnings`, `cargo test --locked`, SBOM at release, `cargo public-api` (if library surface).
* **Images:** scan with Trivy/Grype; block on criticals.
* **Provenance:** attach SLSA v1 provenance for releases (if using builders).
* No transitive feature creep (explicit `features`).

---

## 10) Formal & Destructive Validation Plan

* **Property tests:** conservation; idempotency; determinism (randomized input orders); integer overflow guards; dust behavior.
* **Fuzzing:** DTO decode & policy parser (structure-aware); decompression handlers (reject bombs); run-key collisions (smoke).
* **Concurrency (Loom):** single-writer commitment; no double-emit on races/retries; ready/not-ready under pressure.
* **Chaos & Soak:** dependency timeouts; backoff/jitter; circuit breaking; sustained RPS near cap; verify caps/latency stability.
* **Golden vectors:** canonical small/med/large epochs (include off-by-one rounding).
* **TLA+ (optional):** model `Compute → IntentEmit → Dedup` under crash/retry; assert `NoDoubleSpend`, `IdempotentRunKey`.

---

## 11) Authentication & Authorization (Capabilities)

* **Token:** macaroon (via passport/auth), tenant-bound.
* **Scopes:** `rewarder.run`, `rewarder.inspect`, `rewarder.quarantine`, `ledger.intent.write` (service-to-service).
* **Attenuation:** caveats for `tenant`, `epoch_id`, method/path, TTL, IP, `max_runs`.
* **Interop (legacy):** accept JWT from gateway **only at edge**; **convert to macaroon** internally with reduced scope & short exp; never forward JWT downstream.

---

## 12) Error Taxonomy (security-sensitive)

* `UNAUTHENTICATED` (invalid/missing token)
* `UNAUTHORIZED` (scope/caveat fail)
* `BAD_REQUEST` (schema/validation/decompress cap)
* `CONFLICT` (existing run with different commitment)
* `QUARANTINED` (invariant breach)
* `DEPENDENCY_UNAVAILABLE` (ledger/accounting)
* `INTERNAL` (unexpected; include `corr_id`)

Return minimal details; do not echo user input.

---

## 13) PQ-Readiness (practical now)

* **TLS:** prefer TLS 1.3; near-term **hybrid TLS** terminated at gateway that supports ML-KEM hybrid; svc-rewarder continues rustls TLS 1.3 behind it. Pin ciphersuites.
* **Attestation dual-sig:** optional `manifest_sig_ed25519` + `manifest_sig_dilithium` fields; CI verifies both when enabled.
* **Flag:** `REQUIRE_PQ_ON=true` for regulated tenants; reject if not satisfied.
* **HN/DL exposure:** inputs/manifests are not secrets, but adopt PQ progressively where feasible.

---

## 14) Incident Response & Forensics

* **Tripwires:** spikes in `auth_failures_total`, `rejected_total{oversize|decompress_cap}`, persistent `readyz_degraded`.
* **Immediate actions:** clamp rate caps; enable **quarantine-only** (compute manifests but **suppress** intent emission); rotate macaroon root epoch.
* **Forensics:** query manifests by `run_key`/`policy_hash`; diff against golden vectors; confirm ledger **dup** counters.
* **Targets:** Detect high-sev ≤ **5m**; mitigation started ≤ **15m**; normal ops restored ≤ **60m** (no degraded `/readyz`).
* **Disclosure:** follow repo `SECURITY.md`; provide attestation hash list & timeline.

---

## 15) Configuration Hardening (env / flags)

* `BIND_ADDR` (prefer UDS: `unix:///path/socket`)
* `MAX_INFLIGHT=512`, `MAX_RPS=500`, `REQ_BODY_MAX=1048576`, `DECOMP_RATIO_MAX=10`, `POST_INFLATE_MAX=8388608`
* `AMNESIA=on|off`
* `CAPS_KEYS_PATH` / agent socket (not CLI)
* `DOWNSTREAM_TLS=required|mtls`, `CLIENT_CA_PATH`
* `MAX_EPOCH_COST_MS=5000` (force `dry_run` or reject on exceed)
* `MAX_LEDGER_EMITS_PER_RUN=1`
* `MAX_RETRIES_LEDGER=2` (with exponential backoff/jitter)

---

## 16) Code-Level Standards

* `#![forbid(unsafe_code)]` at crate root; `deny(warnings)`.
* DTOs: `serde(deny_unknown_fields)`.
* Monetary: fixed-point integers; **never** floats.
* Axum 0.7 handlers end with `.into_response()`.
* **Edge tower stack (order):** `Compression → DecompressionCaps → Timeout(5s) → ConcurrencyLimit(512) → RateLimit(500 rps) → Auth(Macaroon) → Handler`.
* Avoid panics on user-triggerable paths; use typed errors.

---

## 17) Definition of Done (Security)

* §6 checklist green in CI.
* §7 metrics exposed & scraped; alert rules deployed.
* Golden vectors present; signed expectations.
* Chaos/soak reports attached.
* SBOM emitted; `cargo-deny`/`audit` clean.
* Security review sign-off; **last-reviewed** updated.

---

## 18) Mermaid — Security & Trust Flow

```mermaid
flowchart LR
  subgraph Gateway/WAF [Gateway / WAF (optional, internet-facing)]
    G1[Global RPS, WAF, Hybrid TLS Termination]
  end

  A[Caller] -->|TLS + macaroon| G1 --> B(svc-rewarder)

  subgraph Compute
    B --> C1[Fetch policy & inputs by CID]
    C1 --> C2[Validate DTOs & bounds]
    C2 --> C3[Estimate cost ≤ MAX_EPOCH_COST_MS?]
    C3 -->|no| Q1[Reject or force dry_run]
    C3 -->|yes| C4[Deterministic compute]
    C4 --> C5[Check invariants: conservation, idempotency, overflow/dust]
    C5 -->|ok| C6[Seal manifest + sign (Ed25519 [+PQ])]
    C5 -->|breach| Q2[Quarantine (no ledger emit)]
  end

  C6 --> L[ron-ledger (idempotent intent; run_key caveat)]
  B -.-> E[Reject: unauth/oversized/decompress_cap/schema]
  style B fill:#b91c1c,stroke:#7f1d1d,color:#fff
  style Q2 fill:#7c3aed,color:#fff
  style G1 fill:#0ea5e9,color:#fff
```

---

### Appendix — Side-Channel Notes

* Pad/bucket manifest response sizes; add small jitter to non-auth error responses to reduce cross-tenant timing leakage.

---
