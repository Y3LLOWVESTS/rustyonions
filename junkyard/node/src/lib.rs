#![forbid(unsafe_code)]
// Intentionally minimal library: the CLI lives in src/main.rs.
// The old `commands` module was from a previous API and caused
// unresolved-import errors. We'll reintroduce a typed library
// interface after the refactor.

pub mod cli {
    // (empty) – retained only to avoid breaking external imports, if any.
}
