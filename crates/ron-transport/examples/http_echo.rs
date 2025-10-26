//! Minimal HTTP-ish echo over raw TCP (not using the library).
//! Purpose: easy curl checks that show a visible response.

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
    let addr = listener.local_addr()?;
    println!("http-echo listening on {}", addr);

    loop {
        let (mut sock, _peer) = listener.accept().await?;
        tokio::spawn(async move {
            let mut buf = vec![0u8; 16 * 1024];

            // Read once (simple demo) — enough for small requests.
            let n = match sock.read(&mut buf).await {
                Ok(0) => return,
                Ok(m) => m,
                Err(_) => return,
            };

            // Craft a simple 200 OK with echoed body.
            let body = &buf[..n];
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\n",
                body.len()
            );

            if sock.write_all(header.as_bytes()).await.is_err() {
                return;
            }
            let _ = sock.write_all(body).await;
            let _ = sock.flush().await;
        });
    }
}
