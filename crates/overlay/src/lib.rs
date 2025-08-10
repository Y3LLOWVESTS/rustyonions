use anyhow::{bail, Result};
use blake3::Hasher;
use sled::Db;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::thread;
use tracing::{error, info};

pub struct Store {
    db: Db,
    chunk_size: usize,
}

impl Store {
    pub fn open(path: impl AsRef<std::path::Path>, chunk_size: usize) -> Result<Self> {
        Ok(Self {
            db: sled::open(path)?,
            chunk_size,
        })
    }

    pub fn put_bytes(&self, bytes: &[u8]) -> Result<String> {
        let mut h = Hasher::new();
        h.update(bytes);
        let digest = h.finalize().to_hex().to_string();
        self.db.insert(digest.as_bytes(), bytes)?;
        self.db.flush()?;
        Ok(digest)
    }

    pub fn get(&self, hash_hex: &str) -> Result<Option<Vec<u8>>> {
        Ok(self.db.get(hash_hex.as_bytes())?.map(|v| v.to_vec()))
    }

    pub fn put_reader<R: Read>(&self, mut r: R) -> Result<String> {
        let mut buf = Vec::new();
        r.read_to_end(&mut buf)?;
        self.put_bytes(&buf)
    }

    pub fn chunk_size(&self) -> usize {
        self.chunk_size
    }
}

impl Clone for Store {
    fn clone(&self) -> Self {
        Self {
            db: self.db.clone(),
            chunk_size: self.chunk_size,
        }
    }
}

/// Protocol (line-based):
///   PUT <len>\n<raw-bytes>    -> "OK <hash>\n"
///   GET <hex>\n               -> "OK <len>\n<raw-bytes>" | "NONE\n"
pub fn run_overlay_listener(addr: SocketAddr, store: Store) -> std::io::Result<()> {
    let listener = TcpListener::bind(addr)?;
    info!("Overlay listening on {addr}");
    for c in listener.incoming() {
        match c {
            Ok(s) => {
                let store2 = store.clone();
                thread::spawn(move || {
                    if let Err(e) = handle_conn(s, &store2) {
                        error!("overlay conn: {e:?}");
                    }
                });
            }
            Err(e) => error!("overlay accept: {e:?}"),
        }
    }
    Ok(())
}

fn handle_conn(mut s: TcpStream, store: &Store) -> std::io::Result<()> {
    // Important: read command line AND payload from the SAME BufReader over the SAME stream handle.
    let mut r = BufReader::new(&mut s);

    let mut line = String::new();
    r.read_line(&mut line)?;
    let line = line.trim().to_string();

    if let Some(rest) = line.strip_prefix("GET ") {
        match store.get(rest).ok().flatten() {
            Some(bytes) => {
                s.write_all(format!("OK {}\n", bytes.len()).as_bytes())?;
                s.write_all(&bytes)?;
            }
            None => {
                s.write_all(b"NONE\n")?;
            }
        }
        return Ok(());
    }

    if let Some(rest) = line.strip_prefix("PUT ") {
        let len: usize = rest.parse().unwrap_or(0);
        let mut buf = vec![0u8; len];
        // Read exactly len bytes from the SAME BufReader buffer/stream
        r.read_exact(&mut buf)?;
        match store.put_bytes(&buf) {
            Ok(hash) => s.write_all(format!("OK {hash}\n").as_bytes())?,
            Err(_) => s.write_all(b"ERR\n")?,
        }
        return Ok(());
    }

    // Unknown command
    s.write_all(b"ERR\n")?;
    Ok(())
}

/// ---- Tiny client helpers (so CLI can use the network, not the DB) ----

pub fn client_put(addr: SocketAddr, bytes: &[u8]) -> Result<String> {
    let mut s = TcpStream::connect(addr)?;
    s.write_all(format!("PUT {}\n", bytes.len()).as_bytes())?;
    s.write_all(bytes)?;
    s.flush()?;

    let mut r = BufReader::new(s);
    let mut line = String::new();
    r.read_line(&mut line)?;
    let line = line.trim();
    if let Some(hash) = line.strip_prefix("OK ") {
        Ok(hash.to_string())
    } else {
        bail!("PUT failed: {line}");
    }
}

pub fn client_get(addr: SocketAddr, hash: &str) -> Result<Option<Vec<u8>>> {
    let mut s = TcpStream::connect(addr)?;
    s.write_all(format!("GET {hash}\n").as_bytes())?;
    s.flush()?;

    let mut r = BufReader::new(s);
    let mut line = String::new();
    r.read_line(&mut line)?;
    let line = line.trim().to_string();

    if line == "NONE" {
        return Ok(None);
    }
    if let Some(rest) = line.strip_prefix("OK ") {
        let len: usize = rest.parse().unwrap_or(0);
        let mut buf = vec![0u8; len];
        r.read_exact(&mut buf)?;
        return Ok(Some(buf));
    }
    bail!("GET failed: {line}");
}
