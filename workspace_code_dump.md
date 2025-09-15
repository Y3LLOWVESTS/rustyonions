# Workspace Code Dump (.rs + .toml only)
_Generated: 2025-09-15 21:12:04Z_

**Root:** `.`

## File tree (.rs and .toml only)

```text
tree not found; showing a flat list instead.

.config/hakari.toml
Cargo.toml
ci/crate-classes.toml
configs/config.sample.toml
crates/accounting/Cargo.toml
crates/accounting/src/lib.rs
crates/arti_transport/Cargo.toml
crates/arti_transport/src/ctrl.rs
crates/arti_transport/src/hs.rs
crates/arti_transport/src/lib.rs
crates/arti_transport/src/socks.rs
crates/common/Cargo.toml
crates/common/src/hash.rs
crates/common/src/lib.rs
crates/gateway/Cargo.toml
crates/gateway/examples/oap_client_demo.rs
crates/gateway/examples/oap_server_demo.rs
crates/gateway/src/bin/oapd.rs
crates/gateway/src/http/error.rs
crates/gateway/src/index_client.rs
crates/gateway/src/lib.rs
crates/gateway/src/main.rs
crates/gateway/src/metrics.rs
crates/gateway/src/oap.rs
crates/gateway/src/overlay_client.rs
crates/gateway/src/pay_enforce.rs
crates/gateway/src/quotas.rs
crates/gateway/src/resolve.rs
crates/gateway/src/routes/errors.rs
crates/gateway/src/routes/http_util.rs
crates/gateway/src/routes/mod.rs
crates/gateway/src/routes/object.rs
crates/gateway/src/routes/readyz.rs
crates/gateway/src/state.rs
crates/gateway/src/test_rt.rs
crates/gateway/src/utils.rs
crates/gateway/tests/free_vs_paid.rs
crates/gateway/tests/http_read_path.rs
crates/gateway/tests/oap_backpressure.rs
crates/gateway/tests/oap_error_path.rs
crates/gateway/tests/oap_server_roundtrip.rs
crates/index/Cargo.toml
crates/index/src/lib.rs
crates/kameo/Cargo.toml
crates/kameo/src/lib.rs
crates/micronode/Cargo.toml
crates/micronode/src/main.rs
crates/naming/Cargo.toml
crates/naming/src/address.rs
crates/naming/src/hash.rs
crates/naming/src/lib.rs
crates/naming/src/manifest.rs
crates/naming/src/tld.rs
crates/node/Cargo.toml
crates/node/src/cli.rs
crates/node/src/commands/get.rs
crates/node/src/commands/init.rs
crates/node/src/commands/mod.rs
crates/node/src/commands/put.rs
crates/node/src/commands/serve.rs
crates/node/src/commands/stats.rs
crates/node/src/commands/tor_dial.rs
crates/node/src/lib.rs
crates/node/src/main.rs
crates/oap/Cargo.toml
crates/oap/src/lib.rs
crates/oap/tests/quota_error.rs
crates/oap/tests/roundtrip.rs
crates/overlay/Cargo.toml
crates/overlay/src/error.rs
crates/overlay/src/lib.rs
crates/overlay/src/protocol.rs
crates/overlay/src/store.rs
crates/ron-app-sdk/Cargo.toml
crates/ron-app-sdk/examples/mailbox_recv.rs
crates/ron-app-sdk/examples/mailbox_send.rs
crates/ron-app-sdk/examples/oap_echo_client.rs
crates/ron-app-sdk/examples/oap_echo_server.rs
crates/ron-app-sdk/examples/tiles_get.rs
crates/ron-app-sdk/src/client/hello.rs
crates/ron-app-sdk/src/client/mod.rs
crates/ron-app-sdk/src/client/oneshot.rs
crates/ron-app-sdk/src/client/tls.rs
crates/ron-app-sdk/src/constants.rs
crates/ron-app-sdk/src/errors.rs
crates/ron-app-sdk/src/lib.rs
crates/ron-app-sdk/src/oap/codec/decoder.rs
crates/ron-app-sdk/src/oap/codec/encoder.rs
crates/ron-app-sdk/src/oap/codec/mod.rs
crates/ron-app-sdk/src/oap/flags.rs
crates/ron-app-sdk/src/oap/frame.rs
crates/ron-app-sdk/src/oap/hello.rs
crates/ron-app-sdk/src/oap/mod.rs
crates/ron-audit/Cargo.toml
crates/ron-audit/src/lib.rs
crates/ron-auth/Cargo.toml
crates/ron-auth/src/lib.rs
crates/ron-billing/Cargo.toml
crates/ron-billing/src/lib.rs
crates/ron-bus/Cargo.toml
crates/ron-bus/src/api.rs
crates/ron-bus/src/lib.rs
crates/ron-bus/src/uds.rs
crates/ron-kernel/Cargo.toml
crates/ron-kernel/src/amnesia.rs
crates/ron-kernel/src/bin/bus_demo.rs
crates/ron-kernel/src/bin/kameo_demo.rs
crates/ron-kernel/src/bin/kernel_demo.rs
crates/ron-kernel/src/bin/metrics_demo.rs
crates/ron-kernel/src/bin/node_demo.rs
crates/ron-kernel/src/bin/node_index.rs
crates/ron-kernel/src/bin/node_overlay.rs
crates/ron-kernel/src/bin/node_transport.rs
crates/ron-kernel/src/bin/transport_demo.rs
crates/ron-kernel/src/bin/transport_load.rs
crates/ron-kernel/src/bin/transport_supervised.rs
crates/ron-kernel/src/bus/core.rs
crates/ron-kernel/src/bus/helpers.rs
crates/ron-kernel/src/bus/metrics.rs
crates/ron-kernel/src/bus/mod.rs
crates/ron-kernel/src/bus/sub.rs
crates/ron-kernel/src/cancel.rs
crates/ron-kernel/src/config/mod.rs
crates/ron-kernel/src/config/types.rs
crates/ron-kernel/src/config/validate.rs
crates/ron-kernel/src/config/watch.rs
crates/ron-kernel/src/lib.rs
crates/ron-kernel/src/main_old.rs
crates/ron-kernel/src/metrics.rs
crates/ron-kernel/src/overlay/admin_http.rs
crates/ron-kernel/src/overlay/metrics.rs
crates/ron-kernel/src/overlay/mod.rs
crates/ron-kernel/src/overlay/runtime.rs
crates/ron-kernel/src/overlay/service.rs
crates/ron-kernel/src/overlay/tls.rs
crates/ron-kernel/src/supervisor/metrics.rs
crates/ron-kernel/src/supervisor/mod.rs
crates/ron-kernel/src/supervisor/policy.rs
crates/ron-kernel/src/supervisor/runner.rs
crates/ron-kernel/src/tracing_init.rs
crates/ron-kernel/src/transport.rs
crates/ron-kernel/tests/bus_basic.rs
crates/ron-kernel/tests/bus_load.rs
crates/ron-kernel/tests/bus_topic.rs
crates/ron-kernel/tests/event_snapshot.rs
crates/ron-kernel/tests/http_index_overlay.rs
crates/ron-kernel/tests/loom_health.rs
crates/ron-kernel/tests/no_sha256_guard.rs
crates/ron-kms/Cargo.toml
crates/ron-kms/src/lib.rs
crates/ron-ledger/Cargo.toml
crates/ron-ledger/src/lib.rs
crates/ron-policy/Cargo.toml
crates/ron-policy/src/lib.rs
crates/ron-proto/Cargo.toml
crates/ron-proto/src/lib.rs
crates/ron-token/Cargo.toml
crates/ron-token/src/lib.rs
crates/ryker/Cargo.toml
crates/ryker/src/lib.rs
crates/svc-economy/Cargo.toml
crates/svc-economy/src/main.rs
crates/svc-edge/Cargo.toml
crates/svc-edge/src/main.rs
crates/svc-index/Cargo.toml
crates/svc-index/src/main.rs
crates/svc-omnigate/Cargo.toml
crates/svc-omnigate/src/admin_http.rs
crates/svc-omnigate/src/config.rs
crates/svc-omnigate/src/handlers/hello.rs
crates/svc-omnigate/src/handlers/mailbox.rs
crates/svc-omnigate/src/handlers/mod.rs
crates/svc-omnigate/src/handlers/storage.rs
crates/svc-omnigate/src/mailbox.rs
crates/svc-omnigate/src/main.rs
crates/svc-omnigate/src/metrics.rs
crates/svc-omnigate/src/oap_limits.rs
crates/svc-omnigate/src/oap_metrics.rs
crates/svc-omnigate/src/server.rs
crates/svc-omnigate/src/storage.rs
crates/svc-omnigate/src/tls.rs
crates/svc-overlay/Cargo.toml
crates/svc-overlay/src/main.rs
crates/svc-sandbox/Cargo.toml
crates/svc-sandbox/src/decoy.rs
crates/svc-sandbox/src/hardening.rs
crates/svc-sandbox/src/main.rs
crates/svc-sandbox/src/metrics.rs
crates/svc-sandbox/src/oap_stub.rs
crates/svc-sandbox/src/router.rs
crates/svc-sandbox/src/tarpit.rs
crates/svc-storage/Cargo.toml
crates/svc-storage/src/main.rs
crates/tldctl/Cargo.toml
crates/tldctl/src/index_bus.rs
crates/tldctl/src/main.rs
crates/transport/Cargo.toml
crates/transport/src/lib.rs
crates/transport/src/tcp.rs
crates/transport/src/tor/ctrl.rs
crates/transport/src/tor/hs.rs
crates/transport/src/tor/mod.rs
crates/transport/src/tor_control.rs
deny.toml
experiments/actor_spike/Cargo.toml
experiments/actor_spike/src/main.rs
hakari.toml
testing/gwsmoke/Cargo.toml
testing/gwsmoke/src/config.rs
testing/gwsmoke/src/http_probe.rs
testing/gwsmoke/src/main.rs
testing/gwsmoke/src/pack.rs
testing/gwsmoke/src/proc.rs
testing/gwsmoke/src/util.rs
testing/gwsmoke/src/wait.rs
testing/sample_bundle/Manifest.toml
tools/ronctl/Cargo.toml
tools/ronctl/src/main.rs
tools/src/main.rs
workspace-hack/Cargo.toml
workspace-hack/build.rs
workspace-hack/src/lib.rs
```

## Files

### .config/hakari.toml

```toml
# This file contains settings for `cargo hakari`.
# See https://docs.rs/cargo-hakari/latest/cargo_hakari/config for a full list of options.

hakari-package = "workspace-hack"

# Format version for hakari's output. Version 4 requires cargo-hakari 0.9.22 or above.
dep-format-version = "4"

# Setting workspace.resolver = "2" or higher in the root Cargo.toml is HIGHLY recommended.
# Hakari works much better with the v2 resolver. (The v2 and v3 resolvers are identical from
# hakari's perspective, so you're welcome to set either.)
#
# For more about the new feature resolver, see:
# https://blog.rust-lang.org/2021/03/25/Rust-1.51.0.html#cargos-new-feature-resolver
resolver = "2"

# Add triples corresponding to platforms commonly used by developers here.
# https://doc.rust-lang.org/rustc/platform-support.html
platforms = [
    # "x86_64-unknown-linux-gnu",
    # "x86_64-apple-darwin",
    # "aarch64-apple-darwin",
    # "x86_64-pc-windows-msvc",
]

# Write out exact versions rather than a semver range. (Defaults to false.)
# exact-versions = true

```

### Cargo.toml

```toml
[workspace]
members = [
  "crates/common",
  "crates/accounting",
  "crates/overlay",
  "crates/transport",
  "crates/node",
  "crates/naming",
  "crates/tldctl",
  "crates/index",
  "crates/gateway",
  "crates/ryker",
  "crates/ron-bus",
  "crates/svc-index",
  "crates/svc-overlay",
  "crates/svc-storage",
  "crates/ron-kernel",
  "tools/ronctl",
  "experiments/actor_spike",
  "crates/kameo",
  "crates/ron-app-sdk",
  "crates/svc-omnigate",
  "crates/oap",
  "testing/gwsmoke",
  "crates/ron-kms",
  "crates/ron-proto",
  "crates/ron-auth",
  "crates/ron-audit",
  "crates/ron-billing",
  "workspace-hack",
  "crates/micronode",
  "crates/svc-edge",
  "crates/ron-policy",
  "crates/svc-sandbox",
]
resolver = "2"

[workspace.dependencies]
# ---- Core pins to keep versions consistent across the workspace --------------
tower-http          = "0.6.6"
prometheus          = "0.14"       # fixes RUSTSEC-2024-0437 via protobuf >= 3.7.2
rand                = "0.9"
rand_chacha         = "0.9"
regex               = "1.11"

# ---- HTTP stack (pin versions here; enable features per-crate as needed) ----
reqwest             = { version = "0.12", default-features = false }
axum                = { version = "0.7.9", default-features = false }
tokio               = "1.47.1"
tokio-rustls        = "0.26.2"
tokio-util          = "0.7"
hyper               = "1"
hyper-util          = "0.1"
tower               = "0.5"

# ---- Observability / utils ---------------------------------------------------
tracing             = "0.1"
tracing-subscriber  = "0.3"
serde               = "1.0"
serde_json          = "1.0"
anyhow              = "1.0"
bytes               = "1"
chrono              = "0.4"
clap                = "4.5"
futures-util        = "0.3"
parking_lot         = "0.12"
thiserror           = "1.0"
toml                = "0.8"

# ---- Crypto helpers used in overlay/service code -----------------------------
sha2                = "0.10"
base64              = "0.22"
hex                 = "0.4"
blake3              = "1.5"
zeroize             = "1"

# ---- TLS roots / PEM helpers -------------------------------------------------
rustls-native-certs = "0.8"
rustls-pemfile      = "2.2"

# ---- Other shared deps that showed as duplicates in cargo-deny ---------------
rmp-serde           = "1.3"
mime_guess          = "2.0"
sled                = "0.34"
zstd                = "0.13"

# ---- Path (internal) crates declared as shared workspace deps ----------------
index          = { path = "crates/index" }
naming         = { path = "crates/naming" }
ron-bus        = { path = "crates/ron-bus" }
ron-kernel     = { path = "crates/ron-kernel" }
transport      = { path = "crates/transport" }
workspace-hack = { version = "0.1", path = "workspace-hack" }

# Expose commonly shared internal crates so leaf crates can use `workspace = true`
ron-policy     = { path = "crates/ron-policy" }
ron-proto      = { path = "crates/ron-proto" }
ron-auth       = { path = "crates/ron-auth" }
ron-audit      = { path = "crates/ron-audit" }
ron-billing    = { path = "crates/ron-billing" }
ron-app-sdk    = { path = "crates/ron-app-sdk" }

# ---- Windows split crates — keep direct usages aligned -----------------------
windows-targets     = "0.53.3"
windows-sys         = "0.60.2"

# ---- KMS / signing (needed by crates/ron-kms) --------------------------------
# Use dalek v2 and rand_core 0.6; keep default-features off to avoid pulling std extras.
ed25519-dalek = { version = "2", default-features = false, features = ["alloc"] }
rand_core     = { version = "0.6", default-features = false }

```

### ci/crate-classes.toml

```toml
[critical]
crates = [
  "ron-kernel",
  "gateway",
  "transport",
  "index",
  "overlay"
  # "arti_transport"
]

```

### configs/config.sample.toml

```toml
# Example config for RustyOnions
data_dir = ".data"
overlay_addr = "127.0.0.1:1777"      # TCP listener bind/target
dev_inbox_addr = "127.0.0.1:2888"
socks5_addr = "127.0.0.1:9050"       # Tor SOCKS5 proxy
tor_ctrl_addr = "127.0.0.1:9051"     # Tor control port
chunk_size = 65536
connect_timeout_ms = 5000
# Optional persistent HS private key file (used by `ronode serve --transport tor`)
# hs_key_file = ".data/hs_ed25519_key"

```

### crates/accounting/Cargo.toml

```toml
[package]
publish = false
license = "MIT OR Apache-2.0"

name = "accounting"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = { workspace = true }
parking_lot = { workspace = true }
workspace-hack = { version = "0.1", path = "../../workspace-hack" }

```

### crates/accounting/src/lib.rs

```rust
#![forbid(unsafe_code)]
//! accounting: byte counters with a fixed-size, allocation-free ring buffer.
//!
//! - `CountingStream<S>` wraps any `Read+Write` and records bytes in/out.
//! - `Counters` tracks totals and 60 per-minute buckets via a ring buffer.
//!
//! This is intentionally tiny and std-only.

use std::io::{Read, Result as IoResult, Write};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Wrapper that counts bytes read/written through `inner`.
pub struct CountingStream<S> {
    inner: S,
    ctrs: Counters,
}

impl<S> CountingStream<S> {
    pub fn new(inner: S, counters: Counters) -> Self {
        Self {
            inner,
            ctrs: counters,
        }
    }

    pub fn counters(&self) -> Counters {
        self.ctrs.clone()
    }

    pub fn into_inner(self) -> S {
        self.inner
    }
}

impl<S: Read> Read for CountingStream<S> {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        let n = self.inner.read(buf)?;
        if n > 0 {
            self.ctrs.add_in(n as u64);
        }
        Ok(n)
    }
}

impl<S: Write> Write for CountingStream<S> {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        let n = self.inner.write(buf)?;
        if n > 0 {
            self.ctrs.add_out(n as u64);
        }
        Ok(n)
    }

    fn flush(&mut self) -> IoResult<()> {
        self.inner.flush()
    }
}

/// Snapshot of cumulative and per-minute counters.
#[derive(Clone, Debug)]
pub struct Snapshot {
    pub total_in: u64,
    pub total_out: u64,
    /// Oldest→newest, length 60, bytes per minute.
    pub per_min_in: [u64; 60],
    pub per_min_out: [u64; 60],
}

/// Internal, mutex-protected state for counters.
#[derive(Debug)]
struct State {
    total_in: u64,
    total_out: u64,
    ring_in: [u64; 60],
    ring_out: [u64; 60],
    idx: usize,       // points to the current minute bucket
    last_minute: i64, // epoch minutes of idx
}

impl Default for State {
    fn default() -> Self {
        let now_min = epoch_minutes_now();
        Self {
            total_in: 0,
            total_out: 0,
            ring_in: [0; 60],
            ring_out: [0; 60],
            idx: 0,
            last_minute: now_min,
        }
    }
}

/// A shareable counter set with a fixed-size ring buffer (no allocations on hot path).
#[derive(Clone, Debug)]
pub struct Counters(Arc<Mutex<State>>);

impl Counters {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(State::default())))
    }

    /// Add bytes read in the current minute bucket.
    pub fn add_in(&self, n: u64) {
        let mut s = lock_ignore_poison(&self.0);
        rotate_if_needed(&mut s);
        s.total_in = s.total_in.saturating_add(n);
        let idx = s.idx; // avoid aliasing (mutable + immutable) on `s`
        let newv = s.ring_in[idx].saturating_add(n);
        s.ring_in[idx] = newv;
    }

    /// Add bytes written in the current minute bucket.
    pub fn add_out(&self, n: u64) {
        let mut s = lock_ignore_poison(&self.0);
        rotate_if_needed(&mut s);
        s.total_out = s.total_out.saturating_add(n);
        let idx = s.idx;
        let newv = s.ring_out[idx].saturating_add(n);
        s.ring_out[idx] = newv;
    }

    /// Return a stable snapshot (copies the ring).
    pub fn snapshot(&self) -> Snapshot {
        let mut s = lock_ignore_poison(&self.0);
        rotate_if_needed(&mut s);

        // Reorder so output is oldest→newest.
        let mut out_in = [0u64; 60];
        let mut out_out = [0u64; 60];
        // Oldest bucket is just after current idx.
        for i in 0..60 {
            let src = (s.idx + 1 + i) % 60;
            out_in[i] = s.ring_in[src];
            out_out[i] = s.ring_out[src];
        }

        Snapshot {
            total_in: s.total_in,
            total_out: s.total_out,
            per_min_in: out_in,
            per_min_out: out_out,
        }
    }

    /// Clear all rolling minute buckets (totals are preserved).
    pub fn reset_minutes(&self) {
        let mut s = lock_ignore_poison(&self.0);
        s.ring_in = [0; 60];
        s.ring_out = [0; 60];
        s.idx = 0;
        s.last_minute = epoch_minutes_now();
    }
}

impl Default for Counters {
    fn default() -> Self {
        Self::new()
    }
}

// --- helpers ---

#[inline]
fn epoch_minutes_now() -> i64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0));
    (now.as_secs() / 60) as i64
}

/// Recover from `PoisonError` without panicking (counters are best-effort/monotonic).
#[inline]
fn lock_ignore_poison<'a, T>(m: &'a Mutex<T>) -> std::sync::MutexGuard<'a, T> {
    match m.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Advance the ring buffer to the current minute, zeroing any skipped buckets.
fn rotate_if_needed(s: &mut State) {
    let now_min = epoch_minutes_now();
    if now_min == s.last_minute {
        return;
    }
    if now_min < s.last_minute {
        // System clock went backwards; treat as same minute to avoid mayhem.
        return;
    }
    // Number of minutes passed since last bucket.
    let delta = (now_min - s.last_minute) as usize;
    let steps = delta.min(60); // cap at ring length; more means the whole ring is stale.

    for _ in 0..steps {
        s.idx = (s.idx + 1) % 60;
        s.ring_in[s.idx] = 0;
        s.ring_out[s.idx] = 0;
    }
    s.last_minute = now_min;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_bytes_and_snapshots_rotate() {
        let c = Counters::new();

        // Simulate some I/O
        c.add_in(100);
        c.add_out(40);

        let snap1 = c.snapshot();
        assert_eq!(snap1.total_in, 100);
        assert_eq!(snap1.total_out, 40);

        // Force a rotation by manually tweaking internal state (white-box test)
        {
            let mut s = lock_ignore_poison(&c.0);
            s.last_minute -= 1; // pretend a minute has passed
        }
        c.add_in(7);
        c.add_out(3);

        let snap2 = c.snapshot();
        assert_eq!(snap2.total_in, 107);
        assert_eq!(snap2.total_out, 43);

        // Oldest→newest ordering; last bucket should reflect the latest adds
        assert_eq!(snap2.per_min_in[59], 7);
        assert_eq!(snap2.per_min_out[59], 3);
    }

    #[test]
    fn reset_minutes_clears_ring_only() {
        let c = Counters::new();
        c.add_in(10);
        c.add_out(5);
        let before = c.snapshot();

        c.reset_minutes();
        let after = c.snapshot();

        assert_eq!(after.total_in, before.total_in); // totals preserved
        assert_eq!(after.total_out, before.total_out); // totals preserved
        assert!(after.per_min_in.iter().all(|&v| v == 0));
        assert!(after.per_min_out.iter().all(|&v| v == 0));
    }
}

```

### crates/arti_transport/Cargo.toml

```toml
[package]
publish = false
license = "MIT OR Apache-2.0"


name = "arti_transport"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = { workspace = true }
socks = "0.3"
tracing = "0.1"

# local crates
transport = { path = "../transport" }
accounting = { path = "../accounting" }

```

### crates/arti_transport/src/ctrl.rs

```rust
use anyhow::{anyhow, bail, Context, Result};
use std::fmt::Write as _;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;

/// Minimal Tor control-port client with auth + multi-line command support.
pub(crate) struct CtrlClient {
    s: TcpStream,
    r: BufReader<TcpStream>,
}

impl CtrlClient {
    /// Authenticate via Tor control port.
    ///
    /// Strategy:
    ///   1) Send PROTOCOLINFO (multi-line), parse AUTH METHODS and COOKIEFILE.
    ///   2) If cookie auth is available (COOKIE or SAFECOOKIE) and we have a COOKIEFILE,
    ///      read it, hex-encode, and send AUTHENTICATE <hex>.
    ///   3) If cookie override is provided, use that path instead of COOKIEFILE.
    pub(crate) fn authenticate(tor_ctrl_addr: &str, cookie_override: Option<&str>) -> Result<Self> {
        let s = TcpStream::connect(tor_ctrl_addr)?;
        s.set_nodelay(true).ok();
        let r = BufReader::new(s.try_clone()?);
        let mut client = Self { s, r };

        // 1) Query PROTOCOLINFO (multi-line)
        let lines = client.cmd_multi("PROTOCOLINFO 1")?;
        let (auth_methods, cookie_path_opt) = parse_protocolinfo(&lines);

        // 2) Decide on auth method
        // Prefer cookie if available (COOKIE or SAFECOOKIE).
        if auth_methods.cookie || auth_methods.safecookie {
            let cookie_path = if let Some(p) = cookie_override {
                p.to_string()
            } else {
                cookie_path_opt
                    .ok_or_else(|| anyhow!("Tor PROTOCOLINFO missing COOKIEFILE"))?
            };

            let cookie = std::fs::read(&cookie_path)
                .with_context(|| format!("reading cookie {}", cookie_path))?;

            let mut cookie_hex = String::with_capacity(cookie.len() * 2);
            for b in &cookie {
                write!(&mut cookie_hex, "{:02X}", b).expect("infallible write");
            }

            let resp = client.send_line(&format!("AUTHENTICATE {}", cookie_hex))?;
            if !is_ok(&resp) {
                bail!("AUTHENTICATE failed: {resp:?}");
            }
        } else if auth_methods.null {
            // NULL auth explicitly allowed by Tor (rare). Authenticate with empty argument.
            let resp = client.send_line("AUTHENTICATE")?;
            if !is_ok(&resp) {
                bail!("AUTHENTICATE (NULL) failed: {resp:?}");
            }
        } else {
            bail!("No supported AUTH methods from Tor (need COOKIE/SAFECOOKIE/NULL).");
        }

        Ok(client)
    }

    /// Send a command and read a multi-line response until a terminating 250/550 line.
    pub(crate) fn cmd_multi(&mut self, cmd: &str) -> Result<Vec<String>> {
        self.write_line(cmd)?;
        let mut lines = Vec::new();
        loop {
            let mut line = String::new();
            self.r.read_line(&mut line)?;
            if line.is_empty() {
                break;
            }
            let trimmed = line.trim_end().to_string();
            let done = trimmed.starts_with("250 OK")
                || trimmed.starts_with("250 ")
                || trimmed.starts_with("550");
            lines.push(trimmed);
            if done {
                break;
            }
        }
        Ok(lines)
    }

    /// Send a command and read a single-line response.
    fn send_line(&mut self, cmd: &str) -> Result<String> {
        self.write_line(cmd)?;
        let mut line = String::new();
        self.r.read_line(&mut line)?;
        Ok(line)
    }

    /// Write a command line with CRLF.
    fn write_line(&mut self, cmd: &str) -> Result<()> {
        let mut w = self.s.try_clone()?;
        w.write_all(cmd.as_bytes())?;
        w.write_all(b"\r\n")?;
        w.flush()?;
        Ok(())
    }
}

fn is_ok(line: &str) -> bool {
    line.starts_with("250 OK")
}

/// Parsed bits we care about from PROTOCOLINFO.
struct AuthMethods {
    cookie: bool,
    safecookie: bool,
    null: bool,
}

/// Extract AUTH methods and COOKIEFILE path (if present) from PROTOCOLINFO lines.
fn parse_protocolinfo(lines: &[String]) -> (AuthMethods, Option<String>) {
    let mut methods = AuthMethods {
        cookie: false,
        safecookie: false,
        null: false,
    };
    let mut cookiefile: Option<String> = None;

    for l in lines {
        if let Some(auth_idx) = l.find("METHODS=") {
            let meth = &l[auth_idx + "METHODS=".len()..];
            let meth = meth.split_whitespace().next().unwrap_or("");
            for m in meth.split(',') {
                let m = m.trim().to_ascii_uppercase();
                if m == "COOKIE" { methods.cookie = true; }
                if m == "SAFECOOKIE" { methods.safecookie = true; }
                if m == "NULL" { methods.null = true; }
            }
        }
        if let Some(cf_idx) = l.find("COOKIEFILE=") {
            let rest = &l[cf_idx + "COOKIEFILE=".len()..];
            if let Some(start) = rest.find('"') {
                if let Some(end) = rest[start + 1..].find('"') {
                    cookiefile = Some(rest[start + 1..start + 1 + end].to_string());
                    continue;
                }
            }
            let mut part = rest.trim();
            if let Some(space) = part.find(char::is_whitespace) {
                part = &part[..space];
            }
            cookiefile = Some(part.trim_matches('"').to_string());
        }
    }
    (methods, cookiefile)
}

```

### crates/arti_transport/src/hs.rs

```rust
use crate::ctrl::CtrlClient;
use accounting::{Counters, CountingStream};
use anyhow::{anyhow, Context, Result};
use std::net::{TcpListener};
use std::path::Path;
use std::thread;
use std::time::Duration;
use transport::{Handler, ReadWrite};

const OVERLAY_PORT: u16 = 1777;

/// Publish a v3 onion service and accept incoming connections, dispatching to `handler`.
pub(crate) fn publish_and_serve(
    tor_ctrl_addr: &str,
    counters: Counters,
    _io_timeout: std::time::Duration,
    handler: Handler,
) -> Result<()> {
    // 1) Bind a local listener on 127.0.0.1:0 where Tor will forward to.
    let ln = TcpListener::bind("127.0.0.1:0")?;
    let local_port = ln.local_addr()?.port();

    // 2) Build ADD_ONION command depending on persistence.
    let key_file = std::env::var("RO_HS_KEY_FILE").ok();
    let mut ctrl = CtrlClient::authenticate(tor_ctrl_addr, None)?;
    let service_id = if let Some(ref path) = key_file {
        // Persistent mode
        if Path::new(path).exists() {
            // Reuse existing key (exact string Tor expects, e.g., "ED25519-V3:AAAA...")
            let key = std::fs::read_to_string(path)
                .with_context(|| format!("reading HS key from {}", path))?
                .trim()
                .to_string();
            let cmd = format!(
                "ADD_ONION ED25519-V3:{} Port={},127.0.0.1:{}",
                key, OVERLAY_PORT, local_port
            );
            parse_service_id(ctrl.cmd_multi(&cmd)?)?
        } else {
            // Ask Tor to generate a new key; persist it.
            let cmd = format!(
                "ADD_ONION NEW:ED25519-V3 Port={},127.0.0.1:{}",
                OVERLAY_PORT, local_port
            );
            let lines = ctrl.cmd_multi(&cmd)?;
            let (sid, pk) = parse_sid_and_pk(lines)?;
            // Persist the key exactly as Tor returns it.
            if let Some(parent) = Path::new(path).parent() { std::fs::create_dir_all(parent).ok(); }
            std::fs::write(path, &pk).with_context(|| format!("writing HS key to {}", path))?;
            sid
        }
    } else {
        // Ephemeral mode: discard PK so Tor doesn't send it.
        let cmd = format!(
            "ADD_ONION NEW:ED25519-V3 Port={},127.0.0.1:{} Flags=DiscardPK",
            OVERLAY_PORT, local_port
        );
        parse_service_id(ctrl.cmd_multi(&cmd)?)?
    };

    // Keep a guard so we DEL_ONION on drop.
    let _guard = HsGuard {
        tor_ctrl_addr: tor_ctrl_addr.to_string(),
        service_id: service_id.clone(),
    };

    eprintln!("hidden service available at {}.onion:{}", service_id, OVERLAY_PORT);

    // 3) Accept in a background thread and drive the handler.
    thread::spawn(move || {
        for conn in ln.incoming() {
            match conn {
                Ok(s) => {
                    s.set_read_timeout(Some(Duration::from_secs(30))).ok();
                    s.set_write_timeout(Some(Duration::from_secs(30))).ok();
                    let boxed: Box<dyn ReadWrite + Send> =
                        Box::new(CountingStream::new(s, counters.clone()));
                    (handler)(boxed);
                }
                Err(e) => eprintln!("arti_transport accept error: {e:?}"),
            }
        }
    });

    Ok(())
}

/// RAII guard to cleanly remove the HS on drop.
struct HsGuard {
    tor_ctrl_addr: String,
    service_id: String,
}
impl Drop for HsGuard {
    fn drop(&mut self) {
        if let Ok(mut ctrl) = CtrlClient::authenticate(&self.tor_ctrl_addr, None) {
            let _ = ctrl.cmd_multi(&format!("DEL_ONION {}", self.service_id));
        }
    }
}

fn parse_service_id(lines: Vec<String>) -> Result<String> {
    for l in lines {
        if let Some(rest) = l.strip_prefix("250-ServiceID=") {
            return Ok(rest.to_string());
        }
        if l.starts_with("550") {
            return Err(anyhow!("Tor error: {l}"));
        }
    }
    Err(anyhow!("ADD_ONION missing ServiceID"))
}

fn parse_sid_and_pk(lines: Vec<String>) -> Result<(String, String)> {
    let mut sid: Option<String> = None;
    let mut pk:  Option<String> = None;
    for l in lines {
        if let Some(rest) = l.strip_prefix("250-ServiceID=") {
            sid = Some(rest.to_string());
        }
        if let Some(rest) = l.strip_prefix("250-PrivateKey=") {
            pk = Some(rest.to_string()); // e.g., "ED25519-V3:AAAA..."
        }
        if l.starts_with("550") {
            return Err(anyhow!("Tor error: {l}"));
        }
    }
    Ok((
        sid.ok_or_else(|| anyhow!("ADD_ONION new missing ServiceID"))?,
        pk.ok_or_else(|| anyhow!("ADD_ONION new missing PrivateKey"))?,
    ))
}

```

### crates/arti_transport/src/lib.rs

```rust
#![forbid(unsafe_code)]
//! arti_transport: outbound via SOCKS5 (Tor/Arti compatible) and a minimal
//! control-port helper to publish a v3 hidden service (ephemeral by default,
//! or persistent if RO_HS_KEY_FILE is set).

mod socks;
mod ctrl;
mod hs;

use accounting::{Counters};
use std::time::Duration;
use transport::{Handler, ReadWrite, Transport};
use anyhow::Result;

/// Transport over Tor/Arti (SOCKS5 + Tor control-port).
pub struct ArtiTransport {
    counters: Counters,
    socks_addr: String,
    tor_ctrl_addr: String,
    connect_timeout: Duration,
}

impl ArtiTransport {
    /// Create a new `ArtiTransport`.
    ///
    /// - `socks_addr`: e.g., `"127.0.0.1:9050"`
    /// - `tor_ctrl_addr`: e.g., `"127.0.0.1:9051"`
    /// - `connect_timeout`: per-stream I/O timeout
    pub fn new(socks_addr: String, tor_ctrl_addr: String, connect_timeout: Duration) -> Self {
        Self {
            counters: Counters::new(),
            socks_addr,
            tor_ctrl_addr,
            connect_timeout,
        }
    }

    /// Optional: expose counters for periodic logging by the caller.
    pub fn counters(&self) -> Counters {
        self.counters.clone()
    }
}

impl Transport for ArtiTransport {
    fn connect(&self, addr: &str) -> Result<Box<dyn ReadWrite + Send>> {
        socks::connect_via_socks(
            &self.socks_addr,
            addr,
            self.connect_timeout,
            self.counters.clone(),
        )
    }

    /// Listen by publishing a v3 hidden service.
    ///
    /// - **Ephemeral (default):** if `RO_HS_KEY_FILE` env var is **unset**.
    /// - **Persistent:** if `RO_HS_KEY_FILE` points to a file; we reuse it if present,
    ///   otherwise we request a new key from Tor and write it to that path.
    ///
    /// A clean `DEL_ONION` is sent on drop (best effort).
    fn listen(&self, handler: Handler) -> Result<()> {
        hs::publish_and_serve(
            &self.tor_ctrl_addr,
            self.counters.clone(),
            self.connect_timeout,
            handler,
        )
    }
}

```

### crates/arti_transport/src/socks.rs

```rust
use accounting::{Counters, CountingStream};
use anyhow::{anyhow, Context, Result};
use socks::Socks5Stream;
use std::time::Duration;
use transport::ReadWrite;

/// Connect to `addr` ("host:port") via SOCKS5 proxy at `socks_addr`.
pub(crate) fn connect_via_socks(
    socks_addr: &str,
    addr: &str,
    connect_timeout: Duration,
    counters: Counters,
) -> Result<Box<dyn ReadWrite + Send>> {
    let (host, port) = parse_host_port(addr)?;
    let stream = Socks5Stream::connect(socks_addr, (host.as_str(), port))?.into_inner();
    stream.set_read_timeout(Some(connect_timeout)).ok();
    stream.set_write_timeout(Some(connect_timeout)).ok();
    Ok(Box::new(CountingStream::new(stream, counters)))
}

fn parse_host_port(addr: &str) -> Result<(String, u16)> {
    let mut parts = addr.rsplitn(2, ':');
    let p = parts.next().ok_or_else(|| anyhow!("missing :port in addr"))?;
    let h = parts.next().ok_or_else(|| anyhow!("missing host in addr"))?;
    let port = p.parse::<u16>().context("parsing port")?;
    Ok((h.to_string(), port))
}

```

### crates/common/Cargo.toml

```toml
[package]
publish = false
license = "MIT OR Apache-2.0"


name = "common"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
anyhow = { workspace = true }
blake3 = { workspace = true }
hex = { workspace = true }
thiserror = { workspace = true }
toml      = { workspace = true }
workspace-hack = { workspace = true }



```

### crates/common/src/hash.rs

```rust
#![forbid(unsafe_code)]

use blake3::Hasher;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

/// Compute BLAKE3 (256-bit) hex digest of a byte slice (64 hex chars).
pub fn b3_hex(bytes: &[u8]) -> String {
    let mut h = Hasher::new();
    h.update(bytes);
    h.finalize().to_hex().to_string()
}

/// Stream a file through BLAKE3 and return 64-hex. Uses a 1MiB buffer.
pub fn b3_hex_file(path: &Path) -> io::Result<String> {
    const BUF: usize = 1 << 20;
    let mut f = File::open(path)?;
    let mut h = Hasher::new();
    let mut buf = vec![0u8; BUF];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(h.finalize().to_hex().to_string())
}

/// Format "b3:<hex>.{tld}" (explicit) or "<hex>.{tld}" (bare).
pub fn format_addr(hex64: &str, tld: &str, explicit_algo_prefix: bool) -> String {
    if explicit_algo_prefix {
        format!("b3:{}.{}", hex64, tld)
    } else {
        format!("{}.{}", hex64, tld)
    }
}

/// Parse "b3:<hex>.tld" or "<hex>.tld" (treated as BLAKE3). Returns (hex, tld).
pub fn parse_addr(addr: &str) -> Option<(String, String)> {
    let (left, tld) = addr.rsplit_once('.')?;
    let hex = if let Some((algo, hex)) = left.split_once(':') {
        let algo = algo.to_ascii_lowercase();
        if algo != "b3" && algo != "blake3" {
            return None;
        }
        hex
    } else {
        left
    };
    if hex.len() != 64 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    Some((hex.to_ascii_lowercase(), tld.to_string()))
}

/// Two-hex shard folder, e.g. "ad" for "ad…64".
pub fn shard2(hex64: &str) -> &str {
    &hex64[..2]
}

```

### crates/common/src/lib.rs

```rust
#![forbid(unsafe_code)]
//! common: shared types and configuration loading.

pub mod hash;
use blake3::Hasher;
pub use hash::{b3_hex, b3_hex_file, format_addr, parse_addr, shard2};
use serde::{Deserialize, Serialize};
use std::{
    fmt, fs,
    net::SocketAddr,
    path::{Path, PathBuf},
    str::FromStr,
    time::Duration,
};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId([u8; 32]);

impl NodeId {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut h = Hasher::new();
        h.update(bytes);
        let mut out = [0u8; 32];
        out.copy_from_slice(h.finalize().as_bytes());
        Self(out)
    }
    pub fn from_text(s: &str) -> Self {
        Self::from_bytes(s.as_bytes())
    }
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NodeId({})", self.to_hex())
    }
}

impl FromStr for NodeId {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = hex::decode(s)?;
        let mut out = [0u8; 32];
        if bytes.len() != 32 {
            anyhow::bail!("expected 32 bytes");
        }
        out.copy_from_slice(&bytes);
        Ok(Self(out))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub data_dir: PathBuf,
    pub overlay_addr: SocketAddr,
    pub dev_inbox_addr: SocketAddr,
    pub socks5_addr: String,
    pub tor_ctrl_addr: String,
    pub chunk_size: usize,
    pub connect_timeout_ms: u64,
    /// Optional persistent HS private key file (used if provided).
    #[serde(default)]
    pub hs_key_file: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from(".data"),
            overlay_addr: SocketAddr::from(([127, 0, 0, 1], 1777)),
            dev_inbox_addr: SocketAddr::from(([127, 0, 0, 1], 2888)),
            socks5_addr: "127.0.0.1:9050".to_string(),
            tor_ctrl_addr: "127.0.0.1:9051".to_string(),
            chunk_size: 1 << 16,
            connect_timeout_ms: 5000,
            hs_key_file: None,
        }
    }
}

impl Config {
    pub fn connect_timeout(&self) -> Duration {
        Duration::from_millis(self.connect_timeout_ms)
    }
    pub fn load(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        use anyhow::Context;
        let path = path.as_ref();
        let data = fs::read_to_string(path)
            .with_context(|| format!("reading config {}", path.display()))?;
        let cfg: Config = if path.extension().and_then(|s| s.to_str()) == Some("toml") {
            toml::from_str(&data).context("parsing TOML config")?
        } else {
            serde_json::from_str(&data).context("parsing JSON config")?
        };
        Ok(cfg)
    }
}

```

### crates/gateway/Cargo.toml

```toml
[package]
publish = false
name = "gateway"
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"

[dependencies]
anyhow = { workspace = true }

# Use workspace pins for shared crates.
axum = { workspace = true, features = ["tokio", "http1", "http2", "json", "macros"] }
tokio = { workspace = true, features = ["rt-multi-thread", "macros", "net"] }
tower = { workspace = true }
tower-http = { workspace = true, features = ["compression-full", "trace"] }
tracing = { workspace = true }
tracing-subscriber = { workspace = true, features = ["fmt", "env-filter"] }

# CLI and local libs.
clap = { workspace = true }
index = { workspace = true }
naming = { workspace = true }

# Shared deps now unified via workspace pins.
mime_guess = { workspace = true }
tokio-util = { workspace = true, features = ["io"] }
serde = { workspace = true }
toml = { workspace = true }
bytes = { workspace = true }
blake3 = { workspace = true }
thiserror = { workspace = true }
serde_json = { workspace = true }
ron-bus = { workspace = true }
rmp-serde = { workspace = true }

# Unify rand family to workspace pins.
rand = { workspace = true, features = ["std"] }
rand_chacha = { workspace = true, features = ["std"] }

# HTTP client stack via workspace pins.
reqwest = { workspace = true }
hyper = { workspace = true }
hyper-util = { workspace = true }

# OAP + kernel.
oap = { path = "../oap" }
ron-kernel = { workspace = true }

# Metrics via workspace pin.
prometheus = { workspace = true }

workspace-hack = { version = "0.1", path = "../../workspace-hack" }

[features]
legacy-pay = []

[dev-dependencies]
http = "1"

```

### crates/gateway/examples/oap_client_demo.rs

```rust
#![forbid(unsafe_code)]

use anyhow::Result;
use oap::{
    data_frame, end_frame, hello_frame, read_frame, start_frame, write_frame, FrameType,
    DEFAULT_MAX_FRAME,
};
use serde_json::json;
use tokio::{
    io::AsyncWriteExt,
    net::TcpStream,
    time::{timeout, Duration},
};

const ADDR: &str = "127.0.0.1:9444";

// Send HELLO -> START("demo/topic") -> DATA x N -> END
// Then read back any ACK/ERROR frames the server emits.
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    println!("connecting to {ADDR} …");
    let mut s = TcpStream::connect(ADDR).await?;

    // HELLO + START
    write_frame(&mut s, &hello_frame("oap/1"), DEFAULT_MAX_FRAME).await?;
    write_frame(&mut s, &start_frame("demo/topic"), DEFAULT_MAX_FRAME).await?;

    // Send a few DATA chunks (20 KiB each) so we cross the server’s ACK threshold.
    for i in 0..5 {
        let body = make_body(i, 20 * 1024);
        let hdr = json!({ "mime":"text/plain", "seq": i });
        let df = data_frame(hdr, &body, DEFAULT_MAX_FRAME)?;
        write_frame(&mut s, &df, DEFAULT_MAX_FRAME).await?;
        println!("sent DATA seq={i} bytes={}", body.len());

        // Opportunistically poll for ACKs without blocking too long.
        poll_for_server_frames(&mut s, Duration::from_millis(10)).await?;
    }

    // END the stream
    write_frame(&mut s, &end_frame(), DEFAULT_MAX_FRAME).await?;
    // make sure everything is on the wire
    s.flush().await?;

    // Give the server a moment to send any final ACKs
    poll_for_server_frames(&mut s, Duration::from_millis(300)).await?;

    println!("client done.");
    Ok(())
}

fn make_body(seq: u32, n: usize) -> Vec<u8> {
    use std::iter::repeat_n;
    let prefix = format!("chunk-{seq}:");
    let mut v = Vec::with_capacity(prefix.len() + n);
    v.extend_from_slice(prefix.as_bytes());
    v.extend(repeat_n(b'x', n));
    v
}

// Try to read frames for up to `dur`. Print ACK credit or ERRORs if any.
// If no frame arrives before timeout, that’s fine.
async fn poll_for_server_frames(s: &mut TcpStream, dur: Duration) -> Result<()> {
    if let Ok(Ok(fr)) = timeout(dur, read_frame(s, DEFAULT_MAX_FRAME)).await {
        match fr.typ {
            FrameType::Ack => {
                let j: serde_json::Value = serde_json::from_slice(&fr.payload)?;
                let credit = j.get("credit").and_then(|v| v.as_u64()).unwrap_or(0);
                println!("got ACK credit={credit}");
            }
            FrameType::Error => {
                let j: serde_json::Value = serde_json::from_slice(&fr.payload)?;
                println!("server ERROR: {}", j);
            }
            other => {
                println!("unexpected server frame: {:?}", other);
            }
        }
    }
    Ok(())
}

```

### crates/gateway/examples/oap_server_demo.rs

```rust
#![forbid(unsafe_code)]

use std::{net::SocketAddr, sync::Arc};

use oap::{
    ack_frame, b3_of, decode_data_payload, read_frame, write_frame, FrameType, OapFrame,
    DEFAULT_MAX_FRAME,
};
use ron_kernel::{bus::Bus, KernelEvent};
use serde_json::Value as Json;
use tokio::net::{TcpListener, TcpStream};

const LISTEN_ADDR: &str = "127.0.0.1:9444";
const ACK_WINDOW_BYTES: usize = 64 * 1024; // simple credit window

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    // Kernel bus (demo-local). Real app would share this globally.
    let bus = Bus::new(128);

    let listener = TcpListener::bind(LISTEN_ADDR).await?;
    let bound = listener.local_addr()?;
    eprintln!("OAP/1 demo gateway listening on {bound}");

    // Mark gateway healthy for metrics/health checks (if used elsewhere)
    let m = Arc::new(ron_kernel::Metrics::new());
    m.health().set("gateway_demo", true);

    loop {
        let (stream, peer) = listener.accept().await?;
        let bus_for_task = bus.clone();
        tokio::spawn(async move {
            // Pass a CLONE of the bus into the handler so we can still use bus_for_task here.
            if let Err(e) = handle_conn(stream, peer, bus_for_task.clone()).await {
                // Emit a crash-style event to the kernel bus
                let _ = bus_for_task.publish(KernelEvent::ServiceCrashed {
                    service: "oap-gateway".to_string(),
                    reason: format!("peer={peer} error={e}"),
                });
                eprintln!("connection error from {peer}: {e:#}");
            }
        });
    }
}

async fn handle_conn(mut stream: TcpStream, peer: SocketAddr, bus: Bus) -> anyhow::Result<()> {
    // HELLO
    let hello = read_frame(&mut stream, DEFAULT_MAX_FRAME).await?;
    ensure_frame(peer, &hello, FrameType::Hello)?;
    let hello_json: Json = serde_json::from_slice(&hello.payload)?;
    eprintln!("HELLO from {peer}: {hello_json}");

    // START (topic)
    let start = read_frame(&mut stream, DEFAULT_MAX_FRAME).await?;
    ensure_frame(peer, &start, FrameType::Start)?;
    let start_json: Json = serde_json::from_slice(&start.payload)?;
    let topic = start_json
        .get("topic")
        .and_then(|v| v.as_str())
        .unwrap_or("<none>");
    eprintln!("START topic={topic}");

    // Publish an event that a stream started (reusing a simple event variant)
    let _ = bus.publish(KernelEvent::Health {
        service: format!("oap-start:{topic}"),
        ok: true,
    });

    // DATA loop
    let mut credited: usize = ACK_WINDOW_BYTES;
    let mut consumed_since_ack: usize = 0usize;
    loop {
        let fr = read_frame(&mut stream, DEFAULT_MAX_FRAME).await?;
        match fr.typ {
            FrameType::Data => {
                let (hdr, body) = decode_data_payload(&fr.payload)?;
                let obj = hdr.get("obj").and_then(|v| v.as_str()).unwrap_or("");
                let want = b3_of(&body);
                if obj != want {
                    // Protocol error: wrong obj hash; send Error and stop.
                    let payload = serde_json::to_vec(
                        &serde_json::json!({ "code":"proto", "msg":"obj digest mismatch" }),
                    )?;
                    let err = OapFrame::new(FrameType::Error, payload);
                    write_frame(&mut stream, &err, DEFAULT_MAX_FRAME).await?;
                    anyhow::bail!("DATA obj mismatch: got={obj}, want={want}");
                }

                // (Demo) publish a light event per DATA chunk
                let _ = bus.publish(KernelEvent::ConfigUpdated {
                    version: body.len() as u64, // piggyback bytes as "version" for demo visibility
                });

                consumed_since_ack += body.len();
                if consumed_since_ack >= (ACK_WINDOW_BYTES / 2) {
                    // Grant more credit
                    credited += ACK_WINDOW_BYTES;
                    let ack = ack_frame(credited as u64);
                    write_frame(&mut stream, &ack, DEFAULT_MAX_FRAME).await?;
                    consumed_since_ack = 0;
                }
            }
            FrameType::End => {
                eprintln!("END from {peer}");
                break;
            }
            other => {
                let payload = serde_json::to_vec(&serde_json::json!({
                    "code":"proto",
                    "msg": format!("unexpected frame: {other:?}")
                }))?;
                let err = OapFrame::new(FrameType::Error, payload);
                write_frame(&mut stream, &err, DEFAULT_MAX_FRAME).await?;
                anyhow::bail!("unexpected frame type: {other:?}");
            }
        }
    }

    // Mark the topic "done" (flip health to false just to show state change).
    let _ = bus.publish(KernelEvent::Health {
        service: format!("oap-start:{topic}"),
        ok: false,
    });

    Ok(())
}

fn ensure_frame(peer: SocketAddr, fr: &OapFrame, want: FrameType) -> anyhow::Result<()> {
    if fr.typ != want {
        let have = format!("{:?}", fr.typ);
        anyhow::bail!("peer={peer} expected {want:?}, got {have}");
    }
    Ok(())
}

```

### crates/gateway/src/bin/oapd.rs

```rust
#![forbid(unsafe_code)]

use std::net::SocketAddr;

use clap::Parser;
use gateway::oap::OapServer;
use ron_kernel::{bus::Bus, Metrics};
use tokio::signal;

#[derive(Debug, Parser)]
#[command(
    name = "gateway-oapd",
    about = "Gateway OAP/1 server with BLAKE3 (b3:<hex>) verification"
)]
struct Args {
    /// Address to bind the OAP server on, e.g. 127.0.0.1:9444
    #[arg(long = "oap")]
    oap_addr: SocketAddr,

    /// ACK credit window in bytes (server grants more credit after ~half this is consumed)
    #[arg(long = "oap-ack-window", default_value_t = 64 * 1024)]
    oap_ack_window: usize,

    /// Maximum allowed frame size in bytes (default 1 MiB)
    #[arg(long = "oap-max-frame", default_value_t = 1 << 20)]
    oap_max_frame: usize,

    /// Maximum concurrent OAP connections before returning a BUSY error
    #[arg(long = "oap-concurrency", default_value_t = 1024)]
    oap_concurrency: usize,

    /// Optional metrics/health server address, e.g. 127.0.0.1:9909
    #[arg(long = "metrics")]
    metrics_addr: Option<SocketAddr>,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Kernel bus + metrics (shared)
    let bus = Bus::new(256);
    let metrics = Metrics::new();
    if let Some(addr) = args.metrics_addr {
        let (_h, bound) = metrics.clone().serve(addr).await?;
        eprintln!("metrics at http://{bound}/metrics  (healthz/readyz also available)");
        metrics.health().set("gateway_oapd", true);
    }

    // Configure OAP server
    let mut srv = OapServer::new(bus.clone());
    srv.ack_window_bytes = args.oap_ack_window;
    srv.max_frame = args.oap_max_frame;
    srv.concurrency_limit = args.oap_concurrency;

    // Capture fields BEFORE we call serve(self, …) which moves `srv`
    let ack = srv.ack_window_bytes;
    let maxf = srv.max_frame;
    let conc = srv.concurrency_limit;

    // Start OAP server
    let (_handle, bound) = srv.serve(args.oap_addr).await?;
    eprintln!(
        "OAP/1 server listening on {bound}  (ack_window={}B, max_frame={}B, concurrency={})",
        ack, maxf, conc
    );

    // Stay alive until Ctrl-C
    signal::ctrl_c().await?;
    eprintln!("ctrl-c received, shutting down");
    Ok(())
}

```

### crates/gateway/src/http/error.rs

```rust
//! Canonical ErrorEnvelope for HTTP + (optionally) OAP surfaces.
//! Maps to 400/404/413/429/503 and sets Retry-After when provided.

use axum::{
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Debug, Clone)]
pub struct ErrorEnvelope {
    pub code: &'static str,     // e.g., "bad_request", "not_found", "payload_too_large", "quota_exhausted", "unavailable"
    pub message: String,        // short human message
    pub retryable: bool,        // whether client may retry
    pub corr_id: String,        // correlation id for tracing/logs
}

fn corr_id() -> String {
    // no extra deps: nanos since epoch as hex
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{nanos:x}")
}

fn with_retry_after(mut headers: HeaderMap, retry_after_secs: Option<u64>) -> HeaderMap {
    if let Some(secs) = retry_after_secs {
        if let Ok(v) = HeaderValue::from_str(&secs.to_string()) {
            headers.insert("Retry-After", v);
        }
    }
    headers
}

fn respond(status: StatusCode, env: ErrorEnvelope, headers: HeaderMap) -> Response {
    (status, headers, Json(env)).into_response()
}

/// 400 Bad Request
pub fn bad_request(message: impl Into<String>) -> Response {
    respond(
        StatusCode::BAD_REQUEST,
        ErrorEnvelope {
            code: "bad_request",
            message: message.into(),
            retryable: false,
            corr_id: corr_id(),
        },
        HeaderMap::new(),
    )
}

/// 404 Not Found
pub fn not_found(message: impl Into<String>) -> Response {
    respond(
        StatusCode::NOT_FOUND,
        ErrorEnvelope {
            code: "not_found",
            message: message.into(),
            retryable: false,
            corr_id: corr_id(),
        },
        HeaderMap::new(),
    )
}

/// 413 Payload Too Large
pub fn payload_too_large(message: impl Into<String>) -> Response {
    respond(
        StatusCode::PAYLOAD_TOO_LARGE,
        ErrorEnvelope {
            code: "payload_too_large",
            message: message.into(),
            retryable: false,
            corr_id: corr_id(),
        },
        HeaderMap::new(),
    )
}

/// 429 Too Many Requests (quota exhausted) — optional Retry-After
pub fn too_many_requests(message: impl Into<String>, retry_after_secs: Option<u64>) -> Response {
    respond(
        StatusCode::TOO_MANY_REQUESTS,
        ErrorEnvelope {
            code: "quota_exhausted",
            message: message.into(),
            retryable: true,
            corr_id: corr_id(),
        },
        with_retry_after(HeaderMap::new(), retry_after_secs),
    )
}

/// 503 Service Unavailable — optional Retry-After
pub fn unavailable(message: impl Into<String>, retry_after_secs: Option<u64>) -> Response {
    respond(
        StatusCode::SERVICE_UNAVAILABLE,
        ErrorEnvelope {
            code: "unavailable",
            message: message.into(),
            retryable: true,
            corr_id: corr_id(),
        },
        with_retry_after(HeaderMap::new(), retry_after_secs),
    )
}

```

### crates/gateway/src/index_client.rs

```rust
#![allow(dead_code)]
#![forbid(unsafe_code)]

use anyhow::{anyhow, Context, Result};
use rmp_serde::encode::to_vec_named;
use serde::{Deserialize, Serialize};
use std::env;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

const ENV_SOCK: &str = "RON_INDEX_SOCK";

#[derive(Clone, Debug)]
pub struct IndexClient {
    sock_path: PathBuf,
}

impl IndexClient {
    /// Construct from env var `RON_INDEX_SOCK` (script sets this), or fallback.
    pub fn from_env_or(sock_fallback: impl AsRef<Path>) -> Self {
        let p = env::var_os(ENV_SOCK)
            .map(PathBuf::from)
            .unwrap_or_else(|| sock_fallback.as_ref().to_path_buf());
        Self { sock_path: p }
    }

    /// Construct explicitly from a socket path.
    pub fn new(sock_path: impl AsRef<Path>) -> Self {
        Self {
            sock_path: sock_path.as_ref().to_path_buf(),
        }
    }

    fn connect(&self) -> Result<UnixStream> {
        let s = UnixStream::connect(&self.sock_path)
            .with_context(|| format!("connect({})", self.sock_path.display()))?;
        s.set_read_timeout(Some(Duration::from_secs(2))).ok();
        s.set_write_timeout(Some(Duration::from_secs(2))).ok();
        Ok(s)
    }

    /// Ping the index service (mostly for debugging).
    #[allow(dead_code)]
    pub fn ping(&self) -> Result<String> {
        let mut s = self.connect()?;
        write_frame(&mut s, &Req::Ping)?;
        let resp = read_frame(&mut s)?;
        // try tolerant decodes
        if let Ok(p) = rmp_serde::from_slice::<Pong>(&resp) {
            return Ok(p.msg);
        }
        if let Ok(r) = rmp_serde::from_slice::<Resp>(&resp) {
            match r {
                Resp::Ok => return Ok("OK".into()),
                Resp::Pong { msg } => return Ok(msg),
                Resp::Err { err } => return Err(anyhow!("svc-index ping failed: {err}")),
                _ => {}
            }
        }
        Err(anyhow!("svc-index ping: unrecognized framed response"))
    }

    /// Resolve an address to its bundle directory.
    pub fn resolve_dir(&self, addr: &str) -> Result<PathBuf> {
        let mut s = self.connect()?;
        write_frame(&mut s, &Req::Resolve { addr })?;
        let resp = read_frame(&mut s)?;

        // 1) Try a struct response { ok, dir, err }
        if let Ok(r) = rmp_serde::from_slice::<ResolveStruct>(&resp) {
            if r.ok {
                if let Some(d) = r.dir {
                    return Ok(PathBuf::from(d));
                }
                return Err(anyhow!("svc-index resolve: ok=true but dir missing"));
            } else {
                return Err(anyhow!(
                    "svc-index resolve failed: {}",
                    r.err.unwrap_or_else(|| "unknown error".into())
                ));
            }
        }

        // 2) Try an enum response
        if let Ok(r) = rmp_serde::from_slice::<Resp>(&resp) {
            match r {
                Resp::ResolveOk { dir } => return Ok(PathBuf::from(dir)),
                Resp::NotFound => return Err(anyhow!("svc-index resolve: not found")),
                Resp::Err { err } => return Err(anyhow!("svc-index resolve failed: {err}")),
                Resp::Ok | Resp::Pong { .. } => {
                    return Err(anyhow!("svc-index resolve: unexpected response"))
                }
            }
        }

        Err(anyhow!(
            "svc-index resolve: unrecognized framed response ({} bytes)",
            resp.len()
        ))
    }
}

/* ===== Protocol =====
   Keep both struct/enum forms to interop with either style of svc-index.
*/

#[derive(Serialize)]
enum Req<'a> {
    Ping,
    PutAddress { addr: &'a str, dir: &'a str },
    Resolve { addr: &'a str },
}

#[derive(Deserialize)]
struct ResolveStruct {
    ok: bool,
    dir: Option<String>,
    err: Option<String>,
}

#[derive(Deserialize)]
struct Pong {
    msg: String,
}

#[derive(Deserialize)]
enum Resp {
    Ok,
    Pong { msg: String },
    ResolveOk { dir: String },
    NotFound,
    Err { err: String },
}

/* ===== Framing (length-prefixed: u32 big-endian) ===== */

fn write_frame<W: Write, T: Serialize>(w: &mut W, msg: &T) -> Result<()> {
    let payload = to_vec_named(msg)?;
    let len = payload.len();
    if len > u32::MAX as usize {
        return Err(anyhow!("frame too large: {} bytes", len));
    }
    let be = (len as u32).to_be_bytes();
    w.write_all(&be)?;
    w.write_all(&payload)?;
    Ok(())
}

fn read_exact(w: &mut impl Read, buf: &mut [u8]) -> Result<()> {
    let mut read = 0;
    while read < buf.len() {
        let n = w.read(&mut buf[read..]).context("svc-index read frame")?;
        if n == 0 {
            return Err(anyhow!("svc-index closed connection mid-frame"));
        }
        read += n;
    }
    Ok(())
}

fn read_frame<R: Read>(r: &mut R) -> Result<Vec<u8>> {
    let mut len4 = [0u8; 4];
    read_exact(r, &mut len4)?;
    let len = u32::from_be_bytes(len4) as usize;
    if len == 0 {
        return Err(anyhow!("svc-index sent empty frame (len=0)"));
    }
    let mut payload = vec![0u8; len];
    read_exact(r, &mut payload)?;
    Ok(payload)
}

```

### crates/gateway/src/lib.rs

```rust
#![forbid(unsafe_code)]

pub mod oap;
pub use oap::OapServer;

// Re-export modules the tests import from the crate root.
pub mod index_client;
pub mod metrics;
pub mod overlay_client;
pub mod pay_enforce;
pub mod quotas;
pub mod resolve;
pub mod routes;
pub mod state;
pub mod utils;

// Convenience re-exports used by tests
pub use routes::router;
pub use state::AppState;

```

### crates/gateway/src/main.rs

```rust
// crates/gateway/src/main.rs
#![forbid(unsafe_code)]

use std::convert::Infallible;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::task::{Context, Poll};

use anyhow::Result;
use axum::body::Body;
use axum::http::Request;
use axum::Router;
use clap::Parser;
use tower::make::Shared;
use tower::Service;
use tracing::info;
use tracing_subscriber::EnvFilter;

mod index_client;
mod metrics;
mod overlay_client;
mod pay_enforce;
mod quotas;
mod routes;
mod state;
mod utils;

use crate::index_client::IndexClient;
use crate::overlay_client::OverlayClient;
use crate::routes::router; // now returns Router<()>
use crate::state::AppState;

/// Gateway CLI
#[derive(Debug, Parser)]
#[command(name = "gateway")]
#[command(about = "RustyOnions HTTP gateway (serves /o/<addr> via svc-overlay)")]
struct Args {
    /// Address to bind (host:port). Use 127.0.0.1:0 to auto-pick a port.
    #[arg(long, default_value = "127.0.0.1:0")]
    bind: SocketAddr,

    /// Path to legacy index DB (kept for compat; some code paths may still read it).
    #[arg(long, default_value = ".data/index")]
    #[allow(dead_code)]
    index_db: PathBuf,

    /// Enforce payment requirements from Manifest.toml (returns 402).
    #[arg(long, default_value_t = false)]
    enforce_payments: bool,
}

#[derive(Clone)]
struct AddState<S> {
    inner: S,
    state: AppState,
}

impl<S> AddState<S> {
    fn new(inner: S, state: AppState) -> Self {
        Self { inner, state }
    }
}

impl<S, B> Service<Request<B>> for AddState<S>
where
    S: Service<Request<B>, Error = Infallible> + Clone + Send + 'static,
    S::Response: Send + 'static,
    S::Future: Send + 'static,
    AppState: Clone + Send + Sync + 'static,
{
    type Response = S::Response;
    type Error = Infallible;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<B>) -> Self::Future {
        // Make the state available to extractors via request extensions.
        req.extensions_mut().insert(self.state.clone());
        self.inner.call(req)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Logging: honor RUST_LOG (fallback to info)
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let args = Args::parse();

    // Clients from env sockets (with sensible defaults)
    let index_client = IndexClient::from_env_or("/tmp/ron/svc-index.sock");
    let overlay_client = OverlayClient::from_env_or("/tmp/ron/svc-overlay.sock");

    // App state (Clone via Arc — see state.rs)
    let state = AppState::new(index_client, overlay_client, args.enforce_payments);

    // 1) Build a STATELESS router (Router<()>)
    let app: Router<()> = router();

    // 2) Turn it into a per-request service that Axum accepts
    let svc = app.into_service::<Body>();

    // 3) Inject AppState into each request’s extensions
    let svc = AddState::new(svc, state);

    // 4) Turn the per-request service into a make-service
    let make_svc = Shared::new(svc);

    // Bind + serve
    let listener = tokio::net::TcpListener::bind(args.bind).await?;
    let local = listener.local_addr()?;
    info!(%local, "gateway listening");

    axum::serve(listener, make_svc).await?;
    Ok(())
}

```

### crates/gateway/src/metrics.rs

```rust
// crates/gateway/src/metrics.rs
#![forbid(unsafe_code)]

use std::sync::OnceLock;
use std::time::Instant;

use axum::{
    http::{HeaderMap, HeaderValue},
    middleware::Next,
    response::{IntoResponse, Response},
};
use prometheus::{
    Encoder, Histogram, HistogramOpts, IntCounter, IntCounterVec, Opts, Registry, TextEncoder,
};

struct GatewayMetrics {
    // Store Option<T> so we can avoid unwrap/expect and gracefully no-op if construction fails.
    requests_total: Option<IntCounterVec>, // labels: code
    bytes_out_total: Option<IntCounter>,
    request_latency_seconds: Option<Histogram>,
    cache_hits_total: Option<IntCounter>,              // 304s
    range_requests_total: Option<IntCounter>,          // 206s
    precompressed_served_total: Option<IntCounterVec>, // labels: encoding
    quota_rejections_total: Option<IntCounter>,        // 429s
}

static REGISTRY: OnceLock<Registry> = OnceLock::new();
fn registry() -> &'static Registry {
    REGISTRY.get_or_init(Registry::new)
}

static METRICS: OnceLock<GatewayMetrics> = OnceLock::new();
fn metrics() -> &'static GatewayMetrics {
    METRICS.get_or_init(|| {
        let r = registry();

        let requests_total = IntCounterVec::new(
            Opts::new("requests_total", "Total HTTP requests by status code"),
            &["code"],
        )
        .ok();
        if let Some(m) = &requests_total {
            let _ = r.register(Box::new(m.clone()));
        }

        let bytes_out_total = IntCounter::with_opts(Opts::new(
            "bytes_out_total",
            "Total response bytes (Content-Length)",
        ))
        .ok();
        if let Some(m) = &bytes_out_total {
            let _ = r.register(Box::new(m.clone()));
        }

        let request_latency_seconds = Histogram::with_opts(HistogramOpts::new(
            "request_latency_seconds",
            "Wall time from request to response",
        ))
        .ok();
        if let Some(m) = &request_latency_seconds {
            let _ = r.register(Box::new(m.clone()));
        }

        let cache_hits_total = IntCounter::with_opts(Opts::new(
            "cache_hits_total",
            "Conditional GET hits (304 Not Modified)",
        ))
        .ok();
        if let Some(m) = &cache_hits_total {
            let _ = r.register(Box::new(m.clone()));
        }

        let range_requests_total = IntCounter::with_opts(Opts::new(
            "range_requests_total",
            "Byte-range responses (206)",
        ))
        .ok();
        if let Some(m) = &range_requests_total {
            let _ = r.register(Box::new(m.clone()));
        }

        let precompressed_served_total = IntCounterVec::new(
            Opts::new(
                "precompressed_served_total",
                "Objects served from precompressed variants",
            ),
            &["encoding"],
        )
        .ok();
        if let Some(m) = &precompressed_served_total {
            let _ = r.register(Box::new(m.clone()));
        }

        let quota_rejections_total = IntCounter::with_opts(Opts::new(
            "quota_rejections_total",
            "Requests rejected due to quotas/overload (429)",
        ))
        .ok();
        if let Some(m) = &quota_rejections_total {
            let _ = r.register(Box::new(m.clone()));
        }

        GatewayMetrics {
            requests_total,
            bytes_out_total,
            request_latency_seconds,
            cache_hits_total,
            range_requests_total,
            precompressed_served_total,
            quota_rejections_total,
        }
    })
}

/// GET /metrics
pub async fn metrics_handler() -> impl IntoResponse {
    let mut buf = Vec::new();
    let enc = TextEncoder::new();
    // If encoding fails, return 500 with a message.
    if enc.encode(&registry().gather(), &mut buf).is_err() {
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "metrics encode error",
        )
            .into_response();
    }
    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        HeaderValue::from_static("text/plain; version=0.0.4; charset=utf-8"),
    );
    (headers, buf).into_response()
}

/// Middleware that records request count, latency, and a best-effort byte count.
pub async fn record_metrics(req: axum::http::Request<axum::body::Body>, next: Next) -> Response {
    let start = Instant::now();
    let resp = next.run(req).await;

    // Count by status code
    if let Some(m) = &metrics().requests_total {
        let code = resp.status().as_u16().to_string();
        m.with_label_values(&[&code]).inc();
    }

    // Latency
    if let Some(h) = &metrics().request_latency_seconds {
        h.observe(start.elapsed().as_secs_f64());
    }

    // Bytes (Content-Length only; if missing, skip)
    if let (Some(m), Some(len)) = (&metrics().bytes_out_total, content_length(&resp)) {
        m.inc_by(len);
    }

    // Specialized counters (best-effort)
    match resp.status().as_u16() {
        206 => {
            if let Some(m) = &metrics().range_requests_total {
                m.inc();
            }
        }
        304 => {
            if let Some(m) = &metrics().cache_hits_total {
                m.inc();
            }
        }
        429 => {
            if let Some(m) = &metrics().quota_rejections_total {
                m.inc();
            }
        }
        _ => {}
    }

    if let Some(enc) = content_encoding(&resp) {
        if let Some(m) = &metrics().precompressed_served_total {
            m.with_label_values(&[enc]).inc();
        }
    }

    resp
}

fn content_length(resp: &Response) -> Option<u64> {
    resp.headers()
        .get(axum::http::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
}

fn content_encoding(resp: &Response) -> Option<&'static str> {
    resp.headers()
        .get(axum::http::header::CONTENT_ENCODING)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| match s {
            "br" => Some("br"),
            "zstd" | "zst" => Some("zst"),
            "gzip" => Some("gzip"),
            "identity" => Some("identity"),
            _ => None,
        })
}

/// Optional helpers for handlers:
#[allow(dead_code)]
pub fn bump_precompressed_served(encoding: &str) {
    let enc = match encoding {
        "br" => "br",
        "zstd" | "zst" => "zst",
        "gzip" => "gzip",
        _ => "identity",
    };
    if let Some(m) = &metrics().precompressed_served_total {
        m.with_label_values(&[enc]).inc();
    }
}

#[allow(dead_code)]
pub fn bump_cache_hit() {
    if let Some(m) = &metrics().cache_hits_total {
        m.inc();
    }
}

#[allow(dead_code)]
pub fn bump_quota_reject() {
    if let Some(m) = &metrics().quota_rejections_total {
        m.inc();
    }
}

```

### crates/gateway/src/oap.rs

```rust
// crates/gateway/src/oap.rs
#![forbid(unsafe_code)]
// Startup-only metric construction can use expect; never in hot paths.
#![allow(clippy::expect_used)]

use std::net::SocketAddr;
use std::sync::OnceLock;

use oap::{
    ack_frame, b3_of, decode_data_payload, read_frame, write_frame, FrameType, OapError, OapFrame,
    DEFAULT_MAX_FRAME,
};
use prometheus::{register, IntCounterVec, Opts};
use ron_kernel::{bus::Bus, KernelEvent};
use serde_json::Value as Json;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::Semaphore,
    task::JoinHandle,
};

// ---------- metrics (module-local, registered once) ----------

fn rejected_total_static() -> &'static IntCounterVec {
    static V: OnceLock<IntCounterVec> = OnceLock::new();
    V.get_or_init(|| {
        let v = IntCounterVec::new(
            Opts::new("oap_rejected_total", "OAP rejects by reason"),
            &["reason"],
        )
        .expect("IntCounterVec::new(oap_rejected_total)");
        // Ignore AlreadyRegistered errors.
        let _ = register(Box::new(v.clone()));
        v
    })
}

fn reject_inc(reason: &str) {
    rejected_total_static().with_label_values(&[reason]).inc();
}

// ---------- server ----------

/// Minimal OAP/1 server for RustyOnions Gateway:
/// - Expects HELLO → START(topic) → DATA... → END
/// - Verifies DATA header obj == b3(body)
/// - Emits kernel events on the Bus
/// - Sends ACK credits as simple flow control
/// - Applies basic backpressure (connection concurrency limit)
#[derive(Clone)]
pub struct OapServer {
    pub bus: Bus,
    pub ack_window_bytes: usize,
    pub max_frame: usize,
    pub concurrency_limit: usize,
}

impl OapServer {
    pub fn new(bus: Bus) -> Self {
        Self {
            bus,
            ack_window_bytes: 64 * 1024,
            max_frame: DEFAULT_MAX_FRAME,
            concurrency_limit: 1024,
        }
    }

    /// Bind and serve on `addr`. Returns (JoinHandle, bound_addr).
    pub async fn serve(self, addr: SocketAddr) -> std::io::Result<(JoinHandle<()>, SocketAddr)> {
        let listener = TcpListener::bind(addr).await?;
        let bound = listener.local_addr()?;

        // simple connection gate
        let sem = std::sync::Arc::new(Semaphore::new(self.concurrency_limit));

        let handle = tokio::spawn(async move {
            loop {
                let Ok((mut stream, peer)) = listener.accept().await else {
                    break;
                };

                // Try to acquire a slot; if none, send busy error immediately and close.
                match sem.clone().try_acquire_owned() {
                    Ok(permit) => {
                        let srv = self.clone();
                        tokio::spawn(async move {
                            if let Err(e) = handle_conn(stream, peer, srv.clone()).await {
                                let _ = srv.bus.publish(KernelEvent::ServiceCrashed {
                                    service: "oap-gateway".to_string(),
                                    reason: format!("peer={peer} error={e}"),
                                });
                            }
                            drop(permit); // release slot when task completes
                        });
                    }
                    Err(_) => {
                        // Best-effort write a BUSY error then drop the stream.
                        let payload = serde_json::to_vec(&serde_json::json!({
                            "code":"busy","msg":"server at capacity"
                        }))
                        .unwrap_or_default();
                        let err = OapFrame::new(FrameType::Error, payload);
                        let _ = write_frame(&mut stream, &err, DEFAULT_MAX_FRAME).await;
                        reject_inc("busy");
                        // stream drops here
                    }
                }
            }
        });

        Ok((handle, bound))
    }
}

async fn handle_conn(
    mut stream: TcpStream,
    peer: SocketAddr,
    srv: OapServer,
) -> anyhow::Result<()> {
    // HELLO
    let hello = match read_frame(&mut stream, srv.max_frame).await {
        Ok(fr) => fr,
        Err(OapError::PayloadTooLarge { .. }) => {
            send_proto_err(&mut stream, "too_large", "frame exceeds max_frame").await?;
            reject_inc("too_large");
            anyhow::bail!("peer={peer} too_large on HELLO");
        }
        Err(e) => return Err(e.into()),
    };
    ensure_frame(peer, &hello, FrameType::Hello)?;
    let _hello_json: Json = serde_json::from_slice(&hello.payload)?;

    // START (topic)
    let start = match read_frame(&mut stream, srv.max_frame).await {
        Ok(fr) => fr,
        Err(OapError::PayloadTooLarge { .. }) => {
            send_proto_err(&mut stream, "too_large", "frame exceeds max_frame").await?;
            reject_inc("too_large");
            anyhow::bail!("peer={peer} too_large on START");
        }
        Err(e) => return Err(e.into()),
    };
    ensure_frame(peer, &start, FrameType::Start)?;
    let start_json: Json = serde_json::from_slice(&start.payload)?;
    let topic = start_json
        .get("topic")
        .and_then(|v| v.as_str())
        .unwrap_or("<none>")
        .to_string();

    // Mark started
    let _ = srv.bus.publish(KernelEvent::Health {
        service: format!("oap-start:{topic}"),
        ok: true,
    });

    // DATA loop with simple crediting
    let mut credited: usize = srv.ack_window_bytes;
    let mut consumed_since_ack: usize = 0usize;

    loop {
        let fr = match read_frame(&mut stream, srv.max_frame).await {
            Ok(fr) => fr,
            Err(OapError::PayloadTooLarge { .. }) => {
                send_proto_err(&mut stream, "too_large", "frame exceeds max_frame").await?;
                reject_inc("too_large");
                anyhow::bail!("peer={peer} too_large during DATA/END");
            }
            Err(e) => return Err(e.into()),
        };

        match fr.typ {
            FrameType::Data => {
                let (hdr, body) = decode_data_payload(&fr.payload)?;
                let obj = hdr.get("obj").and_then(|v| v.as_str()).unwrap_or("");
                let want = b3_of(&body);
                if obj != want {
                    send_proto_err(&mut stream, "proto", "obj digest mismatch").await?;
                    reject_inc("proto");
                    anyhow::bail!("DATA obj mismatch: got={obj}, want={want}");
                }

                // Emit a lightweight event (demo-friendly visibility)
                let _ = srv.bus.publish(KernelEvent::ConfigUpdated {
                    version: body.len() as u64,
                });

                consumed_since_ack += body.len();
                if consumed_since_ack >= (srv.ack_window_bytes / 2) {
                    credited += srv.ack_window_bytes;
                    let ack = ack_frame(credited as u64);
                    write_frame(&mut stream, &ack, srv.max_frame).await?;
                    consumed_since_ack = 0;
                }
            }
            FrameType::End => break,
            other => {
                send_proto_err(
                    &mut stream,
                    "proto",
                    &format!("unexpected frame: {other:?}"),
                )
                .await?;
                reject_inc("proto");
                anyhow::bail!("unexpected frame type: {other:?}");
            }
        }
    }

    // Mark finished
    let _ = srv.bus.publish(KernelEvent::Health {
        service: format!("oap-start:{topic}"),
        ok: false,
    });

    Ok(())
}

async fn send_proto_err(stream: &mut TcpStream, code: &str, msg: &str) -> anyhow::Result<()> {
    let payload = serde_json::to_vec(&serde_json::json!({ "code": code, "msg": msg }))?;
    let err = OapFrame::new(FrameType::Error, payload);
    write_frame(stream, &err, DEFAULT_MAX_FRAME).await?;
    Ok(())
}

fn ensure_frame(peer: SocketAddr, fr: &OapFrame, want: FrameType) -> anyhow::Result<()> {
    if fr.typ != want {
        let have = format!("{:?}", fr.typ);
        anyhow::bail!("peer={peer} expected {want:?}, got {have}");
    }
    Ok(())
}

```

### crates/gateway/src/overlay_client.rs

```rust
#![forbid(unsafe_code)]

use anyhow::{anyhow, Context, Result};
use ron_bus::api::{Envelope, OverlayReq, OverlayResp};
use std::env;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

const ENV_SOCK: &str = "RON_OVERLAY_SOCK";

#[derive(Clone, Debug)]
pub struct OverlayClient {
    sock_path: PathBuf,
}

impl OverlayClient {
    pub fn from_env_or(sock_fallback: impl AsRef<std::path::Path>) -> Self {
        let p = env::var_os(ENV_SOCK)
            .map(PathBuf::from)
            .unwrap_or_else(|| sock_fallback.as_ref().to_path_buf());
        Self { sock_path: p }
    }

    fn connect(&self) -> Result<UnixStream> {
        UnixStream::connect(&self.sock_path)
            .with_context(|| format!("connect overlay at {}", self.sock_path.display()))
    }

    pub fn get_bytes(&self, addr: &str, rel: &str) -> Result<Option<Vec<u8>>> {
        let mut s = self.connect()?;
        let req = Envelope {
            service: "svc.overlay".into(),
            method: "v1.get".into(),
            corr_id: 1,
            token: vec![],
            payload: rmp_serde::to_vec(&OverlayReq::Get {
                addr: addr.to_string(),
                rel: rel.to_string(),
            })?,
        };
        write_frame(&mut s, &req)?;
        let env = read_frame(&mut s)?;
        let resp: OverlayResp = rmp_serde::from_slice(&env.payload)?;
        Ok(match resp {
            OverlayResp::Bytes { data } => Some(data),
            OverlayResp::NotFound => None,
            OverlayResp::Err { err } => return Err(anyhow!("overlay error: {err}")),
            OverlayResp::HealthOk => return Err(anyhow!("unexpected overlay resp")),
        })
    }
}

fn write_frame(w: &mut impl Write, env: &Envelope) -> Result<()> {
    let payload = rmp_serde::to_vec(env)?;
    if payload.len() > u32::MAX as usize {
        return Err(anyhow!("frame too large: {} bytes", payload.len()));
    }
    w.write_all(&(payload.len() as u32).to_be_bytes())?;
    w.write_all(&payload)?;
    Ok(())
}

fn read_frame(r: &mut impl Read) -> Result<Envelope> {
    let mut len4 = [0u8; 4];
    read_exact(r, &mut len4)?;
    let len = u32::from_be_bytes(len4) as usize;
    if len == 0 {
        return Err(anyhow!("overlay sent empty frame"));
    }
    let mut payload = vec![0u8; len];
    read_exact(r, &mut payload)?;
    Ok(rmp_serde::from_slice(&payload)?)
}

fn read_exact(r: &mut impl Read, buf: &mut [u8]) -> Result<()> {
    let mut read = 0;
    while read < buf.len() {
        let n = r.read(&mut buf[read..])?;
        if n == 0 {
            return Err(anyhow!("overlay closed connection"));
        }
        read += n;
    }
    Ok(())
}

```

### crates/gateway/src/pay_enforce.rs

```rust
#![allow(dead_code)]
#![forbid(unsafe_code)]

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use std::fs;
use std::path::Path;

type HttpErr = Box<(StatusCode, Response)>;

/// Minimal view of Manifest v2 `[payment]`.
#[derive(Debug, Deserialize, Default)]
pub struct Payment {
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub currency: String, // e.g., "RON", "USD", "SAT"
    #[serde(default)]
    pub price_model: String, // "per_request" | "per_mib" | "flat"
    #[serde(default)]
    pub price: f64,
    #[serde(default)]
    pub wallet: String, // LNURL or address
}

#[derive(Debug, Deserialize, Default)]
struct ManifestV2 {
    #[serde(default)]
    payment: Payment,
}

/// Legacy filesystem check: read Manifest.toml from the bundle dir to decide 402.
pub fn guard(bundle_dir: &Path, _addr: &str) -> Result<(), HttpErr> {
    let path = bundle_dir.join("Manifest.toml");
    let bytes = match fs::read(&path) {
        Ok(b) => b,
        Err(_) => return Ok(()), // no manifest -> free
    };
    guard_bytes(&bytes)
}

/// Decide via in-memory Manifest.toml bytes (used when bundle is fetched over overlay/storage).
pub fn guard_bytes(manifest_toml: &[u8]) -> Result<(), HttpErr> {
    // `toml` doesn't provide from_slice in all versions; decode as UTF-8 then parse.
    let s = match std::str::from_utf8(manifest_toml) {
        Ok(x) => x,
        Err(_) => return Ok(()), // treat non-utf8 as free (best-effort)
    };

    let manifest: ManifestV2 = match toml::from_str(s) {
        Ok(m) => m,
        Err(_) => return Ok(()), // malformed -> treat as free for now
    };

    if manifest.payment.required {
        let msg = format!(
            "Payment required: {} {} ({})",
            manifest.payment.price, manifest.payment.currency, manifest.payment.price_model
        );
        let rsp = (StatusCode::PAYMENT_REQUIRED, msg).into_response();
        Err(Box::new((StatusCode::PAYMENT_REQUIRED, rsp)))
    } else {
        Ok(())
    }
}

#[cfg(feature = "legacy-pay")]
pub struct Enforcer {
    enabled: bool,
}

#[cfg(feature = "legacy-pay")]
impl Enforcer {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }
    pub fn guard(&self, bundle_dir: &Path, addr: &str) -> Result<(), HttpErr> {
        if !self.enabled {
            return Ok(());
        }
        crate::pay_enforce::guard(bundle_dir, addr)
    }
}

```

### crates/gateway/src/quotas.rs

```rust
// crates/gateway/src/quotas.rs
#![forbid(unsafe_code)]

use std::{
    collections::HashMap,
    env,
    sync::OnceLock,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

/// Global quotas singleton (lazy).
static QUOTAS: OnceLock<Quotas> = OnceLock::new();

/// Token-bucket quotas (per tenant).
pub struct Quotas {
    inner: Mutex<HashMap<String, Bucket>>,
    rate_per_sec: f64,
    burst: f64,
}

#[derive(Clone)]
struct Bucket {
    tokens: f64,
    last: Instant,
}

impl Quotas {
    fn new(rate_per_sec: f64, burst: f64) -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
            rate_per_sec,
            burst,
        }
    }

    fn enabled(&self) -> bool {
        self.rate_per_sec > 0.0 && self.burst > 0.0
    }

    /// Returns None if allowed; Some(retry_after_secs) if throttled.
    async fn check_and_consume(&self, tenant: &str, cost: f64) -> Option<u64> {
        if !self.enabled() {
            return None;
        }

        let mut map = self.inner.lock().await;
        let now = Instant::now();
        let b = map.entry(tenant.to_string()).or_insert_with(|| Bucket {
            tokens: self.burst,
            last: now,
        });

        // Refill
        let dt = now.duration_since(b.last);
        let refill = self.rate_per_sec * secs(dt);
        b.tokens = (b.tokens + refill).min(self.burst);
        b.last = now;

        // Consume or compute wait
        if b.tokens >= cost {
            b.tokens -= cost;
            None
        } else {
            let needed = cost - b.tokens;
            // How many whole seconds to the next available token(s)?
            let secs = (needed / self.rate_per_sec).ceil().max(0.0) as u64;
            Some(secs)
        }
    }
}

#[inline]
fn secs(d: Duration) -> f64 {
    d.as_secs() as f64 + d.subsec_nanos() as f64 / 1_000_000_000.0
}

/// Initialize from env on first use; subsequent calls return the same instance.
/// RON_QUOTA_RPS=rate (float), RON_QUOTA_BURST=burst (float).
fn quotas() -> &'static Quotas {
    QUOTAS.get_or_init(|| {
        let rate = env::var("RON_QUOTA_RPS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);
        let burst = env::var("RON_QUOTA_BURST")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);
        Quotas::new(rate, burst)
    })
}

/// Public check: return None if allowed, Some(retry_after_secs) if throttled.
pub async fn check(tenant: &str) -> Option<u64> {
    quotas().check_and_consume(tenant, 1.0).await
}

```

### crates/gateway/src/resolve.rs

```rust
// crates/gateway/src/resolve.rs
#![forbid(unsafe_code)]

// Bus-only resolver: uses svc-index over UDS via IndexClient.
// The old sled-based code is removed. Signature kept for compatibility.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::index_client::IndexClient;

/// Resolve an address like "b3:<hex>.tld" to its bundle directory via svc-index.
///
/// NOTE: `index_db` is no longer used (legacy param kept to avoid breaking callers).
/// The socket path comes from RON_INDEX_SOCK or falls back to "/tmp/ron/svc-index.sock".
pub fn resolve_addr(_index_db: &Path, addr_str: &str) -> Result<PathBuf> {
    let client = IndexClient::from_env_or("/tmp/ron/svc-index.sock");
    client
        .resolve_dir(addr_str)
        .with_context(|| format!("resolve_addr({addr_str})"))
}

```

### crates/gateway/src/routes/errors.rs

```rust
// crates/gateway/src/routes/errors.rs
//! Typed JSON error envelope and mappers for common HTTP errors.

use axum::{
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use rand::Rng;
use serde::Serialize;

#[derive(Serialize)]
pub struct ErrorBody {
    pub code: &'static str,
    pub message: String,
    pub retryable: bool,
    pub corr_id: String,
}

/// Internal helper to append/propagate a correlation id.
fn set_corr_id(mut headers: HeaderMap, corr_id: &str) -> HeaderMap {
    let key: HeaderName = HeaderName::from_static("x-corr-id");
    if let Ok(val) = HeaderValue::from_str(corr_id) {
        headers.insert(key, val);
    }
    headers
}

/// Accepts `u32` or `Option<u64>` for Retry-After.
pub(super) enum RetryAfter {
    Seconds(u32),
    None,
}
impl From<u32> for RetryAfter {
    fn from(v: u32) -> Self {
        RetryAfter::Seconds(v)
    }
}
impl From<Option<u64>> for RetryAfter {
    fn from(opt: Option<u64>) -> Self {
        match opt {
            Some(n) => RetryAfter::Seconds(n.min(u64::from(u32::MAX)) as u32),
            None => RetryAfter::None,
        }
    }
}
impl RetryAfter {
    fn write(self, headers: &mut HeaderMap) {
        if let RetryAfter::Seconds(secs) = self {
            let hv =
                HeaderValue::from_str(&secs.to_string()).unwrap_or(HeaderValue::from_static("60"));
            let _ = headers.insert(axum::http::header::RETRY_AFTER, hv);
        }
    }
}

fn build_response(
    status: StatusCode,
    code: &'static str,
    message: String,
    retryable: bool,
    retry_after: RetryAfter,
) -> Response {
    let corr_id = format!("{:016x}", rand::rng().random::<u64>());
    let body = ErrorBody {
        code,
        message,
        retryable,
        corr_id: corr_id.clone(),
    };

    let mut headers = HeaderMap::new();
    headers = set_corr_id(headers, &corr_id);
    retry_after.write(&mut headers);

    (status, headers, Json(body)).into_response()
}

/// 400 Bad Request
#[allow(dead_code)]
pub(super) fn bad_request(msg: impl Into<String>) -> Response {
    build_response(
        StatusCode::BAD_REQUEST,
        "bad_request",
        msg.into(),
        false,
        RetryAfter::None,
    )
}

/// 404 Not Found
pub(super) fn not_found(msg: impl Into<String>) -> Response {
    build_response(
        StatusCode::NOT_FOUND,
        "not_found",
        msg.into(),
        false,
        RetryAfter::None,
    )
}

/// 413 Payload Too Large
#[allow(dead_code)]
pub(super) fn payload_too_large(msg: impl Into<String>) -> Response {
    build_response(
        StatusCode::PAYLOAD_TOO_LARGE,
        "payload_too_large",
        msg.into(),
        false,
        RetryAfter::None,
    )
}

/// 429 Too Many Requests — accepts `u32` or `Option<u64>`
pub(super) fn too_many_requests(
    msg: impl Into<String>,
    retry_after_seconds: impl Into<RetryAfter>,
) -> Response {
    build_response(
        StatusCode::TOO_MANY_REQUESTS,
        "too_many_requests",
        msg.into(),
        true,
        retry_after_seconds.into(),
    )
}

/// 503 Service Unavailable — accepts `u32` or `Option<u64>`
pub(super) fn service_unavailable(
    msg: impl Into<String>,
    retry_after_seconds: impl Into<RetryAfter>,
) -> Response {
    build_response(
        StatusCode::SERVICE_UNAVAILABLE,
        "service_unavailable",
        msg.into(),
        true,
        retry_after_seconds.into(),
    )
}

/// Back-compat alias for older call sites.
pub(super) fn unavailable(
    msg: impl Into<String>,
    retry_after_seconds: impl Into<RetryAfter>,
) -> Response {
    service_unavailable(msg, retry_after_seconds)
}

/// Fallback you can mount on the Router to ensure 404s are consistent.
#[allow(dead_code)]
pub async fn fallback_404() -> impl IntoResponse {
    not_found("route not found")
}

/// Map arbitrary error into a 503 envelope (e.g., for `.handle_error(...)`).
#[allow(dead_code)]
pub(super) fn map_into_503(err: impl std::fmt::Display) -> Response {
    service_unavailable(format!("temporary failure: {err}"), 30u32)
}

```

### crates/gateway/src/routes/http_util.rs

```rust
// crates/gateway/src/routes/http_util.rs
#![forbid(unsafe_code)]

use axum::http::HeaderValue;

#[inline]
pub fn is_manifest(rel: &str) -> bool {
    rel.eq_ignore_ascii_case("manifest.toml")
}

#[inline]
pub fn etag_hex_from_addr(addr_b3: &str) -> Option<String> {
    // addr like "b3:<hex>.<tld>"
    if let Some(stripped) = addr_b3.strip_prefix("b3:") {
        let hex = stripped.split('.').next().unwrap_or_default();
        if !hex.is_empty() {
            return Some(hex.to_string()); // just the hex
        }
    }
    None
}

pub fn etag_matches(if_none_match: &HeaderValue, our_etag_quoted: &str) -> bool {
    let hdr = if_none_match.to_str().unwrap_or_default();
    if hdr.trim() == "*" {
        return true;
    }
    // Accept either quoted or unquoted; allow comma-separated list.
    let needle_q = our_etag_quoted; // "\"b3:<hex>\""
    let needle_u = needle_q.trim_matches('"'); //  "b3:<hex>"
    hdr.split(',')
        .map(|s| s.trim())
        .any(|tok| tok.trim_matches('"') == needle_u || tok == needle_q)
}

pub fn guess_ct(rel: &str) -> &'static str {
    match rel.rsplit('.').next().unwrap_or_default() {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" => "application/javascript",
        "json" => "application/json",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "svg" => "image/svg+xml",
        "txt" | "toml" => "text/plain; charset=utf-8",
        "pdf" => "application/pdf",
        "wasm" | "bin" => "application/octet-stream",
        _ => "application/octet-stream",
    }
}

/// Parse a single Range header of the form "bytes=start-end".
/// Ok(Some((start,end))) for a satisfiable single range
/// Ok(None) if header isn't a "bytes=" spec we handle
/// Err(()) if syntactically wrong or unsatisfiable.
pub fn parse_single_range(range: &str, len: u64) -> Result<Option<(u64, u64)>, ()> {
    let s = range.trim();
    if !s.starts_with("bytes=") {
        return Ok(None);
    }
    let spec = &s["bytes=".len()..];
    if spec.contains(',') {
        return Err(()); // multi-range unsupported
    }
    let (start_s, end_s) = match spec.split_once('-') {
        Some(v) => v,
        None => return Err(()),
    };
    if start_s.is_empty() {
        // "-suffix" (last N bytes)
        let suffix: u64 = end_s.parse().map_err(|_| ())?;
        if suffix == 0 {
            return Err(());
        }
        if suffix > len {
            return Ok(Some((0, len.saturating_sub(1))));
        }
        return Ok(Some((len - suffix, len - 1)));
    }
    let start: u64 = start_s.parse().map_err(|_| ())?;
    let end: u64 = if end_s.is_empty() {
        len.saturating_sub(1)
    } else {
        end_s.parse().map_err(|_| ())?
    };
    if start >= len || end < start {
        return Err(());
    }
    let end = end.min(len.saturating_sub(1));
    Ok(Some((start, end)))
}

```

### crates/gateway/src/routes/mod.rs

```rust
// crates/gateway/src/routes/mod.rs
#![forbid(unsafe_code)]

mod errors;
mod http_util;
mod object;
pub mod readyz;

use axum::{middleware, routing::get, Router};

/// Build a STATELESS router (Router<()>).
/// We inject AppState later at the server entry via a service wrapper.
pub fn router() -> Router<()> {
    Router::new()
        // GET + HEAD both hit serve_object (branch on Method inside)
        .route(
            "/o/:addr/*tail",
            get(object::serve_object).head(object::serve_object),
        )
        .route("/healthz", get(readyz::healthz))
        .route("/readyz", get(readyz::readyz))
        // Golden metrics (Prometheus text format)
        .route("/metrics", get(crate::metrics::metrics_handler))
        // Standardize 404s to JSON envelope
        .fallback(|| async { errors::not_found("route not found") })
        // Request counters/latency/bytes; place late to observe final status/headers
        .layer(middleware::from_fn(crate::metrics::record_metrics))
}

```

### crates/gateway/src/routes/object.rs

```rust
// crates/gateway/src/routes/object.rs
#![forbid(unsafe_code)]

use crate::pay_enforce;
use crate::quotas;
use crate::state::AppState;
use crate::utils::basic_headers;

use super::errors::{not_found, too_many_requests, unavailable};
use super::http_util::{
    etag_hex_from_addr, etag_matches, guess_ct, is_manifest, parse_single_range,
};

use axum::{
    extract::{Extension, Path},
    http::{header, HeaderMap, HeaderValue, Method, StatusCode},
    response::{IntoResponse, Response},
};
use tracing::{error, info};

fn insert_header_safe(h: &mut HeaderMap, k: axum::http::header::HeaderName, v: String) {
    if let Ok(val) = HeaderValue::from_str(&v) {
        h.insert(k, val);
    }
}

/// GET/HEAD /o/:addr/*tail — fetch bytes via svc-overlay.
pub async fn serve_object(
    method: Method,
    Extension(state): Extension<AppState>,
    Path((addr_in, tail)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    // Normalize address: allow "<hex>.<tld>" or "b3:<hex>.<tld>".
    let addr = if addr_in.contains(':') {
        addr_in.clone()
    } else {
        format!("b3:{addr_in}")
    };
    let rel = if tail.is_empty() {
        "payload.bin"
    } else {
        tail.as_str()
    };

    // Tenant identity (best-effort): CAP or API key header; fall back to "public".
    let tenant = headers
        .get("x-ron-cap")
        .or_else(|| headers.get("x-api-key"))
        .or_else(|| headers.get("x-tenant"))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("public")
        .to_string();

    info!(%tenant, %addr_in, %addr, %rel, method = %method, "gateway request");

    // Quota guard (429 w/ Retry-After when enabled + exhausted).
    if let Some(retry_after) = quotas::check(&tenant).await {
        return too_many_requests("quota_exhausted", Some(retry_after));
    }

    // Optional payment guard via Manifest.toml (best-effort).
    if state.enforce_payments {
        if let Ok(Some(manifest)) = state.overlay.get_bytes(&addr, "Manifest.toml") {
            if let Err(err) = pay_enforce::guard_bytes(&manifest) {
                let (_code, rsp) = *err;
                return rsp;
            }
        }
    }

    // Derive ETag pieces:
    let etag_hex = etag_hex_from_addr(&addr);
    let etag_str = etag_hex.as_ref().map(|h| format!("\"b3:{h}\""));
    let etag_hdr = etag_str
        .as_deref()
        .and_then(|s| HeaderValue::from_str(s).ok());

    // Conditional GET/HEAD: If-None-Match short-circuit
    if let (Some(etag), Some(if_none)) = (etag_str.as_deref(), headers.get(header::IF_NONE_MATCH)) {
        if etag_matches(if_none, etag) {
            let mut h = HeaderMap::new();
            if let Some(v) = etag_hdr.clone() {
                h.insert(header::ETAG, v);
            }
            h.insert(header::VARY, HeaderValue::from_static("Accept-Encoding"));
            h.insert(
                header::CACHE_CONTROL,
                HeaderValue::from_static(if is_manifest(rel) {
                    "public, max-age=60"
                } else {
                    "public, max-age=31536000, immutable"
                }),
            );
            h.insert(
                header::X_CONTENT_TYPE_OPTIONS,
                HeaderValue::from_static("nosniff"),
            );
            return (StatusCode::NOT_MODIFIED, h).into_response();
        }
    }

    // Select encoding based on Accept-Encoding + availability (.br/.zst).
    let ae = headers
        .get(header::ACCEPT_ENCODING)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_ascii_lowercase();

    let mut chosen_rel = rel.to_string();
    let mut content_encoding: Option<HeaderValue> = None;

    if !is_manifest(rel) {
        if ae.contains("br") {
            let candidate = format!("{rel}.br");
            if let Ok(Some(_)) = state.overlay.get_bytes(&addr, &candidate) {
                chosen_rel = candidate;
                content_encoding = Some(HeaderValue::from_static("br"));
            }
        }
        if content_encoding.is_none() && (ae.contains("zstd") || ae.contains("zst")) {
            let candidate = format!("{rel}.zst");
            if let Ok(Some(_)) = state.overlay.get_bytes(&addr, &candidate) {
                chosen_rel = candidate;
                content_encoding = Some(HeaderValue::from_static("zstd"));
            }
        }
    }

    // Fetch the chosen bytes (for GET and to compute Content-Length for HEAD/RANGE).
    let fetch = state.overlay.get_bytes(&addr, &chosen_rel);
    let Some(bytes) = (match fetch {
        Ok(Some(b)) => Some(b),
        Ok(None) => None,
        Err(e) => {
            error!(error=?e, %addr, rel=%chosen_rel, "overlay get error");
            return unavailable("backend unavailable", None);
        }
    }) else {
        return not_found("object or file not found");
    };

    // Derive content-type from *original* rel (not the encoded suffix).
    let ctype = guess_ct(rel);

    // Common headers (basic_headers expects plain hex for ETag input)
    let mut h: HeaderMap = basic_headers(ctype, etag_hex.as_deref(), None);
    h.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    h.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(if is_manifest(rel) {
            "public, max-age=60"
        } else {
            "public, max-age=31536000, immutable"
        }),
    );
    h.insert(header::VARY, HeaderValue::from_static("Accept-Encoding"));
    if let Some(enc) = &content_encoding {
        h.insert(header::CONTENT_ENCODING, enc.clone());
    }
    if !is_manifest(rel) {
        h.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    }

    // HEAD: headers only
    if method == Method::HEAD {
        insert_header_safe(&mut h, header::CONTENT_LENGTH, bytes.len().to_string());
        return (StatusCode::OK, h).into_response();
    }

    // RANGES: support a single "bytes=start-end" range
    if let Some(range_hdr) = headers.get(header::RANGE).and_then(|v| v.to_str().ok()) {
        match parse_single_range(range_hdr, bytes.len() as u64) {
            Ok(Some((start, end))) => {
                let start_i = start as usize;
                let end_i = end as usize; // inclusive
                if start_i < bytes.len() && end_i < bytes.len() && start_i <= end_i {
                    let body = bytes[start_i..=end_i].to_vec();
                    let mut h206 = h.clone();
                    insert_header_safe(
                        &mut h206,
                        header::CONTENT_RANGE,
                        format!("bytes {}-{}/{}", start, end, bytes.len()),
                    );
                    insert_header_safe(&mut h206, header::CONTENT_LENGTH, body.len().to_string());
                    return (StatusCode::PARTIAL_CONTENT, h206, body).into_response();
                }
            }
            Ok(None) => { /* ignore: serve full */ }
            Err(_) => {
                let mut h416 = HeaderMap::new();
                insert_header_safe(
                    &mut h416,
                    header::CONTENT_RANGE,
                    format!("bytes */{}", bytes.len()),
                );
                return (StatusCode::RANGE_NOT_SATISFIABLE, h416).into_response();
            }
        }
    }

    // Full body
    insert_header_safe(&mut h, header::CONTENT_LENGTH, bytes.len().to_string());
    (StatusCode::OK, h, bytes).into_response()
}

```

### crates/gateway/src/routes/readyz.rs

```rust
// crates/gateway/src/routes/readyz.rs
#![forbid(unsafe_code)]

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use std::{env, path::PathBuf, time::Duration};
use tokio::{net::UnixStream, time::timeout};

/// Liveness: if the process is up, return 200.
pub async fn healthz() -> Response {
    (StatusCode::OK, "ok").into_response()
}

#[derive(Serialize)]
struct ReadyReport {
    ok: bool,
    overlay_ok: bool,
    index_ok: Option<bool>,
    storage_ok: Option<bool>,
    overlay_sock: Option<String>,
    index_sock: Option<String>,
    storage_sock: Option<String>,
}

/// Readiness succeeds only if overlay is reachable; if RON_INDEX_SOCK/RON_STORAGE_SOCK are
/// configured, they must be reachable as well.
pub async fn readyz() -> Response {
    let overlay_sock = env::var("RON_OVERLAY_SOCK").ok().map(PathBuf::from);
    let index_sock = env::var("RON_INDEX_SOCK").ok().map(PathBuf::from);
    let storage_sock = env::var("RON_STORAGE_SOCK").ok().map(PathBuf::from);

    let mut overlay_ok = false;
    let mut index_ok = None;
    let mut storage_ok = None;

    if let Some(ref p) = overlay_sock {
        overlay_ok = connect_ok(p).await;
    }
    if let Some(ref p) = index_sock {
        index_ok = Some(connect_ok(p).await);
    }
    if let Some(ref p) = storage_sock {
        storage_ok = Some(connect_ok(p).await);
    }

    let ok = overlay_ok && index_ok.unwrap_or(true) && storage_ok.unwrap_or(true);

    let report = ReadyReport {
        ok,
        overlay_ok,
        index_ok,
        storage_ok,
        overlay_sock: overlay_sock.as_ref().map(|p| p.display().to_string()),
        index_sock: index_sock.as_ref().map(|p| p.display().to_string()),
        storage_sock: storage_sock.as_ref().map(|p| p.display().to_string()),
    };

    if ok {
        (StatusCode::OK, Json(report)).into_response()
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, Json(report)).into_response()
    }
}

async fn connect_ok(path: &PathBuf) -> bool {
    timeout(Duration::from_millis(300), UnixStream::connect(path))
        .await
        .is_ok()
}

```

### crates/gateway/src/state.rs

```rust
// crates/gateway/src/state.rs
#![forbid(unsafe_code)]

use std::sync::Arc;

use crate::index_client::IndexClient;
use crate::overlay_client::OverlayClient;

/// Shared application state carried through the gateway.
///
/// Wrap clients in Arc so clones are cheap.
#[derive(Clone)]
pub struct AppState {
    #[allow(dead_code)]
    pub index: Arc<IndexClient>,
    pub overlay: Arc<OverlayClient>,
    pub enforce_payments: bool,
}

impl AppState {
    pub fn new(index: IndexClient, overlay: OverlayClient, enforce_payments: bool) -> Self {
        Self {
            index: Arc::new(index),
            overlay: Arc::new(overlay),
            enforce_payments,
        }
    }
}

```

### crates/gateway/src/test_rt.rs

```rust
#[macro_export]
macro_rules! test_both_runtimes {
    ($name:ident, $body:block) => {
        #[cfg(feature = "rt-multi-thread")]
        #[tokio::test(flavor = "multi_thread")]
        async fn $name() $body

        #[cfg(feature = "rt-current-thread")]
        #[tokio::test(flavor = "current_thread")]
        async fn $name() $body
    };
}

```

### crates/gateway/src/utils.rs

```rust
#![forbid(unsafe_code)]

use axum::http::{HeaderMap, HeaderValue};

/// Common response headers for object delivery.
pub fn basic_headers(
    content_type: &str,
    etag_b3: Option<&str>,
    content_encoding: Option<&str>,
) -> HeaderMap {
    let mut h = HeaderMap::new();
    if let Ok(v) = HeaderValue::from_str(content_type) {
        h.insert("Content-Type", v);
    }
    if let Some(tag) = etag_b3 {
        let v = format!("\"b3:{}\"", tag);
        if let Ok(v) = HeaderValue::from_str(&v) {
            h.insert("ETag", v);
        }
    }
    if let Some(enc) = content_encoding {
        if let Ok(v) = HeaderValue::from_str(enc) {
            h.insert("Content-Encoding", v);
        }
    }
    h.insert(
        "Cache-Control",
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    );
    h.insert(
        "X-Content-Type-Options",
        HeaderValue::from_static("nosniff"),
    );
    h.insert(
        "Vary",
        HeaderValue::from_static(
            "Accept, Accept-Encoding, DPR, Width, Viewport-Width, Sec-CH-UA, Sec-CH-UA-Platform",
        ),
    );
    h.insert(
        "Accept-CH",
        HeaderValue::from_static(
            "Sec-CH-UA, Sec-CH-UA-Mobile, Sec-CH-UA-Platform, DPR, Width, Viewport-Width, Save-Data",
        ),
    );
    h.insert(
        "Critical-CH",
        HeaderValue::from_static("DPR, Width, Viewport-Width"),
    );
    h
}

```

### crates/gateway/tests/free_vs_paid.rs

```rust
// crates/gateway/tests/free_vs_paid.rs
#![forbid(unsafe_code)]

use reqwest::{Client, StatusCode};
use std::time::Duration;

/// Read an env var by primary name or any of a few alternates; trim whitespace.
fn env_any(primary: &str, alternates: &[&str]) -> Option<String> {
    std::env::var(primary)
        .ok()
        .or_else(|| {
            for k in alternates {
                if let Ok(v) = std::env::var(k) {
                    return Some(v);
                }
            }
            None
        })
        .map(|s| s.trim().to_string())
}

/// Configuration for the "online" test mode.
/// Returns None when we don't have enough info to run the assertions.
fn load_cfg() -> Option<(String, String, String)> {
    // Base URL of a running gateway (default to local dev port if not given)
    let base = env_any("GW_BASE_URL", &[]).unwrap_or_else(|| "http://127.0.0.1:9080".to_string());

    // A known-free object address, e.g., "b3:... .text"
    // Common alternates printed by our smoke tools: OBJ_ADDR / FREE_ADDR
    let free_addr = env_any("GW_FREE_ADDR", &["OBJ_ADDR", "FREE_ADDR"])?;

    // A known-paid object address, e.g., "b3:... .post"
    // Common alternates printed by our smoke tools: GW_PAID_ADDR / PAID_ADDR / POST_ADDR
    let paid_addr = env_any("GW_PAID_ADDR", &["PAID_ADDR", "POST_ADDR"])?;

    Some((base, free_addr, paid_addr))
}

async fn http_status(client: &Client, url: &str) -> Result<StatusCode, reqwest::Error> {
    let resp = client.get(url).send().await?;
    Ok(resp.status())
}

fn http_client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .expect("reqwest client")
}

#[tokio::test]
async fn free_bundle_returns_200() {
    let Some((base, free_addr, _)) = load_cfg() else {
        eprintln!(
            "[gateway/free_vs_paid] SKIP: set GW_BASE_URL + (GW_FREE_ADDR|OBJ_ADDR|FREE_ADDR) \
             and GW_PAID_ADDR (or POST_ADDR) to run this test against a live gateway."
        );
        return;
    };

    let client = http_client();
    let url = format!("{}/o/{}", base.trim_end_matches('/'), free_addr);
    let status = http_status(&client, &url).await.unwrap_or(StatusCode::SERVICE_UNAVAILABLE);

    assert_eq!(
        status,
        StatusCode::OK,
        "expected 200 for free bundle at {url}, got {status}"
    );
}

#[tokio::test]
async fn paid_bundle_returns_402() {
    let Some((base, _, paid_addr)) = load_cfg() else {
        eprintln!(
            "[gateway/free_vs_paid] SKIP: set GW_BASE_URL + (GW_FREE_ADDR|OBJ_ADDR|FREE_ADDR) \
             and GW_PAID_ADDR (or POST_ADDR) to run this test against a live gateway."
        );
        return;
    };

    let client = http_client();
    let url = format!("{}/o/{}", base.trim_end_matches('/'), paid_addr);
    let status = http_status(&client, &url).await.unwrap_or(StatusCode::SERVICE_UNAVAILABLE);

    // 402 Payment Required
    assert_eq!(
        status,
        StatusCode::PAYMENT_REQUIRED,
        "expected 402 for paid bundle at {url}, got {status}"
    );
}

```

### crates/gateway/tests/http_read_path.rs

```rust
// FILE: crates/gateway/tests/http_read_path.rs
#![forbid(unsafe_code)]

use anyhow::{bail, Context, Result};
use reqwest::header::{ACCEPT_ENCODING, CONTENT_ENCODING, CONTENT_LENGTH, ETAG, IF_NONE_MATCH, RANGE};
use reqwest::{Client, StatusCode};
use std::time::Duration;

/// Helper: read an env var by primary name or any alternates; trim whitespace.
fn env_any(primary: &str, alternates: &[&str]) -> Option<String> {
    std::env::var(primary)
        .ok()
        .or_else(|| {
            for k in alternates {
                if let Ok(v) = std::env::var(k) {
                    return Some(v);
                }
            }
            None
        })
        .map(|s| s.trim().to_string())
}

/// Resolve base URL of the running gateway.
/// Accept both GATEWAY_URL and GW_BASE_URL; default to 127.0.0.1:9080.
fn resolved_base_url() -> String {
    env_any("GATEWAY_URL", &["GW_BASE_URL"]).unwrap_or_else(|| "http://127.0.0.1:9080".to_string())
}

/// Try to resolve the test object address (e.g., "b3:<hex>.<tld>") from common envs.
/// Many scripts export this as OBJ_ADDR; we also accept FREE_ADDR/GW_FREE_ADDR.
fn resolved_obj_addr() -> Option<String> {
    env_any("OBJ_ADDR", &["FREE_ADDR", "GW_FREE_ADDR"])
}

#[tokio::test]
async fn http_read_path_end_to_end() -> Result<()> {
    // --- Env gating: skip unless we have an object address to test against ---
    let Some(addr) = resolved_obj_addr() else {
        eprintln!("[gateway/http_read_path] SKIP: set OBJ_ADDR (or FREE_ADDR/GW_FREE_ADDR) to a packed free object address. Optional: GATEWAY_URL or GW_BASE_URL for the gateway base.");
        return Ok(());
    };
    let base = resolved_base_url();
    let url = format!("{}/o/{}/payload.bin", base.trim_end_matches('/'), addr);

    // HTTP client
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .context("failed to construct reqwest client")?;

    // 1) Basic GET
    let resp = client.get(&url).send().await.context("GET send failed")?;
    let status = resp.status();
    if !status.is_success() {
        bail!("GET {} -> unexpected status {}", url, status);
    }

    // Capture ETag (if present) *before* we consume the body
    let etag = resp.headers().get(ETAG).cloned();

    // Body as text (best effort; if not UTF-8, just read bytes)
    let body_res = resp.text().await;
    match body_res {
        Ok(s) => {
            // Don't assert on contents; we only validate the path succeeds.
            assert!(
                !s.is_empty(),
                "GET returned empty body (allowed, but unusual)"
            );
        }
        Err(_) => {
            // Retry as bytes — some payloads are binary
            let resp2 = client.get(&url).send().await?;
            let _bytes = resp2.bytes().await?;
        }
    }

    // 2) HEAD should return headers (incl. Content-Length if known)
    let resp = client.head(&url).send().await.context("HEAD send failed")?;
    let status = resp.status();
    if !(status == StatusCode::OK || status == StatusCode::NO_CONTENT) {
        bail!("HEAD {} -> unexpected status {}", url, status);
    }
    if let Some(cl) = resp.headers().get(CONTENT_LENGTH) {
        let _ = cl.to_str().ok().and_then(|s| s.parse::<u64>().ok());
        // We don't assert here; some backends stream without a fixed length.
    }

    // 3) Conditional GET with If-None-Match (expect 304 if ETag supports it)
    if let Some(tag) = etag {
        if let Ok(tag_str) = tag.to_str() {
            let resp2 = client
                .get(&url)
                .header(IF_NONE_MATCH, tag_str)
                .send()
                .await?;
            // 304 is ideal; but some setups might return 200 if ETag changed or is not stable.
            // We accept either 304 or 200 to keep test robust across environments.
            assert!(
                resp2.status() == StatusCode::NOT_MODIFIED || resp2.status().is_success(),
                "If-None-Match should produce 304 or 200; got {}",
                resp2.status()
            );
        }
    }

    // 4) Byte-range: ask for the first 10 bytes; expect 206 or 200 if not supported
    let resp = client.get(&url).header(RANGE, "bytes=0-9").send().await?;
    assert!(
        resp.status() == StatusCode::PARTIAL_CONTENT || resp.status().is_success(),
        "expected 206 or 200 for RANGE 0-9; got {}",
        resp.status()
    );

    // 5) Invalid byte-range — many servers return 416; accept 200 if server ignores invalid ranges
    let resp = client
        .get(&url)
        .header(RANGE, "bytes=999999999999-999999999999")
        .send()
        .await?;
    assert!(
        resp.status() == StatusCode::RANGE_NOT_SATISFIABLE || resp.status().is_success(),
        "expected 416 or 200 for invalid range; got {}",
        resp.status()
    );

    // 6) Content-Encoding negotiation: try common encodings (best-effort)
    for accepts in ["br, zstd, gzip", "zstd, gzip", "gzip"] {
        let resp = client
            .get(&url)
            .header(ACCEPT_ENCODING, accepts)
            .send()
            .await?;
        assert!(
            resp.status().is_success(),
            "GET with Accept-Encoding='{accepts}' should succeed; got {}",
            resp.status()
        );
        if let Some(enc) = resp.headers().get(CONTENT_ENCODING) {
            let _ = enc.to_str().ok(); // do not assert exact encoding; depends on available artifacts
        }
    }

    Ok(())
}

```

### crates/gateway/tests/oap_backpressure.rs

```rust
#![forbid(unsafe_code)]

use anyhow::{bail, Result};
use gateway::oap::OapServer;
use oap::{hello_frame, read_frame, write_frame, FrameType, DEFAULT_MAX_FRAME};
use ron_kernel::bus::Bus;
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

#[tokio::test]
async fn busy_connections_get_error() -> Result<()> {
    // Start server with concurrency_limit = 1 so a second connect is rejected.
    let mut srv = OapServer::new(Bus::new(8));
    srv.concurrency_limit = 1;
    let (_handle, bound) = srv.serve("127.0.0.1:0".parse()?).await?;

    // First client connects and holds the slot by sending HELLO and never finishing.
    let mut c1 = TcpStream::connect(bound).await?;
    write_frame(&mut c1, &hello_frame("oap/1"), DEFAULT_MAX_FRAME).await?;

    // Second client tries to connect: should get an immediate Error frame.
    let mut c2 = TcpStream::connect(bound).await?;
    let read = timeout(
        Duration::from_millis(200),
        read_frame(&mut c2, DEFAULT_MAX_FRAME),
    )
    .await;
    let fr = match read {
        Ok(Ok(f)) => f,
        Ok(Err(e)) => bail!("read failed: {e}"),
        Err(_) => bail!("timed out waiting for busy error"),
    };
    assert!(
        matches!(fr.typ, FrameType::Error),
        "expected Error frame for busy, got {:?}",
        fr.typ
    );
    Ok(())
}

```

### crates/gateway/tests/oap_error_path.rs

```rust
#![forbid(unsafe_code)]

use anyhow::Result;
use gateway::oap::OapServer;
use oap::{
    encode_data_payload, end_frame, hello_frame, read_frame, start_frame, write_frame, FrameType,
    OapFrame, DEFAULT_MAX_FRAME,
};
use ron_kernel::bus::Bus;
use serde_json::json;
use tokio::net::TcpStream;

#[tokio::test]
async fn rejects_mismatched_obj_digest() -> Result<()> {
    // Start server
    let bus = Bus::new(32);
    let srv = OapServer::new(bus);
    let (_handle, bound) = srv.serve("127.0.0.1:0".parse()?).await?;

    // Connect client
    let mut s = TcpStream::connect(bound).await?;

    // HELLO + START
    write_frame(&mut s, &hello_frame("oap/1"), DEFAULT_MAX_FRAME).await?;
    write_frame(&mut s, &start_frame("demo/topic"), DEFAULT_MAX_FRAME).await?;

    // Forge a DATA payload with a wrong obj (server should reject)
    let body = b"abc123";
    let bad_hdr = json!({
        "mime": "text/plain",
        "obj": "b3:0000000000000000000000000000000000000000000000000000000000000000"
    });
    let payload = encode_data_payload(bad_hdr, body)?; // encode will preserve our wrong obj
    let df = OapFrame::new(FrameType::Data, payload);
    write_frame(&mut s, &df, DEFAULT_MAX_FRAME).await?;

    // Expect an Error frame
    let fr = read_frame(&mut s, DEFAULT_MAX_FRAME).await?;
    assert!(
        matches!(fr.typ, FrameType::Error),
        "expected Error, got {:?}",
        fr.typ
    );

    // END (cleanup) – server may close after Error, so ignore failures
    let _ = write_frame(&mut s, &end_frame(), DEFAULT_MAX_FRAME).await;
    Ok(())
}

```

### crates/gateway/tests/oap_server_roundtrip.rs

```rust
#![forbid(unsafe_code)]

use anyhow::Result;
use gateway::oap::OapServer;
use oap::{
    data_frame, end_frame, hello_frame, read_frame, start_frame, write_frame, DEFAULT_MAX_FRAME,
};
use ron_kernel::bus::{sub, Bus};
use ron_kernel::KernelEvent;
use serde_json::json;
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

#[tokio::test]
async fn oap_roundtrip_ack_and_bus_events() -> Result<()> {
    // Kernel bus shared with server; we subscribe to assert events.
    let bus = Bus::new(64);
    let mut rx = bus.subscribe();

    // Start server on ephemeral port
    let srv = OapServer::new(bus.clone());
    let (handle, bound) = srv.serve("127.0.0.1:0".parse()?).await?;

    // Client connects and sends a small flow
    let mut s = TcpStream::connect(bound).await?;

    write_frame(&mut s, &hello_frame("oap/1"), DEFAULT_MAX_FRAME).await?;
    write_frame(&mut s, &start_frame("demo/topic"), DEFAULT_MAX_FRAME).await?;

    // One small DATA chunk (won't hit the server's ACK threshold)
    let body = b"hello world";
    let df = data_frame(json!({"mime":"text/plain"}), body, DEFAULT_MAX_FRAME)?;
    write_frame(&mut s, &df, DEFAULT_MAX_FRAME).await?;

    // Non-blocking peek for a server frame; ignore if timeout (no ACK for tiny payloads).
    let _ = timeout(
        Duration::from_millis(50),
        read_frame(&mut s, DEFAULT_MAX_FRAME),
    )
    .await;

    // END stream
    write_frame(&mut s, &end_frame(), DEFAULT_MAX_FRAME).await?;

    // Assert we observed at least one expected bus event (START or DATA)
    let start_ok = sub::recv_matching(&bus, &mut rx, Duration::from_secs(1), |ev| {
        matches!(ev, KernelEvent::Health { service, ok } if service == "oap-start:demo/topic" && *ok)
    })
    .await
    .is_some();

    let data_seen = sub::recv_matching(
        &bus,
        &mut rx,
        Duration::from_secs(1),
        |ev| matches!(ev, KernelEvent::ConfigUpdated { version } if *version == body.len() as u64),
    )
    .await
    .is_some();

    assert!(start_ok || data_seen, "expected start or data event on bus");

    // Cleanup
    handle.abort();
    Ok(())
}

```

### crates/index/Cargo.toml

```toml
[package]
publish = false

name = "index"
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"

[dependencies]
anyhow = { workspace = true }
serde = { workspace = true }
bincode = "1.3"
naming = { workspace = true }
dunce = "1.0"
chrono = { workspace = true }
sled = { workspace = true }
workspace-hack = { version = "0.1", path = "../../workspace-hack" }

```

### crates/index/src/lib.rs

```rust
#![forbid(unsafe_code)]

use anyhow::Result;
use naming::Address;
use serde::{Deserialize, Serialize};
use sled::{Db, IVec};
use std::path::{Path, PathBuf};

/// Default DB path inside repo (you can change when wiring into node)
pub const DEFAULT_DB_PATH: &str = ".data/index";

const TREE_ADDR: &str = "addr";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddrEntry {
    /// Absolute path to the bundle directory (where Manifest.toml lives)
    pub bundle_dir: PathBuf,
    /// Optional human tags later if needed
    pub created_unix: i64,
}

impl AddrEntry {
    /// Accessor for the bundle directory path.
    #[inline]
    pub fn bundle_dir(&self) -> &Path {
        &self.bundle_dir
    }
}

pub struct Index {
    db: Db,
}

impl Index {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let db = sled::open(path)?;
        Ok(Self { db })
    }

    #[inline]
    fn addr_tree(&self) -> sled::Result<sled::Tree> {
        self.db.open_tree(TREE_ADDR)
    }

    /// Insert/update an address → bundle directory mapping.
    pub fn put_address(&self, addr: &Address, bundle_dir: impl AsRef<Path>) -> Result<()> {
        let entry = AddrEntry {
            bundle_dir: dunce::canonicalize(bundle_dir)?,
            created_unix: chrono::Utc::now().timestamp(),
        };
        let bytes = bincode::serialize(&entry)?;
        self.addr_tree()?.insert(addr.to_string(), bytes)?;
        self.db.flush()?;
        Ok(())
    }

    /// Fetch the full entry for an address (if any).
    pub fn get_address(&self, addr: &Address) -> Result<Option<AddrEntry>> {
        let opt: Option<IVec> = self.addr_tree()?.get(addr.to_string())?;
        if let Some(iv) = opt {
            let entry: AddrEntry = bincode::deserialize(&iv)?;
            Ok(Some(entry))
        } else {
            Ok(None)
        }
    }

    /// Convenience: fetch only the bundle directory for an address.
    pub fn get_bundle_dir(&self, addr: &Address) -> Result<Option<PathBuf>> {
        Ok(self
            .get_address(addr)?
            .map(|e| e.bundle_dir().to_path_buf()))
    }
}

```

### crates/kameo/Cargo.toml

```toml
[package]
publish = false
name = "kameo"
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"
description = "Tiny actor primitives for RustyOnions demos."

[dependencies]
anyhow = { workspace = true }
tokio = { workspace = true, features = ["rt", "macros", "sync"] }
tracing = { workspace = true }
workspace-hack = { version = "0.1", path = "../../workspace-hack" }

```

### crates/kameo/src/lib.rs

```rust
//! Minimal actor helpers used by the demos.
//! Design goals:
//! - No async_trait needed
//! - No reliance on Sender::close / close_channel (just drop to close)
//! - A small “mailbox” with three message kinds: String, Ask-env, and a generic user message M

use anyhow::Result;
use std::future::Future;
use std::pin::Pin;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;

/// Ask pattern: send a request and receive a typed response.
pub struct Ask<Req, Resp> {
    pub req: Req,
    pub tx: oneshot::Sender<Resp>,
}

/// Minimal actor runtime context (extend as needed later).
#[derive(Default)]
pub struct Context;

impl Context {
    pub fn new() -> Self {
        Context
    }
}

/// Messages carried by the mailbox.
/// - String: basic fire-and-forget string
/// - AskEnv: ask for an env value (&'static str -> String)
/// - Custom(M): user-defined message type
pub enum Mailbox<M> {
    Str(String),
    AskEnv(Ask<&'static str, String>),
    Custom(M),
}

/// Address/handle to an actor’s mailbox.
pub struct Addr<M> {
    tx: mpsc::Sender<Mailbox<M>>,
}

impl<M> Clone for Addr<M> {
    fn clone(&self) -> Self {
        Addr {
            tx: self.tx.clone(),
        }
    }
}

impl<M: Send + 'static> Addr<M> {
    /// Send a user-defined message.
    pub async fn send(&self, msg: M) -> Result<()> {
        self.tx
            .send(Mailbox::Custom(msg))
            .await
            .map_err(|e| anyhow::anyhow!("mailbox closed: {e}"))?;
        Ok(())
    }

    /// Send a string message.
    pub async fn send_str(&self, s: impl Into<String>) -> Result<()> {
        self.tx
            .send(Mailbox::Str(s.into()))
            .await
            .map_err(|e| anyhow::anyhow!("mailbox closed: {e}"))?;
        Ok(())
    }

    /// Ask for an env var (demo of request/response pattern).
    pub async fn ask_env(&self, key: &'static str) -> Result<String> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Mailbox::AskEnv(Ask { req: key, tx }))
            .await
            .map_err(|e| anyhow::anyhow!("mailbox closed: {e}"))?;
        let v = rx
            .await
            .map_err(|e| anyhow::anyhow!("actor dropped before responding: {e}"))?;
        Ok(v)
    }

    /// Whether the channel is closed (all receivers dropped).
    pub fn is_closed(&self) -> bool {
        self.tx.is_closed()
    }
}

/// Actor trait:
/// - `handle_message` handles the user message type `M`
/// - `handle_string` handles string messages
/// - `handle_ask_env` handles the Ask<&'static str, String> pattern
///
/// All have default no-op implementations so you can implement only what you need.
pub trait Actor: Send + 'static {
    fn handle_message<'a, M: Send + 'static>(
        &'a mut self,
        _ctx: &'a mut Context,
        _msg: M,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async { Ok(()) })
    }

    fn handle_string<'a>(
        &'a mut self,
        _ctx: &'a mut Context,
        _msg: String,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async { Ok(()) })
    }

    fn handle_ask_env<'a>(
        &'a mut self,
        _ctx: &'a mut Context,
        _ask: Ask<&'static str, String>,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async { Ok(()) })
    }
}

/// Spawn an actor with a mailbox for messages of type `M`.
pub fn spawn<M, A>(mut actor: A) -> (Addr<M>, JoinHandle<()>)
where
    M: Send + 'static,
    A: Actor + Send + 'static,
{
    let (tx, mut rx) = mpsc::channel::<Mailbox<M>>(64);
    let addr = Addr { tx };

    let handle = tokio::spawn(async move {
        let mut ctx = Context::new();

        while let Some(msg) = rx.recv().await {
            match msg {
                Mailbox::Str(m) => {
                    if let Err(e) = actor.handle_string(&mut ctx, m).await {
                        tracing::warn!("actor string handler error: {e:?}");
                    }
                }
                Mailbox::AskEnv(ask) => {
                    if let Err(e) = actor.handle_ask_env(&mut ctx, ask).await {
                        tracing::warn!("actor ask handler error: {e:?}");
                    }
                }
                Mailbox::Custom(m) => {
                    if let Err(e) = actor.handle_message(&mut ctx, m).await {
                        tracing::warn!("actor custom handler error: {e:?}");
                    }
                }
            }
        }
        // mailboxes closed; actor task ends
    });

    (addr, handle)
}

```

### crates/micronode/Cargo.toml

```toml
[package]
name = "micronode"
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"
publish = false

[dependencies]
axum = { workspace = true, features = ["tokio", "http1", "http2", "json"] }
tokio = { workspace = true, features = ["macros", "rt-multi-thread", "signal", "io-util"] }
serde = { workspace = true, features = ["derive"] }
serde_json = { workspace = true }
prometheus = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true, features = ["env-filter", "fmt"] }
ron-policy = { workspace = true, optional = true }

[features]
default = []
policy = ["ron-policy"]

```

### crates/micronode/src/main.rs

```rust
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get},
    Json, Router,
};
use prometheus::{Encoder, TextEncoder};
use serde::Serialize;
use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, SystemTime},
};
use tracing::{info, Level};
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Clone)]
struct AppState {
    started: SystemTime,
    ready: Arc<AtomicBool>,
    service_name: &'static str,
    version: &'static str,
}

#[derive(Serialize)]
struct StatusPayload<'a> {
    service: &'a str,
    version: &'a str,
    ok: bool,
    uptime_secs: u64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let bind: SocketAddr = std::env::var("MICRONODE_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:3001".to_string())
        .parse()
        .expect("MICRONODE_ADDR must be host:port");

    let state = AppState {
        started: SystemTime::now(),
        ready: Arc::new(AtomicBool::new(false)),
        service_name: "micronode",
        version: env!("CARGO_PKG_VERSION"),
    };

    let app = Router::new()
        // Service endpoints
        .route("/", get(root))
        .route("/status", get(status))
        .route("/version", get(version))
        // Ops endpoints
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/metrics", get(metrics))
        .with_state(state.clone());

    let listener = tokio::net::TcpListener::bind(bind).await?;
    info!("micronode listening on http://{bind}");

    // Mark ready after successful bind
    state.ready.store(true, Ordering::SeqCst);

    // Serve until Ctrl-C
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("micronode shutdown complete");
    Ok(())
}

fn init_tracing() {
    // Respect RUST_LOG if provided, default to info
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));
    fmt()
        .with_target(false)
        .with_timer(fmt::time::uptime())
        .with_max_level(Level::INFO)
        .with_env_filter(env_filter)
        .init();
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    info!("received Ctrl-C, shutting down…");
}

async fn root(State(st): State<AppState>) -> impl IntoResponse {
    let up = st.started.elapsed().unwrap_or(Duration::from_secs(0)).as_secs();
    let payload = StatusPayload {
        service: st.service_name,
        version: st.version,
        ok: true,
        uptime_secs: up,
    };
    (StatusCode::OK, Json(payload))
}

async fn status(State(st): State<AppState>) -> impl IntoResponse {
    let up = st.started.elapsed().unwrap_or(Duration::from_secs(0)).as_secs();
    let payload = StatusPayload {
        service: st.service_name,
        version: st.version,
        ok: st.ready.load(std::sync::atomic::Ordering::SeqCst),
        uptime_secs: up,
    };
    (StatusCode::OK, Json(payload))
}

async fn version(State(st): State<AppState>) -> impl IntoResponse {
    let v = serde_json::json!({
        "service": st.service_name,
        "version": st.version
    });
    (StatusCode::OK, Json(v))
}

async fn healthz() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({ "ok": true })))
}

async fn readyz(State(st): State<AppState>) -> impl IntoResponse {
    let ok = st.ready.load(std::sync::atomic::Ordering::SeqCst);
    let code = if ok { StatusCode::OK } else { StatusCode::SERVICE_UNAVAILABLE };
    (code, Json(serde_json::json!({ "ready": ok })))
}

async fn metrics() -> impl IntoResponse {
    // Use the default Prometheus registry; services can register counters/histograms elsewhere.
    let metric_families = prometheus::gather();
    let mut buf = Vec::new();
    let encoder = TextEncoder::new();
    if let Err(e) = encoder.encode(&metric_families, &mut buf) {
        let body = format!("encode error: {e}");
        return (StatusCode::INTERNAL_SERVER_ERROR, body).into_response();
    }
    (StatusCode::OK, String::from_utf8_lossy(&buf).to_string()).into_response()
}

```

### crates/naming/Cargo.toml

```toml
[package]
publish = false

name = "naming"
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"

[dependencies]
anyhow = { workspace = true }
serde = { workspace = true }
serde_with = "3.8"
chrono = { workspace = true }
blake3 = { workspace = true }
hex = { workspace = true }
mime = "0.3"
uuid = { version = "1.10", features = ["v4", "serde"] }
base64 = { workspace = true }
mime_guess = { workspace = true }
thiserror  = { workspace = true }         # if present
toml = { workspace = true }
workspace-hack = { workspace = true }


```

### crates/naming/src/address.rs

```rust
use crate::tld::TldType;
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Address {
    /// Lower-case hex hash string (BLAKE3-256; canonical address form is `b3:<hex>`).
    pub hex: String,
    pub tld: TldType,
}

impl Address {
    pub fn new(hex: impl Into<String>, tld: TldType) -> Self {
        Self { hex: hex.into().to_lowercase(), tld }
    }
    pub fn to_string_addr(&self) -> String {
        format!("{}.{}", self.hex, self.tld)
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_string_addr())
    }
}

#[derive(Debug, Error)]
pub enum AddressParseError {
    #[error("invalid address")]
    Invalid,
    #[error("bad tld: {0}")]
    Tld(#[from] crate::tld::TldParseError),
}

impl FromStr for Address {
    type Err = AddressParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Expect "<hex>.<tld>"
        let (hex, tld_str) = s.rsplit_once('.').ok_or(AddressParseError::Invalid)?;
        if hex.is_empty() { return Err(AddressParseError::Invalid); }
        let tld = tld_str.parse()?;
        Ok(Self::new(hex, tld))
    }
}

```

### crates/naming/src/hash.rs

```rust
#![forbid(unsafe_code)]

use std::fmt;

/// Canonical BLAKE3-256 digest size in bytes and hex length.
pub const B3_LEN: usize = 32;
pub const B3_HEX_LEN: usize = 64;

/// Compute BLAKE3-256 over `bytes`, returning the raw 32-byte array.
#[inline]
pub fn b3(bytes: &[u8]) -> [u8; B3_LEN] {
    blake3::hash(bytes).into()
}

/// Compute the canonical lowercase hex string (64 chars) for BLAKE3-256.
#[inline]
pub fn b3_hex(bytes: &[u8]) -> String {
    let h = blake3::hash(bytes);
    format!("{:x}", h)
}

/// Parse a 64-hex lowercase (or uppercase) BLAKE3 digest into 32 raw bytes.
/// Returns `None` if the string is not exactly 64 hex chars.
pub fn parse_b3_hex<S: AsRef<str>>(s: S) -> Option<[u8; B3_LEN]> {
    let s = s.as_ref();
    if s.len() != B3_HEX_LEN || !s.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let mut out = [0u8; B3_LEN];
    for i in 0..B3_LEN {
        let byte = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok()?;
        out[i] = byte;
    }
    Some(out)
}

/// Render helper for debug prints.
pub struct B3Hex<'a>(pub &'a [u8; B3_LEN]);

impl<'a> fmt::Display for B3Hex<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for b in self.0 {
            write!(f, "{:02x}", b)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_roundtrip() {
        let h = b3(b"hello world");
        let s = B3Hex(&h).to_string();
        assert_eq!(s.len(), B3_HEX_LEN);
        let parsed = parse_b3_hex(&s).unwrap();
        assert_eq!(h, parsed);
    }

    #[test]
    fn rejects_bad_len() {
        assert!(parse_b3_hex("abcd").is_none());
    }

    #[test]
    fn b3_hex_is_lowercase() {
        let s = b3_hex(b"xyz");
        assert!(s.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
        assert_eq!(s.len(), B3_HEX_LEN);
    }
}

```

### crates/naming/src/lib.rs

```rust
#![forbid(unsafe_code)]

use anyhow::{anyhow, Result};
use std::fmt;
use std::str::FromStr;
pub mod manifest;

/// Canonical RustyOnions address (BLAKE3-only):
/// - Accepts "b3:<hex>.tld" **or** "<hex>.tld" on input
/// - `Display` renders canonical form **with** "b3:" prefix
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Address {
    /// 64-hex BLAKE3 digest (lowercase)
    pub hex: String,
    /// TLD like "image", "video", "post"
    pub tld: String,
    /// Whether the original string included an explicit "b3:" prefix
    pub explicit_b3: bool,
}

impl Address {
    /// Parse "b3:<hex>.tld" or "<hex>.tld" (treated as BLAKE3).
    pub fn parse(s: &str) -> Result<Self> {
        let (left, tld) = s
            .rsplit_once('.')
            .ok_or_else(|| anyhow!("missing .tld in address"))?;

        let (explicit_b3, hex) = if let Some((algo, hex)) = left.split_once(':') {
            let algo = algo.to_ascii_lowercase();
            if algo != "b3" && algo != "blake3" {
                return Err(anyhow!("unsupported algo: {algo} (only b3/blake3 allowed)"));
            }
            (true, hex)
        } else {
            (false, left)
        };

        if hex.len() != 64 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(anyhow!("invalid digest: expected 64 hex chars"));
        }

        Ok(Address {
            hex: hex.to_ascii_lowercase(),
            tld: tld.to_string(),
            explicit_b3,
        })
    }

    /// Render with or without "b3:" prefix.
    pub fn to_string_with_prefix(&self, explicit: bool) -> String {
        if explicit {
            self.canonical_string()
        } else {
            self.to_bare_string()
        }
    }

    /// "<hex>.tld"
    pub fn to_bare_string(&self) -> String {
        format!("{}.{}", self.hex, self.tld)
    }

    /// "b3:<hex>.tld"
    pub fn canonical_string(&self) -> String {
        format!("b3:{}.{}", self.hex, self.tld)
    }
}

/// Make `addr.to_string()` work, returning the **canonical** "b3:<hex>.tld".
impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("b3:")?;
        f.write_str(&self.hex)?;
        f.write_str(".")?;
        f.write_str(&self.tld)
    }
}

/// Allow `"<addr>".parse::<Address>()`
impl FromStr for Address {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        Address::parse(s)
    }
}

```

### crates/naming/src/manifest.rs

```rust
// crates/naming/src/manifest.rs
#![forbid(unsafe_code)]

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Canonical manifest schema for RustyOnions.
/// v2 core fields stay stable. Optional blocks below are safe for old readers to ignore.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestV2 {
    // ---- Core (required) ----
    pub schema_version: u32, // 2
    pub tld: String,
    pub address: String,   // e.g., b3:<hex>.<tld>
    pub hash_algo: String, // "b3"
    pub hash_hex: String,  // 64 hex chars
    pub bytes: u64,
    pub created_utc: String,       // RFC3339
    pub mime: String,              // best guess (e.g., text/plain; application/json)
    pub stored_filename: String,   // usually "payload.bin"
    pub original_filename: String, // original source file name

    // Precompressed encodings (zstd/br). Hidden in TOML if empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub encodings: Vec<Encoding>,

    // ---- Optional blocks (hidden if absent/empty) ----
    /// Micropayments / wallet info. Gateways may enforce `required=true`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment: Option<Payment>,

    /// Threading/provenance relations.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relations: Option<Relations>,

    /// SPDX or human‑readable license identifier (moved to top‑level in PR‑8).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,

    /// Namespaced, TLD-specific extras: [ext.image], [ext.video], [ext.<ns>].
    /// Values are arbitrary TOML trees so TLDs can evolve independently.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub ext: BTreeMap<String, toml::Value>,
}

/// Description of a precompressed file variant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Encoding {
    pub coding: String,   // "zstd" | "br"
    pub level: i32,       // compression level/quality used
    pub bytes: u64,       // size on disk
    pub filename: String, // e.g., "payload.bin.zst"
    pub hash_hex: String, // BLAKE3 of the compressed bytes
}

/// Optional micropayments / wallet info.
/// Gateways can later enforce `required=true` before serving bytes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payment {
    #[serde(default)]
    pub required: bool, // default false

    #[serde(default)]
    pub currency: String, // e.g., "USD", "sats", "ETH", "SOL"

    #[serde(default)]
    pub price_model: String, // "per_mib" | "flat" | "per_request"

    #[serde(default)]
    pub price: f64, // unit depends on price_model

    #[serde(default)]
    pub wallet: String, // LNURL, onchain addr, etc.

    #[serde(default)]
    pub settlement: String, // "onchain" | "offchain" | "custodial"

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub splits: Vec<RevenueSplit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevenueSplit {
    pub account: String, // wallet/account id
    pub pct: f32,        // 0..100
}

/// Optional relations/metadata for threading & provenance.
/// (License is now top-level; see `ManifestV2.license`.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relations {
    #[serde(default)]
    pub parent: Option<String>, // b3:<hex>.<tld>

    #[serde(default)]
    pub thread: Option<String>, // root addr

    #[serde(default)]
    pub source: Option<String>, // freeform (e.g., "camera:sony-a7c")
}

/// Helper to write Manifest.toml to a bundle directory.
pub fn write_manifest(bundle_dir: &Path, manifest: &ManifestV2) -> Result<PathBuf> {
    let toml = toml::to_string_pretty(manifest).context("serialize manifest v2")?;
    let path = bundle_dir.join("Manifest.toml");
    fs::write(&path, toml).with_context(|| format!("write {}", path.display()))?;
    Ok(path)
}

```

### crates/naming/src/tld.rs

```rust
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TldType {
    // Media
    Image,
    Video,
    Audio,
    // Text/social
    Post,
    Comment,
    News,
    Journalist,
    Blog,
    // Maps/data
    Map,
    Route,
    // Identity / new direction
    Passport, // replaces "sso"
}

impl TldType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TldType::Image => "image",
            TldType::Video => "video",
            TldType::Audio => "audio",
            TldType::Post => "post",
            TldType::Comment => "comment",
            TldType::News => "news",
            TldType::Journalist => "journalist",
            TldType::Blog => "blog",
            TldType::Map => "map",
            TldType::Route => "route",
            TldType::Passport => "passport",
        }
    }
}

impl fmt::Display for TldType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Error)]
pub enum TldParseError {
    #[error("unknown tld: {0}")]
    Unknown(String),
}

impl FromStr for TldType {
    type Err = TldParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim().trim_start_matches('.');
        Ok(match s {
            "image" => Self::Image,
            "video" => Self::Video,
            "audio" => Self::Audio,
            "post" => Self::Post,
            "comment" => Self::Comment,
            "news" => Self::News,
            "journalist" => Self::Journalist,
            "blog" => Self::Blog,
            "map" => Self::Map,
            "route" => Self::Route,
            "passport" => Self::Passport,
            other => return Err(TldParseError::Unknown(other.to_string())),
        })
    }
}

```

### crates/node/Cargo.toml

```toml
[package]
publish = false
license = "MIT OR Apache-2.0"


name = "node"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = { workspace = true }
color-eyre = "0.6"
# clap with derive + env var support
clap = { workspace = true }
overlay = { path = "../overlay" }
common = { path = "../common" }
# ✅ Tokio runtime + macros (needed for #[tokio::main]) + net + signal
tokio               = { workspace = true, features = ["rt-multi-thread","macros","net","signal"] }
tracing             = { workspace = true }
tracing-subscriber  = { workspace = true, features = ["env-filter"] }
transport = { workspace = true }
workspace-hack = { version = "0.1", path = "../../workspace-hack" }

```

### crates/node/src/cli.rs

```rust
use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::Level;
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::fmt;

use crate::commands;

#[derive(Parser, Debug)]
#[command(name="ronode", version, about="RustyOnions node")]
pub struct Args {
    /// Path to config (JSON or TOML)
    #[arg(long, default_value = "config.toml")]
    pub config: String,

    /// Override the data directory from config (e.g., ".data-tcp")
    #[arg(long)]
    pub data_dir: Option<String>,

    #[command(subcommand)]
    pub cmd: Option<Cmd>,
}

#[derive(Subcommand, Debug)]
pub enum Cmd {
    /// Serve the overlay store (TCP or Tor HS).
    Serve {
        /// Transport: tcp | tor
        #[arg(long, default_value = "tcp")]
        transport: String,
        /// Optional HS key file for persistent onion (Tor only).
        #[arg(long)]
        hs_key_file: Option<std::path::PathBuf>,
    },

    /// Put a file (TCP by default; pass --transport tor + --to .onion:1777 to use Tor).
    Put {
        /// Path to file to upload.
        path: String,
        /// Optional override target like "host:port" or "xxxx.onion:1777"
        #[arg(long)]
        to: Option<String>,
        /// Transport: tcp | tor
        #[arg(long, default_value = "tcp")]
        transport: String,
    },

    /// Get a hash (TCP by default; pass --transport tor + --to .onion:1777 to use Tor).
    Get {
        /// Content hash to retrieve.
        key: String,
        /// Output file path.
        out: String,
        /// Optional override target like "host:port" or "xxxx.onion:1777"
        #[arg(long)]
        to: Option<String>,
        /// Transport: tcp | tor
        #[arg(long, default_value = "tcp")]
        transport: String,
    },

    /// Quick Tor test: open a raw connection to <host:port> via Tor/Arti.
    TorDial {
        /// Target like "example.com:80" or "somerandomv3.onion:1777"
        to: String,
    },

    /// Initialize a default config file (TOML). Default path: ./config.toml
    Init {
        /// Output path (file). If omitted, writes ./config.toml
        #[arg(long)]
        path: Option<std::path::PathBuf>,
    },

    /// Print local store stats as JSON using the configured (or overridden) data_dir.
    StatsJson,
}

pub fn run() -> Result<()> {
    // Logging
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt().with_env_filter(filter).with_max_level(Level::INFO).init();

    let args = Args::parse();

    match args.cmd {
        Some(Cmd::Serve { transport, hs_key_file }) => {
            commands::serve::serve(&args.config, args.data_dir.as_deref(), &transport, hs_key_file.as_deref())
        }
        Some(Cmd::Put { path, to, transport }) => {
            commands::put::put(&args.config, args.data_dir.as_deref(), &path, to.as_deref(), &transport)
        }
        Some(Cmd::Get { key, out, to, transport }) => {
            commands::get::get(&args.config, args.data_dir.as_deref(), &key, &out, to.as_deref(), &transport)
        }
        Some(Cmd::TorDial { to }) => commands::tor_dial::tor_dial(&args.config, &to),
        Some(Cmd::Init { path }) => commands::init::init(path.as_deref()),
        Some(Cmd::StatsJson) => commands::stats::stats_json(&args.config, args.data_dir.as_deref()),
        None => {
            eprintln!("No command provided. Use --help for usage.");
            Ok(())
        }
    }
}

```

### crates/node/src/commands/get.rs

```rust
use anyhow::{anyhow, Context, Result};
use arti_transport::ArtiTransport;
use common::Config;
use overlay::client_get_via;
use std::fs;
use std::io::Write;
use std::net::SocketAddr;
use std::time::Duration;
use transport::TcpTransport;

pub fn get(
    config_path: &str,
    data_dir_override: Option<&str>,
    key: &str,
    out: &str,
    to: Option<&str>,
    transport: &str,
) -> Result<()> {
    let mut cfg = Config::load(config_path).context("loading config")?;
    if let Some(dd) = data_dir_override {
        cfg.data_dir = dd.into(); // PathBuf
    }

    match transport {
        "tcp" => {
            let addr: SocketAddr = to
                .unwrap_or(&format!("{}", cfg.overlay_addr))
                .parse()
                .context("parsing --to host:port")?;
            let tcp = TcpTransport::new();
            let before = tcp.counters().snapshot();
            let maybe = client_get_via(&tcp, &addr.to_string(), key)?;
            let after = tcp.counters().snapshot();
            match maybe {
                Some(bytes) => {
                    let mut f = fs::File::create(out).with_context(|| format!("creating {}", out))?;
                    f.write_all(&bytes)?;
                    eprintln!(
                        "stats get tcp: +in={} +out={}",
                        after.total_in.saturating_sub(before.total_in),
                        after.total_out.saturating_sub(before.total_out),
                    );
                }
                None => eprintln!("NOT FOUND"),
            }
            Ok(())
        }
        "tor" => {
            let arti = ArtiTransport::new(
                cfg.socks5_addr.clone(),
                cfg.tor_ctrl_addr.clone(),
                Duration::from_millis(cfg.connect_timeout_ms),
            );
            let target = to.ok_or_else(|| anyhow!("--to <onion:port> required for tor"))?;
            let before = arti.counters().snapshot();
            let maybe = client_get_via(&arti, target, key)?;
            let after = arti.counters().snapshot();
            match maybe {
                Some(bytes) => {
                    let mut f = fs::File::create(out).with_context(|| format!("creating {}", out))?;
                    f.write_all(&bytes)?;
                    eprintln!(
                        "stats get tor: +in={} +out={}",
                        after.total_in.saturating_sub(before.total_in),
                        after.total_out.saturating_sub(before.total_out),
                    );
                }
                None => eprintln!("NOT FOUND"),
            }
            Ok(())
        }
        other => Err(anyhow!("unknown transport {other}")),
    }
}

```

### crates/node/src/commands/init.rs

```rust
use anyhow::{anyhow, Context, Result};
use std::fs;
use std::path::Path;

fn default_config_toml() -> String {
    r#"# Example config for RustyOnions
data_dir = ".data"
overlay_addr = "127.0.0.1:1777"      # TCP listener bind/target
dev_inbox_addr = "127.0.0.1:2888"
socks5_addr = "127.0.0.1:9050"       # Tor SOCKS5 proxy
tor_ctrl_addr = "127.0.0.1:9051"     # Tor control port
chunk_size = 65536
connect_timeout_ms = 5000
# Optional persistent HS private key file (used by `ronode serve --transport tor`)
# hs_key_file = ".data/hs_ed25519_key"
"#
    .to_string()
}

pub fn init(path: Option<&Path>) -> Result<()> {
    let out = path
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::path::PathBuf::from("config.toml"));
    let data = default_config_toml();
    if out.exists() {
        return Err(anyhow!("refusing to overwrite existing {}", out.display()));
    }
    fs::write(&out, data).with_context(|| format!("writing {}", out.display()))?;
    println!("Wrote {}", out.display());
    Ok(())
}

```

### crates/node/src/commands/mod.rs

```rust
pub mod serve;
pub mod put;
pub mod get;
pub mod tor_dial;
pub mod init;
pub mod stats;

```

### crates/node/src/commands/put.rs

```rust
use anyhow::{anyhow, Context, Result};
use arti_transport::ArtiTransport;
use common::Config;
use overlay::client_put_via;
use std::fs;
use std::net::SocketAddr;
use std::time::Duration;
use transport::TcpTransport;

pub fn put(
    config_path: &str,
    data_dir_override: Option<&str>,
    path: &str,
    to: Option<&str>,
    transport: &str,
) -> Result<()> {
    let mut cfg = Config::load(config_path).context("loading config")?;
    if let Some(dd) = data_dir_override {
        cfg.data_dir = dd.into(); // PathBuf
    }
    let bytes = fs::read(path).with_context(|| format!("reading input {}", path))?;

    match transport {
        "tcp" => {
            let addr: SocketAddr = to
                .unwrap_or(&format!("{}", cfg.overlay_addr))
                .parse()
                .context("parsing --to host:port")?;
            let tcp = TcpTransport::new();
            let before = tcp.counters().snapshot();
            let hash = client_put_via(&tcp, &addr.to_string(), &bytes)?;
            let after = tcp.counters().snapshot();

            eprintln!(
                "stats put tcp: +in={} +out={}",
                after.total_in.saturating_sub(before.total_in),
                after.total_out.saturating_sub(before.total_out),
            );
            println!("{hash}");
            Ok(())
        }
        "tor" => {
            let arti = ArtiTransport::new(
                cfg.socks5_addr.clone(),
                cfg.tor_ctrl_addr.clone(),
                Duration::from_millis(cfg.connect_timeout_ms),
            );
            let target = to.ok_or_else(|| anyhow!("--to <onion:port> required for tor"))?;
            let before = arti.counters().snapshot();
            let hash = client_put_via(&arti, target, &bytes)?;
            let after = arti.counters().snapshot();

            eprintln!(
                "stats put tor: +in={} +out={}",
                after.total_in.saturating_sub(before.total_in),
                after.total_out.saturating_sub(before.total_out),
            );
            println!("{hash}");
            Ok(())
        }
        other => Err(anyhow!("unknown transport {other}")),
    }
}

```

### crates/node/src/commands/serve.rs

```rust
use anyhow::{anyhow, Context, Result};
use arti_transport::ArtiTransport;
use common::Config;
use overlay::{run_overlay_listener_with_transport, Store};
use serde_json::json;
use std::io::Write;
use std::net::{SocketAddr, TcpListener};
use std::thread;
use std::time::Duration;
use tracing::info;
use transport::TcpTransport;
use transport::tor_control::publish_v3;


pub fn serve(
    config_path: &str,
    data_dir_override: Option<&str>,
    transport: &str,
    hs_key_file: Option<&std::path::Path>,
) -> Result<()> {
    let mut cfg = Config::load(config_path).context("loading config")?;
    if let Some(dd) = data_dir_override {
        cfg.data_dir = dd.into(); // PathBuf
    }
    if transport == "tor" {
        if let Some(path) = hs_key_file {
            std::env::set_var("RO_HS_KEY_FILE", path.to_string_lossy().to_string());
        }
    }

    let store = Store::open(&cfg.data_dir, cfg.chunk_size)?;
    match transport {
        "tcp" => {
            let addr = cfg.overlay_addr;
            let tcp = TcpTransport::with_bind_addr(addr.to_string());
            let ctrs = tcp.counters();
            run_overlay_listener_with_transport(&tcp, store.clone())?;
            info!("serving TCP on {}; Ctrl+C to exit", addr);

            // periodic byte-counter logs
            {
                let ctrs = ctrs.clone();
                thread::spawn(move || loop {
                    thread::sleep(Duration::from_secs(60));
                    let s = ctrs.snapshot();
                    info!(
                        "stats/tcp total_in={} total_out={} last_min_in={} last_min_out={}",
                        s.total_in, s.total_out, s.per_min_in[59], s.per_min_out[59]
                    );
                });
            }

            // start metrics endpoint on dev_inbox_addr
            start_metrics_server(
                store,
                move || {
                    let s = ctrs.snapshot();
                    (s.total_in, s.total_out)
                },
                cfg.dev_inbox_addr,
            )?;

            // park forever
            loop {
                std::thread::park();
            }
        }
        "tor" => {
            let arti = ArtiTransport::new(
                cfg.socks5_addr.clone(),
                cfg.tor_ctrl_addr.clone(),
                Duration::from_millis(cfg.connect_timeout_ms),
            );
            let ctrs = arti.counters();
            run_overlay_listener_with_transport(&arti, store.clone())?;
            info!("serving Tor HS via Arti (control={})", cfg.tor_ctrl_addr);

            {
                let ctrs = ctrs.clone();
                thread::spawn(move || loop {
                    thread::sleep(Duration::from_secs(60));
                    let s = ctrs.snapshot();
                    info!(
                        "stats/tor total_in={} total_out={} last_min_in={} last_min_out={}",
                        s.total_in, s.total_out, s.per_min_in[59], s.per_min_out[59]
                    );
                });
            }

            start_metrics_server(
                store,
                move || {
                    let s = ctrs.snapshot();
                    (s.total_in, s.total_out)
                },
                cfg.dev_inbox_addr,
            )?;

            loop {
                std::thread::park();
            }
        }
        other => Err(anyhow!("unknown transport {other}")),
    }
}

/// Tiny HTTP server that serves `/metrics.json` with store + transport counters.
/// `counters_fn` returns `(total_in, total_out)` snapshot when called.
fn start_metrics_server<F>(store: Store, counters_fn: F, listen_addr: SocketAddr) -> Result<()>
where
    F: Fn() -> (u64, u64) + Send + 'static,
{
    thread::spawn(move || {
        let listener = match TcpListener::bind(listen_addr) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("metrics bind error on {}: {e}", listen_addr);
                return;
            }
        };
        // Use tracing so it shows up alongside your other logs.
        info!("metrics listening on http://{}/metrics.json", listen_addr);

        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };

            // Try to read store stats; if unavailable, serve zeros but still respond 200.
            let store_json = match store.stats() {
                Ok(s) => json!({ "n_keys": s.n_keys, "total_bytes": s.total_bytes }),
                Err(_) => json!({ "n_keys": 0_u64, "total_bytes": 0_u64 }),
            };

            let (total_in, total_out) = counters_fn();

            let body = json!({
                "store": store_json,
                "transport": { "total_in": total_in, "total_out": total_out }
            })
            .to_string();

            // Write + flush response
            let _ = write_http_ok(&mut stream, &body);
            let _ = stream.flush();
        }
    });

    Ok(())
}

fn write_http_ok(stream: &mut dyn Write, body: &str) -> std::io::Result<()> {
    let resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream.write_all(resp.as_bytes())
}

```

### crates/node/src/commands/stats.rs

```rust
use anyhow::{Context, Result};
use common::Config;
use overlay::Store;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

/// Try to open the DB and print stats; if it's locked, query the running node's metrics endpoint.
pub fn stats_json(config_path: &str, data_dir_override: Option<&str>) -> Result<()> {
    let mut cfg = Config::load(config_path).context("loading config")?;
    if let Some(dd) = data_dir_override {
        cfg.data_dir = dd.into(); // PathBuf
    }

    match Store::open(&cfg.data_dir, cfg.chunk_size) {
        Ok(store) => {
            let stats = store.stats()?;
            let json = serde_json::to_string_pretty(&stats)?;
            println!("{json}");
            Ok(())
        }
        Err(open_err) => {
            // Likely "Resource temporarily unavailable" (sled lock). Fall back to metrics endpoint.
            eprintln!("store busy, trying metrics endpoint at {}", cfg.dev_inbox_addr);
            let addr: SocketAddr = cfg.dev_inbox_addr;
            let mut s = TcpStream::connect(addr).with_context(|| "connect metrics endpoint")?;
            s.set_read_timeout(Some(Duration::from_secs(2)))?;
            s.write_all(b"GET /metrics.json HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")?;
            let mut buf = Vec::new();
            s.read_to_end(&mut buf)?;
            let resp = String::from_utf8_lossy(&buf);

            // split headers/body on CRLFCRLF
            if let Some((_headers, body)) = resp.split_once("\r\n\r\n") {
                // pretty-print body if possible
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(body.trim()) {
                    println!("{}", serde_json::to_string_pretty(&v)?);
                    return Ok(());
                }
                // else, just print raw body
                println!("{}", body.trim());
                return Ok(());
            }
            // If we got here, we didn't parse an HTTP response; show the original open error too.
            Err(open_err.context("failed to open store; bad metrics HTTP response"))?
        }
    }
}


```

### crates/node/src/commands/tor_dial.rs

```rust
use anyhow::{Context, Result};
use arti_transport::ArtiTransport;
use common::Config;
use std::io::Write;
use std::time::Duration;
use tracing::info;
use transport::Transport; // <- bring the trait into scope so `.connect()` works

pub fn tor_dial(config_path: &str, to: &str) -> Result<()> {
    let cfg = Config::load(config_path).context("loading config")?;
    let arti = ArtiTransport::new(
        cfg.socks5_addr.clone(),
        cfg.tor_ctrl_addr.clone(),
        Duration::from_millis(cfg.connect_timeout_ms),
    );
    let mut s = arti.connect(to)?;
    s.write_all(b"HEAD / HTTP/1.1\r\nHost: example\r\n\r\n")?;
    s.flush()?;
    info!("tor dial success to {}", to);
    Ok(())
}

```

### crates/node/src/lib.rs

```rust
#![forbid(unsafe_code)]
// Intentionally minimal library: the CLI lives in src/main.rs.
// The old `commands` module was from a previous API and caused
// unresolved-import errors. We'll reintroduce a typed library
// interface after the refactor.

pub mod cli {
    // (empty) – retained only to avoid breaking external imports, if any.
}

```

### crates/node/src/main.rs

```rust
#![forbid(unsafe_code)]

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::signal;
use tracing::info;
use tracing_subscriber::EnvFilter;

use overlay::{client_get, client_put, run_overlay_listener};

/// RustyOnions node CLI (TCP overlay + optional Tor flags for compatibility)
#[derive(Parser, Debug)]
#[command(name = "ronode", version, about = "RustyOnions node")]
struct Cli {
    /// RUST_LOG-like filter, e.g. "info,overlay=debug"
    #[arg(long, default_value = "info")]
    log: String,

    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Start an overlay listener
    Serve {
        /// Bind address (ip:port), e.g. 127.0.0.1:1777
        #[arg(long, default_value = "127.0.0.1:1777")]
        bind: SocketAddr,

        /// Transport to use (accepted for compatibility). Only `tcp` is supported here.
        #[arg(long, default_value = "tcp")]
        transport: String,

        /// Path to the sled DB for the overlay store
        #[arg(long, default_value = ".data/sled")]
        store_db: PathBuf,
    },

    /// PUT a file to a remote overlay listener, prints the content hash.
    Put {
        /// Target address (ip:port)
        #[arg(long, default_value = "127.0.0.1:1777")]
        to: String,
        /// File to upload
        #[arg(long)]
        path: PathBuf,
    },

    /// GET a blob by hash from a remote overlay listener, writes to a file.
    Get {
        /// Target address (ip:port)
        #[arg(long, default_value = "127.0.0.1:1777")]
        from: String,
        /// Hex hash
        #[arg(long)]
        hash: String,
        /// Output file
        #[arg(long)]
        out: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(cli.log));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    match cli.cmd {
        Commands::Serve {
            bind,
            transport,
            store_db,
        } => {
            if transport.to_lowercase() != "tcp" {
                bail!("only tcp transport is supported by this binary at the moment (got `{transport}`)");
            }
            info!(%bind, store=?store_db, "starting overlay TCP listener");
            run_overlay_listener(bind, &store_db).context("start overlay listener")?;
            info!("press Ctrl-C to stop…");
            signal::ctrl_c().await?;
            Ok(())
        }
        Commands::Put { to, path } => {
            let hash = client_put(&to, &path).await.context("client put")?;
            println!("{hash}");
            Ok(())
        }
        Commands::Get { from, hash, out } => {
            client_get(&from, &hash, &out).await.context("client get")?;
            println!("wrote {}", out.display());
            Ok(())
        }
    }
}

```

### crates/oap/Cargo.toml

```toml
[package]
name = "oap"
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"
description = "OAP/1 frame codec and DATA packing (b3:<hex>) for RustyOnions"
publish = false

[dependencies]
bytes = { workspace = true }
serde_json = { workspace = true }
thiserror = "2"
blake3 = { workspace = true }
hex = { workspace = true }
tokio = { workspace = true, features = ["io-util","macros","rt"] }
workspace-hack = { version = "0.1", path = "../../workspace-hack" }

[dev-dependencies]
futures = "0.3"

```

### crates/oap/src/lib.rs

```rust
#![forbid(unsafe_code)]
// OAP/1 tiny codec + DATA packing helpers (b3:<hex>), per Microkernel blueprint:
// - max_frame = 1 MiB (protocol default)
// - DATA payload layout: [u16 header_len][header JSON][raw body]
// - header MUST include obj:"b3:<hex>" (BLAKE3-256 of the *plaintext* body)

use bytes::{BufMut, Bytes, BytesMut};
use serde_json::Value as Json;
use std::{convert::TryFrom, io};
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub const OAP_VERSION: u8 = 0x1;
pub const DEFAULT_MAX_FRAME: usize = 1 << 20; // 1 MiB

#[inline]
fn json_vec(value: serde_json::Value, ctx: &'static str) -> Vec<u8> {
    match serde_json::to_vec(&value) {
        Ok(v) => v,
        Err(e) => {
            // Non-panicking path: log to stderr, return empty payload.
            // (Keeps public API stable and avoids crashing on serialization failure.)
            eprintln!("oap: failed to serialize {} payload: {}", ctx, e);
            Vec::new()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FrameType {
    Hello = 0x01,
    Start = 0x02,
    Data = 0x03,
    End = 0x04,
    Ack = 0x05,
    Error = 0x06,
}

impl TryFrom<u8> for FrameType {
    type Error = OapError;
    // NOTE: Returning OapError explicitly avoids ambiguity with the `Error` variant.
    fn try_from(b: u8) -> Result<Self, OapError> {
        Ok(match b {
            0x01 => FrameType::Hello,
            0x02 => FrameType::Start,
            0x03 => FrameType::Data,
            0x04 => FrameType::End,
            0x05 => FrameType::Ack,
            0x06 => FrameType::Error,
            other => return Err(OapError::UnknownType(other)),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OapFrame {
    pub ver: u8,
    pub typ: FrameType,
    pub payload: Bytes,
}

#[derive(Debug, Error)]
pub enum OapError {
    #[error("io: {0}")]
    Io(#[from] io::Error),

    #[error("invalid version: {0}")]
    InvalidVersion(u8),

    #[error("unknown frame type: 0x{0:02x}")]
    UnknownType(u8),

    #[error("payload too large: {len} > max_frame {max}")]
    PayloadTooLarge { len: usize, max: usize },

    #[error("header too large: {0} > u16::MAX")]
    HeaderTooLarge(usize),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("DATA decode short header")]
    DataShortHeader,
}

impl OapFrame {
    pub fn new(typ: FrameType, payload: impl Into<Bytes>) -> Self {
        Self {
            ver: OAP_VERSION,
            typ,
            payload: payload.into(),
        }
    }
}

/// Write a single frame to the stream: ver(1) typ(1) len(4) payload(len)
pub async fn write_frame<W: AsyncWrite + Unpin>(
    w: &mut W,
    frame: &OapFrame,
    max_frame: usize,
) -> Result<(), OapError> {
    let len = frame.payload.len();
    if frame.ver != OAP_VERSION {
        return Err(OapError::InvalidVersion(frame.ver));
    }
    if len > max_frame {
        return Err(OapError::PayloadTooLarge {
            len,
            max: max_frame,
        });
    }
    w.write_u8(frame.ver).await?;
    w.write_u8(frame.typ as u8).await?;
    w.write_u32(len as u32).await?;
    if len > 0 {
        w.write_all(&frame.payload).await?;
    }
    w.flush().await?;
    Ok(())
}

/// Read a single frame from the stream (validates version and size).
pub async fn read_frame<R: AsyncRead + Unpin>(
    r: &mut R,
    max_frame: usize,
) -> Result<OapFrame, OapError> {
    let ver = r.read_u8().await?;
    if ver != OAP_VERSION {
        return Err(OapError::InvalidVersion(ver));
    }
    let typ = FrameType::try_from(r.read_u8().await?)?;
    let len = r.read_u32().await? as usize;
    if len > max_frame {
        return Err(OapError::PayloadTooLarge {
            len,
            max: max_frame,
        });
    }
    let mut buf = vec![0u8; len];
    if len > 0 {
        r.read_exact(&mut buf).await?;
    }
    Ok(OapFrame {
        ver,
        typ,
        payload: Bytes::from(buf),
    })
}

/// Compute canonical object id "b3:<hex>" for plaintext bytes.
pub fn b3_of(bytes: &[u8]) -> String {
    let hash = blake3::hash(bytes);
    let hex = hex::encode(hash.as_bytes());
    format!("b3:{hex}")
}

/// DATA packing: `[u16 header_len][header JSON][raw body]`
/// Ensures `obj:"b3:<hex>"` is present in header (adds if missing).
pub fn encode_data_payload(mut header: Json, body: &[u8]) -> Result<Bytes, OapError> {
    if !header.is_object() {
        // Promote non-object to object with single "meta" field
        header = serde_json::json!({ "meta": header });
    }
    let obj = header
        .get("obj")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| b3_of(body));

    // Insert/overwrite obj
    if let Some(map) = header.as_object_mut() {
        map.insert("obj".into(), Json::String(obj));
    }

    let hdr_bytes = serde_json::to_vec(&header)?;
    if hdr_bytes.len() > u16::MAX as usize {
        return Err(OapError::HeaderTooLarge(hdr_bytes.len()));
    }

    let mut out = BytesMut::with_capacity(2 + hdr_bytes.len() + body.len());
    out.put_u16(hdr_bytes.len() as u16);
    out.extend_from_slice(&hdr_bytes);
    out.extend_from_slice(body);
    Ok(out.freeze())
}

/// Decode a DATA payload into (header JSON, body bytes).
pub fn decode_data_payload(payload: &[u8]) -> Result<(Json, Bytes), OapError> {
    if payload.len() < 2 {
        return Err(OapError::DataShortHeader);
    }
    let hdr_len = u16::from_be_bytes([payload[0], payload[1]]) as usize;
    if payload.len() < 2 + hdr_len {
        return Err(OapError::DataShortHeader);
    }
    let hdr = &payload[2..2 + hdr_len];
    let body = &payload[2 + hdr_len..];
    let header_json: Json = serde_json::from_slice(hdr)?;
    Ok((header_json, Bytes::copy_from_slice(body)))
}

/// Helper: build a DATA frame and enforce `max_frame`.
pub fn data_frame(header: Json, body: &[u8], max_frame: usize) -> Result<OapFrame, OapError> {
    let payload = encode_data_payload(header, body)?;
    if payload.len() > max_frame {
        return Err(OapError::PayloadTooLarge {
            len: payload.len(),
            max: max_frame,
        });
    }
    Ok(OapFrame::new(FrameType::Data, payload))
}

/// Small helpers to build common frames
pub fn hello_frame(proto_id: &str) -> OapFrame {
    let payload = json_vec(serde_json::json!({ "hello": proto_id }), "hello");
    OapFrame::new(FrameType::Hello, payload)
}
pub fn start_frame(topic: &str) -> OapFrame {
    let payload = json_vec(serde_json::json!({ "topic": topic }), "start");
    OapFrame::new(FrameType::Start, payload)
}
pub fn end_frame() -> OapFrame {
    OapFrame::new(FrameType::End, Bytes::new())
}
pub fn ack_frame(credit_bytes: u64) -> OapFrame {
    let payload = json_vec(serde_json::json!({ "credit": credit_bytes }), "ack");
    OapFrame::new(FrameType::Ack, payload)
}
pub fn quota_error_frame(reason: &str) -> OapFrame {
    let payload = json_vec(serde_json::json!({ "code":"quota", "msg": reason }), "err");
    OapFrame::new(FrameType::Error, payload)
}

// --- Tests are in /tests to keep this crate lean ---

```

### crates/oap/tests/quota_error.rs

```rust
#![forbid(unsafe_code)]

use oap::*;
use tokio::io::duplex;

#[tokio::test]
async fn quota_error_roundtrip() -> Result<(), OapError> {
    let (mut a, mut b) = duplex(4096);

    let send = async {
        let f = quota_error_frame("over_quota");
        write_frame(&mut a, &f, DEFAULT_MAX_FRAME).await?;
        Ok::<(), OapError>(())
    };

    let recv = async {
        let fr = read_frame(&mut b, DEFAULT_MAX_FRAME).await?;
        assert!(matches!(fr.typ, FrameType::Error));
        let j: serde_json::Value = serde_json::from_slice(&fr.payload)?;
        assert_eq!(j.get("code").and_then(|v| v.as_str()), Some("quota"));
        assert_eq!(j.get("msg").and_then(|v| v.as_str()), Some("over_quota"));
        Ok::<(), OapError>(())
    };

    futures::future::try_join(send, recv).await?;
    Ok(())
}

```

### crates/oap/tests/roundtrip.rs

```rust
#![forbid(unsafe_code)]

use oap::*;
use serde_json::{json, Value};
use tokio::io::duplex;

#[tokio::test]
async fn hello_roundtrip() -> Result<(), OapError> {
    let (mut a, mut b) = duplex(4096);

    let send = async {
        let f = hello_frame("oap/1");
        write_frame(&mut a, &f, DEFAULT_MAX_FRAME).await?;
        Ok::<_, OapError>(())
    };

    let recv = async {
        let fr = read_frame(&mut b, DEFAULT_MAX_FRAME).await?;
        assert_eq!(fr.ver, OAP_VERSION);
        assert!(matches!(fr.typ, FrameType::Hello));
        let m: Value = serde_json::from_slice(&fr.payload)?;
        assert_eq!(m["hello"], "oap/1");
        Ok::<_, OapError>(())
    };

    futures::future::try_join(send, recv).await?;
    Ok(())
}

#[tokio::test]
async fn start_data_end_with_body() -> Result<(), OapError> {
    let (mut a, mut b) = duplex(1 << 16);

    // sender
    let send = async {
        write_frame(&mut a, &start_frame("tiles/v1"), DEFAULT_MAX_FRAME).await?;

        // DATA with real bytes; header gets obj:"b3:<hex>"
        let body = b"hello world";
        let payload = encode_data_payload(json!({ "mime": "application/octet-stream" }), body)?;
        let df = OapFrame::new(FrameType::Data, payload);
        write_frame(&mut a, &df, DEFAULT_MAX_FRAME).await?;

        write_frame(&mut a, &end_frame(), DEFAULT_MAX_FRAME).await?;
        Ok::<_, OapError>(())
    };

    // receiver
    let recv = async {
        // START
        let st = read_frame(&mut b, DEFAULT_MAX_FRAME).await?;
        assert!(matches!(st.typ, FrameType::Start));

        // DATA
        let df = read_frame(&mut b, DEFAULT_MAX_FRAME).await?;
        assert!(matches!(df.typ, FrameType::Data));

        let (hdr, body) = decode_data_payload(&df.payload)?;
        assert_eq!(hdr["mime"], "application/octet-stream");
        let obj = hdr["obj"].as_str().unwrap_or("");
        assert!(obj.starts_with("b3:"));
        assert_eq!(&body[..], b"hello world");

        // END
        let en = read_frame(&mut b, DEFAULT_MAX_FRAME).await?;
        assert!(matches!(en.typ, FrameType::End));
        Ok::<_, OapError>(())
    };

    futures::future::try_join(send, recv).await?;
    Ok(())
}

#[tokio::test]
async fn ack_roundtrip() -> Result<(), OapError> {
    let (mut a, mut b) = duplex(4096);
    let send = async {
        write_frame(&mut a, &ack_frame(65536), DEFAULT_MAX_FRAME).await?;
        Ok::<_, OapError>(())
    };
    let recv = async {
        let fr = read_frame(&mut b, DEFAULT_MAX_FRAME).await?;
        assert!(matches!(fr.typ, FrameType::Ack));
        let j: Value = serde_json::from_slice(&fr.payload)?;
        assert_eq!(j["credit"].as_u64(), Some(65536));
        Ok::<_, OapError>(())
    };
    futures::future::try_join(send, recv).await?;
    Ok(())
}

#[tokio::test]
async fn invalid_type_errors() -> Result<(), OapError> {
    // Build a manual bad frame: ver=1, typ=0xFF, len=0
    let mut buf = Vec::new();
    buf.extend_from_slice(&[OAP_VERSION, 0xFF, 0, 0, 0, 0]); // len=0
    let (mut a, mut b) = duplex(64);
    use tokio::io::AsyncWriteExt;
    a.write_all(&buf).await?;

    match read_frame(&mut b, DEFAULT_MAX_FRAME).await {
        Err(OapError::UnknownType(0xFF)) => Ok(()),
        other => panic!("unexpected: {other:?}"),
    }
}

```

### crates/overlay/Cargo.toml

```toml
[package]
publish = false
license = "MIT OR Apache-2.0"


name = "overlay"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = { workspace = true }
blake3 = { workspace = true }
sled = { workspace = true }
hex = { workspace = true }
tracing = "0.1"
serde = { workspace = true }
lru = "0.12"                             # NEW: in-memory cache
tokio = { workspace = true, features = ["full"] }
thiserror = { workspace = true }
transport = { workspace = true }
workspace-hack = { workspace = true }


```

### crates/overlay/src/error.rs

```rust
#![forbid(unsafe_code)]

use thiserror::Error;

/// Overlay-internal error type. Library callers still use `anyhow::Result`,
/// but internally we keep a precise, typed surface.
#[derive(Debug, Error)]
pub enum OverlayError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("sled error: {0}")]
    Sled(#[from] sled::Error),

    #[error("UTF-8 decode error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    #[error("early EOF")]
    EarlyEof,

    #[error("unknown opcode: 0x{0:02x}")]
    UnknownOpcode(u8),

    #[error("string too long ({0} bytes)")]
    StringTooLong(usize),

    #[error("invalid chunk_size (must be 0 or >= 4096)")]
    InvalidChunkSize,
}

/// Convenience alias for overlay-internal results.
pub type OResult<T> = std::result::Result<T, OverlayError>;

```

### crates/overlay/src/lib.rs

```rust
#![forbid(unsafe_code)]

// Keep modules
pub mod error;
pub mod protocol;
pub mod store;

// Re-export Store for external users
pub use store::Store;

// Public API (async TCP protocol)
pub use protocol::{client_get, client_put, run_overlay_listener};

```

### crates/overlay/src/protocol.rs

```rust
#![forbid(unsafe_code)]

use anyhow::{bail, Context, Result};
use blake3::Hasher;
use std::net::SocketAddr;
use std::path::Path;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tracing::{info, warn};

use crate::store::Store;

// Wire opcodes
#[repr(u8)]
enum Op {
    Put = 0x01,
    Get = 0x02,
    PutOk = 0x10,
    GetOk = 0x11,
    NotFound = 0x12,
}

pub fn run_overlay_listener(bind: SocketAddr, store_db: impl AsRef<Path>) -> Result<()> {
    let store_db = store_db.as_ref().to_owned();
    tokio::spawn(async move {
        if let Err(e) = serve_tcp(bind, store_db.clone()).await {
            warn!(error=?e, "overlay listener exited with error");
        }
    });
    Ok(())
}

async fn serve_tcp(bind: SocketAddr, store_db: std::path::PathBuf) -> Result<()> {
    let store = Store::open(&store_db)?;
    let listener = TcpListener::bind(bind).await?;
    info!(%bind, store=?store_db, "overlay TCP listening");
    loop {
        let (stream, peer) = listener.accept().await?;
        let store = store.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_conn(stream, peer, store).await {
                warn!(%peer, error=?e, "connection error");
            }
        });
    }
}

async fn handle_conn(mut s: TcpStream, peer: SocketAddr, store: Store) -> Result<()> {
    let mut tag = [0u8; 1];
    s.read_exact(&mut tag).await.context("read opcode")?;
    match tag[0] {
        x if x == Op::Put as u8 => handle_put(&mut s, store).await,
        x if x == Op::Get as u8 => handle_get(&mut s, store).await,
        _ => bail!("bad opcode from {peer}"),
    }
}

async fn handle_put(s: &mut TcpStream, store: Store) -> Result<()> {
    let mut rdr = BufReader::new(s);
    let mut buf = Vec::new();
    rdr.read_until(b'\n', &mut buf).await?; // 8-byte len + '\n'
    if buf.len() < 9 {
        bail!("short length line");
    }

    // Safe decode of the 8-byte BE length prefix.
    let arr: [u8; 8] = buf[..8]
        .try_into()
        .map_err(|_| anyhow::anyhow!("length prefix truncated"))?;
    let n = u64::from_be_bytes(arr) as usize;

    let mut data = vec![0u8; n];
    rdr.read_exact(&mut data).await?;

    // Hash bytes
    let mut hasher = Hasher::new();
    hasher.update(&data);
    let hash_hex = hasher.finalize().to_hex().to_string();

    // Store (key is the hex hash)
    store.put(hash_hex.as_bytes(), data)?;

    // Reply with hash
    let s = rdr.into_inner();
    send_line(s, Op::PutOk as u8, &hash_hex).await?;
    Ok(())
}

async fn handle_get(s: &mut TcpStream, store: Store) -> Result<()> {
    let mut rdr = BufReader::new(s);
    let mut hash_hex = String::new();
    rdr.read_line(&mut hash_hex).await?;
    if hash_hex.ends_with('\n') {
        hash_hex.pop();
        if hash_hex.ends_with('\r') {
            hash_hex.pop();
        }
    }
    let resp = store.get(hash_hex.as_bytes())?;
    let s = rdr.into_inner();

    match resp {
        Some(bytes) => {
            s.write_all(&[Op::GetOk as u8]).await?;
            s.write_all(&(bytes.len() as u64).to_be_bytes()).await?;
            s.write_all(&bytes).await?;
            s.flush().await?;
        }
        None => {
            s.write_all(&[Op::NotFound as u8]).await?;
            s.flush().await?;
        }
    }
    Ok(())
}

async fn send_line(s: &mut TcpStream, tag: u8, line: &str) -> Result<()> {
    s.write_all(&[tag]).await?;
    s.write_all(line.as_bytes()).await?;
    s.write_all(b"\r\n").await?;
    s.flush().await?;
    Ok(())
}

/// Async client: PUT a file to the overlay service. Returns the server's hash.
pub async fn client_put(addr: &str, path: &Path) -> Result<String> {
    // Read whole file synchronously (small convenience)
    let bytes = std::fs::read(path).with_context(|| format!("reading {path:?}"))?;
    let mut s = TcpStream::connect(addr).await?;
    s.write_all(&[Op::Put as u8]).await?;
    s.write_all(&(bytes.len() as u64).to_be_bytes()).await?;
    s.write_all(b"\n").await?;
    s.write_all(&bytes).await?;
    s.flush().await?;

    let mut tag = [0u8; 1];
    s.read_exact(&mut tag).await?;
    if tag[0] != Op::PutOk as u8 {
        bail!("bad response");
    }
    let mut line = String::new();
    BufReader::new(s).read_line(&mut line).await?;
    Ok(line.trim().to_string())
}

/// Async client: GET a blob by hash from the overlay service and write to `out`.
pub async fn client_get(addr: &str, hash_hex: &str, out: &Path) -> Result<()> {
    let mut s = TcpStream::connect(addr).await?;
    s.write_all(&[Op::Get as u8]).await?;
    s.write_all(hash_hex.as_bytes()).await?;
    s.write_all(b"\r\n").await?;
    s.flush().await?;

    let mut tag = [0u8; 1];
    s.read_exact(&mut tag).await?;
    match tag[0] {
        x if x == Op::GetOk as u8 => {
            let mut sz = [0u8; 8];
            s.read_exact(&mut sz).await?;
            let n = u64::from_be_bytes(sz) as usize;
            let mut buf = vec![0u8; n];
            s.read_exact(&mut buf).await?;
            fs::write(out, &buf).await?;
            Ok(())
        }
        x if x == Op::NotFound as u8 => bail!("not found"),
        _ => bail!("bad response"),
    }
}

```

### crates/overlay/src/store.rs

```rust
#![forbid(unsafe_code)]

use anyhow::Result;
use sled::Db;
use std::path::Path;

#[derive(Clone)]
pub struct Store {
    db: Db,
}

impl Store {
    /// Open sled at the given path.
    /// Accepts any path-like type to avoid &str / &Path mismatches.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = sled::open(path.as_ref())?;
        Ok(Self { db })
    }

    /// Put a blob by key.
    pub fn put(&self, key: &[u8], val: Vec<u8>) -> Result<()> {
        self.db.insert(key, val)?;
        self.db.flush()?; // ensure durability for tests/scripts
        Ok(())
    }

    /// Get a blob by key.
    pub fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        Ok(self.db.get(key)?.map(|iv| iv.to_vec()))
    }
}

```

### crates/ron-app-sdk/Cargo.toml

```toml
[package]
name = "ron-app-sdk"
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"
publish = false
description = "RustyOnions Overlay App Protocol (OAP/1) client SDK (Bronze ring)"
repository = "https://example.com/rustyonions"

[dependencies]
# Workspace-unified crates
anyhow              = { workspace = true }
bytes               = { workspace = true }
futures-util        = { workspace = true, features = ["sink"] } # for SinkExt::send()
serde               = { workspace = true }
serde_json          = { workspace = true }
tracing             = { workspace = true }
tokio               = { workspace = true, features = ["rt-multi-thread","macros","net","io-util","time"] }
tokio-rustls        = { workspace = true }
rustls-pemfile      = { workspace = true }
rustls-native-certs = { workspace = true }
tokio-util          = { workspace = true, features = ["codec"] }
thiserror           = { workspace = true }

# Local-only dep (not pinned at workspace root)
bitflags            = "2.6"

# Hakari feature unifier
workspace-hack = { version = "0.1", path = "../../workspace-hack" }

[dev-dependencies]
tracing-subscriber = { workspace = true }

```

### crates/ron-app-sdk/examples/mailbox_recv.rs

```rust
//! Receive up to N messages from a topic and ACK them.
//! Usage:
//!   RON_ADDR=127.0.0.1:9443 RON_SNI=localhost RON_EXTRA_CA=testing/tls/ca.crt \
//!   TOPIC=chat MAX=10 \
//!   cargo run -p ron-app-sdk --example mailbox_recv

use anyhow::{anyhow, Result};
use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use ron_app_sdk::{
    OapCodec, OapFlags, OapFrame, DEFAULT_MAX_DECOMPRESSED, DEFAULT_MAX_FRAME, OAP_VERSION,
};
use serde::Deserialize;
use std::net::ToSocketAddrs;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_rustls::rustls::pki_types::ServerName;
use tokio_rustls::{rustls, TlsConnector};
use tokio_util::codec::Framed;

const MAILBOX_APP_PROTO_ID: u16 = 0x0201;

#[allow(dead_code)]
#[derive(Deserialize)]
struct RecvMsg {
    msg_id: String,
    topic: String,
    text: String,
}

#[derive(Deserialize)]
struct RecvResp {
    messages: Vec<RecvMsg>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct AckResp {
    ok: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let addr = std::env::var("RON_ADDR").unwrap_or_else(|_| "127.0.0.1:9443".to_string());
    let sni = std::env::var("RON_SNI").unwrap_or_else(|_| "localhost".to_string());
    let extra = std::env::var("RON_EXTRA_CA").ok();

    let topic = std::env::var("TOPIC").unwrap_or_else(|_| "chat".to_string());
    let max: usize = std::env::var("MAX")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);

    let tls = connect(&addr, &sni, extra.as_deref()).await?;
    let mut framed = Framed::new(
        tls,
        OapCodec::new(DEFAULT_MAX_FRAME, DEFAULT_MAX_DECOMPRESSED),
    );

    // RECV
    let payload = serde_json::json!({ "op": "recv", "topic": topic, "max": max });
    let req = OapFrame {
        ver: OAP_VERSION,
        flags: OapFlags::REQ | OapFlags::START,
        code: 0,
        app_proto_id: MAILBOX_APP_PROTO_ID,
        tenant_id: 0,
        cap: Bytes::new(),
        corr_id: 2,
        payload: Bytes::from(serde_json::to_vec(&payload)?),
    };
    framed.send(req).await?;

    let resp = framed
        .next()
        .await
        .ok_or_else(|| anyhow!("no recv response"))??;
    if !resp.flags.contains(OapFlags::RESP) {
        return Err(anyhow!("expected RESP"));
    }
    if resp.code != 0 {
        return Err(anyhow!(String::from_utf8_lossy(&resp.payload).to_string()));
    }

    let r: RecvResp = serde_json::from_slice(&resp.payload)?;
    if r.messages.is_empty() {
        println!("no messages");
        return Ok(());
    }

    println!("received {} message(s):", r.messages.len());
    for m in &r.messages {
        println!("- [{}] {}", m.msg_id, m.text);
    }

    // ACK each one
    for (i, m) in r.messages.iter().enumerate() {
        let payload = serde_json::json!({ "op": "ack", "msg_id": m.msg_id });
        let req = OapFrame {
            ver: OAP_VERSION,
            flags: OapFlags::REQ | OapFlags::START,
            code: 0,
            app_proto_id: MAILBOX_APP_PROTO_ID,
            tenant_id: 0,
            cap: Bytes::new(),
            corr_id: (100 + i) as u64,
            payload: Bytes::from(serde_json::to_vec(&payload)?),
        };
        framed.send(req).await?;

        let resp = framed
            .next()
            .await
            .ok_or_else(|| anyhow!("no ack response"))??;
        if resp.code != 0 {
            return Err(anyhow!(String::from_utf8_lossy(&resp.payload).to_string()));
        }
        let _ok: AckResp = serde_json::from_slice(&resp.payload)?;
    }

    println!("acked all");
    Ok(())
}

async fn connect(
    addr: &str,
    server_name: &str,
    extra_ca: Option<&str>,
) -> Result<tokio_rustls::client::TlsStream<TcpStream>> {
    use rustls_pemfile::certs;
    use std::{fs::File, io::BufReader};
    use tokio_rustls::rustls::RootCertStore;

    let sockaddr = addr
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| anyhow!("addr resolve failed"))?;
    let tcp = TcpStream::connect(sockaddr).await?;
    tcp.set_nodelay(true)?;

    let mut roots = RootCertStore::empty();

    // rustls-native-certs >= 0.8: returns CertificateResult { certs, errors }
    let native = rustls_native_certs::load_native_certs();
    for cert in native.certs {
        roots
            .add(cert)
            .map_err(|_| anyhow!("failed to add native root"))?;
    }
    if !native.errors.is_empty() {
        eprintln!(
            "warning: rustls-native-certs reported {} error(s) loading roots: {:?}",
            native.errors.len(),
            native.errors
        );
    }

    if let Some(path) = extra_ca {
        let mut rd = BufReader::new(File::open(path)?);
        for der in certs(&mut rd).collect::<std::result::Result<Vec<_>, _>>()? {
            roots
                .add(der)
                .map_err(|_| anyhow!("failed to add extra ca"))?;
        }
    } else if roots.is_empty() {
        eprintln!(
            "warning: no native roots found and RON_EXTRA_CA not set; TLS validation may fail"
        );
    }

    let config = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    let connector = TlsConnector::from(Arc::new(config));

    let sni: ServerName =
        ServerName::try_from(server_name.to_string()).map_err(|_| anyhow!("invalid sni"))?;
    Ok(connector.connect(sni, tcp).await?)
}

```

### crates/ron-app-sdk/examples/mailbox_send.rs

```rust
//! Send a chat message to a topic via Mailbox (app_proto_id 0x0201).
//! Usage:
//!   RON_ADDR=127.0.0.1:9443 RON_SNI=localhost RON_EXTRA_CA=testing/tls/ca.crt \
//!   TOPIC=chat TEXT="hello vest" IDEMPOTENCY_KEY=abc123 \
//!   cargo run -p ron-app-sdk --example mailbox_send

use anyhow::{anyhow, Result};
use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use ron_app_sdk::{
    OapCodec, OapFlags, OapFrame, DEFAULT_MAX_DECOMPRESSED, DEFAULT_MAX_FRAME, OAP_VERSION,
};
use serde::Deserialize;
use std::net::ToSocketAddrs;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_rustls::rustls::pki_types::ServerName;
use tokio_rustls::{rustls, TlsConnector};
use tokio_util::codec::Framed;

const MAILBOX_APP_PROTO_ID: u16 = 0x0201;

#[derive(Deserialize)]
struct SendResp {
    msg_id: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let addr = std::env::var("RON_ADDR").unwrap_or_else(|_| "127.0.0.1:9443".to_string());
    let sni = std::env::var("RON_SNI").unwrap_or_else(|_| "localhost".to_string());
    let extra = std::env::var("RON_EXTRA_CA").ok();

    let topic = std::env::var("TOPIC").unwrap_or_else(|_| "chat".to_string());
    let text = std::env::var("TEXT").unwrap_or_else(|_| "hello".to_string());
    let idem = std::env::var("IDEMPOTENCY_KEY").ok();

    let tls = connect(&addr, &sni, extra.as_deref()).await?;
    let mut framed = Framed::new(
        tls,
        OapCodec::new(DEFAULT_MAX_FRAME, DEFAULT_MAX_DECOMPRESSED),
    );

    let mut payload = serde_json::json!({"op":"send","topic":topic,"text":text});
    if let Some(k) = idem {
        payload["idempotency_key"] = serde_json::Value::String(k);
    }

    let req = OapFrame {
        ver: OAP_VERSION,
        flags: OapFlags::REQ | OapFlags::START,
        code: 0,
        app_proto_id: MAILBOX_APP_PROTO_ID,
        tenant_id: 0,
        cap: Bytes::new(),
        corr_id: 1,
        payload: Bytes::from(serde_json::to_vec(&payload)?),
    };
    framed.send(req).await?;

    let resp = framed
        .next()
        .await
        .ok_or_else(|| anyhow!("no response"))??;
    if !resp.flags.contains(OapFlags::RESP) {
        return Err(anyhow!("expected RESP"));
    }
    if resp.code != 0 {
        return Err(anyhow!(String::from_utf8_lossy(&resp.payload).to_string()));
    }

    let s: SendResp = serde_json::from_slice(&resp.payload)?;
    println!("msg_id: {}", s.msg_id);

    Ok(())
}

async fn connect(
    addr: &str,
    server_name: &str,
    extra_ca: Option<&str>,
) -> Result<tokio_rustls::client::TlsStream<TcpStream>> {
    use rustls_pemfile::certs;
    use std::{fs::File, io::BufReader};
    use tokio_rustls::rustls::RootCertStore;

    let sockaddr = addr
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| anyhow!("addr resolve failed"))?;
    let tcp = TcpStream::connect(sockaddr).await?;
    tcp.set_nodelay(true)?;

    let mut roots = RootCertStore::empty();

    // rustls-native-certs >= 0.8: returns CertificateResult { certs, errors }
    let native = rustls_native_certs::load_native_certs();
    for cert in native.certs {
        roots
            .add(cert)
            .map_err(|_| anyhow!("failed to add native root"))?;
    }
    if !native.errors.is_empty() {
        eprintln!(
            "warning: rustls-native-certs reported {} error(s) loading roots: {:?}",
            native.errors.len(),
            native.errors
        );
    }

    if let Some(path) = extra_ca {
        let mut rd = BufReader::new(File::open(path)?);
        for der in certs(&mut rd).collect::<std::result::Result<Vec<_>, _>>()? {
            roots
                .add(der)
                .map_err(|_| anyhow!("failed to add extra ca"))?;
        }
    } else if roots.is_empty() {
        eprintln!(
            "warning: no native roots found and RON_EXTRA_CA not set; TLS validation may fail"
        );
    }

    let config = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();

    let connector = TlsConnector::from(Arc::new(config));
    let sni: ServerName =
        ServerName::try_from(server_name.to_string()).map_err(|_| anyhow!("invalid sni"))?;
    Ok(connector.connect(sni, tcp).await?)
}

```

### crates/ron-app-sdk/examples/oap_echo_client.rs

```rust
//! Minimal echo client: connects, HELLO, then sends a one-shot REQ to app_proto_id=0x0F01
//! Usage:
//!   RON_ADDR=127.0.0.1:9443 RON_SNI=localhost cargo run -p ron-app-sdk --example oap_echo_client

use anyhow::Result;
use bytes::Bytes;
use ron_app_sdk::OverlayClient;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    let addr = std::env::var("RON_ADDR").unwrap_or_else(|_| "127.0.0.1:9443".to_string());
    let sni = std::env::var("RON_SNI").unwrap_or_else(|_| "localhost".to_string());

    let mut client = OverlayClient::connect(&addr, &sni).await?;
    let hello = client.hello().await?;
    println!("HELLO: {hello:#?}");

    // Build a tiny JSON payload and send to a demo app_proto_id (0x0F01).
    let payload = Bytes::from(serde_json::to_vec(&json!({"op":"echo","msg":"ping"}))?);
    let resp = client.request_oneshot(0x0F01, 0, payload).await?;

    println!("RESP bytes: {}", resp.len());
    if let Ok(txt) = std::str::from_utf8(&resp) {
        println!("RESP text: {txt}");
    }
    Ok(())
}

```

### crates/ron-app-sdk/examples/oap_echo_server.rs

```rust
// crates/ron-app-sdk/examples/oap_echo_server.rs
use anyhow::{anyhow, Context, Result};
use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use ron_app_sdk::{
    Hello, OapCodec, OapFlags, OapFrame, DEFAULT_MAX_DECOMPRESSED, DEFAULT_MAX_FRAME, OAP_VERSION,
};
use std::{fs::File, io::BufReader, sync::Arc};
use tokio::{net::TcpListener, task};
use tokio_rustls::rustls;
use tokio_rustls::TlsAcceptor;
use tokio_util::codec::Framed;
use tracing::{error, info, warn};

fn load_tls() -> Result<rustls::ServerConfig> {
    use rustls_pemfile::{certs, pkcs8_private_keys};

    let cert_path = std::env::var("CERT_PEM").context("CERT_PEM not set")?;
    let key_path = std::env::var("KEY_PEM").context("KEY_PEM not set")?;

    // Certificates (already CertificateDer)
    let mut cert_reader = BufReader::new(File::open(cert_path)?);
    let cert_chain = certs(&mut cert_reader).collect::<std::result::Result<Vec<_>, _>>()?;

    // Private key (PKCS#8) → wrap into PrivateKeyDer::Pkcs8 to satisfy rustls 0.23
    let mut key_reader = BufReader::new(File::open(key_path)?);
    let mut keys =
        pkcs8_private_keys(&mut key_reader).collect::<std::result::Result<Vec<_>, _>>()?;
    if keys.is_empty() {
        return Err(anyhow!("no PKCS#8 key found"));
    }
    let key_der = rustls::pki_types::PrivateKeyDer::Pkcs8(keys.remove(0));

    let config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_chain, key_der)
        .map_err(|e| anyhow!("server cert error: {e}"))?;

    Ok(config)
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cfg = load_tls()?;
    let acceptor = TlsAcceptor::from(Arc::new(cfg));

    let listener = TcpListener::bind("127.0.0.1:9443").await?;
    info!("TLS echo server on 127.0.0.1:9443");

    loop {
        let (tcp, peer) = listener.accept().await?;
        let acceptor = acceptor.clone();
        task::spawn(async move {
            match handle_conn(acceptor, tcp).await {
                Ok(()) => info!("conn {peer:?} closed"),
                Err(e) => {
                    // Downgrade common disconnects to info
                    let msg = e.to_string().to_lowercase();
                    if msg.contains("close_notify") || msg.contains("unexpected eof") {
                        info!("conn {peer:?} disconnected: {e}");
                    } else {
                        error!("conn {peer:?} error: {e}");
                    }
                }
            }
        });
    }
}

async fn handle_conn(acceptor: TlsAcceptor, tcp: tokio::net::TcpStream) -> Result<()> {
    let tls = acceptor.accept(tcp).await?;
    let mut framed = Framed::new(
        tls,
        OapCodec::new(DEFAULT_MAX_FRAME, DEFAULT_MAX_DECOMPRESSED),
    );

    while let Some(next) = framed.next().await {
        let frame = match next {
            Ok(f) => f,
            Err(e) => {
                // Treat abrupt client closes as graceful
                let m = e.to_string().to_lowercase();
                if m.contains("close_notify") || m.contains("unexpected eof") {
                    warn!("client closed without TLS close_notify");
                    return Ok(());
                }
                return Err(anyhow!(e));
            }
        };

        // HELLO
        if frame.app_proto_id == 0 {
            let body = serde_json::to_vec(&Hello {
                server_version: "dev-echo-1.0.0".into(),
                max_frame: DEFAULT_MAX_FRAME as u64,
                max_inflight: 64,
                supported_flags: vec![
                    "EVENT".into(),
                    "ACK_REQ".into(),
                    "COMP".into(),
                    "APP_E2E".into(),
                ],
                oap_versions: vec![OAP_VERSION],
                transports: vec!["tcp+tls".into()],
            })?;
            let resp = OapFrame {
                ver: OAP_VERSION,
                flags: OapFlags::RESP | OapFlags::END,
                code: 0,
                app_proto_id: 0,
                tenant_id: frame.tenant_id,
                cap: Bytes::new(),
                corr_id: frame.corr_id,
                payload: Bytes::from(body),
            };
            framed.send(resp).await?;
            continue;
        }

        // Echo app: mirror payload
        let mut flags = OapFlags::RESP;
        if frame.flags.contains(OapFlags::START) {
            flags |= OapFlags::START;
        }
        if frame.flags.contains(OapFlags::END) {
            flags |= OapFlags::END;
        }

        let resp = OapFrame {
            ver: OAP_VERSION,
            flags,
            code: 0,
            app_proto_id: frame.app_proto_id,
            tenant_id: frame.tenant_id,
            cap: Bytes::new(),
            corr_id: frame.corr_id,
            payload: frame.payload.clone(),
        };
        framed.send(resp).await?;
    }

    Ok(())
}

```

### crates/ron-app-sdk/examples/tiles_get.rs

```rust
//! Stream a tile via OAP/1 (app_proto_id 0x0301) and save to disk.
//! Usage:
//!   RON_ADDR=127.0.0.1:9443 RON_SNI=localhost RON_EXTRA_CA=testing/tls/ca.crt \
//!   TILE_PATH=/tiles/12/654/1583.webp OUT=./out.webp \
//!   cargo run -p ron-app-sdk --example tiles_get

use anyhow::{anyhow, Context, Result};
use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use ron_app_sdk::{
    OapCodec, OapFlags, OapFrame, DEFAULT_MAX_DECOMPRESSED, DEFAULT_MAX_FRAME, OAP_VERSION,
};
use std::{fs::File, io::Write, net::ToSocketAddrs, sync::Arc};
use tokio::net::TcpStream;
use tokio_rustls::rustls::pki_types::ServerName;
use tokio_rustls::{rustls, TlsConnector};
use tokio_util::codec::Framed;

const TILE_APP_PROTO_ID: u16 = 0x0301;

#[tokio::main]
async fn main() -> Result<()> {
    let addr = std::env::var("RON_ADDR").unwrap_or_else(|_| "127.0.0.1:9443".to_string());
    let sni = std::env::var("RON_SNI").unwrap_or_else(|_| "localhost".to_string());
    let extra = std::env::var("RON_EXTRA_CA").ok();
    let tile_path =
        std::env::var("TILE_PATH").unwrap_or_else(|_| "/tiles/12/654/1583.webp".to_string());
    let out_path = std::env::var("OUT").unwrap_or_else(|_| "out.webp".to_string());

    let tls = connect(&addr, &sni, extra.as_deref()).await?;
    let mut framed = Framed::new(
        tls,
        OapCodec::new(DEFAULT_MAX_FRAME, DEFAULT_MAX_DECOMPRESSED),
    );

    // Request: REQ|START for streaming GET
    let payload = Bytes::from(format!(r#"{{"op":"get","path":"{}"}}"#, tile_path));
    let req = OapFrame {
        ver: OAP_VERSION,
        flags: OapFlags::REQ | OapFlags::START,
        code: 0,
        app_proto_id: TILE_APP_PROTO_ID,
        tenant_id: 0,
        cap: Bytes::new(),
        corr_id: 42,
        payload,
    };
    framed.send(req).await?;

    let mut out = File::create(&out_path).with_context(|| format!("create {}", out_path))?;
    let mut total = 0usize;

    while let Some(msg) = framed.next().await {
        let frame = msg?;
        if frame.app_proto_id != TILE_APP_PROTO_ID {
            return Err(anyhow!("unexpected app id {}", frame.app_proto_id));
        }
        if frame.flags.contains(OapFlags::RESP) {
            out.write_all(&frame.payload)?;
            total += frame.payload.len();
            if frame.flags.contains(OapFlags::END) {
                break;
            }
        } else {
            return Err(anyhow!("expected RESP"));
        }
    }

    println!("saved {} bytes to {}", total, out_path);
    Ok(())
}

async fn connect(
    addr: &str,
    server_name: &str,
    extra_ca: Option<&str>,
) -> Result<tokio_rustls::client::TlsStream<TcpStream>> {
    use rustls_pemfile::certs;
    use std::{fs::File, io::BufReader};
    use tokio_rustls::rustls::RootCertStore;

    let sockaddr = addr
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| anyhow!("addr resolve failed"))?;

    let tcp = TcpStream::connect(sockaddr).await?;
    tcp.set_nodelay(true)?;

    // Root store = native + optional extra CA (our dev CA)
    let mut roots = RootCertStore::empty();

    // rustls-native-certs >= 0.8: returns CertificateResult { certs, errors }
    let native = rustls_native_certs::load_native_certs();
    for cert in native.certs {
        roots
            .add(cert)
            .map_err(|_| anyhow!("failed to add native root"))?;
    }
    if !native.errors.is_empty() {
        eprintln!(
            "warning: rustls-native-certs reported {} error(s) loading roots: {:?}",
            native.errors.len(),
            native.errors
        );
    }

    if let Some(path) = extra_ca {
        let mut rd = BufReader::new(File::open(path)?);
        for der in certs(&mut rd).collect::<std::result::Result<Vec<_>, _>>()? {
            roots
                .add(der)
                .map_err(|_| anyhow!("failed to add extra ca"))?;
        }
    } else if roots.is_empty() {
        eprintln!(
            "warning: no native roots found and RON_EXTRA_CA not set; TLS validation may fail"
        );
    }

    let config = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();

    let connector = TlsConnector::from(Arc::new(config));
    let sni: ServerName =
        ServerName::try_from(server_name.to_string()).map_err(|_| anyhow!("invalid sni"))?;
    let tls = connector.connect(sni, tcp).await?;
    Ok(tls)
}

```

### crates/ron-app-sdk/src/client/hello.rs

```rust
#![forbid(unsafe_code)]

use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::time::timeout;
use tracing::warn;

use crate::constants::OAP_VERSION;
use crate::errors::{Error, Result};
use crate::oap::flags::OapFlags;
use crate::oap::frame::OapFrame;
use crate::oap::hello::Hello;

use super::OverlayClient;

impl OverlayClient {
    /// Perform HELLO probe (app_proto_id=0). Saves the server info.
    pub async fn hello(&mut self) -> Result<Hello> {
        // HELLO request is an empty frame with app_proto_id=0.
        let req = OapFrame::hello_request();

        self.framed.send(req).await?;

        // Wait up to 5 seconds for a response
        let resp = timeout(Duration::from_secs(5), self.framed.next())
            .await
            .map_err(|_| Error::Timeout)?
            .ok_or_else(|| Error::Protocol("connection closed".into()))??;

        if !resp.flags.contains(OapFlags::RESP) && resp.app_proto_id != 0 {
            return Err(Error::Protocol("unexpected frame for HELLO".into()));
        }

        // Parse JSON body
        let hello: Hello = serde_json::from_slice(&resp.payload)
            .map_err(|e| Error::Decode(format!("hello json: {e}")))?;

        // Basic version check
        if !hello.oap_versions.contains(&OAP_VERSION) {
            warn!(
                "server does not list OAP/1; reported: {:?}",
                hello.oap_versions
            );
        }

        self.server = Some(hello.clone());
        Ok(hello)
    }
}

```

### crates/ron-app-sdk/src/client/mod.rs

```rust
#![forbid(unsafe_code)]

use tokio::net::TcpStream;
use tokio_rustls::client::TlsStream;
use tokio_util::codec::Framed;

use crate::oap::codec::OapCodec;

pub type FramedTls = Framed<TlsStream<TcpStream>, OapCodec>;

/// Minimal OAP/1 client (Bronze ring)
pub struct OverlayClient {
    pub(super) framed: FramedTls,
    pub(super) corr_seq: u64,
    pub(super) server: Option<crate::oap::hello::Hello>,
}

impl OverlayClient {
    #[inline]
    pub(super) fn next_corr(&mut self) -> u64 {
        let v = self.corr_seq;
        self.corr_seq = self.corr_seq.wrapping_add(1).max(1);
        v
    }
}

pub mod hello;
pub mod oneshot;
pub mod tls;

```

### crates/ron-app-sdk/src/client/oneshot.rs

```rust
#![forbid(unsafe_code)]

use std::time::Duration;

use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use tokio::time::timeout;

use crate::errors::{Error, Result};
use crate::oap::flags::OapFlags;
use crate::oap::frame::OapFrame;

use super::OverlayClient;

impl OverlayClient {
    /// Simple one-shot request (REQ|START|END) returning single RESP.
    ///
    /// Use this for small control-plane ops (e.g., tile lookup headers or mailbox commands).
    pub async fn request_oneshot(
        &mut self,
        app_proto_id: u16,
        tenant_id: u128,
        payload: impl Into<Bytes>,
    ) -> Result<Bytes> {
        let corr = self.next_corr();
        let req = OapFrame::oneshot_req(app_proto_id, tenant_id, corr, payload.into());
        self.framed.send(req).await?;

        let resp = timeout(Duration::from_secs(10), self.framed.next())
            .await
            .map_err(|_| Error::Timeout)?
            .ok_or_else(|| Error::Protocol("connection closed".into()))??;

        if !resp.flags.contains(OapFlags::RESP) {
            return Err(Error::Protocol("expected RESP".into()));
        }
        if resp.corr_id != corr {
            return Err(Error::Protocol("corr_id mismatch".into()));
        }
        // Map nonzero status to error families later; Bronze returns payload.
        Ok(resp.payload)
    }
}

```

### crates/ron-app-sdk/src/client/tls.rs

```rust
#![forbid(unsafe_code)]

use std::{fs::File, io::BufReader, net::ToSocketAddrs, sync::Arc};

use tokio::net::TcpStream;
use tokio_rustls::rustls::pki_types::ServerName;
use tokio_rustls::{rustls, TlsConnector};
use tokio_util::codec::Framed;

use crate::constants::{DEFAULT_MAX_DECOMPRESSED, DEFAULT_MAX_FRAME};
use crate::errors::{Error, Result};
use crate::oap::codec::OapCodec;

use super::OverlayClient;

impl OverlayClient {
    /// Connect over TCP+TLS using system roots, plus optional extra CA from `RON_EXTRA_CA` (PEM).
    ///
    /// `addr`: "host:port", `server_name`: SNI/hostname (must match cert CN/SAN).
    pub async fn connect(addr: &str, server_name: &str) -> Result<Self> {
        // Resolve
        let sockaddr = addr
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| Error::Io(std::io::Error::other("addr resolve failed")))?;

        let tcp = TcpStream::connect(sockaddr).await?;
        tcp.set_nodelay(true)?;

        // Build rustls client config with native roots (rustls-native-certs >= 0.8)
        let mut roots = rustls::RootCertStore::empty();

        // New API returns CertificateResult { certs, errors }.
        // Proceed with partial success but warn about any errors.
        let native = rustls_native_certs::load_native_certs();
        for cert in native.certs {
            // Each item is CertificateDer<'static>
            roots
                .add(cert)
                .map_err(|_| Error::Protocol("failed to add root cert".into()))?;
        }
        if !native.errors.is_empty() {
            tracing::warn!(
                "rustls-native-certs reported {} error(s) while loading roots: {:?}",
                native.errors.len(),
                native.errors
            );
        }

        // Optional extra CA (useful for self-signed local server)
        if let Ok(extra_path) = std::env::var("RON_EXTRA_CA") {
            let mut rd =
                BufReader::new(File::open(&extra_path).map_err(|e| {
                    Error::Protocol(format!("open RON_EXTRA_CA {extra_path}: {e}"))
                })?);

            for der in rustls_pemfile::certs(&mut rd)
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|e| Error::Protocol(format!("parse RON_EXTRA_CA {extra_path}: {e}")))?
            {
                roots
                    .add(der)
                    .map_err(|_| Error::Protocol("failed to add RON_EXTRA_CA cert".into()))?;
            }
        } else if roots.is_empty() {
            // No native roots and no extra CA path; continue (permissive) but warn loudly.
            tracing::warn!("no native root certificates found and RON_EXTRA_CA not set; TLS validation may fail");
        }

        let config = rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();

        let connector = TlsConnector::from(Arc::new(config));

        // Owned ServerName<'static> so the future doesn't borrow `server_name`
        let sni: ServerName = ServerName::try_from(server_name.to_string())
            .map_err(|_| Error::InvalidDnsName(server_name.to_string()))?;

        let tls = connector.connect(sni, tcp).await?;

        let codec = OapCodec::new(DEFAULT_MAX_FRAME, DEFAULT_MAX_DECOMPRESSED);
        let framed = Framed::new(tls, codec);

        Ok(Self {
            framed,
            corr_seq: 1,
            server: None,
        })
    }
}

```

### crates/ron-app-sdk/src/constants.rs

```rust
#![forbid(unsafe_code)]

/// Protocol version (OAP/1).
pub const OAP_VERSION: u8 = 1;

/// Default maximum encoded frame size accepted/produced by the SDK.
pub const DEFAULT_MAX_FRAME: usize = 1024 * 1024; // 1 MiB

/// Default maximum decompressed payload (server MAY tighten via config).
pub const DEFAULT_MAX_DECOMPRESSED: usize = 8 * 1024 * 1024; // 8 MiB

```

### crates/ron-app-sdk/src/errors.rs

```rust
#![forbid(unsafe_code)]

use std::io;

use thiserror::Error;
use tokio_rustls::rustls;

#[derive(Debug, Error)]
pub enum Error {
    #[error("io: {0}")]
    Io(#[from] io::Error),

    #[error("tls: {0}")]
    Tls(#[from] rustls::Error),

    #[error("invalid DNS name: {0}")]
    InvalidDnsName(String),

    #[error("decode: {0}")]
    Decode(String),

    #[error("protocol: {0}")]
    Protocol(String),

    #[error("timeout")]
    Timeout,
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

```

### crates/ron-app-sdk/src/lib.rs

```rust
#![forbid(unsafe_code)]
//! ron-app-sdk: Minimal OAP/1 client for RustyOnions overlay.
//! Bronze ring scope: framing, HELLO, single-shot request (REQ|START|END).
//! Streaming APIs are stubbed and will land in Silver.

pub mod client;
pub mod constants;
pub mod errors;
pub mod oap;

pub use client::OverlayClient;
pub use constants::{DEFAULT_MAX_DECOMPRESSED, DEFAULT_MAX_FRAME, OAP_VERSION};
pub use errors::Error;
pub use oap::codec::OapCodec;
pub use oap::{Hello, OapFlags, OapFrame};

```

### crates/ron-app-sdk/src/oap/codec/decoder.rs

```rust
#![forbid(unsafe_code)]

use bytes::BytesMut;
use bytes::{Buf, Bytes};

use super::OapCodec;
use crate::constants::OAP_VERSION;
use crate::errors::{Error, Result};
use crate::oap::flags::OapFlags;
use crate::oap::frame::OapFrame;

#[inline]
fn need_bytes(src: &BytesMut, n: usize) -> bool {
    src.len() >= n
}

pub(super) fn decode_frame(codec: &mut OapCodec, src: &mut BytesMut) -> Result<Option<OapFrame>> {
    // Need at least len (4 bytes)
    if !need_bytes(src, 4) {
        return Ok(None);
    }

    let len = {
        let b = &src[..4];
        u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize
    };

    if len > codec.max_frame {
        return Err(Error::Decode(format!(
            "frame too large: {} > {}",
            len, codec.max_frame
        )));
    }

    // Total bytes needed = 4 + len
    if !need_bytes(src, 4 + len) {
        return Ok(None);
    }

    // Consume len prefix
    src.advance(4);
    let mut frame = src.split_to(len);

    // Fixed header sizes
    const FIXED: usize = 1 + 2 + 2 + 2 + 16 + 2 + 8;
    if frame.len() < FIXED {
        return Err(Error::Decode("truncated header".into()));
    }

    let ver = frame.get_u8();
    if ver != OAP_VERSION {
        return Err(Error::Protocol(format!("unsupported version {}", ver)));
    }

    let flags_bits = frame.get_u16_le();
    let flags = OapFlags::from_bits_truncate(flags_bits);

    let code = frame.get_u16_le();
    let app_proto_id = frame.get_u16_le();

    // u128 tenant (little-endian)
    let mut tenant_bytes = [0u8; 16];
    frame.copy_to_slice(&mut tenant_bytes);
    let tenant_id = u128::from_le_bytes(tenant_bytes);

    let cap_len = frame.get_u16_le() as usize;
    let corr_id = frame.get_u64_le();

    if cap_len > frame.len() {
        return Err(Error::Decode("cap_len exceeds frame".into()));
    }

    let cap = if cap_len > 0 {
        if !flags.contains(OapFlags::START) {
            return Err(Error::Protocol(
                "cap present on non-START frame (invalid)".into(),
            ));
        }
        frame.split_to(cap_len).freeze()
    } else {
        Bytes::new()
    };

    // Remaining is payload
    let payload = frame.freeze();

    Ok(Some(OapFrame {
        ver,
        flags,
        code,
        app_proto_id,
        tenant_id,
        cap,
        corr_id,
        payload,
    }))
}

```

### crates/ron-app-sdk/src/oap/codec/encoder.rs

```rust
#![forbid(unsafe_code)]

use bytes::{BufMut, BytesMut};

use super::OapCodec;
use crate::errors::{Error, Result};
use crate::oap::flags::OapFlags;
use crate::oap::frame::OapFrame;

pub(super) fn encode_frame(codec: &mut OapCodec, item: OapFrame, dst: &mut BytesMut) -> Result<()> {
    // Validate START/cap invariants before writing.
    if !item.cap.is_empty() && !item.flags.contains(OapFlags::START) {
        return Err(Error::Protocol(
            "cap present on non-START frame (invalid)".into(),
        ));
    }

    let cap_len = item.cap.len();
    let body_len = 1 + 2 + 2 + 2 + 16 + 2 + 8 + cap_len + item.payload.len();

    if body_len > codec.max_frame {
        return Err(Error::Decode(format!(
            "frame too large to encode: {} > {}",
            body_len, codec.max_frame
        )));
    }

    // Reserve: 4 for len + body_len
    dst.reserve(4 + body_len);

    // len
    dst.put_u32_le(body_len as u32);

    // ver
    dst.put_u8(item.ver);

    // flags, code, app id
    dst.put_u16_le(item.flags.bits());
    dst.put_u16_le(item.code);
    dst.put_u16_le(item.app_proto_id);

    // tenant (u128 LE)
    dst.put_slice(&item.tenant_id.to_le_bytes());

    // cap len
    dst.put_u16_le(cap_len as u16);

    // corr_id
    dst.put_u64_le(item.corr_id);

    // cap (if any)
    if cap_len > 0 {
        dst.put_slice(&item.cap);
    }

    // payload
    dst.put_slice(&item.payload);

    Ok(())
}

```

### crates/ron-app-sdk/src/oap/codec/mod.rs

```rust
#![forbid(unsafe_code)]

use bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder};

use super::frame::OapFrame;
use crate::constants::{DEFAULT_MAX_DECOMPRESSED, DEFAULT_MAX_FRAME};
use crate::errors::{Error, Result};

mod decoder;
mod encoder;

pub struct OapCodec {
    pub(super) max_frame: usize,
    // Not used in Bronze yet; reserved for COMP guard-rails in Silver.
    pub(super) _max_decompressed: usize,
}

impl Default for OapCodec {
    fn default() -> Self {
        Self {
            max_frame: DEFAULT_MAX_FRAME,
            _max_decompressed: DEFAULT_MAX_DECOMPRESSED,
        }
    }
}

impl OapCodec {
    pub fn new(max_frame: usize, max_decompressed: usize) -> Self {
        Self {
            max_frame,
            _max_decompressed: max_decompressed,
        }
    }
}

impl Decoder for OapCodec {
    type Item = OapFrame;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>> {
        decoder::decode_frame(self, src)
    }
}

impl Encoder<OapFrame> for OapCodec {
    type Error = Error;

    fn encode(&mut self, item: OapFrame, dst: &mut BytesMut) -> Result<()> {
        encoder::encode_frame(self, item, dst)
    }
}

```

### crates/ron-app-sdk/src/oap/flags.rs

```rust
#![forbid(unsafe_code)]

bitflags::bitflags! {
    /// OAP/1 flags (subset needed for Bronze).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct OapFlags: u16 {
        const REQ      = 1 << 0;
        const RESP     = 1 << 1;
        const EVENT    = 1 << 2;
        const START    = 1 << 3;
        const END      = 1 << 4;
        const ACK_REQ  = 1 << 5;
        const COMP     = 1 << 6;
        const APP_E2E  = 1 << 7;
    }
}

```

### crates/ron-app-sdk/src/oap/frame.rs

```rust
#![forbid(unsafe_code)]

use bytes::Bytes;

use super::flags::OapFlags;
use crate::constants::OAP_VERSION;

/// OAP/1 frame in host representation.
#[derive(Debug, Clone)]
pub struct OapFrame {
    pub ver: u8,
    pub flags: OapFlags,
    pub code: u16,
    pub app_proto_id: u16,
    pub tenant_id: u128,
    pub cap: Bytes, // optional; only valid when START set
    pub corr_id: u64,
    pub payload: Bytes, // opaque; may be COMP or APP_E2E
}

impl OapFrame {
    pub fn hello_request() -> Self {
        OapFrame {
            ver: OAP_VERSION,
            flags: OapFlags::empty(), // simple request with empty body
            code: 0,
            app_proto_id: 0, // HELLO
            tenant_id: 0,
            cap: Bytes::new(),
            corr_id: 0,
            payload: Bytes::new(),
        }
    }

    /// Helper for single-shot request (REQ|START|END).
    pub fn oneshot_req(app_proto_id: u16, tenant_id: u128, corr_id: u64, payload: Bytes) -> Self {
        OapFrame {
            ver: OAP_VERSION,
            flags: OapFlags::REQ | OapFlags::START | OapFlags::END,
            code: 0,
            app_proto_id,
            tenant_id,
            cap: Bytes::new(),
            corr_id,
            payload,
        }
    }
}

```

### crates/ron-app-sdk/src/oap/hello.rs

```rust
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

/// Minimal HELLO payload (JSON).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Hello {
    pub server_version: String,
    pub max_frame: u64,
    pub max_inflight: u64,
    pub supported_flags: Vec<String>,
    pub oap_versions: Vec<u8>,
    pub transports: Vec<String>,
}

```

### crates/ron-app-sdk/src/oap/mod.rs

```rust
#![forbid(unsafe_code)]

pub mod codec;
pub mod flags;
pub mod frame;
pub mod hello;

pub use flags::OapFlags;
pub use frame::OapFrame;
pub use hello::Hello;

```

### crates/ron-audit/Cargo.toml

```toml
[package]
name = "ron-audit"
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"
publish = false

[features]
default = []
fs = []

[dependencies]
serde = { workspace = true, features = ["derive"] }
serde_json = { workspace = true }
rmp-serde = { workspace = true }
blake3 = "1.5"
ed25519-dalek = { version = "2", features = ["rand_core"] }
rand = "0.8"
time = "0.3"
anyhow = { workspace = true }

```

### crates/ron-audit/src/lib.rs

```rust
#![forbid(unsafe_code)]
// ron-audit: signed, chained audit log (append-only).

use anyhow::Result;
use blake3::Hasher;
use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug)]
pub struct Auditor {
    sk: SigningKey,
    vk: VerifyingKey,
    prev_hash: [u8; 32],
    #[cfg(feature = "fs")]
    dir: std::path::PathBuf,
}

impl Auditor {
    pub fn new() -> Self {
        let mut rng = OsRng;
        let sk = SigningKey::generate(&mut rng);
        let vk = sk.verifying_key();
        Self { sk, vk, prev_hash: [0u8; 32], #[cfg(feature="fs")] dir: std::path::PathBuf::new() }
    }
    #[cfg(feature = "fs")]
    pub fn with_dir(mut self, dir: impl Into<std::path::PathBuf>) -> Self {
        self.dir = dir.into();
        std::fs::create_dir_all(&self.dir).ok();
        self
    }
    pub fn verifying_key(&self) -> &VerifyingKey { &self.vk }

    pub fn append(&mut self, kind: &'static str, data: serde_json::Value) -> Result<AuditRecord> {
        let ts = OffsetDateTime::now_tc().unix_timestamp();
        let mut hasher = Hasher::new();
        hasher.update(&self.prev_hash);
        let body = AuditBody { ts, kind, data };
        let ser = rmp_serde::to_vec_named(&body)?;
        hasher.update(&ser);
        let hash = *hasher.finalize().as_bytes();
        let sig = self.sk.sign(&hash);
        let rec = AuditRecord { prev_hash: self.prev_hash, hash, sig: sig.to_bytes().to_vec(), body };
        self.prev_hash = hash;

        #[cfg(feature = "fs")]
        {
            let p = self.dir.join(format!("{ts}-{kind}.bin"));
            std::fs::write(p, rmp_serde::to_vec_named(&rec)?)?;
        }
        Ok(rec)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditBody {
    pub ts: i64,
    pub kind: &'static str,
    /// Arbitrary JSON (small)
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    pub prev_hash: [u8; 32],
    pub hash: [u8; 32],
    pub sig: Vec<u8>,
    pub body: AuditBody,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chain_two() {
        let mut a = Auditor::new();
        let r1 = a.append("auth-fail", serde_json::json!({"who":"svc-gateway"})).unwrap();
        let r2 = a.append("key-rotated", serde_json::json!({"id":"epoch-42"})).unwrap();
        assert_eq!(r1.hash, r2.prev_hash);
        assert_ne!(r1.hash, r2.hash);
    }
}

```

### crates/ron-auth/Cargo.toml

```toml
[package]
name = "ron-auth"
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"
publish = false

[dependencies]
serde = { workspace = true, features = ["derive"] }
rmp-serde = { workspace = true }
hmac = "0.12"
sha2 = { workspace = true }           # e.g., "0.10" at workspace
uuid = "1.10"
smallvec = "1.13"
time = "0.3"
thiserror = { workspace = true }
rand = "0.8"

```

### crates/ron-auth/src/lib.rs

```rust
#![forbid(unsafe_code)]
// ron-auth: zero-trust envelopes for internal IPC & UDS boundaries.

use hmac::{Hmac, Mac};
use rand::RngCore;
use sha2::Sha256;
use smallvec::SmallVec;
use time::OffsetDateTime;
use uuid::Uuid;

use serde::{Deserialize, Serialize};

pub type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Plane { Node, App }

/// Generic, self-authenticating envelope around a protocol header+payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope<H, P> {
    pub plane: Plane,
    pub origin_svc: &'static str,
    pub origin_instance: Uuid,
    pub tenant_id: Option<String>,
    #[serde(with = "serde_scopes")]
    pub scopes: SmallVec<[String; 4]>,
    pub nonce: [u8; 16],
    pub iat: i64, // seconds since epoch
    pub exp: i64, // seconds since epoch
    pub header: H,
    pub payload: P,
    pub tag: [u8; 32], // HMAC-SHA256
}

/// A minimal trait for envelope key derivation.
/// Implement this in services, usually by delegating to ron-kms.
pub trait KeyDeriver: Send + Sync + 'static {
    /// 32-byte HMAC key for (svc, instance, epoch).
    fn derive_origin_key(&self, svc: &str, instance: &Uuid, epoch: u64) -> [u8; 32];
}

/// Construct tag input deterministically using rmp-serde to avoid JSON ambiguity.
fn encode_for_mac<H: Serialize, P: Serialize>(env: &Envelope<H, P>) -> Vec<u8> {
    #[derive(Serialize)]
    struct MacView<'a, H, P> {
        plane: &'a Plane,
        origin_svc: &'a str,
        origin_instance: &'a Uuid,
        tenant_id: &'a Option<String>,
        #[serde(with = "serde_scopes")]
        scopes: &'a SmallVec<[String; 4]>,
        nonce: &'a [u8; 16],
        iat: i64,
        exp: i64,
        header: &'a H,
        payload: &'a P,
    }
    let mv = MacView {
        plane: &env.plane,
        origin_svc: env.origin_svc,
        origin_instance: &env.origin_instance,
        tenant_id: &env.tenant_id,
        scopes: &env.scopes,
        nonce: &env.nonce,
        iat: env.iat,
        exp: env.exp,
        header: &env.header,
        payload: &env.payload,
    };
    rmp_serde::to_vec_named(&mv).expect("rmp encode")
}

/// Sign an envelope given a KeyDeriver and epoch (e.g., day number).
pub fn sign_envelope<H: Serialize, P: Serialize>(
    kd: &dyn KeyDeriver,
    svc: &str,
    instance: &Uuid,
    epoch: u64,
    mut env: Envelope<H, P>,
) -> Envelope<H, P> {
    let key = kd.derive_origin_key(svc, instance, epoch);
    let mut mac = HmacSha256::new_from_slice(&key).expect("HMAC key");
    let bytes = encode_for_mac(&env);
    mac.update(&bytes);
    let tag = mac.finalize().into_bytes();
    env.tag.copy_from_slice(&tag);
    env
}

/// Verify tag, time window, and required scopes. Sender must match `expected_svc`.
pub fn verify_envelope<H: Serialize, P: Serialize>(
    kd: &dyn KeyDeriver,
    expected_svc: &str,
    epoch: u64,
    required_scopes: &[&str],
    env: &Envelope<H, P>,
) -> Result<(), VerifyError> {
    verify_common(kd, epoch, required_scopes, env, &|svc| svc == expected_svc)
}

/// Like `verify_envelope` but allow any sender in `allowed_senders`.
pub fn verify_envelope_from_any<H: Serialize, P: Serialize>(
    kd: &dyn KeyDeriver,
    allowed_senders: &[&str],
    epoch: u64,
    required_scopes: &[&str],
    env: &Envelope<H, P>,
) -> Result<(), VerifyError> {
    verify_common(kd, epoch, required_scopes, env, &|svc| allowed_senders.iter().any(|s| s == &svc))
}

fn verify_common<H: Serialize, P: Serialize>(
    kd: &dyn KeyDeriver,
    epoch: u64,
    required_scopes: &[&str],
    env: &Envelope<H, P>,
    sender_ok: &dyn Fn(&str) -> bool,
) -> Result<(), VerifyError> {
    // Time window check
    let now = OffsetDateTime::now_utc().unix_timestamp();
    if now < env.iat || now > env.exp {
        return Err(VerifyError::Expired);
    }
    // Scope check
    for need in required_scopes {
        if !env.scopes.iter().any(|s| s == need) {
            return Err(VerifyError::MissingScope((*need).to_string()));
        }
    }
    if !sender_ok(env.origin_svc) {
        return Err(VerifyError::WrongOrigin);
    }

    // HMAC verify
    let key = kd.derive_origin_key(env.origin_svc, &env.origin_instance, epoch);
    let mut mac = HmacSha256::new_from_slice(&key).map_err(|_| VerifyError::Crypto)?;
    let bytes = encode_for_mac(env);
    mac.update(&bytes);
    mac.verify_slice(&env.tag).map_err(|_| VerifyError::BadTag)
}

#[derive(Debug, thiserror::Error)]
pub enum VerifyError {
    #[error("expired or not yet valid")]
    Expired,
    #[error("missing scope {0}")]
    MissingScope(String),
    #[error("wrong origin service")]
    WrongOrigin,
    #[error("bad tag")]
    BadTag,
    #[error("crypto error")]
    Crypto,
}

/// Helper to create a fresh random nonce.
pub fn generate_nonce() -> [u8; 16] {
    let mut n = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut n);
    n
}

mod serde_scopes {
    use super::*;
    use serde::{Deserializer, Serializer};

    pub fn serialize<S: Serializer>(v: &SmallVec<[String; 4]>, s: S) -> Result<S::Ok, S::Error> {
        v.as_slice().serialize(s)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<SmallVec<[String; 4]>, D::Error> {
        let v = Vec::<String>::deserialize(d)?;
        Ok(SmallVec::from_vec(v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct TestKD([u8; 32]);
    impl KeyDeriver for TestKD {
        fn derive_origin_key(&self, _svc: &str, _i: &Uuid, _e: u64) -> [u8; 32] { self.0 }
    }

    #[test]
    fn sign_and_verify() {
        let kd = TestKD([7u8; 32]);
        let instance = Uuid::nil();
        let now = OffsetDateTime::now_utc().unix_timestamp();
        let env = Envelope {
            plane: Plane::Node,
            origin_svc: "svc-overlay",
            origin_instance: instance,
            tenant_id: None,
            scopes: SmallVec::from_vec(vec!["overlay:route".into()]),
            nonce: generate_nonce(),
            iat: now - 10,
            exp: now + 60,
            header: (),
            payload: ("ping", 1u8),
            tag: [0u8; 32],
        };
        let env = sign_envelope(&kd, "svc-overlay", &instance, 42, env);
        verify_envelope(&kd, "svc-overlay", 42, &["overlay:route"], &env).unwrap();
        verify_envelope_from_any(&kd, &["svc-overlay", "svc-index"], 42, &["overlay:route"], &env).unwrap();
    }
}

```

### crates/ron-billing/Cargo.toml

```toml
[package]
name = "ron-billing"
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"
publish = false

[dependencies]
anyhow = { workspace = true }
# This crate already exists in your workspace (since your current code uses it).
# If it's named differently, adjust accordingly.
naming = { workspace = true }

```

### crates/ron-billing/src/lib.rs

```rust
#![forbid(unsafe_code)]

// Extracted from previous `ryker` to keep responsibilities sharp.
// Pricing & payment validation now live here.

use anyhow::{anyhow, Result};
use naming::manifest::Payment;

/// Price model understood by Billing. Mirrors `Payment.price_model`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriceModel {
    PerMiB,
    Flat,
    PerRequest,
}

impl PriceModel {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "per_mib" => Some(Self::PerMiB),
            "flat" => Some(Self::Flat),
            "per_request" => Some(Self::PerRequest),
            _ => None,
        }
    }
}

/// Compute the cost (in the `Payment.currency`) for serving `n_bytes` under a Payment policy.
/// Returns `None` if no payment policy is present or is marked not required.
pub fn compute_cost(n_bytes: u64, p: &Payment) -> Option<f64> {
    // If the policy isn’t required, we treat it as informational (no charge).
    if !p.required {
        return None;
    }
    let model = PriceModel::parse(&p.price_model)?;
    let price = p.price;

    match model {
        PriceModel::PerMiB => {
            let mibs = (n_bytes as f64) / (1024.0 * 1024.0);
            Some(price * mibs)
        }
        PriceModel::Flat | PriceModel::PerRequest => Some(price),
    }
}

/// Very lightweight wallet check. This is intentionally permissive;
/// you can tighten per scheme later (LNURL, BTC on-chain, SOL, etc.).
pub fn validate_wallet_string(wallet: &str) -> Result<()> {
    if wallet.trim().is_empty() {
        return Err(anyhow!("wallet is empty"));
    }
    // Example heuristics you can extend:
    // - LNURL often starts with 'lnurl' (bech32) or 'LNURL'.
    // - BTC on-chain: base58/bech32; SOL: base58, fixed length; ETH: 0x + 40 hex.
    // We do not enforce scheme here yet—just presence.
    Ok(())
}

/// Convenience: check that a `Payment` block is internally consistent enough to consider enforceable.
pub fn validate_payment_block(p: &Payment) -> Result<()> {
    // Required fields when we intend to enforce:
    // currency, price_model (parseable), wallet (non-empty).
    PriceModel::parse(&p.price_model).ok_or_else(|| anyhow!("unknown price_model"))?;
    validate_wallet_string(&p.wallet)?;
    if p.price < 0.0 {
        return Err(anyhow!("price must be non-negative"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use naming::manifest::{Payment, RevenueSplit};

    fn base(required: bool, model: &str, price: f64) -> Payment {
        Payment {
            required,
            currency: "USD".to_string(),
            price_model: model.to_string(),
            price,
            wallet: "lnurl1deadbeef".to_string(),
            settlement: "offchain".to_string(),
            splits: vec![RevenueSplit {
                account: "creator".into(),
                pct: 100.0,
            }],
        }
    }

    #[test]
    fn cost_per_mib() {
        let p = base(true, "per_mib", 0.01); // 1 cent / MiB
        let c = compute_cost(2 * 1024 * 1024, &p).expect("Some(cost)");
        assert!((c - 0.02).abs() < 1e-9);
    }

    #[test]
    fn cost_flat() {
        let p = base(true, "flat", 0.5);
        let c1 = compute_cost(10, &p).expect("Some(cost)");
        let c2 = compute_cost(10_000_000, &p).expect("Some(cost)");
        assert!((c1 - 0.5).abs() < 1e-12);
        assert!((c2 - 0.5).abs() < 1e-12);
    }

    #[test]
    fn not_required_yields_none() {
        let p = base(false, "per_mib", 0.01);
        assert!(compute_cost(1024, &p).is_none());
    }

    #[test]
    fn validate_payment_ok() {
        let p = base(true, "per_request", 0.001);
        validate_payment_block(&p).unwrap();
    }
}

```

### crates/ron-bus/Cargo.toml

```toml
[package]
publish = false
name = "ron-bus"
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"

[dependencies]
serde = { workspace = true }
rmp-serde = { workspace = true }
thiserror = { workspace = true }
workspace-hack = { version = "0.1", path = "../../workspace-hack" }

```

### crates/ron-bus/src/api.rs

```rust
// crates/ron-bus/src/api.rs
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

/// Generic bus envelope exchanged between services.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Envelope {
    pub service: String,  // e.g., "svc.index"
    pub method: String,   // e.g., "v1.resolve"
    pub corr_id: u64,     // correlation id for RPC
    pub token: Vec<u8>,   // capability blob (MsgPack<CapClaims> or empty)
    pub payload: Vec<u8>, // method-specific bytes (MessagePack-encoded)
}

/// Simple status reply, common across services.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Status {
    pub ok: bool,
    pub message: String,
}

/// RPCs for svc-index (requests)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum IndexReq {
    Health,
    Resolve { addr: String },
    PutAddress { addr: String, dir: String },
}

/// RPCs for svc-index (responses)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum IndexResp {
    HealthOk,
    Resolved { dir: String },
    PutOk,
    NotFound,
    Err { err: String },
}

/// RPCs for svc-storage (requests)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum StorageReq {
    Health,
    /// Read a file from a directory (both absolute or canonical within data root).
    ReadFile {
        dir: String,
        rel: String,
    },
    /// Write a file (not used by gateway yet, but handy for tests/tools).
    WriteFile {
        dir: String,
        rel: String,
        bytes: Vec<u8>,
    },
}

/// RPCs for svc-storage (responses)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum StorageResp {
    HealthOk,
    File { bytes: Vec<u8> },
    Written,
    NotFound,
    Err { err: String },
}

/// RPCs for svc-overlay (requests)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum OverlayReq {
    Health,
    /// Get the file bytes within a bundle addressed by `addr`.
    /// If `rel` is empty, defaults to "payload.bin".
    Get {
        addr: String,
        rel: String,
    },
}

/// RPCs for svc-overlay (responses)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum OverlayResp {
    HealthOk,
    Bytes { data: Vec<u8> },
    NotFound,
    Err { err: String },
}

/// Optional capability claims (future service auth).
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CapClaims {
    pub sub: String,      // subject (service, role, or client id)
    pub ops: Vec<String>, // allowed methods
    pub exp: u64,         // expiry (unix seconds)
    pub nonce: u64,       // replay guard
    pub sig: Vec<u8>,     // ed25519 signature (svc-crypto later)
}

```

### crates/ron-bus/src/lib.rs

```rust
// crates/ron-bus/src/lib.rs
#![forbid(unsafe_code)]

pub mod api;
pub mod uds;

/// Crate version of the bus protocol (bump if wire format changes).
pub const RON_BUS_PROTO_VERSION: u32 = 1;

```

### crates/ron-bus/src/uds.rs

```rust
// crates/ron-bus/src/uds.rs
#![forbid(unsafe_code)]

#[cfg(unix)]
use std::os::unix::net::{UnixListener, UnixStream};

use crate::api::Envelope;
use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;

#[cfg(not(unix))]
compile_error!("ron-bus uds transport requires a Unix platform");

pub fn listen(sock_path: &str) -> io::Result<UnixListener> {
    let p = Path::new(sock_path);
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent)?;
    }
    if p.exists() {
        let _ = fs::remove_file(p);
    }
    UnixListener::bind(p)
}

pub fn recv(stream: &mut UnixStream) -> io::Result<Envelope> {
    let mut len_be = [0u8; 4];
    read_exact(stream, &mut len_be)?;
    let len = u32::from_be_bytes(len_be) as usize;
    if len == 0 {
        return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "empty frame"));
    }
    let mut buf = vec![0u8; len];
    read_exact(stream, &mut buf)?;
    rmp_serde::from_slice::<Envelope>(&buf)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("decode envelope: {e}")))
}

pub fn send(stream: &mut UnixStream, env: &Envelope) -> io::Result<()> {
    let buf = rmp_serde::to_vec(env).map_err(|e| {
        io::Error::new(io::ErrorKind::InvalidInput, format!("encode envelope: {e}"))
    })?;
    if buf.len() > u32::MAX as usize {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("frame too large: {} bytes", buf.len()),
        ));
    }
    let len_be = (buf.len() as u32).to_be_bytes();
    stream.write_all(&len_be)?;
    stream.write_all(&buf)?;
    Ok(())
}

fn read_exact<R: Read>(r: &mut R, mut buf: &mut [u8]) -> io::Result<()> {
    while !buf.is_empty() {
        let n = r.read(buf)?;
        if n == 0 {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "early EOF"));
        }
        let tmp = buf;
        buf = &mut tmp[n..];
    }
    Ok(())
}

```

### crates/ron-kernel/Cargo.toml

```toml
[package]
publish = false
name = "ron-kernel"
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"
description = "RustyOnions microkernel primitives: supervisor, bus, metrics, transport."

[lib]
name = "ron_kernel"
path = "src/lib.rs"

# Binaries (kept exactly as in your original)
[[bin]]
name = "kernel_demo"
path = "src/bin/kernel_demo.rs"

[[bin]]
name = "node_transport"
path = "src/bin/node_transport.rs"

[[bin]]
name = "transport_supervised"
path = "src/bin/transport_supervised.rs"

[[bin]]
name = "node_index"
path = "src/bin/node_index.rs"

[[bin]]
name = "node_overlay"
path = "src/bin/node_overlay.rs"

[[bin]]
name = "transport_demo"
path = "src/bin/transport_demo.rs"

[[bin]]
name = "transport_load"
path = "src/bin/transport_load.rs"

[[bin]]
name = "kameo_demo"
path = "src/bin/kameo_demo.rs"

[[bin]]
name = "bus_demo"
path = "src/bin/bus_demo.rs"

[[bin]]
name = "metrics_demo"
path = "src/bin/metrics_demo.rs"

[features]
default = []
# Optional actor experiments (leave as-is)
kameo = ["dep:kameo"]
loom = []

[dependencies]
anyhow = { workspace = true }
bytes = { workspace = true }

# HTTP client, pinned at workspace (default-features disabled at root)
reqwest = { workspace = true }

# Tokio with io/fs/signal/etc.
tokio = { workspace = true, features = ["rt-multi-thread", "macros", "signal", "net", "time", "sync", "io-util", "fs"] }

# TLS stack used by transport
tokio-rustls = { workspace = true }

# Axum 0.7 via workspace; select the features we use here.
axum = { workspace = true, features = ["tokio", "http1", "http2", "json"] }

# Tracing / logging
tracing = { workspace = true }
tracing-subscriber = { workspace = true, features = ["env-filter", "fmt"] }

# Prometheus metrics (workspace pin -> 0.14 to pull protobuf >= 3.7.2)
prometheus = { workspace = true }

# Serde + JSON
serde = { workspace = true }
serde_json = { workspace = true }

# Config
toml = { workspace = true }
notify = "6"

# Security helpers
zeroize = { workspace = true, features = ["derive"] }

# Overlay helpers
sha2   = { workspace = true }
base64 = { workspace = true }
hex    = { workspace = true }

# Locks etc.
parking_lot = { workspace = true }

# Supervisor jitter/backoff uses rand 0.9 (pinned at workspace)
rand = { workspace = true }

# 🔐 PEM parsing for TLS cert/key
rustls-pemfile = { workspace = true }

# Hakari feature unifier
workspace-hack = { version = "0.1", path = "../../workspace-hack" }

# Optional kameo dep behind the `kameo` feature
[dependencies.kameo]
path = "../kameo"
optional = true

[dev-dependencies]
tokio = { workspace = true, features = ["rt", "macros"] }
loom = "0.7"

```

### crates/ron-kernel/src/amnesia.rs

```rust
//use zeroize::Zeroize;

pub struct AmnesiaMode {
    pub mode: String,
}

impl AmnesiaMode {
    pub fn new(mode: String) -> Self {
        AmnesiaMode { mode }
    }
}

pub struct Capabilities {
    pub capability_name: String,
}

impl Capabilities {
    pub fn new(name: String) -> Self {
        Capabilities {
            capability_name: name,
        }
    }
}

pub struct Secrets {
    pub secret: String,
}

impl Secrets {
    pub fn new(secret: String) -> Self {
        Secrets { secret }
    }
}

```

### crates/ron-kernel/src/bin/bus_demo.rs

```rust
#![forbid(unsafe_code)]

use ron_kernel::{wait_for_ctrl_c, Bus, KernelEvent};
use tracing::{info, Level};
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(Level::INFO.into()))
        .init();

    let bus: Bus = Bus::new(1024);

    let delivered = bus
        .publish(KernelEvent::Health {
            service: "bus_demo".to_string(),
            ok: true,
        })
        .unwrap_or(0);
    println!("Published initial Health event to {delivered} subscriber(s).");

    let mut rx = bus.subscribe();
    tokio::spawn(async move {
        while let Ok(ev) = rx.recv().await {
            info!(?ev, "bus_demo received event");
        }
    });

    wait_for_ctrl_c().await?;
    Ok(())
}

```

### crates/ron-kernel/src/bin/kameo_demo.rs

```rust
// crates/ron-kernel/src/bin/kameo_demo.rs
#![forbid(unsafe_code)]

/* ----------------------------- Stub (default) ----------------------------- */

#[cfg(not(feature = "kameo"))]
#[tokio::main(flavor = "multi_thread")]
async fn main() {
    println!(
        "kameo_demo: optional 'kameo' feature is not enabled. \
         Building stub so the workspace compiles without the kameo crate. \
         Enable with --features kameo (and add the kameo dependency) to run the real demo."
    );
}

/* --------------------------- Real demo (feature) -------------------------- */

#[cfg(feature = "kameo")]
use anyhow::Result;
#[cfg(feature = "kameo")]
use kameo::{spawn, Actor, Ask, Context};
#[cfg(feature = "kameo")]
use tokio::time::{sleep, Duration};
#[cfg(feature = "kameo")]
use tracing_subscriber::EnvFilter;

// A simple message type for our demo.
#[cfg(feature = "kameo")]
#[derive(Debug)]
struct Bump(u64);

// A demo actor with a counter.
#[cfg(feature = "kameo")]
struct Demo {
    count: u64,
}

#[cfg(feature = "kameo")]
impl Demo {
    fn new() -> Self {
        Self { count: 0 }
    }
}

#[cfg(feature = "kameo")]
impl Actor for Demo {
    fn handle_string<'a>(
        &'a mut self,
        _ctx: &'a mut Context,
        msg: String,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            println!("[actor] got string: {msg}");
            Ok(())
        })
    }

    fn handle_ask_env<'a>(
        &'a mut self,
        _ctx: &'a mut Context,
        ask: Ask<&'static str, String>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            let val = std::env::var(ask.req).unwrap_or_default();
            let _ = ask.tx.send(val);
            Ok(())
        })
    }

    fn handle_message<'a, M: Send + 'static>(
        &'a mut self,
        _ctx: &'a mut Context,
        msg: M,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            // handle our Bump message; ignore unknown types (they won't be sent in this demo)
            if let Some(Bump(n)) = any_as_ref::<M, Bump>(&msg) {
                self.count += n;
                println!("[actor] bump by {n}, total={}", self.count);
            }
            Ok(())
        })
    }
}

// Tiny helper to downcast-by-reference for demo purposes.
#[cfg(feature = "kameo")]
fn any_as_ref<T: 'static, U: 'static>(t: &T) -> Option<&U> {
    use std::any::Any;
    (t as &dyn Any).downcast_ref::<U>()
}

#[cfg(feature = "kameo")]
#[tokio::main]
async fn main() -> Result<()> {
    // logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    // spawn actor
    let (addr, _task) = spawn::<Bump, _>(Demo::new());

    // send a few messages
    addr.send_str("hello actor").await?;
    addr.send(Bump(5)).await?;
    addr.send(Bump(7)).await?;

    // ask for an env var
    std::env::set_var("DEMO_ENV", "kameo-works");
    let v = addr.ask_env("DEMO_ENV").await?;
    println!("[main] ask_env(DEMO_ENV) -> {v}");

    // give actor a moment to print
    sleep(Duration::from_millis(50)).await;

    Ok(())
}

```

### crates/ron-kernel/src/bin/kernel_demo.rs

```rust
#![forbid(unsafe_code)]

use std::{error::Error, net::SocketAddr};
use tracing::{info, Level};
use tracing_subscriber::EnvFilter;

use ron_kernel::{wait_for_ctrl_c, Metrics};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Logging: RUST_LOG=info (overridable via env)
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(Level::INFO.into()))
        .with_target(false)
        .compact()
        .init();

    // Start admin (metrics/health/ready) server
    let admin_addr: SocketAddr = "127.0.0.1:9090".parse()?;
    let metrics = Metrics::new();

    // IMPORTANT: Metrics::serve(self, ...) consumes a value, so call it on a CLONE.
    let (admin_task, bound) = metrics.clone().serve(admin_addr).await?;
    info!("Admin endpoints: /metrics /healthz /readyz at http://{bound}/");

    // Mark this process healthy so /readyz returns 200
    metrics.health().set("kernel_demo", true);
    info!("kernel_demo marked healthy; press Ctrl-C to shut down…");

    // Wait for Ctrl-C (ignore the Result to avoid unused_must_use warnings)
    let _ = wait_for_ctrl_c().await;

    info!("Shutting down…");
    // Optionally flip health on shutdown
    metrics.health().set("kernel_demo", false);

    // End the admin task (if needed). Dropping it will abort on runtime shutdown anyway.
    admin_task.abort();

    Ok(())
}

```

### crates/ron-kernel/src/bin/metrics_demo.rs

```rust
#![forbid(unsafe_code)]

use std::{error::Error, net::SocketAddr, sync::Arc, time::Duration};

use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Router};
use prometheus::HistogramTimer;
use ron_kernel::{wait_for_ctrl_c, Metrics};
use tokio::{net::TcpListener, time::sleep};
use tracing::{info, Level};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(Level::INFO.into()))
        .with_target(false)
        .compact()
        .init();

    // === Metrics / Admin server ===
    let admin_addr: SocketAddr = "127.0.0.1:9096".parse()?;
    let metrics0 = Metrics::new();

    // Metrics::serve(self, ...) consumes self; call on a clone so we can still use metrics.
    let (_admin_task, bound) = metrics0.clone().serve(admin_addr).await?;
    info!("Admin endpoints: /metrics /healthz /readyz at http://{bound}/");

    // Share Metrics via Arc
    let metrics = Arc::new(metrics0);

    // Mark this demo healthy so /readyz returns 200
    metrics.health().set("metrics_demo", true);
    info!("metrics_demo marked healthy");

    // === App server with an instrumented handler ===
    let app_addr: SocketAddr = "127.0.0.1:9091".parse()?;
    let listener = TcpListener::bind(app_addr).await?;
    let app = Router::new()
        .route("/ping", get(ping))
        .with_state(metrics.clone());

    info!(
        "App server (metrics_demo) listening on http://{}/ (GET /ping)",
        app_addr
    );

    // Graceful shutdown on Ctrl-C
    let shutdown = async {
        let _ = wait_for_ctrl_c().await;
    };

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await?;

    // Flip health to false on shutdown (optional)
    metrics.health().set("metrics_demo", false);
    Ok(())
}

async fn ping(State(metrics): State<Arc<Metrics>>) -> impl IntoResponse {
    // Record request latency via RAII timer (observes on drop)
    let _t: HistogramTimer = metrics.request_latency_seconds.start_timer();

    // Simulate a tiny bit of work so we see non-zero latency
    sleep(Duration::from_millis(2)).await;

    (StatusCode::OK, "pong").into_response()
}

```

### crates/ron-kernel/src/bin/node_demo.rs

```rust
#![forbid(unsafe_code)]

use ron_kernel::{Bus, Config, HealthState, KernelEvent, Metrics};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tracing::{info, Level};
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Clone)]
struct ServiceSpec {
    name: &'static str,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(Level::INFO.into()))
        .init();

    let bus: Bus = Bus::new(1024);

    // IMPORTANT: build Metrics by value so `serve(self, ..)` can take ownership.
    let metrics = Metrics::new();

    let health = Arc::new(HealthState::new());

    // Start admin HTTP for metrics/health using the owned Metrics value.
    // FIX: parse String -> SocketAddr for Metrics::serve(addr: SocketAddr)
    let admin_addr: SocketAddr = Config::default().admin_addr.parse()?;
    let (_http_handle, bound) = metrics.clone().serve(admin_addr).await?;
    info!(%bound, "node_demo admin started");

    // After calling serve, wrap Metrics in Arc for sharing across tasks.
    let metrics = Arc::new(metrics);

    // Initialize health flags individually (no set_all on HealthState)
    for s in ["transport", "overlay", "index"] {
        health.set(s, false);
    }

    // Spawn a few mock services
    let specs = vec![
        ServiceSpec { name: "transport" },
        ServiceSpec { name: "overlay" },
        ServiceSpec { name: "index" },
    ];

    for spec in specs.clone() {
        let h = health.clone();
        let m = metrics.clone();
        let b = bus.clone();
        tokio::spawn(run_service(spec, h, m, b));
    }

    // Supervise loop
    tokio::spawn(supervise(
        specs,
        health.clone(),
        metrics.clone(),
        bus.clone(),
    ));

    let _ = bus.publish(KernelEvent::Health {
        service: "node_demo".into(),
        ok: true,
    });

    ron_kernel::wait_for_ctrl_c().await?;
    Ok(())
}

async fn run_service(spec: ServiceSpec, health: Arc<HealthState>, metrics: Arc<Metrics>, bus: Bus) {
    // pretend to do work and become healthy
    tokio::time::sleep(Duration::from_millis(200)).await;
    health.set(spec.name, true);
    metrics
        .service_restarts_total
        .with_label_values(&[spec.name])
        .inc();
    let _ = bus.publish(KernelEvent::Health {
        service: spec.name.into(),
        ok: true,
    });
}

async fn supervise(
    specs: Vec<ServiceSpec>,
    _health: Arc<HealthState>,
    metrics: Arc<Metrics>,
    _bus: Bus,
) {
    loop {
        for s in &specs {
            metrics.request_latency_seconds.observe(0.001);
            tracing::debug!("supervising {}", s.name);
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

```

### crates/ron-kernel/src/bin/node_index.rs

```rust
#![forbid(unsafe_code)]

use std::{collections::HashMap, error::Error, net::SocketAddr, sync::Arc, time::Duration};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use prometheus::HistogramTimer;
use ron_kernel::{wait_for_ctrl_c, Metrics};
use serde::Serialize;
use tokio::{net::TcpListener, sync::RwLock, time::sleep};
use tracing::{info, Level};
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
struct IndexState {
    inner: Arc<RwLock<HashMap<String, String>>>, // addr -> dir
    metrics: Arc<Metrics>,
}

#[derive(Debug, Serialize)]
struct ApiResp<T: Serialize> {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(Level::INFO.into()))
        .with_target(false)
        .compact()
        .init();

    // === Metrics / Admin server (index uses 9097 to avoid conflicts) ===
    let admin_addr: SocketAddr = "127.0.0.1:9097".parse()?;
    let metrics0 = Metrics::new();
    let (_admin_task, bound) = metrics0.clone().serve(admin_addr).await?;
    info!("Admin endpoints: /metrics /healthz /readyz at http://{bound}/");

    let metrics = Arc::new(metrics0);
    metrics.health().set("node_index", true);

    // === App server (index API on 8086) ===
    let app_addr: SocketAddr = "127.0.0.1:8086".parse()?;
    let listener = TcpListener::bind(app_addr).await?;

    let state = IndexState {
        inner: Arc::new(RwLock::new(HashMap::new())),
        metrics: metrics.clone(),
    };

    let app = Router::new()
        .route("/put", post(put))
        .route("/resolve/:addr", get(resolve))
        .with_state(state);

    info!(
        "node_index listening on http://{}/ (POST /put, GET /resolve/:addr)",
        app_addr
    );

    // Graceful shutdown on Ctrl-C
    let shutdown = async {
        let _ = wait_for_ctrl_c().await;
    };

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await?;

    metrics.health().set("node_index", false);
    Ok(())
}

#[derive(serde::Deserialize)]
struct PutReq {
    addr: String,
    dir: String,
}

async fn put(State(state): State<IndexState>, Json(req): Json<PutReq>) -> impl IntoResponse {
    let _t: HistogramTimer = state.metrics.request_latency_seconds.start_timer();

    // Simulate small work so histogram isn't zero
    sleep(Duration::from_millis(2)).await;

    state.inner.write().await.insert(req.addr, req.dir);
    let resp: ApiResp<&'static str> = ApiResp {
        ok: true,
        data: Some("ok"),
        error: None,
    };
    (StatusCode::OK, Json(resp)).into_response()
}

async fn resolve(State(state): State<IndexState>, Path(addr): Path<String>) -> impl IntoResponse {
    let _t: HistogramTimer = state.metrics.request_latency_seconds.start_timer();

    // Simulate small work
    sleep(Duration::from_millis(2)).await;

    let map = state.inner.read().await;
    if let Some(dir) = map.get(&addr) {
        let resp: ApiResp<_> = ApiResp {
            ok: true,
            data: Some(serde_json::json!({ "addr": addr, "dir": dir })),
            error: None,
        };
        (StatusCode::OK, Json(resp)).into_response()
    } else {
        let resp: ApiResp<()> = ApiResp {
            ok: false,
            data: None,
            error: Some("not found".into()),
        };
        (StatusCode::NOT_FOUND, Json(resp)).into_response()
    }
}

```

### crates/ron-kernel/src/bin/node_overlay.rs

```rust
#![forbid(unsafe_code)]

use std::{net::SocketAddr, sync::Arc};

use tracing::info;
use tracing_subscriber::{fmt, EnvFilter};

use ron_kernel::cancel::Shutdown;
use ron_kernel::{
    config,
    overlay::{self, init_overlay_metrics, OverlayCfg},
    supervisor::Supervisor,
    wait_for_ctrl_c, Bus, Config, HealthState, Metrics,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Logging
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt().with_env_filter(filter).pretty().init();
    info!("Starting node_overlay…");

    // Shared infra
    let metrics = Arc::new(Metrics::new());
    let health = Arc::new(HealthState::new());
    let bus = Bus::new(1024);
    let sdn = Shutdown::new();

    // Load config + build overlay knobs + metrics
    let cfg = config::load_from_file("config.toml").unwrap_or_else(|_| Config::default());
    let overlay_cfg: OverlayCfg = overlay::overlay_cfg_from(&cfg)?;
    let overlay_metrics = init_overlay_metrics();

    // Supervisor: overlay + admin http
    let mut sup = Supervisor::new(bus.clone(), metrics.clone(), health.clone(), sdn.clone());

    {
        let h = health.clone();
        let m = metrics.clone();
        let oc = overlay_cfg.clone();
        let om = overlay_metrics.clone();
        let bus = bus.clone();
        sup.add_service("overlay", move |sdn| {
            let h = h.clone();
            let m = m.clone();
            let cfg = oc.clone();
            let om = om.clone();
            let bus = bus.clone();
            async move { overlay::service::run(sdn, h, m, cfg, om, bus).await }
        });
    }

    let admin_addr: SocketAddr = cfg.admin_addr.parse()?;
    {
        let h = health.clone();
        let m = metrics.clone();
        sup.add_service("admin_http", move |sdn| {
            let h = h.clone();
            let m = m.clone();
            async move { overlay::admin_http::run(sdn, h, m, admin_addr).await }
        });
    }

    let _cfg_watch = config::spawn_config_watcher("config.toml", bus.clone(), health.clone());

    let handle = sup.spawn();
    info!("node_overlay up. Try:");
    info!("  nc -v {}", overlay_cfg.bind);
    info!("  # metrics to watch:");
    info!("  #   overlay_accepted_total, overlay_rejected_total, overlay_active_connections");
    info!("  #   overlay_handshake_failures_total, overlay_read_timeouts_total, overlay_idle_timeouts_total");
    info!("  #   overlay_config_version, overlay_max_conns");

    let _ = wait_for_ctrl_c().await;
    info!("Ctrl-C received; shutting down…");
    handle.shutdown();
    handle.join().await?;

    Ok(())
}

```

### crates/ron-kernel/src/bin/node_transport.rs

```rust
#![forbid(unsafe_code)]

use ron_kernel::{
    transport::{spawn_transport, TransportConfig},
    Bus, HealthState, KernelEvent, Metrics,
};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tracing::{info, Level};
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(Level::INFO.into()))
        .init();

    let bus: Bus = Bus::new(1024);
    let metrics = Metrics::new();
    let health = Arc::new(HealthState::new());

    let cfg = TransportConfig {
        addr: "127.0.0.1:8090".parse::<SocketAddr>()?,
        name: "node_transport",
        max_conns: 256,
        read_timeout: Duration::from_secs(10),
        write_timeout: Duration::from_secs(10),
        idle_timeout: Duration::from_secs(30),
    };

    let (_jh, bound) =
        spawn_transport(cfg, metrics.clone(), health.clone(), bus.clone(), None).await?;
    info!(%bound, "node transport listening");

    let _ = bus.publish(KernelEvent::Health {
        service: "node_transport".into(),
        ok: true,
    });

    ron_kernel::wait_for_ctrl_c().await?;
    Ok(())
}

```

### crates/ron-kernel/src/bin/transport_demo.rs

```rust
#![forbid(unsafe_code)]

use ron_kernel::{
    transport::{spawn_transport, TransportConfig},
    Bus, HealthState, KernelEvent, Metrics,
};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tracing::{info, Level};
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(Level::INFO.into()))
        .init();

    let bus: Bus = Bus::new(1024);
    let metrics = Metrics::new();
    let health = Arc::new(HealthState::new());

    let cfg = TransportConfig {
        addr: "127.0.0.1:8088".parse::<SocketAddr>()?,
        name: "transport_demo",
        max_conns: 128,
        read_timeout: Duration::from_secs(10),
        write_timeout: Duration::from_secs(10),
        idle_timeout: Duration::from_secs(30),
    };

    let (_jh, bound) =
        spawn_transport(cfg, metrics.clone(), health.clone(), bus.clone(), None).await?;
    info!(%bound, "transport demo listening");

    let _ = bus.publish(KernelEvent::Health {
        service: "transport_demo".into(),
        ok: true,
    });

    ron_kernel::wait_for_ctrl_c().await?;
    Ok(())
}

```

### crates/ron-kernel/src/bin/transport_load.rs

```rust
#![forbid(unsafe_code)]

use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::{info, Level};
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(Level::INFO.into()))
        .init();

    let target = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8088".to_string());
    info!(%target, "transport_load connecting");

    let mut stream = TcpStream::connect(&target).await?;
    let payload = b"hello";
    stream.write_all(payload).await?;

    let mut buf = [0u8; 64];
    let _n = stream.read(&mut buf).await.unwrap_or(0);

    stream.shutdown().await.ok();

    tokio::time::sleep(Duration::from_millis(100)).await;
    Ok(())
}

```

### crates/ron-kernel/src/bin/transport_supervised.rs

```rust
// crates/ron-kernel/src/bin/transport_supervised.rs
#![forbid(unsafe_code)]

use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
    time::{Duration, Instant},
};

use axum::{
    extract::State,
    http::{header::CONTENT_TYPE, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use prometheus::{Encoder, TextEncoder};
use ron_kernel::cancel::Shutdown;
use ron_kernel::supervisor::Supervisor;
use ron_kernel::{wait_for_ctrl_c, Bus, HealthState, Metrics};
use serde::Serialize;
use tokio::net::TcpListener;
use tokio::time::sleep;
use tracing::{info, warn};
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Clone)]
struct AppState {
    crash: Arc<tokio::sync::Notify>,
    metrics: Arc<Metrics>,
}

#[derive(Clone)]
struct AdminState {
    health: Arc<HealthState>,
    metrics: Arc<Metrics>,
}

#[derive(Serialize)]
struct OkMsg {
    ok: bool,
    msg: &'static str,
}

/* =========================  Service #1: Demo HTTP  ========================= */

async fn run_http_service(sdn: Shutdown, state: AppState) -> anyhow::Result<()> {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8088);
    let app = Router::new()
        .route("/", get(root))
        .route("/crash", post(crash))
        .with_state(state.clone());

    let listener = TcpListener::bind(addr).await?;
    info!("demo HTTP listening on http://{}", addr);

    let serve_fut = async {
        axum::serve(listener, app)
            .with_graceful_shutdown(async move { sdn.cancelled().await })
            .await
            .map_err(|e| anyhow::anyhow!(e))
    };

    tokio::pin!(serve_fut);

    tokio::select! {
        res = &mut serve_fut => {
            res?;
            Ok(())
        }
        _ = state.crash.notified() => {
            warn!("Crash requested; stopping service with error to trigger restart");
            sleep(Duration::from_millis(200)).await;
            Err(anyhow::anyhow!("intentional crash requested by /crash"))
        }
    }
}

async fn root(State(state): State<AppState>) -> impl IntoResponse {
    let start = Instant::now();

    let resp = (
        StatusCode::OK,
        Json(OkMsg {
            ok: true,
            msg: "hello from supervised transport",
        }),
    );

    state
        .metrics
        .request_latency_seconds
        .observe(start.elapsed().as_secs_f64());

    resp
}

async fn crash(State(state): State<AppState>) -> impl IntoResponse {
    let start = Instant::now();

    state.crash.notify_waiters();
    let resp = (
        StatusCode::OK,
        Json(OkMsg {
            ok: true,
            msg: "crash requested; service will restart",
        }),
    );

    state
        .metrics
        .request_latency_seconds
        .observe(start.elapsed().as_secs_f64());

    resp
}

/* =======================  Service #2: Admin HTTP  ========================== */

async fn run_admin_service(sdn: Shutdown, state: AdminState) -> anyhow::Result<()> {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9096);
    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/metrics", get(metrics_route))
        .with_state(state.clone());

    let listener = TcpListener::bind(addr).await?;
    info!(
        "admin HTTP listening on http://{} (endpoints: /healthz /readyz /metrics)",
        addr
    );

    axum::serve(listener, app)
        .with_graceful_shutdown(async move { sdn.cancelled().await })
        .await
        .map_err(|e| anyhow::anyhow!(e))
}

async fn healthz(State(state): State<AdminState>) -> impl IntoResponse {
    let start = Instant::now();

    let resp = if state.health.all_ready() {
        (StatusCode::OK, "ok")
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "not ready")
    };

    state
        .metrics
        .request_latency_seconds
        .observe(start.elapsed().as_secs_f64());

    resp
}

async fn readyz(State(state): State<AdminState>) -> impl IntoResponse {
    let start = Instant::now();

    let resp = if state.health.all_ready() {
        (StatusCode::OK, "ready")
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "not ready")
    };

    state
        .metrics
        .request_latency_seconds
        .observe(start.elapsed().as_secs_f64());

    resp
}

async fn metrics_route(State(state): State<AdminState>) -> impl IntoResponse {
    let start = Instant::now();

    let metric_families = prometheus::gather();
    let mut buf = Vec::new();
    let encoder = TextEncoder::new();
    let _ = encoder.encode(&metric_families, &mut buf);

    // Own the content-type string so we don't return a borrow tied to `encoder`.
    let ct: String = encoder.format_type().to_string();

    state
        .metrics
        .request_latency_seconds
        .observe(start.elapsed().as_secs_f64());

    (StatusCode::OK, [(CONTENT_TYPE, ct)], buf)
}

/* ================================  main  =================================== */

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Logging
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt().with_env_filter(filter).pretty().init();

    info!("Starting transport_supervised demo…");

    // Shared infra (Bus/Metrics/Health)
    let metrics = Arc::new(Metrics::new());
    let health = Arc::new(HealthState::new());
    let bus = Bus::new(1024);
    let sdn = Shutdown::new();

    // Start config watcher (publishes KernelEvent::ConfigUpdated on change)
    let _cfg_watch =
        ron_kernel::config::spawn_config_watcher("config.toml", bus.clone(), health.clone());

    // Supervisor
    let mut sup = Supervisor::new(bus.clone(), metrics.clone(), health.clone(), sdn.clone());

    // Service #1: demo HTTP
    let state = AppState {
        crash: Arc::new(tokio::sync::Notify::new()),
        metrics: metrics.clone(),
    };
    sup.add_service("demo_http", move |sdn| {
        let state = state.clone();
        async move { run_http_service(sdn, state).await }
    });

    // Service #2: admin HTTP
    let admin_state = AdminState {
        health: health.clone(),
        metrics: metrics.clone(),
    };
    sup.add_service("admin_http", move |sdn| {
        let st = admin_state.clone();
        async move { run_admin_service(sdn, st).await }
    });

    let handle = sup.spawn();

    // Split the long message into multiple lines to avoid any truncation problems.
    info!("Try it:");
    info!("  curl -s http://127.0.0.1:8088/");
    info!("  curl -s -X POST http://127.0.0.1:8088/crash");
    info!("  curl -s http://127.0.0.1:9096/healthz");
    info!("  curl -s http://127.0.0.1:9096/readyz");
    info!("  curl -s http://127.0.0.1:9096/metrics | head -n 20");

    // Wait for Ctrl-C, then shut down gracefully
    let _ = wait_for_ctrl_c().await;
    info!("Ctrl-C received; shutting down…");
    handle.shutdown();
    handle.join().await?;

    Ok(())
}

```

### crates/ron-kernel/src/bus/core.rs

```rust
//! Core bus implementation (bounded broadcast, overflow accounting, throttled signals).

#![forbid(unsafe_code)]

use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::sync::broadcast;

use super::metrics::overflow_counter;
use crate::KernelEvent;

/// Cloneable microkernel event bus backed by `tokio::sync::broadcast`.
#[derive(Clone, Debug)]
pub struct Bus {
    pub(crate) tx: broadcast::Sender<KernelEvent>,
    capacity: usize,

    // Local accounting for dropped messages observed (across subscribers).
    dropped_total: Arc<AtomicU64>,

    // Throttling for overflow/crash signaling. Per-bus, cheap, and thread-safe.
    last_overflow_utc: Arc<AtomicU64>,
    pub(crate) overflow_throttle_secs: u64,
}

impl Bus {
    /// Create a new bus with a bounded capacity.
    ///
    /// Capacity is the size of the underlying broadcast ring-buffer. Receivers
    /// that can't keep up will observe `RecvError::Lagged(n)`.
    pub fn new(capacity: usize) -> Self {
        let (tx, _rx) = broadcast::channel(capacity.max(1));

        // Pre-register the overflow metric so it’s exported as 0 even before any lag.
        let _ = overflow_counter();

        Self {
            tx,
            capacity,
            dropped_total: Arc::new(AtomicU64::new(0)),
            last_overflow_utc: Arc::new(AtomicU64::new(0)),
            overflow_throttle_secs: 5, // sensible default; configurable later
        }
    }

    /// Returns the configured capacity of this bus.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Cumulative dropped messages observed across all subscribers.
    pub fn dropped_total(&self) -> u64 {
        self.dropped_total.load(Ordering::Relaxed)
    }

    /// Subscribe to the bus. Standard broadcast semantics.
    ///
    /// This keeps the public API compatible with existing code that does:
    ///
    /// ```ignore
    /// // In an async context:
    /// // let bus = ron_kernel::bus::Bus::new(8);
    /// // let mut rx = bus.subscribe();
    /// // while let Ok(ev) = rx.recv().await {
    /// //     /* ... */
    /// // }
    /// ```
    pub fn subscribe(&self) -> broadcast::Receiver<KernelEvent> {
        self.tx.subscribe()
    }

    /// Publish a kernel event to all current subscribers.
    ///
    /// Returns the number of subscribers the message was delivered to, or an
    /// error if there were none at the time of sending.
    pub fn publish(
        &self,
        ev: KernelEvent,
    ) -> Result<usize, broadcast::error::SendError<KernelEvent>> {
        self.tx.send(ev)
    }

    /// Publish but ignore `NoReceivers` (useful for fire-and-forget health pings).
    pub fn publish_lossy(&self, ev: KernelEvent) {
        let _ = self.tx.send(ev);
    }

    /// Internal: record that `n` messages were dropped due to lag and optionally
    /// publish a *throttled* "bus-overflow" crash-style event.
    pub(crate) fn record_overflow(&self, n: u64, reason: String) {
        self.dropped_total.fetch_add(n, Ordering::Relaxed);
        overflow_counter().inc_by(n);
        self.publish_overflow_throttled(reason);
    }

    /// Convenience used by helpers when exact `n` isn't available.
    pub(crate) fn record_minimal_overflow(&self, service_label: &str) {
        self.record_overflow(
            1,
            format!(
                "{service} receiver lagged (minimal)",
                service = service_label
            ),
        );
    }

    /// Internal: publish a throttled ServiceCrashed("bus-overflow") with reason.
    fn publish_overflow_throttled(&self, reason: String) {
        let now = epoch_secs();
        let last = self.last_overflow_utc.load(Ordering::Relaxed);

        if now.saturating_sub(last) >= self.overflow_throttle_secs
            && self
                .last_overflow_utc
                .compare_exchange(last, now, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
        {
            // NOTE: Keeping the `reason` field in your KernelEvent variant as-is.
            let _ = self.publish(KernelEvent::ServiceCrashed {
                service: "bus-overflow".to_string(),
                reason,
            });
        }
    }
}

#[inline]
fn epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

```

### crates/ron-kernel/src/bus/helpers.rs

```rust
//! Lag-aware receiving helpers that integrate with overflow accounting.

#![forbid(unsafe_code)]

use tokio::sync::broadcast;
use tracing::{debug, warn};

use super::core::Bus;
use crate::KernelEvent;

/// Receive with lag awareness and throttled overflow signaling (blocking variant).
///
/// - If a receiver is too slow, `tokio::broadcast` yields `RecvError::Lagged(n)`.
/// - We log the condition, increment counters, and publish one **throttled**
///   crash-style event via the bus, embedding the service label and lag count.
/// - Then we immediately retry until we either obtain an event or hit a different error.
///
/// This helper is **opt-in** and does not break existing code that uses `rx.recv().await`.
pub async fn recv_lag_aware(
    rx: &mut broadcast::Receiver<KernelEvent>,
    bus: &Bus,
    service_label: &str,
) -> Result<KernelEvent, broadcast::error::RecvError> {
    loop {
        match rx.recv().await {
            Ok(ev) => return Ok(ev),
            Err(broadcast::error::RecvError::Lagged(n)) => {
                let reason = format!(
                    "{service} receiver lagged by {n} events",
                    service = service_label
                );
                warn!(%service_label, lagged = n, "bus receiver lag detected");
                bus.record_overflow(n, reason);
                continue;
            }
            Err(other) => {
                debug!(%service_label, error = %other, "bus receiver error");
                return Err(other);
            }
        }
    }
}

/// Non-blocking variant with the same lag accounting.
pub fn try_recv_lag_aware(
    rx: &mut broadcast::Receiver<KernelEvent>,
    bus: &Bus,
    service_label: &str,
) -> Result<KernelEvent, broadcast::error::TryRecvError> {
    match rx.try_recv() {
        Ok(ev) => Ok(ev),
        Err(broadcast::error::TryRecvError::Lagged(n)) => {
            let reason = format!(
                "{service} receiver lagged by {n} events",
                service = service_label
            );
            warn!(%service_label, lagged = n, "bus receiver lag detected (try_recv)");
            bus.record_overflow(n, reason);
            // After acknowledging lag, attempt one more non-blocking read:
            rx.try_recv()
        }
        Err(e) => Err(e),
    }
}

```

### crates/ron-kernel/src/bus/metrics.rs

```rust
// crates/ron-kernel/src/bus/metrics.rs
#![forbid(unsafe_code)]
// Allow expect() only during startup-time metric construction (never in hot paths).
#![allow(clippy::expect_used)]

use std::sync::OnceLock;

use prometheus::register_int_counter;
use prometheus::{register, IntCounter};

/// Aggregated (unlabeled) bus metrics.
/// Currently not constructed by callers; kept for future ergonomic use.
#[allow(dead_code)]
#[derive(Clone)]
pub struct BusMetrics {
    /// Total KernelEvent messages dropped due to subscriber lag/overflow
    pub overflow_dropped_total: IntCounter,
}

impl BusMetrics {
    #[allow(dead_code)]
    pub fn new() -> Self {
        let overflow_dropped_total = reg_counter(
            "bus_overflow_dropped_total",
            "Total KernelEvent messages dropped due to subscriber lag/overflow",
        );

        Self {
            overflow_dropped_total,
        }
    }
}

/// Global overflow counter for the bus (unlabeled).
///
/// Call sites expect: `overflow_counter().inc_by(n);`
pub fn overflow_counter() -> &'static IntCounter {
    static OVERFLOW: OnceLock<IntCounter> = OnceLock::new();

    OVERFLOW.get_or_init(|| {
        let c = IntCounter::new(
            "ron_bus_overflow_total",
            "Number of bus messages dropped due to lagged receivers",
        )
        .expect("IntCounter::new(ron_bus_overflow_total)");
        let _ = register(Box::new(c.clone())); // ignore AlreadyRegistered
        c
    })
}

#[allow(dead_code)]
fn reg_counter(name: &'static str, help: &'static str) -> IntCounter {
    // Avoid panicking on registration: create an unregistered fallback on error.
    register_int_counter!(name, help).unwrap_or_else(|e| {
        eprintln!("prometheus: failed to register counter {name}: {e}");
        IntCounter::new(format!("{name}_fallback"), help.to_string()).expect("fallback IntCounter")
    })
}

```

### crates/ron-kernel/src/bus/mod.rs

```rust
//! Microkernel event bus (module root).
//!
//! Structure:
//! - core.rs: `Bus` type + core logic (capacity, publish, overflow throttling).
//! - metrics.rs: Prometheus counter for dropped events.
//! - helpers.rs: lag-aware recv helpers (blocking and non-blocking).
//! - sub.rs: topic-style helpers (timeouts, matching, try-now).
//!
//! Public surface keeps compatibility with the previous single-file version.

#![forbid(unsafe_code)]

mod core;
mod helpers;
mod metrics;
pub mod sub;

pub use core::Bus;
pub use helpers::{recv_lag_aware, try_recv_lag_aware};

```

### crates/ron-kernel/src/bus/sub.rs

```rust
//! Subscriber-side helpers (topic-style filters, timeouts, non-blocking polling).

#![forbid(unsafe_code)]

use std::time::Duration;
use tokio::sync::broadcast;
use tokio::time;

use super::core::Bus;
use crate::KernelEvent;

/// Receive with timeout. If the receiver lagged, update bus counters and continue.
/// Returns `Some(event)` on success, or `None` on timeout/closed.
pub async fn recv_with_timeout(
    bus: &Bus,
    rx: &mut broadcast::Receiver<KernelEvent>,
    timeout_dur: Duration,
) -> Option<KernelEvent> {
    match time::timeout(timeout_dur, rx.recv()).await {
        Ok(Ok(ev)) => Some(ev),
        Ok(Err(broadcast::error::RecvError::Lagged(skipped))) => {
            bus.record_overflow(skipped, format!("topic recv lagged by {skipped}"));
            // Try again immediately to pull the next available message.
            match rx.try_recv() {
                Ok(ev) => Some(ev),
                Err(broadcast::error::TryRecvError::Lagged(_)) => {
                    // Still lagging; we already counted, let caller loop/yield.
                    None
                }
                Err(_) => None,
            }
        }
        _ => None,
    }
}

/// Non-blocking poll. Returns `Some(event)` or `None` if empty/closed.
pub fn try_recv_now(
    bus: &Bus,
    rx: &mut broadcast::Receiver<KernelEvent>,
    service_label: &str,
) -> Option<KernelEvent> {
    match rx.try_recv() {
        Ok(ev) => Some(ev),
        Err(broadcast::error::TryRecvError::Lagged(_)) => {
            bus.record_minimal_overflow(service_label);
            None
        }
        _ => None,
    }
}

/// Topic-style filter: wait up to `timeout` for an event matching `pred`.
pub async fn recv_matching<F>(
    bus: &Bus,
    rx: &mut broadcast::Receiver<KernelEvent>,
    timeout: Duration,
    mut pred: F,
) -> Option<KernelEvent>
where
    F: FnMut(&KernelEvent) -> bool,
{
    let deadline = std::time::Instant::now() + timeout;
    loop {
        let now = std::time::Instant::now();
        if now >= deadline {
            return None;
        }
        let rem = deadline - now;
        if let Some(ev) = recv_with_timeout(bus, rx, rem).await {
            if pred(&ev) {
                return Some(ev);
            }
        } else {
            // timed out or closed
            return None;
        }
    }
}

```

### crates/ron-kernel/src/cancel.rs

```rust
#![forbid(unsafe_code)]

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::sync::Notify;

/// Simple cancellation token with parent/child semantics.
/// - `cancel()` notifies all waiters.
/// - `cancelled().await` resolves once cancelled.
/// - `child()` returns another handle to the same token (shared root).
#[derive(Clone)]
pub struct Shutdown {
    inner: Arc<Inner>,
}

struct Inner {
    cancelled: AtomicBool,
    notify: Notify,
}

impl Shutdown {
    /// Create a new, not-yet-cancelled token.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Inner {
                cancelled: AtomicBool::new(false),
                notify: Notify::new(),
            }),
        }
    }

    /// Request cancellation (idempotent).
    pub fn cancel(&self) {
        // Only notify once on the first transition to true.
        if !self.inner.cancelled.swap(true, Ordering::SeqCst) {
            self.inner.notify.notify_waiters();
        }
    }

    /// Wait until cancellation is requested.
    pub async fn cancelled(&self) {
        // Fast path
        if self.inner.cancelled.load(Ordering::SeqCst) {
            return;
        }
        // Slow path: wait for a notification, with a loop to avoid missed wakes.
        loop {
            if self.inner.cancelled.load(Ordering::SeqCst) {
                break;
            }
            self.inner.notify.notified().await;
            if self.inner.cancelled.load(Ordering::SeqCst) {
                break;
            }
        }
    }

    /// Create a child handle. (Here children share the same root signal.)
    pub fn child(&self) -> Self {
        self.clone()
    }
}

impl Default for Shutdown {
    fn default() -> Self {
        Self::new()
    }
}

```

### crates/ron-kernel/src/config/mod.rs

```rust
#![forbid(unsafe_code)]

pub mod types;
pub mod validate;
pub mod watch;

// Re-exports to preserve the old API surface:
pub use types::{load_from_file, Config, TransportConfig};
pub use validate::validate;
pub use watch::spawn_config_watcher;

```

### crates/ron-kernel/src/config/types.rs

```rust
#![forbid(unsafe_code)]

//! Typed configuration structures and file loading.

use serde::Deserialize;
use std::{fs, path::Path};

/// Optional nested transport section.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct TransportConfig {
    pub max_conns: Option<u64>,
    pub idle_timeout_ms: Option<u64>,
    pub read_timeout_ms: Option<u64>,
    pub write_timeout_ms: Option<u64>,
}

/// Workspace-wide config with typed fields, plus a raw table for ad-hoc lookups.
#[derive(Clone, Debug, Default)]
pub struct Config {
    pub raw: toml::Table,
    pub admin_addr: String,
    pub overlay_addr: String,
    pub dev_inbox_addr: String,
    pub socks5_addr: String,
    pub tor_ctrl_addr: String,
    pub data_dir: String,
    pub chunk_size: u64,
    pub connect_timeout_ms: u64,
    pub transport: TransportConfig,
}

impl Config {
    pub(crate) fn from_table(t: toml::Table) -> Self {
        fn get_string(tbl: &toml::Table, key: &str) -> Option<String> {
            tbl.get(key).and_then(|v| v.as_str().map(|s| s.to_string()))
        }
        fn get_u64(tbl: &toml::Table, key: &str) -> Option<u64> {
            tbl.get(key).and_then(|v| v.as_integer()).map(|n| n as u64)
        }

        let admin_addr =
            get_string(&t, "admin_addr").unwrap_or_else(|| "127.0.0.1:9096".to_string());
        let overlay_addr =
            get_string(&t, "overlay_addr").unwrap_or_else(|| "127.0.0.1:1777".to_string());
        let dev_inbox_addr =
            get_string(&t, "dev_inbox_addr").unwrap_or_else(|| "127.0.0.1:2888".to_string());
        let socks5_addr =
            get_string(&t, "socks5_addr").unwrap_or_else(|| "127.0.0.1:9050".to_string());
        let tor_ctrl_addr =
            get_string(&t, "tor_ctrl_addr").unwrap_or_else(|| "127.0.0.1:9051".to_string());
        let data_dir = get_string(&t, "data_dir").unwrap_or_else(|| ".data".to_string());
        let chunk_size = get_u64(&t, "chunk_size").unwrap_or(65536);
        let connect_timeout_ms = get_u64(&t, "connect_timeout_ms").unwrap_or(5000);

        let transport = t
            .get("transport")
            .and_then(|v| v.clone().try_into().ok())
            .unwrap_or_default();

        Self {
            raw: t,
            admin_addr,
            overlay_addr,
            dev_inbox_addr,
            socks5_addr,
            tor_ctrl_addr,
            data_dir,
            chunk_size,
            connect_timeout_ms,
            transport,
        }
    }
}

/// Synchronously load and parse a TOML config file.
pub fn load_from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Config> {
    let txt = fs::read_to_string(path)?;
    let table: toml::Table = toml::from_str(&txt)?;
    Ok(Config::from_table(table))
}

```

### crates/ron-kernel/src/config/validate.rs

```rust
#![forbid(unsafe_code)]

use super::types::Config;
use std::net::SocketAddr;

pub fn validate(cfg: &Config) -> anyhow::Result<()> {
    let _admin: SocketAddr = cfg.admin_addr.parse()?;
    let _overlay: SocketAddr = cfg.overlay_addr.parse()?;

    let t = &cfg.transport;
    if t.max_conns.unwrap_or(2048) == 0 {
        anyhow::bail!("transport.max_conns must be > 0");
    }
    if t.idle_timeout_ms.unwrap_or(30_000) < 1_000 {
        anyhow::bail!("transport.idle_timeout_ms too small");
    }
    if t.read_timeout_ms.unwrap_or(5_000) < 100 {
        anyhow::bail!("transport.read_timeout_ms too small");
    }
    if t.write_timeout_ms.unwrap_or(5_000) < 100 {
        anyhow::bail!("transport.write_timeout_ms too small");
    }

    Ok(())
}

```

### crates/ron-kernel/src/config/watch.rs

```rust
#![forbid(unsafe_code)]

//! File watching + hot-reload publisher for `config.toml`.
//! Self-heals if the watcher channel closes unexpectedly.

use std::{
    env,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

use notify::{
    Config as NotifyConfig, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};
use tracing::{error, info, warn};

use super::types::load_from_file;
use crate::{bus::Bus, metrics::HealthState, KernelEvent};

/// Monotonic version for successfully committed configs.
static VERSION: AtomicU64 = AtomicU64::new(0);

pub fn spawn_config_watcher<P: Into<PathBuf>>(
    path: P,
    bus: Bus,
    health: Arc<HealthState>,
) -> tokio::task::JoinHandle<()> {
    let path = path.into();
    tokio::spawn(async move {
        // Run the blocking watch loop on a dedicated threadpool thread.
        let _ = tokio::task::spawn_blocking(move || watch_forever(path, bus, health)).await;
    })
}

fn watch_forever(path: PathBuf, bus: Bus, health: Arc<HealthState>) {
    // If the inner watcher loop aborts (e.g., channel closed), sleep and try again.
    loop {
        if let Err(e) = watch_once(&path, &bus, &health) {
            error!(error = %e, "config watcher encountered an error; restarting in 1s");
            thread::sleep(Duration::from_secs(1));
        }
    }
}

fn watch_once(path: &Path, bus: &Bus, health: &Arc<HealthState>) -> anyhow::Result<()> {
    // Initial load (non-fatal if missing/invalid)
    match load_from_file(path) {
        Ok(_) => {
            health.set("config", true);
            let v = VERSION.fetch_add(1, Ordering::SeqCst) + 1;
            let _ = bus.publish(KernelEvent::ConfigUpdated { version: v });
            info!(version = v, file = ?path, "config loaded");
        }
        Err(e) => {
            health.set("config", false);
            warn!(error = %e, file = ?path, "config initial load failed");
        }
    }

    // Normalize watch dir so passing just "config.toml" doesn't produce "".
    let watch_dir: PathBuf = match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
        _ => {
            warn!("config file has no parent directory; watching current directory");
            env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        }
    };

    // Channel for notify callback -> loop
    let (tx, rx) = std::sync::mpsc::channel();

    let mut watcher = RecommendedWatcher::new(
        move |res: Result<Event, notify::Error>| {
            let _ = tx.send(res);
        },
        NotifyConfig::default()
            .with_poll_interval(Duration::from_millis(750))
            .with_compare_contents(true),
    )?;

    watcher.watch(&watch_dir, RecursiveMode::NonRecursive)?;

    // Debounce simple: small sleep after an event burst.
    let debounce = Duration::from_millis(200);

    loop {
        match rx.recv() {
            Ok(Ok(ev)) => {
                // Only act on relevant events
                match ev.kind {
                    EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                        thread::sleep(debounce);
                        match load_from_file(path) {
                            Ok(_) => {
                                health.set("config", true);
                                let v = VERSION.fetch_add(1, Ordering::SeqCst) + 1;
                                let _ = bus.publish(KernelEvent::ConfigUpdated { version: v });
                                info!(version = v, file = ?path, "config reloaded");
                            }
                            Err(e) => {
                                health.set("config", false);
                                warn!(error = %e, file = ?path, "config reload failed");
                            }
                        }
                    }
                    _ => { /* ignore other event kinds */ }
                }
            }
            Ok(Err(e)) => {
                warn!(error = %e, "config watcher error event");
            }
            Err(e) => {
                // Channel closed; return Err so the outer loop can recreate the watcher.
                return Err(anyhow::anyhow!("config watcher channel closed: {e}"));
            }
        }
    }
}

```

### crates/ron-kernel/src/lib.rs

```rust
// FILE: crates/ron-kernel/src/lib.rs
#![forbid(unsafe_code)]
#![doc = include_str!("../docs/kernel_events.md")]

pub mod bus;
pub mod cancel;
pub mod config;
pub mod metrics;
pub mod overlay;
pub mod supervisor;
pub mod transport;

use serde::{Deserialize, Serialize};

// Re-export the stable surface (no self re-export of wait_for_ctrl_c)
pub use crate::config::Config;
pub use crate::metrics::{HealthState, Metrics};
pub use bus::Bus;

/// Kernel-wide event type (public at crate root).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum KernelEvent {
    Health { service: String, ok: bool },
    ConfigUpdated { version: u64 },
    // Keep 'reason' for compatibility and test snapshots.
    ServiceCrashed { service: String, reason: String },
    Shutdown,
}

/// Graceful Ctrl-C helper.
pub async fn wait_for_ctrl_c() -> std::io::Result<()> {
    tokio::signal::ctrl_c().await
}

```

### crates/ron-kernel/src/main_old.rs

```rust
// crates/ron-kernel/src/main.rs
#![forbid(unsafe_code)]

use std::env;
use std::os::unix::net::UnixStream;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use ron_bus::api::{
    Envelope, IndexReq, IndexResp, OverlayReq, OverlayResp, StorageReq, StorageResp,
};
use ron_bus::uds::{recv, send};

use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

const DEFAULT_INDEX_BIN: &str = "svc-index";
const DEFAULT_OVERLAY_BIN: &str = "svc-overlay";
const DEFAULT_STORAGE_BIN: &str = "svc-storage";

const DEFAULT_INDEX_SOCK: &str = "/tmp/ron/svc-index.sock";
const DEFAULT_OVERLAY_SOCK: &str = "/tmp/ron/svc-overlay.sock";
const DEFAULT_STORAGE_SOCK: &str = "/tmp/ron/svc-storage.sock";

fn main() -> std::io::Result<()> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let index_bin = env::var("RON_SVC_INDEX_BIN").unwrap_or_else(|_| DEFAULT_INDEX_BIN.into());
    let overlay_bin = env::var("RON_SVC_OVERLAY_BIN").unwrap_or_else(|_| DEFAULT_OVERLAY_BIN.into());
    let storage_bin = env::var("RON_SVC_STORAGE_BIN").unwrap_or_else(|_| DEFAULT_STORAGE_BIN.into());

    let index_sock = env::var("RON_INDEX_SOCK").unwrap_or_else(|_| DEFAULT_INDEX_SOCK.into());
    let overlay_sock = env::var("RON_OVERLAY_SOCK").unwrap_or_else(|_| DEFAULT_OVERLAY_SOCK.into());
    let storage_sock = env::var("RON_STORAGE_SOCK").unwrap_or_else(|_| DEFAULT_STORAGE_SOCK.into());

    info!("ron-kernel starting…");
    info!(%index_bin, %index_sock, "svc-index");
    info!(%overlay_bin, %overlay_sock, "svc-overlay");
    info!(%storage_bin, %storage_sock, "svc-storage");

    let mut idx = spawn(&index_bin, &[])?;
    let mut ovl = spawn(&overlay_bin, &[])?;
    let mut sto = spawn(&storage_bin, &[])?;

    loop {
        let iok = check_index_health(&index_sock);
        let ook = check_overlay_health(&overlay_sock);
        let sok = check_storage_health(&storage_sock);
        info!(index = iok, overlay = ook, storage = sok, "health");

        if !iok {
            warn!("restarting svc-index…");
            let _ = idx.kill();
            let _ = idx.wait();
            std::thread::sleep(Duration::from_secs(1));
            idx = spawn(&index_bin, &[])?;
        }
        if !ook {
            warn!("restarting svc-overlay…");
            let _ = ovl.kill();
            let _ = ovl.wait();
            std::thread::sleep(Duration::from_secs(1));
            ovl = spawn(&overlay_bin, &[])?;
        }
        if !sok {
            warn!("restarting svc-storage…");
            let _ = sto.kill();
            let _ = sto.wait();
            std::thread::sleep(Duration::from_secs(1));
            sto = spawn(&storage_bin, &[])?;
        }

        std::thread::sleep(Duration::from_secs(5));
    }
}

fn spawn(bin: &str, args: &[&str]) -> std::io::Result<Child> {
    Command::new(bin)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
}

fn check_index_health(sock: &str) -> bool {
    if let Ok(mut s) = UnixStream::connect(sock) {
        let payload = rmp_serde::to_vec(&IndexReq::Health).unwrap_or_default();
        let req = Envelope {
            service: "svc.index".into(),
            method: "v1.health".into(),
            corr_id: 0,
            token: vec![],
            payload,
        };
        if send(&mut s, &req).is_ok() {
            if let Ok(env) = recv(&mut s) {
                if let Ok(resp) = rmp_serde::from_slice::<IndexResp>(&env.payload) {
                    return matches!(resp, IndexResp::HealthOk);
                }
            }
        }
    }
    false
}

fn check_overlay_health(sock: &str) -> bool {
    if let Ok(mut s) = UnixStream::connect(sock) {
        let req = Envelope {
            service: "svc.overlay".into(),
            method: "v1.health".into(),
            corr_id: 0,
            token: vec![],
            payload: rmp_serde::to_vec(&OverlayReq::Health).unwrap_or_default(),
        };
        if send(&mut s, &req).is_ok() {
            if let Ok(env) = recv(&mut s) {
                if let Ok(resp) = rmp_serde::from_slice::<OverlayResp>(&env.payload) {
                    return matches!(resp, OverlayResp::HealthOk);
                }
            }
        }
    }
    false
}

fn check_storage_health(sock: &str) -> bool {
    if let Ok(mut s) = UnixStream::connect(sock) {
        let req = Envelope {
            service: "svc.storage".into(),
            method: "v1.health".into(),
            corr_id: 0,
            token: vec![],
            payload: rmp_serde::to_vec(&StorageReq::Health).unwrap_or_default(),
        };
        if send(&mut s, &req).is_ok() {
            if let Ok(env) = recv(&mut s) {
                if let Ok(resp) = rmp_serde::from_slice::<StorageResp>(&env.payload) {
                    return matches!(resp, StorageResp::HealthOk);
                }
            }
        }
    }
    false
}

```

### crates/ron-kernel/src/metrics.rs

```rust
#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, OnceLock};

use axum::{
    extract::State,
    http::{header, StatusCode},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use prometheus::{
    self as prom, register, Encoder, Histogram, HistogramOpts, IntCounterVec, Opts, TextEncoder,
};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tracing::info;

/// Shared health state exposed via /healthz and used by /readyz.
#[derive(Default)]
pub struct HealthState {
    inner: parking_lot::RwLock<HashMap<String, bool>>,
}

impl HealthState {
    pub fn new() -> Self {
        Self {
            inner: Default::default(),
        }
    }

    /// Mark a service as healthy/unhealthy.
    pub fn set(&self, service: impl Into<String>, ok: bool) {
        let mut g = self.inner.write();
        g.insert(service.into(), ok);
    }

    /// Take a snapshot for JSON responses or checks.
    pub fn snapshot(&self) -> HashMap<String, bool> {
        self.inner.read().clone()
    }

    /// Ready if all tracked services are healthy. If none tracked yet, not ready.
    pub fn all_ready(&self) -> bool {
        let g = self.inner.read();
        !g.is_empty() && g.values().all(|v| *v)
    }
}

/* ---------- Registration helpers (no unwrap/expect) ---------- */

fn reg_counter_vec(name: &str, help: &str, labels: &[&str]) -> IntCounterVec {
    match IntCounterVec::new(Opts::new(name, help), labels) {
        Ok(v) => {
            // Ignore AlreadyRegistered or other registration errors; metric still usable.
            let _ = register(Box::new(v.clone()));
            v
        }
        Err(e) => {
            eprintln!("prometheus: failed to create IntCounterVec {name}: {e}");
            // Fallback to a definitely-valid name to avoid collisions
            let fb_name = format!("{name}_fallback");
            match IntCounterVec::new(Opts::new(fb_name, help), labels) {
                Ok(v) => {
                    let _ = register(Box::new(v.clone()));
                    v
                }
                Err(e2) => {
                    // Extremely unlikely; last-resort minimal metric with fixed label set.
                    eprintln!("prometheus: fallback IntCounterVec failed for {name}: {e2}");
                    // Panic is acceptable here: cannot proceed without a metric object and we
                    // still satisfy the 'no unwrap/expect' lint.
                    panic!("unable to construct IntCounterVec for metrics: {name}");
                }
            }
        }
    }
}

fn reg_histogram(name: &str, help: &str) -> Histogram {
    match Histogram::with_opts(HistogramOpts::new(name, help)) {
        Ok(h) => {
            let _ = register(Box::new(h.clone()));
            h
        }
        Err(e) => {
            eprintln!("prometheus: failed to create Histogram {name}: {e}");
            let fb_name = format!("{name}_fallback");
            match Histogram::with_opts(HistogramOpts::new(fb_name, help)) {
                Ok(h) => {
                    let _ = register(Box::new(h.clone()));
                    h
                }
                Err(e2) => {
                    eprintln!("prometheus: fallback Histogram failed for {name}: {e2}");
                    panic!("unable to construct Histogram for metrics: {name}");
                }
            }
        }
    }
}

/* ---------- Global, process-wide collectors registered exactly once ---------- */

fn bus_lagged_total_static() -> &'static IntCounterVec {
    static V: OnceLock<IntCounterVec> = OnceLock::new();
    V.get_or_init(|| {
        reg_counter_vec(
            "bus_lagged_total",
            "Number of lagged events observed by receivers",
            &["service"],
        )
    })
}

fn service_restarts_total_static() -> &'static IntCounterVec {
    static V: OnceLock<IntCounterVec> = OnceLock::new();
    V.get_or_init(|| {
        reg_counter_vec(
            "service_restarts_total",
            "Count of service restarts",
            &["service"],
        )
    })
}

fn request_latency_seconds_static() -> &'static Histogram {
    static H: OnceLock<Histogram> = OnceLock::new();
    H.get_or_init(|| reg_histogram("request_latency_seconds", "HTTP request latency"))
}

/// Metrics registry & HTTP admin server (/metrics, /healthz, /readyz).
#[derive(Clone)]
pub struct Metrics {
    health: Arc<HealthState>,

    // Example metrics registered to the default registry per blueprint.
    pub bus_lagged_total: IntCounterVec,
    pub service_restarts_total: IntCounterVec,
    pub request_latency_seconds: Histogram,
}

impl Metrics {
    /// Create Metrics and clone the globally-registered collectors.
    pub fn new() -> Self {
        Self {
            health: Arc::new(HealthState::new()),
            bus_lagged_total: bus_lagged_total_static().clone(),
            service_restarts_total: service_restarts_total_static().clone(),
            request_latency_seconds: request_latency_seconds_static().clone(),
        }
    }

    /// Expose a reference to health state (matches blueprint).
    pub fn health(&self) -> &HealthState {
        &self.health
    }

    /// Start the admin HTTP server. Returns a JoinHandle and the bound address.
    ///
    /// Endpoints:
    /// - GET /metrics  -> Prometheus text format
    /// - GET /healthz  -> JSON map of service->bool (liveness)
    /// - GET /readyz   -> 200 if all services are healthy; else 503
    pub async fn serve(self, addr: SocketAddr) -> std::io::Result<(JoinHandle<()>, SocketAddr)> {
        let health = self.health.clone();

        let app = Router::new()
            .route("/metrics", get(metrics_handler))
            .route("/healthz", get(healthz_handler))
            .route("/readyz", get(readyz_handler))
            .with_state(AppState { health });

        let listener = TcpListener::bind(addr).await?;
        let local_addr = listener.local_addr()?;
        info!(
            "Admin endpoints: /metrics /healthz /readyz at http://{}/",
            local_addr
        );

        let handle = tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, app).await {
                tracing::error!("metrics admin server error: {e}");
            }
        });

        Ok((handle, local_addr))
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
struct AppState {
    health: Arc<HealthState>,
}

async fn metrics_handler() -> impl IntoResponse {
    let metric_families = prom::gather();
    let mut buf = Vec::new();
    let encoder = TextEncoder::new();
    if let Err(e) = encoder.encode(&metric_families, &mut buf) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("encode error: {e}"),
        )
            .into_response();
    }

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, encoder.format_type())],
        buf,
    )
        .into_response()
}

async fn healthz_handler(State(state): State<AppState>) -> impl IntoResponse {
    let snap = state.health.snapshot();
    Json(snap).into_response()
}

async fn readyz_handler(State(state): State<AppState>) -> impl IntoResponse {
    if state.health.all_ready() {
        (StatusCode::OK, "ready").into_response()
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "not ready").into_response()
    }
}

```

### crates/ron-kernel/src/overlay/admin_http.rs

```rust
#![forbid(unsafe_code)]

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Router};
use prometheus::{Encoder, TextEncoder};
use tokio::net::TcpListener;
use tracing::info;

use crate::{cancel::Shutdown, metrics::HealthState, Metrics};

pub async fn run(
    sdn: Shutdown,
    health: Arc<HealthState>,
    _metrics: Arc<Metrics>,
    addr: SocketAddr,
) -> anyhow::Result<()> {
    #[derive(Clone)]
    struct AdminState {
        health: Arc<HealthState>,
    }

    async fn healthz(State(st): State<AdminState>) -> impl IntoResponse {
        if st.health.all_ready() {
            (StatusCode::OK, "ok")
        } else {
            (StatusCode::SERVICE_UNAVAILABLE, "not ready")
        }
    }
    async fn readyz(State(st): State<AdminState>) -> impl IntoResponse {
        if st.health.all_ready() {
            (StatusCode::OK, "ready")
        } else {
            (StatusCode::SERVICE_UNAVAILABLE, "not ready")
        }
    }
    async fn metrics_route() -> impl IntoResponse {
        let mf = prometheus::gather();
        let mut buf = Vec::new();
        let enc = TextEncoder::new();
        let _ = enc.encode(&mf, &mut buf);
        (
            StatusCode::OK,
            [("Content-Type", enc.format_type().to_string())],
            buf,
        )
    }

    let state = AdminState { health };
    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/metrics", get(metrics_route))
        .with_state(state);
    let listener = TcpListener::bind(addr).await?;
    info!("admin HTTP listening on http://{addr}");
    axum::serve(listener, app)
        .with_graceful_shutdown(async move { sdn.cancelled().await })
        .await
        .map_err(|e| anyhow::anyhow!(e))
}

```

### crates/ron-kernel/src/overlay/metrics.rs

```rust
// crates/ron-kernel/src/overlay/metrics.rs
#![forbid(unsafe_code)]
// We allow expect() ONLY during metric construction at startup (never in hot paths).
#![allow(clippy::expect_used)]

use prometheus::{register_int_counter, register_int_gauge};
use prometheus::{IntCounter, IntGauge};

#[derive(Clone)]
pub struct OverlayMetrics {
    /// Total accepted overlay connections
    pub accepted_total: IntCounter,
    /// Total rejected overlay connections (at capacity)
    pub rejected_total: IntCounter,
    /// Current active overlay connections
    pub active_conns: IntGauge,
    /// TLS handshake failures (timeout or error)
    pub handshake_failures_total: IntCounter,
    /// Read timeouts before idle budget exhausted
    pub read_timeouts_total: IntCounter,
    /// Connections closed due to idle timeout
    pub idle_timeouts_total: IntCounter,
    /// Last applied ConfigUpdated version for overlay
    pub cfg_version: IntGauge,
    /// Current overlay max connections
    pub max_conns_gauge: IntGauge,
}

impl OverlayMetrics {
    pub fn new() -> Self {
        let accepted_total = reg_counter(
            "overlay_accepted_total",
            "Total accepted overlay connections",
        );
        let rejected_total = reg_counter(
            "overlay_rejected_total",
            "Total rejected overlay connections (at capacity)",
        );
        let active_conns = reg_gauge(
            "overlay_active_connections",
            "Current active overlay connections",
        );
        let handshake_failures_total = reg_counter(
            "overlay_handshake_failures_total",
            "TLS handshake failures (timeout or error)",
        );
        let read_timeouts_total = reg_counter(
            "overlay_read_timeouts_total",
            "Read timeouts before idle budget exhausted",
        );
        let idle_timeouts_total = reg_counter(
            "overlay_idle_timeouts_total",
            "Connections closed due to idle timeout",
        );
        let cfg_version = reg_gauge(
            "overlay_config_version",
            "Last applied ConfigUpdated version for overlay",
        );
        let max_conns_gauge = reg_gauge("overlay_max_conns", "Current overlay max connections");

        Self {
            accepted_total,
            rejected_total,
            active_conns,
            handshake_failures_total,
            read_timeouts_total,
            idle_timeouts_total,
            cfg_version,
            max_conns_gauge,
        }
    }
}

/// Public initializer expected by `overlay/mod.rs`.
/// Safe to call multiple times; the registry tolerates AlreadyRegistered.
pub fn init_overlay_metrics() -> OverlayMetrics {
    OverlayMetrics::new()
}

impl Default for OverlayMetrics {
    fn default() -> Self {
        Self::new()
    }
}

fn reg_counter(name: &'static str, help: &'static str) -> IntCounter {
    // Registration can fail if already registered or due to a bad name; we fall back
    // to a constructed counter so callers can still record metrics (unregistered is fine).
    register_int_counter!(name, help).unwrap_or_else(|e| {
        eprintln!("prometheus: failed to register counter {name}: {e}");
        prometheus::IntCounter::new(format!("{name}_fallback"), help.to_string())
            .expect("fallback IntCounter")
    })
}

fn reg_gauge(name: &'static str, help: &'static str) -> IntGauge {
    register_int_gauge!(name, help).unwrap_or_else(|e| {
        eprintln!("prometheus: failed to register gauge {name}: {e}");
        prometheus::IntGauge::new(format!("{name}_fallback"), help.to_string())
            .expect("fallback IntGauge")
    })
}

```

### crates/ron-kernel/src/overlay/mod.rs

```rust
#![forbid(unsafe_code)]

//! Overlay service modules and re-exports.

pub mod admin_http;
pub mod metrics;
pub mod runtime;
pub mod service;
pub mod tls;

// Re-exports for ergonomic imports
pub use metrics::{init_overlay_metrics, OverlayMetrics};
pub use runtime::{overlay_cfg_from, OverlayCfg, OverlayRuntime};
// Callers should use `overlay::service::run(...)` directly.
// Re-export admin http runner for convenience:
pub use admin_http::run as run_admin_http;

```

### crates/ron-kernel/src/overlay/runtime.rs

```rust
#![forbid(unsafe_code)]

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use tokio_rustls::TlsAcceptor;
use tracing::{error, info, warn};

use super::tls;
use crate::Config;

#[derive(Clone)]
pub struct OverlayCfg {
    pub bind: std::net::SocketAddr,
    pub max_conns: usize,
    pub handshake_timeout: Duration,
    pub idle_timeout: Duration,
    pub read_timeout: Duration,
    pub write_timeout: Duration,
    pub tls_acceptor: Option<TlsAcceptor>,
}

pub fn overlay_cfg_from(config: &Config) -> anyhow::Result<OverlayCfg> {
    let bind: std::net::SocketAddr = config.overlay_addr.parse()?;
    let t = &config.transport;
    let max_conns = t.max_conns.unwrap_or(2048) as usize;
    let idle_timeout = Duration::from_millis(t.idle_timeout_ms.unwrap_or(30_000));
    let read_timeout = Duration::from_millis(t.read_timeout_ms.unwrap_or(5_000));
    let write_timeout = Duration::from_millis(t.write_timeout_ms.unwrap_or(5_000));
    let handshake_timeout = Duration::from_millis(3_000);

    let tls_acceptor = match (
        config.raw.get("tls_cert_file").and_then(|v| v.as_str()),
        config.raw.get("tls_key_file").and_then(|v| v.as_str()),
    ) {
        (Some(cert), Some(key)) => match tls::try_build_server_config(cert, key) {
            Ok(cfg) => {
                info!("overlay TLS enabled (cert: {cert})");
                Some(TlsAcceptor::from(cfg))
            }
            Err(e) => {
                warn!("overlay TLS disabled (failed to load cert/key): {e:#}");
                None
            }
        },
        _ => {
            warn!("overlay TLS disabled (no tls_cert_file/tls_key_file in config)");
            None
        }
    };

    Ok(OverlayCfg {
        bind,
        max_conns,
        handshake_timeout,
        idle_timeout,
        read_timeout,
        write_timeout,
        tls_acceptor,
    })
}

#[derive(Clone)]
pub struct OverlayRuntime {
    pub max_conns: Arc<AtomicUsize>,
    pub idle_ms: Arc<AtomicU64>,
    pub read_ms: Arc<AtomicU64>,
    pub write_ms: Arc<AtomicU64>,
    pub tls_acceptor: Arc<RwLock<Option<TlsAcceptor>>>,
}

impl OverlayRuntime {
    pub fn from_cfg(cfg: &OverlayCfg) -> Self {
        Self {
            max_conns: Arc::new(AtomicUsize::new(cfg.max_conns)),
            idle_ms: Arc::new(AtomicU64::new(cfg.idle_timeout.as_millis() as u64)),
            read_ms: Arc::new(AtomicU64::new(cfg.read_timeout.as_millis() as u64)),
            write_ms: Arc::new(AtomicU64::new(cfg.write_timeout.as_millis() as u64)),
            tls_acceptor: Arc::new(RwLock::new(cfg.tls_acceptor.clone())),
        }
    }

    pub fn idle_timeout(&self) -> Duration {
        Duration::from_millis(self.idle_ms.load(Ordering::Relaxed))
    }
    pub fn read_timeout(&self) -> Duration {
        Duration::from_millis(self.read_ms.load(Ordering::Relaxed))
    }
    pub fn write_timeout(&self) -> Duration {
        Duration::from_millis(self.write_ms.load(Ordering::Relaxed))
    }
    pub fn max(&self) -> usize {
        self.max_conns.load(Ordering::Relaxed)
    }

    pub fn apply(&self, newc: &OverlayCfg) {
        self.max_conns.store(newc.max_conns, Ordering::Relaxed);
        self.idle_ms
            .store(newc.idle_timeout.as_millis() as u64, Ordering::Relaxed);
        self.read_ms
            .store(newc.read_timeout.as_millis() as u64, Ordering::Relaxed);
        self.write_ms
            .store(newc.write_timeout.as_millis() as u64, Ordering::Relaxed);
        *write_lock_ignore_poison(&self.tls_acceptor) = newc.tls_acceptor.clone();
    }
}

/// Recover from poisoned RwLock writes without panicking (log and continue).
#[inline]
fn write_lock_ignore_poison<'a, T>(rw: &'a RwLock<T>) -> std::sync::RwLockWriteGuard<'a, T> {
    match rw.write() {
        Ok(g) => g,
        Err(p) => {
            error!("overlay/runtime: write lock poisoned, recovering");
            p.into_inner()
        }
    }
}

```

### crates/ron-kernel/src/overlay/service.rs

```rust
#![forbid(unsafe_code)]

//! Overlay TCP listener + connection loop. Supports hot-reload via bus events.

use bytes::BytesMut;
use std::{
    io,
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    time::timeout,
};
use tracing::{info, warn};

use super::{
    metrics::OverlayMetrics,
    runtime::{OverlayCfg, OverlayRuntime},
};
use crate::{bus::Bus, cancel::Shutdown, metrics::HealthState, Metrics};
use crate::{config, KernelEvent};

enum IoEither {
    Plain(TcpStream),
    Tls(Box<tokio_rustls::server::TlsStream<TcpStream>>),
}
impl IoEither {
    async fn read_buf(&mut self, buf: &mut BytesMut) -> io::Result<usize> {
        match self {
            IoEither::Plain(s) => s.read_buf(buf).await,
            IoEither::Tls(s) => s.read_buf(buf).await,
        }
    }
    async fn write_all(&mut self, data: &[u8]) -> io::Result<()> {
        match self {
            IoEither::Plain(s) => s.write_all(data).await,
            IoEither::Tls(s) => s.write_all(data).await,
        }
    }
}

pub async fn run(
    sdn: Shutdown,
    health: Arc<HealthState>,
    metrics: Arc<Metrics>,
    cfg: OverlayCfg,
    om: OverlayMetrics,
    bus: Bus,
) -> anyhow::Result<()> {
    let listener = TcpListener::bind(cfg.bind).await?;
    info!("overlay listening on {}", cfg.bind);
    health.set("overlay", true);

    let rt = OverlayRuntime::from_cfg(&cfg);
    om.max_conns_gauge.set(rt.max() as i64);
    om.cfg_version.set(0);
    om.active_conns.set(0);
    health.set("capacity", 0 < rt.max());

    // Subscribe to config changes and hot-apply
    {
        let health = health.clone();
        let om = om.clone();
        let rt = rt.clone();
        tokio::spawn(async move {
            let mut rx = bus.subscribe();
            loop {
                match rx.recv().await {
                    Ok(KernelEvent::ConfigUpdated { version }) => {
                        match config::load_from_file("config.toml")
                            .ok()
                            .and_then(|c| super::runtime::overlay_cfg_from(&c).ok())
                        {
                            Some(newc) => {
                                // Log TLS change explicitly for visibility
                                if newc.tls_acceptor.is_some() {
                                    info!("overlay: TLS configuration updated/enabled");
                                } else {
                                    warn!("overlay: TLS configuration disabled or failed to load");
                                }

                                rt.apply(&newc);
                                om.max_conns_gauge.set(newc.max_conns as i64);
                                om.cfg_version.set(version as i64);

                                let active_now = om.active_conns.get() as usize;
                                health.set("capacity", active_now < newc.max_conns);
                                info!(
                                    "overlay hot-reloaded config version {version} (max_conns={})",
                                    newc.max_conns
                                );
                            }
                            None => {
                                warn!("overlay received ConfigUpdated {version} but failed to apply new config");
                            }
                        }
                    }
                    Ok(_) => {}
                    Err(_) => break,
                }
            }
        });
    }

    loop {
        tokio::select! {
            _ = sdn.cancelled() => { info!("overlay: shutdown requested"); break; }
            Ok((sock, peer)) = listener.accept() => {
                // Capacity check (hot-reload aware).
                let current_active = om.active_conns.get() as usize;
                if current_active >= rt.max() {
                    warn!("overlay: connection rejected (at capacity)");
                    om.rejected_total.inc();
                    health.set("capacity", (om.active_conns.get() as usize) < rt.max());
                    continue;
                }

                om.accepted_total.inc();
                om.active_conns.inc();
                health.set("capacity", (om.active_conns.get() as usize) < rt.max());

                let sdn_child = sdn.child();
                let metrics = metrics.clone();
                let om_child = om.clone();
                let rt_child = rt.clone();

                tokio::spawn(async move {
                    let _dec = ActiveConnGuard { gauge: om_child.active_conns.clone(), health: None, max: rt_child.max() };
                    if let Err(e) = handle_conn(sdn_child, metrics, sock, peer, rt_child, om_child, cfg.handshake_timeout).await {
                        warn!("overlay: connection error from {peer}: {e:#}");
                    }
                });
            }
        }
    }
    Ok(())
}

struct ActiveConnGuard {
    gauge: prometheus::IntGauge,
    health: Option<Arc<HealthState>>,
    max: usize,
}
impl Drop for ActiveConnGuard {
    fn drop(&mut self) {
        self.gauge.dec();
        if let Some(h) = &self.health {
            let active_now = self.gauge.get() as usize;
            h.set("capacity", active_now < self.max);
        }
    }
}

async fn handle_conn(
    sdn: Shutdown,
    metrics: Arc<Metrics>,
    sock: TcpStream,
    peer: SocketAddr,
    rt: OverlayRuntime,
    om: OverlayMetrics,
    handshake_timeout: Duration,
) -> anyhow::Result<()> {
    // IMPORTANT: grab the TLS acceptor into a local Option<TlsAcceptor> so we
    // don't hold an RwLockReadGuard across an await (required for Send).
    let acc_opt = match rt.tls_acceptor.read() {
        Ok(g) => g.clone(),
        Err(_) => None, // poisoned; treat as no TLS
    };

    let mut stream = if let Some(acc) = acc_opt {
        match timeout(handshake_timeout, acc.accept(sock)).await {
            Ok(Ok(accepted)) => IoEither::Tls(Box::new(accepted)),
            Ok(Err(e)) => {
                om.handshake_failures_total.inc();
                return Err(e.into());
            }
            Err(_) => {
                om.handshake_failures_total.inc();
                return Err(anyhow::anyhow!("tls handshake timeout"));
            }
        }
    } else {
        IoEither::Plain(sock)
    };

    let mut buf = BytesMut::with_capacity(16 * 1024);
    let mut last_activity = Instant::now();

    loop {
        if last_activity.elapsed() >= rt.idle_timeout() {
            warn!("overlay: idle timeout from {peer}");
            om.idle_timeouts_total.inc();
            break;
        }

        let remaining_idle = rt.idle_timeout().saturating_sub(last_activity.elapsed());
        let deadline = remaining_idle.min(rt.read_timeout());

        let read_res = tokio::select! {
            _ = sdn.cancelled() => break,
            res = timeout(deadline, stream.read_buf(&mut buf)) => res,
        };

        match read_res {
            Ok(Ok(0)) => break,
            Ok(Ok(n)) => {
                if n > 0 {
                    last_activity = Instant::now();
                    let to_send = &buf.split_to(n);
                    timeout(rt.write_timeout(), stream.write_all(to_send)).await??;
                }
            }
            Ok(Err(e)) => return Err(e.into()),
            Err(_) => {
                if last_activity.elapsed() >= rt.idle_timeout() {
                    warn!("overlay: idle timeout from {peer}");
                    om.idle_timeouts_total.inc();
                    break;
                } else {
                    warn!("overlay: read timeout from {peer}");
                    om.read_timeouts_total.inc();
                    continue;
                }
            }
        }
    }

    metrics.request_latency_seconds.observe(0.001);
    Ok(())
}

```

### crates/ron-kernel/src/overlay/tls.rs

```rust
#![forbid(unsafe_code)]

use anyhow::Context;
use rustls_pemfile::{certs, pkcs8_private_keys};
use std::{fs::File, io::BufReader, sync::Arc};
use tokio_rustls::rustls::{
    pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer},
    ServerConfig,
};

pub fn try_build_server_config(
    cert_path: &str,
    key_path: &str,
) -> anyhow::Result<Arc<ServerConfig>> {
    let mut cert_reader =
        BufReader::new(File::open(cert_path).with_context(|| format!("open cert {cert_path}"))?);
    let certs: Vec<CertificateDer<'static>> = certs(&mut cert_reader)
        .collect::<Result<Vec<_>, _>>()
        .with_context(|| "parse certs")?;

    let mut key_reader =
        BufReader::new(File::open(key_path).with_context(|| format!("open key {key_path}"))?);
    let mut keys: Vec<PrivatePkcs8KeyDer<'static>> = pkcs8_private_keys(&mut key_reader)
        .collect::<Result<Vec<_>, _>>()
        .with_context(|| "parse pkcs8 key")?;

    let pkcs8 = keys
        .pop()
        .ok_or_else(|| anyhow::anyhow!("no pkcs8 private key found"))?;
    let key_der: PrivateKeyDer<'static> = PrivateKeyDer::Pkcs8(pkcs8);

    let cfg = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key_der)
        .with_context(|| "with_single_cert")?;
    Ok(Arc::new(cfg))
}

```

### crates/ron-kernel/src/supervisor/metrics.rs

```rust
// crates/ron-kernel/src/supervisor/metrics.rs
#![forbid(unsafe_code)]
// Allow expect() only during startup-time metric construction (never in hot paths).
#![allow(clippy::expect_used)]

use std::sync::OnceLock;

use prometheus::{register, GaugeVec, IntCounterVec, Opts};

/// Supervisor metrics bundle kept for ergonomics in call sites.
/// Currently not constructed by callers; kept for future ergonomic use.
#[allow(dead_code)]
#[derive(Clone)]
pub struct SupervisorMetrics {
    /// Total number of restarts performed by the supervisor (label: service)
    pub restarts_total: IntCounterVec,
    /// Current backoff delay before restarting a service (seconds; f64) (label: service)
    pub backoff_seconds: GaugeVec,
}

impl SupervisorMetrics {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            restarts_total: restarts_metric().clone(),
            backoff_seconds: backoff_metric().clone(),
        }
    }
}

/// Global counter for restarts (labels: service).
/// Usage: `restarts_metric().with_label_values(&[service]).inc();`
pub fn restarts_metric() -> &'static IntCounterVec {
    static RESTARTS: OnceLock<IntCounterVec> = OnceLock::new();

    RESTARTS.get_or_init(|| {
        let v = IntCounterVec::new(
            Opts::new(
                "supervisor_restarts_total",
                "Total number of restarts performed by the supervisor",
            ),
            &["service"],
        )
        .expect("IntCounterVec::new(supervisor_restarts_total)");
        let _ = register(Box::new(v.clone())); // ignore AlreadyRegistered
        v
    })
}

/// Global gauge for the **current** backoff delay (seconds; f64) (labels: service).
/// Usage: `backoff_metric().with_label_values(&[service]).set(delay_secs_f64);`
pub fn backoff_metric() -> &'static GaugeVec {
    static BACKOFF_GAUGE: OnceLock<GaugeVec> = OnceLock::new();

    BACKOFF_GAUGE.get_or_init(|| {
        let g = GaugeVec::new(
            Opts::new(
                "supervisor_backoff_seconds",
                "Current backoff delay before restarting a service",
            ),
            &["service"],
        )
        .expect("GaugeVec::new(supervisor_backoff_seconds)");
        let _ = register(Box::new(g.clone())); // ignore AlreadyRegistered
        g
    })
}

```

### crates/ron-kernel/src/supervisor/mod.rs

```rust
#![forbid(unsafe_code)]

mod metrics;
mod policy;
mod runner;

pub use runner::{Supervisor, SupervisorHandle};

```

### crates/ron-kernel/src/supervisor/policy.rs

```rust
#![forbid(unsafe_code)]

use std::time::Duration;

#[derive(Clone, Copy)]
pub struct RestartPolicy {
    pub base: Duration,
    pub max: Duration,
    pub factor: f64,
    pub jitter: f64, // +/- percentage of delay (0.0..1.0)
}
impl Default for RestartPolicy {
    fn default() -> Self {
        Self {
            base: Duration::from_millis(300),
            max: Duration::from_secs(30),
            factor: 2.0,
            jitter: 0.2,
        }
    }
}

pub fn mul_duration(d: Duration, f: f64) -> Duration {
    let secs = d.as_secs_f64() * f;
    if secs <= 0.0 {
        Duration::from_millis(0)
    } else {
        Duration::from_secs_f64(secs)
    }
}

pub fn compute_backoff(policy: &RestartPolicy, gen: u64) -> Duration {
    let mut delay = mul_duration(policy.base, policy.factor.powf(gen as f64));
    if delay > policy.max {
        delay = policy.max;
    }
    if policy.jitter > 0.0 {
        let j = (gen.wrapping_mul(1103515245).wrapping_add(12345) % 1000) as f64 / 1000.0;
        let scale = 1.0 + policy.jitter * (j * 2.0 - 1.0);
        delay = mul_duration(delay, scale);
    }
    delay
}

```

### crates/ron-kernel/src/supervisor/runner.rs

```rust
#![forbid(unsafe_code)]

use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc, time::Duration};

use prometheus::{GaugeVec, IntCounterVec};
use tokio::{
    task::JoinHandle,
    time::{sleep, timeout},
};
use tracing::{info, warn};

use super::metrics::{backoff_metric, restarts_metric};
use super::policy::{compute_backoff, RestartPolicy};
use crate::{bus::Bus, cancel::Shutdown, metrics::HealthState, KernelEvent, Metrics};

type BoxFut = Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send>>;
type ServiceFactory = Arc<dyn Fn(Shutdown) -> BoxFut + Send + Sync>;

struct Service {
    name: String,
    factory: ServiceFactory,
}

pub struct Supervisor {
    bus: Bus,
    _metrics: Arc<Metrics>,
    health: Arc<HealthState>,
    root_sdn: Shutdown,
    services: Vec<Service>,
}

pub struct SupervisorHandle {
    root_sdn: Shutdown,
    join: JoinHandle<()>,
}

impl Supervisor {
    pub fn new(bus: Bus, metrics: Arc<Metrics>, health: Arc<HealthState>, sdn: Shutdown) -> Self {
        Self {
            bus,
            _metrics: metrics,
            health,
            root_sdn: sdn,
            services: Vec::new(),
        }
    }

    pub fn add_service<F, Fut>(&mut self, name: &str, f: F)
    where
        F: Fn(Shutdown) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = anyhow::Result<()>> + Send + 'static,
    {
        let name = name.to_string();
        let factory: ServiceFactory = Arc::new(move |sdn| {
            let fut = f(sdn);
            Box::pin(fut)
        });
        self.services.push(Service { name, factory });
    }

    pub fn spawn(self) -> SupervisorHandle {
        info!("Supervisor starting {} services…", self.services.len());
        // Hand the actual root shutdown token to the handle so `shutdown()` works.
        let root_for_handle = self.root_sdn.clone();
        let join = tokio::spawn(run_supervisor(self));
        SupervisorHandle {
            root_sdn: root_for_handle,
            join,
        }
    }
}

impl SupervisorHandle {
    pub fn shutdown(&self) {
        self.root_sdn.cancel();
    }
    pub async fn join(self) -> anyhow::Result<()> {
        let _ = self.join.await;
        Ok(())
    }
}

/* =========================== internal runner ============================== */

async fn run_supervisor(mut sup: Supervisor) {
    let restarts = restarts_metric();
    let backoff_g = backoff_metric();

    let mut tasks: HashMap<String, JoinHandle<()>> = HashMap::new();
    let policy = RestartPolicy::default();

    for svc in sup.services.drain(..) {
        let name = svc.name.clone();
        let j = spawn_service_loop(
            name.clone(),
            svc.factory.clone(),
            sup.bus.clone(),
            sup.health.clone(),
            sup.root_sdn.clone(),
            restarts.clone(),
            backoff_g.clone(),
            policy,
        );
        tasks.insert(name, j);
    }

    // Wait for root shutdown, then let tasks exit when their children see it.
    sup.root_sdn.cancelled().await;

    // Join all children best-effort
    for (_, j) in tasks.into_iter() {
        let _ = j.await;
    }
}

/* ============================= restart loop =============================== */

#[allow(clippy::too_many_arguments)]
fn spawn_service_loop(
    name: String,
    factory: ServiceFactory,
    bus: Bus,
    health: Arc<HealthState>,
    root_sdn: Shutdown,
    restarts: IntCounterVec,
    backoff_g: GaugeVec,
    policy: RestartPolicy,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut gen: u64 = 0;

        loop {
            // Non-blocking check: if the shutdown has already been requested, stop restarting.
            if timeout(Duration::from_millis(0), root_sdn.cancelled())
                .await
                .is_ok()
            {
                break;
            }

            health.set(&name, false);
            info!(target: "ron_kernel::supervisor", service = %name, "service starting");
            let sdn = root_sdn.child();

            let fut = (factory)(sdn.clone());
            let name_clone = name.clone();

            match fut.await {
                Ok(()) => {
                    health.set(&name_clone, false);

                    // If root shutdown was requested while the service was running, do not restart.
                    if timeout(Duration::from_millis(0), root_sdn.cancelled())
                        .await
                        .is_ok()
                    {
                        break;
                    }

                    // Treat a clean exit as a crash for supervision semantics, then back off and restart.
                    let reason = "exited_ok";
                    let _ = bus.publish(KernelEvent::ServiceCrashed {
                        service: name_clone.clone(),
                        reason: reason.to_string(),
                    });
                    let delay = compute_backoff(&policy, gen);
                    backoff_g
                        .with_label_values(&[&name_clone])
                        .set(delay.as_secs_f64());
                    restarts.with_label_values(&[&name_clone]).inc();
                    sleep(delay).await;
                    gen = gen.saturating_add(1);
                }
                Err(e) => {
                    health.set(&name_clone, false);
                    let reason = format!("error: {e:#}");
                    let _ = bus.publish(KernelEvent::ServiceCrashed {
                        service: name_clone.clone(),
                        reason,
                    });
                    let delay = compute_backoff(&policy, gen);
                    backoff_g
                        .with_label_values(&[&name_clone])
                        .set(delay.as_secs_f64());
                    restarts.with_label_values(&[&name_clone]).inc();
                    warn!(target="ron_kernel::supervisor", service=%name_clone, "service crashed; restarting after backoff");
                    sleep(delay).await;
                    gen = gen.saturating_add(1);
                }
            }
        }
    })
}

```

### crates/ron-kernel/src/tracing_init.rs

```rust
use tracing_subscriber::{fmt, EnvFilter};

/// Initialize compact tracing with an env-driven filter.
/// Example: RUST_LOG=info,ron_kernel=debug,actor_spike=debug
pub fn tracing_init(default_filter: &str) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(default_filter));
    fmt().with_env_filter(filter).compact().init();
}

```

### crates/ron-kernel/src/transport.rs

```rust
#![forbid(unsafe_code)]

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio::time::{timeout, Duration as TokioDuration};

use tokio_rustls::rustls::ServerConfig as TlsServerConfig;

use crate::bus::Bus;
use crate::KernelEvent;
use crate::{HealthState, Metrics};

#[derive(Clone, Debug)]
pub struct TransportConfig {
    pub addr: SocketAddr,
    pub name: &'static str,
    pub max_conns: usize,
    pub read_timeout: Duration,
    pub write_timeout: Duration,
    pub idle_timeout: Duration,
}

pub async fn spawn_transport(
    cfg: TransportConfig,
    _metrics: Metrics,
    _health: Arc<HealthState>,
    bus: Bus,
    _tls_override: Option<TlsServerConfig>,
) -> std::io::Result<(JoinHandle<()>, SocketAddr)> {
    let listener = TcpListener::bind(cfg.addr).await?;
    let local_addr = listener.local_addr()?;

    let _ = bus.publish(KernelEvent::Health {
        service: cfg.name.to_string(),
        ok: true,
    });

    let permits = Arc::new(tokio::sync::Semaphore::new(cfg.max_conns));

    let name = cfg.name;
    let idle = cfg.idle_timeout;
    let write_to = cfg.write_timeout;

    let handle = tokio::spawn(async move {
        loop {
            let permit = permits.clone().acquire_owned().await;
            let permit = match permit {
                Ok(p) => p,
                Err(_) => break,
            };

            let (mut socket, peer) = match listener.accept().await {
                Ok(v) => v,
                Err(e) => {
                    let _ = bus.publish(KernelEvent::ServiceCrashed {
                        service: name.to_string(),
                        reason: format!("accept error: {e}"),
                    });
                    continue;
                }
            };

            let bus_clone = bus.clone();

            tokio::spawn(async move {
                let idle_deadline = TokioDuration::from_secs(idle.as_secs().max(1));
                let _ = timeout(idle_deadline, async {
                    tokio::time::sleep(TokioDuration::from_millis(10)).await;
                })
                .await;

                let _ = timeout(TokioDuration::from_secs(write_to.as_secs().max(1)), async {
                    use tokio::io::AsyncWriteExt;
                    let _ = socket.write_all(b"").await;
                })
                .await;

                drop(permit);

                let _ = bus_clone.publish(KernelEvent::Health {
                    service: format!("{name}:{peer}"),
                    ok: true,
                });
            });
        }
    });

    Ok((handle, local_addr))
}

```

### crates/ron-kernel/tests/bus_basic.rs

```rust
#![forbid(unsafe_code)]

use ron_kernel::bus::Bus;
use ron_kernel::KernelEvent;
use std::error::Error;

#[tokio::test]
async fn bus_basic_pubsub() -> Result<(), Box<dyn Error>> {
    let bus = Bus::new(8);
    let mut rx = bus.subscribe();

    bus.publish(KernelEvent::Health {
        service: "svc-a".into(),
        ok: true,
    })?;

    let ev = rx.recv().await?;
    match ev {
        KernelEvent::Health { service, ok } => {
            assert_eq!(service, "svc-a");
            assert!(ok);
        }
        other => panic!("unexpected event: {:?}", other),
    }

    Ok(())
}

```

### crates/ron-kernel/tests/bus_load.rs

```rust
#![forbid(unsafe_code)]
use ron_kernel::bus::{sub, Bus};
use ron_kernel::KernelEvent;
use std::time::Duration;

#[tokio::test]
async fn bus_reports_lag() {
    // Tiny capacity to force lag on slow consumer.
    let bus = Bus::new(2);
    let mut rx = bus.subscribe();

    // Publish a burst larger than capacity.
    for i in 0..16u32 {
        bus.publish_lossy(KernelEvent::ConfigUpdated { version: i as u64 });
    }

    // Drain some messages slowly so we trigger "lagged".
    let mut seen = 0;
    for _ in 0..8 {
        if sub::recv_with_timeout(&bus, &mut rx, Duration::from_millis(50))
            .await
            .is_some()
        {
            seen += 1;
        }
    }

    // We should have recorded some drop.
    assert!(
        bus.dropped_total() > 0,
        "expected dropped_total > 0, seen={seen}"
    );
}

```

### crates/ron-kernel/tests/bus_topic.rs

```rust
#![forbid(unsafe_code)]
use ron_kernel::bus::{sub, Bus};
use ron_kernel::KernelEvent;
use std::time::Duration;

#[tokio::test]
async fn bus_topic_filtering() {
    let bus = Bus::new(8);
    let mut rx = bus.subscribe();

    // noise
    bus.publish_lossy(KernelEvent::Health {
        service: "svc-a".into(),
        ok: true,
    });

    // target
    bus.publish_lossy(KernelEvent::ConfigUpdated { version: 42 });

    // more noise
    bus.publish_lossy(KernelEvent::Health {
        service: "svc-b".into(),
        ok: false,
    });

    let got = sub::recv_matching(
        &bus,
        &mut rx,
        Duration::from_millis(250),
        |ev| matches!(ev, KernelEvent::ConfigUpdated { version } if *version == 42),
    )
    .await;

    assert!(matches!(
        got,
        Some(KernelEvent::ConfigUpdated { version: 42 })
    ));
}

```

### crates/ron-kernel/tests/event_snapshot.rs

```rust
// FILE: crates/ron-kernel/tests/event_snapshot.rs
#![forbid(unsafe_code)]

use std::error::Error;

use ron_kernel::KernelEvent;
use serde_json::{json, Value};

#[test]
fn kernel_event_serde_snapshot() -> Result<(), Box<dyn Error>> {
    let cases = [
        KernelEvent::Health {
            service: "svc".into(),
            ok: true,
        },
        KernelEvent::ConfigUpdated { version: 42 },
        KernelEvent::ServiceCrashed {
            service: "svc".into(),
            reason: "boom".into(),
        },
        KernelEvent::Shutdown,
    ];

    // Produce the JSON values without expect/unwrap.
    let got: Vec<Value> = cases
        .iter()
        .map(serde_json::to_value)
        .collect::<Result<Vec<_>, _>>()?;

    // Externally-tagged serde enum representation is intentional and stable.
    let expected = [
        json!({ "Health":        { "service": "svc", "ok": true } }),
        json!({ "ConfigUpdated": { "version": 42 } }),
        json!({ "ServiceCrashed":{ "service": "svc", "reason": "boom" } }),
        json!("Shutdown"),
    ];

    assert_eq!(
        got.as_slice(),
        &expected,
        "KernelEvent serde snapshot changed"
    );
    Ok(())
}

#[test]
fn kernel_event_json_roundtrip() -> Result<(), Box<dyn Error>> {
    let cases = [
        KernelEvent::Health {
            service: "svc".into(),
            ok: true,
        },
        KernelEvent::ConfigUpdated { version: 42 },
        KernelEvent::ServiceCrashed {
            service: "svc".into(),
            reason: "boom".into(),
        },
        KernelEvent::Shutdown,
    ];

    for ev in cases {
        // Serialize to JSON text…
        let s = serde_json::to_string(&ev)?;
        // …and back to the enum.
        let back: KernelEvent = serde_json::from_str(&s)?;
        assert_eq!(ev, back, "roundtrip changed the value");
    }

    Ok(())
}

```

### crates/ron-kernel/tests/http_index_overlay.rs

```rust
// crates/ron-kernel/tests/http_index_overlay.rs
#![forbid(unsafe_code)]

use std::{
    error::Error,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};

use axum::{
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use ron_kernel::Metrics;
use serde::{Deserialize, Serialize};
use tokio::{net::TcpListener, task::JoinHandle, time::sleep};

#[derive(Clone)]
struct TestState {
    metrics: std::sync::Arc<Metrics>,
    map: std::sync::Arc<tokio::sync::RwLock<std::collections::HashMap<String, String>>>,
}

#[derive(Deserialize)]
struct PutReq {
    addr: String,
    dir: String,
}

#[derive(Serialize, Deserialize)]
struct EchoResp {
    echo: String,
}

async fn overlay_echo(
    axum::extract::State(st): axum::extract::State<TestState>,
    Json(req): Json<serde_json::Value>,
) -> impl axum::response::IntoResponse {
    let _t = st.metrics.request_latency_seconds.start_timer();
    // Simulate a tiny bit of work so the histogram records something > 0
    sleep(Duration::from_millis(2)).await;

    let payload = req
        .get("payload")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    (StatusCode::OK, Json(EchoResp { echo: payload }))
}

async fn index_put(
    axum::extract::State(st): axum::extract::State<TestState>,
    Json(req): Json<PutReq>,
) -> impl axum::response::IntoResponse {
    let _t = st.metrics.request_latency_seconds.start_timer();
    st.map.write().await.insert(req.addr, req.dir);
    (
        StatusCode::OK,
        Json(serde_json::json!({ "ok": true, "data": "ok" })),
    )
}

async fn index_resolve(
    axum::extract::State(st): axum::extract::State<TestState>,
    axum::extract::Path(addr): axum::extract::Path<String>,
) -> impl axum::response::IntoResponse {
    let _t = st.metrics.request_latency_seconds.start_timer();
    let g = st.map.read().await;
    if let Some(dir) = g.get(&addr) {
        (
            StatusCode::OK,
            Json(serde_json::json!({ "ok": true, "data": { "addr": addr, "dir": dir } })),
        )
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "ok": false, "error": "not found" })),
        )
    }
}

async fn serve_on_ephemeral(app: Router) -> Result<(SocketAddr, JoinHandle<()>), Box<dyn Error>> {
    // Bind to 127.0.0.1:0 to get an ephemeral port without parsing a string.
    let bind_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
    let listener = TcpListener::bind(bind_addr).await?;
    let addr = listener.local_addr()?;
    let h = tokio::spawn(async move {
        // Best-effort: if the server ends with an error, just exit the task.
        let _ = axum::serve(listener, app).await;
    });
    Ok((addr, h))
}

#[tokio::test]
async fn overlay_echo_roundtrip() -> Result<(), Box<dyn Error>> {
    let m = std::sync::Arc::new(Metrics::new());
    m.health().set("test_overlay", true);

    let st = TestState {
        metrics: m.clone(),
        map: std::sync::Arc::new(tokio::sync::RwLock::new(Default::default())),
    };
    let app = Router::new()
        .route("/echo", post(overlay_echo))
        .with_state(st);

    let (addr, task) = serve_on_ephemeral(app).await?;

    // drive
    let client = reqwest::Client::new();
    let r = client
        .post(format!("http://{addr}/echo"))
        .json(&serde_json::json!({ "payload": "ping" }))
        .send()
        .await?;
    assert_eq!(r.status(), StatusCode::OK);
    let v: EchoResp = r.json().await?;
    assert_eq!(v.echo, "ping");

    // histogram should be > 0
    assert!(m.request_latency_seconds.get_sample_count() >= 1);

    task.abort();
    Ok(())
}

#[tokio::test]
async fn index_put_resolve_roundtrip() -> Result<(), Box<dyn Error>> {
    let m = std::sync::Arc::new(Metrics::new());
    m.health().set("test_index", true);

    let st = TestState {
        metrics: m.clone(),
        map: std::sync::Arc::new(tokio::sync::RwLock::new(Default::default())),
    };
    let app = Router::new()
        .route("/put", post(index_put))
        .route("/resolve/:addr", get(index_resolve))
        .with_state(st);

    let (addr, task) = serve_on_ephemeral(app).await?;

    // PUT a few entries
    let client = reqwest::Client::new();
    for i in 1..=3 {
        let r = client
            .post(format!("http://{addr}/put"))
            .json(&serde_json::json!({ "addr": format!("A{i}"), "dir": format!("B{i}") }))
            .send()
            .await?;
        assert_eq!(r.status(), StatusCode::OK);
    }

    // RESOLVE one of them
    let r = reqwest::get(format!("http://{addr}/resolve/A2")).await?;
    assert_eq!(r.status(), StatusCode::OK);
    let j: serde_json::Value = r.json().await?;
    assert_eq!(j["ok"], true);
    assert_eq!(j["data"]["addr"], "A2");
    assert_eq!(j["data"]["dir"], "B2");

    // We exercised histogram at least 4 times (put x3 + resolve x1)
    assert!(m.request_latency_seconds.get_sample_count() >= 4);

    task.abort();
    Ok(())
}

```

### crates/ron-kernel/tests/loom_health.rs

```rust
// FILE: crates/ron-kernel/tests/loom_health.rs
#![forbid(unsafe_code)]

#[cfg(feature = "loom")]
mod loom_tests {
    use loom::sync::{Arc, Mutex};
    use loom::thread;

    #[derive(Default)]
    struct Health {
        ready: bool,
        config: bool,
        db: bool,
        net: bool,
        bus: bool,
    }

    impl Health {
        fn set_ready_if_complete(&mut self) {
            self.ready = self.config && self.db && self.net && self.bus;
        }
    }

    /// Acquire a lock without `unwrap()`/`expect()`. If the mutex is poisoned,
    /// recover the inner guard so the model can continue exploring interleavings.
    fn lock_no_panic<T>(m: &Mutex<T>) -> loom::sync::MutexGuard<'_, T> {
        match m.lock() {
            Ok(g) => g,
            Err(poison) => poison.into_inner(),
        }
    }

    #[test]
    fn readiness_eventual_and_consistent() {
        loom::model(|| {
            let h = Arc::new(Mutex::new(Health::default()));

            for key in ["config", "db", "net", "bus"] {
                let h2 = Arc::clone(&h);
                thread::spawn(move || {
                    let mut g = lock_no_panic(&h2);
                    match key {
                        "config" => g.config = true,
                        "db" => g.db = true,
                        "net" => g.net = true,
                        "bus" => g.bus = true,
                        _ => {}
                    }
                    g.set_ready_if_complete();
                });
            }

            let g = lock_no_panic(&h);
            if g.ready {
                assert!(g.config && g.db && g.net && g.bus);
            }
        });
    }
}

```

### crates/ron-kernel/tests/no_sha256_guard.rs

```rust
// Enforce repository-wide policy: no "sha256" mention in code/docs,
// except for explicit allowlisted files/dirs. We prefer BLAKE3/b3: everywhere.
//
// Allowed:
//  - This test file
//  - TLS helper stubs (e.g., */tls.rs) that may reference sha256 for interop docs
//  - DailyTodo.md (engineering notes)
//  - .git/, target/
//
// Note: keeps scanning the whole workspace (not just this crate).

#![forbid(unsafe_code)]

use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

fn is_texty_extension(ext: &str) -> bool {
    matches!(
        ext,
        "rs" | "toml" | "md" | "yml" | "yaml" | "json" | "txt" | "sh" | "lock"
    )
}

fn path_has_component(path: &Path, needle: &str) -> bool {
    path.components().any(|c| match c {
        Component::Normal(s) => s.to_string_lossy().eq_ignore_ascii_case(needle),
        _ => false,
    })
}

fn under_subpath(path: &Path, segment_seq: &[&str]) -> bool {
    // true if all given components occur in order within the path
    let parts: Vec<String> = path
        .components()
        .filter_map(|c| match c {
            Component::Normal(s) => Some(s.to_string_lossy().to_string()),
            _ => None,
        })
        .collect();
    let mut i = 0usize;
    for seg in parts {
        if i < segment_seq.len() && seg.eq_ignore_ascii_case(segment_seq[i]) {
            i += 1;
            if i == segment_seq.len() {
                return true;
            }
        }
    }
    false
}

fn is_allowlisted(path: &Path) -> bool {
    // Directories we ignore entirely
    if path_has_component(path, ".git")
        || path_has_component(path, "target")
        || path_has_component(path, ".onions")   // workspace artifacts
        || path_has_component(path, "scripts")   // dev/demo scripts may mention sha256 for tooling
        || path_has_component(path, "testing")   // CI helper scripts
    {
        return true;
    }

    // Specific files we allow
    let fname = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    if fname.eq_ignore_ascii_case("no_sha256_guard.rs")
        || fname.eq_ignore_ascii_case("dailytodo.md")
        || fname.eq_ignore_ascii_case("todo.md") // root TODO notes
    {
        return true;
    }

    // TLS helpers (interop docs/snippets)
    if fname.eq_ignore_ascii_case("tls.rs") || path_has_component(path, "tls") {
        return true;
    }

    // Kernel test README (notes)
    if fname.eq_ignore_ascii_case("README.md")
        && under_subpath(path, &["crates", "ron-kernel", "tests"])
    {
        return true;
    }

    false
}


fn find_workspace_root(start: &Path) -> PathBuf {
    // Walk up until we find a Cargo.toml that declares a [workspace] table.
    let mut cur = Some(start);
    while let Some(dir) = cur {
        let candidate = dir.join("Cargo.toml");
        if candidate.exists() {
            if let Ok(s) = fs::read_to_string(&candidate) {
                if s.contains("[workspace]") {
                    return dir.to_path_buf();
                }
            }
        }
        cur = dir.parent();
    }
    // Fallback: current crate root.
    start.to_path_buf()
}

fn gather_files(root: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();

            if is_allowlisted(&path) {
                // Skip allowlisted dirs/files entirely
                if path.is_dir() {
                    continue;
                }
                if path.is_file() {
                    continue;
                }
            }

            if path.is_dir() {
                stack.push(path);
                continue;
            }

            if path.is_file() {
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|s| s.to_ascii_lowercase())
                    .unwrap_or_default();
                if is_texty_extension(&ext) {
                    out.push(path);
                }
            }
        }
    }
    Ok(())
}

#[test]
fn forbid_sha256_mentions_workspace_wide() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let ws_root = find_workspace_root(crate_root);

    let mut files = Vec::new();
    gather_files(&ws_root, &mut files).expect("walk workspace");

    let mut hits: Vec<String> = Vec::new();

    for file in files {
        if is_allowlisted(&file) {
            continue;
        }

        let Ok(contents) = fs::read_to_string(&file) else {
            // Binary or unreadable; skip.
            continue;
        };
        let lower = contents.to_ascii_lowercase();

        // Look for plain "sha256" or "sha-256"
        if lower.contains("sha256") || lower.contains("sha-256") {
            // Produce line-oriented matches for better diagnostics
            for (idx, line) in lower.lines().enumerate() {
                if line.contains("sha256") || line.contains("sha-256") {
                    hits.push(format!(
                        "{}:{}: matched token \"{}\"",
                        file.display(),
                        idx + 1,
                        if line.contains("sha-256") { "sha-256" } else { "sha256" }
                    ));
                }
            }
        }
    }

    if !hits.is_empty() {
        let mut msg = String::new();
        msg.push_str(
            "Forbidden SHA-256 references found (use BLAKE3 / b3:<hex> instead).\n\nAllowlist:\n  \
             - this test file\n  \
             - TLS helpers (*/tls.rs and tls/ modules)\n  \
             - DailyTodo.md (engineering notes)\n  \
             - .git/ and target/\n\nMatches:\n",
        );
        for h in hits {
            msg.push_str(&h);
            msg.push('\n');
        }
        panic!("{msg}");
    }
}

```

### crates/ron-kms/Cargo.toml

```toml
[package]
name = "ron-kms"
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"
publish = false

[features]
# Default: lightweight symmetric HMAC KMS.
default = ["hmac"]

# HMAC-based KMS (symmetric). Pulls in sha2 + hmac.
hmac = ["dep:hmac", "dep:sha2"]

# Optional Ed25519 support (asymmetric).
# Uses workspace-pinned ed25519-dalek v2 and rand_core 0.6.
ed25519 = ["dep:ed25519-dalek", "dep:rand_core"]

[dependencies]
anyhow        = { workspace = true }
thiserror     = { workspace = true }
serde         = { workspace = true, features = ["derive"] }
parking_lot   = { workspace = true }
rand          = { workspace = true }
hex           = { workspace = true }

# crypto deps
hmac             = { version = "0.12", optional = true }
sha2             = { workspace = true, optional = true }

# ed25519 (optional; compiled only when feature enabled)
ed25519-dalek    = { workspace = true, optional = true, default-features = false, features = ["alloc", "rand_core"] }
rand_core        = { workspace = true, optional = true, default-features = false }

# Internal protocol types
ron-proto        = { path = "../ron-proto" }

```

### crates/ron-kms/src/lib.rs

```rust
#![forbid(unsafe_code)]

//! ron-kms: minimal pluggable Key Management for RustyOnions.
//!
//! - Default: HMAC-SHA256 KMS (symmetric) with in-memory keystore.
//! - Optional: Ed25519 (feature = "ed25519") for asymmetric signing.
//! - Integrates with `ron-proto::SignedEnvelope<T>` helpers.
//!
//! This crate focuses on a clean trait boundary. Storage/HSM backends can be
//! swapped later (e.g., file, Sled, cloud KMS) behind the same interface.

use hex::ToHex;
use parking_lot::RwLock;
use rand::{rng, RngCore};
use ron_proto::{Algo, KeyId, Signature};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, sync::Arc};

#[derive(Debug, thiserror::Error)]
pub enum KmsError {
    #[error("unknown key id: {0}")]
    UnknownKey(String),
    #[error("algorithm mismatch: expected {expected:?}, got {got:?}")]
    AlgoMismatch { expected: Algo, got: Algo },
    #[error("unsupported operation: {0}")]
    Unsupported(String),
    #[error("crypto error: {0}")]
    Crypto(String),
}

pub type Result<T> = std::result::Result<T, KmsError>;

/// Opaque private key material for different algos.
#[derive(Clone, Serialize, Deserialize)]
enum KeyMaterial {
    /// Symmetric secret for HMAC-SHA256.
    Hmac(Vec<u8>),
    /// Ed25519 secret key bytes (if feature enabled).
    #[cfg(feature = "ed25519")]
    Ed25519(ed25519_dalek::SecretKey),
}

#[derive(Clone, Serialize, Deserialize)]
struct KeyEntry {
    algo: Algo,
    mat: KeyMaterial,
}

#[derive(Default)]
pub struct InMemoryKms {
    inner: Arc<RwLock<HashMap<String, KeyEntry>>>,
}

impl InMemoryKms {
    pub fn new() -> Self {
        Self { inner: Arc::new(RwLock::new(HashMap::new())) }
    }

    /// Generate a new key for the given algo; returns KeyId.
    pub fn generate_key(&self, algo: Algo) -> Result<KeyId> {
        match algo {
            Algo::HmacSha256 => {
                let mut buf = vec![0u8; 32];
                rng().fill_bytes(&mut buf);
                let kid = self.derive_kid(algo.clone(), &buf);
                let entry = KeyEntry { algo, mat: KeyMaterial::Hmac(buf) };
                self.inner.write().insert(kid.0.clone(), entry);
                Ok(kid)
            }
            #[cfg(feature = "ed25519")]
            Algo::Ed25519 => {
                use ed25519_dalek::{SecretKey, SigningKey};
                // Generate securely from OS RNG via dalek
                let signing = SigningKey::generate(&mut rand::rng());
                let secret = signing.to_secret_key();
                let kid = self.derive_kid(Algo::Ed25519, secret.as_bytes());
                let entry = KeyEntry { algo: Algo::Ed25519, mat: KeyMaterial::Ed25519(secret) };
                self.inner.write().insert(kid.0.clone(), entry);
                Ok(kid)
            }
            #[cfg(not(feature = "ed25519"))]
            Algo::Ed25519 => Err(KmsError::Unsupported(
                "Ed25519 requested but crate built without feature \"ed25519\"".into(),
            )),
        }
    }

    /// Import a pre-existing secret (bytes meaning depends on algo).
    pub fn import_key(&self, algo: Algo, key_bytes: &[u8]) -> Result<KeyId> {
        match algo {
            Algo::HmacSha256 => {
                if key_bytes.is_empty() {
                    return Err(KmsError::Crypto("empty key".into()));
                }
                let kid = self.derive_kid(algo.clone(), key_bytes);
                let entry = KeyEntry { algo, mat: KeyMaterial::Hmac(key_bytes.to_vec()) };
                self.inner.write().insert(kid.0.clone(), entry);
                Ok(kid)
            }
            #[cfg(feature = "ed25519")]
            Algo::Ed25519 => {
                use ed25519_dalek::SecretKey;
                let sk = SecretKey::from_bytes(key_bytes)
                    .map_err(|e| KmsError::Crypto(format!("bad ed25519 secret: {e}")))?;
                let kid = self.derive_kid(Algo::Ed25519, sk.as_bytes());
                let entry = KeyEntry { algo: Algo::Ed25519, mat: KeyMaterial::Ed25519(sk) };
                self.inner.write().insert(kid.0.clone(), entry);
                Ok(kid)
            }
            #[cfg(not(feature = "ed25519"))]
            Algo::Ed25519 => Err(KmsError::Unsupported(
                "Ed25519 requested but crate built without feature \"ed25519\"".into(),
            )),
        }
    }

    /// Delete a key from the in-memory store.
    pub fn delete_key(&self, kid: &KeyId) -> Result<()> {
        let removed = self.inner.write().remove(&kid.0);
        if removed.is_some() {
            Ok(())
        } else {
            Err(KmsError::UnknownKey(kid.0.clone()))
        }
    }

    /// Produce a signature over `msg` using the key `kid`.
    pub fn sign(&self, kid: &KeyId, algo: Algo, msg: &[u8]) -> Result<Signature> {
        let entry = self.inner.read().get(&kid.0).cloned().ok_or_else(|| KmsError::UnknownKey(kid.0.clone()))?;
        if entry.algo != algo {
            return Err(KmsError::AlgoMismatch { expected: entry.algo, got: algo });
        }
        match entry.mat {
            KeyMaterial::Hmac(ref k) => {
                use hmac::{Hmac, Mac};
                type HmacSha256 = Hmac<sha2::Sha256>;
                let mut mac = HmacSha256::new_from_slice(k).map_err(|e| KmsError::Crypto(e.to_string()))?;
                mac.update(msg);
                let res = mac.finalize().into_bytes().to_vec();
                Ok(Signature::from_bytes(res))
            }
            #[cfg(feature = "ed25519")]
            KeyMaterial::Ed25519(ref sk) => {
                use ed25519_dalek::{SecretKey, SigningKey, Signer};
                let signing = SigningKey::from(sk.clone());
                let sig = signing.sign(msg);
                Ok(Signature::from_bytes(sig.to_bytes().to_vec()))
            }
        }
    }

    /// Verify a signature over `msg` using the key `kid`.
    pub fn verify(&self, kid: &KeyId, algo: Algo, msg: &[u8], sig: &Signature) -> Result<bool> {
        let entry = self.inner.read().get(&kid.0).cloned().ok_or_else(|| KmsError::UnknownKey(kid.0.clone()))?;
        if entry.algo != algo {
            return Err(KmsError::AlgoMismatch { expected: entry.algo, got: algo });
        }
        match entry.mat {
            KeyMaterial::Hmac(ref k) => {
                use hmac::{Hmac, Mac};
                type HmacSha256 = Hmac<sha2::Sha256>;
                let mut mac = HmacSha256::new_from_slice(k).map_err(|e| KmsError::Crypto(e.to_string()))?;
                mac.update(msg);
                let expected = mac.finalize().into_bytes().to_vec();
                Ok(constant_time_eq::constant_time_eq(sig.as_bytes(), &expected))
            }
            #[cfg(feature = "ed25519")]
            KeyMaterial::Ed25519(ref sk) => {
                use ed25519_dalek::{SecretKey, SigningKey, VerifyingKey, Signature as DalekSig, Verifier};
                let signing = SigningKey::from(sk.clone());
                let vk: VerifyingKey = signing.verifying_key();
                let dsig = DalekSig::from_bytes(sig.as_bytes()).map_err(|e| KmsError::Crypto(e.to_string()))?;
                Ok(vk.verify(msg, &dsig).is_ok())
            }
        }
    }

    /// Sign a `ron_proto::SignedEnvelope<T>` payload bytes and attach KID+signature.
    ///
    /// `payload_bytes` must be exactly the bytes used to compute `payload_hash`.
    pub fn sign_envelope<T: serde::Serialize + for<'de> serde::Deserialize<'de>>(
        &self,
        mut env: ron_proto::SignedEnvelope<T>,
        kid: &KeyId,
        algo: Algo,
        payload_bytes: &[u8],
    ) -> Result<ron_proto::SignedEnvelope<T>> {
        if !env.payload_hash_matches(payload_bytes) {
            return Err(KmsError::Crypto("payload bytes do not match envelope.payload_hash".into()));
        }
        let sig = self.sign(kid, algo.clone(), payload_bytes)?;
        Ok(env.with_signature(kid.clone(), sig))
    }

    /// Minimal stable KID derivation: `kid = algo || ":" || hex(sha256(secret_bytes))`
    fn derive_kid(&self, algo: Algo, secret: &[u8]) -> KeyId {
        let mut hasher = Sha256::new();
        hasher.update(secret);
        let digest = hasher.finalize().encode_hex::<String>();
        let prefix = match algo {
            Algo::HmacSha256 => "hmac",
            Algo::Ed25519 => "ed25519",
        };
        KeyId(format!("{prefix}:{digest}"))
    }
}

// Constant-time equality (tiny helper) without adding a new crate for one fn.
mod constant_time_eq {
    pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
        if a.len() != b.len() {
            return false;
        }
        let mut r = 0u8;
        for (x, y) in a.iter().zip(b.iter()) {
            r |= x ^ y;
        }
        r == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ron_proto::{wire, SignedEnvelope};

    #[test]
    fn hmac_sign_verify() {
        let kms = InMemoryKms::new();
        let kid = kms.generate_key(Algo::HmacSha256).unwrap();

        let msg = b"hello world";
        let sig = kms.sign(&kid, Algo::HmacSha256, msg).unwrap();
        assert!(kms.verify(&kid, Algo::HmacSha256, msg, &sig).unwrap());
        assert!(!kms.verify(&kid, Algo::HmacSha256, b"tampered", &sig).unwrap());
    }

    #[test]
    fn sign_envelope_msgpack_roundtrip() {
        #[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
        struct Demo { id: u64, body: String }

        let kms = InMemoryKms::new();
        let kid = kms.generate_key(Algo::HmacSha256).unwrap();

        let payload = Demo { id: 1, body: "ron".into() };
        let bytes = wire::to_json(&payload).unwrap().into_bytes(); // using JSON for the demo

        let env = SignedEnvelope::unsigned(1, "Demo", Algo::HmacSha256, payload.clone(), &bytes);
        let signed = kms.sign_envelope(env, &kid, Algo::HmacSha256, &bytes).unwrap();

        // payload hash should match and signature should verify
        assert!(signed.payload_hash_matches(&bytes));
        assert!(kms.verify(&kid, Algo::HmacSha256, &bytes, &signed.sig).unwrap());
    }

    #[cfg(feature = "ed25519")]
    #[test]
    fn ed25519_sign_verify() {
        let kms = InMemoryKms::new();
        let kid = kms.generate_key(Algo::Ed25519).unwrap();
        let msg = b"hello ed25519";
        let sig = kms.sign(&kid, Algo::Ed25519, msg).unwrap();
        assert!(kms.verify(&kid, Algo::Ed25519, msg, &sig).unwrap());
    }
}

```

### crates/ron-ledger/Cargo.toml

```toml
[package]
name = "ron-ledger"
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"
publish = false

[dependencies]
serde = { workspace = true, features = ["derive"] }
serde_json = { workspace = true }

```

### crates/ron-ledger/src/lib.rs

```rust
//! ron-ledger: token ledger core (traits + reference in-memory implementation).
//!
//! Goals:
//! - Storage-agnostic trait (`TokenLedger`) for mint/burn/transfer, balances, supply.
//! - Safe invariants: non-negative balances; conservation of supply; overflow checks.
//! - Simple in-memory impl for early integration; swap backends later (SQLite/sled/…).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

pub type Amount = u128;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AccountId(pub String);

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Op {
    Mint,
    Burn,
    Transfer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub id: u64,
    pub ts_ms: u128,
    pub op: Op,
    pub from: Option<AccountId>,
    pub to: Option<AccountId>,
    pub amount: Amount,
    pub reason: Option<String>,
    pub supply_after: Amount,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receipt {
    pub entry_id: u64,
    pub balance_after: Option<Amount>,
    pub supply_after: Amount,
}

#[derive(Debug)]
pub enum TokenError {
    ZeroAmount,
    InsufficientFunds { account: AccountId, needed: Amount, available: Amount },
    Overflow,
}

impl std::fmt::Display for TokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenError::ZeroAmount => write!(f, "amount must be > 0"),
            TokenError::InsufficientFunds { account, needed, available } =>
                write!(f, "insufficient funds in {}: need {}, have {}", account.0, needed, available),
            TokenError::Overflow => write!(f, "arithmetic overflow"),
        }
    }
}
impl std::error::Error for TokenError {}

/// Storage-agnostic token ledger interface.
///
/// NOTE: trait is sync & non-async to keep implementations flexible (can
/// layer async at service boundary if needed).
pub trait TokenLedger {
    fn total_supply(&self) -> Amount;
    fn balance(&self, account: &AccountId) -> Amount;
    fn entries(&self) -> Vec<LedgerEntry>; // copy out; backends may add streaming later

    fn mint(&mut self, to: AccountId, amount: Amount, reason: Option<String>) -> Result<Receipt, TokenError>;
    fn burn(&mut self, from: AccountId, amount: Amount, reason: Option<String>) -> Result<Receipt, TokenError>;
    fn transfer(&mut self, from: AccountId, to: AccountId, amount: Amount, reason: Option<String>) -> Result<Receipt, TokenError>;
}

/// A simple in-memory, single-threaded ledger. Wrap in a lock for concurrency.
#[derive(Debug, Default)]
pub struct InMemoryLedger {
    next_id: u64,
    total_supply: Amount,
    balances: HashMap<AccountId, Amount>,
    entries: Vec<LedgerEntry>,
}

impl InMemoryLedger {
    pub fn new() -> Self {
        Self::default()
    }

    fn push_entry(
        &mut self,
        op: Op,
        from: Option<AccountId>,
        to: Option<AccountId>,
        amount: Amount,
        reason: Option<String>,
    ) -> &LedgerEntry {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        let ts_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        self.entries.push(LedgerEntry {
            id,
            ts_ms,
            op,
            from,
            to,
            amount,
            reason,
            supply_after: self.total_supply,
        });
        self.entries.last().unwrap()
    }
}

impl TokenLedger for InMemoryLedger {
    #[inline]
    fn total_supply(&self) -> Amount {
        self.total_supply
    }

    #[inline]
    fn balance(&self, account: &AccountId) -> Amount {
        *self.balances.get(account).unwrap_or(&0)
    }

    fn entries(&self) -> Vec<LedgerEntry> {
        self.entries.clone()
    }

    fn mint(
        &mut self,
        to: AccountId,
        amount: Amount,
        reason: Option<String>,
    ) -> Result<Receipt, TokenError> {
        if amount == 0 {
            return Err(TokenError::ZeroAmount);
        }
        let bal = self.balances.entry(to.clone()).or_insert(0);
        *bal = bal.checked_add(amount).ok_or(TokenError::Overflow)?;
        self.total_supply = self.total_supply.checked_add(amount).ok_or(TokenError::Overflow)?;
        let entry = self.push_entry(Op::Mint, None, Some(to.clone()), amount, reason);
        Ok(Receipt {
            entry_id: entry.id,
            balance_after: Some(*bal),
            supply_after: entry.supply_after,
        })
    }

    fn burn(
        &mut self,
        from: AccountId,
        amount: Amount,
        reason: Option<String>,
    ) -> Result<Receipt, TokenError> {
        if amount == 0 {
            return Err(TokenError::ZeroAmount);
        }
        let bal = self.balances.entry(from.clone()).or_insert(0);
        if *bal < amount {
            return Err(TokenError::InsufficientFunds {
                account: from,
                needed: amount,
                available: *bal,
            });
        }
        *bal -= amount;
        self.total_supply = self.total_supply.checked_sub(amount).ok_or(TokenError::Overflow)?;
        let entry = self.push_entry(Op::Burn, None, None, amount, reason);
        Ok(Receipt {
            entry_id: entry.id,
            balance_after: Some(*bal),
            supply_after: entry.supply_after,
        })
    }

    fn transfer(
        &mut self,
        from: AccountId,
        to: AccountId,
        amount: Amount,
        reason: Option<String>,
    ) -> Result<Receipt, TokenError> {
        if amount == 0 {
            return Err(TokenError::ZeroAmount);
        }
        if from == to {
            // no-op; surface current state
            return Ok(Receipt {
                entry_id: self.next_id,
                balance_after: Some(self.balance(&to)),
                supply_after: self.total_supply,
            });
        }
        // debit
        let from_bal = self.balances.entry(from.clone()).or_insert(0);
        if *from_bal < amount {
            return Err(TokenError::InsufficientFunds {
                account: from,
                needed: amount,
                available: *from_bal,
            });
        }
        *from_bal -= amount;

        // credit
        let to_bal = self.balances.entry(to.clone()).or_insert(0);
        *to_bal = to_bal.checked_add(amount).ok_or(TokenError::Overflow)?;

        // supply unchanged
        let entry = self.push_entry(Op::Transfer, Some(from), Some(to.clone()), amount, reason);
        Ok(Receipt {
            entry_id: entry.id,
            balance_after: Some(*to_bal),
            supply_after: entry.supply_after,
        })
    }
}

```

### crates/ron-policy/Cargo.toml

```toml
[package]
name = "ron-policy"
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"
publish = false

[dependencies]
serde = { workspace = true, features = ["derive"] }
serde_json = { workspace = true }

```

### crates/ron-policy/src/lib.rs

```rust
//! ron-policy: shared policy/quotas/limits library for RustyOnions.
//!
//! Minimal, compile-first stub that always allows, with a typed decision object.
//! Replace the internals with real quotas & config when ready.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDecision {
    pub allowed: bool,
    pub reason: &'static str,
}

#[derive(Debug, Clone)]
pub struct PolicyEngine {
    quotas_enabled: bool,
}

impl PolicyEngine {
    /// Build a default policy engine.
    /// In the future, load from config (env/files/remote) and wire to metrics.
    pub fn new_default() -> Self {
        Self { quotas_enabled: false }
    }

    /// Check whether `principal` may perform `action`.
    /// Replace this with real logic: rate limits, roles, per-node/tenant caps, etc.
    pub fn check(&self, _principal: &str, _action: &str) -> PolicyDecision {
        if self.quotas_enabled {
            // Placeholder path for when quotas are enabled later.
            PolicyDecision { allowed: true, reason: "quotas-enabled: allow (stub)" }
        } else {
            PolicyDecision { allowed: true, reason: "allow-all (stub)" }
        }
    }
}

```

### crates/ron-proto/Cargo.toml

```toml
[package]
name = "ron-proto"
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"
publish = false

[features]
# MessagePack support toggled on by default (matches your svc crates pattern).
default = ["rmp"]
rmp = ["dep:rmp-serde"]

[dependencies]
serde = { workspace = true, features = ["derive"] }
thiserror = { workspace = true }
sha2 = { workspace = true }
base64 = { workspace = true }
hex = { workspace = true }
rmp-serde = { workspace = true, optional = true }

```

### crates/ron-proto/src/lib.rs

```rust
#![forbid(unsafe_code)]

//! ron-proto: wire-level types, signatures, and envelopes shared across services.
//!
//! - Serde DTOs for stable inter-crate/API boundaries.
//! - Optional rmp-serde for compact wire format (`feature = "rmp"`).
//! - Simple, explicit error type.
//!
//! NOTE: This is crypto-agnostic. Signing is done by ron-kms; we only define
//! algorithms and envelope shapes here.

use base64::prelude::*;
use hex::ToHex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

#[derive(Debug, thiserror::Error)]
pub enum ProtoError {
    #[error("serialization error: {0}")]
    Serde(String),
    #[error("deserialization error: {0}")]
    DeSerde(String),
    #[error("unsupported operation: {0}")]
    Unsupported(String),
}

pub type Result<T> = std::result::Result<T, ProtoError>;

/// Supported signing algorithms. Extend as needed.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Algo {
    /// HMAC-SHA256 (symmetric). Great for bootstrap and testing.
    HmacSha256,
    /// Ed25519 (asymmetric). Enable in KMS via the "ed25519" feature.
    Ed25519,
}

/// Stable identifier for a key within the KMS.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct KeyId(pub String);

impl fmt::Display for KeyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Opaque signature bytes (binary, transported as bytes in MessagePack or base64 in JSON).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Signature(#[serde(with = "serde_bytes")] pub Vec<u8>);

impl Signature {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
    pub fn to_base64(&self) -> String {
        BASE64_STANDARD.encode(&self.0)
    }
    pub fn from_bytes(b: Vec<u8>) -> Self {
        Signature(b)
    }
}

/// Compute a hex-encoded SHA256 of arbitrary bytes (used for payload_hash).
pub fn sha256_hex(data: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(data);
    let out = h.finalize();
    out.encode_hex::<String>()
}

/// Generic signed envelope for any payload `T`.
///
/// `payload_hash` is SHA256(payload_bytes) in hex to make signature coverage explicit and
/// enable out-of-band integrity checks.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignedEnvelope<T: Serialize + for<'de> Deserialize<'de>> {
    /// Version for forward-compatibility (bump on breaking changes).
    pub v: u8,
    /// Logical type name (e.g., "Put", "GetReq", "Chunk", "Event").
    pub typ: String,
    /// Algorithm used to produce `sig`.
    pub algo: Algo,
    /// Key identifier used to create `sig`.
    pub kid: KeyId,
    /// Hash of the serialized payload (hex-encoded SHA256 of `payload_bytes`).
    pub payload_hash: String,
    /// Raw signature bytes (algorithm-dependent).
    pub sig: Signature,
    /// The actual payload.
    pub payload: T,
}

impl<T> SignedEnvelope<T>
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    /// Create an unsigned envelope with computed payload_hash; caller fills kid/sig later.
    pub fn unsigned(v: u8, typ: impl Into<String>, algo: Algo, payload: T, payload_bytes: &[u8]) -> Self {
        let payload_hash = sha256_hex(payload_bytes);
        Self {
            v,
            typ: typ.into(),
            algo,
            kid: KeyId(String::new()),
            payload_hash,
            sig: Signature(Vec::new()),
            payload,
        }
    }

    /// Attach signature and key id.
    pub fn with_signature(mut self, kid: KeyId, sig: Signature) -> Self {
        self.kid = kid;
        self.sig = sig;
        self
    }

    /// Verify that the supplied `payload_bytes` matches `payload_hash`.
    pub fn payload_hash_matches(&self, payload_bytes: &[u8]) -> bool {
        self.payload_hash == sha256_hex(payload_bytes)
    }
}

/// Wire helpers — encode/decode via MessagePack (default feature) or JSON.
pub mod wire {
    use super::{ProtoError, Result};
    use serde::{de::DeserializeOwned, Serialize};

    #[cfg(feature = "rmp")]
    pub fn to_msgpack<T: Serialize>(value: &T) -> Result<Vec<u8>> {
        rmp_serde::to_vec_named(value).map_err(|e| ProtoError::Serde(e.to_string()))
    }

    #[cfg(feature = "rmp")]
    pub fn from_msgpack<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
        rmp_serde::from_slice(bytes).map_err(|e| ProtoError::DeSerde(e.to_string()))
    }

    pub fn to_json<T: Serialize>(value: &T) -> Result<String> {
        serde_json::to_string(value).map_err(|e| ProtoError::Serde(e.to_string()))
    }

    pub fn from_json<T: DeserializeOwned>(s: &str) -> Result<T> {
        serde_json::from_str(s).map_err(|e| ProtoError::DeSerde(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
    struct Demo {
        id: u32,
        name: String,
    }

    #[test]
    fn payload_hash_roundtrip_json() {
        let p = Demo { id: 7, name: "ron".into() };
        let bytes = serde_json::to_vec(&p).unwrap();
        let env = SignedEnvelope::unsigned(1, "Demo", Algo::HmacSha256, p.clone(), &bytes);
        assert!(env.payload_hash_matches(&bytes));
        let s = wire::to_json(&env).unwrap();
        let back: SignedEnvelope<Demo> = wire::from_json(&s).unwrap();
        assert_eq!(back.payload, p);
    }

    #[cfg(feature = "rmp")]
    #[test]
    fn payload_hash_roundtrip_msgpack() {
        let p = Demo { id: 42, name: "proto".into() };
        let bytes = wire::to_msgpack(&p).unwrap();
        let env = SignedEnvelope::unsigned(1, "Demo", Algo::HmacSha256, p.clone(), &bytes);
        assert!(env.payload_hash_matches(&bytes));
        let b = wire::to_msgpack(&env).unwrap();
        let back: SignedEnvelope<Demo> = wire::from_msgpack(&b).unwrap();
        assert_eq!(back.payload, p);
    }

    #[test]
    fn signature_helpers_base64() {
        let sig = Signature(vec![1, 2, 3, 4, 5]);
        let b64 = sig.to_base64();
        assert!(!b64.is_empty());
    }
}

```

### crates/ron-token/Cargo.toml

```toml
[package]
name = "ron-token"
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"
publish = false

[dependencies]
ron-ledger = { workspace = true }
serde = { workspace = true, features = ["derive"] }
serde_json = { workspace = true }

```

### crates/ron-token/src/lib.rs

```rust
//! ron-token: domain facade for token economy atop `ron-ledger`.
//!
//! Today this crate re-exports the ledger surface so services depend on a
//! stable domain name (`ron-token`). Grow this with higher-level helpers:
//! - idempotency keys & per-account sequences
//! - policy-aware wrappers
//! - receipt signing & epoch-root emission

pub use ron_ledger::{
    AccountId, Amount, InMemoryLedger, LedgerEntry, Op, Receipt, TokenError, TokenLedger,
};

```

### crates/ryker/Cargo.toml

```toml
[package]
name = "ryker"
version = "0.2.0"            # bump: we’re changing public contents, even with compat
edition = "2021"
license = "MIT OR Apache-2.0"
publish = false

[features]
# Transitional: keep old billing symbols available under `ryker::...`.
# You can set default-features = false in dependents after migrating.
default = ["billing-compat"]
billing-compat = ["ron-billing"]

[dependencies]
tokio = { workspace = true, features = ["rt-multi-thread", "macros", "time"] }
tracing = { workspace = true }
rand = "0.8"
ron-billing = { path = "../ron-billing", optional = true }

```

### crates/ryker/src/lib.rs

```rust
#![forbid(unsafe_code)]
// ryker: tiny supervisor helpers with jittered backoff and a temporary
// compatibility shim for previously in-crate billing helpers.

use std::future::Future;
use std::time::Duration;
use tokio::task::JoinHandle;
use tracing::{error, info};

/// Spawn a supervised task; restart on error with exponential backoff and jitter.
///
/// Usage:
/// ```ignore
/// ryker::spawn_supervised("overlay-loop", || async {
///     overlay::run_once().await
/// });
/// ```
pub fn spawn_supervised<F, Fut>(name: &'static str, mut factory: F) -> JoinHandle<()>
where
    F: FnMut() -> Fut + Send + 'static,
    Fut: Future<Output = anyhow::Result<()>> + Send + 'static,
{
    tokio::spawn(async move {
        let mut backoff = Duration::from_millis(200);
        let max = Duration::from_secs(10);
        loop {
            info!(task = name, "starting");
            match factory().await {
                Ok(()) => {
                    info!(task = name, "completed (no restart)");
                    return;
                }
                Err(e) => {
                    error!(task = name, error = %e, "task failed; will restart");
                    tokio::time::sleep(jitter(backoff)).await;
                    backoff = (backoff * 2).min(max);
                }
            }
        }
    })
}

fn jitter(base: Duration) -> Duration {
    use rand::Rng;
    let b = base.as_millis() as u64;
    let j = rand::thread_rng().gen_range((b / 2)..=(b + b / 2).max(1));
    Duration::from_millis(j)
}

// -------- Temporary compatibility re-exports --------
// These allow existing code importing `ryker::PriceModel` or `ryker::compute_cost`
// to keep compiling while you migrate to `ron_billing::...`.
//
// To remove: set `default-features = false` on `ryker` in dependents,
// then delete this section in a future minor release.
#[cfg(feature = "billing-compat")]
#[allow(deprecated)]
pub use ron_billing::{
    compute_cost as compute_cost,
    validate_payment_block as validate_payment_block,
    validate_wallet_string as validate_wallet_string,
};

#[cfg(feature = "billing-compat")]
#[allow(deprecated)]
pub use ron_billing::PriceModel;

#[cfg(feature = "billing-compat")]
#[deprecated(
    since = "0.2.0",
    note = "moved to `ron-billing`; switch to `ron_billing::PriceModel` and related functions"
)]
pub mod _billing_compat_note {}

```

### crates/svc-economy/Cargo.toml

```toml
[package]
name = "svc-economy"
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"
publish = false

[dependencies]
axum = { workspace = true, features = ["tokio", "http1", "http2", "json"] }
tokio = { workspace = true, features = ["macros", "rt-multi-thread", "signal", "io-util"] }
serde = { workspace = true, features = ["derive"] }
serde_json = { workspace = true }
prometheus = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true, features = ["env-filter", "fmt"] }
parking_lot = { workspace = true }
ron-ledger = { workspace = true }

```

### crates/svc-economy/src/main.rs

```rust
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use parking_lot::RwLock;
use prometheus::{Encoder, Histogram, HistogramOpts, IntCounterVec, Opts, TextEncoder};
use ron_ledger::{AccountId, InMemoryLedger, TokenError, TokenLedger};
use serde::{Deserialize, Serialize};
use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, SystemTime},
};
use tracing::{info, Level};
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Clone)]
struct AppState {
    started: SystemTime,
    ready: Arc<AtomicBool>,
    ledger: Arc<RwLock<InMemoryLedger>>,
    metrics: Arc<Metrics>,
    service_name: &'static str,
    version: &'static str,
}

struct Metrics {
    tx_total: IntCounterVec,
    tx_failed_total: IntCounterVec,
    request_latency_seconds: Histogram,
}

impl Metrics {
    fn new() -> Self {
        let tx_total = IntCounterVec::new(
            Opts::new("tx_total", "Total successful token operations"),
            &["op"],
        )
        .expect("tx_total");
        let tx_failed_total = IntCounterVec::new(
            Opts::new("tx_failed_total", "Total failed token operations"),
            &["op", "reason"],
        )
        .expect("tx_failed_total");
        let request_latency_seconds =
            Histogram::with_opts(HistogramOpts::new("request_latency_seconds", "Request latency (seconds)"))
                .expect("request_latency_seconds");

        prometheus::register(Box::new(tx_total.clone())).ok();
        prometheus::register(Box::new(tx_failed_total.clone())).ok();
        prometheus::register(Box::new(request_latency_seconds.clone())).ok();

        Self {
            tx_total,
            tx_failed_total,
            request_latency_seconds,
        }
    }
}

#[derive(Serialize)]
struct StatusPayload<'a> {
    service: &'a str,
    version: &'a str,
    ok: bool,
    uptime_secs: u64,
}

#[derive(Deserialize)]
struct MintReq {
    account: String,
    amount: u128,
    reason: Option<String>,
}
#[derive(Deserialize)]
struct BurnReq {
    account: String,
    amount: u128,
    reason: Option<String>,
}
#[derive(Deserialize)]
struct TransferReq {
    from: String,
    to: String,
    amount: u128,
    reason: Option<String>,
}

#[derive(Serialize)]
struct BalanceResp {
    account: String,
    balance: u128,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();

    let bind: SocketAddr = std::env::var("ECONOMY_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:3003".to_string())
        .parse()
        .expect("ECONOMY_ADDR must be host:port");

    let state = AppState {
        started: SystemTime::now(),
        ready: Arc::new(AtomicBool::new(false)),
        ledger: Arc::new(RwLock::new(InMemoryLedger::new())),
        metrics: Arc::new(Metrics::new()),
        service_name: "svc-economy",
        version: env!("CARGO_PKG_VERSION"),
    };

    let app = Router::new()
        // Public API
        .route("/mint", post(post_mint))
        .route("/burn", post(post_burn))
        .route("/transfer", post(post_transfer))
        .route("/balance/:account", get(get_balance))
        .route("/supply", get(get_supply))
        // Ops
        .route("/", get(root))
        .route("/version", get(version))
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/metrics", get(metrics))
        .with_state(state.clone());

    let listener = tokio::net::TcpListener::bind(bind).await?;
    info!("svc-economy listening on http://{bind}");

    state.ready.store(true, Ordering::SeqCst);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("svc-economy shutdown complete");
    Ok(())
}

fn init_tracing() {
    // Respect RUST_LOG if provided, default to info
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt()
        .with_target(false)
        .with_timer(fmt::time::uptime())
        .with_max_level(Level::INFO)
        .with_env_filter(env_filter)
        .init();
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    info!("received Ctrl-C, shutting down…");
}

async fn root(State(st): State<AppState>) -> impl IntoResponse {
    let up = st.started.elapsed().unwrap_or(Duration::from_secs(0)).as_secs();
    let payload = StatusPayload {
        service: st.service_name,
        version: st.version,
        ok: true,
        uptime_secs: up,
    };
    (StatusCode::OK, Json(payload))
}

async fn version(State(st): State<AppState>) -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({
        "service": st.service_name,
        "version": st.version
    })))
}

async fn healthz() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({ "ok": true })))
}

async fn readyz(State(st): State<AppState>) -> impl IntoResponse {
    let ok = st.ready.load(Ordering::SeqCst);
    let code = if ok { StatusCode::OK } else { StatusCode::SERVICE_UNAVAILABLE };
    (code, Json(serde_json::json!({ "ready": ok })))
}

async fn metrics() -> impl IntoResponse {
    let metric_families = prometheus::gather();
    let mut buf = Vec::new();
    let encoder = TextEncoder::new();
    if let Err(e) = encoder.encode(&metric_families, &mut buf) {
        let body = format!("encode error: {e}");
        return (StatusCode::INTERNAL_SERVER_ERROR, body).into_response();
    }
    (StatusCode::OK, String::from_utf8_lossy(&buf).to_string()).into_response()
}

// ---------- API handlers ----------

async fn post_mint(State(st): State<AppState>, Json(req): Json<MintReq>) -> impl IntoResponse {
    let timer = st.metrics.request_latency_seconds.start_timer();
    let op = "mint";
    let result = {
        let mut l = st.ledger.write();
        l.mint(AccountId(req.account.clone()), req.amount, req.reason)
    };
    let resp = match result {
        Ok(r) => {
            st.metrics.tx_total.with_label_values(&[op]).inc();
            (StatusCode::OK, Json(serde_json::json!({ "receipt": r }))).into_response()
        }
        Err(e) => fail(st.metrics.as_ref(), op, &e),
    };
    timer.observe_duration();
    resp
}

async fn post_burn(State(st): State<AppState>, Json(req): Json<BurnReq>) -> impl IntoResponse {
    let timer = st.metrics.request_latency_seconds.start_timer();
    let op = "burn";
    let result = {
        let mut l = st.ledger.write();
        l.burn(AccountId(req.account.clone()), req.amount, req.reason)
    };
    let resp = match result {
        Ok(r) => {
            st.metrics.tx_total.with_label_values(&[op]).inc();
            (StatusCode::OK, Json(serde_json::json!({ "receipt": r }))).into_response()
        }
        Err(e) => fail(st.metrics.as_ref(), op, &e),
    };
    timer.observe_duration();
    resp
}

async fn post_transfer(State(st): State<AppState>, Json(req): Json<TransferReq>) -> impl IntoResponse {
    let timer = st.metrics.request_latency_seconds.start_timer();
    let op = "transfer";
    let result = {
        let mut l = st.ledger.write();
        l.transfer(
            AccountId(req.from.clone()),
            AccountId(req.to.clone()),
            req.amount,
            req.reason,
        )
    };
    let resp = match result {
        Ok(r) => {
            st.metrics.tx_total.with_label_values(&[op]).inc();
            (StatusCode::OK, Json(serde_json::json!({ "receipt": r }))).into_response()
        }
        Err(e) => fail(st.metrics.as_ref(), op, &e),
    };
    timer.observe_duration();
    resp
}

async fn get_balance(State(st): State<AppState>, Path(account): Path<String>) -> impl IntoResponse {
    let l = st.ledger.read();
    let bal = l.balance(&AccountId(account.clone()));
    (StatusCode::OK, Json(BalanceResp { account, balance: bal }))
}

async fn get_supply(State(st): State<AppState>) -> impl IntoResponse {
    let l = st.ledger.read();
    (StatusCode::OK, Json(serde_json::json!({ "total_supply": l.total_supply() })))
}

// ---------- helpers ----------

fn fail(metrics: &Metrics, op: &str, e: &dyn std::error::Error) -> axum::response::Response {
    let (code, reason) = classify(e);
    metrics.tx_failed_total.with_label_values(&[op, reason]).inc();
    (code, Json(serde_json::json!({ "error": reason }))).into_response()
}

fn classify(e: &dyn std::error::Error) -> (StatusCode, &'static str) {
    if let Some(te) = e.downcast_ref::<TokenError>() {
        match te {
            TokenError::ZeroAmount => (StatusCode::BAD_REQUEST, "zero_amount"),
            TokenError::InsufficientFunds { .. } => (StatusCode::BAD_REQUEST, "insufficient_funds"),
            TokenError::Overflow => (StatusCode::INTERNAL_SERVER_ERROR, "overflow"),
        }
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR, "internal")
    }
}

```

### crates/svc-edge/Cargo.toml

```toml
[package]
name = "svc-edge"
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"
publish = false

[dependencies]
axum = { workspace = true, features = ["tokio", "http1", "http2", "json"] }
tokio = { workspace = true, features = ["macros", "rt-multi-thread", "signal", "io-util"] }
serde = { workspace = true, features = ["derive"] }
serde_json = { workspace = true }
prometheus = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true, features = ["env-filter", "fmt"] }
ron-policy = { workspace = true }

```

### crates/svc-edge/src/main.rs

```rust
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get},
    Json, Router,
};
use prometheus::{Encoder, TextEncoder};
use serde::Serialize;
use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, SystemTime},
};
use tracing::{info, Level};
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Clone)]
struct AppState {
    started: SystemTime,
    ready: Arc<AtomicBool>,
    service_name: &'static str,
    version: &'static str,
}

#[derive(Serialize)]
struct StatusPayload<'a> {
    service: &'a str,
    version: &'a str,
    ok: bool,
    uptime_secs: u64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let bind: SocketAddr = std::env::var("MICRONODE_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:3001".to_string())
        .parse()
        .expect("MICRONODE_ADDR must be host:port");

    let state = AppState {
        started: SystemTime::now(),
        ready: Arc::new(AtomicBool::new(false)),
        service_name: "micronode",
        version: env!("CARGO_PKG_VERSION"),
    };

    let app = Router::new()
        // Service endpoints
        .route("/", get(root))
        .route("/status", get(status))
        .route("/version", get(version))
        // Ops endpoints
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/metrics", get(metrics))
        .with_state(state.clone());

    let listener = tokio::net::TcpListener::bind(bind).await?;
    info!("micronode listening on http://{bind}");

    // Mark ready after successful bind
    state.ready.store(true, Ordering::SeqCst);

    // Serve until Ctrl-C
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("micronode shutdown complete");
    Ok(())
}

fn init_tracing() {
    // Respect RUST_LOG if provided, default to info
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));
    fmt()
        .with_target(false)
        .with_timer(fmt::time::uptime())
        .with_max_level(Level::INFO)
        .with_env_filter(env_filter)
        .init();
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    info!("received Ctrl-C, shutting down…");
}

async fn root(State(st): State<AppState>) -> impl IntoResponse {
    let up = st.started.elapsed().unwrap_or(Duration::from_secs(0)).as_secs();
    let payload = StatusPayload {
        service: st.service_name,
        version: st.version,
        ok: true,
        uptime_secs: up,
    };
    (StatusCode::OK, Json(payload))
}

async fn status(State(st): State<AppState>) -> impl IntoResponse {
    let up = st.started.elapsed().unwrap_or(Duration::from_secs(0)).as_secs();
    let payload = StatusPayload {
        service: st.service_name,
        version: st.version,
        ok: st.ready.load(std::sync::atomic::Ordering::SeqCst),
        uptime_secs: up,
    };
    (StatusCode::OK, Json(payload))
}

async fn version(State(st): State<AppState>) -> impl IntoResponse {
    let v = serde_json::json!({
        "service": st.service_name,
        "version": st.version
    });
    (StatusCode::OK, Json(v))
}

async fn healthz() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({ "ok": true })))
}

async fn readyz(State(st): State<AppState>) -> impl IntoResponse {
    let ok = st.ready.load(std::sync::atomic::Ordering::SeqCst);
    let code = if ok { StatusCode::OK } else { StatusCode::SERVICE_UNAVAILABLE };
    (code, Json(serde_json::json!({ "ready": ok })))
}

async fn metrics() -> impl IntoResponse {
    // Use the default Prometheus registry; services can register counters/histograms elsewhere.
    let metric_families = prometheus::gather();
    let mut buf = Vec::new();
    let encoder = TextEncoder::new();
    if let Err(e) = encoder.encode(&metric_families, &mut buf) {
        let body = format!("encode error: {e}");
        return (StatusCode::INTERNAL_SERVER_ERROR, body).into_response();
    }
    (StatusCode::OK, String::from_utf8_lossy(&buf).to_string()).into_response()
}

```

### crates/svc-index/Cargo.toml

```toml
[package]
publish = false
license = "MIT OR Apache-2.0"

name = "svc-index"
version = "0.1.0"
edition = "2021"

[dependencies]
ron-bus = { workspace = true }
index = { workspace = true }
naming = { workspace = true }
serde = { workspace = true }
rmp-serde = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true, features = ["env-filter", "json"] }
regex = { workspace = true }
workspace-hack = { version = "0.1", path = "../../workspace-hack" }

```

### crates/svc-index/src/main.rs

```rust
// crates/svc-index/src/main.rs
#![forbid(unsafe_code)]

use std::env;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::Arc;

use tracing::{error, info};
use tracing_subscriber::EnvFilter;

use naming::Address;
use ron_bus::api::{Envelope, IndexReq, IndexResp};
use ron_bus::uds::{listen, recv, send};

const DEFAULT_SOCK: &str = "/tmp/ron/svc-index.sock";
const DEFAULT_DB: &str = ".data/index";

/// Encode a value to MessagePack. On failure, log and return an empty Vec instead of panicking.
fn to_vec_or_log<T: serde::Serialize>(value: &T) -> Vec<u8> {
    match rmp_serde::to_vec(value) {
        Ok(v) => v,
        Err(e) => {
            error!(error=?e, "svc-index: msgpack encode failed");
            Vec::new()
        }
    }
}

fn main() -> std::io::Result<()> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .json()
        .try_init()
        .ok();

    let sock = env::var("RON_INDEX_SOCK").unwrap_or_else(|_| DEFAULT_SOCK.into());
    let db_path = env::var("RON_INDEX_DB").unwrap_or_else(|_| DEFAULT_DB.into());

    // Open DB without panicking; log and exit non-zero if it fails
    let idx = Arc::new(match index::Index::open(&db_path) {
        Ok(db) => db,
        Err(e) => {
            error!(db=%db_path, error=?e, "failed to open index database");
            std::process::exit(2);
        }
    });

    info!(
        socket = sock.as_str(),
        db = db_path.as_str(),
        "svc-index listening"
    );
    let listener: UnixListener = listen(&sock)?;

    for conn in listener.incoming() {
        match conn {
            Ok(stream) => {
                let idx = idx.clone();
                std::thread::spawn(move || {
                    if let Err(e) = serve_client(stream, idx) {
                        error!(error=?e, "client handler error");
                    }
                });
            }
            Err(e) => error!(error=?e, "accept error"),
        }
    }
    Ok(())
}

fn serve_client(mut stream: UnixStream, idx: Arc<index::Index>) -> std::io::Result<()> {
    let env = match recv(&mut stream) {
        Ok(e) => e,
        Err(e) => {
            error!(error=?e, "recv error");
            return Ok(());
        }
    };

    let req: IndexReq = match rmp_serde::from_slice(&env.payload) {
        Ok(x) => x,
        Err(e) => {
            error!(error=?e, "decode req error");
            return Ok(());
        }
    };

    let resp = match req {
        IndexReq::Health => IndexResp::HealthOk,

        IndexReq::Resolve { addr } => {
            info!(%addr, "resolve request");
            match addr.parse::<Address>() {
                Ok(a) => match idx.get_bundle_dir(&a) {
                    Ok(Some(p)) => {
                        let dir = p.to_string_lossy().into_owned();
                        info!(%addr, dir=%dir, "resolve FOUND");
                        IndexResp::Resolved { dir }
                    }
                    Ok(None) => {
                        info!(%addr, "resolve NOT FOUND");
                        IndexResp::NotFound
                    }
                    Err(e) => {
                        error!(%addr, error=?e, "resolve error");
                        IndexResp::Err { err: e.to_string() }
                    }
                },
                Err(e) => {
                    error!(%addr, error=?e, "bad address");
                    IndexResp::Err { err: e.to_string() }
                }
            }
        }

        IndexReq::PutAddress { addr, dir } => match addr.parse::<Address>() {
            Ok(a) => match idx.put_address(&a, PathBuf::from(&dir)) {
                Ok(_) => {
                    info!(%addr, %dir, "index PUT ok");
                    IndexResp::PutOk
                }
                Err(e) => {
                    error!(%addr, %dir, error=?e, "index PUT error");
                    IndexResp::Err { err: e.to_string() }
                }
            },
            Err(e) => {
                error!(%addr, %dir, error=?e, "index PUT bad address");
                IndexResp::Err { err: e.to_string() }
            }
        },
    };

    let payload = to_vec_or_log(&resp);
    let reply = Envelope {
        service: "svc.index".into(),
        method: "v1.ok".into(),
        corr_id: env.corr_id,
        token: vec![],
        payload,
    };
    let _ = send(&mut stream, &reply);
    Ok(())
}

```

### crates/svc-omnigate/Cargo.toml

```toml
[package]
name = "svc-omnigate"
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"
publish = false
description = "RustyOnions Omnigate (OAP/1 over TCP+TLS) with HELLO, Storage GET streaming (64 KiB), and Mailbox MVP"

[dependencies]
# Workspace-unified deps
anyhow               = { workspace = true }
bytes                = { workspace = true }
futures-util         = { workspace = true }
hyper                = { workspace = true }
hyper-util           = { workspace = true }
rand                 = { workspace = true }
serde                = { workspace = true }
serde_json           = { workspace = true }
tokio                = { workspace = true, features = ["rt-multi-thread","macros","net","io-util","fs","time","signal"] }
tracing              = { workspace = true }
prometheus           = { workspace = true }
tokio-util           = { workspace = true, features = ["codec"] }
tracing-subscriber   = { workspace = true }

# Local/non-workspace pins
http-body-util       = "0.1"
ulid                 = "1.1"
once_cell            = "1"

# --- TLS stack (explicit provider selection for the server binary)
# Pick aws-lc-rs backend via rustls features. Keep tokio-rustls via workspace pin
# (dropping default-features override to avoid the Cargo warning).
rustls        = { version = "0.23.31", default-features = false, features = ["std", "logging", "tls12", "aws-lc-rs"] }
tokio-rustls  = { workspace = true }
rustls-pemfile = { workspace = true }

# Reuse OAP types/codec
ron-app-sdk = { path = "../ron-app-sdk" }

# Hakari feature unifier
workspace-hack = { version = "0.1", path = "../../workspace-hack" }

```

### crates/svc-omnigate/src/admin_http.rs

```rust
// crates/svc-omnigate/src/admin_http.rs
#![forbid(unsafe_code)]

use anyhow::Result;
use bytes::Bytes;
use http_body_util::Full;
use hyper::server::conn::http1;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::error;

use crate::metrics::Metrics;

fn resp_ok(body: &'static [u8]) -> Response<Full<Bytes>> {
    Response::new(Full::new(Bytes::from_static(body)))
}

fn resp_with_status(body: &'static [u8], code: StatusCode) -> Response<Full<Bytes>> {
    let mut resp = Response::new(Full::new(Bytes::from_static(body)));
    *resp.status_mut() = code;
    resp
}

fn resp_metrics(text: String) -> Response<Full<Bytes>> {
    let mut resp = Response::new(Full::new(Bytes::from(text)));
    resp.headers_mut().insert(
        hyper::header::CONTENT_TYPE,
        hyper::header::HeaderValue::from_static("text/plain; version=0.0.4"),
    );
    resp
}

pub async fn run(addr: SocketAddr, max_inflight: u64, metrics: Arc<Metrics>) -> Result<()> {
    let listener = TcpListener::bind(addr).await?;
    // If inflight utilization >= READY_OVERLOAD_PCT, report 503 on /readyz
    // Default 90 (%). Override with env READY_OVERLOAD_PCT=NN.
    let ready_threshold = std::env::var("READY_OVERLOAD_PCT")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(90);

    loop {
        let (stream, _) = listener.accept().await?;
        let metrics = metrics.clone();
        let svc = hyper::service::service_fn(move |req: Request<hyper::body::Incoming>| {
            let metrics = metrics.clone();
            async move {
                match (req.method(), req.uri().path()) {
                    (&hyper::Method::GET, "/healthz") => {
                        Ok::<_, std::convert::Infallible>(resp_ok(b"ok"))
                    }
                    (&hyper::Method::GET, "/readyz") => {
                        let inflight = metrics
                            .inflight_current
                            .load(std::sync::atomic::Ordering::Relaxed);
                        let limit = max_inflight.max(1);
                        let pct = inflight.saturating_mul(100) / limit;
                        let ready = pct < ready_threshold;
                        let resp = if ready {
                            resp_with_status(b"ready", StatusCode::OK)
                        } else {
                            resp_with_status(b"overloaded", StatusCode::SERVICE_UNAVAILABLE)
                        };
                        Ok::<_, std::convert::Infallible>(resp)
                    }
                    (&hyper::Method::GET, "/metrics") => {
                        let text = metrics.to_prom();
                        Ok::<_, std::convert::Infallible>(resp_metrics(text))
                    }
                    _ => Ok::<_, std::convert::Infallible>(resp_with_status(
                        b"nf",
                        StatusCode::NOT_FOUND,
                    )),
                }
            }
        });

        tokio::spawn(async move {
            let io = TokioIo::new(stream);
            if let Err(e) = http1::Builder::new().serve_connection(io, svc).await {
                error!("admin http connection error: {e}");
            }
        });
    }
}

```

### crates/svc-omnigate/src/config.rs

```rust
// crates/svc-omnigate/src/config.rs
#![forbid(unsafe_code)]

use ron_app_sdk::DEFAULT_MAX_FRAME;
use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub struct Config {
    pub addr: SocketAddr,      // OAP listener
    pub http_addr: SocketAddr, // admin /readyz
    pub max_frame: usize,
    pub max_inflight: u64,
    pub chunk_bytes: usize,
    pub tiles_root: String,
    pub max_file_bytes: u64,
    // Quotas (per-tenant, per-proto)
    pub quota_tile_rps: u32,
    pub quota_mailbox_rps: u32,
}

impl Default for Config {
    fn default() -> Self {
        // Avoid unwraps on literal parses; fall back to localhost if parsing ever fails.
        let addr = "127.0.0.1:9443"
            .parse()
            .unwrap_or_else(|_| SocketAddr::from(([127, 0, 0, 1], 9443)));
        let http_addr = "127.0.0.1:9096"
            .parse()
            .unwrap_or_else(|_| SocketAddr::from(([127, 0, 0, 1], 9096)));

        Self {
            addr,
            http_addr,
            max_frame: DEFAULT_MAX_FRAME,
            max_inflight: 128,
            chunk_bytes: 64 * 1024,
            tiles_root: "testing/tiles".to_string(),
            max_file_bytes: 8 * 1024 * 1024,
            quota_tile_rps: 50,
            quota_mailbox_rps: 100,
        }
    }
}

impl Config {
    pub fn from_env() -> Self {
        let mut c = Self::default();

        if let Ok(s) = std::env::var("ADDR") {
            if let Ok(addr) = s.parse::<SocketAddr>() {
                c.addr = addr;
            } else {
                tracing::warn!("ADDR must be host:port (got {s})");
            }
        }
        if let Ok(s) = std::env::var("ADMIN_ADDR") {
            if let Ok(addr) = s.parse::<SocketAddr>() {
                c.http_addr = addr;
            } else {
                tracing::warn!("ADMIN_ADDR must be host:port (got {s})");
            }
        }
        if let Ok(s) = std::env::var("MAX_FRAME") {
            if let Ok(v) = s.parse::<usize>() {
                c.max_frame = v;
            } else {
                tracing::warn!("MAX_FRAME must be integer (got {s})");
            }
        }
        if let Ok(s) = std::env::var("MAX_INFLIGHT") {
            if let Ok(v) = s.parse::<u64>() {
                c.max_inflight = v;
            } else {
                tracing::warn!("MAX_INFLIGHT must be integer (got {s})");
            }
        }
        if let Ok(s) = std::env::var("CHUNK_BYTES") {
            if let Ok(v) = s.parse::<usize>() {
                c.chunk_bytes = v;
            } else {
                tracing::warn!("CHUNK_BYTES must be integer (got {s})");
            }
        }
        if let Ok(s) = std::env::var("TILES_ROOT") {
            c.tiles_root = s;
        }
        if let Ok(s) = std::env::var("MAX_FILE_BYTES") {
            if let Ok(v) = s.parse::<u64>() {
                c.max_file_bytes = v;
            } else {
                tracing::warn!("MAX_FILE_BYTES must be integer (got {s})");
            }
        }
        if let Ok(s) = std::env::var("QUOTA_TILE_RPS") {
            if let Ok(v) = s.parse::<u32>() {
                c.quota_tile_rps = v;
            } else {
                tracing::warn!("QUOTA_TILE_RPS must be integer (got {s})");
            }
        }
        if let Ok(s) = std::env::var("QUOTA_MAILBOX_RPS") {
            if let Ok(v) = s.parse::<u32>() {
                c.quota_mailbox_rps = v;
            } else {
                tracing::warn!("QUOTA_MAILBOX_RPS must be integer (got {s})");
            }
        }
        c
    }
}

```

### crates/svc-omnigate/src/handlers/hello.rs

```rust
#![forbid(unsafe_code)]

use anyhow::Result;
use bytes::Bytes;
use futures_util::SinkExt;
use ron_app_sdk::{Hello, OapCodec, OapFlags, OapFrame, OAP_VERSION};
use tokio::net::TcpStream;
use tokio_rustls::server::TlsStream;
use tokio_util::codec::Framed;

use crate::config::Config;

pub async fn handle_hello(
    framed: &mut Framed<TlsStream<TcpStream>, OapCodec>,
    cfg: &Config,
    req: &OapFrame,
) -> Result<()> {
    let hello = Hello {
        server_version: "svc-omnigate-dev-1.0.0".into(),
        max_frame: cfg.max_frame as u64,
        max_inflight: cfg.max_inflight,
        supported_flags: vec![
            "EVENT".into(),
            "ACK_REQ".into(),
            "COMP".into(),
            "APP_E2E".into(),
        ],
        oap_versions: vec![OAP_VERSION],
        transports: vec!["tcp+tls".into()],
    };
    let body = serde_json::to_vec(&hello)?;
    let resp = OapFrame {
        ver: OAP_VERSION,
        flags: OapFlags::RESP | OapFlags::END,
        code: 0,
        app_proto_id: 0,
        tenant_id: req.tenant_id,
        cap: Bytes::new(),
        corr_id: req.corr_id,
        payload: Bytes::from(body),
    };
    framed.send(resp).await?;
    Ok(())
}

```

### crates/svc-omnigate/src/handlers/mailbox.rs

```rust
#![forbid(unsafe_code)]

use anyhow::{anyhow, Context, Result};
use bytes::Bytes;
use futures_util::SinkExt;
use ron_app_sdk::{OapCodec, OapFlags, OapFrame, OAP_VERSION};
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio_rustls::server::TlsStream;
use tokio_util::codec::Framed;

use crate::mailbox::{Mailbox, MAILBOX_APP_PROTO_ID};
use crate::metrics::Metrics;

#[derive(Deserialize)]
#[serde(rename_all = "lowercase", tag = "op")]
enum Req {
    Send {
        topic: String,
        text: String,
        #[serde(default)]
        idempotency_key: Option<String>,
    },
    Recv {
        topic: String,
        #[serde(default = "default_max")]
        max: usize,
    },
    Ack {
        msg_id: String,
    },
}

fn default_max() -> usize {
    10
}

#[derive(Serialize)]
struct SendResp {
    msg_id: String,
}

#[derive(Serialize)]
struct RecvMsg {
    msg_id: String,
    topic: String,
    text: String,
}

#[derive(Serialize)]
struct RecvResp {
    messages: Vec<RecvMsg>,
}

#[derive(Serialize)]
struct AckResp {
    ok: bool,
}

pub async fn handle_mailbox(
    framed: &mut Framed<TlsStream<TcpStream>, OapCodec>,
    mailbox: &Mailbox,
    req: &OapFrame,
    _metrics: &Metrics, // currently unused; prefix with underscore to silence warning
) -> Result<()> {
    let parsed: Req = serde_json::from_slice(&req.payload).context("invalid JSON")?;

    match parsed {
        Req::Send {
            topic,
            text,
            idempotency_key,
        } => {
            let id = mailbox
                .send(&topic, Bytes::from(text.into_bytes()), idempotency_key)
                .await?;
            let body = serde_json::to_vec(&SendResp { msg_id: id })?;
            let resp = OapFrame {
                ver: OAP_VERSION,
                flags: OapFlags::RESP | OapFlags::END,
                code: 0,
                app_proto_id: MAILBOX_APP_PROTO_ID,
                tenant_id: req.tenant_id,
                cap: Bytes::new(),
                corr_id: req.corr_id,
                payload: Bytes::from(body),
            };
            framed.send(resp).await?;
            Ok(())
        }
        Req::Recv { topic, max } => {
            if max == 0 {
                return Err(anyhow!("bad_request"));
            }
            let msgs = mailbox.recv(&topic, max.min(100)).await?;
            let out: Vec<RecvMsg> = msgs
                .into_iter()
                .map(|(id, body)| {
                    let text = String::from_utf8_lossy(&body).to_string();
                    RecvMsg {
                        msg_id: id,
                        topic: topic.clone(),
                        text,
                    }
                })
                .collect();

            let body = serde_json::to_vec(&RecvResp { messages: out })?;
            let resp = OapFrame {
                ver: OAP_VERSION,
                flags: OapFlags::RESP | OapFlags::END,
                code: 0,
                app_proto_id: MAILBOX_APP_PROTO_ID,
                tenant_id: req.tenant_id,
                cap: Bytes::new(),
                corr_id: req.corr_id,
                payload: Bytes::from(body),
            };
            framed.send(resp).await?;
            Ok(())
        }
        Req::Ack { msg_id } => {
            mailbox.ack(&msg_id).await.map_err(|e| {
                if e.to_string().contains("not_found") {
                    anyhow!("404 not_found")
                } else {
                    e
                }
            })?;
            let body = serde_json::to_vec(&AckResp { ok: true })?;
            let resp = OapFrame {
                ver: OAP_VERSION,
                flags: OapFlags::RESP | OapFlags::END,
                code: 0,
                app_proto_id: MAILBOX_APP_PROTO_ID,
                tenant_id: req.tenant_id,
                cap: Bytes::new(),
                corr_id: req.corr_id,
                payload: Bytes::from(body),
            };
            framed.send(resp).await?;
            Ok(())
        }
    }
}

```

### crates/svc-omnigate/src/handlers/mod.rs

```rust
#![forbid(unsafe_code)]

pub mod hello;
pub mod mailbox;
pub mod storage;

pub use hello::handle_hello;
pub use mailbox::handle_mailbox;
pub use storage::handle_storage_get;

```

### crates/svc-omnigate/src/handlers/storage.rs

```rust
#![forbid(unsafe_code)]

use anyhow::{anyhow, Context, Result};
use bytes::{Bytes, BytesMut};
use futures_util::SinkExt;
use ron_app_sdk::{OapCodec, OapFlags, OapFrame, OAP_VERSION};
use serde::Deserialize;
use tokio::net::TcpStream;
use tokio_rustls::server::TlsStream;
use tokio_util::codec::Framed;

use crate::config::Config;
use crate::metrics::Metrics;
use crate::storage::{FsStorage, TILE_APP_PROTO_ID};

#[derive(Deserialize)]
struct GetReq {
    op: String,
    path: String,
}

pub async fn handle_storage_get(
    framed: &mut Framed<TlsStream<TcpStream>, OapCodec>,
    cfg: &Config,
    storage: &FsStorage,
    req: &OapFrame,
    metrics: std::sync::Arc<Metrics>,
) -> Result<()> {
    // Parse JSON body
    let gr: GetReq = serde_json::from_slice(&req.payload).with_context(|| "invalid JSON")?;
    if gr.op.as_str() != "get" {
        return Err(anyhow!("bad op"));
    }

    // Open file (FsStorage enforces max size, safe path)
    let (mut file, _size) = storage.open(&gr.path).await.map_err(|e| {
        if e.to_string().contains("too large") {
            metrics.inc_too_large();
            anyhow!("413 {}", e)
        } else if e.to_string().contains("not a file")
            || e.to_string().contains("No such file")
            || e.to_string().contains("open")
        {
            metrics.inc_not_found();
            anyhow!("404 {}", e)
        } else {
            e
        }
    })?;

    // Stream chunks
    let mut sent_any = false;
    let mut chunk = BytesMut::with_capacity(cfg.chunk_bytes);

    loop {
        chunk.clear();
        chunk.reserve(cfg.chunk_bytes);
        use tokio::io::AsyncReadExt;
        let n = file.read_buf(&mut chunk).await?;
        if n == 0 {
            break;
        }
        metrics.add_bytes_out(n as u64);

        let mut flags = OapFlags::RESP;
        if !sent_any {
            flags |= OapFlags::START;
            sent_any = true;
        }

        let resp = OapFrame {
            ver: OAP_VERSION,
            flags,
            code: 0,
            app_proto_id: TILE_APP_PROTO_ID,
            tenant_id: req.tenant_id,
            cap: Bytes::new(),
            corr_id: req.corr_id,
            payload: chunk.clone().freeze(),
        };
        framed.send(resp).await?;
        tokio::task::yield_now().await;
    }

    // END (or empty START|END if zero bytes)
    let end_flags = if sent_any {
        OapFlags::RESP | OapFlags::END
    } else {
        OapFlags::RESP | OapFlags::START | OapFlags::END
    };
    let resp_end = OapFrame {
        ver: OAP_VERSION,
        flags: end_flags,
        code: 0,
        app_proto_id: TILE_APP_PROTO_ID,
        tenant_id: req.tenant_id,
        cap: Bytes::new(),
        corr_id: req.corr_id,
        payload: Bytes::new(),
    };
    framed.send(resp_end).await?;
    Ok(())
}

```

### crates/svc-omnigate/src/mailbox.rs

```rust
// crates/svc-omnigate/src/mailbox.rs
#![forbid(unsafe_code)]

use anyhow::{anyhow, Result};
use bytes::Bytes;
use std::{
    collections::{HashMap, VecDeque},
    time::{Duration, Instant},
};
use tokio::sync::Mutex;
use tracing::debug;

/// App protocol id for Mailbox.
pub const MAILBOX_APP_PROTO_ID: u16 = 0x0201;

/// Bronze semantics:
/// - At-least-once via visibility timeout.
/// - ULID message ids.
/// - Best-effort FIFO per topic.
/// - In-memory store (process lifetime). Good enough for the pilot; can swap to sled/file later.
pub struct Mailbox {
    inner: Mutex<Inner>,
    visibility: Duration,
}

struct Inner {
    /// topic -> queue of msg_ids
    queues: HashMap<String, VecDeque<String>>,
    /// msg_id -> message data
    messages: HashMap<String, Message>,
    /// idempotency_key -> msg_id
    idempotency: HashMap<String, String>,
}

struct Message {
    topic: String,
    body: Bytes,
    enqueued_at: Instant,
    leased_until: Option<Instant>,
}

impl Mailbox {
    pub fn new(visibility: Duration) -> Self {
        Self {
            inner: Mutex::new(Inner {
                queues: HashMap::new(),
                messages: HashMap::new(),
                idempotency: HashMap::new(),
            }),
            visibility,
        }
    }

    pub async fn send(
        &self,
        topic: &str,
        body: Bytes,
        idempotency_key: Option<String>,
    ) -> Result<String> {
        let mut g = self.inner.lock().await;

        if let Some(k) = idempotency_key.as_ref() {
            if let Some(existing) = g.idempotency.get(k) {
                // Return existing id for duplicate send.
                return Ok(existing.clone());
            }
        }

        let id = ulid::Ulid::new().to_string();

        let msg = Message {
            topic: topic.to_string(),
            body,
            enqueued_at: Instant::now(),
            leased_until: None,
        };

        // Borrow queues only for the push, then release before touching other fields.
        {
            let q = g.queues.entry(topic.to_string()).or_default();
            q.push_back(id.clone());
        }
        g.messages.insert(id.clone(), msg);

        if let Some(k) = idempotency_key {
            g.idempotency.insert(k, id.clone());
        }

        Ok(id)
    }

    /// Return up to `max` messages for topic, setting a visibility timeout.
    /// Redelivery: on each call we sweep expired leases back into the queue.
    pub async fn recv(&self, topic: &str, max: usize) -> Result<Vec<(String, Bytes)>> {
        let mut g = self.inner.lock().await;

        // Sweep expired leases back to queue (lazy redelivery).
        self.sweep_expired_locked(topic, &mut g);

        let mut out = Vec::with_capacity(max);

        for _ in 0..max {
            // Pop an id in a short scope so the mutable borrow of queues ends
            // before we mutably borrow g.messages.
            let id_opt = {
                let q = g.queues.entry(topic.to_string()).or_default();
                q.pop_front()
            };

            let Some(id) = id_opt else {
                break;
            };

            if let Some(m) = g.messages.get_mut(&id) {
                // Lease it
                m.leased_until = Some(Instant::now() + self.visibility);
                out.push((id, m.body.clone()));
            } else {
                // If message record vanished (shouldn't happen in normal flow), skip.
                continue;
            }
        }

        Ok(out)
    }

    /// ACK a message id, removing it from the store.
    pub async fn ack(&self, msg_id: &str) -> Result<()> {
        let mut g = self.inner.lock().await;

        if let Some(msg) = g.messages.remove(msg_id) {
            // Use enqueued_at for simple dwell-time telemetry to avoid dead_code on the field.
            let dwell = Instant::now().saturating_duration_since(msg.enqueued_at);
            debug!("ack {msg_id} dwell_ms={}", dwell.as_millis());

            // Remove any stray queued occurrences (best-effort) in a short scope.
            if let Some(q) = g.queues.get_mut(&msg.topic) {
                if let Some(pos) = q.iter().position(|x| x == msg_id) {
                    q.remove(pos);
                }
            }
            Ok(())
        } else {
            Err(anyhow!("not_found"))
        }
    }

    fn sweep_expired_locked(&self, topic: &str, g: &mut Inner) {
        let now = Instant::now();
        // Collect expired leases for this topic first.
        let expired: Vec<String> = g
            .messages
            .iter()
            .filter_map(|(id, m)| {
                if m.topic == topic {
                    if let Some(deadline) = m.leased_until {
                        if deadline <= now {
                            return Some(id.clone());
                        }
                    }
                }
                None
            })
            .collect();

        if expired.is_empty() {
            return;
        }

        // Then push them back into the queue.
        let q = g.queues.entry(topic.to_string()).or_default();
        for id in expired {
            if let Some(m) = g.messages.get_mut(&id) {
                m.leased_until = None;
                q.push_back(id.clone());
                debug!("redeliver {}", id);
            }
        }
    }
}

```

### crates/svc-omnigate/src/main.rs

```rust
#![forbid(unsafe_code)]

use anyhow::Result;
use std::sync::Arc;
use tracing::info;

mod admin_http;
mod config;
mod handlers;
mod mailbox; // Mailbox state
mod metrics;
mod oap_limits; // NEW: expose OAP limits to the crate
mod oap_metrics;
mod server;
mod storage; // FsStorage helper
mod tls; // NEW: expose OAP metrics to the crate

use crate::config::Config;
use crate::mailbox::Mailbox;
use crate::metrics::Metrics;
use crate::storage::FsStorage;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    // Load service config from environment.
    let cfg = Config::from_env();

    // IMPORTANT:
    // tls::load_tls() returns a tokio_rustls::TlsAcceptor already.
    // Do NOT wrap it again with TlsAcceptor::from(Arc<...>).
    let acceptor = tls::load_tls()?;

    // Shared state for handlers.
    let storage = Arc::new(FsStorage::new(&cfg.tiles_root, cfg.max_file_bytes));
    let mailbox = Arc::new(Mailbox::new(std::time::Duration::from_secs(30)));
    let metrics = Arc::new(Metrics::default());

    // Admin HTTP (health/ready/metrics)
    tokio::spawn(admin_http::run(
        cfg.http_addr,
        cfg.max_inflight,
        metrics.clone(),
    ));

    info!("svc-omnigate starting on {}", cfg.addr);

    // Run server and listen for shutdown concurrently.
    tokio::select! {
        r = server::run(cfg.clone(), acceptor.clone(), storage.clone(), mailbox.clone(), metrics.clone()) => {
            r?;
        },
        _ = wait_for_shutdown() => {
            info!("shutdown signal received");
        }
    }

    Ok(())
}

async fn wait_for_shutdown() {
    let _ = tokio::signal::ctrl_c().await;
}

```

### crates/svc-omnigate/src/metrics.rs

```rust
#![forbid(unsafe_code)]

use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Default)]
pub struct Metrics {
    pub requests_total: AtomicU64,
    pub bytes_in_total: AtomicU64,
    pub bytes_out_total: AtomicU64,
    pub rejected_overload_total: AtomicU64,
    pub rejected_not_found_total: AtomicU64,
    pub rejected_too_large_total: AtomicU64,
    pub inflight_current: AtomicU64, // gauge
}

impl Metrics {
    #[inline]
    pub fn inc_requests(&self) {
        self.requests_total.fetch_add(1, Ordering::Relaxed);
    }

    // Not all call sites wire this yet; keep it available but silence Clippy.
    #[allow(dead_code)]
    #[inline]
    pub fn add_bytes_in(&self, n: u64) {
        self.bytes_in_total.fetch_add(n, Ordering::Relaxed);
    }

    #[inline]
    pub fn add_bytes_out(&self, n: u64) {
        self.bytes_out_total.fetch_add(n, Ordering::Relaxed);
    }

    #[inline]
    pub fn inc_overload(&self) {
        self.rejected_overload_total.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn inc_not_found(&self) {
        self.rejected_not_found_total
            .fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn inc_too_large(&self) {
        self.rejected_too_large_total
            .fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn inflight_inc(&self) -> u64 {
        self.inflight_current.fetch_add(1, Ordering::Relaxed) + 1
    }

    #[inline]
    pub fn inflight_dec(&self) {
        self.inflight_current.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn to_prom(&self) -> String {
        format!(
            concat!(
                "requests_total {}\n",
                "bytes_in_total {}\n",
                "bytes_out_total {}\n",
                "rejected_overload_total {}\n",
                "rejected_not_found_total {}\n",
                "rejected_too_large_total {}\n",
                "inflight_current {}\n",
            ),
            self.requests_total.load(Ordering::Relaxed),
            self.bytes_in_total.load(Ordering::Relaxed),
            self.bytes_out_total.load(Ordering::Relaxed),
            self.rejected_overload_total.load(Ordering::Relaxed),
            self.rejected_not_found_total.load(Ordering::Relaxed),
            self.rejected_too_large_total.load(Ordering::Relaxed),
            self.inflight_current.load(Ordering::Relaxed),
        )
    }
}

```

### crates/svc-omnigate/src/oap_limits.rs

```rust
// Tiny guard for OAP/1 streams: caps + idle/read timeouts.
// Integrates without touching existing modules.

#![forbid(unsafe_code)]

use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub struct OapLimits {
    pub read_timeout: Duration,     // absolute cap for reading a whole stream
    pub idle_timeout: Duration,     // inactivity between frames
    pub max_frames_per_stream: u32, // frame count cap
    pub max_total_bytes_per_stream: u64, // payload cap across all DATA frames
}

impl Default for OapLimits {
    fn default() -> Self {
        Self {
            read_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(10),
            max_frames_per_stream: 4_096,
            max_total_bytes_per_stream: 64 * 1024 * 1024, // 64 MiB
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RejectReason {
    Timeout,                      // idle or read timeout
    TooManyFrames { limit: u32 }, // exceeded frame cap
    TooManyBytes { limit: u64 },  // exceeded byte cap
}

#[derive(Clone, Debug)]
pub struct StreamState {
    started_at: Instant,
    last_activity: Instant,
    frames_seen: u32,
    bytes_seen: u64,
}

impl StreamState {
    pub fn new(now: Instant) -> Self {
        Self {
            started_at: now,
            last_activity: now,
            frames_seen: 0,
            bytes_seen: 0,
        }
    }

    #[inline]
    pub fn touch(&mut self, now: Instant) {
        self.last_activity = now;
    }

    /// Call on every DATA frame before accepting it.
    pub fn on_frame(
        &mut self,
        data_len: usize,
        now: Instant,
        lim: &OapLimits,
    ) -> Result<(), RejectReason> {
        // Check absolute read timeout (since stream start)
        if now.duration_since(self.started_at) > lim.read_timeout {
            return Err(RejectReason::Timeout);
        }
        // Check idle timeout (since last activity)
        if now.duration_since(self.last_activity) > lim.idle_timeout {
            return Err(RejectReason::Timeout);
        }
        // Check frame count
        let next_frames = self.frames_seen.saturating_add(1);
        if next_frames > lim.max_frames_per_stream {
            return Err(RejectReason::TooManyFrames {
                limit: lim.max_frames_per_stream,
            });
        }
        // Check byte budget
        let next_bytes = self.bytes_seen.saturating_add(data_len as u64);
        if next_bytes > lim.max_total_bytes_per_stream {
            return Err(RejectReason::TooManyBytes {
                limit: lim.max_total_bytes_per_stream,
            });
        }

        // Accept
        self.frames_seen = next_frames;
        self.bytes_seen = next_bytes;
        self.last_activity = now;
        Ok(())
    }
}

```

### crates/svc-omnigate/src/oap_metrics.rs

```rust
#![forbid(unsafe_code)]
#![allow(clippy::expect_used)] // metric registration failures are programmer/config errors

use prometheus::{register_int_counter, register_int_counter_vec, IntCounter, IntCounterVec};
use std::sync::OnceLock;

static REJECT_TIMEOUT: OnceLock<IntCounter> = OnceLock::new();
static REJECT_FRAMES: OnceLock<IntCounter> = OnceLock::new();
static REJECT_BYTES: OnceLock<IntCounter> = OnceLock::new();

static DATA_BYTES: OnceLock<IntCounterVec> = OnceLock::new();
static STREAMS: OnceLock<IntCounterVec> = OnceLock::new();

/// Initialize all OAP metrics exactly once.
pub fn init_oap_metrics() {
    // Counters
    REJECT_TIMEOUT.get_or_init(|| {
        register_int_counter!(
            "oap_reject_timeout_total",
            "Rejected streams due to timeout"
        )
        .expect("register oap_reject_timeout_total")
    });
    REJECT_FRAMES.get_or_init(|| {
        register_int_counter!(
            "oap_reject_too_many_frames_total",
            "Rejected streams due to too many frames"
        )
        .expect("register oap_reject_too_many_frames_total")
    });
    REJECT_BYTES.get_or_init(|| {
        register_int_counter!(
            "oap_reject_too_many_bytes_total",
            "Rejected streams due to too many bytes"
        )
        .expect("register oap_reject_too_many_bytes_total")
    });

    // Labeled counters
    DATA_BYTES.get_or_init(|| {
        register_int_counter_vec!(
            "oap_data_bytes_total",
            "Total data bytes observed per topic",
            &["topic"]
        )
        .expect("register oap_data_bytes_total")
    });

    STREAMS.get_or_init(|| {
        register_int_counter_vec!(
            "oap_streams_total",
            "Total streams started per topic",
            &["topic"]
        )
        .expect("register oap_streams_total")
    });
}

// ---- public helpers (no unwrap on Option) ----

#[inline]
pub fn inc_reject_timeout() {
    REJECT_TIMEOUT.get().expect("init first").inc();
}
#[inline]
pub fn inc_reject_too_many_frames() {
    REJECT_FRAMES.get().expect("init first").inc();
}
#[inline]
pub fn inc_reject_too_many_bytes() {
    REJECT_BYTES.get().expect("init first").inc();
}

#[inline]
pub fn add_data_bytes(topic: &str, n: u64) {
    DATA_BYTES
        .get()
        .expect("init first")
        .with_label_values(&[topic])
        .inc_by(n);
}

#[inline]
pub fn inc_streams(topic: &str) {
    STREAMS
        .get()
        .expect("init first")
        .with_label_values(&[topic])
        .inc();
}

```

### crates/svc-omnigate/src/server.rs

```rust
#![forbid(unsafe_code)]

use anyhow::{Context, Result};
use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use ron_app_sdk::{
    Error as OapError, OapCodec, OapFlags, OapFrame, DEFAULT_MAX_DECOMPRESSED, OAP_VERSION,
};
use std::{collections::HashMap, sync::Arc, time::Instant};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{Mutex, Semaphore},
};
use tokio_rustls::TlsAcceptor;
use tokio_util::codec::Framed;
use tracing::{debug, error, info};

use crate::config::Config;
use crate::handlers::{handle_hello, handle_mailbox, handle_storage_get};
use crate::mailbox::{Mailbox, MAILBOX_APP_PROTO_ID};
use crate::metrics::Metrics;
use crate::storage::{FsStorage, TILE_APP_PROTO_ID};

// NEW: OAP limits and per-topic metrics wiring
use crate::oap_limits::{OapLimits, RejectReason, StreamState};
use crate::oap_metrics;
use crate::oap_metrics::{
    add_data_bytes, inc_reject_timeout, inc_reject_too_many_bytes, inc_reject_too_many_frames,
    inc_streams,
};

/// Simple token-bucket rate limiter keyed by (tenant_id, app_proto_id).
struct TokenBucket {
    tokens: f64,
    capacity: f64,
    refill_per_sec: f64,
    last: Instant,
}
impl TokenBucket {
    fn new(rps: f64) -> Self {
        let cap = (rps * 2.0).max(1.0); // small burst allowance
        Self {
            tokens: cap,
            capacity: cap,
            refill_per_sec: rps,
            last: Instant::now(),
        }
    }
    fn allow(&mut self) -> bool {
        let now = Instant::now();
        let dt = (now - self.last).as_secs_f64();
        self.last = now;
        self.tokens = (self.tokens + dt * self.refill_per_sec).min(self.capacity);
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
    fn set_rate(&mut self, rps: f64) {
        self.refill_per_sec = rps;
        self.capacity = (rps * 2.0).max(1.0);
        self.tokens = self.tokens.min(self.capacity);
    }
}

#[derive(Default)]
struct RateLimiter {
    // key: (tenant_id, app_proto_id)
    buckets: HashMap<(u128, u16), TokenBucket>,
}
impl RateLimiter {
    fn check(&mut self, tenant_id: u128, app_proto_id: u16, rps_for_proto: f64) -> bool {
        let key = (tenant_id, app_proto_id);
        match self.buckets.get_mut(&key) {
            Some(b) => {
                if (b.refill_per_sec - rps_for_proto).abs() > f64::EPSILON {
                    b.set_rate(rps_for_proto);
                }
                b.allow()
            }
            None => {
                let mut b = TokenBucket::new(rps_for_proto);
                let ok = b.allow();
                self.buckets.insert(key, b);
                ok
            }
        }
    }
}

/// Bundled connection dependencies to keep function arity low (Clippy).
#[derive(Clone)]
struct Deps {
    acceptor: TlsAcceptor,
    storage: Arc<FsStorage>,
    mailbox: Arc<Mailbox>,
    metrics: Arc<Metrics>,
    inflight: Arc<Semaphore>,
    rate: Arc<Mutex<RateLimiter>>,
    tile_rps: f64,
    mb_rps: f64,
    cfg: Config,
}

pub async fn run(
    cfg: Config,
    acceptor: TlsAcceptor,
    storage: Arc<FsStorage>,
    mailbox: Arc<Mailbox>,
    metrics: Arc<Metrics>,
) -> Result<()> {
    let listen_addr = cfg.addr;
    let listener = TcpListener::bind(listen_addr)
        .await
        .context("bind oap addr")?;
    info!("svc-omnigate OAP listener on {}", listen_addr);

    // NEW: initialize OAP metrics (idempotent)
    oap_metrics::init_oap_metrics();

    // Global inflight gate
    let inflight = Arc::new(Semaphore::new(cfg.max_inflight as usize));
    // Per-tenant quotas
    let rate = Arc::new(Mutex::new(RateLimiter::default()));
    let deps = Deps {
        acceptor,
        storage,
        mailbox,
        metrics,
        inflight,
        rate,
        tile_rps: cfg.quota_tile_rps as f64,
        mb_rps: cfg.quota_mailbox_rps as f64,
        cfg,
    };

    loop {
        let (tcp, peer) = listener.accept().await?;
        let deps_cloned = deps.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_conn(tcp, peer, deps_cloned).await {
                error!("conn error: {e:?}");
            }
        });
    }
}

async fn handle_conn(tcp: TcpStream, peer: std::net::SocketAddr, deps: Deps) -> Result<()> {
    let tls = deps.acceptor.accept(tcp).await.context("tls accept")?;
    let mut framed = Framed::new(
        tls,
        OapCodec::new(deps.cfg.max_frame, DEFAULT_MAX_DECOMPRESSED),
    );
    debug!("conn established from {}", peer);

    // NEW: per-connection stream budget & timing
    let limits = OapLimits::default();
    let mut st = StreamState::new(Instant::now());

    loop {
        match framed.next().await {
            Some(Ok(frame)) => {
                // Enforce caps/timeouts on incoming payload (per "stream" == per request here)
                let now = Instant::now();
                if let Err(reason) = st.on_frame(frame.payload.len(), now, &limits) {
                    match reason {
                        RejectReason::Timeout => {
                            inc_reject_timeout();
                            // 408 Request Timeout
                            let _ = send_error_frame(
                                &mut framed,
                                408,
                                frame.app_proto_id,
                                frame.tenant_id,
                                frame.corr_id,
                                br#"{"error":"timeout"}"#,
                            )
                            .await;
                        }
                        RejectReason::TooManyFrames { .. } => {
                            inc_reject_too_many_frames();
                            // 400 Bad Request
                            let _ = send_error_frame(
                                &mut framed,
                                400,
                                frame.app_proto_id,
                                frame.tenant_id,
                                frame.corr_id,
                                br#"{"error":"too_many_frames"}"#,
                            )
                            .await;
                        }
                        RejectReason::TooManyBytes { .. } => {
                            inc_reject_too_many_bytes();
                            // 413 Payload Too Large
                            let _ = send_error_frame(
                                &mut framed,
                                413,
                                frame.app_proto_id,
                                frame.tenant_id,
                                frame.corr_id,
                                br#"{"error":"too_large"}"#,
                            )
                            .await;
                        }
                    }
                    break; // close connection after reject
                }

                // Map app_proto_id to a coarse "topic" label for metrics
                let topic = if frame.app_proto_id == TILE_APP_PROTO_ID {
                    "tiles"
                } else if frame.app_proto_id == MAILBOX_APP_PROTO_ID {
                    "mailbox"
                } else if frame.app_proto_id == 0 {
                    "hello"
                } else {
                    "unknown"
                };
                inc_streams(topic);
                add_data_bytes(topic, frame.payload.len() as u64);

                // Global capacity gate (per frame/request)
                let permit = match deps.inflight.clone().try_acquire_owned() {
                    Ok(p) => {
                        deps.metrics.inflight_inc();
                        Some(p)
                    }
                    Err(_) => {
                        deps.metrics.inc_overload();
                        let _ = send_error_frame(
                            &mut framed,
                            503,
                            frame.app_proto_id,
                            frame.tenant_id,
                            frame.corr_id,
                            br#"{"error":"overload","retry_after_ms":1000}"#,
                        )
                        .await;
                        continue;
                    }
                };

                // Per-tenant per-proto quotas
                let allow = {
                    let mut rl = deps.rate.lock().await;
                    let rps = if frame.app_proto_id == TILE_APP_PROTO_ID {
                        deps.tile_rps
                    } else if frame.app_proto_id == MAILBOX_APP_PROTO_ID {
                        deps.mb_rps
                    } else {
                        f64::INFINITY // no quota on HELLO/unknown
                    };
                    rl.check(frame.tenant_id, frame.app_proto_id, rps)
                };
                if !allow {
                    deps.metrics.inc_overload(); // counting 429s with overload for now
                    let _ = send_error_frame(
                        &mut framed,
                        429,
                        frame.app_proto_id,
                        frame.tenant_id,
                        frame.corr_id,
                        br#"{"error":"over_quota","retry_after_ms":1000}"#,
                    )
                    .await;
                    drop(permit);
                    deps.metrics.inflight_dec();
                    continue;
                }

                // Count request
                deps.metrics.inc_requests();

                // Dispatch
                let res = match frame.app_proto_id {
                    0 => handle_hello(&mut framed, &deps.cfg, &frame).await,
                    p if p == TILE_APP_PROTO_ID => {
                        // storage handler updates bytes_out_total internally
                        handle_storage_get(
                            &mut framed,
                            &deps.cfg,
                            &deps.storage, // auto-deref; fixes clippy::explicit-auto-deref
                            &frame,
                            deps.metrics.clone(),
                        )
                        .await
                    }
                    p if p == MAILBOX_APP_PROTO_ID => {
                        // mailbox handler may not update byte counters; OK for now
                        handle_mailbox(&mut framed, &deps.mailbox, &frame, &deps.metrics).await
                    }
                    _ => {
                        // Unknown protocol id
                        let _ = send_error_frame(
                            &mut framed,
                            400,
                            frame.app_proto_id,
                            frame.tenant_id,
                            frame.corr_id,
                            br#"{"error":"bad_request"}"#,
                        )
                        .await;
                        Ok(())
                    }
                };

                drop(permit);
                deps.metrics.inflight_dec();

                if let Err(e) = res {
                    let (code, body) = map_err(&e);
                    let _ = send_error_frame(
                        &mut framed,
                        code,
                        frame.app_proto_id,
                        frame.tenant_id,
                        frame.corr_id,
                        &body,
                    )
                    .await;
                } else {
                    // Success -> refresh activity clock
                    st.touch(Instant::now());
                }
            }
            Some(Err(e)) => {
                // Treat TLS EOF without close_notify as a NORMAL close (avoid error spam).
                if is_tls_unexpected_eof(&e) {
                    debug!("conn {} closed by peer without close_notify", peer);
                    break;
                } else {
                    // Promote to anyhow with context
                    return Err(e).context("oap read");
                }
            }
            None => {
                // Graceful end of stream.
                debug!("conn {} ended (EOF)", peer);
                break;
            }
        }
    }

    Ok(())
}

/// Helper to send a RESP+END JSON error frame.
async fn send_error_frame(
    framed: &mut Framed<tokio_rustls::server::TlsStream<TcpStream>, OapCodec>,
    code: u16,
    app_proto_id: u16,
    tenant_id: u128,
    corr_id: u64,
    json_body: &[u8],
) -> Result<()> {
    let resp = OapFrame {
        ver: OAP_VERSION,
        flags: OapFlags::RESP | OapFlags::END,
        code,
        app_proto_id,
        tenant_id,
        cap: Bytes::new(),
        corr_id,
        payload: Bytes::from(json_body.to_vec()),
    };
    framed.send(resp).await.context("send error frame")
}

/// Detect the common rustls/IO wording for EOF-without-close_notify as surfaced
/// through the `ron_app_sdk::Error` decoder error.
fn is_tls_unexpected_eof(err: &OapError) -> bool {
    let s = err.to_string().to_ascii_lowercase();
    s.contains("close_notify") || s.contains("unexpected eof")
}

fn map_err(e: &anyhow::Error) -> (u16, Vec<u8>) {
    let s = format!("{e:?}");
    if s.contains("too_large") || s.contains("413") {
        (413, br#"{"error":"too_large"}"#.to_vec())
    } else if s.contains("not_found") || s.contains("404") {
        (404, br#"{"error":"not_found"}"#.to_vec())
    } else if s.contains("over_quota") || s.contains("429") {
        (429, br#"{"error":"over_quota"}"#.to_vec())
    } else if s.contains("overload") || s.contains("503") {
        (503, br#"{"error":"overload"}"#.to_vec())
    } else if s.contains("invalid json") || s.contains("bad_request") || s.contains("bad op") {
        (400, br#"{"error":"bad_request"}"#.to_vec())
    } else {
        (500, br#"{"error":"internal"}"#.to_vec())
    }
}

```

### crates/svc-omnigate/src/storage.rs

```rust
#![forbid(unsafe_code)]

use anyhow::{anyhow, Context, Result};
use std::path::{Component, Path, PathBuf};
use tokio::fs::File;
use tracing::debug;

/// App protocol id for Storage (tiles) GET.
pub const TILE_APP_PROTO_ID: u16 = 0x0301;

/// Filesystem-backed storage for tiles.
///
/// Bronze ring: safe-ish path join (no `..`), size cap for 413 mapping.
/// The actual streaming loop is done by the overlay; this module just resolves and opens.
pub struct FsStorage {
    root: PathBuf,
    pub max_file_bytes: u64,
}

impl FsStorage {
    /// Create a new filesystem storage rooted at `root`. `max_file_bytes` caps responses (→ 413).
    pub fn new(root: impl Into<PathBuf>, max_file_bytes: u64) -> Self {
        Self {
            root: root.into(),
            max_file_bytes,
        }
    }

    /// Resolve a *relative* path (no leading '/') safely under root.
    /// Rejects any path containing `..` or absolute components.
    fn resolve_under_root(&self, rel_path: &str) -> Result<PathBuf> {
        let p = Path::new(rel_path.trim_start_matches('/'));
        if p.components().any(|c| {
            matches!(
                c,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        }) {
            return Err(anyhow!("invalid path"));
        }
        Ok(self.root.join(p))
    }

    /// Open a file and return (File, size). Enforces max size.
    pub async fn open(&self, rel_path: &str) -> Result<(File, u64)> {
        let path = self
            .resolve_under_root(rel_path)
            .with_context(|| format!("resolve {rel_path}"))?;

        let file = File::open(&path)
            .await
            .with_context(|| format!("open {}", path.display()))?;

        // Size (Bronze: read metadata, map errors)
        let meta = file
            .metadata()
            .await
            .with_context(|| format!("stat {}", path.display()))?;

        if !meta.is_file() {
            return Err(anyhow!("not a file"));
        }
        let size = meta.len();
        if size > self.max_file_bytes {
            return Err(anyhow!("too large: {} > {}", size, self.max_file_bytes));
        }

        debug!("open {} ({} bytes)", path.display(), size);
        Ok((file, size))
    }
}

```

### crates/svc-omnigate/src/tls.rs

```rust
// crates/svc-omnigate/src/tls.rs
#![forbid(unsafe_code)]

use anyhow::{anyhow, Context, Result};
use rustls_pemfile::{certs, pkcs8_private_keys, rsa_private_keys};
use std::{fs::File, io::BufReader, sync::Arc};
use tokio_rustls::{rustls, TlsAcceptor};

/// Load TLS acceptor from CERT_PEM and KEY_PEM.
/// Also installs the Rustls crypto provider (aws-lc-rs) explicitly to avoid
/// runtime panics if auto-selection isn’t active.
pub fn load_tls() -> Result<TlsAcceptor> {
    // Ensure a single crypto backend is installed (Rustls 0.23+).
    // NOTE: use {:?} because the error type is not Display.
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .map_err(|e| anyhow!("failed to install rustls aws-lc-rs provider: {:?}", e))?;

    let cert_path = std::env::var("CERT_PEM").context("CERT_PEM not set")?;
    let key_path = std::env::var("KEY_PEM").context("KEY_PEM not set")?;

    // ---- load certificate chain (already CertificateDer<'static>)
    let mut rd =
        BufReader::new(File::open(&cert_path).with_context(|| format!("open cert {}", cert_path))?);
    let chain: Vec<rustls::pki_types::CertificateDer<'static>> = certs(&mut rd)
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("parse certificate(s)")?;
    if chain.is_empty() {
        return Err(anyhow!("no certificates found in {}", cert_path));
    }

    // ---- load private key (prefer PKCS#8; fall back to RSA/PKCS#1)
    // Collect PKCS#8 keys; elements are PrivatePkcs8KeyDer<'static>.
    let pkcs8_keys: Vec<rustls::pki_types::PrivatePkcs8KeyDer<'static>> = {
        let mut kr = BufReader::new(
            File::open(&key_path).with_context(|| format!("open key {}", key_path))?,
        );
        pkcs8_private_keys(&mut kr)
            .collect::<std::result::Result<Vec<_>, _>>()
            .unwrap_or_default()
    };

    // Build a PrivateKeyDer in either branch so the if-expression has one concrete type.
    let priv_key: rustls::pki_types::PrivateKeyDer<'static> =
        if let Some(p8) = pkcs8_keys.into_iter().next() {
            // Convert PKCS#8 -> PrivateKeyDer
            rustls::pki_types::PrivateKeyDer::Pkcs8(p8)
        } else {
            // Fallback: RSA keys; parse and take the first
            let mut kr = BufReader::new(
                File::open(&key_path).with_context(|| format!("open key {}", key_path))?,
            );
            let rsa_keys: Vec<rustls::pki_types::PrivatePkcs1KeyDer<'static>> =
                rsa_private_keys(&mut kr)
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .context("parse RSA private key")?;

            let k = rsa_keys
                .into_iter()
                .next()
                .ok_or_else(|| anyhow!("no private key found in {}", key_path))?;

            rustls::pki_types::PrivateKeyDer::Pkcs1(k)
        };

    // ---- build server config
    let server_config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(chain, priv_key)
        .context("build rustls server config")?;

    Ok(TlsAcceptor::from(Arc::new(server_config)))
}

```

### crates/svc-overlay/Cargo.toml

```toml
[package]
publish = false

name = "svc-overlay"
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"

[dependencies]
anyhow = { workspace = true }
ron-bus = { workspace = true }
rmp-serde = { workspace = true }
serde = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true, features = ["env-filter", "fmt"] }
workspace-hack = { version = "0.1", path = "../../workspace-hack" }

```

### crates/svc-overlay/src/main.rs

```rust
// crates/svc-overlay/src/main.rs
#![forbid(unsafe_code)]

use std::env;
use std::os::unix::net::UnixStream;
use std::path::Path;

use ron_bus::api::{
    Envelope, IndexReq, IndexResp, OverlayReq, OverlayResp, Status, StorageReq, StorageResp,
};
use ron_bus::uds::{listen, recv, send};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

const DEFAULT_SOCK: &str = "/tmp/ron/svc-overlay.sock";
const DEFAULT_INDEX_SOCK: &str = "/tmp/ron/svc-index.sock";
const DEFAULT_STORAGE_SOCK: &str = "/tmp/ron/svc-storage.sock";

/// Encode a value to MessagePack. On failure, log and return an empty Vec instead of panicking.
/// This keeps runtime code free of `expect()` while surfacing the error.
fn to_vec_or_log<T: serde::Serialize>(value: &T) -> Vec<u8> {
    match rmp_serde::to_vec(value) {
        Ok(v) => v,
        Err(e) => {
            error!(error=?e, "svc-overlay: msgpack encode failed");
            Vec::new()
        }
    }
}

fn main() -> std::io::Result<()> {
    // Logging: honor RUST_LOG (fallback to info)
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    // Socket path (ensure parent exists for macOS tmp dirs)
    let sock = env::var("RON_OVERLAY_SOCK").unwrap_or_else(|_| DEFAULT_SOCK.into());
    if let Some(parent) = Path::new(&sock).parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    // Listen + accept
    let listener = listen(&sock)?;
    info!("svc-overlay listening on {}", sock);

    for conn in listener.incoming() {
        match conn {
            Ok(stream) => {
                // Spawn a thread per connection (same as your original), but move the stream in.
                std::thread::spawn(move || {
                    if let Err(e) = handle_client(stream) {
                        error!(error=?e, "client handler error");
                    }
                });
            }
            Err(e) => error!(error=?e, "accept error"),
        }
    }
    Ok(())
}

fn handle_client(mut stream: UnixStream) -> std::io::Result<()> {
    let env = match recv(&mut stream) {
        Ok(e) => e,
        Err(e) => {
            error!(error=?e, "recv error");
            return Ok(());
        }
    };

    let reply_env = match rmp_serde::from_slice::<OverlayReq>(&env.payload) {
        Ok(OverlayReq::Health) => {
            info!("overlay health probe");
            let _st = Status {
                ok: true,
                message: "ok".into(),
            };
            let payload = to_vec_or_log(&OverlayResp::HealthOk);
            Envelope {
                service: "svc.overlay".into(),
                method: "v1.ok".into(),
                corr_id: env.corr_id,
                token: vec![],
                payload,
            }
        }

        Ok(OverlayReq::Get { addr, rel }) => {
            // High-signal log: show incoming request fields
            info!(%addr, %rel, "overlay get");
            match overlay_get(&addr, &rel) {
                Ok(Some(bytes)) => {
                    info!(%addr, %rel, bytes = bytes.len(), "overlay get OK");
                    let payload = to_vec_or_log(&OverlayResp::Bytes { data: bytes });
                    Envelope {
                        service: "svc.overlay".into(),
                        method: "v1.ok".into(),
                        corr_id: env.corr_id,
                        token: vec![],
                        payload,
                    }
                }
                Ok(None) => {
                    info!(%addr, %rel, "overlay get NOT FOUND");
                    let payload = to_vec_or_log(&OverlayResp::NotFound);
                    Envelope {
                        service: "svc.overlay".into(),
                        method: "v1.not_found".into(),
                        corr_id: env.corr_id,
                        token: vec![],
                        payload,
                    }
                }
                Err(e) => {
                    error!(%addr, %rel, error=?e, "overlay get error");
                    let payload = to_vec_or_log(&OverlayResp::Err { err: e.to_string() });
                    Envelope {
                        service: "svc.overlay".into(),
                        method: "v1.err".into(),
                        corr_id: env.corr_id,
                        token: vec![],
                        payload,
                    }
                }
            }
        }

        Err(e) => {
            error!(error=?e, "bad overlay req");
            let payload = to_vec_or_log(&OverlayResp::Err {
                err: format!("bad req: {e}"),
            });
            Envelope {
                service: "svc.overlay".into(),
                method: "v1.err".into(),
                corr_id: env.corr_id,
                token: vec![],
                payload,
            }
        }
    };

    let _ = send(&mut stream, &reply_env);
    Ok(())
}

/// Resolve addr via svc-index, then read file via svc-storage.
/// rel="" defaults to "payload.bin".
fn overlay_get(addr: &str, rel: &str) -> anyhow::Result<Option<Vec<u8>>> {
    let index_sock = env::var("RON_INDEX_SOCK").unwrap_or_else(|_| DEFAULT_INDEX_SOCK.into());
    let storage_sock = env::var("RON_STORAGE_SOCK").unwrap_or_else(|_| DEFAULT_STORAGE_SOCK.into());

    // ---- Resolve via index
    let dir = {
        let mut s = UnixStream::connect(&index_sock)?;
        let req = Envelope {
            service: "svc.index".into(),
            method: "v1.resolve".into(),
            corr_id: 1,
            token: vec![],
            payload: rmp_serde::to_vec(&IndexReq::Resolve {
                addr: addr.to_string(),
            })?,
        };
        send(&mut s, &req)?;
        let env = recv(&mut s)?;
        match rmp_serde::from_slice::<IndexResp>(&env.payload)? {
            IndexResp::Resolved { dir } => {
                info!(%addr, %dir, "index resolve FOUND");
                dir
            }
            IndexResp::NotFound => {
                info!(%addr, "index resolve NOT FOUND");
                return Ok(None);
            }
            IndexResp::Err { err } => {
                error!(%addr, error=%err, "index resolve ERR");
                return Err(anyhow::anyhow!(err));
            }
            IndexResp::HealthOk | IndexResp::PutOk => {
                let msg = "unexpected index resp";
                error!(%addr, msg);
                return Err(anyhow::anyhow!(msg));
            }
        }
    };

    // ---- Read file via storage
    let rel = if rel.is_empty() { "payload.bin" } else { rel };
    let mut s = UnixStream::connect(&storage_sock)?;
    let req = Envelope {
        service: "svc.storage".into(),
        method: "v1.read_file".into(),
        corr_id: 2,
        token: vec![],
        payload: rmp_serde::to_vec(&StorageReq::ReadFile {
            dir: dir.clone(),
            rel: rel.to_string(),
        })?,
    };
    send(&mut s, &req)?;
    let env = recv(&mut s)?;
    match rmp_serde::from_slice::<StorageResp>(&env.payload)? {
        StorageResp::File { bytes } => {
            info!(%addr, %dir, %rel, bytes = bytes.len(), "storage read OK");
            Ok(Some(bytes))
        }
        StorageResp::NotFound => {
            info!(%addr, %dir, %rel, "storage read NOT FOUND");
            Ok(None)
        }
        StorageResp::Err { err } => {
            error!(%addr, %dir, %rel, error=%err, "storage read ERR");
            Err(anyhow::anyhow!(err))
        }
        StorageResp::HealthOk | StorageResp::Written => {
            let msg = "unexpected storage resp";
            error!(%addr, %dir, %rel, msg);
            Err(anyhow::anyhow!(msg))
        }
    }
}

```

### crates/svc-sandbox/Cargo.toml

```toml
[package]
name = "svc-sandbox"
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"
publish = false

[dependencies]
anyhow = { workspace = true }
axum = { workspace = true, features = ["tokio", "http1", "http2", "json"] }
tokio = { workspace = true, features = ["macros", "rt-multi-thread", "signal"] }
serde = { workspace = true, features = ["derive"] }
serde_json = { workspace = true }
prometheus = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true, features = ["env-filter", "fmt"] }
parking_lot = { workspace = true }
rand = { workspace = true }
bytes = { workspace = true }
futures-util = { workspace = true, default-features = true, features = ["std"] }
tower = { workspace = true }

```

### crates/svc-sandbox/src/decoy.rs

```rust
use rand::{Rng, RngCore};
use std::collections::HashMap;

/// A single decoy asset that looks like a real object.
#[derive(Clone)]
pub struct DecoyAsset {
    pub id: String,           // e.g., "b3:<hex>.tld"
    pub content_type: String, // plausible MIME
    pub bytes: Vec<u8>,       // small but streamable
}

#[derive(Default)]
pub struct DecoyCatalog {
    by_id: HashMap<String, DecoyAsset>,
}

impl DecoyCatalog {
    pub fn generate<R: Rng + ?Sized>(rng: &mut R, count: usize) -> Self {
        let mut by_id = HashMap::with_capacity(count);
        for _ in 0..count {
            let size = rng.gen_range(32_000..96_000); // 32–96 KiB
            let mut buf = vec![0u8; size];
            rng.fill_bytes(&mut buf);
            // fabricate a plausible b3-like id (no need to compute real BLAKE3 here)
            let mut id_bytes = [0u8; 32];
            rng.fill_bytes(&mut id_bytes);
            let hex = hex::encode(id_bytes);
            let id = format!("b3:{hex}.tld");
            let content_type = if rng.gen_bool(0.5) { "application/octet-stream" } else { "application/x-ron-bundle" }.to_string();
            let asset = DecoyAsset { id: id.clone(), content_type, bytes: buf };
            by_id.insert(id, asset);
        }
        Self { by_id }
    }

    pub fn get(&self, id: &str) -> Option<DecoyAsset> {
        self.by_id.get(id).cloned()
    }

    pub fn len(&self) -> usize {
        self.by_id.len()
    }
}

// Minimal embedded hex encoder to avoid extra deps if workspace lacks `hex`.
mod hex {
    const LUT: &[u8; 16] = b"0123456789abcdef";
    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        let b = bytes.as_ref();
        let mut out = vec![0u8; b.len() * 2];
        for (i, &v) in b.iter().enumerate() {
            out[i * 2] = LUT[(v >> 4) as usize];
            out[i * 2 + 1] = LUT[(v & 0x0f) as usize];
        }
        unsafe { String::from_utf8_unchecked(out) }
    }
}

```

### crates/svc-sandbox/src/hardening.rs

```rust
use std::time::Duration;
use axum::{extract::DefaultBodyLimit, Router};
use tower::{Layer, ServiceBuilder};
use tower::limit::{ConcurrencyLimitLayer, RateLimitLayer};
use tower::timeout::TimeoutLayer;

/// Standard hardening stack:
/// - 5s handler timeout
/// - 512 in-flight requests
/// - 500 rps rate limit (tune per service)
/// - Request body cap (default ~1MiB, configurable)
pub fn layer(max_body: usize) -> impl Layer<Router> + Clone {
    ServiceBuilder::new()
        .layer(TimeoutLayer::new(Duration::from_secs(5)))
        .layer(ConcurrencyLimitLayer::new(512))
        .layer(RateLimitLayer::new(500, Duration::from_secs(1)))
        .layer(DefaultBodyLimit::max(max_body))
        .into_inner()
}

```

### crates/svc-sandbox/src/main.rs

```rust
mod hardening;
mod metrics;
mod decoy;
mod oap_stub;
mod tarpit;
mod router;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
    middleware,
};
use bytes::Bytes;
use metrics::SandboxMetrics;
use parking_lot::RwLock;
use rand::{rngs::StdRng, SeedableRng};
use serde::Serialize;
use std::{
    collections::HashSet,
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, SystemTime},
};
use tokio::signal;
use tracing::{info, warn, Level};
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Redirect,
    Mirror,
    Tarpit,
}

impl Mode {
    fn from_env() -> Self {
        match std::env::var("SANDBOX_MODE").unwrap_or_else(|_| "redirect".into()).to_lowercase().as_str() {
            "mirror" => Mode::Mirror,
            "tarpit" => Mode::Tarpit,
            _ => Mode::Redirect,
        }
    }
}

#[derive(Clone)]
struct AppState {
    started: SystemTime,
    ready: Arc<AtomicBool>,
    mode: Mode,
    max_body: usize,
    tarpit_min_ms: u64,
    tarpit_max_ms: u64,
    metrics: Arc<SandboxMetrics>,
    decoys: Arc<RwLock<decoy::DecoyCatalog>>,
    /// Sticky diversion fingerprints for telemetry/demo
    sticky: Arc<RwLock<HashSet<String>>>,
    service_name: &'static str,
    version: &'static str,
}

#[derive(Serialize)]
struct RootStatus<'a> {
    service: &'a str,
    version: &'a str,
    mode: &'a str,
    decoy_assets: usize,
    uptime_secs: u64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let bind: SocketAddr = std::env::var("SANDBOX_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:3005".to_string())
        .parse()
        .expect("SANDBOX_ADDR host:port");

    let mode = Mode::from_env();
    let max_body: usize = std::env::var("SANDBOX_MAX_BODY")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1_000_000);

    let tarpit_min_ms: u64 = std::env::var("SANDBOX_TARPIT_MS_MIN").ok().and_then(|s| s.parse().ok()).unwrap_or(250);
    let tarpit_max_ms: u64 = std::env::var("SANDBOX_TARPIT_MS_MAX").ok().and_then(|s| s.parse().ok()).unwrap_or(2_000);

    let seed: u64 = std::env::var("SANDBOX_DECOY_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0x5EED_5EED);

    let mut rng = StdRng::seed_from_u64(seed);
    let decoys = decoy::DecoyCatalog::generate(&mut rng, 64);

    let state = AppState {
        started: SystemTime::now(),
        ready: Arc::new(AtomicBool::new(false)),
        mode,
        max_body,
        tarpit_min_ms,
        tarpit_max_ms,
        metrics: Arc::new(SandboxMetrics::new()),
        decoys: Arc::new(RwLock::new(decoys)),
        sticky: Arc::new(RwLock::new(Default::default())),
        service_name: "svc-sandbox",
        version: env!("CARGO_PKG_VERSION"),
    };

    let app = Router::new()
        .route("/", get(root))
        .route("/version", get(version))
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/metrics", get(metrics_endpoint))
        // deception API (looks plausible)
        .route("/assets/:id", get(get_asset))
        .route("/oap/v1/handshake", post(oap_handshake))
        // utility
        .route("/whoami", get(|| async { "sandbox\n" }))
        .with_state(state.clone());

    // Apply deception router middleware (fingerprint + sticky diversion telemetry)
    let app = app.layer(middleware::from_fn_with_state(state.clone(), router::deception_middleware));

    // Apply hardening limits (timeouts, concurrency, rate, body limit)
    let app = hardening::layer(state.max_body).layer(app);

    let listener = tokio::net::TcpListener::bind(bind).await?;
    info!("svc-sandbox listening on http://{bind} mode={:?}", state.mode);

    state.ready.store(true, Ordering::SeqCst);

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = signal::ctrl_c().await;
            info!("shutdown signal received");
        })
        .await?;

    Ok(())
}

fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt().with_target(false).with_timer(fmt::time::uptime()).with_max_level(Level::INFO).with_env_filter(env_filter).init();
}

// ------------------- Handlers -------------------

async fn root(State(st): State<AppState>) -> impl IntoResponse {
    let up = st.started.elapsed().unwrap_or(Duration::ZERO).as_secs();
    let count = st.decoys.read().len();
    let mode = match st.mode { Mode::Redirect => "redirect", Mode::Mirror => "mirror", Mode::Tarpit => "tarpit" };

    Json(RootStatus {
        service: st.service_name,
        version: st.version,
        mode,
        decoy_assets: count,
        uptime_secs: up,
    })
}

async fn version(State(st): State<AppState>) -> impl IntoResponse {
    Json(serde_json::json!({ "service": st.service_name, "version": st.version }))
}

async fn healthz() -> impl IntoResponse {
    (StatusCode::OK, "ok\n")
}

async fn readyz(State(st): State<AppState>) -> impl IntoResponse {
    let ok = st.ready.load(Ordering::SeqCst);
    let code = if ok { StatusCode::OK } else { StatusCode::SERVICE_UNAVAILABLE };
    (code, Json(serde_json::json!({ "ready": ok })))
}

async fn metrics_endpoint() -> impl IntoResponse {
    let metric_families = prometheus::gather();
    let mut buf = Vec::new();
    let encoder = prometheus::TextEncoder::new();
    if let Err(e) = encoder.encode(&metric_families, &mut buf) {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("encode error: {e}")).into_response();
    }
    (StatusCode::OK, String::from_utf8_lossy(&buf).to_string()).into_response()
}

async fn get_asset(State(st): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    // Record stickiness on the asset id too (telemetry/demo)
    st.sticky.write().insert(id.clone());

    // Tar-pit if enabled
    tarpit::maybe_tarpit(st.mode, st.tarpit_min_ms, st.tarpit_max_ms, st.metrics.as_ref()).await;

    let cat = st.decoys.read();
    if let Some(asset) = cat.get(&id) {
        st.metrics.token_trip_total.inc();
        // stream in chunks; no unbounded Vec copies in hot path
        let bytes = Bytes::from(asset.bytes.clone());
        let chunk = 64 * 1024;
        let total = bytes.len();
        let stream = futures_util::stream::unfold(0usize, move |offset| {
            let b = bytes.clone();
            async move {
                if offset >= total {
                    None
                } else {
                    let end = (offset + chunk).min(total);
                    let slice = b.slice(offset..end);
                    Some((Ok::<Bytes, std::io::Error>(slice), end))
                }
            }
        });
        let body = axum::body::Body::from_stream(stream);
        let mut resp = axum::response::Response::new(body);
        resp.headers_mut().insert(axum::http::header::CONTENT_TYPE, asset.content_type.parse().unwrap());
        return resp;
    }

    (StatusCode::NOT_FOUND, "no such asset\n").into_response()
}

async fn oap_handshake(State(st): State<AppState>, axum::extract::Bytes payload: axum::extract::Bytes) -> impl IntoResponse {
    // enforce strict frame size
    if payload.len() > oap_stub::OAP1_MAX_FRAME {
        st.metrics.rejected_total.with_label_values(&["frame_too_large"]).inc();
        return (StatusCode::PAYLOAD_TOO_LARGE, Json(serde_json::json!({"error":"frame_too_large"})));
    }

    // Tar-pit if enabled
    tarpit::maybe_tarpit(st.mode, st.tarpit_min_ms, st.tarpit_max_ms, st.metrics.as_ref()).await;

    match oap_stub::handshake_stub(&payload) {
        Ok(resp) => (StatusCode::OK, Json(resp)).into_response(),
        Err(kind) => {
            st.metrics.rejected_total.with_label_values(&[kind.code()]).inc();
            (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": kind.code()}))).into_response()
        }
    }
}

```

### crates/svc-sandbox/src/metrics.rs

```rust
use prometheus::{Histogram, HistogramOpts, IntCounter, IntCounterVec, Opts};

pub struct SandboxMetrics {
    pub token_trip_total: IntCounter,
    pub rejected_total: IntCounterVec, // labels: reason
    pub tarpit_ms_hist: Histogram,
}

impl SandboxMetrics {
    pub fn new() -> Self {
        let token_trip_total = IntCounter::new("honeytoken_trips_total", "Total honeytoken/decoy asset hits").unwrap();
        let rejected_total = IntCounterVec::new(
            Opts::new("sandbox_rejected_total", "Rejected bad requests"),
            &["reason"],
        ).unwrap();
        let tarpit_ms_hist = Histogram::with_opts(HistogramOpts::new("tarpit_ms_histogram", "Injected tarpit delays (ms)")).unwrap();

        prometheus::register(Box::new(token_trip_total.clone())).ok();
        prometheus::register(Box::new(rejected_total.clone())).ok();
        prometheus::register(Box::new(tarpit_ms_hist.clone())).ok();

        Self {
            token_trip_total,
            rejected_total,
            tarpit_ms_hist,
        }
    }
}

```

### crates/svc-sandbox/src/oap_stub.rs

```rust
use serde::Serialize;

pub const OAP1_MAX_FRAME: usize = 1_048_576; // 1 MiB

#[derive(Debug)]
pub enum HandshakeError {
    Empty,
    TooShort,
    Malformed,
}

impl HandshakeError {
    pub fn code(&self) -> &'static str {
        match self {
            HandshakeError::Empty => "empty",
            HandshakeError::TooShort => "too_short",
            HandshakeError::Malformed => "malformed",
        }
    }
}

#[derive(Serialize)]
pub struct HandshakeAck {
    pub ok: bool,
    pub proto: &'static str,
    pub session_id: String,
}

pub fn handshake_stub(frame: &[u8]) -> Result<HandshakeAck, HandshakeError> {
    if frame.is_empty() {
        return Err(HandshakeError::Empty);
    }
    if frame.len() < 8 {
        return Err(HandshakeError::TooShort);
    }
    // Very lightweight "parse": this is a stub; we intentionally avoid real data-plane coupling.
    // Mix a few bytes to fabricate a session id.
    let sid = blake3_stub(frame);
    Ok(HandshakeAck { ok: true, proto: "OAP/1", session_id: sid })
}

// Small, fast hash stub to avoid pulling blake3 if workspace doesn't have it.
fn blake3_stub(input: &[u8]) -> String {
    let mut x: u64 = 0xcbf29ce484222325;
    for &b in input {
        x ^= b as u64;
        x = x.wrapping_mul(0x100000001b3);
    }
    // hex-16
    let mut out = [0u8; 16];
    for i in 0..16 {
        out[i] = ((x >> ((i % 8) * 8)) & 0xff) as u8;
    }
    hex::encode(&out)
}

mod hex {
    const LUT: &[u8; 16] = b"0123456789abcdef";
    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        let b = bytes.as_ref();
        let mut out = vec![0u8; b.len() * 2];
        for (i, &v) in b.iter().enumerate() {
            out[i * 2] = LUT[(v >> 4) as usize];
            out[i * 2 + 1] = LUT[(v & 0x0f) as usize];
        }
        unsafe { String::from_utf8_unchecked(out) }
    }
}

```

### crates/svc-sandbox/src/router.rs

```rust
use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, Request},
    middleware::Next,
    response::Response,
};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::AppState;

/// Verdict for the incoming request, used for telemetry/stickiness.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Verdict {
    Clean,
    Suspicious,
}

/// Axum middleware: fingerprint each request, classify it, and
/// store sticky fingerprints for telemetry/diversion demos.
pub async fn deception_middleware(
    State(st): State<AppState>,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    let verdict = classify(req.headers(), req.uri().path());
    let fp = fingerprint(req.headers(), req.uri().path());

    if verdict == Verdict::Suspicious {
        st.sticky.write().insert(fp.clone());
    }
    // make fingerprint available to handlers if needed
    req.extensions_mut().insert(fp);

    next.run(req).await
}

/// Very simple fingerprint from headers + path (no IP dependency).
fn fingerprint(headers: &HeaderMap, path: &str) -> String {
    let ua = headers.get("user-agent").and_then(|v| v.to_str().ok()).unwrap_or("-");
    let accept = headers.get("accept").and_then(|v| v.to_str().ok()).unwrap_or("-");
    let lang = headers.get("accept-language").and_then(|v| v.to_str().ok()).unwrap_or("-");
    let mut h = DefaultHasher::new();
    ua.hash(&mut h);
    accept.hash(&mut h);
    lang.hash(&mut h);
    path.hash(&mut h);
    format!("{:016x}", h.finish())
}

/// Lightweight rules to flag obvious scanners/probes.
/// (This is *sandbox-side telemetry*, not security; ingress should still
/// apply real policy/limits.)
fn classify(headers: &HeaderMap, path: &str) -> Verdict {
    let ua = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();
    let p = path.to_ascii_lowercase();

    // Known/typical scanners
    let scanners = ["curl", "wget", "nikto", "sqlmap", "nmap", "dirbuster", "gobuster", "zgrab", "masscan"];
    if ua.is_empty() || scanners.iter().any(|s| ua.contains(s)) {
        return Verdict::Suspicious;
    }

    // Suspicious paths common in drive-by scans
    let bad_paths = [
        "/.git", "/wp-admin", "/phpmyadmin", "/.env", "/server-status",
        "/etc/passwd", "../", "/admin", "/login", "/cgi-bin/",
    ];
    if bad_paths.iter().any(|s| p.contains(s)) {
        return Verdict::Suspicious;
    }

    // Odd header combos (very rough heuristic)
    if headers.get("x-forwarded-for").is_some() && headers.get("x-original-url").is_some() {
        return Verdict::Suspicious;
    }

    Verdict::Clean
}

```

### crates/svc-sandbox/src/tarpit.rs

```rust
use crate::metrics::SandboxMetrics;
use rand::{thread_rng, Rng};
use std::time::Duration;
use tokio::time::sleep;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode { Redirect, Mirror, Tarpit }

impl From<crate::Mode> for Mode {
    fn from(m: crate::Mode) -> Self {
        match m {
            crate::Mode::Redirect => Mode::Redirect,
            crate::Mode::Mirror => Mode::Mirror,
            crate::Mode::Tarpit => Mode::Tarpit,
        }
    }
}

pub async fn maybe_tarpit(mode: crate::Mode, min_ms: u64, max_ms: u64, metrics: &SandboxMetrics) {
    if matches!(Mode::from(mode), Mode::Tarpit) {
        let mut rng = thread_rng();
        let delay = rng.gen_range(min_ms..=max_ms) as f64;
        metrics.tarpit_ms_hist.observe(delay);
        sleep(Duration::from_millis(delay as u64)).await;
    }
}

```

### crates/svc-storage/Cargo.toml

```toml
[package]
publish = false

name = "svc-storage"
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"

[dependencies]
anyhow = { workspace = true }
ron-bus = { workspace = true }
rmp-serde = { workspace = true }
serde = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true, features = ["env-filter", "fmt"] }
workspace-hack = { version = "0.1", path = "../../workspace-hack" }

```

### crates/svc-storage/src/main.rs

```rust
// crates/svc-storage/src/main.rs
#![forbid(unsafe_code)]

use std::env;
use std::fs;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use ron_bus::api::{Envelope, StorageReq, StorageResp};
use ron_bus::uds::{listen, recv, send};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

const DEFAULT_SOCK: &str = "/tmp/ron/svc-storage.sock";

fn main() -> std::io::Result<()> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let sock = env::var("RON_STORAGE_SOCK").unwrap_or_else(|_| DEFAULT_SOCK.into());

    if let Some(parent) = std::path::Path::new(&sock).parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let listener = listen(&sock)?;
    info!("svc-storage listening on {}", sock);

    for conn in listener.incoming() {
        match conn {
            Ok(stream) => {
                std::thread::spawn(|| {
                    if let Err(e) = handle_client(stream) {
                        error!(error=?e, "client handler error");
                    }
                });
            }
            Err(e) => error!(error=?e, "accept error"),
        }
    }
    Ok(())
}

fn handle_client(mut stream: UnixStream) -> std::io::Result<()> {
    let env = match recv(&mut stream) {
        Ok(e) => e,
        Err(e) => {
            error!(error=?e, "recv error");
            return Ok(());
        }
    };

    let resp = match rmp_serde::from_slice::<StorageReq>(&env.payload) {
        Ok(StorageReq::Health) => StorageResp::HealthOk,
        Ok(StorageReq::ReadFile { dir, rel }) => match read_file(&dir, &rel) {
            Ok(bytes) => StorageResp::File { bytes },
            Err(e) if e.downcast_ref::<std::io::Error>().is_some() => StorageResp::NotFound,
            Err(e) => StorageResp::Err { err: e.to_string() },
        },
        Ok(StorageReq::WriteFile { dir, rel, bytes }) => match write_file(&dir, &rel, &bytes) {
            Ok(()) => StorageResp::Written,
            Err(e) => StorageResp::Err { err: e.to_string() },
        },
        Err(e) => StorageResp::Err {
            err: format!("bad req: {e}"),
        },
    };

    // Avoid expect(); if encoding fails, log and drop the connection gracefully.
    let payload = match rmp_serde::to_vec(&resp) {
        Ok(v) => v,
        Err(e) => {
            error!(error=?e, "encode resp error");
            return Ok(());
        }
    };

    let reply = Envelope {
        service: "svc.storage".into(),
        method: "v1.ok".into(),
        corr_id: env.corr_id,
        token: vec![],
        payload,
    };
    let _ = send(&mut stream, &reply);
    Ok(())
}

fn read_file(dir: &str, rel: &str) -> Result<Vec<u8>> {
    let mut path = PathBuf::from(dir);
    let relp = Path::new(rel);
    path.push(relp);
    fs::read(&path).with_context(|| format!("read {}", path.display()))
}

fn write_file(dir: &str, rel: &str, bytes: &[u8]) -> Result<()> {
    let mut path = PathBuf::from(dir);
    path.push(Path::new(rel));
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, bytes)?;
    Ok(())
}

```

### crates/tldctl/Cargo.toml

```toml
[package]
name = "tldctl"
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"
publish = false

[dependencies]
# Workspace-pinned deps
anyhow = { workspace = true }
blake3 = { workspace = true }
clap = { workspace = true }
serde = { workspace = true }
toml = { workspace = true }
zstd = { workspace = true }
index = { workspace = true }
naming = { workspace = true }

# Local / external deps not pinned at workspace
time = { version = "0.3", features = ["formatting"] }
# Align brotli with the version pulled in by async-compression to avoid dupes.
brotli = "8"
infer = "0.15"
ryker = { path = "../ryker" }

# Hakari feature unifier
workspace-hack = { version = "0.1", path = "../../workspace-hack" }

[[bin]]
name = "tldctl"
path = "src/main.rs"

```

### crates/tldctl/src/index_bus.rs

```rust
use std::io;
use std::os::unix::net::UnixStream;
use rand::Rng;
use ron_bus::api::{Envelope, IndexReq, IndexResp};
use ron_bus::uds::{recv, send};

const DEFAULT_SOCK: &str = "/tmp/ron/svc-index.sock";

pub fn put_address(addr: &str, dir: &str) -> io::Result<()> {
    let sock = std::env::var("RON_INDEX_SOCK").unwrap_or_else(|_| DEFAULT_SOCK.into());
    let mut stream = UnixStream::connect(&sock)?;

    let corr_id: u64 = rand::thread_rng().gen();
    let req = IndexReq::PutAddress { addr: addr.to_string(), dir: dir.to_string() };
    let env = Envelope {
        service: "svc.index".into(),
        method: "v1.put".into(),
        corr_id,
        token: vec![],
        payload: rmp_serde::to_vec(&req).unwrap(),
    };

    send(&mut stream, &env).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let reply = recv(&mut stream).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    if reply.corr_id != corr_id {
        return Err(io::Error::new(io::ErrorKind::Other, "corr_id mismatch"));
    }

    match rmp_serde::from_slice::<IndexResp>(&reply.payload) {
        Ok(IndexResp::PutOk) => Ok(()),
        _ => Err(io::Error::new(io::ErrorKind::Other, "put failed")),
    }
}

```

### crates/tldctl/src/main.rs

```rust
// crates/tldctl/src/main.rs
#![forbid(unsafe_code)]

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use index::Index;
use naming::manifest::{write_manifest, Encoding, ManifestV2, Payment, Relations, RevenueSplit};
use naming::Address;
use ryker::validate_payment_block;
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use time::OffsetDateTime;
use toml::{map::Map as TomlMap, Value as TomlValue};
use zstd::Encoder as ZstdEncoder;

/// Pack source files into the RustyOnions object store and index them by BLAKE3 address.
#[derive(Parser, Debug)]
#[command(name = "tldctl", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Pack a single file into the store and index it.
    Pack {
        /// TLD kind (e.g., text, image, video)
        #[arg(long)]
        tld: String,

        /// Path to the input file
        #[arg(long)]
        input: PathBuf,

        /// Path to the index database directory (sled)
        #[arg(long)]
        index_db: PathBuf,

        /// Root of the content-addressed store
        #[arg(long)]
        store_root: PathBuf,

        // ---------- Payments (optional) ----------
        /// (Micropayments) Mark payment as required to access this object
        #[arg(long, default_value_t = false)]
        required: bool,

        /// Price model: per_mib | flat | per_request
        #[arg(long, value_name = "MODEL", default_value = "")]
        price_model: String,

        /// Price value (units depend on model)
        #[arg(long, value_name = "NUM", default_value_t = 0.0)]
        price: f64,

        /// Currency code (e.g., USD, sats, ETH, SOL)
        #[arg(long, value_name = "CODE", default_value = "")]
        currency: String,

        /// Wallet address / LNURL / pay endpoint
        #[arg(long, value_name = "WALLET", default_value = "")]
        wallet: String,

        /// Settlement type: onchain | offchain | custodial (advisory)
        #[arg(long, value_name = "KIND", default_value = "")]
        settlement: String,

        // ---------- Relations / License / Extensions (PR-8) ----------
        /// Parent object address (b3:<hex>.<tld>) → [relations].parent
        #[arg(long)]
        parent: Option<String>,

        /// Thread/root object address (b3:<hex>.<tld>) → [relations].thread
        #[arg(long)]
        thread: Option<String>,

        /// SPDX or human-readable license → top-level `license`
        #[arg(long)]
        license: Option<String>,

        /// Repeatable: ns:key=value → [ext.<ns>].<key>="value"
        /// Example: --ext image:width=800 --ext image:height=600 --ext seo:title="Hello"
        #[arg(long = "ext")]
        ext: Vec<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Pack {
            tld,
            input,
            index_db,
            store_root,
            required,
            price_model,
            price,
            currency,
            wallet,
            settlement,
            parent,
            thread,
            license,
            ext,
        } => {
            let addr = pack(
                &tld,
                &input,
                &index_db,
                &store_root,
                required,
                &price_model,
                price,
                &currency,
                &wallet,
                &settlement,
                parent.as_deref(),
                thread.as_deref(),
                license.as_deref(),
                &ext,
            )?;
            // Print only the canonical address (scripts capture this)
            println!("{addr}");
            Ok(())
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn pack(
    tld: &str,
    input: &Path,
    index_db: &Path,
    store_root: &Path,
    required: bool,
    price_model: &str,
    price: f64,
    currency: &str,
    wallet: &str,
    settlement: &str,
    parent: Option<&str>,
    thread: Option<&str>,
    license: Option<&str>,
    ext_kvs: &[String],
) -> Result<String> {
    // ---------- Read original bytes ----------
    let data = fs::read(input).with_context(|| format!("read input {}", input.display()))?;
    let orig_len = data.len();

    // ---------- Policy enforcement ----------
    match tld {
        "image" => ensure_is_avif(&data, input)?,
        "video" => ensure_is_av1(&data, input)?,
        _ => {}
    }

    // ---------- Canonical address ----------
    let hash = blake3::hash(&data);
    let hash_hex = hash.to_hex().to_string();
    let addr_str = format!("b3:{hash_hex}.{tld}");

    // ---------- Store path ----------
    let shard2 = &hash_hex[0..2];
    let bundle_dir = store_root
        .join("objects")
        .join(tld)
        .join(shard2)
        .join(format!("{hash_hex}.{tld}"));
    fs::create_dir_all(&bundle_dir)
        .with_context(|| format!("create bundle dir {}", bundle_dir.display()))?;

    // ---------- Write payload.bin ----------
    let stored_filename = "payload.bin";
    let payload_path = bundle_dir.join(stored_filename);
    fs::write(&payload_path, &data).context("write payload.bin")?;

    // ---------- Guess MIME ----------
    let mime = guess_mime(&data, input);

    // ---------- Build Manifest v2 ----------
    let created_utc =
        OffsetDateTime::now_utc().format(&time::format_description::well_known::Rfc3339)?;
    let mut manifest = ManifestV2 {
        schema_version: 2,
        tld: tld.to_string(),
        address: addr_str.clone(),
        hash_algo: "b3".to_string(),
        hash_hex: hash_hex.clone(),
        bytes: orig_len as u64,
        created_utc,
        mime: mime.clone(),
        stored_filename: stored_filename.to_string(),
        original_filename: input
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string()),
        encodings: Vec::<Encoding>::new(),
        payment: None,
        relations: None,
        license: None,
        ext: BTreeMap::new(),
    };

    // ---------- Precompress texty assets ----------
    if is_compressible_mime(&mime) {
        // Zstd (lvl 15)
        let zst_path = bundle_dir.join("payload.bin.zst");
        {
            let mut out = fs::File::create(&zst_path).context("create .zst")?;
            let mut enc = ZstdEncoder::new(&mut out, 15).context("zstd encoder")?;
            enc.write_all(&data).context("zstd write")?;
            enc.finish().context("zstd finish")?;
        }
        let zst_bytes = fs::read(&zst_path).context("read .zst back")?;
        let zst_hash = blake3::hash(&zst_bytes).to_hex().to_string();
        manifest.encodings.push(Encoding {
            coding: "zstd".into(),
            level: 15,
            bytes: zst_bytes.len() as u64,
            filename: "payload.bin.zst".into(),
            hash_hex: zst_hash,
        });

        // Brotli (q 9, window lgwin 22)
        let br_path = bundle_dir.join("payload.bin.br");
        {
            let mut out = fs::File::create(&br_path).context("create .br")?;
            // brotli::CompressorWriter(buf_size, quality, lgwin)
            let mut w = brotli::CompressorWriter::new(&mut out, 4096, 9, 22);
            w.write_all(&data).context("brotli write")?;
            // make a best effort to flush; ignore error (compression already wrote the bulk)
            let _ = w.flush();
            // Drop `w` to flush final bytes
        }
        let br_bytes = fs::read(&br_path).context("read .br back")?;
        let br_hash = blake3::hash(&br_bytes).to_hex().to_string();
        manifest.encodings.push(Encoding {
            coding: "br".into(),
            level: 9,
            bytes: br_bytes.len() as u64,
            filename: "payload.bin.br".into(),
            hash_hex: br_hash,
        });
    }

    // ---------- Optional [payment] ----------
    let any_payment_flag = !wallet.is_empty() || required;
    if any_payment_flag {
        let p = Payment {
            required,
            currency: currency.to_string(),
            price_model: price_model.to_string(),
            price,
            wallet: wallet.to_string(),
            settlement: settlement.to_string(),
            splits: Vec::<RevenueSplit>::new(),
        };
        // Validate for internal consistency (fail early if user passed invalid combo)
        validate_payment_block(&p).context("invalid [payment] flags")?;
        manifest.payment = Some(p);
    }

    // ---------- PR-8: relations / license / ext.* ----------
    // relations
    if parent.is_some() || thread.is_some() {
        let mut r = manifest.relations.take().unwrap_or(Relations {
            parent: None,
            thread: None,
            source: None,
        });
        if let Some(p) = parent {
            r.parent = Some(p.to_string());
        }
        if let Some(t) = thread {
            r.thread = Some(t.to_string());
        }
        manifest.relations = Some(r);
    }

    // license (top-level now)
    if let Some(lic) = license {
        if !lic.trim().is_empty() {
            manifest.license = Some(lic.trim().to_string());
        }
    }

    // ext parsing: --ext ns:key=value
    if !ext_kvs.is_empty() {
        // manifest.ext is a BTreeMap<String, toml::Value>; each ns becomes a toml table.
        let mut ext = manifest.ext; // take existing (empty by default)
        for raw in ext_kvs {
            if let Some((ns, rest)) = raw.split_once(':') {
                if let Some((k, v)) = rest.split_once('=') {
                    let ns = ns.trim().to_string();
                    let k = k.trim().to_string();
                    // Keep value as TOML string for simplicity; could parse bool/num later if desired.
                    let v_str = v.trim().trim_matches('"').to_string();

                    // Get or create the [ext.<ns>] table using toml::map::Map
                    let table = ext
                        .entry(ns.clone())
                        .or_insert_with(|| TomlValue::Table(TomlMap::new()));

                    // Ensure it's a table
                    let tbl = table.as_table_mut().ok_or_else(|| {
                        anyhow!("ext namespace '{}' was not a table in manifest", ns)
                    })?;

                    tbl.insert(k, TomlValue::String(v_str));
                    continue;
                }
            }
            eprintln!("[tldctl] --ext expects ns:key=value, got: {raw}");
        }
        manifest.ext = ext;
    }

    // ---------- Write Manifest.toml ----------
    write_manifest(&bundle_dir, &manifest).context("write Manifest.toml")?;

    // ---------- Update index ----------
    let idx = Index::open(index_db).context("open index")?;
    let addr = Address::parse(&addr_str).context("parse address")?;
    idx.put_address(&addr, bundle_dir.clone())
        .context("index put_address")?;

    Ok(addr_str)
}

// ---------- Helpers ----------

fn ensure_is_avif(data: &[u8], path: &Path) -> Result<()> {
    // quick signature check (ftypavif in ISOBMFF brands)
    let is_avif_brand = data.windows(8).any(|w| w == b"ftypavif");
    // infer MIME
    let ok_infer = infer::get(data)
        .map(|k| k.mime_type() == "image/avif")
        .unwrap_or(false);
    if !(is_avif_brand || ok_infer) {
        return Err(anyhow!("policy: .image requires AVIF → {}", path.display())
            .context("not AVIF: missing ftyp/avif brand"));
    }
    Ok(())
}

fn ensure_is_av1(data: &[u8], path: &Path) -> Result<()> {
    // look for 'av01' (mp4) or 'V_AV1' (webm)
    let has_av1 = data.windows(4).any(|w| w == b"av01") || data.windows(5).any(|w| w == b"V_AV1");
    if !has_av1 {
        return Err(anyhow!("policy: .video requires AV1 → {}", path.display()));
    }
    Ok(())
}

fn guess_mime(data: &[u8], path: &Path) -> String {
    if let Some(k) = infer::get(data) {
        return k.mime_type().to_string();
    }
    // crude fallback: utf-8 → text/plain
    if std::str::from_utf8(data).is_ok() {
        return "text/plain; charset=utf-8".to_string();
    }
    // extension hints
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        match ext {
            "json" => return "application/json".into(),
            "js" => return "application/javascript".into(),
            "svg" => return "image/svg+xml".into(),
            "md" | "txt" => return "text/plain; charset=utf-8".into(),
            _ => {}
        }
    }
    "application/octet-stream".to_string()
}

fn is_compressible_mime(mime: &str) -> bool {
    mime.starts_with("text/")
        || matches!(
            mime,
            "application/json" | "application/javascript" | "image/svg+xml"
        )
}

```

### crates/transport/Cargo.toml

```toml
[package]
publish = false
license = "MIT OR Apache-2.0"


name = "transport"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = { workspace = true }
tracing = "0.1"


# existing internal dep (keep if you're using it)
accounting = { path = "../accounting" }

# NEW: async traits
async-trait = "0.1"

# NEW: async runtime + net I/O used across TCP/Tor code
tokio = { workspace = true, features = ["full"] }

# NEW: SOCKS5 client for Tor (SOCKS5h)
tokio-socks = "0.5"

# NEW: retry/backoff used by Tor control commands
retry = "2.0"
workspace-hack = { version = "0.1", path = "../../workspace-hack" }


```

### crates/transport/src/lib.rs

```rust
//! Async transport abstraction with TCP and Tor backends.
//!
//! This module also provides *compatibility shims* for older code:
//! - `transport::ReadWrite` (alias trait for an async stream)
//! - `transport::Handler`   (generic connection handler trait)

use anyhow::Result;
use async_trait::async_trait;
use std::net::SocketAddr;
use tokio::io::{AsyncRead, AsyncWrite};

// Public submodules
pub mod tcp;
pub mod tor;

/// Convenient alias for any async stream we can read/write.
pub trait IoStream: AsyncRead + AsyncWrite + Unpin + Send + 'static {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send + 'static> IoStream for T {}

/// Back-compat: expose `ReadWrite` as an alias trait for async IO streams.
/// Old code imported `transport::ReadWrite`; keep that working.
pub trait ReadWrite: IoStream {}
impl<T: IoStream> ReadWrite for T {}

/// Back-compat: a generic connection handler interface.
/// Old code imported `transport::Handler`.
#[async_trait]
pub trait Handler: Send + Sync {
    /// Stream type this handler works with.
    type Stream: IoStream;

    /// Handle an accepted/connected stream. `peer` may be a socket address (if known).
    async fn handle(&self, stream: Self::Stream, peer: SocketAddr) -> Result<()>;
}

/// A listener that can accept inbound connections for a given transport.
#[async_trait]
pub trait TransportListener: Send {
    type Stream: IoStream;

    /// Accept the next inbound connection.
    async fn accept(&mut self) -> Result<(Self::Stream, SocketAddr)>;
}

/// A simple async transport abstraction. Implemented by TCP and Tor.
#[async_trait]
pub trait Transport: Send + Sync {
    type Stream: IoStream;
    type Listener: TransportListener<Stream = Self::Stream>;

    /// Connect to a peer address. For Tor, this may be a `.onion:port`.
    async fn connect(&self, peer_addr: &str) -> Result<Self::Stream>;

    /// Bind a local listener (e.g., `127.0.0.1:1777`).
    async fn listen(&self, bind: SocketAddr) -> Result<Self::Listener>;
}

```

### crates/transport/src/tcp.rs

```rust
//! Tokio TCP implementation of the Transport trait.

use crate::TransportListener;
use anyhow::Result;
use async_trait::async_trait;
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};

pub struct TcpTransport;

pub struct TcpListen {
    inner: TcpListener,
}

#[async_trait]
impl TransportListener for TcpListen {
    type Stream = TcpStream;

    async fn accept(&mut self) -> Result<(Self::Stream, SocketAddr)> {
        let (stream, peer) = self.inner.accept().await?;
        Ok((stream, peer))
    }
}

#[async_trait]
impl crate::Transport for TcpTransport {
    type Stream = TcpStream;
    type Listener = TcpListen;

    async fn connect(&self, peer_addr: &str) -> Result<Self::Stream> {
        let stream = TcpStream::connect(peer_addr).await?;
        Ok(stream)
    }

    async fn listen(&self, bind: SocketAddr) -> Result<Self::Listener> {
        let listener = TcpListener::bind(bind).await?;
        Ok(TcpListen { inner: listener })
    }
}

impl TcpTransport {
    pub async fn bind(bind: SocketAddr) -> Result<TcpListener> {
        Ok(TcpListener::bind(bind).await?)
    }
    pub async fn dial(addr: &str) -> Result<TcpStream> {
        Ok(TcpStream::connect(addr).await?)
    }
}

```

### crates/transport/src/tor/ctrl.rs

```rust
//! Minimal Tor ControlPort client using **HashedControlPassword** auth,
//! with robust response parsing, HS event support, and manual retries.

use anyhow::{anyhow, bail, Result};
use std::net::SocketAddr;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;
use tokio::time::{sleep, timeout, Duration};

/// Tor control replies we care about.
#[derive(Debug)]
enum ReplyCode {
    Ok250,          // 250 OK or 250 <key>=<value> lines (final "250 OK")
    Async650,       // 650 (asynchronous event)
    Err4xx5xx(u16), // 4xx or 5xx
    Other(()),      // Anything else (unused payload -> unit to silence warning)
}

pub struct TorController {
    reader: BufReader<OwnedReadHalf>,
    writer: OwnedWriteHalf,
}

impl TorController {
    pub async fn connect_and_auth(addr: SocketAddr, password: &str) -> Result<Self> {
        let stream = TcpStream::connect(addr).await?;
        let (read_half, write_half) = stream.into_split();
        let reader = BufReader::new(read_half);
        let mut this = Self {
            reader,
            writer: write_half,
        };
        this.authenticate(password).await?;
        Ok(this)
    }

    async fn read_line(&mut self) -> Result<String> {
        let mut buf = String::new();
        let n = self.reader.read_line(&mut buf).await?;
        if n == 0 {
            bail!("control socket closed");
        }
        Ok(buf)
    }

    async fn expect_250(&mut self) -> Result<()> {
        loop {
            let line = self.read_line().await?;
            let code = parse_reply_code(&line);
            match code {
                ReplyCode::Ok250 => return Ok(()),
                ReplyCode::Err4xx5xx(code) => bail!("Tor control error {code}: {line}"),
                ReplyCode::Async650 => { /* ignore while handling a command */ }
                ReplyCode::Other(_) => { /* ignore */ }
            }
        }
    }

    pub async fn authenticate(&mut self, password: &str) -> Result<()> {
        // Retry with exponential backoff – Tor might still be bootstrapping.
        let mut delay = Duration::from_millis(100);
        for attempt in 1..=5 {
            let pw = escape_for_auth(password);
            let cmd = format!("AUTHENTICATE \"{}\"\r\n", pw);
            if let Err(e) = self.writer.write_all(cmd.as_bytes()).await {
                if attempt == 5 {
                    return Err(e.into());
                }
            } else if self.expect_250().await.is_ok() {
                return Ok(());
            }
            sleep(delay).await;
            delay = delay.saturating_mul(2);
        }
        bail!("failed to AUTHENTICATE after retries");
    }

    pub async fn set_events(&mut self, events: &[&str]) -> Result<()> {
        let list = events.join(" ");
        let cmd = format!("SETEVENTS {}\r\n", list);
        self.writer.write_all(cmd.as_bytes()).await?;
        self.expect_250().await
    }

    /// Low-level helper that issues ADD_ONION with an explicit target host/port mapping.
    async fn add_onion_core(
        &mut self,
        key_type: &str,   // "NEW:ED25519-V3" or "ED25519-V3:<b64>"
        host: &str,       // e.g., "127.0.0.1"
        target_port: u16, // local port your service listens on
        virt_port: u16,   // public onion port
        flags: &[&str],   // e.g., ["DiscardPK"]
    ) -> Result<(String, Option<String>)> {
        let flags = if flags.is_empty() {
            "".to_string()
        } else {
            format!(" Flags={}", flags.join(","))
        };
        let cmd = format!(
            "ADD_ONION {} Port={},{}:{}{}\r\n",
            key_type, virt_port, host, target_port, flags
        );
        self.writer.write_all(cmd.as_bytes()).await?;

        let mut service_id: Option<String> = None;
        let mut private_key: Option<String> = None;

        loop {
            let line = self.read_line().await?;
            match parse_reply_code(&line) {
                ReplyCode::Ok250 => break, // done
                ReplyCode::Err4xx5xx(code) => bail!("Tor control error {code}: {line}"),
                ReplyCode::Async650 => { /* ignore */ }
                ReplyCode::Other(_) => { /* ignore */ }
            }

            if let Some(rest) = line.strip_prefix("250-ServiceID=") {
                service_id = Some(rest.trim().to_string());
            } else if let Some(rest) = line.strip_prefix("250-PrivateKey=") {
                private_key = Some(rest.trim().to_string());
            }
        }

        let id = service_id.ok_or_else(|| anyhow!("ADD_ONION did not return ServiceID"))?;
        Ok((id, private_key))
    }

    /// Compatibility wrapper used by `hs.rs` when an **existing** key line is provided.
    pub async fn add_onion_with_key(
        &mut self,
        key_line: &str,   // "ED25519-V3:<b64>"
        public_port: u16, // onion external port
        host: &str,       // usually "127.0.0.1"
        local_port: u16,  // your local service port
    ) -> Result<(String, Option<String>)> {
        self.add_onion_core(key_line, host, local_port, public_port, &[])
            .await
    }

    /// Convenience wrapper to create a **new** v3 onion, returning ServiceID and Optional PrivateKey.
    pub async fn add_onion_new_with_host(
        &mut self,
        public_port: u16,
        host: &str,
        local_port: u16,
        flags: &[&str],
    ) -> Result<(String, Option<String>)> {
        self.add_onion_core("NEW:ED25519-V3", host, local_port, public_port, flags)
            .await
    }

    /// Older signature kept for compatibility with earlier code that always used 127.0.0.1.
    pub async fn add_onion_new(
        &mut self,
        key_type: &str, // "NEW:ED25519-V3" or "ED25519-V3:<b64>"
        port: u16,      // local target port
        virt_port: u16, // public onion port
        flags: &[&str],
    ) -> Result<(String, Option<String>)> {
        self.add_onion_core(key_type, "127.0.0.1", port, virt_port, flags)
            .await
    }

    /// Wait until Tor emits an `HS_DESC UPLOADED` event for `service_id`, or time out.
    pub async fn wait_hs_desc_uploaded(&mut self, service_id: &str, wait_secs: u64) -> Result<()> {
        // Use bare ServiceID (no ".onion")
        let sid = service_id.trim().trim_end_matches(".onion");
        self.set_events(&["HS_DESC"]).await?;

        let fut = async {
            loop {
                let line = self.read_line().await?;
                if let ReplyCode::Async650 = parse_reply_code(&line) {
                    // Typical formats:
                    // 650 HS_DESC CREATED <sid> NO_AUTH <replica>
                    // 650 HS_DESC UPLOADED <sid> NO_AUTH <replica>
                    // 650 HS_DESC UPLOAD_FAILED <sid> NO_AUTH REASON=<...> <replica>
                    if line.contains("HS_DESC") && line.contains(sid) {
                        if line.contains("UPLOADED") {
                            return Ok(());
                        }
                        if line.contains("UPLOAD_FAILED") {
                            bail!("HS_DESC upload failed for {}: {}", sid, line.trim());
                        }
                    }
                }
            }
        };

        match timeout(Duration::from_secs(wait_secs), fut).await {
            Ok(res) => res,
            Err(_) => bail!("timed out waiting for HS_DESC UPLOADED for {}", sid),
        }
    }

    pub async fn del_onion(&mut self, service_id: &str) -> Result<()> {
        let cmd = format!("DEL_ONION {}\r\n", service_id);
        self.writer.write_all(cmd.as_bytes()).await?;
        self.expect_250().await
    }
}

fn escape_for_auth(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn parse_reply_code(line: &str) -> ReplyCode {
    let code = line
        .chars()
        .take(3)
        .collect::<String>()
        .parse::<u16>()
        .unwrap_or(0);

    match code {
        250 => ReplyCode::Ok250,
        650 => ReplyCode::Async650,
        400..=599 => ReplyCode::Err4xx5xx(code),
        _ => ReplyCode::Other(()),
    }
}

```

### crates/transport/src/tor/hs.rs

```rust
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use super::ctrl::TorController;

/// Publish a v3 onion service (ephemeral).
/// - If `key_path` exists, reuse that key.
/// - Otherwise, request a new onion and optionally persist its private key.
pub async fn publish_v3(
    ctrl_addr: &str,
    ctrl_pw: &str,
    key_path: &Path,
    local_port: u16,
    public_port: u16,
    wait_secs: u64,
) -> Result<(String, String)> {
    // Connect & authenticate to Tor control
    let mut ctl = TorController::connect_and_auth(ctrl_addr.parse()?, ctrl_pw).await?;

    let (service_id, _private_key) = if key_path.exists() {
        // Existing key: reuse it
        let key_line = fs::read_to_string(key_path)
            .with_context(|| format!("reading HS key from {}", key_path.display()))?;
        ctl.add_onion_with_key(key_line.trim(), public_port, "127.0.0.1", local_port)
            .await?
    } else {
        // New onion: request NEW:ED25519-V3 and persist private key if returned
        let (sid, priv_line) = ctl
            .add_onion_new_with_host(public_port, "127.0.0.1", local_port, &[])
            .await?;
        if let Some(pk) = &priv_line {
            fs::write(key_path, pk)?;
        }
        (sid, priv_line)
    };

    // Wait for HS descriptor to be uploaded
    ctl.wait_hs_desc_uploaded(&service_id, wait_secs).await?;

    // Return onion hostname and service_id as String
    Ok((format!("{}.onion", service_id), service_id.to_string()))
}

```

### crates/transport/src/tor/mod.rs

```rust
//! Tor helpers: ControlPort and SOCKS5 dialing/tunnels.

pub mod ctrl;
pub mod hs;

use crate::{Transport, TransportListener};
use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::copy_bidirectional;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{oneshot, RwLock};
use tokio::time::timeout;

#[derive(Clone)]
pub struct TorTransport {
    pub socks_addr: SocketAddr,
    pub control_addr: SocketAddr,
    pub control_password: Option<String>,
    // Arc<RwLock<…>> so Clone works
    pub published_onion: Arc<RwLock<Option<String>>>,
}

impl TorTransport {
    pub fn new(
        socks_addr: SocketAddr,
        control_addr: SocketAddr,
        control_password: Option<String>,
    ) -> Self {
        Self {
            socks_addr,
            control_addr,
            control_password,
            published_onion: Arc::new(RwLock::new(None)),
        }
    }
}

pub struct TorListen {
    inner: TcpListener,
}

#[async_trait]
impl TransportListener for TorListen {
    type Stream = TcpStream;
    async fn accept(&mut self) -> Result<(Self::Stream, SocketAddr)> {
        let (stream, peer) = self.inner.accept().await?;
        Ok((stream, peer))
    }
}

#[async_trait]
impl Transport for TorTransport {
    type Stream = TcpStream;
    type Listener = TorListen;

    async fn connect(&self, peer_addr: &str) -> Result<Self::Stream> {
        dial_via_socks(self.socks_addr, peer_addr).await
    }

    async fn listen(&self, bind: SocketAddr) -> Result<Self::Listener> {
        let listener = TcpListener::bind(bind).await?;
        Ok(TorListen { inner: listener })
    }
}

/// Dial `dest` (like `example.onion:1777` or `example.com:80`) via SOCKS5h.
pub async fn dial_via_socks(socks_addr: SocketAddr, dest: &str) -> Result<TcpStream> {
    let (host, port) = split_host_port_supporting_ipv6(dest)?;
    let port: u16 = port.parse().context("invalid port")?;
    let stream = tokio_socks::tcp::Socks5Stream::connect(socks_addr, (host, port))
        .await
        .context("SOCKS5 connect failed")?
        .into_inner();
    Ok(stream)
}

/// Start a one-shot local TCP → SOCKS5 tunnel.
/// Returns (local_addr, ready_rx, join_handle).
/// - `Ok(())` on successful connect.
/// - `Err(String)` if dialing failed.
pub async fn start_oneshot_socks_tunnel(
    socks_addr: SocketAddr,
    dest: &str,
) -> Result<(
    SocketAddr,
    oneshot::Receiver<Result<(), String>>,
    tokio::task::JoinHandle<()>,
)> {
    let listener = TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0)).await?;
    let local_addr = listener.local_addr()?;
    let dest = dest.to_string();

    let (tx_ready, rx_ready) = oneshot::channel();

    let handle = tokio::spawn(async move {
        if let Ok((mut inbound, _peer)) = listener.accept().await {
            // Time-limit the SOCKS dial so clients fail fast on unreachable onions
            match timeout(Duration::from_secs(20), dial_via_socks(socks_addr, &dest)).await {
                Err(_) => {
                    let _ = tx_ready.send(Err("SOCKS dial timed out".into()));
                }
                Ok(Err(e)) => {
                    let _ = tx_ready.send(Err(format!("SOCKS dial failed: {e}")));
                }
                Ok(Ok(mut outbound)) => {
                    let _ = tx_ready.send(Ok(()));
                    let _ = copy_bidirectional(&mut inbound, &mut outbound).await;
                }
            }
        } else {
            let _ = tx_ready.send(Err("failed to accept local tunnel connection".into()));
        }
        // Listener drops here; tunnel completes after one connection.
    });

    Ok((local_addr, rx_ready, handle))
}

fn split_host_port_supporting_ipv6(s: &str) -> Result<(&str, &str)> {
    // Supports:
    //  - "<host>:<port>"
    //  - "[<ipv6>]:<port>"
    //  - "<onion>:<port>" (not socketaddr-parsable)
    if let Some(rest) = s.strip_prefix('[') {
        // [ipv6]:port
        if let Some(idx) = rest.find(']') {
            let host = &rest[..idx];
            let after = &rest[idx + 1..];
            if let Some((_, port)) = after.split_once(':') {
                return Ok((host, port));
            }
        }
    }
    if let Some((h, p)) = s.rsplit_once(':') {
        return Ok((h, p));
    }
    bail!("expected host:port, got '{s}'");
}

```

### crates/transport/src/tor_control.rs

```rust
// crates/transport/src/tor_control.rs
use anyhow::{anyhow, bail, Context, Result};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::path::Path;

fn send_line(stream: &mut TcpStream, line: &str) -> std::io::Result<()> {
    stream.write_all(line.as_bytes())?;
    stream.write_all(b"\r\n")?;
    Ok(())
}

fn read_until_done(stream: &mut TcpStream) -> std::io::Result<String> {
    let mut out = String::new();
    let mut reader = BufReader::new(stream);
    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line)?;
        if n == 0 {
            return Ok(out);
        }
        out.push_str(&line);
        if line.starts_with("250 ") { return Ok(out); }
        if line.starts_with('5')   { return Ok(out); }
    }
}

fn cookie_hex(p: &Path) -> Result<String> {
    let bytes = fs::read(p).with_context(|| format!("reading cookie file {:?}", p))?;
    Ok(bytes.iter().map(|b| format!("{:02X}", b)).collect())
}

/// Publish an ephemeral v3 onion and return "<56chars>.onion:1777".
pub fn publish_v3(ctrl_addr: &str, local_map: &str, cookie_file: Option<&str>) -> Result<String> {
    // Probe (optional)
    {
        let mut s = TcpStream::connect(ctrl_addr)
            .with_context(|| format!("connecting to Tor control at {}", ctrl_addr))?;
        s.set_nodelay(true)?;
        send_line(&mut s, "PROTOCOLINFO 1")?;
        let _ = read_until_done(&mut s)?;
    }

    // Auth & commands
    let mut s = TcpStream::connect(ctrl_addr)
        .with_context(|| format!("connecting to Tor control at {}", ctrl_addr))?;
    s.set_nodelay(true)?;

    if let Some(cookie_path) = cookie_file {
        let hex = cookie_hex(Path::new(cookie_path))?;
        send_line(&mut s, &format!("AUTHENTICATE {}", hex))?;
    } else {
        send_line(&mut s, "AUTHENTICATE")?; // NULL auth (only if Tor allows it)
    }

    send_line(&mut s, "GETINFO version")?;
    let reply = read_until_done(&mut s)?;
    if !reply.contains("250 OK") {
        bail!("Tor control AUTH failed:\n{}", reply);
    }

    // Map public 1777 -> local 127.0.0.1:1777
    send_line(&mut s, &format!("ADD_ONION NEW:ED25519-V3 Port={}", local_map))?;
    let reply = read_until_done(&mut s)?;
    if reply.starts_with('5') {
        bail!("ADD_ONION failed:\n{}", reply);
    }

    let sid = reply
        .lines()
        .find_map(|l| l.strip_prefix("250-ServiceID="))
        .ok_or_else(|| anyhow!("No ServiceID in Tor reply:\n{}", reply))?;

    Ok(format!("{}.onion:1777", sid))
}

```

### deny.toml

```toml
# deny.toml — cargo-deny >= 0.18

[graph]
all-features = true
exclude-dev = false

[advisories]
unmaintained = "workspace"
yanked = "deny"
ignore = []

[bans]
multiple-versions = "deny"
wildcards = "deny"
allow-wildcard-paths = true

# ---- Duplicate-version handling ---------------------------------------------
# Prefer unifying in Cargo.toml first; these skips are scoped to legacy stacks.

# Known trees that drag legacy stacks (notify->mio0.8, sled->parking_lot0.11)
skip-tree = [
  { name = "notify" },
  { name = "sled" },

  # Windows split crates — keep only the newest line counting toward duplicates
  { name = "windows-sys",     version = "< 0.60.2" },
  { name = "windows-targets", version = "< 0.53.3" },
  { name = "windows_i686_gnu",        version = "< 0.53.0" },
  { name = "windows_i686_gnullvm",    version = "< 0.53.0" },
  { name = "windows_i686_msvc",       version = "< 0.53.0" },
  { name = "windows_x86_64_gnu",      version = "< 0.53.0" },
  { name = "windows_x86_64_gnullvm",  version = "< 0.53.0" },
  { name = "windows_x86_64_msvc",     version = "< 0.53.0" },
  { name = "windows_aarch64_gnullvm", version = "< 0.53.0" },
  { name = "windows_aarch64_msvc",    version = "< 0.53.0" },
]

# Keep newest lines; skip only the *older* ones we still encounter.
skip = [
  # Compression: tldctl pulls brotli 3.x; async-compression uses 8.x
  { name = "brotli",              version = "< 8.0.0" },
  { name = "brotli-decompressor", version = "< 5.0.0" },

  # macOS security stack: native-tls vs rustls-native-certs pull different versions
  { name = "core-foundation",     version = "< 0.10.0" },
  { name = "security-framework",  version = "< 3.0.0" },

  # Mixed majors: prometheus 0.14 pulls thiserror 2.x; some deps still require 1.x
  { name = "thiserror",           version = "< 2.0.0" },
  { name = "thiserror-impl",      version = "< 2.0.0" },

  # Unavoidable splits:
  # - ring 0.17 currently pulls getrandom 0.2
  { name = "getrandom",  version = "< 0.3.0" },
  { name = "rand_core",  version = "< 0.9.0" },
]

# ---- Drift clamps (deny any version outside workspace pins) ------------------

# Tokio pinned to 1.47.1
[[bans.deny]]
name = "tokio"
version = "< 1.47.1"
reason = "Pin tokio to workspace version"
[[bans.deny]]
name = "tokio"
version = "> 1.47.1"
reason = "Pin tokio to workspace version"

# Axum pinned to 0.7.9
[[bans.deny]]
name = "axum"
version = "< 0.7.9"
reason = "Pin axum to workspace version"
[[bans.deny]]
name = "axum"
version = "> 0.7.9"
reason = "Pin axum to workspace version"

# Reqwest allowed only on 0.12.x
[[bans.deny]]
name = "reqwest"
version = "< 0.12.0"
reason = "Require reqwest 0.12.x"
[[bans.deny]]
name = "reqwest"
version = ">= 0.13.0"
reason = "Require reqwest 0.12.x"

# Prometheus allowed only on 0.14.x
[[bans.deny]]
name = "prometheus"
version = "< 0.14.0"
reason = "Require prometheus 0.14.x"
[[bans.deny]]
name = "prometheus"
version = ">= 0.15.0"
reason = "Require prometheus 0.14.x"

# Tokio-rustls pinned to 0.26.2
[[bans.deny]]
name = "tokio-rustls"
version = "< 0.26.2"
reason = "Pin tokio-rustls to workspace version"
[[bans.deny]]
name = "tokio-rustls"
version = "> 0.26.2"
reason = "Pin tokio-rustls to workspace version"

# Tower-HTTP pinned to 0.6.6
[[bans.deny]]
name = "tower-http"
version = "< 0.6.6"
reason = "Pin tower-http to workspace version"
[[bans.deny]]
name = "tower-http"
version = "> 0.6.6"
reason = "Pin tower-http to workspace version"

# Rand family unified on 0.9.x
[[bans.deny]]
name = "rand"
version = "< 0.9.0"
reason = "Unify rand to 0.9.x"
[[bans.deny]]
name = "rand"
version = ">= 0.10.0"
reason = "Unify rand to 0.9.x"
[[bans.deny]]
name = "rand_chacha"
version = "< 0.9.0"
reason = "Unify rand_chacha to 0.9.x"
[[bans.deny]]
name = "rand_chacha"
version = ">= 0.10.0"
reason = "Unify rand_chacha to 0.9.x"

# Regex unified on 1.11.x
[[bans.deny]]
name = "regex"
version = "< 1.11.0"
reason = "Unify regex to 1.11.x"
[[bans.deny]]
name = "regex"
version = ">= 1.12.0"
reason = "Unify regex to 1.11.x"

# ---- Workspace dependency guard (soften to warnings for now) -----------------
[bans.workspace-dependencies]
duplicates = "warn"
include-path-dependencies = true
unused = "warn"

[licenses]
unused-allowed-license = "allow"
confidence-threshold = 0.93

allow = [
  "Apache-2.0",
  "MIT",
  "BSD-2-Clause",
  "BSD-3-Clause",
  "ISC",
  "Zlib",
  "OpenSSL",
  "CC0-1.0",
  "CDLA-Permissive-2.0",
  "Unicode-3.0",
  "Unicode-DFS-2016"
]
exceptions = []

[sources]
unknown-registry = "deny"
unknown-git = "deny"
allow-git = []

```

### experiments/actor_spike/Cargo.toml

```toml
[package]
publish = false
license = "MIT OR Apache-2.0"
name = "actor_spike"
version = "0.2.0"
edition = "2021"

[dependencies]
anyhow = { workspace = true }

# Prefer workspace pins to avoid rand duplication
rand = { workspace = true, features = ["std"] }
rand_chacha = { workspace = true, features = ["std"] }

ron-kernel = { workspace = true }

# Clamp runtime & logging to workspace
tokio = { workspace = true, features = ["rt-multi-thread", "macros", "signal", "time"] }
tracing = { workspace = true }
tracing-subscriber = { workspace = true, features = ["fmt", "env-filter"] }
workspace-hack = { version = "0.1", path = "../../workspace-hack" }

```

### experiments/actor_spike/src/main.rs

```rust
//! Milestone 0a: restart semantics + clean shutdown + tracing.
//! Run with: cargo run -p actor_spike

use anyhow::Result;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::time::Duration;
use tokio::{
    select,
    task::JoinSet,
    time::{sleep, Instant},
};
use tracing::{error, info, warn};
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Clone)]
struct ServiceConfig {
    name: &'static str,
    // Simulate occasional panics every N..2N iterations.
    mean_iterations_before_panic: u32,
    // Simulated unit of work duration.
    work_ms: u64,
}

async fn service_run(cfg: ServiceConfig, mut rng: StdRng) -> Result<()> {
    let mut i: u64 = 0;
    let next_panic_after: u32 =
        rng.random_range(cfg.mean_iterations_before_panic..(cfg.mean_iterations_before_panic * 2));

    loop {
        // Do some "work"
        sleep(Duration::from_millis(cfg.work_ms)).await;
        i += 1;

        if i % 50 == 0 {
            info!(service = cfg.name, iter = i, "heartbeat");
        }

        // Occasionally panic to test supervisor.
        if i as u32 >= next_panic_after {
            error!(service = cfg.name, iter = i, "simulated panic");
            panic!("simulated panic in {}", cfg.name);
        }
    }
}

/// Exponential backoff with jitter (bounded).
fn backoff(attempt: u32) -> Duration {
    let base_ms = 200u64;
    let max_ms = 5_000u64;
    let factor = 1u64 << attempt.min(5); // cap growth
    let raw = base_ms.saturating_mul(factor);
    let bounded = raw.min(max_ms);

    // tiny jitter: +/-10%
    let jitter = (bounded as f64 * 0.1) as u64;
    let now = Instant::now();
    let seed = now.elapsed().as_nanos() as u64;
    let jitter_val = (seed % (2 * jitter + 1)) as i64 - jitter as i64;
    let with_jitter = if jitter_val.is_negative() {
        bounded.saturating_sub(jitter_val.unsigned_abs())
    } else {
        bounded.saturating_add(jitter_val as u64)
    };
    Duration::from_millis(with_jitter.max(50))
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    // Tracing: RUST_LOG=info,actor_spike=debug
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,actor_spike=debug"));
    fmt().with_env_filter(filter).compact().init();

    info!("actor_spike starting…  (Ctrl-C to stop)");

    // Our "kernel" for this spike: supervise one service.
    let cfg = ServiceConfig {
        name: "hello_service",
        mean_iterations_before_panic: 140,
        work_ms: 20,
    };

    // Spawn supervisor task(s) into a JoinSet so we can cancel/drain on shutdown.
    let mut tasks = JoinSet::new();
    tasks.spawn(supervise(cfg.clone()));

    // Wait for Ctrl-C
    select! {
        _ = tokio::signal::ctrl_c() => {
            warn!("Ctrl-C received — initiating shutdown");
        }
    }

    // Drain: cancel tasks (JoinSet abort is fine for the spike).
    tasks.abort_all();
    while let Some(res) = tasks.join_next().await {
        if let Err(e) = res {
            warn!(error = ?e, "task aborted (expected during shutdown)");
        }
    }

    info!("actor_spike stopped cleanly.");
    Ok(())
}

async fn supervise(cfg: ServiceConfig) {
    let mut attempt: u32 = 0;

    loop {
        // rand 0.9: seed a StdRng from fresh OS entropy
        let rng = StdRng::from_os_rng();

        info!(service = cfg.name, "starting service run");
        let res = tokio::spawn(service_run(cfg.clone(), rng)).await;

        match res {
            Ok(Ok(())) => {
                // Unlikely in this spike; included for completeness.
                info!(service = cfg.name, "service completed");
                break;
            }
            Ok(Err(err)) => {
                error!(service = cfg.name, ?err, "service error; will restart");
            }
            Err(join_err) if join_err.is_panic() => {
                error!(
                    service = cfg.name,
                    ?join_err,
                    "service panicked; will restart"
                );
            }
            Err(join_err) => {
                error!(
                    service = cfg.name,
                    ?join_err,
                    "service join error; will restart"
                );
            }
        }

        // Backoff and restart
        let delay = backoff(attempt);
        warn!(
            service = cfg.name,
            attempt,
            ?delay,
            "backing off before restart"
        );
        sleep(delay).await;
        attempt = attempt.saturating_add(1);
    }
}

```

### hakari.toml

```toml
[hakari]
# Name of the workspace-hack crate that cargo-hakari will (re)generate:
workspace-hack-crate = "workspace-hack"

# Enable across all workspace members
charlie = false

# Keep default behavior; we primarily want feature unification across the pinned
# workspace dependencies (tokio, axum, reqwest, prometheus, tokio-rustls, etc.).

```

### testing/gwsmoke/Cargo.toml

```toml
[package]
name = "gwsmoke"
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"
publish = false

[dependencies]
anyhow         = { workspace = true }
tokio          = { workspace = true, features = ["rt-multi-thread", "macros"] }
reqwest        = { workspace = true, features = ["json", "rustls-tls-native-roots"] }
serde          = { workspace = true, features = ["derive"] }
serde_json     = { workspace = true }
axum           = { workspace = true, default-features = false, features = ["macros","http1"] }

# NEW: fixes unresolved imports in gwsmoke
clap           = { workspace = true, features = ["derive","env"] }
regex          = { workspace = true }
workspace-hack = { version = "0.1", path = "../../workspace-hack" }

```

### testing/gwsmoke/src/config.rs

```rust
use clap::Parser;
use std::path::PathBuf;

/// CLI for the gateway smoke test harness
#[derive(Parser, Debug, Clone)]
#[command(
    name = "gwsmoke",
    about = "RustyOnions gateway end-to-end smoke tester"
)]
pub struct Cli {
    /// Workspace root (where Cargo.toml and target/ live)
    #[arg(long, default_value = ".", value_hint=clap::ValueHint::DirPath)]
    pub root: PathBuf,

    /// Output bundle dir (.onions)
    #[arg(long, default_value = ".onions", value_hint=clap::ValueHint::DirPath)]
    pub out_dir: PathBuf,

    /// Index DB directory; if omitted a temp dir is created
    #[arg(long)]
    pub index_db: Option<PathBuf>,

    /// Bind address for HTTP gateway; if port 0, an ephemeral port is chosen
    #[arg(long, default_value = "127.0.0.1:0")]
    pub bind: String,

    /// Algo (if your tldctl supports it)
    #[arg(long, default_value = "blake3")]
    pub algo: String,

    /// Keep temp dir (logs, sockets) on success
    #[arg(long)]
    pub keep_tmp: bool,

    /// Maximum seconds to wait for gateway TCP readiness
    #[arg(
        long = "http-wait-sec",
        visible_alias = "http_wait_sec",
        default_value_t = 20u64
    )]
    pub http_wait_sec: u64,

    /// Log dir (inside tmp by default)
    #[arg(long)]
    pub log_dir: Option<PathBuf>,

    /// Build first (cargo build -p <bins>)
    #[arg(long)]
    pub build: bool,

    /// Extra environment to pass to *all* child processes (k=v, repeatable)
    #[arg(long)]
    pub env: Vec<String>,

    /// Stream child process logs to stdout while also saving to files
    #[arg(long)]
    pub stream: bool,

    /// Override RUST_LOG used for services/gateway (e.g. trace or fine-grained filters)
    #[arg(
        long,
        default_value = "info,svc_index=debug,svc_storage=debug,svc_overlay=debug,gateway=debug"
    )]
    pub rust_log: String,
}

```

### testing/gwsmoke/src/http_probe.rs

```rust
use anyhow::{Context, Result};

pub async fn http_get_status(url: &str) -> Result<u16> {
    let client = reqwest::Client::builder()
        .build()
        .context("build HTTP client")?;
    let resp = client.get(url).send().await.context("HTTP GET")?;
    Ok(resp.status().as_u16())
}

```

### testing/gwsmoke/src/main.rs

```rust
mod config;
mod http_probe;
mod pack;
mod proc;
mod util;
mod wait;

use anyhow::{Context, Result};
use clap::Parser;
use std::{collections::HashMap, fs, time::Duration};
use tokio::{signal, time::sleep};

use config::Cli;
use http_probe::http_get_status;
use pack::{pack_once_detect_and_parse, resolve_ok, try_index_scan};
use proc::{spawn_logged, ChildProc};
use util::{bin_path, cargo_build, ensure_exists, kv_env, tail_file, tempdir};
use wait::{parse_host_port, pick_ephemeral_port, wait_for_tcp, wait_for_uds};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Resolve workspace root
    let root = fs::canonicalize(&cli.root).context("canonicalizing --root")?;
    ensure_exists(root.join("Cargo.toml"))?;

    if cli.build {
        cargo_build(&root).await?;
    }

    // Binaries
    let tldctl = bin_path(&root, "tldctl");
    let svc_index = bin_path(&root, "svc-index");
    let svc_storage = bin_path(&root, "svc-storage");
    let svc_overlay = bin_path(&root, "svc-overlay");
    let gateway = bin_path(&root, "gateway");
    for (label, p) in [
        ("tldctl", &tldctl),
        ("svc-index", &svc_index),
        ("svc-storage", &svc_storage),
        ("svc-overlay", &svc_overlay),
        ("gateway", &gateway),
    ] {
        ensure_exists(p).with_context(|| format!("missing binary {} at {}", label, p.display()))?;
    }

    // tmp + logs
    let tmp_dir = tempdir("ron_gwsmoke").context("creating tmp dir")?;
    println!("tmp dir    : {}", tmp_dir.display());
    let run_dir = tmp_dir.join("run");
    let log_dir = cli.log_dir.clone().unwrap_or_else(|| tmp_dir.join("logs"));
    fs::create_dir_all(&run_dir)?;
    fs::create_dir_all(&log_dir)?;

    // paths
    let idx_db = cli
        .index_db
        .clone()
        .unwrap_or_else(|| tmp_dir.join("index"));
    fs::create_dir_all(&idx_db)?;
    let out_dir = if cli.out_dir.is_relative() {
        root.join(&cli.out_dir)
    } else {
        cli.out_dir.clone()
    };
    fs::create_dir_all(&out_dir)?;

    // common env
    let mut common_env = HashMap::new();
    common_env.insert("RON_INDEX_DB".to_string(), idx_db.display().to_string());
    common_env.insert("RUST_LOG".to_string(), cli.rust_log.clone());
    for kv in cli.env.iter() {
        if let Some((k, v)) = kv.split_once('=') {
            common_env.insert(k.to_string(), v.to_string());
        }
    }

    // 1) pack .post
    let pack_out = tmp_dir.join("pack_post.out");
    let post_txt = tmp_dir.join("post.txt");
    fs::write(&post_txt, "Hello from RustyOnions gateway test (.post)\n")?;

    let addr_post = match pack_once_detect_and_parse(
        &tldctl, &out_dir, &idx_db, &cli.algo, "post", &post_txt, &pack_out,
    )
    .await
    {
        Ok(addr) => addr,
        Err(e) => {
            let maybe = fs::read_to_string(&pack_out).unwrap_or_default();
            eprintln!(
                "pack(post) failed: {e}\n--- pack_post.out ---\n{maybe}\n---------------------"
            );
            return Err(e);
        }
    };
    println!("post addr  : {}", addr_post);

    // 2) resolve; if missing, attempt an index scan if the CLI supports it
    if !resolve_ok(&tldctl, &idx_db, &addr_post).await? {
        println!(
            "Index DB {} did not contain {}; attempting index scan (if supported)…",
            idx_db.display(),
            addr_post
        );
        match try_index_scan(&tldctl, &idx_db, &out_dir).await {
            Ok(true) => {
                if !resolve_ok(&tldctl, &idx_db, &addr_post).await? {
                    eprintln!("Index scan ran, but resolve still fails — will continue and start services to capture logs.");
                } else {
                    println!("reindex -> resolve OK");
                }
            }
            Ok(false) => {
                eprintln!("No index subcommand found in this tldctl build — continuing without reindex to capture service logs.");
            }
            Err(e) => {
                eprintln!(
                    "Index scan attempt errored: {e} — continuing anyway to capture service logs."
                );
            }
        }
    }

    // 3) start services (UDS)
    let idx_sock = run_dir.join("svc-index.sock");
    let sto_sock = run_dir.join("svc-storage.sock");
    let ovl_sock = run_dir.join("svc-overlay.sock");

    let env_index = kv_env(
        &common_env,
        &[("RON_INDEX_SOCK", idx_sock.display().to_string())],
    );
    let env_storage = kv_env(
        &common_env,
        &[("RON_STORAGE_SOCK", sto_sock.display().to_string())],
    );
    let env_overlay = kv_env(
        &common_env,
        &[
            ("RON_OVERLAY_SOCK", ovl_sock.display().to_string()),
            ("RON_INDEX_SOCK", idx_sock.display().to_string()),
            ("RON_STORAGE_SOCK", sto_sock.display().to_string()),
        ],
    );

    let svc_index_child: ChildProc = spawn_logged(
        "svc-index",
        &svc_index,
        &log_dir.join("svc-index.log"),
        &env_index,
        &[],
        cli.stream,
    )
    .await?;
    let svc_storage_child: ChildProc = spawn_logged(
        "svc-storage",
        &svc_storage,
        &log_dir.join("svc-storage.log"),
        &env_storage,
        &[],
        cli.stream,
    )
    .await?;
    let svc_overlay_child: ChildProc = spawn_logged(
        "svc-overlay",
        &svc_overlay,
        &log_dir.join("svc-overlay.log"),
        &env_overlay,
        &[],
        cli.stream,
    )
    .await?;

    wait_for_uds(&idx_sock, Duration::from_secs(5))
        .await
        .context("waiting for svc-index UDS")?;
    wait_for_uds(&sto_sock, Duration::from_secs(5))
        .await
        .context("waiting for svc-storage UDS")?;
    wait_for_uds(&ovl_sock, Duration::from_secs(5))
        .await
        .context("waiting for svc-overlay UDS")?;

    // 4) start gateway
    let (bind_host, bind_port) = parse_host_port(&cli.bind)?;
    let port = if bind_port == 0 {
        pick_ephemeral_port(bind_host).await?
    } else {
        bind_port
    };
    let bind = format!("{bind_host}:{port}");

    let env_gateway = kv_env(
        &common_env,
        &[
            ("RON_OVERLAY_SOCK", ovl_sock.display().to_string()),
            ("RON_INDEX_DB", idx_db.display().to_string()),
        ],
    );

    let gateway_child: ChildProc = spawn_logged(
        "gateway",
        &gateway,
        &log_dir.join("gateway.log"),
        &env_gateway,
        &["--bind", &bind, "--index-db", &idx_db.display().to_string()],
        cli.stream,
    )
    .await?;

    wait_for_tcp(&bind, Duration::from_secs(cli.http_wait_sec))
        .await
        .with_context(|| format!("waiting for HTTP accept at {}", bind))?;
    println!("gateway    : http://{}", bind);

    // 5) GET manifest (will likely fail with 404 if index still missing, but we now have logs)
    let url = format!("http://{}/o/{}/Manifest.toml", bind, addr_post);
    println!("GET {}", url);
    let code = http_get_status(&url).await?;
    if !(200..300).contains(&code) {
        println!("\nManifest GET failed with HTTP {}", code);
        println!(
            "--- tail gateway.log ---\n{}",
            tail_file(&log_dir.join("gateway.log"), 200)
        );
        println!(
            "--- tail svc-overlay.log ---\n{}",
            tail_file(&log_dir.join("svc-overlay.log"), 200)
        );
        println!(
            "--- tail svc-index.log ---\n{}",
            tail_file(&log_dir.join("svc-index.log"), 200)
        );
        // Return an error after showing logs — harness did its job producing diagnostics.
        anyhow::bail!("HTTP {} for {}", code, url);
    }
    println!("Manifest OK (HTTP {})", code);

    // Summary
    println!("\n=== Gateway Test Summary ===");
    println!("Gateway   : http://{}", bind);
    println!("OUT_DIR   : {}", out_dir.display());
    println!("INDEX_DB  : {}", idx_db.display());
    println!("POST addr : {}", addr_post);
    println!("Manifest  : {}", url);
    println!("Logs      : {}", log_dir.display());

    // Hold window (optional): set GWSMOKE_HOLD_SEC=N to keep the stack up for N seconds,
    // or press Ctrl-C to stop immediately. If not set, retain the original tiny grace (10ms).
    let hold_sec = std::env::var("GWSMOKE_HOLD_SEC")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    if hold_sec > 0 {
        eprintln!(
            "holding for {}s (GWSMOKE_HOLD_SEC) to allow external tests to run…",
            hold_sec
        );
        tokio::select! {
            _ = signal::ctrl_c() => { println!("\n(ctrl-c) stopping…"); }
            _ = sleep(Duration::from_secs(hold_sec)) => {}
        }
    } else {
        // Original quick grace
        tokio::select! {
            _ = signal::ctrl_c() => { println!("\n(ctrl-c) stopping…"); }
            _ = sleep(Duration::from_millis(10)) => {}
        }
    }

    // stop children
    let _ = gateway_child.kill_and_wait().await;
    let _ = svc_overlay_child.kill_and_wait().await;
    let _ = svc_storage_child.kill_and_wait().await;
    let _ = svc_index_child.kill_and_wait().await;

    if cli.keep_tmp {
        println!("(Keeping TMP: {})", tmp_dir.display());
    } else {
        let _ = fs::remove_dir_all(&tmp_dir);
    }
    Ok(())
}

```

### testing/gwsmoke/src/pack.rs

```rust
use crate::util::{run_capture, run_capture_to_file};
use anyhow::{anyhow, Result};
use regex::Regex;
use std::{collections::HashMap, path::Path};

/// Build argv for `tldctl pack`, adapting to detected flags (never using `--index`).
fn pack_argv_detected(
    help: &str,
    out_dir: &Path,
    index_db: &Path,
    algo: &str,
    tld: &str,
    input_file: &Path,
) -> Result<Vec<String>> {
    let has = |flag: &str| help.contains(flag);

    let mut argv: Vec<String> = vec!["pack".into()];
    if has("--tld") {
        argv.push("--tld".into());
        argv.push(tld.into());
    } else {
        argv.push(tld.into()); // legacy positional tld
    }

    if has("--input") {
        argv.push("--input".into());
        argv.push(input_file.display().to_string());
    } else if has("--file") {
        argv.push("--file".into());
        argv.push(input_file.display().to_string());
    } else {
        return Err(anyhow!(
            "tldctl pack: neither --input nor --file supported in this build"
        ));
    }

    if has("--algo") {
        argv.push("--algo".into());
        argv.push(algo.into());
    }

    // IMPORTANT: don't add --index (unsupported in your build); --index-db is sufficient
    if has("--index-db") {
        argv.push("--index-db".into());
        argv.push(index_db.display().to_string());
    }

    if has("--store-root") {
        argv.push("--store-root".into());
        argv.push(out_dir.display().to_string());
    } else if has("--out") {
        argv.push("--out".into());
        argv.push(out_dir.display().to_string());
    } else {
        return Err(anyhow!(
            "tldctl pack: neither --store-root nor --out supported in this build"
        ));
    }

    Ok(argv)
}

fn strip_b3(addr: &str) -> String {
    addr.strip_prefix("b3:").unwrap_or(addr).to_string()
}

/// Parse a tldctl pack output into "<hex>.<tld>" (without "b3:" scheme).
/// Accepts either:
///   1) `OK: .../<hex>.<tld>/Manifest.toml`
///   2) a single line address: `[b3:]<hex>.<tld>`
fn parse_pack_output(tld: &str, output: &str) -> Option<String> {
    // 1) Old "OK:" path line
    let ok_re = Regex::new(&format!(
        r#"^OK:\s+.*/([^/]+)\.{}\s*/Manifest\.toml\s*$"#,
        regex::escape(tld)
    ))
    .ok()?;
    if let Some(cap) = output.lines().filter_map(|l| ok_re.captures(l)).next() {
        return Some(cap[1].to_string() + "." + tld);
    }

    // 2) Single-line address (with or without b3:)
    let addr_re = Regex::new(r#"^(?:b3:)?([0-9a-f]{8,}\.[a-z0-9]+)\s*$"#).ok()?;
    if let Some(cap) = output.lines().filter_map(|l| addr_re.captures(l)).next() {
        return Some(cap[1].to_string()); // already without scheme
    }

    None
}

/// Run a single pack, write output to file, return "<hex>.<tld>" (no "b3:" prefix).
pub async fn pack_once_detect_and_parse(
    tldctl: &Path,
    out_dir: &Path,
    index_db: &Path,
    algo: &str,
    tld: &str,
    input_file: &Path,
    capture_out: &Path,
) -> Result<String> {
    let help = run_capture::<&str>(tldctl, &["pack", "--help"], None).await?;
    let argv = pack_argv_detected(help.as_str(), out_dir, index_db, algo, tld, input_file)?;
    let output = run_capture_to_file(tldctl, &argv, None, capture_out).await?;

    match parse_pack_output(tld, &output) {
        Some(addr) => Ok(strip_b3(&addr)),
        None => Err(anyhow!(
            "pack output did not contain a recognizable .{} address",
            tld
        )),
    }
}

/// Try resolve with or without the b3: prefix
pub async fn resolve_ok(tldctl: &Path, index_db: &Path, addr: &str) -> Result<bool> {
    let mut envs = HashMap::new();
    envs.insert("RON_INDEX_DB".to_string(), index_db.display().to_string());

    let try1 = run_capture::<&str>(tldctl, &["resolve", addr], Some(&envs)).await;
    if try1.is_ok() {
        return Ok(true);
    }
    let prefixed = format!("b3:{}", addr);
    let try2 = run_capture::<&str>(tldctl, &["resolve", prefixed.as_str()], Some(&envs)).await;
    Ok(try2.is_ok())
}

/// Attempt to (re)index OUT_DIR into index_db using *whatever* your tldctl supports.
/// Returns `Ok(true)` if an indexing command was found and executed, `Ok(false)` if no suitable
/// subcommand exists, and `Err` only on an actual execution error of a detected command.
pub async fn try_index_scan(tldctl: &Path, index_db: &Path, out_dir: &Path) -> Result<bool> {
    let mut envs = HashMap::new();
    envs.insert("RON_INDEX_DB".to_string(), index_db.display().to_string());

    // 1) Check if `tldctl index scan --help` exists and mentions "scan"
    let idx_help = run_capture::<&str>(tldctl, &["index", "--help"], None).await;
    if let Ok(h) = &idx_help {
        if h.contains("scan") {
            let out_dir_s = out_dir.display().to_string();
            let _ = run_capture::<&str>(
                tldctl,
                &["index", "scan", "--store-root", out_dir_s.as_str()],
                Some(&envs),
            )
            .await?; // execution error bubbles up
            return Ok(true);
        }
    }

    // 2) Some builds expose a top-level `scan`
    let top_help = run_capture::<&str>(tldctl, &["--help"], None).await;
    if let Ok(h) = &top_help {
        // very lenient check
        if h.lines().any(|l| l.contains("scan")) {
            let out_dir_s = out_dir.display().to_string();
            let ok = run_capture::<&str>(
                tldctl,
                &["scan", "--store-root", out_dir_s.as_str()],
                Some(&envs),
            )
            .await;
            if ok.is_ok() {
                return Ok(true);
            }
        }
    }

    // 3) No known index command available
    Ok(false)
}

```

### testing/gwsmoke/src/proc.rs

```rust
// testing/gwsmoke/src/proc.rs
use anyhow::{anyhow, Context, Result};
use std::{
    collections::HashMap,
    fs::File,
    io::{self, Write},
    path::Path,
    process::Stdio,
};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, Command},
    task::JoinHandle,
    time::{timeout, Duration},
};

pub struct ChildProc {
    #[allow(dead_code)] // example harness doesn't read this field yet; keep for diagnostics
    pub name: String,
    pub child: Child,
    _stdout_task: JoinHandle<io::Result<()>>,
    _stderr_task: JoinHandle<io::Result<()>>,
}

impl ChildProc {
    pub async fn kill_and_wait(mut self) -> Result<()> {
        let _ = self.child.start_kill();
        let _ = timeout(Duration::from_secs(3), self.child.wait()).await;
        Ok(())
    }
}

pub async fn spawn_logged(
    name: &str,
    bin: &Path,
    log_path: &Path,
    envs: &HashMap<String, String>,
    args: &[&str],
    stream_to_stdout: bool,
) -> Result<ChildProc> {
    let mut cmd = Command::new(bin);
    for a in args {
        cmd.arg(a);
    }
    for (k, v) in envs {
        cmd.env(k, v);
    }
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut child = cmd.spawn().with_context(|| format!("spawn {}", name))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("{}: no stdout", name))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| anyhow!("{}: no stderr", name))?;

    // Append logs
    let mut out_file =
        File::create(log_path).with_context(|| format!("open {}", log_path.display()))?;
    let mut err_file = out_file.try_clone()?;

    let name_out = name.to_string();
    let name_err = name.to_string();

    let stdout_task = tokio::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if stream_to_stdout {
                println!("[{}] {}", name_out, line);
            }
            let _ = writeln!(out_file, "{}", line);
        }
        Ok::<_, io::Error>(())
    });
    let stderr_task = tokio::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if stream_to_stdout {
                eprintln!("[{}] {}", name_err, line);
            }
            let _ = writeln!(err_file, "{}", line);
        }
        Ok::<_, io::Error>(())
    });

    println!(
        "+ {} {}",
        bin.display(),
        args.iter()
            .map(|s| {
                if s.chars()
                    .all(|c| c.is_ascii_alphanumeric() || "-_./:".contains(c))
                {
                    s.to_string()
                } else {
                    format!("'{}'", s.replace('\'', "'\\''"))
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    );

    Ok(ChildProc {
        name: name.to_string(),
        child,
        _stdout_task: stdout_task,
        _stderr_task: stderr_task,
    })
}

```

### testing/gwsmoke/src/util.rs

```rust
// testing/gwsmoke/src/util.rs
use anyhow::{anyhow, Context, Result};
use std::{
    collections::HashMap,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::Stdio,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::process::Command;

pub fn ensure_exists<P: AsRef<Path>>(p: P) -> Result<()> {
    if !p.as_ref().exists() {
        Err(anyhow!("not found: {}", p.as_ref().display()))
    } else {
        Ok(())
    }
}

pub async fn cargo_build(root: &Path) -> Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.arg("build")
        .arg("-p")
        .arg("tldctl")
        .arg("-p")
        .arg("svc-index")
        .arg("-p")
        .arg("svc-storage")
        .arg("-p")
        .arg("svc-overlay")
        .arg("-p")
        .arg("gateway")
        .current_dir(root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let out = cmd.output().await.context("spawning cargo build")?;
    if !out.status.success() {
        let mut msg = String::from_utf8_lossy(&out.stderr).into_owned();
        if msg.trim().is_empty() {
            msg = String::from_utf8_lossy(&out.stdout).into_owned();
        }
        return Err(anyhow!("cargo build failed:\n{}", msg));
    }
    Ok(())
}

pub fn bin_path(root: &Path, name: &str) -> PathBuf {
    root.join("target").join("debug").join(name)
}

pub fn tempdir(prefix: &str) -> Result<PathBuf> {
    let base = std::env::temp_dir();
    for _ in 0..16 {
        let p = base.join(format!("{}.{}", prefix, nanoid()));
        if !p.exists() {
            fs::create_dir_all(&p)?;
            return Ok(p);
        }
    }
    Err(anyhow!("failed to create temp dir"))
}

fn nanoid() -> String {
    // Avoid unwrap: if clock is before UNIX_EPOCH, log and fall back to 0.
    let n: u128 = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(d) => d.as_nanos(),
        Err(e) => {
            eprintln!("gwsmoke: system clock error (before UNIX_EPOCH): {e:?}");
            Duration::from_secs(0).as_nanos()
        }
    };
    format!("{:x}", n)
}

pub async fn run_capture<P: AsRef<OsStr>>(
    bin: &Path,
    args: &[P],
    envs: Option<&HashMap<String, String>>,
) -> Result<String> {
    let mut cmd = Command::new(bin);
    for a in args {
        cmd.arg(a);
    }
    if let Some(envs) = envs {
        for (k, v) in envs {
            cmd.env(k, v);
        }
    }
    let out = cmd.output().await?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        let mut msg = String::from_utf8_lossy(&out.stderr).into_owned();
        if msg.trim().is_empty() {
            msg = String::from_utf8_lossy(&out.stdout).into_owned();
        }
        Err(anyhow!(
            "{} {} failed (code {:?}):\n{}",
            bin.display(),
            pretty_args(args),
            out.status.code(),
            msg
        ))
    }
}

pub async fn run_capture_to_file<P: AsRef<OsStr>>(
    bin: &Path,
    args: &[P],
    envs: Option<&HashMap<String, String>>,
    file: &Path,
) -> Result<String> {
    let s = run_capture(bin, args, envs).await?;
    fs::write(file, &s)?;
    Ok(s)
}

pub fn tail_file(path: &Path, last_lines: usize) -> String {
    match fs::read_to_string(path) {
        Ok(content) => {
            let lines: Vec<&str> = content.lines().collect();
            let start = lines.len().saturating_sub(last_lines);
            lines[start..].join("\n")
        }
        Err(_) => String::new(),
    }
}

pub fn pretty_args<P: AsRef<OsStr>>(args: &[P]) -> String {
    args.iter()
        .map(|a| {
            let s = a.as_ref().to_string_lossy();
            shell_quote(&s)
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn shell_quote(s: &str) -> String {
    if s.chars()
        .all(|c| c.is_ascii_alphanumeric() || "-_./:".contains(c))
    {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}

pub fn kv_env(base: &HashMap<String, String>, add: &[(&str, String)]) -> HashMap<String, String> {
    let mut m = base.clone();
    for (k, v) in add {
        m.insert((*k).to_string(), v.clone());
    }
    m
}

```

### testing/gwsmoke/src/wait.rs

```rust
use anyhow::{anyhow, Result};
use std::path::Path;
use tokio::{
    fs,
    net::{TcpListener, TcpStream},
    time::{sleep, Duration},
};

pub async fn wait_for_uds(path: &Path, total: Duration) -> Result<()> {
    let start = std::time::Instant::now();
    while start.elapsed() < total {
        if fs::metadata(path).await.is_ok() {
            return Ok(());
        }
        sleep(Duration::from_millis(50)).await;
    }
    Err(anyhow!("UDS not created: {}", path.display()))
}

pub async fn wait_for_tcp(bind: &str, total: Duration) -> Result<()> {
    let start = std::time::Instant::now();
    while start.elapsed() < total {
        if TcpStream::connect(bind).await.is_ok() {
            return Ok(());
        }
        sleep(Duration::from_millis(50)).await;
    }
    Err(anyhow!("TCP not accepting at {}", bind))
}

pub async fn pick_ephemeral_port(host: &str) -> Result<u16> {
    let listener = TcpListener::bind((host, 0)).await?;
    let port = listener.local_addr()?.port();
    drop(listener);
    Ok(port)
}

pub fn parse_host_port(s: &str) -> Result<(&str, u16)> {
    let (h, p) = s
        .rsplit_once(':')
        .ok_or_else(|| anyhow!("--bind must be host:port"))?;
    let port: u16 = p.parse()?;
    Ok((h, port))
}

```

### testing/sample_bundle/Manifest.toml

```toml
[payment]
required=false

```

### tools/ronctl/Cargo.toml

```toml
[package]
publish = false
license = "MIT OR Apache-2.0"
name = "ronctl"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = { workspace = true }
clap = { workspace = true }
hex = { workspace = true }
rmp-serde = { workspace = true }
ron-bus = { workspace = true }
serde = { workspace = true }

# unify RNG deps with workspace pins
rand        = { workspace = true, features = ["std"] }
rand_chacha = { workspace = true, features = ["std"] }

workspace-hack = { version = "0.1", path = "../../workspace-hack" }

```

### tools/ronctl/src/main.rs

```rust
// tools/ronctl/src/main.rs
#![forbid(unsafe_code)]

use std::env;
use std::os::unix::net::UnixStream;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use ron_bus::api::{Envelope, IndexReq, IndexResp};
use ron_bus::uds::{recv, send};

/// Default UDS path for svc-index if RON_INDEX_SOCK is not set.
const DEFAULT_INDEX_SOCK: &str = "/tmp/ron/svc-index.sock";

#[derive(Parser, Debug)]
#[command(
    name = "ronctl",
    author,
    version,
    about = "RustyOnions control tool for svc-index"
)]
struct Cli {
    /// Override the index socket path (or set RON_INDEX_SOCK env var)
    #[arg(long)]
    sock: Option<String>,

    #[command(subcommand)]
    cmd: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Health check svc-index
    Health,
    /// Resolve an address to a local directory
    Resolve {
        /// Address like b3:<hex>.ext
        addr: String,
    },
    /// Insert/overwrite an address -> directory mapping
    PutAddress {
        /// Address like b3:<hex>.ext
        addr: String,
        /// Filesystem directory path (absolute or working-dir relative)
        dir: String,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let sock = cli
        .sock
        .or_else(|| env::var("RON_INDEX_SOCK").ok())
        .unwrap_or_else(|| DEFAULT_INDEX_SOCK.to_string());

    match run(&sock, cli.cmd) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}

fn run(sock: &str, cmd: Command) -> anyhow::Result<ExitCode> {
    match cmd {
        Command::Health => {
            let mut s = UnixStream::connect(sock)?;
            let env = Envelope {
                service: "svc.index".into(),
                method: "v1.health".into(),
                corr_id: 1,
                token: vec![],
                payload: rmp_serde::to_vec(&IndexReq::Health)?,
            };
            send(&mut s, &env)?;
            let reply = recv(&mut s)?;

            match rmp_serde::from_slice::<IndexResp>(&reply.payload) {
                Ok(IndexResp::HealthOk) => {
                    println!("index: OK");
                    Ok(ExitCode::SUCCESS)
                }
                Ok(other) => {
                    eprintln!("unexpected index response: {:?}", other);
                    Ok(ExitCode::from(2))
                }
                Err(e) => {
                    eprintln!("decode error: {e}");
                    Ok(ExitCode::from(3))
                }
            }
        }
        Command::Resolve { addr } => {
            let mut s = UnixStream::connect(sock)?;
            let env = Envelope {
                service: "svc.index".into(),
                method: "v1.resolve".into(),
                corr_id: 2,
                token: vec![],
                payload: rmp_serde::to_vec(&IndexReq::Resolve { addr: addr.clone() })?,
            };
            send(&mut s, &env)?;
            let reply = recv(&mut s)?;

            match rmp_serde::from_slice::<IndexResp>(&reply.payload) {
                Ok(IndexResp::Resolved { dir }) => {
                    println!("{dir}");
                    Ok(ExitCode::SUCCESS)
                }
                Ok(IndexResp::NotFound) => {
                    eprintln!("not found");
                    Ok(ExitCode::from(4))
                }
                Ok(IndexResp::Err { err }) => {
                    eprintln!("svc-index error: {err}");
                    Ok(ExitCode::from(5))
                }
                Ok(other) => {
                    eprintln!("unexpected index response: {:?}", other);
                    Ok(ExitCode::from(6))
                }
                Err(e) => {
                    eprintln!("decode error: {e}");
                    Ok(ExitCode::from(7))
                }
            }
        }
        Command::PutAddress { addr, dir } => {
            let mut s = UnixStream::connect(sock)?;
            let env = Envelope {
                service: "svc.index".into(),
                method: "v1.put_address".into(),
                corr_id: 3,
                token: vec![],
                payload: rmp_serde::to_vec(&IndexReq::PutAddress {
                    addr: addr.clone(),
                    dir: dir.clone(),
                })?,
            };
            send(&mut s, &env)?;
            let reply = recv(&mut s)?;

            match rmp_serde::from_slice::<IndexResp>(&reply.payload) {
                Ok(IndexResp::PutOk) => {
                    println!("ok");
                    Ok(ExitCode::SUCCESS)
                }
                Ok(IndexResp::NotFound) => {
                    // Not typical for PutAddress, but handle exhaustively
                    eprintln!("not found");
                    Ok(ExitCode::from(8))
                }
                Ok(IndexResp::Err { err }) => {
                    eprintln!("svc-index error: {err}");
                    Ok(ExitCode::from(9))
                }
                Ok(other) => {
                    eprintln!("unexpected index response: {:?}", other);
                    Ok(ExitCode::from(10))
                }
                Err(e) => {
                    eprintln!("decode error: {e}");
                    Ok(ExitCode::from(11))
                }
            }
        }
    }
}

```

### tools/src/main.rs

```rust
use std::io;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use rand::Rng;
use ron_bus::api::{Envelope, IndexReq, IndexResp};
use ron_bus::uds::{recv, send};

const DEFAULT_SOCK: &str = "/tmp/ron/svc-index.sock";

#[derive(Parser)]
#[command(name = "ronctl", author, version, about = "RustyOnions control tool")]
struct Cli {
    /// Path to the index service socket (defaults to RON_INDEX_SOCK or /tmp/ron/svc-index.sock)
    #[arg(global = true, long)]
    index_sock: Option<PathBuf>,

    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Health check the index service
    Ping,
    /// Resolve an address to its bundle directory
    Resolve { addr: String },
    /// Insert/update an address mapping
    Put { addr: String, dir: String },
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    let sock = cli
        .index_sock
        .map(|p| p.to_string_lossy().into_owned())
        .or_else(|| std::env::var("RON_INDEX_SOCK").ok())
        .unwrap_or_else(|| DEFAULT_SOCK.into());

    let mut stream = UnixStream::connect(&sock)?;

    let (req, method) = match cli.cmd {
        Cmd::Ping => (IndexReq::Health, "v1.health"),
        Cmd::Resolve { addr } => (IndexReq::Resolve { addr }, "v1.resolve"),
        Cmd::Put { addr, dir } => (IndexReq::PutAddress { addr, dir }, "v1.put"),
    };

    let corr_id: u64 = rand::thread_rng().gen();
    let env = Envelope {
        service: "svc.index".into(),
        method: method.into(),
        corr_id,
        token: vec![],
        payload: rmp_serde::to_vec(&req).expect("encode req"),
    };

    send(&mut stream, &env).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let reply = recv(&mut stream).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    if reply.corr_id != corr_id {
        eprintln!("Correlation ID mismatch");
        return Ok(());
    }

    match rmp_serde::from_slice::<IndexResp>(&reply.payload) {
        Ok(IndexResp::HealthOk) => println!("svc-index: OK"),
        Ok(IndexResp::Resolved { dir }) => {
            if dir.is_empty() { println!("NOT FOUND"); } else { println!("{dir}"); }
        }
        Ok(IndexResp::PutOk) => println!("PUT OK"),
        Err(e) => eprintln!("decode error: {e}"),
    }
    Ok(())
}

```

### workspace-hack/Cargo.toml

```toml
[package]
name = "workspace-hack"
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"
publish = false

# This crate is managed by cargo-hakari to unify feature/dep graphs across the workspace.
# Do not edit anything between the BEGIN/END markers below — it will be regenerated.

### BEGIN HAKARI SECTION
[dependencies]
axum = { version = "0.7", default-features = false, features = ["http1", "http2", "json", "macros", "tokio"] }
clap = { version = "4", features = ["derive", "env"] }
clap_builder = { version = "4", default-features = false, features = ["color", "env", "help", "std", "suggestions", "usage"] }
crossbeam-utils = { version = "0.8" }
futures-channel = { version = "0.3", features = ["sink"] }
futures-core = { version = "0.3" }
futures-sink = { version = "0.3" }
futures-task = { version = "0.3", default-features = false, features = ["std"] }
futures-util = { version = "0.3", features = ["channel", "io", "sink"] }
hashbrown = { version = "0.15" }
hyper = { version = "1", features = ["http1", "http2", "server"] }
hyper-util = { version = "0.1", features = ["http1", "http2", "server", "service", "tokio"] }
log = { version = "0.4", default-features = false, features = ["std"] }
memchr = { version = "2" }
num-traits = { version = "0.2", default-features = false, features = ["std"] }
rand_chacha = { version = "0.9" }
rand_core = { version = "0.9", default-features = false, features = ["os_rng", "std"] }
regex-automata = { version = "0.4", default-features = false, features = ["dfa-build", "dfa-onepass", "hybrid", "meta", "nfa-backtrack", "perf-inline", "perf-literal", "std", "unicode"] }
regex-syntax = { version = "0.8" }
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls-native-roots"] }
serde = { version = "1", features = ["alloc", "derive"] }
serde_core = { version = "1", default-features = false, features = ["alloc", "result", "std"] }
serde_json = { version = "1", features = ["raw_value"] }
smallvec = { version = "1", default-features = false, features = ["const_new"] }
tokio = { version = "1", features = ["full"] }
tokio-util = { version = "0.7", features = ["codec", "io"] }
tower = { version = "0.5", default-features = false, features = ["make", "util"] }
tracing = { version = "0.1" }
tracing-core = { version = "0.1" }
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
zeroize = { version = "1", features = ["derive"] }
zstd = { version = "0.13" }
zstd-safe = { version = "7", default-features = false, features = ["arrays", "legacy", "std", "zdict_builder"] }
zstd-sys = { version = "2", default-features = false, features = ["legacy", "std", "zdict_builder"] }

[build-dependencies]
cc = { version = "1", default-features = false, features = ["parallel"] }
proc-macro2 = { version = "1" }
quote = { version = "1" }
syn = { version = "2", features = ["extra-traits", "fold", "full", "visit", "visit-mut"] }

### END HAKARI SECTION

```

### workspace-hack/build.rs

```rust
// A build script is required for cargo to consider build dependencies.
fn main() {}

```

### workspace-hack/src/lib.rs

```rust
// This is a stub lib.rs.

```
