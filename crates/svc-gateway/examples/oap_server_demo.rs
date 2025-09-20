#![forbid(unsafe_code)]

use std::{net::SocketAddr, sync::Arc};

use oap::{
    ack_frame, b3_of, decode_data_payload, read_frame, write_frame, FrameType, OapFrame,
    DEFAULT_MAX_FRAME,
};
use ron_kernel::{bus::Bus, KernelEvent};
use serde_json::Value as Json;
use tokio::net::{TcpListener, TcpStream};

const LISTEN_ADDR: &str = "127.0.0.1:9444";
const ACK_WINDOW_BYTES: usize = 64 * 1024; // simple credit window

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    // Kernel bus (demo-local). Real app would share this globally.
    let bus = Bus::new(128);

    let listener = TcpListener::bind(LISTEN_ADDR).await?;
    let bound = listener.local_addr()?;
    eprintln!("OAP/1 demo gateway listening on {bound}");

    // Mark gateway healthy for metrics/health checks (if used elsewhere)
    let m = Arc::new(ron_kernel::Metrics::new());
    m.health().set("gateway_demo", true);

    loop {
        let (stream, peer) = listener.accept().await?;
        let bus_for_task = bus.clone();
        tokio::spawn(async move {
            // Pass a CLONE of the bus into the handler so we can still use bus_for_task here.
            if let Err(e) = handle_conn(stream, peer, bus_for_task.clone()).await {
                // Emit a crash-style event to the kernel bus
                let _ = bus_for_task.publish(KernelEvent::ServiceCrashed {
                    service: "oap-gateway".to_string(),
                    reason: format!("peer={peer} error={e}"),
                });
                eprintln!("connection error from {peer}: {e:#}");
            }
        });
    }
}

async fn handle_conn(mut stream: TcpStream, peer: SocketAddr, bus: Bus) -> anyhow::Result<()> {
    // HELLO
    let hello = read_frame(&mut stream, DEFAULT_MAX_FRAME).await?;
    ensure_frame(peer, &hello, FrameType::Hello)?;
    let hello_json: Json = serde_json::from_slice(&hello.payload)?;
    eprintln!("HELLO from {peer}: {hello_json}");

    // START (topic)
    let start = read_frame(&mut stream, DEFAULT_MAX_FRAME).await?;
    ensure_frame(peer, &start, FrameType::Start)?;
    let start_json: Json = serde_json::from_slice(&start.payload)?;
    let topic = start_json
        .get("topic")
        .and_then(|v| v.as_str())
        .unwrap_or("<none>");
    eprintln!("START topic={topic}");

    // Publish an event that a stream started (reusing a simple event variant)
    let _ = bus.publish(KernelEvent::Health {
        service: format!("oap-start:{topic}"),
        ok: true,
    });

    // DATA loop
    let mut credited: usize = ACK_WINDOW_BYTES;
    let mut consumed_since_ack: usize = 0usize;
    loop {
        let fr = read_frame(&mut stream, DEFAULT_MAX_FRAME).await?;
        match fr.typ {
            FrameType::Data => {
                let (hdr, body) = decode_data_payload(&fr.payload)?;
                let obj = hdr.get("obj").and_then(|v| v.as_str()).unwrap_or("");
                let want = b3_of(&body);
                if obj != want {
                    // Protocol error: wrong obj hash; send Error and stop.
                    let payload = serde_json::to_vec(
                        &serde_json::json!({ "code":"proto", "msg":"obj digest mismatch" }),
                    )?;
                    let err = OapFrame::new(FrameType::Error, payload);
                    write_frame(&mut stream, &err, DEFAULT_MAX_FRAME).await?;
                    anyhow::bail!("DATA obj mismatch: got={obj}, want={want}");
                }

                // (Demo) publish a light event per DATA chunk
                let _ = bus.publish(KernelEvent::ConfigUpdated {
                    version: body.len() as u64, // piggyback bytes as "version" for demo visibility
                });

                consumed_since_ack += body.len();
                if consumed_since_ack >= (ACK_WINDOW_BYTES / 2) {
                    // Grant more credit
                    credited += ACK_WINDOW_BYTES;
                    let ack = ack_frame(credited as u64);
                    write_frame(&mut stream, &ack, DEFAULT_MAX_FRAME).await?;
                    consumed_since_ack = 0;
                }
            }
            FrameType::End => {
                eprintln!("END from {peer}");
                break;
            }
            other => {
                let payload = serde_json::to_vec(&serde_json::json!({
                    "code":"proto",
                    "msg": format!("unexpected frame: {other:?}")
                }))?;
                let err = OapFrame::new(FrameType::Error, payload);
                write_frame(&mut stream, &err, DEFAULT_MAX_FRAME).await?;
                anyhow::bail!("unexpected frame type: {other:?}");
            }
        }
    }

    // Mark the topic "done" (flip health to false just to show state change).
    let _ = bus.publish(KernelEvent::Health {
        service: format!("oap-start:{topic}"),
        ok: false,
    });

    Ok(())
}

fn ensure_frame(peer: SocketAddr, fr: &OapFrame, want: FrameType) -> anyhow::Result<()> {
    if fr.typ != want {
        let have = format!("{:?}", fr.typ);
        anyhow::bail!("peer={peer} expected {want:?}, got {have}");
    }
    Ok(())
}
