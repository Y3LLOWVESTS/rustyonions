

```md
---
title: Security Console — IDB (Policy, Events, Enforcement, UI)
status: draft
msrv: 1.80.0
last-updated: 2026-01-05
audience: contributors, reviewers, ops
owners: [Stevan White]
profile: RON-CORE
---

# 0) Purpose (Single Source of Truth)

Build a **God-tier Security Console** for RON-CORE ops:

- Global view of access/security signals (logins, denied/challenged requests, probes)
- **Country-level heat + trends** (MVP) → optional map later
- **Allowlist/blocklist** (CIDR/IP ranges) + **geo policy**
- “Tripwires” for **flagged areas** (watched paths / suspicious routes / thresholds)
- Auditable changes to policy and observable enforcement outcomes

**Non-goal (for MVP):**
- Deep SIEM features, full IP plotting, user fingerprinting, or long-term forensic data lakes.
- “Security by geo-block only.” Geo is policy/risk reduction, not authentication.

---

# 1) System Surfaces (Where Security Applies)

Security signals and enforcement MUST occur where the peer IP is observed:

1) **svc-admin backend** (protects admin UI/API)
2) **macronode admin plane** (protects node ops endpoints: status/metrics/system)
3) **svc-gateway / omnigate** (protects ingress surfaces)
4) Later: overlay/dht endpoints if they have operator-facing planes

The Security Console aggregates signals **across surfaces** into one operator experience.

---

# 2) Crate Ownership (Spread across RON-CORE)

## 2.1 ron-proto (shared DTOs, wire-safe)
Owns:
- `SecurityPolicyDto`
- `SecurityEventDto`
- `AccessDecisionDto` / `SecurityReason` enums
- Versioning rules for stability (semver discipline)

## 2.2 ron-policy (policy engine, deny-by-default helpers)
Owns:
- Parsing/normalization (CIDR, ISO country codes, path matchers)
- Policy evaluation function(s): request context → decision
- “Trusted proxy” rules + safe forwarded header extraction
- Matching helpers and test vectors

## 2.3 ron-audit (append-only audit trail / event digest)
Owns:
- Policy change log (who/what/when)
- Event digest chaining (tamper-evident)
- Retention and compaction semantics (bounded storage)
- Optional: signed snapshots for export

## 2.4 ron-metrics (golden metrics hooks)
Owns:
- Standard metric naming + labels conventions for security signals

## 2.5 svc-admin (control plane + UI)
Owns:
- Policy CRUD API (authoritative policy store)
- Event aggregation endpoints (global + per-node)
- UI pages/components for Security Console
- Operator RBAC decisions (via passport/auth)

## 2.6 macronode / svc-gateway / others (enforcement + local signal)
Owns:
- Middleware that calls ron-policy to produce decisions
- Emitting standardized security events + counters
- Optional local endpoints for “recent events” and “policy revision applied”

---

# 3) Threat Model (Operator-Grade)

We defend against:
- Unauthorized admin-plane access (scans, probes, credential stuffing)
- Spoofed origin IP via forwarded headers (proxy misconfig / attacker spoof)
- Insider mistakes (bad policy edits, accidental lockouts)
- Drift between configured policy and enforced policy across nodes

We do NOT claim to stop:
- Advanced persistent threats (APT) without broader infra
- Credential compromise without MFA / stronger identity controls (future)

---

# 4) Invariants (Non-Negotiable)

## I-SEC-1: Enforcement at edge
Policy must be enforced in the service that receives the request.

## I-SEC-2: Forwarded headers are untrusted by default
Only honor X-Forwarded-For / Forwarded if immediate peer ∈ `trusted_proxy_cidrs`.

## I-SEC-3: Deny-by-default for privileged paths (when policy says so)
Flagged paths MUST never “accidentally allow” due to missing config.

## I-SEC-4: Bounded memory + bounded storage
Event buffers must be ring buffers; persistent stores must have TTL/compaction.

## I-SEC-5: Privacy minimization
Default storage uses:
- IPv4 as `/24` prefix, IPv6 as `/56` prefix (configurable)
- Country/ASN as primary aggregations
Full IP retention is an explicit “danger zone” toggle with short TTL.

## I-SEC-6: Deterministic policy evaluation
Same input context + policy revision = same decision.
No hidden randomness, no time-dependent implicit behavior (except thresholds).

## I-SEC-7: Operator safety rails
Policy edits must support:
- Dry-run simulation (sample of recent events)
- Rollback to last known good revision
- “Break-glass” local allowlist override (config/env) to prevent lockout

---

# 5) Data Model (DTOs)

## 5.1 SecurityPolicyDto (authoritative config)
Fields (MVP):
- `revision: u64` (monotonic)
- `updated_at: unix_ms`
- `updated_by: String` (subject/id)
- `trusted_proxy_cidrs: Vec<String>`         # CIDR strings
- `admin_allowlist_cidrs: Vec<String>`
- `admin_blocklist_cidrs: Vec<String>`
- `geo_blocked_countries: Vec<String>`       # ISO-3166-1 alpha-2
- `geo_allowed_countries: Option<Vec<String>>`  # allow-only mode
- `watched_paths: Vec<WatchedPathRuleDto>`
- `ip_prefix_mode: IpPrefixMode`             # v4 /24, v6 /56 default
- `retention: RetentionDto`                  # TTLs, max_events, etc.

### WatchedPathRuleDto
- `id: String`
- `name: String`
- `match: PathMatchDto`          # prefix or regex (regex optional MVP)
- `methods: Option<Vec<String>>` # GET/POST/...
- `action: TripwireAction`       # Alert | Challenge | Deny
- `threshold: Option<ThresholdDto>`
- `notes: Option<String>`

### ThresholdDto (MVP)
- `window_seconds: u32`
- `max_hits: u32`
- `key: ThresholdKey`            # ByIpPrefix | ByCountry | Global

## 5.2 RequestContext (internal, not necessarily DTO)
- `ts: unix_ms`
- `surface: Surface`                   # svc-admin | node-admin | gateway
- `node_id: Option<String>`
- `peer_ip: IpAddr`                    # raw at runtime
- `origin_ip: IpAddr`                  # resolved with proxy rules
- `ip_prefix: String`                  # computed per policy
- `country: Option<String>`            # from geo lookup
- `path: String`
- `method: String`
- `user_agent: Option<String>`
- `identity: Option<IdentitySummary>`  # from passport/auth

## 5.3 AccessDecisionDto (optional to return/debug)
- `action: DecisionAction`             # allow | deny | challenge
- `reason: SecurityReason`
- `matched_rule_id: Option<String>`
- `policy_revision: u64`

## 5.4 SecurityEventDto (standardized everywhere)
- `ts: unix_ms`
- `surface: Surface`
- `node_id: Option<String>`
- `request_id: Option<String>`
- `ip_prefix: String`
- `country: Option<String>`
- `path: String`
- `method: String`
- `action: DecisionAction`
- `reason: SecurityReason`
- `identity: Option<IdentitySummary>`
- `user_agent: Option<String>`
- `meta: Option<Map<String,String>>`   # e.g. threshold counts, rule name

### IdentitySummary (minimal)
- `subject: String`
- `groups: Vec<String>` (optional)
- `auth_mode: String` (ingress/passport/none)

---

# 6) Policy Evaluation (ron-policy)

## 6.1 Decision order (deterministic)
Given RequestContext + SecurityPolicy:

1) Resolve `origin_ip` using trusted proxies rule
2) Compute `ip_prefix` per policy (v4 /24, v6 /56)
3) Hard block: if `ip_prefix` ∈ blocklist → DENY(reason=blocked_cidr)
4) Allow-only: if allowlist non-empty and ip_prefix not in allowlist:
   - If path matches privileged surface → DENY(reason=not_allowlisted)
   - Else → CHALLENGE or ALLOW based on configured surface mode
5) Geo rules:
   - if country ∈ geo_blocked → DENY(reason=geo_blocked)
   - if geo_allowed is Some and country not in it → DENY(reason=geo_not_allowed)
6) Watched paths tripwires:
   - if match: apply action (Alert/Challenge/Deny)
   - if threshold exceeded: escalate to Deny/Challenge per rule
7) Default: ALLOW

## 6.2 Tripwire actions
- Alert: allow request (or challenge optionally) but emit event
- Challenge: require auth (passport token) or a stronger auth mode
- Deny: return 403/404 (configurable) + emit event

---

# 7) APIs (svc-admin Control Plane)

## 7.1 Policy
- `GET  /api/v1/security/policy`
- `PUT  /api/v1/security/policy`          # replace whole policy (revision bump)
- `POST /api/v1/security/policy/validate` # returns errors + normalized preview
- `POST /api/v1/security/policy/simulate` # run policy against sample events

## 7.2 Events (aggregated)
- `GET /api/v1/security/events?since=...&surface=...&node_id=...&reason=...`
- `GET /api/v1/security/summary?window=1h|24h|7d`
  Returns:
  - totals by action/reason
  - top countries
  - new countries in window
  - per-surface totals
- `GET /api/v1/security/geo?window=...`
  Returns:
  - country counts + last_seen
  - (optional) ASN counts later

## 7.3 Node integration (pull model MVP)
- svc-admin polls nodes:
  - `GET http://<node-admin>/api/v1/security/events?since=...`
  - `GET http://<node-admin>/api/v1/security/status`  # policy revision applied, last sync, queue depth

Future push model:
- Nodes push events to svc-admin with signed caps.

---

# 8) Node-side APIs (macronode admin plane, gateway)

## 8.1 Local events endpoint (bounded ring buffer)
- `GET /api/v1/security/events?since=...`
- `GET /api/v1/security/status`
  - `policy_revision_applied`
  - `last_policy_sync_ts`
  - `events_dropped_total`

## 8.2 Policy distribution
MVP:
- Nodes fetch policy from svc-admin periodically (or receive via config watcher)
- Nodes enforce “last applied revision” and expose it

Later:
- svc-registry distributes signed policy snapshot
- nodes verify signature before applying

---

# 9) Observability (Golden Metrics)

Per surface/service:
- `security_requests_total{surface,action,reason}`
- `security_tripwire_hits_total{surface,rule_id,action}`
- `security_geo_denies_total{country}`
- `security_allowlist_denies_total`
- `security_events_dropped_total`
- `security_policy_revision{surface}` gauge
- `security_policy_apply_fail_total`

Latency:
- `security_policy_eval_seconds` histogram (cheap but measurable)

UI also needs:
- “stale data” detection (last event ts, last poll, last applied revision)

---

# 10) UI/UX (svc-admin SPA)

## 10.1 Navigation
Add `Security` entry with badge:
- count of denied/challenged in last N minutes
- count of “new countries” in last 24h

## 10.2 Security Console page (MVP)
Top row (KPI tiles):
- Denied (1h/24h)
- Challenged (1h/24h)
- New countries (24h)
- Tripwire hits (24h)

Geo section:
- Country list with mini heat bars
- Toggle: show only “new” countries

Policy editor:
- Allowlist CIDRs
- Blocklist CIDRs
- Block countries
- Watched paths rules list (add/edit/remove)
- Validate + Simulate buttons (must be fast)

Events feed:
- filter chips (surface, node, reason, action)
- “copy as curl” / “copy details” for ops

## 10.3 Mini-heat widget above Nodes
Compact indicator:
- Active countries now
- New countries in 24h
Click → Security page

---

# 11) Scale & Performance Rules

- Nodes dashboard must not fan out heavy polling to all nodes.
  - Prefer:
    - aggregated global security summary from svc-admin
    - detail polling only for selected node
- All event retrieval is incremental (`since=...`) with caps (limit, max window).
- Ring buffers are bounded; dropped events increment a counter and show warning.

---

# 12) Testing (Acceptance Gates)

## 12.1 ron-policy test vectors
- CIDR parsing + matching (v4/v6)
- trusted proxy behavior (spoof tests)
- geo allow/deny
- watched path prefix matching
- threshold windows (deterministic with fake clock)

## 12.2 svc-admin integration tests
- policy validate rejects invalid CIDR/country/rule
- simulate returns expected decision for sample contexts
- events endpoint pagination/filters
- policy revision increments and audit records written

## 12.3 node enforcement tests
- denies non-allowlisted privileged path
- emits SecurityEventDto with correct reason/action
- policy revision applied exposed correctly

---

# 13) Rollout Plan (No Lockouts)

Phase 0 (dev):
- default allow-all, alert-only tripwires

Phase 1 (soft enforcement):
- allowlist enabled on privileged paths only
- challenge mode available

Phase 2 (strict):
- allowlist enforced across admin surfaces
- geo blocks enabled
- policy simulation required before apply (UI guardrail)

Break-glass:
- local env override allowlist (`SECURITY_BREAKGLASS_ALLOW_CIDR`)
- must be visible in UI as “OVERRIDE ACTIVE”

---

# 14) Open Decisions (Track as TODO)

- GeoIP provider (mmdb vs service), update cadence
- Challenge mechanism (passport-only vs additional)
- Push vs pull event aggregation
- Signed policy snapshots (ron-audit + svc-registry)
- IP prefix retention defaults and operator controls
```

---

## How this keeps code out of “svc-admin only”

* **Shared logic** (CIDR/geo/path matching + deterministic evaluation) lives in **ron-policy**
* **DTOs** live in **ron-proto**
* **Audit trail / digest chaining** lives in **ron-audit**
* **Enforcement** lives where traffic lands: **svc-admin backend + macronode admin plane + gateway**
* **svc-admin** is the **control/aggregation/UI** layer, not the entire security system
