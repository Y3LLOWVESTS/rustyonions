# Internal ROC Stabilization — svc-storage b3 Artifact / Paid Admission Non-Authority Boundary

RO:WHAT — Product beta readiness boundary for svc-storage as canonical b3 byte/artifact storage plus paid-write admission participant.

RO:WHY — Storage must serve bytes and metadata, verify paid-write admission evidence through backend wallet/ledger receipts, settle only through svc-wallet, and export metering to accounting without becoming economic truth.

RO:INTERACTS — storage/*, http/routes/paid_object.rs, http/routes/paid_estimate.rs, policy/paid_write.rs, policy/settlement.rs, accounting/exporter.rs, existing Internal ROC beta and QuickChain storage tests.

RO:INVARIANTS — b3 proves bytes only; ETag proves content only; storage admission is not finality; metering is not balance truth; accounting export is not payout truth; svc-wallet remains mutation front-door; ron-ledger remains durable truth.

RO:SECURITY — no direct wallet mutation, no direct ledger mutation, no receipt creation, no balance truth, no entitlement truth, no paid unlock truth, no cache-only unlock, no b3-only unlock, no manifest-only unlock, no bridge, no ROX/Solana, no staking, no liquidity, no external settlement.

RO:TEST — cargo test -p svc-storage --test internal_roc_stabilization_b3_artifact_non_authority_boundary.

## Allowed

store canonical b3-addressed bytes
retrieve full objects
retrieve bounded byte ranges
return byte/content metadata
return content-derived ETags
verify paid-write admission evidence through backend wallet receipt lookup
bind paid-write evidence to body-derived b3 CID and context idempotency
settle paid storage only through svc-wallet capture/release paths
emit usage events
export metering to ron-accounting as non-authority
estimate paid storage price with integer-only economics

## Forbidden

stored bytes as paid proof
b3 hash as receipt proof
ETag as payment proof
cache hit as entitlement proof
manifest reference as unlock proof
policy-only result as unlock proof
usage event as balance proof
accounting export as payout proof
storage admission as finality proof
direct wallet mutation
direct ledger mutation
receipt creation
balance mutation
paid unlock truth
bridge runtime
staking runtime
liquidity runtime
ROX/Solana runtime
external settlement runtime

## Product beta lock

Correct path:

backend wallet/ledger receipt/access truth
→ storage paid admission check
→ b3 byte write/read
→ wallet capture/release through svc-wallet only
→ usage/metering export to accounting

Incorrect path:

b3 exists
→ paid unlock

storage cache hit
→ entitlement

storage usage event
→ balance truth
