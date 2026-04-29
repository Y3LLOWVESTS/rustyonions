Absolutely. Here is a **paste-ready next-session TODO list** designed to push RustyOnions from roughly **79–83% beta completion** to around **88–91%**, excluding the browser plugin.

The key strategy: **do not scatter into every WEB3_2 feature at once.** The fastest path to 90% is to finish the **server-side beta loop**:

```text
paid storage/pinning
→ wallet hold receipt
→ storage enforcement
→ accounting export
→ rewarder plan
→ wallet issue/capture/release
→ ledger commit
→ receipt/admin visibility
→ b3 asset manifest foundation
→ roc-economics config foundation
```

WEB3.MD defines the WEB3 v1 done state as durable/replayable ledger, wallet as sole mutation front-door, a real paid use case, deterministic reward issuance, and visible metrics/dashboard economy movement.  WEB3_2 then adds the product layer: passports, `crab://`, b3 asset pages, configurable ROC economics, paid site/content flows, and dashboard/admin visibility. 

---

# RustyOnions Next Session TODO — Push Toward 90% Beta

## Current estimated completion

```text
Overall beta excluding browser plugin: ~79–83%
Target after this work:              ~88–91%
Stretch target:                      ~92% if all gates go green
```

---

## 0. Non-negotiable rules for the next session

Completion: **must stay 100% respected**

Do not violate these while chasing speed:

```text
- ROC stays internal only.
- No ROX / Solana / external settlement.
- No floating-point money math.
- All ROC amounts use integer minor units.
- All percentages use basis points.
- ron-ledger remains append-only durable truth.
- svc-wallet remains the sole normal mutation front-door.
- ron-accounting remains transient metering/snapshots, not truth.
- svc-rewarder plans deterministic rewards but does not mutate ledger directly.
- Paid flows use hold → capture → release.
- No direct ledger mutation from svc-storage, svc-gateway, omnigate, accounting, rewarder, dashboard, passport, or plugin code.
- b3:<64 lowercase hex> remains the canonical content ID.
```

WEB3_2 explicitly preserves these value-plane, identity/capability, content-addressing, and service-boundary invariants, including the rule that paid enforcement belongs in storage/gateway/omnigate while economic mutation flows through wallet and durable truth remains in ledger. 

---

# 1. Finish the live paid-storage ROC loop

Current completion: **~84–88% system-level**
Target after section: **~91–94% system-level**

This is the highest-impact path to 90%.

`svc-storage` is already very close to beta-ready: it supports free CAS writes/reads, HEAD/range reads, BLAKE3 `b3:<64 lowercase hex>` addressing, paid `/paid/o` writes, wallet hold receipt validation, configurable verifier mode, real HTTP wallet receipt lookup, paid-write metrics, and a cross-crate paid storage loop with `svc-wallet` and `ron-accounting`. 

Remaining TODO:

```text
1. Run a live real svc-wallet + svc-storage smoke test.
   Goal:
   - Start svc-wallet on real local port.
   - Start svc-storage in wallet-receipt mode.
   - Issue ROC to test account.
   - Create wallet hold.
   - POST /paid/o using real hold receipt txid.
   - Verify object is stored.
   - Verify mismatched receipt rejects.
   - Verify /o or HEAD can retrieve metadata/content.

2. Strengthen hold context binding.
   Goal:
   - Receipt must bind to expected payer account.
   - Receipt must bind to expected action, e.g. storage:put or paid_storage_write.
   - Receipt must bind to expected amount or minimum amount.
   - Receipt must bind to asset/content CID or intended operation if available.
   - Receipt must reject if account/action/amount/context mismatch.

3. Add/finish capture-release automation strategy.
   Goal:
   - Decide whether svc-storage captures directly or emits settlement event.
   - For beta, prefer simplest safe path:
     storage validates hold → write succeeds → storage calls wallet capture OR emits deterministic capture request.
   - On write failure: release hold.
   - On write success but capture failure: emit recoverable failed-settlement event.
   - Add tests for:
     successful write + capture
     failed write + release
     write success + capture failure quarantine

4. Preserve dev-header mode for local testing.
   Goal:
   - wallet-receipt mode is production-shaped.
   - dev-header mode stays available for local beta/dev compatibility.
   - disabled mode should fail closed for paid routes unless explicitly intended.

5. Add one end-to-end script.
   Suggested path:
   scripts/web3_paid_storage_live_smoke.sh

   It should prove:
   - wallet starts
   - storage starts
   - issue works
   - hold works
   - paid write works
   - receipt lookup works
   - accounting event emitted or captured
   - object retrieval works
```

Expected completion gain:

```text
+3% to +5% overall beta completion
```

---

# 2. Add production accounting export path from svc-storage

Current completion: **~70–78% for live accounting integration**
Target after section: **~86–90%**

Right now, storage usage events are proven in tests and returned/ingested in-process, but production needs a real export path. The notes list remaining storage gaps as real accounting export, capture/release automation, stronger hold binding, durable storage/amnesia decisions, expanded metrics, and real auth/passport integration. 

TODO:

```text
1. Define the beta accounting export mechanism.
   Recommended beta path:
   - HTTP POST from svc-storage to ron-accounting ingest endpoint if that exists.
   - If no service endpoint exists yet, add a small local exporter trait and mock sink first.
   - Keep bus/WAL exporter as later hardening unless already scaffolded.

2. Add a StorageUsageExporter trait.
   Suggested shape:
   - export_usage_event(event, idempotency_key) -> Result<(), ExportError>
   - Mock exporter for tests.
   - No-op exporter for dev mode.
   - HTTP exporter for beta mode.

3. Add idempotency key for every usage event.
   Suggested key:
   storage_usage:{operation}:{cid}:{payer}:{txid}:{timestamp/window}

4. Add retry/backoff.
   Beta minimum:
   - bounded retry count
   - deterministic error taxonomy
   - fail observable, not silent

5. Add dead-letter/quarantine behavior.
   Beta minimum:
   - if export fails after retries, log structured warning
   - increment metric
   - optionally write local quarantine file if durable mode is enabled
   - in amnesia mode, keep bounded in-memory quarantine only

6. Add tests.
   Required:
   - paid write emits usage event
   - duplicate export is idempotent
   - exporter failure does not corrupt stored object
   - exporter failure is visible via metric/log
   - amnesia mode does not silently persist forbidden state
```

Expected completion gain:

```text
+2% to +4% overall beta completion
```

---

# 3. Wire accounting → rewarder → wallet → ledger integration test

Current completion: **~76–82% for full value-plane system loop**
Target after section: **~88–92%**

`ron-accounting` is a gold foundation crate at **~88–91%**, with usage events, normalized labels, transient counters, deterministic sealed slices, export lanes, optional WAL, rewarder-compatible snapshots, deterministic reward projection, and b3 CID proofs. Its recommended next focus is specifically the value-plane integration test: **accounting → rewarder → wallet → ledger**. 

TODO:

```text
1. Create a cross-crate integration test.
   Suggested path:
   crates/ron-accounting/tests/web3_accounting_rewarder_wallet_loop.rs
   OR
   tests/web3_accounting_rewarder_wallet_loop.rs

2. Test flow:
   - Create usage events.
   - Ingest into ron-accounting.
   - Seal accounting window/snapshot.
   - Feed snapshot into svc-rewarder.
   - svc-rewarder computes deterministic payout/issue plan.
   - Submit issue request to svc-wallet.
   - svc-wallet commits through ron-ledger.
   - Fetch wallet receipt.
   - Verify ledger replay/balance agrees.
   - Verify same input produces same reward plan and receipt shape.

3. Assertions:
   - no floating point
   - deterministic ordering
   - deterministic reward plan
   - no direct ledger mutation from accounting/rewarder
   - wallet is mutation front-door
   - ledger balance changes only after wallet issue
   - receipt is retrievable
   - idempotent replay does not double issue

4. Add failure tests:
   - duplicate snapshot does not double reward
   - malformed accounting snapshot rejects
   - reward amount exceeding policy cap rejects
   - wallet issue failure surfaces cleanly
```

Expected completion gain:

```text
+4% to +6% overall beta completion
```

This is probably the single biggest “we are really 90%” proof.

---

# 4. Implement roc-economics.toml foundation

Current completion: **~35–45%**
Target after section: **~70–80% foundation**

WEB3_2 recommends the first implementation location for configurable economics in `ron-policy` under a dedicated economics module, with paths like `crates/ron-policy/src/economics/{mod.rs,types.rs,load.rs,validate.rs}`, tests in `crates/ron-policy/tests/economics_policy.rs`, and `configs/roc-economics.toml`. 

TODO:

```text
1. Add configs/roc-economics.toml.
   Include beta-safe defaults:
   - storage write price
   - storage pin price
   - paid content view price
   - site visit price
   - provider split bps
   - owner split bps
   - curator split bps
   - network/reserve split bps
   - max spend per action
   - min charge amount

2. Add ron-policy economics module.
   Suggested files:
   crates/ron-policy/src/economics/mod.rs
   crates/ron-policy/src/economics/types.rs
   crates/ron-policy/src/economics/load.rs
   crates/ron-policy/src/economics/validate.rs

3. Validation rules:
   - all money amounts are integer minor units
   - all percentages are basis points
   - payout splits must sum to 10_000 bps where required
   - no negative values
   - max spend must be >= price
   - unknown action rejects unless explicitly allowed
   - missing required recipient rejects for paid action
   - config parse errors are deterministic

4. Add tests:
   crates/ron-policy/tests/economics_policy.rs

   Required cases:
   - valid config loads
   - invalid split sum rejects
   - negative/overflow value rejects
   - unknown paid action rejects
   - disabled action rejects
   - deterministic output ordering
   - no floating point anywhere

5. Expose a small lookup API:
   - price_for(action, asset_kind/site_kind)
   - split_for(action)
   - validate_capture_plan(action, recipients, amount)
```

Expected completion gain:

```text
+2% to +4% overall beta completion
```

This unlocks paid content/site visits later without hardcoding economics.

---

# 5. Start b3 asset page foundation in ron-naming

Current completion: **~35–45%**
Target after section: **~65–75% foundation**

WEB3_2 says every uploaded content object should receive a BLAKE3 content ID in canonical `b3:<64 lowercase hex>` form, and that asset manifests can add ownership, payout address, tags, description, and curator metadata without changing raw bytes. 

It also recommends beginning b3 asset page foundation in:

```text
crates/ron-naming/src/crab.rs
crates/ron-naming/src/asset.rs
crates/ron-naming/tests/crab_links.rs
crates/ron-naming/tests/asset_manifest.rs
```



TODO:

```text
1. Add crab link parser.
   Path:
   crates/ron-naming/src/crab.rs

   Supported beta forms:
   - crab://b3/<64hex>.image
   - crab://b3/<64hex>.song
   - crab://b3/<64hex>.article
   - crab://b3/<64hex>.video
   - crab://b3/<64hex>.comment
   - crab://b3/<64hex>.page
   - crab://b3/<64hex>.manifest

2. Parser output:
   - scheme = crab
   - namespace = b3
   - hash = normalized lowercase 64 hex
   - asset_kind = enum
   - canonical_string()

3. Add asset manifest DTO/types.
   Path:
   crates/ron-naming/src/asset.rs

   Minimum fields:
   - version
   - asset_cid
   - asset_kind
   - owner.passport_subject
   - owner.wallet_account
   - payout.default_action
   - payout.recipient_account
   - metadata.title
   - metadata.description
   - metadata.tags
   - metadata.license

4. Validation:
   - asset_cid must be canonical b3
   - asset_kind must match crab suffix when provided
   - payout account required for paid asset
   - owner passport subject required for ownership claim
   - tags bounded
   - no unknown asset kind unless generic mode is explicit

5. Tests:
   crates/ron-naming/tests/crab_links.rs
   crates/ron-naming/tests/asset_manifest.rs

   Required:
   - valid typed crab links parse
   - uppercase hash normalizes or rejects by policy
   - invalid hash length rejects
   - invalid suffix rejects
   - manifest validates
   - manifest missing owner rejects
   - manifest missing payout rejects when paid action is enabled
```

Expected completion gain:

```text
+2% to +4% overall beta completion
```

This is the beginning of the jaw-drop asset-page layer.

---

# 6. Add server-side b3 asset page resolver stub

Current completion: **~25–35%**
Target after section: **~55–65% foundation**

This should be **server-side/dev-tool only** for now, since we are excluding the browser plugin.

TODO:

```text
1. Add a simple resolver path in svc-gateway or omnigate.
   Preferred beta shape:
   GET /v1/assets/b3/:hash/:kind
   OR
   GET /v1/crab/resolve?url=crab://b3/<hash>.<kind>

2. Resolver should:
   - parse crab link using ron-naming
   - validate b3 hash
   - ask svc-index for manifest metadata if available
   - ask svc-storage for object metadata/HEAD
   - return JSON asset page DTO

3. Beta response shape:
   {
     "asset_cid": "b3:<64hex>",
     "asset_kind": "image",
     "manifest": { ... },
     "storage": {
       "available": true,
       "size_bytes": 123,
       "content_type": "image/png"
     },
     "owner": { ... },
     "payout": { ... },
     "links": {
       "raw": "/o/<cid>",
       "paid_view": "/v1/pay/content-view/..."
     }
   }

4. Tests:
   - valid asset resolves
   - missing manifest returns generic asset page
   - missing storage object returns not_found/degraded cleanly
   - invalid crab URL rejects
   - invalid b3 rejects
```

Expected completion gain:

```text
+2% to +3% overall beta completion
```

This turns b3 addressing into a visible product surface without needing the extension yet.

---

# 7. Add paid content/view server-side prototype

Current completion: **~20–35%**
Target after section: **~55–65% beta foundation**

This is a stretch goal for today. Do it after sections 1–5 if time remains.

WEB3_2’s definition of done includes paid content routes requiring wallet holds, capture paying configured recipient splits from `roc-economics.toml`, site owners earning ROC from visits/content, asset owners earning ROC from media/articles, users seeing receipts, and dashboard/admin showing paid access/ownership/payout activity. 

TODO:

```text
1. Add a server-side paid content route.
   Suggested:
   POST /v1/pay/content-view/prepare
   POST /v1/pay/content-view/capture

2. prepare:
   - accepts asset_cid, asset_kind, viewer_account
   - loads roc-economics price
   - returns required hold amount and recipient split preview

3. capture:
   - verifies wallet hold receipt
   - validates hold context
   - calls svc-wallet capture or emits capture intent
   - returns final receipt

4. Start with single-recipient owner payout.
   Then add split recipients:
   - owner
   - provider
   - curator
   - reserve/network

5. Tests:
   - paid view requires hold
   - underfunded hold rejects
   - wrong asset_cid rejects
   - split sum conserved
   - capture receipt retrievable
```

Expected completion gain:

```text
+2% to +4% overall beta completion
```

---

# 8. Dashboard/admin visibility for economy movement

Current completion: **~45–60% depending on dashboard state**
Target after section: **~70–80% beta visibility**

The README lists dashboard visibility for balances, receipts, token flows, and network activity as still in progress. 

Do not overbuild the UI today. Add server/admin JSON first.

TODO:

```text
1. Add economy status endpoint.
   Suggested:
   GET /api/v1/econ/status

   Return:
   - total issued
   - total burned
   - active holds
   - captured total
   - released total
   - receipt count
   - latest receipt ids
   - accounting windows available
   - latest reward epoch status

2. Add asset/economy activity endpoint.
   Suggested:
   GET /api/v1/econ/activity

   Return latest:
   - paid storage writes
   - paid content views
   - reward plans
   - wallet captures/releases
   - accounting exports

3. Add tests:
   - endpoint returns stable JSON
   - no private keys/secrets exposed
   - empty state works
   - after paid storage smoke, activity appears

4. Optional dashboard UI:
   - simple card for wallet supply
   - simple table for latest receipts
   - simple table for paid storage events
```

Expected completion gain:

```text
+1.5% to +3% overall beta completion
```

Visibility is what makes the demo feel real.

---

# 9. Real auth/passport capability seam, server-side only

Current completion: **~35–50% for WEB3_2 passport layer**
Target after section: **~55–65% server-side foundation**

Since browser plugin is excluded, do not build extension UX. Build the **server-side capability contract** only.

TODO:

```text
1. Define wallet capability caveats for beta.
   Required caveats:
   - subject
   - account
   - asset = roc
   - action
   - max_amount_minor
   - audience = svc-wallet
   - expires_at
   - optional site id
   - optional asset_cid

2. Add test fixtures for:
   - wallet:read
   - wallet:hold
   - wallet:capture
   - wallet:release
   - storage:put
   - content:view
   - asset:claim

3. Wire fail-closed checks where easy:
   - paid storage requires storage:put or wallet hold proof
   - wallet hold requires wallet:hold capability in production mode
   - asset claim requires asset:claim capability

4. Keep dev mode available:
   - production mode must fail closed
   - dev mode may use local/dev bearer for smoke tests
```

Expected completion gain:

```text
+1.5% to +3% overall beta completion
```

WEB3_2 lists example capabilities including `network:access`, `site:visit`, `site:publish`, `asset:claim`, `asset:publish`, `wallet:read`, `wallet:hold`, `wallet:capture`, `wallet:release`, `storage:put`, and `content:view`, with wallet caveats such as account, asset, max amount, audience, expiry, route, site, asset CID, and action. 

---

# 10. Storage metrics expansion

Current completion: **~65–75%**
Target after section: **~85–90%**

TODO:

```text
Add or verify metrics:

- storage_put_total
- storage_get_total
- storage_paid_put_total
- storage_paid_put_rejected_total
- storage_wallet_receipt_lookup_total
- storage_wallet_receipt_lookup_failed_total
- storage_bytes_written_total
- storage_bytes_read_total
- storage_put_latency_seconds
- storage_get_latency_seconds
- storage_objects_total
- storage_fs_errors_total
- storage_accounting_export_total
- storage_accounting_export_failed_total
- storage_capture_total
- storage_release_total
- storage_settlement_failed_total

Tests:
- metrics endpoint includes names
- paid success increments success metrics
- paid rejection increments rejection metrics
- accounting export failure increments failure metric
```

Expected completion gain:

```text
+1% to +2% overall beta completion
```

---

# 11. Final beta proof script

Current completion: **not yet consolidated**
Target after section: **90% credibility proof**

TODO:

```text
Create one script that proves the whole beta story.

Suggested path:
scripts/web3_beta_smoke.sh

It should run:

1. cargo fmt
2. cargo clippy on touched crates
3. cargo test on touched crates
4. start svc-wallet
5. start svc-storage in wallet-receipt mode
6. issue ROC
7. create hold
8. paid storage write
9. retrieve object
10. export accounting event
11. seal accounting snapshot
12. run rewarder plan
13. issue reward through wallet
14. verify ledger/balance/receipt
15. resolve b3 asset manifest or crab parser test
16. print final “WEB3 beta loop green”
```

Expected completion gain:

```text
+2% to +4% confidence/completion
```

This is the thing that lets us honestly say, “we are around 90% beta.”

---

# Recommended execution order for today

## Batch 1 — fastest path to 85–87%

```text
1. Live real svc-wallet + svc-storage paid storage smoke.
2. Stronger hold context binding.
3. Add final paid-storage live smoke script.
4. Run full gates:
   cargo fmt
   cargo clippy -p svc-storage --all-targets -- -D warnings
   cargo test -p svc-storage --all-targets
   cargo clippy -p svc-wallet --all-targets -- -D warnings
   cargo test -p svc-wallet --all-targets
```

## Batch 2 — push to 88–90%

```text
5. Add production-shaped accounting export seam.
6. Add accounting export tests.
7. Add accounting → rewarder → wallet → ledger integration test.
8. Run gates:
   cargo clippy -p ron-accounting --all-targets -- -D warnings
   cargo test -p ron-accounting --all-targets
   cargo test -p ron-accounting --all-targets --features wal
   cargo clippy -p svc-rewarder --all-targets -- -D warnings
   cargo test -p svc-rewarder --all-targets
```

## Batch 3 — stretch beyond 90%

```text
9. Add configs/roc-economics.toml.
10. Add ron-policy economics parser/validator/tests.
11. Add ron-naming crab link parser and asset manifest foundation.
12. Add server-side asset resolver stub if time remains.
```

---

# Completion-rate dashboard

| Section                             |  Current | After TODO |             Impact |
| ----------------------------------- | -------: | ---------: | -----------------: |
| Core substrate                      |   86–90% |     88–91% |             Medium |
| svc-wallet                          |   90–93% |     93–95% |             Medium |
| ron-ledger                          |   90–94% |     91–95% |         Low/medium |
| ron-accounting                      |   88–91% |     91–94% | High if integrated |
| svc-rewarder                        |   78–84% |     86–90% |               High |
| svc-storage paid beta               |   93–96% |     96–98% |               High |
| Full paid storage loop              |   84–88% |     92–95% |          Very high |
| Accounting → rewarder → wallet loop |   70–78% |     88–92% |          Very high |
| roc-economics config                |   35–45% |     70–80% |        Medium/high |
| b3 asset/crab foundation            |   35–45% |     65–75% |        Medium/high |
| Server-side paid content            |   20–35% |     55–65% |            Stretch |
| Dashboard/admin economy visibility  |   45–60% |     70–80% |             Medium |
| Passport/capability server seam     |   35–50% |     55–65% |             Medium |
| Browser plugin                      | Excluded |   Excluded |                N/A |

---

# Best honest estimate after today

If we complete **Batch 1 only**:

```text
Overall beta excluding browser plugin: ~84–87%
```

If we complete **Batch 1 + Batch 2**:

```text
Overall beta excluding browser plugin: ~88–91%
```

If we complete **Batch 1 + Batch 2 + most of Batch 3**:

```text
Overall beta excluding browser plugin: ~91–93%
```

---

# Paste-ready opening prompt for next session

```text
We are continuing RustyOnions WEB3/ROC work. Treat WEB3.MD and WEB3_2.MD as the active north-star documents. Exclude browser plugin work for now. Current estimated beta completion is ~79–83%, and today’s goal is to push toward 90%+ by finishing the server-side value-plane loop.

Prioritize in this order:

1. Live real svc-wallet + svc-storage paid storage smoke.
2. Strengthen wallet hold receipt context binding for paid storage.
3. Add capture/release automation or deterministic settlement-intent path for paid storage.
4. Add production-shaped accounting export from svc-storage.
5. Add accounting → rewarder → wallet → ledger integration test.
6. Add configs/roc-economics.toml and ron-policy economics parser/validator/tests.
7. Add ron-naming crab://b3 parser and asset manifest DTO/tests.
8. Add server-side asset resolver stub if time remains.
9. Add admin/economy JSON visibility endpoints if time remains.

Preserve all invariants:
- ROC internal only.
- No ROX/Solana/external settlement.
- No floating-point money math.
- Integer minor units only.
- Basis points for percentages.
- svc-wallet remains the only normal economic mutation front-door.
- ron-ledger remains append-only durable truth.
- ron-accounting remains transient metering/snapshots.
- svc-rewarder does not mutate ledger directly.
- paid operations use hold → capture → release.
- b3:<64 lowercase hex> remains canonical content ID.
- no direct ledger mutation from svc-storage, svc-gateway, omnigate, accounting, rewarder, passport, dashboard, or extension/plugin code.

Before generating code, review CODECHECK.md and use CODECOMMENTS.MD header conventions for full paste-ready files. Provide full file replacements with exact paths.
```

---

My recommendation: **start with Batch 2 if you want the biggest completion jump.** Paid storage is already strong. The thing that turns “80%” into “90%” is proving the full economic circuit:

```text
usage → accounting → rewarder → wallet → ledger → receipt visibility
```
