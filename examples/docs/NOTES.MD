### BEGIN NOTE - JANUARY 10 2026 - 15:35 CST


## Build plan for the next WOW demo

Target: **“Managed HTTP App” demo** (Rust Axum workload hosted by **micronode**, exposed via **facets**, controlled via **TypeScript SDK**, visible in **svc-admin**). Big wow: refresh → metrics move, click crash → auto-restart, click reload → banner changes, all with one command.

### Stage 0 — Define the contract (no new code yet)

**Goal:** lock the demo UX and URLs so the rest is implementation detail.

**Demo URLs (human-friendly, no hashes):**

* App (through micronode): `http://127.0.0.1:<MICRO>/facets/hello/`
* API: `.../facets/hello/api/counter`
* Dev ops (gated): `.../facets/hello/admin/reload`, `.../facets/hello/admin/crash`
* Meta: `http://127.0.0.1:<MICRO>/facets/meta` (truthful: what’s mounted)

**Wow actions:**

1. open page, hit refresh → RPS/bytes increase
2. click “Crash” (or TS command) → downtime briefly → auto-restart → back up
3. click “Reload” → banner changes → reload counter increments

---

### Stage 1 — Micronode becomes the app host (generic platform feature)

**Goal:** micronode can supervise a workload process and expose it via a Proxy facet.

**Work items:**

1. **Workload supervision (generic)**

   * micronode config gets an `apps[]` list (name, command, args, env, port, mount)
   * micronode spawns each app, watches exit, restarts with backoff
   * metrics: `app_restarts_total{name}`, `app_up{name}`, maybe `app_last_exit_code{name}`

2. **Proxy facet (generic)**

   * implement `FacetKind::Proxy` routes so they *actually proxy*
   * preserve method/path/query/body
   * strip hop-by-hop headers
   * metrics: proxy requests/latency/bytes (labels: app/facet)

3. **Truthful facet meta**

   * `/facets/meta` lists loaded facets + proxy targets + health-ish status
   * svc-admin can render this without guessing

No demo-specific HTML baked into core. Everything above is reusable for real apps.

---

### Stage 2 — The Rust “hello-app” workload (small, boring, reliable)

**Goal:** a tiny Axum binary that’s easy to supervise and easy to prove restart/reload.

Endpoints:

* `GET /` returns HTML showing banner + current counter
* `GET /api/counter` increments and returns JSON
* `POST /admin/reload` changes banner + increments `reload_total`
* `POST /admin/crash` panics/exits (deterministic)

Keep it intentionally minimal so the platform is the star.

---

### Stage 3 — TypeScript SDK-driven DX (God-tier)

**Goal:** developers don’t “curl around”; they run one script and get typed calls + good errors.

**Deliverables (in `examples/hello-app/`):**

* `tools/demo.ts` (or `demo.mjs`) using `ron-app-sdk-ts` to:

  * wait for micronode ready
  * verify facet mounted via `/facets/meta`
  * hit counter endpoint a few times
  * trigger reload, confirm banner changed
  * trigger crash, poll until recovered
* This is where we enforce “nice DX”: retries, timeouts, helpful output.

If the SDK is missing capabilities, we extend the SDK (preferred) rather than hand-roll fetch.

---

### Stage 4 — svc-admin integration (operator “wow”)

**Goal:** when you open svc-admin, you can *see* the hello-app as a managed thing.

Minimum:

* Node detail shows:

  * mounted facets/apps (from `/facets/meta`)
  * restart counter and last reload time
  * quick action buttons (Reload / Crash) **only in dev mode**

This should be thin UI on top of truthful APIs, not magic.

---

### Stage 5 — One-command demo script (non-negotiable DX)

Create `scripts/demo_hello_app.sh` that:

* builds micronode + hello-app (+ optional svc-admin)
* starts micronode with the app configured
* prints:

  * App URL
  * svc-admin URL
  * “Try this” commands
* traps Ctrl-C and shuts down cleanly

---

## Future WOW demos (language + use-case ladder)

Once the “managed app” demo is solid, we can add demos that reuse the **same platform primitives** (supervision + proxy facet + policy + metrics) to prove breadth:

### 1) “Bring your own runtime”

Same demo, different workload:

* Node/Express app
* Python/FastAPI app
* Go net/http app
* Java/Ktor app

Zero platform changes — just different `apps[]` config entries and a different example folder.

### 2) TypeScript-first developer demo

A TS app using the SDK to:

* deploy/register (when we have that control plane)
* stream logs/events
* push a config update and watch readiness flip

### 3) Storage-backed service demo

Workload uses svc-storage (or micronode storage mode) + shows:

* durable data
* migrations
* backup/export hooks (later)

### 4) Multi-node “small cloud” demo

Two micronodes + overlay/discovery:

* service moves
* traffic shifts
* still no content IDs exposed

---

### END NOTE - JANUARY 10 2026 - 15:35 CST



### BEGIN NOTE - JANUARY 10 2026 - 15:40 CST


According to a document from **2025-11-23** (the generated `ron-app-sdk-ts` code bundle), here’s a clean, “no-digging-through-20-files” breakdown of what we’ve already proven with the WOW static-site demo, what we changed in omnigate (and how to document it without demo-lock-in), and the **God-tier DX** path for the next examples.

---

## 1) What we *actually* did in the WOW static-site demo (plumbing, in plain English)

### The request path (end-to-end)

**Client/browser** hits the *public* gateway route space:

* Public surface is **`/app/*`** (this is what SDKs and browsers talk to)

Then:

1. **svc-gateway** receives `GET /app/...`
2. svc-gateway forwards internally to **omnigate’s internal app-plane prefix** **`/v1/app/*`** (gateway does the translation)
3. **omnigate** handles the request (or passes to an app handler) and returns the response.
4. svc-gateway returns it to the client.

That mapping is also explicitly the TS SDK mental model:

* SDK calls **`baseUrl + "/app" + path`**
* SDK **never** calls `/v1/app/*` directly.

### Why your “zero bytes blank page” happened earlier

That symptom usually means the handler returned **empty body** (or you hit a route that was wired but returned nothing). Your later screenshots show the site *did* render, so the path/wiring is correct now; the earlier blank response was likely from a fallback handler returning `204/empty` or an early stub path.

---

## 2) Did we “bake primitive demo code into omnigate” because it was static?

### The key design principle

**Static vs dynamic shouldn’t decide whether code goes into “core.”**
Static content is just another “workload type.” The only reason to add anything to omnigate for a demo is if it is:

* **generic** (useful for non-demo workloads), and
* **removable/disable-able** via config/feature flags, and
* doesn’t create a “special demo-only path” that becomes the default mental model.

### What the TS SDK docs imply is the *intended* static-site story

Your TS facet schema explicitly models a **static facet**:

* `FacetKind = 'static' | 'echo' | 'proxy'`
* Static routes are **declarative** and map a URL path to a file

That’s *exactly* the direction you were already heading with the TOML manifest you pasted (`kind="static"`, route file path). So the “right” long-term posture is:

✅ **Static sites are served as a facet (declarative manifest), not “hardcoded HTML in omnigate.”**
That keeps omnigate from being “the demo server,” and keeps the same mechanism usable for docs, status pages, asset bundles, etc.

---

## 3) Notes to drop into the omnigate crate about what changed (and why)

Paste this into something like: `crates/omnigate/NOTES_WOW_DEMO.md` (or append to your existing NOTES).

```md
# Omnigate WOW Demo Notes (Static Site Slice)

## What we proved
- End-to-end ingress works with the intended contract:
  - Public surface: /app/*
  - Internal omnigate surface: /v1/app/*
  - svc-gateway performs the translation and forwarding (clients/SDKs never call /v1/app/* directly).
- Ops plane works independently (health/ready/metrics) and is reachable while app-plane routes serve content.

## Why the demo added "WOW" behavior
The WOW demo was intended to showcase:
- ingress routing correctness
- reload + crash hooks (operator UX)
- live metrics visibility

## DX / Governance Invariant (important)
Omnigate must not become "the demo app host."
Any demo-only behavior must satisfy:
- Generic: useful for real workloads (not only demos)
- Optional: fully disabled in production builds/config
- Non-authoritative: does not replace the facet/app contract model

## Correct long-term model for static sites
Static content should be hosted via "facets" with declarative manifests:
- Static facet declares routes mapping URL paths -> files.
- This keeps omnigate core clean and ensures the same mechanism serves docs/assets universally.
(See ron-app-sdk-ts facet schema: static/proxy/echo definitions.)

## Security posture notes
- Static hosting is only as safe as:
  - path normalization / traversal prevention
  - strict MIME handling
  - caching/CSP headers
  - policy/auth for non-public routes
- Prefer "deny by default" + explicit public routes.
```

That directly addresses your concern: we document *why* we touched core and how we keep it generic and optional.

---

## 4) “Did we need macronode or micronode?” — for this static demo, no

For the demo you ran, you effectively validated:

* **svc-gateway + omnigate** is sufficient to serve a static “hello world” and show ops/metrics.

So yes: **for a minimal static-site ingress demo, micronode/macronode aren’t required.**

Where micronode/macronode become relevant is when the static content is coming from:

* distributed storage,
* content addressing,
* multi-node routing,
* capability-scoped publishing,
* per-node accounting (bandwidth per node, etc.).

---

## 5) Is the static-site example “very secure”?

It *can be* secure, but **security isn’t “because static.”** It’s about the implementation and defaults.

A secure static facet approach should do (minimum bar):

* **No directory traversal:** never allow `..`, URL-decoding tricks, symlink escapes.
* **Explicit allowlist roots:** serve only from an approved content root.
* **Correct content-type + nosniff**, conservative caching defaults.
* **CSP + X-Frame-Options / frame-ancestors** (especially if you ever host admin-ish pages).
* **Policy/auth by default**: public only if explicitly marked public (your TS facet schema has `security.public` / `requiresAuth` for exactly this kind of intent signaling).

So: **static facet hosting is a great security story *if* it is implemented as a constrained file server behind policy with safe defaults.**

---

## 6) The biggest DX issue you called out (and you’re right)

> Devs shouldn’t clone the repo and stuff code into RON-CORE and recompile everything.

That’s exactly the “platform moment.” The goal is:

### Target DX (what we should build towards)

* Dev ships an **app artifact** (binary, container, WASM later, etc.)
* Dev ships a **manifest** (facets + policy intent + health/reload hooks)
* They run **one command** to start a node that runs their app under supervision
* They can see it in **svc-admin** immediately

Your TS SDK already encodes the “don’t hardcode paths everywhere” philosophy (facet-level helper) and the invariant that `path` is app-relative and gets joined under `/app` automatically. That’s the same energy we want for *running* apps: one declarative entrypoint, not bespoke wiring.

---

## 7) Comprehensive “plumbing notes” for all upcoming examples (so we stop rediscovering it)

Paste this into: `EXAMPLES_PLUMBING.md` (repo root or `docs/`).

```md
# RON-CORE Examples Plumbing (No-Digging Map)

## Contracts (URLs)
- Public client surface:  /app/*
- Internal omnigate surface: /v1/app/*
- Rule: SDKs and browsers NEVER call /v1/app/* directly; only svc-gateway does.

## Request flow
Client -> svc-gateway (/app/*) -> omnigate (/v1/app/*) -> app handler -> back

## TS SDK invariants (ron-app-sdk-ts)
- SDK builds URLs as: baseUrl + "/app" + path
- path is app-relative ("/hello")
- SDK never calls /v1/app/*
- SDK sets observability headers:
  - x-request-id (uuid if missing)
  - x-correlation-id (defaults to request id)
- SDK forbids hop-by-hop headers (Host, Connection, Content-Length, etc.)

## Facets (declarative routing)
- Facet kinds: static | echo | proxy
- static: maps GET/HEAD path -> file
- proxy: forwards to upstream service (scheme/host/port + optional basePath)

## What each plane is responsible for
- svc-gateway:
  - public ingress (/app/*)
  - forwards to omnigate /v1/app/*
  - emits public /healthz /readyz /metrics
- omnigate:
  - internal app-plane surface (/v1/app/*)
  - policy/admission enforcement
  - ops plane (/ops/*) for metrics/health/ready

## Demo expectations (WOW checklist)
Each example should prove:
1) Ingress works (curl + browser)
2) Supervision works (crash -> restart)
3) Reload works (config reload or app reload)
4) Metrics show real counters changing (not mock)
5) Svc-admin can observe the node/app + key counters
```

The TS SDK specifics here are grounded directly in its README/spec:

* URL construction + “never /v1/app”
* required headers + forbidden headers
* join function that enforces `/app` prefix.

---

## 8) Build plan for the next “WOW” example (Option A: micronode runs the app, visible in svc-admin, uses TS SDK + facets)

You said: **Option A** + must use **micronode + svc-admin + TS SDK + facets**. This is the right next demo because it proves the platform, not a trick.

### The demo we build next: “Managed App” (Rust axum app) + TS control client

* **hello-app (Rust/axum)** is the workload (dev-owned)
* **micronode** supervises it (restart on crash)
* **facet manifest** mounts it (proxy facet)
* **TS SDK**:

  * calls the app through `/app/...`
  * optionally emits/validates the facet manifest types (you already have those types)
* **svc-admin** shows:

  * the mounted facet/app
  * restarts counter
  * request counters/latency
  * reload generation counter that actually increments

### What we ship as the “one command” runner

* A single binary or script that:

  1. starts micronode
  2. starts svc-gateway (pointing at micronode/omnigate as appropriate)
  3. starts svc-admin (optional but recommended)
  4. prints URLs + curl commands

This is the **DX moment**: “drop your app artifact + manifest, run one command, you’re live.”

---

## 9) Future demos (multi-language) without changing the platform story

Once the managed-app demo exists, adding languages is just swapping the upstream:

* **Node/Express**, **Python/FastAPI**, **JVM/Ktor**, **PHP/Laravel**, **Go**, etc.
* They all mount the same way via **proxy facet**.
* The platform’s value proposition stays: ingress + policy + supervision + metrics + admin UI.

---

## 10) Quick answers to your last two questions

### “Will the Rust axum hello app still use TypeScript?”

Yes—**TypeScript is the developer client/control surface** (SDK + scripts + integration tests + example app consuming the runtime). The workload can be Rust, but the DX story still includes TS heavily because web devs live there.

### “Node vs axum — which is better DX?”

* For *RON-CORE platform coherence*: **axum** is cleaner (one ecosystem, consistent patterns, fewer moving parts).
* For *developer adoption breadth*: **proxy facet** means we support Node/Python/etc. without platform changes.
  So the best DX is: **platform demo in Rust first**, then “same demo, other languages” as follow-ons.

---


### END NOTE - JANUARY 10 2026 - 15:40 CST


### BEGIN NOTE - JANUARY 10 2026 - 15:50 CST

## Gaps to call out now (so you don’t get stuck next instance)

1. **Where “WOW demo site” lives now**

* You referenced a function like `default_wow_html()` and a script `scripts/demo_site.sh`.
* For the next example, we need to know **exactly which crate/file implements the /app/wow routes** today (omnigate app plane vs some facet router).
* Otherwise we risk designing the next demo “on top of” something that’s actually a temporary stub.

2. **How facets are intended to mount “proxy” in micronode**

* Your micronode facets system supports `static/echo/proxy`, but `proxy` is still “501 stub” in your pasted code (or at least not shown as implemented).
* Option A depends on proxy facets being real, because that’s how we mount “hello-app” without baking it into micronode.

3. **What the “reload generation counter” is wired to**

* You observed the counter stays at 0.
* For the next example, we need the authoritative metric name + where it’s incremented (gateway? omnigate? bus event?).

4. **The “one-command run” story**

* You already have `scripts/run_dashboard.sh` (svc-admin stack) and `scripts/demo_site.sh` (wow).
* For Option A, we’ll need a **single run script** that starts: micronode + gateway/omnigate (as required) + svc-admin + UI and prints the canonical URLs.

Those are solvable quickly, but they require seeing the right files.

---

# What to upload in the next instance (minimum set)

You do **not** need to upload the whole repo. If you upload the following, we’ll have everything needed to design + implement the next examples cleanly.

### A) The scripts (critical)

1. `scripts/demo_site.sh` (the one you ran)
2. `scripts/run_dashboard.sh` (the svc-admin stack runner)
3. Any other script involved in the WOW demo loop (if there is one), e.g. something like:

   * `scripts/dev_*omnigate*.sh`
   * `scripts/dev_*gateway*.sh`
   * anything that mentions `/app/wow`, `reload`, `crash`

### B) Omnigate (critical)

Upload the **omnigate codebundle** or at least these files:

* `crates/omnigate/src/lib.rs`
* `crates/omnigate/src/routes/v1/app.rs` (or wherever `/v1/app/*` is defined)
* `crates/omnigate/src/routes/v1/mod.rs`
* any module that contains `wow`, `default_wow_html`, `reload`, `crash`
* `crates/omnigate/configs/policy.bundle.json` (or the policy config file referenced in logs)

### C) svc-gateway (critical)

Upload the svc-gateway codebundle or at least:

* `crates/svc-gateway/src/routes/mod.rs`
* `crates/svc-gateway/src/routes/app.rs` (the /app/* proxy)
* `crates/svc-gateway/src/config/*` (especially upstream base url)
* `crates/svc-gateway/src/state.rs`

### D) Micronode (critical for Option A)

Upload the micronode codebundle or at least:

* `crates/micronode/src/app.rs`
* `crates/micronode/src/http/admin_api.rs` (svc-admin contract endpoints)
* `crates/micronode/src/facets/mod.rs`
* `crates/micronode/src/facets/loader.rs`
* `crates/micronode/src/facets/manifest.rs`
* **anything related to `FacetKind::Proxy`** (even if it’s currently a stub)
* `crates/micronode/src/config/schema.rs` + loaders/overlays if they exist

### E) ron-app-sdk-ts (important for DX alignment)

Upload the `ron-app-sdk-ts` codebundle (or the folder equivalent). The key files are:

* README/spec
* client (request building)
* facet types (static/proxy/echo)
* any examples/tests included

### F) svc-admin (optional but recommended)

If you want “view app inside dashboard” in the next example, upload svc-admin codebundle or at least:

* node polling / node DTO ingestion
* any UI route that shows planes/facets/apps
* wherever it renders “reload generation counter”

---

# If you want a single “just upload these files” shortlist

If you want to keep it super tight, upload these **six artifacts** next instance:

1. `scripts/demo_site.sh`
2. `scripts/run_dashboard.sh`
3. `OMNIGATE_CODEBUNDLE.md`
4. `SVC_GATEWAY_CODEBUNDLE.md`
5. `MICRONODE_CODEBUNDLE.md`
6. `ron-app-sdk-ts` codebundle (whatever you generated)

Plus (optional but helpful):

* `SVC_ADMIN_CODEBUNDLE.md` (or the relevant polling/UI parts)

---

If you upload that set, we’ll be able to:

* lock the official contracts (/app vs /v1/app),
* identify exactly what was changed in omnigate for WOW,
* implement the Option A “managed app” demo in a way that’s **not demo-locked**, and
* ship a clean, repeatable “run examples” workflow that matches your God-tier DX goals.


### END NOTE - JANUARY 10 2026 - 15:50 CST


