//! RO:WHAT — Placeholder module for concurrency-aware shutdown helpers.
//! RO:WHY  — Keep a stable home for future drain orchestration (e.g. draining
//!           work queues before shutdown, coordinating with bus + HTTP).
//! RO:NOTE — Today Micronode reuses the kernel’s shutdown wiring and does not
//!           need additional helpers here. This module exists so the
//!           concurrency blueprint has a concrete code anchor.

// This module is intentionally empty for now. Once Micronode grows its own
// work queues / worker pools, shutdown coordination types can live here.
