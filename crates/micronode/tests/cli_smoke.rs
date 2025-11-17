//! RO:WHAT — Smoke tests for Micronode CLI surface.
//! RO:WHY  — Ensure `Cli::from_env()` produces a sensible default shape and
//!           that the public enums/types remain stable across refactors.

use micronode::cli::{Cli, Command, Profile};

#[test]
fn default_cli_uses_serve_command() {
    let cli = Cli::from_env();

    match cli.command() {
        Command::Serve(opts) => {
            // Default profile should be `Dev` to favor DX.
            assert_eq!(opts.profile, Profile::Dev);
            // Dev routes should be on by default.
            assert!(opts.dev_routes);
        }
        other => panic!("expected default command to be Serve(..), got {other:?}"),
    }
}
