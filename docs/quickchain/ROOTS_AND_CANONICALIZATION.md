# QuickChain Roots and Canonicalization

RO:WHAT — Canonicalization and root-design preflight for future QuickChain settlement.
RO:WHY — Consensus, proofs, pruning, and checkpoints are impossible unless all honest nodes hash identical bytes.
RO:INTERACTS — ron-proto quickchain DTOs, future ron-ledger roots, svc-wallet receipts, ron-accounting snapshots, svc-rewarder manifests.
RO:INVARIANTS — no roots before canonical bytes; no fake checkpoint hashes; no floats; b3 hashes remain lowercase; no map-order ambiguity.
RO:METRICS — none yet.
RO:CONFIG — no runtime config yet; QuickChain remains disabled.
RO:SECURITY — canonicalization split-brain is a consensus failure; root code must be domain-separated and test-vector driven.
RO:TEST — ron-proto quickchain canonical JSON tests.

## 1. Why this document exists

QuickChain cannot safely begin with validators.

QuickChain must begin with bytes.

If two honest nodes build the same logical checkpoint but serialize different byte strings, they will compute different hashes. That would break state roots, receipt roots, validator signatures, checkpoint proofs, pruning, and external anchors.

The first rule is:

Same logical input must produce the same canonical bytes.

## 2. Phase 0 canonicalization scope

Phase 0 only defines a small canonical JSON v1 experiment for strict DTOs.

Allowed:

- exact JSON bytes for known QuickChain DTO structs
- field-order normalization by deserializing into strict DTOs and serializing from typed structs
- unknown-field rejection before canonicalization
- float rejection where integer strings are required
- lowercase b3 validation through ContentId
- test vectors for exact bytes

Forbidden:

- pretending these bytes are final consensus bytes
- producing live checkpoint hashes
- producing live state roots
- producing live receipt roots
- verifying validator signatures
- pruning history
- accepting canonical DTOs as wallet/ledger authority

## 3. Canonical JSON v1 experiment

Canonical JSON v1 for Phase 0 means:

- serde struct field order is the encoded field order
- output is minified UTF-8 JSON
- no insignificant whitespace
- no maps inside Phase 0 hashed DTOs
- no floats in money fields
- no unknown fields
- all b3 hashes are ContentId values
- integer money is decimal string form
- timestamps are u64 milliseconds

Important limitation:

This is only a narrow experiment for QuickChain DTO structs. It is not a general JSON canonicalization standard for arbitrary JSON objects.

## 4. Root strategy later

Future roots must be implemented only after canonical bytes are green.

Future account state root:

- root scheme: sorted_merkle_map_v1
- leaf bytes: canonical AccountStateV1 bytes
- sort key: account_id byte order
- hash: BLAKE3
- required domain separation:
  - quickchain.account_state.leaf.v1
  - quickchain.account_state.node.v1

Future receipt root:

- root scheme: ledger_sequence_merkle_v1
- leaf bytes: canonical receipt bytes without receipt_hash
- order: ledger sequence order
- hash: BLAKE3
- required domain separation:
  - quickchain.receipt.leaf.v1
  - quickchain.receipt.node.v1

Future checkpoint hash:

- preimage: canonical checkpoint header bytes without any self-hash field
- hash: BLAKE3
- required domain separation:
  - quickchain.checkpoint.header.v1

## 5. No fake golden hashes

Golden byte vectors may exist now.

Golden hash vectors must wait until:

- canonical byte rules are frozen
- domain separation strings are frozen
- the hash function call is implemented in the correct crate/layer
- test fixtures prove exact bytes first

A hash over unstable bytes is not a proof. It is a trap.

## 6. Acceptance gate

Before local roots begin:

- [ ] QuickChain DTO strictness tests pass
- [ ] canonical JSON v1 byte tests pass
- [ ] field-order normalization test passes
- [ ] unknown fields reject before canonicalization
- [ ] b3 uppercase rejects
- [ ] integer money strings reject floats/noncanonical strings
- [ ] no live service consumes QuickChain canonical bytes as economic authority
