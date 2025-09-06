use clap::Parser;
use std::path::PathBuf;

/// CLI for the gateway smoke test harness
#[derive(Parser, Debug, Clone)]
#[command(name="gwsmoke", about="RustyOnions gateway end-to-end smoke tester")]
pub struct Cli {
    /// Workspace root (where Cargo.toml and target/ live)
    #[arg(long, default_value = ".", value_hint=clap::ValueHint::DirPath)]
    pub root: PathBuf,

    /// Output bundle dir (.onions)
    #[arg(long, default_value = ".onions", value_hint=clap::ValueHint::DirPath)]
    pub out_dir: PathBuf,

    /// Index DB directory; if omitted a temp dir is created
    #[arg(long)]
    pub index_db: Option<PathBuf>,

    /// Bind address for HTTP gateway; if port 0, an ephemeral port is chosen
    #[arg(long, default_value = "127.0.0.1:0")]
    pub bind: String,

    /// Algo (if your tldctl supports it)
    #[arg(long, default_value = "blake3")]
    pub algo: String,

    /// Keep temp dir (logs, sockets) on success
    #[arg(long)]
    pub keep_tmp: bool,

    /// Maximum seconds to wait for gateway TCP readiness
    #[arg(long = "http-wait-sec", visible_alias = "http_wait_sec", default_value_t = 20u64)]
    pub http_wait_sec: u64,

    /// Log dir (inside tmp by default)
    #[arg(long)]
    pub log_dir: Option<PathBuf>,

    /// Build first (cargo build -p <bins>)
    #[arg(long)]
    pub build: bool,

    /// Extra environment to pass to *all* child processes (k=v, repeatable)
    #[arg(long)]
    pub env: Vec<String>,

    /// Stream child process logs to stdout while also saving to files
    #[arg(long)]
    pub stream: bool,

    /// Override RUST_LOG used for services/gateway (e.g. trace or fine-grained filters)
    #[arg(long, default_value = "info,svc_index=debug,svc_storage=debug,svc_overlay=debug,gateway=debug")]
    pub rust_log: String,
}
