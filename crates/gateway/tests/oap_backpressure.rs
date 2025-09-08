#![forbid(unsafe_code)]

use anyhow::{bail, Result};
use gateway::oap::OapServer;
use oap::{read_frame, FrameType, DEFAULT_MAX_FRAME, write_frame, hello_frame};
use ron_kernel::bus::Bus;
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

#[tokio::test]
async fn busy_connections_get_error() -> Result<()> {
    // Start server with concurrency_limit = 1 so a second connect is rejected.
    let mut srv = OapServer::new(Bus::new(8));
    srv.concurrency_limit = 1;
    let (_handle, bound) = srv.serve("127.0.0.1:0".parse()?).await?;

    // First client connects and holds the slot by sending HELLO and never finishing.
    let mut c1 = TcpStream::connect(bound).await?;
    write_frame(&mut c1, &hello_frame("oap/1"), DEFAULT_MAX_FRAME).await?;

    // Second client tries to connect: should get an immediate Error frame.
    let mut c2 = TcpStream::connect(bound).await?;
    let read = timeout(Duration::from_millis(200), read_frame(&mut c2, DEFAULT_MAX_FRAME)).await;
    let fr = match read {
        Ok(Ok(f)) => f,
        Ok(Err(e)) => bail!("read failed: {e}"),
        Err(_) => bail!("timed out waiting for busy error"),
    };
    assert!(matches!(fr.typ, FrameType::Error), "expected Error frame for busy, got {:?}", fr.typ);
    Ok(())
}
