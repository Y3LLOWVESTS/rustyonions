use crate::config::Config;
use anyhow::Result;

/// Parse CLI arguments and environment variables to produce a Config.
pub fn parse_args() -> Result<Config> {
    // TODO: Use clap or similar for real CLI parsing.
    // For now, delegate directly to Config::load().
    Config::load()
}
