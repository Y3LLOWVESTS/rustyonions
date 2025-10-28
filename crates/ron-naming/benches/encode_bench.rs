//! RO:WHAT — Criterion benches for JSON/CBOR encode/decode of DTOs.
//! RO:WHY  — Wire-format throughput snapshot for SDK users.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ron_naming::{
    types::{ContentId, Fqdn, NameRecord},
    version::parse_version,
    wire, Address,
};

fn bench_json_cbor(c: &mut Criterion) {
    let addr = Address::Name {
        fqdn: Fqdn("files.example".into()),
        version: Some(parse_version("1.2.3").unwrap()),
    };
    let rec = NameRecord {
        name: Fqdn("files.example".into()),
        version: Some(parse_version("1.2.3").unwrap()),
        content: ContentId(
            "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".into(),
        ),
    };

    c.bench_function("json/address_roundtrip", |b| {
        b.iter(|| {
            let a = wire::json::roundtrip_address_json(black_box(&addr)).unwrap();
            black_box(a);
        })
    });

    c.bench_function("json/record_roundtrip", |b| {
        b.iter(|| {
            let r = wire::json::roundtrip_record_json(black_box(&rec)).unwrap();
            black_box(r);
        })
    });

    c.bench_function("cbor/address_roundtrip", |b| {
        b.iter(|| {
            let a = wire::cbor::roundtrip_address_cbor(black_box(&addr)).unwrap();
            black_box(a);
        })
    });

    c.bench_function("cbor/record_roundtrip", |b| {
        b.iter(|| {
            let r = wire::cbor::roundtrip_record_cbor(black_box(&rec)).unwrap();
            black_box(r);
        })
    });
}

criterion_group!(naming_encode, bench_json_cbor);
criterion_main!(naming_encode);
