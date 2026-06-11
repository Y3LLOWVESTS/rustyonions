#!/usr/bin/env python3
"""Verify QuickChain Phase 0 receipt-order locked bytes without hashing."""

import json
import re
from pathlib import Path

VECTOR_PATH = (
    Path(__file__).resolve().parents[1]
    / "vectors"
    / "quickchain"
    / "receipt_order"
    / "receipt_sort_keys_locked_bytes_v1.json"
)

TOP_LEVEL_FIELDS = {
    "schema",
    "version",
    "status",
    "rule",
    "key_example",
    "unordered_inputs",
    "expected_order_hex",
    "duplicate_inputs",
    "expected_duplicate_error",
    "notes",
}

INPUT_FIELDS = {
    "ledger_seq_start",
    "txid",
}

EXAMPLE_FIELDS = INPUT_FIELDS | {
    "expected_sort_key_hex",
}

TXID_RE = re.compile(r"^[a-z0-9_.:@/-]+$")
LOWER_HEX_RE = re.compile(r"^[0-9a-f]+$")


class VectorError(RuntimeError):
    """Raised when the receipt-order vector violates the locked contract."""


def reject_duplicate_keys(pairs):
    value = {}

    for key, item in pairs:
        if key in value:
            raise VectorError("duplicate JSON key: %s" % key)

        value[key] = item

    return value


def require(condition, message):
    if not condition:
        raise VectorError(message)


def require_exact_fields(value, expected, label):
    require(
        isinstance(value, dict),
        "%s must be an object" % label,
    )
    require(
        set(value.keys()) == expected,
        "%s fields mismatch" % label,
    )


def receipt_key(value):
    require_exact_fields(
        value,
        INPUT_FIELDS,
        "receipt-order input",
    )

    ledger_seq_start = value["ledger_seq_start"]
    txid = value["txid"]

    require(
        isinstance(ledger_seq_start, int)
        and not isinstance(ledger_seq_start, bool),
        "ledger_seq_start must be an integer",
    )
    require(
        0 < ledger_seq_start <= ((1 << 64) - 1),
        "ledger_seq_start must fit non-zero u64",
    )
    require(
        isinstance(txid, str) and txid,
        "txid must be nonempty",
    )
    require(
        TXID_RE.fullmatch(txid) is not None,
        "txid must be a canonical lowercase token",
    )

    return ledger_seq_start.to_bytes(
        8,
        byteorder="big",
        signed=False,
    ) + txid.encode("utf-8")


def validate_hex(value, label):
    require(
        isinstance(value, str) and value,
        "%s must be nonempty" % label,
    )
    require(
        len(value) % 2 == 0,
        "%s must have even length" % label,
    )
    require(
        LOWER_HEX_RE.fullmatch(value) is not None,
        "%s must be lowercase hex" % label,
    )


def main():
    try:
        value = json.loads(
            VECTOR_PATH.read_text(encoding="utf-8"),
            object_pairs_hook=reject_duplicate_keys,
        )
    except (
        OSError,
        UnicodeError,
        json.JSONDecodeError,
        VectorError,
    ) as error:
        raise VectorError(
            "unable to load strict UTF-8 JSON: %s" % error
        )

    require_exact_fields(
        value,
        TOP_LEVEL_FIELDS,
        "receipt-order vector set",
    )

    require(
        value["schema"]
        == "quickchain.receipt-sort-key-vector-set.v1",
        "bad schema",
    )
    require(value["version"] == 1, "bad version")
    require(value["status"] == "locked_bytes", "bad status")
    require(
        value["rule"]
        == "u64_be(ledger_seq_start) || utf8(txid)",
        "bad receipt sort-key rule",
    )

    example = value["key_example"]

    require_exact_fields(
        example,
        EXAMPLE_FIELDS,
        "key_example",
    )

    example_input = {
        "ledger_seq_start": example["ledger_seq_start"],
        "txid": example["txid"],
    }

    example_bytes = receipt_key(example_input)
    example_hex = example["expected_sort_key_hex"]

    validate_hex(example_hex, "expected_sort_key_hex")

    require(
        example_bytes.hex() == example_hex,
        "key example bytes mismatch",
    )
    require(
        example_bytes[:8] == (1).to_bytes(8, "big"),
        "key example sequence prefix mismatch",
    )
    require(
        example_bytes[8:] == b"tx:roc:0001",
        "key example txid suffix mismatch",
    )

    unordered_inputs = value["unordered_inputs"]

    require(
        isinstance(unordered_inputs, list)
        and unordered_inputs,
        "unordered_inputs must be nonempty",
    )

    sorted_keys = sorted(
        receipt_key(item)
        for item in unordered_inputs
    )

    expected_order_hex = value["expected_order_hex"]

    require(
        isinstance(expected_order_hex, list)
        and expected_order_hex,
        "expected_order_hex must be nonempty",
    )

    for item in expected_order_hex:
        validate_hex(item, "expected_order_hex[]")

    require(
        [item.hex() for item in sorted_keys]
        == expected_order_hex,
        "receipt bytewise order mismatch",
    )

    duplicate_inputs = value["duplicate_inputs"]

    require(
        isinstance(duplicate_inputs, list)
        and len(duplicate_inputs) >= 2,
        "duplicate_inputs must contain at least two entries",
    )

    duplicate_keys = sorted(
        receipt_key(item)
        for item in duplicate_inputs
    )

    has_duplicate = any(
        duplicate_keys[index - 1] == duplicate_keys[index]
        for index in range(1, len(duplicate_keys))
    )

    require(
        has_duplicate,
        "duplicate_inputs must contain a duplicate sort key",
    )
    require(
        value["expected_duplicate_error"]
        == "duplicate sort keys are forbidden",
        "bad duplicate error text",
    )

    notes = value["notes"]

    require(
        isinstance(notes, list) and notes,
        "notes must be nonempty",
    )
    require(
        all(
            isinstance(note, str) and note
            for note in notes
        ),
        "notes must contain nonempty strings",
    )

    print(
        "verified receipt_order_vector_sets=1 "
        "ordered_inputs=%d "
        "duplicate_cases=1 "
        "hashes_computed=0"
        % len(sorted_keys)
    )

    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except VectorError as error:
        raise SystemExit(
            "verification failed: %s" % error
        )
