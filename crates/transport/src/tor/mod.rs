//! Tor helpers: ControlPort and SOCKS5 dialing/tunnels.

pub mod ctrl;
pub mod hs;

use crate::{Transport, TransportListener};
use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::copy_bidirectional;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{oneshot, RwLock};
use tokio::time::timeout;

#[derive(Clone)]
pub struct TorTransport {
    pub socks_addr: SocketAddr,
    pub control_addr: SocketAddr,
    pub control_password: Option<String>,
    // Arc<RwLock<…>> so Clone works
    pub published_onion: Arc<RwLock<Option<String>>>,
}

impl TorTransport {
    pub fn new(
        socks_addr: SocketAddr,
        control_addr: SocketAddr,
        control_password: Option<String>,
    ) -> Self {
        Self {
            socks_addr,
            control_addr,
            control_password,
            published_onion: Arc::new(RwLock::new(None)),
        }
    }
}

pub struct TorListen {
    inner: TcpListener,
}

#[async_trait]
impl TransportListener for TorListen {
    type Stream = TcpStream;
    async fn accept(&mut self) -> Result<(Self::Stream, SocketAddr)> {
        let (stream, peer) = self.inner.accept().await?;
        Ok((stream, peer))
    }
}

#[async_trait]
impl Transport for TorTransport {
    type Stream = TcpStream;
    type Listener = TorListen;

    async fn connect(&self, peer_addr: &str) -> Result<Self::Stream> {
        dial_via_socks(self.socks_addr, peer_addr).await
    }

    async fn listen(&self, bind: SocketAddr) -> Result<Self::Listener> {
        let listener = TcpListener::bind(bind).await?;
        Ok(TorListen { inner: listener })
    }
}

/// Dial `dest` (like `example.onion:1777` or `example.com:80`) via SOCKS5h.
pub async fn dial_via_socks(socks_addr: SocketAddr, dest: &str) -> Result<TcpStream> {
    let (host, port) = split_host_port_supporting_ipv6(dest)?;
    let port: u16 = port.parse().context("invalid port")?;
    let stream = tokio_socks::tcp::Socks5Stream::connect(socks_addr, (host, port))
        .await
        .context("SOCKS5 connect failed")?
        .into_inner();
    Ok(stream)
}

/// Start a one-shot local TCP → SOCKS5 tunnel.
/// Returns (local_addr, ready_rx, join_handle).
/// - `ready_rx` resolves to `Ok(())` once the proxy connection is established,
///    or `Err(String)` if dialing failed.
pub async fn start_oneshot_socks_tunnel(
    socks_addr: SocketAddr,
    dest: &str,
) -> Result<(
    SocketAddr,
    oneshot::Receiver<Result<(), String>>,
    tokio::task::JoinHandle<()>,
)> {
    let listener = TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0)).await?;
    let local_addr = listener.local_addr()?;
    let dest = dest.to_string();

    let (tx_ready, rx_ready) = oneshot::channel();

    let handle = tokio::spawn(async move {
        if let Ok((mut inbound, _peer)) = listener.accept().await {
            // Time-limit the SOCKS dial so clients fail fast on unreachable onions
            match timeout(Duration::from_secs(20), dial_via_socks(socks_addr, &dest)).await {
                Err(_) => {
                    let _ = tx_ready.send(Err("SOCKS dial timed out".into()));
                    return;
                }
                Ok(Err(e)) => {
                    let _ = tx_ready.send(Err(format!("SOCKS dial failed: {e}")));
                    return;
                }
                Ok(Ok(mut outbound)) => {
                    let _ = tx_ready.send(Ok(()));
                    let _ = copy_bidirectional(&mut inbound, &mut outbound).await;
                }
            }
        } else {
            let _ = tx_ready.send(Err("failed to accept local tunnel connection".into()));
        }
        // Listener drops here; tunnel completes after one connection.
    });

    Ok((local_addr, rx_ready, handle))
}

fn split_host_port_supporting_ipv6(s: &str) -> Result<(&str, &str)> {
    // Supports:
    //  - "<host>:<port>"
    //  - "[<ipv6>]:<port>"
    //  - "<onion>:<port>" (not socketaddr-parsable)
    if let Some(rest) = s.strip_prefix('[') {
        // [ipv6]:port
        if let Some(idx) = rest.find(']') {
            let host = &rest[..idx];
            let after = &rest[idx + 1..];
            if let Some((_, port)) = after.split_once(':') {
                return Ok((host, port));
            }
        }
    }
    if let Some((h, p)) = s.rsplit_once(':') {
        return Ok((h, p));
    }
    bail!("expected host:port, got '{s}'");
}
