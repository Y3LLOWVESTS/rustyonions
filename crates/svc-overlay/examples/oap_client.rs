use anyhow::{anyhow, Result};
use bytes::BytesMut;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;

use svc_overlay::conn::writer::write_frame;
use svc_overlay::protocol::flags::Caps;
use svc_overlay::protocol::handshake::handshake;
use svc_overlay::protocol::oap::{try_parse_frame, Frame, FrameKind};

#[tokio::main]
async fn main() -> Result<()> {
    let addr: SocketAddr = std::env::var("OVERLAY_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:9700".into())
        .parse()?;

    eprintln!("[client] connecting to {}", addr);
    let mut sock = TcpStream::connect(addr).await?;

    // Handshake to match server
    let caps = Caps::GOSSIP_V1;
    let neg = handshake(&mut sock, caps, Duration::from_secs(3)).await?;
    eprintln!(
        "[client] negotiated version={} caps={:?}",
        neg.version, neg.caps
    );

    // Send one Data frame and verify echo
    let mut outbuf = BytesMut::with_capacity(1024);
    let mut inbuf = BytesMut::with_capacity(1024);

    let payload = b"hello-overlay";
    let frame = Frame {
        kind: FrameKind::Data,
        payload: payload.as_slice().into(),
    };
    write_frame(&mut sock, &frame, &mut outbuf).await?;

    loop {
        while let Some(f) = try_parse_frame(&mut inbuf)? {
            if let FrameKind::Data = f.kind {
                if f.payload.as_ref() == payload {
                    eprintln!("[client] got echo OK");
                    return Ok(());
                } else {
                    return Err(anyhow!("unexpected payload {:?}", f.payload));
                }
            }
        }
        let n = sock.read_buf(&mut inbuf).await?;
        if n == 0 {
            return Err(anyhow!("server closed before echo"));
        }
    }
}
