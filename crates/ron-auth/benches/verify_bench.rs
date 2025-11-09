// RO:WHAT   Criterion microbench for verify_token / verify_many (small + heavy tokens).
// RO:WHY    Show hybrid crossover; optionally compare streaming-only vs SoA-only.
// RO:INVARIANTS Pure; BLAKE3; deterministic; no I/O.

use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
#[cfg(feature = "bench-eval-modes")]
use ron_auth::verify::{
    verify_many_soa_only, verify_many_streaming_only, verify_token_soa_only,
    verify_token_streaming_only,
};
use ron_auth::{
    sign_and_encode_b64url, verify_many, verify_token, CapabilityBuilder, Caveat, MacKey,
    MacKeyProvider, RequestCtx, Scope, VerifierConfig,
};
use serde_cbor::Value;
use std::net::IpAddr;
use std::str::FromStr;

#[derive(Clone)]
struct StaticKeys;
impl MacKeyProvider for StaticKeys {
    fn key_for(&self, kid: &str, tid: &str) -> Option<MacKey> {
        if kid == "k1" && tid == "test" {
            Some(MacKey(*b"0123456789abcdef0123456789abcdef"))
        } else {
            None
        }
    }
}

fn now() -> u64 {
    1_700_000_000
}
fn make_cfg() -> VerifierConfig {
    VerifierConfig {
        max_token_bytes: 4096,
        max_caveats: 128,
        clock_skew_secs: 60,
        // NEW knob in Perf Pack A (exported by ron-auth); keep default crossover.
        soa_threshold: 8,
    }
}

fn base_ctx() -> RequestCtx {
    // Method uppercased once; matches verifier’s fast path.
    RequestCtx {
        now_unix_s: now(),
        method: "GET".into(),
        path: "/index/1".into(),
        peer_ip: Some(IpAddr::from_str("127.0.0.1").unwrap()),
        object_addr: None,
        tenant: "test".into(),
        amnesia: false,
        policy_digest_hex: Some("aud-demo".into()),
        extras: Value::Null,
    }
}

fn make_token_small(keys: &impl MacKeyProvider) -> String {
    let scope = Scope {
        prefix: Some("/index/".into()),
        methods: vec!["GET".into()],
        max_bytes: None,
    };
    let cap = CapabilityBuilder::new(scope, "test", "k1")
        .caveat(Caveat::Aud("aud-demo".into()))
        .caveat(Caveat::PathPrefix("/index/".into()))
        .caveat(Caveat::Method(vec!["GET".into()]))
        .caveat(Caveat::Tenant("test".into()))
        .caveat(Caveat::Exp(now() + 600))
        .build();
    sign_and_encode_b64url(&cap, keys).unwrap()
}

// Heavier capability: multi-methods, prefixes, CIDRs, and bounds (~24–32 caveats).
fn make_token_heavy(keys: &impl MacKeyProvider) -> String {
    let methods = vec![
        "GET", "HEAD", "POST", "PUT", "PATCH", "DELETE", "OPTIONS", "TRACE", "CONNECT", "PROPFIND",
        "SEARCH", "COPY",
    ]
    .into_iter()
    .map(|s| s.to_string())
    .collect::<Vec<_>>();

    let scope = Scope {
        prefix: Some("/api/v1/tenant/test/objects/".into()),
        methods: methods.clone(),
        max_bytes: None,
    };

    let mut builder = CapabilityBuilder::new(scope, "test", "k1")
        .caveat(Caveat::Aud("aud-demo".into()))
        .caveat(Caveat::Tenant("test".into()))
        .caveat(Caveat::Exp(now() + 600))
        .caveat(Caveat::PathPrefix("/api/".into()))
        .caveat(Caveat::PathPrefix("/api/v1/".into()))
        .caveat(Caveat::PathPrefix("/api/v1/tenant/".into()))
        .caveat(Caveat::PathPrefix("/api/v1/tenant/test/".into()))
        .caveat(Caveat::Method(methods))
        .caveat(Caveat::IpCidr("127.0.0.0/8".into()))
        .caveat(Caveat::IpCidr("10.0.0.0/8".into()))
        .caveat(Caveat::IpCidr("192.168.0.0/16".into()))
        .caveat(Caveat::IpCidr("172.16.0.0/12".into()))
        .caveat(Caveat::BytesLe(1_048_576))
        .caveat(Caveat::Amnesia(false));

    // pad to ~30 caveats with no-op customs (ignored by evaluator)
    for i in 0..6 {
        builder = builder.caveat(Caveat::Custom {
            name: format!("x{}", i),
            ns: "demo".into(),
            cbor: Value::Null,
        });
    }

    let cap = builder.build();
    sign_and_encode_b64url(&cap, keys).unwrap()
}

fn benches_small(c: &mut Criterion) {
    let keys = StaticKeys;
    let token = make_token_small(&keys);
    let cfg_single = make_cfg();
    let ctx_single = base_ctx();

    c.bench_function("verify_single", |b| {
        b.iter(|| {
            let d = verify_token(&cfg_single, black_box(&token), &ctx_single, &keys).unwrap();
            black_box(d);
        })
    });

    c.bench_function("verify_batch_64_loop", |b| {
        b.iter_batched(
            || {
                let v: Vec<String> = vec![token.clone(); 64];
                (v, base_ctx(), make_cfg(), keys.clone())
            },
            |(tokens, ctx, cfg, keys)| {
                for t in tokens.iter() {
                    let d = verify_token(&cfg, t, &ctx, &keys).unwrap();
                    black_box(d);
                }
            },
            BatchSize::SmallInput,
        )
    });

    c.bench_function("verify_many_64", |b| {
        b.iter_batched(
            || {
                let v: Vec<String> = vec![token.clone(); 64];
                (v, base_ctx(), make_cfg(), keys.clone())
            },
            |(tokens, ctx, cfg, keys)| {
                let decisions = verify_many(&cfg, &tokens, &ctx, &keys).unwrap();
                black_box(decisions);
            },
            BatchSize::SmallInput,
        )
    });
}

fn benches_heavy(c: &mut Criterion) {
    let keys = StaticKeys;
    let token = make_token_heavy(&keys);

    c.bench_function("verify_single_heavy", |b| {
        b.iter(|| {
            let d = verify_token(&make_cfg(), black_box(&token), &base_ctx(), &keys).unwrap();
            black_box(d);
        })
    });

    c.bench_function("verify_many_64_heavy", |b| {
        b.iter_batched(
            || {
                let v: Vec<String> = vec![token.clone(); 64];
                (v, base_ctx(), make_cfg(), keys.clone())
            },
            |(tokens, ctx, cfg, keys)| {
                let decisions = verify_many(&cfg, &tokens, &ctx, &keys).unwrap();
                black_box(decisions);
            },
            BatchSize::SmallInput,
        )
    });

    // Optional: hard-toggle comparisons (requires `--features bench-eval-modes`)
    #[cfg(feature = "bench-eval-modes")]
    {
        c.bench_function("verify_single_heavy_streaming_only", |b| {
            b.iter(|| {
                let d =
                    verify_token_streaming_only(&make_cfg(), black_box(&token), &base_ctx(), &keys)
                        .unwrap();
                black_box(d);
            })
        });

        c.bench_function("verify_single_heavy_soa_only", |b| {
            b.iter(|| {
                let d = verify_token_soa_only(&make_cfg(), black_box(&token), &base_ctx(), &keys)
                    .unwrap();
                black_box(d);
            })
        });

        c.bench_function("verify_many_64_heavy_streaming_only", |b| {
            b.iter_batched(
                || {
                    let v: Vec<String> = vec![token.clone(); 64];
                    (v, base_ctx(), make_cfg(), keys.clone())
                },
                |(tokens, ctx, cfg, keys)| {
                    let decisions = verify_many_streaming_only(&cfg, &tokens, &ctx, &keys).unwrap();
                    black_box(decisions);
                },
                BatchSize::SmallInput,
            )
        });

        c.bench_function("verify_many_64_heavy_soa_only", |b| {
            b.iter_batched(
                || {
                    let v: Vec<String> = vec![token.clone(); 64];
                    (v, base_ctx(), make_cfg(), keys.clone())
                },
                |(tokens, ctx, cfg, keys)| {
                    let decisions = verify_many_soa_only(&cfg, &tokens, &ctx, &keys).unwrap();
                    black_box(decisions);
                },
                BatchSize::SmallInput,
            )
        });
    }
}

criterion_group!(benches, benches_small, benches_heavy);
criterion_main!(benches);
