//! RO:WHAT — Minimal ron-ledger example showing a file-backed mint and replayable root publication.
//! RO:WHY  — Pillar 12; Concerns: DX/ECON. Gives a tiny, copyable example for crate consumers.
//! RO:INTERACTS — ron_ledger::{Ledger, FileStorage, IngestRequest, Entry}.
//! RO:INVARIANTS — deterministic root after commit; no service wrapper required.
//! RO:METRICS — none.
//! RO:CONFIG — default LedgerConfig.
//! RO:SECURITY — identifiers only.
//! RO:TEST — example compile check.

use std::time::{SystemTime, UNIX_EPOCH};

use ron_ledger::{
    api::IngestRequest,
    config::LedgerConfig,
    engine::{FileStorage, Ledger},
    types::{AccountId, CapabilityRef, Entry, EntryKind, Kid, Nonce},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let dir = std::env::temp_dir().join(format!("ron-ledger-example-{unique}"));
    let storage = FileStorage::open(&dir)?;
    let ledger = Ledger::new(storage, LedgerConfig::default())?;

    let req = IngestRequest {
        batch: vec![Entry::new(
            "mint-example",
            1,
            EntryKind::Mint,
            AccountId::new("acct_example")?,
            100,
            Nonce::from_base64("AAAAAAAAAAAAAAAAAAAAAA==")?,
            Kid::new("kid-example")?,
            CapabilityRef::new("cap-example")?,
            1,
        )?],
        idem_id: Some("example-batch".into()),
    };

    let resp = ledger.ingest(req)?;
    println!("accepted={} root={}", resp.accepted, resp.new_root.to_hex());
    Ok(())
}
