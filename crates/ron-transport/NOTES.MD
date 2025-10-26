### BEGIN NOTE - ron-transport - OCTOBER 25 2025 - 19:40 CST

here’s a clean, copy-pasteable status brief you can drop into your notes.

# ron-transport — status, how to run, and next steps

## What we built (MVP = ✅)

* **TCP listener** with:

  * connection cap via `Semaphore`
  * **read timeout** and **idle timeout** (per-conn)
  * robust accept-loop with small backoff on errors
* **TLS support** (feature `tls`) using `tokio-rustls` (rustls 0.22 API):

  * optional server config; when present, acceptor wraps the TCP stream
  * **graceful TLS close** (`close_notify`) on shutdown
* **Reader** (`conn/reader.rs`):

  * generic over `AsyncRead`
  * tracks `bytes_in`
  * enforces `read_timeout` + `idle_timeout`
* **Writer** (`conn/writer.rs`):

  * single-writer task per connection (bounded MPSC queue)
  * generic over `AsyncWrite` (TCP + TLS)
  * flushes per message; increments **`bytes_out`**
  * graceful shutdown (`AsyncWrite::shutdown`) → FIN / TLS `close_notify`
* **Metrics** (`metrics.rs`):

  * `connections`, `bytes_in`, `bytes_out`, `latency_seconds`, `rejected_total`
* **Shutdown handle** (public):

  * `spawn_transport_with_cancel(...) -> TransportHandle { task, addr, cancel }`
  * Back-compat wrapper `spawn_transport(...) -> (JoinHandle, SocketAddr)`
* **Scripts & examples** (for human smoke):

  * `examples/http_echo.rs` — easy **curl** visible response
  * `examples/tcp_echo.rs` — raw TCP echo
  * `examples/tls_transport.rs` — TLS accept using library
  * `scripts/local/mk_self_signed.sh` — self-signed cert and key
  * `scripts/local/smoke_tls.sh` — builds + connects with `openssl s_client`
  * `scripts/local/http_echo_smoke.sh` — spins http_echo, probes with curl
  * (You also have `echo_smoke.sh` and `smoke_transport.sh` from earlier work)
* **Integration tests** (green):

  * `idle_timeout_closes` — verifies idle close (race-resilient)
  * `over_capacity_second_conn_dropped` — rejects > max_conns
  * `tls_accepts_handshake` (behind `feature = "tls"`) — rustls client handshake

> Note: **Bus events** parameter is reserved; we do not call methods yet pending API confirmation in `ron-kernel::Bus<T>`.

---

## Project layout (high-value files)

```
src/
  lib.rs                # exports spawn_* APIs; TLS alias; modules
  config.rs             # TransportConfig (timeouts, caps, address, name)
  metrics.rs            # Prometheus metric families
  conn/
    reader.rs           # AsyncRead loop; bytes_in; timeouts
    writer.rs           # Single-writer; bytes_out; shutdown
  tcp/
    listener.rs         # accept loop; TLS upgrade; run_plain/run_tls
  util/
    cancel.rs           # CancellationToken wrapper

examples/
  http_echo.rs          # curl-visible echo (HTTP/1.1 200)
  tcp_echo.rs           # raw echo (nc/curl --data-binary)
  tls_transport.rs      # library TLS listener, prints bound addr

tests/
  integration.rs        # test entry-point: `mod integration;`
  integration/
    idle_timeout.rs
    over_capacity.rs
    tls_accept.rs       # cfg(feature = "tls")
```

---

## How to build, run, and test

### Build (plain + TLS)

```
cargo build -p ron-transport
cargo build -p ron-transport --features tls
```

### Human smoke (HTTP echo)

```
crates/ron-transport/scripts/local/http_echo_smoke.sh
```

You should see:

```
[ OK ] http-echo server: 127.0.0.1:<PORT>
<full HTTP request echoed + body>
[ OK ] done
```

### Human smoke (TLS)

```
crates/ron-transport/scripts/local/mk_self_signed.sh
crates/ron-transport/scripts/local/smoke_tls.sh
```

Expected:

```
[ OK ] tls server: 127.0.0.1:<PORT>
[ OK ] openssl s_client connected + wrote bytes (no response expected)
```

### Examples directly

```
cargo run -p ron-transport --example http_echo
cargo run -p ron-transport --example tcp_echo
cargo run -p ron-transport --features tls --example tls_transport \
  crates/ron-transport/scripts/local/certs/cert.pem \
  crates/ron-transport/scripts/local/certs/key.pem
```

From another terminal (TLS):

```
printf 'hello over TLS\n' | openssl s_client -quiet -connect 127.0.0.1:<PORT> -tls1_3 -servername localhost
```

If you see only the certificate lines and then a clean exit (no “unexpected EOF”), the server sent `close_notify` correctly.

### Tests (integration)

> You chose the “**keep subfolder**” layout: `tests/integration.rs` (entry) + `tests/integration/*.rs` (modules).

```
cargo test -p ron-transport
cargo test -p ron-transport --features tls
```

* `tls_accepts_handshake` will run if certs exist at:
  `crates/ron-transport/scripts/local/certs/cert.pem` and `key.pem`.
  If not present, it **skips** (doesn’t fail CI).

---

## Key implementation notes / invariants

* **Single writer per connection**: prevents interleaved writes; backpressure is applied via `mpsc::channel(64)`.
* **Timeouts**:

  * `read_timeout`: per read
  * `idle_timeout`: if no bytes are seen for this duration, connection closes
* **Graceful close**:

  * Plain TCP: `AsyncWrite::shutdown()` → FIN
  * TLS: `shutdown()` → `close_notify` alert then TLS-level close
* **Metrics** are labeled by `cfg.name` (e.g., `"tls"`, `"test"`) for easy aggregation.
* **Cancel**:

  * `spawn_transport_with_cancel` returns a `Cancel` token. `cancel.cancel()` requests listener exit; existing connections finish naturally.

---

## Troubleshooting quick hits

* `curl: (28) Operation timed out` against library listener:

  * Expected: the **library** listener doesn’t speak HTTP; use `http_echo` example if you want a visible response.
* `openssl s_client ... unexpected eof while reading`:

  * Means the server didn’t send `close_notify` (likely an older binary still running). Kill old process and run the current `tls_transport` example.
* TLS test skipped:

  * Run `crates/ron-transport/scripts/local/mk_self_signed.sh` once to create `cert.pem` / `key.pem`.

---

## What’s left before we call this crate “done”

### 1) Wire **Bus events** (tiny)

* Emit on accept:

  ```
  TransportEvent::Connected { peer, name: cfg.name }
  ```
* Emit on close:

  ```
  TransportEvent::Disconnected { peer, name: cfg.name, reason: <enum> }
  ```
* **Needed from you**: the exact method on `ron_kernel::Bus<TransportEvent>` (e.g., `bus.publish(evt)`, `bus.push(evt)`, `bus.broadcast(&evt)`, etc.). As soon as you confirm, we’ll patch `tcp/listener.rs` in two places.

### 2) Public shutdown example (optional)

* A short example showing `spawn_transport_with_cancel`, then calling `handle.cancel.cancel()` and awaiting `handle.task`.

### 3) Docs polish (quick)

* `README.md`: short “MVP guarantees & limits” section
* `ALL_DOCS.md`: mark which blueprint items are complete, which are deferred
* `TODO.MD` in this crate: flip remaining checkboxes accordingly

### 4) (Deferred features to later crates / phases)

* **Dialers** (TCP + TLS) with connect timeouts
* **Handler trait** (e.g., `on_bytes(&[u8]) -> Option<Bytes>`) for service response path
* **Arti/Tor** feature
* **QUIC** feature
* **Benches** (`criterion`) finalized
* **Fuzz** target wiring and **loom** test expansion
* **Specs** CI (TLA job runs the handshake spec)

---

## Ready-to-use API snippets

### Spawn and stop (in a service)

```rust
use ron_transport::{config::TransportConfig, metrics::TransportMetrics, spawn_transport_with_cancel};
use ron_kernel::{Bus, HealthState};
use std::sync::Arc;

let mut cfg = TransportConfig::default();
cfg.name = "svc";
let metrics = TransportMetrics::new("ron");
let health = Arc::new(HealthState::new());
let bus: Bus<ron_transport::types::TransportEvent> = Bus::new();

let handle = spawn_transport_with_cancel(cfg, metrics, health, bus, None).await?;
println!("listening on {}", handle.addr);

// later: graceful shutdown
handle.cancel.cancel();
let _ = handle.task.await;
```

---

## Short summary (for your changelog)

* **MVP complete**: TCP/TLS listener, reader & writer paths, timeouts, graceful close, metrics, shutdown handle, robust integration tests, and human smoke scripts/examples.
* **Open tiny item**: emit Bus events once `Bus<TransportEvent>` publish method is confirmed.

### END NOTE - OCTOBER 25 2025 - 19:40 CST

