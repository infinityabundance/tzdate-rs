//! Self-contained regression tests: drive `run` through a mock [`Host`] (fixed
//! clock, env map, committed TZif zone fixtures) and compare the captured
//! stdout+stderr+exit against golden transcripts.
//!
//! The goldens lock in the behaviour that the oracle sweep (`lab/oracle/sweep.py`,
//! see `reports/oracle/`) proves byte-identical to the compiled tzcode `date`.
//! Re-bless with `BLESS=1 cargo test`.

use std::fs;
use std::path::PathBuf;

use tzdate_rs::{run, Host};

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

struct MockHost {
    env: Vec<(String, String)>,
    out: String,
    err: String,
}
impl Host for MockHost {
    fn now(&self) -> i64 {
        1_000_000_000 // fixed clock (only matters for the no-`-r` path)
    }
    fn getenv(&self, key: &str) -> Option<String> {
        self.env
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
    }
    fn read_file(&self, path: &str) -> Option<Vec<u8>> {
        fs::read(path).ok()
    }
    fn out(&mut self, s: &str) {
        self.out.push_str(s);
    }
    fn err(&mut self, s: &str) {
        self.err.push_str(s);
    }
}

fn run_case(name: &str, tz: &str, args: &[&str]) {
    let tzdir = dir().join("tests/fixtures/zoneinfo");
    let mut h = MockHost {
        env: vec![
            ("TZ".to_string(), tz.to_string()),
            ("TZDIR".to_string(), tzdir.to_string_lossy().into_owned()),
        ],
        out: String::new(),
        err: String::new(),
    };
    let argv: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let code = run("date", &argv, &mut h);
    let got = format!(
        "=== exit {code} ===\n=== stdout ===\n{}\n=== stderr ===\n{}",
        h.out, h.err
    );
    let golden = dir()
        .join("tests/fixtures/golden")
        .join(format!("{name}.txt"));
    if std::env::var("BLESS").is_ok() {
        fs::create_dir_all(golden.parent().unwrap()).unwrap();
        fs::write(&golden, &got).unwrap();
        return;
    }
    let want = fs::read_to_string(&golden)
        .unwrap_or_else(|_| panic!("missing golden {name}; run BLESS=1"));
    assert_eq!(got, want, "[{name}] transcript diverged from golden");
}

macro_rules! case {
    ($fn:ident, $tz:literal, [$($a:literal),*]) => {
        #[test]
        fn $fn() {
            run_case(stringify!($fn), $tz, &[$($a),*]);
        }
    };
}

case!(utc_epoch, "UTC0", ["-u", "-r", "0"]);
case!(utc_billennium, "UTC0", ["-u", "-r", "1000000000"]);
case!(
    utc_fmt,
    "UTC0",
    ["-u", "-r", "0", "+%Y-%m-%dT%H:%M:%S %A %j %V %G %s %z %Z"]
);
case!(
    utc_all_composites,
    "UTC0",
    ["-u", "-r", "1000000000", "+%c|%D|%r|%T|%F|%R|%x|%X|%+"]
);
case!(ny_winter, "America/New_York", ["-r", "1610000000"]);
case!(ny_summer, "America/New_York", ["-r", "1620000000"]);
case!(
    london_summer,
    "Europe/London",
    ["-r", "1000000000", "+%a %F %T %Z %z"]
);
case!(
    lord_howe_dst,
    "Australia/Lord_Howe",
    ["-r", "1610000000", "+%F %T %Z %z"]
);
case!(kathmandu, "Asia/Kathmandu", ["-r", "0", "+%F %T %Z %z"]);
case!(err_unknown_opt, "UTC0", ["-Z"]);
case!(err_bad_r, "UTC0", ["-u", "-r", "notanumber"]);
case!(err_multiple_r, "UTC0", ["-r", "0", "-r", "1"]);
case!(err_unknown_operand, "UTC0", ["-u", "foo"]);
case!(
    r_hex_octal_neg,
    "UTC0",
    ["-u", "-r", "0x3b9aca00", "+%s|%Y"]
);

#[test]
fn parse_seconds_examples() {
    // -r accepts decimal/hex/octal/sign (strtoimax base 0).
    let probe = |arg: &str| -> (String, i32) {
        let tzdir = dir().join("tests/fixtures/zoneinfo");
        let mut h = MockHost {
            env: vec![
                ("TZ".into(), "UTC0".into()),
                ("TZDIR".into(), tzdir.to_string_lossy().into_owned()),
            ],
            out: String::new(),
            err: String::new(),
        };
        let code = run(
            "date",
            &["-u".into(), "-r".into(), arg.into(), "+%s".into()],
            &mut h,
        );
        (h.out, code)
    };
    assert_eq!(probe("0"), ("0\n".into(), 0));
    assert_eq!(probe("0x10"), ("16\n".into(), 0));
    assert_eq!(probe("010"), ("8\n".into(), 0));
    assert_eq!(probe("-5"), ("-5\n".into(), 0));
    assert_eq!(probe("notanumber").1, 1);
    assert_eq!(probe("12abc").1, 1);
}
