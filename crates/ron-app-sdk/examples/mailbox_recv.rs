//! Receive up to N messages from a topic and ACK them.
//! Usage:
//!   RON_ADDR=127.0.0.1:9443 RON_SNI=localhost RON_EXTRA_CA=testing/tls/ca.crt \
//!   TOPIC=chat MAX=10 \
//!   cargo run -p ron-app-sdk --example mailbox_recv

use anyhow::{anyhow, Result};
use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use ron_app_sdk::{
    OapCodec, OapFlags, OapFrame, DEFAULT_MAX_DECOMPRESSED, DEFAULT_MAX_FRAME, OAP_VERSION,
};
use serde::Deserialize;
use std::net::ToSocketAddrs;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_rustls::rustls::pki_types::ServerName;
use tokio_rustls::{rustls, TlsConnector};
use tokio_util::codec::Framed;

const MAILBOX_APP_PROTO_ID: u16 = 0x0201;

#[allow(dead_code)]
#[derive(Deserialize)]
struct RecvMsg {
    msg_id: String,
    topic: String,
    text: String,
}

#[derive(Deserialize)]
struct RecvResp {
    messages: Vec<RecvMsg>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct AckResp {
    ok: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let addr = std::env::var("RON_ADDR").unwrap_or_else(|_| "127.0.0.1:9443".to_string());
    let sni = std::env::var("RON_SNI").unwrap_or_else(|_| "localhost".to_string());
    let extra = std::env::var("RON_EXTRA_CA").ok();

    let topic = std::env::var("TOPIC").unwrap_or_else(|_| "chat".to_string());
    let max: usize = std::env::var("MAX")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);

    let tls = connect(&addr, &sni, extra.as_deref()).await?;
    let mut framed = Framed::new(
        tls,
        OapCodec::new(DEFAULT_MAX_FRAME, DEFAULT_MAX_DECOMPRESSED),
    );

    // RECV
    let payload = serde_json::json!({ "op": "recv", "topic": topic, "max": max });
    let req = OapFrame {
        ver: OAP_VERSION,
        flags: OapFlags::REQ | OapFlags::START,
        code: 0,
        app_proto_id: MAILBOX_APP_PROTO_ID,
        tenant_id: 0,
        cap: Bytes::new(),
        corr_id: 2,
        payload: Bytes::from(serde_json::to_vec(&payload)?),
    };
    framed.send(req).await?;

    let resp = framed
        .next()
        .await
        .ok_or_else(|| anyhow!("no recv response"))??;
    if !resp.flags.contains(OapFlags::RESP) {
        return Err(anyhow!("expected RESP"));
    }
    if resp.code != 0 {
        return Err(anyhow!(String::from_utf8_lossy(&resp.payload).to_string()));
    }

    let r: RecvResp = serde_json::from_slice(&resp.payload)?;
    if r.messages.is_empty() {
        println!("no messages");
        return Ok(());
    }

    println!("received {} message(s):", r.messages.len());
    for m in &r.messages {
        // We only display msg_id and text to keep output concise.
        println!("- [{}] {}", m.msg_id, m.text);
    }

    // ACK each one
    for (i, m) in r.messages.iter().enumerate() {
        let payload = serde_json::json!({ "op": "ack", "msg_id": m.msg_id });
        let req = OapFrame {
            ver: OAP_VERSION,
            flags: OapFlags::REQ | OapFlags::START,
            code: 0,
            app_proto_id: MAILBOX_APP_PROTO_ID,
            tenant_id: 0,
            cap: Bytes::new(),
            corr_id: (100 + i) as u64,
            payload: Bytes::from(serde_json::to_vec(&payload)?),
        };
        framed.send(req).await?;

        let resp = framed
            .next()
            .await
            .ok_or_else(|| anyhow!("no ack response"))??;
        if resp.code != 0 {
            return Err(anyhow!(String::from_utf8_lossy(&resp.payload).to_string()));
        }
        let _ok: AckResp = serde_json::from_slice(&resp.payload)?;
    }

    println!("acked all");
    Ok(())
}

async fn connect(
    addr: &str,
    server_name: &str,
    extra_ca: Option<&str>,
) -> Result<tokio_rustls::client::TlsStream<TcpStream>> {
    use rustls_pemfile::certs;
    use std::{fs::File, io::BufReader};
    use tokio_rustls::rustls::RootCertStore;

    let sockaddr = addr
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| anyhow!("addr resolve failed"))?;
    let tcp = TcpStream::connect(sockaddr).await?;
    tcp.set_nodelay(true)?;

    let mut roots = RootCertStore::empty();
    for cert in
        rustls_native_certs::load_native_certs().map_err(|e| anyhow!("native certs: {e}"))?
    {
        roots
            .add(cert)
            .map_err(|_| anyhow!("failed to add native root"))?;
    }
    if let Some(path) = extra_ca {
        let mut rd = BufReader::new(File::open(path)?);
        for der in certs(&mut rd).collect::<std::result::Result<Vec<_>, _>>()? {
            // Clippy: avoid useless conversion; `der` is already CertificateDer
            roots
                .add(der)
                .map_err(|_| anyhow!("failed to add extra ca"))?;
        }
    }

    let config = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    let connector = TlsConnector::from(Arc::new(config));
    let sni: ServerName =
        ServerName::try_from(server_name.to_string()).map_err(|_| anyhow!("invalid sni"))?;
    Ok(connector.connect(sni, tcp).await?)
}
