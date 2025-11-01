# Omnigate Configuration (Beta)

Omnigate reads config from CLI flags, environment, then TOML (`--config ./crates/omnigate/configs/omnigate.toml`).

## Server
- `bind` (string): API bind, e.g. `127.0.0.1:5305`
- `metrics_addr` (string): admin/metrics bind, e.g. `127.0.0.1:9605`
- `amnesia` (bool): RAM-only caches + aggressive zeroization

## OAP
- `max_frame_bytes` (≤ 1 MiB)
- `stream_chunk_bytes` (≈ 64 KiB)

## Admission
- `global_quota.{qps,burst}`
- `ip_quota.enabled/qps/burst` + optional `buckets`
- `fair_queue.max_inflight` + `weights` `{anon,auth,admin}`
- `body.max_content_length`, `reject_on_missing_length`
- `decompression.allow`, `deny_stacked`

## Policy (ron-policy)
- `enabled` (bool)
- `bundle_path` (file path for policy bundle)
- `fail_mode` (`deny`|`allow`) if evaluator missing/unavailable

## Readiness / Overload
Degrade `/readyz` while any holds for `hold_for_secs`:
- inflight > `max_inflight_threshold`
- rolling `(429|503)` rate ≥ `error_rate_429_503_pct` over `window_secs`
- admission queue saturation

See `configs/omnigate.toml` for example.
