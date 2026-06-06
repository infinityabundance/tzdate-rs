#![no_main]
//! Drive the whole CLI over arbitrary argv + an arbitrary TZif zone file. Must
//! never panic: no OOB in TZif parsing, no overflow in date decomposition, no
//! non-char-boundary slice in strftime/POSIX-TZ parsing.
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Split: first NUL-free chunk = argv (space-split), rest = the TZif bytes.
    let split = data.iter().position(|&b| b == 0).unwrap_or(data.len());
    let argstr = String::from_utf8_lossy(&data[..split]);
    let args: Vec<String> = argstr.split(' ').map(|s| s.to_string()).collect();
    let tzif = if split < data.len() { &data[split + 1..] } else { &[] };
    tzdate_rs::fuzz::__fuzz_run(&args, tzif);
});
