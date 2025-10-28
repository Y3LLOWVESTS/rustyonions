//! RO:WHAT — Criterion bench for domain normalization (Unicode → ASCII FQDN).
//! RO:WHY  — Track perf and regressions for IDNA/NFC pipeline.
//! RO:NOTES — Keep vectors tiny; this is a types crate (no I/O).

use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use ron_naming::normalize::normalize_fqdn_ascii;

fn bench_normalize(c: &mut Criterion) {
    let cases = [
        "example.com",
        "Café.Example",
        "bücher.example",
        "δοκιμή.Ελλάδα",       // Greek
        "пример.рф",           // Cyrillic
        "예시.테스트",         // Korean
        "مثال.إختبار",         // Arabic
        "παράδειγμα.δοκιμή",   // Greek extended
        "xn--caf-dma.example", // already punycoded
    ];

    c.bench_function("normalize_fqdn_ascii/mixed", |b| {
        b.iter_batched(
            || cases.to_vec(),
            |inputs| {
                for s in inputs {
                    let out = normalize_fqdn_ascii(black_box(s)).unwrap();
                    black_box(out);
                }
            },
            BatchSize::SmallInput,
        )
    });

    c.bench_function("normalize_fqdn_ascii/hot_ascii", |b| {
        b.iter(|| {
            let out = normalize_fqdn_ascii(black_box("sub.service.ron.dev")).unwrap();
            black_box(out);
        })
    });
}

criterion_group!(naming_norm, bench_normalize);
criterion_main!(naming_norm);
