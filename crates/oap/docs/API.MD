---

title: API Surface & SemVer Reference — oap
status: draft
msrv: 1.80.0
last-updated: 2025-10-06
audience: contributors, auditors, API consumers
-----------------------------------------------

# API.md — `oap`

## 0. Purpose

This document captures the **public API surface** of the `oap` crate:

* Snapshot of exported functions, types, traits, modules.
* SemVer discipline: what changes break vs. extend.
* Alignment with `CHANGELOG.md` (behavioral vs. surface changes).
* CI-enforceable via `cargo public-api` (or similar).
* Acts as the **spec** for external consumers embedding the OAP/1 framer.

`oap` is a **pure protocol library** (no sockets/TLS/auth). Its public API focuses on:

* Envelope/Frame types
* Parser/Writer (encode/decode) primitives
* Config & error taxonomy
* Optional metrics trait hooks

---

## 1. Public API Surface

**How we generated this snapshot**

```bash
cargo public-api --simplified --deny-changes -p oap
```

> Note: Items marked `#[non_exhaustive]` are intentionally open for additive evolution.

### 1.1 Current Surface (snapshot)

```text
# Crate root re-exports
pub mod envelope
pub mod frame
pub mod parser
pub mod writer
pub mod error
pub mod metrics
pub mod consts
pub mod seq
pub mod prelude

# Prelude (for convenience imports)
pub mod prelude {
    pub use crate::envelope::{EnvelopeKind, Envelope};
    pub use crate::frame::{Frame, FrameHeader, MAX_FRAME_BYTES};
    pub use crate::parser::{Parser, ParserConfig, Progress};
    pub use crate::writer::{Writer, WriterConfig};
    pub use crate::error::{OapError, OapConfigError, Result};
    pub use crate::seq::{Seq, SeqRolloverPolicy};
    pub use crate::consts::VERSION;
    pub use crate::metrics::OapMetricsExt;
}

# Envelope module
pub mod envelope {
    #[non_exhaustive]
    pub enum EnvelopeKind { HELLO, START, DATA, ACK, END, ERROR }

    #[non_exhaustive]
    pub enum Envelope {
        Hello { version: u32, max_frame: u64, pq_hybrid: bool },
        Start { stream: String },
        Data  { obj: Option<String>, seq: u64, len: u32 }, // body is carried in Frame
        Ack   { up_to: u64 },
        End   { seq: u64, status: u16 },
        Error { code: u16, reason: String },
    }
}

# Frame module
pub mod frame {
    pub const MAX_FRAME_BYTES: usize; // = 1_048_576 (1 MiB)
    pub struct FrameHeader {
        pub kind: crate::envelope::EnvelopeKind,
        pub len: u32,
    }
    pub struct Frame<'a> {
        pub header: FrameHeader,
        pub body: &'a [u8], // zero-copy slice into caller buffer
    }
    pub fn encode_frame<B: AsRef<[u8]>>(env: &crate::envelope::Envelope, body: B) -> crate::error::Result<Vec<u8>>;
    pub fn decode_frame<'a>(st: &mut crate::parser::ParserState, buf: &'a [u8]) -> crate::parser::Progress<'a>;
}

# Parser module
pub mod parser {
    pub struct ParserConfig {
        pub strict_headers: bool,
        pub enforce_obj_header_on_objects: bool,
        pub ack_window_frames: u32,
        pub allow_pq_hello_flags: bool,
        pub seq_rollover_policy: crate::seq::SeqRolloverPolicy,
    }
    pub struct ParserState { /* opaque */ }
    pub struct Parser { /* opaque */ }

    #[non_exhaustive]
    pub enum Progress<'a> {
        NeedMore,
        Frame { frame: crate::frame::Frame<'a>, rest: &'a [u8] },
    }

    impl Parser {
        pub fn new(cfg: ParserConfig) -> crate::error::Result<Self>;
        pub fn with_metrics(cfg: ParserConfig, metrics: impl crate::metrics::OapMetricsExt) -> crate::error::Result<Self>;
        pub fn state(&self) -> &ParserState;
        pub fn feed<'a>(&mut self, buf: &'a [u8]) -> crate::error::Result<Progress<'a>>;
        pub fn reset(&mut self);
    }
}

# Writer module
pub mod writer {
    pub struct WriterConfig { /* reserved for future */ }
    pub struct Writer { /* opaque */ }

    impl Writer {
        pub fn new(cfg: WriterConfig) -> Self;
        pub fn encode<B: AsRef<[u8]>>(&self, env: &crate::envelope::Envelope, body: B) -> crate::error::Result<Vec<u8>>;
    }
}

# Error module
pub mod error {
    #[non_exhaustive]
    pub enum OapError {
        FrameOversize,
        UnknownEnvelope,
        HeaderMalformed,
        CreditViolation,
        Truncated,
        ObjRequiredMissing,
        BadHello,
        Io(std::io::Error),
        // future non-breaking expansions allowed
    }
    pub type Result<T> = core::result::Result<T, OapError>;

    #[non_exhaustive]
    pub enum OapConfigError {
        InvalidAckWindow(u32),
    }
}

# Metrics module (lib-facing trait; host provides impl)
pub mod metrics {
    pub trait OapMetricsExt: Clone + Send + Sync + 'static {
        fn inc_frame(&self, kind: &'static str);
        fn inc_reject(&self, reason: &'static str);
        fn add_frame_bytes(&self, n: u64);
        fn set_inflight(&self, v: i64);
        fn set_ack_window(&self, v: i64);
        fn time_decode<F, R>(&self, f: F) -> R where F: FnOnce() -> R;
        fn time_encode<F, R>(&self, f: F) -> R where F: FnOnce() -> R;
    }
}

# Seq / credit algebra
pub mod seq {
    pub struct Seq(pub u64);
    #[non_exhaustive]
    pub enum SeqRolloverPolicy { Error, Wrap }
}

# Consts
pub mod consts {
    pub const VERSION: u32; // OAP/1 protocol version (semantic integer)
}
```

> **Services:** N/A — `oap` exposes **no** CLI/HTTP/gRPC endpoints.

---

## 2. SemVer Discipline

### Additive (Minor / Non-Breaking)

* Adding new helper functions, methods, modules **without** changing existing signatures.
* Adding new `OapError` variants **only** because it is `#[non_exhaustive]`.
* Adding new fields to `Envelope` variants **only** if the enum remains `#[non_exhaustive]` and defaulting behavior is specified.
* Providing new impls (e.g., `From<std::io::Error> for OapError`) that don’t conflict.

### Breaking (Major)

* Renaming or removing any exported type, function, trait, or module.
* Changing function/impl signatures (parameter/return types) or generics.
* Making previously `#[non_exhaustive]` enums **exhaustive**.
* Changing protocol **invariants** surfaced in API (e.g., changing `MAX_FRAME_BYTES` for OAP/1).

### Patch-Level

* Doc-only fixes/comments.
* Internal perf/memory improvements that keep signatures and behavior equivalent.
* Extending fuzz/property test matrices with no user-facing changes.

---

## 3. Stability Guarantees

* **MSRV:** `1.80.0`.
* **No `unsafe`** in public-facing code paths (any justified `unsafe` gated and documented).
* Parser/Writer APIs are **cancel-safe**; partial inputs never corrupt internal state.
* No leaking of non-portable internal types (e.g., no `tokio::net::TcpStream`) in public signatures.

---

## 4. Invariants (API-aligned)

* `frame::MAX_FRAME_BYTES == 1_048_576` (**1 MiB**) for all of OAP/1.
  This value is **not** a configuration knob and any change is **major**.
* Envelope set locked to OAP/1 grammar; enums are `#[non_exhaustive]` to allow **forward-compatible flags** (e.g., PQ markers in `HELLO`) without breaking.
* `error::OapError` and `error::OapConfigError` are `#[non_exhaustive]` to allow additive hardening outcomes.

---

## 5. Tooling

* **cargo public-api** — detects and prints public surface diffs.
* **cargo semver-checks** — validates SemVer compatibility against previous release.
* **cargo doc** — rendered API docs; examples must compile (`doc(test)`).
* **Snapshots** stored in: `/docs/api-history/oap/<version>.txt`.

---

## 6. CI & Gates

* PR pipeline runs:

  * `cargo public-api --simplified -p oap` (fail on unexpected diffs).
  * `cargo semver-checks check-release -p oap` (advisory gate; fail on breaking).
  * `cargo doc -p oap -Zrustdoc-map` (docs must build cleanly).
* Bot posts a comment with added/removed symbols; PR must include a matching `CHANGELOG.md` entry for any surface change.

**Sample GitHub Actions step**

```yaml
- name: Public API diff
  run: cargo install cargo-public-api && cargo public-api --simplified -p oap
```

---

## 7. Acceptance Checklist (DoD)

* [ ] Current API snapshot generated & saved under `/docs/api-history/oap/`.
* [ ] `cargo public-api` gate green (or intentional diff acknowledged).
* [ ] `cargo semver-checks` reviewed (no unintentional breakage).
* [ ] `CHANGELOG.md` updated for any surface or behavior change.
* [ ] All public items documented (`#![deny(missing_docs)]` satisfied).
* [ ] Fuzz/property tests updated if envelope/codec behavior extended.

---

## 8. Appendix

### References

* Rust SemVer: [https://doc.rust-lang.org/cargo/reference/semver.html](https://doc.rust-lang.org/cargo/reference/semver.html)
* cargo-public-api: [https://github.com/Enselic/cargo-public-api](https://github.com/Enselic/cargo-public-api)
* cargo-semver-checks: [https://github.com/obi1kenobi/cargo-semver-checks](https://github.com/obi1kenobi/cargo-semver-checks)

### Perfection Gates tie-in

* **Gate G:** No undocumented API surface.
* **Gate H:** Breaking changes require **major** bump.
* **Gate J:** CHANGELOG alignment enforced with CI evidence.

### History (notable shifts)

* `1.0.0`: Initial OAP/1 surface — `Parser`, `Writer`, `Envelope`, `Frame`, `OapError`, `ParserConfig`, `WriterConfig`, metrics trait.
* (append future entries here…)

---

✅ With this API.md, `oap`’s external contract is **explicit, testable, and CI-enforced**, keeping the framer stable while allowing careful additive evolution.
