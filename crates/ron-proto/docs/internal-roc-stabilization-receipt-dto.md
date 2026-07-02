# Internal ROC Stabilization — ron-proto Receipt DTO Truth Boundary

RO:WHAT — Product beta readiness boundary for backend-derived Internal ROC receipt DTO shape.

RO:WHY — CrabLink, gateway, omnigate, wallet, ledger, and accounting must agree that receipt DTOs are display/transport contracts only; receipt truth is created by svc-wallet and recorded by ron-ledger.

RO:INTERACTS — quickchain/receipt.rs, quickchain/operation.rs, quickchain/ids.rs, quickchain/money.rs, tests/internal_roc_beta_paid_content_dto.rs, tests/quickchain_receipt_dto.rs, tests/quickchain_ids_and_money.rs.

RO:INVARIANTS — ron-proto is DTO-only; operation_id is durable backend operation identity; idempotency_key is retry metadata only; amount_minor is an integer minor-unit string; receipt status labels must not fabricate finality.

RO:SECURITY — no receipt issuance, no balance truth, no paid entitlement truth, no finality truth, no wallet mutation, no ledger mutation, no bridge, no ROX/Solana, no staking, no liquidity, no external settlement.

RO:TEST — cargo test -p ron-proto --test internal_roc_stabilization_receipt_dto_boundary.

## Required model

svc-wallet accepts an economic mutation
→ ron-ledger records durable economic truth
→ backend returns receipt DTO/reference
→ CrabLink/gateway/omnigate/accounting display or classify the DTO

## The DTO may carry

- operation_id
- idempotency_key
- txid
- receipt status
- integer amount_minor
- ledger sequence references
- backend-derived memo/labels

## The DTO must not create

- receipt truth
- balance truth
- finality truth
- paid entitlement
- wallet authority
- ledger authority
- bridge authority
- external settlement

## Identity meanings

operation_id = backend durable ledger-operation identity
idempotency_key = retry/dedupe metadata only
txid = backend receipt reference
amount_minor = integer minor-unit string only
accepted = backend wallet/ledger accepted mutation; not external finality
