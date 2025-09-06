# Gateway Read-Path Tests

## Prereqs
- Gateway running and serving your `.onions` store.
- One packed object address (BLAKE3): `b3:<hex>`.

## Quick Start
1. Start your e2e smoke (gw + services). Note the base URL, e.g. `http://127.0.0.1:9080`.
2. Pack a small text and note its address `b3:...`.
3. Export env and run:

```bash
export GATEWAY_URL=http://127.0.0.1:9080
export OBJ_ADDR=b3:youraddresshere
cargo test -p gateway --test http_read_path -- --nocapture
