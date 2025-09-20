# Refactor Report (Pro, fast defaults)

- Generated: 2025-09-18 14:46:57 CDT
- Window for churn: 120 days ago
- Target: x86_64-unknown-linux-gnu

## 1) Summary Tables

### Ranked Crates (by reverse dependents, then churn)

| crate | rdep_count | forward_deps | churn_90d | loc_rs | instability |
|------:|-----------:|-------------:|----------:|-------:|------------:|
| workspace-hack | 21 | 0 | 8 | 3 | 0.00 |
| naming | 5 | 1 | 28 | 336 | 0.17 |
| ron-bus | 5 | 1 | 16 | 151 | 0.17 |
| index | 3 | 2 | 20 | 63 | 0.40 |
| ron-kernel | 2 | 2 | 182 | 3123 | 0.50 |
| transport | 2 | 2 | 41 | 513 | 0.50 |
| ron-policy | 2 | 0 | 2 | 31 | 0.00 |
| ron-app-sdk | 1 | 1 | 54 | 994 | 0.50 |
| overlay | 1 | 2 | 53 | 209 | 0.67 |
| common | 1 | 1 | 34 | 149 | 0.50 |
| accounting | 1 | 1 | 25 | 217 | 0.50 |
| oap | 1 | 1 | 15 | 321 | 0.50 |
| ryker | 1 | 1 | 15 | 67 | 0.50 |
| kameo | 1 | 1 | 13 | 139 | 0.50 |
| ron-billing | 1 | 1 | 2 | 105 | 0.50 |
| ron-proto | 1 | 0 | 2 | 166 | 0.00 |
| gateway | 0 | 6 | 122 | 2210 | 1.00 |
| node | 0 | 4 | 57 | 557 | 1.00 |
| svc-omnigate | 0 | 2 | 49 | 1392 | 1.00 |
| gwsmoke | 0 | 1 | 26 | 760 | 1.00 |
| tldctl | 0 | 4 | 21 | 373 | 1.00 |
| svc-overlay | 0 | 2 | 20 | 206 | 1.00 |
| svc-index | 0 | 4 | 18 | 130 | 1.00 |
| actor_spike | 0 | 2 | 15 | 132 | 1.00 |
| svc-storage | 0 | 2 | 15 | 91 | 1.00 |
| ronctl | 0 | 2 | 13 | 157 | 1.00 |
| svc-sandbox | 0 | 0 | 8 | 463 | 0.00 |
| ron-kms | 0 | 1 | 4 | 242 | 1.00 |
| micronode | 0 | 1 | 2 | 128 | 1.00 |
| ron-audit | 0 | 0 | 2 | 75 | 0.00 |
| ron-auth | 0 | 0 | 2 | 188 | 0.00 |
| svc-edge | 0 | 1 | 2 | 128 | 1.00 |

_Instability = Ce / (Ca + Ce). Lower is more stable (core/kernel); higher is leaf/adapter._

### Feature Hotspots (top 25 per crate)

| crate | feature | count |
|------:|:--------|------:|

### Duplicate Dependencies

```text
getrandom v0.2.16
├── rand_core v0.6.4
│   ├── ed25519-dalek v2.2.0
│   │   └── ron-audit v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/ron-audit)
│   ├── rand v0.8.5
│   │   ├── ron-audit v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/ron-audit)
│   │   ├── ron-auth v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/ron-auth)
│   │   └── ryker v0.2.0 (/Users/mymac/Desktop/RustyOnions/crates/ryker)
│   │       └── tldctl v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/tldctl)
│   └── rand_chacha v0.3.1
│       └── rand v0.8.5 (*)
└── ring v0.17.14
    ├── rustls v0.23.31
    │   ├── hyper-rustls v0.27.7
    │   │   └── reqwest v0.12.23
    │   │       ├── gateway v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/gateway)
    │   │       ├── gwsmoke v0.1.0 (/Users/mymac/Desktop/RustyOnions/testing/gwsmoke)
    │   │       ├── ron-kernel v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/ron-kernel)
    │   │       │   ├── actor_spike v0.2.0 (/Users/mymac/Desktop/RustyOnions/experiments/actor_spike)
    │   │       │   └── gateway v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/gateway)
    │   │       └── workspace-hack v0.1.0 (/Users/mymac/Desktop/RustyOnions/workspace-hack)
    │   │           ├── accounting v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/accounting)
    │   │           │   └── transport v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/transport)
    │   │           │       ├── node v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/node)
    │   │           │       └── overlay v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/overlay)
    │   │           │           └── node v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/node)
    │   │           ├── actor_spike v0.2.0 (/Users/mymac/Desktop/RustyOnions/experiments/actor_spike)
    │   │           ├── common v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/common)
    │   │           │   └── node v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/node)
    │   │           ├── gateway v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/gateway)
    │   │           ├── gwsmoke v0.1.0 (/Users/mymac/Desktop/RustyOnions/testing/gwsmoke)
    │   │           ├── index v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/index)
    │   │           │   ├── gateway v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/gateway)
    │   │           │   ├── svc-index v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/svc-index)
    │   │           │   └── tldctl v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/tldctl)
    │   │           ├── kameo v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/kameo)
    │   │           ├── naming v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/naming)
    │   │           │   ├── gateway v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/gateway)
    │   │           │   ├── index v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/index) (*)
    │   │           │   ├── ron-billing v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/ron-billing)
    │   │           │   │   └── ryker v0.2.0 (/Users/mymac/Desktop/RustyOnions/crates/ryker) (*)
    │   │           │   ├── svc-index v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/svc-index)
    │   │           │   └── tldctl v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/tldctl)
    │   │           ├── node v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/node)
    │   │           ├── oap v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/oap)
    │   │           │   └── gateway v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/gateway)
    │   │           ├── overlay v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/overlay) (*)
    │   │           ├── ron-app-sdk v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/ron-app-sdk)
    │   │           │   └── svc-omnigate v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/svc-omnigate)
    │   │           ├── ron-bus v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/ron-bus)
    │   │           │   ├── gateway v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/gateway)
    │   │           │   ├── ronctl v0.1.0 (/Users/mymac/Desktop/RustyOnions/tools/ronctl)
    │   │           │   ├── svc-index v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/svc-index)
    │   │           │   ├── svc-overlay v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/svc-overlay)
    │   │           │   └── svc-storage v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/svc-storage)
    │   │           ├── ron-kernel v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/ron-kernel) (*)
    │   │           ├── ronctl v0.1.0 (/Users/mymac/Desktop/RustyOnions/tools/ronctl)
    │   │           ├── svc-index v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/svc-index)
    │   │           ├── svc-omnigate v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/svc-omnigate)
    │   │           ├── svc-overlay v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/svc-overlay)
    │   │           ├── svc-storage v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/svc-storage)
    │   │           ├── tldctl v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/tldctl)
    │   │           └── transport v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/transport) (*)
    │   ├── reqwest v0.12.23 (*)
    │   ├── svc-omnigate v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/svc-omnigate)
    │   └── tokio-rustls v0.26.2
    │       ├── hyper-rustls v0.27.7 (*)
    │       ├── reqwest v0.12.23 (*)
    │       ├── ron-app-sdk v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/ron-app-sdk) (*)
    │       ├── ron-kernel v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/ron-kernel) (*)
    │       └── svc-omnigate v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/svc-omnigate)
    └── rustls-webpki v0.103.5
        └── rustls v0.23.31 (*)

getrandom v0.3.3
├── rand_core v0.9.3
│   ├── rand v0.9.2
│   │   ├── actor_spike v0.2.0 (/Users/mymac/Desktop/RustyOnions/experiments/actor_spike)
│   │   ├── gateway v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/gateway)
│   │   ├── retry v2.1.0
│   │   │   └── transport v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/transport) (*)
│   │   ├── ron-kernel v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/ron-kernel) (*)
│   │   ├── ron-kms v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/ron-kms)
│   │   ├── ronctl v0.1.0 (/Users/mymac/Desktop/RustyOnions/tools/ronctl)
│   │   ├── svc-omnigate v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/svc-omnigate)
│   │   ├── svc-sandbox v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/svc-sandbox)
│   │   └── ulid v1.2.1
│   │       └── svc-omnigate v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/svc-omnigate)
│   ├── rand_chacha v0.9.0
│   │   ├── actor_spike v0.2.0 (/Users/mymac/Desktop/RustyOnions/experiments/actor_spike)
│   │   ├── gateway v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/gateway)
│   │   ├── rand v0.9.2 (*)
│   │   ├── ronctl v0.1.0 (/Users/mymac/Desktop/RustyOnions/tools/ronctl)
│   │   └── workspace-hack v0.1.0 (/Users/mymac/Desktop/RustyOnions/workspace-hack) (*)
│   └── workspace-hack v0.1.0 (/Users/mymac/Desktop/RustyOnions/workspace-hack) (*)
└── uuid v1.18.1
    ├── cfb v0.7.3
    │   └── infer v0.15.0
    │       └── tldctl v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/tldctl)
    ├── naming v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/naming) (*)
    └── ron-auth v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/ron-auth)

parking_lot v0.11.2
└── sled v0.34.7
    ├── index v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/index) (*)
    └── overlay v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/overlay) (*)

parking_lot v0.12.4
├── accounting v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/accounting) (*)
├── prometheus v0.14.0
│   ├── gateway v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/gateway)
│   ├── micronode v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/micronode)
│   ├── ron-kernel v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/ron-kernel) (*)
│   ├── svc-edge v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/svc-edge)
│   ├── svc-omnigate v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/svc-omnigate)
│   └── svc-sandbox v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/svc-sandbox)
├── ron-kernel v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/ron-kernel) (*)
├── ron-kms v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/ron-kms)
├── svc-sandbox v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/svc-sandbox)
└── tokio v1.47.1
    ├── actor_spike v0.2.0 (/Users/mymac/Desktop/RustyOnions/experiments/actor_spike)
    ├── async-compression v0.4.30
    │   └── tower-http v0.6.6
    │       ├── gateway v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/gateway)
    │       └── reqwest v0.12.23 (*)
    ├── axum v0.7.9
    │   ├── gateway v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/gateway)
    │   ├── gwsmoke v0.1.0 (/Users/mymac/Desktop/RustyOnions/testing/gwsmoke)
    │   ├── micronode v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/micronode)
    │   ├── ron-kernel v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/ron-kernel) (*)
    │   ├── svc-edge v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/svc-edge)
    │   ├── svc-sandbox v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/svc-sandbox)
    │   └── workspace-hack v0.1.0 (/Users/mymac/Desktop/RustyOnions/workspace-hack) (*)
    ├── gateway v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/gateway)
    ├── gwsmoke v0.1.0 (/Users/mymac/Desktop/RustyOnions/testing/gwsmoke)
    ├── h2 v0.4.12
    │   └── hyper v1.7.0
    │       ├── axum v0.7.9 (*)
    │       ├── gateway v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/gateway)
    │       ├── hyper-rustls v0.27.7 (*)
    │       ├── hyper-util v0.1.16
    │       │   ├── axum v0.7.9 (*)
    │       │   ├── gateway v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/gateway)
    │       │   ├── hyper-rustls v0.27.7 (*)
    │       │   ├── reqwest v0.12.23 (*)
    │       │   ├── svc-omnigate v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/svc-omnigate)
    │       │   └── workspace-hack v0.1.0 (/Users/mymac/Desktop/RustyOnions/workspace-hack) (*)
    │       ├── reqwest v0.12.23 (*)
    │       ├── svc-omnigate v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/svc-omnigate)
    │       └── workspace-hack v0.1.0 (/Users/mymac/Desktop/RustyOnions/workspace-hack) (*)
    ├── hyper v1.7.0 (*)
    ├── hyper-rustls v0.27.7 (*)
    ├── hyper-util v0.1.16 (*)
    ├── kameo v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/kameo)
    ├── micronode v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/micronode)
    ├── node v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/node)
    ├── oap v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/oap) (*)
    ├── overlay v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/overlay) (*)
    ├── reqwest v0.12.23 (*)
    ├── ron-app-sdk v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/ron-app-sdk) (*)
    ├── ron-kernel v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/ron-kernel) (*)
    ├── ryker v0.2.0 (/Users/mymac/Desktop/RustyOnions/crates/ryker) (*)
    ├── svc-edge v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/svc-edge)
    ├── svc-omnigate v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/svc-omnigate)
    ├── svc-sandbox v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/svc-sandbox)
    ├── tokio-rustls v0.26.2 (*)
    ├── tokio-socks v0.5.2
    │   └── transport v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/transport) (*)
    ├── tokio-util v0.7.16
    │   ├── gateway v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/gateway)
    │   ├── h2 v0.4.12 (*)
    │   ├── ron-app-sdk v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/ron-app-sdk) (*)
    │   ├── svc-omnigate v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/svc-omnigate)
    │   ├── tower-http v0.6.6 (*)
    │   └── workspace-hack v0.1.0 (/Users/mymac/Desktop/RustyOnions/workspace-hack) (*)
    ├── tower v0.5.2
    │   ├── axum v0.7.9 (*)
    │   ├── gateway v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/gateway)
    │   ├── reqwest v0.12.23 (*)
    │   ├── svc-sandbox v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/svc-sandbox)
    │   ├── tower-http v0.6.6 (*)
    │   └── workspace-hack v0.1.0 (/Users/mymac/Desktop/RustyOnions/workspace-hack) (*)
    ├── tower-http v0.6.6 (*)
    ├── transport v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/transport) (*)
    └── workspace-hack v0.1.0 (/Users/mymac/Desktop/RustyOnions/workspace-hack) (*)
    [dev-dependencies]
    └── ron-kernel v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/ron-kernel) (*)

parking_lot_core v0.8.6
└── parking_lot v0.11.2 (*)

parking_lot_core v0.9.11
└── parking_lot v0.12.4 (*)

rand v0.8.5 (*)

rand v0.9.2 (*)

rand_chacha v0.3.1 (*)

rand_chacha v0.9.0 (*)

rand_core v0.6.4 (*)

rand_core v0.9.3 (*)

thiserror v1.0.69
├── common v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/common) (*)
├── gateway v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/gateway)
├── naming v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/naming) (*)
├── overlay v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/overlay) (*)
├── protobuf v3.7.2
│   └── prometheus v0.14.0 (*)
├── protobuf-support v3.7.2
│   └── protobuf v3.7.2 (*)
├── ron-app-sdk v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/ron-app-sdk) (*)
├── ron-auth v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/ron-auth)
├── ron-bus v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/ron-bus) (*)
├── ron-kms v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/ron-kms)
├── ron-proto v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/ron-proto)
│   └── ron-kms v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/ron-kms)
└── tokio-socks v0.5.2 (*)

thiserror v2.0.16
├── oap v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/oap) (*)
└── prometheus v0.14.0 (*)

thiserror-impl v1.0.69 (proc-macro)
└── thiserror v1.0.69 (*)

thiserror-impl v2.0.16 (proc-macro)
└── thiserror v2.0.16 (*)
```

## 2) Cycles & Forbidden Edges (Heuristics)

**Cycle guesses (from cargo tree --graph):**

```text
```

**Forbidden edge guesses (importing 'kernel::internal')**

```text
```

## 3) Public API Surface (optional)

- Note: nightly rustdoc JSON failed
- Estimated public items: **0**

## 4) Build Timing (optional)

| crate | elapsed_seconds | max_rss_kb |
|------:|----------------:|-----------:|
| workspace-hack | 0.00 | 4:out=refactor_full.md |
| naming | 0.00 | 4:out=refactor_full.md |
| ron-bus | 0.00 | 4:out=refactor_full.md |
| index | 0.00 | 4:out=refactor_full.md |
| ron-kernel | 0.00 | 5:out=refactor_full.md |
| transport | 0.00 | 5:out=refactor_full.md |
| ron-policy | 0.00 | 11:out=refactor_full.md |
| ron-app-sdk | 0.00 | 4:out=refactor_full.md |
| overlay | 0.00 | 4:out=refactor_full.md |
| common | 0.00 | 4:out=refactor_full.md |

## 5) Serde Wire-Contract Scan

**Found serde attributes:**

```text
crates/ron-proto/src/lib.rs:51:pub struct Signature(#[serde(with = "serde_bytes")] pub Vec<u8>);
crates/svc-omnigate/src/handlers/mailbox.rs:16:#[serde(rename_all = "lowercase", tag = "op")]
crates/svc-omnigate/src/handlers/mailbox.rs:21:        #[serde(default)]
crates/svc-omnigate/src/handlers/mailbox.rs:26:        #[serde(default = "default_max")]
crates/ron-auth/src/lib.rs:25:    #[serde(with = "serde_scopes")]
crates/ron-auth/src/lib.rs:50:        #[serde(with = "serde_scopes")]
crates/gateway/src/pay_enforce.rs:15:    #[serde(default)]
crates/gateway/src/pay_enforce.rs:17:    #[serde(default)]
crates/gateway/src/pay_enforce.rs:19:    #[serde(default)]
crates/gateway/src/pay_enforce.rs:21:    #[serde(default)]
crates/gateway/src/pay_enforce.rs:23:    #[serde(default)]
crates/gateway/src/pay_enforce.rs:29:    #[serde(default)]
crates/naming/src/tld.rs:6:#[serde(rename_all = "lowercase")]
crates/ron-kernel/src/bin/node_index.rs:28:    #[serde(skip_serializing_if = "Option::is_none")]
crates/ron-kernel/src/bin/node_index.rs:30:    #[serde(skip_serializing_if = "Option::is_none")]
crates/naming/src/manifest.rs:27:    #[serde(default, skip_serializing_if = "Vec::is_empty")]
crates/naming/src/manifest.rs:32:    #[serde(default, skip_serializing_if = "Option::is_none")]
crates/naming/src/manifest.rs:36:    #[serde(default, skip_serializing_if = "Option::is_none")]
crates/naming/src/manifest.rs:40:    #[serde(default, skip_serializing_if = "Option::is_none")]
crates/naming/src/manifest.rs:45:    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
crates/naming/src/manifest.rs:63:    #[serde(default)]
crates/naming/src/manifest.rs:66:    #[serde(default)]
crates/naming/src/manifest.rs:69:    #[serde(default)]
crates/naming/src/manifest.rs:72:    #[serde(default)]
crates/naming/src/manifest.rs:75:    #[serde(default)]
crates/naming/src/manifest.rs:78:    #[serde(default)]
crates/naming/src/manifest.rs:81:    #[serde(default, skip_serializing_if = "Vec::is_empty")]
crates/naming/src/manifest.rs:95:    #[serde(default)]
crates/naming/src/manifest.rs:98:    #[serde(default)]
crates/naming/src/manifest.rs:101:    #[serde(default)]
crates/common/src/lib.rs:67:    #[serde(default)]
```

**Public enums missing serde tag/rename_all (heuristic):**

```text
crates/naming/src/tld.rs:7:pub enum TldType {
crates/naming/src/tld.rs:50:pub enum TldParseError {
crates/node/src/cli.rs:25:pub enum Cmd {
crates/naming/src/address.rs:29:pub enum AddressParseError {
crates/ron-kms/src/lib.rs:21:pub enum KmsError {
crates/ron-auth/src/lib.rs:16:pub enum Plane { Node, App }
crates/ron-auth/src/lib.rs:143:pub enum VerifyError {
crates/ron-ledger/src/lib.rs:18:pub enum Op {
crates/ron-ledger/src/lib.rs:44:pub enum TokenError {
crates/ron-proto/src/lib.rs:19:pub enum ProtoError {
crates/ron-proto/src/lib.rs:32:pub enum Algo {
crates/ron-billing/src/lib.rs:11:pub enum PriceModel {
crates/ron-bus/src/api.rs:25:pub enum IndexReq {
crates/ron-bus/src/api.rs:33:pub enum IndexResp {
crates/ron-bus/src/api.rs:43:pub enum StorageReq {
crates/ron-bus/src/api.rs:60:pub enum StorageResp {
crates/ron-bus/src/api.rs:70:pub enum OverlayReq {
crates/ron-bus/src/api.rs:82:pub enum OverlayResp {
crates/overlay/src/error.rs:8:pub enum OverlayError {
crates/kameo/src/lib.rs:33:pub enum Mailbox<M> {
crates/svc-sandbox/src/oap_stub.rs:6:pub enum HandshakeError {
crates/svc-sandbox/src/tarpit.rs:7:pub enum Mode { Redirect, Mirror, Tarpit }
crates/oap/src/lib.rs:31:pub enum FrameType {
crates/oap/src/lib.rs:64:pub enum OapError {
crates/ron-app-sdk/src/errors.rs:9:pub enum Error {
crates/svc-omnigate/src/oap_limits.rs:28:pub enum RejectReason {
crates/ron-kernel/src/lib.rs:22:pub enum KernelEvent {
```

## 6) Sanitizer & Supply-Chain Summaries (optional)

**cargo-deny summary:**

```text
error[banned]: crate 'rand = 0.8.5' is explicitly banned
    ┌─ /Users/mymac/Desktop/RustyOnions/deny.toml:122:9
    │
122 │ name = "rand"
    │         ━━━━ banned here
123 │ version = "< 0.9.0"
124 │ reason = "Unify rand to 0.9.x"
    │           ─────────────────── reason
    │
    ├ rand v0.8.5
      ├── ron-audit v0.1.0
      ├── ron-auth v0.1.0
      └── ryker v0.2.0
          └── tldctl v0.1.0

error[duplicate]: found 2 duplicate entries for crate 'rand'
    ┌─ /Users/mymac/Desktop/RustyOnions/Cargo.lock:203:1
    │  
203 │ ╭ rand 0.8.5 registry+https://github.com/rust-lang/crates.io-index
204 │ │ rand 0.9.2 registry+https://github.com/rust-lang/crates.io-index
    │ ╰────────────────────────────────────────────────────────────────┘ lock entries
    │  
    ├ rand v0.8.5
      ├── ron-audit v0.1.0
      ├── ron-auth v0.1.0
      └── ryker v0.2.0
          └── tldctl v0.1.0
    ├ rand v0.9.2
      ├── actor_spike v0.2.0
      ├── gateway v0.1.0
      ├── retry v2.1.0
      │   └── transport v0.1.0
      │       ├── node v0.1.0
      │       └── overlay v0.1.0
      │           └── node v0.1.0 (*)
      ├── ron-kernel v0.1.0
      │   ├── actor_spike v0.2.0 (*)
      │   └── gateway v0.1.0 (*)
      ├── ron-kms v0.1.0
      ├── ronctl v0.1.0
      ├── svc-omnigate v0.1.0
      ├── svc-sandbox v0.1.0
      └── ulid v1.2.1
          └── svc-omnigate v0.1.0 (*)

error[banned]: crate 'rand_chacha = 0.3.1' is explicitly banned
    ┌─ /Users/mymac/Desktop/RustyOnions/deny.toml:130:9
    │
130 │ name = "rand_chacha"
    │         ━━━━━━━━━━━ banned here
131 │ version = "< 0.9.0"
132 │ reason = "Unify rand_chacha to 0.9.x"
    │           ────────────────────────── reason
    │
    ├ rand_chacha v0.3.1
      └── rand v0.8.5
          ├── ron-audit v0.1.0
          ├── ron-auth v0.1.0
          └── ryker v0.2.0
              └── tldctl v0.1.0

error[duplicate]: found 2 duplicate entries for crate 'rand_chacha'
    ┌─ /Users/mymac/Desktop/RustyOnions/Cargo.lock:205:1
    │  
205 │ ╭ rand_chacha 0.3.1 registry+https://github.com/rust-lang/crates.io-index
206 │ │ rand_chacha 0.9.0 registry+https://github.com/rust-lang/crates.io-index
    │ ╰───────────────────────────────────────────────────────────────────────┘ lock entries
    │  
    ├ rand_chacha v0.3.1
      └── rand v0.8.5
          ├── ron-audit v0.1.0
          ├── ron-auth v0.1.0
          └── ryker v0.2.0
              └── tldctl v0.1.0
    ├ rand_chacha v0.9.0
      ├── actor_spike v0.2.0
      ├── gateway v0.1.0
      ├── rand v0.9.2
      │   ├── actor_spike v0.2.0 (*)
      │   ├── gateway v0.1.0 (*)
      │   ├── retry v2.1.0
      │   │   └── transport v0.1.0
      │   │       ├── node v0.1.0
      │   │       └── overlay v0.1.0
      │   │           └── node v0.1.0 (*)
      │   ├── ron-kernel v0.1.0
      │   │   ├── actor_spike v0.2.0 (*)
      │   │   └── gateway v0.1.0 (*)
      │   ├── ron-kms v0.1.0
      │   ├── ronctl v0.1.0
      │   ├── svc-omnigate v0.1.0
      │   ├── svc-sandbox v0.1.0
      │   └── ulid v1.2.1
      │       └── svc-omnigate v0.1.0 (*)
      ├── ronctl v0.1.0 (*)
      └── workspace-hack v0.1.0
          ├── accounting v0.1.0
          │   └── transport v0.1.0 (*)
          ├── actor_spike v0.2.0 (*)
          ├── common v0.1.0
          │   └── node v0.1.0 (*)
          ├── gateway v0.1.0 (*)
          ├── gwsmoke v0.1.0
          ├── index v0.1.0
          │   ├── gateway v0.1.0 (*)
          │   ├── svc-index v0.1.0
          │   └── tldctl v0.1.0
          ├── kameo v0.1.0
          │   └── ron-kernel v0.1.0 (*)
          ├── naming v0.1.0
          │   ├── gateway v0.1.0 (*)
          │   ├── index v0.1.0 (*)
          │   ├── ron-billing v0.1.0
          │   │   └── ryker v0.2.0
          │   │       └── tldctl v0.1.0 (*)
          │   ├── svc-index v0.1.0 (*)
          │   └── tldctl v0.1.0 (*)
          ├── node v0.1.0 (*)
          ├── oap v0.1.0
          │   └── gateway v0.1.0 (*)
          ├── overlay v0.1.0 (*)
          ├── ron-app-sdk v0.1.0
          │   └── svc-omnigate v0.1.0 (*)
          ├── ron-bus v0.1.0
          │   ├── gateway v0.1.0 (*)
          │   ├── ronctl v0.1.0 (*)
          │   ├── svc-index v0.1.0 (*)
          │   ├── svc-overlay v0.1.0
          │   └── svc-storage v0.1.0
          ├── ron-kernel v0.1.0 (*)
          ├── ronctl v0.1.0 (*)
          ├── svc-index v0.1.0 (*)
          ├── svc-omnigate v0.1.0 (*)
          ├── svc-overlay v0.1.0 (*)
          ├── svc-storage v0.1.0 (*)
          ├── tldctl v0.1.0 (*)
          └── transport v0.1.0 (*)

warning[workspace-duplicate]: crate axum = 0.7.9 is used 7 times in the workspace, but not all declarations use the shared workspace dependency
   ┌─ /Users/mymac/Desktop/RustyOnions/Cargo.toml:48:1
   │
48 │ axum                = { version = "0.7.9", default-features = false }
   │ ──── workspace dependency
   │
   ┌─ /Users/mymac/Desktop/RustyOnions/workspace-hack/Cargo.toml:13:1
   │
13 │ axum = { version = "0.7", default-features = false, features = ["http1", "http2", "json", "macros", "tokio"] }
   │ ━━━━
   │
   ├ axum v0.7.9
     ├── gateway v0.1.0
     ├── gwsmoke v0.1.0
     ├── micronode v0.1.0
     ├── ron-kernel v0.1.0
     │   ├── actor_spike v0.2.0
     │   └── gateway v0.1.0 (*)
     ├── svc-edge v0.1.0
     ├── svc-sandbox v0.1.0
     └── workspace-hack v0.1.0
         ├── accounting v0.1.0
         │   └── transport v0.1.0
         │       ├── node v0.1.0
         │       └── overlay v0.1.0
         │           └── node v0.1.0 (*)
         ├── actor_spike v0.2.0 (*)
         ├── common v0.1.0
         │   └── node v0.1.0 (*)
         ├── gateway v0.1.0 (*)
         ├── gwsmoke v0.1.0 (*)
         ├── index v0.1.0
         │   ├── gateway v0.1.0 (*)
         │   ├── svc-index v0.1.0
         │   └── tldctl v0.1.0
         ├── kameo v0.1.0
         │   └── ron-kernel v0.1.0 (*)
         ├── naming v0.1.0
         │   ├── gateway v0.1.0 (*)
         │   ├── index v0.1.0 (*)
         │   ├── ron-billing v0.1.0
         │   │   └── ryker v0.2.0
         │   │       └── tldctl v0.1.0 (*)
         │   ├── svc-index v0.1.0 (*)
         │   └── tldctl v0.1.0 (*)
         ├── node v0.1.0 (*)
         ├── oap v0.1.0
         │   └── gateway v0.1.0 (*)
         ├── overlay v0.1.0 (*)
         ├── ron-app-sdk v0.1.0
         │   └── svc-omnigate v0.1.0
         ├── ron-bus v0.1.0
         │   ├── gateway v0.1.0 (*)
         │   ├── ronctl v0.1.0
         │   ├── svc-index v0.1.0 (*)
         │   ├── svc-overlay v0.1.0
         │   └── svc-storage v0.1.0
         ├── ron-kernel v0.1.0 (*)
         ├── ronctl v0.1.0 (*)
         ├── svc-index v0.1.0 (*)
         ├── svc-omnigate v0.1.0 (*)
         ├── svc-overlay v0.1.0 (*)
```

**ASan run summary:**

```text
error: failed to load manifest for workspace member `/Users/mymac/.rustup/toolchains/nightly-x86_64-apple-darwin/lib/rustlib/src/rust/library/std`
referenced by workspace at `/Users/mymac/.rustup/toolchains/nightly-x86_64-apple-darwin/lib/rustlib/src/rust/library/Cargo.toml`

Caused by:
  failed to load manifest for dependency `panic_unwind`

Caused by:
  failed to load manifest for dependency `unwind`

Caused by:
  failed to read `/Users/mymac/.rustup/toolchains/nightly-x86_64-apple-darwin/lib/rustlib/src/rust/library/unwind/Cargo.toml`

Caused by:
  No such file or directory (os error 2)
```

**TSan run summary:**

```text
error: failed to load manifest for workspace member `/Users/mymac/.rustup/toolchains/nightly-x86_64-apple-darwin/lib/rustlib/src/rust/library/std`
referenced by workspace at `/Users/mymac/.rustup/toolchains/nightly-x86_64-apple-darwin/lib/rustlib/src/rust/library/Cargo.toml`

Caused by:
  failed to load manifest for dependency `panic_unwind`

Caused by:
  failed to load manifest for dependency `unwind`

Caused by:
  failed to read `/Users/mymac/.rustup/toolchains/nightly-x86_64-apple-darwin/lib/rustlib/src/rust/library/unwind/Cargo.toml`

Caused by:
  No such file or directory (os error 2)
```

## 7) Quick Smells (regex heuristics)

```text
=== PUBLIC STRUCT FIELDS (should be private + ctor) ===

=== STRINGLY FIELDS (status/state/kind/type/role/phase/mode/level) ===
crates/ron-kernel/src/amnesia.rs:4:    pub mode: String,
crates/ron-kernel/src/amnesia.rs:8:    pub fn new(mode: String) -> Self {
crates/svc-sandbox/src/decoy.rs:8:    pub content_type: String, // plausible MIME

=== FREE tokio::spawn OUTSIDE SUPERVISOR (grep) ===
crates/ryker/src/lib.rs:23:    tokio::spawn(async move {
crates/transport/src/tor/mod.rs:98:    let handle = tokio::spawn(async move {
crates/gateway/src/oap.rs:76:        let handle = tokio::spawn(async move {
crates/gateway/src/oap.rs:86:                        tokio::spawn(async move {
crates/kameo/src/lib.rs:131:    let handle = tokio::spawn(async move {
crates/overlay/src/protocol.rs:26:    tokio::spawn(async move {
crates/overlay/src/protocol.rs:41:        tokio::spawn(async move {
crates/svc-omnigate/src/admin_http.rs:81:        tokio::spawn(async move {
crates/svc-omnigate/src/main.rs:41:    tokio::spawn(admin_http::run(
crates/svc-omnigate/src/server.rs:142:        tokio::spawn(async move {

=== UNSAFE BLOCKS OUTSIDE ffi/ or hardening/ (grep) ===
crates/svc-sandbox/src/oap_stub.rs:66:        unsafe { String::from_utf8_unchecked(out) }
crates/svc-sandbox/src/decoy.rs:55:        unsafe { String::from_utf8_unchecked(out) }

=== FORBIDDEN EDGE HINT (imports of kernel internals) ===
```

## 8) Per-Crate Trees (reverse + forward)

### common

<details><summary>Reverse tree (-i common -e features)</summary>

```text
```

</details>

<details><summary>Forward tree (-p common -e features)</summary>

```text
common v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/common)
├── anyhow feature "default"
│   ├── anyhow v1.0.99
│   └── anyhow feature "std"
│       └── anyhow v1.0.99
├── blake3 feature "default"
│   ├── blake3 v1.8.2
│   │   ├── arrayvec v0.7.6
│   │   ├── constant_time_eq v0.3.1
│   │   ├── arrayref feature "default"
│   │   │   └── arrayref v0.3.9
│   │   └── cfg-if feature "default"
│   │       └── cfg-if v1.0.3
│   │   [build-dependencies]
│   │   └── cc feature "default"
│   │       └── cc v1.2.37
│   │           ├── jobserver v0.1.34
│   │           │   └── libc feature "default"
│   │           │       ├── libc v0.2.175
│   │           │       └── libc feature "std"
│   │           │           └── libc v0.2.175
│   │           ├── libc v0.2.175
│   │           ├── find-msvc-tools feature "default"
│   │           │   └── find-msvc-tools v0.1.1
│   │           └── shlex feature "default"
│   │               ├── shlex v1.3.0
│   │               └── shlex feature "std"
│   │                   └── shlex v1.3.0
│   └── blake3 feature "std"
│       └── blake3 v1.8.2 (*)
├── hex feature "default"
│   ├── hex v0.4.3
│   └── hex feature "std"
│       ├── hex v0.4.3
│       └── hex feature "alloc"
│           └── hex v0.4.3
├── serde feature "default"
│   ├── serde v1.0.221
│   │   ├── serde_core feature "result"
│   │   │   └── serde_core v1.0.221
│   │   └── serde_derive feature "default"
│   │       └── serde_derive v1.0.221 (proc-macro)
│   │           ├── proc-macro2 feature "proc-macro"
│   │           │   └── proc-macro2 v1.0.101
│   │           │       └── unicode-ident feature "default"
│   │           │           └── unicode-ident v1.0.19
│   │           ├── quote feature "proc-macro"
│   │           │   ├── quote v1.0.40
│   │           │   │   └── proc-macro2 v1.0.101 (*)
│   │           │   └── proc-macro2 feature "proc-macro" (*)
│   │           ├── syn feature "clone-impls"
│   │           │   └── syn v2.0.106
│   │           │       ├── proc-macro2 v1.0.101 (*)
│   │           │       ├── quote v1.0.40 (*)
│   │           │       └── unicode-ident feature "default" (*)
│   │           ├── syn feature "derive"
│   │           │   └── syn v2.0.106 (*)
│   │           ├── syn feature "parsing"
│   │           │   └── syn v2.0.106 (*)
│   │           ├── syn feature "printing"
│   │           │   └── syn v2.0.106 (*)
│   │           └── syn feature "proc-macro"
│   │               ├── syn v2.0.106 (*)
│   │               ├── proc-macro2 feature "proc-macro" (*)
│   │               └── quote feature "proc-macro" (*)
│   └── serde feature "std"
│       ├── serde v1.0.221 (*)
│       └── serde_core feature "std"
│           └── serde_core v1.0.221
├── serde_json feature "default"
│   ├── serde_json v1.0.144
│   │   ├── memchr v2.7.5
│   │   ├── serde_core v1.0.221
│   │   ├── itoa feature "default"
│   │   │   └── itoa v1.0.15
│   │   └── ryu feature "default"
│   │       └── ryu v1.0.20
│   └── serde_json feature "std"
│       ├── serde_json v1.0.144 (*)
│       ├── serde_core feature "std" (*)
│       └── memchr feature "std"
│           ├── memchr v2.7.5
│           └── memchr feature "alloc"
│               └── memchr v2.7.5
├── thiserror feature "default"
│   └── thiserror v1.0.69
│       └── thiserror-impl feature "default"
│           └── thiserror-impl v1.0.69 (proc-macro)
│               ├── proc-macro2 feature "default"
│               │   ├── proc-macro2 v1.0.101 (*)
│               │   └── proc-macro2 feature "proc-macro" (*)
│               ├── quote feature "default"
│               │   ├── quote v1.0.40 (*)
│               │   └── quote feature "proc-macro" (*)
│               └── syn feature "default"
│                   ├── syn v2.0.106 (*)
│                   ├── syn feature "clone-impls" (*)
│                   ├── syn feature "derive" (*)
│                   ├── syn feature "parsing" (*)
│                   ├── syn feature "printing" (*)
│                   └── syn feature "proc-macro" (*)
├── toml feature "default"
│   ├── toml v0.8.23
│   │   ├── serde feature "default" (*)
│   │   ├── serde_spanned feature "default"
│   │   │   └── serde_spanned v0.6.9
│   │   │       └── serde feature "default" (*)
│   │   ├── serde_spanned feature "serde"
│   │   │   └── serde_spanned v0.6.9 (*)
│   │   ├── toml_datetime feature "default"
│   │   │   └── toml_datetime v0.6.11
│   │   │       └── serde feature "default" (*)
│   │   ├── toml_datetime feature "serde"
│   │   │   └── toml_datetime v0.6.11 (*)
│   │   └── toml_edit feature "serde"
│   │       ├── toml_edit v0.22.27
│   │       │   ├── serde feature "default" (*)
│   │       │   ├── serde_spanned feature "default" (*)
│   │       │   ├── serde_spanned feature "serde" (*)
│   │       │   ├── toml_datetime feature "default" (*)
│   │       │   ├── indexmap feature "default"
│   │       │   │   ├── indexmap v2.11.1
│   │       │   │   │   ├── equivalent v1.0.2
│   │       │   │   │   └── hashbrown v0.15.5
│   │       │   │   │       ├── equivalent v1.0.2
│   │       │   │   │       ├── foldhash v0.1.5
│   │       │   │   │       └── allocator-api2 feature "alloc"
│   │       │   │   │           └── allocator-api2 v0.2.21
│   │       │   │   └── indexmap feature "std"
│   │       │   │       └── indexmap v2.11.1 (*)
│   │       │   ├── indexmap feature "std" (*)
│   │       │   ├── toml_write feature "default"
│   │       │   │   ├── toml_write v0.1.2
│   │       │   │   └── toml_write feature "std"
│   │       │   │       ├── toml_write v0.1.2
│   │       │   │       └── toml_write feature "alloc"
│   │       │   │           └── toml_write v0.1.2
│   │       │   └── winnow feature "default"
│   │       │       ├── winnow v0.7.13
│   │       │       └── winnow feature "std"
│   │       │           ├── winnow v0.7.13
│   │       │           └── winnow feature "alloc"
│   │       │               └── winnow v0.7.13
│   │       └── toml_datetime feature "serde" (*)
│   ├── toml feature "display"
│   │   ├── toml v0.8.23 (*)
│   │   └── toml_edit feature "display"
│   │       └── toml_edit v0.22.27 (*)
│   └── toml feature "parse"
│       ├── toml v0.8.23 (*)
│       └── toml_edit feature "parse"
│           └── toml_edit v0.22.27 (*)
└── workspace-hack feature "default"
    └── workspace-hack v0.1.0 (/Users/mymac/Desktop/RustyOnions/workspace-hack)
        ├── serde feature "alloc"
        │   ├── serde v1.0.221 (*)
        │   └── serde_core feature "alloc"
        │       └── serde_core v1.0.221
        ├── serde feature "default" (*)
        ├── serde feature "derive"
        │   ├── serde v1.0.221 (*)
        │   └── serde feature "serde_derive"
        │       └── serde v1.0.221 (*)
        ├── serde_core feature "alloc" (*)
        ├── serde_core feature "result" (*)
        ├── serde_core feature "std" (*)
        ├── serde_json feature "default" (*)
        ├── serde_json feature "raw_value"
        │   └── serde_json v1.0.144 (*)
        ├── memchr feature "default"
        │   ├── memchr v2.7.5
        │   └── memchr feature "std" (*)
        ├── hashbrown feature "default"
        │   ├── hashbrown v0.15.5 (*)
        │   ├── hashbrown feature "allocator-api2"
        │   │   └── hashbrown v0.15.5 (*)
        │   ├── hashbrown feature "default-hasher"
        │   │   └── hashbrown v0.15.5 (*)
        │   ├── hashbrown feature "equivalent"
        │   │   └── hashbrown v0.15.5 (*)
        │   ├── hashbrown feature "inline-more"
        │   │   └── hashbrown v0.15.5 (*)
        │   └── hashbrown feature "raw-entry"
        │       └── hashbrown v0.15.5 (*)
        ├── axum feature "http1"
        │   ├── axum v0.7.9
        │   │   ├── serde feature "default" (*)
        │   │   ├── serde_json feature "default" (*)
        │   │   ├── serde_json feature "raw_value" (*)
        │   │   ├── itoa feature "default" (*)
        │   │   ├── memchr feature "default" (*)
        │   │   ├── async-trait feature "default"
        │   │   │   └── async-trait v0.1.89 (proc-macro)
        │   │   │       ├── proc-macro2 feature "default" (*)
        │   │   │       ├── quote feature "default" (*)
        │   │   │       ├── syn feature "clone-impls" (*)
        │   │   │       ├── syn feature "full"
        │   │   │       │   └── syn v2.0.106 (*)
        │   │   │       ├── syn feature "parsing" (*)
        │   │   │       ├── syn feature "printing" (*)
        │   │   │       ├── syn feature "proc-macro" (*)
        │   │   │       └── syn feature "visit-mut"
        │   │   │           └── syn v2.0.106 (*)
        │   │   ├── axum-core feature "default"
        │   │   │   └── axum-core v0.4.5
        │   │   │       ├── async-trait feature "default" (*)
        │   │   │       ├── bytes feature "default"
        │   │   │       │   ├── bytes v1.10.1
        │   │   │       │   └── bytes feature "std"
        │   │   │       │       └── bytes v1.10.1
        │   │   │       ├── futures-util feature "alloc"
        │   │   │       │   ├── futures-util v0.3.31
        │   │   │       │   │   ├── futures-core v0.3.31
        │   │   │       │   │   ├── futures-macro v0.3.31 (proc-macro)
        │   │   │       │   │   │   ├── proc-macro2 feature "default" (*)
        │   │   │       │   │   │   ├── quote feature "default" (*)
        │   │   │       │   │   │   ├── syn feature "default" (*)
        │   │   │       │   │   │   └── syn feature "full" (*)
        │   │   │       │   │   ├── futures-sink v0.3.31
        │   │   │       │   │   ├── futures-task v0.3.31
        │   │   │       │   │   ├── memchr feature "default" (*)
        │   │   │       │   │   ├── futures-channel feature "std"
        │   │   │       │   │   │   ├── futures-channel v0.3.31
        │   │   │       │   │   │   │   ├── futures-core v0.3.31
        │   │   │       │   │   │   │   └── futures-sink v0.3.31
        │   │   │       │   │   │   ├── futures-channel feature "alloc"
        │   │   │       │   │   │   │   ├── futures-channel v0.3.31 (*)
        │   │   │       │   │   │   │   └── futures-core feature "alloc"
        │   │   │       │   │   │   │       └── futures-core v0.3.31
        │   │   │       │   │   │   └── futures-core feature "std"
        │   │   │       │   │   │       ├── futures-core v0.3.31
        │   │   │       │   │   │       └── futures-core feature "alloc" (*)
        │   │   │       │   │   ├── futures-io feature "std"
        │   │   │       │   │   │   └── futures-io v0.3.31
        │   │   │       │   │   ├── pin-project-lite feature "default"
        │   │   │       │   │   │   └── pin-project-lite v0.2.16
        │   │   │       │   │   ├── pin-utils feature "default"
        │   │   │       │   │   │   └── pin-utils v0.1.0
        │   │   │       │   │   └── slab feature "default"
        │   │   │       │   │       ├── slab v0.4.11
        │   │   │       │   │       └── slab feature "std"
        │   │   │       │   │           └── slab v0.4.11
        │   │   │       │   ├── futures-core feature "alloc" (*)
        │   │   │       │   └── futures-task feature "alloc"
        │   │   │       │       └── futures-task v0.3.31
        │   │   │       ├── pin-project-lite feature "default" (*)
        │   │   │       ├── http feature "default"
        │   │   │       │   ├── http v1.3.1
        │   │   │       │   │   ├── itoa feature "default" (*)
        │   │   │       │   │   ├── bytes feature "default" (*)
        │   │   │       │   │   └── fnv feature "default"
        │   │   │       │   │       ├── fnv v1.0.7
        │   │   │       │   │       └── fnv feature "std"
        │   │   │       │   │           └── fnv v1.0.7
        │   │   │       │   └── http feature "std"
        │   │   │       │       └── http v1.3.1 (*)
        │   │   │       ├── http-body feature "default"
        │   │   │       │   └── http-body v1.0.1
        │   │   │       │       ├── bytes feature "default" (*)
        │   │   │       │       └── http feature "default" (*)
        │   │   │       ├── http-body-util feature "default"
        │   │   │       │   └── http-body-util v0.1.3
        │   │   │       │       ├── futures-core v0.3.31
        │   │   │       │       ├── bytes feature "default" (*)
        │   │   │       │       ├── pin-project-lite feature "default" (*)
        │   │   │       │       ├── http feature "default" (*)
        │   │   │       │       └── http-body feature "default" (*)
        │   │   │       ├── mime feature "default"
        │   │   │       │   └── mime v0.3.17
        │   │   │       ├── rustversion feature "default"
        │   │   │       │   └── rustversion v1.0.22 (proc-macro)
        │   │   │       ├── sync_wrapper feature "default"
        │   │   │       │   └── sync_wrapper v1.0.2
        │   │   │       │       └── futures-core v0.3.31
        │   │   │       ├── tower-layer feature "default"
        │   │   │       │   └── tower-layer v0.3.3
        │   │   │       └── tower-service feature "default"
        │   │   │           └── tower-service v0.3.3
        │   │   ├── bytes feature "default" (*)
        │   │   ├── futures-util feature "alloc" (*)
        │   │   ├── pin-project-lite feature "default" (*)
        │   │   ├── http feature "default" (*)
        │   │   ├── http-body feature "default" (*)
        │   │   ├── http-body-util feature "default" (*)
        │   │   ├── mime feature "default" (*)
        │   │   ├── rustversion feature "default" (*)
        │   │   ├── sync_wrapper feature "default" (*)
        │   │   ├── tower-layer feature "default" (*)
        │   │   ├── tower-service feature "default" (*)
        │   │   ├── axum-macros feature "default"
        │   │   │   └── axum-macros v0.4.2 (proc-macro)
        │   │   │       ├── proc-macro2 feature "default" (*)
        │   │   │       ├── quote feature "default" (*)
        │   │   │       ├── syn feature "default" (*)
        │   │   │       ├── syn feature "extra-traits"
        │   │   │       │   └── syn v2.0.106 (*)
        │   │   │       ├── syn feature "full" (*)
        │   │   │       └── syn feature "parsing" (*)
        │   │   ├── hyper feature "default"
        │   │   │   └── hyper v1.7.0
... (truncated)
```

</details>

### workspace-hack

<details><summary>Reverse tree (-i workspace-hack -e features)</summary>

```text
```

</details>

<details><summary>Forward tree (-p workspace-hack -e features)</summary>

```text
workspace-hack v0.1.0 (/Users/mymac/Desktop/RustyOnions/workspace-hack)
├── axum feature "http1"
│   ├── axum v0.7.9
│   │   ├── async-trait feature "default"
│   │   │   └── async-trait v0.1.89 (proc-macro)
│   │   │       ├── proc-macro2 feature "default"
│   │   │       │   ├── proc-macro2 v1.0.101
│   │   │       │   │   └── unicode-ident feature "default"
│   │   │       │   │       └── unicode-ident v1.0.19
│   │   │       │   └── proc-macro2 feature "proc-macro"
│   │   │       │       └── proc-macro2 v1.0.101 (*)
│   │   │       ├── quote feature "default"
│   │   │       │   ├── quote v1.0.40
│   │   │       │   │   └── proc-macro2 v1.0.101 (*)
│   │   │       │   └── quote feature "proc-macro"
│   │   │       │       ├── quote v1.0.40 (*)
│   │   │       │       └── proc-macro2 feature "proc-macro" (*)
│   │   │       ├── syn feature "clone-impls"
│   │   │       │   └── syn v2.0.106
│   │   │       │       ├── proc-macro2 v1.0.101 (*)
│   │   │       │       ├── quote v1.0.40 (*)
│   │   │       │       └── unicode-ident feature "default" (*)
│   │   │       ├── syn feature "full"
│   │   │       │   └── syn v2.0.106 (*)
│   │   │       ├── syn feature "parsing"
│   │   │       │   └── syn v2.0.106 (*)
│   │   │       ├── syn feature "printing"
│   │   │       │   └── syn v2.0.106 (*)
│   │   │       ├── syn feature "proc-macro"
│   │   │       │   ├── syn v2.0.106 (*)
│   │   │       │   ├── proc-macro2 feature "proc-macro" (*)
│   │   │       │   └── quote feature "proc-macro" (*)
│   │   │       └── syn feature "visit-mut"
│   │   │           └── syn v2.0.106 (*)
│   │   ├── axum-core feature "default"
│   │   │   └── axum-core v0.4.5
│   │   │       ├── async-trait feature "default" (*)
│   │   │       ├── bytes feature "default"
│   │   │       │   ├── bytes v1.10.1
│   │   │       │   └── bytes feature "std"
│   │   │       │       └── bytes v1.10.1
│   │   │       ├── futures-util feature "alloc"
│   │   │       │   ├── futures-util v0.3.31
│   │   │       │   │   ├── futures-core v0.3.31
│   │   │       │   │   ├── futures-macro v0.3.31 (proc-macro)
│   │   │       │   │   │   ├── proc-macro2 feature "default" (*)
│   │   │       │   │   │   ├── quote feature "default" (*)
│   │   │       │   │   │   ├── syn feature "default"
│   │   │       │   │   │   │   ├── syn v2.0.106 (*)
│   │   │       │   │   │   │   ├── syn feature "clone-impls" (*)
│   │   │       │   │   │   │   ├── syn feature "derive"
│   │   │       │   │   │   │   │   └── syn v2.0.106 (*)
│   │   │       │   │   │   │   ├── syn feature "parsing" (*)
│   │   │       │   │   │   │   ├── syn feature "printing" (*)
│   │   │       │   │   │   │   └── syn feature "proc-macro" (*)
│   │   │       │   │   │   └── syn feature "full" (*)
│   │   │       │   │   ├── futures-sink v0.3.31
│   │   │       │   │   ├── futures-task v0.3.31
│   │   │       │   │   ├── futures-channel feature "std"
│   │   │       │   │   │   ├── futures-channel v0.3.31
│   │   │       │   │   │   │   ├── futures-core v0.3.31
│   │   │       │   │   │   │   └── futures-sink v0.3.31
│   │   │       │   │   │   ├── futures-channel feature "alloc"
│   │   │       │   │   │   │   ├── futures-channel v0.3.31 (*)
│   │   │       │   │   │   │   └── futures-core feature "alloc"
│   │   │       │   │   │   │       └── futures-core v0.3.31
│   │   │       │   │   │   └── futures-core feature "std"
│   │   │       │   │   │       ├── futures-core v0.3.31
│   │   │       │   │   │       └── futures-core feature "alloc" (*)
│   │   │       │   │   ├── futures-io feature "std"
│   │   │       │   │   │   └── futures-io v0.3.31
│   │   │       │   │   ├── memchr feature "default"
│   │   │       │   │   │   ├── memchr v2.7.5
│   │   │       │   │   │   └── memchr feature "std"
│   │   │       │   │   │       ├── memchr v2.7.5
│   │   │       │   │   │       └── memchr feature "alloc"
│   │   │       │   │   │           └── memchr v2.7.5
│   │   │       │   │   ├── pin-project-lite feature "default"
│   │   │       │   │   │   └── pin-project-lite v0.2.16
│   │   │       │   │   ├── pin-utils feature "default"
│   │   │       │   │   │   └── pin-utils v0.1.0
│   │   │       │   │   └── slab feature "default"
│   │   │       │   │       ├── slab v0.4.11
│   │   │       │   │       └── slab feature "std"
│   │   │       │   │           └── slab v0.4.11
│   │   │       │   ├── futures-core feature "alloc" (*)
│   │   │       │   └── futures-task feature "alloc"
│   │   │       │       └── futures-task v0.3.31
│   │   │       ├── pin-project-lite feature "default" (*)
│   │   │       ├── http feature "default"
│   │   │       │   ├── http v1.3.1
│   │   │       │   │   ├── bytes feature "default" (*)
│   │   │       │   │   ├── fnv feature "default"
│   │   │       │   │   │   ├── fnv v1.0.7
│   │   │       │   │   │   └── fnv feature "std"
│   │   │       │   │   │       └── fnv v1.0.7
│   │   │       │   │   └── itoa feature "default"
│   │   │       │   │       └── itoa v1.0.15
│   │   │       │   └── http feature "std"
│   │   │       │       └── http v1.3.1 (*)
│   │   │       ├── http-body feature "default"
│   │   │       │   └── http-body v1.0.1
│   │   │       │       ├── bytes feature "default" (*)
│   │   │       │       └── http feature "default" (*)
│   │   │       ├── http-body-util feature "default"
│   │   │       │   └── http-body-util v0.1.3
│   │   │       │       ├── futures-core v0.3.31
│   │   │       │       ├── bytes feature "default" (*)
│   │   │       │       ├── pin-project-lite feature "default" (*)
│   │   │       │       ├── http feature "default" (*)
│   │   │       │       └── http-body feature "default" (*)
│   │   │       ├── mime feature "default"
│   │   │       │   └── mime v0.3.17
│   │   │       ├── rustversion feature "default"
│   │   │       │   └── rustversion v1.0.22 (proc-macro)
│   │   │       ├── sync_wrapper feature "default"
│   │   │       │   └── sync_wrapper v1.0.2
│   │   │       │       └── futures-core v0.3.31
│   │   │       ├── tower-layer feature "default"
│   │   │       │   └── tower-layer v0.3.3
│   │   │       └── tower-service feature "default"
│   │   │           └── tower-service v0.3.3
│   │   ├── bytes feature "default" (*)
│   │   ├── futures-util feature "alloc" (*)
│   │   ├── memchr feature "default" (*)
│   │   ├── pin-project-lite feature "default" (*)
│   │   ├── http feature "default" (*)
│   │   ├── itoa feature "default" (*)
│   │   ├── http-body feature "default" (*)
│   │   ├── http-body-util feature "default" (*)
│   │   ├── mime feature "default" (*)
│   │   ├── rustversion feature "default" (*)
│   │   ├── sync_wrapper feature "default" (*)
│   │   ├── tower-layer feature "default" (*)
│   │   ├── tower-service feature "default" (*)
│   │   ├── axum-macros feature "default"
│   │   │   └── axum-macros v0.4.2 (proc-macro)
│   │   │       ├── proc-macro2 feature "default" (*)
│   │   │       ├── quote feature "default" (*)
│   │   │       ├── syn feature "default" (*)
│   │   │       ├── syn feature "extra-traits"
│   │   │       │   └── syn v2.0.106 (*)
│   │   │       ├── syn feature "full" (*)
│   │   │       └── syn feature "parsing" (*)
│   │   ├── hyper feature "default"
│   │   │   └── hyper v1.7.0
│   │   │       ├── bytes feature "default" (*)
│   │   │       ├── futures-channel feature "default"
│   │   │       │   ├── futures-channel v0.3.31 (*)
│   │   │       │   └── futures-channel feature "std" (*)
│   │   │       ├── futures-core feature "default"
│   │   │       │   ├── futures-core v0.3.31
│   │   │       │   └── futures-core feature "std" (*)
│   │   │       ├── pin-project-lite feature "default" (*)
│   │   │       ├── pin-utils feature "default" (*)
│   │   │       ├── http feature "default" (*)
│   │   │       ├── itoa feature "default" (*)
│   │   │       ├── http-body feature "default" (*)
│   │   │       ├── atomic-waker feature "default"
│   │   │       │   └── atomic-waker v1.1.2
│   │   │       ├── h2 feature "default"
│   │   │       │   └── h2 v0.4.12
│   │   │       │       ├── futures-core v0.3.31
│   │   │       │       ├── futures-sink v0.3.31
│   │   │       │       ├── bytes feature "default" (*)
│   │   │       │       ├── slab feature "default" (*)
│   │   │       │       ├── http feature "default" (*)
│   │   │       │       ├── fnv feature "default" (*)
│   │   │       │       ├── atomic-waker feature "default" (*)
│   │   │       │       ├── indexmap feature "default"
│   │   │       │       │   ├── indexmap v2.11.1
│   │   │       │       │   │   ├── equivalent v1.0.2
│   │   │       │       │   │   └── hashbrown v0.15.5
│   │   │       │       │   │       ├── equivalent v1.0.2
│   │   │       │       │   │       ├── foldhash v0.1.5
│   │   │       │       │   │       └── allocator-api2 feature "alloc"
│   │   │       │       │   │           └── allocator-api2 v0.2.21
│   │   │       │       │   └── indexmap feature "std"
│   │   │       │       │       └── indexmap v2.11.1 (*)
│   │   │       │       ├── indexmap feature "std" (*)
│   │   │       │       ├── tokio feature "default"
│   │   │       │       │   └── tokio v1.47.1
│   │   │       │       │       ├── mio v1.0.4
│   │   │       │       │       │   └── libc feature "default"
│   │   │       │       │       │       ├── libc v0.2.175
│   │   │       │       │       │       └── libc feature "std"
│   │   │       │       │       │           └── libc v0.2.175
│   │   │       │       │       ├── bytes feature "default" (*)
│   │   │       │       │       ├── pin-project-lite feature "default" (*)
│   │   │       │       │       ├── libc feature "default" (*)
│   │   │       │       │       ├── parking_lot feature "default"
│   │   │       │       │       │   └── parking_lot v0.12.4
│   │   │       │       │       │       ├── lock_api feature "default"
│   │   │       │       │       │       │   ├── lock_api v0.4.13
│   │   │       │       │       │       │   │   └── scopeguard v1.2.0
│   │   │       │       │       │       │   │   [build-dependencies]
│   │   │       │       │       │       │   │   └── autocfg feature "default"
│   │   │       │       │       │       │   │       └── autocfg v1.5.0
│   │   │       │       │       │       │   └── lock_api feature "atomic_usize"
│   │   │       │       │       │       │       └── lock_api v0.4.13 (*)
│   │   │       │       │       │       └── parking_lot_core feature "default"
│   │   │       │       │       │           └── parking_lot_core v0.9.11
│   │   │       │       │       │               ├── libc feature "default" (*)
│   │   │       │       │       │               ├── cfg-if feature "default"
│   │   │       │       │       │               │   └── cfg-if v1.0.3
│   │   │       │       │       │               └── smallvec feature "default"
│   │   │       │       │       │                   └── smallvec v1.15.1
│   │   │       │       │       ├── signal-hook-registry feature "default"
│   │   │       │       │       │   └── signal-hook-registry v1.4.6
│   │   │       │       │       │       └── libc feature "default" (*)
│   │   │       │       │       ├── socket2 feature "all"
│   │   │       │       │       │   └── socket2 v0.6.0
│   │   │       │       │       │       └── libc feature "default" (*)
│   │   │       │       │       ├── socket2 feature "default"
│   │   │       │       │       │   └── socket2 v0.6.0 (*)
│   │   │       │       │       └── tokio-macros feature "default"
│   │   │       │       │           └── tokio-macros v2.5.0 (proc-macro)
│   │   │       │       │               ├── proc-macro2 feature "default" (*)
│   │   │       │       │               ├── quote feature "default" (*)
│   │   │       │       │               ├── syn feature "default" (*)
│   │   │       │       │               └── syn feature "full" (*)
│   │   │       │       ├── tokio feature "io-util"
│   │   │       │       │   ├── tokio v1.47.1 (*)
│   │   │       │       │   └── tokio feature "bytes"
│   │   │       │       │       └── tokio v1.47.1 (*)
│   │   │       │       ├── tokio-util feature "codec"
│   │   │       │       │   └── tokio-util v0.7.16
│   │   │       │       │       ├── bytes feature "default" (*)
│   │   │       │       │       ├── futures-core feature "default" (*)
│   │   │       │       │       ├── futures-sink feature "default"
│   │   │       │       │       │   ├── futures-sink v0.3.31
│   │   │       │       │       │   └── futures-sink feature "std"
│   │   │       │       │       │       ├── futures-sink v0.3.31
│   │   │       │       │       │       └── futures-sink feature "alloc"
│   │   │       │       │       │           └── futures-sink v0.3.31
│   │   │       │       │       ├── pin-project-lite feature "default" (*)
│   │   │       │       │       ├── tokio feature "default" (*)
│   │   │       │       │       └── tokio feature "sync"
│   │   │       │       │           └── tokio v1.47.1 (*)
│   │   │       │       ├── tokio-util feature "default"
│   │   │       │       │   └── tokio-util v0.7.16 (*)
│   │   │       │       ├── tokio-util feature "io"
│   │   │       │       │   └── tokio-util v0.7.16 (*)
│   │   │       │       └── tracing feature "std"
│   │   │       │           ├── tracing v0.1.41
│   │   │       │           │   ├── tracing-core v0.1.34
│   │   │       │           │   │   └── once_cell feature "default"
│   │   │       │           │   │       ├── once_cell v1.21.3
│   │   │       │           │   │       └── once_cell feature "std"
│   │   │       │           │   │           ├── once_cell v1.21.3
│   │   │       │           │   │           └── once_cell feature "alloc"
│   │   │       │           │   │               ├── once_cell v1.21.3
│   │   │       │           │   │               └── once_cell feature "race"
│   │   │       │           │   │                   └── once_cell v1.21.3
│   │   │       │           │   ├── pin-project-lite feature "default" (*)
│   │   │       │           │   └── tracing-attributes feature "default"
│   │   │       │           │       └── tracing-attributes v0.1.30 (proc-macro)
│   │   │       │           │           ├── proc-macro2 feature "default" (*)
│   │   │       │           │           ├── quote feature "default" (*)
│   │   │       │           │           ├── syn feature "clone-impls" (*)
│   │   │       │           │           ├── syn feature "extra-traits" (*)
│   │   │       │           │           ├── syn feature "full" (*)
│   │   │       │           │           ├── syn feature "parsing" (*)
│   │   │       │           │           ├── syn feature "printing" (*)
│   │   │       │           │           ├── syn feature "proc-macro" (*)
│   │   │       │           │           └── syn feature "visit-mut" (*)
│   │   │       │           └── tracing-core feature "std"
│   │   │       │               ├── tracing-core v0.1.34 (*)
│   │   │       │               └── tracing-core feature "once_cell"
│   │   │       │                   └── tracing-core v0.1.34 (*)
│   │   │       ├── tokio feature "default" (*)
│   │   │       ├── tokio feature "sync" (*)
│   │   │       ├── smallvec feature "const_generics"
│   │   │       │   └── smallvec v1.15.1
│   │   │       ├── smallvec feature "const_new"
│   │   │       │   ├── smallvec v1.15.1
│   │   │       │   └── smallvec feature "const_generics" (*)
│   │   │       ├── smallvec feature "default" (*)
│   │   │       ├── httparse feature "default"
│   │   │       │   ├── httparse v1.10.1
│   │   │       │   └── httparse feature "std"
│   │   │       │       └── httparse v1.10.1
│   │   │       ├── httpdate feature "default"
│   │   │       │   └── httpdate v1.0.3
│   │   │       └── want feature "default"
│   │   │           └── want v0.3.1
│   │   │               └── try-lock feature "default"
│   │   │                   └── try-lock v0.2.5
│   │   ├── tokio feature "default" (*)
│   │   ├── tokio feature "time"
│   │   │   └── tokio v1.47.1 (*)
│   │   ├── hyper-util feature "default"
│   │   │   └── hyper-util v0.1.16
│   │   │       ├── futures-util v0.3.31 (*)
│   │   │       ├── tokio v1.47.1 (*)
│   │   │       ├── bytes feature "default" (*)
│   │   │       ├── futures-channel feature "default" (*)
│   │   │       ├── futures-core feature "default" (*)
│   │   │       ├── pin-project-lite feature "default" (*)
│   │   │       ├── http feature "default" (*)
... (truncated)
```

</details>

### accounting

<details><summary>Reverse tree (-i accounting -e features)</summary>

```text
```

</details>

<details><summary>Forward tree (-p accounting -e features)</summary>

```text
accounting v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/accounting)
├── anyhow feature "default"
│   ├── anyhow v1.0.99
│   └── anyhow feature "std"
│       └── anyhow v1.0.99
├── parking_lot feature "default"
│   └── parking_lot v0.12.4
│       ├── lock_api feature "default"
│       │   ├── lock_api v0.4.13
│       │   │   └── scopeguard v1.2.0
│       │   │   [build-dependencies]
│       │   │   └── autocfg feature "default"
│       │   │       └── autocfg v1.5.0
│       │   └── lock_api feature "atomic_usize"
│       │       └── lock_api v0.4.13 (*)
│       └── parking_lot_core feature "default"
│           └── parking_lot_core v0.9.11
│               ├── cfg-if feature "default"
│               │   └── cfg-if v1.0.3
│               ├── libc feature "default"
│               │   ├── libc v0.2.175
│               │   └── libc feature "std"
│               │       └── libc v0.2.175
│               └── smallvec feature "default"
│                   └── smallvec v1.15.1
└── workspace-hack feature "default"
    └── workspace-hack v0.1.0 (/Users/mymac/Desktop/RustyOnions/workspace-hack)
        ├── smallvec feature "const_new"
        │   ├── smallvec v1.15.1
        │   └── smallvec feature "const_generics"
        │       └── smallvec v1.15.1
        ├── axum feature "http1"
        │   ├── axum v0.7.9
        │   │   ├── async-trait feature "default"
        │   │   │   └── async-trait v0.1.89 (proc-macro)
        │   │   │       ├── proc-macro2 feature "default"
        │   │   │       │   ├── proc-macro2 v1.0.101
        │   │   │       │   │   └── unicode-ident feature "default"
        │   │   │       │   │       └── unicode-ident v1.0.19
        │   │   │       │   └── proc-macro2 feature "proc-macro"
        │   │   │       │       └── proc-macro2 v1.0.101 (*)
        │   │   │       ├── quote feature "default"
        │   │   │       │   ├── quote v1.0.40
        │   │   │       │   │   └── proc-macro2 v1.0.101 (*)
        │   │   │       │   └── quote feature "proc-macro"
        │   │   │       │       ├── quote v1.0.40 (*)
        │   │   │       │       └── proc-macro2 feature "proc-macro" (*)
        │   │   │       ├── syn feature "clone-impls"
        │   │   │       │   └── syn v2.0.106
        │   │   │       │       ├── proc-macro2 v1.0.101 (*)
        │   │   │       │       ├── quote v1.0.40 (*)
        │   │   │       │       └── unicode-ident feature "default" (*)
        │   │   │       ├── syn feature "full"
        │   │   │       │   └── syn v2.0.106 (*)
        │   │   │       ├── syn feature "parsing"
        │   │   │       │   └── syn v2.0.106 (*)
        │   │   │       ├── syn feature "printing"
        │   │   │       │   └── syn v2.0.106 (*)
        │   │   │       ├── syn feature "proc-macro"
        │   │   │       │   ├── syn v2.0.106 (*)
        │   │   │       │   ├── proc-macro2 feature "proc-macro" (*)
        │   │   │       │   └── quote feature "proc-macro" (*)
        │   │   │       └── syn feature "visit-mut"
        │   │   │           └── syn v2.0.106 (*)
        │   │   ├── axum-core feature "default"
        │   │   │   └── axum-core v0.4.5
        │   │   │       ├── async-trait feature "default" (*)
        │   │   │       ├── bytes feature "default"
        │   │   │       │   ├── bytes v1.10.1
        │   │   │       │   └── bytes feature "std"
        │   │   │       │       └── bytes v1.10.1
        │   │   │       ├── futures-util feature "alloc"
        │   │   │       │   ├── futures-util v0.3.31
        │   │   │       │   │   ├── futures-core v0.3.31
        │   │   │       │   │   ├── futures-macro v0.3.31 (proc-macro)
        │   │   │       │   │   │   ├── proc-macro2 feature "default" (*)
        │   │   │       │   │   │   ├── quote feature "default" (*)
        │   │   │       │   │   │   ├── syn feature "default"
        │   │   │       │   │   │   │   ├── syn v2.0.106 (*)
        │   │   │       │   │   │   │   ├── syn feature "clone-impls" (*)
        │   │   │       │   │   │   │   ├── syn feature "derive"
        │   │   │       │   │   │   │   │   └── syn v2.0.106 (*)
        │   │   │       │   │   │   │   ├── syn feature "parsing" (*)
        │   │   │       │   │   │   │   ├── syn feature "printing" (*)
        │   │   │       │   │   │   │   └── syn feature "proc-macro" (*)
        │   │   │       │   │   │   └── syn feature "full" (*)
        │   │   │       │   │   ├── futures-sink v0.3.31
        │   │   │       │   │   ├── futures-task v0.3.31
        │   │   │       │   │   ├── futures-channel feature "std"
        │   │   │       │   │   │   ├── futures-channel v0.3.31
        │   │   │       │   │   │   │   ├── futures-core v0.3.31
        │   │   │       │   │   │   │   └── futures-sink v0.3.31
        │   │   │       │   │   │   ├── futures-channel feature "alloc"
        │   │   │       │   │   │   │   ├── futures-channel v0.3.31 (*)
        │   │   │       │   │   │   │   └── futures-core feature "alloc"
        │   │   │       │   │   │   │       └── futures-core v0.3.31
        │   │   │       │   │   │   └── futures-core feature "std"
        │   │   │       │   │   │       ├── futures-core v0.3.31
        │   │   │       │   │   │       └── futures-core feature "alloc" (*)
        │   │   │       │   │   ├── futures-io feature "std"
        │   │   │       │   │   │   └── futures-io v0.3.31
        │   │   │       │   │   ├── memchr feature "default"
        │   │   │       │   │   │   ├── memchr v2.7.5
        │   │   │       │   │   │   └── memchr feature "std"
        │   │   │       │   │   │       ├── memchr v2.7.5
        │   │   │       │   │   │       └── memchr feature "alloc"
        │   │   │       │   │   │           └── memchr v2.7.5
        │   │   │       │   │   ├── pin-project-lite feature "default"
        │   │   │       │   │   │   └── pin-project-lite v0.2.16
        │   │   │       │   │   ├── pin-utils feature "default"
        │   │   │       │   │   │   └── pin-utils v0.1.0
        │   │   │       │   │   └── slab feature "default"
        │   │   │       │   │       ├── slab v0.4.11
        │   │   │       │   │       └── slab feature "std"
        │   │   │       │   │           └── slab v0.4.11
        │   │   │       │   ├── futures-core feature "alloc" (*)
        │   │   │       │   └── futures-task feature "alloc"
        │   │   │       │       └── futures-task v0.3.31
        │   │   │       ├── pin-project-lite feature "default" (*)
        │   │   │       ├── http feature "default"
        │   │   │       │   ├── http v1.3.1
        │   │   │       │   │   ├── bytes feature "default" (*)
        │   │   │       │   │   ├── fnv feature "default"
        │   │   │       │   │   │   ├── fnv v1.0.7
        │   │   │       │   │   │   └── fnv feature "std"
        │   │   │       │   │   │       └── fnv v1.0.7
        │   │   │       │   │   └── itoa feature "default"
        │   │   │       │   │       └── itoa v1.0.15
        │   │   │       │   └── http feature "std"
        │   │   │       │       └── http v1.3.1 (*)
        │   │   │       ├── http-body feature "default"
        │   │   │       │   └── http-body v1.0.1
        │   │   │       │       ├── bytes feature "default" (*)
        │   │   │       │       └── http feature "default" (*)
        │   │   │       ├── http-body-util feature "default"
        │   │   │       │   └── http-body-util v0.1.3
        │   │   │       │       ├── futures-core v0.3.31
        │   │   │       │       ├── bytes feature "default" (*)
        │   │   │       │       ├── pin-project-lite feature "default" (*)
        │   │   │       │       ├── http feature "default" (*)
        │   │   │       │       └── http-body feature "default" (*)
        │   │   │       ├── mime feature "default"
        │   │   │       │   └── mime v0.3.17
        │   │   │       ├── rustversion feature "default"
        │   │   │       │   └── rustversion v1.0.22 (proc-macro)
        │   │   │       ├── sync_wrapper feature "default"
        │   │   │       │   └── sync_wrapper v1.0.2
        │   │   │       │       └── futures-core v0.3.31
        │   │   │       ├── tower-layer feature "default"
        │   │   │       │   └── tower-layer v0.3.3
        │   │   │       └── tower-service feature "default"
        │   │   │           └── tower-service v0.3.3
        │   │   ├── bytes feature "default" (*)
        │   │   ├── futures-util feature "alloc" (*)
        │   │   ├── memchr feature "default" (*)
        │   │   ├── pin-project-lite feature "default" (*)
        │   │   ├── http feature "default" (*)
        │   │   ├── itoa feature "default" (*)
        │   │   ├── http-body feature "default" (*)
        │   │   ├── http-body-util feature "default" (*)
        │   │   ├── mime feature "default" (*)
        │   │   ├── rustversion feature "default" (*)
        │   │   ├── sync_wrapper feature "default" (*)
        │   │   ├── tower-layer feature "default" (*)
        │   │   ├── tower-service feature "default" (*)
        │   │   ├── axum-macros feature "default"
        │   │   │   └── axum-macros v0.4.2 (proc-macro)
        │   │   │       ├── proc-macro2 feature "default" (*)
        │   │   │       ├── quote feature "default" (*)
        │   │   │       ├── syn feature "default" (*)
        │   │   │       ├── syn feature "extra-traits"
        │   │   │       │   └── syn v2.0.106 (*)
        │   │   │       ├── syn feature "full" (*)
        │   │   │       └── syn feature "parsing" (*)
        │   │   ├── hyper feature "default"
        │   │   │   └── hyper v1.7.0
        │   │   │       ├── smallvec feature "const_generics" (*)
        │   │   │       ├── smallvec feature "const_new" (*)
        │   │   │       ├── smallvec feature "default" (*)
        │   │   │       ├── bytes feature "default" (*)
        │   │   │       ├── futures-channel feature "default"
        │   │   │       │   ├── futures-channel v0.3.31 (*)
        │   │   │       │   └── futures-channel feature "std" (*)
        │   │   │       ├── futures-core feature "default"
        │   │   │       │   ├── futures-core v0.3.31
        │   │   │       │   └── futures-core feature "std" (*)
        │   │   │       ├── pin-project-lite feature "default" (*)
        │   │   │       ├── pin-utils feature "default" (*)
        │   │   │       ├── http feature "default" (*)
        │   │   │       ├── itoa feature "default" (*)
        │   │   │       ├── http-body feature "default" (*)
        │   │   │       ├── atomic-waker feature "default"
        │   │   │       │   └── atomic-waker v1.1.2
        │   │   │       ├── h2 feature "default"
        │   │   │       │   └── h2 v0.4.12
        │   │   │       │       ├── futures-core v0.3.31
        │   │   │       │       ├── futures-sink v0.3.31
        │   │   │       │       ├── bytes feature "default" (*)
        │   │   │       │       ├── slab feature "default" (*)
        │   │   │       │       ├── http feature "default" (*)
        │   │   │       │       ├── fnv feature "default" (*)
        │   │   │       │       ├── atomic-waker feature "default" (*)
        │   │   │       │       ├── indexmap feature "default"
        │   │   │       │       │   ├── indexmap v2.11.1
        │   │   │       │       │   │   ├── equivalent v1.0.2
        │   │   │       │       │   │   └── hashbrown v0.15.5
        │   │   │       │       │   │       ├── equivalent v1.0.2
        │   │   │       │       │   │       ├── foldhash v0.1.5
        │   │   │       │       │   │       └── allocator-api2 feature "alloc"
        │   │   │       │       │   │           └── allocator-api2 v0.2.21
        │   │   │       │       │   └── indexmap feature "std"
        │   │   │       │       │       └── indexmap v2.11.1 (*)
        │   │   │       │       ├── indexmap feature "std" (*)
        │   │   │       │       ├── tokio feature "default"
        │   │   │       │       │   └── tokio v1.47.1
        │   │   │       │       │       ├── mio v1.0.4
        │   │   │       │       │       │   └── libc feature "default" (*)
        │   │   │       │       │       ├── parking_lot feature "default" (*)
        │   │   │       │       │       ├── libc feature "default" (*)
        │   │   │       │       │       ├── bytes feature "default" (*)
        │   │   │       │       │       ├── pin-project-lite feature "default" (*)
        │   │   │       │       │       ├── signal-hook-registry feature "default"
        │   │   │       │       │       │   └── signal-hook-registry v1.4.6
        │   │   │       │       │       │       └── libc feature "default" (*)
        │   │   │       │       │       ├── socket2 feature "all"
        │   │   │       │       │       │   └── socket2 v0.6.0
        │   │   │       │       │       │       └── libc feature "default" (*)
        │   │   │       │       │       ├── socket2 feature "default"
        │   │   │       │       │       │   └── socket2 v0.6.0 (*)
        │   │   │       │       │       └── tokio-macros feature "default"
        │   │   │       │       │           └── tokio-macros v2.5.0 (proc-macro)
        │   │   │       │       │               ├── proc-macro2 feature "default" (*)
        │   │   │       │       │               ├── quote feature "default" (*)
        │   │   │       │       │               ├── syn feature "default" (*)
        │   │   │       │       │               └── syn feature "full" (*)
        │   │   │       │       ├── tokio feature "io-util"
        │   │   │       │       │   ├── tokio v1.47.1 (*)
        │   │   │       │       │   └── tokio feature "bytes"
        │   │   │       │       │       └── tokio v1.47.1 (*)
        │   │   │       │       ├── tokio-util feature "codec"
        │   │   │       │       │   └── tokio-util v0.7.16
        │   │   │       │       │       ├── bytes feature "default" (*)
        │   │   │       │       │       ├── futures-core feature "default" (*)
        │   │   │       │       │       ├── futures-sink feature "default"
        │   │   │       │       │       │   ├── futures-sink v0.3.31
        │   │   │       │       │       │   └── futures-sink feature "std"
        │   │   │       │       │       │       ├── futures-sink v0.3.31
        │   │   │       │       │       │       └── futures-sink feature "alloc"
        │   │   │       │       │       │           └── futures-sink v0.3.31
        │   │   │       │       │       ├── pin-project-lite feature "default" (*)
        │   │   │       │       │       ├── tokio feature "default" (*)
        │   │   │       │       │       └── tokio feature "sync"
        │   │   │       │       │           └── tokio v1.47.1 (*)
        │   │   │       │       ├── tokio-util feature "default"
        │   │   │       │       │   └── tokio-util v0.7.16 (*)
        │   │   │       │       ├── tokio-util feature "io"
        │   │   │       │       │   └── tokio-util v0.7.16 (*)
        │   │   │       │       └── tracing feature "std"
        │   │   │       │           ├── tracing v0.1.41
        │   │   │       │           │   ├── tracing-core v0.1.34
        │   │   │       │           │   │   └── once_cell feature "default"
        │   │   │       │           │   │       ├── once_cell v1.21.3
        │   │   │       │           │   │       └── once_cell feature "std"
        │   │   │       │           │   │           ├── once_cell v1.21.3
        │   │   │       │           │   │           └── once_cell feature "alloc"
        │   │   │       │           │   │               ├── once_cell v1.21.3
        │   │   │       │           │   │               └── once_cell feature "race"
        │   │   │       │           │   │                   └── once_cell v1.21.3
        │   │   │       │           │   ├── pin-project-lite feature "default" (*)
        │   │   │       │           │   └── tracing-attributes feature "default"
        │   │   │       │           │       └── tracing-attributes v0.1.30 (proc-macro)
        │   │   │       │           │           ├── proc-macro2 feature "default" (*)
        │   │   │       │           │           ├── quote feature "default" (*)
        │   │   │       │           │           ├── syn feature "clone-impls" (*)
        │   │   │       │           │           ├── syn feature "extra-traits" (*)
        │   │   │       │           │           ├── syn feature "full" (*)
        │   │   │       │           │           ├── syn feature "parsing" (*)
        │   │   │       │           │           ├── syn feature "printing" (*)
        │   │   │       │           │           ├── syn feature "proc-macro" (*)
        │   │   │       │           │           └── syn feature "visit-mut" (*)
        │   │   │       │           └── tracing-core feature "std"
        │   │   │       │               ├── tracing-core v0.1.34 (*)
        │   │   │       │               └── tracing-core feature "once_cell"
        │   │   │       │                   └── tracing-core v0.1.34 (*)
        │   │   │       ├── tokio feature "default" (*)
        │   │   │       ├── tokio feature "sync" (*)
        │   │   │       ├── httparse feature "default"
        │   │   │       │   ├── httparse v1.10.1
        │   │   │       │   └── httparse feature "std"
        │   │   │       │       └── httparse v1.10.1
        │   │   │       ├── httpdate feature "default"
        │   │   │       │   └── httpdate v1.0.3
        │   │   │       └── want feature "default"
        │   │   │           └── want v0.3.1
        │   │   │               └── try-lock feature "default"
        │   │   │                   └── try-lock v0.2.5
        │   │   ├── tokio feature "default" (*)
        │   │   ├── tokio feature "time"
        │   │   │   └── tokio v1.47.1 (*)
        │   │   ├── hyper-util feature "default"
... (truncated)
```

</details>

### overlay

<details><summary>Reverse tree (-i overlay -e features)</summary>

```text
```

</details>

<details><summary>Forward tree (-p overlay -e features)</summary>

```text
overlay v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/overlay)
├── anyhow feature "default"
│   ├── anyhow v1.0.99
│   └── anyhow feature "std"
│       └── anyhow v1.0.99
├── blake3 feature "default"
│   ├── blake3 v1.8.2
│   │   ├── arrayvec v0.7.6
│   │   ├── constant_time_eq v0.3.1
│   │   ├── arrayref feature "default"
│   │   │   └── arrayref v0.3.9
│   │   └── cfg-if feature "default"
│   │       └── cfg-if v1.0.3
│   │   [build-dependencies]
│   │   └── cc feature "default"
│   │       └── cc v1.2.37
│   │           ├── jobserver v0.1.34
│   │           │   └── libc feature "default"
│   │           │       ├── libc v0.2.175
│   │           │       └── libc feature "std"
│   │           │           └── libc v0.2.175
│   │           ├── libc v0.2.175
│   │           ├── find-msvc-tools feature "default"
│   │           │   └── find-msvc-tools v0.1.1
│   │           └── shlex feature "default"
│   │               ├── shlex v1.3.0
│   │               └── shlex feature "std"
│   │                   └── shlex v1.3.0
│   └── blake3 feature "std"
│       └── blake3 v1.8.2 (*)
├── hex feature "default"
│   ├── hex v0.4.3
│   └── hex feature "std"
│       ├── hex v0.4.3
│       └── hex feature "alloc"
│           └── hex v0.4.3
├── lru feature "default"
│   ├── lru v0.12.5
│   │   └── hashbrown feature "default"
│   │       ├── hashbrown v0.15.5
│   │       │   ├── equivalent v1.0.2
│   │       │   ├── foldhash v0.1.5
│   │       │   └── allocator-api2 feature "alloc"
│   │       │       └── allocator-api2 v0.2.21
│   │       ├── hashbrown feature "allocator-api2"
│   │       │   └── hashbrown v0.15.5 (*)
│   │       ├── hashbrown feature "default-hasher"
│   │       │   └── hashbrown v0.15.5 (*)
│   │       ├── hashbrown feature "equivalent"
│   │       │   └── hashbrown v0.15.5 (*)
│   │       ├── hashbrown feature "inline-more"
│   │       │   └── hashbrown v0.15.5 (*)
│   │       └── hashbrown feature "raw-entry"
│   │           └── hashbrown v0.15.5 (*)
│   └── lru feature "hashbrown"
│       └── lru v0.12.5 (*)
├── serde feature "default"
│   ├── serde v1.0.221
│   │   ├── serde_core feature "result"
│   │   │   └── serde_core v1.0.221
│   │   └── serde_derive feature "default"
│   │       └── serde_derive v1.0.221 (proc-macro)
│   │           ├── proc-macro2 feature "proc-macro"
│   │           │   └── proc-macro2 v1.0.101
│   │           │       └── unicode-ident feature "default"
│   │           │           └── unicode-ident v1.0.19
│   │           ├── quote feature "proc-macro"
│   │           │   ├── quote v1.0.40
│   │           │   │   └── proc-macro2 v1.0.101 (*)
│   │           │   └── proc-macro2 feature "proc-macro" (*)
│   │           ├── syn feature "clone-impls"
│   │           │   └── syn v2.0.106
│   │           │       ├── proc-macro2 v1.0.101 (*)
│   │           │       ├── quote v1.0.40 (*)
│   │           │       └── unicode-ident feature "default" (*)
│   │           ├── syn feature "derive"
│   │           │   └── syn v2.0.106 (*)
│   │           ├── syn feature "parsing"
│   │           │   └── syn v2.0.106 (*)
│   │           ├── syn feature "printing"
│   │           │   └── syn v2.0.106 (*)
│   │           └── syn feature "proc-macro"
│   │               ├── syn v2.0.106 (*)
│   │               ├── proc-macro2 feature "proc-macro" (*)
│   │               └── quote feature "proc-macro" (*)
│   └── serde feature "std"
│       ├── serde v1.0.221 (*)
│       └── serde_core feature "std"
│           └── serde_core v1.0.221
├── sled feature "default"
│   ├── sled v0.34.7
│   │   ├── libc feature "default" (*)
│   │   ├── crc32fast feature "default"
│   │   │   ├── crc32fast v1.5.0
│   │   │   │   └── cfg-if feature "default" (*)
│   │   │   └── crc32fast feature "std"
│   │   │       └── crc32fast v1.5.0 (*)
│   │   ├── crossbeam-epoch feature "default"
│   │   │   ├── crossbeam-epoch v0.9.18
│   │   │   │   └── crossbeam-utils v0.8.21
│   │   │   └── crossbeam-epoch feature "std"
│   │   │       ├── crossbeam-epoch v0.9.18 (*)
│   │   │       ├── crossbeam-epoch feature "alloc"
│   │   │       │   └── crossbeam-epoch v0.9.18 (*)
│   │   │       └── crossbeam-utils feature "std"
│   │   │           └── crossbeam-utils v0.8.21
│   │   ├── crossbeam-utils feature "default"
│   │   │   ├── crossbeam-utils v0.8.21
│   │   │   └── crossbeam-utils feature "std" (*)
│   │   ├── fs2 feature "default"
│   │   │   └── fs2 v0.4.3
│   │   │       └── libc feature "default" (*)
│   │   ├── fxhash feature "default"
│   │   │   └── fxhash v0.2.1
│   │   │       └── byteorder feature "default"
│   │   │           ├── byteorder v1.5.0
│   │   │           └── byteorder feature "std"
│   │   │               └── byteorder v1.5.0
│   │   ├── log feature "default"
│   │   │   └── log v0.4.28
│   │   └── parking_lot feature "default"
│   │       └── parking_lot v0.11.2
│   │           ├── instant feature "default"
│   │           │   └── instant v0.1.13
│   │           │       └── cfg-if feature "default" (*)
│   │           ├── lock_api feature "default"
│   │           │   ├── lock_api v0.4.13
│   │           │   │   └── scopeguard v1.2.0
│   │           │   │   [build-dependencies]
│   │           │   │   └── autocfg feature "default"
│   │           │   │       └── autocfg v1.5.0
│   │           │   └── lock_api feature "atomic_usize"
│   │           │       └── lock_api v0.4.13 (*)
│   │           └── parking_lot_core feature "default"
│   │               └── parking_lot_core v0.8.6
│   │                   ├── libc feature "default" (*)
│   │                   ├── cfg-if feature "default" (*)
│   │                   ├── instant feature "default" (*)
│   │                   └── smallvec feature "default"
│   │                       └── smallvec v1.15.1
│   └── sled feature "no_metrics"
│       └── sled v0.34.7 (*)
├── thiserror feature "default"
│   └── thiserror v1.0.69
│       └── thiserror-impl feature "default"
│           └── thiserror-impl v1.0.69 (proc-macro)
│               ├── proc-macro2 feature "default"
│               │   ├── proc-macro2 v1.0.101 (*)
│               │   └── proc-macro2 feature "proc-macro" (*)
│               ├── quote feature "default"
│               │   ├── quote v1.0.40 (*)
│               │   └── quote feature "proc-macro" (*)
│               └── syn feature "default"
│                   ├── syn v2.0.106 (*)
│                   ├── syn feature "clone-impls" (*)
│                   ├── syn feature "derive" (*)
│                   ├── syn feature "parsing" (*)
│                   ├── syn feature "printing" (*)
│                   └── syn feature "proc-macro" (*)
├── tokio feature "default"
│   └── tokio v1.47.1
│       ├── mio v1.0.4
│       │   └── libc feature "default" (*)
│       ├── libc feature "default" (*)
│       ├── bytes feature "default"
│       │   ├── bytes v1.10.1
│       │   └── bytes feature "std"
│       │       └── bytes v1.10.1
│       ├── parking_lot feature "default"
│       │   └── parking_lot v0.12.4
│       │       ├── lock_api feature "default" (*)
│       │       └── parking_lot_core feature "default"
│       │           └── parking_lot_core v0.9.11
│       │               ├── libc feature "default" (*)
│       │               ├── cfg-if feature "default" (*)
│       │               └── smallvec feature "default" (*)
│       ├── pin-project-lite feature "default"
│       │   └── pin-project-lite v0.2.16
│       ├── signal-hook-registry feature "default"
│       │   └── signal-hook-registry v1.4.6
│       │       └── libc feature "default" (*)
│       ├── socket2 feature "all"
│       │   └── socket2 v0.6.0
│       │       └── libc feature "default" (*)
│       ├── socket2 feature "default"
│       │   └── socket2 v0.6.0 (*)
│       └── tokio-macros feature "default"
│           └── tokio-macros v2.5.0 (proc-macro)
│               ├── proc-macro2 feature "default" (*)
│               ├── quote feature "default" (*)
│               ├── syn feature "default" (*)
│               └── syn feature "full"
│                   └── syn v2.0.106 (*)
├── tokio feature "full"
│   ├── tokio v1.47.1 (*)
│   ├── tokio feature "fs"
│   │   └── tokio v1.47.1 (*)
│   ├── tokio feature "io-std"
│   │   └── tokio v1.47.1 (*)
│   ├── tokio feature "io-util"
│   │   ├── tokio v1.47.1 (*)
│   │   └── tokio feature "bytes"
│   │       └── tokio v1.47.1 (*)
│   ├── tokio feature "macros"
│   │   ├── tokio v1.47.1 (*)
│   │   └── tokio feature "tokio-macros"
│   │       └── tokio v1.47.1 (*)
│   ├── tokio feature "net"
│   │   ├── tokio v1.47.1 (*)
│   │   ├── tokio feature "libc"
│   │   │   └── tokio v1.47.1 (*)
│   │   ├── tokio feature "mio"
│   │   │   └── tokio v1.47.1 (*)
│   │   ├── tokio feature "socket2"
│   │   │   └── tokio v1.47.1 (*)
│   │   ├── mio feature "net"
│   │   │   └── mio v1.0.4 (*)
│   │   ├── mio feature "os-ext"
│   │   │   ├── mio v1.0.4 (*)
│   │   │   └── mio feature "os-poll"
│   │   │       └── mio v1.0.4 (*)
│   │   └── mio feature "os-poll" (*)
│   ├── tokio feature "parking_lot"
│   │   └── tokio v1.47.1 (*)
│   ├── tokio feature "process"
│   │   ├── tokio v1.47.1 (*)
│   │   ├── tokio feature "bytes" (*)
│   │   ├── tokio feature "libc" (*)
│   │   ├── tokio feature "mio" (*)
│   │   ├── tokio feature "signal-hook-registry"
│   │   │   └── tokio v1.47.1 (*)
│   │   ├── mio feature "net" (*)
│   │   ├── mio feature "os-ext" (*)
│   │   └── mio feature "os-poll" (*)
│   ├── tokio feature "rt"
│   │   └── tokio v1.47.1 (*)
│   ├── tokio feature "rt-multi-thread"
│   │   ├── tokio v1.47.1 (*)
│   │   └── tokio feature "rt" (*)
│   ├── tokio feature "signal"
│   │   ├── tokio v1.47.1 (*)
│   │   ├── tokio feature "libc" (*)
│   │   ├── tokio feature "mio" (*)
│   │   ├── tokio feature "signal-hook-registry" (*)
│   │   ├── mio feature "net" (*)
│   │   ├── mio feature "os-ext" (*)
│   │   └── mio feature "os-poll" (*)
│   ├── tokio feature "sync"
│   │   └── tokio v1.47.1 (*)
│   └── tokio feature "time"
│       └── tokio v1.47.1 (*)
├── tracing feature "default"
│   ├── tracing v0.1.41
│   │   ├── tracing-core v0.1.34
│   │   │   └── once_cell feature "default"
│   │   │       ├── once_cell v1.21.3
│   │   │       └── once_cell feature "std"
│   │   │           ├── once_cell v1.21.3
│   │   │           └── once_cell feature "alloc"
│   │   │               ├── once_cell v1.21.3
│   │   │               └── once_cell feature "race"
│   │   │                   └── once_cell v1.21.3
│   │   ├── pin-project-lite feature "default" (*)
│   │   └── tracing-attributes feature "default"
│   │       └── tracing-attributes v0.1.30 (proc-macro)
│   │           ├── proc-macro2 feature "default" (*)
│   │           ├── quote feature "default" (*)
│   │           ├── syn feature "clone-impls" (*)
│   │           ├── syn feature "extra-traits"
│   │           │   └── syn v2.0.106 (*)
│   │           ├── syn feature "full" (*)
│   │           ├── syn feature "parsing" (*)
│   │           ├── syn feature "printing" (*)
│   │           ├── syn feature "proc-macro" (*)
│   │           └── syn feature "visit-mut"
│   │               └── syn v2.0.106 (*)
│   ├── tracing feature "attributes"
│   │   ├── tracing v0.1.41 (*)
│   │   └── tracing feature "tracing-attributes"
│   │       └── tracing v0.1.41 (*)
│   └── tracing feature "std"
│       ├── tracing v0.1.41 (*)
│       └── tracing-core feature "std"
│           ├── tracing-core v0.1.34 (*)
│           └── tracing-core feature "once_cell"
│               └── tracing-core v0.1.34 (*)
├── transport feature "default"
│   └── transport v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/transport)
│       ├── anyhow feature "default" (*)
│       ├── tokio feature "default" (*)
│       ├── tokio feature "full" (*)
│       ├── tracing feature "default" (*)
│       ├── accounting feature "default"
│       │   └── accounting v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/accounting)
│       │       ├── anyhow feature "default" (*)
│       │       ├── parking_lot feature "default" (*)
│       │       └── workspace-hack feature "default"
│       │           └── workspace-hack v0.1.0 (/Users/mymac/Desktop/RustyOnions/workspace-hack)
│       │               ├── hashbrown feature "default" (*)
│       │               ├── serde feature "alloc"
... (truncated)
```

</details>

### transport

<details><summary>Reverse tree (-i transport -e features)</summary>

```text
```

</details>

<details><summary>Forward tree (-p transport -e features)</summary>

```text
transport v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/transport)
├── accounting feature "default"
│   └── accounting v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/accounting)
│       ├── anyhow feature "default"
│       │   ├── anyhow v1.0.99
│       │   └── anyhow feature "std"
│       │       └── anyhow v1.0.99
│       ├── parking_lot feature "default"
│       │   └── parking_lot v0.12.4
│       │       ├── lock_api feature "default"
│       │       │   ├── lock_api v0.4.13
│       │       │   │   └── scopeguard v1.2.0
│       │       │   │   [build-dependencies]
│       │       │   │   └── autocfg feature "default"
│       │       │   │       └── autocfg v1.5.0
│       │       │   └── lock_api feature "atomic_usize"
│       │       │       └── lock_api v0.4.13 (*)
│       │       └── parking_lot_core feature "default"
│       │           └── parking_lot_core v0.9.11
│       │               ├── cfg-if feature "default"
│       │               │   └── cfg-if v1.0.3
│       │               ├── libc feature "default"
│       │               │   ├── libc v0.2.175
│       │               │   └── libc feature "std"
│       │               │       └── libc v0.2.175
│       │               └── smallvec feature "default"
│       │                   └── smallvec v1.15.1
│       └── workspace-hack feature "default"
│           └── workspace-hack v0.1.0 (/Users/mymac/Desktop/RustyOnions/workspace-hack)
│               ├── smallvec feature "const_new"
│               │   ├── smallvec v1.15.1
│               │   └── smallvec feature "const_generics"
│               │       └── smallvec v1.15.1
│               ├── axum feature "http1"
│               │   ├── axum v0.7.9
│               │   │   ├── async-trait feature "default"
│               │   │   │   └── async-trait v0.1.89 (proc-macro)
│               │   │   │       ├── proc-macro2 feature "default"
│               │   │   │       │   ├── proc-macro2 v1.0.101
│               │   │   │       │   │   └── unicode-ident feature "default"
│               │   │   │       │   │       └── unicode-ident v1.0.19
│               │   │   │       │   └── proc-macro2 feature "proc-macro"
│               │   │   │       │       └── proc-macro2 v1.0.101 (*)
│               │   │   │       ├── quote feature "default"
│               │   │   │       │   ├── quote v1.0.40
│               │   │   │       │   │   └── proc-macro2 v1.0.101 (*)
│               │   │   │       │   └── quote feature "proc-macro"
│               │   │   │       │       ├── quote v1.0.40 (*)
│               │   │   │       │       └── proc-macro2 feature "proc-macro" (*)
│               │   │   │       ├── syn feature "clone-impls"
│               │   │   │       │   └── syn v2.0.106
│               │   │   │       │       ├── proc-macro2 v1.0.101 (*)
│               │   │   │       │       ├── quote v1.0.40 (*)
│               │   │   │       │       └── unicode-ident feature "default" (*)
│               │   │   │       ├── syn feature "full"
│               │   │   │       │   └── syn v2.0.106 (*)
│               │   │   │       ├── syn feature "parsing"
│               │   │   │       │   └── syn v2.0.106 (*)
│               │   │   │       ├── syn feature "printing"
│               │   │   │       │   └── syn v2.0.106 (*)
│               │   │   │       ├── syn feature "proc-macro"
│               │   │   │       │   ├── syn v2.0.106 (*)
│               │   │   │       │   ├── proc-macro2 feature "proc-macro" (*)
│               │   │   │       │   └── quote feature "proc-macro" (*)
│               │   │   │       └── syn feature "visit-mut"
│               │   │   │           └── syn v2.0.106 (*)
│               │   │   ├── axum-core feature "default"
│               │   │   │   └── axum-core v0.4.5
│               │   │   │       ├── async-trait feature "default" (*)
│               │   │   │       ├── bytes feature "default"
│               │   │   │       │   ├── bytes v1.10.1
│               │   │   │       │   └── bytes feature "std"
│               │   │   │       │       └── bytes v1.10.1
│               │   │   │       ├── futures-util feature "alloc"
│               │   │   │       │   ├── futures-util v0.3.31
│               │   │   │       │   │   ├── futures-core v0.3.31
│               │   │   │       │   │   ├── futures-macro v0.3.31 (proc-macro)
│               │   │   │       │   │   │   ├── proc-macro2 feature "default" (*)
│               │   │   │       │   │   │   ├── quote feature "default" (*)
│               │   │   │       │   │   │   ├── syn feature "default"
│               │   │   │       │   │   │   │   ├── syn v2.0.106 (*)
│               │   │   │       │   │   │   │   ├── syn feature "clone-impls" (*)
│               │   │   │       │   │   │   │   ├── syn feature "derive"
│               │   │   │       │   │   │   │   │   └── syn v2.0.106 (*)
│               │   │   │       │   │   │   │   ├── syn feature "parsing" (*)
│               │   │   │       │   │   │   │   ├── syn feature "printing" (*)
│               │   │   │       │   │   │   │   └── syn feature "proc-macro" (*)
│               │   │   │       │   │   │   └── syn feature "full" (*)
│               │   │   │       │   │   ├── futures-sink v0.3.31
│               │   │   │       │   │   ├── futures-task v0.3.31
│               │   │   │       │   │   ├── futures-channel feature "std"
│               │   │   │       │   │   │   ├── futures-channel v0.3.31
│               │   │   │       │   │   │   │   ├── futures-core v0.3.31
│               │   │   │       │   │   │   │   └── futures-sink v0.3.31
│               │   │   │       │   │   │   ├── futures-channel feature "alloc"
│               │   │   │       │   │   │   │   ├── futures-channel v0.3.31 (*)
│               │   │   │       │   │   │   │   └── futures-core feature "alloc"
│               │   │   │       │   │   │   │       └── futures-core v0.3.31
│               │   │   │       │   │   │   └── futures-core feature "std"
│               │   │   │       │   │   │       ├── futures-core v0.3.31
│               │   │   │       │   │   │       └── futures-core feature "alloc" (*)
│               │   │   │       │   │   ├── futures-io feature "std"
│               │   │   │       │   │   │   └── futures-io v0.3.31
│               │   │   │       │   │   ├── memchr feature "default"
│               │   │   │       │   │   │   ├── memchr v2.7.5
│               │   │   │       │   │   │   └── memchr feature "std"
│               │   │   │       │   │   │       ├── memchr v2.7.5
│               │   │   │       │   │   │       └── memchr feature "alloc"
│               │   │   │       │   │   │           └── memchr v2.7.5
│               │   │   │       │   │   ├── pin-project-lite feature "default"
│               │   │   │       │   │   │   └── pin-project-lite v0.2.16
│               │   │   │       │   │   ├── pin-utils feature "default"
│               │   │   │       │   │   │   └── pin-utils v0.1.0
│               │   │   │       │   │   └── slab feature "default"
│               │   │   │       │   │       ├── slab v0.4.11
│               │   │   │       │   │       └── slab feature "std"
│               │   │   │       │   │           └── slab v0.4.11
│               │   │   │       │   ├── futures-core feature "alloc" (*)
│               │   │   │       │   └── futures-task feature "alloc"
│               │   │   │       │       └── futures-task v0.3.31
│               │   │   │       ├── pin-project-lite feature "default" (*)
│               │   │   │       ├── http feature "default"
│               │   │   │       │   ├── http v1.3.1
│               │   │   │       │   │   ├── bytes feature "default" (*)
│               │   │   │       │   │   ├── fnv feature "default"
│               │   │   │       │   │   │   ├── fnv v1.0.7
│               │   │   │       │   │   │   └── fnv feature "std"
│               │   │   │       │   │   │       └── fnv v1.0.7
│               │   │   │       │   │   └── itoa feature "default"
│               │   │   │       │   │       └── itoa v1.0.15
│               │   │   │       │   └── http feature "std"
│               │   │   │       │       └── http v1.3.1 (*)
│               │   │   │       ├── http-body feature "default"
│               │   │   │       │   └── http-body v1.0.1
│               │   │   │       │       ├── bytes feature "default" (*)
│               │   │   │       │       └── http feature "default" (*)
│               │   │   │       ├── http-body-util feature "default"
│               │   │   │       │   └── http-body-util v0.1.3
│               │   │   │       │       ├── futures-core v0.3.31
│               │   │   │       │       ├── bytes feature "default" (*)
│               │   │   │       │       ├── pin-project-lite feature "default" (*)
│               │   │   │       │       ├── http feature "default" (*)
│               │   │   │       │       └── http-body feature "default" (*)
│               │   │   │       ├── mime feature "default"
│               │   │   │       │   └── mime v0.3.17
│               │   │   │       ├── rustversion feature "default"
│               │   │   │       │   └── rustversion v1.0.22 (proc-macro)
│               │   │   │       ├── sync_wrapper feature "default"
│               │   │   │       │   └── sync_wrapper v1.0.2
│               │   │   │       │       └── futures-core v0.3.31
│               │   │   │       ├── tower-layer feature "default"
│               │   │   │       │   └── tower-layer v0.3.3
│               │   │   │       └── tower-service feature "default"
│               │   │   │           └── tower-service v0.3.3
│               │   │   ├── bytes feature "default" (*)
│               │   │   ├── futures-util feature "alloc" (*)
│               │   │   ├── memchr feature "default" (*)
│               │   │   ├── pin-project-lite feature "default" (*)
│               │   │   ├── http feature "default" (*)
│               │   │   ├── itoa feature "default" (*)
│               │   │   ├── http-body feature "default" (*)
│               │   │   ├── http-body-util feature "default" (*)
│               │   │   ├── mime feature "default" (*)
│               │   │   ├── rustversion feature "default" (*)
│               │   │   ├── sync_wrapper feature "default" (*)
│               │   │   ├── tower-layer feature "default" (*)
│               │   │   ├── tower-service feature "default" (*)
│               │   │   ├── axum-macros feature "default"
│               │   │   │   └── axum-macros v0.4.2 (proc-macro)
│               │   │   │       ├── proc-macro2 feature "default" (*)
│               │   │   │       ├── quote feature "default" (*)
│               │   │   │       ├── syn feature "default" (*)
│               │   │   │       ├── syn feature "extra-traits"
│               │   │   │       │   └── syn v2.0.106 (*)
│               │   │   │       ├── syn feature "full" (*)
│               │   │   │       └── syn feature "parsing" (*)
│               │   │   ├── hyper feature "default"
│               │   │   │   └── hyper v1.7.0
│               │   │   │       ├── smallvec feature "const_generics" (*)
│               │   │   │       ├── smallvec feature "const_new" (*)
│               │   │   │       ├── smallvec feature "default" (*)
│               │   │   │       ├── bytes feature "default" (*)
│               │   │   │       ├── futures-channel feature "default"
│               │   │   │       │   ├── futures-channel v0.3.31 (*)
│               │   │   │       │   └── futures-channel feature "std" (*)
│               │   │   │       ├── futures-core feature "default"
│               │   │   │       │   ├── futures-core v0.3.31
│               │   │   │       │   └── futures-core feature "std" (*)
│               │   │   │       ├── pin-project-lite feature "default" (*)
│               │   │   │       ├── pin-utils feature "default" (*)
│               │   │   │       ├── http feature "default" (*)
│               │   │   │       ├── itoa feature "default" (*)
│               │   │   │       ├── http-body feature "default" (*)
│               │   │   │       ├── atomic-waker feature "default"
│               │   │   │       │   └── atomic-waker v1.1.2
│               │   │   │       ├── h2 feature "default"
│               │   │   │       │   └── h2 v0.4.12
│               │   │   │       │       ├── futures-core v0.3.31
│               │   │   │       │       ├── futures-sink v0.3.31
│               │   │   │       │       ├── bytes feature "default" (*)
│               │   │   │       │       ├── slab feature "default" (*)
│               │   │   │       │       ├── http feature "default" (*)
│               │   │   │       │       ├── fnv feature "default" (*)
│               │   │   │       │       ├── atomic-waker feature "default" (*)
│               │   │   │       │       ├── indexmap feature "default"
│               │   │   │       │       │   ├── indexmap v2.11.1
│               │   │   │       │       │   │   ├── equivalent v1.0.2
│               │   │   │       │       │   │   └── hashbrown v0.15.5
│               │   │   │       │       │   │       ├── equivalent v1.0.2
│               │   │   │       │       │   │       ├── foldhash v0.1.5
│               │   │   │       │       │   │       └── allocator-api2 feature "alloc"
│               │   │   │       │       │   │           └── allocator-api2 v0.2.21
│               │   │   │       │       │   └── indexmap feature "std"
│               │   │   │       │       │       └── indexmap v2.11.1 (*)
│               │   │   │       │       ├── indexmap feature "std" (*)
│               │   │   │       │       ├── tokio feature "default"
│               │   │   │       │       │   └── tokio v1.47.1
│               │   │   │       │       │       ├── mio v1.0.4
│               │   │   │       │       │       │   └── libc feature "default" (*)
│               │   │   │       │       │       ├── parking_lot feature "default" (*)
│               │   │   │       │       │       ├── libc feature "default" (*)
│               │   │   │       │       │       ├── bytes feature "default" (*)
│               │   │   │       │       │       ├── pin-project-lite feature "default" (*)
│               │   │   │       │       │       ├── signal-hook-registry feature "default"
│               │   │   │       │       │       │   └── signal-hook-registry v1.4.6
│               │   │   │       │       │       │       └── libc feature "default" (*)
│               │   │   │       │       │       ├── socket2 feature "all"
│               │   │   │       │       │       │   └── socket2 v0.6.0
│               │   │   │       │       │       │       └── libc feature "default" (*)
│               │   │   │       │       │       ├── socket2 feature "default"
│               │   │   │       │       │       │   └── socket2 v0.6.0 (*)
│               │   │   │       │       │       └── tokio-macros feature "default"
│               │   │   │       │       │           └── tokio-macros v2.5.0 (proc-macro)
│               │   │   │       │       │               ├── proc-macro2 feature "default" (*)
│               │   │   │       │       │               ├── quote feature "default" (*)
│               │   │   │       │       │               ├── syn feature "default" (*)
│               │   │   │       │       │               └── syn feature "full" (*)
│               │   │   │       │       ├── tokio feature "io-util"
│               │   │   │       │       │   ├── tokio v1.47.1 (*)
│               │   │   │       │       │   └── tokio feature "bytes"
│               │   │   │       │       │       └── tokio v1.47.1 (*)
│               │   │   │       │       ├── tokio-util feature "codec"
│               │   │   │       │       │   └── tokio-util v0.7.16
│               │   │   │       │       │       ├── bytes feature "default" (*)
│               │   │   │       │       │       ├── futures-core feature "default" (*)
│               │   │   │       │       │       ├── futures-sink feature "default"
│               │   │   │       │       │       │   ├── futures-sink v0.3.31
│               │   │   │       │       │       │   └── futures-sink feature "std"
│               │   │   │       │       │       │       ├── futures-sink v0.3.31
│               │   │   │       │       │       │       └── futures-sink feature "alloc"
│               │   │   │       │       │       │           └── futures-sink v0.3.31
│               │   │   │       │       │       ├── pin-project-lite feature "default" (*)
│               │   │   │       │       │       ├── tokio feature "default" (*)
│               │   │   │       │       │       └── tokio feature "sync"
│               │   │   │       │       │           └── tokio v1.47.1 (*)
│               │   │   │       │       ├── tokio-util feature "default"
│               │   │   │       │       │   └── tokio-util v0.7.16 (*)
│               │   │   │       │       ├── tokio-util feature "io"
│               │   │   │       │       │   └── tokio-util v0.7.16 (*)
│               │   │   │       │       └── tracing feature "std"
│               │   │   │       │           ├── tracing v0.1.41
│               │   │   │       │           │   ├── tracing-core v0.1.34
│               │   │   │       │           │   │   └── once_cell feature "default"
│               │   │   │       │           │   │       ├── once_cell v1.21.3
│               │   │   │       │           │   │       └── once_cell feature "std"
│               │   │   │       │           │   │           ├── once_cell v1.21.3
│               │   │   │       │           │   │           └── once_cell feature "alloc"
│               │   │   │       │           │   │               ├── once_cell v1.21.3
│               │   │   │       │           │   │               └── once_cell feature "race"
│               │   │   │       │           │   │                   └── once_cell v1.21.3
│               │   │   │       │           │   ├── pin-project-lite feature "default" (*)
│               │   │   │       │           │   └── tracing-attributes feature "default"
│               │   │   │       │           │       └── tracing-attributes v0.1.30 (proc-macro)
│               │   │   │       │           │           ├── proc-macro2 feature "default" (*)
│               │   │   │       │           │           ├── quote feature "default" (*)
│               │   │   │       │           │           ├── syn feature "clone-impls" (*)
│               │   │   │       │           │           ├── syn feature "extra-traits" (*)
│               │   │   │       │           │           ├── syn feature "full" (*)
│               │   │   │       │           │           ├── syn feature "parsing" (*)
│               │   │   │       │           │           ├── syn feature "printing" (*)
│               │   │   │       │           │           ├── syn feature "proc-macro" (*)
│               │   │   │       │           │           └── syn feature "visit-mut" (*)
│               │   │   │       │           └── tracing-core feature "std"
│               │   │   │       │               ├── tracing-core v0.1.34 (*)
│               │   │   │       │               └── tracing-core feature "once_cell"
│               │   │   │       │                   └── tracing-core v0.1.34 (*)
│               │   │   │       ├── tokio feature "default" (*)
│               │   │   │       ├── tokio feature "sync" (*)
│               │   │   │       ├── httparse feature "default"
│               │   │   │       │   ├── httparse v1.10.1
│               │   │   │       │   └── httparse feature "std"
│               │   │   │       │       └── httparse v1.10.1
│               │   │   │       ├── httpdate feature "default"
│               │   │   │       │   └── httpdate v1.0.3
│               │   │   │       └── want feature "default"
│               │   │   │           └── want v0.3.1
│               │   │   │               └── try-lock feature "default"
│               │   │   │                   └── try-lock v0.2.5
│               │   │   ├── tokio feature "default" (*)
│               │   │   ├── tokio feature "time"
... (truncated)
```

</details>

### node

<details><summary>Reverse tree (-i node -e features)</summary>

```text
```

</details>

<details><summary>Forward tree (-p node -e features)</summary>

```text
node v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/node)
├── anyhow feature "default"
│   ├── anyhow v1.0.99
│   └── anyhow feature "std"
│       └── anyhow v1.0.99
├── clap feature "default"
│   ├── clap v4.5.47
│   │   ├── clap_builder v4.5.47
│   │   │   ├── anstream feature "default"
│   │   │   │   ├── anstream v0.6.20
│   │   │   │   │   ├── anstyle feature "default"
│   │   │   │   │   │   ├── anstyle v1.0.11
│   │   │   │   │   │   └── anstyle feature "std"
│   │   │   │   │   │       └── anstyle v1.0.11
│   │   │   │   │   ├── anstyle-parse feature "default"
│   │   │   │   │   │   ├── anstyle-parse v0.2.7
│   │   │   │   │   │   │   └── utf8parse feature "default"
│   │   │   │   │   │   │       └── utf8parse v0.2.2
│   │   │   │   │   │   └── anstyle-parse feature "utf8"
│   │   │   │   │   │       └── anstyle-parse v0.2.7 (*)
│   │   │   │   │   ├── utf8parse feature "default" (*)
│   │   │   │   │   ├── anstyle-query feature "default"
│   │   │   │   │   │   └── anstyle-query v1.1.4
│   │   │   │   │   ├── colorchoice feature "default"
│   │   │   │   │   │   └── colorchoice v1.0.4
│   │   │   │   │   └── is_terminal_polyfill feature "default"
│   │   │   │   │       └── is_terminal_polyfill v1.70.1
│   │   │   │   ├── anstream feature "auto"
│   │   │   │   │   └── anstream v0.6.20 (*)
│   │   │   │   └── anstream feature "wincon"
│   │   │   │       └── anstream v0.6.20 (*)
│   │   │   ├── anstyle feature "default" (*)
│   │   │   ├── clap_lex feature "default"
│   │   │   │   └── clap_lex v0.7.5
│   │   │   └── strsim feature "default"
│   │   │       └── strsim v0.11.1
│   │   └── clap_derive feature "default"
│   │       └── clap_derive v4.5.47 (proc-macro)
│   │           ├── heck feature "default"
│   │           │   └── heck v0.5.0
│   │           ├── proc-macro2 feature "default"
│   │           │   ├── proc-macro2 v1.0.101
│   │           │   │   └── unicode-ident feature "default"
│   │           │   │       └── unicode-ident v1.0.19
│   │           │   └── proc-macro2 feature "proc-macro"
│   │           │       └── proc-macro2 v1.0.101 (*)
│   │           ├── quote feature "default"
│   │           │   ├── quote v1.0.40
│   │           │   │   └── proc-macro2 v1.0.101 (*)
│   │           │   └── quote feature "proc-macro"
│   │           │       ├── quote v1.0.40 (*)
│   │           │       └── proc-macro2 feature "proc-macro" (*)
│   │           ├── syn feature "default"
│   │           │   ├── syn v2.0.106
│   │           │   │   ├── proc-macro2 v1.0.101 (*)
│   │           │   │   ├── quote v1.0.40 (*)
│   │           │   │   └── unicode-ident feature "default" (*)
│   │           │   ├── syn feature "clone-impls"
│   │           │   │   └── syn v2.0.106 (*)
│   │           │   ├── syn feature "derive"
│   │           │   │   └── syn v2.0.106 (*)
│   │           │   ├── syn feature "parsing"
│   │           │   │   └── syn v2.0.106 (*)
│   │           │   ├── syn feature "printing"
│   │           │   │   └── syn v2.0.106 (*)
│   │           │   └── syn feature "proc-macro"
│   │           │       ├── syn v2.0.106 (*)
│   │           │       ├── proc-macro2 feature "proc-macro" (*)
│   │           │       └── quote feature "proc-macro" (*)
│   │           └── syn feature "full"
│   │               └── syn v2.0.106 (*)
│   ├── clap feature "color"
│   │   ├── clap v4.5.47 (*)
│   │   └── clap_builder feature "color"
│   │       └── clap_builder v4.5.47 (*)
│   ├── clap feature "error-context"
│   │   ├── clap v4.5.47 (*)
│   │   └── clap_builder feature "error-context"
│   │       └── clap_builder v4.5.47 (*)
│   ├── clap feature "help"
│   │   ├── clap v4.5.47 (*)
│   │   └── clap_builder feature "help"
│   │       └── clap_builder v4.5.47 (*)
│   ├── clap feature "std"
│   │   ├── clap v4.5.47 (*)
│   │   └── clap_builder feature "std"
│   │       ├── clap_builder v4.5.47 (*)
│   │       └── anstyle feature "std" (*)
│   ├── clap feature "suggestions"
│   │   ├── clap v4.5.47 (*)
│   │   └── clap_builder feature "suggestions"
│   │       ├── clap_builder v4.5.47 (*)
│   │       └── clap_builder feature "error-context" (*)
│   └── clap feature "usage"
│       ├── clap v4.5.47 (*)
│       └── clap_builder feature "usage"
│           └── clap_builder v4.5.47 (*)
├── color-eyre feature "default"
│   ├── color-eyre v0.6.5
│   │   ├── backtrace feature "default"
│   │   │   ├── backtrace v0.3.75
│   │   │   │   ├── addr2line v0.24.2
│   │   │   │   │   └── gimli feature "read"
│   │   │   │   │       ├── gimli v0.31.1
│   │   │   │   │       └── gimli feature "read-core"
│   │   │   │   │           └── gimli v0.31.1
│   │   │   │   ├── libc v0.2.175
│   │   │   │   ├── miniz_oxide v0.8.9
│   │   │   │   │   └── adler2 v2.0.1
│   │   │   │   ├── cfg-if feature "default"
│   │   │   │   │   └── cfg-if v1.0.3
│   │   │   │   ├── object feature "archive"
│   │   │   │   │   └── object v0.36.7
│   │   │   │   │       └── memchr v2.7.5
│   │   │   │   ├── object feature "elf"
│   │   │   │   │   └── object v0.36.7 (*)
│   │   │   │   ├── object feature "macho"
│   │   │   │   │   └── object v0.36.7 (*)
│   │   │   │   ├── object feature "pe"
│   │   │   │   │   ├── object v0.36.7 (*)
│   │   │   │   │   └── object feature "coff"
│   │   │   │   │       └── object v0.36.7 (*)
│   │   │   │   ├── object feature "read_core"
│   │   │   │   │   └── object v0.36.7 (*)
│   │   │   │   ├── object feature "unaligned"
│   │   │   │   │   └── object v0.36.7 (*)
│   │   │   │   ├── object feature "xcoff"
│   │   │   │   │   └── object v0.36.7 (*)
│   │   │   │   └── rustc-demangle feature "default"
│   │   │   │       └── rustc-demangle v0.1.26
│   │   │   └── backtrace feature "std"
│   │   │       └── backtrace v0.3.75 (*)
│   │   ├── color-spantrace feature "default"
│   │   │   └── color-spantrace v0.3.0
│   │   │       ├── once_cell feature "default"
│   │   │       │   ├── once_cell v1.21.3
│   │   │       │   └── once_cell feature "std"
│   │   │       │       ├── once_cell v1.21.3
│   │   │       │       └── once_cell feature "alloc"
│   │   │       │           ├── once_cell v1.21.3
│   │   │       │           └── once_cell feature "race"
│   │   │       │               └── once_cell v1.21.3
│   │   │       ├── owo-colors feature "default"
│   │   │       │   └── owo-colors v4.2.2
│   │   │       ├── tracing-core feature "default"
│   │   │       │   ├── tracing-core v0.1.34
│   │   │       │   │   └── once_cell feature "default" (*)
│   │   │       │   └── tracing-core feature "std"
│   │   │       │       ├── tracing-core v0.1.34 (*)
│   │   │       │       └── tracing-core feature "once_cell"
│   │   │       │           └── tracing-core v0.1.34 (*)
│   │   │       └── tracing-error feature "default"
│   │   │           ├── tracing-error v0.2.1
│   │   │           │   ├── tracing feature "std"
│   │   │           │   │   ├── tracing v0.1.41
│   │   │           │   │   │   ├── tracing-core v0.1.34 (*)
│   │   │           │   │   │   ├── pin-project-lite feature "default"
│   │   │           │   │   │   │   └── pin-project-lite v0.2.16
│   │   │           │   │   │   └── tracing-attributes feature "default"
│   │   │           │   │   │       └── tracing-attributes v0.1.30 (proc-macro)
│   │   │           │   │   │           ├── proc-macro2 feature "default" (*)
│   │   │           │   │   │           ├── quote feature "default" (*)
│   │   │           │   │   │           ├── syn feature "clone-impls" (*)
│   │   │           │   │   │           ├── syn feature "extra-traits"
│   │   │           │   │   │           │   └── syn v2.0.106 (*)
│   │   │           │   │   │           ├── syn feature "full" (*)
│   │   │           │   │   │           ├── syn feature "parsing" (*)
│   │   │           │   │   │           ├── syn feature "printing" (*)
│   │   │           │   │   │           ├── syn feature "proc-macro" (*)
│   │   │           │   │   │           └── syn feature "visit-mut"
│   │   │           │   │   │               └── syn v2.0.106 (*)
│   │   │           │   │   └── tracing-core feature "std" (*)
│   │   │           │   ├── tracing-subscriber feature "fmt"
│   │   │           │   │   ├── tracing-subscriber v0.3.20
│   │   │           │   │   │   ├── tracing v0.1.41 (*)
│   │   │           │   │   │   ├── tracing-core v0.1.34 (*)
│   │   │           │   │   │   ├── once_cell feature "default" (*)
│   │   │           │   │   │   ├── matchers feature "default"
│   │   │           │   │   │   │   └── matchers v0.2.0
│   │   │           │   │   │   │       ├── regex-automata feature "dfa-build"
│   │   │           │   │   │   │       │   ├── regex-automata v0.4.10
│   │   │           │   │   │   │       │   │   ├── aho-corasick v1.1.3
│   │   │           │   │   │   │       │   │   │   └── memchr v2.7.5
│   │   │           │   │   │   │       │   │   ├── memchr v2.7.5
│   │   │           │   │   │   │       │   │   └── regex-syntax v0.8.6
│   │   │           │   │   │   │       │   ├── regex-automata feature "dfa-search"
│   │   │           │   │   │   │       │   │   └── regex-automata v0.4.10 (*)
│   │   │           │   │   │   │       │   └── regex-automata feature "nfa-thompson"
│   │   │           │   │   │   │       │       ├── regex-automata v0.4.10 (*)
│   │   │           │   │   │   │       │       └── regex-automata feature "alloc"
│   │   │           │   │   │   │       │           └── regex-automata v0.4.10 (*)
│   │   │           │   │   │   │       ├── regex-automata feature "dfa-search" (*)
│   │   │           │   │   │   │       └── regex-automata feature "syntax"
│   │   │           │   │   │   │           ├── regex-automata v0.4.10 (*)
│   │   │           │   │   │   │           └── regex-automata feature "alloc" (*)
│   │   │           │   │   │   ├── regex-automata feature "std"
│   │   │           │   │   │   │   ├── regex-automata v0.4.10 (*)
│   │   │           │   │   │   │   ├── memchr feature "std"
│   │   │           │   │   │   │   │   ├── memchr v2.7.5
│   │   │           │   │   │   │   │   └── memchr feature "alloc"
│   │   │           │   │   │   │   │       └── memchr v2.7.5
│   │   │           │   │   │   │   ├── regex-automata feature "alloc" (*)
│   │   │           │   │   │   │   ├── aho-corasick feature "std"
│   │   │           │   │   │   │   │   ├── aho-corasick v1.1.3 (*)
│   │   │           │   │   │   │   │   └── memchr feature "std" (*)
│   │   │           │   │   │   │   └── regex-syntax feature "std"
│   │   │           │   │   │   │       └── regex-syntax v0.8.6
│   │   │           │   │   │   ├── nu-ansi-term feature "default"
│   │   │           │   │   │   │   └── nu-ansi-term v0.50.1
│   │   │           │   │   │   ├── serde feature "default"
│   │   │           │   │   │   │   ├── serde v1.0.221
│   │   │           │   │   │   │   │   ├── serde_core feature "result"
│   │   │           │   │   │   │   │   │   └── serde_core v1.0.221
│   │   │           │   │   │   │   │   └── serde_derive feature "default"
│   │   │           │   │   │   │   │       └── serde_derive v1.0.221 (proc-macro)
│   │   │           │   │   │   │   │           ├── proc-macro2 feature "proc-macro" (*)
│   │   │           │   │   │   │   │           ├── quote feature "proc-macro" (*)
│   │   │           │   │   │   │   │           ├── syn feature "clone-impls" (*)
│   │   │           │   │   │   │   │           ├── syn feature "derive" (*)
│   │   │           │   │   │   │   │           ├── syn feature "parsing" (*)
│   │   │           │   │   │   │   │           ├── syn feature "printing" (*)
│   │   │           │   │   │   │   │           └── syn feature "proc-macro" (*)
│   │   │           │   │   │   │   └── serde feature "std"
│   │   │           │   │   │   │       ├── serde v1.0.221 (*)
│   │   │           │   │   │   │       └── serde_core feature "std"
│   │   │           │   │   │   │           └── serde_core v1.0.221
│   │   │           │   │   │   ├── serde_json feature "default"
│   │   │           │   │   │   │   ├── serde_json v1.0.144
│   │   │           │   │   │   │   │   ├── memchr v2.7.5
│   │   │           │   │   │   │   │   ├── serde_core v1.0.221
│   │   │           │   │   │   │   │   ├── itoa feature "default"
│   │   │           │   │   │   │   │   │   └── itoa v1.0.15
│   │   │           │   │   │   │   │   └── ryu feature "default"
│   │   │           │   │   │   │   │       └── ryu v1.0.20
│   │   │           │   │   │   │   └── serde_json feature "std"
│   │   │           │   │   │   │       ├── serde_json v1.0.144 (*)
│   │   │           │   │   │   │       ├── memchr feature "std" (*)
│   │   │           │   │   │   │       └── serde_core feature "std" (*)
│   │   │           │   │   │   ├── sharded-slab feature "default"
│   │   │           │   │   │   │   └── sharded-slab v0.1.7
│   │   │           │   │   │   │       └── lazy_static feature "default"
│   │   │           │   │   │   │           └── lazy_static v1.5.0
│   │   │           │   │   │   ├── smallvec feature "default"
│   │   │           │   │   │   │   └── smallvec v1.15.1
│   │   │           │   │   │   ├── thread_local feature "default"
│   │   │           │   │   │   │   └── thread_local v1.1.9
│   │   │           │   │   │   │       └── cfg-if feature "default" (*)
│   │   │           │   │   │   ├── tracing-log feature "log-tracer"
│   │   │           │   │   │   │   └── tracing-log v0.2.0
│   │   │           │   │   │   │       ├── once_cell feature "default" (*)
│   │   │           │   │   │   │       ├── tracing-core feature "default" (*)
│   │   │           │   │   │   │       └── log feature "default"
│   │   │           │   │   │   │           └── log v0.4.28
│   │   │           │   │   │   ├── tracing-log feature "std"
│   │   │           │   │   │   │   ├── tracing-log v0.2.0 (*)
│   │   │           │   │   │   │   └── log feature "std"
│   │   │           │   │   │   │       └── log v0.4.28
│   │   │           │   │   │   └── tracing-serde feature "default"
│   │   │           │   │   │       └── tracing-serde v0.2.0
│   │   │           │   │   │           ├── tracing-core feature "default" (*)
│   │   │           │   │   │           └── serde feature "default" (*)
│   │   │           │   │   ├── tracing-subscriber feature "registry"
│   │   │           │   │   │   ├── tracing-subscriber v0.3.20 (*)
│   │   │           │   │   │   ├── tracing-subscriber feature "sharded-slab"
│   │   │           │   │   │   │   └── tracing-subscriber v0.3.20 (*)
│   │   │           │   │   │   ├── tracing-subscriber feature "std"
│   │   │           │   │   │   │   ├── tracing-subscriber v0.3.20 (*)
│   │   │           │   │   │   │   ├── tracing-core feature "std" (*)
│   │   │           │   │   │   │   └── tracing-subscriber feature "alloc"
│   │   │           │   │   │   │       └── tracing-subscriber v0.3.20 (*)
│   │   │           │   │   │   └── tracing-subscriber feature "thread_local"
│   │   │           │   │   │       └── tracing-subscriber v0.3.20 (*)
│   │   │           │   │   └── tracing-subscriber feature "std" (*)
│   │   │           │   └── tracing-subscriber feature "registry" (*)
│   │   │           └── tracing-error feature "traced-error"
│   │   │               └── tracing-error v0.2.1 (*)
│   │   ├── once_cell feature "default" (*)
│   │   ├── owo-colors feature "default" (*)
│   │   ├── tracing-error feature "default" (*)
│   │   ├── eyre feature "default"
│   │   │   ├── eyre v0.6.12
│   │   │   │   ├── once_cell feature "default" (*)
│   │   │   │   └── indenter feature "default"
│   │   │   │       └── indenter v0.3.4
│   │   │   ├── eyre feature "auto-install"
│   │   │   │   └── eyre v0.6.12 (*)
│   │   │   └── eyre feature "track-caller"
│   │   │       └── eyre v0.6.12 (*)
│   │   └── indenter feature "default" (*)
│   ├── color-eyre feature "capture-spantrace"
│   │   ├── color-eyre v0.6.5 (*)
│   │   ├── color-eyre feature "color-spantrace"
│   │   │   └── color-eyre v0.6.5 (*)
│   │   └── color-eyre feature "tracing-error"
│   │       └── color-eyre v0.6.5 (*)
│   └── color-eyre feature "track-caller"
│       └── color-eyre v0.6.5 (*)
├── tracing feature "default"
│   ├── tracing v0.1.41 (*)
│   ├── tracing feature "attributes"
... (truncated)
```

</details>

### naming

<details><summary>Reverse tree (-i naming -e features)</summary>

```text
```

</details>

<details><summary>Forward tree (-p naming -e features)</summary>

```text
naming v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/naming)
├── anyhow feature "default"
│   ├── anyhow v1.0.99
│   └── anyhow feature "std"
│       └── anyhow v1.0.99
├── base64 feature "default"
│   ├── base64 v0.22.1
│   └── base64 feature "std"
│       ├── base64 v0.22.1
│       └── base64 feature "alloc"
│           └── base64 v0.22.1
├── blake3 feature "default"
│   ├── blake3 v1.8.2
│   │   ├── arrayvec v0.7.6
│   │   ├── constant_time_eq v0.3.1
│   │   ├── arrayref feature "default"
│   │   │   └── arrayref v0.3.9
│   │   └── cfg-if feature "default"
│   │       └── cfg-if v1.0.3
│   │   [build-dependencies]
│   │   └── cc feature "default"
│   │       └── cc v1.2.37
│   │           ├── jobserver v0.1.34
│   │           │   └── libc feature "default"
│   │           │       ├── libc v0.2.175
│   │           │       └── libc feature "std"
│   │           │           └── libc v0.2.175
│   │           ├── libc v0.2.175
│   │           ├── find-msvc-tools feature "default"
│   │           │   └── find-msvc-tools v0.1.1
│   │           └── shlex feature "default"
│   │               ├── shlex v1.3.0
│   │               └── shlex feature "std"
│   │                   └── shlex v1.3.0
│   └── blake3 feature "std"
│       └── blake3 v1.8.2 (*)
├── chrono feature "default"
│   ├── chrono v0.4.42
│   │   ├── num-traits v0.2.19
│   │   │   [build-dependencies]
│   │   │   └── autocfg feature "default"
│   │   │       └── autocfg v1.5.0
│   │   ├── iana-time-zone feature "default"
│   │   │   └── iana-time-zone v0.1.64
│   │   │       └── core-foundation-sys feature "default"
│   │   │           ├── core-foundation-sys v0.8.7
│   │   │           └── core-foundation-sys feature "link"
│   │   │               └── core-foundation-sys v0.8.7
│   │   └── iana-time-zone feature "fallback"
│   │       └── iana-time-zone v0.1.64 (*)
│   ├── chrono feature "clock"
│   │   ├── chrono v0.4.42 (*)
│   │   ├── chrono feature "iana-time-zone"
│   │   │   └── chrono v0.4.42 (*)
│   │   ├── chrono feature "now"
│   │   │   ├── chrono v0.4.42 (*)
│   │   │   └── chrono feature "std"
│   │   │       ├── chrono v0.4.42 (*)
│   │   │       └── chrono feature "alloc"
│   │   │           └── chrono v0.4.42 (*)
│   │   └── chrono feature "winapi"
│   │       ├── chrono v0.4.42 (*)
│   │       └── chrono feature "windows-link"
│   │           └── chrono v0.4.42 (*)
│   ├── chrono feature "oldtime"
│   │   └── chrono v0.4.42 (*)
│   ├── chrono feature "std" (*)
│   └── chrono feature "wasmbind"
│       ├── chrono v0.4.42 (*)
│       ├── chrono feature "js-sys"
│       │   └── chrono v0.4.42 (*)
│       └── chrono feature "wasm-bindgen"
│           └── chrono v0.4.42 (*)
├── hex feature "default"
│   ├── hex v0.4.3
│   └── hex feature "std"
│       ├── hex v0.4.3
│       └── hex feature "alloc"
│           └── hex v0.4.3
├── mime feature "default"
│   └── mime v0.3.17
├── mime_guess feature "default"
│   ├── mime_guess v2.0.5
│   │   ├── mime feature "default" (*)
│   │   └── unicase feature "default"
│   │       └── unicase v2.8.1
│   │   [build-dependencies]
│   │   └── unicase feature "default" (*)
│   └── mime_guess feature "rev-mappings"
│       └── mime_guess v2.0.5 (*)
├── serde feature "default"
│   ├── serde v1.0.221
│   │   ├── serde_core feature "result"
│   │   │   └── serde_core v1.0.221
│   │   └── serde_derive feature "default"
│   │       └── serde_derive v1.0.221 (proc-macro)
│   │           ├── proc-macro2 feature "proc-macro"
│   │           │   └── proc-macro2 v1.0.101
│   │           │       └── unicode-ident feature "default"
│   │           │           └── unicode-ident v1.0.19
│   │           ├── quote feature "proc-macro"
│   │           │   ├── quote v1.0.40
│   │           │   │   └── proc-macro2 v1.0.101 (*)
│   │           │   └── proc-macro2 feature "proc-macro" (*)
│   │           ├── syn feature "clone-impls"
│   │           │   └── syn v2.0.106
│   │           │       ├── proc-macro2 v1.0.101 (*)
│   │           │       ├── quote v1.0.40 (*)
│   │           │       └── unicode-ident feature "default" (*)
│   │           ├── syn feature "derive"
│   │           │   └── syn v2.0.106 (*)
│   │           ├── syn feature "parsing"
│   │           │   └── syn v2.0.106 (*)
│   │           ├── syn feature "printing"
│   │           │   └── syn v2.0.106 (*)
│   │           └── syn feature "proc-macro"
│   │               ├── syn v2.0.106 (*)
│   │               ├── proc-macro2 feature "proc-macro" (*)
│   │               └── quote feature "proc-macro" (*)
│   └── serde feature "std"
│       ├── serde v1.0.221 (*)
│       └── serde_core feature "std"
│           └── serde_core v1.0.221
├── serde_with feature "default"
│   ├── serde_with v3.14.0
│   │   ├── serde v1.0.221 (*)
│   │   ├── serde_derive feature "default" (*)
│   │   └── serde_with_macros feature "default"
│   │       └── serde_with_macros v3.14.0 (proc-macro)
│   │           ├── proc-macro2 feature "default"
│   │           │   ├── proc-macro2 v1.0.101 (*)
│   │           │   └── proc-macro2 feature "proc-macro" (*)
│   │           ├── quote feature "default"
│   │           │   ├── quote v1.0.40 (*)
│   │           │   └── quote feature "proc-macro" (*)
│   │           ├── syn feature "default"
│   │           │   ├── syn v2.0.106 (*)
│   │           │   ├── syn feature "clone-impls" (*)
│   │           │   ├── syn feature "derive" (*)
│   │           │   ├── syn feature "parsing" (*)
│   │           │   ├── syn feature "printing" (*)
│   │           │   └── syn feature "proc-macro" (*)
│   │           ├── syn feature "extra-traits"
│   │           │   └── syn v2.0.106 (*)
│   │           ├── syn feature "full"
│   │           │   └── syn v2.0.106 (*)
│   │           ├── syn feature "parsing" (*)
│   │           └── darling feature "default"
│   │               ├── darling v0.20.11
│   │               │   ├── darling_core feature "default"
│   │               │   │   └── darling_core v0.20.11
│   │               │   │       ├── proc-macro2 feature "default" (*)
│   │               │   │       ├── quote feature "default" (*)
│   │               │   │       ├── syn feature "default" (*)
│   │               │   │       ├── syn feature "extra-traits" (*)
│   │               │   │       ├── syn feature "full" (*)
│   │               │   │       ├── fnv feature "default"
│   │               │   │       │   ├── fnv v1.0.7
│   │               │   │       │   └── fnv feature "std"
│   │               │   │       │       └── fnv v1.0.7
│   │               │   │       ├── ident_case feature "default"
│   │               │   │       │   └── ident_case v1.0.1
│   │               │   │       └── strsim feature "default"
│   │               │   │           └── strsim v0.11.1
│   │               │   └── darling_macro feature "default"
│   │               │       └── darling_macro v0.20.11 (proc-macro)
│   │               │           ├── quote feature "default" (*)
│   │               │           ├── syn feature "default" (*)
│   │               │           └── darling_core feature "default" (*)
│   │               └── darling feature "suggestions"
│   │                   ├── darling v0.20.11 (*)
│   │                   └── darling_core feature "suggestions"
│   │                       ├── darling_core v0.20.11 (*)
│   │                       └── darling_core feature "strsim"
│   │                           └── darling_core v0.20.11 (*)
│   ├── serde_with feature "macros"
│   │   └── serde_with v3.14.0 (*)
│   └── serde_with feature "std"
│       ├── serde_with v3.14.0 (*)
│       ├── serde feature "std" (*)
│       └── serde_with feature "alloc"
│           ├── serde_with v3.14.0 (*)
│           └── serde feature "alloc"
│               ├── serde v1.0.221 (*)
│               └── serde_core feature "alloc"
│                   └── serde_core v1.0.221
├── thiserror feature "default"
│   └── thiserror v1.0.69
│       └── thiserror-impl feature "default"
│           └── thiserror-impl v1.0.69 (proc-macro)
│               ├── proc-macro2 feature "default" (*)
│               ├── quote feature "default" (*)
│               └── syn feature "default" (*)
├── toml feature "default"
│   ├── toml v0.8.23
│   │   ├── serde feature "default" (*)
│   │   ├── serde_spanned feature "default"
│   │   │   └── serde_spanned v0.6.9
│   │   │       └── serde feature "default" (*)
│   │   ├── serde_spanned feature "serde"
│   │   │   └── serde_spanned v0.6.9 (*)
│   │   ├── toml_datetime feature "default"
│   │   │   └── toml_datetime v0.6.11
│   │   │       └── serde feature "default" (*)
│   │   ├── toml_datetime feature "serde"
│   │   │   └── toml_datetime v0.6.11 (*)
│   │   └── toml_edit feature "serde"
│   │       ├── toml_edit v0.22.27
│   │       │   ├── serde feature "default" (*)
│   │       │   ├── serde_spanned feature "default" (*)
│   │       │   ├── serde_spanned feature "serde" (*)
│   │       │   ├── toml_datetime feature "default" (*)
│   │       │   ├── indexmap feature "default"
│   │       │   │   ├── indexmap v2.11.1
│   │       │   │   │   ├── equivalent v1.0.2
│   │       │   │   │   └── hashbrown v0.15.5
│   │       │   │   │       ├── equivalent v1.0.2
│   │       │   │   │       ├── foldhash v0.1.5
│   │       │   │   │       └── allocator-api2 feature "alloc"
│   │       │   │   │           └── allocator-api2 v0.2.21
│   │       │   │   └── indexmap feature "std"
│   │       │   │       └── indexmap v2.11.1 (*)
│   │       │   ├── indexmap feature "std" (*)
│   │       │   ├── toml_write feature "default"
│   │       │   │   ├── toml_write v0.1.2
│   │       │   │   └── toml_write feature "std"
│   │       │   │       ├── toml_write v0.1.2
│   │       │   │       └── toml_write feature "alloc"
│   │       │   │           └── toml_write v0.1.2
│   │       │   └── winnow feature "default"
│   │       │       ├── winnow v0.7.13
│   │       │       └── winnow feature "std"
│   │       │           ├── winnow v0.7.13
│   │       │           └── winnow feature "alloc"
│   │       │               └── winnow v0.7.13
│   │       └── toml_datetime feature "serde" (*)
│   ├── toml feature "display"
│   │   ├── toml v0.8.23 (*)
│   │   └── toml_edit feature "display"
│   │       └── toml_edit v0.22.27 (*)
│   └── toml feature "parse"
│       ├── toml v0.8.23 (*)
│       └── toml_edit feature "parse"
│           └── toml_edit v0.22.27 (*)
├── uuid feature "default"
│   ├── uuid v1.18.1
│   │   ├── serde v1.0.221 (*)
│   │   └── getrandom feature "default"
│   │       └── getrandom v0.3.3
│   │           ├── libc v0.2.175
│   │           └── cfg-if feature "default" (*)
│   └── uuid feature "std"
│       └── uuid v1.18.1 (*)
├── uuid feature "serde"
│   └── uuid v1.18.1 (*)
├── uuid feature "v4"
│   ├── uuid v1.18.1 (*)
│   └── uuid feature "rng"
│       └── uuid v1.18.1 (*)
└── workspace-hack feature "default"
    └── workspace-hack v0.1.0 (/Users/mymac/Desktop/RustyOnions/workspace-hack)
        ├── num-traits feature "std"
        │   └── num-traits v0.2.19 (*)
        ├── serde feature "alloc" (*)
        ├── serde feature "default" (*)
        ├── serde feature "derive"
        │   ├── serde v1.0.221 (*)
        │   └── serde feature "serde_derive"
        │       └── serde v1.0.221 (*)
        ├── serde_core feature "alloc" (*)
        ├── serde_core feature "result" (*)
        ├── serde_core feature "std" (*)
        ├── hashbrown feature "default"
        │   ├── hashbrown v0.15.5 (*)
        │   ├── hashbrown feature "allocator-api2"
        │   │   └── hashbrown v0.15.5 (*)
        │   ├── hashbrown feature "default-hasher"
        │   │   └── hashbrown v0.15.5 (*)
        │   ├── hashbrown feature "equivalent"
        │   │   └── hashbrown v0.15.5 (*)
        │   ├── hashbrown feature "inline-more"
        │   │   └── hashbrown v0.15.5 (*)
        │   └── hashbrown feature "raw-entry"
        │       └── hashbrown v0.15.5 (*)
        ├── axum feature "http1"
        │   ├── axum v0.7.9
        │   │   ├── mime feature "default" (*)
        │   │   ├── serde feature "default" (*)
        │   │   ├── async-trait feature "default"
        │   │   │   └── async-trait v0.1.89 (proc-macro)
        │   │   │       ├── proc-macro2 feature "default" (*)
        │   │   │       ├── quote feature "default" (*)
        │   │   │       ├── syn feature "clone-impls" (*)
        │   │   │       ├── syn feature "full" (*)
        │   │   │       ├── syn feature "parsing" (*)
        │   │   │       ├── syn feature "printing" (*)
        │   │   │       ├── syn feature "proc-macro" (*)
        │   │   │       └── syn feature "visit-mut"
        │   │   │           └── syn v2.0.106 (*)
        │   │   ├── axum-core feature "default"
... (truncated)
```

</details>

### tldctl

<details><summary>Reverse tree (-i tldctl -e features)</summary>

```text
```

</details>

<details><summary>Forward tree (-p tldctl -e features)</summary>

```text
tldctl v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/tldctl)
├── anyhow feature "default"
│   ├── anyhow v1.0.99
│   └── anyhow feature "std"
│       └── anyhow v1.0.99
├── blake3 feature "default"
│   ├── blake3 v1.8.2
│   │   ├── arrayvec v0.7.6
│   │   ├── constant_time_eq v0.3.1
│   │   ├── arrayref feature "default"
│   │   │   └── arrayref v0.3.9
│   │   └── cfg-if feature "default"
│   │       └── cfg-if v1.0.3
│   │   [build-dependencies]
│   │   └── cc feature "default"
│   │       └── cc v1.2.37
│   │           ├── jobserver v0.1.34
│   │           │   └── libc feature "default"
│   │           │       ├── libc v0.2.175
│   │           │       └── libc feature "std"
│   │           │           └── libc v0.2.175
│   │           ├── libc v0.2.175
│   │           ├── find-msvc-tools feature "default"
│   │           │   └── find-msvc-tools v0.1.1
│   │           └── shlex feature "default"
│   │               ├── shlex v1.3.0
│   │               └── shlex feature "std"
│   │                   └── shlex v1.3.0
│   └── blake3 feature "std"
│       └── blake3 v1.8.2 (*)
├── brotli feature "default"
│   ├── brotli v8.0.2
│   │   ├── brotli-decompressor v5.0.0
│   │   │   ├── alloc-no-stdlib feature "default"
│   │   │   │   └── alloc-no-stdlib v2.0.4
│   │   │   └── alloc-stdlib feature "default"
│   │   │       └── alloc-stdlib v0.2.2
│   │   │           └── alloc-no-stdlib feature "default" (*)
│   │   ├── alloc-no-stdlib feature "default" (*)
│   │   └── alloc-stdlib feature "default" (*)
│   └── brotli feature "std"
│       ├── brotli v8.0.2 (*)
│       ├── brotli feature "alloc-stdlib"
│       │   └── brotli v8.0.2 (*)
│       └── brotli-decompressor feature "std"
│           ├── brotli-decompressor v5.0.0 (*)
│           └── brotli-decompressor feature "alloc-stdlib"
│               └── brotli-decompressor v5.0.0 (*)
├── clap feature "default"
│   ├── clap v4.5.47
│   │   ├── clap_builder v4.5.47
│   │   │   ├── anstream feature "default"
│   │   │   │   ├── anstream v0.6.20
│   │   │   │   │   ├── anstyle feature "default"
│   │   │   │   │   │   ├── anstyle v1.0.11
│   │   │   │   │   │   └── anstyle feature "std"
│   │   │   │   │   │       └── anstyle v1.0.11
│   │   │   │   │   ├── anstyle-parse feature "default"
│   │   │   │   │   │   ├── anstyle-parse v0.2.7
│   │   │   │   │   │   │   └── utf8parse feature "default"
│   │   │   │   │   │   │       └── utf8parse v0.2.2
│   │   │   │   │   │   └── anstyle-parse feature "utf8"
│   │   │   │   │   │       └── anstyle-parse v0.2.7 (*)
│   │   │   │   │   ├── utf8parse feature "default" (*)
│   │   │   │   │   ├── anstyle-query feature "default"
│   │   │   │   │   │   └── anstyle-query v1.1.4
│   │   │   │   │   ├── colorchoice feature "default"
│   │   │   │   │   │   └── colorchoice v1.0.4
│   │   │   │   │   └── is_terminal_polyfill feature "default"
│   │   │   │   │       └── is_terminal_polyfill v1.70.1
│   │   │   │   ├── anstream feature "auto"
│   │   │   │   │   └── anstream v0.6.20 (*)
│   │   │   │   └── anstream feature "wincon"
│   │   │   │       └── anstream v0.6.20 (*)
│   │   │   ├── anstyle feature "default" (*)
│   │   │   ├── clap_lex feature "default"
│   │   │   │   └── clap_lex v0.7.5
│   │   │   └── strsim feature "default"
│   │   │       └── strsim v0.11.1
│   │   └── clap_derive feature "default"
│   │       └── clap_derive v4.5.47 (proc-macro)
│   │           ├── heck feature "default"
│   │           │   └── heck v0.5.0
│   │           ├── proc-macro2 feature "default"
│   │           │   ├── proc-macro2 v1.0.101
│   │           │   │   └── unicode-ident feature "default"
│   │           │   │       └── unicode-ident v1.0.19
│   │           │   └── proc-macro2 feature "proc-macro"
│   │           │       └── proc-macro2 v1.0.101 (*)
│   │           ├── quote feature "default"
│   │           │   ├── quote v1.0.40
│   │           │   │   └── proc-macro2 v1.0.101 (*)
│   │           │   └── quote feature "proc-macro"
│   │           │       ├── quote v1.0.40 (*)
│   │           │       └── proc-macro2 feature "proc-macro" (*)
│   │           ├── syn feature "default"
│   │           │   ├── syn v2.0.106
│   │           │   │   ├── proc-macro2 v1.0.101 (*)
│   │           │   │   ├── quote v1.0.40 (*)
│   │           │   │   └── unicode-ident feature "default" (*)
│   │           │   ├── syn feature "clone-impls"
│   │           │   │   └── syn v2.0.106 (*)
│   │           │   ├── syn feature "derive"
│   │           │   │   └── syn v2.0.106 (*)
│   │           │   ├── syn feature "parsing"
│   │           │   │   └── syn v2.0.106 (*)
│   │           │   ├── syn feature "printing"
│   │           │   │   └── syn v2.0.106 (*)
│   │           │   └── syn feature "proc-macro"
│   │           │       ├── syn v2.0.106 (*)
│   │           │       ├── proc-macro2 feature "proc-macro" (*)
│   │           │       └── quote feature "proc-macro" (*)
│   │           └── syn feature "full"
│   │               └── syn v2.0.106 (*)
│   ├── clap feature "color"
│   │   ├── clap v4.5.47 (*)
│   │   └── clap_builder feature "color"
│   │       └── clap_builder v4.5.47 (*)
│   ├── clap feature "error-context"
│   │   ├── clap v4.5.47 (*)
│   │   └── clap_builder feature "error-context"
│   │       └── clap_builder v4.5.47 (*)
│   ├── clap feature "help"
│   │   ├── clap v4.5.47 (*)
│   │   └── clap_builder feature "help"
│   │       └── clap_builder v4.5.47 (*)
│   ├── clap feature "std"
│   │   ├── clap v4.5.47 (*)
│   │   └── clap_builder feature "std"
│   │       ├── clap_builder v4.5.47 (*)
│   │       └── anstyle feature "std" (*)
│   ├── clap feature "suggestions"
│   │   ├── clap v4.5.47 (*)
│   │   └── clap_builder feature "suggestions"
│   │       ├── clap_builder v4.5.47 (*)
│   │       └── clap_builder feature "error-context" (*)
│   └── clap feature "usage"
│       ├── clap v4.5.47 (*)
│       └── clap_builder feature "usage"
│           └── clap_builder v4.5.47 (*)
├── index feature "default"
│   └── index v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/index)
│       ├── anyhow feature "default" (*)
│       ├── bincode feature "default"
│       │   └── bincode v1.3.3
│       │       └── serde feature "default"
│       │           ├── serde v1.0.221
│       │           │   ├── serde_core feature "result"
│       │           │   │   └── serde_core v1.0.221
│       │           │   └── serde_derive feature "default"
│       │           │       └── serde_derive v1.0.221 (proc-macro)
│       │           │           ├── proc-macro2 feature "proc-macro" (*)
│       │           │           ├── quote feature "proc-macro" (*)
│       │           │           ├── syn feature "clone-impls" (*)
│       │           │           ├── syn feature "derive" (*)
│       │           │           ├── syn feature "parsing" (*)
│       │           │           ├── syn feature "printing" (*)
│       │           │           └── syn feature "proc-macro" (*)
│       │           └── serde feature "std"
│       │               ├── serde v1.0.221 (*)
│       │               └── serde_core feature "std"
│       │                   └── serde_core v1.0.221
│       ├── serde feature "default" (*)
│       ├── chrono feature "default"
│       │   ├── chrono v0.4.42
│       │   │   ├── num-traits v0.2.19
│       │   │   │   [build-dependencies]
│       │   │   │   └── autocfg feature "default"
│       │   │   │       └── autocfg v1.5.0
│       │   │   ├── iana-time-zone feature "default"
│       │   │   │   └── iana-time-zone v0.1.64
│       │   │   │       └── core-foundation-sys feature "default"
│       │   │   │           ├── core-foundation-sys v0.8.7
│       │   │   │           └── core-foundation-sys feature "link"
│       │   │   │               └── core-foundation-sys v0.8.7
│       │   │   └── iana-time-zone feature "fallback"
│       │   │       └── iana-time-zone v0.1.64 (*)
│       │   ├── chrono feature "clock"
│       │   │   ├── chrono v0.4.42 (*)
│       │   │   ├── chrono feature "iana-time-zone"
│       │   │   │   └── chrono v0.4.42 (*)
│       │   │   ├── chrono feature "now"
│       │   │   │   ├── chrono v0.4.42 (*)
│       │   │   │   └── chrono feature "std"
│       │   │   │       ├── chrono v0.4.42 (*)
│       │   │   │       └── chrono feature "alloc"
│       │   │   │           └── chrono v0.4.42 (*)
│       │   │   └── chrono feature "winapi"
│       │   │       ├── chrono v0.4.42 (*)
│       │   │       └── chrono feature "windows-link"
│       │   │           └── chrono v0.4.42 (*)
│       │   ├── chrono feature "oldtime"
│       │   │   └── chrono v0.4.42 (*)
│       │   ├── chrono feature "std" (*)
│       │   └── chrono feature "wasmbind"
│       │       ├── chrono v0.4.42 (*)
│       │       ├── chrono feature "js-sys"
│       │       │   └── chrono v0.4.42 (*)
│       │       └── chrono feature "wasm-bindgen"
│       │           └── chrono v0.4.42 (*)
│       ├── dunce feature "default"
│       │   └── dunce v1.0.5
│       ├── naming feature "default"
│       │   └── naming v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/naming)
│       │       ├── anyhow feature "default" (*)
│       │       ├── blake3 feature "default" (*)
│       │       ├── serde feature "default" (*)
│       │       ├── chrono feature "default" (*)
│       │       ├── base64 feature "default"
│       │       │   ├── base64 v0.22.1
│       │       │   └── base64 feature "std"
│       │       │       ├── base64 v0.22.1
│       │       │       └── base64 feature "alloc"
│       │       │           └── base64 v0.22.1
│       │       ├── hex feature "default"
│       │       │   ├── hex v0.4.3
│       │       │   └── hex feature "std"
│       │       │       ├── hex v0.4.3
│       │       │       └── hex feature "alloc"
│       │       │           └── hex v0.4.3
│       │       ├── mime feature "default"
│       │       │   └── mime v0.3.17
│       │       ├── mime_guess feature "default"
│       │       │   ├── mime_guess v2.0.5
│       │       │   │   ├── mime feature "default" (*)
│       │       │   │   └── unicase feature "default"
│       │       │   │       └── unicase v2.8.1
│       │       │   │   [build-dependencies]
│       │       │   │   └── unicase feature "default" (*)
│       │       │   └── mime_guess feature "rev-mappings"
│       │       │       └── mime_guess v2.0.5 (*)
│       │       ├── serde_with feature "default"
│       │       │   ├── serde_with v3.14.0
│       │       │   │   ├── serde v1.0.221 (*)
│       │       │   │   ├── serde_derive feature "default" (*)
│       │       │   │   └── serde_with_macros feature "default"
│       │       │   │       └── serde_with_macros v3.14.0 (proc-macro)
│       │       │   │           ├── proc-macro2 feature "default" (*)
│       │       │   │           ├── quote feature "default" (*)
│       │       │   │           ├── syn feature "default" (*)
│       │       │   │           ├── syn feature "extra-traits"
│       │       │   │           │   └── syn v2.0.106 (*)
│       │       │   │           ├── syn feature "full" (*)
│       │       │   │           ├── syn feature "parsing" (*)
│       │       │   │           └── darling feature "default"
│       │       │   │               ├── darling v0.20.11
│       │       │   │               │   ├── darling_core feature "default"
│       │       │   │               │   │   └── darling_core v0.20.11
│       │       │   │               │   │       ├── strsim feature "default" (*)
│       │       │   │               │   │       ├── proc-macro2 feature "default" (*)
│       │       │   │               │   │       ├── quote feature "default" (*)
│       │       │   │               │   │       ├── syn feature "default" (*)
│       │       │   │               │   │       ├── syn feature "extra-traits" (*)
│       │       │   │               │   │       ├── syn feature "full" (*)
│       │       │   │               │   │       ├── fnv feature "default"
│       │       │   │               │   │       │   ├── fnv v1.0.7
│       │       │   │               │   │       │   └── fnv feature "std"
│       │       │   │               │   │       │       └── fnv v1.0.7
│       │       │   │               │   │       └── ident_case feature "default"
│       │       │   │               │   │           └── ident_case v1.0.1
│       │       │   │               │   └── darling_macro feature "default"
│       │       │   │               │       └── darling_macro v0.20.11 (proc-macro)
│       │       │   │               │           ├── quote feature "default" (*)
│       │       │   │               │           ├── syn feature "default" (*)
│       │       │   │               │           └── darling_core feature "default" (*)
│       │       │   │               └── darling feature "suggestions"
│       │       │   │                   ├── darling v0.20.11 (*)
│       │       │   │                   └── darling_core feature "suggestions"
│       │       │   │                       ├── darling_core v0.20.11 (*)
│       │       │   │                       └── darling_core feature "strsim"
│       │       │   │                           └── darling_core v0.20.11 (*)
│       │       │   ├── serde_with feature "macros"
│       │       │   │   └── serde_with v3.14.0 (*)
│       │       │   └── serde_with feature "std"
│       │       │       ├── serde_with v3.14.0 (*)
│       │       │       ├── serde feature "std" (*)
│       │       │       └── serde_with feature "alloc"
│       │       │           ├── serde_with v3.14.0 (*)
│       │       │           └── serde feature "alloc"
│       │       │               ├── serde v1.0.221 (*)
│       │       │               └── serde_core feature "alloc"
│       │       │                   └── serde_core v1.0.221
│       │       ├── thiserror feature "default"
│       │       │   └── thiserror v1.0.69
│       │       │       └── thiserror-impl feature "default"
│       │       │           └── thiserror-impl v1.0.69 (proc-macro)
│       │       │               ├── proc-macro2 feature "default" (*)
│       │       │               ├── quote feature "default" (*)
│       │       │               └── syn feature "default" (*)
│       │       ├── toml feature "default"
│       │       │   ├── toml v0.8.23
│       │       │   │   ├── serde feature "default" (*)
│       │       │   │   ├── serde_spanned feature "default"
│       │       │   │   │   └── serde_spanned v0.6.9
│       │       │   │   │       └── serde feature "default" (*)
│       │       │   │   ├── serde_spanned feature "serde"
│       │       │   │   │   └── serde_spanned v0.6.9 (*)
│       │       │   │   ├── toml_datetime feature "default"
│       │       │   │   │   └── toml_datetime v0.6.11
│       │       │   │   │       └── serde feature "default" (*)
... (truncated)
```

</details>

### index

<details><summary>Reverse tree (-i index -e features)</summary>

```text
```

</details>

<details><summary>Forward tree (-p index -e features)</summary>

```text
index v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/index)
├── anyhow feature "default"
│   ├── anyhow v1.0.99
│   └── anyhow feature "std"
│       └── anyhow v1.0.99
├── bincode feature "default"
│   └── bincode v1.3.3
│       └── serde feature "default"
│           ├── serde v1.0.221
│           │   ├── serde_core feature "result"
│           │   │   └── serde_core v1.0.221
│           │   └── serde_derive feature "default"
│           │       └── serde_derive v1.0.221 (proc-macro)
│           │           ├── proc-macro2 feature "proc-macro"
│           │           │   └── proc-macro2 v1.0.101
│           │           │       └── unicode-ident feature "default"
│           │           │           └── unicode-ident v1.0.19
│           │           ├── quote feature "proc-macro"
│           │           │   ├── quote v1.0.40
│           │           │   │   └── proc-macro2 v1.0.101 (*)
│           │           │   └── proc-macro2 feature "proc-macro" (*)
│           │           ├── syn feature "clone-impls"
│           │           │   └── syn v2.0.106
│           │           │       ├── proc-macro2 v1.0.101 (*)
│           │           │       ├── quote v1.0.40 (*)
│           │           │       └── unicode-ident feature "default" (*)
│           │           ├── syn feature "derive"
│           │           │   └── syn v2.0.106 (*)
│           │           ├── syn feature "parsing"
│           │           │   └── syn v2.0.106 (*)
│           │           ├── syn feature "printing"
│           │           │   └── syn v2.0.106 (*)
│           │           └── syn feature "proc-macro"
│           │               ├── syn v2.0.106 (*)
│           │               ├── proc-macro2 feature "proc-macro" (*)
│           │               └── quote feature "proc-macro" (*)
│           └── serde feature "std"
│               ├── serde v1.0.221 (*)
│               └── serde_core feature "std"
│                   └── serde_core v1.0.221
├── serde feature "default" (*)
├── chrono feature "default"
│   ├── chrono v0.4.42
│   │   ├── num-traits v0.2.19
│   │   │   [build-dependencies]
│   │   │   └── autocfg feature "default"
│   │   │       └── autocfg v1.5.0
│   │   ├── iana-time-zone feature "default"
│   │   │   └── iana-time-zone v0.1.64
│   │   │       └── core-foundation-sys feature "default"
│   │   │           ├── core-foundation-sys v0.8.7
│   │   │           └── core-foundation-sys feature "link"
│   │   │               └── core-foundation-sys v0.8.7
│   │   └── iana-time-zone feature "fallback"
│   │       └── iana-time-zone v0.1.64 (*)
│   ├── chrono feature "clock"
│   │   ├── chrono v0.4.42 (*)
│   │   ├── chrono feature "iana-time-zone"
│   │   │   └── chrono v0.4.42 (*)
│   │   ├── chrono feature "now"
│   │   │   ├── chrono v0.4.42 (*)
│   │   │   └── chrono feature "std"
│   │   │       ├── chrono v0.4.42 (*)
│   │   │       └── chrono feature "alloc"
│   │   │           └── chrono v0.4.42 (*)
│   │   └── chrono feature "winapi"
│   │       ├── chrono v0.4.42 (*)
│   │       └── chrono feature "windows-link"
│   │           └── chrono v0.4.42 (*)
│   ├── chrono feature "oldtime"
│   │   └── chrono v0.4.42 (*)
│   ├── chrono feature "std" (*)
│   └── chrono feature "wasmbind"
│       ├── chrono v0.4.42 (*)
│       ├── chrono feature "js-sys"
│       │   └── chrono v0.4.42 (*)
│       └── chrono feature "wasm-bindgen"
│           └── chrono v0.4.42 (*)
├── dunce feature "default"
│   └── dunce v1.0.5
├── naming feature "default"
│   └── naming v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/naming)
│       ├── anyhow feature "default" (*)
│       ├── serde feature "default" (*)
│       ├── chrono feature "default" (*)
│       ├── base64 feature "default"
│       │   ├── base64 v0.22.1
│       │   └── base64 feature "std"
│       │       ├── base64 v0.22.1
│       │       └── base64 feature "alloc"
│       │           └── base64 v0.22.1
│       ├── blake3 feature "default"
│       │   ├── blake3 v1.8.2
│       │   │   ├── arrayvec v0.7.6
│       │   │   ├── constant_time_eq v0.3.1
│       │   │   ├── arrayref feature "default"
│       │   │   │   └── arrayref v0.3.9
│       │   │   └── cfg-if feature "default"
│       │   │       └── cfg-if v1.0.3
│       │   │   [build-dependencies]
│       │   │   └── cc feature "default"
│       │   │       └── cc v1.2.37
│       │   │           ├── jobserver v0.1.34
│       │   │           │   └── libc feature "default"
│       │   │           │       ├── libc v0.2.175
│       │   │           │       └── libc feature "std"
│       │   │           │           └── libc v0.2.175
│       │   │           ├── libc v0.2.175
│       │   │           ├── find-msvc-tools feature "default"
│       │   │           │   └── find-msvc-tools v0.1.1
│       │   │           └── shlex feature "default"
│       │   │               ├── shlex v1.3.0
│       │   │               └── shlex feature "std"
│       │   │                   └── shlex v1.3.0
│       │   └── blake3 feature "std"
│       │       └── blake3 v1.8.2 (*)
│       ├── hex feature "default"
│       │   ├── hex v0.4.3
│       │   └── hex feature "std"
│       │       ├── hex v0.4.3
│       │       └── hex feature "alloc"
│       │           └── hex v0.4.3
│       ├── mime feature "default"
│       │   └── mime v0.3.17
│       ├── mime_guess feature "default"
│       │   ├── mime_guess v2.0.5
│       │   │   ├── mime feature "default" (*)
│       │   │   └── unicase feature "default"
│       │   │       └── unicase v2.8.1
│       │   │   [build-dependencies]
│       │   │   └── unicase feature "default" (*)
│       │   └── mime_guess feature "rev-mappings"
│       │       └── mime_guess v2.0.5 (*)
│       ├── serde_with feature "default"
│       │   ├── serde_with v3.14.0
│       │   │   ├── serde v1.0.221 (*)
│       │   │   ├── serde_derive feature "default" (*)
│       │   │   └── serde_with_macros feature "default"
│       │   │       └── serde_with_macros v3.14.0 (proc-macro)
│       │   │           ├── proc-macro2 feature "default"
│       │   │           │   ├── proc-macro2 v1.0.101 (*)
│       │   │           │   └── proc-macro2 feature "proc-macro" (*)
│       │   │           ├── quote feature "default"
│       │   │           │   ├── quote v1.0.40 (*)
│       │   │           │   └── quote feature "proc-macro" (*)
│       │   │           ├── syn feature "default"
│       │   │           │   ├── syn v2.0.106 (*)
│       │   │           │   ├── syn feature "clone-impls" (*)
│       │   │           │   ├── syn feature "derive" (*)
│       │   │           │   ├── syn feature "parsing" (*)
│       │   │           │   ├── syn feature "printing" (*)
│       │   │           │   └── syn feature "proc-macro" (*)
│       │   │           ├── syn feature "extra-traits"
│       │   │           │   └── syn v2.0.106 (*)
│       │   │           ├── syn feature "full"
│       │   │           │   └── syn v2.0.106 (*)
│       │   │           ├── syn feature "parsing" (*)
│       │   │           └── darling feature "default"
│       │   │               ├── darling v0.20.11
│       │   │               │   ├── darling_core feature "default"
│       │   │               │   │   └── darling_core v0.20.11
│       │   │               │   │       ├── proc-macro2 feature "default" (*)
│       │   │               │   │       ├── quote feature "default" (*)
│       │   │               │   │       ├── syn feature "default" (*)
│       │   │               │   │       ├── syn feature "extra-traits" (*)
│       │   │               │   │       ├── syn feature "full" (*)
│       │   │               │   │       ├── fnv feature "default"
│       │   │               │   │       │   ├── fnv v1.0.7
│       │   │               │   │       │   └── fnv feature "std"
│       │   │               │   │       │       └── fnv v1.0.7
│       │   │               │   │       ├── ident_case feature "default"
│       │   │               │   │       │   └── ident_case v1.0.1
│       │   │               │   │       └── strsim feature "default"
│       │   │               │   │           └── strsim v0.11.1
│       │   │               │   └── darling_macro feature "default"
│       │   │               │       └── darling_macro v0.20.11 (proc-macro)
│       │   │               │           ├── quote feature "default" (*)
│       │   │               │           ├── syn feature "default" (*)
│       │   │               │           └── darling_core feature "default" (*)
│       │   │               └── darling feature "suggestions"
│       │   │                   ├── darling v0.20.11 (*)
│       │   │                   └── darling_core feature "suggestions"
│       │   │                       ├── darling_core v0.20.11 (*)
│       │   │                       └── darling_core feature "strsim"
│       │   │                           └── darling_core v0.20.11 (*)
│       │   ├── serde_with feature "macros"
│       │   │   └── serde_with v3.14.0 (*)
│       │   └── serde_with feature "std"
│       │       ├── serde_with v3.14.0 (*)
│       │       ├── serde feature "std" (*)
│       │       └── serde_with feature "alloc"
│       │           ├── serde_with v3.14.0 (*)
│       │           └── serde feature "alloc"
│       │               ├── serde v1.0.221 (*)
│       │               └── serde_core feature "alloc"
│       │                   └── serde_core v1.0.221
│       ├── thiserror feature "default"
│       │   └── thiserror v1.0.69
│       │       └── thiserror-impl feature "default"
│       │           └── thiserror-impl v1.0.69 (proc-macro)
│       │               ├── proc-macro2 feature "default" (*)
│       │               ├── quote feature "default" (*)
│       │               └── syn feature "default" (*)
│       ├── toml feature "default"
│       │   ├── toml v0.8.23
│       │   │   ├── serde feature "default" (*)
│       │   │   ├── serde_spanned feature "default"
│       │   │   │   └── serde_spanned v0.6.9
│       │   │   │       └── serde feature "default" (*)
│       │   │   ├── serde_spanned feature "serde"
│       │   │   │   └── serde_spanned v0.6.9 (*)
│       │   │   ├── toml_datetime feature "default"
│       │   │   │   └── toml_datetime v0.6.11
│       │   │   │       └── serde feature "default" (*)
│       │   │   ├── toml_datetime feature "serde"
│       │   │   │   └── toml_datetime v0.6.11 (*)
│       │   │   └── toml_edit feature "serde"
│       │   │       ├── toml_edit v0.22.27
│       │   │       │   ├── serde feature "default" (*)
│       │   │       │   ├── serde_spanned feature "default" (*)
│       │   │       │   ├── serde_spanned feature "serde" (*)
│       │   │       │   ├── toml_datetime feature "default" (*)
│       │   │       │   ├── indexmap feature "default"
│       │   │       │   │   ├── indexmap v2.11.1
│       │   │       │   │   │   ├── equivalent v1.0.2
│       │   │       │   │   │   └── hashbrown v0.15.5
│       │   │       │   │   │       ├── equivalent v1.0.2
│       │   │       │   │   │       ├── foldhash v0.1.5
│       │   │       │   │   │       └── allocator-api2 feature "alloc"
│       │   │       │   │   │           └── allocator-api2 v0.2.21
│       │   │       │   │   └── indexmap feature "std"
│       │   │       │   │       └── indexmap v2.11.1 (*)
│       │   │       │   ├── indexmap feature "std" (*)
│       │   │       │   ├── toml_write feature "default"
│       │   │       │   │   ├── toml_write v0.1.2
│       │   │       │   │   └── toml_write feature "std"
│       │   │       │   │       ├── toml_write v0.1.2
│       │   │       │   │       └── toml_write feature "alloc"
│       │   │       │   │           └── toml_write v0.1.2
│       │   │       │   └── winnow feature "default"
│       │   │       │       ├── winnow v0.7.13
│       │   │       │       └── winnow feature "std"
│       │   │       │           ├── winnow v0.7.13
│       │   │       │           └── winnow feature "alloc"
│       │   │       │               └── winnow v0.7.13
│       │   │       └── toml_datetime feature "serde" (*)
│       │   ├── toml feature "display"
│       │   │   ├── toml v0.8.23 (*)
│       │   │   └── toml_edit feature "display"
│       │   │       └── toml_edit v0.22.27 (*)
│       │   └── toml feature "parse"
│       │       ├── toml v0.8.23 (*)
│       │       └── toml_edit feature "parse"
│       │           └── toml_edit v0.22.27 (*)
│       ├── uuid feature "default"
│       │   ├── uuid v1.18.1
│       │   │   ├── serde v1.0.221 (*)
│       │   │   └── getrandom feature "default"
│       │   │       └── getrandom v0.3.3
│       │   │           ├── libc v0.2.175
│       │   │           └── cfg-if feature "default" (*)
│       │   └── uuid feature "std"
│       │       └── uuid v1.18.1 (*)
│       ├── uuid feature "serde"
│       │   └── uuid v1.18.1 (*)
│       ├── uuid feature "v4"
│       │   ├── uuid v1.18.1 (*)
│       │   └── uuid feature "rng"
│       │       └── uuid v1.18.1 (*)
│       └── workspace-hack feature "default"
│           └── workspace-hack v0.1.0 (/Users/mymac/Desktop/RustyOnions/workspace-hack)
│               ├── serde feature "alloc" (*)
│               ├── serde feature "default" (*)
│               ├── serde feature "derive"
│               │   ├── serde v1.0.221 (*)
│               │   └── serde feature "serde_derive"
│               │       └── serde v1.0.221 (*)
│               ├── serde_core feature "alloc" (*)
│               ├── serde_core feature "result" (*)
│               ├── serde_core feature "std" (*)
│               ├── num-traits feature "std"
│               │   └── num-traits v0.2.19 (*)
│               ├── hashbrown feature "default"
│               │   ├── hashbrown v0.15.5 (*)
│               │   ├── hashbrown feature "allocator-api2"
│               │   │   └── hashbrown v0.15.5 (*)
│               │   ├── hashbrown feature "default-hasher"
│               │   │   └── hashbrown v0.15.5 (*)
│               │   ├── hashbrown feature "equivalent"
│               │   │   └── hashbrown v0.15.5 (*)
│               │   ├── hashbrown feature "inline-more"
│               │   │   └── hashbrown v0.15.5 (*)
│               │   └── hashbrown feature "raw-entry"
│               │       └── hashbrown v0.15.5 (*)
│               ├── axum feature "http1"
│               │   ├── axum v0.7.9
│               │   │   ├── serde feature "default" (*)
│               │   │   ├── mime feature "default" (*)
│               │   │   ├── async-trait feature "default"
│               │   │   │   └── async-trait v0.1.89 (proc-macro)
... (truncated)
```

</details>

### ryker

<details><summary>Reverse tree (-i ryker -e features)</summary>

```text
```

</details>

<details><summary>Forward tree (-p ryker -e features)</summary>

```text
ryker v0.2.0 (/Users/mymac/Desktop/RustyOnions/crates/ryker)
├── rand feature "default"
│   ├── rand v0.8.5
│   │   ├── libc v0.2.175
│   │   ├── rand_chacha v0.3.1
│   │   │   ├── ppv-lite86 feature "simd"
│   │   │   │   └── ppv-lite86 v0.2.21
│   │   │   │       ├── zerocopy feature "default"
│   │   │   │       │   └── zerocopy v0.8.27
│   │   │   │       └── zerocopy feature "simd"
│   │   │   │           └── zerocopy v0.8.27
│   │   │   └── rand_core feature "default"
│   │   │       └── rand_core v0.6.4
│   │   │           └── getrandom feature "default"
│   │   │               └── getrandom v0.2.16
│   │   │                   ├── libc v0.2.175
│   │   │                   └── cfg-if feature "default"
│   │   │                       └── cfg-if v1.0.3
│   │   └── rand_core feature "default" (*)
│   ├── rand feature "std"
│   │   ├── rand v0.8.5 (*)
│   │   ├── rand feature "alloc"
│   │   │   ├── rand v0.8.5 (*)
│   │   │   └── rand_core feature "alloc"
│   │   │       └── rand_core v0.6.4 (*)
│   │   ├── rand feature "getrandom"
│   │   │   ├── rand v0.8.5 (*)
│   │   │   └── rand_core feature "getrandom"
│   │   │       └── rand_core v0.6.4 (*)
│   │   ├── rand feature "libc"
│   │   │   └── rand v0.8.5 (*)
│   │   ├── rand feature "rand_chacha"
│   │   │   └── rand v0.8.5 (*)
│   │   ├── rand_chacha feature "std"
│   │   │   ├── rand_chacha v0.3.1 (*)
│   │   │   └── ppv-lite86 feature "std"
│   │   │       └── ppv-lite86 v0.2.21 (*)
│   │   └── rand_core feature "std"
│   │       ├── rand_core v0.6.4 (*)
│   │       ├── rand_core feature "alloc" (*)
│   │       ├── rand_core feature "getrandom" (*)
│   │       └── getrandom feature "std"
│   │           └── getrandom v0.2.16 (*)
│   └── rand feature "std_rng"
│       ├── rand v0.8.5 (*)
│       └── rand feature "rand_chacha" (*)
├── ron-billing feature "default"
│   └── ron-billing v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/ron-billing)
│       ├── anyhow feature "default"
│       │   ├── anyhow v1.0.99
│       │   └── anyhow feature "std"
│       │       └── anyhow v1.0.99
│       └── naming feature "default"
│           └── naming v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/naming)
│               ├── anyhow feature "default" (*)
│               ├── base64 feature "default"
│               │   ├── base64 v0.22.1
│               │   └── base64 feature "std"
│               │       ├── base64 v0.22.1
│               │       └── base64 feature "alloc"
│               │           └── base64 v0.22.1
│               ├── blake3 feature "default"
│               │   ├── blake3 v1.8.2
│               │   │   ├── arrayvec v0.7.6
│               │   │   ├── constant_time_eq v0.3.1
│               │   │   ├── cfg-if feature "default" (*)
│               │   │   └── arrayref feature "default"
│               │   │       └── arrayref v0.3.9
│               │   │   [build-dependencies]
│               │   │   └── cc feature "default"
│               │   │       └── cc v1.2.37
│               │   │           ├── jobserver v0.1.34
│               │   │           │   └── libc feature "default"
│               │   │           │       ├── libc v0.2.175
│               │   │           │       └── libc feature "std"
│               │   │           │           └── libc v0.2.175
│               │   │           ├── libc v0.2.175
│               │   │           ├── find-msvc-tools feature "default"
│               │   │           │   └── find-msvc-tools v0.1.1
│               │   │           └── shlex feature "default"
│               │   │               ├── shlex v1.3.0
│               │   │               └── shlex feature "std"
│               │   │                   └── shlex v1.3.0
│               │   └── blake3 feature "std"
│               │       └── blake3 v1.8.2 (*)
│               ├── chrono feature "default"
│               │   ├── chrono v0.4.42
│               │   │   ├── num-traits v0.2.19
│               │   │   │   [build-dependencies]
│               │   │   │   └── autocfg feature "default"
│               │   │   │       └── autocfg v1.5.0
│               │   │   ├── iana-time-zone feature "default"
│               │   │   │   └── iana-time-zone v0.1.64
│               │   │   │       └── core-foundation-sys feature "default"
│               │   │   │           ├── core-foundation-sys v0.8.7
│               │   │   │           └── core-foundation-sys feature "link"
│               │   │   │               └── core-foundation-sys v0.8.7
│               │   │   └── iana-time-zone feature "fallback"
│               │   │       └── iana-time-zone v0.1.64 (*)
│               │   ├── chrono feature "clock"
│               │   │   ├── chrono v0.4.42 (*)
│               │   │   ├── chrono feature "iana-time-zone"
│               │   │   │   └── chrono v0.4.42 (*)
│               │   │   ├── chrono feature "now"
│               │   │   │   ├── chrono v0.4.42 (*)
│               │   │   │   └── chrono feature "std"
│               │   │   │       ├── chrono v0.4.42 (*)
│               │   │   │       └── chrono feature "alloc"
│               │   │   │           └── chrono v0.4.42 (*)
│               │   │   └── chrono feature "winapi"
│               │   │       ├── chrono v0.4.42 (*)
│               │   │       └── chrono feature "windows-link"
│               │   │           └── chrono v0.4.42 (*)
│               │   ├── chrono feature "oldtime"
│               │   │   └── chrono v0.4.42 (*)
│               │   ├── chrono feature "std" (*)
│               │   └── chrono feature "wasmbind"
│               │       ├── chrono v0.4.42 (*)
│               │       ├── chrono feature "js-sys"
│               │       │   └── chrono v0.4.42 (*)
│               │       └── chrono feature "wasm-bindgen"
│               │           └── chrono v0.4.42 (*)
│               ├── hex feature "default"
│               │   ├── hex v0.4.3
│               │   └── hex feature "std"
│               │       ├── hex v0.4.3
│               │       └── hex feature "alloc"
│               │           └── hex v0.4.3
│               ├── mime feature "default"
│               │   └── mime v0.3.17
│               ├── mime_guess feature "default"
│               │   ├── mime_guess v2.0.5
│               │   │   ├── mime feature "default" (*)
│               │   │   └── unicase feature "default"
│               │   │       └── unicase v2.8.1
│               │   │   [build-dependencies]
│               │   │   └── unicase feature "default" (*)
│               │   └── mime_guess feature "rev-mappings"
│               │       └── mime_guess v2.0.5 (*)
│               ├── serde feature "default"
│               │   ├── serde v1.0.221
│               │   │   ├── serde_core feature "result"
│               │   │   │   └── serde_core v1.0.221
│               │   │   └── serde_derive feature "default"
│               │   │       └── serde_derive v1.0.221 (proc-macro)
│               │   │           ├── proc-macro2 feature "proc-macro"
│               │   │           │   └── proc-macro2 v1.0.101
│               │   │           │       └── unicode-ident feature "default"
│               │   │           │           └── unicode-ident v1.0.19
│               │   │           ├── quote feature "proc-macro"
│               │   │           │   ├── quote v1.0.40
│               │   │           │   │   └── proc-macro2 v1.0.101 (*)
│               │   │           │   └── proc-macro2 feature "proc-macro" (*)
│               │   │           ├── syn feature "clone-impls"
│               │   │           │   └── syn v2.0.106
│               │   │           │       ├── proc-macro2 v1.0.101 (*)
│               │   │           │       ├── quote v1.0.40 (*)
│               │   │           │       └── unicode-ident feature "default" (*)
│               │   │           ├── syn feature "derive"
│               │   │           │   └── syn v2.0.106 (*)
│               │   │           ├── syn feature "parsing"
│               │   │           │   └── syn v2.0.106 (*)
│               │   │           ├── syn feature "printing"
│               │   │           │   └── syn v2.0.106 (*)
│               │   │           └── syn feature "proc-macro"
│               │   │               ├── syn v2.0.106 (*)
│               │   │               ├── proc-macro2 feature "proc-macro" (*)
│               │   │               └── quote feature "proc-macro" (*)
│               │   └── serde feature "std"
│               │       ├── serde v1.0.221 (*)
│               │       └── serde_core feature "std"
│               │           └── serde_core v1.0.221
│               ├── serde_with feature "default"
│               │   ├── serde_with v3.14.0
│               │   │   ├── serde v1.0.221 (*)
│               │   │   ├── serde_derive feature "default" (*)
│               │   │   └── serde_with_macros feature "default"
│               │   │       └── serde_with_macros v3.14.0 (proc-macro)
│               │   │           ├── proc-macro2 feature "default"
│               │   │           │   ├── proc-macro2 v1.0.101 (*)
│               │   │           │   └── proc-macro2 feature "proc-macro" (*)
│               │   │           ├── quote feature "default"
│               │   │           │   ├── quote v1.0.40 (*)
│               │   │           │   └── quote feature "proc-macro" (*)
│               │   │           ├── syn feature "default"
│               │   │           │   ├── syn v2.0.106 (*)
│               │   │           │   ├── syn feature "clone-impls" (*)
│               │   │           │   ├── syn feature "derive" (*)
│               │   │           │   ├── syn feature "parsing" (*)
│               │   │           │   ├── syn feature "printing" (*)
│               │   │           │   └── syn feature "proc-macro" (*)
│               │   │           ├── syn feature "extra-traits"
│               │   │           │   └── syn v2.0.106 (*)
│               │   │           ├── syn feature "full"
│               │   │           │   └── syn v2.0.106 (*)
│               │   │           ├── syn feature "parsing" (*)
│               │   │           └── darling feature "default"
│               │   │               ├── darling v0.20.11
│               │   │               │   ├── darling_core feature "default"
│               │   │               │   │   └── darling_core v0.20.11
│               │   │               │   │       ├── proc-macro2 feature "default" (*)
│               │   │               │   │       ├── quote feature "default" (*)
│               │   │               │   │       ├── syn feature "default" (*)
│               │   │               │   │       ├── syn feature "extra-traits" (*)
│               │   │               │   │       ├── syn feature "full" (*)
│               │   │               │   │       ├── fnv feature "default"
│               │   │               │   │       │   ├── fnv v1.0.7
│               │   │               │   │       │   └── fnv feature "std"
│               │   │               │   │       │       └── fnv v1.0.7
│               │   │               │   │       ├── ident_case feature "default"
│               │   │               │   │       │   └── ident_case v1.0.1
│               │   │               │   │       └── strsim feature "default"
│               │   │               │   │           └── strsim v0.11.1
│               │   │               │   └── darling_macro feature "default"
│               │   │               │       └── darling_macro v0.20.11 (proc-macro)
│               │   │               │           ├── quote feature "default" (*)
│               │   │               │           ├── syn feature "default" (*)
│               │   │               │           └── darling_core feature "default" (*)
│               │   │               └── darling feature "suggestions"
│               │   │                   ├── darling v0.20.11 (*)
│               │   │                   └── darling_core feature "suggestions"
│               │   │                       ├── darling_core v0.20.11 (*)
│               │   │                       └── darling_core feature "strsim"
│               │   │                           └── darling_core v0.20.11 (*)
│               │   ├── serde_with feature "macros"
│               │   │   └── serde_with v3.14.0 (*)
│               │   └── serde_with feature "std"
│               │       ├── serde_with v3.14.0 (*)
│               │       ├── serde feature "std" (*)
│               │       └── serde_with feature "alloc"
│               │           ├── serde_with v3.14.0 (*)
│               │           └── serde feature "alloc"
│               │               ├── serde v1.0.221 (*)
│               │               └── serde_core feature "alloc"
│               │                   └── serde_core v1.0.221
│               ├── thiserror feature "default"
│               │   └── thiserror v1.0.69
│               │       └── thiserror-impl feature "default"
│               │           └── thiserror-impl v1.0.69 (proc-macro)
│               │               ├── proc-macro2 feature "default" (*)
│               │               ├── quote feature "default" (*)
│               │               └── syn feature "default" (*)
│               ├── toml feature "default"
│               │   ├── toml v0.8.23
│               │   │   ├── serde feature "default" (*)
│               │   │   ├── serde_spanned feature "default"
│               │   │   │   └── serde_spanned v0.6.9
│               │   │   │       └── serde feature "default" (*)
│               │   │   ├── serde_spanned feature "serde"
│               │   │   │   └── serde_spanned v0.6.9 (*)
│               │   │   ├── toml_datetime feature "default"
│               │   │   │   └── toml_datetime v0.6.11
│               │   │   │       └── serde feature "default" (*)
│               │   │   ├── toml_datetime feature "serde"
│               │   │   │   └── toml_datetime v0.6.11 (*)
│               │   │   └── toml_edit feature "serde"
│               │   │       ├── toml_edit v0.22.27
│               │   │       │   ├── serde feature "default" (*)
│               │   │       │   ├── serde_spanned feature "default" (*)
│               │   │       │   ├── serde_spanned feature "serde" (*)
│               │   │       │   ├── toml_datetime feature "default" (*)
│               │   │       │   ├── indexmap feature "default"
│               │   │       │   │   ├── indexmap v2.11.1
│               │   │       │   │   │   ├── equivalent v1.0.2
│               │   │       │   │   │   └── hashbrown v0.15.5
│               │   │       │   │   │       ├── equivalent v1.0.2
│               │   │       │   │   │       ├── foldhash v0.1.5
│               │   │       │   │   │       └── allocator-api2 feature "alloc"
│               │   │       │   │   │           └── allocator-api2 v0.2.21
│               │   │       │   │   └── indexmap feature "std"
│               │   │       │   │       └── indexmap v2.11.1 (*)
│               │   │       │   ├── indexmap feature "std" (*)
│               │   │       │   ├── toml_write feature "default"
│               │   │       │   │   ├── toml_write v0.1.2
│               │   │       │   │   └── toml_write feature "std"
│               │   │       │   │       ├── toml_write v0.1.2
│               │   │       │   │       └── toml_write feature "alloc"
│               │   │       │   │           └── toml_write v0.1.2
│               │   │       │   └── winnow feature "default"
│               │   │       │       ├── winnow v0.7.13
│               │   │       │       └── winnow feature "std"
│               │   │       │           ├── winnow v0.7.13
│               │   │       │           └── winnow feature "alloc"
│               │   │       │               └── winnow v0.7.13
│               │   │       └── toml_datetime feature "serde" (*)
│               │   ├── toml feature "display"
│               │   │   ├── toml v0.8.23 (*)
│               │   │   └── toml_edit feature "display"
│               │   │       └── toml_edit v0.22.27 (*)
│               │   └── toml feature "parse"
│               │       ├── toml v0.8.23 (*)
│               │       └── toml_edit feature "parse"
│               │           └── toml_edit v0.22.27 (*)
│               ├── uuid feature "default"
│               │   ├── uuid v1.18.1
│               │   │   ├── serde v1.0.221 (*)
│               │   │   └── getrandom feature "default"
│               │   │       └── getrandom v0.3.3
│               │   │           ├── libc v0.2.175
│               │   │           └── cfg-if feature "default" (*)
... (truncated)
```

</details>

### ron-billing

<details><summary>Reverse tree (-i ron-billing -e features)</summary>

```text
```

</details>

<details><summary>Forward tree (-p ron-billing -e features)</summary>

```text
ron-billing v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/ron-billing)
├── anyhow feature "default"
│   ├── anyhow v1.0.99
│   └── anyhow feature "std"
│       └── anyhow v1.0.99
└── naming feature "default"
    └── naming v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/naming)
        ├── anyhow feature "default" (*)
        ├── base64 feature "default"
        │   ├── base64 v0.22.1
        │   └── base64 feature "std"
        │       ├── base64 v0.22.1
        │       └── base64 feature "alloc"
        │           └── base64 v0.22.1
        ├── blake3 feature "default"
        │   ├── blake3 v1.8.2
        │   │   ├── arrayvec v0.7.6
        │   │   ├── constant_time_eq v0.3.1
        │   │   ├── arrayref feature "default"
        │   │   │   └── arrayref v0.3.9
        │   │   └── cfg-if feature "default"
        │   │       └── cfg-if v1.0.3
        │   │   [build-dependencies]
        │   │   └── cc feature "default"
        │   │       └── cc v1.2.37
        │   │           ├── jobserver v0.1.34
        │   │           │   └── libc feature "default"
        │   │           │       ├── libc v0.2.175
        │   │           │       └── libc feature "std"
        │   │           │           └── libc v0.2.175
        │   │           ├── libc v0.2.175
        │   │           ├── find-msvc-tools feature "default"
        │   │           │   └── find-msvc-tools v0.1.1
        │   │           └── shlex feature "default"
        │   │               ├── shlex v1.3.0
        │   │               └── shlex feature "std"
        │   │                   └── shlex v1.3.0
        │   └── blake3 feature "std"
        │       └── blake3 v1.8.2 (*)
        ├── chrono feature "default"
        │   ├── chrono v0.4.42
        │   │   ├── num-traits v0.2.19
        │   │   │   [build-dependencies]
        │   │   │   └── autocfg feature "default"
        │   │   │       └── autocfg v1.5.0
        │   │   ├── iana-time-zone feature "default"
        │   │   │   └── iana-time-zone v0.1.64
        │   │   │       └── core-foundation-sys feature "default"
        │   │   │           ├── core-foundation-sys v0.8.7
        │   │   │           └── core-foundation-sys feature "link"
        │   │   │               └── core-foundation-sys v0.8.7
        │   │   └── iana-time-zone feature "fallback"
        │   │       └── iana-time-zone v0.1.64 (*)
        │   ├── chrono feature "clock"
        │   │   ├── chrono v0.4.42 (*)
        │   │   ├── chrono feature "iana-time-zone"
        │   │   │   └── chrono v0.4.42 (*)
        │   │   ├── chrono feature "now"
        │   │   │   ├── chrono v0.4.42 (*)
        │   │   │   └── chrono feature "std"
        │   │   │       ├── chrono v0.4.42 (*)
        │   │   │       └── chrono feature "alloc"
        │   │   │           └── chrono v0.4.42 (*)
        │   │   └── chrono feature "winapi"
        │   │       ├── chrono v0.4.42 (*)
        │   │       └── chrono feature "windows-link"
        │   │           └── chrono v0.4.42 (*)
        │   ├── chrono feature "oldtime"
        │   │   └── chrono v0.4.42 (*)
        │   ├── chrono feature "std" (*)
        │   └── chrono feature "wasmbind"
        │       ├── chrono v0.4.42 (*)
        │       ├── chrono feature "js-sys"
        │       │   └── chrono v0.4.42 (*)
        │       └── chrono feature "wasm-bindgen"
        │           └── chrono v0.4.42 (*)
        ├── hex feature "default"
        │   ├── hex v0.4.3
        │   └── hex feature "std"
        │       ├── hex v0.4.3
        │       └── hex feature "alloc"
        │           └── hex v0.4.3
        ├── mime feature "default"
        │   └── mime v0.3.17
        ├── mime_guess feature "default"
        │   ├── mime_guess v2.0.5
        │   │   ├── mime feature "default" (*)
        │   │   └── unicase feature "default"
        │   │       └── unicase v2.8.1
        │   │   [build-dependencies]
        │   │   └── unicase feature "default" (*)
        │   └── mime_guess feature "rev-mappings"
        │       └── mime_guess v2.0.5 (*)
        ├── serde feature "default"
        │   ├── serde v1.0.221
        │   │   ├── serde_core feature "result"
        │   │   │   └── serde_core v1.0.221
        │   │   └── serde_derive feature "default"
        │   │       └── serde_derive v1.0.221 (proc-macro)
        │   │           ├── proc-macro2 feature "proc-macro"
        │   │           │   └── proc-macro2 v1.0.101
        │   │           │       └── unicode-ident feature "default"
        │   │           │           └── unicode-ident v1.0.19
        │   │           ├── quote feature "proc-macro"
        │   │           │   ├── quote v1.0.40
        │   │           │   │   └── proc-macro2 v1.0.101 (*)
        │   │           │   └── proc-macro2 feature "proc-macro" (*)
        │   │           ├── syn feature "clone-impls"
        │   │           │   └── syn v2.0.106
        │   │           │       ├── proc-macro2 v1.0.101 (*)
        │   │           │       ├── quote v1.0.40 (*)
        │   │           │       └── unicode-ident feature "default" (*)
        │   │           ├── syn feature "derive"
        │   │           │   └── syn v2.0.106 (*)
        │   │           ├── syn feature "parsing"
        │   │           │   └── syn v2.0.106 (*)
        │   │           ├── syn feature "printing"
        │   │           │   └── syn v2.0.106 (*)
        │   │           └── syn feature "proc-macro"
        │   │               ├── syn v2.0.106 (*)
        │   │               ├── proc-macro2 feature "proc-macro" (*)
        │   │               └── quote feature "proc-macro" (*)
        │   └── serde feature "std"
        │       ├── serde v1.0.221 (*)
        │       └── serde_core feature "std"
        │           └── serde_core v1.0.221
        ├── serde_with feature "default"
        │   ├── serde_with v3.14.0
        │   │   ├── serde v1.0.221 (*)
        │   │   ├── serde_derive feature "default" (*)
        │   │   └── serde_with_macros feature "default"
        │   │       └── serde_with_macros v3.14.0 (proc-macro)
        │   │           ├── proc-macro2 feature "default"
        │   │           │   ├── proc-macro2 v1.0.101 (*)
        │   │           │   └── proc-macro2 feature "proc-macro" (*)
        │   │           ├── quote feature "default"
        │   │           │   ├── quote v1.0.40 (*)
        │   │           │   └── quote feature "proc-macro" (*)
        │   │           ├── syn feature "default"
        │   │           │   ├── syn v2.0.106 (*)
        │   │           │   ├── syn feature "clone-impls" (*)
        │   │           │   ├── syn feature "derive" (*)
        │   │           │   ├── syn feature "parsing" (*)
        │   │           │   ├── syn feature "printing" (*)
        │   │           │   └── syn feature "proc-macro" (*)
        │   │           ├── syn feature "extra-traits"
        │   │           │   └── syn v2.0.106 (*)
        │   │           ├── syn feature "full"
        │   │           │   └── syn v2.0.106 (*)
        │   │           ├── syn feature "parsing" (*)
        │   │           └── darling feature "default"
        │   │               ├── darling v0.20.11
        │   │               │   ├── darling_core feature "default"
        │   │               │   │   └── darling_core v0.20.11
        │   │               │   │       ├── proc-macro2 feature "default" (*)
        │   │               │   │       ├── quote feature "default" (*)
        │   │               │   │       ├── syn feature "default" (*)
        │   │               │   │       ├── syn feature "extra-traits" (*)
        │   │               │   │       ├── syn feature "full" (*)
        │   │               │   │       ├── fnv feature "default"
        │   │               │   │       │   ├── fnv v1.0.7
        │   │               │   │       │   └── fnv feature "std"
        │   │               │   │       │       └── fnv v1.0.7
        │   │               │   │       ├── ident_case feature "default"
        │   │               │   │       │   └── ident_case v1.0.1
        │   │               │   │       └── strsim feature "default"
        │   │               │   │           └── strsim v0.11.1
        │   │               │   └── darling_macro feature "default"
        │   │               │       └── darling_macro v0.20.11 (proc-macro)
        │   │               │           ├── quote feature "default" (*)
        │   │               │           ├── syn feature "default" (*)
        │   │               │           └── darling_core feature "default" (*)
        │   │               └── darling feature "suggestions"
        │   │                   ├── darling v0.20.11 (*)
        │   │                   └── darling_core feature "suggestions"
        │   │                       ├── darling_core v0.20.11 (*)
        │   │                       └── darling_core feature "strsim"
        │   │                           └── darling_core v0.20.11 (*)
        │   ├── serde_with feature "macros"
        │   │   └── serde_with v3.14.0 (*)
        │   └── serde_with feature "std"
        │       ├── serde_with v3.14.0 (*)
        │       ├── serde feature "std" (*)
        │       └── serde_with feature "alloc"
        │           ├── serde_with v3.14.0 (*)
        │           └── serde feature "alloc"
        │               ├── serde v1.0.221 (*)
        │               └── serde_core feature "alloc"
        │                   └── serde_core v1.0.221
        ├── thiserror feature "default"
        │   └── thiserror v1.0.69
        │       └── thiserror-impl feature "default"
        │           └── thiserror-impl v1.0.69 (proc-macro)
        │               ├── proc-macro2 feature "default" (*)
        │               ├── quote feature "default" (*)
        │               └── syn feature "default" (*)
        ├── toml feature "default"
        │   ├── toml v0.8.23
        │   │   ├── serde feature "default" (*)
        │   │   ├── serde_spanned feature "default"
        │   │   │   └── serde_spanned v0.6.9
        │   │   │       └── serde feature "default" (*)
        │   │   ├── serde_spanned feature "serde"
        │   │   │   └── serde_spanned v0.6.9 (*)
        │   │   ├── toml_datetime feature "default"
        │   │   │   └── toml_datetime v0.6.11
        │   │   │       └── serde feature "default" (*)
        │   │   ├── toml_datetime feature "serde"
        │   │   │   └── toml_datetime v0.6.11 (*)
        │   │   └── toml_edit feature "serde"
        │   │       ├── toml_edit v0.22.27
        │   │       │   ├── serde feature "default" (*)
        │   │       │   ├── serde_spanned feature "default" (*)
        │   │       │   ├── serde_spanned feature "serde" (*)
        │   │       │   ├── toml_datetime feature "default" (*)
        │   │       │   ├── indexmap feature "default"
        │   │       │   │   ├── indexmap v2.11.1
        │   │       │   │   │   ├── equivalent v1.0.2
        │   │       │   │   │   └── hashbrown v0.15.5
        │   │       │   │   │       ├── equivalent v1.0.2
        │   │       │   │   │       ├── foldhash v0.1.5
        │   │       │   │   │       └── allocator-api2 feature "alloc"
        │   │       │   │   │           └── allocator-api2 v0.2.21
        │   │       │   │   └── indexmap feature "std"
        │   │       │   │       └── indexmap v2.11.1 (*)
        │   │       │   ├── indexmap feature "std" (*)
        │   │       │   ├── toml_write feature "default"
        │   │       │   │   ├── toml_write v0.1.2
        │   │       │   │   └── toml_write feature "std"
        │   │       │   │       ├── toml_write v0.1.2
        │   │       │   │       └── toml_write feature "alloc"
        │   │       │   │           └── toml_write v0.1.2
        │   │       │   └── winnow feature "default"
        │   │       │       ├── winnow v0.7.13
        │   │       │       └── winnow feature "std"
        │   │       │           ├── winnow v0.7.13
        │   │       │           └── winnow feature "alloc"
        │   │       │               └── winnow v0.7.13
        │   │       └── toml_datetime feature "serde" (*)
        │   ├── toml feature "display"
        │   │   ├── toml v0.8.23 (*)
        │   │   └── toml_edit feature "display"
        │   │       └── toml_edit v0.22.27 (*)
        │   └── toml feature "parse"
        │       ├── toml v0.8.23 (*)
        │       └── toml_edit feature "parse"
        │           └── toml_edit v0.22.27 (*)
        ├── uuid feature "default"
        │   ├── uuid v1.18.1
        │   │   ├── serde v1.0.221 (*)
        │   │   └── getrandom feature "default"
        │   │       └── getrandom v0.3.3
        │   │           ├── libc v0.2.175
        │   │           └── cfg-if feature "default" (*)
        │   └── uuid feature "std"
        │       └── uuid v1.18.1 (*)
        ├── uuid feature "serde"
        │   └── uuid v1.18.1 (*)
        ├── uuid feature "v4"
        │   ├── uuid v1.18.1 (*)
        │   └── uuid feature "rng"
        │       └── uuid v1.18.1 (*)
        └── workspace-hack feature "default"
            └── workspace-hack v0.1.0 (/Users/mymac/Desktop/RustyOnions/workspace-hack)
                ├── num-traits feature "std"
                │   └── num-traits v0.2.19 (*)
                ├── serde feature "alloc" (*)
                ├── serde feature "default" (*)
                ├── serde feature "derive"
                │   ├── serde v1.0.221 (*)
                │   └── serde feature "serde_derive"
                │       └── serde v1.0.221 (*)
                ├── serde_core feature "alloc" (*)
                ├── serde_core feature "result" (*)
                ├── serde_core feature "std" (*)
                ├── hashbrown feature "default"
                │   ├── hashbrown v0.15.5 (*)
                │   ├── hashbrown feature "allocator-api2"
                │   │   └── hashbrown v0.15.5 (*)
                │   ├── hashbrown feature "default-hasher"
                │   │   └── hashbrown v0.15.5 (*)
                │   ├── hashbrown feature "equivalent"
                │   │   └── hashbrown v0.15.5 (*)
                │   ├── hashbrown feature "inline-more"
                │   │   └── hashbrown v0.15.5 (*)
                │   └── hashbrown feature "raw-entry"
                │       └── hashbrown v0.15.5 (*)
                ├── axum feature "http1"
                │   ├── axum v0.7.9
                │   │   ├── mime feature "default" (*)
                │   │   ├── serde feature "default" (*)
                │   │   ├── async-trait feature "default"
                │   │   │   └── async-trait v0.1.89 (proc-macro)
                │   │   │       ├── proc-macro2 feature "default" (*)
                │   │   │       ├── quote feature "default" (*)
                │   │   │       ├── syn feature "clone-impls" (*)
                │   │   │       ├── syn feature "full" (*)
                │   │   │       ├── syn feature "parsing" (*)
                │   │   │       ├── syn feature "printing" (*)
                │   │   │       ├── syn feature "proc-macro" (*)
... (truncated)
```

</details>

### gateway

<details><summary>Reverse tree (-i gateway -e features)</summary>

```text
```

</details>

<details><summary>Forward tree (-p gateway -e features)</summary>

```text
gateway v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/gateway)
├── reqwest v0.12.23
│   ├── futures-core v0.3.31
│   ├── bytes feature "default"
│   │   ├── bytes v1.10.1
│   │   └── bytes feature "std"
│   │       └── bytes v1.10.1
│   ├── pin-project-lite feature "default"
│   │   └── pin-project-lite v0.2.16
│   ├── http feature "default"
│   │   ├── http v1.3.1
│   │   │   ├── bytes feature "default" (*)
│   │   │   ├── fnv feature "default"
│   │   │   │   ├── fnv v1.0.7
│   │   │   │   └── fnv feature "std"
│   │   │   │       └── fnv v1.0.7
│   │   │   └── itoa feature "default"
│   │   │       └── itoa v1.0.15
│   │   └── http feature "std"
│   │       └── http v1.3.1 (*)
│   ├── http-body feature "default"
│   │   └── http-body v1.0.1
│   │       ├── bytes feature "default" (*)
│   │       └── http feature "default" (*)
│   ├── http-body-util feature "default"
│   │   └── http-body-util v0.1.3
│   │       ├── futures-core v0.3.31
│   │       ├── bytes feature "default" (*)
│   │       ├── pin-project-lite feature "default" (*)
│   │       ├── http feature "default" (*)
│   │       └── http-body feature "default" (*)
│   ├── sync_wrapper feature "default"
│   │   └── sync_wrapper v1.0.2
│   │       └── futures-core v0.3.31
│   ├── sync_wrapper feature "futures"
│   │   ├── sync_wrapper v1.0.2 (*)
│   │   └── sync_wrapper feature "futures-core"
│   │       └── sync_wrapper v1.0.2 (*)
│   ├── tower-service feature "default"
│   │   └── tower-service v0.3.3
│   ├── hyper feature "client"
│   │   └── hyper v1.7.0
│   │       ├── bytes feature "default" (*)
│   │       ├── futures-channel feature "default"
│   │       │   ├── futures-channel v0.3.31
│   │       │   │   ├── futures-core v0.3.31
│   │       │   │   └── futures-sink v0.3.31
│   │       │   └── futures-channel feature "std"
│   │       │       ├── futures-channel v0.3.31 (*)
│   │       │       ├── futures-channel feature "alloc"
│   │       │       │   ├── futures-channel v0.3.31 (*)
│   │       │       │   └── futures-core feature "alloc"
│   │       │       │       └── futures-core v0.3.31
│   │       │       └── futures-core feature "std"
│   │       │           ├── futures-core v0.3.31
│   │       │           └── futures-core feature "alloc" (*)
│   │       ├── futures-core feature "default"
│   │       │   ├── futures-core v0.3.31
│   │       │   └── futures-core feature "std" (*)
│   │       ├── pin-project-lite feature "default" (*)
│   │       ├── pin-utils feature "default"
│   │       │   └── pin-utils v0.1.0
│   │       ├── http feature "default" (*)
│   │       ├── itoa feature "default" (*)
│   │       ├── http-body feature "default" (*)
│   │       ├── atomic-waker feature "default"
│   │       │   └── atomic-waker v1.1.2
│   │       ├── h2 feature "default"
│   │       │   └── h2 v0.4.12
│   │       │       ├── futures-core v0.3.31
│   │       │       ├── futures-sink v0.3.31
│   │       │       ├── bytes feature "default" (*)
│   │       │       ├── slab feature "default"
│   │       │       │   ├── slab v0.4.11
│   │       │       │   └── slab feature "std"
│   │       │       │       └── slab v0.4.11
│   │       │       ├── http feature "default" (*)
│   │       │       ├── fnv feature "default" (*)
│   │       │       ├── atomic-waker feature "default" (*)
│   │       │       ├── indexmap feature "default"
│   │       │       │   ├── indexmap v2.11.1
│   │       │       │   │   ├── equivalent v1.0.2
│   │       │       │   │   └── hashbrown v0.15.5
│   │       │       │   │       ├── equivalent v1.0.2
│   │       │       │   │       ├── foldhash v0.1.5
│   │       │       │   │       └── allocator-api2 feature "alloc"
│   │       │       │   │           └── allocator-api2 v0.2.21
│   │       │       │   └── indexmap feature "std"
│   │       │       │       └── indexmap v2.11.1 (*)
│   │       │       ├── indexmap feature "std" (*)
│   │       │       ├── tokio feature "default"
│   │       │       │   └── tokio v1.47.1
│   │       │       │       ├── mio v1.0.4
│   │       │       │       │   └── libc feature "default"
│   │       │       │       │       ├── libc v0.2.175
│   │       │       │       │       └── libc feature "std"
│   │       │       │       │           └── libc v0.2.175
│   │       │       │       ├── bytes feature "default" (*)
│   │       │       │       ├── pin-project-lite feature "default" (*)
│   │       │       │       ├── libc feature "default" (*)
│   │       │       │       ├── parking_lot feature "default"
│   │       │       │       │   └── parking_lot v0.12.4
│   │       │       │       │       ├── lock_api feature "default"
│   │       │       │       │       │   ├── lock_api v0.4.13
│   │       │       │       │       │   │   └── scopeguard v1.2.0
│   │       │       │       │       │   │   [build-dependencies]
│   │       │       │       │       │   │   └── autocfg feature "default"
│   │       │       │       │       │   │       └── autocfg v1.5.0
│   │       │       │       │       │   └── lock_api feature "atomic_usize"
│   │       │       │       │       │       └── lock_api v0.4.13 (*)
│   │       │       │       │       └── parking_lot_core feature "default"
│   │       │       │       │           └── parking_lot_core v0.9.11
│   │       │       │       │               ├── libc feature "default" (*)
│   │       │       │       │               ├── cfg-if feature "default"
│   │       │       │       │               │   └── cfg-if v1.0.3
│   │       │       │       │               └── smallvec feature "default"
│   │       │       │       │                   └── smallvec v1.15.1
│   │       │       │       ├── signal-hook-registry feature "default"
│   │       │       │       │   └── signal-hook-registry v1.4.6
│   │       │       │       │       └── libc feature "default" (*)
│   │       │       │       ├── socket2 feature "all"
│   │       │       │       │   └── socket2 v0.6.0
│   │       │       │       │       └── libc feature "default" (*)
│   │       │       │       ├── socket2 feature "default"
│   │       │       │       │   └── socket2 v0.6.0 (*)
│   │       │       │       └── tokio-macros feature "default"
│   │       │       │           └── tokio-macros v2.5.0 (proc-macro)
│   │       │       │               ├── proc-macro2 feature "default"
│   │       │       │               │   ├── proc-macro2 v1.0.101
│   │       │       │               │   │   └── unicode-ident feature "default"
│   │       │       │               │   │       └── unicode-ident v1.0.19
│   │       │       │               │   └── proc-macro2 feature "proc-macro"
│   │       │       │               │       └── proc-macro2 v1.0.101 (*)
│   │       │       │               ├── quote feature "default"
│   │       │       │               │   ├── quote v1.0.40
│   │       │       │               │   │   └── proc-macro2 v1.0.101 (*)
│   │       │       │               │   └── quote feature "proc-macro"
│   │       │       │               │       ├── quote v1.0.40 (*)
│   │       │       │               │       └── proc-macro2 feature "proc-macro" (*)
│   │       │       │               ├── syn feature "default"
│   │       │       │               │   ├── syn v2.0.106
│   │       │       │               │   │   ├── proc-macro2 v1.0.101 (*)
│   │       │       │               │   │   ├── quote v1.0.40 (*)
│   │       │       │               │   │   └── unicode-ident feature "default" (*)
│   │       │       │               │   ├── syn feature "clone-impls"
│   │       │       │               │   │   └── syn v2.0.106 (*)
│   │       │       │               │   ├── syn feature "derive"
│   │       │       │               │   │   └── syn v2.0.106 (*)
│   │       │       │               │   ├── syn feature "parsing"
│   │       │       │               │   │   └── syn v2.0.106 (*)
│   │       │       │               │   ├── syn feature "printing"
│   │       │       │               │   │   └── syn v2.0.106 (*)
│   │       │       │               │   └── syn feature "proc-macro"
│   │       │       │               │       ├── syn v2.0.106 (*)
│   │       │       │               │       ├── proc-macro2 feature "proc-macro" (*)
│   │       │       │               │       └── quote feature "proc-macro" (*)
│   │       │       │               └── syn feature "full"
│   │       │       │                   └── syn v2.0.106 (*)
│   │       │       ├── tokio feature "io-util"
│   │       │       │   ├── tokio v1.47.1 (*)
│   │       │       │   └── tokio feature "bytes"
│   │       │       │       └── tokio v1.47.1 (*)
│   │       │       ├── tokio-util feature "codec"
│   │       │       │   └── tokio-util v0.7.16
│   │       │       │       ├── bytes feature "default" (*)
│   │       │       │       ├── futures-core feature "default" (*)
│   │       │       │       ├── futures-sink feature "default"
│   │       │       │       │   ├── futures-sink v0.3.31
│   │       │       │       │   └── futures-sink feature "std"
│   │       │       │       │       ├── futures-sink v0.3.31
│   │       │       │       │       └── futures-sink feature "alloc"
│   │       │       │       │           └── futures-sink v0.3.31
│   │       │       │       ├── pin-project-lite feature "default" (*)
│   │       │       │       ├── tokio feature "default" (*)
│   │       │       │       └── tokio feature "sync"
│   │       │       │           └── tokio v1.47.1 (*)
│   │       │       ├── tokio-util feature "default"
│   │       │       │   └── tokio-util v0.7.16 (*)
│   │       │       ├── tokio-util feature "io"
│   │       │       │   └── tokio-util v0.7.16 (*)
│   │       │       └── tracing feature "std"
│   │       │           ├── tracing v0.1.41
│   │       │           │   ├── tracing-core v0.1.34
│   │       │           │   │   └── once_cell feature "default"
│   │       │           │   │       ├── once_cell v1.21.3
│   │       │           │   │       └── once_cell feature "std"
│   │       │           │   │           ├── once_cell v1.21.3
│   │       │           │   │           └── once_cell feature "alloc"
│   │       │           │   │               ├── once_cell v1.21.3
│   │       │           │   │               └── once_cell feature "race"
│   │       │           │   │                   └── once_cell v1.21.3
│   │       │           │   ├── pin-project-lite feature "default" (*)
│   │       │           │   └── tracing-attributes feature "default"
│   │       │           │       └── tracing-attributes v0.1.30 (proc-macro)
│   │       │           │           ├── proc-macro2 feature "default" (*)
│   │       │           │           ├── quote feature "default" (*)
│   │       │           │           ├── syn feature "clone-impls" (*)
│   │       │           │           ├── syn feature "extra-traits"
│   │       │           │           │   └── syn v2.0.106 (*)
│   │       │           │           ├── syn feature "full" (*)
│   │       │           │           ├── syn feature "parsing" (*)
│   │       │           │           ├── syn feature "printing" (*)
│   │       │           │           ├── syn feature "proc-macro" (*)
│   │       │           │           └── syn feature "visit-mut"
│   │       │           │               └── syn v2.0.106 (*)
│   │       │           └── tracing-core feature "std"
│   │       │               ├── tracing-core v0.1.34 (*)
│   │       │               └── tracing-core feature "once_cell"
│   │       │                   └── tracing-core v0.1.34 (*)
│   │       ├── tokio feature "default" (*)
│   │       ├── tokio feature "sync" (*)
│   │       ├── smallvec feature "const_generics"
│   │       │   └── smallvec v1.15.1
│   │       ├── smallvec feature "const_new"
│   │       │   ├── smallvec v1.15.1
│   │       │   └── smallvec feature "const_generics" (*)
│   │       ├── smallvec feature "default" (*)
│   │       ├── httparse feature "default"
│   │       │   ├── httparse v1.10.1
│   │       │   └── httparse feature "std"
│   │       │       └── httparse v1.10.1
│   │       ├── httpdate feature "default"
│   │       │   └── httpdate v1.0.3
│   │       └── want feature "default"
│   │           └── want v0.3.1
│   │               └── try-lock feature "default"
│   │                   └── try-lock v0.2.5
│   ├── hyper feature "default"
│   │   └── hyper v1.7.0 (*)
│   ├── hyper feature "http1"
│   │   └── hyper v1.7.0 (*)
│   ├── tokio feature "net"
│   │   ├── tokio v1.47.1 (*)
│   │   ├── tokio feature "libc"
│   │   │   └── tokio v1.47.1 (*)
│   │   ├── tokio feature "mio"
│   │   │   └── tokio v1.47.1 (*)
│   │   ├── tokio feature "socket2"
│   │   │   └── tokio v1.47.1 (*)
│   │   ├── mio feature "net"
│   │   │   └── mio v1.0.4 (*)
│   │   ├── mio feature "os-ext"
│   │   │   ├── mio v1.0.4 (*)
│   │   │   └── mio feature "os-poll"
│   │   │       └── mio v1.0.4 (*)
│   │   └── mio feature "os-poll" (*)
│   ├── tokio feature "time"
│   │   └── tokio v1.47.1 (*)
│   ├── hyper-util feature "client"
│   │   ├── hyper-util v0.1.16
│   │   │   ├── futures-util v0.3.31
│   │   │   │   ├── futures-core v0.3.31
│   │   │   │   ├── futures-macro v0.3.31 (proc-macro)
│   │   │   │   │   ├── proc-macro2 feature "default" (*)
│   │   │   │   │   ├── quote feature "default" (*)
│   │   │   │   │   ├── syn feature "default" (*)
│   │   │   │   │   └── syn feature "full" (*)
│   │   │   │   ├── futures-sink v0.3.31
│   │   │   │   ├── futures-task v0.3.31
│   │   │   │   ├── futures-channel feature "std" (*)
│   │   │   │   ├── futures-io feature "std"
│   │   │   │   │   └── futures-io v0.3.31
│   │   │   │   ├── memchr feature "default"
│   │   │   │   │   ├── memchr v2.7.5
│   │   │   │   │   └── memchr feature "std"
│   │   │   │   │       ├── memchr v2.7.5
│   │   │   │   │       └── memchr feature "alloc"
│   │   │   │   │           └── memchr v2.7.5
│   │   │   │   ├── pin-project-lite feature "default" (*)
│   │   │   │   ├── pin-utils feature "default" (*)
│   │   │   │   └── slab feature "default" (*)
│   │   │   ├── tokio v1.47.1 (*)
│   │   │   ├── bytes feature "default" (*)
│   │   │   ├── futures-channel feature "default" (*)
│   │   │   ├── futures-core feature "default" (*)
│   │   │   ├── pin-project-lite feature "default" (*)
│   │   │   ├── http feature "default" (*)
│   │   │   ├── http-body feature "default" (*)
│   │   │   ├── tower-service feature "default" (*)
│   │   │   ├── hyper feature "default" (*)
│   │   │   ├── libc feature "default" (*)
│   │   │   ├── socket2 feature "all" (*)
│   │   │   ├── socket2 feature "default" (*)
│   │   │   ├── tracing feature "std" (*)
│   │   │   ├── base64 feature "default"
│   │   │   │   ├── base64 v0.22.1
│   │   │   │   └── base64 feature "std"
│   │   │   │       ├── base64 v0.22.1
│   │   │   │       └── base64 feature "alloc"
│   │   │   │           └── base64 v0.22.1
│   │   │   ├── ipnet feature "default"
│   │   │   │   ├── ipnet v2.11.0
│   │   │   │   └── ipnet feature "std"
│   │   │   │       └── ipnet v2.11.0
│   │   │   └── percent-encoding feature "default"
│   │   │       ├── percent-encoding v2.3.2
│   │   │       └── percent-encoding feature "std"
│   │   │           ├── percent-encoding v2.3.2
│   │   │           └── percent-encoding feature "alloc"
│   │   │               └── percent-encoding v2.3.2
... (truncated)
```

</details>

### oap

<details><summary>Reverse tree (-i oap -e features)</summary>

```text
```

</details>

<details><summary>Forward tree (-p oap -e features)</summary>

```text
oap v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/oap)
├── blake3 feature "default"
│   ├── blake3 v1.8.2
│   │   ├── arrayvec v0.7.6
│   │   ├── constant_time_eq v0.3.1
│   │   ├── arrayref feature "default"
│   │   │   └── arrayref v0.3.9
│   │   └── cfg-if feature "default"
│   │       └── cfg-if v1.0.3
│   │   [build-dependencies]
│   │   └── cc feature "default"
│   │       └── cc v1.2.37
│   │           ├── jobserver v0.1.34
│   │           │   └── libc feature "default"
│   │           │       ├── libc v0.2.175
│   │           │       └── libc feature "std"
│   │           │           └── libc v0.2.175
│   │           ├── libc v0.2.175
│   │           ├── find-msvc-tools feature "default"
│   │           │   └── find-msvc-tools v0.1.1
│   │           └── shlex feature "default"
│   │               ├── shlex v1.3.0
│   │               └── shlex feature "std"
│   │                   └── shlex v1.3.0
│   └── blake3 feature "std"
│       └── blake3 v1.8.2 (*)
├── bytes feature "default"
│   ├── bytes v1.10.1
│   └── bytes feature "std"
│       └── bytes v1.10.1
├── hex feature "default"
│   ├── hex v0.4.3
│   └── hex feature "std"
│       ├── hex v0.4.3
│       └── hex feature "alloc"
│           └── hex v0.4.3
├── serde_json feature "default"
│   ├── serde_json v1.0.144
│   │   ├── memchr v2.7.5
│   │   ├── serde_core v1.0.221
│   │   ├── itoa feature "default"
│   │   │   └── itoa v1.0.15
│   │   └── ryu feature "default"
│   │       └── ryu v1.0.20
│   └── serde_json feature "std"
│       ├── serde_json v1.0.144 (*)
│       ├── memchr feature "std"
│       │   ├── memchr v2.7.5
│       │   └── memchr feature "alloc"
│       │       └── memchr v2.7.5
│       └── serde_core feature "std"
│           └── serde_core v1.0.221
├── thiserror feature "default"
│   ├── thiserror v2.0.16
│   │   └── thiserror-impl feature "default"
│   │       └── thiserror-impl v2.0.16 (proc-macro)
│   │           ├── proc-macro2 feature "default"
│   │           │   ├── proc-macro2 v1.0.101
│   │           │   │   └── unicode-ident feature "default"
│   │           │   │       └── unicode-ident v1.0.19
│   │           │   └── proc-macro2 feature "proc-macro"
│   │           │       └── proc-macro2 v1.0.101 (*)
│   │           ├── quote feature "default"
│   │           │   ├── quote v1.0.40
│   │           │   │   └── proc-macro2 v1.0.101 (*)
│   │           │   └── quote feature "proc-macro"
│   │           │       ├── quote v1.0.40 (*)
│   │           │       └── proc-macro2 feature "proc-macro" (*)
│   │           └── syn feature "default"
│   │               ├── syn v2.0.106
│   │               │   ├── proc-macro2 v1.0.101 (*)
│   │               │   ├── quote v1.0.40 (*)
│   │               │   └── unicode-ident feature "default" (*)
│   │               ├── syn feature "clone-impls"
│   │               │   └── syn v2.0.106 (*)
│   │               ├── syn feature "derive"
│   │               │   └── syn v2.0.106 (*)
│   │               ├── syn feature "parsing"
│   │               │   └── syn v2.0.106 (*)
│   │               ├── syn feature "printing"
│   │               │   └── syn v2.0.106 (*)
│   │               └── syn feature "proc-macro"
│   │                   ├── syn v2.0.106 (*)
│   │                   ├── proc-macro2 feature "proc-macro" (*)
│   │                   └── quote feature "proc-macro" (*)
│   └── thiserror feature "std"
│       └── thiserror v2.0.16 (*)
├── tokio feature "default"
│   └── tokio v1.47.1
│       ├── mio v1.0.4
│       │   └── libc feature "default" (*)
│       ├── libc feature "default" (*)
│       ├── bytes feature "default" (*)
│       ├── pin-project-lite feature "default"
│       │   └── pin-project-lite v0.2.16
│       ├── parking_lot feature "default"
│       │   └── parking_lot v0.12.4
│       │       ├── lock_api feature "default"
│       │       │   ├── lock_api v0.4.13
│       │       │   │   └── scopeguard v1.2.0
│       │       │   │   [build-dependencies]
│       │       │   │   └── autocfg feature "default"
│       │       │   │       └── autocfg v1.5.0
│       │       │   └── lock_api feature "atomic_usize"
│       │       │       └── lock_api v0.4.13 (*)
│       │       └── parking_lot_core feature "default"
│       │           └── parking_lot_core v0.9.11
│       │               ├── libc feature "default" (*)
│       │               ├── cfg-if feature "default" (*)
│       │               └── smallvec feature "default"
│       │                   └── smallvec v1.15.1
│       ├── signal-hook-registry feature "default"
│       │   └── signal-hook-registry v1.4.6
│       │       └── libc feature "default" (*)
│       ├── socket2 feature "all"
│       │   └── socket2 v0.6.0
│       │       └── libc feature "default" (*)
│       ├── socket2 feature "default"
│       │   └── socket2 v0.6.0 (*)
│       └── tokio-macros feature "default"
│           └── tokio-macros v2.5.0 (proc-macro)
│               ├── proc-macro2 feature "default" (*)
│               ├── quote feature "default" (*)
│               ├── syn feature "default" (*)
│               └── syn feature "full"
│                   └── syn v2.0.106 (*)
├── tokio feature "io-util"
│   ├── tokio v1.47.1 (*)
│   └── tokio feature "bytes"
│       └── tokio v1.47.1 (*)
├── tokio feature "macros"
│   ├── tokio v1.47.1 (*)
│   └── tokio feature "tokio-macros"
│       └── tokio v1.47.1 (*)
├── tokio feature "rt"
│   └── tokio v1.47.1 (*)
└── workspace-hack feature "default"
    └── workspace-hack v0.1.0 (/Users/mymac/Desktop/RustyOnions/workspace-hack)
        ├── futures-channel feature "default"
        │   ├── futures-channel v0.3.31
        │   │   ├── futures-core v0.3.31
        │   │   └── futures-sink v0.3.31
        │   └── futures-channel feature "std"
        │       ├── futures-channel v0.3.31 (*)
        │       ├── futures-channel feature "alloc"
        │       │   ├── futures-channel v0.3.31 (*)
        │       │   └── futures-core feature "alloc"
        │       │       └── futures-core v0.3.31
        │       └── futures-core feature "std"
        │           ├── futures-core v0.3.31
        │           └── futures-core feature "alloc" (*)
        ├── futures-channel feature "sink"
        │   ├── futures-channel v0.3.31 (*)
        │   └── futures-channel feature "futures-sink"
        │       └── futures-channel v0.3.31 (*)
        ├── futures-core feature "default"
        │   ├── futures-core v0.3.31
        │   └── futures-core feature "std" (*)
        ├── futures-sink feature "default"
        │   ├── futures-sink v0.3.31
        │   └── futures-sink feature "std"
        │       ├── futures-sink v0.3.31
        │       └── futures-sink feature "alloc"
        │           └── futures-sink v0.3.31
        ├── futures-task feature "std"
        │   ├── futures-task v0.3.31
        │   └── futures-task feature "alloc"
        │       └── futures-task v0.3.31
        ├── futures-util feature "channel"
        │   ├── futures-util v0.3.31
        │   │   ├── futures-core v0.3.31
        │   │   ├── futures-macro v0.3.31 (proc-macro)
        │   │   │   ├── proc-macro2 feature "default" (*)
        │   │   │   ├── quote feature "default" (*)
        │   │   │   ├── syn feature "default" (*)
        │   │   │   └── syn feature "full" (*)
        │   │   ├── futures-sink v0.3.31
        │   │   ├── futures-task v0.3.31
        │   │   ├── futures-channel feature "std" (*)
        │   │   ├── futures-io feature "std"
        │   │   │   └── futures-io v0.3.31
        │   │   ├── memchr feature "default"
        │   │   │   ├── memchr v2.7.5
        │   │   │   └── memchr feature "std" (*)
        │   │   ├── pin-project-lite feature "default" (*)
        │   │   ├── pin-utils feature "default"
        │   │   │   └── pin-utils v0.1.0
        │   │   └── slab feature "default"
        │   │       ├── slab v0.4.11
        │   │       └── slab feature "std"
        │   │           └── slab v0.4.11
        │   ├── futures-util feature "futures-channel"
        │   │   └── futures-util v0.3.31 (*)
        │   └── futures-util feature "std"
        │       ├── futures-util v0.3.31 (*)
        │       ├── futures-core feature "std" (*)
        │       ├── futures-task feature "std" (*)
        │       ├── futures-util feature "alloc"
        │       │   ├── futures-util v0.3.31 (*)
        │       │   ├── futures-core feature "alloc" (*)
        │       │   └── futures-task feature "alloc" (*)
        │       └── futures-util feature "slab"
        │           └── futures-util v0.3.31 (*)
        ├── futures-util feature "default"
        │   ├── futures-util v0.3.31 (*)
        │   ├── futures-util feature "async-await"
        │   │   └── futures-util v0.3.31 (*)
        │   ├── futures-util feature "async-await-macro"
        │   │   ├── futures-util v0.3.31 (*)
        │   │   ├── futures-util feature "async-await" (*)
        │   │   └── futures-util feature "futures-macro"
        │   │       └── futures-util v0.3.31 (*)
        │   └── futures-util feature "std" (*)
        ├── futures-util feature "io"
        │   ├── futures-util v0.3.31 (*)
        │   ├── futures-util feature "futures-io"
        │   │   └── futures-util v0.3.31 (*)
        │   ├── futures-util feature "memchr"
        │   │   └── futures-util v0.3.31 (*)
        │   └── futures-util feature "std" (*)
        ├── futures-util feature "sink"
        │   ├── futures-util v0.3.31 (*)
        │   └── futures-util feature "futures-sink"
        │       └── futures-util v0.3.31 (*)
        ├── memchr feature "default" (*)
        ├── serde_json feature "default" (*)
        ├── serde_json feature "raw_value"
        │   └── serde_json v1.0.144 (*)
        ├── serde_core feature "alloc"
        │   └── serde_core v1.0.221
        ├── serde_core feature "result"
        │   └── serde_core v1.0.221
        ├── serde_core feature "std" (*)
        ├── tokio feature "default" (*)
        ├── tokio feature "full"
        │   ├── tokio v1.47.1 (*)
        │   ├── tokio feature "fs"
        │   │   └── tokio v1.47.1 (*)
        │   ├── tokio feature "io-std"
        │   │   └── tokio v1.47.1 (*)
        │   ├── tokio feature "io-util" (*)
        │   ├── tokio feature "macros" (*)
        │   ├── tokio feature "net"
        │   │   ├── tokio v1.47.1 (*)
        │   │   ├── tokio feature "libc"
        │   │   │   └── tokio v1.47.1 (*)
        │   │   ├── tokio feature "mio"
        │   │   │   └── tokio v1.47.1 (*)
        │   │   ├── tokio feature "socket2"
        │   │   │   └── tokio v1.47.1 (*)
        │   │   ├── mio feature "net"
        │   │   │   └── mio v1.0.4 (*)
        │   │   ├── mio feature "os-ext"
        │   │   │   ├── mio v1.0.4 (*)
        │   │   │   └── mio feature "os-poll"
        │   │   │       └── mio v1.0.4 (*)
        │   │   └── mio feature "os-poll" (*)
        │   ├── tokio feature "parking_lot"
        │   │   └── tokio v1.47.1 (*)
        │   ├── tokio feature "process"
        │   │   ├── tokio v1.47.1 (*)
        │   │   ├── tokio feature "bytes" (*)
        │   │   ├── tokio feature "libc" (*)
        │   │   ├── tokio feature "mio" (*)
        │   │   ├── tokio feature "signal-hook-registry"
        │   │   │   └── tokio v1.47.1 (*)
        │   │   ├── mio feature "net" (*)
        │   │   ├── mio feature "os-ext" (*)
        │   │   └── mio feature "os-poll" (*)
        │   ├── tokio feature "rt" (*)
        │   ├── tokio feature "rt-multi-thread"
        │   │   ├── tokio v1.47.1 (*)
        │   │   └── tokio feature "rt" (*)
        │   ├── tokio feature "signal"
        │   │   ├── tokio v1.47.1 (*)
        │   │   ├── tokio feature "libc" (*)
        │   │   ├── tokio feature "mio" (*)
        │   │   ├── tokio feature "signal-hook-registry" (*)
        │   │   ├── mio feature "net" (*)
        │   │   ├── mio feature "os-ext" (*)
        │   │   └── mio feature "os-poll" (*)
        │   ├── tokio feature "sync"
        │   │   └── tokio v1.47.1 (*)
        │   └── tokio feature "time"
        │       └── tokio v1.47.1 (*)
        ├── smallvec feature "const_new"
        │   ├── smallvec v1.15.1
        │   └── smallvec feature "const_generics"
        │       └── smallvec v1.15.1
        ├── axum feature "http1"
        │   ├── axum v0.7.9
        │   │   ├── bytes feature "default" (*)
        │   │   ├── futures-util feature "alloc" (*)
        │   │   ├── memchr feature "default" (*)
        │   │   ├── pin-project-lite feature "default" (*)
        │   │   ├── serde_json feature "default" (*)
        │   │   ├── serde_json feature "raw_value" (*)
        │   │   ├── itoa feature "default" (*)
        │   │   ├── tokio feature "default" (*)
        │   │   ├── tokio feature "time" (*)
... (truncated)
```

</details>

### ron-bus

<details><summary>Reverse tree (-i ron-bus -e features)</summary>

```text
```

</details>

<details><summary>Forward tree (-p ron-bus -e features)</summary>

```text
ron-bus v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/ron-bus)
├── rmp-serde feature "default"
│   └── rmp-serde v1.3.0
│       ├── byteorder feature "default"
│       │   ├── byteorder v1.5.0
│       │   └── byteorder feature "std"
│       │       └── byteorder v1.5.0
│       ├── rmp feature "default"
│       │   ├── rmp v0.8.14
│       │   │   ├── byteorder v1.5.0
│       │   │   ├── num-traits v0.2.19
│       │   │   │   [build-dependencies]
│       │   │   │   └── autocfg feature "default"
│       │   │   │       └── autocfg v1.5.0
│       │   │   └── paste feature "default"
│       │   │       └── paste v1.0.15 (proc-macro)
│       │   └── rmp feature "std"
│       │       ├── rmp v0.8.14 (*)
│       │       ├── byteorder feature "std" (*)
│       │       └── num-traits feature "std"
│       │           └── num-traits v0.2.19 (*)
│       └── serde feature "default"
│           ├── serde v1.0.221
│           │   ├── serde_core feature "result"
│           │   │   └── serde_core v1.0.221
│           │   └── serde_derive feature "default"
│           │       └── serde_derive v1.0.221 (proc-macro)
│           │           ├── proc-macro2 feature "proc-macro"
│           │           │   └── proc-macro2 v1.0.101
│           │           │       └── unicode-ident feature "default"
│           │           │           └── unicode-ident v1.0.19
│           │           ├── quote feature "proc-macro"
│           │           │   ├── quote v1.0.40
│           │           │   │   └── proc-macro2 v1.0.101 (*)
│           │           │   └── proc-macro2 feature "proc-macro" (*)
│           │           ├── syn feature "clone-impls"
│           │           │   └── syn v2.0.106
│           │           │       ├── proc-macro2 v1.0.101 (*)
│           │           │       ├── quote v1.0.40 (*)
│           │           │       └── unicode-ident feature "default" (*)
│           │           ├── syn feature "derive"
│           │           │   └── syn v2.0.106 (*)
│           │           ├── syn feature "parsing"
│           │           │   └── syn v2.0.106 (*)
│           │           ├── syn feature "printing"
│           │           │   └── syn v2.0.106 (*)
│           │           └── syn feature "proc-macro"
│           │               ├── syn v2.0.106 (*)
│           │               ├── proc-macro2 feature "proc-macro" (*)
│           │               └── quote feature "proc-macro" (*)
│           └── serde feature "std"
│               ├── serde v1.0.221 (*)
│               └── serde_core feature "std"
│                   └── serde_core v1.0.221
├── serde feature "default" (*)
├── thiserror feature "default"
│   └── thiserror v1.0.69
│       └── thiserror-impl feature "default"
│           └── thiserror-impl v1.0.69 (proc-macro)
│               ├── proc-macro2 feature "default"
│               │   ├── proc-macro2 v1.0.101 (*)
│               │   └── proc-macro2 feature "proc-macro" (*)
│               ├── quote feature "default"
│               │   ├── quote v1.0.40 (*)
│               │   └── quote feature "proc-macro" (*)
│               └── syn feature "default"
│                   ├── syn v2.0.106 (*)
│                   ├── syn feature "clone-impls" (*)
│                   ├── syn feature "derive" (*)
│                   ├── syn feature "parsing" (*)
│                   ├── syn feature "printing" (*)
│                   └── syn feature "proc-macro" (*)
└── workspace-hack feature "default"
    └── workspace-hack v0.1.0 (/Users/mymac/Desktop/RustyOnions/workspace-hack)
        ├── num-traits feature "std" (*)
        ├── serde feature "alloc"
        │   ├── serde v1.0.221 (*)
        │   └── serde_core feature "alloc"
        │       └── serde_core v1.0.221
        ├── serde feature "default" (*)
        ├── serde feature "derive"
        │   ├── serde v1.0.221 (*)
        │   └── serde feature "serde_derive"
        │       └── serde v1.0.221 (*)
        ├── serde_core feature "alloc" (*)
        ├── serde_core feature "result" (*)
        ├── serde_core feature "std" (*)
        ├── axum feature "http1"
        │   ├── axum v0.7.9
        │   │   ├── serde feature "default" (*)
        │   │   ├── async-trait feature "default"
        │   │   │   └── async-trait v0.1.89 (proc-macro)
        │   │   │       ├── proc-macro2 feature "default" (*)
        │   │   │       ├── quote feature "default" (*)
        │   │   │       ├── syn feature "clone-impls" (*)
        │   │   │       ├── syn feature "full"
        │   │   │       │   └── syn v2.0.106 (*)
        │   │   │       ├── syn feature "parsing" (*)
        │   │   │       ├── syn feature "printing" (*)
        │   │   │       ├── syn feature "proc-macro" (*)
        │   │   │       └── syn feature "visit-mut"
        │   │   │           └── syn v2.0.106 (*)
        │   │   ├── axum-core feature "default"
        │   │   │   └── axum-core v0.4.5
        │   │   │       ├── async-trait feature "default" (*)
        │   │   │       ├── bytes feature "default"
        │   │   │       │   ├── bytes v1.10.1
        │   │   │       │   └── bytes feature "std"
        │   │   │       │       └── bytes v1.10.1
        │   │   │       ├── futures-util feature "alloc"
        │   │   │       │   ├── futures-util v0.3.31
        │   │   │       │   │   ├── futures-core v0.3.31
        │   │   │       │   │   ├── futures-macro v0.3.31 (proc-macro)
        │   │   │       │   │   │   ├── proc-macro2 feature "default" (*)
        │   │   │       │   │   │   ├── quote feature "default" (*)
        │   │   │       │   │   │   ├── syn feature "default" (*)
        │   │   │       │   │   │   └── syn feature "full" (*)
        │   │   │       │   │   ├── futures-sink v0.3.31
        │   │   │       │   │   ├── futures-task v0.3.31
        │   │   │       │   │   ├── futures-channel feature "std"
        │   │   │       │   │   │   ├── futures-channel v0.3.31
        │   │   │       │   │   │   │   ├── futures-core v0.3.31
        │   │   │       │   │   │   │   └── futures-sink v0.3.31
        │   │   │       │   │   │   ├── futures-channel feature "alloc"
        │   │   │       │   │   │   │   ├── futures-channel v0.3.31 (*)
        │   │   │       │   │   │   │   └── futures-core feature "alloc"
        │   │   │       │   │   │   │       └── futures-core v0.3.31
        │   │   │       │   │   │   └── futures-core feature "std"
        │   │   │       │   │   │       ├── futures-core v0.3.31
        │   │   │       │   │   │       └── futures-core feature "alloc" (*)
        │   │   │       │   │   ├── futures-io feature "std"
        │   │   │       │   │   │   └── futures-io v0.3.31
        │   │   │       │   │   ├── memchr feature "default"
        │   │   │       │   │   │   ├── memchr v2.7.5
        │   │   │       │   │   │   └── memchr feature "std"
        │   │   │       │   │   │       ├── memchr v2.7.5
        │   │   │       │   │   │       └── memchr feature "alloc"
        │   │   │       │   │   │           └── memchr v2.7.5
        │   │   │       │   │   ├── pin-project-lite feature "default"
        │   │   │       │   │   │   └── pin-project-lite v0.2.16
        │   │   │       │   │   ├── pin-utils feature "default"
        │   │   │       │   │   │   └── pin-utils v0.1.0
        │   │   │       │   │   └── slab feature "default"
        │   │   │       │   │       ├── slab v0.4.11
        │   │   │       │   │       └── slab feature "std"
        │   │   │       │   │           └── slab v0.4.11
        │   │   │       │   ├── futures-core feature "alloc" (*)
        │   │   │       │   └── futures-task feature "alloc"
        │   │   │       │       └── futures-task v0.3.31
        │   │   │       ├── pin-project-lite feature "default" (*)
        │   │   │       ├── http feature "default"
        │   │   │       │   ├── http v1.3.1
        │   │   │       │   │   ├── bytes feature "default" (*)
        │   │   │       │   │   ├── fnv feature "default"
        │   │   │       │   │   │   ├── fnv v1.0.7
        │   │   │       │   │   │   └── fnv feature "std"
        │   │   │       │   │   │       └── fnv v1.0.7
        │   │   │       │   │   └── itoa feature "default"
        │   │   │       │   │       └── itoa v1.0.15
        │   │   │       │   └── http feature "std"
        │   │   │       │       └── http v1.3.1 (*)
        │   │   │       ├── http-body feature "default"
        │   │   │       │   └── http-body v1.0.1
        │   │   │       │       ├── bytes feature "default" (*)
        │   │   │       │       └── http feature "default" (*)
        │   │   │       ├── http-body-util feature "default"
        │   │   │       │   └── http-body-util v0.1.3
        │   │   │       │       ├── futures-core v0.3.31
        │   │   │       │       ├── bytes feature "default" (*)
        │   │   │       │       ├── pin-project-lite feature "default" (*)
        │   │   │       │       ├── http feature "default" (*)
        │   │   │       │       └── http-body feature "default" (*)
        │   │   │       ├── mime feature "default"
        │   │   │       │   └── mime v0.3.17
        │   │   │       ├── rustversion feature "default"
        │   │   │       │   └── rustversion v1.0.22 (proc-macro)
        │   │   │       ├── sync_wrapper feature "default"
        │   │   │       │   └── sync_wrapper v1.0.2
        │   │   │       │       └── futures-core v0.3.31
        │   │   │       ├── tower-layer feature "default"
        │   │   │       │   └── tower-layer v0.3.3
        │   │   │       └── tower-service feature "default"
        │   │   │           └── tower-service v0.3.3
        │   │   ├── bytes feature "default" (*)
        │   │   ├── futures-util feature "alloc" (*)
        │   │   ├── memchr feature "default" (*)
        │   │   ├── pin-project-lite feature "default" (*)
        │   │   ├── http feature "default" (*)
        │   │   ├── itoa feature "default" (*)
        │   │   ├── http-body feature "default" (*)
        │   │   ├── http-body-util feature "default" (*)
        │   │   ├── mime feature "default" (*)
        │   │   ├── rustversion feature "default" (*)
        │   │   ├── sync_wrapper feature "default" (*)
        │   │   ├── tower-layer feature "default" (*)
        │   │   ├── tower-service feature "default" (*)
        │   │   ├── axum-macros feature "default"
        │   │   │   └── axum-macros v0.4.2 (proc-macro)
        │   │   │       ├── proc-macro2 feature "default" (*)
        │   │   │       ├── quote feature "default" (*)
        │   │   │       ├── syn feature "default" (*)
        │   │   │       ├── syn feature "extra-traits"
        │   │   │       │   └── syn v2.0.106 (*)
        │   │   │       ├── syn feature "full" (*)
        │   │   │       └── syn feature "parsing" (*)
        │   │   ├── hyper feature "default"
        │   │   │   └── hyper v1.7.0
        │   │   │       ├── bytes feature "default" (*)
        │   │   │       ├── futures-channel feature "default"
        │   │   │       │   ├── futures-channel v0.3.31 (*)
        │   │   │       │   └── futures-channel feature "std" (*)
        │   │   │       ├── futures-core feature "default"
        │   │   │       │   ├── futures-core v0.3.31
        │   │   │       │   └── futures-core feature "std" (*)
        │   │   │       ├── pin-project-lite feature "default" (*)
        │   │   │       ├── pin-utils feature "default" (*)
        │   │   │       ├── http feature "default" (*)
        │   │   │       ├── itoa feature "default" (*)
        │   │   │       ├── http-body feature "default" (*)
        │   │   │       ├── atomic-waker feature "default"
        │   │   │       │   └── atomic-waker v1.1.2
        │   │   │       ├── h2 feature "default"
        │   │   │       │   └── h2 v0.4.12
        │   │   │       │       ├── futures-core v0.3.31
        │   │   │       │       ├── futures-sink v0.3.31
        │   │   │       │       ├── bytes feature "default" (*)
        │   │   │       │       ├── slab feature "default" (*)
        │   │   │       │       ├── http feature "default" (*)
        │   │   │       │       ├── fnv feature "default" (*)
        │   │   │       │       ├── atomic-waker feature "default" (*)
        │   │   │       │       ├── indexmap feature "default"
        │   │   │       │       │   ├── indexmap v2.11.1
        │   │   │       │       │   │   ├── equivalent v1.0.2
        │   │   │       │       │   │   └── hashbrown v0.15.5
        │   │   │       │       │   │       ├── equivalent v1.0.2
        │   │   │       │       │   │       ├── foldhash v0.1.5
        │   │   │       │       │   │       └── allocator-api2 feature "alloc"
        │   │   │       │       │   │           └── allocator-api2 v0.2.21
        │   │   │       │       │   └── indexmap feature "std"
        │   │   │       │       │       └── indexmap v2.11.1 (*)
        │   │   │       │       ├── indexmap feature "std" (*)
        │   │   │       │       ├── tokio feature "default"
        │   │   │       │       │   └── tokio v1.47.1
        │   │   │       │       │       ├── mio v1.0.4
        │   │   │       │       │       │   └── libc feature "default"
        │   │   │       │       │       │       ├── libc v0.2.175
        │   │   │       │       │       │       └── libc feature "std"
        │   │   │       │       │       │           └── libc v0.2.175
        │   │   │       │       │       ├── bytes feature "default" (*)
        │   │   │       │       │       ├── pin-project-lite feature "default" (*)
        │   │   │       │       │       ├── libc feature "default" (*)
        │   │   │       │       │       ├── parking_lot feature "default"
        │   │   │       │       │       │   └── parking_lot v0.12.4
        │   │   │       │       │       │       ├── lock_api feature "default"
        │   │   │       │       │       │       │   ├── lock_api v0.4.13
        │   │   │       │       │       │       │   │   └── scopeguard v1.2.0
        │   │   │       │       │       │       │   │   [build-dependencies]
        │   │   │       │       │       │       │   │   └── autocfg feature "default" (*)
        │   │   │       │       │       │       │   └── lock_api feature "atomic_usize"
        │   │   │       │       │       │       │       └── lock_api v0.4.13 (*)
        │   │   │       │       │       │       └── parking_lot_core feature "default"
        │   │   │       │       │       │           └── parking_lot_core v0.9.11
        │   │   │       │       │       │               ├── libc feature "default" (*)
        │   │   │       │       │       │               ├── cfg-if feature "default"
        │   │   │       │       │       │               │   └── cfg-if v1.0.3
        │   │   │       │       │       │               └── smallvec feature "default"
        │   │   │       │       │       │                   └── smallvec v1.15.1
        │   │   │       │       │       ├── signal-hook-registry feature "default"
        │   │   │       │       │       │   └── signal-hook-registry v1.4.6
        │   │   │       │       │       │       └── libc feature "default" (*)
        │   │   │       │       │       ├── socket2 feature "all"
        │   │   │       │       │       │   └── socket2 v0.6.0
        │   │   │       │       │       │       └── libc feature "default" (*)
        │   │   │       │       │       ├── socket2 feature "default"
        │   │   │       │       │       │   └── socket2 v0.6.0 (*)
        │   │   │       │       │       └── tokio-macros feature "default"
        │   │   │       │       │           └── tokio-macros v2.5.0 (proc-macro)
        │   │   │       │       │               ├── proc-macro2 feature "default" (*)
        │   │   │       │       │               ├── quote feature "default" (*)
        │   │   │       │       │               ├── syn feature "default" (*)
        │   │   │       │       │               └── syn feature "full" (*)
        │   │   │       │       ├── tokio feature "io-util"
        │   │   │       │       │   ├── tokio v1.47.1 (*)
        │   │   │       │       │   └── tokio feature "bytes"
        │   │   │       │       │       └── tokio v1.47.1 (*)
        │   │   │       │       ├── tokio-util feature "codec"
        │   │   │       │       │   └── tokio-util v0.7.16
        │   │   │       │       │       ├── bytes feature "default" (*)
        │   │   │       │       │       ├── futures-core feature "default" (*)
        │   │   │       │       │       ├── futures-sink feature "default"
        │   │   │       │       │       │   ├── futures-sink v0.3.31
        │   │   │       │       │       │   └── futures-sink feature "std"
        │   │   │       │       │       │       ├── futures-sink v0.3.31
        │   │   │       │       │       │       └── futures-sink feature "alloc"
        │   │   │       │       │       │           └── futures-sink v0.3.31
        │   │   │       │       │       ├── pin-project-lite feature "default" (*)
        │   │   │       │       │       ├── tokio feature "default" (*)
        │   │   │       │       │       └── tokio feature "sync"
        │   │   │       │       │           └── tokio v1.47.1 (*)
        │   │   │       │       ├── tokio-util feature "default"
... (truncated)
```

</details>

### ron-kernel

<details><summary>Reverse tree (-i ron-kernel -e features)</summary>

```text
```

</details>

<details><summary>Forward tree (-p ron-kernel -e features)</summary>

```text
ron-kernel v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/ron-kernel)
├── reqwest v0.12.23
│   ├── futures-core v0.3.31
│   ├── bytes feature "default"
│   │   ├── bytes v1.10.1
│   │   └── bytes feature "std"
│   │       └── bytes v1.10.1
│   ├── pin-project-lite feature "default"
│   │   └── pin-project-lite v0.2.16
│   ├── http feature "default"
│   │   ├── http v1.3.1
│   │   │   ├── bytes feature "default" (*)
│   │   │   ├── fnv feature "default"
│   │   │   │   ├── fnv v1.0.7
│   │   │   │   └── fnv feature "std"
│   │   │   │       └── fnv v1.0.7
│   │   │   └── itoa feature "default"
│   │   │       └── itoa v1.0.15
│   │   └── http feature "std"
│   │       └── http v1.3.1 (*)
│   ├── http-body feature "default"
│   │   └── http-body v1.0.1
│   │       ├── bytes feature "default" (*)
│   │       └── http feature "default" (*)
│   ├── http-body-util feature "default"
│   │   └── http-body-util v0.1.3
│   │       ├── futures-core v0.3.31
│   │       ├── bytes feature "default" (*)
│   │       ├── pin-project-lite feature "default" (*)
│   │       ├── http feature "default" (*)
│   │       └── http-body feature "default" (*)
│   ├── sync_wrapper feature "default"
│   │   └── sync_wrapper v1.0.2
│   │       └── futures-core v0.3.31
│   ├── sync_wrapper feature "futures"
│   │   ├── sync_wrapper v1.0.2 (*)
│   │   └── sync_wrapper feature "futures-core"
│   │       └── sync_wrapper v1.0.2 (*)
│   ├── tower-service feature "default"
│   │   └── tower-service v0.3.3
│   ├── hyper feature "client"
│   │   └── hyper v1.7.0
│   │       ├── bytes feature "default" (*)
│   │       ├── futures-channel feature "default"
│   │       │   ├── futures-channel v0.3.31
│   │       │   │   ├── futures-core v0.3.31
│   │       │   │   └── futures-sink v0.3.31
│   │       │   └── futures-channel feature "std"
│   │       │       ├── futures-channel v0.3.31 (*)
│   │       │       ├── futures-channel feature "alloc"
│   │       │       │   ├── futures-channel v0.3.31 (*)
│   │       │       │   └── futures-core feature "alloc"
│   │       │       │       └── futures-core v0.3.31
│   │       │       └── futures-core feature "std"
│   │       │           ├── futures-core v0.3.31
│   │       │           └── futures-core feature "alloc" (*)
│   │       ├── futures-core feature "default"
│   │       │   ├── futures-core v0.3.31
│   │       │   └── futures-core feature "std" (*)
│   │       ├── pin-project-lite feature "default" (*)
│   │       ├── pin-utils feature "default"
│   │       │   └── pin-utils v0.1.0
│   │       ├── http feature "default" (*)
│   │       ├── itoa feature "default" (*)
│   │       ├── http-body feature "default" (*)
│   │       ├── atomic-waker feature "default"
│   │       │   └── atomic-waker v1.1.2
│   │       ├── h2 feature "default"
│   │       │   └── h2 v0.4.12
│   │       │       ├── futures-core v0.3.31
│   │       │       ├── futures-sink v0.3.31
│   │       │       ├── bytes feature "default" (*)
│   │       │       ├── slab feature "default"
│   │       │       │   ├── slab v0.4.11
│   │       │       │   └── slab feature "std"
│   │       │       │       └── slab v0.4.11
│   │       │       ├── http feature "default" (*)
│   │       │       ├── fnv feature "default" (*)
│   │       │       ├── atomic-waker feature "default" (*)
│   │       │       ├── indexmap feature "default"
│   │       │       │   ├── indexmap v2.11.1
│   │       │       │   │   ├── equivalent v1.0.2
│   │       │       │   │   └── hashbrown v0.15.5
│   │       │       │   │       ├── equivalent v1.0.2
│   │       │       │   │       ├── foldhash v0.1.5
│   │       │       │   │       └── allocator-api2 feature "alloc"
│   │       │       │   │           └── allocator-api2 v0.2.21
│   │       │       │   └── indexmap feature "std"
│   │       │       │       └── indexmap v2.11.1 (*)
│   │       │       ├── indexmap feature "std" (*)
│   │       │       ├── tokio feature "default"
│   │       │       │   └── tokio v1.47.1
│   │       │       │       ├── mio v1.0.4
│   │       │       │       │   └── libc feature "default"
│   │       │       │       │       ├── libc v0.2.175
│   │       │       │       │       └── libc feature "std"
│   │       │       │       │           └── libc v0.2.175
│   │       │       │       ├── bytes feature "default" (*)
│   │       │       │       ├── pin-project-lite feature "default" (*)
│   │       │       │       ├── libc feature "default" (*)
│   │       │       │       ├── parking_lot feature "default"
│   │       │       │       │   └── parking_lot v0.12.4
│   │       │       │       │       ├── lock_api feature "default"
│   │       │       │       │       │   ├── lock_api v0.4.13
│   │       │       │       │       │   │   └── scopeguard v1.2.0
│   │       │       │       │       │   │   [build-dependencies]
│   │       │       │       │       │   │   └── autocfg feature "default"
│   │       │       │       │       │   │       └── autocfg v1.5.0
│   │       │       │       │       │   └── lock_api feature "atomic_usize"
│   │       │       │       │       │       └── lock_api v0.4.13 (*)
│   │       │       │       │       └── parking_lot_core feature "default"
│   │       │       │       │           └── parking_lot_core v0.9.11
│   │       │       │       │               ├── libc feature "default" (*)
│   │       │       │       │               ├── cfg-if feature "default"
│   │       │       │       │               │   └── cfg-if v1.0.3
│   │       │       │       │               └── smallvec feature "default"
│   │       │       │       │                   └── smallvec v1.15.1
│   │       │       │       ├── signal-hook-registry feature "default"
│   │       │       │       │   └── signal-hook-registry v1.4.6
│   │       │       │       │       └── libc feature "default" (*)
│   │       │       │       ├── socket2 feature "all"
│   │       │       │       │   └── socket2 v0.6.0
│   │       │       │       │       └── libc feature "default" (*)
│   │       │       │       ├── socket2 feature "default"
│   │       │       │       │   └── socket2 v0.6.0 (*)
│   │       │       │       └── tokio-macros feature "default"
│   │       │       │           └── tokio-macros v2.5.0 (proc-macro)
│   │       │       │               ├── proc-macro2 feature "default"
│   │       │       │               │   ├── proc-macro2 v1.0.101
│   │       │       │               │   │   └── unicode-ident feature "default"
│   │       │       │               │   │       └── unicode-ident v1.0.19
│   │       │       │               │   └── proc-macro2 feature "proc-macro"
│   │       │       │               │       └── proc-macro2 v1.0.101 (*)
│   │       │       │               ├── quote feature "default"
│   │       │       │               │   ├── quote v1.0.40
│   │       │       │               │   │   └── proc-macro2 v1.0.101 (*)
│   │       │       │               │   └── quote feature "proc-macro"
│   │       │       │               │       ├── quote v1.0.40 (*)
│   │       │       │               │       └── proc-macro2 feature "proc-macro" (*)
│   │       │       │               ├── syn feature "default"
│   │       │       │               │   ├── syn v2.0.106
│   │       │       │               │   │   ├── proc-macro2 v1.0.101 (*)
│   │       │       │               │   │   ├── quote v1.0.40 (*)
│   │       │       │               │   │   └── unicode-ident feature "default" (*)
│   │       │       │               │   ├── syn feature "clone-impls"
│   │       │       │               │   │   └── syn v2.0.106 (*)
│   │       │       │               │   ├── syn feature "derive"
│   │       │       │               │   │   └── syn v2.0.106 (*)
│   │       │       │               │   ├── syn feature "parsing"
│   │       │       │               │   │   └── syn v2.0.106 (*)
│   │       │       │               │   ├── syn feature "printing"
│   │       │       │               │   │   └── syn v2.0.106 (*)
│   │       │       │               │   └── syn feature "proc-macro"
│   │       │       │               │       ├── syn v2.0.106 (*)
│   │       │       │               │       ├── proc-macro2 feature "proc-macro" (*)
│   │       │       │               │       └── quote feature "proc-macro" (*)
│   │       │       │               └── syn feature "full"
│   │       │       │                   └── syn v2.0.106 (*)
│   │       │       ├── tokio feature "io-util"
│   │       │       │   ├── tokio v1.47.1 (*)
│   │       │       │   └── tokio feature "bytes"
│   │       │       │       └── tokio v1.47.1 (*)
│   │       │       ├── tokio-util feature "codec"
│   │       │       │   └── tokio-util v0.7.16
│   │       │       │       ├── bytes feature "default" (*)
│   │       │       │       ├── futures-core feature "default" (*)
│   │       │       │       ├── futures-sink feature "default"
│   │       │       │       │   ├── futures-sink v0.3.31
│   │       │       │       │   └── futures-sink feature "std"
│   │       │       │       │       ├── futures-sink v0.3.31
│   │       │       │       │       └── futures-sink feature "alloc"
│   │       │       │       │           └── futures-sink v0.3.31
│   │       │       │       ├── pin-project-lite feature "default" (*)
│   │       │       │       ├── tokio feature "default" (*)
│   │       │       │       └── tokio feature "sync"
│   │       │       │           └── tokio v1.47.1 (*)
│   │       │       ├── tokio-util feature "default"
│   │       │       │   └── tokio-util v0.7.16 (*)
│   │       │       ├── tokio-util feature "io"
│   │       │       │   └── tokio-util v0.7.16 (*)
│   │       │       └── tracing feature "std"
│   │       │           ├── tracing v0.1.41
│   │       │           │   ├── tracing-core v0.1.34
│   │       │           │   │   └── once_cell feature "default"
│   │       │           │   │       ├── once_cell v1.21.3
│   │       │           │   │       └── once_cell feature "std"
│   │       │           │   │           ├── once_cell v1.21.3
│   │       │           │   │           └── once_cell feature "alloc"
│   │       │           │   │               ├── once_cell v1.21.3
│   │       │           │   │               └── once_cell feature "race"
│   │       │           │   │                   └── once_cell v1.21.3
│   │       │           │   ├── pin-project-lite feature "default" (*)
│   │       │           │   └── tracing-attributes feature "default"
│   │       │           │       └── tracing-attributes v0.1.30 (proc-macro)
│   │       │           │           ├── proc-macro2 feature "default" (*)
│   │       │           │           ├── quote feature "default" (*)
│   │       │           │           ├── syn feature "clone-impls" (*)
│   │       │           │           ├── syn feature "extra-traits"
│   │       │           │           │   └── syn v2.0.106 (*)
│   │       │           │           ├── syn feature "full" (*)
│   │       │           │           ├── syn feature "parsing" (*)
│   │       │           │           ├── syn feature "printing" (*)
│   │       │           │           ├── syn feature "proc-macro" (*)
│   │       │           │           └── syn feature "visit-mut"
│   │       │           │               └── syn v2.0.106 (*)
│   │       │           └── tracing-core feature "std"
│   │       │               ├── tracing-core v0.1.34 (*)
│   │       │               └── tracing-core feature "once_cell"
│   │       │                   └── tracing-core v0.1.34 (*)
│   │       ├── tokio feature "default" (*)
│   │       ├── tokio feature "sync" (*)
│   │       ├── smallvec feature "const_generics"
│   │       │   └── smallvec v1.15.1
│   │       ├── smallvec feature "const_new"
│   │       │   ├── smallvec v1.15.1
│   │       │   └── smallvec feature "const_generics" (*)
│   │       ├── smallvec feature "default" (*)
│   │       ├── httparse feature "default"
│   │       │   ├── httparse v1.10.1
│   │       │   └── httparse feature "std"
│   │       │       └── httparse v1.10.1
│   │       ├── httpdate feature "default"
│   │       │   └── httpdate v1.0.3
│   │       └── want feature "default"
│   │           └── want v0.3.1
│   │               └── try-lock feature "default"
│   │                   └── try-lock v0.2.5
│   ├── hyper feature "default"
│   │   └── hyper v1.7.0 (*)
│   ├── hyper feature "http1"
│   │   └── hyper v1.7.0 (*)
│   ├── tokio feature "net"
│   │   ├── tokio v1.47.1 (*)
│   │   ├── tokio feature "libc"
│   │   │   └── tokio v1.47.1 (*)
│   │   ├── tokio feature "mio"
│   │   │   └── tokio v1.47.1 (*)
│   │   ├── tokio feature "socket2"
│   │   │   └── tokio v1.47.1 (*)
│   │   ├── mio feature "net"
│   │   │   └── mio v1.0.4 (*)
│   │   ├── mio feature "os-ext"
│   │   │   ├── mio v1.0.4 (*)
│   │   │   └── mio feature "os-poll"
│   │   │       └── mio v1.0.4 (*)
│   │   └── mio feature "os-poll" (*)
│   ├── tokio feature "time"
│   │   └── tokio v1.47.1 (*)
│   ├── hyper-util feature "client"
│   │   ├── hyper-util v0.1.16
│   │   │   ├── futures-util v0.3.31
│   │   │   │   ├── futures-core v0.3.31
│   │   │   │   ├── futures-macro v0.3.31 (proc-macro)
│   │   │   │   │   ├── proc-macro2 feature "default" (*)
│   │   │   │   │   ├── quote feature "default" (*)
│   │   │   │   │   ├── syn feature "default" (*)
│   │   │   │   │   └── syn feature "full" (*)
│   │   │   │   ├── futures-sink v0.3.31
│   │   │   │   ├── futures-task v0.3.31
│   │   │   │   ├── futures-channel feature "std" (*)
│   │   │   │   ├── futures-io feature "std"
│   │   │   │   │   └── futures-io v0.3.31
│   │   │   │   ├── memchr feature "default"
│   │   │   │   │   ├── memchr v2.7.5
│   │   │   │   │   └── memchr feature "std"
│   │   │   │   │       ├── memchr v2.7.5
│   │   │   │   │       └── memchr feature "alloc"
│   │   │   │   │           └── memchr v2.7.5
│   │   │   │   ├── pin-project-lite feature "default" (*)
│   │   │   │   ├── pin-utils feature "default" (*)
│   │   │   │   └── slab feature "default" (*)
│   │   │   ├── tokio v1.47.1 (*)
│   │   │   ├── bytes feature "default" (*)
│   │   │   ├── futures-channel feature "default" (*)
│   │   │   ├── futures-core feature "default" (*)
│   │   │   ├── pin-project-lite feature "default" (*)
│   │   │   ├── http feature "default" (*)
│   │   │   ├── http-body feature "default" (*)
│   │   │   ├── tower-service feature "default" (*)
│   │   │   ├── hyper feature "default" (*)
│   │   │   ├── libc feature "default" (*)
│   │   │   ├── socket2 feature "all" (*)
│   │   │   ├── socket2 feature "default" (*)
│   │   │   ├── tracing feature "std" (*)
│   │   │   ├── base64 feature "default"
│   │   │   │   ├── base64 v0.22.1
│   │   │   │   └── base64 feature "std"
│   │   │   │       ├── base64 v0.22.1
│   │   │   │       └── base64 feature "alloc"
│   │   │   │           └── base64 v0.22.1
│   │   │   ├── ipnet feature "default"
│   │   │   │   ├── ipnet v2.11.0
│   │   │   │   └── ipnet feature "std"
│   │   │   │       └── ipnet v2.11.0
│   │   │   └── percent-encoding feature "default"
│   │   │       ├── percent-encoding v2.3.2
│   │   │       └── percent-encoding feature "std"
│   │   │           ├── percent-encoding v2.3.2
│   │   │           └── percent-encoding feature "alloc"
│   │   │               └── percent-encoding v2.3.2
... (truncated)
```

</details>

### kameo

<details><summary>Reverse tree (-i kameo -e features)</summary>

```text
```

</details>

<details><summary>Forward tree (-p kameo -e features)</summary>

```text
kameo v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/kameo)
├── anyhow feature "default"
│   ├── anyhow v1.0.99
│   └── anyhow feature "std"
│       └── anyhow v1.0.99
├── tokio feature "default"
│   └── tokio v1.47.1
│       ├── mio v1.0.4
│       │   └── libc feature "default"
│       │       ├── libc v0.2.175
│       │       └── libc feature "std"
│       │           └── libc v0.2.175
│       ├── bytes feature "default"
│       │   ├── bytes v1.10.1
│       │   └── bytes feature "std"
│       │       └── bytes v1.10.1
│       ├── libc feature "default" (*)
│       ├── parking_lot feature "default"
│       │   └── parking_lot v0.12.4
│       │       ├── lock_api feature "default"
│       │       │   ├── lock_api v0.4.13
│       │       │   │   └── scopeguard v1.2.0
│       │       │   │   [build-dependencies]
│       │       │   │   └── autocfg feature "default"
│       │       │   │       └── autocfg v1.5.0
│       │       │   └── lock_api feature "atomic_usize"
│       │       │       └── lock_api v0.4.13 (*)
│       │       └── parking_lot_core feature "default"
│       │           └── parking_lot_core v0.9.11
│       │               ├── libc feature "default" (*)
│       │               ├── cfg-if feature "default"
│       │               │   └── cfg-if v1.0.3
│       │               └── smallvec feature "default"
│       │                   └── smallvec v1.15.1
│       ├── pin-project-lite feature "default"
│       │   └── pin-project-lite v0.2.16
│       ├── signal-hook-registry feature "default"
│       │   └── signal-hook-registry v1.4.6
│       │       └── libc feature "default" (*)
│       ├── socket2 feature "all"
│       │   └── socket2 v0.6.0
│       │       └── libc feature "default" (*)
│       ├── socket2 feature "default"
│       │   └── socket2 v0.6.0 (*)
│       └── tokio-macros feature "default"
│           └── tokio-macros v2.5.0 (proc-macro)
│               ├── proc-macro2 feature "default"
│               │   ├── proc-macro2 v1.0.101
│               │   │   └── unicode-ident feature "default"
│               │   │       └── unicode-ident v1.0.19
│               │   └── proc-macro2 feature "proc-macro"
│               │       └── proc-macro2 v1.0.101 (*)
│               ├── quote feature "default"
│               │   ├── quote v1.0.40
│               │   │   └── proc-macro2 v1.0.101 (*)
│               │   └── quote feature "proc-macro"
│               │       ├── quote v1.0.40 (*)
│               │       └── proc-macro2 feature "proc-macro" (*)
│               ├── syn feature "default"
│               │   ├── syn v2.0.106
│               │   │   ├── proc-macro2 v1.0.101 (*)
│               │   │   ├── quote v1.0.40 (*)
│               │   │   └── unicode-ident feature "default" (*)
│               │   ├── syn feature "clone-impls"
│               │   │   └── syn v2.0.106 (*)
│               │   ├── syn feature "derive"
│               │   │   └── syn v2.0.106 (*)
│               │   ├── syn feature "parsing"
│               │   │   └── syn v2.0.106 (*)
│               │   ├── syn feature "printing"
│               │   │   └── syn v2.0.106 (*)
│               │   └── syn feature "proc-macro"
│               │       ├── syn v2.0.106 (*)
│               │       ├── proc-macro2 feature "proc-macro" (*)
│               │       └── quote feature "proc-macro" (*)
│               └── syn feature "full"
│                   └── syn v2.0.106 (*)
├── tokio feature "macros"
│   ├── tokio v1.47.1 (*)
│   └── tokio feature "tokio-macros"
│       └── tokio v1.47.1 (*)
├── tokio feature "rt"
│   └── tokio v1.47.1 (*)
├── tokio feature "sync"
│   └── tokio v1.47.1 (*)
├── tracing feature "default"
│   ├── tracing v0.1.41
│   │   ├── tracing-core v0.1.34
│   │   │   └── once_cell feature "default"
│   │   │       ├── once_cell v1.21.3
│   │   │       └── once_cell feature "std"
│   │   │           ├── once_cell v1.21.3
│   │   │           └── once_cell feature "alloc"
│   │   │               ├── once_cell v1.21.3
│   │   │               └── once_cell feature "race"
│   │   │                   └── once_cell v1.21.3
│   │   ├── pin-project-lite feature "default" (*)
│   │   └── tracing-attributes feature "default"
│   │       └── tracing-attributes v0.1.30 (proc-macro)
│   │           ├── proc-macro2 feature "default" (*)
│   │           ├── quote feature "default" (*)
│   │           ├── syn feature "clone-impls" (*)
│   │           ├── syn feature "extra-traits"
│   │           │   └── syn v2.0.106 (*)
│   │           ├── syn feature "full" (*)
│   │           ├── syn feature "parsing" (*)
│   │           ├── syn feature "printing" (*)
│   │           ├── syn feature "proc-macro" (*)
│   │           └── syn feature "visit-mut"
│   │               └── syn v2.0.106 (*)
│   ├── tracing feature "attributes"
│   │   ├── tracing v0.1.41 (*)
│   │   └── tracing feature "tracing-attributes"
│   │       └── tracing v0.1.41 (*)
│   └── tracing feature "std"
│       ├── tracing v0.1.41 (*)
│       └── tracing-core feature "std"
│           ├── tracing-core v0.1.34 (*)
│           └── tracing-core feature "once_cell"
│               └── tracing-core v0.1.34 (*)
└── workspace-hack feature "default"
    └── workspace-hack v0.1.0 (/Users/mymac/Desktop/RustyOnions/workspace-hack)
        ├── tokio feature "default" (*)
        ├── tokio feature "full"
        │   ├── tokio v1.47.1 (*)
        │   ├── tokio feature "fs"
        │   │   └── tokio v1.47.1 (*)
        │   ├── tokio feature "io-std"
        │   │   └── tokio v1.47.1 (*)
        │   ├── tokio feature "io-util"
        │   │   ├── tokio v1.47.1 (*)
        │   │   └── tokio feature "bytes"
        │   │       └── tokio v1.47.1 (*)
        │   ├── tokio feature "macros" (*)
        │   ├── tokio feature "net"
        │   │   ├── tokio v1.47.1 (*)
        │   │   ├── tokio feature "libc"
        │   │   │   └── tokio v1.47.1 (*)
        │   │   ├── tokio feature "mio"
        │   │   │   └── tokio v1.47.1 (*)
        │   │   ├── tokio feature "socket2"
        │   │   │   └── tokio v1.47.1 (*)
        │   │   ├── mio feature "net"
        │   │   │   └── mio v1.0.4 (*)
        │   │   ├── mio feature "os-ext"
        │   │   │   ├── mio v1.0.4 (*)
        │   │   │   └── mio feature "os-poll"
        │   │   │       └── mio v1.0.4 (*)
        │   │   └── mio feature "os-poll" (*)
        │   ├── tokio feature "parking_lot"
        │   │   └── tokio v1.47.1 (*)
        │   ├── tokio feature "process"
        │   │   ├── tokio v1.47.1 (*)
        │   │   ├── tokio feature "bytes" (*)
        │   │   ├── tokio feature "libc" (*)
        │   │   ├── tokio feature "mio" (*)
        │   │   ├── tokio feature "signal-hook-registry"
        │   │   │   └── tokio v1.47.1 (*)
        │   │   ├── mio feature "net" (*)
        │   │   ├── mio feature "os-ext" (*)
        │   │   └── mio feature "os-poll" (*)
        │   ├── tokio feature "rt" (*)
        │   ├── tokio feature "rt-multi-thread"
        │   │   ├── tokio v1.47.1 (*)
        │   │   └── tokio feature "rt" (*)
        │   ├── tokio feature "signal"
        │   │   ├── tokio v1.47.1 (*)
        │   │   ├── tokio feature "libc" (*)
        │   │   ├── tokio feature "mio" (*)
        │   │   ├── tokio feature "signal-hook-registry" (*)
        │   │   ├── mio feature "net" (*)
        │   │   ├── mio feature "os-ext" (*)
        │   │   └── mio feature "os-poll" (*)
        │   ├── tokio feature "sync" (*)
        │   └── tokio feature "time"
        │       └── tokio v1.47.1 (*)
        ├── smallvec feature "const_new"
        │   ├── smallvec v1.15.1
        │   └── smallvec feature "const_generics"
        │       └── smallvec v1.15.1
        ├── tracing feature "default" (*)
        ├── tracing-core feature "default"
        │   ├── tracing-core v0.1.34 (*)
        │   └── tracing-core feature "std" (*)
        ├── axum feature "http1"
        │   ├── axum v0.7.9
        │   │   ├── tokio feature "default" (*)
        │   │   ├── tokio feature "time" (*)
        │   │   ├── bytes feature "default" (*)
        │   │   ├── pin-project-lite feature "default" (*)
        │   │   ├── async-trait feature "default"
        │   │   │   └── async-trait v0.1.89 (proc-macro)
        │   │   │       ├── proc-macro2 feature "default" (*)
        │   │   │       ├── quote feature "default" (*)
        │   │   │       ├── syn feature "clone-impls" (*)
        │   │   │       ├── syn feature "full" (*)
        │   │   │       ├── syn feature "parsing" (*)
        │   │   │       ├── syn feature "printing" (*)
        │   │   │       ├── syn feature "proc-macro" (*)
        │   │   │       └── syn feature "visit-mut" (*)
        │   │   ├── axum-core feature "default"
        │   │   │   └── axum-core v0.4.5
        │   │   │       ├── bytes feature "default" (*)
        │   │   │       ├── pin-project-lite feature "default" (*)
        │   │   │       ├── async-trait feature "default" (*)
        │   │   │       ├── futures-util feature "alloc"
        │   │   │       │   ├── futures-util v0.3.31
        │   │   │       │   │   ├── futures-core v0.3.31
        │   │   │       │   │   ├── futures-macro v0.3.31 (proc-macro)
        │   │   │       │   │   │   ├── proc-macro2 feature "default" (*)
        │   │   │       │   │   │   ├── quote feature "default" (*)
        │   │   │       │   │   │   ├── syn feature "default" (*)
        │   │   │       │   │   │   └── syn feature "full" (*)
        │   │   │       │   │   ├── futures-sink v0.3.31
        │   │   │       │   │   ├── futures-task v0.3.31
        │   │   │       │   │   ├── pin-project-lite feature "default" (*)
        │   │   │       │   │   ├── futures-channel feature "std"
        │   │   │       │   │   │   ├── futures-channel v0.3.31
        │   │   │       │   │   │   │   ├── futures-core v0.3.31
        │   │   │       │   │   │   │   └── futures-sink v0.3.31
        │   │   │       │   │   │   ├── futures-channel feature "alloc"
        │   │   │       │   │   │   │   ├── futures-channel v0.3.31 (*)
        │   │   │       │   │   │   │   └── futures-core feature "alloc"
        │   │   │       │   │   │   │       └── futures-core v0.3.31
        │   │   │       │   │   │   └── futures-core feature "std"
        │   │   │       │   │   │       ├── futures-core v0.3.31
        │   │   │       │   │   │       └── futures-core feature "alloc" (*)
        │   │   │       │   │   ├── futures-io feature "std"
        │   │   │       │   │   │   └── futures-io v0.3.31
        │   │   │       │   │   ├── memchr feature "default"
        │   │   │       │   │   │   ├── memchr v2.7.5
        │   │   │       │   │   │   └── memchr feature "std"
        │   │   │       │   │   │       ├── memchr v2.7.5
        │   │   │       │   │   │       └── memchr feature "alloc"
        │   │   │       │   │   │           └── memchr v2.7.5
        │   │   │       │   │   ├── pin-utils feature "default"
        │   │   │       │   │   │   └── pin-utils v0.1.0
        │   │   │       │   │   └── slab feature "default"
        │   │   │       │   │       ├── slab v0.4.11
        │   │   │       │   │       └── slab feature "std"
        │   │   │       │   │           └── slab v0.4.11
        │   │   │       │   ├── futures-core feature "alloc" (*)
        │   │   │       │   └── futures-task feature "alloc"
        │   │   │       │       └── futures-task v0.3.31
        │   │   │       ├── http feature "default"
        │   │   │       │   ├── http v1.3.1
        │   │   │       │   │   ├── bytes feature "default" (*)
        │   │   │       │   │   ├── fnv feature "default"
        │   │   │       │   │   │   ├── fnv v1.0.7
        │   │   │       │   │   │   └── fnv feature "std"
        │   │   │       │   │   │       └── fnv v1.0.7
        │   │   │       │   │   └── itoa feature "default"
        │   │   │       │   │       └── itoa v1.0.15
        │   │   │       │   └── http feature "std"
        │   │   │       │       └── http v1.3.1 (*)
        │   │   │       ├── http-body feature "default"
        │   │   │       │   └── http-body v1.0.1
        │   │   │       │       ├── bytes feature "default" (*)
        │   │   │       │       └── http feature "default" (*)
        │   │   │       ├── http-body-util feature "default"
        │   │   │       │   └── http-body-util v0.1.3
        │   │   │       │       ├── futures-core v0.3.31
        │   │   │       │       ├── bytes feature "default" (*)
        │   │   │       │       ├── pin-project-lite feature "default" (*)
        │   │   │       │       ├── http feature "default" (*)
        │   │   │       │       └── http-body feature "default" (*)
        │   │   │       ├── mime feature "default"
        │   │   │       │   └── mime v0.3.17
        │   │   │       ├── rustversion feature "default"
        │   │   │       │   └── rustversion v1.0.22 (proc-macro)
        │   │   │       ├── sync_wrapper feature "default"
        │   │   │       │   └── sync_wrapper v1.0.2
        │   │   │       │       └── futures-core v0.3.31
        │   │   │       ├── tower-layer feature "default"
        │   │   │       │   └── tower-layer v0.3.3
        │   │   │       └── tower-service feature "default"
        │   │   │           └── tower-service v0.3.3
        │   │   ├── futures-util feature "alloc" (*)
        │   │   ├── memchr feature "default" (*)
        │   │   ├── http feature "default" (*)
        │   │   ├── itoa feature "default" (*)
        │   │   ├── http-body feature "default" (*)
        │   │   ├── http-body-util feature "default" (*)
        │   │   ├── mime feature "default" (*)
        │   │   ├── rustversion feature "default" (*)
        │   │   ├── sync_wrapper feature "default" (*)
        │   │   ├── tower-layer feature "default" (*)
        │   │   ├── tower-service feature "default" (*)
        │   │   ├── axum-macros feature "default"
        │   │   │   └── axum-macros v0.4.2 (proc-macro)
        │   │   │       ├── proc-macro2 feature "default" (*)
        │   │   │       ├── quote feature "default" (*)
        │   │   │       ├── syn feature "default" (*)
        │   │   │       ├── syn feature "extra-traits" (*)
        │   │   │       ├── syn feature "full" (*)
        │   │   │       └── syn feature "parsing" (*)
        │   │   ├── hyper feature "default"
        │   │   │   └── hyper v1.7.0
        │   │   │       ├── tokio feature "default" (*)
        │   │   │       ├── tokio feature "sync" (*)
... (truncated)
```

</details>

### svc-index

<details><summary>Reverse tree (-i svc-index -e features)</summary>

```text
```

</details>

<details><summary>Forward tree (-p svc-index -e features)</summary>

```text
svc-index v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/svc-index)
├── index feature "default"
│   └── index v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/index)
│       ├── anyhow feature "default"
│       │   ├── anyhow v1.0.99
│       │   └── anyhow feature "std"
│       │       └── anyhow v1.0.99
│       ├── bincode feature "default"
│       │   └── bincode v1.3.3
│       │       └── serde feature "default"
│       │           ├── serde v1.0.221
│       │           │   ├── serde_core feature "result"
│       │           │   │   └── serde_core v1.0.221
│       │           │   └── serde_derive feature "default"
│       │           │       └── serde_derive v1.0.221 (proc-macro)
│       │           │           ├── proc-macro2 feature "proc-macro"
│       │           │           │   └── proc-macro2 v1.0.101
│       │           │           │       └── unicode-ident feature "default"
│       │           │           │           └── unicode-ident v1.0.19
│       │           │           ├── quote feature "proc-macro"
│       │           │           │   ├── quote v1.0.40
│       │           │           │   │   └── proc-macro2 v1.0.101 (*)
│       │           │           │   └── proc-macro2 feature "proc-macro" (*)
│       │           │           ├── syn feature "clone-impls"
│       │           │           │   └── syn v2.0.106
│       │           │           │       ├── proc-macro2 v1.0.101 (*)
│       │           │           │       ├── quote v1.0.40 (*)
│       │           │           │       └── unicode-ident feature "default" (*)
│       │           │           ├── syn feature "derive"
│       │           │           │   └── syn v2.0.106 (*)
│       │           │           ├── syn feature "parsing"
│       │           │           │   └── syn v2.0.106 (*)
│       │           │           ├── syn feature "printing"
│       │           │           │   └── syn v2.0.106 (*)
│       │           │           └── syn feature "proc-macro"
│       │           │               ├── syn v2.0.106 (*)
│       │           │               ├── proc-macro2 feature "proc-macro" (*)
│       │           │               └── quote feature "proc-macro" (*)
│       │           └── serde feature "std"
│       │               ├── serde v1.0.221 (*)
│       │               └── serde_core feature "std"
│       │                   └── serde_core v1.0.221
│       ├── serde feature "default" (*)
│       ├── chrono feature "default"
│       │   ├── chrono v0.4.42
│       │   │   ├── num-traits v0.2.19
│       │   │   │   [build-dependencies]
│       │   │   │   └── autocfg feature "default"
│       │   │   │       └── autocfg v1.5.0
│       │   │   ├── iana-time-zone feature "default"
│       │   │   │   └── iana-time-zone v0.1.64
│       │   │   │       └── core-foundation-sys feature "default"
│       │   │   │           ├── core-foundation-sys v0.8.7
│       │   │   │           └── core-foundation-sys feature "link"
│       │   │   │               └── core-foundation-sys v0.8.7
│       │   │   └── iana-time-zone feature "fallback"
│       │   │       └── iana-time-zone v0.1.64 (*)
│       │   ├── chrono feature "clock"
│       │   │   ├── chrono v0.4.42 (*)
│       │   │   ├── chrono feature "iana-time-zone"
│       │   │   │   └── chrono v0.4.42 (*)
│       │   │   ├── chrono feature "now"
│       │   │   │   ├── chrono v0.4.42 (*)
│       │   │   │   └── chrono feature "std"
│       │   │   │       ├── chrono v0.4.42 (*)
│       │   │   │       └── chrono feature "alloc"
│       │   │   │           └── chrono v0.4.42 (*)
│       │   │   └── chrono feature "winapi"
│       │   │       ├── chrono v0.4.42 (*)
│       │   │       └── chrono feature "windows-link"
│       │   │           └── chrono v0.4.42 (*)
│       │   ├── chrono feature "oldtime"
│       │   │   └── chrono v0.4.42 (*)
│       │   ├── chrono feature "std" (*)
│       │   └── chrono feature "wasmbind"
│       │       ├── chrono v0.4.42 (*)
│       │       ├── chrono feature "js-sys"
│       │       │   └── chrono v0.4.42 (*)
│       │       └── chrono feature "wasm-bindgen"
│       │           └── chrono v0.4.42 (*)
│       ├── dunce feature "default"
│       │   └── dunce v1.0.5
│       ├── naming feature "default"
│       │   └── naming v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/naming)
│       │       ├── anyhow feature "default" (*)
│       │       ├── serde feature "default" (*)
│       │       ├── chrono feature "default" (*)
│       │       ├── base64 feature "default"
│       │       │   ├── base64 v0.22.1
│       │       │   └── base64 feature "std"
│       │       │       ├── base64 v0.22.1
│       │       │       └── base64 feature "alloc"
│       │       │           └── base64 v0.22.1
│       │       ├── blake3 feature "default"
│       │       │   ├── blake3 v1.8.2
│       │       │   │   ├── arrayvec v0.7.6
│       │       │   │   ├── constant_time_eq v0.3.1
│       │       │   │   ├── arrayref feature "default"
│       │       │   │   │   └── arrayref v0.3.9
│       │       │   │   └── cfg-if feature "default"
│       │       │   │       └── cfg-if v1.0.3
│       │       │   │   [build-dependencies]
│       │       │   │   └── cc feature "default"
│       │       │   │       └── cc v1.2.37
│       │       │   │           ├── jobserver v0.1.34
│       │       │   │           │   └── libc feature "default"
│       │       │   │           │       ├── libc v0.2.175
│       │       │   │           │       └── libc feature "std"
│       │       │   │           │           └── libc v0.2.175
│       │       │   │           ├── libc v0.2.175
│       │       │   │           ├── find-msvc-tools feature "default"
│       │       │   │           │   └── find-msvc-tools v0.1.1
│       │       │   │           └── shlex feature "default"
│       │       │   │               ├── shlex v1.3.0
│       │       │   │               └── shlex feature "std"
│       │       │   │                   └── shlex v1.3.0
│       │       │   └── blake3 feature "std"
│       │       │       └── blake3 v1.8.2 (*)
│       │       ├── hex feature "default"
│       │       │   ├── hex v0.4.3
│       │       │   └── hex feature "std"
│       │       │       ├── hex v0.4.3
│       │       │       └── hex feature "alloc"
│       │       │           └── hex v0.4.3
│       │       ├── mime feature "default"
│       │       │   └── mime v0.3.17
│       │       ├── mime_guess feature "default"
│       │       │   ├── mime_guess v2.0.5
│       │       │   │   ├── mime feature "default" (*)
│       │       │   │   └── unicase feature "default"
│       │       │   │       └── unicase v2.8.1
│       │       │   │   [build-dependencies]
│       │       │   │   └── unicase feature "default" (*)
│       │       │   └── mime_guess feature "rev-mappings"
│       │       │       └── mime_guess v2.0.5 (*)
│       │       ├── serde_with feature "default"
│       │       │   ├── serde_with v3.14.0
│       │       │   │   ├── serde v1.0.221 (*)
│       │       │   │   ├── serde_derive feature "default" (*)
│       │       │   │   └── serde_with_macros feature "default"
│       │       │   │       └── serde_with_macros v3.14.0 (proc-macro)
│       │       │   │           ├── proc-macro2 feature "default"
│       │       │   │           │   ├── proc-macro2 v1.0.101 (*)
│       │       │   │           │   └── proc-macro2 feature "proc-macro" (*)
│       │       │   │           ├── quote feature "default"
│       │       │   │           │   ├── quote v1.0.40 (*)
│       │       │   │           │   └── quote feature "proc-macro" (*)
│       │       │   │           ├── syn feature "default"
│       │       │   │           │   ├── syn v2.0.106 (*)
│       │       │   │           │   ├── syn feature "clone-impls" (*)
│       │       │   │           │   ├── syn feature "derive" (*)
│       │       │   │           │   ├── syn feature "parsing" (*)
│       │       │   │           │   ├── syn feature "printing" (*)
│       │       │   │           │   └── syn feature "proc-macro" (*)
│       │       │   │           ├── syn feature "extra-traits"
│       │       │   │           │   └── syn v2.0.106 (*)
│       │       │   │           ├── syn feature "full"
│       │       │   │           │   └── syn v2.0.106 (*)
│       │       │   │           ├── syn feature "parsing" (*)
│       │       │   │           └── darling feature "default"
│       │       │   │               ├── darling v0.20.11
│       │       │   │               │   ├── darling_core feature "default"
│       │       │   │               │   │   └── darling_core v0.20.11
│       │       │   │               │   │       ├── proc-macro2 feature "default" (*)
│       │       │   │               │   │       ├── quote feature "default" (*)
│       │       │   │               │   │       ├── syn feature "default" (*)
│       │       │   │               │   │       ├── syn feature "extra-traits" (*)
│       │       │   │               │   │       ├── syn feature "full" (*)
│       │       │   │               │   │       ├── fnv feature "default"
│       │       │   │               │   │       │   ├── fnv v1.0.7
│       │       │   │               │   │       │   └── fnv feature "std"
│       │       │   │               │   │       │       └── fnv v1.0.7
│       │       │   │               │   │       ├── ident_case feature "default"
│       │       │   │               │   │       │   └── ident_case v1.0.1
│       │       │   │               │   │       └── strsim feature "default"
│       │       │   │               │   │           └── strsim v0.11.1
│       │       │   │               │   └── darling_macro feature "default"
│       │       │   │               │       └── darling_macro v0.20.11 (proc-macro)
│       │       │   │               │           ├── quote feature "default" (*)
│       │       │   │               │           ├── syn feature "default" (*)
│       │       │   │               │           └── darling_core feature "default" (*)
│       │       │   │               └── darling feature "suggestions"
│       │       │   │                   ├── darling v0.20.11 (*)
│       │       │   │                   └── darling_core feature "suggestions"
│       │       │   │                       ├── darling_core v0.20.11 (*)
│       │       │   │                       └── darling_core feature "strsim"
│       │       │   │                           └── darling_core v0.20.11 (*)
│       │       │   ├── serde_with feature "macros"
│       │       │   │   └── serde_with v3.14.0 (*)
│       │       │   └── serde_with feature "std"
│       │       │       ├── serde_with v3.14.0 (*)
│       │       │       ├── serde feature "std" (*)
│       │       │       └── serde_with feature "alloc"
│       │       │           ├── serde_with v3.14.0 (*)
│       │       │           └── serde feature "alloc"
│       │       │               ├── serde v1.0.221 (*)
│       │       │               └── serde_core feature "alloc"
│       │       │                   └── serde_core v1.0.221
│       │       ├── thiserror feature "default"
│       │       │   └── thiserror v1.0.69
│       │       │       └── thiserror-impl feature "default"
│       │       │           └── thiserror-impl v1.0.69 (proc-macro)
│       │       │               ├── proc-macro2 feature "default" (*)
│       │       │               ├── quote feature "default" (*)
│       │       │               └── syn feature "default" (*)
│       │       ├── toml feature "default"
│       │       │   ├── toml v0.8.23
│       │       │   │   ├── serde feature "default" (*)
│       │       │   │   ├── serde_spanned feature "default"
│       │       │   │   │   └── serde_spanned v0.6.9
│       │       │   │   │       └── serde feature "default" (*)
│       │       │   │   ├── serde_spanned feature "serde"
│       │       │   │   │   └── serde_spanned v0.6.9 (*)
│       │       │   │   ├── toml_datetime feature "default"
│       │       │   │   │   └── toml_datetime v0.6.11
│       │       │   │   │       └── serde feature "default" (*)
│       │       │   │   ├── toml_datetime feature "serde"
│       │       │   │   │   └── toml_datetime v0.6.11 (*)
│       │       │   │   └── toml_edit feature "serde"
│       │       │   │       ├── toml_edit v0.22.27
│       │       │   │       │   ├── serde feature "default" (*)
│       │       │   │       │   ├── serde_spanned feature "default" (*)
│       │       │   │       │   ├── serde_spanned feature "serde" (*)
│       │       │   │       │   ├── toml_datetime feature "default" (*)
│       │       │   │       │   ├── indexmap feature "default"
│       │       │   │       │   │   ├── indexmap v2.11.1
│       │       │   │       │   │   │   ├── equivalent v1.0.2
│       │       │   │       │   │   │   └── hashbrown v0.15.5
│       │       │   │       │   │   │       ├── equivalent v1.0.2
│       │       │   │       │   │   │       ├── foldhash v0.1.5
│       │       │   │       │   │   │       └── allocator-api2 feature "alloc"
│       │       │   │       │   │   │           └── allocator-api2 v0.2.21
│       │       │   │       │   │   └── indexmap feature "std"
│       │       │   │       │   │       └── indexmap v2.11.1 (*)
│       │       │   │       │   ├── indexmap feature "std" (*)
│       │       │   │       │   ├── toml_write feature "default"
│       │       │   │       │   │   ├── toml_write v0.1.2
│       │       │   │       │   │   └── toml_write feature "std"
│       │       │   │       │   │       ├── toml_write v0.1.2
│       │       │   │       │   │       └── toml_write feature "alloc"
│       │       │   │       │   │           └── toml_write v0.1.2
│       │       │   │       │   └── winnow feature "default"
│       │       │   │       │       ├── winnow v0.7.13
│       │       │   │       │       └── winnow feature "std"
│       │       │   │       │           ├── winnow v0.7.13
│       │       │   │       │           └── winnow feature "alloc"
│       │       │   │       │               └── winnow v0.7.13
│       │       │   │       └── toml_datetime feature "serde" (*)
│       │       │   ├── toml feature "display"
│       │       │   │   ├── toml v0.8.23 (*)
│       │       │   │   └── toml_edit feature "display"
│       │       │   │       └── toml_edit v0.22.27 (*)
│       │       │   └── toml feature "parse"
│       │       │       ├── toml v0.8.23 (*)
│       │       │       └── toml_edit feature "parse"
│       │       │           └── toml_edit v0.22.27 (*)
│       │       ├── uuid feature "default"
│       │       │   ├── uuid v1.18.1
│       │       │   │   ├── serde v1.0.221 (*)
│       │       │   │   └── getrandom feature "default"
│       │       │   │       └── getrandom v0.3.3
│       │       │   │           ├── libc v0.2.175
│       │       │   │           └── cfg-if feature "default" (*)
│       │       │   └── uuid feature "std"
│       │       │       └── uuid v1.18.1 (*)
│       │       ├── uuid feature "serde"
│       │       │   └── uuid v1.18.1 (*)
│       │       ├── uuid feature "v4"
│       │       │   ├── uuid v1.18.1 (*)
│       │       │   └── uuid feature "rng"
│       │       │       └── uuid v1.18.1 (*)
│       │       └── workspace-hack feature "default"
│       │           └── workspace-hack v0.1.0 (/Users/mymac/Desktop/RustyOnions/workspace-hack)
│       │               ├── serde feature "alloc" (*)
│       │               ├── serde feature "default" (*)
│       │               ├── serde feature "derive"
│       │               │   ├── serde v1.0.221 (*)
│       │               │   └── serde feature "serde_derive"
│       │               │       └── serde v1.0.221 (*)
│       │               ├── serde_core feature "alloc" (*)
│       │               ├── serde_core feature "result" (*)
│       │               ├── serde_core feature "std" (*)
│       │               ├── num-traits feature "std"
│       │               │   └── num-traits v0.2.19 (*)
│       │               ├── hashbrown feature "default"
│       │               │   ├── hashbrown v0.15.5 (*)
│       │               │   ├── hashbrown feature "allocator-api2"
│       │               │   │   └── hashbrown v0.15.5 (*)
│       │               │   ├── hashbrown feature "default-hasher"
│       │               │   │   └── hashbrown v0.15.5 (*)
│       │               │   ├── hashbrown feature "equivalent"
│       │               │   │   └── hashbrown v0.15.5 (*)
│       │               │   ├── hashbrown feature "inline-more"
│       │               │   │   └── hashbrown v0.15.5 (*)
│       │               │   └── hashbrown feature "raw-entry"
│       │               │       └── hashbrown v0.15.5 (*)
│       │               ├── axum feature "http1"
│       │               │   ├── axum v0.7.9
│       │               │   │   ├── serde feature "default" (*)
│       │               │   │   ├── mime feature "default" (*)
... (truncated)
```

</details>

### svc-overlay

<details><summary>Reverse tree (-i svc-overlay -e features)</summary>

```text
```

</details>

<details><summary>Forward tree (-p svc-overlay -e features)</summary>

```text
svc-overlay v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/svc-overlay)
├── anyhow feature "default"
│   ├── anyhow v1.0.99
│   └── anyhow feature "std"
│       └── anyhow v1.0.99
├── rmp-serde feature "default"
│   └── rmp-serde v1.3.0
│       ├── byteorder feature "default"
│       │   ├── byteorder v1.5.0
│       │   └── byteorder feature "std"
│       │       └── byteorder v1.5.0
│       ├── rmp feature "default"
│       │   ├── rmp v0.8.14
│       │   │   ├── byteorder v1.5.0
│       │   │   ├── num-traits v0.2.19
│       │   │   │   [build-dependencies]
│       │   │   │   └── autocfg feature "default"
│       │   │   │       └── autocfg v1.5.0
│       │   │   └── paste feature "default"
│       │   │       └── paste v1.0.15 (proc-macro)
│       │   └── rmp feature "std"
│       │       ├── rmp v0.8.14 (*)
│       │       ├── byteorder feature "std" (*)
│       │       └── num-traits feature "std"
│       │           └── num-traits v0.2.19 (*)
│       └── serde feature "default"
│           ├── serde v1.0.221
│           │   ├── serde_core feature "result"
│           │   │   └── serde_core v1.0.221
│           │   └── serde_derive feature "default"
│           │       └── serde_derive v1.0.221 (proc-macro)
│           │           ├── proc-macro2 feature "proc-macro"
│           │           │   └── proc-macro2 v1.0.101
│           │           │       └── unicode-ident feature "default"
│           │           │           └── unicode-ident v1.0.19
│           │           ├── quote feature "proc-macro"
│           │           │   ├── quote v1.0.40
│           │           │   │   └── proc-macro2 v1.0.101 (*)
│           │           │   └── proc-macro2 feature "proc-macro" (*)
│           │           ├── syn feature "clone-impls"
│           │           │   └── syn v2.0.106
│           │           │       ├── proc-macro2 v1.0.101 (*)
│           │           │       ├── quote v1.0.40 (*)
│           │           │       └── unicode-ident feature "default" (*)
│           │           ├── syn feature "derive"
│           │           │   └── syn v2.0.106 (*)
│           │           ├── syn feature "parsing"
│           │           │   └── syn v2.0.106 (*)
│           │           ├── syn feature "printing"
│           │           │   └── syn v2.0.106 (*)
│           │           └── syn feature "proc-macro"
│           │               ├── syn v2.0.106 (*)
│           │               ├── proc-macro2 feature "proc-macro" (*)
│           │               └── quote feature "proc-macro" (*)
│           └── serde feature "std"
│               ├── serde v1.0.221 (*)
│               └── serde_core feature "std"
│                   └── serde_core v1.0.221
├── serde feature "default" (*)
├── ron-bus feature "default"
│   └── ron-bus v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/ron-bus)
│       ├── rmp-serde feature "default" (*)
│       ├── serde feature "default" (*)
│       ├── thiserror feature "default"
│       │   └── thiserror v1.0.69
│       │       └── thiserror-impl feature "default"
│       │           └── thiserror-impl v1.0.69 (proc-macro)
│       │               ├── proc-macro2 feature "default"
│       │               │   ├── proc-macro2 v1.0.101 (*)
│       │               │   └── proc-macro2 feature "proc-macro" (*)
│       │               ├── quote feature "default"
│       │               │   ├── quote v1.0.40 (*)
│       │               │   └── quote feature "proc-macro" (*)
│       │               └── syn feature "default"
│       │                   ├── syn v2.0.106 (*)
│       │                   ├── syn feature "clone-impls" (*)
│       │                   ├── syn feature "derive" (*)
│       │                   ├── syn feature "parsing" (*)
│       │                   ├── syn feature "printing" (*)
│       │                   └── syn feature "proc-macro" (*)
│       └── workspace-hack feature "default"
│           └── workspace-hack v0.1.0 (/Users/mymac/Desktop/RustyOnions/workspace-hack)
│               ├── num-traits feature "std" (*)
│               ├── serde feature "alloc"
│               │   ├── serde v1.0.221 (*)
│               │   └── serde_core feature "alloc"
│               │       └── serde_core v1.0.221
│               ├── serde feature "default" (*)
│               ├── serde feature "derive"
│               │   ├── serde v1.0.221 (*)
│               │   └── serde feature "serde_derive"
│               │       └── serde v1.0.221 (*)
│               ├── serde_core feature "alloc" (*)
│               ├── serde_core feature "result" (*)
│               ├── serde_core feature "std" (*)
│               ├── axum feature "http1"
│               │   ├── axum v0.7.9
│               │   │   ├── serde feature "default" (*)
│               │   │   ├── async-trait feature "default"
│               │   │   │   └── async-trait v0.1.89 (proc-macro)
│               │   │   │       ├── proc-macro2 feature "default" (*)
│               │   │   │       ├── quote feature "default" (*)
│               │   │   │       ├── syn feature "clone-impls" (*)
│               │   │   │       ├── syn feature "full"
│               │   │   │       │   └── syn v2.0.106 (*)
│               │   │   │       ├── syn feature "parsing" (*)
│               │   │   │       ├── syn feature "printing" (*)
│               │   │   │       ├── syn feature "proc-macro" (*)
│               │   │   │       └── syn feature "visit-mut"
│               │   │   │           └── syn v2.0.106 (*)
│               │   │   ├── axum-core feature "default"
│               │   │   │   └── axum-core v0.4.5
│               │   │   │       ├── async-trait feature "default" (*)
│               │   │   │       ├── bytes feature "default"
│               │   │   │       │   ├── bytes v1.10.1
│               │   │   │       │   └── bytes feature "std"
│               │   │   │       │       └── bytes v1.10.1
│               │   │   │       ├── futures-util feature "alloc"
│               │   │   │       │   ├── futures-util v0.3.31
│               │   │   │       │   │   ├── futures-core v0.3.31
│               │   │   │       │   │   ├── futures-macro v0.3.31 (proc-macro)
│               │   │   │       │   │   │   ├── proc-macro2 feature "default" (*)
│               │   │   │       │   │   │   ├── quote feature "default" (*)
│               │   │   │       │   │   │   ├── syn feature "default" (*)
│               │   │   │       │   │   │   └── syn feature "full" (*)
│               │   │   │       │   │   ├── futures-sink v0.3.31
│               │   │   │       │   │   ├── futures-task v0.3.31
│               │   │   │       │   │   ├── futures-channel feature "std"
│               │   │   │       │   │   │   ├── futures-channel v0.3.31
│               │   │   │       │   │   │   │   ├── futures-core v0.3.31
│               │   │   │       │   │   │   │   └── futures-sink v0.3.31
│               │   │   │       │   │   │   ├── futures-channel feature "alloc"
│               │   │   │       │   │   │   │   ├── futures-channel v0.3.31 (*)
│               │   │   │       │   │   │   │   └── futures-core feature "alloc"
│               │   │   │       │   │   │   │       └── futures-core v0.3.31
│               │   │   │       │   │   │   └── futures-core feature "std"
│               │   │   │       │   │   │       ├── futures-core v0.3.31
│               │   │   │       │   │   │       └── futures-core feature "alloc" (*)
│               │   │   │       │   │   ├── futures-io feature "std"
│               │   │   │       │   │   │   └── futures-io v0.3.31
│               │   │   │       │   │   ├── memchr feature "default"
│               │   │   │       │   │   │   ├── memchr v2.7.5
│               │   │   │       │   │   │   └── memchr feature "std"
│               │   │   │       │   │   │       ├── memchr v2.7.5
│               │   │   │       │   │   │       └── memchr feature "alloc"
│               │   │   │       │   │   │           └── memchr v2.7.5
│               │   │   │       │   │   ├── pin-project-lite feature "default"
│               │   │   │       │   │   │   └── pin-project-lite v0.2.16
│               │   │   │       │   │   ├── pin-utils feature "default"
│               │   │   │       │   │   │   └── pin-utils v0.1.0
│               │   │   │       │   │   └── slab feature "default"
│               │   │   │       │   │       ├── slab v0.4.11
│               │   │   │       │   │       └── slab feature "std"
│               │   │   │       │   │           └── slab v0.4.11
│               │   │   │       │   ├── futures-core feature "alloc" (*)
│               │   │   │       │   └── futures-task feature "alloc"
│               │   │   │       │       └── futures-task v0.3.31
│               │   │   │       ├── pin-project-lite feature "default" (*)
│               │   │   │       ├── http feature "default"
│               │   │   │       │   ├── http v1.3.1
│               │   │   │       │   │   ├── bytes feature "default" (*)
│               │   │   │       │   │   ├── fnv feature "default"
│               │   │   │       │   │   │   ├── fnv v1.0.7
│               │   │   │       │   │   │   └── fnv feature "std"
│               │   │   │       │   │   │       └── fnv v1.0.7
│               │   │   │       │   │   └── itoa feature "default"
│               │   │   │       │   │       └── itoa v1.0.15
│               │   │   │       │   └── http feature "std"
│               │   │   │       │       └── http v1.3.1 (*)
│               │   │   │       ├── http-body feature "default"
│               │   │   │       │   └── http-body v1.0.1
│               │   │   │       │       ├── bytes feature "default" (*)
│               │   │   │       │       └── http feature "default" (*)
│               │   │   │       ├── http-body-util feature "default"
│               │   │   │       │   └── http-body-util v0.1.3
│               │   │   │       │       ├── futures-core v0.3.31
│               │   │   │       │       ├── bytes feature "default" (*)
│               │   │   │       │       ├── pin-project-lite feature "default" (*)
│               │   │   │       │       ├── http feature "default" (*)
│               │   │   │       │       └── http-body feature "default" (*)
│               │   │   │       ├── mime feature "default"
│               │   │   │       │   └── mime v0.3.17
│               │   │   │       ├── rustversion feature "default"
│               │   │   │       │   └── rustversion v1.0.22 (proc-macro)
│               │   │   │       ├── sync_wrapper feature "default"
│               │   │   │       │   └── sync_wrapper v1.0.2
│               │   │   │       │       └── futures-core v0.3.31
│               │   │   │       ├── tower-layer feature "default"
│               │   │   │       │   └── tower-layer v0.3.3
│               │   │   │       └── tower-service feature "default"
│               │   │   │           └── tower-service v0.3.3
│               │   │   ├── bytes feature "default" (*)
│               │   │   ├── futures-util feature "alloc" (*)
│               │   │   ├── memchr feature "default" (*)
│               │   │   ├── pin-project-lite feature "default" (*)
│               │   │   ├── http feature "default" (*)
│               │   │   ├── itoa feature "default" (*)
│               │   │   ├── http-body feature "default" (*)
│               │   │   ├── http-body-util feature "default" (*)
│               │   │   ├── mime feature "default" (*)
│               │   │   ├── rustversion feature "default" (*)
│               │   │   ├── sync_wrapper feature "default" (*)
│               │   │   ├── tower-layer feature "default" (*)
│               │   │   ├── tower-service feature "default" (*)
│               │   │   ├── axum-macros feature "default"
│               │   │   │   └── axum-macros v0.4.2 (proc-macro)
│               │   │   │       ├── proc-macro2 feature "default" (*)
│               │   │   │       ├── quote feature "default" (*)
│               │   │   │       ├── syn feature "default" (*)
│               │   │   │       ├── syn feature "extra-traits"
│               │   │   │       │   └── syn v2.0.106 (*)
│               │   │   │       ├── syn feature "full" (*)
│               │   │   │       └── syn feature "parsing" (*)
│               │   │   ├── hyper feature "default"
│               │   │   │   └── hyper v1.7.0
│               │   │   │       ├── bytes feature "default" (*)
│               │   │   │       ├── futures-channel feature "default"
│               │   │   │       │   ├── futures-channel v0.3.31 (*)
│               │   │   │       │   └── futures-channel feature "std" (*)
│               │   │   │       ├── futures-core feature "default"
│               │   │   │       │   ├── futures-core v0.3.31
│               │   │   │       │   └── futures-core feature "std" (*)
│               │   │   │       ├── pin-project-lite feature "default" (*)
│               │   │   │       ├── pin-utils feature "default" (*)
│               │   │   │       ├── http feature "default" (*)
│               │   │   │       ├── itoa feature "default" (*)
│               │   │   │       ├── http-body feature "default" (*)
│               │   │   │       ├── atomic-waker feature "default"
│               │   │   │       │   └── atomic-waker v1.1.2
│               │   │   │       ├── h2 feature "default"
│               │   │   │       │   └── h2 v0.4.12
│               │   │   │       │       ├── futures-core v0.3.31
│               │   │   │       │       ├── futures-sink v0.3.31
│               │   │   │       │       ├── bytes feature "default" (*)
│               │   │   │       │       ├── slab feature "default" (*)
│               │   │   │       │       ├── http feature "default" (*)
│               │   │   │       │       ├── fnv feature "default" (*)
│               │   │   │       │       ├── atomic-waker feature "default" (*)
│               │   │   │       │       ├── indexmap feature "default"
│               │   │   │       │       │   ├── indexmap v2.11.1
│               │   │   │       │       │   │   ├── equivalent v1.0.2
│               │   │   │       │       │   │   └── hashbrown v0.15.5
│               │   │   │       │       │   │       ├── equivalent v1.0.2
│               │   │   │       │       │   │       ├── foldhash v0.1.5
│               │   │   │       │       │   │       └── allocator-api2 feature "alloc"
│               │   │   │       │       │   │           └── allocator-api2 v0.2.21
│               │   │   │       │       │   └── indexmap feature "std"
│               │   │   │       │       │       └── indexmap v2.11.1 (*)
│               │   │   │       │       ├── indexmap feature "std" (*)
│               │   │   │       │       ├── tokio feature "default"
│               │   │   │       │       │   └── tokio v1.47.1
│               │   │   │       │       │       ├── mio v1.0.4
│               │   │   │       │       │       │   └── libc feature "default"
│               │   │   │       │       │       │       ├── libc v0.2.175
│               │   │   │       │       │       │       └── libc feature "std"
│               │   │   │       │       │       │           └── libc v0.2.175
│               │   │   │       │       │       ├── bytes feature "default" (*)
│               │   │   │       │       │       ├── pin-project-lite feature "default" (*)
│               │   │   │       │       │       ├── libc feature "default" (*)
│               │   │   │       │       │       ├── parking_lot feature "default"
│               │   │   │       │       │       │   └── parking_lot v0.12.4
│               │   │   │       │       │       │       ├── lock_api feature "default"
│               │   │   │       │       │       │       │   ├── lock_api v0.4.13
│               │   │   │       │       │       │       │   │   └── scopeguard v1.2.0
│               │   │   │       │       │       │       │   │   [build-dependencies]
│               │   │   │       │       │       │       │   │   └── autocfg feature "default" (*)
│               │   │   │       │       │       │       │   └── lock_api feature "atomic_usize"
│               │   │   │       │       │       │       │       └── lock_api v0.4.13 (*)
│               │   │   │       │       │       │       └── parking_lot_core feature "default"
│               │   │   │       │       │       │           └── parking_lot_core v0.9.11
│               │   │   │       │       │       │               ├── libc feature "default" (*)
│               │   │   │       │       │       │               ├── cfg-if feature "default"
│               │   │   │       │       │       │               │   └── cfg-if v1.0.3
│               │   │   │       │       │       │               └── smallvec feature "default"
│               │   │   │       │       │       │                   └── smallvec v1.15.1
│               │   │   │       │       │       ├── signal-hook-registry feature "default"
│               │   │   │       │       │       │   └── signal-hook-registry v1.4.6
│               │   │   │       │       │       │       └── libc feature "default" (*)
│               │   │   │       │       │       ├── socket2 feature "all"
│               │   │   │       │       │       │   └── socket2 v0.6.0
│               │   │   │       │       │       │       └── libc feature "default" (*)
│               │   │   │       │       │       ├── socket2 feature "default"
│               │   │   │       │       │       │   └── socket2 v0.6.0 (*)
│               │   │   │       │       │       └── tokio-macros feature "default"
│               │   │   │       │       │           └── tokio-macros v2.5.0 (proc-macro)
│               │   │   │       │       │               ├── proc-macro2 feature "default" (*)
│               │   │   │       │       │               ├── quote feature "default" (*)
│               │   │   │       │       │               ├── syn feature "default" (*)
│               │   │   │       │       │               └── syn feature "full" (*)
│               │   │   │       │       ├── tokio feature "io-util"
│               │   │   │       │       │   ├── tokio v1.47.1 (*)
│               │   │   │       │       │   └── tokio feature "bytes"
│               │   │   │       │       │       └── tokio v1.47.1 (*)
│               │   │   │       │       ├── tokio-util feature "codec"
│               │   │   │       │       │   └── tokio-util v0.7.16
│               │   │   │       │       │       ├── bytes feature "default" (*)
│               │   │   │       │       │       ├── futures-core feature "default" (*)
│               │   │   │       │       │       ├── futures-sink feature "default"
│               │   │   │       │       │       │   ├── futures-sink v0.3.31
│               │   │   │       │       │       │   └── futures-sink feature "std"
... (truncated)
```

</details>

### svc-storage

<details><summary>Reverse tree (-i svc-storage -e features)</summary>

```text
```

</details>

<details><summary>Forward tree (-p svc-storage -e features)</summary>

```text
svc-storage v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/svc-storage)
├── anyhow feature "default"
│   ├── anyhow v1.0.99
│   └── anyhow feature "std"
│       └── anyhow v1.0.99
├── rmp-serde feature "default"
│   └── rmp-serde v1.3.0
│       ├── byteorder feature "default"
│       │   ├── byteorder v1.5.0
│       │   └── byteorder feature "std"
│       │       └── byteorder v1.5.0
│       ├── rmp feature "default"
│       │   ├── rmp v0.8.14
│       │   │   ├── byteorder v1.5.0
│       │   │   ├── num-traits v0.2.19
│       │   │   │   [build-dependencies]
│       │   │   │   └── autocfg feature "default"
│       │   │   │       └── autocfg v1.5.0
│       │   │   └── paste feature "default"
│       │   │       └── paste v1.0.15 (proc-macro)
│       │   └── rmp feature "std"
│       │       ├── rmp v0.8.14 (*)
│       │       ├── byteorder feature "std" (*)
│       │       └── num-traits feature "std"
│       │           └── num-traits v0.2.19 (*)
│       └── serde feature "default"
│           ├── serde v1.0.221
│           │   ├── serde_core feature "result"
│           │   │   └── serde_core v1.0.221
│           │   └── serde_derive feature "default"
│           │       └── serde_derive v1.0.221 (proc-macro)
│           │           ├── proc-macro2 feature "proc-macro"
│           │           │   └── proc-macro2 v1.0.101
│           │           │       └── unicode-ident feature "default"
│           │           │           └── unicode-ident v1.0.19
│           │           ├── quote feature "proc-macro"
│           │           │   ├── quote v1.0.40
│           │           │   │   └── proc-macro2 v1.0.101 (*)
│           │           │   └── proc-macro2 feature "proc-macro" (*)
│           │           ├── syn feature "clone-impls"
│           │           │   └── syn v2.0.106
│           │           │       ├── proc-macro2 v1.0.101 (*)
│           │           │       ├── quote v1.0.40 (*)
│           │           │       └── unicode-ident feature "default" (*)
│           │           ├── syn feature "derive"
│           │           │   └── syn v2.0.106 (*)
│           │           ├── syn feature "parsing"
│           │           │   └── syn v2.0.106 (*)
│           │           ├── syn feature "printing"
│           │           │   └── syn v2.0.106 (*)
│           │           └── syn feature "proc-macro"
│           │               ├── syn v2.0.106 (*)
│           │               ├── proc-macro2 feature "proc-macro" (*)
│           │               └── quote feature "proc-macro" (*)
│           └── serde feature "std"
│               ├── serde v1.0.221 (*)
│               └── serde_core feature "std"
│                   └── serde_core v1.0.221
├── serde feature "default" (*)
├── ron-bus feature "default"
│   └── ron-bus v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/ron-bus)
│       ├── rmp-serde feature "default" (*)
│       ├── serde feature "default" (*)
│       ├── thiserror feature "default"
│       │   └── thiserror v1.0.69
│       │       └── thiserror-impl feature "default"
│       │           └── thiserror-impl v1.0.69 (proc-macro)
│       │               ├── proc-macro2 feature "default"
│       │               │   ├── proc-macro2 v1.0.101 (*)
│       │               │   └── proc-macro2 feature "proc-macro" (*)
│       │               ├── quote feature "default"
│       │               │   ├── quote v1.0.40 (*)
│       │               │   └── quote feature "proc-macro" (*)
│       │               └── syn feature "default"
│       │                   ├── syn v2.0.106 (*)
│       │                   ├── syn feature "clone-impls" (*)
│       │                   ├── syn feature "derive" (*)
│       │                   ├── syn feature "parsing" (*)
│       │                   ├── syn feature "printing" (*)
│       │                   └── syn feature "proc-macro" (*)
│       └── workspace-hack feature "default"
│           └── workspace-hack v0.1.0 (/Users/mymac/Desktop/RustyOnions/workspace-hack)
│               ├── num-traits feature "std" (*)
│               ├── serde feature "alloc"
│               │   ├── serde v1.0.221 (*)
│               │   └── serde_core feature "alloc"
│               │       └── serde_core v1.0.221
│               ├── serde feature "default" (*)
│               ├── serde feature "derive"
│               │   ├── serde v1.0.221 (*)
│               │   └── serde feature "serde_derive"
│               │       └── serde v1.0.221 (*)
│               ├── serde_core feature "alloc" (*)
│               ├── serde_core feature "result" (*)
│               ├── serde_core feature "std" (*)
│               ├── axum feature "http1"
│               │   ├── axum v0.7.9
│               │   │   ├── serde feature "default" (*)
│               │   │   ├── async-trait feature "default"
│               │   │   │   └── async-trait v0.1.89 (proc-macro)
│               │   │   │       ├── proc-macro2 feature "default" (*)
│               │   │   │       ├── quote feature "default" (*)
│               │   │   │       ├── syn feature "clone-impls" (*)
│               │   │   │       ├── syn feature "full"
│               │   │   │       │   └── syn v2.0.106 (*)
│               │   │   │       ├── syn feature "parsing" (*)
│               │   │   │       ├── syn feature "printing" (*)
│               │   │   │       ├── syn feature "proc-macro" (*)
│               │   │   │       └── syn feature "visit-mut"
│               │   │   │           └── syn v2.0.106 (*)
│               │   │   ├── axum-core feature "default"
│               │   │   │   └── axum-core v0.4.5
│               │   │   │       ├── async-trait feature "default" (*)
│               │   │   │       ├── bytes feature "default"
│               │   │   │       │   ├── bytes v1.10.1
│               │   │   │       │   └── bytes feature "std"
│               │   │   │       │       └── bytes v1.10.1
│               │   │   │       ├── futures-util feature "alloc"
│               │   │   │       │   ├── futures-util v0.3.31
│               │   │   │       │   │   ├── futures-core v0.3.31
│               │   │   │       │   │   ├── futures-macro v0.3.31 (proc-macro)
│               │   │   │       │   │   │   ├── proc-macro2 feature "default" (*)
│               │   │   │       │   │   │   ├── quote feature "default" (*)
│               │   │   │       │   │   │   ├── syn feature "default" (*)
│               │   │   │       │   │   │   └── syn feature "full" (*)
│               │   │   │       │   │   ├── futures-sink v0.3.31
│               │   │   │       │   │   ├── futures-task v0.3.31
│               │   │   │       │   │   ├── futures-channel feature "std"
│               │   │   │       │   │   │   ├── futures-channel v0.3.31
│               │   │   │       │   │   │   │   ├── futures-core v0.3.31
│               │   │   │       │   │   │   │   └── futures-sink v0.3.31
│               │   │   │       │   │   │   ├── futures-channel feature "alloc"
│               │   │   │       │   │   │   │   ├── futures-channel v0.3.31 (*)
│               │   │   │       │   │   │   │   └── futures-core feature "alloc"
│               │   │   │       │   │   │   │       └── futures-core v0.3.31
│               │   │   │       │   │   │   └── futures-core feature "std"
│               │   │   │       │   │   │       ├── futures-core v0.3.31
│               │   │   │       │   │   │       └── futures-core feature "alloc" (*)
│               │   │   │       │   │   ├── futures-io feature "std"
│               │   │   │       │   │   │   └── futures-io v0.3.31
│               │   │   │       │   │   ├── memchr feature "default"
│               │   │   │       │   │   │   ├── memchr v2.7.5
│               │   │   │       │   │   │   └── memchr feature "std"
│               │   │   │       │   │   │       ├── memchr v2.7.5
│               │   │   │       │   │   │       └── memchr feature "alloc"
│               │   │   │       │   │   │           └── memchr v2.7.5
│               │   │   │       │   │   ├── pin-project-lite feature "default"
│               │   │   │       │   │   │   └── pin-project-lite v0.2.16
│               │   │   │       │   │   ├── pin-utils feature "default"
│               │   │   │       │   │   │   └── pin-utils v0.1.0
│               │   │   │       │   │   └── slab feature "default"
│               │   │   │       │   │       ├── slab v0.4.11
│               │   │   │       │   │       └── slab feature "std"
│               │   │   │       │   │           └── slab v0.4.11
│               │   │   │       │   ├── futures-core feature "alloc" (*)
│               │   │   │       │   └── futures-task feature "alloc"
│               │   │   │       │       └── futures-task v0.3.31
│               │   │   │       ├── pin-project-lite feature "default" (*)
│               │   │   │       ├── http feature "default"
│               │   │   │       │   ├── http v1.3.1
│               │   │   │       │   │   ├── bytes feature "default" (*)
│               │   │   │       │   │   ├── fnv feature "default"
│               │   │   │       │   │   │   ├── fnv v1.0.7
│               │   │   │       │   │   │   └── fnv feature "std"
│               │   │   │       │   │   │       └── fnv v1.0.7
│               │   │   │       │   │   └── itoa feature "default"
│               │   │   │       │   │       └── itoa v1.0.15
│               │   │   │       │   └── http feature "std"
│               │   │   │       │       └── http v1.3.1 (*)
│               │   │   │       ├── http-body feature "default"
│               │   │   │       │   └── http-body v1.0.1
│               │   │   │       │       ├── bytes feature "default" (*)
│               │   │   │       │       └── http feature "default" (*)
│               │   │   │       ├── http-body-util feature "default"
│               │   │   │       │   └── http-body-util v0.1.3
│               │   │   │       │       ├── futures-core v0.3.31
│               │   │   │       │       ├── bytes feature "default" (*)
│               │   │   │       │       ├── pin-project-lite feature "default" (*)
│               │   │   │       │       ├── http feature "default" (*)
│               │   │   │       │       └── http-body feature "default" (*)
│               │   │   │       ├── mime feature "default"
│               │   │   │       │   └── mime v0.3.17
│               │   │   │       ├── rustversion feature "default"
│               │   │   │       │   └── rustversion v1.0.22 (proc-macro)
│               │   │   │       ├── sync_wrapper feature "default"
│               │   │   │       │   └── sync_wrapper v1.0.2
│               │   │   │       │       └── futures-core v0.3.31
│               │   │   │       ├── tower-layer feature "default"
│               │   │   │       │   └── tower-layer v0.3.3
│               │   │   │       └── tower-service feature "default"
│               │   │   │           └── tower-service v0.3.3
│               │   │   ├── bytes feature "default" (*)
│               │   │   ├── futures-util feature "alloc" (*)
│               │   │   ├── memchr feature "default" (*)
│               │   │   ├── pin-project-lite feature "default" (*)
│               │   │   ├── http feature "default" (*)
│               │   │   ├── itoa feature "default" (*)
│               │   │   ├── http-body feature "default" (*)
│               │   │   ├── http-body-util feature "default" (*)
│               │   │   ├── mime feature "default" (*)
│               │   │   ├── rustversion feature "default" (*)
│               │   │   ├── sync_wrapper feature "default" (*)
│               │   │   ├── tower-layer feature "default" (*)
│               │   │   ├── tower-service feature "default" (*)
│               │   │   ├── axum-macros feature "default"
│               │   │   │   └── axum-macros v0.4.2 (proc-macro)
│               │   │   │       ├── proc-macro2 feature "default" (*)
│               │   │   │       ├── quote feature "default" (*)
│               │   │   │       ├── syn feature "default" (*)
│               │   │   │       ├── syn feature "extra-traits"
│               │   │   │       │   └── syn v2.0.106 (*)
│               │   │   │       ├── syn feature "full" (*)
│               │   │   │       └── syn feature "parsing" (*)
│               │   │   ├── hyper feature "default"
│               │   │   │   └── hyper v1.7.0
│               │   │   │       ├── bytes feature "default" (*)
│               │   │   │       ├── futures-channel feature "default"
│               │   │   │       │   ├── futures-channel v0.3.31 (*)
│               │   │   │       │   └── futures-channel feature "std" (*)
│               │   │   │       ├── futures-core feature "default"
│               │   │   │       │   ├── futures-core v0.3.31
│               │   │   │       │   └── futures-core feature "std" (*)
│               │   │   │       ├── pin-project-lite feature "default" (*)
│               │   │   │       ├── pin-utils feature "default" (*)
│               │   │   │       ├── http feature "default" (*)
│               │   │   │       ├── itoa feature "default" (*)
│               │   │   │       ├── http-body feature "default" (*)
│               │   │   │       ├── atomic-waker feature "default"
│               │   │   │       │   └── atomic-waker v1.1.2
│               │   │   │       ├── h2 feature "default"
│               │   │   │       │   └── h2 v0.4.12
│               │   │   │       │       ├── futures-core v0.3.31
│               │   │   │       │       ├── futures-sink v0.3.31
│               │   │   │       │       ├── bytes feature "default" (*)
│               │   │   │       │       ├── slab feature "default" (*)
│               │   │   │       │       ├── http feature "default" (*)
│               │   │   │       │       ├── fnv feature "default" (*)
│               │   │   │       │       ├── atomic-waker feature "default" (*)
│               │   │   │       │       ├── indexmap feature "default"
│               │   │   │       │       │   ├── indexmap v2.11.1
│               │   │   │       │       │   │   ├── equivalent v1.0.2
│               │   │   │       │       │   │   └── hashbrown v0.15.5
│               │   │   │       │       │   │       ├── equivalent v1.0.2
│               │   │   │       │       │   │       ├── foldhash v0.1.5
│               │   │   │       │       │   │       └── allocator-api2 feature "alloc"
│               │   │   │       │       │   │           └── allocator-api2 v0.2.21
│               │   │   │       │       │   └── indexmap feature "std"
│               │   │   │       │       │       └── indexmap v2.11.1 (*)
│               │   │   │       │       ├── indexmap feature "std" (*)
│               │   │   │       │       ├── tokio feature "default"
│               │   │   │       │       │   └── tokio v1.47.1
│               │   │   │       │       │       ├── mio v1.0.4
│               │   │   │       │       │       │   └── libc feature "default"
│               │   │   │       │       │       │       ├── libc v0.2.175
│               │   │   │       │       │       │       └── libc feature "std"
│               │   │   │       │       │       │           └── libc v0.2.175
│               │   │   │       │       │       ├── bytes feature "default" (*)
│               │   │   │       │       │       ├── pin-project-lite feature "default" (*)
│               │   │   │       │       │       ├── libc feature "default" (*)
│               │   │   │       │       │       ├── parking_lot feature "default"
│               │   │   │       │       │       │   └── parking_lot v0.12.4
│               │   │   │       │       │       │       ├── lock_api feature "default"
│               │   │   │       │       │       │       │   ├── lock_api v0.4.13
│               │   │   │       │       │       │       │   │   └── scopeguard v1.2.0
│               │   │   │       │       │       │       │   │   [build-dependencies]
│               │   │   │       │       │       │       │   │   └── autocfg feature "default" (*)
│               │   │   │       │       │       │       │   └── lock_api feature "atomic_usize"
│               │   │   │       │       │       │       │       └── lock_api v0.4.13 (*)
│               │   │   │       │       │       │       └── parking_lot_core feature "default"
│               │   │   │       │       │       │           └── parking_lot_core v0.9.11
│               │   │   │       │       │       │               ├── libc feature "default" (*)
│               │   │   │       │       │       │               ├── cfg-if feature "default"
│               │   │   │       │       │       │               │   └── cfg-if v1.0.3
│               │   │   │       │       │       │               └── smallvec feature "default"
│               │   │   │       │       │       │                   └── smallvec v1.15.1
│               │   │   │       │       │       ├── signal-hook-registry feature "default"
│               │   │   │       │       │       │   └── signal-hook-registry v1.4.6
│               │   │   │       │       │       │       └── libc feature "default" (*)
│               │   │   │       │       │       ├── socket2 feature "all"
│               │   │   │       │       │       │   └── socket2 v0.6.0
│               │   │   │       │       │       │       └── libc feature "default" (*)
│               │   │   │       │       │       ├── socket2 feature "default"
│               │   │   │       │       │       │   └── socket2 v0.6.0 (*)
│               │   │   │       │       │       └── tokio-macros feature "default"
│               │   │   │       │       │           └── tokio-macros v2.5.0 (proc-macro)
│               │   │   │       │       │               ├── proc-macro2 feature "default" (*)
│               │   │   │       │       │               ├── quote feature "default" (*)
│               │   │   │       │       │               ├── syn feature "default" (*)
│               │   │   │       │       │               └── syn feature "full" (*)
│               │   │   │       │       ├── tokio feature "io-util"
│               │   │   │       │       │   ├── tokio v1.47.1 (*)
│               │   │   │       │       │   └── tokio feature "bytes"
│               │   │   │       │       │       └── tokio v1.47.1 (*)
│               │   │   │       │       ├── tokio-util feature "codec"
│               │   │   │       │       │   └── tokio-util v0.7.16
│               │   │   │       │       │       ├── bytes feature "default" (*)
│               │   │   │       │       │       ├── futures-core feature "default" (*)
│               │   │   │       │       │       ├── futures-sink feature "default"
│               │   │   │       │       │       │   ├── futures-sink v0.3.31
│               │   │   │       │       │       │   └── futures-sink feature "std"
... (truncated)
```

</details>

### ronctl

<details><summary>Reverse tree (-i ronctl -e features)</summary>

```text
```

</details>

<details><summary>Forward tree (-p ronctl -e features)</summary>

```text
ronctl v0.1.0 (/Users/mymac/Desktop/RustyOnions/tools/ronctl)
├── anyhow feature "default"
│   ├── anyhow v1.0.99
│   └── anyhow feature "std"
│       └── anyhow v1.0.99
├── clap feature "default"
│   ├── clap v4.5.47
│   │   ├── clap_builder v4.5.47
│   │   │   ├── anstream feature "default"
│   │   │   │   ├── anstream v0.6.20
│   │   │   │   │   ├── anstyle feature "default"
│   │   │   │   │   │   ├── anstyle v1.0.11
│   │   │   │   │   │   └── anstyle feature "std"
│   │   │   │   │   │       └── anstyle v1.0.11
│   │   │   │   │   ├── anstyle-parse feature "default"
│   │   │   │   │   │   ├── anstyle-parse v0.2.7
│   │   │   │   │   │   │   └── utf8parse feature "default"
│   │   │   │   │   │   │       └── utf8parse v0.2.2
│   │   │   │   │   │   └── anstyle-parse feature "utf8"
│   │   │   │   │   │       └── anstyle-parse v0.2.7 (*)
│   │   │   │   │   ├── utf8parse feature "default" (*)
│   │   │   │   │   ├── anstyle-query feature "default"
│   │   │   │   │   │   └── anstyle-query v1.1.4
│   │   │   │   │   ├── colorchoice feature "default"
│   │   │   │   │   │   └── colorchoice v1.0.4
│   │   │   │   │   └── is_terminal_polyfill feature "default"
│   │   │   │   │       └── is_terminal_polyfill v1.70.1
│   │   │   │   ├── anstream feature "auto"
│   │   │   │   │   └── anstream v0.6.20 (*)
│   │   │   │   └── anstream feature "wincon"
│   │   │   │       └── anstream v0.6.20 (*)
│   │   │   ├── anstyle feature "default" (*)
│   │   │   ├── clap_lex feature "default"
│   │   │   │   └── clap_lex v0.7.5
│   │   │   └── strsim feature "default"
│   │   │       └── strsim v0.11.1
│   │   └── clap_derive feature "default"
│   │       └── clap_derive v4.5.47 (proc-macro)
│   │           ├── heck feature "default"
│   │           │   └── heck v0.5.0
│   │           ├── proc-macro2 feature "default"
│   │           │   ├── proc-macro2 v1.0.101
│   │           │   │   └── unicode-ident feature "default"
│   │           │   │       └── unicode-ident v1.0.19
│   │           │   └── proc-macro2 feature "proc-macro"
│   │           │       └── proc-macro2 v1.0.101 (*)
│   │           ├── quote feature "default"
│   │           │   ├── quote v1.0.40
│   │           │   │   └── proc-macro2 v1.0.101 (*)
│   │           │   └── quote feature "proc-macro"
│   │           │       ├── quote v1.0.40 (*)
│   │           │       └── proc-macro2 feature "proc-macro" (*)
│   │           ├── syn feature "default"
│   │           │   ├── syn v2.0.106
│   │           │   │   ├── proc-macro2 v1.0.101 (*)
│   │           │   │   ├── quote v1.0.40 (*)
│   │           │   │   └── unicode-ident feature "default" (*)
│   │           │   ├── syn feature "clone-impls"
│   │           │   │   └── syn v2.0.106 (*)
│   │           │   ├── syn feature "derive"
│   │           │   │   └── syn v2.0.106 (*)
│   │           │   ├── syn feature "parsing"
│   │           │   │   └── syn v2.0.106 (*)
│   │           │   ├── syn feature "printing"
│   │           │   │   └── syn v2.0.106 (*)
│   │           │   └── syn feature "proc-macro"
│   │           │       ├── syn v2.0.106 (*)
│   │           │       ├── proc-macro2 feature "proc-macro" (*)
│   │           │       └── quote feature "proc-macro" (*)
│   │           └── syn feature "full"
│   │               └── syn v2.0.106 (*)
│   ├── clap feature "color"
│   │   ├── clap v4.5.47 (*)
│   │   └── clap_builder feature "color"
│   │       └── clap_builder v4.5.47 (*)
│   ├── clap feature "error-context"
│   │   ├── clap v4.5.47 (*)
│   │   └── clap_builder feature "error-context"
│   │       └── clap_builder v4.5.47 (*)
│   ├── clap feature "help"
│   │   ├── clap v4.5.47 (*)
│   │   └── clap_builder feature "help"
│   │       └── clap_builder v4.5.47 (*)
│   ├── clap feature "std"
│   │   ├── clap v4.5.47 (*)
│   │   └── clap_builder feature "std"
│   │       ├── clap_builder v4.5.47 (*)
│   │       └── anstyle feature "std" (*)
│   ├── clap feature "suggestions"
│   │   ├── clap v4.5.47 (*)
│   │   └── clap_builder feature "suggestions"
│   │       ├── clap_builder v4.5.47 (*)
│   │       └── clap_builder feature "error-context" (*)
│   └── clap feature "usage"
│       ├── clap v4.5.47 (*)
│       └── clap_builder feature "usage"
│           └── clap_builder v4.5.47 (*)
├── hex feature "default"
│   ├── hex v0.4.3
│   └── hex feature "std"
│       ├── hex v0.4.3
│       └── hex feature "alloc"
│           └── hex v0.4.3
├── rand feature "default"
│   ├── rand v0.9.2
│   │   ├── rand_chacha v0.9.0
│   │   │   ├── ppv-lite86 feature "simd"
│   │   │   │   └── ppv-lite86 v0.2.21
│   │   │   │       ├── zerocopy feature "default"
│   │   │   │       │   └── zerocopy v0.8.27
│   │   │   │       └── zerocopy feature "simd"
│   │   │   │           └── zerocopy v0.8.27
│   │   │   └── rand_core feature "default"
│   │   │       └── rand_core v0.9.3
│   │   │           └── getrandom feature "default"
│   │   │               └── getrandom v0.3.3
│   │   │                   ├── libc v0.2.175
│   │   │                   └── cfg-if feature "default"
│   │   │                       └── cfg-if v1.0.3
│   │   └── rand_core v0.9.3 (*)
│   ├── rand feature "os_rng"
│   │   ├── rand v0.9.2 (*)
│   │   └── rand_core feature "os_rng"
│   │       └── rand_core v0.9.3 (*)
│   ├── rand feature "small_rng"
│   │   └── rand v0.9.2 (*)
│   ├── rand feature "std"
│   │   ├── rand v0.9.2 (*)
│   │   ├── rand feature "alloc"
│   │   │   └── rand v0.9.2 (*)
│   │   ├── rand_chacha feature "std"
│   │   │   ├── rand_chacha v0.9.0 (*)
│   │   │   ├── ppv-lite86 feature "std"
│   │   │   │   └── ppv-lite86 v0.2.21 (*)
│   │   │   └── rand_core feature "std"
│   │   │       ├── rand_core v0.9.3 (*)
│   │   │       └── getrandom feature "std"
│   │   │           └── getrandom v0.3.3 (*)
│   │   └── rand_core feature "std" (*)
│   ├── rand feature "std_rng"
│   │   └── rand v0.9.2 (*)
│   └── rand feature "thread_rng"
│       ├── rand v0.9.2 (*)
│       ├── rand feature "os_rng" (*)
│       ├── rand feature "std" (*)
│       └── rand feature "std_rng" (*)
├── rand feature "std" (*)
├── rand_chacha feature "default"
│   ├── rand_chacha v0.9.0 (*)
│   └── rand_chacha feature "std" (*)
├── rand_chacha feature "std" (*)
├── rmp-serde feature "default"
│   └── rmp-serde v1.3.0
│       ├── byteorder feature "default"
│       │   ├── byteorder v1.5.0
│       │   └── byteorder feature "std"
│       │       └── byteorder v1.5.0
│       ├── rmp feature "default"
│       │   ├── rmp v0.8.14
│       │   │   ├── byteorder v1.5.0
│       │   │   ├── num-traits v0.2.19
│       │   │   │   [build-dependencies]
│       │   │   │   └── autocfg feature "default"
│       │   │   │       └── autocfg v1.5.0
│       │   │   └── paste feature "default"
│       │   │       └── paste v1.0.15 (proc-macro)
│       │   └── rmp feature "std"
│       │       ├── rmp v0.8.14 (*)
│       │       ├── byteorder feature "std" (*)
│       │       └── num-traits feature "std"
│       │           └── num-traits v0.2.19 (*)
│       └── serde feature "default"
│           ├── serde v1.0.221
│           │   ├── serde_core feature "result"
│           │   │   └── serde_core v1.0.221
│           │   └── serde_derive feature "default"
│           │       └── serde_derive v1.0.221 (proc-macro)
│           │           ├── proc-macro2 feature "proc-macro" (*)
│           │           ├── quote feature "proc-macro" (*)
│           │           ├── syn feature "clone-impls" (*)
│           │           ├── syn feature "derive" (*)
│           │           ├── syn feature "parsing" (*)
│           │           ├── syn feature "printing" (*)
│           │           └── syn feature "proc-macro" (*)
│           └── serde feature "std"
│               ├── serde v1.0.221 (*)
│               └── serde_core feature "std"
│                   └── serde_core v1.0.221
├── serde feature "default" (*)
├── ron-bus feature "default"
│   └── ron-bus v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/ron-bus)
│       ├── rmp-serde feature "default" (*)
│       ├── serde feature "default" (*)
│       ├── thiserror feature "default"
│       │   └── thiserror v1.0.69
│       │       └── thiserror-impl feature "default"
│       │           └── thiserror-impl v1.0.69 (proc-macro)
│       │               ├── proc-macro2 feature "default" (*)
│       │               ├── quote feature "default" (*)
│       │               └── syn feature "default" (*)
│       └── workspace-hack feature "default"
│           └── workspace-hack v0.1.0 (/Users/mymac/Desktop/RustyOnions/workspace-hack)
│               ├── clap feature "default" (*)
│               ├── clap feature "derive"
│               │   └── clap v4.5.47 (*)
│               ├── clap feature "env"
│               │   ├── clap v4.5.47 (*)
│               │   └── clap_builder feature "env"
│               │       └── clap_builder v4.5.47 (*)
│               ├── clap_builder feature "color" (*)
│               ├── clap_builder feature "env" (*)
│               ├── clap_builder feature "help" (*)
│               ├── clap_builder feature "std" (*)
│               ├── clap_builder feature "suggestions" (*)
│               ├── clap_builder feature "usage" (*)
│               ├── rand_chacha feature "default" (*)
│               ├── rand_core feature "os_rng" (*)
│               ├── rand_core feature "std" (*)
│               ├── num-traits feature "std" (*)
│               ├── serde feature "alloc"
│               │   ├── serde v1.0.221 (*)
│               │   └── serde_core feature "alloc"
│               │       └── serde_core v1.0.221
│               ├── serde feature "default" (*)
│               ├── serde feature "derive"
│               │   ├── serde v1.0.221 (*)
│               │   └── serde feature "serde_derive"
│               │       └── serde v1.0.221 (*)
│               ├── serde_core feature "alloc" (*)
│               ├── serde_core feature "result" (*)
│               ├── serde_core feature "std" (*)
│               ├── axum feature "http1"
│               │   ├── axum v0.7.9
│               │   │   ├── serde feature "default" (*)
│               │   │   ├── async-trait feature "default"
│               │   │   │   └── async-trait v0.1.89 (proc-macro)
│               │   │   │       ├── proc-macro2 feature "default" (*)
│               │   │   │       ├── quote feature "default" (*)
│               │   │   │       ├── syn feature "clone-impls" (*)
│               │   │   │       ├── syn feature "full" (*)
│               │   │   │       ├── syn feature "parsing" (*)
│               │   │   │       ├── syn feature "printing" (*)
│               │   │   │       ├── syn feature "proc-macro" (*)
│               │   │   │       └── syn feature "visit-mut"
│               │   │   │           └── syn v2.0.106 (*)
│               │   │   ├── axum-core feature "default"
│               │   │   │   └── axum-core v0.4.5
│               │   │   │       ├── async-trait feature "default" (*)
│               │   │   │       ├── bytes feature "default"
│               │   │   │       │   ├── bytes v1.10.1
│               │   │   │       │   └── bytes feature "std"
│               │   │   │       │       └── bytes v1.10.1
│               │   │   │       ├── futures-util feature "alloc"
│               │   │   │       │   ├── futures-util v0.3.31
│               │   │   │       │   │   ├── futures-core v0.3.31
│               │   │   │       │   │   ├── futures-macro v0.3.31 (proc-macro)
│               │   │   │       │   │   │   ├── proc-macro2 feature "default" (*)
│               │   │   │       │   │   │   ├── quote feature "default" (*)
│               │   │   │       │   │   │   ├── syn feature "default" (*)
│               │   │   │       │   │   │   └── syn feature "full" (*)
│               │   │   │       │   │   ├── futures-sink v0.3.31
│               │   │   │       │   │   ├── futures-task v0.3.31
│               │   │   │       │   │   ├── futures-channel feature "std"
│               │   │   │       │   │   │   ├── futures-channel v0.3.31
│               │   │   │       │   │   │   │   ├── futures-core v0.3.31
│               │   │   │       │   │   │   │   └── futures-sink v0.3.31
│               │   │   │       │   │   │   ├── futures-channel feature "alloc"
│               │   │   │       │   │   │   │   ├── futures-channel v0.3.31 (*)
│               │   │   │       │   │   │   │   └── futures-core feature "alloc"
│               │   │   │       │   │   │   │       └── futures-core v0.3.31
│               │   │   │       │   │   │   └── futures-core feature "std"
│               │   │   │       │   │   │       ├── futures-core v0.3.31
│               │   │   │       │   │   │       └── futures-core feature "alloc" (*)
│               │   │   │       │   │   ├── futures-io feature "std"
│               │   │   │       │   │   │   └── futures-io v0.3.31
│               │   │   │       │   │   ├── memchr feature "default"
│               │   │   │       │   │   │   ├── memchr v2.7.5
│               │   │   │       │   │   │   └── memchr feature "std"
│               │   │   │       │   │   │       ├── memchr v2.7.5
│               │   │   │       │   │   │       └── memchr feature "alloc"
│               │   │   │       │   │   │           └── memchr v2.7.5
│               │   │   │       │   │   ├── pin-project-lite feature "default"
│               │   │   │       │   │   │   └── pin-project-lite v0.2.16
│               │   │   │       │   │   ├── pin-utils feature "default"
│               │   │   │       │   │   │   └── pin-utils v0.1.0
│               │   │   │       │   │   └── slab feature "default"
│               │   │   │       │   │       ├── slab v0.4.11
│               │   │   │       │   │       └── slab feature "std"
│               │   │   │       │   │           └── slab v0.4.11
│               │   │   │       │   ├── futures-core feature "alloc" (*)
│               │   │   │       │   └── futures-task feature "alloc"
│               │   │   │       │       └── futures-task v0.3.31
│               │   │   │       ├── pin-project-lite feature "default" (*)
│               │   │   │       ├── http feature "default"
│               │   │   │       │   ├── http v1.3.1
│               │   │   │       │   │   ├── bytes feature "default" (*)
│               │   │   │       │   │   ├── fnv feature "default"
│               │   │   │       │   │   │   ├── fnv v1.0.7
│               │   │   │       │   │   │   └── fnv feature "std"
│               │   │   │       │   │   │       └── fnv v1.0.7
... (truncated)
```

</details>

### actor_spike

<details><summary>Reverse tree (-i actor_spike -e features)</summary>

```text
```

</details>

<details><summary>Forward tree (-p actor_spike -e features)</summary>

```text
actor_spike v0.2.0 (/Users/mymac/Desktop/RustyOnions/experiments/actor_spike)
├── anyhow feature "default"
│   ├── anyhow v1.0.99
│   └── anyhow feature "std"
│       └── anyhow v1.0.99
├── rand feature "default"
│   ├── rand v0.9.2
│   │   ├── rand_chacha v0.9.0
│   │   │   ├── ppv-lite86 feature "simd"
│   │   │   │   └── ppv-lite86 v0.2.21
│   │   │   │       ├── zerocopy feature "default"
│   │   │   │       │   └── zerocopy v0.8.27
│   │   │   │       └── zerocopy feature "simd"
│   │   │   │           └── zerocopy v0.8.27
│   │   │   └── rand_core feature "default"
│   │   │       └── rand_core v0.9.3
│   │   │           └── getrandom feature "default"
│   │   │               └── getrandom v0.3.3
│   │   │                   ├── libc v0.2.175
│   │   │                   └── cfg-if feature "default"
│   │   │                       └── cfg-if v1.0.3
│   │   └── rand_core v0.9.3 (*)
│   ├── rand feature "os_rng"
│   │   ├── rand v0.9.2 (*)
│   │   └── rand_core feature "os_rng"
│   │       └── rand_core v0.9.3 (*)
│   ├── rand feature "small_rng"
│   │   └── rand v0.9.2 (*)
│   ├── rand feature "std"
│   │   ├── rand v0.9.2 (*)
│   │   ├── rand feature "alloc"
│   │   │   └── rand v0.9.2 (*)
│   │   ├── rand_chacha feature "std"
│   │   │   ├── rand_chacha v0.9.0 (*)
│   │   │   ├── ppv-lite86 feature "std"
│   │   │   │   └── ppv-lite86 v0.2.21 (*)
│   │   │   └── rand_core feature "std"
│   │   │       ├── rand_core v0.9.3 (*)
│   │   │       └── getrandom feature "std"
│   │   │           └── getrandom v0.3.3 (*)
│   │   └── rand_core feature "std" (*)
│   ├── rand feature "std_rng"
│   │   └── rand v0.9.2 (*)
│   └── rand feature "thread_rng"
│       ├── rand v0.9.2 (*)
│       ├── rand feature "os_rng" (*)
│       ├── rand feature "std" (*)
│       └── rand feature "std_rng" (*)
├── rand feature "std" (*)
├── rand_chacha feature "default"
│   ├── rand_chacha v0.9.0 (*)
│   └── rand_chacha feature "std" (*)
├── rand_chacha feature "std" (*)
├── ron-kernel feature "default"
│   └── ron-kernel v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/ron-kernel)
│       ├── reqwest v0.12.23
│       │   ├── futures-core v0.3.31
│       │   ├── bytes feature "default"
│       │   │   ├── bytes v1.10.1
│       │   │   └── bytes feature "std"
│       │   │       └── bytes v1.10.1
│       │   ├── pin-project-lite feature "default"
│       │   │   └── pin-project-lite v0.2.16
│       │   ├── http feature "default"
│       │   │   ├── http v1.3.1
│       │   │   │   ├── bytes feature "default" (*)
│       │   │   │   ├── fnv feature "default"
│       │   │   │   │   ├── fnv v1.0.7
│       │   │   │   │   └── fnv feature "std"
│       │   │   │   │       └── fnv v1.0.7
│       │   │   │   └── itoa feature "default"
│       │   │   │       └── itoa v1.0.15
│       │   │   └── http feature "std"
│       │   │       └── http v1.3.1 (*)
│       │   ├── http-body feature "default"
│       │   │   └── http-body v1.0.1
│       │   │       ├── bytes feature "default" (*)
│       │   │       └── http feature "default" (*)
│       │   ├── http-body-util feature "default"
│       │   │   └── http-body-util v0.1.3
│       │   │       ├── futures-core v0.3.31
│       │   │       ├── bytes feature "default" (*)
│       │   │       ├── pin-project-lite feature "default" (*)
│       │   │       ├── http feature "default" (*)
│       │   │       └── http-body feature "default" (*)
│       │   ├── sync_wrapper feature "default"
│       │   │   └── sync_wrapper v1.0.2
│       │   │       └── futures-core v0.3.31
│       │   ├── sync_wrapper feature "futures"
│       │   │   ├── sync_wrapper v1.0.2 (*)
│       │   │   └── sync_wrapper feature "futures-core"
│       │   │       └── sync_wrapper v1.0.2 (*)
│       │   ├── tower-service feature "default"
│       │   │   └── tower-service v0.3.3
│       │   ├── hyper feature "client"
│       │   │   └── hyper v1.7.0
│       │   │       ├── bytes feature "default" (*)
│       │   │       ├── futures-channel feature "default"
│       │   │       │   ├── futures-channel v0.3.31
│       │   │       │   │   ├── futures-core v0.3.31
│       │   │       │   │   └── futures-sink v0.3.31
│       │   │       │   └── futures-channel feature "std"
│       │   │       │       ├── futures-channel v0.3.31 (*)
│       │   │       │       ├── futures-channel feature "alloc"
│       │   │       │       │   ├── futures-channel v0.3.31 (*)
│       │   │       │       │   └── futures-core feature "alloc"
│       │   │       │       │       └── futures-core v0.3.31
│       │   │       │       └── futures-core feature "std"
│       │   │       │           ├── futures-core v0.3.31
│       │   │       │           └── futures-core feature "alloc" (*)
│       │   │       ├── futures-core feature "default"
│       │   │       │   ├── futures-core v0.3.31
│       │   │       │   └── futures-core feature "std" (*)
│       │   │       ├── pin-project-lite feature "default" (*)
│       │   │       ├── pin-utils feature "default"
│       │   │       │   └── pin-utils v0.1.0
│       │   │       ├── http feature "default" (*)
│       │   │       ├── itoa feature "default" (*)
│       │   │       ├── http-body feature "default" (*)
│       │   │       ├── atomic-waker feature "default"
│       │   │       │   └── atomic-waker v1.1.2
│       │   │       ├── h2 feature "default"
│       │   │       │   └── h2 v0.4.12
│       │   │       │       ├── futures-core v0.3.31
│       │   │       │       ├── futures-sink v0.3.31
│       │   │       │       ├── bytes feature "default" (*)
│       │   │       │       ├── slab feature "default"
│       │   │       │       │   ├── slab v0.4.11
│       │   │       │       │   └── slab feature "std"
│       │   │       │       │       └── slab v0.4.11
│       │   │       │       ├── http feature "default" (*)
│       │   │       │       ├── fnv feature "default" (*)
│       │   │       │       ├── atomic-waker feature "default" (*)
│       │   │       │       ├── indexmap feature "default"
│       │   │       │       │   ├── indexmap v2.11.1
│       │   │       │       │   │   ├── equivalent v1.0.2
│       │   │       │       │   │   └── hashbrown v0.15.5
│       │   │       │       │   │       ├── equivalent v1.0.2
│       │   │       │       │   │       ├── foldhash v0.1.5
│       │   │       │       │   │       └── allocator-api2 feature "alloc"
│       │   │       │       │   │           └── allocator-api2 v0.2.21
│       │   │       │       │   └── indexmap feature "std"
│       │   │       │       │       └── indexmap v2.11.1 (*)
│       │   │       │       ├── indexmap feature "std" (*)
│       │   │       │       ├── tokio feature "default"
│       │   │       │       │   └── tokio v1.47.1
│       │   │       │       │       ├── mio v1.0.4
│       │   │       │       │       │   └── libc feature "default"
│       │   │       │       │       │       ├── libc v0.2.175
│       │   │       │       │       │       └── libc feature "std"
│       │   │       │       │       │           └── libc v0.2.175
│       │   │       │       │       ├── libc feature "default" (*)
│       │   │       │       │       ├── bytes feature "default" (*)
│       │   │       │       │       ├── pin-project-lite feature "default" (*)
│       │   │       │       │       ├── parking_lot feature "default"
│       │   │       │       │       │   └── parking_lot v0.12.4
│       │   │       │       │       │       ├── lock_api feature "default"
│       │   │       │       │       │       │   ├── lock_api v0.4.13
│       │   │       │       │       │       │   │   └── scopeguard v1.2.0
│       │   │       │       │       │       │   │   [build-dependencies]
│       │   │       │       │       │       │   │   └── autocfg feature "default"
│       │   │       │       │       │       │   │       └── autocfg v1.5.0
│       │   │       │       │       │       │   └── lock_api feature "atomic_usize"
│       │   │       │       │       │       │       └── lock_api v0.4.13 (*)
│       │   │       │       │       │       └── parking_lot_core feature "default"
│       │   │       │       │       │           └── parking_lot_core v0.9.11
│       │   │       │       │       │               ├── cfg-if feature "default" (*)
│       │   │       │       │       │               ├── libc feature "default" (*)
│       │   │       │       │       │               └── smallvec feature "default"
│       │   │       │       │       │                   └── smallvec v1.15.1
│       │   │       │       │       ├── signal-hook-registry feature "default"
│       │   │       │       │       │   └── signal-hook-registry v1.4.6
│       │   │       │       │       │       └── libc feature "default" (*)
│       │   │       │       │       ├── socket2 feature "all"
│       │   │       │       │       │   └── socket2 v0.6.0
│       │   │       │       │       │       └── libc feature "default" (*)
│       │   │       │       │       ├── socket2 feature "default"
│       │   │       │       │       │   └── socket2 v0.6.0 (*)
│       │   │       │       │       └── tokio-macros feature "default"
│       │   │       │       │           └── tokio-macros v2.5.0 (proc-macro)
│       │   │       │       │               ├── proc-macro2 feature "default"
│       │   │       │       │               │   ├── proc-macro2 v1.0.101
│       │   │       │       │               │   │   └── unicode-ident feature "default"
│       │   │       │       │               │   │       └── unicode-ident v1.0.19
│       │   │       │       │               │   └── proc-macro2 feature "proc-macro"
│       │   │       │       │               │       └── proc-macro2 v1.0.101 (*)
│       │   │       │       │               ├── quote feature "default"
│       │   │       │       │               │   ├── quote v1.0.40
│       │   │       │       │               │   │   └── proc-macro2 v1.0.101 (*)
│       │   │       │       │               │   └── quote feature "proc-macro"
│       │   │       │       │               │       ├── quote v1.0.40 (*)
│       │   │       │       │               │       └── proc-macro2 feature "proc-macro" (*)
│       │   │       │       │               ├── syn feature "default"
│       │   │       │       │               │   ├── syn v2.0.106
│       │   │       │       │               │   │   ├── proc-macro2 v1.0.101 (*)
│       │   │       │       │               │   │   ├── quote v1.0.40 (*)
│       │   │       │       │               │   │   └── unicode-ident feature "default" (*)
│       │   │       │       │               │   ├── syn feature "clone-impls"
│       │   │       │       │               │   │   └── syn v2.0.106 (*)
│       │   │       │       │               │   ├── syn feature "derive"
│       │   │       │       │               │   │   └── syn v2.0.106 (*)
│       │   │       │       │               │   ├── syn feature "parsing"
│       │   │       │       │               │   │   └── syn v2.0.106 (*)
│       │   │       │       │               │   ├── syn feature "printing"
│       │   │       │       │               │   │   └── syn v2.0.106 (*)
│       │   │       │       │               │   └── syn feature "proc-macro"
│       │   │       │       │               │       ├── syn v2.0.106 (*)
│       │   │       │       │               │       ├── proc-macro2 feature "proc-macro" (*)
│       │   │       │       │               │       └── quote feature "proc-macro" (*)
│       │   │       │       │               └── syn feature "full"
│       │   │       │       │                   └── syn v2.0.106 (*)
│       │   │       │       ├── tokio feature "io-util"
│       │   │       │       │   ├── tokio v1.47.1 (*)
│       │   │       │       │   └── tokio feature "bytes"
│       │   │       │       │       └── tokio v1.47.1 (*)
│       │   │       │       ├── tokio-util feature "codec"
│       │   │       │       │   └── tokio-util v0.7.16
│       │   │       │       │       ├── bytes feature "default" (*)
│       │   │       │       │       ├── futures-core feature "default" (*)
│       │   │       │       │       ├── futures-sink feature "default"
│       │   │       │       │       │   ├── futures-sink v0.3.31
│       │   │       │       │       │   └── futures-sink feature "std"
│       │   │       │       │       │       ├── futures-sink v0.3.31
│       │   │       │       │       │       └── futures-sink feature "alloc"
│       │   │       │       │       │           └── futures-sink v0.3.31
│       │   │       │       │       ├── pin-project-lite feature "default" (*)
│       │   │       │       │       ├── tokio feature "default" (*)
│       │   │       │       │       └── tokio feature "sync"
│       │   │       │       │           └── tokio v1.47.1 (*)
│       │   │       │       ├── tokio-util feature "default"
│       │   │       │       │   └── tokio-util v0.7.16 (*)
│       │   │       │       ├── tokio-util feature "io"
│       │   │       │       │   └── tokio-util v0.7.16 (*)
│       │   │       │       └── tracing feature "std"
│       │   │       │           ├── tracing v0.1.41
│       │   │       │           │   ├── tracing-core v0.1.34
│       │   │       │           │   │   └── once_cell feature "default"
│       │   │       │           │   │       ├── once_cell v1.21.3
│       │   │       │           │   │       └── once_cell feature "std"
│       │   │       │           │   │           ├── once_cell v1.21.3
│       │   │       │           │   │           └── once_cell feature "alloc"
│       │   │       │           │   │               ├── once_cell v1.21.3
│       │   │       │           │   │               └── once_cell feature "race"
│       │   │       │           │   │                   └── once_cell v1.21.3
│       │   │       │           │   ├── pin-project-lite feature "default" (*)
│       │   │       │           │   └── tracing-attributes feature "default"
│       │   │       │           │       └── tracing-attributes v0.1.30 (proc-macro)
│       │   │       │           │           ├── proc-macro2 feature "default" (*)
│       │   │       │           │           ├── quote feature "default" (*)
│       │   │       │           │           ├── syn feature "clone-impls" (*)
│       │   │       │           │           ├── syn feature "extra-traits"
│       │   │       │           │           │   └── syn v2.0.106 (*)
│       │   │       │           │           ├── syn feature "full" (*)
│       │   │       │           │           ├── syn feature "parsing" (*)
│       │   │       │           │           ├── syn feature "printing" (*)
│       │   │       │           │           ├── syn feature "proc-macro" (*)
│       │   │       │           │           └── syn feature "visit-mut"
│       │   │       │           │               └── syn v2.0.106 (*)
│       │   │       │           └── tracing-core feature "std"
│       │   │       │               ├── tracing-core v0.1.34 (*)
│       │   │       │               └── tracing-core feature "once_cell"
│       │   │       │                   └── tracing-core v0.1.34 (*)
│       │   │       ├── tokio feature "default" (*)
│       │   │       ├── tokio feature "sync" (*)
│       │   │       ├── smallvec feature "const_generics"
│       │   │       │   └── smallvec v1.15.1
│       │   │       ├── smallvec feature "const_new"
│       │   │       │   ├── smallvec v1.15.1
│       │   │       │   └── smallvec feature "const_generics" (*)
│       │   │       ├── smallvec feature "default" (*)
│       │   │       ├── httparse feature "default"
│       │   │       │   ├── httparse v1.10.1
│       │   │       │   └── httparse feature "std"
│       │   │       │       └── httparse v1.10.1
│       │   │       ├── httpdate feature "default"
│       │   │       │   └── httpdate v1.0.3
│       │   │       └── want feature "default"
│       │   │           └── want v0.3.1
│       │   │               └── try-lock feature "default"
│       │   │                   └── try-lock v0.2.5
│       │   ├── hyper feature "default"
│       │   │   └── hyper v1.7.0 (*)
│       │   ├── hyper feature "http1"
│       │   │   └── hyper v1.7.0 (*)
│       │   ├── tokio feature "net"
│       │   │   ├── tokio v1.47.1 (*)
│       │   │   ├── tokio feature "libc"
│       │   │   │   └── tokio v1.47.1 (*)
│       │   │   ├── tokio feature "mio"
│       │   │   │   └── tokio v1.47.1 (*)
│       │   │   ├── tokio feature "socket2"
│       │   │   │   └── tokio v1.47.1 (*)
│       │   │   ├── mio feature "net"
│       │   │   │   └── mio v1.0.4 (*)
│       │   │   ├── mio feature "os-ext"
│       │   │   │   ├── mio v1.0.4 (*)
│       │   │   │   └── mio feature "os-poll"
│       │   │   │       └── mio v1.0.4 (*)
│       │   │   └── mio feature "os-poll" (*)
│       │   ├── tokio feature "time"
... (truncated)
```

</details>

### ron-app-sdk

<details><summary>Reverse tree (-i ron-app-sdk -e features)</summary>

```text
```

</details>

<details><summary>Forward tree (-p ron-app-sdk -e features)</summary>

```text
ron-app-sdk v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/ron-app-sdk)
├── anyhow feature "default"
│   ├── anyhow v1.0.99
│   └── anyhow feature "std"
│       └── anyhow v1.0.99
├── bitflags feature "default"
│   └── bitflags v2.9.4
├── bytes feature "default"
│   ├── bytes v1.10.1
│   └── bytes feature "std"
│       └── bytes v1.10.1
├── futures-util feature "default"
│   ├── futures-util v0.3.31
│   │   ├── futures-core v0.3.31
│   │   ├── futures-macro v0.3.31 (proc-macro)
│   │   │   ├── proc-macro2 feature "default"
│   │   │   │   ├── proc-macro2 v1.0.101
│   │   │   │   │   └── unicode-ident feature "default"
│   │   │   │   │       └── unicode-ident v1.0.19
│   │   │   │   └── proc-macro2 feature "proc-macro"
│   │   │   │       └── proc-macro2 v1.0.101 (*)
│   │   │   ├── quote feature "default"
│   │   │   │   ├── quote v1.0.40
│   │   │   │   │   └── proc-macro2 v1.0.101 (*)
│   │   │   │   └── quote feature "proc-macro"
│   │   │   │       ├── quote v1.0.40 (*)
│   │   │   │       └── proc-macro2 feature "proc-macro" (*)
│   │   │   ├── syn feature "default"
│   │   │   │   ├── syn v2.0.106
│   │   │   │   │   ├── proc-macro2 v1.0.101 (*)
│   │   │   │   │   ├── quote v1.0.40 (*)
│   │   │   │   │   └── unicode-ident feature "default" (*)
│   │   │   │   ├── syn feature "clone-impls"
│   │   │   │   │   └── syn v2.0.106 (*)
│   │   │   │   ├── syn feature "derive"
│   │   │   │   │   └── syn v2.0.106 (*)
│   │   │   │   ├── syn feature "parsing"
│   │   │   │   │   └── syn v2.0.106 (*)
│   │   │   │   ├── syn feature "printing"
│   │   │   │   │   └── syn v2.0.106 (*)
│   │   │   │   └── syn feature "proc-macro"
│   │   │   │       ├── syn v2.0.106 (*)
│   │   │   │       ├── proc-macro2 feature "proc-macro" (*)
│   │   │   │       └── quote feature "proc-macro" (*)
│   │   │   └── syn feature "full"
│   │   │       └── syn v2.0.106 (*)
│   │   ├── futures-sink v0.3.31
│   │   ├── futures-task v0.3.31
│   │   ├── futures-channel feature "std"
│   │   │   ├── futures-channel v0.3.31
│   │   │   │   ├── futures-core v0.3.31
│   │   │   │   └── futures-sink v0.3.31
│   │   │   ├── futures-channel feature "alloc"
│   │   │   │   ├── futures-channel v0.3.31 (*)
│   │   │   │   └── futures-core feature "alloc"
│   │   │   │       └── futures-core v0.3.31
│   │   │   └── futures-core feature "std"
│   │   │       ├── futures-core v0.3.31
│   │   │       └── futures-core feature "alloc" (*)
│   │   ├── futures-io feature "std"
│   │   │   └── futures-io v0.3.31
│   │   ├── memchr feature "default"
│   │   │   ├── memchr v2.7.5
│   │   │   └── memchr feature "std"
│   │   │       ├── memchr v2.7.5
│   │   │       └── memchr feature "alloc"
│   │   │           └── memchr v2.7.5
│   │   ├── pin-project-lite feature "default"
│   │   │   └── pin-project-lite v0.2.16
│   │   ├── pin-utils feature "default"
│   │   │   └── pin-utils v0.1.0
│   │   └── slab feature "default"
│   │       ├── slab v0.4.11
│   │       └── slab feature "std"
│   │           └── slab v0.4.11
│   ├── futures-util feature "async-await"
│   │   └── futures-util v0.3.31 (*)
│   ├── futures-util feature "async-await-macro"
│   │   ├── futures-util v0.3.31 (*)
│   │   ├── futures-util feature "async-await" (*)
│   │   └── futures-util feature "futures-macro"
│   │       └── futures-util v0.3.31 (*)
│   └── futures-util feature "std"
│       ├── futures-util v0.3.31 (*)
│       ├── futures-util feature "alloc"
│       │   ├── futures-util v0.3.31 (*)
│       │   ├── futures-core feature "alloc" (*)
│       │   └── futures-task feature "alloc"
│       │       └── futures-task v0.3.31
│       ├── futures-util feature "slab"
│       │   └── futures-util v0.3.31 (*)
│       ├── futures-core feature "std" (*)
│       └── futures-task feature "std"
│           ├── futures-task v0.3.31
│           └── futures-task feature "alloc" (*)
├── futures-util feature "sink"
│   ├── futures-util v0.3.31 (*)
│   └── futures-util feature "futures-sink"
│       └── futures-util v0.3.31 (*)
├── rustls-native-certs feature "default"
│   └── rustls-native-certs v0.8.1
│       ├── rustls-pki-types feature "default"
│       │   ├── rustls-pki-types v1.12.0
│       │   │   └── zeroize feature "default"
│       │   │       ├── zeroize v1.8.1
│       │   │       │   └── zeroize_derive feature "default"
│       │   │       │       └── zeroize_derive v1.4.2 (proc-macro)
│       │   │       │           ├── proc-macro2 feature "default" (*)
│       │   │       │           ├── quote feature "default" (*)
│       │   │       │           ├── syn feature "default" (*)
│       │   │       │           ├── syn feature "extra-traits"
│       │   │       │           │   └── syn v2.0.106 (*)
│       │   │       │           ├── syn feature "full" (*)
│       │   │       │           └── syn feature "visit"
│       │   │       │               └── syn v2.0.106 (*)
│       │   │       └── zeroize feature "alloc"
│       │   │           └── zeroize v1.8.1 (*)
│       │   └── rustls-pki-types feature "alloc"
│       │       └── rustls-pki-types v1.12.0 (*)
│       ├── rustls-pki-types feature "std"
│       │   ├── rustls-pki-types v1.12.0 (*)
│       │   └── rustls-pki-types feature "alloc" (*)
│       └── security-framework feature "default"
│           ├── security-framework v3.4.0
│           │   ├── security-framework-sys v2.15.0
│           │   │   ├── core-foundation-sys feature "default"
│           │   │   │   ├── core-foundation-sys v0.8.7
│           │   │   │   └── core-foundation-sys feature "link"
│           │   │   │       └── core-foundation-sys v0.8.7
│           │   │   └── libc feature "default"
│           │   │       ├── libc v0.2.175
│           │   │       └── libc feature "std"
│           │   │           └── libc v0.2.175
│           │   ├── bitflags feature "default" (*)
│           │   ├── core-foundation feature "default"
│           │   │   ├── core-foundation v0.10.1
│           │   │   │   ├── core-foundation-sys v0.8.7
│           │   │   │   └── libc feature "default" (*)
│           │   │   └── core-foundation feature "link"
│           │   │       ├── core-foundation v0.10.1 (*)
│           │   │       └── core-foundation-sys feature "link" (*)
│           │   ├── core-foundation-sys feature "default" (*)
│           │   └── libc feature "default" (*)
│           └── security-framework feature "OSX_10_12"
│               ├── security-framework v3.4.0 (*)
│               └── security-framework-sys feature "OSX_10_12"
│                   ├── security-framework-sys v2.15.0 (*)
│                   └── security-framework-sys feature "OSX_10_11"
│                       ├── security-framework-sys v2.15.0 (*)
│                       └── security-framework-sys feature "OSX_10_10"
│                           ├── security-framework-sys v2.15.0 (*)
│                           └── security-framework-sys feature "OSX_10_9"
│                               └── security-framework-sys v2.15.0 (*)
├── rustls-pemfile feature "default"
│   ├── rustls-pemfile v2.2.0
│   │   └── rustls-pki-types feature "default" (*)
│   └── rustls-pemfile feature "std"
│       ├── rustls-pemfile v2.2.0 (*)
│       └── rustls-pki-types feature "std" (*)
├── serde feature "default"
│   ├── serde v1.0.221
│   │   ├── serde_core feature "result"
│   │   │   └── serde_core v1.0.221
│   │   └── serde_derive feature "default"
│   │       └── serde_derive v1.0.221 (proc-macro)
│   │           ├── proc-macro2 feature "proc-macro" (*)
│   │           ├── quote feature "proc-macro" (*)
│   │           ├── syn feature "clone-impls" (*)
│   │           ├── syn feature "derive" (*)
│   │           ├── syn feature "parsing" (*)
│   │           ├── syn feature "printing" (*)
│   │           └── syn feature "proc-macro" (*)
│   └── serde feature "std"
│       ├── serde v1.0.221 (*)
│       └── serde_core feature "std"
│           └── serde_core v1.0.221
├── serde_json feature "default"
│   ├── serde_json v1.0.144
│   │   ├── memchr v2.7.5
│   │   ├── serde_core v1.0.221
│   │   ├── itoa feature "default"
│   │   │   └── itoa v1.0.15
│   │   └── ryu feature "default"
│   │       └── ryu v1.0.20
│   └── serde_json feature "std"
│       ├── serde_json v1.0.144 (*)
│       ├── memchr feature "std" (*)
│       └── serde_core feature "std" (*)
├── thiserror feature "default"
│   └── thiserror v1.0.69
│       └── thiserror-impl feature "default"
│           └── thiserror-impl v1.0.69 (proc-macro)
│               ├── proc-macro2 feature "default" (*)
│               ├── quote feature "default" (*)
│               └── syn feature "default" (*)
├── tokio feature "default"
│   └── tokio v1.47.1
│       ├── mio v1.0.4
│       │   └── libc feature "default" (*)
│       ├── bytes feature "default" (*)
│       ├── pin-project-lite feature "default" (*)
│       ├── libc feature "default" (*)
│       ├── parking_lot feature "default"
│       │   └── parking_lot v0.12.4
│       │       ├── lock_api feature "default"
│       │       │   ├── lock_api v0.4.13
│       │       │   │   └── scopeguard v1.2.0
│       │       │   │   [build-dependencies]
│       │       │   │   └── autocfg feature "default"
│       │       │   │       └── autocfg v1.5.0
│       │       │   └── lock_api feature "atomic_usize"
│       │       │       └── lock_api v0.4.13 (*)
│       │       └── parking_lot_core feature "default"
│       │           └── parking_lot_core v0.9.11
│       │               ├── libc feature "default" (*)
│       │               ├── cfg-if feature "default"
│       │               │   └── cfg-if v1.0.3
│       │               └── smallvec feature "default"
│       │                   └── smallvec v1.15.1
│       ├── signal-hook-registry feature "default"
│       │   └── signal-hook-registry v1.4.6
│       │       └── libc feature "default" (*)
│       ├── socket2 feature "all"
│       │   └── socket2 v0.6.0
│       │       └── libc feature "default" (*)
│       ├── socket2 feature "default"
│       │   └── socket2 v0.6.0 (*)
│       └── tokio-macros feature "default"
│           └── tokio-macros v2.5.0 (proc-macro)
│               ├── proc-macro2 feature "default" (*)
│               ├── quote feature "default" (*)
│               ├── syn feature "default" (*)
│               └── syn feature "full" (*)
├── tokio feature "io-util"
│   ├── tokio v1.47.1 (*)
│   └── tokio feature "bytes"
│       └── tokio v1.47.1 (*)
├── tokio feature "macros"
│   ├── tokio v1.47.1 (*)
│   └── tokio feature "tokio-macros"
│       └── tokio v1.47.1 (*)
├── tokio feature "net"
│   ├── tokio v1.47.1 (*)
│   ├── tokio feature "libc"
│   │   └── tokio v1.47.1 (*)
│   ├── tokio feature "mio"
│   │   └── tokio v1.47.1 (*)
│   ├── tokio feature "socket2"
│   │   └── tokio v1.47.1 (*)
│   ├── mio feature "net"
│   │   └── mio v1.0.4 (*)
│   ├── mio feature "os-ext"
│   │   ├── mio v1.0.4 (*)
│   │   └── mio feature "os-poll"
│   │       └── mio v1.0.4 (*)
│   └── mio feature "os-poll" (*)
├── tokio feature "rt-multi-thread"
│   ├── tokio v1.47.1 (*)
│   └── tokio feature "rt"
│       └── tokio v1.47.1 (*)
├── tokio feature "time"
│   └── tokio v1.47.1 (*)
├── tokio-rustls feature "default"
│   ├── tokio-rustls v0.26.2
│   │   ├── tokio feature "default" (*)
│   │   └── rustls feature "std"
│   │       ├── rustls v0.23.31
│   │       │   ├── aws-lc-rs v1.14.0
│   │       │   │   ├── zeroize feature "default" (*)
│   │       │   │   └── aws-lc-sys feature "default"
│   │       │   │       └── aws-lc-sys v0.31.0
│   │       │   │           [build-dependencies]
│   │       │   │           ├── cc feature "default"
│   │       │   │           │   └── cc v1.2.37
│   │       │   │           │       ├── jobserver v0.1.34
│   │       │   │           │       │   └── libc feature "default" (*)
│   │       │   │           │       ├── libc v0.2.175
│   │       │   │           │       ├── find-msvc-tools feature "default"
│   │       │   │           │       │   └── find-msvc-tools v0.1.1
│   │       │   │           │       └── shlex feature "default"
│   │       │   │           │           ├── shlex v1.3.0
│   │       │   │           │           └── shlex feature "std"
│   │       │   │           │               └── shlex v1.3.0
│   │       │   │           ├── cc feature "parallel"
│   │       │   │           │   └── cc v1.2.37 (*)
│   │       │   │           ├── cmake feature "default"
│   │       │   │           │   └── cmake v0.1.54
│   │       │   │           │       └── cc feature "default" (*)
│   │       │   │           ├── dunce feature "default"
│   │       │   │           │   └── dunce v1.0.5
│   │       │   │           └── fs_extra feature "default"
│   │       │   │               └── fs_extra v1.3.0
│   │       │   ├── subtle v2.6.1
│   │       │   ├── rustls-pki-types feature "alloc" (*)
│   │       │   ├── rustls-pki-types feature "default" (*)
│   │       │   ├── zeroize feature "default" (*)
│   │       │   ├── log feature "default"
│   │       │   │   └── log v0.4.28
│   │       │   ├── once_cell feature "alloc"
│   │       │   │   ├── once_cell v1.21.3
... (truncated)
```

</details>

### svc-omnigate

<details><summary>Reverse tree (-i svc-omnigate -e features)</summary>

```text
```

</details>

<details><summary>Forward tree (-p svc-omnigate -e features)</summary>

```text
svc-omnigate v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/svc-omnigate)
├── anyhow feature "default"
│   ├── anyhow v1.0.99
│   └── anyhow feature "std"
│       └── anyhow v1.0.99
├── bytes feature "default"
│   ├── bytes v1.10.1
│   └── bytes feature "std"
│       └── bytes v1.10.1
├── futures-util feature "default"
│   ├── futures-util v0.3.31
│   │   ├── futures-core v0.3.31
│   │   ├── futures-macro v0.3.31 (proc-macro)
│   │   │   ├── proc-macro2 feature "default"
│   │   │   │   ├── proc-macro2 v1.0.101
│   │   │   │   │   └── unicode-ident feature "default"
│   │   │   │   │       └── unicode-ident v1.0.19
│   │   │   │   └── proc-macro2 feature "proc-macro"
│   │   │   │       └── proc-macro2 v1.0.101 (*)
│   │   │   ├── quote feature "default"
│   │   │   │   ├── quote v1.0.40
│   │   │   │   │   └── proc-macro2 v1.0.101 (*)
│   │   │   │   └── quote feature "proc-macro"
│   │   │   │       ├── quote v1.0.40 (*)
│   │   │   │       └── proc-macro2 feature "proc-macro" (*)
│   │   │   ├── syn feature "default"
│   │   │   │   ├── syn v2.0.106
│   │   │   │   │   ├── proc-macro2 v1.0.101 (*)
│   │   │   │   │   ├── quote v1.0.40 (*)
│   │   │   │   │   └── unicode-ident feature "default" (*)
│   │   │   │   ├── syn feature "clone-impls"
│   │   │   │   │   └── syn v2.0.106 (*)
│   │   │   │   ├── syn feature "derive"
│   │   │   │   │   └── syn v2.0.106 (*)
│   │   │   │   ├── syn feature "parsing"
│   │   │   │   │   └── syn v2.0.106 (*)
│   │   │   │   ├── syn feature "printing"
│   │   │   │   │   └── syn v2.0.106 (*)
│   │   │   │   └── syn feature "proc-macro"
│   │   │   │       ├── syn v2.0.106 (*)
│   │   │   │       ├── proc-macro2 feature "proc-macro" (*)
│   │   │   │       └── quote feature "proc-macro" (*)
│   │   │   └── syn feature "full"
│   │   │       └── syn v2.0.106 (*)
│   │   ├── futures-sink v0.3.31
│   │   ├── futures-task v0.3.31
│   │   ├── futures-channel feature "std"
│   │   │   ├── futures-channel v0.3.31
│   │   │   │   ├── futures-core v0.3.31
│   │   │   │   └── futures-sink v0.3.31
│   │   │   ├── futures-channel feature "alloc"
│   │   │   │   ├── futures-channel v0.3.31 (*)
│   │   │   │   └── futures-core feature "alloc"
│   │   │   │       └── futures-core v0.3.31
│   │   │   └── futures-core feature "std"
│   │   │       ├── futures-core v0.3.31
│   │   │       └── futures-core feature "alloc" (*)
│   │   ├── futures-io feature "std"
│   │   │   └── futures-io v0.3.31
│   │   ├── memchr feature "default"
│   │   │   ├── memchr v2.7.5
│   │   │   └── memchr feature "std"
│   │   │       ├── memchr v2.7.5
│   │   │       └── memchr feature "alloc"
│   │   │           └── memchr v2.7.5
│   │   ├── pin-project-lite feature "default"
│   │   │   └── pin-project-lite v0.2.16
│   │   ├── pin-utils feature "default"
│   │   │   └── pin-utils v0.1.0
│   │   └── slab feature "default"
│   │       ├── slab v0.4.11
│   │       └── slab feature "std"
│   │           └── slab v0.4.11
│   ├── futures-util feature "async-await"
│   │   └── futures-util v0.3.31 (*)
│   ├── futures-util feature "async-await-macro"
│   │   ├── futures-util v0.3.31 (*)
│   │   ├── futures-util feature "async-await" (*)
│   │   └── futures-util feature "futures-macro"
│   │       └── futures-util v0.3.31 (*)
│   └── futures-util feature "std"
│       ├── futures-util v0.3.31 (*)
│       ├── futures-util feature "alloc"
│       │   ├── futures-util v0.3.31 (*)
│       │   ├── futures-core feature "alloc" (*)
│       │   └── futures-task feature "alloc"
│       │       └── futures-task v0.3.31
│       ├── futures-util feature "slab"
│       │   └── futures-util v0.3.31 (*)
│       ├── futures-core feature "std" (*)
│       └── futures-task feature "std"
│           ├── futures-task v0.3.31
│           └── futures-task feature "alloc" (*)
├── http-body-util feature "default"
│   └── http-body-util v0.1.3
│       ├── futures-core v0.3.31
│       ├── bytes feature "default" (*)
│       ├── pin-project-lite feature "default" (*)
│       ├── http feature "default"
│       │   ├── http v1.3.1
│       │   │   ├── bytes feature "default" (*)
│       │   │   ├── fnv feature "default"
│       │   │   │   ├── fnv v1.0.7
│       │   │   │   └── fnv feature "std"
│       │   │   │       └── fnv v1.0.7
│       │   │   └── itoa feature "default"
│       │   │       └── itoa v1.0.15
│       │   └── http feature "std"
│       │       └── http v1.3.1 (*)
│       └── http-body feature "default"
│           └── http-body v1.0.1
│               ├── bytes feature "default" (*)
│               └── http feature "default" (*)
├── hyper feature "default"
│   └── hyper v1.7.0
│       ├── bytes feature "default" (*)
│       ├── futures-channel feature "default"
│       │   ├── futures-channel v0.3.31 (*)
│       │   └── futures-channel feature "std" (*)
│       ├── futures-core feature "default"
│       │   ├── futures-core v0.3.31
│       │   └── futures-core feature "std" (*)
│       ├── pin-project-lite feature "default" (*)
│       ├── pin-utils feature "default" (*)
│       ├── http feature "default" (*)
│       ├── itoa feature "default" (*)
│       ├── http-body feature "default" (*)
│       ├── atomic-waker feature "default"
│       │   └── atomic-waker v1.1.2
│       ├── h2 feature "default"
│       │   └── h2 v0.4.12
│       │       ├── futures-core v0.3.31
│       │       ├── futures-sink v0.3.31
│       │       ├── bytes feature "default" (*)
│       │       ├── slab feature "default" (*)
│       │       ├── http feature "default" (*)
│       │       ├── fnv feature "default" (*)
│       │       ├── atomic-waker feature "default" (*)
│       │       ├── indexmap feature "default"
│       │       │   ├── indexmap v2.11.1
│       │       │   │   ├── equivalent v1.0.2
│       │       │   │   └── hashbrown v0.15.5
│       │       │   │       ├── equivalent v1.0.2
│       │       │   │       ├── foldhash v0.1.5
│       │       │   │       └── allocator-api2 feature "alloc"
│       │       │   │           └── allocator-api2 v0.2.21
│       │       │   └── indexmap feature "std"
│       │       │       └── indexmap v2.11.1 (*)
│       │       ├── indexmap feature "std" (*)
│       │       ├── tokio feature "default"
│       │       │   └── tokio v1.47.1
│       │       │       ├── mio v1.0.4
│       │       │       │   └── libc feature "default"
│       │       │       │       ├── libc v0.2.175
│       │       │       │       └── libc feature "std"
│       │       │       │           └── libc v0.2.175
│       │       │       ├── bytes feature "default" (*)
│       │       │       ├── pin-project-lite feature "default" (*)
│       │       │       ├── libc feature "default" (*)
│       │       │       ├── parking_lot feature "default"
│       │       │       │   └── parking_lot v0.12.4
│       │       │       │       ├── lock_api feature "default"
│       │       │       │       │   ├── lock_api v0.4.13
│       │       │       │       │   │   └── scopeguard v1.2.0
│       │       │       │       │   │   [build-dependencies]
│       │       │       │       │   │   └── autocfg feature "default"
│       │       │       │       │   │       └── autocfg v1.5.0
│       │       │       │       │   └── lock_api feature "atomic_usize"
│       │       │       │       │       └── lock_api v0.4.13 (*)
│       │       │       │       └── parking_lot_core feature "default"
│       │       │       │           └── parking_lot_core v0.9.11
│       │       │       │               ├── libc feature "default" (*)
│       │       │       │               ├── cfg-if feature "default"
│       │       │       │               │   └── cfg-if v1.0.3
│       │       │       │               └── smallvec feature "default"
│       │       │       │                   └── smallvec v1.15.1
│       │       │       ├── signal-hook-registry feature "default"
│       │       │       │   └── signal-hook-registry v1.4.6
│       │       │       │       └── libc feature "default" (*)
│       │       │       ├── socket2 feature "all"
│       │       │       │   └── socket2 v0.6.0
│       │       │       │       └── libc feature "default" (*)
│       │       │       ├── socket2 feature "default"
│       │       │       │   └── socket2 v0.6.0 (*)
│       │       │       └── tokio-macros feature "default"
│       │       │           └── tokio-macros v2.5.0 (proc-macro)
│       │       │               ├── proc-macro2 feature "default" (*)
│       │       │               ├── quote feature "default" (*)
│       │       │               ├── syn feature "default" (*)
│       │       │               └── syn feature "full" (*)
│       │       ├── tokio feature "io-util"
│       │       │   ├── tokio v1.47.1 (*)
│       │       │   └── tokio feature "bytes"
│       │       │       └── tokio v1.47.1 (*)
│       │       ├── tokio-util feature "codec"
│       │       │   └── tokio-util v0.7.16
│       │       │       ├── bytes feature "default" (*)
│       │       │       ├── futures-core feature "default" (*)
│       │       │       ├── futures-sink feature "default"
│       │       │       │   ├── futures-sink v0.3.31
│       │       │       │   └── futures-sink feature "std"
│       │       │       │       ├── futures-sink v0.3.31
│       │       │       │       └── futures-sink feature "alloc"
│       │       │       │           └── futures-sink v0.3.31
│       │       │       ├── pin-project-lite feature "default" (*)
│       │       │       ├── tokio feature "default" (*)
│       │       │       └── tokio feature "sync"
│       │       │           └── tokio v1.47.1 (*)
│       │       ├── tokio-util feature "default"
│       │       │   └── tokio-util v0.7.16 (*)
│       │       ├── tokio-util feature "io"
│       │       │   └── tokio-util v0.7.16 (*)
│       │       └── tracing feature "std"
│       │           ├── tracing v0.1.41
│       │           │   ├── tracing-core v0.1.34
│       │           │   │   └── once_cell feature "default"
│       │           │   │       ├── once_cell v1.21.3
│       │           │   │       └── once_cell feature "std"
│       │           │   │           ├── once_cell v1.21.3
│       │           │   │           └── once_cell feature "alloc"
│       │           │   │               ├── once_cell v1.21.3
│       │           │   │               └── once_cell feature "race"
│       │           │   │                   └── once_cell v1.21.3
│       │           │   ├── pin-project-lite feature "default" (*)
│       │           │   └── tracing-attributes feature "default"
│       │           │       └── tracing-attributes v0.1.30 (proc-macro)
│       │           │           ├── proc-macro2 feature "default" (*)
│       │           │           ├── quote feature "default" (*)
│       │           │           ├── syn feature "clone-impls" (*)
│       │           │           ├── syn feature "extra-traits"
│       │           │           │   └── syn v2.0.106 (*)
│       │           │           ├── syn feature "full" (*)
│       │           │           ├── syn feature "parsing" (*)
│       │           │           ├── syn feature "printing" (*)
│       │           │           ├── syn feature "proc-macro" (*)
│       │           │           └── syn feature "visit-mut"
│       │           │               └── syn v2.0.106 (*)
│       │           └── tracing-core feature "std"
│       │               ├── tracing-core v0.1.34 (*)
│       │               └── tracing-core feature "once_cell"
│       │                   └── tracing-core v0.1.34 (*)
│       ├── tokio feature "default" (*)
│       ├── tokio feature "sync" (*)
│       ├── smallvec feature "const_generics"
│       │   └── smallvec v1.15.1
│       ├── smallvec feature "const_new"
│       │   ├── smallvec v1.15.1
│       │   └── smallvec feature "const_generics" (*)
│       ├── smallvec feature "default" (*)
│       ├── httparse feature "default"
│       │   ├── httparse v1.10.1
│       │   └── httparse feature "std"
│       │       └── httparse v1.10.1
│       ├── httpdate feature "default"
│       │   └── httpdate v1.0.3
│       └── want feature "default"
│           └── want v0.3.1
│               └── try-lock feature "default"
│                   └── try-lock v0.2.5
├── tokio feature "default" (*)
├── tokio feature "fs"
│   └── tokio v1.47.1 (*)
├── tokio feature "io-util" (*)
├── tokio feature "macros"
│   ├── tokio v1.47.1 (*)
│   └── tokio feature "tokio-macros"
│       └── tokio v1.47.1 (*)
├── tokio feature "net"
│   ├── tokio v1.47.1 (*)
│   ├── tokio feature "libc"
│   │   └── tokio v1.47.1 (*)
│   ├── tokio feature "mio"
│   │   └── tokio v1.47.1 (*)
│   ├── tokio feature "socket2"
│   │   └── tokio v1.47.1 (*)
│   ├── mio feature "net"
│   │   └── mio v1.0.4 (*)
│   ├── mio feature "os-ext"
│   │   ├── mio v1.0.4 (*)
│   │   └── mio feature "os-poll"
│   │       └── mio v1.0.4 (*)
│   └── mio feature "os-poll" (*)
├── tokio feature "rt-multi-thread"
│   ├── tokio v1.47.1 (*)
│   └── tokio feature "rt"
│       └── tokio v1.47.1 (*)
├── tokio feature "signal"
│   ├── tokio v1.47.1 (*)
│   ├── tokio feature "libc" (*)
│   ├── tokio feature "mio" (*)
│   ├── tokio feature "signal-hook-registry"
│   │   └── tokio v1.47.1 (*)
│   ├── mio feature "net" (*)
│   ├── mio feature "os-ext" (*)
│   └── mio feature "os-poll" (*)
├── tokio feature "time"
│   └── tokio v1.47.1 (*)
├── tokio-util feature "codec" (*)
├── tokio-util feature "default" (*)
├── tracing feature "default"
... (truncated)
```

</details>

### gwsmoke

<details><summary>Reverse tree (-i gwsmoke -e features)</summary>

```text
```

</details>

<details><summary>Forward tree (-p gwsmoke -e features)</summary>

```text
gwsmoke v0.1.0 (/Users/mymac/Desktop/RustyOnions/testing/gwsmoke)
├── anyhow feature "default"
│   ├── anyhow v1.0.99
│   └── anyhow feature "std"
│       └── anyhow v1.0.99
├── axum feature "http1"
│   ├── axum v0.7.9
│   │   ├── async-trait feature "default"
│   │   │   └── async-trait v0.1.89 (proc-macro)
│   │   │       ├── proc-macro2 feature "default"
│   │   │       │   ├── proc-macro2 v1.0.101
│   │   │       │   │   └── unicode-ident feature "default"
│   │   │       │   │       └── unicode-ident v1.0.19
│   │   │       │   └── proc-macro2 feature "proc-macro"
│   │   │       │       └── proc-macro2 v1.0.101 (*)
│   │   │       ├── quote feature "default"
│   │   │       │   ├── quote v1.0.40
│   │   │       │   │   └── proc-macro2 v1.0.101 (*)
│   │   │       │   └── quote feature "proc-macro"
│   │   │       │       ├── quote v1.0.40 (*)
│   │   │       │       └── proc-macro2 feature "proc-macro" (*)
│   │   │       ├── syn feature "clone-impls"
│   │   │       │   └── syn v2.0.106
│   │   │       │       ├── proc-macro2 v1.0.101 (*)
│   │   │       │       ├── quote v1.0.40 (*)
│   │   │       │       └── unicode-ident feature "default" (*)
│   │   │       ├── syn feature "full"
│   │   │       │   └── syn v2.0.106 (*)
│   │   │       ├── syn feature "parsing"
│   │   │       │   └── syn v2.0.106 (*)
│   │   │       ├── syn feature "printing"
│   │   │       │   └── syn v2.0.106 (*)
│   │   │       ├── syn feature "proc-macro"
│   │   │       │   ├── syn v2.0.106 (*)
│   │   │       │   ├── proc-macro2 feature "proc-macro" (*)
│   │   │       │   └── quote feature "proc-macro" (*)
│   │   │       └── syn feature "visit-mut"
│   │   │           └── syn v2.0.106 (*)
│   │   ├── axum-core feature "default"
│   │   │   └── axum-core v0.4.5
│   │   │       ├── async-trait feature "default" (*)
│   │   │       ├── bytes feature "default"
│   │   │       │   ├── bytes v1.10.1
│   │   │       │   └── bytes feature "std"
│   │   │       │       └── bytes v1.10.1
│   │   │       ├── futures-util feature "alloc"
│   │   │       │   ├── futures-util v0.3.31
│   │   │       │   │   ├── futures-core v0.3.31
│   │   │       │   │   ├── futures-macro v0.3.31 (proc-macro)
│   │   │       │   │   │   ├── proc-macro2 feature "default" (*)
│   │   │       │   │   │   ├── quote feature "default" (*)
│   │   │       │   │   │   ├── syn feature "default"
│   │   │       │   │   │   │   ├── syn v2.0.106 (*)
│   │   │       │   │   │   │   ├── syn feature "clone-impls" (*)
│   │   │       │   │   │   │   ├── syn feature "derive"
│   │   │       │   │   │   │   │   └── syn v2.0.106 (*)
│   │   │       │   │   │   │   ├── syn feature "parsing" (*)
│   │   │       │   │   │   │   ├── syn feature "printing" (*)
│   │   │       │   │   │   │   └── syn feature "proc-macro" (*)
│   │   │       │   │   │   └── syn feature "full" (*)
│   │   │       │   │   ├── futures-sink v0.3.31
│   │   │       │   │   ├── futures-task v0.3.31
│   │   │       │   │   ├── futures-channel feature "std"
│   │   │       │   │   │   ├── futures-channel v0.3.31
│   │   │       │   │   │   │   ├── futures-core v0.3.31
│   │   │       │   │   │   │   └── futures-sink v0.3.31
│   │   │       │   │   │   ├── futures-channel feature "alloc"
│   │   │       │   │   │   │   ├── futures-channel v0.3.31 (*)
│   │   │       │   │   │   │   └── futures-core feature "alloc"
│   │   │       │   │   │   │       └── futures-core v0.3.31
│   │   │       │   │   │   └── futures-core feature "std"
│   │   │       │   │   │       ├── futures-core v0.3.31
│   │   │       │   │   │       └── futures-core feature "alloc" (*)
│   │   │       │   │   ├── futures-io feature "std"
│   │   │       │   │   │   └── futures-io v0.3.31
│   │   │       │   │   ├── memchr feature "default"
│   │   │       │   │   │   ├── memchr v2.7.5
│   │   │       │   │   │   └── memchr feature "std"
│   │   │       │   │   │       ├── memchr v2.7.5
│   │   │       │   │   │       └── memchr feature "alloc"
│   │   │       │   │   │           └── memchr v2.7.5
│   │   │       │   │   ├── pin-project-lite feature "default"
│   │   │       │   │   │   └── pin-project-lite v0.2.16
│   │   │       │   │   ├── pin-utils feature "default"
│   │   │       │   │   │   └── pin-utils v0.1.0
│   │   │       │   │   └── slab feature "default"
│   │   │       │   │       ├── slab v0.4.11
│   │   │       │   │       └── slab feature "std"
│   │   │       │   │           └── slab v0.4.11
│   │   │       │   ├── futures-core feature "alloc" (*)
│   │   │       │   └── futures-task feature "alloc"
│   │   │       │       └── futures-task v0.3.31
│   │   │       ├── pin-project-lite feature "default" (*)
│   │   │       ├── http feature "default"
│   │   │       │   ├── http v1.3.1
│   │   │       │   │   ├── bytes feature "default" (*)
│   │   │       │   │   ├── fnv feature "default"
│   │   │       │   │   │   ├── fnv v1.0.7
│   │   │       │   │   │   └── fnv feature "std"
│   │   │       │   │   │       └── fnv v1.0.7
│   │   │       │   │   └── itoa feature "default"
│   │   │       │   │       └── itoa v1.0.15
│   │   │       │   └── http feature "std"
│   │   │       │       └── http v1.3.1 (*)
│   │   │       ├── http-body feature "default"
│   │   │       │   └── http-body v1.0.1
│   │   │       │       ├── bytes feature "default" (*)
│   │   │       │       └── http feature "default" (*)
│   │   │       ├── http-body-util feature "default"
│   │   │       │   └── http-body-util v0.1.3
│   │   │       │       ├── futures-core v0.3.31
│   │   │       │       ├── bytes feature "default" (*)
│   │   │       │       ├── pin-project-lite feature "default" (*)
│   │   │       │       ├── http feature "default" (*)
│   │   │       │       └── http-body feature "default" (*)
│   │   │       ├── mime feature "default"
│   │   │       │   └── mime v0.3.17
│   │   │       ├── rustversion feature "default"
│   │   │       │   └── rustversion v1.0.22 (proc-macro)
│   │   │       ├── sync_wrapper feature "default"
│   │   │       │   └── sync_wrapper v1.0.2
│   │   │       │       └── futures-core v0.3.31
│   │   │       ├── tower-layer feature "default"
│   │   │       │   └── tower-layer v0.3.3
│   │   │       └── tower-service feature "default"
│   │   │           └── tower-service v0.3.3
│   │   ├── bytes feature "default" (*)
│   │   ├── futures-util feature "alloc" (*)
│   │   ├── memchr feature "default" (*)
│   │   ├── pin-project-lite feature "default" (*)
│   │   ├── http feature "default" (*)
│   │   ├── itoa feature "default" (*)
│   │   ├── http-body feature "default" (*)
│   │   ├── http-body-util feature "default" (*)
│   │   ├── mime feature "default" (*)
│   │   ├── rustversion feature "default" (*)
│   │   ├── sync_wrapper feature "default" (*)
│   │   ├── tower-layer feature "default" (*)
│   │   ├── tower-service feature "default" (*)
│   │   ├── axum-macros feature "default"
│   │   │   └── axum-macros v0.4.2 (proc-macro)
│   │   │       ├── proc-macro2 feature "default" (*)
│   │   │       ├── quote feature "default" (*)
│   │   │       ├── syn feature "default" (*)
│   │   │       ├── syn feature "extra-traits"
│   │   │       │   └── syn v2.0.106 (*)
│   │   │       ├── syn feature "full" (*)
│   │   │       └── syn feature "parsing" (*)
│   │   ├── hyper feature "default"
│   │   │   └── hyper v1.7.0
│   │   │       ├── bytes feature "default" (*)
│   │   │       ├── futures-channel feature "default"
│   │   │       │   ├── futures-channel v0.3.31 (*)
│   │   │       │   └── futures-channel feature "std" (*)
│   │   │       ├── futures-core feature "default"
│   │   │       │   ├── futures-core v0.3.31
│   │   │       │   └── futures-core feature "std" (*)
│   │   │       ├── pin-project-lite feature "default" (*)
│   │   │       ├── pin-utils feature "default" (*)
│   │   │       ├── http feature "default" (*)
│   │   │       ├── itoa feature "default" (*)
│   │   │       ├── http-body feature "default" (*)
│   │   │       ├── atomic-waker feature "default"
│   │   │       │   └── atomic-waker v1.1.2
│   │   │       ├── h2 feature "default"
│   │   │       │   └── h2 v0.4.12
│   │   │       │       ├── futures-core v0.3.31
│   │   │       │       ├── futures-sink v0.3.31
│   │   │       │       ├── bytes feature "default" (*)
│   │   │       │       ├── slab feature "default" (*)
│   │   │       │       ├── http feature "default" (*)
│   │   │       │       ├── fnv feature "default" (*)
│   │   │       │       ├── atomic-waker feature "default" (*)
│   │   │       │       ├── indexmap feature "default"
│   │   │       │       │   ├── indexmap v2.11.1
│   │   │       │       │   │   ├── equivalent v1.0.2
│   │   │       │       │   │   └── hashbrown v0.15.5
│   │   │       │       │   │       ├── equivalent v1.0.2
│   │   │       │       │   │       ├── foldhash v0.1.5
│   │   │       │       │   │       └── allocator-api2 feature "alloc"
│   │   │       │       │   │           └── allocator-api2 v0.2.21
│   │   │       │       │   └── indexmap feature "std"
│   │   │       │       │       └── indexmap v2.11.1 (*)
│   │   │       │       ├── indexmap feature "std" (*)
│   │   │       │       ├── tokio feature "default"
│   │   │       │       │   └── tokio v1.47.1
│   │   │       │       │       ├── mio v1.0.4
│   │   │       │       │       │   └── libc feature "default"
│   │   │       │       │       │       ├── libc v0.2.175
│   │   │       │       │       │       └── libc feature "std"
│   │   │       │       │       │           └── libc v0.2.175
│   │   │       │       │       ├── bytes feature "default" (*)
│   │   │       │       │       ├── pin-project-lite feature "default" (*)
│   │   │       │       │       ├── libc feature "default" (*)
│   │   │       │       │       ├── parking_lot feature "default"
│   │   │       │       │       │   └── parking_lot v0.12.4
│   │   │       │       │       │       ├── lock_api feature "default"
│   │   │       │       │       │       │   ├── lock_api v0.4.13
│   │   │       │       │       │       │   │   └── scopeguard v1.2.0
│   │   │       │       │       │       │   │   [build-dependencies]
│   │   │       │       │       │       │   │   └── autocfg feature "default"
│   │   │       │       │       │       │   │       └── autocfg v1.5.0
│   │   │       │       │       │       │   └── lock_api feature "atomic_usize"
│   │   │       │       │       │       │       └── lock_api v0.4.13 (*)
│   │   │       │       │       │       └── parking_lot_core feature "default"
│   │   │       │       │       │           └── parking_lot_core v0.9.11
│   │   │       │       │       │               ├── libc feature "default" (*)
│   │   │       │       │       │               ├── cfg-if feature "default"
│   │   │       │       │       │               │   └── cfg-if v1.0.3
│   │   │       │       │       │               └── smallvec feature "default"
│   │   │       │       │       │                   └── smallvec v1.15.1
│   │   │       │       │       ├── signal-hook-registry feature "default"
│   │   │       │       │       │   └── signal-hook-registry v1.4.6
│   │   │       │       │       │       └── libc feature "default" (*)
│   │   │       │       │       ├── socket2 feature "all"
│   │   │       │       │       │   └── socket2 v0.6.0
│   │   │       │       │       │       └── libc feature "default" (*)
│   │   │       │       │       ├── socket2 feature "default"
│   │   │       │       │       │   └── socket2 v0.6.0 (*)
│   │   │       │       │       └── tokio-macros feature "default"
│   │   │       │       │           └── tokio-macros v2.5.0 (proc-macro)
│   │   │       │       │               ├── proc-macro2 feature "default" (*)
│   │   │       │       │               ├── quote feature "default" (*)
│   │   │       │       │               ├── syn feature "default" (*)
│   │   │       │       │               └── syn feature "full" (*)
│   │   │       │       ├── tokio feature "io-util"
│   │   │       │       │   ├── tokio v1.47.1 (*)
│   │   │       │       │   └── tokio feature "bytes"
│   │   │       │       │       └── tokio v1.47.1 (*)
│   │   │       │       ├── tokio-util feature "codec"
│   │   │       │       │   └── tokio-util v0.7.16
│   │   │       │       │       ├── bytes feature "default" (*)
│   │   │       │       │       ├── futures-core feature "default" (*)
│   │   │       │       │       ├── futures-sink feature "default"
│   │   │       │       │       │   ├── futures-sink v0.3.31
│   │   │       │       │       │   └── futures-sink feature "std"
│   │   │       │       │       │       ├── futures-sink v0.3.31
│   │   │       │       │       │       └── futures-sink feature "alloc"
│   │   │       │       │       │           └── futures-sink v0.3.31
│   │   │       │       │       ├── pin-project-lite feature "default" (*)
│   │   │       │       │       ├── tokio feature "default" (*)
│   │   │       │       │       └── tokio feature "sync"
│   │   │       │       │           └── tokio v1.47.1 (*)
│   │   │       │       ├── tokio-util feature "default"
│   │   │       │       │   └── tokio-util v0.7.16 (*)
│   │   │       │       ├── tokio-util feature "io"
│   │   │       │       │   └── tokio-util v0.7.16 (*)
│   │   │       │       └── tracing feature "std"
│   │   │       │           ├── tracing v0.1.41
│   │   │       │           │   ├── tracing-core v0.1.34
│   │   │       │           │   │   └── once_cell feature "default"
│   │   │       │           │   │       ├── once_cell v1.21.3
│   │   │       │           │   │       └── once_cell feature "std"
│   │   │       │           │   │           ├── once_cell v1.21.3
│   │   │       │           │   │           └── once_cell feature "alloc"
│   │   │       │           │   │               ├── once_cell v1.21.3
│   │   │       │           │   │               └── once_cell feature "race"
│   │   │       │           │   │                   └── once_cell v1.21.3
│   │   │       │           │   ├── pin-project-lite feature "default" (*)
│   │   │       │           │   └── tracing-attributes feature "default"
│   │   │       │           │       └── tracing-attributes v0.1.30 (proc-macro)
│   │   │       │           │           ├── proc-macro2 feature "default" (*)
│   │   │       │           │           ├── quote feature "default" (*)
│   │   │       │           │           ├── syn feature "clone-impls" (*)
│   │   │       │           │           ├── syn feature "extra-traits" (*)
│   │   │       │           │           ├── syn feature "full" (*)
│   │   │       │           │           ├── syn feature "parsing" (*)
│   │   │       │           │           ├── syn feature "printing" (*)
│   │   │       │           │           ├── syn feature "proc-macro" (*)
│   │   │       │           │           └── syn feature "visit-mut" (*)
│   │   │       │           └── tracing-core feature "std"
│   │   │       │               ├── tracing-core v0.1.34 (*)
│   │   │       │               └── tracing-core feature "once_cell"
│   │   │       │                   └── tracing-core v0.1.34 (*)
│   │   │       ├── tokio feature "default" (*)
│   │   │       ├── tokio feature "sync" (*)
│   │   │       ├── smallvec feature "const_generics"
│   │   │       │   └── smallvec v1.15.1
│   │   │       ├── smallvec feature "const_new"
│   │   │       │   ├── smallvec v1.15.1
│   │   │       │   └── smallvec feature "const_generics" (*)
│   │   │       ├── smallvec feature "default" (*)
│   │   │       ├── httparse feature "default"
│   │   │       │   ├── httparse v1.10.1
│   │   │       │   └── httparse feature "std"
│   │   │       │       └── httparse v1.10.1
│   │   │       ├── httpdate feature "default"
│   │   │       │   └── httpdate v1.0.3
│   │   │       └── want feature "default"
│   │   │           └── want v0.3.1
│   │   │               └── try-lock feature "default"
│   │   │                   └── try-lock v0.2.5
│   │   ├── tokio feature "default" (*)
│   │   ├── tokio feature "time"
│   │   │   └── tokio v1.47.1 (*)
│   │   ├── hyper-util feature "default"
│   │   │   └── hyper-util v0.1.16
│   │   │       ├── futures-util v0.3.31 (*)
│   │   │       ├── tokio v1.47.1 (*)
│   │   │       ├── bytes feature "default" (*)
... (truncated)
```

</details>

### ron-kms

<details><summary>Reverse tree (-i ron-kms -e features)</summary>

```text
```

</details>

<details><summary>Forward tree (-p ron-kms -e features)</summary>

```text
ron-kms v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/ron-kms)
├── anyhow feature "default"
│   ├── anyhow v1.0.99
│   └── anyhow feature "std"
│       └── anyhow v1.0.99
├── hex feature "default"
│   ├── hex v0.4.3
│   └── hex feature "std"
│       ├── hex v0.4.3
│       └── hex feature "alloc"
│           └── hex v0.4.3
├── hmac feature "default"
│   └── hmac v0.12.1
│       ├── digest feature "default"
│       │   ├── digest v0.10.7
│       │   │   ├── subtle v2.6.1
│       │   │   ├── block-buffer feature "default"
│       │   │   │   └── block-buffer v0.10.4
│       │   │   │       └── generic-array feature "default"
│       │   │   │           └── generic-array v0.14.7
│       │   │   │               └── typenum feature "default"
│       │   │   │                   └── typenum v1.18.0
│       │   │   │               [build-dependencies]
│       │   │   │               └── version_check feature "default"
│       │   │   │                   └── version_check v0.9.5
│       │   │   └── crypto-common feature "default"
│       │   │       └── crypto-common v0.1.6
│       │   │           ├── generic-array feature "default" (*)
│       │   │           ├── generic-array feature "more_lengths"
│       │   │           │   └── generic-array v0.14.7 (*)
│       │   │           └── typenum feature "default" (*)
│       │   └── digest feature "core-api"
│       │       ├── digest v0.10.7 (*)
│       │       └── digest feature "block-buffer"
│       │           └── digest v0.10.7 (*)
│       └── digest feature "mac"
│           ├── digest v0.10.7 (*)
│           └── digest feature "subtle"
│               └── digest v0.10.7 (*)
├── parking_lot feature "default"
│   └── parking_lot v0.12.4
│       ├── lock_api feature "default"
│       │   ├── lock_api v0.4.13
│       │   │   └── scopeguard v1.2.0
│       │   │   [build-dependencies]
│       │   │   └── autocfg feature "default"
│       │   │       └── autocfg v1.5.0
│       │   └── lock_api feature "atomic_usize"
│       │       └── lock_api v0.4.13 (*)
│       └── parking_lot_core feature "default"
│           └── parking_lot_core v0.9.11
│               ├── cfg-if feature "default"
│               │   └── cfg-if v1.0.3
│               ├── libc feature "default"
│               │   ├── libc v0.2.175
│               │   └── libc feature "std"
│               │       └── libc v0.2.175
│               └── smallvec feature "default"
│                   └── smallvec v1.15.1
├── rand feature "default"
│   ├── rand v0.9.2
│   │   ├── rand_chacha v0.9.0
│   │   │   ├── ppv-lite86 feature "simd"
│   │   │   │   └── ppv-lite86 v0.2.21
│   │   │   │       ├── zerocopy feature "default"
│   │   │   │       │   └── zerocopy v0.8.27
│   │   │   │       └── zerocopy feature "simd"
│   │   │   │           └── zerocopy v0.8.27
│   │   │   └── rand_core feature "default"
│   │   │       └── rand_core v0.9.3
│   │   │           └── getrandom feature "default"
│   │   │               └── getrandom v0.3.3
│   │   │                   ├── libc v0.2.175
│   │   │                   └── cfg-if feature "default" (*)
│   │   └── rand_core v0.9.3 (*)
│   ├── rand feature "os_rng"
│   │   ├── rand v0.9.2 (*)
│   │   └── rand_core feature "os_rng"
│   │       └── rand_core v0.9.3 (*)
│   ├── rand feature "small_rng"
│   │   └── rand v0.9.2 (*)
│   ├── rand feature "std"
│   │   ├── rand v0.9.2 (*)
│   │   ├── rand feature "alloc"
│   │   │   └── rand v0.9.2 (*)
│   │   ├── rand_chacha feature "std"
│   │   │   ├── rand_chacha v0.9.0 (*)
│   │   │   ├── ppv-lite86 feature "std"
│   │   │   │   └── ppv-lite86 v0.2.21 (*)
│   │   │   └── rand_core feature "std"
│   │   │       ├── rand_core v0.9.3 (*)
│   │   │       └── getrandom feature "std"
│   │   │           └── getrandom v0.3.3 (*)
│   │   └── rand_core feature "std" (*)
│   ├── rand feature "std_rng"
│   │   └── rand v0.9.2 (*)
│   └── rand feature "thread_rng"
│       ├── rand v0.9.2 (*)
│       ├── rand feature "os_rng" (*)
│       ├── rand feature "std" (*)
│       └── rand feature "std_rng" (*)
├── ron-proto feature "default"
│   ├── ron-proto v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/ron-proto)
│   │   ├── hex feature "default" (*)
│   │   ├── base64 feature "default"
│   │   │   ├── base64 v0.22.1
│   │   │   └── base64 feature "std"
│   │   │       ├── base64 v0.22.1
│   │   │       └── base64 feature "alloc"
│   │   │           └── base64 v0.22.1
│   │   ├── rmp-serde feature "default"
│   │   │   └── rmp-serde v1.3.0
│   │   │       ├── byteorder feature "default"
│   │   │       │   ├── byteorder v1.5.0
│   │   │       │   └── byteorder feature "std"
│   │   │       │       └── byteorder v1.5.0
│   │   │       ├── rmp feature "default"
│   │   │       │   ├── rmp v0.8.14
│   │   │       │   │   ├── byteorder v1.5.0
│   │   │       │   │   ├── num-traits v0.2.19
│   │   │       │   │   │   [build-dependencies]
│   │   │       │   │   │   └── autocfg feature "default" (*)
│   │   │       │   │   └── paste feature "default"
│   │   │       │   │       └── paste v1.0.15 (proc-macro)
│   │   │       │   └── rmp feature "std"
│   │   │       │       ├── rmp v0.8.14 (*)
│   │   │       │       ├── byteorder feature "std" (*)
│   │   │       │       └── num-traits feature "std"
│   │   │       │           └── num-traits v0.2.19 (*)
│   │   │       └── serde feature "default"
│   │   │           ├── serde v1.0.221
│   │   │           │   ├── serde_core feature "result"
│   │   │           │   │   └── serde_core v1.0.221
│   │   │           │   └── serde_derive feature "default"
│   │   │           │       └── serde_derive v1.0.221 (proc-macro)
│   │   │           │           ├── proc-macro2 feature "proc-macro"
│   │   │           │           │   └── proc-macro2 v1.0.101
│   │   │           │           │       └── unicode-ident feature "default"
│   │   │           │           │           └── unicode-ident v1.0.19
│   │   │           │           ├── quote feature "proc-macro"
│   │   │           │           │   ├── quote v1.0.40
│   │   │           │           │   │   └── proc-macro2 v1.0.101 (*)
│   │   │           │           │   └── proc-macro2 feature "proc-macro" (*)
│   │   │           │           ├── syn feature "clone-impls"
│   │   │           │           │   └── syn v2.0.106
│   │   │           │           │       ├── proc-macro2 v1.0.101 (*)
│   │   │           │           │       ├── quote v1.0.40 (*)
│   │   │           │           │       └── unicode-ident feature "default" (*)
│   │   │           │           ├── syn feature "derive"
│   │   │           │           │   └── syn v2.0.106 (*)
│   │   │           │           ├── syn feature "parsing"
│   │   │           │           │   └── syn v2.0.106 (*)
│   │   │           │           ├── syn feature "printing"
│   │   │           │           │   └── syn v2.0.106 (*)
│   │   │           │           └── syn feature "proc-macro"
│   │   │           │               ├── syn v2.0.106 (*)
│   │   │           │               ├── proc-macro2 feature "proc-macro" (*)
│   │   │           │               └── quote feature "proc-macro" (*)
│   │   │           └── serde feature "std"
│   │   │               ├── serde v1.0.221 (*)
│   │   │               └── serde_core feature "std"
│   │   │                   └── serde_core v1.0.221
│   │   ├── serde feature "default" (*)
│   │   ├── serde feature "derive"
│   │   │   ├── serde v1.0.221 (*)
│   │   │   └── serde feature "serde_derive"
│   │   │       └── serde v1.0.221 (*)
│   │   ├── sha2 feature "default"
│   │   │   ├── sha2 v0.10.9
│   │   │   │   ├── digest feature "default" (*)
│   │   │   │   ├── cfg-if feature "default" (*)
│   │   │   │   └── cpufeatures feature "default"
│   │   │   │       └── cpufeatures v0.2.17
│   │   │   └── sha2 feature "std"
│   │   │       ├── sha2 v0.10.9 (*)
│   │   │       └── digest feature "std"
│   │   │           ├── digest v0.10.7 (*)
│   │   │           ├── digest feature "alloc"
│   │   │           │   └── digest v0.10.7 (*)
│   │   │           └── crypto-common feature "std"
│   │   │               └── crypto-common v0.1.6 (*)
│   │   └── thiserror feature "default"
│   │       └── thiserror v1.0.69
│   │           └── thiserror-impl feature "default"
│   │               └── thiserror-impl v1.0.69 (proc-macro)
│   │                   ├── proc-macro2 feature "default"
│   │                   │   ├── proc-macro2 v1.0.101 (*)
│   │                   │   └── proc-macro2 feature "proc-macro" (*)
│   │                   ├── quote feature "default"
│   │                   │   ├── quote v1.0.40 (*)
│   │                   │   └── quote feature "proc-macro" (*)
│   │                   └── syn feature "default"
│   │                       ├── syn v2.0.106 (*)
│   │                       ├── syn feature "clone-impls" (*)
│   │                       ├── syn feature "derive" (*)
│   │                       ├── syn feature "parsing" (*)
│   │                       ├── syn feature "printing" (*)
│   │                       └── syn feature "proc-macro" (*)
│   └── ron-proto feature "rmp"
│       └── ron-proto v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/ron-proto) (*)
├── serde feature "default" (*)
├── serde feature "derive" (*)
├── sha2 feature "default" (*)
└── thiserror feature "default" (*)
```

</details>

### ron-proto

<details><summary>Reverse tree (-i ron-proto -e features)</summary>

```text
```

</details>

<details><summary>Forward tree (-p ron-proto -e features)</summary>

```text
ron-proto v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/ron-proto)
├── base64 feature "default"
│   ├── base64 v0.22.1
│   └── base64 feature "std"
│       ├── base64 v0.22.1
│       └── base64 feature "alloc"
│           └── base64 v0.22.1
├── hex feature "default"
│   ├── hex v0.4.3
│   └── hex feature "std"
│       ├── hex v0.4.3
│       └── hex feature "alloc"
│           └── hex v0.4.3
├── rmp-serde feature "default"
│   └── rmp-serde v1.3.0
│       ├── byteorder feature "default"
│       │   ├── byteorder v1.5.0
│       │   └── byteorder feature "std"
│       │       └── byteorder v1.5.0
│       ├── rmp feature "default"
│       │   ├── rmp v0.8.14
│       │   │   ├── byteorder v1.5.0
│       │   │   ├── num-traits v0.2.19
│       │   │   │   [build-dependencies]
│       │   │   │   └── autocfg feature "default"
│       │   │   │       └── autocfg v1.5.0
│       │   │   └── paste feature "default"
│       │   │       └── paste v1.0.15 (proc-macro)
│       │   └── rmp feature "std"
│       │       ├── rmp v0.8.14 (*)
│       │       ├── byteorder feature "std" (*)
│       │       └── num-traits feature "std"
│       │           └── num-traits v0.2.19 (*)
│       └── serde feature "default"
│           ├── serde v1.0.221
│           │   ├── serde_core feature "result"
│           │   │   └── serde_core v1.0.221
│           │   └── serde_derive feature "default"
│           │       └── serde_derive v1.0.221 (proc-macro)
│           │           ├── proc-macro2 feature "proc-macro"
│           │           │   └── proc-macro2 v1.0.101
│           │           │       └── unicode-ident feature "default"
│           │           │           └── unicode-ident v1.0.19
│           │           ├── quote feature "proc-macro"
│           │           │   ├── quote v1.0.40
│           │           │   │   └── proc-macro2 v1.0.101 (*)
│           │           │   └── proc-macro2 feature "proc-macro" (*)
│           │           ├── syn feature "clone-impls"
│           │           │   └── syn v2.0.106
│           │           │       ├── proc-macro2 v1.0.101 (*)
│           │           │       ├── quote v1.0.40 (*)
│           │           │       └── unicode-ident feature "default" (*)
│           │           ├── syn feature "derive"
│           │           │   └── syn v2.0.106 (*)
│           │           ├── syn feature "parsing"
│           │           │   └── syn v2.0.106 (*)
│           │           ├── syn feature "printing"
│           │           │   └── syn v2.0.106 (*)
│           │           └── syn feature "proc-macro"
│           │               ├── syn v2.0.106 (*)
│           │               ├── proc-macro2 feature "proc-macro" (*)
│           │               └── quote feature "proc-macro" (*)
│           └── serde feature "std"
│               ├── serde v1.0.221 (*)
│               └── serde_core feature "std"
│                   └── serde_core v1.0.221
├── serde feature "default" (*)
├── serde feature "derive"
│   ├── serde v1.0.221 (*)
│   └── serde feature "serde_derive"
│       └── serde v1.0.221 (*)
├── sha2 feature "default"
│   ├── sha2 v0.10.9
│   │   ├── cfg-if feature "default"
│   │   │   └── cfg-if v1.0.3
│   │   ├── cpufeatures feature "default"
│   │   │   └── cpufeatures v0.2.17
│   │   └── digest feature "default"
│   │       ├── digest v0.10.7
│   │       │   ├── block-buffer feature "default"
│   │       │   │   └── block-buffer v0.10.4
│   │       │   │       └── generic-array feature "default"
│   │       │   │           └── generic-array v0.14.7
│   │       │   │               └── typenum feature "default"
│   │       │   │                   └── typenum v1.18.0
│   │       │   │               [build-dependencies]
│   │       │   │               └── version_check feature "default"
│   │       │   │                   └── version_check v0.9.5
│   │       │   └── crypto-common feature "default"
│   │       │       └── crypto-common v0.1.6
│   │       │           ├── generic-array feature "default" (*)
│   │       │           ├── generic-array feature "more_lengths"
│   │       │           │   └── generic-array v0.14.7 (*)
│   │       │           └── typenum feature "default" (*)
│   │       └── digest feature "core-api"
│   │           ├── digest v0.10.7 (*)
│   │           └── digest feature "block-buffer"
│   │               └── digest v0.10.7 (*)
│   └── sha2 feature "std"
│       ├── sha2 v0.10.9 (*)
│       └── digest feature "std"
│           ├── digest v0.10.7 (*)
│           ├── digest feature "alloc"
│           │   └── digest v0.10.7 (*)
│           └── crypto-common feature "std"
│               └── crypto-common v0.1.6 (*)
└── thiserror feature "default"
    └── thiserror v1.0.69
        └── thiserror-impl feature "default"
            └── thiserror-impl v1.0.69 (proc-macro)
                ├── proc-macro2 feature "default"
                │   ├── proc-macro2 v1.0.101 (*)
                │   └── proc-macro2 feature "proc-macro" (*)
                ├── quote feature "default"
                │   ├── quote v1.0.40 (*)
                │   └── quote feature "proc-macro" (*)
                └── syn feature "default"
                    ├── syn v2.0.106 (*)
                    ├── syn feature "clone-impls" (*)
                    ├── syn feature "derive" (*)
                    ├── syn feature "parsing" (*)
                    ├── syn feature "printing" (*)
                    └── syn feature "proc-macro" (*)
```

</details>

### ron-auth

<details><summary>Reverse tree (-i ron-auth -e features)</summary>

```text
```

</details>

<details><summary>Forward tree (-p ron-auth -e features)</summary>

```text
ron-auth v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/ron-auth)
├── hmac feature "default"
│   └── hmac v0.12.1
│       ├── digest feature "default"
│       │   ├── digest v0.10.7
│       │   │   ├── subtle v2.6.1
│       │   │   ├── block-buffer feature "default"
│       │   │   │   └── block-buffer v0.10.4
│       │   │   │       └── generic-array feature "default"
│       │   │   │           └── generic-array v0.14.7
│       │   │   │               └── typenum feature "default"
│       │   │   │                   └── typenum v1.18.0
│       │   │   │               [build-dependencies]
│       │   │   │               └── version_check feature "default"
│       │   │   │                   └── version_check v0.9.5
│       │   │   └── crypto-common feature "default"
│       │   │       └── crypto-common v0.1.6
│       │   │           ├── generic-array feature "default" (*)
│       │   │           ├── generic-array feature "more_lengths"
│       │   │           │   └── generic-array v0.14.7 (*)
│       │   │           └── typenum feature "default" (*)
│       │   └── digest feature "core-api"
│       │       ├── digest v0.10.7 (*)
│       │       └── digest feature "block-buffer"
│       │           └── digest v0.10.7 (*)
│       └── digest feature "mac"
│           ├── digest v0.10.7 (*)
│           └── digest feature "subtle"
│               └── digest v0.10.7 (*)
├── rand feature "default"
│   ├── rand v0.8.5
│   │   ├── libc v0.2.175
│   │   ├── rand_chacha v0.3.1
│   │   │   ├── ppv-lite86 feature "simd"
│   │   │   │   └── ppv-lite86 v0.2.21
│   │   │   │       ├── zerocopy feature "default"
│   │   │   │       │   └── zerocopy v0.8.27
│   │   │   │       └── zerocopy feature "simd"
│   │   │   │           └── zerocopy v0.8.27
│   │   │   └── rand_core feature "default"
│   │   │       └── rand_core v0.6.4
│   │   │           └── getrandom feature "default"
│   │   │               └── getrandom v0.2.16
│   │   │                   ├── libc v0.2.175
│   │   │                   └── cfg-if feature "default"
│   │   │                       └── cfg-if v1.0.3
│   │   └── rand_core feature "default" (*)
│   ├── rand feature "std"
│   │   ├── rand v0.8.5 (*)
│   │   ├── rand feature "alloc"
│   │   │   ├── rand v0.8.5 (*)
│   │   │   └── rand_core feature "alloc"
│   │   │       └── rand_core v0.6.4 (*)
│   │   ├── rand feature "getrandom"
│   │   │   ├── rand v0.8.5 (*)
│   │   │   └── rand_core feature "getrandom"
│   │   │       └── rand_core v0.6.4 (*)
│   │   ├── rand feature "libc"
│   │   │   └── rand v0.8.5 (*)
│   │   ├── rand feature "rand_chacha"
│   │   │   └── rand v0.8.5 (*)
│   │   ├── rand_chacha feature "std"
│   │   │   ├── rand_chacha v0.3.1 (*)
│   │   │   └── ppv-lite86 feature "std"
│   │   │       └── ppv-lite86 v0.2.21 (*)
│   │   └── rand_core feature "std"
│   │       ├── rand_core v0.6.4 (*)
│   │       ├── rand_core feature "alloc" (*)
│   │       ├── rand_core feature "getrandom" (*)
│   │       └── getrandom feature "std"
│   │           └── getrandom v0.2.16 (*)
│   └── rand feature "std_rng"
│       ├── rand v0.8.5 (*)
│       └── rand feature "rand_chacha" (*)
├── rmp-serde feature "default"
│   └── rmp-serde v1.3.0
│       ├── byteorder feature "default"
│       │   ├── byteorder v1.5.0
│       │   └── byteorder feature "std"
│       │       └── byteorder v1.5.0
│       ├── rmp feature "default"
│       │   ├── rmp v0.8.14
│       │   │   ├── byteorder v1.5.0
│       │   │   ├── num-traits v0.2.19
│       │   │   │   [build-dependencies]
│       │   │   │   └── autocfg feature "default"
│       │   │   │       └── autocfg v1.5.0
│       │   │   └── paste feature "default"
│       │   │       └── paste v1.0.15 (proc-macro)
│       │   └── rmp feature "std"
│       │       ├── rmp v0.8.14 (*)
│       │       ├── byteorder feature "std" (*)
│       │       └── num-traits feature "std"
│       │           └── num-traits v0.2.19 (*)
│       └── serde feature "default"
│           ├── serde v1.0.221
│           │   ├── serde_core feature "result"
│           │   │   └── serde_core v1.0.221
│           │   └── serde_derive feature "default"
│           │       └── serde_derive v1.0.221 (proc-macro)
│           │           ├── proc-macro2 feature "proc-macro"
│           │           │   └── proc-macro2 v1.0.101
│           │           │       └── unicode-ident feature "default"
│           │           │           └── unicode-ident v1.0.19
│           │           ├── quote feature "proc-macro"
│           │           │   ├── quote v1.0.40
│           │           │   │   └── proc-macro2 v1.0.101 (*)
│           │           │   └── proc-macro2 feature "proc-macro" (*)
│           │           ├── syn feature "clone-impls"
│           │           │   └── syn v2.0.106
│           │           │       ├── proc-macro2 v1.0.101 (*)
│           │           │       ├── quote v1.0.40 (*)
│           │           │       └── unicode-ident feature "default" (*)
│           │           ├── syn feature "derive"
│           │           │   └── syn v2.0.106 (*)
│           │           ├── syn feature "parsing"
│           │           │   └── syn v2.0.106 (*)
│           │           ├── syn feature "printing"
│           │           │   └── syn v2.0.106 (*)
│           │           └── syn feature "proc-macro"
│           │               ├── syn v2.0.106 (*)
│           │               ├── proc-macro2 feature "proc-macro" (*)
│           │               └── quote feature "proc-macro" (*)
│           └── serde feature "std"
│               ├── serde v1.0.221 (*)
│               └── serde_core feature "std"
│                   └── serde_core v1.0.221
├── serde feature "default" (*)
├── serde feature "derive"
│   ├── serde v1.0.221 (*)
│   └── serde feature "serde_derive"
│       └── serde v1.0.221 (*)
├── sha2 feature "default"
│   ├── sha2 v0.10.9
│   │   ├── digest feature "default" (*)
│   │   ├── cfg-if feature "default" (*)
│   │   └── cpufeatures feature "default"
│   │       └── cpufeatures v0.2.17
│   └── sha2 feature "std"
│       ├── sha2 v0.10.9 (*)
│       └── digest feature "std"
│           ├── digest v0.10.7 (*)
│           ├── digest feature "alloc"
│           │   └── digest v0.10.7 (*)
│           └── crypto-common feature "std"
│               └── crypto-common v0.1.6 (*)
├── smallvec feature "default"
│   └── smallvec v1.15.1
├── thiserror feature "default"
│   └── thiserror v1.0.69
│       └── thiserror-impl feature "default"
│           └── thiserror-impl v1.0.69 (proc-macro)
│               ├── proc-macro2 feature "default"
│               │   ├── proc-macro2 v1.0.101 (*)
│               │   └── proc-macro2 feature "proc-macro" (*)
│               ├── quote feature "default"
│               │   ├── quote v1.0.40 (*)
│               │   └── quote feature "proc-macro" (*)
│               └── syn feature "default"
│                   ├── syn v2.0.106 (*)
│                   ├── syn feature "clone-impls" (*)
│                   ├── syn feature "derive" (*)
│                   ├── syn feature "parsing" (*)
│                   ├── syn feature "printing" (*)
│                   └── syn feature "proc-macro" (*)
├── time feature "default"
│   ├── time v0.3.43
│   │   ├── powerfmt v0.2.0
│   │   ├── deranged feature "default"
│   │   │   └── deranged v0.5.3
│   │   │       └── powerfmt v0.2.0
│   │   ├── deranged feature "powerfmt"
│   │   │   └── deranged v0.5.3 (*)
│   │   ├── num-conv feature "default"
│   │   │   └── num-conv v0.1.0
│   │   └── time-core feature "default"
│   │       └── time-core v0.1.6
│   └── time feature "std"
│       ├── time v0.3.43 (*)
│       └── time feature "alloc"
│           └── time v0.3.43 (*)
└── uuid feature "default"
    ├── uuid v1.18.1
    └── uuid feature "std"
        └── uuid v1.18.1
```

</details>

### ron-audit

<details><summary>Reverse tree (-i ron-audit -e features)</summary>

```text
```

</details>

<details><summary>Forward tree (-p ron-audit -e features)</summary>

```text
ron-audit v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/ron-audit)
├── anyhow feature "default"
│   ├── anyhow v1.0.99
│   └── anyhow feature "std"
│       └── anyhow v1.0.99
├── blake3 feature "default"
│   ├── blake3 v1.8.2
│   │   ├── arrayvec v0.7.6
│   │   ├── constant_time_eq v0.3.1
│   │   ├── arrayref feature "default"
│   │   │   └── arrayref v0.3.9
│   │   └── cfg-if feature "default"
│   │       └── cfg-if v1.0.3
│   │   [build-dependencies]
│   │   └── cc feature "default"
│   │       └── cc v1.2.37
│   │           ├── find-msvc-tools feature "default"
│   │           │   └── find-msvc-tools v0.1.1
│   │           └── shlex feature "default"
│   │               ├── shlex v1.3.0
│   │               └── shlex feature "std"
│   │                   └── shlex v1.3.0
│   └── blake3 feature "std"
│       └── blake3 v1.8.2 (*)
├── ed25519-dalek feature "default"
│   ├── ed25519-dalek v2.2.0
│   │   ├── ed25519 v2.2.3
│   │   │   └── signature v2.2.0
│   │   ├── rand_core v0.6.4
│   │   │   └── getrandom feature "default"
│   │   │       └── getrandom v0.2.16
│   │   │           ├── libc v0.2.175
│   │   │           └── cfg-if feature "default" (*)
│   │   ├── sha2 v0.10.9
│   │   │   ├── cfg-if feature "default" (*)
│   │   │   ├── cpufeatures feature "default"
│   │   │   │   └── cpufeatures v0.2.17
│   │   │   └── digest feature "default"
│   │   │       ├── digest v0.10.7
│   │   │       │   ├── block-buffer feature "default"
│   │   │       │   │   └── block-buffer v0.10.4
│   │   │       │   │       └── generic-array feature "default"
│   │   │       │   │           └── generic-array v0.14.7
│   │   │       │   │               └── typenum feature "default"
│   │   │       │   │                   └── typenum v1.18.0
│   │   │       │   │               [build-dependencies]
│   │   │       │   │               └── version_check feature "default"
│   │   │       │   │                   └── version_check v0.9.5
│   │   │       │   └── crypto-common feature "default"
│   │   │       │       └── crypto-common v0.1.6
│   │   │       │           ├── generic-array feature "default" (*)
│   │   │       │           ├── generic-array feature "more_lengths"
│   │   │       │           │   └── generic-array v0.14.7 (*)
│   │   │       │           └── typenum feature "default" (*)
│   │   │       └── digest feature "core-api"
│   │   │           ├── digest v0.10.7 (*)
│   │   │           └── digest feature "block-buffer"
│   │   │               └── digest v0.10.7 (*)
│   │   ├── subtle v2.6.1
│   │   ├── zeroize v1.8.1
│   │   └── curve25519-dalek feature "digest"
│   │       └── curve25519-dalek v4.1.3
│   │           ├── digest v0.10.7 (*)
│   │           ├── subtle v2.6.1
│   │           ├── zeroize v1.8.1
│   │           ├── cfg-if feature "default" (*)
│   │           ├── cpufeatures feature "default" (*)
│   │           └── curve25519-dalek-derive feature "default"
│   │               └── curve25519-dalek-derive v0.1.1 (proc-macro)
│   │                   ├── proc-macro2 feature "default"
│   │                   │   ├── proc-macro2 v1.0.101
│   │                   │   │   └── unicode-ident feature "default"
│   │                   │   │       └── unicode-ident v1.0.19
│   │                   │   └── proc-macro2 feature "proc-macro"
│   │                   │       └── proc-macro2 v1.0.101 (*)
│   │                   ├── quote feature "default"
│   │                   │   ├── quote v1.0.40
│   │                   │   │   └── proc-macro2 v1.0.101 (*)
│   │                   │   └── quote feature "proc-macro"
│   │                   │       ├── quote v1.0.40 (*)
│   │                   │       └── proc-macro2 feature "proc-macro" (*)
│   │                   ├── syn feature "default"
│   │                   │   ├── syn v2.0.106
│   │                   │   │   ├── proc-macro2 v1.0.101 (*)
│   │                   │   │   ├── quote v1.0.40 (*)
│   │                   │   │   └── unicode-ident feature "default" (*)
│   │                   │   ├── syn feature "clone-impls"
│   │                   │   │   └── syn v2.0.106 (*)
│   │                   │   ├── syn feature "derive"
│   │                   │   │   └── syn v2.0.106 (*)
│   │                   │   ├── syn feature "parsing"
│   │                   │   │   └── syn v2.0.106 (*)
│   │                   │   ├── syn feature "printing"
│   │                   │   │   └── syn v2.0.106 (*)
│   │                   │   └── syn feature "proc-macro"
│   │                   │       ├── syn v2.0.106 (*)
│   │                   │       ├── proc-macro2 feature "proc-macro" (*)
│   │                   │       └── quote feature "proc-macro" (*)
│   │                   └── syn feature "full"
│   │                       └── syn v2.0.106 (*)
│   │           [build-dependencies]
│   │           └── rustc_version feature "default"
│   │               └── rustc_version v0.4.1
│   │                   └── semver feature "default"
│   │                       ├── semver v1.0.27
│   │                       └── semver feature "std"
│   │                           └── semver v1.0.27
│   ├── ed25519-dalek feature "fast"
│   │   ├── ed25519-dalek v2.2.0 (*)
│   │   └── curve25519-dalek feature "precomputed-tables"
│   │       └── curve25519-dalek v4.1.3 (*)
│   ├── ed25519-dalek feature "std"
│   │   ├── ed25519-dalek v2.2.0 (*)
│   │   ├── ed25519-dalek feature "alloc"
│   │   │   ├── ed25519-dalek v2.2.0 (*)
│   │   │   ├── ed25519-dalek feature "zeroize"
│   │   │   │   └── ed25519-dalek v2.2.0 (*)
│   │   │   ├── curve25519-dalek feature "alloc"
│   │   │   │   ├── curve25519-dalek v4.1.3 (*)
│   │   │   │   └── zeroize feature "alloc"
│   │   │   │       └── zeroize v1.8.1
│   │   │   ├── zeroize feature "alloc" (*)
│   │   │   └── ed25519 feature "alloc"
│   │   │       └── ed25519 v2.2.3 (*)
│   │   ├── ed25519 feature "std"
│   │   │   ├── ed25519 v2.2.3 (*)
│   │   │   └── signature feature "std"
│   │   │       ├── signature v2.2.0
│   │   │       └── signature feature "alloc"
│   │   │           └── signature v2.2.0
│   │   └── sha2 feature "std"
│   │       ├── sha2 v0.10.9 (*)
│   │       └── digest feature "std"
│   │           ├── digest v0.10.7 (*)
│   │           ├── digest feature "alloc"
│   │           │   └── digest v0.10.7 (*)
│   │           └── crypto-common feature "std"
│   │               └── crypto-common v0.1.6 (*)
│   └── ed25519-dalek feature "zeroize" (*)
├── ed25519-dalek feature "rand_core"
│   └── ed25519-dalek v2.2.0 (*)
├── rand feature "default"
│   ├── rand v0.8.5
│   │   ├── libc v0.2.175
│   │   ├── rand_chacha v0.3.1
│   │   │   ├── rand_core feature "default"
│   │   │   │   └── rand_core v0.6.4 (*)
│   │   │   └── ppv-lite86 feature "simd"
│   │   │       └── ppv-lite86 v0.2.21
│   │   │           ├── zerocopy feature "default"
│   │   │           │   └── zerocopy v0.8.27
│   │   │           └── zerocopy feature "simd"
│   │   │               └── zerocopy v0.8.27
│   │   └── rand_core feature "default" (*)
│   ├── rand feature "std"
│   │   ├── rand v0.8.5 (*)
│   │   ├── rand_core feature "std"
│   │   │   ├── rand_core v0.6.4 (*)
│   │   │   ├── rand_core feature "alloc"
│   │   │   │   └── rand_core v0.6.4 (*)
│   │   │   ├── rand_core feature "getrandom"
│   │   │   │   └── rand_core v0.6.4 (*)
│   │   │   └── getrandom feature "std"
│   │   │       └── getrandom v0.2.16 (*)
│   │   ├── rand feature "alloc"
│   │   │   ├── rand v0.8.5 (*)
│   │   │   └── rand_core feature "alloc" (*)
│   │   ├── rand feature "getrandom"
│   │   │   ├── rand v0.8.5 (*)
│   │   │   └── rand_core feature "getrandom" (*)
│   │   ├── rand feature "libc"
│   │   │   └── rand v0.8.5 (*)
│   │   ├── rand feature "rand_chacha"
│   │   │   └── rand v0.8.5 (*)
│   │   └── rand_chacha feature "std"
│   │       ├── rand_chacha v0.3.1 (*)
│   │       └── ppv-lite86 feature "std"
│   │           └── ppv-lite86 v0.2.21 (*)
│   └── rand feature "std_rng"
│       ├── rand v0.8.5 (*)
│       └── rand feature "rand_chacha" (*)
├── rmp-serde feature "default"
│   └── rmp-serde v1.3.0
│       ├── byteorder feature "default"
│       │   ├── byteorder v1.5.0
│       │   └── byteorder feature "std"
│       │       └── byteorder v1.5.0
│       ├── rmp feature "default"
│       │   ├── rmp v0.8.14
│       │   │   ├── byteorder v1.5.0
│       │   │   ├── num-traits v0.2.19
│       │   │   │   [build-dependencies]
│       │   │   │   └── autocfg feature "default"
│       │   │   │       └── autocfg v1.5.0
│       │   │   └── paste feature "default"
│       │   │       └── paste v1.0.15 (proc-macro)
│       │   └── rmp feature "std"
│       │       ├── rmp v0.8.14 (*)
│       │       ├── byteorder feature "std" (*)
│       │       └── num-traits feature "std"
│       │           └── num-traits v0.2.19 (*)
│       └── serde feature "default"
│           ├── serde v1.0.221
│           │   ├── serde_core feature "result"
│           │   │   └── serde_core v1.0.221
│           │   └── serde_derive feature "default"
│           │       └── serde_derive v1.0.221 (proc-macro)
│           │           ├── proc-macro2 feature "proc-macro" (*)
│           │           ├── quote feature "proc-macro" (*)
│           │           ├── syn feature "clone-impls" (*)
│           │           ├── syn feature "derive" (*)
│           │           ├── syn feature "parsing" (*)
│           │           ├── syn feature "printing" (*)
│           │           └── syn feature "proc-macro" (*)
│           └── serde feature "std"
│               ├── serde v1.0.221 (*)
│               └── serde_core feature "std"
│                   └── serde_core v1.0.221
├── serde feature "default" (*)
├── serde feature "derive"
│   ├── serde v1.0.221 (*)
│   └── serde feature "serde_derive"
│       └── serde v1.0.221 (*)
├── serde_json feature "default"
│   ├── serde_json v1.0.144
│   │   ├── memchr v2.7.5
│   │   ├── serde_core v1.0.221
│   │   ├── itoa feature "default"
│   │   │   └── itoa v1.0.15
│   │   └── ryu feature "default"
│   │       └── ryu v1.0.20
│   └── serde_json feature "std"
│       ├── serde_json v1.0.144 (*)
│       ├── serde_core feature "std" (*)
│       └── memchr feature "std"
│           ├── memchr v2.7.5
│           └── memchr feature "alloc"
│               └── memchr v2.7.5
└── time feature "default"
    ├── time v0.3.43
    │   ├── powerfmt v0.2.0
    │   ├── deranged feature "default"
    │   │   └── deranged v0.5.3
    │   │       └── powerfmt v0.2.0
    │   ├── deranged feature "powerfmt"
    │   │   └── deranged v0.5.3 (*)
    │   ├── num-conv feature "default"
    │   │   └── num-conv v0.1.0
    │   └── time-core feature "default"
    │       └── time-core v0.1.6
    └── time feature "std"
        ├── time v0.3.43 (*)
        └── time feature "alloc"
            └── time v0.3.43 (*)
```

</details>

### micronode

<details><summary>Reverse tree (-i micronode -e features)</summary>

```text
```

</details>

<details><summary>Forward tree (-p micronode -e features)</summary>

```text
micronode v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/micronode)
├── axum feature "http1"
│   ├── axum v0.7.9
│   │   ├── async-trait feature "default"
│   │   │   └── async-trait v0.1.89 (proc-macro)
│   │   │       ├── proc-macro2 feature "default"
│   │   │       │   ├── proc-macro2 v1.0.101
│   │   │       │   │   └── unicode-ident feature "default"
│   │   │       │   │       └── unicode-ident v1.0.19
│   │   │       │   └── proc-macro2 feature "proc-macro"
│   │   │       │       └── proc-macro2 v1.0.101 (*)
│   │   │       ├── quote feature "default"
│   │   │       │   ├── quote v1.0.40
│   │   │       │   │   └── proc-macro2 v1.0.101 (*)
│   │   │       │   └── quote feature "proc-macro"
│   │   │       │       ├── quote v1.0.40 (*)
│   │   │       │       └── proc-macro2 feature "proc-macro" (*)
│   │   │       ├── syn feature "clone-impls"
│   │   │       │   └── syn v2.0.106
│   │   │       │       ├── proc-macro2 v1.0.101 (*)
│   │   │       │       ├── quote v1.0.40 (*)
│   │   │       │       └── unicode-ident feature "default" (*)
│   │   │       ├── syn feature "full"
│   │   │       │   └── syn v2.0.106 (*)
│   │   │       ├── syn feature "parsing"
│   │   │       │   └── syn v2.0.106 (*)
│   │   │       ├── syn feature "printing"
│   │   │       │   └── syn v2.0.106 (*)
│   │   │       ├── syn feature "proc-macro"
│   │   │       │   ├── syn v2.0.106 (*)
│   │   │       │   ├── proc-macro2 feature "proc-macro" (*)
│   │   │       │   └── quote feature "proc-macro" (*)
│   │   │       └── syn feature "visit-mut"
│   │   │           └── syn v2.0.106 (*)
│   │   ├── axum-core feature "default"
│   │   │   └── axum-core v0.4.5
│   │   │       ├── async-trait feature "default" (*)
│   │   │       ├── bytes feature "default"
│   │   │       │   ├── bytes v1.10.1
│   │   │       │   └── bytes feature "std"
│   │   │       │       └── bytes v1.10.1
│   │   │       ├── futures-util feature "alloc"
│   │   │       │   ├── futures-util v0.3.31
│   │   │       │   │   ├── futures-core v0.3.31
│   │   │       │   │   ├── futures-task v0.3.31
│   │   │       │   │   ├── pin-project-lite feature "default"
│   │   │       │   │   │   └── pin-project-lite v0.2.16
│   │   │       │   │   └── pin-utils feature "default"
│   │   │       │   │       └── pin-utils v0.1.0
│   │   │       │   ├── futures-core feature "alloc"
│   │   │       │   │   └── futures-core v0.3.31
│   │   │       │   └── futures-task feature "alloc"
│   │   │       │       └── futures-task v0.3.31
│   │   │       ├── pin-project-lite feature "default" (*)
│   │   │       ├── http feature "default"
│   │   │       │   ├── http v1.3.1
│   │   │       │   │   ├── bytes feature "default" (*)
│   │   │       │   │   ├── fnv feature "default"
│   │   │       │   │   │   ├── fnv v1.0.7
│   │   │       │   │   │   └── fnv feature "std"
│   │   │       │   │   │       └── fnv v1.0.7
│   │   │       │   │   └── itoa feature "default"
│   │   │       │   │       └── itoa v1.0.15
│   │   │       │   └── http feature "std"
│   │   │       │       └── http v1.3.1 (*)
│   │   │       ├── http-body feature "default"
│   │   │       │   └── http-body v1.0.1
│   │   │       │       ├── bytes feature "default" (*)
│   │   │       │       └── http feature "default" (*)
│   │   │       ├── http-body-util feature "default"
│   │   │       │   └── http-body-util v0.1.3
│   │   │       │       ├── futures-core v0.3.31
│   │   │       │       ├── bytes feature "default" (*)
│   │   │       │       ├── pin-project-lite feature "default" (*)
│   │   │       │       ├── http feature "default" (*)
│   │   │       │       └── http-body feature "default" (*)
│   │   │       ├── mime feature "default"
│   │   │       │   └── mime v0.3.17
│   │   │       ├── rustversion feature "default"
│   │   │       │   └── rustversion v1.0.22 (proc-macro)
│   │   │       ├── sync_wrapper feature "default"
│   │   │       │   └── sync_wrapper v1.0.2
│   │   │       ├── tower-layer feature "default"
│   │   │       │   └── tower-layer v0.3.3
│   │   │       └── tower-service feature "default"
│   │   │           └── tower-service v0.3.3
│   │   ├── bytes feature "default" (*)
│   │   ├── futures-util feature "alloc" (*)
│   │   ├── pin-project-lite feature "default" (*)
│   │   ├── http feature "default" (*)
│   │   ├── itoa feature "default" (*)
│   │   ├── http-body feature "default" (*)
│   │   ├── http-body-util feature "default" (*)
│   │   ├── mime feature "default" (*)
│   │   ├── rustversion feature "default" (*)
│   │   ├── sync_wrapper feature "default" (*)
│   │   ├── tower-layer feature "default" (*)
│   │   ├── tower-service feature "default" (*)
│   │   ├── hyper feature "default"
│   │   │   └── hyper v1.7.0
│   │   │       ├── bytes feature "default" (*)
│   │   │       ├── futures-core feature "default"
│   │   │       │   ├── futures-core v0.3.31
│   │   │       │   └── futures-core feature "std"
│   │   │       │       ├── futures-core v0.3.31
│   │   │       │       └── futures-core feature "alloc" (*)
│   │   │       ├── pin-project-lite feature "default" (*)
│   │   │       ├── pin-utils feature "default" (*)
│   │   │       ├── http feature "default" (*)
│   │   │       ├── itoa feature "default" (*)
│   │   │       ├── http-body feature "default" (*)
│   │   │       ├── atomic-waker feature "default"
│   │   │       │   └── atomic-waker v1.1.2
│   │   │       ├── futures-channel feature "default"
│   │   │       │   ├── futures-channel v0.3.31
│   │   │       │   │   └── futures-core v0.3.31
│   │   │       │   └── futures-channel feature "std"
│   │   │       │       ├── futures-channel v0.3.31 (*)
│   │   │       │       ├── futures-core feature "std" (*)
│   │   │       │       └── futures-channel feature "alloc"
│   │   │       │           ├── futures-channel v0.3.31 (*)
│   │   │       │           └── futures-core feature "alloc" (*)
│   │   │       ├── h2 feature "default"
│   │   │       │   └── h2 v0.4.12
│   │   │       │       ├── futures-core v0.3.31
│   │   │       │       ├── futures-sink v0.3.31
│   │   │       │       ├── bytes feature "default" (*)
│   │   │       │       ├── http feature "default" (*)
│   │   │       │       ├── fnv feature "default" (*)
│   │   │       │       ├── atomic-waker feature "default" (*)
│   │   │       │       ├── indexmap feature "default"
│   │   │       │       │   ├── indexmap v2.11.1
│   │   │       │       │   │   ├── equivalent v1.0.2
│   │   │       │       │   │   └── hashbrown v0.15.5
│   │   │       │       │   └── indexmap feature "std"
│   │   │       │       │       └── indexmap v2.11.1 (*)
│   │   │       │       ├── indexmap feature "std" (*)
│   │   │       │       ├── slab feature "default"
│   │   │       │       │   ├── slab v0.4.11
│   │   │       │       │   └── slab feature "std"
│   │   │       │       │       └── slab v0.4.11
│   │   │       │       ├── tokio feature "default"
│   │   │       │       │   └── tokio v1.47.1
│   │   │       │       │       ├── mio v1.0.4
│   │   │       │       │       │   └── libc feature "default"
│   │   │       │       │       │       ├── libc v0.2.175
│   │   │       │       │       │       └── libc feature "std"
│   │   │       │       │       │           └── libc v0.2.175
│   │   │       │       │       ├── bytes feature "default" (*)
│   │   │       │       │       ├── pin-project-lite feature "default" (*)
│   │   │       │       │       ├── libc feature "default" (*)
│   │   │       │       │       ├── signal-hook-registry feature "default"
│   │   │       │       │       │   └── signal-hook-registry v1.4.6
│   │   │       │       │       │       └── libc feature "default" (*)
│   │   │       │       │       ├── socket2 feature "all"
│   │   │       │       │       │   └── socket2 v0.6.0
│   │   │       │       │       │       └── libc feature "default" (*)
│   │   │       │       │       ├── socket2 feature "default"
│   │   │       │       │       │   └── socket2 v0.6.0 (*)
│   │   │       │       │       └── tokio-macros feature "default"
│   │   │       │       │           └── tokio-macros v2.5.0 (proc-macro)
│   │   │       │       │               ├── proc-macro2 feature "default" (*)
│   │   │       │       │               ├── quote feature "default" (*)
│   │   │       │       │               ├── syn feature "default"
│   │   │       │       │               │   ├── syn v2.0.106 (*)
│   │   │       │       │               │   ├── syn feature "clone-impls" (*)
│   │   │       │       │               │   ├── syn feature "derive"
│   │   │       │       │               │   │   └── syn v2.0.106 (*)
│   │   │       │       │               │   ├── syn feature "parsing" (*)
│   │   │       │       │               │   ├── syn feature "printing" (*)
│   │   │       │       │               │   └── syn feature "proc-macro" (*)
│   │   │       │       │               └── syn feature "full" (*)
│   │   │       │       ├── tokio feature "io-util"
│   │   │       │       │   ├── tokio v1.47.1 (*)
│   │   │       │       │   └── tokio feature "bytes"
│   │   │       │       │       └── tokio v1.47.1 (*)
│   │   │       │       ├── tokio-util feature "codec"
│   │   │       │       │   └── tokio-util v0.7.16
│   │   │       │       │       ├── bytes feature "default" (*)
│   │   │       │       │       ├── futures-core feature "default" (*)
│   │   │       │       │       ├── pin-project-lite feature "default" (*)
│   │   │       │       │       ├── futures-sink feature "default"
│   │   │       │       │       │   ├── futures-sink v0.3.31
│   │   │       │       │       │   └── futures-sink feature "std"
│   │   │       │       │       │       ├── futures-sink v0.3.31
│   │   │       │       │       │       └── futures-sink feature "alloc"
│   │   │       │       │       │           └── futures-sink v0.3.31
│   │   │       │       │       ├── tokio feature "default" (*)
│   │   │       │       │       └── tokio feature "sync"
│   │   │       │       │           └── tokio v1.47.1 (*)
│   │   │       │       ├── tokio-util feature "default"
│   │   │       │       │   └── tokio-util v0.7.16 (*)
│   │   │       │       ├── tokio-util feature "io"
│   │   │       │       │   └── tokio-util v0.7.16 (*)
│   │   │       │       └── tracing feature "std"
│   │   │       │           ├── tracing v0.1.41
│   │   │       │           │   ├── tracing-core v0.1.34
│   │   │       │           │   │   └── once_cell feature "default"
│   │   │       │           │   │       ├── once_cell v1.21.3
│   │   │       │           │   │       └── once_cell feature "std"
│   │   │       │           │   │           ├── once_cell v1.21.3
│   │   │       │           │   │           └── once_cell feature "alloc"
│   │   │       │           │   │               ├── once_cell v1.21.3
│   │   │       │           │   │               └── once_cell feature "race"
│   │   │       │           │   │                   └── once_cell v1.21.3
│   │   │       │           │   ├── pin-project-lite feature "default" (*)
│   │   │       │           │   └── tracing-attributes feature "default"
│   │   │       │           │       └── tracing-attributes v0.1.30 (proc-macro)
│   │   │       │           │           ├── proc-macro2 feature "default" (*)
│   │   │       │           │           ├── quote feature "default" (*)
│   │   │       │           │           ├── syn feature "clone-impls" (*)
│   │   │       │           │           ├── syn feature "extra-traits"
│   │   │       │           │           │   └── syn v2.0.106 (*)
│   │   │       │           │           ├── syn feature "full" (*)
│   │   │       │           │           ├── syn feature "parsing" (*)
│   │   │       │           │           ├── syn feature "printing" (*)
│   │   │       │           │           ├── syn feature "proc-macro" (*)
│   │   │       │           │           └── syn feature "visit-mut" (*)
│   │   │       │           └── tracing-core feature "std"
│   │   │       │               ├── tracing-core v0.1.34 (*)
│   │   │       │               └── tracing-core feature "once_cell"
│   │   │       │                   └── tracing-core v0.1.34 (*)
│   │   │       ├── tokio feature "default" (*)
│   │   │       ├── tokio feature "sync" (*)
│   │   │       ├── httparse feature "default"
│   │   │       │   ├── httparse v1.10.1
│   │   │       │   └── httparse feature "std"
│   │   │       │       └── httparse v1.10.1
│   │   │       ├── httpdate feature "default"
│   │   │       │   └── httpdate v1.0.3
│   │   │       ├── smallvec feature "const_generics"
│   │   │       │   └── smallvec v1.15.1
│   │   │       ├── smallvec feature "const_new"
│   │   │       │   ├── smallvec v1.15.1
│   │   │       │   └── smallvec feature "const_generics" (*)
│   │   │       └── smallvec feature "default"
│   │   │           └── smallvec v1.15.1
│   │   ├── tokio feature "default" (*)
│   │   ├── tokio feature "time"
│   │   │   └── tokio v1.47.1 (*)
│   │   ├── hyper-util feature "default"
│   │   │   └── hyper-util v0.1.16
│   │   │       ├── tokio v1.47.1 (*)
│   │   │       ├── bytes feature "default" (*)
│   │   │       ├── futures-core feature "default" (*)
│   │   │       ├── pin-project-lite feature "default" (*)
│   │   │       ├── http feature "default" (*)
│   │   │       ├── http-body feature "default" (*)
│   │   │       ├── tower-service feature "default" (*)
│   │   │       └── hyper feature "default" (*)
│   │   ├── hyper-util feature "server"
│   │   │   ├── hyper-util v0.1.16 (*)
│   │   │   └── hyper feature "server"
│   │   │       └── hyper v1.7.0 (*)
│   │   ├── hyper-util feature "service"
│   │   │   └── hyper-util v0.1.16 (*)
│   │   ├── hyper-util feature "tokio"
│   │   │   ├── hyper-util v0.1.16 (*)
│   │   │   ├── tokio feature "rt"
│   │   │   │   └── tokio v1.47.1 (*)
│   │   │   ├── tokio feature "time" (*)
│   │   │   └── hyper-util feature "tokio" (*)
│   │   ├── matchit feature "default"
│   │   │   └── matchit v0.7.3
│   │   ├── memchr feature "default"
│   │   │   ├── memchr v2.7.5
│   │   │   └── memchr feature "std"
│   │   │       ├── memchr v2.7.5
│   │   │       └── memchr feature "alloc"
│   │   │           └── memchr v2.7.5
│   │   ├── percent-encoding feature "default"
│   │   │   ├── percent-encoding v2.3.2
│   │   │   └── percent-encoding feature "std"
│   │   │       ├── percent-encoding v2.3.2
│   │   │       └── percent-encoding feature "alloc"
│   │   │           └── percent-encoding v2.3.2
│   │   ├── serde feature "default"
│   │   │   ├── serde v1.0.221
│   │   │   │   ├── serde_core feature "result"
│   │   │   │   │   └── serde_core v1.0.221
│   │   │   │   └── serde_derive feature "default"
│   │   │   │       └── serde_derive v1.0.221 (proc-macro)
│   │   │   │           ├── proc-macro2 feature "proc-macro" (*)
│   │   │   │           ├── quote feature "proc-macro" (*)
│   │   │   │           ├── syn feature "clone-impls" (*)
│   │   │   │           ├── syn feature "derive" (*)
│   │   │   │           ├── syn feature "parsing" (*)
│   │   │   │           ├── syn feature "printing" (*)
│   │   │   │           └── syn feature "proc-macro" (*)
│   │   │   └── serde feature "std"
│   │   │       ├── serde v1.0.221 (*)
│   │   │       └── serde_core feature "std"
│   │   │           └── serde_core v1.0.221
│   │   ├── serde_json feature "default"
│   │   │   ├── serde_json v1.0.144
│   │   │   │   ├── memchr v2.7.5
│   │   │   │   ├── serde_core v1.0.221
│   │   │   │   ├── itoa feature "default" (*)
│   │   │   │   └── ryu feature "default"
│   │   │   │       └── ryu v1.0.20
... (truncated)
```

</details>

### ron-policy

<details><summary>Reverse tree (-i ron-policy -e features)</summary>

```text
```

</details>

<details><summary>Forward tree (-p ron-policy -e features)</summary>

```text
ron-policy v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/ron-policy)
├── serde feature "default"
│   ├── serde v1.0.221
│   │   ├── serde_core feature "result"
│   │   │   └── serde_core v1.0.221
│   │   └── serde_derive feature "default"
│   │       └── serde_derive v1.0.221 (proc-macro)
│   │           ├── proc-macro2 feature "proc-macro"
│   │           │   └── proc-macro2 v1.0.101
│   │           │       └── unicode-ident feature "default"
│   │           │           └── unicode-ident v1.0.19
│   │           ├── quote feature "proc-macro"
│   │           │   ├── quote v1.0.40
│   │           │   │   └── proc-macro2 v1.0.101 (*)
│   │           │   └── proc-macro2 feature "proc-macro" (*)
│   │           ├── syn feature "clone-impls"
│   │           │   └── syn v2.0.106
│   │           │       ├── proc-macro2 v1.0.101 (*)
│   │           │       ├── quote v1.0.40 (*)
│   │           │       └── unicode-ident feature "default" (*)
│   │           ├── syn feature "derive"
│   │           │   └── syn v2.0.106 (*)
│   │           ├── syn feature "parsing"
│   │           │   └── syn v2.0.106 (*)
│   │           ├── syn feature "printing"
│   │           │   └── syn v2.0.106 (*)
│   │           └── syn feature "proc-macro"
│   │               ├── syn v2.0.106 (*)
│   │               ├── proc-macro2 feature "proc-macro" (*)
│   │               └── quote feature "proc-macro" (*)
│   └── serde feature "std"
│       ├── serde v1.0.221 (*)
│       └── serde_core feature "std"
│           └── serde_core v1.0.221
├── serde feature "derive"
│   ├── serde v1.0.221 (*)
│   └── serde feature "serde_derive"
│       └── serde v1.0.221 (*)
└── serde_json feature "default"
    ├── serde_json v1.0.144
    │   ├── memchr v2.7.5
    │   ├── serde_core v1.0.221
    │   ├── itoa feature "default"
    │   │   └── itoa v1.0.15
    │   └── ryu feature "default"
    │       └── ryu v1.0.20
    └── serde_json feature "std"
        ├── serde_json v1.0.144 (*)
        ├── serde_core feature "std" (*)
        └── memchr feature "std"
            ├── memchr v2.7.5
            └── memchr feature "alloc"
                └── memchr v2.7.5
```

</details>

### svc-edge

<details><summary>Reverse tree (-i svc-edge -e features)</summary>

```text
```

</details>

<details><summary>Forward tree (-p svc-edge -e features)</summary>

```text
svc-edge v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/svc-edge)
├── axum feature "http1"
│   ├── axum v0.7.9
│   │   ├── async-trait feature "default"
│   │   │   └── async-trait v0.1.89 (proc-macro)
│   │   │       ├── proc-macro2 feature "default"
│   │   │       │   ├── proc-macro2 v1.0.101
│   │   │       │   │   └── unicode-ident feature "default"
│   │   │       │   │       └── unicode-ident v1.0.19
│   │   │       │   └── proc-macro2 feature "proc-macro"
│   │   │       │       └── proc-macro2 v1.0.101 (*)
│   │   │       ├── quote feature "default"
│   │   │       │   ├── quote v1.0.40
│   │   │       │   │   └── proc-macro2 v1.0.101 (*)
│   │   │       │   └── quote feature "proc-macro"
│   │   │       │       ├── quote v1.0.40 (*)
│   │   │       │       └── proc-macro2 feature "proc-macro" (*)
│   │   │       ├── syn feature "clone-impls"
│   │   │       │   └── syn v2.0.106
│   │   │       │       ├── proc-macro2 v1.0.101 (*)
│   │   │       │       ├── quote v1.0.40 (*)
│   │   │       │       └── unicode-ident feature "default" (*)
│   │   │       ├── syn feature "full"
│   │   │       │   └── syn v2.0.106 (*)
│   │   │       ├── syn feature "parsing"
│   │   │       │   └── syn v2.0.106 (*)
│   │   │       ├── syn feature "printing"
│   │   │       │   └── syn v2.0.106 (*)
│   │   │       ├── syn feature "proc-macro"
│   │   │       │   ├── syn v2.0.106 (*)
│   │   │       │   ├── proc-macro2 feature "proc-macro" (*)
│   │   │       │   └── quote feature "proc-macro" (*)
│   │   │       └── syn feature "visit-mut"
│   │   │           └── syn v2.0.106 (*)
│   │   ├── axum-core feature "default"
│   │   │   └── axum-core v0.4.5
│   │   │       ├── async-trait feature "default" (*)
│   │   │       ├── bytes feature "default"
│   │   │       │   ├── bytes v1.10.1
│   │   │       │   └── bytes feature "std"
│   │   │       │       └── bytes v1.10.1
│   │   │       ├── futures-util feature "alloc"
│   │   │       │   ├── futures-util v0.3.31
│   │   │       │   │   ├── futures-core v0.3.31
│   │   │       │   │   ├── futures-task v0.3.31
│   │   │       │   │   ├── pin-project-lite feature "default"
│   │   │       │   │   │   └── pin-project-lite v0.2.16
│   │   │       │   │   └── pin-utils feature "default"
│   │   │       │   │       └── pin-utils v0.1.0
│   │   │       │   ├── futures-core feature "alloc"
│   │   │       │   │   └── futures-core v0.3.31
│   │   │       │   └── futures-task feature "alloc"
│   │   │       │       └── futures-task v0.3.31
│   │   │       ├── pin-project-lite feature "default" (*)
│   │   │       ├── http feature "default"
│   │   │       │   ├── http v1.3.1
│   │   │       │   │   ├── bytes feature "default" (*)
│   │   │       │   │   ├── fnv feature "default"
│   │   │       │   │   │   ├── fnv v1.0.7
│   │   │       │   │   │   └── fnv feature "std"
│   │   │       │   │   │       └── fnv v1.0.7
│   │   │       │   │   └── itoa feature "default"
│   │   │       │   │       └── itoa v1.0.15
│   │   │       │   └── http feature "std"
│   │   │       │       └── http v1.3.1 (*)
│   │   │       ├── http-body feature "default"
│   │   │       │   └── http-body v1.0.1
│   │   │       │       ├── bytes feature "default" (*)
│   │   │       │       └── http feature "default" (*)
│   │   │       ├── http-body-util feature "default"
│   │   │       │   └── http-body-util v0.1.3
│   │   │       │       ├── futures-core v0.3.31
│   │   │       │       ├── bytes feature "default" (*)
│   │   │       │       ├── pin-project-lite feature "default" (*)
│   │   │       │       ├── http feature "default" (*)
│   │   │       │       └── http-body feature "default" (*)
│   │   │       ├── mime feature "default"
│   │   │       │   └── mime v0.3.17
│   │   │       ├── rustversion feature "default"
│   │   │       │   └── rustversion v1.0.22 (proc-macro)
│   │   │       ├── sync_wrapper feature "default"
│   │   │       │   └── sync_wrapper v1.0.2
│   │   │       ├── tower-layer feature "default"
│   │   │       │   └── tower-layer v0.3.3
│   │   │       └── tower-service feature "default"
│   │   │           └── tower-service v0.3.3
│   │   ├── bytes feature "default" (*)
│   │   ├── futures-util feature "alloc" (*)
│   │   ├── pin-project-lite feature "default" (*)
│   │   ├── http feature "default" (*)
│   │   ├── itoa feature "default" (*)
│   │   ├── http-body feature "default" (*)
│   │   ├── http-body-util feature "default" (*)
│   │   ├── mime feature "default" (*)
│   │   ├── rustversion feature "default" (*)
│   │   ├── sync_wrapper feature "default" (*)
│   │   ├── tower-layer feature "default" (*)
│   │   ├── tower-service feature "default" (*)
│   │   ├── hyper feature "default"
│   │   │   └── hyper v1.7.0
│   │   │       ├── bytes feature "default" (*)
│   │   │       ├── futures-core feature "default"
│   │   │       │   ├── futures-core v0.3.31
│   │   │       │   └── futures-core feature "std"
│   │   │       │       ├── futures-core v0.3.31
│   │   │       │       └── futures-core feature "alloc" (*)
│   │   │       ├── pin-project-lite feature "default" (*)
│   │   │       ├── pin-utils feature "default" (*)
│   │   │       ├── http feature "default" (*)
│   │   │       ├── itoa feature "default" (*)
│   │   │       ├── http-body feature "default" (*)
│   │   │       ├── atomic-waker feature "default"
│   │   │       │   └── atomic-waker v1.1.2
│   │   │       ├── futures-channel feature "default"
│   │   │       │   ├── futures-channel v0.3.31
│   │   │       │   │   └── futures-core v0.3.31
│   │   │       │   └── futures-channel feature "std"
│   │   │       │       ├── futures-channel v0.3.31 (*)
│   │   │       │       ├── futures-core feature "std" (*)
│   │   │       │       └── futures-channel feature "alloc"
│   │   │       │           ├── futures-channel v0.3.31 (*)
│   │   │       │           └── futures-core feature "alloc" (*)
│   │   │       ├── h2 feature "default"
│   │   │       │   └── h2 v0.4.12
│   │   │       │       ├── futures-core v0.3.31
│   │   │       │       ├── futures-sink v0.3.31
│   │   │       │       ├── bytes feature "default" (*)
│   │   │       │       ├── http feature "default" (*)
│   │   │       │       ├── fnv feature "default" (*)
│   │   │       │       ├── atomic-waker feature "default" (*)
│   │   │       │       ├── indexmap feature "default"
│   │   │       │       │   ├── indexmap v2.11.1
│   │   │       │       │   │   ├── equivalent v1.0.2
│   │   │       │       │   │   └── hashbrown v0.15.5
│   │   │       │       │   └── indexmap feature "std"
│   │   │       │       │       └── indexmap v2.11.1 (*)
│   │   │       │       ├── indexmap feature "std" (*)
│   │   │       │       ├── slab feature "default"
│   │   │       │       │   ├── slab v0.4.11
│   │   │       │       │   └── slab feature "std"
│   │   │       │       │       └── slab v0.4.11
│   │   │       │       ├── tokio feature "default"
│   │   │       │       │   └── tokio v1.47.1
│   │   │       │       │       ├── mio v1.0.4
│   │   │       │       │       │   └── libc feature "default"
│   │   │       │       │       │       ├── libc v0.2.175
│   │   │       │       │       │       └── libc feature "std"
│   │   │       │       │       │           └── libc v0.2.175
│   │   │       │       │       ├── bytes feature "default" (*)
│   │   │       │       │       ├── pin-project-lite feature "default" (*)
│   │   │       │       │       ├── libc feature "default" (*)
│   │   │       │       │       ├── signal-hook-registry feature "default"
│   │   │       │       │       │   └── signal-hook-registry v1.4.6
│   │   │       │       │       │       └── libc feature "default" (*)
│   │   │       │       │       ├── socket2 feature "all"
│   │   │       │       │       │   └── socket2 v0.6.0
│   │   │       │       │       │       └── libc feature "default" (*)
│   │   │       │       │       ├── socket2 feature "default"
│   │   │       │       │       │   └── socket2 v0.6.0 (*)
│   │   │       │       │       └── tokio-macros feature "default"
│   │   │       │       │           └── tokio-macros v2.5.0 (proc-macro)
│   │   │       │       │               ├── proc-macro2 feature "default" (*)
│   │   │       │       │               ├── quote feature "default" (*)
│   │   │       │       │               ├── syn feature "default"
│   │   │       │       │               │   ├── syn v2.0.106 (*)
│   │   │       │       │               │   ├── syn feature "clone-impls" (*)
│   │   │       │       │               │   ├── syn feature "derive"
│   │   │       │       │               │   │   └── syn v2.0.106 (*)
│   │   │       │       │               │   ├── syn feature "parsing" (*)
│   │   │       │       │               │   ├── syn feature "printing" (*)
│   │   │       │       │               │   └── syn feature "proc-macro" (*)
│   │   │       │       │               └── syn feature "full" (*)
│   │   │       │       ├── tokio feature "io-util"
│   │   │       │       │   ├── tokio v1.47.1 (*)
│   │   │       │       │   └── tokio feature "bytes"
│   │   │       │       │       └── tokio v1.47.1 (*)
│   │   │       │       ├── tokio-util feature "codec"
│   │   │       │       │   └── tokio-util v0.7.16
│   │   │       │       │       ├── bytes feature "default" (*)
│   │   │       │       │       ├── futures-core feature "default" (*)
│   │   │       │       │       ├── pin-project-lite feature "default" (*)
│   │   │       │       │       ├── futures-sink feature "default"
│   │   │       │       │       │   ├── futures-sink v0.3.31
│   │   │       │       │       │   └── futures-sink feature "std"
│   │   │       │       │       │       ├── futures-sink v0.3.31
│   │   │       │       │       │       └── futures-sink feature "alloc"
│   │   │       │       │       │           └── futures-sink v0.3.31
│   │   │       │       │       ├── tokio feature "default" (*)
│   │   │       │       │       └── tokio feature "sync"
│   │   │       │       │           └── tokio v1.47.1 (*)
│   │   │       │       ├── tokio-util feature "default"
│   │   │       │       │   └── tokio-util v0.7.16 (*)
│   │   │       │       ├── tokio-util feature "io"
│   │   │       │       │   └── tokio-util v0.7.16 (*)
│   │   │       │       └── tracing feature "std"
│   │   │       │           ├── tracing v0.1.41
│   │   │       │           │   ├── tracing-core v0.1.34
│   │   │       │           │   │   └── once_cell feature "default"
│   │   │       │           │   │       ├── once_cell v1.21.3
│   │   │       │           │   │       └── once_cell feature "std"
│   │   │       │           │   │           ├── once_cell v1.21.3
│   │   │       │           │   │           └── once_cell feature "alloc"
│   │   │       │           │   │               ├── once_cell v1.21.3
│   │   │       │           │   │               └── once_cell feature "race"
│   │   │       │           │   │                   └── once_cell v1.21.3
│   │   │       │           │   ├── pin-project-lite feature "default" (*)
│   │   │       │           │   └── tracing-attributes feature "default"
│   │   │       │           │       └── tracing-attributes v0.1.30 (proc-macro)
│   │   │       │           │           ├── proc-macro2 feature "default" (*)
│   │   │       │           │           ├── quote feature "default" (*)
│   │   │       │           │           ├── syn feature "clone-impls" (*)
│   │   │       │           │           ├── syn feature "extra-traits"
│   │   │       │           │           │   └── syn v2.0.106 (*)
│   │   │       │           │           ├── syn feature "full" (*)
│   │   │       │           │           ├── syn feature "parsing" (*)
│   │   │       │           │           ├── syn feature "printing" (*)
│   │   │       │           │           ├── syn feature "proc-macro" (*)
│   │   │       │           │           └── syn feature "visit-mut" (*)
│   │   │       │           └── tracing-core feature "std"
│   │   │       │               ├── tracing-core v0.1.34 (*)
│   │   │       │               └── tracing-core feature "once_cell"
│   │   │       │                   └── tracing-core v0.1.34 (*)
│   │   │       ├── tokio feature "default" (*)
│   │   │       ├── tokio feature "sync" (*)
│   │   │       ├── httparse feature "default"
│   │   │       │   ├── httparse v1.10.1
│   │   │       │   └── httparse feature "std"
│   │   │       │       └── httparse v1.10.1
│   │   │       ├── httpdate feature "default"
│   │   │       │   └── httpdate v1.0.3
│   │   │       ├── smallvec feature "const_generics"
│   │   │       │   └── smallvec v1.15.1
│   │   │       ├── smallvec feature "const_new"
│   │   │       │   ├── smallvec v1.15.1
│   │   │       │   └── smallvec feature "const_generics" (*)
│   │   │       └── smallvec feature "default"
│   │   │           └── smallvec v1.15.1
│   │   ├── tokio feature "default" (*)
│   │   ├── tokio feature "time"
│   │   │   └── tokio v1.47.1 (*)
│   │   ├── hyper-util feature "default"
│   │   │   └── hyper-util v0.1.16
│   │   │       ├── tokio v1.47.1 (*)
│   │   │       ├── bytes feature "default" (*)
│   │   │       ├── futures-core feature "default" (*)
│   │   │       ├── pin-project-lite feature "default" (*)
│   │   │       ├── http feature "default" (*)
│   │   │       ├── http-body feature "default" (*)
│   │   │       ├── tower-service feature "default" (*)
│   │   │       └── hyper feature "default" (*)
│   │   ├── hyper-util feature "server"
│   │   │   ├── hyper-util v0.1.16 (*)
│   │   │   └── hyper feature "server"
│   │   │       └── hyper v1.7.0 (*)
│   │   ├── hyper-util feature "service"
│   │   │   └── hyper-util v0.1.16 (*)
│   │   ├── hyper-util feature "tokio"
│   │   │   ├── hyper-util v0.1.16 (*)
│   │   │   ├── tokio feature "rt"
│   │   │   │   └── tokio v1.47.1 (*)
│   │   │   ├── tokio feature "time" (*)
│   │   │   └── hyper-util feature "tokio" (*)
│   │   ├── matchit feature "default"
│   │   │   └── matchit v0.7.3
│   │   ├── memchr feature "default"
│   │   │   ├── memchr v2.7.5
│   │   │   └── memchr feature "std"
│   │   │       ├── memchr v2.7.5
│   │   │       └── memchr feature "alloc"
│   │   │           └── memchr v2.7.5
│   │   ├── percent-encoding feature "default"
│   │   │   ├── percent-encoding v2.3.2
│   │   │   └── percent-encoding feature "std"
│   │   │       ├── percent-encoding v2.3.2
│   │   │       └── percent-encoding feature "alloc"
│   │   │           └── percent-encoding v2.3.2
│   │   ├── serde feature "default"
│   │   │   ├── serde v1.0.221
│   │   │   │   ├── serde_core feature "result"
│   │   │   │   │   └── serde_core v1.0.221
│   │   │   │   └── serde_derive feature "default"
│   │   │   │       └── serde_derive v1.0.221 (proc-macro)
│   │   │   │           ├── proc-macro2 feature "proc-macro" (*)
│   │   │   │           ├── quote feature "proc-macro" (*)
│   │   │   │           ├── syn feature "clone-impls" (*)
│   │   │   │           ├── syn feature "derive" (*)
│   │   │   │           ├── syn feature "parsing" (*)
│   │   │   │           ├── syn feature "printing" (*)
│   │   │   │           └── syn feature "proc-macro" (*)
│   │   │   └── serde feature "std"
│   │   │       ├── serde v1.0.221 (*)
│   │   │       └── serde_core feature "std"
│   │   │           └── serde_core v1.0.221
│   │   ├── serde_json feature "default"
│   │   │   ├── serde_json v1.0.144
│   │   │   │   ├── memchr v2.7.5
│   │   │   │   ├── serde_core v1.0.221
│   │   │   │   ├── itoa feature "default" (*)
│   │   │   │   └── ryu feature "default"
│   │   │   │       └── ryu v1.0.20
... (truncated)
```

</details>

### svc-sandbox

<details><summary>Reverse tree (-i svc-sandbox -e features)</summary>

```text
```

</details>

<details><summary>Forward tree (-p svc-sandbox -e features)</summary>

```text
svc-sandbox v0.1.0 (/Users/mymac/Desktop/RustyOnions/crates/svc-sandbox)
├── anyhow feature "default"
│   ├── anyhow v1.0.99
│   └── anyhow feature "std"
│       └── anyhow v1.0.99
├── axum feature "http1"
│   ├── axum v0.7.9
│   │   ├── async-trait feature "default"
│   │   │   └── async-trait v0.1.89 (proc-macro)
│   │   │       ├── proc-macro2 feature "default"
│   │   │       │   ├── proc-macro2 v1.0.101
│   │   │       │   │   └── unicode-ident feature "default"
│   │   │       │   │       └── unicode-ident v1.0.19
│   │   │       │   └── proc-macro2 feature "proc-macro"
│   │   │       │       └── proc-macro2 v1.0.101 (*)
│   │   │       ├── quote feature "default"
│   │   │       │   ├── quote v1.0.40
│   │   │       │   │   └── proc-macro2 v1.0.101 (*)
│   │   │       │   └── quote feature "proc-macro"
│   │   │       │       ├── quote v1.0.40 (*)
│   │   │       │       └── proc-macro2 feature "proc-macro" (*)
│   │   │       ├── syn feature "clone-impls"
│   │   │       │   └── syn v2.0.106
│   │   │       │       ├── proc-macro2 v1.0.101 (*)
│   │   │       │       ├── quote v1.0.40 (*)
│   │   │       │       └── unicode-ident feature "default" (*)
│   │   │       ├── syn feature "full"
│   │   │       │   └── syn v2.0.106 (*)
│   │   │       ├── syn feature "parsing"
│   │   │       │   └── syn v2.0.106 (*)
│   │   │       ├── syn feature "printing"
│   │   │       │   └── syn v2.0.106 (*)
│   │   │       ├── syn feature "proc-macro"
│   │   │       │   ├── syn v2.0.106 (*)
│   │   │       │   ├── proc-macro2 feature "proc-macro" (*)
│   │   │       │   └── quote feature "proc-macro" (*)
│   │   │       └── syn feature "visit-mut"
│   │   │           └── syn v2.0.106 (*)
│   │   ├── axum-core feature "default"
│   │   │   └── axum-core v0.4.5
│   │   │       ├── async-trait feature "default" (*)
│   │   │       ├── bytes feature "default"
│   │   │       │   ├── bytes v1.10.1
│   │   │       │   └── bytes feature "std"
│   │   │       │       └── bytes v1.10.1
│   │   │       ├── futures-util feature "alloc"
│   │   │       │   ├── futures-util v0.3.31
│   │   │       │   │   ├── futures-core v0.3.31
│   │   │       │   │   ├── futures-macro v0.3.31 (proc-macro)
│   │   │       │   │   │   ├── proc-macro2 feature "default" (*)
│   │   │       │   │   │   ├── quote feature "default" (*)
│   │   │       │   │   │   ├── syn feature "default"
│   │   │       │   │   │   │   ├── syn v2.0.106 (*)
│   │   │       │   │   │   │   ├── syn feature "clone-impls" (*)
│   │   │       │   │   │   │   ├── syn feature "derive"
│   │   │       │   │   │   │   │   └── syn v2.0.106 (*)
│   │   │       │   │   │   │   ├── syn feature "parsing" (*)
│   │   │       │   │   │   │   ├── syn feature "printing" (*)
│   │   │       │   │   │   │   └── syn feature "proc-macro" (*)
│   │   │       │   │   │   └── syn feature "full" (*)
│   │   │       │   │   ├── futures-task v0.3.31
│   │   │       │   │   ├── pin-project-lite feature "default"
│   │   │       │   │   │   └── pin-project-lite v0.2.16
│   │   │       │   │   ├── pin-utils feature "default"
│   │   │       │   │   │   └── pin-utils v0.1.0
│   │   │       │   │   └── slab feature "default"
│   │   │       │   │       ├── slab v0.4.11
│   │   │       │   │       └── slab feature "std"
│   │   │       │   │           └── slab v0.4.11
│   │   │       │   ├── futures-core feature "alloc"
│   │   │       │   │   └── futures-core v0.3.31
│   │   │       │   └── futures-task feature "alloc"
│   │   │       │       └── futures-task v0.3.31
│   │   │       ├── pin-project-lite feature "default" (*)
│   │   │       ├── http feature "default"
│   │   │       │   ├── http v1.3.1
│   │   │       │   │   ├── bytes feature "default" (*)
│   │   │       │   │   ├── fnv feature "default"
│   │   │       │   │   │   ├── fnv v1.0.7
│   │   │       │   │   │   └── fnv feature "std"
│   │   │       │   │   │       └── fnv v1.0.7
│   │   │       │   │   └── itoa feature "default"
│   │   │       │   │       └── itoa v1.0.15
│   │   │       │   └── http feature "std"
│   │   │       │       └── http v1.3.1 (*)
│   │   │       ├── http-body feature "default"
│   │   │       │   └── http-body v1.0.1
│   │   │       │       ├── bytes feature "default" (*)
│   │   │       │       └── http feature "default" (*)
│   │   │       ├── http-body-util feature "default"
│   │   │       │   └── http-body-util v0.1.3
│   │   │       │       ├── futures-core v0.3.31
│   │   │       │       ├── bytes feature "default" (*)
│   │   │       │       ├── pin-project-lite feature "default" (*)
│   │   │       │       ├── http feature "default" (*)
│   │   │       │       └── http-body feature "default" (*)
│   │   │       ├── mime feature "default"
│   │   │       │   └── mime v0.3.17
│   │   │       ├── rustversion feature "default"
│   │   │       │   └── rustversion v1.0.22 (proc-macro)
│   │   │       ├── sync_wrapper feature "default"
│   │   │       │   └── sync_wrapper v1.0.2
│   │   │       ├── tower-layer feature "default"
│   │   │       │   └── tower-layer v0.3.3
│   │   │       └── tower-service feature "default"
│   │   │           └── tower-service v0.3.3
│   │   ├── bytes feature "default" (*)
│   │   ├── futures-util feature "alloc" (*)
│   │   ├── pin-project-lite feature "default" (*)
│   │   ├── http feature "default" (*)
│   │   ├── itoa feature "default" (*)
│   │   ├── http-body feature "default" (*)
│   │   ├── http-body-util feature "default" (*)
│   │   ├── mime feature "default" (*)
│   │   ├── rustversion feature "default" (*)
│   │   ├── sync_wrapper feature "default" (*)
│   │   ├── tower-layer feature "default" (*)
│   │   ├── tower-service feature "default" (*)
│   │   ├── hyper feature "default"
│   │   │   └── hyper v1.7.0
│   │   │       ├── bytes feature "default" (*)
│   │   │       ├── futures-core feature "default"
│   │   │       │   ├── futures-core v0.3.31
│   │   │       │   └── futures-core feature "std"
│   │   │       │       ├── futures-core v0.3.31
│   │   │       │       └── futures-core feature "alloc" (*)
│   │   │       ├── pin-project-lite feature "default" (*)
│   │   │       ├── pin-utils feature "default" (*)
│   │   │       ├── http feature "default" (*)
│   │   │       ├── itoa feature "default" (*)
│   │   │       ├── http-body feature "default" (*)
│   │   │       ├── atomic-waker feature "default"
│   │   │       │   └── atomic-waker v1.1.2
│   │   │       ├── futures-channel feature "default"
│   │   │       │   ├── futures-channel v0.3.31
│   │   │       │   │   └── futures-core v0.3.31
│   │   │       │   └── futures-channel feature "std"
│   │   │       │       ├── futures-channel v0.3.31 (*)
│   │   │       │       ├── futures-core feature "std" (*)
│   │   │       │       └── futures-channel feature "alloc"
│   │   │       │           ├── futures-channel v0.3.31 (*)
│   │   │       │           └── futures-core feature "alloc" (*)
│   │   │       ├── h2 feature "default"
│   │   │       │   └── h2 v0.4.12
│   │   │       │       ├── futures-core v0.3.31
│   │   │       │       ├── futures-sink v0.3.31
│   │   │       │       ├── bytes feature "default" (*)
│   │   │       │       ├── slab feature "default" (*)
│   │   │       │       ├── http feature "default" (*)
│   │   │       │       ├── fnv feature "default" (*)
│   │   │       │       ├── atomic-waker feature "default" (*)
│   │   │       │       ├── indexmap feature "default"
│   │   │       │       │   ├── indexmap v2.11.1
│   │   │       │       │   │   ├── equivalent v1.0.2
│   │   │       │       │   │   └── hashbrown v0.15.5
│   │   │       │       │   └── indexmap feature "std"
│   │   │       │       │       └── indexmap v2.11.1 (*)
│   │   │       │       ├── indexmap feature "std" (*)
│   │   │       │       ├── tokio feature "default"
│   │   │       │       │   └── tokio v1.47.1
│   │   │       │       │       ├── mio v1.0.4
│   │   │       │       │       │   └── libc feature "default"
│   │   │       │       │       │       ├── libc v0.2.175
│   │   │       │       │       │       └── libc feature "std"
│   │   │       │       │       │           └── libc v0.2.175
│   │   │       │       │       ├── bytes feature "default" (*)
│   │   │       │       │       ├── pin-project-lite feature "default" (*)
│   │   │       │       │       ├── libc feature "default" (*)
│   │   │       │       │       ├── signal-hook-registry feature "default"
│   │   │       │       │       │   └── signal-hook-registry v1.4.6
│   │   │       │       │       │       └── libc feature "default" (*)
│   │   │       │       │       ├── socket2 feature "all"
│   │   │       │       │       │   └── socket2 v0.6.0
│   │   │       │       │       │       └── libc feature "default" (*)
│   │   │       │       │       ├── socket2 feature "default"
│   │   │       │       │       │   └── socket2 v0.6.0 (*)
│   │   │       │       │       └── tokio-macros feature "default"
│   │   │       │       │           └── tokio-macros v2.5.0 (proc-macro)
│   │   │       │       │               ├── proc-macro2 feature "default" (*)
│   │   │       │       │               ├── quote feature "default" (*)
│   │   │       │       │               ├── syn feature "default" (*)
│   │   │       │       │               └── syn feature "full" (*)
│   │   │       │       ├── tokio feature "io-util"
│   │   │       │       │   ├── tokio v1.47.1 (*)
│   │   │       │       │   └── tokio feature "bytes"
│   │   │       │       │       └── tokio v1.47.1 (*)
│   │   │       │       ├── tokio-util feature "codec"
│   │   │       │       │   └── tokio-util v0.7.16
│   │   │       │       │       ├── bytes feature "default" (*)
│   │   │       │       │       ├── futures-core feature "default" (*)
│   │   │       │       │       ├── pin-project-lite feature "default" (*)
│   │   │       │       │       ├── futures-sink feature "default"
│   │   │       │       │       │   ├── futures-sink v0.3.31
│   │   │       │       │       │   └── futures-sink feature "std"
│   │   │       │       │       │       ├── futures-sink v0.3.31
│   │   │       │       │       │       └── futures-sink feature "alloc"
│   │   │       │       │       │           └── futures-sink v0.3.31
│   │   │       │       │       ├── tokio feature "default" (*)
│   │   │       │       │       └── tokio feature "sync"
│   │   │       │       │           └── tokio v1.47.1 (*)
│   │   │       │       ├── tokio-util feature "default"
│   │   │       │       │   └── tokio-util v0.7.16 (*)
│   │   │       │       ├── tokio-util feature "io"
│   │   │       │       │   └── tokio-util v0.7.16 (*)
│   │   │       │       └── tracing feature "std"
│   │   │       │           ├── tracing v0.1.41
│   │   │       │           │   ├── tracing-core v0.1.34
│   │   │       │           │   │   └── once_cell feature "default"
│   │   │       │           │   │       ├── once_cell v1.21.3
│   │   │       │           │   │       └── once_cell feature "std"
│   │   │       │           │   │           ├── once_cell v1.21.3
│   │   │       │           │   │           └── once_cell feature "alloc"
│   │   │       │           │   │               ├── once_cell v1.21.3
│   │   │       │           │   │               └── once_cell feature "race"
│   │   │       │           │   │                   └── once_cell v1.21.3
│   │   │       │           │   ├── pin-project-lite feature "default" (*)
│   │   │       │           │   └── tracing-attributes feature "default"
│   │   │       │           │       └── tracing-attributes v0.1.30 (proc-macro)
│   │   │       │           │           ├── proc-macro2 feature "default" (*)
│   │   │       │           │           ├── quote feature "default" (*)
│   │   │       │           │           ├── syn feature "clone-impls" (*)
│   │   │       │           │           ├── syn feature "extra-traits"
│   │   │       │           │           │   └── syn v2.0.106 (*)
│   │   │       │           │           ├── syn feature "full" (*)
│   │   │       │           │           ├── syn feature "parsing" (*)
│   │   │       │           │           ├── syn feature "printing" (*)
│   │   │       │           │           ├── syn feature "proc-macro" (*)
│   │   │       │           │           └── syn feature "visit-mut" (*)
│   │   │       │           └── tracing-core feature "std"
│   │   │       │               ├── tracing-core v0.1.34 (*)
│   │   │       │               └── tracing-core feature "once_cell"
│   │   │       │                   └── tracing-core v0.1.34 (*)
│   │   │       ├── tokio feature "default" (*)
│   │   │       ├── tokio feature "sync" (*)
│   │   │       ├── httparse feature "default"
│   │   │       │   ├── httparse v1.10.1
│   │   │       │   └── httparse feature "std"
│   │   │       │       └── httparse v1.10.1
│   │   │       ├── httpdate feature "default"
│   │   │       │   └── httpdate v1.0.3
│   │   │       ├── smallvec feature "const_generics"
│   │   │       │   └── smallvec v1.15.1
│   │   │       ├── smallvec feature "const_new"
│   │   │       │   ├── smallvec v1.15.1
│   │   │       │   └── smallvec feature "const_generics" (*)
│   │   │       └── smallvec feature "default"
│   │   │           └── smallvec v1.15.1
│   │   ├── tokio feature "default" (*)
│   │   ├── tokio feature "time"
│   │   │   └── tokio v1.47.1 (*)
│   │   ├── hyper-util feature "default"
│   │   │   └── hyper-util v0.1.16
│   │   │       ├── tokio v1.47.1 (*)
│   │   │       ├── bytes feature "default" (*)
│   │   │       ├── futures-core feature "default" (*)
│   │   │       ├── pin-project-lite feature "default" (*)
│   │   │       ├── http feature "default" (*)
│   │   │       ├── http-body feature "default" (*)
│   │   │       ├── tower-service feature "default" (*)
│   │   │       └── hyper feature "default" (*)
│   │   ├── hyper-util feature "server"
│   │   │   ├── hyper-util v0.1.16 (*)
│   │   │   └── hyper feature "server"
│   │   │       └── hyper v1.7.0 (*)
│   │   ├── hyper-util feature "service"
│   │   │   └── hyper-util v0.1.16 (*)
│   │   ├── hyper-util feature "tokio"
│   │   │   ├── hyper-util v0.1.16 (*)
│   │   │   ├── tokio feature "rt"
│   │   │   │   └── tokio v1.47.1 (*)
│   │   │   ├── tokio feature "time" (*)
│   │   │   └── hyper-util feature "tokio" (*)
│   │   ├── matchit feature "default"
│   │   │   └── matchit v0.7.3
│   │   ├── memchr feature "default"
│   │   │   ├── memchr v2.7.5
│   │   │   └── memchr feature "std"
│   │   │       ├── memchr v2.7.5
│   │   │       └── memchr feature "alloc"
│   │   │           └── memchr v2.7.5
│   │   ├── percent-encoding feature "default"
│   │   │   ├── percent-encoding v2.3.2
│   │   │   └── percent-encoding feature "std"
│   │   │       ├── percent-encoding v2.3.2
│   │   │       └── percent-encoding feature "alloc"
│   │   │           └── percent-encoding v2.3.2
│   │   ├── serde feature "default"
│   │   │   ├── serde v1.0.221
│   │   │   │   ├── serde_core feature "result"
│   │   │   │   │   └── serde_core v1.0.221
│   │   │   │   └── serde_derive feature "default"
│   │   │   │       └── serde_derive v1.0.221 (proc-macro)
│   │   │   │           ├── proc-macro2 feature "proc-macro" (*)
│   │   │   │           ├── quote feature "proc-macro" (*)
│   │   │   │           ├── syn feature "clone-impls" (*)
│   │   │   │           ├── syn feature "derive" (*)
│   │   │   │           ├── syn feature "parsing" (*)
│   │   │   │           ├── syn feature "printing" (*)
│   │   │   │           └── syn feature "proc-macro" (*)
│   │   │   └── serde feature "std"
... (truncated)
```

</details>


## 9) Raw Data Files

- `refactor_dump/crates_overview.csv` — per-crate metrics (rdeps, fdeps, churn, loc, instability)
- `refactor_dump/crates_ranked.csv` — ranked crates by rdeps → churn
- `refactor_dump/feature_hotspots.csv` — top features per crate by reverse graph
- `refactor_dump/duplicates.txt` — duplicate dependency versions
- `refactor_dump/metadata.json` — cargo metadata snapshot
- `refactor_dump/cycles_guess.txt` — crude cycle hints
- `refactor_dump/forbidden_edges_guess.txt` — kernel/internal import hints
- `refactor_dump/public_api_count.txt` — public API count (if RUN_API_SCAN=1)
- `refactor_dump/build_time.csv` — per-crate build time/RSS (if RUN_BUILD_TIME=1)
- `refactor_dump/serde_scan.txt`, `refactor_dump/serde_missing_guess.txt` — serde attributes
- `refactor_dump/cargo_deny_summary.txt`, `refactor_dump/asan_summary.txt`, `refactor_dump/tsan_summary.txt` (if enabled)
- `refactor_dump/smells.txt` — quick smells (regex-only)
