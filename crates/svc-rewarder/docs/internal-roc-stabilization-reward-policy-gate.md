# Internal ROC Stabilization — svc-rewarder Reward Planning / Policy Gate Boundary

RO:WHAT — Product beta readiness boundary for svc-rewarder as deterministic capped reward planner and wallet-handoff producer.

RO:WHY — The rewarder must consume only accounting/policy-approved, verified, capped inputs and produce deterministic plans or wallet issue request candidates. It must not become wallet, ledger, receipt, balance, payout execution, finality, bridge, staking, liquidity, or external settlement authority.

RO:INTERACTS — inputs/anti_farming.rs, inputs/accounting.rs, outputs/intents.rs, outputs/wallet.rs, core/compute.rs, existing Internal ROC beta rewarder tests.

RO:INVARIANTS — rewarder plans only; ron-policy gates eligibility declaratively; svc-wallet remains mutation front-door; ron-ledger remains durable truth; idempotent wallet issue requests are handoff DTOs, not receipts.

RO:SECURITY — no raw engagement direct payout, no analytics_only payout, no metering direct payout, no proof_eligible payout without verification/caps/policy gate, no ad_budgeted protocol-pool emission, no direct ledger mutation, no fake receipt, no fake balance, no fake finality, no bridge, no ROX/Solana, no staking, no liquidity, no external settlement.

RO:TEST — cargo test -p svc-rewarder --test internal_roc_stabilization_reward_policy_gate_boundary.

## Allowed

accounting snapshot
→ event class classification
→ verification
→ anti-farming caps
→ ron-policy declarative gate
→ deterministic reward manifest
→ deterministic settlement intent / wallet issue request DTO
→ svc-wallet approved execution only
→ ron-ledger durable receipt/balance truth

## Forbidden

raw engagement → payout
analytics_only → payout
metering → direct payout
proof_eligible without policy gate → payout
ad_budgeted without explicit non-protocol budget → protocol ROC emission
rewarder plan → direct ledger mutation
rewarder plan → receipt truth
rewarder plan → balance truth
rewarder plan → finality truth
rewarder plan → paid unlock
rewarder plan → bridge/staking/liquidity/external settlement

## Product beta lock

This is a stabilization wrapper over the proven Internal ROC value loop. It does not reopen Internal ROC Beta Phase 6 and does not authorize public bridge, ROX/Solana, staking, liquidity, external settlement, validator, checkpoint, or root runtime.
