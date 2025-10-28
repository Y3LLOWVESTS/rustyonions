//! Example: GET /dht/find_providers/:cid from a running svc-dht.
//! Run the service in another terminal: `cargo run -p svc-dht`
//! Then: `cargo run -p svc-dht --example find_providers -- b3:deadbeef`

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cid = std::env::args().nth(1).unwrap_or_else(|| "b3:deadbeef".into());
    let addr = std::env::var("DHT_ADDR").unwrap_or_else(|_| "127.0.0.1:5301".into());
    let url = format!("http://{addr}/dht/find_providers/{cid}");
    let txt = reqwest::get(url).await?.text().await?;
    println!("{txt}");
    Ok(())
}
