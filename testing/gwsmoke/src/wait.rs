use anyhow::{anyhow, Result};
use std::path::Path;
use tokio::{
    fs,
    net::{TcpListener, TcpStream},
    time::{sleep, Duration},
};

pub async fn wait_for_uds(path: &Path, total: Duration) -> Result<()> {
    let start = std::time::Instant::now();
    while start.elapsed() < total {
        if fs::metadata(path).await.is_ok() {
            return Ok(());
        }
        sleep(Duration::from_millis(50)).await;
    }
    Err(anyhow!("UDS not created: {}", path.display()))
}

pub async fn wait_for_tcp(bind: &str, total: Duration) -> Result<()> {
    let start = std::time::Instant::now();
    while start.elapsed() < total {
        if TcpStream::connect(bind).await.is_ok() {
            return Ok(());
        }
        sleep(Duration::from_millis(50)).await;
    }
    Err(anyhow!("TCP not accepting at {}", bind))
}

pub async fn pick_ephemeral_port(host: &str) -> Result<u16> {
    let listener = TcpListener::bind((host, 0)).await?;
    let port = listener.local_addr()?.port();
    drop(listener);
    Ok(port)
}

pub fn parse_host_port(s: &str) -> Result<(&str, u16)> {
    let (h, p) = s
        .rsplit_once(':')
        .ok_or_else(|| anyhow!("--bind must be host:port"))?;
    let port: u16 = p.parse()?;
    Ok((h, port))
}
