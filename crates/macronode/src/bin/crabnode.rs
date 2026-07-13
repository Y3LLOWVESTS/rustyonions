//! RO:WHAT — CLI-first CrabLink service-node operator surface.
//! RO:WHY — BUILD_PLAN_Z requires headless service-node management without svc-admin UI.
//! RO:INTERACTS — macronode admin HTTP plane and svc-admin first-run setup HTTP surface.
//! RO:INVARIANTS — loopback-first; no fake admin success; UI is optional, CLI remains available.
//! RO:SECURITY — refuses non-loopback admin targets unless explicitly allowed; secrets come from env.
//! RO:TEST — integration: crates/macronode/tests/crabnode_cli.rs, crabnode_policy_cli.rs.

#[path = "crabnode/policy.rs"]
mod crabnode_policy;

use std::{
    env, fmt,
    io::{Read, Write},
    net::{SocketAddr, TcpStream, ToSocketAddrs},
    path::PathBuf,
    process::ExitCode,
    time::Duration,
};

use crabnode_policy::ModerationAction;
use ron_policy::B3Id;
use ron_proto::asset::AssetKind;
use svc_storage::persistence_catalog::MAX_PERSISTENCE_REVIEW_ITEMS;

const DEFAULT_ADMIN_URL: &str = "http://127.0.0.1:8080";
const DEFAULT_SVC_ADMIN_URL: &str = "http://127.0.0.1:5300";
const DEFAULT_PERSISTENCE_PENDING_LIMIT: usize = 100;

fn main() -> ExitCode {
    match run(env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("crabnode: {err}");
            ExitCode::from(err.exit_code())
        }
    }
}

fn run(args: Vec<String>) -> Result<(), CrabnodeError> {
    if args.is_empty() || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_help();
        return Ok(());
    }

    let opts = parse_args(args)?;
    match opts.command {
        Command::Status => query_or_dry_run(&opts, "/api/v1/status", "status"),
        Command::Ready => query_or_dry_run(&opts, "/readyz", "ready"),
        Command::Version => query_or_dry_run(&opts, "/version", "version"),
        Command::PolicyStatus => crabnode_policy::status(opts.policy_file.as_deref()),
        Command::Moderation(ref action) => {
            crabnode_policy::mutate(opts.policy_file.as_deref(), opts.dry_run, action)
        }
        Command::Prune(ref object) => prune_or_dry_run(&opts, object),
        Command::Persistence(ref command) => persistence_or_dry_run(&opts, command),
        Command::Admin(AdminCommand::EnableWeb) => {
            admin_post_or_dry_run(&opts, "enable-web", "/api/v1/admin/ui/enable")
        }
        Command::Admin(AdminCommand::DisableWeb) => {
            admin_post_or_dry_run(&opts, "disable-web", "/api/v1/admin/ui/disable")
        }
        Command::Admin(AdminCommand::SetupToken) => {
            admin_post_or_dry_run(&opts, "setup-token", "/api/v1/admin/setup-token")
        }
        Command::Admin(AdminCommand::CreateUser(ref username)) => {
            create_user_or_dry_run(&opts, username)
        }
        Command::Rewards(RewardsCommand::Show) => rewards_show_or_dry_run(&opts),
        Command::Rewards(RewardsCommand::Bind(ref address)) => {
            rewards_bind_or_dry_run(&opts, address)
        }
        Command::Rewards(RewardsCommand::Rotate(ref address)) => {
            rewards_rotate_or_dry_run(&opts, address)
        }
    }
}

#[derive(Debug)]
struct Options {
    admin_url: String,
    svc_admin_url: String,
    dry_run: bool,
    allow_non_loopback: bool,
    policy_file: Option<PathBuf>,
    command: Command,
}

#[derive(Debug)]
enum Command {
    Status,
    Ready,
    Version,
    PolicyStatus,
    Moderation(ModerationAction),
    Prune(String),
    Persistence(PersistenceCommand),
    Admin(AdminCommand),
    Rewards(RewardsCommand),
}

#[derive(Debug)]
enum PersistenceCommand {
    Register { object: String, asset_kind: String },
    Status(String),
    Pending(usize),
    Submit(String),
    Approve(String),
    Pin(String),
    Unpin(String),
    Reject(String),
}

#[derive(Debug)]
enum AdminCommand {
    EnableWeb,
    DisableWeb,
    SetupToken,
    CreateUser(String),
}

#[derive(Debug)]
enum RewardsCommand {
    Show,
    Bind(String),
    Rotate(String),
}

fn parse_args(args: Vec<String>) -> Result<Options, CrabnodeError> {
    let mut admin_url =
        env::var("CRABNODE_ADMIN_URL").unwrap_or_else(|_| DEFAULT_ADMIN_URL.to_string());
    let mut svc_admin_url =
        env::var("CRABNODE_SVC_ADMIN_URL").unwrap_or_else(|_| DEFAULT_SVC_ADMIN_URL.to_string());
    let mut dry_run = false;
    let mut allow_non_loopback = false;
    let mut policy_file = None;
    let mut positional = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--admin-url" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| CrabnodeError::usage("--admin-url requires a URL argument"))?;
                admin_url = value.clone();
                i += 2;
            }
            "--svc-admin-url" => {
                let value = args.get(i + 1).ok_or_else(|| {
                    CrabnodeError::usage("--svc-admin-url requires a URL argument")
                })?;
                svc_admin_url = value.clone();
                i += 2;
            }
            "--policy-file" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| CrabnodeError::usage("--policy-file requires a path"))?;
                let trimmed = value.trim();

                if trimmed.is_empty() {
                    return Err(CrabnodeError::usage("--policy-file cannot be empty"));
                }

                policy_file = Some(PathBuf::from(trimmed));
                i += 2;
            }
            "--dry-run" => {
                dry_run = true;
                i += 1;
            }
            "--allow-non-loopback" => {
                allow_non_loopback = true;
                i += 1;
            }
            arg if arg.starts_with("--") => {
                return Err(CrabnodeError::usage(format!("unknown option {arg:?}")));
            }
            _ => {
                positional.push(args[i].clone());
                i += 1;
            }
        }
    }

    let command = parse_command(&positional)?;
    Ok(Options {
        admin_url,
        svc_admin_url,
        dry_run,
        allow_non_loopback,
        policy_file,
        command,
    })
}

fn parse_command(positional: &[String]) -> Result<Command, CrabnodeError> {
    match positional {
        [cmd] if cmd == "status" => Ok(Command::Status),
        [cmd] if cmd == "ready" => Ok(Command::Ready),
        [cmd] if cmd == "version" => Ok(Command::Version),
        [policy, sub] if policy == "policy" && sub == "status" => Ok(Command::PolicyStatus),
        [action, object] if action == "block" => {
            Ok(Command::Moderation(ModerationAction::Block(object.clone())))
        }
        [action, object] if action == "unblock" => Ok(Command::Moderation(
            ModerationAction::Unblock(object.clone()),
        )),
        [action, object] if action == "allow" => {
            Ok(Command::Moderation(ModerationAction::Allow(object.clone())))
        }
        [action, object] if action == "remove-allow" => Ok(Command::Moderation(
            ModerationAction::RemoveAllow(object.clone()),
        )),
        [action, object] if action == "quarantine" => Ok(Command::Moderation(
            ModerationAction::Quarantine(object.clone()),
        )),
        [action, object] if action == "release-quarantine" => Ok(Command::Moderation(
            ModerationAction::ReleaseQuarantine(object.clone()),
        )),
        [action, object] if action == "prune" => Ok(Command::Prune(object.clone())),
        [persistence, sub, object, asset_kind]
            if persistence == "persistence" && sub == "register" =>
        {
            Ok(Command::Persistence(PersistenceCommand::Register {
                object: object.clone(),
                asset_kind: asset_kind.clone(),
            }))
        }
        [persistence, sub, object] if persistence == "persistence" && sub == "status" => Ok(
            Command::Persistence(PersistenceCommand::Status(object.clone())),
        ),
        [persistence, sub] if persistence == "persistence" && sub == "pending" => {
            Ok(Command::Persistence(PersistenceCommand::Pending(
                DEFAULT_PERSISTENCE_PENDING_LIMIT,
            )))
        }
        [persistence, sub, limit] if persistence == "persistence" && sub == "pending" => {
            Ok(Command::Persistence(PersistenceCommand::Pending(
                parse_persistence_pending_limit(limit)?,
            )))
        }
        [persistence, sub, object] if persistence == "persistence" && sub == "submit" => Ok(
            Command::Persistence(PersistenceCommand::Submit(object.clone())),
        ),
        [persistence, sub, object] if persistence == "persistence" && sub == "approve" => Ok(
            Command::Persistence(PersistenceCommand::Approve(object.clone())),
        ),
        [persistence, sub, object] if persistence == "persistence" && sub == "pin" => Ok(
            Command::Persistence(PersistenceCommand::Pin(object.clone())),
        ),
        [persistence, sub, object] if persistence == "persistence" && sub == "unpin" => Ok(
            Command::Persistence(PersistenceCommand::Unpin(object.clone())),
        ),
        [persistence, sub, object] if persistence == "persistence" && sub == "reject" => Ok(
            Command::Persistence(PersistenceCommand::Reject(object.clone())),
        ),
        [admin, sub] if admin == "admin" && sub == "enable-web" => {
            Ok(Command::Admin(AdminCommand::EnableWeb))
        }
        [admin, sub] if admin == "admin" && sub == "disable-web" => {
            Ok(Command::Admin(AdminCommand::DisableWeb))
        }
        [admin, sub] if admin == "admin" && sub == "setup-token" => {
            Ok(Command::Admin(AdminCommand::SetupToken))
        }
        [admin, sub, username] if admin == "admin" && sub == "create-user" => {
            Ok(Command::Admin(AdminCommand::CreateUser(username.clone())))
        }
        [rewards, sub] if rewards == "rewards" && sub == "show" => {
            Ok(Command::Rewards(RewardsCommand::Show))
        }
        [rewards, sub, address] if rewards == "rewards" && sub == "bind" => {
            Ok(Command::Rewards(RewardsCommand::Bind(address.clone())))
        }
        [rewards, sub, address] if rewards == "rewards" && sub == "rotate" => {
            Ok(Command::Rewards(RewardsCommand::Rotate(address.clone())))
        }
        [] => Err(CrabnodeError::usage("missing command")),
        other => Err(CrabnodeError::usage(format!(
            "unknown command path: {}",
            other.join(" ")
        ))),
    }
}

fn parse_persistence_pending_limit(raw: &str) -> Result<usize, CrabnodeError> {
    let limit = raw.parse::<usize>().map_err(|_| {
        CrabnodeError::usage("persistence pending limit must be an unsigned integer")
    })?;

    if limit == 0 || limit > MAX_PERSISTENCE_REVIEW_ITEMS {
        return Err(CrabnodeError::usage(format!(
            "persistence pending limit must be within 1..={MAX_PERSISTENCE_REVIEW_ITEMS}"
        )));
    }

    Ok(limit)
}

fn persistence_or_dry_run(
    opts: &Options,
    command: &PersistenceCommand,
) -> Result<(), CrabnodeError> {
    match command {
        PersistenceCommand::Register { object, asset_kind } => {
            let object = parse_persistence_object(object)?;
            let asset_kind = parse_persistence_asset_kind(asset_kind)?;

            let target = ParsedAdminUrl::parse(&opts.admin_url, opts.allow_non_loopback)?;
            let path = "/api/v1/persistence/register";

            let body = serde_json::json!({
                "object": object.as_str(),
                "assetKind": asset_kind.suffix(),
            })
            .to_string();

            if opts.dry_run {
                println!(
                    "crabnode persistence register {object} {asset_kind}:                      would POST {}{}; candidate begins amnesia-first;                      durable bytes written=false; no wallet or ledger mutation",
                    target.display_base(),
                    path
                );

                return Ok(());
            }

            let response = http_post_json(&target, path, &body)?;
            println!("{response}");
            Ok(())
        }

        PersistenceCommand::Status(object) => {
            let object = parse_persistence_object(object)?;
            let target = ParsedAdminUrl::parse(&opts.admin_url, opts.allow_non_loopback)?;
            let path = format!("/api/v1/persistence/status/{object}");

            if opts.dry_run {
                println!(
                    "crabnode persistence status {object}: would query {}{};                      metadata and eligibility only; no durability claim",
                    target.display_base(),
                    path
                );

                return Ok(());
            }

            let response = http_get(&target, &path)?;
            println!("{response}");
            Ok(())
        }

        PersistenceCommand::Pending(limit) => {
            let target = ParsedAdminUrl::parse(&opts.admin_url, opts.allow_non_loopback)?;
            let path = format!("/api/v1/persistence/pending?limit={limit}");

            if opts.dry_run {
                println!(
                    "crabnode persistence pending {limit}: would query {}{};                      bounded process-local review metadata only",
                    target.display_base(),
                    path
                );

                return Ok(());
            }

            let response = http_get(&target, &path)?;
            println!("{response}");
            Ok(())
        }

        PersistenceCommand::Submit(object) => {
            persistence_object_post_or_dry_run(opts, "submit", "/api/v1/persistence/submit", object)
        }

        PersistenceCommand::Approve(object) => persistence_object_post_or_dry_run(
            opts,
            "approve",
            "/api/v1/persistence/approve",
            object,
        ),

        PersistenceCommand::Pin(object) => {
            persistence_object_post_or_dry_run(opts, "pin", "/api/v1/persistence/pin", object)
        }

        PersistenceCommand::Unpin(object) => {
            persistence_object_post_or_dry_run(opts, "unpin", "/api/v1/persistence/unpin", object)
        }

        PersistenceCommand::Reject(object) => {
            persistence_object_post_or_dry_run(opts, "reject", "/api/v1/persistence/reject", object)
        }
    }
}

fn persistence_object_post_or_dry_run(
    opts: &Options,
    action: &'static str,
    path: &'static str,
    object: &str,
) -> Result<(), CrabnodeError> {
    let object = parse_persistence_object(object)?;
    let target = ParsedAdminUrl::parse(&opts.admin_url, opts.allow_non_loopback)?;

    let body = serde_json::json!({
        "object": object.as_str(),
    })
    .to_string();

    if opts.dry_run {
        println!(
            "crabnode persistence {action} {object}: would POST {}{};              changes persistence-review metadata only;              durable bytes written=false; no wallet or ledger mutation",
            target.display_base(),
            path
        );

        return Ok(());
    }

    let response = http_post_json(&target, path, &body)?;
    println!("{response}");
    Ok(())
}

fn parse_persistence_asset_kind(raw: &str) -> Result<AssetKind, CrabnodeError> {
    let normalized = raw.trim().to_ascii_lowercase();

    if normalized.is_empty() {
        return Err(CrabnodeError::usage(
            "invalid persistence asset kind: value cannot be empty",
        ));
    }

    serde_json::from_value::<AssetKind>(serde_json::Value::String(normalized))
        .map_err(|err| CrabnodeError::usage(format!("invalid persistence asset kind: {err}")))
}

fn parse_persistence_object(raw: &str) -> Result<B3Id, CrabnodeError> {
    raw.parse::<B3Id>()
        .map_err(|err| CrabnodeError::usage(format!("invalid persistence object: {err}")))
}

fn query_or_dry_run(opts: &Options, path: &str, label: &str) -> Result<(), CrabnodeError> {
    let target = ParsedAdminUrl::parse(&opts.admin_url, opts.allow_non_loopback)?;

    if opts.dry_run {
        println!(
            "crabnode {label}: would query {}{}",
            target.display_base(),
            path
        );
        return Ok(());
    }

    let body = http_get(&target, path)?;
    println!("{body}");
    Ok(())
}

fn admin_post_or_dry_run(
    opts: &Options,
    action: &'static str,
    path: &'static str,
) -> Result<(), CrabnodeError> {
    let target = ParsedAdminUrl::parse(&opts.admin_url, opts.allow_non_loopback)?;

    if opts.dry_run {
        println!(
            "crabnode admin {action}: would POST {}{}",
            target.display_base(),
            path
        );
        return Ok(());
    }

    let body = http_post(&target, path)?;
    println!("{body}");
    Ok(())
}

fn create_user_or_dry_run(opts: &Options, username: &str) -> Result<(), CrabnodeError> {
    let target = ParsedAdminUrl::parse(&opts.svc_admin_url, opts.allow_non_loopback)?;
    let path = "/api/setup/create-admin";

    if opts.dry_run {
        println!(
            "crabnode admin create-user {username}: would POST {}{} using CRABNODE_USER_PASSWORD and CRABNODE_SETUP_TOKEN; secrets are not printed",
            target.display_base(),
            path
        );
        return Ok(());
    }

    let password = required_secret_env(
        "CRABNODE_USER_PASSWORD",
        "admin create-user requires CRABNODE_USER_PASSWORD; refusing to accept passwords as CLI args",
    )?;
    let setup_token = required_secret_env(
        "CRABNODE_SETUP_TOKEN",
        "admin create-user requires CRABNODE_SETUP_TOKEN from `crabnode admin setup-token`",
    )?;

    let body = format!(
        "{{\"username\":\"{}\",\"password\":\"{}\",\"setupToken\":\"{}\"}}",
        json_escape(username),
        json_escape(&password),
        json_escape(&setup_token)
    );

    let response = http_post_json(&target, path, &body)?;
    println!("{response}");
    Ok(())
}

fn prune_or_dry_run(opts: &Options, object_text: &str) -> Result<(), CrabnodeError> {
    // Validate before resolving or contacting the admin endpoint. Invalid
    // object text must never result in a mutation request.
    let object = object_text
        .parse::<B3Id>()
        .map_err(|err| CrabnodeError::usage(format!("invalid prune object: {err}")))?;

    let target = ParsedAdminUrl::parse(&opts.admin_url, opts.allow_non_loopback)?;
    let path = "/api/v1/moderation/prune";

    let body = serde_json::json!({
        "object": object.as_str(),
    })
    .to_string();

    if opts.dry_run {
        println!(
            "crabnode prune {object}: would POST {}{}; \
             would attempt exact local byte deletion, configured-node \
             provider withdrawal, and exact provider-cache invalidation; \
             network propagation=false; resolve cache invalidation=false; \
             manifest pointer removal=false; no wallet or ledger mutation; \
             no reward finality",
            target.display_base(),
            path
        );

        return Ok(());
    }

    let response = http_post_json(&target, path, &body)?;
    println!("{response}");
    Ok(())
}

fn rewards_show_or_dry_run(opts: &Options) -> Result<(), CrabnodeError> {
    let target = ParsedAdminUrl::parse(&opts.admin_url, opts.allow_non_loopback)?;
    let path = "/api/v1/rewards/status";

    if opts.dry_run {
        println!(
            "crabnode rewards show: would query {}{}; confirmed ROC still comes from wallet/ledger receipts",
            target.display_base(),
            path
        );
        return Ok(());
    }

    let body = http_get(&target, path)?;
    println!("{body}");
    Ok(())
}

fn rewards_bind_or_dry_run(opts: &Options, address: &str) -> Result<(), CrabnodeError> {
    validate_crablink_address(address)?;
    let target = ParsedAdminUrl::parse(&opts.admin_url, opts.allow_non_loopback)?;
    let path = "/api/v1/rewards/bind";

    let body = format!(
        "{{\"rewardRecipientDisplayAddress\":\"{}\",\"note\":\"registry binding request only; no wallet or ledger mutation\"}}",
        json_escape(address)
    );

    if opts.dry_run {
        println!(
            "crabnode rewards bind {address}: would POST {}{}; no wallet/ledger mutation; no fake confirmed ROC",
            target.display_base(),
            path
        );
        return Ok(());
    }

    let response = http_post_json(&target, path, &body)?;
    println!("{response}");
    Ok(())
}

fn rewards_rotate_or_dry_run(opts: &Options, address: &str) -> Result<(), CrabnodeError> {
    validate_crablink_address(address)?;
    let target = ParsedAdminUrl::parse(&opts.admin_url, opts.allow_non_loopback)?;
    let path = "/api/v1/rewards/rotate";

    let body = format!(
        "{{\"newRewardRecipientDisplayAddress\":\"{}\",\"note\":\"future-epoch registry rotation request only; no wallet or ledger mutation\"}}",
        json_escape(address)
    );

    if opts.dry_run {
        println!(
            "crabnode rewards rotate {address}: would POST {}{}; rotation becomes active only after registry acceptance/effective epoch",
            target.display_base(),
            path
        );
        return Ok(());
    }

    let response = http_post_json(&target, path, &body)?;
    println!("{response}");
    Ok(())
}

fn validate_crablink_address(address: &str) -> Result<(), CrabnodeError> {
    let Some(username) = address.strip_prefix('@') else {
        return Err(CrabnodeError::usage(
            "reward recipient must be a CrabLink/RON @ address like @operator",
        ));
    };

    let bytes = username.as_bytes();
    if !(3..=32).contains(&bytes.len()) {
        return Err(CrabnodeError::usage(
            "reward recipient username must be 3..=32 characters after @",
        ));
    }

    if !bytes[0].is_ascii_alphanumeric() {
        return Err(CrabnodeError::usage(
            "reward recipient username must start with an ASCII letter or digit",
        ));
    }

    if matches!(bytes[bytes.len() - 1], b'.' | b'-' | b'_') {
        return Err(CrabnodeError::usage(
            "reward recipient username must not end with '.', '-', or '_'",
        ));
    }

    let mut previous_dot = false;
    for byte in bytes {
        let valid = byte.is_ascii_lowercase()
            || byte.is_ascii_digit()
            || matches!(*byte, b'_' | b'-' | b'.');

        if !valid {
            return Err(CrabnodeError::usage(
                "reward recipient username must use lowercase ASCII letters, digits, '.', '-', or '_'",
            ));
        }

        if previous_dot && *byte == b'.' {
            return Err(CrabnodeError::usage(
                "reward recipient username must not contain consecutive dots",
            ));
        }

        previous_dot = *byte == b'.';
    }

    Ok(())
}

fn required_secret_env(key: &'static str, note: &'static str) -> Result<String, CrabnodeError> {
    match env::var(key) {
        Ok(value) if !value.is_empty() => Ok(value),
        _ => Err(CrabnodeError::config(note)),
    }
}

fn json_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());

    for ch in input.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => {
                let code = ch as u32;
                out.push_str(&format!("\\u{code:04x}"));
            }
            ch => out.push(ch),
        }
    }

    out
}

#[derive(Debug)]
struct ParsedAdminUrl {
    host: String,
    port: u16,
}

impl ParsedAdminUrl {
    fn parse(input: &str, allow_non_loopback: bool) -> Result<Self, CrabnodeError> {
        let rest = input
            .strip_prefix("http://")
            .ok_or_else(|| CrabnodeError::config("only http:// admin URLs are supported today"))?;

        let authority = rest.split('/').next().unwrap_or_default();
        if authority.is_empty() {
            return Err(CrabnodeError::config("admin URL is missing host"));
        }

        let (host, port) = if let Some((host, port)) = authority.rsplit_once(':') {
            let parsed_port = port
                .parse::<u16>()
                .map_err(|e| CrabnodeError::config(format!("invalid admin URL port: {e}")))?;
            (host.to_string(), parsed_port)
        } else {
            (authority.to_string(), 80)
        };

        if !allow_non_loopback && !is_loopback_host(&host) {
            return Err(CrabnodeError::config(format!(
                "refusing non-loopback admin host {host:?}; pass --allow-non-loopback explicitly"
            )));
        }

        Ok(Self { host, port })
    }

    fn socket_addr(&self) -> Result<SocketAddr, CrabnodeError> {
        let mut addrs = (self.host.as_str(), self.port)
            .to_socket_addrs()
            .map_err(|e| CrabnodeError::io(format!("failed to resolve admin host: {e}")))?;

        addrs
            .next()
            .ok_or_else(|| CrabnodeError::io("admin host resolved to no addresses"))
    }

    fn display_base(&self) -> String {
        format!("http://{}:{}", self.host, self.port)
    }
}

fn is_loopback_host(host: &str) -> bool {
    host == "localhost" || host == "::1" || host == "127.0.0.1" || host.starts_with("127.")
}

fn http_post(target: &ParsedAdminUrl, path: &str) -> Result<String, CrabnodeError> {
    http_request(target, "POST", path, "")
}

fn http_post_json(
    target: &ParsedAdminUrl,
    path: &str,
    body: &str,
) -> Result<String, CrabnodeError> {
    http_request(target, "POST", path, body)
}

fn http_get(target: &ParsedAdminUrl, path: &str) -> Result<String, CrabnodeError> {
    http_request(target, "GET", path, "")
}

fn http_request(
    target: &ParsedAdminUrl,
    method: &str,
    path: &str,
    body: &str,
) -> Result<String, CrabnodeError> {
    let addr = target.socket_addr()?;
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(2)).map_err(|e| {
        CrabnodeError::io(format!(
            "failed to connect to {}: {e}",
            target.display_base()
        ))
    })?;

    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .map_err(|e| CrabnodeError::io(format!("failed to set read timeout: {e}")))?;
    stream
        .set_write_timeout(Some(Duration::from_secs(3)))
        .map_err(|e| CrabnodeError::io(format!("failed to set write timeout: {e}")))?;

    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: {}:{}\r\nUser-Agent: crabnode/phase4e\r\nAccept: application/json,text/plain,*/*\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        target.host,
        target.port,
        body.len(),
        body
    );

    stream
        .write_all(request.as_bytes())
        .map_err(|e| CrabnodeError::io(format!("failed to write request: {e}")))?;

    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|e| CrabnodeError::io(format!("failed to read response: {e}")))?;

    let status_line = response.lines().next().unwrap_or_default().to_string();
    let status_code = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse::<u16>().ok())
        .ok_or_else(|| CrabnodeError::http(format!("invalid HTTP response: {status_line:?}")))?;

    let body = response
        .split_once("\r\n\r\n")
        .map(|(_, body)| body.trim().to_string())
        .unwrap_or_default();

    if !(200..300).contains(&status_code) {
        return Err(CrabnodeError::http(format!(
            "admin endpoint returned {status_code}: {body}"
        )));
    }

    if body.is_empty() {
        Ok(status_line)
    } else {
        Ok(body)
    }
}

fn print_help() {
    println!(
        r#"crabnode — headless CrabLink service-node operator CLI

USAGE:
  crabnode [--admin-url URL] [--svc-admin-url URL] [--policy-file PATH] [--dry-run] [--allow-non-loopback] <command>

COMMANDS:
  status                       Query /api/v1/status from the node admin plane
  ready                        Query /readyz from the node admin plane
  version                      Query /version from the node admin plane
  policy status                Inspect bounded counts in the configured policy file
  block b3:<hash>              Add an exact object to the local block set
  unblock b3:<hash>            Remove an exact object from the local block set
  allow b3:<hash>              Add an exact object to the local allow set
  remove-allow b3:<hash>       Remove an exact object from the local allow set
  quarantine b3:<hash>         Add an exact object to quarantine
  release-quarantine b3:<hash> Remove an exact object from quarantine
  prune b3:<hash>              Delete exact local bytes and withdraw this node's provider record
  persistence register b3:<hash> KIND
                               Register an amnesia-first review candidate
  persistence status b3:<hash>
                               Query one persistence candidate
  persistence pending [LIMIT] List bounded undecided candidates
  persistence submit b3:<hash>
                               Submit an exact object for review
  persistence approve b3:<hash>
                               Approve persistence eligibility after policy checks
  persistence pin b3:<hash>
                               Pin a verified candidate after policy checks
  persistence unpin b3:<hash>
                               Remove an operator pin while preserving eligibility
  persistence reject b3:<hash>
                               Reject persistence for an exact object
  admin enable-web             Enable optional local admin UI through macronode
  admin disable-web            Disable optional local admin UI through macronode
  admin setup-token            Issue a local one-time setup token through macronode
  admin create-user USER       Create first local svc-admin user using setup token env
  rewards show                 Query reward-recipient binding status from macronode
  rewards bind @operator       Request reward-recipient binding through macronode
  rewards rotate @new          Request future-epoch reward-recipient rotation

DEFAULTS:
  CRABNODE_ADMIN_URL or http://127.0.0.1:8080
  CRABNODE_SVC_ADMIN_URL or http://127.0.0.1:5300
  CRABNODE_POLICY_FILE or RON_SERVICE_NODE_MODERATION_POLICY_PATH
  loopback-only unless --allow-non-loopback is explicit

CREATE-USER ENV:
  CRABNODE_USER_PASSWORD      Password for the first local admin user
  CRABNODE_SETUP_TOKEN        One-time token from `crabnode admin setup-token`

NOTES:
  setup-token/enable-web/disable-web call real macronode endpoints.
  create-user calls the real svc-admin first-run setup endpoint and never prints secrets.
  rewards commands call real macronode endpoints; they do not mutate wallet or ledger state.
  policy mutations use ron-policy exact-b3 rules and require node restart for activation.
  prune calls the real macronode coordinator and reports storage, provider, and index-cache outcomes.
  prune is local-only today: no network-wide deletion, resolve-cache removal, or manifest-pointer deletion.
  persistence commands call the real process-local review catalog; eligibility does not prove durable bytes.
  persistence register KIND accepts canonical asset suffixes such as image, video, article, page, and manifest.
  local admin credentials are node-local; recreate the node if they are lost."#
    );
}

#[derive(Debug)]
enum CrabnodeError {
    Usage(String),
    Config(String),
    Io(String),
    Http(String),
}

impl CrabnodeError {
    fn usage(msg: impl Into<String>) -> Self {
        Self::Usage(msg.into())
    }

    fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    fn io(msg: impl Into<String>) -> Self {
        Self::Io(msg.into())
    }

    fn http(msg: impl Into<String>) -> Self {
        Self::Http(msg.into())
    }

    fn exit_code(&self) -> u8 {
        match self {
            Self::Usage(_) => 64,
            Self::Config(_) => 78,
            Self::Io(_) | Self::Http(_) => 69,
        }
    }
}

impl fmt::Display for CrabnodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(msg) => write!(f, "{msg}\n\nRun `crabnode --help` for usage."),
            Self::Config(msg) => write!(f, "configuration error: {msg}"),
            Self::Io(msg) => write!(f, "I/O error: {msg}"),
            Self::Http(msg) => write!(f, "HTTP error: {msg}"),
        }
    }
}
