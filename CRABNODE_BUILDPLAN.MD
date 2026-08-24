# CRABNODE_BUILDPLAN.md — Finish the Public CrabNode Service-Node Product

**Version:** 0.1.0  
**Date:** 2026-08-18  
**Status:** Proposed implementation authority after approval  
**Primary repo:** RustyOnions  
**Integration repos:** CrabLink, ROX Anchor (display/handoff only; not a CrabNode runtime dependency)  
**Umbrella milestone:** `FINAL_BETA` Phase 21 and subsequent desktop/private-beta acceptance  

---

## 0. Purpose

This plan turns the substantial RustyOnions Service Node implementation that already exists into the public product users should experience as **CrabNode**.

This is **not** a plan to invent a second node architecture or replace the existing 33-crate RustyOnions design.

The implementation doctrine is:

```text
CrabNode = public Service Node product
crabnode = public operator executable / CLI
macronode = internal Service Node host/composition implementation
canonical svc-* / ron-* crates = owners of real behavior
svc-admin = optional operator UI
CrabLink = optional friendly operator controller and client
```

The key outcome is simple:

```text
A normal operator can install CrabNode,
run it headlessly,
join the network,
serve verified b3 content,
discover other providers,
replicate/repair content under policy,
participate in service evidence/reward flows,
and stop/restart/recover it without developer scripts.
```

No operator should need to know that `macronode`, `svc-dht`, `svc-storage`, `omnigate`, or other internal crates exist.

---

# 1. Source Authority for This Plan

When evidence conflicts, use this order while implementing this plan:

```text
1. Fresh repository source in the 2026-08-18 codebundles.
2. Fresh terminal/runtime evidence from the 2026-08-18 carry-over notes.
3. Current focused tests adjacent to the behavior.
4. FINAL_BETA current buildplan.
5. BUILD_PLAN_Z current file.
6. Current crate-specific IDBs/buildplans.
7. 12_PILLARS / SIX_CONCERNS / INTEROP / APP_INTEGRATION / HARDENING / SCALING.
8. Older ALLNOTES / historical combined docs.
```

The source review used:

```text
RustyOnions:
  CODEBUNDLE_RS(20260818-053100).md
  Git 51108bc8
  2417 Rust files

CrabLink:
  CODEBUNDLE_TAURI_APP(20260818-053114).md
  Git 2079231
  clean
  1158 text files

ROX Anchor:
  CODEBUNDLE(20260818-053049).md
  Git e82bcf1
  316 files

Current project plans/notes:
  BUILD_PLAN_Z(20260818-053150).MD
  FINAL_BETA(20260818-053125).md
  ALLNOTES(20260818-053151).MD
  current NOTES files
  PASSPORT_NATIVE_BUILDPLAN(20260818-053150).MD

Architecture references:
  12_PILLARS.MD
  SIX_CONCERNS.MD
  HARDENING_BLUEPRINT.MD
  CONCURRENCY_AND_ALIASING_BLUEPRINT.MD
  FULL_PROJECT_BLUEPRINT.MD
  INTEROP_BLUEPRINT.MD
  OMNIGATE_BLUEPRINT.MD
  SCALING_BLUEPRINT.MD
  APP_INTEGRATION_BLUEPRINT.MD
  COMPLETECRATELIST.MD
  COMBINED_IDB.MD
  ALL_DOCS_COMBINED.MD
  INTERNAL_ROC_MODEL.md
  INTERNAL_ROC_BETA_VALUE_LOOP_BLUEPRINT.md
  INTERNAL_ROC_BETA_BUILDPLAN.md
```

This buildplan deliberately records older-document drift but does **not** amend those documents yet. Documentation cleanup is a later explicit phase after this plan is accepted.

---

# 2. Executive Review — What Already Exists

The project is much closer to a real CrabNode than older phase counters imply.

The correct approach is **productization and integration**, not a rewrite.

## 2.1 High-level current state

| Surface | Current reality | Plan posture |
|---|---|---|
| `crabnode` CLI | Real, tested operator/admin client; lacks full daemon lifecycle | Extend existing binary; no new CLI crate |
| `macronode` | Real headless host/admin/status shell with substantial service composition | Keep thin; use as internal CrabNode runtime |
| Embedded gateway | Real listener but only MVP `/ingress/ping` behavior | Replace/compose with canonical full gateway behavior |
| `svc-gateway` standalone | Large, real product ingress/proxy route surface | Reuse; do not duplicate |
| `omnigate` | Large, real BFF/product route surface | Reuse; do not duplicate |
| Embedded storage | Real `svc-storage` router backed by `MemoryStorage` | Keep route behavior; add real durable backend later |
| Embedded index | Real `svc-index` server | Keep canonical crate ownership |
| Embedded DHT | Real current `svc-dht` provider router/store/pruner/bootstrap | Continue; finish network replication/transport behavior |
| Macronode overlay wrapper | Still host shell | Wire canonical `svc-overlay` implementation |
| Standalone overlay | Real TCP/OAP handshake, bounded writer, metrics, simple gossip | Reuse and mature |
| Macronode mailbox wrapper | Still host shell | Optional for first content beta; wire when product path needs it |
| `svc-passport` profile claim | Real validation/HTTP behavior, but process-local memory store | Make durable locally first, then distributed namespace |
| Supervisor | Has task handles, crash-policy primitives and counters, but no actual restart loop | Finish real supervision |
| Service evidence/quorum | Considerable later-phase code and green Phase21B proof already exists | Reuse/regression-test, do not rebuild |
| Micronode/User Node | Private verifier truth posture already proven | Keep separate from Service Node |
| CrabLink operator mode | Real safe read/control boundaries; explicitly does not start daemon | Preserve optional-controller model |
| ROX Anchor | Separate private testnet ROC↔ROX surface | Keep outside CrabNode core runtime |

---

# 3. Detailed RustyOnions Review

## 3.1 Existing `crabnode` operator CLI

Current source is:

```text
crates/macronode/src/bin/crabnode.rs
```

It is already correctly designed as a loopback-first operator surface.

Current commands include real behavior for:

```text
crabnode status
crabnode ready
crabnode version
crabnode policy status
crabnode block <b3>
crabnode unblock <b3>
crabnode allow <b3>
crabnode remove-allow <b3>
crabnode quarantine <b3>
crabnode release-quarantine <b3>
crabnode prune <b3>

crabnode persistence register ...
crabnode persistence status ...
crabnode persistence pending ...
crabnode persistence submit ...
crabnode persistence approve ...
crabnode persistence pin ...
crabnode persistence unpin ...
crabnode persistence reject ...

crabnode admin enable-web
crabnode admin disable-web
crabnode admin setup-token
crabnode admin create-user ...

crabnode rewards show
crabnode rewards bind @operator
crabnode rewards rotate @operator
```

Important current protections:

```text
loopback-first admin target
explicit opt-in required for non-loopback admin
admin token comes from environment rather than CLI output
no fake success
existing dry-run support
bounded token size
```

What it does **not** yet provide is the product lifecycle an ordinary node operator expects:

```text
init
run/start
stop
restart
logs
comprehensive doctor
identity/bootstrap inspection
peer/provider inspection
```

Therefore the correct move is to expand this existing executable rather than creating another crate or launcher.

---

## 3.2 Current macronode service composition

`crates/macronode/src/services/spawn.rs` currently schedules:

```text
svc-gateway
svc-index
svc-overlay
svc-storage
svc-mailbox
svc-dht
```

It now has materially better truth than older notes:

```text
gateway: real listener
storage: real embedded svc-storage router
index: real embedded svc-index router
DHT: real embedded canonical svc-dht provider router
```

But `spawn_all` still explicitly says:

```text
no restarts
fire-and-forget until process shutdown
```

`deps_ok` currently means workers were scheduled, not that every optional service is healthy.

Required readiness currently gates primarily on:

```text
admin listeners/config/metrics
gateway bound
storage bound
index bound
```

Overlay/mailbox/DHT are still informational in the central `ReadyProbes` model. That is acceptable only while they are not required for the claimed runtime capability. Once CrabNode claims network discovery/swarm serving, DHT and overlay must become capability-specific readiness gates.

---

## 3.3 Embedded gateway is not the complete product gateway

The current macronode wrapper:

```text
crates/macronode/src/services/svc_gateway.rs
```

binds the expected local gateway port and exposes:

```text
GET /ingress/ping
```

It is intentionally labeled an MVP.

This is a major distinction because the **canonical standalone `svc-gateway` crate already has the real product route/middleware/proxy surface** and builds its router from `svc_gateway::routes::build_router`.

Likewise, `omnigate::App::build` creates the full `/v1` product BFF route tree.

Therefore:

```text
DO NOT expand macronode's one-route MVP gateway by copying routes into macronode.
DO NOT create parallel identity/site/wallet/storage rules inside macronode.
REUSE the canonical svc-gateway and omnigate implementations.
```

The finishing work is composition and lifecycle, not route duplication.

---

## 3.4 Storage is real locally, but not durable yet

Macronode's embedded storage currently:

```text
uses canonical svc-storage router
uses B3/CAS behavior
supports moderation admission
can seed and independently verify a deterministic b3 object
registers the exact live store for prune behavior
fails closed on bad moderation configuration
sets readiness only after listener/seed verification
```

However, the actual backend is still:

```text
MemoryStorage
```

and the source explicitly calls it:

```text
in-memory / amnesia-first
```

BUILD_PLAN_Z correctly records that persistence-review states such as:

```text
verified_persistent
pinned_by_operator
```

do **not** prove bytes were durably written.

`durableBytesWritten` must remain false until a real durable backend exists.

This is one of the most important truth boundaries to preserve.

---

## 3.5 Index is genuinely embedded

Macronode uses canonical `svc-index` AppState/router and exposes the real server.

It also registers the exact provider-response cache used by the live router so prune can invalidate truthful state rather than creating a parallel fake cache.

This is a good pattern for the rest of CrabNode:

```text
macronode owns composition/lifecycle
canonical service crate owns semantics
cross-service coordinator references the exact live state
```

Do not move index logic into macronode.

---

## 3.6 DHT has advanced beyond the old host shell

The current macronode DHT wrapper now embeds real `svc-dht` behavior.

Current useful behavior includes:

```text
typed B3 CID validation
typed crab://node/<id> provider identities
local ProviderStore
provider TTL expiry
provider pruning
bounded provider candidate selection
local provider status hints
explicit provider withdrawal
embedded HTTP/provider router
bootstrap supervisor integration
exact local provider store registration for pruning
```

Provider selection already correctly separates externally advertised availability from **local observations** such as:

```text
responsive
unknown
degraded
unavailable
```

and unavailable providers are excluded from fetch candidates.

This is strong foundation work.

But the critical missing network behavior is explicit in source:

```text
provider/republish.rs:
  Placeholder for republish/refresh logic
  Left for Phase 2 (networked replication)
```

Other DHT transport/supervision areas also retain placeholders.

So today's DHT is best described as:

```text
real typed provider model + local provider API + bootstrap foundation
```

not yet:

```text
complete global provider propagation / Kademlia swarm
```

That distinction drives the next phases.

---

## 3.7 Overlay: standalone implementation exists; macronode wrapper is stale

The standalone `svc-overlay` crate already has useful real behavior:

```text
transport listener
OAP-style handshake
connection/session metrics
bounded writer queue
backpressure/drop accounting
frame parsing
simple data-frame gossip publish
clean read/close/error handling
readiness sampling
```

But macronode's wrapper still says:

```text
host shell, waiting for real svc-overlay wiring
```

and only flips an informational readiness bit while sleeping.

This is exactly the kind of integration gap this buildplan should close:

```text
wire the canonical service, do not rewrite it in the wrapper
```

---

## 3.8 Mailbox remains incomplete for the public CrabNode composition

Macronode's mailbox wrapper remains a host shell.

The architecture documents want eventual mailbox semantics such as:

```text
at-least-once delivery
ACK
visibility/retry
DLQ
idempotency
```

But those claims must not be attributed to the current CrabNode runtime until the real service is wired and tested.

For the first CrabNode content/identity beta, mailbox should be treated as:

```text
optional unless a specific live product route requires it
```

This prevents an incomplete non-critical service from blocking the highest-value content/network milestones.

---

## 3.9 Supervisor is the largest product-lifecycle gap

The repository has:

```text
supervisor/backoff.rs
supervisor/crash_policy.rs
supervisor/health_reporter.rs
supervisor/lifecycle.rs
supervisor/shutdown.rs
```

and restart counters exist in readiness/status.

But `ManagedTask::watch` is still explicitly a placeholder that simply awaits the task and swallows the join result.

Current behavior therefore does **not** yet provide:

```text
real crash classification
actual restart
jittered restart loop
restart budget enforcement
crash-loop terminal state
required-service readiness downgrade on crash
full intentional-shutdown distinction
```

A public node cannot be considered finished until this is real.

---

## 3.10 Passport/profile network state is real behavior but only process-local truth

`svc-passport` currently has real profile routes:

```text
POST /v1/passport/profile/claim
GET  /v1/passport/profile/by-subject/:passport_subject
GET  /v1/passport/profile/:username
```

The code is explicit that the current claim route uses:

```text
current in-memory store
not final durable production persistence
```

and router construction creates a fresh:

```text
UsernameClaimStore::new()
```

The store enforces useful deterministic behavior:

```text
canonical username validation
one username per passport subject
no duplicate username between subjects
idempotent repeat for same subject
confirmed status inside this process
safe public profile projection
```

But:

```text
restart loses claims
another independent Service Node has its own claim map
there is no network-global conflict resolution yet
public_profile_cid remains None until publication wiring exists
```

This is the nearest concrete blocker to the two-client Phase21 identity demonstration and a critical distinction for later true decentralization.

---

## 3.11 Quorum, evidence, and economics are farther ahead than the old phase counter

The fresh Rust tree already contains substantial Service Node behavior around:

```text
delivery evidence
policy evidence
range-repair evidence
challenge evidence
evidence outbox
quorum bootstrap
quorum participation
checkpoint committee validation
validator signing
checkpoint finality
economic status
service-node lifecycle status
```

Current Phase21B runtime proof has already demonstrated a real multi-process private network with:

```text
3 Service Nodes
1 User Node
cryptographic quorum
one-of-three rejection
two-of-three acceptance
member failure isolation
continued quorum after one loss
member restart
stable restarted identity
```

That proof is extremely valuable.

But the test starts child processes, proves the model, and terminates them. It does not itself deliver the persistent operator product or a content-provider swarm.

Therefore later economic/quorum phases in this plan are **integration/closure phases**, not a request to rebuild all of that work.

---

# 4. CrabLink Review Relevant to CrabNode

## 4.1 CrabLink is already correctly separated from daemon authority

The Tauri Service Node operator boundary explicitly says it can connect to a real independently running Service Node but:

```text
cannot start the daemon
cannot manage daemon lifecycle
is read-only for canonical status
is disabled by default
remote connection requires HTTPS + explicit ephemeral bearer credential
credentials are not persisted/returned/logged
```

This is a good boundary and should remain.

CrabLink may later expose friendly Start/Stop controls only by deliberately calling a local CrabNode lifecycle API or OS integration. It must never become the only way to operate the node.

---

## 4.2 Username availability is correctly gateway-backed and fail-closed

The current onboarding adapter follows:

```text
CrabLink
  -> configured gateway
  -> identity client
  -> GET public profile by username
```

and interprets only:

```text
HTTP 404 + reason=profile_not_found
```

as an **availability hint**.

An existing profile is unavailable. Every other failure remains an error.

It also correctly states that availability is not ownership.

Therefore the CrabNode work must fix the real backend chain; it must **not** add a frontend availability bypass for normal release behavior.

---

## 4.3 CrabLink already has extensive Service Node safety boundaries

The current code has native command bridges/checkers for:

```text
Service Node operator status
moderation review
persistence review
signed reward binding
OAP object verification
User Node verification handoff
lifecycle isolation
```

This means CrabNode should expose truthful, typed operator/network behavior and let CrabLink consume it.

Do not put node policy, economic truth, or raw capabilities in React.

---

# 5. ROX Anchor Review Relevant to CrabNode

ROX Anchor is a separate trust/economic integration surface.

Its fresh codebundle explicitly disclaims authorization for:

```text
mainnet
production bridge settlement
public ROX mint/burn
staking
liquidity
exchange-facing behavior
external settlement
```

and its CrabLink-facing work is display/dry-run/private-testnet oriented.

Therefore:

```text
CrabNode must not depend on Solana or ROX Anchor to start,
serve content,
join the provider network,
resolve identity,
or participate in basic RustyOnions service evidence.
```

ROX Anchor remains downstream of accepted internal ROC/economic state and is only relevant to the separate FINAL_BETA private-testnet demonstration.

This separation is mandatory for portability and node operator simplicity.

---

# 6. Blueprint Review — Architecture That Should Be Preserved

The older blueprints contain some stale implementation assumptions, but several core boundaries remain excellent and align with current source.

## 6.1 33-crate canon and thin node host

The project canon says:

```text
macronode = node profile/composition host
keep it thin
compose services via config
never embed duplicate service logic into it
```

This plan preserves that.

## 6.2 Overlay/DHT separation

Canonical rule:

```text
svc-overlay = sessions/gossip/peer connection behavior
svc-dht     = discovery/provider records/routing
```

No DHT routing logic should move into overlay.

## 6.3 Content addressing

Canonical content identity is:

```text
BLAKE3-256
b3:<hex>
```

Content bytes must be verified against the full digest before becoming trusted/cached/served as that object.

## 6.4 Ingress boundary

Application integration is correctly modeled as:

```text
CrabLink / SDK
    -> svc-gateway
    -> omnigate
    -> canonical downstream service owners
```

The gateway owns edge admission/quotas; Omnigate owns BFF/orchestration; storage/index/passport/wallet own their own truth.

## 6.5 Security/resilience boundary

Relevant non-negotiables remain:

```text
no locks held across await on hot paths
bounded queues
backpressure
body caps
safe decompression
explicit capabilities
crash-only supervision with backoff/jitter
truthful readiness
golden metrics
amnesia mode
no secret logging
no ambient authority
```

## 6.6 Economics boundary

The project consistently separates:

```text
ron-accounting = metering/observation
svc-rewarder   = reward planning
ron-policy     = reviewed policy/gating
svc-wallet     = mutation ingress
ron-ledger     = durable economic truth
```

CrabNode service behavior may produce evidence, but no node may self-mint or invent a confirmed balance/receipt.

---

# 7. The Correct Network Model: Torrent-Like on One Plane, Not Everywhere

The useful torrent analogy applies to **immutable content distribution**.

It should not be used as the model for usernames, mutable pointers, policy state, or economic state.

The finished architecture should have four distinct planes.

## 7.1 Plane A — Immutable content/swarm plane

This is the BitTorrent-like part:

```text
b3 object exists
    -> one or more CrabNodes possess verified bytes
    -> provider records advertise availability
    -> requesting CrabNode discovers candidates
    -> fetches from one or more candidates
    -> verifies BLAKE3
    -> may cache/replicate under policy
    -> serves to clients/peers
```

Properties:

```text
immutable content identity
multiple providers
no canonical origin server required
alternate-provider retry
integrity verification independent of provider trust
replication and repair
bounded caching
operator policy controls persistence/serving
```

For beta, object-level and HTTP/OAP range transfer is sufficient. A literal BitTorrent piece protocol is **not** required.

Parallel chunk/range fetching can be added later for large media if measurement proves it valuable.

## 7.2 Plane B — Mutable public network state / namespace

Examples:

```text
@username -> Passport/profile ownership
profile -> latest public manifest pointer
site name -> current manifest pointer
signed tombstone/current-owner transitions
selected globally shared policy roots
node descriptor -> currently advertised service endpoints/capabilities
```

These are not immutable files and cannot be safely resolved by "whoever the DHT returned first."

Required properties:

```text
signed transition records
domain separation
monotonic sequence/epoch/nonces
idempotency
conflict rules
deterministic winner/finality semantics
replay protection
cross-node convergence
```

The DHT may distribute/cache accepted records, but it must not become the authority that decides username uniqueness.

Likewise, `ron-ledger` should not be overloaded into generic namespace authority just because it already has consensus machinery. Economic truth remains a separate domain.

## 7.3 Plane C — Economic/evidence plane

```text
real service
 -> signed evidence
 -> accounting/reward projection
 -> policy
 -> quorum/checkpoint acceptance
 -> svc-wallet
 -> ron-ledger
 -> confirmed receipt/balance
```

Evidence examples:

```text
delivery
availability
repair
policy refusal
verification/audit
```

No raw page view or self-asserted bandwidth is enough to mint ROC.

## 7.4 Plane D — Operator/control plane

This is `crabnode`:

```text
init
run/start/stop/restart
status/ready/doctor
logs
config
identity
peers/providers
moderation
persistence
reward binding
optional admin UI control
```

Admin stays local/private by default.

---

# 8. Target CrabNode Runtime Shape

## 8.1 Public experience

The desired final operator experience is approximately:

```bash
crabnode init
crabnode start
crabnode status
crabnode stop
```

Advanced operators can use:

```bash
crabnode run
crabnode doctor
crabnode logs --follow
crabnode config validate
crabnode identity show
crabnode peers
crabnode providers b3:<hash>
crabnode policy status
crabnode persistence pending
crabnode rewards show
```

## 8.2 Internal composition rule

A public CrabNode may internally use:

```text
one process with embedded canonical services
or
one supervisor with child service processes
```

but the operator contract must be the same.

Implementation rule:

```text
Prefer in-process composition where the canonical crate already exposes
stable router/runtime builders and the integration remains thin.

Use supervised child processes where embedding would duplicate bootstrap,
create large coupling, or delay the beta.

Never copy service semantics into macronode just to avoid a child process.
```

For the current codebase, the likely fastest path is:

```text
keep embedded index/storage/DHT
replace the MVP gateway wrapper with canonical full svc-gateway behavior
compose a real Omnigate runtime
wire canonical svc-overlay
add svc-passport to the required private-beta profile
leave mailbox optional until a live route requires it
keep economic services profile-controlled
```

This should be confirmed in CN-0/CN-2 against current Cargo/config surfaces before implementation.

---

# 9. Required vs Optional Services by Capability

CrabNode readiness must be capability-aware rather than pretending every future service is required on day one.

## 9.1 Base operator capability

Required:

```text
macronode control/admin plane
configuration
node identity
logging/metrics/readiness
```

## 9.2 CrabLink product-backend capability

Required:

```text
full svc-gateway
omnigate
svc-index
svc-storage
svc-passport
```

Conditional:

```text
svc-wallet / ron-ledger stack for paid/economic product flows
```

## 9.3 Decentralized provider capability

Required in addition:

```text
svc-overlay
svc-dht
provider descriptor/address resolution
```

## 9.4 Messaging capability

Required only when claimed/enabled:

```text
svc-mailbox
```

## 9.5 Quorum/reward capability

Required only when the node is eligible/activated:

```text
service-node identity/signing
quorum participation
service evidence outbox
canonical economic downstream connectivity
```

`/readyz` should truthfully report both overall profile readiness and capability-specific degraded states.

---

# 10. Network/Addressing Rules

## 10.1 Separate identities from transport addresses

Provider records correctly use:

```text
crab://node/<canonical-node-id>
```

rather than raw socket/IP text.

That is good, but a node identity alone is not dialable.

A separate signed descriptor must resolve:

```text
CrabNodeId
 -> protocol version
 -> capabilities
 -> one or more service endpoints
 -> expiry
 -> signature
```

This descriptor can change address without changing node identity.

## 10.2 Listen vs advertise must be separate

Configuration must distinguish:

```text
listen address
advertised public endpoint
admin address
client ingress address
peer transport address
```

A node behind NAT may listen locally while advertising only an explicitly configured reachable endpoint.

Never infer a public endpoint from an untrusted request header.

## 10.3 Admin posture

Default:

```text
127.0.0.1 only
```

Remote admin:

```text
explicit opt-in
TLS
explicit credential/capability
bounded routes
no secrets in URLs/logs
```

## 10.4 User Node privacy remains separate

Normal CrabLink User Nodes remain:

```text
private/outbound
no public inbound requirement
no user residential IP publication
no direct user-to-user TCP by default
```

Only public Service Nodes advertise public service endpoints.

---

# 11. Hard Invariants for Every Phase

The following are non-negotiable:

```text
NO new CrabNode crate unless a concrete source-level blocker proves necessary.
NO duplicate gateway/Omnigate/storage/index/DHT rules in macronode.
NO frontend username/economic bypass to compensate for backend gaps.
NO fake readiness.
NO "task spawned" treated as "service healthy" when capability requires it.
NO persistence claim before bytes actually exist durably.
NO provider advertisement for bytes the node cannot serve.
NO acceptance of peer bytes before BLAKE3 verification.
NO DHT deciding global username ownership.
NO generic namespace state moved into ron-ledger.
NO single Service Node unilateral ROC mint/payment.
NO admin listener public by default.
NO user residential IP publication.
NO secrets in status/logs/CLI arguments where avoidable.
NO unbounded queues/caches/provider tables.
NO lock held across await on hot paths.
NO restart storm without backoff/budget.
NO ROX/Solana dependency in core CrabNode startup/content path.
```

---

# 12. Implementation Workflow

Every implementation slice follows:

```text
1. Run the smallest relevant compile/test gate.
2. Inspect the first real failure.
3. Patch only the behavior needed for that slice.
4. Add/adjust focused tests beside the behavior.
5. Rerun focused tests.
6. Run the next integration test.
7. Run workspace/broader checks only at milestone boundaries.
```

Default entry command when starting work:

```bash
cd /Users/mymac/Desktop/RustyOnions
cargo check --workspace
```

Then narrow to the first failing/active crate.

Do not use `cargo clean` unless evidence proves it necessary.

---

# 13. CN-0 — Fresh Baseline and Runtime Truth Inventory

## Goal

Freeze current source truth before changing CrabNode behavior.

## Work

```text
Run cargo check --workspace.
Fix only the first compiler failure if any.
Run focused macronode check/tests.
Run focused checks for:
  svc-gateway
  omnigate
  svc-storage
  svc-index
  svc-dht
  svc-overlay
  svc-passport
Inventory current bind/default/env config for every service.
Inventory current required Cargo dependencies and reusable router/runtime builders.
Inventory exact current standalone binaries and their graceful shutdown surfaces.
Confirm current gateway/Omnigate route chain.
Confirm current Service Node admin/status schema.
Do not amend old docs yet.
```

## Focused verification

```bash
cargo check -p macronode
cargo test -p macronode --tests
cargo check -p svc-gateway
cargo check -p omnigate
cargo check -p svc-storage
cargo check -p svc-index
cargo check -p svc-dht
cargo check -p svc-overlay
cargo check -p svc-passport
```

Add narrower test commands only after current test names are inspected.

## Exit gate

```text
CRABNODE_CN0_WORKSPACE_BASELINE=GREEN
CRABNODE_CN0_RUNTIME_MAP=FROZEN
CRABNODE_CN0_PORT_COLLISIONS=KNOWN
CRABNODE_CN0_CANONICAL_SERVICE_OWNERS=KNOWN
DOCS_AMENDED=NO
```

---

# 14. CN-1 — Public `crabnode` Lifecycle

## Goal

Make the existing `crabnode` executable the real public operator front door.

## Work

Add lifecycle commands to the existing binary:

```text
crabnode init
crabnode run
crabnode start
crabnode stop
crabnode restart
crabnode status
crabnode ready
crabnode doctor
crabnode logs
crabnode config show
crabnode config validate
```

Preserve current commands for moderation, persistence, admin, and rewards.

### `crabnode init`

Creates only local operator state:

```text
config directory
data directory
log directory
run/PID state directory
node identity if absent
safe default config
```

Must be idempotent.

### `crabnode run`

Foreground mode:

```text
best debugging mode
signals visible
no daemonization magic required
owns the full local CrabNode runtime
```

### `start/stop/restart`

Requirements:

```text
never kill unrelated process from stale PID
verify process identity before stop
idempotent stop
stale PID recovery
bounded shutdown timeout
force termination only after explicit safe timeout policy
no shell eval
cross-platform abstraction
```

### `logs`

```text
bounded read
tail/follow support later in slice
no secret-bearing environment dump
no raw admin tokens
```

## Tests

```text
fresh init
repeated init
start when stopped
start when already running
stop when running
stop when already stopped
stale PID cannot kill unrelated process
restart preserves node identity
logs do not print secrets
run works with svc-admin UI absent
```

## Exit gate

```text
CRABNODE_CN1_PUBLIC_CLI=GREEN
CRABNODE_CN1_HEADLESS_RUN=GREEN
CRABNODE_CN1_DASHBOARD_REQUIRED=NO
CRABNODE_CN1_NODE_VITE_REQUIRED=NO
CRABNODE_CN1_NEW_CLI_CRATE=NO
```

---

# 15. CN-2 — Canonical CrabNode Profile and Port Ownership

## Goal

Define one collision-free private-beta Service Node composition.

## Work

Create a single canonical CrabNode profile/config consumed by the runtime.

Required decisions from current source inspection:

```text
which service owns client ingress
which service owns Omnigate/BFF bind
which services remain embedded
which services run as supervised children
which services are optional
which binds are loopback
which binds may be public/LAN
```

Existing project convention to reconcile:

```text
macronode admin: 127.0.0.1:8080
client/full gateway convention: :8090
Omnigate convention: :9090
embedded overlay: :5301
embedded DHT: :5302
embedded storage: :5303
embedded index: :5304
embedded mailbox: :5305
```

Do **not** allow both the MVP macronode gateway and canonical full gateway to claim the same port.

Preferred outcome:

```text
one full product gateway owns the client ingress port
MVP ping-only gateway is retired/disabled/replaced in the CrabNode full profile
```

## Config rules

Configuration should eventually be representable in one operator file such as:

```text
crabnode.toml
```

with separate sections for:

```text
instance/admin
client ingress
internal services
node identity
storage
policy
persistence
overlay
discovery/bootstrap
reward/quorum
economics connectivity
```

No need to rename every historical environment variable immediately; the CrabNode config can map onto canonical service config during migration.

## `doctor`

`crabnode doctor` should detect:

```text
port collision
missing required config
invalid bind
public admin exposure
missing/invalid node identity
missing data/log permissions
unreachable required internal service
unsupported profile combination
```

## Exit gate

```text
CRABNODE_CN2_PROFILE=GREEN
CRABNODE_CN2_PORT_MAP=COLLISION_FREE
CRABNODE_CN2_PUBLIC_ADMIN_DEFAULT=NO
CRABNODE_CN2_SINGLE_CLIENT_INGRESS=YES
```

---

# 16. CN-3 — Full Product Ingress and Truthful Capability Readiness

## Goal

A standalone CrabNode must expose the real CrabLink backend, not only `/ingress/ping`.

## Work

Wire canonical:

```text
svc-gateway
omnigate
```

under CrabNode composition.

Do this by reusing canonical router/runtime builders or supervised binaries.

Do not copy routes into macronode.

Configure downstreams for:

```text
svc-passport
svc-storage
svc-index
DHT when enabled
mailbox when enabled
wallet/economic services when enabled
```

## Readiness redesign

Required readiness must represent real health.

Example capability projection:

```text
operator_core: ready
product_ingress: ready/degraded/disabled
content_store: ready/degraded
identity: ready/degraded
discovery: ready/degraded/disabled
peer_transport: ready/degraded/disabled
messaging: ready/degraded/disabled
economic_participation: ready/degraded/disabled
```

Rules:

```text
listener successfully bound != downstream healthy
worker scheduled != ready
optional disabled != failed
required crashed = not ready
fake readiness forbidden
```

## Integration test

Prove through the actual public gateway:

```text
health/readiness
public identity lookup
object read/head
index read
one safe product route
```

## Exit gate

```text
CRABNODE_CN3_FULL_GATEWAY=GREEN
CRABNODE_CN3_OMNIGATE=GREEN
CRABNODE_CN3_PING_ONLY_BACKEND=NO
CRABNODE_CN3_READINESS_TRUTHFUL=YES
```

---

# 17. CN-4 — Durable Local Passport Username/Profile State

## Goal

Unblock the current Phase21 two-client identity demonstration without pretending local persistence is global consensus.

## Work

Replace the process-local `UsernameClaimStore` backend with a durable service-owned store.

Requirements:

```text
svc-passport remains identity/profile owner for this route
canonical handle syntax/policy comes from ron-naming
no wallet/ledger authority
no frontend bypass
atomic durable writes
schema/version field
restart recovery
corruption/failure returns safe error
one subject -> one main username
one username -> one subject within this node's accepted state
idempotent same-subject repeat
```

Recommended initial storage shape:

```text
small local transactional file/DB store behind a trait
```

Do not bind the implementation to the final global consensus mechanism.

## Critical wording/truth rule

For this phase:

```text
"confirmed" means accepted by the configured private-beta namespace authority/state,
not universal internet finality.
```

The UI should continue to treat pre-claim availability as only a hint.

## Integration proof

```text
CrabLink A checks @testmac -> available hint
A claims @testmac
CrabLink B checks @testmac -> unavailable
restart CrabNode
B checks @testmac -> still unavailable
profile read after restart -> same owner/profile
```

## Exit gate

```text
CRABNODE_CN4_IDENTITY_ROUTE_CHAIN=GREEN
CRABNODE_CN4_USERNAME_DURABLE_RESTART=GREEN
CRABNODE_CN4_FRONTEND_BYPASS=NO
CRABNODE_CN4_GLOBAL_CONSENSUS_CLAIM=NO
```

### Milestone M1 — FINAL_BETA immediate unblock

At this point the persistent standalone node can support the immediate two-client username/profile path needed for the live desktop demo.

---

# 18. CN-5 — Real CrabLink Content Product Path Through One CrabNode

## Goal

Prove one CrabNode is a complete product backend before making it a swarm.

## Work

Through the real gateway/Omnigate path prove:

```text
B3 object upload/write
B3 object read/head
hash verification
range behavior where current route supports it
index pointer storage/read
post publication
image publication
article publication
profile timeline read
site manifest read/write
blog/imageboard/forum routes needed by FINAL_BETA
paid content/site path only through existing wallet/ledger authority
```

Do not change product semantics just to simplify node composition.

## Tests

Use existing focused route tests as regression coverage, then add one CrabNode-composed integration suite.

## Exit gate

```text
CRABNODE_CN5_SINGLE_NODE_PRODUCT_BACKEND=GREEN
CRABNODE_CN5_B3_VERIFY=GREEN
CRABNODE_CN5_INDEX_STORAGE_CHAIN=GREEN
CRABNODE_CN5_FAKE_ECONOMIC_SUCCESS=NO
```

---

# 19. CN-6 — Real Supervision, Restart, and Degraded-State Semantics

## Goal

A CrabNode must survive service failures without lying.

## Work

Finish real supervisor behavior using existing primitives.

Required:

```text
classify clean exit vs crash/panic/cancel
clear service readiness immediately on unexpected exit
record restart counter
apply jittered exponential backoff
apply crash-policy rolling restart budget
restart only restartable service
surface crash-loop terminal/degraded state
never restart during intentional node shutdown
gracefully drain where supported
bounded shutdown deadline
preserve node identity/config across child restart
```

Do not hold locks across await.

## Tests

```text
kill gateway -> product readiness false -> gateway restarts -> readiness true
kill storage -> content readiness false -> bounded restart
kill optional mailbox -> optional capability degraded, core remains ready
crash required service repeatedly -> restart budget trips -> truthful failed/degraded state
intentional shutdown -> no restart
one service crash does not kill unrelated healthy service unless required by policy
```

## Exit gate

```text
CRABNODE_CN6_CRASH_DETECTION=GREEN
CRABNODE_CN6_BACKOFF=GREEN
CRABNODE_CN6_RESTART_BUDGET=GREEN
CRABNODE_CN6_INTENTIONAL_SHUTDOWN_RESTART=NO
CRABNODE_CN6_FAKE_READY_AFTER_CRASH=NO
```

### Milestone M2 — Standalone CrabNode product

After CN-6, an operator can run a persistent, headless, self-monitoring CrabNode without `run_dashboard.sh` or a browser.

---

# 20. CN-7 — Durable CrabNode Identity and Signed Service Descriptor

## Goal

Give each Service Node a stable cryptographic identity independent of its IP address.

## Work

Use the existing node signing identity foundation proven by Phase21B and make it part of the public operator lifecycle.

Descriptor should include at least:

```text
schema/version
node_id
node public key / key id
protocol versions
capabilities
advertised peer endpoints
advertised client-serving endpoint if applicable
created/refreshed time
expiry
sequence/nonce
signature
```

Rules:

```text
node_id stable across restart
endpoint may rotate without changing node_id
admin endpoint never advertised
private paths/secrets never advertised
invalid signature rejected
expired descriptor rejected
replay/stale sequence rejected where applicable
```

## Operator commands

```text
crabnode identity show
crabnode identity rotate   # future/explicit, not silent
crabnode descriptor show
```

## Exit gate

```text
CRABNODE_CN7_NODE_ID_STABLE=GREEN
CRABNODE_CN7_DESCRIPTOR_SIGNED=GREEN
CRABNODE_CN7_ADMIN_ADVERTISED=NO
```

---

# 21. CN-8 — Wire Real Overlay Peer Transport

## Goal

Two CrabNodes must exchange real peer traffic using canonical overlay behavior.

## Work

Replace the macronode overlay shell with `svc-overlay` runtime composition.

Required beta behavior:

```text
real listener
OAP/peer handshake
bind peer connection to CrabNode identity
bounded per-peer send queue
backpressure metrics
connection timeout/idle timeout
peer session registry
bootstrap/static peer list for first private beta
clean disconnect/reconnect
message dedupe for gossip class
```

Keep DHT routing out of overlay.

Transport priorities:

```text
1. TCP/TLS or current canonical secure transport path sufficient for private beta
2. WAN hardening
3. optional Tor/other transport feature later
```

Do not make a historical Tor/Arti blueprint requirement block the first physical two-node private beta if current project policy does not require it.

## Tests

```text
A <-> B handshake
invalid identity rejected
oversized frame rejected
bounded queue under slow peer
reconnect after drop
peer session count correct on all exit paths
no lock held across await regression checks
```

## Exit gate

```text
CRABNODE_CN8_REAL_OVERLAY=GREEN
CRABNODE_CN8_TWO_NODE_TRANSPORT=GREEN
CRABNODE_CN8_BOUNDED_BACKPRESSURE=GREEN
```

---

# 22. CN-9 — Networked Provider Discovery

## Goal

Turn the strong local DHT provider model into real multi-node discovery.

## Work

Finish network provider publication/lookup around existing typed store.

Required:

```text
signed provider advertisement
CID + CrabNodeId only in provider identity record
TTL
refresh/republish
explicit withdrawal
expiry pruning
network propagation/query
bootstrap peer use
bounded alpha/beta/hop/deadline behavior
candidate selection using existing local status hints
provider spam bounds
```

Critical distinction:

```text
Provider advertisement says "node X claims availability".
Local status hint says "this node locally observed X as responsive/degraded/etc".
A provider may not erase local degraded status by re-advertising.
```

### Address resolution

Provider discovery returns CrabNode IDs. Dialing must resolve those identities through signed descriptors rather than embedding raw IPs into provider records.

## Tests

```text
A advertises CID
B queries network and discovers A
expired A record disappears
A refresh extends advertisement
A prune withdraws exact provider record
forged provider signature rejected
malformed CID/node ID rejected
unavailable local status excluded from candidates
```

## Exit gate

```text
CRABNODE_CN9_PROVIDER_REPUBLISH=GREEN
CRABNODE_CN9_NETWORK_LOOKUP=GREEN
CRABNODE_CN9_PROVIDER_WITHDRAWAL=GREEN
CRABNODE_CN9_RAW_USER_IP_PROVIDER_RECORD=NO
```

---

# 23. CN-10 — Alternate-Provider Fetch and Verify

## Goal

A CrabNode can retrieve content from another CrabNode without trusting it.

## Flow

```text
request b3:X
 -> local CAS miss
 -> query provider candidates
 -> resolve node descriptor
 -> dial provider
 -> request object/range
 -> enforce size/deadline limits
 -> compute/verify BLAKE3
 -> accept only exact match
 -> update local provider observation
 -> optionally cache under policy
 -> return verified bytes
```

## Failure behavior

```text
timeout -> try next candidate
connection failure -> local degraded/unavailable observation
hash mismatch -> reject bytes, record integrity failure, try next candidate
all candidates fail -> honest unavailable error
```

No corrupted object may be admitted to local CAS under the requested CID.

## Tests

```text
A owns object, B does not
B discovers A
B fetches exact bytes
B verifies hash
B serves verified bytes to client
A returns corrupt bytes -> B rejects
A times out -> B tries C
all providers fail -> no fake success
```

## Exit gate

```text
CRABNODE_CN10_REMOTE_FETCH=GREEN
CRABNODE_CN10_B3_VERIFY_BEFORE_TRUST=GREEN
CRABNODE_CN10_ALTERNATE_PROVIDER_RETRY=GREEN
```

---

# 24. CN-11 — Replication, Repair, and Real Swarm Behavior

## Goal

Move from "remote fetch works" to "the network can remain useful after a provider disappears."

## Initial beta model

Use a simple configurable replication factor rather than complex erasure coding first.

Suggested beta default to validate experimentally:

```text
RF target = 2
```

This is a starting implementation target, not a permanent economic constant.

## Work

```text
track observed provider count per eligible object
bounded repair queue
select repair source from verified providers
fetch + verify
persist/cache according to policy
publish new provider advertisement only after bytes available
withdraw advertisement when bytes removed/unavailable
never repair globally denied/local blocked/owner-tombstoned/quarantined object
bounded bandwidth and concurrency
repair idempotency/dedupe
```

## Torrent analogy boundary

Do **not** add a bespoke piece map/protocol yet.

Use:

```text
whole-object verification
existing range/chunk streaming for large objects
one or more providers
```

Add multi-source parallel range fetching only after performance measurements justify the complexity.

## Failure drill

```text
A and B hold object
A stops
client asks B -> success
A restarts without object -> does not advertise it
repair policy restores target RF when eligible
```

## Exit gate

```text
CRABNODE_CN11_REPLICATION=GREEN
CRABNODE_CN11_REPAIR=GREEN
CRABNODE_CN11_PROVIDER_LOSS_SURVIVAL=GREEN
CRABNODE_CN11_POLICY_BYPASS_REPAIR=NO
```

---

# 25. CN-12 — Real Durable CAS and Persistence Truth

## Goal

Make approved/pinned Service Node content actually survive restart.

## Work

Introduce a durable backend behind the existing storage trait.

Recommended properties:

```text
B3 fan-out path or equivalent bounded layout
temp-file write
stream/hash verify
atomic rename/commit
metadata/catalog transaction
startup scan/reconciliation
corrupt file detection
bounded disk quota
LRU/age/policy eviction where appropriate
explicit pinned protection
fsync policy documented/tested for beta expectations
```

## Storage classes

Preserve:

```text
ephemeral_unvetted
pending_review
verified_persistent
pinned_by_operator
operator_blocked
global_denied
owner_tombstoned
quarantined
```

Rules:

```text
unvetted content does not silently become permanent
approved/pinned content may become durable
durable flag only true after successful durable commit
provider advertisement only if serveable bytes exist
restart reconciles catalog with disk truth
```

## Tests

```text
approved object survives restart
pinned object survives restart
unvetted amnesia object does not survive when policy says RAM-only
crash during temp write does not expose partial object
corrupt durable bytes fail hash verification
catalog says present but file missing -> reconcile + withdraw provider
file exists but policy blocked -> not served/advertised
```

## Exit gate

```text
CRABNODE_CN12_DURABLE_CAS=GREEN
CRABNODE_CN12_DURABLE_BYTES_WRITTEN_TRUTH=GREEN
CRABNODE_CN12_AMNESIA_SPILL=NO
```

### Milestone M3 — Real decentralized content swarm

After CN-12, CrabNode has the essential decentralized content behavior:

```text
discover -> fetch -> verify -> cache/persist -> advertise -> survive provider loss -> repair
```

---

# 26. CN-13 — Distributed Mutable Namespace and Public State

## Goal

Make public mutable state converge across independent CrabNodes without turning the DHT or economic ledger into the wrong authority.

## Why this is separate

Immutable content is easy to decentralize because:

```text
b3 hash decides identity/integrity
```

A username claim is different:

```text
@alice cannot simultaneously belong canonically to two unrelated Passports
```

Therefore a global network needs ordered, signed transitions.

## Initial record domains

At minimum:

```text
username_claim
username_rotation/revocation if supported
public_profile_pointer
site/name_pointer
owner_tombstone pointer/state where globally meaningful
node descriptor sequence
```

## Record requirements

```text
explicit schema/version
domain tag
record key
previous state reference or sequence
authority/owner identity
nonce/idempotency key
created/accepted epoch or deterministic ordering field
payload commitment
signature
```

## Consensus rule

Reuse existing RustyOnions deterministic/quorum/checkpoint primitives where appropriate, but maintain a **separate state domain/store** from economic balances.

Forbidden shortcuts:

```text
DHT first-write-wins username truth
wall-clock latest-write-wins across untrusted nodes
DB insertion order
ron-ledger generic key/value naming state
single arbitrary Service Node permanent naming authority
```

Private-beta transition path:

```text
CN-4 local durable authority
 -> CN-13 replicated/quorum-accepted namespace transitions
```

## DHT role

DHT may:

```text
cache/distribute accepted namespace records
help locate current accepted pointer
```

DHT may not:

```text
choose the winner of conflicting signed username claims
```

## Tests

```text
A and B simultaneously propose same username for different subjects
exactly one canonical transition accepted under defined ordering/quorum
all nodes converge
losing proposal cannot become available through stale DHT cache
replay rejected
stale sequence rejected
forged signature rejected
accepted profile/site pointer survives node restart
```

## Exit gate

```text
CRABNODE_CN13_NAMESPACE_CONVERGENCE=GREEN
CRABNODE_CN13_USERNAME_CONFLICT_DETERMINISTIC=GREEN
CRABNODE_CN13_DHT_NAMESPACE_AUTHORITY=NO
CRABNODE_CN13_LEDGER_NAMESPACE_AUTHORITY=NO
```

### Milestone M4 — Network-consistent public state

At this point "any CrabNode" can participate in a network that agrees on mutable public identity/pointers rather than merely sharing immutable objects.

---

# 27. CN-14 — Moderation, Tombstones, Prune, and Persistence Across the Swarm

## Goal

Preserve operator autonomy and network safety once content has multiple providers.

## Work

Integrate current moderation/persistence policy with discovery/repair.

Rules to freeze in tests:

```text
local operator block:
  this node refuses to serve/cache/persist
  does not falsely claim global deletion

global signed deny:
  every compliant node rejects serving/repairing after valid policy acceptance

owner tombstone:
  immutable bytes may still physically exist elsewhere
  compliant resolution/serving policy follows the tombstone contract

quarantine:
  node does not serve while review is unresolved

local prune:
  remove local bytes as policy allows
  invalidate local index/provider state
  withdraw own provider advertisement
  never delete another node's bytes
```

Repair must consult policy before fetching or advertising.

## Tests

```text
prune withdraws provider
blocked object not refetched by repair
quarantined object not served
expired/invalid signed policy fails according to explicit safety posture
global deny propagation converges
local block does not masquerade as global state
```

## Exit gate

```text
CRABNODE_CN14_SWARM_MODERATION=GREEN
CRABNODE_CN14_PRUNE_WITHDRAWAL=GREEN
CRABNODE_CN14_POLICY_REPAIR_BYPASS=NO
```

---

# 28. CN-15 — Real Service Evidence and ROC Reward Closure

## Goal

Connect actual network service to the already-developed evidence/quorum/economic pipeline without giving a node unilateral payout authority.

## Reuse, do not rebuild

Reuse current:

```text
delivery evidence
availability evidence
range-repair evidence
policy evidence/challenge evidence
reward binding
quorum participation
checkpoint/finality foundations
accounting/reward policy
svc-wallet
ron-ledger
User Node replay/challenge
```

## Tighten evidence source

Evidence should be emitted only from real operations such as:

```text
verified remote object delivery
accepted availability challenge/probe
completed repair
valid policy refusal
independent User Node verification
```

## Anti-farming

Reject or downweight as designed:

```text
self-fetch loops
same-node fabricated client/provider pair
duplicate operation/idempotency key
replayed evidence
hash-invalid delivery
unverified byte counts
raw page views with no useful service
```

Do not expose user residential addresses as evidence identifiers.

## Reward flow

```text
service_node_id produces eligible evidence
 -> accounting snapshot/reward plan
 -> policy validation
 -> quorum/checkpoint acceptance
 -> registry resolves signed reward recipient binding
 -> svc-wallet performs canonical mutation
 -> ron-ledger records durable receipt/balance truth
```

## Tests

```text
real A->B verified delivery creates evidence
replay rejected
self-farming rejected
provider cannot set payout address in evidence
unbound node receives no payout
accepted plan resolves canonical binding
ledger receipt only after wallet/ledger success
one node alone cannot issue ROC
```

## Exit gate

```text
CRABNODE_CN15_REAL_SERVICE_EVIDENCE=GREEN
CRABNODE_CN15_SELF_PAYMENT=NO
CRABNODE_CN15_QUORUM_REQUIRED=YES
CRABNODE_CN15_LEDGER_RECEIPT_TRUTH=GREEN
```

### Milestone M5 — Decentralized service/economic closure

---

# 29. CN-16 — Physical Multi-Machine Private-Beta Acceptance

## Goal

Prove the network outside same-process/shared-memory test harnesses.

## Stage A

```text
Mac:
  CrabNode A
  CrabLink Client A

Windows:
  CrabLink Client B
```

Prove:

```text
Windows CrabLink -> Mac CrabNode full product gateway
Passport/profile flow
@testmac / @testpc availability/claims
profile reads
follow/feed
post/image/article publish
site resolution
paid flow where configured
```

## Stage B

```text
Mac:
  CrabNode A

Windows:
  CrabNode B
```

Prove real peer behavior:

```text
stable different node identities
peer handshake
provider advertisement
provider discovery
A-only object fetched through B
B verifies BLAKE3
B may cache/replicate under policy
stop A
B continues serving replicated object
restart A
reconcile provider state
```

## Stage C

Add Linux hardware/VM/USB test when practical:

```text
CrabNode C
```

Use it for:

```text
three-node churn
cross-platform identity persistence
quorum regression
repair behavior
```

## Hard rule

No shared in-memory fixture or test-only provider injection may count as the physical acceptance proof.

## Exit gate

```text
CRABNODE_CN16_MAC_WINDOWS=GREEN
CRABNODE_CN16_REAL_LAN_PEERING=GREEN
CRABNODE_CN16_REAL_PROVIDER_DISCOVERY=GREEN
CRABNODE_CN16_FAILURE_RESTART_RECONCILE=GREEN
```

### Milestone M6 — Controlled private beta ready

A controlled invite-only beta can reasonably begin after M6 if the remaining FINAL_BETA security/packaging/operations gates are also satisfied.

---

# 30. CN-17 — WAN/Public Operator Networking

## Goal

Move from LAN/private developer network to independently hosted public Service Nodes.

## First public-beta posture

Keep NAT traversal simple first:

```text
explicit public endpoint / port forwarding / public VPS
signed descriptor
TLS/authenticated peer transport
bootstrap seed list
```

Do not block launch on a giant NAT traversal stack.

Later opt-in enhancements may include:

```text
UPnP/NAT-PMP where safe
relay fallback
advanced traversal
alternate privacy transports
```

## Work

```text
bootstrap seed rotation
WAN endpoint validation
TLS certificate/key lifecycle or authenticated transport equivalent
anti-scan/rate-limit/connection caps
peer reputation/status hints
ban/quarantine on protocol abuse
bounded inbound concurrency
DDoS-safe early rejects where possible
```

## Privacy

```text
Service Node public address may be advertised deliberately.
User Node residential address remains private/not published.
Admin endpoint never becomes the advertised peer endpoint.
```

## Exit gate

```text
CRABNODE_CN17_WAN_TWO_NODE=GREEN
CRABNODE_CN17_PUBLIC_ADMIN=NO
CRABNODE_CN17_USER_IP_PUBLICATION=NO
```

---

# 31. CN-18 — Packaging, Installation, Upgrade, and OS Service

## Goal

Make "anyone can run a CrabNode" operationally true.

## Runtime package requirements

A released CrabNode must not require:

```text
Rust toolchain
Node.js
Vite
CrabLink Desktop
browser
source checkout
```

## Platforms

```text
Linux primary headless server target
macOS operator/dev target
Windows operator/test target
ARM/Raspberry Pi target after compile/runtime proof
```

## Packaging sequence

1. Prove foreground `crabnode run`.
2. Prove internal lifecycle.
3. Package release binaries/config.
4. Add OS service integration:

```text
systemd
launchd
Windows Service
```

OS service wrappers must call the same core runtime rather than implement a second lifecycle model.

## Upgrade rules

```text
versioned config migrations
versioned durable state
backup before destructive migration
atomic upgrade where possible
rollback path
node identity preserved
Passport/economic secrets never bundled in logs/diagnostics
```

## Exit gate

```text
CRABNODE_CN18_CLEAN_MACHINE_INSTALL=GREEN
CRABNODE_CN18_NODE_VITE_RUNTIME=NO
CRABNODE_CN18_IDENTITY_PRESERVED_UPGRADE=GREEN
CRABNODE_CN18_ROLLBACK=GREEN
```

---

# 32. CN-19 — Chaos, Soak, Security, and Public-Beta Exit

## Goal

Prove a public CrabNode fails safely under real abuse and churn.

## Chaos matrix

```text
peer disconnect storm
bootstrap outage
DHT stale/poisoned records
provider timeout
provider hash corruption
partial disk write
disk full
read-only filesystem
catalog/file mismatch
service crash loop
slow storage
slow peer/backpressure
packet loss/latency
network partition
namespace conflicting claims
stale namespace replay
expired node descriptor
reward evidence replay
quorum member loss
operator policy expiry
restart during repair
restart during persistence commit
```

## Security matrix

```text
malformed OAP frames
oversized frame/body
compression bomb where compression accepted
path traversal
CID mismatch
provider-record spoof
node descriptor forgery
admin brute force/rate behavior
remote admin without explicit opt-in
secret/log redaction
capability replay
unbounded metric labels
unbounded peer/provider state
Sybil/identity churn probes
self-reward farming probes
```

## Soak

Run progressively:

```text
1 hour
8 hours
24 hours
```

with:

```text
content reads/writes
provider refresh
peer reconnect
repair
moderation/persistence activity
metrics sampling
```

No uncontrolled memory/FD/task growth.

## Final broad gates

```bash
cargo check --workspace
```

Then run current project-wide focused preflights/strict clippy/test scripts appropriate to the touched crates.

## Exit gate

```text
CRABNODE_CN19_CHAOS=GREEN
CRABNODE_CN19_SOAK=GREEN
CRABNODE_CN19_SECURITY=GREEN
CRABNODE_CN19_RESOURCE_LEAK=NO
CRABNODE_PUBLIC_BETA_OPERATOR_READY=GREEN
```

### Milestone M7 — Public operator CrabNode ready

---

# 33. Milestone / Beta Critical Path

The plan intentionally separates **what must happen immediately for FINAL_BETA** from the full public decentralized-node goal.

## M1 — Immediate Phase21 unblock

```text
CN-0
CN-1 enough lifecycle for standalone run
CN-2 canonical profile
CN-3 full gateway/Omnigate
CN-4 durable local username/profile
```

Outcome:

```text
Mac can run one real standalone CrabNode.
Mac and Windows CrabLink can use the real backend.
Username/profile state survives restart.
No developer dashboard is required.
```

## M2 — Standalone node product

Add:

```text
CN-5 full product path
CN-6 supervision
```

Outcome:

```text
CrabNode is a coherent headless product even before full swarm behavior.
```

## M3 — Real content decentralization

Add:

```text
CN-7 identity/descriptor
CN-8 overlay
CN-9 DHT network provider discovery
CN-10 remote fetch
CN-11 replication/repair
CN-12 durable CAS
```

Outcome:

```text
The content layer is genuinely swarm-like and no longer dependent on one origin node.
```

## M4 — Decentralized mutable public state

Add:

```text
CN-13 namespace/state consensus
CN-14 distributed moderation/prune semantics
```

Outcome:

```text
Independent CrabNodes can converge on mutable identity/site state safely.
```

## M5 — Economic closure

Add:

```text
CN-15 real service evidence -> ROC pipeline
```

## M6 — Physical private beta

Add:

```text
CN-16 Mac/Windows/Linux physical proof
```

## M7 — "Anyone can run one" public-beta posture

Add:

```text
CN-17 WAN
CN-18 packaging
CN-19 chaos/security/soak
```

---

# 34. Acceptance Scenarios That Define a Finished CrabNode

A buildplan should end in behaviors, not only crate checks.

## Scenario A — Fresh operator

```text
install CrabNode
crabnode init
crabnode start
crabnode status -> healthy
no browser/Node/Vite/CrabLink required
admin bound locally
```

## Scenario B — CrabLink product backend

```text
clean CrabLink -> CrabNode
create/restore Passport
check username
claim username
create profile
publish content
second client reads it
restart node
identity/profile/content state required by policy remains correct
```

## Scenario C — Content swarm

```text
A owns b3:X
B does not
A advertises
B discovers A
B fetches X
B verifies BLAKE3
B stores according to policy
B advertises if eligible
client reads X from B
A disappears
B still serves X
```

## Scenario D — Corrupt provider

```text
A advertises b3:X but sends wrong bytes
B rejects bytes
B records local degraded/integrity observation
B tries C
C sends correct X
B accepts only C's verified bytes
```

## Scenario E — Prune/policy

```text
operator blocks/prunes X on B
B stops serving X
B withdraws provider record
repair does not immediately fetch X again
A/C remain independent
```

## Scenario F — Namespace conflict

```text
A proposes @alice -> Passport A
B concurrently proposes @alice -> Passport B
network applies deterministic accepted transition rule
all honest nodes converge on one accepted owner
loser does not become canonical due to DHT timing
```

## Scenario G — Service evidence

```text
B performs real verified service
signed evidence enters canonical pipeline
single B cannot mint itself ROC
quorum accepts valid epoch transition
wallet/ledger produce canonical receipt
CrabLink displays backend-derived result only
```

## Scenario H — Crash recovery

```text
storage/gateway child crashes
ready turns false/degraded immediately
bounded backoff/restart occurs
crash loop eventually stops retrying and reports failure
operator can diagnose through crabnode status/doctor/logs
```

---

# 35. Performance/Scaling Posture

Do not prematurely implement global-scale complexity before measuring the two/three-node system.

## Beta priorities

```text
bounded correctness
backpressure
stream/range behavior
provider TTL
alternate-provider retry
RF=2 style replication
repair throttling
```

## Later measurement-driven enhancements

```text
hedged provider dials
parallel range fetch
regional preference
larger DHT routing tables
adaptive provider scoring
erasure coding
edge tier
advanced NAT traversal
```

The older scaling blueprint's global SLOs and signed-provider/TTL concepts remain useful targets, but current code should graduate toward them through measurements rather than being declared implemented early.

---

# 36. Observability Requirements

At minimum CrabNode status/metrics should make these visible without leaking secrets:

```text
node_id
version
uptime
profile
overall ready state
capability readiness
service lifecycle states
restart counts
connected peers
provider records local count
provider lookup success/failure
remote fetch success/failure/hash mismatch
bytes in/out
storage object count/cache/durable usage
repair queue depth/results
policy/moderation state
persistence pending count
reward binding state
quorum eligibility/status when enabled
```

High-cardinality values such as full arbitrary user strings must not become metric labels.

Logs should use structured safe identifiers where possible and redact credentials/capabilities.

---

# 37. Configuration and Migration Rules

## Configuration

```text
one canonical CrabNode operator config
strict parsing
unknown-field rejection where practical
explicit profile/capability enablement
safe defaults
loopback admin default
no secret echo in config print
```

## Migration

Every durable format must have:

```text
schema/version
migration test
rollback/backup story
corruption behavior
```

Node identity must not silently change because a config or state format migrated.

---

# 38. Things We Deliberately Do Not Build to Finish CrabNode

This plan does not require:

```text
new node type
new CLI crate
new application gateway semantics
custom BitTorrent protocol
blockchain for immutable content lookup
ron-ledger as generic database
public ROX/Solana bridge
staking
liquidity
exchange support
mainnet
browser dashboard dependency
cloud login
server custody of Passport recovery secrets
public follower graph
user residential inbound sockets
```

---

# 39. Documentation Drift Ledger — Record Now, Fix After Buildplan Approval

The user explicitly requested that better designs/corrections to older docs be handled **after** this buildplan is created.

Therefore no old document should be mass-edited as part of initial CN-0 implementation.

The following drift is already identified for later surgical cleanup:

## D1 — Public product naming

Older wording often says:

```text
Service Node daemon = macronode
crabnode = CLI
```

Preferred public model after approval:

```text
CrabNode = complete public Service Node product
crabnode = executable/operator interface
macronode = internal host/runtime implementation
```

Internal crate name can remain unchanged.

## D2 — BUILD_PLAN_Z phase counter is stale relative to source

BUILD_PLAN_Z says:

```text
Phases 0-11 complete
Phase 12 next
```

but current source and Phase21B proof contain substantial behavior from later conceptual phases.

Do not rerun/rebuild later phases blindly from the old counter.

## D3 — Old macronode descriptions overstate full service wiring

Some docs describe a complete full service mesh while current runtime still has at least:

```text
macronode svc-overlay wrapper = host shell
macronode svc-mailbox wrapper = host shell
macronode gateway wrapper = MVP ping only
```

DHT has improved beyond the older shell and should be documented accordingly later.

## D4 — Older CLI lists include commands not present in current source

Historical docs include examples such as:

```text
logs tail
metrics summary
identity init/show
policy sync
```

Current source does not expose all of those commands yet.

CN-1 should implement the desired product lifecycle first, then docs should be corrected to exact source.

## D5 — Gateway port/role ambiguity

There are references to both:

```text
macronode embedded MVP gateway
standalone full svc-gateway
```

with the same general product port convention.

CN-2 must establish one owner before docs are normalized.

## D6 — Username "confirmed" wording

Current svc-passport profile store is local/in-memory today.

Docs/UI must eventually distinguish:

```text
local/private-beta accepted claim
vs
network-global quorum/finalized namespace ownership
```

## D7 — DHT global capability overstatement

Older scaling docs describe mature signed provider republish/global routing behavior, while current source still has explicit republish/transport placeholders.

Retain those as targets, not current implementation claims.

## D8 — Mailbox overstatement

Do not describe the current macronode mailbox wrapper as a complete at-least-once/DLQ service until real wiring proves it.

## D9 — Persistence state vs durable bytes

Keep the current correct distinction:

```text
policy eligibility/operator intent != bytes successfully persisted
```

## D10 — Economic consensus vs namespace consensus

Do not rewrite docs in a way that makes QuickChain/ron-ledger the generic authority for usernames/sites merely because consensus primitives already exist.

## D11 — Historical transport/Tor wording

Some old blueprints make Tor/Arti a strong interop target while newer source comments and current BUILD_PLAN_Z posture have narrowed or changed this area.

Resolve that policy explicitly after the current TCP/TLS/private-beta transport is green; do not let stale wording drive implementation accidentally.

## D12 — CrabLink operator terminology

CrabLink correctly treats the Service Node as independently running. Later copy should consistently say it is optionally **managing/observing a CrabNode**, not that CrabLink itself is the node runtime.

---

# 40. First Implementation Action After This Plan Is Approved

Do **not** begin by editing all docs.

Do **not** begin with DHT swarm code.

Do **not** begin with a new crate.

Start with the current repo and first failure:

```bash
cd /Users/mymac/Desktop/RustyOnions
cargo check --workspace
```

Then inspect the current CrabNode/macronode compilation/tests and implement **CN-0**, followed by **CN-1/CN-2/CN-3**.

The fastest useful sequence is:

```text
workspace green
 -> real crabnode lifecycle
 -> one canonical Service Node profile
 -> full gateway + Omnigate composition
 -> durable local Passport username/profile state
 -> two-client Mac/Windows FINAL_BETA identity/feed demonstration
 -> real supervision
 -> overlay + provider network
 -> remote fetch/replication/repair
```

This sequence gets the existing beta unstuck while moving directly toward the final decentralized architecture rather than building throwaway demo shortcuts.

---

# 41. Final Definition of Done

A CrabNode is finished for public beta when all of the following are true:

```text
PUBLIC PRODUCT
  one installable CrabNode product
  one crabnode operator entrypoint
  no developer dashboard/runtime dependency

LIFECYCLE
  init/start/run/stop/restart/status/doctor/logs
  self-monitoring and bounded restarts

CONTENT
  B3-addressed
  hash verified
  provider discovery
  remote fetch
  replication/repair
  policy-aware durable persistence

NETWORK
  stable signed node identity
  signed endpoint descriptor
  real peer transport
  DHT provider propagation
  no user residential IP publication

MUTABLE PUBLIC STATE
  deterministic signed namespace transitions
  restart durability
  cross-node convergence
  DHT is distribution/cache, not uniqueness authority

POLICY
  local operator autonomy
  signed global policy/tombstone support where defined
  prune withdraws provider state
  repair never bypasses policy

ECONOMICS
  real service evidence
  anti-replay/anti-farming
  reward binding
  quorum acceptance
  wallet/ledger-confirmed ROC truth only

OPERATIONS
  clean Mac/Windows/Linux proof
  WAN proof for public operators
  safe upgrades/rollback
  chaos/soak/security green
```

One-line finished-product doctrine:

```text
CrabNodes discover and serve immutable verified content like a policy-aware swarm,
converge on mutable public state through signed deterministic network transitions,
and earn ROC only through independently verifiable service evidence and quorum-confirmed economic truth.
```

