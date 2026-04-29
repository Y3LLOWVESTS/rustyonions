//! RO:WHAT — Small local helper for BLAKE3 CID and paid-storage context-idem generation.
//! RO:WHY — Pillar 12; Concerns: ECON/DX/SEC. Live WEB3 smoke scripts need repo-local deterministic hashes.
//! RO:INTERACTS — svc_storage::policy::paid_write::paid_storage_context_idem, blake3.
//! RO:INVARIANTS — emits canonical b3:<64 lowercase hex>; mirrors paid-storage context binding exactly.
//! RO:METRICS — none; CLI helper only.
//! RO:CONFIG — no runtime config.
//! RO:SECURITY — reads local files only; no network, no secrets.
//! RO:TEST — exercised by scripts/web3_paid_storage_live_smoke.sh and cargo clippy --all-targets.

#![forbid(unsafe_code)]

use std::{env, fs, process};

use svc_storage::policy::paid_write::paid_storage_context_idem;

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);

    match args.next().as_deref() {
        Some("cid") => {
            let path = args
                .next()
                .ok_or_else(|| usage("missing path for cid command"))?;

            if args.next().is_some() {
                return Err(usage("cid command accepts exactly one path"));
            }

            let bytes = fs::read(&path).map_err(|err| format!("failed to read {path}: {err}"))?;
            println!("b3:{}", blake3::hash(&bytes).to_hex());
            Ok(())
        }
        Some("context-idem") => {
            let cid = args
                .next()
                .ok_or_else(|| usage("missing cid for context-idem command"))?;
            let payer = args
                .next()
                .ok_or_else(|| usage("missing payer for context-idem command"))?;
            let escrow = args
                .next()
                .ok_or_else(|| usage("missing escrow for context-idem command"))?;
            let asset = args
                .next()
                .ok_or_else(|| usage("missing asset for context-idem command"))?;
            let amount = args
                .next()
                .ok_or_else(|| usage("missing amount_minor for context-idem command"))?;

            if args.next().is_some() {
                return Err(usage("context-idem command accepts exactly five arguments"));
            }

            let amount_minor = amount
                .parse::<u128>()
                .map_err(|_| "amount_minor must be an unsigned integer".to_string())?;

            let idem = paid_storage_context_idem(&cid, &payer, &escrow, &asset, amount_minor)
                .map_err(|err| err.to_string())?;

            println!("{idem}");
            Ok(())
        }
        _ => Err(usage("unknown or missing command")),
    }
}

fn usage(reason: &str) -> String {
    format!(
        "{reason}\nusage:\n  roc_b3_tool cid <path>\n  roc_b3_tool context-idem <cid> <payer> <escrow> <asset> <amount_minor>"
    )
}
