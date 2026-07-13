//! Example: POST /dht/provide to a running svc-dht.
//! Run the service in another terminal: `cargo run -p svc-dht`
//! Then: `cargo run -p svc-dht --example provide -- b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef crab://node/00000000000000000000000000000000000000000000000000000000000000a1 60`

use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let cid = args.next().unwrap_or_else(|| {
        "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string()
    });
    let node = args.next().unwrap_or_else(|| {
        "crab://node/00000000000000000000000000000000000000000000000000000000000000a1".to_string()
    });
    let ttl = args.next().and_then(|s| s.parse::<u64>().ok()).unwrap_or(60);
    let addr = std::env::var("DHT_ADDR").unwrap_or_else(|_| "127.0.0.1:5301".into());

    let url = format!("http://{addr}/dht/provide");
    let body = serde_json::json!({ "cid": cid, "node": node, "ttl_secs": ttl });
    let cli = reqwest::Client::builder().timeout(Duration::from_secs(5)).build()?;
    let res = cli.post(url).json(&body).send().await?.text().await?;
    println!("{res}");
    Ok(())
}
