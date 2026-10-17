# 2026-10-17: Reading `rustfmt.toml` and the crate's edition

chloro read no configuration file and formatted every file at edition 2024, so a project
with a `rustfmt.toml`, or a crate at edition 2021, came out differently from `cargo fmt`
(#105). The command line now finds each file's configuration as `cargo fmt` does.

## Current State

- **rustfmt's rules, observed with rustfmt 1.9.0** (output hashes under each candidate
  width), not taken from memory:
  - the nearest `.rustfmt.toml` or `rustfmt.toml` of each file (in that order in a
    directory) applies, even when rustfmt runs in a directory with another one;
  - without one, `$HOME`'s, then `rustfmt/` in the user configuration directory;
  - an unknown option, and an unstable option whatever its value, is a warning, and the
    option is ignored;
  - a value of the wrong TOML type (a string for `max_width`, an integer for `edition`, a
    float, a negative width) makes the file fail to parse, and rustfmt stops;
  - choices are case-insensitive (`newline_style = "unix"`);
  - `fn_args_layout` warns that it is deprecated and applies;
  - `print_misformatted_file_names` is stable and silent.
- **`cargo fmt`'s edition.** It passes each package's edition as `--edition`, which beats an
  `edition` in `rustfmt.toml` but not a `style_edition` there (both checked with
  `cargo fmt`). A package without `edition` is 2015; `edition.workspace = true` reads
  `[workspace.package] edition` of the workspace root (the `package.workspace` path, else
  the nearest ancestor manifest with `[workspace]`).
- **The check.** A three-package workspace (editions 2021, inherited 2018, and none), with a
  root `rustfmt.toml` holding an unstable and an unknown option and a `.rustfmt.toml` in one
  package: `chloro --write .` and `cargo fmt` leave identical trees. The CLI tests encode
  those outputs.
- **Structure.** `cli/project.rs` resolves every file before any is formatted (a refused
  file stops the run first, as in rustfmt), memoised per directory for both lookups; equal
  configurations share one index. Each distinct configuration gets its own fixed-point and
  unit cache, opened from a `cache::Tool` identity taken once at start-up, as before; the
  workers take the file's `Setup`. `Config::set_from_file` in chloro-core holds rustfmt's
  file rules beside `Config::set`, so the option table stays in one crate.
- **TOML.** The `toml` crate with only `parse` and `std` (`DeTable`, no serde): five crates
  (`toml`, `toml_parser`, `toml_datetime`, `serde_spanned`, `winnow` 1.0) under the `cli`
  feature only; the release binary grows from 2,253,960 to 2,456,848 bytes (+9%).
  `Cargo.toml` uses the whole TOML grammar (multi-line arrays and strings, dotted and quoted
  keys, inline tables), which a hand-written reader would have to get right.
- **Cost.** A warm `--check` of the 1238-file corpus (129 directories) takes 16.8 ms against
  15.7 ms before, on four threads; `--isolated` takes 15.6 ms. The difference is the lookup:
  three `stat`s per new directory, then memoised ancestors.
- **Tests.** The CLI tests now run chloro with `HOME`, `USERPROFILE`, `XDG_CONFIG_HOME` and
  `APPDATA` pointing at a directory that does not exist (`tests/common/mod.rs`), so a
  developer's own global `rustfmt.toml` cannot change their results.
- **Benchmarks.** `benchmarks/compare_rustfmt.sh` passes `--isolated` to a chloro that has
  it, as it gives rustfmt an empty `--config-path`: both keep the defaults at edition 2024.
  `just vs-rustfmt` compares with `cargo fmt` on a real project, where both now read its
  configuration.

## Missing

- `rust-toolchain.toml`, which picks the rustfmt that `cargo fmt` runs (`rustfmt_version`
  does that by hand).
- rustfmt's `ignore` option (unstable, so a warning on stable as in rustfmt).
- A lookup that is free on deep trees: one `read_dir` per directory could answer both file
  names and `Cargo.toml` at once.
