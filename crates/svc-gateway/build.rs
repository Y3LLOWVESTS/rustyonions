// Stamp SVC_GATEWAY_BUILD_TS (UNIX seconds) into the binary at compile time.
// Zero deps; works in any workspace layout.
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    // Ensure rebuild when these change (the git lines are optional)
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs");

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    println!("cargo:rustc-env=SVC_GATEWAY_BUILD_TS={}", ts);
}
