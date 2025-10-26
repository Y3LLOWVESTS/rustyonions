#![cfg(feature = "tls")]

use ron_kernel::{Bus, HealthState};
use ron_transport::{
    config::TransportConfig, metrics::TransportMetrics, spawn_transport, types::TransportEvent,
    TlsServerConfig,
};
use rustls_pemfile::{certs, pkcs8_private_keys, rsa_private_keys};
use std::{fs::File, io::BufReader, path::Path, sync::Arc};
use tokio::net::TcpStream;
use tokio_rustls::rustls::{
    pki_types::{CertificateDer, PrivatePkcs1KeyDer, PrivatePkcs8KeyDer, ServerName},
    ClientConfig, RootCertStore,
};
use tokio_rustls::TlsConnector;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tls_accepts_handshake() -> anyhow::Result<()> {
    let cert_path = "crates/ron-transport/scripts/local/certs/cert.pem";
    let key_path = "crates/ron-transport/scripts/local/certs/key.pem";
    if !(Path::new(cert_path).exists() && Path::new(key_path).exists()) {
        eprintln!("(skipping tls_accepts_handshake: local certs not found)");
        return Ok(());
    }

    let server_cfg = Arc::new(load_rustls_server(cert_path, key_path)?);

    let mut cfg = TransportConfig::default();
    cfg.name = "tls-test";
    let metrics = TransportMetrics::new("ron");
    let health = Arc::new(HealthState::new());
    let bus: Bus<TransportEvent> = Bus::new();

    let (_jh, addr) = spawn_transport(cfg, metrics, health, bus, Some(server_cfg)).await?;

    let mut roots = RootCertStore::empty();
    let cert_file = File::open(Path::new(cert_path))?;
    let mut cert_rd = BufReader::new(cert_file);
    let certs: Vec<CertificateDer<'static>> = certs(&mut cert_rd).collect::<Result<_, _>>()?;
    for c in certs.into_iter() {
        roots.add(c)?;
    }

    let client = ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    let connector = TlsConnector::from(Arc::new(client));
    let tcp = TcpStream::connect(addr).await?;
    let _tls = connector
        .connect(ServerName::try_from("localhost")?, tcp)
        .await?;
    Ok(())
}

fn load_rustls_server(cert_path: &str, key_path: &str) -> anyhow::Result<TlsServerConfig> {
    use tokio_rustls::rustls::{
        pki_types::{CertificateDer, PrivateKeyDer},
        ServerConfig,
    };

    let cert_file = File::open(Path::new(cert_path))?;
    let mut cert_rd = BufReader::new(cert_file);
    let certs: Vec<CertificateDer<'static>> = certs(&mut cert_rd).collect::<Result<_, _>>()?;

    let key: PrivateKeyDer<'static> = {
        let key_file = File::open(Path::new(key_path))?;
        let mut key_rd = BufReader::new(key_file);
        let mut pkcs8: Vec<PrivateKeyDer<'static>> = pkcs8_private_keys(&mut key_rd)
            .map(|res: std::io::Result<PrivatePkcs8KeyDer<'static>>| res.map(Into::into))
            .collect::<Result<_, _>>()?;
        if let Some(k) = pkcs8.pop() {
            k
        } else {
            let key_file = File::open(Path::new(key_path))?;
            let mut key_rd = BufReader::new(key_file);
            let mut rsa: Vec<PrivateKeyDer<'static>> = rsa_private_keys(&mut key_rd)
                .map(|res: std::io::Result<PrivatePkcs1KeyDer<'static>>| res.map(Into::into))
                .collect::<Result<_, _>>()?;
            rsa.pop()
                .ok_or_else(|| anyhow::anyhow!("no private key found in {}", key_path))?
        }
    };

    let cfg = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)?;
    Ok(cfg)
}
