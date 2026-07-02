# Internal ROC Stabilization — ron-policy Declarative Gate Non-Authority Boundary

RO:WHAT — Product beta readiness boundary for ron-policy as a declarative policy/economics gate.

RO:WHY — Policy may allow, deny, explain, validate economics config, and require obligations. Policy must not manufacture receipts, balances, paid unlocks, payout execution, finality, wallet mutation, ledger mutation, bridge, staking, liquidity, or external settlement.

RO:INTERACTS — model.rs, engine/eval.rs, parse/validate.rs, economics/validate.rs, existing Internal ROC beta policy tests.

RO:INVARIANTS — policy decision is not economic truth; policy allow is not paid proof; policy obligation is not receipt proof; policy explanation is not finality proof; economics config is not wallet authority.

RO:SECURITY — no wallet mutation, no ledger mutation, no receipt creation, no balance truth, no payout execution, no refund authority, no paid unlock authority, no finality truth, no bridge, no ROX/Solana, no staking, no liquidity, no external settlement.

RO:TEST — cargo test -p ron-policy --test internal_roc_stabilization_policy_gate_non_authority_boundary.

## Allowed

strict policy bundle parsing
strict economics config validation
deterministic allow/deny decision
reason and trace output
declarative obligations
anti-farming eligibility gate
approved-payout candidate gate
paid-content policy gate
config/economics validation

## Forbidden

policy allow → wallet mutation
policy allow → ledger mutation
policy allow → receipt truth
policy allow → balance truth
policy allow → payout execution
policy allow → paid unlock
policy allow → finality
policy obligation → receipt
policy obligation → settlement
economics config → balance mutation
economics config → payout execution
feature flag → bridge/staking/liquidity/external settlement

## Product beta lock

ron-policy is a gate. Services enforce decisions. svc-wallet mutates. ron-ledger records truth. Accounting snapshots and rewarder plans remain non-authoritative until svc-wallet and ron-ledger accept the resulting operation.
