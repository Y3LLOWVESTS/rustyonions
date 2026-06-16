//! RO:WHAT — Parser consistency tests for ron-policy QuickChain preflight.
//! RO:WHY — Ensures JSON and TOML loaders both reject authority-shaped obligations.
//! RO:INTERACTS — `load_json`, `load_toml`, `parse::validate`.
//! RO:INVARIANTS — parser format must not change policy authority boundaries.

use ron_policy::{load_json, load_toml};

#[test]
fn json_loader_rejects_camel_case_authority_obligation_kind() {
    let err = load_json(
        br#"{
          "version": 1,
          "rules": [
            {
              "id": "bad-json-camel-kind",
              "when": { "method": "GET" },
              "action": "allow",
              "obligations": [
                { "kind": "unlockPaidContent", "params": {} }
              ]
            }
          ]
        }"#,
    )
    .expect_err("JSON loader must reject camelCase authority obligation kind");

    assert_authority_error(&err.to_string());
}

#[test]
fn json_loader_rejects_dash_separated_authority_obligation_param() {
    let err = load_json(
        br#"{
          "version": 1,
          "rules": [
            {
              "id": "bad-json-dash-param",
              "when": { "method": "GET" },
              "action": "allow",
              "obligations": [
                {
                  "kind": "require-backend-wallet-ledger-proof",
                  "params": { "receipt-id": "fake" }
                }
              ]
            }
          ]
        }"#,
    )
    .expect_err("JSON loader must reject dash-separated authority param key");

    assert_authority_error(&err.to_string());
}

#[test]
fn toml_loader_rejects_authority_obligation_kind() {
    let err = load_toml(
        br#"
version = 1

[[rules]]
id = "bad-toml-kind"
action = "allow"

[rules.when]
method = "GET"

[[rules.obligations]]
kind = "finalize_receipt"
"#,
    )
    .expect_err("TOML loader must reject authority obligation kind");

    assert_authority_error(&err.to_string());
}

#[test]
fn toml_loader_rejects_authority_obligation_param_key() {
    let err = load_toml(
        br#"
version = 1

[[rules]]
id = "bad-toml-param"
action = "allow"

[rules.when]
method = "GET"

[[rules.obligations]]
kind = "require-backend-wallet-ledger-proof"

[rules.obligations.params]
"checkpoint-root" = "fake"
"#,
    )
    .expect_err("TOML loader must reject authority obligation param key");

    assert_authority_error(&err.to_string());
}

fn assert_authority_error(message: &str) {
    assert!(
        message.contains("economic authority"),
        "expected economic authority validation error, got: {message}"
    );
}
