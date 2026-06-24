# svc-wallet QuickChain Phase-0 Preflight

RO:WHAT — Documents the svc-wallet QuickChain Phase-0/preflight boundary.
RO:WHY — svc-wallet is the ROC wallet mutation front-door; QuickChain remains future settlement infrastructure.
RO:INTERACTS — svc-wallet, ron-ledger, ron-proto quickchain DTOs, ron-accounting, svc-gateway, omnigate.
RO:INVARIANTS — wallet receipts are backend-derived; ron-ledger remains economic truth; no fake balances; no fake receipts; no silent spend; no roots/checkpoints/validators/settlement/anchors/bridges.
RO:METRICS — wallet metrics are derivative observability only and never balance truth.
RO:CONFIG — QuickChain preflight is feature-gated with `quickchain-preflight`.
RO:SECURITY — no private keys, spend authority, validators, anchors, bridges, staking, liquidity, or external settlement.
RO:TEST — `crates/svc-wallet/scripts/dev-quickchain-preflight.sh` and `crates/svc-wallet/scripts/dev-quickchain-park.sh`.

## 0. Status

`svc-wallet` is the wallet mutation front-door for internal ROC.

QuickChain is future settlement infrastructure.

This crate-level preflight does **not** authorize root-producing code, checkpoint code, validator code, settlement code, bridge code, anchor code, staking code, liquidity code, pruning code, public-chain code, or external settlement code.

The current purpose is narrower:

```text
svc-wallet accepts explicit wallet mutations.
svc-wallet validates capability/policy/idempotency/nonce boundaries.
svc-wallet commits only through ron-ledger.
svc-wallet returns backend-derived wallet receipts.
svc-wallet may expose inert QuickChain projection DTOs under a feature gate.
svc-wallet does not become QuickChain runtime authority.
```

## 1. svc-wallet role

`svc-wallet` owns the wallet service boundary.

Allowed wallet behavior:

```text
issue
transfer
burn
hold
capture
release
balance read
receipt lookup
idempotent replay of already accepted wallet receipts
```

Required truth boundary:

```text
ron-ledger remains economic truth.
svc-wallet is the mutation front-door.
ron-accounting remains derivative metering/snapshot infrastructure.
svc-rewarder plans payouts later but does not mutate balances.
gateway/omnigate may orchestrate paid access but must not mutate ledger directly.
```

## 2. QuickChain role in this crate

QuickChain in `svc-wallet` is Phase-0/preflight only.

Allowed now:

```text
compile-time feature-gated compatibility checks
manual wallet receipt projection helpers
strict DTO shape validation
request poisoning tests
idempotency identity tests
live route boundary tests
tooling boundary tests
docs/runbook hardening
```

Forbidden now:

```text
no roots
no receipt roots
no account state roots
no hold roots
no checkpoints
no validators
no settlement
no finality labels beyond wallet-side accepted
no anchors
no external anchors
no bridges
no staking
no liquidity
no pruning
no public-chain authority
no Solana/ROX/external settlement path
no gateway/omnigate/rewarder direct ledger mutation
```

## 3. Receipt honesty

`svc-wallet` may honestly report wallet-side acceptance after the wallet/ledger hot path accepts a mutation.

Allowed status:

```text
accepted
```

Forbidden invented status:

```text
finalized
checkpointed
anchored
settled
validator_accepted
epoch_included
bridge_confirmed
external_final
```

Those future states belong only to later QuickChain phases after canonical bytes, locked hashes, roots, proofs, and governance gates are explicitly authorized.

## 4. Idempotency and operation identity

`idempotency_key` is a retry key.

It is not operation authority.

It is not consensus identity.

It must not be treated as the durable ledger operation identity.

QuickChain doctrine for later phases:

```text
operation_id = backend-assigned durable ledger-operation identity
idempotency_key = retry key only
account_sequence = ledger-assigned sequence identity
hold_id = one hold lifecycle identity
expiry = epoch-based where applicable
```

Current `svc-wallet` tests must preserve that distinction.

## 5. Request poisoning boundary

Wallet mutation DTOs must reject unknown QuickChain authority fields.

Examples of fields that must not be accepted in wallet mutation request bodies:

```text
quickchain_root
state_root
receipt_root
checkpoint
checkpoint_id
validator_signature
validator_set
settlement_status
finality
anchor
external_anchor
bridge_txid
staking
liquidity
```

Client-supplied QuickChain authority must fail closed before nonce/idempotency identity is consumed.

## 6. Projection boundary

Any QuickChain projection helper in `svc-wallet` is manual, inert, and feature-gated.

Projection helpers may:

```text
copy accepted wallet receipt facts into a strict preflight DTO
validate chain/operation/context strings
validate b3-shaped receipt hashes
validate ledger sequence presence
reject unknown future authority fields
```

Projection helpers must not:

```text
mutate ledger
change wallet receipts
replace receipt lookup route behavior
invent roots
invent finality
invent anchors
construct checkpoint state
unlock paid content
grant spend authority
```

## 7. Live route boundary

Live wallet routes must continue returning wallet receipts, not QuickChain authority artifacts.

This applies even with:

```text
--features quickchain-preflight
```

The feature gate is compatibility scaffolding only.

It must not alter runtime wallet authority, route shapes, balance truth, or receipt lookup semantics.

## 8. Accounting relationship

`svc-wallet` may emit accounting events after wallet operations.

Accounting events are derivative.

Accounting events are not balances.

Accounting events are not receipts.

Accounting events are not proof of spend.

Accounting events are not root inputs until later authorized artifact/vector work.

The balance source remains:

```text
ron-ledger
```

## 9. Paid access relationship

Paid access must remain explicit:

```text
prepare/quote
explicit confirmation
backend wallet path
backend receipt
unlock/render
display-only receipt cache
balance refresh
```

Forbidden paid-access behavior:

```text
no silent spend
no fake receipt
no fake balance
no unlock from cache alone
no client-supplied finality
no gateway-side ledger mutation
no omnigate-side ledger mutation
```

## 10. Parking criteria

`svc-wallet` can be parked for the current QuickChain Phase-0/preflight pass when:

```text
cargo fmt -p svc-wallet -- --check passes
cargo clippy -p svc-wallet --all-targets -- -D warnings passes
cargo test -p svc-wallet --all-targets passes
feature-gated QuickChain tests pass
feature-gated clippy passes
feature-gated all-target tests pass
docs regression passes
parking script passes
```

Expected local gates:

```bash
crates/svc-wallet/scripts/dev-quickchain-preflight.sh
crates/svc-wallet/scripts/dev-quickchain-park.sh
```

## 11. Future work intentionally deferred

Deferred until canonical QuickChain gates are explicitly green:

```text
canonical account-state leaves
receipt root construction
state root construction
hold root construction
checkpoint root construction
proof inclusion formats
validator execution
settlement finality
external anchors
public bridges
staking
liquidity
pruning
archive/challenge/DA fallback
```

No fake hashes or placeholder roots are allowed.

QuickChain future vector doctrine remains:

```text
sketch -> locked_bytes -> locked_hash
```


## Phase 2 Round 2 committee boundary

svc-wallet may expose backend-derived accepted wallet receipt evidence to downstream verifier tooling, but it is not a committee member, not an attestation signer, not quorum authority, not fork-choice authority, not finality authority, and not settlement authority.

For this phase the safe wallet rule remains:

    wallet receipt evidence = backend-derived accepted receipt evidence
    committee attestation = not produced by svc-wallet
    quorum certificate = not produced by svc-wallet
    fork choice = not produced by svc-wallet
    finality = not produced by svc-wallet
    staking/slashing/bonding = not svc-wallet active runtime scope
    external settlement / bridge = forbidden

The Phase 2 Round 2 gate must preserve no silent spend, no fake receipts, no fake balances, no external settlement, no bridge, no staking, no slashing, and no validator-economy behavior.
