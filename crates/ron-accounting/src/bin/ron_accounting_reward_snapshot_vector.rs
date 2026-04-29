//! RO:WHAT — Emits the stable ron-accounting reward snapshot interop vector for live WEB3 smoke tests.
//! RO:WHY — Pillar 12; Concerns: ECON/DX/GOV. Lets svc-rewarder consume accounting-produced reward inputs.
//! RO:INTERACTS — ron_accounting::reward_snapshot_interop_vector_v1, svc-rewarder compute routes, scripts.
//! RO:INVARIANTS — usage/snapshot data only; no wallet mutation; no ledger mutation; canonical b3 CID preserved.
//! RO:METRICS — none; this is an operator/dev proof helper.
//! RO:CONFIG — no runtime config; output mode selected by CLI arg.
//! RO:SECURITY — synthetic fixture accounts only; no bearer tokens, secrets, object bytes, or PII.
//! RO:TEST — scripts/web3_accounting_rewarder_wallet_smoke.sh and ron-accounting interop vector tests.

#![forbid(unsafe_code)]

use ron_accounting::reward_snapshot_interop_vector_v1;

fn main() {
    if let Err(err) = run() {
        eprintln!("ron-accounting reward snapshot vector failed: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mode = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "--json-pretty".to_string());

    let vector = reward_snapshot_interop_vector_v1()?;
    vector.validate()?;

    match mode.as_str() {
        "--json" | "--json-compact" => {
            println!("{}", serde_json::to_string(&vector)?);
        }
        "--json-pretty" => {
            println!("{}", serde_json::to_string_pretty(&vector)?);
        }
        "--snapshot-json" => {
            println!("{}", vector.canonical_snapshot_json);
        }
        "--snapshot-cid" => {
            println!("{}", vector.snapshot_cid);
        }
        "--epoch-id" => {
            println!("{}", vector.epoch_id);
        }
        "--help" | "-h" => {
            print_help();
        }
        other => {
            return Err(format!(
                "unknown mode {other:?}; expected --json-pretty, --json, --snapshot-json, --snapshot-cid, --epoch-id"
            )
            .into());
        }
    }

    Ok(())
}

fn print_help() {
    println!(
        "\
ron_accounting_reward_snapshot_vector

USAGE:
  cargo run -p ron-accounting --bin ron_accounting_reward_snapshot_vector -- [MODE]

MODES:
  --json-pretty    Print the full interop vector as pretty JSON. Default.
  --json           Print the full interop vector as compact JSON.
  --snapshot-json  Print only the canonical rewarder-compatible snapshot JSON.
  --snapshot-cid   Print only the canonical b3:<64hex> snapshot CID.
  --epoch-id       Print only the fixture epoch id.
  --help           Print this help.

PURPOSE:
  Supplies deterministic ron-accounting reward inputs to svc-rewarder smoke tests.
"
    );
}
