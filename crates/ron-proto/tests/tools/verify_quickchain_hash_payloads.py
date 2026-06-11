#!/usr/bin/env python3
"""Independent pure-Python verification for QuickChain locked_hash payload vectors."""

import json
import re
import sys
from pathlib import Path

BASE = Path(__file__).resolve().parents[1] / "vectors" / "quickchain" / "hash_payloads"
OUTER = {
    "schema", "version", "vector_id", "status", "purpose", "domain_separator",
    "canonical_encoding", "preimage_framing", "hash_algorithm", "human_readable_json",
    "canonical_payload_utf8", "canonical_payload_hex", "preimage_hex", "expected_b3", "notes",
}
CASES = {
    "operation_hash_payload_locked_hash_v1.json": (
        "canonical_operation_hash_payload_vector_001", "operation_hash_payload_locked_hash",
        "quickchain.operation-intent.v1", "quickchain.operation-hash-payload.v1",
        {"schema", "version", "chain_id", "operation_id", "op_class", "actor_account_id",
         "counterparty_account_id", "asset", "amount_minor", "purpose", "hold_id",
         "session_budget_id", "policy_hash", "chain_params_hash", "idempotency_scope_account_id",
         "idempotency_scope_operation_family", "idempotency_key"}, "operation",
    ),
    "receipt_hash_payload_locked_hash_v1.json": (
        "canonical_receipt_hash_payload_vector_001", "receipt_hash_payload_locked_hash",
        "quickchain.receipt.v1", "quickchain.receipt-hash-payload.v1",
        {"schema", "version", "chain_id", "txid", "operation_id", "operation_hash", "op",
         "op_class", "from_account_id", "to_account_id", "asset", "amount_minor",
         "account_sequence", "hold_id", "session_budget_id", "idempotency_key",
         "ledger_seq_start", "ledger_seq_end", "previous_ledger_root", "new_ledger_root",
         "produced_at_ms"}, "receipt",
    ),
    "account_leaf_payload_locked_hash_v1.json": (
        "canonical_account_leaf_payload_vector_001", "account_leaf_payload_locked_hash",
        "quickchain.account-state.v1", "quickchain.account-leaf-payload.v1",
        {"schema", "version", "chain_id", "account_id", "asset", "balance_minor", "held_minor",
         "available_minor", "account_sequence", "receipt_root", "holds_root", "permissions_root",
         "updated_at_epoch"}, "account",
    ),
    "active_hold_leaf_payload_locked_hash_v1.json": (
        "canonical_active_hold_leaf_payload_vector_001", "active_hold_leaf_payload_locked_hash",
        "quickchain.hold-state.v1", "quickchain.active-hold-leaf-payload.v1",
        {"schema", "version", "chain_id", "hold_id", "account_id", "counterparty_account_id",
         "amount_minor", "purpose", "created_at_epoch", "expires_at_epoch", "status",
         "operation_id", "idempotency_key", "policy_hash"}, "hold",
    ),
    "unsigned_checkpoint_payload_locked_hash_v1.json": (
        "canonical_unsigned_checkpoint_payload_vector_001", "unsigned_checkpoint_payload_locked_hash",
        "quickchain.checkpoint.v1", "quickchain.unsigned-checkpoint-payload.v1",
        {"schema", "version", "chain_id", "height", "epoch_id", "execution_spec_version",
         "previous_checkpoint_hash", "previous_state_root", "new_state_root", "receipt_root",
         "accounting_snapshot_root", "reward_manifest_root", "data_availability_root", "policy_hash",
         "validator_set_hash", "chain_params_hash", "canonical_encoding", "state_root_scheme",
         "receipt_root_scheme", "supply_delta", "conservation", "settlement_mode", "started_at_ms",
         "ended_at_ms", "produced_at_ms"}, "checkpoint",
    ),
}
B3_RE = re.compile(r"^b3:[0-9a-f]{64}$")
OP_RE = re.compile(r"^op_[0-9a-f]{32}$")
HOLD_RE = re.compile(r"^hold_[0-9a-f]{32}$")
MONEY_RE = re.compile(r"^(0|[1-9][0-9]*)$")
SIGNED_RE = re.compile(r"^(0|[1-9][0-9]*|-[1-9][0-9]*)$")
TOKEN_RE = re.compile(r"^[a-z0-9_.:@/-]+$")
HEX_RE = re.compile(r"^[0-9a-f]+$")

IV = [0x6A09E667, 0xBB67AE85, 0x3C6EF372, 0xA54FF53A,
      0x510E527F, 0x9B05688C, 0x1F83D9AB, 0x5BE0CD19]
PERM = [2, 6, 3, 10, 7, 0, 4, 13, 1, 11, 12, 5, 9, 14, 15, 8]
CHUNK_START, CHUNK_END, PARENT, ROOT = 1, 2, 4, 8
BLOCK_LEN, CHUNK_LEN, MASK = 64, 1024, 0xFFFFFFFF


def fail(message):
    raise ValueError(message)


def exact_keys(value, expected, label):
    if not isinstance(value, dict) or set(value) != expected:
        actual = set(value) if isinstance(value, dict) else set()
        fail("{} keys mismatch: missing={} extra={}".format(
            label, sorted(expected - actual), sorted(actual - expected)))


def token(value, label):
    if not isinstance(value, str) or not TOKEN_RE.fullmatch(value):
        fail("{} is not a canonical token".format(label))


def b3(value, label, optional=False):
    if optional and value is None:
        return
    if not isinstance(value, str) or not B3_RE.fullmatch(value):
        fail("{} is not canonical b3 lowercase hex".format(label))


def unsigned(value, label):
    if not isinstance(value, str) or not MONEY_RE.fullmatch(value) or int(value) >= 2**128:
        fail("{} is not canonical u128 minor-unit text".format(label))
    return int(value)


def signed(value, label):
    if not isinstance(value, str) or not SIGNED_RE.fullmatch(value):
        fail("{} is not canonical signed minor-unit text".format(label))
    magnitude = int(value[1:] if value.startswith("-") else value)
    if magnitude >= 2**128:
        fail("{} magnitude exceeds u128".format(label))
    return int(value)


def positive(value, label):
    if not isinstance(value, int) or isinstance(value, bool) or value <= 0:
        fail("{} must be a positive integer".format(label))


def validate_semantics(kind, payload):
    if payload["version"] != 1 or payload["chain_id"] != "roc-dev":
        fail("common payload identity mismatch")

    if kind == "operation":
        if not OP_RE.fullmatch(payload["operation_id"]):
            fail("bad operation_id")
        token(payload["actor_account_id"], "actor_account_id")
        if payload["counterparty_account_id"] is not None:
            token(payload["counterparty_account_id"], "counterparty")
        if payload["asset"] != "roc":
            fail("operation asset must be roc")
        unsigned(payload["amount_minor"], "amount_minor")
        token(payload["purpose"], "purpose")
        if not HOLD_RE.fullmatch(payload["hold_id"]):
            fail("bad operation hold_id")
        if payload["session_budget_id"] is not None:
            fail("fixture session budget must be null")
        b3(payload["policy_hash"], "policy_hash")
        b3(payload["chain_params_hash"], "chain_params_hash")
        if payload["idempotency_scope_account_id"] != payload["actor_account_id"]:
            fail("scope account mismatch")
        if payload["idempotency_scope_operation_family"] != payload["op_class"]:
            fail("scope family mismatch")

    elif kind == "receipt":
        token(payload["txid"], "txid")
        if not OP_RE.fullmatch(payload["operation_id"]):
            fail("bad receipt operation_id")
        b3(payload["operation_hash"], "operation_hash")
        if payload["op_class"] != "hold_open":
            fail("receipt fixture must be hold_open")
        token(payload["from_account_id"], "from_account_id")
        token(payload["to_account_id"], "to_account_id")
        if payload["asset"] != "roc":
            fail("receipt asset must be roc")
        unsigned(payload["amount_minor"], "amount_minor")
        positive(payload["account_sequence"], "account_sequence")
        if not HOLD_RE.fullmatch(payload["hold_id"]):
            fail("bad receipt hold_id")
        if payload["session_budget_id"] is not None:
            fail("fixture session budget must be null")
        positive(payload["ledger_seq_start"], "ledger_seq_start")
        positive(payload["ledger_seq_end"], "ledger_seq_end")
        if payload["ledger_seq_end"] < payload["ledger_seq_start"]:
            fail("reversed ledger range")
        b3(payload["previous_ledger_root"], "previous_ledger_root")
        b3(payload["new_ledger_root"], "new_ledger_root")
        positive(payload["produced_at_ms"], "produced_at_ms")

    elif kind == "account":
        token(payload["account_id"], "account_id")
        if payload["asset"] != "roc":
            fail("account asset must be roc")
        balance = unsigned(payload["balance_minor"], "balance_minor")
        held = unsigned(payload["held_minor"], "held_minor")
        available = unsigned(payload["available_minor"], "available_minor")
        if balance != held + available:
            fail("account arithmetic mismatch")
        b3(payload["receipt_root"], "receipt_root")
        b3(payload["holds_root"], "holds_root")
        b3(payload["permissions_root"], "permissions_root", optional=True)
        token(payload["updated_at_epoch"], "updated_at_epoch")

    elif kind == "hold":
        if not HOLD_RE.fullmatch(payload["hold_id"]):
            fail("bad active hold_id")
        token(payload["account_id"], "account_id")
        token(payload["counterparty_account_id"], "counterparty")
        unsigned(payload["amount_minor"], "amount_minor")
        token(payload["purpose"], "purpose")
        token(payload["created_at_epoch"], "created_at_epoch")
        token(payload["expires_at_epoch"], "expires_at_epoch")
        if payload["status"] != "open":
            fail("active hold status must be open")
        if not OP_RE.fullmatch(payload["operation_id"]):
            fail("bad active hold operation_id")
        b3(payload["policy_hash"], "policy_hash")

    else:
        for field in (
            "previous_checkpoint_hash",
            "previous_state_root",
            "new_state_root",
            "receipt_root",
            "accounting_snapshot_root",
            "reward_manifest_root",
            "policy_hash",
            "chain_params_hash",
        ):
            b3(payload[field], field)

        b3(payload["data_availability_root"], "data_availability_root", optional=True)
        b3(payload["validator_set_hash"], "validator_set_hash", optional=True)

        if payload["canonical_encoding"] != "json-v1":
            fail("checkpoint encoding mismatch")
        if payload["settlement_mode"] != "local_root":
            fail("checkpoint settlement mode mismatch")

        exact_keys(
            payload["supply_delta"],
            {"issued_minor", "burned_minor", "net_minor"},
            "supply_delta",
        )
        issued = unsigned(payload["supply_delta"]["issued_minor"], "issued_minor")
        burned = unsigned(payload["supply_delta"]["burned_minor"], "burned_minor")
        if signed(payload["supply_delta"]["net_minor"], "net_minor") != issued - burned:
            fail("net supply mismatch")

        exact_keys(
            payload["conservation"],
            {
                "debits_minor",
                "credits_minor",
                "issue_exceptions_minor",
                "burn_exceptions_minor",
                "valid",
            },
            "conservation",
        )
        conservation = payload["conservation"]
        debit_side = (
            unsigned(conservation["debits_minor"], "debits_minor")
            + unsigned(conservation["issue_exceptions_minor"], "issue_exceptions_minor")
        )
        credit_side = (
            unsigned(conservation["credits_minor"], "credits_minor")
            + unsigned(conservation["burn_exceptions_minor"], "burn_exceptions_minor")
        )

        if conservation["valid"] is not True or debit_side != credit_side:
            fail("conservation mismatch")

        positive(payload["started_at_ms"], "started_at_ms")
        positive(payload["ended_at_ms"], "ended_at_ms")
        positive(payload["produced_at_ms"], "produced_at_ms")

        if not (
            payload["started_at_ms"]
            <= payload["ended_at_ms"]
            <= payload["produced_at_ms"]
        ):
            fail("checkpoint time order mismatch")


def rotr(value, shift):
    return ((value >> shift) | ((value << (32 - shift)) & MASK)) & MASK


def mix(state, a, b, c, d, x, y):
    state[a] = (state[a] + state[b] + x) & MASK
    state[d] = rotr(state[d] ^ state[a], 16)
    state[c] = (state[c] + state[d]) & MASK
    state[b] = rotr(state[b] ^ state[c], 12)
    state[a] = (state[a] + state[b] + y) & MASK
    state[d] = rotr(state[d] ^ state[a], 8)
    state[c] = (state[c] + state[d]) & MASK
    state[b] = rotr(state[b] ^ state[c], 7)


def compress(cv, block_words, counter, block_len, flags):
    state = list(cv) + IV[:4] + [
        counter & MASK,
        (counter >> 32) & MASK,
        block_len,
        flags,
    ]
    message = list(block_words)

    for round_index in range(7):
        mix(state, 0, 4, 8, 12, message[0], message[1])
        mix(state, 1, 5, 9, 13, message[2], message[3])
        mix(state, 2, 6, 10, 14, message[4], message[5])
        mix(state, 3, 7, 11, 15, message[6], message[7])
        mix(state, 0, 5, 10, 15, message[8], message[9])
        mix(state, 1, 6, 11, 12, message[10], message[11])
        mix(state, 2, 7, 8, 13, message[12], message[13])
        mix(state, 3, 4, 9, 14, message[14], message[15])

        if round_index != 6:
            message = [message[index] for index in PERM]

    for index in range(8):
        state[index] ^= state[index + 8]
        state[index + 8] ^= cv[index]

    return [word & MASK for word in state]


def words(block):
    padded = block + bytes(BLOCK_LEN - len(block))
    return [
        int.from_bytes(padded[index:index + 4], "little")
        for index in range(0, BLOCK_LEN, 4)
    ]


class Output:
    def __init__(self, cv, block_words, counter, block_len, flags):
        self.cv = list(cv)
        self.words = list(block_words)
        self.counter = counter
        self.block_len = block_len
        self.flags = flags

    def chaining_value(self):
        return compress(
            self.cv,
            self.words,
            self.counter,
            self.block_len,
            self.flags,
        )[:8]

    def root_bytes(self):
        result = compress(
            self.cv,
            self.words,
            0,
            self.block_len,
            self.flags | ROOT,
        )
        return b"".join(word.to_bytes(4, "little") for word in result)[:32]


def chunk_output(chunk, counter):
    cv = list(IV)
    blocks = [
        chunk[index:index + BLOCK_LEN]
        for index in range(0, len(chunk), BLOCK_LEN)
    ] or [b""]

    for index, block in enumerate(blocks):
        flags = 0
        if index == 0:
            flags |= CHUNK_START
        if index == len(blocks) - 1:
            flags |= CHUNK_END

        output = Output(cv, words(block), counter, len(block), flags)

        if index == len(blocks) - 1:
            return output

        cv = output.chaining_value()

    raise AssertionError("unreachable")


def parent(left, right):
    return Output(IV, list(left) + list(right), 0, BLOCK_LEN, PARENT)


def blake3_256(data):
    chunks = [
        data[index:index + CHUNK_LEN]
        for index in range(0, len(data), CHUNK_LEN)
    ] or [b""]

    stack = []

    for index, chunk in enumerate(chunks[:-1]):
        cv = chunk_output(chunk, index).chaining_value()
        total = index + 1

        while total & 1 == 0:
            cv = parent(stack.pop(), cv).chaining_value()
            total >>= 1

        stack.append(cv)

    output = chunk_output(chunks[-1], len(chunks) - 1)

    while stack:
        output = parent(stack.pop(), output.chaining_value())

    return output.root_bytes()


def self_test_blake3():
    known = {
        b"": "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262",
        b"abc": "6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85",
    }

    for message, expected in known.items():
        if blake3_256(message).hex() != expected:
            fail("BLAKE3 self-test failed")


def verify(path, spec):
    vector_id, purpose, domain, payload_schema, payload_keys, kind = spec
    root = json.loads(path.read_text(encoding="utf-8"))
    exact_keys(root, OUTER, path.name)

    metadata = (
        root["schema"],
        root["version"],
        root["vector_id"],
        root["status"],
        root["purpose"],
        root["domain_separator"],
    )
    expected_metadata = (
        "quickchain.test-vector.v1",
        1,
        vector_id,
        "locked_hash",
        purpose,
        domain,
    )

    if metadata != expected_metadata:
        fail("{} metadata mismatch".format(path.name))

    if root["canonical_encoding"] != "quickchain.canonical-json.v1":
        fail("{} canonical encoding mismatch".format(path.name))
    if root["preimage_framing"] != (
        "domain_separator_bytes || 0x00 || canonical_payload_bytes"
    ):
        fail("{} preimage framing mismatch".format(path.name))
    if root["hash_algorithm"] != "blake3-256":
        fail("{} hash algorithm mismatch".format(path.name))

    if not root["notes"] or not all(
        isinstance(note, str) and note for note in root["notes"]
    ):
        fail("{} notes mismatch".format(path.name))

    payload = root["human_readable_json"]
    exact_keys(payload, payload_keys, path.name + " payload")

    if payload["schema"] != payload_schema:
        fail("{} payload schema mismatch".format(path.name))

    validate_semantics(kind, payload)

    canonical = json.dumps(
        payload,
        ensure_ascii=False,
        separators=(",", ":"),
    )
    payload_bytes = canonical.encode("utf-8")

    if root["canonical_payload_utf8"] != canonical:
        fail("{} canonical UTF-8 mismatch".format(path.name))
    if root["canonical_payload_hex"] != payload_bytes.hex():
        fail("{} canonical payload hex mismatch".format(path.name))
    if not HEX_RE.fullmatch(root["canonical_payload_hex"]):
        fail("{} canonical payload hex is not lowercase".format(path.name))

    preimage = domain.encode("ascii") + b"\x00" + payload_bytes

    if root["preimage_hex"] != preimage.hex():
        fail("{} preimage mismatch".format(path.name))
    if not HEX_RE.fullmatch(root["preimage_hex"]):
        fail("{} preimage hex is not lowercase".format(path.name))

    expected = "b3:" + blake3_256(preimage).hex()

    if root["expected_b3"] != expected:
        fail("{} BLAKE3 mismatch".format(path.name))

    b3(root["expected_b3"], "expected_b3")

    return len(payload_bytes), len(preimage)


def main():
    self_test_blake3()

    actual = {path.name for path in BASE.glob("*.json")}
    expected = set(CASES)

    if actual != expected:
        fail(
            "vector file set mismatch: missing={} extra={}".format(
                sorted(expected - actual),
                sorted(actual - expected),
            )
        )

    payload_total = 0
    preimage_total = 0

    for filename, spec in CASES.items():
        payload_size, preimage_size = verify(BASE / filename, spec)
        payload_total += payload_size
        preimage_total += preimage_size

    print(
        "verified hash_payload_vectors={} payload_bytes={} "
        "preimage_bytes={} hashes_computed={}".format(
            len(CASES),
            payload_total,
            preimage_total,
            len(CASES),
        )
    )


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print("verification failed: {}".format(error), file=sys.stderr)
        sys.exit(1)
