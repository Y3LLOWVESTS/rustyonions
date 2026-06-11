# QuickChain Domain Separation

RO:WHAT — Versioned domain names for future QuickChain hash/signature preimages.
RO:WHY — Prevent cross-context hash/signature reuse before roots, validators, pruning, or anchors exist.
RO:INTERACTS — QUICKCHAIN.MD, ron-proto quickchain DTOs, future ron-ledger roots, future receipt/state/accounting/reward/checkpoint hashes.
RO:INVARIANTS — constants only; no hashing; no roots; no signatures; no ledger mutation; no settlement authority.
RO:METRICS — none.
RO:CONFIG — none; QuickChain remains disabled.
RO:SECURITY — every future root/hash context must use a distinct, versioned, test-gated domain.
RO:TEST — crates/ron-proto/tests/quickchain_domain_separation.rs.

## Status

This file is part of QC-0A preflight.

It does not implement QuickChain.

It does not compute hashes.

It does not produce roots.

It only freezes names that future root-producing code must use after canonical bytes and golden vectors are ready.

## Required root/hash domains

The current QuickChain blueprint defines these mandatory domains:

```text
receipt_hash_domain = "quickchain.receipt.v1"
account_leaf_hash_domain = "quickchain.account-state.v1"
hold_leaf_hash_domain = "quickchain.hold-state.v1"
receipt_root_domain = "quickchain.receipt-root.v1"
state_root_domain = "quickchain.state-root.v1"
accounting_root_domain = "quickchain.accounting-root.v1"
reward_root_domain = "quickchain.reward-root.v1"
checkpoint_hash_domain = "quickchain.checkpoint.v1"
```

These names are intentionally short, versioned, human-auditable, and context-specific.

## Extra preflight domains

The preflight package also reserves domains for future compact artifacts:

```text
quickchain.chain-params.v1
quickchain.validator-set.v1
quickchain.policy.v1
quickchain.data-availability-root.v1
quickchain.receipt-batch.v1
quickchain.accounting-window.v1
quickchain.reward-manifest.v1
quickchain.challenge-evidence.v1
quickchain.anchor-payload.v1
```

These remain constants only.

They do not enable anchors, validators, DA, rewards, pruning, or settlement.

## Future preimage rule

Future root-producing code must use explicit framing.

Candidate framing:

```text
domain_separator_bytes || 0x00 || canonical_payload_bytes
```

Do not implement the framing until TEST_VECTORS.md contains exact byte and hash vectors.

## Rules

Every domain separator must:

- start with `quickchain.`
- end with `.v1`
- use lowercase ASCII only
- use only lowercase letters, digits, dots, underscores, and hyphens
- be unique
- be test-gated
- never be silently renamed after vectors exist

## Go / no-go

If these domain names change, all future vectors must be regenerated.

If there are no golden vectors, no root-producing code may ship.
