#!/usr/bin/env python3
"""Independently verify QuickChain concurrent-hold and compaction locked bytes."""

import json
import re
import sys
from pathlib import Path

BASE = (
    Path(__file__).resolve().parents[1]
    / "vectors"
    / "quickchain"
    / "hold_scenarios"
)

REPLAY_PATH = BASE / "concurrent_holds_replay_locked_bytes_v1.json"
COMPACTION_PATH = BASE / "closed_hold_compaction_locked_bytes_v1.json"

OUTER_KEYS = {
    "schema",
    "version",
    "status",
    "canonical_encoding",
    "scenario",
    "canonical_payload_utf8",
    "canonical_payload_hex",
    "preimage_hex",
    "expected_b3",
    "notes",
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

SNAPSHOT_KEYS = {
    "account_id",
    "total_minor",
    "available_minor",
    "held_minor",
    "account_sequence",
}

STEP_KEYS = {
    "ordinal",
    "intent",
    "expected_outcome",
    "receipt_txid",
    "receipt_account_sequence",
}

REPLAY_KEYS = {
    "schema",
    "version",
    "chain_id",
    "scenario_id",
    "initial_account",
    "steps",
    "expected_final_account",
    "expected_active_hold_ids",
    "expected_terminal_receipt_txids",
    "expected_economic_commit_count",
    "expected_state_transition_count",
    "invariants",
}

HOLD_KEYS = {
    "schema",
    "version",
    "chain_id",
    "hold_id",
    "account_id",
    "counterparty_account_id",
    "amount_minor",
    "status",
    "opened_operation_id",
    "terminal_operation_id",
    "opened_at_ms",
    "expires_at_ms",
    "terminal_at_ms",
    "account_sequence_opened",
    "account_sequence_terminal",
}

TERMINAL_RECEIPT_KEYS = {
    "hold_id",
    "terminal_status",
    "receipt_txid",
}

COMPACTION_KEYS = {
    "schema",
    "version",
    "chain_id",
    "scenario_id",
    "unordered_holds",
    "terminal_receipts",
    "expected_active_hold_ids",
    "expected_rejected_resurrection_hold_ids",
    "expected_compacted_terminal_count",
    "invariants",
}

REPLAY_INVARIANTS = {
    "retry_capture_no_double_spend",
    "retry_release_no_state_change",
    "account_sequence_advances_on_commit_only",
    "closed_holds_absent_from_active_set",
    "terminal_lifecycle_retained_by_receipt",
}

COMPACTION_INVARIANTS = {
    "only_open_holds_remain_active",
    "terminal_holds_retain_receipt_evidence",
    "terminal_holds_cannot_resurrect",
    "input_order_does_not_affect_active_set",
    "duplicate_hold_ids_reject",
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


def require_positive_integer(value, label):
    if not isinstance(value, int) or isinstance(value, bool) or value <= 0:
        fail("{} must be a positive integer".format(label))


def require_nonnegative_integer(value, label):
    if not isinstance(value, int) or isinstance(value, bool) or value < 0:
        fail("{} must be a non-negative integer".format(label))


def require_token(value, label):
    if not isinstance(value, str) or not TOKEN.fullmatch(value):
        fail("{} is not a canonical token".format(label))


def require_money(value, label):
    if not isinstance(value, str) or not MONEY.fullmatch(value):
        fail("{} is not canonical integer minor-unit text".format(label))

    parsed = int(value)

    if parsed >= 2**128:
        fail("{} exceeds u128".format(label))

    return parsed


def canonical_intent(value, label):
    require_exact_keys(value, INTENT_KEYS, label)

    if value["schema"] != "quickchain.operation-intent.v1":
        fail("{} schema mismatch".format(label))

    if value["version"] != 1 or value["chain_id"] != "roc-dev":
        fail("{} version or chain mismatch".format(label))

    if not OPERATION_ID.fullmatch(value["operation_id"]):
        fail("{} operation_id mismatch".format(label))

    if not isinstance(value["idempotency_key"], str) or not value["idempotency_key"]:
        fail("{} idempotency_key must not be empty".format(label))

    if value["op_class"] not in {
        "hold_open",
        "hold_capture",
        "hold_release",
        "hold_expire",
    }:
        fail("{} must contain a hold lifecycle operation".format(label))

    require_token(value["actor_account_id"], "{} actor".format(label))

    counterparty = value["counterparty_account_id"]

    if counterparty is not None:
        require_token(counterparty, "{} counterparty".format(label))

    require_money(value["amount_minor"], "{} amount".format(label))

    if not HOLD_ID.fullmatch(value["hold_id"]):
        fail("{} hold_id mismatch".format(label))

    if value["account_sequence"] is not None:
        fail("{} account_sequence must remain null".format(label))

    require_positive_integer(value["produced_at_ms"], "{} produced_at_ms".format(label))

    if value["op_class"] == "hold_capture" and counterparty is None:
        fail("{} hold_capture requires counterparty".format(label))

    if value["op_class"] in {"hold_release", "hold_expire"} and counterparty is not None:
        fail("{} terminal return operation forbids counterparty".format(label))

    return {
        "schema": value["schema"],
        "version": value["version"],
        "chain_id": value["chain_id"],
        "operation_id": value["operation_id"],
        "idempotency_key": value["idempotency_key"],
        "op_class": value["op_class"],
        "actor_account_id": value["actor_account_id"],
        "counterparty_account_id": counterparty,
        "amount_minor": value["amount_minor"],
        "hold_id": value["hold_id"],
        "account_sequence": None,
        "produced_at_ms": value["produced_at_ms"],
    }


def canonical_snapshot(value, label):
    require_exact_keys(value, SNAPSHOT_KEYS, label)
    require_token(value["account_id"], "{} account_id".format(label))

    total = require_money(value["total_minor"], "{} total".format(label))
    available = require_money(
        value["available_minor"],
        "{} available".format(label),
    )
    held = require_money(value["held_minor"], "{} held".format(label))

    if total != available + held:
        fail("{} total must equal available plus held".format(label))

    require_nonnegative_integer(
        value["account_sequence"],
        "{} account_sequence".format(label),
    )

    return {
        "account_id": value["account_id"],
        "total_minor": value["total_minor"],
        "available_minor": value["available_minor"],
        "held_minor": value["held_minor"],
        "account_sequence": value["account_sequence"],
    }


def canonical_step(value, label):
    require_exact_keys(value, STEP_KEYS, label)
    require_positive_integer(value["ordinal"], "{} ordinal".format(label))

    if value["expected_outcome"] not in {
        "committed",
        "return_original_receipt",
    }:
        fail("{} outcome mismatch".format(label))

    require_token(value["receipt_txid"], "{} receipt_txid".format(label))
    require_positive_integer(
        value["receipt_account_sequence"],
        "{} receipt sequence".format(label),
    )

    return {
        "ordinal": value["ordinal"],
        "intent": canonical_intent(value["intent"], "{} intent".format(label)),
        "expected_outcome": value["expected_outcome"],
        "receipt_txid": value["receipt_txid"],
        "receipt_account_sequence": value["receipt_account_sequence"],
    }


def canonical_replay(value):
    require_exact_keys(value, REPLAY_KEYS, "replay scenario")

    if value["schema"] != "quickchain.concurrent-hold-replay.v1":
        fail("replay scenario schema mismatch")

    if value["version"] != 1 or value["chain_id"] != "roc-dev":
        fail("replay scenario version or chain mismatch")

    require_token(value["scenario_id"], "replay scenario_id")

    initial = canonical_snapshot(value["initial_account"], "initial_account")
    final = canonical_snapshot(
        value["expected_final_account"],
        "expected_final_account",
    )

    if initial["account_id"] != final["account_id"]:
        fail("initial and final account IDs must match")

    if not isinstance(value["steps"], list) or not value["steps"]:
        fail("replay steps must be non-empty")

    steps = [
        canonical_step(step, "steps[{}]".format(index))
        for index, step in enumerate(value["steps"])
    ]

    active_ids = value["expected_active_hold_ids"]

    if (
        not isinstance(active_ids, list)
        or active_ids != sorted(active_ids)
        or len(active_ids) != len(set(active_ids))
        or not all(HOLD_ID.fullmatch(item) for item in active_ids)
    ):
        fail("expected_active_hold_ids must be sorted and unique")

    terminal_txids = value["expected_terminal_receipt_txids"]

    if (
        not isinstance(terminal_txids, list)
        or len(terminal_txids) != len(set(terminal_txids))
    ):
        fail("terminal receipt txids must be unique")

    for txid in terminal_txids:
        require_token(txid, "terminal receipt txid")

    invariants = value["invariants"]

    if set(invariants) != REPLAY_INVARIANTS or len(invariants) != len(
        REPLAY_INVARIANTS
    ):
        fail("replay invariant set mismatch")

    require_nonnegative_integer(
        value["expected_economic_commit_count"],
        "expected_economic_commit_count",
    )
    require_nonnegative_integer(
        value["expected_state_transition_count"],
        "expected_state_transition_count",
    )

    return {
        "schema": value["schema"],
        "version": value["version"],
        "chain_id": value["chain_id"],
        "scenario_id": value["scenario_id"],
        "initial_account": initial,
        "steps": steps,
        "expected_final_account": final,
        "expected_active_hold_ids": active_ids,
        "expected_terminal_receipt_txids": terminal_txids,
        "expected_economic_commit_count": value[
            "expected_economic_commit_count"
        ],
        "expected_state_transition_count": value[
            "expected_state_transition_count"
        ],
        "invariants": invariants,
    }


def validate_replay_semantics(scenario):
    if scenario["scenario_id"] != "concurrent-holds-replay-001":
        fail("unexpected replay scenario_id")

    if scenario["initial_account"] != {
        "account_id": "account:payer-holds",
        "total_minor": "1000",
        "available_minor": "1000",
        "held_minor": "0",
        "account_sequence": 0,
    }:
        fail("initial replay snapshot mismatch")

    if scenario["expected_final_account"] != {
        "account_id": "account:payer-holds",
        "total_minor": "750",
        "available_minor": "750",
        "held_minor": "0",
        "account_sequence": 4,
    }:
        fail("final replay snapshot mismatch")

    if scenario["expected_active_hold_ids"] != []:
        fail("final active hold set must be empty")

    accepted = {}
    committed_receipts = set()
    terminal_receipts = []
    last_sequence = scenario["initial_account"]["account_sequence"]
    committed_count = 0

    for index, step in enumerate(scenario["steps"], start=1):
        if step["ordinal"] != index:
            fail("replay ordinals must be contiguous")

        intent = step["intent"]

        if intent["actor_account_id"] != scenario["initial_account"]["account_id"]:
            fail("replay actor account mismatch")

        operation_id = intent["operation_id"]

        if step["expected_outcome"] == "committed":
            if operation_id in accepted:
                fail("operation_id committed more than once")

            if step["receipt_account_sequence"] != last_sequence + 1:
                fail("new commit did not advance sequence by one")

            if step["receipt_txid"] in committed_receipts:
                fail("new commit receipt txid is not unique")

            accepted[operation_id] = (
                intent,
                step["receipt_txid"],
                step["receipt_account_sequence"],
            )
            committed_receipts.add(step["receipt_txid"])
            last_sequence = step["receipt_account_sequence"]
            committed_count += 1

            if intent["op_class"] in {
                "hold_capture",
                "hold_release",
                "hold_expire",
            }:
                terminal_receipts.append(step["receipt_txid"])
        else:
            if operation_id not in accepted:
                fail("retry does not reference an earlier commit")

            original_intent, original_txid, original_sequence = accepted[
                operation_id
            ]

            if intent != original_intent:
                fail("retry intent differs from original")

            if step["receipt_txid"] != original_txid:
                fail("retry did not return original receipt")

            if step["receipt_account_sequence"] != original_sequence:
                fail("retry did not retain original sequence")

    if committed_count != 4:
        fail("replay must contain exactly four commits")

    if scenario["expected_economic_commit_count"] != committed_count:
        fail("economic commit count mismatch")

    if scenario["expected_state_transition_count"] != committed_count:
        fail("state transition count mismatch")

    if scenario["expected_final_account"]["account_sequence"] != last_sequence:
        fail("final sequence mismatch")

    if scenario["expected_terminal_receipt_txids"] != terminal_receipts:
        fail("terminal receipt order mismatch")


def canonical_hold(value, label):
    require_exact_keys(value, HOLD_KEYS, label)

    if value["schema"] != "quickchain.hold-state.v1":
        fail("{} schema mismatch".format(label))

    if value["version"] != 1 or value["chain_id"] != "roc-dev":
        fail("{} version or chain mismatch".format(label))

    if not HOLD_ID.fullmatch(value["hold_id"]):
        fail("{} hold_id mismatch".format(label))

    require_token(value["account_id"], "{} account_id".format(label))

    counterparty = value["counterparty_account_id"]

    if counterparty is not None:
        require_token(counterparty, "{} counterparty".format(label))

    require_money(value["amount_minor"], "{} amount".format(label))

    if value["status"] not in {"open", "captured", "released", "expired"}:
        fail("{} status mismatch".format(label))

    if not OPERATION_ID.fullmatch(value["opened_operation_id"]):
        fail("{} opened operation mismatch".format(label))

    terminal_operation = value["terminal_operation_id"]

    if terminal_operation is not None and not OPERATION_ID.fullmatch(
        terminal_operation
    ):
        fail("{} terminal operation mismatch".format(label))

    require_positive_integer(value["opened_at_ms"], "{} opened_at".format(label))
    require_positive_integer(value["expires_at_ms"], "{} expires_at".format(label))
    require_positive_integer(
        value["account_sequence_opened"],
        "{} opening sequence".format(label),
    )

    if value["expires_at_ms"] < value["opened_at_ms"]:
        fail("{} expiry precedes opening".format(label))

    if value["status"] == "open":
        if (
            terminal_operation is not None
            or value["terminal_at_ms"] is not None
            or value["account_sequence_terminal"] is not None
        ):
            fail("{} open hold contains terminal fields".format(label))
    else:
        if (
            terminal_operation is None
            or value["terminal_at_ms"] is None
            or value["account_sequence_terminal"] is None
        ):
            fail("{} terminal hold lacks terminal fields".format(label))

        require_positive_integer(
            value["terminal_at_ms"],
            "{} terminal_at".format(label),
        )
        require_positive_integer(
            value["account_sequence_terminal"],
            "{} terminal sequence".format(label),
        )

    return {
        "schema": value["schema"],
        "version": value["version"],
        "chain_id": value["chain_id"],
        "hold_id": value["hold_id"],
        "account_id": value["account_id"],
        "counterparty_account_id": counterparty,
        "amount_minor": value["amount_minor"],
        "status": value["status"],
        "opened_operation_id": value["opened_operation_id"],
        "terminal_operation_id": terminal_operation,
        "opened_at_ms": value["opened_at_ms"],
        "expires_at_ms": value["expires_at_ms"],
        "terminal_at_ms": value["terminal_at_ms"],
        "account_sequence_opened": value["account_sequence_opened"],
        "account_sequence_terminal": value["account_sequence_terminal"],
    }


def canonical_terminal_receipt(value, label):
    require_exact_keys(value, TERMINAL_RECEIPT_KEYS, label)

    if not HOLD_ID.fullmatch(value["hold_id"]):
        fail("{} hold_id mismatch".format(label))

    if value["terminal_status"] not in {"captured", "released", "expired"}:
        fail("{} terminal status mismatch".format(label))

    require_token(value["receipt_txid"], "{} txid".format(label))

    return {
        "hold_id": value["hold_id"],
        "terminal_status": value["terminal_status"],
        "receipt_txid": value["receipt_txid"],
    }


def canonical_compaction(value):
    require_exact_keys(value, COMPACTION_KEYS, "compaction scenario")

    if value["schema"] != "quickchain.hold-compaction.v1":
        fail("compaction schema mismatch")

    if value["version"] != 1 or value["chain_id"] != "roc-dev":
        fail("compaction version or chain mismatch")

    require_token(value["scenario_id"], "compaction scenario_id")

    if not isinstance(value["unordered_holds"], list) or not value[
        "unordered_holds"
    ]:
        fail("unordered_holds must be non-empty")

    holds = [
        canonical_hold(hold, "unordered_holds[{}]".format(index))
        for index, hold in enumerate(value["unordered_holds"])
    ]

    receipts = [
        canonical_terminal_receipt(
            receipt,
            "terminal_receipts[{}]".format(index),
        )
        for index, receipt in enumerate(value["terminal_receipts"])
    ]

    for field in (
        "expected_active_hold_ids",
        "expected_rejected_resurrection_hold_ids",
    ):
        values = value[field]

        if (
            not isinstance(values, list)
            or values != sorted(values)
            or len(values) != len(set(values))
            or not all(HOLD_ID.fullmatch(item) for item in values)
        ):
            fail("{} must be sorted unique hold IDs".format(field))

    require_nonnegative_integer(
        value["expected_compacted_terminal_count"],
        "expected_compacted_terminal_count",
    )

    invariants = value["invariants"]

    if set(invariants) != COMPACTION_INVARIANTS or len(invariants) != len(
        COMPACTION_INVARIANTS
    ):
        fail("compaction invariant set mismatch")

    return {
        "schema": value["schema"],
        "version": value["version"],
        "chain_id": value["chain_id"],
        "scenario_id": value["scenario_id"],
        "unordered_holds": holds,
        "terminal_receipts": receipts,
        "expected_active_hold_ids": value["expected_active_hold_ids"],
        "expected_rejected_resurrection_hold_ids": value[
            "expected_rejected_resurrection_hold_ids"
        ],
        "expected_compacted_terminal_count": value[
            "expected_compacted_terminal_count"
        ],
        "invariants": invariants,
    }


def validate_compaction_semantics(scenario):
    if scenario["scenario_id"] != "closed-hold-compaction-001":
        fail("unexpected compaction scenario_id")

    hold_ids = [hold["hold_id"] for hold in scenario["unordered_holds"]]

    if len(hold_ids) != len(set(hold_ids)):
        fail("duplicate hold ID")

    if hold_ids == sorted(hold_ids):
        fail("compaction fixture must begin in non-canonical input order")

    statuses = {
        hold["hold_id"]: hold["status"]
        for hold in scenario["unordered_holds"]
    }

    active = sorted(
        hold_id
        for hold_id, status in statuses.items()
        if status == "open"
    )
    terminal = {
        hold_id
        for hold_id, status in statuses.items()
        if status != "open"
    }

    if scenario["expected_active_hold_ids"] != active:
        fail("active hold output mismatch")

    receipt_holds = set()
    receipt_txids = set()

    for receipt in scenario["terminal_receipts"]:
        hold_id = receipt["hold_id"]

        if hold_id not in statuses:
            fail("terminal receipt references unknown hold")

        if statuses[hold_id] != receipt["terminal_status"]:
            fail("terminal receipt status mismatch")

        if hold_id in receipt_holds:
            fail("duplicate terminal receipt hold")

        if receipt["receipt_txid"] in receipt_txids:
            fail("duplicate terminal receipt txid")

        receipt_holds.add(hold_id)
        receipt_txids.add(receipt["receipt_txid"])

    if receipt_holds != terminal:
        fail("terminal receipt coverage mismatch")

    if set(scenario["expected_rejected_resurrection_hold_ids"]) != terminal:
        fail("resurrection rejection set mismatch")

    if scenario["expected_compacted_terminal_count"] != len(terminal):
        fail("compacted terminal count mismatch")


def verify_outer(path, expected_schema, canonicalizer, semantic_validator):
    root = json.loads(path.read_text(encoding="utf-8"))

    require_exact_keys(root, OUTER_KEYS, path.name)

    if root["schema"] != expected_schema:
        fail("{} outer schema mismatch".format(path.name))

    if root["version"] != 1:
        fail("{} version mismatch".format(path.name))

    if root["status"] != "locked_bytes":
        fail("{} status must be locked_bytes".format(path.name))

    if root["canonical_encoding"] != "quickchain.canonical-json.v1":
        fail("{} canonical encoding mismatch".format(path.name))

    if root["preimage_hex"] is not None or root["expected_b3"] is not None:
        fail("{} must not contain hash claims".format(path.name))

    if not root["notes"] or not all(
        isinstance(note, str) and note for note in root["notes"]
    ):
        fail("{} notes must be non-empty strings".format(path.name))

    scenario = canonicalizer(root["scenario"])
    semantic_validator(scenario)

    canonical = json.dumps(
        scenario,
        ensure_ascii=False,
        separators=(",", ":"),
    )
    canonical_bytes = canonical.encode("utf-8")

    if canonical != root["canonical_payload_utf8"]:
        fail("{} canonical UTF-8 mismatch".format(path.name))

    if canonical_bytes.hex() != root["canonical_payload_hex"]:
        fail("{} canonical hex mismatch".format(path.name))

    return len(canonical_bytes)


def main():
    replay_bytes = verify_outer(
        REPLAY_PATH,
        "quickchain.concurrent-hold-replay-vector.v1",
        canonical_replay,
        validate_replay_semantics,
    )

    compaction_bytes = verify_outer(
        COMPACTION_PATH,
        "quickchain.hold-compaction-vector.v1",
        canonical_compaction,
        validate_compaction_semantics,
    )

    print(
        "verified hold_scenario_vectors=2 "
        "replay_steps=6 compaction_holds=4 "
        "payload_bytes={} hashes_computed=0".format(
            replay_bytes + compaction_bytes
        )
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
