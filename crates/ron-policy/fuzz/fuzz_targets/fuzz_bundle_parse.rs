#![no_main]
use libfuzzer_sys::fuzz_target;
use ron_policy::{load_json, load_toml};

fuzz_target!(|data: &[u8]| {
    let _ = load_json(data);
    let _ = load_toml(data);
});
