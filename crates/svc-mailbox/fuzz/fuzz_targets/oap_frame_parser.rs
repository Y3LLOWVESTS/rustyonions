// fuzz target placeholder for OAP frame parser
#![no_main]
use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: &[u8]| {
    let _ = data; // placeholder
});

