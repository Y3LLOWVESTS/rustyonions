//! Minimal Tor ControlPort client using **HashedControlPassword** auth,
//! with robust response parsing, HS event support, and manual retries.

use anyhow::{anyhow, bail, Result};
use std::net::SocketAddr;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;
use tokio::time::{sleep, timeout, Duration};

/// Tor control replies we care about.
#[derive(Debug)]
enum ReplyCode {
    Ok250,          // 250 OK or 250 <key>=<value> lines (final "250 OK")
    Async650,       // 650 (asynchronous event)
    Err4xx5xx(u16), // 4xx or 5xx
    Other(()),      // Anything else (unused payload -> unit to silence warning)
}

pub struct TorController {
    reader: BufReader<OwnedReadHalf>,
    writer: OwnedWriteHalf,
}

impl TorController {
    pub async fn connect_and_auth(addr: SocketAddr, password: &str) -> Result<Self> {
        let stream = TcpStream::connect(addr).await?;
        let (read_half, write_half) = stream.into_split();
        let reader = BufReader::new(read_half);
        let mut this = Self { reader, writer: write_half };
        this.authenticate(password).await?;
        Ok(this)
    }

    async fn read_line(&mut self) -> Result<String> {
        let mut buf = String::new();
        let n = self.reader.read_line(&mut buf).await?;
        if n == 0 {
            bail!("control socket closed");
        }
        Ok(buf)
    }

    async fn expect_250(&mut self) -> Result<()> {
        loop {
            let line = self.read_line().await?;
            let code = parse_reply_code(&line);
            match code {
                ReplyCode::Ok250 => return Ok(()),
                ReplyCode::Err4xx5xx(code) => bail!("Tor control error {code}: {line}"),
                ReplyCode::Async650 => { /* ignore while handling a command */ }
                ReplyCode::Other(_) => { /* ignore */ }
            }
        }
    }

    pub async fn authenticate(&mut self, password: &str) -> Result<()> {
        // Retry with exponential backoff – Tor might still be bootstrapping.
        let mut delay = Duration::from_millis(100);
        for attempt in 1..=5 {
            let pw = escape_for_auth(password);
            let cmd = format!("AUTHENTICATE \"{}\"\r\n", pw);
            if let Err(e) = self.writer.write_all(cmd.as_bytes()).await {
                if attempt == 5 {
                    return Err(e.into());
                }
            } else if self.expect_250().await.is_ok() {
                return Ok(());
            }
            sleep(delay).await;
            delay = delay.saturating_mul(2);
        }
        bail!("failed to AUTHENTICATE after retries");
    }

    pub async fn set_events(&mut self, events: &[&str]) -> Result<()> {
        let list = events.join(" ");
        let cmd = format!("SETEVENTS {}\r\n", list);
        self.writer.write_all(cmd.as_bytes()).await?;
        self.expect_250().await
    }

    /// Low-level helper that issues ADD_ONION with an explicit target host/port mapping.
    async fn add_onion_core(
        &mut self,
        key_type: &str,   // "NEW:ED25519-V3" or "ED25519-V3:<b64>"
        host: &str,       // e.g., "127.0.0.1"
        target_port: u16, // local port your service listens on
        virt_port: u16,   // public onion port
        flags: &[&str],   // e.g., ["DiscardPK"]
    ) -> Result<(String, Option<String>)> {
        let flags = if flags.is_empty() {
            "".to_string()
        } else {
            format!(" Flags={}", flags.join(","))
        };
        let cmd = format!(
            "ADD_ONION {} Port={},{}:{}{}\r\n",
            key_type, virt_port, host, target_port, flags
        );
        self.writer.write_all(cmd.as_bytes()).await?;

        let mut service_id: Option<String> = None;
        let mut private_key: Option<String> = None;

        loop {
            let line = self.read_line().await?;
            match parse_reply_code(&line) {
                ReplyCode::Ok250 => break, // done
                ReplyCode::Err4xx5xx(code) => bail!("Tor control error {code}: {line}"),
                ReplyCode::Async650 => { /* ignore */ }
                ReplyCode::Other(_) => { /* ignore */ }
            }

            if let Some(rest) = line.strip_prefix("250-ServiceID=") {
                service_id = Some(rest.trim().to_string());
            } else if let Some(rest) = line.strip_prefix("250-PrivateKey=") {
                private_key = Some(rest.trim().to_string());
            }
        }

        let id = service_id.ok_or_else(|| anyhow!("ADD_ONION did not return ServiceID"))?;
        Ok((id, private_key))
    }

    /// Compatibility wrapper used by `hs.rs` when an **existing** key line is provided.
    pub async fn add_onion_with_key(
        &mut self,
        key_line: &str,   // "ED25519-V3:<b64>"
        public_port: u16, // onion external port
        host: &str,       // usually "127.0.0.1"
        local_port: u16,  // your local service port
    ) -> Result<(String, Option<String>)> {
        self.add_onion_core(key_line, host, local_port, public_port, &[]).await
    }

    /// Convenience wrapper to create a **new** v3 onion, returning ServiceID and Optional PrivateKey.
    pub async fn add_onion_new_with_host(
        &mut self,
        public_port: u16,
        host: &str,
        local_port: u16,
        flags: &[&str],
    ) -> Result<(String, Option<String>)> {
        self.add_onion_core("NEW:ED25519-V3", host, local_port, public_port, flags).await
    }

    /// Older signature kept for compatibility with earlier code that always used 127.0.0.1.
    pub async fn add_onion_new(
        &mut self,
        key_type: &str, // "NEW:ED25519-V3" or "ED25519-V3:<b64>"
        port: u16,      // local target port
        virt_port: u16, // public onion port
        flags: &[&str],
    ) -> Result<(String, Option<String>)> {
        self.add_onion_core(key_type, "127.0.0.1", port, virt_port, flags).await
    }

    /// Wait until Tor emits an `HS_DESC UPLOADED` event for `service_id`, or time out.
    pub async fn wait_hs_desc_uploaded(&mut self, service_id: &str, wait_secs: u64) -> Result<()> {
        // Use bare ServiceID (no ".onion")
        let sid = service_id.trim().trim_end_matches(".onion");
        self.set_events(&["HS_DESC"]).await?;

        let fut = async {
            loop {
                let line = self.read_line().await?;
                if let ReplyCode::Async650 = parse_reply_code(&line) {
                    // Typical formats:
                    // 650 HS_DESC CREATED <sid> NO_AUTH <replica>
                    // 650 HS_DESC UPLOADED <sid> NO_AUTH <replica>
                    // 650 HS_DESC UPLOAD_FAILED <sid> NO_AUTH REASON=<...> <replica>
                    if line.contains("HS_DESC") && line.contains(sid) {
                        if line.contains("UPLOADED") {
                            return Ok(());
                        }
                        if line.contains("UPLOAD_FAILED") {
                            bail!("HS_DESC upload failed for {}: {}", sid, line.trim());
                        }
                    }
                }
            }
        };

        match timeout(Duration::from_secs(wait_secs), fut).await {
            Ok(res) => res,
            Err(_) => bail!("timed out waiting for HS_DESC UPLOADED for {}", sid),
        }
    }

    pub async fn del_onion(&mut self, service_id: &str) -> Result<()> {
        let cmd = format!("DEL_ONION {}\r\n", service_id);
        self.writer.write_all(cmd.as_bytes()).await?;
        self.expect_250().await
    }
}

fn escape_for_auth(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn parse_reply_code(line: &str) -> ReplyCode {
    let code = line
        .chars()
        .take(3)
        .collect::<String>()
        .parse::<u16>()
        .unwrap_or(0);

    match code {
        250 => ReplyCode::Ok250,
        650 => ReplyCode::Async650,
        400..=599 => ReplyCode::Err4xx5xx(code),
        _ => ReplyCode::Other(()),
    }
}
