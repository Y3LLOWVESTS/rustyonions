# Phase 2 to Phase 3 Chasm

RO:WHAT — QC-0A warning document for the local-root to replicated-validator transition.
RO:WHY — Moving from deterministic local roots to a validator committee is a major architectural boundary, not a toggle.
RO:INTERACTS — ron-ledger, svc-wallet, ron-accounting, svc-rewarder, svc-registry, ron-kms, future validators.
RO:INVARIANTS — no committee until artifacts are independently replayable; no validator signing until checkpoint hashes reproduce.
RO:METRICS — future replay and quorum verification counters.
RO:CONFIG — future settlement_mode and consensus mode.
RO:SECURITY — validators must not trust original operator state; they must replay canonical artifacts.
RO:TEST — future validator simulator and independent replay tests.

## Chasm rule

Phase 2 local epoch sealing must be validator-executable before Phase 3 begins.

Phase 3 is blocked until:

```text
root-producing execution is pure and deterministic
all inputs are canonical
all outputs have test vectors
state transition can be replayed outside the original service
no wall-clock reads affect roots
no DB-order assumptions affect roots
operator tools can export/import epoch artifacts
validator simulator reproduces checkpoint hash
```

## Required artifact export

Future Phase 2 must export:

```text
checkpoint header bytes
receipt batches
account state leaves
hold state leaves
accounting windows
reward manifests
policy hash input
chain params hash input
validator set hash input if used
```

## No-go

If a validator cannot independently reproduce the checkpoint hash, Phase 3 must not begin.
