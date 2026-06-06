//! `tzdate` — CLI front end for the [`tzdate_rs`] port of tzcode `date.c`.
//!
//! `tzdate [-u] [-c] [-r seconds] [+format]` — prints the time (current, or the
//! `-r` `time_t`) under `$TZ`/`$TZDIR`, formatted by `strftime`. A faithful port
//! of upstream tzcode `date.c`; not a general date/time library.
#![forbid(unsafe_code)]

use std::io::Write;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use tzdate_rs::{run, Host};

struct RealHost;

impl Host for RealHost {
    fn now(&self) -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }
    fn getenv(&self, key: &str) -> Option<String> {
        std::env::var(key).ok()
    }
    fn read_file(&self, path: &str) -> Option<Vec<u8>> {
        std::fs::read(path).ok()
    }
    fn out(&mut self, s: &str) {
        let mut o = std::io::stdout();
        let _ = o.write_all(s.as_bytes());
        let _ = o.flush();
    }
    fn err(&mut self, s: &str) {
        let mut e = std::io::stderr();
        let _ = e.write_all(s.as_bytes());
        let _ = e.flush();
    }
}

fn main() -> ExitCode {
    let argv0 = std::env::args()
        .next()
        .unwrap_or_else(|| "date".to_string());
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut host = RealHost;
    match run(&argv0, &args, &mut host) {
        0 => ExitCode::SUCCESS,
        _ => ExitCode::FAILURE,
    }
}
