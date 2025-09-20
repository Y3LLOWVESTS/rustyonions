Absolutely—here’s the consolidated, scan-aware update. I’ve locked the **33-crate roster** and mapped them to the **12 pillars** so you can refactor and review against a single source of truth.

# Updated crate list (canonical 33)

The list below applies your adds/merges/removals (e.g., **`ron-billing` → folded into `ron-ledger`**; **deprecated crates dropped; `svc-edge` excluded** to avoid role overlap). &#x20;

1. ron-kernel
2. ron-bus
3. ryker
4. ron-policy
5. ron-kms
6. ron-auth
7. svc-passport
8. ron-audit
9. metrics
10. svc-gateway
11. omnigate
12. ron-app-sdk
13. oap
14. micronode
15. macronode
16. svc-storage
17. svc-index
18. naming
19. tldctl
20. svc-overlay
21. transport
22. svc-arti-transport
23. svc-mailbox
24. svc-sandbox
25. ron-ledger  (absorbs reporting/“billing”)
26. svc-wallet
27. accounting
28. svc-rewarder
29. svc-registry
30. svc-mod
31. svc-ads
32. interop
33. ron-proto&#x20;

# The 12 pillars (boundary contracts) + crate mapping

These pillars are your review “chapters.” Every crate/PR proves compliance against all relevant gates. &#x20;

1. **Kernel & Orchestration** — lifecycle, supervision, backoff
   Crates: `ron-kernel`, `ron-bus`, `ryker`.
   Signals for good separation: tight interfaces, DAG deps, kernel at root.&#x20;

2. **Policy & Capability Control** — decide, don’t execute
   Crates: `ron-policy` (quotas/pricing/splits; no side-effects).&#x20;

3. **Identity & Key Management** — custody & authz
   Crates: `ron-kms`, `ron-auth`, `svc-passport`. PQ-hybrid & zeroization expectations live here.&#x20;

4. **Audit & Compliance** — evidence, not metrics or money
   Crates: `ron-audit`. Tamper-evident receipts/trails.&#x20;

5. **Observability** — health/metrics/tracing only
   Crates: `metrics`. No policy/evidence in here.&#x20;

6. **Ingress (Gateway)** — neutral entry, enforcement/termination
   Crates: `svc-gateway`. (We explicitly keep **`svc-edge` out** to preserve clean roles.)&#x20;

7. **App BFF & SDK** — DTO shaping & developer experience
   Crates: `omnigate`, `ron-app-sdk`, `oap`, `micronode`, `macronode`. Keep this distinct from ingress and ledger.&#x20;

8. **Content Addressing & Naming** — storage vs. mapping vs. semantics
   Crates: `svc-storage`, `svc-index`, `naming`, `tldctl`. Preserve the trio boundary explicitly.&#x20;

9. **Overlay & Transport** — moving bits (not storing them)
   Crates: `svc-overlay`, `transport`, `svc-arti-transport`, `svc-mailbox`.&#x20;

10. **Discovery / Relay / Safety** — isolation, containment
    Crates: `svc-sandbox`. Not routing, not money—pure safety.&#x20;

11. **Economics & Wallets** — value movement, usage accounting
    Crates: `ron-ledger` (money moves here; includes reporting), `svc-wallet`, `accounting`, `svc-rewarder` (ZK/commitment helper). Economic invariants apply (e.g., Σ credits == debits). &#x20;

12. **Governance & Interop** — registry, moderation, ads, bridges, protocol
    Crates: `svc-registry`, `svc-mod`, `svc-ads`, `interop`, `ron-proto` (shared DTO/error contracts).&#x20;

# How this ties into refactor gates (what to enforce)

Use the same six “constitution” chapters and their concrete checks during PR review: Runtime Safety; Interop & App Experience; Boundaries & Security; Verification & Scale; Economics & Governance; Usability & Ops. Each chapter has explicit gates (e.g., **no locks across `.await`**, **deny\_unknown\_fields on DTOs**, **no ambient authority**, **SLOs**, **economic conservation**, **docs/metrics**). That checklist is already sketched and ready to apply to the crates above. &#x20;

# Enforcement glue (already drafted)

To make the pillars “real,” apply the lint wall + custom `xtask` checks and CI shown in your notes (Clippy hygiene, AST rules for public fields/finite enums/serde tags, **ASan/TSan**, **cargo-deny**, README sync, invariant-tag scan). This plugs directly into the pillars and prevents drift as you refactor.    &#x20;


* **No contrived roles.** Every crate has a single sentence that isn’t duplicative of another. If we can’t write that sentence, we removed it.
* **No math errors.** It’s a real 33, with deprecated libs out, ZK kept explicit (`svc-rewarder`), billing merged into ledger, and `ron-proto` kept as a cross-service contract surface.
* **Boundaries hold.** Storage≠Index≠Naming; Gateway≠Omnigate; Overlay≠Transport; Ledger≠Accounting; Policy decides, Ledger executes, Audit records, Metrics observes.
