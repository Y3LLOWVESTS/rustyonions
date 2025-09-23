---
title: ZK Circuits Catalog
version: 0.1.0
status: draft
last-updated: 2025-09-22
audience: contributors, security auditors, ops
---

## Purpose
Catalog the zero-knowledge circuits that macronode MAY verify at capability and policy boundaries without revealing witnesses/PII.

## Circuits

1) Capability Attributes (Selective Disclosure)
- Goal: Prove capability has attributes {role ∈ GroupX, not expired} without revealing identity.
- Public Input: capability commitment, expiry, group root.
- Domain: `domain:capability`.
- Verifier: `ZkVerifierKind::<backend>`; VK pinned via KMS; param hash recorded.

2) Quota Compliance
- Goal: Prove “requests_used < quota_limit” using a committed counter (private).
- Public Input: commitment root, limit, current epoch.
- Domain: `domain:quota`.

3) Audit Inclusion (Merkle)
- Goal: Prove event is included in transparency log without revealing neighbors.
- Public Input: Merkle root + event hash.
- Domain: `domain:audit`.

### Operational Notes
- Verifier keys are authenticated via KMS; param hashes (BLAKE3-256) pinned in config.
- Witness material is never logged or persisted; amnesia mode zeroizes all buffers.
- Batch verification path required for throughput (target: p95 ≤ 15 ms/proof for small preds).
