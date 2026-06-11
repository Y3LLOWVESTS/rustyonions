#!/usr/bin/env python3
"""Independently reproduce QuickChain replay/idempotency locked bytes."""

import json
import re
import sys
from pathlib import Path

VECTOR_PATH = (
    Path(__file__).resolve().parents[1]
    / "vectors"
    / "quickchain"
    / "replay"
    / "replay_idempotency_locked_bytes_v1.json"
)

ROOT_KEYS = {
    "schema",
    "version",
    "status",
    "canonical_encoding",
    "cases",
    "notes",
}

CASE_KEYS = {
    "scenario",
    "canonical_payload_utf8",
    "canonical_payload_hex",
    "preimage_hex",
    "expected_b3",
    "notes",
}

SCENARIO_KEYS = {
    "schema",
    "version",
    "chain_id",
    "scenario_id",
    "scenario_kind",
    "original_intent",
    "submitted_intent",
    "original_receipt_txid",
    "attempted_client_account_sequence",
    "expected_outcome",
    "expected_economic_commit_count",
    "expected_state_transition_count",
    "account_sequence_source",
}

INTENT_KEYS = {
    "schema",
    "version",
    "chain_id",
    "operation_id",
    "idempotency_key",
    "op_class",
    "actor_account_id",
    "counterparty_account_id",
    "amount_minor",
    "hold_id",
    "account_sequence",
    "produced_at_ms",
}

EXPECTED = {
    "identical-idempotent-retry-001": (
        "identical_idempotent_retry",
        "return_original_receipt",
        1,
        1,
    ),
    "conflicting-idempotency-reuse-001": (
        "conflicting_idempotency_reuse",
        "reject_idempotency_conflict",
        1,
        1,
    ),
    "duplicate-operation-commit-001": (
        "duplicate_operation_commit",
        "reject_duplicate_operation_commit",
        1,
        1,
    ),
    "retry-hold-capture-001": (
        "retry_hold_capture",
        "return_original_receipt",
        1,
        1,
    ),
    "retry-hold-release-001": (
        "retry_hold_release",
        "return_original_receipt",
        1,
        1,
    ),
    "ledger-assigned-account-sequence-001": (
        "ledger_assigned_account_sequence",
        "reject_client_assigned_account_sequence",
        0,
        0,
    ),
}

OPERATION_ID = re.compile(r"^op_[0-9a-f]{32}$")
HOLD_ID = re.compile(r"^hold_[0-9a-f]{32}$")
MONEY = re.compile(r"^(0|[1-9][0-9]*)$")
TOKEN = re.compile(r"^[a-z0-9_.:@/-]+$")


def fail(message):
    raise ValueError(message)


def require_exact_keys(value, expected, label):
    if not isinstance(value, dict):
        fail("{} must be an object".format(label))

    actual = set(value.keys())

    if actual != expected:
        fail(
            "{} keys mismatch: missing={} extra={}".format(
                label,
                sorted(expected - actual),
                sorted(actual - expected),
            )
        )


def canonical_intent(intent_value, label):
    require_exact_keys(intent_value, INTENT_KEYS, label)

    if intent_value["schema"] != "quickchain.operation-intent.v1":
        fail("{} schema mismatch".format(label))

    if intent_value["version"] != 1:
        fail("{} version mismatch".format(label))

    if intent_value["chain_id"] != "roc-dev":
        fail("{} chain_id mismatch".format(label))

    if not OPERATION_ID.fullmatch(intent_value["operation_id"]):
        fail("{} operation_id is not canonical".format(label))

    if not intent_value["idempotency_key"]:
        fail("{} idempotency_key must not be empty".format(label))

    if intent_value["account_sequence"] is not None:
        fail("{} account_sequence must remain null".format(label))

    if not MONEY.fullmatch(intent_value["amount_minor"]):
        fail("{} amount_minor is not canonical integer text".format(label))

    if not TOKEN.fullmatch(intent_value["actor_account_id"]):
        fail("{} actor_account_id is not canonical".format(label))

    counterparty = intent_value["counterparty_account_id"]

    if counterparty is not None and not TOKEN.fullmatch(counterparty):
        fail("{} counterparty_account_id is not canonical".format(label))

    hold_id = intent_value["hold_id"]

    if hold_id is not None and not HOLD_ID.fullmatch(hold_id):
        fail("{} hold_id is not canonical".format(label))

    if not isinstance(intent_value["produced_at_ms"], int) or isinstance(
        intent_value["produced_at_ms"], bool
    ):
        fail("{} produced_at_ms must be an integer".format(label))

    if intent_value["produced_at_ms"] <= 0:
        fail("{} produced_at_ms must be positive".format(label))

    return {
        "schema": intent_value["schema"],
        "version": intent_value["version"],
        "chain_id": intent_value["chain_id"],
        "operation_id": intent_value["operation_id"],
        "idempotency_key": intent_value["idempotency_key"],
        "op_class": intent_value["op_class"],
        "actor_account_id": intent_value["actor_account_id"],
        "counterparty_account_id": counterparty,
        "amount_minor": intent_value["amount_minor"],
        "hold_id": hold_id,
        "account_sequence": None,
        "produced_at_ms": intent_value["produced_at_ms"],
    }


def canonical_scenario(scenario_value):
    require_exact_keys(scenario_value, SCENARIO_KEYS, "scenario")

    if scenario_value["schema"] != "quickchain.replay-scenario.v1":
        fail("scenario schema mismatch")

    if scenario_value["version"] != 1:
        fail("scenario version mismatch")

    if scenario_value["chain_id"] != "roc-dev":
        fail("scenario chain_id mismatch")

    if scenario_value["account_sequence_source"] != "ledger_assigned":
        fail("account_sequence_source must be ledger_assigned")

    original_raw = scenario_value["original_intent"]
    original = None

    if original_raw is not None:
        original = canonical_intent(original_raw, "original_intent")

    submitted = canonical_intent(
        scenario_value["submitted_intent"],
        "submitted_intent",
    )

    if submitted["chain_id"] != scenario_value["chain_id"]:
        fail("submitted intent chain mismatch")

    if original is not None and original["chain_id"] != scenario_value["chain_id"]:
        fail("original intent chain mismatch")

    receipt = scenario_value["original_receipt_txid"]

    if receipt is not None and not TOKEN.fullmatch(receipt):
        fail("original_receipt_txid is not canonical")

    attempted = scenario_value["attempted_client_account_sequence"]

    if attempted is not None:
        if (
            not isinstance(attempted, int)
            or isinstance(attempted, bool)
            or attempted <= 0
        ):
            fail(
                "attempted_client_account_sequence must be a positive integer"
            )

    for field in (
        "expected_economic_commit_count",
        "expected_state_transition_count",
    ):
        value = scenario_value[field]

        if not isinstance(value, int) or isinstance(value, bool) or value < 0:
            fail("{} must be a non-negative integer".format(field))

    return {
        "schema": scenario_value["schema"],
        "version": scenario_value["version"],
        "chain_id": scenario_value["chain_id"],
        "scenario_id": scenario_value["scenario_id"],
        "scenario_kind": scenario_value["scenario_kind"],
        "original_intent": original,
        "submitted_intent": submitted,
        "original_receipt_txid": receipt,
        "attempted_client_account_sequence": attempted,
        "expected_outcome": scenario_value["expected_outcome"],
        "expected_economic_commit_count": scenario_value[
            "expected_economic_commit_count"
        ],
        "expected_state_transition_count": scenario_value[
            "expected_state_transition_count"
        ],
        "account_sequence_source": "ledger_assigned",
    }


def validate_semantics(scenario):
    scenario_id = scenario["scenario_id"]

    if scenario_id not in EXPECTED:
        fail("unexpected scenario_id {}".format(scenario_id))

    expected_kind, expected_outcome, expected_commits, expected_transitions = (
        EXPECTED[scenario_id]
    )

    if scenario["scenario_kind"] != expected_kind:
        fail("{} kind mismatch".format(scenario_id))

    if scenario["expected_outcome"] != expected_outcome:
        fail("{} outcome mismatch".format(scenario_id))

    if scenario["expected_economic_commit_count"] != expected_commits:
        fail("{} economic commit count mismatch".format(scenario_id))

    if scenario["expected_state_transition_count"] != expected_transitions:
        fail("{} state transition count mismatch".format(scenario_id))

    original = scenario["original_intent"]
    submitted = scenario["submitted_intent"]
    attempted = scenario["attempted_client_account_sequence"]

    if expected_kind == "ledger_assigned_account_sequence":
        if (
            original is not None
            or scenario["original_receipt_txid"] is not None
        ):
            fail("ledger sequence rejection must have no accepted original")

        if attempted is None:
            fail("ledger sequence rejection requires a client attempt")

        return

    if original is None or scenario["original_receipt_txid"] is None:
        fail("{} requires original intent and receipt".format(scenario_id))

    if attempted is not None:
        fail("{} must not include client sequence attempt".format(scenario_id))

    if expected_kind in (
        "identical_idempotent_retry",
        "retry_hold_capture",
        "retry_hold_release",
    ):
        if original != submitted:
            fail("{} must be an identical retry".format(scenario_id))

    if (
        expected_kind == "retry_hold_capture"
        and submitted["op_class"] != "hold_capture"
    ):
        fail("retry hold capture must use hold_capture")

    if (
        expected_kind == "retry_hold_release"
        and submitted["op_class"] != "hold_release"
    ):
        fail("retry hold release must use hold_release")

    if expected_kind == "conflicting_idempotency_reuse":
        if original["idempotency_key"] != submitted["idempotency_key"]:
            fail("conflicting reuse must keep the same idempotency key")

        if original == submitted:
            fail("conflicting reuse must change the intent")

    if expected_kind == "duplicate_operation_commit":
        if original["operation_id"] != submitted["operation_id"]:
            fail("duplicate commit must keep the same operation_id")

        if original["idempotency_key"] == submitted["idempotency_key"]:
            fail("duplicate commit vector must isolate operation identity")


def main():
    root = json.loads(VECTOR_PATH.read_text(encoding="utf-8"))

    require_exact_keys(root, ROOT_KEYS, "vector set")

    if root["schema"] != "quickchain.replay-scenario-vector-set.v1":
        fail("vector set schema mismatch")

    if root["version"] != 1:
        fail("vector set version mismatch")

    if root["status"] != "locked_bytes":
        fail("vector set status must be locked_bytes")

    if root["canonical_encoding"] != "quickchain.canonical-json.v1":
        fail("canonical encoding mismatch")

    if not isinstance(root["cases"], list) or len(root["cases"]) != 6:
        fail("vector set must contain exactly six cases")

    seen = set()
    total_bytes = 0

    for index, case in enumerate(root["cases"]):
        require_exact_keys(case, CASE_KEYS, "case[{}]".format(index))

        if case["preimage_hex"] is not None or case["expected_b3"] is not None:
            fail("locked_bytes cases must not contain hash claims")

        if not case["notes"] or not all(
            isinstance(note, str) and note for note in case["notes"]
        ):
            fail("case notes must be non-empty strings")

        scenario = canonical_scenario(case["scenario"])

        validate_semantics(scenario)

        scenario_id = scenario["scenario_id"]

        if scenario_id in seen:
            fail("duplicate scenario_id {}".format(scenario_id))

        seen.add(scenario_id)

        canonical = json.dumps(
            scenario,
            ensure_ascii=False,
            separators=(",", ":"),
        )
        canonical_bytes = canonical.encode("utf-8")

        if canonical != case["canonical_payload_utf8"]:
            fail("{} canonical UTF-8 mismatch".format(scenario_id))

        if canonical_bytes.hex() != case["canonical_payload_hex"]:
            fail("{} canonical hex mismatch".format(scenario_id))

        total_bytes += len(canonical_bytes)

    if seen != set(EXPECTED.keys()):
        fail("scenario coverage mismatch")

    print(
        "verified {} QuickChain replay/idempotency locked-byte scenarios "
        "({} bytes)".format(len(seen), total_bytes)
    )


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(
            "verification failed: {}".format(error),
            file=sys.stderr,
        )
        sys.exit(1)
