# Carryover Notes for Next Instance

## Goal: Build the “WOW Demo” Example for RON-CORE

These notes are intended to be pasted into the next instance so we can immediately resume work on the **WOW demo** with zero context loss. They assume **svc-admin local login is now working** and we’re ready to start showcasing RON-CORE with a simple, highly observable example.

---

## 0) What “WOW Demo” means (definition + constraints)

### The WOW demo is:

A **super simple showcase** that proves RON-CORE is a real runtime / cloud substrate in minutes:

* A tiny **static website** (HTML/CSS/JS) is served through the node stack.
* We can **crash/restart** the service from the admin dashboard and watch it come back automatically.
* We can see **live metrics** (RPS, latency, bytes in/out, restarts, health/ready transitions).
* We can **reload** (config reload or site content reload) to demonstrate “control plane → data plane effect”.
* The demo must feel **“wow this is really cool and simple”**.

### Hard UX constraint:

* **Do NOT expose content IDs or hashes** (e.g., `b3:`) in the demo UX.

  * Even if RON internally uses content addressing, the demo should show “site is live at URL” and keep IDs opaque.
  * If an ID exists, treat it as `asset_id` or hide it entirely.

### DX constraint:

* DX must be **top tier**. The demo is also a DX audit.
* We will likely discover “this is annoying” spots and fix them as we go.
* We have a **TypeScript SDK** (`ron-app-sdk-ts`) whose job is to prevent hand-rolling.

  * Next instance: user will paste TS SDK codebundle + docs.
  * We should prioritize using the SDK where it makes sense (auth/calls/retries/DTOs).

---

## 1) What’s already done (so we don’t redo work)

### svc-admin login completion (current baseline)

* Local auth login is functioning:

  * `/api/auth/login` returns 200 + session cookie
  * `/api/auth/me` returns identity JSON (username/roles/expiresAtUnixS)
* UI improvements already made:

  * Login card includes centered **🦀 RON-CORE 🧅**
  * TopBar has a **Logout button** (works in local mode by detecting local-session identity shape)
* Outcome: We can use svc-admin as the “operator console” in the WOW demo.

### Practical implication for WOW demo

* The WOW demo can rely on a working admin dashboard:

  * show node/service health
  * show metrics changing
  * trigger debug crash/reload actions (where supported)

---

## 2) “WOW Demo” scope and staging plan

We are explicitly **not** starting with DHT/overlay multi-node. This is a **single-node, single-command** demo first.

### Stage 0 (must hit first)

**“Static site + restart + live metrics + reload”** on a single local node profile.

**What the audience can do:**

* Run one command.
* Open the site URL and see it render.
* Open svc-admin and see:

  * node/service status
  * request metrics increasing when you refresh the site
* Click “Crash service” (or equivalent) → see service restart + availability return.
* Click “Reload” (or equivalent) → see new content/config reflected.

### Stage 1 (optional upgrade after Stage 0 is clean)

* “Publish demo site” (friendly name → site becomes available at a stable URL)
* More polished UX in svc-admin (buttons + logs + status pills)

### Stage 2 (later)

* Multi-node / overlay / discovery
* Still not required for WOW v1

---

## 3) What we need the WOW demo to prove (pillars without complexity)

The WOW demo should demonstrate the following RON-CORE capabilities *without explaining them*:

### Operator-visible proof:

* **Supervision / restart:** crash → auto restart
* **Truthful readiness/health:** ready flips appropriately during restart
* **Metrics:** requests + bytes + latency increase when loading the site
* **Control plane influence:** reload modifies runtime behavior / content

### Developer-visible proof:

* “I can deploy a tiny app and it behaves like a real managed service.”

---

## 4) Deliverables we should produce in-repo

### A) Example directory structure (recommended)

Create a new example area (do not pollute core crates):

* `examples/wow-demo/`

  * `site/`

    * `index.html`
    * `app.js`
    * `style.css`
    * (optional: image assets)
  * `README.md` (60-second runbook)
  * `demo-script.md` (exact copy/paste commands)
  * `screenshots/` (optional later)

### B) Scripts (DX-critical)

* `scripts/demo_wow.sh`

  * starts required services (node + admin)
  * prints URLs and next steps
  * traps Ctrl+C and shuts down cleanly

Optional:

* `scripts/demo_wow_docker.sh` or compose file later (not mandatory in first pass)

### C) Minimal UI support in svc-admin (only if needed)

* If there is already a node “debug crash” endpoint / restart control, use it.
* If not, we may add a **single** dev-only control endpoint for the WOW demo (with strong gating).

---

## 5) WOW demo runtime architecture options (choose simplest)

We will choose the path that requires the least new infrastructure.

### Option 1: Serve static site directly from a simple service/facet

* A small service serves files from `examples/wow-demo/site/`.
* Node supervises it (or we run it beside node initially).
* This is simplest for “site appears” and “crash/restart”.

### Option 2: Serve through existing gateway/ingress plane

* Node’s gateway routes to the demo service.
* Better authenticity (“this is our ingress”), but depends on current gateway routing readiness.

**Decision rule:** Start with whatever is already stable and trivial to run. We can refine routing later.

---

## 6) Metrics and observability expectations for WOW demo

The WOW demo must show at least these metrics in a visible place (svc-admin and/or /metrics):

* Requests total for site endpoint(s)
* Request latency (basic histogram OK)
* Bytes in/out (approx counters acceptable)
* Service restarts counter
* Health/ready state (truthful transitions)

If svc-admin is still showing mock numbers for certain facets, the WOW demo should be wired to show *real values* for at least the demo service and the node.

---

## 7) “Reload” semantics (keep it simple)

Reload can mean:

* re-read config and update something small (like a banner text)
* or re-scan the `site/` directory and update content served

The reload mechanism should be:

* operator-triggered (button in svc-admin if available; otherwise curl endpoint)
* observable (log message + metric bump)
* safe (no complicated state changes)

---

## 8) Crash / restart semantics (keep it safe)

We want a deterministic crash path for the demo:

* a “crash me” endpoint only available in dev/demo mode
* or use existing supervisor “panic trigger” if already present

We must ensure:

* crash endpoint is not exposed accidentally in production
* gated behind local auth + dev flag

---

## 9) TypeScript SDK usage plan (important)

We have a TS SDK intended to prevent hand-rolling.

Next instance, user will paste the TS SDK codebundle + docs.

### How we should use it in the WOW demo:

* Use the TS SDK for:

  * calling node/admin APIs with consistent base URL handling
  * auth/cookie/session handling if supported
  * request wrappers (timeouts, retries, typed DTOs)
* Avoid writing ad-hoc fetch wrappers unless the SDK is missing a feature.

### DX check:

If the SDK can’t easily do something the demo needs, we should:

* document the friction
* decide whether to extend the SDK (preferred) or temporarily patch locally

---

## 10) The “one command” DX goal (non-negotiable)

We should aim for a single command that produces:

* Admin dashboard URL (with login working)
* Demo site URL
* Metrics/health URL(s)
* A short “Try this” section printed in terminal:

  * refresh site
  * click crash/restart
  * click reload
  * watch metrics

Example target output (conceptual):

* Admin: `http://127.0.0.1:5300`
* Site: `http://127.0.0.1:8080/wow`
* Metrics: `http://127.0.0.1:5310/metrics`
* Commands: `curl .../crash`, `curl .../reload`

---

## 11) What we need to ask/confirm in next instance

When starting the WOW demo work, confirm:

1. Which node profile should power the demo first?

   * Micronode (preferred for simplicity)
   * Macronode (if it’s already the stable base)
2. Which existing scripts are currently used to run:

   * svc-admin
   * micronode/macronode
3. Which ports are standard in your dev environment (so the demo prints correct URLs)

We will **not** ask for long clarification; we will pick sane defaults and move.

---

## 12) Known improvement opportunities we may discover (DX audit checklist)

As we build the demo, watch for:

* too many env vars required
* confusing “RBAC bootstrap” behavior (dev ergonomics)
* inconsistent endpoint naming / DTO mismatch
* lack of a single “demo mode” flag
* missing metrics labels or confusing dashboards
* non-obvious service discovery (even on localhost)
* any “manual steps” that could be automated by scripts

Each friction point should become:

* a TODO
* and ideally a small fix in the SDK / CLI / scripts

---

## 13) What to bring into next instance (inputs we need)

User will provide TS SDK codebundle + docs next time.

Additionally, to build the WOW demo efficiently, we should have docs/code visibility into the crates that define:

* node startup/run scripts and port conventions
* gateway/ingress routing and static serving capabilities
* supervision/crash/restart hooks
* metrics endpoints and counters

---

# At the end: Which crates’ documentation should you provide next time?

To build the WOW demo cleanly and avoid guessing, please provide documentation/codebundle (or ALL_DOCS extracts) for these crates:

1. **micronode** (or whichever node profile we’ll run for the demo)

   * startup config, ports, what services are enabled by default
2. **macronode** (optional, but helpful if micronode is still partially mocked)
3. **svc-gateway / omnigate** (ingress routing: how to expose the site URL cleanly)
4. **svc-admin** (we already have it, but include any docs about control-plane actions: reload/crash endpoints)
5. **ron-app-sdk-ts** (TypeScript SDK codebundle + docs — you already plan to paste this)
6. **ron-kernel / ron-bus / ron-metrics** (only the parts relevant to supervision + restart counters + metrics naming)
7. **svc-overlay / ron-transport** (not required for Stage 0, but useful later for Stage 2; optional to paste now)

If you want to minimize what you paste: **micronode + svc-gateway/omnigate + ron-app-sdk-ts** are the top 3 that will let us build Stage 0 without hand-waving.

---
