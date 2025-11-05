// crates/micronode/examples/quickstart.rs
//! RO:WHAT — Minimal example entrypoint so `cargo build` succeeds.
//! RO:HOW  — Prints a hint to use the primary binary (`cargo run -p micronode`).
//! RO:FUTURE — Can be replaced later with a runnable SDK demo.

fn main() {
    println!("Micronode quickstart example.");
    println!("Run the server with:");
    println!("  MICRONODE_DEV_ROUTES=1 cargo run -p micronode");
}
