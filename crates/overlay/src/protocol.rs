use anyhow::{bail, Context, Result};
use blake3::Hasher;
use std::net::SocketAddr;
use std::path::Path;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tracing::{info, warn};

use crate::store::Store;

// Wire opcodes
#[repr(u8)]
enum Op {
    Put = 0x01,
    Get = 0x02,
    PutOk = 0x10,
    GetOk = 0x11,
    NotFound = 0x12,
}

pub fn run_overlay_listener(bind: SocketAddr) -> Result<()> {
    tokio::spawn(async move {
        if let Err(e) = serve_tcp(bind).await {
            warn!(error=?e, "overlay listener exited with error");
        }
    });
    Ok(())
}

async fn serve_tcp(bind: SocketAddr) -> Result<()> {
    let store = Store::open(".data/sled")?;
    let listener = TcpListener::bind(bind).await?;
    info!(%bind, "overlay TCP listening");
    loop {
        let (stream, peer) = listener.accept().await?;
        let store = store.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_conn(stream, peer, store).await {
                warn!(%peer, error=?e, "connection error");
            }
        });
    }
}

async fn handle_conn(mut s: TcpStream, peer: SocketAddr, store: Store) -> Result<()> {
    let mut tag = [0u8; 1];
    s.read_exact(&mut tag).await.context("read opcode")?;
    match tag[0] {
        x if x == Op::Put as u8 => handle_put(&mut s, store).await,
        x if x == Op::Get as u8 => handle_get(&mut s, store).await,
        _ => bail!("bad opcode from {peer}"),
    }
}

async fn handle_put(s: &mut TcpStream, store: Store) -> Result<()> {
    let mut path_len = [0u8; 2];
    s.read_exact(&mut path_len).await?;
    let n = u16::from_be_bytes(path_len) as usize;
    let mut path_bytes = vec![0u8; n];
    s.read_exact(&mut path_bytes).await?;

    let mut sz = [0u8; 8];
    s.read_exact(&mut sz).await?;
    let sz = u64::from_be_bytes(sz) as usize;

    let mut data = vec![0u8; sz];
    s.read_exact(&mut data).await?;

    // Hash and store
    let mut hasher = Hasher::new();
    hasher.update(&data);
    let hash = hasher.finalize().to_hex().to_string();

    store.put(hash.as_bytes(), data)?;

    // Reply with hash
    s.write_u8(Op::PutOk as u8).await?;
    s.write_all(hash.as_bytes()).await?;
    s.write_all(b"\r\n").await?;
    s.flush().await?;
    Ok(())
}

async fn handle_get(s: &mut TcpStream, store: Store) -> Result<()> {
    let mut rdr = BufReader::new(s);
    let mut hash_hex = String::new();
    rdr.read_line(&mut hash_hex).await?;
    if hash_hex.ends_with('\n') {
        hash_hex.pop();
        if hash_hex.ends_with('\r') {
            hash_hex.pop();
        }
    }
    let resp = store.get(hash_hex.as_bytes())?;
    // move back the underlying stream to write
    let inner = rdr.into_inner();

    match resp {
        Some(bytes) => {
            inner.write_u8(Op::GetOk as u8).await?;
            inner.write_all(&(bytes.len() as u64).to_be_bytes()).await?;
            inner.write_all(&bytes).await?;
            inner.flush().await?;
        }
        None => {
            inner.write_u8(Op::NotFound as u8).await?;
            inner.flush().await?;
        }
    }
    Ok(())
}

pub fn client_put(addr: &str, path: &Path) -> Result<String> {
    // Read whole file
    let bytes = std::fs::read(path).with_context(|| format!("reading {path:?}"))?;
    let mut hasher = Hasher::new();
    hasher.update(&bytes);
    let hash = hasher.finalize().to_hex().to_string();

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async move {
        let mut s = TcpStream::connect(addr).await.context("connect")?;
        // PUT <path_len><path_bytes><size><data>
        let p = path.as_os_str().as_encoded_bytes();
        s.write_u8(Op::Put as u8).await?;
        s.write_all(&(p.len() as u16).to_be_bytes()).await?;
        s.write_all(p).await?;
        s.write_all(&(bytes.len() as u64).to_be_bytes()).await?;
        s.write_all(&bytes).await?;
        s.flush().await?;

        // Expect PutOk + hash
        let mut tag = [0u8; 1];
        s.read_exact(&mut tag).await?;
        if tag[0] != Op::PutOk as u8 {
            bail!("bad response tag");
        }
        let mut line = String::new();
        let mut r = BufReader::new(s);
        r.read_line(&mut line).await?;
        let got = line.trim().to_string();
        if got != hash {
            bail!("hash mismatch: expected {hash} got {got}");
        }
        Ok::<String, anyhow::Error>(hash)
    })
}

pub fn client_get(addr: &str, hash: &str, out: &Path) -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async move {
        let mut s = TcpStream::connect(addr).await.context("connect")?;
        // GET <hash>\r\n
        s.write_u8(Op::Get as u8).await?;
        s.write_all(hash.as_bytes()).await?;
        s.write_all(b"\r\n").await?;
        s.flush().await?;

        // Read tag
        let mut tag = [0u8; 1];
        s.read_exact(&mut tag).await?;
        match tag[0] {
            x if x == Op::GetOk as u8 => {
                let mut sz = [0u8; 8];
                s.read_exact(&mut sz).await?;
                let n = u64::from_be_bytes(sz) as usize;
                let mut buf = vec![0u8; n];
                s.read_exact(&mut buf).await?;
                fs::write(out, &buf).await?;
                Ok(())
            }
            x if x == Op::NotFound as u8 => bail!("not found"),
            _ => bail!("bad response"),
        }
    })
}
