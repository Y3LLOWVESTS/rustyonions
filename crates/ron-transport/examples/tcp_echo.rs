//! Minimal TCP echo (for human-visible round-trips with curl/nc).
//! NOTE: This is a standalone echo using Tokio, not the library accept loop.
//! It’s just for smoke-testing with tools that expect a response.

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
    let addr = listener.local_addr()?;
    println!("echo listening on {}", addr);

    loop {
        let (mut sock, _peer) = listener.accept().await?;
        tokio::spawn(async move {
            let mut buf = vec![0u8; 4096];
            loop {
                let n = match sock.read(&mut buf).await {
                    Ok(0) => return, // closed
                    Ok(n) => n,
                    Err(_) => return,
                };
                if sock.write_all(&buf[..n]).await.is_err() {
                    return;
                }
            }
        });
    }
}
