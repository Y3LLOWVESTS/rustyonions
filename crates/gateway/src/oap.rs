#![forbid(unsafe_code)]

use std::net::SocketAddr;

use oap::{
    ack_frame, b3_of, decode_data_payload, read_frame, write_frame, FrameType, OapFrame,
    DEFAULT_MAX_FRAME,
};
use ron_kernel::{bus::Bus, KernelEvent};
use serde_json::Value as Json;
use tokio::{
    net::{TcpListener, TcpStream},
    task::JoinHandle,
};

/// Minimal OAP/1 server for RustyOnions Gateway:
/// - Expects HELLO → START(topic) → DATA... → END
/// - Verifies DATA header obj == b3(body)
/// - Emits kernel events on the Bus
/// - Sends ACK credits as simple flow control
#[derive(Clone)]
pub struct OapServer {
    pub bus: Bus,
    pub ack_window_bytes: usize,
    pub max_frame: usize,
}

impl OapServer {
    pub fn new(bus: Bus) -> Self {
        Self {
            bus,
            ack_window_bytes: 64 * 1024,
            max_frame: DEFAULT_MAX_FRAME,
        }
    }

    /// Bind and serve on `addr`. Returns (JoinHandle, bound_addr).
    pub async fn serve(self, addr: SocketAddr) -> std::io::Result<(JoinHandle<()>, SocketAddr)> {
        let listener = TcpListener::bind(addr).await?;
        let bound = listener.local_addr()?;

        let handle = tokio::spawn(async move {
            loop {
                let Ok((stream, peer)) = listener.accept().await else { break };
                let srv = self.clone();
                tokio::spawn(async move {
                    // NOTE: pass a CLONE into handle_conn so we can still use `srv` after await.
                    if let Err(e) = handle_conn(stream, peer, srv.clone()).await {
                        let _ = srv.bus.publish(KernelEvent::ServiceCrashed {
                            service: "oap-gateway".to_string(),
                            reason: format!("peer={peer} error={e}"),
                        });
                    }
                });
            }
        });

        Ok((handle, bound))
    }
}

async fn handle_conn(mut stream: TcpStream, peer: SocketAddr, srv: OapServer) -> anyhow::Result<()> {
    // HELLO
    let hello = read_frame(&mut stream, srv.max_frame).await?;
    ensure_frame(peer, &hello, FrameType::Hello)?;
    let _hello_json: Json = serde_json::from_slice(&hello.payload)?;

    // START (topic)
    let start = read_frame(&mut stream, srv.max_frame).await?;
    ensure_frame(peer, &start, FrameType::Start)?;
    let start_json: Json = serde_json::from_slice(&start.payload)?;
    let topic = start_json
        .get("topic")
        .and_then(|v| v.as_str())
        .unwrap_or("<none>")
        .to_string();

    // Mark started
    let _ = srv.bus.publish(KernelEvent::Health {
        service: format!("oap-start:{topic}"),
        ok: true,
    });

    // DATA loop with simple crediting
    let mut credited: usize = srv.ack_window_bytes;
    let mut consumed_since_ack: usize = 0usize;

    loop {
        let fr = read_frame(&mut stream, srv.max_frame).await?;
        match fr.typ {
            FrameType::Data => {
                let (hdr, body) = decode_data_payload(&fr.payload)?;
                let obj = hdr.get("obj").and_then(|v| v.as_str()).unwrap_or("");
                let want = b3_of(&body);
                if obj != want {
                    let payload = serde_json::to_vec(&serde_json::json!({
                        "code":"proto", "msg":"obj digest mismatch"
                    }))?;
                    let err = OapFrame::new(FrameType::Error, payload);
                    write_frame(&mut stream, &err, srv.max_frame).await?;
                    anyhow::bail!("DATA obj mismatch: got={obj}, want={want}");
                }

                // Emit a lightweight event (demo-friendly visibility)
                let _ = srv.bus.publish(KernelEvent::ConfigUpdated {
                    version: body.len() as u64,
                });

                consumed_since_ack += body.len();
                if consumed_since_ack >= (srv.ack_window_bytes / 2) {
                    credited += srv.ack_window_bytes;
                    let ack = ack_frame(credited as u64);
                    write_frame(&mut stream, &ack, srv.max_frame).await?;
                    consumed_since_ack = 0;
                }
            }
            FrameType::End => break,
            other => {
                let payload = serde_json::to_vec(&serde_json::json!({
                    "code":"proto", "msg": format!("unexpected frame: {other:?}")
                }))?;
                let err = OapFrame::new(FrameType::Error, payload);
                write_frame(&mut stream, &err, srv.max_frame).await?;
                anyhow::bail!("unexpected frame type: {other:?}");
            }
        }
    }

    // Mark finished
    let _ = srv.bus.publish(KernelEvent::Health {
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
