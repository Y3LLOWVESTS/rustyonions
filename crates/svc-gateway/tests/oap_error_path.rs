#![forbid(unsafe_code)]

use anyhow::Result;
use gateway::oap::OapServer;
use oap::{
    encode_data_payload, end_frame, hello_frame, read_frame, start_frame, write_frame, FrameType,
    OapFrame, DEFAULT_MAX_FRAME,
};
use ron_kernel::bus::Bus;
use serde_json::json;
use tokio::net::TcpStream;

#[tokio::test]
async fn rejects_mismatched_obj_digest() -> Result<()> {
    // Start server
    let bus = Bus::new(32);
    let srv = OapServer::new(bus);
    let (_handle, bound) = srv.serve("127.0.0.1:0".parse()?).await?;

    // Connect client
    let mut s = TcpStream::connect(bound).await?;

    // HELLO + START
    write_frame(&mut s, &hello_frame("oap/1"), DEFAULT_MAX_FRAME).await?;
    write_frame(&mut s, &start_frame("demo/topic"), DEFAULT_MAX_FRAME).await?;

    // Forge a DATA payload with a wrong obj (server should reject)
    let body = b"abc123";
    let bad_hdr = json!({
        "mime": "text/plain",
        "obj": "b3:0000000000000000000000000000000000000000000000000000000000000000"
    });
    let payload = encode_data_payload(bad_hdr, body)?; // encode will preserve our wrong obj
    let df = OapFrame::new(FrameType::Data, payload);
    write_frame(&mut s, &df, DEFAULT_MAX_FRAME).await?;

    // Expect an Error frame
    let fr = read_frame(&mut s, DEFAULT_MAX_FRAME).await?;
    assert!(
        matches!(fr.typ, FrameType::Error),
        "expected Error, got {:?}",
        fr.typ
    );

    // END (cleanup) – server may close after Error, so ignore failures
    let _ = write_frame(&mut s, &end_frame(), DEFAULT_MAX_FRAME).await;
    Ok(())
}
