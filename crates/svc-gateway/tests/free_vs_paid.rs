// crates/gateway/tests/free_vs_paid.rs
#![forbid(unsafe_code)]

use reqwest::{Client, StatusCode};
use std::time::Duration;

/// Read an env var by primary name or any of a few alternates; trim whitespace.
fn env_any(primary: &str, alternates: &[&str]) -> Option<String> {
    std::env::var(primary)
        .ok()
        .or_else(|| {
            for k in alternates {
                if let Ok(v) = std::env::var(k) {
                    return Some(v);
                }
            }
            None
        })
        .map(|s| s.trim().to_string())
}

/// Configuration for the "online" test mode.
/// Returns None when we don't have enough info to run the assertions.
fn load_cfg() -> Option<(String, String, String)> {
    // Base URL of a running gateway (default to local dev port if not given)
    let base = env_any("GW_BASE_URL", &[]).unwrap_or_else(|| "http://127.0.0.1:9080".to_string());

    // A known-free object address, e.g., "b3:... .text"
    // Common alternates printed by our smoke tools: OBJ_ADDR / FREE_ADDR
    let free_addr = env_any("GW_FREE_ADDR", &["OBJ_ADDR", "FREE_ADDR"])?;

    // A known-paid object address, e.g., "b3:... .post"
    // Common alternates printed by our smoke tools: GW_PAID_ADDR / PAID_ADDR / POST_ADDR
    let paid_addr = env_any("GW_PAID_ADDR", &["PAID_ADDR", "POST_ADDR"])?;

    Some((base, free_addr, paid_addr))
}

async fn http_status(client: &Client, url: &str) -> Result<StatusCode, reqwest::Error> {
    let resp = client.get(url).send().await?;
    Ok(resp.status())
}

fn http_client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .expect("reqwest client")
}

#[tokio::test]
async fn free_bundle_returns_200() {
    let Some((base, free_addr, _)) = load_cfg() else {
        eprintln!(
            "[gateway/free_vs_paid] SKIP: set GW_BASE_URL + (GW_FREE_ADDR|OBJ_ADDR|FREE_ADDR) \
             and GW_PAID_ADDR (or POST_ADDR) to run this test against a live gateway."
        );
        return;
    };

    let client = http_client();
    let url = format!("{}/o/{}", base.trim_end_matches('/'), free_addr);
    let status = http_status(&client, &url).await.unwrap_or(StatusCode::SERVICE_UNAVAILABLE);

    assert_eq!(
        status,
        StatusCode::OK,
        "expected 200 for free bundle at {url}, got {status}"
    );
}

#[tokio::test]
async fn paid_bundle_returns_402() {
    let Some((base, _, paid_addr)) = load_cfg() else {
        eprintln!(
            "[gateway/free_vs_paid] SKIP: set GW_BASE_URL + (GW_FREE_ADDR|OBJ_ADDR|FREE_ADDR) \
             and GW_PAID_ADDR (or POST_ADDR) to run this test against a live gateway."
        );
        return;
    };

    let client = http_client();
    let url = format!("{}/o/{}", base.trim_end_matches('/'), paid_addr);
    let status = http_status(&client, &url).await.unwrap_or(StatusCode::SERVICE_UNAVAILABLE);

    // 402 Payment Required
    assert_eq!(
        status,
        StatusCode::PAYMENT_REQUIRED,
        "expected 402 for paid bundle at {url}, got {status}"
    );
}
