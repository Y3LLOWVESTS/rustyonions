#![forbid(unsafe_code)]

use anyhow::Result;
use oap::{
    data_frame, end_frame, hello_frame, read_frame, start_frame, write_frame, FrameType,
    DEFAULT_MAX_FRAME,
};
use serde_json::json;
use tokio::{io::AsyncWriteExt, net::TcpStream, time::{timeout, Duration}};

const ADDR: &str = "127.0.0.1:9444";

// Send HELLO -> START("demo/topic") -> DATA x N -> END
// Then read back any ACK/ERROR frames the server emits.
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    println!("connecting to {ADDR} …");
    let mut s = TcpStream::connect(ADDR).await?;

    // HELLO + START
    write_frame(&mut s, &hello_frame("oap/1"), DEFAULT_MAX_FRAME).await?;
    write_frame(&mut s, &start_frame("demo/topic"), DEFAULT_MAX_FRAME).await?;

    // Send a few DATA chunks (20 KiB each) so we cross the server’s ACK threshold.
    for i in 0..5 {
        let body = make_body(i, 20 * 1024);
        let hdr = json!({ "mime":"text/plain", "seq": i });
        let df = data_frame(hdr, &body, DEFAULT_MAX_FRAME)?;
        write_frame(&mut s, &df, DEFAULT_MAX_FRAME).await?;
        println!("sent DATA seq={i} bytes={}", body.len());

        // Opportunistically poll for ACKs without blocking too long.
        poll_for_server_frames(&mut s, Duration::from_millis(10)).await?;
    }

    // END the stream
    write_frame(&mut s, &end_frame(), DEFAULT_MAX_FRAME).await?;
    // make sure everything is on the wire
    s.flush().await?;

    // Give the server a moment to send any final ACKs
    poll_for_server_frames(&mut s, Duration::from_millis(300)).await?;

    println!("client done.");
    Ok(())
}

fn make_body(seq: u32, n: usize) -> Vec<u8> {
    let prefix = format!("chunk-{seq}:");
    let mut v = Vec::with_capacity(prefix.len() + n);
    v.extend_from_slice(prefix.as_bytes());
    v.extend(std::iter::repeat(b'x').take(n));
    v
}

// Try to read frames for up to `dur`. Print ACK credit or ERRORs if any.
// If no frame arrives before timeout, that’s fine.
async fn poll_for_server_frames(s: &mut TcpStream, dur: Duration) -> Result<()> {
    if let Ok(Ok(fr)) = timeout(dur, read_frame(s, DEFAULT_MAX_FRAME)).await {
        match fr.typ {
            FrameType::Ack => {
                let j: serde_json::Value = serde_json::from_slice(&fr.payload)?;
                let credit = j.get("credit").and_then(|v| v.as_u64()).unwrap_or(0);
                println!("got ACK credit={credit}");
            }
            FrameType::Error => {
                let j: serde_json::Value = serde_json::from_slice(&fr.payload)?;
                println!("server ERROR: {}", j);
            }
            other => {
                println!("unexpected server frame: {:?}", other);
            }
        }
    }
    Ok(())
}
