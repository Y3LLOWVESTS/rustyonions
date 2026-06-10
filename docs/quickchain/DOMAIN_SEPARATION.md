# QuickChain Domain Separation

RO:WHAT — Names and rules for future QuickChain hash/signature preimage domain separation.
RO:WHY — Prevent cross-context hash/signature reuse before roots, signatures, pruning, or anchors exist.
RO:INTERACTS — ron-proto quickchain DTOs, future ron-ledger roots, future svc-wallet receipt roots, future validator signatures.
RO:INVARIANTS — no hashing yet; no roots yet; no signatures yet; constants only; exact strings are test-gated.
RO:METRICS — none.
RO:CONFIG — none; QuickChain remains disabled.
RO:SECURITY — every future hash/signature context must use a distinct, versioned domain string.
RO:TEST — tests/quickchain_domain_separation.rs.

## 1. Purpose

Domain separation prevents one byte string from being valid in the wrong context.

Without domain separation, a hash or signature created for one purpose could accidentally or maliciously be reused as if it belonged to another purpose.

QuickChain must never hash raw concatenated data without context.

## 2. Phase 0 scope

This document and the matching ron-proto constants define names only.

They do not:

- compute hashes
- compute roots
- verify signatures
- create checkpoints
- mutate ledgers
- authorize balances
- enable validators
- enable pruning
- enable anchors

## 3. Future rule

Every future QuickChain hash preimage must be built as:

    domain separator bytes
    versioned canonical payload bytes

The exact preimage framing is intentionally not implemented in this batch.

Before any root/hash code is added, the future implementation must decide whether the framing is:

    domain || 0x00 || canonical_payload

or another explicit, test-vector-backed framing.

## 4. Domain separator rules

Every separator must:

- start with quickchain.
- end with .v1
- contain lowercase ASCII only
- contain only lowercase letters, digits, dot, and underscore
- be unique
- be short enough to audit by eye

## 5. Phase 0 separators

Account state root contexts:

- quickchain.account_state.leaf.v1
- quickchain.account_state.node.v1
- quickchain.account_state.empty.v1

Receipt root contexts:

- quickchain.receipt.leaf.v1
- quickchain.receipt.node.v1
- quickchain.receipt.empty.v1
- quickchain.receipt_batch.header.v1

Checkpoint contexts:

- quickchain.checkpoint.header.v1
- quickchain.checkpoint.signature.v1

Validator/governance contexts:

- quickchain.validator_set.v1
- quickchain.chain_params.v1
- quickchain.challenge.evidence.v1

Data availability contexts:

- quickchain.data_availability.leaf.v1
- quickchain.data_availability.node.v1
- quickchain.data_availability.empty.v1

Accounting/reward contexts:

- quickchain.accounting_snapshot.v1
- quickchain.reward_manifest.v1

External anchor context:

- quickchain.anchor_payload.v1

## 6. Acceptance

This batch is acceptable only if:

- [ ] every domain string validates
- [ ] every domain string is unique
- [ ] all domain strings are exact test-gated constants
- [ ] no hash/root/signature function is added
- [ ] no runtime service consumes these constants as authority
