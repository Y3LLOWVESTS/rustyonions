# RustyOnions

> **Status — Aug 18, 2025:** 🚧 *Active build phase.* We’ve just stabilized two test scripts:
> - `testing/test_tcp.sh` — local TCP overlay smoke test
> - `testing/test_tor.sh` — Tor bootstrap smoke test (client-only; bridges supported)
>
> Tor transport is **not fully integrated end-to-end yet**, but we’ve made solid progress on bootstrapping and smoke-testing. This README replaces earlier instructions to reflect the current state.

RustyOnions is an experimental two-plane, peer-to-peer platform written in Rust. The long-term vision is a **decentralized internet** with special-purpose TLDs, a bandwidth/token economy, and privacy-first transport. The near-term focus is **reliable local overlay** (TCP) and **robust Tor transport**.

---

## Project Overview

### Two Core Planes

- **Overlay Plane (public data):** chunk storage/distribution for things like maps, images, and other public assets.
- **Private Message Plane (anonymous transport):** Tor-backed messaging/requests for privacy and metadata resistance.

---

### 🌐 TLD Vision (Long-Term)

RustyOnions will introduce **special-purpose TLDs** to keep the network cleanly organized and economically sustainable. The corresponding identifyer hash + TLD serves as an address space for plug and play content that could be shared across the Rusty Onions ecosystem with attribution. Each TLD serves a **single purpose**:

#### Data & Mapping
- **`.map`** → hosts public map data  
- **`.traffic`** → provides near-live traffic data  

#### Web & Identity
- **`.web3`** → the core decentralized internet namespace  
- **`.passport`** → single sign-on identity layer  

#### Compute Services
- **`.ai`** → nodes offering AI compute services  
- **`.gpu`** → GPU compute providers  
- **`.cpu`** → CPU compute providers  

#### Media Hosting
- **`.image`** → image hosting  
- **`.video`** → video hosting  
- **`.music`** → music hosting  
- **`.musicvideo`** → music videos  

#### Music Ecosystem
- **`.radio`** → radio-style streams (based on `.music` songs)  
- **`.playlist`** → user playlists (based on `.music` assets)  

#### Information Ecosystem
- **`.news`** → registry of news site with payout address in manifest
- **`.blog`** → registry of blog site with payout address in manifest
- **`.article`** → registry of individual articles + manifest.toml showing originating site + author metadata
- **`.post`** → registry of individual post + manifest.toml showing originating site + author metadata
- **`.comment`** → registry of individual comment + manifest.toml showing originating site + author metadata

#### Creator Economy
- **`.creator`** → registry of content creators with payout addresses  
- **`.mod`** → registry of moderators with scores & reputation metadata 
- **`.journalist`** → registry of journalists with payout addresses
- **`.blogger`** → registry of blog writers with payout address

#### Algorithm Transparency
- **`.alg`** → fair and transparent content algorithms to prevent astroturfing and forced amplification  

---

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

## What Works Today

- ✅ **Local overlay via TCP**: basic PUT/GET path validated through `test_tcp.sh`.
- ✅ **Tor smoke test**: start an isolated Tor process, monitor bootstrap via control port, optional obfs4/snowflake bridges.
- 🧪 **Node over Tor**: partial; server/client e2e over hidden service is **in progress**.
- 🔧 **Refactoring**: ongoing; interfaces and crate boundaries are being simplified.

---

## Quick Start

### Prerequisites

- Rust (stable toolchain)
- macOS or Linux
- Homebrew (macOS) or your package manager (Linux)

### Build

```bash
cargo build --workspace
```

---

## Test Scripts

We currently ship **two** maintained scripts under `testing/`:

### 1) `test_tcp.sh` — local TCP overlay smoke test

**What it does:**  
Starts (or pings) the local overlay TCP listener and performs a small PUT/GET round-trip to verify the path.

**Run it:**
```bash
chmod +x testing/test_tcp.sh
./testing/test_tcp.sh
# or explicitly:
bash testing/test_tcp.sh
```

**Expected:**  
- Prints success with a matching hash or content diff = OK.  
- Saves minimal logs (see `.tcp_test_logs/<RUN_ID>/` if the script emits that path).

**If it fails:**  
- Make sure the node binary for local overlay is available in the workspace.
- Rebuild: `cargo build --workspace`
- Check that no other process is occupying the overlay port (see script header for defaults/flags).

---

### 2) `test_tor.sh` — Tor bootstrap smoke test

**What it does:**  
Launches a **temporary, isolated** Tor client with a custom `torrc`, monitors bootstrap progress via the control port, and (by default) cleans up on exit. It supports **obfs4** and **snowflake** bridges.

**Run it (default ports 19050/19051):**
```bash
chmod +x testing/test_tor.sh
./testing/test_tor.sh
# or:
bash testing/test_tor.sh
```

**Common env knobs:**
```bash
# Keep Tor alive after success (so you can curl through it)
KEEP_TOR=1 ./testing/test_tor.sh

# Auto-pick free ports if defaults are busy
AUTO_PORTS=1 ./testing/test_tor.sh

# More verbose Tor logs and longer timeout
TOR_DEBUG=1 TOR_BOOTSTRAP_TIMEOUT=420 ./testing/test_tor.sh

# If control cookie causes issues for your node, disable control auth
TOR_NO_AUTH=1 KEEP_TOR=1 ./testing/test_tor.sh
```

**Bridges (recommended if you stall at 50%):**
```bash
# Install helpers:
#   obfs4:   brew install obfs4proxy
#   snowflake: brew install snowflake   # provides 'snowflake-client' on macOS

# Inline bridges (replace with your real lines)
TOR_BRIDGES_INLINE=$'Bridge obfs4 1.2.3.4:443 <FP> cert=AAAA... iat-mode=0
Bridge obfs4 ...' KEEP_TOR=1 TOR_DEBUG=1 ./testing/test_tor.sh

# File-based bridges
TOR_BRIDGES=bridges.txt KEEP_TOR=1 ./testing/test_tor.sh
```

**Verify Tor is proxying traffic (when `KEEP_TOR=1`):**
```bash
curl --socks5-hostname 127.0.0.1:19050 https://check.torproject.org/api/ip
# Expect: {"IsTor":true, ...}
```

**Troubleshooting:**
- **Port busy:** `AUTO_PORTS=1` or set `SOCKS_PORT=... CTRL_PORT=...`.
- **Stalls ~50% (“Loading relay descriptors”):** use bridges; try `TOR_DEBUG=1`.
- **“Connection refused” from node:** ensure Tor is still running (`KEEP_TOR=1`) and your node’s Tor settings match the script’s ports/auth mode.

---

## Node (Work-In-Progress) — Minimal Usage

> This is subject to change as we finish the Tor transport integration.

Start a node locally (TCP overlay):
```bash
RUST_LOG=info cargo run -p node -- --config config.sample.toml serve --transport tcp
```

Attempt Tor transport (experimental):
```bash
# Ensure test_tor.sh is running with KEEP_TOR=1 and ports that match your node config.
RUST_LOG=info cargo run -p node -- --config config.sample.toml serve --transport tor
```

---

## Contributing

We’re actively refactoring. Expect churn. Bug reports, small PRs, and script fixes are welcome—especially around:
- Robustness of `testing/test_tcp.sh` and `testing/test_tor.sh`
- Cross-platform behavior (Linux/macOS)
- Clean interfaces between crates

---

## Legal & Safety Guidelines

- **No illegal content.** Public overlay is for safe/open data.  
- **Respect Tor bandwidth.** Contribute back when feasible (relay mode) and use bridges responsibly.  
- **Privacy ≠ impunity.** Don’t use RustyOnions for harassment, intrusion, or anything unlawful.
- **Pornography is prohibited** 

---

## License

MIT — see `LICENSE`.

---

## Credits

Created by **Stevan White** with assistance from **OpenAI’s ChatGPT** and **xAI’s Grok**.  
Generated code and scripts are reviewed and adapted for the project’s goals.

*(This README updates and replaces previous instructions.)*
