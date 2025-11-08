// crates/svc-passport/benches/verify.rs
use std::sync::Arc;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use futures::executor;
use svc_passport::{
    config::Config,
    kms::client::{DevKms, KmsClient},
    state::issuer::IssuerState,
};

// Embed default config so benches don't rely on filesystem layout.
const EMBEDDED_DEFAULT_TOML: &str = include_str!("../config/default.toml");

fn ensure_bench_config() {
    // Only set if caller didn't provide something explicit.
    if std::env::var("PASSPORT_CONFIG").is_err() && std::env::var("PASSPORT_CONFIG_FILE").is_err() {
        std::env::set_var("PASSPORT_CONFIG", EMBEDDED_DEFAULT_TOML);
    }
}

fn make_issuer() -> IssuerState {
    ensure_bench_config();
    let kms: Arc<dyn KmsClient> = Arc::new(DevKms::new());
    let cfg = Config::load().expect("Config::load() in benches");
    IssuerState::new(cfg, kms)
}

pub fn bench_verify_single(c: &mut Criterion) {
    c.bench_function("verify_single", |b| {
        b.iter_batched(
            || {
                let issuer = make_issuer();
                let msg = br#"{"hello":"world"}"#.to_vec();
                let (kid, sig) = executor::block_on(issuer.sign(&msg)).unwrap();
                (issuer, kid, msg, sig)
            },
            |(issuer, kid, msg, sig)| {
                let ok = executor::block_on(issuer.verify(&kid, &msg, &sig)).unwrap();
                black_box(ok);
            },
            BatchSize::SmallInput,
        )
    });
}

pub fn bench_verify_batch_64(c: &mut Criterion) {
    c.bench_function("verify_batch_64", |b| {
        b.iter_batched(
            || {
                let issuer = make_issuer();
                let msg = br#"{"hello":"world"}"#.to_vec();
                let mut envs = Vec::with_capacity(64);
                for _ in 0..64 {
                    let (kid, sig) = executor::block_on(issuer.sign(&msg)).unwrap();
                    let env = serde_json::json!({
                        "kid": kid,
                        "msg_b64": STANDARD.encode(&msg),
                        "sig_b64": STANDARD.encode(&sig),
                        "alg": "Ed25519"
                    });
                    envs.push(env);
                }
                (issuer, envs)
            },
            |(issuer, envs)| {
                for env in &envs {
                    let kid = env["kid"].as_str().unwrap();
                    let msg = STANDARD.decode(env["msg_b64"].as_str().unwrap()).unwrap();
                    let sig = STANDARD.decode(env["sig_b64"].as_str().unwrap()).unwrap();
                    let ok = futures::executor::block_on(issuer.verify(kid, &msg, &sig)).unwrap();
                    black_box(ok);
                }
            },
            BatchSize::SmallInput,
        )
    });
}

criterion_group!(benches, bench_verify_single, bench_verify_batch_64);
criterion_main!(benches);
