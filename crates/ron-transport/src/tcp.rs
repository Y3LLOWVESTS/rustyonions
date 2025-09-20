//! Tokio TCP implementation of the Transport trait.

use crate::TransportListener;
use anyhow::Result;
use async_trait::async_trait;
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};

pub struct TcpTransport;

pub struct TcpListen {
    inner: TcpListener,
}

#[async_trait]
impl TransportListener for TcpListen {
    type Stream = TcpStream;

    async fn accept(&mut self) -> Result<(Self::Stream, SocketAddr)> {
        let (stream, peer) = self.inner.accept().await?;
        Ok((stream, peer))
    }
}

#[async_trait]
impl crate::Transport for TcpTransport {
    type Stream = TcpStream;
    type Listener = TcpListen;

    async fn connect(&self, peer_addr: &str) -> Result<Self::Stream> {
        let stream = TcpStream::connect(peer_addr).await?;
        Ok(stream)
    }

    async fn listen(&self, bind: SocketAddr) -> Result<Self::Listener> {
        let listener = TcpListener::bind(bind).await?;
        Ok(TcpListen { inner: listener })
    }
}

impl TcpTransport {
    pub async fn bind(bind: SocketAddr) -> Result<TcpListener> {
        Ok(TcpListener::bind(bind).await?)
    }
    pub async fn dial(addr: &str) -> Result<TcpStream> {
        Ok(TcpStream::connect(addr).await?)
    }
}
