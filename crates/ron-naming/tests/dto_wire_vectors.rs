//! RO:WHAT — DTO wire-vector tests for ron-naming Address and NameRecord.
//! RO:WHY — Pillar 9; Concerns: SEC/GOV/DX. Canonical naming DTOs must round-trip.
//! RO:INTERACTS — ron_naming::{Address, ContentId, NameRecord, wire::{json,cbor}}.
//! RO:INVARIANTS — pure serde only; no IO/network; b3 IDs stay "b3:<64 lowercase hex>".
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — rejects malformed DTOs through strict typed deserialization.
//! RO:TEST — run with `cargo clippy -p ron-naming --all-targets -- -D warnings`.

use ron_naming::{
    normalize::normalize_fqdn_ascii,
    wire::{cbor, json},
    Address, ContentId, NameRecord,
};

const B3: &str = "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

#[test]
fn address_json_roundtrip_content_id() {
    let address = Address::parse(B3).expect("valid b3 content address");
    let decoded = json::roundtrip_address_json(&address).expect("json roundtrip");

    assert_eq!(decoded.to_compact(), B3);
    assert!(matches!(decoded, Address::Content { .. }));
}

#[test]
fn address_json_roundtrip_name_with_version() {
    let address = Address::parse("Files.Example@1.2.3").expect("valid versioned name");
    let decoded = json::roundtrip_address_json(&address).expect("json roundtrip");

    assert_eq!(decoded.to_compact(), "files.example@1.2.3");
    assert!(matches!(decoded, Address::Name { .. }));
}

#[test]
fn address_cbor_roundtrip_content_id() {
    let address = Address::parse(B3).expect("valid b3 content address");
    let decoded = cbor::roundtrip_address_cbor(&address).expect("cbor roundtrip");

    assert_eq!(decoded.to_compact(), B3);
    assert!(matches!(decoded, Address::Content { .. }));
}

#[test]
fn name_record_json_roundtrip() {
    let normalized = normalize_fqdn_ascii("Café.Example").expect("valid unicode domain");
    let record = NameRecord {
        name: normalized.0,
        version: None,
        content: ContentId(B3.to_owned()),
    };

    let decoded = json::roundtrip_record_json(&record).expect("json record roundtrip");

    assert_eq!(decoded.name.0, "xn--caf-dma.example");
    assert!(decoded.version.is_none());
    assert_eq!(decoded.content.0, B3);
}

#[test]
fn name_record_cbor_roundtrip() {
    let normalized = normalize_fqdn_ascii("files.example").expect("valid ascii domain");
    let record = NameRecord {
        name: normalized.0,
        version: None,
        content: ContentId(B3.to_owned()),
    };

    let decoded = cbor::roundtrip_record_cbor(&record).expect("cbor record roundtrip");

    assert_eq!(decoded.name.0, "files.example");
    assert!(decoded.version.is_none());
    assert_eq!(decoded.content.0, B3);
}

#[test]
fn json_bytes_are_valid_and_preserve_content_address() {
    let address = Address::parse(B3).expect("valid b3 content address");
    let bytes = json::to_json_bytes(&address).expect("json encode");

    assert!(!bytes.is_empty());

    let json_text = String::from_utf8(bytes.clone()).expect("utf8 json");
    assert!(json_text.contains(B3));

    let parsed: serde_json::Value = serde_json::from_slice(&bytes).expect("valid json value");
    assert!(!parsed.is_null());

    let decoded: Address = serde_json::from_slice(&bytes).expect("decode address from json");
    assert_eq!(decoded.to_compact(), B3);
    assert!(matches!(decoded, Address::Content { .. }));
}
