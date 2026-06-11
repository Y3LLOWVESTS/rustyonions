# Concurrent Holds and Operation IDs

RO:WHAT — QC-0A model for idempotency, operation IDs, account sequences, concurrent holds, and closed-hold compaction.
RO:WHY — QuickChain must support multi-tab paid UX without nonce holes, duplicate commits, ambiguous replay, or unbounded hold bloat.
RO:INTERACTS — svc-wallet, ron-ledger, ron-proto quickchain DTOs, future account/hold roots, future receipt roots, future checkpoint replay.
RO:INVARIANTS — wallet remains mutation front-door; client idempotency keys are hints, not authority; replay must be deterministic; closed holds compact only through deterministic receipt-root rules.
RO:METRICS — future duplicate operation/idempotency reject counters, hold-expiry counters, hold-compaction counters.
RO:CONFIG — future maximum open holds, session budget limits, expiration epoch windows, idempotency retention windows.
RO:SECURITY — duplicate operation_id/idempotency_key must not create duplicate economic commits; hold compaction must not allow hold resurrection.
RO:TEST — future concurrent_holds_replay_vector_001, expired_hold_epoch_transition_vector_001, duplicate_operation_id_rejected, duplicate_idempotency_returns_original_receipt.

---

## 0. Status

This is a QC-0A-R1 decision record.

It is still pre-implementation.

It does not implement hold roots, receipts, account roots, hashes, validators, pruning, or settlement.

It narrows the model enough that future DTOs and golden vectors have a concrete target.

---

## 1. Rule

QuickChain must not rely on a fragile strict-single-nonce UX model.

A stuck paid action must not freeze unrelated CrabLink paid actions.

The system needs both:

```text
good UX:
  multiple tabs / views / streams can prepare paid actions concurrently

economic safety:
  retries do not double commit
  captures do not double spend
  releases do not resurrect funds
  holds do not bloat state forever
  replay is deterministic
```

---

## 2. Scope decisions

### 2.1 operation_id

Decision:

```text
operation_id is a backend-assigned durable ledger-operation identifier.
```

Properties:

```text
- unique across ledger mutation attempts
- treated as immutable replay input
- generated before ledger append
- never generated during root replay
- bound to canonical_operation_hash
- conflict checked if reused with different payload
- appears in every economic receipt
```

Phase 1 candidate string form:

```text
op_<32 lowercase hex>
```

Example:

```text
op_0123456789abcdef0123456789abcdef
```

Rules:

```text
- Client may send a suggested operation ID only in dev/testing flows.
- Production backend may ignore client suggestion and mint its own.
- Once accepted, operation_id is durable.
- Reusing operation_id with identical canonical_operation_hash returns the original receipt.
- Reusing operation_id with different canonical_operation_hash rejects as conflict.
```

No-go:

```text
operation_id must not be a UI-only nonce.
operation_id must not be generated during deterministic replay.
operation_id must not be scoped only to a browser session.
```

### 2.2 idempotency_key

Decision:

```text
idempotency_key is a retry key scoped by account_id + operation_family + canonical_operation_hash.
```

Purpose:

```text
- lets clients retry after timeout
- maps repeat requests to the original operation/receipt
- does not itself authorize spend
- does not replace operation_id uniqueness
```

Suggested scope tuple:

```text
(account_id, operation_family, idempotency_key)
```

Conflict rule:

```text
same scope + same canonical_operation_hash:
  return original receipt

same scope + different canonical_operation_hash:
  reject as idempotency conflict
```

Suggested string limits:

```text
- UTF-8 visible ASCII
- max 128 bytes
- no secrets
- no account IDs embedded by the client
```

No-go:

```text
Client idempotency keys are hints, not economic authority.
```

### 2.3 canonical_operation_hash

Decision:

```text
canonical_operation_hash commits to the operation intent before receipt fields exist.
```

It should eventually cover:

```text
operation_family
account_id
counterparty_account_id where applicable
asset
amount_minor
purpose
hold_id/session_budget_id where applicable
policy_hash
chain_params_hash where applicable
idempotency scope
```

It must not cover:

```text
receipt_hash
settlement_status
validator signatures
checkpoint fields
wall-clock data not part of the accepted operation
local UI state
```

Status:

```text
No hash is implemented in QC-0A.
The hash input must be frozen in TEST_VECTORS.md before code.
```

### 2.4 account_sequence

Decision:

```text
account_sequence is ledger-assigned at commit time.
```

Purpose:

```text
- deterministic account replay
- replay ordering inside one account
- audit/proof clarity
```

Rules:

```text
- account_sequence is not a preflight UX nonce.
- account_sequence is assigned by wallet/ledger commit path.
- account_sequence may be sequential internally without forcing UI to pre-reserve nonces.
- failed prepare attempts do not create user-visible nonce holes.
- each committed account-affecting receipt records the resulting account_sequence.
```

No-go:

```text
Do not require CrabLink to know the next account_sequence before a paid action.
```

### 2.5 hold_id

Decision:

```text
hold_id identifies one hold lifecycle.
```

Lifecycle:

```text
open -> captured
open -> released
open -> expired
```

Rules:

```text
- hold_id is assigned by backend wallet path.
- hold_id is unique within ledger replay history.
- hold_id is included in hold/capture/release receipts.
- hold_id cannot be reused to create a new hold after terminal state.
```

Suggested Phase 1 candidate string form:

```text
hold_<32 lowercase hex>
```

### 2.6 session_budget_id

Decision:

```text
session_budget_id groups multiple small paid captures under one user-confirmed budget.
```

Status:

```text
future optional model
not required for first paid site visit proof
not required for first hold root vector
```

Rules when introduced:

```text
- explicit user confirmation required
- maximum budget required
- expiration epoch required
- capture rules must be deterministic
- no uncapped spend authority
```

---

## 3. Replay identity

Future replay protection must include:

```text
account_id
operation_id
idempotency_key
operation_family
canonical_operation_hash
hold_id or session_budget_id when applicable
ledger sequence bounds
account_sequence
```

Authority:

```text
client idempotency key:
  retry hint only

server-side durable uniqueness:
  authority

svc-wallet:
  mutation front-door

ron-ledger:
  durable replay truth
```

---

## 4. Required behavior

```text
same idempotency_key + same operation:
  returns original receipt

same idempotency_key + different operation:
  rejects as idempotency conflict

same operation_id + same canonical_operation_hash:
  returns original receipt

same operation_id + different canonical_operation_hash:
  rejects as operation conflict

hold capture retry:
  returns original capture receipt

hold release retry:
  returns original release receipt

expired hold:
  handled by deterministic epoch/ledger rule, not local timer race
```

---

## 5. Concurrent hold model

### 5.1 Account balance fields

Account state must distinguish:

```text
balance_minor:
  total account balance including held funds

held_minor:
  sum of currently open holds

available_minor:
  balance_minor - held_minor
```

Rules:

```text
- opening a hold increases held_minor and decreases available_minor.
- capturing a hold decreases balance_minor and held_minor.
- releasing a hold decreases held_minor and increases available_minor.
- expiring a hold behaves like deterministic release.
- no field may become negative.
```

### 5.2 Open holds

Open holds are active state.

They belong in the future holds_root.

Minimum open hold fields:

```text
hold_id
account_id
counterparty_account_id
amount_minor
purpose
created_at_epoch
expires_at_epoch
status = "open"
operation_id
idempotency_key
policy_hash
```

Rules:

```text
- multiple open holds per account are allowed within policy limits.
- open holds are sorted by hold_id for root purposes.
- maximum open holds per account must be policy-limited.
- maximum total held_minor must not exceed balance_minor.
```

### 5.3 Terminal holds

Terminal states:

```text
captured
released
expired
```

Terminal hold receipts must exist.

Terminal holds must not remain in active state forever.

---

## 6. Closed-hold compaction rule

Decision:

```text
The active holds_root commits only to currently open holds.
```

Terminal hold history is proven through receipt_root, not through indefinite hold-state retention.

Compaction rule:

```text
1. A hold opens and appears in active holds_root.
2. A terminal receipt is committed:
   - capture
   - release
   - expiry release
3. The terminal receipt enters the receipt_root for the epoch.
4. At the deterministic epoch transition that includes the terminal receipt,
   the hold is removed from the next active holds_root.
5. Future proof of that hold lifecycle is through receipt inclusion proofs,
   not active hold state.
```

Safety requirements:

```text
- hold_id uniqueness remains durable.
- terminal receipt must be included before removal.
- replay must reject capture/release against a missing or already-terminal active hold.
- before pruning is proven, full receipt history remains available.
- after future pruning, receipt inclusion proof plus checkpoint proof must prove closure.
```

No-go:

```text
Do not compact a hold merely because local time passed.
Do not compact a hold before terminal receipt inclusion.
Do not keep closed holds forever in active state.
Do not allow hold_id reuse after compaction.
```

---

## 7. Expiration rule

Decision:

```text
Hold expiration is epoch-based, not wall-clock based.
```

A hold may include:

```text
created_at_epoch
expires_at_epoch
```

At epoch transition:

```text
if current_epoch >= expires_at_epoch and hold is still open:
  deterministic expiry release is eligible
```

Rules:

```text
- expiry order is stable sorted by hold_id.
- expiry effect must produce a durable receipt or equivalent replay artifact.
- expiry must not depend on local timers.
- expiry must not depend on scheduler order.
- expiry cannot spend funds; it releases held funds.
```

No-go:

```text
No root-producing transition may read system wall-clock to decide expiry.
```

---

## 8. Concrete replay vector sketch

This is a sketch, not a production vector.

It becomes a production vector only after canonical bytes, domains, and preimage framing are frozen in TEST_VECTORS.md.

### 8.1 Initial state

```text
account_id = "acct_visitor_b"
asset = "roc"
balance_minor = "1000"
held_minor = "0"
available_minor = "1000"
account_sequence = 0
open_holds = []
```

### 8.2 Operations

```text
1. open hold H1
   hold_id = "hold_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
   operation_id = "op_11111111111111111111111111111111"
   amount_minor = "100"
   expires_at_epoch = "epoch_0002"

2. open hold H2
   hold_id = "hold_bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
   operation_id = "op_22222222222222222222222222222222"
   amount_minor = "250"
   expires_at_epoch = "epoch_0002"

3. capture H2
   operation_id = "op_33333333333333333333333333333333"

4. retry capture H2 with same idempotency_key
   expected: return original capture receipt, no balance change

5. release H1
   operation_id = "op_44444444444444444444444444444444"

6. retry release H1 with same idempotency_key
   expected: return original release receipt, no balance change
```

### 8.3 Expected human-readable state transitions

```text
after open H1:
  balance_minor = "1000"
  held_minor = "100"
  available_minor = "900"
  open_holds = [H1]

after open H2:
  balance_minor = "1000"
  held_minor = "350"
  available_minor = "650"
  open_holds = [H1, H2]

after capture H2:
  balance_minor = "750"
  held_minor = "100"
  available_minor = "650"
  open_holds = [H1]

after retry capture H2:
  no state change

after release H1:
  balance_minor = "750"
  held_minor = "0"
  available_minor = "750"
  open_holds = []

after retry release H1:
  no state change
```

### 8.4 Expected proof implications

```text
H1 open receipt is in receipt_root.
H2 open receipt is in receipt_root.
H2 capture receipt is in receipt_root.
H1 release receipt is in receipt_root.
After terminal receipts are included, active holds_root is empty.
Closed hold lifecycle remains provable by receipt inclusion.
```

---

## 9. Open decisions remaining

These are deliberately narrowed.

```text
[ ] exact canonical OperationIntentV1 DTO fields
[ ] exact canonical HoldStateV1 DTO fields
[ ] exact idempotency conflict error wire name
[ ] exact expiry receipt op_class
[ ] exact session_budget_id model, deferred until needed
```

These do not block this document from serving as QC-0A-R1 guidance.

They do block root-producing code.

---

## 10. No-go

Do not implement hold roots until:

```text
[ ] canonical HoldStateV1 bytes exist
[ ] canonical OperationIntentV1 bytes exist
[ ] concurrent_holds_replay_vector_001 exists
[ ] expired_hold_epoch_transition_vector_001 exists
[ ] duplicate_operation_id_rejected exists
[ ] duplicate_idempotency_returns_original_receipt exists
[ ] closed-hold compaction vector exists
```
