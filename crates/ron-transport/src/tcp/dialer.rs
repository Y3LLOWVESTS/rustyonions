//! RO:WHAT — TCP dialer (MVP).
use std::net::SocketAddr;
use tokio::net::TcpStream;

pub async fn dial(addr: SocketAddr) -> std::io::Result<TcpStream> {
    TcpStream::connect(addr).await
}
