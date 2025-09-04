# OAP/1 — canonical spec pointer

OAP/1 is defined by GMI-1.6 (this file is a mirror stub to prevent spec drift).

Defaults:
- `max_frame = 1 MiB` (protocol default)
- DATA payload layout: `[u16 header_len][header JSON][raw body]`
- DATA header includes `obj:"b3:<hex>"` (BLAKE3-256 of plaintext body)

See repo docs/blueprints for normative details.
