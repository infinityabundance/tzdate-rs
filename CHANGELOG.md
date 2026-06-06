# Changelog

## 0.1.0

Initial release. Faithful Rust port of upstream tzcode `date.c`, as the `tzdate`
CLI.

- Reproduces the `date.c` CLI contract (`[-u] [-c] [-r seconds] [+format]`,
  errors, usage, exit codes) byte-for-byte.
- A minimal, scoped `localtime` (TZif v1/v2 + POSIX footer projection + UT) and
  `strftime` (the C-locale conversions, including `%+`), verified
  conversion-by-conversion against the compiled tzcode oracle.
- Oracle sweep 1293/0; the current-time path is classified (live clock).
- Kani proof of `gmtime` field ranges; `cargo fuzz` over the parser, strftime,
  and the full CLI + arbitrary TZif.
- Zero runtime dependencies; `#![forbid(unsafe_code)]`; `overflow-checks` in
  release.
