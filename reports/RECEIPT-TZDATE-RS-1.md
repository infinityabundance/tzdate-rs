# RECEIPT — TZDATE-RS.1

**Goal.** Port upstream tzcode `date.c` to memory-safe Rust as the `tzdate` CLI,
oracle-first: observable CLI behaviour (stdout/stderr/exit) verified against the
compiled upstream C oracle under a pinned environment, with classified
time-dependent paths, Kani, and fuzz.

**Result.** ✅ Complete. `tzdate-rs 0.1.0` reproduces the compiled tzcode `date`
oracle **byte-for-byte** across **1293** deterministic comparisons (every
`strftime` conversion × many timestamps, 12 named zones with DST / odd offsets /
footer-projected edges, the `-r` parse matrix, and the error/usage surface); the
current-time path is classified; `gmtime` ranges are Kani-proven; the parser,
`strftime`, and the full CLI are fuzz-clean. Zero runtime deps;
`#![forbid(unsafe_code)]`.

## Method (oracle-first)

1. **Admit** tzdb-2026b (`ffad46a0…`, GOODSIG); read `date.c` (212 lines) +
   `date.1`; build the oracle `date` (`make date` → `date.o localtime.o
   strftime.o`, gcc 16.1.1). See `reports/admission/`.
2. **Pin the oracle env:** `LC_ALL=C.UTF-8 TZ=… TZDIR=<zic-compiled tree>`. The
   `-r seconds` flag gives a **fixed `time_t`**, making most output deterministic.
3. **Port** to native Rust (no deps): the CLI contract (`src/lib.rs`), a scoped
   TZif reader + POSIX-TZ projector (`src/tzif.rs`, `src/posixtz.rs`), and a
   C-locale `strftime` (`src/strftime.rs`) — each cited to `date.c`/the probed
   oracle behaviour.
4. **Verify** with an exhaustive sweep (`reports/oracle/`).
5. **Kani** (`reports/kani/`) + **fuzz** (`reports/fuzz/`).

## The hard part (diagnosed, not guessed)

- **Footer projection.** Modern TZif files store explicit transitions only to
  ~2007, then a POSIX footer rule (`EST5EDT,M3.2.0,M11.1.0`). `localtime` must
  *project* that rule for later times — a first cut used the last explicit
  transition and rendered winter-2021 `America/New_York` as `EDT` instead of
  `EST`. Implementing the POSIX-TZ `M`/`J`/`n` projection fixed all 44 DST cases.
- **getopt argv0** in `invalid option` / `option requires an argument` (the
  comparison normalizes it).
- **`multiple -r's used`** has no trailing newline (runs into the usage line).
- **strftime passthrough:** `%P`/`%-d` stay literal; `%O`/`%E` modifiers are
  ignored under C; `%+` = `%a %b %e %H:%M:%S %Z %Y`.

## Scope discipline

A boring reproduction of `date.c` — not a general date/time library, GNU `date`
replacement, chrono competitor, new formatting engine, or timezone runtime
library. The `localtime`/`strftime` are private to the binary, scoped to exactly
what `date.c` exercises, and **outside** any `localtime.c`/`strftime.c`
runtime-library claim.

## Gate

`cargo test` 15 passed · `cargo fmt --check` clean · `cargo clippy --all-targets
-- -D warnings` clean · `cargo kani` verified · release `overflow-checks = true`.

## Non-claims

Not a date/time library; not a coreutils `date` replacement. Byte parity is
scoped to the deterministic `-r` paths under the pinned env; the live-clock path
and locale-dependent formatting are classified.
