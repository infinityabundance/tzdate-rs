//! tzdate-rs — a faithful Rust port of upstream tzcode `date.c`, provided as the
//! `tzdate` CLI.
//!
//! Upstream `date.c` (BSD, in the tz distribution) is a small standalone utility:
//! it parses `[-u] [-c] [-r seconds] [+format]`, picks a `time_t` (now, or `-r`),
//! runs `localtime` on it, and prints `strftime(format, …)`. This crate ports that
//! observable behaviour to native Rust with **no runtime dependencies**.
//!
//! ## Claim boundary
//!
//! tzdate-rs **does not implement a general date/time library and does not
//! replace GNU/coreutils `date`.** It ports upstream tzcode `date.c` as a small
//! standalone tzcode utility and verifies observable CLI behaviour against the
//! compiled upstream C oracle under pinned environment settings. The private
//! `localtime`/`strftime` here cover exactly what `date.c` exercises — they are
//! not exposed as a library and are **outside** any `localtime.c`/`strftime.c`
//! runtime-library scope. It is the small "show the time" utility of the Rust
//! tzdb toolchain, beside `zic-rs`, `tzselect-rs`, and the producer/QA crates.
//!
//! ## Parity contract
//!
//! The deterministic oracle is `LC_ALL=C.UTF-8 TZ=… TZDIR=… date`. With `-r`
//! (a fixed `time_t`) the output is fully deterministic; the no-`-r` current-time
//! path is live-clock-dependent and **classified**, not byte-compared.
#![forbid(unsafe_code)]

/// Host services: the clock, the environment, and zoneinfo file reads — injected
/// so the port is unit-testable and the live clock is deterministic in tests.
pub trait Host {
    /// `time(NULL)` — current time in seconds since the POSIX epoch.
    fn now(&self) -> i64;
    /// `getenv` for `TZ` / `TZDIR`.
    fn getenv(&self, key: &str) -> Option<String>;
    /// Read a zoneinfo (TZif) file by absolute path.
    fn read_file(&self, path: &str) -> Option<Vec<u8>>;
    /// Write to stdout.
    fn out(&mut self, s: &str);
    /// Write to stderr.
    fn err(&mut self, s: &str);
}

/// `date: usage: …` text (`date.c:155-157`).
const USAGE: &str = "date: usage: date [-u] [-c] [-r seconds] [+format]\n";

/// Run `date` with the given `argv0` and argument list. Returns the exit code;
/// output is delivered via [`Host`].
pub fn run(argv0: &str, args: &[String], h: &mut dyn Host) -> i32 {
    let mut retval = 0i32;
    let errensure = |rv: &mut i32| {
        if *rv == 0 {
            *rv = 1;
        }
    };
    let mut format = "+%+".to_string();
    let mut t = h.now();
    let mut utc = false;
    let mut rflag = false;

    // getopt(argc, argv, "ucr:") (date.c:62-93).
    let mut operands: Vec<String> = Vec::new();
    let mut i = 0;
    let mut pending_chars: Vec<char> = Vec::new();
    let mut idx_in_cluster = 0usize;
    // A small getopt: -u/-c flags, -r takes a value, clusters allowed, `--`/`-`,
    // first non-option-ish operand stops option scanning only after options.
    let mut stop = false;
    while i < args.len() {
        let a = &args[i];
        if stop {
            operands.push(a.clone());
            i += 1;
            continue;
        }
        if pending_chars.is_empty() {
            if a == "--" {
                stop = true;
                i += 1;
                continue;
            }
            if a.starts_with('-') && a.len() > 1 {
                pending_chars = a.chars().skip(1).collect();
                idx_in_cluster = 0;
            } else {
                operands.push(a.clone());
                i += 1;
                continue;
            }
        }
        if !pending_chars.is_empty() {
            let ch = pending_chars[idx_in_cluster];
            idx_in_cluster += 1;
            match ch {
                'u' | 'c' => utc = true,
                'r' => {
                    // optarg = rest of cluster, or the next argv.
                    let optarg: Option<String> = if idx_in_cluster < pending_chars.len() {
                        let s: String = pending_chars[idx_in_cluster..].iter().collect();
                        Some(s)
                    } else {
                        i += 1;
                        args.get(i).cloned()
                    };
                    let optarg = match optarg {
                        Some(s) => s,
                        None => {
                            // getopt: option requires an argument -- 'r' + usage.
                            h.err(&format!("{argv0}: option requires an argument -- 'r'\n"));
                            h.err(USAGE);
                            errensure(&mut retval);
                            return retval;
                        }
                    };
                    pending_chars.clear();
                    if rflag {
                        h.err("date: error: multiple -r's used");
                        h.err(USAGE);
                        errensure(&mut retval);
                        return retval;
                    }
                    rflag = true;
                    match parse_seconds(&optarg) {
                        Ok(secs) => t = secs,
                        Err(e) => {
                            h.err(&format!("date: {optarg}: {e}\n"));
                            errensure(&mut retval);
                            return retval;
                        }
                    }
                    i += 1;
                    continue;
                }
                _ => {
                    // getopt: unknown option (uses argv0).
                    h.err(&format!("{argv0}: invalid option -- '{ch}'\n"));
                    h.err(USAGE);
                    errensure(&mut retval);
                    return retval;
                }
            }
            if idx_in_cluster >= pending_chars.len() {
                pending_chars.clear();
                i += 1;
            }
            continue;
        }
        i += 1;
    }

    if utc {
        // dogmt(): localtime now uses UTC0.
    }

    // Operand handling (date.c:94-105).
    if !operands.is_empty() {
        if operands.len() != 1 {
            h.err("date: error: multiple operands in command line\n");
            h.err(USAGE);
            errensure(&mut retval);
            return retval;
        }
        let op = &operands[0];
        if !op.starts_with('+') {
            h.err(&format!("date: unknown operand: {op}\n"));
            h.err(USAGE);
            errensure(&mut retval);
            return retval;
        }
        format = op.clone();
    }

    // display(format, t) (date.c:162-183): localtime → strftime → "\n".
    let lt = if utc {
        Some(Localtime::utc())
    } else {
        localtime(t, h)
    };
    match lt {
        Some(local) => {
            let tm = gmtime(t + local.utoff);
            let body = strftime(&format[1..], &tm, t, local.utoff, &local.abbrev);
            h.out(&body);
            h.out("\n");
            retval
        }
        None => {
            h.err("date: error: time out of range\n");
            errensure(&mut retval);
            retval
        }
    }
}

/// `strtoimax(optarg, &endarg, 0)` + range/empty validation (date.c:77-90).
/// Returns the strerror text on failure (`EINVAL`/`ERANGE`).
fn parse_seconds(s: &str) -> Result<i64, &'static str> {
    // Base 0: 0x → hex, 0 → octal, else decimal; optional leading +/-.
    let bytes = s.as_bytes();
    let mut p = 0;
    let neg = match bytes.first() {
        Some(b'-') => {
            p = 1;
            true
        }
        Some(b'+') => {
            p = 1;
            false
        }
        _ => false,
    };
    let rest = &s[p..];
    let (radix, digits) =
        if let Some(hex) = rest.strip_prefix("0x").or_else(|| rest.strip_prefix("0X")) {
            (16, hex)
        } else if rest.starts_with('0') && rest.len() > 1 {
            (8, &rest[1..])
        } else {
            (10, rest)
        };
    if digits.is_empty() {
        // optarg == endarg (no digits consumed) or trailing junk → EINVAL.
        // Note: bare "0" parses as decimal 0 (digits non-empty for radix 10).
        if rest == "0" {
            return Ok(0);
        }
        return Err("Invalid argument");
    }
    let mag = match i128::from_str_radix(digits, radix) {
        Ok(v) => v,
        Err(_) => {
            // Distinguish bad-digit (EINVAL) from overflow (ERANGE) by re-scan.
            if digits.bytes().all(|b| (b as char).is_digit(radix)) {
                return Err("Numerical result out of range");
            }
            return Err("Invalid argument");
        }
    };
    let val = if neg { -mag } else { mag };
    if val < i64::MIN as i128 || val > i64::MAX as i128 {
        return Err("Numerical result out of range");
    }
    Ok(val as i64)
}

/// The result of `localtime`: UT offset (seconds), DST flag, zone abbreviation.
struct Localtime {
    utoff: i64,
    #[allow(dead_code)]
    isdst: bool,
    abbrev: String,
}
impl Localtime {
    fn utc() -> Localtime {
        Localtime {
            utoff: 0,
            isdst: false,
            abbrev: "UTC".to_string(),
        }
    }
}

/// `localtime(t)` honouring `$TZ` / `$TZDIR` — scoped to what `date.c` needs: a
/// TZif zone file (with its POSIX footer for out-of-range times), or a POSIX
/// `TZ` string, else UTC.
fn localtime(t: i64, h: &dyn Host) -> Option<Localtime> {
    let tz = h.getenv("TZ").unwrap_or_default();
    if tz.is_empty() {
        // No TZ: defer to the system /etc/localtime if readable, else UTC.
        if let Some(bytes) = h.read_file("/etc/localtime") {
            if let Some(l) = tzif_localtime(&bytes, t) {
                return Some(l);
            }
        }
        return Some(Localtime::utc());
    }
    let name = tz.strip_prefix(':').unwrap_or(&tz);
    // Try as a zoneinfo file (named zone under $TZDIR, or an absolute path).
    let tzdir = h
        .getenv("TZDIR")
        .unwrap_or_else(|| "/usr/share/zoneinfo".to_string());
    let path = if name.starts_with('/') {
        name.to_string()
    } else {
        format!("{tzdir}/{name}")
    };
    if let Some(bytes) = h.read_file(&path) {
        if let Some(l) = tzif_localtime(&bytes, t) {
            return Some(l);
        }
    }
    // Else interpret it as a POSIX `TZ` string (e.g. `UTC0`, `EST5EDT,…`).
    posix_tz_offset(name, t).or_else(|| Some(Localtime::utc()))
}

include!("tzif.rs");
include!("posixtz.rs");
include!("strftime.rs");

/// Fuzzing-only entry points (behind the `fuzzing` feature).
#[cfg(feature = "fuzzing")]
#[doc(hidden)]
pub mod fuzz {
    use super::*;

    struct FuzzHost {
        tzif: Vec<u8>,
    }
    impl Host for FuzzHost {
        fn now(&self) -> i64 {
            0
        }
        fn getenv(&self, k: &str) -> Option<String> {
            if k == "TZ" {
                Some("Fuzz/Zone".to_string())
            } else {
                None
            }
        }
        fn read_file(&self, _p: &str) -> Option<Vec<u8>> {
            Some(self.tzif.clone())
        }
        fn out(&mut self, _s: &str) {}
        fn err(&mut self, _s: &str) {}
    }

    /// Drive the whole CLI over arbitrary argv + an arbitrary TZif zone file.
    pub fn __fuzz_run(args: &[String], tzif: &[u8]) {
        let mut h = FuzzHost {
            tzif: tzif.to_vec(),
        };
        let _ = run("date", args, &mut h);
    }

    /// Fuzz the `-r seconds` strtoimax-style parser.
    pub fn __fuzz_parse_seconds(s: &str) {
        let _ = parse_seconds(s);
    }

    /// Fuzz strftime over an arbitrary format at a fixed time.
    pub fn __fuzz_strftime(fmt: &str, t: i64) {
        let tm = gmtime(t);
        let _ = strftime(fmt, &tm, t, 0, "UTC");
    }
}

#[cfg(kani)]
mod kani_harness {
    use super::*;
    /// `gmtime` never panics and yields in-range month (1..=12) and a weekday in
    /// 0..=6 for any `time_t` in a bounded window — so every `MONTHS`/`DAYS` index
    /// strftime performs is in bounds. Pure arithmetic, reduced surface.
    #[kani::proof]
    #[kani::unwind(4)]
    fn gmtime_fields_in_range() {
        let t: i64 = kani::any();
        kani::assume((-62135596800..=253402300799).contains(&t)); // years 1..=9999
        let tm = gmtime(t);
        assert!((1..=12).contains(&tm.month));
        assert!((0..=6).contains(&tm.wday));
        assert!((1..=31).contains(&tm.mday));
    }
}
