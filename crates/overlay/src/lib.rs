use anyhow::Result;
use blake3::Hasher;
use sled::Db;
use std::io::Read;
use std::net::{TcpListener, TcpStream};
use std::thread;
use tracing::{info, error};

pub struct Store { db: Db, chunk_size: usize }
impl Store {
    pub fn open(path: impl AsRef<std::path::Path>, chunk_size: usize) -> Result<Self> {
        Ok(Self { db: sled::open(path)?, chunk_size })
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
    pub fn chunk_size(&self) -> usize { self.chunk_size }
}

/// Super-simple overlay node that accepts GET <hex>\n and returns raw bytes or "NONE".
pub fn run_overlay_listener(addr: std::net::SocketAddr, store: Store) -> std::io::Result<()> {
    let listener = TcpListener::bind(addr)?;
    info!("Overlay listening on {addr}");
    for c in listener.incoming() {
        match c {
            Ok(mut s) => {
                let store = store.clone();
                thread::spawn(move || {
                    if let Err(e) = handle_conn(&mut s, &store) { error!("overlay conn: {e:?}"); }
                });
            }
            Err(e) => error!("overlay accept: {e:?}"),
        }
    }
    Ok(())
}

fn handle_conn(s: &mut TcpStream, store: &Store) -> std::io::Result<()> {
    use std::io::{BufRead, BufReader, Write};
    let mut r = BufReader::new(s.try_clone()?);
    let mut line = String::new();
    r.read_line(&mut line)?;
    let line = line.trim();
    if let Some(rest) = line.strip_prefix("GET ") {
        match store.get(rest).ok().flatten() {
            Some(bytes) => {
                let _ = s.write_all(format!("OK {}\n", bytes.len()).as_bytes());
                let _ = s.write_all(&bytes);
            }
            None => { let _ = s.write_all(b"NONE\n"); }
        }
    } else {
        let _ = s.write_all(b"ERR\n");
    }
    Ok(())
}

impl Clone for Store {
    fn clone(&self) -> Self { Self { db: self.db.clone(), chunk_size: self.chunk_size } }
}
