# Accounting Crate

The accounting crate manages counters and metrics for RustyOnions.

## Features
- Thread-safe counters using `Arc<Mutex<...>>`
- Increment and retrieve TX/RX counts

## Example
```rust
let counters = Counters::default();
counters.inc_tx();
println!("TX count: {}", counters.get_tx());
```
