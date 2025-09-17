awesome — we’ll keep this 100% **written** (no scripts), and we’ll make it ruthless about the “golden rule”: if we have to invent a contrived reason to include a crate in a scenario, it’s probably redundant.

Below is a **God-tier, scenario-driven blueprint** that:

* enumerates **realistic, orthogonal scenarios**;
* maps **exact crates/services per scenario** with the unique reason each is needed (so redundancy pops);
* gives you a **coverage matrix** (Scenarios × Crates);
* flags **redundancy risks** and **drift sentinels** right where they’ll show up.

---

# RustyOnions Scenario Blueprint (Write-Only, Redundancy-Focused)

## Canonical crates/services (normalized roles)

Core/runtime: **ron-kernel** (supervision), **ron-bus** (events), **metrics** (Prometheus), **ron-policy** (config/policy), **transport** (TCP/TLS), **arti-transport** (onion overlay)
Ingress/plane services: **svc-gateway** (ingress), **svc-index** (address→providers), **svc-overlay** (PUT/GET routing), **svc-storage** (CAS)
Edge/app: **ron-app-sdk** (client SDK), **micronode** (single-tenant node/dev), **omnigate** (app BFF)
Identity/semantics: **naming** (IDs/addresses), **oap** (on-wire app protocol), **interop** (external systems)
Security/ledger: **ron-kms** (keys), **ron-audit** (trail), **ron-billing** (invoices/usage), **accounting** (counters/snapshots)

> If any “legacy lib” variants exist (e.g., `gateway` vs `svc-gateway`, `index` vs `svc-index`), we treat the `svc-*` ones as authoritative. If you find yourself adding the legacy lib next to the service in a scenario, that’s a redundancy smell (see “Redundancy alarms” near the end).

---

## 16 written scenarios (no code; each lists crates used + why they’re uniquely needed)

### S1. Bootstrap & Health (single-process baseline)

**Goal:** prove the runtime spine works before anything else.
**Crates used & non-redundant reasons**

* ron-kernel — supervision loop and lifecycle gates (health/ready)
* ron-bus — emits/observes `KernelEvent::Health`, `Shutdown`
* metrics — exposes `/metrics`, `/healthz`, `/readyz`
* ron-policy — loads baseline config; emits `ConfigUpdated` if changed
* transport — base TCP/TLS server readiness
  **Anti-redundancy check:** nothing else can own supervision or health gating → unique to kernel.

---

### S2. Config Reload & Propagation

**Goal:** config versioning and live apply without restart.
**Crates**

* ron-policy — source of truth; versioned config
* ron-bus — `ConfigUpdated { version }` broadcast
* ron-kernel — orchestrates safe apply windows
* metrics — increments config apply counters
  **Uniqueness:** policy is distinct from metrics and bus; if another crate “applies config” directly, it’s drift.

---

### S3. Service Supervision & Backoff (crash → restart)

**Goal:** graceful recovery and correct readiness after restart.
**Crates**

* ron-kernel — backoff & supervised restart
* ron-bus — `ServiceCrashed { service }`
* metrics — `service_restarts_total`, `bus_lagged_total`
  **Uniqueness:** only kernel may restart services. If “actors” elsewhere restart themselves → redundancy drift.

---

### S4. Content Publish (PUT) → Address

**Goal:** canonical “write path” proving CAS/storage/index alignment.
**Crates**

* svc-gateway — ingress boundary (HTTP or RPC)
* naming — manifest/address formation (canonical rules)
* svc-index — packs/records mapping address → providers
* svc-storage — stores chunks; returns content address header
* svc-overlay — orchestrates PUT routing (even if local in dev)
* ron-audit — logs immutable trail of the write
* transport — request/response IO plane
* ron-bus — state/health events on the path
  **Uniqueness:** storage (CAS) vs index (mapping) must be separate. If one tries to do both → redundancy.

---

### S5. Content Resolve (GET) → Stream

**Goal:** “read path” via resolution → provider selection → streaming.
**Crates**

* svc-gateway — ingress
* svc-index — resolves address → provider set
* svc-overlay — selects route
* svc-storage — streams content if local; otherwise pulls via overlay
* naming — verifies address canonicalization
* ron-audit — read trail
* transport — data plane
  **Uniqueness:** overlay is routing/selection; storage is bytes; index is mapping. No crossover.

---

### S6. SDK-Driven Fetch (App happy path)

**Goal:** exercise the public developer experience.
**Crates**

* ron-app-sdk — request builder, retries, canonical headers
* oap — framing/protocol for app requests
* omnigate — app BFF (rate-limit, auth, DTO shaping)
* svc-gateway, svc-index, svc-overlay, svc-storage — as in S5
* metrics — latency histograms from ingress edge
  **Uniqueness:** omnigate ≠ gateway. If gateway starts doing BFF transforms → redundancy.

---

### S7. Micronode Join & Offline Serve

**Goal:** single-tenant node that can serve offline packs.
**Crates**

* micronode — one-command node; dev-oriented serve
* svc-storage — reads local packs; serves without network
* ron-kernel/ron-bus/metrics — local runtime spine
* naming — local address registry
  **Uniqueness:** SDK (client library) vs Micronode (standalone node) are different roles. If SDK pretends to be a node → redundancy.

---

### S8. Secure Message Send/Receive (E2E)

**Goal:** small signed+encrypted message between two parties.
**Crates**

* ron-kms — keys/sign/verify; rotate independently of services
* oap — small-message framing
* svc-overlay — route selection (store-and-forward or live)
* arty-transport — onion/private path when configured
* ron-audit — immutable event trail (send, deliver)
* transport — IO edges
  **Uniqueness:** KMS is not “crypto utils”; it’s a proper boundary with ops (keys/rotation). If crypto spills into overlay → redundancy.

---

### S9. Naming Registration & Canonicalization

**Goal:** register aliases, canonicalize IDs, prevent loops.
**Crates**

* naming — canonical rules, alias graph
* svc-index — consumes updates (names to addresses)
* svc-overlay — updates routing tables if name affects path
* ron-bus — emits naming updates; consumers react
  **Uniqueness:** naming = rules/semantics; index = state/mapping.

---

### S10. Policy-Driven Path Split (OAP: onion vs direct)

**Goal:** some requests go onion plane; others direct.
**Crates**

* ron-policy — decl for split + caveats
* oap — negotiates required path
* svc-gateway — enforces decision; forwards
* svc-overlay (+ arti-transport) — onion path
* transport — direct path
* metrics — per-path buckets to observe split
  **Uniqueness:** oap decides/negotiates; gateway enforces; overlay executes.

---

### S11. Interop Import → Normalize → Serve

**Goal:** ingest from external (HTTP/S3-like), normalize, expose internally.
**Crates**

* interop — external adapters, normalization
* svc-storage — ingests normalized chunks
* svc-index — registers address→provider mapping
* svc-gateway — serves back internally
  **Uniqueness:** interop is the *only* place that knows external formats. If storage starts fetching from S3 → redundancy.

---

### S12. Billing & Accounting (usage → invoice)

**Goal:** track usage, roll up counters, produce invoices.
**Crates**

* accounting — counters/rings/snapshots (operational telemetry)
* ron-billing — aggregates usage → invoice artifacts
* svc-gateway — emits metered units from ingress
* metrics — exports operational counters (not the invoice!)
  **Uniqueness:** accounting ≠ billing. Accounting counts; billing invoices. If one tries to do both → redundancy.

---

### S13. Admin Capabilities: Mint/Rotate/Revoke

**Goal:** issue/rotate macaroons/permissions.
**Crates**

* ron-policy — capability schema, caveats
* ron-kms — signs capabilities
* ron-bus — emits capability changes
* omnigate/svc-gateway — enforce capabilities at ingress
  **Uniqueness:** KMS handles keys; Policy defines what a cap means; Gateway enforces. Keep these separated.

---

### S14. Federation Handshake (private mesh)

**Goal:** join private federation with trust criteria.
**Crates**

* arti-transport — private overlay bootstrap
* svc-overlay — neighbor discovery/handshake
* ron-policy — allowlist/denylist expressions
* ron-bus — membership events
  **Uniqueness:** overlay handles neighbor graph; policy sets who is allowed. No mixing.

---

### S15. Observability Golden Signals

**Goal:** baseline metrics/logs across plane & edge.
**Crates**

* metrics — standard endpoints & histograms
* ron-audit — security/ops trails
* ron-bus — event stream for health/drift
  **Uniqueness:** audit is not metrics; audit is evidence.

---

### S16. Graceful Shutdown (drain + persist)

**Goal:** no data loss, clean drain.
**Crates**

* ron-kernel — coordinates drain/stop
* svc-overlay — finishes in-flight; no half writes
* svc-storage — fsync/flush where needed
* ron-bus — `Shutdown` broadcast
  **Uniqueness:** only kernel can orchestrate global drain.

---

## Coverage matrix (who’s hit where)

| Crate/Service  |  S1 |  S2 |  S3 |  S4 |  S5 |  S6 |  S7 |  S8 |  S9 | S10 | S11 | S12 | S13 | S14 | S15 | S16 |
| -------------- | :-: | :-: | :-: | :-: | :-: | :-: | :-: | :-: | :-: | :-: | :-: | :-: | :-: | :-: | :-: | :-: |
| ron-kernel     |  ✅  |  ✅  |  ✅  |     |     |     |  ✅  |     |     |     |     |     |     |     |     |  ✅  |
| ron-bus        |  ✅  |  ✅  |  ✅  |  ✅  |  ✅  |  ✅  |  ✅  |  ✅  |  ✅  |  ✅  |  ✅  |  ✅  |  ✅  |  ✅  |  ✅  |  ✅  |
| metrics        |  ✅  |  ✅  |  ✅  |     |  ✅  |  ✅  |  ✅  |  ✅  |  ✅  |  ✅  |     |  ✅  |     |     |  ✅  |     |
| ron-policy     |  ✅  |  ✅  |     |     |     |  ✅  |     |     |     |  ✅  |     |     |  ✅  |  ✅  |     |     |
| transport      |  ✅  |     |     |  ✅  |  ✅  |     |     |  ✅  |     |  ✅  |     |     |     |     |     |     |
| arti-transport |     |     |     |     |     |     |     |  ✅  |     |  ✅  |     |     |     |  ✅  |     |     |
| svc-gateway    |     |     |     |  ✅  |  ✅  |  ✅  |     |     |     |  ✅  |  ✅  |  ✅  |  ✅  |     |     |     |
| svc-index      |     |     |     |  ✅  |  ✅  |  ✅  |     |     |  ✅  |     |  ✅  |     |     |     |     |     |
| svc-overlay    |     |     |  ✅  |  ✅  |  ✅  |  ✅  |     |  ✅  |  ✅  |  ✅  |     |     |     |  ✅  |     |  ✅  |
| svc-storage    |     |     |     |  ✅  |  ✅  |  ✅  |  ✅  |     |     |     |  ✅  |     |     |     |     |  ✅  |
| ron-app-sdk    |     |     |     |     |     |  ✅  |     |     |     |     |     |     |     |     |     |     |
| micronode      |     |     |     |     |     |     |  ✅  |     |     |     |     |     |     |     |     |     |
| naming         |     |     |     |  ✅  |  ✅  |     |  ✅  |     |  ✅  |     |     |     |     |     |     |     |
| oap            |     |     |     |     |     |  ✅  |     |  ✅  |     |  ✅  |     |     |     |     |     |     |
| interop        |     |     |     |     |     |     |     |     |     |     |  ✅  |     |     |     |     |     |
| ron-kms        |     |     |     |     |     |     |     |  ✅  |     |     |     |     |  ✅  |     |     |     |
| ron-audit      |     |     |     |  ✅  |  ✅  |  ✅  |     |  ✅  |     |     |     |     |     |     |  ✅  |     |
| ron-billing    |     |     |     |     |     |     |     |     |     |     |     |  ✅  |     |     |     |     |
| accounting     |     |     |     |     |     |     |     |     |     |     |     |  ✅  |     |     |     |     |
| omnigate       |     |     |     |     |     |  ✅  |     |     |     |     |     |     |  ✅  |     |     |     |

**Result:** every crate is exercised at least once with a **unique reason** to exist. If you cannot write the listed “unique reason” for a crate in a given scenario without repeating another crate’s purpose, the crate is likely redundant *for that scenario*. If this repeats across scenarios, it’s redundant overall.

---

## Per-crate “uniqueness tests” (fast redundancy litmus)

* **ron-kernel**: must be the only place that supervises, drains, and flips readiness globally. If any service manages its own restart windows, it’s drift.
* **ron-bus**: must be the only event fan-out for lifecycle. If services communicate out-of-band for lifecycle, flag.
* **metrics**: operational counters/latency only. If someone logs security evidence here, that’s audit’s job.
* **ron-policy**: schema + version + enforcement hooks. If policy logic shows up hard-coded in gateway/overlay, drift.
* **transport** vs **arti-transport**: transport = generic TCP/TLS; arti-transport = onion overlay driver. If either implements the other’s role, redundancy.
* **svc-gateway** vs **omnigate**: gateway = neutral ingress; omnigate = app BFF (DTO shaping, rate limits). If gateway starts acting like BFF (or vice versa), redundancy.
* **svc-index** (mapping) vs **svc-storage** (bytes) vs **svc-overlay** (routing): three distinct verbs; no blending.
* **ron-app-sdk** (library) vs **micronode** (process): if SDK spins servers or persists; or micronode starts linking into app memory → role bleed.
* **naming** (semantics) vs **svc-index** (state): rules vs data—keep separate.
* **ron-kms** (keys) vs crypto sprinkled elsewhere: keys and signing live here; services call through.
* **ron-audit** (evidence) vs **metrics**: never mix.
* **ron-billing** (invoice artifacts) vs **accounting** (counters): different audiences and retention.

---

## Redundancy alarms to investigate (actionable)

1. **gateway (legacy lib) vs svc-gateway**
   If both exist, deprecate the lib and keep only the service interface. Using both in the same scenario is a smell.
2. **index (lib) vs svc-index**
   If the lib is public, ensure it’s service-internal. External callers should depend on svc-index’s API surface, not the lib directly.
3. **Actor runtimes (e.g., an optional `kameo` feature) vs kernel supervision**
   Runtimes must not usurp kernel-level lifecycle. Keep them behind kernel contracts, or remove if duplicative.
4. **SDK vs Micronode**
   If either claims the other’s responsibilities (client vs node), simplify — pick one per use case.

---

## Drift sentinels (what to watch in the docs/code while writing scenarios)

* **Naming discipline:** all deployable services are `svc-*`. Any doc or manifest deviating is drift.
* **Transport types:** TLS config type is `tokio_rustls::rustls::ServerConfig`. Seeing `rustls::ServerConfig` in code or doc is drift.
* **Index/Storage DB identity:** a single env/setting (often `RON_INDEX_DB`) is shared. If a scenario requires different DBs without hard justification, drift.
* **Kernel events:** `Health`, `ConfigUpdated`, `ServiceCrashed`, `Shutdown` are the canonical event set for lifecycle. Extra ad-hoc events = drift unless justified.
* **Observability floor:** every networked service exposes `/metrics`, `/healthz`, `/readyz`. If a scenario documents exceptions, drift.

---

## How to *use* this document (write-only workflow)

* For each scenario above, create a **short narrative** page (1–2 paragraphs) that re-states the **crates list** and the **unique reasons** exactly as above.
* At the bottom of each page, add a **“Redundancy/Drift Notes”** box:

  * “We tried to include crate X. Could we justify it uniquely? Y/N. If N → redundancy candidate.”
  * “Any crate performed another’s role? → drift candidate.”
* Maintain a single **Scenario × Crate** table (like the matrix above). If a crate appears in only 1 scenario and still feels forced, take that as a strong redundancy signal.
* When you spot redundancy, record a **Decision**: *keep A, deprecate B* (and note where to move any unique capabilities from B).

---

## If you want an even tighter net (optional extras, still write-only)

* Add a **“Unique Value Assertion”** one-liner per crate in each scenario doc (e.g., “Overlay selected route X → Storage didn’t route”).
* Keep a **“No-Code Drift Ledger”** where you jot mismatches (e.g., doc says svc-overlay exposes `/readyz`, but config file names differ). This primes your next implementation pass.

---

### Bottom line

You now have a **purely written**, scenario-driven map that forces each crate to justify its existence with **unique, non-overlapping responsibility**. Anywhere that justification can’t be written **without repeating another crate’s purpose**, that crate is on the chopping block.




#NEXT PROMPT


Here’s your scenario rewritten in precise, technical terms, then mapped—end-to-end—across the RustyOnions stack with explicit crates/services, artifacts, events, and trust boundaries. I’ll also call out what’s missing.

# Technical restatement of the scenario

A user, via a standard web browser, navigates to a RustyOnions-hosted decentralized web application (dApp) using a CRAB-style link (i.e., a name/content address resolvable on the RustyOnions overlay). The user creates a new post. As other users fetch/share that post through the network, usage/propagation is metered and rewarded in ROC (RustyOnions Credits). ROC is credited to the user’s RON wallet. The user then off-ramps: exchanging ROC via an external ROX exchange (outside the RustyOnions network).

---

# End-to-end flow (phases → services/crates → why they’re uniquely needed)

## Phase A — Accessing the dApp (“open the crab link”)

**Goal:** resolve the CRAB/Name → content (site bundle), serve to browser over RustyOnions.

* **svc-gateway** (ingress HTTP): terminates the user’s browser request; enforces rate limits and basic policy at the edge.
  *Why unique:* the neutral ingress; not a BFF.
* **naming** (semantics): normalizes the CRAB/name to a canonical address (e.g., content address or service id).
  *Why unique:* rules & canonicalization—not state.
* **svc-index** (mapping): resolves canonical address → current providers (who can serve the site assets).
  *Why unique:* address→provider map, independent of bytes.
* **svc-overlay** (routing): selects the route/provider (local vs remote) for the fetch.
  *Why unique:* path selection; does not store bytes.
* **svc-storage** (CAS): returns the dApp’s static bundle (HTML/JS/CSS) by content address, potentially warming cache.
  *Why unique:* storage/streaming of bytes; not routing or mapping.
* **transport** + **arti-transport**: I/O plane; onion/private path when policy requires; direct transport when allowed.
  *Why unique:* data plane separation: generic TLS vs onion overlay.
* **ron-policy**: decides if this fetch must go overlay/direct; applies edge constraints.
  *Why unique:* central policy schema + enforcement knobs.
* **metrics** + **ron-audit**: record operational telemetry and evidence (who/what was served, when).
  *Why unique:* metrics ≠ audit: ops vs evidence.
* **ron-bus**: lifecycle/events (health/ready, config updates) that keep edge services consistent.
  *Why unique:* the event spine for coordination.

**Artifacts/Events:**

* Address canonicalization record (naming)
* Index resolution event (svc-index)
* Access log/evidence entry (audit)
* Latency/throughput counters (metrics)

**Boundary:** Browser ⇄ Gateway (public edge). Policy decides overlay vs direct.

---

## Phase B — AuthN/Capabilities for posting (pre-post)

**Goal:** ensure the author can create content; bind post to an identity/capability.

* **omnigate** (app BFF): handles dApp-specific DTOs and authdance (e.g., macaroons), hides internal topology from the web app.
  *Why unique:* the only place that shapes app-facing APIs.
* **ron-policy**: capability model (what “post” entails, possible caveats/quotas).
* **ron-kms**: issues/signs capabilities or verifies signatures.
  *Why unique:* key custody, rotation, signing.
* **ron-audit**: capability issuance/verification trail.

**Artifacts/Events:**

* Capability/macaroons issued/validated
* Audit evidence of issuance/use

**Boundary:** BFF ⇄ core policy/KMS. (This is where we enforce “no app logic in kernel.”)

---

## Phase C — Creating the post (write path)

**Goal:** persist the post as content-addressed data; register for discovery.

* **ron-app-sdk** (client-side library inside the dApp): packages the post payload, handles retries/backoff, adds canonical headers.
  *Why unique:* developer ergonomics; not a process/node.
* **svc-gateway** (ingress for write): receives the post PUT.
* **naming**: (if post has a human-friendly name/slug) computes canonical form → content address binding policy.
* **svc-storage** (CAS): stores the post’s immutable body/chunks; returns a content address (e.g., `b3:…`).
* **svc-index**: registers address → provider mapping so others can find it.
* **svc-overlay**: orchestrates PUT path (could be local CAS on this node, or forwarded to a designated provider).
* **ron-audit**: immutable write trail (who posted which content address).
* **metrics**: PUT latencies, cache/store counters.

**Artifacts/Events:**

* Content address (post id)
* Index registration entry
* Audit trail of the write
* Bus health/ready signals throughout (implicit)

**Boundary:** Author app ⇄ Edge ⇄ Core CAS/Index.

---

## Phase D — Sharing/Propagation (read path at scale)

**Goal:** lots of users fetch the post; network propagation causes ROC rewards to accrue.

* **svc-gateway**: ingress for readers.
* **svc-index** → **svc-overlay** → **svc-storage**: resolve, route, stream (as in Phase A).
* **ron-policy**: throttles/quotas if abuse; path selection.
* **metrics**: per-path histograms; cache hit-ratio; request totals.
* **ron-audit**: (optional) sampling or specific evidence for regulated paths.

**Economics signal (this is key):**

* **svc-gateway** (and/or **omnigate**) must emit **usage/metering events** for successful reads/shares.
* Those events feed the **economics pipeline** (next phase).

---

## Phase E — ROC reward accrual (economics)

**Goal:** translate usage/shares into ROC credited to the author.

* **accounting**: low-level counters/rings/snapshots of usage (e.g., “post X fetched N times by unique peers”).
  *Why unique:* counts over time; not financial artifacts.
* **ron-billing** (or **ron-ledger**/**ron-token**, depending on how you’ve named it): applies the reward rules to accounting snapshots, mints ROC units, and produces ledger-grade records (credits to author).
  *Why unique:* transforms usage into credit; persistent economic artifacts.
* **ron-policy**: governs reward rates, caps, fraud signals.
* **ron-kms**: signs mint/credit transactions.
* **ron-audit**: immutable record of issuance/credits (who, what, why; ties back to usage evidence).

**Artifacts:**

* Usage snapshots (accounting)
* ROC mint/credit entries (ledger/billing)
* Signed credit txns (KMS)
* Audit entries

**Boundary:** Metering events ⇄ economics pipeline. (No user interaction here; it’s internal.)

---

## Phase F — RON wallet credit (custody)

**Goal:** reflect the author’s ROC balance and let them hold/spend it.

* **(missing/uncertain)** **svc-wallet** (or equivalent) — *not yet present in your canonical set*: a service to expose balances and custody ROC for users (with KMS used for keys).
  *Why needed:* wallet abstraction for balances/addresses; UX & API for withdrawals/deposits.
* **ron-kms**: key material for addresses; signing spends.
* **ron-ledger**/**ron-billing**: the source of truth for ledger entries that the wallet surfaces.

**Artifacts:**

* Wallet account record (user ↔ address set)
* ROC balance updated (after mint/credit posts)

**Boundary:** Internal ledger ⇄ external-facing wallet API.

---

## Phase G — Off-ramp to ROX exchange (outside RustyOnions)

**Goal:** exchange ROC on an external ROX venue.

* **interop**: the only legitimate bridge to external exchanges/APIs (ROX). Handles protocol translation, retries, and error taxonomy.
  *Why unique:* quarantine of external dependencies; consistent adapters.
* **ron-policy**: off-ramp policy (allowlist of exchanges; AML/KYC toggles; daily caps).
* **ron-kms**: signs withdrawal/transfer transactions.
* **ron-audit**: full off-ramp evidence (who/amount/destination).
* **metrics**: success/error counters; latency.

**Artifacts:**

* Off-ramp request (wallet → interop)
* Signed transfer (KMS)
* Exchange confirmation (interop)
* Audit record

**Boundary:** RustyOnions trust domain ⇄ ROX external trust domain.

---

# Minimal responsibility map (so redundancy is obvious)

| Concern                         | The *one* crate/service that owns it                      |
| ------------------------------- | --------------------------------------------------------- |
| Ingress HTTP                    | svc-gateway                                               |
| App-specific DTOs / API shaping | omnigate                                                  |
| Name/CRAB canonicalization      | naming                                                    |
| Address→providers mapping       | svc-index                                                 |
| Routing / path selection        | svc-overlay                                               |
| Content storage/streaming       | svc-storage                                               |
| Data plane (TLS)                | transport                                                 |
| Onion/private routing           | arti-transport                                            |
| Policy/quotas/splits            | ron-policy                                                |
| Keys, signing, rotation         | ron-kms                                                   |
| Operational telemetry           | metrics                                                   |
| Evidence/audit trail            | ron-audit                                                 |
| Lifecycle/supervision           | ron-kernel                                                |
| Lifecycle/event fan-out         | ron-bus                                                   |
| Usage counters/snapshots        | accounting                                                |
| Rewarding/mint/credits          | ron-billing / ron-ledger / ron-token (pick one canonical) |
| Wallet custody & balances       | **(missing)** svc-wallet (or equivalent)                  |
| External exchange bridge        | interop                                                   |
| Client developer ergonomics     | ron-app-sdk                                               |
| Single-tenant dev node          | micronode                                                 |

If any *other* crate tries to own those responsibilities in this scenario, that’s redundancy or drift.

---

# What’s missing or ambiguous (actionable)

**P0 (blockers for this scenario)**

1. **Wallet service/API**: a first-class **svc-wallet** (or similarly named) is required to expose balances, addresses, and signed spends for ROC. Without it, you can mint credits but the user can’t *hold* or *withdraw* them cleanly.
2. **Canonical economics surface**: you mention ROC and ledgering; in your crates this appears split across `accounting` and billing/ledger/token naming variants. Pick **one** canonical service/crate name for economic artifacts (e.g., `ron-ledger` or `ron-billing` + `ron-token`) and make the others internal libs (or deprecate). The wallet and interop need a single, authoritative ledger API.
3. **Off-ramp policy/AML guardrails**: `ron-policy` must explicitly model off-ramp rules (allowed venues, daily caps, flags). If that schema isn’t in place, you’ll lack controls at Phase G.
4. **dApp write API shape**: `omnigate` endpoints for “createPost” and “getPost” should be specified (DTOs, error taxonomy). If not present, authoring is under-defined.

**P1 (important but you can defer a version)**
5\) **Attribution & anti-fraud** for “lots of people share the post”: define how a “share” contributes to ROC (unique users? time window? sybil resistance?). Likely lives in `accounting` + `ron-policy` (rules) with optional ZK/fraud hooks later.
6\) **Index consistency proofs**: optional but valuable—prove that the index’s provider list aligns with actual availability (could be a background verifier).
7\) **Content moderation boundary**: if off-ramp depends on content class (e.g., restricted media), define where policy evaluates that (omnigate vs policy vs separate `svc-moderation` if you create one).

---

# Open questions (drive them to closure in the blueprints)

* CRAB link exact semantics: is it a name→address indirection (naming) or a bundled “content+policy address”? Document in **Naming & Addressing**.
* ROC unit economics: fixed schedule or demand-based? Where does that live? (**ron-policy** for parameters; **ledger** for artifacts).
* Wallet custody: are keys user-held (client-side) or server-custodied with HSM-like protections via **ron-kms**? Decide early—affects omnigate and interop.

---

# TL;DR

Your scenario—browser → dApp → post → share→ ROC → wallet → ROX—uses nearly the full stack. The critical **missing** piece is a **wallet service** (custody + balances) and a **single, canonical ledger/billing/token surface** that the wallet and interop depend on. Lock those down and the rest of the flow cleanly composes without redundancy: gateway/omnigate (edge), naming/index/overlay/storage (content plane), accounting/ledger/policy/kms/audit (economics & governance), interop (off-ramp).


#NEXT PROMPT

Got it. Here’s a tight, exhaustive write-only blueprint for your new scenario—no scripts, just crisp flows, responsibilities, and gaps.

# Technical restatement

A user must possess a local identity bundle to use RustyOnions as a web3 participant. This bundle comes in two flavors: a named identity **`.passport`** and a privacy-preserving **`.alt`** (anonymous) variant. The user loads one of these into their browser or app. With a valid passport/alt:

1. they can **relay public assets** (acting as an edge relay node) and **earn ROC** proportional to verifiably forwarded bytes;
2. they can **spend ROC** when visiting sites/apps (micro-payments for content/services).
   The bundle must be **offline-storable** (“cold storage”), **password-protected**, and **tamper-evident** with strong KDF + AEAD; revocation/rotation must be supported.

---

# End-to-end flow (phases → services/crates → why each is uniquely needed)

## Phase A — Issuance: create `.passport` and `.alt`, then cold-store

* **New: `svc-passport` (missing service)**

  * Issues versioned identity manifests: `.passport` (pseudonymous, feature-full) and `.alt` (least-privilege, unlinkable where possible).
  * Assembles the **encrypted bundle** for cold storage.
* **`ron-kms`**

  * Generates key material (long-term identity, signing, and optional relay/settlement subkeys).
  * Signs the manifest; holds rotation policies.
* **`ron-policy`**

  * Defines capability/caveat schema: what a passport can do (relay, spend, post), time windows, rate limits, KYC/AML flags (if required).
* **`ron-audit`**

  * Records issuance, rotations, revocations (immutable trail).
* **`metrics`**

  * Operational counters for issuance/rotations.

**Security blueprint for the bundle**

* **Format**: versioned header + metadata + ciphertext + authentication tag.
* **KDF**: Argon2id with per-bundle salt (high memory/time params).
* **AEAD**: XChaCha20-Poly1305 (or AES-GCM if hardware acceleration demanded).
* **Separation of keys**: identity keypair (signing), relay keypair, spending key (ledgery), each with distinct derivation paths and scoped capabilities.
* **Revocation list**: embedded pointers (revoked key ids, issuance chain).
* **Tamper evidence**: outer manifest signed by KMS and the identity’s public key (double attestation).

> “Hack-proof” is not a promise we can make—use **defense-in-depth** + strong defaults + rotation + revocation.

---

## Phase B — Load into browser/app (becoming a relay-capable participant)

* **`ron-app-sdk`**

  * Loads and decrypts bundle (via password), validates signatures, exposes a minimal identity facade to the app (no raw keys leaked).
  * Provides **relay adapter** and **spend adapter** to the app and to the in-browser gateway client.
* **`omnigate`** (app BFF)

  * Presents APIs to import/check a passport, request rotations, export a fresh cold-bundle, and query capabilities.
* **`svc-gateway`** (ingress)

  * Edge acceptance of the browser sessions; enforces that any relay participation and spending attach the proper capabilities/macaroons.
* **`ron-policy`**

  * Enforces that `.alt` identities have reduced capability scope (e.g., spend caps, relay-only, no posting) vs `.passport`.

**Trust boundary**: Browser/app (untrusted UI) ↔ omnigate/gateway (edge); **no kernel-embedded app logic**.

---

## Phase C — Relay participation (earn ROC for bandwidth)

* **`svc-overlay`**

  * Handles **forwarding/relaying** of eligible public assets across nodes; chooses routes.
* **`arti-transport` + `transport`**

  * Data plane for onion/private vs direct paths; meter actual bytes forwarded at the transport layer.
* **`accounting`**

  * Aggregates **relay-metering** counters (bytes/requests forwarded, time-windowed, de-duplicated, anti-replay).
* **`ron-policy`**

  * Defines **reward schedule** (ROC per GiB, per region/time, anti-abuse rules), per-passport caps, `.alt` throttles, and sybil limits.
* **`ron-ledger`** (canonical economics surface; consolidate here)

  * Mints/credits **ROC** to the user’s spend account based on validated accounting snapshots and current policy.
* **`ron-kms`**

  * Signs mint/credit transactions; key isolation for economics.
* **`ron-audit`**

  * Evidence of relay rewards (who/what/when, link to metering).
* **`metrics`**

  * Relay throughput, cache hit/miss, reward latency histograms.

> **Non-overlap**: overlay = routing/forwarding; accounting = counts; ledger = crediting. If any one tries to do the others’ job → redundancy.

---

## Phase D — Spending ROC on sites/apps (micro-payments)

* **`svc-gateway`**

  * Enforces payment requirements exposed by the site (paywall/metered API).
* **`omnigate`**

  * Presents dApp-level payment DTOs (e.g., “pay-per-view”, “per-MB streamed”), abstracts receipt format.
* **`ron-policy`**

  * Pricing and spend rules (per resource class, per identity type, `.alt` daily caps, jurisdiction flags).
* **`ron-ledger`**

  * Debits from the user’s spend account; issues receipts; ensures idempotency and double-spend prevention.
* **`ron-kms`**

  * Signs spend authorizations; rotation on compromise.
* **`ron-audit`**

  * Immutable proof of payment: payer, amount, resource address, policy version.
* **`svc-index` + `svc-storage` + `svc-overlay`**

  * Content retrieval after payment (read path).

---

## Phase E — Wallet surface & cold storage lifecycle

* **New: `svc-wallet` (missing service)**

  * Presents balances, sub-accounts (earn vs spend), address sets for ROC; initiates withdrawals/rotations; exports **new** password-protected cold bundles.
  * Integrates with `ron-ledger` as the source of truth.
* **`ron-kms`**

  * Manages wallet keys; creates recovery packages (Shamir or M-of-N optional).
* **`ron-audit`**

  * Logs exports, imports, wallet operations (non-sensitive, but sufficient evidence).
* **`omnigate` + `ron-app-sdk`**

  * UX/API to trigger exports/imports, rotation, password updates.

---

# Data model proposals (concise)

## `.passport` (manifest)

* **version**: u32
* **id**: stable identifier (hash over canonical pubkeys + salt)
* **capabilities**: list of caveated grants (relay, spend, post, limits, expiries)
* **pubkeys**: `{ identity, relay, spend }` (public parts)
* **policy\_ref**: digest of the policy version applied at issuance
* **signatures**: `{ kms_sig, self_sig }` (double attestation)
* **revocation**: list of revoked key ids (optional link to CRL endpoint)

## `.alt` (anonymous variant)

* Same schema but:

  * **no stable id** (ephemeral or per-site pseudonyms)
  * **capabilities** minimized (relay + capped spend)
  * **key rotation** enforced more aggressively
  * **anti-linkage**: derivation per relying party (pairwise pseudonyms)

## Encrypted cold bundle (single file)

* **header**: magic, version, kdf params (Argon2id M/T/P, salt), nonce
* **ciphertext**: `{ passport_or_alt_manifest + private keys + recovery meta }`
* **aead\_tag**
* **checksum**: outer Blake3 or included MAC only (prefer MAC only to avoid confusion)
* **footers**: optional recovery shards (if local) or references to offline shard locations

---

# What this scenario uses (explicit crate/service map)

| Concern                           | Crate/Service                  |
| --------------------------------- | ------------------------------ |
| Manifest issuance & bundling      | **(Missing)** `svc-passport`   |
| Key generation/rotation/signing   | `ron-kms`                      |
| Capability schema & rules         | `ron-policy`                   |
| Evidence of issuance/ops          | `ron-audit`                    |
| App-side load/decrypt/use         | `ron-app-sdk`                  |
| App BFF & import/export APIs      | `omnigate`                     |
| Ingress for browsing & payments   | `svc-gateway`                  |
| Name/address semantics            | `naming` (if used by sites)    |
| Address→providers mapping         | `svc-index`                    |
| Routing/forwarding                | `svc-overlay`                  |
| Content store/stream              | `svc-storage`                  |
| Generic TLS / Onion transport     | `transport` / `arti-transport` |
| Relay metering (bytes/reqs)       | `accounting`                   |
| Rewards & spends (credits/debits) | **`ron-ledger`** (canonical)   |
| Balances, withdrawals, exports    | **(Missing)** `svc-wallet`     |
| Ops telemetry                     | `metrics`                      |
| Lifecycle/event spine             | `ron-kernel`, `ron-bus`        |

---

# Missing pieces (actionable, prioritized)

## P0 (must add to make this scenario real)

1. **`svc-passport`** (new): issuance/rotation/revocation + cold-bundle creation (KDF/AEAD) + `.alt` derivation policy.

   * Inputs: `ron-policy` schemas, `ron-kms` key ops.
   * Outputs: signed manifest(s) + encrypted bundle file, audit record.
2. **`svc-wallet`** (new): balances, spend/earn separation, withdrawals, export/import of updated cold bundles.

   * Source of truth: `ron-ledger`; keys via `ron-kms`.
3. **Relay metering hooks** (likely inside `svc-overlay` + `transport` to `accounting`): precise, verifiable byte/flow accounting with anti-replay.
4. **Canonical economics surface**: consolidate to **`ron-ledger`** (deprecate parallel “billing/token” crates as public surfaces; keep as internal libs if needed).
5. **Policy surfaces** for:

   * `.alt` capability minima/caps;
   * ROC reward per GiB windows;
   * spend pricing classes;
   * rotation/revocation semantics.

## P1 (important soon)

6. **Recovery/backup**: optional M-of-N shards (Shamir) for cold bundles (documented in `svc-passport` UX).
7. **Sybil resistance heuristics** for relay rewards (policy+accounting integration; pairwise cost functions).
8. **Revocation distribution**: CRL/OCSP-like fetch on import; short-lived passports/`alt` by default.
9. **Pairwise pseudonyms** in `.alt` to avoid cross-site linkage (derive per relying-party tag).

---

# Redundancy & drift sentinels (apply your golden rule)

* **`svc-passport` vs `svc-wallet`**: issuance/bundling is not the same as balance/withdrawals. Keep them separate; if one starts doing the other’s job, it’s redundancy.
* **`ron-ledger` vs `accounting`**: ledger makes credit/debit artifacts; accounting counts usage. Never conflate.
* **`omnigate` vs `svc-gateway`**: omnigate is app-specific DTOs; gateway is neutral ingress. Don’t let gateway grow BFF logic.
* **`svc-overlay` vs `svc-storage` vs `svc-index`**: routing vs bytes vs mapping; any blending is drift.
* **`.passport` vs `.alt`**: if `.alt` ends up with the same capability footprint as `.passport`, the anonymity story collapses—capabilities must be strictly scoped.

---

# Minimal written acceptance for this scenario (no code)

1. A versioned **manifest spec** for `.passport` and `.alt` (fields above).
2. A **bundle crypto spec** (KDF params, AEAD choice, rotation/CRL rules).
3. A **relay metering spec** (what counts, how we dedupe, how we sign proofs).
4. **Economics rules** (ROC earn/spend tables) in `ron-policy`; **ledger API** is the only source for balances.
5. A **role chart** that shows exactly the crates/services above—so if you can’t justify one without repeating another, it’s redundant.

---

## TL;DR

To make “load passport → relay to earn ROC → spend ROC while browsing → keep an offline, password-protected identity” real, you need two new first-class services—**`svc-passport`** and **`svc-wallet`**—plus relay-metering hooks feeding **`accounting`** and a single canonical **`ron-ledger`** for credits/debits. Keep boundaries sharp (passport/wallet/ledger/overlay/gateway), specify the crypto and capability model, and your architecture stays clean with zero contrived overlaps.


#NEXT PROMPT

Love this scenario—it’s exactly the kind of “money flows + identity + content plane” path that flushes out gaps. Here’s a precise, written blueprint (no scripts), with phases, crates/services, artifacts, and explicit rev-share logic.

# Technical restatement

Every **creator** (journalist/blogger/influencer/etc.) must possess a local identity bundle: a **.passport** (pseudonymous, full-capability) or **.alt** (privacy-focused, minimized capability). Their identity bundle embeds:

* a **b3:** content/creator address (BLAKE3) under a chosen **TLD** (e.g., `b3:… .journal`, `b3:… .blog`);
* one or more **wallet addresses** for receiving rewards.

When a viewer consumes the creator’s content, **revenue is split**: a percentage to the **creator**, and a percentage to the **hosting site/platform**. Splits are governed by policy and recorded as ledger transactions. Privacy: `.alt` may hide/rotate public identifiers but still carries spend/earn subkeys and rev-share caveats.

---

# End-to-end flow (phases → crates/services → why each is uniquely needed)

## Phase A — Identity issuance (passport/alt) with TLD + wallets

**Goal:** issue versioned, signed identity manifests with (1) creator b3 address on a TLD and (2) wallet(s).

* **(Missing) svc-passport**: issues `.passport` and `.alt` manifests; assembles encrypted offline bundle (Argon2id + XChaCha20-Poly1305); inserts:

  * `creator_id`: b3 hash (BLAKE3 over canonical public descriptor);
  * `tld`: chosen namespace (e.g., `.journal`);
  * `wallets`: `{ earn, spend, settlement }`.
* **ron-kms**: generates/derives keys (identity/relay/spend); signs manifests; rotation/revocation.
* **ron-policy**: defines passport vs alt capability matrices (earn/spend/post), rev-share schema references, TLD eligibility rules.
* **ron-audit**: immutable issuance/rotation trail.
* **metrics**: issuance/rotation counters.
* **naming**: (semantics) canonicalizes the creator’s name/label to an addressable record under the TLD (not storing bytes).

**Artifacts:** `.passport`/`.alt` manifests (signed), encrypted bundle, audit entries.

---

## Phase B — Creator registration/binding in the name/index plane

**Goal:** bind the creator’s b3+TLD identity to content endpoints discoverable by the network.

* **naming**: publishes/updates the creator’s name record → canonical address under TLD (e.g., `creator@journal` → `b3:… .journal`).
* **svc-index**: maps the creator’s address → provider set (hosting sites/nodes serving the creator’s pages, feeds, media).
* **ron-bus / ron-kernel**: lifecycle and health; ensure updates propagate cleanly.
* **ron-audit**: records registry bindings/updates.

**Artifacts:** Name record, index entry (address → providers), audit evidence.

---

## Phase C — Hosting platform declares rev-share terms

**Goal:** a site hosts the creator’s content and declares the platform’s wallet + rev-share basis points.

* **ron-policy**: canonical **RevShare Policy** (e.g., `{ creator_bp: 85_00, platform_bp: 15_00, min_fee, caps, jurisdiction flags }`, versioned and referencable).
* **svc-gateway** (site edge): advertises the required payment class and **policy\_ref** (rev-share policy version) for the content endpoints.
* **omnigate** (app BFF for that site): shapes app-facing DTOs for previews/paywalls/pricing (no kernel logic).
* **ron-audit**: evidences policy publication by the platform.

**Artifacts:** Policy doc hash (policy\_ref), platform wallet, advertised price class.

---

## Phase D — Viewer consumes content (pay + fetch)

**Goal:** enforce micro-payment (or metered access) then deliver the bytes.

* **ron-app-sdk** (in the viewer’s browser/app): handles capability/macaroons and payment request, then content retrieval.
* **svc-gateway** (platform edge): enforces that payment or a metered debit is authorized before allowing fetch.
* **ron-ledger** (canonical economics surface): debits viewer’s spend account (or consumes prepaid allowance), **allocates rev-share** per policy\_ref.
* **ron-kms**: signs the spend authorization; rotates if needed.
* **svc-index → svc-overlay → svc-storage**: resolve → route → stream the actual content.
* **transport / arti-transport**: data plane (direct/onion per policy).
* **metrics**: request\_latency, bytes\_streamed, payments\_total.
* **ron-audit**: payment proof and access trail (policy\_ref, amounts, content address).

**Artifacts:** debit receipt (ledger), content delivered, audit entries.

---

## Phase E — Revenue split allocation (creator + platform)

**Goal:** allocate and post two credits from the viewer’s payment: creator share and platform share.

* **accounting**: aggregates view/stream counts and bytes (for analytics or hybrid payout logic); not the money line.
* **ron-ledger**: posts two credit entries atomically:

  * `credit(creator_wallet, amount = pay * creator_bp/10_000, memo: content_addr, policy_ref)`
  * `credit(platform_wallet, amount = pay * platform_bp/10_000, memo: content_addr, policy_ref)`
    Guarantees idempotency and correct rounding rules (e.g., banker’s rounding; platform pennies precedence policy clearly defined).
* **ron-kms**: signs the ledger transaction set.
* **ron-audit**: immutable evidence linking debit → split credits with policy\_ref.
* **metrics**: ledger\_credit\_latency\_seconds, split\_anomalies\_total.

**Artifacts:** paired credit transactions; cross-link to original debit; audit records.

---

## Phase F — Wallet surfaces (balances, statements, withdrawals)

**Goal:** expose balances to creator and platform; enable withdrawals.

* **(Missing) svc-wallet**: balance queries, statements, export of fresh encrypted bundles, and withdrawals (internal transfers or off-ramp).
* **ron-ledger**: single source of truth for balances and statements (earn/spend sub-accounts).
* **interop** (if off-ramping to ROX/fiat): exchange bridge; policy gates.
* **ron-policy**: withdrawal caps/allowlists/AML toggles; regional rules.
* **ron-kms**: signs withdrawals; optional recovery flows.
* **ron-audit**: withdrawal evidence.

**Artifacts:** statements, withdrawal records, audit entries.

---

# Responsibility/uniqueness map (so redundancy pops immediately)

| Concern                                                | Sole owner                 |
| ------------------------------------------------------ | -------------------------- |
| Manifest issuance (.passport/.alt) + offline bundle    | **(Missing) svc-passport** |
| Keys/sign/rotate                                       | ron-kms                    |
| Capability schema & rev-share policy                   | ron-policy                 |
| Evidence/immutability                                  | ron-audit                  |
| Developer/client UX (load/pay/fetch)                   | ron-app-sdk                |
| App-facing APIs (previews/paywall DTOs)                | omnigate                   |
| Edge enforcement (ingress, paywall gate)               | svc-gateway                |
| Naming semantics (b3+TLD canonicalization)             | naming                     |
| Address→providers mapping                              | svc-index                  |
| Routing/forwarding                                     | svc-overlay                |
| Content bytes                                          | svc-storage                |
| Data plane (TLS) / onion plane                         | transport / arti-transport |
| Usage counters/analytics                               | accounting                 |
| **Money line** (debits, credits, **rev-share splits**) | **ron-ledger** (canonical) |
| Balances/statements/withdrawals                        | **(Missing) svc-wallet**   |
| Lifecycle/event spine                                  | ron-kernel, ron-bus        |
| Ops telemetry                                          | metrics                    |

> If any other crate tries to implement a money split, that’s **redundancy**; if gateway/omnigate start keeping balances, that’s **drift**.

---

# Data/contract shapes you’ll want in writing

## In `.passport` / `.alt` manifest

* `creator_id` (b3 digest)
* `tld` (e.g., `.journal`)
* `wallets` `{ earn, spend, settlement }` (one may be shared)
* `capabilities` (earn/spend/post/relay with caveats & TTLs)
* `privacy` (for `.alt`: pairwise pseudonyms; stricter caps)
* `signatures` `{ kms_sig, self_sig }`
* `revocation` list pointer

## RevShare Policy (ron-policy)

* `policy_id` / `policy_ref` (digest)
* `creator_bp`, `platform_bp` (basis points; sum ≤ 10\_000)
* `min_fee`, `caps`, `jurisdiction_flags`
* `rounding` rules and **ordering** (e.g., creator first, platform second, with remainder to creator)

## Ledger API (ron-ledger)

* `debit(viewer_wallet, amount, memo, policy_ref)` → `debit_id`
* `split_credit(debit_id, [{to, amount, memo}], invariant: Σamount == debit.amount)` (atomic)
* `receipt(debit_id)` → full chain (debit + split credits)
* Idempotency keys to prevent double-posting.

---

# Missing or ambiguous (actionable)

## P0 (blockers)

1. **svc-passport** (first-class): manifest issuance for creators (.passport/.alt) including **TLD and wallets**; encrypted bundle creation; revocation/rotation.
2. **svc-wallet** (first-class): balances/statements/withdrawals for both creators and platforms; export refreshed bundles.
3. **ron-ledger** consolidation: make it the **only** public economics surface (fold “billing/token” into internal libs or deprecate). Add **split-credit** atomic API.
4. **RevShare Policy** in ron-policy: formal schema + versioning; include rounding/caps/regional flags; tested invariants.

## P1 (soon after)

5. **Attribution & bot resistance**: tie rewards to legitimate views (unique users/time windows); place heuristics in `accounting` + rules in `ron-policy`.
6. **`.alt` unlinkability concretization**: pairwise pseudonyms and stricter caps; policy-driven rotation.
7. **Index/provider verification**: optional background verifier to ensure index reflects availability (prevents phantom hosts).
8. **Receipts & dispute resolution**: human-readable receipts and a minimal dispute flow (policy + ledger annotations).

---

# Minimal written acceptance (no code)

* A **RevShare Policy** document (creator\_bp/platform\_bp, rounding, caps).
* A **Ledger Split** contract: atomic debit→two credits with idempotency.
* `.passport`/`.alt` **manifest fields** including `tld`, `wallets`, `creator_id (b3)`.
* A **responsibility matrix** (above)—if a crate can’t be justified uniquely in this scenario, it’s redundant.

---

## TL;DR

Viewer pays → **ron-ledger** debits viewer and **atomically splits** to **creator** and **platform** wallets (per **ron-policy**). Identity and wallets come from **.passport/.alt** (issued by **svc-passport**), discoverability via **naming/svc-index**, bytes via **svc-overlay/svc-storage**, and balances/withdrawals via **svc-wallet**. The biggest to-dos: add **svc-passport**, add **svc-wallet**, and lock rev-share semantics into **ron-policy** + **ron-ledger**.


#NEXT PROMPT

Love it—this is a perfect “control-plane + economics + identity” scenario. Here’s a clean, **write-only** blueprint: technical restatement, end-to-end flows, exact crates/services (and why each is uniquely needed), data/contract shapes, and what’s missing.

---

# Technical restatement

RustyOnions needs an **automated domain registration** system for its naming plane. Users (identity via `.passport` or `.alt`) can register names (TLD-scoped labels) by **burning ROC**. Registrations are **time-bound** (e.g., 1–3 years), **renewable**, and **revocable** under policy. Additionally, RustyOnions pre-reserves (“courtesy block list”) \~10,000 high-value clearnet domains so they can be transferred later to the respective brand owners once RustyOnions reaches critical mass. The block list must be **non-assignable** to ordinary users, visible/read-only, and **transferable via proof-of-brand** (e.g., DNS TXT or legal attestation) when the rightful owner requests.

---

# Phases and flows (with crates/services and unique reasons)

## Phase A — Availability & Quote

**Goal:** user checks if `label.tld` is free and learns the ROC burn requirement and term.

* **naming** — canonicalizes `label.tld` (normalization, collision rules).
  *Unique:* semantics/rules only.
* **ron-policy** — pricing/term table (e.g., base burn, premium label classes, TLD-specific rules, term lengths, renewal grace).
  *Unique:* policy schema & evaluation.
* **ron-ledger** — (read path) optional: shows user’s ROC balance (via wallet).
* **(Missing) svc-wallet** — balance surface for ROC (earn/spend).
* **omnigate** — app-facing DTOs: `GET /name/availability?label=…&tld=…` returns `{available, burn_quote, term, policy_ref}`.
* **svc-gateway** — neutral ingress.

**Artifacts:** Availability response with `policy_ref` (digest of rules used).

---

## Phase B — Registration (ROC burn + issuance)

**Goal:** user pays (burns ROC) and receives a **signed registration** bound to their identity.

* **ron-app-sdk** — signs the registration request using keys from `.passport`/`.alt`.
* **svc-gateway** — enforces capability (register), anti-spam quotas.
* **(New) svc-registry** — orchestrates the **registration transaction**:

  1. double-checks availability and block-list,
  2. requests **burn** from ledger,
  3. writes the **name record** (owner keys, term),
  4. publishes to index.
     *Unique:* state machine for name lifecycle (pending → active → expired).
* **ron-ledger** — **burns ROC** (moves to a burn sink) with idempotency; emits a “burn receipt”.
  *Unique:* the money line; do not duplicate elsewhere.
* **ron-kms** — signs the **registration certificate** (service attestation) and countersigns with owner key (optional double attestation).
* **naming** — produces the canonical **NameRecord**; validates policy constraints.
* **svc-index** — binds `label.tld` → provider set (creator or site hosting landing page/ownership proof).
* **ron-audit** — immutable record of registration, including burn receipt and policy\_ref.
* **metrics** — burn latency, registration success/failure.

**Artifacts:**

* `RegistrationCert { name, owner_pub, term_start, term_end, policy_ref, svc_sig, owner_sig? }`
* `BurnReceipt { debit_id, amount, sink, policy_ref }`
* NameRecord in naming + index entry.

---

## Phase C — Renewal / Grace / Expiry

**Goal:** extend term before/after expiry window; enforce grace/hold rules.

* **svc-registry** — state transitions (active → grace → expired → redeemable, etc.).
* **ron-policy** — renewal price schedules, grace periods, redemption penalties.
* **ron-ledger** — burns ROC for renewal; idempotent debits.
* **ron-audit / metrics** — trail and counters.

**Artifacts:** Renewal receipts, updated RegistrationCert.

---

## Phase D — Block-list (“Courtesy Reservations”)

**Goal:** reserve \~10,000 clearnet domains (labels) so ordinary users cannot register them; enable **brand transfer later**.

* **(New) registry seed list**: a signed, versioned **ReservedList** artifact (shipped with svc-registry and publishable via naming/read API).
* **svc-registry** — on lookup, treats reserved names as **unavailable** to the public.
* **ron-policy** — rules for **transfer eligibility** (acceptable proofs, review requirements).
* **ron-audit** — publishes the ReservedList hash/version and any changes.

**Artifacts:**

* `ReservedList { version, generated_at, entries: [ {label, tld, reason: "courtesy"} … ] , svc_sig }` (read-only).
* Availability API returns `{available: false, reserved: true, reserved_reason: "courtesy"}` for such names.

> This list is a **hold**, not ownership. It prevents squatting while staying neutral.

---

## Phase E — Brand Transfer (Reserved → Verified Owner)

**Goal:** assign a reserved name to the rightful brand owner once they prove control.

* **omnigate** — `POST /name/claim` with **proof bundle**:

  * **DNS TXT**: set `_ron-claim.<domain>` → challenge token;
  * or **Web**: well-known URL with token;
  * or **Legal**: signed attestation (last resort).
* **(New) svc-verify** (optional) — performs out-of-band verification (DNS/HTTP).
  *Alternative:* keep this in **interop** if you treat external DNS/HTTP as “outside”.
* **ron-policy** — acceptance criteria and human review toggles.
* **svc-registry** — on verified → issues RegistrationCert to claimant (burn may be waived or symbolic), updates naming/index, records provenance.
* **ron-audit** — full evidence chain.

**Artifacts:** VerificationReport, assigned RegistrationCert, audit trail linking ReservedList → claimant.

---

## Phase F — Dispute / Revocation (edge case)

**Goal:** handle fraud or legal disputes.

* **ron-policy** — due-process rules (freeze, escrow, appeal windows).
* **svc-registry** — can **freeze** a name (cannot transfer/renew) pending resolution; **revoke** with signed RevocationCert.
* **ron-audit** — full trail.
* **ron-ledger** — handles any refunds (if policy allows) to spend accounts; burns remain burns unless explicitly refundable.

---

# Data & contract shapes (concise, versioned)

### Name (canonical)

```text
NameId      = b3(label || "." || tld || policy_ref || owner_pub)
Label       = normalized UTF-8 (NFKC), length & charset policy
TLD         = one of approved TLDs in policy
Owner       = { owner_pub, capabilities: [transfer, update, renew], caveats }
```

### RegistrationCert (signed)

```text
{
  name: "label.tld",
  owner_pub: <hex>,
  term_start: epoch_ms,
  term_end: epoch_ms,
  policy_ref: <b3>,
  burn_amount_roc: u64,
  burn_debit_id: <uuid>,
  svc_sig: <ed25519/sphincs+>,
  owner_sig?: <ed25519>,
  prev_cert?: <hash>   // for renewal chains
}
```

### ReservedList (read-only, signed)

```text
{
  version: u32,
  generated_at: epoch_ms,
  entries: [{ label: "google", tld: "web", reason: "courtesy" }, …],
  svc_sig: <sig>
}
```

### Ledger ops (ron-ledger)

* `burn(from_wallet, amount, sink="naming:register", idempotency_key, policy_ref) -> BurnReceipt`
* `refund(optional, policy-gated)`
* Balance query via **svc-wallet**.

---

# Which crates/services are used (and why—no overlaps)

| Concern                                                     | Crate/Service                                                       | Why uniquely needed                                                              |
| ----------------------------------------------------------- | ------------------------------------------------------------------- | -------------------------------------------------------------------------------- |
| App DTOs / UX (availability, register, claim)               | **omnigate**                                                        | App BFF; shapes user-facing APIs; no kernel logic.                               |
| Ingress (security, rate limiting)                           | **svc-gateway**                                                     | Neutral edge enforcement.                                                        |
| Name semantics (normalization, collision)                   | **naming**                                                          | Canonical rules only (no money).                                                 |
| Registration state machine (active/grace/expired, reserved) | **(New) svc-registry**                                              | Orchestrates lifecycle + writes NameRecord; consults policy; calls ledger/audit. |
| Pricing/terms/eligibility                                   | **ron-policy**                                                      | Single source of truth for costs, terms, reserved behavior, transfer rules.      |
| Money (ROC burn, receipts)                                  | **ron-ledger**                                                      | The money line; idempotent debits/burns.                                         |
| Wallet surfaces (balances)                                  | **(New) svc-wallet**                                                | User/platform balances, statements, withdrawals.                                 |
| Signing certs & revocations                                 | **ron-kms**                                                         | Issuer keys; rotation; PQ-readiness if you choose.                               |
| Immutable evidence                                          | **ron-audit**                                                       | All decisions/transactions auditable.                                            |
| Discoverability (addr → providers)                          | **svc-index**                                                       | After registration, resolution works.                                            |
| Transport/routing (serving name landing page)               | **svc-overlay**, **transport**, **arti-transport**, **svc-storage** | Standard content plane (unmodified).                                             |
| External verification (DNS/HTTP)                            | **interop** *(or new `svc-verify`)*                                 | Talks to clearnet DNS/HTTP.                                                      |
| Lifecycle/coordination                                      | **ron-kernel**, **ron-bus**                                         | Health, config updates, consistency.                                             |
| Ops telemetry                                               | **metrics**                                                         | Rates, latencies, failures.                                                      |

> **Redundancy rule check:**
>
> * Only **ron-ledger** burns ROC.
> * Only **svc-registry** writes/updates name lifecycle.
> * Only **naming** defines normalization/collision rules.
>   Any mixing is drift.

---

# Policy knobs you’ll likely want (all in **ron-policy**)

* **Base burn** per TLD, **premium label classes** (length < 4, dictionary words, reserved prefixes).
* **Term lengths** (e.g., 1y, 2y, 3y), **grace** (30–45 days), **redemption** (optional).
* **Reserved behavior**: block-list is `reserved: courtesy`, not assignable except via **claim** flow.
* **Transfer rules**: DNS TXT/HTTP challenge; optional legal review; fees (zero or nominal).
* **Refund policy**: burns are normally final; policy may allow partial refunds on system error.
* **Anti-squat defenses**: per-identity daily caps, cooldowns, `.alt` stricter caps.
* **PQ-ready signatures**: opt-in (e.g., ed25519 + SPHINCS+ dual-sig).

---

# Courtesy block-list (practical details)

* Input source: generate offline (e.g., top 10k from multiple public lists), dedupe/normalize, store as **ReservedList v1**.
* Governance: versioned, signed by **ron-kms**; changes require governance action per policy.
* Publication: serve the signed list over **omnigate** read API and **naming** read endpoints for transparency.
* Claim UX: encourage brands to **claim early** via DNS TXT verification; no fee or nominal fee; no auctions for reserved entries.

---

# Edge cases to define early

* **Name squatting attempts**: queue/register contention → use **idempotency keys** and **first-valid-burn wins** logic.
* **Simultaneous burns**: ledger guarantees serializable outcome; **svc-registry** checks “pending” locks.
* **Expired reserved transfer tokens**: set TTL on verification challenges.
* **Dispute**: freeze name; keep content resolving or show a neutral interstitial (policy-controlled).
* **TLD growth**: TLD list is policy-driven; adding/removing TLDs updates pricing tables & validators.

---

# What’s missing to ship this scenario (actionable)

## P0 (must add)

1. **`svc-registry`** — domain lifecycle orchestrator (availability → register → renew → transfer → revoke).
2. **`svc-wallet`** — balances/statements/withdrawals for ROC (creators, platforms, registrants).
3. **Ledger split/refund primitives** — you already need split for rev-share; add **burn** & optional **refund** APIs in **ron-ledger** with idempotency and audit hooks.
4. **ReservedList artifact & publishing** — signing via **ron-kms**, read API via **omnigate**/**naming**.
5. **Policy schema** for registration/renewal/transfer/reserved behavior (with `policy_ref` digests used in certs and receipts).

## P1 (soon)

6. **Verification component** — treat as **interop** or a small **svc-verify** to do DNS/HTTP proofs.
7. **Dispute/Freeze** workflow — policy + registry states + receipts.
8. **Premium label rules** — short labels, dictionary words, emoji handling; documented normalization.

---

# Minimal written acceptance (no code)

* A **RegistrationCert** spec (fields/signatures), **BurnReceipt**, and state machine (active/grace/expired/frozen).
* A **ron-policy** document for pricing/terms/reserved/transfer.
* A **ReservedList v1** sample (with signature + hash).
* An explicit **Responsibility Matrix** (above). If any crate can’t be justified uniquely here, it’s redundant.

---

## TL;DR

Add **svc-registry** (domain lifecycle) and **svc-wallet** (balances), keep **ron-ledger** as the **only** place that burns ROC, codify pricing/terms in **ron-policy**, and publish a signed **ReservedList** blocking the top \~10k clearnet domains until brands claim them via DNS/HTTP proof. All other pieces—**naming**, **index/overlay/storage**, **gateway/omnigate**, **kms/audit/metrics**—plug in cleanly with zero overlap.

#NEXT PROMPT

Awesome scenario. Here’s a precise, write-only map that shows how someone can (a) run an app with a **micronode**, (b) **register** a web3 domain, (c) **collect ROC** from user traffic, and (d) **consume map tiles** from other nodes in exchange for a percentage—while keeping clear ownership lines so redundancy pops if it creeps in.

---

# Technical restatement

An app operator uses a **micronode** to host their dApp bundle on RustyOnions. They **register** a TLD-scoped name (ROC **burn**), bind the name to their content, and expose the app via the network plane. When users visit, the app charges/collects **ROC** (earn side). The app also **pulls map tiles** from third-party nodes; a policy-defined **rev-share** allocates a percentage of the app’s revenue to those tile providers (and optionally to any caching/relay nodes) via atomic **multi-split** credits.

---

# End-to-end phases (with crates/services and why each is uniquely needed)

## Phase 0 — Identity & wallet readiness (one time)

* **(Missing) svc-passport**: issues `.passport`/`.alt` (manifest + encrypted bundle) for the operator; embeds wallets.
* **ron-kms**: generates/rotates keys; signs manifests.
* **ron-policy**: capability schema (host/collect/spend).
* **ron-audit**: issuance trail.
* **(Missing) svc-wallet**: exposes ROC balances and addresses for the operator.
* **ron-app-sdk**: loads/decrypts the bundle in operator tooling.

> Uniqueness: issuance ≠ custody (svc-passport vs svc-wallet).

---

## Phase 1 — Stand up the app on a micronode

* **micronode**: single-tenant node that serves the dApp bundle and attaches to the plane.
* **svc-storage**: stores the app’s static bundle (CAS).
* **svc-index**: registers content address → provider (the operator’s micronode).
* **svc-overlay**: makes the app reachable (routing/forwarding).
* **transport / arti-transport**: data plane (direct vs onion).
* **metrics / ron-audit / ron-bus / ron-kernel**: observability + lifecycle.

> Uniqueness: storage = bytes, index = mapping, overlay = routing (no blending).

---

## Phase 2 — Register a web3 domain for the app

* **(New) svc-registry**: domain lifecycle orchestrator (availability → register → renew).
* **naming**: canonical rules for `label.tld`.
* **ron-policy**: pricing/term table; reserved/transfer rules.
* **ron-ledger**: **burns ROC** for registration; emits **BurnReceipt** (idempotent).
* **ron-kms**: signs **RegistrationCert** (issuer) + optional owner countersign.
* **svc-index**: binds `label.tld` → the app’s content/provider(s).
* **ron-audit / metrics**: evidence + counters.

> Uniqueness: only **ron-ledger** moves/“burns” ROC; only **svc-registry** mutates domain state.

---

## Phase 3 — Serve the app to users (collect ROC)

* **svc-gateway**: public ingress; enforces capability/paywall before serving bytes.
* **omnigate**: app-BFF DTOs for previews/payments (no kernel logic).
* **ron-ledger**: debits the viewer’s spend account; issues receipt.
* **svc-index → svc-overlay → svc-storage**: resolve/route/stream the app content.
* **ron-policy**: pricing classes, regional caps, onion/direct split.
* **metrics / ron-audit**: latency/counters & immutable payment/access trail.

> Uniqueness: gateway = neutral edge; omnigate = BFF; money line lives only in ledger.

---

## Phase 4 — Use third-party **map tiles** (and pay a percentage)

Two sub-flows happen per user tile request:

### 4A. Resolve & fetch tiles

* **svc-index**: resolves tile address or a **“tiles” tag** to providers (one or more tile nodes).
* **svc-overlay**: selects route (origin and/or cache); can prefer regional providers.
* **svc-storage**: streams tiles (CAS). Caches may serve hot tiles.
* **transport / arti-transport**: plane details.

### 4B. Pay rev-share (atomic multi-split)

* **ron-policy**: a **Tile RevShare Policy** that says, for the app’s *tile-related* revenue, send:

  * `provider_bp` to the **origin tile provider** (or to each provider proportionally if the app mixes providers),
  * optional `cache_bp` to any cache/relay that served the bytes,
  * remainder to the **app operator** (and, if applicable, a platform slice).
* **accounting**: produces usage snapshots to attribute which provider (and cache) actually served bytes (anti-replay/dedup).
* **ron-ledger**: performs an **atomic multi-split** credit for each debit (or at a batched interval):

  ```
  debit(viewer_wallet, amount_app_view)
  split_credit(debit_id, [
    {to: app_wallet,       amount: A},
    {to: provider_wallet,  amount: P},
    {to: cache_wallet?,    amount: C}
  ])   // Σ == debit amount; idempotent
  ```
* **ron-kms**: signs the split set.
* **ron-audit**: binds debit → credit splits → policy\_ref → usage evidence.

> Uniqueness: *attribution* (accounting) ≠ *credits* (ledger). Policy defines exact bp splits and rounding order.

---

## Phase 5 — Operator settlements & visibility

* **(Missing) svc-wallet**: shows balances/statements for the app operator; withdrawals/off-ramp.
* **interop** (optional): if the operator off-ramps ROC to ROX/fiat.
* **ron-policy**: off-ramp caps/allowlists.
* **ron-audit**: settlement/withdrawal evidence.

---

# Responsibility map (quick redundancy check)

| Concern                                           | Owner                              |
| ------------------------------------------------- | ---------------------------------- |
| Run a small app node                              | **micronode**                      |
| Store/serve app bundle                            | **svc-storage**                    |
| Discoverability                                   | **svc-index**                      |
| Routing                                           | **svc-overlay**                    |
| Data plane (TLS/onion)                            | **transport / arti-transport**     |
| Public ingress / paywall                          | **svc-gateway**                    |
| App-facing APIs                                   | **omnigate**                       |
| Domain lifecycle (register/renew)                 | **(New) svc-registry**             |
| Name semantics                                    | **naming**                         |
| Money (debits, burns, **multi-split** credits)    | **ron-ledger**                     |
| Usage attribution (who served tiles)              | **accounting**                     |
| Keys/signing/rotation                             | **ron-kms**                        |
| Policy (pricing, rev-share, path split, off-ramp) | **ron-policy**                     |
| Evidence/immutability                             | **ron-audit**                      |
| Balances/statements/withdrawals                   | **(New) svc-wallet**               |
| Lifecycle spine / telemetry                       | **ron-kernel / ron-bus / metrics** |

If any other crate tries to, say, “also do credits” or “also own registration state,” it’s redundancy.

---

# Contracts & knobs you’ll want in writing (no code)

## A) **Tile RevShare Policy** (in `ron-policy`)

* `policy_ref` (b3 digest)
* `provider_bp`, `cache_bp` (basis points; sum ≤ 10\_000)
* rounding order (banker’s vs floor; who gets remainder)
* attribution mode: **origin** only vs **origin+cache share**
* min payout threshold (to reduce dust)

## B) **Ledger multi-split** (in `ron-ledger`)

* `debit(viewer_wallet, amount, memo, policy_ref) -> debit_id`
* `split_credit(debit_id, [{to, amount, memo}], invariant: Σ == debit.amount)` (atomic, idempotent)
* `receipt(debit_id)` → full chain (debit + splits)
* batching window option for micro-transactions

## C) **Domain Registration** (from your prior scenario)

* `RegistrationCert` + `BurnReceipt` shapes; reused here

## D) **Attribution Snapshot** (from `accounting`)

* `{ content_addr, provider_id, cache_id?, bytes, window, proofs }`

---

# What’s missing to fully ship this scenario (actionable)

**P0 (must add):**

1. **svc-registry** (domain lifecycle)
2. **svc-wallet** (balances/withdrawals/statements)
3. **ron-ledger multi-split** API (N-way atomic credit)
4. **Tile RevShare Policy** (explicit schema + rounding)
5. **Attribution hooks** in `svc-overlay`/`transport` → `accounting` (who actually served which bytes)

**P1 (soon):**
6\) **Provider declaration** (tile nodes advertise “tiles” capability + price hints through `svc-index` tags)
7\) **Anti-fraud** (sybil & replay controls in accounting/policy)
8\) **Caching share rules** (when does a cache earn vs. only origin?)
9\) **Graceful batching** (ledger lets the app post splits every N seconds to cut overhead)

---

## Happy-path trace (reader’s one request, summarized)

1. Browser hits `appname.tld` → **svc-gateway** checks capabilities → **ron-ledger** debits tiny ROC (policy).
2. App needs a tile → **svc-index** finds provider(s) → **svc-overlay** fetches (maybe via a cache) → **svc-storage** streams bytes.
3. **accounting** attributes the bytes to `provider_id` (and `cache_id` if applicable).
4. **ron-ledger** performs **multi-split**: credits app operator + provider (+ cache).
5. **svc-wallet** updates balances; **ron-audit** records the whole chain.

---

### TL;DR

* Run the app on **micronode**, bind it to a **registered domain** (ROC burn via **ron-ledger** + **svc-registry**), serve via **gateway/index/overlay/storage**.
* Charge users in ROC; for **map tiles**, pay providers using **policy-driven multi-split** credits (**ron-ledger**) backed by **accounting** attribution.
* You’ll need three concrete additions to make this airtight: **svc-registry**, **svc-wallet**, and **ron-ledger multi-split** (plus tile attribution hooks).


#NEXT PROMPT

Here’s how the **ad-buying and revenue loop** would map out in RustyOnions, using your golden-rule analysis (each crate/service must have a unique, non-redundant reason to exist):

---

## Technical Restatement of the Scenario

A user (advertiser) wants to buy an ad. Ads are **opinion-agnostic** assets: creation is centralized (must go through the ad service + moderator approval). Creation requires **burning ROC** (internal credit). Ads are then stored, indexed, and attached to delivery slots. Sites, videos, radio, podcasts, etc. must serve a policy-defined minimum number of ads per unit of content. When ads are displayed, impression/click counters generate ROC spend events (debited from advertiser escrow) and credits flow to publishers and relays. Ads represent the **major sink of ROX (external)** → ROC (internal), so this loop must be economically airtight.

---

## End-to-End Phases

### Phase A — Ad Creation & Burn

* **svc-ads** *(new)*: orchestrates lifecycle (submit creative → burn ROC → pending approval).
* **ron-ledger**: burns ROC into escrow for the ad campaign.
* **ron-kms**: signs ad cert (tamper-evident).
* **ron-policy**: enforces min spend, expiry, targeting limits.
* **ron-audit**: trails creation.
* **metrics**: counters for created ads.
* **Artifacts**: `AdCert {ad_id, creative_hash, spend_cap, expiry, sig}` + `BurnReceipt`.

### Phase B — Moderator Approval (next scenario → svc-mod)

* **svc-mod** *(planned)*: queues pending ads, requires human/mod input.
* **ron-policy**: encodes “opinion-agnostic” acceptance criteria.
* **Artifacts**: `ApprovalCert` (linked to `AdCert`).

### Phase C — Storage & Indexing

* **svc-storage**: stores creative (CAS chunks).
* **svc-index**: registers ad address for retrieval.
* **naming**: canonicalizes ad namespace (`ad:<b3>`).
* **Artifacts**: `IndexEntry {ad_id, addr, providers}`.

### Phase D — Distribution & Serving

* **svc-gateway**: enforces ad slot quotas (X ads per page/video/etc.).
* **omnigate**: app-facing API to fetch eligible ads.
* **svc-overlay** + **transport/arti-transport**: deliver ad content.
* **accounting**: meter impressions/clicks (per-viewer).
* **ron-policy**: defines per-slot minimums, rotation rules.
* **Artifacts**: `ImpressionEvent {ad_id, viewer_id?, ts, slot}`.

### Phase E — Economics & Rev-Share

* **ron-ledger**: debits advertiser escrow, credits publishers/relays.
* **accounting**: rollups of impressions → debit units.
* **ron-policy**: rev-share split % (publisher vs relay vs treasury).
* **ron-kms**: signs debit/credit receipts.
* **ron-audit**: evidence trail (who got paid).
* **Artifacts**: `DebitReceipt {ad_id, amount}`, `CreditReceipt {to, amount}`.

### Phase F — Off-Ramp / External ROX Tie-In

* **interop**: bridge to ROX exchange (ads funded in ROX, burned to ROC).
* **svc-wallet** *(missing)*: advertiser/publisher custody; balances visible.
* **ron-policy**: AML/KYC allowlists if required.
* **Artifacts**: `OffRampTxn {rox_in, roc_out}`.

---

## Responsibility Map (Golden Rule: no redundancy)

| Concern                    | Owning Crate/Service                   |
| -------------------------- | -------------------------------------- |
| Ad lifecycle orchestration | **svc-ads** (new)                      |
| Human approval             | **svc-mod** (next scenario)            |
| Creative storage           | svc-storage                            |
| Creative indexing          | svc-index                              |
| Naming / canonical IDs     | naming                                 |
| Delivery edge              | svc-gateway                            |
| App API / DTO shaping      | omnigate                               |
| Routing / transport        | svc-overlay, transport, arti-transport |
| Metering                   | accounting                             |
| Economic debits/credits    | ron-ledger                             |
| Rev-share policy           | ron-policy                             |
| Signatures                 | ron-kms                                |
| Custody / balances         | svc-wallet *(missing)*                 |
| Off-ramp                   | interop                                |
| Evidence trail             | ron-audit                              |
| Telemetry                  | metrics                                |
| Events / supervision       | ron-bus / ron-kernel                   |

Distinct roles, no overlaps—**svc-ads** and **svc-mod** are new required crates.

---

## What’s Missing / P0 Blockers

1. **svc-ads** — central orchestrator for ad lifecycle (must scaffold in Crate\_Map).
2. **svc-mod** — moderation workflow (manual review).
3. **svc-wallet** — custody for advertiser and publisher balances.
4. **Ledger multi-split API** — atomic split credits for rev-share.
5. **Policy schemas** — enforce x ads per page/video/etc., pricing, expiry.

---

## Minimal Written Acceptance (no code yet)

* Spec for `AdCert`, `ApprovalCert`, `ImpressionEvent`, `Debit/CreditReceipt`.
* ron-policy doc for min slots, rev-share %.
* Crate\_Map.md update: add `svc-ads`, `svc-mod`, `svc-wallet`.
* Ledger extension doc: multi-split semantics.

---

## TL;DR

Ad scenario = **svc-ads** (creation) → **svc-mod** (approval) → **storage/index/naming** (content) → **gateway/omnigate** (delivery, quota enforcement) → **ledger/accounting/policy** (impression-based spend + rev-share) → **wallet/interop** (custody/off-ramp).
**Critical gaps**: new services (ads, mod, wallet), multi-split in ledger, policy schemas.
This loop is economically central: ROX in → ROC burn → ads served → ROC spend → publisher rewards → off-ramp again.

---

Would you like me to draft a **Crate\_Map update** showing where `svc-ads`, `svc-mod`, and `svc-wallet` should slot in, with owners and maturity level, so we can bake them into the blueprint right away?

#NEXT PROMPT

Got it — switching to **content-agnostic** (free-speech preserving).

Quick adjustments to the ad blueprint:

* **Policy stance:** `ron-policy` must encode **content neutrality**: no viewpoint/ideology filters. Allowed checks are *only* objective constraints (malware, illegal content per jurisdiction toggles, deceptive practices, format/size/spec compliance, fraud/spam, impersonation, prohibited tracking).
* **Moderation (`svc-mod`) scope:** rename its mandate to **format/safety/legality review**, not taste/opinion. It verifies:

  1. creative integrity (hash matches, media spec, no malware),
  2. legal/compliance toggles from `ron-policy` (e.g., age-gating when required),
  3. anti-fraud signals (bot nets, click farms) from `accounting`.
     It does **not** judge message viewpoints.
* **Audit transparency:** `ron-audit` records decision + exact **policy clause IDs** triggered; publish an **appeals** path (same content neutrality rule).
* **Delivery rules:** `svc-gateway` quota enforcement stays content-neutral; rotation/eligibility purely on **policy and budget**, not content class.
* **Dispute/abuse:** handle via narrowly scoped policy flags (e.g., proven illegal order, malware) with **revocation certs**; all actions auditable and reversible on appeal.

Everything else (svc-ads, ron-ledger burns/escrow, multi-split rev-share, storage/index/overlay) remains the same—now explicitly under a **content-agnostic** charter.

#NEXT PROMPT

Here’s a mapped-out scenario for **moderators** in the RustyOnions economy, consistent with your “content agnostic / free speech protection” rule and the existing ROX→ROC burn model:

---

## Moderator Onboarding & Vetting

1. **Signup Flow**

   * Any Passport holder (main or alt) may apply to be a moderator.
   * Application requires staking a minimum ROC bond (skin in the game).

2. **Automated Vetting**

   * Identity check: Passport consistency + fraud heuristics (avoid Sybil).
   * Behavior check: prior violation history, spam detection.
   * Initial mod score assigned = neutral baseline (e.g., 100).

3. **Graduated Trust**

   * New mods can only review lower-impact content (ads, low-traffic bundles).
   * Higher trust mods gain access to escalations (e.g., network-wide spam mitigation).

---

## Mod Score System

* Every Passport and Alt has a **mod score** displayed (public transparency).
* **Upward adjustment:**

  * Reports submitted by a user are validated (content is later confirmed to be spam/illegal/terms-violating by quorum or higher-trust mods).
  * Accurate decisions add points to their score.
* **Downward adjustment:**

  * Reports are invalidated or shown to be abuse (false flagging).
  * Frivolous/incorrect reports subtract points.
* **Decay:**

  * Scores decay over time without activity to prevent “resting on laurels.”
* **Thresholds:**

  * High scores = eligibility for elevated moderation roles and larger revenue share.
  * Low scores = demotion to “light moderator only,” or eventual suspension from moderation.

---

## Light Moderation for Everyone

* Every user is by default a “light moderator.”
* Reporting bad content is open to all.
* Reports are routed into a **consensus queue** (multiple independent reports before auto-action).
* This spreads moderation burden across the whole network and ensures scale.

---

## Revenue Split for Moderators

* Ads (the main economic engine) → ROX burned → ROC minted → revenue pool.
* Moderator pool: a percentage (e.g., 10–15%) of burn → redistributed to mods.
* Distribution formula:

  * **Weight 1:** Contribution (number of accurate reports / reviews).
  * **Weight 2:** Mod score multiplier (higher accuracy = higher multiplier).
  * **Weight 3:** Activity tier (ensures small mods still get some share).
* Prevents concentration by whales but rewards consistent, accurate moderation.

---

## System Dynamics

* **Content Agnostic:** Moderation is strictly about spam, fraud, malware, and rule-violations; not about political or opinion content.
* **Checks & Balances:**

  * Automated AI filters → flag candidates.
  * Human mod reports → consensus validation.
  * Appeals channel (escalation to high-score mods).
* **Synergy with Ads:**

  * Mods review ad payloads pre-publication (to filter malware).
  * Ad moderation has higher revenue weighting because it directly protects revenue flow.

---

## Implementation Fit with Current Codebase

Looking at your workspace:

* `ron-auth` + `ron-policy`: passports, identity, and permission enforcement.
* `ron-audit`: could log moderation events for transparency.
* `ron-billing` + `svc-economy`: revenue flows, mod split logic.
* `ron-ledger`: tracks ROC payouts and mod score staking.
* New candidate: **`ron-mod` crate** (to implement mod score tracking, reporting logic, and reward distribution).

---

✅ This model makes **every user a light moderator** while still reserving **formal power + revenue** for those with proven accuracy. It ties directly into your ROX/ROC burn-mint loop, ensuring ad revenue security and protecting free speech (content agnostic, only violations handled).

---

Do you want me to extend this into a **blueprint section** (like your Hardening / Scaling docs) so it can slot directly into your docs set? That way the “Ad Scenario” and “Mod Scenario” sit side by side as formal design docs.

#NEXT PROMPT

Perfect—this is the content-plane’s “truth & provenance” loop. Here’s a clean, write-only blueprint that maps: (1) first-time content registration, (2) duplicate detection and handling, and (3) manifest shape (public vs private) with attribution/provenance—plus exactly which crates/services are touched and why (so redundancy pops if it creeps in).

---

# Technical restatement

When a user uploads a post/photo/video, the client computes a BLAKE3 digest over the canonicalized bytes. That digest is the **content ID**. The system binds this ID to a **media-type TLD** (e.g., `b3:<digest>.photo`, `.video`, `.post`). If later anyone uploads the exact same bytes, the digest matches; the system treats it as a **duplicate**: no new bytes are stored, but the uploader can become an additional **provider**. Every content item has a **manifest** with (a) public fields (discoverable) and (b) private fields (encrypted), and supports **attribution/provenance** (original author, derivative links, license, etc.).

---

# End-to-end flows (with crates/services and why each is uniquely needed)

## Phase A — Client-side canonicalization & hashing

**Goal:** get a stable, collision-resistant ID before touching the network.

* **ron-app-sdk**:

  * Canonicalize bytes (exact, deterministic rules per media type; e.g., no EXIF strip unless policy says so).
  * Compute BLAKE3 digest → `b3:<hex>`.
  * Draft a **ContentManifest** (see schema below), split into `public` and `private` sections; encrypt the private section client-side.
  * Sign an upload intent (capability/macaroons).

*Uniqueness:* SDK does client ergonomics (hashing, encrypting, signing). No other crate should.

---

## Phase B — Ingress & policy checks

**Goal:** accept only well-formed, policy-conformant uploads.

* **svc-gateway**: neutral ingress; enforces auth, size/rate limits, and that the TLD matches declared media type.
* **omnigate**: shapes the app-facing API (`/content/register`, `/content/status`), decoupling frontends from internals.
* **ron-policy**: TLD mapping (MIME→TLD), max size, registration fee rules (if any), manifest field constraints.

*Uniqueness:* gateway = edge enforcement; omnigate = app DTOs; policy = rules. Keep them separate.

---

## Phase C — Dedup aware storage (CAS) and index/provider mapping

**Goal:** store bytes once; add/refresh providers on duplicates.

* **svc-storage** (CAS):

  * Check if `b3:<digest>` exists.
  * If **new**: accept chunked upload, verify digest, commit.
  * If **duplicate**: skip storing; record a hit.
* **svc-index**:

  * Bind `b3:<digest>.<tld>` → provider set (add this uploader’s node as a provider if they opt to host/seed).
  * Maintain provider weights/health for routing.
* **svc-overlay** + **transport / arti-transport**: routing & data plane for later GETs.

*Uniqueness:* storage = bytes, index = address→providers, overlay/transport = path/pipe. No blending.

---

## Phase D — Content registration & naming

**Goal:** create the **canonical name record** that binds the content ID to a TLD and publish metadata.

* **naming**: normalize and register the name `b3:<digest>.<tld>` (pure semantics, no money).
* **(New/Extend)** **svc-registry** (content lane):

  * Validate the submitted **ContentManifest.public** against policy.
  * Persist the **public** manifest fields in a metadata store; seal in an immutable log.
  * Store the **private** section as an encrypted blob (CAS) referenced by hash (opaque to the platform).
  * Attach **owner\_pub** and time bounds; issue a **ContentCert** (service-signed attestation that the name↔digest↔manifest binding is valid).
* **ron-audit**: append evidence: who registered what, when, with which policy version.
* **(Optional)** **ron-ledger**: take a small anti-spam **registration fee** (ROC burn or escrow), if enabled by policy.

*Uniqueness:* naming = canonical string rules; registry = lifecycle/metadata binding; audit = evidence; ledger = money (if used).

---

## Phase E — Duplicate uploads (same bytes)

**Goal:** avoid re-storing, but properly reflect new participants and attribution.

* **svc-storage**: returns “already exists” with canonical `b3:<digest>`.
* **svc-index**:

  * Offer the uploader to **become a provider** for that content (opt-in hosting).
  * If accepted, add them to provider set; if not, do nothing.
* **svc-registry**:

  * **No new ContentCert** (the binding already exists).
  * Optionally record a **RehostEntry** (uploader became a host of existing content) for provenance/analytics.
* **ron-audit**: log the duplicate encounter and any provider change.

*Attribution on duplicates:* the **first ContentCert timestamp** is the “original registration.” Later identical uploads are treated as rehosts; not new works.

---

## Phase F — Attribution, provenance, derivatives

**Goal:** represent authorship and downstream relationships cleanly.

* **ContentManifest.public.attribution**:

  * `author_id` (passport/alt id), `author_signature` over the digest, `license`, optional `source_claim` (URL or name on RustyOnions).
* **ContentManifest.public.provenance**:

  * `parents: [b3:<digest>.<tld>]` for derivatives/remixes, each with role (`transform`, `excerpt`, `thumbnail_of`, etc.).
* **ron-policy**: validation rules: what counts as a derivative vs duplicate, required fields per media class.
* **svc-mod** (if disputes): handles **authorship disputes** or illegal content reports (content-agnostic beyond legality/fraud/malware).
* **ron-audit**: immutable chains of claims, disputes, resolutions.

*Uniqueness:* attribution lives in the manifest; policy defines minimums; mod handles disputes—not opinions.

---

# Manifest schema (concise, versioned)

```text
ContentManifest v1
------------------
cid:          "b3:<hex>"                   // digest over canonical bytes
tld:          ".photo" | ".video" | ".post" | ...
media_type:   "image/jpeg" | "video/mp4" | "text/markdown" | ...
size_bytes:   u64
chunking:     { algo: "fixed", chunk_size: 65536 } | ...
created_ts:   epoch_ms

public: {
  title?:            string
  description?:      string
  tags?:             [string]
  attribution: {
    author_id:       "<passport_or_alt_id>"
    author_pub:      "<hex>"
    author_signature:"<sig over cid+tld>"
    license?:        "CC-BY-4.0" | "All Rights Reserved" | ...
    source_claim?:   "b3:<parent>.<tld>" | "https://..."
  }
  provenance?: {
    parents?:        ["b3:<digest>.<tld>", ...]
    roles?:          ["transform","excerpt","thumbnail_of", ...]
  }
  visibility: {
    listed:          bool          // discoverable via search/browse
    nsfw?:           bool          // policy flags; not opinion
  }
}

private: {                                  // encrypted blob, SDK-side
  contact?:        string
  revenue_splits?: [{to_wallet, bp}]
  geo_hints?:      ...                       // e.g., preferred regions for hosting
  notes?:          string
} -> stored as CAS blob with hash reference

owner_pub:        "<hex>"                   // registering key
kms_sig:          "<svc-registry signature over (cid|tld|public_hash|owner_pub|policy_ref)>"
policy_ref:       "b3:<digest-of-policy-doc>"
```

**ContentCert (attestation)**

```text
{
  cid, tld, owner_pub, policy_ref, public_hash, issued_at, expires_at?,
  svc_sig, prev_cert? // for metadata updates without changing bytes
}
```

> Public vs private: the **public** section is plaintext and indexable; the **private** section is encrypted client-side (SDK) and stored as a separate CAS object referenced by hash from the public part. The platform never sees its contents.

---

# Duplicate logic (precise)

1. **Byte-level duplicate** (exact same canonical bytes):

   * **storage**: “exists” → no write
   * **index**: optionally add uploader as provider
   * **registry**: do not mint a new ContentCert; record RehostEntry (optional)
   * **attribution**: original registration (earliest `issued_at`) remains the “author of record”; later rehosts don’t change authorship

2. **Near-duplicate** (same picture re-encoded/resized):

   * **storage**: new digest → treated as a new object
   * **registry**: manifest **provenance.parents** should reference the parent work if the uploader declares it; otherwise it’s independent
   * **policy**: can require a “derivative claim” when EXIF or perceptual hash suggests close similarity (advisory only; content-agnostic)

3. **Malicious duplicate claim** (claiming authorship on existing CID):

   * **registry** refuses: CID already has an owner; offer rehost role only
   * **mod**: if dispute escalates, run authorship dispute workflow; audit everything

---

# Which crates/services are used (and why)

| Concern                                        | Owner (unique)                                     |
| ---------------------------------------------- | -------------------------------------------------- |
| Client canonicalize + hash + encrypt private   | **ron-app-sdk**                                    |
| Edge auth/quotas + TLD/MIME gate               | **svc-gateway**                                    |
| App-facing API (register/status)               | **omnigate**                                       |
| TLD semantics, name normalization              | **naming**                                         |
| Byte storage (CAS), digest verify              | **svc-storage**                                    |
| Address → providers mapping (rehosts)          | **svc-index**                                      |
| Routing / data plane                           | **svc-overlay**, **transport**, **arti-transport** |
| Content lifecycle & metadata binding           | **svc-registry** *(extend to content lane)*        |
| Policy (MIME→TLD, fees, manifest minima)       | **ron-policy**                                     |
| Evidence / trail (register, duplicate, rehost) | **ron-audit**                                      |
| Economics (optional anti-spam fee)             | **ron-ledger**                                     |
| Disputes (authorship/illegal)                  | **svc-mod**                                        |
| Telemetry                                      | **metrics**                                        |
| Lifecycle/events                               | **ron-kernel**, **ron-bus**                        |

> If **svc-storage** or **svc-index** start storing manifest metadata, that’s redundancy. If **svc-registry** starts handling bytes, that’s drift. Keep the walls sharp.

---

# Minimal contracts/knobs to write down (no code)

* **Canonicalization rules per media type** (what bytes go into the hash; avoid any lossy transforms unless explicitly required by policy).
* **MIME→TLD mapping** (policy).
* **Registration economics** (fee on/off; ROC burn or escrow; exemptions).
* **Duplicate policy** (exact/near-duplicate handling; rehost semantics).
* **Manifest required fields** (per TLD; attribution minimums).
* **Provenance vocabulary** (roles and expectations).
* **Dispute process** (mod workflow; who can overturn what; strict content-agnostic scope).

---

# What’s missing to ship this (actionable P0s)

1. **Extend `svc-registry` with a “content” lane** (not just domains): lifecycle & ContentCert issuance for `b3:<digest>.<tld>` + manifest binding.
2. **SDK canonicalization spec** per media class (lock this to avoid hash churn).
3. **Public/Private manifest split** (SDK encryption, CAS reference wire format).
4. **Duplicate & rehost API** in **svc-index/registry** (atomic add-provider flow; idempotent).
5. **Policy tables** for MIME→TLD, manifest minima, and (optional) registration fee schedule.

*(P1)* perceptual-hash advisory and derivative nudges; authorship dispute flow in **svc-mod**.

---

## TL;DR

* Hash on the client (BLAKE3) → name it `b3:<digest>.<tld>` → store once (CAS), bind metadata via **svc-registry**, and publish providers via **svc-index**.
* Duplicates never store bytes again; new uploaders can become providers (rehosts).
* A versioned **ContentManifest** carries **public** (searchable) and **private** (client-encrypted) fields, with **attribution/provenance** baked in.
* Keep ownership lines crisp: SDK (hash/encrypt), storage (bytes), index (providers), registry (metadata/cert), policy (rules), audit (evidence).

#NEXT PROMPT

Awesome—this is the “hash-addressed CDN” scenario. Here’s a crisp, write-only map showing how a third-party runs a CDN on RustyOnions, gets **automatic ROC micropayments for bandwidth**, and how **hash routing** works from a site’s `<img>` tag all the way to bytes served—plus exactly which crates/services are involved and why.

---

# Technical restatement

A CDN operator brings up nodes that cache and serve **immutable, hash-addressed** assets (e.g., `b3:<digest>.photo`, `.video`, `.css`). They can **prefetch/pin** daily viral/trending content or static site assets. When a site embeds an address (e.g., `<img src="ron://b3:<digest>.photo">`), the viewer’s client resolves providers; the **nearest/healthiest CDN node** serves the bytes. **Accounting** measures the exact bytes delivered, and **ron-ledger** performs **atomic micropayments** (ROC) to the CDN (and, if policy says so, a split to the origin/publisher). Integrity is guaranteed by BLAKE3 verification; “cache poisoning” can’t succeed because content is immutable by ID.

---

# End-to-end phases (who does what, and why it’s unique)

## Phase 0 — CDN operator onboarding

* **(Missing) svc-wallet**: operator’s ROC balance and deposit address.
* **svc-passport (missing)** + **ron-kms**: operator’s identity bundle and serving keys.
* **ron-app-sdk**: loads the bundle in admin tooling.
* **ron-policy**: declares CDN capability class and pricing knobs (e.g., ROC/GiB ceilings, regional constraints).

> Uniqueness: wallet=custody; policy=rules; kms=keys. No overlaps.

---

## Phase 1 — Declare CDN capability & advertise inventory

* **svc-index**

  * Operator registers as a **provider** with a `cdn` capability tag and **provider metadata**: regions, capacity, price hints (optional), QoS signals.
  * For prefetch/pin: the node **adds itself as a provider** for targeted IDs (trending list, site asset manifest).
* **svc-storage**

  * **Pin** assets locally (CAS) or **warm** the cache by fetching from existing providers.
* **svc-overlay** + **transport / arti-transport**

  * Ensure reachability (routing) and secure data plane.

> Uniqueness: index=who can serve; storage=bytes; overlay/transport=path/pipe.

---

## Phase 2 — Site embeds hash-addressed assets

Example:

```html
<img src="ron://b3:7d…9a.photo?hint=web&maxprice=5&multiproof=on">
<link rel="stylesheet" href="ron://b3:ab…ef.css">
<video src="ron://b3:cd…01.video" controls>
```

* **MIME→TLD** handled by **ron-policy**.
* Optional URL params: **price caps**, **routing hints**, **proof flags** (policy-dependent).

> Immutable by design: any byte change → new BLAKE3 → new address.

---

## Phase 3 — Client resolution & **hash routing** (the request path)

1. **ron-app-sdk** (in the browser/app) parses `ron://` URL → extracts `b3:<digest>.<tld>`.
2. **naming** normalizes the name (pure semantics).
3. **svc-index** returns **provider set** for that ID, including CDN nodes (with health/latency/region/pricing metadata).
4. **svc-overlay** selects the **best route** (policy-aware): proximity, health, price cap, congestion.
5. **svc-gateway** at the chosen provider terminates the request; **omnigate** presents any paywall DTOs.
6. **svc-storage** streams verified chunks; **transport / arti-transport** handles the data plane (direct vs onion).

> “Hash routing” = name→providers (index) + policy-aware path selection (overlay). Integrity verified on the fly by chunk/whole hash.

---

## Phase 4 — **Micropayments for bandwidth** (earn ROC)

Two standard modes; choose per policy:

### A) **Per-request debit** (synchronous)

* **ron-ledger** debits the viewer (or site’s sponsor wallet) for the expected bytes/tariff before or during stream; rate-limited by **svc-gateway**.
* **accounting** confirms actual bytes served; any delta is reconciled (top-up or refund) post-delivery.
* **ron-ledger** performs an **atomic multi-split** credit:

  * `{to: cdn_wallet, amount: P}`
  * `{to: origin_wallet?, amount: O}` (optional origin royalty)
  * `{to: relay/cache_wallet?, amount: C}` (if policy pays intermediate caches)
  * invariant: `P+O+C == debit_amount`, rounding order fixed in policy.

### B) **Batched debit** (asynchronous)

* **accounting** aggregates signed usage snapshots over a short window (e.g., 30–60s).
* **ron-ledger** posts an **idempotent batch debit** → multi-split credits to CDN and any policy recipients.

> Uniqueness: accounting=attribution/counts; ledger=money; policy=split/price; gateway=enforcement. Keep them separate.

---

## Phase 5 — Trending/prefetch loop (operator growth)

* Operator ingests external “trending/viral” feeds (outside RO network) via **interop** or publishes a **pin manifest** (a list of addresses) that other CDNs can reuse.
* **svc-index** is updated with provider ads for those items; **svc-storage** pins/warm-caches.
* **metrics** exports hit ratio, tail latency, served bytes by region/content class; **ron-audit** records payouts and policy refs for transparency.

---

# Duplicate, cache, and integrity behaviors

* **Duplicates** don’t exist at the byte level: same bytes → same `b3` → same object. Multiple CDNs simply appear as more **providers** for that same address (index update).
* **Cache poisoning** is prevented: wrong bytes ≠ expected BLAKE3 digest → client rejects.
* **Invalidation** isn’t needed for immutable assets; new versions are new addresses. (Optional: **alias manifests** can map “logical name” → current address if a publisher wants version rolling; that’s a separate naming record, not mutation.)

---

# Money/pricing knobs (in **ron-policy**)

* **Tariff class per TLD** (e.g., `.photo`, `.video`, `.css`) with ROC/GiB targets.
* **Regional price multipliers** (operator can advertise hints; policy applies caps).
* **Rounding & remainder order** for multi-split (e.g., prefer CDN, then origin, remainder to treasury).
* **Min payout threshold / batching window** to avoid dust.
* **Anti-abuse**: viewer/session rate caps; free-tier bytes; bot heuristics from **accounting**.

---

# Minimal contracts (no code, but must be written)

**A) Provider advertisement (svc-index)**

```
ProviderAd v1 {
  provider_id,
  capabilities: ["cdn"],
  regions: ["NA","EU",...],
  qos: {latency_p50_ms, availability_5m},
  price_hint_roc_per_gib?: u64,
  pinned: ["b3:<digest>.<tld>", ...] // optional inventory manifest
}
```

**B) Usage snapshot (accounting)**

```
UsageSnapshot v1 {
  content: "b3:<digest>.<tld>",
  provider_id,
  consumer_hint: <viewer/session id>,
  bytes_served: u64,
  window_start/end_ms,
  proof: <provider-signed; optional consumer co-sig>
}
```

**C) Ledger ops (ron-ledger)**

```
debit(consumer_wallet, amount, memo="cdn:<content>", policy_ref) -> debit_id
split_credit(debit_id, [{to, amount, memo}], Σ==debit_amount) // atomic, idempotent
batch_debit_and_split([snapshots], policy_ref) -> receipt
```

**D) Hash URL grammar**

```
ron://b3:<hex>.<tld>[?hint=web&maxprice=5&multiproof=on]
```

---

# Full crate/service map (golden-rule ownership)

| Concern                                      | Sole owner                              |
| -------------------------------------------- | --------------------------------------- |
| Identity & keys                              | **svc-passport (missing)**, **ron-kms** |
| Custody/balances                             | **svc-wallet (missing)**                |
| Policy (price/splits/caps)                   | **ron-policy**                          |
| App/client behavior (URL parse, verify, pay) | **ron-app-sdk**                         |
| Ingress & paywall                            | **svc-gateway**                         |
| App DTOs                                     | **omnigate**                            |
| Name semantics                               | **naming**                              |
| Providers/ads & discovery                    | **svc-index**                           |
| Routing & reachability                       | **svc-overlay**                         |
| Data plane (TLS/onion)                       | **transport / arti-transport**          |
| Content storage (CAS)                        | **svc-storage**                         |
| Attribution/metering                         | **accounting**                          |
| Money (debits, **multi-split** credits)      | **ron-ledger**                          |
| External feeds/bridges                       | **interop**                             |
| Evidence & transparency                      | **ron-audit**                           |
| Lifecycle/telemetry                          | **ron-kernel / ron-bus / metrics**      |

If any other crate tries to “also do credits” or “also keep provider ads,” it’s redundancy; if storage starts enforcing paywalls, that’s drift.

---

# What’s missing to ship this (P0)

1. **svc-wallet** (balances/statements/withdrawals for CDN operators).
2. **ron-ledger multi-split** + **batch** APIs (atomic N-way credits, idempotent, with rounding order).
3. **Provider advertisement schema** and **price hints** in **svc-index**.
4. **Attribution hooks** in **svc-overlay/transport → accounting** for exact served bytes (consumer-hint privacy-safe).
5. **Client URL grammar** and **SDK resolver** behaviors (max price, proof flags).

*(P1)*: optional co-signed usage proofs; QoS-aware routing weights; operator pin manifest format.

---

## Happy-path trace (one image tag)

1. Browser hits a page; `<img src="ron://b3:… .photo">`.
2. **SDK** parses; **naming** normalizes; **index** returns providers; **overlay** picks a CDN node.
3. **gateway** enforces micro-pay; **ledger** debits viewer (or sponsor).
4. **storage** streams chunks; **SDK** verifies BLAKE3; **accounting** records bytes.
5. **ledger** multi-splits: credit CDN (and optional origin/relay).
6. **wallet** shows the CDN operator’s updated balance; **audit** links debit→split with `policy_ref`.

---

### TL;DR

CDN nodes **pin or warm** hash-addressed assets, advertise capability in **svc-index**, and serve via **overlay/storage** with integrity guaranteed by BLAKE3. Viewers (or sites) pay **ROC micropayments**; **ron-ledger** does **atomic multi-split** credits (CDN + optional origin/relay) driven by **ron-policy**, using **accounting** for byte-true attribution. Boundaries stay sharp; redundancy can’t creep in.


#NEXT PROMPT

Great point—you want a **holding period for ROC before cashing out to ROX** to protect against bad actors, DMCA takedowns, or IP theft. This is consistent with your network’s **zero-trust, evidence-first ethos**. Based on the blueprints you’ve shared (esp. **Full\_Project\_Blueprint.md**, **Interop\_Blueprint.md**, and **Hardening\_Blueprint.md**) here’s how that maps out:

---

## Technical Restatement

When a participant earns ROC (from ads, CDN bandwidth, relay credits, etc.), those credits don’t immediately convert to ROX (the off-network, liquid token). Instead, ROC goes into a **pending escrow pool** with a **time-locked holding period** (e.g., 7–30 days, policy-configurable).
During this window:

* If DMCA/IP takedown or fraud is reported and validated → those ROC credits are clawed back.
* If no disputes → ROC “vests” and becomes eligible for off-ramp to ROX.

This creates a safety buffer, disincentivizes abuse, and gives moderators/reporters time to act.

---

## End-to-End Flow

### Phase A — Earning ROC

* Crates: `svc-gateway`, `svc-overlay`, `accounting`, `ron-ledger`
* ROC is credited as **PendingBalance** with timestamp.
* `ron-ledger` issues `CreditReceipt{status: pending, unlock_at: epoch_ms}`.

### Phase B — Holding/Escrow

* Crates: `ron-ledger`, `ron-policy`
* Policy defines `HOLD_PERIOD_DAYS`, minimums, exceptions (e.g., trusted publishers).
* `ron-ledger` maintains two balances per user: `pending` vs. `available`.

### Phase C — Review & Challenges

* Crates: `svc-mod`, `ron-audit`, `ron-policy`
* Reports filed → svc-mod queues them.
* If report validated, corresponding `pending` credits are voided or slashed.
* Audit log (`ron-audit`) records decision + link to CreditReceipt.

### Phase D — Unlock & Conversion

* At `unlock_at` with no disputes, `ron-ledger` automatically shifts funds:

  * `pending → available`
  * Issues `UnlockReceipt{credit_id, amount, policy_ref}`

### Phase E — Off-Ramp to ROX

* Crates: `svc-wallet`, `interop`, `ron-ledger`
* User requests conversion: ROC → ROX bridge.
* `ron-ledger` debits `available` balance, sends through `interop` to ROX.

---

## Artifacts

* **CreditReceipt**: `{amount, status=pending, unlock_at, sig}`
* **UnlockReceipt**: `{credit_id, amount, status=available}`
* **DisputeRecord**: `{credit_id, outcome=slashed|released, reason}`

---

## Responsibility Map

| Concern              | Owning Crate/Service |
| -------------------- | -------------------- |
| Credit Issuance      | `ron-ledger`         |
| Holding/Unlock Rules | `ron-policy`         |
| Disputes/Reports     | `svc-mod`            |
| Evidence Trails      | `ron-audit`          |
| Balances             | `svc-wallet`         |
| Off-Ramp             | `interop`            |
| Supervision          | `ron-kernel`         |

---

## What’s Missing (P0)

1. **Ledger extension** for `pending vs available` state.
2. **Policy schema** for hold durations/exemptions.
3. **Unlock job** (ron-bus scheduled event).
4. **Dispute workflow** in `svc-mod` to hook into pending credits.

---

✅ This design matches the invariants:

* **Content-agnostic/free speech** (only slashed for IP/fraud, not opinions).
* **Zero-trust** (audit trails + time for disputes).
* **Economic soundness** (ROX inflow → ROC burns → ROC earning → time-locked conversion).

---

Would you like me to **draft a formal spec block** (like the others Grok generated: `CreditReceipt`, `UnlockReceipt`, `DisputeRecord`, plus state machine diagram for pending → available → off-ramp) so it can slot directly into `Crate_Map.md` and `Full_Project_Blueprint.md`?

#NEXT PROMPT

That’s a solid plan ✅

You’ve now walked through scenarios for:

* **Passports/.alt issuance** (identity bundles, cold storage, scope separation)
* **Creator revenue** (b3-hashed TLDs, rev-share with sites)
* **Automated domain registration** (ROC burns, reserved list, brand claims)
* **Micronodes & app hosting** (earn ROC, multi-split rev-share on dependencies)
* **Ads system** (content-agnostic lifecycle, escrow burns, serving quotas)
* **Moderation** (light users as reporters, mod scores, staking, rev-share pools)
* **Content uniqueness** (BLAKE3 + TLD, deduplication, attribution manifests)
* **CDN services** (bandwidth metering, automatic ROC micro-payments)
* **Holding periods** (pending → available balances, DMCA/fraud clawbacks)

Together, these map almost the **entire RustyOnions economy + governance loop**. Putting them into a single README gives you:

* A **design overview** of flows and their crate dependencies.
* A reference for **crate\_map verification** (to spot gaps/redundancies).
* A launchpad for the next instance to cross-review against **blueprints**.

👉 Suggestion: Structure the README like this:

1. **Introduction** (what this document is)
2. **Scenario Index** (list of all scenarios with links/anchors)
3. **Detailed Scenarios** (each mapped with phases, crates, artifacts, trust boundaries, P0 gaps)
4. **Responsibility Matrix Summary** (which crates own which concerns)
5. **Next Steps** (blueprint cross-review, crate\_map updates, TLA+ specs, etc.)

Do you want me to **compile all of the mapped scenarios into a single README.md** (structured and ready to paste into your repo), or just outline the skeleton and let you paste each scenario in?

#NEXT PROMPT

Love the ambition in that README — it still sells the right vision — but a few things are out of sync with where our scenarios/blueprints landed. I’ll (1) call out the key gaps/drift risks I see from the README, then (2) add a fresh set of **written-only** scenarios you can drop into your scenarios README to pressure-test those gaps. No scripts, just flows + unique crate reasons per the golden rule.

---

# Quick review: what’s missing or drift-y in the current README

* **Gateway vs Omnigate naming drift.** The diagram labels “gateway (svc-omnigate)”. In our model:

  * **svc-gateway** = neutral ingress/paywall/quotas.
  * **omnigate** = app BFF/DTO shaping.
    If one is doing the other’s job, that’s redundancy.

* **Identity service mismatch.** README shows a `svc-identity` for keys/tokens, while our scenarios put key custody and signing in **ron-kms** (+ new **svc-passport** for issuance/bundling). Decide: either keep `svc-identity` and scope it to *only* do issuance/bundling (becoming `svc-passport`), or drop it and keep kms + passport clearly split.

* **Wallet & registry not first-class.** The README doesn’t surface **svc-wallet** (balances, holds, withdrawals) or **svc-registry** (domains & content certificates). Our scenarios rely on both as P0s.

* **Economics plumbing incomplete.** We repeatedly need:

  * **ron-ledger multi-split** (atomic N-way credits)
  * **pending → available** balances with **holding periods**
  * clear **policy** for rounding/thresholds.
    README mentions rewards and billing but not these invariants.

* **Ads & moderation not modeled yet.** The doc hints at “privacy-first ads” and “moderators” but not the concrete lifecycle we mapped: **svc-ads**, **svc-mod**, rev-share, content-agnostic policy, stake/score/decay.

* **Content manifests need public/private split.** README shows a single `Manifest.toml`. Our scenarios require a **public** indexable section + **private** client-encrypted blob referenced by hash.

* **Two-plane diagram is light on storage/index split.** It loosely assigns both to overlay in places. Keep: **svc-index = mapping**, **svc-storage = bytes**, **svc-overlay = routing**.

* **Addressing scheme terms.** README uses 🦀/crab URIs; our flows use `ron://b3:<hex>.<tld>`. Fine either way—just pick one canonical grammar and write it down (incl. query params like `maxprice`, `proof`, etc.).

* **Special TLDs explosion.** Big list is fine aspirationally, but policy needs **MIME→TLD** and per-TLD **pricing/requirements** before we promise them.

* **Private plane.** Good Tor posture, but mailbox/E2E moderation & abuse surfaces aren’t described; our scenarios cover score-based moderation and appeals in **svc-mod**.

---

# Additional written scenarios to add (all “golden-rule” mapped)

Each one is short: goal → crates (with unique reason) → artifacts/outputs. Drop them into your scenarios README.

---

### S17. Policy Hot-Reload with Safe Rollback

**Goal:** Reload a bad policy safely; auto-rollback on health regressions.
**Crates (unique reasons):**

* **ron-policy** (source of truth; versioned; staged apply).
* **ron-kernel** (coordinates canary + rollback window).
* **ron-bus** (PolicyUpdated{version}, Rollback{from,to}).
* **metrics** (error budgets; apply/rollback counters).
  **Artifacts:** `policy_ref` b3 digest; rollback decision log in **ron-audit**.
  **Sentinel:** if any service hard-codes policy, flag drift.

---

### S18. Key Rotation & Bundle Re-Issue (.passport/.alt)

**Goal:** Rotate compromised keys without breaking content ownership.
**Crates:**

* **ron-kms** (generates new keys; signs rotation cert).
* **svc-passport (new)** (re-issues encrypted bundle; preserves identity lineage).
* **svc-registry** (updates `owner_pub` in ContentCert with chain).
* **ron-audit** (rotation trail).
  **Artifacts:** `RotationCert{prev_pub,new_pub,policy_ref}`.
  **Sentinel:** if registry stores private keys → drift.

---

### S19. Lost Password Recovery (Shamir M-of-N)

**Goal:** Recover a locked bundle via social recovery.
**Crates:**

* **ron-kms** (splits recovery secret; recombine).
* **svc-passport** (repack bundle after recovery).
* **ron-policy** (M,N thresholds; cooldowns).
* **ron-audit** (recovery evidence).
  **Artifacts:** `RecoveryEvent{m,n,participants}`.
  **Sentinel:** if wallet handles recovery secrets → redundancy.

---

### S20. DMCA/IP Dispute Against Pending Credits

**Goal:** Claw back ROC before off-ramp.
**Crates:**

* **svc-mod** (intake, quorum, appeal).
* **ron-ledger** (holds pending; slashes on upheld dispute).
* **ron-policy** (hold duration, dispute SLA).
* **ron-audit** (decision + clause IDs).
  **Artifacts:** `DisputeRecord{credit_id,outcome}`; balance transitions.
  **Sentinel:** if gateway refuses content by opinion → drift.

---

### S21. Alias/Version Roll (Mutable logical name → immutable b3)

**Goal:** “latest” pointer without mutating content.
**Crates:**

* **naming** (alias record semantics).
* **svc-registry** (issues AliasCert mapping logical → b3 target).
* **svc-index** (optional prefetch on alias advance).
* **ron-audit** (alias change log).
  **Artifacts:** `AliasCert{alias, target_b3, prev}`.
  **Sentinel:** if storage overwrites bytes → drift.

---

### S22. Perceptual Near-Duplicate Advisory (not blocking)

**Goal:** Nudge derivative attribution for re-encodes/resizes.
**Crates:**

* **interop** (optional pHash module; not authoritative).
* **svc-registry** (surfaces advisory; uploader can declare parents).
* **ron-policy** (advisory threshold; no opinion).
  **Artifacts:** `SimilarityHint{source_b3, score}`.
  **Sentinel:** advisory only; no silent reassignments.

---

### S23. Federation Split-Brain & Healing

**Goal:** Divergent provider views converge safely.
**Crates:**

* **svc-overlay** (peer selection; gossip).
* **ron-policy** (federation allowlists/weights).
* **ron-bus** (member join/leave).
* **metrics** (convergence time, partition count).
  **Artifacts:** `FederationState{view_id, members}`; audit of policy used.
  **Sentinel:** if index assumes single authority → drift.

---

### S24. Quota Exhaustion & Backpressure

**Goal:** Protect the edge under traffic spikes.
**Crates:**

* **svc-gateway** (429/503, token buckets per cap).
* **ron-policy** (RPS/byte ceilings per class).
* **metrics** (shed counts, queue depth).
* **ron-kernel** (shed gates/overload mode broadcast).
  **Artifacts:** `OverloadEvent{class, duration}`.
  **Sentinel:** if overlay starts rate-limiting itself → drift.

---

### S25. Disaster Recovery: Partial Storage Loss

**Goal:** Rehydrate from providers; keep names intact.
**Crates:**

* **svc-index** (find healthy providers).
* **svc-overlay** (bulk fetch orchestration).
* **svc-storage** (rehydrate; verify b3).
* **ron-audit** (rehydration report).
* **metrics** (RTO/RPO gauges).
  **Artifacts:** `RehydrateReport{objects_restored}`.
  **Sentinel:** if registry is used as backup store → drift.

---

### S26. Ads Campaign Refund/Slashing Edge Cases

**Goal:** Refunds for mis-serve or policy breach, slashing for fraud.
**Crates:**

* **svc-ads** (campaign state; proofs required).
* **accounting** (impression proofs vs policy).
* **ron-ledger** (partial refund; idempotent adjustments).
* **ron-policy** (refund/penalty tables).
* **ron-audit** (refund evidence).
  **Artifacts:** `AdjustmentReceipt{campaign_id, delta}`.
  **Sentinel:** if billing invents money flows → drift (ledger is sole mover).

---

### S27. CDN Proof-of-Delivery (co-signed optional)

**Goal:** Strengthen payouts with viewer co-sign (privacy-safe).
**Crates:**

* **accounting** (co-signed snapshots).
* **ron-app-sdk** (viewer signature; blinding optional).
* **ron-ledger** (accept snapshots; pay out).
* **ron-policy** (when co-sig is required).
  **Artifacts:** `UsageSnapshot{provider_sig, viewer_sig?}`.
  **Sentinel:** if gateway rejects unsigned when policy says optional → drift.

---

### S28. Multi-Tenant Abuse Guard (micronode marketplaces)

**Goal:** One noisy tenant shouldn’t tank the node.
**Crates:**

* **micronode** (per-tenant cgroups/limits).
* **ron-policy** (tenant quotas).
* **metrics** (per-tenant dashboards).
* **ron-kernel** (tenant-scoped drain/restart).
  **Artifacts:** `TenantPolicy{cpu, io, net}`.
  **Sentinel:** SDKs mustn’t bypass tenant limits.

---

### S29. Wallet Holding → Off-Ramp Sanity

**Goal:** Holding period before ROX, with exemptions.
**Crates:**

* **ron-ledger** (pending vs available).
* **svc-wallet** (statements; unlock schedule).
* **interop** (bridge to ROX).
* **ron-policy** (hold windows; trusted exemptions).
  **Artifacts:** `CreditReceipt{pending}`, `UnlockReceipt`, `OffRampTxn`.
  **Sentinel:** direct off-ramp from pending = policy violation.

---

### S30. Appeals Pipeline (content-agnostic moderation)

**Goal:** Transparent appeals; mod score adjusts.
**Crates:**

* **svc-mod** (appeal queue, quorum).
* **ron-policy** (appeal SLAs; quorum sizes).
* **ron-ledger** (adjust mod pool payouts if decisions overturned).
* **ron-audit** (full chain).
  **Artifacts:** `AppealDecision{upheld|overturned}`; `ScoreUpdate`.
  **Sentinel:** any viewpoint test in policy = drift.

---

### S31. Reserved Domain Claim (brand handover)

**Goal:** Claim a courtesy-reserved name with DNS/web proof.
**Crates:**

* **svc-registry** (claim flow; state machine).
* **interop** (DNS/web challenge).
* **ron-kms** (sign final cert).
* **ron-audit** (evidence chain).
  **Artifacts:** `VerificationReport`, `RegistrationCert`.
  **Sentinel:** registry issuing without proof = drift.

---

### S32. Price-Cap Route Selection (client maxprice hint)

**Goal:** Client enforces affordability.
**Crates:**

* **ron-app-sdk** (parses `maxprice`; filters providers).
* **svc-index** (exposes price hints).
* **svc-overlay** (route selection under cap).
* **ron-policy** (price cap semantics).
  **Artifacts:** `RouteDecision{provider, price}`.
  **Sentinel:** storage setting prices → redundancy (policy/index own it).

---

### S33. Batch Micro-Txn Windowing (dust control)

**Goal:** Reduce churn; respect min payout.
**Crates:**

* **accounting** (window aggregation).
* **ron-ledger** (batch debit + multi-split).
* **ron-policy** (window length; min amounts).
* **metrics** (batch size histograms).
  **Artifacts:** `BatchReceipt{n_items, total_amount}`.
  **Sentinel:** per-request credits when batch is enabled = drift.

---

### S34. Content Keyed Access (private manifests)

**Goal:** Serve private content with client-side decryption.
**Crates:**

* **ron-app-sdk** (AEAD decrypt private blob).
* **svc-gateway** (capability check; no data inspection).
* **svc-storage** (dumb bytes).
* **ron-policy** (access caveats).
  **Artifacts:** client-side success; no server knowledge of plaintext.
  **Sentinel:** services asking for plaintext → drift.

---

### S35. Threat Drill: Cache Poison Attempt

**Goal:** Validate hash-verified immunity.
**Crates:**

* **ron-app-sdk** (reject mismatched chunk hashes).
* **svc-storage** (only commit after full digest).
* **metrics** (poison\_attempts\_total).
* **ron-audit** (incident record).
  **Artifacts:** rejected stream; audit evidence.
  **Sentinel:** accepting partials without verify = drift.

---

## How to use these now

* Add them as **S17–S35** in your scenarios README (purely written).
* For each, include the short “unique reason” line per crate so redundancy is obvious.
* Keep the “Redundancy/Drift Notes” box at the bottom of each scenario page.
* Roll these into your **Scenario × Crate** coverage matrix; you’ll light up the missing P0s (svc-wallet, svc-registry, svc-ads, svc-mod, ledger multi-split, pending balances) and catch the gateway/omnigate naming drift.

If you want, I can stitch **all 35** into a single, clean README layout (with anchors + a consolidated coverage matrix) for you to paste in when you’re ready.


#NEXT PROMPT

Absolutely—here’s the **call sheet** after a full, ruthless, golden-rule pass over all 30 scenarios and the current README positioning. I’m being decisive: what to **add**, what to **keep (canonical)**, what to **merge**, and what to **drop**—with the *unique reason to exist* for each decision so redundancy can’t hide.

---

# Executive decision (God-tier, no-regrets)

## ✅ Add (P0—must exist to satisfy scenarios)

1. **svc-passport**
   **Unique reason:** Issue, package, and re-issue encrypted identity bundles (`.passport` full-scope, `.alt` minimal) with policy-scoped capabilities and rotation lineage.
   **Scenarios unlocked:** S17, S18, S19, S26 (identity), plus anything that loads bundles in the browser/app.

2. **svc-wallet**
   **Unique reason:** Custody API + statements for ROC balances (`pending` vs `available`), unlock schedules, payouts, and off-ramp requests. Ledger is the money-mover; wallet is the user surface.
   **Scenarios unlocked:** S17, S20, S21, S24, S25, S29.

3. **svc-registry**
   **Unique reason:** Lifecycle for names and content certificates: availability/quote → register (ROC burn) → renew/grace → transfer/claim (reserved list) → revoke/freeze.
   **Scenarios unlocked:** S19, S21, S31; also used by content-cert flows in S23.

4. **svc-ads**
   **Unique reason:** Ad campaign state machine (submit → burn escrow → pending → approved → serving → adjust/refund/slash), separate from moderation and from money-movement.
   **Scenarios unlocked:** S21 (ads), S26 (refund/slash).

5. **svc-mod**
   **Unique reason:** Moderation workflow: onboarding/vetting (with stake), report queues, quorum decisions, score/decay, appeals. Not money, not policy, not ads.
   **Scenarios unlocked:** S22, S20, S30.

6. **(Ledger feature) Multi-split + Pending balances** *(inside `ron-ledger`, not a new crate)*
   **Unique reason:** Atomic N-way credit splits and `pending → available` hold periods with unlock jobs; the ledger is the **only** money mover.
   **Scenarios unlocked:** S20, S21, S24, S25, S29, S33.

7. **svc-rewarder (optional but recommended)**
   **Unique reason:** ZK/commitment helpers for verifiable usage accounting (generate/verify commitments and feed ledger). Lives beside ledger, not inside it.
   **Scenarios unlocked:** S30 (ZK commitments), strengthens S24/S27.

> These six (plus the ledger feature) are the only true “new” pieces the scenarios force us to add. Everything else below is consolidation.

---

## 🧱 Keep (canonical; passes the unique-reason test)

* **ron-kernel** (supervision/drain/readiness orchestration)
* **ron-bus** (lifecycle event fan-out)
* **metrics** (operational counters/histograms; not evidence)
* **ron-policy** (schema + versioned rules; no inlined business logic in services)
* **transport** (TCP/TLS data plane)
* **arti-transport** (onion/private path driver)
* **svc-gateway** (neutral ingress/paywall/quotas; does **not** shape app DTOs)
* **omnigate** (app BFF: DTO shaping/rate-limit/cap issuance endpoints; not money, not routing)
* **svc-index** (address → provider mapping; **mapping only**)
* **svc-overlay** (routing & provider/path selection; **no bytes persisted**)
* **svc-storage** (CAS: bytes/chunks with BLAKE3 verify; **no routing or mapping**)
* **naming** (semantics/canonicalization/aliases; **no lifecycle issuance**)
* **oap** (on-wire app protocol/framing)
* **ron-kms** (key custody, signing, rotation; **no workflow**)
* **ron-audit** (immutable evidence chains; **not metrics**)
* **ron-app-sdk** (client DX; never spins servers)
* **micronode** (single-tenant node; dev/edge)
* **accounting** (usage counters/snapshots/attribution; **not invoices and not money movement**)
* **interop** (adapters/bridges to external systems: ROX, DNS/web challenges, etc.)
* **svc-mailbox** (private-plane async message queues: durable E2E ciphertexts; distinct from CAS)
* **svc-sandbox** (isolation/containment for risky workloads; used in chaos/overload scenarios)
* **ron-auth** (capability envelopes/macaroons/parsing/validation)
* **ron-proto** (shared DTOs/errors across services)

> Each of these is exercised across S1–S35 with a crisp, non-overlapping role.

---

## 🔄 Merge / Consolidate (choose a single authority)

1. **Money semantics:**
   **Keep:** `ron-ledger` as *the* money mover (debits, credits, holds, multi-splits).
   **Keep (as reporting):** `ron-billing` for invoice artifacts & statements only.
   **Merge/Absorb:**

   * Fold any monetary parts of `ron-token` into `ron-ledger` types (denoms/precision). If `ron-token` is only data types, keep as a **pure types crate**; otherwise deprecate.
   * **Drop `svc-economy`** if it duplicates ledger/policy/accounting orchestration; any “economy” orchestration belongs as workflows *in* the specific services (ads, registry) driven by policy + ledger.

2. **Actors:**
   **Choose one:** `ryker` **or** `kameo`.
   **Decision:** Keep **ryker** (newer trajectory, better fit with supervisor model), **deprecate kameo**. Kernel remains the only place for lifecycle; actor runtime is an implementation detail behind services.

3. **Edge naming:**
   **Keep:** `svc-gateway`.
   **Deprecate public use of:** `gateway` (legacy lib). If it exists as an internal helper lib inside `svc-gateway`, mark it **internal-only**.

4. **Index naming:**
   **Keep:** `svc-index`.
   **Deprecate public use of:** `index` (legacy lib). Keep as **internal-only** helper behind svc-index if needed.

5. **Identity service name:**
   **Replace:** `svc-identity` → **`svc-passport`** (clear, bundle-focused). `ron-kms` keeps custody/rotation; passport does issuance/bundling/policy scoping.

---

## 🗑️ Drop / Deprecate (fails the unique-reason test)

* **svc-economy** — Everything it wants to do is covered cleanly by **ron-ledger + ron-policy + accounting + service-specific workflows** (ads/registry/mod).
* **kameo** — Actor runtime duplication; keep **ryker** as the single actor choice.
* **gateway** (public lib) — Redundant with **svc-gateway**.
* **index** (public lib) — Redundant with **svc-index**.
* **svc-identity** — Superseded by **svc-passport** (+ `ron-kms`).
* **Redundant money libs** — Any crate that writes/adjusts balances outside **ron-ledger** must be removed or refactored to call ledger APIs.

> If any of these still lurk in code: keep them compiling only as internal helpers temporarily, mark **deprecated**, and remove external dependencies immediately.

---

## 🧭 Rename / Normalize (clarity & grep-ability)

* **arti-transport** ← unify spelling (avoid `arti_transport`).
* **omnigate** ← single canonical name (avoid `svc-omnigate` in docs/diagrams).
* URI grammar: choose one (`crab://` or `ron://b3:<hex>.<tld>`) and standardize param keys (e.g., `?maxprice=`, `?proof=`).

---

## 📌 Final canonical crate set (post-consolidation)

**Core/runtime:** `ron-kernel`, `ron-bus`, `metrics`, `ron-policy`
**Net:** `transport`, `arti-transport`
**Ingress/App:** `svc-gateway`, `omnigate`, `ron-app-sdk`, `micronode`
**Data plane:** `svc-index`, `svc-overlay`, `svc-storage`, `naming`, `oap`
**Security & evidence:** `ron-kms`, `ron-auth`, `ron-audit`, `ron-proto`
**Economics:** `ron-ledger` *(with multi-split + holding)*, `ron-billing` *(reports/invoices only)*, `accounting`
**New governance/economy services:** `svc-passport`, `svc-wallet`, `svc-registry`, `svc-ads`, `svc-mod`, `svc-rewarder` *(optional)*
**Other plane:** `svc-mailbox`, `svc-sandbox`, `interop`
**Actor runtime (one):** `ryker` *(keep)*

> Everything else is either deprecated or internal-only helper code.

---

## Why this set is minimal and sufficient (scenario pressure test)

* **Identity & custody loops (S17–S19, S26, S29):** Passport (issuance), Wallet (balances/holds), Ledger (truth), KMS (keys), Policy (rules).
* **Naming/registry governance (S19, S21, S31):** Registry (lifecycle), Naming (semantics), Ledger (burns), Interop (proofs), KMS/Audit (tamper evidence).
* **Serving & payments (S4–S6, S20, S24, S33):** Gateway/Omnigate (edge/BFF), Index/Overlay/Storage (map/route/bytes), Accounting (usage), Ledger (multi-split), Policy (pricing).
* **Ads & moderation (S21–S22, S26, S30):** Ads (campaign lifecycle), Mod (queues/scores), Ledger (escrow/debits/pools), Policy (quotas/criteria).
* **Fraud & safety (S20, S25, S29, S35):** Ledger (holds), Mod (disputes), Audit (evidence), Storage (verify), SDK (reject mismatches).
* **ZK/verification (S27, S30):** Rewarder (commitments), Interop (external verifiers), Ledger (commit anchoring).
* **Ops/fed/scaling (S1–S3, S14–S16, S23–S24):** Kernel/Bus/Metrics + Overlay/Sandbox cover resilience and backpressure.

There is **no crate here that needs a contrived justification** across the 30 scenarios. Conversely, everything dropped failed the “unique reason” test or was a duplicate interface for an existing responsibility.

---

## Migration plan (sane, reversible)

1. **Freeze money flows:** Route *all* debits/credits through `ron-ledger`. Add `pending/available`, unlock jobs, multi-split API.
2. **Stand up P0 services (scaffold first, then flesh):** `svc-wallet`, `svc-passport`, `svc-registry`, `svc-ads`, `svc-mod`.
3. **Soft-deprecate** `gateway`, `index`, `svc-identity`, `svc-economy`, `kameo`. Mark as internal-only or remove public deps.
4. **Doc alignment:** Update diagrams/names (gateway vs omnigate; identity→passport; canonical URI grammar).
5. **Scenario ledger:** Keep the S1–S30 matrix live; any crate not hit with a unique reason in ≥2 orthogonal scenarios is re-reviewed for deprecation in the next pass.

---

## Risk register (and the guardrails we already have)

* **Scope creep in wallet/registry/ads/mod:** Controlled by policy schemas + audit demands; money lives in ledger only.
* **Actor runtime churn:** Standardize on **ryker**; forbid lifecycle outside kernel.
* **Economics fragmentation:** Billing ≠ ledger; accounting ≠ ledger. Enforce borders in code review.
* **Private-plane ambiguity:** Keep `svc-mailbox` strictly as async E2E queues; no content inspection; storage stays content-addressed and immutable.

---

If you want, I can turn this into a one-page **“Authority Map”** you can paste at the top of *Scenario\_Blueprint.md* so every reviewer has the same ground truth (who owns what, and what no longer exists).
