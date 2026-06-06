# STATUS — tzdate-rs

**Campaign:** TZDATE-RS.1 — port upstream tzcode `date.c` to memory-safe Rust as
the `tzdate` CLI, oracle-first.

**State:** ✅ complete. Observable CLI behaviour byte-identical to the compiled
tzcode `date` oracle across 1293 deterministic comparisons; live-clock path
classified; Kani-proven decomposition; fuzz-clean.

## Evidence ledger

| Axis | Result |
|------|--------|
| Oracle sweep (deterministic, `-r` fixed `time_t`) | **PASS=1293 · FAIL=0** |
| — every `strftime` conversion × many timestamps (UTC) | match |
| — 12 named zones, DST / odd offsets / footer-projected | match |
| — `-r` hex/octal/sign/overflow/empty, errors, usage, exit codes | match |
| Current-time (no `-r`) | classified (live clock), not byte-compared |
| Kani | `gmtime_fields_in_range` verified (0/164 failed) |
| Fuzz | `parse_seconds` 18.8M · `strftime` 898k · `driver` 2.06M execs · 0 crashes |
| Tests | 15 (`cargo test`: mock-host goldens + parse-seconds) |
| `fmt` / `clippy -D warnings` | clean |
| Runtime dependencies | 0 |
| `unsafe` | forbidden (`#![forbid(unsafe_code)]`) |

## What the port covers

- **CLI contract** (`date.c:42-160`): getopt `ucr:`, `-r` strtoimax base-0 +
  range/empty validation, `-u`/`-c`→UT, `+format` operand rule, multiple-`-r` /
  multiple-operand / unknown-operand / unknown-option errors, usage, exit codes.
- **`localtime`**: a minimal, bounds-checked TZif reader (v1/v2) + POSIX `TZ`
  footer projection (the recurring DST rule for times beyond the last explicit
  transition — the key to matching modern zones) + UT.
- **`strftime`**: the C-locale conversions tzcode emits, including `%+`, the
  composites (`%c`/`%D`/`%r`/`%T`/`%F`/`%R`/`%x`/`%X`), the ISO week fields
  (`%V`/`%G`/`%g`/`%u`), `%z`/`%Z`/`%s`, and the unsupported-conversion
  passthrough (`%P`/`%-d` left literal, `%O`/`%E` modifiers ignored).

## Subtle bytes (diagnosed, not guessed)

- modern TZif stores explicit transitions only to ~2007 then a **POSIX footer
  rule** — `localtime` must *project* the footer for later times (a winter 2021
  `America/New_York` is `EST`, not the last transition's `EDT`).
- getopt embeds **`argv0`** in `invalid option` / `option requires an argument`
  (normalized in the comparison).
- `date: error: multiple -r's used` has **no trailing newline** (runs into the
  usage line) — reproduced verbatim.

## Pinned admission (tzdb-2026b)

| Artifact | sha256 |
|----------|--------|
| `date.c` (212 lines) | `6c093549ddbbc94bde79a313cf37e7fe479ad007464c44f91d002d7c2b819d24` |
| oracle binary (`date.o localtime.o strftime.o`) | `4920a1532f87825acd65af71d843879829ff9ff0dfc7a60b6085d93b88527ad7` |
| bundle `tzdb-2026b.tar.lz` | `ffad46a04c8d1624197056630af475a35f3556d0887f028ac1bd33b7d47dc653` (GOODSIG) |

Compiler: `gcc 16.1.1`; build `make date`; pinned env `LC_ALL=C.UTF-8 TZ=… TZDIR=…`.

## Non-claims

- Not a general date/time library; not a GNU/coreutils `date` replacement.
- `localtime`/`strftime` are private to the `date.c` port, scoped to its needs —
  not a `localtime.c`/`strftime.c` runtime library.
- Byte parity is scoped to the deterministic `-r` paths under the pinned env;
  the live-clock path and locale-dependent formatting are classified.
- POSIX `TZ` DST rules cover the standard `M`/`J`/`n` forms used by tzdb footers.
