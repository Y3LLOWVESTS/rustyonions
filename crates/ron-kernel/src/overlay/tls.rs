#![forbid(unsafe_code)]

use std::{fs::File, io::BufReader, sync::Arc};
use anyhow::Context;
use rustls_pemfile::{certs, pkcs8_private_keys};
use tokio_rustls::rustls::{
    pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer},
    ServerConfig,
};

pub fn try_build_server_config(cert_path: &str, key_path: &str) -> anyhow::Result<Arc<ServerConfig>> {
    let mut cert_reader = BufReader::new(File::open(cert_path).with_context(|| format!("open cert {cert_path}"))?);
    let certs: Vec<CertificateDer<'static>> =
        certs(&mut cert_reader).collect::<Result<Vec<_>, _>>().with_context(|| "parse certs")?;

    let mut key_reader = BufReader::new(File::open(key_path).with_context(|| format!("open key {key_path}"))?);
    let mut keys: Vec<PrivatePkcs8KeyDer<'static>> =
        pkcs8_private_keys(&mut key_reader).collect::<Result<Vec<_>, _>>().with_context(|| "parse pkcs8 key")?;

    let pkcs8 = keys.pop().ok_or_else(|| anyhow::anyhow!("no pkcs8 private key found"))?;
    let key_der: PrivateKeyDer<'static> = PrivateKeyDer::Pkcs8(pkcs8);

    let cfg = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key_der)
        .with_context(|| "with_single_cert")?;
    Ok(Arc::new(cfg))
}
