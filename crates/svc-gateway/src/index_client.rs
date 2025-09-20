#![allow(dead_code)]
#![forbid(unsafe_code)]

use anyhow::{anyhow, Context, Result};
use rmp_serde::encode::to_vec_named;
use serde::{Deserialize, Serialize};
use std::env;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

const ENV_SOCK: &str = "RON_INDEX_SOCK";

#[derive(Clone, Debug)]
pub struct IndexClient {
    sock_path: PathBuf,
}

impl IndexClient {
    /// Construct from env var `RON_INDEX_SOCK` (script sets this), or fallback.
    pub fn from_env_or(sock_fallback: impl AsRef<Path>) -> Self {
        let p = env::var_os(ENV_SOCK)
            .map(PathBuf::from)
            .unwrap_or_else(|| sock_fallback.as_ref().to_path_buf());
        Self { sock_path: p }
    }

    /// Construct explicitly from a socket path.
    pub fn new(sock_path: impl AsRef<Path>) -> Self {
        Self {
            sock_path: sock_path.as_ref().to_path_buf(),
        }
    }

    fn connect(&self) -> Result<UnixStream> {
        let s = UnixStream::connect(&self.sock_path)
            .with_context(|| format!("connect({})", self.sock_path.display()))?;
        s.set_read_timeout(Some(Duration::from_secs(2))).ok();
        s.set_write_timeout(Some(Duration::from_secs(2))).ok();
        Ok(s)
    }

    /// Ping the index service (mostly for debugging).
    #[allow(dead_code)]
    pub fn ping(&self) -> Result<String> {
        let mut s = self.connect()?;
        write_frame(&mut s, &Req::Ping)?;
        let resp = read_frame(&mut s)?;
        // try tolerant decodes
        if let Ok(p) = rmp_serde::from_slice::<Pong>(&resp) {
            return Ok(p.msg);
        }
        if let Ok(r) = rmp_serde::from_slice::<Resp>(&resp) {
            match r {
                Resp::Ok => return Ok("OK".into()),
                Resp::Pong { msg } => return Ok(msg),
                Resp::Err { err } => return Err(anyhow!("svc-index ping failed: {err}")),
                _ => {}
            }
        }
        Err(anyhow!("svc-index ping: unrecognized framed response"))
    }

    /// Resolve an address to its bundle directory.
    pub fn resolve_dir(&self, addr: &str) -> Result<PathBuf> {
        let mut s = self.connect()?;
        write_frame(&mut s, &Req::Resolve { addr })?;
        let resp = read_frame(&mut s)?;

        // 1) Try a struct response { ok, dir, err }
        if let Ok(r) = rmp_serde::from_slice::<ResolveStruct>(&resp) {
            if r.ok {
                if let Some(d) = r.dir {
                    return Ok(PathBuf::from(d));
                }
                return Err(anyhow!("svc-index resolve: ok=true but dir missing"));
            } else {
                return Err(anyhow!(
                    "svc-index resolve failed: {}",
                    r.err.unwrap_or_else(|| "unknown error".into())
                ));
            }
        }

        // 2) Try an enum response
        if let Ok(r) = rmp_serde::from_slice::<Resp>(&resp) {
            match r {
                Resp::ResolveOk { dir } => return Ok(PathBuf::from(dir)),
                Resp::NotFound => return Err(anyhow!("svc-index resolve: not found")),
                Resp::Err { err } => return Err(anyhow!("svc-index resolve failed: {err}")),
                Resp::Ok | Resp::Pong { .. } => {
                    return Err(anyhow!("svc-index resolve: unexpected response"))
                }
            }
        }

        Err(anyhow!(
            "svc-index resolve: unrecognized framed response ({} bytes)",
            resp.len()
        ))
    }
}

/* ===== Protocol =====
   Keep both struct/enum forms to interop with either style of svc-index.
*/

#[derive(Serialize)]
enum Req<'a> {
    Ping,
    PutAddress { addr: &'a str, dir: &'a str },
    Resolve { addr: &'a str },
}

#[derive(Deserialize)]
struct ResolveStruct {
    ok: bool,
    dir: Option<String>,
    err: Option<String>,
}

#[derive(Deserialize)]
struct Pong {
    msg: String,
}

#[derive(Deserialize)]
enum Resp {
    Ok,
    Pong { msg: String },
    ResolveOk { dir: String },
    NotFound,
    Err { err: String },
}

/* ===== Framing (length-prefixed: u32 big-endian) ===== */

fn write_frame<W: Write, T: Serialize>(w: &mut W, msg: &T) -> Result<()> {
    let payload = to_vec_named(msg)?;
    let len = payload.len();
    if len > u32::MAX as usize {
        return Err(anyhow!("frame too large: {} bytes", len));
    }
    let be = (len as u32).to_be_bytes();
    w.write_all(&be)?;
    w.write_all(&payload)?;
    Ok(())
}

fn read_exact(w: &mut impl Read, buf: &mut [u8]) -> Result<()> {
    let mut read = 0;
    while read < buf.len() {
        let n = w.read(&mut buf[read..]).context("svc-index read frame")?;
        if n == 0 {
            return Err(anyhow!("svc-index closed connection mid-frame"));
        }
        read += n;
    }
    Ok(())
}

fn read_frame<R: Read>(r: &mut R) -> Result<Vec<u8>> {
    let mut len4 = [0u8; 4];
    read_exact(r, &mut len4)?;
    let len = u32::from_be_bytes(len4) as usize;
    if len == 0 {
        return Err(anyhow!("svc-index sent empty frame (len=0)"));
    }
    let mut payload = vec![0u8; len];
    read_exact(r, &mut payload)?;
    Ok(payload)
}
