//! RO:WHAT — JSON-schema boundary tests for ron-policy QuickChain preflight.
//! RO:WHY — Keeps published policy schema aligned with parser authority rejection.
//! RO:INTERACTS — `schema/policybundle.schema.json`, policy DTO strictness, obligation validation.
//! RO:INVARIANTS — schema must remain strict and must not authorize receipt/balance/finality/root fields.

use serde_json::Value;

const POLICYBUNDLE_SCHEMA: &str = include_str!("../schema/policybundle.schema.json");

#[test]
fn policybundle_schema_is_valid_json() {
    serde_json::from_str::<Value>(POLICYBUNDLE_SCHEMA).expect("policybundle schema must be JSON");
}

#[test]
fn policybundle_schema_preserves_strict_dto_shape() {
    let schema =
        serde_json::from_str::<Value>(POLICYBUNDLE_SCHEMA).expect("policybundle schema JSON");

    assert!(
        count_exact_key_value(&schema, "additionalProperties", &Value::Bool(false)) >= 4,
        "schema must keep deny-unknown-fields / additionalProperties=false discipline"
    );

    assert!(
        POLICYBUNDLE_SCHEMA.contains("\"required\": [\"version\", \"rules\"]"),
        "schema must require version and rules"
    );

    assert!(
        POLICYBUNDLE_SCHEMA.contains("\"required\": [\"id\", \"when\", \"action\"]"),
        "schema must require rule id, when, and action"
    );

    assert!(
        POLICYBUNDLE_SCHEMA.contains("\"required\": [\"kind\"]"),
        "schema must require obligation kind"
    );
}

#[test]
fn policybundle_schema_preserves_body_cap_limit() {
    assert!(
        POLICYBUNDLE_SCHEMA.contains("\"maximum\": 1048576"),
        "schema must preserve 1 MiB body cap maximum"
    );
}

#[test]
fn policybundle_schema_rejects_authority_shaped_obligations() {
    for phrase in [
        "[Cc]reate[_-]?[Rr]eceipt",
        "[Ff]inalize[_-]?[Rr]eceipt",
        "[Mm]utate[_-]?[Bb]alance",
        "[Uu]nlock[_-]?[Pp]aid[_-]?[Cc]ontent",
        "[Pp]rove[_-]?[Pp]ayment[_-]?[Ff]inality",
        "[Pp]roduce[_-]?[Cc]heckpoint",
        "[Pp]roduce[_-]?[Rr]oot",
        "[Bb]ridge[_-]?[Ss]ettlement",
    ] {
        assert!(
            POLICYBUNDLE_SCHEMA.contains(phrase),
            "schema missing authority-kind rejection pattern: {phrase}"
        );
    }
}

#[test]
fn policybundle_schema_rejects_authority_shaped_obligation_params() {
    assert!(
        POLICYBUNDLE_SCHEMA.contains("\"propertyNames\""),
        "schema must constrain obligation param names"
    );

    for phrase in [
        "[Rr]eceipt[_-]?[Ii][Dd]",
        "[Rr]eceipt[_-]?[Rr]oot",
        "[Ww]allet[_-]?[Bb]alance",
        "[Ll]edger[_-]?[Bb]alance",
        "[Uu]nlock[_-]?[Gg]ranted",
        "[Ss]tate[_-]?[Rr]oot",
        "[Cc]heckpoint[_-]?[Hh]ash",
        "[Vv]alidator[_-]?[Ss]ignature",
        "[Mm]int[_-]?[Aa]uthority",
    ] {
        assert!(
            POLICYBUNDLE_SCHEMA.contains(phrase),
            "schema missing authority-param rejection pattern: {phrase}"
        );
    }
}

fn count_exact_key_value(value: &Value, key: &str, expected: &Value) -> usize {
    match value {
        Value::Object(map) => {
            let current = usize::from(map.get(key) == Some(expected));
            current
                + map
                    .values()
                    .map(|nested| count_exact_key_value(nested, key, expected))
                    .sum::<usize>()
        }
        Value::Array(items) => items
            .iter()
            .map(|nested| count_exact_key_value(nested, key, expected))
            .sum(),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => 0,
    }
}
