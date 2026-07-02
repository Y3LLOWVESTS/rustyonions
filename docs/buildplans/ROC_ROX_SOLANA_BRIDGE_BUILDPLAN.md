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
