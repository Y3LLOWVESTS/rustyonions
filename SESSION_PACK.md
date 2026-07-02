# INTERNAL ROC PHASE 6 / STABILIZATION / BRIDGE DOCS SESSION PACK

RO:WHAT — Single uploadable session pack containing the Internal ROC Phase 6 closeout, Internal ROC Beta active doctrine, QuickChain parked decision gate, and ROC ↔ ROX bridge docs/threat-model package.

RO:WHY — Lets the next session load one Markdown file instead of many separate documents.

RO:INVARIANTS — Internal ROC Phase 6 is COMPLETE / GREEN / PARKED. Current workstream is Internal ROC Beta Stabilization / Product Beta Readiness. Bridge remains docs / threat-model / decision-gate only. No ROX/Solana/bridge/staking/liquidity/external settlement runtime is authorized.

---

## SESSION SAFE LABEL

Internal ROC Beta Phase 6: COMPLETE / GREEN / PARKED.

Internal ROC beta value-loop proof: COMPLETE / GREEN / PARKED.

Current workstream: Internal ROC Beta Stabilization / Product Beta Readiness.

Bridge work: docs / threat-model / decision-gate only.

No ROX/Solana/bridge/staking/liquidity/external settlement runtime.

---

## INCLUDED FILES

1. docs/internal-roc-beta/PHASE6_CLOSEOUT.md
2. docs/roadmap/INTERNAL_ROC_BETA_VALUE_LOOP_BLUEPRINT.md
3. docs/roadmap/INTERNAL_ROC_BETA_BUILDPLAN.md
4. docs/roadmap/POST_QUICKCHAIN_DECISION_GATE.md
5. docs/roadmap/POST_INTERNAL_ROC_BRIDGE_DECISION_GATE.md
6. docs/blueprints/ROC_ROX_SOLANA_ANCHOR_BRIDGE_BLUEPRINT.md
7. docs/buildplans/ROC_ROX_SOLANA_BRIDGE_BUILDPLAN.md
8. docs/threat-models/ROC_ROX_BRIDGE_THREAT_MODEL.md

---


<!-- ============================================================ -->
<!-- BEGIN SOURCE FILE: docs/internal-roc-beta/PHASE6_CLOSEOUT.md -->
<!-- ============================================================ -->

# SOURCE FILE: docs/internal-roc-beta/PHASE6_CLOSEOUT.md

# INTERNAL ROC BETA PHASE 6 CLOSEOUT

RO:WHAT — Records the completed Internal ROC Beta Phase 6 proof and locks the safe stabilization label for the next workstream.

RO:WHY — Prevents future sessions from accidentally restarting earlier beta phases or confusing internal ROC proof completion with ROX, Solana, bridge, staking, liquidity, or external settlement runtime.

RO:INTERACTS — POST_QUICKCHAIN_DECISION_GATE.md, INTERNAL_ROC_BETA_VALUE_LOOP_BLUEPRINT.md, INTERNAL_ROC_BETA_BUILDPLAN.md, INTERNAL_ROC_MODEL.md, CrabLink Tauri, svc-gateway, omnigate, svc-wallet, ron-ledger, ron-accounting, svc-rewarder, ron-policy, svc-storage, svc-index.

RO:INVARIANTS — Internal ROC Phase 6 is complete/green/parked; svc-wallet remains the only internal ROC mutation front-door; ron-ledger remains durable economic truth; CrabLink remains display/user-intent only; no bridge/runtime/external settlement/staking/liquidity path is authorized by this closeout.

RO:METRICS — Future stabilization metrics may track smoke-suite reproducibility, paid-flow proof health, receipt/balance refresh truth, replay/conservation proof status, forbidden-scope checker coverage, and CrabLink UX regressions.

RO:CONFIG — Mutable economics values remain in configs/roc-economics.toml. Future bridge/staking fields remain inert, disabled-by-default, and non-operational unless separately authorized.

RO:SECURITY — No fake balances, fake receipts, fake finality, silent spend, cache-only paid unlock, client payout authority, gateway/omnigate/index/storage/policy/accounting/rewarder ledger mutation, ROX runtime, Solana runtime, bridge runtime, staking runtime, liquidity runtime, or exchange-facing runtime.

RO:TEST — This closeout is enforced by docs checks, forbidden-scope checks, paid-flow smoke records, replay/conservation tests, tokenomics validation, CrabLink receipt UX checks, and future stabilization gates.

---

## 0. Safe current label

Use this label in carry-over notes, session openers, and future buildplans:

```text
Internal ROC Beta Phase 6: COMPLETE / GREEN / PARKED.
Internal ROC beta value-loop proof: COMPLETE / GREEN / PARKED.
Current workstream: Internal ROC Beta Stabilization / Product Beta Readiness.
Bridge work: docs / threat-model / decision-gate only.
No ROX/Solana/bridge/staking/liquidity/external settlement runtime.
```

This label supersedes any older instruction that says to begin Internal ROC Beta Phase 1, Phase 2, Phase 3, Phase 4, Phase 5, or Phase 6 implementation work.

Older phase notes remain useful as historical evidence, but they must not reopen parked work unless a new reviewed decision gate explicitly says so.

---

## 1. What Phase 6 proved

Internal ROC Beta Phase 6 proved the closed internal value loop:

```text
CrabLink user intent
→ backend quote / prepare
→ explicit confirmation
→ svc-wallet mutation request
→ ron-ledger durable receipt and balance truth
→ backend paid access enforcement
→ CrabLink display-only receipt and balance UX
→ replay / conservation proof
→ accounting snapshot as derivative reporting
→ rewarder planning as non-authority
→ policy gating as non-authority
→ approved payout execution only through svc-wallet
```

The proof does not mean:

```text
ROX is live
Solana is live
a bridge is live
staking is live
liquidity is live
external settlement is live
exchange-facing behavior is live
public validator economics are live
QuickChain is a public runtime
CrabLink has economic authority
gateway or omnigate can mutate balances
accounting, rewarder, policy, storage, or index can mutate balances
```

Correct meaning:

```text
The internal ROC value plane has been proven as a closed, replayable, auditable, wallet/ledger-truthful beta loop.
```

---

## 2. Completed Phase 6 scope

Phase 6 is considered complete because the beta smoke posture reached:

```text
gateway and omnigate health proven
wallet health proven
CrabLink launched through the Tauri path
paid content prepare / quote / confirm flow proven
backend receipt displayed in CrabLink
ledger-backed balance refresh shown in CrabLink
paid route resolution worked
asset / manifest fetch worked
negative probes failed closed
quote tamper probes failed closed
forbidden external scope remained absent
reproducible smoke suite reached green state
```

The closeout records this as a project-management and doctrine milestone.

It does not create any new runtime authority.

---

## 3. Active workstream after closeout

The active workstream after this file is:

```text
Internal ROC Beta Stabilization / Product Beta Readiness
```

This workstream may include:

```text
polishing CrabLink receipt and balance UX
hardening idempotent retry UX
tightening paid-content route regressions
adding clearer error states
improving stabilization checkers
creating carry-over notes
refreshing targeted codebundles
improving docs consistency
building non-authority dashboards
strengthening smoke reproducibility
```

This workstream must not include:

```text
ROX token runtime
Solana program runtime
bridge mint / burn runtime
external settlement runtime
staking runtime
liquidity runtime
exchange-facing runtime
client-side balance authority
client-side receipt authority
gateway ledger mutation
omnigate ledger mutation
index / storage / policy / accounting / rewarder ledger mutation
cache-only paid unlock
silent spend
fake finality
```

---

## 4. Stabilization rules

### Rule 1 — Do not restart parked beta phases

If a future note says “start Phase 1 paid content,” read it as stale unless the current decision gate says otherwise.

Correct interpretation:

```text
Phase 1 through Phase 6 implementation work is parked.
Stabilization may harden the same surfaces, but it is not a restart of the beta buildplan.
```

### Rule 2 — Keep CrabLink display-only

CrabLink may display:

```text
backend-derived quote
backend-derived receipt
backend-derived balance
backend-derived access status
backend-derived pending / failed / completed status
```

CrabLink must not invent:

```text
balance
receipt
finality
bridge completion
paid entitlement
payout authority
refund authority
```

### Rule 3 — Keep gateway and omnigate non-authoritative

Gateway and omnigate may route, enforce, hydrate, and display backend-derived status.

They must not mutate balances, fabricate receipts, decide finality, unlock from cache alone, or replace svc-wallet / ron-ledger truth.

### Rule 4 — Keep accounting and rewarder non-authoritative

ron-accounting may snapshot and report.

svc-rewarder may plan capped payouts.

Neither may mutate ledger balances or create final receipts.

Approved payout execution remains:

```text
approved payout intent
→ svc-wallet
→ ron-ledger
→ durable receipt
```

---

## 5. Bridge planning status

Bridge planning is allowed only as:

```text
docs
decision gates
threat models
DTO sketches
state-machine design
test plans
review prompts
configuration placeholders that are inert/off-by-default
```

Bridge planning is not:

```text
runtime activation
Solana deployment
ROX live token launch
mint/burn execution
external settlement
liquidity
staking
exchange support
public validator economics
```

The bridge docs must preserve this order:

```text
internal ROC proof
→ closeout
→ docs-only bridge decision gate
→ bridge threat model
→ proof/finality model
→ DTO sketches
→ local-only test skeletons later, if separately authorized
```

---

## 6. Carry-over note template

Use this snippet at the top of future carry-over notes:

```text
Current safe status:
- QuickChain boundary/preflight: COMPLETE / GREEN / PARKED through Phase 5.
- Internal ROC Beta Phase 6: COMPLETE / GREEN / PARKED.
- Active workstream: Internal ROC Beta Stabilization / Product Beta Readiness.
- Bridge status: docs / threat-model / decision-gate only.
- Forbidden runtime: ROX, Solana, public bridge, staking, liquidity, exchange-facing external settlement.
- Economic truth: svc-wallet mutates; ron-ledger records; CrabLink displays.
```

---

## 7. Next safe artifacts

Recommended next docs and checkers:

```text
docs/roadmap/POST_INTERNAL_ROC_BRIDGE_DECISION_GATE.md
docs/blueprints/ROC_ROX_SOLANA_ANCHOR_BRIDGE_BLUEPRINT.md
docs/buildplans/ROC_ROX_SOLANA_BRIDGE_BUILDPLAN.md
docs/threat-models/ROC_ROX_BRIDGE_THREAT_MODEL.md
scripts/check-internal-roc-phase6-closeout.sh
scripts/check-roc-rox-bridge-docs-v2.sh
```

Recommended next code-generation action:

```text
refresh targeted codebundles for stabilization review only
```

Recommended target crates for refreshed codebundles:

```text
ron-proto
ron-ledger
svc-wallet
ron-accounting
svc-rewarder
ron-policy
svc-gateway
omnigate
svc-storage
svc-index
```

CrabLink Tauri should remain primary for product UX stabilization.

---

## 8. Completion statement

Internal ROC Beta Phase 6 is closed.

The internal ROC value-loop proof is complete.

The project may now move to stabilization and product beta readiness while keeping bridge work parked behind docs, threat model, and future explicit authorization.

Safe completion label:

```text
Internal ROC Beta Phase 6 COMPLETE / GREEN / PARKED.
```

<!-- ============================================================ -->
<!-- END SOURCE FILE: docs/internal-roc-beta/PHASE6_CLOSEOUT.md -->
<!-- ============================================================ -->


<!-- ============================================================ -->
<!-- BEGIN SOURCE FILE: docs/roadmap/INTERNAL_ROC_BETA_VALUE_LOOP_BLUEPRINT.md -->
<!-- ============================================================ -->

# SOURCE FILE: docs/roadmap/INTERNAL_ROC_BETA_VALUE_LOOP_BLUEPRINT.md

# INTERNAL_ROC_BETA_VALUE_LOOP_BLUEPRINT.md — Internal ROC Beta Value-Loop Blueprint

RO:WHAT — Defines the active post-QuickChain blueprint for proving RustyOnions / CrabLink internal ROC as a usable, replayable, auditable, wallet/ledger-truthful value loop.

RO:WHY — Before ROX, Solana, public bridge, external settlement, staking, liquidity, or exchange-facing logic can be considered, the internal ROC economy must prove paid creation, paid access, receipts, replay, accounting snapshots, reward planning, approved payout execution, and CrabLink receipt UX using existing authority boundaries.

RO:INTERACTS — POST_QUICKCHAIN_DECISION_GATE.md, INTERNAL_ROC_MODEL.md, QUICKCHAIN_REVIEW_BUNDLE.MD, QUICKCHAIN_BUILDPLAN.MD, configs/roc-economics.toml, ron-proto, ron-ledger, svc-wallet, ron-accounting, svc-rewarder, ron-policy, svc-ads, svc-storage, svc-gateway, omnigate, svc-index, CrabLink Tauri, future QuickChain proof/checkpoint work.

RO:INVARIANTS — Internal ROC first; svc-wallet remains mutation front-door; ron-ledger remains durable economic truth; ron-accounting is not balance truth; svc-rewarder plans but does not mutate; CrabLink displays/user-confirms only; no fake balances, fake receipts, fake finality, silent spend, cache-only paid unlock, bridge runtime, staking runtime, liquidity, or external settlement.

RO:METRICS — Future beta metrics should track paid-flow prepare/confirm/capture success, wallet receipt counts, hold/capture/release outcomes, ledger replay equality, balance conservation, accounting snapshot seals, reward plan generation, payout execution receipts, policy rejections, anti-farming rejections, tokenomics config validation, and CrabLink paid UX outcomes.

RO:CONFIG — Mutable economics values live in configs/roc-economics.toml. Future bridge/staking fields may exist only as inert, disabled-by-default placeholders until later reviewed phases authorize runtime behavior.

RO:SECURITY — No client-side payout authority; no gateway/omnigate direct ledger mutation; no accounting/rewarder/policy wallet mutation; no raw engagement direct ROC minting; no bridge/staking/liquidity/exchange-facing behavior; no private keys/seeds/raw capabilities in React/TypeScript/localStorage/logs/URLs.

RO:TEST — Beta acceptance requires focused paid-flow tests, wallet/ledger replay tests, conservation tests, no-authority tests, paid/cache boundary tests, accounting/rewarder non-mutation tests, tokenomics TOML validation, CrabLink explicit-confirmation tests, and reproducible smoke suites.

---

## INTERNAL-ROC-PHASE6-SAFE-LABEL

Current safe project label:

```text
Internal ROC Beta Phase 6: COMPLETE / GREEN / PARKED.
Internal ROC beta value-loop proof: COMPLETE / GREEN / PARKED.
Current workstream: Internal ROC Beta Stabilization / Product Beta Readiness.
Bridge work: docs / threat-model / decision-gate only.
No ROX/Solana/bridge/staking/liquidity/external settlement runtime.
```

This label prevents older carry-over notes from reopening parked Internal ROC Beta implementation phases or authorizing external runtime scope.

## 0. Status

This document is the active blueprint for the next RustyOnions / CrabLink workstream after QuickChain Phase 5 boundary/preflight completion.

Safe status:

```text id="6h2tdj"
QuickChain boundary/preflight scope is complete through Phase 5.
Internal ROC value-plane beta proof is now the active priority.
```

This blueprint is not:

```text id="5m5tgm"
a ROX launch plan
a Solana launch plan
a public bridge plan
a staking plan
a liquidity plan
an exchange-facing plan
a public validator economy plan
a replacement for wallet/ledger truth
a shortcut around svc-wallet
a shortcut around ron-ledger
legal, tax, or financial advice
```

This blueprint exists to prove one thing:

```text id="m8mql2"
Internal ROC works as a closed, honest, replayable, auditable value loop before anything external is activated.
```

---

## 1. Relationship to the post-QuickChain decision gate

This blueprint depends on:

```text id="kzx1wa"
docs/roadmap/POST_QUICKCHAIN_DECISION_GATE.md
```

That decision gate locks the safe transition from QuickChain boundary/preflight completion into internal ROC beta proof.

This blueprint must preserve the decision gate’s core statement:

```text id="bfv589"
QuickChain boundary/preflight scope is complete through Phase 5.
RustyOnions / CrabLink now moves to internal ROC beta value-loop proof.
Bridge, ROX, Solana, staking, liquidity, exchange-facing logic, and public validator economy remain deferred.
Staking may remain a future dormant capability, but it is off by default and not active runtime.
No new ledger mutation paths are authorized.
```

If this blueprint conflicts with the decision gate, the decision gate wins.

---

## 2. One-sentence thesis

Internal ROC beta is the proof that:

```text id="ekmh3c"
A user can spend internal ROC through explicit confirmation, the backend wallet can mutate ledger truth, the ledger can produce durable receipts, paid access can unlock from backend truth, balances can refresh from backend truth, accounting can snapshot deterministic usage, rewarder can plan bounded payouts, and approved payouts can execute only through svc-wallet.
```

Short version:

```text id="9vyo0g"
Paid action → explicit confirmation → svc-wallet → ron-ledger → receipt → paid enforcement → CrabLink display → replay/conservation proof.
```

---

## 3. Core goals

The beta value loop must prove:

```text id="tm63sd"
paid creation
paid access
paid site visits
paid content views
creator revenue
backend-derived receipts
backend-derived balance refresh
ledger replay
balance conservation
accounting snapshots
reward planning
approved payout execution
CrabLink receipt UX
tokenomics config validation
anti-farming boundaries
```

Minimum product proof targets:

```text id="q4mtg5"
image
site
site_visit
post
comment
article
content_view
```

Optional later beta targets:

```text id="fl297p"
music
video
podcast
stream
chat
Make/reuse flows
ad-budgeted flows
node/provider rewards
moderation rewards
curation/referral rewards
```

The optional targets must not be pulled into the first active batch unless the first proof targets are stable.

---

## 4. Non-goals

This blueprint does not authorize:

```text id="y59xdi"
ROX active runtime
Solana active runtime
Solana Anchor program code
ROC/ROX bridge code
bridge mint/burn
bridge custody
external settlement
staking runtime
staking UI
staking reward accrual
liquidity pools
market-making
exchange integrations
public validator economy
client-side wallet authority
gateway ledger mutation
omnigate ledger mutation
policy ledger mutation
accounting ledger mutation
rewarder ledger mutation
index payment truth
storage payment truth
cache-only paid unlock
raw engagement direct ROC minting
```

Staking posture:

```text id="5poydy"
Staking is preserved only as a future dormant capability.
It remains off by default.
It is not part of internal ROC beta runtime.
It is not user-facing.
It is not yield-bearing.
It is not bridge-enabled.
It is not exchange-facing.
```

Bridge posture:

```text id="qpcd35"
Bridge is parked.
Bridge threat-model work may happen later.
Bridge implementation is not active scope.
```

---

## 5. Authority model

The internal ROC beta value loop must preserve the existing authority model.

```text id="vmt7eo"
ron-proto:
  DTOs only

ron-policy:
  validates/evaluates declarative policy and economics config

svc-wallet:
  only allowed economic mutation front-door

ron-ledger:
  durable economic truth

ron-accounting:
  snapshots/reports/usage windows only; not balance truth

svc-rewarder:
  capped payout planning only; no ledger mutation

svc-storage:
  bytes/artifacts by b3 only; not payment truth

svc-index:
  lookup/pointer/search only; not payment truth

svc-gateway:
  public ingress/proxy/admission boundary only

omnigate:
  hydration/access coordination only

CrabLink Tauri:
  display, routing, user intent, explicit confirmation UX only
```

The only active mutation path is:

```text id="uj6n01"
user intent
→ prepare/quote
→ explicit confirmation
→ svc-wallet
→ ron-ledger
→ backend receipt
→ backend access decision
→ paid enforcement/render
→ display-only receipt cache
→ backend-derived balance refresh
```

No other path may mutate ledger truth.

---

## 6. Internal ROC value-loop architecture

### 6.1 Paid action loop

The canonical paid action loop is:

```text id="isszpb"
CrabLink user action
→ gateway route
→ omnigate/backend prepare or quote
→ explicit confirmation in CrabLink
→ gateway route
→ svc-wallet mutation request
→ ron-ledger durable commit
→ backend receipt/access response
→ paid access enforcement
→ content render
→ receipt displayed in CrabLink
→ balance refreshed from backend
```

Requirements:

```text id="cv39l5"
Every spend has explicit confirmation.
Every accepted spend has backend receipt truth.
Every unlock depends on backend access truth.
Every balance display refreshes from backend truth.
Every local receipt cache is display-only.
Every failure is source-labeled and redacted.
Every retry is idempotent or safely rejected.
```

### 6.2 Creator revenue loop

The canonical creator revenue loop is:

```text id="mp6hul"
creator publishes content
→ content receives b3/manifest/site pointer
→ visitor prepares paid access
→ visitor confirms spend
→ svc-wallet commits payment
→ ron-ledger records transfer/hold/capture
→ backend returns receipt
→ creator balance becomes backend-derived truth
→ CrabLink displays creator receipt/balance
```

Requirements:

```text id="8t0zmt"
Creator revenue must be ledger-derived.
Manifest split rules must not become payment truth by themselves.
Index pointers must not imply payment.
Storage bytes must not imply payment.
Policy allow decisions must not imply payment.
CrabLink display must not imply payment.
```

### 6.3 Accounting and reward loop

The canonical reward loop is:

```text id="xwl4oz"
classified event
→ ron-accounting snapshot/report
→ svc-rewarder capped payout plan
→ ron-policy validation/gating
→ svc-wallet approved payout mutation
→ ron-ledger durable receipt
→ optional later QuickChain proof/commitment
```

Requirements:

```text id="hn5bpi"
Raw engagement never directly mints ROC.
Accounting snapshots are not balances.
Reward plans are not receipts.
Policy decisions are not receipts.
Only svc-wallet executes approved payout intents.
Only ron-ledger records durable economic truth.
Reward replay cannot double issue.
```

---

## 7. Paid-flow product targets

### 7.1 Image

Beta target:

```text id="wl7wfa"
A creator can publish a paid image asset.
A visitor can prepare paid image access.
The visitor sees a quote.
The visitor explicitly confirms.
The backend wallet path commits payment.
A backend receipt is returned.
The image unlocks only from backend access truth.
Both visitor and creator can see backend-derived receipt/balance status.
```

Must not:

```text id="og8ifn"
unlock from b3 existence alone
unlock from local cache alone
unlock from manifest paid=true alone
unlock from index pointer alone
invent image receipt in client
```

### 7.2 Site

Beta target:

```text id="j1ammz"
A creator can publish a named crab:// site.
The site root can reference b3-backed assets.
Paid site or paid site component access can be prepared.
Visitor payment can route through existing wallet/ledger path.
Site render uses backend-derived paid access truth.
```

Must not:

```text id="es2uxz"
treat named site pointer as payment proof
treat site manifest as receipt
treat local site cache as entitlement
```

### 7.3 Site visit

Beta target:

```text id="k1fcdp"
Visitor B pays Creator A for a site visit.
The paid visit produces backend wallet/ledger receipt truth.
CrabLink displays the visit receipt.
Ledger replay proves the visit payment.
Accounting can classify the visit event without becoming balance truth.
```

Must not:

```text id="gp27bf"
pay from raw page view alone
mint from fake visit counters
treat analytics event as economic receipt
```

### 7.4 Post

Beta target:

```text id="7q5ior"
A creator can publish a paid post.
A visitor can prepare, confirm, pay, receive receipt, and view the post.
Receipt and balance views are backend-derived.
```

Must not:

```text id="ynzsh0"
create a new wallet mutation path
unlock from local post cache
treat post manifest as paid proof
```

### 7.5 Comment

Beta target:

```text id="rsz3pd"
A creator/site can define paid comment access or paid comment publishing.
The paid comment path uses existing prepare/quote → confirm → wallet → receipt flow.
Moderation/policy outcomes may gate visibility, but cannot create receipt truth.
```

Must not:

```text id="oc6mdu"
reward raw comments directly
allow comment spam to mint ROC
treat moderation approval as payment
```

### 7.6 Article

Beta target:

```text id="ht8w2e"
A creator can publish a paid article.
A visitor can prepare, confirm, pay, receive receipt, and read the article.
Article access uses backend-derived paid entitlement.
```

Must not:

```text id="eg4xhy"
unlock from article cache alone
unlock from index pointer
fake article access receipt locally
```

### 7.7 Generic content_view

Beta target:

```text id="xsgryv"
The system has a reusable paid content_view path for supported asset kinds.
The path is source-labeled, backend-derived, receipt-based, and replayable.
```

Must not:

```text id="ccf5ds"
become a generic client-side entitlement engine
become a policy-only unlock engine
become a cache-only unlock engine
```

---

## 8. Receipt doctrine

Receipt truth belongs to backend wallet/ledger acceptance.

Receipt display belongs to CrabLink.

Allowed receipt statuses:

```text id="zjstr2"
prepared
quoted
accepted
captured
released
expired
failed
```

Future proof/checkpoint statuses may be displayed only if source-labeled:

```text id="8dwvhu"
epoch_included
finalized
anchored
verifier_ready
committee_ready
quorum_ready
```

Display rule:

```text id="fpzlck"
Accepted backend wallet/ledger receipt can unlock current paid content.
Future proof/checkpoint/anchor labels must not become client-side unlock authority.
```

Forbidden receipt behavior:

```text id="o1mo1p"
local receipt fabrication
receipt-shaped JSON as truth
recent receipt cache as entitlement
offline cache as entitlement
manifest receipt as entitlement
policy receipt as entitlement
index receipt as entitlement
gateway-generated fake receipt
omnigate-generated fake receipt
CrabLink-generated receipt
```

---

## 9. Balance doctrine

Balance truth belongs to the backend wallet/ledger path.

CrabLink may display:

```text id="dgio2g"
backend-derived balance
backend-derived pending holds
backend-derived available balance
backend-derived recent receipts
backend-derived refresh time
stale/offline labels
```

CrabLink must not display as truth:

```text id="u09hni"
locally computed balance
cached balance without stale label
estimated balance as real balance
receipt-cache-derived balance
accounting-derived balance
reward-plan-derived balance
policy-derived balance
QuickChain-readiness-derived balance
```

Balance refresh rule:

```text id="lhwh8j"
After any paid action, CrabLink must refresh balance from backend truth or clearly show that refresh failed/stale.
```

---

## 10. Hold/capture/release doctrine

Paid flows may use holds where appropriate.

Canonical lifecycle:

```text id="865pqo"
prepare/quote
→ open hold
→ capture hold
→ receipt
```

Other allowed outcomes:

```text id="oo8v0s"
release hold
expire hold
fail hold
retry idempotently
```

Hard rules:

```text id="o3dpud"
A captured hold cannot be captured again.
A released hold cannot be captured later.
An expired hold cannot be captured later.
A terminal hold cannot resurrect.
Retries must return the accepted result or fail safely.
Holds must preserve balance conservation.
```

Beta tests should cover:

```text id="mrzxn4"
open hold
capture hold
release hold
expire hold
duplicate capture rejection
capture-after-release rejection
capture-after-expire rejection
retry-after-accepted behavior
ledger replay equality
```

---

## 11. Ledger replay and conservation

The internal ROC beta is not credible until replay and conservation are proven.

Required replay proof:

```text id="9zkjyr"
same accepted operation history
→ same balances
→ same receipts
→ same holds
→ same terminal state
```

Required conservation proof:

```text id="lu7ekn"
ROC cannot appear without authorized issue/reward execution.
ROC cannot disappear except authorized burn/fee/remainder sink.
Transfers conserve value.
Holds conserve value.
Captures conserve value.
Releases conserve value.
Payout execution is traceable to approved wallet mutation.
Rounding remainders go to explicit configured sinks.
```

Forbidden:

```text id="5ijj4h"
non-deterministic replay
database iteration order changing balances
wall-clock order changing balances
accounting snapshot changing balance truth
reward plan changing balance truth
QuickChain checkpoint changing balance truth
external anchor changing balance truth
```

---

## 12. Event classes

Internal ROC beta must preserve strict event classification.

Allowed event classes:

```text id="rgou8w"
economic_receipt
metering
proof_eligible
ad_budgeted
analytics_only
```

Rules:

```text id="u5dolq"
economic_receipt:
  backend wallet/ledger accepted economic truth only

metering:
  usage facts; not payout truth

proof_eligible:
  candidate reward input requiring verification, caps, challenge, and policy

ad_budgeted:
  funded by explicit payer/sponsor/advertiser budget; not unlimited protocol emission

analytics_only:
  product analytics only; never protocol ROC payout material
```

Hard invariant:

```text id="1m0o62"
Raw engagement must not directly mint or allocate protocol ROC.
```

---

## 13. Accounting snapshot doctrine

ron-accounting may:

```text id="ao7fcf"
record usage facts
normalize classified events
seal deterministic windows
produce accounting snapshots
produce accounting reports
produce reward input summaries
emit audit-friendly evidence
```

ron-accounting must not:

```text id="i4fyl4"
own balance truth
mutate ledger
issue ROC
transfer ROC
burn ROC
capture holds
release holds
create paid entitlement
create receipt truth
override wallet/ledger
```

Accounting beta requirements:

```text id="nyj7a1"
deterministic sealed windows
stable event ordering rules
integer-safe counters
unknown-field rejection where applicable
clear event class labels
anti-poisoning tests
snapshot replay tests
no balance mutation tests
```

---

## 14. Rewarder doctrine

svc-rewarder may:

```text id="evv6jg"
consume accounting snapshots
apply capped reward planning logic
produce deterministic payout plans
apply category pool limits
apply anti-farming limits
emit payout intent candidates
emit audit reports
```

svc-rewarder must not:

```text id="48isfk"
mutate ledger
create wallet receipt truth
issue ROC directly
transfer ROC directly
burn ROC directly
capture/release holds
treat raw engagement as payout
bypass ron-policy
bypass svc-wallet
```

Reward plan lifecycle:

```text id="swuidz"
accounting snapshot
→ rewarder payout plan
→ policy validation/gating
→ explicit approved payout intent
→ svc-wallet mutation
→ ron-ledger receipt
```

Rewarder beta requirements:

```text id="gjo8p1"
deterministic payout plan output
no mutation tests
category pool cap tests
anti-farming cap tests
rounding remainder tests
duplicate payout prevention
policy-gate required tests
wallet-execution-only tests
```

---

## 15. Tokenomics config doctrine

Mutable economics values live in:

```text id="lmx8pu"
configs/roc-economics.toml
```

The config should eventually cover:

```text id="top8pt"
paid content split defaults
creator/reuse/referrer split defaults
burn rates
ad-budgeted rates
epoch pool category percentages
reward weights
reward point conversion parameters
quality/reputation/scarcity multipliers
anti-farming caps
max spend limits
hold multipliers
rounding mode
remainder sink
future bridge placeholders
future staking placeholders
```

Hard rules:

```text id="vuqhfo"
No hard-coded payout amounts in business logic.
No hard-coded reward rates scattered across crates.
No float money.
No hidden rounding.
No missing remainder sink.
No config value can create receipt truth.
No config value can create balance truth.
No config value can unlock paid content.
No config value can enable bridge by accident.
No config value can enable staking by accident.
```

Required validation:

```text id="pdmzs3"
TOML rejects unknown fields
TOML rejects floats
TOML rejects invalid basis-point totals
TOML rejects malformed money
TOML rejects missing remainder sink
TOML rejects bridge-enabled config unless future gate allows it
TOML rejects staking-enabled config unless future gate allows it
splits total exactly 10000 bps
category budgets total correctly
rounding is deterministic
```

---

## 16. CrabLink Tauri beta UX doctrine

CrabLink Tauri is the primary client/product layer.

CrabLink may:

```text id="cehfq5"
display backend-derived balances
display backend-derived receipts
display stale/offline labels
display paid access status
collect user intent
show prepare/quote details
require explicit confirmation
call TypeScript adapters
request allowlisted Tauri commands
call configured gateway paths
render content after backend access truth
show display-only recent receipt history
show display-only local catalog entries
verify b3 cache bytes before trusted render
```

CrabLink must not:

```text id="gcma3m"
own wallet truth
own ledger truth
invent balances
invent receipts
invent finality
invent paid entitlement
silently spend ROC
unlock paid content from cache alone
store private keys/seeds/raw capabilities in React
store spend authority in localStorage
store secrets in URLs
store secrets in logs
call raw Tauri invoke from random UI files
call internal services directly as normal runtime
mutate ledger
mutate wallet
run bridge logic
run staking logic
run liquidity logic
```

CrabLink UX requirements:

```text id="3fvj0j"
every spend shows explicit human-readable quote
every confirmation names amount, asset/action, recipient/split if known, and consequence
cancel path does not mutate wallet
failure path does not unlock content
retry path is idempotent or safely rejected
receipt panel clearly labels backend source
balance refresh is backend-derived
local cache labels are honest
```

---

## 17. Gateway and omnigate doctrine

svc-gateway may:

```text id="tarr7i"
expose stable public routes
enforce admission/quotas/rate limits
proxy selected requests
filter headers
preserve source labels
return backend-derived response bodies
```

svc-gateway must not:

```text id="vrzb4i"
mutate ledger
mutate wallet
create receipts
invent paid access
generate settlement truth
run bridge logic
run staking logic
```

omnigate may:

```text id="iv3m9b"
hydrate product views
coordinate backend access responses
call downstream services
compose display payloads
enforce backend-derived access decisions
```

omnigate must not:

```text id="m04h34"
mutate ledger
mutate wallet
create receipt truth
become finality authority
become bridge authority
become staking authority
unlock from cache alone
```

---

## 18. Storage and index doctrine

svc-storage may:

```text id="o8vwxp"
store bytes
serve bytes
verify b3-addressed objects
support range/segment access
store manifests/artifacts
```

svc-storage must not:

```text id="uz9t3a"
prove payment
create paid entitlement
mutate wallet
mutate ledger
create receipts
```

svc-index may:

```text id="entxv1"
store pointers
resolve names
index assets/sites
support lookup/search/discovery
```

svc-index must not:

```text id="1dfnmy"
prove payment
create receipt truth
create paid entitlement
mutate wallet
mutate ledger
```

Rule:

```text id="n432r5"
b3 proves bytes, not payment.
crab:// navigates, not authority.
names are pointers, not truth.
```

---

## 19. Policy doctrine

ron-policy may:

```text id="1z2l4b"
validate economics config
evaluate declarative rules
gate reward plans
gate paid access preconditions
gate abuse/moderation conditions
emit reasons
```

ron-policy must not:

```text id="h0n645"
mutate ledger
mutate wallet
create balances
create receipts
create paid entitlement
issue rewards directly
settle disputes economically by itself
enable bridge/staking by itself
```

Policy result rule:

```text id="gmhmcz"
Policy allow is not payment.
Policy deny is not refund.
Policy obligation is not receipt.
Policy config is not balance truth.
```

---

## 20. Paid content split doctrine

Paid content may include splits for:

```text id="r244bw"
creator
site owner
source asset owner
music/source clip owner
curator/referrer
moderation/community allocation
protocol burn/sink
treasury/stability buffer
node/provider share
```

Rules:

```text id="niqiol"
Splits must be explicit.
Splits must total exactly 10000 bps where applicable.
Splits must be integer-safe.
Remainders must go to configured sink.
Split manifests are not payment truth.
Only wallet/ledger execution creates actual balances/receipts.
```

Beta split proof should start simple:

```text id="fqpf4g"
single creator recipient
optional site owner split
explicit burn/remainder sink
```

Do not start with complex mashup/reuse/music splits unless required for a specific beta flow.

---

## 21. Anti-farming doctrine

Internal ROC beta must resist fakeable activity.

Forbidden reward bases:

```text id="ebasxv"
raw views directly mint ROC
raw likes directly mint ROC
raw comments directly mint ROC
raw watch time directly mints ROC
raw clicks directly mint ROC
raw reposts/shares directly mint ROC
raw impressions directly mint ROC
raw page visits directly mint protocol ROC
```

Allowed safer inputs:

```text id="zgzbkq"
payer-authorized economic receipts
bounded ad-budgeted events
verified service delivery facts
policy-verified moderation outcomes
challengeable proof_eligible events
reputation-weighted capped summaries
```

Required anti-farming controls:

```text id="3sk0fk"
per-account caps
per-site caps
per-content caps
per-epoch caps
ad-budget limits
duplicate event rejection
sybil-resistant gating where applicable
abuse report review
appeal/challenge paths where applicable
analytics_only isolation
metering isolation
```

---

## 22. Ad-budgeted flow doctrine

Ad-budgeted participation is allowed only if the funds originate from explicit payer-authorized budget.

Correct ad-budgeted path:

```text id="4qx42k"
advertiser/sponsor budget
→ explicit campaign authorization
→ budget hold/allocation
→ classified ad_budgeted events
→ accounting snapshot
→ rewarder allocation plan
→ policy validation
→ svc-wallet execution
→ ron-ledger receipt
```

Forbidden:

```text id="8f4s5a"
ad impression directly mints protocol ROC
raw click directly mints protocol ROC
unbounded ad pool
client-side ad payout
rewarder direct ad payout
accounting direct ad payout
```

---

## 23. Node/provider reward doctrine

Node/provider rewards may be planned later, but beta must keep them bounded and proof-aware.

Possible future reward inputs:

```text id="qy6v1z"
storage availability proof
delivery proof
range/segment service proof
archive support
gateway/service capacity support
challenge response
verified uptime windows
```

Rules:

```text id="h61zdi"
Node rewards are not raw self-claims.
Node rewards require proof/check/challenge posture.
Node rewards are capped.
Node rewards are planned by rewarder.
Node rewards execute only through svc-wallet.
```

Internal beta may document node rewards, but should not prioritize them before paid content flows and replay/conservation are proven.

---

## 24. Moderation/community reward doctrine

Moderation/community rewards may be planned later, but must be bounded and abuse-resistant.

Possible future reward inputs:

```text id="qygk68"
accepted abuse report
upheld moderation action
appeal review
policy-approved community work
documentation/education contribution
accessibility improvement
onboarding/referral program if approved
```

Rules:

```text id="p8cvwl"
No raw reports directly mint ROC.
No raw votes directly mint ROC.
No moderation action pays without policy/caps/review.
Rewarder plans.
Policy gates.
Wallet executes.
Ledger records.
```

---

## 25. Offline cache doctrine

Offline/local cache may:

```text id="6bmcuu"
store display data
store b3-addressed bytes
store display-only receipt summaries
verify b3 before trusted render
show stale/offline labels
help resume UX
```

Offline/local cache must not:

```text id="t36h2b"
unlock paid content alone
create entitlement
refresh entitlement without backend
claim finality
claim settlement
create proof
replace wallet/ledger truth
create balance truth
create receipt truth
```

Rule:

```text id="l809jy"
Verified b3 proves bytes, not paid entitlement.
```

---

## 26. Beta smoke suite target

The internal ROC beta should eventually have a reproducible smoke suite.

The suite should prove:

```text id="t7p0pa"
local/dev stack starts
creator wallet exists
visitor wallet exists
creator publishes asset/site/post/comment/article
visitor prepares paid access
visitor sees quote
visitor explicitly confirms
svc-wallet commits mutation
ron-ledger records receipt
paid content unlocks from backend truth
visitor balance refreshes
creator balance refreshes
receipt displays in CrabLink
ledger replay equals current truth
accounting snapshot seals
rewarder produces non-mutating payout plan
policy validates/gates payout plan
approved payout executes through svc-wallet
final receipts are displayable
```

The smoke suite should be safe by default:

```text id="o9ps7y"
dev-only wallets
dev-only config
bounded test values
no bridge
no staking
no liquidity
no external settlement
no real gas
no production endpoints
no irreversible external actions
```

---

## 27. Testing matrix

Required test categories:

```text id="fepq7k"
paid prepare/quote tests
explicit confirmation tests
wallet mutation tests
ledger receipt tests
ledger replay tests
balance conservation tests
hold/capture/release tests
idempotency/retry tests
paid unlock enforcement tests
paid/cache boundary tests
CrabLink display-only receipt tests
CrabLink no silent spend tests
CrabLink balance refresh tests
accounting snapshot tests
rewarder non-mutation tests
reward plan determinism tests
policy config validation tests
tokenomics TOML validation tests
anti-farming tests
no bridge runtime tests
no staking runtime tests
no liquidity runtime tests
```

Suggested crate/service focus:

```text id="6bugno"
ron-proto:
  DTO shape and strictness

ron-ledger:
  replay/conservation/hold transitions

svc-wallet:
  prepare/quote/issue/transfer/burn/hold/capture/release/receipt

ron-accounting:
  snapshots/reports/non-authority

svc-rewarder:
  payout planning/non-mutation/caps

ron-policy:
  economics config validation/gating

svc-storage:
  b3 bytes/artifacts; no payment truth

svc-index:
  pointers/lookup; no payment truth

svc-gateway:
  public paid routes/proxy boundaries

omnigate:
  access/hydration boundaries

CrabLink Tauri:
  explicit confirmation/display-only receipts/no cache unlock
```

---

## 28. Metrics and audit events

Future beta metrics should include:

```text id="4zxuiw"
roc_paid_prepare_total
roc_paid_confirm_total
roc_paid_cancel_total
roc_paid_capture_total
roc_paid_release_total
roc_paid_expire_total
roc_paid_unlock_total
roc_paid_unlock_denied_total
roc_wallet_receipt_total
roc_wallet_receipt_replay_total
roc_ledger_replay_mismatch_total
roc_balance_conservation_failure_total
roc_accounting_snapshot_sealed_total
roc_reward_plan_created_total
roc_reward_plan_rejected_total
roc_reward_payout_executed_total
roc_policy_rejection_total
roc_tokenomics_config_invalid_total
roc_antifarming_rejection_total
crablink_paid_confirmation_shown_total
crablink_balance_refresh_total
crablink_receipt_display_total
```

Audit events should be structured and redacted.

Audit event families:

```text id="l4numj"
paid_action_prepared
paid_action_confirmed
paid_action_cancelled
wallet_mutation_accepted
wallet_mutation_rejected
ledger_receipt_created
paid_access_granted
paid_access_denied
accounting_window_sealed
reward_plan_created
reward_plan_rejected
payout_intent_approved
payout_intent_executed
tokenomics_config_loaded
tokenomics_config_rejected
anti_farming_rejection
```

No audit event should leak:

```text id="jik2p8"
private keys
seeds
raw capabilities
uncapped spend authority
tokens
secret wallet material
private passport linkage
```

---

## 29. Security rules

Hard security rules:

```text id="6rbyo2"
No secrets in React state.
No secrets in localStorage.
No secrets in sessionStorage.
No secrets in URLs.
No secrets in logs.
No raw capabilities in client display caches.
No uncapped spend authority in CrabLink.
No raw Tauri invoke from arbitrary UI.
No shell/eval/run/native/raw command creep.
No backend internal service calls as normal product path.
No silent spend.
No cache-only paid unlock.
No fake balances.
No fake receipts.
No fake finality.
```

Every paid action must have:

```text id="2yp9hg"
source-labeled request
explicit user intent
explicit quote
explicit confirmation
backend result
redacted error path
idempotency behavior
receipt display path
balance refresh path
```

---

## 30. Staking dormant-capability rule

Staking is not active internal ROC beta scope.

Allowed now:

```text id="pjbg44"
dormant docs language
disabled-by-default config placeholder discussion
risk checklist
legal/regulatory review checklist
tests proving no staking runtime is exposed
tests proving no staking rewards accrue
tests proving no staking ledger mutation path exists
```

Forbidden now:

```text id="qm4pyf"
staking runtime
staking UI
staking APR/APY display
staking reward accrual
delegated staking
pooled staking
liquid staking
staking derivatives
bridge-connected staking
external-chain staking
staking enabled by default
staking marketing copy
staking payout execution
```

Default rule:

```text id="fqsw2h"
If staking exists at all, it is inert.
If there is any doubt, staking remains off.
```

---

## 31. Bridge parking rule

Bridge is not active internal ROC beta scope.

Allowed now:

```text id="dj67w5"
short parking-lot notes
future prerequisite list
risk list
explicit deferred status
```

Forbidden now:

```text id="v2fr4t"
bridge runtime
bridge routes
bridge UI
bridge DTO runtime
bridge mint/burn
bridge custody
Solana Anchor program code
ROX runtime
external settlement
exchange-facing logic
liquidity
```

Bridge prerequisite reminder:

```text id="xp39lb"
Internal ROC value plane must be stable before bridge threat-model work becomes serious.
Wallet/ledger replay and conservation must be proven before bridge design becomes active.
Bridge code must not begin until a later reviewed bridge phase explicitly authorizes it.
```

---

## 32. Recommended implementation order

This blueprint recommends the next buildplan use:

```text id="z0xrcb"
Phase 0 — Post-QuickChain doc sync + safe language audit
Phase 1 — Repeatable paid content flows using existing wallet path
Phase 2 — Ledger replay + conservation proof
Phase 3 — Accounting → rewarder → wallet payout loop
Phase 4 — CrabLink Tauri wallet/receipt UX hardening
Phase 5 — Tokenomics TOML validation and anti-farming gates
Phase 6 — Reproducible internal ROC beta smoke suite
```

Do not implement the buildplan from this blueprint alone.

Create a separate buildplan:

```text id="i35g1d"
docs/buildplans/INTERNAL_ROC_BETA_BUILDPLAN.md
```

---

## 33. First coding target after this blueprint/buildplan

The first coding target after the blueprint and buildplan are reviewed should be:

```text id="gj3xj8"
repeatable paid post / comment / article flows
using the existing prepare/quote → explicit confirmation → backend wallet path → backend receipt → unlock/render → balance refresh path
```

This first target should avoid:

```text id="fo13wn"
new ledger mutation paths
new wallet bypasses
rewarder/accounting mutation
bridge prep code
staking code
liquidity code
complex tokenomics rewrites
```

Goal:

```text id="6v0ddf"
Make the internal ROC paid-content product loop visibly and repeatedly work.
```

---

## 34. Acceptance gates for this blueprint

This blueprint is accepted when reviewers agree that:

```text id="ew6ff7"
the active priority is internal ROC beta value-loop proof
the authority model is correct
the paid action loop uses existing wallet/ledger path
the creator revenue loop is ledger-derived
the accounting/rewarder loop remains non-mutating until wallet execution
the tokenomics config doctrine is clear
raw engagement cannot directly mint ROC
CrabLink remains display/user intent only
paid/cache boundaries are preserved
replay/conservation are required
bridge is parked
staking is dormant/off by default
no external settlement is authorized
the next document should be INTERNAL_ROC_BETA_BUILDPLAN.md
```

---

## 35. Grok / external reviewer checklist

Ask reviewers:

```text id="t67h0y"
1. Does this blueprint correctly follow the post-QuickChain decision gate?
2. Does it keep internal ROC beta as the active priority?
3. Does it avoid creating new ledger mutation paths?
4. Does it keep svc-wallet as mutation front-door?
5. Does it keep ron-ledger as durable economic truth?
6. Does it prevent accounting/rewarder/policy/gateway/omnigate/index/storage from becoming authority?
7. Does it keep CrabLink display/user-intent only?
8. Does it preserve paid/cache boundaries?
9. Does it block raw engagement from directly minting ROC?
10. Does it treat tokenomics TOML as mutable economics source of truth?
11. Does it keep bridge deferred?
12. Does it keep staking dormant/off by default?
13. Does it identify the right first coding target?
14. Does anything accidentally imply external settlement, liquidity, exchange-facing logic, or public staking?
15. What missing tests or gates should be added before coding?
```

---

## 36. Final blueprint statement

The internal ROC beta value-loop mission is:

```text id="r4fg6f"
Prove that RustyOnions / CrabLink can run a closed internal ROC economy where paid actions are explicitly confirmed, wallet mutations flow only through svc-wallet, ledger truth lives only in ron-ledger, receipts unlock paid content only through backend truth, balances refresh from backend truth, accounting snapshots remain non-authoritative, rewarder plans capped payouts without mutation, approved payouts execute only through svc-wallet, CrabLink displays truth without owning it, and replay/conservation prove the value loop is honest.
```

<!-- ============================================================ -->
<!-- END SOURCE FILE: docs/roadmap/INTERNAL_ROC_BETA_VALUE_LOOP_BLUEPRINT.md -->
<!-- ============================================================ -->


<!-- ============================================================ -->
<!-- BEGIN SOURCE FILE: docs/roadmap/INTERNAL_ROC_BETA_BUILDPLAN.md -->
<!-- ============================================================ -->

# SOURCE FILE: docs/roadmap/INTERNAL_ROC_BETA_BUILDPLAN.md

# INTERNAL_ROC_BETA_BUILDPLAN.md — Minimum-Round Build Plan for Internal ROC Beta

RO:WHAT — Defines the efficient phase/round build plan for proving internal ROC beta value-loop behavior after QuickChain Phase 5 boundary/preflight completion.

RO:WHY — Prevents scope creep and duplicate crate-pair churn while proving paid content flows, wallet/ledger receipts, ledger replay, balance conservation, accounting snapshots, reward planning, tokenomics config validation, and CrabLink receipt UX.

RO:INTERACTS — POST_QUICKCHAIN_DECISION_GATE.md, INTERNAL_ROC_BETA_VALUE_LOOP_BLUEPRINT.md, INTERNAL_ROC_MODEL.md, QUICKCHAIN_REVIEW_BUNDLE.MD, QUICKCHAIN_BUILDPLAN.MD, configs/roc-economics.toml, ron-proto, ron-ledger, svc-wallet, ron-accounting, svc-rewarder, ron-policy, svc-ads, svc-storage, svc-index, svc-gateway, omnigate, CrabLink Tauri.

RO:INVARIANTS — Internal ROC first; no new ledger mutation paths; svc-wallet remains mutation front-door; ron-ledger remains durable economic truth; ron-accounting is not balance truth; svc-rewarder plans but does not mutate; CrabLink displays/user-confirms only; no fake balances, fake receipts, fake finality, silent spend, bridge runtime, staking runtime, liquidity, exchange-facing logic, or external settlement.

RO:METRICS — Future completion metrics should track phase/round park status, paid-flow green status, receipt coverage, replay equality, conservation proofs, tokenomics validation, reward-plan determinism, payout execution receipts, CrabLink UX proof status, and forbidden-scope checker coverage.

RO:CONFIG — Mutable economics values live in configs/roc-economics.toml. Future bridge/staking placeholders must remain inert, disabled-by-default, and non-operational unless a later reviewed phase explicitly authorizes runtime behavior.

RO:SECURITY — No client-side payout authority; no gateway/omnigate/index/storage/policy/accounting/rewarder ledger mutation; no cache-only paid unlock; no raw engagement direct ROC minting; no ROX/Solana/bridge/staking/liquidity runtime; no secrets or uncapped spend authority in React/TypeScript/localStorage/URLs/logs.

RO:TEST — Each phase requires focused tests first, then crate-local or app-local park/preflight once, then notes. Avoid duplicate exhaustive reruns. Final beta acceptance requires a reproducible internal ROC smoke suite.

---

## INTERNAL-ROC-PHASE6-SAFE-LABEL

Current safe project label:

```text
Internal ROC Beta Phase 6: COMPLETE / GREEN / PARKED.
Internal ROC beta value-loop proof: COMPLETE / GREEN / PARKED.
Current workstream: Internal ROC Beta Stabilization / Product Beta Readiness.
Bridge work: docs / threat-model / decision-gate only.
No ROX/Solana/bridge/staking/liquidity/external settlement runtime.
```

This label prevents older carry-over notes from reopening parked Internal ROC Beta implementation phases or authorizing external runtime scope.

## 0. Status

This buildplan is the active post-QuickChain implementation map for internal ROC beta.

Safe status:

```text id="e6eyva"
QuickChain boundary/preflight scope is complete through Phase 5.
Internal ROC beta value-loop proof is now the active build priority.
```

This buildplan is not:

```text id="j53rkb"
a QuickChain Phase 6 implementation plan
a ROX implementation plan
a Solana implementation plan
a bridge implementation plan
a staking implementation plan
a liquidity implementation plan
an exchange-facing plan
a public validator economy plan
```

This buildplan must be used with:

```text id="flskyd"
docs/roadmap/POST_QUICKCHAIN_DECISION_GATE.md
docs/blueprints/INTERNAL_ROC_BETA_VALUE_LOOP_BLUEPRINT.md
docs/tokenomics/INTERNAL_ROC_MODEL.md
```

---

## 1. Buildplan model

This buildplan uses the same efficiency discipline as the QuickChain buildplan.

Correct model:

```text id="td2vdt"
One phase = one coherent product/economic proof target.
One round = one intentional pass through selected crate pairs.
Each round has:
  - a purpose
  - exact crate-pair work
  - exit gate
  - forbidden scope
  - no duplicate exhaustive reruns
```

Wrong model:

```text id="8jhs1r"
touch every crate forever
run every test repeatedly
regenerate every codebundle after every tiny change
add bridge prep while doing paid content
add staking config while doing receipt UX
let client fixes become wallet authority
```

Preferred workflow per crate pair:

```text id="cps00v"
1. focused patch
2. cargo fmt / npm check as applicable
3. focused tests
4. crate-local/app-local park once
5. notes
6. move to next pair
```

Do not run both preflight and park if park already delegates to preflight.

Do not run workspace clippy unless explicitly needed.

Do not add Python helper scripts for patches unless explicitly requested.

---

## 2. Fixed crate-pair order for full internal ROC beta rounds

Use this order for full rounds unless a phase explicitly narrows scope:

```text id="d7mvn7"
1. ron-proto + ron-ledger
2. svc-wallet + ron-accounting
3. svc-rewarder + ron-policy
4. svc-storage + svc-index
5. svc-gateway + omnigate
6. CrabLink Tauri + client adapters
```

Why this order exists:

```text id="46bcg5"
DTO/economic shape
→ ledger truth/replay
→ wallet mutation
→ accounting snapshots
→ reward planning/policy gating
→ storage/index artifacts/pointers
→ gateway/omnigate paid enforcement
→ CrabLink display/user confirmation
```

The active internal ROC value loop is:

```text id="v3y7sx"
ron-proto economic DTOs
→ svc-wallet issue/transfer/burn/hold/capture/release/receipt
→ ron-ledger durable truth and replay
→ svc-storage/svc-gateway/omnigate paid enforcement
→ ron-accounting snapshots
→ svc-rewarder payout planning
→ ron-policy validation/gating
→ svc-wallet approved mutation path
→ ron-ledger receipt and balance truth
→ CrabLink display/user intent only
```

Note:

```text id="4tvfpz"
svc-ads may be added to focused rounds when ad-budgeted flows become active.
svc-passport may be added to focused rounds when beta identity gating requires it.
svc-registry may be added to focused rounds when node/provider or validator-adjacent flows require it.
Do not add them by default.
```

---

## 3. Global forbidden scope

Forbidden in every phase unless a later reviewed decision gate explicitly authorizes it:

```text id="mtsmom"
ROX active runtime
Solana active runtime
Solana Anchor program code
bridge mint/burn code
bridge custody code
external settlement
exchange-facing logic
liquidity
liquidity pools
market making
public staking runtime
staking UI
staking APR/APY display
staking reward accrual
delegated staking
pooled staking
liquid staking
staking derivatives
public validator economy
gateway direct ledger mutation
omnigate direct ledger mutation
index as payment truth
storage as payment truth
policy as balance truth
accounting as balance truth
rewarder direct ledger mutation
CrabLink wallet authority
CrabLink bridge authority
CrabLink staking authority
cache-only paid unlock
fake receipts
fake balances
fake finality
silent spend
raw engagement directly minting protocol ROC
```

Allowed staking posture:

```text id="c865tt"
Future dormant capability only.
Off by default.
Not active runtime.
Not user-facing.
Not yield-bearing.
Not bridge-enabled.
Not exchange-facing.
```

Allowed bridge posture:

```text id="c6p6ji"
Future/deferred bridge design only.
No bridge code.
No bridge routes.
No bridge UI.
No bridge DTO runtime.
No mint/burn runtime.
```

---

## 4. Completion labels

Use precise labels.

Allowed labels:

```text id="1mx6tx"
Internal ROC Beta Phase 0 docs/safe-language sync complete.
Internal ROC Beta Phase 1 paid content flow proof complete.
Internal ROC Beta Phase 2 replay/conservation proof complete.
Internal ROC Beta Phase 3 accounting/rewarder/wallet payout loop proof complete.
Internal ROC Beta Phase 4 CrabLink receipt UX proof complete.
Internal ROC Beta Phase 5 tokenomics config/anti-farming proof complete.
Internal ROC Beta Phase 6 reproducible smoke suite complete.
Internal ROC beta value-loop proof complete.
```

Forbidden labels:

```text id="z2s0fd"
ROX ready
bridge ready
staking ready
external settlement ready
public chain ready
exchange ready
liquidity ready
QuickChain live
settlement live
public validator economy ready
```

---

## 5. Minimum-round phase plan

Optimized plan:

```text id="xs2ads"
Phase 0 — Post-QuickChain doc sync + safe language audit
  Ideal rounds: 1 docs-only round

Phase 1 — Repeatable paid content flows using existing wallet path
  Ideal rounds: 2 implementation rounds

Phase 2 — Ledger replay + conservation proof
  Ideal rounds: 1–2 core-heavy rounds

Phase 3 — Accounting → rewarder → wallet payout loop
  Ideal rounds: 2 implementation rounds

Phase 4 — CrabLink Tauri wallet/receipt UX hardening
  Ideal rounds: 1–2 client-heavy rounds

Phase 5 — Tokenomics TOML validation and anti-farming gates
  Ideal rounds: 2 implementation rounds

Phase 6 — Reproducible internal ROC beta smoke suite
  Ideal rounds: 1 integration round
```

Total target:

```text id="y9nprr"
10–12 intentional rounds
```

This replaces unbounded crate-pair churn.

---

# Phase 0 — Post-QuickChain doc sync + safe language audit

## Goal

Lock safe language before new implementation.

## Ideal rounds

```text id="buoqj2"
1 docs-only round
```

## Phase 0 Round 1 — safe scope lock

### Purpose

Make sure the repo clearly says:

```text id="qvsihi"
QuickChain boundary/preflight scope is complete through Phase 5.
Internal ROC beta value-loop proof is now the active priority.
Bridge, ROX, Solana, staking, liquidity, exchange-facing logic, and public validator economy remain deferred.
No new ledger mutation paths are authorized.
```

### Work

```text id="skahsc"
docs/roadmap/POST_QUICKCHAIN_DECISION_GATE.md:
  - create/confirm safe language

docs/blueprints/INTERNAL_ROC_BETA_VALUE_LOOP_BLUEPRINT.md:
  - create/confirm active blueprint

docs/buildplans/INTERNAL_ROC_BETA_BUILDPLAN.md:
  - create/confirm this file

docs/tokenomics/INTERNAL_ROC_MODEL.md:
  - confirm internal ROC doctrine and TOML ownership

QuickChain docs:
  - ensure references say boundary/preflight complete, not public chain live

CrabLink notes:
  - ensure Tauri/client adapters remain display/user intent only
```

### Suggested checks

```bash id="sd7mtm"
grep -Rni "QuickChain complete" docs QUICKCHAIN*.MD || true
grep -Rni "public chain live\|bridge live\|staking live\|liquidity live\|exchange ready" docs QUICKCHAIN*.MD || true
grep -Rni "Internal ROC value-plane beta proof is now the active priority" docs || true
grep -Rni "No new ledger mutation paths" docs || true
```

Any match must be reviewed for context.

### Exit gate

```text id="mob768"
Decision gate exists.
Internal ROC beta blueprint exists.
Internal ROC beta buildplan exists.
QuickChain completion wording is safe.
Bridge is parked.
Staking is dormant/off by default.
No new ledger mutation paths are authorized.
Next coding target is paid post/comment/article using existing wallet path.
```

### Notes

This phase is docs-only.

No Cargo changes.

No npm changes.

No code changes.

---

# Phase 1 — Repeatable paid content flows using existing wallet path

## Goal

Make paid post, paid comment, paid article, and generic content_view flows repeatable using the existing paid path.

Canonical path:

```text id="sdnyvc"
prepare/quote
→ explicit confirmation
→ backend wallet path
→ backend receipt
→ unlock/render
→ backend balance refresh
```

## Ideal rounds

```text id="wjfcbp"
2 implementation rounds
```

---

## Phase 1 Round 1 — backend route and authority proof

### Purpose

Confirm or implement the backend route contracts needed for repeatable paid post/comment/article/content_view flows without adding a new mutation path.

### Primary crate pairs

```text id="pyp9u3"
1. ron-proto + ron-ledger
2. svc-wallet + ron-accounting
3. svc-storage + svc-index
4. svc-gateway + omnigate
```

### Crate-pair work

```text id="fv0b6u"
ron-proto + ron-ledger:
  - audit existing DTOs for paid post/comment/article/content_view needs
  - add DTO shape only if required
  - keep money as integer minor units/strings
  - reject unknown fields where applicable
  - confirm ledger operation history can represent paid access
  - no bridge/staking/liquidity fields
  - no new mutation authority

svc-wallet + ron-accounting:
  - confirm paid access prepare/quote/hold/capture/receipt route expectations
  - confirm existing wallet mutation path can support post/comment/article
  - add focused tests if paid action labels/type enums are missing
  - accounting observes paid events after wallet/ledger truth
  - accounting cannot create entitlement
  - accounting cannot mutate ledger

svc-storage + svc-index:
  - confirm post/comment/article assets/manifests can be stored by b3
  - confirm names/pointers/reference graphs resolve correctly
  - confirm storage bytes are not payment truth
  - confirm index pointers are not payment truth
  - add paid metadata only as display/policy input, not entitlement truth

svc-gateway + omnigate:
  - expose or confirm prepare/access/render routes for post/comment/article/content_view
  - preserve existing paid path
  - no direct ledger mutation from gateway/omnigate
  - no fake receipt generation
  - no cache-only unlock
  - source-label responses
```

### Explicitly out of scope

```text id="gzcskq"
CrabLink UI polish
rewarder payout planning
tokenomics TOML overhaul
bridge prep
staking prep
new wallet mutation service
new ledger API bypass
```

### Focused tests

Suggested names:

```text id="n6pr2b"
ron-proto:
  internal_roc_beta_paid_content_dto

ron-ledger:
  internal_roc_beta_paid_content_replay_label

svc-wallet:
  internal_roc_beta_paid_content_receipt_path

ron-accounting:
  internal_roc_beta_paid_content_snapshot_non_authority

svc-storage:
  internal_roc_beta_paid_content_artifact_boundary

svc-index:
  internal_roc_beta_paid_content_pointer_boundary

svc-gateway:
  internal_roc_beta_paid_content_route_boundary

omnigate:
  internal_roc_beta_paid_content_access_boundary
```

### Exit gate

```text id="4g8h8u"
Backend paid post/comment/article/content_view route contracts are clear.
Existing wallet path remains the only mutation path.
No gateway/omnigate direct ledger mutation.
No storage/index payment truth.
No accounting entitlement.
No fake receipts.
No cache-only unlock.
Focused tests green.
Crate-local park/preflight green once.
```

---

## Phase 1 Round 2 — CrabLink paid content UX proof

### Purpose

Wire or confirm CrabLink Tauri can show paid post/comment/article flows with explicit confirmation and backend-derived receipts.

### Primary crate pairs

```text id="b5i0ki"
1. svc-gateway + omnigate
2. CrabLink Tauri + client adapters
```

### Work

```text id="08nfg2"
svc-gateway + omnigate:
  - confirm route response shapes match client expectations
  - confirm errors are redacted/source-labeled
  - confirm paid denial does not leak protected content
  - confirm paid accepted response is backend-derived

CrabLink Tauri + client adapters:
  - paid post flow uses adapter path
  - paid comment flow uses adapter path
  - paid article flow uses adapter path
  - generic content_view uses adapter path if available
  - every spend shows explicit quote
  - cancel does not mutate wallet
  - failure does not unlock content
  - accepted receipt displays as backend-derived
  - balance refresh is backend-derived
  - cache is display-only
```

### Focused checks/tests

Suggested names:

```text id="nxgdgp"
CrabLink:
  check:internal-roc-paid-content-boundary
  check:internal-roc-no-silent-spend
  check:internal-roc-receipt-display-only
  check:internal-roc-paid-cache-boundary

Tauri Rust:
  internal_roc_paid_content_command_boundary
  internal_roc_receipt_redaction_boundary
```

### Exit gate

```text id="l23ivm"
CrabLink can prepare paid post/comment/article access.
CrabLink shows explicit confirmation before spend.
Cancel path does not mutate wallet.
Accepted path displays backend receipt.
Balance refresh is backend-derived.
No raw invoke from arbitrary UI.
No secret/spend authority in React.
No cache-only unlock.
App-local park/check green once.
```

### Phase 1 completion label

```text id="r15zul"
Internal ROC Beta Phase 1 paid content flow proof complete.
```

---

# Phase 2 — Ledger replay + conservation proof

## Goal

Prove paid flows replay deterministically and conserve ROC.

## Ideal rounds

```text id="k5jnat"
1–2 core-heavy rounds
```

---

## Phase 2 Round 1 — replay/conservation core

### Purpose

Make ledger replay and conservation visible enough for beta trust.

### Primary crate pairs

```text id="i6vqu3"
1. ron-proto + ron-ledger
2. svc-wallet + ron-accounting
```

### Crate-pair work

```text id="3dmjv1"
ron-proto + ron-ledger:
  - define or confirm operation/replay report DTOs if needed
  - replay paid image/site/site_visit/post/comment/article/content_view histories
  - prove same accepted history = same balances/receipts/holds
  - prove transfers conserve value
  - prove holds conserve value
  - prove captures/releases conserve value
  - prove burns/remainders go to explicit sink
  - reject duplicate terminal hold operations
  - reject replay ambiguity from DB order/wall clock

svc-wallet + ron-accounting:
  - wallet idempotency for paid actions
  - wallet receipt lookup stable after replay
  - accounting snapshots observe accepted receipts only
  - accounting cannot change replay result
  - accounting cannot create balances/receipts
```

### Required scenarios

```text id="i7m6ws"
paid access accepted
paid access cancelled before mutation
hold opened then captured
hold opened then released
hold opened then expired
duplicate capture rejected
capture after release rejected
capture after expiry rejected
retry after accepted returns same result or safe conflict
ledger replay equals current truth
accounting replay cannot alter balances
```

### Focused tests

Suggested names:

```text id="t2rqvn"
ron-ledger:
  internal_roc_beta_paid_flow_replay_equality
  internal_roc_beta_balance_conservation
  internal_roc_beta_hold_terminality
  internal_roc_beta_replay_order_independence

svc-wallet:
  internal_roc_beta_paid_action_idempotency
  internal_roc_beta_receipt_lookup_after_replay

ron-accounting:
  internal_roc_beta_snapshot_cannot_change_replay
```

### Exit gate

```text id="i1r6xb"
Same accepted operation history produces same balances/receipts/holds.
Paid flow value is conserved.
Hold terminality is enforced.
Retries are safe.
Accounting cannot alter replay.
Focused tests green.
Crate-local park/preflight green once.
```

---

## Phase 2 Round 2 — downstream replay visibility

### Purpose

Expose replay/conservation status as read-only audit/display data without creating new authority.

### Primary crate pairs

```text id="bb0bd2"
1. svc-gateway + omnigate
2. CrabLink Tauri + client adapters
```

### Work

```text id="1oqt9v"
svc-gateway + omnigate:
  - optional read-only replay/conservation status route
  - source-label replay status
  - no unlock from replay status alone
  - no finality overstatement

CrabLink Tauri:
  - optional receipt detail panel shows replay/audit status
  - labels are display-only
  - missing replay status does not fabricate truth
  - stale/offline labels are honest
```

### Exit gate

```text id="wyg623"
Replay/conservation status is display-only.
Paid unlock still depends on backend access truth.
No client-side replay authority.
No gateway/omnigate mutation.
```

### Phase 2 completion label

```text id="13astk"
Internal ROC Beta Phase 2 replay/conservation proof complete.
```

---

# Phase 3 — Accounting → rewarder → wallet payout loop

## Goal

Prove that usage can become bounded payout plans and approved payouts without letting accounting or rewarder mutate ledger truth.

Correct loop:

```text id="ebn5wk"
classified event
→ ron-accounting snapshot/report
→ svc-rewarder capped payout plan
→ ron-policy validation/gating
→ svc-wallet approved payout mutation
→ ron-ledger durable receipt
```

## Ideal rounds

```text id="i20qze"
2 implementation rounds
```

---

## Phase 3 Round 1 — snapshots and non-mutating payout plans

### Purpose

Create deterministic accounting snapshots and reward plans without wallet mutation.

### Primary crate pairs

```text id="u4o3to"
1. ron-proto + ron-ledger
2. svc-wallet + ron-accounting
3. svc-rewarder + ron-policy
```

### Crate-pair work

```text id="vkjzzt"
ron-proto + ron-ledger:
  - DTO support for accounting/reward plan references if missing
  - economic_receipt classification remains wallet/ledger-derived
  - ledger does not accept reward plan as receipt
  - no raw engagement minting

svc-wallet + ron-accounting:
  - accounting ingests/normalizes classified events
  - sealed windows deterministic
  - paid receipt events become snapshot input
  - metering/proof_eligible/ad_budgeted/analytics_only separation
  - accounting cannot mutate balances

svc-rewarder + ron-policy:
  - rewarder consumes sealed snapshots
  - rewarder produces deterministic capped payout plans
  - policy validates reward plan/config
  - no payout execution yet
  - no rewarder direct ledger mutation
  - no policy-created receipt
```

### Required event-class tests

```text id="6kcf67"
economic_receipt can feed accounting as receipt truth
metering cannot directly become payout
analytics_only never becomes reward material
proof_eligible requires verification/caps/policy
ad_budgeted must be funded by explicit budget
```

### Focused tests

Suggested names:

```text id="ebw7ks"
ron-accounting:
  internal_roc_beta_snapshot_determinism
  internal_roc_beta_event_class_isolation
  internal_roc_beta_accounting_non_authority

svc-rewarder:
  internal_roc_beta_reward_plan_determinism
  internal_roc_beta_rewarder_non_mutation
  internal_roc_beta_pool_cap_enforcement

ron-policy:
  internal_roc_beta_reward_plan_policy_gate
```

### Exit gate

```text id="8h5wvz"
Accounting snapshots deterministic.
Reward plans deterministic.
Raw engagement cannot directly mint ROC.
Rewarder cannot mutate ledger.
Policy cannot create receipt/balance/finality.
No payout execution yet unless Round 2 begins.
```

---

## Phase 3 Round 2 — approved payout execution through svc-wallet

### Purpose

Execute approved payout intents only through svc-wallet and prove ledger receipts.

### Primary crate pairs

```text id="sz2q0t"
1. svc-wallet + ron-accounting
2. svc-rewarder + ron-policy
3. ron-proto + ron-ledger
```

### Work

```text id="s34dic"
svc-rewarder + ron-policy:
  - reward plan emits payout intent candidates
  - policy validates/gates payout plan
  - duplicate payout prevention markers
  - caps and category pools enforced

svc-wallet + ron-accounting:
  - approved payout intent sent to svc-wallet only
  - svc-wallet executes issue/transfer as appropriate
  - svc-wallet returns receipt
  - accounting observes payout receipt after ledger truth
  - duplicate payout idempotency tested

ron-proto + ron-ledger:
  - payout receipt replay stable
  - reward execution conserves configured issuance/burn/sink rules
  - no reward plan as ledger truth
```

### Focused tests

Suggested names:

```text id="1305pe"
svc-wallet:
  internal_roc_beta_approved_payout_execution
  internal_roc_beta_duplicate_payout_prevention

ron-ledger:
  internal_roc_beta_payout_receipt_replay
  internal_roc_beta_reward_issuance_conservation

svc-rewarder:
  internal_roc_beta_payout_plan_to_wallet_intent

ron-accounting:
  internal_roc_beta_payout_receipt_observation_only
```

### Exit gate

```text id="yg00pr"
Approved payouts execute only through svc-wallet.
ron-ledger records durable receipt.
Reward plan alone cannot create balance.
Accounting observes but cannot mutate.
Duplicate payout is prevented.
Focused tests green.
Park/preflight green once.
```

### Phase 3 completion label

```text id="13d459"
Internal ROC Beta Phase 3 accounting/rewarder/wallet payout loop proof complete.
```

---

# Phase 4 — CrabLink Tauri wallet/receipt UX hardening

## Goal

Make CrabLink’s wallet/receipt UX trustworthy, clear, and impossible to confuse with backend authority.

## Ideal rounds

```text id="mc9wf7"
1–2 client-heavy rounds
```

---

## Phase 4 Round 1 — receipt and balance UX truth labels

### Purpose

CrabLink must clearly display backend-derived receipts/balances and stale/offline status.

### Primary pair

```text id="83hsch"
CrabLink Tauri + client adapters
```

### Work

```text id="wp7pkx"
CrabLink Tauri:
  - receipt panel labels backend source
  - recent receipts are display-only
  - balance chip uses backend refresh
  - stale balance labels are visible
  - failed refresh is honest
  - paid action receipt detail links to asset/action
  - no local computed balance as truth
  - no local receipt cache as entitlement
```

### Checks

Suggested names:

```text id="my00my"
check:internal-roc-receipt-display-only
check:internal-roc-balance-backend-derived
check:internal-roc-no-cache-entitlement
check:internal-roc-no-silent-spend
```

### Exit gate

```text id="dy4n82"
Receipt display is backend-derived/display-only.
Balance display is backend-derived or stale-labeled.
No cache-only entitlement.
No fake receipt.
No silent spend.
App-local checks green.
```

---

## Phase 4 Round 2 — explicit confirmation and failure UX

### Purpose

Make paid actions understandable and safe.

### Primary pairs

```text id="1sqnic"
1. svc-gateway + omnigate
2. CrabLink Tauri + client adapters
```

### Work

```text id="kl063w"
svc-gateway + omnigate:
  - prepare/quote responses include enough display-safe detail
  - recipient/split labels are safe and bounded
  - errors are redacted/source-labeled
  - denial never leaks protected body

CrabLink Tauri:
  - every spend shows amount
  - every spend shows action/asset
  - every spend shows recipient/split if known
  - every spend has cancel
  - cancel never mutates
  - confirm triggers adapter path only
  - failure does not unlock
  - retry is clear and idempotent/safe
```

### Exit gate

```text id="my46w4"
User cannot spend silently.
User can cancel before mutation.
Failures are honest.
Paid unlock only after backend access truth.
No secrets/spend authority in React.
No raw invoke creep.
```

### Phase 4 completion label

```text id="jzds4w"
Internal ROC Beta Phase 4 CrabLink wallet/receipt UX proof complete.
```

---

# Phase 5 — Tokenomics TOML validation and anti-farming gates

## Goal

Make `configs/roc-economics.toml` the validated mutable economics source of truth and prove raw engagement cannot directly mint ROC.

## Ideal rounds

```text id="imv3sr"
2 implementation rounds
```

---

## Phase 5 Round 1 — TOML schema and validation

### Purpose

Create or harden the canonical economics config schema.

### Primary crate pairs

```text id="94julr"
1. ron-proto + ron-ledger
2. svc-rewarder + ron-policy
3. svc-wallet + ron-accounting
```

### Work

```text id="navxyw"
configs/roc-economics.toml:
  - define canonical internal ROC economics config
  - include schema/version
  - include units and basis-point rules
  - include paid content split defaults
  - include reward pool/category caps
  - include anti-farming caps
  - include rounding mode
  - include remainder sink
  - include bridge placeholders as disabled/inert only if needed
  - include staking placeholders as disabled/inert only if needed

ron-proto + ron-ledger:
  - money/split DTO support if required
  - no floats
  - no malformed minor-unit money
  - no config-derived receipt truth

ron-policy:
  - validate TOML shape
  - reject unknown fields
  - reject floats
  - reject invalid bps totals
  - reject missing remainder sink
  - reject enabled bridge/staking config unless future gate allows it

svc-rewarder:
  - consume validated config for planning only
  - no hard-coded payout constants

svc-wallet + ron-accounting:
  - confirm config cannot directly mutate wallet/ledger
  - accounting labels config version only
```

### Focused tests

Suggested names:

```text id="ds8syo"
ron-policy:
  internal_roc_beta_economics_toml_valid
  internal_roc_beta_economics_toml_rejects_unknown_fields
  internal_roc_beta_economics_toml_rejects_floats
  internal_roc_beta_economics_toml_rejects_invalid_bps
  internal_roc_beta_economics_toml_requires_remainder_sink
  internal_roc_beta_bridge_config_inert
  internal_roc_beta_staking_config_inert

svc-rewarder:
  internal_roc_beta_no_hardcoded_payout_constants
  internal_roc_beta_config_driven_planning_only
```

### Exit gate

```text id="vfke4a"
configs/roc-economics.toml exists or current config path is confirmed.
TOML validation rejects unsafe values.
No floats.
BPS totals validated.
Remainder sink explicit.
Bridge/staking fields inert if present.
No hard-coded payout constants in business logic.
Focused tests green.
```

---

## Phase 5 Round 2 — anti-farming and event-class gates

### Purpose

Prove raw engagement cannot directly mint or allocate protocol ROC.

### Primary crate pairs

```text id="sqdy1i"
1. ron-accounting + svc-rewarder
2. ron-policy + svc-rewarder
3. svc-ads if ad-budgeted flow is active
```

### Work

```text id="f71p5d"
ron-accounting:
  - isolate analytics_only
  - isolate metering
  - mark proof_eligible only after rules
  - preserve economic_receipt separation

svc-rewarder:
  - consume only eligible/capped inputs
  - enforce category pool caps
  - enforce per-account/site/content/epoch caps
  - deterministic reward point conversion
  - deterministic rounding/remainder handling

ron-policy:
  - gate reward plan eligibility
  - reject uncapped raw engagement
  - reject analytics_only reward material
  - reject metering direct payout

svc-ads, if included:
  - ad_budgeted events must have payer-authorized budget
  - ad impressions/clicks do not mint protocol ROC
```

### Focused tests

Suggested names:

```text id="xjjl74"
ron-accounting:
  internal_roc_beta_analytics_only_quarantine
  internal_roc_beta_metering_not_payout
  internal_roc_beta_proof_eligible_requires_verification

svc-rewarder:
  internal_roc_beta_antifarming_caps
  internal_roc_beta_reward_point_determinism
  internal_roc_beta_ad_budgeted_not_protocol_emission

ron-policy:
  internal_roc_beta_policy_rejects_raw_engagement_payout
```

### Exit gate

```text id="0akvqy"
Raw engagement cannot directly mint/allocate ROC.
analytics_only never becomes payout material.
metering never directly becomes payout.
proof_eligible requires verification/caps/policy.
ad_budgeted uses explicit budget.
Reward plans remain non-mutating.
Focused tests green.
Park/preflight green once.
```

### Phase 5 completion label

```text id="hctbk7"
Internal ROC Beta Phase 5 tokenomics config/anti-farming proof complete.
```

---

# Phase 6 — Reproducible internal ROC beta smoke suite

## Goal

Create a repeatable dev/local smoke suite proving the full internal ROC beta loop without external settlement, staking, bridge, liquidity, or production endpoints.

## Ideal rounds

```text id="8fipyx"
1 integration round
```

---

## Phase 6 Round 1 — beta green gate

### Purpose

Prove the whole internal loop in a safe dev environment.

### Primary scope

```text id="w7982x"
RustyOnions local/dev stack
CrabLink Tauri dev app
smoke scripts
focused route contracts
receipt/replay/audit checks
```

### Smoke suite should prove

```text id="vqzucd"
local/dev stack starts
creator wallet exists
visitor wallet exists
creator publishes post/comment/article or selected paid asset
visitor prepares paid access
visitor sees quote
visitor explicitly confirms
svc-wallet commits mutation
ron-ledger records receipt
paid content unlocks from backend truth
visitor balance refreshes
creator balance refreshes
receipt displays in CrabLink
ledger replay equals current truth
accounting snapshot seals
rewarder creates non-mutating payout plan
policy validates/gates payout plan
approved payout executes through svc-wallet if enabled for smoke
final receipts display correctly
```

### Safety constraints

```text id="ac0zdp"
dev-only wallets
dev-only config
bounded test values
no production endpoints
no external gas
no ROX
no Solana
no bridge
no staking
no liquidity
no exchange-facing calls
no irreversible external actions
```

### Suggested scripts/checks

```bash id="cvkal3"
scripts/internal-roc-beta-smoke.sh
scripts/internal-roc-beta-replay-check.sh
scripts/internal-roc-beta-tokenomics-check.sh
scripts/internal-roc-beta-crablink-check.sh
```

CrabLink side:

```text id="ru4ab9"
npm --prefix apps/crablink-tauri run check:internal-roc-beta
npm --prefix apps/crablink-tauri run check:internal-roc-paid-cache-boundary
npm --prefix apps/crablink-tauri run check:internal-roc-no-silent-spend
```

Rust side examples:

```text id="ju9emt"
cargo test -p ron-ledger --test internal_roc_beta_replay_conservation
cargo test -p svc-wallet --test internal_roc_beta_paid_content_receipts
cargo test -p ron-accounting --test internal_roc_beta_snapshot_non_authority
cargo test -p svc-rewarder --test internal_roc_beta_reward_plan_non_authority
cargo test -p ron-policy --test internal_roc_beta_economics_config
```

### Exit gate

```text id="6xm7cd"
Internal ROC beta smoke suite is reproducible.
Paid content flow works end-to-end.
Receipts are backend-derived.
Balances are backend-derived.
Replay/conservation passes.
Accounting snapshot seals.
Rewarder plan is non-mutating.
Approved payout path uses svc-wallet only.
CrabLink displays truth without owning it.
No bridge/staking/liquidity/external settlement behavior.
```

### Phase 6 completion label

```text id="9b89f1"
Internal ROC Beta Phase 6 reproducible smoke suite complete.
```

---

# Final beta acceptance gate

Internal ROC beta value-loop proof is complete only when:

```text id="fs7852"
Phase 0 complete:
  safe docs and language locked

Phase 1 complete:
  repeatable paid post/comment/article/content_view flows work through existing wallet path

Phase 2 complete:
  ledger replay and conservation are proven

Phase 3 complete:
  accounting → rewarder → policy → wallet payout loop is proven

Phase 4 complete:
  CrabLink receipt/balance/confirmation UX is truthful and safe

Phase 5 complete:
  tokenomics TOML validation and anti-farming gates are proven

Phase 6 complete:
  reproducible internal ROC beta smoke suite is green
```

Global acceptance must also prove:

```text id="w20p0k"
No new ledger mutation paths.
No fake balances.
No fake receipts.
No fake finality.
No silent spend.
No cache-only paid unlock.
No raw engagement direct ROC minting.
No accounting balance truth.
No rewarder ledger mutation.
No policy receipt/balance/finality truth.
No gateway/omnigate ledger mutation.
No index/storage payment truth.
No CrabLink wallet authority.
No bridge runtime.
No staking runtime.
No liquidity.
No external settlement.
```

Safe final label:

```text id="e7lyxg"
Internal ROC beta value-loop proof is COMPLETE / GREEN / PARKED.
```

Still not allowed to say:

```text id="u78jta"
ROX ready
bridge ready
staking ready
external settlement ready
public chain ready
exchange ready
```

---

# First coding batch recommendation

After this buildplan is reviewed and accepted, the first coding batch should be:

```text id="snskeg"
Internal ROC Beta Phase 1 Round 1 — backend route and authority proof for repeatable paid post/comment/article/content_view flows.
```

Recommended first attachment set:

```text id="nr1u29"
1. POST_QUICKCHAIN_DECISION_GATE.md
2. INTERNAL_ROC_BETA_VALUE_LOOP_BLUEPRINT.md
3. INTERNAL_ROC_BETA_BUILDPLAN.md
4. INTERNAL_ROC_MODEL.md
5. CODEBUNDLE_RS.md or focused bundles for:
   - svc-gateway
   - omnigate
   - svc-wallet
   - ron-ledger
   - svc-storage
   - svc-index
6. CRABLINK-NOTES.MD if client flow context is needed
7. CODEBUNDLE_TAURI_APP.md only when CrabLink work begins
```

Recommended first crate focus:

```text id="da76gd"
svc-gateway + omnigate first if paid post/comment/article routes are mostly routing/hydration.
svc-wallet + ron-ledger first if receipt/payment type support is missing.
svc-storage + svc-index first if post/comment/article artifact/index support is missing.
```

Do not guess. Inspect code before choosing.

---

# Grok / external reviewer checklist

Ask reviewers:

```text id="wzreye"
1. Does this buildplan correctly implement the decision gate?
2. Does it keep bridge/staking/liquidity out of active runtime?
3. Does it preserve staking as dormant/off by default?
4. Does it avoid new ledger mutation paths?
5. Does the crate-pair order make sense?
6. Are Phase 1 paid post/comment/article flows the right first coding target?
7. Are replay/conservation gates strong enough?
8. Are accounting/rewarder non-authority gates strong enough?
9. Are tokenomics TOML validation gates strong enough?
10. Are anti-farming gates strong enough?
11. Is the smoke suite safe by default?
12. Are any phases too broad?
13. Are any missing crates needed early?
14. Does any wording accidentally imply ROX/Solana/bridge/staking/liquidity authorization?
15. What focused tests should be added before the first coding batch?
```

---

# Final buildplan statement

The internal ROC beta build mission is:

```text id="f1suvd"
Use the fewest practical rounds to prove a closed internal ROC economy: paid content flows through explicit confirmation and svc-wallet only; ron-ledger records durable truth; paid access unlocks only from backend receipt/access truth; balances refresh from backend truth; replay and conservation prove correctness; accounting snapshots remain non-authoritative; rewarder plans capped payouts; policy gates those plans; approved payouts execute only through svc-wallet; tokenomics are config-driven; CrabLink displays truth without owning it; bridge, staking, liquidity, and external settlement remain off.
```

<!-- ============================================================ -->
<!-- END SOURCE FILE: docs/roadmap/INTERNAL_ROC_BETA_BUILDPLAN.md -->
<!-- ============================================================ -->


<!-- ============================================================ -->
<!-- BEGIN SOURCE FILE: docs/roadmap/POST_QUICKCHAIN_DECISION_GATE.md -->
<!-- ============================================================ -->

# SOURCE FILE: docs/roadmap/POST_QUICKCHAIN_DECISION_GATE.md

# POST_QUICKCHAIN_DECISION_GATE.md — Post-QuickChain Decision Gate and Safe Scope Lock

RO:WHAT — Defines the official post-QuickChain Phase 5 decision gate for RustyOnions / CrabLink and locks the safe transition from QuickChain boundary/preflight completion into internal ROC beta value-loop proof.

RO:WHY — QuickChain’s authorized Phase 0–5 boundary/preflight scope is complete, but the project must not confuse that with a public chain launch, ROX launch, bridge launch, staking launch, liquidity launch, or external settlement runtime.

RO:INTERACTS — QUICKCHAIN_REVIEW_BUNDLE.MD, QUICKCHAIN_BUILDPLAN.MD, INTERNAL_ROC_MODEL.md, ron-proto, ron-ledger, svc-wallet, ron-accounting, svc-rewarder, svc-storage, svc-gateway, omnigate, svc-index, ron-policy, svc-ads, CrabLink Tauri, future ROX/bridge/staking documents.

RO:INVARIANTS — Internal ROC first; wallet/ledger truth; no fake balances, fake receipts, fake finality, or silent spend; no new ledger mutation paths; no public bridge/staking/liquidity/external settlement until explicitly authorized by a later reviewed phase.

RO:METRICS — Future decision-gate metrics may track beta paid-flow proofs, wallet receipt proofs, ledger replay proofs, balance conservation checks, tokenomics config validation, payout planning audits, and forbidden-scope checker coverage.

RO:CONFIG — Internal ROC beta uses configs/roc-economics.toml for mutable economics values. Bridge and staking config placeholders may exist only as inert/off-by-default future fields until a later phase explicitly authorizes runtime behavior.

RO:SECURITY — CrabLink remains display/user intent only; svc-wallet remains mutation front-door; ron-ledger remains durable economic truth; no client/gateway/omnigate/accounting/rewarder/policy/index/storage authority over balances, receipts, settlement, finality, bridge mint/burn, staking, or liquidity.

RO:TEST — This decision gate should be enforced by docs checks, forbidden-language checks, boundary tests, paid-flow smokes, replay/conservation tests, no-authority tests, tokenomics TOML validation, and future beta green-gate scripts.

---

## INTERNAL-ROC-PHASE6-SAFE-LABEL

Current safe project label:

```text
Internal ROC Beta Phase 6: COMPLETE / GREEN / PARKED.
Internal ROC beta value-loop proof: COMPLETE / GREEN / PARKED.
Current workstream: Internal ROC Beta Stabilization / Product Beta Readiness.
Bridge work: docs / threat-model / decision-gate only.
No ROX/Solana/bridge/staking/liquidity/external settlement runtime.
```

This label prevents older carry-over notes from reopening parked Internal ROC Beta implementation phases or authorizing external runtime scope.

## 0. Status

QuickChain’s current authorized boundary/preflight scope is complete through Phase 5.

Safe project label:

```text
QuickChain boundary/preflight scope is COMPLETE / GREEN / PARKED through Phase 5.
Internal ROC value-plane beta proof is now the active priority.
External settlement, ROX, Solana, public bridge, staking, liquidity, exchange-facing logic, and public validator economy remain deferred.
```

This document is a decision gate.

It is not a code patch.

It is not a bridge implementation plan.

It is not a staking implementation plan.

It is not a ROX launch plan.

It is not a Solana runtime plan.

It is not legal, tax, or financial advice.

It exists to prevent scope drift while the project moves from QuickChain boundary/preflight work into internal ROC beta proof.

---

## 1. Why this decision gate exists

QuickChain Phase 0–5 work created strong safety boundaries around:

```text
DTO strictness
canonicalization
replay posture
receipt/status wording
wallet/ledger truth
accounting non-authority
rewarder non-authority
storage/index/policy/gateway/omnigate/client non-authority
anchor-only evidence posture
DA/archive/challenge fallback posture
external integration posture boundaries
```

That is a major milestone.

But it must be described precisely.

QuickChain completion does not mean:

```text
public blockchain live
QuickChain production chain live
ROX live
Solana live
public bridge live
external settlement live
staking live
liquidity live
exchange-facing runtime live
public validator economy live
CrabLink chain authority live
gateway settlement authority live
omnigate settlement authority live
client paid-unlock authority live
```

Correct meaning:

```text
The planned QuickChain safety, boundary, posture, and preflight sweep is complete through the currently defined Phase 0–5 buildplan.
```

---

## 2. Authority order after QuickChain Phase 5

If documents conflict, use this authority order:

```text
1. QUICKCHAIN_REVIEW_BUNDLE.MD
2. QUICKCHAIN_BUILDPLAN.MD
3. INTERNAL_ROC_MODEL.md
4. This POST_QUICKCHAIN_DECISION_GATE.md
5. Current repository source
6. Latest terminal output
7. Final crate / CrabLink notes
8. ALLNOTES.MD and older carry-over notes
9. Focused codebundles
```

Rules:

```text
Do not let older notes reopen completed QuickChain Phase 5 work.
Do not let older notes authorize bridge/runtime/staking/liquidity scope.
Do not treat “QuickChain complete” as “public chain live.”
Do not treat “anchor” as “bridge.”
Do not treat “bond” as public staking.
Do not treat “receipt display” as receipt authority.
Do not treat “accounting snapshot” as balance truth.
Do not treat “reward plan” as wallet mutation.
```

---

## 3. Active priority decision

The next active project priority is:

```text
Internal ROC beta value-loop proof.
```

This means proving that RustyOnions / CrabLink can run a usable internal ROC economy with:

```text
paid content creation
paid content access
explicit user confirmation
backend wallet mutation
durable ledger truth
backend receipts
paid unlock enforcement
backend-derived balance refresh
accounting snapshots
bounded reward planning
approved payout execution through svc-wallet
CrabLink display-only receipt UX
ledger replay and conservation proof
```

This should become the next active blueprint/buildplan family:

```text
docs/blueprints/INTERNAL_ROC_BETA_VALUE_LOOP_BLUEPRINT.md
docs/buildplans/INTERNAL_ROC_BETA_BUILDPLAN.md
```

---

## 4. Active allowed scope

Allowed active scope after this decision gate:

```text
internal ROC paid-flow polish
paid post/comment/article flows
paid site visit proof
paid content view proof
wallet receipt UX
backend-derived balance refresh UX
ledger replay proof
hold/capture/release conservation audit
paid unlock path audit
receipt truth audit
config/tokenomics TOML validation
reward plan non-authority audit
accounting snapshot audit
creator payout planning
approved payout execution through svc-wallet only
CrabLink receipt display polish
CrabLink explicit confirmation polish
beta green-gate smoke suite
route contract documentation
developer docs
```

Safe first coding focus:

```text
Repeatable paid post / comment / article flows using the existing prepare/quote → explicit confirmation → backend wallet path → backend receipt → unlock/render → balance refresh path.
```

The first coding focus must not invent a new economic mutation path.

---

## 5. Active forbidden scope

Forbidden in active post-QuickChain internal beta work:

```text
ROX active runtime
Solana active runtime
Solana Anchor program code
bridge mint/burn code
bridge custody
bridge settlement
external settlement
exchange integrations
liquidity
liquidity pools
market making
public validator economy
public staking runtime
yield-bearing staking runtime
delegated staking runtime
liquid staking
staking derivatives
client-side bridge authority
CrabLink bridge authority
gateway bridge authority
omnigate bridge authority
policy bridge authority
index bridge authority
rewarder wallet mutation
accounting wallet mutation
policy wallet mutation
gateway wallet mutation
omnigate wallet mutation
index as payment truth
storage as payment truth
cache-only paid unlock
client-side payout truth
fake receipts
fake balances
fake finality
silent spend
raw engagement protocol minting
```

---

## 6. No new ledger mutation paths rule

This is the most important post-QuickChain rule:

```text
No new ledger mutation paths may be introduced during internal ROC beta work.
```

Allowed mutation path remains:

```text
user intent
→ prepare/quote
→ explicit confirmation
→ svc-wallet
→ ron-ledger
→ backend receipt
→ backend access decision
→ paid enforcement/render
→ display-only receipt cache
→ backend-derived balance refresh
```

Forbidden mutation shortcuts:

```text
CrabLink → ron-ledger
CrabLink → accounting balance update
CrabLink → rewarder payout execution
gateway → ron-ledger
omnigate → ron-ledger
policy → ron-ledger
index → ron-ledger
storage → ron-ledger
accounting → ron-ledger balance mutation
rewarder → ron-ledger balance mutation
external evidence → wallet mutation
external evidence → paid unlock
cache → paid unlock
```

---

## 7. Staking capability policy

The project may preserve staking as a long-term dormant capability in the architecture, but staking must remain off by default.

This decision gate distinguishes four concepts:

```text
1. Internal ROC paid-use economy
2. Internal validator safety bonds / slashable reserves
3. Future public staking or delegated staking
4. Future bridge / external settlement
```

Internal validator safety bonds are not public staking.

Internal validator safety bonds may exist only as bounded internal accountability mechanisms when explicitly authorized by the relevant internal validator/bonding phase.

Public staking, delegated staking, yield-bearing staking, liquid staking, staking derivatives, and staking marketed as passive income are not active scope.

Current staking status:

```text
Future/dormant capability only.
Off by default.
Not active runtime.
Not user-facing.
Not yield-bearing.
Not bridge-enabled.
Not exchange-facing.
Not liquidity-facing.
Not production-enabled.
```

Allowed now:

```text
docs-only staking parking notes
risk notes
legal/regulatory review checklist
feature flag naming discussion
config placeholder discussion
explicit disabled-by-default posture
tests proving staking remains disabled
tests proving no staking UI/runtime is exposed
tests proving no staking reward accrual occurs
tests proving no staking ledger mutation path exists
```

Forbidden now:

```text
staking runtime
staking UI
staking APR/APY display
staking reward accrual
delegated staking
pooled staking
liquid staking
staking derivatives
bridge-connected staking
external-chain staking
auto-staking
staking enabled by default
staking marketing copy
staking-based paid unlock
staking-based wallet mutation
staking-based validator admission in public runtime
staking config that activates behavior
```

Future activation gates for any staking capability:

```text
separate staking blueprint
separate staking buildplan
legal/regulatory review
tax/product language review
risk disclosure review
governance controls
halt/pause controls
no-yield-claim review
independent security audit
abuse/farming analysis
wallet/ledger conservation analysis
clear distinction from validator safety bonds
explicit user opt-in
default disabled in every environment
feature flag default false
runtime config default false
CI proving default-disabled posture
```

Default rule:

```text
If staking exists in docs or config, it must be inert.
If staking exists in code later, it must be feature-gated, config-disabled, UI-hidden, and non-operational by default.
If there is any doubt, staking remains off.
```

---

## 8. Bridge parking lot

The bridge remains parked.

Bridge means any bidirectional ROC/ROX path such as:

```text
ROC → ROX
ROX → ROC
```

Possible future bridge concepts may include:

```text
burn/mint
lock/release
challenge windows
halt/pause controls
audit events
future Solana Anchor program
future external contract/program
user-paid gas
```

But none of those are active implementation scope.

Current bridge status:

```text
Future/deferred bridge design only.
No bridge code.
No bridge runtime.
No bridge mint/burn.
No Solana runtime.
No ROX runtime.
No external settlement.
No bridge custody.
No exchange-facing logic.
No liquidity.
No public staking.
```

Allowed now:

```text
short parking-lot notes
future prerequisite list
risk list
explicit deferred status
```

Not allowed yet:

```text
full bridge implementation plan as active buildplan
smart contract code
Solana Anchor program code
bridge DTO runtime code
bridge wallet mutation code
bridge gateway routes
bridge UI
bridge receipts
bridge finality claims
bridge custody logic
bridge economic activation
```

A full bridge threat model may be drafted later, after the internal ROC beta blueprint and buildplan are stable.

---

## 9. Internal ROC beta proof requirements

The internal ROC beta value loop should prove:

```text
a creator can publish content
a visitor can prepare paid access
the user sees an explicit quote
the user explicitly confirms
svc-wallet performs the mutation
ron-ledger records durable truth
a backend receipt is returned
paid unlock uses backend receipt/access truth
CrabLink displays the receipt as display-only
balances refresh from backend truth
the ledger can replay the result
conservation holds
accounting can snapshot events
rewarder can plan payouts without mutation
approved payout intents execute only through svc-wallet
```

The proof should be repeated across at least:

```text
image
site
site_visit
post
comment
article
content_view
```

Media-heavy flows may be phased, but must preserve bounded media and paid/cache boundaries.

---

## 10. Tokenomics config direction

The next active buildplan should harden the economics config path.

Mutable economics values belong in:

```text
configs/roc-economics.toml
```

This includes:

```text
payout amounts
reward weights
split defaults
burn rates
epoch pool category percentages
reward point conversion parameters
quality/reputation/scarcity multipliers
anti-farming caps
max spend limits
hold multipliers
rounding mode
remainder sink
future bridge placeholders
future staking placeholders
```

Hard rules:

```text
No hard-coded payout amounts in business logic.
No float money.
No raw engagement direct protocol minting.
No config field can create wallet/ledger truth by itself.
No config field can enable bridge by accident.
No config field can enable staking by accident.
No config field can create paid unlock authority.
```

Future bridge and staking config placeholders must remain inert until explicitly authorized by later phases.

---

## 11. CrabLink Tauri boundary

CrabLink Tauri remains the primary client/product layer.

CrabLink may:

```text
display backend-derived balances
display backend-derived receipts
display paid access status
collect user intent
show prepare/quote information
ask for explicit confirmation
call TypeScript adapters
request bounded Tauri commands
render paid content after backend access truth
verify b3 cache before trusted render
display QuickChain/readiness status
display internal ROC beta proof status
```

CrabLink must not:

```text
hold wallet truth
hold ledger truth
invent balances
invent receipts
invent finality
invent paid access
unlock paid content from cache alone
mutate ledger
mutate wallet
run bridge mint/burn
run staking
hold private keys/seeds/raw capabilities in React
store secrets in localStorage
store secrets in URLs
store secrets in logs
act as chain authority
act as bridge authority
act as staking authority
act as settlement authority
```

---

## 12. Recommended next documents

After this decision gate is reviewed and accepted, create:

```text
docs/blueprints/INTERNAL_ROC_BETA_VALUE_LOOP_BLUEPRINT.md
docs/buildplans/INTERNAL_ROC_BETA_BUILDPLAN.md
```

Do not create active bridge or staking buildplans yet.

Allowed later after internal ROC beta docs are stable:

```text
docs/bridges/ROC_ROX_BRIDGE_THREAT_MODEL_DEFERRED.md
docs/staking/STAKING_CAPABILITY_PARKING_LOT.md
```

Do not create:

```text
ROC_ROX_BRIDGE_BUILDPLAN.md
STAKING_BUILDPLAN.md
ROX_RUNTIME_BUILDPLAN.md
SOLANA_ANCHOR_BUILDPLAN.md
LIQUIDITY_BUILDPLAN.md
EXCHANGE_BUILDPLAN.md
```

unless a later reviewed decision gate explicitly authorizes them.

---

## 13. Proposed internal ROC beta phase outline

The next buildplan should probably use this phase sequence:

```text
Phase 0 — Post-QuickChain doc sync + safe language audit
Phase 1 — Repeatable paid content flows using existing wallet path
Phase 2 — Ledger replay + conservation proof
Phase 3 — Accounting → rewarder → wallet payout loop
Phase 4 — CrabLink Tauri wallet/receipt UX hardening
Phase 5 — Tokenomics TOML validation and anti-farming gates
Phase 6 — Reproducible internal ROC beta smoke suite
```

Bridge and staking stay out of this phase list except as forbidden/deferred scope.

---

## 14. Acceptance gates for this decision document

This document is accepted when reviewers agree that:

```text
QuickChain completion wording is safe.
Internal ROC beta is the active priority.
No new ledger mutation paths are allowed.
Bridge is parked.
Staking is preserved only as a dormant/off-by-default future capability.
CrabLink remains display/user intent only.
Wallet/ledger remain economic truth.
Tokenomics config is the mutable economics source of truth.
No public bridge/staking/liquidity/exchange-facing scope is authorized.
The next two active docs are the internal ROC beta blueprint and buildplan.
```

---

## 15. Suggested docs-only grep checks

Optional docs-only checks after adding this file:

```bash
grep -n "QuickChain boundary/preflight scope is COMPLETE / GREEN / PARKED through Phase 5" docs/roadmap/POST_QUICKCHAIN_DECISION_GATE.md
grep -n "Internal ROC value-plane beta proof is now the active priority" docs/roadmap/POST_QUICKCHAIN_DECISION_GATE.md
grep -n "No new ledger mutation paths" docs/roadmap/POST_QUICKCHAIN_DECISION_GATE.md
grep -n "Future/dormant capability only" docs/roadmap/POST_QUICKCHAIN_DECISION_GATE.md
grep -n "Off by default" docs/roadmap/POST_QUICKCHAIN_DECISION_GATE.md
grep -n "Future/deferred bridge design only" docs/roadmap/POST_QUICKCHAIN_DECISION_GATE.md
```

Optional forbidden-language review:

```bash
grep -n "public chain live\|bridge live\|staking live\|liquidity live\|exchange ready" docs/roadmap/POST_QUICKCHAIN_DECISION_GATE.md
```

Any match must be in an explicitly negative/forbidden context.

---

## 16. Grok / external reviewer checklist

Ask external reviewers to check:

```text
1. Does this document correctly separate QuickChain boundary/preflight completion from public chain launch?
2. Does it keep the active project focused on internal ROC beta proof?
3. Does it block accidental bridge implementation?
4. Does it preserve future staking optionality while keeping staking off by default?
5. Does it clearly distinguish validator safety bonds from public staking?
6. Does it prevent new ledger mutation paths?
7. Does it keep CrabLink as display/user intent only?
8. Does it properly defer ROX/Solana/external settlement?
9. Does it properly defer liquidity/exchange-facing logic?
10. Does it provide the right next-doc sequence?
```

---

## 17. Final decision statement

The project decision after QuickChain Phase 5 is:

```text
QuickChain boundary/preflight scope is complete through Phase 5.
RustyOnions / CrabLink now moves to internal ROC beta value-loop proof.
The active goal is to prove paid creation, paid access, wallet receipts, ledger replay, balance conservation, accounting snapshots, reward planning, approved payout execution, and CrabLink receipt UX using existing authority boundaries.
Bridge, ROX, Solana, staking, liquidity, exchange-facing logic, and public validator economy remain deferred.
Staking may remain a future dormant capability, but it is off by default and not active runtime.
No new ledger mutation paths are authorized.
```

<!-- ============================================================ -->
<!-- END SOURCE FILE: docs/roadmap/POST_QUICKCHAIN_DECISION_GATE.md -->
<!-- ============================================================ -->


<!-- ============================================================ -->
<!-- BEGIN SOURCE FILE: docs/roadmap/POST_INTERNAL_ROC_BRIDGE_DECISION_GATE.md -->
<!-- ============================================================ -->

# SOURCE FILE: docs/roadmap/POST_INTERNAL_ROC_BRIDGE_DECISION_GATE.md

# POST_INTERNAL_ROC_BRIDGE_DECISION_GATE.md — ROC ↔ ROX Bridge Planning Gate

RO:WHAT — Defines the safe post-Internal-ROC decision gate for ROC ↔ ROX / Solana bridge planning.

RO:WHY — Allows bridge documentation, threat modeling, DTO sketches, proof-model design, and review without activating ROX, Solana, bridge runtime, staking, liquidity, or external settlement.

RO:INTERACTS — PHASE6_CLOSEOUT.md, POST_QUICKCHAIN_DECISION_GATE.md, INTERNAL_ROC_BETA_VALUE_LOOP_BLUEPRINT.md, INTERNAL_ROC_BETA_BUILDPLAN.md, INTERNAL_ROC_MODEL.md, ROC_ROX_SOLANA_ANCHOR_BRIDGE_BLUEPRINT.md, ROC_ROX_SOLANA_BRIDGE_BUILDPLAN.md, ROC_ROX_BRIDGE_THREAT_MODEL.md, ron-proto, ron-ledger, svc-wallet, ron-policy, svc-interop, ron-audit, CrabLink Tauri, future Solana Anchor program docs.

RO:INVARIANTS — Internal ROC truth remains svc-wallet + ron-ledger; bridge scope is docs/threat-model only; burn/mint is preferred over custody/swap/liquidity; no runtime is authorized; no client/gateway/omnigate/accounting/rewarder/policy/index/storage bridge authority.

RO:METRICS — Future bridge-planning metrics may track threat-model coverage, docs v2 checker status, forbidden-runtime language checks, nonce/proof/finality test-plan coverage, pause/halt drill coverage, and Solana/Anchor risk coverage.

RO:CONFIG — Bridge config placeholders may exist only as inert, disabled-by-default, non-operational fields. Any future enablement requires caps, pause/halt defaults, challenge windows, per-user limits, cluster binding, mint binding, and separate reviewed authorization.

RO:SECURITY — No active ROX token, Solana deployment, bridge mint/burn runtime, external settlement, staking, liquidity, exchange-facing logic, public validator economics, or client-side bridge authority.

RO:TEST — Enforced by docs-only checkers, forbidden-scope scanners, threat-model review, nonce/replay test-plan requirements, multi-RPC proof-model requirements, Anchor account-constraint test-plan requirements, and UI language restrictions.

---

## 0. Safe current status

```text
Internal ROC Beta Phase 6: COMPLETE / GREEN / PARKED.
Internal ROC beta value-loop proof: COMPLETE / GREEN / PARKED.
Current workstream: Internal ROC Beta Stabilization / Product Beta Readiness.
Bridge work: docs / threat-model / decision-gate only.
No ROX/Solana/bridge/staking/liquidity/external settlement runtime.
```

This decision gate authorizes only planning artifacts.

It does not authorize code that executes a bridge.

It does not authorize a Solana program deployment.

It does not authorize a ROX token launch.

It does not authorize liquidity, staking, exchange integrations, or public validator economics.

---

## 1. Authorized scope

Allowed now:

```text
bridge docs
bridge threat model
bridge buildplan
bridge blueprint
review prompts
DTO sketches
state-machine sketches
config placeholder documentation
nonce / replay design
proof / finality design
pause / halt design
recovery design
Anchor account model notes
Anchor account constraint test plans
Solana Token Program risk notes
UI language rules
forbidden-scope checkers
```

Not allowed now:

```text
ROX live token
Solana mainnet deployment
Solana devnet value-bearing deployment
bridge mint runtime
bridge burn runtime
bridge relayer runtime
external settlement runtime
liquidity pool
DEX integration
CEX integration
staking
yield
exchange-facing UI
client-side finality authority
gateway bridge authority
omnigate bridge authority
direct ledger mutation outside svc-wallet
```

---

## 2. Doctrine lock

The bridge must never weaken this order:

```text
user intent
→ prepare / quote
→ explicit confirmation
→ svc-wallet
→ ron-ledger
→ durable internal receipt
→ proof / challenge / finality model
→ external action only if separately authorized
```

Internal ROC remains the truth plane.

ROX, if later authorized, is an external representation created only through a reviewed burn/mint lifecycle.

The bridge must be modeled as:

```text
irreversible internal burn
→ delayed proof / challenge / finality
→ external mint
```

or:

```text
irreversible external burn
→ delayed proof / challenge / finality
→ internal issue through svc-wallet
```

The bridge must not be modeled as:

```text
instant conversion
exchange
trade
swap
custodial IOU
guaranteed value
guaranteed redemption
client-side transfer
gateway-side ledger mutation
omnigate-side settlement
```

---

## 3. Anchor terminology lock

The word "Anchor" has two different meanings and must not drift.

```text
QuickChain anchor
→ evidence/checkpoint/posture artifact
→ does not mutate ROC balances
→ does not mint ROX
→ does not settle value
```

```text
Solana Anchor program
→ possible future Solana smart contract/program framework
→ not active now
→ not deployed now
→ not authorized by this decision gate
```

Any future Solana document must explicitly state which meaning is being used.

---

## 4. Mandatory v2 bridge fixes

Before bridge docs may be used for implementation planning, the bridge package must include:

```text
cross-domain nonce binding
multi-RPC / quorum proof model
minimum Solana commitment policy
cluster binding
program-id binding
mint binding
direction binding
operation-id binding
Solana transaction signature binding
challenge window griefing mitigation
halt semantics for pending finalizations
coordinator compromise drills
key rotation ceremony
upgrade authority policy
stuck challenge recovery
recovery-account issue path for failed ROX→ROC internal issue
CPI and token-program account validation
Anchor account constraint test plan
verifiable build / reproducible deployment plan
CrabLink stale/offline status language
strict UI forbidden wording
```

---

## 5. Bridge launch prerequisites

No bridge runtime may begin until all are true:

```text
Internal ROC Phase 6 closeout exists
bridge decision gate accepted
bridge blueprint v2 accepted
bridge buildplan v2 accepted
bridge threat model accepted
proof/finality model accepted
cross-domain nonce test matrix accepted
pause/halt/recovery drills documented
Anchor account constraint test matrix accepted
upgrade authority and key rotation ceremony documented
UI language and risk disclaimer templates accepted
legal/product review completed where applicable
separate runtime authorization issued
```

---

## 6. Safe public language

Allowed language for UI/docs:

```text
planned bridge
experimental bridge design
burn request
external mint request
pending verification
pending challenge window
not final
delayed
risk of failure
manual review may be required
```

Forbidden language for UI/docs:

```text
convert
exchange
trade
swap
instant
guaranteed
cash out
redeem guaranteed value
risk-free
final before backend proof
final before challenge close
```

---

## 7. Decision

This file approves:

```text
bridge docs and threat model work
```

This file rejects:

```text
bridge runtime work
```

Safe label:

```text
ROC ↔ ROX bridge: DOCS / THREAT-MODEL ONLY.
```

---

## NO-VALUE-BEARING-DEVNET-PHASE9-GATE

No value-bearing devnet, toy-value devnet, public bridge demo, external mint/burn, or user-facing ROX bridge path may begin until Bridge Phase 9 audit/recovery drills are COMPLETE / GREEN / PARKED and a separate runtime decision gate authorizes the next phase.

This applies even if docs, local-validator skeletons, read-only devnet shadow observation, or toy simulations are green.

This gate preserves the active boundary:

```text
Bridge work: docs / threat-model / decision-gate only.
No ROX/Solana/bridge/staking/liquidity/external settlement runtime.
No value-bearing devnet until Phase 9 audit/recovery drills pass and a later decision gate explicitly authorizes runtime work.
```

<!-- ============================================================ -->
<!-- END SOURCE FILE: docs/roadmap/POST_INTERNAL_ROC_BRIDGE_DECISION_GATE.md -->
<!-- ============================================================ -->


<!-- ============================================================ -->
<!-- BEGIN SOURCE FILE: docs/blueprints/ROC_ROX_SOLANA_ANCHOR_BRIDGE_BLUEPRINT.md -->
<!-- ============================================================ -->

# SOURCE FILE: docs/blueprints/ROC_ROX_SOLANA_ANCHOR_BRIDGE_BLUEPRINT.md

# ROC_ROX_SOLANA_ANCHOR_BRIDGE_BLUEPRINT.md — V2 Docs-Only Bridge Blueprint

RO:WHAT — Defines the future docs-only ROC ↔ ROX burn/mint bridge blueprint using a possible Solana Anchor program.

RO:WHY — Captures bridge architecture, threat boundaries, state machines, nonce binding, proof/finality, pause/halt, and recovery requirements before any runtime is considered.

RO:INTERACTS — PHASE6_CLOSEOUT.md, POST_INTERNAL_ROC_BRIDGE_DECISION_GATE.md, ROC_ROX_SOLANA_BRIDGE_BUILDPLAN.md, ROC_ROX_BRIDGE_THREAT_MODEL.md, ron-proto, ron-ledger, svc-wallet, svc-interop, ron-policy, ron-audit, CrabLink Tauri, future isolated Solana Anchor program.

RO:INVARIANTS — This blueprint is docs-only; internal ROC truth remains svc-wallet/ron-ledger; burn/mint only, no custody/swap/liquidity; no bridge runtime; no direct ledger mutation outside svc-wallet; no client/gateway/omnigate authority.

RO:METRICS — Future metrics may track bridge request lifecycle states, challenge windows, proof quorum failures, nonce rejections, pause/halt events, stuck-state recoveries, recovery-account issues, and UI stale-status displays.

RO:CONFIG — Future bridge config must include disabled-by-default enablement, cluster/program/mint binding, challenge windows, quorum thresholds, pause/halt flags, per-user caps, daily caps, burn/fee/remainder sinks, key rotation windows, upgrade authority policy, and recovery-account policy.

RO:SECURITY — Protect against double mint, double issue, replay, RPC equivocation, cluster confusion, CPI abuse, upgrade authority compromise, coordinator compromise, stuck challenge states, stale UI, and liquidity/regulatory language creep.

RO:TEST — Future tests must include DTO strictness, replay conservation, nonce binding, cluster mismatch rejection, Anchor account constraints, CPI safety, pause/halt drills, recovery drills, multi-RPC equivocation, stale UI, and forbidden wording scanners.

---

## 0. Status

This is a docs-only blueprint.

It does not authorize:

```text
ROX live token
Solana deployment
bridge runtime
mint/burn runtime
external settlement
staking
liquidity
exchange integrations
public validator economics
```

Safe status:

```text
ROC ↔ ROX bridge: DOCS / THREAT-MODEL ONLY.
```

---

## 1. Bridge model

The only acceptable first bridge model is:

```text
burn first
prove later
challenge/finality later
mint only after proof acceptance
```

### ROC → ROX

```text
user explicit intent
→ backend quote
→ explicit confirmation
→ svc-wallet burns internal ROC
→ ron-ledger records durable burn receipt
→ proof package is created
→ challenge window opens
→ proof quorum validates
→ future Solana Anchor program mints ROX
→ backend records bridge lifecycle status
→ CrabLink displays backend-derived status
```

### ROX → ROC

```text
user explicit intent
→ future Solana burn request
→ ROX is burned on Solana
→ proof package is observed
→ challenge window opens
→ proof quorum validates
→ svc-wallet issues internal ROC
→ ron-ledger records durable issue receipt
→ CrabLink displays backend-derived status
```

The bridge is not:

```text
instant conversion
liquidity pool
DEX
CEX
custodial IOU
staking system
yield system
price oracle system
guaranteed redemption system
```

---

## 2. Authority model

### svc-wallet

Allowed future role:

```text
prepare bridge quotes
burn ROC after explicit confirmation
issue ROC after verified external burn
refund recoverable internal failures
emit backend-derived receipt/status
```

Forbidden:

```text
trust client finality
trust gateway finality
trust omnigate finality
trust single RPC finality
mint ROX directly
skip ron-ledger
skip challenge windows
```

### ron-ledger

Allowed future role:

```text
record internal burn
record internal issue
record refund/recovery movements
provide replay and conservation truth
```

Forbidden:

```text
perform Solana IO
trust Solana directly
replace proof model
accept client-supplied balance truth
```

### svc-interop

Allowed future role:

```text
observe external chain data
coordinate proof packages
track lifecycle states
submit non-authoritative observations
```

Forbidden:

```text
mutate ROC balances
issue ROC
burn ROC
mark finality alone
trust one RPC source alone
```

### ron-policy

Allowed future role:

```text
validate bridge config
enforce caps
enforce pause/halt
validate allowed cluster/program/mint
gate proof acceptance conditions
```

Forbidden:

```text
mutate ledger
create receipt
override wallet/ledger truth
```

### ron-audit

Allowed future role:

```text
append lifecycle audit events
record governance decisions
record key rotation ceremony evidence
record emergency halt evidence
```

Forbidden:

```text
decide finality
mutate balances
replace ledger
```

### svc-gateway and omnigate

Allowed future role:

```text
route bridge status requests
hydrate backend-derived bridge status
enforce request quotas
display structured errors
```

Forbidden:

```text
mutate balances
create receipts
decide bridge finality
unlock paid access from bridge cache
```

### CrabLink Tauri

Allowed future role:

```text
display quote
request explicit confirmation
display pending / challenge / failed / completed states
display risk disclaimers
refresh backend-derived status
```

Forbidden:

```text
own bridge authority
invent finality
invent receipts
invent balances
cache-only finality
use forbidden exchange language
```

---

## 3. Cross-domain nonce binding

Every bridge lifecycle must bind a nonce to the complete domain context.

Minimum nonce binding fields:

```text
bridge_version
direction
internal_operation_id
internal_ledger_receipt_id
internal_account_id
external_cluster
external_program_id
external_mint
external_token_program_id
external_tx_signature
external_slot
external_commitment
amount_minor_units
fee_minor_units
burn_minor_units
remainder_minor_units
recipient_external_address_hash
recipient_internal_account_hash
created_at_epoch_ms
expires_at_epoch_ms
```

The consumed nonce record must reject reuse across:

```text
direction
cluster
program id
mint
token program
internal operation id
ledger receipt id
Solana tx signature
recipient
amount
```

Required future test names:

```text
test_nonce_rejects_same_internal_burn_on_second_cluster
test_nonce_rejects_same_solana_burn_for_second_internal_issue
test_nonce_rejects_program_id_mismatch
test_nonce_rejects_mint_mismatch
test_nonce_rejects_direction_mismatch
test_nonce_rejects_amount_mismatch
test_nonce_rejects_recipient_mismatch
```

---

## 4. Proof and finality model

Single-RPC proof is forbidden.

Future proof acceptance requires:

```text
configured minimum Solana commitment
independent multi-RPC quorum
cluster binding
program id binding
mint binding
token program binding
slot / blockhash context
transaction signature binding
instruction data validation
account owner validation
token account validation
supply delta validation where applicable
challenge window close
policy gate acceptance
audit event
```

Minimum proof states:

```text
OBSERVED_UNTRUSTED
QUORUM_OBSERVED
CHALLENGE_OPEN
CHALLENGE_DISPUTED
CHALLENGE_CLOSED
FINALITY_ACCEPTED
FINALITY_REJECTED
RECOVERY_REQUIRED
```

A Solana proof must never directly mutate internal ROC.

Only the accepted proof may create an approved intent for svc-wallet.

---

## 5. Challenge window and griefing mitigation

Challenge windows must be configurable.

They must include:

```text
minimum duration
maximum duration
dispute reason taxonomy
challenge rate limits
per-account challenge caps
stake-free anti-spam throttle
manual governance escalation path
audit trail
```

Forbidden:

```text
infinite unbounded challenge spam
automatic challenge acceptance from unauthenticated clients
challenge resolution without audit evidence
finalization while emergency halt is active
```

---

## 6. Pause / halt semantics

Pause and halt are distinct.

```text
pause
→ stops new prepares
→ may allow safe observation
→ may allow already-safe refunds
```

```text
halt
→ stops new prepares
→ stops pending finalizations
→ stops mint submission
→ stops issue execution
→ requires governance/audit recovery path
```

Emergency halt must freeze both:

```text
new bridge requests
pending finalizations in both directions
```

Required future drills:

```text
halt_after_internal_burn_before_external_mint
halt_after_external_burn_before_internal_issue
halt_after_mint_submitted_before_observed
coordinator_compromised_during_challenge_window
rpc_equivocation_during_quorum_observation
```

---

## 7. Recovery and refund model

### Stuck challenge timeout

A bridge request must not remain in `CHALLENGE_OPEN` forever.

Future config must define:

```text
challenge_max_duration_ms
stuck_review_after_ms
manual_review_required
auto_refund_allowed_for_internal_burn_only
recovery_account_required_for_external_burn_issue_failure
```

### ROC → ROX recovery

If internal ROC is burned but external mint cannot safely happen:

```text
recoverable_before_external_mint
→ governance/audit review
→ svc-wallet refund or partial refund
→ ron-ledger durable refund receipt
```

Fees, burns, and remainders must be explicitly classified:

```text
refundable_amount
non_refundable_protocol_burn
non_refundable_network_fee
manual_recovery_amount
remainder_sink_amount
```

### ROX → ROC recovery

If ROX is burned externally and internal issue fails after proof acceptance:

```text
external burn remains final
→ issue ROC to governance-controlled recovery account
→ audit event
→ manual review
→ later svc-wallet transfer to intended recipient if approved
```

The system must not silently lose the user’s claim.

The system must not mint directly from policy, interop, gateway, omnigate, or CrabLink.

---

## 8. Solana Anchor program model

A possible future Anchor program may include:

```text
BridgeConfig PDA
BridgeMintAuthority PDA
BridgeNonce PDA
BridgeBatch PDA
BridgeReceipt PDA
EmergencyState PDA
UpgradeAuthority policy
```

Candidate instructions:

```text
initialize_config
set_pause
set_halt
rotate_authority
register_bridge_batch
mint_rox_from_roc_burn
burn_rox_for_roc_entry
mark_nonce_consumed
```

Every instruction must define:

```text
account owner constraints
PDA seed constraints
signer constraints
mint constraints
token account constraints
token program constraints
amount constraints
nonce constraints
pause/halt constraints
clock/slot constraints
event emission
```

CPI safety requirements:

```text
token program id must match configured token program
mint account must match configured ROX mint
mint authority must be the expected PDA
recipient token account mint must match ROX mint
burn source token account mint must match ROX mint
no arbitrary CPI target
no unchecked token account owner
no unchecked mint authority
no unchecked remaining accounts for authority
```

Required future Anchor tests:

```text
test_anchor_rejects_wrong_token_program
test_anchor_rejects_wrong_mint
test_anchor_rejects_wrong_mint_authority
test_anchor_rejects_wrong_pda_seed
test_anchor_rejects_wrong_recipient_token_account
test_anchor_rejects_reused_nonce
test_anchor_rejects_paused_new_request
test_anchor_rejects_halted_finalization
test_anchor_rejects_malicious_cpi_attempt
```

---

## 9. Upgrade authority and key rotation

Before any Solana deployment is considered, docs must define:

```text
upgrade authority owner
multi-sig threshold
emergency signer threshold
key rotation ceremony
lost-key recovery path
compromised-key halt path
verifiable build requirement
deployment artifact hash
program id registry
audit event format
public disclosure posture
```

No value-bearing deployment is allowed until:

```text
upgrade authority policy is reviewed
verifiable build process is proven
emergency halt drill passes
key rotation drill passes
rollback or freeze posture is defined
```

---

## 10. Conservation

All bridge math uses integer minor-unit strings.

No floats.

ROC → ROX:

```text
roc_burn_amount
= rox_mint_amount
+ bridge_fee_amount
+ protocol_burn_amount
+ remainder_sink_amount
```

ROX → ROC:

```text
rox_burn_amount
= roc_issue_amount
+ bridge_fee_amount
+ protocol_burn_amount
+ remainder_sink_amount
```

The replay/conservation proof must reject:

```text
negative amounts
float amounts
scientific notation
overflow
fee > amount
burn > amount
remainder mismatch
cross-direction reuse
```

---

## 11. CrabLink UX truthfulness

CrabLink must show bridge status as backend-derived.

Required stale/offline behavior:

```text
If backend bridge status cannot be refreshed:
  show "pending verification — status not refreshed"
  do not show final
  do not show complete
  do not show minted
  do not show issued
  do not unlock anything from cache
```

Required pending wording:

```text
Burn request submitted.
Verification is pending.
Challenge window may delay completion.
This is not final yet.
```

Forbidden words in bridge UI:

```text
convert
exchange
trade
swap
instant
guaranteed
cash out
risk-free
guaranteed value
```

Required disclaimer template:

```text
Bridge requests are experimental and delayed. A burn may be irreversible. External mint or internal issue depends on backend verification, challenge windows, policy gates, and recovery controls. Do not treat pending status as final.
```

---

## 12. First implementation posture

Even after docs are accepted, first implementation posture must be:

```text
local-only
non-value-bearing
feature-gated
disabled by default
not reachable from normal CrabLink UI
no public endpoint
no production endpoint
no mainnet
no devnet value
no liquidity
no staking
no exchange language
```

Runtime requires a later decision gate.

---

## 13. Acceptance gate for this blueprint

This blueprint is acceptable only if checkers confirm:

```text
docs-only status present
no runtime authorization
cross-domain nonce binding present
multi-RPC proof model present
halt pending finalizations present
stuck challenge recovery present
Anchor account constraints present
CPI safety present
upgrade authority policy present
CrabLink stale/offline status present
forbidden UI wording list present
no liquidity/staking/exchange runtime authorization
```

<!-- ============================================================ -->
<!-- END SOURCE FILE: docs/blueprints/ROC_ROX_SOLANA_ANCHOR_BRIDGE_BLUEPRINT.md -->
<!-- ============================================================ -->


<!-- ============================================================ -->
<!-- BEGIN SOURCE FILE: docs/buildplans/ROC_ROX_SOLANA_BRIDGE_BUILDPLAN.md -->
<!-- ============================================================ -->

# SOURCE FILE: docs/buildplans/ROC_ROX_SOLANA_BRIDGE_BUILDPLAN.md

# ROC_ROX_SOLANA_BRIDGE_BUILDPLAN.md — V2 Docs-Only Bridge Buildplan

RO:WHAT — Defines a safe, phased, docs-first buildplan for possible future ROC ↔ ROX / Solana bridge work.

RO:WHY — Prevents premature bridge runtime by requiring closeout, threat model, proof hardening, nonce binding, audit drills, and explicit decision gates before any value-bearing external action.

RO:INTERACTS — PHASE6_CLOSEOUT.md, POST_INTERNAL_ROC_BRIDGE_DECISION_GATE.md, ROC_ROX_SOLANA_ANCHOR_BRIDGE_BLUEPRINT.md, ROC_ROX_BRIDGE_THREAT_MODEL.md, ron-proto, ron-ledger, svc-wallet, svc-interop, ron-policy, ron-audit, CrabLink Tauri, future isolated Solana Anchor program.

RO:INVARIANTS — No bridge runtime until separate authorization; internal ROC first; svc-wallet mutates; ron-ledger records; burn before mint; proof before release; challenge before completion; caps before public access; audit before mainnet; liquidity/staking/exchange work remains separate.

RO:METRICS — Future phase metrics may track doc acceptance, threat-model coverage, nonce/replay tests, proof quorum tests, halt/recovery drills, Anchor constraint tests, verifiable build proof, and forbidden-runtime checks.

RO:CONFIG — Bridge enablement must stay false by default. Any future enable=true must be rejected unless caps, pause/halt, challenge windows, cluster/mint/program binding, recovery accounts, and governance thresholds are configured.

RO:SECURITY — No value-bearing deployment before Phase 9 audit/recovery drills; no mainnet before separate decision gate; no liquidity/staking/exchange-facing behavior in this buildplan.

RO:TEST — Each phase requires focused checkers, docs tests, DTO strictness tests, replay/conservation tests, proof-model tests, Anchor local-validator tests only when authorized, and no-authority boundary tests.

---

## 0. Current status

```text
Internal ROC Beta Phase 6: COMPLETE / GREEN / PARKED.
Bridge work: DOCS / THREAT-MODEL ONLY.
No bridge runtime authorized.
```

This buildplan does not authorize:

```text
ROX live token
Solana deployment
bridge runtime
external settlement
staking
liquidity
exchange integrations
public validator economics
```

---

## 1. Phase map

```text
Phase 0 — Docs gate and threat model
Phase 1 — DTO and ledger label sketches only
Phase 2 — Internal-only bridge accounting simulation
Phase 3 — Proof model and multi-RPC verification hardening
Phase 4 — Solana Anchor local-validator skeleton, isolated and non-value-bearing
Phase 5 — Local end-to-end simulation only
Phase 6 — Devnet shadow observation only
Phase 7 — Devnet toy-value ROC→ROX candidate only after audit gate
Phase 8 — Devnet toy-value ROX→ROC candidate only after audit gate
Phase 9 — Recovery, halt, upgrade, and key-rotation audit drills
Phase 10 — Mainnet canary candidate only by separate decision gate
Phase 11 — Public beta candidate only by separate decision gate
```

No phase in this document authorizes liquidity, staking, yield, DEX, CEX, or exchange-facing language.

---

## 2. Phase 0 — Docs gate and threat model

Purpose:

```text
Move from bridge idea to reviewed docs without creating runtime.
```

Work:

```text
create / accept POST_INTERNAL_ROC_BRIDGE_DECISION_GATE.md
create / accept ROC_ROX_SOLANA_ANCHOR_BRIDGE_BLUEPRINT.md
create / accept ROC_ROX_SOLANA_BRIDGE_BUILDPLAN.md
create / accept ROC_ROX_BRIDGE_THREAT_MODEL.md
run docs-only checker
collect adversarial review
```

Exit gate:

```text
all docs say docs/threat-model only
all docs forbid runtime
all docs preserve svc-wallet/ron-ledger truth
all docs include nonce/proof/halt/recovery/Anchor/UX risks
```

Forbidden:

```text
runtime code
Solana code
ROX mint code
bridge endpoints
client bridge UI
```

Completion label:

```text
Bridge Phase 0 docs gate COMPLETE / GREEN / PARKED.
```

---

## 3. Phase 1 — DTO and ledger label sketches only

Crate pair:

```text
ron-proto + ron-ledger
```

Allowed:

```text
DTO sketches
schema docs
test vectors
deny_unknown_fields tests
bridge lifecycle labels
replay label sketches
conservation test plans
no runtime routing
```

Required DTO properties:

```text
versioned schema
deny unknown fields
integer minor-unit strings
direction binding
cluster binding
program id binding
mint binding
token program binding
operation id binding
nonce binding
status enum forward-only
```

Required tests:

```text
bridge_dto_rejects_unknown_fields
bridge_dto_rejects_float_amounts
bridge_dto_rejects_cross_cluster_replay
bridge_lifecycle_status_forward_only
bridge_conservation_rejects_remainder_mismatch
```

Exit gate:

```text
DTOs are strict and non-authoritative.
Ledger labels do not execute bridge runtime.
```

Forbidden:

```text
Solana calls
wallet mutation changes
bridge endpoints
mint/burn runtime
```

---

## 4. Phase 2 — Internal-only bridge accounting simulation

Crate pair:

```text
svc-wallet + ron-accounting
```

Allowed:

```text
internal simulation of burn/issue/refund labels
quote shape validation
accounting snapshots
replay/conservation proofs
no external IO
```

Required boundary:

```text
svc-wallet remains only mutation front-door
ron-accounting remains reporting only
```

Required tests:

```text
bridge_sim_internal_burn_has_receipt
bridge_sim_internal_issue_requires_approved_intent
bridge_sim_accounting_cannot_mutate
bridge_sim_replay_conservation
bridge_sim_refund_timeout_path
```

Exit gate:

```text
internal simulation proves conservation without Solana or ROX runtime.
```

Forbidden:

```text
real bridge
Solana proof acceptance
external mint
external burn
```

---

## 5. Phase 3 — Proof model and multi-RPC verification hardening

Crate pair:

```text
svc-interop + ron-policy
```

Secondary:

```text
ron-audit
```

Purpose:

```text
Harden proof/finality before any Solana program skeleton.
```

Allowed:

```text
proof model design
multi-RPC quorum simulation
cluster/program/mint binding validation
challenge-window state machine
griefing mitigation design
audit event schemas
policy validation for disabled bridge config
```

Required tests:

```text
proof_rejects_single_rpc
proof_rejects_cluster_mismatch
proof_rejects_program_id_mismatch
proof_rejects_mint_mismatch
proof_rejects_commitment_below_minimum
policy_rejects_bridge_enable_without_pause_halt_caps
policy_rejects_bridge_enable_without_challenge_window
audit_records_challenge_dispute
```

Exit gate:

```text
Proof model is hardened before any Anchor skeleton exists.
```

Forbidden:

```text
Solana program code
mint/burn execution
client bridge UI
mainnet/devnet value
```

Completion label:

```text
Bridge Phase 3 proof model hardening COMPLETE / GREEN / PARKED.
```

---

## 6. Phase 4 — Solana Anchor local-validator skeleton

Location:

```text
external/solana/rox-bridge-anchor/
```

or:

```text
programs/rox-bridge-anchor/
```

Constraint:

```text
isolated from RustyOnions service crates
local-validator only
non-value-bearing
feature-gated
disabled by default
not reachable from CrabLink
```

Allowed:

```text
Anchor account skeleton
PDA constraints
token program constraints
local-validator tests
verifiable build notes
upgrade authority notes
```

Required tests:

```text
test_anchor_rejects_wrong_token_program
test_anchor_rejects_wrong_mint
test_anchor_rejects_wrong_mint_authority
test_anchor_rejects_wrong_pda_seed
test_anchor_rejects_wrong_recipient_token_account
test_anchor_rejects_reused_nonce
test_anchor_rejects_paused_new_request
test_anchor_rejects_halted_finalization
test_anchor_rejects_malicious_cpi_attempt
```

Exit gate:

```text
local-validator skeleton proves account constraints only.
No value-bearing network.
```

Forbidden:

```text
mainnet
devnet value
public mint
CrabLink bridge UI
production relayer
```

---

## 7. Phase 5 — Local end-to-end simulation only

Allowed:

```text
local RustyOnions dev stack
local Solana validator
toy mint
toy accounts
no public endpoint
no real value
```

Required:

```text
end-to-end nonce binding
ROC burn simulation
ROX mint simulation
ROX burn simulation
ROC issue simulation
replay rejection
pause/halt simulation
stuck challenge recovery simulation
```

Exit gate:

```text
simulation proves safety properties, not launch readiness.
```

---

## 8. Phase 6 — Devnet shadow observation only

Allowed:

```text
read-only devnet observation
multi-RPC quorum measurements
no value-bearing bridge
no user-facing bridge
no public UI
```

Forbidden:

```text
devnet bridge value promises
devnet public beta
exchange wording
mainnet
```

Exit gate:

```text
observer proves quorum/finality behavior under non-value conditions.
```

---

## 9. Phase 7 — Devnet toy-value ROC→ROX candidate

Prerequisite:

```text
Phase 9 audit drills must pass first, or this phase remains locked.
```

Allowed only after separate authorization:

```text
toy-value ROC→ROX devnet burn/mint candidate
hard caps
allowlisted accounts
manual review
no public UI
```

Forbidden:

```text
mainnet
liquidity
exchange wording
guaranteed value
```

---

## 10. Phase 8 — Devnet toy-value ROX→ROC candidate

Prerequisite:

```text
Phase 7 and Phase 9 must pass first, or this phase remains locked.
```

Allowed only after separate authorization:

```text
toy-value ROX→ROC devnet burn/issue candidate
hard caps
allowlisted accounts
manual review
recovery account path
```

Forbidden:

```text
mainnet
liquidity
exchange wording
guaranteed value
```

---

## 11. Phase 9 — Recovery, halt, upgrade, and key-rotation audit drills

Purpose:

```text
Prove emergency controls before any value-bearing network decision.
```

Required drills:

```text
halt_after_internal_burn_before_external_mint
halt_after_external_burn_before_internal_issue
halt_after_mint_submitted_before_observed
coordinator_compromised_during_challenge_window
rpc_equivocation_during_quorum_observation
upgrade_authority_rotation
emergency_key_rotation
stuck_challenge_governance_recovery
recovery_account_manual_issue
verifiable_build_reproduction
```

Exit gate:

```text
all drills pass
audit logs complete
manual recovery paths documented
key rotation ceremony documented
upgrade authority policy accepted
```

Completion label:

```text
Bridge Phase 9 audit/recovery drills COMPLETE / GREEN / PARKED.
```

---

## 12. Phase 10 — Mainnet canary candidate

Locked unless a separate decision gate exists:

```text
docs/roadmap/ROX_MAINNET_CANARY_DECISION_GATE.md
```

No work may start from this buildplan alone.

---

## 13. Phase 11 — Public beta candidate

Locked unless separate decision gates exist:

```text
docs/roadmap/ROX_PUBLIC_BETA_DECISION_GATE.md
docs/roadmap/ROX_LIQUIDITY_DECISION_GATE.md
docs/roadmap/ROX_STAKING_DECISION_GATE.md
docs/roadmap/ROX_EXCHANGE_FACING_RISK_MODEL.md
```

Public beta must still not imply:

```text
liquidity
staking
yield
exchange listing
guaranteed value
instant conversion
```

---

## 14. Global forbidden scope

Forbidden in all phases unless a later reviewed decision gate explicitly authorizes it:

```text
liquidity
staking
yield
DEX
CEX
exchange-facing UI
guaranteed value language
instant conversion language
public validator economics
client-side bridge authority
gateway bridge authority
omnigate bridge authority
direct ledger mutation outside svc-wallet
```

---

## 15. Current next action

Safe next action:

```text
complete Phase 0 docs gate
accept threat model
run docs-only checker
park bridge runtime
return to Internal ROC Beta Stabilization / Product Beta Readiness
```

---

## NO-VALUE-BEARING-DEVNET-PHASE9-GATE

No value-bearing devnet, toy-value devnet, public bridge demo, external mint/burn, or user-facing ROX bridge path may begin before Phase 9 audit/recovery drills are COMPLETE / GREEN / PARKED.

Phase 7 and Phase 8 remain locked unless:

```text
Phase 9 audit/recovery drills pass
a separate runtime decision gate exists
hard caps exist
allowlists exist
pause/halt controls exist
recovery paths exist
UI language remains non-exchange-facing
```

This buildplan still does not authorize runtime bridge work.

<!-- ============================================================ -->
<!-- END SOURCE FILE: docs/buildplans/ROC_ROX_SOLANA_BRIDGE_BUILDPLAN.md -->
<!-- ============================================================ -->


<!-- ============================================================ -->
<!-- BEGIN SOURCE FILE: docs/threat-models/ROC_ROX_BRIDGE_THREAT_MODEL.md -->
<!-- ============================================================ -->

# SOURCE FILE: docs/threat-models/ROC_ROX_BRIDGE_THREAT_MODEL.md

# ROC_ROX_BRIDGE_THREAT_MODEL.md — ROC ↔ ROX / Solana Bridge Threat Model

RO:WHAT — Threat model for possible future ROC ↔ ROX burn/mint bridge design.

RO:WHY — Identifies double-mint, double-issue, replay, proof/finality, RPC equivocation, Solana Anchor, custody, recovery, key compromise, UX truthfulness, liquidity creep, and governance risks before any runtime work.

RO:INTERACTS — POST_INTERNAL_ROC_BRIDGE_DECISION_GATE.md, ROC_ROX_SOLANA_ANCHOR_BRIDGE_BLUEPRINT.md, ROC_ROX_SOLANA_BRIDGE_BUILDPLAN.md, ron-proto, ron-ledger, svc-wallet, svc-interop, ron-policy, ron-audit, CrabLink Tauri, future Solana Anchor program.

RO:INVARIANTS — Threat model only; no runtime authorization; internal ROC truth remains svc-wallet/ron-ledger; no direct ledger mutation outside svc-wallet; no client/gateway/omnigate/accounting/rewarder/policy authority.

RO:METRICS — Future controls may expose nonce replay rejection counts, proof quorum failures, challenge disputes, halt activations, recovery-account issues, key rotation events, stuck challenge timeouts, and stale UI status displays.

RO:CONFIG — Future bridge config must be disabled by default and rejected if pause/halt/caps/challenge windows/quorum thresholds/cluster binding/program binding/mint binding/recovery policy are missing.

RO:SECURITY — Treat Solana RPCs, clients, coordinators, UI caches, relayers, upgrade keys, and bridge configs as hostile or compromiseable.

RO:TEST — Future red-team tests must target nonce replay, cluster replay, RPC equivocation, stale proof, malicious CPI, wrong token program, stuck challenge, halt bypass, recovery failure, forbidden UI language, and accidental liquidity scope.

---

## 0. Status

This is a threat model.

It does not authorize bridge runtime.

Safe label:

```text
ROC ↔ ROX bridge: DOCS / THREAT-MODEL ONLY.
```

---

## 1. Assets

Protected assets:

```text
internal ROC balances
ron-ledger replay/conservation truth
svc-wallet mutation boundary
bridge lifecycle status truth
ROX mint authority if future program exists
bridge nonces
proof packages
challenge decisions
pause/halt controls
upgrade authority
recovery account
audit trail
CrabLink user trust
```

---

## 2. Trust boundaries

Trusted only within their defined roles:

```text
svc-wallet
→ internal ROC mutation front-door only

ron-ledger
→ internal durable truth only

ron-policy
→ declarative gate only

ron-audit
→ append-only evidence only
```

Untrusted or partially trusted:

```text
CrabLink client
browser/client storage
gateway
omnigate
svc-interop coordinator
Solana RPC provider
single Solana transaction observation
relayer
external wallet
public network
logs
cache
user-provided proof
```

No single untrusted component may cause mint, issue, burn, refund, or finality.

---

## 3. Primary threats

### T1 — Double mint

Attack:

```text
same ROC burn receipt is used to mint ROX twice
```

Controls:

```text
cross-domain nonce binding
consumed nonce PDA
internal operation_id binding
ledger receipt binding
direction binding
cluster binding
program id binding
mint binding
amount binding
recipient binding
audit event
```

Required tests:

```text
same_internal_burn_cannot_mint_twice
same_internal_burn_cannot_mint_on_second_cluster
same_internal_burn_cannot_mint_to_second_recipient
```

### T2 — Double issue

Attack:

```text
same ROX burn is used to issue internal ROC twice
```

Controls:

```text
Solana tx signature binding
instruction binding
token account binding
mint binding
burn amount binding
consumed nonce record
svc-wallet idempotency
ron-ledger replay rejection
```

Required tests:

```text
same_external_burn_cannot_issue_twice
same_external_burn_cannot_issue_to_second_account
same_external_burn_cannot_issue_after_reorg_observation_mismatch
```

### T3 — Cluster replay

Attack:

```text
devnet proof or fork proof is replayed as mainnet proof
```

Controls:

```text
cluster binding
genesis/hash context where available
configured allowed cluster
policy validation
proof package includes cluster
nonce includes cluster
```

Required test:

```text
proof_rejects_cluster_mismatch
```

### T4 — RPC equivocation

Attack:

```text
coordinator is fed false or stale data by one RPC
```

Controls:

```text
single-RPC proof forbidden
independent multi-RPC quorum
minimum commitment
slot context
transaction status confirmation
challenge window
audit trail
```

Required tests:

```text
proof_rejects_single_rpc
proof_rejects_rpc_quorum_disagreement
proof_rejects_below_minimum_commitment
```

### T5 — Reorg / stale finality assumption

Attack:

```text
system treats observation as final before sufficient proof window
```

Controls:

```text
minimum commitment
challenge window
delayed finality
status remains pending until accepted
manual review for conflicts
```

Required test:

```text
stale_observation_does_not_finalize
```

### T6 — Coordinator compromise

Attack:

```text
svc-interop coordinator marks proof accepted without valid quorum
```

Controls:

```text
coordinator cannot mutate ledger
policy gate independently validates proof conditions
svc-wallet requires approved intent
audit trail required
halt switch freezes pending finalizations
```

Required drill:

```text
coordinator_compromised_during_challenge_window
```

### T7 — Emergency halt bypass

Attack:

```text
new or pending bridge action completes after halt
```

Controls:

```text
halt blocks new prepares
halt blocks pending finalizations
halt blocks mint submission
halt blocks internal issue
halt requires audited recovery path
```

Required tests:

```text
halt_blocks_new_prepare
halt_blocks_pending_finalization
halt_blocks_internal_issue
halt_blocks_external_mint_submission
```

### T8 — Stuck challenge forever

Attack/failure:

```text
request remains CHALLENGE_OPEN indefinitely
```

Controls:

```text
challenge max duration
stuck review timer
governance review path
refund path for internal burn before external mint
recovery account path for external burn before internal issue
```

Required tests:

```text
challenge_open_expires_to_review_required
internal_burn_stuck_path_can_refund_after_review
external_burn_stuck_path_routes_to_recovery_account
```

### T9 — Malicious CPI / wrong token program

Attack:

```text
Anchor instruction is called with wrong token program, mint, authority, or account
```

Controls:

```text
token program constraint
mint constraint
PDA seed constraint
mint authority constraint
recipient token account mint constraint
source token account mint constraint
no unchecked authority through remaining accounts
```

Required tests:

```text
anchor_rejects_wrong_token_program
anchor_rejects_wrong_mint
anchor_rejects_wrong_mint_authority
anchor_rejects_malicious_cpi_attempt
```

### T10 — Upgrade authority compromise

Attack:

```text
program is upgraded to malicious bridge logic
```

Controls:

```text
multi-sig upgrade authority
threshold policy
key rotation ceremony
emergency halt
verifiable builds
deployment artifact hashes
public program id registry
audit event
```

Required drills:

```text
upgrade_authority_rotation
compromised_upgrade_key_halt
verifiable_build_reproduction
```

### T11 — Client stale status

Attack/failure:

```text
CrabLink shows completed/final based on stale cache or offline status
```

Controls:

```text
backend-derived status only
offline status shows pending verification
no cache-only finality
no cache-only paid unlock
stale status warning
```

Required tests:

```text
crablink_offline_bridge_status_is_not_final
crablink_stale_status_shows_pending_verification
crablink_cache_cannot_complete_bridge
```

### T12 — Product language / liquidity creep

Attack/failure:

```text
docs or UI imply exchange, conversion, guaranteed value, instant settlement, or tradability
```

Controls:

```text
forbidden wording scanner
risk disclaimer template
separate liquidity decision gate
separate staking decision gate
separate exchange-facing risk model
```

Forbidden UI terms:

```text
convert
exchange
trade
swap
instant
guaranteed
cash out
risk-free
guaranteed value
```

Required test:

```text
bridge_ui_forbidden_wording_scanner
```

---

## 4. Abuse cases

```text
User replays old burn receipt.
User submits devnet proof against mainnet config.
RPC provider lies about transaction status.
Coordinator tries to approve proof alone.
Gateway tries to mark finality.
Omnigate hydrates stale complete status.
CrabLink cache shows bridge complete offline.
Policy config enables bridge without halt/caps.
Anchor instruction accepts wrong mint.
Upgrade authority deploys malicious program.
Challenge spam blocks all finalization.
Internal burn succeeds but external mint fails.
External burn succeeds but internal issue fails.
```

Every abuse case must have a future test or drill before runtime.

---

## 5. Required config gates

ron-policy must reject bridge enablement unless all exist:

```text
bridge.enabled=false by default
allowed_cluster
allowed_program_id
allowed_mint
allowed_token_program_id
minimum_commitment
rpc_quorum_threshold
challenge_min_duration_ms
challenge_max_duration_ms
per_user_daily_cap
global_daily_cap
pause_flag
halt_flag
recovery_account
upgrade_authority_policy
emergency_signer_threshold
forbidden_ui_wording_check_enabled
```

If any are absent, bridge runtime must fail closed.

---

## 6. Severity table

| Threat | Severity | Required before runtime |
| --- | --- | --- |
| Double mint | Critical | nonce + replay + Anchor tests |
| Double issue | Critical | nonce + wallet idempotency + ledger replay tests |
| RPC equivocation | Critical | multi-RPC quorum |
| Halt bypass | Critical | halt pending finalization tests |
| Upgrade compromise | Critical | multi-sig + verifiable build + drills |
| Malicious CPI | High | Anchor account constraint tests |
| Stuck challenge | High | recovery timeout and governance path |
| Stale UI | High | stale/offline UX tests |
| Product language creep | Medium/High | wording scanner and disclaimers |

---

## 7. Red-team prompt

Use this prompt for future LLM/human review:

```text
You are reviewing the RustyOnions / CrabLink ROC ↔ ROX bridge design. Be adversarial. Look for double-mint, double-issue, replay, cluster replay, Solana RPC equivocation, false finality, challenge-window griefing, stuck states, emergency halt bypass, coordinator compromise, Anchor account constraint gaps, malicious CPI, wrong token program, upgrade authority compromise, key rotation gaps, recovery account abuse, stale UI finality, forbidden exchange language, liquidity creep, and any bypass of svc-wallet/ron-ledger truth. Do not suggest weakening the invariant that svc-wallet is the only internal ROC mutation front-door and ron-ledger is internal durable economic truth.
```

---

## 8. Threat model decision

Current decision:

```text
Threat model created.
Runtime remains unauthorized.
Next safe action is docs review and checker execution.
```

<!-- ============================================================ -->
<!-- END SOURCE FILE: docs/threat-models/ROC_ROX_BRIDGE_THREAT_MODEL.md -->
<!-- ============================================================ -->

