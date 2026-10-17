# Changelog

All notable changes to chloro, for people who use it. The format follows [Keep a Changelog](<https://keepachangelog.com/en/1.1.0/>), and versions follow [Semantic Versioning](<https://semver.org/spec/v2.0.0.html>). `chloro` and `chloro-core` are released together at the same version.

This file is written by hand. Each crate also has a commit-level changelog generated at release time by release-plz: [chloro/CHANGELOG.md](<https://github.com/lmmx/chloro/blob/master/chloro/CHANGELOG.md>) and [chloro-core/CHANGELOG.md](<https://github.com/lmmx/chloro/blob/master/chloro-core/CHANGELOG.md>).

## [Unreleased](<https://github.com/lmmx/chloro/compare/chloro-v0.9.0...HEAD>)

## [0.9.0](<https://github.com/lmmx/chloro/compare/chloro-v0.8.0...chloro-v0.9.0>) - 2026-10-17

The performance and conformance work of [\#88](<https://github.com/lmmx/chloro/pull/88>). Every output is identical to 0.8.0's except for the conformance fixes listed under *Fixed*. The measurements behind the figures below are described on the [Performance](<https://lmmx.github.io/chloro/performance/>) page.

### Added

- The command line formats files on every core, one worker per core, taking the largest files first; `--threads N` sets the number of workers.
- Files of 128 KiB or more also have their top-level items formatted on several threads, with identical output.
- A cache of file contents known to be formatted: a later `--check` or `--write` skips them without parsing. It is keyed by the chloro binary and the configuration, lives in the user cache directory, and does not change any output or exit status. `--no-cache` turns it off; `--cache-dir` and `CHLORO_CACHE_DIR` move it.
- `--unit-cache`, an opt-in cache of formatted top-level items, so that a changed file formats only the items not seen before.
- The `rustfmt_version` option reproduces rustfmt 1.8.0 (as shipped with Rust 1.88) as well as 1.9.0, the default: `--config rustfmt_version=1.8.0`.
- `#[rustfmt::skip::macros(..)]` and `#[rustfmt::skip::attributes(..)]` are honoured.
- `CHLORO_TRACE=FILE` writes a per-worker timeline of a command-line run.
- `-V`/`--version` prints the version. A build from anything but the release's tagged commit is marked as a development build, with its commit and whether it had uncommitted changes: `chloro 0.8.0-dev (2f9b060, dirty)`.
- `chloro-core` library functions: `format_source_with_config_parallel` (one file's items on several threads), `format_source_with_unit_cache` with the `UnitCache` trait, `UnitKey` and `CachedUnit` (reuse formatted items across calls), and `parse_only`. The existing functions are unchanged.
- Release binaries for the targets GitHub's runners build natively are built with profile-guided optimisation.
- A documentation website at [https://lmmx.github.io/chloro/](<https://lmmx.github.io/chloro/>), and this changelog.

### Changed

- Formatting is faster. Against rustfmt 1.9.0 on nine crates, chloro takes 0.21–0.35 of rustfmt's wall time on one core and 0.08–0.17 on four. Against 0.8.0, the formatter executes 66% fewer instructions on the same files and formats 2.7 times as many bytes per second on one core.
- On the whole ruff workspace (2,008 files), chloro took 0.28 of `cargo fmt`'s wall time on one thread and 0.038 on 20.
- Peak memory on the same crates is 4–10 MiB on one core, against 6–29 MiB for 0.8.0 (rustfmt: 43–86 MiB).
- Nested closures no longer slow formatting down superlinearly: a 2 KB file of closures nested 16 deep takes 5.5 million instructions, against 242.5 million in 0.8.0.
- `chloro-core`'s runtime dependency tree is 18 crates, from 37. rust-analyzer's syntax tree crates (`ra_ap_syntax`, `rowan`) are no longer runtime dependencies; the formatter reads its own flat syntax tree. `bumpalo` is added; `ra-ap-rustc_lexer` and `rustc-literal-escaper`, already in the tree through `ra_ap_parser`, are now named directly.
- The release profile uses fat link-time optimisation and one codegen unit.
- A file reachable twice on the command line (named twice, or through a symlink) is formatted once.

### Fixed

Output now matches rustfmt 1.9.0 on all 1220 rust-analyzer conformance files (1219 in 0.8.0) and on all 20,540 files of a 577-crate registry corpus that rustfmt formats (20,526 of 20,536 before):

- Nested blocks in a macro definition get rustfmt's trailing semicolons.
- A repeat array, `[x; n]`, is not laid out as a block.
- Doc comments inside macro arguments are formatted as rustfmt formats them.
- Files containing syntax rustc's parser rejects are left unchanged, as rustfmt leaves them: `<-`, `..a = b`, `a..=` without an end, an ambiguous `+` in a type, an unbraced const parameter default, `builtin #` syntax, pattern parameters in an edition-2015 trait method, a misplaced doc comment, and a C string literal before edition 2021.
- Files with syntax rustc accepts but rust-analyzer's parser rejects are now formatted: a parenthesised first bound of a bare trait object (`Box<(Copy) + Send>`), a float literal as a field name, a `const` block item, `impl impl Trait {}`, and an unknown literal prefix before edition 2021.
- A panic while formatting on a worker thread is reported with its own message.

## [0.8.0](<https://github.com/lmmx/chloro/compare/chloro-v0.7.5...chloro-v0.8.0>) - 2026-10-11

A rewrite of the formatter: a module-for-module port of rustfmt 1.9.0 onto rust-analyzer's syntax trees ([\#85](<https://github.com/lmmx/chloro/pull/85>)). Output matches rustfmt on 1219 of the 1220 rust-analyzer conformance files, against 233 before.

### Added

- `Config`, with a public field for every stable rustfmt option and `Config::set` taking rustfmt's option names and values; `format_source_with_config` and `Config` are re-exported from `chloro`.
- `--config key=value,...` on the command line, validated before any file is read.
- `#[rustfmt::skip]` on files and nodes.
- `extern crate` items ([\#80](<https://github.com/lmmx/chloro/pull/80>)) and `if let` chains ([\#82](<https://github.com/lmmx/chloro/pull/82>)).

### Changed

- The formatting rules are rustfmt's: files rustfmt rejects are returned unchanged, unstable rustfmt options keep their defaults, and `style_edition` follows `edition` unless set.
- Method chains are no longer "snuggled" ([\#83](<https://github.com/lmmx/chloro/pull/83>)).

### Fixed

- Comments on `use` items are kept ([\#81](<https://github.com/lmmx/chloro/pull/81>)).

## [0.7.5](<https://github.com/lmmx/chloro/compare/chloro-v0.7.4...chloro-v0.7.5>) - 2025-12-05

### Fixed

- Long comments are no longer split at the maximum line width ([\#78](<https://github.com/lmmx/chloro/pull/78>)).

(The release-plz changelogs of both crates list this change under 0.8.0.)

## [0.7.0](<https://github.com/lmmx/chloro/releases/tag/chloro-v0.7.0>) – [0.7.4](<https://github.com/lmmx/chloro/compare/chloro-v0.6.8...chloro-v0.7.4>) - 2025-11-26 to 2025-12-05

Incremental work on the hand-written formatter that preceded the rustfmt port: comments kept in enums, structs, macro calls and before items; `use` and `mod` ordering as in the 2024 style; struct literals, enum variants, `extern` blocks, `impl` blocks and more expressions. The per-crate changelogs list each release.

## [0.6.0](<https://github.com/lmmx/chloro/releases/tag/chloro-v0.6.0>) – [0.6.8](<https://github.com/lmmx/chloro/compare/chloro-v0.5.3...chloro-v0.6.8>) - 2025-11-22 to 2025-11-26

Nested imports, generics and `where` clauses, struct literals and function formatting, and comment whitespace handling; a conformance report against rust-analyzer's sources in the README.

## [0.5.0](<https://crates.io/crates/chloro/0.5.0>) – [0.5.3](<https://github.com/lmmx/chloro/releases/tag/chloro-v0.5.3>) - 2025-11-21

The first releases: a proof-of-concept formatter over rust-analyzer's `rowan` trees, with library and command-line interfaces.
