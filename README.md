# RustyOnions



![RustyOnions Logo](assets/rustyonionslogo.png)



> **Status — Aug 25, 2025:** 🚧 *Active build phase.*  
> Current focus: stable TCP overlay and Tor transport testing.  
> Future roadmap: URI scheme adoption, manifest standardization, and micronode research.

RustyOnions is an experimental peer-to-peer platform written entirely in Rust.  
Its long-term vision is a **decentralized internet** that is **private, resilient, free of ads or tracking, and fair**.


### 🔑 How It Works

- **Manifest.toml Attribution Files**  
  Every hash address under a TLD includes a `Manifest.toml` with attribution + payout addresses and other neccessary information. When content is accessed, **automatic micro-payments** are distributed to owners, creators, moderators, and service providers.  

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

## 🚫 Zero Ads, Zero Tracking

RustyOnions has a **hard guarantee**:

- ❌ **No tracking**  
- ❌ **No popups**  
- ❌ **No advertising**  

The network is built for **privacy and utility**, not surveillance capitalism.

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
