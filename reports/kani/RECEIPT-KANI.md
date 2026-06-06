# RECEIPT — Kani

**Tool:** `cargo-kani 0.67.0`.

**Harness:** `gmtime_fields_in_range` (`src/lib.rs`, `#[cfg(kani)]`).

## What it proves

`gmtime(t)` (the POSIX-timestamp → civil-date decomposition) yields `month ∈
1..=12`, `wday ∈ 0..=6`, and `mday ∈ 1..=31` for any `time_t` in years 1..9999 —
so every month/day-name table index `strftime` performs is in bounds, and there
is no panic. Pure arithmetic, reduced surface, per the Kani doctrine.

## Result

```
SUMMARY:
 ** 0 of 164 failed (1 unreachable)
VERIFICATION:- SUCCESSFUL
Verification Time: 9.1s
```

✅ **Verified — 0 failed.**

## Reproduce

```
cargo kani --harness gmtime_fields_in_range
```
