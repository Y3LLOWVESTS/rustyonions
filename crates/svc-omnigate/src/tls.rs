#![forbid(unsafe_code)]

use anyhow::{anyhow, Context, Result};
use std::{fs::File, io::BufReader, sync::Arc};
use tokio_rustls::{rustls, TlsAcceptor};
use rustls_pemfile::{certs, pkcs8_private_keys, rsa_private_keys};

/// Load TLS acceptor from CERT_PEM and KEY_PEM.
/// This also **installs the Rustls crypto provider** (aws-lc-rs) explicitly,
/// which avoids runtime panics when the automatic provider selection is not in effect.
pub fn load_tls() -> Result<TlsAcceptor> {
    // Ensure a single crypto backend is installed (Rustls 0.23+).
    // NOTE: use {:?} because the error type is not Display.
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .map_err(|e| anyhow!("failed to install rustls aws-lc-rs provider: {:?}", e))?;

    let cert_path = std::env::var("CERT_PEM").context("CERT_PEM not set")?;
    let key_path  = std::env::var("KEY_PEM").context("KEY_PEM not set")?;

    // ---- load certificate chain
    let mut rd = BufReader::new(
        File::open(&cert_path).with_context(|| format!("open cert {}", cert_path))?
    );
    let chain = certs(&mut rd)
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("parse certificate(s)")?
        .into_iter()
        .map(rustls::pki_types::CertificateDer::from)
        .collect::<Vec<_>>();
    if chain.is_empty() {
        return Err(anyhow!("no certificates found in {}", cert_path));
    }

    // ---- load private key (prefer PKCS#8; fall back to RSA/PKCS#1)
    // Read PKCS#8 keys (owned items via collect); pick the first by moving it out.
    let pkcs8 = {
        let mut kr = BufReader::new(
            File::open(&key_path).with_context(|| format!("open key {}", key_path))?
        );
        pkcs8_private_keys(&mut kr)
            .collect::<std::result::Result<Vec<_>, _>>()
            .unwrap_or_default()
    };

    // Build a PrivateKeyDer in either branch so the if-expression has one concrete type.
    let priv_key: rustls::pki_types::PrivateKeyDer = if let Some(p8) = pkcs8.into_iter().next() {
        // p8 is moved (no clone on DER wrapper needed)
        rustls::pki_types::PrivateKeyDer::from(p8)
    } else {
        // Fallback: RSA keys
        let mut kr = BufReader::new(
            File::open(&key_path).with_context(|| format!("open key {}", key_path))?
        );
        let rsa = rsa_private_keys(&mut kr)
            .collect::<std::result::Result<Vec<_>, _>>()
            .context("parse RSA private key")?;

        // Move the first RSA key out (avoid .cloned() which requires Clone)
        let k = rsa.into_iter()
            .next()
            .ok_or_else(|| anyhow!("no private key found in {}", key_path))?;

        rustls::pki_types::PrivateKeyDer::from(k)
    };

    // ---- build server config
    let server_config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(chain, priv_key)
        .context("build rustls server config")?;

    Ok(TlsAcceptor::from(Arc::new(server_config)))
}
