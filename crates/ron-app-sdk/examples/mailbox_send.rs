//! Send a chat message to a topic via Mailbox (app_proto_id 0x0201).
//! Usage:
//!   RON_ADDR=127.0.0.1:9443 RON_SNI=localhost RON_EXTRA_CA=testing/tls/ca.crt \
//!   TOPIC=chat TEXT="hello vest" IDEMPOTENCY_KEY=abc123 \
//!   cargo run -p ron-app-sdk --example mailbox_send

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

#[derive(Deserialize)]
struct SendResp {
    msg_id: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let addr = std::env::var("RON_ADDR").unwrap_or_else(|_| "127.0.0.1:9443".to_string());
    let sni = std::env::var("RON_SNI").unwrap_or_else(|_| "localhost".to_string());
    let extra = std::env::var("RON_EXTRA_CA").ok();

    let topic = std::env::var("TOPIC").unwrap_or_else(|_| "chat".to_string());
    let text = std::env::var("TEXT").unwrap_or_else(|_| "hello".to_string());
    let idem = std::env::var("IDEMPOTENCY_KEY").ok();

    let tls = connect(&addr, &sni, extra.as_deref()).await?;
    let mut framed = Framed::new(
        tls,
        OapCodec::new(DEFAULT_MAX_FRAME, DEFAULT_MAX_DECOMPRESSED),
    );

    let mut payload = serde_json::json!({"op":"send","topic":topic,"text":text});
    if let Some(k) = idem {
        payload["idempotency_key"] = serde_json::Value::String(k);
    }

    let req = OapFrame {
        ver: OAP_VERSION,
        flags: OapFlags::REQ | OapFlags::START,
        code: 0,
        app_proto_id: MAILBOX_APP_PROTO_ID,
        tenant_id: 0,
        cap: Bytes::new(),
        corr_id: 1,
        payload: Bytes::from(serde_json::to_vec(&payload)?),
    };
    framed.send(req).await?;

    let resp = framed
        .next()
        .await
        .ok_or_else(|| anyhow!("no response"))??;
    if !resp.flags.contains(OapFlags::RESP) {
        return Err(anyhow!("expected RESP"));
    }
    if resp.code != 0 {
        return Err(anyhow!(String::from_utf8_lossy(&resp.payload).to_string()));
    }

    let s: SendResp = serde_json::from_slice(&resp.payload)?;
    println!("msg_id: {}", s.msg_id);

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
