//! RO:WHAT — Macronode CLI surface and entrypoint.
//! RO:WHY  — Provide a stable operator-facing CLI (`run`, `version`, `check`,
//!           `config print|validate`, `doctor`) without committing to a
//!           specific argument parser crate yet.
//! RO:INVARIANTS —
//!   - Parsing is intentionally minimal but deterministic.
//!   - All subcommands return `errors::Result<()>` so main can stay boring.

pub mod args;
pub mod check;
pub mod config_print;
pub mod config_validate;
pub mod doctor;
pub mod run;
pub mod version;

use crate::errors::Result;
pub use args::{Cli, Command, RunOpts};

/// Parse CLI args and dispatch to the selected subcommand.
pub async fn entrypoint() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Command::Run(opts) => run::run(opts).await,
        Command::Version => {
            version::run();
            Ok(())
        }
        Command::Check => check::run(),
        Command::ConfigPrint => config_print::run(),
        Command::ConfigValidate => config_validate::run(),
        Command::Doctor => doctor::run(),
    }
}
