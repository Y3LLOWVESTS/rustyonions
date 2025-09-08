#![forbid(unsafe_code)]

use anyhow::Result;
use gateway::oap::OapServer;
use oap::{
    data_frame, end_frame, hello_frame, read_frame, start_frame, write_frame, DEFAULT_MAX_FRAME,
};
use ron_kernel::bus::{sub, Bus};
use ron_kernel::KernelEvent;
use serde_json::json;
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

#[tokio::test]
async fn oap_roundtrip_ack_and_bus_events() -> Result<()> {
    // Kernel bus shared with server; we subscribe to assert events.
    let bus = Bus::new(64);
    let mut rx = bus.subscribe();

    // Start server on ephemeral port
    let srv = OapServer::new(bus.clone());
    let (handle, bound) = srv.serve("127.0.0.1:0".parse()?).await?;

    // Client connects and sends a small flow
    let mut s = TcpStream::connect(bound).await?;

    write_frame(&mut s, &hello_frame("oap/1"), DEFAULT_MAX_FRAME).await?;
    write_frame(&mut s, &start_frame("demo/topic"), DEFAULT_MAX_FRAME).await?;

    // One small DATA chunk (won't hit the server's ACK threshold)
    let body = b"hello world";
    let df = data_frame(json!({"mime":"text/plain"}), body, DEFAULT_MAX_FRAME)?;
    write_frame(&mut s, &df, DEFAULT_MAX_FRAME).await?;

    // Non-blocking peek for a server frame; ignore if timeout (no ACK for tiny payloads).
    let _ = timeout(Duration::from_millis(50), read_frame(&mut s, DEFAULT_MAX_FRAME)).await;

    // END stream
    write_frame(&mut s, &end_frame(), DEFAULT_MAX_FRAME).await?;

    // Assert we observed at least one expected bus event (START or DATA)
    let start_ok = sub::recv_matching(&bus, &mut rx, Duration::from_secs(1), |ev| {
        matches!(ev, KernelEvent::Health { service, ok } if service == "oap-start:demo/topic" && *ok)
    })
    .await
    .is_some();

    let data_seen = sub::recv_matching(&bus, &mut rx, Duration::from_secs(1), |ev| {
        matches!(ev, KernelEvent::ConfigUpdated { version } if *version == body.len() as u64)
    })
    .await
    .is_some();

    assert!(start_ok || data_seen, "expected start or data event on bus");

    // Cleanup
    handle.abort();
    Ok(())
}
