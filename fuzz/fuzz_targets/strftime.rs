#![no_main]
//! Fuzz strftime over an arbitrary format at a fixed time. Must never panic.
use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: &[u8]| {
    tzdate_rs::fuzz::__fuzz_strftime(&String::from_utf8_lossy(data), 1000000000);
});
