// crates/ron-app-sdk/examples/oap_echo_server.rs
use anyhow::{anyhow, Context, Result};
use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use ron_app_sdk::{
    Hello, OapCodec, OapFlags, OapFrame, OAP_VERSION, DEFAULT_MAX_DECOMPRESSED, DEFAULT_MAX_FRAME,
};
use std::{fs::File, io::BufReader, sync::Arc};
use tokio::{net::TcpListener, task};
use tokio_rustls::rustls;
use tokio_rustls::TlsAcceptor;
use tokio_util::codec::Framed;
use tracing::{error, info, warn};

fn load_tls() -> Result<rustls::ServerConfig> {
    use rustls_pemfile::{certs, pkcs8_private_keys};

    let cert_path = std::env::var("CERT_PEM").context("CERT_PEM not set")?;
    let key_path = std::env::var("KEY_PEM").context("KEY_PEM not set")?;

    // Certificates (already CertificateDer)
    let mut cert_reader = BufReader::new(File::open(cert_path)?);
    let cert_chain = certs(&mut cert_reader).collect::<std::result::Result<Vec<_>, _>>()?;

    // Private key (PKCS#8) → wrap into PrivateKeyDer::Pkcs8 to satisfy rustls 0.23
    let mut key_reader = BufReader::new(File::open(key_path)?);
    let mut keys =
        pkcs8_private_keys(&mut key_reader).collect::<std::result::Result<Vec<_>, _>>()?;
    if keys.is_empty() {
        return Err(anyhow!("no PKCS#8 key found"));
    }
    let key_der = rustls::pki_types::PrivateKeyDer::Pkcs8(keys.remove(0));

    let config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_chain, key_der)
        .map_err(|e| anyhow!("server cert error: {e}"))?;

    Ok(config)
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cfg = load_tls()?;
    let acceptor = TlsAcceptor::from(Arc::new(cfg));

    let listener = TcpListener::bind("127.0.0.1:9443").await?;
    info!("TLS echo server on 127.0.0.1:9443");

    loop {
        let (tcp, peer) = listener.accept().await?;
        let acceptor = acceptor.clone();
        task::spawn(async move {
            match handle_conn(acceptor, tcp).await {
                Ok(()) => info!("conn {peer:?} closed"),
                Err(e) => {
                    // Downgrade common disconnects to info
                    let msg = e.to_string().to_lowercase();
                    if msg.contains("close_notify") || msg.contains("unexpected eof") {
                        info!("conn {peer:?} disconnected: {e}");
                    } else {
                        error!("conn {peer:?} error: {e}");
                    }
                }
            }
        });
    }
}

async fn handle_conn(acceptor: TlsAcceptor, tcp: tokio::net::TcpStream) -> Result<()> {
    let tls = acceptor.accept(tcp).await?;
    let mut framed = Framed::new(
        tls,
        OapCodec::new(DEFAULT_MAX_FRAME, DEFAULT_MAX_DECOMPRESSED),
    );

    while let Some(next) = framed.next().await {
        let frame = match next {
            Ok(f) => f,
            Err(e) => {
                // Treat abrupt client closes as graceful
                let m = e.to_string().to_lowercase();
                if m.contains("close_notify") || m.contains("unexpected eof") {
                    warn!("client closed without TLS close_notify");
                    return Ok(());
                }
                return Err(anyhow!(e));
            }
        };

        // HELLO
        if frame.app_proto_id == 0 {
            let body = serde_json::to_vec(&Hello {
                server_version: "dev-echo-1.0.0".into(),
                max_frame: DEFAULT_MAX_FRAME as u64,
                max_inflight: 64,
                supported_flags: vec![
                    "EVENT".into(),
                    "ACK_REQ".into(),
                    "COMP".into(),
                    "APP_E2E".into(),
                ],
                oap_versions: vec![OAP_VERSION],
                transports: vec!["tcp+tls".into()],
            })?;
            let resp = OapFrame {
                ver: OAP_VERSION,
                flags: OapFlags::RESP | OapFlags::END,
                code: 0,
                app_proto_id: 0,
                tenant_id: frame.tenant_id,
                cap: Bytes::new(),
                corr_id: frame.corr_id,
                payload: Bytes::from(body),
            };
            framed.send(resp).await?;
            continue;
        }

        // Echo app: mirror payload
        let mut flags = OapFlags::RESP;
        if frame.flags.contains(OapFlags::START) {
            flags |= OapFlags::START;
        }
        if frame.flags.contains(OapFlags::END) {
            flags |= OapFlags::END;
        }

        let resp = OapFrame {
            ver: OAP_VERSION,
            flags,
            code: 0,
            app_proto_id: frame.app_proto_id,
            tenant_id: frame.tenant_id,
            cap: Bytes::new(),
            corr_id: frame.corr_id,
            payload: frame.payload.clone(),
        };
        framed.send(resp).await?;
    }

    Ok(())
}
