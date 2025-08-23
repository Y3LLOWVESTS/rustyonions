#![forbid(unsafe_code)]

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

pub fn run_overlay_listener(bind: SocketAddr, store_db: impl AsRef<Path>) -> Result<()> {
    let store_db = store_db.as_ref().to_owned();
    tokio::spawn(async move {
        if let Err(e) = serve_tcp(bind, store_db.clone()).await {
            warn!(error=?e, "overlay listener exited with error");
        }
    });
    Ok(())
}

async fn serve_tcp(bind: SocketAddr, store_db: std::path::PathBuf) -> Result<()> {
    let store = Store::open(&store_db)?;
    let listener = TcpListener::bind(bind).await?;
    info!(%bind, store=?store_db, "overlay TCP listening");
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
    let mut rdr = BufReader::new(s);
    let mut buf = Vec::new();
    rdr.read_until(b'\n', &mut buf).await?; // 8-byte len + '\n'
    if buf.len() < 9 {
        bail!("short length line");
    }
    let n = u64::from_be_bytes(buf[..8].try_into().unwrap()) as usize;

    let mut data = vec![0u8; n];
    rdr.read_exact(&mut data).await?;

    // Hash bytes
    let mut hasher = Hasher::new();
    hasher.update(&data);
    let hash_hex = hasher.finalize().to_hex().to_string();

    // Store (key is the hex hash)
    store.put(hash_hex.as_bytes(), data)?;

    // Reply with hash
    let mut s = rdr.into_inner();
    send_line(&mut s, Op::PutOk as u8, &hash_hex).await?;
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
    let mut s = rdr.into_inner();

    match resp {
        Some(bytes) => {
            s.write_all(&[Op::GetOk as u8]).await?;
            s.write_all(&(bytes.len() as u64).to_be_bytes()).await?;
            s.write_all(&bytes).await?;
            s.flush().await?;
        }
        None => {
            s.write_all(&[Op::NotFound as u8]).await?;
            s.flush().await?;
        }
    }
    Ok(())
}

async fn send_line(s: &mut TcpStream, tag: u8, line: &str) -> Result<()> {
    s.write_all(&[tag]).await?;
    s.write_all(line.as_bytes()).await?;
    s.write_all(b"\r\n").await?;
    s.flush().await?;
    Ok(())
}

/// Async client: PUT a file to the overlay service. Returns the server's hash.
pub async fn client_put(addr: &str, path: &Path) -> Result<String> {
    // Read whole file synchronously (small convenience)
    let bytes = std::fs::read(path).with_context(|| format!("reading {path:?}"))?;
    let mut s = TcpStream::connect(addr).await?;
    s.write_all(&[Op::Put as u8]).await?;
    s.write_all(&(bytes.len() as u64).to_be_bytes()).await?;
    s.write_all(b"\n").await?;
    s.write_all(&bytes).await?;
    s.flush().await?;

    let mut tag = [0u8; 1];
    s.read_exact(&mut tag).await?;
    if tag[0] != Op::PutOk as u8 {
        bail!("bad response");
    }
    let mut line = String::new();
    BufReader::new(s).read_line(&mut line).await?;
    Ok(line.trim().to_string())
}

/// Async client: GET a blob by hash from the overlay service and write to `out`.
pub async fn client_get(addr: &str, hash_hex: &str, out: &Path) -> Result<()> {
    let mut s = TcpStream::connect(addr).await?;
    s.write_all(&[Op::Get as u8]).await?;
    s.write_all(hash_hex.as_bytes()).await?;
    s.write_all(b"\r\n").await?;
    s.flush().await?;

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
}
