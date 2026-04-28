//! RO:WHAT — Minimal example for recording and sealing a ron-accounting usage slice.
//! RO:WHY — Pillar 12; Concerns: DX/ECON. Shows library-first embedding without services.
//! RO:INTERACTS — Recorder, LabelSet, Dimension, Window, SealedSlice.
//! RO:INVARIANTS — transient counters only; deterministic seal; no ledger mutation.
//! RO:METRICS — none in this example.
//! RO:CONFIG — uses default recorder/window choices.
//! RO:SECURITY — route labels are normalized before storage.
//! RO:TEST — compiled by cargo test --examples.

use ron_accounting::{Dimension, LabelSet, Recorder, SliceId, Window};

fn main() -> ron_accounting::Result<()> {
    let recorder = Recorder::default();
    let labels = LabelSet::new(1, "svc-storage", "local", "PUT", "/objects/12345");

    recorder.record(labels, Dimension::Bytes, 64 * 1024)?;

    let window = Window::for_timestamp_ms(300_000, 300)?;
    let slice = recorder.seal_slice(
        SliceId {
            tenant: 1,
            dimension: Dimension::Bytes,
            seq: 1,
        },
        window,
        None,
        true,
    )?;

    println!(
        "sealed {} row(s), digest={}",
        slice.rows.len(),
        slice.digest()
    );
    Ok(())
}
