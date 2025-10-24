//! RO:WHAT — OAP/1 envelope DTOs (HELLO/START/DATA/END/ERROR).
//! RO:WHY  — Interop contract: frames carry DTOs between services/SDKs.
//! RO:INTERACTS — id::ContentId; version::PROTO_VERSION; error::ProtoError.
//! RO:INVARIANTS — OAP/1 max_frame=1MiB; chunk≈64KiB is a storage guideline (not enforced here). Strict serde.
//! RO:TEST — vectors under tests/vectors; fuzz targets for headers.

use serde::{Deserialize, Serialize};

pub const MAX_FRAME_BYTES: usize = 1 * 1024 * 1024; // spec note only

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
#[non_exhaustive]
pub enum OapKind {
    Hello,
    Start,
    Data,
    End,
    Error,
}

pub mod hello;
pub mod start;
pub mod data;
pub mod end;
pub mod error;
