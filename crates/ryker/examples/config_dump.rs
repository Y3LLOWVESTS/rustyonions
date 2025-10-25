// RO:WHAT — Print the effective RykerConfig as JSON.
// RO:HOW  — cargo run -p ryker --example config_dump

use serde_json::json;

fn main() -> anyhow::Result<()> {
    let cfg = ryker::config::from_env_validated()?;
    let d = &cfg.defaults;
    let f = &cfg.fairness;
    let s = &cfg.supervisor;

    let out = json!({
        "defaults": {
            "mailbox_capacity": d.mailbox_capacity,
            "max_msg_bytes": d.max_msg_bytes,
            "deadline_ms": d.deadline.as_millis(),
        },
        "fairness": {
            "batch_messages": f.batch_messages,
            "yield_every_n_msgs": f.yield_every_n_msgs,
        },
        "supervisor": {
            "backoff_base_ms": s.backoff_base_ms,
            "backoff_cap_ms": s.backoff_cap_ms,
            "decorrelated_jitter": s.decorrelated_jitter,
        },
        "amnesia": cfg.amnesia,
        "observe": {
            "queue_depth_sampling": cfg.observe.queue_depth_sampling,
        }
    });

    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}
