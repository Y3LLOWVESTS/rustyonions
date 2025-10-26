//! Minimal client: dial localhost, do OAP/1 hello, send one DATA frame, read echo.
//! Run with: cargo run -p svc-overlay --example libapi_embed

use anyhow::Result;
use bytes::{Bytes, BytesMut};
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;

use svc_overlay::conn::writer::write_frame;
use svc_overlay::protocol::flags::Caps;
use svc_overlay::protocol::handshake::handshake;
use svc_overlay::protocol::oap::{try_parse_frame, Frame, FrameKind};

#[tokio::main]
async fn main() -> Result<()> {
    // Quick logger
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "info");
    }
    tracing_subscriber::fmt::init();

    // 1) Connect to the overlay's temporary listener.
    let addr = "127.0.0.1:9700";
    tracing::info!("dialing {addr}");
    let mut sock = TcpStream::connect(addr).await?;
    tracing::info!("connected; performing OAP/1 handshake");

    // 2) Symmetric OAP/1 hello.
    let caps = Caps::GOSSIP_V1;
    let neg = handshake(&mut sock, caps, Duration::from_secs(3)).await?;
    tracing::info!("negotiated: ver={}, caps={:?}", neg.version, neg.caps);

    // 3) Send a single DATA frame.
    let payload = Bytes::from_static(b"hello, overlay!");
    let frame = Frame {
        kind: FrameKind::Data,
        payload: payload.clone(),
    };
    let mut scratch = BytesMut::with_capacity(1024);
    write_frame(&mut sock, &frame, &mut scratch).await?;
    tracing::info!("sent one DATA frame; waiting for echo");

    // 4) Read back echo and print.
    let mut inbuf = BytesMut::with_capacity(4096);
    loop {
        // Try parse any buffered frames first.
        if let Some(f) = try_parse_frame(&mut inbuf)? {
            if let FrameKind::Data = f.kind {
                let text = String::from_utf8_lossy(&f.payload);
                println!("echo from overlay: {}", text);
                break;
            }
        }
        // Need more bytes.
        let n = sock.read_buf(&mut inbuf).await?;
        if n == 0 {
            anyhow::bail!("server closed before echo");
        }
    }

    Ok(())
}
