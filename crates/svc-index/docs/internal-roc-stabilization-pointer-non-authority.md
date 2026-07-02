# Internal ROC Stabilization — svc-index Pointer / Lookup Non-Authority Boundary

RO:WHAT — Product beta readiness boundary for svc-index as lookup, provider, asset-manifest, and site-manifest pointer infrastructure.

RO:WHY — Product beta needs asset/site pointers and provider lookups for CrabLink hydration, but index records must not become wallet, ledger, receipt, balance, paid entitlement, finality, bridge, staking, liquidity, or external settlement authority.

RO:INTERACTS — types.rs, store/keys.rs, http/routes/index_manifests.rs, http/routes/resolve.rs, http/routes/providers.rs, pipeline/resolve.rs, existing Internal ROC beta and QuickChain index tests.

RO:INVARIANTS — b3 identifies bytes; crab:// is navigation; names are mutable pointers; manifests describe content; owner wallet/passport fields are reference metadata only; backend paid access truth remains wallet/ledger/gateway/omnigate enforced.

RO:SECURITY — no direct wallet mutation, no direct ledger mutation, no receipt creation, no balance truth, no paid unlock truth, no entitlement truth, no finality truth, no index-only unlock, no pointer-only unlock, no manifest-only unlock, no cache-only unlock, no bridge, no ROX/Solana, no staking, no liquidity, no external settlement.

RO:TEST — cargo test -p svc-index --test internal_roc_stabilization_pointer_non_authority_boundary.

## Allowed

store asset CID to asset manifest CID pointers
store site/name to site manifest CID pointers
resolve names to manifest references
return provider references
normalize b3 CIDs
normalize asset kinds
normalize site names
carry owner wallet account as reference string
carry owner passport subject as reference string
support crab:// navigation metadata
return lookup/cache/provider metadata

## Forbidden

index pointer as paid access proof
manifest pointer as receipt proof
provider lookup as settlement proof
owner wallet reference as spend authority
passport subject as paid proof
cache hit as paid unlock
resolve response as entitlement truth
site pointer as ownership finality
asset pointer as ledger truth
index route as bridge/staking/liquidity/external settlement runtime

## Product beta lock

Correct path:

creator/content manifest
→ svc-index pointer/reference graph
→ gateway/omnigate lookup
→ backend wallet/ledger receipt/access truth
→ paid render

Incorrect path:

index pointer
→ paid unlock

manifest reference
→ receipt truth

owner wallet reference
→ spend authority

resolve response
→ entitlement truth
