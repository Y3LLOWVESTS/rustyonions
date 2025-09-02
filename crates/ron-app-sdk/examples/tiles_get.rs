//! Stream a tile via OAP/1 (app_proto_id 0x0301) and save to disk.
//! Usage:
//!   RON_ADDR=127.0.0.1:9443 RON_SNI=localhost RON_EXTRA_CA=testing/tls/ca.crt \
//!   TILE_PATH=/tiles/12/654/1583.webp OUT=./out.webp \
//!   cargo run -p ron-app-sdk --example tiles_get

use anyhow::{anyhow, Context, Result};
use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use ron_app_sdk::{
    OapCodec, OapFlags, OapFrame, OAP_VERSION,
    DEFAULT_MAX_DECOMPRESSED, DEFAULT_MAX_FRAME,
};
use std::{fs::File, io::Write, net::ToSocketAddrs, sync::Arc};
use tokio::net::TcpStream;
use tokio_rustls::{rustls, TlsConnector};
use tokio_rustls::rustls::pki_types::ServerName;
use tokio_util::codec::Framed;

const TILE_APP_PROTO_ID: u16 = 0x0301;

#[tokio::main]
async fn main() -> Result<()> {
    let addr = std::env::var("RON_ADDR").unwrap_or_else(|_| "127.0.0.1:9443".to_string());
    let sni  = std::env::var("RON_SNI").unwrap_or_else(|_| "localhost".to_string());
    let extra = std::env::var("RON_EXTRA_CA").ok();
    let tile_path = std::env::var("TILE_PATH").unwrap_or_else(|_| "/tiles/12/654/1583.webp".to_string());
    let out_path  = std::env::var("OUT").unwrap_or_else(|_| "out.webp".to_string());

    let tls = connect(&addr, &sni, extra.as_deref()).await?;
    let mut framed = Framed::new(tls, OapCodec::new(DEFAULT_MAX_FRAME, DEFAULT_MAX_DECOMPRESSED));

    // Request: REQ|START for streaming GET
    let payload = Bytes::from(format!(r#"{{"op":"get","path":"{}"}}"#, tile_path));
    let req = OapFrame {
        ver: OAP_VERSION,
        flags: OapFlags::REQ | OapFlags::START,
        code: 0,
        app_proto_id: TILE_APP_PROTO_ID,
        tenant_id: 0,
        cap: Bytes::new(),
        corr_id: 42,
        payload,
    };
    framed.send(req).await?;

    let mut out = File::create(&out_path).with_context(|| format!("create {}", out_path))?;
    let mut total = 0usize;

    while let Some(msg) = framed.next().await {
        let frame = msg?;
        if frame.app_proto_id != TILE_APP_PROTO_ID {
            return Err(anyhow!("unexpected app id {}", frame.app_proto_id));
        }
        if frame.flags.contains(OapFlags::RESP) {
            out.write_all(&frame.payload)?;
            total += frame.payload.len();
            if frame.flags.contains(OapFlags::END) {
                break;
            }
        } else {
            return Err(anyhow!("expected RESP"));
        }
    }

    println!("saved {} bytes to {}", total, out_path);
    Ok(())
}

async fn connect(addr: &str, server_name: &str, extra_ca: Option<&str>)
    -> Result<tokio_rustls::client::TlsStream<TcpStream>>
{
    use rustls_pemfile::certs;
    use std::{fs::File, io::BufReader};
    use tokio_rustls::rustls::RootCertStore;

    let sockaddr = addr
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| anyhow!("addr resolve failed"))?;

    let tcp = TcpStream::connect(sockaddr).await?;
    tcp.set_nodelay(true)?;

    // Root store = native + optional extra CA (our dev CA)
    let mut roots = RootCertStore::empty();
    for cert in rustls_native_certs::load_native_certs()
        .map_err(|e| anyhow!("native certs: {e}"))?
    {
        roots.add(cert).map_err(|_| anyhow!("failed to add native root"))?;
    }
    if let Some(path) = extra_ca {
        let mut rd = BufReader::new(File::open(path)?);
        for der in certs(&mut rd).collect::<std::result::Result<Vec<_>, _>>()? {
            roots.add(rustls::pki_types::CertificateDer::from(der))
                .map_err(|_| anyhow!("failed to add extra ca"))?;
        }
    }

    let config = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();

    let connector = TlsConnector::from(Arc::new(config));
    let sni: ServerName = ServerName::try_from(server_name.to_string())
        .map_err(|_| anyhow!("invalid sni"))?;
    let tls = connector.connect(sni, tcp).await?;
    Ok(tls)
}
