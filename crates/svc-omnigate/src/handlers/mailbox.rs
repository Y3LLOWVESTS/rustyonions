#![forbid(unsafe_code)]

use anyhow::{anyhow, Context, Result};
use bytes::Bytes;
use futures_util::SinkExt;
use ron_app_sdk::{OapCodec, OapFlags, OapFrame, OAP_VERSION};
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio_rustls::server::TlsStream;
use tokio_util::codec::Framed;

use crate::mailbox::{Mailbox, MAILBOX_APP_PROTO_ID};
use crate::metrics::Metrics;

#[derive(Deserialize)]
#[serde(rename_all = "lowercase", tag = "op")]
enum Req {
    Send { topic: String, text: String, #[serde(default)] idempotency_key: Option<String> },
    Recv { topic: String, #[serde(default = "default_max")] max: usize },
    Ack  { msg_id: String },
}

fn default_max() -> usize { 10 }

#[derive(Serialize)]
struct SendResp { msg_id: String }

#[derive(Serialize)]
struct RecvMsg { msg_id: String, topic: String, text: String }

#[derive(Serialize)]
struct RecvResp { messages: Vec<RecvMsg> }

#[derive(Serialize)]
struct AckResp { ok: bool }

pub async fn handle_mailbox(
    framed: &mut Framed<TlsStream<TcpStream>, OapCodec>,
    mailbox: &Mailbox,
    req: &OapFrame,
    _metrics: &Metrics, // currently unused; prefix with underscore to silence warning
) -> Result<()> {
    let parsed: Req = serde_json::from_slice(&req.payload).context("invalid JSON")?;

    match parsed {
        Req::Send { topic, text, idempotency_key } => {
            let id = mailbox.send(&topic, Bytes::from(text.into_bytes()), idempotency_key).await?;
            let body = serde_json::to_vec(&SendResp { msg_id: id })?;
            let resp = OapFrame {
                ver: OAP_VERSION,
                flags: OapFlags::RESP | OapFlags::END,
                code: 0,
                app_proto_id: MAILBOX_APP_PROTO_ID,
                tenant_id: req.tenant_id,
                cap: Bytes::new(),
                corr_id: req.corr_id,
                payload: Bytes::from(body),
            };
            framed.send(resp).await?;
            Ok(())
        }
        Req::Recv { topic, max } => {
            if max == 0 { return Err(anyhow!("bad_request")); }
            let msgs = mailbox.recv(&topic, max.min(100)).await?;
            let out: Vec<RecvMsg> = msgs.into_iter().map(|(id, body)| {
                let text = String::from_utf8_lossy(&body).to_string();
                RecvMsg { msg_id: id, topic: topic.clone(), text }
            }).collect();

            let body = serde_json::to_vec(&RecvResp { messages: out })?;
            let resp = OapFrame {
                ver: OAP_VERSION,
                flags: OapFlags::RESP | OapFlags::END,
                code: 0,
                app_proto_id: MAILBOX_APP_PROTO_ID,
                tenant_id: req.tenant_id,
                cap: Bytes::new(),
                corr_id: req.corr_id,
                payload: Bytes::from(body),
            };
            framed.send(resp).await?;
            Ok(())
        }
        Req::Ack { msg_id } => {
            mailbox.ack(&msg_id).await.map_err(|e| {
                if e.to_string().contains("not_found") { anyhow!("404 not_found") } else { e }
            })?;
            let body = serde_json::to_vec(&AckResp { ok: true })?;
            let resp = OapFrame {
                ver: OAP_VERSION,
                flags: OapFlags::RESP | OapFlags::END,
                code: 0,
                app_proto_id: MAILBOX_APP_PROTO_ID,
                tenant_id: req.tenant_id,
                cap: Bytes::new(),
                corr_id: req.corr_id,
                payload: Bytes::from(body),
            };
            framed.send(resp).await?;
            Ok(())
        }
    }
}
