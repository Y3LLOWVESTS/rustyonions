//! TLS listener using the ron-transport library.
//! Loads cert+key, builds rustls::ServerConfig, and passes it to spawn_transport.

#![cfg(feature = "tls")]

use ron_kernel::{Bus, HealthState};
use ron_transport::{
    config::TransportConfig,
    metrics::TransportMetrics,
    spawn_transport,
    types::TransportEvent,
    TlsServerConfig, // re-export
};
use std::{fs::File, io::BufReader, path::Path, sync::Arc};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cert_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "crates/ron-transport/scripts/local/certs/cert.pem".into());
    let key_path = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "crates/ron-transport/scripts/local/certs/key.pem".into());

    let tls_cfg = Arc::new(load_rustls_server(&cert_path, &key_path)?);

    let mut cfg = TransportConfig::default();
    cfg.name = "tls";
    let metrics = TransportMetrics::new("ron");
    let health = Arc::new(HealthState::new());
    let bus: Bus<TransportEvent> = Bus::new();

    let (_jh, addr) = spawn_transport(cfg, metrics, health, bus, Some(tls_cfg)).await?;
    println!("tls-transport listening on {}", addr);
    tokio::signal::ctrl_c().await.ok();
    Ok(())
}

#[cfg(feature = "tls")]
fn load_rustls_server(cert_path: &str, key_path: &str) -> anyhow::Result<TlsServerConfig> {
    use rustls_pemfile::{certs, pkcs8_private_keys, rsa_private_keys};
    use tokio_rustls::rustls::{
        pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs1KeyDer, PrivatePkcs8KeyDer},
        ServerConfig,
    };

    // Load cert chain
    let cert_file = File::open(Path::new(cert_path))?;
    let mut cert_rd = BufReader::new(cert_file);
    let certs: Vec<CertificateDer<'static>> = certs(&mut cert_rd).collect::<Result<_, _>>()?;

    // Load first available private key (PKCS#8 preferred, fall back to PKCS#1/RSA)
    let key: PrivateKeyDer<'static> = {
        // Try PKCS#8
        let key_file = File::open(Path::new(key_path))?;
        let mut key_rd = BufReader::new(key_file);
        let mut pkcs8: Vec<PrivateKeyDer<'static>> = pkcs8_private_keys(&mut key_rd)
            .map(|res: std::io::Result<PrivatePkcs8KeyDer<'static>>| res.map(Into::into))
            .collect::<Result<_, _>>()?;
        if let Some(k) = pkcs8.pop() {
            k
        } else {
            // Try PKCS#1 (RSA)
            let key_file = File::open(Path::new(key_path))?;
            let mut key_rd = BufReader::new(key_file);
            let mut rsa: Vec<PrivateKeyDer<'static>> = rsa_private_keys(&mut key_rd)
                .map(|res: std::io::Result<PrivatePkcs1KeyDer<'static>>| res.map(Into::into))
                .collect::<Result<_, _>>()?;
            rsa.pop()
                .ok_or_else(|| anyhow::anyhow!("no private key found in {}", key_path))?
        }
    };

    // rustls 0.22 API
    let cfg = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)?;

    Ok(cfg)
}
