# Node Crate

Implements the CLI for RustyOnions.

## Commands
- `serve` — start the node overlay listener
- `put <PATH>` — store a file
- `get <KEY> <OUT>` — retrieve a file

## Example
```bash
cargo run -p node -- serve
cargo run -p node -- put ./file.txt
cargo run -p node -- get <KEY> ./out.txt
```
