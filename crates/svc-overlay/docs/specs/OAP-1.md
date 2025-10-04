# OAP/1 — Local Normative Notes (svc-overlay)
- Frame cap: 1 MiB (reject above).
- Storage streaming chunk: ~64 KiB (separate concern, noted for context).
- REQUIRED headers for object frames include obj:"b3:<hex>".
- Interop vectors for hello/ack and error paths live under tests/.
