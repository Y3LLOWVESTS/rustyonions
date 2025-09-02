#![forbid(unsafe_code)]

use tokio_util::codec::Framed;
use tokio_rustls::client::TlsStream;
use tokio::net::TcpStream;

use crate::oap::codec::OapCodec;

pub type FramedTls = Framed<TlsStream<TcpStream>, OapCodec>;

/// Minimal OAP/1 client (Bronze ring)
pub struct OverlayClient {
    pub(super) framed: FramedTls,
    pub(super) corr_seq: u64,
    pub(super) server: Option<crate::oap::hello::Hello>,
}

impl OverlayClient {
    #[inline]
    pub(super) fn next_corr(&mut self) -> u64 {
        let v = self.corr_seq;
        self.corr_seq = self.corr_seq.wrapping_add(1).max(1);
        v
    }
}

pub mod tls;
pub mod hello;
pub mod oneshot;
