#![no_main]

use libfuzzer_sys::fuzz_target;
use ron_ledger::{config::LedgerConfig, engine::{Ledger, MemoryStorage}, api::IngestRequest};

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        if let Ok(req) = serde_json::from_str::<IngestRequest>(s) {
            let ledger = Ledger::new(MemoryStorage::default(), LedgerConfig::default());
            if let Ok(ledger) = ledger {
                let _ = ledger.ingest(req);
                let _ = ledger.roots_since(0);
            }
        }
    }
});
