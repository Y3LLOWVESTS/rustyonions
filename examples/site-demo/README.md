# RON-CORE WOW Demo (Stage 0)

This is the smallest “wow” proof:
- A static site is served **from Micronode** via `facets`.
- You can refresh the site and watch **real metrics** move.
- Optional: use dev endpoints for reload/crash if enabled.

## Run

From repo root:

MICRONODE_DEV_ROUTES=1 MICRONODE_CONFIG=crates/micronode/configs/micronode.facets.toml cargo run -p micronode

Then open:

- Site:   http://127.0.0.1:5310/facets/wow/
- Health: http://127.0.0.1:5310/healthz
- Ready:  http://127.0.0.1:5310/readyz
- Metrics:http://127.0.0.1:5310/metrics

## Expected “wow” moments

1) Refresh the site a few times → observe counters/histograms in `/metrics`.
2) If `/dev/reload` exists: click Reload in the UI (or curl it).
3) If `/dev/crash` exists: click Crash in the UI (or curl it) → watch ready/health flip and recover.

## Notes

- No content IDs/hashes are shown anywhere in the UX.
- Stage 1 will route this through `svc-gateway` (clean public URL).
