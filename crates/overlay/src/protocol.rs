#![forbid(unsafe_code)]
//! Binary, length-prefixed framing for the overlay line protocol.
//!
//! Requests (client → server)
//! -------------------------
//! PUT:  [0x01][u64:len][len bytes payload]
//! GET:  [0x02][u16:hlen][hlen bytes ASCII hex key]
//!
//! Responses (server → client)
//! --------------------------
//! PUT OK:   [0x81][u16:hlen][hlen bytes ASCII hex key]
//! GET OK:   [0x82][u64:len][len bytes payload]
//! NOTFOUND: [0x7F]
//! ERR:      [0xFF][u16:mlen][mlen bytes UTF-8 message]   (currently unused)

use crate::error::{OResult, OverlayError};
use crate::store::Store;
use anyhow::{anyhow, Context as AnyContext, Result as AnyResult};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::thread;
use tracing::{error, info};

// NEW: generic transport support
use transport::{Handler, ReadWrite, Transport};

#[repr(u8)]
enum Op {
    PutReq = 0x01,
    GetReq = 0x02,
    PutOk = 0x81,
    GetOk = 0x82,
    NotFound = 0x7F,
    Err = 0xFF,
}

impl From<Op> for u8 {
    fn from(v: Op) -> Self {
        v as u8
    }
}

// === server (TCP convenience) ===

pub fn run_overlay_listener(addr: SocketAddr, store: Store) -> AnyResult<()> {
    let ln = TcpListener::bind(addr)?;
    info!("overlay listening on {}", addr);
    let store = store.clone();
    thread::spawn(move || {
        for conn in ln.incoming() {
            match conn {
                Ok(stream) => {
                    if let Err(e) = handle_conn_tcp(stream, &store) {
                        error!("overlay handler error: {e:?}");
                    }
                }
                Err(e) => error!("overlay accept error: {e:?}"),
            }
        }
    });
    Ok(())
}

// === NEW: server over a generic Transport ===

pub fn run_overlay_listener_with_transport<T: Transport + Send + Sync + 'static>(
    transport: &T,
    store: Store,
) -> AnyResult<()> {
    let store = store.clone();
    let handler: Handler = std::sync::Arc::new(move |rw| {
        if let Err(e) = handle_conn_rw(rw, &store) {
            error!("overlay handler error: {e:?}");
        }
    });
    transport.listen(handler)?;
    info!("overlay listening via generic transport");
    Ok(())
}

// === handler (now works with any Read+Write) ===

fn handle_conn_tcp(s: TcpStream, store: &Store) -> AnyResult<()> {
    handle_conn_rw(Box::new(s), store)
}

fn handle_conn_rw(mut rw: Box<dyn ReadWrite + Send>, store: &Store) -> AnyResult<()> {
    let tag = read_u8(&mut rw)?;
    match tag {
        x if x == u8::from(Op::PutReq) => {
            let n = read_u64(&mut rw).context("reading PUT length")? as usize;
            let mut buf = vec![0u8; n];
            read_exact(&mut rw, &mut buf)?;
            let hash = store.put(&buf)?;
            write_u8(&mut rw, u8::from(Op::PutOk))?;
            write_str(&mut rw, &hash)?;
            rw.flush()?;
            Ok(())
        }
        x if x == u8::from(Op::GetReq) => {
            let key = read_string(&mut rw).context("reading GET key")?;
            if let Some(bytes) = store.get(&key)? {
                write_u8(&mut rw, u8::from(Op::GetOk))?;
                write_u64(&mut rw, bytes.len() as u64)?;
                write_exact(&mut rw, &bytes)?;
                rw.flush()?;
            } else {
                write_u8(&mut rw, u8::from(Op::NotFound))?;
                rw.flush()?;
            }
            Ok(())
        }
        other => Err(OverlayError::UnknownOpcode(other).into()),
    }
}

// === TCP client helpers (existing public API) ===

pub fn client_put(addr: SocketAddr, data: &[u8]) -> AnyResult<String> {
    let mut s = TcpStream::connect(addr)?;
    write_u8(&mut s, u8::from(Op::PutReq))?;
    write_u64(&mut s, data.len() as u64)?;
    write_exact(&mut s, data)?;
    s.flush()?;
    let tag = read_u8(&mut s)?;
    match tag {
        x if x == u8::from(Op::PutOk) => read_string(&mut s).map_err(Into::into),
        _ => Err(anyhow!("unexpected tag in PUT response: 0x{tag:02x}")),
    }
}

pub fn client_get(addr: SocketAddr, hash: &str) -> AnyResult<Option<Vec<u8>>> {
    let mut s = TcpStream::connect(addr)?;
    write_u8(&mut s, u8::from(Op::GetReq))?;
    write_str(&mut s, hash)?;
    s.flush()?;
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

// === NEW: Transport-based client helpers (for Tor/.onion) ===

pub fn client_put_via<T: Transport>(transport: &T, to: &str, data: &[u8]) -> AnyResult<String> {
    let mut s = transport.connect(to)?;
    write_u8(&mut s, u8::from(Op::PutReq))?;
    write_u64(&mut s, data.len() as u64)?;
    write_exact(&mut s, data)?;
    s.flush()?;
    let tag = read_u8(&mut s)?;
    match tag {
        x if x == u8::from(Op::PutOk) => read_string(&mut s).map_err(Into::into),
        _ => Err(anyhow!("unexpected tag in PUT response: 0x{tag:02x}")),
    }
}

pub fn client_get_via<T: Transport>(
    transport: &T,
    to: &str,
    hash: &str,
) -> AnyResult<Option<Vec<u8>>> {
    let mut s = transport.connect(to)?;
    write_u8(&mut s, u8::from(Op::GetReq))?;
    write_str(&mut s, hash)?;
    s.flush()?;
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

fn read_exact<R: Read>(r: &mut R, b: &mut [u8]) -> OResult<()> {
    let mut n = 0usize;
    while n < b.len() {
        let m = r.read(&mut b[n..])?;
        if m == 0 {
            return Err(OverlayError::EarlyEof);
        }
        n += m;
    }
    Ok(())
}

fn write_exact<W: Write>(w: &mut W, b: &[u8]) -> OResult<()> {
    let mut n = 0usize;
    while n < b.len() {
        let m = w.write(&b[n..])?;
        if m == 0 {
            return Err(OverlayError::EarlyEof);
        }
        n += m;
    }
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
    if s.len() > u16::MAX as usize {
        return Err(OverlayError::StringTooLong(s.len()));
    }
    write_u16(w, s.len() as u16)?;
    write_exact(w, s.as_bytes())
}
