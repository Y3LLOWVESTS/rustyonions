//! RO:WHAT — Filesystem helper placeholders for the WAL persistence backend.
//! RO:WHY — Pillar 12; Concerns: RES/SEC. Disk work must be bounded and permission-checked.
//! RO:INTERACTS — wal::segment, wal::replay, config::WalConfig.
//! RO:INVARIANTS — disk paths validated before use; no hidden writes in amnesia mode.
//! RO:METRICS — future fs ops report latency/failure counters.
//! RO:CONFIG — wal.dir and fsync flags.
//! RO:SECURITY — later batch enforces 0700 WAL dir and owner-only writes.
//! RO:TEST — wal_tests feature lane.

use std::path::Path;

use crate::errors::{Error, Result};

/// Validate that a WAL directory value is present. Permission hardening lands in Batch 2.
pub fn validate_wal_dir_present(path: &Path) -> Result<()> {
    if path.as_os_str().is_empty() {
        return Err(Error::schema("wal.dir is empty"));
    }
    Ok(())
}
