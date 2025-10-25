//! RO:WHAT — Common imports for ergonomic use of OAP.
//! RO:WHY  — Reduce `use` boilerplate in services and SDKs.
//! RO:INTERACTS — Pure re-exports.

pub use crate::{
    codec::{OapDecoder, OapEncoder},
    constants::{MAX_FRAME_BYTES, OAP_VERSION, STREAM_CHUNK_SIZE},
    envelope::{
        hello_reply_default, hello_request, is_fire_and_forget, is_terminal, wants_ack, Capability,
        FrameBuilder,
    },
    error::{OapDecodeError, OapEncodeError, OapError, StatusCode},
    flags::Flags,
    frame::Frame,
    header::Header,
    metrics::{
        is_client_err, is_server_err, is_success, labels_for_outcome, outcome_from_decode,
        outcome_from_status, reason, OutcomeClass,
    },
    seq::Seq,
};
