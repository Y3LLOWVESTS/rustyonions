//! RO:WHAT — Micronode CLI surface (shape only).
//! RO:WHY  — Give micronode a stable CLI shape (`serve`, `check`) without
//!           committing yet to any particular argument parser crate.
//! RO:INTERACTS — `main.rs` can later call `Cli::from_env()` instead of
//!                hard-coding config/env logic.
//! RO:INVARIANTS —
//!     - All types here are `pub` so `dead_code` does not fire when the CLI
//!       is not yet wired into `main.rs`.
//!     - No external dependencies (no `clap`/`argh` yet); safe to evolve later.
//!     RO:TEST — Exercised by `tests/cli_smoke.rs`.

/// High-level profile for Micronode behavior.
///
/// This is intentionally coarse; config/env can refine details later.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Profile {
    /// Developer profile: dev routes enabled, in-memory storage, verbose logs.
    #[default]
    Dev,
    /// Amnesia-first profile: prefer non-persistent storage where possible.
    Amnesia,
    /// Durable profile: persistent storage (sled or remote CAS) is allowed.
    Durable,
}

/// Options for the `serve` subcommand.
#[derive(Debug, Clone)]
pub struct ServeOpts {
    /// Optional path to a config file; if `None`, Micronode will fall back
    /// to its default config discovery (env vars, default paths, etc.).
    pub config_path: Option<String>,
    /// Optional bind override for the HTTP listener (e.g., "127.0.0.1:5310").
    pub bind_addr: Option<String>,
    /// High-level runtime profile (dev/amnesia/durable).
    pub profile: Profile,
    /// Whether to expose dev-only routes (e.g. `/dev/echo`).
    pub dev_routes: bool,
}

impl Default for ServeOpts {
    fn default() -> Self {
        Self { config_path: None, bind_addr: None, profile: Profile::default(), dev_routes: true }
    }
}

/// Supported Micronode CLI commands.
///
/// More subcommands (e.g., `smoke`, `diag`) can be added without breaking
/// the existing API surface.
#[derive(Debug, Clone)]
pub enum Command {
    /// Run the Micronode HTTP server.
    Serve(ServeOpts),
    /// Validate config and exit (non-zero status on error).
    Check {
        /// Optional path to a config file.
        config_path: Option<String>,
    },
}

/// Top-level CLI representation.
///
/// For now we provide only a very small API:
/// - `Cli::from_env()` to construct a baseline value.
/// - `command()` accessor to drive dispatch in `main.rs` later.
#[derive(Debug, Clone)]
pub struct Cli {
    pub cmd: Command,
}

impl Cli {
    /// Construct a CLI representation from the environment.
    ///
    /// Foundation cut:
    /// - Ignores actual `std::env::args()` for now.
    /// - Always returns `Command::Serve(ServeOpts::default())`.
    ///
    /// This gives us a stable type and test surface; later we can replace
    /// the body with a proper parser (e.g., `clap`) without changing call
    /// sites or tests.
    pub fn from_env() -> Self {
        // Placeholder behavior: default to `serve` with default options.
        Self { cmd: Command::Serve(ServeOpts::default()) }
    }

    /// Borrow the parsed command.
    pub fn command(&self) -> &Command {
        &self.cmd
    }
}
