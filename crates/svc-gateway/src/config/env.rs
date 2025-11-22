//! Env-based configuration overlay for `svc-gateway`.
//! RO:WHAT  Read a small set of `SVC_GATEWAY_*` variables and overlay `Config::default()`.
//! RO:WHY   Keep defaults safe while allowing simple overrides for dev/tests.

use super::Config;

/// Load configuration from environment variables on top of `Config::default()`.
///
/// Currently this only supports a small subset of fields (notably the omnigate
/// app-plane upstream URL).
///
/// # Errors
///
/// Returns an error if any environment variable that is present but required
/// to be non-empty fails validation.
pub fn load() -> anyhow::Result<Config> {
    let mut cfg = Config::default();

    // Optional override for the omnigate app-plane base URL.
    //
    // Example:
    //   SVC_GATEWAY_OMNIGATE_BASE_URL=http://127.0.0.1:9090
    if let Ok(url) = std::env::var("SVC_GATEWAY_OMNIGATE_BASE_URL") {
        let trimmed = url.trim();
        if trimmed.is_empty() {
            anyhow::bail!("SVC_GATEWAY_OMNIGATE_BASE_URL must not be empty");
        }
        // Avoid assigning a new String; reuse the existing allocation.
        trimmed.clone_into(&mut cfg.upstreams.omnigate_base_url);
    }

    Ok(cfg)
}
