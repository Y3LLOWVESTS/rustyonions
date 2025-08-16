use accounting::{Counters, CountingStream};
use anyhow::{anyhow, Context, Result};
use socks::Socks5Stream;
use std::time::Duration;
use transport::ReadWrite;

/// Connect to `addr` ("host:port") via SOCKS5 proxy at `socks_addr`.
pub(crate) fn connect_via_socks(
    socks_addr: &str,
    addr: &str,
    connect_timeout: Duration,
    counters: Counters,
) -> Result<Box<dyn ReadWrite + Send>> {
    let (host, port) = parse_host_port(addr)?;
    let stream = Socks5Stream::connect(socks_addr, (host.as_str(), port))?.into_inner();
    stream.set_read_timeout(Some(connect_timeout)).ok();
    stream.set_write_timeout(Some(connect_timeout)).ok();
    Ok(Box::new(CountingStream::new(stream, counters)))
}

fn parse_host_port(addr: &str) -> Result<(String, u16)> {
    let mut parts = addr.rsplitn(2, ':');
    let p = parts.next().ok_or_else(|| anyhow!("missing :port in addr"))?;
    let h = parts.next().ok_or_else(|| anyhow!("missing host in addr"))?;
    let port = p.parse::<u16>().context("parsing port")?;
    Ok((h.to_string(), port))
}
