//! RO:WHAT — Micronode CLI surface.
//! RO:WHY  — Provide stable local commands for CrabLink/user-node lifecycle
//!           integration without adding a parser dependency.
//! RO:INVARIANTS —
//!     - `serve` remains the default command.
//!     - `check` validates real config and exits.
//!     - `status` renders the same local passive-runtime contract used by
//!       the admin API, not a fake success string.
//! RO:TEST — Exercised by `tests/cli_smoke.rs`.

use std::{error::Error as StdError, fmt};

pub mod run;

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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServeOpts {
    /// Optional path to a config file; if `None`, Micronode will fall back
    /// to its default config discovery.
    pub config_path: Option<String>,
    /// Optional bind override for the HTTP listener, e.g. "127.0.0.1:5310".
    pub bind_addr: Option<String>,
    /// High-level runtime profile.
    pub profile: Profile,
    /// Effective dev-route preference exposed for tests/UX.
    pub dev_routes: bool,
    /// Whether the CLI explicitly requested a dev-routes override.
    ///
    /// This avoids changing runtime behavior merely because `ServeOpts::default`
    /// is DX-friendly.
    pub dev_routes_override: Option<bool>,
}

impl Default for ServeOpts {
    fn default() -> Self {
        Self {
            config_path: None,
            bind_addr: None,
            profile: Profile::default(),
            dev_routes: true,
            dev_routes_override: None,
        }
    }
}

/// Supported Micronode CLI commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Run the Micronode HTTP server.
    Serve(ServeOpts),
    /// Validate config and exit.
    Check {
        /// Optional path to a config file.
        config_path: Option<String>,
    },
    /// Print local passive user-node status and exit.
    Status {
        /// Optional path to a config file.
        config_path: Option<String>,
    },
    /// Print help and exit.
    Help,
    /// Parser error captured for main.rs so it can print usage and exit.
    UsageError { message: String },
}

/// Top-level CLI representation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cli {
    pub cmd: Command,
}

impl Cli {
    /// Parse CLI args from the process environment.
    ///
    /// On parser failure, return a `UsageError` command rather than panicking.
    pub fn from_env() -> Self {
        match Self::from_args(std::env::args().skip(1)) {
            Ok(cli) => cli,
            Err(err) => Self { cmd: Command::UsageError { message: err.to_string() } },
        }
    }

    /// Parse args without a parser dependency.
    ///
    /// Accepted shapes:
    /// - `micronode`
    /// - `micronode serve [--config PATH] [--bind ADDR] [--profile dev|amnesia|durable]`
    /// - `micronode check [--config PATH]`
    /// - `micronode status [--config PATH]`
    /// - `micronode --help`
    pub fn from_args<I, S>(args: I) -> Result<Self, CliParseError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let args: Vec<String> = args.into_iter().map(Into::into).collect();

        if args.is_empty() {
            return Ok(Self { cmd: Command::Serve(ServeOpts::default()) });
        }

        let cmd = args[0].as_str();
        match cmd {
            "serve" => Ok(Self { cmd: Command::Serve(parse_serve_opts(&args[1..])?) }),
            "check" => Ok(Self {
                cmd: Command::Check { config_path: parse_config_only_opts(&args[1..])? },
            }),
            "status" => Ok(Self {
                cmd: Command::Status { config_path: parse_config_only_opts(&args[1..])? },
            }),
            "help" | "-h" | "--help" => Ok(Self { cmd: Command::Help }),
            // Convenience: allow `micronode --bind 127.0.0.1:5311` as implicit serve.
            other if other.starts_with('-') => {
                Ok(Self { cmd: Command::Serve(parse_serve_opts(&args)?) })
            }
            other => Err(CliParseError::new(format!("unknown command: {other}"))),
        }
    }

    /// Borrow the parsed command.
    pub fn command(&self) -> &Command {
        &self.cmd
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliParseError {
    message: String,
}

impl CliParseError {
    fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl fmt::Display for CliParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.message.fmt(f)
    }
}

impl StdError for CliParseError {}

fn parse_serve_opts(args: &[String]) -> Result<ServeOpts, CliParseError> {
    let mut opts = ServeOpts::default();
    let mut i = 0usize;

    while i < args.len() {
        match args[i].as_str() {
            "-c" | "--config" => {
                i += 1;
                opts.config_path = Some(required_value(args, i, "--config")?);
            }
            "-b" | "--bind" => {
                i += 1;
                opts.bind_addr = Some(required_value(args, i, "--bind")?);
            }
            "--profile" => {
                i += 1;
                opts.profile = parse_profile(&required_value(args, i, "--profile")?)?;
            }
            "--dev-routes" => {
                opts.dev_routes = true;
                opts.dev_routes_override = Some(true);
            }
            "--no-dev-routes" => {
                opts.dev_routes = false;
                opts.dev_routes_override = Some(false);
            }
            "-h" | "--help" => {
                return Err(CliParseError::new("help is only valid as `micronode --help`"));
            }
            other => return Err(CliParseError::new(format!("unknown serve option: {other}"))),
        }

        i += 1;
    }

    Ok(opts)
}

fn parse_config_only_opts(args: &[String]) -> Result<Option<String>, CliParseError> {
    let mut config_path = None;
    let mut i = 0usize;

    while i < args.len() {
        match args[i].as_str() {
            "-c" | "--config" => {
                i += 1;
                config_path = Some(required_value(args, i, "--config")?);
            }
            other => return Err(CliParseError::new(format!("unknown option: {other}"))),
        }

        i += 1;
    }

    Ok(config_path)
}

fn required_value(args: &[String], i: usize, flag: &str) -> Result<String, CliParseError> {
    args.get(i)
        .cloned()
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| CliParseError::new(format!("missing value for {flag}")))
}

fn parse_profile(raw: &str) -> Result<Profile, CliParseError> {
    match raw.to_ascii_lowercase().as_str() {
        "dev" => Ok(Profile::Dev),
        "amnesia" => Ok(Profile::Amnesia),
        "durable" => Ok(Profile::Durable),
        other => Err(CliParseError::new(format!(
            "invalid profile {other:?}; expected dev, amnesia, or durable"
        ))),
    }
}

pub fn usage() -> &'static str {
    r#"Usage:
  micronode serve [--config PATH] [--bind ADDR] [--profile dev|amnesia|durable] [--dev-routes|--no-dev-routes]
  micronode check [--config PATH]
  micronode status [--config PATH]
  micronode --help

Commands:
  serve    Run the local Micronode HTTP server. This is the default.
  check    Load and validate config, then exit.
  status   Print local passive user-node status JSON, then exit.

Safety:
  User-node passive runtime requires loopback-only bind.
  CLI status does not claim wallet mutation, ledger mutation, or confirmed ROC.
"#
}
