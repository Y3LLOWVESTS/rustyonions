# ROC_ROX_BRIDGE_THREAT_MODEL.md — ROC ↔ ROX / Solana Bridge Threat Model

RO:WHAT — Threat model for possible future ROC ↔ ROX burn/mint bridge design.

RO:WHY — Identifies double-mint, double-issue, replay, proof/finality, RPC equivocation, Solana Anchor, custody, recovery, key compromise, UX truthfulness, liquidity creep, and governance risks before any runtime work.

RO:INTERACTS — POST_INTERNAL_ROC_BRIDGE_DECISION_GATE.md, ROC_ROX_SOLANA_ANCHOR_BRIDGE_BLUEPRINT.md, ROC_ROX_SOLANA_BRIDGE_BUILDPLAN.md, ron-proto, ron-ledger, svc-wallet, svc-interop, ron-policy, ron-audit, CrabLink Tauri, future Solana Anchor program.

RO:INVARIANTS — Threat model only; no runtime authorization; internal ROC truth remains svc-wallet/ron-ledger; no direct ledger mutation outside svc-wallet; no client/gateway/omnigate/accounting/rewarder/policy authority.

RO:METRICS — Future controls may expose nonce replay rejection counts, proof quorum failures, challenge disputes, halt activations, recovery-account issues, key rotation events, stuck challenge timeouts, and stale UI status displays.

RO:CONFIG — Future bridge config must be disabled by default and rejected if pause/halt/caps/challenge windows/quorum thresholds/cluster binding/program binding/mint binding/recovery policy are missing.

RO:SECURITY — Treat Solana RPCs, clients, coordinators, UI caches, relayers, upgrade keys, and bridge configs as hostile or compromiseable.

RO:TEST — Future red-team tests must target nonce replay, cluster replay, RPC equivocation, stale proof, malicious CPI, wrong token program, stuck challenge, halt bypass, recovery failure, forbidden UI language, and accidental liquidity scope.

---

## 0. Status

This is a threat model.

It does not authorize bridge runtime.

Safe label:

```text
ROC ↔ ROX bridge: DOCS / THREAT-MODEL ONLY.
```

---

## 1. Assets

Protected assets:

```text
internal ROC balances
ron-ledger replay/conservation truth
svc-wallet mutation boundary
bridge lifecycle status truth
ROX mint authority if future program exists
bridge nonces
proof packages
challenge decisions
pause/halt controls
upgrade authority
recovery account
audit trail
CrabLink user trust
```

---

## 2. Trust boundaries

Trusted only within their defined roles:

```text
svc-wallet
→ internal ROC mutation front-door only

ron-ledger
→ internal durable truth only

ron-policy
→ declarative gate only

ron-audit
→ append-only evidence only
```

Untrusted or partially trusted:

```text
CrabLink client
browser/client storage
gateway
omnigate
svc-interop coordinator
Solana RPC provider
single Solana transaction observation
relayer
external wallet
public network
logs
cache
user-provided proof
```

No single untrusted component may cause mint, issue, burn, refund, or finality.

---

## 3. Primary threats

### T1 — Double mint

Attack:

```text
same ROC burn receipt is used to mint ROX twice
```

Controls:

```text
cross-domain nonce binding
consumed nonce PDA
internal operation_id binding
ledger receipt binding
direction binding
cluster binding
program id binding
mint binding
amount binding
recipient binding
audit event
```

Required tests:

```text
same_internal_burn_cannot_mint_twice
same_internal_burn_cannot_mint_on_second_cluster
same_internal_burn_cannot_mint_to_second_recipient
```

### T2 — Double issue

Attack:

```text
same ROX burn is used to issue internal ROC twice
```

Controls:

```text
Solana tx signature binding
instruction binding
token account binding
mint binding
burn amount binding
consumed nonce record
svc-wallet idempotency
ron-ledger replay rejection
```

Required tests:

```text
same_external_burn_cannot_issue_twice
same_external_burn_cannot_issue_to_second_account
same_external_burn_cannot_issue_after_reorg_observation_mismatch
```

### T3 — Cluster replay

Attack:

```text
devnet proof or fork proof is replayed as mainnet proof
```

Controls:

```text
cluster binding
genesis/hash context where available
configured allowed cluster
policy validation
proof package includes cluster
nonce includes cluster
```

Required test:

```text
proof_rejects_cluster_mismatch
```

### T4 — RPC equivocation

Attack:

```text
coordinator is fed false or stale data by one RPC
```

Controls:

```text
single-RPC proof forbidden
independent multi-RPC quorum
minimum commitment
slot context
transaction status confirmation
challenge window
audit trail
```

Required tests:

```text
proof_rejects_single_rpc
proof_rejects_rpc_quorum_disagreement
proof_rejects_below_minimum_commitment
```

### T5 — Reorg / stale finality assumption

Attack:

```text
system treats observation as final before sufficient proof window
```

Controls:

```text
minimum commitment
challenge window
delayed finality
status remains pending until accepted
manual review for conflicts
```

Required test:

```text
stale_observation_does_not_finalize
```

### T6 — Coordinator compromise

Attack:

```text
svc-interop coordinator marks proof accepted without valid quorum
```

Controls:

```text
coordinator cannot mutate ledger
policy gate independently validates proof conditions
svc-wallet requires approved intent
audit trail required
halt switch freezes pending finalizations
```

Required drill:

```text
coordinator_compromised_during_challenge_window
```

### T7 — Emergency halt bypass

Attack:

```text
new or pending bridge action completes after halt
```

Controls:

```text
halt blocks new prepares
halt blocks pending finalizations
halt blocks mint submission
halt blocks internal issue
halt requires audited recovery path
```

Required tests:

```text
halt_blocks_new_prepare
halt_blocks_pending_finalization
halt_blocks_internal_issue
halt_blocks_external_mint_submission
```

### T8 — Stuck challenge forever

Attack/failure:

```text
request remains CHALLENGE_OPEN indefinitely
```

Controls:

```text
challenge max duration
stuck review timer
governance review path
refund path for internal burn before external mint
recovery account path for external burn before internal issue
```

Required tests:

```text
challenge_open_expires_to_review_required
internal_burn_stuck_path_can_refund_after_review
external_burn_stuck_path_routes_to_recovery_account
```

### T9 — Malicious CPI / wrong token program

Attack:

```text
Anchor instruction is called with wrong token program, mint, authority, or account
```

Controls:

```text
token program constraint
mint constraint
PDA seed constraint
mint authority constraint
recipient token account mint constraint
source token account mint constraint
no unchecked authority through remaining accounts
```

Required tests:

```text
anchor_rejects_wrong_token_program
anchor_rejects_wrong_mint
anchor_rejects_wrong_mint_authority
anchor_rejects_malicious_cpi_attempt
```

### T10 — Upgrade authority compromise

Attack:

```text
program is upgraded to malicious bridge logic
```

Controls:

```text
multi-sig upgrade authority
threshold policy
key rotation ceremony
emergency halt
verifiable builds
deployment artifact hashes
public program id registry
audit event
```

Required drills:

```text
upgrade_authority_rotation
compromised_upgrade_key_halt
verifiable_build_reproduction
```

### T11 — Client stale status

Attack/failure:

```text
CrabLink shows completed/final based on stale cache or offline status
```

Controls:

```text
backend-derived status only
offline status shows pending verification
no cache-only finality
no cache-only paid unlock
stale status warning
```

Required tests:

```text
crablink_offline_bridge_status_is_not_final
crablink_stale_status_shows_pending_verification
crablink_cache_cannot_complete_bridge
```

### T12 — Product language / liquidity creep

Attack/failure:

```text
docs or UI imply exchange, conversion, guaranteed value, instant settlement, or tradability
```

Controls:

```text
forbidden wording scanner
risk disclaimer template
separate liquidity decision gate
separate staking decision gate
separate exchange-facing risk model
```

Forbidden UI terms:

```text
convert
exchange
trade
swap
instant
guaranteed
cash out
risk-free
guaranteed value
```

Required test:

```text
bridge_ui_forbidden_wording_scanner
```

---

## 4. Abuse cases

```text
User replays old burn receipt.
User submits devnet proof against mainnet config.
RPC provider lies about transaction status.
Coordinator tries to approve proof alone.
Gateway tries to mark finality.
Omnigate hydrates stale complete status.
CrabLink cache shows bridge complete offline.
Policy config enables bridge without halt/caps.
Anchor instruction accepts wrong mint.
Upgrade authority deploys malicious program.
Challenge spam blocks all finalization.
Internal burn succeeds but external mint fails.
External burn succeeds but internal issue fails.
```

Every abuse case must have a future test or drill before runtime.

---

## 5. Required config gates

ron-policy must reject bridge enablement unless all exist:

```text
bridge.enabled=false by default
allowed_cluster
allowed_program_id
allowed_mint
allowed_token_program_id
minimum_commitment
rpc_quorum_threshold
challenge_min_duration_ms
challenge_max_duration_ms
per_user_daily_cap
global_daily_cap
pause_flag
halt_flag
recovery_account
upgrade_authority_policy
emergency_signer_threshold
forbidden_ui_wording_check_enabled
```

If any are absent, bridge runtime must fail closed.

---

## 6. Severity table

| Threat | Severity | Required before runtime |
| --- | --- | --- |
| Double mint | Critical | nonce + replay + Anchor tests |
| Double issue | Critical | nonce + wallet idempotency + ledger replay tests |
| RPC equivocation | Critical | multi-RPC quorum |
| Halt bypass | Critical | halt pending finalization tests |
| Upgrade compromise | Critical | multi-sig + verifiable build + drills |
| Malicious CPI | High | Anchor account constraint tests |
| Stuck challenge | High | recovery timeout and governance path |
| Stale UI | High | stale/offline UX tests |
| Product language creep | Medium/High | wording scanner and disclaimers |

---

## 7. Red-team prompt

Use this prompt for future LLM/human review:

```text
You are reviewing the RustyOnions / CrabLink ROC ↔ ROX bridge design. Be adversarial. Look for double-mint, double-issue, replay, cluster replay, Solana RPC equivocation, false finality, challenge-window griefing, stuck states, emergency halt bypass, coordinator compromise, Anchor account constraint gaps, malicious CPI, wrong token program, upgrade authority compromise, key rotation gaps, recovery account abuse, stale UI finality, forbidden exchange language, liquidity creep, and any bypass of svc-wallet/ron-ledger truth. Do not suggest weakening the invariant that svc-wallet is the only internal ROC mutation front-door and ron-ledger is internal durable economic truth.
```

---

## 8. Threat model decision

Current decision:

```text
Threat model created.
Runtime remains unauthorized.
Next safe action is docs review and checker execution.
```
