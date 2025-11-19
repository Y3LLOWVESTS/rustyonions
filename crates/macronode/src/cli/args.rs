//! RO:WHAT — Macronode CLI command/option types.
//! RO:WHY  — Keep the CLI surface explicit and testable without tying
//!           directly to a particular parsing crate.
//! RO:INVARIANTS —
//!   - `Cli::parse()` is a tiny, predictable parser over `std::env::args`.
//!   - Unknown commands fall back to `run` with a warning.
//!   - For `run`, we parse a small subset of flags by hand.

#[derive(Debug, Clone)]
pub enum Command {
    /// Run the Macronode host (admin HTTP + services).
    Run(RunOpts),
    /// Print version/build information and exit.
    Version,
    /// Validate environment/config and exit.
    Check,
    /// Print effective redacted config.
    ConfigPrint,
    /// Validate a supplied config file without starting the node.
    ConfigValidate,
    /// Run diagnostics bundle (fs/dns/time drift/ports).
    Doctor,
}

/// Options for the `run` subcommand.
///
/// NOTE: Fields are intentionally conservative for now; we keep them
/// around so the CLI surface is stable while we gradually implement
/// overlays. `#[allow(dead_code)]` keeps clippy happy under `-D warnings`
/// until all fields are used.
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct RunOpts {
    /// Optional path to a config file (`--config`).
    pub config_path: Option<String>,
    /// Optional bind override for admin HTTP (`--http-addr`).
    pub http_addr: Option<String>,
    /// Optional bind override for metrics (`--metrics-addr`).
    pub metrics_addr: Option<String>,
    /// Optional log level override (`--log-level`).
    pub log_level: Option<String>,
    /// Optional amnesia flag (`--amnesia`).
    pub amnesia: Option<bool>,
}

impl RunOpts {
    /// Parse flags for the `run` command from a slice of arguments.
    ///
    /// Supported flags (MVP):
    ///   --config PATH
    ///   --http-addr ADDR
    ///   --log-level LEVEL
    ///
    /// Unknown flags are ignored with a warning.
    pub fn from_args(args: &[String]) -> Self {
        let mut opts = RunOpts::default();
        let mut i = 0;

        while i < args.len() {
            match args[i].as_str() {
                "--config" => {
                    if let Some(val) = args.get(i + 1) {
                        opts.config_path = Some(val.clone());
                        i += 1;
                    } else {
                        eprintln!("macronode: --config requires a path argument");
                    }
                }
                "--http-addr" => {
                    if let Some(val) = args.get(i + 1) {
                        opts.http_addr = Some(val.clone());
                        i += 1;
                    } else {
                        eprintln!("macronode: --http-addr requires an address argument");
                    }
                }
                "--log-level" => {
                    if let Some(val) = args.get(i + 1) {
                        opts.log_level = Some(val.clone());
                        i += 1;
                    } else {
                        eprintln!("macronode: --log-level requires a level argument");
                    }
                }
                "--metrics-addr" => {
                    if let Some(val) = args.get(i + 1) {
                        opts.metrics_addr = Some(val.clone());
                        i += 1;
                    } else {
                        eprintln!("macronode: --metrics-addr requires an address argument");
                    }
                }
                "--amnesia" => {
                    // For now we accept `--amnesia` as a bare flag and treat it as true.
                    opts.amnesia = Some(true);
                }
                other => {
                    // Ignore unknown flags for now, but let the operator know.
                    if other.starts_with('-') {
                        eprintln!(
                            "macronode: ignoring unknown flag `{other}` on `run` (see README CLI section)"
                        );
                    }
                }
            }

            i += 1;
        }

        opts
    }
}

/// Top-level CLI wrapper.
#[derive(Debug, Clone)]
pub struct Cli {
    pub cmd: Command,
}

impl Cli {
    /// Parse CLI arguments into a `Cli` value.
    ///
    /// Today we:
    ///   - Look at the first positional argument to decide the subcommand.
    ///   - For `run`, parse a small set of flags from the remaining args.
    pub fn parse() -> Self {
        let mut args = std::env::args().skip(1);
        let sub = args.next();

        let cmd = match sub.as_deref() {
            None => {
                // No subcommand; treat as `run` with default options.
                Command::Run(RunOpts::default())
            }
            Some("run") => {
                let rest: Vec<String> = args.collect();
                let opts = RunOpts::from_args(&rest);
                Command::Run(opts)
            }
            Some("version") | Some("--version") | Some("-V") => Command::Version,
            Some("check") => Command::Check,
            Some("config") => match args.next().as_deref() {
                Some("print") => Command::ConfigPrint,
                Some("validate") => Command::ConfigValidate,
                other => {
                    eprintln!(
                        "macronode: expected `config print` or `config validate`, got {:?}; \
                         defaulting to `config print`",
                        other
                    );
                    Command::ConfigPrint
                }
            },
            Some("doctor") => Command::Doctor,
            Some(other) => {
                eprintln!(
                    "macronode: unknown command `{other}`, defaulting to `run` (see README CLI section)"
                );
                Command::Run(RunOpts::default())
            }
        };

        Cli { cmd }
    }
}
