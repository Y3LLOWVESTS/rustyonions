# svc-wallet QuickChain Preflight Boundary

RO:WHAT — Documents the svc-wallet QuickChain Phase-0/preflight boundary.
RO:WHY — svc-wallet is the ROC mutation front-door and must not accidentally become a chain node, root producer, validator, bridge, finality oracle, or settlement runtime.
RO:INTERACTS — svc-wallet, ron-ledger, ron-proto::quickchain, ron-accounting, svc-rewarder, svc-gateway, omnigate, CrabLink.
RO:INVARIANTS — wallet remains mutation front-door; ron-ledger remains durable economic truth; QuickChain preflight is inert; no fake balances, receipts, roots, checkpoints, validators, anchors, bridges, staking, liquidity, or external settlement.
RO:METRICS — none; this is a boundary/runbook document.
RO:CONFIG — QuickChain remains disabled by default and only compiles under the quickchain-preflight feature.
RO:SECURITY — request bodies cannot smuggle chain authority; live wallet routes do not leak projection authority; idempotency is retry identity, not operation authority.
RO:TEST — crates/svc-wallet/scripts/dev-quickchain-preflight.sh plus focused quickchain_preflight_* integration tests.

---

## 0. Status

This document describes the current allowed `svc-wallet` QuickChain Phase-0/preflight posture.

It is not a chain implementation.

It does not enable:

    roots
    checkpoints
    validators
    settlement
    external anchors
    public bridge
    staking
    liquidity
    exchange-facing behavior
    pruning
    CrabLink chain authority
    gateway ledger mutation
    omnigate ledger mutation
    accounting ledger mutation
    rewarder ledger mutation

The current `svc-wallet` preflight work is intentionally narrow:

    prove wallet routes remain wallet routes
    prove wallet receipts remain wallet receipts
    prove QuickChain projection is manual and inert
    prove request bodies cannot smuggle chain authority
    prove idempotency_key is not operation_id
    prove accepted is not finalized
    prove feature-gated preflight does not change default wallet behavior

---

## 1. Economic boundary

`svc-wallet` is the mutation front-door for internal ROC operations.

Allowed wallet mutations:

    issue
    transfer
    burn
    hold
    capture
    release

Durable economic truth belongs to:

    ron-ledger

`svc-wallet` may:

    validate requests
    enforce capabilities and policy
    resolve idempotency
    reserve and roll back nonces
    call the ledger adapter
    return backend-derived wallet receipts
    serve receipt lookup
    serve balance reads backed by ledger state

`svc-wallet` must not:

    invent balances
    invent receipts
    silently spend
    accept client-supplied finality
    accept client-supplied operation authority
    produce QuickChain roots
    produce checkpoints
    act as a validator
    act as a bridge
    anchor externally
    replace ron-ledger as economic truth

---

## 2. QuickChain preflight boundary

The `quickchain-preflight` feature is a review and compatibility surface only.

It may expose inert projection helpers for future review work.

It must not create live chain behavior.

Allowed under `quickchain-preflight`:

    manual wallet receipt projection
    strict validation helpers
    schema constants
    unknown-field rejection tests
    invalid b3 rejection tests
    accepted-only status validation
    request-poisoning tests
    route-shape tests
    idempotency/operation identity tests

Forbidden under `quickchain-preflight`:

    root production
    checkpoint production
    validator signatures
    consensus
    fork choice
    settlement
    external anchors
    bridge logic
    staking
    liquidity
    public chain state
    service calls from QuickChain DTOs
    ledger mutation caused by QuickChain DTOs
    CrabLink chain authority

---

## 3. Live HTTP routes remain wallet-shaped

The following live routes are wallet routes even when compiled with `quickchain-preflight`:

    POST /v1/issue
    POST /v1/transfer
    POST /v1/burn
    POST /v1/hold
    POST /v1/capture
    POST /v1/release
    GET  /v1/tx/{txid}

Live wallet responses must retain wallet receipt vocabulary such as:

    txid
    op
    from
    to
    asset
    amount_minor
    nonce
    idem
    ts
    ledger_seq_start
    ledger_seq_end
    ledger_root
    settlement_status
    receipt_hash

Live routes must not leak projection or chain-authority fields such as:

    schema
    chain_id
    operation_id
    idempotency_key
    produced_at_ms
    legacy_ledger_root
    state_root
    receipt_root
    checkpoint
    anchor
    validator
    finalized
    finality
    settlement_root

Receipt lookup remains wallet receipt lookup.

It is not a QuickChain projection endpoint.

---

## 4. Request poisoning boundary

Client request bodies must not be able to smuggle chain authority into live wallet mutations.

Mutation request DTOs must reject unknown authority-looking fields such as:

    schema
    chain_id
    operation_id
    state_root
    receipt_root
    checkpoint
    anchor
    validator
    finalized
    finality
    settlement_root
    settlement_status

Expected behavior for poisoned mutation requests:

    client error
    not 200 OK
    no wallet receipt returned
    no idempotency key consumed
    no nonce consumed
    clean retry with the same idempotency key still succeeds

This boundary protects the wallet from accidentally accepting future-chain vocabulary as spend, unlock, settlement, finality, root, bridge, validator, or chain authority.

---

## 5. Idempotency and operation identity

`idempotency_key` is retry identity.

It is not economic authority.

It is not durable chain operation identity.

`operation_id` is future backend-assigned durable ledger-operation identity.

It must not be derived from:

    idempotency_key
    client request field
    txid
    route label
    UI state
    wall clock
    database row order
    cache key

Current proven behavior:

    same idempotency key + same body -> byte-identical wallet receipt replay
    same idempotency key + changed body -> conflict
    projection requires explicit operation_id
    changing projection operation_id does not rewrite wallet receipt hash
    wallet receipt hash remains wallet-derived evidence

---

## 6. Finality honesty

Wallet receipts currently use:

    accepted

They must not accept or emit future finality labels such as:

    finalized
    anchored
    settled
    checkpointed
    confirmed

Accepted means:

    backend wallet/ledger path accepted and recorded the mutation

Accepted does not mean:

    QuickChain finalized
    validator finalized
    externally anchored
    publicly settled
    bridge settled
    pruning-safe

---

## 7. Projection boundary

The preflight projection helper is manual and inert.

It requires explicit context:

    chain_id
    operation_id
    settlement_status = accepted

Projection validation rejects malformed:

    chain_id
    operation_id
    txid
    account ids
    idempotency key
    timestamp
    ledger sequence range
    legacy ledger root
    receipt hash
    asset

Projection DTOs reject unknown future authority fields such as:

    state_root
    receipt_root
    checkpoint
    anchor
    validator
    finality
    finalized
    settlement_root

The projection schema is intentionally svc-wallet-specific.

It must not masquerade as a canonical future QuickChain receipt DTO.

---

## 8. Test inventory

Focused QuickChain preflight tests:

    crates/svc-wallet/tests/quickchain_preflight_boundary.rs
    crates/svc-wallet/tests/quickchain_preflight_no_runtime_authority.rs
    crates/svc-wallet/tests/quickchain_preflight_live_route_matrix.rs
    crates/svc-wallet/tests/quickchain_preflight_idempotency_identity_boundary.rs
    crates/svc-wallet/tests/quickchain_preflight_request_poisoning_matrix.rs
    crates/svc-wallet/tests/quickchain_preflight_projection_validation_matrix.rs

What they prove:

    default wallet posture remains unchanged
    ron-ledger preflight surface is inert from wallet boundary
    manual projection requires explicit context
    fake receipt hashes reject
    live routes remain wallet-shaped
    live routes do not leak chain authority fields
    request bodies cannot smuggle chain authority fields
    idempotency_key is not operation_id
    accepted is not finalized
    projection DTO rejects malformed identity/hash/sequence/schema/status data

---

## 9. Local runbook

Run focused preflight tests:

    cargo test -p svc-wallet --features quickchain-preflight --test quickchain_preflight_boundary
    cargo test -p svc-wallet --features quickchain-preflight --test quickchain_preflight_no_runtime_authority
    cargo test -p svc-wallet --features quickchain-preflight --test quickchain_preflight_live_route_matrix
    cargo test -p svc-wallet --features quickchain-preflight --test quickchain_preflight_idempotency_identity_boundary
    cargo test -p svc-wallet --features quickchain-preflight --test quickchain_preflight_request_poisoning_matrix
    cargo test -p svc-wallet --features quickchain-preflight --test quickchain_preflight_projection_validation_matrix

Run the full local wallet preflight gate:

    crates/svc-wallet/scripts/dev-quickchain-preflight.sh

Recommended wider confidence gate:

    cargo clippy -p svc-wallet --all-targets --features quickchain-preflight -- -D warnings
    cargo check --workspace

Workspace warnings outside `svc-wallet` should not be treated as `svc-wallet` QuickChain failures unless the workspace is promoted to a zero-warning gate.

---

## 10. No-go checklist before leaving svc-wallet Phase 0

Do not mark the `svc-wallet` QuickChain preflight slice complete unless all are true:

    [ ] all normal svc-wallet tests pass
    [ ] all quickchain-preflight svc-wallet tests pass
    [ ] clippy passes for svc-wallet without quickchain-preflight
    [ ] clippy passes for svc-wallet with quickchain-preflight
    [ ] request poisoning matrix passes
    [ ] projection validation matrix passes
    [ ] live route matrix passes
    [ ] idempotency/operation identity boundary passes
    [ ] no live route emits QuickChain authority fields
    [ ] no live route accepts QuickChain authority fields
    [ ] accepted remains the only wallet receipt status
    [ ] projection remains manual and inert
    [ ] QuickChain remains disabled by default
    [ ] no roots/checkpoints/validators/anchors/bridges/settlement were added

---

## 11. Next crate handoff

After this `svc-wallet` preflight slice is stable, the next likely QuickChain Phase-0 crate is:

    ron-accounting

Reason:

    ron-accounting must prove it is derivative metering/snapshot infrastructure, not balance truth.

`ron-accounting` should prove:

    it consumes wallet/ledger-derived events or receipts
    it can produce deterministic snapshots
    snapshots are not spend authority
    snapshots cannot mint/burn/transfer
    snapshot ids/hashes are candidates only until vectors are locked
    no fake finality
    no root-producing code before vectors are locked

After `ron-accounting`, the likely next crate is:

    svc-rewarder

`svc-rewarder` must prove:

    it plans payouts
    it never mutates ledger directly
    it never pays raw engagement directly
    it separates economic_receipt, metering, proof_eligible, ad_budgeted, and analytics_only event classes
    approved payout intent later goes through svc-wallet and ron-ledger

---

## 12. Plain-English summary

`svc-wallet` is not QuickChain.

`svc-wallet` is the internal ROC mutation front-door.

The current QuickChain preflight work makes sure that when future QuickChain proof/root work arrives, wallet receipts can be projected and reviewed without accidentally turning wallet routes into chain routes.

The safest current doctrine is:

    wallet accepts and records internal ROC mutations
    ledger remains economic truth
    wallet receipt is backend-derived hot-path evidence
    QuickChain projection is manual and inert
    idempotency is retry identity
    operation_id is backend durable operation identity
    accepted is not finalized
    clients cannot smuggle finality or roots
    live routes remain wallet-shaped
    roots wait for locked canonical vectors
