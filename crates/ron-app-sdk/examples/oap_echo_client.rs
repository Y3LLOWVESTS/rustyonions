//! Minimal echo client: connects, HELLO, then sends a one-shot REQ to app_proto_id=0x0F01
//! Usage:
//!   RON_ADDR=127.0.0.1:9443 RON_SNI=localhost cargo run -p ron-app-sdk --example oap_echo_client

use anyhow::Result;
use bytes::Bytes;
use ron_app_sdk::OverlayClient;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    let addr = std::env::var("RON_ADDR").unwrap_or_else(|_| "127.0.0.1:9443".to_string());
    let sni = std::env::var("RON_SNI").unwrap_or_else(|_| "localhost".to_string());

    let mut client = OverlayClient::connect(&addr, &sni).await?;
    let hello = client.hello().await?;
    println!("HELLO: {hello:#?}");

    // Build a tiny JSON payload and send to a demo app_proto_id (0x0F01).
    let payload = Bytes::from(serde_json::to_vec(&json!({"op":"echo","msg":"ping"}))?);
    let resp = client.request_oneshot(0x0F01, 0, payload).await?;

    println!("RESP bytes: {}", resp.len());
    if let Ok(txt) = std::str::from_utf8(&resp) {
        println!("RESP text: {txt}");
    }
    Ok(())
}
