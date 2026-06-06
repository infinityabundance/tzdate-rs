# RECEIPT — fuzz

**Tool:** `cargo-fuzz 0.13.1` (libFuzzer + ASan), Rust nightly.

**Targets** (`fuzz/fuzz_targets/`):
- `driver` → the whole CLI over arbitrary argv + an arbitrary TZif zone file.
- `parse_seconds` → the `-r seconds` strtoimax-style parser.
- `strftime` → the format engine over an arbitrary format at a fixed time.

**Detachment:** `fuzz/Cargo.toml` declares an empty `[workspace]`; the parent
gate never compiles the fuzz crate (verified).

## What it stresses

No OOB in the TZif reader (counts/offsets/abbrev), no overflow in the date
decomposition or POSIX-TZ projection, no slice-on-non-char-boundary in strftime.

## Result

```
parse_seconds: Done 18810117 runs — 0 crashes
strftime:      Done   898132 runs — 0 crashes
driver:        Done  2061454 runs — 0 crashes
```

✅ **0 crashes / 0 panics.** Bounded clean runs, not saturation.

## Reproduce

```
cargo +nightly fuzz run parse_seconds -- -max_total_time=20
cargo +nightly fuzz run strftime      -- -max_total_time=20
cargo +nightly fuzz run driver        -- -max_total_time=35
```
