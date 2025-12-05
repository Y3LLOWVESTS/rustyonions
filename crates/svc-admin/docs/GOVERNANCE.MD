```md
# 🏛 GOVERNANCE.md — svc-admin

---
title: Governance & Operational Integrity — svc-admin
status: draft
msrv: 1.80.0
last-updated: 2025-12-04
audience: contributors, ops, auditors, stakeholders
crate-type: policy
---

## 0. Purpose

This document defines the **rules of engagement** for `svc-admin`:

- Who is allowed to use it and **for what**.
- What it is **allowed to do** to RON-CORE nodes (and what it is explicitly *not* allowed to do).
- How dangerous actions (reload/shutdown) are **gated, audited, and approved**.
- How configuration and credentials are **owned, reviewed, and rotated**.
- What happens when `svc-admin` and the “real world” disagree (appeals / disputes).

`svc-admin` does **not** implement economic logic (no balances, no ledgers, no rewards).  
Its governance is about **operational authority & visibility**, not tokens or money.

This doc ties into:

- **IDB for svc-admin** (role: admin GUI + HTTP proxy, read-mostly).
- **SECURITY.md** (auth modes, surfaces, threat model).
- **CONFIG.md** (who edits what, environments).
- **OBSERVABILITY.md** (audit logs, metrics).
- **Hardening & Scaling blueprints** (bounded authority, safe defaults).

---

## 1. Invariants (MUST)

Non-negotiable rules for `svc-admin` governance:

- **[G-SA1] No “hidden” control plane**

  All actions `svc-admin` can perform on nodes MUST flow through **documented node admin HTTP endpoints** (e.g., `/healthz`, `/readyz`, `/version`, `/metrics`, `/api/v1/status`, future `/api/v1/reload`, `/api/v1/shutdown`).  
  No shell access, no RPC backdoors, no “just for ops” secret endpoints.

- **[G-SA2] Read-mostly by default**

  In all environments, `svc-admin` MUST default to **read-only** mode:

  - `ui.read_only = true` unless explicitly disabled.
  - Mutating actions on nodes (`reload`, `shutdown`, future `scale`, etc.) are **disabled by default**, behind explicit feature flags (*and* auth).

- **[G-SA3] Bounded destructive authority**

  When mutating actions are enabled:

  - They MUST be gated by both:
    - configuration (`actions.enable_reload`, `actions.enable_shutdown`, etc.), and
    - operator role (e.g., `role = "admin"` or stronger).
  - `svc-admin` MUST never implement “super-admin” powers that bypass:
    - node-level policy,
    - auth on the node’s admin plane,
    - global RON policy.

- **[G-SA4] Every operator action is attributable & auditable**

  All operator-visible actions in `svc-admin` MUST:

  - be tied to an **identity** (subject from auth/inbound headers),
  - be logged with:
    - `subject`, `roles`,
    - `action` (e.g., `view_node`, `trigger_reload`, `trigger_shutdown`),
    - `node_id` / `plane` (when applicable),
    - `result` (success / error / denied),
    - `corr_id` for cross-service tracing.

- **[G-SA5] `svc-admin` cannot change economic state**

  `svc-admin` MUST NOT:

  - alter node economic policy,
  - mint/distribute rewards,
  - change quotas or billing rates directly.

  If such changes exist, they must be delegated to dedicated policy / econ services (`ron-policy`, `ron-ledger`, `svc-rewarder`) and surfaced from there as read-only status.

- **[G-SA6] Truthful readiness & status**

  `svc-admin` MUST NOT display a node/plane as “ready/healthy” when underlying node admin endpoints indicate otherwise.  
  Derived views may aggregate or simplify details, but **never invert** health/ready signals.

---

## 2. Roles & Authority

### 2.1 Human Roles

- **Viewer (Read-Only Operator)**

  - Can:
    - log in,
    - view node lists, statuses, metrics,
    - view environment labels (`dev/staging/prod`),
    - download logs/metrics snapshots if allowed.
  - Cannot:
    - trigger reloads/shutdowns,
    - change `svc-admin` config,
    - add/remove nodes.

- **Admin Operator**

  - Can:
    - everything Viewer can,
    - trigger **node-level admin actions** when explicitly enabled:
      - e.g., `/api/nodes/{id}/reload`, `/shutdown`.
    - initiate UI-level operations like “refresh metrics now”.
  - Cannot:
    - bypass node’s own auth/policy,
    - change `svc-admin` binary or deployment topology,
    - touch economic services.

- **Platform SRE / Infra Owner**

  - Can:
    - deploy/upgrade/rollback `svc-admin`,
    - edit `svc-admin.toml` (and environment-specific configs),
    - wire svc-admin behind ingress/LB,
    - connect svc-admin to IdP (Passport) or set `auth.mode=ingress`.
  - Should:
    - treat `svc-admin` config as **IaC** (Git + code reviews),
    - coordinate with Security on auth changes.

- **Security Officer / Auditor**

  - Can:
    - review logs & metrics,
    - review `svc-admin` config (including auth modes, node lists),
    - verify that dangerous features are disabled in prod unless there is explicit approval,
    - audit role assignments and use of admin actions.
  - Cannot:
    - necessarily operate nodes directly (unless they also hold an operator role).

### 2.2 Service Roles & Authority Boundaries

- **svc-admin itself:**

  - *May*:
    - proxy read-only calls to nodes (status, metrics, health),
    - call node admin actions when config + auth allow,
    - display aggregated health & metrics to operators.
  - *Must not*:
    - modify data plane traffic,
    - orchestrate cluster-level changes (e.g., rebalancing, moving shards),
    - host long-lived economic state,
    - act as a “root of truth” for anything beyond the health/status it reads.

- **Nodes (macronode/micronode/etc.):**

  - Are the **source of truth** for:
    - their own health/readiness,
    - their own admin actions semantics,
    - economic/logical behavior of their services.
  - Can ignore or reject `svc-admin` admin requests if they violate node-level policy.

---

## 3. Rules & Operational SLAs

`svc-admin` doesn’t enforce financial SLAs, but it does have **operational governance rules**:

### 3.1 Environment Separation

- Dev, staging, and prod should each have **distinct** `svc-admin` instances with separate configs:

  - Dev: can allow `auth.mode="none"` and enable more dangerous actions for testing.
  - Staging: should mirror prod in auth/config but with non-critical nodes.
  - Prod: must use `auth.mode="ingress"` or `auth.mode="passport"`; `auth.mode="none"` is forbidden.

- Config repository and deployment pipelines MUST make it obvious which environment you’re working on (env labels, naming).

### 3.2 Operational SLAs

These are not user-visible API SLAs but internal commitments:

- **Visibility SLA:**
  - Node status/metrics views should refresh in UI with lag ≤ 30–60s under normal conditions.
- **Truthfulness SLA:**
  - If nodes change readiness/health, `svc-admin` must reflect that within the metrics/status polling interval.
- **Action SLA (when enabled):**
  - When an operator triggers a node reload/shutdown, `svc-admin` must:
    - either clearly confirm success/failure,
    - or show a timeout/error state.
  - No silent “fire and forget”.

### 3.3 Governance Rules for Dangerous Features

- `actions.enable_reload` and `actions.enable_shutdown` (names illustrative) MUST:

  - default to `false` in all configs.
  - be changed via **PR + review** in the config repo (no ad-hoc edits on prod machines).
  - be restricted to specific environments or clusters (e.g., only ops training cluster).
  - be visible in the UI (badge or warning like “Destructive actions enabled”).

---

## 4. Governance Process

### 4.1 Changes to svc-admin Itself

Any change to:

- binary/behavior (code),
- config (addresses, nodes, auth modes, actions),
- deployed number of replicas,

must go through:

1. **Proposal / PR:**

   - Code change with tests, or
   - Config change with clear description (e.g., “enable reload for staging only”).

2. **Review:**

   - At least one reviewer who understands:
     - `svc-admin` role and limitations,
     - security implications of the change.

3. **Approval & Deploy:**

   - Merged to main/stable branch.
   - Deployed via CI/CD, not ad-hoc.

4. **Verification:**

   - Check `/readyz`, dashboards, auth mode behavior.
   - Update runbooks / docs if semantics changed.

### 4.2 Enabling / Disabling Mutating Actions

Specifically for actions like node reload/shutdown:

1. **Justification:**

   - E.g., “Ops training for new SREs”, “Rapid failover drill”, “Emergency node drain tool”.

2. **Scope:**

   - Which environment(s)?
   - Which nodes (maybe only a subset)?

3. **Configuration Change:**

   - Update `svc-admin.toml` under `actions.*` section.
   - Add warnings/labels in UI if supported.

4. **Approval:**

   - At least **two** sign-offs:
     - Platform SRE / Ops lead.
     - Security or product owner (depending on org).

5. **Time-boxing:**

   - Set explicit expiry for this configuration (e.g., revert after training day).
   - Create a follow-up ticket to revert.

### 4.3 Emergency Operations

If an emergency requires temporary enabling of destructive actions (e.g., urgent cluster shutdown via svc-admin):

- Use **pre-defined emergency procedure**:

  1. Declare incident and incident commander.
  2. Temporarily enable required actions via PR or emergency config path (with explicit comment).
  3. Run only the minimum actions required.
  4. Disable again immediately afterward.
  5. Document in incident report and governance history.

---

## 5. Audit & Observability

### 5.1 Audit Logging

`svc-admin` MUST log:

- Operator identity (`subject`, `roles`).
- `auth.mode` in effect.
- Administrative actions:

  - `view_node`, `view_metrics`, `download_logs`.
  - `trigger_reload`, `trigger_shutdown` (or any future mutating actions).
  - Configuration errors or misconfig detection.

- Node-level outcomes:

  - Node unreachable, TLS failure, bad credentials.
  - Node 4xx/5xx on admin endpoints.

Logs should be:

- structured (JSON),
- shipped to a central log store,
- retained for a reasonable period (per security/compliance policy).

### 5.2 Metrics

Core governance-relevant metrics:

- `svc_admin_actions_total{action, result, node_id}`:
  - counts view vs admin actions, success vs error vs denied.
- `svc_admin_auth_failures_total{mode, reason}`:
  - track misconfig or auth drift.
- `svc_admin_upstream_errors_total{kind}`:
  - distinguish node issues vs svc-admin issues.

These metrics allow auditors and SREs to:

- see who is using admin features, how often,
- detect misconfig (e.g., sudden spike in auth failures),
- verify that destructive operations are rare and intentional.

---

## 6. Config & Custody

### 6.1 Config Ownership

- `svc-admin.toml` (and env-specific overlays) are owned by the **Platform SRE / Infra team**, with Security and Product as stakeholders.

- Config is **version-controlled**:
  - No manual edits in `/etc/ron` on prod without IaC backing.
  - No “secret” local configs that diverge from the repo.

### 6.2 Secrets & Credentials

- Node admin credentials:
  - stored in secret manager (Vault/KMS/Cloud secret store),
  - never committed to git,
  - never printed in logs.

- Passport / IdP secrets:
  - same as above; no raw secrets in `svc-admin.toml`.

- TLS keys for ingress:
  - handled by ingress/termination layer where possible.
  - if `svc-admin` terminates TLS directly, keys must be:
    - stored securely,
    - rotated per org policy (e.g., every 90 days).

### 6.3 Rotation & Revocation

- Credential or config mistakes discovered later must prompt:

  - **Revocation** of affected tokens/keys.
  - Redeployment with corrected config.
  - Post-mortem documenting impact and remediation.

---

## 7. Appeal Path

`svc-admin` is an **observer and proxy**, not the ultimate authority. Disputes may arise when:

- the UI shows a node as down but the node seems fine, or vice versa;
- an operator believes an action was executed incorrectly or without authorization;
- security/audit suspects misuse.

**Appeal path:**

1. **Local Investigation:**

   - Operator or SRE gathers:
     - svc-admin logs for relevant `corr_id` and `subject`,
     - node logs/admin metrics around the same time.

2. **Dispute Flag:**

   - Record the suspected issue in the incident/issue tracker:
     - “svc-admin vs node disagreement”,
     - “unintended node shutdown via svc-admin”.

3. **Review by Platform SRE + Security:**

   - Determine:
     - whether svc-admin misrepresented data,
     - whether operator auth/role gating failed,
     - whether config allowed unexpected actions.

4. **Outcome Options:**

   - **Bug in svc-admin**:
     - fix and ship patch,
     - add test/guardrail,
     - update GOVERNANCE history.
   - **Misuse within configured boundaries**:
     - adjust training and/or role assignment,
     - possibly tighten config.
   - **Misconfiguration of boundaries**:
     - revert/lock config,
     - require stronger governance on future changes (more reviewers, env constraints).

5. **Documentation & Lessons Learned:**

   - Update this GOVERNANCE doc and/or RUNBOOK with:
     - new rules or clarifications,
     - examples of what went wrong and how to avoid it.

---

## 8. Acceptance Checklist (DoD)

For `svc-admin` to be considered **governance-ready** (Beta+):

- [ ] **Invariants enforced:**  
  - Read-only by default, no hidden admin channels, no economic state, truthful readiness.

- [ ] **Roles & authority documented and implemented:**  
  - Viewer vs Admin Operator vs Platform SRE vs Auditor; mapped to auth claims/roles.

- [ ] **Mutating actions gated:**  
  - Config flags exist and default to off,  
  - Auth guards enforce admin role,  
  - UI indicates when such actions are enabled.

- [ ] **Governance process in place:**  
  - Config changes go through PR + review,  
  - Emergency procedures documented.

- [ ] **Metrics & logs exported:**  
  - Action and auth metrics exist,  
  - Structured logs capture operator actions with IDs & corr_ids.

- [ ] **SLA & appeals:**  
  - Operational SLAs for visibility and truthfulness defined and monitored,  
  - Appeal path from dispute → investigation → remediation documented.

---

## 9. Appendix

### 9.1 Blueprints & Docs

- **svc-admin:**
  - `IDB.md` — role, DTOs, constraints.
  - `CONFIG.md` — config schema & env overrides.
  - `SECURITY.md` — auth modes, surfaces, threat model.
  - `OBSERVABILITY.md` — metrics, logs, tracing.
  - `CONCURRENCY.md` — task model, bounded channels.
  - `PERFORMANCE.md` — SLOs, perf gates.
  - `API.md` & `INTEROP.md` — HTTP surface & contracts.

- **Global:**
  - Hardening Blueprint — bounded authority, TLS, timeouts.
  - Concurrency & Aliasing Blueprint — safe concurrency practices.
  - Scaling Blueprint — SLO and scaling policies for services.
  - Perfection Gates — especially:
    - Gate I: bounded invariants,
    - Gate K: continuous vigilance,
    - Gate L: scaling & chaos behavior,
    - Gate M: appeal paths and incident learning.

### 9.2 History

- **2025-12-04:** Initial GOVERNANCE draft for `svc-admin` (read-mostly admin GUI + proxy).
- Future incidents, governance changes, or disputes involving `svc-admin` should be summarized here with links to post-mortems and config changes.

```
