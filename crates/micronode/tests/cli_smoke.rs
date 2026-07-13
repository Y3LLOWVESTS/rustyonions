//! RO:WHAT — Smoke tests for Micronode CLI surface.
//! RO:WHY  — Ensure local commands are parseable and status output is tied to
//!           the real admin status contract.

use micronode::{
    cli::{run::render_local_status_json, Cli, Command, Profile},
    config::schema::{Config, Server},
};

#[test]
fn default_cli_uses_serve_command() {
    let cli = Cli::from_args(std::iter::empty::<&str>()).expect("parse default cli");

    match cli.command() {
        Command::Serve(opts) => {
            // Default profile should be `Dev` to favor DX.
            assert_eq!(opts.profile, Profile::Dev);
            // Dev routes should be on by default in CLI shape.
            assert!(opts.dev_routes);
        }
        other => panic!("expected default command to be Serve(..), got {other:?}"),
    }
}

#[test]
fn parses_check_command_with_config_path() {
    let cli = Cli::from_args(["check", "--config", "crates/micronode/configs/micronode.toml"])
        .expect("parse check command");

    match cli.command() {
        Command::Check { config_path } => {
            assert_eq!(config_path.as_deref(), Some("crates/micronode/configs/micronode.toml"));
        }
        other => panic!("expected Check command, got {other:?}"),
    }
}

#[test]
fn parses_status_command_with_config_path() {
    let cli = Cli::from_args(["status", "-c", "crates/micronode/configs/micronode.toml"])
        .expect("parse status command");

    match cli.command() {
        Command::Status { config_path } => {
            assert_eq!(config_path.as_deref(), Some("crates/micronode/configs/micronode.toml"));
        }
        other => panic!("expected Status command, got {other:?}"),
    }
}

#[test]
fn parses_implicit_serve_options() {
    let cli =
        Cli::from_args(["--bind", "127.0.0.1:5311", "--profile", "amnesia", "--no-dev-routes"])
            .expect("parse implicit serve");

    match cli.command() {
        Command::Serve(opts) => {
            assert_eq!(opts.bind_addr.as_deref(), Some("127.0.0.1:5311"));
            assert_eq!(opts.profile, Profile::Amnesia);
            assert!(!opts.dev_routes);
            assert_eq!(opts.dev_routes_override, Some(false));
        }
        other => panic!("expected Serve command, got {other:?}"),
    }
}

#[test]
fn invalid_command_becomes_parse_error() {
    let err = Cli::from_args(["explode"]).expect_err("expected parser error");
    assert!(err.to_string().contains("unknown command"));
}

#[test]
fn status_json_comes_from_admin_contract_without_claiming_rewards() {
    let cfg = Config {
        server: Server {
            bind: "127.0.0.1:5310".parse().expect("valid loopback bind"),
            dev_routes: false,
        },
        ..Config::default()
    };

    let body = render_local_status_json(cfg);

    assert_eq!(body["node_role"], "user_node");
    assert_eq!(body["node_profile"], "micronode");
    assert_eq!(body["privacy_mode"], true);
    assert_eq!(body["public_inbound_enabled"], false);
    assert_eq!(body["peer_ip_display"], "forbidden");
    assert_eq!(body["passive_runtime"]["verification_queue"]["status"], "stubbed");
    assert_eq!(body["passive_runtime"]["economic_replay_worker"]["status"], "stubbed");
    assert_eq!(body["passive_runtime"]["wallet_mutation"], false);
    assert_eq!(body["passive_runtime"]["ledger_mutation"], false);
    assert_eq!(body["passive_runtime"]["confirmed_roc_minor_units"], serde_json::Value::Null);
}
