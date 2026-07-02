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
