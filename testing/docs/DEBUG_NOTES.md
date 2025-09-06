# RustyOnions — gateway/UDS smoke-test notes (carry-forward)

## TL;DR (why the old script hung)
- The current **`crates/gateway`** talks to **`svc-overlay` over a UNIX domain socket**, not to `svc-omnigate` over OAP/TCP.  
  Therefore, a script that starts only `svc-omnigate` + `gateway` will stall when the gateway tries to fetch bytes (no overlay).
- Gateway logging was sparse because it didn’t initialize `tracing`. Services (`svc-index`, `svc-storage`, `svc-overlay`) already use `tracing_subscriber` and honor `RUST_LOG`.

## What we changed
- Rewrote `testing/test_gateway.sh` to:
  - Pack two bundles with `tldctl` (`post` and `image`), writing to `.onions` + `.data/index`.
  - Launch **UDS stack**: `svc-index`, `svc-storage`, `svc-overlay` (with sockets under a temp dir).
  - Launch **gateway** bound to `127.0.0.1:<port>` and pointed at `RON_OVERLAY_SOCK`.
  - Verify `GET /o/<addr>/Manifest.toml` and `HEAD /o/<addr>/payload.bin`.
  - Stream logs on demand (`STREAM_LOGS=1`) and archive to `testing/_logs/last_run/`.
- Added a tiny logging bootstrap to `crates/gateway/src/main.rs` so `RUST_LOG=debug,gateway=trace` produces useful startup lines.

## How to run
STREAM_LOGS=1 RUST_LOG=info,svc_index=debug,svc_storage=debug,svc_overlay=debug \
bash testing/test_gateway.sh

## Where to look if something fails
- `testing/_logs/last_run/svc-index.log` — should say `svc-index listening` and show the DB path.
- `testing/_logs/last_run/svc-storage.log` — should say `svc-storage listening`.
- `testing/_logs/last_run/svc-overlay.log` — should say `svc-overlay listening`.
- `testing/_logs/last_run/gateway.log` — should say `gateway listening` with the bound socket.

## Next incremental improvements
1. **Gateway structured logs**: keep the `tracing_subscriber` init; emit a startup summary (bind addr, `RON_OVERLAY_SOCK`, payments on/off).
2. **Health/Ready endpoints**:
   - Gateway: `/healthz` (always 200), `/readyz` (200 once it can open `RON_OVERLAY_SOCK`).
3. **OAP migration (optional)**:
   - If you want **gateway → OAP** instead of UDS: add a small OAP client in the gateway that speaks to `svc-omnigate`, or run a thin OAP→UDS adapter.
