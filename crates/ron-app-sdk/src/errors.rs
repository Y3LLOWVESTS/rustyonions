#![forbid(unsafe_code)]

use std::io;

use thiserror::Error;
use tokio_rustls::rustls;

#[derive(Debug, Error)]
pub enum Error {
    #[error("io: {0}")]
    Io(#[from] io::Error),

    #[error("tls: {0}")]
    Tls(#[from] rustls::Error),

    #[error("invalid DNS name: {0}")]
    InvalidDnsName(String),

    #[error("decode: {0}")]
    Decode(String),

    #[error("protocol: {0}")]
    Protocol(String),

    #[error("timeout")]
    Timeout,
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
