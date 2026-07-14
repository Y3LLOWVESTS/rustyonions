
# BUILD_PLAN_Z.md — CrabLink Two-Node Network, Service-Node Operator UX, and ROC Rewards

RO:WHAT — Build plan for the RustyOnions / CrabLink two-node network: passive CrabLink User Nodes and headless-first CrabLink Service Nodes with optional operator UI, moderation/pruning controls, persistence policy, ROC reward recipient binding, and decentralized issuance rules.

RO:WHY — CrabLink needs a practical decentralized node layer where normal users passively verify useful work and earn ROC, while service-node operators serve b3/OAP content, moderate what they host, reduce liability exposure, and earn ROC through evidence-backed useful service. The design must not depend on a central trusted issuer, must not expose user IPs, and must not require node operators to run a full web-cloud dashboard or desktop OS just to operate a node.

RO:INTERACTS — CrabLink Tauri, crabnode CLI, svc-admin, micronode, macronode, oap, ron-app-sdk, ron-proto, svc-gateway, omnigate, svc-storage, svc-index, svc-dht, svc-overlay, svc-mailbox, ron-accounting, svc-rewarder, ron-policy, svc-wallet, ron-ledger, ron-audit, svc-passport, ron-auth, ron-kms, svc-registry, ron-transport.

RO:INVARIANTS — Only two public node products exist at this stage: CrabLink User Node and CrabLink Service Node. Regular users are not sysadmins. User nodes run passively and privately in the background. Service nodes are headless-first public infrastructure nodes with optional admin UI. Service nodes can serve content, moderate local hosting, emit useful-service evidence, and participate in quorum, but no single node can mint ROC. ROC issuance is an evidence-backed epoch transition accepted by quorum and replay-verified by user nodes. Node operators may receive ROC only through a bound reward recipient account, such as a CrabLink `@` address / RON Passport identity, after wallet/ledger-confirmed payout. No fake balances, fake receipts, fake finality, raw-engagement minting, direct self-payment, or unilateral minting. All mutable ROC economics values must come from reviewed TOML under `configs/`; runtime crates must not scatter mutable reward rates.

RO:SECURITY — Capabilities only; no ambient authority. User nodes are outbound/private by default and must not expose residential IPs to other users. Direct user-to-user TCP is forbidden by default. Service nodes may publish public service addresses but must not leak user residential IPs. Service-node admin is loopback-only by default, UI disabled or on-demand by default, and CLI always available. Moderation/pruning/persistence controls must be first-class. Unvetted asset bytes are amnesia-first. Durable economic data is allowed only where required for wallet/ledger/audit truth. Bad nodes are challenged, downranked, quarantined, reward-denied, or blocked.

RO:TEST — Focused crate checks per phase, then integration smokes: two-node status truth, headless service-node runtime, CLI admin controls, optional local admin UI bootstrap, no-IP-leak tests, OAP b3 serve/fetch, moderation/pruning tests, persistence policy tests, denylist/tombstone rejection, delivery/availability evidence, node reward recipient binding, accounting/rewarder/policy planning, service-node quorum issuance, svc-wallet/ron-ledger replay, user-node fraud challenges, anti-farming, Sybil resistance, chaos drills, and private-beta readiness. Canonical ROC economics config parsing, validation, hashing, explicit profile selection, and hash-mismatch rejection are required before reward execution.

---

# 0. Current Status Snapshot

The authoritative two-node direction remains:

```text
CrabLink User Node:
  private passive verifier / economic auditor / useful-work miner

CrabLink Service Node:
  public service node / content host / provider / quorum participant
```

Current phase status:

```text
Phases 0 through 11: COMPLETE / GREEN / PARKED
Phase 12: NEXT
Completed phases: 12 of 25
Raw phase-count completion: 48%
```

Most recent closeout:

```text
Phase 11 established:
  canonical persistence policy in ron-policy
  bounded amnesia-first MemoryStorage in svc-storage
  transactional PersistenceCatalog in svc-storage
  macronode HTTP persistence controls
  crabnode persistence CLI controls
  live cross-process persistence-state proof
  moderation refusal and fail-closed closeout tests
```

Important Phase 11 truth:

```text
verified_persistent and pinned_by_operator express policy eligibility
and operator intent. They do not prove that durable bytes were written.
durableBytesWritten remains false until a real durable backend exists.
```

Next implementation posture:

```text
Phase 12 — Provider Discovery, Redundancy, and Repair

Start with current-source inspection and the smallest real provider
record or expiration behavior. Do not claim provider withdrawal,
repair completion, or redundancy before those behaviors exist.
```

---

# 1. Executive Summary

CrabLink node architecture uses two public node types.

```text
1. CrabLink User Node
   Passive background node bundled/managed by CrabLink.
   Runs for normal users.
   Users are not sysadmins.
   Verifies receipts, reward plans, ledger samples, epoch transitions, content integrity, and service evidence.
   Produces reward-eligible verification/audit evidence.
   Can earn ROC through accepted verification work.
   Does not expose public inbound services by default.
   Does not reveal residential IPs.
   Does not mint, issue, transfer, burn, or create ledger receipts.

2. CrabLink Service Node
   Headless-first public operator node based on macronode/service composition.
   Serves b3-addressed content over OAP/1.
   Advertises provider availability.
   Runs policy, denylist, tombstone, pruning, and local operator moderation.
   Emits delivery, availability, repair, policy-refusal, and service evidence.
   Participates in decentralized reward/ledger epochs.
   May propose/sign valid epoch transitions.
   Cannot unilaterally mint ROC.
   Receives ROC only through a bound operator reward recipient account after approved ledger payout.
```

The previous third public node type:

```text
Economic Node
```

remains removed as a public product category.

Economic functionality is redistributed:

```text
User Node:
  read-only economic verifier
  ledger replay auditor
  epoch challenger
  fraud detector
  reward evidence producer

Service Node:
  service evidence producer
  content provider
  reward epoch proposer
  quorum participant
  wallet/ledger transition participant only under quorum rules
```

One-line doctrine:

```text
Users verify privately. Service nodes serve safely. ROC pays only through ledger-confirmed evidence.
```

---

# 2. Product Posture Reset: RON-CORE vs RustyOnions Service Nodes

## 2.1 RON-CORE posture

RON-CORE may keep a larger self-hostable cloud/admin posture in a separate repo or future profile.

That posture may include:

```text
full web dashboard
multi-tenant admin console
SSO/ingress auth
custom templates
advanced metrics dashboards
deep cloud-style configuration
public/self-hosted operations
```

This is not the default RustyOnions service-node posture.

## 2.2 RustyOnions service-node posture

RustyOnions service nodes should be:

```text
headless-first
small footprint
Raspberry Pi friendly
CLI-manageable
optional web UI
loopback-only admin by default
amnesia-first for unvetted asset bytes
moderation/pruning first-class
operator liability aware
reward-recipient bound
```

Required runtime:

```text
service-node daemon
crabnode CLI
node identity/key material
policy/denylist/tombstone engine
storage/cache service
OAP service
metrics/health endpoints
```

Optional runtime:

```text
local admin web UI
CrabLink operator management view
advanced dashboards
remote/LAN admin exposure
```

Not required for runtime:

```text
desktop OS
browser GUI
Vite dev server
Node.js at runtime
full RON-CORE cloud dashboard
public admin console
```

## 2.3 svc-admin new role

For this build plan, `svc-admin` becomes:

```text
optional local service-node operator console
not required for node runtime
not normal user UX
not the source of wallet/ledger truth
not a cloud dashboard by default
```

The service node must remain operable when `svc-admin` is disabled.

---

# 3. Node Operator ROC Reward Identity

## 3.1 Core rule

A node operator can earn ROC, but the node itself is not the recipient identity.

Correct model:

```text
service_node_id earns reward-eligible service evidence
→ reward plan references service_node_id
→ registry resolves service_node_id to reward recipient
→ wallet pays the bound CrabLink/RON account
→ ledger records receipt
```

The recipient should be something like:

```text
@operator
@creator
@node-owner
RON Passport account
CrabLink account address
```

The exact UI spelling may be:

```text
@alice
@my-service-node
@rusty-cache-01
```

But internally it should resolve to a durable account identifier, not just a display string.

## 3.2 Node reward recipient binding

Each service node must register a reward recipient binding.

Future DTO:

```text
ServiceNodeRewardBindingV1
  binding_id
  service_node_id
  operator_account_id
  operator_display_address
  operator_passport_id optional
  reward_recipient_account_id
  reward_recipient_display_address
  node_public_key
  operator_signature
  node_signature
  created_at
  effective_epoch
  expires_at optional
  rotation_nonce
  policy_hash
```

Human-facing example:

```text
service_node_id: node_b3c9...
reward_recipient_display_address: @stevan
reward_recipient_account_id: acct_...
```

## 3.3 Binding creation flow

Service-node setup should include:

```text
operator starts service node
→ node creates or loads node keypair
→ operator enters CrabLink @ address or connects CrabLink/RON Passport
→ system resolves @ address to account id
→ operator signs binding with account/passport capability
→ node signs binding with node key
→ svc-registry stores signed descriptor
→ node starts as candidate/probation
```

CLI flow:

```bash
crabnode identity init
crabnode rewards bind @operator
crabnode rewards show
crabnode registry register
```

Admin UI flow:

```text
Setup wizard:
  1. Create/load node identity
  2. Enter reward @ address
  3. Verify account ownership
  4. Sign reward binding
  5. Register node descriptor
```

CrabLink management flow:

```text
CrabLink → Node Operator Mode → Bind reward address → sign with CrabLink account
```

## 3.4 Reward payout rule

Rewards may only pay to the currently valid reward binding.

```text
reward plan references service_node_id
service_node_id resolves to reward_recipient_account_id
svc-wallet pays reward_recipient_account_id
ron-ledger records account receipt
CrabLink displays confirmed ROC for that @ account
```

Reward plans must not accept arbitrary payout addresses directly from service evidence.

Forbidden:

```text
delivery proof includes payout address
availability proof includes payout address
service node chooses payout account per claim
rewarder pays raw @ address string without registry/account resolution
unverified node rewrites recipient during payout
```

Allowed:

```text
service evidence references service_node_id
registry maps service_node_id → bound recipient account
wallet executes payout to canonical account id
ledger records receipt
```

## 3.5 Reward recipient rotation

Changing a reward address must be explicit and delayed.

Rotation flow:

```text
operator requests new @ address
new account signs rotation
node signs rotation
old account signs if available
rotation enters pending state
challenge/cooldown window opens
rotation effective at future epoch
old unpaid epochs remain payable to old binding unless policy says otherwise
new epochs pay to new binding
```

Tests:

```text
valid reward binding accepted
unbound service node cannot receive ROC
reward evidence cannot override payout recipient
reward plan resolves recipient through registry
recipient rotation requires signed request
recipient rotation has effective_epoch
duplicate rotation nonce rejected
invalid @ address rejected
deleted/disabled account cannot receive payout
ledger receipt goes to bound account id
```

---

# 4. ROC Reward Safety and Non-Investment Posture

ROC rewards are internal utility/accounting rewards inside the RustyOnions/CrabLink system.

This build plan does not authorize:

```text
public sale of ROC
cash redemption
staking yield
liquidity pools
exchange listings
external settlement
investment promises
profit guarantees
ROX/Solana runtime
public bridge settlement
```

Node rewards mean:

```text
internal ROC credited to a bound CrabLink/RON account
after useful service evidence
through the accounting/rewarder/policy/wallet/ledger path
```

Node rewards do not mean:

```text
financial return
investment income
staking return
real-world redemption
exchange-facing reward
public token distribution
```

---

# 5. First-Run Admin and Bootstrap UX

## 5.1 Problem

The current local login flow expects a bootstrap admin password environment variable or an existing RBAC store.

That is acceptable for advanced RON-CORE operators, but not ideal for service-node operators.

## 5.2 New service-node default

Default service-node admin posture:

```text
admin UI disabled or loopback-only
CLI always available
first-run setup token instead of env-only bootstrap
local auth after setup
no public admin exposure
```

## 5.3 First-run setup token

When no admin user exists:

```text
service node starts
→ detects missing local admin
→ prints one-time setup URL/code to local console
→ operator opens loopback setup page
→ operator creates admin password
→ setup token is burned
→ future access uses local auth
```

Example:

```text
CrabLink Service Node is running.

Admin setup:
  URL: http://127.0.0.1:5300/setup
  Code: 8H4K-2VQD-N9P7

This code expires in 10 minutes.
Admin UI is bound to loopback only.
```

CLI equivalents:

```bash
crabnode admin setup-token
crabnode admin create-user admin
crabnode admin disable-web
crabnode admin enable-web --bind 127.0.0.1:5300
```

## 5.4 Auth modes

RustyOnions default:

```text
local auth
httpOnly cookie session
loopback-only
short session TTL
read-only by default until operator enables actions
```

RON-CORE/cloud advanced:

```text
ingress auth
passport/JWT auth
SSO/reverse-proxy auth
public web deployment
```

Advanced modes remain allowed but are not the default service-node profile.

---

# 6. CLI-First, UI-Optional Operator Model

## 6.1 Required CLI

The service-node operator must be able to manage the node without a browser.

Required CLI:

```bash
crabnode status
crabnode ready
crabnode version
crabnode logs tail
crabnode metrics summary

crabnode identity init
crabnode identity show
crabnode rewards bind @operator
crabnode rewards show
crabnode rewards rotate @new-operator

crabnode policy sync
crabnode policy status
crabnode block b3:<hash> --reason local-operator-block
crabnode unblock b3:<hash>
crabnode prune b3:<hash>
crabnode quarantine b3:<hash> --reason suspicious
crabnode allow b3:<hash>
crabnode persistence register b3:<hash> <asset-kind>
crabnode persistence status b3:<hash>
crabnode persistence pending <limit>
crabnode persistence submit b3:<hash>
crabnode persistence approve b3:<hash>
crabnode persistence reject b3:<hash>
crabnode persistence pin b3:<hash>
crabnode persistence unpin b3:<hash>

crabnode admin setup-token
crabnode admin enable-web
crabnode admin disable-web
```

## 6.2 Optional admin UI

The UI exists for workflows that are painful in CLI:

```text
visual content review
moderation queue
multi-mod approvals
bulk pruning
persistence review
blocklist/allowlist management
reward recipient setup
reward status review
quarantine review
```

Admin UI deployment modes:

```text
disabled
loopback-only on-demand
LAN-only explicit operator mode
CrabLink embedded controller
RON-CORE full dashboard profile
```

Default:

```text
disabled or loopback-only on-demand
```

## 6.3 CrabLink as optional controller

CrabLink may manage a service node, but should not be required for service-node runtime.

Allowed:

```text
CrabLink → Node Operator Mode
CrabLink manages local or remote service node through authenticated API
CrabLink shows moderation/reward/health UI
CrabLink signs reward recipient binding
```

Forbidden:

```text
service node requires CrabLink desktop to run
service node wallet/quorum authority fused into CrabLink UI
CrabLink silently mutates node policy
CrabLink shows fake confirmed ROC
```

---

# 7. Service-Node Storage and Persistence Policy

## 7.1 Storage doctrine

Content liability and economic truth are separate surfaces.

```text
asset/content bytes:
  amnesia-first unless vetted

ledger/receipt/audit/policy data:
  may be durable when required for economic truth, replay, and audit
```

## 7.2 Content storage states

```text
ephemeral_unvetted
  default for unknown asset/content bytes
  RAM/cache only
  evicted freely
  not a long-term hosting commitment

pending_review
  candidate for persistence
  waiting on local/mod/community verification

verified_persistent
  eligible for future durable storage
  requires operator policy approval or multi-mod threshold
  does not prove durable bytes were written

operator_blocked
  local operator refuses to host it

global_denied
  blocked by signed global/TOS denylist

owner_tombstoned
  creator/owner has pruned/deleted it

quarantined
  suspicious/disputed; not served while under review

pinned_by_operator
  explicitly pinned by local operator after policy checks
  expresses retention intent; does not prove durable bytes were written
```

## 7.3 Default persistence mode

Default service node:

```text
unvetted assets: RAM/cache only
verified assets: eligible for future durable storage only after approval
ledger/receipt/audit/policy data: durable if required
logs/caches: bounded and redacted
```

## 7.4 Persistence approval

A content item may become `verified_persistent` only if:

```text
b3 digest verifies
not globally denied
not locally blocked
not owner-tombstoned
not quarantined
operator persistence policy allows type/category
moderation threshold satisfied
```

Suggested initial threshold:

```text
operator manual approval
or 2-of-3 moderator approval
or approved community policy proof
```

## 7.5 Persistence modes

Operators may choose:

```text
ram_only
persist_own_content_only
persist_verified_assets
persist_selected_categories
archive_pin_mode_future
```

Initial default:

```text
ram_only for unvetted content
persist_verified_assets only after explicit enable
```

---

# 8. Moderation, Pruning, Blocklists, and Tombstones

## 8.1 Policy layers

```text
global signed denylist
community/moderator list
local operator blocklist
local operator allowlist
owner tombstone/prune list
temporary quarantine list
operator pinlist
```

## 8.2 Addressing

Moderation records may target:

```text
b3:<hash>
manifest hash
site id
community id
creator id
asset type
policy reason code
```

## 8.3 Serving path

```text
request b3
→ check global denylist
→ check owner tombstone
→ check local operator blocklist
→ check quarantine
→ check local allow/pin policy
→ check content availability
→ verify b3 digest
→ serve only if allowed
```

## 8.4 Prune semantics

Pruning can mean:

```text
remove local cached bytes
withdraw provider record once the real Phase 12 provider model exists
add local block entry
honor owner tombstone
stop serving
emit policy refusal evidence if requested later
```

Pruning must not mean:

```text
delete global history
erase ledger/audit truth
mutate wallet/ledger
claim reward
prove settlement
```

## 8.5 Moderation events

Future DTO:

```text
NodeModerationEventV1
  event_id
  node_id
  operator_account_id
  action
  target_kind
  target_hash_or_id
  reason_code
  notes_hash optional
  created_at
  policy_version
  signature
```

Actions:

```text
block
unblock
quarantine
unquarantine
prune
allow
pin
approve_persistence
reject_persistence
```

Tests:

```text
blocked b3 is not served
globally denied b3 is not served
owner tombstone stops local serving immediately
provider withdrawal is tested in Phase 12 once the real provider model exists
quarantined content is not served
operator prune removes local bytes
operator prune does not mutate ledger
denied serving does not earn ROC
serving denied content flags bad node
moderation event signatures verify
```

---

# 9. IP Privacy and Network Privacy

## 9.1 User-node privacy

User nodes must be:

```text
outbound-only by default
loopback-admin-only
not public DHT providers by residential IP
not public OAP servers
not direct P2P listeners
not LAN-visible admin servers
```

They must not publish:

```text
residential IP
LAN IP
raw socket address
port mapping
ISP metadata
geolocation metadata
```

They may publish:

```text
node_id
capability class
privacy_route_id
crab_node_id
epoch participation proof
verification capability statement
```

## 9.2 Service-node public posture

Service nodes may publish:

```text
public service descriptor
crab://node/<node-id>
RustyOnions route descriptor
provider records
OAP endpoint descriptor
capabilities
policy version
node reputation state
```

## 9.3 User-to-content path

Bad:

```text
User A fetches directly from User B residential IP.
```

Correct default:

```text
User A
→ Service Node / relay / overlay route
→ OAP b3 fetch
→ local b3 verification
```

Strong privacy:

```text
User A
→ RustyOnions privacy route
→ Service Node
→ OAP b3 fetch
→ local b3 verification
```

## 9.4 DTO/log privacy

Must not appear in public/user-visible DTOs:

```text
source_ip
viewer_ip
creator_ip
residential_ip
lan_ip
socket_addr
geo_from_ip
```

Allowed:

```text
node_id
relay_id
service_node_id
privacy_route_id
request_id
nonce
capability_id
epoch_id
content_hash
receipt_hash
```

---

# 10. Decentralized Issuance Doctrine

## 10.1 No node mints alone

```text
No single node can mint ROC.
```

ROC issuance must be:

```text
epoch-based
evidence-backed
reward-pool-capped
policy-validated
quorum-accepted
ledger-conserved
user-node-replayable
recipient-binding-resolved
```

## 10.2 Correct issuance flow

```text
user verification evidence
+ service delivery evidence
+ availability evidence
+ repair evidence
+ accepted challenge evidence

→ ron-accounting snapshot
→ svc-rewarder capped reward plan
→ ron-policy validates eligibility/caps/config
→ svc-registry resolves service_node_id to reward_recipient_account_id
→ service-node quorum accepts epoch transition
→ svc-wallet executes authorized mutation path
→ ron-ledger records receipts/state
→ user nodes replay and verify
→ CrabLink displays confirmed ROC to recipient @ account
```

## 10.3 Forbidden issuance flow

```text
service node says "I served content" → service node pays itself
service evidence includes arbitrary payout address → wallet pays it
node operator changes reward address silently → old rewards redirected
rewarder creates plan → balance changes without wallet/ledger
policy allows claim → receipt appears
single node signs epoch → mint happens
service node issues or approves payout to its own bound reward recipient without independent quorum
CrabLink pending points → confirmed ROC display
```

## 10.4 Epoch transition object

Future DTO:

```text
RocEpochTransitionV1
  epoch_id
  previous_ledger_root
  accounting_snapshot_root
  reward_plan_root
  policy_hash
  economics_config_hash
  registry_root
  reward_binding_root
  authorized_reward_pool_minor
  total_planned_payout_minor
  burns_minor
  fees_minor
  sinks_minor
  resulting_ledger_root
  service_node_quorum
  quorum_signatures
  challenge_window
  created_at
```

Validation rules:

```text
total_planned_payout_minor <= authorized_reward_pool_minor
previous state + accepted operations = resulting state
all payouts reference evidence roots
all service-node payouts resolve through reward binding root
all evidence references accepted event classes
policy hash matches accepted policy bundle
economics config hash matches accepted config
registry root matches accepted node registry
quorum signatures meet threshold
no duplicate operation_id
no duplicate idempotency_key
no unauthorized issue operation
```

---

# 11. Crate Ownership Map

## 11.1 CrabLink Tauri

Owns:

```text
normal user UX
user intent
simple user-node status
background user-node lifecycle
privacy mode UX
resource mode controls
confirmed ROC display
pending evidence display
fraud/challenge status display
optional service-node management view
reward recipient signing flow for node operators
```

Does not own:

```text
ledger truth
wallet mutation
reward planning
policy authority
quorum signing
direct payout execution
epoch acceptance
service-node daemon runtime requirement
```

## 11.2 crabnode CLI

Owns:

```text
service-node setup
service-node identity commands
reward recipient binding
admin setup token
headless status
policy sync
block/prune/quarantine/allow commands
persistence approval/rejection
web UI enable/disable
operator diagnostics
```

## 11.3 micronode

Owns:

```text
CrabLink User Node runtime
private passive node behavior
loopback admin/health/status
amnesia-first operation
privacy-first network posture
light OAP object fetch/cache
verification work queue
economic replay/audit worker
verification evidence generation
fraud challenge generation
resource budgeting
```

## 11.4 macronode

Owns:

```text
CrabLink Service Node runtime
headless service daemon composition
OAP/content serving service composition
gateway/storage/index/mailbox/overlay/dht integration
operator persistence/amnesia mode
policy/denylist/tombstone enforcement
provider advertisement
service-node reward evidence emission
service-node quorum participation wrapper
service-node status/readiness
```

## 11.5 svc-admin

Owns:

```text
optional service-node operator console
local setup/bootstrap UI
operator dashboard
moderation/pruning UI
persistence review UI
policy list status
reward recipient status
quorum participation status
epoch transition status
node health/status cards
developer diagnostics
```

Does not own:

```text
node runtime requirement
wallet/ledger truth
direct payout authority
mandatory user UX
cloud dashboard default
```

## 11.6 Economic crates

```text
ron-accounting:
  snapshots/reports only
  service evidence classification
  user verification evidence classification
  no balance mutation

svc-rewarder:
  capped deterministic planning only
  no ledger mutation
  references service_node_id, not arbitrary payout address

ron-policy:
  declarative gates only
  validates eligibility/caps/config/policy
  validates persistence/moderation policy
  no ledger mutation

svc-wallet:
  mutation front-door code path
  executes only approved/quorum transitions
  pays canonical account id resolved from registry/binding
  no unilateral local mint

ron-ledger:
  durable balance/receipt truth
  replay/conservation state machine
  user nodes verify read-only replay

svc-registry:
  service-node descriptors
  reward recipient bindings
  eligibility/probation/quarantine state
  registry roots for epochs

ron-audit:
  audit trail/proofs
  moderation events
  challenge records
  epoch evidence records
```

---

# 12. Global Invariants

```text
Two public node products only:
  CrabLink User Node
  CrabLink Service Node

Service node runtime is headless-first.
CLI is required.
Admin UI is optional.
CrabLink service-node management is optional.

No node mints alone.
No node self-pays.
No arbitrary payout address in service evidence.
No raw engagement directly mints ROC.
No fake balances.
No fake receipts.
No fake finality.

Service-node rewards require:
  service_node_id
  accepted evidence
  registry identity
  bound reward recipient account
  reward plan
  policy validation
  quorum acceptance
  wallet execution
  ledger receipt

User nodes remain private/outbound by default.
Users cannot see other users' residential IPs.
Unvetted content is amnesia-first.
Persistent asset hosting requires policy/moderation approval.
Ledger/receipt/audit data may be durable where required.
```

---


## 12.1 Centralized ROC Economics Configuration

### Core rule

All mutable ROC economics values must be stored in reviewed TOML
configuration files under:

```text
configs/
```

Mutable ROC prices, reward rates, pool limits, category caps, account
limits, content limits, split percentages, anti-farming limits,
rounding rules, and remainder destinations must not be scattered as
Rust constants across the workspace.

Canonical configuration paths:

```text
configs/roc-economics.toml
  canonical non-development ROC economics configuration

configs/roc-economics.dev.toml
  explicit development/private-beta ROC economics profile
```

The purpose of this rule is to ensure that ROC economics can be
reviewed, audited, tested, and changed from one centralized
configuration surface instead of requiring edits across many crates.

### Canonical non-development source

The canonical non-development economics source is:

```text
configs/roc-economics.toml
```

It owns the reviewed values used outside explicit development mode,
including applicable:

```text
ROC minor-unit definitions
minimum prices
paid-content splits
reward-pool caps
reward-category caps
per-account reward limits
per-content reward limits
anti-farming limits
rounding mode
remainder sink
future evidence reward rates
future payout ceilings
future epoch economics limits
```

Before evidence-backed rewards become active, all mutable reward rates
for evidence classes must be added to this canonical configuration
rather than hardcoded into implementation crates.

Expected future reward categories may include:

```text
delivery evidence
availability evidence
range-request evidence
repair evidence
hot-cache evidence
user verification evidence
fraud-challenge evidence
moderation evidence
policy-refusal evidence
```

The exact categories and values must follow the accepted evidence DTOs
and reward policy, but their mutable rates must remain configuration
data.

### Development/private-beta profile

The explicit development/private-beta economics profile is:

```text
configs/roc-economics.dev.toml
```

This profile may contain developer-friendly values for:

```text
paid storage puts
paid storage pins
paid content views
song plays
site visits
local spending caps
development split percentages
private-beta reward limits
```

The development profile must be selected explicitly through a
development configuration path or profile.

Example:

```text
RON_STORAGE_ROC_ECONOMICS_PATH=configs/roc-economics.dev.toml
```

The development profile must never be selected implicitly in a
non-development runtime.

### Values that must remain centralized

The following mutable values belong in the economics TOML
configuration and must not be duplicated as crate-local constants:

```text
action prices
minimum charges
maximum action spend
reward amount per accepted evidence class
reward amount per byte or range when applicable
reward-pool cap
reward-category cap
per-account epoch cap
per-content epoch cap
per-node epoch cap when introduced
event-count limits
anti-farming limits
split basis points
treasury percentage
reward-pool percentage
provider percentage
creator percentage
curator percentage
rounding behavior
remainder destination
hold multipliers
reward ceilings
development pricing
private-beta limits
```

A crate may define test fixtures with explicit local values, but those
fixtures must not become runtime defaults or duplicate the canonical
mutable economics source.

### Permitted hardcoded structural constants

Hardcoded constants are allowed only when they are protocol or schema
invariants rather than mutable economics policy.

Examples include:

```text
basis-point denominator = 10000
schema identifiers
version discriminants
integer minor-unit encoding rules
security-critical protocol maximums
serialization field names
```

A value must not be treated as a structural constant merely to avoid
placing it in the economics configuration.

### Configuration ownership

Crate responsibilities are:

```text
ron-policy:
  parse the economics configuration
  validate schema and version
  reject malformed or unsafe values
  validate basis-point totals
  validate caps and limits
  produce the normalized accepted economics model
  expose the canonical economics config hash
  perform no wallet or ledger mutation

ron-accounting:
  classify and aggregate accepted evidence
  reference the accepted economics configuration
  avoid embedding reward prices
  produce deterministic accounting snapshots
  perform no wallet or ledger mutation

svc-rewarder:
  consume the validated economics model
  calculate deterministic capped reward plans
  avoid crate-local mutable reward-rate constants
  bind every plan to economics_config_hash
  perform no wallet or ledger mutation

svc-registry:
  resolve service_node_id to the canonical bound reward recipient
  provide registry and reward-binding roots
  not define reward rates

svc-wallet:
  execute only an approved and quorum-authorized reward plan
  verify the accepted economics_config_hash
  not reinterpret or independently calculate reward rates

ron-ledger:
  record authoritative balances and receipts
  preserve the economics_config_hash associated with execution
  reject replay or mismatched approved operations
  not invent reward rates

macronode:
  emit service evidence
  expose configuration status where appropriate
  not calculate or hardcode authoritative reward rates

micronode:
  emit verification and challenge evidence
  replay accepted plans and transitions
  not calculate or hardcode authoritative reward rates

CrabLink and svc-admin:
  display configured or confirmed economics information
  not become the source of reward-rate truth
```

### Canonical hashing and deterministic binding

The accepted economics configuration must have a deterministic
canonical representation and hash.

Every deterministic reward plan must include or bind to:

```text
economics_config_hash
```

Every accepted ROC epoch transition must also include:

```text
economics_config_hash
```

The hash used by accounting, reward planning, policy validation,
quorum review, wallet execution, ledger recording, and user-node
replay must refer to the same validated canonical economics
configuration.

The following must fail closed:

```text
missing economics configuration
unreadable economics configuration
unsupported schema version
malformed integer minor-unit value
floating-point economics value
negative amount
invalid basis-point value
split basis points not totaling 10000
category caps exceeding the epoch pool
account/content/node caps exceeding allowed bounds
unknown reward category where strict matching is required
ambiguous configuration layering
economics_config_hash mismatch
development profile selected in non-development mode
reward plan produced without a config hash
epoch transition produced without a config hash
```

### Integer-only economics

ROC economics must use deterministic integer arithmetic.

Required posture:

```text
integer ROC minor units
integer basis points
explicit floor or other approved rounding mode
explicit remainder sink
no floating-point calculations
```

The configuration must not use floating-point reward or pricing values.

### Shared standalone economics schema decision

The economics configuration design is now fixed.

```text
configs/roc-economics.toml
  complete standalone canonical/non-development profile

configs/roc-economics.dev.toml
  complete standalone development/private-beta profile
```

Both files must conform to the same strict canonical schema.

The development file is not an overlay, patch, partial document, or
inheritance layer over the canonical file.

Exactly one complete economics document is selected, parsed,
validated, normalized, and hashed for a runtime.

Required loading model:

```text
select exactly one explicit economics file
→ parse through one shared ron-policy parser
→ validate through one shared schema and rule set
→ produce one normalized economics model
→ serialize that model canonically
→ calculate economics_config_hash with BLAKE3
→ bind accounting, reward plans, epoch transitions, wallet execution,
  ledger receipts, and user-node replay to that hash
```

Forbidden loading models:

```text
load canonical and then merge development values
inherit missing development fields from canonical
implicitly fall back from one profile to another
merge TOML tables or arrays at runtime
allow different crates to interpret the files independently
hash raw TOML file bytes as economic consensus truth
use comments, whitespace, or TOML key order as hash-significant input
```

### Required shared top-level identity

Both files must contain the same schema identity fields:

```toml
schema = "internal_roc.economics-config.v1"
version = 1
profile = "canonical"
```

The development document uses:

```toml
schema = "internal_roc.economics-config.v1"
version = 1
profile = "development"
```

Allowed profile values are initially:

```text
canonical
development
```

Unknown profile values must fail closed.

A development profile must never be selected implicitly in a
non-development runtime.

### One shared normalized model

`ron-policy` must own one shared parsed and validated economics model
used for both files.

The model must be capable of representing all required economics
surfaces, including:

```text
units
accounts
roles
limits
paid action pricing
paid action splits
reward pools
reward category caps
evidence reward rates
anti-farming limits
rounding mode
remainder sink
future disabled/inert feature declarations
```

Both files must contain every required section for the accepted schema.

A field that is required by the schema cannot be omitted from the
development profile on the assumption that it will be inherited from
the canonical profile.

Optional fields must have explicit, deterministic semantics in the
shared schema.

### Monetary representation

All ROC monetary values must use one consistent representation in both
profiles.

Required rule:

```text
ROC monetary values:
  non-negative decimal strings
  parsed into bounded integer minor-unit types

basis points and bounded counters:
  TOML integers
```

Examples:

```toml
price_minor = "25"
minimum_charge_minor = "25"
max_spend_minor = "50"
epoch_pool_cap_minor = "1000000"
bps = 7000
max_events_per_account_per_epoch = 1000
```

Forbidden:

```text
floating-point monetary values
negative monetary values
mixed integer/string monetary representations between profiles
scientific notation
locale-dependent number formatting
implicit conversion from ROC major units
```

### Canonical hashing rule

`economics_config_hash` must be calculated from the validated,
normalized economics model rather than from raw TOML bytes.

Required hash pipeline:

```text
selected TOML file
→ strict shared schema
→ validated Rust economics model
→ deterministic canonical serialization
→ BLAKE3 digest
→ economics_config_hash
```

This ensures:

```text
comments do not affect the hash
whitespace does not affect the hash
TOML key order does not affect the hash
equivalent validated models produce the same hash
any accepted price, rate, cap, split, limit, profile, or rule change
produces a different hash
```

Canonical serialization must have:

```text
stable field ordering
stable collection ordering
stable enum labels
stable integer encoding
no map-order nondeterminism
no floating-point values
explicit schema, version, and profile
```

### Profile-selection rule

The normal non-development path is:

```text
configs/roc-economics.toml
profile = "canonical"
```

The explicit development/private-beta path is:

```text
configs/roc-economics.dev.toml
profile = "development"
```

Selection must occur through an explicit runtime profile or config
path.

Example:

```text
RON_STORAGE_ROC_ECONOMICS_PATH=configs/roc-economics.dev.toml
```

There must be no runtime behavior equivalent to:

```text
try development config
fall back to canonical config
merge missing development fields from canonical
select development merely because the file exists
```

### Migration requirement for the existing files

The existing economics files currently contain different field
layouts.

Before Phase 14 reward planning is complete, they must be migrated into
one shared superset schema.

The shared schema must preserve the intended configuration surfaces
currently represented across both files, including:

```text
canonical units and minor-unit rules
canonical paid-content behavior
development action pricing
development action splits
account and role identifiers
spending limits
reward-pool caps
reward-category caps
anti-farming limits
rounding and remainder handling
disabled future bridge and staking declarations
future evidence reward rates
```

Migration must not silently discard an existing field or reinterpret
its meaning.

### Crate ownership

```text
ron-policy:
  owns the one strict schema
  owns parsing
  owns validation
  owns normalization
  owns canonical serialization
  owns economics_config_hash generation

ron-accounting:
  consumes the validated normalized model
  never parses an alternate economics shape

svc-rewarder:
  consumes the same validated normalized model
  never owns a second reward-rate table

svc-wallet:
  verifies the accepted economics_config_hash
  does not recalculate rates independently

ron-ledger:
  records the accepted economics_config_hash
  does not interpret raw TOML independently

macronode and micronode:
  emit evidence and replay results
  do not own mutable reward rates

CrabLink and svc-admin:
  display selected profile and confirmed results
  do not become economics-config authority
```


### Required tests

Focused economics configuration tests must prove:

```text
canonical profile parses through the shared schema
development profile parses through the same shared schema
both profiles produce the same normalized Rust model type
both profiles contain all required schema sections
development profile does not inherit canonical values
overlay or merge behavior does not exist
unknown profile values fail closed
missing config fails closed
malformed config fails closed
unsupported version fails closed
floating-point economics values are rejected
negative values are rejected
invalid integer strings are rejected
split basis points must total 10000
category caps cannot exceed the epoch pool cap
per-account caps are enforced
per-content caps are enforced
per-node caps are enforced when introduced
anti-farming event limits are enforced
rounding mode is deterministic
remainder sink is explicit
raw comments and whitespace do not change economics_config_hash
TOML key ordering does not change economics_config_hash
equivalent normalized models produce the same economics_config_hash
changing any accepted rate, cap, split, limit, profile, or rule changes the hash
same config plus same evidence produces the same reward plan
changing a configured rate changes the plan deterministically
reward plan includes the canonical economics_config_hash
epoch transition includes the canonical economics_config_hash
wallet rejects an economics_config_hash mismatch
ledger receipt preserves the accepted config hash
user-node replay rejects a config-hash mismatch
development config is never selected implicitly
development config is never merged with canonical config
raw TOML bytes are not used directly as economics consensus truth
runtime reward rates are not duplicated across implementation crates
```

### Economic authority boundary

The existence of an economics configuration does not itself:

```text
mint ROC
transfer ROC
burn ROC
unlock ROC
stake ROC
bridge ROC
settle an epoch
create a wallet balance
create a ledger receipt
prove payout
prove finality
```

The economics configuration is declarative policy input only.

The authoritative execution path remains:

```text
accepted evidence
→ ron-accounting snapshot
→ svc-rewarder deterministic capped plan
→ ron-policy validation
→ svc-registry reward-recipient resolution
→ service-node quorum acceptance
→ svc-wallet execution
→ ron-ledger durable receipt
→ user-node replay and challenge
```

### Exit invariant

The centralized economics requirement is satisfied only when:

```text
all mutable ROC rates are represented in reviewed configs/ TOML
both economics files use one strict shared schema
both economics files are complete standalone profiles
no overlay, inheritance, implicit fallback, or runtime merge exists
no runtime crate owns a duplicate mutable reward-rate table
the selected profile is explicit
the configuration validates fail closed
reward plans bind to economics_config_hash
epoch transitions bind to economics_config_hash
wallet and ledger verify the same accepted hash
changing a rate requires changing the reviewed TOML source,
not searching across the repository for scattered constants
```


# 13. Build Phases

## Phase 0 — BUILD_PLAN_Z Landing and Scope Lock

### Goal

Land this updated plan as the controlling document.

### Work

```text
Create or replace BUILD_PLAN_Z.md.
Record two-node doctrine.
Record service-node operator posture reset.
Record CLI-first / UI-optional model.
Record reward recipient @ address binding.
Record moderation/pruning/persistence policy.
Record no unilateral mint and no node self-pay invariants.
```

### Checks

```bash
cargo check --workspace
```

### Exit Gate

```text
BUILD_PLAN_Z exists.
Two public node products only.
svc-admin is optional operator console, not required runtime.
Node operator rewards require bound account.
Persistence and moderation policy are explicit.
```

---

## Phase 1 — Workspace Baseline and First Failure Fix

### Goal

Reconfirm current green baseline.

### Work

```bash
cargo check --workspace
cargo check -p micronode
cargo check -p macronode
cargo check -p svc-admin
cargo test -p micronode
cargo test -p macronode
cargo test -p svc-admin
```

### Exit Gate

```text
Workspace either passes or has one documented first failure.
No broad expansion before first failure is fixed.
```

---

## Phase 2 — Two-Node Status Truth Contract

Status:

```text
GREEN / PARKED
```

### Proven

```text
micronode reports user_node
macronode reports service_node
privacy/amnesia/public inbound fields report truthfully
svc-admin auth-protected registry behavior is accepted
micronode amnesia metric is truthful
```

### Regression Checks

```bash
cargo test -p micronode --test admin_parity
cargo test -p svc-admin --test fake_node
cargo test -p macronode --test admin_smoke
bash scripts/check_two_node_admin_truth.sh
```

### Exit Gate

```text
No fake node role.
No public economic_node role.
No fake quorum status.
No user IP publication by user node.
```

---

## Phase 3 — Admin Posture Reset: Headless-First Service Node

### Goal

Make service-node runtime independent of the web dashboard.

### Work

```text
Define service-node daemon as required runtime.
Define crabnode CLI as required operator surface.
Define svc-admin UI as optional/on-demand.
Ensure npm/Vite/dev UI is never required at runtime.
Ensure service node can run on Raspberry Pi/headless Linux.
Ensure admin UI can be disabled entirely.
```

### Work in macronode

```text
add service-node profile config
ensure daemon runs without svc-admin UI
ensure status/metrics remain available
ensure operator actions are API/CLI compatible
```

### Work in svc-admin

```text
rename posture in UI copy away from RON-CORE cloud dashboard by default
add service-node operator profile
hide cloud/SSO/advanced panels unless RON_CORE_PROFILE=cloud
make local service-node admin mode explicit
```

### Work in crabnode CLI

```text
add command skeleton:
  crabnode status
  crabnode admin enable-web
  crabnode admin disable-web
  crabnode admin setup-token
```

### Tests

```text
service node starts with admin UI disabled
service node starts with admin UI loopback-only
CLI can query status without UI
UI build not required for daemon start
service-node status remains truthful without UI
```

### Exit Gate

```text
Node operator does not need full web dashboard.
Node can run headless.
CLI can manage minimum operations.
```

---

## Phase 4 — First-Run Admin Bootstrap UX

### Goal

Replace env-only bootstrap with a local one-time setup flow.

### Work in svc-admin

```text
detect no local admin user exists
generate setup token
print setup URL/code to local console
serve /setup only on loopback by default
create admin password from setup flow
burn setup token after use
expire setup token after short TTL
keep env bootstrap as advanced fallback
```

### Work in crabnode CLI

```text
crabnode admin setup-token
crabnode admin create-user admin
crabnode admin disable-web
crabnode admin enable-web

No reset-password flow exists in the service-node default.
If local admin credentials are lost, the operator recreates the node-local admin state.
Durable ROC payout identity remains external through the bound CrabLink/RON @ account.
```

### Tests

```text
no admin user creates setup-needed state
setup token can create first admin
setup token expires
setup token single-use
invalid setup token rejected
login works after setup
env bootstrap still works as advanced path
anonymous /api/nodes remains protected in local auth mode
password reset is not advertised as a default service-node recovery path
```

### Exit Gate

```text
Fresh service-node operator can bootstrap admin locally without knowing env var internals.
Admin UI remains protected.
```

---

## Phase 5 — Node Operator Reward Recipient Binding

### Goal

Allow service-node operators to receive ROC through a bound CrabLink/RON account.

### Work in ron-proto

```text
ServiceNodeRewardBindingV1
ServiceNodeRewardBindingRotationV1
RewardRecipientResolutionV1
NodeRewardRecipientStatusV1
```

### Work in svc-registry

```text
store service-node descriptor
store reward recipient binding
resolve service_node_id to reward_recipient_account_id
support binding rotation
expose registry root for epoch transitions
```

### Work in svc-passport / ron-auth

```text
prove ownership of @ address/account
sign reward binding
capability for node reward binding
capability for binding rotation
```

### Work in crabnode CLI

```text
crabnode identity init
crabnode rewards bind @operator
crabnode rewards show
crabnode rewards rotate @new-operator
```

### Work in svc-admin / CrabLink

```text
show reward recipient @ address
show binding status
support setup wizard binding
allow CrabLink to sign binding in operator mode
show confirmed rewards only after ledger receipt
```

### Tests

```text
valid @ address binding accepted
unknown @ address rejected
unbound node cannot receive ROC
node evidence cannot include arbitrary payout address
reward plan resolves recipient through registry
recipient rotation requires signature
rotation effective only at future epoch
duplicate rotation nonce rejected
ledger payout goes to bound account id
```

### Exit Gate

```text
Service-node operator can receive ROC through a bound account.
No service evidence can redirect payout.
No node self-pay shortcut exists.
```

---

## Phase 6 — User Node as Passive CrabLink-Managed Runtime

### Goal

Make the User Node a CrabLink-managed passive verifier.

### Work

```text
CrabLink starts/stops/restarts micronode.
Micronode binds loopback only.
Public inbound disabled.
Verification queue enabled.
Economic replay worker stub enabled.
Resource budgets enforced.
CrabLink shows privacy/status/pending evidence/confirmed ROC.
```

### Tests

```text
CrabLink starts user node
CrabLink stops user node
node crash shows degraded state
no raw admin UI for normal user
no public inbound
no peer IP display
```

### Exit Gate

```text
Normal user can passively mine/verify without admin burden.
```

---

## Phase 7 — IP Privacy and No Direct User-to-User Exposure

### Goal

Prevent users from seeing each other’s residential IPs.

### Work

```text
user node outbound-only
provider records reject residential user IP
DTOs/logs/receipts omit IP fields
crab://node provider identities supported
CrabLink never displays peer IPs
```

### Tests

```text
user node binds loopback
user node cannot publish residential provider record
receipt/proof/evidence DTOs contain no IP fields
direct user-to-user TCP path disabled by default
crab://node provider identity works
```

### Exit Gate

```text
CrabLink users cannot see other users' residential IPs through normal operation.
```

---

## Phase 8 — Service Node Foundation

### Goal

Make macronode run as a headless CrabLink Service Node.

### Work

```text
node_role=service_node
wire storage/index/OAP readiness
wire provider status
wire policy status
wire admin UI disabled/on-demand mode
wire reward binding status
quorum parked until later phase
```

### Tests

```text
service node starts headless
service node reports service_node
service node serves seeded object when enabled
admin UI off does not break daemon
reward binding status reports unbound/bound truthfully
```

### Exit Gate

```text
Service node can run as real headless infrastructure.
```

---

## Phase 9 — OAP/1 b3 Object Fetch and Verification

### Goal

Enable b3-addressed content fetch/serve over OAP/1.

### Work

```text
OBJ_GET/object stream flow
1 MiB frame limit
64 KiB chunk limit
full BLAKE3 digest verification
policy check before serve
privacy-aware fetch path
```

### Tests

```text
happy fetch
corrupt bytes rejected
wrong b3 rejected
oversized frame rejected
oversized chunk rejected
not found/not ready behavior
```

### Exit Gate

```text
OAP b3 object fetch works locally with mandatory digest verification.
```

---

## Phase 10 — Operator Policy, Blocklists, Tombstones, and Pruning

### Goal

Make operator moderation and pruning first-class.

### Work in ron-policy

```text
policy decision model
global denylist
local blocklist
local allowlist
owner tombstone
quarantine
reason codes
signed policy list verification
```

### Work in macronode

```text
check policy before serve
reject denied or tombstoned content
prune exact local runtime surfaces truthfully
report provider withdrawal unavailable until the Phase 12 provider model exists
defer signed policy-refusal evidence to Phase 13
preserve no-wallet/no-ledger authority
```

### Work in crabnode CLI

```text
crabnode block b3:<hash>
crabnode unblock b3:<hash>
crabnode prune b3:<hash>
crabnode quarantine b3:<hash>
crabnode allow b3:<hash>
```

### Optional svc-admin follow-up

The Phase 10 headless runtime, HTTP, and `crabnode` controls are the
authoritative moderation surfaces.

The optional visual moderation console is deferred to Phase 20. Its
future responsibilities include:

```text
moderation queue
hash detail view
block/prune/quarantine buttons
reason code selection
policy list freshness
operator local rules
```

### Tests

```text
globally denied hash not served
local block not served
owner tombstone honored
prune removes exact local bytes and indexes
repeated prune reports truthful already-absent state
missing provider infrastructure does not produce fake withdrawal success
denied serving remains ineligible for later reward evidence
prune does not mutate wallet or ledger state
policy false positive requires exact lookup
```

### Exit Gate

```text
Operators can control what they host through the headless runtime,
operator HTTP surface, and crabnode CLI.
The optional visual operator UI remains deferred to Phase 20.
```

---

## Phase 11 — Persistence Policy and Verified Asset Storage

### Goal

Ensure unvetted content is amnesia-first and only verified/mod-approved assets persist.

### Work in svc-storage

```text
storage state machine:
  ephemeral_unvetted
  pending_review
  verified_persistent
  operator_blocked
  global_denied
  owner_tombstoned
  quarantined
  pinned_by_operator
```

### Work in ron-policy

```text
persistence eligibility policy
moderation threshold policy
asset type/category policy
```

### Work in macronode / crabnode CLI

```text
list pending persistence
approve persistence
reject persistence
pin asset
unpin asset
show persistence state
share one real runtime catalog across separate crabnode processes
retain the exact effective moderation policy
fail closed when moderation is unavailable, failed, expired, or inactive
```

The optional `svc-admin` persistence-review UI is deferred to Phase 20.
Phase 11 requires the real headless HTTP and `crabnode` CLI surfaces.

### Tests

```text
unvetted assets are bounded and amnesia-first
new candidates begin ephemeral_unvetted
only verified_persistent and pinned_by_operator are durable-storage eligible
approval and pinning require canonical policy
pinning cannot skip approval
unpinning preserves verified eligibility
operator_blocked cannot become durable eligible
global_denied cannot become durable eligible
owner_tombstoned cannot become durable eligible
quarantined cannot become durable eligible
unsafe moderation postures fail closed
repeated operations report truthful no-op state
durableBytesWritten remains false
walletMutation remains false
ledgerMutation remains false
```

### Exit Gate

```text
Service nodes do not persist random unvetted assets by default.
Persistence eligibility is deliberate, bounded, moderation-aware, and policy-backed.
Phase 11 does not claim that a durable-byte backend has been implemented.
```

---

## Phase 12 — Provider Discovery, Redundancy, and Repair

### Goal

Ensure content has multiple compliant providers and can recover from churn.

### Work

```text
provider record model
provider expiration
provider reputation/status hints
provider withdrawal on prune/delete
repair requests
privacy-compatible provider selection
```

### Tests

```text
provider records expire
fetch tries alternate provider
repair candidate created below target
tombstoned content does not repair
user residential IP never appears as provider address
```

### Exit Gate

```text
Network recovers from service-node churn without violating policy/privacy.
```

---

## Phase 13 — Delivery, Availability, and Policy Refusal Evidence

### Goal

Generate useful-service evidence without fake claims.

### Evidence types

```text
DeliveryProofV1
AvailabilityProofV1
RangeRequestProofV1
RepairProofV1
HotCacheProofV1
PolicyRefusalProofV1
ModerationActionProofV1
```

### Rules

```text
provider-only claim rejected
self-traffic rejected/capped
duplicate proof rejected
corrupt delivery penalizes provider
policy refusal can be evidence but not direct payout truth
no requester IP in evidence
```

### Exit Gate

```text
Service nodes produce evidence, not reward truth.
```

---

## Phase 14 — Accounting Snapshots and Reward Plan Determinism

### Goal

Turn verified evidence into deterministic reward plans without payout authority.

### Work

```text
load one validated economics model through ron-policy
use one strict schema for canonical and development profiles
load exactly one complete standalone economics document
select configs/roc-economics.toml outside explicit development mode
select configs/roc-economics.dev.toml only through an explicit development profile
perform no overlay, inheritance, fallback, or runtime merge
normalize and canonically serialize the selected model
calculate economics_config_hash from the normalized model with BLAKE3
classify service evidence
classify user verification evidence
classify moderation/policy-refusal evidence carefully
create deterministic snapshot roots
consume mutable rates, caps, splits, and limits from the validated economics model
create deterministic capped reward plans
bind snapshots and reward plans to economics_config_hash
resolve service_node_id but do not mutate ledger
keep mutable reward rates out of ron-accounting and svc-rewarder constants
```

### Tests

```text
canonical and development profiles parse through one shared schema
both profiles are complete standalone documents
development config requires explicit development mode
overlay, inheritance, fallback, and merge behavior do not exist
missing or malformed economics config fails closed
normalized-model hashing ignores comments, whitespace, and key ordering
valid service evidence can become planned points
valid user verification can become planned points
raw engagement rejected
metering never directly pays
reward plan cannot mutate ledger
reward plan references service_node_id not arbitrary recipient
same config plus same evidence produces the same plan
changing a TOML rate changes the plan deterministically
pool, category, account, content, and anti-farming caps are enforced
reward plan includes the canonical economics_config_hash
no crate-local mutable reward table overrides the accepted config
```

### Exit Gate

```text
Reward plans are deterministic, capped, config-bound, and non-authoritative.
All mutable ROC rates come from the reviewed economics TOML configuration.
```

---

## Phase 15 — Service-Node Quorum and Epoch Transition Model

### Goal

Implement decentralized service-node acceptance for ROC epoch transitions.

### Work

```text
RocEpochTransitionV1
ServiceNodeQuorumV1
ServiceNodeSignatureV1
InvalidEpochChallengeV1
EpochEligibilityV1
EpochQuorumThresholdV1
policy_hash
economics_config_hash
registry_root
reward_binding_root
```

### Tests

```text
valid epoch reaches quorum
invalid reward root rejected
invalid policy hash rejected
missing economics_config_hash rejected
invalid economics_config_hash rejected
invalid registry root rejected
invalid reward binding root rejected
cap overflow rejected
duplicate payout rejected
single node cannot mint
manual trusted issuer flag does not exist
```

### Exit Gate

```text
Service nodes can participate in decentralized epoch transitions.
No single service node can mint.
```

---

## Phase 16 — svc-wallet and ron-ledger Quorum Execution

### Goal

Move approved epoch transitions through wallet/ledger while preserving recipient binding and replay.

### Work

```text
verify quorum signatures
verify policy_hash
verify economics_config_hash
verify registry root
verify reward binding root
resolve recipients
execute payout intents
record receipts
reject unilateral local mint
```

### Required operation fields

```text
operation_id
idempotency_key
epoch_id
source_pool
service_node_id optional
recipient_account_id
amount
policy_approval_hash
economics_config_hash
reward_plan_hash
accounting_snapshot_hash
registry_hash
reward_binding_hash
evidence_root
quorum_signature_set
```

### Tests

```text
valid quorum transition executes
single-node transition rejected
duplicate payout prevented
invalid recipient binding rejected
recipient mismatch rejected
policy hash mismatch rejected
economics config hash mismatch rejected
missing economics config hash rejected
ledger receipt preserves the accepted economics_config_hash
ledger receipt replay stable
balance conservation holds
```

### Exit Gate

```text
ROC rewards execute only through approved wallet/ledger path.
```

---

## Phase 17 — User-Node Epoch Replay and Fraud Challenges

### Goal

Make user nodes verify and challenge service-node economic transitions.

### Work

```text
verify quorum signatures
verify eligibility set
verify accounting root
verify reward plan root
verify policy_hash
verify economics_config_hash
verify registry root
verify reward binding root
verify cap bounds
verify ledger replay
verify supply conservation
submit InvalidEpochChallengeV1
```

### Exit Gate

```text
Regular user nodes can audit the economy and challenge invalid epochs.
```

---

## Phase 18 — Sybil Resistance and Protocol-Earned Eligibility

### Goal

Prevent attackers from spinning up many service nodes to control quorum.

### Work

```text
service-node identity descriptor
reward recipient binding
candidate/probation/eligible/degraded/quarantined/blocked states
service history root
challenge history
new-node caps
quorum weighting
requester/provider diversity rules
```

### Tests

```text
new node starts as candidate/probation
candidate cannot control quorum
probation rewards capped
service history promotes eligibility
many fresh nodes cannot exceed threshold
quarantined node cannot sign valid quorum
manual founder-approved trust flag does not exist
```

### Exit Gate

```text
Permissionless participation has objective protocol gates.
```

---

## Phase 19 — Bad Node Detection and Quarantine

### Goal

Contain malicious/unreliable nodes.

### Signals

```text
hash mismatch
denylist violation
tombstone violation
fake delivery proof
provider spam
replay attempts
self-traffic loops
challenge failure
invalid epoch proposal
invalid epoch signature
unilateral mint attempt
privacy leak
reward recipient binding abuse
```

### Tests

```text
bad hash provider quarantined
denylist violator blocked
privacy leak rejected
invalid epoch signer quarantined
binding abuse blocks rewards
blocked node earns no rewards
appeal status visible
```

### Exit Gate

```text
Bad nodes are contained and reward-denied.
```

---

## Phase 20 — Optional svc-admin Operator Console

### Goal

Make optional UI useful for moderation and rewards without requiring it for runtime.

### UI must show

```text
service-node role/profile/health/readiness
OAP status
amnesia/persistence mode
storage use
bandwidth use
provider records
policy freshness
blocked/pruned count
pending review queue
persistence approvals
reward recipient @ address
reward binding status
delivery evidence count
reward evidence count
pending reward plans
confirmed ledger receipts
quorum eligibility/status
quarantine status
privacy compliance
```

### UI must not show

```text
fake ROC
fake receipt
fake finality
service-node-only controls on user_node
cloud dashboard surfaces by default
public admin exposure as normal
```

### Exit Gate

```text
Operators have useful optional dashboard.
Service node remains CLI/headless operable.
```

---

## Phase 21 — CrabLink Operator Mode

### Goal

Allow CrabLink to optionally manage a service node without fusing app and daemon authority.

### Work

```text
Node Operator Mode
connect to local/remote service node
authenticate locally
show status
bind reward @ address
review moderation queue
approve/reject persistence
show confirmed ROC receipts
```

### Tests

```text
CrabLink can connect to local service node
CrabLink cannot silently mutate policy
CrabLink can sign reward binding
CrabLink displays confirmed rewards only after ledger receipt
CrabLink does not require service node to run
```

### Exit Gate

```text
CrabLink can be a friendly controller, not a required daemon container.
```

---

## Phase 22 — End-to-End Local Two-Node Devnet Smoke

### Scenario

```text
start CrabLink with User Node
start Service Node A/B/C or local quorum fixture
bind Service Node reward recipient to @operator
creator publishes image/site
creator source bundle created
service node stores/serves object over OAP
viewer fetches through privacy-safe path
viewer verifies b3
service node emits delivery proof
user node verifies/challenges sample
accounting snapshots evidence
rewarder creates capped plan
policy gates plan
registry resolves node recipient
service-node quorum accepts epoch
wallet executes payout
ledger records receipt
user node replays epoch
CrabLink shows confirmed ROC for @operator
svc-admin/crabnode show truthful status
```

### Required proof

```text
user node active
service node headless
optional UI not required
reward recipient bound
b3 verified
OAP path used
privacy path valid
evidence accepted
reward plan deterministic
quorum required
single node cannot mint
ledger receipt stable
confirmed ROC appears only after receipt
```

### Exit Gate

```text
Local devnet proves the full user-node/service-node/reward-recipient loop.
```

---

## Phase 23 — Hardening, Chaos, and Abuse Drills

### Chaos cases

```text
service node offline
admin UI disabled
admin setup token expired
user node paused
provider corrupt bytes
provider denylisted content
provider leaks IP
provider replays proof
stale DHT record
storage cache evicted
source bundle missing
tombstone arrives while serving
reward binding rotation mid-epoch
rewarder unavailable
wallet duplicate payout
ledger replay mismatch
single node tries to mint
Sybil nodes attempt quorum
invalid challenge submitted
privacy relay unavailable
```

### Required behavior

```text
reads degrade
writes fail closed where needed
economic paths fail closed
bad content rejected
bad service node quarantined
invalid epoch challenged
Sybil nodes capped/probationed
privacy leak rejected
reward recipient abuse rejected
no fake success
no fake payout
no fake finality
```

### Clippy gates

```bash
cargo clippy -p micronode --all-targets -- -D warnings
cargo clippy -p macronode --all-targets -- -D warnings
cargo clippy -p svc-admin --all-targets -- -D warnings
cargo clippy -p oap --all-targets -- -D warnings
cargo clippy -p ron-proto --all-targets -- -D warnings
cargo clippy -p ron-accounting --all-targets -- -D warnings
cargo clippy -p svc-rewarder --all-targets -- -D warnings
cargo clippy -p ron-policy --all-targets -- -D warnings
cargo clippy -p svc-wallet --all-targets -- -D warnings
cargo clippy -p ron-ledger --all-targets -- -D warnings
cargo clippy -p svc-registry --all-targets -- -D warnings
cargo clippy -p svc-dht --all-targets -- -D warnings
cargo clippy -p svc-overlay --all-targets -- -D warnings
cargo clippy -p ron-transport --all-targets -- -D warnings
cargo clippy --workspace --all-targets -- -D warnings
```

### Exit Gate

```text
Node stack fails safely under churn, abuse, privacy attacks, moderation attacks, and economic attacks.
```

---

## Phase 24 — Private Beta Readiness

### Required artifacts

```text
private beta node runbook
CrabLink User Node UX guide
CrabLink Service Node operator quickstart
crabnode CLI guide
optional admin UI guide
first-run setup guide
reward @ address binding guide
ROC mining/reward explanation
service-node quorum explanation
IP privacy explanation
moderation/pruning guide
persistence policy guide
policy/denylist operations guide
incident response guide
known limitations
```

### Constraints

```text
bounded ROC values
dev/private configs
no public ROX/Solana bridge runtime
no staking/liquidity/exchange-facing behavior
no manual trusted-node production doctrine
temporary lab/devnet caps allowed
aggressive telemetry/audit
easy emergency halt
privacy mode enabled by default
admin UI loopback-only unless explicitly changed
unvetted content amnesia-first
```

### Acceptance

```text
normal user installs CrabLink and passively verifies
normal user does not manage servers/admin
service operator runs headless node
service operator can use CLI only
optional UI supports moderation/pruning/persistence
service operator binds reward @ address
service node earns ROC only through evidence/quorum/wallet/ledger
single service node cannot mint
bad node can be blocked/quarantined
content can be served, verified, repaired, pruned
confirmed ROC appears only after ledger receipt
users cannot see other users' residential IPs
```

### Exit Gate

```text
Private beta candidate ready.
Two-node model intact.
Headless service-node operation works.
Node operator reward binding works.
IP privacy works.
Moderation/persistence controls work.
No founder-gatekeeper issuance model.
```

---

# 14. Final Success Definition

After completing this build plan:

```text
A normal user installs CrabLink.
CrabLink quietly runs a User Node in the background.
The user does not manage ports, logs, routes, RPC URLs, or admin dashboards.
The User Node verifies useful ROC/network/economic evidence.
The User Node submits bounded proof_eligible evidence.
The User Node can earn ROC only after accepted evidence and ledger-confirmed payout.
The User Node does not expose the user's residential IP to other users.

A service operator runs a CrabLink Service Node.
The Service Node runs headless on small hardware.
The Service Node is manageable by CLI.
The optional admin UI is local/on-demand and not required.
The Service Node serves b3/OAP content.
The Service Node honors denylist, tombstones, local moderation, and pruning.
The Service Node keeps unvetted assets amnesia-first.
The Service Node persists only verified/mod-approved assets by policy.
The operator binds a reward @ address / account.
The Service Node emits delivery/availability/repair evidence.
The Service Node participates in decentralized epoch/quorum issuance.
The Service Node cannot mint ROC alone.
The Service Node earns ROC only through accepted evidence, valid quorum transition, bound recipient resolution, and ledger receipt.

The network processes ROC issuance as an epoch transition.
Accounting snapshots evidence.
Rewarder creates deterministic capped plans.
Policy validates eligibility/caps/config.
Registry resolves service_node_id to reward recipient account.
Service-node quorum accepts valid transitions.
svc-wallet executes only approved/quorum transitions.
ron-ledger records durable receipts/state.
User nodes replay and challenge invalid transitions.
No raw engagement directly mints ROC.
No node self-pays.
No fake balances or fake receipts appear.
No permanent manual trusted-node gatekeeper exists.
```

Final doctrine:

```text
Users verify privately. Service nodes run headless, moderate locally, and earn through bound ledger receipts. No node mints alone.
```

---

# 15. Recommended Immediate Next Work

Current status:

```text
Phases 0 through 11: COMPLETE / GREEN / PARKED
Phase 12: NEXT
```

Current temporary repository rule:

```text
Do not run Git commands.
Do not create commits.
Do not push changes.
This restriction remains until the user explicitly lifts it.
```

Next milestone:

```text
Phase 12 — Provider Discovery, Redundancy, and Repair
```

Start with:

```bash
cd /Users/mymac/Desktop/RustyOnions
cargo check --workspace
```

If the workspace is green, inspect the current provider-related source before patching:

```bash
sed -n \
  '/## Phase 12 — Provider Discovery, Redundancy, and Repair/,/## Phase 13 —/p' \
  BUILD_PLAN_Z.md

grep -Rni \
  --exclude-dir=target \
  --exclude='*.lock' \
  -E 'provider record|provider_record|provider.*expire|withdraw.*provider|repair request|repair_candidate|alternate provider' \
  crates/svc-dht \
  crates/svc-storage \
  crates/macronode \
  crates/svc-index \
  crates/svc-overlay \
  crates/ron-proto \
  crates/ron-policy
```

Recommended first Phase 12 sequence:

```text
1. Locate or define one canonical provider-record model.
2. Keep provider identity distinct from user residential network identity.
3. Add deterministic provider expiration.
4. Add focused tests for active versus expired records.
5. Integrate truthful provider withdrawal only after the real record path exists.
6. Add alternate-provider selection with bounded retries.
7. Add repair-candidate creation below a configured redundancy target.
8. Suppress repair for tombstoned, denied, blocked, or quarantined content.
9. Prove that user residential IPs never appear as provider addresses.
```

Required honesty boundary:

```text
A provider record is not proof of successful delivery.
A repair candidate is not proof that repair completed.
A redundancy target is not proof that multiple providers exist.
Do not emit fake provider withdrawal, repair completion, or redundancy success.
```

Use the updated `ALLNOTES.MD`, current source, and fresh terminal output as supporting context. Current source remains authoritative.

---

# 16. BUILD_PLAN_Z Architectural Authority (Current)

This section supersedes any earlier architectural examples or historical implementation
notes contained elsewhere in this document.

The current RustyOnions / CrabLink network architecture is authoritative.

### URI Namespace

The canonical URI namespace for the network is:

    crab://

Examples include (but are not limited to):

    crab://node/<node-id>
    crab://creator/<creator-id>
    crab://community/<community-id>
    crab://passport/<passport-id>
    crab://asset/<b3-hash>
    crab://service/<service-id>

Additional transport-specific URI schemes are not part of the RustyOnions
architecture and should not be introduced.

### Routing

RustyOnions uses its own routing architecture.

Legacy references to Tor, Arti, onion services, hidden services, onion routing,
or transport-specific routing mechanisms should be interpreted as historical
prototype references only unless explicitly reintroduced by a future design
decision.

Current implementations should target the RustyOnions routing layer.

### Node Identity

Nodes are identified by canonical Crab node identities.

The canonical external representation is:

    crab://node/<node-id>

Internal implementations should prefer strongly typed node identifiers rather
than parsing URI strings throughout the networking stack.

### Design Principle

Applications communicate using CrabLink resources.

Networking components communicate using RustyOnions identities.

Transport implementations remain internal details and are intentionally hidden
from application-facing APIs.

# NEW: IP BLOCK RANGES FOR SANCTIONED COUNTRIES:

## Future Compliance Gate — Sanctioned-Jurisdiction IP Range Blocking

### Purpose

CrabLink and the RustyOnions node layer must support centrally managed network-access restrictions for countries, territories, regions, and IP address ranges subject to applicable sanctions or service restrictions.

This requirement applies to both:

* CrabLink user-facing applications and frontend/API access;
* RustyOnions node registration, admission, participation, reward eligibility, and runtime connectivity.

Blocking only the frontend is insufficient. The same validated compliance posture must be enforced server-side so a restricted client cannot bypass the user interface and directly access node, gateway, registry, reward, or protocol endpoints.

### Canonical Configuration

Use centralized configuration following the same ownership model as ROC economics:

* `configs/sanctions-compliance.toml` — canonical reviewed non-development policy;
* `configs/sanctions-compliance.dev.toml` — explicit development and test profile.

Do not hardcode sanctioned countries, territories, CIDR ranges, lookup behavior, or enforcement posture across CrabLink or RustyOnions crates.

The canonical configuration should include:

* schema and version;
* explicit profile identity;
* enablement posture;
* blocked ISO country codes;
* blocked territory or region identifiers;
* explicit IPv4 and IPv6 CIDR block lists;
* source name and source revision;
* effective and review timestamps;
* behavior when IP geolocation or range resolution fails;
* enforcement flags for each protected product boundary;
* optional exemptions reserved for reviewed local development fixtures.

### Example Configuration Shape

```toml
schema = "rustyonions.sanctions-compliance.v1"
version = 1
profile = "canonical"
enabled = true
default_action = "allow"
matched_range_action = "deny"
lookup_failure_posture = "fail_closed"

[enforcement]
crablink_frontend = true
public_api = true
gateway_ingress = true
node_registration = true
node_runtime = true
node_peer_admission = true
node_reward_eligibility = true

[jurisdictions]
blocked_country_codes = []
blocked_region_codes = []

[ip_ranges]
blocked_ipv4_cidrs = []
blocked_ipv6_cidrs = []

[metadata]
source_name = ""
source_revision = ""
effective_at_unix_s = 0
reviewed_at_unix_s = 0
```

The example contains no authoritative sanctions list. Production ranges and jurisdiction identifiers must come from a reviewed compliance source and must be updateable without scattering code changes across services.

### Shared Loader and Validation

One shared owner must parse, normalize, validate, and identify the configuration. Suitable ownership may live in `ron-policy` or a dedicated narrowly scoped compliance crate.

Validation must reject:

* unsupported schema or version;
* unknown configuration fields;
* invalid ISO country identifiers;
* malformed IPv4 or IPv6 CIDR ranges;
* duplicate or overlapping entries that create ambiguous policy;
* development profiles used implicitly in production;
* enabled enforcement with no defined source or revision;
* unsafe lookup-failure behavior for protected production operations;
* contradictory allow and deny entries;
* expired or superseded configuration where freshness is required.

Consumers must receive a validated policy object rather than parsing the TOML independently.

### Enforcement Boundaries

The shared policy must be applied consistently at:

1. CrabLink frontend bootstrap and session establishment;
2. public API and gateway ingress;
3. account or passport registration where applicable;
4. Service Node registration;
5. node peer admission and session establishment;
6. node runtime participation;
7. reward-evidence acceptance and reward eligibility;
8. administrative or creator-facing product surfaces exposed through CrabLink.

A frontend denial must be accompanied by server-side enforcement. Client-provided country, region, or IP metadata must never be accepted as authoritative.

### IP Range Evaluation

Evaluation must support both IPv4 and IPv6.

The original observed remote address must be derived only from a trusted ingress boundary. Forwarding headers such as `Forwarded` or `X-Forwarded-For` must be accepted only from explicitly configured trusted proxies. Untrusted callers must not be able to spoof the address used for compliance review.

The evaluator should return a stable, low-cardinality result such as:

* `allowed`;
* `blocked_country`;
* `blocked_region`;
* `blocked_ip_range`;
* `lookup_failed`;
* `configuration_invalid`;
* `policy_disabled_for_development`.

Do not expose the full internal block list or sensitive network intelligence to untrusted clients.

### Failure and Privacy Posture

Production behavior must be explicit and fail closed for protected admission or economic operations when the required compliance policy cannot be loaded or validated.

Temporary lookup failures should not create permanent identity accusations. They should deny or defer the protected operation with a stable retryable result where appropriate.

Logs and metrics must avoid retaining unnecessary full client IP addresses. Prefer bounded audit identifiers, normalized jurisdiction results, and privacy-preserving network prefixes where operationally sufficient.

### Economic and Authority Boundaries

The compliance policy may allow or deny later processing, but it must not:

* create or modify ROC balances;
* issue rewards;
* write ledger receipts;
* establish payment or settlement truth;
* mint or burn ROC or ROX;
* create quorum authority;
* alter Service Node lifecycle state without the appropriate registry transition;
* claim legal finality or universal sanctions compliance.

A blocked node must not become reward eligible merely because earlier evidence was accepted before the compliance decision.

### Required Tests

Add focused tests proving:

* blocked IPv4 CIDR addresses are denied;
* blocked IPv6 CIDR addresses are denied;
* addresses outside blocked ranges remain eligible for later checks;
* malformed CIDR entries reject configuration loading;
* client-supplied forwarding headers cannot bypass trusted-ingress resolution;
* frontend denial is mirrored by server-side API denial;
* blocked nodes cannot register or join runtime participation;
* blocked nodes cannot enter reward planning;
* lookup failure follows the configured production posture;
* development fixtures require the explicit development profile;
* no sanctioned range or jurisdiction list is hardcoded in consuming crates;
* policy decisions do not mutate wallet, ledger, rewards, quorum, or finality state.

### Implementation Timing

This requirement is recorded now as a mandatory future compliance gate. It must be implemented before public production access, production node admission, or production ROC reward participation is enabled.

The authoritative sanctions source, update process, retention rules, appeal/error-correction flow, and jurisdiction-specific legal review must be finalized before production activation.
