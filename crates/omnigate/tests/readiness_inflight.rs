//! Verifies that sustained in-flight pressure trips /readyz → 503,
//! holds for hold_for_secs, then recovers to 200.

use anyhow::Result;
use omnigate::{config, App};
use reqwest::Client;
use std::net::SocketAddr;
use tokio::{
    net::TcpListener,
    time::{sleep, Duration},
};

async fn spawn_app_with_cfg(mut cfg: config::Config) -> Result<(SocketAddr, SocketAddr)> {
    // Bind API to an ephemeral port; we’ll override at serve-time.
    cfg.server.bind = "127.0.0.1:0"
        .parse()
        .unwrap_or("127.0.0.1:0".parse().unwrap());
    // Bind metrics/admin plane to ephemeral as well; App::build will return the actual addr.
    cfg.server.metrics_addr = "127.0.0.1:0".parse().unwrap();

    let app = App::build(cfg).await?;
    // Bind listener for API
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let api_addr = listener.local_addr()?;
    let admin_addr = app.admin_addr;

    // Serve API in background
    let router = app.router;
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    Ok((api_addr, admin_addr))
}

fn cfg_inflight_low_threshold() -> config::Config {
    config::Config {
        server: config::Server {
            bind: "127.0.0.1:5305".parse().unwrap(), // ignored (we bind 0)
            metrics_addr: "127.0.0.1:9605".parse().unwrap(), // ignored (we bind 0)
            amnesia: true,
        },
        oap: config::Oap {
            max_frame_bytes: 1_048_576,
            stream_chunk_bytes: 65_536,
        },
        admission: config::Admission {
            global_quota: config::GlobalQuota {
                qps: 20_000,
                burst: 40_000,
            },
            ip_quota: config::IpQuota {
                enabled: true,
                qps: 2_000,
                burst: 4_000,
            },
            fair_queue: config::FairQueue {
                max_inflight: 2048,
                headroom: Some(256),
                weights: config::Weights {
                    anon: 1,
                    auth: 5,
                    admin: 10,
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
            enabled: true,
            bundle_path: "crates/omnigate/configs/policy.bundle.json".into(),
            fail_mode: "deny".into(),
        },
        // Low thresholds so laptops trip quickly
        readiness: config::Readiness {
            max_inflight_threshold: 64,
            error_rate_429_503_pct: 100.0, // make inflight the only trigger here
            window_secs: 5,
            hold_for_secs: 6,
        },
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn readiness_trips_on_inflight_then_recovers() -> Result<()> {
    let (api, admin) = spawn_app_with_cfg(cfg_inflight_low_threshold()).await?;
    let api_base = format!("http://{}", api);
    let ops_base = format!("http://{}", admin);
    let client = Client::builder().build()?;

    // 1) Sanity: /readyz starts 200
    let code = client
        .get(format!("{}/readyz", api_base))
        .send()
        .await?
        .status();
    assert!(code.is_success(), "expected 200 from /readyz, got {}", code);

    // 2) Create sustained in-flight using /v1/sleep
    let concurrency = 200usize;
    let sleep_ms = 800u64;

    let mut joins = Vec::with_capacity(concurrency);
    for _ in 0..concurrency {
        let url = format!("{}/v1/sleep?ms={}", api_base, sleep_ms);
        let c = client.clone();
        joins.push(tokio::spawn(async move {
            let _ = c.get(url).send().await;
        }));
    }

    // 3) Poll /readyz until it flips to 503 or timeout
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
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
        sleep(Duration::from_millis(200)).await;
    }
    assert!(degraded, "expected /readyz to degrade to 503 under load");

    // 4) During hold, /readyz should remain 503
    let code_hold = client
        .get(format!("{}/readyz", api_base))
        .send()
        .await?
        .status();
    assert_eq!(code_hold.as_u16(), 503, "expected hold window to stick");

    // 5) Wait hold_for_secs + cushion and expect recovery to 200
    sleep(Duration::from_secs(7)).await;
    let code_recover = client
        .get(format!("{}/readyz", api_base))
        .send()
        .await?
        .status();
    assert!(
        code_recover.is_success(),
        "expected /readyz to recover to 200"
    );

    // Optional: sample metrics once to ensure exporter is alive
    let _ = client.get(format!("{}/metrics", ops_base)).send().await?;

    // Ensure sleepers finish
    for j in joins {
        let _ = j.await;
    }

    Ok(())
}
