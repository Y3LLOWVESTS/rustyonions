use accounting::{Counters, CountingStream};
use anyhow::{anyhow, Context, Result};
use socks::Socks5Stream;
use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;
use transport::{Handler, SmallMsgTransport};

/// Tor/Arti transport:
/// - Outbound via SOCKS5 (e.g., 127.0.0.1:9050)
/// - Hidden service creation via ControlPort (e.g., 127.0.0.1:9051)
pub struct ArtiTransport {
    counters: Counters,
    socks_addr: String,
    connect_timeout: Duration,
}

impl ArtiTransport {
    pub fn new(window: Duration, socks_addr: impl Into<String>) -> Self {
        Self {
            counters: Counters::new(window),
            socks_addr: socks_addr.into(),
            connect_timeout: Duration::from_secs(10),
        }
    }

    /// Create an ephemeral v3 onion that forwards:
    ///   .onion:<virt_port>  ->  target_addr (e.g., 127.0.0.1:7000)
    ///
    /// Auth strategy:
    ///   1) PROTOCOLINFO to discover supported methods
    ///   2) Try NULL auth: `AUTHENTICATE ""`, then `AUTHENTICATE` (no arg)
    ///   3) If still not authed and COOKIE supported, try COOKIE (uses override or COOKIEFILE)
    pub fn create_hidden_service(
        control_addr: &str,
        cookie_path: Option<&std::path::Path>,
        virt_port: u16,
        target_addr: SocketAddr,
    ) -> Result<String> {
        // 1) Connect to ControlPort
        let mut ctrl = TcpStream::connect(control_addr)
            .with_context(|| format!("connect to Tor Control {}", control_addr))?;
        ctrl.set_read_timeout(Some(Duration::from_secs(5))).ok();
        ctrl.set_write_timeout(Some(Duration::from_secs(5))).ok();
        let mut reader = BufReader::new(ctrl.try_clone()?);

        // Helper: send a command (adds CRLF) and read until "250 ..." or "4xx/5xx" line.
        let mut send_cmd = |cmd: &str| -> Result<Vec<String>> {
            ctrl.write_all(cmd.as_bytes())?;
            ctrl.write_all(b"\r\n")?;
            ctrl.flush()?;
            let mut lines = Vec::new();
            loop {
                let mut s = String::new();
                let n = reader.read_line(&mut s)?;
                if n == 0 {
                    break;
                }
                let l = s.trim_end().to_string();
                lines.push(l.clone());
                // Success terminators for control replies
                if l == "250 OK" || l.starts_with("250 ") {
                    break;
                }
                // Error terminators
                if l.starts_with('4') || l.starts_with('5') {
                    break;
                }
            }
            Ok(lines)
        };

        let is_ok = |lines: &[String]| lines.iter().any(|l| l == "250 OK" || l.starts_with("250 "));

        // 2) Discover auth methods
        let proto = send_cmd("PROTOCOLINFO 1")?;
        let auth_line = proto
            .iter()
            .find(|l| l.starts_with("250-AUTH "))
            .cloned()
            .unwrap_or_default();

        let supports_null =
            auth_line.contains("METHODS=NULL") || auth_line.contains("METHODS=\"NULL\"");
        let supports_cookie =
            auth_line.contains("METHODS=COOKIE")
                || auth_line.contains("METHODS=SAFECOOKIE")
                || auth_line.contains("METHODS=\"COOKIE\"")
                || auth_line.contains("METHODS=\"SAFECOOKIE\"");

        // 3) Authenticate
        let mut authed = false;
        let mut last_resp: Vec<String> = Vec::new();

        if supports_null {
            // First try the common empty-string form
            last_resp = send_cmd(r#"AUTHENTICATE ""#)?;
            authed = is_ok(&last_resp);

            // Some builds accept AUTHENTICATE with no argument
            if !authed {
                last_resp = send_cmd("AUTHENTICATE")?;
                authed = is_ok(&last_resp);
            }
        }

        if !authed && supports_cookie {
            // Determine COOKIEFILE (use override if provided)
            let mut cookie_file = cookie_path.map(|p| p.to_path_buf());
            if cookie_file.is_none() {
                for l in &proto {
                    if let Some(pos) = l.find("COOKIEFILE=") {
                        if let Some(sq) = l[pos..].find('"') {
                            let rest = &l[pos + sq + 1..];
                            if let Some(eq) = rest.find('"') {
                                cookie_file = Some(std::path::PathBuf::from(&rest[..eq]));
                            }
                        }
                    }
                }
            }
            let cookie_file = cookie_file
                .ok_or_else(|| anyhow!("Tor COOKIEFILE not found (and NULL auth not available)"))?;
            let cookie_bytes = std::fs::read(&cookie_file)
                .with_context(|| format!("reading cookie {}", cookie_file.display()))?;
            let cookie_hex = hex::encode(cookie_bytes);
            last_resp = send_cmd(&format!("AUTHENTICATE {}", cookie_hex))?;
            authed = is_ok(&last_resp);
        }

        if !authed {
            return Err(anyhow!(
                "AUTHENTICATE failed (no supported method succeeded). Last response: {:?}",
                last_resp
            ));
        }

        // 4) Create ephemeral onion: Port=<virt_port>,<target_ip>:<target_port>
        let mapping = format!(
            "Port={},{}:{}",
            virt_port,
            target_addr.ip(),
            target_addr.port()
        );
        let add = send_cmd(&format!("ADD_ONION NEW:ED25519-V3 {}", mapping))?;

        // 5) Parse ServiceID
        let service_id = add
            .iter()
            .find_map(|l| l.strip_prefix("250-ServiceID=").map(|s| s.trim().to_string()))
            .ok_or_else(|| anyhow!("Tor did not return ServiceID (reply: {:?})", add))?;

        Ok(format!("{}.onion:{}", service_id, virt_port))
    }

    fn connect_via_socks(&self, to_addr: &str) -> Result<TcpStream> {
        // Accept host:port (supports .onion hostnames)
        let (host, port) = split_host_port(to_addr)
            .ok_or_else(|| anyhow!("expected host:port, got '{to_addr}'"))?;
        let proxy: SocketAddr = self
            .socks_addr
            .parse()
            .with_context(|| format!("parsing SOCKS addr '{}'", self.socks_addr))?;
        let s = Socks5Stream::connect(proxy, (host, port))
            .with_context(|| format!("SOCKS connect via {} -> {}:{}", proxy, host, port))?;
        let raw = s.into_inner();
        raw.set_read_timeout(Some(self.connect_timeout)).ok();
        raw.set_write_timeout(Some(self.connect_timeout)).ok();
        Ok(raw)
    }
}

fn split_host_port(s: &str) -> Option<(&str, u16)> {
    let idx = s.rfind(':')?;
    let (h, p) = s.split_at(idx);
    let port: u16 = p.trim_start_matches(':').parse().ok()?;
    Some((h, port))
}

impl SmallMsgTransport for ArtiTransport {
    type Stream = CountingStream<TcpStream>;

    fn dial(&self, to_addr: &str) -> Result<Self::Stream> {
        let raw = self.connect_via_socks(to_addr)?;
        Ok(CountingStream::new(raw, self.counters.clone()))
    }

    fn listen(&self, _bind: &str, _handler: Handler) -> Result<()> {
        // We expose the *existing* TCP listener via an onion; direct listen isn't used here.
        Err(anyhow!(
            "Use create_hidden_service(...) to expose a local listener over Tor"
        ))
    }

    fn counters(&self) -> Counters {
        self.counters.clone()
    }
}
