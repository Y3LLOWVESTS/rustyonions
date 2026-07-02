# Internal ROC Stabilization — svc-wallet Mutation Front-Door Boundary

RO:WHAT — Product beta readiness boundary for svc-wallet as the only normal Internal ROC mutation front-door.

RO:WHY — Paid access, paid storage, approved payouts, and balance refresh must all depend on backend wallet and ledger truth. svc-wallet may accept user/backend-approved economic intent, enforce idempotency/nonce/policy/capability boundaries, commit through the ledger adapter, and return backend-derived receipts.

RO:INTERACTS — routes/v1, dto/requests.rs, dto/responses.rs, idem/store.rs, ledger/client.rs, accounting/client.rs, existing Internal ROC beta wallet tests.

RO:INVARIANTS — svc-wallet is the mutation front-door; ron-ledger is durable economic truth; idempotency_key is retry metadata; receipt_hash is backend evidence; settlement_status remains accepted-only unless a later authorized runtime gate exists.

RO:SECURITY — no fake receipt, fake balance, fake finality, silent spend, cache-only unlock, gateway/omnigate/accounting/rewarder/policy/client direct ledger mutation, bridge, ROX/Solana, staking, liquidity, or external settlement.

RO:TEST — cargo test -p svc-wallet --test internal_roc_stabilization_mutation_frontdoor_boundary.

## Allowed

user intent or backend-approved payout intent
→ svc-wallet DTO validation
→ capability/policy checks
→ idempotency and nonce discipline
→ ledger adapter commit
→ backend-derived accepted wallet receipt
→ receipt lookup and derivative accounting observation

## Forbidden

gateway mutation bypass
omnigate mutation bypass
accounting mutation
rewarder mutation
policy mutation
client-created receipt
client-created balance
cache-only unlock
fake receipt
fake balance
fake finality
silent spend
bridge runtime
staking runtime
liquidity runtime
ROX/Solana runtime
external settlement runtime

## Product beta lock

The product beta wallet boundary is not a new phase and does not reopen Internal ROC Beta Phase 6. It is a stabilization wrapper over the proven value loop.

The only accepted receipt status here is wallet/ledger accepted. Stronger labels such as finalized, anchored, or external settlement are future-gated and must not be invented by svc-wallet.
