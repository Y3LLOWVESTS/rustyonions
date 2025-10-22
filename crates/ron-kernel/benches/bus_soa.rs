// benches/bus_soa.rs
//
// RO:WHAT
// - Criterion bench entrypoint for the SoA backend.
// - Compiles cleanly whether or not the "bus_soa" feature is enabled.
//
// RO:WHY
// - Cargo builds all benches during `cargo bench`. Without guarding, this file
//   would emit E0601 (no main) when the feature is off.
//
// RO:INTERACTS
// - Uses Criterion only when "bus_soa" is enabled.
// - When the feature is disabled, provides a tiny stub `main()` so the bench
//   target still builds and the rest of the suite can run.
//
// RO:INVARIANTS
// - Never pull SoA symbols unless the feature is on.
// - Keep a deterministic group name for CI diffing even if the body is trivial.

#[cfg(not(feature = "bus_soa"))]
fn main() {
    // Feature not enabled; benign stub so `cargo bench` can proceed.
    // Tip: run with `--features bus_soa` to enable this bench's real body.
    eprintln!("bench 'bus_soa' compiled without --features bus_soa; skipping.");
}

#[cfg(feature = "bus_soa")]
mod soa_bench {
    use criterion::{criterion_group, criterion_main, Criterion};

    // If you already have shared bench helpers, import them here, e.g.:
    // use ron_kernel::bench_support::{run_publish_matrix_soa, BenchCfg};

    // Minimal placeholder so the bench runs even before the SoA runner lands.
    // Replace with your real SoA matrix once implemented.
    fn bench_bus_soa(c: &mut Criterion) {
        let mut group = c.benchmark_group("bus_soa");
        // TODO: swap this placeholder with the real SoA publish/deliver matrix.
        group.bench_function("noop_build_only", |b| b.iter(|| 0u64));
        group.finish();
    }

    criterion_group!(name = soa; config = Criterion::default(); targets = bench_bus_soa);
    criterion_main!(soa);
}
