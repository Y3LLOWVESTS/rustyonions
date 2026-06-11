#!/usr/bin/env python3
"""Independently verify the explicit QuickChain locked-byte vector corpus.

This verifier treats canonical_payload_utf8 and canonical_payload_hex as the
reviewed byte authority. It does not compute hashes, roots, or settlement truth.
"""

from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Any

VECTOR_DIR = Path(__file__).resolve().parents[1] / "vectors" / "quickchain"

EXPECTED_FIELDS = {
    "schema",
    "version",
    "vector_id",
    "status",
    "purpose",
    "domain_separator",
    "canonical_encoding",
    "preimage_framing",
    "hash_algorithm",
    "human_readable_json",
    "canonical_payload_utf8",
    "canonical_payload_hex",
    "preimage_hex",
    "expected_b3",
    "notes",
}

EXPECTED_VECTORS = {
    "account_state_locked_bytes_v1.json": (
        "canonical_account_state_vector_001",
        "account_state_canonical_bytes",
        "quickchain.account-state.v1",
    ),
    "operation_intent_locked_bytes_v1.json": (
        "canonical_operation_intent_vector_001",
        "operation_intent_canonical_bytes",
        "quickchain.operation-intent.v1",
    ),
    "issue_operation_intent_locked_bytes_v1.json": (
        "canonical_issue_operation_intent_vector_001",
        "issue_operation_intent_canonical_bytes",
        "quickchain.operation-intent.v1",
    ),
    "transfer_operation_intent_locked_bytes_v1.json": (
        "canonical_transfer_operation_intent_vector_001",
        "transfer_operation_intent_canonical_bytes",
        "quickchain.operation-intent.v1",
    ),
    "burn_operation_intent_locked_bytes_v1.json": (
        "canonical_burn_operation_intent_vector_001",
        "burn_operation_intent_canonical_bytes",
        "quickchain.operation-intent.v1",
    ),
    "hold_capture_operation_intent_locked_bytes_v1.json": (
        "canonical_hold_capture_operation_intent_vector_001",
        "hold_capture_operation_intent_canonical_bytes",
        "quickchain.operation-intent.v1",
    ),
    "hold_release_operation_intent_locked_bytes_v1.json": (
        "canonical_hold_release_operation_intent_vector_001",
        "hold_release_operation_intent_canonical_bytes",
        "quickchain.operation-intent.v1",
    ),
    "hold_expire_operation_intent_locked_bytes_v1.json": (
        "canonical_hold_expire_operation_intent_vector_001",
        "hold_expire_operation_intent_canonical_bytes",
        "quickchain.operation-intent.v1",
    ),
    "open_hold_state_locked_bytes_v1.json": (
        "canonical_open_hold_state_vector_001",
        "hold_state_canonical_bytes",
        "quickchain.hold-state.v1",
    ),
    "accepted_receipt_locked_bytes_v1.json": (
        "canonical_accepted_receipt_vector_001",
        "receipt_canonical_bytes",
        "quickchain.receipt.v1",
    ),
    "empty_state_tree_locked_bytes_v1.json": (
        "canonical_empty_tree_state_vector_001",
        "empty_tree_canonical_bytes",
        "quickchain.state-root.v1",
    ),
    "empty_holds_tree_locked_bytes_v1.json": (
        "canonical_empty_tree_holds_vector_001",
        "empty_tree_canonical_bytes",
        "quickchain.hold-root.v1",
    ),
    "empty_receipts_tree_locked_bytes_v1.json": (
        "canonical_empty_tree_receipts_vector_001",
        "empty_tree_canonical_bytes",
        "quickchain.receipt-root.v1",
    ),
    "empty_accounting_tree_locked_bytes_v1.json": (
        "canonical_empty_tree_accounting_vector_001",
        "empty_tree_canonical_bytes",
        "quickchain.accounting-root.v1",
    ),
    "empty_rewards_tree_locked_bytes_v1.json": (
        "canonical_empty_tree_rewards_vector_001",
        "empty_tree_canonical_bytes",
        "quickchain.reward-root.v1",
    ),
    "checkpoint_header_locked_bytes_v1.json": (
        "canonical_checkpoint_header_vector_001",
        "checkpoint_header_canonical_bytes",
        "quickchain.checkpoint.v1",
    ),
    "captured_hold_state_locked_bytes_v1.json": (
        "canonical_captured_hold_state_vector_001",
        "captured_hold_state_canonical_bytes",
        "quickchain.hold-state.v1",
    ),
    "released_hold_state_locked_bytes_v1.json": (
        "canonical_released_hold_state_vector_001",
        "released_hold_state_canonical_bytes",
        "quickchain.hold-state.v1",
    ),
    "expired_hold_state_locked_bytes_v1.json": (
        "canonical_expired_hold_state_vector_001",
        "expired_hold_state_canonical_bytes",
        "quickchain.hold-state.v1",
    ),
    "disabled_chain_params_locked_bytes_v1.json": (
        "canonical_disabled_chain_params_vector_001",
        "chain_params_canonical_bytes",
        "quickchain.chain-params.v1",
    ),
    "accepted_issue_receipt_locked_bytes_v1.json": (
        "canonical_accepted_issue_receipt_vector_001",
        "issue_receipt_canonical_bytes",
        "quickchain.receipt.v1",
    ),
    "accepted_burn_receipt_locked_bytes_v1.json": (
        "canonical_accepted_burn_receipt_vector_001",
        "burn_receipt_canonical_bytes",
        "quickchain.receipt.v1",
    ),
    "accepted_hold_open_receipt_locked_bytes_v1.json": (
        "canonical_accepted_hold_open_receipt_vector_001",
        "hold_open_receipt_canonical_bytes",
        "quickchain.receipt.v1",
    ),
    "accepted_hold_capture_receipt_locked_bytes_v1.json": (
        "canonical_accepted_hold_capture_receipt_vector_001",
        "hold_capture_receipt_canonical_bytes",
        "quickchain.receipt.v1",
    ),
    "accepted_hold_release_receipt_locked_bytes_v1.json": (
        "canonical_accepted_hold_release_receipt_vector_001",
        "hold_release_receipt_canonical_bytes",
        "quickchain.receipt.v1",
    ),
    "accepted_hold_expire_receipt_locked_bytes_v1.json": (
        "canonical_accepted_hold_expire_receipt_vector_001",
        "hold_expire_receipt_canonical_bytes",
        "quickchain.receipt.v1",
    ),
}

LOWER_HEX_RE = re.compile(r"^[0-9a-f]+$")


class VectorError(RuntimeError):
    """Raised when a checked-in vector violates the locked-byte contract."""


def reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    obj: dict[str, Any] = {}

    for key, value in pairs:
        if key in obj:
            raise VectorError(f"duplicate JSON key: {key}")

        obj[key] = value

    return obj


def require(condition: bool, message: str) -> None:
    if not condition:
        raise VectorError(message)


def load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(
            path.read_text(encoding="utf-8"),
            object_pairs_hook=reject_duplicate_keys,
        )
    except (OSError, UnicodeError, json.JSONDecodeError, VectorError) as error:
        raise VectorError(
            f"{path.name}: unable to load strict UTF-8 JSON: {error}"
        ) from error

    require(isinstance(value, dict), f"{path.name}: outer value must be an object")
    return value


def verify_vector(
    path: Path,
    expected: tuple[str, str, str],
    seen_ids: set[str],
) -> int:
    value = load_json(path)
    expected_id, expected_purpose, expected_domain = expected

    keys = set(value)

    require(keys == EXPECTED_FIELDS, f"{path.name}: outer fields mismatch")
    require(
        value["schema"] == "quickchain.test-vector.v1",
        f"{path.name}: bad schema",
    )
    require(value["version"] == 1, f"{path.name}: bad version")
    require(value["vector_id"] == expected_id, f"{path.name}: bad vector_id")
    require(value["purpose"] == expected_purpose, f"{path.name}: bad purpose")
    require(
        value["domain_separator"] == expected_domain,
        f"{path.name}: bad domain separator",
    )
    require(
        value["status"] == "locked_bytes",
        f"{path.name}: status must be locked_bytes",
    )
    require(
        value["canonical_encoding"] == "quickchain.canonical-json.v1",
        f"{path.name}: bad canonical encoding",
    )
    require(
        value["preimage_framing"]
        == "domain_separator_bytes || 0x00 || canonical_payload_bytes",
        f"{path.name}: bad preimage framing label",
    )
    require(
        value["hash_algorithm"] == "blake3-256",
        f"{path.name}: bad hash label",
    )
    require(
        value["preimage_hex"] is None,
        f"{path.name}: preimage_hex must remain null",
    )
    require(
        value["expected_b3"] is None,
        f"{path.name}: expected_b3 must remain null",
    )

    vector_id = value["vector_id"]

    require(
        isinstance(vector_id, str) and vector_id,
        f"{path.name}: vector_id must be nonempty",
    )
    require(
        vector_id not in seen_ids,
        f"{path.name}: duplicate vector_id {vector_id}",
    )

    seen_ids.add(vector_id)

    human = value["human_readable_json"]

    require(
        human is not None,
        f"{path.name}: human_readable_json must be non-null",
    )

    payload_utf8 = value["canonical_payload_utf8"]
    payload_hex = value["canonical_payload_hex"]

    require(
        isinstance(payload_utf8, str) and payload_utf8,
        f"{path.name}: canonical_payload_utf8 must be nonempty",
    )
    require(
        isinstance(payload_hex, str) and payload_hex,
        f"{path.name}: canonical_payload_hex must be nonempty",
    )
    require(
        len(payload_hex) % 2 == 0,
        f"{path.name}: payload hex must have even length",
    )
    require(
        LOWER_HEX_RE.fullmatch(payload_hex) is not None,
        f"{path.name}: payload hex must be lowercase",
    )

    payload_bytes = payload_utf8.encode("utf-8")

    require(
        bytes.fromhex(payload_hex) == payload_bytes,
        f"{path.name}: payload hex does not equal UTF-8 payload bytes",
    )

    try:
        parsed_payload = json.loads(
            payload_utf8,
            object_pairs_hook=reject_duplicate_keys,
        )
    except (json.JSONDecodeError, VectorError) as error:
        raise VectorError(
            f"{path.name}: canonical payload is not strict JSON: {error}"
        ) from error

    require(
        parsed_payload == human,
        f"{path.name}: human_readable_json does not equal canonical payload JSON",
    )

    notes = value["notes"]

    require(
        isinstance(notes, list) and notes,
        f"{path.name}: notes must be nonempty",
    )
    require(
        all(isinstance(note, str) and note for note in notes),
        f"{path.name}: every note must be a nonempty string",
    )

    return len(payload_bytes)


def main() -> int:
    actual_files = {path.name for path in VECTOR_DIR.glob("*.json")}
    expected_files = set(EXPECTED_VECTORS)

    missing = sorted(expected_files - actual_files)
    unexpected = sorted(actual_files - expected_files)

    require(not missing, f"missing allowlisted vectors: {missing}")
    require(
        not unexpected,
        f"unexpected vectors outside explicit allowlist: {unexpected}",
    )

    seen_ids: set[str] = set()
    payload_bytes = 0

    for filename, expected in EXPECTED_VECTORS.items():
        payload_bytes += verify_vector(
            VECTOR_DIR / filename,
            expected,
            seen_ids,
        )

    print(
        f"verified vectors={len(EXPECTED_VECTORS)} "
        f"unique_ids={len(seen_ids)} "
        f"payload_bytes={payload_bytes} "
        "hashes_computed=0"
    )

    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except VectorError as error:
        raise SystemExit(f"verification failed: {error}") from error
