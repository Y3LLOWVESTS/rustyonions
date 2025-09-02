# OAP/1 — Overlay App Protocol (normative, Bronze)

Status: Draft-for-Implementation • Version: 1 • Scope: Envelope, flags, HELLO, errors, parser rules, vectors  
Non-goal: App semantics (APP_E2E is opaque to the node)

## 0. TL;DR
One binary envelope lets any app speak over the overlay. Unknown flags must be ignored. APP_E2E payloads are opaque. Bounds are enforced before allocation. Backpressure and overload are explicit (429/503 with Retry-After).

## 1. Wire Format

All integers unsigned, little-endian.

| Field          | Type | Size  | Notes |
|----------------|------|-------|------|
| `len`          | u32  | 4     | Length of the rest of the frame |
| `ver`          | u8   | 1     | Protocol version (1) |
| `flags`        | u16  | 2     | Bitset: REQ, RESP, EVENT, START, END, ACK_REQ, COMP, APP_E2E |
| `code`         | u16  | 2     | Status (0 OK; 2xx/4xx/5xx) |
| `app_proto_id` | u16  | 2     | Registry ID for the application protocol |
| `tenant_id`    | u128 | 16    | ULID/UUID, 0 = unspecified |
| `cap_len`      | u16  | 2     | Capability bytes (only when START) |
| `corr_id`      | u64  | 8     | Stream correlation identifier |
| `cap`          | []   | var   | Present iff `cap_len>0` **and** `START` set |
| `payload`      | []   | var   | Opaque application bytes (may be COMP/E2E) |

### 1.1 Flags
- `REQ`, `RESP`, `EVENT`, `START`, `END`, `ACK_REQ`, `COMP`, `APP_E2E`.  
- Streams: `REQ|START` opens; further `REQ` chunks share `corr_id`; `REQ|END` closes.  
- Server replies with `RESP` chunks; `RESP|END` closes.  
- `EVENT` is server push; only if capability permits subscriptions.  
- `ACK_REQ` enables explicit backpressure windows.

### 1.2 Status Codes (subset)
- Success: `0 OK`, `202 Accepted`, `206 Partial Content`  
- Client: `400,401,403,404,408,409,413,429 (+Retry-After)`  
- Server: `500,502,503 (+Retry-After),504`  
- Payments: `402 Denied` with structured reason

## 2. Negotiation (HELLO)
A request with `app_proto_id=0` returns server limits and features.

Example HELLO response body (JSON):
```json
{
  "server_version":"1.0.0",
  "max_frame":1048576,
  "max_inflight":64,
  "supported_flags":["EVENT","ACK_REQ","COMP","APP_E2E"],
  "oap_versions":[1],
  "transports":["tcp+tls","tor"]
}
