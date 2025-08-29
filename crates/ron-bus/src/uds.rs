// crates/ron-bus/src/uds.rs
#![forbid(unsafe_code)]

#[cfg(unix)]
use std::os::unix::net::{UnixListener, UnixStream};

use crate::api::Envelope;
use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;

#[cfg(not(unix))]
compile_error!("ron-bus uds transport requires a Unix platform");

pub fn listen(sock_path: &str) -> io::Result<UnixListener> {
    let p = Path::new(sock_path);
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent)?;
    }
    if p.exists() {
        let _ = fs::remove_file(p);
    }
    UnixListener::bind(p)
}

pub fn recv(stream: &mut UnixStream) -> io::Result<Envelope> {
    let mut len_be = [0u8; 4];
    read_exact(stream, &mut len_be)?;
    let len = u32::from_be_bytes(len_be) as usize;
    if len == 0 {
        return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "empty frame"));
    }
    let mut buf = vec![0u8; len];
    read_exact(stream, &mut buf)?;
    rmp_serde::from_slice::<Envelope>(&buf)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("decode envelope: {e}")))
}

pub fn send(stream: &mut UnixStream, env: &Envelope) -> io::Result<()> {
    let buf = rmp_serde::to_vec(env)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, format!("encode envelope: {e}")))?;
    if buf.len() > u32::MAX as usize {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("frame too large: {} bytes", buf.len()),
        ));
    }
    let len_be = (buf.len() as u32).to_be_bytes();
    stream.write_all(&len_be)?;
    stream.write_all(&buf)?;
    Ok(())
}

fn read_exact<R: Read>(r: &mut R, mut buf: &mut [u8]) -> io::Result<()> {
    while !buf.is_empty() {
        let n = r.read(buf)?;
        if n == 0 {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "early EOF"));
        }
        let tmp = buf;
        buf = &mut tmp[n..];
    }
    Ok(())
}
