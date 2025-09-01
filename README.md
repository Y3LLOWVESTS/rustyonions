# RustyOnions



![RustyOnions Logo](assets/rustyonionslogo.png)

## NEW : Microkernel Architecture 
>🚧 *Active build phase.* 

### Core Principles
- **Isolation:** Each service (index, overlay, storage, etc.) runs as its own process.  
- **Bus-first IPC:** All communication happens over `ron-bus` (UDS + MessagePack).  
- **Fault tolerance:** `ron-kernel` supervises services, restarting them if they crash.  
- **Minimal kernel:** The kernel only supervises and health-checks; services do the work.  

### System Diagram
```text
            +-----------+
 client --> |  gateway  |  (HTTP façade)
            +-----------+
                   |
                   v
            +-------------+
            | svc-overlay |  (bundle handler)
            +-------------+
              /        \
             v          v
    +--------------+   +---------------+
    |  svc-index   |   |  svc-storage  |
    | (addr -> dir)|   | (read/write)  |
    +--------------+   +---------------+

               supervised by
               +-----------+
               | ron-kernel|
               +-----------+

 all services communicate over:
               +--------+
               | ron-bus|
               +--------+

```

## Run Kernel + Services

```rust

RON_SVC_INDEX_BIN=target/debug/svc-index \
RON_SVC_OVERLAY_BIN=target/debug/svc-overlay \
RON_SVC_STORAGE_BIN=target/debug/svc-storage \
cargo run -p ron-kernel

```

## Run Gateway

```rust
export RON_INDEX_SOCK=/tmp/ron/svc-index.sock
export RON_OVERLAY_SOCK=/tmp/ron/svc-overlay.sock
export RON_STORAGE_SOCK=/tmp/ron/svc-storage.sock

cargo run -p gateway -- --bind 127.0.0.1:54087 --enforce-payments true
```

### Core Services
- svc-index – Maps content addresses (b3:<hash>.<ext>) → bundle directories.
- svc-storage – Reads/writes actual bundle files from the filesystem.
- svc-overlay – Uses index + storage to fetch bundle files. Middle layer between gateway and the lower services.
- gateway – Public HTTP API. Delegates all work to svc-overlay.

### Support Libraries
- common – Shared utilities (hashing, config, constants).
- accounting – Counters and metrics.
- naming – Address parsing and validation.
- transport – Async network transports (TCP, Tor).
- overlay – Legacy overlay implementation (reference only).

### Tools
- ronctl – CLI tool for svc-index (health, resolve, put-address).
- tldctl – Manifest/TLD control tool (Manifest.toml validation, scaffolding).
- node – Legacy CLI node (serve/put/get in one process).
- ryker – Experimental crate (sandbox for prototypes).


> **Status — Aug 28, 2025:** 🚧 *Active build phase.*  
> Current focus: stable TCP overlay and Tor transport testing.  
> Future roadmap: URI scheme adoption, manifest standardization, and micronode research.

RustyOnions is an experimental peer-to-peer platform written entirely in Rust.  
Its long-term vision is a **decentralized internet** that is **private, resilient, and fair**.


### 🔑 How It Works

- **Manifest.toml Attribution Files**  
  Every hash address under a TLD includes a `Manifest.toml` with attribution + payout addresses and other neccessary information. When content is accessed, **automatic micro-payments** are distributed to owners, creators, moderators, and service providers.  

- **Hashing**
  Every file, photo, video, written work, social media post or comment, etc is hashed by a hashing algorithm, likely BLAKE3, and that hash becomes the identifier or address of that asset. EG (BLAKE3 HASH OF PHOTO).image or (BLAKE3 HASH OF video).video or (BLAKE3 HASH OF POST).post, so on and so forth.


- **Earning Tokens**  
  - Contribute excess bandwidth (all nodes forward more than they consume).  
  - Publish content (creator economy).  
  - Provide compute services (`.ai`, `.gpu`, `.cpu`).  
  - Moderate content (`.mod`).  

- **Spending Tokens**  
  - Use services across TLDs (map, traffic, AI compute, media).  
  - Visit `.web3` sites (a portion goes to site owner + a portion to creators).  

- **Service + Creator Layers**  
  - **Service layer**: moderators, bandwidth, compute.  
  - **Creator layer**: all social media posts or comments, writers, bloggers, musicians, video producers, artists etc.  
  - **Economic flow**: every interaction routes micro-payments to the right parties.  

---

## 🦀 Addressing Scheme

RustyOnions uses a **crab-based URI format**:

```
🦀://<hash>.<tld>
```

- `<hash>` → unique identifier for a bundle  
- `<tld>` → functional namespace (`.passport`, `.web3`, `.music`, `.news`, etc.)  

### 🔹 Examples
- 🦀://a1b2c3d4e5f6g7h8i9j0.passport → identity/session manifest  
- 🦀://deadbeefcafebabef00d1234.music → music/video stream  
- 🦀://feedfeedfeedfeedfeed1234.blog → blog entry  
- 🦀://1234567890abcdef12345678.news → news/article  

Every RustyOnions address **always resolves to a manifest**.

---

## 📦 Manifest.toml

Every address is backed by a `Manifest.toml` that defines metadata and payloads.  
This guarantees attribution, integrity, and optional micro-payment routing.

### Example
```toml
[meta]
tld = "music"
hash = "deadbeef1234567890abcd"
created = "2025-08-25T12:34:56Z"

[payload]
file = "track.av1"
sha256 = "abcd1234..."
size = 8234567

[options]
chunks = true
resolutions = ["480p", "720p", "1080p"]
license = "CC-BY-4.0"
```

- **Manifest first:** clients always fetch this before content.  
- **Payloads:** can be files, chunks, streams, or multi-res variants.  
- **Extensible:** supports signatures, consensus notes, payout metadata.  

---

## 🌐 TLD Vision

RustyOnions introduces **special-purpose TLDs** for clean organization:

- **Data/Mapping:** `.map`, `.traffic`  
- **Web & Identity:** `.web3`, `.passport`  
- **Compute Services:** `.ai`, `.gpu`, `.cpu`  
- **Media:** `.image`, `.video`, `.music`, `.musicvideo`  
- **Creator Economy:** `.creator`, `.mod`, `.journalist`, `.blogger`  
- **Information:** `.news`, `.blog`, `.article`, `.post`, `.comment`  
- **Music Ecosystem:** `.radio`, `.playlist`  
- **Algorithm Transparency:** `.alg`  

---

## Advertising model to guarantee free speech, support token economy, and better user experience
RustyOnions runs privacy-preserving, tag-targeted ads with no cookies, IDs, fingerprinting, or cross-site tracking—and, **crucially for free speech, advertisers cannot target or exclude specific sites under any circumstance;** matching is strictly by public Manifest tags (e.g., dev:rust, privacy), not by publisher. This design insulates creators from advertiser pressure while keeping ads relevant to content. Inventory is limited and tasteful—no pop-ups, interstitials, sticky overlays, or auto-play—and each unit is clearly labeled “Advertisement” with a disclaimer (e.g., “Views expressed do not necessarily reflect the advertiser”). Measurement is aggregate-only. Settlement uses a two-token model: advertisers acquire ROX off-network and burn ROX to mint ROC on the RustyOnions network; ROC is the internal unit used to bid, settle, and pay campaign fees—creating a one-way spend sink that funds the network without surveillance or site-level control.

---

## 🔮 Future Feature: Micronodes

We plan to explore **micronodes** — ultra-lightweight RustyOnions nodes running on Bluetooth-capable hardware.  

- Useful for local mesh swarms, offline handoffs, disaster recovery.  
- Minimal resource footprint.  
- Still enforce the same manifest + attribution model.  

---

## ✅ What Works Today

- **Local overlay via TCP** → `test_tcp.sh` validates PUT/GET round-trip.  
- **Tor smoke test** → `test_tor.sh` monitors bootstrap (bridges supported).  
- **Node over Tor** → partial; hidden-service e2e in progress.  
- **Refactoring** → crate boundaries and interfaces are being cleaned.  

---

## 🚀 Quick Start

### Prerequisites
- Rust (stable toolchain)  
- macOS or Linux  

### Build
```bash
cargo build --workspace
```

---

## 🧪 Test Scripts

### `test_tcp.sh`
Verifies local TCP overlay path.  
```bash
chmod +x testing/test_tcp.sh
./testing/test_tcp.sh
```

### `test_tor.sh`
Bootstraps isolated Tor and monitors progress.  
```bash
chmod +x testing/test_tor.sh
./testing/test_tor.sh
```

Supports `KEEP_TOR=1`, `AUTO_PORTS=1`, `TOR_BRIDGES=...`.

---

## 🦀 Node Usage (WIP)

Overlay:
```bash
RUST_LOG=info cargo run -p node -- serve --transport tcp
```

Experimental Tor transport:
```bash
RUST_LOG=info cargo run -p node -- serve --transport tor
```

---

## 🤝 Contributing

Bug reports, PRs, and testing feedback welcome.  
Focus areas: script robustness, cross-platform behavior, Tor integration.

---

## ⚖️ Legal & Safety Guidelines

- **No illegal content**  
- **No pornography, gore, or nudity**  
- **Respect Tor bandwidth**  
- **Privacy ≠ impunity**  

---

## 📜 License

MIT — see `LICENSE`.

---

## 🙌 Credits

Created by **Stevan White** with assistance from **OpenAI’s ChatGPT** and **xAI’s Grok**.  
Generated code and scripts are adapted for the project’s goals.
