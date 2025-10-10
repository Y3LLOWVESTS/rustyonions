// fuzz target scaffold
#![no_main]
use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: &[u8]| {
    let _ = data.iter().fold(0u8, |acc, b| acc ^ b);
});
