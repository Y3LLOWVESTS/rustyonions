# QuickChain Test Vectors

RO:WHAT — QC-0A format and acceptance rules for future QuickChain canonical byte/hash/root vectors.
RO:WHY — No root-producing code may ship without independent replayable vectors.
RO:INTERACTS — ron-proto canonical JSON helpers, future ron-ledger roots, future receipt/account/hold/checkpoint hashes.
RO:INVARIANTS — no fake hashes; no unstable golden vectors; no Phase 1 roots until vectors are frozen and independently reproducible.
RO:METRICS — none.
RO:CONFIG — none.
RO:SECURITY — fake vectors create false confidence and are forbidden.
RO:TEST — future canonical_receipt_vector_001, canonical_account_state_vector_001, canonical_hold_state_vector_001, checkpoint_hash_vector_001.

---

## 0. Status

This is a QC-0A-R1 vector-format decision record.

No production hash vectors exist yet.

This is intentional.

Canonical JSON byte tests may exist now, but hash/root vectors must wait until:

```text
canonical byte rules are frozen
domain separator strings are frozen
preimage framing is frozen
receipt/account/hold/checkpoint schemas are frozen
BLAKE3 call site is placed in the correct crate/layer
independent replay expectations are documented
```

---

## 1. Vector status levels

Vectors use explicit status.

```text
sketch:
  human-readable scenario only
  may contain placeholders
  cannot drive root-producing tests

locked_bytes:
  canonical_payload_utf8 and canonical_payload_hex are frozen
  no expected_b3 required yet
  may drive canonical byte tests

locked_hash:
  preimage_hex and expected_b3 are frozen
  may drive hash/root tests
```

No-go:

```text
Do not use sketch vectors as production tests.
Do not use placeholder expected_b3 values.
Do not call a vector golden until it is locked_hash.
```

---

## 2. Vector schema

Future vectors must use this outer shape.

```json
{
  "schema": "quickchain.test-vector.v1",
  "vector_id": "canonical_account_state_vector_001",
  "status": "locked_bytes",
  "purpose": "account_state_canonical_bytes",
  "domain_separator": "quickchain.account-state.v1",
  "canonical_encoding": "quickchain.canonical-json.v1",
  "preimage_framing": "domain_separator_bytes || 0x00 || canonical_payload_bytes",
  "hash_algorithm": "blake3-256",
  "human_readable_json": {},
  "canonical_payload_utf8": "",
  "canonical_payload_hex": "",
  "preimage_hex": null,
  "expected_b3": null,
  "notes": []
}
```

Required fields:

```text
schema
vector_id
status
purpose
domain_separator
canonical_encoding
preimage_framing
hash_algorithm
human_readable_json
canonical_payload_utf8
canonical_payload_hex
preimage_hex
expected_b3
notes
```

Rules:

```text
- status must be sketch, locked_bytes, or locked_hash.
- locked_bytes requires canonical_payload_utf8 and canonical_payload_hex.
- locked_hash requires preimage_hex and expected_b3.
- expected_b3 must be b3:<64 lowercase hex> for locked_hash.
- preimage_hex must equal hex(domain || 0x00 || payload) for locked_hash.
- human_readable_json is explanatory and not used as authority.
- canonical_payload_utf8 and canonical_payload_hex are the byte authority.
```

---

## 3. Preimage framing

Decision:

```text
preimage = domain_separator_bytes || 0x00 || canonical_payload_bytes
```

Example:

```text
domain_separator:
  quickchain.account-state.v1

delimiter:
  00

canonical_payload_utf8:
  {"schema":"quickchain.account-state.v1","account_id":"acct_visitor_b"}
```

The preimage bytes are:

```text
utf8("quickchain.account-state.v1")
+
00
+
utf8(canonical_payload_utf8)
```

No-go:

```text
No hash/root test may use a different preimage frame.
```

### 3.1 Receipt ordering bytes

Decision:

```text
receipt_sort_key =
    u64_be(ledger_seq_start)
    || utf8(txid)
```

Required locked-byte coverage:

```text
- exact 8-byte unsigned big-endian sequence prefix
- bytewise txid tie-breaking
- unordered inputs produce identical sorted output
- duplicate complete sort keys reject
- zero ledger_seq_start rejects
- invalid txid tokens reject
```

This vector category does not contain receipt hashes or receipt roots.

---

## 4. Vector file layout

Recommended future directory:

```text
docs/quickchain/vectors/
  canonical_account_state_vector_001.json
  canonical_hold_state_vector_001.json
  canonical_receipt_vector_001.json
  receipt_sort_key_locked_bytes_v1.json
  receipt_hash_domain_separation_001.json
  state_root_determinism_vector_001.json
  receipt_root_determinism_vector_001.json
  checkpoint_hash_vector_001.json
  concurrent_holds_replay_vector_001.json
  expired_hold_epoch_transition_vector_001.json
  duplicate_operation_id_rejected_001.json
  duplicate_idempotency_returns_original_receipt_001.json
```

This directory does not need to exist until vectors are actually frozen.

---

## 5. First required vectors

### 5.1 Canonical bytes

```text
canonical_receipt_vector_001
canonical_account_state_vector_001
canonical_hold_state_vector_001
canonical_operation_intent_vector_001
canonical_empty_tree_vector_001
receipt_sort_key_locked_bytes_v1
```

### 5.2 Domain separation

```text
receipt_hash_domain_separation_001
account_leaf_domain_separation_001
hold_leaf_domain_separation_001
checkpoint_hash_domain_separation_001
```

### 5.3 Roots

```text
state_root_determinism_vector_001
receipt_root_determinism_vector_001
hold_root_determinism_vector_001
checkpoint_hash_vector_001
unordered_input_same_root_001
odd_leaf_duplication_vector_001
empty_tree_root_vector_001
duplicate_sort_key_rejected_001
```

### 5.4 Wallet/replay behavior

```text
concurrent_holds_replay_vector_001
expired_hold_epoch_transition_vector_001
closed_hold_compaction_vector_001
duplicate_operation_id_rejected_001
duplicate_idempotency_returns_original_receipt_001
idempotency_conflict_rejected_001
```

### 5.5 Reward/event safety

```text
unknown_field_rejected_001
float_money_rejected_001
invalid_money_string_rejected_001
ambiguous_event_class_rejected_001
raw_engagement_reward_formula_rejected_001
reward_epoch_replay_no_double_issue_001
```

---

## 6. Draft vector sketch: account state

Status:

```text
sketch
```

Human-readable payload target:

```json
{
  "schema": "quickchain.account-state.v1",
  "account_id": "acct_visitor_b",
  "asset": "roc",
  "balance_minor": "750",
  "held_minor": "0",
  "available_minor": "750",
  "account_sequence": 4,
  "receipt_root": "b3:1111111111111111111111111111111111111111111111111111111111111111",
  "holds_root": "b3:2222222222222222222222222222222222222222222222222222222222222222",
  "permissions_root": null,
  "updated_at_epoch": "epoch_0001"
}
```

Required future locked_bytes vector:

```text
canonical_payload_utf8 must be exact minified JSON.
canonical_payload_hex must be exact UTF-8 hex.
```

Required future locked_hash vector:

```text
domain_separator = "quickchain.account-state.v1"
preimage_hex = hex(domain || 00 || canonical_payload)
expected_b3 = b3:<64 lowercase hex>
```

---

## 7. Draft vector sketch: concurrent holds

Status:

```text
sketch
```

Initial state:

```text
account_id = "acct_visitor_b"
balance_minor = "1000"
held_minor = "0"
available_minor = "1000"
account_sequence = 0
open_holds = []
```

Operations:

```text
open H1 amount 100
open H2 amount 250
capture H2
retry capture H2
release H1
retry release H1
```

Expected final state:

```text
balance_minor = "750"
held_minor = "0"
available_minor = "750"
open_holds = []
```

Expected safety properties:

```text
retry capture does not double spend
retry release does not change balance
closed holds are absent from active holds_root
terminal hold lifecycle is proven through receipt_root
```

Required future locked vectors:

```text
concurrent_holds_replay_vector_001
closed_hold_compaction_vector_001
duplicate_idempotency_returns_original_receipt_001
```

---

## 8. Production vector acceptance rule

A vector is production-ready only if:

```text
[ ] schema is quickchain.test-vector.v1
[ ] status is locked_hash
[ ] canonical_payload_utf8 is exact
[ ] canonical_payload_hex matches canonical_payload_utf8 bytes
[ ] domain_separator is exact and validated
[ ] preimage_framing is exact
[ ] preimage_hex matches domain || 00 || payload
[ ] hash_algorithm is blake3-256
[ ] expected_b3 is b3:<64 lowercase hex>
[ ] independent verifier reproduces expected_b3
[ ] vector is stable across macOS/Linux
[ ] vector does not depend on map/DB iteration order
```

---

## 9. No-go

If an independent verifier cannot reproduce the same:

```text
receipt_hash
state_root
receipt_root
accounting_root
reward_root
checkpoint_hash
```

from the published vectors, QuickChain Phase 1 must not start.
