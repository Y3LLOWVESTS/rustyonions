#![forbid(unsafe_code)]
//! Binary, length-prefixed framing for the overlay protocol.
//!
//! Requests (client → server)
//! -------------------------
//! PUT:  [0x01][u64:len][len bytes payload]
//! GET:  [0x02][u16:hlen][hlen bytes ASCII hex key]
//
//! Responses (server → client)
//! --------------------------
//! PUT OK:   [0x81][u16:hlen][hlen bytes ASCII hex key]
//! GET OK:   [0x82][u64:len][len bytes payload]
//! NOTFOUND: [0x7F]
//! ERR:      [0xFF][u16:mlen][mlen bytes UTF-8 message]   (currently unused)
//
//! Notes
//! -----
//! - Little-endian integers (`to_le_bytes`/`from_le_bytes`).
//! - One request per TCP connection (simple & robust).
//! - Public API returns `anyhow::Result`; internals use `OverlayError`.

use crate::error::{OResult, OverlayError};
use crate::store::Store;
use anyhow::{anyhow, Context, Result as AnyResult};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::thread;
use tracing::{error, info};

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Op {
    PutReq   = 0x01,
    GetReq   = 0x02,
    PutOk    = 0x81,
    GetOk    = 0x82,
    NotFound = 0x7F,
    Err      = 0xFF,
}

impl From<Op> for u8 {
    #[inline]
    fn from(v: Op) -> Self { v as u8 }
}

// === server ===

/// Spawn a background listener thread that serves the framed protocol.
pub fn run_overlay_listener(addr: SocketAddr, store: Store) -> AnyResult<()> {
    let ln = TcpListener::bind(addr)?;
    info!("overlay listening on {}", addr);
    thread::spawn(move || {
        for conn in ln.incoming() {
            match conn {
                Ok(stream) => {
                    if let Err(e) = handle_conn(stream, &store) {
                        error!("overlay handler error: {e:?}");
                    }
                }
                Err(e) => error!("overlay accept error: {e:?}"),
            }
        }
    });
    Ok(())
}

/// Handle one connection (one request per connection for simplicity).
fn handle_conn(mut s: TcpStream, store: &Store) -> AnyResult<()> {
    let tag = read_u8(&mut s)?;
    match tag {
        x if x == u8::from(Op::PutReq) => {
            let n = read_u64(&mut s).context("reading PUT length")? as usize;
            let mut buf = vec![0u8; n];
            read_exact(&mut s, &mut buf)?;
            let hash = store.put(&buf)?;
            // respond
            write_u8(&mut s, u8::from(Op::PutOk))?;
            write_str(&mut s, &hash)?;
            s.flush()?;
            Ok(())
        }
        x if x == u8::from(Op::GetReq) => {
            let key = read_string(&mut s).context("reading GET key")?;
            if let Some(bytes) = store.get(&key)? {
                write_u8(&mut s, u8::from(Op::GetOk))?;
                write_u64(&mut s, bytes.len() as u64)?;
                write_exact(&mut s, &bytes)?;
                s.flush()?;
                Ok(())
            } else {
                write_u8(&mut s, u8::from(Op::NotFound))?;
                s.flush()?;
                Ok(())
            }
        }
        other => Err(OverlayError::UnknownOpcode(other).into()),
    }
}

// === client helpers (public API unchanged) ===

/// Client helper: PUT bytes and return content hash (hex).
pub fn client_put(addr: SocketAddr, data: &[u8]) -> AnyResult<String> {
    let mut s = TcpStream::connect(addr)?;
    write_u8(&mut s, u8::from(Op::PutReq))?;
    write_u64(&mut s, data.len() as u64)?;
    write_exact(&mut s, data)?;
    s.flush()?;

    // response
    let tag = read_u8(&mut s)?;
    match tag {
        x if x == u8::from(Op::PutOk) => read_string(&mut s).map_err(Into::into),
        _ => Err(anyhow!("unexpected tag in PUT response: 0x{tag:02x}")),
    }
}

/// Client helper: GET bytes by hex content hash.
pub fn client_get(addr: SocketAddr, hash: &str) -> AnyResult<Option<Vec<u8>>> {
    let mut s = TcpStream::connect(addr)?;
    write_u8(&mut s, u8::from(Op::GetReq))?;
    write_str(&mut s, hash)?;
    s.flush()?;

    // response
    let tag = read_u8(&mut s)?;
    match tag {
        x if x == u8::from(Op::GetOk) => {
            let n = read_u64(&mut s)? as usize;
            let mut buf = vec![0u8; n];
            read_exact(&mut s, &mut buf)?;
            Ok(Some(buf))
        }
        x if x == u8::from(Op::NotFound) => Ok(None),
        x if x == u8::from(Op::Err) => Err(anyhow!(read_string(&mut s)?)),
        _ => Err(anyhow!("unexpected tag in GET response: 0x{tag:02x}")),
    }
}

// === tiny framing utils (LE, overlay-internal errors) ===

fn read_exact<R: Read>(r: &mut R, buf: &mut [u8]) -> OResult<()> {
    let mut read_total = 0;
    while read_total < buf.len() {
        let n = r.read(&mut buf[read_total..])?;
        if n == 0 {
            return Err(OverlayError::EarlyEof);
        }
        read_total += n;
    }
    Ok(())
}

fn write_exact<W: Write>(w: &mut W, buf: &[u8]) -> OResult<()> {
    w.write_all(buf)?;
    Ok(())
}

fn read_u8<R: Read>(r: &mut R) -> OResult<u8> {
    let mut b = [0u8; 1];
    read_exact(r, &mut b)?;
    Ok(b[0])
}

fn write_u8<W: Write>(w: &mut W, v: u8) -> OResult<()> {
    write_exact(w, &[v])
}

fn read_u16<R: Read>(r: &mut R) -> OResult<u16> {
    let mut b = [0u8; 2];
    read_exact(r, &mut b)?;
    Ok(u16::from_le_bytes(b))
}

fn write_u16<W: Write>(w: &mut W, v: u16) -> OResult<()> {
    write_exact(w, &v.to_le_bytes())
}

fn read_u64<R: Read>(r: &mut R) -> OResult<u64> {
    let mut b = [0u8; 8];
    read_exact(r, &mut b)?;
    Ok(u64::from_le_bytes(b))
}

fn write_u64<W: Write>(w: &mut W, v: u64) -> OResult<()> {
    write_exact(w, &v.to_le_bytes())
}

fn read_string<R: Read>(r: &mut R) -> OResult<String> {
    let n = read_u16(r)? as usize;
    let mut buf = vec![0u8; n];
    read_exact(r, &mut buf)?;
    Ok(String::from_utf8(buf)?)
}

fn write_str<W: Write>(w: &mut W, s: &str) -> OResult<()> {
    let bytes = s.as_bytes();
    if bytes.len() > u16::MAX as usize {
        return Err(OverlayError::StringTooLong(bytes.len()));
    }
    write_u16(w, bytes.len() as u16)?;
    write_exact(w, bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use tempfile::tempdir;

    // Serve exactly one incoming connection, then return.
    fn serve_once(listener: TcpListener, store: Store) {
        std::thread::spawn(move || {
            if let Some(Ok(stream)) = listener.incoming().next() {
                let _ = super::handle_conn(stream, &store);
            }
        });
    }

    #[test]
    fn put_get_roundtrip() {
        let dir = tempdir().unwrap();
        let store = Store::open(dir.path(), 0).unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        serve_once(listener.try_clone().unwrap(), store.clone());
        let data = b"hello rusty onions";
        let hash = client_put(addr, data).expect("put ok");

        serve_once(listener, store);
        let got = client_get(addr, &hash).expect("get ok").expect("some");
        assert_eq!(got, data);
    }

    #[test]
    fn get_not_found() {
        let dir = tempdir().unwrap();
        let store = Store::open(dir.path(), 0).unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        serve_once(listener, store);

        let none = client_get(addr, "deadbeef").unwrap();
        assert!(none.is_none());
    }

    #[test]
    fn rejects_too_long_string() {
        use std::io::Cursor;
        let mut cur = Cursor::new(Vec::<u8>::new());
        let big = "a".repeat(u16::MAX as usize + 1);
        let err = super::write_str(&mut cur, &big).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("string too long"));
    }
}
