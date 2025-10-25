//! RO:WHAT — Crate facade for ryker (actor & bounded mailbox runtime).
//! RO:WHY  — Pillar 1 (Kernel & Orchestration); Concerns: RES/PERF.
//! RO:INTERACTS — modules: config, runtime, mailbox, supervisor, observe, errors.
//! RO:INVARIANTS — bounded mailboxes (reject-new Busy); deadlines enforced; no locks across .await.
//! RO:METRICS — via observe::MailboxObserver callbacks (host integrates Prometheus).
//! RO:CONFIG — env `RYKER_*` honored; builder > env > file > defaults precedence.
//! RO:SECURITY — library-only; no sockets/TLS/PII; amnesia feature zeroizes on drop (host-verified).
//! RO:TEST — unit/integration/loom per docs; property tests optional (proptest).

#![forbid(unsafe_code)]
#![doc = include_str!("../README.md")]

pub mod config;
pub mod errors;
pub mod mailbox;
pub mod observe;
pub mod runtime;
pub mod supervisor;

pub mod prelude;
