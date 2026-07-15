# CrabLink Private Beta Node Runbook

RO:WHAT — Authoritative private-beta guide for the two-node CrabLink product model.
RO:WHY — Gives normal CrabLink users and Service Node operators one truthful entry point without requiring them to assemble current behavior from legacy crate runbooks.
RO:INTERACTS — CrabLink desktop, micronode, macronode, crabnode, svc-admin, svc-storage, svc-dht, svc-registry, svc-rewarder, svc-wallet, and ron-ledger.
RO:INVARIANTS — Users verify privately; Service Nodes serve headlessly; no node mints alone; confirmed ROC comes only from a durable wallet/ledger receipt.
RO:CONFIG — Private/dev profiles only; privacy mode enabled by default; admin UI loopback-only; unvetted content amnesia-first.
RO:SECURITY — No residential-IP publication, arbitrary payout address, self-payment, unilateral minting, fake balance, fake receipt, fake confirmed ROC, or fake finality.
RO:TEST — bash scripts/check-phase24-private-beta-node-runbook.sh.

## 1. Status and scope

This is the operator and user entry point for the CrabLink private beta.

The product has two public node roles:

```text
CrabLink User Node
CrabLink Service Node
```

There is no third public Economic Node product.

Economic responsibilities remain separated across evidence review, accounting,
policy, registry resolution, service-node quorum, wallet execution, ledger
receipts, and User Node replay.

The private beta does not authorize:

```text
public ROX runtime
public Solana runtime
ROC ↔ ROX bridge runtime
staking
liquidity
exchange-facing behavior
public validator economy
production mint/burn settlement
```

## 2. Two-node model

### CrabLink User Node

The User Node is the private, passive verifier managed by the CrabLink desktop
application.

It is expected to be:

```text
loopback-only
amnesia-first
resource bounded
non-serving
non-public
pauseable
private by default
```

It can verify content, evidence, epoch transitions, quorum signatures, caps,
replay consistency, and supply conservation.

It cannot:

```text
serve public content by default
publish the user's residential IP
mint ROC
approve its own rewards
mutate a wallet
mutate the ledger
invent confirmed ROC
```

### CrabLink Service Node

The Service Node is the public, headless content and protocol-service node.

It can:

```text
serve canonical b3/OAP content
advertise canonical crab:// node identity
honor moderation and tombstones
prune exact local content
manage persistence eligibility
produce delivery/availability/repair evidence
participate in eligible service-node quorum
bind rewards to an external CrabLink/RON @ address
```

It cannot:

```text
mint ROC alone
pay its own operator directly
choose an arbitrary payout address in evidence
bypass registry reward binding
turn evidence into receipt truth
turn a reward plan into confirmed ROC
```

## 3. CrabLink User Node UX guide

A normal user should not manage:

```text
ports
public listeners
RPC URLs
server logs
admin passwords
routing tables
provider records
quorum membership
```

CrabLink owns the User Node lifecycle.

The expected user-facing states are:

```text
warming
active
paused
degraded
```

A paused or degraded node must remain truthful. It may retain read-only pending
evidence, but it must reject new verification work when paused and must not
consume replay identity for rejected work.

Pending evidence is not a reward.

A pending reward plan is not a balance.

Confirmed ROC may appear only after CrabLink receives backend-derived receipt
truth from the wallet/ledger path.

### Local Micronode inspection

From the RustyOnions workspace:

```bash
cargo run -p micronode -- check
cargo run -p micronode -- status
cargo run -p micronode -- --help
```

`check` validates the real configuration without starting listeners.

`status` uses the same status builder as the local admin API. It must not print
a fabricated balance, receipt, or confirmed-ROC value.

CrabLink ordinarily starts and manages the User Node. Direct `serve` use is a
development and diagnostic surface:

```bash
cargo run -p micronode -- serve
```

The passive User Node configuration must remain loopback-only.

## 4. Service Node operator quickstart

A Service Node is headless-first. The daemon does not require:

```text
a browser
the optional svc-admin UI
the CrabLink desktop application
a Node/Vite development server
a desktop operating system
```

Before changing local state, inspect the available CLI:

```bash
cargo run -p macronode --bin crabnode -- --help
```

Inspect the Service Node through its loopback admin surface:

```bash
cargo run -p macronode --bin crabnode -- \
  --admin-url http://127.0.0.1:8080 \
  status
```

Use the configured loopback URL when the node uses a different local admin
port.

The CLI refuses non-loopback admin URLs by default. Do not work around that
protection for the private beta.

## 5. crabnode CLI guide

The implemented operator command families include:

```text
status
admin
rewards
policy/moderation
prune
persistence
```

Always inspect current help before a mutation:

```bash
cargo run -p macronode --bin crabnode -- --help
```

Use dry-run where the command supports it:

```bash
cargo run -p macronode --bin crabnode -- \
  --dry-run \
  rewards show
```

A dry-run describes the local request or mutation boundary. It does not claim
runtime activation, network-wide propagation, wallet mutation, ledger
mutation, payout, receipt, or finality.

## 6. Optional admin UI guide

The optional admin UI is an operator convenience layer.

It is not required for Service Node runtime.

Private-beta posture:

```text
disabled by default
loopback-only when enabled
authenticated after bootstrap
not publicly exposed
not a wallet
not a ledger
not issuance authority
```

The UI may display and control the same real local operator surfaces used by
the CLI, including moderation, pruning, persistence review, binding status,
evidence counts, quorum status, and confirmed ledger receipts.

The UI must not turn pending evidence or pending reward plans into confirmed
ROC.

Basic CLI vocabulary:

```bash
cargo run -p macronode --bin crabnode -- admin setup-token
cargo run -p macronode --bin crabnode -- admin create-user admin
cargo run -p macronode --bin crabnode -- admin enable-web
cargo run -p macronode --bin crabnode -- admin disable-web
```

Run `admin --help` before use to confirm the current required flags and local
paths.

## 7. First-run setup guide

For a fresh optional admin UI:

1. Keep the Service Node admin bind on loopback.
2. Request the short-lived setup token.
3. Open or submit the local setup flow.
4. Create the first local administrator.
5. Confirm the token is burned after successful use.
6. Disable the web UI when it is not needed.

Setup tokens are:

```text
short-lived
single-use
local bootstrap credentials
not reward credentials
not wallet credentials
not ledger credentials
```

There is no default centralized password-reset flow.

If the local administrator credentials are lost, stop the optional admin
surface and recreate its node-local admin state. The durable reward recipient
remains the separately bound external CrabLink/RON account.

## 8. Reward @ address binding guide

Service Node rewards resolve through a registry binding:

```text
service_node_id
→ registry reward binding
→ reward_recipient_account_id
→ wallet execution
→ ledger receipt
```

An operator-facing `@address` is a display identity backed by the registry
binding. It is not an arbitrary address carried inside delivery evidence.

Inspect current binding state:

```bash
cargo run -p macronode --bin crabnode -- \
  --admin-url http://127.0.0.1:8080 \
  rewards show
```

Preview a new binding:

```bash
cargo run -p macronode --bin crabnode -- \
  --dry-run \
  rewards bind @operator
```

Submit through the configured local admin endpoint:

```bash
cargo run -p macronode --bin crabnode -- \
  --admin-url http://127.0.0.1:8080 \
  rewards bind @operator
```

Rotate to a new future recipient:

```bash
cargo run -p macronode --bin crabnode -- \
  --admin-url http://127.0.0.1:8080 \
  rewards rotate @new-operator
```

A rotation is future-epoch registry state. It is not an immediate payout
redirect.

A second pending rotation for the same Service Node is rejected. A rejected
replacement does not overwrite the accepted rotation or consume its nonce.

## 9. ROC mining and reward explanation

“Mining” in this private beta means producing useful, reviewable protocol
evidence. It does not mean unilateral block production or unilateral minting.

The reward path is:

```text
useful service or verification
→ signed/validated evidence
→ replay and abuse checks
→ accounting snapshot
→ deterministic capped reward plan
→ policy and eligibility review
→ registry-bound recipient resolution
→ valid service-node quorum
→ svc-wallet execution
→ ron-ledger durable receipt
→ confirmed ROC display
```

The following are not confirmed ROC:

```text
raw engagement
bytes served by themselves
pending evidence
proof_eligible evidence
accounting totals
reward plans
quorum proposals
wallet requests without accepted execution
client-side cached values
```

## 10. Service-node quorum explanation

A single Service Node cannot execute a reward transition.

Quorum review includes:

```text
eligible signer set
real signature verification
policy hash
economics configuration hash
registry root
reward-binding root
accounting root
reward-plan root
evidence root
cap and conservation checks
```

Candidate and probation nodes cannot control mature quorum weight.

Many newly created nodes cannot gain control merely by increasing node count.

Quarantined or blocked nodes cannot provide valid reward authority.

There is no permanent founder-approved trusted-node flag in the production
doctrine.

## 11. IP privacy explanation

A User Node must not publish:

```text
residential IP address
raw socket address
transport route
public admin bind
peer IP display
```

Canonical public node identity uses:

```text
crab://node/<node-id>
```

Legacy `relay://`, `onion://`, and `service://` addressing is not part of the
current product contract.

Service provider records must use canonical node identity rather than raw IP
or transport addresses.

CrabLink fetches through its configured privacy-compatible OAP route. If that
route is unavailable, verification fails before User Node submission. CrabLink
does not silently fall back to a direct public route.

Privacy mode is a safety boundary, not a claim that every future network
transport has already shipped.

## 12. Moderation, pruning, and policy operations

Object-taking commands require a canonical content identifier:

```text
b3:<64 lowercase hexadecimal characters>
```

Inspect CLI help first:

```bash
cargo run -p macronode --bin crabnode -- --help
```

Examples:

```bash
cargo run -p macronode --bin crabnode -- \
  --dry-run \
  block b3:<64-lowercase-hex>

cargo run -p macronode --bin crabnode -- \
  --dry-run \
  quarantine b3:<64-lowercase-hex>

cargo run -p macronode --bin crabnode -- \
  --dry-run \
  prune b3:<64-lowercase-hex>
```

The local operator policy may:

```text
block
unblock
allow
remove a local allow
quarantine
release local quarantine
inspect local policy state
```

Precedence remains fail-closed:

```text
global deny defeats local allow
owner tombstone defeats local allow
local block defeats local allow
quarantine defeats local allow
```

Pruning is exact local-state work. It must report each unavailable surface
truthfully and must not claim network-wide deletion when propagation or remote
invalidation is unavailable.

Moderation and pruning never mutate wallet or ledger state.

## 13. Persistence policy guide

Unvetted content is amnesia-first.

The persistence state model includes:

```text
ephemeral_unvetted
pending_review
verified_persistent
operator_blocked
global_denied
owner_tombstoned
quarantined
pinned_by_operator
```

Only policy-approved states may become durable-storage eligible.

Important distinctions:

```text
approval is eligibility, not proof of a durable write
pinning cannot skip approval
unpinning does not erase verified eligibility
blocked, denied, tombstoned, or quarantined content cannot become eligible
unsafe or unavailable moderation fails closed
```

Inspect current persistence commands:

```bash
cargo run -p macronode --bin crabnode -- persistence --help
```

Typical operator vocabulary includes:

```text
persistence list
persistence approve
persistence reject
persistence pin
persistence unpin
persistence show
```

Use canonical B3 identifiers and dry-run before mutations when supported.

## 14. Policy and denylist operations

Local operator policy is distinct from signed global policy.

A local command must not claim that it changed:

```text
global deny authority
owner tombstone authority
accepted signed policy epoch
trusted signer configuration
network-wide policy
```

Policy writes must be validated and replaced atomically.

An invalid existing policy fails closed rather than being overwritten.

A changed policy file must not be described as active in an already-running
node unless the runtime has actually loaded or reloaded it.

## 15. Incident response guide

### User Node incident

Use CrabLink to pause the User Node when:

```text
verification errors repeat
resource use is abnormal
privacy posture is uncertain
configured privacy route is unavailable
pending evidence grows unexpectedly
```

Expected safe posture:

```text
new verification rejected
read-only local truth retained
no replay identity consumed for rejected work
no reward or receipt fabricated
```

### Service Node content incident

For corrupt, denied, tombstoned, or suspicious content:

1. Stop serving or quarantine the exact B3 object.
2. Inspect the effective local and signed policy state.
3. Prune exact local state when appropriate.
4. Verify provider withdrawal and cache/index invalidation results separately.
5. Do not claim network-wide deletion from a local prune.
6. Preserve evidence needed for audit without exposing private user routing.

### Economic incident

For rewarder, wallet, ledger, quorum, or binding anomalies:

1. Stop new economic writes or emission at the failing readiness boundary.
2. Keep liveness and read-only inspection available where safe.
3. Preserve the original idempotency and operation identities.
4. Do not retry a quarantined invariant failure as ordinary transient work.
5. Reject replacement reward bindings while one is already pending.
6. Verify wallet receipts against durable ledger truth.
7. Resume only after deterministic replay and conservation checks pass.

### Privacy incident

If an IP, socket, or transport identity appears in a public provider record:

1. Reject the record.
2. Remove or quarantine the publishing source.
3. Inspect logs and status output for additional leaks.
4. Do not replace it with another raw transport identifier.
5. Restore only canonical `crab://node/<node-id>` identity.

### Optional admin credential loss

There is no central password recovery.

Stop the optional UI, recreate its node-local administrative state, generate a
new setup token, and bootstrap a new local administrator.

Do not alter the external reward binding merely to recover local UI access.

## 16. Known limitations

The private beta currently has these deliberate limitations:

```text
private/dev configuration posture
no public ROX or Solana runtime
no live ROC ↔ ROX bridge
no staking, liquidity, or exchange surface
admin UI is optional and local-only
User Node is managed through CrabLink rather than as a public server
unvetted content is amnesia-first
persistence approval alone is not durable-write proof
reward plans are not balances
only ledger receipts confirm ROC
no centralized Service Node admin-password recovery
some live multi-process tests require explicit opt-in
privacy depends on the configured privacy-compatible route failing closed
```

The older per-crate runbooks remain useful technical references, but some
contain broader historic RON-CORE deployment language. This document controls
the CrabLink private-beta product posture when the two conflict.

## 17. Private-beta acceptance proof

The current acceptance evidence includes:

```text
Phase 22 final acceptance: GREEN / PARKED
Phase 23 final acceptance: GREEN / PARKED
strict Clippy green for touched node/economic crates
RustyOnions workspace cargo check green
CrabLink Tauri privacy boundary green
```

Phase 23 proved fail-closed behavior for:

```text
disabled admin UI
expired setup token
paused User Node
stale or IP-leaking provider records
corrupt and denylisted content
cache eviction and resident tombstones
proof replay
mid-epoch reward-binding replacement
rewarder dependency outage
duplicate payout
ledger replay mismatch
single-node issuance attempt
Sybil quorum attempt
invalid challenge submission
privacy-route outage
```

All Phase 24 documentation, first-run, operator, behavior, strict-Clippy,
workspace-compilation, and CrabLink privacy checks are green.

The private-beta candidate is now:

```text
COMPLETE / GREEN / PARKED
```

The reproducible final acceptance marker is:

```text
PHASE24_FINAL_STATUS=GREEN_PARKED
```
