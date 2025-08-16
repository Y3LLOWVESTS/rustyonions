use anyhow::{anyhow, bail, Context, Result};
use std::fmt::Write as _;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;

/// Minimal Tor control-port client with auth + multi-line command support.
pub(crate) struct CtrlClient {
    s: TcpStream,
    r: BufReader<TcpStream>,
}

impl CtrlClient {
    /// Authenticate via Tor control port.
    ///
    /// Strategy:
    ///   1) Send PROTOCOLINFO (multi-line), parse AUTH METHODS and COOKIEFILE.
    ///   2) If cookie auth is available (COOKIE or SAFECOOKIE) and we have a COOKIEFILE,
    ///      read it, hex-encode, and send AUTHENTICATE <hex>.
    ///   3) If cookie override is provided, use that path instead of COOKIEFILE.
    pub(crate) fn authenticate(tor_ctrl_addr: &str, cookie_override: Option<&str>) -> Result<Self> {
        let s = TcpStream::connect(tor_ctrl_addr)?;
        s.set_nodelay(true).ok();
        let r = BufReader::new(s.try_clone()?);
        let mut client = Self { s, r };

        // 1) Query PROTOCOLINFO (multi-line)
        let lines = client.cmd_multi("PROTOCOLINFO 1")?;
        let (auth_methods, cookie_path_opt) = parse_protocolinfo(&lines);

        // 2) Decide on auth method
        // Prefer cookie if available (COOKIE or SAFECOOKIE).
        if auth_methods.cookie || auth_methods.safecookie {
            let cookie_path = if let Some(p) = cookie_override {
                p.to_string()
            } else {
                cookie_path_opt
                    .ok_or_else(|| anyhow!("Tor PROTOCOLINFO missing COOKIEFILE"))?
            };

            let cookie = std::fs::read(&cookie_path)
                .with_context(|| format!("reading cookie {}", cookie_path))?;

            let mut cookie_hex = String::with_capacity(cookie.len() * 2);
            for b in &cookie {
                write!(&mut cookie_hex, "{:02X}", b).expect("infallible write");
            }

            let resp = client.send_line(&format!("AUTHENTICATE {}", cookie_hex))?;
            if !is_ok(&resp) {
                bail!("AUTHENTICATE failed: {resp:?}");
            }
        } else if auth_methods.null {
            // NULL auth explicitly allowed by Tor (rare). Authenticate with empty argument.
            let resp = client.send_line("AUTHENTICATE")?;
            if !is_ok(&resp) {
                bail!("AUTHENTICATE (NULL) failed: {resp:?}");
            }
        } else {
            bail!("No supported AUTH methods from Tor (need COOKIE/SAFECOOKIE/NULL).");
        }

        Ok(client)
    }

    /// Send a command and read a multi-line response until a terminating 250/550 line.
    pub(crate) fn cmd_multi(&mut self, cmd: &str) -> Result<Vec<String>> {
        self.write_line(cmd)?;
        let mut lines = Vec::new();
        loop {
            let mut line = String::new();
            self.r.read_line(&mut line)?;
            if line.is_empty() {
                break;
            }
            let trimmed = line.trim_end().to_string();
            let done = trimmed.starts_with("250 OK")
                || trimmed.starts_with("250 ")
                || trimmed.starts_with("550");
            lines.push(trimmed);
            if done {
                break;
            }
        }
        Ok(lines)
    }

    /// Send a command and read a single-line response.
    fn send_line(&mut self, cmd: &str) -> Result<String> {
        self.write_line(cmd)?;
        let mut line = String::new();
        self.r.read_line(&mut line)?;
        Ok(line)
    }

    /// Write a command line with CRLF.
    fn write_line(&mut self, cmd: &str) -> Result<()> {
        let mut w = self.s.try_clone()?;
        w.write_all(cmd.as_bytes())?;
        w.write_all(b"\r\n")?;
        w.flush()?;
        Ok(())
    }
}

fn is_ok(line: &str) -> bool {
    line.starts_with("250 OK")
}

/// Parsed bits we care about from PROTOCOLINFO.
struct AuthMethods {
    cookie: bool,
    safecookie: bool,
    null: bool,
}

/// Extract AUTH methods and COOKIEFILE path (if present) from PROTOCOLINFO lines.
fn parse_protocolinfo(lines: &[String]) -> (AuthMethods, Option<String>) {
    let mut methods = AuthMethods {
        cookie: false,
        safecookie: false,
        null: false,
    };
    let mut cookiefile: Option<String> = None;

    for l in lines {
        if let Some(auth_idx) = l.find("METHODS=") {
            let meth = &l[auth_idx + "METHODS=".len()..];
            let meth = meth.split_whitespace().next().unwrap_or("");
            for m in meth.split(',') {
                let m = m.trim().to_ascii_uppercase();
                if m == "COOKIE" { methods.cookie = true; }
                if m == "SAFECOOKIE" { methods.safecookie = true; }
                if m == "NULL" { methods.null = true; }
            }
        }
        if let Some(cf_idx) = l.find("COOKIEFILE=") {
            let rest = &l[cf_idx + "COOKIEFILE=".len()..];
            if let Some(start) = rest.find('"') {
                if let Some(end) = rest[start + 1..].find('"') {
                    cookiefile = Some(rest[start + 1..start + 1 + end].to_string());
                    continue;
                }
            }
            let mut part = rest.trim();
            if let Some(space) = part.find(char::is_whitespace) {
                part = &part[..space];
            }
            cookiefile = Some(part.trim_matches('"').to_string());
        }
    }
    (methods, cookiefile)
}
