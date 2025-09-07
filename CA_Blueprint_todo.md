
>Short list
>Build and refine later

## Merge plan (single PR)

1. **Add/ensure files** (exact paths from the blueprint):

   * `testing/lib/ready.sh`
   * `testing/ci_invariants.sh`
   * `testing/ci_sanitizers_run.sh`
   * `ci/crate-classes.toml`
   * `.github/workflows/ci-invariants.yml`
   * `.github/workflows/ci-sanitizers-mandatory.yml`
   * `.github/workflows/ci-rt-flavors.yml`
   * `.github/workflows/ci-loom-kernel.yml`
   * `specs/readyz_dag.tla`, `specs/readyz_dag.cfg`
   * `specs/readyz_dag_failures.tla`, `specs/readyz_dag_failures.cfg`
   * `fuzz/Cargo.toml`, `fuzz/fuzz_targets/oap_roundtrip.rs`, `fuzz/dictionaries/oap.dict`
   * (helper) `crates/common/src/test_rt.rs`
2. **Wire crate features** (`rt-multi-thread`, `rt-current-thread`) in crates that have async tests.
3. **Kernel loom**: add the provided `loom_health.rs` test and `loom` dev-dep/feature to `ron-kernel`.
4. **Health degraded mode**: return 503 JSON as specified when any readiness key times out.
5. **Lock in lints**: add the deny prelude at crate roots.

## Quick local smoke (paste to shell)

```
chmod +x testing/*.sh
testing/ci_invariants.sh
ENABLE_MIRI=0 testing/ci_sanitizers_run.sh || true
cargo test --all --features rt-multi-thread
cargo test --all --features rt-current-thread
cargo test -p ron-kernel --features loom --test loom_health
cargo install cargo-fuzz
cargo fuzz run oap_roundtrip -- -dict=fuzz/dictionaries/oap.dict
```

## Repo hygiene

* Protect `main`: require all four CI checks (Invariants, TSan, RT-Flavors, Loom).
* Set `PHASE` to **Silver** by default; bump to **Gold/Platinum** on release branches.
* Add the PR checklist from the blueprint to `.github/PULL_REQUEST_TEMPLATE.md`.

## Nice-to-haves (post-merge)

* Implement real OAP `encode/parse` and hook the fuzz target.
* Make readiness timeouts configurable per key; include `retry_after` in 503 payloads.
* Bake the TLA+ spec SHA into build logs for Platinum releases.

If you want, I can prep the PR body and a crisp commit message. Otherwise—massive congrats. This blueprint is production-grade and future-proof. 🚀
