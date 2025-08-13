# RustyOnions

> **Status:** 🚧 *Active build phase.* Expect rapid changes, breaking commits, and frequent refactors. Contributions and PR reviews welcome.  
> See the roadmap: [TODO.md](TODO.md)
> Highly ambitious!

RustyOnions is an experimental two-plane, peer-to-peer Web3 infrastructure platform built in Rust.  
Its core mission: **decentralize data hosting, bandwidth sharing, and identity across a privacy-focused network**.

---

## Overview

RustyOnions began as a way to serve secure map and route data peer-to-peer for a taxi app, eliminating expensive centralized servers.  
It has since evolved into a **general-purpose decentralized internet** with **its own TLDs, economy, and identity system**.

### Two Core Planes

- **Overlay Plane** – Public data distribution (e.g., map chunks, images, traffic updates) stored redundantly among participating nodes.
- **Private Message Plane** – End-to-end encrypted messages sent via Tor (Arti Rust Tor implementation) for anonymity and security.

---

## Vision — The Web3 TLD Network

RustyOnions introduces **special-purpose decentralized TLDs**:

- **`.map`** — Map information chunks
- **`.route`** — Traffic information  
- **`.sso`** — Universal profile & basic user info (single sign-on for the entire network)
- **`.image`** — Decentralized image hosting with hash-based verification & ownership metadata
- **`.video`** — Decentralized video hosting with hash-based verification & ownership metadata
- **`.web3`** — General-purpose websites, similar to clearnet domains

Ownership metadata includes **SHA/SHA256 or MD5 checksums** plus a **crypto address** for attribution and payments.

---

## Token Economy

A future Solana-based (or similar) token will be minted to power the ecosystem:

- **Earn tokens** — By providing bandwidth, hosting chunks, or running relay nodes for any TLD.
- **Spend tokens** — For consuming bandwidth or accessing high-demand content.
- **Automatic revenue sharing**:
  - Site owners earn part of the tokens spent by visitors.
  - Content creators (image/video owners) earn a share when their work is viewed on other sites.
  - Social platforms and forums can attribute a crypto address to user profiles and posts so creators are rewarded.

This **replaces ad revenue** with **direct, usage-based payouts**.

---

## Current Status
- Overlay plane functional (sled-backed storage, TCP listener)
- Dev TCP transport for private messages with bandwidth metering
- CLI for overlay get/put, message send, usage stats
- Initial Tor transport integration for hidden services

---

## Planned Features
- Decentralized identity with `.sso` profiles
- Universal login for all Web3 sites on the network
- Automated micropayments for content use
- Discovery via optional Kademlia DHT
- Amnesia mode for ephemeral nodes

---

## Build & Run

```bash
# Build
cargo build

# Start node
cargo run -p node --bin ronode -- run

# Put a file in overlay
echo "hello rusty onions" > hello.txt
cargo run -p node --bin ronode -- overlay-put hello.txt

# Get it back
cargo run -p node --bin ronode -- overlay-get <HASH> out.txt

# Send a tiny message
cargo run -p node --bin ronode -- msg-send 127.0.0.1:47110 ping

# Tor hidden service PUT/GET example
ONION=<your_hidden_service>.onion:1777
HASH=$(cargo run -p node -- put --transport tor --to "$ONION" hello.txt | tail -n1)
cargo run -p node -- get --transport tor --to "$ONION" "$HASH" out.txt

```

## Legal & Safety Guidelines

These guidelines apply to **all users and contributors** of RustyOnions. They are mandatory to ensure the project remains safe, legal, and ethical in all jurisdictions where it operates.

RustyOnions uses Tor for transport. To ensure **legal and ethical** use:

1. **No illegal content** — Never share or store anything that violates laws.
2. **Public overlay plane is for safe data only** — e.g., maps, open data, free media.
3. **End-to-end encryption is for privacy**, not concealment of unlawful acts.
4. **Respect Tor bandwidth** — contribute back via relay mode when possible.
5. **No targeted attacks** — The network must not be used for harassment, intrusion, or unauthorized access.

## License
MIT License — see [LICENSE](LICENSE)

## Credits
This project was created collaboratively by **Stevan White** with assistance from **OpenAI’s ChatGPT** (GPT-5).
All generated code was reviewed, adapted, and integrated to fit the project’s goals.
