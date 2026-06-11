#!/usr/bin/env python3
"""Verify QuickChain Phase 0 sort-key locked-byte vectors without hashing."""

import json
import re
from pathlib import Path

VECTOR_PATH = (
    Path(__file__).resolve().parents[1]
    / "vectors"
    / "quickchain"
    / "sort_keys"
    / "sort_keys_locked_bytes_v1.json"
)

TOP_LEVEL_FIELDS = {
    "schema",
    "version",
    "status",
    "account_leaf",
    "hold_leaf",
    "unordered_account_inputs",
    "expected_account_order_hex",
    "unordered_hold_inputs",
    "expected_hold_order_hex",
    "duplicate_account_inputs",
    "expected_duplicate_error",
    "notes",
}

ACCOUNT_FIELDS = {
    "account_id",
    "asset",
}

ACCOUNT_VECTOR_FIELDS = ACCOUNT_FIELDS | {
    "expected_sort_key_utf8",
    "expected_sort_key_hex",
}

HOLD_FIELDS = {
    "hold_id",
}

HOLD_VECTOR_FIELDS = HOLD_FIELDS | {
    "expected_sort_key_utf8",
    "expected_sort_key_hex",
}

TOKEN_RE = re.compile(r"^[a-z0-9_.:@/\-]+$")
HOLD_ID_RE = re.compile(r"^hold_[0-9a-f]{32}$")
LOWER_HEX_RE = re.compile(r"^[0-9a-f]+$")


class VectorError(RuntimeError):
    """Raised when the sort-key vector set violates the locked contract."""


def reject_duplicate_keys(pairs):
    obj = {}

    for key, value in pairs:
        if key in obj:
            raise VectorError("duplicate JSON key: %s" % key)

        obj[key] = value

    return obj


def require(condition, message):
    if not condition:
        raise VectorError(message)


def require_exact_fields(value, expected, label):
    require(isinstance(value, dict), "%s must be an object" % label)
    require(set(value.keys()) == expected, "%s fields mismatch" % label)


def lower_hex(value):
    return value.hex()


def account_key(value):
    require_exact_fields(value, ACCOUNT_FIELDS, "account input")

    account_id = value["account_id"]
    asset = value["asset"]

    require(
        isinstance(account_id, str) and account_id,
        "account_id must be nonempty",
    )
    require(
        TOKEN_RE.fullmatch(account_id) is not None,
        "account_id must be a canonical token",
    )
    require(asset == "roc", "asset must be exactly roc")

    return (
        account_id.encode("utf-8")
        + b"\x00"
        + asset.encode("utf-8")
    )


def hold_key(value):
    require_exact_fields(value, HOLD_FIELDS, "hold input")

    hold_id = value["hold_id"]

    require(isinstance(hold_id, str), "hold_id must be a string")
    require(
        HOLD_ID_RE.fullmatch(hold_id) is not None,
        "hold_id shape mismatch",
    )

    return hold_id.encode("utf-8")


def main():
    try:
        value = json.loads(
            VECTOR_PATH.read_text(encoding="utf-8"),
            object_pairs_hook=reject_duplicate_keys,
        )
    except (OSError, UnicodeError, json.JSONDecodeError, VectorError) as error:
        raise VectorError(
            "unable to load strict UTF-8 JSON: %s" % error
        )

    require_exact_fields(
        value,
        TOP_LEVEL_FIELDS,
        "sort-key vector set",
    )

    require(
        value["schema"] == "quickchain.sort-key-vector-set.v1",
        "bad schema",
    )
    require(value["version"] == 1, "bad version")
    require(value["status"] == "locked_bytes", "bad status")

    account_leaf = value["account_leaf"]
    require_exact_fields(
        account_leaf,
        ACCOUNT_VECTOR_FIELDS,
        "account_leaf",
    )

    account_input = {
        "account_id": account_leaf["account_id"],
        "asset": account_leaf["asset"],
    }
    account_bytes = account_key(account_input)

    require(
        account_leaf["expected_sort_key_utf8"].encode("utf-8")
        == account_bytes,
        "account UTF-8 sort key mismatch",
    )
    require(
        account_leaf["expected_sort_key_hex"]
        == lower_hex(account_bytes),
        "account hex sort key mismatch",
    )

    hold_leaf = value["hold_leaf"]
    require_exact_fields(
        hold_leaf,
        HOLD_VECTOR_FIELDS,
        "hold_leaf",
    )

    hold_input = {
        "hold_id": hold_leaf["hold_id"],
    }
    hold_bytes = hold_key(hold_input)

    require(
        hold_leaf["expected_sort_key_utf8"].encode("utf-8")
        == hold_bytes,
        "hold UTF-8 sort key mismatch",
    )
    require(
        hold_leaf["expected_sort_key_hex"]
        == lower_hex(hold_bytes),
        "hold hex sort key mismatch",
    )

    account_inputs = value["unordered_account_inputs"]
    require(
        isinstance(account_inputs, list) and account_inputs,
        "account inputs must be nonempty",
    )

    account_keys = sorted(
        account_key(item)
        for item in account_inputs
    )

    require(
        [lower_hex(item) for item in account_keys]
        == value["expected_account_order_hex"],
        "account bytewise order mismatch",
    )

    hold_inputs = value["unordered_hold_inputs"]
    require(
        isinstance(hold_inputs, list) and hold_inputs,
        "hold inputs must be nonempty",
    )

    hold_keys = sorted(
        hold_key(item)
        for item in hold_inputs
    )

    require(
        [lower_hex(item) for item in hold_keys]
        == value["expected_hold_order_hex"],
        "hold bytewise order mismatch",
    )

    duplicate_inputs = value["duplicate_account_inputs"]
    require(
        isinstance(duplicate_inputs, list),
        "duplicate inputs must be a list",
    )

    duplicate_keys = sorted(
        account_key(item)
        for item in duplicate_inputs
    )

    has_duplicate = any(
        duplicate_keys[index - 1] == duplicate_keys[index]
        for index in range(1, len(duplicate_keys))
    )

    require(
        has_duplicate,
        "duplicate vector must contain a duplicate key",
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
        all(isinstance(note, str) and note for note in notes),
        "notes must be nonempty strings",
    )

    ordered_hex = (
        value["expected_account_order_hex"]
        + value["expected_hold_order_hex"]
    )

    for item in ordered_hex:
        require(
            isinstance(item, str) and len(item) % 2 == 0,
            "ordered hex must have even length",
        )
        require(
            LOWER_HEX_RE.fullmatch(item) is not None,
            "ordered hex must be lowercase",
        )

    print(
        "verified sort_key_vector_sets=1 "
        "account_keys=%d "
        "hold_keys=%d "
        "duplicate_cases=1 "
        "hashes_computed=0"
        % (len(account_keys), len(hold_keys))
    )

    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except VectorError as error:
        raise SystemExit(
            "verification failed: %s" % error
        )
