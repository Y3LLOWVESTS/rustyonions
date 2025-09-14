#![forbid(unsafe_code)]

use std::{fs::File, io::BufReader, net::ToSocketAddrs, sync::Arc};

use tokio::net::TcpStream;
use tokio_rustls::rustls::pki_types::ServerName;
use tokio_rustls::{rustls, TlsConnector};
use tokio_util::codec::Framed;

use crate::constants::{DEFAULT_MAX_DECOMPRESSED, DEFAULT_MAX_FRAME};
use crate::errors::{Error, Result};
use crate::oap::codec::OapCodec;

use super::OverlayClient;

impl OverlayClient {
    /// Connect over TCP+TLS using system roots, plus optional extra CA from `RON_EXTRA_CA` (PEM).
    ///
    /// `addr`: "host:port", `server_name`: SNI/hostname (must match cert CN/SAN).
    pub async fn connect(addr: &str, server_name: &str) -> Result<Self> {
        // Resolve
        let sockaddr = addr
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| Error::Io(std::io::Error::other("addr resolve failed")))?;

        let tcp = TcpStream::connect(sockaddr).await?;
        tcp.set_nodelay(true)?;

        // Build rustls client config with native roots (rustls-native-certs >= 0.8)
        let mut roots = rustls::RootCertStore::empty();

        // New API returns CertificateResult { certs, errors }.
        // Proceed with partial success but warn about any errors.
        let native = rustls_native_certs::load_native_certs();
        for cert in native.certs {
            // Each item is CertificateDer<'static>
            roots
                .add(cert)
                .map_err(|_| Error::Protocol("failed to add root cert".into()))?;
        }
        if !native.errors.is_empty() {
            tracing::warn!(
                "rustls-native-certs reported {} error(s) while loading roots: {:?}",
                native.errors.len(),
                native.errors
            );
        }

        // Optional extra CA (useful for self-signed local server)
        if let Ok(extra_path) = std::env::var("RON_EXTRA_CA") {
            let mut rd =
                BufReader::new(File::open(&extra_path).map_err(|e| {
                    Error::Protocol(format!("open RON_EXTRA_CA {extra_path}: {e}"))
                })?);

            for der in rustls_pemfile::certs(&mut rd)
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|e| Error::Protocol(format!("parse RON_EXTRA_CA {extra_path}: {e}")))?
            {
                roots
                    .add(der)
                    .map_err(|_| Error::Protocol("failed to add RON_EXTRA_CA cert".into()))?;
            }
        } else if roots.is_empty() {
            // No native roots and no extra CA path; continue (permissive) but warn loudly.
            tracing::warn!("no native root certificates found and RON_EXTRA_CA not set; TLS validation may fail");
        }

        let config = rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();

        let connector = TlsConnector::from(Arc::new(config));

        // Owned ServerName<'static> so the future doesn't borrow `server_name`
        let sni: ServerName = ServerName::try_from(server_name.to_string())
            .map_err(|_| Error::InvalidDnsName(server_name.to_string()))?;

        let tls = connector.connect(sni, tcp).await?;

        let codec = OapCodec::new(DEFAULT_MAX_FRAME, DEFAULT_MAX_DECOMPRESSED);
        let framed = Framed::new(tls, codec);

        Ok(Self {
            framed,
            corr_seq: 1,
            server: None,
        })
    }
}
