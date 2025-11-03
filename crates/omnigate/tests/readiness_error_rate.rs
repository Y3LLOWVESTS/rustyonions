//! Verifies that sustained 503 drop/error rate trips /readyz → 503 (error-rate branch).
//! Strategy: set *tiny* fair-queue capacity to force 503 drops, but set inflight-threshold huge
//! so we do not trip via inflight. The sampler counts fair_q drops toward error-rate.

use anyhow::Result;
use omnigate::{config, App};
use reqwest::Client;
use std::net::SocketAddr;
use tokio::{
    net::TcpListener,
    time::{sleep, Duration},
};

async fn spawn_app_with_cfg(mut cfg: config::Config) -> Result<(SocketAddr, SocketAddr)> {
    cfg.server.bind = "127.0.0.1:0".parse().unwrap();
    cfg.server.metrics_addr = "127.0.0.1:0".parse().unwrap();

    let app = App::build(cfg).await?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let api_addr = listener.local_addr()?;
    let admin_addr = app.admin_addr;

    let router = app.router;
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    Ok((api_addr, admin_addr))
}

fn cfg_error_rate_via_fair_queue() -> config::Config {
    config::Config {
        server: config::Server {
            bind: "127.0.0.1:5305".parse().unwrap(),
            metrics_addr: "127.0.0.1:9605".parse().unwrap(),
            amnesia: true,
        },
        oap: config::Oap {
            max_frame_bytes: 1_048_576,
            stream_chunk_bytes: 65_536,
        },
        admission: config::Admission {
            // Quotas generous (we don't want 429s for this test)
            global_quota: config::GlobalQuota {
                qps: 1_000_000,
                burst: 1_000_000,
            },
            ip_quota: config::IpQuota {
                enabled: false,
                qps: 0,
                burst: 0,
            },
            // FAIR-QUEUE: extremely tiny hard limit to force 503 drops under concurrency
            fair_queue: config::FairQueue {
                max_inflight: 8,
                headroom: Some(0),
                weights: config::Weights {
                    anon: 1,
                    auth: 1,
                    admin: 1,
                },
            },
            body: config::BodyCaps {
                max_content_length: 1_048_576,
                reject_on_missing_length: true,
            },
            decompression: config::Decompress {
                allow: vec!["identity".into(), "gzip".into()],
                deny_stacked: true,
            },
        },
        policy: config::Policy {
            enabled: false, // keep policy out of the picture for this test
            bundle_path: "crates/omnigate/configs/policy.bundle.json".into(),
            fail_mode: "deny".into(),
        },
        readiness: config::Readiness {
            max_inflight_threshold: 1_000_000, // make inflight path irrelevant
            error_rate_429_503_pct: 0.1,       // very easy to trip
            window_secs: 2,                    // short window => quick sampler ticks
            hold_for_secs: 3,
        },
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn readiness_trips_on_error_rate() -> Result<()> {
    let (api, _admin) = spawn_app_with_cfg(cfg_error_rate_via_fair_queue()).await?;
    let api_base = format!("http://{}", api);
    let client = Client::builder().build()?;

    // 1) Sanity: /readyz starts 200
    let code = client
        .get(format!("{}/readyz", api_base))
        .send()
        .await?
        .status();
    assert!(code.is_success(), "expected 200 from /readyz, got {}", code);

    // 2) Create a heavy burst against /v1/sleep that will exceed the tiny fair-queue capacity
    let concurrency = 200usize;
    let sleep_ms = 800u64;
    let mut joins = Vec::with_capacity(concurrency);
    for _ in 0..concurrency {
        let url = format!("{}/v1/sleep?ms={}", api_base, sleep_ms);
        let c = client.clone();
        joins.push(tokio::spawn(async move {
            let _ = c.get(url).send().await; // many of these will be 503 from shedding
        }));
    }

    // 3) Poll /readyz until it flips to 503 (expect via error-rate branch)
    // Give sampler windows a chance to observe drops: up to ~8s
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    let mut degraded = false;
    while tokio::time::Instant::now() < deadline {
        let code = client
            .get(format!("{}/readyz", api_base))
            .send()
            .await?
            .status();
        if code.as_u16() == 503 {
            degraded = true;
            break;
        }
        sleep(Duration::from_millis(150)).await;
    }

    for j in joins {
        let _ = j.await;
    }
    assert!(
        degraded,
        "expected /readyz to degrade to 503 by error-rate (fair-queue drops)"
    );
    Ok(())
}
