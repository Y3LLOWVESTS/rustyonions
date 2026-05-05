//! RO:WHAT — I-6 DTO strictness and validation checks for `ron-app-sdk`.
//! RO:WHY — Ensures SDK boundary types reject malformed canonical identifiers.
//! RO:INTERACTS — AddrB3, Capability, ron-proto capability caveats.
//! RO:INVARIANTS — b3 addresses are lowercase `b3:<64hex>`; capability caveats are typed.
//! RO:SECURITY — Rejects ambiguous content IDs before a backend call.
//! RO:TEST — cargo clippy -p ron-app-sdk --all-targets -- -D warnings.

use ron_app_sdk::{AddrB3, Capability};
use ron_proto::cap::Caveat;

#[test]
fn addr_b3_accepts_only_canonical_lowercase_digest_form() {
    let canonical = "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let uppercase = "b3:0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF";
    let missing_prefix = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let short = "b3:0123456789abcdef";

    let parsed = AddrB3::parse(canonical).expect("canonical b3 address should parse");
    assert_eq!(parsed.as_str(), canonical);

    assert!(AddrB3::parse(uppercase).is_err());
    assert!(AddrB3::parse(missing_prefix).is_err());
    assert!(AddrB3::parse(short).is_err());
}

#[test]
fn capability_dto_requires_explicit_subject_scope_and_typed_caveats() {
    let cap = Capability {
        subject: "passport:main:test".to_owned(),
        scope: "storage:write".to_owned(),
        issued_at: 1_700_000_000,
        expires_at: 1_700_003_600,
        caveats: vec![
            Caveat::ContentPrefix {
                prefix: "account:acct_dev".to_owned(),
            },
            Caveat::ContentPrefix {
                prefix: "b3:".to_owned(),
            },
            Caveat::WriteOnce,
        ],
    };

    assert_eq!(cap.subject, "passport:main:test");
    assert_eq!(cap.scope, "storage:write");
    assert!(cap.expires_at > cap.issued_at);
    assert_eq!(cap.caveats.len(), 3);

    match &cap.caveats[0] {
        Caveat::ContentPrefix { prefix } => assert_eq!(prefix, "account:acct_dev"),
        other => panic!("expected ContentPrefix caveat, got {other:?}"),
    }

    assert!(matches!(cap.caveats[2], Caveat::WriteOnce));
}
