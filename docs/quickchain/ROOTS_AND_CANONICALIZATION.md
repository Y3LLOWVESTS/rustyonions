# QuickChain Roots and Canonicalization

RO:WHAT — QC-0A canonicalization and future root-design preflight.
RO:WHY — Consensus, proofs, pruning, and checkpoints require identical bytes, explicit preimage framing, and exact hash domains.
RO:INTERACTS — QUICKCHAIN.MD, ron-proto quickchain DTOs, future ron-ledger roots, svc-wallet receipts, ron-accounting windows, svc-rewarder manifests.
RO:INVARIANTS — no roots before canonical bytes; no fake checkpoint hashes; no floats; no DB-order dependency; no wall-clock dependency; no unknown fields.
RO:METRICS — none.
RO:CONFIG — none; QuickChain remains disabled.
RO:SECURITY — canonicalization drift is a consensus failure.
RO:TEST — ron-proto quickchain canonical JSON tests now; future golden byte/hash/root vectors later.

---

## 0. Status

This is a QC-0A-R1 canonicalization decision record.

It is still pre-implementation.

It does not implement hashes, roots, receipts, validators, pruning, settlement, or anchors.

It freezes enough byte/preimage policy for the next safe DTO/vector work.

---

## 1. Current scope

The current ron-proto canonical JSON helpers are a Phase 0A byte experiment.

They prove:

```text
strict DTO deserialization
unknown-field rejection
minified typed JSON serialization
exact bytes for current DTO experiments
shuffled input normalizes to struct order
```

They do not prove:

```text
receipt hashes
state roots
receipt roots
accounting roots
reward roots
checkpoint hashes
validator signatures
pruning safety
settlement finality
```

---

## 2. Canonical JSON v1 decision

Decision:

```text
First QuickChain vectors use canonical JSON v1.
```

Canonical JSON v1 is not general-purpose JSON canonicalization.

It is a restricted RustyOnions vector encoding profile.

Rules:

```text
- UTF-8 only
- minified JSON only
- root-producing payloads are typed structs
- no unordered maps in root-producing payloads unless explicitly represented as sorted arrays
- field order is schema-defined and test-locked
- all root-producing DTOs reject unknown fields
- money is integer minor-unit string
- no floating point
- b3 values are lowercase b3:<64hex>
- optional fields serialize as explicit null when part of the schema
- no implicit default fields in root-producing vectors
- no skip_serializing_if for root-producing vector payloads
- timestamps are u64 milliseconds only when explicitly required
- epoch IDs are strings when epoch identity, not wall-clock behavior, is required
```

No-go:

```text
Do not hash arbitrary serde_json::Value.
Do not hash BTreeMap payloads unless the map-to-array canonicalization rule is vectorized.
Do not allow unknown fields in root-producing DTOs.
```

---

## 3. Preimage framing decision

Decision:

```text
QuickChain v1 hash preimage framing is:

domain_separator_bytes || 0x00 || canonical_payload_bytes
```

Where:

```text
domain_separator_bytes:
  ASCII bytes of the exact domain separator string

0x00:
  single zero delimiter byte

canonical_payload_bytes:
  canonical JSON v1 UTF-8 bytes
```

Example shape:

```text
quickchain.account-state.v1 || 0x00 || {"schema":"quickchain.account-state.v1",...}
```

Rules:

```text
- delimiter is exactly one byte: 0x00
- domain separator must validate before use
- canonical payload must validate before use
- preimage framing must appear in TEST_VECTORS.md
- no hash code until preimage_hex vectors exist
```

No-go:

```text
No root-producing code may use raw canonical payload bytes without domain separation.
No root-producing code may use a different delimiter without a version bump.
No root-producing code may use implicit domain context.
```

---

## 4. Required future root/hash domains

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

Reserved supporting domains:

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

---

## 5. Canonicalization hazards

Root-producing code must not depend on:

```text
map iteration order
database scan order
wall-clock reads
local timezone
locale sorting
floating point
random IDs generated during replay
unordered JSON
implicit default fields
unknown fields
thread scheduling
retry timing
network timing
service-local mutable caches
```

---

## 6. Sorting rules

All root-producing sorted collections must define:

```text
sort key
duplicate-key behavior
byte encoding
ascending/descending order
tie rejection
```

Default rule:

```text
bytewise ascending over explicit UTF-8 sort keys
```

Duplicate key rule:

```text
duplicate keys are invalid
```

### 6.1 Receipt ordering rule

Decision:

```text
receipt_sort_key =
    u64_be(ledger_seq_start)
    || utf8(txid)
```

Requirements:

```text
- ledger_seq_start is encoded as exactly 8 unsigned big-endian bytes
- ledger_seq_start must be greater than zero
- fixed-width big-endian bytes preserve unsigned numeric order
- txid is the bytewise ascending tie-breaker
- duplicate complete receipt sort keys are invalid
- ledger_seq_end is validated separately and is not part of the sort key
- txid uniqueness and non-overlapping ledger ranges remain ledger replay invariants
```

This rule defines ordering bytes only. It does not hash receipts, build a
receipt tree, or produce a receipt root.

No-go:

```text
Do not use DB iteration order.
Do not use hash map iteration order.
Do not use locale-aware string comparison.
```

---

## 7. Money rules

All ROC money values must be:

```text
integer minor-unit strings
```

Allowed examples:

```text
"0"
"1"
"1000000"
```

Forbidden examples:

```text
1
1.0
"1.0"
"+1"
"-1"
"01"
"1_000"
"1 ROC"
```

Rules:

```text
- no floats
- no signed strings
- no leading zero unless exactly "0"
- no separators
- no units inside value
```

---

## 8. Null and optional fields

Root-producing DTOs must avoid optional ambiguity.

Rules:

```text
- optional fields must be explicitly declared
- optional fields serialize as null when absent
- absent optional fields are not allowed in canonical payloads unless the schema says a field may be omitted
- root-producing DTOs should prefer explicit null over omission
```

Reason:

```text
Omitted-vs-null drift creates cross-language vector hazards.
```

---

## 9. Hash algorithm

Decision for QuickChain v1 vectors:

```text
hash_algorithm = "blake3-256"
content ID encoding = "b3:<64 lowercase hex>"
```

Rules:

```text
- expected hashes in production vectors must be b3:<64 lowercase hex>
- all vectors must name hash_algorithm
- all vectors must name domain_separator
```

No-go:

```text
No fake hashes.
No placeholder hashes in production vectors.
```

---

## 10. Root-producing code no-go

No Phase 1 roots until:

```text
[ ] canonical receipt bytes are defined
[ ] canonical account-state bytes are defined
[ ] canonical hold-state bytes are defined
[ ] canonical checkpoint bytes are defined
[ ] hash domains are exact
[ ] preimage framing is exact
[ ] money string rules are tested
[ ] null/optional rules are tested
[ ] sorted collection rules are tested
[ ] golden vectors exist
[ ] independent verifier reproduces expected b3 hashes
```
