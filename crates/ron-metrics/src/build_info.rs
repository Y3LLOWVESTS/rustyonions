//! RO:WHAT — Build/version helpers for base labels.

pub fn build_version() -> String {
    // Package version + short git if provided by outer build
    let ver = env!("CARGO_PKG_VERSION");
    match option_env!("GIT_SHA_SHORT") {
        Some(sha) if !sha.is_empty() => format!("{ver}+{sha}"),
        _ => ver.to_string(),
    }
}
