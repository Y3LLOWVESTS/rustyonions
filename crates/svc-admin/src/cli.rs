use crate::config::Config;
use anyhow::Result;

/// Parse CLI arguments and environment variables to produce a Config.
///
/// Dev-preview behavior:
/// - No real CLI flags yet; we simply delegate to `Config::load()`,
///   which reads env + defaults.
///
/// Future (per CONFIG.MD):
/// - Add `clap`-based parsing for:
///     * --config / -c (TOML path)
///     * --bind-addr / --metrics-addr overrides
///     * --auth-mode, etc.
/// - Precedence: CLI → env → file → defaults.
pub fn parse_args() -> Result<Config> {
    Ok(Config::load()?)
}
