fmt:
	cargo fmt --all

# Clippy for lib/bin/tests/examples, but NOT benches (avoids criterion).
clippy:
	cargo clippy --workspace --tests --examples -- -D warnings

# Tests only (no benches)
test:
	cargo test --workspace -- --nocapture

check: fmt clippy test
