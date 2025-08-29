#![forbid(unsafe_code)]

use anyhow::{anyhow, Context, Result};
use ron_bus::api::{Envelope, OverlayReq, OverlayResp};
use std::env;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

const ENV_SOCK: &str = "RON_OVERLAY_SOCK";

#[derive(Clone, Debug)]
pub struct OverlayClient {
    sock_path: PathBuf,
}

impl OverlayClient {
    pub fn from_env_or(sock_fallback: impl AsRef<std::path::Path>) -> Self {
        let p = env::var_os(ENV_SOCK)
            .map(PathBuf::from)
            .unwrap_or_else(|| sock_fallback.as_ref().to_path_buf());
        Self { sock_path: p }
    }

    fn connect(&self) -> Result<UnixStream> {
        Ok(UnixStream::connect(&self.sock_path).with_context(|| {
            format!("connect overlay at {}", self.sock_path.display())
        })?)
    }

    pub fn get_bytes(&self, addr: &str, rel: &str) -> Result<Option<Vec<u8>>> {
        let mut s = self.connect()?;
        let req = Envelope {
            service: "svc.overlay".into(),
            method: "v1.get".into(),
            corr_id: 1,
            token: vec![],
            payload: rmp_serde::to_vec(&OverlayReq::Get { addr: addr.to_string(), rel: rel.to_string() })?,
        };
        write_frame(&mut s, &req)?;
        let env = read_frame(&mut s)?;
        let resp: OverlayResp = rmp_serde::from_slice(&env.payload)?;
        Ok(match resp {
            OverlayResp::Bytes { data } => Some(data),
            OverlayResp::NotFound => None,
            OverlayResp::Err { err } => return Err(anyhow!("overlay error: {err}")),
            OverlayResp::HealthOk => return Err(anyhow!("unexpected overlay resp")),
        })
    }
}

fn write_frame(w: &mut impl Write, env: &Envelope) -> Result<()> {
    let payload = rmp_serde::to_vec(env)?;
    if payload.len() > u32::MAX as usize {
        return Err(anyhow!("frame too large: {} bytes", payload.len()));
    }
    w.write_all(&(payload.len() as u32).to_be_bytes())?;
    w.write_all(&payload)?;
    Ok(())
}

fn read_frame(r: &mut impl Read) -> Result<Envelope> {
    let mut len4 = [0u8; 4];
    read_exact(r, &mut len4)?;
    let len = u32::from_be_bytes(len4) as usize;
    if len == 0 {
        return Err(anyhow!("overlay sent empty frame"));
    }
    let mut payload = vec![0u8; len];
    read_exact(r, &mut payload)?;
    Ok(rmp_serde::from_slice(&payload)?)
}

fn read_exact(r: &mut impl Read, buf: &mut [u8]) -> Result<()> {
    let mut read = 0;
    while read < buf.len() {
        let n = r.read(&mut buf[read..])?;
        if n == 0 {
            return Err(anyhow!("overlay closed connection"));
        }
        read += n;
    }
    Ok(())
}
