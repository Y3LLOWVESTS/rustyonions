//! RO:WHAT — CLI contract tests for `tldctl`.
//! RO:WHY  — Ensure CLI stays stable. Only runs with `--features cli`.

#![cfg(feature = "cli")]

use assert_cmd::Command;

#[test]
fn normalize_cli_outputs_ascii() {
    let mut cmd = Command::cargo_bin("tldctl").expect("build tldctl");
    cmd.args(["normalize", "Café.Example"]);
    let assert = cmd.assert().success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert_eq!(out.trim(), "xn--caf-dma.example");
}

#[test]
fn parse_cli_emits_json() {
    let mut cmd = Command::cargo_bin("tldctl").expect("build tldctl");
    cmd.args(["parse", "files.example@1.2.3"]);
    let assert = cmd.assert().success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(out.contains(r#""kind": "name""#));
    assert!(out.contains(r#""fqdn": "files.example""#));
    assert!(out.contains(r#""version": "1.2.3""#));
}
