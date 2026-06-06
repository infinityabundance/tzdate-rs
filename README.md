# tzdate-rs

`tzdate-rs` is a faithful Rust port of upstream tzcode `date.c`, provided as the
`tzdate` CLI. It is not a general-purpose date/time library and does not replace
GNU/coreutils `date`.

```
tzdate                       # current time in $TZ (live clock)
tzdate -u                    # current time in UT
tzdate -r 1000000000         # a fixed time_t   -> Sun Sep  9 01:46:40 UTC 2001
tzdate -u -r 0 +%Y-%m-%dT%H:%M:%S    # 1970-01-01T00:00:00
TZ=America/New_York tzdate -r 1620000000   # Sun May  2 20:00:00 EDT 2021
```

Upstream `date.c` parses `[-u] [-c] [-r seconds] [+format]`, picks a `time_t`
(now, or `-r`), runs `localtime`, and prints `strftime(format, …)`. tzdate-rs
reproduces that observable behaviour in native Rust with **no runtime
dependencies**.

## Claim boundary

tzdate-rs **does not implement a general date/time library and does not replace
GNU/coreutils `date`.** It ports upstream tzcode `date.c` as a small standalone
tzcode utility and verifies observable CLI behaviour against the compiled
upstream C oracle under pinned environment settings. The private
`localtime`/`strftime` here cover exactly what `date.c` exercises — they are not
exposed as a library and are **outside** any `localtime.c`/`strftime.c`
runtime-library scope. It is the small "show the time" utility of the Rust tzdb
toolchain, beside [`zic-rs`](https://github.com/infinityabundance/zic-rs),
[`tzselect-rs`](https://github.com/infinityabundance/tzselect-rs), and the
producer/QA crates.

## Scope (first seal — boring on purpose)

A boring reproduction of upstream tzcode `date.c`, not a better `date`, a chrono
competitor, a GNU `date` replacement, a new formatting engine, or a timezone
runtime library.

- **CLI contract:** getopt `ucr:`, `-r seconds` (strtoimax base-0 + range
  validation), `-u`/`-c` (UT), the `+format` operand rule, the error/usage
  surface, exit codes — reproduced byte-for-byte.
- **`localtime`:** TZif zone files (with their POSIX footer projected for
  out-of-range times) + POSIX `TZ` strings + UT — scoped to `date.c`'s use.
- **`strftime`:** the C-locale conversions tzcode's `strftime.c` produces
  (including `%+`), verified conversion-by-conversion against the oracle.

## Parity contract

The deterministic oracle is `LC_ALL=C.UTF-8 TZ=… TZDIR=… date` (the compiled
tzcode binary). With `-r` (a fixed `time_t`) the output is fully deterministic.
The no-`-r` current-time path is **live-clock-dependent and classified** — not
byte-compared. `argv0` (which getopt embeds in its error text) is normalized in
the comparison.

## Evidence

- **Oracle sweep:** **1293 / 0** deterministic comparisons match — every
  `strftime` conversion across many timestamps (UTC), 12 named zones with
  DST/odd-offset/footer-projected edges, the `-r` hex/octal/sign/overflow cases,
  and the option/operand/usage error matrix. Receipt: `reports/oracle/`.
- **Kani:** `gmtime` yields in-range month/day/weekday for any `time_t` in
  years 1..9999 — so every month/day table index strftime performs is in bounds.
- **Fuzz:** `parse_seconds` 18.8M, `strftime` 898k, `driver` (full CLI +
  arbitrary TZif) 2.06M executions, **0 crashes**. Receipt: `reports/fuzz/`.

## Build & test

```
cargo build --release
cargo test                    # mock-host golden regression + parse-seconds
cargo clippy --all-targets -- -D warnings
cargo kani --harness gmtime_fields_in_range
python3 lab/oracle/sweep.py   # the oracle sweep (needs the compiled 2026b oracle)
```

## License

BSD-3-Clause, retaining the upstream Regents of the University of California
copyright notice (`date.c` is BSD-licensed). An independent reimplementation.
