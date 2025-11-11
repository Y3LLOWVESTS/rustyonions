//! CLI parsing (flags are minimal for the first increment).

use std::path::PathBuf;

#[cfg(feature = "cli")]
use clap::Parser;

/// CLI flags (mirrors docs/CONFIG.md as we grow).
#[cfg_attr(feature = "cli", derive(Parser))]
#[derive(Clone, Debug, Default)]
#[cfg_attr(
    feature = "cli",
    command(name = "svc-edge", author, version, about = "svc-edge service")
)]
pub struct Cli {
    /// Path to the svc-edge TOML config file.
    #[cfg_attr(feature = "cli", arg(long = "config"))]
    pub config_path: Option<PathBuf>,
}

impl Cli {
    /// Parse CLI flags from the current process environment/argv.
    ///
    /// When the `cli` feature is disabled, returns defaults.
    #[cfg(feature = "cli")]
    pub fn parse_from_env() -> Self {
        <Self as clap::Parser>::parse()
    }

    /// Parse CLI flags from the current process environment/argv.
    ///
    /// When the `cli` feature is disabled, returns defaults.
    #[cfg(not(feature = "cli"))]
    pub fn parse_from_env() -> Self {
        Self::default()
    }
}
