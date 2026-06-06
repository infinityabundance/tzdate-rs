#![no_main]
//! Fuzz the `-r seconds` strtoimax-style parser.
use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: &[u8]| {
    tzdate_rs::fuzz::__fuzz_parse_seconds(&String::from_utf8_lossy(data));
});
