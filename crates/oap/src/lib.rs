//! RO:WHAT — OAP/1 constants, flags, header & frame types, and a Tokio codec (Encoder/Decoder).
//! RO:WHY  — Pillar 7 (App BFF & SDK); Concerns: SEC/RES/PERF/DX. Stable envelopes for interop.
//! RO:INTERACTS — bytes, tokio-util::codec; optional ron-proto DTOs; consumed by ron-app-sdk/omnigate.
//! RO:INVARIANTS — OAP/1 max_frame=1MiB; stream chunk≈64KiB (storage); no lock across .await in codec.
//! RO:METRICS — (none here; callers record RED metrics around IO).
//! RO:CONFIG — Bounds are constants; optional zstd with ≤8× expansion guard.
//! RO:SECURITY — No ambient auth; capabilities carried as opaque bytes in START; DTOs deny_unknown_fields upstream.
//! RO:TEST — unit tests in module; golden vectors for HELLO/START/DATA; fuzz hooks to be added in oap-fuzz.

#![forbid(unsafe_code)]

pub mod constants;
pub mod error;
pub mod flags;
pub mod header;
pub mod frame;
pub mod codec;
pub mod hello;

// TODO-aligned modules
pub mod envelope;
pub mod metrics;
pub mod prelude;
pub mod seq;

// Stream helpers (as per TODO folders)
pub mod parser;
pub mod writer;

// Core exports
pub use constants::*;
pub use error::{OapDecodeError, OapEncodeError, OapError, StatusCode};
pub use flags::Flags;
pub use header::Header;
pub use frame::Frame;
pub use codec::{OapDecoder, OapEncoder};
pub use hello::{Hello, HelloReply};

// Ergonomic helpers from TODO modules
pub use envelope::{
    hello_reply_default, hello_request, Capability, FrameBuilder, is_fire_and_forget, is_terminal,
    wants_ack,
};
pub use metrics::{
    is_client_err, is_server_err, is_success, labels_for_outcome, outcome_from_decode,
    outcome_from_status, reason, OutcomeClass,
};
pub use seq::Seq;

// Parser/Writer facades
pub use parser::{ParserConfig, ParserState};
pub use writer::{OapWriter, WriterConfig};
