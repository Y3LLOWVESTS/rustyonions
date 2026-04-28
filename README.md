
# RustyOnions

> **For the benefit of humanity.**

RustyOnions is an experimental Rust platform for building self-hostable, content-addressed application infrastructure.

It combines a cloud-like node runtime, storage, indexing, gateway routing, identity, capabilities, SDKs, admin tooling, and an internal ROC token plane into one modular system.

The project is currently in **developer preview / proof-of-concept**. It is not production-ready and is not intended for public token launches, custody of external assets, exchange activity, or external chain settlement.

---

## What RustyOnions is

RustyOnions is a self-hostable application substrate.

It is designed to let developers and operators run their own RustyOnions node and launch applications with built-in:

- content-addressed storage
- indexing and naming
- gateway / app routing
- admin dashboard visibility
- local and operator node profiles
- SDK-based app integration
- capability-gated access
- identity and passport primitives
- deterministic receipts
- internal ROC token accounting
- creator/provider token allocation flows

Think of it as an experimental, Rust-native foundation for building content-addressed apps, self-hosted services, and future token-aware application networks.

RustyOnions is not just a WEB3 layer.  
The WEB3 / ROC work is the current active focus on top of a broader infrastructure stack.

---

## Current Focus

RustyOnions currently has two major tracks:

### 1. Core Infrastructure

The core infrastructure is the self-hostable substrate:

```text
Micronode / Macronode
+ gateway
+ omnigate
+ storage
+ index
+ naming
+ policy
+ identity
+ metrics
+ admin dashboard
+ SDKs
```

This layer is what makes RustyOnions feel like a self-hostable cloud platform.

### 2. WEB3 / ROC Internal Token Plane

The WEB3 layer adds ROC-based internal token accounting:

```text
content addressing
+ local identity
+ capability-gated access
+ internal ROC token accounting
+ token-enforced storage and access
+ deterministic receipts
+ creator/provider token allocation
```

ROC is an internal accounting token used to prove the ecosystem.
ROX, Solana, public bridges, staking, liquidity pools, and exchange-facing logic are deferred.

---

## Current Status

**Status:** Developer preview
**Date:** April 2026

Working or actively tested:

* core RustyOnions substrate crates
* Micronode developer profile
* Macronode operator profile
* admin dashboard developer preview
* static site demo through Omnigate
* storage / index / gateway infrastructure
* SDK integration work
* ROC token-plane foundation:

  * `ron-ledger`
  * `svc-wallet`
  * `ron-accounting`
  * `svc-rewarder`
* wallet holds, captures, releases, and receipts
* token-enforced storage / pinning smoke tests
* b3 content addressing and `crab://` product planning

In progress:

* full end-to-end ROC loop:

  * usage
  * accounting
  * reward planning
  * wallet mutation
  * ledger commit
  * receipt visibility
* RON Passport product flow
* browser extension UX for `crab://`
* b3 asset pages
* configurable ROC token rules
* dashboard visibility for balances, receipts, token flows, and network activity
* expanded SDK examples

Deferred:

* ROX
* Solana
* external chain settlement
* staking
* liquidity
* exchange-facing features

---

## Node Profiles

RustyOnions has two main node profiles.

### Micronode

Micronode is the developer-friendly profile.

It is intended for:

* local development
* app demos
* small self-hosted deployments
* quick experiments
* SDK testing
* local dashboard testing
* single-node RustyOnions apps

Micronode is the “start here” path.

### Macronode

Macronode is the operator-grade profile.

It is intended for:

* larger deployments
* multi-service composition
* gateway / storage / index orchestration
* operator dashboards
* service supervision
* production-style observability
* future hosted RustyOnions infrastructure

Macronode is the “run the network infrastructure” path.

---

## Quick Start

### Build the workspace

```bash
cargo build --workspace
```

### Run the admin dashboard

```bash
export SVC_ADMIN_AUTH_MODE=local
export SVC_ADMIN_AUTH_BOOTSTRAP_ADMIN_USERNAME=admin
export SVC_ADMIN_AUTH_BOOTSTRAP_ADMIN_PASSWORD='password'

bash scripts/run_dashboard.sh
```

Open:

```text
http://localhost:5173/
```

Login:

```text
username: admin
password: password
```

### Run the static site demo

```bash
bash scripts/demo_site.sh
```

Open:

```text
http://127.0.0.1:5304/app/site
```

Useful paths:

```text
crates/omnigate/src/routes/v1/app.rs
examples/site-demo/
```

---

## Platform Layers

RustyOnions is organized into four major layers:

```text
Core Runtime
  Kernel, bus, supervision, metrics, readiness

Cloud Substrate
  Gateway, Omnigate, storage, index, naming, policy, admin dashboard

Developer Layer
  SDKs, examples, app routes, dashboard UX, local tooling

ROC Token Plane
  Ledger, wallet, accounting, reward planning, receipts
```

This keeps the project modular.

The infrastructure can stand on its own, and the ROC token plane builds on top of it.

---

## Core Concepts

### Content Addressing

Every uploaded object receives a BLAKE3 content ID:

```text
b3:<64 lowercase hex>
```

The hash is the canonical address of the bytes.

Examples of objects that should receive b3 IDs:

* images
* videos
* songs
* posts
* comments
* pages
* articles
* manifests
* site bundles
* app bundles

Names can point to content, but the b3 hash is the canonical identity of the object.

---

### Omnigate

Omnigate is the app-facing composition layer.

It helps turn lower-level RustyOnions services into application views by coordinating:

* app routes
* storage reads
* index lookups
* manifest hydration
* policy checks
* token-plane hooks
* dashboard-visible activity

The static site demo currently runs through Omnigate.

---

### SDKs

RustyOnions is intended to support multiple developer SDKs.

The SDK layer exists so apps can interact with RustyOnions without needing to manually understand every service boundary.

SDK goals include:

* typed requests
* stable DTOs
* retries and deadlines
* idempotency
* structured errors
* app-friendly APIs
* browser and backend integration

Current and planned SDK work includes:

* Rust SDK
* TypeScript SDK
* future Swift, Kotlin, Java, PHP, Python, and other bindings

---

### RON Passports

A RON Passport is a local identity credential.

A passport is not a wallet.

```text
Passport = identity / permissions
Wallet   = ROC account
Cap      = short-lived scoped authority
```

Passports may be:

* main identity passports
* alternate anonymous/pseudonymous passports
* wallet-linked through explicit permissions
* scoped by account, action, amount, audience, and expiry

Alternate passports must not automatically inherit main-passport wallet authority.

---

### crab:// Links

`crab://` is the planned RustyOnions navigation scheme.

Examples:

```text
crab://sealobsta
crab://b3/<64hex>.image
crab://b3/<64hex>.song
crab://b3/<64hex>.article
crab://b3/<64hex>.video
crab://u/<passport>
```

The goal is to support RustyOnions navigation through a browser extension and local resolver rather than requiring a full custom browser.

---

### b3 Asset Pages

Typed asset pages are manifest-backed pages for b3-addressed content.

Example:

```text
crab://b3/<64hex>.image
```

An asset page can show:

* owner
* allocation account
* tags
* description
* provenance
* curator metadata
* storage/provider data
* receipts
* related assets

The bytes remain immutable. Metadata and ownership claims live in manifests and signed records around the content ID.

---

### ROC

ROC is RustyOnions’ internal accounting token.

The current goal is to prove a deterministic closed-loop token system before any external chain work.

Important rules:

* integer minor units only
* basis points for percentages
* no negative balances by default
* all wallet mutations require nonce and idempotency
* transfers conserve token units
* issue and burn are explicit audited exceptions
* token operations use `hold → capture → release`
* receipts must be stable and replayable

---

## Main Crates

| Area                     | Examples                                                     |
| ------------------------ | ------------------------------------------------------------ |
| Runtime / orchestration  | `ron-kernel`, `ron-bus`, `ryker`                             |
| Protocol / DTOs          | `oap`, `ron-proto`                                           |
| Gateway / app hydration  | `svc-gateway`, `omnigate`                                    |
| Storage / index / naming | `svc-storage`, `svc-index`, `ron-naming`                     |
| Identity / auth / keys   | `svc-passport`, `ron-auth`, `ron-kms`                        |
| Policy / audit / metrics | `ron-policy`, `ron-audit`, `ron-metrics`                     |
| Node profiles            | `micronode`, `macronode`                                     |
| ROC token plane          | `ron-ledger`, `svc-wallet`, `ron-accounting`, `svc-rewarder` |
| Admin / dashboard        | `svc-admin`                                                  |
| SDKs                     | `ron-app-sdk`, TypeScript SDK, future SDKs                   |

Generated review files such as `CODEBUNDLE_RS.md`, `ALLNOTES.MD`, and `ALL_DOCS_COMBINED.MD` are local workflow artifacts, not source of truth.

---

## Development Workflow

Before committing:

```bash
cargo fmt
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

For targeted crate work:

```bash
cargo fmt
cargo clippy -p svc-wallet --all-targets -- -D warnings
cargo test -p svc-wallet --all-targets
```

Replace `svc-wallet` with the crate being changed.

---

## Project Rules

RustyOnions preserves strict boundaries:

* no app-specific logic in `ron-kernel`
* no direct ledger mutation from storage, gateway, accounting, rewarder, dashboard, or extension code
* `svc-wallet` is the normal token-plane mutation front-door
* `ron-ledger` is durable token truth
* `ron-accounting` is transient metering/snapshots
* `svc-rewarder` plans rewards but does not mutate the ledger directly
* capability checks fail closed
* queues are bounded
* services expose truthful health/readiness/metrics
* avoid locks across `.await`
* prefer safe, idiomatic Rust

---

## Roadmap

Near-term priorities:

1. Keep repo root clean and current.
2. Keep Micronode and Macronode demos easy to run.
3. Expand SDK examples.
4. Improve dashboard visibility.
5. Finish the end-to-end ROC loop.
6. Prove token-enforced storage with wallet receipts.
7. Build the RON Passport local identity path.
8. Add `crab://` parsing and browser-extension UX.
9. Add b3 asset pages and manifest-backed ownership/allocation metadata.
10. Expand token-enforced access from storage to content and site/app use cases.

Long-term deferred work:

* ROX
* external chain settlement
* public bridge
* staking
* liquidity
* exchange-facing systems

---

## Important Warning

This is experimental software.

Do not use RustyOnions for:

* production hosting
* custody of external assets
* public token launches
* exchange activity
* illegal or abusive content
* claims of production privacy/security without independent review

ROC is a closed-loop internal accounting token for proving the RustyOnions ecosystem. It is not currently a public cryptocurrency.

---

## Contributing

Useful contributions include:

* reproducible bug reports
* build/test reports
* docs cleanup
* examples
* dashboard UX feedback
* parser tests
* property tests
* ROC invariant tests
* SDK examples
* service-boundary tests

Large changes should explain:

```text
what changes
why it belongs in that crate
which boundary it touches
which invariants must remain true
how it is tested
```

---

## Legal and Safety

Do not use RustyOnions for illegal content, exploitation, harassment, malware, doxxing, non-consensual material, unauthorized data sharing, external asset custody, or public token claims before the internal token system is proven.

RustyOnions is experimental. Use responsibly.

---

## Credits

Created by **Stevan White** with assistance from **OpenAI’s ChatGPT** and **xAI’s Grok**.

---

## License

MIT — see `LICENSE`.

