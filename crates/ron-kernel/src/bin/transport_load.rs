#![forbid(unsafe_code)]

use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::{info, Level};
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(Level::INFO.into()))
        .init();

    let target = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8088".to_string());
    info!(%target, "transport_load connecting");

    let mut stream = TcpStream::connect(&target).await?;
    let payload = b"hello";
    stream.write_all(payload).await?;

    let mut buf = [0u8; 64];
    let _n = stream.read(&mut buf).await.unwrap_or(0);

    stream.shutdown().await.ok();

    tokio::time::sleep(Duration::from_millis(100)).await;
    Ok(())
}
