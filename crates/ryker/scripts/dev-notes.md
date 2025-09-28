# Dev notes — ryker
- Loom: run targeted: RUSTFLAGS="--cfg loom" cargo test -p ryker --test loom/loom_mailbox_basic -- --nocapture
- Fuzz: cargo fuzz run fuzz_parse_config_toml -- -max_total_time=60
- Bench: cargo bench -p ryker && see docs/benches/README.md

