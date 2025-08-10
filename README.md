# RustyOnions

RustyOnions is an experimental two-plane peer-to-peer system built in Rust:

- **Overlay Plane**: Public data distribution (map chunks + safe metadata) stored redundantly among all users.
- **Private Message Plane**: Small, end-to-end encrypted messages sent via Tor (via the Arti Rust Tor implementation) for anonymity and security.

**Responsible design**: Each user’s Tor bandwidth usage is metered, and users are encouraged to contribute 2× that bandwidth back to the Tor network as a middle relay.

## Current Status
- Overlay plane functional (sled-backed storage, TCP listener)
- Dev TCP transport for private messages with bandwidth metering
- CLI for overlay get/put, message send, usage stats
- Relay helper stub

## Planned
- Arti integration for Tor transport
- Hidden service inbox for each user
- Encrypted, onion-routed private messaging
- Tor relay helper with dynamic bandwidth caps based on usage

## Build & Run
```bash
# First time
cargo build

# Start node
cargo run -p node -- run

# Put a file in overlay
echo "hello rusty onions" > hello.txt
cargo run -p node -- overlayput --file hello.txt

# Get it back
cargo run -p node -- overlayget --hash <HASH> --out out.txt

# Send a tiny message over dev transport
cargo run -p node -- msgsend --to 127.0.0.1:47110 --text "ping"

# View bandwidth stats (per-process; resets on restart)
cargo run -p node -- stats
```

## License
MIT License — see [LICENSE](LICENSE)

## Credits
This project was created collaboratively by **Stevan White** with assistance from **OpenAI’s ChatGPT** (GPT-5).
All generated code was reviewed, adapted, and integrated to fit the project’s goals.
