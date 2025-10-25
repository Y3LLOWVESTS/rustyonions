//! RO:WHAT — Common imports for ergonomic use of OAP.
//! RO:WHY  — Reduce `use` boilerplate in services and SDKs.
//! RO:INTERACTS — Pure re-exports.

pub use crate::{
    codec::{OapDecoder, OapEncoder},
    envelope::{hello_reply_default, hello_request, Capability, FrameBuilder, is_fire_and_forget, is_terminal, wants_ack},
    error::{OapDecodeError, OapEncodeError, OapError, StatusCode},
    flags::Flags,
    frame::Frame,
    header::Header,
    metrics::{
        is_client_err, is_server_err, is_success, labels_for_outcome, outcome_from_decode,
        outcome_from_status, reason, OutcomeClass,
    },
    seq::Seq,
    constants::{OAP_VERSION, MAX_FRAME_BYTES, STREAM_CHUNK_SIZE},
};
