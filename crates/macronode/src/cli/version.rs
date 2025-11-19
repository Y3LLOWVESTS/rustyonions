//! RO:WHAT — Implementation of the `version` subcommand.
//! RO:WHY  — Provide a simple CLI-friendly equivalent to `/version`.

use crate::types::BuildInfo;

/// Print version information to stdout.
///
/// Shape matches the `/version` HTTP payload, minus the API version.
pub fn run() {
    let info = BuildInfo::current();
    println!(
        "service={service} version={version} git_sha={git_sha} build_ts={build_ts} rustc={rustc} msrv={msrv}",
        service = info.service,
        version = info.version,
        git_sha = info.git_sha,
        build_ts = info.build_ts,
        rustc = info.rustc,
        msrv = info.msrv,
    );
}
