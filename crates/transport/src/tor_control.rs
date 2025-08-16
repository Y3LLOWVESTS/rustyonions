// crates/transport/src/tor_control.rs
use anyhow::{anyhow, bail, Context, Result};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::path::Path;

fn send_line(stream: &mut TcpStream, line: &str) -> std::io::Result<()> {
    stream.write_all(line.as_bytes())?;
    stream.write_all(b"\r\n")?;
    Ok(())
}

fn read_until_done(stream: &mut TcpStream) -> std::io::Result<String> {
    let mut out = String::new();
    let mut reader = BufReader::new(stream);
    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line)?;
        if n == 0 {
            return Ok(out);
        }
        out.push_str(&line);
        if line.starts_with("250 ") { return Ok(out); }
        if line.starts_with('5')   { return Ok(out); }
    }
}

fn cookie_hex(p: &Path) -> Result<String> {
    let bytes = fs::read(p).with_context(|| format!("reading cookie file {:?}", p))?;
    Ok(bytes.iter().map(|b| format!("{:02X}", b)).collect())
}

/// Publish an ephemeral v3 onion and return "<56chars>.onion:1777".
pub fn publish_v3(ctrl_addr: &str, local_map: &str, cookie_file: Option<&str>) -> Result<String> {
    // Probe (optional)
    {
        let mut s = TcpStream::connect(ctrl_addr)
            .with_context(|| format!("connecting to Tor control at {}", ctrl_addr))?;
        s.set_nodelay(true)?;
        send_line(&mut s, "PROTOCOLINFO 1")?;
        let _ = read_until_done(&mut s)?;
    }

    // Auth & commands
    let mut s = TcpStream::connect(ctrl_addr)
        .with_context(|| format!("connecting to Tor control at {}", ctrl_addr))?;
    s.set_nodelay(true)?;

    if let Some(cookie_path) = cookie_file {
        let hex = cookie_hex(Path::new(cookie_path))?;
        send_line(&mut s, &format!("AUTHENTICATE {}", hex))?;
    } else {
        send_line(&mut s, "AUTHENTICATE")?; // NULL auth (only if Tor allows it)
    }

    send_line(&mut s, "GETINFO version")?;
    let reply = read_until_done(&mut s)?;
    if !reply.contains("250 OK") {
        bail!("Tor control AUTH failed:\n{}", reply);
    }

    // Map public 1777 -> local 127.0.0.1:1777
    send_line(&mut s, &format!("ADD_ONION NEW:ED25519-V3 Port={}", local_map))?;
    let reply = read_until_done(&mut s)?;
    if reply.starts_with('5') {
        bail!("ADD_ONION failed:\n{}", reply);
    }

    let sid = reply
        .lines()
        .find_map(|l| l.strip_prefix("250-ServiceID="))
        .ok_or_else(|| anyhow!("No ServiceID in Tor reply:\n{}", reply))?;

    Ok(format!("{}.onion:1777", sid))
}
