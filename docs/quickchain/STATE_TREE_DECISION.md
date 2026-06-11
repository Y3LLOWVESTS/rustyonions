# QuickChain State Tree Decision

RO:WHAT — QC-0A decision record for the future QuickChain state tree.
RO:WHY — State roots must be deterministic, validator-executable, and replayable before Phase 1 begins.
RO:INTERACTS — QUICKCHAIN.MD, ron-ledger, ron-proto quickchain DTOs, future account/hold/state proofs, future receipt roots.
RO:INVARIANTS — no root-producing code yet; no pruning; no validator runtime; no DB-order dependency; no wall-clock dependency.
RO:METRICS — none.
RO:CONFIG — none.
RO:SECURITY — state tree choice must avoid nondeterministic ordering, hidden service-local assumptions, and unbounded active hold bloat.
RO:TEST — future state tree golden vectors, unordered_input_same_root, account_state_vector_001, hold_compaction_vector_001.

---

## 0. Status

This is a QC-0A-R1 decision record.

It is still pre-implementation.

It does not implement state roots, Merkle roots, account proofs, hold proofs, validators, pruning, or settlement.

It narrows the first Phase 1 candidate so DTOs and vectors have a concrete target.

---

## 1. Phase 1 decision

Decision:

```text
Phase 1 state tree candidate = sorted binary Merkle map over canonical account-state leaves.
```

This means:

```text
- account-based state model
- canonical account-state leaf bytes
- bytewise deterministic sort keys
- duplicate keys rejected
- binary Merkle reduction over sorted leaf hashes
- no DB iteration order
- no map iteration order
- no sparse tree in Phase 1
```

Sparse Merkle tree is deferred.

Account trie is deferred.

Reason:

```text
A sorted Merkle map is the smallest auditable state-root design for Phase 1.
It is easier to vectorize, easier to replay independently, and less likely to hide consensus-breaking path rules.
```

---

## 2. Non-decision: final forever tree

This is not a forever commitment.

Future designs may migrate to:

```text
sparse Merkle tree
account trie
verkle-like structure
external DA-native proof tree
```

But any migration must be:

```text
versioned
governance-gated
test-vector-backed
replayable from old checkpoint to new checkpoint
explicit about proof compatibility
```

No silent tree migration is allowed.

---

## 3. Account-based model

QuickChain uses an account-based state model, not UTXO.

Reason:

```text
RustyOnions already has passports, wallet accounts, balances, holds, receipts, creator payouts, and paid access flows.
```

Future account leaf target:

```text
AccountStateV1 {
  schema: "quickchain.account-state.v1"
  account_id: string
  asset: "roc"

  balance_minor: string
  held_minor: string
  available_minor: string

  account_sequence: u64
  receipt_root: b3:<64hex>
  holds_root: b3:<64hex>
  permissions_root: b3:<64hex> | null

  updated_at_epoch: string
}
```

Rules:

```text
- all money values are integer minor-unit strings
- available_minor = balance_minor - held_minor
- no floats
- no negative values
- account_sequence assigned by ledger commit path
- receipt_root commits account-related receipts according to future receipt rules
- holds_root commits currently open holds only
```

---

## 4. Hold subtree decision

Decision:

```text
Each account has a holds_root over currently open holds only.
```

Open hold leaf target:

```text
HoldStateV1 {
  schema: "quickchain.hold-state.v1"
  hold_id: string
  account_id: string
  counterparty_account_id: string | null
  amount_minor: string
  purpose: string
  created_at_epoch: string
  expires_at_epoch: string
  status: "open"
  operation_id: string
  idempotency_key: string
  policy_hash: b3:<64hex>
}
```

Rules:

```text
- open holds sorted by hold_id
- duplicate hold_id rejected
- terminal holds are removed from active holds_root after terminal receipt inclusion
- terminal lifecycle is proven by receipt_root
- no closed-hold bloat in active state
```

---

## 5. Sort keys

### 5.1 Account leaf sort key

Decision:

```text
account_leaf_sort_key = utf8(account_id) || 0x00 || utf8(asset)
```

Rules:

```text
- account_id must be normalized before reaching root code
- asset for current ROC is exactly "roc"
- bytewise ascending order
- duplicate sort key is invalid
```

### 5.2 Hold leaf sort key

Decision:

```text
hold_leaf_sort_key = utf8(hold_id)
```

Rules:

```text
- hold_id must be normalized before reaching root code
- bytewise ascending order
- duplicate hold_id is invalid
```

### 5.3 Receipt sort key

Decision:

```text
receipt_sort_key =
    u64_be(ledger_seq_start)
    || utf8(txid)
```

Rules:

```text
- ledger_seq_start is encoded as exactly 8 unsigned big-endian bytes
- ledger_seq_start must be greater than zero
- fixed-width big-endian bytes preserve unsigned numeric order
- txid bytes provide the deterministic tie-breaker
- ordering is bytewise ascending over the complete key
- duplicate complete sort keys are invalid
- ledger_seq_end is validated separately
- txid uniqueness and non-overlapping ledger ranges remain replay invariants
- no database, map, arrival, scheduler, or wall-clock ordering is permitted
```

This rule defines ordering bytes only. It does not hash receipts, build a
receipt tree, or produce a receipt root.

---

## 6. Node model

Phase 1 candidate:

```text
leaf_hash = b3(domain("quickchain.account-state.v1") || 0x00 || canonical_account_state_bytes)

node_hash = b3(domain("quickchain.state-root.v1") || 0x00 || left_hash || right_hash)
```

Status:

```text
This is a target shape, not implemented code.
```

Before implementation:

```text
- exact preimage bytes must be in TEST_VECTORS.md
- exact odd-leaf promotion rule must be chosen
- exact empty tree rule must be chosen
- exact account leaf bytes must be frozen
```

---

## 7. Empty tree rule

Decision:

```text
No all-zero root is used as a normal empty tree root.
```

Reason:

```text
All-zero values are ambiguous and should be reserved only for explicit genesis sentinels where the schema says so.
```

Future empty tree root must be computed from a canonical empty-tree payload.

Target payload shape:

```text
EmptyTreeV1 {
  schema: "quickchain.empty-tree.v1"
  tree: "state" | "holds" | "receipts" | "accounting" | "rewards"
}
```

Status:

```text
Not implemented.
No production empty-root hash exists yet.
```

No-go:

```text
No root code until empty tree vectors exist.
```

---

## 8. Odd leaf rule

Decision for first vectors:

```text
Odd leaf/node promotion duplicates the final child at that level.
```

Example:

```text
[A, B, C] -> [hash(A,B), hash(C,C)]
```

Reason:

```text
This is simple, deterministic, and easy to vectorize.
```

No-go:

```text
If this rule changes, all vectors must be regenerated.
```

---

## 9. Proof format, deferred but bounded

Future account proof must include:

```text
schema
version
checkpoint_hash
state_root
account_leaf
account_leaf_hash
sort_key
sibling_path
path_directions
domain_separator
hash_algorithm
```

Future hold proof must include:

```text
schema
version
account_id
holds_root
hold_leaf
hold_leaf_hash
sort_key
sibling_path
path_directions
domain_separator
hash_algorithm
```

Rules:

```text
- no proof may omit domain separator
- no proof may omit hash algorithm
- no proof may rely on implicit tree rules
```

---

## 10. Why not sparse Merkle tree first?

Sparse Merkle tree is attractive later because:

```text
stable proof size
non-inclusion proofs
large keyspace support
```

But it is deferred because it requires early decisions on:

```text
key hashing
path length
empty subtree constants
non-inclusion proofs
path compression if any
update semantics
migration story
```

Those are too many consensus-critical choices before internal ROC is proven.

Phase 1 should optimize for:

```text
boring replay
small surface
clear vectors
independent verification
```

---

## 11. Phase 1 state-root no-go list

Do not implement state roots until:

```text
[ ] AccountStateV1 canonical bytes exist
[ ] HoldStateV1 canonical bytes exist
[ ] EmptyTreeV1 canonical bytes exist
[ ] account leaf sort vector exists
[ ] hold leaf sort vector exists
[ ] duplicate key rejection vector exists
[ ] unordered input same root vector exists
[ ] odd leaf duplication vector exists
[ ] empty tree root vector exists
[ ] closed-hold compaction vector exists
[ ] independent verifier can reproduce vectors
```

---

## 12. Remaining open decisions

These remain open but narrowed:

```text
[ ] exact AccountStateV1 DTO field set
[ ] exact HoldStateV1 DTO field set
[x] exact receipt tree order — finalized in Section 5.3
[ ] exact account proof DTO
[ ] exact hold proof DTO
[ ] exact permissions_root model
```

These do not block this document as QC-0A-R1 guidance.

They block root-producing code.
