# 2026-10-17: When a file with `\r\n` line endings counts as formatted

ruff's `phf.rs` has three `\r\n` line endings among `\n` ones; `cargo fmt --check` passes on
it and `chloro --check` failed (#97). The first diagnosis, that rustfmt keeps the stray
`\r\n`, was wrong: rustfmt's output has `\n` throughout, and it never writes the file.

## Current State

- **rustfmt's rule** (`source_file::write_file`). Under `newline_style = Auto` it compares its
  output with the source map's text, which rustc normalised on reading (no byte order mark,
  `\r\n` read as `\n`); under any other style it reads the file again and compares bytes.
  The overwrite emitter writes only on a difference, and `--check` diffs the same pair.
- **Observed with rustfmt 1.9.0**, for five files (mixed endings, all `\r\n` formatted, all
  `\r\n` unformatted, a byte order mark, plain `\n`) under Auto, Unix, Windows and Native:
  under Auto, the first, second and fourth are formatted and left as they are; the third is
  written with `\n` line endings; under the other styles every byte counts. Files with
  `#![rustfmt::skip]` and runs with `disable_all_formatting` are never changed. chloro now
  gives the same result in all twenty cases.
- **Auto writes `\n`.** rustfmt's `apply_newline_style` detects the style on the normalised
  source, in which no `\r\n` is left, so Auto output has `\n` line endings (the platform's
  for a source without a newline). The docs had said that Auto keeps the input's line
  endings; the CLI guide, the library guide, the crate documentation and `NewlineStyle::Auto`
  now say what it does.
- **The change.** `chloro_core::is_unchanged(source, formatted, config)` ports the
  comparison: bytes equal, or under Auto the normalised source equal to the output. The CLI's
  changed-file test uses it; the library's output is the same as before.
- **The cache.** A hit prints the file's content as the output, so only content the
  formatter returns byte for byte is recorded; a `\r\n` file formatted under Auto is formatted
  again on every run, as it was before.

## Missing

- **A cache for `\r\n` checkouts.** On a Windows checkout with `core.autocrlf`, every file has
  `\r\n` line endings and none is ever cached. A record that answers `--check` and `--write`
  but not print mode would let warm runs skip them.
