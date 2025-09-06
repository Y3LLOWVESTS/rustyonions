# ErrorEnvelope — Canonical Error Format (HTTP + OAP)

_Status: Stable • Last updated: 2025-09-05_

All RustyOnions services **MUST** return errors using this envelope across **HTTP** and **OAP/1**.

## JSON structure
```json
{
  "code": "string",          // machine code: e.g., "bad_request", "not_found", "payload_too_large", "quota_exhausted", "unavailable"
  "message": "string",       // human message (short)
  "retryable": false,        // whether client may retry
  "corr_id": "string"        // correlation id for tracing
}
```

## HTTP mapping
- `400` → `bad_request` (retryable=false)
- `404` → `not_found` (retryable=false)
- `413` → `payload_too_large` (retryable=false)
- `429` → `quota_exhausted` (retryable=true) + **`Retry-After`**
- `503` → `unavailable` (retryable=true) + optional **`Retry-After`**

## OAP mapping
- Frame type `ERROR` with the same envelope as payload (UTF‑8 JSON).

## Client requirements
- Surface `corr_id` to logs/UX.
- Honor `Retry-After` if present; apply bounded jitter backoff.
- Treat only `retryable:true` as eligible for automatic retry.
- Preserve idempotency: prefer safe methods and repeatable operations.

## Server guidelines
- Always generate a `corr_id` and log it with the error cause.
- Do not leak internal details in `message`; keep it short/human.
- Be consistent: do not invent new codes casually — prefer the set above.
