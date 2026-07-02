# Internal ROC Stabilization — ron-ledger Replay / Conservation Truth Boundary

RO:WHAT — Product beta readiness boundary for ron-ledger accepted replay, conservation, receipt evidence, and retry semantics.

RO:WHY — ron-ledger is durable economic truth. Product beta stabilization must lock replay/conservation behavior without granting authority to gateway, omnigate, accounting, rewarder, policy, storage, index, or CrabLink.

RO:INTERACTS — quickchain/accepted_replay.rs, quickchain/execution_state.rs, quickchain/replay_index.rs, quickchain/types.rs, tests/internal_roc_beta_phase2_replay_conservation.rs, tests/internal_roc_beta_phase3_approved_payout_replay.rs.

RO:INVARIANTS — accepted replay is not a root, checkpoint, proof, consensus field, persistence format, bridge, or settlement artifact; retries must not mutate; rejected operations must not advance replay boundaries; conservation remains checked integer ROC truth.

RO:SECURITY — no fake receipt, fake balance, fake finality, silent spend, cache-only unlock, gateway/omnigate/index/storage/policy/accounting/rewarder/client ledger mutation, bridge, ROX/Solana, staking, liquidity, or external settlement.

RO:TEST — cargo test -p ron-ledger --features quickchain-preflight --test internal_roc_stabilization_replay_truth_boundary.

## Required model

svc-wallet-approved mutation intent
→ ron-ledger deterministic execution
→ accepted operation record
→ replayable balance/supply/hold state
→ backend receipt evidence

## Accepted replay must verify

- same operation intent
- same supply decision where required
- same hold epoch input where required
- same trusted receipt txid
- same account sequence evidence
- same primitive ledger sequence range
- same chain binding when present

## Accepted replay must not be treated as

- QuickChain root
- checkpoint
- signature
- consensus proof
- validator finality
- external anchor
- bridge settlement
- public chain runtime
