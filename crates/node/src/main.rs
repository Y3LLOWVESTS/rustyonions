//! Minimal binary for `ronode` — all logic is in the library.
fn main() -> anyhow::Result<()> {
    node::cli::run()
}
