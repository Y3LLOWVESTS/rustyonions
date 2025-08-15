use anyhow::{anyhow, Context, Result};
use std::fs;
use std::path::Path;

fn default_config_toml() -> String {
    r#"# Example config for RustyOnions
data_dir = ".data"
overlay_addr = "127.0.0.1:1777"      # TCP listener bind/target
dev_inbox_addr = "127.0.0.1:2888"
socks5_addr = "127.0.0.1:9050"       # Tor SOCKS5 proxy
tor_ctrl_addr = "127.0.0.1:9051"     # Tor control port
chunk_size = 65536
connect_timeout_ms = 5000
# Optional persistent HS private key file (used by `ronode serve --transport tor`)
# hs_key_file = ".data/hs_ed25519_key"
"#
    .to_string()
}

pub fn init(path: Option<&Path>) -> Result<()> {
    let out = path
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::path::PathBuf::from("config.toml"));
    let data = default_config_toml();
    if out.exists() {
        return Err(anyhow!("refusing to overwrite existing {}", out.display()));
    }
    fs::write(&out, data).with_context(|| format!("writing {}", out.display()))?;
    println!("Wrote {}", out.display());
    Ok(())
}
