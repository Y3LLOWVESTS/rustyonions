// crates/gateway/src/oap.rs
#![forbid(unsafe_code)]
// Startup-only metric construction can use expect; never in hot paths.
#![allow(clippy::expect_used)]

use std::net::SocketAddr;
use std::sync::OnceLock;

use oap::{
    ack_frame, b3_of, decode_data_payload, read_frame, write_frame, FrameType, OapFrame,
    DEFAULT_MAX_FRAME, OapError,
};
use prometheus::{register, IntCounterVec, Opts};
use ron_kernel::{bus::Bus, KernelEvent};
use serde_json::Value as Json;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::Semaphore,
    task::JoinHandle,
};

// ---------- metrics (module-local, registered once) ----------

fn rejected_total_static() -> &'static IntCounterVec {
    static V: OnceLock<IntCounterVec> = OnceLock::new();
    V.get_or_init(|| {
        let v = IntCounterVec::new(
            Opts::new("oap_rejected_total", "OAP rejects by reason"),
            &["reason"],
        )
        .expect("IntCounterVec::new(oap_rejected_total)");
        // Ignore AlreadyRegistered errors.
        let _ = register(Box::new(v.clone()));
        v
    })
}

fn reject_inc(reason: &str) {
    rejected_total_static().with_label_values(&[reason]).inc();
}

// ---------- server ----------

/// Minimal OAP/1 server for RustyOnions Gateway:
/// - Expects HELLO → START(topic) → DATA... → END
/// - Verifies DATA header obj == b3(body)
/// - Emits kernel events on the Bus
/// - Sends ACK credits as simple flow control
/// - Applies basic backpressure (connection concurrency limit)
#[derive(Clone)]
pub struct OapServer {
    pub bus: Bus,
    pub ack_window_bytes: usize,
    pub max_frame: usize,
    pub concurrency_limit: usize,
}

impl OapServer {
    pub fn new(bus: Bus) -> Self {
        Self {
            bus,
            ack_window_bytes: 64 * 1024,
            max_frame: DEFAULT_MAX_FRAME,
            concurrency_limit: 1024,
        }
    }

    /// Bind and serve on `addr`. Returns (JoinHandle, bound_addr).
    pub async fn serve(self, addr: SocketAddr) -> std::io::Result<(JoinHandle<()>, SocketAddr)> {
        let listener = TcpListener::bind(addr).await?;
        let bound = listener.local_addr()?;

        // simple connection gate
        let sem = std::sync::Arc::new(Semaphore::new(self.concurrency_limit));

        let handle = tokio::spawn(async move {
            loop {
                let Ok((mut stream, peer)) = listener.accept().await else { break };

                // Try to acquire a slot; if none, send busy error immediately and close.
                match sem.clone().try_acquire_owned() {
                    Ok(permit) => {
                        let srv = self.clone();
                        tokio::spawn(async move {
                            if let Err(e) = handle_conn(stream, peer, srv.clone()).await {
                                let _ = srv.bus.publish(KernelEvent::ServiceCrashed {
                                    service: "oap-gateway".to_string(),
                                    reason: format!("peer={peer} error={e}"),
                                });
                            }
                            drop(permit); // release slot when task completes
                        });
                    }
                    Err(_) => {
                        // Best-effort write a BUSY error then drop the stream.
                        let payload = serde_json::to_vec(&serde_json::json!({
                            "code":"busy","msg":"server at capacity"
                        }))
                        .unwrap_or_default();
                        let err = OapFrame::new(FrameType::Error, payload);
                        let _ = write_frame(&mut stream, &err, DEFAULT_MAX_FRAME).await;
                        reject_inc("busy");
                        // stream drops here
                    }
                }
            }
        });

        Ok((handle, bound))
    }
}

async fn handle_conn(mut stream: TcpStream, peer: SocketAddr, srv: OapServer) -> anyhow::Result<()> {
    // HELLO
    let hello = match read_frame(&mut stream, srv.max_frame).await {
        Ok(fr) => fr,
        Err(OapError::PayloadTooLarge { .. }) => {
            send_proto_err(&mut stream, "too_large", "frame exceeds max_frame").await?;
            reject_inc("too_large");
            anyhow::bail!("peer={peer} too_large on HELLO");
        }
        Err(e) => return Err(e.into()),
    };
    ensure_frame(peer, &hello, FrameType::Hello)?;
    let _hello_json: Json = serde_json::from_slice(&hello.payload)?;

    // START (topic)
    let start = match read_frame(&mut stream, srv.max_frame).await {
        Ok(fr) => fr,
        Err(OapError::PayloadTooLarge { .. }) => {
            send_proto_err(&mut stream, "too_large", "frame exceeds max_frame").await?;
            reject_inc("too_large");
            anyhow::bail!("peer={peer} too_large on START");
        }
        Err(e) => return Err(e.into()),
    };
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
        let fr = match read_frame(&mut stream, srv.max_frame).await {
            Ok(fr) => fr,
            Err(OapError::PayloadTooLarge { .. }) => {
                send_proto_err(&mut stream, "too_large", "frame exceeds max_frame").await?;
                reject_inc("too_large");
                anyhow::bail!("peer={peer} too_large during DATA/END");
            }
            Err(e) => return Err(e.into()),
        };

        match fr.typ {
            FrameType::Data => {
                let (hdr, body) = decode_data_payload(&fr.payload)?;
                let obj = hdr.get("obj").and_then(|v| v.as_str()).unwrap_or("");
                let want = b3_of(&body);
                if obj != want {
                    send_proto_err(&mut stream, "proto", "obj digest mismatch").await?;
                    reject_inc("proto");
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
                send_proto_err(
                    &mut stream,
                    "proto",
                    &format!("unexpected frame: {other:?}"),
                )
                .await?;
                reject_inc("proto");
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

async fn send_proto_err(stream: &mut TcpStream, code: &str, msg: &str) -> anyhow::Result<()> {
    let payload = serde_json::to_vec(&serde_json::json!({ "code": code, "msg": msg }))?;
    let err = OapFrame::new(FrameType::Error, payload);
    write_frame(stream, &err, DEFAULT_MAX_FRAME).await?;
    Ok(())
}

fn ensure_frame(peer: SocketAddr, fr: &OapFrame, want: FrameType) -> anyhow::Result<()> {
    if fr.typ != want {
        let have = format!("{:?}", fr.typ);
        anyhow::bail!("peer={peer} expected {want:?}, got {have}");
    }
    Ok(())
}
