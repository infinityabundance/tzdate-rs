# RECEIPT — oracle sweep

**Harness:** `lab/oracle/sweep.py` (paths derived from the script location).

## What it tests

Feed identical argv to the compiled tzcode `date` oracle and to `tzdate` (rs)
under `LC_ALL=C.UTF-8 TZ=… TZDIR=…`, and compare stdout + stderr + exit. `argv0`
is normalized (getopt embeds it). Only the **deterministic** `-r <fixed time_t>`
paths are byte-compared; the no-`-r` current-time path is classified.

## Coverage

- **UTC specifier sweep:** every `strftime` conversion (`a A b B c C d D e F g G h
  H I j k l m M n p P r R s S t T u U V w W x X y Y z Z % +`) + composites + the
  default `%+`, across ~20 timestamps (epoch, pre-1970, 2001, the 2³¹ boundary,
  2038, far future, leap day, year/ISO-week boundaries).
- **Named zones (localtime via TZif + footer):** America/New_York, Europe/London,
  Asia/Kathmandu (+5:45), Australia/Lord_Howe (southern DST, half-hour),
  Pacific/Kiritimati / Etc/GMT-14 (+14), Pacific/Chatham, Asia/Tehran, … across
  DST-edge timestamps.
- **`-r` parse matrix:** decimal/hex(`0x`)/octal(`0`)/`+`/`-`/overflow/empty/junk.
- **Error/usage matrix:** unknown option, multiple `-r`, multiple operands,
  non-`+` operand, missing `-r` value, `--`, clusters (`-ur0`, `-uc`).

## Result

```
PASS=1293  FAIL=0
```

**0 failures.** Every deterministic path is byte-identical to the compiled
tzcode oracle.

## Reproduce

```
make -C lab/admit/tzdb-2026b date
zic -d lab/oracle/tzdir lab/admit/tzdb-2026b/tzdata.zi
cargo build --release
python3 lab/oracle/sweep.py
```
