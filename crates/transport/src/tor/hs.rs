use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use super::ctrl::TorController;

/// Publish a v3 onion service (ephemeral).
/// - If `key_path` exists, reuse that key.
/// - Otherwise, request a new onion and optionally persist its private key.
pub async fn publish_v3(
    ctrl_addr: &str,
    ctrl_pw: &str,
    key_path: &Path,
    local_port: u16,
    public_port: u16,
    wait_secs: u64,
) -> Result<(String, String)> {
    // Connect & authenticate to Tor control
    let mut ctl = TorController::connect_and_auth(ctrl_addr.parse()?, ctrl_pw).await?;

    let (service_id, _private_key) = if key_path.exists() {
        // Existing key: reuse it
        let key_line = fs::read_to_string(key_path)
            .with_context(|| format!("reading HS key from {}", key_path.display()))?;
        ctl.add_onion_with_key(key_line.trim(), public_port, "127.0.0.1", local_port)
            .await?
    } else {
        // New onion: request NEW:ED25519-V3 and persist private key if returned
        let (sid, priv_line) = ctl
            .add_onion_new_with_host(public_port, "127.0.0.1", local_port, &[])
            .await?;
        if let Some(pk) = &priv_line {
            fs::write(key_path, pk)?;
        }
        (sid, priv_line)
    };

    // Wait for HS descriptor to be uploaded
    ctl.wait_hs_desc_uploaded(&service_id, wait_secs).await?;

    // Return onion hostname and service_id as String
    Ok((format!("{}.onion", service_id), service_id.to_string()))
}
