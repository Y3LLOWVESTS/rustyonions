#![no_main]
use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: &[u8]| {
    let _ = data.iter().fold(0u64, |acc, b| acc.wrapping_add(*b as u64));
});
