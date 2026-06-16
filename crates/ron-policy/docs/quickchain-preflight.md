# ron-policy QuickChain Phase-0 Preflight

RO:WHAT — QuickChain preflight boundary notes for `ron-policy`.
RO:WHY — Prevent declarative policy from becoming wallet authority, ledger truth, receipt authority, balance authority, paid-access finality, or QuickChain runtime.
RO:INTERACTS — ron-policy, svc-gateway, omnigate, svc-wallet, ron-ledger, ron-accounting, svc-rewarder, svc-storage, svc-index, ron-proto::quickchain.
RO:INVARIANTS — policy is declarative only; policy decisions are not economic truth; economics config is not mutation authority; no roots/checkpoints/validators/settlement.
RO:METRICS — none directly; services that consume policy own service metrics.
RO:CONFIG — policy bundles and ROC economics config are caller-provided declarative input.
RO:SECURITY — no fake receipts, no fake balances, no silent spend, no local paid unlock from policy alone.
RO:TEST — `quickchain_preflight_docs`, `quickchain_preflight_boundary`, `quickchain_preflight_decision_non_authority`, `quickchain_preflight_economics_config_non_authority`.

## 0. Status

ron-policy is declarative policy infrastructure.

`ron-policy` evaluates explicit rules and returns:

```text
allow / deny decisions
reasons
trace steps
obligations
validated economics configuration
```

`ron-policy` does not mutate economic state.

`ron-policy` is not a wallet.

`ron-policy` is not a ledger.

`ron-policy` is not QuickChain runtime.

`ron-policy` is not paid-access finality.

`ron-policy` is not settlement infrastructure.

## 1. Correct authority model

Policy may say:

```text
this rule matched
this request is allowed by policy
this request is denied by policy
this obligation must be satisfied by the caller/service boundary
this policy input failed validation
this economics config branch is enabled
this price/cap/split config is valid
```

Policy must not say as final economic truth:

```text
this user has paid
this wallet balance is correct
this receipt is final
this content is unlocked
this reward has been paid
this checkpoint is canonical
this state root is canonical
this settlement is complete
```

Plain scanner phrase: policy decision is not economic truth.

Plain scanner phrase: policy allow is not paid proof.

Plain scanner phrase: policy obligation is not receipt proof.

Plain scanner phrase: policy explanation is not finality proof.

Plain scanner phrase: economics policy config is not ledger mutation.

Plain scanner phrase: feature flag is not settlement authority.

## 2. Paid-access relationship

Allowed relationship:

```text
Policy may require a backend wallet/ledger/storage receipt path.
Policy may deny access if required proof is absent.
Policy may allow a service request to proceed after external services have proven truth.
Policy may express an obligation that a caller must satisfy through approved backend paths.
```

Forbidden shortcut paths:

```text
policy allow -> unlock
policy tag says paid -> unlock
policy config says paid -> unlock
policy obligation satisfied locally -> receipt
policy explanation -> finality
feature flag -> settlement
economics config -> balance mutation
```

Policy must not manufacture paid proof.

Policy must not manufacture receipt proof.

Policy must not manufacture finality proof.

Policy must not manufacture balance proof.

## 3. Economics config boundary

The ROC economics policy shape is configuration and validation only.

It can describe:

```text
enabled beta paid actions
integer minor-unit caps
integer minor-unit prices
hold multiplier basis points
payout split basis points
recipient roles and account aliases
```

It must not perform:

```text
issue
transfer
burn
hold
capture
release
mint
credit
debit
balance mutation
receipt creation
receipt finalization
paid unlock
settlement
```

The wallet remains the mutation front-door.

The ledger remains durable economic truth.

Accounting remains snapshot/projection infrastructure.

Rewarder remains payout planning only.

Policy remains declarative governance only.

## 4. QuickChain forbidden scope for this crate

`ron-policy` must not define or implement:

```text
state roots
receipt roots
accounting roots
reward roots
checkpoint roots
checkpoint headers
checkpoint hashes
validator sets
validator signatures
committee quorum
fork choice
consensus
finality engine
settlement status
external anchors
bridge settlement
staking
liquidity
ROX
Solana
public chain state
root producer
checkpoint producer
```

QuickChain may later verify a policy hash as part of a future deterministic checkpoint artifact, but `ron-policy` itself does not produce checkpoints, roots, validators, bridges, anchors, or settlement.

## 5. What this crate-local preflight proves

This preflight proves:

```text
docs preserve declarative-policy-only doctrine
Cargo.toml does not pull direct wallet/ledger/accounting/rewarder mutation crates
production source does not import wallet/ledger/accounting/rewarder authority crates
production source does not call mutation verbs
production source does not define QuickChain root/checkpoint/validator/settlement machinery
policy decisions do not carry receipt/balance/finality/unlock/root/settlement authority fields
authority-shaped obligation kinds and parameter keys reject during validation
economics config rejects authority-shaped unknown fields
economics config serialization contains config fields, not receipt/balance/finality/root/settlement fields
existing economics and policy tests remain green
strict clippy remains green
```

## 6. Future allowed work

Allowed later in this crate before Phase 1:

```text
more policy DTO strictness tests
more economics validation tests
policy-hash discussion only after canonical bytes are locked elsewhere
source boundary tests for future feature gates
more docs explaining caller/service responsibilities
```

Still forbidden here:

```text
root-producing code
checkpoint-producing code
validator code
settlement code
wallet mutation
ledger mutation
receipt creation
balance truth
paid unlock finality
external anchors
bridge logic
staking
liquidity
ROX
Solana
```
